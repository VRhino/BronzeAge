//! Pruebas del trazado urbano (Docs/Mechanics/Trazado_Urbano.md): determinismo, invariantes de T §13, llenado de una ciudad,
//! perfiles de T §8, afueras, recintos y rendimiento. El «catálogo» es una tabla de pruebas.

mod comun;

use comun::*;
use std::collections::HashSet;
use std::time::Instant;
use trazado::*;

// ---- pruebas ----

#[test]
fn fundacion_deja_centro_viviendas_y_granja() {
    let s = Sim::nueva(7, Perfil::Nucleos, 1);
    let p = s.c.piezas();
    assert_eq!(p[0].rol, Rol::Centro);
    assert_eq!(p[0].rect, Rect::new(-2, -2, 4, 4));
    assert_eq!(p[0].posicion(), base::Punto::new(0, 0));
    assert_eq!(p.iter().filter(|x| x.categoria == Categoria::Residencial && x.rol == Rol::Normal).count(), 3);
    assert_eq!(p.iter().filter(|x| x.categoria == Categoria::Afueras).count(), 1);
    verificar(&s.c, 30 + 100);
    // el anillo del Centro Urbano es calle
    for cel in Rect::new(-2, -2, 4, 4).anillo(1) {
        assert!(s.c.es_calle(cel), "falta calle en el anillo {cel:?}");
    }
}

#[test]
fn determinista_dos_ejecuciones_y_semillas_distintas() {
    for perfil in Perfil::TODOS {
        let a = ciudad_de(42, perfil, 80);
        let b = ciudad_de(42, perfil, 80);
        assert_eq!(a.c, b.c, "mismo input, distinta ciudad ({perfil:?})");
        assert_eq!(a.c.calles(), b.c.calles());
    }
    let a = ciudad_de(42, Perfil::Nucleos, 80);
    let b = ciudad_de(43, Perfil::Nucleos, 80);
    assert_ne!(a.c.piezas(), b.c.piezas(), "otra semilla debería dar otra ciudad");
}

#[test]
fn red_incremental_igual_a_reconstruida_desde_las_piezas() {
    for semilla in [1u64, 2, 3, 99] {
        let s = ciudad_de(semilla, Perfil::TODOS[(semilla % 4) as usize], 120);
        let c2 = Ciudad::desde_partes(semilla, *s.c.params(), LAYOUT_VERSION, s.c.piezas().to_vec(), s.c.recintos().to_vec()).unwrap();
        assert_eq!(s.c.calles(), c2.calles(), "calles distintas (semilla {semilla})");
        assert_eq!(s.c.caminos(), c2.caminos(), "caminos distintos (semilla {semilla})");
    }
}

#[test]
fn una_version_de_geometria_distinta_se_rechaza() {
    let s = Sim::nueva(1, Perfil::Nucleos, 1);
    let r = Ciudad::desde_partes(1, Parametros::default(), LAYOUT_VERSION - 1, s.c.piezas().to_vec(), vec![]);
    assert_eq!(r.unwrap_err(), ErrorTrazado::VersionDeGeometria(LAYOUT_VERSION - 1));
}

#[test]
fn ciudad_de_150_edificios_se_llena_sin_solaparse_ni_salirse_del_disco() {
    for (semilla, perfil) in [(5u64, Perfil::Nucleos), (6, Perfil::Caminera), (7, Perfil::Compacta), (8, Perfil::Gremial)] {
        let s = ciudad_de(semilla, perfil, 150);
        assert!(s.c.piezas().len() >= 120, "{perfil:?}: solo {} piezas ({} fallos)", s.c.piezas().len(), s.fallos);
        verificar(&s.c, s.radio());
        // hay anclas de saturación además del Centro y el Mercado
        assert!(s.c.piezas().iter().filter(|p| p.rol == Rol::AnclaSaturacion).count() >= 1, "{perfil:?}: sin anclas de saturación");
    }
}

#[test]
fn los_cuatro_perfiles_dan_ordenes_distintos() {
    let ciudades: Vec<Vec<Rect>> =
        Perfil::TODOS.iter().map(|&p| ciudad_de(11, p, 70).c.piezas().iter().map(|x| x.rect).collect()).collect();
    for i in 0..4 {
        for j in i + 1..4 {
            assert_ne!(ciudades[i], ciudades[j], "{:?} y {:?} dan la misma ciudad", Perfil::TODOS[i], Perfil::TODOS[j]);
        }
    }
}

#[test]
fn el_perfil_solo_afecta_a_colocaciones_nuevas() {
    let mut s = ciudad_de(3, Perfil::Nucleos, 40);
    let antes: Vec<Rect> = s.c.piezas().iter().map(|p| p.rect).collect();
    s.perfil = Perfil::Compacta;
    for _ in 0..15 {
        s.construir(T_VIVIENDA);
    }
    let despues: Vec<Rect> = s.c.piezas().iter().take(antes.len()).map(|p| p.rect).collect();
    assert_eq!(antes, despues, "cambiar de perfil recolocó edificios");
    verificar(&s.c, s.radio());
}

#[test]
fn perfil_por_tradicion_es_estable_y_cubre_los_cuatro() {
    assert_eq!(Perfil::por_tradicion(5), Perfil::por_tradicion(5));
    let vistos: HashSet<Perfil> = (0..200u64).map(Perfil::por_tradicion).collect();
    assert_eq!(vistos.len(), 4);
}

#[test]
fn granjas_y_corrales_van_fuera_de_las_afueras_minimas_con_camino() {
    let mut s = ciudad_de(21, Perfil::Nucleos, 60);
    for _ in 0..6 {
        s.construir(T_GRANJA);
        s.construir(T_CORRAL);
    }
    let afueras: Vec<&Pieza> = s.c.piezas().iter().filter(|p| p.categoria == Categoria::Afueras).collect();
    assert!(afueras.len() >= 10, "pocas granjas/corrales: {}", afueras.len());
    assert!(!s.c.caminos().is_empty(), "las afueras se conectan por caminos rurales");
    verificar(&s.c, s.radio());
}

#[test]
fn los_muros_bloquean_y_las_puertas_se_cruzan() {
    let mut s = Sim::nueva(9, Perfil::Nucleos, 3);
    // recinto cuadrado de radio 10 con una puerta al norte y otra al sur
    let (puerta_n, puerta_s) = (Celda::new(0, -10), Celda::new(0, 10));
    let mut muros = Vec::new();
    for i in -10..=10 {
        for cel in [Celda::new(i, -10), Celda::new(i, 10), Celda::new(-10, i), Celda::new(10, i)] {
            if cel != puerta_n && cel != puerta_s {
                muros.push(cel);
            }
        }
    }
    muros.sort();
    muros.dedup();
    s.c.set_recintos(vec![Recinto { muros: muros.clone(), puertas: vec![puerta_n, puerta_s], completo: true }]);
    assert!(muros.iter().all(|&m| s.c.esta_ocupada(m)), "los muros ocupan suelo");
    for pu in [puerta_n, puerta_s] {
        // una puerta no ocupa suelo: o está libre o ya es red (nunca la bloquea el recinto)
        assert!(!s.c.esta_ocupada(pu) || s.c.es_calle(pu) || s.c.es_camino(pu), "las puertas son transitables");
    }
    for _ in 0..14 {
        s.construir(T_VIVIENDA);
    }
    for _ in 0..4 {
        s.construir(T_GRANJA);
    }
    verificar(&s.c, s.radio());
    // lo urbano, entero dentro del recinto; las granjas, fuera, y su camino cruza una puerta
    for p in s.c.piezas().iter().filter(|p| p.categoria != Categoria::Afueras) {
        assert!(p.rect.col > -10 && p.rect.fin_col() <= 10 && p.rect.row > -10 && p.rect.fin_row() <= 10, "{:?} fuera del recinto", p.rect);
    }
    let granjas = s.c.piezas().iter().filter(|p| p.categoria == Categoria::Afueras).count();
    assert!(granjas >= 2, "granjas colocadas: {granjas}");
    assert!(
        s.c.es_camino(puerta_n) || s.c.es_camino(puerta_s) || s.c.es_calle(puerta_n) || s.c.es_calle(puerta_s),
        "el camino de las granjas debe cruzar una puerta"
    );
}

#[test]
fn con_un_recinto_completo_lo_urbano_se_prefiere_dentro() {
    let mut s = Sim::nueva(4, Perfil::Caminera, 5);
    let mut muros = Vec::new();
    for i in -9..=9 {
        muros.extend([Celda::new(i, -9), Celda::new(i, 9), Celda::new(-9, i), Celda::new(9, i)]);
    }
    muros.sort();
    muros.dedup();
    s.c.set_recintos(vec![Recinto { muros, puertas: vec![], completo: true }]);
    for _ in 0..40 {
        s.construir(T_ALMACEN);
    }
    for p in s.c.piezas().iter().filter(|p| p.categoria != Categoria::Afueras) {
        assert!(p.rect.col > -9 && p.rect.fin_col() <= 9 && p.rect.row > -9 && p.rect.fin_row() <= 9);
    }
}

#[test]
fn reubicar_por_tamano_busca_un_sitio_con_la_huella_nueva() {
    let s = ciudad_de(13, Perfil::Nucleos, 40);
    let granja = s.c.piezas().iter().find(|p| p.categoria == Categoria::Afueras).unwrap().clone();
    let nuevo = s.c.reubicar_por_tamano(granja.id, Huella::new(4, 3), Perfil::Nucleos, s.radio()).expect("cabe en algún sitio");
    assert_eq!((nuevo.rect.ancho, nuevo.rect.alto), (4, 3));
    let mut c = s.c.clone();
    c.reubicar(granja.id, nuevo);
    verificar(&c, s.radio());
    assert_eq!(c.piezas().iter().position(|p| p.id == granja.id), s.c.piezas().iter().position(|p| p.id == granja.id));
}

#[test]
fn colocar_rechaza_solapes_ids_repetidos_y_fuera_de_ventana() {
    let mut s = Sim::nueva(2, Perfil::Nucleos, 1);
    let ped = s.pedido(T_VIVIENDA, 500);
    assert!(matches!(s.c.colocar(&ped, Colocacion { rect: Rect::new(0, 0, 1, 1), rotado: false }), Err(ErrorTrazado::Ocupada(_))));
    assert_eq!(s.c.colocar(&ped, Colocacion { rect: Rect::new(1000, 0, 1, 1), rotado: false }), Err(ErrorTrazado::FueraDeVentana));
    let repetido = Pedido { id: 1, ..ped };
    assert_eq!(s.c.colocar(&repetido, Colocacion { rect: Rect::new(30, 30, 1, 1), rotado: false }), Err(ErrorTrazado::IdRepetido(1)));
}

#[test]
fn el_gate_de_anclas_exige_nivel_2() {
    let mut s = Sim::nueva(31, Perfil::Nucleos, 1);
    let industria = s.pedido(T_FUNDICION, 900);
    assert_eq!(s.c.asegurar_ancla_para(&industria, 901, 1, &SATURACION_INDUSTRIA), Ancla::Ninguna);
    assert!(s.c.siguiente_sitio(&industria).is_none(), "sin ancla de industria no hay sitio");
    s.nivel = 2;
    let r = s.c.asegurar_ancla_para(&industria, 901, 2, &SATURACION_INDUSTRIA);
    assert_eq!(r, Ancla::Creada(901));
    assert!(s.c.siguiente_sitio(&industria).is_some());
}

#[test]
fn fusionar_celdas_agrupa_tiradas_horizontales() {
    let celdas = [Celda::new(1, 0), Celda::new(0, 0), Celda::new(2, 0), Celda::new(5, 0), Celda::new(0, 1), Celda::new(-1, 1)];
    let v = fusionar_celdas(&celdas);
    assert_eq!(
        v,
        vec![
            RectLocal { x: 0, y: 0, ancho: 9, alto: 3 },
            RectLocal { x: 15, y: 0, ancho: 3, alto: 3 },
            RectLocal { x: -3, y: 3, ancho: 6, alto: 3 },
        ]
    );
}

#[test]
fn rect_y_posicion_son_inversos_para_lados_pares_e_impares() {
    for (w, h) in [(1, 1), (2, 3), (4, 4), (3, 2), (6, 6)] {
        for (col, row) in [(-5, 7), (0, 0), (12, -3)] {
            let r = Rect::new(col, row, w, h);
            assert_eq!(Rect::desde_centro(r.centro(), Huella::new(w, h)), r);
        }
    }
}

#[test]
fn radio_de_ciudad_crece_por_edificio_y_topa_por_nivel() {
    let p = Parametros::default();
    assert_eq!(radio_ciudad(&p, 0, 1), 30);
    assert_eq!(radio_ciudad(&p, 2, 1), 40);
    assert_eq!(radio_ciudad(&p, 100, 1), 60);
    assert_eq!(radio_ciudad(&p, 100, 3), 120);
    assert_eq!(radio_ciudad(&p, 1000, 5), 180);
}

#[test]
fn rendimiento_200_edificios() {
    let t0 = Instant::now();
    let s = ciudad_de(77, Perfil::Nucleos, 200);
    let dt = t0.elapsed();
    eprintln!("{} piezas en {dt:?} ({} fallos)", s.c.piezas().len(), s.fallos);
    assert!(s.c.piezas().len() >= 150);
    let tope = if cfg!(debug_assertions) { 60.0 } else { 1.0 };
    assert!(dt.as_secs_f64() < tope, "demasiado lento: {dt:?} (tope {tope} s)");
}

#[test]
fn quitar_una_pieza_deja_la_ciudad_como_si_no_se_hubiera_colocado() {
    let mut s = ciudad_de(5, Perfil::Nucleos, 30);
    let antes = s.c.clone();
    let id = s.construir(T_VIVIENDA).expect("cabe");
    assert!(s.c.pieza(id).is_some() && s.c != antes);
    assert!(s.c.quitar(id));
    assert!(s.c.pieza(id).is_none());
    assert_eq!(s.c.piezas(), antes.piezas());
    assert_eq!((s.c.calles(), s.c.caminos()), (antes.calles(), antes.caminos()), "la red se recalcula sin la pieza");
    assert!(!s.c.quitar(id), "ya no existe");
    verificar(&s.c, s.radio());
}
