//! Los hechos que dan renombre y cuentan medallas, la valoración y la medalla Leal (Mercenarios_Por_Contrato §8.2 y §9). Tarea T9 del plan.

use super::*;

impl Nucleo {
    pub(super) fn valorar_mercenario(&mut self, t: Instante, p: &Peticion, heroe: u64, mercenario: u64, buena: bool) -> Result<Vec<u64>, Respuesta> {
        let _ = (t, p, heroe, mercenario, buena);
        Err(Respuesta::sin_cambios(self.estado.version))
    }

    pub(super) fn cita_leal(&mut self, mercenario: u64, t: Instante) {
        let _ = (mercenario, t);
    }
}
