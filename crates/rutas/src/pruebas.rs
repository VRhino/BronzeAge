//! Pruebas con retículas sintéticas (tipos de `contrato::v1`) y, ignorada, una medición con el artefacto real.

use super::*;
use contrato::v1::{AristaGrafo, NodoGrafo, Punto as PuntoPb};

const ESP: i64 = 8_000;

/// Centro de la celda `(x, y)` de la retícula.
fn c(x: i64, y: i64) -> Punto {
    Punto::new(x * ESP + ESP / 2, y * ESP + ESP / 2)
}

/// Retícula `w × h` con 8 vecinos (la diagonal solo si las dos ortogonales están libres). `libre(x, y)` decide qué
/// celdas tienen nodo; `vado(a, b)` marca aristas (por coordenadas de celda). Ids: `y * w + x`.
fn reticula(w: i64, h: i64, libre: impl Fn(i64, i64) -> bool, vado: impl Fn((i64, i64), (i64, i64)) -> bool) -> GrafoNavegacion {
    let mut g = GrafoNavegacion { espaciado_min: ESP as u32, ..Default::default() };
    for y in 0..h {
        for x in 0..w {
            if libre(x, y) {
                let p = c(x, y);
                g.nodos.push(NodoGrafo { id: (y * w + x) as u32, posicion: Some(PuntoPb { x: p.x, y: p.y }) });
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            for (dx, dy) in [(1, 0), (0, 1), (1, 1), (-1, 1)] {
                let (x2, y2) = (x + dx, y + dy);
                if !(0..w).contains(&x2) || !(0..h).contains(&y2) || !libre(x, y) || !libre(x2, y2) {
                    continue;
                }
                if dx != 0 && dy != 0 && !(libre(x2, y) && libre(x, y2)) {
                    continue;
                }
                g.aristas.push(AristaGrafo {
                    a: (y * w + x) as u32,
                    b: (y2 * w + x2) as u32,
                    coste_milesimas: if dx != 0 && dy != 0 { 1414 } else { 1000 },
                    vado: vado((x, y), (x2, y2)),
                });
            }
        }
    }
    g
}

fn abierta(w: i64, h: i64) -> Rutas {
    Rutas::desde_grafo(&reticula(w, h, |_, _| true, |_, _| false))
}

#[test]
fn recta_en_llano_y_tiempos_del_plan() {
    let r = abierta(12, 12);
    let cam = r.ruta(c(1, 1), c(9, 1), &Opciones::default()).unwrap();
    assert_eq!(cam.puntos.first(), Some(&c(1, 1)));
    assert_eq!(cam.puntos.last(), Some(&c(9, 1)));
    assert_eq!(cam.coste_total(), 8_000, "8 espaciados en llano");
    assert_eq!(cam.puntos.len(), 9);
    // 4.000 milésimas/s ⇒ un espaciado (8.000) tarda 2.000 ms.
    let plan = cam.a_plan(100_000, 4_000, 7);
    assert_eq!(plan.generacion, 7);
    assert_eq!(plan.inicio(), Some(100_000));
    assert_eq!(plan.fin(), Some(100_000 + 16_000));
    assert_eq!(plan.vertices[1].en, 102_000);
    assert!(plan.vertices.windows(2).all(|w| w[0].en < w[1].en));
}

#[test]
fn diagonal_cuesta_raiz_de_dos_y_no_se_zigzaguea() {
    let r = abierta(12, 12);
    let cam = r.ruta(c(1, 1), c(7, 7), &Opciones::default()).unwrap();
    assert_eq!(cam.coste_total(), 6 * 1414);
    assert_eq!(cam.puntos.len(), 7);
    // Tiempo = longitud geométrica / velocidad: 6·√2·8.000 ≈ 67.882 ⇒ a 8.000/s, ≈ 8.485 ms.
    let t = cam.a_plan(0, 8_000, 0).fin().unwrap();
    assert_eq!(t, (6 * 1414 * ESP + 4_000) / 8_000);
}

#[test]
fn extremos_reales_fuera_de_los_nodos() {
    let r = abierta(12, 12);
    let (o, d) = (Punto::new(c(1, 1).x + 1_500, c(1, 1).y - 900), Punto::new(c(9, 1).x - 700, c(9, 1).y + 2_000));
    let cam = r.ruta(o, d, &Opciones::default()).unwrap();
    assert_eq!((cam.puntos[0], *cam.puntos.last().unwrap()), (o, d));
    assert_eq!(cam.costes.len(), cam.puntos.len() - 1);
    let plan = cam.a_plan(0, 1_000, 1);
    assert_eq!(plan.vertices[0].p, o);
    assert!(plan.vertices.windows(2).all(|w| w[0].en <= w[1].en));
}

#[test]
fn origen_y_destino_cercanos_van_en_recta() {
    let r = abierta(12, 12);
    let cam = r.ruta(c(3, 3), Punto::new(c(3, 3).x + 5_000, c(3, 3).y), &Opciones::default()).unwrap();
    assert_eq!(cam.puntos.len(), 2);
    assert!(cam.nodos.is_empty());
    let mismo = r.ruta(c(3, 3), c(3, 3), &Opciones::default()).unwrap();
    assert_eq!(mismo.a_plan(5, 1_000, 0).vertices.len(), 1);
}

/// Muro vertical en x = 5; con hueco en y = 8 o sin hueco.
fn muro(hueco: Option<i64>) -> Rutas {
    Rutas::desde_grafo(&reticula(12, 12, |x, y| x != 5 || Some(y) == hueco, |_, _| false))
}

#[test]
fn muro_con_hueco_se_rodea_por_el_hueco() {
    let r = muro(Some(8));
    let cam = r.ruta(c(1, 1), c(9, 1), &Opciones::default()).unwrap();
    assert!(cam.puntos.contains(&c(5, 8)), "pasa por el hueco");
    assert!(cam.coste_total() > 8_000);
    // Sin la ruta alterna, el coste es el óptimo: baja en diagonal 7, cruza 4 en llano… se comprueba contra un Dijkstra.
    assert_eq!(cam.coste_total(), dijkstra(&r, c(1, 1), c(9, 1)));
}

#[test]
fn muro_sin_hueco_no_hay_camino() {
    let r = muro(None);
    assert!(r.ruta(c(1, 1), c(9, 1), &Opciones::default()).is_none());
    assert!(r.ruta(c(1, 1), c(3, 9), &Opciones::default()).is_some(), "mismo lado: sí");
}

/// Dijkstra ingenuo entre nodos (para comparar el coste óptimo; los extremos deben ser nodos exactos).
fn dijkstra(r: &Rutas, a: Punto, b: Punto) -> i64 {
    let n = r.nodos();
    let idx = |p: Punto| r.pos.iter().position(|&q| q == p).unwrap();
    let mut d = vec![i64::MAX; n];
    let mut cola = BinaryHeap::new();
    d[idx(a)] = 0;
    cola.push(Reverse((0i64, idx(a))));
    while let Some(Reverse((du, u))) = cola.pop() {
        if du > d[u] {
            continue;
        }
        for k in r.inicio[u] as usize..r.inicio[u + 1] as usize {
            let (v, nd) = (r.destino[k] as usize, du + r.coste[k] as i64);
            if nd < d[v] {
                d[v] = nd;
                cola.push(Reverse((nd, v)));
            }
        }
    }
    d[idx(b)]
}

#[test]
fn astar_es_optimo_frente_a_dijkstra_en_un_laberinto() {
    // Paredes alternas en columnas pares con huecos desplazados.
    let r = Rutas::desde_grafo(&reticula(20, 12, |x, y| x % 4 != 2 || y == (x / 4 * 5) % 12, |_, _| false));
    for (a, b) in [((0, 0), (19, 11)), ((1, 5), (17, 2)), ((0, 11), (19, 0))] {
        let (pa, pb) = (c(a.0, a.1), c(b.0, b.1));
        let esperado = dijkstra(&r, pa, pb);
        let cam = r.ruta(pa, pb, &Opciones::default()).expect("alcanzable");
        assert_eq!(cam.coste_total(), esperado, "{a:?} → {b:?}");
    }
}

#[test]
fn vado_con_y_sin_penalizacion() {
    // Río en x = 5 (sin nodos) salvo dos pasos: y = 1 (vado, corto) y y = 10 (puente sin marca, largo).
    let r = Rutas::desde_grafo(&reticula(12, 12, |x, y| x != 5 || y == 1 || y == 10, |a, b| (a.0 == 5 || b.0 == 5) && (a.1 == 1 || b.1 == 1) && a.1 <= 1 && b.1 <= 1));
    let sin = r.ruta(c(2, 1), c(8, 1), &Opciones::default()).unwrap();
    assert!(sin.puntos.contains(&c(5, 1)), "sin penalización usa el vado");
    let con = r.ruta(c(2, 1), c(8, 1), &Opciones { penalizacion_vado: 50_000, ..Default::default() }).unwrap();
    assert!(con.puntos.contains(&c(5, 10)) && !con.puntos.contains(&c(5, 1)), "con penalización, el otro paso");
    // La penalización desvía pero no encarece el plan: el coste real es el de la ruta tomada.
    assert_eq!(con.coste_total(), con.costes.iter().sum::<i64>());
    assert!(con.coste_total() < 50_000 + sin.coste_total());
    // Si solo hay vado y se penaliza, se usa igualmente.
    let solo = Rutas::desde_grafo(&reticula(12, 12, |x, y| x != 5 || y == 1, |a, b| a.0 == 5 || b.0 == 5));
    let pasa = solo.ruta(c(2, 1), c(8, 1), &Opciones { penalizacion_vado: 1_000_000, ..Default::default() }).unwrap();
    assert_eq!(pasa.coste_total(), sin.coste_total());
}

#[test]
fn nodos_bloqueados_desvian_o_cortan() {
    let r = muro(Some(8));
    let ids = |pts: &[Punto]| pts.iter().map(|p| (p.y / ESP) as u32 * 12 + (p.x / ESP) as u32).collect::<Vec<_>>();
    let hueco = ids(&[c(5, 8)])[0];
    let sin = r.ruta(c(1, 1), c(9, 1), &Opciones { bloqueados: vec![hueco], ..Default::default() });
    assert!(sin.is_none(), "bloqueado el único hueco, no hay paso");
    let libre = abierta(12, 12);
    let medio = 12 + 5;
    let cam = libre.ruta(c(1, 1), c(9, 1), &Opciones { bloqueados: vec![medio], ..Default::default() }).unwrap();
    assert!(!cam.nodos.contains(&medio));
    assert!(cam.coste_total() > 8_000);
}

#[test]
fn extremo_aislado_no_tiene_camino() {
    // Isla de un nodo lejos de todo: ningún nodo a menos de 32 espaciados.
    let r = Rutas::desde_grafo(&reticula(100, 4, |x, y| x < 5 || (x == 99 && y == 0), |_, _| false));
    assert!(r.ruta(c(1, 1), c(99, 0), &Opciones::default()).is_none());
}

#[test]
fn determinista_y_el_lote_en_paralelo_coincide() {
    let g = reticula(40, 40, |x, y| (x * 7 + y * 13) % 11 != 0 || x == 0, |_, _| false);
    let (r1, r2) = (Rutas::desde_grafo(&g), Rutas::desde_grafo(&g));
    let mut rng = base::Rng::nuevo(1);
    let peticiones: Vec<Peticion> = (0..200)
        .map(|_| Peticion {
            origen: c(rng.por_debajo(40) as i64, rng.por_debajo(40) as i64),
            destino: Punto::new(rng.por_debajo(320_000) as i64, rng.por_debajo(320_000) as i64),
            opciones: Opciones::default(),
        })
        .collect();
    let a = r1.rutas_en_paralelo(&peticiones);
    let b: Vec<_> = peticiones.iter().map(|p| r2.ruta(p.origen, p.destino, &p.opciones)).collect();
    assert_eq!(a, b);
    assert!(a.iter().filter(|x| x.is_some()).count() > 100);
    // Reutilizar el escritorio no cambia nada.
    let mut e = Escritorio::nuevo(r1.nodos());
    for (p, esperado) in peticiones.iter().zip(&a) {
        assert_eq!(&r1.ruta_con(&mut e, p.origen, p.destino, &p.opciones), esperado);
    }
}

#[test]
fn plan_redondea_el_acumulado_y_es_monotono() {
    let cam = Camino {
        puntos: (0..6).map(|i| Punto::new(i * 100, 0)).collect(),
        costes: vec![333; 5], // 333 milésimas de espaciado cada tramo
        nodos: Vec::new(),
        espaciado: 1_000,
    };
    // velocidad 7 mil/s: cada tramo ≈ 47,57 ms; el redondeo es sobre el acumulado (sin deriva).
    let plan = cam.a_plan(10, 7, 3);
    let esperado: Vec<i64> = (0..6).map(|i| 10 + (i * 333 * 1_000 + 3) / 7).collect();
    assert_eq!(plan.vertices.iter().map(|v| v.en).collect::<Vec<_>>(), esperado);
    assert!(plan.vertices.windows(2).all(|w| w[0].en <= w[1].en));
    assert_eq!(cam.a_plan(0, 0, 0).vertices.len(), 6, "velocidad 0 no revienta");
}

/// Con el artefacto real (`generador/salida/mundo/mundo-*.bin`): construir, validar enganches y medir 1.000 rutas entre
/// campamentos. `cargo test -p rutas --release -- --ignored --nocapture`.
#[test]
#[ignore]
fn mundo_real_mide_rutas_entre_campamentos() {
    use std::time::Instant as T;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generador/salida/mundo");
    let Some(f) = std::fs::read_dir(&dir).ok().and_then(|d| d.flatten().find(|e| e.file_name().to_string_lossy().starts_with("mundo-"))) else {
        eprintln!("sin artefacto en {dir:?}: se omite");
        return;
    };
    let bytes = std::fs::read(f.path()).unwrap();
    let t = T::now();
    let m = Mundo::cargar(&bytes, &mundo::OpcionesValidacion::default()).expect("artefacto válido");
    eprintln!("cargar y validar: {:?}", t.elapsed());
    let t = T::now();
    let r = Rutas::desde_mundo(&m);
    eprintln!("construir Rutas ({} nodos, espaciado {}): {:?}", r.nodos(), r.espaciado(), t.elapsed());

    let pts: Vec<Punto> = m.campamentos().iter().map(|c| c.posicion.map(|p| Punto::new(p.x, p.y)).unwrap()).collect();
    assert!(pts.len() > 2);
    let mut rng = base::Rng::nuevo(99);
    let peticiones: Vec<Peticion> = (0..1000)
        .map(|_| {
            let (a, b) = (rng.por_debajo(pts.len() as u64) as usize, rng.por_debajo(pts.len() as u64) as usize);
            Peticion { origen: pts[a], destino: pts[b], opciones: Opciones::default() }
        })
        .collect();

    let mut e = Escritorio::nuevo(r.nodos());
    let t = T::now();
    let sec: Vec<_> = peticiones.iter().map(|p| r.ruta_con(&mut e, p.origen, p.destino, &p.opciones)).collect();
    let dur = t.elapsed();
    let ok = sec.iter().flatten().count();
    let largo: usize = sec.iter().flatten().map(|c| c.puntos.len()).sum::<usize>() / ok.max(1);
    eprintln!("secuencial: {ok}/1000 con ruta, {:.3} ms/ruta, {largo} vértices de media", dur.as_secs_f64() * 1000.0 / 1000.0);

    let t = T::now();
    let par = r.rutas_en_paralelo(&peticiones);
    eprintln!("paralelo: {:.3} ms/ruta (total {:?})", t.elapsed().as_secs_f64(), t.elapsed());
    assert_eq!(par, sec, "paralelo = secuencial");

    // Un plan de muestra: ligera (30 u/min = 500 mil/s).
    if let Some(c) = sec.iter().flatten().max_by_key(|c| c.coste_total()) {
        let plan = c.a_plan(0, 500, 1);
        eprintln!("ruta más larga: {} vértices, coste {} → {:.1} min a 500 mil/s", c.puntos.len(), c.coste_total(), plan.fin().unwrap() as f64 / 60_000.0);
    }

    // Suavizado (columnas), con las mismas peticiones; solo si el artefacto trae la capa `coste`.
    if !m.tiene_coste() {
        eprintln!("el artefacto no trae la capa coste: sin suavizado");
        return;
    }
    let suaves: Vec<Peticion> = peticiones.iter().map(|p| Peticion { opciones: Opciones::de_columna(), ..p.clone() }).collect();
    let t = T::now();
    let sua: Vec<_> = suaves.iter().map(|p| r.ruta_con(&mut e, p.origen, p.destino, &p.opciones)).collect();
    let dur = t.elapsed();
    let (mut v0, mut v1, mut c0, mut c1) = (0usize, 0usize, 0i64, 0i64);
    for (a, b) in sec.iter().zip(&sua) {
        if let (Some(a), Some(b)) = (a, b) {
            (v0, v1, c0, c1) = (v0 + a.puntos.len(), v1 + b.puntos.len(), c0 + a.coste_total(), c1 + b.coste_total());
        }
    }
    eprintln!("suavizado: {:.3} ms/ruta, vértices {v0} → {v1}, coste {c0} → {c1} ({:+.2} %)", dur.as_secs_f64(), 100.0 * (c1 - c0) as f64 / c0.max(1) as f64);
    assert_eq!(sua.iter().flatten().count(), ok, "suavizar no quita rutas");
    // Las rectas nuevas están libres; lo que no lo esté tiene que ser una arista del A* que se quedó (los vados, por ejemplo).
    let tr = r.transitable.as_ref().unwrap();
    let mut no_libres_del_astar = 0;
    for (a, b) in sec.iter().zip(&sua) {
        if let (Some(a), Some(b)) = (a, b) {
            no_libres_del_astar += a.puntos.windows(2).filter(|w| !tr.libre(w[0], w[1])).count();
            for w in b.puntos.windows(2) {
                assert!(tr.libre(w[0], w[1]) || a.puntos.windows(2).any(|v| v == w), "recta nueva que pisa lo que no se puede: {w:?}");
            }
        }
    }
    eprintln!("tramos del A* que no pasan `libre` (vados y diagonales que rozan): {no_libres_del_astar}");
}

// ───────────────────────────── Red de caminos: atracción y plan por minuto (caravanas) ─────────────────────────────

/// Una atracción con las aristas que unen los nodos `ruta` (por celdas), todas con `peso`.
fn atraccion_por(ruta: &[(i64, i64)], peso: u32) -> Arc<Atraccion> {
    let pesos = ruta.windows(2).map(|w| (clave_arista(c(w[0].0, w[0].1), c(w[1].0, w[1].1)), peso)).collect();
    Arc::new(Atraccion { pesos, por_peso: 100_000, minimo: 500_000 })
}

#[test]
fn la_atraccion_de_la_red_desvia_la_ruta_hacia_las_aristas_con_peso_y_sin_ella_nada_cambia() {
    let r = abierta(12, 12);
    let (a, b) = (c(0, 5), c(10, 5));
    let recta = r.ruta(a, b, &Opciones::default()).unwrap();
    assert_eq!(recta.coste_total(), 10_000);
    // Un camino con rodeo por la fila 2 (la ida: 3 filas arriba, 10 a lo ancho, 3 abajo): con mucho peso compensa.
    let mut rodeo: Vec<(i64, i64)> = vec![(0, 5), (0, 4), (0, 3), (0, 2)];
    rodeo.extend((1..=10).map(|x| (x, 2)));
    rodeo.extend([(10, 3), (10, 4), (10, 5)]);
    let con = Opciones { atraccion: Some(atraccion_por(&rodeo, 5)), ..Opciones::default() };
    let desviada = r.ruta(a, b, &con).unwrap();
    assert!(desviada.puntos.iter().any(|p| p.y == c(0, 2).y), "se arrima al camino: {:?}", desviada.puntos);
    // El coste que devuelve es el real del terreno (sin atracción): el plan no se abarata por el peso.
    assert!(desviada.coste_total() > recta.coste_total());
    // Sin pesos, o con un mapa de pesos vacío, el resultado es idéntico al de siempre.
    let vacia = Opciones { atraccion: Some(Arc::new(Atraccion { pesos: Default::default(), por_peso: 100_000, minimo: 500_000 })), ..Opciones::default() };
    assert_eq!(r.ruta(a, b, &vacia).unwrap(), recta);
    // El factor: 1 − 0,1 × peso, con suelo en 0,5; peso 0 no atrae.
    let at = atraccion_por(&[], 0);
    assert_eq!((at.factor(0), at.factor(1), at.factor(3), at.factor(9)), (1_000_000, 900_000, 700_000, 500_000));
    // Determinista.
    assert_eq!(r.ruta(a, b, &con).unwrap(), r.ruta(a, b, &con).unwrap());
}

#[test]
fn la_clave_de_arista_redondea_a_entero_y_no_depende_del_sentido() {
    assert_eq!(clave_arista(Punto::new(10_400, 20_600), Punto::new(30_000, 40_499)), ((10, 21), (30, 40)));
    assert_eq!(clave_arista(Punto::new(30_000, 40_499), Punto::new(10_400, 20_600)), ((10, 21), (30, 40)));
}

#[test]
fn el_plan_por_minuto_reparte_el_tiempo_por_el_coste_de_cada_tramo() {
    let puntos = [c(0, 0), c(1, 0), c(2, 0)];
    // Un espaciado (8 unidades) en llano a 24 unidades por minuto tarda 20 s; el segundo tramo, a mitad de coste (camino), 10 s.
    let plan = a_plan_por_minuto(&puntos, &[1_000, 500], ESP, 5_000, 24_000, 3);
    assert_eq!(plan.generacion, 3);
    assert_eq!(plan.vertices.iter().map(|v| v.en).collect::<Vec<_>>(), vec![5_000, 25_000, 35_000]);
    // Es lo mismo que el plan por segundo a la velocidad equivalente (24 u/min = 400 milésimas/s).
    let cam = Camino { puntos: puntos.to_vec(), costes: vec![1_000, 1_000], nodos: vec![], espaciado: ESP };
    assert_eq!(a_plan_por_minuto(&puntos, &cam.costes, ESP, 5_000, 24_000, 3), cam.a_plan(5_000, 400, 3));
}

// ── Suavizado por coste ──

/// Retícula `w × h` sobre un terreno con coste por celda de la retícula (0 = no se pisa: sin nodo). Las aristas cuestan la
/// media de sus dos extremos (× √2 en diagonal) y la capa de coste del mundo es la misma, a celdas de 4 (dos por lado).
fn con_terreno(w: i64, h: i64, coste: impl Fn(i64, i64) -> u16 + Copy, vado: impl Fn((i64, i64), (i64, i64)) -> bool) -> Rutas {
    let mut g = reticula(w, h, |x, y| coste(x, y) > 0, vado);
    for a in &mut g.aristas {
        let (pa, pb) = ((a.a as i64 % w, a.a as i64 / w), (a.b as i64 % w, a.b as i64 / w));
        let media = (coste(pa.0, pa.1) as u32 + coste(pb.0, pb.1) as u32) / 2;
        a.coste_milesimas = a.coste_milesimas * media / 1000;
    }
    Rutas::con_terreno(&g, 2 * w, 2 * h, |cx, cy| coste(cx / 2, cy / 2))
}

fn suave() -> Opciones {
    Opciones { suavizar: true, ..Default::default() }
}

#[test]
fn suavizar_en_llano_deja_la_recta_y_cuesta_menos() {
    let r = con_terreno(12, 12, |_, _| 1000, |_, _| false);
    let (o, d) = (c(1, 1), c(9, 4));
    let a = r.ruta(o, d, &Opciones::default()).unwrap();
    let s = r.ruta(o, d, &suave()).unwrap();
    assert!(a.puntos.len() > 2, "el A* va por la retícula");
    assert_eq!(s.puntos, vec![o, d]);
    assert_eq!(s.coste_total(), (o.dist(d) * 1000 + ESP / 2) / ESP, "la recta en llano: su largo en espaciados");
    assert!(s.coste_total() < a.coste_total());
    assert!(s.nodos.is_empty());
    assert!(s.a_plan(0, 1_000, 0).vertices.windows(2).all(|w| w[0].en < w[1].en));
}

#[test]
fn sin_capa_de_coste_o_sin_pedirlo_no_se_suaviza() {
    let (o, d) = (c(1, 1), c(9, 4));
    let solo_grafo = abierta(12, 12);
    assert_eq!(solo_grafo.ruta(o, d, &suave()), solo_grafo.ruta(o, d, &Opciones::default()));
    let r = con_terreno(12, 12, |_, _| 1000, |_, _| false);
    assert!(r.ruta(o, d, &Opciones::default()).unwrap().puntos.len() > 2, "con capa pero sin pedirlo, la ruta del A*");
}

#[test]
fn el_suavizado_no_atraviesa_lo_que_no_se_pisa() {
    // Muro en x = 5 con hueco en y = 8.
    let r = con_terreno(12, 12, |x, y| if x == 5 && y != 8 { 0 } else { 1000 }, |_, _| false);
    let a = r.ruta(c(1, 1), c(9, 1), &Opciones::default()).unwrap();
    let s = r.ruta(c(1, 1), c(9, 1), &suave()).unwrap();
    let t = r.transitable.as_ref().unwrap();
    assert!(s.puntos.windows(2).all(|w| t.libre(w[0], w[1])));
    assert!(s.puntos.len() < a.puntos.len());
    assert!(s.puntos.iter().any(|p| p.y / ESP >= 7), "baja hasta el hueco");
    assert!(s.coste_total() < a.coste_total());
}

#[test]
fn el_suavizado_no_corta_por_el_terreno_caro() {
    // Montaña que se pisa pero cuesta como una cima (×12): x 5-8, y 0-6. El A* la rodea por abajo y la recta no la pisa.
    let caro = |x: i64, y: i64| (5..=8).contains(&x) && y <= 6;
    let r = con_terreno(14, 12, |x, y| if caro(x, y) { 12_000 } else { 1000 }, |_, _| false);
    let (o, d) = (c(1, 2), c(12, 2));
    let a = r.ruta(o, d, &Opciones::default()).unwrap();
    let s = r.ruta(o, d, &suave()).unwrap();
    assert!(a.puntos.iter().all(|p| !caro(p.x / ESP, p.y / ESP)), "el A* rodea");
    assert!(s.puntos.len() < a.puntos.len(), "y el suavizado estira");
    // Las rectas se muestrean cada media celda (2 u), como `libre`: pueden rozar una esquina por menos de eso, no entrar.
    let dentro = |q: Punto| (5 * ESP + 2_000..9 * ESP - 2_000).contains(&q.x) && q.y < 7 * ESP - 2_000;
    for w in s.puntos.windows(2) {
        let n = w[0].dist(w[1]) / 500 + 1;
        for k in 0..=n {
            let q = Punto::new(w[0].x + (w[1].x - w[0].x) * k / n, w[0].y + (w[1].y - w[0].y) * k / n);
            assert!(!dentro(q), "la recta entra en la montaña en {q:?}");
        }
    }
    assert!(s.coste_total() <= a.coste_total());
}

#[test]
fn el_suavizado_conserva_los_vados() {
    // Río en y = 5 (no se pisa) salvo el vado de x = 6; las aristas que tocan (6, 5) son de vado.
    let r = con_terreno(12, 12, |x, y| if y == 5 && x != 6 { 0 } else { 1000 }, |a, b| a == (6, 5) || b == (6, 5));
    let (o, d) = (c(1, 1), c(11, 10));
    let s = r.ruta(o, d, &suave()).unwrap();
    let i = s.puntos.iter().position(|&p| p == c(6, 5)).expect("cruza por el vado");
    assert_eq!((s.puntos[i - 1], s.puntos[i + 1]), (c(6, 4), c(6, 6)), "los dos tramos del vado se quedan");
    assert!(s.puntos.len() < r.ruta(o, d, &Opciones::default()).unwrap().puntos.len());
}
