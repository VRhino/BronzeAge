//! Geometría de rejilla: celdas, huellas y rectángulos (T §2.2). Todo en enteros.
//!
//! Convenciones: `+row` es el sur; `Rect.col/row` es la celda de menor índice; el centro de un rectángulo se
//! expresa en milésimas de unidad local (`Punto` de `base`), exacto para lados pares e impares.

use base::{isqrt, Punto};

/// Lado de una celda en unidades locales (`REJILLA_ASENTAMIENTO.tamanoCelda`, T §2.1).
pub const CELDA_UL: i32 = 3;
/// Milésimas por celda.
pub const CELDA_MIL: i64 = CELDA_UL as i64 * 1000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Celda {
    pub col: i32,
    pub row: i32,
}

impl Celda {
    pub const fn new(col: i32, row: i32) -> Self {
        Self { col, row }
    }
}

/// Tamaño en celdas de un edificio, sin girar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Huella {
    pub ancho: i32,
    pub alto: i32,
}

impl Huella {
    pub const fn new(ancho: i32, alto: i32) -> Self {
        Self { ancho, alto }
    }
    pub const fn girada(self) -> Self {
        Self { ancho: self.alto, alto: self.ancho }
    }
    pub const fn es_cuadrada(self) -> bool {
        self.ancho == self.alto
    }
}

/// Huella de cualquier tipo sin entrada propia en la tabla del catálogo (T §2.4).
pub const HUELLA_POR_DEFECTO: Huella = Huella::new(1, 1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rect {
    pub col: i32,
    pub row: i32,
    pub ancho: i32,
    pub alto: i32,
}

impl Rect {
    pub const fn new(col: i32, row: i32, ancho: i32, alto: i32) -> Self {
        Self { col, row, ancho, alto }
    }
    pub const fn fin_col(&self) -> i32 {
        self.col + self.ancho
    }
    pub const fn fin_row(&self) -> i32 {
        self.row + self.alto
    }
    pub const fn contiene(&self, c: Celda) -> bool {
        c.col >= self.col && c.col < self.col + self.ancho && c.row >= self.row && c.row < self.row + self.alto
    }
    pub fn celdas(&self) -> impl Iterator<Item = Celda> + '_ {
        (self.row..self.fin_row()).flat_map(move |row| (self.col..self.fin_col()).map(move |col| Celda { col, row }))
    }
    pub fn solapa(&self, o: &Rect) -> bool {
        self.col < o.fin_col() && o.col < self.fin_col() && self.row < o.fin_row() && o.row < self.fin_row()
    }
    /// Centro en milésimas de unidad local: `((col + ancho/2)·T, (row + alto/2)·T)`.
    pub fn centro(&self) -> Punto {
        Punto::new((2 * self.col + self.ancho) as i64 * (CELDA_MIL / 2), (2 * self.row + self.alto) as i64 * (CELDA_MIL / 2))
    }
    /// Inversa exacta de [`Rect::centro`]: `min = round(posicion / T − ancho/2)`.
    pub fn desde_centro(p: Punto, h: Huella) -> Rect {
        let col = (2 * p.x - h.ancho as i64 * CELDA_MIL + CELDA_MIL).div_euclid(2 * CELDA_MIL) as i32;
        let row = (2 * p.y - h.alto as i64 * CELDA_MIL + CELDA_MIL).div_euclid(2 * CELDA_MIL) as i32;
        Rect { col, row, ancho: h.ancho, alto: h.alto }
    }
    /// Anillo de una celda de grosor, con las cuatro esquinas (T §5.1).
    pub fn anillo(&self, grosor: i32) -> Vec<Celda> {
        let mut v = Vec::new();
        for g in 0..grosor {
            let (c0, c1, r0, r1) = (self.col - 1 - g, self.fin_col() + g, self.row - 1 - g, self.fin_row() + g);
            for col in c0..=c1 {
                v.push(Celda::new(col, r0));
                v.push(Celda::new(col, r1));
            }
            for row in r0 + 1..r1 {
                v.push(Celda::new(c0, row));
                v.push(Celda::new(c1, row));
            }
        }
        v
    }
    /// Celdas ortogonalmente pegadas, sin esquinas (por una diagonal no se sale). Orden fijo: arriba (izq→der),
    /// derecha (arriba→abajo), abajo (izq→der), izquierda (arriba→abajo).
    pub fn adyacentes(&self) -> Vec<Celda> {
        let mut v = Vec::with_capacity(2 * (self.ancho + self.alto) as usize);
        for col in self.col..self.fin_col() {
            v.push(Celda::new(col, self.row - 1));
        }
        for row in self.row..self.fin_row() {
            v.push(Celda::new(self.fin_col(), row));
        }
        for col in self.col..self.fin_col() {
            v.push(Celda::new(col, self.fin_row()));
        }
        for row in self.row..self.fin_row() {
            v.push(Celda::new(self.col - 1, row));
        }
        v
    }
    /// El rectángulo ensanchado `n` celdas por cada lado.
    pub fn ensanchado(&self, n: i32) -> Rect {
        Rect { col: self.col - n, row: self.row - n, ancho: self.ancho + 2 * n, alto: self.alto + 2 * n }
    }
    /// Chebyshev entre bordes: 0 si se tocan o solapan (T §6.5).
    pub fn hueco(&self, o: &Rect) -> i32 {
        let gx = (self.col - o.fin_col()).max(o.col - self.fin_col()).max(0);
        let gy = (self.row - o.fin_row()).max(o.row - self.fin_row()).max(0);
        gx.max(gy)
    }
    /// Largo de lado compartido con `o` (0 si solo se tocan en diagonal o no se tocan).
    pub fn borde_compartido(&self, o: &Rect) -> i32 {
        let ox = self.fin_col().min(o.fin_col()) - self.col.max(o.col);
        let oy = self.fin_row().min(o.fin_row()) - self.row.max(o.row);
        let tocan_en_y = self.row == o.fin_row() || o.row == self.fin_row();
        let tocan_en_x = self.col == o.fin_col() || o.col == self.fin_col();
        if tocan_en_y && ox > 0 {
            ox
        } else if tocan_en_x && oy > 0 {
            oy
        } else {
            0
        }
    }
    /// Distancia² (milésimas²) del origen al punto más cercano del rectángulo (0 si lo contiene).
    pub fn dist2_borde_al_origen(&self) -> i64 {
        let (x0, x1) = (self.col as i64 * CELDA_MIL, self.fin_col() as i64 * CELDA_MIL);
        let (y0, y1) = (self.row as i64 * CELDA_MIL, self.fin_row() as i64 * CELDA_MIL);
        let dx = x0.max(-x1).max(0);
        let dy = y0.max(-y1).max(0);
        dx * dx + dy * dy
    }
    /// Distancia² (milésimas²) del centro al origen.
    pub fn dist2_centro_al_origen(&self) -> i64 {
        let c = self.centro();
        c.x * c.x + c.y * c.y
    }
}

/// `((i − desfase) mod paso) == 0`, módulo siempre no negativo (T §5.1).
pub fn es_borde_de_manzana(i: i32, paso: i32, desfase: i32) -> bool {
    (i - desfase).rem_euclid(paso) == 0
}

/// Semidiagonal de la huella en milésimas de unidad local, redondeada hacia abajo.
pub fn media_diagonal_mil(h: Huella) -> i64 {
    let (a, b) = (h.ancho as i128, h.alto as i128);
    isqrt((a * a + b * b) * (CELDA_MIL as i128 / 2) * (CELDA_MIL as i128 / 2)) as i64
}
