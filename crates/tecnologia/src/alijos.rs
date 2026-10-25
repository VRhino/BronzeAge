//! Alijos de exploración (Generacion_Del_Mundo §12, D29, D60-D63): escondites de oro que coloca el artefacto del mundo (`mundo::Mundo::alijos`, por
//! índice). Son **por héroe**: cada uno abre cada alijo una vez, nadie le quita uno a otro. Solo los ven y los abren los héroes sin Facción o de una
//! Facción sin asentamientos; su oro sube **al carro** de la columna y lo que no cabe se pierde (decisión técnica 10). Nada programa eventos: ver un
//! alijo es una consulta de la vista y abrirlo, una orden.

use base::{entero, Mil, Punto};
use std::collections::BTreeSet;

/// Los índices de los alijos del artefacto que un héroe ya abrió (`Heroe.alijos_abiertos`).
pub type Abiertos = BTreeSet<u32>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorAlijo {
    /// `alijo.no_puedes_buscar`: el héroe es de una Facción con asentamientos.
    NoPuedesBuscar,
    /// `alijo.ya_abierto`.
    YaAbierto,
    /// `alijo.lejos`: sin columna o con ella a más de `radioPuerta` del alijo.
    Lejos,
}

/// Lo que dio un alijo (`alijo.abierto { heroe, alijo, oro, cargado }`): su oro en unidades enteras y lo que cupo en el carro (milésimas).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Apertura {
    pub oro: Mil,
    pub cargado: Mil,
}

/// `alijosALaVista`: los alijos (índice) que el héroe no ha abierto a `vision` (`VISION.jugadorSolo`) de su columna. Sin columna, ninguno.
pub fn a_la_vista<'a>(alijos: &'a [Punto], abiertos: &'a Abiertos, columna: Option<Punto>, vision: Mil) -> impl Iterator<Item = u32> + 'a {
    let v2 = (vision as i128) * (vision as i128);
    alijos.iter().enumerate().filter(move |(i, p)| columna.is_some_and(|c| c.dist2(**p) <= v2) && !abiertos.contains(&(*i as u32))).map(|(i, _)| i as u32)
}

/// `abrirAlijo` en el instante de la orden, sobre el alijo `indice` del artefacto (posición y oro): `puede_buscar` (sin Facción o de una sin asentamientos), no abierto antes, la columna a `radio` (`radioPuerta`)
/// del alijo. El oro del artefacto (milésimas) se redondea hacia abajo a unidades enteras y sube al carro hasta `hueco` (lo que cabe); el resto se pierde.
pub fn abrir(indice: u32, (posicion, oro): (Punto, Mil), abiertos: &mut Abiertos, columna: Option<Punto>, radio: Mil, puede_buscar: bool, hueco: Mil) -> Result<Apertura, ErrorAlijo> {
    if !puede_buscar {
        return Err(ErrorAlijo::NoPuedesBuscar);
    }
    if abiertos.contains(&indice) {
        return Err(ErrorAlijo::YaAbierto);
    }
    if !columna.is_some_and(|c| c.dist2(posicion) <= (radio as i128) * (radio as i128)) {
        return Err(ErrorAlijo::Lejos);
    }
    abiertos.insert(indice);
    let oro = entero(oro);
    Ok(Apertura { oro, cargado: oro.min(entero(hueco.max(0))) })
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/alijos.test.ts` («ver y abrir un alijo»; colocarlos es del generador del artefacto). Las
    //! distancias son las calibradas de Rust: visión 15 y puerta 2 (en el TS, otras).
    use super::*;

    const VISION: Mil = 15_000;
    const PUERTA: Mil = 2_000;

    fn p(x: Mil, y: Mil) -> Punto {
        Punto::new(x, y)
    }

    #[test]
    fn se_ve_a_la_vista_del_heroe_y_hasta_que_lo_abre() {
        let alijos = [p(500_000, 500_000)];
        let nada = Abiertos::new();
        assert_eq!(a_la_vista(&alijos, &nada, Some(p(500_000 + VISION, 500_000)), VISION).collect::<Vec<_>>(), vec![0]);
        assert_eq!(a_la_vista(&alijos, &nada, Some(p(500_000 + VISION + 1, 500_000)), VISION).count(), 0);
        assert_eq!(a_la_vista(&alijos, &[0].into(), Some(p(500_000, 500_000)), VISION).count(), 0);
        assert_eq!(a_la_vista(&alijos, &nada, None, VISION).count(), 0, "sin columna no ve nada");
    }

    #[test]
    fn se_abre_en_el_sitio_una_vez_por_heroe_y_su_oro_sube_al_carro_lo_que_no_cabe_se_pierde() {
        let (sitio, cerca, lejos) = (p(500_000, 500_000), Some(p(500_000, 501_500)), Some(p(600_000, 600_000)));
        let mut abiertos = Abiertos::new();
        assert_eq!(abrir(0, (sitio, 5_000), &mut abiertos, cerca, PUERTA, true, 100_000), Ok(Apertura { oro: 5_000, cargado: 5_000 }));
        assert_eq!(abiertos, [0].into());
        assert_eq!(abrir(0, (sitio, 5_000), &mut Abiertos::new(), cerca, PUERTA, true, 3_000), Ok(Apertura { oro: 5_000, cargado: 3_000 }), "solo cabe lo que cabe");
        assert_eq!(abrir(0, (sitio, 5_000), &mut abiertos, cerca, PUERTA, true, 100_000), Err(ErrorAlijo::YaAbierto));
        assert_eq!(abrir(0, (sitio, 5_000), &mut Abiertos::new(), lejos, PUERTA, true, 100_000), Err(ErrorAlijo::Lejos));
        assert_eq!(abrir(0, (sitio, 5_000), &mut Abiertos::new(), None, PUERTA, true, 100_000), Err(ErrorAlijo::Lejos));
        assert_eq!(abrir(0, (sitio, 5_000), &mut Abiertos::new(), cerca, PUERTA, false, 100_000), Err(ErrorAlijo::NoPuedesBuscar), "con asentamiento");
    }

    #[test]
    fn el_oro_va_en_unidades_enteras_hacia_abajo() {
        assert_eq!(abrir(3, (p(0, 0), 7_999), &mut Abiertos::new(), Some(p(0, 0)), PUERTA, true, 2_500), Ok(Apertura { oro: 7_000, cargado: 2_000 }));
    }
}
