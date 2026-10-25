//! Diario: un [`Registro`] de **salidas deterministas** (la crónica pública del núcleo) con un índice en memoria de clave → ubicación, para leerlo por páginas sin recorrerlo.
//!
//! - Cada entrada lleva una **clave** creciente (la versión del mundo que la produjo; varias entradas pueden compartirla) en los 8 primeros bytes de sus datos; su secuencia es la del registro
//!   (su propio contador, sin huecos).
//! - **No es la verdad del mundo**: se recompone repitiendo el registro de órdenes desde una foto. Por eso al abrirlo se **truncan las entradas con clave mayor que la de la foto** que se carga,
//!   y quien lo escribe las vuelve a producir idénticas: ni duplicadas ni perdidas, se corte donde se corte.
//! - **Índice**: una marca (clave, secuencia, ubicación) cada [`CADA`] entradas. Leer una página cuesta leer, como mucho, la página más dos bloques.
//! - **Páginas por claves enteras**: una página nunca parte las entradas de una clave (si la primera no cabe, va entera igualmente), así que pedir la siguiente con la clave de al lado no repite
//!   ni salta ninguna.

use crate::registro::{decodificar, listar_segmentos, nombre_segmento, Fallo, Ubicacion};
use crate::{Aviso, Registro, Sincronia};
use base::Instante;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Entradas entre dos marcas del índice.
pub const CADA: u64 = 256;
const TIPO: u16 = 1;

/// Una entrada del diario.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Apunte {
    pub secuencia: u64,
    pub clave: u64,
    pub instante: Instante,
    pub datos: Vec<u8>,
}

/// Una página: las entradas en orden y si quedan más antes o después de ellas.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pagina {
    pub apuntes: Vec<Apunte>,
    pub mas_antiguas: bool,
    pub mas_recientes: bool,
}

struct Marca {
    clave: u64,
    secuencia: u64,
    en: Ubicacion,
}

pub struct Diario {
    dir: PathBuf,
    registro: Registro,
    marcas: Vec<Marca>,
    total: u64,
}

impl Diario {
    /// Abre (o crea) el diario de `dir`, repara su cola dañada como el registro (avisa) y **trunca las entradas con clave mayor que `hasta`** (la versión de la foto que se carga). Lo que
    /// queda se indexa leyéndolo una vez. Si no existía y `hasta > 0` (se esperaba historia), lo avisa.
    pub fn abrir(dir: &Path, max_segmento: u64, hasta: u64) -> io::Result<(Diario, Vec<Aviso>)> {
        let perdido = (hasta > 0 && !dir.exists()).then(|| Aviso::DiarioPerdido { directorio: dir.display().to_string() });
        let (registro, mut avisos) = Registro::abrir(dir, max_segmento, Sincronia::Nunca)?;
        avisos.extend(perdido);
        drop(registro); // suelta el segmento activo antes de truncar (en Windows no se borra un fichero abierto)
        let mut marcas = Vec::new();
        let mut total = 0;
        let mut corte = None;
        'segs: for seg in listar_segmentos(dir)? {
            let bytes = fs::read(&seg.ruta)?;
            let mut off = 0;
            while off < bytes.len() {
                let (e, sig) = decodificar(&bytes, off).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "diario dañado tras repararlo"))?;
                let en = Ubicacion { segmento: seg.inicio, desplazamiento: off as u64 };
                let clave = clave_de(&e.datos);
                if clave > hasta {
                    corte = Some(en);
                    break 'segs;
                }
                total = e.secuencia;
                if (total - 1).is_multiple_of(CADA) {
                    marcas.push(Marca { clave, secuencia: total, en });
                }
                off = sig;
            }
        }
        if let Some(c) = corte {
            OpenOptions::new().write(true).open(dir.join(nombre_segmento(c.segmento)))?.set_len(c.desplazamiento)?;
            for s in listar_segmentos(dir)?.into_iter().filter(|s| s.inicio > c.segmento) {
                fs::remove_file(&s.ruta)?;
            }
        }
        let (registro, _) = Registro::abrir(dir, max_segmento, Sincronia::Nunca)?;
        debug_assert_eq!(registro.siguiente_secuencia(), total + 1);
        Ok((Diario { dir: dir.to_path_buf(), registro, marcas, total }, avisos))
    }

    /// Entradas guardadas (la secuencia de la última).
    pub fn total(&self) -> u64 {
        self.total
    }

    /// Prepara una entrada (la escribe [`Diario::confirmar`]). Las claves no bajan.
    pub fn anadir(&mut self, clave: u64, instante: Instante, datos: &[u8]) -> io::Result<()> {
        let mut d = Vec::with_capacity(8 + datos.len());
        d.extend_from_slice(&clave.to_le_bytes());
        d.extend_from_slice(datos);
        let en = self.registro.anadir_ubicado(instante, TIPO, &d)?;
        self.total += 1;
        if (self.total - 1).is_multiple_of(CADA) {
            debug_assert!(self.marcas.last().is_none_or(|m| m.clave <= clave));
            self.marcas.push(Marca { clave, secuencia: self.total, en });
        }
        Ok(())
    }

    /// Escribe lo preparado (sin `fsync`).
    pub fn confirmar(&mut self) -> io::Result<()> {
        self.registro.confirmar()
    }

    /// Escribe lo preparado y lo fuerza a disco.
    pub fn sincronizar(&mut self) -> io::Result<()> {
        self.registro.sincronizar()
    }

    /// Una página de como mucho `limite` entradas (al menos las de una clave), por claves enteras:
    /// - con `desde`, hacia delante: las primeras con clave ≥ `desde` (y ≤ `hasta`, si lo hay);
    /// - sin `desde`, hacia atrás: las últimas con clave ≤ `hasta` (sin `hasta`, las últimas de todas).
    pub fn pagina(&self, desde: Option<u64>, hasta: Option<u64>, limite: u64) -> io::Result<Pagina> {
        let limite = limite.max(1);
        let ini = match desde {
            Some(d) => self.primera_desde(d)?,
            None => 1,
        };
        let fin = match hasta {
            Some(h) => self.ultima_hasta(h)?,
            None => self.total,
        };
        if fin < ini {
            return Ok(Pagina { apuntes: vec![], mas_antiguas: ini.min(fin + 1) > 1, mas_recientes: ini.max(fin + 1) <= self.total });
        }
        let apuntes = if desde.is_some() {
            // La primera clave entera; si cabe, hasta `limite` sin partir la última.
            let fin_primera = self.ultima_hasta(self.clave(ini)?)?.min(fin);
            if fin_primera + 1 - ini >= limite {
                self.leer(ini, fin_primera)?
            } else {
                let b = (ini + limite - 1).min(fin);
                let mut v = self.leer(ini, (b + 1).min(fin))?;
                if v.len() as u64 > b + 1 - ini {
                    let partida = v.pop().expect("leída").clave;
                    v.retain(|a| a.clave != partida);
                }
                v
            }
        } else {
            let ini_ultima = self.primera_desde(self.clave(fin)?)?.max(ini);
            if fin + 1 - ini_ultima >= limite {
                self.leer(ini_ultima, fin)?
            } else {
                let a = (fin + 1).saturating_sub(limite).max(ini);
                let mut v = self.leer(a.saturating_sub(1).max(ini), fin)?;
                if v.len() as u64 > fin + 1 - a {
                    let partida = v.remove(0).clave;
                    v.retain(|x| x.clave != partida);
                }
                v
            }
        };
        let (primera, ultima) = (apuntes[0].secuencia, apuntes[apuntes.len() - 1].secuencia);
        Ok(Pagina { apuntes, mas_antiguas: primera > 1, mas_recientes: ultima < self.total })
    }

    /// La clave de la entrada `s`.
    fn clave(&self, s: u64) -> io::Result<u64> {
        Ok(self.leer(s, s)?[0].clave)
    }

    /// La primera entrada con clave ≥ `k` (`total + 1` si no hay).
    fn primera_desde(&self, k: u64) -> io::Result<u64> {
        let b = self.marcas.partition_point(|m| m.clave < k);
        if b == 0 {
            return Ok(1);
        }
        let fin_bloque = self.marcas.get(b).map_or(self.total, |m| m.secuencia - 1);
        let bloque = self.leer(self.marcas[b - 1].secuencia, fin_bloque)?;
        Ok(bloque.iter().find(|a| a.clave >= k).map_or(fin_bloque + 1, |a| a.secuencia))
    }

    /// La última entrada con clave ≤ `k` (0 si no hay).
    fn ultima_hasta(&self, k: u64) -> io::Result<u64> {
        let b = self.marcas.partition_point(|m| m.clave <= k);
        if b == 0 {
            return Ok(0);
        }
        let fin_bloque = self.marcas.get(b).map_or(self.total, |m| m.secuencia - 1);
        let bloque = self.leer(self.marcas[b - 1].secuencia, fin_bloque)?;
        Ok(bloque.iter().rev().find(|a| a.clave <= k).expect("la marca la cumple").secuencia)
    }

    /// Las entradas `a..=b` (1 ≤ a ≤ b ≤ total), desde la marca anterior a `a`, pasando de segmento cuando se acaba uno.
    fn leer(&self, a: u64, b: u64) -> io::Result<Vec<Apunte>> {
        let m = &self.marcas[((a - 1) / CADA) as usize];
        let mut v = Vec::with_capacity((b + 1 - a) as usize);
        let (mut seg, mut off, mut s) = (m.en.segmento, m.en.desplazamiento, m.secuencia);
        let mut buf = Vec::new();
        while s <= b {
            let mut f = BufReader::new(File::open(self.dir.join(nombre_segmento(seg)))?);
            f.seek(SeekFrom::Start(off))?;
            let antes = s;
            while s <= b {
                buf.resize(8, 0);
                match f.read_exact(&mut buf) {
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                    r => r?,
                }
                let len = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
                buf.resize(8 + len.min(crate::MAX_DATOS + 18), 0);
                f.read_exact(&mut buf[8..])?;
                let (e, _) = decodificar(&buf, 0).map_err(|f| io::Error::new(io::ErrorKind::InvalidData, match f {
                    Fallo::Truncado => "entrada del diario incompleta",
                    Fallo::Corrupto => "entrada del diario dañada",
                }))?;
                debug_assert_eq!(e.secuencia, s);
                if s >= a {
                    v.push(Apunte { secuencia: s, clave: clave_de(&e.datos), instante: e.instante, datos: e.datos[8..].to_vec() });
                }
                s += 1;
            }
            if s == antes {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("falta la entrada {s} del diario")));
            }
            (seg, off) = (s, 0);
        }
        Ok(v)
    }
}

fn clave_de(datos: &[u8]) -> u64 {
    u64::from_le_bytes(datos[..8].try_into().expect("una entrada del diario lleva su clave"))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// Entradas con claves 1, 1, 2, 3, 3, 3, 4…: la clave `k` lleva `k % 3 + 1` entradas.
    fn llenar(d: &mut Diario, claves: std::ops::RangeInclusive<u64>) -> Vec<(u64, Vec<u8>)> {
        let mut todo = Vec::new();
        for k in claves {
            for j in 0..(k % 3 + 1) {
                let datos = format!("{k}-{j}").into_bytes();
                d.anadir(k, k as i64 * 10, &datos).unwrap();
                todo.push((k, datos));
            }
            if k.is_multiple_of(50) {
                d.confirmar().unwrap();
            }
        }
        d.confirmar().unwrap();
        todo
    }

    fn de(p: &Pagina) -> Vec<(u64, Vec<u8>)> {
        p.apuntes.iter().map(|a| (a.clave, a.datos.clone())).collect()
    }

    /// Lo que debe devolver una página: el mismo recorte hecho sobre la lista entera.
    fn esperada(todo: &[(u64, Vec<u8>)], desde: Option<u64>, hasta: Option<u64>, limite: usize) -> Vec<(u64, Vec<u8>)> {
        let sel: Vec<_> = todo.iter().filter(|(k, _)| desde.is_none_or(|d| *k >= d) && hasta.is_none_or(|h| *k <= h)).cloned().collect();
        let mut grupos: Vec<Vec<(u64, Vec<u8>)>> = Vec::new();
        for e in sel {
            match grupos.last_mut() {
                Some(g) if g[0].0 == e.0 => g.push(e),
                _ => grupos.push(vec![e]),
            }
        }
        if desde.is_none() {
            grupos.reverse();
        }
        let mut out: Vec<Vec<(u64, Vec<u8>)>> = Vec::new();
        let mut n = 0;
        for g in grupos {
            if !out.is_empty() && n + g.len() > limite {
                break;
            }
            n += g.len();
            out.push(g);
        }
        if desde.is_none() {
            out.reverse();
        }
        out.concat()
    }

    #[test]
    fn las_paginas_recortan_por_claves_enteras_hacia_atras_y_hacia_delante() {
        let dir = tempfile::tempdir().unwrap();
        let (mut d, _) = Diario::abrir(dir.path(), 4 << 10, 0).unwrap();
        let todo = llenar(&mut d, 1..=700);
        assert!(listar_segmentos(dir.path()).unwrap().len() > 3, "la lectura cruza segmentos");
        for limite in [1, 2, 3, 5, 200, 1_000, 5_000] {
            for (desde, hasta) in [(None, None), (None, Some(1)), (None, Some(300)), (Some(1), None), (Some(299), None), (Some(299), Some(310)), (Some(700), None), (Some(701), None), (None, Some(0)), (Some(400), Some(399))] {
                let p = d.pagina(desde, hasta, limite).unwrap();
                let e = esperada(&todo, desde, hasta, limite as usize);
                assert_eq!(de(&p), e, "desde {desde:?} hasta {hasta:?} límite {limite}");
                if let (Some(a), Some(z)) = (p.apuntes.first(), p.apuntes.last()) {
                    assert_eq!((p.mas_antiguas, p.mas_recientes), (a.secuencia > 1, z.secuencia < d.total()));
                }
            }
        }
        // Recorrer hacia atrás con `hasta = primera - 1` y hacia delante con `desde = última + 1` da todo, sin repetir ni saltar.
        for limite in [1, 7, 200] {
            let (mut atras, mut hasta) = (Vec::new(), None);
            loop {
                let p = d.pagina(None, hasta, limite).unwrap();
                atras.splice(0..0, de(&p));
                if !p.mas_antiguas {
                    break;
                }
                hasta = Some(p.apuntes[0].clave - 1);
            }
            assert_eq!(atras, todo, "hacia atrás con límite {limite}");
            let (mut delante, mut desde) = (Vec::new(), 0);
            loop {
                let p = d.pagina(Some(desde), None, limite).unwrap();
                delante.extend(de(&p));
                if !p.mas_recientes {
                    break;
                }
                desde = p.apuntes.last().unwrap().clave + 1;
            }
            assert_eq!(delante, todo, "hacia delante con límite {limite}");
        }
    }

    #[test]
    fn al_abrir_se_trunca_lo_posterior_a_la_foto_y_se_sigue_sin_huecos() {
        let dir = tempfile::tempdir().unwrap();
        let (mut d, _) = Diario::abrir(dir.path(), 4 << 10, 0).unwrap();
        let todo = llenar(&mut d, 1..=600);
        drop(d);
        let (mut d, avisos) = Diario::abrir(dir.path(), 4 << 10, 400).unwrap();
        assert!(avisos.is_empty(), "{avisos:?}");
        let quedan: Vec<_> = todo.iter().filter(|(k, _)| *k <= 400).cloned().collect();
        assert_eq!(d.total(), quedan.len() as u64);
        assert_eq!(de(&d.pagina(Some(0), None, 10_000).unwrap()), quedan);
        // Al volver a producir lo posterior, queda exactamente lo de antes.
        llenar(&mut d, 401..=600);
        drop(d);
        let (d, _) = Diario::abrir(dir.path(), 4 << 10, u64::MAX).unwrap();
        assert_eq!(de(&d.pagina(Some(0), None, 10_000).unwrap()), todo);
        // Truncar a nada lo vacía.
        drop(d);
        let (d, _) = Diario::abrir(dir.path(), 4 << 10, 0).unwrap();
        assert_eq!((d.total(), d.pagina(None, None, 200).unwrap()), (0, Pagina::default()));
        // Uno que no estaba cuando se esperaba historia se crea vacío y lo avisa.
        let (d, avisos) = Diario::abrir(&dir.path().join("otro"), 4 << 10, 7).unwrap();
        assert!(d.total() == 0 && matches!(avisos[..], [Aviso::DiarioPerdido { .. }]), "{avisos:?}");
    }

    #[test]
    fn una_cola_cortada_se_repara_y_avisa() {
        let dir = tempfile::tempdir().unwrap();
        let (mut d, _) = Diario::abrir(dir.path(), 64 << 20, 0).unwrap();
        let todo = llenar(&mut d, 1..=20);
        drop(d);
        let seg = listar_segmentos(dir.path()).unwrap().pop().unwrap().ruta;
        let len = fs::metadata(&seg).unwrap().len();
        OpenOptions::new().write(true).open(&seg).unwrap().set_len(len - 3).unwrap();
        let (d, avisos) = Diario::abrir(dir.path(), 64 << 20, u64::MAX).unwrap();
        assert!(matches!(avisos[..], [Aviso::RegistroTruncado { .. }]), "{avisos:?}");
        assert_eq!(de(&d.pagina(Some(0), None, 1_000).unwrap()), todo[..todo.len() - 1]);
    }

    /// 100.000 entradas del tamaño de una de la crónica: lo que cuesta escribirlas, abrir el diario (indexarlo) y leer una página. En release:
    /// `cargo test --release -p persistencia historial_largo -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn historial_largo() {
        use std::time::Instant;
        let dir = tempfile::tempdir().unwrap();
        let (mut d, _) = Diario::abrir(dir.path(), 64 << 20, 0).unwrap();
        let datos = vec![7u8; 120];
        let t = Instant::now();
        for i in 0..100_000u64 {
            d.anadir(i / 2 + 1, i as i64, &datos).unwrap();
            if i % 8 == 7 {
                d.confirmar().unwrap(); // un lote cada pocas versiones, como el hilo de la persistencia
            }
        }
        d.sincronizar().unwrap();
        let escribir = t.elapsed();
        let bytes: u64 = listar_segmentos(dir.path()).unwrap().iter().map(|s| fs::metadata(&s.ruta).unwrap().len()).sum();
        drop(d);
        let t = Instant::now();
        let (d, _) = Diario::abrir(dir.path(), 64 << 20, u64::MAX).unwrap();
        let abrir = t.elapsed();
        let medir = |desde, hasta, limite| {
            let t = Instant::now();
            let n = (0..100).map(|_| d.pagina(desde, hasta, limite).unwrap().apuntes.len()).sum::<usize>() / 100;
            (t.elapsed() / 100, n)
        };
        let ultimas = medir(None, None, 200);
        let mitad = medir(None, Some(25_000), 1_000);
        let principio = medir(Some(0), None, 200);
        println!("100.000 entradas, {bytes} bytes: escribir {escribir:?}, abrir e indexar {abrir:?}; página de las últimas 200 {:?} ({}), 1.000 hasta la mitad {:?} ({}), 200 desde el principio {:?} ({})", ultimas.0, ultimas.1, mitad.0, mitad.1, principio.0, principio.1);
        assert_eq!((ultimas.1, mitad.1, principio.1), (200, 1_000, 200));
    }
}
