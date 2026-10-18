//! Pruebas del trazado urbano (Docs/Mechanics/Trazado_Urbano.md): determinismo, invariantes de T §13, llenado de una ciudad,
//! perfiles de T §8, afueras, recintos y rendimiento. El «catálogo» es una tabla de pruebas.

use std::collections::{HashSet, VecDeque};
use std::time::Instant;
use trazado::*;

const CENTRO: TipoPieza = TipoPieza { tipo: 1, huella: Huella::new(4, 4) };
const VIVIENDA: TipoPieza = TipoPieza { tipo: 2, huella: Huella::new(1, 1) };
const GRANJA: TipoPieza = TipoPieza { tipo: 3, huella: Huella::new(2, 2) };

#[derive(Clone, Copy)]
struct Tipo {
    tipo: u16,
    cat: Categoria,
    rol: Rol,
    huella: Huella,
    rota: bool,
}
const fn t(tipo: u16, cat: Categoria, w: i32, h: i32) -> Tipo {
    Tipo { tipo, cat, rol: Rol::Normal, huella: Huella::new(w, h), rota: true }
}
const T_VIVIENDA: Tipo = t(2, Categoria::Residencial, 1, 1);
const T_FUNDICION: Tipo = t(5, Categoria::Industria, 2, 2);
const T_BARRACON: Tipo = t(6, Categoria::Militar, 2, 2);
const T_CARPINTERIA: Tipo = t(7, Categoria::Militar, 4, 2);
const T_ALMACEN: Tipo = t(8, Categoria::Generico, 2, 1);
const T_GRANERO: Tipo = t(9, Categoria::Generico, 4, 2);
const T_TABERNA: Tipo = t(11, Categoria::Generico, 2, 2);
const T_MERCADO: Tipo = Tipo { tipo: 12, cat: Categoria::Mercado, rol: Rol::AnclaPrimaria, huella: Huella::new(3, 2), rota: true };
const T_PUESTO: Tipo = t(13, Categoria::Mercado, 1, 2);
const T_GRANJA: Tipo = Tipo { tipo: 3, cat: Categoria::Afueras, rol: Rol::Normal, huella: Huella::new(2, 2), rota: false };
const T_CORRAL: Tipo = Tipo { tipo: 4, cat: Categoria::Afueras, rol: Rol::Normal, huella: Huella::new(4, 3), rota: true };

const SATURACION_RESIDENCIAL: [OpcionAncla; 3] = [
    OpcionAncla { tipo: 20, huella: Huella::new(2, 2) },
    OpcionAncla { tipo: 21, huella: Huella::new(1, 1) },
    OpcionAncla { tipo: 22, huella: Huella::new(3, 2) },
];
const SATURACION_MILITAR: [OpcionAncla; 1] = [OpcionAncla { tipo: 23, huella: Huella::new(2, 2) }];
const SATURACION_INDUSTRIA: [OpcionAncla; 1] = [OpcionAncla { tipo: 24, huella: Huella::new(2, 2) }];

fn opciones(c: Categoria) -> &'static [OpcionAncla] {
    match c {
        Categoria::Residencial => &SATURACION_RESIDENCIAL,
        Categoria::Militar => &SATURACION_MILITAR,
        Categoria::Industria => &SATURACION_INDUSTRIA,
        _ => &[],
    }
}

/// Ciudad en construcción: lleva el contador de ids y el nivel de asentamiento.
struct Sim {
    c: Ciudad,
    sig_id: u32,
    perfil: Perfil,
    nivel: u8,
    fallos: u32,
}

impl Sim {
    fn nueva(semilla: u64, perfil: Perfil, nivel: u8) -> Sim {
        let e = EntradaFundacion { semilla, params: Parametros::default(), perfil, primer_id: 1, centro: CENTRO, vivienda: VIVIENDA, granja: GRANJA };
        let c = Ciudad::fundar(&e);
        let sig_id = 1 + c.piezas().len() as u32;
        Sim { c, sig_id, perfil, nivel, fallos: 0 }
    }

    fn radio(&self) -> i32 {
        radio_ciudad(self.c.params(), self.c.piezas().len() as u32, self.nivel)
    }

    fn pedido(&self, t: Tipo, id: u32) -> Pedido {
        Pedido {
            id,
            tipo: t.tipo,
            categoria: t.cat,
            rol: t.rol,
            huella: t.huella,
            permitir_rotacion: t.rota,
            perfil: self.perfil,
            radio_ul: self.radio(),
        }
    }

    /// Lo que haría la auto-construcción: garantizar ancla, pedir sitio y colocar.
    fn construir(&mut self, t: Tipo) -> Option<u32> {
        let id = self.sig_id;
        let ped = self.pedido(t, id);
        if ped.rol == Rol::Normal && t.cat.tiene_barrio() {
            if let Ancla::Creada(_) = self.c.asegurar_ancla_para(&ped, self.sig_id, self.nivel, opciones(t.cat)) {
                self.sig_id += 1;
            }
        }
        let ped = Pedido { id: self.sig_id, ..self.pedido(t, self.sig_id) };
        match self.c.colocar_siguiente(&ped) {
            Some(_) => {
                self.sig_id += 1;
                Some(ped.id)
            }
            None => {
                self.fallos += 1;
                None
            }
        }
    }
}

/// Una ciudad de ~`n` edificios con una mezcla de todas las ramas de colocación.
fn ciudad_de(semilla: u64, perfil: Perfil, n: usize) -> Sim {
    let mut s = Sim::nueva(semilla, perfil, 5);
    let ciclo = [
        T_VIVIENDA, T_VIVIENDA, T_VIVIENDA, T_ALMACEN, T_VIVIENDA, T_FUNDICION, T_VIVIENDA, T_BARRACON, T_VIVIENDA, T_GRANERO, T_VIVIENDA,
        T_TABERNA, T_CARPINTERIA, T_VIVIENDA, T_GRANJA, T_VIVIENDA, T_PUESTO, T_VIVIENDA, T_CORRAL, T_VIVIENDA,
    ];
    s.construir(T_MERCADO);
    let mut i = 0;
    while s.c.piezas().len() < n && i < 3 * n {
        s.construir(ciclo[i % ciclo.len()]);
        i += 1;
    }
    s
}

// ---- verificación de invariantes (T §13) ----

fn conexa(c: &Ciudad) -> bool {
    let red: HashSet<Celda> = c.calles().into_iter().chain(c.caminos()).collect();
    let Some(&inicio) = red.iter().min() else { return true };
    let mut vistas = HashSet::from([inicio]);
    let mut cola = VecDeque::from([inicio]);
    while let Some(Celda { col, row }) = cola.pop_front() {
        for (dc, dr) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let n = Celda::new(col + dc, row + dr);
            if red.contains(&n) && vistas.insert(n) {
                cola.push_back(n);
            }
        }
    }
    vistas.len() == red.len()
}

fn verificar(c: &Ciudad, radio_ul: i32) {
    let prm = *c.params();
    let piezas = c.piezas();
    // nunca se pisan entre sí, ni pisan calles, caminos o muros
    for (k, a) in piezas.iter().enumerate() {
        for b in &piezas[k + 1..] {
            assert!(!a.rect.solapa(&b.rect), "{} y {} se solapan: {:?} {:?}", a.id, b.id, a.rect, b.rect);
        }
        for cel in a.rect.celdas() {
            assert!(!c.es_calle(cel) && !c.es_camino(cel), "la pieza {} pisa la red en {cel:?}", a.id);
            for r in c.recintos() {
                assert!(!r.muros.contains(&cel), "la pieza {} pisa un muro", a.id);
            }
        }
    }
    // la red es un único componente conexo
    assert!(conexa(c), "la red de calles tiene islas");
    // todo sitio tiene salida a la red
    for p in piezas.iter().filter(|p| p.rol != Rol::Centro) {
        let frente = p.rect.adyacentes().into_iter().any(|cel| c.es_calle(cel) || c.es_camino(cel));
        assert!(frente, "la pieza {} ({:?}) no tiene frente de calle", p.id, p.rect);
    }
    // separación mínima entre anclas reales
    let anclas: Vec<&Pieza> = piezas.iter().filter(|p| p.rol.es_ancla()).collect();
    for (k, a) in anclas.iter().enumerate() {
        for b in &anclas[k + 1..] {
            assert!(a.rect.hueco(&b.rect) >= prm.separacion_seguridad_anclas, "anclas {} y {} demasiado juntas", a.id, b.id);
        }
    }
    // dentro del disco urbano; las afueras, fuera de las afueras mínimas y dentro de su banda
    let banda = prm.radio_afueras_min_ul.max(0) as i64 * 1000;
    for p in piezas {
        let c0 = p.rect.centro();
        let d2 = c0.x * c0.x + c0.y * c0.y;
        if p.categoria == Categoria::Afueras {
            let (x0, x1) = (p.rect.col as i64 * 3000, p.rect.fin_col() as i64 * 3000);
            let (y0, y1) = (p.rect.row as i64 * 3000, p.rect.fin_row() as i64 * 3000);
            let (dx, dy) = (x0.max(-x1).max(0), y0.max(-y1).max(0));
            assert!(dx * dx + dy * dy >= banda * banda, "afuera {} dentro de las afueras mínimas", p.id);
        } else {
            let r = radio_ul as i64 * 1000;
            assert!(d2 <= r * r, "la pieza {} ({:?}) se sale del disco de {radio_ul} u.l.", p.id, p.rect);
        }
    }
}

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
