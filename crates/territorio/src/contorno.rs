//! El contorno de la zona de una Facción (`ZonaFaccion`, Niebla_De_Guerra §5.4): la unión de los polígonos de sus plazas, que se solapan porque las de la misma Facción no se recortan
//! entre sí. Es presentación, como [`crate::poligono`]: no entra en ninguna regla ni en el hash, y usa `f64`.
//!
//! Cada polígono es convexo (un disco recortado por semiplanos), así que el borde de la unión son los trozos de sus lados que no caen dentro de ningún otro: se recorta cada lado contra
//! cada vecino (Cyrus-Beck), se quitan los trozos cubiertos y lo que queda se encadena en lazos cerrados. Los lazos conservan la orientación de los polígonos (antihoraria), así que un
//! hueco que la Facción rodea sin reclamar sale al revés (horario): pintados juntos con la regla `nonzero` el hueco queda recortado solo.

use base::Punto;
use std::collections::HashMap;

type P = (f64, f64);

/// Media milésima: lo que separa «sobre el borde» de «fuera», y lo que se tolera al encadenar los trozos.
const EPS: f64 = 0.5;

fn resta(a: P, b: P) -> P {
    (a.0 - b.0, a.1 - b.1)
}

fn cruz(a: P, b: P) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

fn area2(p: &[P]) -> f64 {
    (0..p.len()).map(|i| cruz(p[i], p[(i + 1) % p.len()])).sum()
}

/// Los lazos cerrados del borde de la unión (el último punto enlaza con el primero; no se repite). Los polígonos de menos de 3 puntos no cuentan.
pub fn contornos(poligonos: &[Vec<Punto>]) -> Vec<Vec<Punto>> {
    let pols: Vec<Vec<P>> = poligonos
        .iter()
        .filter(|p| p.len() >= 3)
        .map(|p| {
            let mut v: Vec<P> = p.iter().map(|q| (q.x as f64, q.y as f64)).collect();
            if area2(&v) < 0.0 {
                v.reverse();
            }
            v
        })
        .collect();
    let cajas: Vec<(P, P)> = pols.iter().map(|p| p.iter().fold(((f64::MAX, f64::MAX), (f64::MIN, f64::MIN)), |(lo, hi), q| ((lo.0.min(q.0), lo.1.min(q.1)), (hi.0.max(q.0), hi.1.max(q.1))))).collect();
    let solapan = |i: usize, j: usize| {
        let ((a, b), (c, d)) = (cajas[i], cajas[j]);
        a.0 <= d.0 + EPS && c.0 <= b.0 + EPS && a.1 <= d.1 + EPS && c.1 <= b.1 + EPS
    };

    // Los trozos de cada lado que no cubre ningún otro polígono.
    let mut trozos: Vec<(P, P)> = Vec::new();
    for (i, pol) in pols.iter().enumerate() {
        for k in 0..pol.len() {
            let (a, b) = (pol[k], pol[(k + 1) % pol.len()]);
            let mut cubierto: Vec<(f64, f64)> = (0..pols.len()).filter(|&j| j != i && solapan(i, j)).filter_map(|j| cubierto_por(a, b, &pols[j], j < i)).collect();
            cubierto.sort_by(|x, y| x.0.total_cmp(&y.0));
            let mut desde = 0.0;
            let largo = resta(b, a).0.hypot(resta(b, a).1);
            let mut quedan = |t0: f64, t1: f64| {
                if (t1 - t0) * largo > EPS {
                    trozos.push(((a.0 + (b.0 - a.0) * t0, a.1 + (b.1 - a.1) * t0), (a.0 + (b.0 - a.0) * t1, a.1 + (b.1 - a.1) * t1)));
                }
            };
            for (t0, t1) in cubierto {
                if t0 > desde {
                    quedan(desde, t0);
                }
                desde = desde.max(t1);
            }
            if desde < 1.0 {
                quedan(desde, 1.0);
            }
        }
    }
    encadenar(trozos)
}

/// El tramo `(t0, t1)` del lado `a → b` que cae dentro del polígono convexo `q` (antihorario), o `None`. Un lado que coincide con uno de `q` se cubre si van en sentidos opuestos (es un borde
/// interior de la unión) o, si van en el mismo sentido, solo en uno de los dos polígonos (`q_va_antes`), para que el borde se quede una vez.
fn cubierto_por(a: P, b: P, q: &[P], q_va_antes: bool) -> Option<(f64, f64)> {
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for k in 0..q.len() {
        let (c, d) = (q[k], q[(k + 1) % q.len()]);
        let lado = resta(d, c);
        let largo = lado.0.hypot(lado.1);
        // Distancia con signo a la recta del lado de `q` (positiva dentro), de los dos extremos.
        let (f0, f1) = (cruz(lado, resta(a, c)) / largo, cruz(lado, resta(b, c)) / largo);
        if f0.abs() <= EPS && f1.abs() <= EPS {
            let mismo_sentido = lado.0 * (b.0 - a.0) + lado.1 * (b.1 - a.1) > 0.0;
            if mismo_sentido && !q_va_antes {
                return None;
            }
            continue;
        }
        // Dentro (o sobre la recta, hasta `EPS`) o fuera; si el lado cruza la recta, el corte es el exacto.
        let corte = || (f0 / (f0 - f1)).clamp(0.0, 1.0);
        match (f0 > -EPS, f1 > -EPS) {
            (true, true) => {}
            (false, false) => return None,
            (false, true) => t0 = t0.max(corte()),
            (true, false) => t1 = t1.min(corte()),
        }
    }
    (t0 < t1).then_some((t0, t1))
}

/// Encadena los trozos (cada uno sigue al que acaba donde él empieza) en lazos cerrados.
fn encadenar(trozos: Vec<(P, P)>) -> Vec<Vec<Punto>> {
    let clave = |p: P| ((p.0 / (2.0 * EPS)).round() as i64, (p.1 / (2.0 * EPS)).round() as i64);
    let mut por_inicio: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, (a, _)) in trozos.iter().enumerate() {
        por_inicio.entry(clave(*a)).or_default().push(i);
    }
    let mut usado = vec![false; trozos.len()];
    let siguiente = |fin: P, usado: &mut Vec<bool>| -> Option<usize> {
        let (x, y) = clave(fin);
        let cerca = (-1..=1).flat_map(|dx| (-1..=1).map(move |dy| (x + dx, y + dy)));
        let i = cerca.filter_map(|k| por_inicio.get(&k)).flatten().copied().filter(|&i| !usado[i]).min_by(|&i, &j| {
            let d = |k: usize| resta(trozos[k].0, fin).0.hypot(resta(trozos[k].0, fin).1);
            d(i).total_cmp(&d(j))
        })?;
        usado[i] = true;
        Some(i)
    };
    let punto = |p: P| Punto::new(p.0.round() as i64, p.1.round() as i64);
    let mut lazos = Vec::new();
    for inicio in 0..trozos.len() {
        if usado[inicio] {
            continue;
        }
        usado[inicio] = true;
        let mut lazo = vec![punto(trozos[inicio].0)];
        let mut fin = trozos[inicio].1;
        while let Some(i) = siguiente(fin, &mut usado) {
            lazo.push(punto(trozos[i].0));
            fin = trozos[i].1;
        }
        lazo.dedup();
        if lazo.len() >= 3 {
            lazos.push(lazo);
        }
    }
    lazos
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<Punto> {
        vec![Punto::new(x0, y0), Punto::new(x1, y0), Punto::new(x1, y1), Punto::new(x0, y1)]
    }

    fn area(l: &[Punto]) -> f64 {
        area2(&l.iter().map(|p| (p.x as f64, p.y as f64)).collect::<Vec<_>>()) / 2.0
    }

    #[test]
    fn la_union_de_dos_que_se_solapan_es_un_solo_lazo_con_su_area() {
        let l = contornos(&[rect(0, 0, 10_000, 10_000), rect(5_000, 5_000, 15_000, 15_000)]);
        assert_eq!(l.len(), 1);
        assert_eq!(area(&l[0]), 175_000_000.0);
    }

    #[test]
    fn dos_que_no_se_tocan_son_dos_lazos_y_uno_dentro_de_otro_desaparece() {
        assert_eq!(contornos(&[rect(0, 0, 10_000, 10_000), rect(20_000, 0, 30_000, 10_000)]).len(), 2);
        let l = contornos(&[rect(0, 0, 10_000, 10_000), rect(2_000, 2_000, 4_000, 4_000)]);
        assert_eq!((l.len(), area(&l[0])), (1, 100_000_000.0));
    }

    #[test]
    fn un_marco_deja_el_hueco_como_un_lazo_al_reves_y_sus_bordes_comunes_una_vez() {
        // Cuatro rectángulos que comparten bordes rectos (mismo sentido en el exterior, sentido opuesto en los solapes).
        let marco = [rect(0, 0, 10_000, 2_000), rect(0, 8_000, 10_000, 10_000), rect(0, 0, 2_000, 10_000), rect(8_000, 0, 10_000, 10_000)];
        let l = contornos(&marco);
        let mut areas: Vec<f64> = l.iter().map(|x| area(x)).collect();
        areas.sort_by(f64::total_cmp);
        assert_eq!(areas, [-36_000_000.0, 100_000_000.0], "el exterior antihorario y el hueco horario");
    }

    #[test]
    fn dos_discos_solapados_dan_un_lazo_cuya_area_es_la_de_la_union() {
        let disco = |cx: f64| (0..48).map(|i| {
            let a = std::f64::consts::TAU * i as f64 / 48.0;
            Punto::new((cx + 100_000.0 * a.cos()).round() as i64, (100_000.0 * a.sin()).round() as i64)
        }).collect::<Vec<_>>();
        let (a, b) = (disco(0.0), disco(120_000.0));
        let l = contornos(&[a.clone(), b.clone()]);
        assert_eq!(l.len(), 1);
        // Área de la unión = A + B − intersección (la de dos polígonos convexos: el lazo de la unión la descuenta una vez).
        let suma = area(&a) + area(&b);
        assert!(area(&l[0]) < suma && area(&l[0]) > suma * 0.7, "{} frente a {suma}", area(&l[0]));
    }
}
