//! El canal de control del servicio de bots, para la consola de administración: el mismo protocolo que el servicio TypeScript (`src/bots/control/contrato.ts`).
//! WebSocket en `/control`, JSON por mensaje de texto; la primera acción tiene que ser `autenticar` con el token compartido (en 5 s, o se cierra). `GET /salud` responde `{fase}`.
//! El servicio arranca y para un [`Proceso`] por partida, con su registro en `bots-<partida>.json`.

use std::{collections::VecDeque, path::PathBuf, sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{broadcast, Mutex},
};
use tokio_tungstenite::tungstenite::{
    handshake::server::{ErrorResponse, Request, Response},
    http, Message,
};

use crate::{
    cerebro::Bot,
    llegadas::{Llegada, TipoPerfil},
    mercenario::Mercenario,
    puerto,
    runner::{Cerebro, Config, Control, Lugares, Modo},
    Fallo, Proceso,
};

/// Qué bots lleva este servicio. Cada población es un proceso aparte, con su puerto, su registro y su plan de llegadas: unos se arrancan y se paran sin tocar a los otros.
#[derive(Clone, Copy, PartialEq)]
pub enum Poblacion {
    Libres,
    Mercenarios,
}

impl Poblacion {
    fn prefijo(self) -> &'static str {
        match self {
            Self::Libres => "bots",
            Self::Mercenarios => "mercs",
        }
    }

    fn plan(self, semilla: u64, total: u32, dias: f64) -> Vec<Llegada> {
        match self {
            Self::Libres => Bot::plan(semilla, total, dias),
            Self::Mercenarios => Mercenario::plan(semilla, total, dias),
        }
    }
}

/// Lo que el servicio sabe de su entorno.
pub struct Ajustes {
    pub token: String,
    pub poblacion: Poblacion,
    /// `CODIGO_REGISTRO_BOTS` del entorno; si está, el panel no lo manda.
    pub codigo_bots: String,
    /// Dónde guarda los registros de cada partida.
    pub registros: PathBuf,
    /// Cada cuánto piensa un bot, en ms de mundo (el `cadaMs` del panel no lo cambia: allí es el ritmo de una vuelta, aquí no hay vueltas).
    pub cada_ms: i64,
    /// Los valores que se proponen al configurar.
    pub por_defecto: Value,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ConfigBots {
    servidor: String,
    partida: String,
    // No se guarda en disco ni vuelve en el estado.
    #[serde(default, skip_serializing)]
    codigo_registro_bots: Option<String>,
    total: u32,
    dias_llegada: f64,
    semilla: u64,
    cada_ms: i64,
    horario: String,
    #[serde(default)]
    campamentos: Vec<String>,
    #[serde(default)]
    por_campamento: u32,
}

#[derive(Default)]
struct Interno {
    fase: &'static str,
    iniciado_en: Option<i64>,
    config: Option<ConfigBots>,
    error: Option<String>,
    proceso: Option<Proceso>,
    control: Arc<Control>,
}

pub struct Servicio {
    ajustes: Ajustes,
    interno: Mutex<Interno>,
    /// Lo último que pasó, para el `historial` al conectar.
    acciones: std::sync::Mutex<VecDeque<Value>>,
    registros: std::sync::Mutex<VecDeque<Value>>,
    eventos: std::sync::Mutex<VecDeque<Value>>,
    avisos: broadcast::Sender<Value>,
}

const HISTORIAL: usize = 100;

fn ahora() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

impl Servicio {
    pub fn new(ajustes: Ajustes) -> Arc<Self> {
        let s = Arc::new(Self {
            ajustes,
            interno: Mutex::new(Interno { fase: "inactivo", ..Default::default() }),
            acciones: Default::default(),
            registros: Default::default(),
            eventos: Default::default(),
            avisos: broadcast::channel(64).0,
        });
        // Y los eventos de las Facciones.
        let yo = s.clone();
        let mut rx = crate::runner::eventos().subscribe();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(e) => {
                        let mut h = yo.eventos.lock().unwrap();
                        h.push_back(e);
                        if h.len() > HISTORIAL {
                            h.pop_front();
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(_) => break,
                }
            }
        });
        // Guarda las acciones de los bots para el historial y la última de cada uno.
        let yo = s.clone();
        let mut rx = puerto::acciones().subscribe();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(a) => {
                        let mut h = yo.acciones.lock().unwrap();
                        h.push_back(a);
                        if h.len() > HISTORIAL {
                            h.pop_front();
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(_) => break,
                }
            }
        });
        s
    }

    fn registrar(&self, nivel: &str, texto: String) {
        eprintln!("bots: {texto}");
        let r = json!({"nivel": nivel, "texto": texto, "en": ahora()});
        let mut h = self.registros.lock().unwrap();
        h.push_back(r.clone());
        if h.len() > HISTORIAL {
            h.pop_front();
        }
        let _ = self.avisos.send(json!({"tipo": "registro", "nivel": r["nivel"], "texto": r["texto"], "en": r["en"]}));
    }

    /// Escucha en `dir` hasta que se cae el proceso.
    pub async fn escuchar(self: Arc<Self>, dir: &str) -> std::io::Result<()> {
        let l = TcpListener::bind(dir).await?;
        loop {
            let (s, _) = l.accept().await?;
            tokio::spawn(self.clone().atender(s));
        }
    }

    async fn atender(self: Arc<Self>, mut s: TcpStream) {
        let mut ojeo = [0u8; 11];
        if s.peek(&mut ojeo).await.is_ok() && ojeo.starts_with(b"GET /salud") {
            let mut basura = [0u8; 2048];
            let _ = s.read(&mut basura).await;
            let cuerpo = json!({"fase": self.interno.lock().await.fase}).to_string();
            let r = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{cuerpo}", cuerpo.len());
            let _ = s.write_all(r.as_bytes()).await;
            return;
        }
        // La relación pública: el estado de cada Facción de bots respecto a las demás, sin el número ni los planes. La lee el cliente del jugador, desde otro origen.
        let mut ojeo = [0u8; 15];
        if s.peek(&mut ojeo).await.is_ok() && ojeo.starts_with(b"GET /relaciones") {
            let mut basura = [0u8; 2048];
            let _ = s.read(&mut basura).await;
            let cuerpo = self.interno.lock().await.control.relaciones.lock().unwrap().to_string();
            let r = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\naccess-control-allow-origin: *\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{cuerpo}", cuerpo.len());
            let _ = s.write_all(r.as_bytes()).await;
            return;
        }
        let solo_control = |req: &Request, resp: Response| -> Result<Response, ErrorResponse> {
            if req.uri().path() == "/control" {
                Ok(resp)
            } else {
                Err(http::Response::builder().status(404).body(None).unwrap())
            }
        };
        let Ok(ws) = tokio_tungstenite::accept_hdr_async(s, solo_control).await else { return };
        let (mut tx, mut rx) = ws.split();
        let enviar = |v: Value| Message::Text(v.to_string().into());
        // Autenticarse primero, en 5 s.
        let primero = tokio::time::timeout(Duration::from_secs(5), rx.next()).await;
        let Ok(Some(Ok(Message::Text(t)))) = primero else { return };
        let v: Value = serde_json::from_str(&t).unwrap_or_default();
        let id = v["id"].clone();
        if v["accion"] != "autenticar" || self.ajustes.token.is_empty() || v["token"].as_str() != Some(self.ajustes.token.as_str()) {
            let _ = tx.send(enviar(json!({"tipo": "respuesta", "id": id, "ok": false, "error": "no autorizado"}))).await;
            return;
        }
        let mut acciones = puerto::acciones().subscribe();
        let mut avisos = self.avisos.subscribe();
        let mut eventos = crate::runner::eventos().subscribe();
        let historial = json!({
            "tipo": "historial",
            "acciones": Vec::from(self.acciones.lock().unwrap().clone()),
            "registros": Vec::from(self.registros.lock().unwrap().clone()),
            "eventos": Vec::from(self.eventos.lock().unwrap().clone()),
        });
        for m in [json!({"tipo": "respuesta", "id": id, "ok": true}), historial, json!({"tipo": "estado", "estado": self.estado().await})] {
            if tx.send(enviar(m)).await.is_err() {
                return;
            }
        }
        let mut cada = tokio::time::interval(Duration::from_secs(2));
        loop {
            let m = tokio::select! {
                m = rx.next() => match m {
                    Some(Ok(Message::Text(t))) => {
                        let v: Value = serde_json::from_str(&t).unwrap_or_default();
                        let r = match self.clone().comando(&v).await {
                            Ok(datos) => json!({"tipo": "respuesta", "id": v["id"], "ok": true, "datos": datos}),
                            Err(e) => json!({"tipo": "respuesta", "id": v["id"], "ok": false, "error": e}),
                        };
                        if tx.send(enviar(r)).await.is_err() {
                            return;
                        }
                        json!({"tipo": "estado", "estado": self.estado().await})
                    }
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
                    _ => return,
                },
                a = acciones.recv() => match a {
                    Ok(a) => json!({"tipo": "accion", "accion": a}),
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return,
                },
                a = avisos.recv() => match a {
                    Ok(a) => a,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return,
                },
                e = eventos.recv() => match e {
                    Ok(e) => json!({"tipo": "evento", "evento": e}),
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return,
                },
                _ = cada.tick() => json!({"tipo": "estado", "estado": self.estado().await}),
            };
            if tx.send(enviar(m)).await.is_err() {
                return;
            }
        }
    }

    /// Lo que dice que el servicio estaba en marcha: apagar el proceso no cuenta como parar (12_NPC §14).
    fn ruta_en_marcha(&self) -> PathBuf {
        self.ajustes.registros.join(format!("servicio-{}.json", self.ajustes.poblacion.prefijo()))
    }

    /// Si se apagó con los bots en marcha, los retoma con su registro.
    pub async fn retomar(self: &Arc<Self>) {
        let Ok(t) = std::fs::read_to_string(self.ruta_en_marcha()) else { return };
        match serde_json::from_str::<ConfigBots>(&t) {
            // Una partida borrada no se retoma: sus cuentas ya no existen.
            Ok(c) if !self.ruta_registro(&c.partida).exists() => {
                let _ = std::fs::remove_file(self.ruta_en_marcha());
                self.registrar("info", format!("la partida {} ya no tiene registro de bots: no se retoma", c.partida));
            }
            Ok(c) => {
                self.registrar("info", format!("retomando los bots de la partida {}", c.partida));
                let mut i = self.interno.lock().await;
                self.clone().lanzar(&mut i, c, true);
            }
            Err(e) => self.registrar("error", format!("{}: {e}", self.ruta_en_marcha().display())),
        }
    }

    /// Arranca un [`Proceso`]. Con un registro de esta partida es reconectar sus bots: se queda con su plan de llegadas, no con el de `c`. Al retomar (`esperar`), espera a que
    /// el servidor del juego esté levantado.
    fn lanzar(self: Arc<Self>, i: &mut Interno, c: ConfigBots, esperar: bool) {
        let codigo = c.codigo_registro_bots.clone().filter(|c| !c.is_empty()).unwrap_or_else(|| self.ajustes.codigo_bots.clone());
        let servidor: String = crate::acceso::normalizar(&c.servidor);
        let registro = self.ruta_registro(&c.partida);
        let control = Arc::new(Control::default());
        (i.fase, i.error, i.config, i.control) = ("arrancando", None, Some(c.clone()), control.clone());
        let yo = self;
        tokio::spawn(async move {
            let r = loop {
                let cfg = Config {
                    servidor: servidor.clone(),
                    codigo_bots: codigo.clone(),
                    registro: registro.clone(),
                    total: c.total,
                    dias_llegada: c.dias_llegada,
                    semilla: c.semilla,
                    cada_ms: yo.ajustes.cada_ms,
                    // Quien paga a un mercenario espera respuesta a cualquier hora.
                    siempre: c.horario == "siempre" || yo.ajustes.poblacion == Poblacion::Mercenarios,
                    lugares: Lugares { campamentos: c.campamentos.clone(), por_campamento: c.por_campamento },
                };
                let r = match yo.ajustes.poblacion {
                    Poblacion::Libres => Proceso::arrancar_con(cfg, Bot, control.clone()).await,
                    Poblacion::Mercenarios => Proceso::arrancar_con(cfg, Mercenario, control.clone()).await,
                };
                match r {
                    Err(Fallo::Red(_)) if esperar => {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        // Lo pararon desde el panel mientras esperaba.
                        if yo.interno.lock().await.fase != "arrancando" {
                            return;
                        }
                    }
                    r => break r,
                }
            };
            let mut i = yo.interno.lock().await;
            match r {
                Ok(p) => {
                    let conf = i.config.as_mut().unwrap();
                    (conf.total, conf.dias_llegada, conf.semilla) = (p.plan.total, p.plan.dias_llegada, p.plan.semilla);
                    (conf.campamentos, conf.por_campamento) = (p.plan.lugares.campamentos.clone(), p.plan.lugares.por_campamento);
                    if let Err(e) = serde_json::to_string(conf).map_err(|e| e.to_string()).and_then(|t| std::fs::write(yo.ruta_en_marcha(), t).map_err(|e| e.to_string())) {
                        eprintln!("bots: {}: {e}", yo.ruta_en_marcha().display());
                    }
                    (i.fase, i.iniciado_en, i.proceso) = ("corriendo", Some(ahora()), Some(p));
                    drop(i);
                    yo.registrar("info", "bots en marcha".into());
                }
                Err(f) => {
                    (i.fase, i.error) = ("error", Some(format!("{f:?}")));
                    drop(i);
                    yo.registrar("error", format!("al arrancar: {f:?}"));
                }
            }
        });
    }

    /// El registro de una partida. Si `BOTS_REGISTROS` es la carpeta de partidas del motor, va dentro de la carpeta de la suya: la que lleva su nombre o la cuyo `partida.id`
    /// es ese id (el admin manda el id de la partida, no el nombre de su carpeta). Así, borrar la partida lo borra. Si no hay ninguna, en `BOTS_REGISTROS` mismo.
    fn ruta_registro(&self, partida: &str) -> PathBuf {
        let limpia: String = partida.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
        let prefijo = self.ajustes.poblacion.prefijo();
        let r = &self.ajustes.registros;
        let suya = |d: &PathBuf| d.file_name().is_some_and(|n| n == limpia.as_str()) || std::fs::read_to_string(d.join("partida.id")).is_ok_and(|id| id.trim() == partida);
        let carpeta = std::fs::read_dir(r).into_iter().flatten().flatten().map(|e| e.path()).find(|d| d.is_dir() && suya(d));
        match carpeta {
            Some(c) => c.join(format!("{prefijo}.json")),
            None => r.join(format!("{prefijo}-{limpia}.json")),
        }
    }

    async fn comando(self: Arc<Self>, v: &Value) -> Result<Value, String> {
        let mut i = self.interno.lock().await;
        let heroe = || v["heroeId"].as_str().map(String::from).ok_or("falta heroeId");
        let en_marcha = |i: &Interno| if i.proceso.is_some() { Ok(()) } else { Err("el servicio no está corriendo".to_string()) };
        match v["accion"].as_str().unwrap_or_default() {
            // `iniciar` empieza una partida con su plan de llegadas; `conectar` solo reconecta a los bots que ya están en ella (registro de la partida) y sigue su plan.
            accion @ ("iniciar" | "conectar") => {
                if !matches!(i.fase, "inactivo" | "error") {
                    return Err(format!("ya está {}", i.fase));
                }
                let c: ConfigBots = serde_json::from_value(v["config"].clone()).map_err(|e| format!("config: {e}"))?;
                if accion == "conectar" && !self.ruta_registro(&c.partida).exists() {
                    return Err(format!("no hay bots registrados en la partida {}: usa iniciar", c.partida));
                }
                self.clone().lanzar(&mut i, c, false);
                Ok(Value::Null)
            }
            "pausar" | "reanudar" => {
                en_marcha(&i)?;
                let pausar = v["accion"] == "pausar";
                i.control.pausado.store(pausar, std::sync::atomic::Ordering::Relaxed);
                i.fase = if pausar { "pausado" } else { "corriendo" };
                Ok(Value::Null)
            }
            "parar" => {
                let _ = std::fs::remove_file(self.ruta_en_marcha());
                // Retomando, aún sin servidor: deja de esperarlo.
                if i.fase == "arrancando" && i.proceso.is_none() {
                    i.fase = "inactivo";
                    return Ok(Value::Null);
                }
                let p = i.proceso.take().ok_or("el servicio no está corriendo")?;
                i.fase = "parando";
                let r = p.parar().await;
                (i.fase, i.iniciado_en) = ("inactivo", None);
                drop(i);
                match r {
                    Ok(()) => self.registrar("info", "bots parados".into()),
                    Err(f) => self.registrar("error", format!("al parar: {f:?}")),
                }
                Ok(Value::Null)
            }
            "reiniciarRegistro" => {
                if !matches!(i.fase, "inactivo" | "error") {
                    return Err("para el servicio antes".into());
                }
                let partida = v["partida"].as_str().map(String::from).or_else(|| i.config.as_ref().map(|c| c.partida.clone())).ok_or("¿de qué partida?")?;
                match std::fs::remove_file(self.ruta_registro(&partida)) {
                    Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("{e}")),
                    _ => Ok(Value::Null),
                }
            }
            "ajustar" => {
                let ms = v["cadaMs"].as_i64().ok_or("falta cadaMs")?;
                if let Some(c) = i.config.as_mut() {
                    c.cada_ms = ms;
                }
                Ok(Value::Null)
            }
            "forzarLlegada" => {
                en_marcha(&i)?;
                let perfil = match (self.ajustes.poblacion, v["perfil"].as_str()) {
                    (Poblacion::Mercenarios, _) => TipoPerfil::Mercenario,
                    (_, Some("amigos")) => TipoPerfil::Amigos,
                    (_, Some("solitario")) => TipoPerfil::Solitario,
                    (_, Some("tardio")) => TipoPerfil::Tardio,
                    _ => return Err("perfil: amigos, solitario o tardio".into()),
                };
                i.control.forzadas.lock().unwrap().push(perfil);
                Ok(Value::Null)
            }
            "modoBot" => {
                let modo: Modo = serde_json::from_value(v["modo"].clone()).map_err(|_| "modo: auto, conectado, desconectado o congelado")?;
                i.control.modos.lock().unwrap().insert(heroe()?, modo);
                Ok(Value::Null)
            }
            "retirarBot" => {
                en_marcha(&i)?;
                i.control.retirar.lock().unwrap().push(heroe()?);
                Ok(Value::Null)
            }
            "pensarYa" => {
                i.control.pensar_ya.lock().unwrap().insert(heroe()?);
                Ok(Value::Null)
            }
            "volcarMemoria" => {
                let id = heroe()?;
                let bots = i.control.bots.lock().unwrap();
                let b = bots.get(&id).ok_or("ese bot no está en el servicio")?;
                let mut memoria = i.control.memorias.lock().unwrap().get(&id).cloned().unwrap_or_else(|| json!({}));
                memoria["residenciaId"] = json!(b.residencia_id);
                memoria["pizarra"] = json!(b.faccion_id.clone().unwrap_or_else(|| format!("sin-faccion:{id}")));
                let diag = json!({"nombre": b.nombre, "fase": b.fase, "modo": b.modo, "retirado": b.retirado, "faccionId": b.faccion_id, "vistaEn": b.vista_en, "pensamientos": b.pensamientos});
                Ok(json!({"heroeId": id, "conectado": b.conectado, "sesiones": b.sesiones, "diag": diag, "memoria": memoria}))
            }
            "volcarPizarra" => {
                // La de una Facción (`faccionId`) o la de un bot sin Facción (`sin-faccion:<heroe>`): entera, con su libro de agravios y su relación con las demás.
                let clave = v["faccionId"].as_str().or(v["pizarra"].as_str()).ok_or("falta faccionId")?;
                let ver = i.control.pizarras.lock().unwrap().clone().ok_or("los bots no están en marcha")?;
                let t = i.control.mundo_ms.load(std::sync::atomic::Ordering::Relaxed);
                match ver(Some(clave), t) {
                    Value::Null => Err(format!("no hay pizarra {clave}")),
                    p => Ok(p),
                }
            }
            otra => Err(format!("acción desconocida: {otra}")),
        }
    }

    async fn estado(&self) -> Value {
        let i = self.interno.lock().await;
        let ultimas: std::collections::HashMap<String, Value> =
            self.acciones.lock().unwrap().iter().filter_map(|a| Some((a["heroeId"].as_str()?.to_string(), a.clone()))).collect();
        let memorias = i.control.memorias.lock().unwrap();
        let bots: Vec<Value> = i
            .control
            .bots
            .lock()
            .unwrap()
            .values()
            .map(|b| {
                let mut v = serde_json::to_value(b).unwrap_or_default();
                v["esperas"] = memorias.get(&b.heroe_id).and_then(|m| m["esperas"].as_array()).map_or(json!([]), |e| json!(e.iter().map(|x| json!({"clave": x[0], "hasta": x[1]})).collect::<Vec<_>>()));
                v["ultimoError"] = b.ultimo_error.as_ref().map_or(Value::Null, |e| json!({"mensaje": e, "en": 0}));
                v["ultimaAccion"] = ultimas.get(&b.heroe_id).cloned().unwrap_or(Value::Null);
                v
            })
            .collect();
        drop(memorias);
        let conectados = bots.iter().filter(|b| b["conectado"] == true).count();
        let (hechas, extras) = *i.control.llegadas.lock().unwrap();
        let plan: Vec<Value> = i.config.as_ref().map_or_else(Vec::new, |c| {
            self.ajustes
                .poblacion
                .plan(c.semilla, c.total, c.dias_llegada)
                .iter()
                .enumerate()
                .map(|(n, l)| {
                    let perfil = match l.perfil {
                        TipoPerfil::Amigos => "amigos",
                        TipoPerfil::Solitario => "solitario",
                        TipoPerfil::Tardio => "tardio",
                        TipoPerfil::Refuerzo(_) => "refuerzo",
                        TipoPerfil::Mercenario => "mercenario",
                    };
                    // Aquí no hay ticks: `tick` es el instante de mundo (ms) desde el arranque.
                    json!({"grupo": l.grupo, "perfil": perfil, "cuantos": l.cuantos, "tick": l.en_ms, "hecha": n < hechas})
                })
                .collect()
        });
        let mundo = i.control.mundo_ms.load(std::sync::atomic::Ordering::Relaxed);
        // Las pizarras, con los bots de cada una (los de la flota que son de esa Facción; un bot sin Facción lleva la suya).
        let ver = i.control.pizarras.lock().unwrap().clone();
        let mut pizarras = ver.map_or(json!([]), |ver| ver(None, mundo));
        for p in pizarras.as_array_mut().into_iter().flatten() {
            let id = p["id"].as_str().unwrap_or_default().to_string();
            p["bots"] = json!(bots
                .iter()
                .filter(|b| b["retirado"] != true && b["faccionId"].as_str().map_or(format!("sin-faccion:{}", b["heroeId"].as_str().unwrap_or_default()), String::from) == id)
                .map(|b| b["heroeId"].clone())
                .collect::<Vec<_>>());
        }
        let (peticiones, errores, p50, p95) = puerto::salud();
        let config = i.config.as_ref().map(|c| {
            json!({"servidor": c.servidor, "partida": c.partida, "total": c.total, "diasLlegada": c.dias_llegada, "semilla": c.semilla, "cadaMs": c.cada_ms, "horario": c.horario, "campamentos": c.campamentos, "porCampamento": c.por_campamento})
        });
        json!({
            "fase": i.fase,
            "ahora": ahora(),
            "iniciadoEn": i.iniciado_en,
            "config": config,
            "tick": (mundo > 0).then_some(mundo),
            "error": i.error,
            "llegadas": {"hechas": hechas, "extras": extras, "plan": plan},
            // `erroresMin`: órdenes rechazadas por el motor o fallidas en el último minuto. ponytail: sin cuenta de sesiones renovadas.
            "salud": {"peticionesMin": peticiones, "erroresMin": errores, "latenciaP50": p50, "latenciaP95": p95, "socketsAbiertos": conectados, "sesionesRenovadas": 0},
            "bots": bots,
            "pizarras": pizarras,
            "codigoEnEntorno": !self.ajustes.codigo_bots.is_empty(),
            "poblacion": self.ajustes.poblacion.prefijo(),
            "porDefecto": self.ajustes.por_defecto,
        })
    }
}
