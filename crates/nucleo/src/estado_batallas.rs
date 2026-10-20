//! El estado de las batallas de Unity dentro del núcleo (Batalla_Unity_Ciclo §3, fase 4E): las abiertas y las cerradas recientes, cada una con su ticket
//! congelado, sus incorporaciones, su asignación y su resultado. Entra entero en el hash y en las fotos (`codec_batallas`). Las reglas puras de la máquina de
//! estados viven en `combate::ciclo`; aquí solo está lo que el núcleo guarda y la clave de cola de sus plazos.
//!
//! La clave de generación de los plazos de una batalla lleva una etiqueta en los bits altos (como las del comercio y las del combate): sus tres plazos
//! —el fin de la convocatoria de un asedio, el vencimiento del estado en que está y la retirada de la cerrada— comparten la entidad, y cada cambio de
//! estado sube su generación y los reprograma.

use crate::estado_comercio::ETIQUETA;
use combate::ciclo::Batalla;
use std::collections::BTreeMap;

const ETIQUETA_BATALLA: u64 = 13 * ETIQUETA;

/// Clave de entidad (de generación y de cola) de una batalla.
pub fn entidad_batalla(id: u64) -> u64 {
    ETIQUETA_BATALLA | id
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EstadoBatallas {
    /// Las batallas abiertas y las cerradas que aún se conservan (`conservarCerradasMinutos`), por id ascendente. El id es el de la partida (`nuevo_id`).
    pub batallas: BTreeMap<u64, Batalla>,
}
