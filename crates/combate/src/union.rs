//! A qué bando se une quien llega a una batalla (Batalla_Unity_Ciclo §7.1, Doc 5.15.1b). Es puro: las relaciones entre Facciones (aliadas para
//! batalla, el Rey que admite otras en los ataques) y si quien llega está dentro de lo que se defiende son del mundo y entran como datos.
//!
//! **Solo defiende quien está dentro y solo ataca quien está fuera** (decisiones del autor, 2026-10-22): una columna en el mapa, a la puerta de una plaza, un pueblo o una
//! avanzada, no se une a su defensa; hay que haber entrado en el elemento. Y al ataque solo se une quien está en el mapa: quien está dentro de lo que se ataca (entró como
//! neutral, por ejemplo) o de cualquier otro lugar no puede atacar desde dentro. Una batalla en el mapa (persecución, bandidos, caravana) tampoco admite a quien está dentro de algo.
//! Un asedio, un pueblo o una avanzada admiten en el ataque, desde fuera, a cualquiera que no sea amigo del defensor (su misma Facción, aliados y vasallos): el Rey de la Facción atacante ya no decide (2026-10-22, tercera ronda).

use crate::ciclo::Contexto;
use crate::resolucion::Lado;
use militar::FaccionId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorUnion {
    /// Una batalla campal entre ejércitos: nadie de fuera.
    NadieDeFuera,
    /// Una persecución solo admite columnas personales.
    SoloColumnasPersonales,
    /// Una persecución exige decir a qué bando.
    LadoObligatorio,
    /// Nadie ayuda a los bandidos.
    NadieAyudaALosBandidos,
    /// Un asedio, una caravana, un pueblo o una avanzada piden Facción.
    SinFaccion,
    /// «Tu Facción no combate aquí.»
    NoCombateAqui,
    /// Pidió un bando distinto del que le toca.
    LadoDistinto(Lado),
    /// Le toca defender una plaza, un pueblo o una avanzada y no está dentro: a la puerta, en el mapa, no se defiende.
    HayQueEstarDentro,
    /// Le toca atacar (o la batalla es en el mapa) y no está fuera: desde dentro de un lugar no se ataca ni se entra en una batalla del mapa.
    HayQueEstarFuera,
}

/// Dónde está quien se une respecto de lo que se disputa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Donde {
    /// En el mapa, con su columna.
    Fuera,
    /// Dentro de lo que se disputa (ha entrado en la plaza, el pueblo o la avanzada).
    Dentro,
    /// Dentro de otro lugar (otra plaza, un campamento de mercenarios…): fuera del mapa, pero no dentro de lo que se disputa.
    EnOtroLugar,
}

/// Quién combate en la batalla y qué relaciones cuentan.
pub struct Reparto<'a> {
    pub contexto: Contexto,
    pub atacante: Option<FaccionId>,
    pub defensor: Option<FaccionId>,
    /// `sonAliadasEnBatalla`: alianza **o** vasallaje activos, en cualquier sentido (distinto de `estanAliadas`: para unirse, señor y vasallo se
    /// defienden entre sí y ninguno se une al ataque contra el otro).
    pub aliadas_en_batalla: &'a dyn Fn(FaccionId, FaccionId) -> bool,
    /// El Rey de la Facción atacante admite otras Facciones en sus ataques (`admiteOtrasEnAtaques`).
    pub rey_admite_otras: bool,
    /// Dónde está quien se une. Defender un asedio, un pueblo o una avanzada exige [`Donde::Dentro`]; atacar, y cualquier lado de una batalla en el mapa
    /// (persecución, caravana, bandidos), exige [`Donde::Fuera`].
    pub donde: Donde,
}

/// `ladoParaUnirse`. `columna_personal`: quien se une va en una columna personal (no en un ejército).
///
/// | Contexto | Regla |
/// |---|---|
/// | campal | nadie de fuera |
/// | persecución | solo columnas personales, de cualquier Facción, y `lado` obligatorio |
/// | caravana | hace falta Facción: **defensor** si es la del defensor o aliada para batalla; **atacante** si es la del atacante o, si su Rey lo admite, de cualquier otra que no sea del defensor ni aliada suya |
/// | asedio, pueblo, avanzada | hace falta Facción: **defensor** si es la del defensor o aliada para batalla; **atacante** cualquier otra (la del atacante y los neutrales y enemigos del dueño) |
///
/// Defender un asedio, un pueblo o una avanzada exige además estar **dentro** (`Reparto::donde`): quien llega a la puerta solo puede atacar, y atacar exige estar **fuera**.
pub fn lado_para_unirse(rep: &Reparto, faccion: Option<FaccionId>, columna_personal: bool, pedido: Option<Lado>) -> Result<Lado, ErrorUnion> {
    let fuera = rep.donde == Donde::Fuera;
    let lado = match rep.contexto {
        Contexto::CampoAbierto { solitarios: false, .. } => return Err(ErrorUnion::NadieDeFuera),
        Contexto::CampoAbierto { solitarios: true, .. } => {
            if !columna_personal {
                return Err(ErrorUnion::SoloColumnasPersonales);
            }
            let lado = pedido.ok_or(ErrorUnion::LadoObligatorio)?;
            return if fuera { Ok(lado) } else { Err(ErrorUnion::HayQueEstarFuera) };
        }
        Contexto::CampamentoBandidos { .. } => {
            if pedido == Some(Lado::Defensor) {
                return Err(ErrorUnion::NadieAyudaALosBandidos);
            }
            return if fuera { Ok(Lado::Atacante) } else { Err(ErrorUnion::HayQueEstarFuera) };
        }
        Contexto::Asedio { .. } | Contexto::Caravana { .. } | Contexto::SaqueoPueblo { .. } | Contexto::Avanzada { .. } => {
            let f = faccion.ok_or(ErrorUnion::SinFaccion)?;
            let del_defensor = rep.defensor.is_some_and(|d| d == f || (rep.aliadas_en_batalla)(f, d));
            // Una caravana va por el mapa: no hay puerta ni dentro, así que los dos lados están fuera. Lo demás se defiende desde dentro y se ataca desde fuera.
            let sin_dentro = matches!(rep.contexto, Contexto::Caravana { .. });
            if del_defensor {
                match (sin_dentro, rep.donde) {
                    (true, Donde::Fuera) | (false, Donde::Dentro) => Lado::Defensor,
                    (true, _) => return Err(ErrorUnion::HayQueEstarFuera),
                    (false, _) => return Err(ErrorUnion::HayQueEstarDentro),
                }
            } else if rep.atacante == Some(f) || rep.rey_admite_otras || !sin_dentro {
                // Un asedio, un pueblo o una avanzada admiten en el ataque, desde fuera, a quien no es amigo del defensor: la Facción atacante y los neutrales y enemigos del dueño (2026-10-22, tercera ronda).
                // Una caravana sigue pidiendo la Facción atacante o que su Rey admita otras.
                if !fuera {
                    return Err(ErrorUnion::HayQueEstarFuera);
                }
                Lado::Atacante
            } else {
                return Err(ErrorUnion::NoCombateAqui);
            }
        }
    };
    match pedido {
        Some(p) if p != lado => Err(ErrorUnion::LadoDistinto(lado)),
        _ => Ok(lado),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::avanzada::Etapa;
    use crate::caravana::Modalidad;
    use crate::ciclo::CaravanaEnBatalla;
    use base::Punto;

    const ATACANTE: FaccionId = 1;
    const DEFENSOR: FaccionId = 2;
    const ALIADA_DEL_DEFENSOR: FaccionId = 3;
    const OTRA: FaccionId = 4;
    const ALIADA_DEL_ATACANTE: FaccionId = 5;

    fn aliadas(a: FaccionId, b: FaccionId) -> bool {
        matches!((a, b), (ALIADA_DEL_DEFENSOR, DEFENSOR) | (DEFENSOR, ALIADA_DEL_DEFENSOR) | (ALIADA_DEL_ATACANTE, ATACANTE) | (ATACANTE, ALIADA_DEL_ATACANTE))
    }

    /// Quien se une está en el mapa si ataca y dentro de lo que se disputa si defiende (los tests que miran el sitio lo cambian a mano).
    fn rep(contexto: Contexto, rey: bool) -> Reparto<'static> {
        Reparto { contexto, atacante: Some(ATACANTE), defensor: Some(DEFENSOR), aliadas_en_batalla: &aliadas, rey_admite_otras: rey, donde: Donde::Fuera }
    }

    /// Lo mismo, pero con quien se une dentro de lo que se disputa.
    fn dentro(contexto: Contexto) -> Reparto<'static> {
        Reparto { donde: Donde::Dentro, ..rep(contexto, false) }
    }

    const ASEDIO: Contexto = Contexto::Asedio { asentamiento: 1 };

    #[test]
    fn la_batalla_campal_no_admite_a_nadie_de_fuera() {
        let c = Contexto::CampoAbierto { punto: Punto::default(), solitarios: false };
        assert_eq!(lado_para_unirse(&rep(c.clone(), true), Some(OTRA), true, Some(Lado::Atacante)), Err(ErrorUnion::NadieDeFuera));
    }

    #[test]
    fn la_persecucion_es_libre_pero_solo_para_columnas_personales_y_con_lado() {
        let c = Contexto::CampoAbierto { punto: Punto::default(), solitarios: true };
        let r = rep(c.clone(), false);
        assert_eq!(lado_para_unirse(&r, Some(OTRA), true, Some(Lado::Defensor)), Ok(Lado::Defensor));
        assert_eq!(lado_para_unirse(&r, None, true, Some(Lado::Atacante)), Ok(Lado::Atacante), "también sin Facción");
        assert_eq!(lado_para_unirse(&r, Some(OTRA), true, None), Err(ErrorUnion::LadoObligatorio));
        assert_eq!(lado_para_unirse(&r, Some(OTRA), false, Some(Lado::Atacante)), Err(ErrorUnion::SoloColumnasPersonales), "un ejército no se une");
        let desde_dentro = Reparto { donde: Donde::Dentro, ..rep(c.clone(), false) };
        assert_eq!(lado_para_unirse(&desde_dentro, Some(OTRA), true, Some(Lado::Atacante)), Err(ErrorUnion::HayQueEstarFuera), "quien está dentro de un lugar no se une a una persecución del mapa");
    }

    #[test]
    fn contra_bandidos_cualquiera_siempre_como_atacante() {
        let r = rep(Contexto::CampamentoBandidos { campamento: 1, punto: Punto::default() }, false);
        for f in [None, Some(DEFENSOR), Some(OTRA)] {
            assert_eq!(lado_para_unirse(&r, f, false, None), Ok(Lado::Atacante));
            assert_eq!(lado_para_unirse(&r, f, true, Some(Lado::Atacante)), Ok(Lado::Atacante));
            assert_eq!(lado_para_unirse(&r, f, true, Some(Lado::Defensor)), Err(ErrorUnion::NadieAyudaALosBandidos));
        }
        let desde_dentro = Reparto { donde: Donde::EnOtroLugar, ..rep(Contexto::CampamentoBandidos { campamento: 1, punto: Punto::default() }, false) };
        assert_eq!(lado_para_unirse(&desde_dentro, None, false, None), Err(ErrorUnion::HayQueEstarFuera));
    }

    #[test]
    fn en_un_asedio_defienden_la_facciones_del_defensor_y_sus_aliadas_y_el_vasallo_cuenta() {
        let r = dentro(ASEDIO);
        assert_eq!(lado_para_unirse(&r, Some(DEFENSOR), false, None), Ok(Lado::Defensor));
        assert_eq!(lado_para_unirse(&r, Some(ALIADA_DEL_DEFENSOR), true, None), Ok(Lado::Defensor), "aliada o vasalla para batalla");
        assert_eq!(lado_para_unirse(&rep(ASEDIO, false), Some(ATACANTE), false, None), Ok(Lado::Atacante));
        assert_eq!(lado_para_unirse(&r, None, false, None), Err(ErrorUnion::SinFaccion));
    }

    #[test]
    fn un_neutral_ataca_un_asedio_desde_fuera_y_una_caravana_solo_si_el_rey_del_atacante_lo_admite() {
        assert_eq!(lado_para_unirse(&rep(ASEDIO, false), Some(OTRA), false, None), Ok(Lado::Atacante), "un asedio admite a neutrales y enemigos del dueño sin el Rey");
        // Los amigos del defensor no atacan: desde fuera solo les queda esperar a entrar; los neutrales y enemigos de dentro tampoco atacan desde dentro.
        assert_eq!(lado_para_unirse(&rep(ASEDIO, false), Some(ALIADA_DEL_DEFENSOR), false, None), Err(ErrorUnion::HayQueEstarDentro));
        assert_eq!(lado_para_unirse(&dentro(ASEDIO), Some(OTRA), false, None), Err(ErrorUnion::HayQueEstarFuera));
        let caravana = Contexto::Caravana { caravanas: vec![CaravanaEnBatalla { id: 1, carros: 2 }], punto: Punto::default(), modalidad: Modalidad::Disputa };
        assert_eq!(lado_para_unirse(&rep(caravana.clone(), false), Some(OTRA), false, None), Err(ErrorUnion::NoCombateAqui), "tu Facción no combate aquí");
        assert_eq!(lado_para_unirse(&rep(caravana.clone(), true), Some(OTRA), false, None), Ok(Lado::Atacante));
        // Nunca un aliado del defensor, aunque el Rey admita a todos.
        let dentro_con_rey = Reparto { donde: Donde::Dentro, ..rep(ASEDIO, true) };
        assert_eq!(lado_para_unirse(&dentro_con_rey, Some(ALIADA_DEL_DEFENSOR), false, Some(Lado::Atacante)), Err(ErrorUnion::LadoDistinto(Lado::Defensor)));
    }

    #[test]
    fn pedir_un_lado_distinto_del_que_toca_se_rechaza() {
        let r = rep(ASEDIO, false);
        assert_eq!(lado_para_unirse(&r, Some(ATACANTE), false, Some(Lado::Defensor)), Err(ErrorUnion::LadoDistinto(Lado::Atacante)));
        assert_eq!(lado_para_unirse(&dentro(ASEDIO), Some(DEFENSOR), false, Some(Lado::Defensor)), Ok(Lado::Defensor));
    }

    #[test]
    fn la_caravana_el_pueblo_y_la_avanzada_se_reparten_como_el_asedio() {
        let punto = Punto::default();
        for c in [
            Contexto::Caravana { caravanas: vec![CaravanaEnBatalla { id: 1, carros: 2 }], punto, modalidad: Modalidad::Disputa },
            Contexto::SaqueoPueblo { pueblo: 1, punto },
            Contexto::Avanzada { avanzada: 1, etapa: Etapa::Fuerte, punto },
        ] {
            // Una caravana va por el mapa (los dos lados están fuera); el pueblo y la avanzada se defienden desde dentro.
            let defendiendo = if matches!(c, Contexto::Caravana { .. }) { rep(c.clone(), false) } else { dentro(c.clone()) };
            assert_eq!(lado_para_unirse(&defendiendo, Some(ALIADA_DEL_DEFENSOR), false, None), Ok(Lado::Defensor));
            let r = rep(c.clone(), false);
            if matches!(c, Contexto::Caravana { .. }) {
                assert_eq!(lado_para_unirse(&r, Some(ALIADA_DEL_ATACANTE), false, None), Err(ErrorUnion::NoCombateAqui), "una caravana no admite a un aliado del atacante sin el Rey");
            } else {
                assert_eq!(lado_para_unirse(&r, Some(ALIADA_DEL_ATACANTE), false, None), Ok(Lado::Atacante), "el pueblo y la avanzada admiten a quien no es amigo del defensor");
            }
            assert_eq!(lado_para_unirse(&r, None, false, None), Err(ErrorUnion::SinFaccion));
        }
    }

    #[test]
    fn defender_una_plaza_un_pueblo_o_una_avanzada_exige_estar_dentro_y_atacar_exige_estar_fuera() {
        let punto = Punto::default();
        for c in [ASEDIO, Contexto::SaqueoPueblo { pueblo: 1, punto }, Contexto::Avanzada { avanzada: 1, etapa: Etapa::Fuerte, punto }] {
            let en_el_mapa = rep(c.clone(), false);
            // Ni la propia Facción ni una aliada defienden desde la puerta, aunque pidan el bando que les toca; ni desde otro lugar.
            assert_eq!(lado_para_unirse(&en_el_mapa, Some(DEFENSOR), false, None), Err(ErrorUnion::HayQueEstarDentro));
            assert_eq!(lado_para_unirse(&en_el_mapa, Some(ALIADA_DEL_DEFENSOR), true, Some(Lado::Defensor)), Err(ErrorUnion::HayQueEstarDentro));
            let en_otro_lugar = Reparto { donde: Donde::EnOtroLugar, ..rep(c.clone(), false) };
            assert_eq!(lado_para_unirse(&en_otro_lugar, Some(DEFENSOR), false, None), Err(ErrorUnion::HayQueEstarDentro));
            // Dentro sí; y el ataque solo desde el mapa: quien está dentro de lo que se ataca (entró como neutral, p. ej.) o de otro lugar no ataca.
            assert_eq!(lado_para_unirse(&dentro(c.clone()), Some(DEFENSOR), false, None), Ok(Lado::Defensor));
            assert_eq!(lado_para_unirse(&en_el_mapa, Some(ATACANTE), false, None), Ok(Lado::Atacante));
            assert_eq!(lado_para_unirse(&dentro(c.clone()), Some(ATACANTE), false, None), Err(ErrorUnion::HayQueEstarFuera), "un héroe de la Facción atacante que está dentro no se une al ataque desde dentro");
            assert_eq!(lado_para_unirse(&en_otro_lugar, Some(ATACANTE), false, None), Err(ErrorUnion::HayQueEstarFuera));
            let con_rey = Reparto { donde: Donde::Dentro, ..rep(c.clone(), true) };
            assert_eq!(lado_para_unirse(&con_rey, Some(OTRA), false, None), Err(ErrorUnion::HayQueEstarFuera), "tampoco el neutral que el Rey admite");
            assert_eq!(lado_para_unirse(&en_el_mapa, Some(OTRA), false, None), Ok(Lado::Atacante), "un neutral o un enemigo del dueño, desde fuera, se une al ataque aunque no sea de la Facción atacante");
        }
        // Una caravana va por el mapa: no hay puerta ni dentro, así que los dos lados se unen desde fuera.
        let caravana = Contexto::Caravana { caravanas: vec![CaravanaEnBatalla { id: 1, carros: 2 }], punto, modalidad: Modalidad::Aguante };
        assert_eq!(lado_para_unirse(&rep(caravana.clone(), false), Some(DEFENSOR), false, None), Ok(Lado::Defensor));
        assert_eq!(lado_para_unirse(&dentro(caravana.clone()), Some(DEFENSOR), false, None), Err(ErrorUnion::HayQueEstarFuera));
        assert_eq!(lado_para_unirse(&dentro(caravana.clone()), Some(ATACANTE), false, None), Err(ErrorUnion::HayQueEstarFuera));
    }

    #[test]
    fn un_defensor_sin_faccion_no_tiene_aliados() {
        let mut r = rep(Contexto::Caravana { caravanas: vec![CaravanaEnBatalla { id: 1, carros: 2 }], punto: Punto::default(), modalidad: Modalidad::Aguante }, false);
        r.defensor = None;
        assert_eq!(lado_para_unirse(&r, Some(DEFENSOR), false, None), Err(ErrorUnion::NoCombateAqui));
        assert_eq!(lado_para_unirse(&r, Some(ATACANTE), false, None), Ok(Lado::Atacante));
    }
}
