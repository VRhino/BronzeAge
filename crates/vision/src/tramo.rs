//! Horizonte de lo ajeno (Niebla_De_Guerra §5.4, Arquitectura/13 §5): de un plan ajeno solo viaja el tramo que cae
//! dentro de la vista y del horizonte corto, de modo que el destino final no se revela.

use base::{interpolar, Instante, Plan, Punto, Vertice};

/// Rectángulo de vista (milésimas), límites incluidos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub min: Punto,
    pub max: Punto,
}

impl Rect {
    pub fn contiene(&self, p: Punto) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
    fn acotar(&self, p: Punto) -> Punto {
        Punto::new(p.x.clamp(self.min.x, self.max.x), p.y.clamp(self.min.y, self.max.y))
    }
}

fn suelo(a: i128, b: i128) -> i128 {
    let q = a / b;
    if a % b != 0 && ((a < 0) != (b < 0)) { q - 1 } else { q }
}

fn techo(a: i128, b: i128) -> i128 {
    -suelo(-a, b)
}

/// Milisegundos `[lo, hi]` dentro del tramo `a → b` (`b.en > a.en`) en que la posición cae en `r` (exacto con enteros).
fn recorte(a: Vertice, b: Vertice, r: Rect) -> Option<(i64, i64)> {
    let dt = (b.en - a.en) as i128;
    let (mut lo, mut hi) = (0i128, dt);
    for (a0, d, mn, mx) in [(a.p.x, b.p.x - a.p.x, r.min.x, r.max.x), (a.p.y, b.p.y - a.p.y, r.min.y, r.max.y)] {
        if d == 0 {
            if a0 < mn || a0 > mx {
                return None;
            }
            continue;
        }
        let (n1, n2) = ((mn - a0) as i128 * dt, (mx - a0) as i128 * dt);
        let d = d as i128;
        if d > 0 {
            lo = lo.max(techo(n1, d));
            hi = hi.min(suelo(n2, d));
        } else {
            lo = lo.max(techo(n2, d));
            hi = hi.min(suelo(n1, d));
        }
    }
    (lo <= hi).then_some((lo as i64, hi as i64))
}

/// Tramo del plan visible en `t`: lo que cae dentro de `rect` y de `[t, t + horizonte_ms]`, con vértices de corte
/// interpolados en los bordes. Vacío si el plan no entra en la vista en ese horizonte.
///
/// - El plan no se revela más allá del horizonte: el último vértice es la posición en `t + horizonte_ms`, no el destino.
///   Solo si el plan **acaba** dentro del horizonte se ve dónde se detiene (la columna se para ahí a la vista).
/// - Devuelve solo el **primer pedazo contiguo** dentro del rectángulo; el resto, en el siguiente muestreo.
/// - Un plan que ya acabó (o de un solo punto) da un vértice en `t` si está dentro.
pub fn tramo_visible(plan: &Plan, t: Instante, horizonte_ms: i64, rect: Rect) -> Vec<Vertice> {
    let (Some(p0), Some(fin_plan)) = (plan.posicion_en(t), plan.fin()) else { return vec![] };
    if fin_plan <= t {
        return if rect.contiene(p0) { vec![Vertice { p: p0, en: t }] } else { vec![] };
    }
    let limite = t + horizonte_ms;
    let mut pts = vec![Vertice { p: p0, en: t }];
    for v in plan.vertices.iter().filter(|v| v.en > t && v.en <= limite) {
        if pts.last().is_some_and(|u| u.en == v.en) {
            pts.pop(); // salto instantáneo: manda el último vértice
        }
        pts.push(*v);
    }
    if fin_plan > limite && pts.last().is_some_and(|u| u.en < limite) {
        pts.push(Vertice { p: plan.posicion_en(limite).unwrap_or(p0), en: limite });
    }

    let mut out: Vec<Vertice> = Vec::new();
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let Some((lo, hi)) = recorte(a, b, rect) else {
            if out.is_empty() { continue } else { break }
        };
        if out.is_empty() {
            out.push(Vertice { p: rect.acotar(interpolar(a, b, a.en + lo)), en: a.en + lo });
        } else if lo != 0 {
            break;
        }
        out.push(Vertice { p: rect.acotar(interpolar(a, b, a.en + hi)), en: a.en + hi });
        if hi != (b.en - a.en) {
            break;
        }
    }
    out
}
