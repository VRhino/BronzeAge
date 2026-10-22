//! El asedio de una plaza con números (Combate §6): el orden de las ramas y a quién hiere. Las consecuencias de que la plaza caiga (saqueo,
//! desalojo, derrota de Facción) están en `conquista`.
//!
//! # El multiplicador del recinto
//! Quien resuelve un asedio con números le da el **multiplicador del defensor**: el de la plaza (`asentamiento::recintos::multiplicador_defensivo`, Murallas §9.1) cuando se
//! asedia una plaza, y [`SIN_RECINTO`] (1,0) cuando no hay recinto que valga —la avanzada, cuya fortaleza son sus guarniciones de serie, K16—. El multiplicador es una
//! constante de la plaza en ese instante: no consume tirada, así que un asedio sin recinto es bit a bit el de antes.

use crate::datos::Reglas;
use crate::resolucion::{resolver_combate, Bando, Informe};
use asentamiento::datos::UNO;
use base::Rng;
use militar::HeroeId;

/// El multiplicador del defensor cuando no hay recinto que cuente (columna, caravana, avanzada o plaza sin muralla): 1,0 (ppm).
pub const SIN_RECINTO: i64 = UNO;

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
/// aunque sí suma su poder cuando lo hay. `multiplicador_defensor` (ppm) multiplica solo el poder del defensor, además de la ventaja del defensor.
pub fn resolver_asedio(r: &Reglas, protegida: bool, atacante: Bando, defensor: Bando, multiplicador_defensor: i64, rng: &mut Rng) -> Asedio {
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
    Asedio::Combate(resolver_combate(r, atacante, defensor, multiplicador_defensor, rng).expect("los dos bandos tienen escuadras con soldados"))
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
        assert_eq!(resolver_asedio(r, true, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut d, heroes: &[] }, SIN_RECINTO, &mut rng), Asedio::Protegida);
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
            let res = resolver_asedio(r, false, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: defensores, heroes: &[9] }, SIN_RECINTO, &mut rng);
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
        let res = resolver_asedio(r, false, Bando { escuadras: &mut [], heroes: &[1] }, Bando { escuadras: &mut d, heroes: &[] }, SIN_RECINTO, &mut Rng::nuevo(1));
        assert_eq!(res, Asedio::SinAtacantes);
        assert!(!res.conquistada());
        let res = resolver_asedio(r, false, Bando { escuadras: &mut [], heroes: &[1] }, Bando { escuadras: &mut [], heroes: &[] }, SIN_RECINTO, &mut Rng::nuevo(1));
        assert_eq!(res, Asedio::SinAtacantes, "«sin nadie con quien combatir»");
    }

    #[test]
    fn con_defensores_hay_combate_con_dos_tiradas_y_sin_recinto_el_multiplicador_es_uno() {
        let r = r();
        let (mut a, mut d) = ([escuadra(1, 1, Tropa::ArquerosCompuesto), escuadra(3, 1, Tropa::ArquerosCompuesto)], [escuadra(2, 2, Tropa::MiliciaLanceros)]);
        let mut rng = Rng::nuevo(6);
        let mut espejo = Rng::nuevo(6);
        espejo.siguiente();
        espejo.siguiente();
        let res = resolver_asedio(r, false, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut d, heroes: &[2] }, SIN_RECINTO, &mut rng);
        assert_eq!(rng.estado(), espejo.estado());
        let Asedio::Combate(i) = &res else { panic!("{res:?}") };
        assert_eq!(i.ganador, Lado::Atacante);
        assert!(res.conquistada());
        assert_eq!(SIN_RECINTO, 1_000_000);
    }

    #[test]
    fn el_multiplicador_del_recinto_solo_cambia_el_poder_del_defensor_y_con_uno_es_el_de_antes() {
        let r = r();
        let (a0, d0) = ([escuadra(1, 1, Tropa::ArquerosCompuesto)], [escuadra(2, 2, Tropa::MiliciaLanceros), escuadra(4, 2, Tropa::MiliciaLanceros)]);
        let resolver = |mult: i64| {
            let (mut a, mut d) = (a0.clone(), d0.clone());
            let mut rng = Rng::nuevo(6);
            let res = resolver_asedio(r, false, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut d, heroes: &[2] }, mult, &mut rng);
            let Asedio::Combate(i) = res else { panic!("{res:?}") };
            (i, rng.estado())
        };
        let (sin, estado_sin) = resolver(SIN_RECINTO);
        assert_eq!(sin.ganador, Lado::Atacante, "sin muralla cae");
        // Los ejemplos del documento (nivel 3, anillo entero): ×2,5 con una puerta, ×1,375 con cuatro y ×1,1875 con ocho.
        for (puertas, mult) in [(1, 2_500_000), (4, 1_375_000), (8, 1_187_500)] {
            let (con, estado_con) = resolver(mult);
            assert_eq!(con.atacante.poder, sin.atacante.poder, "{puertas} puertas: el atacante no cambia");
            let esperado = sin.defensor.poder as i128 * mult as i128 / UNO as i128;
            assert!((con.defensor.poder as i128 - esperado).abs() <= 3, "{puertas} puertas: el defensor vale ×{mult} (salvo el redondeo): {} contra {esperado}", con.defensor.poder);
            assert_eq!(estado_sin, estado_con, "el multiplicador no consume tirada: el generador queda en el mismo punto");
        }
        assert_eq!(resolver(2_500_000).0.ganador, Lado::Defensor, "con una sola puerta la misma plaza resiste");
    }

    #[test]
    fn si_cae_se_hieren_los_defensores_en_persona_y_si_resiste_todo_el_ejercito() {
        let r = r();
        let participantes = [1, 5, 6];
        let (mut a, mut d) = ([escuadra(1, 1, Tropa::ArquerosCompuesto), escuadra(3, 1, Tropa::ArquerosCompuesto)], [escuadra(2, 2, Tropa::MiliciaLanceros)]);
        let cae = resolver_asedio(r, false, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut d, heroes: &[2] }, SIN_RECINTO, &mut Rng::nuevo(6));
        assert_eq!(heridos_del_asedio(&cae, &participantes), vec![2]);
        let (mut a, mut d) = ([escuadra(1, 1, Tropa::MiliciaLanceros)], [escuadra(2, 2, Tropa::ArquerosCompuesto), escuadra(4, 2, Tropa::ArquerosCompuesto)]);
        let resiste = resolver_asedio(r, false, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut d, heroes: &[2] }, SIN_RECINTO, &mut Rng::nuevo(6));
        assert!(!resiste.conquistada());
        assert_eq!(heridos_del_asedio(&resiste, &participantes), vec![1, 5, 6], "también los que el tope apartó");
        for sin_combate in [Asedio::Protegida, Asedio::SinAtacantes, Asedio::CaeSinCombate] {
            assert!(heridos_del_asedio(&sin_combate, &participantes).is_empty(), "sin combate no pierde nadie");
        }
    }
}
