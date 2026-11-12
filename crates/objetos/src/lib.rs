//! Objetos del héroe (entradas 70 y 71). Ver `Docs/Mechanics/Objetos_Del_Heroe.md` y los diseños `Objetos_Del_Heroe.md`, `Catalogo_De_Objetos.md` y
//! `Clases_De_Heroe.md` de `Docs/Disenos_Por_Implementar/`.
//!
//! **Reglas puras**, en enteros y sin héroes, plazas ni órdenes (D11): el núcleo guarda la mochila, el almacén de objetos y el equipo de cada héroe,
//! comprueba dónde está y llama a estas funciones. El azar es solo el `base::Rng` que pasa quien llama (la calidad de la forja, el cofre y el botín);
//! si una tecnología está adoptada se lo dice quien llama con una función.
//!
//! Módulos: `datos` (el catálogo: fichas, clases y tablas de botín, comprobados al cargar), `inventario` (la mochila y el almacén: meter, sacar,
//! pasar y entregar lo que llega sin pedirlo), `equipo` (los siete huecos, la clase del arma, los atributos, el equipo inicial, equipar y quitar),
//! `tienda` (comprar y vender), `forja`, `consumible` (usar uno) y `botin` (tirar una tabla).

pub mod botin;
pub mod consumible;
pub mod datos;
pub mod equipo;
pub mod forja;
pub mod inventario;
pub mod tienda;

pub use datos::Catalogo;
pub use equipo::Equipo;
pub use inventario::{entregar, Destino, Inventario, Pieza};
