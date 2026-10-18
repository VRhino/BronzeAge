//! Fundación (T §11) y salida de dibujo (T §10).

use crate::ciudad::Ciudad;
use crate::geom::{Celda, Huella, Rect, CELDA_UL};
use crate::tipos::*;
use base::Punto;
use std::collections::BTreeMap;

/// Tipo del catálogo y huella de una de las piezas de la fundación.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TipoPieza {
    pub tipo: u16,
    pub huella: Huella,
}

/// Entrada de [`Ciudad::fundar`]. Los ids son consecutivos desde `primer_id`: Centro Urbano, viviendas, granja.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntradaFundacion {
    pub semilla: u64,
    pub params: Parametros,
    pub perfil: Perfil,
    pub primer_id: u32,
    /// Centro Urbano (4×4 en el catálogo): su centro cae en el origen.
    pub centro: TipoPieza,
    pub vivienda: TipoPieza,
    pub granja: TipoPieza,
}

impl Ciudad {
    /// `edificiosIniciales`: el Centro Urbano en `(0,0)`, `viviendas_iniciales` viviendas y la granja inicial, cada una colocada
    /// viendo las anteriores, con el `radioCiudad` inicial.
    ///
    /// DUDA: si la granja no cabe, T §11 la deja caer al origen, encima del Centro Urbano («defensivo»); aquí simplemente no
    /// se coloca (nunca se solapan piezas). Compruébese con `pieza(primer_id + 1 + viviendas)`.
    pub fn fundar(e: &EntradaFundacion) -> Ciudad {
        let mut c = Ciudad::vacia(e.semilla, e.params);
        let radio_ul = e.params.radio_inicial_ul;
        let mut id = e.primer_id;
        let centro = Pedido {
            id,
            tipo: e.centro.tipo,
            categoria: Categoria::Residencial,
            rol: Rol::Centro,
            huella: e.centro.huella,
            permitir_rotacion: false,
            perfil: e.perfil,
            radio_ul,
        };
        let rect = Rect::desde_centro(Punto::new(0, 0), e.centro.huella);
        c.colocar(&centro, Colocacion { rect, rotado: false }).expect("la ciudad vacía admite el Centro Urbano");
        for _ in 0..e.params.viviendas_iniciales {
            id += 1;
            let ped = Pedido {
                id,
                tipo: e.vivienda.tipo,
                categoria: Categoria::Residencial,
                rol: Rol::Normal,
                huella: e.vivienda.huella,
                permitir_rotacion: false,
                perfil: e.perfil,
                radio_ul,
            };
            c.colocar_siguiente(&ped);
        }
        id += 1;
        let granja = Pedido {
            id,
            tipo: e.granja.tipo,
            categoria: Categoria::Afueras,
            rol: Rol::Normal,
            huella: e.granja.huella,
            permitir_rotacion: false,
            perfil: e.perfil,
            radio_ul,
        };
        c.colocar_siguiente(&granja);
        c
    }

    /// Calles como rectángulos en unidades locales (T §10).
    pub fn calles_rectangulos(&self) -> Vec<RectLocal> {
        fusionar_celdas(&self.calles())
    }
    /// Caminos como rectángulos en unidades locales (T §10).
    pub fn caminos_rectangulos(&self) -> Vec<RectLocal> {
        fusionar_celdas(&self.caminos())
    }
}

/// Rectángulo en unidades locales (lo que ve el cliente: no ve celdas).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RectLocal {
    pub x: i32,
    pub y: i32,
    pub ancho: i32,
    pub alto: i32,
}

/// `fusionarCeldas` (T §10): agrupa cada fila en tiradas horizontales, filas y columnas de menor a mayor. Cada tirada es
/// `{ x: inicio·T, y: row·T, ancho: n·T, alto: T }`.
pub fn fusionar_celdas(celdas: &[Celda]) -> Vec<RectLocal> {
    let mut por_fila: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
    for c in celdas {
        por_fila.entry(c.row).or_default().push(c.col);
    }
    let mut v = Vec::new();
    for (row, mut cols) in por_fila {
        cols.sort_unstable();
        cols.dedup();
        let mut inicio = cols[0];
        let mut previa = cols[0];
        for &col in &cols[1..] {
            if col != previa + 1 {
                v.push(RectLocal { x: inicio * CELDA_UL, y: row * CELDA_UL, ancho: (previa - inicio + 1) * CELDA_UL, alto: CELDA_UL });
                inicio = col;
            }
            previa = col;
        }
        v.push(RectLocal { x: inicio * CELDA_UL, y: row * CELDA_UL, ancho: (previa - inicio + 1) * CELDA_UL, alto: CELDA_UL });
    }
    v
}
