//! Serialización canónica de la economía de una plaza (`asentamiento::Asentamiento`) para el hash y las fotos. Cada estructura se
//! desestructura **entera** (sin `..`): si el crate añade un campo, esto deja de compilar y obliga a decidir cómo se guarda.
//!
//! Formato (little endian, el de `codec`): `fundado_en i64 · nivel u32 · medidor{valor i64, racha u32, nivel_actual u32} ·
//! población{pesants, artesanos, nobleza, nutrición: i64} · almacén{29 × (cantidad i64, capacidad i64)} · políticas{n u64 ·
//! n × (id texto, cargo u8, expira_en i64)} · edificios{n u64 · n × edificio} · siguiente_edificio u32 · ascenso{opt: nivel_objetivo u32,
//! iniciado_en i64, completa_en i64} · recetas_pausadas{n u64 · n × recurso u8} · auto_pausada bool · reserva_manual 29 × i64 · ocupado
//! bool · extractor_minutos_sin_cupo{n u64 · n × (tipo u8, i64)} · completados_pendientes u32 · destruido bool · ultimo_minuto i64`.
//! Un edificio: `id u32 · tipo u8 · estado u8 · completa_en opt_i64 · prioridad i64 · nivel_interno u8 · mejora{bool, nivel u8,
//! completa_en i64} · danado bool · fuente{u8: 0 ninguna, 1 nodo, 2 parcela; u32} · pausado_por_almacen bool`.
//! Los tipos y recursos van por su índice en el enum (`Tipo as u8`, `Recurso::indice`).

use crate::codec::{ErrorFormato, Escritor, Lector};
use asentamiento::asentamiento::{Ascenso, Asentamiento};
use asentamiento::catalogo::{Tipo, TIPOS};
use asentamiento::edificio::{Edificio, Estado, Fuente, Mejora};
use asentamiento::mantenimiento::Medidor;
use asentamiento::politicas::Activa;
use asentamiento::recurso::TODOS;
use asentamiento::{Almacen, Cargo, Politicas, Poblacion, Recurso};

pub(crate) fn tipo_a_u8(t: Tipo) -> u8 {
    t as u8
}

pub(crate) fn tipo_de_u8(i: u8) -> Result<Tipo, ErrorFormato> {
    TIPOS.get(i as usize).copied().ok_or(ErrorFormato("tipo de edificio desconocido"))
}

pub(crate) fn recurso_a_u8(r: Recurso) -> u8 {
    r.indice() as u8
}

pub(crate) fn recurso_de_u8(i: u8) -> Result<Recurso, ErrorFormato> {
    TODOS.get(i as usize).copied().ok_or(ErrorFormato("recurso desconocido"))
}

pub(crate) fn cargo_a_u8(c: Cargo) -> u8 {
    match c {
        Cargo::Gobernador => 0,
        Cargo::Tesorero => 1,
        Cargo::General => 2,
        Cargo::MaestroObras => 3,
        Cargo::Sacerdote => 4,
    }
}

pub(crate) fn cargo_de_u8(i: u8) -> Result<Cargo, ErrorFormato> {
    Ok(match i {
        0 => Cargo::Gobernador,
        1 => Cargo::Tesorero,
        2 => Cargo::General,
        3 => Cargo::MaestroObras,
        4 => Cargo::Sacerdote,
        _ => return Err(ErrorFormato("cargo desconocido")),
    })
}

fn estado_a_u8(e: Estado) -> u8 {
    match e {
        Estado::EnCola => 0,
        Estado::EnConstruccion => 1,
        Estado::Activo => 2,
    }
}

pub(crate) fn escribir(w: &mut Escritor, a: &Asentamiento) {
    let Asentamiento {
        fundado_en,
        nivel,
        medidor,
        poblacion,
        almacen,
        politicas,
        edificios,
        siguiente_edificio,
        ascenso,
        recetas_pausadas,
        auto_pausada,
        reserva_manual,
        ocupado,
        extractor_minutos_sin_cupo,
        completados_pendientes,
        destruido,
        ultimo_minuto,
    } = a;
    w.i64(*fundado_en);
    w.u32(*nivel);
    let Medidor { valor, racha, nivel_actual } = medidor;
    w.i64(*valor);
    w.u32(*racha);
    w.u32(*nivel_actual);
    let Poblacion { pesants, artesanos, nobleza, nutricion } = poblacion;
    [pesants, artesanos, nobleza, nutricion].iter().for_each(|&&x| w.i64(x));
    let (cantidad, capacidad) = almacen.partes();
    cantidad.iter().zip(capacidad).for_each(|(&c, &k)| {
        w.i64(c);
        w.i64(k);
    });
    w.u64(politicas.activas().len() as u64);
    for Activa { id, cargo, expira_en } in politicas.activas() {
        w.texto(id);
        w.u8(cargo_a_u8(*cargo));
        w.i64(*expira_en);
    }
    w.u64(edificios.len() as u64);
    for e in edificios {
        let Edificio { id, tipo, estado, completa_en, prioridad, nivel_interno, mejora, danado, fuente, pausado_por_almacen } = e;
        w.u32(*id);
        w.u8(tipo_a_u8(*tipo));
        w.u8(estado_a_u8(*estado));
        w.opt_i64(*completa_en);
        w.i64(*prioridad);
        w.u8(*nivel_interno);
        match mejora {
            Some(Mejora { nivel_objetivo, completa_en }) => {
                w.bool(true);
                w.u8(*nivel_objetivo);
                w.i64(*completa_en);
            }
            None => w.bool(false),
        }
        w.bool(*danado);
        match fuente {
            None => w.u8(0),
            Some(Fuente::Nodo(n)) => {
                w.u8(1);
                w.u32(*n);
            }
            Some(Fuente::Parcela(p)) => {
                w.u8(2);
                w.u32(*p);
            }
        }
        w.bool(*pausado_por_almacen);
    }
    w.u32(*siguiente_edificio);
    match ascenso {
        Some(Ascenso { nivel_objetivo, iniciado_en, completa_en }) => {
            w.bool(true);
            w.u32(*nivel_objetivo);
            w.i64(*iniciado_en);
            w.i64(*completa_en);
        }
        None => w.bool(false),
    }
    w.u64(recetas_pausadas.len() as u64);
    recetas_pausadas.iter().for_each(|&r| w.u8(recurso_a_u8(r)));
    w.bool(*auto_pausada);
    reserva_manual.iter().for_each(|&x| w.i64(x));
    w.bool(*ocupado);
    w.u64(extractor_minutos_sin_cupo.len() as u64);
    for &(t, n) in extractor_minutos_sin_cupo {
        w.u8(tipo_a_u8(t));
        w.i64(n);
    }
    w.u32(*completados_pendientes);
    w.bool(*destruido);
    w.i64(*ultimo_minuto);
}

pub(crate) fn leer(r: &mut Lector) -> Result<Asentamiento, ErrorFormato> {
    let (fundado_en, nivel) = (r.i64()?, r.u32()?);
    let medidor = Medidor { valor: r.i64()?, racha: r.u32()?, nivel_actual: r.u32()? };
    let poblacion = Poblacion { pesants: r.i64()?, artesanos: r.i64()?, nobleza: r.i64()?, nutricion: r.i64()? };
    let (mut cantidad, mut capacidad) = ([0; 29], [0; 29]);
    for i in 0..29 {
        cantidad[i] = r.i64()?;
        capacidad[i] = r.i64()?;
    }
    let almacen = Almacen::desde_partes(cantidad, capacidad);
    let mut activas = Vec::new();
    for _ in 0..r.cuenta()? {
        activas.push(Activa { id: r.texto()?, cargo: cargo_de_u8(r.u8()?)?, expira_en: r.i64()? });
    }
    let mut edificios = Vec::new();
    for _ in 0..r.cuenta()? {
        let (id, tipo) = (r.u32()?, tipo_de_u8(r.u8()?)?);
        let estado = match r.u8()? {
            0 => Estado::EnCola,
            1 => Estado::EnConstruccion,
            2 => Estado::Activo,
            _ => return Err(ErrorFormato("estado de edificio desconocido")),
        };
        let (completa_en, prioridad, nivel_interno) = (r.opt_i64()?, r.i64()?, r.u8()?);
        let mejora = if r.bool()? { Some(Mejora { nivel_objetivo: r.u8()?, completa_en: r.i64()? }) } else { None };
        let danado = r.bool()?;
        let fuente = match r.u8()? {
            0 => None,
            1 => Some(Fuente::Nodo(r.u32()?)),
            2 => Some(Fuente::Parcela(r.u32()?)),
            _ => return Err(ErrorFormato("fuente de edificio desconocida")),
        };
        edificios.push(Edificio { id, tipo, estado, completa_en, prioridad, nivel_interno, mejora, danado, fuente, pausado_por_almacen: r.bool()? });
    }
    let siguiente_edificio = r.u32()?;
    let ascenso = if r.bool()? { Some(Ascenso { nivel_objetivo: r.u32()?, iniciado_en: r.i64()?, completa_en: r.i64()? }) } else { None };
    let mut recetas_pausadas = Vec::new();
    for _ in 0..r.cuenta()? {
        recetas_pausadas.push(recurso_de_u8(r.u8()?)?);
    }
    let auto_pausada = r.bool()?;
    let mut reserva_manual = [0; 29];
    for x in reserva_manual.iter_mut() {
        *x = r.i64()?;
    }
    let ocupado = r.bool()?;
    let mut extractor_minutos_sin_cupo = Vec::new();
    for _ in 0..r.cuenta()? {
        extractor_minutos_sin_cupo.push((tipo_de_u8(r.u8()?)?, r.i64()?));
    }
    Ok(Asentamiento {
        fundado_en,
        nivel,
        medidor,
        poblacion,
        almacen,
        politicas: Politicas::desde_activas(activas),
        edificios,
        siguiente_edificio,
        ascenso,
        recetas_pausadas,
        auto_pausada,
        reserva_manual,
        ocupado,
        extractor_minutos_sin_cupo,
        completados_pendientes: r.u32()?,
        destruido: r.bool()?,
        ultimo_minuto: r.i64()?,
    })
}
