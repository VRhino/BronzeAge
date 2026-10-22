//! El ataque a una avanzada (Combate §7.4, Doc 5.17): lo abre solo un ejército, dura 30 minutos como un asedio (capacidad de 15 héroes) y exige
//! guerra o la declara. Aquí van la invulnerabilidad de las ruinas, qué queda de la avanzada según gane o no el atacante, la guarnición de serie
//! de cada etapa y el saqueo de su almacén; las etapas, el estado y la obra de mapa (colocar, mejorar, reparar, destruir a mano) son del crate
//! `asentamiento` (`avanzadas`), y el núcleo las une con la guarnición de jugadores.
//!
//! - **Atalaya**: si gana el atacante, destruida.
//! - **Fuerte**: en ruinas, con un día de invulnerabilidad; si pasado ese día sigue en ruinas, lo atacan y gana el atacante, destruido.
//! - **Ciudadela**: en ruinas, **nunca se destruye**.
//!
//! **En ruinas no funciona nada** (ni visión, ni guarnición, ni reabastecimiento): una avanzada en ruinas no tiene defensores.

use crate::asedio::{resolver_asedio, Asedio, SIN_RECINTO};
use crate::botin::{repartir_botin, CarroDeColumna};
use crate::datos::Reglas;
use crate::resolucion::{Bando, SIN_DUENO};
pub use asentamiento::avanzadas::{Estado, Etapa};
use asentamiento::recurso::TODOS;
use asentamiento::Almacen;
use base::{Instante, Rng};
use militar::escuadra::Escuadra;
use militar::{total, Carga, EscuadraId, Tropa};
use std::collections::BTreeMap;

/// ¿Se puede abrir un ataque ahora? No contra una obra que aún no es un puesto, ni contra una destruida, ni contra unas ruinas invulnerables.
pub fn atacable(estado: Estado, ahora: Instante) -> bool {
    match estado {
        Estado::Operativa => true,
        Estado::EnRuinas { invulnerable_hasta } => invulnerable_hasta.is_none_or(|h| ahora >= h),
        Estado::EnConstruccion | Estado::Destruida => false,
    }
}

/// Resuelve el ataque. Rebota (`Protegida`) si no es atacable; unas ruinas no tienen defensores y caen sin combate ni tirada; una operativa se
/// resuelve como un asedio (`defensor`: su guarnición de serie y la de jugadores, y los héroes que la defienden).
pub fn resolver_ataque(r: &Reglas, estado: Estado, ahora: Instante, atacante: Bando, defensor: Bando, rng: &mut Rng) -> Asedio {
    let rebota = !atacable(estado, ahora);
    match estado {
        Estado::EnRuinas { .. } if !rebota => {
            if atacante.escuadras.iter().any(|e| e.con_soldados()) {
                Asedio::CaeSinCombate
            } else {
                Asedio::SinAtacantes
            }
        }
        // Una avanzada no tiene recintos (su fortaleza son sus guarniciones de serie, Combate §7.4): multiplicador 1.
        _ => resolver_asedio(r, rebota, atacante, defensor, SIN_RECINTO, rng),
    }
}

/// Las tropas de la guarnición de serie de una etapa, una por escuadra y por orden de escalón: la que el Gobernador eligió para cada escalón o, si no
/// eligió, `tropaPorDefecto`, la del escalón 2 del canon (milicia de lanceros) en los tres escalones.
pub fn composicion(r: &Reglas, etapa: Etapa, elegidas: &BTreeMap<u8, Tropa>) -> Vec<Tropa> {
    let a = &r.avanzada;
    a.guarnicion_de_serie[etapa.indice()].iter().flat_map(|&(escalon, n)| std::iter::repeat_n(elegidas.get(&escalon).copied().unwrap_or(a.tropa_por_defecto), n)).collect()
}

/// La guarnición de serie de una etapa, completa y sin dueño (IA, fuera de cupo). `nuevo_id` da el id de cada escuadra nueva.
pub fn guarnicion_de_serie(r: &Reglas, etapa: Etapa, elegidas: &BTreeMap<u8, Tropa>, mut nuevo_id: impl FnMut() -> EscuadraId, ahora: Instante) -> Vec<Escuadra> {
    composicion(r, etapa, elegidas).into_iter().map(|t| Escuadra::nueva(&r.militar, nuevo_id(), SIN_DUENO, t, r.militar.tropas.tropa(t).unidades, None, ahora)).collect()
}

/// Cuándo se repone gratis la guarnición de serie que cayó.
pub fn reposicion_en(r: &Reglas, cayo_en: Instante) -> Instante {
    cayo_en + r.avanzada.reposicion
}

/// ¿Le falta gente a la guarnición de serie respecto de su número de serie?
pub fn incompleta(r: &Reglas, guarnicion: &[Escuadra]) -> bool {
    guarnicion.iter().any(|e| e.cantidad < r.militar.tropas.tropa(e.tropa).unidades)
}

/// Repone la guarnición de serie: cada escuadra de nuevo completa, con su identidad. La experiencia y el nivel que ganó se conservan.
pub fn reponer(r: &Reglas, guarnicion: &mut [Escuadra]) {
    guarnicion.iter_mut().for_each(|e| e.cantidad = r.militar.tropas.tropa(e.tropa).unidades);
}

/// El tope de la guarnición de jugadores de un héroe en la etapa, en puntos de Liderazgo.
pub fn cupo_de_guarnicion(r: &Reglas, etapa: Etapa) -> i64 {
    r.avanzada.cupo_guarnicion[etapa.indice()]
}

/// ¿Puede el Gobernador elegir esta tropa para la guarnición de serie? Solo los escalones que usa alguna etapa (el 2, el 3 y el 4); devuelve su escalón. Que la
/// plaza madre pueda reclutarla (tecnología y edificio) lo mira el núcleo.
pub fn escalon_elegible(r: &Reglas, tropa: Tropa) -> Option<u8> {
    let escalon = r.militar.tropas.tropa(tropa).escalon;
    r.avanzada.guarnicion_de_serie.iter().flatten().any(|&(e, _)| e == escalon).then_some(escalon)
}

/// El atacante que gana se lleva del almacén **lo que le quepa en los carros** (Doc 5.17), recurso a recurso y por rondas entre sus columnas; quita del
/// almacén solo lo que se llevan. Devuelve lo llevado.
pub fn saquear(almacen: &mut Almacen, carros: &mut [CarroDeColumna]) -> Carga {
    let botin: Carga = TODOS.iter().map(|&rec| (rec, almacen.cantidad(rec))).filter(|&(_, n)| n > 0).collect();
    let perdido = repartir_botin(carros, &botin);
    let mut llevado = Carga::new();
    for (&rec, &n) in &botin {
        let x = n - perdido.get(&rec).copied().unwrap_or(0);
        if x > 0 {
            almacen.quitar(rec, x);
            llevado.insert(rec, x);
        }
    }
    debug_assert!(total(&llevado) <= total(&botin));
    llevado
}

/// Qué queda de la avanzada tras el ataque. Si el atacante no gana, nada cambia.
pub fn tras_ataque(r: &Reglas, etapa: Etapa, estado: Estado, ahora: Instante, gana_atacante: bool) -> Estado {
    if !gana_atacante || matches!(estado, Estado::Destruida | Estado::EnConstruccion) {
        return estado;
    }
    match (etapa, estado) {
        (Etapa::Atalaya, _) => Estado::Destruida,
        (Etapa::Fuerte, Estado::Operativa) => Estado::EnRuinas { invulnerable_hasta: Some(ahora + r.avanzada.invulnerabilidad) },
        (Etapa::Fuerte, _) => Estado::Destruida,
        (Etapa::Ciudadela, _) => Estado::EnRuinas { invulnerable_hasta: None },
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::resolucion::apoyo::*;
    use base::MS_POR_MINUTO;
    use militar::Tropa;

    const DIA: i64 = 1440 * MS_POR_MINUTO;

    #[test]
    fn la_atalaya_cae_destruida_si_gana_el_atacante() {
        let r = r();
        assert_eq!(tras_ataque(r, Etapa::Atalaya, Estado::Operativa, 5, true), Estado::Destruida);
        assert_eq!(tras_ataque(r, Etapa::Atalaya, Estado::Operativa, 5, false), Estado::Operativa);
    }

    #[test]
    fn el_fuerte_queda_en_ruinas_con_un_dia_de_invulnerabilidad_y_luego_se_destruye() {
        let r = r();
        let ruinas = tras_ataque(r, Etapa::Fuerte, Estado::Operativa, 1_000, true);
        assert_eq!(ruinas, Estado::EnRuinas { invulnerable_hasta: Some(1_000 + DIA) });
        assert!(!atacable(ruinas, 1_000 + DIA - 1));
        assert!(atacable(ruinas, 1_000 + DIA), "pasado el día vuelve a poder atacarse");
        assert_eq!(tras_ataque(r, Etapa::Fuerte, ruinas, 1_000 + DIA, true), Estado::Destruida);
        assert_eq!(tras_ataque(r, Etapa::Fuerte, ruinas, 1_000 + DIA, false), ruinas);
    }

    #[test]
    fn la_ciudadela_queda_en_ruinas_y_nunca_se_destruye() {
        let r = r();
        let ruinas = tras_ataque(r, Etapa::Ciudadela, Estado::Operativa, 7, true);
        assert_eq!(ruinas, Estado::EnRuinas { invulnerable_hasta: None });
        assert_eq!(tras_ataque(r, Etapa::Ciudadela, ruinas, 8, true), ruinas, "atacar unas ruinas de ciudadela no la destruye");
    }

    #[test]
    fn una_destruida_no_se_ataca_ni_cambia() {
        let r = r();
        assert!(!atacable(Estado::Destruida, 0));
        for etapa in [Etapa::Atalaya, Etapa::Fuerte, Etapa::Ciudadela] {
            assert_eq!(tras_ataque(r, etapa, Estado::Destruida, 0, true), Estado::Destruida);
        }
    }

    #[test]
    fn unas_ruinas_invulnerables_rebotan_y_las_demas_caen_sin_combate_ni_tirada() {
        let r = r();
        let mut a = [escuadra(1, 1, Tropa::MiliciaLanceros)];
        let mut rng = Rng::nuevo(1);
        let antes = rng.estado();
        let invulnerable = Estado::EnRuinas { invulnerable_hasta: Some(100) };
        assert_eq!(resolver_ataque(r, invulnerable, 99, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut [], heroes: &[] }, &mut rng), Asedio::Protegida);
        assert_eq!(resolver_ataque(r, invulnerable, 100, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut [], heroes: &[] }, &mut rng), Asedio::CaeSinCombate);
        assert_eq!(resolver_ataque(r, Estado::EnRuinas { invulnerable_hasta: None }, 0, Bando { escuadras: &mut [], heroes: &[1] }, Bando { escuadras: &mut [], heroes: &[] }, &mut rng), Asedio::SinAtacantes);
        assert_eq!(resolver_ataque(r, Estado::Destruida, 0, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut [], heroes: &[] }, &mut rng), Asedio::Protegida);
        assert_eq!(rng.estado(), antes);
    }

    #[test]
    fn una_obra_pendiente_rebota_sin_tirada_y_no_cambia() {
        let r = r();
        let mut a = [escuadra(1, 1, Tropa::MiliciaLanceros)];
        let mut rng = Rng::nuevo(1);
        let antes = rng.estado();
        assert!(!atacable(Estado::EnConstruccion, 0));
        assert_eq!(resolver_ataque(r, Estado::EnConstruccion, 0, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut [], heroes: &[] }, &mut rng), Asedio::Protegida);
        assert_eq!(rng.estado(), antes);
        assert_eq!(tras_ataque(r, Etapa::Atalaya, Estado::EnConstruccion, 0, true), Estado::EnConstruccion);
    }

    fn tropas(v: &[Tropa]) -> Vec<&'static str> {
        v.iter().map(|t| t.nombre()).collect()
    }

    #[test]
    fn la_guarnicion_de_serie_sale_de_la_tabla_del_canon_con_el_defecto_o_lo_que_elige_el_gobernador() {
        let r = r();
        let sin_elegir = BTreeMap::new();
        let milicias = |n: usize| vec![Tropa::MiliciaLanceros; n];
        assert_eq!(composicion(r, Etapa::Atalaya, &sin_elegir), milicias(1));
        assert_eq!(composicion(r, Etapa::Fuerte, &sin_elegir), milicias(5), "3 del escalón 2 y 2 del 3");
        assert_eq!(composicion(r, Etapa::Ciudadela, &sin_elegir), milicias(8), "5 del escalón 3 y 3 del 4");
        let elegidas = BTreeMap::from([(2, Tropa::LancerosMimbre), (4, Tropa::Shardana)]);
        assert_eq!(tropas(&composicion(r, Etapa::Fuerte, &elegidas)), ["lanceros_mimbre"; 3].into_iter().chain(["milicia_lanceros"; 2]).collect::<Vec<_>>());
        assert_eq!(tropas(&composicion(r, Etapa::Ciudadela, &elegidas)), ["milicia_lanceros"; 5].into_iter().chain(["shardana"; 3]).collect::<Vec<_>>());
    }

    #[test]
    fn la_guarnicion_nace_completa_sin_dueno_se_repone_gratis_y_conserva_su_experiencia() {
        let r = r();
        let mut id = 100;
        let mut g = guarnicion_de_serie(r, Etapa::Fuerte, &BTreeMap::new(), || { id += 1; id }, 7);
        assert_eq!((g.len(), g.iter().map(|e| e.id).collect::<Vec<_>>()), (5, vec![101, 102, 103, 104, 105]));
        assert!(g.iter().all(|e| e.heroe == SIN_DUENO && e.cantidad == r.militar.tropas.tropa(e.tropa).unidades && e.moral_en(7) > 0));
        assert!(!incompleta(r, &g));
        g[1].cantidad = 3;
        g[2].cantidad = 0;
        g[2].experiencia = 4_000;
        assert!(incompleta(r, &g));
        assert_eq!(reposicion_en(r, 10), 10 + 360 * base::MS_POR_MINUTO);
        reponer(r, &mut g);
        assert!(!incompleta(r, &g) && g[2].experiencia == 4_000, "de nuevo completa, con su experiencia");
    }

    #[test]
    fn el_cupo_de_jugadores_y_los_escalones_elegibles() {
        let r = r();
        assert_eq!(Etapa::TODAS.map(|e| cupo_de_guarnicion(r, e)), [14, 45, 90]);
        assert_eq!(escalon_elegible(r, Tropa::MiliciaLanceros), Some(2));
        assert_eq!(escalon_elegible(r, Tropa::Arqueros), Some(3));
        assert_eq!(escalon_elegible(r, Tropa::Shardana), Some(4));
        assert_eq!((escalon_elegible(r, Tropa::Mineros), escalon_elegible(r, Tropa::Hequetai)), (None, None), "ni leva (1) ni élite (5)");
    }

    #[test]
    fn el_atacante_se_lleva_del_almacen_lo_que_le_cabe_en_los_carros() {
        use asentamiento::Recurso::{Madera, Oro, Trigo};
        let mut almacen = Almacen::nuevo(2_000_000);
        almacen.poner(Madera, 600_000);
        almacen.poner(Oro, 50_000);
        let mut carros = [CarroDeColumna { carga: Carga::new(), capacidad: 400_000 }, CarroDeColumna { carga: Carga::new(), capacidad: 100_000 }];
        let llevado = saquear(&mut almacen, &mut carros);
        assert_eq!(total(&llevado), 500_000, "caben 400 + 100");
        assert_eq!(almacen.cantidad(Madera) + almacen.cantidad(Oro), 650_000 - 500_000, "lo que no cupo se queda en la avanzada");
        assert!(saquear(&mut Almacen::nuevo(0), &mut carros).is_empty() && almacen.cantidad(Trigo) == 0);
        assert!(saquear(&mut almacen, &mut []).is_empty(), "sin carros no se lleva nada");
    }

    #[test]
    fn una_operativa_se_resuelve_como_un_asedio_con_su_guarnicion() {
        let r = r();
        let mut a = [escuadra(1, 1, Tropa::ArquerosCompuesto), escuadra(2, 1, Tropa::ArquerosCompuesto)];
        let mut g = [escuadra(3, 0, Tropa::MiliciaLanceros)];
        let res = resolver_ataque(r, Estado::Operativa, 0, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut g, heroes: &[] }, &mut Rng::nuevo(2));
        assert!(matches!(res, Asedio::Combate(_)) && res.conquistada());
        assert_eq!(tras_ataque(r, Etapa::Atalaya, Estado::Operativa, 0, res.conquistada()), Estado::Destruida);
    }

    #[test]
    fn una_avanzada_no_tiene_recintos_y_su_ataque_es_el_asedio_con_multiplicador_uno() {
        let r = r();
        let (a0, g0) = ([escuadra(1, 1, Tropa::ArquerosCompuesto), escuadra(2, 1, Tropa::ArquerosCompuesto)], [escuadra(3, 0, Tropa::MiliciaLanceros)]);
        let (mut a, mut g, mut rng) = (a0.clone(), g0.clone(), Rng::nuevo(2));
        let ataque = resolver_ataque(r, Estado::Operativa, 0, Bando { escuadras: &mut a, heroes: &[1] }, Bando { escuadras: &mut g, heroes: &[] }, &mut rng);
        let (mut a2, mut g2, mut rng2) = (a0, g0, Rng::nuevo(2));
        let asedio = resolver_asedio(r, false, Bando { escuadras: &mut a2, heroes: &[1] }, Bando { escuadras: &mut g2, heroes: &[] }, SIN_RECINTO, &mut rng2);
        assert_eq!((ataque, rng.estado()), (asedio, rng2.estado()), "mismas tiradas y mismo resultado que un asedio sin recinto");
    }
}
