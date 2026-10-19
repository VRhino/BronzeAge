//! Los invariantes del comercio que el batch comprueba sobre el estado y la cola del núcleo (todos deben ser 0): contenedores dentro de su
//! capacidad, trueques coherentes, ninguna caravana en camino sin plan ni sin su evento de llegada, ningún evento vigente de una entidad
//! que ya no corresponde y ningún reparto programado en una plaza ociosa.

use asentamiento::datos::UNO;
use asentamiento::recurso::TODOS;
use comercio::caravana::{EstadoCaravana, TipoCaravana};
use comercio::trueque::EstadoTrueque;
use nucleo::{Carga, Nucleo, SucesoComercio};
use std::collections::BTreeSet;

/// Bits altos de las claves de entidad de comercio (`estado_comercio`): quedan las que son un id.
const ID_DE_ENTIDAD: u64 = (1 << 56) - 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Violaciones {
    /// Un recurso de una plaza o de un pueblo fuera de `[0, capacidad]`.
    pub almacen: u64,
    /// Un carro de columna o una caravana por encima de su capacidad, o con una cantidad negativa.
    pub carga: u64,
    /// Un trueque con más entregado que pactado, o activo con una de sus plazas ya inexistente.
    pub trueque: u64,
    /// Una caravana en camino sin plan o sin su evento de llegada vigente.
    pub sin_plan: u64,
    /// Un evento de comercio vigente de una entidad que no está en el estado que lo justifica.
    pub cola: u64,
    /// Un reparto programado en una plaza sin ningún trueque activo (una plaza ociosa).
    pub ociosa: u64,
    /// Una plaza sin su evento de minuto, o un evento de minuto sin plaza.
    pub minuto: u64,
    /// Versiones del registro de cambios perdidas antes de leerse.
    pub registro: u64,
    /// Un mineral cuyo total en el mundo no es lo comprado a los mercenarios.
    pub conservacion: u64,
}

impl Violaciones {
    pub fn sumar(&mut self, o: &Violaciones) {
        self.almacen += o.almacen;
        self.carga += o.carga;
        self.trueque += o.trueque;
        self.sin_plan += o.sin_plan;
        self.cola += o.cola;
        self.ociosa += o.ociosa;
        self.minuto += o.minuto;
        self.registro += o.registro;
        self.conservacion += o.conservacion;
    }
}

/// Almacenes, carros, flota y trueques.
pub fn auditar_contenedores(n: &Nucleo) -> Violaciones {
    let (e, mut v) = (n.estado(), Violaciones::default());
    for a in e.asentamientos.values() {
        v.almacen += TODOS.iter().filter(|&&r| a.economia.almacen.cantidad(r) < 0 || a.economia.almacen.cantidad(r) > a.economia.almacen.capacidad(r)).count() as u64;
        for p in &a.economia.pueblos {
            v.almacen += TODOS.iter().filter(|&&r| p.almacen.cantidad(r) < 0 || p.almacen.cantidad(r) > p.almacen.capacidad(r)).count() as u64;
        }
    }
    for c in e.ejercitos.values() {
        let total: i64 = c.suministro.values().sum();
        v.carga += (c.suministro.values().any(|&x| x < 0) || n.capacidad_de_carga(c.id).is_some_and(|cap| total > cap)) as u64;
    }
    let flota = &e.reglas.economia.comercio.flota;
    for c in &e.comercio.flota {
        let total: i64 = c.contenido.values().sum();
        v.carga += (c.contenido.values().any(|&x| x < 0) || total > c.capacidad_efectiva(flota, UNO)) as u64;
    }
    for a in &e.comercio.acuerdos {
        let incoherente = [&a.lineas_a, &a.lineas_b].iter().any(|ls| ls.iter().any(|l| l.cantidad_entregada > l.cantidad_total || l.cantidad_entregada < 0));
        let sin_plaza = a.estado == EstadoTrueque::Activo && !(e.asentamientos.contains_key(&(a.a as u64)) && e.asentamientos.contains_key(&(a.b as u64)));
        v.trueque += (incoherente || sin_plaza) as u64;
    }
    v
}

/// La cola de eventos frente al estado: cada evento de comercio vigente tiene a su entidad en el estado que lo justifica, cada caravana en
/// camino tiene el suyo, y cada plaza tiene exactamente un minuto.
pub fn auditar_cola(n: &Nucleo) -> Violaciones {
    let (e, mut v) = (n.estado(), Violaciones::default());
    let mut por_suceso: [BTreeSet<u64>; 5] = Default::default();
    let mut minutos: Vec<u64> = Vec::new();
    for ev in e.cola.volcar().iter().filter(|ev| e.generaciones.es_vigente(ev.clave)) {
        let id = ev.clave.entidad & ID_DE_ENTIDAD;
        match &ev.carga {
            Carga::Minuto => minutos.push(ev.clave.entidad),
            Carga::Comercio { suceso: SucesoComercio::LlegadaCaravana, .. } => drop(por_suceso[0].insert(id)),
            Carga::Comercio { suceso: SucesoComercio::PeajeCaravana, .. } => drop(por_suceso[1].insert(id)),
            Carga::Comercio { suceso: SucesoComercio::FinPreparacion, .. } => drop(por_suceso[2].insert(id)),
            Carga::Comercio { suceso: SucesoComercio::CaducaTrueque, .. } => drop(por_suceso[3].insert(id)),
            Carga::Comercio { suceso: SucesoComercio::PulsoReparto, .. } => drop(por_suceso[4].insert(id)),
            _ => {}
        }
    }
    let [llegadas, peajes, preparaciones, caducidades, pulsos] = &por_suceso;
    let en_camino = |s: EstadoCaravana| matches!(s, EstadoCaravana::EnTransito | EstadoCaravana::Retornando);
    for c in e.comercio.flota.iter().filter(|c| matches!(c.tipo, TipoCaravana::Comercial | TipoCaravana::Acarreo) && en_camino(c.estado)) {
        v.sin_plan += (!c.plan.as_ref().is_some_and(|p| p.vertices.len() >= 2) || !llegadas.contains(&c.id)) as u64;
    }
    v.cola += llegadas.iter().filter(|&&id| !e.comercio.caravana(id).is_some_and(|c| en_camino(c.estado))).count() as u64;
    v.cola += peajes.iter().filter(|&&id| !e.comercio.caravana(id).is_some_and(|c| c.estado == EstadoCaravana::EnTransito && c.proximo_peaje().is_some())).count() as u64;
    v.cola += preparaciones.iter().filter(|&&id| !e.comercio.caravana(id).is_some_and(|c| c.estado == EstadoCaravana::Preparando)).count() as u64;
    v.cola += caducidades.iter().filter(|&&id| !e.comercio.acuerdo(id).is_some_and(|a| matches!(a.estado, EstadoTrueque::Propuesto | EstadoTrueque::Activo))).count() as u64;
    for &plaza in pulsos {
        let activo = e.comercio.acuerdos.iter().any(|a| a.estado == EstadoTrueque::Activo && (a.a as u64 == plaza || a.b as u64 == plaza));
        v.ociosa += (!activo) as u64;
        v.cola += (!e.asentamientos.contains_key(&plaza) || !e.comercio.reparto.contains_key(&plaza)) as u64;
    }
    minutos.sort_unstable();
    let plazas: Vec<u64> = e.asentamientos.keys().copied().collect();
    v.minuto += (minutos != plazas) as u64;
    v
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn sumar_suma_campo_a_campo() {
        let mut a = Violaciones { almacen: 1, cola: 2, ..Violaciones::default() };
        a.sumar(&Violaciones { carga: 3, conservacion: 4, ..Violaciones::default() });
        assert_eq!((a.almacen, a.carga, a.cola, a.conservacion), (1, 3, 2, 4));
    }
}
