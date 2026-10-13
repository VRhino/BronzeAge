//! Del estado del núcleo a las entidades del contrato 05 (`Cambio`, `Entidad`, `EventoDominio`).

use crate::estado::{Asentamiento, Ejercito, EventoNucleo};
use base::Instante;
use contrato::v1 as pb;
use std::collections::BTreeMap;

pub fn id_ejercito(id: u64) -> String {
    format!("ej-{id}")
}

pub fn id_asentamiento(id: u64) -> String {
    format!("as-{id}")
}

pub fn parse_id_ejercito(s: &str) -> Option<u64> {
    s.strip_prefix("ej-")?.parse().ok()
}

pub fn ambito_heroe(cliente: &str) -> pb::Ambito {
    pb::Ambito { tipo: pb::AmbitoTipo::AmbitoHeroe as i32, id: cliente.into() }
}

/// Clave de `Hola.versiones_conocidas` del ámbito del héroe.
pub fn clave_ambito_heroe(cliente: &str) -> String {
    format!("HEROE:{cliente}")
}

fn punto(p: base::Punto) -> pb::Punto {
    pb::Punto { x: p.x, y: p.y }
}

/// La columna como entidad del contrato. Lo propio viaja con el plan entero; sin plan, con su `posicion`.
pub fn entidad_ejercito(e: &Ejercito, t: Instante) -> pb::Entidad {
    let v = &e.viveres;
    let plan = e.marchando().then(|| pb::Plan {
        generacion: e.plan.generacion,
        vertices: e.plan.vertices.iter().map(|v| pb::Vertice { p: Some(punto(v.p)), en_ms: v.en }).collect(),
    });
    pb::Entidad {
        cuerpo: Some(pb::entidad::Cuerpo::Ejercito(pb::Ejercito {
            id: id_ejercito(e.id),
            version: e.version,
            faccion_id: format!("f{}", e.faccion),
            participantes: vec![pb::Participante { heroe_id: e.lider.clone(), unido_en_ms: 0 }],
            tipo: pb::TipoColumna::ColumnaPersonal as i32,
            lider_id: e.lider.clone(),
            politica_de_union: pb::PoliticaUnion::UnionRechazar as i32,
            viveres: Some(pb::Ritmo { valor_en_t0: v.valor_en_t0, t0_ms: v.t0, por_minuto: v.por_minuto, maximo: v.maximo, minimo: v.minimo }),
            capacidad_viveres: e.capacidad_viveres,
            estado: if e.marchando() { pb::EstadoColumna::ColumnaMarchando } else { pb::EstadoColumna::ColumnaEstacionada } as i32,
            posicion: Some(punto(e.posicion_en(t))),
            plan,
            ..Default::default()
        })),
    }
}

/// La plaza como entidad del contrato. `zona` es el polígono real recortado (`Nucleo::poligono_zona`); el radio viaja como
/// ritmo (valor, por minuto, techo) para que el cliente anime el crecimiento sin nuevos mensajes.
pub fn entidad_asentamiento(a: &Asentamiento, zona: &[base::Punto]) -> pb::Entidad {
    let r = &a.radio;
    pb::Entidad {
        cuerpo: Some(pb::entidad::Cuerpo::Asentamiento(pb::Asentamiento {
            id: id_asentamiento(a.id),
            version: a.version,
            faccion_id: format!("f{}", a.faccion),
            posicion: Some(punto(a.posicion)),
            nivel: a.nivel,
            nivel_actual: a.nivel,
            fundado_en_ms: a.fundado_en,
            radio_potencial: Some(pb::Ritmo { valor_en_t0: r.valor_en_t0, t0_ms: r.t0, por_minuto: r.por_minuto, maximo: r.maximo, minimo: r.minimo }),
            zona: zona.iter().map(|&p| punto(p)).collect(),
            ..Default::default()
        })),
    }
}

pub fn cambio_ejercito(e: &Ejercito, t: Instante) -> pb::Cambio {
    pb::Cambio {
        tipo: pb::TipoCambio::CambioActualiza as i32,
        entidad: pb::TipoEntidad::Ejercito as i32,
        id: id_ejercito(e.id),
        datos: Some(entidad_ejercito(e, t)),
        campos_borrados: vec![],
    }
}

fn numero(n: f64) -> prost_types::Value {
    prost_types::Value { kind: Some(prost_types::value::Kind::NumberValue(n)) }
}

pub fn evento_dominio(ev: &EventoNucleo, version: u64, heroes: Vec<String>) -> pb::EventoDominio {
    let mut campos: BTreeMap<String, prost_types::Value> = BTreeMap::new();
    if let Some(&id) = ev.ejercitos.first() {
        campos.insert("ejercitoId".into(), prost_types::Value { kind: Some(prost_types::value::Kind::StringValue(id_ejercito(id))) });
    }
    if let Some(p) = ev.punto {
        campos.insert("x".into(), numero(p.x as f64));
        campos.insert("y".into(), numero(p.y as f64));
    }
    for (k, v) in &ev.datos {
        campos.insert((*k).into(), numero(*v as f64));
    }
    pb::EventoDominio {
        codigo: ev.codigo.to_string(),
        version,
        en_ms: ev.en,
        asentamiento_id: ev.asentamientos.first().map(|&id| id_asentamiento(id)),
        heroes_ids: heroes,
        payload: Some(prost_types::Struct { fields: campos }),
    }
}
