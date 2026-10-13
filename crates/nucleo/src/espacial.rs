//! Generadores espaciales que se enchufan al núcleo: encuentros exactos entre columnas (`encuentros`) y visión muestreada
//! (`vision`). Son los dos modos del mundo reactivo: lo que cambia el juego es un evento exacto; lo informativo se muestrea.

use std::collections::BTreeSet;

use crate::estado::EventoNucleo;
use crate::generadores::{Cx, GeneradorDeEventos};
use base::{Mil, Plan};
use cola::Clave;
use encuentros::{Rejilla, PRIORIDAD_ENCUENTROS};
use vision::{Donde, Movil, Muestra, Observador, Vision};

/// Tipo de evento externo del generador de encuentros: `a` = la otra columna, `b` = su generación al calcular el cruce.
const TIPO_ENCUENTRO: u16 = 1;

/// Radio de encuentro entre columnas (Movimiento_Ejercitos: 3 unidades estratégicas, en milésimas).
pub const RADIO_ENCUENTRO: Mil = 3_000;

/// Encuentros exactos: cada vez que una columna cambia de plan se recalculan solo sus pares (rejilla espacio-temporal) y
/// se programan como eventos de la cola, ligados a la generación de ambas columnas.
pub struct GeneradorEncuentros {
    rejilla: Rejilla,
    radio: Mil,
}

impl GeneradorEncuentros {
    pub fn nuevo() -> Self {
        Self { rejilla: Rejilla::para_mundo_real(), radio: RADIO_ENCUENTRO }
    }
}

impl Default for GeneradorEncuentros {
    fn default() -> Self {
        Self::nuevo()
    }
}

impl GeneradorDeEventos for GeneradorEncuentros {
    fn plan_vigente(&mut self, ejercito: u64, plan: &Plan) {
        self.rejilla.apuntar(ejercito, plan.generacion, plan);
    }

    fn plan_cambiado(&mut self, cx: &mut Cx<'_>, ejercito: u64, plan: &Plan) {
        self.rejilla.quitar(ejercito);
        self.rejilla.apuntar(ejercito, plan.generacion, plan);
        let ahora = cx.ahora();
        let clave = Clave { entidad: ejercito, generacion: cx.generacion(ejercito) };
        for e in self.rejilla.encuentros_de(ejercito, plan, self.radio) {
            // Un cruce que ya empezó antes de este instante (planes que se solapan al emitirse) se trata como «ahora».
            cx.programar_con(e.instante.max(ahora), PRIORIDAD_ENCUENTROS, clave, TIPO_ENCUENTRO, e.otra, e.otra_generacion as u64);
        }
    }

    fn plan_retirado(&mut self, _cx: &mut Cx<'_>, ejercito: u64) {
        self.rejilla.quitar(ejercito);
    }

    fn al_vencer(&mut self, cx: &mut Cx<'_>, entidad: u64, tipo: u16, otra: u64, otra_generacion: u64) {
        if tipo != TIPO_ENCUENTRO {
            return;
        }
        // La clave de la cola ya garantiza que `entidad` no cambió de plan; falta comprobar la otra.
        if cx.generacion(otra) as u64 != otra_generacion {
            return;
        }
        let (Some(a), Some(b)) = (cx.estado().ejercitos.get(&entidad), cx.estado().ejercitos.get(&otra)) else { return };
        let t = cx.ahora();
        let punto = {
            let (pa, pb) = (a.plan.posicion_en(t).unwrap_or(a.posicion), b.plan.posicion_en(t).unwrap_or(b.posicion));
            base::Punto::new((pa.x + pb.x) / 2, (pa.y + pb.y) / 2)
        };
        let (fa, fb) = (a.faccion as i64, b.faccion as i64);
        let (lo, hi) = (entidad.min(otra), entidad.max(otra));
        cx.emitir(EventoNucleo {
            codigo: "ejercito.encuentro".into(),
            en: t,
            ejercitos: vec![lo, hi],
            punto: Some(punto),
            datos: vec![("faccionA", fa), ("faccionB", fb)],
        });
    }
}

/// Visión muestreada (cada `Ajustes::muestreo_ms`, p. ej. 250 ms): quién ve a quién. Emite solo el **primer contacto**
/// entre dos facciones; el detalle por columna es lo que alimentará los `Delta` del ámbito de vista.
pub struct GeneradorVision {
    vision: Vision,
    radio_ojo: Mil,
    previa: Muestra,
    contactos: BTreeSet<(u32, u32)>,
    /// Diagnóstico: muestreos hechos y entradas/salidas de vista acumuladas.
    pub muestreos: u64,
    pub entradas: u64,
    pub salidas: u64,
}

impl GeneradorVision {
    pub fn nuevo() -> Self {
        Self {
            vision: Vision::new(),
            radio_ojo: vision::radios::EJERCITO,
            previa: Muestra::default(),
            contactos: BTreeSet::new(),
            muestreos: 0,
            entradas: 0,
            salidas: 0,
        }
    }
}

impl Default for GeneradorVision {
    fn default() -> Self {
        Self::nuevo()
    }
}

impl GeneradorDeEventos for GeneradorVision {
    fn muestrear(&mut self, cx: &mut Cx<'_>) {
        let t = cx.ahora();
        let estado = cx.estado();
        let (mut ojos, mut moviles) = (Vec::new(), Vec::new());
        for e in estado.ejercitos.values() {
            let donde = if e.marchando() { Donde::Plan(&e.plan) } else { Donde::Fijo(e.posicion) };
            ojos.push(Observador { faccion: e.faccion, radio: self.radio_ojo, donde, graba: true });
            moviles.push(Movil::new(e.id, e.faccion, donde));
        }
        let muestra = self.vision.muestrear(t, &ojos, &moviles);
        let cambios = vision::diff(&self.previa, &muestra);
        self.muestreos += 1;
        let mut nuevos = Vec::new();
        for (faccion, c) in &cambios.por_faccion {
            self.entradas += c.entran.len() as u64;
            self.salidas += c.salen.len() as u64;
            for &visto in &c.entran {
                let Some(otro) = estado.ejercitos.get(&visto) else { continue };
                if otro.faccion != *faccion && self.contactos.insert((*faccion, otro.faccion)) {
                    nuevos.push((*faccion, otro.faccion, visto));
                }
            }
        }
        self.previa = muestra;
        for (a, b, visto) in nuevos {
            cx.emitir(EventoNucleo {
                codigo: "vision.contacto".into(),
                en: t,
                ejercitos: vec![visto],
                punto: None,
                datos: vec![("faccionQueVe", a as i64), ("faccionVista", b as i64)],
            });
        }
    }
}
