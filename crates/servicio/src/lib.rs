//! El proceso del servidor (fase 7, tanda 7B): carga el artefacto de mundo, abre el núcleo persistente, lanza su actor y sirve en un solo puerto el WebSocket del contrato 05 y el
//! servicio de acceso (`acceso`). Lo usan los bots, el admin web y los clientes de jugador, siempre desde fuera del proceso.
//!
//! La carpeta de datos lleva `mundo/` (registro de eventos, fotos y crónica del núcleo) y `acceso/` (cuentas y sesiones). Un reinicio con la misma carpeta y el mismo artefacto sigue la
//! partida donde se quedó: el reloj de mundo continúa desde el último instante guardado y los tokens vigentes siguen valiendo.
//!
//! El mismo puerto sirve el artefacto de mundo por hash (`GET /v1/mundo/<hash>`, contrato 06) y la `Bienvenida` lo anuncia en `InfoMundo.url_artefacto` (ruta relativa al servidor).
//! Si junto al artefacto está su paquete de relieve (`relieve-<hash>.bin`, 06 §8), lo sirve en `GET /v1/mundo/<hash>/relieve` y lo anuncia en `InfoMundo.url_relieve` y `hash_relieve`.
//! Las rutas de administración (`/v1/admin/...`, solo administradores) están en [`admin`].
//! Y el archivo común de datos de juego ([`datos`]): `GET /v1/datos/<versión>` (y `GET /v1/datos`, el vigente), anunciado en `InfoMundo.version_datos_de_juego` y `url_datos_de_juego`.

pub mod admin;
pub mod datos;

use std::{
    future::Future,
    io,
    net::SocketAddr,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

use acceso::Acceso;
use mundo::{validar_relieve, ErrorRelieve, Mundo, OpcionesValidacion};
use nucleo::{sesiones, Ajustes, Arranque, Nucleo, Reglas};
use rutas::Rutas;
use servidor::{servir_con_http, ComandoSesion, Config, Http, PeticionHttp, RelojCompartido, RespuestaHttp, Servidor};
use tokio::{sync::mpsc, task::JoinHandle};

/// El artefacto de mundo: `ruta` si es un fichero; si es una carpeta, el primer `mundo-*.bin` que tenga.
pub fn buscar_mundo(ruta: &Path) -> io::Result<PathBuf> {
    if ruta.is_file() {
        return Ok(ruta.to_owned());
    }
    let mut candidatos: Vec<PathBuf> = std::fs::read_dir(ruta)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("mundo-")) && p.extension().is_some_and(|x| x == "bin"))
        .collect();
    candidatos.sort();
    candidatos.into_iter().next().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("no hay ningún mundo-*.bin en {}", ruta.display())))
}

/// La ruta del artefacto de mundo, sin el hash.
pub const RUTA_MUNDO: &str = "/v1/mundo/";
/// Lo que va tras el hash del artefacto en la ruta de su paquete de relieve.
pub const SUFIJO_RELIEVE: &str = "/relieve";
/// La ruta del archivo de datos de juego (sin versión: el vigente).
pub const RUTA_DATOS: &str = "/v1/datos";

/// El artefacto de mundo cargado: los bytes (para servirlos tal cual) y el arranque del núcleo.
pub struct Artefacto {
    pub bytes: Arc<Vec<u8>>,
    pub arranque: Arranque,
}

/// El arranque del núcleo con el artefacto de `ruta`: reglas por defecto (sin servidores de batalla: todo con números) y la semilla dada.
pub fn arranque_de(ruta: &Path, semilla: u64) -> io::Result<Artefacto> {
    let bytes = std::fs::read(ruta)?;
    let m = Mundo::cargar(&bytes, &OpcionesValidacion::default()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e:?}", ruta.display())))?;
    let rutas = Arc::new(Rutas::desde_mundo(&m));
    let arranque = Arranque {
        reglas: Reglas::default(),
        ajustes: Ajustes::default(),
        semilla,
        inicio: 0,
        rutas,
        mundo_hash: m.hash().into(),
        mundo: Some(Arc::new(m)),
    };
    Ok(Artefacto { bytes: Arc::new(bytes), arranque })
}

/// El paquete de relieve validado: su hash y los bytes, para servirlos tal cual.
#[derive(Clone)]
pub struct Relieve {
    pub hash: String,
    pub bytes: Arc<Vec<u8>>,
}

/// El paquete de relieve que está junto al artefacto `ruta` (`relieve-<hash>.bin`, en la misma carpeta), validado contra el mundo del arranque.
/// `Ok(None)` si no está: el servidor arranca igual y los clientes 3D se quedan sin la altura fina.
pub fn relieve_junto_a(ruta: &Path, arranque: &Arranque) -> Result<Option<Relieve>, ErrorRelieve> {
    let Some(m) = arranque.mundo.as_ref() else { return Ok(None) };
    let Ok(bytes) = std::fs::read(ruta.with_file_name(format!("relieve-{}.bin", m.hash()))) else { return Ok(None) };
    let hash = validar_relieve(&bytes, m.hash(), m.tamano())?;
    Ok(Some(Relieve { hash, bytes: Arc::new(bytes) }))
}

/// El HTTP del puerto: el artefacto de mundo y su paquete de relieve por hash y, lo demás, el servicio de acceso.
struct RutasHttp {
    acceso: Acceso,
    admin: admin::Admin,
    mundo: Option<(String, Arc<Vec<u8>>)>,
    relieve: Option<Relieve>,
    datos: datos::DatosDeJuego,
}

impl Http for RutasHttp {
    fn atender(&self, p: PeticionHttp) -> Pin<Box<dyn Future<Output = RespuestaHttp> + Send + '_>> {
        if let Some(resto) = p.ruta.strip_prefix(RUTA_DATOS).filter(|r| r.is_empty() || r.starts_with('/')) {
            let version = resto.trim_start_matches('/');
            let r = match p.metodo.as_str() {
                "GET" if version.is_empty() || version == self.datos.version => RespuestaHttp::json(200, self.datos.json.clone()),
                "GET" => RespuestaHttp::texto(404, "no hay datos de juego con esa versión"),
                _ => RespuestaHttp::texto(405, "método no permitido"),
            };
            return Box::pin(async move { r });
        }
        if p.ruta.starts_with(admin::RUTA_ADMIN) {
            return Box::pin(self.admin.atender(p));
        }
        if let Some(resto) = p.ruta.strip_prefix(RUTA_MUNDO) {
            let (hash, relieve) = match resto.strip_suffix(SUFIJO_RELIEVE) {
                Some(h) => (h, true),
                None => (resto, false),
            };
            let vigente = self.mundo.as_ref().filter(|(h, _)| h == hash);
            let r = match (vigente, relieve, p.metodo.as_str()) {
                (Some((_, bytes)), false, "GET") => RespuestaHttp { estado: 200, tipo: "application/octet-stream", cuerpo: bytes.as_ref().clone() },
                (Some(_), true, "GET") => match &self.relieve {
                    Some(r) => RespuestaHttp { estado: 200, tipo: "application/octet-stream", cuerpo: r.bytes.as_ref().clone() },
                    None => RespuestaHttp::texto(404, "este mundo no tiene paquete de relieve"),
                },
                (None, _, "GET") => RespuestaHttp::texto(404, "no hay artefacto con ese hash"),
                _ => RespuestaHttp::texto(405, "método no permitido"),
            };
            return Box::pin(async move { r });
        }
        self.acceso.atender(p)
    }
}

/// El proceso en marcha.
pub struct Servicio {
    pub direccion: SocketAddr,
    pub acceso: Acceso,
    srv: Servidor,
    tx: mpsc::Sender<ComandoSesion>,
    actor: JoinHandle<Nucleo>,
}

/// Lo que dice el arranque, para el registro del proceso.
#[derive(Clone, Debug)]
pub struct Informe {
    pub mundo_hash: String,
    pub version: u64,
    pub ahora: i64,
    pub recuperado: bool,
    pub cuentas: usize,
}

/// El fichero de la carpeta de datos con el id de la partida (el `gameId` del contrato de batalla).
pub const FICHERO_ID_PARTIDA: &str = "partida.id";

/// El id de la partida de la carpeta `datos` (paridad P4: el `gameId` real del TypeScript): el de `partida.id` o, si falta, uno nuevo que se guarda allí. Una partida que ya existía
/// sin ese fichero conserva el de antes (el hash del mundo y la semilla, `anterior`), para que el `battleId` de sus batallas no cambie.
fn id_de_partida(datos: &Path, recuperada: bool, anterior: &str) -> io::Result<String> {
    let fichero = datos.join(FICHERO_ID_PARTIDA);
    if let Ok(id) = std::fs::read_to_string(&fichero) {
        let id = id.trim();
        if !id.is_empty() {
            return Ok(id.to_owned());
        }
    }
    let id = if recuperada {
        anterior.to_owned()
    } else {
        let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        format!("p-{ms:x}-{:x}", std::process::id())
    };
    std::fs::create_dir_all(datos)?;
    std::fs::write(&fichero, &id)?;
    Ok(id)
}

/// Abre los datos de `datos` (o los crea), lanza el núcleo y escucha en `direccion` (puerto 0: uno libre). Con `artefacto`, el puerto lo sirve por su hash, y con `relieve`
/// (solo junto al artefacto), también su paquete de relieve.
pub async fn arrancar(
    arranque: Arranque,
    artefacto: Option<Arc<Vec<u8>>>,
    relieve: Option<Relieve>,
    datos: &Path,
    direccion: SocketAddr,
    acceso: acceso::Config,
) -> io::Result<(Servicio, Informe)> {
    let datos_de_juego = datos::datos_de_juego(&arranque.reglas);
    let semilla = arranque.semilla;
    let dir_mundo = datos.join("mundo");
    let mut n = tokio::task::block_in_place(|| Nucleo::con_persistencia(arranque, &dir_mundo, persistencia::Config::default()))?;
    let id_partida = id_de_partida(datos, n.recuperado(), &n.id_de_partida())?;
    n.fijar_id_de_partida(id_partida.clone());
    let acceso = Acceso::abrir(&datos.join("acceso"), acceso)?;
    let informe = Informe { mundo_hash: n.mundo_hash().into(), version: n.version(), ahora: n.ahora(), recuperado: n.recuperado(), cuentas: acceso.cuentas() };
    let reloj = RelojCompartido::nuevo(n.ahora());
    let (tx, rx) = mpsc::channel(1024);
    let nombres = servidor::NombresHeroes::default();
    n.conectar_nombres(nombres.clone());
    let actor = sesiones::lanzar(n, reloj.clone(), rx);
    let mundo = artefacto.map(|b| (informe.mundo_hash.clone(), b));
    let relieve = relieve.filter(|_| mundo.is_some());
    let config = Config {
        url_artefacto: mundo.is_some().then(|| RUTA_MUNDO.to_owned()),
        relieve: relieve.as_ref().map(|r| (r.hash.clone(), format!("{RUTA_MUNDO}{}{SUFIJO_RELIEVE}", informe.mundo_hash))),
        datos_de_juego: Some((datos_de_juego.version.clone(), format!("{RUTA_DATOS}/{}", datos_de_juego.version))),
        nombres,
        ..Config::default()
    };
    let admin = admin::Admin { acceso: acceso.clone(), nucleo: tx.clone(), carpeta: datos.to_owned(), mundo_hash: informe.mundo_hash.clone(), semilla, id_partida };
    let http = Arc::new(RutasHttp { acceso: acceso.clone(), admin, mundo, relieve, datos: datos_de_juego });
    let listener = tokio::net::TcpListener::bind(direccion).await?;
    let srv = servir_con_http(listener, tx.clone(), acceso.clone(), http, reloj, config).await;
    Ok((Servicio { direccion: srv.direccion, acceso, srv, tx, actor }, informe))
}

impl Servicio {
    /// Deja de aceptar, cierra las sesiones y apaga el núcleo (su última foto y el registro quedan en disco). Devuelve el núcleo apagado.
    pub async fn parar(self) -> Nucleo {
        self.srv.detener().await;
        drop(self.tx);
        self.actor.await.expect("el actor del núcleo terminó bien")
    }
}
