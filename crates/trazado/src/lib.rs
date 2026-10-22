//! Trazado urbano (fase 6, sistema 2). Ver `Docs/Mechanics/Trazado_Urbano.md` (T).
//!
//! Decide **dónde cae cada edificio** de un asentamiento: espacio local plano, rejilla de celdas de 3 u.l., red de calles
//! derivada, anclas con sus núcleos de satélites, afueras (granjas y corrales), perfiles de orden, fusión de celdas y fundación.
//! Reglas puras, deterministas y en enteros: sin estado de partida ni reloj. El núcleo guarda las piezas (y los recintos) y
//! reconstruye la [`Ciudad`] con [`Ciudad::desde_partes`]; todo lo demás (suelo, red) es derivado.
//!
//! # Flujo típico
//! 1. [`Ciudad::fundar`] al fundar el asentamiento.
//! 2. Para un edificio con barrio: [`Ciudad::asegurar_ancla_para`] y luego [`Ciudad::siguiente_sitio`] /
//!    [`Ciudad::colocar_siguiente`] con un [`Pedido`] (tipo, categoría y huella los pone el catálogo).
//! 3. Con recintos de muralla: [`Ciudad::set_recintos`] (muros bloquean, puertas son transitables).
//! 4. Para dibujar: [`Ciudad::piezas`], [`Ciudad::calles_rectangulos`], [`Ciudad::caminos_rectangulos`].
//! 5. Para trazar la muralla: [`trazar_recinto`] (el anillo, sus puertas y torres), y con el anillo congelado [`interior_de`],
//!    [`contiene_a`], [`edificios_extramuros`] y [`marcar_torres`] (módulo `muralla`; Murallas.md).
//!
//! # Versión de geometría
//! [`LAYOUT_VERSION`] = 3 (T usaba 2). `pseudoAleatorio` con `sin` (T §3) se sustituyó por un hash entero (FNV-1a de 64 bits
//! con mezcla SplitMix64) sobre (semilla, sal, enteros), y las ranuras de ancla usan una tabla entera de senos: no se reproduce
//! el trazado de TypeScript. El perfil activo solo afecta a colocaciones nuevas; nada se recoloca nunca.
//!
//! # No cubierto aquí
//! Coste y levantamiento de recintos (Murallas.md): el llamador pasa las celdas ya trazadas ([`Recinto`], con
//! [`Recinto::desde_celdas`]); las tarifas, la obra celda a celda y la defensa viven en `asentamiento` y `nucleo`. La tabla de huellas por
//! tipo (vive en `nucleo::urbanismo`, leída de los bloques de T §2.4) y los cachés LRU de T §9 (la red es incremental y el suelo es un
//! estado de la `Ciudad`; el núcleo cachea por plaza las preguntas «¿cabe?»).

mod anclas;
mod azar;
mod ciudad;
mod colocacion;
mod fundacion;
mod geom;
mod muralla;
mod tipos;

pub use azar::{cos_sen_1000, hash, sorteo};
pub use ciudad::{Ciudad, MITAD};
pub use fundacion::{fusionar_celdas, EntradaFundacion, RectLocal, TipoPieza};
pub use geom::{Celda, Huella, Rect, CELDA_UL, HUELLA_POR_DEFECTO};
pub use muralla::{
    contiene_a, edificios_extramuros, interior_de, marcar_torres, trazar_recinto, CeldaMuro, ClaseCelda, ErrorMuralla, Interior,
    ParametrosMuralla, Trazo,
};
pub use tipos::*;
