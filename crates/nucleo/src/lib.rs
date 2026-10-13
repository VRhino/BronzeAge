//! Núcleo de un solo hilo del mundo reactivo (fase 5): aplica órdenes y eventos en `(instante, prioridad, secuencia)`, sin
//! tick. Genérico: columnas con plan y víveres como ritmo; las reglas de juego completas llegan en la fase 6.
//!
//! - [`Nucleo`]: el bucle (modo acelerado y en vivo), las órdenes con idempotencia y `version_esperada`, la persistencia
//!   por registro de peticiones + fotos, y `hash_estado()` determinista.
//! - [`Estado`]: lo que entra en el hash y en las fotos. [`CambioVersion`]: el registro de cambios por versión.
//! - [`GeneradorDeEventos`]: el hueco para `encuentros` y `vision`.
//! - [`sesiones::lanzar`]: el actor que habla con `servidor` (foto, deltas, resultado causal, reanudación).

mod codec;
mod comando;
mod estado;
mod generadores;
mod nucleo;
pub mod espacial;
pub mod proyeccion;
pub mod sesiones;

pub use codec::ErrorFormato;
pub use comando::{rechazos, AccionReloj, Comando, Peticion, Resultado, Respuesta};
pub use estado::{Asentamiento, Carga, CambioVersion, Ejercito, Estado, EventoNucleo, Reglas, ReglasTerritorio, PRIO_LLEGADA, PRIO_UMBRAL, PRIO_ZONA};
pub use generadores::{Cx, GeneradorDeEventos};
pub use nucleo::{Ajustes, Arranque, InformeViabilidad, Metricas, Nucleo};

/// sha256 en hexadecimal minúsculas.
pub fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}
