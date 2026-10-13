//! Piezas para probar el servidor sin el núcleo real: autenticador, núcleo falso, cliente WebSocket y medición de rendimiento.
//! Se queda en la librería (no solo en `tests/`) para que el ejemplo `rendimiento` y los bots puedan reutilizarlo.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use contrato::v1::{
    entidad, mensaje_de_cliente as mc, mensaje_de_servidor as ms, AmbitoTipo, AmbitoSuscrito, Ambito, Bienvenida,
    Cambio, DecisionReanudacion, Delta, Entidad, ErrorSesion, Foto, Heroe, Hola, InfoMundo, MensajeDeCliente,
    MensajeDeServidor, Orden, Ping, RechazoOrden, Resultado, ResultadoOrden, TipoCambio, TipoCliente, TipoEntidad,
};
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;
use tokio::{net::TcpStream, sync::mpsc, time::timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::nucleo::{
    Autenticador, ComandoSesion, Difusion, ErrorAcceso, Identidad, RelojCompartido, SesionId,
};

// ───────────────────────────── Autenticador ─────────────────────────────

/// Tokens: `ok:<cliente>` (Unity, admin web y bot), `bot:<cliente>` (solo bot), `svc:<cliente>` (solo servidor de batalla),
/// `caido` (servicio de acceso caído); cualquier otro es inválido.
pub struct AutenticadorDePrueba;

impl Autenticador for AutenticadorDePrueba {
    async fn autenticar(&self, token: &str) -> Result<Identidad, ErrorAcceso> {
        let (prefijo, cliente) = token.split_once(':').unwrap_or((token, ""));
        let roles = match prefijo {
            "ok" => vec![TipoCliente::ClienteUnity, TipoCliente::ClienteAdminWeb, TipoCliente::ClienteBot],
            "bot" => vec![TipoCliente::ClienteBot],
            "svc" => vec![TipoCliente::ClienteServidorBatalla],
            "caido" => return Err(ErrorAcceso::NoDisponible),
            _ => return Err(ErrorAcceso::Invalido),
        };
        if cliente.is_empty() {
            return Err(ErrorAcceso::Invalido);
        }
        Ok(Identidad { cliente_id: cliente.into(), roles })
    }
}

// ───────────────────────────── Núcleo falso ─────────────────────────────

struct SesionFalsa {
    cliente: String,
    difusion: mpsc::Sender<Difusion>,
}

#[derive(Default)]
struct Estado {
    version: u64,
    sesiones: HashMap<SesionId, SesionFalsa>,
    vistos: HashSet<String>,
    ordenes: u64,
}

/// Núcleo de mentira: un único ámbito `HEROE:<cliente>` y una versión de mundo global.
/// - `Conecta` → `Bienvenida` + `Foto` (o `Delta`/nada si `versiones_conocidas` lo permite: hasta 8 versiones de retraso hay deltas).
/// - `Orden` → `Delta` y luego `ResultadoOrden` aceptada (versión +1). Reglas por `peticion_id`: `rechaza…` se rechaza,
///   `lenta…` nunca se contesta (para probar el límite de pendientes), una repetida es `ORDEN_DUPLICADA`.
/// - `Vista` → una `Foto` del ámbito VISTA. `partida_id == "inexistente"` → `partida_no_disponible`.
#[derive(Clone)]
pub struct NucleoFalso {
    pub comandos: mpsc::Sender<ComandoSesion>,
    estado: Arc<Mutex<Estado>>,
}

fn ambito_heroe(cliente: &str) -> Ambito {
    Ambito { tipo: AmbitoTipo::AmbitoHeroe as i32, id: cliente.into() }
}

fn heroe(cliente: &str, version: u64) -> Entidad {
    Entidad { cuerpo: Some(entidad::Cuerpo::Heroe(Heroe { id: cliente.into(), version, ..Default::default() })) }
}

fn delta(cliente: &str, desde: u64, hasta: u64) -> Delta {
    Delta {
        ambito: Some(ambito_heroe(cliente)),
        desde,
        hasta,
        cambios: vec![Cambio {
            tipo: TipoCambio::CambioActualiza as i32,
            entidad: TipoEntidad::Heroe as i32,
            id: cliente.into(),
            datos: Some(heroe(cliente, hasta)),
            campos_borrados: vec![],
        }],
        ..Default::default()
    }
}

/// Decide la reanudación del ámbito del héroe y deposita lo que corresponda en `tx`.
fn reanudar(version: u64, cliente: &str, conocidas: &BTreeMap<String, u64>, tx: &mpsc::Sender<Difusion>) -> AmbitoSuscrito {
    let decision = match conocidas.get(&format!("HEROE:{cliente}")) {
        Some(&k) if k == version => DecisionReanudacion::ReanudaSinNovedad,
        Some(&k) if k < version && version - k <= 8 => {
            let _ = tx.try_send(Difusion::Delta(delta(cliente, k, version)));
            DecisionReanudacion::ReanudaConDeltas
        }
        _ => {
            let foto = Foto { ambito: Some(ambito_heroe(cliente)), version, entidades: vec![heroe(cliente, version)], niebla: None };
            let _ = tx.try_send(Difusion::Foto(foto));
            DecisionReanudacion::ReanudaConFoto
        }
    };
    AmbitoSuscrito { ambito: Some(ambito_heroe(cliente)), version_actual: version, decision: decision as i32 }
}

impl NucleoFalso {
    pub fn arrancar(reloj: RelojCompartido) -> Self {
        let (comandos, mut rx) = mpsc::channel(8192);
        let estado = Arc::new(Mutex::new(Estado::default()));
        let est = estado.clone();
        tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                let mut e = est.lock().unwrap();
                match cmd {
                    ComandoSesion::Conecta { sesion, identidad, hola, difusion, respuesta } => {
                        if hola.partida_id == "inexistente" {
                            let err = ErrorSesion { codigo: "partida_no_disponible".into(), mensaje: "no existe".into(), fatal: true };
                            let _ = respuesta.send(Err(err));
                            continue;
                        }
                        let amb = reanudar(e.version, &identidad.cliente_id, &hola.versiones_conocidas, &difusion);
                        let b = Bienvenida {
                            sesion_id: format!("s{sesion}"),
                            heroe_id: Some(identidad.cliente_id.clone()),
                            mundo: Some(InfoMundo { hash: "hash-falso".into(), ..Default::default() }),
                            reloj: Some(reloj.reloj()),
                            ambitos: vec![amb],
                            ..Default::default()
                        };
                        e.sesiones.insert(sesion, SesionFalsa { cliente: identidad.cliente_id, difusion });
                        let _ = respuesta.send(Ok(b));
                    }
                    ComandoSesion::Orden { sesion, orden } => {
                        e.ordenes += 1;
                        let id = orden.peticion_id.clone();
                        if id.starts_with("lenta") {
                            continue;
                        }
                        let Some(s) = e.sesiones.get(&sesion) else { continue };
                        let (cliente, tx) = (s.cliente.clone(), s.difusion.clone());
                        let resultado = if !e.vistos.insert(id.clone()) {
                            ResultadoOrden {
                                peticion_id: id,
                                resultado: Resultado::OrdenDuplicada as i32,
                                version_tras: e.version,
                                ..Default::default()
                            }
                        } else if id.starts_with("rechaza") {
                            ResultadoOrden {
                                peticion_id: id,
                                resultado: Resultado::OrdenRechazada as i32,
                                version_tras: e.version,
                                rechazo: Some(RechazoOrden { codigo: "no_autorizado".into(), mensaje: "falso".into(), detalle: None }),
                                ..Default::default()
                            }
                        } else {
                            e.version += 1;
                            // Causalidad: la Delta se deposita ANTES que el resultado que la produjo.
                            if tx.try_send(Difusion::Delta(delta(&cliente, e.version - 1, e.version))).is_err() {
                                e.sesiones.remove(&sesion);
                                continue;
                            }
                            ResultadoOrden {
                                peticion_id: id,
                                resultado: Resultado::OrdenAceptada as i32,
                                version_tras: e.version,
                                ..Default::default()
                            }
                        };
                        if tx.try_send(Difusion::Resultado(resultado)).is_err() {
                            e.sesiones.remove(&sesion);
                        }
                    }
                    ComandoSesion::Vista { sesion, .. } => {
                        if let Some(s) = e.sesiones.get(&sesion) {
                            let ambito = Ambito { tipo: AmbitoTipo::AmbitoVista as i32, id: String::new() };
                            let foto = Foto { ambito: Some(ambito), version: e.version, ..Default::default() };
                            let _ = s.difusion.try_send(Difusion::Foto(foto));
                        }
                    }
                    ComandoSesion::Batalla { .. } => {}
                    ComandoSesion::Desconecta { sesion } => {
                        e.sesiones.remove(&sesion);
                    }
                }
            }
        });
        Self { comandos, estado }
    }

    pub fn version(&self) -> u64 {
        self.estado.lock().unwrap().version
    }
    pub fn sesiones_activas(&self) -> usize {
        self.estado.lock().unwrap().sesiones.len()
    }
    pub fn ordenes_recibidas(&self) -> u64 {
        self.estado.lock().unwrap().ordenes
    }
    /// Sube la versión del mundo y manda la `Delta` correspondiente a todas las sesiones (haya o no alguien conectado).
    pub fn difundir_delta(&self) -> u64 {
        let mut e = self.estado.lock().unwrap();
        e.version += 1;
        let v = e.version;
        e.sesiones.retain(|_, s| s.difusion.try_send(Difusion::Delta(delta(&s.cliente, v - 1, v))).is_ok());
        v
    }
}

// ───────────────────────────── Cliente de prueba ─────────────────────────────

pub fn hola(token: &str, cliente: TipoCliente) -> Hola {
    Hola {
        version_protocolo: 1,
        cliente: cliente as i32,
        version_cliente: "prueba".into(),
        token: token.into(),
        partida_id: "p1".into(),
        ..Default::default()
    }
}

/// Cliente WebSocket de pruebas. `recibir` comprueba que la `secuencia` crece de uno en uno desde 1 (el invariante sin huecos).
pub struct ClientePrueba {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ultima: u64,
    comprimir: bool,
    /// Cuántas tramas recibidas venían comprimidas con zstd.
    pub tramas_zstd: u32,
}

impl ClientePrueba {
    pub async fn conectar(dir: SocketAddr) -> Self {
        let (ws, _) = connect_async(format!("ws://{dir}")).await.expect("conectar");
        Self { ws, ultima: 0, comprimir: false, tramas_zstd: 0 }
    }

    /// Conecta, manda el `Hola` y devuelve la `Bienvenida` (los tests de error usan `conectar` + `enviar`).
    pub async fn entrar(dir: SocketAddr, hola: Hola) -> (Self, Bienvenida) {
        let mut c = Self::conectar(dir).await;
        c.comprimir = hola.compresiones.iter().any(|x| x == "zstd");
        c.enviar(mc::Cuerpo::Hola(hola)).await;
        let Some(ms::Cuerpo::Bienvenida(b)) = c.esperar().await.cuerpo else { panic!("se esperaba Bienvenida") };
        (c, b)
    }

    pub async fn enviar(&mut self, cuerpo: mc::Cuerpo) {
        let mut bytes = MensajeDeCliente { cuerpo: Some(cuerpo) }.encode_to_vec();
        let es_hola = matches!(MensajeDeCliente::decode(bytes.as_slice()).ok().and_then(|m| m.cuerpo), Some(mc::Cuerpo::Hola(_)));
        if self.comprimir && !es_hola {
            bytes = zstd::bulk::compress(&bytes, 3).unwrap();
        }
        self.ws.send(Message::Binary(bytes.into())).await.expect("enviar");
    }

    /// Una trama vacía: decodifica como `MensajeDeCliente` sin cuerpo (inválido).
    pub async fn enviar_vacio(&mut self) {
        self.ws.send(Message::Binary(Vec::new().into())).await.expect("enviar");
    }

    pub async fn orden(&mut self, peticion_id: &str) {
        self.enviar(mc::Cuerpo::Orden(Orden { peticion_id: peticion_id.into(), ..Default::default() })).await;
    }

    pub async fn ping(&mut self, cliente_envio_ms: i64) {
        self.enviar(mc::Cuerpo::Ping(Ping { cliente_envio_ms })).await;
    }

    /// Siguiente mensaje del servidor, o `None` si cerró. Falla la prueba si pasan 5 s o si hay un hueco de secuencia.
    pub async fn recibir(&mut self) -> Option<MensajeDeServidor> {
        loop {
            let m = timeout(Duration::from_secs(5), self.ws.next()).await.expect("tiempo agotado esperando al servidor");
            match m {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return None,
                Some(Ok(Message::Binary(b))) => {
                    let msg = if b.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
                        self.tramas_zstd += 1;
                        MensajeDeServidor::decode(zstd::bulk::decompress(&b, 64 << 20).unwrap().as_slice())
                    } else {
                        MensajeDeServidor::decode(&b[..])
                    }
                    .expect("decodificar");
                    assert_eq!(msg.secuencia, self.ultima + 1, "hueco de secuencia");
                    self.ultima = msg.secuencia;
                    return Some(msg);
                }
                Some(Ok(_)) => {}
            }
        }
    }

    pub async fn esperar(&mut self) -> MensajeDeServidor {
        self.recibir().await.expect("el servidor cerró la sesión")
    }

    pub async fn cerrar(mut self) {
        let _ = self.ws.close(None).await;
    }
}

// ───────────────────────────── Rendimiento ─────────────────────────────

#[derive(Debug)]
pub struct Medicion {
    pub sesiones: usize,
    pub ordenes: u64,
    /// Mensajes del servidor recibidos por los clientes (Delta + Resultado por orden).
    pub mensajes: u64,
    pub segundos: f64,
}

impl Medicion {
    pub fn mensajes_por_s(&self) -> f64 {
        self.mensajes as f64 / self.segundos
    }
}

/// `sesiones` clientes simultáneos, cada uno manda `ordenes` órdenes en ráfagas de 16 y lee Delta+Resultado de cada una
/// (verificando el orden causal y la secuencia). Levanta su propio servidor y núcleo falso en `127.0.0.1`.
pub async fn medir_rendimiento(sesiones: usize, ordenes: usize) -> Medicion {
    use crate::{servir, Config};
    let reloj = RelojCompartido::nuevo(0);
    let nucleo = NucleoFalso::arrancar(reloj.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let config = Config { max_ordenes_pendientes: 16, ..Config::default() };
    let srv = servir(listener, nucleo.comandos.clone(), AutenticadorDePrueba, reloj, config).await;
    let dir = srv.direccion;

    let listos = Arc::new(tokio::sync::Barrier::new(sesiones + 1));
    let mut tareas = Vec::new();
    for i in 0..sesiones {
        let listos = listos.clone();
        tareas.push(tokio::spawn(async move {
            let (mut c, _) = ClientePrueba::entrar(dir, hola(&format!("ok:c{i}"), TipoCliente::ClienteBot)).await;
            c.esperar().await; // Foto
            listos.wait().await;
            let mut recibidos = 0u64;
            let mut enviadas = 0;
            while enviadas < ordenes {
                let n = (ordenes - enviadas).min(16);
                for j in 0..n {
                    c.orden(&format!("c{i}-{}", enviadas + j)).await;
                }
                for _ in 0..n {
                    let (Some(ms::Cuerpo::Delta(d)), Some(ms::Cuerpo::Resultado(r))) =
                        (c.esperar().await.cuerpo, c.esperar().await.cuerpo)
                    else {
                        panic!("se esperaba Delta y luego Resultado");
                    };
                    assert!(d.hasta <= r.version_tras);
                    recibidos += 2;
                }
                enviadas += n;
            }
            c.cerrar().await;
            recibidos
        }));
    }
    listos.wait().await;
    let t0 = Instant::now();
    let mut mensajes = 0;
    for t in tareas {
        mensajes += t.await.unwrap();
    }
    let segundos = t0.elapsed().as_secs_f64();
    srv.detener().await;
    Medicion { sesiones, ordenes: (sesiones * ordenes) as u64, mensajes, segundos }
}
