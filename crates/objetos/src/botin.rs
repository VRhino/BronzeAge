//! Tirar una tabla de `BOTIN_OBJETOS` (Objetos_Del_Heroe §3.6 y §3.7, D5 y D6) con el generador que pasa quien llama. Qué tabla toca (el nivel del
//! campamento, el rango del mercenario, si el héroe ganó la batalla) y adónde va lo que sale (la mochila y, si no cabe, el almacén de objetos; D17)
//! lo decide quien llama.

use crate::datos::{Entrada, Tabla};
use base::{Mil, Rng};

/// Uno de la lista, por sus probabilidades en milésimas (suman 1.000, comprobado al cargar): una sola tirada.
pub(crate) fn uno_de(lista: &[(String, Mil)], rng: &mut Rng) -> String {
    let mut tirada = rng.por_debajo(1000) as Mil;
    for (objeto, p) in lista {
        if tirada < *p {
            return objeto.clone();
        }
        tirada -= p;
    }
    unreachable!("las probabilidades suman 1.000 (comprobado al cargar)")
}

impl Tabla {
    /// Lo que sale, en el orden de la tabla: cada entrada fija se tira por su lado (una tirada en milésimas, también la que sale siempre) y las de
    /// `solo_si_gana` solo cuentan si `gana`; cada `unoDe` da uno, siempre. Devuelve el id del objeto y la cantidad.
    pub fn tirar(&self, gana: bool, rng: &mut Rng) -> Vec<(String, u32)> {
        let mut sale = Vec::new();
        for e in &self.0 {
            match e {
                Entrada::Fija { objeto, cantidad, probabilidad, solo_si_gana } => {
                    if (gana || !solo_si_gana) && (rng.por_debajo(1000) as Mil) < *probabilidad {
                        sale.push((objeto.clone(), *cantidad));
                    }
                }
                Entrada::UnoDe(lista) => sale.push((uno_de(lista, rng), 1)),
            }
        }
        sale
    }
}

#[cfg(test)]
mod pruebas {
    use crate::datos::Catalogo;
    use base::Rng;
    use std::collections::BTreeMap;

    #[test]
    fn lo_que_sale_siempre_sale_y_lo_de_ganar_solo_si_gana() {
        let cat = Catalogo::default();
        let mut rng = Rng::nuevo(3);
        assert_eq!(cat.botin.mision_diaria.tirar(false, &mut rng), vec![("crisol".into(), 1), ("medalla_tropa_pequena".into(), 1), ("medalla_heroe_muy_pequena".into(), 1)]);
        assert_eq!(cat.botin.rango_mercenario[&5].tirar(false, &mut rng), vec![("plano_5".into(), 1), ("medalla_heroe_grande".into(), 1), ("cofre_monturas".into(), 1)]);
        for _ in 0..200 {
            let pierde = cat.botin.unity.tirar(false, &mut rng);
            assert_eq!(pierde.iter().filter(|(o, _)| o == "crisol").count(), 1, "un crisol; el otro, solo si gana");
            assert!(!pierde.iter().any(|(o, _)| o.starts_with("medalla") || o == "cofre_monturas"));
            assert_eq!(cat.botin.unity.tirar(true, &mut rng).iter().filter(|(o, _)| o == "crisol").count(), 2);
        }
    }

    #[test]
    fn las_frecuencias_siguen_las_probabilidades() {
        let cat = Catalogo::default();
        let mut rng = Rng::nuevo(11);
        let mut veces: BTreeMap<String, u32> = BTreeMap::new();
        for _ in 0..10_000 {
            for (o, n) in cat.botin.bandidos[&3].tirar(false, &mut rng).into_iter().chain(cat.botin.alijo.tirar(false, &mut rng)) {
                *veces.entry(o).or_default() += n;
            }
        }
        let cerca = |o: &str, esperado: u32| {
            let v = veces.get(o).copied().unwrap_or(0);
            assert!(v.abs_diff(esperado) <= esperado / 10 + 30, "{o}: {v} veces, se esperaban unas {esperado}");
        };
        // Bandidos de nivel 3 (crisol 50 %, plano II 6 %, plano III 2 %, medallas 10 % y 5 %) más el alijo (3 crisoles, plano III 70 % o IV 30 %, medalla grande).
        cerca("crisol", 5_000 + 30_000);
        cerca("plano_2", 600);
        cerca("plano_3", 200 + 7_000);
        cerca("plano_4", 3_000);
        cerca("medalla_tropa_muy_pequena", 1_000);
        cerca("medalla_heroe_muy_pequena", 500);
        cerca("medalla_tropa_grande", 10_000);
    }
}
