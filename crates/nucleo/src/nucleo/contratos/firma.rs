//! Ofrecerse en el tablón de un campamento, contratar y prorrogar (Mercenarios_Por_Contrato §1 a §3, §5 y §6.2). Tarea T1 del plan.

use super::*;

impl Nucleo {
    pub(super) fn ofrecerse_como_mercenario(&mut self, t: Instante, p: &Peticion, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let _ = (t, p, heroe);
        Err(Respuesta::sin_cambios(self.estado.version))
    }

    pub(super) fn retirar_oferta(&mut self, t: Instante, p: &Peticion, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        let _ = (t, p, heroe);
        Err(Respuesta::sin_cambios(self.estado.version))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn contratar_mercenario(&mut self, t: Instante, p: &Peticion, heroe: u64, mercenario: u64, trabajo: Trabajo, objetivo: Objetivo, plazo: Option<u8>, residencia: Option<u64>) -> Result<Vec<u64>, Respuesta> {
        let _ = (t, p, heroe, mercenario, trabajo, objetivo, plazo, residencia);
        Err(Respuesta::sin_cambios(self.estado.version))
    }

    pub(super) fn prorrogar_contrato(&mut self, t: Instante, p: &Peticion, heroe: u64, mercenario: u64, plazo: u8) -> Result<Vec<u64>, Respuesta> {
        let _ = (t, p, heroe, mercenario, plazo);
        Err(Respuesta::sin_cambios(self.estado.version))
    }
}
