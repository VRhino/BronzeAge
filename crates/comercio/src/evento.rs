//! Los eventos del comercio (Comercio_Caravanas §12). El crate no emite nada por sí mismo: el núcleo construye el [`Evento`] que
//! corresponde a cada transición y lo traduce a su registro de cambios con [`Evento::codigo`].

use crate::tipos::{AcuerdoId, AsentamientoId, CaravanaId, Carga, HeroeId, OrdenId};
use asentamiento::Recurso;
use base::Mil;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Evento {
    // --- Trueque ---
    TruequePropuesto { acuerdo: AcuerdoId },
    TruequeAceptado { acuerdo: AcuerdoId },
    TruequeRechazado { acuerdo: AcuerdoId },
    /// `entre_facciones` e `intercambiado` los lee el mercado de mercenarios.
    TruequeCumplido { acuerdo: AcuerdoId, entre_facciones: bool, intercambiado: Carga },
    TruequeExpirado { acuerdo: AcuerdoId },
    /// Por la ruina o la conquista de una de las plazas; se avisa a la socia.
    TruequeCancelado { acuerdo: AcuerdoId, socia: AsentamientoId },
    // --- Comandos sobre la flota (narración) ---
    CaravanaConstruida { caravana: CaravanaId },
    CaravanaCarroAgregado { caravana: CaravanaId },
    CaravanaCarroMovido { caravana: CaravanaId },
    CaravanaReserva { caravana: CaravanaId, reservada: bool },
    CaravanaPreparando { caravana: CaravanaId },
    CaravanaCancelada { caravana: CaravanaId },
    CaravanaEscoltaCedida { caravana: CaravanaId },
    CaravanaEscoltaRetirada { caravana: CaravanaId },
    CargaCaravanaAparcada { caravana: CaravanaId },
    CaravanaEnviadaAlOrigen { caravana: CaravanaId },
    /// Contador de logros `animales.comprados`.
    CaravanaAnimalComprado { caravana: CaravanaId },
    // --- Viaje ---
    CaravanaSale { caravana: CaravanaId },
    PeajePaso { caravana: CaravanaId, ciudad: AsentamientoId, cobrado: Carga },
    /// Lleva `comision` y `recargo` (hecho `caravana` de las épicas: origen = la plaza que cuenta).
    CaravanaLlega { caravana: CaravanaId, destino: AsentamientoId, comision: Mil, recargo: Mil },
    EntregaEscoltada { acuerdo: AcuerdoId, caravana: CaravanaId },
    // --- Ruina, conquista y presencia ---
    CaravanaPerdida { caravana: CaravanaId },
    CaravanaVuelve { caravana: CaravanaId },
    // --- Mercado de una plaza ---
    OrdenColocada { orden: OrdenId },
    OrdenExpirada { orden: OrdenId },
    /// Lo lee el mercado de mercenarios (`entre_facciones`).
    ComercioEnPlaza { orden: OrdenId, cantidad: Mil, entre_facciones: bool },
    // --- Mercado de mercenarios ---
    MercenariosComprado { campamento: u32, heroe: HeroeId, recurso: Recurso, cantidad: Mil },
    /// Sin plaza.
    MercenariosRepostado { campamento: u32, heroe: HeroeId, cantidad: Mil },
}

impl Evento {
    /// El código del evento, tal como lo escribe `Comercio_Caravanas.md` §12.
    pub fn codigo(&self) -> &'static str {
        match self {
            Evento::TruequePropuesto { .. } => "comercio.trueque_propuesto",
            Evento::TruequeAceptado { .. } => "comercio.trueque_aceptado",
            Evento::TruequeRechazado { .. } => "comercio.trueque_rechazado",
            Evento::TruequeCumplido { .. } => "comercio.trueque_cumplido",
            Evento::TruequeExpirado { .. } => "comercio.trueque_expirado",
            Evento::TruequeCancelado { .. } => "comercio.trueque_cancelado",
            Evento::CaravanaConstruida { .. } => "comercio.caravana_construida",
            Evento::CaravanaCarroAgregado { .. } => "comercio.caravana_carro_agregado",
            Evento::CaravanaCarroMovido { .. } => "comercio.caravana_carro_movido",
            Evento::CaravanaReserva { .. } => "comercio.caravana_reserva",
            Evento::CaravanaPreparando { .. } => "comercio.caravana_preparando",
            Evento::CaravanaCancelada { .. } => "comercio.caravana_cancelada",
            Evento::CaravanaEscoltaCedida { .. } => "comercio.caravana_escolta_cedida",
            Evento::CaravanaEscoltaRetirada { .. } => "comercio.caravana_escolta_retirada",
            Evento::CargaCaravanaAparcada { .. } => "comercio.carga_caravana_aparcada",
            Evento::CaravanaEnviadaAlOrigen { .. } => "comercio.caravana_enviada_al_origen",
            Evento::CaravanaAnimalComprado { .. } => "comercio.caravana_animal_comprado",
            Evento::CaravanaSale { .. } => "comercio.caravana_sale",
            Evento::PeajePaso { .. } => "comercio.peaje_paso",
            Evento::CaravanaLlega { .. } => "comercio.caravana_llega",
            Evento::EntregaEscoltada { .. } => "comercio.entrega_escoltada",
            Evento::CaravanaPerdida { .. } => "comercio.caravana_perdida",
            Evento::CaravanaVuelve { .. } => "comercio.caravana_vuelve",
            Evento::OrdenColocada { .. } => "mercado.orden_colocada",
            Evento::OrdenExpirada { .. } => "mercado.orden_expirada",
            Evento::ComercioEnPlaza { .. } => "mercado.comercio_en_plaza",
            Evento::MercenariosComprado { .. } => "mercenarios.comprado",
            Evento::MercenariosRepostado { .. } => "mercenarios.repostado",
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::COMERCIO;
    use std::collections::BTreeSet;

    fn todos() -> Vec<Evento> {
        let c = Carga::new();
        vec![
            Evento::TruequePropuesto { acuerdo: 1 },
            Evento::TruequeAceptado { acuerdo: 1 },
            Evento::TruequeRechazado { acuerdo: 1 },
            Evento::TruequeCumplido { acuerdo: 1, entre_facciones: true, intercambiado: c.clone() },
            Evento::TruequeExpirado { acuerdo: 1 },
            Evento::TruequeCancelado { acuerdo: 1, socia: 2 },
            Evento::CaravanaConstruida { caravana: 1 },
            Evento::CaravanaCarroAgregado { caravana: 1 },
            Evento::CaravanaCarroMovido { caravana: 1 },
            Evento::CaravanaReserva { caravana: 1, reservada: true },
            Evento::CaravanaPreparando { caravana: 1 },
            Evento::CaravanaCancelada { caravana: 1 },
            Evento::CaravanaEscoltaCedida { caravana: 1 },
            Evento::CaravanaEscoltaRetirada { caravana: 1 },
            Evento::CargaCaravanaAparcada { caravana: 1 },
            Evento::CaravanaEnviadaAlOrigen { caravana: 1 },
            Evento::CaravanaAnimalComprado { caravana: 1 },
            Evento::CaravanaSale { caravana: 1 },
            Evento::PeajePaso { caravana: 1, ciudad: 2, cobrado: c },
            Evento::CaravanaLlega { caravana: 1, destino: 2, comision: 0, recargo: 0 },
            Evento::EntregaEscoltada { acuerdo: 1, caravana: 1 },
            Evento::CaravanaPerdida { caravana: 1 },
            Evento::CaravanaVuelve { caravana: 1 },
            Evento::OrdenColocada { orden: 1 },
            Evento::OrdenExpirada { orden: 1 },
            Evento::ComercioEnPlaza { orden: 1, cantidad: 1, entre_facciones: false },
            Evento::MercenariosComprado { campamento: 1, heroe: 1, recurso: Recurso::Cobre, cantidad: 1 },
            Evento::MercenariosRepostado { campamento: 1, heroe: 1, cantidad: 1 },
        ]
    }

    #[test]
    fn cada_evento_sale_con_su_codigo_y_sin_repetirse() {
        let codigos: Vec<_> = todos().iter().map(Evento::codigo).collect();
        assert_eq!(codigos.len(), 28);
        assert_eq!(codigos.iter().collect::<BTreeSet<_>>().len(), 28, "ningún código se repite");
        assert!(codigos.iter().all(|c| c.starts_with("comercio.") || c.starts_with("mercado.") || c.starts_with("mercenarios.")));
    }

    #[test]
    fn los_codigos_estan_escritos_en_el_documento() {
        // La tabla de §12 abrevia los sufijos (`_aceptado`, `_carro_agregado`…): cada código se busca por su última palabra.
        for c in todos().iter().map(Evento::codigo) {
            let palabra = c.rsplit(['_', '.']).next().unwrap();
            assert!(COMERCIO.contains(palabra), "«{c}» no aparece en Comercio_Caravanas.md");
        }
    }
}
