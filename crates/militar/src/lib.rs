//! Héroe, tropa y columna (fase 6, sistema 4). Ver `Docs/Game/5_*` y `Docs/Mechanics/{Heroe_Y_Tropa, Movimiento_Ejercitos, Presencia_Y_Convocatoria,
//! Campamentos_Mercenarios}.md`.
//!
//! **Fase 4A-1: reglas puras**, sin mundo ni núcleo, en enteros: instantes `base::Instante`, cantidades y distancias en milésimas, tasas en
//! ppm, sin coma flotante en ninguna decisión. Este crate no consume ninguna tirada del generador aleatorio (eso es del combate, 4A-2) y no
//! depende del núcleo: el núcleo guarda el estado, programa los eventos y llama a estas funciones.
//!
//! Módulos, en orden de dependencia: `datos` (cifras de los bloques, **vigentes** sin correcciones en el código), `catalogo` (las 27
//! tropas), `escuadra` (nivel, moral y deserción), `liderazgo` (Doc 5.11), `presencia` (ubicación y conexión), `heroe` (progresión, loadouts,
//! orden de escuadras y herido), `guarnicion` (cupo y asignación), `reclutamiento` (en una plaza) y `mercenarios` (en un campamento, y la
//! tropa prestada), `racion` (víveres como ritmo), `columna` (unirse, separarse, formar, velocidad), `convocatoria`, `persecucion` e `inspeccion`.

pub mod catalogo;
pub mod columna;
pub mod convocatoria;
pub mod datos;
pub mod escuadra;
pub mod guarnicion;
pub mod heroe;
pub mod inspeccion;
pub mod liderazgo;
pub mod mercenarios;
pub mod persecucion;
pub mod presencia;
pub mod racion;
pub mod reclutamiento;
pub mod tipos;

pub use catalogo::{Origen, Tropa};
pub use datos::Reglas;
pub use tipos::*;
