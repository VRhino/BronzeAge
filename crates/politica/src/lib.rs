//! Política y Facciones (fase 6, sistema 5). Ver `Docs/Game/0_*`, `Docs/Game/2_*` y `Docs/Mechanics/Diplomacia_Facciones.md`.
//!
//! **Tanda 5A: reglas puras**, sin mundo ni núcleo (plan en `Docs/Arquitectura/Sistema5_Contraste_y_Plan.md`). Cada módulo trabaja sobre estructuras
//! que el núcleo guardará y aplicará (5B-5F): los instantes son `base::Instante`, la experiencia va en milésimas, las cantidades de recursos en
//! **unidades enteras** con resto oculto (decisión C3b, `Consideraciones/Calibracion_Sistema4_Decisiones.md`) y no hay coma flotante en ninguna
//! decisión. La política no consume ninguna tirada del generador aleatorio. La reputación vive en `comercio::reputacion`: aquí solo se devuelven
//! los `Suceso`s que la mueven, y el núcleo los aplica.
//!
//! Módulos: `datos` (cifras de los bloques de Diplomacia_Facciones), `tipos` (ids, cargos, grupos de la puerta), `sigilo` (§3.8), `faccion` (§3.1-§3.4:
//! crear, entrar, salir con la sucesión, Rey y Embajador; Q7), `nivel` (§3.5: experiencia, nivel, cap y cupo), `cargos` (§3.4: cargos locales, Q4),
//! `residencia` (§4.1-§4.2: cooldown y cambio de residencia; Q8), `capital` (§3.7), `puerta` (§4.3, Q1), `relaciones` (§5: par, guerra con arrastre,
//! paz, ruptura, rebelión, desarme, vasalla conquistada, traición al aliado Q2, defensa del vasallo), `propuestas` (§2.4, §5.3, §6.1-§6.2),
//! `tributo` (§5.6, Q3), `liga` (§5.7), `titulos` (§7), `traslado` (§6.3: el plan de cambios) y `autorizacion` (Proyecciones §4.3 y §5.2: los predicados de la
//! matriz).

pub mod autorizacion;
pub mod capital;
pub mod cargos;
pub mod datos;
pub mod faccion;
pub mod liga;
pub mod nivel;
pub mod propuestas;
pub mod puerta;
pub mod relaciones;
pub mod residencia;
pub mod sigilo;
pub mod tipos;
pub mod titulos;
pub mod traslado;
pub mod tributo;

pub use datos::Reglas;
pub use faccion::{Faccion, Facciones};
pub use propuestas::{Propuesta, Propuestas};
pub use relaciones::{RelacionPolitica, Relaciones};
pub use tipos::*;

/// Todo el estado político que no cuelga de plazas ni de héroes: Facciones, relaciones y propuestas. El núcleo lo guarda tal cual (5B-5D).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Politica {
    pub facciones: Facciones,
    pub relaciones: Relaciones,
    pub propuestas: Propuestas,
}
