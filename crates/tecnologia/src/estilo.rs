//! El estilo de época de una plaza (decisión del autor, 2026-11-16; Tecnologia_Aedas §4.2): el aspecto de sus edificios sale de las tecnologías que tiene adoptadas su
//! Facción. Es un par (Era, grado): la Era más alta de la que la Facción tiene adoptada al menos una tecnología y, de esa Era, cuántas (1 primeros atisbos, 2 más cambiada,
//! 3 del todo; nunca 0, que sería la Era anterior). La Era I es la base: siempre entera (grado 3). Cada plaza guarda el máximo al que llegó (lo lleva el núcleo).

use crate::datos::Reglas;
use crate::Conjunto;

/// El grado más alto de una Era: con tres tecnologías adoptadas de ella, la plaza ya luce del todo esa Era.
pub const GRADO_MAXIMO: u8 = 3;

/// El estilo de época: la Era (su `orden`, de 1 a 5) y el grado (1 a 3). Se compara por la Era y luego por el grado (el orden derivado de los campos).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Estilo {
    pub era: u8,
    pub grado: u8,
}

impl Estilo {
    /// La base: la Era I entera. Es el de toda Facción recién nacida (las de arranque son de la Era I) y el de una plaza al fundarse, antes de mirar su Facción.
    pub const BASE: Estilo = Estilo { era: 1, grado: GRADO_MAXIMO };

    /// ¿Es un estilo posible? La Era entre 1 y 5 (las del documento) y el grado entre 1 y 3; en la Era I, solo el 3.
    pub fn valido(self, r: &Reglas) -> bool {
        let eras = r.eras.iter().map(|e| e.orden).max().unwrap_or(1) as u8;
        (1..=eras).contains(&self.era) && (1..=GRADO_MAXIMO).contains(&self.grado) && (self.era > 1 || self.grado == GRADO_MAXIMO)
    }
}

/// El estilo que dan las tecnologías `adoptadas` de una Facción: la Era más alta con alguna adoptada y, de ella, `min(3, adoptadas de esa Era)`; en la Era I, 3. Sin ninguna, la base.
pub fn estilo_de(r: &Reglas, adoptadas: Conjunto) -> Estilo {
    let Some(era) = adoptadas.iter().map(|t| r.orden_era(t)).max() else { return Estilo::BASE };
    if era <= 1 {
        return Estilo::BASE;
    }
    let de_esa = adoptadas.iter().filter(|&t| r.orden_era(t) == era).count();
    Estilo { era: era as u8, grado: de_esa.min(usize::from(GRADO_MAXIMO)) as u8 }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::TecId;

    fn de_la_era(r: &Reglas, orden: u32) -> Vec<TecId> {
        (0..r.catalogo.len() as TecId).filter(|&t| r.orden_era(t) == orden && !r.def(t).de_arranque).collect()
    }

    fn con(base: Conjunto, ts: &[TecId]) -> Conjunto {
        let mut c = base;
        ts.iter().for_each(|&t| c.poner(t));
        c
    }

    #[test]
    fn solo_con_las_de_arranque_es_la_era_i_entera() {
        let r = Reglas::default();
        assert_eq!(estilo_de(&r, r.de_arranque), Estilo::BASE);
        assert_eq!(estilo_de(&r, Conjunto::VACIO), Estilo::BASE);
        // Más tecnologías de la Era I no cambian nada: la Era I ya es entera.
        assert_eq!(estilo_de(&r, con(r.de_arranque, &de_la_era(&r, 1))), Estilo::BASE);
    }

    #[test]
    fn el_grado_cuenta_las_adoptadas_de_la_era_mas_alta_hasta_tres() {
        let r = Reglas::default();
        let ii = de_la_era(&r, 2);
        assert!(ii.len() >= 4, "la Era II tiene al menos cuatro tecnologías");
        for k in 1..=4 {
            assert_eq!(estilo_de(&r, con(r.de_arranque, &ii[..k])), Estilo { era: 2, grado: k.min(3) as u8 }, "{k} de la Era II");
        }
    }

    #[test]
    fn manda_la_era_mas_alta_aunque_falten_las_de_en_medio() {
        let r = Reglas::default();
        let (ii, iii) = (de_la_era(&r, 2), de_la_era(&r, 3));
        // Tres de la II y una de la III: primeros atisbos de la III (la III va por delante aunque sea con una).
        let c = con(r.de_arranque, &[ii[0], ii[1], ii[2], iii[0]]);
        assert_eq!(estilo_de(&r, c), Estilo { era: 3, grado: 1 });
        // Saltarse una Era: solo una de la III, ninguna de la II.
        assert_eq!(estilo_de(&r, con(r.de_arranque, &[iii[0]])), Estilo { era: 3, grado: 1 });
    }

    #[test]
    fn se_compara_por_la_era_y_luego_por_el_grado() {
        assert!(Estilo { era: 3, grado: 1 } > Estilo { era: 2, grado: 3 });
        assert!(Estilo { era: 2, grado: 2 } > Estilo { era: 2, grado: 1 });
        assert!(Estilo { era: 2, grado: 1 } > Estilo::BASE);
        let r = Reglas::default();
        assert!(Estilo::BASE.valido(&r) && Estilo { era: 5, grado: 1 }.valido(&r));
        assert!(!Estilo { era: 1, grado: 1 }.valido(&r) && !Estilo { era: 2, grado: 0 }.valido(&r) && !Estilo { era: 6, grado: 1 }.valido(&r));
    }
}
