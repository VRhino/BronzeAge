//! Las reglas puras de los alijos de exploración (sistema 6, tanda 6F; `Generacion_Del_Mundo.md` §12, Doc 1 D29 y D60-D63; `typescript-final:src/engine/alijos.ts`): quién puede
//! buscarlos, cuándo se ven y cuándo se abren. No conocen el núcleo, para poder moverlas tal cual al crate `tecnologia` (6A) al integrar; el núcleo pone el estado (`alijos_abiertos`
//! de cada héroe), el artefacto (posición y oro de cada alijo) y la orden (`nucleo/alijos.rs`).
//!
//! Colocarlos no es de aquí: los trae el artefacto de mundo (`generador/src/contenidoVivo.ts`). Son **por héroe**: cada uno abre cada alijo una sola vez y el alijo no desaparece.

use base::Mil;

/// Por qué no se abre un alijo (sus códigos son `alijo.no_puedes_buscar`, `alijo.ya_abierto` y `alijo.lejos`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoSeAbre {
    /// El héroe es de una Facción con plazas (`buscaAlijos`).
    NoPuedesBuscar,
    /// Ese héroe ya lo abrió.
    YaAbierto,
    /// Su columna no está en el mapa a `radioPuerta` del alijo (sin columna, tampoco: «hay que estar en el sitio»).
    Lejos,
}

/// `buscaAlijos` (D63): solo el héroe sin Facción o el de una Facción sin ninguna plaza.
pub fn puede_buscar(faccion: Option<u32>, tiene_plazas: impl FnOnce(u32) -> bool) -> bool {
    faccion.is_none_or(|f| !tiene_plazas(f))
}

/// `alijosALaVista` (D62): lo ve quien puede buscarlo, con su columna a `vision` (`VISION.jugadorSolo`) o menos, mientras no lo haya abierto. `distancia2` es el cuadrado de la
/// distancia de la columna al alijo, en milésimas.
pub fn a_la_vista(puede: bool, abierto: bool, distancia2: Option<i128>, vision: Mil) -> bool {
    puede && !abierto && distancia2.is_some_and(|d| d <= (vision as i128).pow(2))
}

/// `abrirAlijo` (D62), sin tocar nada: en este orden, poder buscar, no haberlo abierto ya y estar en el sitio (la columna a `radio_puerta` o menos). `distancia2` como en [`a_la_vista`].
pub fn abrir(puede: bool, abierto: bool, distancia2: Option<i128>, radio_puerta: Mil) -> Result<(), NoSeAbre> {
    if !puede {
        return Err(NoSeAbre::NoPuedesBuscar);
    }
    if abierto {
        return Err(NoSeAbre::YaAbierto);
    }
    if distancia2.is_none_or(|d| d > (radio_puerta as i128).pow(2)) {
        return Err(NoSeAbre::Lejos);
    }
    Ok(())
}

/// El oro que da un alijo, en unidades enteras hacia abajo (C3b): el artefacto lo trae en milésimas.
pub fn oro(milesimas: Mil) -> Mil {
    base::entero(milesimas.max(0))
}

#[cfg(test)]
mod pruebas {
    //! `typescript-final:src/engine/__tests__/alijos.test.ts`, «ver y abrir un alijo» (la colocación es del generador). Lo que no cabe en el carro se pierde lo prueba el núcleo
    //! (`tests/alijos.rs`), que es quien carga.
    use super::*;

    const VISION: Mil = 15_000;
    const PUERTA: Mil = 10_000;
    fn d2(x: i64) -> Option<i128> {
        Some((x as i128).pow(2))
    }

    #[test]
    fn se_ve_a_la_vista_del_heroe_y_hasta_que_lo_abre() {
        assert!(a_la_vista(true, false, d2(VISION), VISION));
        assert!(!a_la_vista(true, false, d2(VISION + 1_000), VISION));
        assert!(!a_la_vista(true, true, d2(0), VISION), "abierto ya no se ve");
        assert!(!a_la_vista(false, false, d2(0), VISION), "con plazas no se ve");
        assert!(!a_la_vista(true, false, None, VISION), "sin columna en el mapa no se ve");
    }

    #[test]
    fn se_abre_en_el_sitio_una_vez_por_heroe_y_solo_sin_plazas() {
        assert_eq!(abrir(true, false, d2(5_000), PUERTA), Ok(()));
        assert_eq!(abrir(true, false, d2(PUERTA), PUERTA), Ok(()), "en el borde de la puerta vale");
        assert_eq!(abrir(true, true, d2(5_000), PUERTA), Err(NoSeAbre::YaAbierto));
        assert_eq!(abrir(true, false, d2(PUERTA + 1), PUERTA), Err(NoSeAbre::Lejos));
        assert_eq!(abrir(true, false, None, PUERTA), Err(NoSeAbre::Lejos), "sin columna");
        assert_eq!(abrir(false, true, None, PUERTA), Err(NoSeAbre::NoPuedesBuscar), "poder buscar se mira primero");
    }

    #[test]
    fn busca_quien_no_tiene_faccion_o_es_de_una_sin_plazas() {
        assert!(puede_buscar(None, |_| unreachable!()));
        assert!(puede_buscar(Some(3), |f| f != 3));
        assert!(!puede_buscar(Some(3), |f| f == 3));
    }

    #[test]
    fn el_oro_va_en_unidades_enteras_hacia_abajo() {
        assert_eq!((oro(11_000), oro(5_999), oro(-4)), (11_000, 5_000, 0));
    }
}
