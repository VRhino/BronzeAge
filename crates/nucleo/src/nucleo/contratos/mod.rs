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

/// La entidad que nombra `o`, si es una (un punto no lo es): la que se mira para saber si la transición que se cierra tocó lo que nombra una orden, y la que se toca cuando un contrato o
/// una orden empiezan o dejan de nombrarla ([`Nucleo::tocar_objetivo`]).
fn entidad_de(o: &Objetivo) -> Option<(Clase, u64)> {
    Some(match *o {
        Objetivo::Plaza(id) => (Clase::Asentamiento, id),
        Objetivo::Pueblo(id) => (Clase::Pueblo, id),
        Objetivo::Avanzada(id) => (Clase::Avanzada, id),
        Objetivo::Caravana(id) => (Clase::Caravana, id),
        Objetivo::Bandidos(id) => (Clase::Bandidos, id),
        Objetivo::Ejercito(id) => (Clase::Ejercito, id),
        Objetivo::Convocatoria(id) => (Clase::Convocatoria, id),
        Objetivo::Faccion(id) => (Clase::Faccion, u64::from(id)),
        Objetivo::Punto(_) => return None,
    })
}

impl Nucleo {
    /// Toca lo que nombra `o`: lo de su Facción que nombran el contrato o la orden de un mercenario le llega por su `HEROE` (`proyeccion::audiencia`), así que cuando empiezan o dejan de
    /// nombrarlo hay que dárselo o quitárselo aunque no haya cambiado.
    fn tocar_objetivo(&mut self, o: &Objetivo) {
        if let Some((clase, id)) = entidad_de(o) {
            self.tr.tocar(clase, id);
        }
    }

    /// Despacha los nueve comandos de los contratos, cada uno a la función de su parte.
    pub(super) fn ejecutar_contratos(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        match &p.comando {
            Comando::OfrecerseComoMercenario { heroe } => self.ofrecerse_como_mercenario(t, p, *heroe),
            Comando::RetirarOferta { heroe } => self.retirar_oferta(t, p, *heroe),
            Comando::ContratarMercenario { heroe, mercenario, trabajo, objetivo, plazo, residencia, pagar_con } => {
                self.contratar_mercenario(t, p, *heroe, *mercenario, *trabajo, *objetivo, *plazo, *residencia, *pagar_con)
            }
            Comando::ProrrogarContrato { heroe, mercenario, plazo, pagar_con } => self.prorrogar_contrato(t, p, *heroe, *mercenario, *plazo, *pagar_con),
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
