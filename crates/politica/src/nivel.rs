//! Nivel de Facción por experiencia (Diplomacia_Facciones §3.5, Doc 2.2.1). La experiencia es **monótona** y va en milésimas; el nivel es
//! `1 + umbrales alcanzados`, con tope. El nivel alimenta el cap de fundación (Doc 1.7) y el cupo de plazas en nivel 2 y 3.

use crate::datos::ReglasNivel;
use base::Mil;

/// Un hecho que da experiencia de Facción (`NIVEL_FACCION.xp`). Cada sistema dice cuándo ocurre; aquí solo cuánto vale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hecho {
    /// Un combate con números: `heroes` de la Facción participaron y el combate fue digno (si no, no da nada).
    Combate { heroes: u32, digno: bool },
    /// Una batalla de Unity: `heroes` participaron. Sin filtro de dignidad: su resultado no trae el poder de cada bando.
    BatallaUnity { heroes: u32 },
    /// Un edificio nuevo completado.
    EdificioCompletado,
    /// Una plaza conquistada.
    Conquista,
    /// Un asentamiento termina su subida al nivel `nivel`.
    Ascenso { nivel: u32 },
    /// Una plaza fundada; la primera de la Facción no cuenta.
    Fundacion { primera: bool },
    /// Un trueque cumplido entre Facciones distintas (a cada lado).
    TruequeCumplido,
    /// Un campamento de bandidos destruido (no por héroe), por una Facción de nivel `nivel_faccion`.
    Bandidos { nivel_faccion: u32 },
}

/// Experiencia (milésimas) que da un hecho. Los bandidos dan `xp.bandidos / nivel` (hacia abajo) y nada desde `nivelSinXpBandidos`.
pub fn xp_de(r: &ReglasNivel, h: Hecho) -> Mil {
    let x = &r.xp;
    match h {
        Hecho::Combate { heroes, digno } => if digno { x.combate * heroes as Mil } else { 0 },
        Hecho::BatallaUnity { heroes } => x.combate * heroes as Mil,
        Hecho::EdificioCompletado => x.edificio_completado,
        Hecho::Conquista => x.conquista,
        Hecho::Ascenso { nivel } => x.ascenso_por_nivel * nivel as Mil,
        Hecho::Fundacion { primera } => if primera { 0 } else { x.fundacion },
        Hecho::TruequeCumplido => x.trueque_cumplido,
        Hecho::Bandidos { nivel_faccion } if nivel_faccion >= r.nivel_sin_xp_bandidos => 0,
        Hecho::Bandidos { nivel_faccion } => x.bandidos / nivel_faccion.max(1) as Mil,
    }
}

/// `esCombateDigno`: el más débil tiene al menos `ratioCombateDigno` del poder del más fuerte. Lo mismo que `combate::resolucion::Informe::digno` con este ratio.
pub fn combate_digno(r: &ReglasNivel, poder_a: i64, poder_b: i64) -> bool {
    let (debil, fuerte) = (poder_a.min(poder_b) as i128, poder_a.max(poder_b) as i128);
    fuerte == 0 || debil * 1_000_000 >= r.ratio_combate_digno as i128 * fuerte
}

/// `calcularNivelFaccion`.
pub fn nivel(r: &ReglasNivel, experiencia: Mil) -> u32 {
    (1 + r.umbrales.iter().take_while(|&&u| experiencia >= u).count() as u32).min(r.nivel_maximo)
}

/// La experiencia mínima de un nivel (lo que `FijarNivelFaccion` de la administración fija desde 5C, decisión 9).
pub fn experiencia_minima(r: &ReglasNivel, nivel: u32) -> Mil {
    match nivel.clamp(1, r.nivel_maximo) {
        1 => 0,
        n => r.umbrales.get(n as usize - 2).copied().unwrap_or(Mil::MAX),
    }
}

/// `calcularCapFundacion`: plazas fundadas en total que admite el nivel (`CAP_FUNDACION_POR_NIVEL[min(10, nivel) − 1]`).
pub fn cap_fundacion(r: &ReglasNivel, nivel: u32) -> u32 {
    por_nivel(&r.cap_fundacion, nivel)
}

/// `calcularCupoNivel`: cuántas plazas de la Facción pueden estar en el nivel `objetivo` (2 o 3) a la vez; los demás niveles no tienen cupo (`None`).
pub fn cupo_nivel(r: &ReglasNivel, nivel_faccion: u32, objetivo: u32) -> Option<u32> {
    match objetivo {
        2 => Some(por_nivel(&r.cupo_nivel2, nivel_faccion)),
        3 => Some(por_nivel(&r.cupo_nivel3, nivel_faccion)),
        _ => None,
    }
}

fn por_nivel(curva: &[u32], nivel: u32) -> u32 {
    curva[(nivel.max(1) as usize).min(curva.len()) - 1]
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/experienciaFaccion.test.ts` (la parte de reglas) y `eventosDominioFaccion.test.ts`.
    use super::*;
    use crate::datos::Reglas;

    fn r() -> ReglasNivel {
        Reglas::cargar().nivel
    }

    #[test]
    fn el_nivel_cuenta_los_umbrales_alcanzados_con_tope() {
        let r = r();
        assert_eq!(nivel(&r, 0), 1);
        assert_eq!(nivel(&r, 99_999), 1);
        assert_eq!(nivel(&r, 100_000), 2, "experiencia en el primer umbral: nivel 2");
        assert_eq!(nivel(&r, 4_800_000), 10);
        assert_eq!(nivel(&r, Mil::MAX), 10);
        for n in 1..=10 {
            assert_eq!(nivel(&r, experiencia_minima(&r, n)), n);
        }
    }

    #[test]
    fn combate_digno_con_el_ratio_del_bloque() {
        let r = r();
        assert!(combate_digno(&r, 100, 50));
        assert!(!combate_digno(&r, 100, 49));
        assert!(combate_digno(&r, 50, 100), "simétrico");
    }

    #[test]
    fn el_combate_da_por_heroe_y_solo_si_es_digno() {
        let r = r();
        assert_eq!(xp_de(&r, Hecho::Combate { heroes: 3, digno: true }), 7_500);
        assert_eq!(xp_de(&r, Hecho::Combate { heroes: 3, digno: false }), 0);
        assert_eq!(xp_de(&r, Hecho::BatallaUnity { heroes: 2 }), 5_000);
    }

    #[test]
    fn los_bandidos_dan_cada_vez_menos_y_nada_desde_el_nivel_5() {
        let r = r();
        assert_eq!(xp_de(&r, Hecho::Bandidos { nivel_faccion: 1 }), 1_000);
        assert_eq!(xp_de(&r, Hecho::Bandidos { nivel_faccion: 2 }), 500);
        assert_eq!(xp_de(&r, Hecho::Bandidos { nivel_faccion: 3 }), 333, "hacia abajo");
        assert_eq!(xp_de(&r, Hecho::Bandidos { nivel_faccion: 5 }), 0);
    }

    #[test]
    fn crecer_en_paz() {
        let r = r();
        assert_eq!(xp_de(&r, Hecho::Fundacion { primera: true }), 0, "la primera plaza no cuenta");
        assert_eq!(xp_de(&r, Hecho::Fundacion { primera: false }), 20_000);
        assert_eq!(xp_de(&r, Hecho::Ascenso { nivel: 2 }), 60_000, "30 × el nivel alcanzado");
        assert_eq!(xp_de(&r, Hecho::TruequeCumplido), 2_000);
        assert_eq!(xp_de(&r, Hecho::EdificioCompletado), 1_000);
        assert_eq!(xp_de(&r, Hecho::Conquista), 10_000);
    }

    #[test]
    fn cap_y_cupo_por_nivel() {
        let r = r();
        assert_eq!((1..=12).map(|n| cap_fundacion(&r, n)).collect::<Vec<_>>(), vec![1, 2, 3, 3, 4, 5, 5, 6, 6, 7, 7, 7]);
        assert_eq!(cap_fundacion(&r, 0), 1);
        assert_eq!(cupo_nivel(&r, 1, 2), Some(1));
        assert_eq!(cupo_nivel(&r, 1, 3), Some(0));
        assert_eq!(cupo_nivel(&r, 10, 3), Some(3));
        assert_eq!(cupo_nivel(&r, 5, 4), None);
    }
}
