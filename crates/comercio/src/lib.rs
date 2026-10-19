//! Economía y comercio (fase 6, sistema 3). Ver `Docs/Game/3_*` y `Docs/Mechanics/Comercio_Caravanas.md`.
//!
//! **Fase 3A: reglas puras y máquinas de estado**, sin mundo ni núcleo. Cada módulo trabaja sobre estructuras que el núcleo
//! guardará y programará (fase 3B): los instantes son `base::Instante`, las cantidades milésimas, las tasas ppm, y no hay coma flotante
//! en ninguna decisión. El comercio no consume ninguna tirada del generador aleatorio.
//!
//! Módulos, en orden de dependencia: `datos` (cifras de los bloques), `precio` (§3), `reputacion` (§4.6), `trueque` (§4), `reparto`
//! (§4.3), `orden_mercado` (§8.1-8.2) y `mostrador` (§8.3), `viaje` (§6: planes y peajes), `caravana` (§5-§6: la entidad y su máquina
//! de estados), `flota` (§5.1: cupos y políticas), `entrega` (§4.4), `red_caminos` (§7.4), `mercenarios` (§9), `ruina` (§10, ruina y
//! conquista), `acarreo` (carro de acarreo, Doc 3.13.8) y `evento` (§12).

pub mod acarreo;
pub mod caravana;
pub mod datos;
pub mod entrega;
pub mod evento;
pub mod flota;
pub mod mercenarios;
pub mod mostrador;
pub mod orden_mercado;
pub mod precio;
pub mod red_caminos;
pub mod reparto;
pub mod reputacion;
pub mod ruina;
pub mod tipos;
pub mod trueque;
pub mod viaje;

pub use datos::Reglas;
pub use tipos::*;
