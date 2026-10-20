//! Del estado del núcleo a las entidades del contrato 05 (`Cambio`, `Entidad`, `EventoDominio`).

use crate::estado::{Asentamiento, Ejercito, EventoNucleo};
use base::Instante;
use contrato::v1 as pb;
use militar::columna::{Politica, TipoColumna};
use std::collections::BTreeMap;

pub fn id_ejercito(id: u64) -> String {
    format!("ej-{id}")
}

/// El id de un héroe en el contrato (`heroe-{id}`): lo que viaja en `Orden*.heroe_id` y en las listas de participantes.
pub fn id_heroe(id: u64) -> String {
    format!("heroe-{id}")
}

pub fn parse_id_heroe(s: &str) -> Option<u64> {
    s.strip_prefix("heroe-")?.parse().ok()
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
    let c = &e.columna;
    let v = &c.viveres;
    let plan = e.marchando().then(|| pb::Plan {
        generacion: e.plan.generacion,
        vertices: e.plan.vertices.iter().map(|v| pb::Vertice { p: Some(punto(v.p)), en_ms: v.en }).collect(),
    });
    let politica = match c.politica {
        Politica::Rechazar => pb::PoliticaUnion::UnionRechazar,
        Politica::Aceptar => pb::PoliticaUnion::UnionAceptar,
        Politica::Preguntar => pb::PoliticaUnion::UnionPreguntar,
    };
    pb::Entidad {
        cuerpo: Some(pb::entidad::Cuerpo::Ejercito(pb::Ejercito {
            id: id_ejercito(e.id),
            version: e.version,
            faccion_id: c.faccion.map_or(String::new(), |f| format!("f{f}")),
            participantes: c.participantes.iter().map(|p| pb::Participante { heroe_id: id_heroe(p.heroe), unido_en_ms: p.unido_en }).collect(),
            tipo: if c.tipo == TipoColumna::Personal { pb::TipoColumna::ColumnaPersonal } else { pb::TipoColumna::ColumnaEjercito } as i32,
            lider_id: id_heroe(c.lider),
            politica_de_union: politica as i32,
            peticiones_de_union: c.peticiones.iter().map(|p| pb::PeticionUnion { heroe_id: id_heroe(p.heroe), pedido_en_ms: p.pedido_en, expira_en_ms: p.expira_en }).collect(),
            escuadron_ids: c.escuadras.iter().map(|s| format!("esc-{s}")).collect(),
            suministro: Some(pb::Cantidades { por_id: c.carga.iter().map(|(r, n)| (r.nombre().to_owned(), *n)).collect() }),
            viveres: Some(pb::Ritmo { valor_en_t0: v.valor_en_t0, t0_ms: v.t0, por_minuto: v.por_minuto, maximo: v.maximo, minimo: v.minimo }),
            capacidad_viveres: v.maximo.unwrap_or(0),
            formacion_expira_en_ms: c.formacion_expira_en,
            caravanas_adjuntas_ids: e.caravanas_adjuntas.iter().map(|c| format!("car-{c}")).collect(),
            estado: if e.marchando() { pb::EstadoColumna::ColumnaMarchando } else { pb::EstadoColumna::ColumnaEstacionada } as i32,
            posicion: Some(punto(e.posicion_en(t))),
            plan,
            ..Default::default()
        })),
    }
}

/// La plaza como entidad del contrato. `zona` es el polígono real recortado (`Nucleo::poligono_zona`).
pub fn entidad_asentamiento(a: &Asentamiento, zona: &[base::Punto]) -> pb::Entidad {
    pb::Entidad {
        cuerpo: Some(pb::entidad::Cuerpo::Asentamiento(pb::Asentamiento {
            id: id_asentamiento(a.id),
            version: a.version,
            faccion_id: format!("f{}", a.faccion),
            posicion: Some(punto(a.posicion)),
            nivel: a.nivel(),
            nivel_actual: a.nivel_actual(),
            fundado_en_ms: a.fundado_en,
            radio_potencial: a.radio,
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
