//! Usar un consumible de la mochila (`usar_objeto`, Objetos_Del_Heroe §3.5): gasta uno y devuelve su efecto, que aplica quien llama. Dónde se puede
//! usar (D2) y si el efecto cabe (una escuadra en el nivel máximo no puede usar una medalla de tropa) lo comprueba quien llama antes.

use crate::botin::uno_de;
use crate::datos::{Catalogo, Consumo, Tipo};
use crate::inventario::{ErrorInventario, Inventario};
use base::Rng;

/// El efecto de un consumible usado.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Efecto {
    /// Experiencia para una escuadra del héroe.
    ExperienciaTropa(i64),
    /// Este porcentaje de la experiencia que pide el nivel del héroe.
    ExperienciaHeroePct(i64),
    /// La montura (id del catálogo) que sale del cofre, al 100 %.
    Montura(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorUso {
    /// Al sacar de la mochila (una casilla vacía o fuera de rango).
    Inventario(ErrorInventario),
    /// El objeto no está en el catálogo.
    Desconocido,
    /// No es un consumible.
    NoConsumible,
}

impl From<ErrorInventario> for ErrorUso {
    fn from(e: ErrorInventario) -> Self {
        ErrorUso::Inventario(e)
    }
}

/// Usa uno del consumible de la casilla `casilla`. El cofre de monturas tira cuál con el generador que pasa quien llama.
pub fn usar(mochila: &mut Inventario, cat: &Catalogo, casilla: u32, rng: &mut Rng) -> Result<Efecto, ErrorUso> {
    let objeto = &mochila.casillas.get(&casilla).ok_or(ErrorInventario::SinCasilla)?.pieza.objeto;
    let Tipo::Consumible(consumo) = &cat.objetos.get(objeto).ok_or(ErrorUso::Desconocido)?.tipo else { return Err(ErrorUso::NoConsumible) };
    mochila.sacar(casilla, 1)?;
    Ok(match consumo {
        Consumo::ExperienciaTropa(x) => Efecto::ExperienciaTropa(*x),
        Consumo::ExperienciaHeroePct(x) => Efecto::ExperienciaHeroePct(*x),
        Consumo::MonturaDe(lista) => Efecto::Montura(uno_de(lista, rng)),
    })
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::inventario::{Casilla, Pieza};
    use std::collections::BTreeMap;

    #[test]
    fn una_medalla_da_su_experiencia_y_se_gasta() {
        let cat = Catalogo::default();
        let mut m = Inventario::vacio(cat.casillas_mochila);
        m.meter(&cat, Pieza::del_catalogo("medalla_tropa_grande"), 2).unwrap();
        m.meter(&cat, Pieza::del_catalogo("medalla_heroe_pequena"), 1).unwrap();
        let mut rng = Rng::nuevo(1);
        assert_eq!(usar(&mut m, &cat, 0, &mut rng), Ok(Efecto::ExperienciaTropa(50)));
        assert_eq!(m.casillas[&0].cantidad, 1);
        assert_eq!(usar(&mut m, &cat, 1, &mut rng), Ok(Efecto::ExperienciaHeroePct(10)));
        assert_eq!(m.casillas.keys().copied().collect::<Vec<_>>(), vec![0]);
        assert_eq!(rng.estado(), Rng::nuevo(1).estado(), "una medalla no tira el generador");
    }

    #[test]
    fn el_cofre_da_una_montura_de_tier_iii_a_v_por_sus_probabilidades() {
        let cat = Catalogo::default();
        let mut rng = Rng::nuevo(5);
        let mut veces: BTreeMap<String, u32> = BTreeMap::new();
        for _ in 0..10_000 {
            let mut m = Inventario::vacio(1);
            m.meter(&cat, Pieza::del_catalogo("cofre_monturas"), 1).unwrap();
            let Ok(Efecto::Montura(o)) = usar(&mut m, &cat, 0, &mut rng) else { panic!("una montura") };
            assert!(m.casillas.is_empty());
            *veces.entry(o).or_default() += 1;
        }
        assert_eq!(veces.keys().map(String::as_str).collect::<Vec<_>>(), vec!["montura_escita", "montura_nisea", "montura_tesalia"]);
        assert!(veces["montura_escita"].abs_diff(7_000) < 300 && veces["montura_tesalia"].abs_diff(2_500) < 250 && veces["montura_nisea"].abs_diff(500) < 100, "{veces:?}");
    }

    #[test]
    fn usar_rechaza_lo_que_no_es_consumible_la_casilla_que_no_hay_y_lo_desconocido() {
        let cat = Catalogo::default();
        let mut m = Inventario::vacio(cat.casillas_mochila);
        m.meter(&cat, Pieza::del_catalogo("crisol"), 1).unwrap();
        m.meter(&cat, Pieza::del_catalogo("montura_estepa"), 1).unwrap();
        m.casillas.insert(2, Casilla { pieza: Pieza::del_catalogo("pocion"), cantidad: 1 });
        let mut rng = Rng::nuevo(1);
        assert_eq!(usar(&mut m, &cat, 0, &mut rng), Err(ErrorUso::NoConsumible));
        assert_eq!(usar(&mut m, &cat, 1, &mut rng), Err(ErrorUso::NoConsumible));
        assert_eq!(usar(&mut m, &cat, 2, &mut rng), Err(ErrorUso::Desconocido));
        assert_eq!(usar(&mut m, &cat, 3, &mut rng), Err(ErrorUso::Inventario(ErrorInventario::SinCasilla)));
        assert_eq!(m.casillas.len(), 3);
    }
}
