//! Dar la recompensa de una misión al reclamarla (entrada 66; `Docs/Mechanics/Misiones.md` §4 y `Misiones_Recurrentes.md` §5): un solo reparto para las cadenas y las
//! recurrentes. Cada orden de reclamar dice quién cobra ([`Cobro`]) y [`Nucleo::repartir`] lo da todo o nada. Lo puro (leer y validar la recompensa) es de `crate::recompensas`.

use super::*;
use crate::estado::Dato;
use crate::recompensas::Recompensa;

/// Adónde van los recursos, el oro y la tropa: una misión personal paga al héroe; una colectiva, a la plaza o a la capital de la Facción.
pub(super) enum Destino {
    /// El almacén personal del héroe y, la tropa, su campamento.
    Heroe(u64),
    /// El almacén de la plaza.
    Plaza(u64),
    /// El almacén de la capital efectiva de la Facción ([`Nucleo::capital_de`]).
    Capital(u32),
}

/// Quién cobra una recompensa.
pub(super) struct Cobro {
    pub destino: Destino,
    /// La Facción que suma la experiencia (la fija y la de la lista). `None`: no se suma.
    pub faccion: Option<u32>,
    /// La plaza y los héroes entre los que se reparte la participación fija, a partes iguales (división entera; el resto se pierde).
    pub participacion: Option<(u64, Vec<u64>)>,
}

/// Lo que se dio, para el hecho `mision.reclamada`.
pub(super) struct Dado {
    pub datos: Vec<(&'static str, i64)>,
    pub valores: Vec<(&'static str, Dato)>,
}

impl Nucleo {
    /// Reclamar es todo o nada: comprueba que todo cabe y solo entonces lo da todo.
    pub(super) fn repartir(&mut self, _t: Instante, _r: &Recompensa, _cobro: Cobro) -> Result<Dado, Respuesta> {
        todo!("T1: el reparto de la recompensa (plan de recompensas de misión, §2.2 y §4)")
    }
}
