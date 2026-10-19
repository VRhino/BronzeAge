//! Un dibujo de la ciudad de una plaza de ejemplo (piezas, calles y caminos) como SVG autónomo. El crate `informe` solo admite un mapa por
//! informe (el del mundo), así que este dibujo se escribe aparte, junto a `informe.html`, como `ciudad_ejemplo.svg`.

use crate::simpob::{Corrida, Villa};
use asentamiento::catalogo::Tipo;
use nucleo::urbanismo::PiezaVista;
use std::fmt::Write;
use trazado::{Categoria, Rol};

fn color(c: Categoria) -> &'static str {
    match c {
        Categoria::Afueras => "#a3b18a",
        Categoria::Generico => "#8d99ae",
        Categoria::Residencial => "#f4a261",
        Categoria::Industria => "#9d8189",
        Categoria::Militar => "#bc4749",
        Categoria::Mercado => "#2a9d8f",
    }
}

/// La plaza en pie de nivel 2 con más piezas (o, si ninguna llegó al 2, la que más tiene).
fn ejemplo(c: &Corrida) -> Option<&Villa> {
    let piezas = |v: &&Villa| v.urbe.ciudad().piezas().len();
    let vivas = || c.villas.iter().filter(|v| !v.a.destruido);
    vivas().filter(|v| v.a.nivel == 2).max_by_key(piezas).or_else(|| vivas().max_by_key(piezas))
}

/// El SVG de la ciudad de la plaza de ejemplo; `None` si no queda ninguna plaza en pie.
pub fn ciudad_ejemplo(c: &Corrida, reglas: &asentamiento::Reglas) -> Option<String> {
    let v = ejemplo(c)?;
    let radio = v.urbe.radio_ul(&v.a.urbano(reglas));
    let vista = v.urbe.vista(radio);
    // Encuadre: todas las piezas y el disco urbano, con margen.
    let (mut x0, mut y0, mut x1, mut y1) = (-radio, -radio, radio, radio);
    for p in &vista.piezas {
        let r = p.rect;
        (x0, y0, x1, y1) = (x0.min(r.x), y0.min(r.y), x1.max(r.x + r.ancho), y1.max(r.y + r.alto));
    }
    let m = 12;
    let (x0, y0, w, h) = (x0 - m, y0 - m - 14, x1 - x0 + 2 * m, y1 - y0 + 2 * m + 14);
    let mut s = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{x0} {y0} {w} {h}\" width=\"900\" font-family=\"sans-serif\">\n<rect x=\"{x0}\" y=\"{y0}\" width=\"{w}\" height=\"{h}\" fill=\"#fafaf7\"/>\n");
    let _ = writeln!(s, "<text x=\"{}\" y=\"{}\" font-size=\"5\">Plaza de ejemplo: nivel {}, {} piezas, radio de ciudad {} u.l., perfil {:?}. Contorno grueso = ancla. Gris = calles, beis = caminos.</text>", x0 + 4, y0 + 8, v.a.nivel, vista.piezas.len(), radio, v.urbe.ciudad().perfil_por_tradicion());
    let _ = writeln!(s, "<circle cx=\"0\" cy=\"0\" r=\"{radio}\" fill=\"none\" stroke=\"#999\" stroke-width=\"0.4\" stroke-dasharray=\"2 2\"/>");
    for (rs, relleno) in [(&vista.calles, "#cfcfcf"), (&vista.caminos, "#e3d5b0")] {
        for r in rs {
            let _ = writeln!(s, "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{relleno}\"/>", r.x, r.y, r.ancho, r.alto);
        }
    }
    for PiezaVista { id, tipo, categoria, rol, rect, .. } in &vista.piezas {
        let (trazo, grosor) = if *rol == Rol::Normal { ("#444", 0.3) } else { ("#000", 1.0) };
        let relleno = if *tipo == Tipo::CentroUrbano { "#e9c46a" } else { color(*categoria) };
        let _ = writeln!(s, "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{relleno}\" stroke=\"{trazo}\" stroke-width=\"{grosor}\"><title>{} #{id}</title></rect>", rect.x, rect.y, rect.ancho, rect.alto, tipo.nombre());
    }
    s.push_str("</svg>\n");
    Some(s)
}
