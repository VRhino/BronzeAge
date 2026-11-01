//! Lo que un mercenario con marca no hace por su cuenta, en una sola función antes del despacho (Mercenarios_Por_Contrato §2 y §11; decisiones 10 a 15 del plan). Tarea T4 del plan.

use super::*;

impl Nucleo {
    /// Se mira justo después de `rechazo_por_batalla`, antes de despachar cualquier orden.
    pub(in crate::nucleo) fn rechazo_por_contrato(&self, t: Instante, p: &Peticion) -> Option<Respuesta> {
        let _ = (t, p);
        None
    }
}
