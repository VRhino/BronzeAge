//! La capital de una Facción (Diplomacia_Facciones §3.7, Doc 2.2). La **designada vigente** es la plaza de la Facción con su marca y un Palacio activo; la
//! **efectiva**, la designada vigente o, si no la hay, la plaza viva **más antigua**. El Rey la traslada con un cooldown que no cuenta si la designada se perdió.

use crate::datos::Reglas;
use crate::faccion::Faccion;
use crate::tipos::{AsentamientoId, FaccionId};
use base::Instante;

/// Lo que la regla mira de cada plaza; el núcleo la arma desde su estado (como mucho las 7 del cap de fundación por Facción, decisión 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlazaDeCapital {
    pub id: AsentamientoId,
    pub faccion: FaccionId,
    /// Desde cuándo es de la Facción (5C-1, 2026-10-24): la fundación o, si la conquistó, la conquista. La más antigua en la Facción es la capital sin designar.
    pub desde: Instante,
    pub palacio_activo: bool,
    /// `capitalDeFaccionId`: la marca que deja `designarCapital`. Sobrevive a un cambio de dueño (y entonces no vale).
    pub marca: Option<FaccionId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCapital {
    /// No es una plaza de la Facción.
    PlazaAjena,
    SinPalacio,
    YaEsLaCapital,
    /// La designada sigue vigente y no ha pasado `CAPITAL.cooldownDias`.
    Cooldown { libre_en: Instante },
}

/// `capitalDesignada`.
pub fn designada(f: FaccionId, plazas: impl IntoIterator<Item = PlazaDeCapital>) -> Option<AsentamientoId> {
    plazas.into_iter().find(|p| p.faccion == f && p.marca == Some(f) && p.palacio_activo).map(|p| p.id)
}

/// `encontrarCapital`: la designada vigente o la plaza que lleva más tiempo en la Facción (a igual instante, el id menor; 5C-1).
/// Las plazas llegan por un iterador (el paso de minuto las saca del núcleo sin reservar memoria).
pub fn efectiva<I>(f: FaccionId, plazas: I) -> Option<AsentamientoId>
where
    I: IntoIterator<Item = PlazaDeCapital> + Clone,
{
    designada(f, plazas.clone()).or_else(|| plazas.into_iter().filter(|p| p.faccion == f).min_by_key(|p| (p.desde, p.id)).map(|p| p.id))
}

/// `designarCapital`: una plaza propia con Palacio activo que no sea ya la capital, y el cooldown si la designada sigue vigente. Anota `capital_designada_en`
/// y devuelve las plazas a las que hay que **quitar** la marca de esta Facción (el núcleo la pone en `destino`).
pub fn designar(r: &Reglas, faccion: &mut Faccion, plazas: &[PlazaDeCapital], destino: AsentamientoId, ahora: Instante) -> Result<Vec<AsentamientoId>, ErrorCapital> {
    let p = plazas.iter().find(|p| p.id == destino && p.faccion == faccion.id).ok_or(ErrorCapital::PlazaAjena)?;
    if !p.palacio_activo {
        return Err(ErrorCapital::SinPalacio);
    }
    if efectiva(faccion.id, plazas.iter().copied()) == Some(destino) {
        return Err(ErrorCapital::YaEsLaCapital);
    }
    if let (Some(en), Some(_)) = (faccion.capital_designada_en, designada(faccion.id, plazas.iter().copied())) {
        if ahora < en + r.cooldown_capital {
            return Err(ErrorCapital::Cooldown { libre_en: en + r.cooldown_capital });
        }
    }
    faccion.capital_designada_en = Some(ahora);
    Ok(plazas.iter().filter(|q| q.id != destino && q.marca == Some(faccion.id)).map(|q| q.id).collect())
}

impl crate::faccion::Facciones {
    /// [`designar`] sobre la Facción `faccion` del almacén (el núcleo pone la marca en `destino` y la quita de las que devuelve). `None` si la Facción no existe.
    pub fn designar_capital(&mut self, r: &Reglas, faccion: FaccionId, plazas: &[PlazaDeCapital], destino: AsentamientoId, ahora: Instante) -> Option<Result<Vec<AsentamientoId>, ErrorCapital>> {
        self.get_mut(faccion).map(|f| designar(r, f, plazas, destino, ahora))
    }
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/capital.test.ts`.
    use super::*;
    use crate::datos::DIA;
    use crate::faccion::pruebas::con_faccion;
    use crate::faccion::Facciones;

    fn plaza(id: AsentamientoId, desde: Instante, palacio: bool) -> PlazaDeCapital {
        PlazaDeCapital { id, faccion: 1, desde, palacio_activo: palacio, marca: None }
    }
    const VIEJA: AsentamientoId = 1;
    const NUEVA: AsentamientoId = 2;
    const OTRA: AsentamientoId = 3;
    fn vieja() -> PlazaDeCapital {
        plaza(VIEJA, 1, false)
    }
    fn nueva() -> PlazaDeCapital {
        plaza(NUEVA, 2, true)
    }
    fn faccion() -> Faccion {
        let mut fs = Facciones::default();
        con_faccion(&mut fs, 1, "F", 10, &[]);
        fs.get(1).unwrap().clone()
    }
    fn aplicar(plazas: &mut [PlazaDeCapital], destino: AsentamientoId, quitar: &[AsentamientoId]) {
        for p in plazas.iter_mut() {
            if p.id == destino {
                p.marca = Some(1);
            } else if quitar.contains(&p.id) {
                p.marca = None;
            }
        }
    }

    #[test]
    fn sin_designar_la_plaza_viva_mas_antigua() {
        assert_eq!(efectiva(1, [nueva(), vieja()]), Some(VIEJA));
        assert_eq!(efectiva(9, [nueva(), vieja()]), None);
    }

    #[test]
    fn la_designada_gana_mientras_conserve_el_palacio_y_sea_de_la_faccion() {
        let marcada = PlazaDeCapital { marca: Some(1), ..nueva() };
        assert_eq!(efectiva(1, [vieja(), marcada]), Some(NUEVA));
        assert_eq!(efectiva(1, [vieja(), PlazaDeCapital { palacio_activo: false, ..marcada }]), Some(VIEJA), "perdió el Palacio");
        // Cambió de manos: la marca era de la otra Facción.
        let ajena = PlazaDeCapital { faccion: 2, ..marcada };
        assert_eq!(efectiva(2, [ajena, PlazaDeCapital { id: 7, faccion: 2, ..vieja() }]), Some(7));
    }

    #[test]
    fn exige_una_plaza_propia_con_palacio_activo() {
        let r = Reglas::default();
        let mut f = faccion();
        assert_eq!(designar(&r, &mut f, &[vieja(), nueva()], VIEJA, 0), Err(ErrorCapital::SinPalacio));
        assert_eq!(designar(&r, &mut f, &[vieja(), nueva()], 99, 0), Err(ErrorCapital::PlazaAjena));
        assert_eq!(designar(&r, &mut f, &[PlazaDeCapital { faccion: 2, ..nueva() }], NUEVA, 0), Err(ErrorCapital::PlazaAjena));
    }

    #[test]
    fn designa_marca_solo_esa_plaza_y_anota_cuando() {
        let r = Reglas::default();
        let mut f = faccion();
        let mut plazas = [vieja(), nueva()];
        let quitar = designar(&r, &mut f, &plazas, NUEVA, 5).unwrap();
        aplicar(&mut plazas, NUEVA, &quitar);
        assert_eq!(f.capital_designada_en, Some(5));
        assert_eq!(efectiva(1, plazas.iter().copied()), Some(NUEVA));
    }

    #[test]
    fn rechaza_designar_la_que_ya_es_capital() {
        let r = Reglas::default();
        assert_eq!(designar(&r, &mut faccion(), &[plaza(VIEJA, 1, true)], VIEJA, 0), Err(ErrorCapital::YaEsLaCapital));
    }

    #[test]
    fn el_traslado_tiene_cooldown_y_se_levanta_al_pasar_el_plazo() {
        let r = Reglas::default();
        let mut f = faccion();
        let mut plazas = [vieja(), nueva(), plaza(OTRA, 3, true)];
        let q = designar(&r, &mut f, &plazas, NUEVA, 0).unwrap();
        aplicar(&mut plazas, NUEVA, &q);
        assert_eq!(designar(&r, &mut f.clone(), &plazas, OTRA, 13 * DIA), Err(ErrorCapital::Cooldown { libre_en: 14 * DIA }));
        let q = designar(&r, &mut f, &plazas, OTRA, 14 * DIA).unwrap();
        assert_eq!(q, vec![NUEVA]);
        aplicar(&mut plazas, OTRA, &q);
        assert_eq!(plazas.iter().filter(|p| p.marca == Some(1)).map(|p| p.id).collect::<Vec<_>>(), vec![OTRA]);
    }

    #[test]
    fn si_la_capital_designada_se_perdio_no_hay_cooldown() {
        let r = Reglas::default();
        let mut f = faccion();
        let mut plazas = [vieja(), nueva(), plaza(OTRA, 3, true)];
        let q = designar(&r, &mut f, &plazas, NUEVA, 0).unwrap();
        aplicar(&mut plazas, NUEVA, &q);
        plazas[1].palacio_activo = false;
        assert!(designar(&r, &mut f, &plazas, OTRA, DIA).is_ok());
    }
}
