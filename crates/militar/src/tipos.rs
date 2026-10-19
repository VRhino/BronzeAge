//! Tipos comunes del sistema militar: identificadores y la carga del carro.

use asentamiento::Recurso;
use base::Mil;
use std::collections::BTreeMap;

pub type HeroeId = u64;
pub type EscuadraId = u64;
pub type ColumnaId = u64;
pub type CaravanaId = u64;
pub type BatallaId = u64;
pub type AsentamientoId = u32;
pub type CampamentoId = u32;
pub type FaccionId = u32;

/// Lo que lleva el carro de una columna: recurso → cantidad (milésimas). `BTreeMap`: el orden de iteración es determinista.
pub type Carga = BTreeMap<Recurso, Mil>;

/// Total de unidades de una carga.
pub fn total(carga: &Carga) -> Mil {
    carga.values().sum()
}

/// Suma `n` (> 0) a la carga.
pub fn sumar(carga: &mut Carga, r: Recurso, n: Mil) {
    if n > 0 {
        *carga.entry(r).or_insert(0) += n;
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_carga_suma_y_totaliza() {
        let mut c = Carga::new();
        sumar(&mut c, Recurso::Madera, 2_000);
        sumar(&mut c, Recurso::Madera, 1_000);
        sumar(&mut c, Recurso::Piedra, 0);
        sumar(&mut c, Recurso::Trigo, -5);
        assert_eq!(c.len(), 1);
        assert_eq!(total(&c), 3_000);
    }
}
