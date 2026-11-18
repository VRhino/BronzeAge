//! Las orillas de los ríos (Economia_Construccion §14.3, `ORILLA_RIO`; decisión del autor del 2026-11-18): nada que se levante en el mapa puede
//! quedar dentro del cauce que se ve. El río corta el paso solo en las celdas de 4 u que pisa su trazo, y no en el hueco de los vados, así que
//! la transitabilidad no basta: aquí se mide la distancia al trazo mismo.
//!
//! Los tramos de los ríos del artefacto van en una rejilla de cubos de [`LADO_CUBO`]; cada tramo se apunta en todos los cubos que toca su caja
//! ensanchada en [`ALCANCE`], así que una consulta de hasta ese alcance solo mira su cubo.

use base::{Mil, Punto};
use mundo::Mundo;
use std::collections::HashMap;

/// Lado de un cubo de la rejilla, en milésimas (16 u).
const LADO_CUBO: Mil = 16_000;
/// Lo más lejos que se pregunta (media anchura navegable más el radio más grande, con holgura), en milésimas.
pub(crate) const ALCANCE: Mil = 8_000;

#[derive(Clone, Copy, Debug)]
struct Tramo {
    a: Punto,
    b: Punto,
    navegable: bool,
}

/// Los tramos de todos los ríos, con su rejilla de búsqueda.
pub(crate) struct Cauces {
    tramos: Vec<Tramo>,
    cubos: HashMap<(i64, i64), Vec<u32>>,
}

fn cubo(v: Mil) -> i64 {
    v.div_euclid(LADO_CUBO)
}

/// La distancia al cuadrado (en milésimas²) de `p` al segmento `a`-`b`, en enteros: el paso tiene que ser el mismo en cualquier máquina.
fn dist2_a_tramo(p: Punto, a: Punto, b: Punto) -> i128 {
    let (dx, dy) = ((b.x - a.x) as i128, (b.y - a.y) as i128);
    let (px, py) = ((p.x - a.x) as i128, (p.y - a.y) as i128);
    let l2 = dx * dx + dy * dy;
    let t = px * dx + py * dy;
    if l2 == 0 || t <= 0 {
        return px * px + py * py;
    }
    if t >= l2 {
        let (qx, qy) = ((p.x - b.x) as i128, (p.y - b.y) as i128);
        return qx * qx + qy * qy;
    }
    // Distancia a la recta: (px·dy − py·dx)² / l2, redondeada hacia arriba (así nunca se da por lejos lo que está justo en el límite).
    let c = px * dy - py * dx;
    (c * c + l2 - 1) / l2
}

impl Cauces {
    pub(crate) fn de(mundo: &Mundo) -> Self {
        let mut tramos = vec![];
        for r in mundo.rios() {
            let puntos: Vec<Punto> = r.puntos.iter().map(|q| Punto::new(q.x, q.y)).collect();
            tramos.extend(puntos.windows(2).map(|w| Tramo { a: w[0], b: w[1], navegable: r.navegable }));
        }
        Self::de_tramos(tramos)
    }

    fn de_tramos(tramos: Vec<Tramo>) -> Self {
        let mut cubos: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
        for (i, t) in tramos.iter().enumerate() {
            let (x0, x1) = (cubo(t.a.x.min(t.b.x) - ALCANCE), cubo(t.a.x.max(t.b.x) + ALCANCE));
            let (y0, y1) = (cubo(t.a.y.min(t.b.y) - ALCANCE), cubo(t.a.y.max(t.b.y) + ALCANCE));
            for cx in x0..=x1 {
                for cy in y0..=y1 {
                    cubos.entry((cx, cy)).or_default().push(i as u32);
                }
            }
        }
        Self { tramos, cubos }
    }

    /// ¿Queda `p` a menos de la media anchura de algún río más `radio` de su trazo? `media` y `media_navegable` son las medias anchuras del
    /// cauce dibujado (`ORILLA_RIO`). Con `radio` + media anchura por encima de [`ALCANCE`] solo se miran los tramos hasta [`ALCANCE`].
    pub(crate) fn junto_al_rio(&self, p: Punto, radio: Mil, media: Mil, media_navegable: Mil) -> bool {
        let Some(ids) = self.cubos.get(&(cubo(p.x), cubo(p.y))) else { return false };
        ids.iter().any(|&i| {
            let t = self.tramos[i as usize];
            let limite = (if t.navegable { media_navegable } else { media } + radio) as i128;
            dist2_a_tramo(p, t.a, t.b) < limite * limite
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rio(puntos: &[(i64, i64)], navegable: bool) -> Cauces {
        let p: Vec<Punto> = puntos.iter().map(|&(x, y)| Punto::new(x, y)).collect();
        Cauces::de_tramos(p.windows(2).map(|w| Tramo { a: w[0], b: w[1], navegable }).collect())
    }

    #[test]
    fn mide_la_distancia_al_trazo_y_no_a_la_celda() {
        // Un río recto por y = 10 u. Con media anchura 0,75 u y radio 0,75 u, el límite es 1,5 u.
        let c = rio(&[(0, 10_000), (40_000, 10_000)], false);
        assert!(c.junto_al_rio(Punto::new(20_000, 10_400), 750, 750, 1_500), "a 0,4 u, encima del agua");
        assert!(c.junto_al_rio(Punto::new(20_000, 11_499), 750, 750, 1_500), "a décimas del límite");
        assert!(!c.junto_al_rio(Punto::new(20_000, 11_500), 750, 750, 1_500), "a 1,5 u justos ya vale");
        assert!(!c.junto_al_rio(Punto::new(20_000, 30_000), 750, 750, 1_500), "lejos");
        // Más allá del extremo cuenta la distancia al extremo.
        assert!(c.junto_al_rio(Punto::new(41_000, 10_000), 750, 750, 1_500));
        assert!(!c.junto_al_rio(Punto::new(41_600, 10_000), 750, 750, 1_500));
    }

    #[test]
    fn el_navegable_pide_su_media_anchura_y_el_radio_es_el_de_lo_que_se_levanta() {
        let c = rio(&[(0, 0), (30_000, 30_000)], true);
        // A 2 u de la diagonal: con 1,5 + 0,75 = 2,25 u está junto al río; con un radio de 0,4 u (1,9 u), no.
        let p = Punto::new(15_000 + 1_414, 15_000 - 1_414);
        assert!(c.junto_al_rio(p, 750, 750, 1_500));
        assert!(!c.junto_al_rio(p, 400, 750, 1_500));
    }

    #[test]
    fn la_rejilla_encuentra_los_tramos_en_el_borde_de_su_cubo() {
        let c = rio(&[(15_900, -5_000), (15_900, 50_000)], false);
        assert!(c.junto_al_rio(Punto::new(16_100, 20_000), 750, 750, 1_500), "el punto cae en el cubo de al lado del tramo");
        assert!(c.junto_al_rio(Punto::new(15_900 + 5_000, 20_000), 4_800, 750, 1_500), "una plaza a 5 u");
    }
}
