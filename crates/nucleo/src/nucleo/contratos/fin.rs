//! El fin del contrato: despido, plazo y lo que lo acaba desde el mundo (Mercenarios_Por_Contrato §7). Tarea T6 del plan.

use super::*;

impl Nucleo {
    pub(super) fn despedir_mercenario(&mut self, t: Instante, p: &Peticion, heroe: u64, mercenario: u64) -> Result<Vec<u64>, Respuesta> {
        let _ = (t, p, heroe, mercenario);
        Err(Respuesta::sin_cambios(self.estado.version))
    }

    pub(super) fn vence_contrato(&mut self, mercenario: u64, t: Instante) {
        let _ = (mercenario, t);
    }

    pub(super) fn vigilar_fines(&mut self, t: Instante) {
        let _ = t;
    }
}
