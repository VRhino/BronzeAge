//! Bots de BronzeAge: juegan desde fuera del motor, solo por el contrato 05, como cualquier jugador.

pub mod acceso;
pub mod azar;
pub mod balance;
pub mod cerebro;
pub mod control;
pub mod datos;
pub mod libro;
pub mod llegadas;
pub mod mapa;
pub mod mercenario;
pub mod plaza;
pub mod politica;
pub mod puerto;
pub mod runner;
pub mod tablero;

pub use acceso::Credenciales;
pub use puerto::Puerto;
pub use runner::{Cerebro, Contexto, Proceso};

/// Por qué no salió algo.
#[derive(Debug)]
pub enum Fallo {
    /// La red o el servidor (corte, hueco de secuencia, sin respuesta): el puerto lo intenta reconectando.
    Red(String),
    /// Un `ErrorSesion` fatal del servidor que reconectar no arregla (`rol_no_permitido`, `version_protocolo`...).
    Sesion(contrato::v1::ErrorSesion),
    /// El servicio de acceso dijo que no (`codigo_incorrecto`, `nick_repetido`, `credenciales_invalidas`...).
    Acceso { estado: u16, codigo: String },
    /// La `VISTA` no se abre (`vista.invalida`, `vista.sin_derecho`).
    Vista(String),
    /// El registro del runner no se lee o no se escribe, o no es de esta partida.
    Registro(String),
}
