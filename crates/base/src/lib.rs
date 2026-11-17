//! Tipos y matemática comunes del núcleo (fase 5): tiempo de mundo, punto fijo, planes de movimiento, ritmos, reloj y
//! generador aleatorio determinista. Todo en enteros de 64 bits: dos máquinas dan el mismo resultado (Arquitectura/13 §4).
//!
//! - **Instante**: milisegundos de mundo desde la época. «Vence» significa `instante >= vencimiento`.
//! - **Mil**: milésimas (espacio en milésimas de unidad estratégica; cantidades de juego en milésimas).

/// Milisegundos de mundo desde la época (`2026-01-01T00:00:00.000Z`).
pub type Instante = i64;
/// Milésimas de unidad (espacio y cantidades).
pub type Mil = i64;

pub const MS_POR_MINUTO: i64 = 60_000;

/// Una unidad entera de recurso, en milésimas. **Todo lo almacenado va en unidades enteras** (decisión C3b del autor, 2026-10-24): almacenes,
/// carros, caravanas, almacén personal y existencias guardan múltiplos de `UNIDAD`; lo continuo acumula un resto oculto aparte.
pub const UNIDAD: Mil = 1000;

/// `x` redondeado hacia abajo a unidades enteras (nunca se crea de más).
pub const fn entero(x: Mil) -> Mil {
    x.div_euclid(UNIDAD) * UNIDAD
}

/// `x` redondeado hacia arriba a unidades enteras (lo que se cobra: un coste fraccionario nunca sale más barato).
pub const fn entero_arriba(x: Mil) -> Mil {
    -entero(-x)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Punto {
    pub x: Mil,
    pub y: Mil,
}

impl Punto {
    pub const fn new(x: Mil, y: Mil) -> Self {
        Self { x, y }
    }
    /// Distancia al cuadrado, sin desbordar (milésimas²).
    pub fn dist2(self, o: Punto) -> i128 {
        let (dx, dy) = ((self.x - o.x) as i128, (self.y - o.y) as i128);
        dx * dx + dy * dy
    }
    /// Distancia redondeada hacia abajo.
    pub fn dist(self, o: Punto) -> Mil {
        isqrt(self.dist2(o)) as Mil
    }
}

/// Raíz cuadrada entera (suelo) exacta.
pub fn isqrt(n: i128) -> i128 {
    if n <= 0 {
        return 0;
    }
    let mut x = (n as f64).sqrt() as i128;
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    x
}

/// Un vértice del plan: dónde está la entidad en un instante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vertice {
    pub p: Punto,
    pub en: Instante,
}

/// Plan de movimiento: tramos rectos con el instante de paso por cada vértice (monótono). Sin vértices, no hay plan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// Sube cada vez que el plan cambia; los eventos que dependían del plan viejo se descartan.
    pub generacion: u32,
    pub vertices: Vec<Vertice>,
}

impl Plan {
    pub fn nuevo(generacion: u32, vertices: Vec<Vertice>) -> Self {
        debug_assert!(vertices.windows(2).all(|w| w[0].en <= w[1].en), "los instantes del plan deben ser monótonos");
        Self { generacion, vertices }
    }

    pub fn inicio(&self) -> Option<Instante> {
        self.vertices.first().map(|v| v.en)
    }
    pub fn fin(&self) -> Option<Instante> {
        self.vertices.last().map(|v| v.en)
    }

    /// Posición en `t`: antes del primer vértice, la del primero; después del último, la del último.
    pub fn posicion_en(&self, t: Instante) -> Option<Punto> {
        let (primero, ultimo) = (self.vertices.first()?, self.vertices.last()?);
        if t <= primero.en {
            return Some(primero.p);
        }
        if t >= ultimo.en {
            return Some(ultimo.p);
        }
        let i = self.vertices.partition_point(|v| v.en <= t); // primer vértice con en > t
        let (a, b) = (self.vertices[i - 1], self.vertices[i]);
        Some(interpolar(a, b, t))
    }

    /// Tramo `(a, b)` activo en `t` (para el cálculo de encuentros), o `None` si `t` está fuera.
    pub fn tramo_en(&self, t: Instante) -> Option<(Vertice, Vertice)> {
        let i = self.vertices.partition_point(|v| v.en <= t);
        if i == 0 || i >= self.vertices.len() {
            return None;
        }
        Some((self.vertices[i - 1], self.vertices[i]))
    }

    /// Longitud total (milésimas), suma de tramos redondeados hacia abajo.
    pub fn longitud(&self) -> Mil {
        self.vertices.windows(2).map(|w| w[0].p.dist(w[1].p)).sum()
    }
}

/// Interpolación lineal exacta (i128) entre dos vértices; el redondeo es hacia el primero.
pub fn interpolar(a: Vertice, b: Vertice, t: Instante) -> Punto {
    let dt = (b.en - a.en) as i128;
    if dt == 0 {
        return b.p;
    }
    let k = (t - a.en) as i128;
    Punto {
        x: a.p.x + ((b.p.x - a.p.x) as i128 * k / dt) as Mil,
        y: a.p.y + ((b.p.y - a.p.y) as i128 * k / dt) as Mil,
    }
}

/// Una cantidad que crece o se gasta de forma continua: valor en `t0` más un ritmo por minuto, con tope y suelo.
/// Nada se «actualiza por tick»: el valor en cualquier instante sale de la fórmula.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ritmo {
    pub valor_en_t0: Mil,
    pub t0: Instante,
    /// Variación por minuto de mundo (negativa si se gasta).
    pub por_minuto: Mil,
    pub maximo: Option<Mil>,
    pub minimo: Option<Mil>,
}

impl Ritmo {
    pub fn constante(valor: Mil, t0: Instante) -> Self {
        Self { valor_en_t0: valor, t0, por_minuto: 0, maximo: None, minimo: None }
    }

    pub fn valor_en(&self, t: Instante) -> Mil {
        let dt = (t - self.t0) as i128;
        let v = self.valor_en_t0 as i128 + self.por_minuto as i128 * dt / MS_POR_MINUTO as i128;
        let v = v.clamp(i64::MIN as i128, i64::MAX as i128) as Mil;
        let v = self.maximo.map_or(v, |m| v.min(m));
        self.minimo.map_or(v, |m| v.max(m))
    }

    /// Primer instante `>= desde` en que el valor alcanza `objetivo` (por arriba si crece, por abajo si decrece).
    /// `None` si no lo alcanza nunca (ritmo cero, sentido contrario o tope/suelo antes del objetivo).
    pub fn instante_de_valor(&self, objetivo: Mil, desde: Instante) -> Option<Instante> {
        let actual = self.valor_en(desde);
        if actual == objetivo {
            return Some(desde);
        }
        let sube = self.por_minuto > 0;
        if self.por_minuto == 0 || (objetivo > actual) != sube {
            return None;
        }
        if sube && self.maximo.is_some_and(|m| objetivo > m) {
            return None;
        }
        if !sube && self.minimo.is_some_and(|m| objetivo < m) {
            return None;
        }
        // valor_en_t0 + por_minuto * dt / 60000 >= objetivo  →  dt = ceil((objetivo - valor_en_t0) * 60000 / por_minuto)
        let num = (objetivo as i128 - self.valor_en_t0 as i128) * MS_POR_MINUTO as i128;
        let den = self.por_minuto as i128;
        let dt = if (num >= 0) == (den > 0) { (num.abs() + den.abs() - 1) / den.abs() } else { -(num.abs() / den.abs()) };
        let t = self.t0 + dt as Instante;
        Some(t.max(desde))
    }

    /// El mismo ritmo, reanclado en `t` con el valor que tenía (cambia `por_minuto` sin saltos).
    pub fn reanclado(&self, t: Instante, nuevo_por_minuto: Mil) -> Self {
        Self { valor_en_t0: self.valor_en(t), t0: t, por_minuto: nuevo_por_minuto, ..*self }
    }
}

/// Reloj de mundo = reloj de pared × factor. Se pausa y se congela sin perder el instante (Reloj_Y_Eventos §1 y §10).
/// El reloj de pared no entra en el estado del mundo: solo convierte «ahora» en un instante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelojMundo {
    pared_base_ms: i64,
    mundo_base: Instante,
    /// Segundos de mundo por segundo real × 1000 (1000 = tiempo real).
    factor_milesimas: u32,
    pausado: bool,
}

impl RelojMundo {
    pub fn nuevo(pared_ms: i64, mundo: Instante) -> Self {
        Self { pared_base_ms: pared_ms, mundo_base: mundo, factor_milesimas: 1000, pausado: false }
    }
    pub fn mundo_en(&self, pared_ms: i64) -> Instante {
        if self.pausado {
            return self.mundo_base;
        }
        self.mundo_base + ((pared_ms - self.pared_base_ms) as i128 * self.factor_milesimas as i128 / 1000) as Instante
    }
    pub fn factor_milesimas(&self) -> u32 {
        self.factor_milesimas
    }
    pub fn pausado(&self) -> bool {
        self.pausado
    }
    pub fn fijar_factor(&mut self, pared_ms: i64, factor_milesimas: u32) {
        self.reanclar(pared_ms);
        self.factor_milesimas = factor_milesimas;
    }
    pub fn pausar(&mut self, pared_ms: i64) {
        self.reanclar(pared_ms);
        self.pausado = true;
    }
    pub fn reanudar(&mut self, pared_ms: i64) {
        self.pared_base_ms = pared_ms;
        self.pausado = false;
    }
    /// Pared en que el mundo llegará a `t` (con el factor actual), o `None` si está pausado o `t` ya pasó.
    pub fn pared_de(&self, t: Instante) -> Option<i64> {
        if self.pausado || self.factor_milesimas == 0 || t < self.mundo_base {
            return None;
        }
        Some(self.pared_base_ms + ((t - self.mundo_base) as i128 * 1000 / self.factor_milesimas as i128) as i64)
    }
    fn reanclar(&mut self, pared_ms: i64) {
        self.mundo_base = self.mundo_en(pared_ms);
        self.pared_base_ms = pared_ms;
    }
}

/// Generador aleatorio determinista (SplitMix64). Mismo `semilla` y mismo uso → misma secuencia en cualquier máquina.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn nuevo(semilla: u64) -> Self {
        Self(semilla)
    }
    /// Semilla derivada de otra y de un nombre de etapa (FNV-1a), como `derivarSeed` del generador de mundo.
    pub fn derivado(semilla: u64, etapa: &str) -> Self {
        let mut h: u64 = 0xcbf29ce484222325 ^ semilla;
        for b in etapa.bytes() {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
        Self(h)
    }
    pub fn siguiente(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    /// Entero en `[0, n)`.
    pub fn por_debajo(&mut self, n: u64) -> u64 {
        ((self.siguiente() as u128 * n as u128) >> 64) as u64
    }
    /// Estado interno, para guardarlo en las fotos y retomar la misma secuencia.
    pub fn estado(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> Plan {
        Plan::nuevo(1, vec![
            Vertice { p: Punto::new(0, 0), en: 1_000 },
            Vertice { p: Punto::new(10_000, 0), en: 11_000 },
            Vertice { p: Punto::new(10_000, 20_000), en: 31_000 },
        ])
    }

    #[test]
    fn plan_interpola_y_se_ancla_en_los_extremos() {
        let p = plan();
        assert_eq!(p.posicion_en(0), Some(Punto::new(0, 0)));
        assert_eq!(p.posicion_en(6_000), Some(Punto::new(5_000, 0)));
        assert_eq!(p.posicion_en(11_000), Some(Punto::new(10_000, 0)));
        assert_eq!(p.posicion_en(21_000), Some(Punto::new(10_000, 10_000)));
        assert_eq!(p.posicion_en(99_000), Some(Punto::new(10_000, 20_000)));
        assert_eq!(p.longitud(), 30_000);
        assert_eq!(Plan::default().posicion_en(5), None);
    }

    #[test]
    fn tramo_activo() {
        let p = plan();
        assert_eq!(p.tramo_en(500), None);
        assert_eq!(p.tramo_en(1_000).unwrap().1.en, 11_000);
        assert_eq!(p.tramo_en(31_000), None);
    }

    #[test]
    fn ritmo_crece_con_tope_y_cruza_umbrales() {
        let r = Ritmo { valor_en_t0: 100_000, t0: 0, por_minuto: 60_000, maximo: Some(500_000), minimo: None };
        assert_eq!(r.valor_en(0), 100_000);
        assert_eq!(r.valor_en(30_000), 130_000);
        assert_eq!(r.valor_en(10 * MS_POR_MINUTO), 500_000);
        assert_eq!(r.instante_de_valor(160_000, 0), Some(60_000));
        assert_eq!(r.instante_de_valor(500_000, 0), Some(400_000), "400.000 milésimas a 60.000 por minuto = 400.000 ms");
        assert_eq!(r.instante_de_valor(600_000, 0), None, "por encima del tope no se alcanza");
        assert_eq!(r.instante_de_valor(50_000, 0), None, "sentido contrario");
    }

    #[test]
    fn ritmo_que_se_gasta_y_se_reancla() {
        let r = Ritmo { valor_en_t0: 1_000_000, t0: 0, por_minuto: -100_000, maximo: None, minimo: Some(0) };
        assert_eq!(r.valor_en(5 * MS_POR_MINUTO), 500_000);
        assert_eq!(r.valor_en(20 * MS_POR_MINUTO), 0);
        assert_eq!(r.instante_de_valor(0, 0), Some(10 * MS_POR_MINUTO));
        let r2 = r.reanclado(2 * MS_POR_MINUTO, -50_000);
        assert_eq!(r2.valor_en(2 * MS_POR_MINUTO), 800_000);
        assert_eq!(r2.valor_en(4 * MS_POR_MINUTO), 700_000);
    }

    #[test]
    fn reloj_factor_pausa_y_reanuda_sin_saltos() {
        let mut r = RelojMundo::nuevo(1_000, 50_000);
        assert_eq!(r.mundo_en(3_000), 52_000);
        r.fijar_factor(3_000, 60_000); // ×60
        assert_eq!(r.mundo_en(4_000), 52_000 + 60_000);
        assert_eq!(r.pared_de(52_000 + 120_000), Some(5_000));
        r.pausar(4_000);
        assert_eq!(r.mundo_en(99_000), 112_000);
        r.reanudar(100_000);
        assert_eq!(r.mundo_en(101_000), 112_000 + 60_000);
    }

    #[test]
    fn rng_determinista() {
        let (mut a, mut b) = (Rng::derivado(42, "movimiento"), Rng::derivado(42, "movimiento"));
        assert!((0..100).all(|_| a.siguiente() == b.siguiente()));
        assert_ne!(Rng::derivado(42, "x").siguiente(), Rng::derivado(42, "y").siguiente());
        let mut r = Rng::nuevo(7);
        assert!((0..1000).all(|_| r.por_debajo(10) < 10));
    }

    #[test]
    fn redondeo_a_unidades() {
        assert_eq!((entero(2_999), entero(3_000), entero(0), entero(-1)), (2_000, 3_000, 0, -1_000));
        assert_eq!((entero_arriba(2_001), entero_arriba(3_000), entero_arriba(0)), (3_000, 3_000, 0));
    }

    #[test]
    fn isqrt_exacta() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(99), 9);
        assert_eq!(isqrt(100), 10);
        assert_eq!(isqrt(i64::MAX as i128 * 4), 6_074_000_999);
        assert_eq!(Punto::new(0, 0).dist(Punto::new(3_000, 4_000)), 5_000);
    }
}
