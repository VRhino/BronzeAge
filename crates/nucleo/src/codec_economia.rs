//! Serialización canónica de la economía de una plaza (`asentamiento::Asentamiento`) para el hash y las fotos. Cada estructura se
//! desestructura **entera** (sin `..`): si el crate añade un campo, esto deja de compilar y obliga a decidir cómo se guarda.
//!
//! Formato (BAN27: los almacenes guardan unidades enteras —cantidad múltiplo de 1000— y el resto oculto de sus flujos, en `(−1000, 1000)`; BAN20: BAN15 más los recintos de muralla de cada plaza, que ya no se guardan con su ciudad; BAN15 era BAN14, con la población entera, sus restos y la marca de recuperación del hambre, más las avanzadas de cada plaza; little endian, el de `codec`): `fundado_en i64 · nivel u32 · medidor{valor i64, racha u32, nivel_actual u32} ·
//! población{pesants, artesanos, nobleza: i64 (habitantes enteros) · restos 3 × i64 (milésimas de habitante, uno por clase) · nutrición i64 · recuperando bool} · almacén{29 × (cantidad i64, capacidad i64, resto i64)} · políticas{n u64 ·
//! n × (id texto, cargo u8, expira_en i64)} · edificios{n u64 · n × edificio} · pueblos{n u64 · n × pueblo} · avanzadas{n u64 · n × avanzada} · recintos{n u64 · n × recinto} · ingresos_acarreo{n u64 · n × (recurso u8, hora i64, cantidad i64)} · siguiente_edificio u32 · ascenso{opt: nivel_objetivo u32,
//! iniciado_en i64, completa_en i64} · recetas_pausadas{n u64 · n × recurso u8} · auto_pausada bool · reserva_manual 29 × i64 · ocupado
//! bool · extractor_minutos_sin_cupo{n u64 · n × (tipo u8, i64)} · completados_pendientes u32 · destruido bool · ultimo_minuto i64 · tendencia{almacén 29 × i64 · población 3 × i64}` (BAN22: la tendencia «+N/min» del último paso de minuto).
//! Un edificio: `id u32 · tipo u8 · estado u8 · completa_en opt_i64 · prioridad i64 · nivel_interno u8 · mejora{bool, nivel u8,
//! completa_en i64} · danado bool · fuente{u8: 0 ninguna, 1 nodo, 2 parcela; u32} · pausado_por_almacen bool`.
//! Un pueblo: `id u64 · tipo u8 · nodo u32 · estado{u8: 0 pendiente → requerido y entregado, cada uno n u64 · n × (recurso u8, i64);
//! 1 en obra → completa_en i64; 2 activo} · almacén{29 × (cantidad i64, capacidad i64, resto i64)} · carro opt_u64 · lleno bool`.
//! Una avanzada: `id u64 · posición punto · etapa u8 (0 atalaya, 1 fuerte, 2 ciudadela) · estado{u8: 0 en construcción, 1 operativa, 2 en ruinas → invulnerable_hasta opt_i64} ·
//! obra{opt: tipo u8 (0 construir, 1 mejorar, 2 reparar) · requerido y entregado, cada uno n u64 · n × (recurso u8, i64) · completa_en opt_i64} · almacén{29 × (cantidad i64, capacidad i64, resto i64)}`.
//! Un recinto: `id u32 · nivel u8 · celdas{n u64 · n × (col i32, row i32, clase u8: 0 muro, 1 puerta, 2 torre)} · avance i32 · mejora{opt: nivel u8, avance i32} ·
//! siguiente_celda_en opt_i64 · comprometido_en i64 · completado_en opt_i64`. La ciudad de la plaza (`codec_ciudad`) no guarda recintos: se derivan de estos al leer.
//! Los tipos y recursos van por su índice en el enum (`Tipo as u8`, `Recurso::indice`).

use crate::codec::{ErrorFormato, Escritor, Lector};
use asentamiento::asentamiento::{Ascenso, Asentamiento, Tendencia};
use asentamiento::avanzadas::{Avanzada, Estado as EstadoAvanzada, Etapa, Obra, TipoObra};
use asentamiento::catalogo::{Tipo, TIPOS};
use asentamiento::edificio::{Edificio, Estado, Fuente, Mejora};
use asentamiento::mantenimiento::Medidor;
use asentamiento::politicas::Activa;
use asentamiento::pueblos::{EstadoPueblo, IngresoAcarreo, Pueblo};
use asentamiento::recintos::{Mejora as MejoraDeRecinto, Recinto};
use asentamiento::recurso::TODOS;
use asentamiento::{Almacen, Cargo, Costo, Politicas, Poblacion, Recurso};
use trazado::{Celda, CeldaDeRed, CeldaMuro, ClaseCelda};

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

fn escribir_almacen(w: &mut Escritor, a: &Almacen) {
    let (cantidad, capacidad, resto) = a.partes();
    for i in 0..29 {
        w.i64(cantidad[i]);
        w.i64(capacidad[i]);
        w.i64(resto[i]);
    }
}

fn leer_almacen(r: &mut Lector) -> Result<Almacen, ErrorFormato> {
    let (mut cantidad, mut capacidad, mut resto) = ([0; 29], [0; 29], [0; 29]);
    for i in 0..29 {
        cantidad[i] = r.i64()?;
        capacidad[i] = r.i64()?;
        resto[i] = r.i64()?;
    }
    Ok(Almacen::desde_partes(cantidad, capacidad, resto))
}

pub(crate) fn escribir_avanzada(w: &mut Escritor, a: &Avanzada) {
    let Avanzada { id, posicion, etapa, estado, obra, almacen } = a;
    w.u64(*id);
    w.punto(*posicion);
    w.u8(etapa.indice() as u8);
    match estado {
        EstadoAvanzada::EnConstruccion => w.u8(0),
        EstadoAvanzada::Operativa => w.u8(1),
        EstadoAvanzada::EnRuinas { invulnerable_hasta } => {
            w.u8(2);
            w.opt_i64(*invulnerable_hasta);
        }
        EstadoAvanzada::Destruida => unreachable!("una avanzada destruida sale del estado: no se guarda"),
    }
    match obra {
        Some(Obra { tipo, requerido, entregado, completa_en }) => {
            w.bool(true);
            w.u8(*tipo as u8);
            escribir_costo(w, requerido);
            escribir_costo(w, entregado);
            w.opt_i64(*completa_en);
        }
        None => w.bool(false),
    }
    escribir_almacen(w, almacen);
}

pub(crate) fn leer_avanzada(r: &mut Lector) -> Result<Avanzada, ErrorFormato> {
    let (id, posicion) = (r.u64()?, r.punto()?);
    let etapa = Etapa::TODAS.get(r.u8()? as usize).copied().ok_or(ErrorFormato("etapa de avanzada desconocida"))?;
    let estado = match r.u8()? {
        0 => EstadoAvanzada::EnConstruccion,
        1 => EstadoAvanzada::Operativa,
        2 => EstadoAvanzada::EnRuinas { invulnerable_hasta: r.opt_i64()? },
        _ => return Err(ErrorFormato("estado de avanzada desconocido")),
    };
    let obra = if r.bool()? {
        let tipo = match r.u8()? {
            0 => TipoObra::Construir,
            1 => TipoObra::Mejorar,
            2 => TipoObra::Reparar,
            _ => return Err(ErrorFormato("tipo de obra de avanzada desconocido")),
        };
        Some(Obra { tipo, requerido: leer_costo(r)?, entregado: leer_costo(r)?, completa_en: r.opt_i64()? })
    } else {
        None
    };
    Ok(Avanzada { id, posicion, etapa, estado, obra, almacen: leer_almacen(r)? })
}

fn escribir_costo(w: &mut Escritor, c: &Costo) {
    w.u64(c.len() as u64);
    for &(rec, n) in c {
        w.u8(recurso_a_u8(rec));
        w.i64(n);
    }
}

fn leer_costo(r: &mut Lector) -> Result<Costo, ErrorFormato> {
    let mut c = Costo::new();
    for _ in 0..r.cuenta()? {
        c.push((recurso_de_u8(r.u8()?)?, r.i64()?));
    }
    Ok(c)
}

fn clase_a_u8(c: ClaseCelda) -> u8 {
    match c {
        ClaseCelda::Muro => 0,
        ClaseCelda::Puerta => 1,
        ClaseCelda::Torre => 2,
    }
}

fn clase_de_u8(i: u8) -> Result<ClaseCelda, ErrorFormato> {
    Ok(match i {
        0 => ClaseCelda::Muro,
        1 => ClaseCelda::Puerta,
        2 => ClaseCelda::Torre,
        _ => return Err(ErrorFormato("clase de celda de muralla desconocida")),
    })
}

/// Las celdas de un anillo: `n u64 · n × (col i32, fila i32, clase u8)`.
pub(crate) fn escribir_celdas(w: &mut Escritor, celdas: &[CeldaMuro]) {
    w.u64(celdas.len() as u64);
    for CeldaMuro { celda: Celda { col, row }, clase } in celdas {
        w.i32(*col);
        w.i32(*row);
        w.u8(clase_a_u8(*clase));
    }
}

pub(crate) fn leer_celdas(r: &mut Lector) -> Result<Vec<CeldaMuro>, ErrorFormato> {
    (0..r.cuenta()?).map(|_| Ok(CeldaMuro { celda: Celda::new(r.i32()?, r.i32()?), clase: clase_de_u8(r.u8()?)? })).collect()
}

fn escribir_recinto(w: &mut Escritor, r: &Recinto) {
    let Recinto { id, nivel, celdas, red_de_puertas, tras_pieza, avance, mejora, siguiente_celda_en, comprometido_en, completado_en } = r;
    w.u32(*id);
    w.u8(*nivel);
    escribir_celdas(w, celdas);
    w.u64(red_de_puertas.len() as u64);
    for CeldaDeRed { celda: Celda { col, row }, camino } in red_de_puertas {
        w.i32(*col);
        w.i32(*row);
        w.bool(*camino);
    }
    w.u32(*tras_pieza);
    w.i32(*avance);
    match mejora {
        Some(MejoraDeRecinto { nivel, avance }) => {
            w.bool(true);
            w.u8(*nivel);
            w.i32(*avance);
        }
        None => w.bool(false),
    }
    w.opt_i64(*siguiente_celda_en);
    w.i64(*comprometido_en);
    w.opt_i64(*completado_en);
}

fn leer_recinto(r: &mut Lector) -> Result<Recinto, ErrorFormato> {
    let (id, nivel) = (r.u32()?, r.u8()?);
    if !(1..=3).contains(&nivel) {
        return Err(ErrorFormato("nivel de recinto desconocido"));
    }
    let celdas = leer_celdas(r)?;
    if celdas.is_empty() {
        return Err(ErrorFormato("recinto sin celdas"));
    }
    let red_de_puertas = (0..r.cuenta()?).map(|_| Ok(CeldaDeRed { celda: Celda::new(r.i32()?, r.i32()?), camino: r.bool()? })).collect::<Result<Vec<_>, ErrorFormato>>()?;
    let tras_pieza = r.u32()?;
    let ultima = celdas.len() as i32 - 1;
    let avance = r.i32()?;
    let mejora = if r.bool()? { Some(MejoraDeRecinto { nivel: r.u8()?, avance: r.i32()? }) } else { None };
    if !(-1..=ultima).contains(&avance) || mejora.is_some_and(|m| m.nivel != nivel + 1 || m.nivel > 3 || !(-1..ultima).contains(&m.avance)) {
        return Err(ErrorFormato("avance de recinto fuera del anillo"));
    }
    Ok(Recinto { id, nivel, celdas, red_de_puertas, tras_pieza, avance, mejora, siguiente_celda_en: r.opt_i64()?, comprometido_en: r.i64()?, completado_en: r.opt_i64()? })
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
        pueblos,
        avanzadas,
        recintos,
        ingresos_acarreo,
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
        tendencia,
    } = a;
    w.i64(*fundado_en);
    w.u32(*nivel);
    let Medidor { valor, racha, nivel_actual } = medidor;
    w.i64(*valor);
    w.u32(*racha);
    w.u32(*nivel_actual);
    let Poblacion { pesants, artesanos, nobleza, restos, nutricion, recuperando } = poblacion;
    [pesants, artesanos, nobleza].iter().for_each(|&&x| w.i64(x));
    restos.iter().for_each(|&x| w.i64(x));
    w.i64(*nutricion);
    w.bool(*recuperando);
    escribir_almacen(w, almacen);
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
    w.u64(pueblos.len() as u64);
    for Pueblo { id, tipo, nodo, estado, almacen, carro, lleno } in pueblos {
        w.u64(*id);
        w.u8(tipo_a_u8(*tipo));
        w.u32(*nodo);
        match estado {
            EstadoPueblo::Pendiente { requerido, entregado } => {
                w.u8(0);
                escribir_costo(w, requerido);
                escribir_costo(w, entregado);
            }
            EstadoPueblo::EnObra { completa_en } => {
                w.u8(1);
                w.i64(*completa_en);
            }
            EstadoPueblo::Activo => w.u8(2),
        }
        escribir_almacen(w, almacen);
        w.opt_u64(*carro);
        w.bool(*lleno);
    }
    w.u64(avanzadas.len() as u64);
    avanzadas.iter().for_each(|av| escribir_avanzada(w, av));
    w.u64(recintos.len() as u64);
    recintos.iter().for_each(|rec| escribir_recinto(w, rec));
    w.u64(ingresos_acarreo.len() as u64);
    for IngresoAcarreo { recurso, hora, cantidad } in ingresos_acarreo {
        w.u8(recurso_a_u8(*recurso));
        w.i64(*hora);
        w.i64(*cantidad);
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
    tendencia.almacen.iter().for_each(|&x| w.i64(x));
    tendencia.poblacion.iter().for_each(|&x| w.i64(x));
}

pub(crate) fn leer(r: &mut Lector) -> Result<Asentamiento, ErrorFormato> {
    let (fundado_en, nivel) = (r.i64()?, r.u32()?);
    let medidor = Medidor { valor: r.i64()?, racha: r.u32()?, nivel_actual: r.u32()? };
    let (pesants, artesanos, nobleza) = (r.i64()?, r.i64()?, r.i64()?);
    let restos = [r.i64()?, r.i64()?, r.i64()?];
    let poblacion = Poblacion { pesants, artesanos, nobleza, restos, nutricion: r.i64()?, recuperando: r.bool()? };
    let almacen = leer_almacen(r)?;
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
    let mut pueblos = Vec::new();
    for _ in 0..r.cuenta()? {
        let (id, tipo, nodo) = (r.u64()?, tipo_de_u8(r.u8()?)?, r.u32()?);
        let estado = match r.u8()? {
            0 => EstadoPueblo::Pendiente { requerido: leer_costo(r)?, entregado: leer_costo(r)? },
            1 => EstadoPueblo::EnObra { completa_en: r.i64()? },
            2 => EstadoPueblo::Activo,
            _ => return Err(ErrorFormato("estado de pueblo desconocido")),
        };
        pueblos.push(Pueblo { id, tipo, nodo, estado, almacen: leer_almacen(r)?, carro: r.opt_u64()?, lleno: r.bool()? });
    }
    let mut avanzadas = Vec::new();
    for _ in 0..r.cuenta()? {
        avanzadas.push(leer_avanzada(r)?);
    }
    let mut recintos = Vec::new();
    for _ in 0..r.cuenta()? {
        recintos.push(leer_recinto(r)?);
    }
    let mut ingresos_acarreo = Vec::new();
    for _ in 0..r.cuenta()? {
        ingresos_acarreo.push(IngresoAcarreo { recurso: recurso_de_u8(r.u8()?)?, hora: r.i64()?, cantidad: r.i64()? });
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
        pueblos,
        avanzadas,
        recintos,
        ingresos_acarreo,
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
        tendencia: {
            let mut t = Tendencia::default();
            for x in &mut t.almacen {
                *x = r.i64()?;
            }
            for x in &mut t.poblacion {
                *x = r.i64()?;
            }
            t
        },
    })
}
