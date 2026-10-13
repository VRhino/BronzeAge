use super::*;
use base::{Rng, Vertice};

const U: Mil = 1000; // una unidad estratégica

fn plan(vs: &[(Mil, Mil, Instante)]) -> Plan {
    Plan::nuevo(1, vs.iter().map(|&(x, y, en)| Vertice { p: Punto::new(x, y), en }).collect())
}

fn pt(x: Mil, y: Mil) -> Punto {
    Punto::new(x, y)
}

#[test]
fn entra_y_sale_de_la_vista() {
    // Plaza de la Facción 1 en (100,100) con radio 30; una columna de la 2 cruza de x=0 a x=200 en 200 s (1 u/s).
    let columna = plan(&[(0, 100 * U, 0), (200 * U, 100 * U, 200_000)]);
    let obs = [Observador::fijo(1, pt(100 * U, 100 * U), 30 * U)];
    let mov = [Movil::new(7, 2, Donde::Plan(&columna))];
    let v = Vision::new();
    let m0 = v.muestrear(0, &obs, &mov);
    let m1 = v.muestrear(100_000, &obs, &mov); // x=100: dentro
    let m2 = v.muestrear(200_000, &obs, &mov); // x=200: fuera
    assert_eq!(m0.de(1), &[] as &[u64]);
    assert_eq!(m1.de(1), &[7]);
    assert_eq!(m2.de(1), &[] as &[u64]);
    assert_eq!(diff(&m0, &m1).por_faccion, vec![(1, CambioVista { entran: vec![7], salen: vec![] })]);
    assert_eq!(diff(&m1, &m2).por_faccion, vec![(1, CambioVista { entran: vec![], salen: vec![7] })]);
    assert!(diff(&m1, &m1).por_faccion.is_empty());
    // La Facción 2 siempre ve lo suyo.
    assert_eq!(m0.de(2), &[7]);
}

#[test]
fn el_radio_no_es_estricto_y_la_plaza_ve_solo_su_zona() {
    let obs = [Observador::fijo(1, pt(0, 0), 30 * U)];
    let fijo = |x| Donde::Fijo(pt(x, 0));
    let mov = [Movil::new(1, 2, fijo(30 * U)), Movil::new(2, 2, fijo(30 * U + 1)), Movil::new(3, 2, fijo(60 * U))];
    // Una plaza de radio 30 no ve a 30,001 ni a 60 (margen 0: solo su zona); a 30 exactos sí.
    assert_eq!(Vision::new().muestrear(0, &obs, &mov).de(1), &[1]);
}

#[test]
fn aliados_comparten_lo_visible_pero_no_lo_explorado() {
    let columna_c = plan(&[(500 * U, 500 * U, 0), (500 * U, 500 * U, 1)]);
    let obs = [
        Observador::fijo(2, pt(500 * U, 500 * U), radios::EJERCITO), // B ve a C
        Observador::fijo(1, pt(0, 0), radios::EJERCITO),             // A, lejos
    ];
    let mov = [Movil::new(9, 3, Donde::Plan(&columna_c))];
    let sin = Vision::new().muestrear(0, &obs, &mov);
    assert_eq!((sin.de(1), sin.de(2), sin.de(4)), (&[] as &[u64], &[9u64][..], &[] as &[u64]));
    let con = Vision::con_alianzas([(2, 1)]); // cualquier dirección
    let m = con.muestrear(0, &obs, &mov);
    assert_eq!((m.de(1), m.de(2)), (&[9u64][..], &[9u64][..]));
    assert_eq!(con.aliados(1), vec![2]);

    // Niebla: A solo explora lo suyo; lo de B entra en `visibles`, no en `celdas`.
    let mut n = Niebla::nueva(667, 500);
    n.grabar(0, &obs);
    let p = con.niebla_proyectada(&n, 1, 0, &obs);
    let (celdas, visibles) = (&p.celdas, &p.visibles);
    let bit = |m: &Vec<u8>, c: usize, f: usize| m[(f * 667 + c) >> 3] >> ((f * 667 + c) & 7) & 1 == 1;
    assert!(bit(celdas, 0, 0) && bit(visibles, 0, 0));
    assert!(!bit(celdas, 125, 125), "lo que ve el aliado no se explora");
    assert!(bit(visibles, 125, 125), "pero sí se ve ahora");
    assert!(!n.explorada(1, 125, 125) && n.explorada(2, 125, 125));
    assert_eq!((p.tamano_celda, p.columnas, p.filas, p.celdas.len()), (4000, 667, 500, (667 * 500usize).div_ceil(8)));
}

#[test]
fn mirada_ve_pero_no_explora() {
    let obs = [Observador::mirada(1, pt(100 * U, 100 * U), radios::MIRADA)];
    let mut n = Niebla::nueva(667, 500);
    n.grabar(0, &obs);
    assert!(!n.explorado_en(1, pt(100 * U, 100 * U)));
    let mov = [Movil::new(5, 2, Donde::Fijo(pt(110 * U, 100 * U)))];
    assert_eq!(Vision::new().muestrear(0, &obs, &mov).de(1), &[5]);
    let p = Vision::new().niebla_proyectada(&n, 1, 0, &obs);
    assert!(p.celdas.iter().all(|&b| b == 0) && p.visibles.iter().any(|&b| b != 0));
}

#[test]
fn caravana_grande_llama_la_atencion_desde_mas_lejos() {
    let obs = [Observador::fijo(1, pt(0, 0), radios::EJERCITO)];
    let mut grande = Movil::new(1, 2, Donde::Fijo(pt(50 * U, 0)));
    let normal = Movil::new(2, 2, Donde::Fijo(pt(50 * U, 0)));
    grande.atencion = Some(radios::CARAVANA_GRANDE);
    assert_eq!(Vision::new().muestrear(0, &obs, &[grande, normal]).de(1), &[1]);
    grande.donde = Donde::Fijo(pt(61 * U, 0));
    assert_eq!(Vision::new().muestrear(0, &obs, &[grande]).de(1), &[] as &[u64]);
}

fn mundo_aleatorio(semilla: u64, n_mov: usize, n_fac: u32) -> (Vec<Plan>, Vec<Plan>, Vec<(u32, bool)>) {
    // Devuelve planes de móviles, planes de observadores y (facción, atención) de cada móvil.
    let mut rng = Rng::nuevo(semilla);
    let plan_al_azar = |rng: &mut Rng| {
        let n = 1 + rng.por_debajo(4) as usize;
        let mut t = 0;
        let vs: Vec<_> = (0..n)
            .map(|_| {
                t += rng.por_debajo(60_000) as i64;
                (rng.por_debajo(400_000) as i64, rng.por_debajo(300_000) as i64, t)
            })
            .collect();
        plan(&vs)
    };
    let moviles: Vec<_> = (0..n_mov).map(|_| plan_al_azar(&mut rng)).collect();
    let obs: Vec<_> = (0..n_mov / 2).map(|_| plan_al_azar(&mut rng)).collect();
    let meta = (0..n_mov).map(|_| (rng.por_debajo(n_fac as u64) as u32, rng.por_debajo(20) == 0)).collect();
    (moviles, obs, meta)
}

fn armar<'a>(pm: &'a [Plan], po: &'a [Plan], meta: &[(u32, bool)], n_fac: u32) -> (Vec<Observador<'a>>, Vec<Movil<'a>>) {
    let obs = po.iter().enumerate().map(|(i, p)| Observador::movil(i as u32 % n_fac, p, if i % 3 == 0 { 12 * U } else { 30 * U })).collect();
    let mov = pm
        .iter()
        .zip(meta)
        .enumerate()
        .map(|(i, (p, &(f, grande)))| {
            let mut m = Movil::new(i as u64, f, Donde::Plan(p));
            m.atencion = grande.then_some(radios::CARAVANA_GRANDE);
            m
        })
        .collect();
    (obs, mov)
}

/// Referencia lenta O(observadores × móviles) para comprobar la rejilla.
fn fuerza_bruta(v: &Vision, t: Instante, obs: &[Observador], mov: &[Movil]) -> Muestra {
    let mut fs: Vec<u32> = obs.iter().map(|o| o.faccion).chain(mov.iter().map(|m| m.faccion)).collect();
    fs.sort_unstable();
    fs.dedup();
    let por_faccion = fs
        .into_iter()
        .map(|f| {
            let mut grupo = v.aliados(f);
            grupo.push(f);
            let mut ids = vec![];
            for m in mov {
                let Some(p) = m.donde.en(t) else { continue };
                let visto = m.faccion == f
                    || obs.iter().filter(|o| grupo.contains(&o.faccion)).any(|o| {
                        let c = o.donde.en(t).unwrap();
                        c.dist2(p) <= (o.radio as i128).pow(2) || m.atencion.is_some_and(|a| c.dist2(p) <= (a as i128).pow(2))
                    });
                if visto {
                    ids.push(m.entidad);
                }
            }
            ids.sort_unstable();
            (f, ids)
        })
        .collect();
    Muestra { t, por_faccion }
}

fn con_hilos<T: Send>(n: usize, f: impl FnOnce() -> T + Send) -> T {
    rayon::ThreadPoolBuilder::new().num_threads(n).build().unwrap().install(f)
}

#[test]
fn coincide_con_fuerza_bruta_y_no_depende_de_los_hilos() {
    let n_fac = 12;
    let (pm, po, meta) = mundo_aleatorio(7, 600, n_fac);
    let (obs, mov) = armar(&pm, &po, &meta, n_fac);
    let v = Vision::con_alianzas([(0, 1), (1, 2), (5, 9)]);
    for t in [0, 90_000, 250_000] {
        let esperado = fuerza_bruta(&v, t, &obs, &mov);
        let a = con_hilos(1, || v.muestrear(t, &obs, &mov));
        let b = con_hilos(8, || v.muestrear(t, &obs, &mov));
        assert_eq!(a, esperado, "rejilla == fuerza bruta en t={t}");
        assert_eq!(a, b, "1 y 8 hilos dan lo mismo");
        assert!(a.por_faccion.iter().map(|(_, v)| v.len()).sum::<usize>() > 100, "la prueba debe ver cosas");
    }
}

#[test]
fn el_orden_de_entrada_no_cambia_la_muestra() {
    let n_fac = 6;
    let (pm, po, meta) = mundo_aleatorio(3, 200, n_fac);
    let (obs, mov) = armar(&pm, &po, &meta, n_fac);
    let v = Vision::new();
    let a = v.muestrear(60_000, &obs, &mov);
    let (mut obs2, mut mov2) = (obs.clone(), mov.clone());
    obs2.reverse();
    mov2.reverse();
    assert_eq!(a, v.muestrear(60_000, &obs2, &mov2));
}

#[test]
fn niebla_reglas_del_documento() {
    let mut n = Niebla::nueva(10, 10);
    // Radio 0 en el centro de la celda (2,3): marca solo la propia celda del ojo.
    assert!(n.descubrir(1, pt(2 * 4000 + 2000, 3 * 4000 + 2000), 0));
    assert!(n.explorada(1, 2, 3) && !n.explorada(1, 3, 3));
    assert!(!n.descubrir(1, pt(2 * 4000 + 2000, 3 * 4000 + 2000), 0), "sin novedad: false");
    // La celda del ojo se marca aunque el radio no alcance su centro; el vecino (3,3) solo si su centro está a ≤ radio.
    let mut m = Niebla::nueva(10, 10);
    m.descubrir(1, pt(2 * 4000 + 100, 3 * 4000 + 100), 100);
    assert!(m.explorada(1, 2, 3) && !m.explorada(1, 3, 3) && !m.explorada(1, 2, 4));
    // Desde el centro de la celda (5,5) con radio 4000: el centro de las 4 vecinas está a exactamente 4000 → entran;
    // las diagonales (5657) no.
    let mut k = Niebla::nueva(10, 10);
    k.descubrir(1, pt(5 * 4000 + 2000, 5 * 4000 + 2000), 4000);
    let marcadas: Vec<_> = (0..10).flat_map(|f| (0..10).map(move |c| (c, f))).filter(|&(c, f)| k.explorada(1, c, f)).collect();
    assert_eq!(marcadas.len(), 5);
    assert!(k.explorada(1, 4, 5) && k.explorada(1, 6, 5) && k.explorada(1, 5, 4) && k.explorada(1, 5, 6) && !k.explorada(1, 4, 4));
    // Ojo fuera del mapa: sin pánico, y el disco que entra en el mapa se marca.
    k.descubrir(1, pt(-1000, -1000), 50_000);
    assert!(k.explorada(1, 0, 0));
    assert!(!k.explorada(2, 0, 0), "otra Facción no ha visto nada");
}

#[test]
fn niebla_orden_de_bits_volcado_y_restauracion() {
    let mut n = Niebla::nueva(10, 10);
    n.descubrir(1, pt(9 * 4000 + 2000, 2000), 0); // celda i=9 → byte 1, bit 1
    let b = n.volcar(1);
    assert_eq!(b.len(), 13);
    assert_eq!((b[0], b[1]), (0, 0b10));
    n.descubrir(1, pt(2000, 2000), 0); // i=0
    let b = n.volcar(1);
    assert_eq!((b[0], b[1]), (1, 0b10));
    let mut r = Niebla::nueva(10, 10);
    r.restaurar(1, &b);
    assert_eq!(r.volcar(1), b);
    // Más corta: el resto sin explorar. Más larga: se trunca y se limpian los bits sobrantes del último byte.
    r.restaurar(2, &[0xff]);
    assert_eq!(r.volcar(2).iter().map(|x| x.count_ones()).sum::<u32>(), 8);
    r.restaurar(3, &[0xff; 40]);
    assert_eq!(r.volcar(3).iter().map(|x| x.count_ones()).sum::<u32>(), 100);
    assert_eq!(r.volcar(99), Vec::<u8>::new());
}

#[test]
fn niebla_fundir_une_y_borra_el_origen() {
    let mut n = Niebla::nueva(10, 10);
    n.descubrir(100, pt(2000, 2000), 0);
    n.descubrir(1, pt(6000, 2000), 0);
    n.fundir(100, 1);
    assert!(n.explorada(1, 0, 0) && n.explorada(1, 1, 0));
    assert!(!n.explorada(100, 0, 0));
    assert_eq!(n.facciones().collect::<Vec<_>>(), vec![1]);
}

#[test]
fn grabar_es_determinista_con_1_y_8_hilos() {
    let n_fac = 20;
    let (_, po, _) = mundo_aleatorio(11, 400, n_fac);
    let (obs, _) = armar(&[], &po, &[], n_fac);
    let hacer = || {
        let mut n = Niebla::nueva(667, 500);
        for t in [0, 60_000, 120_000] {
            n.grabar(t, &obs);
        }
        (0..n_fac).map(|f| n.volcar(f)).collect::<Vec<_>>()
    };
    let (a, b) = (con_hilos(1, hacer), con_hilos(8, hacer));
    assert_eq!(a, b);
    assert!(a.iter().all(|v| v.iter().any(|&x| x != 0)));
}

fn rect(x0: Mil, y0: Mil, x1: Mil, y1: Mil) -> Rect {
    Rect { min: pt(x0, y0), max: pt(x1, y1) }
}

#[test]
fn tramo_visible_corta_en_el_horizonte_y_no_revela_el_destino() {
    // De (0,0) a (1000,0) unidades en 1000 s. Con horizonte 30 s en t=100 s solo se ve de x=100 a x=130.
    let p = plan(&[(0, 0, 0), (1000 * U, 0, 1_000_000)]);
    let todo = rect(-10_000 * U, -10_000 * U, 10_000 * U, 10_000 * U);
    let tr = tramo_visible(&p, 100_000, 30_000, todo);
    assert_eq!(tr, vec![Vertice { p: pt(100 * U, 0), en: 100_000 }, Vertice { p: pt(130 * U, 0), en: 130_000 }]);
    assert!(tr.iter().all(|v| v.p.x < 1000 * U), "ningún vértice llega al destino");
}

#[test]
fn tramo_visible_recorta_a_la_vista_y_conserva_vertices_intermedios() {
    let p = plan(&[(0, 0, 0), (100 * U, 0, 100_000), (100 * U, 100 * U, 200_000)]);
    // Vista x ∈ [40,80]: en t=0 con horizonte 60 s, entra en x=40 (t=40 s) y sale en x=60? No: el horizonte acaba en t=60 s.
    let r = rect(40 * U, -U, 80 * U, 10 * U);
    let tr = tramo_visible(&p, 0, 60_000, r);
    assert_eq!(tr, vec![Vertice { p: pt(40 * U, 0), en: 40_000 }, Vertice { p: pt(60 * U, 0), en: 60_000 }]);
    // Con horizonte 150 s: sale por x=80 (t=80 s) y no llega a ver la vuelta.
    let tr = tramo_visible(&p, 0, 150_000, r);
    assert_eq!(tr.first().unwrap().en, 40_000);
    assert_eq!(tr.last(), Some(&Vertice { p: pt(80 * U, 0), en: 80_000 }));
    // Plan que dobla dentro de la vista: el vértice de la esquina se conserva.
    let grande = rect(0, -U, 200 * U, 200 * U);
    let tr = tramo_visible(&p, 50_000, 100_000, grande);
    assert_eq!(tr.len(), 3);
    assert_eq!(tr[1], Vertice { p: pt(100 * U, 0), en: 100_000 });
    assert_eq!(tr[2], Vertice { p: pt(100 * U, 50 * U), en: 150_000 });
}

#[test]
fn tramo_visible_vacio_fuera_de_la_vista_o_del_horizonte() {
    let p = plan(&[(0, 0, 0), (1000 * U, 0, 1_000_000)]);
    assert!(tramo_visible(&p, 0, 30_000, rect(500 * U, -U, 600 * U, U)).is_empty(), "entra después del horizonte");
    assert!(tramo_visible(&p, 0, 30_000, rect(0, 100 * U, 50 * U, 200 * U)).is_empty(), "lejos de la vista");
    assert!(tramo_visible(&Plan::default(), 0, 30_000, rect(0, 0, U, U)).is_empty());
    // Parado (plan acabado): un vértice en t si está dentro.
    let q = plan(&[(5 * U, 5 * U, 0)]);
    assert_eq!(tramo_visible(&q, 99, 30_000, rect(0, 0, 10 * U, 10 * U)), vec![Vertice { p: pt(5 * U, 5 * U), en: 99 }]);
    assert!(tramo_visible(&q, 99, 30_000, rect(6 * U, 6 * U, 10 * U, 10 * U)).is_empty());
}
