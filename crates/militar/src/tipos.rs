//! Tipos comunes del sistema militar: identificadores y la carga del carro.

use asentamiento::Recurso;
use base::Mil;
use std::collections::BTreeMap;

pub type HeroeId = u64;
pub type EscuadraId = u64;
pub type ColumnaId = u64;
pub type CaravanaId = u64;
pub type AvanzadaId = u64;
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

/// Saca de la carga lo que pasa de `capacidad` (del último recurso al primero, en el orden de la carga) y lo devuelve: lo que ya no cabe cuando el carro pierde capacidad.
pub fn sacar_lo_que_sobra(carga: &mut Carga, capacidad: Mil) -> Carga {
    let mut sobra = total(carga) - capacidad.max(0);
    let mut fuera = Carga::new();
    for (&r, n) in carga.iter_mut().rev() {
        if sobra <= 0 {
            break;
        }
        let x = (*n).min(sobra);
        (*n, sobra) = (*n - x, sobra - x);
        sumar(&mut fuera, r, x);
    }
    carga.retain(|_, n| *n > 0);
    fuera
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn lo_que_sobra_sale_del_ultimo_recurso_al_primero() {
        let mut c = Carga::from([(Recurso::Madera, 5_000), (Recurso::Piedra, 3_000)]);
        assert!(sacar_lo_que_sobra(&mut c, 9_000).is_empty(), "si cabe, nada");
        let fuera = sacar_lo_que_sobra(&mut c, 4_000);
        assert_eq!((total(&fuera), total(&c)), (4_000, 4_000));
        assert_eq!(c, Carga::from([(Recurso::Madera, 4_000)]), "la piedra sale entera y luego la madera");
    }

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
