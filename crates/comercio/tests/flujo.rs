//! Pruebas de extremo a extremo de las reglas puras del comercio: un trueque entero (proponer → aceptar → reparto → viaje con peaje →
//! entrega → abono → cierre), los invariantes de §14 (conservación de recursos y de oro, estados válidos), el determinismo y los casos
//! límite de §15. El núcleo (fase 3B) hará lo mismo con eventos reales; aquí se ejecuta a mano, en el orden que él seguirá.

use asentamiento::datos::UNO;
use asentamiento::{Almacen, Recurso};
use base::{Instante, Mil, Punto, Rng};
use comercio::caravana::{candidatas_de_reparto, transicion_valida, Animal, Carro, Caravana, EstadoCaravana, Llegada, TipoCarro};
use comercio::entrega::{entregar, DatosEntrega};
use comercio::precio::{ContextoComision, TablaPrecios};
use comercio::reparto::{asignar_origen, lados_pendientes, origenes, EntornoReparto};
use comercio::reputacion::Reputacion;
use comercio::trueque::{AcuerdoTrueque, EstadoTrueque, Vencimiento};
use comercio::viaje::{longitud_ruta, peajes_de_paradas, plan_uniforme, Peaje};
use comercio::{Carga, Lado, Reglas};
use Recurso::*;

const MIN: i64 = 60_000;
const A: u32 = 10;
const B: u32 = 20;
const CIUDAD: u32 = 30;

fn p(x: i64, y: i64) -> Punto {
    Punto::new(x * 1000, y * 1000)
}

fn pos(id: u32) -> Punto {
    match id {
        A => p(0, 0),
        B => p(100, 0),
        _ => p(50, 0),
    }
}

fn buey(id: u64, origen: u32) -> Caravana {
    let mut c = Caravana::nueva(id, origen, pos(origen));
    c.carros = vec![Carro { tipo: TipoCarro::Basico, animal: Some(Animal::Buey) }];
    c
}

fn almacen(items: &[(Recurso, Mil)]) -> Almacen {
    let mut a = Almacen::nuevo(10_000_000);
    for &(r, n) in items {
        a.poner(r, n);
    }
    a
}

/// Todo lo que hay de un recurso en el mundo del test: tres almacenes y lo que va en las caravanas.
fn mundo(r: Recurso, almacenes: &[&Almacen], caravanas: &[&Caravana]) -> Mil {
    almacenes.iter().map(|a| a.cantidad(r)).sum::<Mil>() + caravanas.iter().map(|c| c.contenido.get(&r).copied().unwrap_or(0)).sum::<Mil>()
}

struct Resultado {
    a: Almacen,
    b: Almacen,
    ciudad: Almacen,
    acuerdo: AcuerdoTrueque,
    comisiones: Mil,
    viajes: usize,
    cierres: usize,
}

/// Un trueque entre dos Facciones (1 y 2) con una ciudad ajena (3) cruzada a mitad de camino en el viaje de A hacia B. Como el peaje se
/// lleva el 2 % de lo que va de A a B, la línea de A queda corta y hace falta otro viaje (§4.4): se reparte, se viaja y se abona hasta que
/// se salda, con las mismas dos caravanas (se reutilizan, no se reconstruyen).
fn trueque_completo() -> Resultado {
    trueque(true)
}

/// El mismo trueque; `con_peaje` dice si el viaje de A hacia B cruza la ciudad ajena.
fn trueque(con_peaje: bool) -> Resultado {
    let r = Reglas::cargar();
    let mut alm_a = almacen(&[(Madera, 300_000)]);
    let mut alm_b = almacen(&[(Cobre, 100_000)]);
    let mut ciudad = almacen(&[]);
    let mut acuerdo = AcuerdoTrueque::proponer(&r.trueque, 1, A, B, &[(Madera, 100_000)], &[(Cobre, 30_000)], 0, true).unwrap();
    let mut ahora = 10 * MIN;
    acuerdo.aceptar(&r.trueque, ahora, true).unwrap();
    let facciones = (1, 2);
    let mut caravanas = vec![buey(1, A), buey(2, B)];
    let (mut comisiones, mut viajes, mut cierres) = (0, 0, 0);
    let distancia = |a: u32, b: u32| pos(a).dist(pos(b));

    while acuerdo.estado == EstadoTrueque::Activo && viajes < 10 {
        // Reparto: qué asigno ahora.
        let existe = |_| true;
        let en_ruta = |_, _: Lado| false;
        let e = EntornoReparto { ahora, distancia: &distancia, existe: &existe, en_ruta: &en_ruta };
        let pendientes = lados_pendientes(&r.asignacion, std::slice::from_ref(&acuerdo), &e);
        let mut salidas = Vec::new();
        for origen in origenes(&pendientes) {
            let alm = if origen == A { &mut alm_a } else { &mut alm_b };
            let cands = candidatas_de_reparto(&caravanas, &r.flota, origen, UNO);
            let mut trazar = |o: u32, _d: u32| {
                let ruta = if o == A { vec![pos(A), pos(CIUDAD), pos(B)] } else { vec![pos(B), pos(A)] };
                let peajes = if o == A && con_peaje { peajes_de_paradas(&[(CIUDAD, longitud_ruta(&ruta[..2]))], longitud_ruta(&ruta)) } else { vec![] };
                Some((ruta, peajes))
            };
            salidas.extend(asignar_origen(origen, &pendientes, &cands, alm, &mut trazar));
        }
        assert!(!salidas.is_empty(), "mientras quede algo pendiente y haya caravanas libres, el reparto envía");
        for s in salidas {
            let (ruta, peajes) = s.trazado;
            let c = caravanas.iter_mut().find(|c| c.id == s.caravana).unwrap();
            let plan = plan_uniforme(&ruta, ahora, c.velocidad(&r.flota), 1).unwrap();
            c.salir_por_reparto(s.destino, s.carga, s.acuerdo, s.lado, ruta, peajes, plan).unwrap();
            viajes += 1;
        }
        // Conservación: lo que salió de los almacenes va en las caravanas.
        let (c0, c1) = (&caravanas[0], &caravanas[1]);
        assert_eq!(mundo(Madera, &[&alm_a, &alm_b, &ciudad], &[c0, c1]), 300_000);
        assert_eq!(mundo(Cobre, &[&alm_a, &alm_b, &ciudad], &[c0, c1]), 100_000);

        // Peajes y llegadas, en orden de instante.
        let tasa = r.peaje;
        for c in caravanas.iter_mut().filter(|c| c.estado == EstadoCaravana::EnTransito) {
            let t = c.proximo_peaje();
            for cobro in t.map(|t| c.cobrar_peajes(t, tasa)).unwrap_or_default() {
                assert_eq!(cobro.ciudad, CIUDAD);
                for (rec, n) in cobro.cobrado {
                    ciudad.agregar(rec, n);
                }
            }
        }
        let precios = TablaPrecios::de(&r.precio, [&alm_a, &alm_b, &ciudad]);
        let mut orden: Vec<usize> = (0..caravanas.len()).filter(|&i| caravanas[i].estado == EstadoCaravana::EnTransito).collect();
        orden.sort_by_key(|&i| caravanas[i].llegada_en());
        for i in orden {
            let fin = caravanas[i].llegada_en().unwrap();
            ahora = ahora.max(fin);
            let Llegada::Entrega { destino, contenido, origen_acuerdo } = caravanas[i].llegar(fin).unwrap() else { panic!() };
            let (_, lado) = origen_acuerdo.unwrap();
            let (origen, dest) = if destino == B { (&mut alm_a, &mut alm_b) } else { (&mut alm_b, &mut alm_a) };
            let d = DatosEntrega {
                comision: ContextoComision { mismo_asentamiento: false, misma_faccion: false, factor_comision_externa: UNO, distancia_recta: pos(A).dist(pos(B)) },
                reputacion_origen: 0,
                oro_origen: origen.cantidad(Oro),
                precios: &precios,
            };
            let e = entregar(&r, &contenido, dest, &d);
            comisiones += e.comision;
            if let Some(c) = acuerdo.abonar(lado, &contenido, facciones) {
                assert!(c.entre_facciones);
                cierres += 1;
            }
        }
        // Las caravanas vuelven a casa por la ruta de ida y quedan libres para el siguiente envío.
        for c in caravanas.iter_mut().filter(|c| c.estado == EstadoCaravana::Retornando) {
            let fin = c.llegada_en().unwrap();
            ahora = ahora.max(fin);
            let Llegada::Casa { contenido, .. } = c.llegar(fin).unwrap() else { panic!() };
            assert!(contenido.is_empty(), "el regreso va vacío");
            assert_eq!(c.estado, EstadoCaravana::Disponible);
        }
    }
    Resultado { a: alm_a, b: alm_b, ciudad, acuerdo, comisiones, viajes, cierres }
}

#[test]
fn un_trueque_entero_entre_facciones_se_cumple_y_conserva_los_recursos() {
    let r = trueque(false);
    assert_eq!(r.acuerdo.estado, EstadoTrueque::Cumplido);
    assert_eq!(r.cierres, 1, "el cierre y su recompensa salen una sola vez");
    assert_eq!(r.viajes, 2, "sin peaje, un viaje por lado");
    assert_eq!(r.acuerdo.pendiente(Lado::A), vec![]);
    assert_eq!((r.a.cantidad(Cobre), r.b.cantidad(Madera)), (30_000, 100_000), "todo llegó entero");
    assert_eq!(r.a.cantidad(Madera) + r.b.cantidad(Madera) + r.ciudad.cantidad(Madera), 300_000);
    assert_eq!(r.a.cantidad(Cobre) + r.b.cantidad(Cobre), 100_000);
    // El oro del mundo es exactamente lo acuñado en comisiones.
    assert_eq!(r.a.cantidad(Oro) + r.b.cantidad(Oro) + r.ciudad.cantidad(Oro), r.comisiones);
    assert!(r.comisiones > 0);
}

#[test]
fn con_un_peaje_en_el_camino_la_ultima_unidad_llega_porque_el_peaje_nunca_deja_la_carga_en_cero() {
    // EE-4 + EE-6 y la decisión del autor del 2026-10-24: el peaje cobra como mínimo una unidad, pero nunca deja la carga en cero, y ya no se salda una
    // línea a la que falta menos de una. Ida 1: A→B 100 (peaje 2) y B→A 30; ida 2: A→B 2 (peaje 1, llega 1); ida 3: A→B 1, que pasa sin pagar y
    // salda la línea. Tres viajes de A y uno de B; la ciudad se queda 3. Nada se pierde: lo que no está en A ni en B está en la ciudad.
    let r = trueque_completo();
    assert_eq!((r.acuerdo.estado, r.viajes, r.cierres), (EstadoTrueque::Cumplido, 4, 1));
    assert_eq!(r.acuerdo.pendiente(Lado::A), vec![]);
    assert_eq!((r.b.cantidad(Madera), r.ciudad.cantidad(Madera)), (100_000, 3_000));
    assert_eq!(r.a.cantidad(Madera) + r.b.cantidad(Madera) + r.ciudad.cantidad(Madera), 300_000);
    assert_eq!(r.a.cantidad(Oro) + r.b.cantidad(Oro) + r.ciudad.cantidad(Oro), r.comisiones);
}

#[test]
fn el_mismo_trueque_dos_veces_da_exactamente_lo_mismo() {
    let (x, y) = (trueque_completo(), trueque_completo());
    assert_eq!((x.a, x.b, x.ciudad, x.acuerdo, x.comisiones, x.viajes), (y.a, y.b, y.ciudad, y.acuerdo, y.comisiones, y.viajes));
}

#[test]
fn con_el_peaje_la_linea_queda_corta_y_el_trueque_no_se_salda() {
    // Si el peaje se lleva un 2 %, la línea de 100 de madera solo recibe 98: hace falta otro viaje (§4.4).
    let r = Reglas::cargar();
    let mut a = AcuerdoTrueque::proponer(&r.trueque, 1, A, B, &[(Madera, 100_000)], &[(Cobre, 30_000)], 0, true).unwrap();
    a.aceptar(&r.trueque, 0, true).unwrap();
    let llevaba: Carga = [(Madera, 98_000)].into_iter().collect();
    assert!(a.abonar(Lado::A, &llevaba, (1, 2)).is_none());
    let cobre: Carga = [(Cobre, 30_000)].into_iter().collect();
    assert!(a.abonar(Lado::B, &cobre, (1, 2)).is_none());
    assert_eq!(a.pendiente(Lado::A), vec![(Madera, 2_000)], "queda un 2 % por enviar");
    assert_eq!(a.estado, EstadoTrueque::Activo);
}

#[test]
fn caso_limite_un_casco_vacio_o_sin_animal_no_bloquea_el_reparto() {
    let r = Reglas::cargar();
    let casco = Caravana::nueva(1, A, pos(A));
    let sin_animal = {
        let mut c = Caravana::nueva(2, A, pos(A));
        c.carros = vec![Carro { tipo: TipoCarro::Basico, animal: None }];
        c
    };
    let buena = buey(3, A);
    let caravanas = [casco, sin_animal, buena];
    let c = candidatas_de_reparto(&caravanas, &r.flota, A, UNO);
    assert_eq!(c.iter().map(|x| x.caravana).collect::<Vec<_>>(), vec![3], "el casco vacío y el carro sin animal no ocupan el turno");
}

#[test]
fn caso_limite_un_destino_inalcanzable_no_gasta_la_caravana() {
    let r = Reglas::cargar();
    let mut acuerdo = AcuerdoTrueque::proponer(&r.trueque, 1, A, B, &[(Madera, 100_000)], &[(Cobre, 1_000)], 0, true).unwrap();
    acuerdo.aceptar(&r.trueque, 0, true).unwrap();
    let (d, ex, er) = (|_, _| 1_000, |_| true, |_, _: Lado| false);
    let e = EntornoReparto { ahora: 0, distancia: &d, existe: &ex, en_ruta: &er };
    let pendientes = lados_pendientes(&r.asignacion, &[acuerdo], &e);
    let mut alm = almacen(&[(Madera, 500_000)]);
    let caravanas = [buey(1, A)];
    let cands = candidatas_de_reparto(&caravanas, &r.flota, A, UNO);
    let salidas: Vec<comercio::reparto::Asignacion<()>> = asignar_origen(A, &pendientes, &cands, &mut alm, &mut |_, _| None);
    assert!(salidas.is_empty());
    assert_eq!(alm.cantidad(Madera), 500_000, "la carga se queda en el almacén");
}

#[test]
fn caso_limite_un_expirado_que_se_salda_pasa_a_cumplido_y_reparte_la_recompensa() {
    let r = Reglas::cargar();
    let mut a = AcuerdoTrueque::proponer(&r.trueque, 1, A, B, &[(Madera, 100_000)], &[(Cobre, 30_000)], 0, true).unwrap();
    a.aceptar(&r.trueque, 0, true).unwrap();
    let Some(Vencimiento::Incumplido { reputacion }) = a.vencer(a.expira_en, (1, 2)) else { panic!() };
    assert_eq!(reputacion.len(), 2, "los dos lados deben todo");
    let mut rep = Reputacion::nueva(0);
    for (_, s) in &reputacion {
        rep.aplicar(&r.reputacion, 0, *s);
    }
    assert_eq!(rep.valor_en(0), -16_000_000, "−8 −8 sobre la misma cuenta de prueba");
    // Llegan tarde las dos cargas: se salda, pasa a cumplido y reparte el bonus aunque ya se hubiera penalizado.
    a.abonar(Lado::A, &[(Madera, 100_000)].into_iter().collect(), (1, 2));
    let cierre = a.abonar(Lado::B, &[(Cobre, 30_000)].into_iter().collect(), (1, 2)).unwrap();
    assert!(cierre.tras_vencer);
    assert_eq!(a.estado, EstadoTrueque::Cumplido);
}

#[test]
fn caso_limite_llevar_la_carga_al_propio_origen_no_acuna() {
    let r = Reglas::cargar();
    let mut alm = almacen(&[]);
    let precios = TablaPrecios::de(&r.precio, [&alm]);
    let d = DatosEntrega {
        comision: ContextoComision { mismo_asentamiento: true, misma_faccion: true, factor_comision_externa: UNO, distancia_recta: 0 },
        reputacion_origen: 0,
        oro_origen: 0,
        precios: &precios,
    };
    let e = entregar(&r, &[(Cobre, 100_000)].into_iter().collect(), &mut alm, &d);
    assert_eq!((e.comision, alm.cantidad(Oro)), (0, 0));
}

/// Invariante de §14: una caravana sometida a operaciones al azar solo hace transiciones permitidas, nunca pierde ni crea carga fuera de
/// las vías previstas y el resultado es el mismo con la misma semilla.
fn recorrido_aleatorio(semilla: u64) -> (Vec<EstadoCaravana>, Almacen, Caravana) {
    let r = Reglas::cargar();
    let mut rng = Rng::nuevo(semilla);
    let mut alm = almacen(&[(Madera, 2_000_000), (Oro, 500_000)]);
    let mut c = buey(1, A);
    let mut historia = vec![c.estado];
    let mut ahora: Instante = 0;
    let ruta = vec![pos(A), pos(B)];
    for _ in 0..400 {
        ahora += rng.por_debajo(5 * MIN as u64 + 1) as i64;
        let antes = c.estado;
        let total_antes = alm.cantidad(Madera) + c.contenido.get(&Madera).copied().unwrap_or(0);
        match rng.por_debajo(9) {
            0 => {
                let lanz = comercio::caravana::Lanzamiento {
                    destino: B,
                    carga: [(Madera, 1_000 * (1 + rng.por_debajo(450) as i64))].into_iter().collect(),
                    escolta_nueva: vec![],
                    cupo_escolta: 100,
                    salir_en: (rng.por_debajo(2) == 0).then(|| ahora + 1 + rng.por_debajo(60 * MIN as u64) as i64),
                    factor_capacidad: UNO,
                    ruta: Some(ruta.clone()),
                    peajes: vec![Peaje { ciudad: CIUDAD, progreso: 500_000 }],
                };
                if let Ok(prep) = c.preparar(&r.flota, lanz, &mut alm, ahora) {
                    if prep.estado == EstadoCaravana::EnTransito {
                        c.poner_plan(plan_uniforme(&ruta, ahora, c.velocidad(&r.flota), 1).unwrap()).unwrap();
                    }
                }
            }
            1 => {
                let _ = c.cancelar(&mut alm);
            }
            2 => {
                if let Some(t) = c.prepara_hasta {
                    let plan = plan_uniforme(&ruta, t, c.velocidad(&r.flota), 1).unwrap();
                    let _ = c.salir(ahora.max(t), plan);
                    ahora = ahora.max(t);
                }
            }
            3 => {
                if let Some(t) = c.llegada_en() {
                    ahora = ahora.max(t);
                    if let Ok(Llegada::Casa { contenido, .. }) = c.llegar(ahora) {
                        for (rec, n) in contenido {
                            alm.agregar(rec, n);
                        }
                    } else if c.estado == EstadoCaravana::Retornando {
                        // era una entrega: el contenido salió y se da por entregado (fuera del almacén del test)
                    }
                }
            }
            4 => {
                let _ = c.cobrar_peajes(ahora, r.peaje);
            }
            5 => {
                let _ = c.adjuntar();
            }
            6 => {
                let _ = c.aparcar(CIUDAD, pos(CIUDAD));
            }
            7 => {
                let _ = c.entrar_en_batalla();
            }
            _ => {
                let _ = c.salir_de_batalla();
            }
        }
        let despues = c.estado;
        assert!(antes == despues || transicion_valida(antes, despues), "{antes:?} → {despues:?} no es una transición válida");
        // La carga que se pierde solo puede ser la que se entregó/cobró; el almacén más la caravana nunca crean madera.
        let total_despues = alm.cantidad(Madera) + c.contenido.get(&Madera).copied().unwrap_or(0);
        assert!(total_despues <= total_antes, "ninguna operación crea mercancía de la nada");
        assert!(c.contenido.values().all(|&n| n > 0), "el contenido no guarda ceros");
        if c.estado == EstadoCaravana::Preparando {
            assert!(c.prepara_hasta.is_some());
        } else {
            assert!(c.prepara_hasta.is_none());
        }
        if historia.last() != Some(&c.estado) {
            historia.push(c.estado);
        }
    }
    (historia, alm, c)
}

#[test]
fn las_transiciones_de_una_caravana_siempre_son_validas_y_deterministas() {
    for semilla in 0..40 {
        let (h1, a1, c1) = recorrido_aleatorio(semilla);
        let (h2, a2, c2) = recorrido_aleatorio(semilla);
        assert_eq!((&h1, &a1, &c1), (&h2, &a2, &c2), "semilla {semilla}");
        assert!(h1.windows(2).all(|w| transicion_valida(w[0], w[1])));
    }
    // El recorrido aleatorio llega a usar la mayoría de los estados.
    let vistos: std::collections::BTreeSet<String> = (0..40).flat_map(|s| recorrido_aleatorio(s).0).map(|e| format!("{e:?}")).collect();
    assert!(vistos.len() >= 5, "estados alcanzados: {vistos:?}");
}
