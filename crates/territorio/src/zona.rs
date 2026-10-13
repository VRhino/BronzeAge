//! Zonas de influencia y fronteras duras (Doc 1.2). Una zona es un disco que crece con el tiempo; cuando toca la de otra
//! Facción se recorta con un semiplano que reparte la distancia según el radio de cada una, y se mueve al crecer los radios.
//! Las de la misma Facción se solapan y no se recortan.
//!
//! Todo lo que decide es aritmética entera exacta (`i128`, sin raíces); el polígono es solo presentación y usa `f64`.

use base::{Instante, Mil, Punto, Ritmo, MS_POR_MINUTO};

/// Una zona en un instante: centro, radio (milésimas) y Facción dueña.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disco {
    pub centro: Punto,
    pub radio: Mil,
    pub faccion: u32,
}

/// ¿Se tocan los discos? (distancia ≤ suma de radios, con igualdad: tocarse es llegar.)
pub fn se_tocan(a: &Disco, b: &Disco) -> bool {
    let r = (a.radio + b.radio) as i128;
    a.centro.dist2(b.centro) <= r * r
}

/// ¿`otro` recorta la zona de `yo`? Solo las de otra Facción que la tocan (y que no coinciden en el centro).
fn recorta(yo: &Disco, otro: &Disco) -> bool {
    yo.faccion != otro.faccion && yo.centro != otro.centro && se_tocan(yo, otro)
}

/// ¿`p` está del lado de `yo` respecto de la frontera con `otro`? La frontera pasa a `d · r_yo / (r_yo + r_otro)` de su
/// centro, perpendicular a la recta que une los centros: `(p − c)·(o − c) ≤ d² · r_yo / (r_yo + r_otro)`.
fn de_mi_lado(p: Punto, yo: &Disco, otro: &Disco) -> bool {
    let (vx, vy) = ((otro.centro.x - yo.centro.x) as i128, (otro.centro.y - yo.centro.y) as i128);
    let (px, py) = ((p.x - yo.centro.x) as i128, (p.y - yo.centro.y) as i128);
    let punto = px * vx + py * vy;
    punto * (yo.radio + otro.radio) as i128 <= (vx * vx + vy * vy) * yo.radio as i128
}

/// ¿`p` cae dentro de la zona real de `yo` (disco recortado por las fronteras con `vecinos`)? Los bordes cuentan.
pub fn en_zona<'a>(p: Punto, yo: &Disco, vecinos: impl IntoIterator<Item = &'a Disco>) -> bool {
    let r = yo.radio as i128;
    p.dist2(yo.centro) <= r * r && vecinos.into_iter().all(|o| !recorta(yo, o) || de_mi_lado(p, yo, o))
}

/// Polígono de la zona real de `yo`: `lados` vértices inscritos en el disco, recortados (Sutherland-Hodgman) por cada
/// frontera. Es presentación (`Asentamiento.zona`): no entra en ninguna decisión ni en el hash.
pub fn poligono<'a>(yo: &Disco, vecinos: impl IntoIterator<Item = &'a Disco>, lados: usize) -> Vec<Punto> {
    let (cx, cy, r) = (yo.centro.x as f64, yo.centro.y as f64, yo.radio as f64);
    let mut pol: Vec<(f64, f64)> = (0..lados.max(3))
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / lados.max(3) as f64;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect();
    for o in vecinos.into_iter().filter(|o| recorta(yo, o)) {
        let (vx, vy) = ((o.centro.x - yo.centro.x) as f64, (o.centro.y - yo.centro.y) as f64);
        let d = vx.hypot(vy);
        let (nx, ny) = (vx / d, vy / d);
        let k = d * yo.radio as f64 / (yo.radio + o.radio) as f64;
        // Dentro: n·(p − c) ≤ k.
        let dentro = |p: (f64, f64)| nx * (p.0 - cx) + ny * (p.1 - cy) - k;
        let mut nuevo = Vec::with_capacity(pol.len() + 1);
        for i in 0..pol.len() {
            let (a, b) = (pol[i], pol[(i + 1) % pol.len()]);
            let (da, db) = (dentro(a), dentro(b));
            if da <= 0.0 {
                nuevo.push(a);
            }
            if (da < 0.0 && db > 0.0) || (da > 0.0 && db < 0.0) {
                let t = da / (da - db);
                nuevo.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
            }
        }
        pol = nuevo;
        if pol.is_empty() {
            break;
        }
    }
    pol.into_iter().map(|(x, y)| Punto::new(x.round() as Mil, y.round() as Mil)).collect()
}

/// Primer instante `>= desde` en que la suma de los dos radios llega a `distancia` (las zonas se tocan). `Some(desde)` si
/// ya se tocan; `None` si no se tocarán nunca con los ritmos vigentes (se plantan en su techo antes de alcanzarse).
pub fn instante_de_contacto(a: &Ritmo, b: &Ritmo, distancia: Mil, desde: Instante) -> Option<Instante> {
    let suma = |t: Instante| a.valor_en(t) as i128 + b.valor_en(t) as i128;
    if suma(desde) >= distancia as i128 {
        return Some(desde);
    }
    // Cada radio es lineal hasta su tope: los cortes de tramo son esos dos instantes.
    let mut cortes: Vec<Instante> = [a, b].iter().filter_map(|r| r.maximo.and_then(|m| r.instante_de_valor(m, desde))).filter(|&t| t > desde).collect();
    cortes.sort_unstable();
    cortes.insert(0, desde);
    for (i, &inicio) in cortes.iter().enumerate() {
        let fin = cortes.get(i + 1).copied();
        // Pendiente del tramo: la de cada radio que aún no está en su tope al empezar.
        let pendiente: i128 = [a, b].iter().map(|r| if r.maximo.is_some_and(|m| r.valor_en(inicio) >= m) { 0 } else { r.por_minuto as i128 }).sum();
        if pendiente <= 0 {
            continue;
        }
        let falta = distancia as i128 - suma(inicio);
        let dt = (falta * MS_POR_MINUTO as i128 + pendiente - 1) / pendiente;
        let mut t = inicio + dt as Instante;
        // Los redondeos hacia abajo de cada ritmo por separado pueden dejar la suma 1 ms corta.
        while suma(t) < distancia as i128 && fin.is_none_or(|f| t < f) {
            t += 1;
        }
        if suma(t) >= distancia as i128 && fin.is_none_or(|f| t <= f) {
            return Some(t);
        }
    }
    None
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn d(x: Mil, y: Mil, r: Mil, f: u32) -> Disco {
        Disco { centro: Punto::new(x, y), radio: r, faccion: f }
    }

    #[test]
    fn disco_solo() {
        let a = d(0, 0, 10_000, 1);
        assert!(en_zona(Punto::new(10_000, 0), &a, []));
        assert!(!en_zona(Punto::new(10_001, 0), &a, []));
    }

    #[test]
    fn frontera_reparte_por_radios() {
        // A (r 6) y B (r 12) a 18: se tocan justo; la frontera pasa a 18·6/18 = 6 de A.
        let (a, b) = (d(0, 0, 6_000, 1), d(18_000, 0, 12_000, 2));
        assert!(se_tocan(&a, &b));
        assert!(en_zona(Punto::new(6_000, 0), &a, [&b]));
        assert!(!en_zona(Punto::new(6_001, 0), &a, [&b]));
        // Desde B la frontera queda a 12 de B, es decir, en x = 6.
        assert!(en_zona(Punto::new(6_000, 0), &b, [&a]));
        assert!(!en_zona(Punto::new(5_999, 0), &b, [&a]));
    }

    #[test]
    fn sin_contacto_no_se_recorta() {
        let (a, b) = (d(0, 0, 6_000, 1), d(30_000, 0, 6_000, 2));
        assert!(!se_tocan(&a, &b));
        assert!(en_zona(Punto::new(6_000, 0), &a, [&b]));
    }

    #[test]
    fn la_misma_faccion_se_solapa() {
        let (a, b) = (d(0, 0, 10_000, 1), d(15_000, 0, 10_000, 1));
        assert!(en_zona(Punto::new(10_000, 0), &a, [&b]));
        assert!(en_zona(Punto::new(5_000, 0), &b, [&a]));
    }

    #[test]
    fn la_frontera_se_mueve_al_crecer() {
        let a = d(0, 0, 6_000, 1);
        let b = d(20_000, 0, 6_000, 2);
        assert!(!se_tocan(&a, &b));
        let (a2, b2) = (d(0, 0, 10_000, 1), d(15_000, 0, 6_000, 2));
        // k = 15·10/16 = 9,375: la de radio mayor conserva más que la mitad.
        assert!(en_zona(Punto::new(9_375, 0), &a2, [&b2]));
        assert!(!en_zona(Punto::new(9_376, 0), &a2, [&b2]));
    }

    fn area(p: &[Punto]) -> f64 {
        let n = p.len();
        (0..n).map(|i| (p[i].x as f64 * p[(i + 1) % n].y as f64 - p[(i + 1) % n].x as f64 * p[i].y as f64) / 2.0).sum::<f64>().abs()
    }

    #[test]
    fn poligono_de_un_disco_y_recortado() {
        let a = d(0, 0, 10_000, 1);
        let ideal = std::f64::consts::PI * 1e8;
        let entero = area(&poligono(&a, [], 48));
        assert!((entero / ideal - 1.0).abs() < 0.01, "{entero} vs {ideal}");
        // Con un rival igual a 15: la frontera está a 7,5 y el polígono pierde bastante.
        let b = d(15_000, 0, 10_000, 2);
        let recortado = poligono(&a, [&b], 48);
        assert!(area(&recortado) < entero * 0.95);
        assert!(recortado.iter().all(|p| p.x <= 7_501), "ningún vértice pasa de la frontera");
        // Misma Facción: sin recorte.
        let c = d(15_000, 0, 10_000, 1);
        assert_eq!(poligono(&a, [&c], 48), poligono(&a, [], 48));
    }

    fn ritmo(v0: Mil, por_min: Mil, max: Mil) -> Ritmo {
        Ritmo { valor_en_t0: v0, t0: 0, por_minuto: por_min, maximo: Some(max), minimo: None }
    }

    #[test]
    fn contacto_con_dos_radios_que_crecen() {
        // 6+6 = 12 en t0; distancia 30; cada uno crece 200/min: faltan 18 → 18000/400 = 45 min.
        let (a, b) = (ritmo(6_000, 200, 30_000), ritmo(6_000, 200, 30_000));
        assert_eq!(instante_de_contacto(&a, &b, 30_000, 0), Some(45 * MS_POR_MINUTO));
        // Ya se tocan.
        assert_eq!(instante_de_contacto(&a, &b, 12_000, 5), Some(5));
    }

    #[test]
    fn contacto_que_nunca_llega_por_los_topes() {
        // Techos 10+10 = 20 < 30.
        let (a, b) = (ritmo(6_000, 200, 10_000), ritmo(6_000, 200, 10_000));
        assert_eq!(instante_de_contacto(&a, &b, 30_000, 0), None);
        // Con techos 10+10 y distancia 20 llegan justo al tope.
        assert_eq!(instante_de_contacto(&a, &b, 20_000, 0), Some(20 * MS_POR_MINUTO));
    }

    #[test]
    fn contacto_con_uno_ya_en_su_tope() {
        // A está plantada en 10 (tope); B crece de 6 a 20/min=200: 10+x ≥ 25 → x = 15 → 45 min.
        let a = ritmo(10_000, 200, 10_000);
        let b = ritmo(6_000, 200, 30_000);
        assert_eq!(instante_de_contacto(&a, &b, 25_000, 0), Some(45 * MS_POR_MINUTO));
    }

    #[test]
    fn contacto_cuando_uno_se_planta_a_mitad_de_camino() {
        // A llega a su tope (8) a los 10 min; luego solo crece B. Objetivo 20: a 10 min suman 8+8=16; faltan 4 a 200/min → 20 min más.
        let a = ritmo(6_000, 200, 8_000);
        let b = ritmo(6_000, 200, 30_000);
        assert_eq!(instante_de_contacto(&a, &b, 20_000, 0), Some(30 * MS_POR_MINUTO));
    }
}
