//! El servidor WebSocket: acepta conexiones, hace el handshake y mueve mensajes entre el cliente y el núcleo.
//! Una tarea por sesión (`sesion`) que es la única que numera y escribe en el socket.

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use contrato::v1::{
    mensaje_de_cliente as mc, mensaje_de_servidor as ms, Bienvenida, ErrorSesion, MensajeDeCliente, MensajeDeServidor,
    Pong, Aviso, TipoCliente,
};
use futures_util::{stream::SplitSink, stream::SplitStream, SinkExt, StreamExt};
use prost::Message as _;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, watch, Notify},
    task::{JoinHandle, JoinSet},
    time::timeout,
};
use tokio_tungstenite::{
    accept_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
    WebSocketStream,
};

use crate::http::{self, Http, Inicio, Rebobinado, SinHttp};
use crate::nucleo::{
    codigos, Autenticador, ComandoSesion, Config, Difusion, ErrorAcceso, RelojCompartido, SesionId,
};

/// `Hola.version_protocolo` que habla este servidor.
pub const VERSION_PROTOCOLO: u32 = 1;
const FIRMA_ZSTD: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];
/// Máximo de mensajes del núcleo que se agrupan en un solo vaciado del socket.
const LOTE: usize = 64;

/// Servidor en marcha. Al soltarlo (o con [`Servidor::detener`]) deja de aceptar y las sesiones cierran con `servidor.reiniciando`.
pub struct Servidor {
    pub direccion: SocketAddr,
    parar: watch::Sender<bool>,
    tarea: JoinHandle<()>,
}

impl Servidor {
    /// Para de aceptar, avisa a las sesiones y espera (hasta 5 s) a que terminen.
    pub async fn detener(self) {
        let _ = self.parar.send(true);
        let _ = self.tarea.await;
    }
}

/// Arranca el servidor sobre `listener`. `nucleo` es el canal de comandos del núcleo; `reloj` el reloj de mundo que se sirve en `Pong`. Sin HTTP más que `GET /salud`.
pub async fn servir<A: Autenticador>(
    listener: TcpListener,
    nucleo: mpsc::Sender<ComandoSesion>,
    autenticador: A,
    reloj: RelojCompartido,
    config: Config,
) -> Servidor {
    servir_con_http(listener, nucleo, autenticador, Arc::new(SinHttp), reloj, config).await
}

/// Como [`servir`], con `http` atendiendo las peticiones HTTP del mismo puerto (el servicio de acceso, 7B).
pub async fn servir_con_http<A: Autenticador>(
    listener: TcpListener,
    nucleo: mpsc::Sender<ComandoSesion>,
    autenticador: A,
    http: Arc<dyn Http>,
    reloj: RelojCompartido,
    config: Config,
) -> Servidor {
    let direccion = listener.local_addr().expect("dirección local");
    let (parar, parada) = watch::channel(false);
    let ctx = Arc::new(Ctx {
        nucleo,
        autenticador,
        http,
        reloj,
        config,
        registro: Mutex::new(HashMap::new()),
        siguiente_id: AtomicU64::new(1),
        parada,
    });
    let tarea = tokio::spawn(aceptar(listener, ctx));
    Servidor { direccion, parar, tarea }
}

struct Ctx<A> {
    nucleo: mpsc::Sender<ComandoSesion>,
    autenticador: A,
    http: Arc<dyn Http>,
    reloj: RelojCompartido,
    config: Config,
    /// Sesión viva por `cliente_id`.
    registro: Mutex<HashMap<String, Viva>>,
    siguiente_id: AtomicU64,
    parada: watch::Receiver<bool>,
}

struct Viva {
    id: SesionId,
    expulsar: Arc<Notify>,
    fin: watch::Receiver<bool>,
}

async fn aceptar<A: Autenticador>(listener: TcpListener, ctx: Arc<Ctx<A>>) {
    let mut sesiones = JoinSet::new();
    let mut parada = ctx.parada.clone();
    loop {
        tokio::select! {
            _ = parada.changed() => break,
            r = listener.accept() => {
                while sesiones.try_join_next().is_some() {}
                if let Ok((tcp, _)) = r {
                    let _ = tcp.set_nodelay(true);
                    sesiones.spawn(sesion(ctx.clone(), tcp));
                }
            }
        }
    }
    let _ = timeout(Duration::from_secs(5), async { while sesiones.join_next().await.is_some() {} }).await;
}

// ───────────────────────────── Códec ─────────────────────────────

fn error(codigo: &str, mensaje: &str, fatal: bool) -> ErrorSesion {
    ErrorSesion { codigo: codigo.into(), mensaje: mensaje.into(), fatal }
}

/// Decodifica una trama del cliente (descomprimiendo si lleva la firma zstd).
fn decodificar(bytes: &[u8], max: usize) -> Option<MensajeDeCliente> {
    if bytes.starts_with(&FIRMA_ZSTD) {
        let plano = zstd::bulk::decompress(bytes, max).ok()?;
        MensajeDeCliente::decode(plano.as_slice()).ok()
    } else if bytes.len() <= max {
        MensajeDeCliente::decode(bytes).ok()
    } else {
        None
    }
}

#[allow(clippy::large_enum_variant)]
enum Entrada {
    Mensaje(MensajeDeCliente),
    Cierre,
    Invalido,
}

struct Receptor {
    stream: SplitStream<WebSocketStream<Rebobinado>>,
    max: usize,
}

impl Receptor {
    /// Siguiente mensaje del cliente. Cancel-safe: solo espera en `stream.next()`.
    async fn siguiente(&mut self) -> Entrada {
        loop {
            match self.stream.next().await {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return Entrada::Cierre,
                Some(Ok(Message::Binary(b))) => {
                    return decodificar(&b, self.max).map_or(Entrada::Invalido, Entrada::Mensaje)
                }
                Some(Ok(Message::Text(_))) => return Entrada::Invalido,
                Some(Ok(_)) => {} // ping/pong del WebSocket: tungstenite contesta solo
            }
        }
    }
}

struct Emisor {
    sink: SplitSink<WebSocketStream<Rebobinado>, Message>,
    secuencia: u64,
    comprimir: bool,
    escritura: Duration,
    umbral: usize,
    nivel: i32,
}

impl Emisor {
    /// Numera y encola un mensaje. Es el único punto donde nace una `secuencia`.
    async fn encolar(&mut self, cuerpo: ms::Cuerpo) -> bool {
        self.secuencia += 1;
        let mut bytes = MensajeDeServidor { secuencia: self.secuencia, cuerpo: Some(cuerpo) }.encode_to_vec();
        if self.comprimir && bytes.len() >= self.umbral {
            if let Ok(c) = zstd::bulk::compress(&bytes, self.nivel) {
                if c.len() < bytes.len() {
                    bytes = c;
                }
            }
        }
        matches!(timeout(self.escritura, self.sink.feed(Message::Binary(bytes.into()))).await, Ok(Ok(())))
    }
    async fn vaciar(&mut self) -> bool {
        matches!(timeout(self.escritura, self.sink.flush()).await, Ok(Ok(())))
    }
    async fn enviar(&mut self, cuerpo: ms::Cuerpo) -> bool {
        self.encolar(cuerpo).await && self.vaciar().await
    }
    async fn error(&mut self, e: ErrorSesion) -> bool {
        self.enviar(ms::Cuerpo::Error(e)).await
    }
    async fn cerrar(&mut self) {
        let _ = timeout(self.escritura, self.sink.close()).await;
    }
}

// ───────────────────────────── Sesión ─────────────────────────────

/// Lo que hay que deshacer al terminar la sesión, pase lo que pase.
#[derive(Default)]
struct Limpieza {
    cliente_id: Option<String>,
    conectada: bool,
}

struct Abierta {
    difusion: mpsc::Receiver<Difusion>,
    rol: TipoCliente,
}

async fn sesion<A: Autenticador>(ctx: Arc<Ctx<A>>, tcp: TcpStream) {
    let cfg = WebSocketConfig::default()
        .max_message_size(Some(ctx.config.max_trama_cliente))
        .max_frame_size(Some(ctx.config.max_trama_cliente));
    let flujo = match timeout(ctx.config.timeout_hola, http::leer_inicio(tcp)).await {
        Ok(Inicio::WebSocket(f)) => f,
        Ok(Inicio::Http(tcp, p)) => return http::contestar(tcp, p, ctx.http.as_ref()).await,
        Ok(Inicio::Nada) | Err(_) => return,
    };
    let Ok(Ok(ws)) = timeout(ctx.config.timeout_hola, accept_async_with_config(flujo, Some(cfg))).await else {
        return;
    };
    let (sink, stream) = ws.split();
    let mut em = Emisor {
        sink,
        secuencia: 0,
        comprimir: false,
        escritura: ctx.config.timeout_escritura,
        umbral: ctx.config.umbral_compresion,
        nivel: ctx.config.nivel_zstd,
    };
    let mut rx = Receptor { stream, max: ctx.config.max_trama_cliente };
    let id = ctx.siguiente_id.fetch_add(1, Ordering::Relaxed);
    let (fin_tx, fin_rx) = watch::channel(false);
    let expulsar = Arc::new(Notify::new());
    let mut limpieza = Limpieza::default();

    if let Some(abierta) = handshake(&ctx, &mut em, &mut rx, id, &expulsar, fin_rx, &mut limpieza).await {
        bucle(&ctx, &mut em, &mut rx, id, &expulsar, abierta).await;
    }

    em.cerrar().await;
    if limpieza.conectada {
        let _ = ctx.nucleo.send(ComandoSesion::Desconecta { sesion: id }).await;
    }
    if let Some(c) = limpieza.cliente_id {
        let mut reg = ctx.registro.lock().unwrap_or_else(|e| e.into_inner());
        if reg.get(&c).is_some_and(|v| v.id == id) {
            reg.remove(&c);
        }
    }
    fin_tx.send_replace(true);
}

/// Hola → autenticación → registro único → `Conecta` → `Bienvenida`. Los fallos se comunican al cliente aquí.
async fn handshake<A: Autenticador>(
    ctx: &Ctx<A>,
    em: &mut Emisor,
    rx: &mut Receptor,
    id: SesionId,
    expulsar: &Arc<Notify>,
    fin: watch::Receiver<bool>,
    limpieza: &mut Limpieza,
) -> Option<Abierta> {
    let hola = match timeout(ctx.config.timeout_hola, rx.siguiente()).await {
        Ok(Entrada::Mensaje(MensajeDeCliente { cuerpo: Some(mc::Cuerpo::Hola(h)) })) => h,
        Ok(Entrada::Mensaje(_)) => {
            em.error(error(codigos::HOLA_ESPERADO, "el primer mensaje debe ser Hola", true)).await;
            return None;
        }
        Ok(Entrada::Invalido) => {
            em.error(error(codigos::MENSAJE_INVALIDO, "trama no decodificable", true)).await;
            return None;
        }
        Ok(Entrada::Cierre) | Err(_) => return None,
    };

    if hola.version_protocolo != VERSION_PROTOCOLO {
        let msg = format!("protocolo {} no soportado; este servidor habla {VERSION_PROTOCOLO}", hola.version_protocolo);
        em.error(error(codigos::VERSION_PROTOCOLO, &msg, true)).await;
        return None;
    }
    let identidad = match ctx.autenticador.autenticar(&hola.token).await {
        Ok(i) => i,
        Err(ErrorAcceso::Invalido) => {
            em.error(error(codigos::TOKEN_INVALIDO, "token inválido o caducado", true)).await;
            return None;
        }
        Err(ErrorAcceso::NoDisponible) => {
            em.error(error(codigos::INTERNO, "servicio de acceso no disponible", true)).await;
            return None;
        }
    };
    let rol = hola.cliente();
    if rol == TipoCliente::NoEspecificado || !identidad.roles.contains(&rol) {
        em.error(error(codigos::ROL_NO_PERMITIDO, "el token no permite este tipo de cliente", true)).await;
        return None;
    }

    // Una sola sesión por cliente: la nueva sustituye a la vieja y espera a que esta termine (el núcleo verá Desconecta antes que Conecta).
    let viejo = {
        let mut reg = ctx.registro.lock().unwrap_or_else(|e| e.into_inner());
        reg.insert(identidad.cliente_id.clone(), Viva { id, expulsar: expulsar.clone(), fin })
    };
    limpieza.cliente_id = Some(identidad.cliente_id.clone());
    if let Some(v) = viejo {
        v.expulsar.notify_one();
        let mut fin = v.fin;
        let _ = timeout(Duration::from_secs(5), fin.wait_for(|f| *f)).await;
    }

    let zstd = hola.compresiones.iter().any(|c| c == "zstd");
    let (difusion_tx, difusion_rx) = mpsc::channel(ctx.config.capacidad_difusion);
    let (respuesta, esperar) = tokio::sync::oneshot::channel();
    limpieza.conectada = true;
    let cmd = ComandoSesion::Conecta { sesion: id, identidad, hola, difusion: difusion_tx, respuesta };
    if ctx.nucleo.send(cmd).await.is_err() {
        em.error(error(codigos::INTERNO, "núcleo no disponible", true)).await;
        return None;
    }
    let mut bienvenida: Bienvenida = match timeout(ctx.config.timeout_hola, esperar).await {
        Ok(Ok(Ok(b))) => b,
        Ok(Ok(Err(e))) => {
            em.error(ErrorSesion { fatal: true, ..e }).await;
            return None;
        }
        _ => {
            em.error(error(codigos::INTERNO, "el núcleo no respondió", true)).await;
            return None;
        }
    };
    bienvenida.maximo_ordenes_pendientes = ctx.config.max_ordenes_pendientes;
    if !em.enviar(ms::Cuerpo::Bienvenida(bienvenida)).await {
        return None;
    }
    em.comprimir = zstd; // la Bienvenida va siempre sin comprimir
    Some(Abierta { difusion: difusion_rx, rol })
}

async fn bucle<A: Autenticador>(
    ctx: &Ctx<A>,
    em: &mut Emisor,
    rx: &mut Receptor,
    id: SesionId,
    expulsar: &Notify,
    mut abierta: Abierta,
) {
    let mut parada = ctx.parada.clone();
    let mut pendientes: u32 = 0;
    loop {
        tokio::select! {
            _ = parada.changed() => {
                aviso(em, "servidor.reiniciando", "el servidor se detiene").await;
                return;
            }
            _ = expulsar.notified() => {
                aviso(em, "sesion.reemplazada", "otra sesión del mismo cliente ha tomado el relevo").await;
                return;
            }
            d = abierta.difusion.recv() => {
                // Reenvío en el orden de llegada (causalidad), agrupando lo que ya esté listo en un solo vaciado.
                let Some(mut d) = d else {
                    em.error(error(codigos::INTERNO, "el núcleo cerró la sesión", true)).await;
                    return;
                };
                let mut n = 0;
                loop {
                    let (cuerpo, fatal) = match d {
                        Difusion::Foto(x) => (ms::Cuerpo::Foto(x), false),
                        Difusion::Delta(x) => (ms::Cuerpo::Delta(x), false),
                        Difusion::Resultado(x) => {
                            pendientes = pendientes.saturating_sub(1);
                            (ms::Cuerpo::Resultado(x), false)
                        }
                        Difusion::Revocado(x) => (ms::Cuerpo::Revocado(x), false),
                        Difusion::Aviso(x) => (ms::Cuerpo::Aviso(x), false),
                        Difusion::Batalla(x) => (ms::Cuerpo::Batalla(x), false),
                        Difusion::Cronica(x) => {
                            pendientes = pendientes.saturating_sub(1);
                            (ms::Cuerpo::Cronica(x), false)
                        }
                        Difusion::Cerrar(e) => {
                            let f = e.fatal;
                            (ms::Cuerpo::Error(e), f)
                        }
                    };
                    if !em.encolar(cuerpo).await {
                        return;
                    }
                    if fatal {
                        em.vaciar().await;
                        return;
                    }
                    n += 1;
                    if n >= LOTE {
                        break;
                    }
                    match abierta.difusion.try_recv() {
                        Ok(x) => d = x,
                        Err(_) => break,
                    }
                }
                if !em.vaciar().await {
                    return;
                }
            }
            e = rx.siguiente() => {
                let m = match e {
                    Entrada::Mensaje(m) => m,
                    Entrada::Cierre => return,
                    Entrada::Invalido => {
                        em.error(error(codigos::MENSAJE_INVALIDO, "trama no decodificable", true)).await;
                        return;
                    }
                };
                let cmd = match m.cuerpo {
                    Some(mc::Cuerpo::Ping(p)) => {
                        let pong = Pong {
                            cliente_envio_ms: p.cliente_envio_ms,
                            servidor_recibio_ms: ctx.reloj.pared_ms(),
                            reloj: Some(ctx.reloj.reloj()),
                        };
                        if !em.enviar(ms::Cuerpo::Pong(pong)).await {
                            return;
                        }
                        continue;
                    }
                    Some(mc::Cuerpo::Orden(orden)) => {
                        let Ok(admitida) = admitir(em, &mut pendientes, ctx.config.max_ordenes_pendientes, &orden.peticion_id).await else { return };
                        if !admitida {
                            continue;
                        }
                        ComandoSesion::Orden { sesion: id, orden }
                    }
                    Some(mc::Cuerpo::PedirCronica(pedir)) => {
                        let Ok(admitida) = admitir(em, &mut pendientes, ctx.config.max_ordenes_pendientes, &pedir.peticion_id).await else { return };
                        if !admitida {
                            continue;
                        }
                        ComandoSesion::Cronica { sesion: id, pedir }
                    }
                    Some(mc::Cuerpo::Vista(v)) => match v.rectangulo {
                        Some(rect) => ComandoSesion::Vista { sesion: id, rect },
                        None => continue,
                    },
                    Some(mc::Cuerpo::Hola(_)) => {
                        // Reanudar es reconectar: un segundo Hola no se pasa al núcleo (contrato 05 §7).
                        if !em.error(error(codigos::HOLA_REPETIDO, "Hola solo puede ser el primer mensaje; para reanudar, reconecta", false)).await {
                            return;
                        }
                        continue;
                    }
                    Some(mc::Cuerpo::Batalla(mensaje)) => {
                        if abierta.rol != TipoCliente::ClienteServidorBatalla {
                            if !em.error(error(codigos::ROL_NO_PERMITIDO, "solo un servidor de batalla envía mensajes de batalla", false)).await {
                                return;
                            }
                            continue;
                        }
                        ComandoSesion::Batalla { sesion: id, mensaje }
                    }
                    None => {
                        em.error(error(codigos::MENSAJE_INVALIDO, "mensaje vacío", true)).await;
                        return;
                    }
                };
                if ctx.nucleo.send(cmd).await.is_err() {
                    em.error(error(codigos::INTERNO, "núcleo no disponible", true)).await;
                    return;
                }
            }
        }
    }
}

/// Una orden (o un `PedirCronica`) más, si cabe en el límite de pendientes: `Ok(false)` si no cabe (ya se contestó `demasiadas_ordenes`), `Err` si el socket falló.
async fn admitir(em: &mut Emisor, pendientes: &mut u32, maximo: u32, peticion_id: &str) -> Result<bool, ()> {
    if *pendientes >= maximo {
        let msg = format!("más de {maximo} órdenes pendientes; la orden {peticion_id} no se ejecuta");
        return if em.error(error(codigos::DEMASIADAS_ORDENES, &msg, false)).await { Ok(false) } else { Err(()) };
    }
    *pendientes += 1;
    Ok(true)
}

async fn aviso(em: &mut Emisor, codigo: &str, mensaje: &str) {
    let a = Aviso { codigo: codigo.into(), mensaje: mensaje.into(), reloj: None };
    em.enviar(ms::Cuerpo::Aviso(a)).await;
}
