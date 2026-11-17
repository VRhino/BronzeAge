//! Horizonte de lo ajeno (Niebla_De_Guerra §5.4, Arquitectura/13 §5): de un plan ajeno solo viaja el tramo que cae
//! al alcance de los ojos que lo ven y dentro del horizonte corto, de modo que el destino final no se revela.

use base::{interpolar, isqrt, Instante, Mil, Plan, Punto, Vertice};

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
}

fn suelo(a: i128, b: i128) -> i128 {
    let q = a / b;
    if a % b != 0 && ((a < 0) != (b < 0)) { q - 1 } else { q }
}

fn techo(a: i128, b: i128) -> i128 {
    -suelo(-a, b)
}

/// Milisegundos `[lo, hi]` dentro del tramo `a → b` (`b.en > a.en`) en que la posición cae en el disco de centro `c` y radio `r`, límite incluido
/// (enteros: la raíz por defecto y los extremos hacia dentro, así que todo instante del intervalo está de verdad dentro).
fn en_disco(a: Vertice, b: Vertice, c: Punto, r: Mil) -> Option<(i128, i128)> {
    let dt = (b.en - a.en) as i128;
    let (dx, dy) = ((b.p.x - a.p.x) as i128, (b.p.y - a.p.y) as i128);
    let (ex, ey) = ((a.p.x - c.x) as i128, (a.p.y - c.y) as i128);
    // |e + d·s/dt|² ≤ r²  ⇔  A·u² + B·u + C ≤ 0 con u = s/dt ∈ [0, 1].
    let (aa, cc) = (dx * dx + dy * dy, ex * ex + ey * ey - r as i128 * r as i128);
    if aa == 0 {
        return (cc <= 0).then_some((0, dt));
    }
    let bb = 2 * (ex * dx + ey * dy);
    let discriminante = bb * bb - 4 * aa * cc;
    if discriminante < 0 {
        return None;
    }
    let raiz = isqrt(discriminante);
    let (lo, hi) = (techo((-bb - raiz) * dt, 2 * aa).max(0), suelo((-bb + raiz) * dt, 2 * aa).min(dt));
    (lo <= hi).then_some((lo, hi))
}

/// Milisegundos `[lo, hi]` del primer pedazo del tramo `a → b` al alcance de algún ojo (la unión de los discos, juntando los que se solapan).
fn al_alcance(a: Vertice, b: Vertice, ojos: &[(Punto, Mil)]) -> Option<(i64, i64)> {
    let mut trozos: Vec<(i128, i128)> = ojos.iter().filter_map(|&(c, r)| en_disco(a, b, c, r)).collect();
    trozos.sort_unstable();
    let mut trozos = trozos.into_iter();
    let (lo, mut hi) = trozos.next()?;
    for (l, h) in trozos {
        if l > hi {
            break;
        }
        hi = hi.max(h);
    }
    Some((lo as i64, hi as i64))
}

/// Tramo del plan visible en `t`: lo que cae al alcance de los `ojos` (centro y radio) y dentro de `[t, t + horizonte_ms]`, con vértices de corte
/// interpolados en el borde de los discos. Vacío si el plan no entra en lo que ven en ese horizonte.
///
/// - El plan no se revela más allá del horizonte: el último vértice es la posición en `t + horizonte_ms`, no el destino.
///   Solo si el plan **acaba** dentro del horizonte se ve dónde se detiene (la columna se para ahí a la vista).
/// - Devuelve solo el **primer pedazo contiguo** al alcance de los ojos; el resto, en el siguiente muestreo.
/// - Un plan que ya acabó (o de un solo punto) da un vértice en `t` si está al alcance.
pub fn tramo_visible(plan: &Plan, t: Instante, horizonte_ms: i64, ojos: &[(Punto, Mil)]) -> Vec<Vertice> {
    let (Some(p0), Some(fin_plan)) = (plan.posicion_en(t), plan.fin()) else { return vec![] };
    if fin_plan <= t {
        let visto = ojos.iter().any(|&(c, r)| (p0.x - c.x) as i128 * (p0.x - c.x) as i128 + (p0.y - c.y) as i128 * (p0.y - c.y) as i128 <= r as i128 * r as i128);
        return if visto { vec![Vertice { p: p0, en: t }] } else { vec![] };
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
        let Some((lo, hi)) = al_alcance(a, b, ojos) else {
            if out.is_empty() { continue } else { break }
        };
        if out.is_empty() {
            out.push(Vertice { p: interpolar(a, b, a.en + lo), en: a.en + lo });
        } else if lo != 0 {
            break;
        }
        out.push(Vertice { p: interpolar(a, b, a.en + hi), en: a.en + hi });
        if hi != (b.en - a.en) {
            break;
        }
    }
    out
}
