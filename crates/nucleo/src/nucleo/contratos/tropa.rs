//! El rango y la tropa del mercenario: la leva propia, la escuadra de premio y la licencia de reclutar (Mercenarios_Por_Contrato §8.3). Tarea T8 del plan.

use super::*;

impl Nucleo {
    pub(super) fn reclamar_premio_de_rango(&mut self, t: Instante, p: &Peticion, heroe: u64, tropa: Tropa) -> Result<Vec<u64>, Respuesta> {
        let _ = (t, p, heroe, tropa);
        Err(Respuesta::sin_cambios(self.estado.version))
    }
}
