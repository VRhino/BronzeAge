//! El ataque a una avanzada (Combate §7.4, Doc 5.17): lo abre solo un ejército, dura 30 minutos como un asedio (capacidad de 15 héroes) y exige
//! guerra o la declara. Aquí van las etapas, la invulnerabilidad de las ruinas y qué queda de la avanzada según gane o no el atacante; la
//! obra de mapa (colocar, mejorar, reparar, destruir a mano), la guarnición de jugadores y el almacén son de la fase 4F.
//!
//! - **Atalaya**: si gana el atacante, destruida.
//! - **Fuerte**: en ruinas, con un día de invulnerabilidad; si pasado ese día sigue en ruinas, lo atacan y gana el atacante, destruido.
//! - **Ciudadela**: en ruinas, **nunca se destruye**.
//!
//! **En ruinas no funciona nada** (ni visión, ni guarnición, ni reabastecimiento): una avanzada en ruinas no tiene defensores.

use crate::asedio::{resolver_asedio, Asedio};
use crate::datos::Reglas;
use crate::resolucion::Bando;
use base::{Instante, Rng};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Etapa {
    Atalaya,
    Fuerte,
    Ciudadela,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estado {
    Operativa,
    /// Sin guarnición ni visión. Un fuerte recién arruinado es invulnerable hasta `invulnerable_hasta`; una ciudadela arruinada no lo es (no hay
    /// nada más que perder).
    EnRuinas { invulnerable_hasta: Option<Instante> },
    Destruida,
}

/// ¿Se puede abrir un ataque ahora? No contra una destruida ni contra unas ruinas invulnerables.
pub fn atacable(estado: Estado, ahora: Instante) -> bool {
    match estado {
        Estado::Operativa => true,
        Estado::EnRuinas { invulnerable_hasta } => invulnerable_hasta.is_none_or(|h| ahora >= h),
        Estado::Destruida => false,
    }
}

/// Resuelve el ataque. Rebota (`Protegida`) si no es atacable; unas ruinas no tienen defensores y caen sin combate ni tirada; una operativa se
/// resuelve como un asedio (`defensor`: su guarnición de serie y la de jugadores, y los héroes que la defienden).
pub fn resolver_ataque(r: &Reglas, estado: Estado, ahora: Instante, atacante: Bando, defensor: Bando, rng: &mut Rng) -> Asedio {
    let rebota = !atacable(estado, ahora);
    match estado {
        Estado::EnRuinas { .. } if !rebota => {
            if atacante.escuadras.iter().any(|e| e.con_soldados()) {
                Asedio::CaeSinCombate
            } else {
                Asedio::SinAtacantes
            }
        }
        _ => resolver_asedio(r, rebota, atacante, defensor, rng),
    }
}

/// Qué queda de la avanzada tras el ataque. Si el atacante no gana, nada cambia.
pub fn tras_ataque(r: &Reglas, etapa: Etapa, estado: Estado, ahora: Instante, gana_atacante: bool) -> Estado {
    if !gana_atacante || estado == Estado::Destruida {
        return estado;
    }
    match (etapa, estado) {
        (Etapa::Atalaya, _) => Estado::Destruida,
        (Etapa::Fuerte, Estado::Operativa) => Estado::EnRuinas { invulnerable_hasta: Some(ahora + r.invulnerabilidad_avanzada) },
        (Etapa::Fuerte, _) => Estado::Destruida,
        (Etapa::Ciudadela, _) => Estado::EnRuinas { invulnerable_hasta: None },
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::resolucion::apoyo::*;
    use base::MS_POR_MINUTO;
    use militar::Tropa;

    const DIA: i64 = 1440 * MS_POR_MINUTO;

    #[test]
    fn la_atalaya_cae_destruida_si_gana_el_atacante() {
        let r = r();
        assert_eq!(tras_ataque(r, Etapa::Atalaya, Estado::Operativa, 5, true), Estado::Destruida);
        assert_eq!(tras_ataque(r, Etapa::Atalaya, Estado::Operativa, 5, false), Estado::Operativa);
    }

    #[test]
    fn el_fuerte_queda_en_ruinas_con_un_dia_de_invulnerabilidad_y_luego_se_destruye() {
        let r = r();
        let ruinas = tras_ataque(r, Etapa::Fuerte, Estado::Operativa, 1_000, true);
        assert_eq!(ruinas, Estado::EnRuinas { invulnerable_hasta: Some(1_000 + DIA) });
        assert!(!atacable(ruinas, 1_000 + DIA - 1));
        assert!(atacable(ruinas, 1_000 + DIA), "pasado el día vuelve a poder atacarse");
        assert_eq!(tras_ataque(r, Etapa::Fuerte, ruinas, 1_000 + DIA, true), Estado::Destruida);
        assert_eq!(tras_ataque(r, Etapa::Fuerte, ruinas, 1_000 + DIA, false), ruinas);
    }

    #[test]
    fn la_ciudadela_queda_en_ruinas_y_nunca_se_destruye() {
        let r = r();
        let ruinas = tras_ataque(r, Etapa::Ciudadela, Estado::Operativa, 7, true);
        assert_eq!(ruinas, Estado::EnRuinas { invulnerable_hasta: None });
        assert_eq!(tras_ataque(r, Etapa::Ciudadela, ruinas, 8, true), ruinas, "atacar unas ruinas de ciudadela no la destruye");
    }

    #[test]
    fn una_destruida_no_se_ataca_ni_cambia() {
        let r = r();
        assert!(!atacable(Estado::Destruida, 0));
        for etapa in [Etapa::Atalaya, Etapa::Fuerte, Etapa::Ciudadela] {
            assert_eq!(tras_ataque(r, etapa, Estado::Destruida, 0, true), Estado::Destruida);
        }
    }

    #[test]
    fn unas_ruinas_invulnerables_rebotan_y_las_demas_caen_sin_combate_ni_tirada() {
        let r = r();
        let mut a = [escuadra(1, 1, Tropa::MiliciaLanceros)];
        let mut rng = Rng::nuevo(1);
        let antes = rng.estado();
        let invulnerable = Estado::EnRuinas { invulnerable_hasta: Some(100) };
        assert_eq!(resolver_ataque(r, invulnerable, 99, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut [], heroes: &[] }, &mut rng), Asedio::Protegida);
        assert_eq!(resolver_ataque(r, invulnerable, 100, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut [], heroes: &[] }, &mut rng), Asedio::CaeSinCombate);
        assert_eq!(resolver_ataque(r, Estado::EnRuinas { invulnerable_hasta: None }, 0, Bando { escuadras: &mut [], heroes: &[1] }, Bando { escuadras: &mut [], heroes: &[] }, &mut rng), Asedio::SinAtacantes);
        assert_eq!(resolver_ataque(r, Estado::Destruida, 0, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut [], heroes: &[] }, &mut rng), Asedio::Protegida);
        assert_eq!(rng.estado(), antes);
    }

    #[test]
    fn una_operativa_se_resuelve_como_un_asedio_con_su_guarnicion() {
        let r = r();
        let mut a = [escuadra(1, 1, Tropa::ArquerosCompuesto), escuadra(2, 1, Tropa::ArquerosCompuesto)];
        let mut g = [escuadra(3, 0, Tropa::MiliciaLanceros)];
        let res = resolver_ataque(r, Estado::Operativa, 0, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut g, heroes: &[] }, &mut Rng::nuevo(2));
        assert!(matches!(res, Asedio::Combate(_)) && res.conquistada());
        assert_eq!(tras_ataque(r, Etapa::Atalaya, Estado::Operativa, 0, res.conquistada()), Estado::Destruida);
    }
}
