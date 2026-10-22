//! El hueco por donde se enchufan `encuentros` (eventos exactos entre planes) y `vision` (muestreo informativo).
//!
//! Un [`GeneradorDeEventos`] se registra con `Nucleo::agregar_generador` y recibe, en el hilo del núcleo y en orden:
//! - `plan_cambiado` / `plan_retirado`: cada vez que una columna recibe un plan nuevo o se queda quieta (orden, llegada,
//!   hambre). Aquí `encuentros` apunta y quita tramos en su rejilla y **programa** con [`Cx::programar`] los cruces.
//! - `al_vencer`: cuando sale de la cola un evento que programó él (con la generación vigente de su entidad).
//! - `muestrear`: cada `Ajustes::muestreo_ms` de mundo (0 = nunca), para lo informativo (la visión cada 250 ms).
//!
//! Todo lo que haga debe ser **determinista** (misma entrada, misma salida): sus eventos entran en la cola y en las fotos.
//! Su propio estado interno no entra en `hash_estado`; si afecta al juego debe poder reconstruirse de `Estado`: por eso, al
//! registrarlo, el núcleo le pasa los planes vigentes con `plan_vigente` (sin `Cx`: solo para rehacer su índice; los
//! eventos que programó antes de una caída ya están en la cola recuperada).

use crate::estado::{Carga, Clase, Estado, EventoNucleo, Tocados};
use base::{Instante, Plan};
use cola::Clave;

/// Lo que produce una transición: las entidades tocadas y los eventos de dominio, que se cierran en una versión.
#[derive(Default)]
pub(crate) struct Transicion {
    pub tocados: Tocados,
    pub eventos: Vec<EventoNucleo>,
}

impl Transicion {
    pub fn tocar(&mut self, clase: Clase, id: u64) {
        self.tocados.marcar(clase, id);
    }
    pub fn tocar_todos(&mut self, clase: Clase, ids: impl IntoIterator<Item = u64>) {
        ids.into_iter().for_each(|id| self.tocados.marcar(clase, id));
    }
}

/// Qué puede hacer un generador en cada llamada: leer el estado, programar eventos propios y emitir eventos de dominio.
pub struct Cx<'a> {
    pub(crate) estado: &'a mut Estado,
    pub(crate) tr: &'a mut Transicion,
    pub(crate) generador: u16,
}

impl Cx<'_> {
    /// Instante del evento (o de la muestra, o de la orden) que se está tratando.
    pub fn ahora(&self) -> Instante {
        self.estado.ahora
    }
    pub fn estado(&self) -> &Estado {
        self.estado
    }
    /// Generación vigente de la entidad: si cambia, los eventos programados con la anterior se descartan solos.
    pub fn generacion(&self, entidad: u64) -> u32 {
        self.estado.generaciones.actual(entidad)
    }
    /// Programa un evento propio ligado a la generación vigente de `entidad`. `prioridad` en `1..=10`.
    pub fn programar(&mut self, instante: Instante, prioridad: u8, entidad: u64, tipo: u16, a: u64, b: u64) {
        let clave = self.estado.generaciones.clave(entidad);
        self.programar_con(instante, prioridad, clave, tipo, a, b);
    }
    /// Igual, con una clave explícita (p. ej. la generación que tenía la entidad cuando se calculó el cruce).
    pub fn programar_con(&mut self, instante: Instante, prioridad: u8, clave: Clave, tipo: u16, a: u64, b: u64) {
        let carga = Carga::Externa { generador: self.generador, tipo, a, b };
        self.estado.cola.programar(instante, prioridad, clave, carga);
    }
    /// Emite un hecho (crónica): se cierra en una versión nueva y llega a quien oye a las columnas que nombra.
    pub fn emitir(&mut self, evento: EventoNucleo) {
        self.tr.eventos.push(evento);
    }
}

pub trait GeneradorDeEventos: Send {
    /// Plan ya vigente al registrar el generador (p. ej. tras recuperar). Solo para reconstruir el índice propio.
    fn plan_vigente(&mut self, _ejercito: u64, _plan: &Plan) {}
    fn plan_cambiado(&mut self, _cx: &mut Cx<'_>, _ejercito: u64, _plan: &Plan) {}
    fn plan_retirado(&mut self, _cx: &mut Cx<'_>, _ejercito: u64) {}
    fn al_vencer(&mut self, _cx: &mut Cx<'_>, _entidad: u64, _tipo: u16, _a: u64, _b: u64) {}
    fn muestrear(&mut self, _cx: &mut Cx<'_>) {}
}
