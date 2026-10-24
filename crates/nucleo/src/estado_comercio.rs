//! El estado del comercio dentro del núcleo (Comercio_Caravanas §2): trueques, libro de órdenes de mercado, flota de caravanas, red de
//! caminos, mercado de mercenarios, reputación de las Facciones y los contadores de ids. Entra entero en el hash y en las fotos
//! (`codec_comercio`). Las reglas puras viven en el crate `comercio`; aquí solo está lo que el núcleo guarda y las claves con las que sus
//! eventos se invalidan en la cola.

use base::Instante;
use comercio::caravana::{Caravana, TipoCaravana};
use comercio::mercenarios::{MercadoMercenario, StockCampamento};
use comercio::orden_mercado::LibroOrdenes;
use comercio::red_caminos::RedCaminos;
use comercio::reputacion::Reputacion;
use comercio::trueque::AcuerdoTrueque;
use comercio::{AcuerdoId, CaravanaId, OrdenId};
use std::collections::BTreeMap;

/// Los ids de comercio (trueques, órdenes, caravanas) y las Facciones comparten con los ejércitos y las plazas la tabla de generaciones
/// de la cola (`u64`): cada clase lleva una etiqueta en los bits altos para que sus claves no choquen con los ids de la partida. Las etiquetas 1 a
/// 5 son de este módulo; las 6 a 8 (formaciones y peticiones de unión), de `estado_militar`.
pub(crate) const ETIQUETA: u64 = 1 << 56;
const ETIQUETA_TRUEQUE: u64 = ETIQUETA;
const ETIQUETA_ORDEN: u64 = 2 * ETIQUETA;
const ETIQUETA_CARAVANA: u64 = 3 * ETIQUETA;
const ETIQUETA_REPUTACION: u64 = 4 * ETIQUETA;
const ETIQUETA_MERCENARIOS: u64 = 5 * ETIQUETA;

/// Clave de entidad (de generación y de cola) de un trueque.
pub fn entidad_trueque(id: AcuerdoId) -> u64 {
    ETIQUETA_TRUEQUE | id
}
/// Clave de entidad de una orden de mercado.
pub fn entidad_orden(id: OrdenId) -> u64 {
    ETIQUETA_ORDEN | id
}
/// Clave de entidad de una caravana. Es también su id en la rejilla de encuentros (`encuentros::Rejilla`), que comparte con las columnas: la
/// etiqueta evita que choque con el id de un ejército.
pub fn entidad_caravana(id: CaravanaId) -> u64 {
    ETIQUETA_CARAVANA | id
}
/// Clave de entidad de la reputación de una Facción.
pub fn entidad_reputacion(faccion: u32) -> u64 {
    ETIQUETA_REPUTACION | faccion as u64
}
/// Clave de entidad del mercado de mercenarios (es uno solo para todo el mundo: su reposición es global).
pub fn entidad_mercenarios() -> u64 {
    ETIQUETA_MERCENARIOS
}

/// El id de comercio que lleva una clave de entidad (sin la etiqueta).
pub(crate) fn id_de_entidad(entidad: u64) -> u64 {
    entidad & (ETIQUETA - 1)
}

/// ¿Es esta clave de entidad la de una caravana? (la rejilla de encuentros mezcla columnas y caravanas).
pub fn es_entidad_caravana(entidad: u64) -> bool {
    entidad & !(ETIQUETA - 1) == ETIQUETA_CARAVANA
}

/// El mercado de los campamentos de mercenarios (Comercio_Caravanas §9, §2.5). Los campamentos son los del artefacto de mundo (por su índice);
/// solo se guarda el stock de los que ya se tocaron: el que falta tiene el inicial. La residencia, el almacén personal y el cupo diario de cada
/// comprador son de su héroe (`estado_militar`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EstadoMercenarios {
    /// Lo comerciado entre Facciones desde la última reposición y cuándo toca la siguiente (global).
    pub mercado: MercadoMercenario,
    /// Stock de los campamentos tocados (índice del artefacto).
    pub stock: BTreeMap<u32, StockCampamento>,
    /// Instante del evento de reposición pendiente, si lo hay: sin él no se programa otro.
    pub pulso: Option<Instante>,
}

impl Default for EstadoMercenarios {
    fn default() -> Self {
        Self { mercado: MercadoMercenario { contadores: BTreeMap::new(), repone: BTreeMap::new(), repone_en: 0 }, stock: BTreeMap::new(), pulso: None }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EstadoComercio {
    /// Todos los trueques, por id ascendente (el id crece con el contador y la poda conserva el orden).
    pub acuerdos: Vec<AcuerdoTrueque>,
    /// Las órdenes de mercado en pie de todas las plazas y el historial de las cerradas.
    pub libro: LibroOrdenes,
    /// La flota de caravanas de todas las plazas, por id ascendente.
    pub flota: Vec<Caravana>,
    /// Reputación de cada Facción que alguna vez ha comerciado entre Facciones (la que no consta vale 0).
    pub reputacion: BTreeMap<u32, Reputacion>,
    /// `Asentamiento.ultimaCaravanaCreadaEn`: el cooldown de creación es por plaza.
    pub ultima_caravana: BTreeMap<u64, Instante>,
    /// La red de caminos que dejan las rutas comerciales (§7.4).
    pub red: RedCaminos,
    /// Plazas de origen con un pulso de reparto programado → instante del pulso (no se programa otro mientras tanto).
    pub reparto: BTreeMap<u64, Instante>,
    /// El mercado de mercenarios.
    pub mercenarios: EstadoMercenarios,
    pub siguiente_acuerdo: AcuerdoId,
    pub siguiente_orden: OrdenId,
    pub siguiente_caravana: CaravanaId,
}

impl Default for EstadoComercio {
    fn default() -> Self {
        Self {
            acuerdos: Vec::new(),
            libro: LibroOrdenes::default(),
            flota: Vec::new(),
            reputacion: BTreeMap::new(),
            ultima_caravana: BTreeMap::new(),
            red: RedCaminos::default(),
            reparto: BTreeMap::new(),
            mercenarios: EstadoMercenarios::default(),
            siguiente_acuerdo: 1,
            siguiente_orden: 1,
            siguiente_caravana: 1,
        }
    }
}

impl EstadoComercio {
    pub fn acuerdo(&self, id: AcuerdoId) -> Option<&AcuerdoTrueque> {
        self.acuerdos.binary_search_by_key(&id, |a| a.id).ok().map(|i| &self.acuerdos[i])
    }

    pub(crate) fn posicion_acuerdo(&self, id: AcuerdoId) -> Option<usize> {
        self.acuerdos.binary_search_by_key(&id, |a| a.id).ok()
    }

    pub fn caravana(&self, id: CaravanaId) -> Option<&Caravana> {
        self.posicion_caravana(id).map(|i| &self.flota[i])
    }

    pub(crate) fn posicion_caravana(&self, id: CaravanaId) -> Option<usize> {
        self.flota.binary_search_by_key(&id, |c| c.id).ok()
    }

    /// Las caravanas comerciales de una plaza, en orden de id.
    pub fn flota_de(&self, plaza: u64) -> impl Iterator<Item = &Caravana> {
        self.flota.iter().filter(move |c| c.tipo == TipoCaravana::Comercial && c.origen as u64 == plaza)
    }

    /// ¿Cita alguna caravana este trueque? (lo que `podar` respeta).
    pub fn citado_por_caravana(&self, acuerdo: AcuerdoId) -> bool {
        self.flota.iter().any(|c| c.origen_acuerdo.is_some_and(|(a, _)| a == acuerdo))
    }

    /// Valor de la reputación de una Facción en `t` (ppm de punto; 0 si no consta).
    pub fn reputacion_en(&self, faccion: u32, t: Instante) -> i64 {
        self.reputacion.get(&faccion).map_or(0, |r| r.valor_en(t))
    }
}
