//! Fotos: instantánea opaca del estado hasta una `secuencia`, comprimida con zstd, con hash sha256.
//!
//! Fichero `foto-<secuencia>.zst`: `"BAF1" | secuencia u64 | instante i64 | sha256 [32] | zstd(datos)`.
//! El hash cubre `secuencia | instante | datos`, de modo que cualquier byte dañado se detecta.

use base::Instante;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const MAGIA: &[u8; 4] = b"BAF1";
const CABECERA: usize = 4 + 8 + 8 + 32;
const NIVEL_ZSTD: i32 = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Foto {
    /// Última secuencia del registro que ya está reflejada en `datos`.
    pub secuencia: u64,
    pub instante: Instante,
    pub hash: [u8; 32],
    pub datos: Vec<u8>,
}

fn calcular_hash(secuencia: u64, instante: Instante, datos: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(secuencia.to_le_bytes());
    h.update(instante.to_le_bytes());
    h.update(datos);
    h.finalize().into()
}

impl Foto {
    pub fn nueva(secuencia: u64, instante: Instante, datos: Vec<u8>) -> Foto {
        let hash = calcular_hash(secuencia, instante, &datos);
        Foto { secuencia, instante, hash, datos }
    }
}

fn nombre(secuencia: u64) -> String {
    format!("foto-{secuencia:020}.zst")
}

/// Fotos del directorio `(secuencia, ruta)`, de la más antigua a la más reciente.
pub(crate) fn listar(dir: &Path) -> io::Result<Vec<(u64, PathBuf)>> {
    let mut v = Vec::new();
    for e in fs::read_dir(dir)? {
        let e = e?;
        let n = e.file_name();
        let Some(n) = n.to_str() else { continue };
        if let Some(num) = n.strip_prefix("foto-").and_then(|s| s.strip_suffix(".zst")) {
            if let Ok(s) = num.parse::<u64>() {
                v.push((s, e.path()));
            }
        }
    }
    v.sort();
    Ok(v)
}

/// Escritura atómica: fichero temporal, `sync_all`, renombrar. Un corte deja la foto vieja o la nueva, nunca media.
pub(crate) fn escribir(dir: &Path, f: &Foto) -> io::Result<()> {
    let comprimido = zstd::bulk::compress(&f.datos, NIVEL_ZSTD)?;
    let tmp = dir.join(format!("foto-{:020}.tmp", f.secuencia));
    let mut fich = File::create(&tmp)?;
    fich.write_all(MAGIA)?;
    fich.write_all(&f.secuencia.to_le_bytes())?;
    fich.write_all(&f.instante.to_le_bytes())?;
    fich.write_all(&f.hash)?;
    fich.write_all(&comprimido)?;
    fich.sync_all()?;
    drop(fich);
    // ponytail: no se hace fsync del directorio (en Windows no se puede abrir); en Unix añadirlo si importa la
    // durabilidad del propio renombrado ante un corte de corriente.
    fs::rename(&tmp, dir.join(nombre(f.secuencia)))
}

/// Lee y valida (cabecera, descompresión, hash y nombre). El error es el motivo para el aviso.
pub(crate) fn leer(ruta: &Path) -> Result<Foto, String> {
    let bytes = fs::read(ruta).map_err(|e| e.to_string())?;
    if bytes.len() < CABECERA || &bytes[0..4] != MAGIA {
        return Err("cabecera inválida".into());
    }
    let secuencia = u64::from_le_bytes(bytes[4..12].try_into().unwrap());
    let instante = i64::from_le_bytes(bytes[12..20].try_into().unwrap());
    let hash: [u8; 32] = bytes[20..52].try_into().unwrap();
    let datos = zstd::stream::decode_all(&bytes[CABECERA..]).map_err(|e| format!("zstd: {e}"))?;
    if calcular_hash(secuencia, instante, &datos) != hash {
        return Err("hash sha256 no coincide".into());
    }
    if ruta.file_name().and_then(|n| n.to_str()) != Some(nombre(secuencia).as_str()) {
        return Err("la secuencia no coincide con el nombre del fichero".into());
    }
    Ok(Foto { secuencia, instante, hash, datos })
}

/// Borra las fotos más antiguas dejando `conservar`.
pub(crate) fn podar(dir: &Path, conservar: usize) -> io::Result<()> {
    let fotos = listar(dir)?;
    for (_, ruta) in &fotos[..fotos.len().saturating_sub(conservar)] {
        fs::remove_file(ruta)?;
    }
    Ok(())
}
