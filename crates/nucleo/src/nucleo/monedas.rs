//! Las monedas del héroe por combatir contra héroes de otra Facción (tanda 69, Economia_Heroe §4.2; `MONEDAS` de Combate). Cada héroe ganador cobra `ganador` y cada perdedor `perdedor`,
//! con rendimientos decrecientes por **par de Facciones** (`EstadoCombate::encuentros`): el factor de un par se calcula una vez por combate. Contra bandidos, entre héroes de la misma
//! Facción o con un héroe sin Facción no se cobra nada (el diseño pide dos Facciones distintas).

use super::*;
use std::collections::BTreeSet;
use combate::resolucion::Informe;

impl Nucleo {
    /// Las monedas de un combate con números: los héroes de cada bando que combatieron, con la Facción que tienen.
    pub(super) fn monedas_de_informe(&mut self, t: Instante, informe: &Informe) {
        let con_faccion = |heroes: &[u64]| -> Vec<(u64, u32)> { heroes.iter().filter_map(|&h| Some((h, self.heroe(h)?.faccion?))).collect() };
        let (ganadores, perdedores) = (con_faccion(&informe.bando(informe.ganador).heroes), con_faccion(&informe.bando(informe.perdedor()).heroes));
        self.monedas_de_combate(t, &ganadores, &perdedores);
    }

    /// Las monedas de un combate: `ganadores` y `perdedores` son (héroe, Facción con la que combatió); cada uno cobra su cuantía por el factor de rendimiento de su par de Facciones con el bando contrario; si tiene varios pares, el mejor.
    pub(super) fn monedas_de_combate(&mut self, t: Instante, ganadores: &[(u64, u32)], perdedores: &[(u64, u32)]) {
        let r = self.estado.reglas.economia.combate.monedas.clone();
        // El factor de cada par, una vez por combate.
        let factores: BTreeMap<(u32, u32), i64> = pares_de_facciones(ganadores, perdedores)
            .into_iter()
            .map(|par| (par, combate::bandidos::factor_de_rendimiento(&r.rendimientos, self.estado.combate.encuentros.entry(par).or_default(), t)))
            .collect();
        for (heroes, cuantia) in [(ganadores, r.ganador), (perdedores, r.perdedor)] {
            for &(h, f) in heroes {
                let Some(factor) = factores.iter().filter(|(&(a, b), _)| a == f || b == f).map(|(_, &x)| x).max() else { continue };
                let monedas = (cuantia as i128 * factor as i128 / asentamiento::datos::UNO as i128) as i64;
                if monedas > 0 {
                    let h = &mut self.heroe_mut(h).heroe;
                    h.monedas = h.monedas.saturating_add(monedas);
                }
            }
        }
    }
}

/// Los pares de Facciones distintas (la menor primero) que se enfrentan: una de cada bando.
fn pares_de_facciones(ganadores: &[(u64, u32)], perdedores: &[(u64, u32)]) -> BTreeSet<(u32, u32)> {
    let enfrentadas = ganadores.iter().flat_map(|&(_, a)| perdedores.iter().map(move |&(_, b)| (a, b)));
    enfrentadas.filter(|(a, b)| a != b).map(|(a, b)| (a.min(b), a.max(b))).collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn solo_se_enfrentan_facciones_distintas_y_cada_par_cuenta_una_vez() {
        // Dos héroes de la Facción 3 contra uno de la 1 y otro de la 3: un solo par, (1, 3), y nada entre los de la misma Facción.
        assert_eq!(pares_de_facciones(&[(1, 3), (2, 3)], &[(3, 1), (4, 3)]), BTreeSet::from([(1, 3)]));
        assert!(pares_de_facciones(&[(1, 2), (3, 2)], &[(4, 2)]).is_empty());
        assert_eq!(pares_de_facciones(&[(1, 5)], &[(2, 7), (3, 2)]), BTreeSet::from([(2, 5), (5, 7)]));
    }
}
