//! El viaje de una caravana (Comercio_Caravanas §6-§7.3): un **plan** (`base::Plan`, vértices con su instante de paso) más los peajes
//! que aún no ha cruzado. Aquí está lo que no necesita mundo: peajes por progreso, progreso ↔ instante sobre un plan, el regreso por la
//! ruta invertida y un plan de velocidad uniforme. El coste del terreno y la red de caminos (más rápido sobre camino) los pone
//! `crates/rutas` al construir el plan real.

use crate::tipos::{AsentamientoId, Carga};
use asentamiento::datos::{razon, UNO};
use base::{Instante, Mil, Plan, Punto, Vertice, MS_POR_MINUTO};

/// Un paso forzado por la zona de una ciudad ajena que la caravana aún no ha cruzado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Peaje {
    pub ciudad: AsentamientoId,
    /// Avance sobre la longitud total de la ruta, en ppm (0..UNO), en el que se cruza.
    pub progreso: i64,
}

/// Lo que se cobró en un peaje.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CobroPeaje {
    pub ciudad: AsentamientoId,
    pub cobrado: Carga,
}

/// Longitud de una polilínea (suma de tramos, cada uno redondeado hacia abajo, como `Plan::longitud`).
pub fn longitud_ruta(ruta: &[Punto]) -> Mil {
    ruta.windows(2).map(|w| w[0].dist(w[1])).sum()
}

/// Los peajes de una ruta trazada: cada parada es `(ciudad, distancia acumulada hasta ella)`; el progreso es esa distancia entre la
/// longitud **total** de la ruta final (§7.2).
pub fn peajes_de_paradas(paradas: &[(AsentamientoId, Mil)], longitud_total: Mil) -> Vec<Peaje> {
    paradas.iter().map(|&(ciudad, d)| Peaje { ciudad, progreso: if longitud_total <= 0 { 0 } else { razon(d.clamp(0, longitud_total), longitud_total) } }).collect()
}

/// Plan de velocidad uniforme sobre una ruta: `velocidad` en milésimas de unidad por minuto. `None` si la velocidad es 0 o no hay ruta.
pub fn plan_uniforme(ruta: &[Punto], salida: Instante, velocidad: Mil, generacion: u32) -> Option<Plan> {
    if velocidad <= 0 || ruta.len() < 2 {
        return None;
    }
    let mut acum: Mil = 0;
    let mut vertices = vec![Vertice { p: ruta[0], en: salida }];
    for w in ruta.windows(2) {
        acum += w[0].dist(w[1]);
        let ms = (acum as i128 * MS_POR_MINUTO as i128 + velocidad as i128 / 2) / velocidad as i128;
        vertices.push(Vertice { p: w[1], en: salida + ms as Instante });
    }
    Some(Plan::nuevo(generacion, vertices))
}

/// Avance (ppm, 0..UNO) del plan en `t`, medido por distancia recorrida sobre la longitud total.
pub fn progreso_en(plan: &Plan, t: Instante) -> i64 {
    let total = plan.longitud();
    let (Some(p), Some(primero)) = (plan.posicion_en(t), plan.vertices.first()) else { return 0 };
    if total <= 0 {
        return if plan.fin().is_some_and(|f| t >= f) { UNO } else { 0 };
    }
    if t <= primero.en {
        return 0;
    }
    if plan.fin().is_some_and(|f| t >= f) {
        return UNO;
    }
    let i = plan.vertices.partition_point(|v| v.en <= t); // primer vértice con en > t
    let antes: Mil = plan.vertices[..i].windows(2).map(|w| w[0].p.dist(w[1].p)).sum();
    razon((antes + plan.vertices[i - 1].p.dist(p)).min(total), total)
}

/// Primer instante en que el plan alcanza el avance `progreso` (ppm): lo contrario de [`progreso_en`], redondeado hacia arriba para que
/// en ese instante el avance sea `>= progreso`.
pub fn instante_de_progreso(plan: &Plan, progreso: i64) -> Option<Instante> {
    let (primero, ultimo) = (plan.vertices.first()?, plan.vertices.last()?);
    let total = plan.longitud();
    if progreso <= 0 || total <= 0 {
        return Some(primero.en);
    }
    if progreso >= UNO {
        return Some(ultimo.en);
    }
    let objetivo = (total as i128 * progreso as i128 + UNO as i128 - 1) / UNO as i128; // distancia a recorrer, hacia arriba
    let mut acum: i128 = 0;
    for w in plan.vertices.windows(2) {
        let largo = w[0].p.dist(w[1].p) as i128;
        if acum + largo >= objetivo {
            if largo == 0 {
                return Some(w[1].en);
            }
            let dt = (w[1].en - w[0].en) as i128;
            let num = (objetivo - acum) * dt;
            return Some(w[0].en + ((num + largo - 1) / largo) as Instante);
        }
        acum += largo;
    }
    Some(ultimo.en)
}

/// La misma ruta recorrida al revés, empezando en `desde` y con las mismas duraciones tramo a tramo: el regreso **desanda el camino de
/// ida**, sin recalcular (§6.4).
pub fn invertir_plan(plan: &Plan, desde: Instante, generacion: u32) -> Plan {
    let fin = plan.fin().unwrap_or(0);
    let vertices = plan.vertices.iter().rev().map(|v| Vertice { p: v.p, en: desde + (fin - v.en) }).collect();
    Plan::nuevo(generacion, vertices)
}

/// La polilínea al revés.
pub fn invertir_ruta(ruta: &[Punto]) -> Vec<Punto> {
    ruta.iter().rev().copied().collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn p(x: Mil, y: Mil) -> Punto {
        Punto::new(x, y)
    }

    fn ruta() -> Vec<Punto> {
        vec![p(0, 0), p(30_000, 0), p(30_000, 40_000)] // 30 + 40 = 70 unidades
    }

    #[test]
    fn el_plan_uniforme_a_velocidad_24_tarda_lo_que_dice_la_cuenta() {
        let plan = plan_uniforme(&ruta(), 1_000, 24_000, 1).unwrap();
        assert_eq!(plan.vertices.len(), 3);
        // 30 unidades a 24 por minuto = 1,25 min = 75.000 ms; 70 unidades = 175.000 ms.
        assert_eq!(plan.vertices[1].en, 1_000 + 75_000);
        assert_eq!(plan.fin(), Some(1_000 + 175_000));
        assert!(plan_uniforme(&ruta(), 0, 0, 1).is_none());
        assert!(plan_uniforme(&[p(0, 0)], 0, 24_000, 1).is_none());
    }

    #[test]
    fn longitud_y_peajes_por_progreso() {
        let r = ruta();
        assert_eq!(longitud_ruta(&r), 70_000);
        let pj = peajes_de_paradas(&[(7, 30_000), (9, 70_000)], 70_000);
        assert_eq!(pj, vec![Peaje { ciudad: 7, progreso: 428_571 }, Peaje { ciudad: 9, progreso: UNO }]);
        assert_eq!(peajes_de_paradas(&[(7, 5)], 0)[0].progreso, 0);
    }

    #[test]
    fn progreso_e_instante_son_inversos() {
        let plan = plan_uniforme(&ruta(), 0, 24_000, 1).unwrap();
        assert_eq!(progreso_en(&plan, -5), 0);
        assert_eq!(progreso_en(&plan, 75_000), 428_571);
        assert_eq!(progreso_en(&plan, 87_500), 500_000);
        assert_eq!(progreso_en(&plan, 175_000), UNO);
        assert_eq!(progreso_en(&plan, 999_999), UNO);
        for pr in [1, 100_000, 428_571, 500_000, 900_000, 999_999] {
            let t = instante_de_progreso(&plan, pr).unwrap();
            assert!(progreso_en(&plan, t) >= pr, "en {t} el avance debe alcanzar {pr}");
            assert!(progreso_en(&plan, t - 1) <= pr, "y un instante antes aún no pasa de él");
        }
        assert_eq!(instante_de_progreso(&plan, 0), Some(0));
        assert_eq!(instante_de_progreso(&plan, UNO), Some(175_000));
    }

    #[test]
    fn el_regreso_desanda_la_ida_con_las_mismas_duraciones() {
        let plan = plan_uniforme(&ruta(), 0, 24_000, 1).unwrap();
        let vuelta = invertir_plan(&plan, 1_000_000, 2);
        assert_eq!(vuelta.generacion, 2);
        assert_eq!(vuelta.vertices.first().unwrap(), &Vertice { p: p(30_000, 40_000), en: 1_000_000 });
        assert_eq!(vuelta.vertices[1], Vertice { p: p(30_000, 0), en: 1_000_000 + 100_000 });
        assert_eq!(vuelta.fin(), Some(1_000_000 + 175_000));
        assert_eq!(vuelta.vertices.last().unwrap().p, p(0, 0), "acaba en el origen");
        assert_eq!(invertir_ruta(&ruta())[0], p(30_000, 40_000));
    }
}
