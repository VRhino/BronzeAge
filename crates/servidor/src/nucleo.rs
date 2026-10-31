//! La interfaz entre el servidor de sesiones y el núcleo (que se escribe aparte), más la configuración y el reloj compartido.

use std::{
    future::Future,
    io,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::{Duration, Instant as Pared},
};

use base::{Instante, RelojMundo};
use contrato::v1::{
    Aviso, Bienvenida, Cronica, Delta, ErrorSesion, Foto, Hola, MensajeDeServidorDeBatalla, MensajeParaServidorDeBatalla, Orden,
    PedirCronica, Rectangulo, Reloj, ResultadoOrden, Revocado, TipoCliente,
};
use tokio::sync::{mpsc, oneshot};

/// Identificador de sesión que asigna el servidor (único mientras el proceso vive). El núcleo lo usa para direccionar.
pub type SesionId = u64;

/// Códigos de `ErrorSesion.codigo` que emite este crate.
pub mod codigos {
    pub const TOKEN_INVALIDO: &str = "token_invalido";
    pub const VERSION_PROTOCOLO: &str = "version_protocolo";
    pub const DEMASIADAS_ORDENES: &str = "demasiadas_ordenes";
    pub const ROL_NO_PERMITIDO: &str = "rol_no_permitido";
    pub const INTERNO: &str = "interno";
    /// Aditivo: el primer mensaje del cliente no fue `Hola`.
    pub const HOLA_ESPERADO: &str = "hola_esperado";
    /// Un segundo `Hola` en una sesión abierta (no fatal): reanudar es reconectar.
    pub const HOLA_REPETIDO: &str = "hola_repetido";
    /// Aditivo: trama no decodificable, de texto, o `MensajeDeCliente` vacío.
    pub const MENSAJE_INVALIDO: &str = "mensaje_invalido";
}

/// Quién es el dueño del token. Lo devuelve el [`Autenticador`].
#[derive(Clone, Debug)]
pub struct Identidad {
    /// Una sola sesión viva por `cliente_id`.
    pub cliente_id: String,
    /// Tipos de cliente que este token puede abrir (`Hola.cliente` debe estar aquí, si no `rol_no_permitido`).
    pub roles: Vec<TipoCliente>,
    /// El rol técnico de su membresía en la partida (Proyecciones_Y_Autorizacion §2.2; sistema 5, tanda 5E).
    pub rol: RolTecnico,
}

/// El rol técnico de la membresía de una cuenta en la partida (Proyecciones_Y_Autorizacion §2.2): un eje distinto del de los cargos del juego. Solo matiza las sesiones de administración
/// (`CLIENTE_ADMIN_WEB`): el **moderador** ve el ámbito `ADMIN` y no da ninguna orden; el administrador de la partida y el global dan las de administración. Para un jugador no cuenta: lo que
/// puede lo dice la matriz de autorización del núcleo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RolTecnico {
    #[default]
    Jugador,
    AdministradorPartida,
    Moderador,
    AdministradorGlobal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorAcceso {
    /// Inválido o caducado → `token_invalido`.
    Invalido,
    /// El servicio de acceso no responde → `interno`.
    NoDisponible,
}

/// Servicio de acceso (fase 3). Asíncrona porque normalmente consulta otro servicio.
pub trait Autenticador: Send + Sync + 'static {
    fn autenticar(&self, token: &str) -> impl Future<Output = Result<Identidad, ErrorAcceso>> + Send;
}

/// Del servidor al núcleo.
#[derive(Debug)]
pub enum ComandoSesion {
    /// Sesión autenticada que pide entrar. El núcleo decide la reanudación con `hola.versiones_conocidas`, contesta por
    /// `respuesta` con la `Bienvenida` (o un `ErrorSesion`; el servidor lo manda y cierra) y empieza a escribir en `difusion`.
    /// El servidor sobrescribe `Bienvenida.maximo_ordenes_pendientes` con su configuración.
    Conecta {
        sesion: SesionId,
        identidad: Identidad,
        hola: Hola,
        difusion: mpsc::Sender<Difusion>,
        respuesta: oneshot::Sender<Result<Bienvenida, ErrorSesion>>,
    },
    Orden { sesion: SesionId, orden: Orden },
    /// `CambiarVista`: el interés en el mapa. El núcleo abre (o rehace) el ámbito `VISTA` con una `Foto` nueva, o avisa de por qué no puede (`Aviso{vista.invalida | vista.sin_derecho}`).
    Vista { sesion: SesionId, rect: Rectangulo },
    /// Mensaje de un servidor de batalla (solo si la sesión es `CLIENTE_SERVIDOR_BATALLA`).
    Batalla { sesion: SesionId, mensaje: MensajeDeServidorDeBatalla },
    /// `PedirCronica`: una página de la crónica. El núcleo la contesta con `Difusion::Cronica` (cuenta como pendiente, como una orden) sin leerla en su hilo.
    Cronica { sesion: SesionId, pedir: PedirCronica },
    /// La sesión terminó (cliente, red, expulsión o error). Llega siempre tras un `Conecta`, aceptado o no: el núcleo ignora ids que no conoce.
    Desconecta { sesion: SesionId },
    /// Del servicio, no de una sesión: lo que pide la administración del proceso (las rutas `/v1/admin/...`, fase 7).
    Admin(PeticionAdmin),
}

/// Lo que la administración del proceso pide al núcleo. Va por el mismo canal que las órdenes, así que se atiende entre dos órdenes.
#[derive(Debug)]
pub enum PeticionAdmin {
    /// La versión y el instante de ahora.
    Estado { respuesta: oneshot::Sender<EstadoNucleo> },
    /// Una foto ahora y la copia de la carpeta del núcleo en `destino`, con el núcleo esperando mientras copia (la copia es coherente: foto, registro y crónica).
    Respaldo { destino: PathBuf, respuesta: oneshot::Sender<io::Result<EstadoNucleo>> },
    /// El catálogo vigente de las misiones recurrentes (mecánica 55), en el JSON que acepta `fijar_catalogo_misiones`.
    CatalogoMisiones { respuesta: oneshot::Sender<String> },
}

/// La versión y el instante del núcleo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EstadoNucleo {
    pub version: u64,
    pub instante: Instante,
}

/// Del núcleo a una sesión, ya construido. El servidor solo numera, comprime y envía, en el orden recibido.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // mensajes Protobuf: se mueven una vez por canal, no vale la pena el Box
pub enum Difusion {
    Foto(Foto),
    Delta(Delta),
    Resultado(ResultadoOrden),
    Revocado(Revocado),
    Aviso(Aviso),
    Batalla(MensajeParaServidorDeBatalla),
    /// La respuesta a un `PedirCronica`.
    Cronica(Cronica),
    /// Error de sesión originado en el núcleo; si es `fatal`, la sesión se cierra tras enviarlo.
    Cerrar(ErrorSesion),
}

/// Parámetros del servidor.
#[derive(Clone, Debug)]
pub struct Config {
    pub max_ordenes_pendientes: u32,
    /// Tiempo máximo para recibir `Hola` y que el núcleo conteste.
    pub timeout_hola: Duration,
    /// Tiempo máximo para escribir una trama a un cliente; si lo supera, la sesión se cierra (cliente lento).
    pub timeout_escritura: Duration,
    /// Capacidad del canal núcleo → sesión.
    pub capacidad_difusion: usize,
    /// Tamaño máximo de una trama del cliente, ya descomprimida.
    pub max_trama_cliente: usize,
    /// Tamaño mínimo (bytes) de una trama saliente para comprimirla, si el cliente negoció zstd.
    pub umbral_compresion: usize,
    pub nivel_zstd: i32,
    /// Si el mismo puerto sirve el artefacto de mundo: el prefijo de `InfoMundo.url_artefacto` en la `Bienvenida`, al que se le añade el hash
    /// (`/v1/mundo/`: una ruta relativa al servidor). `None`: el núcleo decide (hoy la deja vacía).
    pub url_artefacto: Option<String>,
    /// Si el mismo puerto sirve el archivo común de datos de juego: su versión y su ruta (`InfoMundo.version_datos_de_juego` y `url_datos_de_juego` de la `Bienvenida`).
    pub datos_de_juego: Option<(String, String)>,
    /// Si el mismo puerto sirve el paquete de relieve (06 §8): su hash y su ruta (`InfoMundo.hash_relieve` y `url_relieve` de la `Bienvenida`).
    pub relieve: Option<(String, String)>,
    /// Los nombres de los héroes que la sesión añade a cada mensaje (`MensajeDeServidor.nombres_heroes`); el núcleo los apunta (`Nucleo::conectar_nombres`).
    pub nombres: crate::NombresHeroes,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_ordenes_pendientes: 64,
            timeout_hola: Duration::from_secs(5),
            timeout_escritura: Duration::from_secs(10),
            capacidad_difusion: 1024,
            max_trama_cliente: 1 << 20,
            umbral_compresion: 256,
            nivel_zstd: 3,
            url_artefacto: None,
            datos_de_juego: None,
            relieve: None,
            nombres: crate::NombresHeroes::default(),
        }
    }
}

/// El reloj de mundo compartido entre el núcleo (que lo gobierna) y las sesiones (que lo sirven en `Pong`).
/// «Pared» es el tiempo monótono en ms desde que se creó este reloj: el núcleo debe usar la misma noción.
#[derive(Clone)]
pub struct RelojCompartido {
    reloj: Arc<RwLock<RelojMundo>>,
    origen: Pared,
}

impl RelojCompartido {
    /// Reloj con `mundo` como instante actual, a ritmo normal.
    pub fn nuevo(mundo: Instante) -> Self {
        Self { reloj: Arc::new(RwLock::new(RelojMundo::nuevo(0, mundo))), origen: Pared::now() }
    }
    pub fn pared_ms(&self) -> i64 {
        self.origen.elapsed().as_millis() as i64
    }
    pub fn mundo_ahora(&self) -> Instante {
        self.con(|r, pared| r.mundo_en(pared))
    }
    /// Lee o muta el reloj; la función recibe también la pared actual.
    pub fn con<R>(&self, f: impl FnOnce(&mut RelojMundo, i64) -> R) -> R {
        let pared = self.pared_ms();
        let mut g = self.reloj.write().unwrap_or_else(|e| e.into_inner());
        f(&mut g, pared)
    }
    /// El `Reloj` Protobuf en este instante.
    pub fn reloj(&self) -> Reloj {
        self.con(|r, pared| Reloj {
            mundo_ms: r.mundo_en(pared),
            factor_milesimas: r.factor_milesimas(),
            pausado: r.pausado(),
            servidor_ms: pared,
        })
    }
}
