//! Niebla: lo explorado como máscara de bits sobre una rejilla de celdas cuadradas (Niebla_De_Guerra §2).
//!
//! Celda `i = fila · columnas + columna` → byte `i >> 3`, bit `i & 7`, LSB primero; fila 0 al norte. Una máscara tiene `⌈columnas · filas / 8⌉` bytes. El lado de la celda es el del bloque
//! `EXPLORACION.tamanoCelda` (5 unidades); la rejilla se deriva de los límites del mapa ([`rejilla`]), no se guarda.
//!
//! Lo explorado no se muestrea: se **barre** a lo largo del plan de cada ojo que se mueve ([`Mascara::barrer`], con el recorrido de [`recorrido`]), así que una columna graba exactamente lo
//! que alcanzó a ver entre dos instantes, sin depender de cada cuánto se mire. [`ultima_vez`] dice el último instante de un recorrido en que un punto estaba a la vista (la fecha de una ficha).

use base::{interpolar, isqrt, Instante, Mil, Plan, Punto, Vertice};
use contrato::v1::NieblaProyectada;

/// Columnas y filas de la rejilla que cubre un mapa de `ancho × alto` milésimas con celdas de lado `tamano` (§2.1: `ceil(ancho / tamanoCelda)`).
pub fn rejilla(ancho: Mil, alto: Mil, tamano: Mil) -> (usize, usize) {
    ((ancho.max(0) as usize).div_ceil(tamano as usize), (alto.max(0) as usize).div_ceil(tamano as usize))
}

/// Largo máximo de un trozo de recorrido (milésimas): parte los tramos largos para que las cuentas exactas con enteros quepan en `i128`.
const TROZO: Mil = 100_000;

/// Máscara de bits sobre la rejilla.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mascara {
    columnas: usize,
    filas: usize,
    tamano: i64,
    bits: Vec<u8>,
}

impl Mascara {
    pub fn nueva(columnas: usize, filas: usize, tamano: i64) -> Self {
        Self { columnas, filas, tamano, bits: vec![0; (columnas * filas).div_ceil(8)] }
    }

    /// La máscara guardada en `bytes` (§2.1): más corta se completa con ceros, más larga se trunca, y los bits sobrantes del último byte se limpian.
    pub fn desde_bytes(columnas: usize, filas: usize, tamano: i64, bytes: &[u8]) -> Self {
        let mut m = Self::nueva(columnas, filas, tamano);
        let n = m.bits.len();
        let k = n.min(bytes.len());
        m.bits[..k].copy_from_slice(&bytes[..k]);
        let total = columnas * filas;
        if !total.is_multiple_of(8) && n > 0 {
            m.bits[n - 1] &= (1u8 << (total % 8)) - 1;
        }
        m
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bits
    }

    pub fn columnas(&self) -> usize {
        self.columnas
    }

    pub fn filas(&self) -> usize {
        self.filas
    }

    pub fn tamano(&self) -> i64 {
        self.tamano
    }

    pub fn get(&self, columna: usize, fila: usize) -> bool {
        columna < self.columnas && fila < self.filas && {
            let i = fila * self.columnas + columna;
            self.bits[i >> 3] >> (i & 7) & 1 == 1
        }
    }

    /// ¿Está explorada la celda del punto? (`false` fuera del mapa.)
    pub fn explorado_en(&self, p: Punto) -> bool {
        let (c, f) = (p.x.div_euclid(self.tamano), p.y.div_euclid(self.tamano));
        c >= 0 && f >= 0 && self.get(c as usize, f as usize)
    }

    /// Marca la celda; `true` si el bit era nuevo.
    fn poner(&mut self, columna: usize, fila: usize) -> bool {
        let i = fila * self.columnas + columna;
        let (b, m) = (i >> 3, 1u8 << (i & 7));
        let nuevo = self.bits[b] & m == 0;
        self.bits[b] |= m;
        nuevo
    }

    /// Marca la celda que contiene `p`, si está en el mapa.
    fn poner_en(&mut self, p: Punto) -> bool {
        let (c, f) = (p.x.div_euclid(self.tamano), p.y.div_euclid(self.tamano));
        c >= 0 && f >= 0 && (c as usize) < self.columnas && (f as usize) < self.filas && self.poner(c as usize, f as usize)
    }

    /// `marcarVisto` (§2.2): marca la celda del ojo siempre y las celdas cuyo **centro** está a `≤ radio`. Devuelve `true` si hubo algún bit nuevo.
    pub fn marcar_vision(&mut self, centro: Punto, radio: Mil) -> bool {
        let t = self.tamano;
        let mut nuevo = self.poner_en(centro);
        if radio < 0 || self.filas == 0 {
            return nuevo;
        }
        let f0 = (centro.y - radio).div_euclid(t).max(0);
        let f1 = (centro.y + radio).div_euclid(t).min(self.filas as i64 - 1);
        let r2 = radio as i128 * radio as i128;
        for fila in f0..=f1 {
            // Por fila: las columnas con centro en [x - semi, x + semi], con semi = ⌊√(r² − dy²)⌋ (exacto con enteros).
            let dy = (fila * t + t / 2 - centro.y) as i128;
            if dy * dy > r2 {
                continue;
            }
            let semi = isqrt(r2 - dy * dy) as i64;
            let c0 = (centro.x - semi - t / 2 + t - 1).div_euclid(t).max(0); // ⌈(x − semi − t/2) / t⌉
            let c1 = (centro.x + semi - t / 2).div_euclid(t).min(self.columnas as i64 - 1);
            for col in c0..=c1 {
                nuevo |= self.poner(col as usize, fila as usize);
            }
        }
        nuevo
    }

    /// Lo que ve un ojo de alcance `radio` que va de `a` a `b` en línea recta: las celdas cuyo centro está a `≤ radio` del segmento, y las de los dos extremos (el ojo siempre ve su celda).
    /// `true` si hubo algún bit nuevo. Exacto con enteros.
    // ponytail: recorre la caja del segmento celda a celda (≈ 3 celdas mirada por cada marcada); por filas, como `marcar_vision`, si llega a pesar.
    fn marcar_tramo(&mut self, a: Punto, b: Punto, radio: Mil) -> bool {
        let t = self.tamano;
        let mut nuevo = self.poner_en(a) | self.poner_en(b);
        if radio < 0 || self.filas == 0 || self.columnas == 0 {
            return nuevo;
        }
        let celda = |v: Mil, n: usize| v.div_euclid(t).clamp(0, n as i64 - 1);
        let (c0, c1) = (celda(a.x.min(b.x) - radio, self.columnas), celda(a.x.max(b.x) + radio, self.columnas));
        let (f0, f1) = (celda(a.y.min(b.y) - radio, self.filas), celda(a.y.max(b.y) + radio, self.filas));
        for fila in f0..=f1 {
            for col in c0..=c1 {
                if distancia2_a_segmento(Punto::new(col * t + t / 2, fila * t + t / 2), a, b) <= radio as i128 * radio as i128 {
                    nuevo |= self.poner(col as usize, fila as usize);
                }
            }
        }
        nuevo
    }

    /// Barre un recorrido ([`recorrido`]) con un ojo de alcance `radio`: todo lo que vio por el camino. `true` si hubo algún bit nuevo.
    pub fn barrer(&mut self, recorrido: &[Vertice], radio: Mil) -> bool {
        match recorrido {
            [] => false,
            [v] => self.marcar_vision(v.p, radio),
            _ => recorrido.windows(2).fold(false, |n, w| self.marcar_tramo(w[0].p, w[1].p, radio) | n),
        }
    }

    /// OR bit a bit (`fundirExploraciones`). Con rejillas distintas no hace nada.
    pub fn fundir(&mut self, otra: &Mascara) {
        if self.bits.len() == otra.bits.len() {
            self.bits.iter_mut().zip(&otra.bits).for_each(|(a, b)| *a |= b);
        }
    }

    /// Número de celdas marcadas.
    pub fn contar(&self) -> usize {
        self.bits.iter().map(|b| b.count_ones() as usize).sum()
    }
}

/// La `NieblaProyectada` (§2.3) con las dos máscaras sobre la misma rejilla: `celdas` (lo explorado alguna vez) y `visibles` (lo que se ve ahora, que puede incluir celdas sin explorar: la
/// visión prestada no explora).
pub fn niebla_proyectada(celdas: &Mascara, visibles: &Mascara) -> NieblaProyectada {
    NieblaProyectada { tamano_celda: celdas.tamano as u32, columnas: celdas.columnas as u32, filas: celdas.filas as u32, celdas: celdas.bits.clone(), visibles: visibles.bits.clone() }
}

/// Distancia al cuadrado de `p` al segmento `a → b`, exacta (en `i128`; los segmentos de [`recorrido`] no pasan de [`TROZO`] y `p` está cerca).
fn distancia2_a_segmento(p: Punto, a: Punto, b: Punto) -> i128 {
    let (dx, dy) = ((b.x - a.x) as i128, (b.y - a.y) as i128);
    let (wx, wy) = ((p.x - a.x) as i128, (p.y - a.y) as i128);
    let (dd, wd, ww) = (dx * dx + dy * dy, wx * dx + wy * dy, wx * wx + wy * wy);
    if dd == 0 || wd <= 0 {
        return ww;
    }
    if wd >= dd {
        let (ux, uy) = ((p.x - b.x) as i128, (p.y - b.y) as i128);
        return ux * ux + uy * uy;
    }
    // |w|² − (w·d)²/|d|², redondeado hacia arriba para no ver de más.
    (ww * dd - wd * wd + dd - 1) / dd
}

/// Por dónde pasa un ojo entre `desde` y `hasta` (`desde ≤ hasta`): las posiciones de `plan` en esos instantes y sus vértices de en medio, con los tramos largos partidos en trozos de a lo sumo
/// 100 unidades. Sin plan (quieto en `quieto`), el sitio en los dos instantes. Es lo que barre [`Mascara::barrer`] y donde busca [`ultima_vez`].
pub fn recorrido(plan: &Plan, quieto: Punto, desde: Instante, hasta: Instante) -> Vec<Vertice> {
    let hasta = hasta.max(desde);
    let en = |t: Instante| Vertice { p: plan.posicion_en(t).unwrap_or(quieto), en: t };
    let mut puntos = vec![en(desde)];
    puntos.extend(plan.vertices.iter().filter(|v| v.en > desde && v.en < hasta).copied());
    puntos.push(en(hasta));
    let mut out: Vec<Vertice> = vec![puntos[0]];
    for w in puntos.windows(2) {
        let (a, b) = (w[0], w[1]);
        let largo = (b.p.x - a.p.x).abs().max((b.p.y - a.p.y).abs());
        let trozos = (largo / TROZO + 1).max(1);
        for k in 1..=trozos {
            let p = if k == trozos { b.p } else { partir(a, b, k, trozos) };
            let t = a.en + (b.en - a.en) * k / trozos;
            out.push(Vertice { p, en: t });
        }
    }
    out
}

/// El punto `k / n` del camino de `a` a `b`, por su geometría (también si los dos vértices tienen el mismo instante).
fn partir(a: Vertice, b: Vertice, k: i64, n: i64) -> Punto {
    if b.en > a.en {
        interpolar(a, b, a.en + (b.en - a.en) * k / n)
    } else {
        Punto::new(a.p.x + (b.p.x - a.p.x) * k / n, a.p.y + (b.p.y - a.p.y) * k / n)
    }
}

/// El último instante del recorrido en que `p` estaba a `≤ radio` del ojo (no estricto), o `None` si nunca lo estuvo. Exacto con enteros salvo el redondeo hacia abajo del instante de salida.
pub fn ultima_vez(recorrido: &[Vertice], p: Punto, radio: Mil) -> Option<Instante> {
    let r2 = radio as i128 * radio as i128;
    let cerca = |q: Punto| {
        let (x, y) = ((q.x - p.x) as i128, (q.y - p.y) as i128);
        x * x + y * y <= r2
    };
    if let [v] = recorrido {
        return cerca(v.p).then_some(v.en);
    }
    for w in recorrido.windows(2).rev() {
        let (a, b) = (w[0], w[1]);
        if cerca(b.p) {
            return Some(b.en);
        }
        // Posición `a + s·d` con `s ∈ [0, 1]`: dentro mientras dd·s² + 2·fd·s + (ff − r²) ≤ 0. La salida es la raíz mayor; si cae en [0, 1) el ojo lo vio hasta entonces.
        let (dx, dy) = ((b.p.x - a.p.x) as i128, (b.p.y - a.p.y) as i128);
        let (fx, fy) = ((a.p.x - p.x) as i128, (a.p.y - p.y) as i128);
        let (dd, fd, ff) = (dx * dx + dy * dy, fx * dx + fy * dy, fx * fx + fy * fy);
        if dd == 0 {
            if cerca(a.p) {
                return Some(b.en);
            }
            continue;
        }
        let disc = fd * fd - dd * (ff - r2);
        if disc < 0 {
            continue;
        }
        // s₊ · dd. Con `b` fuera, s₊ ≥ 1 quiere decir que el disco queda entero más allá del tramo.
        let num = -fd + isqrt(disc);
        if num < 0 || num >= dd {
            continue;
        }
        return Some(a.en + (num * (b.en - a.en) as i128 / dd) as i64);
    }
    None
}
