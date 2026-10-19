//! Gráficos SVG hechos a mano: línea, barras agrupadas, histograma y mapa.

use std::fmt::Write;

use crate::{esc, num, Grafico, Mapa, TipoGrafico};

const W: f64 = 480.0;
const H: f64 = 300.0;
const MI: f64 = 54.0; // margen izquierdo
const MD: f64 = 12.0;
const MS: f64 = 12.0;
const MB: f64 = 48.0; // margen inferior: marcas y título del eje
/// Colores de serie: clases `s0`..`s5` (trazo) y `f0`..`f5` (relleno) del CSS de la página.
const N_COLORES: usize = 6;

/// Rango de un eje y paso de las marcas.
struct Escala {
    lo: f64,
    hi: f64,
    paso: f64,
}

impl Escala {
    /// Escala "redonda" que cubre [min, max]; con `cero` el rango incluye el 0.
    fn nueva(mut min: f64, mut max: f64, cero: bool) -> Escala {
        if cero {
            min = min.min(0.0);
            max = max.max(0.0);
        }
        if max - min < 1e-12 {
            let d = if min == 0.0 { 1.0 } else { min.abs() * 0.1 };
            min -= d;
            max += d;
        }
        let bruto = (max - min) / 5.0;
        let mag = 10f64.powf(bruto.log10().floor());
        let n = bruto / mag;
        let mult = if n <= 1.0 {
            1.0
        } else if n <= 2.0 {
            2.0
        } else if n <= 5.0 {
            5.0
        } else {
            10.0
        };
        let paso = mag * mult;
        Escala { lo: (min / paso).floor() * paso, hi: (max / paso).ceil() * paso, paso }
    }

    fn marcas(&self) -> Vec<f64> {
        let n = ((self.hi - self.lo) / self.paso).round() as usize;
        (0..=n).map(|i| self.lo + i as f64 * self.paso).collect()
    }

    /// Posición de `v` entre `a` (en `lo`) y `b` (en `hi`).
    fn pos(&self, v: f64, a: f64, b: f64) -> f64 {
        a + (v - self.lo) / (self.hi - self.lo) * (b - a)
    }
}

/// Coordenada con un decimal.
fn c(v: f64) -> String {
    format!("{v:.1}")
}

/// Color recibido del JSON reducido a los caracteres de un color CSS (nada de inyectar estilos).
fn color_seguro(s: &str) -> String {
    s.chars().filter(|ch| ch.is_ascii_alphanumeric() || "#(),.% -".contains(*ch)).collect()
}

/// Puntos finitos de una serie (descarta lo que no sea un número).
fn limpios(puntos: &[(f64, f64)]) -> Vec<(f64, f64)> {
    puntos.iter().copied().filter(|(x, y)| x.is_finite() && y.is_finite()).collect()
}

/// Leyenda HTML con una muestra de color por serie.
fn leyenda(g: &Grafico) -> String {
    let mut s = String::from("<ul class=\"leyenda\">");
    for (i, serie) in g.series.iter().enumerate() {
        let _ = write!(s, "<li><span class=\"muestra f{}\"></span>{}</li>", i % N_COLORES, esc(&serie.nombre));
    }
    s + "</ul>"
}

/// Un gráfico completo (título, SVG y leyenda), o un aviso si no hay datos.
pub fn grafico(g: &Grafico) -> String {
    let series: Vec<Vec<(f64, f64)>> = g.series.iter().map(|s| limpios(&s.puntos)).collect();
    let mut o = format!("<section class=\"grafico\"><h3>{}</h3>", esc(&g.titulo));
    if series.iter().all(|s| s.is_empty()) {
        o.push_str("<p class=\"vacio\">Sin datos para este gráfico.</p></section>");
        return o;
    }
    let _ = write!(o, "<svg viewBox=\"0 0 {W} {H}\" role=\"img\" aria-label=\"{}\">", esc(&g.titulo));
    match g.tipo {
        TipoGrafico::Linea => lineas(&mut o, g, &series),
        TipoGrafico::Barras => barras(&mut o, g, &series, 0.2),
        TipoGrafico::Histograma => barras(&mut o, g, &series, 0.0),
    }
    o.push_str("</svg>");
    o.push_str(&leyenda(g));
    o + "</section>"
}

/// Rejilla horizontal, marcas del eje Y, ejes y títulos de ambos ejes.
fn marco(o: &mut String, g: &Grafico, ey: &Escala) {
    let (y0, y1) = (MS, H - MB);
    for v in ey.marcas() {
        let y = ey.pos(v, y1, y0);
        let _ = write!(
            o,
            "<line class=\"rej\" x1=\"{}\" x2=\"{}\" y1=\"{y2}\" y2=\"{y2}\"/><text class=\"tk\" x=\"{}\" y=\"{}\" text-anchor=\"end\">{}</text>",
            c(MI),
            c(W - MD),
            c(MI - 6.0),
            c(y + 4.0),
            num(v),
            y2 = c(y)
        );
    }
    let _ = write!(
        o,
        "<line class=\"eje\" x1=\"{0}\" x2=\"{0}\" y1=\"{1}\" y2=\"{2}\"/><line class=\"eje\" x1=\"{0}\" x2=\"{3}\" y1=\"{2}\" y2=\"{2}\"/>",
        c(MI),
        c(y0),
        c(y1),
        c(W - MD)
    );
    let _ = write!(
        o,
        "<text class=\"tt\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
        c((MI + W - MD) / 2.0),
        c(H - 6.0),
        esc(&g.eje_x)
    );
    let _ = write!(
        o,
        "<text class=\"tt\" transform=\"translate(14 {}) rotate(-90)\" text-anchor=\"middle\">{}</text>",
        c((MS + H - MB) / 2.0),
        esc(&g.eje_y)
    );
}

fn lineas(o: &mut String, g: &Grafico, series: &[Vec<(f64, f64)>]) {
    let todos: Vec<(f64, f64)> = series.iter().flatten().copied().collect();
    let (xmin, xmax) = rango(todos.iter().map(|p| p.0));
    let (ymin, ymax) = rango(todos.iter().map(|p| p.1));
    let ex = Escala::nueva(xmin, xmax, false);
    let ey = Escala::nueva(ymin, ymax, true);
    marco(o, g, &ey);
    for v in ex.marcas() {
        let x = ex.pos(v, MI, W - MD);
        let _ = write!(
            o,
            "<line class=\"eje\" x1=\"{0}\" x2=\"{0}\" y1=\"{1}\" y2=\"{2}\"/><text class=\"tk\" x=\"{0}\" y=\"{3}\" text-anchor=\"middle\">{4}</text>",
            c(x),
            c(H - MB),
            c(H - MB + 4.0),
            c(H - MB + 18.0),
            num(v)
        );
    }
    for (i, s) in series.iter().enumerate() {
        let pts: Vec<(f64, f64)> =
            s.iter().map(|&(x, y)| (ex.pos(x, MI, W - MD), ey.pos(y, H - MB, MS))).collect();
        let lista: Vec<String> = pts.iter().map(|(x, y)| format!("{},{}", c(*x), c(*y))).collect();
        if pts.len() > 1 {
            let _ = write!(o, "<polyline class=\"l s{}\" points=\"{}\"/>", i % N_COLORES, lista.join(" "));
        }
        for (x, y) in pts {
            let _ = write!(o, "<circle class=\"f{}\" cx=\"{}\" cy=\"{}\" r=\"3\"/>", i % N_COLORES, c(x), c(y));
        }
    }
}

/// Mínimo y máximo de una lista no vacía de números.
fn rango(it: impl Iterator<Item = f64>) -> (f64, f64) {
    it.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(v), b.max(v)))
}

/// Barras agrupadas por categoría (valor de x); `hueco` es la fracción de ancho que se deja libre.
fn barras(o: &mut String, g: &Grafico, series: &[Vec<(f64, f64)>], hueco: f64) {
    let mut cats: Vec<f64> = Vec::new();
    for &(x, _) in series.iter().flatten() {
        if !cats.contains(&x) {
            cats.push(x);
        }
    }
    cats.sort_by(f64::total_cmp);
    let (ymin, ymax) = rango(series.iter().flatten().map(|p| p.1));
    let ey = Escala::nueva(ymin, ymax, true);
    marco(o, g, &ey);
    let ancho_cat = (W - MD - MI) / cats.len() as f64;
    let ancho_barra = ancho_cat * (1.0 - hueco) / series.len() as f64;
    let base = ey.pos(0.0, H - MB, MS);
    let cada = cats.len().div_ceil(12); // unas 12 etiquetas visibles como mucho
    for (j, &cat) in cats.iter().enumerate() {
        let x0 = MI + j as f64 * ancho_cat + ancho_cat * hueco / 2.0;
        if j % cada == 0 {
            let _ = write!(
                o,
                "<text class=\"tk\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
                c(MI + (j as f64 + 0.5) * ancho_cat),
                c(H - MB + 18.0),
                num(cat)
            );
        }
        for (i, s) in series.iter().enumerate() {
            let Some(&(_, y)) = s.iter().find(|p| p.0 == cat) else { continue };
            let yp = ey.pos(y, H - MB, MS);
            let _ = write!(
                o,
                "<rect class=\"f{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                i % N_COLORES,
                c(x0 + i as f64 * ancho_barra),
                c(yp.min(base)),
                c(ancho_barra.max(0.5)),
                c((yp - base).abs())
            );
        }
    }
}

/// Mapa del mundo con sus capas; `None` si el tamaño no es válido.
pub fn mapa(m: &Mapa) -> Option<String> {
    if !(m.ancho.is_finite() && m.alto.is_finite() && m.ancho > 0.0 && m.alto > 0.0) {
        return None;
    }
    let punto = m.ancho.max(m.alto) / 250.0;
    let mut o = format!(
        "<svg class=\"mapa\" viewBox=\"0 0 {w} {h}\" role=\"img\" aria-label=\"Mapa del mundo\"><rect class=\"mundo\" width=\"{w}\" height=\"{h}\"/>",
        w = c(m.ancho),
        h = c(m.alto)
    );
    if let Some(f) = m.fondo.as_deref().filter(|f| f.starts_with("data:image/bmp;base64,") && f.bytes().all(|b| b.is_ascii_alphanumeric() || b"+/=:;,-./".contains(&b))) {
        let _ = write!(o, "<image href=\"{f}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\" style=\"image-rendering:pixelated\"/>", c(m.ancho), c(m.alto));
    }
    for cp in &m.capas {
        let color = color_seguro(&cp.color);
        let _ = write!(o, "<g fill=\"{color}\" stroke=\"{color}\">");
        for k in cp.circulos.iter().filter(|k| k.x.is_finite() && k.y.is_finite() && k.r.is_finite()) {
            let _ = write!(
                o,
                "<circle class=\"zona\" cx=\"{}\" cy=\"{}\" r=\"{}\"/>",
                c(k.x),
                c(k.y),
                c(k.r.abs())
            );
        }
        for l in cp.lineas.iter().filter(|l| l.len() >= 2 && l.iter().all(|p| p.x.is_finite() && p.y.is_finite())) {
            let puntos: Vec<String> = l.iter().map(|p| format!("{},{}", c(p.x), c(p.y))).collect();
            let _ = write!(o, "<polyline class=\"via\" points=\"{}\"/>", puntos.join(" "));
        }
        for p in cp.puntos.iter().filter(|p| p.x.is_finite() && p.y.is_finite()) {
            let _ = write!(
                o,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" stroke=\"none\"/>",
                c(p.x),
                c(p.y),
                c(punto)
            );
        }
        o.push_str("</g>");
    }
    o.push_str("</svg><ul class=\"leyenda\">");
    for cp in &m.capas {
        let _ = write!(
            o,
            "<li><span class=\"muestra\" style=\"background:{}\"></span>{}</li>",
            color_seguro(&cp.color),
            esc(&cp.nombre)
        );
    }
    Some(o + "</ul>")
}
