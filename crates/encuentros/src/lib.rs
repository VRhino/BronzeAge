//! Encuentros exactos entre planes: rejilla espacio-temporal, persecución e intercepción (fase 5, Arquitectura/13 §3).
//!
//! Lo que cambia el juego (dos columnas que se acercan, llegar a la puerta de una plaza, interceptar una presa) se
//! calcula **exacto y con eventos**, nunca por sondeo. El modelo: cada tramo es recto a velocidad constante y la posición
//! en un instante entero es la **racional exacta** (no la redondeada de [`base::interpolar`]); «a distancia ≤ r» se
//! decide con enteros (`i128`/`u128`) y el resultado es el **primer milisegundo entero** en que la condición ya es cierta.
//!
//! Método: la distancia relativa entre dos tramos es una cuadrática convexa en el tiempo, y la de persecución
//! (`distancia ≤ alcance`) o bien es convexa o bien pasa de falsa a cierta una sola vez. En ambos casos basta evaluar un
//! predicado **exacto** en el vértice y bisecar: sin raíces cuadradas ni `f64`.
//!
//! Cotas de validez (holgadas para el mundo real, 2.667.000 × 2.000.000 milésimas): un tramo que se mueve dura menos de
//! 2^26 ms (~18 h), los radios son < 2^16 milésimas y las coordenadas < 2^26. Un tramo **quieto** puede durar lo que sea.
//!
//! Piezas: [`primer_acercamiento`], [`llegada_a_radio`] / [`salida_de_radio`], [`interceptar`], la [`Rejilla`] y los
//! eventos para la cola ([`eventos`]).

pub mod eventos;
pub mod rejilla;

pub use eventos::{evento_de_persecucion, eventos_de_plan, programar, Programado, Puerta, EventoEspacial, PRIORIDAD_ENCUENTROS};
pub use rejilla::{Encuentro, Rejilla};

use base::{interpolar, Instante, Mil, Plan, Punto, Vertice};

/// Un tramo recto recorrido a velocidad constante entre dos vértices (`a.en <= b.en`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tramo {
    pub a: Vertice,
    pub b: Vertice,
}

impl Tramo {
    pub fn nuevo(p0: Punto, t0: Instante, p1: Punto, t1: Instante) -> Self {
        debug_assert!(t0 <= t1, "un tramo no retrocede en el tiempo");
        Self { a: Vertice { p: p0, en: t0 }, b: Vertice { p: p1, en: t1 } }
    }
    /// Una entidad parada en `p` entre dos instantes.
    pub fn quieto(p: Punto, desde: Instante, hasta: Instante) -> Self {
        Self::nuevo(p, desde, p, hasta)
    }
    /// Los tramos de un plan, en orden.
    pub fn del_plan(plan: &Plan) -> impl Iterator<Item = Tramo> + '_ {
        plan.vertices.windows(2).map(|w| Tramo { a: w[0], b: w[1] })
    }
    pub fn es_quieto(&self) -> bool {
        self.a.p == self.b.p
    }
    /// Posición (redondeada, la de [`Plan::posicion_en`]) en `t`, anclada a los extremos.
    pub fn posicion(&self, t: Instante) -> Punto {
        interpolar(self.a, self.b, t.clamp(self.a.en, self.b.en))
    }
    /// Se «mueve» en cero milisegundos (salto): no tiene trayectoria que cruzar.
    fn degenerado(&self) -> bool {
        self.a.en == self.b.en && !self.es_quieto()
    }
    /// Duración con la que se calcula la velocidad: un tramo quieto cuenta 1 ms (su velocidad es 0 igualmente) para
    /// que un aparcamiento larguísimo no agrande los números.
    fn duracion(&self) -> i128 {
        if self.es_quieto() { 1 } else { (self.b.en - self.a.en) as i128 }
    }
    fn delta(&self) -> (i128, i128) {
        ((self.b.p.x - self.a.p.x) as i128, (self.b.p.y - self.a.p.y) as i128)
    }
}

// --- Búsqueda del primer milisegundo ----------------------------------------------------------------------------

enum Modo {
    /// El predicado es cierto en un intervalo (cuadrática convexa); `vertice` = suelo del mínimo real.
    Convexa { vertice: i128 },
    /// El predicado, si es cierto en algún punto de la ventana, lo es también en su extremo final y en lo que sigue.
    Monotona,
}

/// Primer `s` en `[0, l]` con `pred(s)` cierto, y un punto cierto (testigo). `None` si no hay ninguno.
fn buscar(l: i64, pred: &impl Fn(i64) -> bool, modo: Modo) -> Option<(i64, i64)> {
    if pred(0) {
        return Some((0, 0));
    }
    let testigo = match modo {
        Modo::Monotona => pred(l).then_some(l)?,
        Modo::Convexa { vertice } => {
            let v = vertice.clamp(0, l as i128) as i64;
            [v, (v + 1).min(l)].into_iter().find(|&s| pred(s))?
        }
    };
    let (mut lo, mut hi) = (0, testigo); // pred(lo) falso, pred(hi) cierto
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if pred(mid) { hi = mid } else { lo = mid }
    }
    Some((hi, testigo))
}

/// Último `s` cierto de un intervalo convexo, desde un testigo cierto.
fn ultimo(testigo: i64, l: i64, pred: &impl Fn(i64) -> bool) -> i64 {
    let (mut lo, mut hi) = (testigo, l + 1); // pred(lo) cierto, pred(hi) falso (virtual)
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if pred(mid) { lo = mid } else { hi = mid }
    }
    lo
}

/// `(c·w, w·w)`; si desbordan, se reducen ambos vectores en bits (solo orienta la búsqueda del vértice).
fn punto_punto(c: (i128, i128), w: (i128, i128)) -> (i128, i128) {
    for k in (0..120).step_by(4) {
        let (cx, cy, wx, wy) = (c.0 >> k, c.1 >> k, w.0 >> k, w.1 >> k);
        let dot = cx.checked_mul(wx).zip(cy.checked_mul(wy)).and_then(|(a, b)| a.checked_add(b));
        let ww = wx.checked_mul(wx).zip(wy.checked_mul(wy)).and_then(|(a, b)| a.checked_add(b));
        if let Some((dot, ww)) = dot.zip(ww) {
            return (dot, ww);
        }
    }
    (0, 0)
}

/// Movimiento relativo `a - b` en la ventana común: `D·(a-b)(T0 + s) = c + s·w`, con `D` = mcm de las duraciones.
struct Relativo {
    t0: Instante,
    l: i64,
    c: (i128, i128),
    w: (i128, i128),
    d: i128,
}

fn mcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn relativo(a: &Tramo, b: &Tramo) -> Option<Relativo> {
    if a.degenerado() || b.degenerado() {
        return None;
    }
    let (t0, t1) = (a.a.en.max(b.a.en), a.b.en.min(b.b.en));
    if t0 > t1 {
        return None;
    }
    let (da, db) = (a.duracion(), b.duracion());
    let d = da / mcd(da, db) * db;
    let (ma, mb) = (d / da, d / db);
    let ((ax, ay), (bx, by)) = (a.delta(), b.delta());
    let (ka, kb) = ((t0 - a.a.en) as i128, (t0 - b.a.en) as i128);
    let c = (
        d * (a.a.p.x - b.a.p.x) as i128 + ka * ax * ma - kb * bx * mb,
        d * (a.a.p.y - b.a.p.y) as i128 + ka * ay * ma - kb * by * mb,
    );
    let w = (ax * ma - bx * mb, ay * ma - by * mb);
    Some(Relativo { t0, l: t1 - t0, c, w, d })
}

impl Relativo {
    /// ¿A distancia ≤ `radio` en `t0 + s`? Exacto: compara `|c + s·w|` con `radio·D` sin pasar de `u128`.
    fn dentro(&self, radio: Mil) -> impl Fn(i64) -> bool + '_ {
        let lim = (radio as i128 * self.d) as u128;
        let lim2 = lim.saturating_mul(lim);
        move |s| {
            let s = s as i128;
            let (qx, qy) = ((self.c.0 + s * self.w.0).unsigned_abs(), (self.c.1 + s * self.w.1).unsigned_abs());
            if qx > lim || qy > lim {
                return false;
            }
            let (x2, y2) = (qx.saturating_mul(qx), qy.saturating_mul(qy));
            x2 <= lim2 && y2 <= lim2 - x2
        }
    }
    fn modo(&self) -> Modo {
        let (dot, ww) = punto_punto(self.c, self.w);
        Modo::Convexa { vertice: if ww == 0 { 0 } else { (-dot).div_euclid(ww) } }
    }
    /// `(primer, ultimo)` milisegundos de la ventana con distancia ≤ `radio`.
    fn intervalo(&self, radio: Mil, con_ultimo: bool) -> Option<(Instante, Instante)> {
        if radio < 0 {
            return None;
        }
        let pred = self.dentro(radio);
        let (primero, testigo) = buscar(self.l, &pred, self.modo())?;
        let fin = if con_ultimo { ultimo(testigo, self.l, &pred) } else { testigo };
        Some((self.t0 + primero, self.t0 + fin))
    }
}

/// Primer instante entero (ms) en que dos tramos rectos quedan a distancia ≤ `radio`, dentro de la ventana temporal
/// que comparten (extremos incluidos). Si ya empiezan dentro del radio, el inicio de la ventana. `None` si no se acercan.
pub fn primer_acercamiento(a: Tramo, b: Tramo, radio: Mil) -> Option<Instante> {
    relativo(&a, &b)?.intervalo(radio, false).map(|(primero, _)| primero)
}

/// Primer y último instante entero de la ventana común con distancia ≤ `radio`.
pub fn intervalo_de_acercamiento(a: Tramo, b: Tramo, radio: Mil) -> Option<(Instante, Instante)> {
    relativo(&a, &b)?.intervalo(radio, true)
}

/// Primer instante en que el plan llega a ≤ `radio` de `centro` (p. ej. la puerta de una plaza, radio 2). Si el plan ya
/// empieza dentro, su inicio. Un plan de un solo vértice solo cuenta en ese instante.
pub fn llegada_a_radio(plan: &Plan, centro: Punto, radio: Mil) -> Option<Instante> {
    if let [v] = plan.vertices.as_slice() {
        return (v.p.dist2(centro) <= (radio as i128).pow(2)).then_some(v.en);
    }
    Tramo::del_plan(plan).find_map(|t| primer_acercamiento(t, Tramo::quieto(centro, t.a.en, t.b.en), radio))
}

/// Primer instante en que el plan, tras haber estado a ≤ `radio` de `centro`, ya está **fuera** (distancia > radio).
/// `None` si nunca entra o si el plan acaba dentro (se queda ahí).
pub fn salida_de_radio(plan: &Plan, centro: Punto, radio: Mil) -> Option<Instante> {
    let mut dentro = false;
    for t in Tramo::del_plan(plan) {
        let Some((primero, ultimo)) = intervalo_de_acercamiento(t, Tramo::quieto(centro, t.a.en, t.b.en), radio) else {
            if dentro {
                return Some(t.a.en); // no se da: tramos contiguos comparten el vértice. Defensa ante saltos de cero ms.
            }
            continue;
        };
        if dentro && primero > t.a.en {
            return Some(t.a.en);
        }
        dentro = true;
        if ultimo < t.b.en {
            return Some(ultimo + 1);
        }
    }
    None
}

// --- Persecución ------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Intercepcion {
    pub instante: Instante,
    /// Dónde está la presa en ese instante.
    pub punto: Punto,
}

/// Más allá del último vértice la presa se queda quieta; para ese tramo final se usa este horizonte (2^40 ms ≈ 35 años).
const HORIZONTE_MS: i64 = 1 << 40;

/// Primer `t >= desde` en que `perseguidor`, saliendo en línea recta hacia la presa a `velocidad` (milésimas por
/// segundo), puede estar a ≤ `radio` de ella: `dist(P, presa(t)) <= velocidad·(t - desde)/1000 + radio`. Es el instante
/// de la intercepción y `punto` la posición de la presa entonces. Antes del plan la presa está quieta en su primer
/// vértice y después del último, en el último (siempre se la alcanza si `velocidad > 0`). Un plan nuevo de la presa
/// obliga a llamar de nuevo: es un evento, no un sondeo.
pub fn interceptar(perseguidor: Punto, desde: Instante, velocidad: Mil, radio: Mil, presa: &Plan) -> Option<Intercepcion> {
    let (primero, ultimo) = (presa.vertices.first()?, presa.vertices.last()?);
    let antes = (desde < primero.en).then(|| Tramo::quieto(primero.p, desde, primero.en));
    let despues = Tramo::quieto(ultimo.p, ultimo.en.max(desde), ultimo.en.max(desde) + HORIZONTE_MS);
    let instante = antes
        .into_iter()
        .chain(Tramo::del_plan(presa).filter(|t| t.b.en >= desde))
        .chain(std::iter::once(despues))
        .find_map(|t| interceptar_tramo(perseguidor, desde, velocidad, radio, &t))?;
    Some(Intercepcion { instante, punto: presa.posicion_en(instante)? })
}

fn interceptar_tramo(p: Punto, desde: Instante, v: Mil, radio: Mil, t: &Tramo) -> Option<Instante> {
    if t.degenerado() || radio < 0 || v < 0 {
        return None;
    }
    let t0 = t.a.en.max(desde);
    if t0 > t.b.en {
        return None;
    }
    let da = t.duracion();
    let (dx, dy) = t.delta();
    let k = (t0 - t.a.en) as i128;
    // da·(presa - P)(t0 + s) = c + s·w
    let c = (da * (t.a.p.x - p.x) as i128 + k * dx, da * (t.a.p.y - p.y) as i128 + k * dy);
    let w = (dx, dy);
    // alcance·1000·da = da·(u0 + v·s), con u0 = v·(t0 - desde) + 1000·radio
    let u0 = v as i128 * (t0 - desde) as i128 + 1000 * radio as i128;
    let (v, l) = (v as i128, t.b.en - t0);
    let pred = |s: i64| {
        let s = s as i128;
        let (cx, cy) = (c.0 + s * w.0, c.1 + s * w.1);
        let alcance = da * (u0 + v * s);
        (cx * cx + cy * cy) * 1_000_000 <= alcance * alcance
    };
    // q(s) = 1e6·|c + s·w|² - (da·u0 + da·v·s)² = A·s² + B·s + K; con A <= 0 la presa no escapa de un perseguidor más
    // rápido: pred pasa de falso a cierto una sola vez.
    let a2 = 1_000_000 * (w.0 * w.0 + w.1 * w.1) - da * da * v * v;
    let modo = if a2 > 0 {
        let b = 2 * (1_000_000 * (c.0 * w.0 + c.1 * w.1) - da * da * u0 * v);
        Modo::Convexa { vertice: (-b).div_euclid(2 * a2) }
    } else {
        Modo::Monotona
    };
    buscar(l, &pred, modo).map(|(primero, _)| t0 + primero)
}

#[cfg(test)]
mod pruebas;
