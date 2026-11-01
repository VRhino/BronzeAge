//! Los mercenarios por contrato dentro del núcleo (entrada 48, `Docs/Mechanics/Mercenarios_Por_Contrato.md`). Las reglas puras (tarifa, quién firma, qué orden se admite, rangos y
//! medallas) son del crate `mercenarios`; aquí está lo que mira el mundo: el despacho de los nueve comandos, los plazos de la cola, la vigilancia al cerrar cada versión y el
//! traslado a hechos. Cada fichero es una parte: `firma` (ofrecerse, contratar, prorrogar), `mando` (órdenes y mudar), `fin` (despedir y fin del contrato), `tropa` (rango, premio
//! y leva propia), `renombre` (hechos que lo dan, valoración y Leal) y `guardas` (lo que un marcado no hace).
//!
//! Lo que se llama desde fuera de `contratos/` es `pub(in crate::nucleo)`; lo que solo usan sus ficheros entre sí, `pub(super)`.

use super::*;
use crate::estado::{entidad_contrato, SucesoContrato, PRIO_CONTRATO};
use ::mercenarios::{Objetivo, TipoOrden, Trabajo};
use militar::catalogo::Tropa;

mod fin;
mod firma;
mod guardas;
mod mando;
mod renombre;
mod tropa;

pub(in crate::nucleo) use mando::Desenlace;

impl Nucleo {
    /// Despacha los nueve comandos de los contratos, cada uno a la función de su parte.
    pub(super) fn ejecutar_contratos(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        match &p.comando {
            Comando::OfrecerseComoMercenario { heroe } => self.ofrecerse_como_mercenario(t, p, *heroe),
            Comando::RetirarOferta { heroe } => self.retirar_oferta(t, p, *heroe),
            Comando::ContratarMercenario { heroe, mercenario, trabajo, objetivo, plazo, residencia } => self.contratar_mercenario(t, p, *heroe, *mercenario, *trabajo, *objetivo, *plazo, *residencia),
            Comando::ProrrogarContrato { heroe, mercenario, plazo } => self.prorrogar_contrato(t, p, *heroe, *mercenario, *plazo),
            Comando::OrdenarMercenario { heroe, mercenarios, tipo, objetivo } => self.ordenar_mercenario(t, p, *heroe, mercenarios, *tipo, *objetivo),
            Comando::MudarMercenario { heroe, mercenario, plaza } => self.mudar_mercenario(t, p, *heroe, *mercenario, *plaza),
            Comando::DespedirMercenario { heroe, mercenario } => self.despedir_mercenario(t, p, *heroe, *mercenario),
            Comando::ValorarMercenario { heroe, mercenario, buena } => self.valorar_mercenario(t, p, *heroe, *mercenario, *buena),
            Comando::ReclamarPremioDeRango { heroe, tropa } => self.reclamar_premio_de_rango(t, p, *heroe, *tropa),
            _ => unreachable!("solo se llama con órdenes de los contratos de mercenarios"),
        }
    }

    /// Lo que depende del mundo se mira al cerrar cada versión (decisión 18 del plan): las órdenes cuyo objetivo cayó o que la paz anuló, y los contratos que deben acabar.
    pub(super) fn vigilar_contratos(&mut self, t: Instante) {
        if self.estado.contratos.contratos.is_empty() {
            return;
        }
        self.vigilar_ordenes(t);
        self.vigilar_fines(t);
    }

    /// Pone en la cola un plazo del contrato de `mercenario` para el instante `en` (decisión 17 del plan). No sube generaciones: la cita que quede vieja (un contrato
    /// prorrogado, una orden sustituida, un contrato acabado) se ignora al vencer porque su instante ya no es el guardado.
    pub(super) fn citar_contrato(&mut self, mercenario: u64, suceso: SucesoContrato, en: Instante) {
        let clave = self.estado.generaciones.clave(entidad_contrato(mercenario));
        self.estado.cola.programar(en, PRIO_CONTRATO, clave, Carga::Contrato(suceso));
    }

    /// Vence un plazo del contrato de `mercenario` (`Carga::Contrato`).
    pub(super) fn suceso_de_contrato(&mut self, mercenario: u64, suceso: SucesoContrato, t: Instante) {
        match suceso {
            SucesoContrato::Vence => self.vence_contrato(mercenario, t),
            SucesoContrato::VenceOrden => self.vence_orden(mercenario, t),
            SucesoContrato::Leal => self.cita_leal(mercenario, t),
        }
    }
}
