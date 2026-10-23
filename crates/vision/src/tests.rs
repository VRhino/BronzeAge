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

const T: i64 = 4000; // celda de las pruebas de la máscara

fn marcadas(m: &Mascara) -> Vec<(usize, usize)> {
    (0..m.filas()).flat_map(|f| (0..m.columnas()).map(move |c| (c, f))).filter(|&(c, f)| m.get(c, f)).collect()
}

#[test]
fn niebla_reglas_del_documento() {
    let mut n = Mascara::nueva(10, 10, T);
    // Radio 0 en el centro de la celda (2,3): marca solo la propia celda del ojo.
    assert!(n.marcar_vision(pt(2 * T + 2000, 3 * T + 2000), 0));
    assert!(n.get(2, 3) && !n.get(3, 3));
    assert!(!n.marcar_vision(pt(2 * T + 2000, 3 * T + 2000), 0), "sin novedad: false");
    // La celda del ojo se marca aunque el radio no alcance su centro; el vecino (3,3) solo si su centro está a ≤ radio.
    let mut m = Mascara::nueva(10, 10, T);
    m.marcar_vision(pt(2 * T + 100, 3 * T + 100), 100);
    assert!(m.get(2, 3) && !m.get(3, 3) && !m.get(2, 4));
    // Desde el centro de la celda (5,5) con radio 4000: el centro de las 4 vecinas está a exactamente 4000 → entran; las diagonales (5657) no.
    let mut k = Mascara::nueva(10, 10, T);
    k.marcar_vision(pt(5 * T + 2000, 5 * T + 2000), 4000);
    assert_eq!(marcadas(&k), vec![(5, 4), (4, 5), (5, 5), (6, 5), (5, 6)]);
    // Ojo fuera del mapa: sin pánico, y el disco que entra en el mapa se marca.
    k.marcar_vision(pt(-1000, -1000), 50_000);
    assert!(k.get(0, 0) && k.explorado_en(pt(10, 10)) && !k.explorado_en(pt(-1, 10)));
}

#[test]
fn niebla_orden_de_bits_volcado_y_restauracion() {
    let mut n = Mascara::nueva(10, 10, T);
    n.marcar_vision(pt(9 * T + 2000, 2000), 0); // celda i=9 → byte 1, bit 1
    assert_eq!(n.bytes().len(), 13);
    assert_eq!((n.bytes()[0], n.bytes()[1]), (0, 0b10));
    n.marcar_vision(pt(2000, 2000), 0); // i=0
    assert_eq!((n.bytes()[0], n.bytes()[1]), (1, 0b10));
    assert_eq!(Mascara::desde_bytes(10, 10, T, n.bytes()), n);
    // Más corta: el resto sin explorar. Más larga: se trunca y se limpian los bits sobrantes del último byte.
    assert_eq!(Mascara::desde_bytes(10, 10, T, &[0xff]).contar(), 8);
    assert_eq!(Mascara::desde_bytes(10, 10, T, &[0xff; 40]).contar(), 100);
    let mut a = Mascara::desde_bytes(10, 10, T, &[0b1]);
    a.fundir(&Mascara::desde_bytes(10, 10, T, &[0b10]));
    assert_eq!(a.bytes()[0], 0b11, "fundir es el OR de lo explorado");
    assert_eq!(rejilla(2_667_000, 2_000_000, 5_000), (534, 400), "la rejilla del mapa real con celdas de 5 unidades");
}

#[test]
fn barrer_marca_lo_que_vio_por_el_camino_y_nada_mas() {
    // Una columna en diagonal y en zigzag, con alcance 30: lo barrido contiene lo que se ve desde cada punto del camino (muestreado cada 0,1 u) y no pasa del alcance más 0,1 u.
    let p = plan(&[(20 * U, 30 * U, 0), (180 * U, 150 * U, 200_000), (60 * U, 210 * U, 330_000)]);
    let r = recorrido(&p, pt(0, 0), 0, 400_000);
    assert!(r.windows(2).all(|w| (w[1].p.x - w[0].p.x).abs().max((w[1].p.y - w[0].p.y).abs()) <= 100 * U), "los tramos largos se parten");
    let mut barrido = Mascara::nueva(60, 60, T);
    assert!(barrido.barrer(&r, 30 * U));
    let (mut dentro, mut fuera) = (Mascara::nueva(60, 60, T), Mascara::nueva(60, 60, T));
    for t in (0..=330_000).step_by(100) {
        let q = p.posicion_en(t).unwrap();
        dentro.marcar_vision(q, 30 * U);
        fuera.marcar_vision(q, 30 * U + 100);
    }
    let mut union = barrido.clone();
    union.fundir(&dentro);
    assert_eq!(union, barrido, "todo lo que se ve desde el camino queda barrido");
    let mut techo = fuera.clone();
    techo.fundir(&barrido);
    assert_eq!(techo, fuera, "y nada que no se viera");
    assert!(!barrido.barrer(&r, 30 * U), "barrer dos veces lo mismo no descubre nada");
    // Quieto (sin plan): el disco en su sitio.
    let mut quieto = Mascara::nueva(60, 60, T);
    quieto.barrer(&recorrido(&Plan::default(), pt(100 * U, 100 * U), 0, 50_000), 15 * U);
    let mut disco = Mascara::nueva(60, 60, T);
    disco.marcar_vision(pt(100 * U, 100 * U), 15 * U);
    assert_eq!(quieto, disco);
}

#[test]
fn ultima_vez_da_el_instante_en_que_el_punto_salio_de_la_vista() {
    // De (0,0) a (100,0) en 100 s y luego quieto. Un punto en (50,10) con alcance 30 se ve mientras |x − 50| ≤ √800 = 28,28: hasta x = 78,28 (t = 78,28 s).
    let p = plan(&[(0, 0, 0), (100 * U, 0, 100_000)]);
    let r = recorrido(&p, pt(0, 0), 0, 200_000);
    let t = ultima_vez(&r, pt(50 * U, 10 * U), 30 * U).unwrap();
    assert!((78_283..=78_285).contains(&t), "{t}");
    assert_eq!(ultima_vez(&r, pt(50 * U, 31 * U), 30 * U), None, "nunca a menos de 30");
    assert_eq!(ultima_vez(&r, pt(100 * U, 20 * U), 30 * U), Some(200_000), "lo ve todavía al acabar el recorrido");
    assert_eq!(ultima_vez(&r, pt(-30 * U, 0), 30 * U), Some(0), "solo en el primer instante (no estricto)");
    // Sobre un recorrido que solo empieza a verlo al final del primer tramo.
    let r = recorrido(&p, pt(0, 0), 0, 50_000);
    assert_eq!(ultima_vez(&r, pt(80 * U, 0), 30 * U), Some(50_000));
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
