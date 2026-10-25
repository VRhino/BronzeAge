//! Serialización canónica del estado de tecnología del mundo (`tecnologia::EstadoTecnologia`) para el hash y las fotos (desde BAN36, tanda 6B). La estructura se desestructura **entera**
//! (sin `..`): si el crate `tecnologia` añade un campo, esto deja de compilar y obliga a decidir cómo se guarda.
//!
//! Formato (little endian, el de `codec`): `era u8 · era_desde i64 · contadores{24 × u64} · restos{24 × u64} · logros{n u64 · n × opt_i64} · primeros{n u64 · n × (opt: facción u32, en i64)} ·
//! cantadas u64 · por_faccion{n u64 · n × (facción u32, aparecidas u64, adoptadas u64, reveladas u64)}`. Las tecnologías van por índice del catálogo (el orden del bloque `TECNOLOGIAS`, que
//! fija la huella de las reglas) y los conjuntos, como mapas de bits.

use crate::codec::{ErrorFormato, Escritor, Lector};
use tecnologia::estado::Primero;
use tecnologia::{Conjunto, EstadoTecnologia, TecnologiasFaccion};

pub(crate) fn escribir(w: &mut Escritor, e: &EstadoTecnologia) {
    let EstadoTecnologia { era, era_desde, contadores, restos, logros, primeros, cantadas, por_faccion } = e;
    w.u8(*era);
    w.i64(*era_desde);
    contadores.iter().chain(restos).for_each(|&n| w.u64(n));
    w.u64(logros.len() as u64);
    logros.iter().for_each(|&l| w.opt_i64(l));
    w.u64(primeros.len() as u64);
    for p in primeros {
        w.bool(p.is_some());
        if let Some(Primero { faccion, en }) = p {
            w.u32(*faccion);
            w.i64(*en);
        }
    }
    w.u64(cantadas.0);
    w.u64(por_faccion.len() as u64);
    for (&f, &TecnologiasFaccion { aparecidas, adoptadas, reveladas }) in por_faccion {
        w.u32(f);
        [aparecidas, adoptadas, reveladas].iter().for_each(|c| w.u64(c.0));
    }
}

pub(crate) fn leer(r: &mut Lector) -> Result<EstadoTecnologia, ErrorFormato> {
    let (era, era_desde) = (r.u8()?, r.i64()?);
    let (mut contadores, mut restos) = ([0u64; 24], [0u64; 24]);
    for n in contadores.iter_mut().chain(restos.iter_mut()) {
        *n = r.u64()?;
    }
    let logros = (0..r.cuenta()?).map(|_| r.opt_i64()).collect::<Result<Vec<_>, _>>()?;
    let mut primeros = Vec::new();
    for _ in 0..r.cuenta()? {
        primeros.push(if r.bool()? { Some(Primero { faccion: r.u32()?, en: r.i64()? }) } else { None });
    }
    let cantadas = Conjunto(r.u64()?);
    let mut por_faccion = std::collections::BTreeMap::new();
    for _ in 0..r.cuenta()? {
        let f = r.u32()?;
        por_faccion.insert(f, TecnologiasFaccion { aparecidas: Conjunto(r.u64()?), adoptadas: Conjunto(r.u64()?), reveladas: Conjunto(r.u64()?) });
    }
    Ok(EstadoTecnologia { era, era_desde, contadores, restos, logros, primeros, cantadas, por_faccion })
}
