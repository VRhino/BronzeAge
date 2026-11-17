//! Persistencia por registro de eventos + fotos (event sourcing), genérica: guarda bytes opacos y no conoce las reglas
//! del juego (Arquitectura/13 §6).
//!
//! - **Registro** ([`Registro`]): entradas con secuencia 1, 2, 3… sin huecos, en segmentos con longitud + CRC32.
//! - **Foto** ([`Foto`]): estado serializado hasta una secuencia, zstd + sha256, escritura atómica.
//! - **Diario** ([`Diario`]): un registro aparte de salidas deterministas (la crónica) con índice por clave para leerlo por páginas; se trunca a la foto al
//!   arrancar y lo vuelve a escribir quien repite el registro.
//! - **Arranque** ([`Almacen::recuperar`]): última foto válida + entradas posteriores, en orden. El reloj de mundo se
//!   congela con el servidor apagado, así que no hay nada «vencido» que procesar al volver: repetir las entradas con
//!   sus instantes basta.
//!
//! Tolerancia a cortes: una cola truncada o con un byte dañado se descarta desde el primer registro malo y se
//! reporta en [`Aviso`]; nunca se entrega un registro que no pase el CRC. Lo posterior al daño no se puede
//! reproducir (rompería la contigüidad) y [`Almacen::abrir`] lo retira antes de volver a escribir.

mod diario;
mod foto;
mod registro;

pub use diario::{Apunte, Diario, Pagina, CADA};
pub use foto::Foto;
pub use registro::{crc32, Entrada, Registro, Sincronia, Ubicacion, MAX_DATOS};

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Algo anormal encontrado al abrir o recuperar. No impide recuperar: dice qué se descartó.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Aviso {
    /// El último registro estaba incompleto (corte durante la escritura).
    RegistroTruncado { segmento: String, desplazamiento: u64, descartados: u64 },
    /// Un registro no pasa el CRC o su longitud es absurda. Se descarta desde ahí hasta el final del segmento.
    RegistroCorrupto { segmento: String, desplazamiento: u64, descartados: u64 },
    /// Faltan secuencias entre lo que se esperaba y lo que hay en disco; no se puede seguir reproduciendo.
    Hueco { esperada: u64, hallada: u64 },
    /// Una foto no se pudo leer o no pasó la validación; se usó una anterior (o ninguna).
    FotoCorrupta { archivo: String, motivo: String },
    /// Segmento posterior a un daño, apartado como `.descartado` al abrir.
    SegmentoDescartado { archivo: String },
    /// El diario no estaba y se esperaba historia (hay foto): lo anterior a ella no se puede recomponer sin repetir desde el principio.
    DiarioPerdido { directorio: String },
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Tamaño a partir del cual se rota de segmento (se comprueba antes de cada entrada).
    pub max_segmento: u64,
    pub sincronia: Sincronia,
    /// Fotos que se conservan (las más recientes).
    pub fotos_conservadas: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config { max_segmento: 64 << 20, sincronia: Sincronia::PorLote, fotos_conservadas: 3 }
    }
}

/// Resultado de [`Almacen::recuperar`]: carga `foto` (si la hay) y reproduce `entradas_posteriores` en orden.
#[derive(Debug)]
pub struct Recuperacion {
    pub foto: Option<Foto>,
    /// Entradas con `secuencia` mayor que la de la foto (todas, si no hay foto), contiguas y en orden.
    pub entradas_posteriores: std::vec::IntoIter<Entrada>,
    pub avisos: Vec<Aviso>,
}

/// Escritor de fotos desprendible del [`Almacen`], para serializar y escribir desde otro hilo sin parar el núcleo.
#[derive(Clone, Debug)]
pub struct Fotos {
    dir: PathBuf,
    conservar: usize,
}

impl Fotos {
    /// Escribe la foto de forma atómica y poda las más antiguas. El llamante garantiza que `foto.secuencia` ya está
    /// en el registro (para que la foto no apunte a entradas inexistentes).
    pub fn guardar(&self, foto: &Foto) -> io::Result<()> {
        foto::escribir(&self.dir, foto)?;
        foto::podar(&self.dir, self.conservar)
    }
}

pub struct Almacen {
    dir: PathBuf,
    registro: Registro,
    conservar: usize,
    avisos_apertura: Vec<Aviso>,
}

impl Almacen {
    /// Abre (o crea) el directorio, borra temporales de fotos a medias y repara la cola del registro.
    pub fn abrir(dir: impl AsRef<Path>, config: Config) -> io::Result<Almacen> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        for e in fs::read_dir(dir)? {
            let e = e?;
            if e.path().extension().is_some_and(|x| x == "tmp") {
                fs::remove_file(e.path())?;
            }
        }
        let (registro, avisos_apertura) = Registro::abrir(dir, config.max_segmento, config.sincronia)?;
        Ok(Almacen { dir: dir.to_path_buf(), registro, conservar: config.fotos_conservadas.max(1), avisos_apertura })
    }

    pub fn registro(&mut self) -> &mut Registro {
        &mut self.registro
    }

    pub fn fotos(&self) -> Fotos {
        Fotos { dir: self.dir.clone(), conservar: self.conservar }
    }

    /// Carga la foto más reciente válida (si la última está dañada, la anterior) y las entradas posteriores. Solo
    /// lee: llamarla dos veces da lo mismo.
    pub fn recuperar(&self) -> io::Result<Recuperacion> {
        let mut avisos = self.avisos_apertura.clone();
        let mut foto = None;
        for (_, ruta) in foto::listar(&self.dir)?.into_iter().rev() {
            match foto::leer(&ruta) {
                Ok(f) => {
                    foto = Some(f);
                    break;
                }
                Err(motivo) => avisos.push(Aviso::FotoCorrupta {
                    archivo: ruta.file_name().unwrap().to_string_lossy().into_owned(),
                    motivo,
                }),
            }
        }
        let mut entradas = Vec::new();
        let esc = registro::escanear(&self.dir, foto.as_ref().map_or(0, |f| f.secuencia), &mut |e| entradas.push(e))?;
        avisos.extend(esc.avisos);
        Ok(Recuperacion { foto, entradas_posteriores: entradas.into_iter(), avisos })
    }

    /// Borra los segmentos enteramente anteriores a la foto válida más antigua de las conservadas (así cualquier
    /// foto conservada sigue pudiendo recuperar). Nunca toca el último segmento. Devuelve cuántos borró.
    pub fn compactar(&self) -> io::Result<usize> {
        let Some(base) = foto::listar(&self.dir)?.into_iter().find_map(|(_, r)| foto::leer(&r).ok()).map(|f| f.secuencia) else {
            return Ok(0);
        };
        let segs = registro::listar_segmentos(&self.dir)?;
        let mut borrados = 0;
        for w in segs.windows(2) {
            if w[1].inicio <= base + 1 {
                fs::remove_file(&w[0].ruta)?;
                borrados += 1;
            }
        }
        Ok(borrados)
    }
}

#[cfg(test)]
mod pruebas;
