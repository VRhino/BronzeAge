//! La forja de la Armería (D1, Objetos_Del_Heroe §3.4; Forja_En_Plazas §4 y §5): una pieza de los tiers II a V con su receta. El plano y los crisoles
//! salen de la mochila; los materiales y el impuesto los cobra el núcleo. La calidad se tira con el generador que pasa quien llama y multiplica todas
//! las cifras de la pieza, que nace con id propio (D13).

use crate::datos::{Catalogo, ReglasForja, CRISOL};
use crate::inventario::{ErrorInventario, Forjada, Inventario, Pieza};
use base::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorForja {
    /// Al meter la pieza en la mochila (`NoCabe`: mochila llena).
    Inventario(ErrorInventario),
    /// El objeto no está en el catálogo.
    Desconocido,
    /// No tiene receta: no es arma ni armadura, o es de tier I.
    SinReceta,
    /// La Facción de la plaza no tiene adoptada la tecnología de su metal.
    MetalCerrado,
    /// Su tier pide una Era (`OBJETOS.forja.eraPorTier`) que el mundo no ha alcanzado (Forja_En_Plazas §4).
    EraCerrada,
    /// No hay en la mochila un plano de su tier.
    SinPlano,
    /// No hay en la mochila los crisoles de su tier.
    SinCrisoles,
}

impl From<ErrorInventario> for ErrorForja {
    fn from(e: ErrorInventario) -> Self {
        ErrorForja::Inventario(e)
    }
}

/// `forjar` el objeto con el id propio `id`, con el mundo en la Era de orden `era`: gasta de la mochila el plano y los crisoles, tira la calidad en el
/// rango de la receta (los dos extremos dentro) y mete la pieza con sus cifras multiplicadas. Devuelve los materiales que cobra el núcleo, por recurso y
/// en unidades, y la calidad que salió (en %, para el hecho `heroe.objeto_forjado`). Si algo se rechaza, no gasta nada ni tira el generador; los
/// materiales y el impuesto los comprueba quien llama.
pub fn forjar(mochila: &mut Inventario, cat: &Catalogo, objeto: &str, id: u64, era: u32, rng: &mut Rng, adoptada: impl Fn(&str) -> bool) -> Result<(Vec<(String, i64)>, i64), ErrorForja> {
    let ficha = cat.objetos.get(objeto).ok_or(ErrorForja::Desconocido)?;
    let (combate, receta) = ficha.de_combate().and_then(|c| Some((c, c.forja.as_ref()?))).ok_or(ErrorForja::SinReceta)?;
    if combate.tecnologia.as_deref().is_some_and(|t| !adoptada(t)) {
        return Err(ErrorForja::MetalCerrado);
    }
    if ficha.tier.and_then(|t| cat.forja.era_por_tier.get(&t)).is_some_and(|&pide| u32::from(pide) > era) {
        return Err(ErrorForja::EraCerrada);
    }
    let mut m = mochila.clone();
    let plano = m.pila(&receta.plano).ok_or(ErrorForja::SinPlano)?;
    m.sacar(plano, 1)?;
    let crisoles = m.pila(CRISOL).ok_or(ErrorForja::SinCrisoles)?;
    m.sacar(crisoles, receta.crisoles).map_err(|_| ErrorForja::SinCrisoles)?;
    let mut r = rng.clone();
    let (min, max) = receta.calidad;
    let calidad = min + r.por_debajo((max - min + 1) as u64) as i64;
    m.meter(cat, Pieza { objeto: objeto.to_owned(), forjada: Some(Forjada { id, cifras: combate.cifras.por_calidad(calidad) }) }, 1)?;
    (*mochila, *rng) = (m, r);
    Ok((receta.materiales.clone(), calidad))
}

/// El impuesto de forjar una pieza de `tier` (Forja_En_Plazas §5), en monedas: la base del tier con el recargo del grupo de quien forja,
/// `base × (100 + recargo) / 100` (la misma Facción, sin recargo). `tier` es el de una pieza con receta, que tiene su base (comprobado al cargar).
pub fn impuesto(r: &ReglasForja, tier: u8, recargo_pct: u32) -> i64 {
    let base = r.impuesto_por_tier.get(&tier).expect("una pieza con receta tiene su impuesto (comprobado al cargar)");
    base * (100 + i64::from(recargo_pct)) / 100
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::{Atributos, Cifras, PorDano};

    fn mochila(cat: &Catalogo, cosas: &[(&str, u32)]) -> Inventario {
        let mut m = Inventario::vacio(cat.casillas_mochila);
        for (o, n) in cosas {
            m.meter(cat, Pieza::del_catalogo(o), *n).unwrap();
        }
        m
    }

    /// El `orden` de la última Era: no cierra ningún tier.
    const QUINTA: u32 = 5;

    fn todo_abierto(_: &str) -> bool {
        true
    }

    #[test]
    fn forjar_gasta_plano_y_crisoles_cobra_los_materiales_y_multiplica_las_cifras() {
        let cat = Catalogo::default();
        let mut m = mochila(&cat, &[("plano_4", 1), ("crisol", 6)]);
        let mut rng = Rng::nuevo(7);
        let (materiales, salio) = forjar(&mut m, &cat, "torso_pesada_bronce", 3, QUINTA, &mut rng, |t| t == "aleacion_bronce").unwrap();
        assert_eq!(materiales, vec![("cueroCurtido".to_owned(), 4), ("lingoteBronce".to_owned(), 9)]);
        assert_eq!(m.casillas.iter().map(|(n, c)| (*n, c.pieza.objeto.as_str(), c.cantidad)).collect::<Vec<_>>(), vec![(0, "torso_pesada_bronce", 1), (1, "crisol", 2)], "el plano se gasta entero y la pieza ocupa su casilla");
        let calidad = 90 + Rng::nuevo(7).por_debajo(21) as i64;
        assert_eq!(salio, calidad, "devuelve la calidad que tiró");
        let x = |v: i64| (v * calidad + 50) / 100;
        let esperada = Cifras::Armadura { atributos: Atributos { fuerza: x(1), destreza: 0, armadura: x(8), vitalidad: x(6) }, defensa: PorDano { perforante: x(14), cortante: x(14), contundente: x(8) }, salud: x(948) };
        assert_eq!(m.casillas[&0].pieza.forjada, Some(Forjada { id: 3, cifras: esperada }));
    }

    #[test]
    fn la_calidad_cae_en_el_rango_con_sus_extremos() {
        let cat = Catalogo::default();
        let mut rng = Rng::nuevo(1);
        let mut vistas = std::collections::BTreeSet::new();
        for id in 0..400 {
            let mut m = mochila(&cat, &[("plano_2", 1), ("crisol", 2)]);
            forjar(&mut m, &cat, "dagas_cobre", id, QUINTA, &mut rng, todo_abierto).unwrap();
            let Some(Forjada { cifras: Cifras::Arma { dano, .. }, .. }) = &m.casillas[&0].pieza.forjada else { panic!("forjada") };
            vistas.insert(dano.cortante);
        }
        let Cifras::Arma { dano, .. } = cat.objetos["dagas_cobre"].de_combate().unwrap().cifras else { panic!("arma") };
        assert_eq!((vistas.first().copied(), vistas.last().copied()), (Some((dano.cortante * 90 + 50) / 100), Some((dano.cortante * 110 + 50) / 100)));
    }

    #[test]
    fn forjar_rechaza_sin_receta_el_metal_cerrado_y_lo_desconocido() {
        let cat = Catalogo::default();
        let mut m = mochila(&cat, &[("plano_2", 1), ("crisol", 5)]);
        let mut rng = Rng::nuevo(1);
        assert_eq!(forjar(&mut m, &cat, "espada_de_madera", 1, QUINTA, &mut rng, todo_abierto), Err(ErrorForja::Desconocido));
        for o in ["dagas_inicio", "torso_media_inicio", "montura_estepa", "crisol", "medalla_heroe_grande"] {
            assert_eq!(forjar(&mut m, &cat, o, 1, QUINTA, &mut rng, todo_abierto), Err(ErrorForja::SinReceta), "{o}");
        }
        assert_eq!(forjar(&mut m, &cat, "dagas_hierro_temprano", 1, QUINTA, &mut rng, |t| t == "metalurgia_cobre"), Err(ErrorForja::MetalCerrado));
        assert_eq!(m, mochila(&cat, &[("plano_2", 1), ("crisol", 5)]));
        assert_eq!(rng.estado(), Rng::nuevo(1).estado(), "no tira el generador");
    }

    #[test]
    fn forjar_rechaza_sin_plano_sin_crisoles_o_con_la_mochila_llena_sin_gastar_nada() {
        let cat = Catalogo::default();
        let mut rng = Rng::nuevo(1);
        let mut m = mochila(&cat, &[("plano_2", 1), ("crisol", 5)]);
        assert_eq!(forjar(&mut m, &cat, "dagas_bronce", 1, QUINTA, &mut rng, todo_abierto), Err(ErrorForja::SinPlano), "un plano de otro tier no vale");
        let mut m = mochila(&cat, &[("plano_3", 1), ("crisol", 2)]);
        assert_eq!(forjar(&mut m, &cat, "dagas_bronce", 1, QUINTA, &mut rng, todo_abierto), Err(ErrorForja::SinCrisoles));
        let mut m = mochila(&cat, &[("plano_3", 1)]);
        assert_eq!(forjar(&mut m, &cat, "dagas_bronce", 1, QUINTA, &mut rng, todo_abierto), Err(ErrorForja::SinCrisoles));
        assert_eq!(m, mochila(&cat, &[("plano_3", 1)]));
        let mut m = Inventario::vacio(3);
        for (o, n) in [("plano_3", 2), ("crisol", 4), ("dagas_inicio", 1)] {
            m.meter(&cat, Pieza::del_catalogo(o), n).unwrap();
        }
        let antes = m.clone();
        assert_eq!(forjar(&mut m, &cat, "dagas_bronce", 1, QUINTA, &mut rng, todo_abierto), Err(ErrorForja::Inventario(ErrorInventario::NoCabe)));
        assert_eq!((m, rng.estado()), (antes, Rng::nuevo(1).estado()));
    }

    #[test]
    fn los_tiers_iii_a_v_piden_su_era_y_el_ii_ninguna() {
        let cat = Catalogo::default();
        let mut rng = Rng::nuevo(1);
        for (objeto, plano, pide) in [("dagas_cobre", "plano_2", 1), ("dagas_bronce", "plano_3", 3), ("torso_pesada_bronce", "plano_4", 4), ("dagas_acero", "plano_5", 5)] {
            let tier = cat.objetos[objeto].tier.unwrap();
            let lleno = mochila(&cat, &[(plano, 1), ("crisol", u32::from(tier))]);
            let mut m = lleno.clone();
            if pide > 1 {
                assert_eq!(forjar(&mut m, &cat, objeto, 1, pide - 1, &mut rng, todo_abierto), Err(ErrorForja::EraCerrada), "{objeto}, una Era antes");
                assert_eq!((&m, rng.estado()), (&lleno, Rng::nuevo(1).estado()), "{objeto}: no gasta nada ni tira el generador");
            }
            assert!(forjar(&mut m, &cat, objeto, 1, pide, &mut Rng::nuevo(1), todo_abierto).is_ok(), "{objeto}, en su Era");
        }
        let mut m = mochila(&cat, &[("plano_5", 1), ("crisol", 5)]);
        assert_eq!(forjar(&mut m, &cat, "dagas_acero", 1, 1, &mut rng, |t| t != "acero_templado"), Err(ErrorForja::MetalCerrado), "el metal se mira antes que la Era");
    }

    #[test]
    fn el_impuesto_es_la_base_del_tier_con_el_recargo_del_grupo() {
        let r = &Catalogo::default().forja;
        assert_eq!([2, 3, 4, 5].map(|t| impuesto(r, t, 0)), [100, 175, 250, 350], "la misma Facción, la base");
        assert_eq!([0, 100, 200, 500].map(|x| impuesto(r, 3, x)), [175, 350, 525, 1_050], "×1, ×2, ×3 y ×6");
    }
}
