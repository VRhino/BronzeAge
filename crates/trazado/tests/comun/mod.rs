//! Ciudad de pruebas compartida por los tests de integración: un «catálogo» en una tabla, un simulador de auto-construcción y las
//! invariantes de T §13.
#![allow(dead_code)]

use std::collections::{HashSet, VecDeque};
use trazado::*;

pub const CENTRO: TipoPieza = TipoPieza { tipo: 1, huella: Huella::new(4, 4) };
pub const VIVIENDA: TipoPieza = TipoPieza { tipo: 2, huella: Huella::new(1, 1) };
pub const GRANJA: TipoPieza = TipoPieza { tipo: 3, huella: Huella::new(2, 2) };

#[derive(Clone, Copy)]
pub struct Tipo {
    pub tipo: u16,
    pub cat: Categoria,
    pub rol: Rol,
    pub huella: Huella,
    pub rota: bool,
}
pub const fn t(tipo: u16, cat: Categoria, w: i32, h: i32) -> Tipo {
    Tipo { tipo, cat, rol: Rol::Normal, huella: Huella::new(w, h), rota: true }
}
pub const T_VIVIENDA: Tipo = t(2, Categoria::Residencial, 1, 1);
pub const T_FUNDICION: Tipo = t(5, Categoria::Industria, 2, 2);
pub const T_BARRACON: Tipo = t(6, Categoria::Militar, 2, 2);
pub const T_CARPINTERIA: Tipo = t(7, Categoria::Militar, 4, 2);
pub const T_ALMACEN: Tipo = t(8, Categoria::Generico, 2, 1);
pub const T_GRANERO: Tipo = t(9, Categoria::Generico, 4, 2);
pub const T_TABERNA: Tipo = t(11, Categoria::Generico, 2, 2);
pub const T_MERCADO: Tipo = Tipo { tipo: 12, cat: Categoria::Mercado, rol: Rol::AnclaPrimaria, huella: Huella::new(3, 2), rota: true };
pub const T_PUESTO: Tipo = t(13, Categoria::Mercado, 1, 2);
pub const T_GRANJA: Tipo = Tipo { tipo: 3, cat: Categoria::Afueras, rol: Rol::Normal, huella: Huella::new(2, 2), rota: false };
pub const T_CORRAL: Tipo = Tipo { tipo: 4, cat: Categoria::Afueras, rol: Rol::Normal, huella: Huella::new(4, 3), rota: true };

pub const SATURACION_RESIDENCIAL: [OpcionAncla; 3] = [
    OpcionAncla { tipo: 20, huella: Huella::new(2, 2) },
    OpcionAncla { tipo: 21, huella: Huella::new(1, 1) },
    OpcionAncla { tipo: 22, huella: Huella::new(3, 2) },
];
pub const SATURACION_MILITAR: [OpcionAncla; 1] = [OpcionAncla { tipo: 23, huella: Huella::new(2, 2) }];
pub const SATURACION_INDUSTRIA: [OpcionAncla; 1] = [OpcionAncla { tipo: 24, huella: Huella::new(2, 2) }];

pub fn opciones(c: Categoria) -> &'static [OpcionAncla] {
    match c {
        Categoria::Residencial => &SATURACION_RESIDENCIAL,
        Categoria::Militar => &SATURACION_MILITAR,
        Categoria::Industria => &SATURACION_INDUSTRIA,
        _ => &[],
    }
}

/// Ciudad en construcción: lleva el contador de ids y el nivel de asentamiento.
pub struct Sim {
    pub c: Ciudad,
    pub sig_id: u32,
    pub perfil: Perfil,
    pub nivel: u8,
    pub fallos: u32,
}

impl Sim {
    pub fn nueva(semilla: u64, perfil: Perfil, nivel: u8) -> Sim {
        let e = EntradaFundacion { semilla, params: Parametros::default(), perfil, primer_id: 1, centro: CENTRO, vivienda: VIVIENDA, granja: GRANJA };
        let c = Ciudad::fundar(&e);
        let sig_id = 1 + c.piezas().len() as u32;
        Sim { c, sig_id, perfil, nivel, fallos: 0 }
    }

    pub fn radio(&self) -> i32 {
        radio_ciudad(self.c.params(), self.c.piezas().len() as u32, self.nivel)
    }

    pub fn pedido(&self, t: Tipo, id: u32) -> Pedido {
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
    pub fn construir(&mut self, t: Tipo) -> Option<u32> {
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
pub fn ciudad_de(semilla: u64, perfil: Perfil, n: usize) -> Sim {
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

pub fn conexa(c: &Ciudad) -> bool {
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

pub fn verificar(c: &Ciudad, radio_ul: i32) {
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
    // los genéricos, dentro del disco urbano; las afueras, fuera de las afueras mínimas (anclas y satélites no miran el disco: T §6.5, §7.2)
    let banda = prm.radio_afueras_min_ul.max(0) as i64 * 1000;
    for p in piezas {
        let c0 = p.rect.centro();
        let d2 = c0.x * c0.x + c0.y * c0.y;
        if p.categoria == Categoria::Afueras {
            let (x0, x1) = (p.rect.col as i64 * 3000, p.rect.fin_col() as i64 * 3000);
            let (y0, y1) = (p.rect.row as i64 * 3000, p.rect.fin_row() as i64 * 3000);
            let (dx, dy) = (x0.max(-x1).max(0), y0.max(-y1).max(0));
            assert!(dx * dx + dy * dy >= banda * banda, "afuera {} dentro de las afueras mínimas", p.id);
        } else if p.categoria == Categoria::Generico {
            let r = radio_ul as i64 * 1000;
            assert!(d2 <= r * r, "la pieza {} ({:?}) se sale del disco de {radio_ul} u.l.", p.id, p.rect);
        }
    }
}

