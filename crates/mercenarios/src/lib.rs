//! Mercenarios por contrato (entrada 48; reglas y parámetros en `Docs/Mechanics/Mercenarios_Por_Contrato.md`).
//!
//! Reglas puras, sin mundo ni reloj: el núcleo averigua lo que hace falta del mundo ([`Situacion`]), guarda [`EstadoContratos`] y aplica lo que estas funciones deciden.
//! Módulos: `datos` (los tres bloques de parámetros), `tipos` (contrato, orden, ficha, oferta), `contrato` (firma y tarifa), `orden` (alcance, admisión y rango de las
//! órdenes) y `progresion` (renombre, rangos, medallas, licencia y valoración).

pub mod contrato;
pub mod datos;
pub mod orden;
pub mod progresion;
pub mod tipos;

pub use datos::Datos;
pub use tipos::*;
