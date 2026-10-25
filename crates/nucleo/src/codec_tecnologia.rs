//! Serialización canónica del estado de tecnología del mundo (`tecnologia::EstadoTecnologia`) para el hash y las fotos (desde BAN36, tanda 6B). La estructura se desestructura **entera**
//! (sin `..`): si el crate `tecnologia` añade un campo, esto deja de compilar y obliga a decidir cómo se guarda.
//!
//! Formato (little endian, el de `codec`): `era u8 · era_desde i64 · contadores{24 × u64} · restos{24 × u64} · logros{n u64 · n × opt_i64} · primeros{n u64 · n × (opt: facción u32, en i64)} ·
//! cantadas u64 · por_faccion{n u64 · n × (facción u32, aparecidas u64, adoptadas u64, reveladas u64)}`. Las tecnologías van por índice del catálogo (el orden del bloque `TECNOLOGIAS`, que
//! fija la huella de las reglas) y los conjuntos, como mapas de bits.

use crate::codec::{ErrorFormato, Escritor, Lector};
use tecnologia::estado::Primero;
use tecnologia::itinerantes::AedaItinerante;
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

/// Los Aedas itinerantes (desde BAN39, tanda 6D), en el orden de su número, desestructurados enteros: `n u64 · n × (id u32, posición, en opt u64, hasta opt i64, destino opt u64,
/// plan (opt: el de `codec_columna`), recientes{n u64 · n × u32})`.
pub(crate) fn escribir_aedas(w: &mut Escritor, aedas: &[AedaItinerante]) {
    w.u64(aedas.len() as u64);
    for AedaItinerante { id, posicion, en, hasta, destino, plan, recientes } in aedas {
        w.u32(*id);
        w.punto(*posicion);
        w.opt_u64(en.map(u64::from));
        w.opt_i64(*hasta);
        w.opt_u64(destino.map(u64::from));
        w.bool(plan.is_some());
        if let Some(p) = plan {
            crate::codec_columna::plan_escribir(w, p);
        }
        w.u64(recientes.len() as u64);
        recientes.iter().for_each(|&p| w.u32(p));
    }
}

pub(crate) fn leer_aedas(r: &mut Lector) -> Result<Vec<AedaItinerante>, ErrorFormato> {
    let mut aedas = Vec::new();
    for _ in 0..r.cuenta()? {
        let (id, posicion) = (r.u32()?, r.punto()?);
        let (en, hasta, destino) = (r.opt_u64()?.map(|p| p as u32), r.opt_i64()?, r.opt_u64()?.map(|p| p as u32));
        let plan = if r.bool()? { Some(crate::codec_columna::plan_leer(r)?) } else { None };
        let recientes = (0..r.cuenta()?).map(|_| r.u32()).collect::<Result<Vec<_>, _>>()?;
        aedas.push(AedaItinerante { id, posicion, en, hasta, destino, plan, recientes });
    }
    Ok(aedas)
}
