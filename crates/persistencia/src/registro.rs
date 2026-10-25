//! Registro de entradas: ficheros de solo añadir, partidos en segmentos `registro-<primera secuencia>.log`.
//!
//! Formato de cada registro (little endian): `longitud: u32 | crc32: u32 | cuerpo`, con
//! `cuerpo = secuencia: u64 | instante: i64 | tipo: u16 | datos`. El CRC cubre el cuerpo entero.

use crate::foto;
use crate::Aviso;
use base::Instante;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Lo que entró al núcleo. Reproducidas en orden de `secuencia` reconstruyen el mismo mundo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entrada {
    /// Empieza en 1 y crece de uno en uno, sin huecos.
    pub secuencia: u64,
    pub instante: Instante,
    pub tipo: u16,
    pub datos: Vec<u8>,
}

/// Dónde empieza una entrada en disco: el segmento (por su primera secuencia) y el desplazamiento dentro de él.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ubicacion {
    pub segmento: u64,
    pub desplazamiento: u64,
}

/// Cuándo se fuerza el `fsync` (`sync_data`) al disco.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sincronia {
    /// Solo en la rotación de segmento y al llamar a `sincronizar()`. Sobrevive a un cierre del proceso, no a un
    /// corte de corriente.
    Nunca,
    /// Al final de cada `anadir` / `anadir_lote`: un lote entero cuesta un solo `fsync`.
    PorLote,
    /// Al final de la llamada que cumpla el intervalo desde el último `fsync`. No hay hilo temporizador: si el
    /// núcleo queda ocioso, la cola sin sincronizar espera a la siguiente llamada o a un `sincronizar()` externo.
    Intervalo(Duration),
}

const CABECERA: usize = 8;
const FIJO: usize = 18; // secuencia + instante + tipo
/// Tope de `datos` de una entrada; también protege la lectura de longitudes absurdas por corrupción.
pub const MAX_DATOS: usize = 64 << 20;

// --- CRC32 (IEEE 802.3, el de zlib) con tabla; sin dependencia externa ---
const TABLA: [u32; 256] = {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
};

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in bytes {
        c = TABLA[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

fn codificar(buf: &mut Vec<u8>, secuencia: u64, instante: Instante, tipo: u16, datos: &[u8]) {
    let ini = buf.len();
    buf.extend_from_slice(&[0; CABECERA]);
    buf.extend_from_slice(&secuencia.to_le_bytes());
    buf.extend_from_slice(&instante.to_le_bytes());
    buf.extend_from_slice(&tipo.to_le_bytes());
    buf.extend_from_slice(datos);
    let cuerpo = &buf[ini + CABECERA..];
    let (len, crc) = (cuerpo.len() as u32, crc32(cuerpo));
    buf[ini..ini + 4].copy_from_slice(&len.to_le_bytes());
    buf[ini + 4..ini + 8].copy_from_slice(&crc.to_le_bytes());
}

pub(crate) enum Fallo {
    /// Faltan bytes: la escritura se cortó.
    Truncado,
    /// Hay bytes pero no cuadran (longitud absurda o CRC distinto).
    Corrupto,
}

pub(crate) fn decodificar(bytes: &[u8], off: usize) -> Result<(Entrada, usize), Fallo> {
    let resto = &bytes[off..];
    if resto.len() < CABECERA {
        return Err(Fallo::Truncado);
    }
    let len = u32::from_le_bytes(resto[0..4].try_into().unwrap()) as usize;
    let crc = u32::from_le_bytes(resto[4..8].try_into().unwrap());
    if !(FIJO..=FIJO + MAX_DATOS).contains(&len) {
        return Err(Fallo::Corrupto);
    }
    if resto.len() < CABECERA + len {
        return Err(Fallo::Truncado);
    }
    let cuerpo = &resto[CABECERA..CABECERA + len];
    if crc32(cuerpo) != crc {
        return Err(Fallo::Corrupto);
    }
    let e = Entrada {
        secuencia: u64::from_le_bytes(cuerpo[0..8].try_into().unwrap()),
        instante: i64::from_le_bytes(cuerpo[8..16].try_into().unwrap()),
        tipo: u16::from_le_bytes(cuerpo[16..18].try_into().unwrap()),
        datos: cuerpo[FIJO..].to_vec(),
    };
    Ok((e, off + CABECERA + len))
}

// --- Segmentos y escaneo ---

pub(crate) struct Segmento {
    pub inicio: u64,
    pub ruta: PathBuf,
}

pub(crate) fn nombre_segmento(inicio: u64) -> String {
    format!("registro-{inicio:020}.log")
}

/// Segmentos del directorio, ordenados por primera secuencia.
pub(crate) fn listar_segmentos(dir: &Path) -> io::Result<Vec<Segmento>> {
    let mut v = Vec::new();
    for e in fs::read_dir(dir)? {
        let e = e?;
        let n = e.file_name();
        let Some(n) = n.to_str() else { continue };
        if let Some(num) = n.strip_prefix("registro-").and_then(|s| s.strip_suffix(".log")) {
            if let Ok(inicio) = num.parse::<u64>() {
                v.push(Segmento { inicio, ruta: e.path() });
            }
        }
    }
    v.sort_by_key(|s| s.inicio);
    Ok(v)
}

pub(crate) struct Escaneo {
    pub avisos: Vec<Aviso>,
    /// Última secuencia válida y contigua leída (o `despues` si no había ninguna).
    pub ultima: u64,
    /// Primer daño: índice del segmento y desplazamiento válido hasta el que se puede conservar.
    pub corte: Option<(usize, u64)>,
    /// Si hay un hueco entre `despues` y lo que hay en disco, no se puede seguir leyendo.
    pub hueco: bool,
    /// Última secuencia del último segmento leído (para decidir si se puede seguir añadiendo a él).
    pub fin_ultimo_segmento: Option<u64>,
    pub segmentos: Vec<Segmento>,
}

/// Lee en orden las entradas con `secuencia > despues`, exigiendo contigüidad. Se detiene en el primer daño o hueco
/// y lo reporta; jamás entrega bytes que no pasen el CRC. Los segmentos enteramente anteriores no se abren.
pub(crate) fn escanear(dir: &Path, despues: u64, sink: &mut dyn FnMut(Entrada)) -> io::Result<Escaneo> {
    let segmentos = listar_segmentos(dir)?;
    let mut esc = Escaneo { avisos: vec![], ultima: despues, corte: None, hueco: false, fin_ultimo_segmento: None, segmentos: vec![] };
    let mut esperada = despues + 1;
    'segs: for (i, seg) in segmentos.iter().enumerate() {
        if segmentos.get(i + 1).is_some_and(|s| s.inicio <= esperada) {
            continue; // todo lo suyo es anterior a lo que busco
        }
        let bytes = fs::read(&seg.ruta)?;
        let nombre = seg.ruta.file_name().unwrap().to_string_lossy().into_owned();
        let mut fin_seg = seg.inicio.saturating_sub(1);
        let mut off = 0usize;
        while off < bytes.len() {
            match decodificar(&bytes, off) {
                Ok((e, sig)) => {
                    if e.secuencia < esperada {
                        // anterior a la foto: se salta, pero cuenta como contenido del segmento
                        fin_seg = e.secuencia;
                        off = sig;
                        continue;
                    }
                    if e.secuencia != esperada {
                        esc.avisos.push(Aviso::Hueco { esperada, hallada: e.secuencia });
                        esc.hueco = true;
                        break 'segs;
                    }
                    esperada += 1;
                    esc.ultima = e.secuencia;
                    fin_seg = e.secuencia;
                    off = sig;
                    sink(e);
                }
                Err(f) => {
                    let (segmento, desplazamiento, descartados) = (nombre, off as u64, (bytes.len() - off) as u64);
                    esc.avisos.push(match f {
                        Fallo::Truncado => Aviso::RegistroTruncado { segmento, desplazamiento, descartados },
                        Fallo::Corrupto => Aviso::RegistroCorrupto { segmento, desplazamiento, descartados },
                    });
                    esc.corte = Some((i, off as u64));
                    esc.fin_ultimo_segmento = Some(fin_seg);
                    break 'segs;
                }
            }
        }
        if i + 1 == segmentos.len() {
            esc.fin_ultimo_segmento = Some(fin_seg);
        }
    }
    esc.segmentos = segmentos;
    Ok(esc)
}

// --- Escritor ---

/// Escritor del registro. Tras un `Err` de escritura el estado interno queda dudoso: hay que volver a `abrir`.
pub struct Registro {
    dir: PathBuf,
    max_segmento: u64,
    sincronia: Sincronia,
    archivo: Option<File>,
    /// Primera secuencia del segmento activo (su nombre).
    inicio_activo: u64,
    /// Bytes ya volcados al segmento activo.
    tam: u64,
    siguiente: u64,
    buf: Vec<u8>,
    sin_sincronizar: bool,
    ultimo_sync: Instant,
}

impl Registro {
    /// Abre el directorio (creándolo), repara la cola dañada y deja el escritor listo. Reparar significa truncar el
    /// segmento en el primer registro dañado y apartar (`.descartado`) los segmentos posteriores, que ya no se
    /// pueden reproducir. Devuelve los avisos de esa reparación.
    pub fn abrir(dir: &Path, max_segmento: u64, sincronia: Sincronia) -> io::Result<(Registro, Vec<Aviso>)> {
        fs::create_dir_all(dir)?;
        // Nunca se reutiliza una secuencia que ya cubre una foto, aunque se haya perdido la cola del registro.
        let desde_foto = foto::listar(dir)?.last().map_or(0, |f| f.0);
        let mut esc = escanear(dir, desde_foto, &mut |_| {})?;
        if esc.hueco {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("hueco en el registro: {:?}", esc.avisos)));
        }
        let mut avisos = std::mem::take(&mut esc.avisos);
        if let Some((i, off)) = esc.corte {
            OpenOptions::new().write(true).open(&esc.segmentos[i].ruta)?.set_len(off)?;
            for s in &esc.segmentos[i + 1..] {
                let mut nuevo = s.ruta.clone().into_os_string();
                nuevo.push(".descartado");
                fs::rename(&s.ruta, &nuevo)?;
                avisos.push(Aviso::SegmentoDescartado { archivo: s.ruta.file_name().unwrap().to_string_lossy().into_owned() });
            }
            esc.segmentos.truncate(i + 1);
        }
        let siguiente = esc.ultima + 1;
        let activo = match (esc.segmentos.last(), esc.fin_ultimo_segmento) {
            (Some(s), Some(fin)) if fin + 1 == siguiente => Some((s.inicio, s.ruta.clone())),
            _ => None, // sin segmentos, o el último termina antes de una foto: segmento nuevo
        };
        let mut r = Registro {
            dir: dir.to_path_buf(),
            max_segmento: max_segmento.max(1),
            sincronia,
            archivo: None,
            inicio_activo: 0,
            tam: 0,
            siguiente,
            buf: Vec::new(),
            sin_sincronizar: false,
            ultimo_sync: Instant::now(),
        };
        if let Some((inicio, ruta)) = activo {
            r.archivo = Some(OpenOptions::new().append(true).open(&ruta)?);
            r.tam = fs::metadata(&ruta)?.len();
            r.inicio_activo = inicio;
        }
        Ok((r, avisos))
    }

    /// Secuencia que recibirá la próxima entrada.
    pub fn siguiente_secuencia(&self) -> u64 {
        self.siguiente
    }

    /// Añade una entrada y devuelve su secuencia.
    pub fn anadir(&mut self, instante: Instante, tipo: u16, datos: &[u8]) -> io::Result<u64> {
        self.anadir_lote([(instante, tipo, datos)])
    }

    /// Añade varias entradas con un solo volcado (y un solo `fsync` según la política). Devuelve la secuencia de la
    /// última (la anterior a la siguiente si el lote va vacío).
    pub fn anadir_lote<'a>(&mut self, entradas: impl IntoIterator<Item = (Instante, u16, &'a [u8])>) -> io::Result<u64> {
        for (instante, tipo, datos) in entradas {
            self.empujar(instante, tipo, datos)?;
        }
        self.confirmar()?;
        Ok(self.siguiente - 1)
    }

    /// Prepara una entrada sin volcarla y dice dónde quedará en disco; [`Registro::confirmar`] la escribe (con las demás preparadas).
    pub fn anadir_ubicado(&mut self, instante: Instante, tipo: u16, datos: &[u8]) -> io::Result<Ubicacion> {
        self.empujar(instante, tipo, datos)
    }

    /// Vuelca lo preparado con un solo `write` y hace el `fsync` si la política lo pide.
    pub fn confirmar(&mut self) -> io::Result<()> {
        self.volcar()?;
        let toca = match self.sincronia {
            Sincronia::Nunca => false,
            Sincronia::PorLote => true,
            Sincronia::Intervalo(d) => self.ultimo_sync.elapsed() >= d,
        };
        if toca {
            self.fsync()?;
        }
        Ok(())
    }

    /// Fuerza lo escrito a disco, sea cual sea la política.
    pub fn sincronizar(&mut self) -> io::Result<()> {
        self.volcar()?;
        self.fsync()
    }

    fn empujar(&mut self, instante: Instante, tipo: u16, datos: &[u8]) -> io::Result<Ubicacion> {
        if datos.len() > MAX_DATOS {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "entrada demasiado grande"));
        }
        if self.archivo.is_some() && self.tam + self.buf.len() as u64 >= self.max_segmento {
            self.volcar()?;
            self.fsync()?;
            self.archivo = None; // rotación
        }
        if self.archivo.is_none() {
            let ruta = self.dir.join(nombre_segmento(self.siguiente));
            self.archivo = Some(OpenOptions::new().create(true).append(true).open(ruta)?);
            self.tam = 0;
            self.inicio_activo = self.siguiente;
        }
        let u = Ubicacion { segmento: self.inicio_activo, desplazamiento: self.tam + self.buf.len() as u64 };
        codificar(&mut self.buf, self.siguiente, instante, tipo, datos);
        self.siguiente += 1;
        Ok(u)
    }

    fn volcar(&mut self) -> io::Result<()> {
        if self.buf.is_empty() {
            return Ok(());
        }
        let f = self.archivo.as_mut().expect("hay segmento activo si hay bytes pendientes");
        f.write_all(&self.buf)?;
        self.tam += self.buf.len() as u64;
        self.buf.clear();
        self.sin_sincronizar = true;
        Ok(())
    }

    fn fsync(&mut self) -> io::Result<()> {
        if self.sin_sincronizar {
            if let Some(f) = &self.archivo {
                f.sync_data()?;
            }
            self.sin_sincronizar = false;
        }
        self.ultimo_sync = Instant::now();
        Ok(())
    }
}
