//! Población, economía y construcción de un asentamiento (fase 6, sistema 2). Ver `Docs/Game/4_*` y
//! `Docs/Mechanics/Economia_Construccion.md`. Reglas puras en enteros: el núcleo guarda el estado y programa los eventos.
//!
//! Este crate crece por subsistemas, en el orden de dependencia: recursos y datos, almacén, políticas, población (hecho);
//! después producción, mantenimiento, obras y cola, auto-construcción y ascenso.

pub mod almacen;
pub mod datos;
pub mod politicas;
pub mod poblacion;
pub mod recurso;

pub use almacen::Almacen;
pub use datos::Reglas;
pub use poblacion::{Entorno, Poblacion, Suceso};
pub use politicas::{Activador, Cargo, Politicas};
pub use recurso::{Costo, Recurso};
