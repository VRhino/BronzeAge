//! El servidor de batalla de mentira: sin Unity, para probar el lanzador y como ejemplo del protocolo que tiene que hablar Conquest (Batalla_Unity_Ciclo §8.8). Lo llevan
//! `bronzeage-batalla-de-mentira` y el propio `bronzeage-lanzador mentira` (el que arranca el lanzador con `LANZADOR_PROCESO=mentira`).
//!
//! El lanzador lo arranca con `--batalla <uuid> --lanzador 127.0.0.1:<p> --puerto <n>`. Se conecta a ese socket, lee el ticket y la asignación (`MensajeParaProcesoDeBatalla`,
//! delimitados), manda `inicio`, espera `--espera-ms` (200 por defecto) y manda el `resultado`: nadie muere, todos los héroes participan y sobreviven, y gana el defensor por tiempo
//! agotado (la experiencia la pone el motor, D7). Sale cuando el núcleo le acusa el resultado. Con `--sin-resultado`, sale tras el inicio sin mandarlo.

use std::{collections::BTreeSet, time::Duration};

use contrato::v1::{
    self as pb, mensaje_de_servidor_de_batalla as de_srv, mensaje_para_proceso_de_batalla as al_proceso, MensajeDeServidorDeBatalla, MensajeParaProcesoDeBatalla,
};
use crate::{marco, SCHEMA_VERSION};
use tokio::{io::BufReader, net::TcpStream};

fn arg(nombre: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == nombre).and_then(|i| a.get(i + 1)).cloned()
}

fn ahora_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

fn resultado(t: &pb::BattleTicket, incorporaciones: &[pb::IncorporacionBatalla], a: &pb::BattleServerAssignment, inicio: i64) -> pb::BattleResult {
    let lados = [&t.atacante, &t.defensor];
    let participantes: Vec<&pb::BattleParticipantSnapshot> =
        lados.iter().filter_map(|b| b.as_ref()).flat_map(|b| &b.participantes).chain(incorporaciones.iter().filter_map(|i| i.participante.as_ref())).collect();
    let mut vistas = BTreeSet::new();
    let escuadras = participantes.iter().flat_map(|p| &p.escuadras).chain(lados.iter().filter_map(|b| b.as_ref()).flat_map(|b| &b.escuadras_sin_heroe)).filter(|e| vistas.insert(e.squad_id.clone()));
    let duracion = t.reglas.as_ref().map_or(60_000, |r| i64::from(r.duracion_maxima_segundos) * 1000);
    pb::BattleResult {
        schema_version: SCHEMA_VERSION,
        battle_id: t.battle_id.clone(),
        result_id: format!("res-{}-{}", t.battle_id, a.intento_asignacion_id),
        ticket_revision: t.ticket_revision,
        intento_asignacion_id: a.intento_asignacion_id.clone(),
        inicio_ms: inicio,
        fin_ms: (inicio + 1).max(ahora_ms()).min(inicio + duracion),
        ganador: pb::LadoId::Defensor as i32,
        razon: pb::RazonVictoria::TiempoAgotado as i32,
        objetivos: vec![],
        por_escuadra: escuadras
            .map(|e| pb::ResultadoEscuadra { squad_id: e.squad_id.clone(), desplegados: e.efectivos_autorizados, supervivientes_al_cierre: e.efectivos_autorizados, muertos: 0 })
            .collect(),
        por_heroe: participantes
            .iter()
            .map(|p| pb::ResultadoHeroe { heroe_id: p.heroe_id.clone(), participo: true, sobrevivio_al_cierre: true, derrotado: Some(false) })
            .collect(),
        version_servidor: format!("batalla-de-mentira {}", env!("CARGO_PKG_VERSION")),
        carros_perdidos: vec![],
    }
}

/// Juega la batalla de los argumentos del proceso y vuelve al acabar.
pub async fn correr() {
    let (Some(lanzador), Some(batalla)) = (arg("--lanzador"), arg("--batalla")) else {
        eprintln!("uso: bronzeage-batalla-de-mentira --batalla <uuid> --lanzador <host:puerto> --puerto <n> [--espera-ms N] [--sin-resultado]");
        std::process::exit(2);
    };
    let espera = arg("--espera-ms").and_then(|e| e.parse().ok()).unwrap_or(200);
    let sin_resultado = std::env::args().any(|a| a == "--sin-resultado");
    let tcp = TcpStream::connect(&lanzador).await.expect("conectar con el lanzador");
    let (lectura, mut escritura) = tcp.into_split();
    let mut lectura = BufReader::new(lectura);
    let mandar = |c| MensajeDeServidorDeBatalla { cuerpo: Some(c) };
    let (mut ticket, mut incorporaciones): (Option<pb::BattleTicket>, Vec<pb::IncorporacionBatalla>) = (None, Vec::new());
    let mut empezada: Option<(pb::BattleServerAssignment, i64)> = None;
    let mut fin = tokio::time::Instant::now() + Duration::from_secs(3600);
    loop {
        tokio::select! {
            m = marco::leer::<MensajeParaProcesoDeBatalla, _>(&mut lectura) => {
                let Ok(Some(m)) = m else { return };
                match m.cuerpo {
                    Some(al_proceso::Cuerpo::Ticket(t)) => {
                        assert_eq!(t.battle_id, batalla, "el ticket es de mi batalla");
                        ticket = Some(t);
                    }
                    // Con cada asignación (la primera, o la de una revisión nueva) empieza: el inicio, y el resultado pasada la espera.
                    Some(al_proceso::Cuerpo::Asignacion(a)) => {
                        let t = ticket.as_ref().expect("el ticket llega antes que la asignación");
                        let inicio = pb::InicioBatalla { schema_version: SCHEMA_VERSION, battle_id: t.battle_id.clone(), ticket_revision: t.ticket_revision, intento_asignacion_id: a.intento_asignacion_id.clone() };
                        marco::escribir(&mut escritura, &mandar(de_srv::Cuerpo::Inicio(inicio))).await.expect("escribir");
                        empezada = Some((a, ahora_ms()));
                        fin = tokio::time::Instant::now() + Duration::from_millis(espera);
                    }
                    Some(al_proceso::Cuerpo::Incorporacion(i)) => incorporaciones.push(i),
                    Some(al_proceso::Cuerpo::Acuse(a)) if a.mensaje == "resultado" => return,
                    _ => {}
                }
            }
            _ = tokio::time::sleep_until(fin), if empezada.is_some() => {
                if sin_resultado {
                    return;
                }
                let (a, inicio) = empezada.take().unwrap();
                let r = resultado(ticket.as_ref().unwrap(), &incorporaciones, &a, inicio);
                marco::escribir(&mut escritura, &mandar(de_srv::Cuerpo::Resultado(r))).await.expect("escribir");
                fin = tokio::time::Instant::now() + Duration::from_secs(3600);
            }
        }
    }
}
