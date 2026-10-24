//! Residencia (Diplomacia_Facciones §4.1-§4.2, Doc 2.5): el cooldown entre cambios y qué plaza admite a un héroe que se muda. Mover al héroe, soltar su
//! guarnición, vaciar sus cargos y llevarlo al campamento más cercano es del núcleo (5B). También aquí, **Q8**: quién cuenta para la nobleza.

use crate::datos::Reglas;
use crate::tipos::FaccionId;
use base::Instante;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorResidencia {
    /// Cambió hace menos de `cooldownCambioResidenciaDias`.
    Cooldown { libre_en: Instante },
    /// Solo se reside en una plaza de la propia Facción.
    PlazaAjena,
    /// Ya reside ahí.
    YaReside,
    /// La plaza lo ha vetado.
    Vetado,
}

/// `exigirSinCooldownDeResidencia`: `cambiarResidencia`, `dejarResidencia` y `residirEnCampamento` exigen que haya pasado el cooldown desde el último cambio.
/// El primero es libre; la reubicación forzosa (conquista, ruina) no lo estampa.
pub fn sin_cooldown(r: &Reglas, ultimo_cambio: Option<Instante>, ahora: Instante) -> Result<(), ErrorResidencia> {
    match ultimo_cambio.map(|t| t + r.cooldown_residencia) {
        Some(libre_en) if ahora < libre_en => Err(ErrorResidencia::Cooldown { libre_en }),
        _ => Ok(()),
    }
}

/// `cambiarResidencia(destino)`: el destino es de **su** Facción, aún no reside ahí y no lo ha vetado; y sin cooldown.
pub fn puede_mudarse(r: &Reglas, ultimo_cambio: Option<Instante>, ahora: Instante, faccion_del_heroe: Option<FaccionId>, dueña: FaccionId, ya_reside: bool, vetado: bool) -> Result<(), ErrorResidencia> {
    sin_cooldown(r, ultimo_cambio, ahora)?;
    if faccion_del_heroe != Some(dueña) {
        return Err(ErrorResidencia::PlazaAjena);
    }
    if ya_reside {
        return Err(ErrorResidencia::YaReside);
    }
    if vetado {
        return Err(ErrorResidencia::Vetado);
    }
    Ok(())
}

/// **Q8** (decisión del autor, 2026-10-24): los «ciudadanos en el asentamiento» que pide la nobleza (`nobleza.minCiudadanos`) son **todos sus residentes**,
/// fundadores y quienes se mudaron.
pub fn ciudadanos_para_nobleza(residentes: usize) -> u32 {
    residentes as u32
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/faccion.test.ts` (`cambiarResidencia`) y del cooldown de `comandosResidencia.test.ts`.
    use super::*;
    use crate::datos::DIA;

    #[test]
    fn el_cooldown_cuenta_desde_el_ultimo_cambio_y_el_primero_es_libre() {
        let r = Reglas::default();
        assert_eq!(sin_cooldown(&r, None, 0), Ok(()));
        assert_eq!(sin_cooldown(&r, Some(DIA), 3 * DIA), Err(ErrorResidencia::Cooldown { libre_en: 4 * DIA }));
        assert_eq!(sin_cooldown(&r, Some(DIA), 4 * DIA), Ok(()));
    }

    #[test]
    fn solo_a_una_plaza_de_la_propia_faccion_sin_veto_y_donde_no_reside() {
        let r = Reglas::default();
        assert_eq!(puede_mudarse(&r, None, 0, Some(1), 1, false, false), Ok(()));
        assert_eq!(puede_mudarse(&r, None, 0, Some(2), 1, false, false), Err(ErrorResidencia::PlazaAjena));
        assert_eq!(puede_mudarse(&r, None, 0, None, 1, false, false), Err(ErrorResidencia::PlazaAjena));
        assert_eq!(puede_mudarse(&r, None, 0, Some(1), 1, true, false), Err(ErrorResidencia::YaReside));
        assert_eq!(puede_mudarse(&r, None, 0, Some(1), 1, false, true), Err(ErrorResidencia::Vetado));
        assert!(matches!(puede_mudarse(&r, Some(0), DIA, Some(1), 1, false, false), Err(ErrorResidencia::Cooldown { .. })));
    }

    #[test]
    fn q8_cuentan_todos_los_residentes() {
        assert_eq!(ciudadanos_para_nobleza(3), 3);
    }
}
