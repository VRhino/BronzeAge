//! El ticket de una batalla de Unity y sus incorporaciones en el contrato 05 (`BattleTicket`, `IncorporacionBatalla`; 4H T4): lo que BronzeAge le manda a un servidor de batalla y lo que ven los
//! combatientes en `Batalla.ticket`. El ticket de `combate::ciclo` lleva lo que se congeló al abrir (los bandos, el nivel, la experiencia y la moral de cada escuadra, la plaza de un asedio); lo que no
//! puede cambiar mientras dura la batalla —el héroe, sus atributos y la carga de una caravana apartada— se lee del estado al proyectar.
//!
//! DUDA: el héroe sale con los valores por defecto de nombre, clase, género y avatar (decisión 4 del autor), sin equipo (CQ-004) y con los atributos efectivos iguales a los base; una escuadra no trae
//! habilidades, formaciones ni formación elegida (Conquest); `experiencia` y `moral` de una escuadra viajan en puntos (como `xp_ganada` del resultado y como el contrato de TypeScript), no en milésimas;
//! `version_balance` y `version_catalogo_tropas` valen 1 mientras Rust no tenga su numeración; la carga de cada carro es la parte de la carga de la caravana proporcional a su capacidad
//! (`Caravana::carga_por_carro`); el terreno de una batalla fuera de un asedio es el artefacto de mundo (`BattleMapReference`).

use super::propios::{atributos, celdas, nombre_de_heroe, CLASE_POR_DEFECTO};
use super::{ids, punto};
use crate::nucleo::Nucleo;
use combate::caravana::Modalidad;
use combate::ciclo::{Asignacion, BandoDeTicket, Batalla, CaravanaEnBatalla, Contexto, EscuadraEnBatalla, Incorporacion, Participante, PlazaCongelada};
use combate::resolucion::Lado;
use combate::resultado::{Botin, Razon, Resultado};
use contrato::v1 as pb;
use militar::heroe::Controlador;

/// La versión del esquema del contrato de batalla que habla este servidor (`SCHEMA_VERSION`).
pub const SCHEMA_VERSION: u32 = 1;

/// Hasta que Rust numere sus catálogos y su balance (el contrato de TypeScript los tomaba de `BALANCE_VERSION` y `VERSION_CATALOGO_TROPAS`).
const VERSION_POR_DEFECTO: u32 = 1;

/// Milésimas de punto a puntos enteros (la experiencia y la moral de un `SquadSnapshot`).
const MILESIMAS_POR_PUNTO: i64 = 1_000;

pub fn lado(l: Lado) -> pb::LadoId {
    if l == Lado::Atacante {
        pb::LadoId::Atacante
    } else {
        pb::LadoId::Defensor
    }
}

fn escuadra(e: &EscuadraEnBatalla) -> pb::SquadSnapshot {
    pb::SquadSnapshot {
        squad_id: ids::escuadra(e.id),
        heroe_id: e.dueno.map(ids::heroe),
        tropa_id: e.tropa.nombre().to_owned(),
        efectivos_autorizados: e.efectivos.max(0) as u32,
        nivel: u32::from(e.nivel),
        experiencia: e.experiencia / MILESIMAS_POR_PUNTO,
        moral: e.moral / MILESIMAS_POR_PUNTO,
        ..Default::default()
    }
}

fn participante(n: &Nucleo, p: &Participante) -> pb::BattleParticipantSnapshot {
    let h = n.heroe(p.heroe).map(|h| &h.heroe);
    pb::BattleParticipantSnapshot {
        heroe_id: ids::heroe(p.heroe),
        faccion_id: p.faccion.map(ids::faccion),
        es_bot: p.controlador == Controlador::Bot,
        heroe: Some(pb::HeroSnapshot {
            display_name: nombre_de_heroe(p.heroe),
            class_definition_id: CLASE_POR_DEFECTO.to_owned(),
            nivel: h.map_or(1, |h| h.nivel),
            avatar: Some(pb::Avatar::default()),
            perks_desbloqueados: h.map(|h| h.perks_desbloqueados.clone()).unwrap_or_default(),
            equipamiento: Some(pb::Equipamiento::default()),
            atributos_efectivos: h.map(atributos),
            casillas_inventario_libres: p.casillas_libres,
            ..Default::default()
        }),
        escuadras: p.escuadras.iter().map(escuadra).collect(),
    }
}

fn bando(n: &Nucleo, b: &BandoDeTicket) -> pb::BattleSide {
    pb::BattleSide {
        faccion_id: b.faccion.map(ids::faccion),
        capacidad_maxima: b.capacidad as u32,
        participantes: b.participantes.iter().map(|p| participante(n, p)).collect(),
        escuadras_sin_heroe: b.sin_heroe.iter().map(escuadra).collect(),
    }
}

/// Los carros de una caravana con su id (`"<caravana>:<posición>"`) y su carga. Una caravana que ya no existe no trae carros.
fn carros(n: &Nucleo, caravana: u64) -> Vec<pb::CarroEnBatalla> {
    let Some(c) = n.estado().comercio.caravana(caravana) else { return Vec::new() };
    let flota = &n.estado().reglas.economia.comercio.flota;
    c.carga_por_carro(flota)
        .into_iter()
        .enumerate()
        .map(|(i, carga)| pb::CarroEnBatalla { carro_id: format!("{}:{i}", ids::caravana(caravana)), carga: Some(pb::Cantidades { por_id: carga.into_iter().map(|(r, n)| (r.nombre().to_owned(), n)).collect() }) })
        .collect()
}

pub(super) fn contexto(n: &Nucleo, c: &Contexto) -> pb::ContextoEstrategico {
    use pb::contexto_estrategico::Tipo;
    let tipo = match c {
        Contexto::Asedio { asentamiento } => Tipo::Asedio(pb::ContextoAsedio { asentamiento_id: ids::asentamiento(u64::from(*asentamiento)) }),
        Contexto::CampoAbierto { punto: p, solitarios } => Tipo::CampoAbierto(pb::ContextoCampoAbierto { punto: Some(punto(*p)), solitarios: *solitarios }),
        Contexto::Caravana { caravanas, punto: p, modalidad } => {
            let en_juego = |k: &CaravanaEnBatalla| pb::CaravanaEnJuego { caravana_id: ids::caravana(k.id), carros: carros(n, k.id) };
            let primera = caravanas.first();
            Tipo::Caravana(pb::ContextoCaravana {
                caravana_id: primera.map(|k| ids::caravana(k.id)).unwrap_or_default(),
                punto: Some(punto(*p)),
                modalidad: if *modalidad == Modalidad::Disputa { pb::ModalidadCaravana::ModalidadDisputa } else { pb::ModalidadCaravana::ModalidadAguante } as i32,
                carros: primera.map(|k| carros(n, k.id)).unwrap_or_default(),
                caravanas: caravanas.iter().map(en_juego).collect(),
            })
        }
        Contexto::CampamentoBandidos { campamento, punto: p } => Tipo::CampamentoBandidos(pb::ContextoCampamentoBandidos { campamento_id: ids::bandidos(u64::from(*campamento)), punto: Some(punto(*p)) }),
        Contexto::SaqueoPueblo { pueblo, punto: p } => {
            // Lo que el pueblo no deja llevarse: la fracción protegida de la capacidad de su almacén, del recurso que extrae.
            let e = n.estado();
            let protegido = n.pueblo(*pueblo).map(|(_, v)| {
                let r = v.recurso(&e.reglas.economia.datos);
                (r.nombre().to_owned(), (i128::from(v.almacen.capacidad(r)) * i128::from(e.reglas.economia.combate.numericas.protegida_pueblo) / i128::from(asentamiento::datos::UNO)) as i64)
            });
            Tipo::SaqueoPueblo(pb::ContextoSaqueoPueblo { pueblo_id: ids::pueblo(*pueblo), punto: Some(punto(*p)), protegido: Some(pb::Cantidades { por_id: protegido.into_iter().collect() }) })
        }
        Contexto::Avanzada { avanzada, etapa, punto: p } => Tipo::Avanzada(pb::ContextoAvanzada { avanzada_id: ids::avanzada(*avanzada), etapa: etapa.indice() as u32 + 1, punto: Some(punto(*p)) }),
    };
    pb::ContextoEstrategico { tipo: Some(tipo) }
}

fn mapa_de_plaza(plaza: u64, p: &PlazaCongelada) -> pb::SettlementBattleSnapshot {
    pb::SettlementBattleSnapshot {
        settlement_id: ids::asentamiento(plaza),
        layout_version: p.layout_version,
        unidades_por_celda: p.unidades_por_celda,
        edificios: p
            .edificios
            .iter()
            .map(|e| pb::EdificioEnBatalla {
                edificio_id: ids::edificio(plaza, e.id),
                tipo: e.tipo.clone(),
                posicion: Some(punto(e.centro)),
                ancho: e.ancho,
                alto: e.alto,
                nivel_interno: Some(u32::from(e.nivel_interno)),
                estado: match e.estado {
                    asentamiento::edificio::Estado::EnCola => pb::EstadoEdificio::EdificioEnCola,
                    asentamiento::edificio::Estado::EnConstruccion => pb::EstadoEdificio::EdificioEnConstruccion,
                    asentamiento::edificio::Estado::Activo => pb::EstadoEdificio::EdificioActivo,
                } as i32,
                danado: e.danado,
            })
            .collect(),
        recintos: p.recintos.iter().map(|r| pb::RecintoEnBatalla { recinto_id: ids::recinto(plaza, r.id), nivel: u32::from(r.nivel), celdas: celdas(&r.celdas) }).collect(),
    }
}

/// El `BattleTicket` de una batalla, tal como se congeló al abrir (con la revisión vigente).
pub fn ticket(n: &Nucleo, b: &Batalla) -> pb::BattleTicket {
    let t = &b.ticket;
    let c = &n.estado().reglas.economia.combate.batalla;
    let mapa = match (&t.contexto, &t.plaza) {
        (&Contexto::Asedio { asentamiento }, Some(p)) => pb::battle_ticket::Mapa::MapaAsentamiento(mapa_de_plaza(u64::from(asentamiento), p)),
        _ => pb::battle_ticket::Mapa::MapaReferencia(pb::BattleMapReference { mapa_id: n.mundo_hash().to_owned(), centro: Some(punto(b.punto)) }),
    };
    pb::BattleTicket {
        schema_version: SCHEMA_VERSION,
        battle_id: n.uuid_de_batalla(b.id),
        game_id: n.id_de_partida(),
        ticket_revision: t.revision,
        contexto_estrategico: Some(contexto(n, &t.contexto)),
        atacante: Some(bando(n, &t.atacante)),
        defensor: Some(bando(n, &t.defensor)),
        mapa: Some(mapa),
        reglas: Some(pb::BattleRules {
            duracion_maxima_segundos: (t.duracion_maxima / 1_000) as u32,
            version_balance: VERSION_POR_DEFECTO,
            version_catalogo_tropas: VERSION_POR_DEFECTO,
            version_catalogo_heroe: c.version_catalogo_heroe.clone(),
            version_catalogo_objetos: c.version_catalogo_objetos.clone(),
        }),
    }
}

/// Quien se unió después del ticket.
pub fn incorporacion(n: &Nucleo, b: &Batalla, i: &Incorporacion) -> pb::IncorporacionBatalla {
    pb::IncorporacionBatalla { schema_version: SCHEMA_VERSION, battle_id: n.uuid_de_batalla(b.id), secuencia: i.secuencia, lado: lado(i.lado) as i32, participante: Some(participante(n, &i.participante)) }
}

/// La asignación que ganó, **sin los tokens** (`tokens_participante` vacío): lo que ve la administración (`ADMIN`). Los tokens son de cada combatiente (`Batalla.asignacion_propia`).
pub fn asignacion_sin_tokens(n: &Nucleo, b: &Batalla, a: &Asignacion) -> pb::BattleServerAssignment {
    let i = &a.instancia;
    pb::BattleServerAssignment {
        schema_version: SCHEMA_VERSION,
        battle_id: n.uuid_de_batalla(b.id),
        ticket_revision: a.revision,
        intento_asignacion_id: a.intento.clone(),
        instancia: Some(pb::InstanciaDeBatalla { host: i.host.clone(), puerto: u32::from(i.puerto), protocolo: i.protocolo.clone() }),
        tokens_participante: Vec::new(),
        servidor_id: a.servidor.clone(),
    }
}

/// Por qué ganó el que ganó.
pub(super) fn razon(r: Razon) -> pb::RazonVictoria {
    match r {
        Razon::ObjetivosCapturados => pb::RazonVictoria::ObjetivosCapturados,
        Razon::Aniquilacion => pb::RazonVictoria::Aniquilacion,
        Razon::TiempoAgotado => pb::RazonVictoria::TiempoAgotado,
    }
}

/// El resultado aplicado, tal como lo mandó Unity (se guarda entero en la batalla).
pub fn resultado(n: &Nucleo, b: &Batalla, r: &Resultado) -> pb::BattleResult {
    let botin = |b: &Botin| pb::Botin {
        objetos: b.objetos.iter().map(|o| pb::ObjetoBotin { item_definition_id: o.definicion.clone(), cantidad: o.cantidad.max(0) as u32, item_instance_id: o.instancia.clone(), estadisticas: Vec::new() }).collect(),
        monedas: Some(pb::Monedas { bronce: b.monedas.bronce.max(0) as u64, plata: b.monedas.plata.max(0) as u64, oro: b.monedas.oro.max(0) as u64 }),
    };
    pb::BattleResult {
        schema_version: SCHEMA_VERSION,
        battle_id: n.uuid_de_batalla(b.id),
        result_id: r.resultado_id.clone(),
        ticket_revision: r.revision,
        intento_asignacion_id: r.intento.clone(),
        inicio_ms: r.inicio,
        fin_ms: r.fin,
        ganador: lado(r.ganador) as i32,
        razon: razon(r.razon) as i32,
        objetivos: r.objetivos.iter().map(|(id, por)| pb::ObjetivoResultado { objetivo_id: id.clone(), capturado_por: por.map(|l| lado(l) as i32) }).collect(),
        por_escuadra: r
            .por_escuadra
            .iter()
            .map(|e| pb::ResultadoEscuadra { squad_id: ids::escuadra(e.escuadra), desplegados: e.desplegados.max(0) as u32, supervivientes_al_cierre: e.supervivientes_al_cierre.max(0) as u32, muertos: e.muertos.max(0) as u32, xp_ganada: e.xp_ganada })
            .collect(),
        por_heroe: r
            .por_heroe
            .iter()
            .map(|h| pb::ResultadoHeroe { heroe_id: ids::heroe(h.heroe), participo: h.participo, sobrevivio_al_cierre: h.sobrevivio_al_cierre, xp_ganada: h.xp_ganada, derrotado: h.derrotado, botin: h.botin.as_ref().map(botin) })
            .collect(),
        version_servidor: r.version_servidor.clone(),
        carros_perdidos: r.carros_perdidos.clone(),
    }
}
