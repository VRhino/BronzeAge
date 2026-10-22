//! Población, economía y construcción de un asentamiento (fase 6, sistema 2). Ver `Docs/Game/4_*` y
//! `Docs/Mechanics/Economia_Construccion.md`. Reglas puras en enteros: el núcleo guarda el estado y programa los eventos.
//!
//! Este crate crece por subsistemas, en el orden de dependencia: recursos y datos, almacén, políticas, población, producción,
//! mantenimiento, obras y cola, auto-construcción, ascenso y, fuera de la ciudad, los pueblos extractores, las avanzadas y sus obras de mapa.

pub mod almacen;
pub mod ascenso;
pub mod avanzadas;
pub mod asentamiento;
pub mod catalogo;
pub mod datos;
pub mod edificio;
pub mod mantenimiento;
pub mod necesidades;
pub mod nivel;
pub mod obras;
pub mod politicas;
pub mod poblacion;
pub mod produccion;
pub mod pueblos;
pub mod recurso;

pub use almacen::Almacen;
pub use catalogo::{Catalogo, Tipo};
pub use datos::Reglas;
pub use poblacion::{Entorno, Poblacion, Suceso};
pub use politicas::{Activador, Cargo, Politicas};
pub use recurso::{Costo, Recurso};
