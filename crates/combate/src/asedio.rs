//! El asedio de una plaza con números (Combate §6): el orden de las ramas y a quién hiere. Las consecuencias de que la plaza caiga (saqueo,
//! desalojo, derrota de Facción) están en `conquista`.
//!
//! Las **murallas son una fase propia** posterior al sistema militar (decisión del autor, 2026-10-19): el recinto es 0, el multiplicador del
//! defensor vale 1 ([`SIN_MURALLA`]) y no hay cupo de muralla en la guarnición.

use crate::datos::Reglas;
use crate::resolucion::{resolver_combate, Bando, Informe};
use asentamiento::datos::UNO;
use base::Rng;
use militar::HeroeId;

/// El multiplicador del defensor sin recintos: 1,0 (ppm).
pub const SIN_MURALLA: i64 = UNO;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asedio {
    /// La plaza fue conquistada hace menos de `proteccionMinutos`: rebota sin combate ni tirada.
    Protegida,
    /// Sin atacantes con soldados: «sin nadie con quien combatir», no cae.
    SinAtacantes,
    /// Sin defensores con soldados: cae sin combate y sin tirada.
    CaeSinCombate,
    /// Combate con números; la plaza cae si gana el atacante.
    Combate(Informe),
}

impl Asedio {
    pub fn conquistada(&self) -> bool {
        match self {
            Asedio::CaeSinCombate => true,
            Asedio::Combate(i) => i.ganador == crate::resolucion::Lado::Atacante,
            Asedio::Protegida | Asedio::SinAtacantes => false,
        }
    }
}

/// Resuelve un asedio. Las ramas sin combate **no consumen aleatoriedad**: una partida sin asedios hace las mismas llamadas al generador que
/// antes de existir los asedios. Cuentan los defensores y atacantes **con soldados**: un héroe sin escuadras no basta para abrir el combate,
/// aunque sí suma su poder cuando lo hay.
pub fn resolver_asedio(r: &Reglas, protegida: bool, atacante: Bando, defensor: Bando, rng: &mut Rng) -> Asedio {
    if protegida {
        return Asedio::Protegida;
    }
    let con_soldados = |b: &Bando| b.escuadras.iter().any(|e| e.con_soldados());
    if !con_soldados(&atacante) {
        return Asedio::SinAtacantes;
    }
    if !con_soldados(&defensor) {
        return Asedio::CaeSinCombate;
    }
    Asedio::Combate(resolver_combate(r, atacante, defensor, SIN_MURALLA, rng).expect("los dos bandos tienen escuadras con soldados"))
}

/// Quiénes quedan heridos tras un asedio: si hubo combate real, los **defensores en persona** si la plaza cae, o **todos los participantes del
/// ejército** si resiste (la derrota es de la columna, hayan entrado o no a la batalla). Sin combate, nadie.
pub fn heridos_del_asedio(resultado: &Asedio, participantes_del_ejercito: &[HeroeId]) -> Vec<HeroeId> {
    match resultado {
        Asedio::Combate(i) if resultado.conquistada() => i.defensor.heroes.clone(),
        Asedio::Combate(_) => participantes_del_ejercito.to_vec(),
        _ => vec![],
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::resolucion::apoyo::*;
    use crate::resolucion::Lado;
    use militar::Tropa;

    #[test]
    fn una_plaza_protegida_rebota_sin_tirada() {
        let r = r();
        let (mut a, mut d) = ([escuadra(1, 1, Tropa::Shardana)], [escuadra(2, 2, Tropa::MiliciaLanceros)]);
        let (copia_a, copia_d) = (a.clone(), d.clone());
        let mut rng = Rng::nuevo(1);
        let antes = rng.estado();
        assert_eq!(resolver_asedio(r, true, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut d, heroes: &[] }, &mut rng), Asedio::Protegida);
        assert_eq!(rng.estado(), antes);
        assert_eq!((a.to_vec(), d.to_vec()), (copia_a.to_vec(), copia_d.to_vec()));
    }

    #[test]
    fn sin_defensores_con_soldados_cae_sin_combate_ni_tirada() {
        let r = r();
        let mut a = [escuadra(1, 1, Tropa::MiliciaLanceros)];
        let mut aniquilada = [con_cantidad(escuadra(2, 2, Tropa::MiliciaLanceros), 0)];
        for defensores in [&mut aniquilada[..], &mut []] {
            let mut rng = Rng::nuevo(1);
            let antes = rng.estado();
            let res = resolver_asedio(r, false, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: defensores, heroes: &[9] }, &mut rng);
            assert_eq!(res, Asedio::CaeSinCombate);
            assert!(res.conquistada());
            assert_eq!(rng.estado(), antes, "una rama sin combate no toca el generador");
        }
        assert_eq!(a[0].cantidad, 25);
    }

    #[test]
    fn sin_atacantes_con_soldados_no_cae_aunque_no_haya_defensa() {
        let r = r();
        let mut d = [escuadra(2, 2, Tropa::MiliciaLanceros)];
        let res = resolver_asedio(r, false, Bando { escuadras: &mut [], heroes: &[1] }, Bando { escuadras: &mut d, heroes: &[] }, &mut Rng::nuevo(1));
        assert_eq!(res, Asedio::SinAtacantes);
        assert!(!res.conquistada());
        let res = resolver_asedio(r, false, Bando { escuadras: &mut [], heroes: &[1] }, Bando { escuadras: &mut [], heroes: &[] }, &mut Rng::nuevo(1));
        assert_eq!(res, Asedio::SinAtacantes, "«sin nadie con quien combatir»");
    }

    #[test]
    fn con_defensores_hay_combate_con_dos_tiradas_y_muralla_uno() {
        let r = r();
        let (mut a, mut d) = ([escuadra(1, 1, Tropa::ArquerosCompuesto), escuadra(3, 1, Tropa::ArquerosCompuesto)], [escuadra(2, 2, Tropa::MiliciaLanceros)]);
        let mut rng = Rng::nuevo(6);
        let mut espejo = Rng::nuevo(6);
        espejo.siguiente();
        espejo.siguiente();
        let res = resolver_asedio(r, false, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut d, heroes: &[2] }, &mut rng);
        assert_eq!(rng.estado(), espejo.estado());
        let Asedio::Combate(i) = &res else { panic!("{res:?}") };
        assert_eq!(i.ganador, Lado::Atacante);
        assert!(res.conquistada());
        assert_eq!(SIN_MURALLA, 1_000_000);
    }

    #[test]
    fn si_cae_se_hieren_los_defensores_en_persona_y_si_resiste_todo_el_ejercito() {
        let r = r();
        let participantes = [1, 5, 6];
        let (mut a, mut d) = ([escuadra(1, 1, Tropa::ArquerosCompuesto), escuadra(3, 1, Tropa::ArquerosCompuesto)], [escuadra(2, 2, Tropa::MiliciaLanceros)]);
        let cae = resolver_asedio(r, false, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut d, heroes: &[2] }, &mut Rng::nuevo(6));
        assert_eq!(heridos_del_asedio(&cae, &participantes), vec![2]);
        let (mut a, mut d) = ([escuadra(1, 1, Tropa::MiliciaLanceros)], [escuadra(2, 2, Tropa::ArquerosCompuesto), escuadra(4, 2, Tropa::ArquerosCompuesto)]);
        let resiste = resolver_asedio(r, false, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut d, heroes: &[2] }, &mut Rng::nuevo(6));
        assert!(!resiste.conquistada());
        assert_eq!(heridos_del_asedio(&resiste, &participantes), vec![1, 5, 6], "también los que el tope apartó");
        for sin_combate in [Asedio::Protegida, Asedio::SinAtacantes, Asedio::CaeSinCombate] {
            assert!(heridos_del_asedio(&sin_combate, &participantes).is_empty(), "sin combate no pierde nadie");
        }
    }
}
