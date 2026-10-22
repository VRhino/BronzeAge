//! Combate con números y ciclo de la batalla de Unity (fase 6, sistema 4). Ver `Docs/Game/5_*` y `Docs/Mechanics/{Combate, Batalla_Unity_Ciclo}.md`.
//!
//! **Fase 4A-2: reglas puras**, sin mundo ni núcleo, en enteros: poder en milésimas, factores en ppm, instantes `base::Instante`, sin coma flotante
//! en ninguna decisión. Reutiliza los tipos de `militar` (`Escuadra`, `Heroe`, `Carga`…), de `comercio` (`Caravana`) y de `asentamiento`
//! (`Asentamiento`, `Almacen`). El azar es el `base::Rng` de la partida, con las tiradas documentadas en `Combate.md`; las ramas sin combate no
//! lo tocan. El núcleo guarda el estado, programa los eventos y llama a estas funciones.
//!
//! **Unity decide lo de dentro de la partida**: aquí van el combate numérico (sin servidores de batalla) y las consecuencias,
//! que son las mismas por los dos caminos.
//!
//! Módulos, en orden de dependencia: `datos` (cifras de los bloques, **vigentes** sin correcciones en el código), `resolucion` (poder, tirada,
//! `resolver_combate`, bajas, chatarra), `seleccion` (el tope de héroes por bando), `botin` (mitad del carro y reparto por rondas), `asedio`,
//! `caravana` (asalto A/B), `bandidos`, `pueblo` (saqueo), `avanzada`, `conquista` (saqueo de la plaza y desalojo) y el ciclo de Unity:
//! `ciclo` (la batalla como máquina de estados), `resultado` (el checklist y los efectos) y `union` (a qué bando se une quien llega).

pub mod asedio;
pub mod avanzada;
pub mod bandidos;
pub mod botin;
pub mod caravana;
pub mod ciclo;
pub mod conquista;
pub mod datos;
pub mod pueblo;
pub mod resolucion;
pub mod resultado;
pub mod seleccion;
pub mod union;

pub use datos::Reglas;
pub use resolucion::{Bando, Lado};
