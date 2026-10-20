//! Serialización canónica de las columnas (`Ejercito`) para el hash y las fotos (formato BAN11). La composición y el mando son de
//! `militar::columna::Columna`, que se desestructura **entera** (sin `..`): si el crate `militar` añade un campo, esto deja de compilar y
//! obliga a decidir cómo se guarda.
//!
//! Una columna (little endian, el de `codec`): `id u64 · facción opt_u64 · tipo u8 (0 personal, 1 ejército) · líder u64 · política u8 (0
//! rechazar, 1 aceptar, 2 preguntar) · estado u8 (0 marchando, 1 estacionada) · participantes{n u64 · n × (héroe u64, unido_en i64)} ·
//! peticiones{n u64 · n × (héroe u64, pedido_en i64, expira_en i64)} · formación_expira_en opt_i64 · escuadras{n u64 · n × u64} · carga{n u64 · n ×
//! (recurso u8, cantidad i64)} · víveres{valor i64, t0 i64, por_minuto i64, máximo opt_i64, mínimo opt_i64} · posición{x i64, y i64} ·
//! plan{generación u32 · n u32 · n × (punto, instante i64)} · velocidad_fija opt_i64 · caravana_fundación bool · caravanas_adjuntas{n u32 · n ×
//! u64} · persiguiendo opt_u64 · en_batalla bool · versión u64`.

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::codec_comercio::{carga_escribir, carga_leer};
use crate::estado::Ejercito;
use base::{Plan, Ritmo, Vertice};
use militar::columna::{Columna, Participante, Peticion, Politica, TipoColumna};
use militar::racion::Estado;

pub(crate) fn politica_a_u8(p: Politica) -> u8 {
    match p {
        Politica::Rechazar => 0,
        Politica::Aceptar => 1,
        Politica::Preguntar => 2,
    }
}

pub(crate) fn politica_de_u8(i: u8) -> Result<Politica, ErrorFormato> {
    Ok(match i {
        0 => Politica::Rechazar,
        1 => Politica::Aceptar,
        2 => Politica::Preguntar,
        _ => return Err(ErrorFormato("política de unión desconocida")),
    })
}

pub(crate) fn ritmo_escribir(w: &mut Escritor, r: &Ritmo) {
    let Ritmo { valor_en_t0, t0, por_minuto, maximo, minimo } = r;
    w.i64(*valor_en_t0);
    w.i64(*t0);
    w.i64(*por_minuto);
    w.opt_i64(*maximo);
    w.opt_i64(*minimo);
}

pub(crate) fn ritmo_leer(r: &mut Lector) -> Result<Ritmo, ErrorFormato> {
    Ok(Ritmo { valor_en_t0: r.i64()?, t0: r.i64()?, por_minuto: r.i64()?, maximo: r.opt_i64()?, minimo: r.opt_i64()? })
}

pub(crate) fn escribir(w: &mut Escritor, e: &Ejercito) {
    let Ejercito { id, columna, posicion, plan, velocidad_fija, caravana_fundacion, caravanas_adjuntas, persiguiendo, en_batalla, version } = e;
    let Columna { id: _, faccion, tipo, lider, politica, estado, participantes, peticiones, formacion_expira_en, escuadras, carga, viveres } = columna;
    w.u64(*id);
    w.opt_u64(faccion.map(u64::from));
    w.u8(match tipo {
        TipoColumna::Personal => 0,
        TipoColumna::Ejercito => 1,
    });
    w.u64(*lider);
    w.u8(politica_a_u8(*politica));
    w.u8(match estado {
        Estado::Marchando => 0,
        Estado::Estacionada => 1,
    });
    w.u64(participantes.len() as u64);
    for Participante { heroe, unido_en } in participantes {
        w.u64(*heroe);
        w.i64(*unido_en);
    }
    w.u64(peticiones.len() as u64);
    for Peticion { heroe, pedido_en, expira_en } in peticiones {
        w.u64(*heroe);
        w.i64(*pedido_en);
        w.i64(*expira_en);
    }
    w.opt_i64(*formacion_expira_en);
    w.u64(escuadras.len() as u64);
    escuadras.iter().for_each(|&s| w.u64(s));
    carga_escribir(w, carga);
    ritmo_escribir(w, viveres);
    w.punto(*posicion);
    w.u32(plan.generacion);
    w.u32(plan.vertices.len() as u32);
    for v in &plan.vertices {
        w.punto(v.p);
        w.i64(v.en);
    }
    w.opt_i64(*velocidad_fija);
    w.bool(*caravana_fundacion);
    w.u32(caravanas_adjuntas.len() as u32);
    caravanas_adjuntas.iter().for_each(|&c| w.u64(c));
    w.opt_u64(*persiguiendo);
    w.bool(*en_batalla);
    w.u64(*version);
}

pub(crate) fn leer(r: &mut Lector) -> Result<Ejercito, ErrorFormato> {
    let id = r.u64()?;
    let faccion = r.opt_u64()?.map(u32::try_from).transpose().map_err(|_| ErrorFormato("Facción fuera de rango"))?;
    let tipo = match r.u8()? {
        0 => TipoColumna::Personal,
        1 => TipoColumna::Ejercito,
        _ => return Err(ErrorFormato("tipo de columna desconocido")),
    };
    let (lider, politica) = (r.u64()?, politica_de_u8(r.u8()?)?);
    let estado = match r.u8()? {
        0 => Estado::Marchando,
        1 => Estado::Estacionada,
        _ => return Err(ErrorFormato("estado de columna desconocido")),
    };
    let mut participantes = Vec::new();
    for _ in 0..r.cuenta()? {
        participantes.push(Participante { heroe: r.u64()?, unido_en: r.i64()? });
    }
    let mut peticiones = Vec::new();
    for _ in 0..r.cuenta()? {
        peticiones.push(Peticion { heroe: r.u64()?, pedido_en: r.i64()?, expira_en: r.i64()? });
    }
    let formacion_expira_en = r.opt_i64()?;
    let escuadras = (0..r.cuenta()?).map(|_| r.u64()).collect::<Result<_, _>>()?;
    let (carga, viveres, posicion) = (carga_leer(r)?, ritmo_leer(r)?, r.punto()?);
    let generacion = r.u32()?;
    let mut vertices = Vec::new();
    for _ in 0..r.u32()? {
        vertices.push(Vertice { p: r.punto()?, en: r.i64()? });
    }
    let (velocidad_fija, caravana_fundacion) = (r.opt_i64()?, r.bool()?);
    let mut caravanas_adjuntas = Vec::new();
    for _ in 0..r.u32()? {
        caravanas_adjuntas.push(r.u64()?);
    }
    let columna = Columna { id, faccion, tipo, lider, politica, estado, participantes, peticiones, formacion_expira_en, escuadras, carga, viveres };
    let (persiguiendo, en_batalla) = (r.opt_u64()?, r.bool()?);
    Ok(Ejercito { id, columna, posicion, plan: Plan { generacion, vertices }, velocidad_fija, caravana_fundacion, caravanas_adjuntas, persiguiendo, en_batalla, version: r.u64()? })
}
