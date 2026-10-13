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
}
