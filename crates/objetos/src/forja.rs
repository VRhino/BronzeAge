//! La forja de la Armería (D1, Catalogo_De_Objetos §6b): una pieza de los tiers II a V con su receta. El plano y los crisoles salen de la mochila;
//! los materiales los cobra el núcleo del almacén personal del héroe. La calidad se tira con el generador que pasa quien llama y multiplica todas
//! las cifras de la pieza, que nace con id propio (D13).

use crate::datos::{Catalogo, CRISOL};
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

/// `forjar` el objeto con el id propio `id`: gasta de la mochila el plano y los crisoles, tira la calidad en el rango de la receta (los dos extremos
/// dentro) y mete la pieza con sus cifras multiplicadas. Devuelve los materiales que cobra el núcleo, por recurso y en unidades. Si algo se rechaza,
/// no gasta nada ni tira el generador; los materiales los comprueba quien llama antes de llamar.
pub fn forjar(mochila: &mut Inventario, cat: &Catalogo, objeto: &str, id: u64, rng: &mut Rng, adoptada: impl Fn(&str) -> bool) -> Result<Vec<(String, i64)>, ErrorForja> {
    let combate = cat.objetos.get(objeto).ok_or(ErrorForja::Desconocido)?.de_combate();
    let (combate, receta) = combate.and_then(|c| Some((c, c.forja.as_ref()?))).ok_or(ErrorForja::SinReceta)?;
    if combate.tecnologia.as_deref().is_some_and(|t| !adoptada(t)) {
        return Err(ErrorForja::MetalCerrado);
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
    Ok(receta.materiales.clone())
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

    fn todo_abierto(_: &str) -> bool {
        true
    }

    #[test]
    fn forjar_gasta_plano_y_crisoles_cobra_los_materiales_y_multiplica_las_cifras() {
        let cat = Catalogo::default();
        let mut m = mochila(&cat, &[("plano_4", 1), ("crisol", 6)]);
        let mut rng = Rng::nuevo(7);
        let materiales = forjar(&mut m, &cat, "torso_pesada_bronce", 3, &mut rng, |t| t == "aleacion_bronce").unwrap();
        assert_eq!(materiales, vec![("cueroCurtido".to_owned(), 4), ("lingoteBronce".to_owned(), 9)]);
        assert_eq!(m.casillas.iter().map(|(n, c)| (*n, c.pieza.objeto.as_str(), c.cantidad)).collect::<Vec<_>>(), vec![(0, "torso_pesada_bronce", 1), (1, "crisol", 2)], "el plano se gasta entero y la pieza ocupa su casilla");
        let calidad = 90 + Rng::nuevo(7).por_debajo(21) as i64;
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
            forjar(&mut m, &cat, "dagas_cobre", id, &mut rng, todo_abierto).unwrap();
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
        assert_eq!(forjar(&mut m, &cat, "espada_de_madera", 1, &mut rng, todo_abierto), Err(ErrorForja::Desconocido));
        for o in ["dagas_inicio", "torso_media_inicio", "montura_estepa", "crisol", "medalla_heroe_grande"] {
            assert_eq!(forjar(&mut m, &cat, o, 1, &mut rng, todo_abierto), Err(ErrorForja::SinReceta), "{o}");
        }
        assert_eq!(forjar(&mut m, &cat, "dagas_hierro_temprano", 1, &mut rng, |t| t == "metalurgia_cobre"), Err(ErrorForja::MetalCerrado));
        assert_eq!(m, mochila(&cat, &[("plano_2", 1), ("crisol", 5)]));
        assert_eq!(rng.estado(), Rng::nuevo(1).estado(), "no tira el generador");
    }

    #[test]
    fn forjar_rechaza_sin_plano_sin_crisoles_o_con_la_mochila_llena_sin_gastar_nada() {
        let cat = Catalogo::default();
        let mut rng = Rng::nuevo(1);
        let mut m = mochila(&cat, &[("plano_2", 1), ("crisol", 5)]);
        assert_eq!(forjar(&mut m, &cat, "dagas_bronce", 1, &mut rng, todo_abierto), Err(ErrorForja::SinPlano), "un plano de otro tier no vale");
        let mut m = mochila(&cat, &[("plano_3", 1), ("crisol", 2)]);
        assert_eq!(forjar(&mut m, &cat, "dagas_bronce", 1, &mut rng, todo_abierto), Err(ErrorForja::SinCrisoles));
        let mut m = mochila(&cat, &[("plano_3", 1)]);
        assert_eq!(forjar(&mut m, &cat, "dagas_bronce", 1, &mut rng, todo_abierto), Err(ErrorForja::SinCrisoles));
        assert_eq!(m, mochila(&cat, &[("plano_3", 1)]));
        let mut m = Inventario::vacio(3);
        for (o, n) in [("plano_3", 2), ("crisol", 4), ("dagas_inicio", 1)] {
            m.meter(&cat, Pieza::del_catalogo(o), n).unwrap();
        }
        let antes = m.clone();
        assert_eq!(forjar(&mut m, &cat, "dagas_bronce", 1, &mut rng, todo_abierto), Err(ErrorForja::Inventario(ErrorInventario::NoCabe)));
        assert_eq!((m, rng.estado()), (antes, Rng::nuevo(1).estado()));
    }
}
