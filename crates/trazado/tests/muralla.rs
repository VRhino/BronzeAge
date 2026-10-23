//! Pruebas del trazo de recintos (Docs/Mechanics/Murallas.md §3, §4, §12). El TypeScript usa otro hash y no hay paridad celda a
//! celda: se portan sus invariantes (`muralla.test.ts`) y se prueban propiedades sobre 20 ciudades sembradas.

mod comun;

use comun::*;
use std::collections::{HashSet, VecDeque};
use trazado::*;

const P: ParametrosMuralla = ParametrosMuralla { franja_de_ronda: 1, paso_torres: [0, 8, 5] };

/// 20 ciudades: semilla, perfil y tamaño distintos (de 35 a 140 piezas).
fn ciudades() -> Vec<Sim> {
    (0..20u64).map(|k| ciudad_de(100 + k, Perfil::TODOS[k as usize % 4], 35 + 5 * k as usize)).collect()
}

fn trazar(c: &Ciudad, nivel: u8) -> Trazo {
    trazar_recinto(c, &P, nivel).unwrap_or_else(|e| panic!("sin trazo: {e:?}"))
}

fn celdas_de(t: &Trazo) -> HashSet<Celda> {
    t.celdas.iter().map(|c| c.celda).collect()
}

/// Inundación 4-conexa con conjuntos (independiente de la rejilla del módulo) desde un marco por encima de la ventana.
fn alcanzables_desde_fuera(muro: &HashSet<Celda>) -> HashSet<Celda> {
    let m = MITAD + 1;
    let mut vistas = HashSet::new();
    let mut cola = VecDeque::new();
    for i in -m..=m {
        for c in [Celda::new(i, -m), Celda::new(i, m), Celda::new(-m, i), Celda::new(m, i)] {
            if vistas.insert(c) {
                cola.push_back(c);
            }
        }
    }
    while let Some(Celda { col, row }) = cola.pop_front() {
        for (dc, dr) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let n = Celda::new(col + dc, row + dr);
            if n.col.abs() <= m && n.row.abs() <= m && !muro.contains(&n) && vistas.insert(n) {
                cola.push_back(n);
            }
        }
    }
    vistas
}

fn red_de(c: &Ciudad) -> HashSet<Celda> {
    c.calles().into_iter().chain(c.caminos()).collect()
}

#[test]
fn el_trazo_cierra_no_pisa_y_encierra_al_centro_en_20_ciudades() {
    for (k, s) in ciudades().iter().enumerate() {
        let t = trazar(&s.c, 2);
        let muro = celdas_de(&t);
        assert_eq!(muro.len(), t.celdas.len(), "ciudad {k}: celdas repetidas");
        // cierra: desde fuera, sin cruzar el anillo, no se llega al Centro Urbano
        let fuera = alcanzables_desde_fuera(&muro);
        let centro = s.c.piezas().iter().find(|p| p.rol == Rol::Centro).unwrap();
        assert!(centro.rect.celdas().all(|c| !fuera.contains(&c)), "ciudad {k}: el anillo no aísla el Centro Urbano");
        // no pisa ningún edificio
        for p in s.c.piezas() {
            assert!(p.rect.celdas().all(|c| !muro.contains(&c)), "ciudad {k}: el anillo pisa la pieza {}", p.id);
        }
        // toda celda del anillo cabe en la ventana
        assert!(muro.iter().all(|c| c.col.abs() <= MITAD && c.row.abs() <= MITAD));
        // dentro / fuera / afueras: el Centro dentro, las granjas fuera, y la mayor parte de la ciudad dentro
        assert!(t.dentro.contains(&centro.id), "ciudad {k}");
        assert!(t.afueras_dentro.is_empty(), "ciudad {k}: granjas dentro {:?}", t.afueras_dentro);
        assert!(t.dentro.len() > t.fuera.len(), "ciudad {k}: {} dentro y {} fuera", t.dentro.len(), t.fuera.len());
        let urbanas = s.c.piezas().iter().filter(|p| p.categoria != Categoria::Afueras).count();
        assert_eq!(t.dentro.len() + t.fuera.len(), urbanas, "ciudad {k}");
        // el interior es lo que el anillo deja dentro: ni una celda de más, y el área coincide
        let interior = interior_de(&s.c, &t.celdas).expect("hay centro");
        assert_eq!(interior.area(), t.area_encerrada, "ciudad {k}");
        assert!(t.dentro.iter().all(|&id| s.c.pieza(id).unwrap().rect.celdas().all(|c| interior.contiene(c))));
    }
}

#[test]
fn el_anillo_es_la_capa_exterior_del_interior_y_se_recorre_sin_perder_celdas() {
    for (k, s) in ciudades().iter().enumerate() {
        let t = trazar(&s.c, 3);
        let interior = interior_de(&s.c, &t.celdas).unwrap();
        // el anillo = celdas fuera del interior con una vecina 4 dentro (impermeable y de una celda de grosor)
        let mut capa = HashSet::new();
        for col in -MITAD..=MITAD {
            for row in -MITAD..=MITAD {
                let c = Celda::new(col, row);
                if interior.contiene(c) {
                    for (dc, dr) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                        let n = Celda::new(col + dc, row + dr);
                        if !interior.contiene(n) {
                            capa.insert(n);
                        }
                    }
                }
            }
        }
        assert_eq!(capa, celdas_de(&t), "ciudad {k}");
        // el recorrido avanza por vecindad 8: como mucho unos pocos saltos (puntos gruesos donde el paseo se atasca; en las 20 ciudades salen 0-6)
        let n = t.celdas.len();
        let saltos = (0..n)
            .filter(|&i| {
                let (a, b) = (t.celdas[i].celda, t.celdas[(i + 1) % n].celda);
                (a.col - b.col).abs() > 1 || (a.row - b.row).abs() > 1
            })
            .count();
        assert!(saltos <= 6, "ciudad {k}: {saltos} saltos en un anillo de {n} celdas");
    }
}

#[test]
fn las_puertas_son_una_por_cruce_y_conectan_con_la_red() {
    for (k, s) in ciudades().iter().enumerate() {
        let t = trazar(&s.c, 1);
        assert!(t.puertas() >= 1, "ciudad {k}: sin puertas");
        let red = red_de(&s.c);
        let n = t.celdas.len();
        for i in 0..n {
            let c = t.celdas[i];
            // puerta ⇔ la celda ya era calle o camino de la ciudad
            if c.clase == ClaseCelda::Puerta {
                assert!(red.contains(&c.celda), "ciudad {k}: la puerta {:?} no está en la red", c.celda);
                assert_ne!(t.celdas[(i + 1) % n].clase, ClaseCelda::Puerta, "ciudad {k}: dos puertas seguidas");
            }
        }
        // cada cruce con la red deja al menos una puerta: toda tirada de celdas de red del anillo tiene una puerta
        let con_red = t.celdas.iter().filter(|c| red.contains(&c.celda)).count();
        assert!(t.puertas() <= con_red, "ciudad {k}");
    }
}

#[test]
fn con_el_anillo_puesto_la_red_sigue_siendo_un_solo_componente() {
    for (k, s) in ciudades().iter().enumerate().step_by(3) {
        let t = trazar(&s.c, 2);
        let mut c = s.c.clone();
        c.set_recintos(vec![Recinto::desde_celdas(&t.celdas, false)]);
        // el muro ocupa y las puertas no
        for cm in &t.celdas {
            assert_eq!(
                c.esta_ocupada(cm.celda) && !c.es_calle(cm.celda) && !c.es_camino(cm.celda),
                cm.clase != ClaseCelda::Puerta,
                "ciudad {k} {cm:?}"
            );
        }
        verificar(&c, s.radio());
        // y el trazo sobre la ciudad con su propio recinto es el mismo: se calcula sobre la ciudad sin recintos
        assert_eq!(trazar_recinto(&c, &P, 2), Ok(t));
    }
}

#[test]
fn las_torres_no_existen_en_la_empalizada_y_estan_separadas_en_los_otros_niveles() {
    for (k, s) in ciudades().iter().enumerate() {
        assert_eq!(trazar(&s.c, 1).torres(), 0, "ciudad {k}");
        for (nivel, paso) in [(2u8, 8usize), (3, 5)] {
            let t = trazar(&s.c, nivel);
            let pos: Vec<usize> = (0..t.celdas.len()).filter(|&i| t.celdas[i].clase == ClaseCelda::Torre).collect();
            assert!(!pos.is_empty(), "ciudad {k} nivel {nivel}: sin torres");
            assert!(pos.windows(2).all(|w| w[1] - w[0] >= paso), "ciudad {k} nivel {nivel}: torres juntas {pos:?}");
            // la puerta manda: ninguna torre es una puerta, y el nivel no cambia ni el anillo ni las puertas
            assert_eq!(t.puertas(), trazar(&s.c, 1).puertas(), "ciudad {k}");
        }
    }
}

#[test]
fn al_mejorar_las_torres_se_recalculan_sobre_el_mismo_anillo_sin_juntarse() {
    for (k, s) in ciudades().iter().enumerate() {
        let (n2, n3) = (trazar(&s.c, 2), trazar(&s.c, 3));
        let interior = interior_de(&s.c, &n2.celdas).unwrap();
        let mut mejorado = n2.celdas.clone();
        marcar_torres(&mut mejorado, &interior, &P, 3);
        assert_eq!(mejorado, n3.celdas, "ciudad {k}: mejorar = trazar directamente al nivel nuevo");
        let pos: Vec<usize> = (0..mejorado.len()).filter(|&i| mejorado[i].clase == ClaseCelda::Torre).collect();
        assert!(pos.windows(2).all(|w| w[1] - w[0] >= 5), "ciudad {k}: torres juntas tras mejorar {pos:?}");
        // y bajar a la empalizada las quita todas
        marcar_torres(&mut mejorado, &interior, &P, 1);
        assert!(mejorado.iter().all(|c| c.clase != ClaseCelda::Torre));
    }
}

#[test]
fn el_trazo_es_determinista_y_no_depende_de_como_se_reconstruya_la_ciudad() {
    for s in ciudades().iter().step_by(4) {
        let t = trazar(&s.c, 2);
        assert_eq!(trazar(&s.c, 2), t);
        let c = Ciudad::desde_partes(s.c.semilla(), *s.c.params(), LAYOUT_VERSION, s.c.piezas().to_vec(), Vec::new()).unwrap();
        assert_eq!(trazar(&c, 2), t);
    }
}

#[test]
fn sin_centro_urbano_no_hay_trazo() {
    let c = Ciudad::vacia(1, Parametros::default());
    assert_eq!(trazar_recinto(&c, &P, 1), Err(ErrorMuralla::SinCentroUrbano));
}

/// Una cadena de viviendas de 1×1 cada 10 celdas por la fila 0 hasta la columna `ultima`; cada una se conecta a la anterior por su corredor.
fn ciudad_con_brazo(ultima: i32) -> Ciudad {
    let mut s = Sim::nueva(3, Perfil::Nucleos, 5);
    let mut columnas: Vec<i32> = (10..ultima).step_by(10).collect();
    columnas.push(ultima);
    for (id, col) in (100..).zip(columnas) {
        let ped = s.pedido(T_VIVIENDA, id);
        s.c.colocar(&ped, Colocacion { rect: Rect::new(col, 0, 1, 1), rotado: false }).unwrap();
    }
    s.c
}

#[test]
fn si_el_anillo_no_cabe_en_la_ventana_se_rechaza_sin_recortar() {
    // la región llega a la columna 94 y el anillo a la 96: justo cabe
    let cabe = ciudad_con_brazo(94);
    let t = trazar_recinto(&cabe, &P, 1).expect("cabe");
    assert!(t.celdas.iter().any(|c| c.celda.col == MITAD));
    // una celda más y el anillo se saldría de la ventana
    assert_eq!(trazar_recinto(&ciudad_con_brazo(95), &P, 1), Err(ErrorMuralla::FueraDeVentana));
}

#[test]
fn ampliar_exige_envolver_estrictamente_al_anillo_anterior() {
    let mut envuelven = 0;
    for k in 0..12u64 {
        let perfil = Perfil::TODOS[k as usize % 4];
        // la misma ciudad (construcción determinista) cuando era pequeña y cuando ha crecido hasta 5 veces
        let (pequena, grande) = (ciudad_de(200 + k, perfil, 40), ciudad_de(200 + k, perfil, 40 + 15 * (k as usize + 1)));
        let (viejo, nuevo) = (trazar(&pequena.c, 1), trazar(&grande.c, 1));
        assert!(!contiene_a(&viejo.celdas, &viejo.celdas), "k{k}: calcar el anillo no es ampliar");
        // el extramuros del anillo viejo sobre la ciudad grande son los edificios que han crecido fuera
        let afuera = edificios_extramuros(&grande.c, &interior_de(&grande.c, &viejo.celdas).unwrap());
        assert!(afuera > 0, "k{k}");
        if contiene_a(&nuevo.celdas, &viejo.celdas) {
            envuelven += 1;
            assert!(nuevo.area_encerrada > viejo.area_encerrada, "k{k}: el área encerrada nunca decrece");
            assert!(!contiene_a(&viejo.celdas, &nuevo.celdas), "k{k}: el viejo no envuelve al nuevo");
            assert!(afuera >= 6, "k{k}: una ampliación que envuelve deja ≥ 6 edificios fuera del viejo (hay {afuera})");
        }
    }
    assert!(envuelven >= 4, "pocas ampliaciones que envuelvan: {envuelven}");
}

#[test]
fn el_extramuros_de_un_anillo_son_los_edificios_urbanos_que_deja_fuera() {
    for (k, s) in ciudades().iter().enumerate() {
        let t = trazar(&s.c, 2);
        let interior = interior_de(&s.c, &t.celdas).unwrap();
        assert_eq!(edificios_extramuros(&s.c, &interior), t.fuera.len(), "ciudad {k}");
    }
}

#[test]
fn el_trazo_es_rapido() {
    let s = ciudad_de(130, Perfil::Nucleos, 150);
    let t0 = std::time::Instant::now();
    let t = trazar(&s.c, 3);
    let dt = t0.elapsed();
    eprintln!("anillo de {} celdas en {dt:?}", t.celdas.len());
    let tope = if cfg!(debug_assertions) { 5.0 } else { 0.5 };
    assert!(dt.as_secs_f64() < tope, "demasiado lento: {dt:?}");
}
