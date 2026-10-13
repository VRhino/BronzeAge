//! Parcelas de explotación de los bosques (Doc 1.9c, cierra O2). La Leñera se construye dentro de la ciudad; lo que limita
//! cuántas hay es el derecho a explotar una parte del bosque. Cada masa tiene una rejilla fija de parcelas; la Leñera
//! recibe una libre cuyo centro cae en la zona del asentamiento, y cada asentamiento puede tener como máximo una cuota de
//! las de cada masa. Este módulo es la elección; quién tiene qué parcela vive en el estado del núcleo.

use base::{Mil, Punto};
use std::collections::BTreeSet;

/// Una parcela del artefacto (la posición en el vector es su índice estable).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parcela {
    pub bosque: u32,
    pub centro: Punto,
    /// Densidad media bajo la parcela, 0..=1000.
    pub densidad: u32,
}

/// Cuántas parcelas de una masa de `total` puede tener un asentamiento: `cuota_pct` % redondeado hacia abajo, mínimo 1.
pub fn cupo_por_masa(total: usize, cuota_pct: u32) -> usize {
    (total * cuota_pct as usize / 100).max(1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SinParcela {
    /// Ninguna parcela de la zona está libre.
    NoHayLibres,
    /// Hay libres en la zona, pero el asentamiento ya tiene su cupo en cada masa que alcanza.
    CupoAgotado,
}

/// Elige la parcela para una Leñera nueva: libre, con el centro en la zona y en una masa donde el asentamiento no ha
/// llegado a su cupo. Entre las posibles, la de más densidad; a igualdad, la más cercana al centro de la ciudad y luego la de
/// menor índice (determinista).
///
/// - `libre(i)`: la parcela `i` no la tiene nadie.
/// - `propias`: parcelas que ya tiene este asentamiento.
/// - `total_por_masa[b]`: parcelas de la masa `b`.
pub fn elegir(
    parcelas: &[Parcela],
    total_por_masa: &[usize],
    cuota_pct: u32,
    ciudad: Punto,
    propias: &BTreeSet<u32>,
    libre: impl Fn(u32) -> bool,
    en_zona: impl Fn(Punto) -> bool,
) -> Result<u32, SinParcela> {
    let mut en_masa = vec![0usize; total_por_masa.len()];
    for &i in propias {
        en_masa[parcelas[i as usize].bosque as usize] += 1;
    }
    let (mut mejor, mut hubo_libre): (Option<(std::cmp::Reverse<u32>, Mil, u32)>, bool) = (None, false);
    for (i, p) in parcelas.iter().enumerate() {
        let i = i as u32;
        if !libre(i) || !en_zona(p.centro) {
            continue;
        }
        hubo_libre = true;
        if en_masa[p.bosque as usize] >= cupo_por_masa(total_por_masa[p.bosque as usize], cuota_pct) {
            continue;
        }
        // Más densidad, más cerca, menor índice.
        let clave = (std::cmp::Reverse(p.densidad), p.centro.dist(ciudad), i);
        if mejor.is_none_or(|m| clave < m) {
            mejor = Some(clave);
        }
    }
    match mejor {
        Some((_, _, i)) => Ok(i),
        None if hubo_libre => Err(SinParcela::CupoAgotado),
        None => Err(SinParcela::NoHayLibres),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn p(bosque: u32, x: Mil, densidad: u32) -> Parcela {
        Parcela { bosque, centro: Punto::new(x, 0), densidad }
    }

    #[test]
    fn cupos() {
        assert_eq!([1, 2, 3, 5, 10, 40].map(|n| cupo_por_masa(n, 50)), [1, 1, 1, 2, 5, 20]);
        assert_eq!(cupo_por_masa(7, 100), 7);
    }

    fn todas(_: Punto) -> bool {
        true
    }

    #[test]
    fn prefiere_densidad_y_luego_cercania() {
        let parcelas = [p(0, 5_000, 500), p(0, 9_000, 800), p(0, 3_000, 800)];
        let r = elegir(&parcelas, &[3], 100, Punto::new(0, 0), &BTreeSet::new(), |_| true, todas);
        assert_eq!(r, Ok(2), "la de 800 más cercana");
    }

    #[test]
    fn respeta_ocupadas_y_zona() {
        let parcelas = [p(0, 1_000, 900), p(0, 2_000, 100), p(0, 50_000, 999)];
        let r = elegir(&parcelas, &[3], 100, Punto::new(0, 0), &BTreeSet::new(), |i| i != 0, |c| c.x < 10_000);
        assert_eq!(r, Ok(1), "la 0 está ocupada y la 2 fuera de la zona");
        let ninguna = elegir(&parcelas, &[3], 100, Punto::new(0, 0), &BTreeSet::new(), |_| false, todas);
        assert_eq!(ninguna, Err(SinParcela::NoHayLibres));
    }

    #[test]
    fn la_cuota_deja_parcelas_a_los_que_llegan_despues() {
        // Masa de 4 parcelas, cuota 50 % = 2. Quien ya tiene 2 no recibe una tercera aunque queden libres.
        let parcelas = [p(0, 1_000, 500), p(0, 2_000, 500), p(0, 3_000, 500), p(0, 4_000, 500)];
        let propias: BTreeSet<u32> = [0, 1].into();
        let r = elegir(&parcelas, &[4], 50, Punto::new(0, 0), &propias, |i| i >= 2, todas);
        assert_eq!(r, Err(SinParcela::CupoAgotado));
        // Con una sola propia sí.
        let una: BTreeSet<u32> = [0].into();
        assert_eq!(elegir(&parcelas, &[4], 50, Punto::new(0, 0), &una, |i| i >= 1, todas), Ok(1));
    }

    #[test]
    fn un_bosque_pequeno_da_una_parcela_por_asentamiento() {
        // Masa de 2: cupo mínimo 1. El primero toma una y la otra queda para otro asentamiento.
        let parcelas = [p(0, 1_000, 500), p(0, 2_000, 500)];
        let primero = elegir(&parcelas, &[2], 50, Punto::new(0, 0), &BTreeSet::new(), |_| true, todas).unwrap();
        let propias: BTreeSet<u32> = [primero].into();
        assert_eq!(elegir(&parcelas, &[2], 50, Punto::new(0, 0), &propias, |i| i != primero, todas), Err(SinParcela::CupoAgotado));
        // Otro asentamiento (sin propias) toma la que queda.
        assert_eq!(elegir(&parcelas, &[2], 50, Punto::new(0, 0), &BTreeSet::new(), |i| i != primero, todas), Ok(1 - primero));
    }

    #[test]
    fn el_cupo_es_por_masa() {
        // Dos masas de 2 parcelas: tener la de una no impide tomar la de la otra.
        let parcelas = [p(0, 1_000, 500), p(0, 2_000, 500), p(1, 3_000, 500), p(1, 4_000, 500)];
        let propias: BTreeSet<u32> = [0].into();
        let r = elegir(&parcelas, &[2, 2], 50, Punto::new(0, 0), &propias, |i| i != 0, todas);
        assert_eq!(r, Ok(2));
    }
}
