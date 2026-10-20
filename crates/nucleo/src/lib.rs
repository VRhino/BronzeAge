//! Núcleo de un solo hilo del mundo reactivo (fase 5): aplica órdenes y eventos en `(instante, prioridad, secuencia)`, sin
//! tick. Genérico: columnas con plan y víveres como ritmo; las reglas de juego completas llegan en la fase 6.
//!
//! - [`Nucleo`]: el bucle (modo acelerado y en vivo), las órdenes con idempotencia y `version_esperada`, la persistencia
//!   por registro de peticiones + fotos, y `hash_estado()` determinista.
//! - [`Estado`]: lo que entra en el hash y en las fotos. [`CambioVersion`]: el registro de cambios por versión.
//! - [`GeneradorDeEventos`]: el hueco para `encuentros` y `vision`.
//! - [`economia`]: cada plaza fundada lleva su economía (crate `asentamiento`) y avanza sola un minuto de mundo por evento
//!   (`Carga::Minuto`); aquí están el mapa, los yacimientos y los sitios que comparten el núcleo y el batch de población.
//! - [`urbanismo`]: la ciudad de cada plaza (crate `trazado`): dónde cae cada edificio, anclas, puestos del Mercado y reubicación de la Granja.
//! - [`EstadoComercio`]: el comercio (fases 3B-1 y 3B-2): trueques, órdenes de mercado y mostrador, flota de caravanas que viajan por el mapa
//!   (plan, peajes, reparto automático, red de caminos), mercado de mercenarios, reputación entre Facciones y sus plazos como eventos de prioridad 6 y 9 (`Carga::Comercio`, `Carga::Reputacion`); las reglas puras son el
//!   crate `comercio`. Las órdenes están en `Comando` (códigos 21-40) y el cierre por ruina o conquista, en `Nucleo`.
//! - Pueblos extractores (fase 3C): obra de mapa, extracción contra un almacén local y carro de acarreo hacia la plaza madre; las reglas
//!   puras son `asentamiento::pueblos` y `comercio::acarreo`, y las órdenes (códigos 45-48), en `Nucleo`.
//! - Héroes y tropa (fase 4B-1): [`EstadoMilitar`] guarda los héroes con sus escuadras, loadouts y almacén personal, los reclutas de los campamentos de
//!   mercenarios y las tecnologías que da la administración; reclutar en una plaza y en un campamento, la tropa prestada, la guarnición (que come en
//!   el paso de minuto de su plaza), la escolta de caravanas, conectarse y desconectarse (prioridad 1, `Carga::SalidaDelMundo`). Las reglas puras son el
//!   crate `militar` y las órdenes (códigos 41-44 y 49-68), `Nucleo`.
//! - [`sesiones::lanzar`]: el actor que habla con `servidor` (foto, deltas, resultado causal, reanudación).

mod codec;
mod codec_ciudad;
mod codec_comercio;
mod codec_economia;
mod codec_militar;
mod comando;
mod estado;
mod estado_comercio;
mod estado_militar;
mod generadores;
mod nucleo;
pub mod economia;
pub mod espacial;
pub mod proyeccion;
pub mod sesiones;
pub mod urbanismo;

pub use codec::ErrorFormato;
pub use comando::{rechazos, AccionReloj, Comando, PagarCon, Peticion, Resultado, Respuesta};
pub use estado::{Asentamiento, Carga, CambioVersion, CitaReputacion, Ejercito, Estado, EventoNucleo, Reglas, ReglasEconomia, ReglasTerritorio, SucesoComercio, PRIO_ASENTAMIENTO, PRIO_COMERCIO, PRIO_LLEGADA, PRIO_REPUTACION, PRIO_SALIDA, PRIO_UMBRAL};
pub use estado_comercio::{entidad_caravana, entidad_orden, entidad_reputacion, entidad_trueque, EstadoComercio};
pub use estado_militar::{EstadoMilitar, HeroeNucleo};
pub use generadores::{Cx, GeneradorDeEventos};
pub use nucleo::{Ajustes, Arranque, InformeViabilidad, Metricas, Nucleo};

/// sha256 en hexadecimal minúsculas.
pub fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}
