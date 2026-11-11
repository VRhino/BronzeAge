//! El lanzador de servidores de batalla (decisión del autor del 2026-11-11; Batalla_Unity_Ciclo §8.8). El servidor de batallas es Conquest compilado como servidor dedicado,
//! **un proceso por batalla**; quien habla con el motor es el lanzador, que corre en la misma máquina que esos procesos:
//!
//! 1. Entra en el motor con una cuenta de servidor de batalla (`POST /v1/sesiones`; la cuenta se registra con `CODIGO_REGISTRO_SERVIDORES`) y abre la sesión
//!    `CLIENTE_SERVIDOR_BATALLA` del contrato 05 ([`motor`]). Si se cae, vuelve a entrar y repite lo que el núcleo no le había acusado.
//! 2. Por cada `BattleTicket` que le llega, si tiene un puerto de juego libre, **reclama** la batalla: manda la `BattleServerAssignment` con su instancia (`host_publico`, el
//!    puerto, `udp`) y un token al azar para cada héroe humano. Gana la primera asignación; si el núcleo la rechaza (`batalla.ya_asignada`), suelta el puerto.
//! 3. Aceptada, **arranca el proceso de batalla** (`proceso` con sus `argumentos` y detrás `--batalla <uuid> --lanzador 127.0.0.1:<p> --puerto <n>`) y le habla por ese socket
//!    local con mensajes delimitados ([`marco`]): le manda `MensajeParaProcesoDeBatalla` (el ticket, la asignación, las incorporaciones con los tokens de los humanos que se
//!    unen y los acuses del núcleo) y reenvía al núcleo lo que el proceso le contesta (`inicio`, `salida` y `resultado`).
//! 4. Un ticket con la revisión subida (una salida `no_conectado` antes de empezar) se reclama otra vez con el mismo puerto y se le pasa al mismo proceso.
//! 5. Cuando el proceso termina, el puerto queda libre. Si acabó sin resultado, la batalla la vence el núcleo por su plazo (§6), sin castigo.
//!
//! El lanzador no guarda nada en disco: si se reinicia, las batallas que tenía las vence el núcleo por su plazo.

pub mod marco;
pub mod mentira;
pub mod motor;

use std::{
    collections::BTreeMap,
    future::Future,
    path::PathBuf,
    process::Stdio,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use contrato::v1::{
    self as pb, mensaje_de_servidor_de_batalla as de_srv, mensaje_para_proceso_de_batalla as al_proceso, mensaje_para_servidor_de_batalla as para, MensajeDeServidorDeBatalla,
    MensajeParaProcesoDeBatalla, Resultado,
};
use tokio::{
    io::BufReader,
    net::TcpListener,
    sync::{mpsc, oneshot},
};

/// La versión de los esquemas del contrato de batalla que habla (`schema_version`).
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct Config {
    /// Dónde escucha el motor (`host:puerto`): el HTTP de acceso y el WebSocket del contrato 05 van por el mismo puerto.
    pub motor: String,
    /// La cuenta de servidor de batalla.
    pub nick: String,
    pub clave: String,
    /// El `CODIGO_REGISTRO_SERVIDORES` del motor: con él, si la cuenta no existe, la da de alta al entrar.
    pub codigo: Option<String>,
    /// A dónde se conectan los jugadores (`InstanciaDeBatalla.host`): la dirección pública de esta máquina.
    pub host_publico: String,
    /// Los puertos de juego: uno por batalla a la vez, así que son también cuántas batallas lleva a la vez.
    pub puertos: Vec<u16>,
    /// El ejecutable del proceso de batalla y sus argumentos fijos (los de cada batalla van detrás).
    pub proceso: PathBuf,
    pub argumentos: Vec<String>,
    /// Lo que espera a que un proceso recién arrancado se conecte a su socket; si no lo hace, lo mata.
    pub espera_proceso: Duration,
    /// Lo que vale un token de jugador desde que se emite.
    pub duracion_token: Duration,
    /// Lo que espera para volver a entrar en el motor tras perder la sesión.
    pub espera_reconexion: Duration,
}

impl Config {
    pub fn nueva(motor: &str, nick: &str, clave: &str, proceso: impl Into<PathBuf>) -> Self {
        Config {
            motor: motor.into(),
            nick: nick.into(),
            clave: clave.into(),
            codigo: None,
            host_publico: "127.0.0.1".into(),
            puertos: (7777..=7786).collect(),
            proceso: proceso.into(),
            argumentos: Vec::new(),
            espera_proceso: Duration::from_secs(120),
            duracion_token: Duration::from_secs(2 * 3600),
            espera_reconexion: Duration::from_secs(5),
        }
    }
}

/// Los puertos de `7777-7786`, `7777,7780` o una mezcla.
pub fn puertos(texto: &str) -> Result<Vec<u16>, String> {
    let mut v = Vec::new();
    for parte in texto.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let numero = |s: &str| s.trim().parse::<u16>().map_err(|_| format!("«{s}» no es un puerto"));
        match parte.split_once('-') {
            Some((a, b)) => {
                let (a, b) = (numero(a)?, numero(b)?);
                if a > b {
                    return Err(format!("«{parte}»: el primero es mayor"));
                }
                v.extend(a..=b);
            }
            None => v.push(numero(parte)?),
        }
    }
    v.sort_unstable();
    v.dedup();
    if v.is_empty() {
        return Err("sin puertos".into());
    }
    Ok(v)
}

/// Lo que va pasando, para el registro del proceso y para las pruebas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Informe {
    Conectado,
    Desconectado(String),
    /// Manda la asignación de una batalla (con el puerto y los héroes humanos a los que dio token).
    Reclama { batalla: String, revision: u32, puerto: u16, humanos: Vec<String> },
    SinPuerto { batalla: String },
    /// El núcleo le dio la batalla.
    Asignada { batalla: String, revision: u32 },
    /// El núcleo se la dio a otro, o la rechazó.
    Rechazada { batalla: String, mensaje: String, codigo: String },
    ProcesoArrancado { batalla: String },
    ProcesoConectado { batalla: String },
    /// Reenvió al núcleo un mensaje del proceso (`inicio`, `salida`, `resultado`) o un token suyo.
    Reenvia { batalla: String, mensaje: String },
    /// El acuse del núcleo de ese mensaje.
    Acuse { batalla: String, mensaje: String, resultado: Resultado, codigo: Option<String> },
    /// El proceso mandó algo que no le toca (la asignación o los tokens los pone el lanzador).
    Descartado { batalla: String, mensaje: String },
    /// El proceso terminó; `con_resultado` si el núcleo había acusado su resultado.
    Terminada { batalla: String, con_resultado: bool, motivo: String },
}

/// Lo que le llega al bucle del lanzador.
#[derive(Debug)]
pub enum Evento {
    MotorConectado(mpsc::Sender<MensajeDeServidorDeBatalla>),
    MotorCaido(String),
    Motor(Box<para::Cuerpo>),
    Proceso { batalla: String, que: DelProceso },
}

#[derive(Debug)]
pub enum DelProceso {
    Arrancado,
    Conectado(mpsc::UnboundedSender<MensajeParaProcesoDeBatalla>),
    Mensaje(Box<MensajeDeServidorDeBatalla>),
    Terminado(String),
}

fn ahora_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

/// `n` bytes al azar en hexadecimal.
fn azar(n: usize) -> String {
    let mut b = vec![0u8; n];
    getrandom::getrandom(&mut b).expect("el sistema da bytes al azar");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn nombre(c: &de_srv::Cuerpo) -> &'static str {
    match c {
        de_srv::Cuerpo::Asignacion(_) => "asignacion",
        de_srv::Cuerpo::Inicio(_) => "inicio",
        de_srv::Cuerpo::Salida(_) => "salida",
        de_srv::Cuerpo::Resultado(_) => "resultado",
        de_srv::Cuerpo::Tokens(_) => "tokens",
    }
}

/// Los héroes humanos de un ticket (los bots no se conectan).
fn humanos(t: &pb::BattleTicket) -> Vec<String> {
    [&t.atacante, &t.defensor].into_iter().flatten().flat_map(|b| &b.participantes).filter(|p| !p.es_bot).map(|p| p.heroe_id.clone()).collect()
}

/// Una batalla que el lanzador reclama o tiene.
struct Batalla {
    ticket: pb::BattleTicket,
    asignacion: pb::BattleServerAssignment,
    puerto: u16,
    /// El núcleo aceptó la asignación vigente.
    aceptada: bool,
    /// El proceso: mientras arranca, `None` en `proceso` y lo que haya que mandarle espera en `cola`.
    arrancado: bool,
    proceso: Option<mpsc::UnboundedSender<MensajeParaProcesoDeBatalla>>,
    cola: Vec<MensajeParaProcesoDeBatalla>,
    matar: Option<oneshot::Sender<()>>,
    /// Las incorporaciones que llegaron antes de que el núcleo aceptara la asignación.
    incorporaciones: Vec<pb::IncorporacionBatalla>,
    /// Lo que se mandó al núcleo y aún no tiene acuse: se repite al volver a entrar.
    sin_acuse: Vec<MensajeDeServidorDeBatalla>,
    con_resultado: bool,
}

impl Batalla {
    fn al_proceso(&mut self, c: al_proceso::Cuerpo) {
        let m = MensajeParaProcesoDeBatalla { cuerpo: Some(c) };
        match &self.proceso {
            Some(tx) => {
                let _ = tx.send(m);
            }
            None => self.cola.push(m),
        }
    }
}

struct Lanzador {
    config: Config,
    eventos: mpsc::UnboundedSender<Evento>,
    informes: Option<mpsc::UnboundedSender<Informe>>,
    motor: Option<mpsc::Sender<MensajeDeServidorDeBatalla>>,
    batallas: BTreeMap<String, Batalla>,
}

impl Lanzador {
    fn informar(&self, i: Informe) {
        if let Some(tx) = &self.informes {
            let _ = tx.send(i);
        }
    }

    fn al_motor(&self, m: MensajeDeServidorDeBatalla) {
        if let Some(tx) = &self.motor {
            let _ = tx.try_send(m);
        }
    }

    fn token(&self, heroe: &str) -> pb::TokenParticipante {
        pb::TokenParticipante { heroe_id: heroe.to_owned(), token: azar(32), expira_en_ms: ahora_ms() + self.config.duracion_token.as_millis() as i64 }
    }

    fn puerto_libre(&self) -> Option<u16> {
        self.config.puertos.iter().copied().find(|p| !self.batallas.values().any(|b| b.puerto == *p))
    }

    /// La asignación para la revisión de `ticket`, con un intento nuevo.
    fn asignacion(&self, ticket: &pb::BattleTicket, puerto: u16) -> pb::BattleServerAssignment {
        pb::BattleServerAssignment {
            schema_version: SCHEMA_VERSION,
            battle_id: ticket.battle_id.clone(),
            ticket_revision: ticket.ticket_revision,
            intento_asignacion_id: format!("lz-{}", azar(8)),
            instancia: Some(pb::InstanciaDeBatalla { host: self.config.host_publico.clone(), puerto: u32::from(puerto), protocolo: "udp".into() }),
            tokens_participante: humanos(ticket).iter().map(|h| self.token(h)).collect(),
            servidor_id: String::new(),
        }
    }

    fn reclamar(&mut self, id: &str) {
        let Some(b) = self.batallas.get(id) else { return };
        let m = MensajeDeServidorDeBatalla { cuerpo: Some(de_srv::Cuerpo::Asignacion(b.asignacion.clone())) };
        self.informar(Informe::Reclama {
            batalla: id.to_owned(),
            revision: b.asignacion.ticket_revision,
            puerto: b.puerto,
            humanos: b.asignacion.tokens_participante.iter().map(|t| t.heroe_id.clone()).collect(),
        });
        self.batallas.get_mut(id).unwrap().sin_acuse.push(m.clone());
        self.al_motor(m);
    }

    fn ticket(&mut self, t: pb::BattleTicket) {
        let id = t.battle_id.clone();
        match self.batallas.get(&id) {
            // Al volver a entrar el núcleo repite el ticket de lo que ya tiene: nada que hacer.
            Some(b) if t.ticket_revision <= b.ticket.ticket_revision => {}
            // La revisión subió: se reclama otra vez, con el mismo puerto y el mismo proceso.
            Some(b) => {
                let asignacion = self.asignacion(&t, b.puerto);
                let b = self.batallas.get_mut(&id).unwrap();
                (b.ticket, b.asignacion, b.aceptada) = (t, asignacion, false);
                b.sin_acuse.retain(|m| !matches!(m.cuerpo, Some(de_srv::Cuerpo::Asignacion(_))));
                self.reclamar(&id);
            }
            None => {
                let Some(puerto) = self.puerto_libre() else {
                    self.informar(Informe::SinPuerto { batalla: id });
                    return;
                };
                let asignacion = self.asignacion(&t, puerto);
                let b = Batalla {
                    ticket: t,
                    asignacion,
                    puerto,
                    aceptada: false,
                    arrancado: false,
                    proceso: None,
                    cola: Vec::new(),
                    matar: None,
                    incorporaciones: Vec::new(),
                    sin_acuse: Vec::new(),
                    con_resultado: false,
                };
                self.batallas.insert(id.clone(), b);
                self.reclamar(&id);
            }
        }
    }

    /// Un humano que se incorpora con la batalla ya asignada: su token al núcleo y al proceso.
    fn incorporar(&mut self, id: &str, i: pb::IncorporacionBatalla) {
        let humano = i.participante.as_ref().filter(|p| !p.es_bot).map(|p| self.token(&p.heroe_id));
        let Some(b) = self.batallas.get_mut(id) else { return };
        b.al_proceso(al_proceso::Cuerpo::Incorporacion(i));
        if let Some(token) = humano {
            let tokens = pb::TokensBatalla { schema_version: SCHEMA_VERSION, battle_id: id.to_owned(), intento_asignacion_id: b.asignacion.intento_asignacion_id.clone(), tokens_participante: vec![token] };
            b.al_proceso(al_proceso::Cuerpo::Tokens(tokens.clone()));
            let m = MensajeDeServidorDeBatalla { cuerpo: Some(de_srv::Cuerpo::Tokens(tokens)) };
            b.sin_acuse.push(m.clone());
            self.informar(Informe::Reenvia { batalla: id.to_owned(), mensaje: "tokens".into() });
            self.al_motor(m);
        }
    }

    fn arrancar(&mut self, id: &str) {
        let Some(b) = self.batallas.get_mut(id) else { return };
        b.arrancado = true;
        let (matar, muerte) = oneshot::channel();
        b.matar = Some(matar);
        let (config, eventos, id, puerto) = (self.config.clone(), self.eventos.clone(), id.to_owned(), b.puerto);
        tokio::spawn(proceso(config, id, puerto, eventos, muerte));
    }

    fn acuse(&mut self, a: pb::AcuseBatalla) {
        let id = a.battle_id.clone();
        let resultado = Resultado::try_from(a.resultado).unwrap_or(Resultado::OrdenRechazada);
        let bien = matches!(resultado, Resultado::OrdenAceptada | Resultado::OrdenSinCambios | Resultado::OrdenDuplicada);
        let codigo = a.rechazo.as_ref().map(|r| r.codigo.clone());
        let Some(b) = self.batallas.get_mut(&id) else { return };
        if let Some(i) = b.sin_acuse.iter().position(|m| m.cuerpo.as_ref().is_some_and(|c| nombre(c) == a.mensaje)) {
            b.sin_acuse.remove(i);
        }
        self.informar(Informe::Acuse { batalla: id.clone(), mensaje: a.mensaje.clone(), resultado, codigo: codigo.clone() });
        match a.mensaje.as_str() {
            "asignacion" if bien => {
                let b = self.batallas.get_mut(&id).unwrap();
                if b.aceptada {
                    return;
                }
                b.aceptada = true;
                let (ticket, asignacion, revision) = (b.ticket.clone(), b.asignacion.clone(), b.ticket.ticket_revision);
                b.al_proceso(al_proceso::Cuerpo::Ticket(ticket));
                b.al_proceso(al_proceso::Cuerpo::Asignacion(asignacion));
                let (arrancado, pendientes) = (b.arrancado, std::mem::take(&mut b.incorporaciones));
                self.informar(Informe::Asignada { batalla: id.clone(), revision });
                for i in pendientes {
                    self.incorporar(&id, i);
                }
                if !arrancado {
                    self.arrancar(&id);
                }
            }
            "asignacion" => {
                // Otro servidor la tiene, o ya no vale: se suelta (y, si era una revisión nueva de una que llevaba, se para su proceso).
                let b = self.batallas.remove(&id).unwrap();
                if let Some(m) = b.matar {
                    let _ = m.send(());
                }
                self.informar(Informe::Rechazada { batalla: id, mensaje: a.mensaje, codigo: codigo.unwrap_or_default() });
            }
            "tokens" => {}
            _ => {
                let b = self.batallas.get_mut(&id).unwrap();
                if a.mensaje == "resultado" && bien {
                    b.con_resultado = true;
                }
                b.al_proceso(al_proceso::Cuerpo::Acuse(a));
            }
        }
    }

    fn del_motor(&mut self, c: para::Cuerpo) {
        match c {
            para::Cuerpo::Ticket(t) => self.ticket(t),
            para::Cuerpo::Incorporacion(i) => {
                let id = i.battle_id.clone();
                match self.batallas.get_mut(&id) {
                    Some(b) if !b.aceptada => b.incorporaciones.push(i),
                    Some(_) => self.incorporar(&id, i),
                    None => {}
                }
            }
            para::Cuerpo::Acuse(a) => self.acuse(a),
            // El núcleo custodia los tokens y no manda este (05 §9).
            para::Cuerpo::Tokens(_) => {}
        }
    }

    fn del_proceso(&mut self, id: String, que: DelProceso) {
        match que {
            DelProceso::Arrancado => self.informar(Informe::ProcesoArrancado { batalla: id }),
            DelProceso::Conectado(tx) => {
                let Some(b) = self.batallas.get_mut(&id) else { return };
                for m in b.cola.drain(..) {
                    let _ = tx.send(m);
                }
                b.proceso = Some(tx);
                self.informar(Informe::ProcesoConectado { batalla: id });
            }
            DelProceso::Mensaje(m) => {
                let mut m = *m;
                let Some(c) = m.cuerpo.as_mut() else { return };
                let mensaje = nombre(c).to_owned();
                if matches!(c, de_srv::Cuerpo::Asignacion(_) | de_srv::Cuerpo::Tokens(_)) || !self.batallas.contains_key(&id) {
                    self.informar(Informe::Descartado { batalla: id, mensaje });
                    return;
                }
                // Lo que el proceso deje sin poner (la versión del esquema, la batalla), lo pone el lanzador.
                let b = &self.batallas[&id];
                let (intento, revision) = (b.asignacion.intento_asignacion_id.clone(), b.ticket.ticket_revision);
                match c {
                    de_srv::Cuerpo::Inicio(x) => completar(&mut x.schema_version, &mut x.battle_id, Some((&mut x.intento_asignacion_id, &mut x.ticket_revision)), &id, &intento, revision),
                    de_srv::Cuerpo::Salida(x) => completar(&mut x.schema_version, &mut x.battle_id, Some((&mut x.intento_asignacion_id, &mut x.ticket_revision)), &id, &intento, revision),
                    de_srv::Cuerpo::Resultado(x) => completar(&mut x.schema_version, &mut x.battle_id, Some((&mut x.intento_asignacion_id, &mut x.ticket_revision)), &id, &intento, revision),
                    _ => {}
                }
                self.batallas.get_mut(&id).unwrap().sin_acuse.push(m.clone());
                self.informar(Informe::Reenvia { batalla: id, mensaje });
                self.al_motor(m);
            }
            DelProceso::Terminado(motivo) => {
                let Some(b) = self.batallas.remove(&id) else { return };
                self.informar(Informe::Terminada { batalla: id, con_resultado: b.con_resultado, motivo });
            }
        }
    }

    fn evento(&mut self, e: Evento) {
        match e {
            Evento::MotorConectado(tx) => {
                self.motor = Some(tx);
                self.informar(Informe::Conectado);
                // Lo que se quedó sin acuse se repite: el núcleo contesta «sin cambios» a lo que ya había aplicado.
                let pendientes: Vec<_> = self.batallas.values().flat_map(|b| b.sin_acuse.clone()).collect();
                for m in pendientes {
                    self.al_motor(m);
                }
            }
            Evento::MotorCaido(motivo) => {
                self.motor = None;
                self.informar(Informe::Desconectado(motivo));
            }
            Evento::Motor(c) => self.del_motor(*c),
            Evento::Proceso { batalla, que } => self.del_proceso(batalla, que),
        }
    }
}

fn completar(esquema: &mut u32, batalla: &mut String, intento: Option<(&mut String, &mut u32)>, id: &str, vigente: &str, revision: u32) {
    if *esquema == 0 {
        *esquema = SCHEMA_VERSION;
    }
    if batalla.is_empty() {
        *batalla = id.to_owned();
    }
    if let Some((i, r)) = intento {
        if i.is_empty() {
            *i = vigente.to_owned();
            *r = revision;
        }
    }
}

/// Arranca el proceso de una batalla y le atiende su socket hasta que termina (o hasta que el lanzador lo mata).
async fn proceso(config: Config, id: String, puerto: u16, eventos: mpsc::UnboundedSender<Evento>, mut muerte: oneshot::Receiver<()>) {
    let avisar = |que| {
        let _ = eventos.send(Evento::Proceso { batalla: id.clone(), que });
    };
    let socket = match TcpListener::bind("127.0.0.1:0").await {
        Ok(s) => s,
        Err(e) => return avisar(DelProceso::Terminado(format!("sin socket local: {e}"))),
    };
    let local = socket.local_addr().map(|a| a.to_string()).unwrap_or_default();
    let mut orden = tokio::process::Command::new(&config.proceso);
    orden.args(&config.argumentos).args(["--batalla", &id, "--lanzador", &local, "--puerto", &puerto.to_string()]).stdin(Stdio::null()).kill_on_drop(true);
    let mut hijo = match orden.spawn() {
        Ok(h) => h,
        Err(e) => return avisar(DelProceso::Terminado(format!("no arranca {}: {e}", config.proceso.display()))),
    };
    avisar(DelProceso::Arrancado);
    let conexion = tokio::select! {
        c = tokio::time::timeout(config.espera_proceso, socket.accept()) => c,
        s = hijo.wait() => return avisar(DelProceso::Terminado(format!("salió sin conectarse ({})", s.map(|s| s.to_string()).unwrap_or_default()))),
        _ = &mut muerte => return avisar(DelProceso::Terminado("parado por el lanzador".into())),
    };
    let tcp = match conexion {
        Ok(Ok((tcp, _))) => tcp,
        Ok(Err(e)) => return avisar(DelProceso::Terminado(format!("socket local: {e}"))),
        Err(_) => return avisar(DelProceso::Terminado(format!("no se conectó en {} s", config.espera_proceso.as_secs()))),
    };
    drop(socket);
    let (lectura, mut escritura) = tcp.into_split();
    let (tx, mut rx) = mpsc::unbounded_channel::<MensajeParaProcesoDeBatalla>();
    avisar(DelProceso::Conectado(tx));
    let escritor = tokio::spawn(async move {
        while let Some(m) = rx.recv().await {
            if marco::escribir(&mut escritura, &m).await.is_err() {
                break;
            }
        }
    });
    let mut lectura = BufReader::new(lectura);
    let motivo = loop {
        tokio::select! {
            m = marco::leer::<MensajeDeServidorDeBatalla, _>(&mut lectura) => match m {
                Ok(Some(m)) => avisar(DelProceso::Mensaje(Box::new(m))),
                Ok(None) => break "cerró su socket".to_owned(),
                Err(e) => break format!("mensaje mal formado: {e}"),
            },
            _ = &mut muerte => break "parado por el lanzador".to_owned(),
        }
    };
    escritor.abort();
    // Le da unos segundos para salir solo; si no, lo mata.
    let salida = match tokio::time::timeout(Duration::from_secs(10), hijo.wait()).await {
        Ok(s) => s.map(|s| s.to_string()).unwrap_or_default(),
        Err(_) => {
            let _ = hijo.kill().await;
            "matado".into()
        }
    };
    avisar(DelProceso::Terminado(format!("{motivo}; {salida}")));
}

/// Corre el lanzador hasta que `parar` termine: mantiene la sesión con el motor y atiende las batallas. `informes` recibe lo que va pasando.
pub async fn correr(config: Config, informes: Option<mpsc::UnboundedSender<Informe>>, parar: impl Future<Output = ()>) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let sesion = tokio::spawn(motor::mantener(config.motor.clone(), config.nick.clone(), config.clave.clone(), config.codigo.clone(), config.espera_reconexion, tx.clone()));
    let mut l = Lanzador { config, eventos: tx, informes, motor: None, batallas: BTreeMap::new() };
    tokio::pin!(parar);
    loop {
        tokio::select! {
            Some(e) = rx.recv() => l.evento(e),
            _ = &mut parar => break,
        }
    }
    sesion.abort();
    for b in l.batallas.into_values() {
        if let Some(m) = b.matar {
            let _ = m.send(());
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn los_puertos_se_leen_por_rangos_y_sueltos() {
        assert_eq!(puertos("7777-7779, 7790,7778").unwrap(), vec![7777, 7778, 7779, 7790]);
        assert!(puertos("").is_err());
        assert!(puertos("7779-7777").is_err());
        assert!(puertos("siete").is_err());
        assert!(puertos("70000").is_err());
    }

    #[test]
    fn los_humanos_del_ticket_son_los_participantes_que_no_son_bot() {
        let p = |h: &str, bot| pb::BattleParticipantSnapshot { heroe_id: h.into(), es_bot: bot, ..Default::default() };
        let t = pb::BattleTicket {
            atacante: Some(pb::BattleSide { participantes: vec![p("heroe-1", false), p("heroe-2", true)], ..Default::default() }),
            defensor: Some(pb::BattleSide { participantes: vec![p("heroe-3", false)], ..Default::default() }),
            ..Default::default()
        };
        assert_eq!(humanos(&t), vec!["heroe-1", "heroe-3"]);
        assert_eq!(azar(32).len(), 64);
        assert_ne!(azar(8), azar(8));
    }
}
