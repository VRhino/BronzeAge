//! Visión muestreada (`vision`): quién ve a quién. Es lo informativo del mundo reactivo —se muestrea, no hay evento exacto—; lo que cambia el juego
//! (los encuentros y las persecuciones) son eventos exactos del núcleo, en `nucleo/espacio.rs`.

use std::collections::BTreeSet;

use crate::estado::EventoNucleo;
use crate::estado_comercio::entidad_caravana;
use crate::generadores::{Cx, GeneradorDeEventos};
use comercio::caravana::EstadoCaravana;
use vision::{Donde, Movil, Muestra, Observador, Vision};

/// Visión muestreada (cada `Ajustes::muestreo_ms`, p. ej. 250 ms): quién ve a quién. Emite solo el **primer contacto**
/// entre dos facciones; el detalle por columna es lo que alimentará los `Delta` del ámbito de vista.
pub struct GeneradorVision {
    vision: Vision,
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
            // Una columna con soldados en pie ve 30; un héroe solo, o con la tropa aniquilada, 15 (Niebla_De_Guerra §3).
            let participantes: Vec<u64> = e.columna.participantes.iter().map(|p| p.heroe).collect();
            let radio = vision::radios::columna(estado.militar.soldados_de(&participantes, &e.columna.escuadras) > 0);
            ojos.push(Observador { faccion: e.bandera(), radio, donde, graba: true });
            moviles.push(Movil::new(e.id, e.bandera(), donde));
        }
        // Las caravanas en el mapa se ven como móviles de la Facción de su plaza de origen; una «grande» en tránsito llama la atención a 60.
        let reglas = &estado.reglas.economia.comercio.flota;
        for c in estado.comercio.flota.iter().filter(|c| matches!(c.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando)) {
            let (Some(plan), Some(origen)) = (c.plan.as_ref(), estado.asentamientos.get(&(c.origen as u64))) else { continue };
            let mut m = Movil::new(entidad_caravana(c.id), origen.faccion, Donde::Plan(plan));
            if c.estado == EstadoCaravana::EnTransito && c.es_grande(reglas) {
                m.atencion = Some(vision::radios::CARAVANA_GRANDE);
            }
            moviles.push(m);
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
                if otro.bandera() != *faccion && self.contactos.insert((*faccion, otro.bandera())) {
                    nuevos.push((*faccion, otro.bandera(), visto));
                }
            }
        }
        self.previa = muestra;
        for (a, b, visto) in nuevos {
            cx.emitir(EventoNucleo {
                codigo: "vision.contacto".into(),
                en: t,
                ejercitos: vec![visto],
                asentamientos: vec![],
                punto: None,
                datos: vec![("faccionQueVe", a as i64), ("faccionVista", b as i64)],
            });
        }
    }
}
