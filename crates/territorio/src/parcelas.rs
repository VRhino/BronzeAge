//! Parcelas de explotación de los bosques (Doc 1.9c, cierra O2). La Leñera se construye dentro de la ciudad; lo que limita
//! cuántas hay es el derecho a explotar una parte del bosque. Cada masa tiene una rejilla fija de parcelas; la Leñera
//! recibe una libre cuyo centro cae en la zona del asentamiento. Sin cuota por masa (quitada el 2026-11-06, decisión del autor):
//! la frontera entre zonas ya reparte el bosque. Este módulo es la elección; quién tiene qué parcela vive en el estado del núcleo.

use base::{Mil, Punto};

/// Una parcela del artefacto (la posición en el vector es su índice estable).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parcela {
    pub bosque: u32,
    pub centro: Punto,
    /// Densidad media bajo la parcela, 0..=1000.
    pub densidad: u32,
}

/// Elige la parcela para una Leñera nueva: libre y con el centro en la zona. Entre las posibles, la de más densidad; a igualdad,
/// la más cercana al centro de la ciudad y luego la de menor índice (determinista). `None`: ninguna parcela de la zona está libre.
///
/// - `libre(i)`: la parcela `i` no la tiene nadie.
pub fn elegir(parcelas: &[Parcela], ciudad: Punto, libre: impl Fn(u32) -> bool, en_zona: impl Fn(Punto) -> bool) -> Option<u32> {
    let mut mejor: Option<(std::cmp::Reverse<u32>, Mil, u32)> = None;
    for (i, p) in parcelas.iter().enumerate() {
        let i = i as u32;
        if !libre(i) || !en_zona(p.centro) {
            continue;
        }
        // Más densidad, más cerca, menor índice.
        let clave = (std::cmp::Reverse(p.densidad), p.centro.dist(ciudad), i);
        if mejor.is_none_or(|m| clave < m) {
            mejor = Some(clave);
        }
    }
    mejor.map(|(_, _, i)| i)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn p(bosque: u32, x: Mil, densidad: u32) -> Parcela {
        Parcela { bosque, centro: Punto::new(x, 0), densidad }
    }

    fn todas(_: Punto) -> bool {
        true
    }

    #[test]
    fn prefiere_densidad_y_luego_cercania() {
        let parcelas = [p(0, 5_000, 500), p(0, 9_000, 800), p(0, 3_000, 800)];
        let r = elegir(&parcelas, Punto::new(0, 0), |_| true, todas);
        assert_eq!(r, Some(2), "la de 800 más cercana");
    }

    #[test]
    fn respeta_ocupadas_y_zona() {
        let parcelas = [p(0, 1_000, 900), p(0, 2_000, 100), p(0, 50_000, 999)];
        let r = elegir(&parcelas, Punto::new(0, 0), |i| i != 0, |c| c.x < 10_000);
        assert_eq!(r, Some(1), "la 0 está ocupada y la 2 fuera de la zona");
        assert_eq!(elegir(&parcelas, Punto::new(0, 0), |_| false, todas), None);
    }

    #[test]
    fn sin_cuota_un_asentamiento_toma_todas_las_de_su_zona() {
        // Masa de 4 parcelas: quien ya tiene 3 toma la cuarta (hasta el 2026-11-06 la cuota del 50 % se lo impedía).
        let parcelas = [p(0, 1_000, 500), p(0, 2_000, 500), p(0, 3_000, 500), p(0, 4_000, 500)];
        assert_eq!(elegir(&parcelas, Punto::new(0, 0), |i| i == 3, todas), Some(3));
    }
}
