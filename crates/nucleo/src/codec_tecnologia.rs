//! Serialización canónica del estado de tecnología del mundo (`tecnologia::EstadoTecnologia`) para el hash y las fotos (desde BAN36, tanda 6B). La estructura se desestructura **entera**
//! (sin `..`): si el crate `tecnologia` añade un campo, esto deja de compilar y obliga a decidir cómo se guarda.
//!
//! Formato (little endian, el de `codec`): `era u8 · era_desde i64 · contadores{24 × u64} · restos{24 × u64} · logros{n u64 · n × opt_i64} · primeros{n u64 · n × (opt: facción u32, en i64)} ·
//! cantadas u64 · por_faccion{n u64 · n × (facción u32, aparecidas u64, adoptadas u64, reveladas u64)}`. Las tecnologías van por índice del catálogo (el orden del bloque `TECNOLOGIAS`, que
//! fija la huella de las reglas) y los conjuntos, como mapas de bits.

use crate::codec::{ErrorFormato, Escritor, Lector};
use base::Instante;
use std::collections::BTreeMap;
use tecnologia::datos::HechoEpico;
use tecnologia::epicas::Clave;
use tecnologia::estado::Primero;
use tecnologia::{AedaResidente, Conjunto, Epica, EstadoTecnologia, Residentes, TecnologiasFaccion};

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

/// Los Aedas residentes (desde BAN40, tanda 6E), desestructurados enteros como el estado de tecnología. Formato: `siguiente u32 · aedas{n · (id u32, plaza u32, facción u32, llegada i64,
/// épica?: (tecnología u8, capítulo u8, hechos u32, último opt_i64, claves{n · clave}))} · espera{n · (plaza u32, i64)} · cumplidas{n · (facción u32, u32)} · citas{n · (plaza u64, i64)}`. Una
/// clave es su etiqueta (`u8`, el orden de `Clave`) y sus campos en orden.
pub(crate) fn escribir_residentes(w: &mut Escritor, r: &Residentes, citas: &BTreeMap<u64, Instante>) {
    let Residentes { aedas, siguiente, espera_desde, cumplidas } = r;
    w.u32(*siguiente);
    w.u64(aedas.len() as u64);
    for AedaResidente { id, plaza, faccion, llegada_en, epica } in aedas {
        w.u32(*id);
        w.u32(*plaza);
        w.u32(*faccion);
        w.i64(*llegada_en);
        w.bool(epica.is_some());
        if let Some(Epica { tec, capitulo, hechos, ultimo_hecho_en, claves }) = epica {
            w.u8(*tec);
            w.u8(*capitulo);
            w.u32(*hechos);
            w.opt_i64(*ultimo_hecho_en);
            w.u64(claves.len() as u64);
            claves.iter().for_each(|c| escribir_clave(w, c));
        }
    }
    w.u64(espera_desde.len() as u64);
    for (&p, &en) in espera_desde {
        w.u32(p);
        w.i64(en);
    }
    w.u64(cumplidas.len() as u64);
    for (&f, &n) in cumplidas {
        w.u32(f);
        w.u32(n);
    }
    w.u64(citas.len() as u64);
    for (&p, &en) in citas {
        w.u64(p);
        w.i64(en);
    }
}

fn escribir_clave(w: &mut Escritor, c: &Clave) {
    match *c {
        Clave::Defensa { plaza, en } => {
            w.u8(0);
            w.u32(plaza);
            w.i64(en);
        }
        Clave::Conquista { plaza, en } => {
            w.u8(1);
            w.u32(plaza);
            w.i64(en);
        }
        Clave::Batalla { a, b, en } => {
            w.u8(2);
            w.u64(a);
            w.u64(b);
            w.i64(en);
        }
        Clave::Unity { tipo, batalla } => {
            w.u8(3);
            w.u8(tipo as u8);
            w.u64(batalla);
        }
        Clave::Obra { plaza, edificio, nivel } => {
            w.u8(4);
            w.u32(plaza);
            w.u32(edificio);
            w.u8(nivel);
        }
        Clave::Ascenso { plaza, nivel } => {
            w.u8(5);
            w.u32(plaza);
            w.u8(nivel);
        }
        Clave::Adopcion { faccion, tec } => {
            w.u8(6);
            w.u32(faccion);
            w.u8(tec);
        }
        Clave::Caravana(id) => {
            w.u8(7);
            w.u64(id);
        }
    }
}

fn leer_clave(r: &mut Lector) -> Result<Clave, ErrorFormato> {
    Ok(match r.u8()? {
        0 => Clave::Defensa { plaza: r.u32()?, en: r.i64()? },
        1 => Clave::Conquista { plaza: r.u32()?, en: r.i64()? },
        2 => Clave::Batalla { a: r.u64()?, b: r.u64()?, en: r.i64()? },
        3 => Clave::Unity { tipo: *HechoEpico::TODOS.get(r.u8()? as usize).ok_or(ErrorFormato("hecho de épica desconocido"))?, batalla: r.u64()? },
        4 => Clave::Obra { plaza: r.u32()?, edificio: r.u32()?, nivel: r.u8()? },
        5 => Clave::Ascenso { plaza: r.u32()?, nivel: r.u8()? },
        6 => Clave::Adopcion { faccion: r.u32()?, tec: r.u8()? },
        7 => Clave::Caravana(r.u64()?),
        _ => return Err(ErrorFormato("clave de épica desconocida")),
    })
}

pub(crate) fn leer_residentes(r: &mut Lector) -> Result<(Residentes, BTreeMap<u64, Instante>), ErrorFormato> {
    let siguiente = r.u32()?;
    let mut aedas = Vec::new();
    for _ in 0..r.cuenta()? {
        let (id, plaza, faccion, llegada_en) = (r.u32()?, r.u32()?, r.u32()?, r.i64()?);
        let epica = if r.bool()? {
            let (tec, capitulo, hechos, ultimo_hecho_en) = (r.u8()?, r.u8()?, r.u32()?, r.opt_i64()?);
            let claves = (0..r.cuenta()?).map(|_| leer_clave(r)).collect::<Result<_, _>>()?;
            Some(Epica { tec, capitulo, hechos, ultimo_hecho_en, claves })
        } else {
            None
        };
        aedas.push(AedaResidente { id, plaza, faccion, llegada_en, epica });
    }
    let mut espera_desde = BTreeMap::new();
    for _ in 0..r.cuenta()? {
        espera_desde.insert(r.u32()?, r.i64()?);
    }
    let mut cumplidas = BTreeMap::new();
    for _ in 0..r.cuenta()? {
        cumplidas.insert(r.u32()?, r.u32()?);
    }
    let mut citas = BTreeMap::new();
    for _ in 0..r.cuenta()? {
        citas.insert(r.u64()?, r.i64()?);
    }
    Ok((Residentes { aedas, siguiente, espera_desde, cumplidas }, citas))
}
