//! Las órdenes de mando y mudar a un permanente (Mercenarios_Por_Contrato §6). Tarea T5 del plan.

use super::*;

impl Nucleo {
    pub(super) fn ordenar_mercenario(&mut self, t: Instante, p: &Peticion, heroe: u64, mercenarios: &[u64], tipo: TipoOrden, objetivo: Objetivo) -> Result<Vec<u64>, Respuesta> {
        let _ = (t, p, heroe, mercenarios, tipo, objetivo);
        Err(Respuesta::sin_cambios(self.estado.version))
    }

    pub(super) fn mudar_mercenario(&mut self, t: Instante, p: &Peticion, heroe: u64, mercenario: u64, plaza: u64) -> Result<Vec<u64>, Respuesta> {
        let _ = (t, p, heroe, mercenario, plaza);
        Err(Respuesta::sin_cambios(self.estado.version))
    }

    pub(super) fn vigilar_ordenes(&mut self, t: Instante) {
        let _ = t;
    }

    pub(super) fn vence_orden(&mut self, mercenario: u64, t: Instante) {
        let _ = (mercenario, t);
    }
}
