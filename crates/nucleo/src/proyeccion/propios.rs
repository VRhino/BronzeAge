//! Las entidades de los ámbitos propios, `HEROE` y `FACCION` (contrato 05 §3, 4H T3b), y el campamento de mercenarios (`CAMPAMENTO`): el héroe con sus escuadras y loadouts, las columnas en que participa, las caravanas que le
//! tocan, las convocatorias y las batallas en que combate; y de la Facción, sus plazas en resumen, su flota, sus relaciones y tecnologías, sus pueblos y avanzadas (con sus obras),
//! sus trueques y sus órdenes de mercado; y los campamentos de bandidos. Cada función va del estado del núcleo a la entidad del contrato; [`proyectar`] reúne las de una clase del registro de cambios.
//! `ADMIN` (T3e) usa estas mismas proyecciones para todo el mundo, con [`Espectador::Admin`].
//!
//! Qué ámbito recibe qué lo decide [`super::audiencia`]; aquí solo se dice **qué es** cada cosa en el contrato. Las entidades viajan completas (`reemplaza = true`, decisión 2).
//!
//! DUDA: el contrato trae más de lo que el núcleo guarda; hasta que existan sus sistemas, esos campos van a su valor por defecto: nombre, clase, género y avatar del héroe
//! (decisión 4: «Héroe N», clase `default`), inventario y equipo (Conquest, CQ-004) y el titular de una caravana. (El almacén de una avanzada va por recurso desde el 2026-10-27.)

use super::{audiencia, batalla, ids, plan, punto, ritmo};
use crate::estado::{Clase, Ejercito};
use crate::estado_militar::{HeroeNucleo, PlazaRecordada};
use politica::relaciones::{RelacionPolitica, TipoRelacion};
use crate::nucleo::{Nucleo, TokenDeJugador};
use asentamiento::avanzadas::{Avanzada, Estado as EstadoAvanzada, TipoObra};
use asentamiento::pueblos::{EstadoPueblo, Pueblo};
use asentamiento::{Almacen, Recurso};
use base::Mil;
use combate::ciclo::{Batalla, EstadoBatalla};
use comercio::caravana::{Animal, Caravana, EstadoCaravana, TipoCaravana, TipoCarro};
use comercio::orden_mercado::{EstadoOrden, OrdenMercado, TipoOrden};
use comercio::trueque::{AcuerdoTrueque, EstadoTrueque, Linea};
use contrato::v1 as pb;
use militar::convocatoria::{Convocatoria, Lugar, Seleccion};
use militar::escuadra::{Contenedor, Escuadra};
use militar::heroe::Residencia;
use militar::inspeccion::{Avistada, Contenido, Informe, Objetivo};
use militar::presencia::Ubicacion;
use militar::tipos::Carga;
use pb::entidad::Cuerpo;
use std::collections::BTreeMap;

/// Una entidad del contrato lista para viajar: su clase, su id y su cuerpo.
pub type Proyectada = (pb::TipoEntidad, String, Cuerpo);

/// El id del estado de tecnología del mundo en el contrato (una sola entidad).
pub const ID_ESTADO_TECNOLOGIA: &str = "tecnologia";

/// Quién mira: el héroe de una sesión de jugador (`None` sin héroe) o la administración (`ADMIN`), que lo ve todo sin redactar menos lo que es secreto de cada jugador (los tokens de batalla).
/// Decide lo que de una entidad depende de quién la mira: las peticiones de una convocatoria y lo que se ve de una batalla.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Espectador {
    Heroe(Option<u64>),
    Admin,
}

pub(super) fn cantidades(c: impl IntoIterator<Item = (Recurso, Mil)>) -> pb::Cantidades {
    pb::Cantidades { por_id: c.into_iter().map(|(r, n)| (r.nombre().to_owned(), n)).collect() }
}

fn carga(c: &Carga) -> pb::Cantidades {
    cantidades(c.iter().map(|(&r, &n)| (r, n)))
}

/// El ritmo constante de una cifra que solo cambia a saltos: como no tiene pendiente, su `t0` no importa y vale 0, así que la proyección es una función del estado y no del instante en que se pide.
pub(super) fn constante(valor: Mil) -> pb::Ritmo {
    ritmo(base::Ritmo::constante(valor, 0))
}

/// Lo que hay de un recurso, en escalones (la economía va por minuto): `tendencia` es el neto del último paso de minuto, «+N/min»; un almacén sin tendencia (el de un pueblo) la lleva a 0.
pub(super) fn existencia(a: &Almacen, r: Recurso, tendencia: Mil) -> pb::Existencia {
    pb::Existencia { cantidad: Some(pb::Ritmo { en_escalones: true, maximo: Some(a.capacidad(r)), por_minuto: tendencia, ..constante(a.visible(r)) }), capacidad: a.capacidad(r) }
}

/// Las celdas de un anillo de muralla en el contrato (columna, fila y clase).
pub(super) fn celdas(cs: &[trazado::CeldaMuro]) -> Vec<pb::CeldaMuro> {
    cs.iter()
        .map(|c| pb::CeldaMuro {
            col: c.celda.col,
            fila: c.celda.row,
            clase: match c.clase {
                trazado::ClaseCelda::Muro => pb::ClaseCelda::CeldaMuro,
                trazado::ClaseCelda::Puerta => pb::ClaseCelda::CeldaPuerta,
                trazado::ClaseCelda::Torre => pb::ClaseCelda::CeldaTorre,
            } as i32,
        })
        .collect()
}

/// El id de la batalla que se juega ahora contra un lugar (la plaza, el pueblo o la avanzada), si la hay: el asedio visible.
pub(super) fn batalla_sobre(n: &Nucleo, lugar: Ubicacion) -> Option<String> {
    n.batalla_sobre(lugar).map(|b| n.uuid_de_batalla(b.id))
}

// ───────────────────────────── Héroe, escuadras, loadouts ─────────────────────────────

/// La clase de un héroe hasta que Conquest publique su catálogo (decisión 4 del autor).
pub(super) const CLASE_POR_DEFECTO: &str = "default";

/// El nombre de un héroe: el que eligió el jugador al crearlo (2026-10-27) o, si no eligió ninguno, «Héroe N».
pub(super) fn nombre_de_heroe(n: &Nucleo, id: u64) -> String {
    n.estado().militar.nombres.get(&id).cloned().unwrap_or_else(|| format!("Héroe {id}"))
}

/// Los atributos base de un héroe (sin equipo: el efectivo es el base hasta CQ-004).
pub(super) fn atributos(h: &militar::heroe::Heroe) -> pb::Atributos {
    pb::Atributos { fuerza: h.atributos.fuerza.max(0) as u32, destreza: h.atributos.destreza.max(0) as u32, armadura: h.atributos.armadura.max(0) as u32, vitalidad: h.atributos.vitalidad.max(0) as u32 }
}

/// El héroe propio. DUDA: nombre, clase, género y avatar por defecto hasta que Conquest publique su catálogo (decisión 4 del autor); inventario, equipo y exploración personal, vacíos.
pub fn heroe(n: &Nucleo, h: &HeroeNucleo) -> pb::Heroe {
    let e = n.estado();
    let ubicacion = match h.heroe.presencia.ubicacion {
        Ubicacion::Asentamiento(a) => pb::ubicacion_heroe::Donde::AsentamientoId(ids::asentamiento(u64::from(a))),
        Ubicacion::Columna(c) => pb::ubicacion_heroe::Donde::EjercitoId(ids::ejercito(c)),
        Ubicacion::Mercenarios(c) => pb::ubicacion_heroe::Donde::CampamentoId(ids::campamento(c)),
        Ubicacion::Pueblo(p) => pb::ubicacion_heroe::Donde::PuebloId(ids::pueblo(p)),
        Ubicacion::Avanzada(a) => pb::ubicacion_heroe::Donde::AvanzadaId(ids::avanzada(a)),
        Ubicacion::Desconectado(p) => pb::ubicacion_heroe::Donde::DesconectadoEn(punto(p)),
    };
    let r = &e.reglas.economia.militar;
    let loadouts = h
        .heroe
        .loadouts
        .iter()
        .map(|l| pb::Loadout {
            id: ids::loadout(l.id),
            display_name: l.nombre.clone(),
            squad_ids: l.escuadras.iter().map(|&s| ids::escuadra(s)).collect(),
            perks_seleccionados: l.perks.clone(),
            activo: l.activo,
            liderazgo_total: u32::try_from(h.heroe.liderazgo_de_loadout(r, l)).unwrap_or(0),
        })
        .collect();
    let cupo_guarnicion = match h.heroe.residencia {
        Some(Residencia::Asentamiento(a)) => n.cupo_de_guarnicion(u64::from(a)).unwrap_or(0),
        _ => 0,
    };
    let residencia = h.heroe.residencia.map(|r| match r {
        Residencia::Asentamiento(a) => ids::asentamiento(u64::from(a)),
        Residencia::Campamento(c) => ids::campamento(c),
    });
    let con_algo = |c: &Carga| (!c.is_empty()).then(|| carga(c));
    pb::Heroe {
        id: ids::heroe(h.heroe.id),
        version: n.version_de(Clase::Heroe, h.heroe.id),
        jugador_id: (!h.sintetico).then(|| h.cuenta.clone()),
        es_bot: h.heroe.controlador == militar::heroe::Controlador::Bot,
        display_name: nombre_de_heroe(n, h.heroe.id),
        class_definition_id: CLASE_POR_DEFECTO.into(),
        avatar: Some(pb::Avatar::default()),
        nivel: h.heroe.nivel,
        experiencia_hacia_siguiente_nivel: h.heroe.experiencia.max(0) as u64,
        puntos_de_atributo_sin_gastar: h.heroe.puntos_de_atributo.max(0) as u32,
        puntos_de_perk_sin_gastar: h.heroe.puntos_de_perk.max(0) as u32,
        atributos_base: Some(atributos(&h.heroe)),
        perks_desbloqueados: h.heroe.perks_desbloqueados.clone(),
        liderazgo_base: h.heroe.liderazgo_base.max(0) as u32,
        ubicacion: Some(pb::UbicacionHeroe { donde: Some(ubicacion) }),
        almacen_personal: Some(carga(&h.almacen_personal)),
        capacidad_almacen_personal: n.estado().reglas.economia.comercio.mercenarios.capacidad_almacen_personal,
        cupo_campamento: (!h.cupo.comprado.is_empty()).then(|| pb::CupoCampamento {
            dia: u32::try_from(h.cupo.dia).unwrap_or(0),
            comprado: h.cupo.comprado.iter().map(|(r, &n)| (r.nombre().to_owned(), u32::try_from(n).unwrap_or(u32::MAX))).collect(),
        }),
        racion_en_ms: h.heroe.racion_en,
        viveres: Some(constante(base::entero_arriba(h.heroe.viveres))), // en unidades enteras: la que se está comiendo sigue hasta acabarse (EE-3)
        bandidos_destruidos_en_ms: e.combate.destruidos.get(&h.heroe.id).cloned().unwrap_or_default(),
        alijos_abiertos: h.alijos_abiertos.iter().filter_map(|&i| n.alijo(i).map(|a| a.id.clone())).collect(), // los ids del artefacto (6F)
        herido_hasta_ms: h.heroe.herido_hasta,
        desconecta_en_ms: h.heroe.presencia.desconecta_en,
        fuera_carro: con_algo(&h.carro),
        escuadrones_ids: h.heroe.escuadras.iter().map(|s| ids::escuadra(s.id)).collect(),
        loadouts,
        monedas_heroe: Some(pb::Monedas { bronce: h.heroe.monedas.bronce.max(0) as u64, plata: h.heroe.monedas.plata.max(0) as u64, oro: h.heroe.monedas.oro.max(0) as u64 }),
        cupo_guarnicion: u32::try_from(cupo_guarnicion).unwrap_or(0),
        residencia_id: residencia,
        faccion_id: h.faccion.map(ids::faccion),
        solicitudes_faccion_ids: e.politica.facciones.iter().filter(|f| f.solicitudes.contains(&h.heroe.id)).map(|f| ids::faccion(f.id)).collect(),
        plazas_recordadas: h.plazas_recordadas.iter().map(|(&plaza, x)| (ids::asentamiento(u64::from(plaza)), interior_recordado(u64::from(plaza), x))).collect(),
        misiones: misiones(n, crate::misiones::Dueno::Heroe(h.heroe.id)),
        ..Default::default()
    }
}

/// Las misiones de un dueño (Misiones §5), en el orden del catálogo: las activas con su avance, las cumplidas y las reclamadas. Las que esperan a su previa y las deshabilitadas no viajan.
pub fn misiones(n: &Nucleo, d: crate::misiones::Dueno) -> Vec<pb::Mision> {
    use crate::misiones::{DeDueno, Recompensa};
    let r = &n.estado().reglas.economia.misiones;
    let vacio = DeDueno::default();
    let x = n.misiones_de(d).unwrap_or(&vacio);
    r.de(d.clase())
        .filter_map(|(i, m)| {
            let estado = if x.reclamadas.contains(&i) {
                pb::EstadoMision::MisionReclamada
            } else if x.cumplidas.contains(&i) {
                pb::EstadoMision::MisionCumplida
            } else if x.activa(r, i) {
                pb::EstadoMision::MisionActiva
            } else {
                return None;
            };
            let hecha = estado != pb::EstadoMision::MisionActiva;
            let (recompensa, recompensa_experiencia) = match &m.recompensa {
                &Recompensa::Oro(oro) => (cantidades([(Recurso::Oro, oro)]), 0),
                Recompensa::Recursos(lista) => (cantidades(lista.iter().copied()), 0),
                &Recompensa::Experiencia(xp) => (pb::Cantidades::default(), xp),
            };
            Some(pb::Mision {
                id: m.id.clone(),
                estado: estado as i32,
                condiciones: m.condiciones.iter().map(|(c, u)| pb::CondicionMision { contador: c.clone(), avance: if hecha { *u } else { x.contador(c).min(*u) }, umbral: *u }).collect(),
                previa: m.previa.map(|p| r.misiones[p as usize].id.clone()),
                recompensa: Some(recompensa),
                recompensa_experiencia,
            })
        })
        .collect()
}

/// La foto del interior de una plaza que el héroe dejó al salir al mundo (Niebla_De_Guerra §4.5): el almacén como cifras fijas (sin capacidad ni tendencia: es lo que había), la cola con su
/// estado, su nivel y el fin de cada obra (sin sitio en la ciudad, que es de lo vivo) y la guarnición.
fn interior_recordado(plaza: u64, x: &PlazaRecordada) -> pb::InteriorRecordado {
    pb::InteriorRecordado {
        visto_en_ms: x.visto_en,
        almacen: x.almacen.iter().map(|(r, &n)| (r.nombre().to_owned(), pb::Existencia { cantidad: Some(constante(n)), capacidad: 0 })).collect(),
        cola: x
            .cola
            .iter()
            .map(|b| pb::Edificio {
                id: ids::edificio(plaza, b.id),
                tipo: b.tipo.nombre().to_owned(),
                estado: if b.completa_en.is_some() { pb::EstadoEdificio::EdificioEnConstruccion } else { pb::EstadoEdificio::EdificioEnCola } as i32,
                completa_en_ms: b.completa_en,
                ambito: pb::AmbitoEdificio::Asentamiento as i32,
                nivel_interno: Some(u32::from(b.nivel)),
                ..Default::default()
            })
            .collect(),
        guarnicion_escuadrones_ids: x.guarnicion.iter().map(|&s| ids::escuadra(s)).collect(),
    }
}

/// Una escuadra. La que está en la guarnición de una avanzada sale con `avanzada_id` (paridad, 2026-10-27; antes, «en el campamento»).
pub fn escuadron(n: &Nucleo, e: &Escuadra) -> pb::Escuadron {
    let contenedor = match e.contenedor {
        Contenedor::Campamento => pb::contenedor_escuadron::Donde::Campamento(true),
        Contenedor::Avanzada(a) => pb::contenedor_escuadron::Donde::AvanzadaId(ids::avanzada(a)),
        Contenedor::Ejercito(c) => pb::contenedor_escuadron::Donde::EjercitoId(ids::ejercito(c)),
        Contenedor::Escolta(c) => pb::contenedor_escuadron::Donde::CaravanaId(ids::caravana(c)),
        Contenedor::Fuera => pb::contenedor_escuadron::Donde::Fuera(true),
    };
    let def = n.estado().reglas.economia.militar.tropas.tropa(e.tropa);
    pb::Escuadron {
        id: ids::escuadra(e.id),
        version: n.version_de(Clase::Escuadra, e.id),
        nombre: def.nombre.clone(),
        heroe_id: ids::heroe(e.heroe),
        origen: match e.origen {
            militar::catalogo::Origen::Pesants => "pesants",
            militar::catalogo::Origen::Artesanos => "artesanos",
            militar::catalogo::Origen::Nobleza => "nobleza",
        }
        .into(),
        tropa_id: e.tropa.nombre().into(),
        cantidad: e.cantidad.max(0) as u32,
        nivel: u32::from(e.nivel),
        experiencia: e.experiencia,
        moral: Some(ritmo(e.moral)),
        contenedor: Some(pb::ContenedorEscuadron { donde: Some(contenedor) }),
        en_guarnicion: e.en_guarnicion,
        reserva_batalla: e.reserva_batalla.map(|b| pb::ReservaBatalla { battle_id: n.uuid_de_batalla(b) }),
        prestada_por_campamento_id: e.prestada.map(ids::campamento),
        ..Default::default()
    }
}

// ───────────────────────────── Informes de inspección ─────────────────────────────

pub(super) fn avistadas(a: &[Avistada]) -> Vec<pb::GuarnicionInforme> {
    a.iter().map(|x| pb::GuarnicionInforme { tropa_id: x.tropa.nombre().to_owned(), cantidad: x.cantidad.max(0) as u32, heroe_id: ids::heroe(x.heroe) }).collect()
}

/// El id de objetivo de un informe de inspección.
fn objetivo_de_informe(i: &Informe) -> String {
    match i.objetivo {
        Objetivo::Ejercito(x) => ids::ejercito(x),
        Objetivo::Caravana(x) => ids::caravana(x),
        Objetivo::Asentamiento(x) => ids::asentamiento(x),
    }
}

/// El id de un informe de inspección en el contrato (uno por héroe y objetivo).
pub fn id_de_informe(heroe: u64, i: &Informe) -> String {
    ids::informe(heroe, &objetivo_de_informe(i))
}

/// Lo que un héroe sacó de mirar de cerca (`inspeccionar`): una columna, una caravana o la defensa de una plaza con sus recintos. Lo ve solo quien lo hizo y vale hasta `expira_en_ms`; al caducar,
/// la sesión lo quita con `QUITA` en su reconciliación siguiente (`sesiones::ambitos`, paridad del 2026-10-27).
pub fn informe_de_inspeccion(n: &Nucleo, heroe: u64, i: &Informe) -> pb::InformeInspeccion {
    use pb::informe_inspeccion::Contenido as C;
    let objetivo = objetivo_de_informe(i);
    let contenido = match &i.contenido {
        Contenido::Columna { faccion, heroes, escuadras } => C::Columna(pb::InspeccionColumna { faccion_id: faccion.map(ids::faccion), heroes_ids: heroes.iter().map(|&h| ids::heroe(h)).collect(), escuadras: avistadas(escuadras) }),
        Contenido::Caravana { escoltada, recursos } => C::Caravana(pb::InspeccionCaravana { escoltada: *escoltada, recursos: recursos.iter().map(|r| r.nombre().to_owned()).collect() }),
        Contenido::Plaza { faccion, heroes, guarnicion, recintos } => C::Plaza(pb::InspeccionPlaza {
            faccion_id: ids::faccion(*faccion),
            heroes_ids: heroes.iter().map(|&h| ids::heroe(h)).collect(),
            guarnicion: avistadas(guarnicion),
            recintos: recintos.iter().map(|r| pb::RecintoInforme { nivel: u32::from(r.nivel), celdas: celdas(&r.celdas), avance: r.avance }).collect(),
        }),
    };
    pb::InformeInspeccion {
        id: ids::informe(heroe, &objetivo),
        version: n.version_de(Clase::Heroe, heroe),
        heroe_id: ids::heroe(heroe),
        objetivo_id: objetivo,
        visto_en_ms: i.visto_en,
        expira_en_ms: i.expira_en,
        donde: Some(punto(i.donde)),
        contenido: Some(contenido),
    }
}

// ───────────────────────────── Columnas, caravanas, convocatorias, batallas ─────────────────────────────

/// La columna, completa: con lo que persigue, si una batalla la tiene apartada y dentro de qué lugar está. `te_sigue` no va aquí: es de lo ajeno, y lo pone la vista del perseguido
/// (`vista::Mirada`, en la columna ajena que persigue a una propia).
pub fn ejercito(n: &Nucleo, e: &Ejercito) -> pb::Ejercito {
    let mut x = super::ejercito_base(e);
    x.version = e.version;
    x.persiguiendo = e.persiguiendo.map(|presa| {
        let es_caravana = crate::estado_comercio::es_entidad_caravana(presa);
        let id = if es_caravana { ids::caravana(crate::estado_comercio::id_de_entidad(presa)) } else { ids::ejercito(presa) };
        // El destino es el punto de intercepción: el final del plan del perseguidor (sin plan, ya espera en él; paridad P7).
        let destino = e.plan.vertices.last().map_or_else(|| e.posicion_en(n.ahora()), |v| v.p);
        pb::Persecucion { presa_es_caravana: es_caravana, presa_id: id, destino: Some(punto(destino)) }
    });
    x.en_batalla = e.en_batalla;
    // La marcha forzada solo la ve su dueño: `ejercito_base` (lo que también va a lo ajeno) la deja en false.
    x.marcha_forzada = e.columna.forzada;
    x.caravana_fundacion_id = n.estado().fundacion.de_columna(e.id).map(ids::caravana);
    x.batalla_id = e.en_batalla.then(|| n.batallas().find(|b| b.activa(n.ahora()) && b.bloqueo.ejercitos.contains(&e.id)).map(|b| n.uuid_de_batalla(b.id))).flatten();
    x.dentro_de = n.heroe(e.columna.lider).and_then(|h| match h.heroe.presencia.ubicacion {
        Ubicacion::Asentamiento(a) => Some(ids::asentamiento(u64::from(a))),
        Ubicacion::Pueblo(p) => Some(ids::pueblo(p)),
        Ubicacion::Avanzada(a) => Some(ids::avanzada(a)),
        Ubicacion::Mercenarios(c) => Some(ids::campamento(c)),
        Ubicacion::Columna(_) | Ubicacion::Desconectado(_) => None,
    });
    x
}

/// La Caravana de Fundación (5G-b, Fundacion_Y_Refundacion §2): suelta en su origen, enganchada a una columna (donde está la columna) o volviendo sola con su plan; con su titular, su caducidad,
/// lo que costó y, la de un campamento, lo que puso cada aportante. Su origen es la plaza (`as-N`) o el campamento (`camp-N`, también en `origen_campamento_id`); la de la administración no tiene.
pub fn caravana_de_fundacion(n: &Nucleo, c: &crate::estado_fundacion::CaravanaFundacion) -> pb::Caravana {
    use crate::estado_fundacion::{Origen, Situacion};
    let (e, t) = (n.estado(), n.ahora());
    let (estado, posicion, en_plan) = match &c.situacion {
        Situacion::Disponible => (pb::EstadoCaravana::CaravanaDisponible, c.posicion, None),
        Situacion::Adjunta(x) => (pb::EstadoCaravana::CaravanaAdjunta, e.ejercitos.get(x).map_or(c.posicion, |x| x.posicion_en(t)), None),
        Situacion::Retornando(p) => (pb::EstadoCaravana::CaravanaRetornando, c.posicion_en(t), Some(plan(p))),
    };
    let (origen, campamento) = match c.origen {
        Origen::Plaza(a) => (ids::asentamiento(a), None),
        Origen::Campamento(k) => (ids::campamento(k), Some(ids::campamento(k))),
        Origen::Ninguno => (String::new(), None),
    };
    pb::Caravana {
        id: ids::caravana(c.id),
        version: n.version_de(Clase::Caravana, c.id),
        tipo: pb::TipoCaravana::CaravanaDeFundacion as i32,
        origen_asentamiento_id: origen,
        origen_campamento_id: campamento,
        faccion_id: c.faccion.map(ids::faccion),
        contenido: Some(carga(&c.contenido)),
        posicion: Some(punto(posicion)),
        plan: en_plan,
        titular_id: Some(ids::heroe(c.titular)),
        caduca_en_ms: c.caduca_en,
        aportes: c.aportes.iter().map(|(&h, m)| (ids::heroe(h), carga(m))).collect(),
        estado: estado as i32,
        escoltada: matches!(c.situacion, Situacion::Adjunta(_)),
        ..Default::default()
    }
}

/// La caravana, con su estado en el contrato (una en batalla muestra el que tenía) y su carga entera: es de la Facción o la lleva una columna propia.
pub fn caravana(n: &Nucleo, c: &Caravana) -> pb::Caravana {
    let (e, t) = (n.estado(), n.ahora());
    let faccion = e.asentamientos.get(&u64::from(c.origen)).map(|a| ids::faccion(a.faccion));
    let estado = match if c.estado == EstadoCaravana::EnBatalla { c.antes_de_batalla.unwrap_or(EstadoCaravana::EnTransito) } else { c.estado } {
        EstadoCaravana::Disponible => pb::EstadoCaravana::CaravanaDisponible,
        EstadoCaravana::Preparando => pb::EstadoCaravana::CaravanaPreparando,
        EstadoCaravana::Adjunta => pb::EstadoCaravana::CaravanaAdjunta,
        EstadoCaravana::Aparcada => pb::EstadoCaravana::CaravanaAparcada,
        EstadoCaravana::EnTransito | EstadoCaravana::EnBatalla => pb::EstadoCaravana::CaravanaEnTransito,
        EstadoCaravana::Retornando => pb::EstadoCaravana::CaravanaRetornando,
    };
    let tipo = match c.tipo {
        TipoCaravana::Comercial => pb::TipoCaravana::CaravanaComercial,
        TipoCaravana::Construccion => pb::TipoCaravana::CaravanaDeFundacion,
        TipoCaravana::Acarreo => pb::TipoCaravana::CaravanaDeAcarreo,
        TipoCaravana::Militar | TipoCaravana::Contrabando => pb::TipoCaravana::NoEspecificado,
    };
    let pueblo = (c.tipo == TipoCaravana::Acarreo).then(|| e.asentamientos.values().flat_map(|a| &a.economia.pueblos).find(|p| p.carro == Some(c.id)).map(|p| ids::pueblo(p.id))).flatten();
    pb::Caravana {
        id: ids::caravana(c.id),
        version: n.version_de(Clase::Caravana, c.id),
        tipo: tipo as i32,
        origen_asentamiento_id: ids::asentamiento(u64::from(c.origen)),
        faccion_id: faccion,
        destino_asentamiento_id: c.destino.map(|d| ids::asentamiento(u64::from(d))),
        contenido: Some(carga(&c.contenido)),
        posicion: Some(punto(n.posicion_de_caravana(c.id, t).unwrap_or(c.posicion))),
        plan: c.plan.as_ref().filter(|p| !p.vertices.is_empty() && c.estado != EstadoCaravana::Adjunta).map(plan),
        origen_acuerdo_id: c.origen_acuerdo.map(|(a, _)| ids::acuerdo(a)),
        lado_acuerdo: c.origen_acuerdo.map(|(_, l)| if l == comercio::Lado::A { "A" } else { "B" }.to_owned()),
        estado: estado as i32,
        carros: c
            .carros
            .iter()
            .map(|k| pb::CarroCaravana {
                tipo_carro: if k.tipo == TipoCarro::Basico { "basico" } else { "reforzado" }.into(),
                animal: k.animal.map(|a| match a {
                    Animal::Buey => "buey",
                    Animal::Caballo => "caballo",
                    Animal::Camello => "camello",
                }
                .to_owned()),
            })
            .collect(),
        escolta_ids: c.escolta.iter().map(|s| ids::escuadra(s.id)).collect(),
        reservada_manual: c.reservada_manual,
        prepara_hasta_ms: c.prepara_hasta,
        pueblo_id: pueblo,
        escoltada: c.estado == EstadoCaravana::Adjunta || !c.escolta.is_empty(),
        // Los peajes que le quedan a la ida, con el instante en que su plan cruza cada zona (paridad P7); se cobran solo en tránsito.
        peajes: match (&c.plan, c.estado) {
            (Some(p), EstadoCaravana::EnTransito) => c
                .peajes
                .iter()
                .filter_map(|x| Some(pb::Peaje { asentamiento_id: ids::asentamiento(u64::from(x.ciudad)), en_ms: comercio::viaje::instante_de_progreso(p, x.progreso)? }))
                .collect(),
            _ => vec![],
        },
        ..Default::default()
    }
}

fn seleccion(s: &Seleccion) -> pb::SeleccionDeSalida {
    pb::SeleccionDeSalida { escuadron_ids: s.escuadras.iter().map(|&e| ids::escuadra(e)).collect(), carga: Some(carga(&s.carga)) }
}

/// La convocatoria tal como la ve `quien`: su Líder y la administración ven todas las peticiones y cada solicitante la suya.
pub fn convocatoria(c: &Convocatoria, quien: Espectador, version: u64) -> pb::Convocatoria {
    let ve = |heroe: u64| match quien {
        Espectador::Admin => true,
        Espectador::Heroe(v) => v == Some(c.lider) || v == Some(heroe),
    };
    let (lugar_es_campamento, lugar_id) = match c.lugar {
        Lugar::Asentamiento(a) => (false, ids::asentamiento(u64::from(a))),
        Lugar::Campamento(k) => (true, ids::campamento(k)),
    };
    pb::Convocatoria {
        id: ids::convocatoria(c.id),
        version,
        faccion_id: ids::faccion(c.faccion),
        lugar_es_campamento,
        lugar_id,
        lider_id: ids::heroe(c.lider),
        politica_de_union: super::politica(c.politica) as i32,
        integrantes: c.integrantes.iter().map(|i| pb::IntegranteConvocatoria { heroe_id: ids::heroe(i.heroe), unido_en_ms: i.unido_en, seleccion: Some(seleccion(&i.seleccion)) }).collect(),
        peticiones: c
            .peticiones
            .iter()
            .filter(|p| ve(p.heroe))
            .map(|p| pb::PeticionConvocatoria { heroe_id: ids::heroe(p.heroe), pedido_en_ms: p.pedido_en, expira_en_ms: p.expira_en, seleccion: Some(seleccion(&p.seleccion)) })
            .collect(),
        creada_en_ms: c.creada_en,
    }
}

/// La batalla tal como la ve `quien`. Quien combate en ella la ve entera: el ticket congelado, las incorporaciones, el resultado, su bando y, si ya hay asignación y
/// token para él, cómo conectar con el servidor de batalla (`asignacion_propia`, solo suyo). Quien no combate ve lo que hace falta para decidir si se une: el estado, lo que se bloquea y, si podría unirse, a
/// qué bando (`lado_para_unirse`). La administración la ve entera como quien combate, más la asignación que ganó, y **nunca** un token (ni el propio de nadie ni los de la asignación).
/// `lado_para_unirse` se calcula al proyectar (cuando la batalla cambia o la sesión pide la foto): si el espectador se acerca o se aleja después, no se actualiza hasta el siguiente cambio de la
/// batalla, y la orden `unirse_a_batalla` vuelve a comprobarlo (Batalla_Unity_Ciclo §7.1).
pub fn batalla(n: &Nucleo, b: &Batalla, quien: Espectador) -> pb::Batalla {
    let estado = match b.estado {
        EstadoBatalla::Convocando => pb::EstadoBatalla::BatallaConvocando,
        EstadoBatalla::Asignada => pb::EstadoBatalla::BatallaAsignada,
        EstadoBatalla::EnCurso => pb::EstadoBatalla::BatallaEnCurso,
        EstadoBatalla::Aplicada => pb::EstadoBatalla::BatallaAplicada,
        EstadoBatalla::Cancelada => pb::EstadoBatalla::BatallaCancelada,
        EstadoBatalla::Fallida => pb::EstadoBatalla::BatallaFallida,
    };
    // El héroe del espectador que combate en ella: el suyo o, en una cuenta con columnas de campo, el de campo que combate por la cuenta.
    let e = n.estado();
    let (viewer, admin) = match quien {
        Espectador::Heroe(v) => (v, false),
        Espectador::Admin => (None, true),
    };
    let propio = viewer.and_then(|v| b.heroes_que_combaten().find(|&h| e.militar.heroes.get(&h).is_some_and(|x| audiencia::principal(e, x) == Some(v))));
    let entera = admin || propio.is_some();
    let asignacion_propia = propio.filter(|_| b.activa(n.ahora())).and_then(|h| match n.token_de_jugador(h, b.id) {
        TokenDeJugador::Token { instancia, token, expira_en } => Some(pb::AsignacionPropia { instancia: Some(pb::InstanciaDeBatalla { host: instancia.host, puerto: u32::from(instancia.puerto), protocolo: instancia.protocolo }), token, expira_en_ms: expira_en }),
        _ => None,
    });
    pb::Batalla {
        id: n.uuid_de_batalla(b.id),
        version: n.version_de(Clase::Batalla, b.id),
        estado: estado as i32,
        iniciada_por: if n.heroe(b.iniciada_por).is_some() { ids::heroe(b.iniciada_por) } else { String::new() },
        punto: Some(punto(b.punto)),
        ticket: entera.then(|| batalla::ticket(n, b)),
        incorporaciones: if entera { b.incorporaciones.iter().map(|i| batalla::incorporacion(n, b, i)).collect() } else { Vec::new() },
        bloqueo_ejercito_ids: b.bloqueo.ejercitos.iter().map(|&e| ids::ejercito(e)).collect(),
        bloqueo_caravana_ids: b.bloqueo.caravanas.iter().map(|&c| ids::caravana(c)).collect(),
        bloqueo_asentamiento_id: b.bloqueo.asentamiento.map(|a| ids::asentamiento(u64::from(a))),
        bloqueo_campamento_id: b.bloqueo.campamento.map(ids::campamento),
        expira_en_ms: b.expira_en,
        convocatoria_hasta_ms: b.convocatoria_hasta,
        iniciada_en_ms: b.iniciada_en,
        resultado: b.resultado.as_ref().filter(|_| entera).map(|r| batalla::resultado(n, b, r)),
        aplicada_en_ms: b.aplicada_en,
        cerrada_en_ms: b.cerrada_en,
        lado_propio: propio.and_then(|h| b.lado_de(h)).map(|l| batalla::lado(l) as i32),
        lado_para_unirse: propio.is_none().then(|| viewer.and_then(|v| n.lado_para_unirse(v, b.id))).flatten().map(|l| batalla::lado(l) as i32),
        asignacion_propia,
        asignacion: b.asignacion.as_ref().filter(|_| admin).map(|a| batalla::asignacion_sin_tokens(n, b, a)),
    }
}

// ───────────────────────────── Facción ─────────────────────────────

/// La Facción (5B): nombre, sigilo, Rey, Embajador, ciudadanos en orden de ingreso, solicitudes, experiencia, capital designada, derrota y admisión en los ataques, más su nivel y su reputación
/// (en milésimas de punto: el núcleo la guarda en ppm). Una Facción que solo nombra la administración (una plaza o una relación de una Facción que nadie creó) va sin nada de lo de la entidad.
/// Desde 5C, el nivel se deriva de la experiencia y `capital_id` es la capital efectiva (la designada vigente o la plaza viva más antigua).
pub fn faccion(n: &Nucleo, f: u32) -> pb::Faccion {
    let e = n.estado();
    let rep = e.comercio.reputacion.get(&f).map_or_else(|| base::Ritmo::constante(0, 0), |r| r.ritmo());
    let milesimas = |v: Mil| v / 1_000;
    let base = pb::Faccion {
        id: ids::faccion(f),
        version: n.version_de(Clase::Faccion, u64::from(f)),
        liga: n.liga_de(f).map(|l| pb::LigaDeFaccion { miembros_ids: l.miembros.into_iter().map(ids::faccion).collect(), tiene_vasallaje: l.tiene_vasallaje, gran_rey_id: l.gran_rey.map(ids::faccion) }),
        nivel: n.nivel_de_faccion(f),
        capital_id: n.capital_de(f).map(ids::asentamiento),
        reputacion: Some(ritmo(base::Ritmo { valor_en_t0: milesimas(rep.valor_en_t0), por_minuto: milesimas(rep.por_minuto), maximo: rep.maximo.map(milesimas), minimo: rep.minimo.map(milesimas), ..rep })),
        ..Default::default()
    };
    let Some(x) = e.politica.facciones.get(f) else { return base };
    let heroes = |v: &[u64]| v.iter().map(|&h| ids::heroe(h)).collect();
    let s = &x.sigilo;
    pb::Faccion {
        nombre: x.nombre.clone(),
        sigilo: Some(pb::Sigilo {
            forma_id: s.forma.clone(),
            campo_id: s.campo.clone(),
            emblema_id: s.emblema.clone(),
            color_primario_id: s.color_primario.clone(),
            color_secundario_id: s.color_secundario.clone(),
            color_emblema_id: s.color_emblema.clone(),
            orla_id: s.orla.clone(),
            color_orla_id: s.color_orla.clone(),
        }),
        rey_id: x.rey.map(ids::heroe),
        embajador_id: x.embajador.map(ids::heroe),
        capital_designada_en_ms: x.capital_designada_en,
        experiencia: x.experiencia,
        ciudadanos_ids: heroes(&x.ciudadanos),
        solicitudes_ids: heroes(&x.solicitudes),
        // Ausente: no está derrotada; "": su última plaza colapsó sola.
        derrotada_por: x.derrota.map(|d| match d {
            politica::faccion::Derrota::Por(g) => ids::faccion(g),
            politica::faccion::Derrota::Colapso => String::new(),
        }),
        admite_otras_en_ataques: x.admite_otras_en_ataques,
        misiones: misiones(n, crate::misiones::Dueno::Faccion(f)),
        ..base
    }
}

/// La forma pública de otra Facción (5H, `VISTA`; Proyecciones §6: «facciones: públicas en la ficción»): la Facción sin su experiencia, su capital ni cuándo la designó (dónde está su capital es
/// de la niebla). Regla (decisión del autor del 2026-10-26): de una Facción ajena se ve **solo el nivel**, sin la experiencia exacta ni la capital (el TypeScript la mandaba entera).
pub fn faccion_publica(n: &Nucleo, f: u32) -> pb::Faccion {
    pb::Faccion { experiencia: 0, capital_designada_en_ms: None, capital_id: None, misiones: vec![], ..faccion(n, f) }
}

/// Las tecnologías de la Facción (6G, `Estado::tecnologia`; Proyecciones §6.3: las de un rival no viajan nunca): las aparecidas y las reveladas por su id, las adoptadas con las de arranque
/// primero (como desde 6B); de cada revelada, su descubridora y qué condiciones de su hito cumple ya (en el orden de `reveladas`), y las épicas que podría empezar ahora un Aeda suyo.
pub fn tecnologias(n: &Nucleo, f: u32) -> pb::TecnologiasFaccion {
    let e = n.estado();
    let r = &e.reglas.economia.tecnologia;
    let nombres = |c: tecnologia::Conjunto| {
        let mut v: Vec<String> = c.iter().map(|t| r.def(t).id.clone()).collect();
        v.sort();
        v
    };
    let propias = e.tecnologia.de(r, f);
    let mut adoptadas = nombres(r.de_arranque);
    adoptadas.extend(nombres(tecnologia::Conjunto(propias.adoptadas.0 & !r.de_arranque.0)));
    let mut detalle: Vec<pb::TecnologiaRevelada> = n
        .reveladas_con_detalle(f)
        .into_iter()
        .map(|(x, descubridora, cumplidas)| pb::TecnologiaRevelada { tecnologia_id: r.def(x).id.clone(), descubridora_faccion_id: descubridora.map(ids::faccion).unwrap_or_default(), cumplidas })
        .collect();
    detalle.sort_by(|a, b| a.tecnologia_id.cmp(&b.tecnologia_id));
    pb::TecnologiasFaccion {
        faccion_id: ids::faccion(f),
        version: n.version_de(Clase::Faccion, u64::from(f)),
        aparecidas: nombres(propias.aparecidas),
        adoptadas,
        reveladas: nombres(propias.reveladas),
        detalle_reveladas: detalle,
        epicas_posibles: nombres(n.epicas_posibles(f)),
    }
}

/// El estado de tecnología del mundo (6G; la entidad `ESTADO_TECNOLOGIA`, id `tecnologia`). Para un jugador, su forma pública (decisión Q6 del autor; Proyecciones §6.3): la Era, desde cuándo, los
/// logros cumplidos como hechos del mundo (`contador` llegó a `umbral`, por orden, sin decir qué tecnología abren), lo cantado y, de las descubridoras, solo las cantadas. La administración lo ve
/// entero: los contadores en curso y los logros y las descubridoras por tecnología.
pub fn estado_tecnologia(n: &Nucleo, quien: Espectador) -> pb::EstadoTecnologia {
    let e = n.estado();
    let (r, t) = (&e.reglas.economia.tecnologia, &e.tecnologia);
    let admin = quien == Espectador::Admin;
    let id = |x: usize| r.def(x as tecnologia::TecId).id.clone();
    let mut logros_cumplidos: Vec<pb::LogroCumplido> = t
        .logros
        .iter()
        .enumerate()
        .filter_map(|(x, en)| Some((r.def(x as tecnologia::TecId).logro?, (*en)?)))
        .map(|(l, en)| pb::LogroCumplido { contador: l.contador.nombre().to_owned(), umbral: l.umbral as i64, en_ms: en })
        .collect();
    logros_cumplidos.sort_by(|a, b| (a.en_ms, &a.contador, a.umbral).cmp(&(b.en_ms, &b.contador, b.umbral)));
    let primeros = t
        .primeros
        .iter()
        .enumerate()
        .filter(|&(x, _)| admin || t.cantadas.contiene(x as tecnologia::TecId))
        .filter_map(|(x, p)| p.map(|p| (id(x), pb::PrimeroEnLograr { faccion_id: ids::faccion(p.faccion), en_ms: p.en })))
        .collect();
    let mut cantadas: Vec<String> = t.cantadas.iter().map(|x| id(x as usize)).collect();
    cantadas.sort();
    pb::EstadoTecnologia {
        era: r.eras[t.era as usize].id.clone(),
        era_desde_ms: t.era_desde,
        contadores: if admin { tecnologia::contador::Contador::TODOS.into_iter().map(|c| (c.nombre().to_owned(), t.contador(c) as i64)).collect() } else { Default::default() },
        logros: if admin { t.logros.iter().enumerate().filter_map(|(x, en)| Some((id(x), (*en)?))).collect() } else { Default::default() },
        primeros,
        cantadas,
        logros_cumplidos,
        // Los dos bordes de la Era vigente (decisión del autor sobre 6B): no dicen si sus logros están cumplidos, así que son públicos.
        era_minimo_hasta_ms: t.fin_del_plazo(r).map(|_| t.era_desde + r.eras[t.era as usize].minimo),
        era_plazo_hasta_ms: t.fin_del_plazo(r),
    }
}

/// Un Aeda residente (6G) con su épica: el capítulo en curso, los hechos que lleva y los que pide, cuántos capítulos tiene y cuándo contó el último hecho (el enfriamiento). La memoria de hechos ya
/// contados (`claves`), solo para la administración.
pub fn aeda_residente(n: &Nucleo, a: &tecnologia::residentes::AedaResidente, quien: Espectador) -> pb::AedaResidente {
    let r = &n.estado().reglas.economia.tecnologia;
    let epica = a.epica.as_ref().map(|x| {
        let capitulos = r.def(x.tec).epica.as_ref().map_or(&[][..], |d| &d.capitulos[..]);
        pb::EpicaEnCurso {
            tecnologia_id: r.def(x.tec).id.clone(),
            capitulo: u32::from(x.capitulo),
            hechos: x.hechos,
            ultimo_hecho_en_ms: x.ultimo_hecho_en,
            claves: if quien == Espectador::Admin { x.claves.iter().map(|c| clave_de_epica(n, r, c)).collect() } else { Vec::new() },
            capitulos: capitulos.len() as u32,
            hechos_necesarios: capitulos.get(usize::from(x.capitulo)).map_or(0, |c| c.cantidad),
        }
    });
    pb::AedaResidente {
        id: ids::aeda_residente(a.id),
        version: n.version_de(Clase::Residente, u64::from(a.id)),
        nombre: a.nombre().into(),
        asentamiento_id: ids::asentamiento(u64::from(a.plaza)),
        faccion_id: ids::faccion(a.faccion),
        llegada_en_ms: a.llegada_en,
        epica,
    }
}

/// La clave de un hecho ya contado por una épica, como la del TypeScript con los ids del contrato (`defensa:as-3:<instante>`, `obra:ed-3-2:1`, `batalla:<battleId>`...).
fn clave_de_epica(n: &Nucleo, r: &tecnologia::Reglas, c: &tecnologia::epicas::Clave) -> String {
    use tecnologia::epicas::Clave as K;
    match *c {
        K::Defensa { plaza, en } => format!("defensa:{}:{en}", ids::asentamiento(u64::from(plaza))),
        K::Conquista { plaza, en } => format!("conquista:{}:{en}", ids::asentamiento(u64::from(plaza))),
        K::Batalla { a, b, en } => format!("batalla:{}:{}:{en}", ids::ejercito(a), ids::ejercito(b)),
        K::Unity { tipo, batalla } => format!("{}:{}", tipo.nombre(), n.uuid_de_batalla(batalla)),
        K::Obra { plaza, edificio, nivel } => format!("obra:{}:{nivel}", ids::edificio(u64::from(plaza), edificio)),
        K::Ascenso { plaza, nivel } => format!("ascenso:{}:{nivel}", ids::asentamiento(u64::from(plaza))),
        K::Adopcion { faccion, tec } => format!("adopcion:{}:{}", ids::faccion(faccion), r.def(tec).id),
        K::Caravana(c) => format!("caravana:{}", ids::caravana(c)),
    }
}

/// Un Aeda itinerante entero (6G, `ADMIN`): dónde se paró o de dónde salió, la plaza en que está y hasta cuándo, adónde va con su plan entero y las plazas que visitó hace poco. La `VISTA` lo
/// redacta (`vista::Mirada::contenido`). Los Aedas no son una clase del registro de cambios (moverse no sube la versión): `version` va a 0 y `ADMIN` los compara en cada reparto.
pub fn aeda_itinerante(a: &tecnologia::itinerantes::AedaItinerante) -> pb::AedaItinerante {
    pb::AedaItinerante {
        id: ids::aeda(a.id),
        version: 0,
        posicion: Some(punto(a.posicion)),
        en_asentamiento_id: a.en.map(|p| ids::asentamiento(u64::from(p))),
        hasta_ms: a.hasta,
        destino_id: a.destino.map(|p| ids::asentamiento(u64::from(p))),
        plan: a.plan.as_ref().map(plan),
        recientes: a.recientes.iter().map(|&p| ids::asentamiento(u64::from(p))).collect(),
    }
}

/// Un alijo del artefacto (6G, los que el héroe tiene a la vista en `HEROE`): su id, dónde está y su oro (milésimas, el del artefacto).
pub fn alijo(n: &Nucleo, i: u32) -> Option<pb::Alijo> {
    n.alijo(i).map(|a| pb::Alijo { id: a.id.clone(), posicion: Some(punto(a.posicion)), oro: a.oro })
}

/// Una relación entre dos Facciones (5D): con su dirección (en un vasallaje `a` es la señora; en una guerra, quien la declaró), su tributo, cuándo nació, quién la rompió y quién ofreció la paz.
/// Viaja mientras está activa: al romperse, sale del almacén del cliente (`QUITA`) y el hecho dice cómo acabó. Es pública: la ven también las demás Facciones (`VISTA`).
pub fn relacion(n: &Nucleo, r: &RelacionPolitica) -> pb::RelacionPolitica {
    pb::RelacionPolitica {
        id: ids::relacion(r.id),
        version: n.version_de(Clase::Relacion, r.id),
        tipo: match r.tipo {
            TipoRelacion::Alianza => pb::TipoRelacion::RelacionAlianza,
            TipoRelacion::Vasallaje => pb::TipoRelacion::RelacionVasallaje,
            TipoRelacion::Guerra => pb::TipoRelacion::RelacionGuerra,
        } as i32,
        faccion_a_id: ids::faccion(r.a),
        faccion_b_id: ids::faccion(r.b),
        tributo: r.tributo.map(|t| pb::Tributo { recurso: t.recurso.nombre().to_owned(), cantidad_por_minuto: t.cantidad_por_minuto }),
        creado_en_ms: r.creada_en,
        estado: if r.activa() { pb::EstadoRelacion::RelacionActiva } else { pb::EstadoRelacion::RelacionRota } as i32,
        rota_por: r.rota_por.map(ids::faccion),
        rota_en_ms: r.rota_en,
        paz_propuesta_por: r.paz_propuesta_por.map(ids::faccion),
    }
}

/// Una propuesta entre dos Facciones (5H, Diplomacia §2.4): de relación (alianza o vasallaje, con su tributo), de anexión o de fusión (con el nombre y el Rey de la Facción nueva). `propuesta_por`
/// es el héroe que la hizo (5A-7). Viaja mientras está en el almacén, también caducada: el cliente la descarta por `expira_en_ms` (caducar no es un hecho, decisión 4) y sale con `QUITA` al
/// contestarla, retirarla o podarla.
pub fn propuesta(n: &Nucleo, p: &politica::Propuesta) -> pb::Propuesta {
    use pb::propuesta::Clase as C;
    use politica::propuestas::{Clase as Cp, Pacto};
    let clase = match &p.clase {
        Cp::Relacion { pacto, a, b, tributo } => C::Relacion(pb::PropuestaRelacion {
            tipo: if *pacto == Pacto::Vasallaje { pb::TipoRelacion::RelacionVasallaje } else { pb::TipoRelacion::RelacionAlianza } as i32,
            faccion_a_id: ids::faccion(*a),
            faccion_b_id: ids::faccion(*b),
            tributo: tributo.map(|t| pb::Tributo { recurso: t.recurso.nombre().to_owned(), cantidad_por_minuto: t.cantidad_por_minuto }),
        }),
        Cp::Anexion { absorbente, absorbida } => C::Anexion(pb::PropuestaAnexion { absorbente_id: ids::faccion(*absorbente), absorbida_id: ids::faccion(*absorbida) }),
        Cp::Fusion { a, b, nombre, rey } => C::Fusion(pb::PropuestaFusion { faccion_a_id: ids::faccion(*a), faccion_b_id: ids::faccion(*b), nuevo_nombre: nombre.clone(), nuevo_rey_id: ids::heroe(*rey) }),
    };
    pb::Propuesta {
        id: ids::propuesta(p.id),
        version: n.version_de(Clase::Propuesta, p.id),
        propuesta_por: ids::heroe(p.propuesta_por),
        creada_en_ms: p.creada_en,
        expira_en_ms: p.expira_en,
        clase: Some(clase),
    }
}

// ───────────────────────────── Pueblos, avanzadas y sus obras ─────────────────────────────

/// El pueblo extractor activo. Mientras es una obra (pendiente de materiales o levantándose) el cliente lo ve como [`obra_de_pueblo`].
/// El paso de minuto refresca su almacén local sin avanzar la versión del mundo: `version_interior` es la secuencia de su plaza madre y dice de qué minuto es (en `ASENTAMIENTO` llega con cada
/// cambio del interior; en `FACCION` solo con el último cambio discreto del pueblo: DUDA, T3c).
pub fn pueblo(n: &Nucleo, madre: u64, p: &Pueblo) -> pb::Pueblo {
    let e = n.estado();
    let recurso = p.recurso(&e.reglas.economia.datos);
    let guarnicion = e.combate.guarniciones.get(&p.id);
    pb::Pueblo {
        id: ids::pueblo(p.id),
        version: n.version_de(Clase::Pueblo, p.id),
        asentamiento_madre_id: ids::asentamiento(madre),
        nodo_id: p.nodo().map(ids::nodo).unwrap_or_default(),
        tipo_extractor: p.nombre_de_tipo().into(),
        posicion: n.posicion_de_pueblo(p.id).map(punto),
        almacen_local: Some(existencia(&p.almacen, recurso, 0)),
        carro_acarreo_id: p.carro.map(ids::caravana),
        guarnicion_escuadron_id: guarnicion.map(|g| ids::escuadra(g.escuadra.id)),
        guarnicion_repone_en_ms: guarnicion.and_then(|g| g.repone_en),
        batalla_id: batalla_sobre(n, Ubicacion::Pueblo(p.id)),
        version_interior: n.version_interior(madre).unwrap_or(0),
        nivel_aldea: p.es_aldea().then_some(p.nivel),
        fertilidad: match p.sitio {
            asentamiento::pueblos::Sitio::Punto { fertilidad, .. } => Some(fertilidad / 1000),
            asentamiento::pueblos::Sitio::Nodo(_) => None,
        },
        almacen_por_recurso: if p.es_aldea() {
            p.recursos(&e.reglas.economia.datos).into_iter().map(|r| (r.nombre().to_owned(), existencia(&p.almacen, r, 0))).collect()
        } else {
            Default::default()
        },
        rebano_especie: p.rebano.as_ref().map(|r| r.especie.clone()),
        rebano_cabezas: p.rebano.as_ref().map(|r| r.cabezas),
    }
}

fn requerido(costo: &[(Recurso, Mil)]) -> pb::Cantidades {
    cantidades(costo.iter().copied())
}

/// La obra de un pueblo que aún no está activo, o la subida de nivel de una aldea agrícola activa (`None`: activo y sin subida). DUDA: una vez que arranca
/// (`EnObra`) el núcleo ya no guarda lo pedido y lo entregado: salen vacíos y queda `termina_en_ms`.
pub fn obra_de_pueblo(n: &Nucleo, madre: u64, p: &Pueblo) -> Option<pb::ObraDeMapa> {
    let (estado, tipo, destino) = match (&p.estado, &p.mejora) {
        (EstadoPueblo::Activo, Some(m)) => (m, pb::TipoObra::ObraMejoraAldea, Some(ids::pueblo(p.id))),
        (EstadoPueblo::Activo, None) => return None,
        (e, _) => (e, if p.es_aldea() { pb::TipoObra::ObraAldea } else { pb::TipoObra::ObraPueblo }, None),
    };
    let (req, ent, termina) = match estado {
        EstadoPueblo::Pendiente { requerido: r, entregado } => (r.clone(), entregado.clone(), None),
        EstadoPueblo::EnObra { completa_en } => (Vec::new(), Vec::new(), Some(*completa_en)),
        EstadoPueblo::Activo => (Vec::new(), Vec::new(), None),
    };
    Some(pb::ObraDeMapa {
        id: ids::obra(p.id),
        version: n.version_de(Clase::Pueblo, p.id),
        asentamiento_madre_id: ids::asentamiento(madre),
        tipo: tipo as i32,
        posicion: n.posicion_de_pueblo(p.id).map(punto),
        nodo_id: p.nodo().map(ids::nodo),
        destino_id: destino,
        requerido: Some(requerido(&req)),
        entregado: Some(requerido(&ent)),
        termina_en_ms: termina,
        ..Default::default()
    })
}

/// La avanzada que ya es un puesto (operativa o en ruinas). La primera obra, la mejora y la reparación van aparte, en [`obra_de_avanzada`].
/// El almacén va por recurso en `almacen_por_recurso` (`almacen`, de una sola `Existencia`, no se usa); `batalla_id` es el asedio visible.
pub fn avanzada(n: &Nucleo, madre: Option<u64>, v: &Avanzada) -> pb::Avanzada {
    let e = n.estado();
    let g = e.combate.avanzadas.get(&v.id);
    let elegida = |escalon: u8| g.and_then(|g| g.elegidas.get(&escalon)).map(|t| t.nombre().to_owned());
    pb::Avanzada {
        id: ids::avanzada(v.id),
        version: n.version_de(Clase::Avanzada, v.id),
        asentamiento_madre_id: madre.map(ids::asentamiento).unwrap_or_default(),
        etapa: v.etapa.indice() as u32 + 1,
        posicion: Some(punto(v.posicion)),
        estado: if v.en_ruinas() { pb::EstadoAvanzada::AvanzadaEnRuinas } else { pb::EstadoAvanzada::AvanzadaActiva } as i32,
        radio_vision: vision::radios::AVANZADA,
        guarnicion_serie_ids: g.into_iter().flat_map(|g| &g.escuadras).map(|s| ids::escuadra(s.id)).collect(),
        tropa_elegida_escalon2: elegida(2),
        invulnerable_hasta_ms: match v.estado {
            EstadoAvanzada::EnRuinas { invulnerable_hasta } => invulnerable_hasta,
            _ => None,
        },
        tropa_elegida_por_escalon: g.into_iter().flat_map(|g| &g.elegidas).map(|(&k, t)| (u32::from(k), t.nombre().to_owned())).collect::<BTreeMap<_, _>>().into_iter().collect(),
        guarnicion_jugadores_ids: n.guarnicion_de_jugadores_de_avanzada(v.id).iter().map(|s| ids::escuadra(s.id)).collect(),
        batalla_id: batalla_sobre(n, Ubicacion::Avanzada(v.id)),
        // Lo que hay de cada recurso, con su tope de la etapa (paridad, 2026-10-27); la atalaya, sin almacén, no lleva ninguno.
        almacen_por_recurso: asentamiento::recurso::TODOS
            .iter()
            .filter(|&&r| v.almacen.capacidad(r) > 0)
            .map(|&r| (r.nombre().to_owned(), existencia(&v.almacen, r, 0)))
            .collect(),
        ..Default::default()
    }
}

/// La obra de una avanzada: levantar la atalaya, pasar de etapa o reparar unas ruinas.
pub fn obra_de_avanzada(n: &Nucleo, madre: Option<u64>, v: &Avanzada) -> Option<pb::ObraDeMapa> {
    let o = v.obra.as_ref()?;
    let tipo = match o.tipo {
        TipoObra::Construir => pb::TipoObra::ObraAvanzada,
        TipoObra::Mejorar => pb::TipoObra::ObraMejoraAvanzada,
        TipoObra::Reparar => pb::TipoObra::ObraReparacion,
    };
    Some(pb::ObraDeMapa {
        id: ids::obra(v.id),
        version: n.version_de(Clase::Avanzada, v.id),
        asentamiento_madre_id: madre.map(ids::asentamiento).unwrap_or_default(),
        tipo: tipo as i32,
        posicion: Some(punto(v.posicion)),
        destino_id: (o.tipo != TipoObra::Construir).then(|| ids::avanzada(v.id)),
        requerido: Some(requerido(&o.requerido)),
        entregado: Some(requerido(&o.entregado)),
        termina_en_ms: o.completa_en,
        ..Default::default()
    })
}

// ───────────────────────────── Bandidos ─────────────────────────────

/// Un campamento de bandidos tal cual: dónde, su nivel y su poder, su bosque y de dónde salió (su plaza, el anillo de un campamento o una veta).
pub fn campamento_bandidos(n: &Nucleo, b: &crate::estado_combate::CampamentoBandidos) -> pb::CampamentoBandidos {
    use crate::estado_combate::OrigenBandidos;
    let r = &n.estado().reglas.economia.combate;
    pb::CampamentoBandidos {
        id: ids::bandidos(b.id),
        version: n.version_de(Clase::Bandidos, b.id),
        posicion: Some(punto(b.posicion)),
        bosque_id: b.bosque.map(|x| format!("bosque-{x}")).unwrap_or_default(),
        asentamiento_id: if let OrigenBandidos::Plaza(a) = b.origen { Some(ids::asentamiento(a)) } else { None },
        campamento_mercenarios_id: if let OrigenBandidos::Anillo(c) = b.origen { Some(ids::campamento(c)) } else { None },
        veta_id: if let OrigenBandidos::Veta(v) = b.origen { Some(format!("veta-{v}")) } else { None },
        nivel: u32::from(b.nivel),
        poder: (combate::bandidos::nivel_de(r, b.nivel).poder / 1000).max(0) as u32,
    }
}

// ───────────────────────────── Comercio ─────────────────────────────

fn lineas(ls: &[Linea]) -> Vec<pb::LineaTrueque> {
    ls.iter().map(|l| pb::LineaTrueque { recurso: l.recurso.nombre().into(), cantidad_total: l.cantidad_total, cantidad_entregada: l.cantidad_entregada }).collect()
}

pub fn trueque(n: &Nucleo, a: &AcuerdoTrueque) -> pb::AcuerdoTrueque {
    let estado = match a.estado {
        EstadoTrueque::Propuesto => pb::EstadoTrueque::TruequePropuesto,
        EstadoTrueque::Activo => pb::EstadoTrueque::TruequeActivo,
        EstadoTrueque::Rechazado => pb::EstadoTrueque::TruequeRechazado,
        EstadoTrueque::Cumplido => pb::EstadoTrueque::TruequeCumplido,
        EstadoTrueque::Expirado => pb::EstadoTrueque::TruequeExpirado,
    };
    pb::AcuerdoTrueque {
        id: ids::acuerdo(a.id),
        version: n.version_de(Clase::Acuerdo, a.id),
        asentamiento_a_id: ids::asentamiento(u64::from(a.a)),
        asentamiento_b_id: ids::asentamiento(u64::from(a.b)),
        lineas_a: lineas(&a.lineas_a),
        lineas_b: lineas(&a.lineas_b),
        creado_en_ms: a.creado_en,
        expira_en_ms: a.expira_en,
        estado: estado as i32,
    }
}

pub fn orden_de_mercado(n: &Nucleo, o: &OrdenMercado) -> pb::OrdenMercado {
    let estado = match o.estado {
        EstadoOrden::Activa => pb::EstadoOrden::OrdenActiva,
        EstadoOrden::Cumplida => pb::EstadoOrden::OrdenCumplida,
        EstadoOrden::Expirada => pb::EstadoOrden::OrdenExpirada,
    };
    pb::OrdenMercado {
        id: ids::orden_de_mercado(o.id),
        version: n.version_de(Clase::OrdenMercado, o.id),
        asentamiento_id: ids::asentamiento(u64::from(o.asentamiento)),
        es_compra: o.tipo == TipoOrden::Compra,
        recurso: o.recurso.nombre().into(),
        cantidad: o.cantidad,
        cantidad_cumplida: o.cantidad_cumplida,
        precio_unitario: o.precio_unitario,
        creado_en_ms: o.creado_en,
        expira_en_ms: o.expira_en,
        estado: estado as i32,
    }
}

/// Un título de prestigio (5F, Diplomacia_Facciones §7): público, con su poseedor y su valor (plazas, oro en unidades, soldados; 1 el Gran Rey). Su id es su clave estable (`faccionMasGrande`...).
pub fn titulo(n: &Nucleo, x: &politica::titulos::Titulo) -> pb::Titulo {
    pb::Titulo {
        titulo_id: x.id.clave().into(),
        version: n.version_de(Clase::Titulo, x.id.indice()),
        nombre: x.id.nombre().into(),
        poseedor_id: ids::faccion(x.poseedor),
        valor_metrica: x.valor,
    }
}

// ───────────────────────────── Por clase del registro de cambios ─────────────────────────────

/// Lo que una entidad del registro de cambios es en el contrato: las entidades que existen ahora y todos los ids en que puede aparecer (lo que no existe, si se quita, sale con `QUITA`).
/// Una clase puede ser dos entidades: un pueblo es `Pueblo` o `ObraDeMapa` según haya terminado; una avanzada es `Avanzada` y, mientras la obra dure, `ObraDeMapa`.
pub struct Proyeccion {
    pub existen: Vec<Proyectada>,
    pub posibles: Vec<(pb::TipoEntidad, String)>,
}

/// Proyecta la entidad `(clase, id)` para `quien` (el héroe de la sesión, que decide qué peticiones y qué bando ve, o la administración). `Interior` no es una entidad: es la plaza (`interior`).
pub fn proyectar(n: &Nucleo, quien: Espectador, clase: Clase, id: u64) -> Proyeccion {
    use pb::TipoEntidad as T;
    let e = n.estado();
    let un = |tipo: T, id: String, cuerpo: Option<Cuerpo>| Proyeccion { posibles: vec![(tipo, id.clone())], existen: cuerpo.into_iter().map(|c| (tipo, id.clone(), c)).collect() };
    match clase {
        Clase::Ejercito => un(T::Ejercito, ids::ejercito(id), e.ejercitos.get(&id).map(|x| Cuerpo::Ejercito(ejercito(n, x)))),
        // Desde fuera, de una plaza propia viajan además sus cargos (Doc 1.10.1; 5E) y sus fundadores (5H).
        Clase::Asentamiento => un(
            T::Asentamiento,
            ids::asentamiento(id),
            e.asentamientos.get(&id).map(|a| Cuerpo::Asentamiento(pb::Asentamiento { cargos: Some(super::interior::cargos(&a.cargos)), fundadores_ids: a.fundadores.iter().map(|&h| ids::heroe(h)).collect(), ..super::ficha_de_asentamiento(a, &n.poligono_zona(a.id), batalla_sobre(n, Ubicacion::Asentamiento(a.id as u32))) })),
        ),
        Clase::Heroe => {
            // El héroe y sus informes de inspección vigentes (los vencidos los quita la sesión al reconciliar).
            let mut p = un(T::Heroe, ids::heroe(id), n.heroe(id).map(|h| Cuerpo::Heroe(heroe(n, h))));
            let informes = n.informes_de_inspeccion(id).into_iter().map(|i| informe_de_inspeccion(n, id, i));
            p.existen.extend(informes.map(|i| (T::InformeInspeccion, i.id.clone(), Cuerpo::InformeInspeccion(i))));
            p
        }
        Clase::Escuadra => un(T::Escuadron, ids::escuadra(id), n.escuadra(id).map(|(_, s)| Cuerpo::Escuadron(escuadron(n, s)))),
        Clase::Caravana => {
            let cuerpo = e.comercio.caravana(id).map(|c| caravana(n, c)).or_else(|| e.fundacion.caravanas.get(&id).map(|c| caravana_de_fundacion(n, c)));
            un(T::Caravana, ids::caravana(id), cuerpo.map(Cuerpo::Caravana))
        }
        Clase::Convocatoria => un(T::Convocatoria, ids::convocatoria(id), e.militar.convocatorias.get(&id).map(|c| Cuerpo::Convocatoria(convocatoria(c, quien, n.version_de(Clase::Convocatoria, id))))),
        Clase::Acuerdo => un(T::AcuerdoTrueque, ids::acuerdo(id), e.comercio.acuerdo(id).map(|a| Cuerpo::AcuerdoTrueque(trueque(n, a)))),
        Clase::OrdenMercado => un(
            T::OrdenMercado,
            ids::orden_de_mercado(id),
            e.comercio.libro.ordenes.iter().chain(&e.comercio.libro.historial).find(|o| o.id == id).map(|o| Cuerpo::OrdenMercado(orden_de_mercado(n, o))),
        ),
        Clase::Batalla => un(T::Batalla, n.uuid_de_batalla(id), n.batalla(id).map(|b| Cuerpo::Batalla(batalla(n, b, quien)))),
        Clase::Relacion => un(T::RelacionPolitica, ids::relacion(id), e.politica.relaciones.get(id).filter(|r| r.activa()).map(|r| Cuerpo::Relacion(relacion(n, r)))),
        Clase::Propuesta => un(T::Propuesta, ids::propuesta(id), e.politica.propuestas.get(id).map(|p| Cuerpo::Propuesta(propuesta(n, p)))),
        Clase::Faccion => {
            let f = id as u32;
            // La que desapareció en una anexión o una fusión (5F) ya no existe: sale con `QUITA`.
            let existen = if audiencia::existe_faccion(e, f) {
                vec![(T::Faccion, ids::faccion(f), Cuerpo::Faccion(faccion(n, f))), (T::TecnologiaFaccion, ids::faccion(f), Cuerpo::Tecnologias(tecnologias(n, f)))]
            } else {
                Vec::new()
            };
            Proyeccion { posibles: vec![(T::Faccion, ids::faccion(f)), (T::TecnologiaFaccion, ids::faccion(f))], existen }
        }
        Clase::Residente => un(T::Aeda, ids::aeda_residente(id as u32), e.residentes.get(id as u32).map(|a| Cuerpo::AedaResidente(aeda_residente(n, a, quien)))),
        Clase::Tecnologia => un(T::EstadoTecnologia, ID_ESTADO_TECNOLOGIA.into(), Some(Cuerpo::EstadoTecnologia(estado_tecnologia(n, quien)))),
        Clase::Titulo => {
            let t = politica::titulos::TituloId::de_indice(id);
            let x = t.and_then(|t| e.politica.titulos.iter().find(|x| x.id == t));
            let clave = t.map_or_else(|| format!("titulo-{id}"), |t| t.clave().to_string());
            un(T::Titulo, clave, x.map(|x| Cuerpo::Titulo(titulo(n, x))))
        }
        Clase::Pueblo => {
            let (pueblo_id, obra_id) = (ids::pueblo(id), ids::obra(id));
            let mut existen = Vec::new();
            if let Some((madre, p)) = n.pueblo(id) {
                if p.activo() {
                    existen.push((T::Pueblo, pueblo_id.clone(), Cuerpo::Pueblo(pueblo(n, madre, p))));
                }
                // La obra del pueblo mientras no está activo; la de la subida de nivel de una aldea activa, junto al pueblo.
                existen.extend(obra_de_pueblo(n, madre, p).map(|o| (T::ObraDeMapa, obra_id.clone(), Cuerpo::Obra(o))));
            }
            Proyeccion { posibles: vec![(T::Pueblo, pueblo_id), (T::ObraDeMapa, obra_id)], existen }
        }
        Clase::Avanzada => {
            let (av_id, obra_id) = (ids::avanzada(id), ids::obra(id));
            let mut existen = Vec::new();
            if let Some((madre, v)) = n.avanzada(id) {
                if v.estado != EstadoAvanzada::EnConstruccion {
                    existen.push((T::Avanzada, av_id.clone(), Cuerpo::Avanzada(avanzada(n, madre, v))));
                }
                existen.extend(obra_de_avanzada(n, madre, v).map(|o| (T::ObraDeMapa, obra_id.clone(), Cuerpo::Obra(o))));
            }
            Proyeccion { posibles: vec![(T::Avanzada, av_id), (T::ObraDeMapa, obra_id)], existen }
        }
        Clase::Campamento => un(T::CampamentoMercenarios, ids::campamento(id as u32), super::interior::campamento_por_dentro(n, id as u32, quien).map(Cuerpo::CampamentoMercenarios)),
        Clase::Bandidos => un(T::CampamentoBandidos, ids::bandidos(id), e.combate.bandidos.get(&id).map(|b| Cuerpo::CampamentoBandidos(campamento_bandidos(n, b)))),
        Clase::Interior => Proyeccion { existen: Vec::new(), posibles: Vec::new() },
    }
}

/// Todo lo que hay en el mundo menos las plazas (que `ADMIN` lleva con su interior, `interior::asentamiento`), por clase e id: lo que [`proyectar`] da a la administración. Los Aedas itinerantes
/// no son una clase: la foto de `ADMIN` los añade aparte (`sesiones::ambitos::foto`).
///
/// Lo que no tiene clase en el registro de cambios (la memoria de la niebla de cada Facción, sus Miradas, las zonas fundidas y la red de caminos, desde la paridad del 2026-10-27) lo añade
/// la foto de `ADMIN` aparte, como los Aedas (`vista::huellas_de_admin`). DUDA: no están los precios de referencia ni el historial de cada héroe, que el `admin/partida` del TypeScript sí llevaba
/// (fila «Contrato (4H, T3e)»). Los títulos van desde 5F; los Aedas, sus épicas, el estado de tecnología del mundo y los alijos (en el artefacto), desde 6G.
pub fn todo_menos_plazas(n: &Nucleo) -> Vec<(Clase, u64)> {
    let e = n.estado();
    let mut v: Vec<(Clase, u64)> = audiencia::CLASES.into_iter().filter(|c| !matches!(c, Clase::Asentamiento | Clase::Escuadra)).flat_map(|c| audiencia::todas(e, c).into_iter().map(move |id| (c, id))).collect();
    v.extend(e.militar.heroes.values().flat_map(|h| &h.heroe.escuadras).map(|s| (Clase::Escuadra, s.id)));
    v.extend(n.avanzadas_huerfanas().map(|x| (Clase::Avanzada, x.id)));
    v.extend(n.puertas_de_campamentos().map(|(c, _)| (Clase::Campamento, u64::from(c))));
    v.extend(e.combate.bandidos.keys().map(|&b| (Clase::Bandidos, b)));
    v.extend(e.politica.titulos.iter().map(|x| (Clase::Titulo, x.id.indice())));
    v.push((Clase::Tecnologia, 0));
    v
}

/// Los cambios de `(clase, id)` para `quien`: `ACTUALIZA` de lo que existe si `pertenece`, y `QUITA` de lo demás (todo, si la entidad ya no es del ámbito o ya no existe).
pub fn cambios(n: &Nucleo, quien: Espectador, clase: Clase, id: u64, pertenece: bool) -> Vec<pb::Cambio> {
    let Proyeccion { existen, posibles } = proyectar(n, quien, clase, id);
    let existen = if pertenece { existen } else { Vec::new() };
    let actualiza = existen.iter().map(|(tipo, id, cuerpo)| pb::Cambio {
        tipo: pb::TipoCambio::CambioActualiza as i32,
        entidad: *tipo as i32,
        id: id.clone(),
        datos: Some(pb::Entidad { cuerpo: Some(cuerpo.clone()) }),
        reemplaza: true,
        ..Default::default()
    });
    let quita = posibles.into_iter().filter(|(t, i)| !existen.iter().any(|(t2, i2, _)| t2 == t && i2 == i)).map(|(t, i)| super::cambio_quita(t, i));
    actualiza.chain(quita).collect()
}
