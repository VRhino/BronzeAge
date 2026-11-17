//! Tipos comunes del comercio: identificadores, el lado de un trueque y la carga (recurso → cantidad en milésimas).

use asentamiento::Recurso;
use base::Mil;
use std::collections::BTreeMap;

pub type AsentamientoId = u32;
pub type FaccionId = u32;
pub type CaravanaId = u64;
pub type AcuerdoId = u64;
pub type OrdenId = u64;
pub type EscuadraId = u64;
pub type HeroeId = u64;

/// Lo que lleva una caravana o un carro: recurso → cantidad (milésimas). `BTreeMap`: el orden de iteración es determinista.
pub type Carga = BTreeMap<Recurso, Mil>;

/// Uno de los dos lados de un trueque. A propone y sale de A hacia B con `lineas_a`; B al revés.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Lado {
    A,
    B,
}

impl Lado {
    pub fn otro(self) -> Lado {
        match self {
            Lado::A => Lado::B,
            Lado::B => Lado::A,
        }
    }
}

/// Suma `n` (> 0) a la carga.
pub fn sumar(carga: &mut Carga, r: Recurso, n: Mil) {
    if n > 0 {
        *carga.entry(r).or_insert(0) += n;
    }
}

/// Total de unidades de una carga (suma sin ponderar por precio).
pub fn total(carga: &Carga) -> Mil {
    carga.values().sum()
}

/// Cantidad de un recurso en la carga (0 si no está).
pub fn de(carga: &Carga, r: Recurso) -> Mil {
    carga.get(&r).copied().unwrap_or(0)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_carga_suma_y_ordena() {
        let mut c = Carga::new();
        sumar(&mut c, Recurso::Piedra, 3_000);
        sumar(&mut c, Recurso::Madera, 2_000);
        sumar(&mut c, Recurso::Madera, 1_000);
        sumar(&mut c, Recurso::Oro, 0);
        assert_eq!(total(&c), 6_000);
        assert_eq!(de(&c, Recurso::Madera), 3_000);
        assert_eq!(de(&c, Recurso::Oro), 0);
        assert_eq!(c.keys().next(), Some(&Recurso::Madera), "orden determinista del enum");
        assert_eq!(Lado::A.otro(), Lado::B);
    }
}
