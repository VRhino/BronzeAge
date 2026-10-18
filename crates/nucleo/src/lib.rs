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
//! - [`sesiones::lanzar`]: el actor que habla con `servidor` (foto, deltas, resultado causal, reanudación).

mod codec;
mod codec_ciudad;
mod codec_economia;
mod comando;
mod estado;
mod generadores;
mod nucleo;
pub mod economia;
pub mod espacial;
pub mod proyeccion;
pub mod sesiones;
pub mod urbanismo;

pub use codec::ErrorFormato;
pub use comando::{rechazos, AccionReloj, Comando, Peticion, Resultado, Respuesta};
pub use estado::{Asentamiento, Carga, CambioVersion, Ejercito, Estado, EventoNucleo, Reglas, ReglasEconomia, ReglasTerritorio, PRIO_ASENTAMIENTO, PRIO_LLEGADA, PRIO_UMBRAL};
pub use generadores::{Cx, GeneradorDeEventos};
pub use nucleo::{Ajustes, Arranque, InformeViabilidad, Metricas, Nucleo};

/// sha256 en hexadecimal minúsculas.
pub fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}
