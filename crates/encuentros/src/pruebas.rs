//! Pruebas contra una referencia por fuerza bruta que avanza de 1 ms en 1 ms con aritmética racional exacta
//! (independiente de la fórmula del crate: multiplica por las duraciones de cada tramo, sin mcm ni vértices).

use super::*;
use base::Rng;
use cola::{Clave, Cola, Generaciones};

fn rango(rng: &mut Rng, lo: i64, hi: i64) -> i64 {
    lo + rng.por_debajo((hi - lo + 1) as u64) as i64
}

fn pt(rng: &mut Rng, m: i64) -> Punto {
    Punto::new(rango(rng, -m, m), rango(rng, -m, m))
}

/// Posición exacta en `t` como `(x·den, y·den, den)`.
fn exacta(tr: &Tramo, t: Instante) -> (i128, i128, i128) {
    let den = tr.duracion();
    let k = (t.clamp(tr.a.en, tr.b.en) - tr.a.en) as i128;
    let (dx, dy) = tr.delta();
    (tr.a.p.x as i128 * den + k * dx, tr.a.p.y as i128 * den + k * dy, den)
}

fn cerca(pa: (i128, i128, i128), pb: (i128, i128, i128), radio: Mil) -> bool {
    let (dx, dy, dd) = (pa.0 * pb.2 - pb.0 * pa.2, pa.1 * pb.2 - pb.1 * pa.2, pa.2 * pb.2);
    dx * dx + dy * dy <= (radio as i128 * dd).pow(2)
}

fn fuerza_bruta(a: &Tramo, b: &Tramo, radio: Mil) -> Option<Instante> {
    if a.degenerado() || b.degenerado() {
        return None;
    }
    (a.a.en.max(b.a.en)..=a.b.en.min(b.b.en)).find(|&t| cerca(exacta(a, t), exacta(b, t), radio))
}

fn tramo_aleatorio(rng: &mut Rng, m: i64, dur_max: i64) -> Tramo {
    let t0 = rango(rng, 0, 1500);
    let (p0, dur) = (pt(rng, m), rango(rng, 1, dur_max));
    let p1 = if rng.por_debajo(4) == 0 { p0 } else { pt(rng, m) };
    Tramo::nuevo(p0, t0, p1, t0 + dur)
}

#[test]
fn acercamiento_coincide_con_la_fuerza_bruta_en_escenarios_aleatorios() {
    let mut rng = Rng::derivado(1, "acercamiento");
    let (mut aciertos, mut total) = (0, 0);
    for caso in 0..30_000 {
        let radio = rango(&mut rng, 0, 1500);
        let a = tramo_aleatorio(&mut rng, 3000, 3000);
        let b = match caso % 5 {
            0 => tramo_aleatorio(&mut rng, 3000, 3000),
            1 => Tramo::quieto(pt(&mut rng, 3000), rango(&mut rng, 0, 1500), rango(&mut rng, 1500, 4500)), // quieto
            2 => {
                // paralelo: misma dirección, otra velocidad y desfase lateral
                let (dx, dy) = (a.b.p.x - a.a.p.x, a.b.p.y - a.a.p.y);
                let (k, off) = (rango(&mut rng, -3, 3), pt(&mut rng, 800));
                let t0 = a.a.en + rango(&mut rng, -500, 500);
                let p0 = Punto::new(a.a.p.x + off.x, a.a.p.y + off.y);
                Tramo::nuevo(p0, t0.max(0), Punto::new(p0.x + k * dx, p0.y + k * dy), t0.max(0) + rango(&mut rng, 1, 3000))
            }
            3 => {
                // perpendicular: el vector gira 90 grados
                let (dx, dy) = (a.b.p.x - a.a.p.x, a.b.p.y - a.a.p.y);
                let p0 = pt(&mut rng, 3000);
                Tramo::nuevo(p0, a.a.en, Punto::new(p0.x - dy, p0.y + dx), a.b.en)
            }
            _ => Tramo::nuevo(pt(&mut rng, 3000), a.a.en, pt(&mut rng, 3000), a.b.en), // misma ventana
        };
        let esperado = fuerza_bruta(&a, &b, radio);
        assert_eq!(primer_acercamiento(a, b, radio), esperado, "caso {caso}: {a:?} {b:?} radio {radio}");
        assert_eq!(primer_acercamiento(b, a, radio), esperado, "simetría, caso {caso}");
        total += 1;
        aciertos += esperado.is_some() as u32;
    }
    assert!(aciertos > total / 8 && aciertos < total * 9 / 10, "escenarios poco variados: {aciertos}/{total}");
}

#[test]
fn rozando_el_radio() {
    // B quieto en el origen; A cruza en horizontal a altura y, con el punto más cercano en un milisegundo exacto.
    let b = Tramo::quieto(Punto::new(0, 0), 0, 5_000);
    for y in [1, 7, 100, 777] {
        let a = Tramo::nuevo(Punto::new(-1000, y), 1_000, Punto::new(1000, y), 3_000);
        assert_eq!(primer_acercamiento(a, b, y), Some(2_000), "toca justo en el punto más cercano");
        assert_eq!(primer_acercamiento(a, b, y - 1), None, "un milésimo menos y no llega");
        assert_eq!(primer_acercamiento(a, b, y + 1), fuerza_bruta(&a, &b, y + 1));
    }
    // empieza dentro del radio: el inicio de la ventana común
    let a = Tramo::nuevo(Punto::new(5, 0), 2_000, Punto::new(900, 0), 4_000);
    assert_eq!(primer_acercamiento(a, b, 10), Some(2_000));
    // ventanas que no se solapan, o que solo coinciden en un instante
    assert_eq!(primer_acercamiento(a, Tramo::quieto(Punto::new(5, 0), 4_001, 9_000), 10), None);
    assert_eq!(primer_acercamiento(Tramo::quieto(Punto::new(0, 0), 0, 100), Tramo::quieto(Punto::new(3, 4), 100, 200), 5), Some(100));
    assert_eq!(primer_acercamiento(Tramo::quieto(Punto::new(0, 0), 0, 100), Tramo::quieto(Punto::new(3, 4), 100, 200), 4), None);
}

#[test]
fn tramos_quietos_muy_largos_y_paralelos_identicos() {
    // aparcado un año: no desborda ni cambia el resultado
    let anio = 31_536_000_000;
    let parado = Tramo::quieto(Punto::new(1_000_000, 1_000_000), 0, anio);
    let a = Tramo::nuevo(Punto::new(1_000_000 + 5_000, 1_000_000), 100_000, Punto::new(1_000_000 - 5_000, 1_000_000), 110_000);
    assert_eq!(primer_acercamiento(a, parado, 2_000), Some(103_000), "a 1.000 mil/s recorre 3.000 mil en 3 s");
    // idénticos en paralelo a 3 de distancia: siempre dentro (radio 3) o nunca (radio 2)
    let p = Tramo::nuevo(Punto::new(0, 0), 0, Punto::new(10_000, 0), 1_000);
    let q = Tramo::nuevo(Punto::new(0, 3), 0, Punto::new(10_000, 3), 1_000);
    assert_eq!(primer_acercamiento(p, q, 3), Some(0));
    assert_eq!(primer_acercamiento(p, q, 2), None);
}

#[test]
fn intervalo_dentro_coincide_con_la_fuerza_bruta() {
    let mut rng = Rng::derivado(2, "intervalo");
    for caso in 0..5_000 {
        let radio = rango(&mut rng, 0, 700);
        let (a, b) = (tramo_aleatorio(&mut rng, 2500, 2500), tramo_aleatorio(&mut rng, 2500, 2500));
        if a.degenerado() || b.degenerado() {
            continue;
        }
        let dentro: Vec<Instante> = (a.a.en.max(b.a.en)..=a.b.en.min(b.b.en)).filter(|&t| cerca(exacta(&a, t), exacta(&b, t), radio)).collect();
        let esperado = dentro.first().zip(dentro.last()).map(|(&f, &l)| (f, l));
        assert_eq!(intervalo_de_acercamiento(a, b, radio), esperado, "caso {caso}");
        if let Some((f, l)) = esperado {
            assert_eq!(dentro.len() as i64, l - f + 1, "el conjunto es un intervalo contiguo");
        }
    }
}

fn plan_aleatorio(rng: &mut Rng, m: i64, tramos: usize, gen: u32) -> Plan {
    let mut t = rango(rng, 0, 3000);
    let mut v = vec![Vertice { p: pt(rng, m), en: t }];
    for _ in 0..tramos {
        t += rango(rng, 1, 2000);
        let p = if rng.por_debajo(5) == 0 { v.last().unwrap().p } else { pt(rng, m) };
        v.push(Vertice { p, en: t });
    }
    Plan::nuevo(gen, v)
}

fn dentro_en(plan: &Plan, centro: Punto, radio: Mil, t: Instante) -> bool {
    let tr = Tramo::del_plan(plan).find(|x| x.b.en >= t).unwrap_or_else(|| Tramo::del_plan(plan).last().unwrap());
    cerca(exacta(&tr, t), (centro.x as i128, centro.y as i128, 1), radio)
}

#[test]
fn llegada_y_salida_de_radio() {
    let mut rng = Rng::derivado(3, "radio");
    for caso in 0..3_000 {
        let plan = plan_aleatorio(&mut rng, 3000, 4, 1);
        let (centro, radio) = (pt(&mut rng, 3000), rango(&mut rng, 0, 900));
        let (ini, fin) = (plan.inicio().unwrap(), plan.fin().unwrap());
        let dentro: Vec<bool> = (ini..=fin).map(|t| dentro_en(&plan, centro, radio, t)).collect();
        let llegada = dentro.iter().position(|&d| d).map(|i| ini + i as i64);
        assert_eq!(llegada_a_radio(&plan, centro, radio), llegada, "llegada, caso {caso}");
        let salida = llegada.and_then(|l| (l..=fin).find(|&t| !dentro[(t - ini) as usize]));
        assert_eq!(salida_de_radio(&plan, centro, radio), salida, "salida, caso {caso}");
    }
    let una = Plan::nuevo(1, vec![Vertice { p: Punto::new(1, 1), en: 50 }]);
    assert_eq!(llegada_a_radio(&una, Punto::new(0, 0), 2), Some(50));
    assert_eq!(salida_de_radio(&una, Punto::new(0, 0), 2), None);
    assert_eq!(llegada_a_radio(&Plan::default(), Punto::new(0, 0), 2), None);
}

#[test]
fn llegada_a_la_puerta_de_una_plaza() {
    // 10 unidades en 10 s hacia una plaza con puerta de radio 2: llega a 2 de la puerta, a los 8 s
    let plan = Plan::nuevo(1, vec![Vertice { p: Punto::new(0, 0), en: 0 }, Vertice { p: Punto::new(10_000, 0), en: 10_000 }]);
    assert_eq!(llegada_a_radio(&plan, Punto::new(10_000, 0), 2_000), Some(8_000));
    assert_eq!(salida_de_radio(&plan, Punto::new(5_000, 0), 1_000), Some(6_001));
}

// --- Persecución ----------------------------------------------------------------------------------------------------

fn alcanzada(plan: &Plan, p: Punto, desde: Instante, v: Mil, radio: Mil, t: Instante) -> bool {
    let (first, last) = (plan.vertices[0], *plan.vertices.last().unwrap());
    let tr = if t <= first.en {
        Tramo::quieto(first.p, t, t)
    } else if t >= last.en {
        Tramo::quieto(last.p, t, t)
    } else {
        Tramo::del_plan(plan).find(|x| x.b.en >= t).unwrap()
    };
    let (x, y, den) = exacta(&tr, t);
    let (qx, qy) = (x - p.x as i128 * den, y - p.y as i128 * den);
    let alcance = den * (v as i128 * (t - desde) as i128 + 1000 * radio as i128);
    (qx * qx + qy * qy) * 1_000_000 <= alcance * alcance
}

#[test]
fn interceptar_coincide_con_la_fuerza_bruta() {
    let mut rng = Rng::derivado(4, "persecucion");
    let (mut hits, mut tardios) = (0, 0);
    for caso in 0..6_000 {
        let n = rango(&mut rng, 1, 4) as usize;
        let plan = plan_aleatorio(&mut rng, 4000, n, 1);
        let (p, desde) = (pt(&mut rng, 4000), rango(&mut rng, 0, 5000));
        let (v, radio) = (rango(&mut rng, 0, 2500), rango(&mut rng, 0, 300));
        let limite = plan.fin().unwrap().max(desde) + 120_000;
        let esperado = (desde..=limite).find(|&t| alcanzada(&plan, p, desde, v, radio, t));
        let obtenido = interceptar(p, desde, v, radio, &plan);
        match esperado {
            Some(t) => {
                assert_eq!(obtenido.map(|i| i.instante), Some(t), "caso {caso}: p {p:?} desde {desde} v {v} r {radio} {plan:?}");
                assert_eq!(obtenido.unwrap().punto, plan.posicion_en(t).unwrap());
                hits += 1;
                tardios += (t > plan.fin().unwrap()) as u32;
            }
            None => assert!(obtenido.is_none_or(|i| i.instante > limite), "caso {caso}: sobra {obtenido:?}"),
        }
    }
    assert!(hits > 3_000 && tardios > 300, "escenarios poco variados: {hits} alcances, {tardios} tras el último vértice");
}

#[test]
fn interceptar_casos_simples() {
    // presa quieta a 6.000 mil, perseguidor a 1.000 mil/s sin radio: 6 s
    let quieta = Plan::nuevo(1, vec![Vertice { p: Punto::new(6_000, 0), en: 0 }]);
    assert_eq!(interceptar(Punto::new(0, 0), 10_000, 1_000, 0, &quieta).unwrap().instante, 16_000);
    assert_eq!(interceptar(Punto::new(0, 0), 10_000, 1_000, 3_000, &quieta).unwrap().instante, 13_000);
    assert_eq!(interceptar(Punto::new(0, 0), 10_000, 0, 0, &quieta), None, "sin velocidad nunca alcanza");
    // presa que huye más rápido que el perseguidor por un tramo finito y luego se para: la alcanza tras parar
    let huye = Plan::nuevo(1, vec![Vertice { p: Punto::new(1_000, 0), en: 0 }, Vertice { p: Punto::new(11_000, 0), en: 5_000 }]);
    let i = interceptar(Punto::new(0, 0), 0, 1_000, 0, &huye).unwrap();
    assert_eq!(i.punto, Punto::new(11_000, 0));
    assert_eq!(i.instante, 11_000);
    assert_eq!(interceptar(Punto::new(0, 0), 0, 1_000, 0, &Plan::default()), None);
}

// --- Rejilla --------------------------------------------------------------------------------------------------------

/// Referencia: primer acercamiento entre dos planes comparando todos los pares de tramos.
fn bruto_planes(a: &Plan, b: &Plan, radio: Mil) -> Option<Instante> {
    let tb: Vec<Tramo> = Tramo::del_plan(b).collect();
    Tramo::del_plan(a).filter_map(|x| tb.iter().filter_map(|y| fuerza_bruta(&x, y, radio)).min()).min()
}

fn mundo_pequeno(rng: &mut Rng, n: u64) -> Vec<(u64, Plan)> {
    (0..n)
        .map(|e| {
            let plan = if e % 7 == 0 {
                // aparcado «hasta el horizonte»: va al índice solo espacial
                let t = rango(rng, 0, 3000);
                let p = pt(rng, 6000);
                Plan::nuevo(1, vec![Vertice { p, en: t }, Vertice { p, en: t + 100_000 }])
            } else {
                let n = rango(rng, 1, 5) as usize;
                plan_aleatorio(rng, 6000, n, 1)
            };
            (e, plan)
        })
        .collect()
}

fn comprobar(rej: &Rejilla, mundo: &[(u64, Plan)], radio: Mil) {
    for (e, plan) in mundo {
        let mut esperado: Vec<(Instante, u64)> =
            mundo.iter().filter(|(o, _)| o != e).filter_map(|(o, q)| bruto_planes(plan, q, radio).map(|t| (t, *o))).collect();
        esperado.sort_unstable();
        let obtenido: Vec<(Instante, u64)> = rej.encuentros_de(*e, plan, radio).iter().map(|x| (x.instante, x.otra)).collect();
        assert_eq!(obtenido, esperado, "entidad {e}");
        let cands = rej.candidatos(*e);
        assert!(esperado.iter().all(|(_, o)| cands.contains(o)), "un par con encuentro no es candidato");
    }
}

#[test]
fn rejilla_encuentra_los_mismos_encuentros_que_la_fuerza_bruta() {
    let mut rng = Rng::derivado(5, "rejilla");
    let radio = 400;
    let mut rej = Rejilla::nueva(2_000, 1_000, 500);
    let mut mundo = mundo_pequeno(&mut rng, 70);
    for (e, p) in &mundo {
        rej.apuntar(*e, 1, p);
    }
    assert_eq!(rej.len(), 70);
    comprobar(&rej, &mundo, radio);
    let con_encuentro = mundo.iter().filter(|(e, p)| !rej.encuentros_de(*e, p, radio).is_empty()).count();
    assert!(con_encuentro > 10, "mundo sin encuentros: {con_encuentro}");
    let sin_candidatos = (0..70).filter(|&e| rej.candidatos(e).is_empty()).count();
    assert!(sin_candidatos > 0 || rej.candidatos(0).len() < 69, "la rejilla no filtra nada");

    // cambios de plan: quitan sus apuntes, meten los nuevos y todo sigue igual
    for e in [3u64, 14, 21, 40, 41, 69] {
        let nuevo = plan_aleatorio(&mut rng, 6000, 3, 2);
        rej.apuntar(e, 2, &nuevo);
        mundo[e as usize].1 = nuevo;
    }
    comprobar(&rej, &mundo, radio);
    // quitar
    rej.quitar(5);
    rej.quitar(5);
    let quedan: Vec<_> = mundo.iter().filter(|(e, _)| *e != 5).cloned().collect();
    assert!(!rej.contiene(5) && rej.candidatos(5).is_empty());
    assert!((0..70).all(|e| !rej.candidatos(e).contains(&5)));
    comprobar(&rej, &quedan, radio);
    // generación del otro en el encuentro
    let (e, p) = &mundo[3];
    assert!(rej.encuentros_de(*e, p, radio).iter().all(|x| x.otra_generacion == if [14, 21, 40, 41, 69].contains(&x.otra) { 2 } else { 1 }));
}

#[test]
fn encuentro_simetrico_y_con_punto_medio() {
    let mut rej = Rejilla::nueva(5_000, 10_000, 1_000);
    let a = Plan::nuevo(1, vec![Vertice { p: Punto::new(0, 0), en: 0 }, Vertice { p: Punto::new(10_000, 0), en: 10_000 }]);
    let b = Plan::nuevo(1, vec![Vertice { p: Punto::new(10_000, 0), en: 0 }, Vertice { p: Punto::new(0, 0), en: 10_000 }]);
    rej.apuntar(1, 1, &a);
    rej.apuntar(2, 1, &b);
    let (ea, eb) = (rej.encuentros_de(1, &a, 1_000), rej.encuentros_de(2, &b, 1_000));
    assert_eq!((ea.len(), eb.len()), (1, 1));
    assert_eq!((ea[0].instante, ea[0].punto), (eb[0].instante, eb[0].punto));
    assert_eq!(ea[0].instante, 4_500, "se acercan a 1.000 mil cuando cada una ha hecho 4.500");
    assert_eq!(ea[0].punto, Punto::new(5_000, 0));
}

// --- Cola ----------------------------------------------------------------------------------------------------------

#[test]
fn eventos_de_un_plan_nuevo_en_la_cola() {
    let mut rej = Rejilla::nueva(5_000, 10_000, 3_000);
    let mut gens = Generaciones::nuevas();
    let mut cola: Cola<EventoEspacial> = Cola::nueva();

    let g2 = gens.subir(2);
    rej.apuntar(2, g2, &Plan::nuevo(1, vec![Vertice { p: Punto::new(5_000, 0), en: 0 }, Vertice { p: Punto::new(5_000, 0), en: 100_000 }]));

    let g1 = gens.subir(1);
    let plan = Plan::nuevo(1, vec![Vertice { p: Punto::new(0, 0), en: 0 }, Vertice { p: Punto::new(10_000, 0), en: 10_000 }]);
    rej.apuntar(1, g1, &plan);
    let puerta = Puerta { id: 77, centro: Punto::new(10_000, 0), radio: 2_000 };
    let evs = eventos_de_plan(&rej, 1, g1, &plan, 3_000, &[puerta]);
    assert_eq!(evs.iter().map(|e| e.instante).collect::<Vec<_>>(), [2_000, 8_000]);
    assert!(evs.iter().all(|e| e.prioridad == PRIORIDAD_ENCUENTROS && e.clave == Clave { entidad: 1, generacion: g1 }));
    programar(&mut cola, evs);

    // un persecutor sobre la entidad 2 (quieta en x=5.000)
    let pers = evento_de_persecucion(9, 0, Punto::new(5_000, 5_000), 0, 1_000, 3_000, 2, g2, &Plan::nuevo(1, vec![Vertice { p: Punto::new(5_000, 0), en: 0 }])).unwrap();
    assert_eq!(pers.instante, 2_000);
    programar(&mut cola, [pers]);

    // la entidad 2 cambia de plan: el encuentro con la 1 ya no vale y la intercepción tampoco
    gens.subir(2);
    let vigente = |c: Clave| gens.es_vigente(c);
    let mut salidos = Vec::new();
    while let Some(e) = cola.siguiente_hasta(i64::MAX, &vigente) {
        if e.carga.otras_vigentes(&gens) {
            salidos.push(e.carga);
        }
    }
    assert!(matches!(salidos.as_slice(), [EventoEspacial::Llegada { entidad: 1, puerta: 77, .. }]), "{salidos:?}");
}
