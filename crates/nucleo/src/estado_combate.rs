//! El estado del combate dentro del núcleo (Combate, Generacion_Del_Mundo §13, fases 4D y 4F): los campamentos de bandidos del mapa, los plazos con los
//! que reaparecen, la guarnición de serie de los pueblos y de las avanzadas, las ruinas de ciudadela sin dueño y lo que cada héroe ha destruido hace poco. Entra entero en el hash y en las fotos
//! (`codec_combate`). Las reglas puras viven en el crate `combate`; aquí solo está lo que el núcleo guarda y las claves de cola de sus plazos.
//!
//! Lo que sigue a una conquista no está aquí: la protección y la ocupación de una plaza son campos de la propia plaza
//! ([`Asentamiento`](crate::estado::Asentamiento)), porque cambian cómo se la ataca y cómo produce.
//!
//! Las claves de generación de los plazos llevan una etiqueta en los bits altos (como las del comercio y las de las columnas): un campamento de
//! bandidos, la conquista de una plaza, el anillo de un campamento de mercenarios y la guarnición de un pueblo comparten la tabla de la cola con
//! los ids de la partida.

use crate::estado_comercio::ETIQUETA;
use asentamiento::avanzadas::Avanzada;
use base::{Instante, Punto};
use militar::escuadra::Escuadra;
use militar::Tropa;
use std::collections::BTreeMap;

const ETIQUETA_BANDIDOS: u64 = 9 * ETIQUETA;
const ETIQUETA_CONQUISTA: u64 = 10 * ETIQUETA;
const ETIQUETA_ANILLO: u64 = 11 * ETIQUETA;
const ETIQUETA_GUARNICION: u64 = 12 * ETIQUETA;
const ETIQUETA_AVANZADA: u64 = 14 * ETIQUETA;
const ETIQUETA_OBRA_DE_AVANZADA: u64 = 15 * ETIQUETA;

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

/// Clave de entidad (de generación, de cola y de la rejilla de encuentros) de una avanzada: sus plazos (fin de la invulnerabilidad, reposición de su
/// guarnición de serie) y su sitio en el espacio. Cambiar de estado la sube y descarta los plazos del anterior.
pub fn entidad_avanzada(id: u64) -> u64 {
    ETIQUETA_AVANZADA | id
}
/// Clave de entidad de la cita de fin de la obra de una avanzada (primera obra, mejora o reparación). Aparte de [`entidad_avanzada`] para poder cancelarla sin llevarse
/// la reposición de la guarnición ni el fin de la invulnerabilidad: cancelar la obra, o perderla con la avanzada, sube su generación.
pub fn entidad_obra_de_avanzada(id: u64) -> u64 {
    ETIQUETA_OBRA_DE_AVANZADA | id
}
/// ¿Es esta clave de entidad la de una avanzada? (la rejilla mezcla columnas, caravanas, plazas, bandidos y avanzadas).
pub fn es_entidad_avanzada(entidad: u64) -> bool {
    entidad & !(ETIQUETA - 1) == ETIQUETA_AVANZADA
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

/// Lo que la IA y el Gobernador ponen en una avanzada (Doc 5.17): las tropas que eligió para cada escalón y la guarnición de serie, que nace completa la
/// primera vez que se la necesita y se repone gratis a las `reposicionMinutos` de quedar incompleta. Solo existe para una avanzada a la que se ha elegido
/// tropa o que ha tenido que defenderse: una avanzada sin tocar no ocupa nada.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GuarnicionDeAvanzada {
    /// Escalón → tropa elegida por el Gobernador de la madre; el escalón sin elección usa la tropa por defecto.
    pub elegidas: BTreeMap<u8, Tropa>,
    /// Las escuadras de serie, sin dueño, en el orden de `combate::avanzada::composicion`. Vacío mientras no se ha necesitado.
    pub escuadras: Vec<Escuadra>,
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
    /// Avanzada → su guarnición de serie y las tropas elegidas.
    pub avanzadas: BTreeMap<u64, GuarnicionDeAvanzada>,
    /// Ruinas de ciudadela sin dueño (su plaza madre desapareció), por id: quien tenga después su zona puede repararlas. Las demás avanzadas viven en
    /// su plaza madre.
    pub huerfanas: BTreeMap<u64, Avanzada>,
    /// Héroe → instantes en que destruyó campamentos de bandidos en la ventana de rendimientos decrecientes.
    pub destruidos: BTreeMap<u64, Vec<Instante>>,
}

impl Default for EstadoCombate {
    fn default() -> Self {
        Self { bandidos: BTreeMap::new(), siguiente_bandidos: 1, reaparecen: BTreeMap::new(), anillos: BTreeMap::new(), guarniciones: BTreeMap::new(), avanzadas: BTreeMap::new(), huerfanas: BTreeMap::new(), destruidos: BTreeMap::new() }
    }
}
