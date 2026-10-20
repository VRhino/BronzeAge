//! El estado del combate dentro del núcleo (Combate, Generacion_Del_Mundo §13, fase 4D): los campamentos de bandidos del mapa, los plazos con los
//! que reaparecen, la guarnición de serie de los pueblos y lo que cada héroe ha destruido hace poco. Entra entero en el hash y en las fotos
//! (`codec_combate`). Las reglas puras viven en el crate `combate`; aquí solo está lo que el núcleo guarda y las claves de cola de sus plazos.
//!
//! Lo que sigue a una conquista no está aquí: la protección y la ocupación de una plaza son campos de la propia plaza
//! ([`Asentamiento`](crate::estado::Asentamiento)), porque cambian cómo se la ataca y cómo produce.
//!
//! Las claves de generación de los plazos llevan una etiqueta en los bits altos (como las del comercio y las de las columnas): un campamento de
//! bandidos, la conquista de una plaza, el anillo de un campamento de mercenarios y la guarnición de un pueblo comparten la tabla de la cola con
//! los ids de la partida.

use crate::estado_comercio::ETIQUETA;
use base::{Instante, Punto};
use militar::escuadra::Escuadra;
use std::collections::BTreeMap;

const ETIQUETA_BANDIDOS: u64 = 9 * ETIQUETA;
const ETIQUETA_CONQUISTA: u64 = 10 * ETIQUETA;
const ETIQUETA_ANILLO: u64 = 11 * ETIQUETA;
const ETIQUETA_GUARNICION: u64 = 12 * ETIQUETA;

/// Clave de entidad (de generación, de cola y de la rejilla de encuentros) de un campamento de bandidos.
pub fn entidad_bandidos(id: u64) -> u64 {
    ETIQUETA_BANDIDOS | id
}
/// ¿Es esta clave de entidad la de un campamento de bandidos? (la rejilla mezcla columnas, caravanas, plazas y bandidos).
pub fn es_entidad_bandidos(entidad: u64) -> bool {
    entidad & !(ETIQUETA - 1) == ETIQUETA_BANDIDOS
}
/// Clave de entidad de los plazos que deja la conquista de una plaza (fin de la protección y de la ocupación). Una conquista nueva sube su
/// generación y descarta los plazos de la anterior.
pub fn entidad_conquista(plaza: u64) -> u64 {
    ETIQUETA_CONQUISTA | plaza
}
/// Clave de entidad de la cita de aparición de bandidos en el anillo de un campamento de mercenarios (índice del artefacto).
pub fn entidad_anillo(campamento: u32) -> u64 {
    ETIQUETA_ANILLO | campamento as u64
}
/// Clave de entidad de la reposición de la guarnición de serie de un pueblo.
pub fn entidad_guarnicion(pueblo: u64) -> u64 {
    ETIQUETA_GUARNICION | pueblo
}

/// De dónde sale un campamento de bandidos (Generacion_Del_Mundo §13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrigenBandidos {
    /// El que toda plaza tiene siempre, en su bosque no reclamado más cercano; reaparece junto a ella.
    Plaza(u64),
    /// El del anillo de un campamento de mercenarios (índice del artefacto), según la demanda de sus residentes.
    Anillo(u32),
    /// El que el generador del mundo coloca junto a una veta de estaño u oro (índice de `bandidos_iniciales`); no reaparece.
    Veta(u32),
    /// El que la administración coloca (escenarios y pruebas); no reaparece.
    Admin,
}

/// Un campamento de bandidos: una entidad fija del mapa.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CampamentoBandidos {
    pub id: u64,
    pub posicion: Punto,
    /// 1 a 3: fija su poder y su oro (`CAMPAMENTOS_BANDIDOS.niveles`).
    pub nivel: u8,
    pub origen: OrigenBandidos,
    /// El bosque (índice del artefacto) sobre el que está, si lo hay: no se pone otro campamento encima.
    pub bosque: Option<u32>,
}

/// La guarnición de serie de un pueblo: los mineros sin dueño. Nace completa la primera vez que se la necesita y se repone gratis al cabo del
/// plazo de `repone_en`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuarnicionDePueblo {
    pub escuadra: Escuadra,
    /// Cuándo se repone, si está por debajo de su número de serie (hay un evento en la cola a ese instante).
    pub repone_en: Option<Instante>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EstadoCombate {
    /// Los campamentos de bandidos vivos, por id ascendente.
    pub bandidos: BTreeMap<u64, CampamentoBandidos>,
    /// Contador propio de ids de campamento (no gasta el de la partida: los ids de héroes y plazas no se mueven).
    pub siguiente_bandidos: u64,
    /// Plaza → cuándo vuelve a tener campamento tras destruirse el suyo (`Asentamiento.bandidosReaparecenEn`). Sin entrada, ya.
    pub reaparecen: BTreeMap<u64, Instante>,
    /// Campamento de mercenarios (índice) → desde cuándo puede aparecer otro en su anillo (`CampamentoMercenarios.bandidosEn`).
    pub anillos: BTreeMap<u32, Instante>,
    /// Pueblo → su guarnición de serie, la que ya se necesitó alguna vez.
    pub guarniciones: BTreeMap<u64, GuarnicionDePueblo>,
    /// Héroe → instantes en que destruyó campamentos de bandidos en la ventana de rendimientos decrecientes.
    pub destruidos: BTreeMap<u64, Vec<Instante>>,
}

impl Default for EstadoCombate {
    fn default() -> Self {
        Self { bandidos: BTreeMap::new(), siguiente_bandidos: 1, reaparecen: BTreeMap::new(), anillos: BTreeMap::new(), guarniciones: BTreeMap::new(), destruidos: BTreeMap::new() }
    }
}
