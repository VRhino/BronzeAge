//! El proceso del servidor (fase 7, tanda 7B): carga el artefacto de mundo, abre el núcleo persistente, lanza su actor y sirve en un solo puerto el WebSocket del contrato 05 y el
//! servicio de acceso (`acceso`). Lo usan los bots, el admin web y los clientes de jugador, siempre desde fuera del proceso.
//!
//! La carpeta de datos lleva `mundo/` (registro de eventos, fotos y crónica del núcleo) y `acceso/` (cuentas y sesiones). Un reinicio con la misma carpeta y el mismo artefacto sigue la
//! partida donde se quedó: el reloj de mundo continúa desde el último instante guardado y los tokens vigentes siguen valiendo.
//!
//! El mismo puerto sirve el artefacto de mundo por hash (`GET /v1/mundo/<hash>`, contrato 06) y la `Bienvenida` lo anuncia en `InfoMundo.url_artefacto` (ruta relativa al servidor).
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
use mundo::{Mundo, OpcionesValidacion};
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

/// El HTTP del puerto: el artefacto de mundo por hash y, lo demás, el servicio de acceso.
struct RutasHttp {
    acceso: Acceso,
    admin: admin::Admin,
    mundo: Option<(String, Arc<Vec<u8>>)>,
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
        if let Some(hash) = p.ruta.strip_prefix(RUTA_MUNDO) {
            let r = match (&self.mundo, p.metodo.as_str()) {
                (Some((h, bytes)), "GET") if h == hash => RespuestaHttp { estado: 200, tipo: "application/octet-stream", cuerpo: bytes.as_ref().clone() },
                (_, "GET") => RespuestaHttp::texto(404, "no hay artefacto con ese hash"),
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

/// Abre los datos de `datos` (o los crea), lanza el núcleo y escucha en `direccion` (puerto 0: uno libre). Con `artefacto`, el puerto lo sirve por su hash.
pub async fn arrancar(arranque: Arranque, artefacto: Option<Arc<Vec<u8>>>, datos: &Path, direccion: SocketAddr, acceso: acceso::Config) -> io::Result<(Servicio, Informe)> {
    let datos_de_juego = datos::datos_de_juego(&arranque.reglas);
    let semilla = arranque.semilla;
    let dir_mundo = datos.join("mundo");
    let n = tokio::task::block_in_place(|| Nucleo::con_persistencia(arranque, &dir_mundo, persistencia::Config::default()))?;
    let acceso = Acceso::abrir(&datos.join("acceso"), acceso)?;
    let informe = Informe { mundo_hash: n.mundo_hash().into(), version: n.version(), ahora: n.ahora(), recuperado: n.recuperado(), cuentas: acceso.cuentas() };
    let reloj = RelojCompartido::nuevo(n.ahora());
    let (tx, rx) = mpsc::channel(1024);
    let actor = sesiones::lanzar(n, reloj.clone(), rx);
    let mundo = artefacto.map(|b| (informe.mundo_hash.clone(), b));
    let config = Config {
        url_artefacto: mundo.is_some().then(|| RUTA_MUNDO.to_owned()),
        datos_de_juego: Some((datos_de_juego.version.clone(), format!("{RUTA_DATOS}/{}", datos_de_juego.version))),
        ..Config::default()
    };
    let admin = admin::Admin { acceso: acceso.clone(), nucleo: tx.clone(), carpeta: datos.to_owned(), mundo_hash: informe.mundo_hash.clone(), semilla };
    let http = Arc::new(RutasHttp { acceso: acceso.clone(), admin, mundo, datos: datos_de_juego });
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
