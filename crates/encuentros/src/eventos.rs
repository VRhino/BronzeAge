//! Los encuentros como eventos de la cola (prioridad 8 «Llegadas y encuentros de columnas», Reloj_Y_Eventos §2).
//!
//! Cada evento lleva la clave `(entidad, generación)` de la entidad **cuyo plan acaba de cambiar**: si vuelve a cambiar,
//! el evento se descarta al salir. Los eventos que dependen también de otra entidad (un encuentro, una intercepción)
//! guardan la generación de esa otra, y al salir se comprueba con [`EventoEspacial::otras_vigentes`].

use crate::{interceptar, llegada_a_radio, Rejilla};
use base::{Instante, Mil, Plan, Punto};
use cola::{Clave, Cola, Generaciones};

pub const PRIORIDAD_ENCUENTROS: u8 = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventoEspacial {
    /// `a` (la del plan nuevo) y `b` quedan a ≤ radio. `punto`: punto medio.
    Encuentro { a: u64, b: u64, b_generacion: u32, punto: Punto },
    /// `entidad` llega a la puerta `puerta` (≤ su radio).
    Llegada { entidad: u64, puerta: u64, punto: Punto },
    /// `perseguidor` alcanza a `presa` en `punto` (posición de la presa).
    Intercepcion { perseguidor: u64, presa: u64, presa_generacion: u32, punto: Punto },
}

impl EventoEspacial {
    /// ¿Sigue vigente el plan de la **otra** entidad con la que se calculó? (La de la clave la comprueba la cola.)
    pub fn otras_vigentes(&self, g: &Generaciones) -> bool {
        match *self {
            Self::Encuentro { b, b_generacion, .. } => g.actual(b) == b_generacion,
            Self::Intercepcion { presa, presa_generacion, .. } => g.actual(presa) == presa_generacion,
            Self::Llegada { .. } => true,
        }
    }
}

/// Un evento listo para `Cola::programar`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Programado {
    pub instante: Instante,
    pub prioridad: u8,
    pub clave: Clave,
    pub carga: EventoEspacial,
}

/// Una puerta (de plaza, de campamento) con su radio de llegada (`MOVIMIENTO.radioPuerta`, 2).
#[derive(Clone, Copy, Debug)]
pub struct Puerta {
    pub id: u64,
    pub centro: Punto,
    pub radio: Mil,
}

/// Eventos a programar para el plan nuevo de `entidad` (que ya debe estar apuntada en la rejilla con esa generación o
/// no: aquí solo se consulta): sus encuentros con las demás a ≤ `radio_encuentro` y sus llegadas a las `puertas`.
/// Ordenados por instante.
pub fn eventos_de_plan(
    rejilla: &Rejilla,
    entidad: u64,
    generacion: u32,
    plan: &Plan,
    radio_encuentro: Mil,
    puertas: &[Puerta],
) -> Vec<Programado> {
    let clave = Clave { entidad, generacion };
    let encuentros = rejilla.encuentros_de(entidad, plan, radio_encuentro).into_iter().map(|e| Programado {
        instante: e.instante,
        prioridad: PRIORIDAD_ENCUENTROS,
        clave,
        carga: EventoEspacial::Encuentro { a: entidad, b: e.otra, b_generacion: e.otra_generacion, punto: e.punto },
    });
    let llegadas = puertas.iter().filter_map(|p| {
        let instante = llegada_a_radio(plan, p.centro, p.radio)?;
        let punto = plan.posicion_en(instante)?;
        Some(Programado {
            instante,
            prioridad: PRIORIDAD_ENCUENTROS,
            clave,
            carga: EventoEspacial::Llegada { entidad, puerta: p.id, punto },
        })
    });
    let mut v: Vec<Programado> = encuentros.chain(llegadas).collect();
    v.sort_by_key(|p| p.instante); // estable: a igual instante, encuentros antes que llegadas
    v
}

/// Evento de intercepción de `perseguidor` (en `pos` desde `desde`, a `velocidad` mil/s) sobre el plan de la presa.
/// Hay que recalcularlo cuando la presa cambia de plan (nueva generación del perseguidor o de la presa).
#[allow(clippy::too_many_arguments)]
pub fn evento_de_persecucion(
    perseguidor: u64,
    generacion: u32,
    pos: Punto,
    desde: Instante,
    velocidad: Mil,
    radio: Mil,
    presa: u64,
    presa_generacion: u32,
    plan_presa: &Plan,
) -> Option<Programado> {
    let i = interceptar(pos, desde, velocidad, radio, plan_presa)?;
    Some(Programado {
        instante: i.instante,
        prioridad: PRIORIDAD_ENCUENTROS,
        clave: Clave { entidad: perseguidor, generacion },
        carga: EventoEspacial::Intercepcion { perseguidor, presa, presa_generacion, punto: i.punto },
    })
}

/// Mete los eventos en la cola.
pub fn programar(cola: &mut Cola<EventoEspacial>, eventos: impl IntoIterator<Item = Programado>) {
    for e in eventos {
        cola.programar(e.instante, e.prioridad, e.clave, e.carga);
    }
}
