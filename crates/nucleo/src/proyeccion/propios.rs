//! Las entidades de los ámbitos propios, `HEROE` y `FACCION` (contrato 05 §3, 4H T3b): el héroe con sus escuadras y loadouts, las columnas en que participa, las caravanas que le
//! tocan, las convocatorias y las batallas en que combate; y de la Facción, sus plazas en resumen, su flota, sus relaciones y tecnologías, sus pueblos y avanzadas (con sus obras),
//! sus trueques y sus órdenes de mercado. Cada función va del estado del núcleo a la entidad del contrato; [`proyectar`] reúne las de una clase del registro de cambios.
//!
//! Qué ámbito recibe qué lo decide [`super::audiencia`]; aquí solo se dice **qué es** cada cosa en el contrato. Las entidades viajan completas (`reemplaza = true`, decisión 2).
//!
//! DUDA: el contrato trae más de lo que el núcleo guarda; hasta que existan sus sistemas, esos campos van a su valor por defecto: nombre, clase, género y avatar del héroe
//! (decisión 4: «Héroe N», clase `default`), nombre y sigilo de la Facción (sistema 5), inventario y equipo (Conquest, CQ-004), peajes y titular de una caravana, y el almacén de una avanzada
//! (el contrato trae una sola `Existencia` y el núcleo un almacén de 29 recursos).

use super::{audiencia, ids, plan, punto, ritmo};
use crate::estado::{Clase, Ejercito};
use crate::estado_militar::{HeroeNucleo, Relacion};
use crate::nucleo::Nucleo;
use asentamiento::avanzadas::{Avanzada, Estado as EstadoAvanzada, TipoObra};
use asentamiento::pueblos::{EstadoPueblo, Pueblo};
use asentamiento::{Almacen, Recurso};
use base::Mil;
use combate::ciclo::{Batalla, EstadoBatalla};
use combate::resolucion::Lado;
use comercio::caravana::{Animal, Caravana, EstadoCaravana, TipoCaravana, TipoCarro};
use comercio::orden_mercado::{EstadoOrden, OrdenMercado, TipoOrden};
use comercio::trueque::{AcuerdoTrueque, EstadoTrueque, Linea};
use contrato::v1 as pb;
use militar::convocatoria::{Convocatoria, Lugar, Seleccion};
use militar::escuadra::{Contenedor, Escuadra};
use militar::heroe::Residencia;
use militar::presencia::Ubicacion;
use militar::tipos::Carga;
use pb::entidad::Cuerpo;
use std::collections::BTreeMap;

/// Una entidad del contrato lista para viajar: su clase, su id y su cuerpo.
pub type Proyectada = (pb::TipoEntidad, String, Cuerpo);

fn cantidades(c: impl IntoIterator<Item = (Recurso, Mil)>) -> pb::Cantidades {
    pb::Cantidades { por_id: c.into_iter().map(|(r, n)| (r.nombre().to_owned(), n)).collect() }
}

fn carga(c: &Carga) -> pb::Cantidades {
    cantidades(c.iter().map(|(&r, &n)| (r, n)))
}

/// El ritmo constante de una cifra que solo cambia a saltos: como no tiene pendiente, su `t0` no importa y vale 0, así que la proyección es una función del estado y no del instante en que se pide.
fn constante(valor: Mil) -> pb::Ritmo {
    ritmo(base::Ritmo::constante(valor, 0))
}

fn existencia(a: &Almacen, r: Recurso) -> pb::Existencia {
    pb::Existencia { cantidad: Some(pb::Ritmo { en_escalones: true, maximo: Some(a.capacidad(r)), ..constante(a.cantidad(r)) }), capacidad: a.capacidad(r) }
}

// ───────────────────────────── Héroe, escuadras, loadouts ─────────────────────────────

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
        display_name: format!("Héroe {}", h.heroe.id),
        class_definition_id: "default".into(),
        avatar: Some(pb::Avatar::default()),
        nivel: h.heroe.nivel,
        experiencia_hacia_siguiente_nivel: h.heroe.experiencia.max(0) as u64,
        puntos_de_atributo_sin_gastar: h.heroe.puntos_de_atributo.max(0) as u32,
        puntos_de_perk_sin_gastar: h.heroe.puntos_de_perk.max(0) as u32,
        atributos_base: Some(pb::Atributos {
            fuerza: h.heroe.atributos.fuerza.max(0) as u32,
            destreza: h.heroe.atributos.destreza.max(0) as u32,
            armadura: h.heroe.atributos.armadura.max(0) as u32,
            vitalidad: h.heroe.atributos.vitalidad.max(0) as u32,
        }),
        perks_desbloqueados: h.heroe.perks_desbloqueados.clone(),
        liderazgo_base: h.heroe.liderazgo_base.max(0) as u32,
        ubicacion: Some(pb::UbicacionHeroe { donde: Some(ubicacion) }),
        almacen_personal: Some(carga(&h.almacen_personal)),
        cupo_campamento: (!h.cupo.comprado.is_empty()).then(|| pb::CupoCampamento {
            dia: u32::try_from(h.cupo.dia).unwrap_or(0),
            comprado: h.cupo.comprado.iter().map(|(r, &n)| (r.nombre().to_owned(), u32::try_from(n).unwrap_or(u32::MAX))).collect(),
        }),
        racion_en_ms: h.heroe.racion_en,
        viveres: Some(constante(h.heroe.viveres)),
        bandidos_destruidos_en_ms: e.combate.destruidos.get(&h.heroe.id).cloned().unwrap_or_default(),
        herido_hasta_ms: h.heroe.herido_hasta,
        desconecta_en_ms: h.heroe.presencia.desconecta_en,
        fuera_carro: con_algo(&h.carro),
        escuadrones_ids: h.heroe.escuadras.iter().map(|s| ids::escuadra(s.id)).collect(),
        loadouts,
        monedas_heroe: Some(pb::Monedas { bronce: h.heroe.monedas.bronce.max(0) as u64, plata: h.heroe.monedas.plata.max(0) as u64, oro: h.heroe.monedas.oro.max(0) as u64 }),
        cupo_guarnicion: u32::try_from(cupo_guarnicion).unwrap_or(0),
        residencia_id: residencia,
        ..Default::default()
    }
}

/// Una escuadra. DUDA: la guarnición de una avanzada no tiene sitio en `ContenedorEscuadron`: sale como «en el campamento» (la lista `Avanzada.guarnicion_jugadores_ids` dice dónde está de verdad).
pub fn escuadron(n: &Nucleo, e: &Escuadra) -> pb::Escuadron {
    let contenedor = match e.contenedor {
        Contenedor::Campamento | Contenedor::Avanzada(_) => pb::contenedor_escuadron::Donde::Campamento(true),
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

// ───────────────────────────── Columnas, caravanas, convocatorias, batallas ─────────────────────────────

/// La columna, completa: con lo que persigue, si una batalla la tiene apartada y dentro de qué lugar está. DUDA: `te_sigue` es de la vista del perseguido (T3d): depende de otras columnas
/// y no está en el registro de cambios.
pub fn ejercito(n: &Nucleo, e: &Ejercito) -> pb::Ejercito {
    let mut x = super::ejercito_base(e, n.ahora());
    x.version = e.version;
    x.persiguiendo = e.persiguiendo.map(|presa| {
        let es_caravana = crate::estado_comercio::es_entidad_caravana(presa);
        let id = if es_caravana { ids::caravana(crate::estado_comercio::id_de_entidad(presa)) } else { ids::ejercito(presa) };
        pb::Persecucion { presa_es_caravana: es_caravana, presa_id: id, destino: None }
    });
    x.en_batalla = e.en_batalla;
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
        ..Default::default()
    }
}

fn seleccion(s: &Seleccion) -> pb::SeleccionDeSalida {
    pb::SeleccionDeSalida { escuadron_ids: s.escuadras.iter().map(|&e| ids::escuadra(e)).collect(), carga: Some(carga(&s.carga)) }
}

/// La convocatoria tal como la ve `viewer`: su Líder ve todas las peticiones y cada solicitante la suya.
pub fn convocatoria(c: &Convocatoria, viewer: Option<u64>, version: u64) -> pb::Convocatoria {
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
            .filter(|p| viewer == Some(c.lider) || viewer == Some(p.heroe))
            .map(|p| pb::PeticionConvocatoria { heroe_id: ids::heroe(p.heroe), pedido_en_ms: p.pedido_en, expira_en_ms: p.expira_en, seleccion: Some(seleccion(&p.seleccion)) })
            .collect(),
        creada_en_ms: c.creada_en,
    }
}

/// La batalla sin su ticket (T4 lo trae completo, con la asignación y los tokens): lo que el combatiente necesita para saber que combate y por qué. `lado_propio`, el de `viewer`.
pub fn batalla(n: &Nucleo, b: &Batalla, viewer: Option<u64>) -> pb::Batalla {
    let estado = match b.estado {
        EstadoBatalla::Convocando => pb::EstadoBatalla::BatallaConvocando,
        EstadoBatalla::Asignada => pb::EstadoBatalla::BatallaAsignada,
        EstadoBatalla::EnCurso => pb::EstadoBatalla::BatallaEnCurso,
        EstadoBatalla::Aplicada => pb::EstadoBatalla::BatallaAplicada,
        EstadoBatalla::Cancelada => pb::EstadoBatalla::BatallaCancelada,
        EstadoBatalla::Fallida => pb::EstadoBatalla::BatallaFallida,
    };
    // El bando del espectador: el de cualquiera de los héroes de la batalla cuya sesión sea la suya (el de campo de una cuenta combate por ella).
    let e = n.estado();
    let lado = viewer.and_then(|v| b.heroes_que_combaten().find(|&h| e.militar.heroes.get(&h).is_some_and(|x| audiencia::principal(e, x) == Some(v)))).and_then(|h| b.lado_de(h)).map(|l| if l == Lado::Atacante { pb::LadoId::Atacante } else { pb::LadoId::Defensor } as i32);
    pb::Batalla {
        id: n.uuid_de_batalla(b.id),
        version: n.version_de(Clase::Batalla, b.id),
        estado: estado as i32,
        iniciada_por: if n.heroe(b.iniciada_por).is_some() { ids::heroe(b.iniciada_por) } else { String::new() },
        punto: Some(punto(b.punto)),
        bloqueo_ejercito_ids: b.bloqueo.ejercitos.iter().map(|&e| ids::ejercito(e)).collect(),
        bloqueo_caravana_ids: b.bloqueo.caravanas.iter().map(|&c| ids::caravana(c)).collect(),
        bloqueo_asentamiento_id: b.bloqueo.asentamiento.map(|a| ids::asentamiento(u64::from(a))),
        bloqueo_campamento_id: b.bloqueo.campamento.map(ids::campamento),
        expira_en_ms: b.expira_en,
        convocatoria_hasta_ms: b.convocatoria_hasta,
        iniciada_en_ms: b.iniciada_en,
        aplicada_en_ms: b.aplicada_en,
        cerrada_en_ms: b.cerrada_en,
        lado_propio: lado,
        ..Default::default()
    }
}

// ───────────────────────────── Facción ─────────────────────────────

/// La Facción: nivel, reputación y ciudadanos. DUDA: sin nombre, sigilo, Rey, Embajador ni experiencia hasta el sistema 5 (política); la reputación va en milésimas de punto (el núcleo la guarda en ppm).
pub fn faccion(n: &Nucleo, f: u32) -> pb::Faccion {
    let e = n.estado();
    let rep = e.comercio.reputacion.get(&f).map_or_else(|| base::Ritmo::constante(0, 0), |r| r.ritmo());
    let milesimas = |v: Mil| v / 1_000;
    pb::Faccion {
        id: ids::faccion(f),
        version: n.version_de(Clase::Faccion, u64::from(f)),
        nivel: e.niveles_faccion.get(&f).copied().unwrap_or(1),
        ciudadanos_ids: e.militar.heroes.values().filter(|h| h.faccion == Some(f)).map(|h| ids::heroe(h.heroe.id)).collect(),
        reputacion: Some(ritmo(base::Ritmo { valor_en_t0: milesimas(rep.valor_en_t0), por_minuto: milesimas(rep.por_minuto), maximo: rep.maximo.map(milesimas), minimo: rep.minimo.map(milesimas), ..rep })),
        ..Default::default()
    }
}

/// Las tecnologías de la Facción: las de arranque y las que dio la administración. DUDA: sin `aparecidas` ni `reveladas` hasta el sistema 6.
pub fn tecnologias(n: &Nucleo, f: u32) -> pb::TecnologiasFaccion {
    let e = n.estado();
    let mut adoptadas: Vec<String> = e.reglas.economia.militar.tecnologias.de_arranque.clone();
    adoptadas.extend(e.militar.tecnologias.get(&f).into_iter().flatten().filter(|t| !adoptadas.contains(t)).cloned().collect::<Vec<_>>());
    pb::TecnologiasFaccion { faccion_id: ids::faccion(f), version: n.version_de(Clase::Faccion, u64::from(f)), adoptadas, ..Default::default() }
}

/// La relación que fijó la administración entre dos Facciones. DUDA: el stub no distingue al señor del vasallo (`a` es la menor) y no guarda cuándo se fijó.
pub fn relacion(n: &Nucleo, par: u64, r: Relacion) -> pb::RelacionPolitica {
    let (a, b) = Clase::facciones_del_par(par);
    pb::RelacionPolitica {
        id: ids::relacion(a, b),
        version: n.version_de(Clase::Relacion, par),
        tipo: match r {
            Relacion::Alianza => pb::TipoRelacion::RelacionAlianza,
            Relacion::Vasallaje => pb::TipoRelacion::RelacionVasallaje,
            Relacion::Guerra => pb::TipoRelacion::RelacionGuerra,
        } as i32,
        faccion_a_id: ids::faccion(a),
        faccion_b_id: ids::faccion(b),
        estado: pb::EstadoRelacion::RelacionActiva as i32,
        ..Default::default()
    }
}

// ───────────────────────────── Pueblos, avanzadas y sus obras ─────────────────────────────

/// El pueblo extractor activo. Mientras es una obra (pendiente de materiales o levantándose) el cliente lo ve como [`obra_de_pueblo`].
/// DUDA: el paso de minuto no marca el pueblo, así que su almacén local solo se refresca cuando otra cosa lo toca (T3c, con la tendencia).
pub fn pueblo(n: &Nucleo, madre: u64, p: &Pueblo) -> pb::Pueblo {
    let e = n.estado();
    let recurso = p.recurso(&e.reglas.economia.datos);
    let guarnicion = e.combate.guarniciones.get(&p.id);
    pb::Pueblo {
        id: ids::pueblo(p.id),
        version: n.version_de(Clase::Pueblo, p.id),
        asentamiento_madre_id: ids::asentamiento(madre),
        nodo_id: ids::nodo(p.nodo),
        tipo_extractor: p.tipo.nombre().into(),
        posicion: n.posicion_de_pueblo(p.id).map(punto),
        almacen_local: Some(existencia(&p.almacen, recurso)),
        carro_acarreo_id: p.carro.map(ids::caravana),
        guarnicion_escuadron_id: guarnicion.map(|g| ids::escuadra(g.escuadra.id)),
        guarnicion_repone_en_ms: guarnicion.and_then(|g| g.repone_en),
        ..Default::default()
    }
}

fn requerido(costo: &[(Recurso, Mil)]) -> pb::Cantidades {
    cantidades(costo.iter().copied())
}

/// La obra de un pueblo que aún no está activo. DUDA: una vez que arranca (`EnObra`) el núcleo ya no guarda lo pedido y lo entregado: salen vacíos y queda `termina_en_ms`.
pub fn obra_de_pueblo(n: &Nucleo, madre: u64, p: &Pueblo) -> pb::ObraDeMapa {
    let (req, ent, termina) = match &p.estado {
        EstadoPueblo::Pendiente { requerido: r, entregado } => (r.clone(), entregado.clone(), None),
        EstadoPueblo::EnObra { completa_en } => (Vec::new(), Vec::new(), Some(*completa_en)),
        EstadoPueblo::Activo => (Vec::new(), Vec::new(), None),
    };
    pb::ObraDeMapa {
        id: ids::obra(p.id),
        version: n.version_de(Clase::Pueblo, p.id),
        asentamiento_madre_id: ids::asentamiento(madre),
        tipo: pb::TipoObra::ObraPueblo as i32,
        posicion: n.posicion_de_pueblo(p.id).map(punto),
        nodo_id: Some(ids::nodo(p.nodo)),
        requerido: Some(requerido(&req)),
        entregado: Some(requerido(&ent)),
        termina_en_ms: termina,
        ..Default::default()
    }
}

/// La avanzada que ya es un puesto (operativa o en ruinas). La primera obra, la mejora y la reparación van aparte, en [`obra_de_avanzada`].
/// DUDA: sin `almacen` (el contrato trae una sola `Existencia` y el núcleo un almacén de 29 recursos) ni `batalla_id` (T4).
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

// ───────────────────────────── Por clase del registro de cambios ─────────────────────────────

/// Lo que una entidad del registro de cambios es en el contrato: las entidades que existen ahora y todos los ids en que puede aparecer (lo que no existe, si se quita, sale con `QUITA`).
/// Una clase puede ser dos entidades: un pueblo es `Pueblo` o `ObraDeMapa` según haya terminado; una avanzada es `Avanzada` y, mientras la obra dure, `ObraDeMapa`.
pub struct Proyeccion {
    pub existen: Vec<Proyectada>,
    pub posibles: Vec<(pb::TipoEntidad, String)>,
}

/// Proyecta la entidad `(clase, id)` para `viewer` (el héroe de la sesión, que decide qué peticiones y qué bando ve). Las clases que no viajan a `HEROE` ni a `FACCION` no tienen proyección.
pub fn proyectar(n: &Nucleo, viewer: Option<u64>, clase: Clase, id: u64) -> Proyeccion {
    use pb::TipoEntidad as T;
    let e = n.estado();
    let un = |tipo: T, id: String, cuerpo: Option<Cuerpo>| Proyeccion { posibles: vec![(tipo, id.clone())], existen: cuerpo.into_iter().map(|c| (tipo, id.clone(), c)).collect() };
    match clase {
        Clase::Ejercito => un(T::Ejercito, ids::ejercito(id), e.ejercitos.get(&id).map(|x| Cuerpo::Ejercito(ejercito(n, x)))),
        Clase::Asentamiento => un(T::Asentamiento, ids::asentamiento(id), e.asentamientos.get(&id).and_then(|a| super::entidad_asentamiento(a, &n.poligono_zona(a.id)).cuerpo)),
        Clase::Heroe => un(T::Heroe, ids::heroe(id), n.heroe(id).map(|h| Cuerpo::Heroe(heroe(n, h)))),
        Clase::Escuadra => un(T::Escuadron, ids::escuadra(id), n.escuadra(id).map(|(_, s)| Cuerpo::Escuadron(escuadron(n, s)))),
        Clase::Caravana => un(T::Caravana, ids::caravana(id), e.comercio.caravana(id).map(|c| Cuerpo::Caravana(caravana(n, c)))),
        Clase::Convocatoria => un(T::Convocatoria, ids::convocatoria(id), e.militar.convocatorias.get(&id).map(|c| Cuerpo::Convocatoria(convocatoria(c, viewer, n.version_de(Clase::Convocatoria, id))))),
        Clase::Acuerdo => un(T::AcuerdoTrueque, ids::acuerdo(id), e.comercio.acuerdo(id).map(|a| Cuerpo::AcuerdoTrueque(trueque(n, a)))),
        Clase::OrdenMercado => un(
            T::OrdenMercado,
            ids::orden_de_mercado(id),
            e.comercio.libro.ordenes.iter().chain(&e.comercio.libro.historial).find(|o| o.id == id).map(|o| Cuerpo::OrdenMercado(orden_de_mercado(n, o))),
        ),
        Clase::Batalla => un(T::Batalla, n.uuid_de_batalla(id), n.batalla(id).map(|b| Cuerpo::Batalla(batalla(n, b, viewer)))),
        Clase::Relacion => {
            let (a, b) = Clase::facciones_del_par(id);
            un(T::RelacionPolitica, ids::relacion(a, b), e.militar.relacion(a, b).map(|r| Cuerpo::Relacion(relacion(n, id, r))))
        }
        Clase::Faccion => {
            let f = id as u32;
            Proyeccion {
                posibles: vec![(T::Faccion, ids::faccion(f)), (T::TecnologiaFaccion, ids::faccion(f))],
                existen: vec![(T::Faccion, ids::faccion(f), Cuerpo::Faccion(faccion(n, f))), (T::TecnologiaFaccion, ids::faccion(f), Cuerpo::Tecnologias(tecnologias(n, f)))],
            }
        }
        Clase::Pueblo => {
            let (pueblo_id, obra_id) = (ids::pueblo(id), ids::obra(id));
            let encontrado = n.pueblo(id);
            let existe = encontrado.map(|(madre, p)| if p.activo() { (T::Pueblo, pueblo_id.clone(), Cuerpo::Pueblo(pueblo(n, madre, p))) } else { (T::ObraDeMapa, obra_id.clone(), Cuerpo::Obra(obra_de_pueblo(n, madre, p))) });
            Proyeccion { posibles: vec![(T::Pueblo, pueblo_id), (T::ObraDeMapa, obra_id)], existen: existe.into_iter().collect() }
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
        Clase::Campamento | Clase::Bandidos => Proyeccion { existen: Vec::new(), posibles: Vec::new() },
    }
}

/// Los cambios de `(clase, id)` para `viewer`: `ACTUALIZA` de lo que existe si `pertenece`, y `QUITA` de lo demás (todo, si la entidad ya no es del ámbito o ya no existe).
pub fn cambios(n: &Nucleo, viewer: Option<u64>, clase: Clase, id: u64, pertenece: bool) -> Vec<pb::Cambio> {
    let Proyeccion { existen, posibles } = proyectar(n, viewer, clase, id);
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
