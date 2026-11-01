//! Serialización canónica de los contratos de mercenarios (`mercenarios::EstadoContratos`, entrada 48) para el hash y las fotos (desde BAN54). Cada estructura se desestructura
//! **entera** (sin `..`): si el crate `mercenarios` añade un campo, esto deja de compilar y obliga a decidir cómo se guarda.
//!
//! Formato (little endian, el de `codec`): `ofertas{n u64 · n × (héroe u64 · campamento u32 · desde i64)} · contratos{n u64 · n × contrato} · fichas{n u64 · n × (héroe u64 ·
//! ficha)} · esperas{n u64 · n × (mercenario u64 · Facción u32 · hasta i64)} · terminados{n u64 · n × (mercenario u64 · Facción u32 · firmado_por u64 · renombre u32 ·
//! valorable_hasta i64 · valorado bool)}`.
//!
//! Un contrato: `mercenario u64 · Facción u32 · ámbito{u8: 0 Facción, 1 plaza · u64} · trabajo u8 (su posición en Trabajo::TODOS) · objetivo · plazo{bool · u8} ·
//! firmado_por u64 · puesto · firmado_en i64 · vence opt_i64 · campamento_base u32 · orden{bool · orden} · fin_aplazado{u8: 0 ninguno, 1 plazo, 2 despido, 3 plaza perdida,
//! 4 sin ciudadanos, 5 Facción perdida} · renombre u32`.
//! Una orden: `tipo u8 (0 guardar, 1 ir_a, 2 atacar, 3 unirse) · objetivo · dada_por u64 · puesto{bool · puesto: sin él, la orden inicial del contrato} · dada_en i64 · vence opt_i64 ·
//! contra opt_u64 · unido bool`.
//! Un objetivo: `u8 (0 plaza, 1 pueblo, 2 avanzada, 3 caravana, 4 bandidos, 5 ejército, 6 convocatoria: · u64; 7 Facción · u32; 8 punto · punto)`.
//! Un puesto: `u8 (0 Rey, 1 Embajador; 2 General, 3 Gobernador, 4 Maestro de Caravanas: · plaza u64)`.
//! Una ficha: `renombre u32 · rango u8 · premios u8 · contadores{n u64 · n × (medalla u8 · u32)} · grados{n u64 · n × (medalla u8 · u8)} · historial{firmados, terminados,
//! despidos, buenas, malas: 5 × u32} · leva_recibida bool`, con la medalla por su posición en `Medalla::TODAS`.

use crate::codec::{ErrorFormato, Escritor, Lector};
use mercenarios::{Ambito, Contrato, EstadoContratos, Ficha, Fin, Historial, Medalla, Objetivo, Oferta, Orden, Puesto, Terminado, TipoOrden, Trabajo};
use std::collections::BTreeMap;

pub(crate) fn escribir(w: &mut Escritor, e: &EstadoContratos) {
    let EstadoContratos { ofertas, contratos, fichas, esperas, terminados } = e;
    w.u64(ofertas.len() as u64);
    for (&h, &Oferta { campamento, desde }) in ofertas {
        w.u64(h);
        w.u32(campamento);
        w.i64(desde);
    }
    w.u64(contratos.len() as u64);
    contratos.values().for_each(|c| contrato_escribir(w, c));
    w.u64(fichas.len() as u64);
    for (&h, f) in fichas {
        w.u64(h);
        ficha_escribir(w, f);
    }
    w.u64(esperas.len() as u64);
    for (&(m, f), &hasta) in esperas {
        w.u64(m);
        w.u32(f);
        w.i64(hasta);
    }
    w.u64(terminados.len() as u64);
    for (&(m, f), &Terminado { firmado_por, renombre, valorable_hasta, valorado }) in terminados {
        w.u64(m);
        w.u32(f);
        w.u64(firmado_por);
        w.u32(renombre);
        w.i64(valorable_hasta);
        w.bool(valorado);
    }
}

pub(crate) fn leer(r: &mut Lector) -> Result<EstadoContratos, ErrorFormato> {
    let ofertas = (0..r.cuenta()?).map(|_| Ok((r.u64()?, Oferta { campamento: r.u32()?, desde: r.i64()? }))).collect::<Result<_, ErrorFormato>>()?;
    let contratos = (0..r.cuenta()?)
        .map(|_| {
            let c = contrato_leer(r)?;
            Ok((c.mercenario, c))
        })
        .collect::<Result<_, ErrorFormato>>()?;
    let fichas = (0..r.cuenta()?).map(|_| Ok((r.u64()?, ficha_leer(r)?))).collect::<Result<_, ErrorFormato>>()?;
    let esperas = (0..r.cuenta()?).map(|_| Ok(((r.u64()?, r.u32()?), r.i64()?))).collect::<Result<_, ErrorFormato>>()?;
    let terminados = (0..r.cuenta()?)
        .map(|_| Ok(((r.u64()?, r.u32()?), Terminado { firmado_por: r.u64()?, renombre: r.u32()?, valorable_hasta: r.i64()?, valorado: r.bool()? })))
        .collect::<Result<_, ErrorFormato>>()?;
    Ok(EstadoContratos { ofertas, contratos, fichas, esperas, terminados })
}

fn contrato_escribir(w: &mut Escritor, c: &Contrato) {
    let Contrato { mercenario, faccion, ambito, trabajo, objetivo, plazo, firmado_por, puesto_firma, firmado_en, vence, campamento_base, orden, fin_aplazado, renombre } = c;
    w.u64(*mercenario);
    w.u32(*faccion);
    match ambito {
        Ambito::Faccion => w.u8(0),
        Ambito::Plaza(p) => {
            w.u8(1);
            w.u64(*p);
        }
    }
    w.u8(trabajo_a_u8(*trabajo));
    objetivo_escribir(w, objetivo);
    w.bool(plazo.is_some());
    if let Some(d) = plazo {
        w.u8(*d);
    }
    w.u64(*firmado_por);
    puesto_escribir(w, puesto_firma);
    w.i64(*firmado_en);
    w.opt_i64(*vence);
    w.u32(*campamento_base);
    w.bool(orden.is_some());
    if let Some(o) = orden {
        orden_escribir(w, o);
    }
    w.u8(match fin_aplazado {
        None => 0,
        Some(Fin::Plazo) => 1,
        Some(Fin::Despido) => 2,
        Some(Fin::PlazaPerdida) => 3,
        Some(Fin::SinCiudadanos) => 4,
        Some(Fin::FaccionPerdida) => 5,
    });
    w.u32(*renombre);
}

fn contrato_leer(r: &mut Lector) -> Result<Contrato, ErrorFormato> {
    Ok(Contrato {
        mercenario: r.u64()?,
        faccion: r.u32()?,
        ambito: match r.u8()? {
            0 => Ambito::Faccion,
            1 => Ambito::Plaza(r.u64()?),
            _ => return Err(ErrorFormato("ámbito de contrato desconocido")),
        },
        trabajo: trabajo_de_u8(r.u8()?)?,
        objetivo: objetivo_leer(r)?,
        plazo: if r.bool()? { Some(r.u8()?) } else { None },
        firmado_por: r.u64()?,
        puesto_firma: puesto_leer(r)?,
        firmado_en: r.i64()?,
        vence: r.opt_i64()?,
        campamento_base: r.u32()?,
        orden: if r.bool()? { Some(orden_leer(r)?) } else { None },
        fin_aplazado: match r.u8()? {
            0 => None,
            1 => Some(Fin::Plazo),
            2 => Some(Fin::Despido),
            3 => Some(Fin::PlazaPerdida),
            4 => Some(Fin::SinCiudadanos),
            5 => Some(Fin::FaccionPerdida),
            _ => return Err(ErrorFormato("fin de contrato desconocido")),
        },
        renombre: r.u32()?,
    })
}

fn orden_escribir(w: &mut Escritor, o: &Orden) {
    let Orden { tipo, objetivo, dada_por, puesto, dada_en, vence, contra, unido } = o;
    w.u8(tipo_orden_a_u8(*tipo));
    objetivo_escribir(w, objetivo);
    w.u64(*dada_por);
    w.bool(puesto.is_some());
    if let Some(p) = puesto {
        puesto_escribir(w, p);
    }
    w.i64(*dada_en);
    w.opt_i64(*vence);
    w.opt_u64(contra.map(u64::from));
    w.bool(*unido);
}

fn orden_leer(r: &mut Lector) -> Result<Orden, ErrorFormato> {
    Ok(Orden {
        tipo: tipo_orden_de_u8(r.u8()?)?,
        objetivo: objetivo_leer(r)?,
        dada_por: r.u64()?,
        puesto: if r.bool()? { Some(puesto_leer(r)?) } else { None },
        dada_en: r.i64()?,
        vence: r.opt_i64()?,
        contra: r.opt_u64()?.map(|x| u32::try_from(x).map_err(|_| ErrorFormato("id de Facción fuera de rango"))).transpose()?,
        unido: r.bool()?,
    })
}

/// El trabajo por su posición en `Trabajo::TODOS` (también en el códec de los comandos).
pub(crate) fn trabajo_a_u8(t: Trabajo) -> u8 {
    Trabajo::TODOS.iter().position(|&x| x == t).expect("TODOS lista cada trabajo") as u8
}

pub(crate) fn trabajo_de_u8(x: u8) -> Result<Trabajo, ErrorFormato> {
    Trabajo::TODOS.get(usize::from(x)).copied().ok_or(ErrorFormato("trabajo de mercenario desconocido"))
}

pub(crate) fn tipo_orden_a_u8(t: TipoOrden) -> u8 {
    match t {
        TipoOrden::Guardar => 0,
        TipoOrden::IrA => 1,
        TipoOrden::Atacar => 2,
        TipoOrden::Unirse => 3,
    }
}

pub(crate) fn tipo_orden_de_u8(x: u8) -> Result<TipoOrden, ErrorFormato> {
    Ok(match x {
        0 => TipoOrden::Guardar,
        1 => TipoOrden::IrA,
        2 => TipoOrden::Atacar,
        3 => TipoOrden::Unirse,
        _ => return Err(ErrorFormato("tipo de orden de mercenario desconocido")),
    })
}

pub(crate) fn objetivo_escribir(w: &mut Escritor, o: &Objetivo) {
    let (clase, id) = match *o {
        Objetivo::Plaza(x) => (0, x),
        Objetivo::Pueblo(x) => (1, x),
        Objetivo::Avanzada(x) => (2, x),
        Objetivo::Caravana(x) => (3, x),
        Objetivo::Bandidos(x) => (4, x),
        Objetivo::Ejercito(x) => (5, x),
        Objetivo::Convocatoria(x) => (6, x),
        Objetivo::Faccion(f) => {
            w.u8(7);
            w.u32(f);
            return;
        }
        Objetivo::Punto(p) => {
            w.u8(8);
            w.punto(p);
            return;
        }
    };
    w.u8(clase);
    w.u64(id);
}

pub(crate) fn objetivo_leer(r: &mut Lector) -> Result<Objetivo, ErrorFormato> {
    Ok(match r.u8()? {
        0 => Objetivo::Plaza(r.u64()?),
        1 => Objetivo::Pueblo(r.u64()?),
        2 => Objetivo::Avanzada(r.u64()?),
        3 => Objetivo::Caravana(r.u64()?),
        4 => Objetivo::Bandidos(r.u64()?),
        5 => Objetivo::Ejercito(r.u64()?),
        6 => Objetivo::Convocatoria(r.u64()?),
        7 => Objetivo::Faccion(r.u32()?),
        8 => Objetivo::Punto(r.punto()?),
        _ => return Err(ErrorFormato("objetivo de mercenario desconocido")),
    })
}

fn puesto_escribir(w: &mut Escritor, p: &Puesto) {
    let (etiqueta, plaza) = match *p {
        Puesto::Rey => (0, None),
        Puesto::Embajador => (1, None),
        Puesto::General(x) => (2, Some(x)),
        Puesto::Gobernador(x) => (3, Some(x)),
        Puesto::MaestroCaravanas(x) => (4, Some(x)),
    };
    w.u8(etiqueta);
    if let Some(x) = plaza {
        w.u64(x);
    }
}

fn puesto_leer(r: &mut Lector) -> Result<Puesto, ErrorFormato> {
    Ok(match r.u8()? {
        0 => Puesto::Rey,
        1 => Puesto::Embajador,
        2 => Puesto::General(r.u64()?),
        3 => Puesto::Gobernador(r.u64()?),
        4 => Puesto::MaestroCaravanas(r.u64()?),
        _ => return Err(ErrorFormato("puesto desconocido")),
    })
}

fn medalla_a_u8(m: Medalla) -> u8 {
    Medalla::TODAS.iter().position(|&x| x == m).expect("TODAS lista cada medalla") as u8
}

fn medalla_de_u8(x: u8) -> Result<Medalla, ErrorFormato> {
    Medalla::TODAS.get(usize::from(x)).copied().ok_or(ErrorFormato("medalla desconocida"))
}

fn ficha_escribir(w: &mut Escritor, f: &Ficha) {
    let Ficha { renombre, rango, premios, contadores, grados, historial, leva_recibida } = f;
    w.u32(*renombre);
    w.u8(*rango);
    w.u8(*premios);
    w.u64(contadores.len() as u64);
    for (&m, &n) in contadores {
        w.u8(medalla_a_u8(m));
        w.u32(n);
    }
    w.u64(grados.len() as u64);
    for (&m, &g) in grados {
        w.u8(medalla_a_u8(m));
        w.u8(g);
    }
    let Historial { firmados, terminados, despidos, buenas, malas } = historial;
    [firmados, terminados, despidos, buenas, malas].into_iter().for_each(|&x| w.u32(x));
    w.bool(*leva_recibida);
}

fn ficha_leer(r: &mut Lector) -> Result<Ficha, ErrorFormato> {
    let (renombre, rango, premios) = (r.u32()?, r.u8()?, r.u8()?);
    let contadores: BTreeMap<Medalla, u32> = (0..r.cuenta()?).map(|_| Ok((medalla_de_u8(r.u8()?)?, r.u32()?))).collect::<Result<_, ErrorFormato>>()?;
    let grados: BTreeMap<Medalla, u8> = (0..r.cuenta()?).map(|_| Ok((medalla_de_u8(r.u8()?)?, r.u8()?))).collect::<Result<_, ErrorFormato>>()?;
    let historial = Historial { firmados: r.u32()?, terminados: r.u32()?, despidos: r.u32()?, buenas: r.u32()?, malas: r.u32()? };
    Ok(Ficha { renombre, rango, premios, contadores, grados, historial, leva_recibida: r.bool()? })
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use base::Punto;

    #[test]
    fn los_contratos_dan_la_vuelta_enteros() {
        let mut e = EstadoContratos::default();
        e.ofertas.insert(7, Oferta { campamento: 3, desde: 100 });
        let orden = Orden {
            tipo: TipoOrden::Atacar,
            objetivo: Objetivo::Pueblo(40),
            dada_por: 2,
            puesto: Some(Puesto::General(11)),
            dada_en: 500,
            vence: Some(900),
            contra: Some(4),
            unido: false,
        };
        let temporal = Contrato {
            mercenario: 8,
            faccion: 1,
            ambito: Ambito::Plaza(11),
            trabajo: Trabajo::Atacar,
            objetivo: Objetivo::Pueblo(40),
            plazo: Some(3),
            firmado_por: 2,
            puesto_firma: Puesto::Gobernador(11),
            firmado_en: 400,
            vence: Some(10_000),
            campamento_base: 3,
            orden: Some(orden),
            fin_aplazado: Some(Fin::Despido),
            renombre: 25,
        };
        let ir_a = Orden { tipo: TipoOrden::IrA, objetivo: Objetivo::Punto(Punto { x: 1_500, y: -2_000 }), vence: None, contra: None, unido: true, puesto: Some(Puesto::Rey), ..orden };
        let permanente = Contrato {
            mercenario: 9,
            ambito: Ambito::Faccion,
            trabajo: Trabajo::Permanente,
            objetivo: Objetivo::Faccion(1),
            plazo: None,
            puesto_firma: Puesto::Embajador,
            vence: None,
            orden: Some(ir_a),
            fin_aplazado: None,
            ..temporal.clone()
        };
        // Una orden de cada clase: la de cada puesto y la inicial del contrato, sin puesto.
        let puestos = [Some(Puesto::Rey), Some(Puesto::Embajador), Some(Puesto::General(11)), Some(Puesto::Gobernador(12)), Some(Puesto::MaestroCaravanas(13)), None];
        for (i, puesto) in puestos.into_iter().enumerate() {
            let mercenario = 20 + i as u64;
            e.contratos.insert(mercenario, Contrato { mercenario, orden: Some(Orden { puesto, ..orden }), ..temporal.clone() });
        }
        e.contratos.insert(8, temporal);
        e.contratos.insert(9, permanente);
        let mut ficha = Ficha::nueva();
        ficha.renombre = 120;
        ficha.premios = 1;
        ficha.contadores.insert(Medalla::Leal, 7);
        ficha.grados.insert(Medalla::Leal, 1);
        ficha.historial = Historial { firmados: 3, terminados: 2, despidos: 1, buenas: 1, malas: 0 };
        ficha.leva_recibida = true;
        e.fichas.insert(8, ficha);
        e.esperas.insert((8, 2), 7_000);
        e.terminados.insert((8, 2), Terminado { firmado_por: 5, renombre: 10, valorable_hasta: 8_000, valorado: true });
        let mut w = Escritor::default();
        escribir(&mut w, &e);
        let mut r = Lector::nuevo(&w.0);
        assert_eq!(leer(&mut r).unwrap(), e);
        r.fin().unwrap();
    }
}
