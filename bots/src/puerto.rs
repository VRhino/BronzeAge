//! El puerto de un bot (`Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md` §4 del motor): lo único que un cerebro conoce del mundo. **Observar** es lo que el servidor le manda por sus
//! ámbitos, con su niebla, guardado en un [`Almacen`]; **actuar** es una orden del contrato 05 con la autorización de cualquier jugador. Hay un solo adaptador, el remoto: el WebSocket del
//! contrato 05, como un jugador, nunca dentro del motor.
//!
//! Reanudar es reconectar (contrato 05 §7): ante un corte, un hueco de secuencia o un token caducado, el puerto vuelve a entrar con lo que conserva (`Almacen::hola`) y el servidor manda
//! solo lo que falta. Una orden cortada se reenvía con el mismo `peticion_id`: si ya se aplicó, vuelve `ORDEN_DUPLICADA` y no se aplica dos veces.

use std::{
    collections::{BTreeMap, HashSet},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use contrato::almacen::{clave_de_ambito, Almacen};
use contrato::v1::{
    mensaje_de_cliente as mc, mensaje_de_servidor as ms, orden, AmbitoTipo, Aviso, Bienvenida, CambiarVista, DecisionReanudacion, MensajeDeCliente, MensajeDeServidor, Orden,
    Rectangulo, Reloj, Resultado, ResultadoOrden, TipoCliente,
};
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;
use tokio::{net::TcpStream, time::timeout};
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::Message, Connector, MaybeTlsStream, WebSocketStream};

use crate::{acceso, Credenciales, Fallo};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Intentos de conexión antes de rendirse; entre uno y otro la espera se dobla desde 250 ms (≈ 8 s en total).
const INTENTOS: u32 = 6;
/// Lo que se espera la respuesta de una orden o de la `VISTA` antes de darla por cortada.
const SIN_RESPUESTA: Duration = Duration::from_secs(30);

/// Cuántas órdenes dio el proceso, por orden y resultado (o código de rechazo).
static ORDENES: std::sync::Mutex<Option<BTreeMap<(String, String), u64>>> = std::sync::Mutex::new(None);

/// Cada orden que da un puerto, en vivo, para el canal de control: `{heroeId, tipo, ok, motivo?, ms, en}` (`AccionDeBot` del contrato de control).
pub fn acciones() -> &'static tokio::sync::broadcast::Sender<serde_json::Value> {
    static ACCIONES: std::sync::OnceLock<tokio::sync::broadcast::Sender<serde_json::Value>> = std::sync::OnceLock::new();
    ACCIONES.get_or_init(|| tokio::sync::broadcast::channel(256).0)
}

/// Cada `MarcharA` aceptada fuera de casa: Facción, columna, a qué va (`band:`, `as:`, `pueblo:`, `obra:`, `avanzada:`; un punto, `celda:` de 20 u, o, sin tropa, `ver:`
/// desde una plaza y `vuelta:` desde un campamento) y cuándo. Para medir cuántas veces dos columnas de una Facción salen por separado a lo mismo.
static MARCHAS: std::sync::Mutex<Vec<(String, String, String, i64)>> = std::sync::Mutex::new(Vec::new());

/// Las órdenes del último minuto: cuándo, cuánto tardó la respuesta (ms) y si salió mal (rechazo o fallo). Para la salud del servicio en el panel.
static RESPUESTAS: std::sync::Mutex<std::collections::VecDeque<(Instant, u32, bool)>> = std::sync::Mutex::new(std::collections::VecDeque::new());

/// La salud del último minuto: órdenes, cuántas salieron mal y el tiempo de respuesta (mediana y percentil 95, ms).
pub fn salud() -> (usize, usize, u32, u32) {
    let mut r = RESPUESTAS.lock().unwrap();
    while r.front().is_some_and(|(en, _, _)| en.elapsed() > Duration::from_secs(60)) {
        r.pop_front();
    }
    let mut ms: Vec<u32> = r.iter().map(|(_, m, _)| *m).collect();
    ms.sort_unstable();
    let p = |q: f64| ms.get(((ms.len() as f64 - 1.0) * q).round() as usize).copied().unwrap_or(0);
    (r.len(), r.iter().filter(|(_, _, mal)| *mal).count(), p(0.5), p(0.95))
}

/// Si se apuntan las salidas en [`MARCHAS`]: solo `partida`, que las lee al final; el servicio, que corre días, no.
pub static MEDIR_MARCHAS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Las salidas apuntadas hasta ahora ([`MARCHAS`]).
pub fn marchas() -> Vec<(String, String, String, i64)> {
    MARCHAS.lock().unwrap().clone()
}

/// Las órdenes que dieron todos los puertos del proceso hasta ahora, por orden y resultado.
pub fn ordenes_dadas() -> BTreeMap<(String, String), u64> {
    ORDENES.lock().unwrap().clone().unwrap_or_default()
}

/// `BOTS_TRAZA=1`: cada orden y su resultado, por la salida de errores.
fn traza() -> bool {
    static T: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *T.get_or_init(|| std::env::var("BOTS_TRAZA").is_ok_and(|v| v == "1"))
}

/// El instante de mundo ahora, desde un reloj recibido en `cuando`.
pub fn mundo_ms((r, cuando): &(Reloj, Instant)) -> i64 {
    if r.pausado {
        return r.mundo_ms;
    }
    r.mundo_ms + (cuando.elapsed().as_millis() as i64) * r.factor_milesimas as i64 / 1000
}

pub struct Puerto {
    dir: String,
    credenciales: Credenciales,
    token: String,
    ws: Option<Ws>,
    ultima: u64,
    /// La `Bienvenida` de la última conexión: héroe, Facción, mundo y, sin héroe, los campamentos donde nacer.
    pub bienvenida: Bienvenida,
    almacen: Almacen,
    /// El último `Reloj` recibido y cuándo llegó.
    reloj: (Reloj, Instant),
    /// Los avisos que llegaron (`reloj.cambiado`, `sesion.reemplazada`...).
    pub avisos: Vec<Aviso>,
    /// Hace únicos los `peticion_id` de la cuenta también entre reinicios del proceso.
    prefijo: String,
    peticiones: u64,
    vista: Option<Rectangulo>,
    /// Cuántas veces ha conectado (la primera incluida).
    pub conexiones: u32,
    tipo: TipoCliente,
}

impl Puerto {
    /// Login con `credenciales` en el servidor de `dir` (`127.0.0.1:7878`) y entrada al contrato 05 como `CLIENTE_BOT`.
    pub async fn entrar(dir: &str, credenciales: Credenciales) -> Result<Self, Fallo> {
        Self::entrar_como(dir, credenciales, TipoCliente::ClienteBot).await
    }

    /// Como [`Puerto::entrar`], con otra clase de cliente: el batch entra como `CLIENTE_ADMIN_WEB` (con una cuenta de administrador) para acelerar el reloj.
    pub async fn entrar_como(dir: &str, credenciales: Credenciales, tipo: TipoCliente) -> Result<Self, Fallo> {
        let token = acceso::iniciar_sesion(dir, &credenciales).await?;
        let ms_epoca = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let mut p = Self {
            dir: dir.into(),
            prefijo: format!("{}-{ms_epoca}", credenciales.nick),
            credenciales,
            token,
            ws: None,
            ultima: 0,
            bienvenida: Bienvenida::default(),
            almacen: Almacen::default(),
            reloj: (Reloj::default(), Instant::now()),
            avisos: Vec::new(),
            peticiones: 0,
            vista: None,
            conexiones: 0,
            tipo,
        };
        p.reconectar().await?;
        Ok(p)
    }

    /// El servidor al que habla (`127.0.0.1:7878`).
    pub fn servidor(&self) -> &str {
        &self.dir
    }

    /// Lo que ve: sus ámbitos con su niebla y los hechos que le llegaron.
    pub fn observar(&self) -> &Almacen {
        &self.almacen
    }

    /// Los hechos que llegaron desde la última vez, que dejan de guardarse.
    pub fn hechos(&mut self) -> Vec<(String, contrato::v1::EventoDominio)> {
        std::mem::take(&mut self.almacen.hechos)
    }

    /// El instante de mundo ahora, según el último reloj recibido.
    pub fn mundo_ms(&self) -> i64 {
        mundo_ms(&self.reloj)
    }

    /// Da una orden y aplica todo lo que llega hasta su resultado (respuesta causal: al volver, [`Puerto::observar`] ya tiene lo que cambió). Un corte se reintenta una vez tras reconectar,
    /// con el mismo `peticion_id`: si ya se había aplicado, el resultado es `ORDEN_DUPLICADA`.
    pub async fn actuar(&mut self, cuerpo: orden::Cuerpo) -> Result<ResultadoOrden, Fallo> {
        self.peticiones += 1;
        let empieza = Instant::now();
        let o = Orden { peticion_id: format!("{}-{}", self.prefijo, self.peticiones), cuerpo: Some(cuerpo), ..Default::default() };
        let r = match self.intentar(&o).await {
            Err(Fallo::Red(_)) => {
                self.reconectar().await?;
                self.intentar(&o).await
            }
            r => r,
        };
        let que = format!("{:?}", o.cuerpo.as_ref().unwrap());
        let que = que.split('(').next().unwrap_or(&que).to_string();
        let como = match &r {
            Ok(r) => r.rechazo.as_ref().map_or_else(|| format!("{:?}", Resultado::try_from(r.resultado).unwrap_or_default()), |x| x.codigo.clone()),
            Err(f) => format!("{f:?}"),
        };
        if traza() {
            eprintln!("{} {que} → {como}", self.credenciales.nick);
        }
        if acciones().receiver_count() > 0 {
            let ok = matches!(&r, Ok(x) if x.rechazo.is_none());
            let en = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64);
            let heroe = self.bienvenida.heroe_id.clone().unwrap_or_else(|| self.credenciales.nick.clone());
            let _ = acciones().send(serde_json::json!({"heroeId": heroe, "tipo": que, "ok": ok, "motivo": (!ok).then_some(&como), "ms": self.mundo_ms(), "en": en}));
        }
        let mal = !matches!(&r, Ok(x) if x.rechazo.is_none());
        {
            // Solo el último minuto: sin panel que pida la salud, no crece.
            let mut r = RESPUESTAS.lock().unwrap();
            r.push_back((Instant::now(), empieza.elapsed().as_millis() as u32, mal));
            while r.front().is_some_and(|(en, _, _)| en.elapsed() > Duration::from_secs(60)) {
                r.pop_front();
            }
        }
        *ORDENES.lock().unwrap().get_or_insert_with(BTreeMap::new).entry((que, como)).or_default() += 1;
        if let (Some(orden::Cuerpo::MarcharA(m)), Ok(x)) = (&o.cuerpo, &r) {
            if x.resultado == Resultado::OrdenAceptada as i32 {
                self.apuntar_marcha(m);
            }
        }
        r
    }

    /// Apunta en [`MARCHAS`] una salida aceptada, salvo la vuelta a casa (una plaza propia o un campamento).
    fn apuntar_marcha(&self, m: &contrato::v1::OrdenMarcharA) {
        use contrato::v1::{entidad::Cuerpo as E, orden_marchar_a::objetivo::Tipo};
        let a = self.observar();
        let Some(faccion) = a.ambitos.keys().find_map(|k| k.strip_prefix("FACCION:")) else { return };
        let Some(col) = crate::cerebro::columna(a, &m.heroe_id) else { return };
        let todo = || crate::cerebro::entidades(a);
        let clave = match m.objetivo.as_ref().and_then(|o| o.tipo.as_ref()) {
            Some(Tipo::AsentamientoId(id)) => match todo().find_map(|c| match c {
                E::Asentamiento(x) if x.id == *id => Some(x.faccion_id == faccion),
                _ => None,
            }) {
                Some(true) => return,
                _ => format!("as:{id}"),
            },
            Some(Tipo::Punto(p)) => {
                let cerca = |q: &Option<contrato::v1::Punto>| q.as_ref().is_some_and(|q| crate::cerebro::distancia(q, p) <= 3_000);
                let casa = String::new();
                let c = todo().find_map(|c| match c {
                    E::CampamentoBandidos(b) if cerca(&b.posicion) => Some(format!("band:{}", b.id)),
                    E::Asentamiento(x) if cerca(&x.posicion) => Some(if x.faccion_id == faccion { casa.clone() } else { format!("as:{}", x.id) }),
                    E::CampamentoMercenarios(k) if cerca(&k.posicion) => Some(casa.clone()),
                    E::Pueblo(x) if cerca(&x.posicion) => Some(format!("pueblo:{}", x.id)),
                    E::Obra(x) if cerca(&x.posicion) => Some(format!("obra:{}", x.id)),
                    E::Avanzada(x) if cerca(&x.posicion) => Some(format!("avanzada:{}", x.id)),
                    _ => None,
                });
                // Sin tropa es explorar: `ver` desde una plaza; `vuelta`, la vuelta al anillo de un campamento.
                let en_campamento = todo().any(|c| matches!(c, E::Heroe(h) if h.id == m.heroe_id && h.residencia_id.as_deref().is_some_and(|r| r.starts_with("camp-"))));
                let tipo = match (col.escuadron_ids.is_empty(), en_campamento) {
                    (false, _) => "celda",
                    (true, false) => "ver",
                    (true, true) => "vuelta",
                };
                match c {
                    Some(c) if c.is_empty() => return,
                    Some(c) => c,
                    None => format!("{tipo}:{},{}", p.x.div_euclid(20_000), p.y.div_euclid(20_000)),
                }
            }
            None => return,
        };
        if MEDIR_MARCHAS.load(std::sync::atomic::Ordering::Relaxed) {
            MARCHAS.lock().unwrap().push((faccion.into(), col.id, clave, self.mundo_ms()));
        }
    }

    /// Mira una zona del mapa (en milésimas de unidad) y aplica la `Foto` de su `VISTA`. Se recuerda: al reconectar se vuelve a abrir.
    pub async fn mirar(&mut self, r: Rectangulo) -> Result<(), Fallo> {
        self.vista = Some(r);
        match self.abrir_vista().await {
            Err(Fallo::Red(_)) => self.reconectar().await,
            Err(f) => {
                self.vista = None;
                Err(f)
            }
            ok => ok,
        }
    }

    /// Aplica lo que llegue sin pedirlo (lo que hacen los demás, el reloj) durante `cuanto` como mucho; devuelve cuántas `Delta` llegaron. Un bot conectado tiene que atender su sesión: el
    /// servidor descarta a quien no lee.
    pub async fn atender(&mut self, cuanto: Duration) -> Result<usize, Fallo> {
        let fin = Instant::now() + cuanto;
        let mut deltas = 0;
        while let Some(queda) = fin.checked_duration_since(Instant::now()) {
            match timeout(queda, self.recibir()).await {
                Err(_) => break,
                Ok(Ok(ms::Cuerpo::Delta(_))) => deltas += 1,
                Ok(Ok(_)) => {}
                Ok(Err(Fallo::Red(_))) => self.reconectar().await?,
                Ok(Err(f)) => return Err(f),
            }
        }
        Ok(deltas)
    }

    /// Corta la conexión sin despedirse, como un corte de red (para probar la reanudación).
    pub fn cortar(&mut self) {
        self.ws = None;
    }

    /// El reloj de mundo tal como llegó, con cuándo: para quien lo comparte con otros bots.
    pub fn reloj(&self) -> (Reloj, Instant) {
        self.reloj.clone()
    }

    /// Cierra la conexión como quien cierra el cliente; lo que ve se conserva y la siguiente orden vuelve a entrar con ello.
    pub async fn cerrar(&mut self) {
        if let Some(mut ws) = self.ws.take() {
            let _ = ws.close(None).await;
        }
    }

    async fn intentar(&mut self, o: &Orden) -> Result<ResultadoOrden, Fallo> {
        self.enviar(mc::Cuerpo::Orden(o.clone())).await?;
        let esperar = async {
            loop {
                if let ms::Cuerpo::Resultado(r) = self.recibir().await? {
                    if r.peticion_id == o.peticion_id {
                        return Ok(r);
                    }
                }
            }
        };
        timeout(SIN_RESPUESTA, esperar).await.unwrap_or_else(|_| Err(Fallo::Red("sin respuesta a la orden".into())))
    }

    async fn abrir_vista(&mut self) -> Result<(), Fallo> {
        let r = self.vista.clone();
        self.enviar(mc::Cuerpo::Vista(CambiarVista { rectangulo: r })).await?;
        let esperar = async {
            loop {
                match self.recibir().await? {
                    ms::Cuerpo::Foto(f) if f.ambito.as_ref().is_some_and(|a| a.tipo == AmbitoTipo::AmbitoVista as i32) => return Ok(()),
                    ms::Cuerpo::Aviso(a) if a.codigo.starts_with("vista.") => return Err(Fallo::Vista(a.codigo)),
                    _ => {}
                }
            }
        };
        timeout(SIN_RESPUESTA, esperar).await.unwrap_or_else(|_| Err(Fallo::Red("sin respuesta a la vista".into())))
    }

    /// Vuelve a entrar con lo que conserva, con esperas crecientes; un token caducado o rechazado se renueva con un login.
    async fn reconectar(&mut self) -> Result<(), Fallo> {
        let mut espera = Duration::from_millis(250);
        let mut ultimo = Fallo::Red("sin intentos".into());
        for _ in 0..INTENTOS {
            match self.conectar().await {
                Ok(()) => return Ok(()),
                Err(Fallo::Sesion(e)) if e.codigo == "token_invalido" => {
                    self.token = acceso::iniciar_sesion(&self.dir, &self.credenciales).await?;
                    ultimo = Fallo::Sesion(e);
                }
                Err(Fallo::Red(e)) => {
                    tokio::time::sleep(espera).await;
                    espera *= 2;
                    ultimo = Fallo::Red(e);
                }
                Err(f) => return Err(f),
            }
        }
        Err(ultimo)
    }

    async fn conectar(&mut self) -> Result<(), Fallo> {
        self.ws = None;
        let (ws, _) = connect_async_tls_with_config(acceso::url_ws(&self.dir), None, false, Some(Connector::Rustls(acceso::tls()))).await.map_err(|e| Fallo::Red(e.to_string()))?;
        self.ws = Some(ws);
        self.ultima = 0;
        let mut hola = self.almacen.hola(&self.token, self.tipo);
        hola.version_cliente = concat!("bots-", env!("CARGO_PKG_VERSION")).into();
        hola.compresiones = vec!["zstd".into()];
        self.enviar(mc::Cuerpo::Hola(hola)).await?;
        let b = match timeout(SIN_RESPUESTA, self.siguiente()).await.unwrap_or_else(|_| Err(Fallo::Red("sin Bienvenida".into())))? {
            ms::Cuerpo::Bienvenida(b) => b,
            otro => return Err(Fallo::Red(format!("se esperaba la Bienvenida: {otro:?}"))),
        };
        self.conexiones += 1;
        // Lo que ya no le toca (y la `VISTA`, que no se reanuda) se descarta.
        let suscritos: HashSet<String> = b.ambitos.iter().map(|a| clave_de_ambito(&a.ambito)).collect();
        self.almacen.ambitos.retain(|k, _| suscritos.contains(k));
        self.reloj = (b.reloj.clone().unwrap_or_default(), Instant::now());
        let pendientes = b.ambitos.iter().filter(|a| a.decision != DecisionReanudacion::ReanudaSinNovedad as i32).count();
        self.bienvenida = b;
        for _ in 0..pendientes {
            timeout(SIN_RESPUESTA, self.recibir()).await.unwrap_or_else(|_| Err(Fallo::Red("sin la reanudación".into())))?;
        }
        if self.vista.is_some() {
            self.abrir_vista().await?;
        }
        Ok(())
    }

    /// El siguiente mensaje, aplicado: al almacén lo de los ámbitos, el reloj del `Pong` y de los avisos.
    async fn recibir(&mut self) -> Result<ms::Cuerpo, Fallo> {
        let m = self.siguiente().await?;
        match &m {
            ms::Cuerpo::Pong(p) => {
                if let Some(r) = &p.reloj {
                    self.reloj = (r.clone(), Instant::now());
                }
            }
            ms::Cuerpo::Aviso(a) => {
                if let Some(r) = &a.reloj {
                    self.reloj = (r.clone(), Instant::now());
                }
                self.avisos.push(a.clone());
            }
            otro => self.almacen.aplicar(otro),
        }
        Ok(m)
    }

    /// El siguiente mensaje del servidor, descomprimido y con la secuencia comprobada. Un error fatal es un `Fallo` (`interno`, que es el de un cliente lento descartado, se trata como un
    /// corte); uno que no lo es se devuelve como mensaje.
    async fn siguiente(&mut self) -> Result<ms::Cuerpo, Fallo> {
        let ws = self.ws.as_mut().ok_or_else(|| Fallo::Red("sin conexión".into()))?;
        let b = loop {
            match ws.next().await {
                Some(Ok(Message::Binary(b))) => break b,
                Some(Ok(Message::Close(_))) | None => return Err(self.caido("el servidor cerró".into())),
                Some(Err(e)) => return Err(self.caido(e.to_string())),
                Some(Ok(_)) => {}
            }
        };
        let decodificado = if b.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
            zstd::bulk::decompress(&b, 64 << 20).map_err(|e| e.to_string()).and_then(|v| MensajeDeServidor::decode(v.as_slice()).map_err(|e| e.to_string()))
        } else {
            MensajeDeServidor::decode(&b[..]).map_err(|e| e.to_string())
        };
        let m = decodificado.map_err(|e| self.caido(format!("mensaje ilegible: {e}")))?;
        if m.secuencia != self.ultima + 1 {
            return Err(self.caido(format!("hueco de secuencia: {} tras {}", m.secuencia, self.ultima)));
        }
        self.ultima = m.secuencia;
        match m.cuerpo {
            Some(ms::Cuerpo::Error(e)) if e.fatal && e.codigo == "interno" => Err(self.caido(format!("interno: {}", e.mensaje))),
            Some(ms::Cuerpo::Error(e)) if e.fatal => {
                self.ws = None;
                Err(Fallo::Sesion(e))
            }
            Some(c) => Ok(c),
            None => Err(self.caido("mensaje sin cuerpo".into())),
        }
    }

    fn caido(&mut self, por: String) -> Fallo {
        self.ws = None;
        Fallo::Red(por)
    }

    async fn enviar(&mut self, cuerpo: mc::Cuerpo) -> Result<(), Fallo> {
        let ws = self.ws.as_mut().ok_or_else(|| Fallo::Red("sin conexión".into()))?;
        // ponytail: se manda sin comprimir (el servidor acepta las dos); comprimir las órdenes si algún día pesan.
        let bytes = MensajeDeCliente { cuerpo: Some(cuerpo) }.encode_to_vec();
        if let Err(e) = ws.send(Message::Binary(bytes.into())).await {
            return Err(self.caido(e.to_string()));
        }
        Ok(())
    }
}
