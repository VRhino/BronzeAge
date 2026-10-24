//! Del estado del núcleo a las entidades del contrato 05 (`Cambio`, `Entidad`, `EventoDominio`).
//!
//! - [`ambito`]: los ámbitos de la sesión. [`ids`]: los ids de las entidades.
//! - [`interior`]: la plaza y el campamento de mercenarios vistos desde dentro (los ámbitos `ASENTAMIENTO` y `CAMPAMENTO`), con la máscara de campos de los cambios parciales de la plaza.
//! - [`propios`]: las entidades de los ámbitos `HEROE` y `FACCION` (y la `Batalla` y el informe de inspección). [`audiencia`]: a qué ámbito pertenece cada entidad del registro de cambios. [`batalla`]: el ticket, las incorporaciones y el resultado de una batalla de Unity.
//! - [`eventos`]: los hechos del núcleo como `EventoDominio`, con su payload y, en los quince que mueven la interfaz, su mensaje tipado.

use crate::comando::{Detalle, TipoCreado};
use crate::estado::{Asentamiento, Ejercito};
use contrato::v1 as pb;
use militar::columna::{Politica, TipoColumna};

pub mod ambito;
pub mod audiencia;
pub mod batalla;
pub mod eventos;
pub mod ids;
pub mod interior;
pub mod propios;
pub mod vista;

fn punto(p: base::Punto) -> pb::Punto {
    pb::Punto { x: p.x, y: p.y }
}

fn plan(p: &base::Plan) -> pb::Plan {
    pb::Plan { generacion: p.generacion, vertices: p.vertices.iter().map(|v| pb::Vertice { p: Some(punto(v.p)), en_ms: v.en }).collect() }
}

fn ritmo(r: base::Ritmo) -> pb::Ritmo {
    pb::Ritmo { valor_en_t0: r.valor_en_t0, t0_ms: r.t0, por_minuto: r.por_minuto, maximo: r.maximo, minimo: r.minimo, ..Default::default() }
}

fn politica(p: Politica) -> pb::PoliticaUnion {
    match p {
        Politica::Rechazar => pb::PoliticaUnion::UnionRechazar,
        Politica::Aceptar => pb::PoliticaUnion::UnionAceptar,
        Politica::Preguntar => pb::PoliticaUnion::UnionPreguntar,
    }
}

/// La columna como entidad del contrato, sin lo que depende del resto del mundo (`propios::ejercito` lo completa). Lo propio viaja con el plan entero; sin plan, con su `posicion` (con plan, la de
/// su primer vértice: así la proyección es función del estado y no del instante en que se pide, y una columna que anda no hay que mandarla otra vez).
fn ejercito_base(e: &Ejercito) -> pb::Ejercito {
    let c = &e.columna;
    let v = &c.viveres;
    pb::Ejercito {
        id: ids::ejercito(e.id),
        version: e.version,
        faccion_id: c.faccion.map_or(String::new(), ids::faccion),
        participantes: c.participantes.iter().map(|p| pb::Participante { heroe_id: ids::heroe(p.heroe), unido_en_ms: p.unido_en }).collect(),
        tipo: if c.tipo == TipoColumna::Personal { pb::TipoColumna::ColumnaPersonal } else { pb::TipoColumna::ColumnaEjercito } as i32,
        lider_id: ids::heroe(c.lider),
        politica_de_union: politica(c.politica) as i32,
        peticiones_de_union: c.peticiones.iter().map(|p| pb::PeticionUnion { heroe_id: ids::heroe(p.heroe), pedido_en_ms: p.pedido_en, expira_en_ms: p.expira_en }).collect(),
        escuadron_ids: c.escuadras.iter().map(|s| ids::escuadra(*s)).collect(),
        suministro: Some(pb::Cantidades { por_id: c.carga.iter().map(|(r, n)| (r.nombre().to_owned(), *n)).collect() }),
        viveres: Some(pb::Ritmo { valor_en_t0: v.valor_en_t0, t0_ms: v.t0, por_minuto: v.por_minuto, maximo: v.maximo, minimo: v.minimo, ..Default::default() }),
        capacidad_viveres: v.maximo.unwrap_or(0),
        formacion_expira_en_ms: c.formacion_expira_en,
        caravanas_adjuntas_ids: e.caravanas_adjuntas.iter().map(|c| ids::caravana(*c)).collect(),
        estado: if e.marchando() { pb::EstadoColumna::ColumnaMarchando } else { pb::EstadoColumna::ColumnaEstacionada } as i32,
        posicion: Some(punto(e.plan.vertices.first().map_or(e.posicion, |v| v.p))),
        plan: e.marchando().then(|| plan(&e.plan)),
        ..Default::default()
    }
}

/// La plaza como ficha del contrato (la que va a `FACCION`; el interior es de `proyeccion::interior`). `zona` es el polígono real recortado (`Nucleo::poligono_zona`) y `batalla_id`, la batalla que se juega ahora contra ella (el asedio visible). Es la ficha: id, Facción,
/// sitio, nivel, zona, los ids de sus pueblos y avanzadas y el asedio.
pub fn ficha_de_asentamiento(a: &Asentamiento, zona: &[base::Punto], batalla_id: Option<String>) -> pb::Asentamiento {
    pb::Asentamiento {
        id: ids::asentamiento(a.id),
        version: a.version,
        faccion_id: ids::faccion(a.faccion),
        posicion: Some(punto(a.posicion)),
        nivel: a.nivel(),
        nivel_actual: a.nivel_actual(),
        fundado_en_ms: a.fundado_en,
        radio_potencial: a.radio,
        zona: zona.iter().map(|&p| punto(p)).collect(),
        pueblos_ids: a.economia.pueblos.iter().map(|p| ids::pueblo(p.id)).collect(),
        avanzadas_ids: a.economia.avanzadas.iter().map(|v| ids::avanzada(v.id)).collect(),
        batalla_id,
        nombre: (!a.nombre.is_empty()).then(|| a.nombre.clone()),
        capital_de_faccion_id: a.capital_de.map(ids::faccion),
        ..Default::default()
    }
}

/// La entidad deja de formar parte del ámbito (`QUITA`: sin datos).
pub fn cambio_quita(entidad: pb::TipoEntidad, id: String) -> pb::Cambio {
    pb::Cambio { tipo: pb::TipoCambio::CambioQuita as i32, entidad: entidad as i32, id, ..Default::default() }
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
        TipoCreado::Mirada => (T::MiradaIntel, ids::mirada(id)),
        TipoCreado::Faccion => (T::Faccion, ids::faccion(u32::try_from(id).ok()?)),
        TipoCreado::Propuesta => (T::Propuesta, ids::propuesta(id)),
        TipoCreado::Parcela => return None,
    };
    Some(pb::ReferenciaCreada { entidad: entidad as i32, id })
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
        assert_eq!(ref_de(TipoCreado::Mirada, 16), Some((T::MiradaIntel, "mirada-16".into())));
        // Un id de edificio que no cabe en 32 bits no es de ninguna plaza; la parcela del administrador no es una entidad del contrato.
        assert_eq!(ref_de(TipoCreado::Edificio { plaza: 3 }, u64::MAX), None);
        assert_eq!(ref_de(TipoCreado::Parcela, 1), None);
    }
}
