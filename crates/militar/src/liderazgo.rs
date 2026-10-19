//! Liderazgo (Doc 5.11, Heroe_Y_Tropa §5): cuánta tropa puede sacar a campaña un héroe. Es un límite de **salida**, no de posesión; el coste
//! sigue al **escalón** de la tropa (no a su poder) y **no mira cuántos soldados quedan**: una escuadra mermada paga el precio completo.
//! En un ejército cada héroe se valida contra **el suyo**, sin tope agregado.

use crate::catalogo::Tropa;
use crate::datos::Reglas;
use crate::escuadra::Escuadra;

/// Coste de Liderazgo de una tropa: el de su escalón.
pub fn coste(r: &Reglas, tropa: Tropa) -> i64 {
    r.liderazgo.coste_por_escalon[r.tropas.tropa(tropa).escalon as usize - 1]
}

/// Liderazgo que consumen estas escuadras juntas. Quien llama decide que son todas del mismo héroe.
pub fn comprometido<'a>(r: &Reglas, escuadras: impl IntoIterator<Item = &'a Escuadra>) -> i64 {
    escuadras.into_iter().map(|e| coste(r, e.tropa)).sum()
}

/// ¿Caben estas escuadras en este Liderazgo?
pub fn puede_llevar<'a>(r: &Reglas, liderazgo: i64, escuadras: impl IntoIterator<Item = &'a Escuadra>) -> bool {
    comprometido(r, escuadras) <= liderazgo
}

/// Lo que queda libre (nunca negativo), para que la interfaz muestre «38/100» sin reimplementar la resta.
pub fn disponible<'a>(r: &Reglas, liderazgo: i64, escuadras: impl IntoIterator<Item = &'a Escuadra>) -> i64 {
    (liderazgo - comprometido(r, escuadras)).max(0)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn esc(r: &Reglas, id: u64, tropa: Tropa, cantidad: i64) -> Escuadra {
        Escuadra { cantidad, ..Escuadra::nueva(r, id, 1, tropa, 1, None, 0) }
    }

    /// Una escuadra de cada escalón: leva (leñadores), línea (honderos), veterana (arqueros), pesada (lanceros pesados), élite (hequetai).
    fn mezcla(r: &Reglas, escalones: &[u8]) -> Vec<Escuadra> {
        let por_escalon = [Tropa::Lenadores, Tropa::Honderos, Tropa::Arqueros, Tropa::LancerosPesados, Tropa::Hequetai];
        escalones.iter().enumerate().map(|(i, &e)| esc(r, i as u64, por_escalon[e as usize - 1], 10)).collect()
    }

    #[test]
    fn el_coste_sigue_al_escalon_y_crece() {
        let r = Reglas::cargar();
        let costes: Vec<_> = [Tropa::Lenadores, Tropa::MiliciaLanceros, Tropa::Arqueros, Tropa::Shardana, Tropa::ArquerosEscitas].map(|t| coste(&r, t)).to_vec();
        assert_eq!(costes, vec![7, 14, 22, 32, 45]);
        // El poder no cuenta: el Mineros y los Granjeros (mismo escalón, mismo poder) y dos tropas de poder distinto en el mismo escalón.
        assert_eq!(coste(&r, Tropa::Mineros), coste(&r, Tropa::Granjeros));
        assert_eq!(coste(&r, Tropa::Hequetai), coste(&r, Tropa::ArquerosCompuesto));
        assert!(r.tropas.tropa(Tropa::Hequetai).poder_base != r.tropas.tropa(Tropa::ArquerosCompuesto).poder_base);
    }

    #[test]
    fn las_mezclas_del_canon_quedan_justo_en_el_techo() {
        let r = Reglas::cargar();
        let l = r.liderazgo.base;
        // 1 élite + 1 pesada + 1 veterana = 99; 2 pesadas + 1 veterana + 1 de línea = 100; 3 veteranas + 1 pesada = 98; 2 élites + 1 leva = 97.
        for (escalones, total) in [(&[5u8, 4, 3][..], 99), (&[4, 4, 3, 2], 100), (&[3, 3, 3, 4], 98), (&[5, 5, 1], 97)] {
            let m = mezcla(&r, escalones);
            assert_eq!(comprometido(&r, &m), total, "{escalones:?}");
            assert!(puede_llevar(&r, l, &m));
        }
        // «Ninguna admite una unidad más»: con una leva más (o una de línea, la de 97) se pasan del techo.
        for (escalones, mas) in [(&[5u8, 4, 3][..], 1u8), (&[4, 4, 3, 2], 1), (&[3, 3, 3, 4], 1), (&[5, 5, 1], 2)] {
            let mut e = escalones.to_vec();
            e.push(mas);
            assert!(!puede_llevar(&r, l, &mezcla(&r, &e)), "{e:?}");
        }
        // La hueste de leva: 14 escuadras caben y 15 no.
        assert!(puede_llevar(&r, l, &mezcla(&r, &[1; 14])));
        assert!(!puede_llevar(&r, l, &mezcla(&r, &[1; 15])));
    }

    #[test]
    fn una_escuadra_mermada_paga_el_liderazgo_completo() {
        let r = Reglas::cargar();
        let entera = esc(&r, 1, Tropa::Hequetai, 12);
        let mermada = esc(&r, 2, Tropa::Hequetai, 3);
        let aniquilada = esc(&r, 3, Tropa::Hequetai, 0);
        assert_eq!(comprometido(&r, [&entera]), comprometido(&r, [&mermada]));
        assert_eq!(comprometido(&r, [&aniquilada]), 45, "ni siquiera una vacía sale gratis");
    }

    #[test]
    fn lo_libre_nunca_es_negativo_y_el_limite_es_de_salida() {
        let r = Reglas::cargar();
        let m = mezcla(&r, &[5, 5, 5]); // 135
        assert!(!puede_llevar(&r, 100, &m));
        assert_eq!(disponible(&r, 100, &m), 0);
        assert_eq!(disponible(&r, 100, &mezcla(&r, &[5, 4])), 23);
        assert!(puede_llevar(&r, 150, &m), "con más Liderazgo cabe");
        assert!(puede_llevar(&r, 0, &[]), "sin escuadras siempre se puede salir");
        assert_eq!(disponible(&r, 100, &[]), 100);
    }
}
