//! Del estado del núcleo a las entidades del contrato 05 (`Cambio`, `Entidad`, `EventoDominio`).

use crate::comando::{Detalle, TipoCreado};
use crate::estado::{Asentamiento, Ejercito, EventoNucleo};
use base::Instante;
use contrato::v1 as pb;
use militar::columna::{Politica, TipoColumna};
use std::collections::BTreeMap;

pub mod ambito;
pub mod ids;

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
            id: ids::ejercito(e.id),
            version: e.version,
            faccion_id: c.faccion.map_or(String::new(), ids::faccion),
            participantes: c.participantes.iter().map(|p| pb::Participante { heroe_id: ids::heroe(p.heroe), unido_en_ms: p.unido_en }).collect(),
            tipo: if c.tipo == TipoColumna::Personal { pb::TipoColumna::ColumnaPersonal } else { pb::TipoColumna::ColumnaEjercito } as i32,
            lider_id: ids::heroe(c.lider),
            politica_de_union: politica as i32,
            peticiones_de_union: c.peticiones.iter().map(|p| pb::PeticionUnion { heroe_id: ids::heroe(p.heroe), pedido_en_ms: p.pedido_en, expira_en_ms: p.expira_en }).collect(),
            escuadron_ids: c.escuadras.iter().map(|s| ids::escuadra(*s)).collect(),
            suministro: Some(pb::Cantidades { por_id: c.carga.iter().map(|(r, n)| (r.nombre().to_owned(), *n)).collect() }),
            viveres: Some(pb::Ritmo { valor_en_t0: v.valor_en_t0, t0_ms: v.t0, por_minuto: v.por_minuto, maximo: v.maximo, minimo: v.minimo, ..Default::default() }),
            capacidad_viveres: v.maximo.unwrap_or(0),
            formacion_expira_en_ms: c.formacion_expira_en,
            caravanas_adjuntas_ids: e.caravanas_adjuntas.iter().map(|c| ids::caravana(*c)).collect(),
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
            id: ids::asentamiento(a.id),
            version: a.version,
            faccion_id: ids::faccion(a.faccion),
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
        id: ids::ejercito(e.id),
        datos: Some(entidad_ejercito(e, t)),
        reemplaza: true,
        ..Default::default()
    }
}

/// La plaza como cambio. DUDA: entera y con `reemplaza` mientras la entidad sea la ficha de 9 campos; en T3c pasa a la forma parcial con máscara (`reemplaza = false`, `campos`), que es la de la plaza.
pub fn cambio_asentamiento(a: &Asentamiento, zona: &[base::Punto]) -> pb::Cambio {
    pb::Cambio {
        tipo: pb::TipoCambio::CambioActualiza as i32,
        entidad: pb::TipoEntidad::Asentamiento as i32,
        id: ids::asentamiento(a.id),
        datos: Some(entidad_asentamiento(a, zona)),
        reemplaza: true,
        ..Default::default()
    }
}

/// La entidad deja de formar parte del ámbito (`QUITA`: sin datos).
pub fn cambio_quita(entidad: pb::TipoEntidad, id: String) -> pb::Cambio {
    pb::Cambio { tipo: pb::TipoCambio::CambioQuita as i32, entidad: entidad as i32, id, ..Default::default() }
}

fn numero(n: f64) -> prost_types::Value {
    prost_types::Value { kind: Some(prost_types::value::Kind::NumberValue(n)) }
}

fn texto(s: String) -> prost_types::Value {
    prost_types::Value { kind: Some(prost_types::value::Kind::StringValue(s)) }
}

/// `RechazoOrden.detalle`: para `guerra.confirmar`, `{ objetivo: "f-N", facciones: ["f-N"...] }` (la Facción contra la que se declararía y las que entrarían por arrastre).
pub fn detalle_de_rechazo(d: &Detalle) -> prost_types::Struct {
    let Detalle::Guerra { objetivo, arrastre } = d;
    let arrastre = prost_types::ListValue { values: arrastre.iter().map(|&f| texto(ids::faccion(f))).collect() };
    let campos = [("objetivo".to_owned(), texto(ids::faccion(*objetivo))), ("facciones".to_owned(), prost_types::Value { kind: Some(prost_types::value::Kind::ListValue(arrastre)) })];
    prost_types::Struct { fields: campos.into_iter().collect() }
}

/// Una entidad creada por una orden (`ResultadoOrden.creadas`), con su clase y su id del contrato. `None` si el contrato no la tiene como entidad (la parcela de administración).
/// `uuid_de_batalla` da el id de una batalla en el contrato (la incorporación se nombra `<battle_id>:<secuencia>`).
pub fn referencia_creada(tipo: TipoCreado, id: u64, uuid_de_batalla: &dyn Fn(u64) -> String) -> Option<pb::ReferenciaCreada> {
    use pb::TipoEntidad as T;
    let (entidad, id) = match tipo {
        TipoCreado::Ejercito => (T::Ejercito, ids::ejercito(id)),
        TipoCreado::Asentamiento => (T::Asentamiento, ids::asentamiento(id)),
        TipoCreado::Heroe => (T::Heroe, ids::heroe(id)),
        TipoCreado::Escuadra => (T::Escuadron, ids::escuadra(id)),
        TipoCreado::Caravana => (T::Caravana, ids::caravana(id)),
        TipoCreado::Convocatoria => (T::Convocatoria, ids::convocatoria(id)),
        TipoCreado::CampamentoBandidos => (T::CampamentoBandidos, ids::bandidos(id)),
        TipoCreado::Acuerdo => (T::AcuerdoTrueque, ids::acuerdo(id)),
        TipoCreado::OrdenMercado => (T::OrdenMercado, ids::orden_de_mercado(id)),
        // DUDA: el contrato también tiene `Pueblo` y `Avanzada`; mientras la obra no termina el cliente la ve como `ObraDeMapa` (y a ella entrega los materiales).
        TipoCreado::Pueblo | TipoCreado::Avanzada => (T::ObraDeMapa, ids::obra(id)),
        TipoCreado::Loadout => (T::Loadout, ids::loadout(id)),
        TipoCreado::Edificio { plaza } => (T::Edificio, ids::edificio(plaza, u32::try_from(id).ok()?)),
        TipoCreado::Recinto { plaza } => (T::Recinto, ids::recinto(plaza, u32::try_from(id).ok()?)),
        TipoCreado::Incorporacion { batalla } => (T::Incorporacion, format!("{}:{id}", uuid_de_batalla(batalla))),
        TipoCreado::Parcela => return None,
    };
    Some(pb::ReferenciaCreada { entidad: entidad as i32, id })
}

pub fn evento_dominio(ev: &EventoNucleo, version: u64, heroes: Vec<String>) -> pb::EventoDominio {
    let mut campos: BTreeMap<String, prost_types::Value> = BTreeMap::new();
    if let Some(&id) = ev.ejercitos.first() {
        campos.insert("ejercitoId".into(), texto(ids::ejercito(id)));
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
        asentamiento_id: ev.asentamientos.first().map(|&id| ids::asentamiento(id)),
        heroes_ids: heroes,
        payload: Some(prost_types::Struct { fields: campos }),
        ..Default::default()
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn cada_clase_creada_sale_con_su_tipo_y_su_id_del_contrato() {
        let uuid = |b: u64| format!("uuid-{b}");
        let ref_de = |t: TipoCreado, id: u64| referencia_creada(t, id, &uuid).map(|r| (pb::TipoEntidad::try_from(r.entidad).unwrap(), r.id));
        use pb::TipoEntidad as T;
        assert_eq!(ref_de(TipoCreado::Ejercito, 1), Some((T::Ejercito, "ej-1".into())));
        assert_eq!(ref_de(TipoCreado::Asentamiento, 2), Some((T::Asentamiento, "as-2".into())));
        assert_eq!(ref_de(TipoCreado::Heroe, 3), Some((T::Heroe, "heroe-3".into())));
        assert_eq!(ref_de(TipoCreado::Escuadra, 4), Some((T::Escuadron, "esc-4".into())));
        assert_eq!(ref_de(TipoCreado::Caravana, 5), Some((T::Caravana, "car-5".into())));
        assert_eq!(ref_de(TipoCreado::Convocatoria, 6), Some((T::Convocatoria, "conv-6".into())));
        assert_eq!(ref_de(TipoCreado::CampamentoBandidos, 7), Some((T::CampamentoBandidos, "band-7".into())));
        assert_eq!(ref_de(TipoCreado::Acuerdo, 8), Some((T::AcuerdoTrueque, "acu-8".into())));
        assert_eq!(ref_de(TipoCreado::OrdenMercado, 9), Some((T::OrdenMercado, "ord-9".into())));
        assert_eq!(ref_de(TipoCreado::Loadout, 10), Some((T::Loadout, "loadout-10".into())));
        // El pueblo y la avanzada recién marcados son obras de mapa; el edificio y el recinto llevan su plaza; la incorporación, la batalla.
        assert_eq!(ref_de(TipoCreado::Pueblo, 11), Some((T::ObraDeMapa, "obra-11".into())));
        assert_eq!(ref_de(TipoCreado::Avanzada, 12), Some((T::ObraDeMapa, "obra-12".into())));
        assert_eq!(ref_de(TipoCreado::Edificio { plaza: 3 }, 14), Some((T::Edificio, "ed-3-14".into())));
        assert_eq!(ref_de(TipoCreado::Recinto { plaza: 3 }, 2), Some((T::Recinto, "recinto-3-2".into())));
        assert_eq!(ref_de(TipoCreado::Incorporacion { batalla: 15 }, 1), Some((T::Incorporacion, "uuid-15:1".into())));
        // Un id de edificio que no cabe en 32 bits no es de ninguna plaza; la parcela del administrador no es una entidad del contrato.
        assert_eq!(ref_de(TipoCreado::Edificio { plaza: 3 }, u64::MAX), None);
        assert_eq!(ref_de(TipoCreado::Parcela, 1), None);
    }
}
