//! A qué bando se une quien llega a una batalla (Batalla_Unity_Ciclo §7.1, Doc 5.15.1b). Es puro: las relaciones entre Facciones (aliadas para
//! batalla, el Rey que admite otras en los ataques) y si quien llega está dentro de lo que se defiende son del mundo y entran como datos.
//!
//! **Solo defiende quien está dentro** (decisión del autor, 2026-10-22): una columna en el mapa, a la puerta de una plaza, un pueblo o una avanzada, no se une a su
//! defensa; hay que haber entrado en el elemento. Se une, en cambio, al ataque.

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
    /// Quien se une está **dentro** de lo que se disputa (ha entrado en la plaza; nadie puede entrar aún en un pueblo ni en una avanzada). Solo se mira para
    /// defender un asedio, un pueblo o una avanzada; en el resto (campo abierto, caravana) no hay dentro.
    pub dentro: bool,
}

/// `ladoParaUnirse`. `columna_personal`: quien se une va en una columna personal (no en un ejército).
///
/// | Contexto | Regla |
/// |---|---|
/// | campal | nadie de fuera |
/// | persecución | solo columnas personales, de cualquier Facción, y `lado` obligatorio |
/// | campamento de bandidos | cualquiera, también sin Facción, **siempre atacante** |
/// | asedio, caravana, pueblo, avanzada | hace falta Facción: **defensor** si es la del defensor o aliada para batalla; **atacante** si es la del atacante o, si su Rey lo admite, de cualquier otra que no sea del defensor ni aliada suya |
///
/// Defender un asedio, un pueblo o una avanzada exige además estar **dentro** (`Reparto::dentro`): quien llega a la puerta solo puede atacar.
pub fn lado_para_unirse(rep: &Reparto, faccion: Option<FaccionId>, columna_personal: bool, pedido: Option<Lado>) -> Result<Lado, ErrorUnion> {
    let lado = match rep.contexto {
        Contexto::CampoAbierto { solitarios: false, .. } => return Err(ErrorUnion::NadieDeFuera),
        Contexto::CampoAbierto { solitarios: true, .. } => {
            if !columna_personal {
                return Err(ErrorUnion::SoloColumnasPersonales);
            }
            return pedido.ok_or(ErrorUnion::LadoObligatorio);
        }
        Contexto::CampamentoBandidos { .. } => {
            if pedido == Some(Lado::Defensor) {
                return Err(ErrorUnion::NadieAyudaALosBandidos);
            }
            return Ok(Lado::Atacante);
        }
        Contexto::Asedio { .. } | Contexto::Caravana { .. } | Contexto::SaqueoPueblo { .. } | Contexto::Avanzada { .. } => {
            let f = faccion.ok_or(ErrorUnion::SinFaccion)?;
            let del_defensor = rep.defensor.is_some_and(|d| d == f || (rep.aliadas_en_batalla)(f, d));
            if del_defensor {
                // Una caravana no tiene dentro (va por el mapa); lo demás se defiende desde dentro.
                if !rep.dentro && !matches!(rep.contexto, Contexto::Caravana { .. }) {
                    return Err(ErrorUnion::HayQueEstarDentro);
                }
                Lado::Defensor
            } else if rep.atacante == Some(f) || rep.rey_admite_otras {
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
    use base::Punto;

    const ATACANTE: FaccionId = 1;
    const DEFENSOR: FaccionId = 2;
    const ALIADA_DEL_DEFENSOR: FaccionId = 3;
    const OTRA: FaccionId = 4;
    const ALIADA_DEL_ATACANTE: FaccionId = 5;

    fn aliadas(a: FaccionId, b: FaccionId) -> bool {
        matches!((a, b), (ALIADA_DEL_DEFENSOR, DEFENSOR) | (DEFENSOR, ALIADA_DEL_DEFENSOR) | (ALIADA_DEL_ATACANTE, ATACANTE) | (ATACANTE, ALIADA_DEL_ATACANTE))
    }

    fn rep(contexto: Contexto, rey: bool) -> Reparto<'static> {
        Reparto { contexto, atacante: Some(ATACANTE), defensor: Some(DEFENSOR), aliadas_en_batalla: &aliadas, rey_admite_otras: rey, dentro: true }
    }

    const ASEDIO: Contexto = Contexto::Asedio { asentamiento: 1 };

    #[test]
    fn la_batalla_campal_no_admite_a_nadie_de_fuera() {
        let c = Contexto::CampoAbierto { punto: Punto::default(), solitarios: false };
        assert_eq!(lado_para_unirse(&rep(c, true), Some(OTRA), true, Some(Lado::Atacante)), Err(ErrorUnion::NadieDeFuera));
    }

    #[test]
    fn la_persecucion_es_libre_pero_solo_para_columnas_personales_y_con_lado() {
        let c = Contexto::CampoAbierto { punto: Punto::default(), solitarios: true };
        let r = rep(c, false);
        assert_eq!(lado_para_unirse(&r, Some(OTRA), true, Some(Lado::Defensor)), Ok(Lado::Defensor));
        assert_eq!(lado_para_unirse(&r, None, true, Some(Lado::Atacante)), Ok(Lado::Atacante), "también sin Facción");
        assert_eq!(lado_para_unirse(&r, Some(OTRA), true, None), Err(ErrorUnion::LadoObligatorio));
        assert_eq!(lado_para_unirse(&r, Some(OTRA), false, Some(Lado::Atacante)), Err(ErrorUnion::SoloColumnasPersonales), "un ejército no se une");
    }

    #[test]
    fn contra_bandidos_cualquiera_siempre_como_atacante() {
        let r = rep(Contexto::CampamentoBandidos { campamento: 1, punto: Punto::default() }, false);
        for f in [None, Some(DEFENSOR), Some(OTRA)] {
            assert_eq!(lado_para_unirse(&r, f, false, None), Ok(Lado::Atacante));
            assert_eq!(lado_para_unirse(&r, f, true, Some(Lado::Atacante)), Ok(Lado::Atacante));
            assert_eq!(lado_para_unirse(&r, f, true, Some(Lado::Defensor)), Err(ErrorUnion::NadieAyudaALosBandidos));
        }
    }

    #[test]
    fn en_un_asedio_defienden_la_facciones_del_defensor_y_sus_aliadas_y_el_vasallo_cuenta() {
        let r = rep(ASEDIO, false);
        assert_eq!(lado_para_unirse(&r, Some(DEFENSOR), false, None), Ok(Lado::Defensor));
        assert_eq!(lado_para_unirse(&r, Some(ALIADA_DEL_DEFENSOR), true, None), Ok(Lado::Defensor), "aliada o vasalla para batalla");
        assert_eq!(lado_para_unirse(&r, Some(ATACANTE), false, None), Ok(Lado::Atacante));
        assert_eq!(lado_para_unirse(&r, None, false, None), Err(ErrorUnion::SinFaccion));
    }

    #[test]
    fn un_neutral_ataca_solo_si_el_rey_del_atacante_lo_admite() {
        assert_eq!(lado_para_unirse(&rep(ASEDIO, false), Some(OTRA), false, None), Err(ErrorUnion::NoCombateAqui), "tu Facción no combate aquí");
        assert_eq!(lado_para_unirse(&rep(ASEDIO, true), Some(OTRA), false, None), Ok(Lado::Atacante));
        // Nunca un aliado del defensor, aunque el Rey admita a todos.
        assert_eq!(lado_para_unirse(&rep(ASEDIO, true), Some(ALIADA_DEL_DEFENSOR), false, Some(Lado::Atacante)), Err(ErrorUnion::LadoDistinto(Lado::Defensor)));
    }

    #[test]
    fn pedir_un_lado_distinto_del_que_toca_se_rechaza() {
        let r = rep(ASEDIO, false);
        assert_eq!(lado_para_unirse(&r, Some(ATACANTE), false, Some(Lado::Defensor)), Err(ErrorUnion::LadoDistinto(Lado::Atacante)));
        assert_eq!(lado_para_unirse(&r, Some(DEFENSOR), false, Some(Lado::Defensor)), Ok(Lado::Defensor));
    }

    #[test]
    fn la_caravana_el_pueblo_y_la_avanzada_se_reparten_como_el_asedio() {
        let punto = Punto::default();
        for c in [
            Contexto::Caravana { caravana: 1, punto, modalidad: Modalidad::Disputa, carros: 2 },
            Contexto::SaqueoPueblo { pueblo: 1, punto },
            Contexto::Avanzada { avanzada: 1, etapa: Etapa::Fuerte, punto },
        ] {
            let r = rep(c, false);
            assert_eq!(lado_para_unirse(&r, Some(ALIADA_DEL_DEFENSOR), false, None), Ok(Lado::Defensor));
            assert_eq!(lado_para_unirse(&r, Some(ALIADA_DEL_ATACANTE), false, None), Err(ErrorUnion::NoCombateAqui), "un aliado del atacante tampoco entra sin el Rey");
            assert_eq!(lado_para_unirse(&r, None, false, None), Err(ErrorUnion::SinFaccion));
        }
    }

    #[test]
    fn defender_una_plaza_un_pueblo_o_una_avanzada_exige_estar_dentro_y_la_caravana_no_tiene_dentro() {
        let punto = Punto::default();
        let a_la_puerta = |c: Contexto| Reparto { dentro: false, ..rep(c, false) };
        for c in [ASEDIO, Contexto::SaqueoPueblo { pueblo: 1, punto }, Contexto::Avanzada { avanzada: 1, etapa: Etapa::Fuerte, punto }] {
            let fuera = a_la_puerta(c);
            // Ni la propia Facción ni una aliada defienden desde la puerta, aunque pidan el bando que les toca.
            assert_eq!(lado_para_unirse(&fuera, Some(DEFENSOR), false, None), Err(ErrorUnion::HayQueEstarDentro));
            assert_eq!(lado_para_unirse(&fuera, Some(ALIADA_DEL_DEFENSOR), true, Some(Lado::Defensor)), Err(ErrorUnion::HayQueEstarDentro));
            // Dentro sí; y el ataque no pide estar dentro de nada.
            assert_eq!(lado_para_unirse(&rep(c, false), Some(DEFENSOR), false, None), Ok(Lado::Defensor));
            assert_eq!(lado_para_unirse(&fuera, Some(ATACANTE), false, None), Ok(Lado::Atacante));
            assert_eq!(lado_para_unirse(&fuera, Some(OTRA), false, None), Err(ErrorUnion::NoCombateAqui), "lo demás no cambia");
        }
        let caravana = a_la_puerta(Contexto::Caravana { caravana: 1, punto, modalidad: Modalidad::Aguante, carros: 2 });
        assert_eq!(lado_para_unirse(&caravana, Some(DEFENSOR), false, None), Ok(Lado::Defensor), "una caravana va por el mapa: no hay puerta ni dentro");
    }

    #[test]
    fn un_defensor_sin_faccion_no_tiene_aliados() {
        let mut r = rep(Contexto::Caravana { caravana: 1, punto: Punto::default(), modalidad: Modalidad::Aguante, carros: 2 }, false);
        r.defensor = None;
        assert_eq!(lado_para_unirse(&r, Some(DEFENSOR), false, None), Err(ErrorUnion::NoCombateAqui));
        assert_eq!(lado_para_unirse(&r, Some(ATACANTE), false, None), Ok(Lado::Atacante));
    }
}
