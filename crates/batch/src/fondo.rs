//! Fondo del mapa del informe: el terreno completo del mundo (relieve con sombreado, bosque, ríos) con la piedra y las
//! parcelas de bosque pintadas como píxeles. Una celda del raster del artefacto = un píxel.

use informe::bmp_uri;
use mundo::Mundo;

type Rgb = [u8; 3];

pub const AGUA: Rgb = [70, 120, 175];
pub const LLANO: Rgb = [168, 190, 120];
pub const COLINA: Rgb = [170, 150, 100];
pub const MONTANA: Rgb = [135, 120, 105];
pub const BOSQUE: Rgb = [38, 92, 52];
pub const RIO: Rgb = [40, 105, 190];
pub const PIEDRA: Rgb = [255, 255, 255];
pub const PARCELA: Rgb = [255, 120, 0];

/// Densidad de bosque (0..255) a partir de la cual se pinta bosque (≈ 0,15, la `densidadMinima` del generador).
const BOSQUE_MIN: u8 = 38;

fn mezcla(a: Rgb, b: Rgb, t: f64) -> Rgb {
    std::array::from_fn(|i| (a[i] as f64 * (1.0 - t) + b[i] as f64 * t).round() as u8)
}

fn sombrear(c: Rgb, k: f64) -> Rgb {
    std::array::from_fn(|i| (c[i] as f64 * k).clamp(0.0, 255.0) as u8)
}

/// URI de datos (BMP) con el terreno del mundo, o `None` si la rejilla es vacía.
pub fn fondo(m: &Mundo) -> Option<String> {
    let (cols, filas) = m.rejilla();
    let lado = mundo::TAMANO_CELDA as i64;
    let centro = |c: usize, f: usize| (c as i64 * lado + lado / 2, f as i64 * lado + lado / 2);
    let altura = |c: i64, f: i64| {
        let (c, f) = (c.clamp(0, cols as i64 - 1) as usize, f.clamp(0, filas as i64 - 1) as usize);
        let (x, y) = centro(c, f);
        m.altura_en(x, y).unwrap_or(0) as f64 / 65535.0
    };
    let mut px = vec![[0u8; 3]; cols * filas];
    for f in 0..filas {
        for c in 0..cols {
            let (x, y) = centro(c, f);
            let (ci, fi) = (c as i64, f as i64);
            let clase = m.terreno_en(x, y).map_or(0, |t| t as i32);
            let h = altura(ci, fi);
            // Sombreado de ladera con luz del noroeste, más un poco de claridad con la altura.
            let sombra = (1.0 + (altura(ci - 1, fi - 1) - altura(ci + 1, fi + 1)) * 60.0).clamp(0.7, 1.3) * (0.92 + 0.16 * h);
            let mut color = match clase {
                1 => AGUA,
                2 => [214, 205, 160],
                3 => sombrear(LLANO, sombra),
                4 => sombrear(COLINA, sombra),
                5 => sombrear(MONTANA, sombra),
                6 => sombrear([225, 225, 232], sombra),
                _ => [200, 200, 200],
            };
            let densidad = m.densidad_bosque_en(x, y).unwrap_or(0);
            if clase != 1 && densidad >= BOSQUE_MIN {
                color = mezcla(color, sombrear(BOSQUE, 0.85 + 0.3 * h), (densidad as f64 / 255.0 * 1.15).min(0.85));
            }
            px[f * cols + c] = color;
        }
    }
    // Ríos: cada tramo por Bresenham sobre las celdas.
    for r in m.rios() {
        for par in r.puntos.windows(2) {
            let (a, b) = (((par[0].x / lado), (par[0].y / lado)), ((par[1].x / lado), (par[1].y / lado)));
            for (c, f) in linea(a, b) {
                if c >= 0 && f >= 0 && (c as usize) < cols && (f as usize) < filas {
                    px[f as usize * cols + c as usize] = RIO;
                }
            }
        }
    }
    let mut pintar = |x: i64, y: i64, color: Rgb| {
        let (c, f) = (x / lado, y / lado);
        if c >= 0 && f >= 0 && (c as usize) < cols && (f as usize) < filas {
            px[f as usize * cols + c as usize] = color;
        }
    };
    for p in m.parcelas() {
        if let Some(q) = p.centro.as_ref() {
            pintar(q.x, q.y, PARCELA);
        }
    }
    for n in m.nodos().iter().filter(|n| n.tipo == "piedra") {
        if let Some(q) = n.posicion.as_ref() {
            pintar(q.x, q.y, PIEDRA);
        }
    }
    bmp_uri(cols, filas, &px.concat())
}

/// Celdas de un segmento entre dos celdas (Bresenham), extremos incluidos.
fn linea(a: (i64, i64), b: (i64, i64)) -> Vec<(i64, i64)> {
    let (mut x, mut y) = a;
    let (dx, dy) = ((b.0 - a.0).abs(), -(b.1 - a.1).abs());
    let (sx, sy) = (if a.0 < b.0 { 1 } else { -1 }, if a.1 < b.1 { 1 } else { -1 });
    let mut err = dx + dy;
    let mut o = vec![];
    loop {
        o.push((x, y));
        if (x, y) == b {
            return o;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::linea;

    #[test]
    fn bresenham_incluye_extremos_y_es_conexa() {
        let l = linea((0, 0), (5, 2));
        assert_eq!((l[0], *l.last().unwrap()), ((0, 0), (5, 2)));
        assert!(l.windows(2).all(|w| (w[0].0 - w[1].0).abs() <= 1 && (w[0].1 - w[1].1).abs() <= 1));
        assert_eq!(linea((3, 3), (3, 3)), vec![(3, 3)]);
    }
}
