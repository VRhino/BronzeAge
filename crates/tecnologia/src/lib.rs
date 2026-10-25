//! Tecnología por Eras y Aedas (fase 6, sistema 6). Ver `Docs/Game/6_*` y `Docs/Mechanics/Tecnologia_Aedas.md` (TA).
//!
//! **Tanda 6A: reglas puras**, sin mundo ni núcleo (plan en `Docs/Arquitectura/Sistema6_Contraste_y_Plan.md`). Cada módulo trabaja sobre estructuras
//! que el núcleo guardará y aplicará (6B-6F): los instantes son `base::Instante` (ms de mundo), los costes `asentamiento::Costo` en unidades enteras
//! (milésimas múltiplo de 1.000, decisión C3b), los factores en ppm y los contadores `u64` en unidades. Nada consume el generador aleatorio
//! (TA §3: determinista y sin azar). Las tecnologías van por **índice del catálogo** ([`TecId`], el orden del bloque `TECNOLOGIAS`) y los
//! conjuntos de una Facción son mapas de bits ([`Conjunto`]).
//!
//! Módulos: `datos` (cifras de los bloques de TA; también [`datos::ReglasTecnologias`], que antes vivía en `militar`), `contador` (los 24
//! contadores y de qué vía sale cada uno), `estado` (estado del servidor: contadores con su siguiente umbral en O(1), logros, Eras, aparición,
//! descubridoras y crónica de descubrimientos), `hito` (las condiciones sobre una [`hito::VistaFaccion`] que da el núcleo, revelar),
//! `adopcion` (adoptar, vender, bonus de producción, tecnologías del campamento), `itinerantes` (objetivo, nacimiento, llegada y destinos),
//! `residentes` (cupo, llegada, salida, conquista), `epicas` (empezar, abandonar, hechos tipados y su avance) y `alijos` (ver y abrir).

pub mod adopcion;
pub mod alijos;
pub mod contador;
pub mod datos;
pub mod epicas;
pub mod estado;
pub mod hito;
pub mod itinerantes;
pub mod residentes;

pub use contador::Contador;
pub use datos::Reglas;
pub use estado::{EstadoTecnologia, TecnologiasFaccion};
pub use itinerantes::AedaItinerante;
pub use residentes::{AedaResidente, Residentes};
pub use epicas::Epica;

/// Índice de una tecnología en el catálogo (el orden del bloque `TECNOLOGIAS`). En el contrato viaja su id de cadena.
pub type TecId = u8;
pub type FaccionId = u32;
pub type AsentamientoId = u32;

/// Un conjunto de tecnologías por índice del catálogo, como mapa de bits: el catálogo de las Eras I-III tiene 26 y el mapa admite 64
/// (`datos::Reglas::cargar` lo comprueba al leer el bloque).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Conjunto(pub u64);

impl Conjunto {
    pub const VACIO: Conjunto = Conjunto(0);

    pub fn contiene(self, t: TecId) -> bool {
        self.0 >> t & 1 == 1
    }

    pub fn poner(&mut self, t: TecId) {
        self.0 |= 1 << t;
    }

    pub fn quitar(&mut self, t: TecId) {
        self.0 &= !(1 << t);
    }

    pub fn union(self, o: Conjunto) -> Conjunto {
        Conjunto(self.0 | o.0)
    }

    pub fn es_subconjunto_de(self, o: Conjunto) -> bool {
        self.0 & !o.0 == 0
    }

    pub fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Los índices, de menor a mayor (el orden del catálogo).
    pub fn iter(self) -> impl Iterator<Item = TecId> {
        (0..64u8).filter(move |&t| self.contiene(t))
    }
}

impl FromIterator<TecId> for Conjunto {
    fn from_iter<I: IntoIterator<Item = TecId>>(it: I) -> Self {
        let mut c = Conjunto::VACIO;
        it.into_iter().for_each(|t| c.poner(t));
        c
    }
}
