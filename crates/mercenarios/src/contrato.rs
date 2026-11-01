//! La firma de un contrato (diseño §3 y §5): quién puede firmar qué trabajo sobre qué objetivo, la tarifa y la orden inicial.

use crate::datos::Datos;
use crate::tipos::{Ambito, Contrato, Objetivo, Orden, Puesto, Situacion, TipoOrden, Trabajo};
use base::{Instante, Mil, MS_POR_MINUTO, UNIDAD};
use std::cmp::Reverse;

/// Por qué una firma no vale (§3, §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorFirma {
    /// El objetivo no es de una clase que admita el trabajo o no vale para él.
    ObjetivoInvalido,
    /// El plazo no es uno de la tabla, o no es el que lleva ese trabajo.
    PlazoInvalido,
    /// Ninguno de los cargos de quien firma puede firmar ese trabajo sobre ese objetivo.
    SinAlcance,
}

/// ¿Vale `objetivo` para `trabajo` de la Facción `faccion`? La clase la dice la tabla de trabajos (§4); además, defender, escoltar, acompañar y el permanente piden algo de la propia
/// Facción, y atacar pide bandidos o algo de una Facción en guerra con ella (§3). `s` es lo que el núcleo averiguó del objetivo.
pub fn objetivo_valido(d: &Datos, trabajo: Trabajo, objetivo: &Objetivo, faccion: u32, s: &Situacion) -> Result<(), ErrorFirma> {
    let vale = d.trabajo(trabajo).objetivos.contains(&objetivo.clase())
        && match trabajo {
            Trabajo::Atacar => matches!(objetivo, Objetivo::Bandidos(_)) || s.en_guerra,
            _ => s.duena == Some(faccion),
        };
    vale.then_some(()).ok_or(ErrorFirma::ObjetivoInvalido)
}

/// El renombre de terminar sin despido un contrato de este `trabajo` y este `plazo` (§8.2), si el plazo es el que le toca: el temporal lleva uno de la tabla y el permanente
/// ninguno (§5), que no gana renombre por plazo.
pub fn renombre_del_plazo(d: &Datos, trabajo: Trabajo, plazo: Option<u8>) -> Result<u32, ErrorFirma> {
    match (trabajo, plazo) {
        (Trabajo::Permanente, None) => Ok(0),
        (Trabajo::Permanente, Some(_)) | (_, None) => Err(ErrorFirma::PlazoInvalido),
        (_, Some(dias)) => d.plazo(dias).map(|p| p.renombre).ok_or(ErrorFirma::PlazoInvalido),
    }
}

/// El puesto de más rango de `puestos` que puede firmar este trabajo sobre este objetivo (§3); con el mismo rango, el primero de la lista. `s` es lo que el núcleo averiguó del
/// objetivo y `firmante`, quien firma.
pub fn puesto_para_firmar(trabajo: Trabajo, objetivo: &Objetivo, s: &Situacion, puestos: &[Puesto], firmante: u64) -> Result<Puesto, ErrorFirma> {
    puestos.iter().copied().filter(|&p| puede_firmar(p, trabajo, objetivo, s, firmante)).min_by_key(|p| Reverse(p.rango())).ok_or(ErrorFirma::SinAlcance)
}

/// La tabla de §3. «Su plaza» es aquella de la que cuelga el objetivo (`Situacion::plaza`: la plaza misma, la madre de un pueblo o una avanzada, el origen de una caravana) y «su
/// zona», aquella en la que están los bandidos. El General de enemigos en guerra es cualquier objetivo de una Facción en guerra con la suya, esté donde esté (decisión 8 del plan).
fn puede_firmar(puesto: Puesto, trabajo: Trabajo, objetivo: &Objetivo, s: &Situacion, firmante: u64) -> bool {
    let su_plaza = |plaza: u64| s.plaza == Some(plaza);
    let bandidos_de_su_zona = |plaza: u64| matches!(objetivo, Objetivo::Bandidos(_)) && s.en_zona_de.contains(&plaza);
    match (puesto, trabajo) {
        (Puesto::Rey | Puesto::Embajador, _) => true,
        (Puesto::Gobernador(p), Trabajo::Defender | Trabajo::Escoltar) | (Puesto::General(p), Trabajo::Defender) | (Puesto::MaestroCaravanas(p), Trabajo::Escoltar) => su_plaza(p),
        (Puesto::Gobernador(p), Trabajo::Atacar) => bandidos_de_su_zona(p),
        (Puesto::General(p), Trabajo::Atacar) => bandidos_de_su_zona(p) || s.en_guerra,
        // Acompañar: el General, solo a lo que encabeza él (decisión 8 del plan: su convocatoria es la que convocó él).
        (Puesto::General(_), Trabajo::Acompanar) => s.cabeza == Some(firmante),
        // El resto de la tabla de §3 es «no».
        _ => false,
    }
}

/// De quién es el contrato que firma ese puesto (§5): de la Facción si lo firma el Rey o el Embajador; de la plaza, si lo firma un cargo local.
pub fn ambito_de(puesto: Puesto) -> Ambito {
    match puesto {
        Puesto::Rey | Puesto::Embajador => Ambito::Faccion,
        Puesto::General(p) | Puesto::Gobernador(p) | Puesto::MaestroCaravanas(p) => Ambito::Plaza(p),
    }
}

/// Lo que cuesta un contrato (duda 1 del plan, §5): ⌈(oroPorNivelHeroe × nivel + valor_tropa) × pct del plazo (o `permanentePct`) × tarifaPct del rango⌉, en oro entero.
/// `plazo` son los días de un temporal, o `None` en el permanente; `valor_tropa` es lo que costaría hoy reclutar la tropa del mercenario, en milésimas de oro; `rango`, el de su
/// ficha (1 a 5, §8.3).
pub fn tarifa(d: &Datos, plazo: Option<u8>, nivel: u8, valor_tropa: Mil, rango: u8) -> Result<Mil, ErrorFirma> {
    let pct_del_plazo = match plazo {
        Some(dias) => d.plazo(dias).ok_or(ErrorFirma::PlazoInvalido)?.tarifa_pct,
        None => d.permanente_pct,
    };
    let valor = d.oro_por_nivel * Mil::from(nivel) + valor_tropa;
    let pct_del_rango = d.rangos[usize::from(rango) - 1].tarifa_pct;
    // Los dos porcentajes son centésimas, y el oro se cobra en unidades enteras, hacia arriba.
    let divisor = 100 * 100 * UNIDAD;
    Ok((valor * Mil::from(pct_del_plazo) * Mil::from(pct_del_rango) + divisor - 1) / divisor * UNIDAD)
}

/// La duración de un plazo de `dias` días de mundo, en milisegundos.
pub fn duracion(dias: u8) -> Instante {
    Instante::from(dias) * 24 * 60 * MS_POR_MINUTO
}

/// La orden inicial del contrato `c` (§6.3, §6.4): la que dice su fila de `TRABAJOS_MERCENARIOS` sobre su objetivo, dada por quien lo firmó en `t`. La del permanente es `guardar`
/// de la plaza donde reside (`residencia`), no de su objetivo, y sin plaza no tiene ninguna. `contra` es la Facción dueña del objetivo en `t`, que solo guarda un `atacar`;
/// `ir_a` y `atacar` caducan a las `plazoOrdenHoras` (decisión 6 del plan).
pub fn orden_inicial(d: &Datos, c: &Contrato, residencia: Option<u64>, contra: Option<u32>, t: Instante) -> Option<Orden> {
    let tipo = d.trabajo(c.trabajo).inicial;
    let objetivo = if c.trabajo == Trabajo::Permanente { Objetivo::Plaza(residencia?) } else { c.objetivo };
    Some(Orden {
        tipo,
        objetivo,
        dada_por: c.firmado_por,
        puesto: c.puesto_firma,
        dada_en: t,
        vence: matches!(tipo, TipoOrden::IrA | TipoOrden::Atacar).then_some(t + d.plazo_orden),
        contra: contra.filter(|_| tipo == TipoOrden::Atacar),
        unido: false,
    })
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use base::Punto;

    fn datos() -> Datos {
        Datos::cargar()
    }

    /// Lo que cuelga de la plaza `plaza` de la Facción 1.
    fn de_la_plaza(plaza: u64) -> Situacion {
        Situacion { duena: Some(1), plaza: Some(plaza), ..Situacion::default() }
    }

    /// Unos bandidos en la zona de las plazas dadas.
    fn bandidos_en(zonas: &[u64]) -> Situacion {
        Situacion { en_zona_de: zonas.to_vec(), ..Situacion::default() }
    }

    /// Lo que firma `puesto` solo, sin más cargos, siendo el héroe 7 quien firma.
    fn firma(puesto: Puesto, trabajo: Trabajo, objetivo: Objetivo, s: &Situacion) -> Result<Puesto, ErrorFirma> {
        puesto_para_firmar(trabajo, &objetivo, s, &[puesto], 7)
    }

    #[test]
    fn el_rey_y_el_embajador_firman_todos_los_trabajos() {
        for puesto in [Puesto::Rey, Puesto::Embajador] {
            let casos = [
                (Trabajo::Defender, Objetivo::Plaza(4), de_la_plaza(4)),
                (Trabajo::Escoltar, Objetivo::Caravana(8), de_la_plaza(4)),
                (Trabajo::Atacar, Objetivo::Plaza(9), Situacion { duena: Some(2), plaza: Some(9), en_guerra: true, ..Situacion::default() }),
                (Trabajo::Acompanar, Objetivo::Ejercito(3), Situacion { duena: Some(1), cabeza: Some(99), ..Situacion::default() }),
                (Trabajo::Permanente, Objetivo::Faccion(1), Situacion { duena: Some(1), ..Situacion::default() }),
            ];
            for (trabajo, objetivo, s) in casos {
                assert_eq!(firma(puesto, trabajo, objetivo, &s), Ok(puesto), "{puesto:?} firma {trabajo:?}");
            }
        }
    }

    #[test]
    fn el_gobernador_y_el_general_defienden_su_plaza_con_sus_pueblos_y_avanzadas_y_no_la_de_otra() {
        let en_la_4 = de_la_plaza(4);
        for puesto in [Puesto::Gobernador(4), Puesto::General(4)] {
            assert_eq!(firma(puesto, Trabajo::Defender, Objetivo::Plaza(4), &en_la_4), Ok(puesto));
            assert_eq!(firma(puesto, Trabajo::Defender, Objetivo::Pueblo(30), &en_la_4), Ok(puesto), "un pueblo de su plaza");
            assert_eq!(firma(puesto, Trabajo::Defender, Objetivo::Avanzada(31), &en_la_4), Ok(puesto), "una avanzada de su plaza");
            assert_eq!(firma(puesto, Trabajo::Defender, Objetivo::Plaza(5), &de_la_plaza(5)), Err(ErrorFirma::SinAlcance), "la plaza de otro");
            assert_eq!(firma(puesto, Trabajo::Defender, Objetivo::Pueblo(32), &de_la_plaza(5)), Err(ErrorFirma::SinAlcance), "un pueblo de otra plaza");
        }
        assert_eq!(firma(Puesto::MaestroCaravanas(4), Trabajo::Defender, Objetivo::Plaza(4), &en_la_4), Err(ErrorFirma::SinAlcance), "el Maestro de Caravanas no contrata defensa");
    }

    #[test]
    fn escoltan_el_gobernador_y_el_maestro_de_su_plaza_pero_no_el_general() {
        let caravana = Objetivo::Caravana(8);
        assert_eq!(firma(Puesto::Gobernador(4), Trabajo::Escoltar, caravana, &de_la_plaza(4)), Ok(Puesto::Gobernador(4)));
        assert_eq!(firma(Puesto::MaestroCaravanas(4), Trabajo::Escoltar, caravana, &de_la_plaza(4)), Ok(Puesto::MaestroCaravanas(4)));
        assert_eq!(firma(Puesto::MaestroCaravanas(4), Trabajo::Escoltar, caravana, &de_la_plaza(5)), Err(ErrorFirma::SinAlcance), "la caravana de otra plaza");
        assert_eq!(firma(Puesto::Gobernador(4), Trabajo::Escoltar, caravana, &de_la_plaza(5)), Err(ErrorFirma::SinAlcance));
        assert_eq!(firma(Puesto::General(4), Trabajo::Escoltar, caravana, &de_la_plaza(4)), Err(ErrorFirma::SinAlcance), "el General no escolta");
    }

    #[test]
    fn el_gobernador_ataca_solo_bandidos_de_su_zona() {
        let g = Puesto::Gobernador(4);
        assert_eq!(firma(g, Trabajo::Atacar, Objetivo::Bandidos(50), &bandidos_en(&[3, 4])), Ok(g));
        assert_eq!(firma(g, Trabajo::Atacar, Objetivo::Bandidos(50), &bandidos_en(&[3])), Err(ErrorFirma::SinAlcance), "bandidos fuera de su zona");
        let enemigo = Situacion { duena: Some(2), plaza: Some(9), en_guerra: true, ..Situacion::default() };
        assert_eq!(firma(g, Trabajo::Atacar, Objetivo::Plaza(9), &enemigo), Err(ErrorFirma::SinAlcance), "un enemigo en guerra no es cosa del Gobernador");
    }

    #[test]
    fn el_general_ataca_bandidos_de_su_zona_y_cualquier_enemigo_en_guerra() {
        let g = Puesto::General(4);
        assert_eq!(firma(g, Trabajo::Atacar, Objetivo::Bandidos(50), &bandidos_en(&[4])), Ok(g));
        assert_eq!(firma(g, Trabajo::Atacar, Objetivo::Bandidos(50), &bandidos_en(&[3])), Err(ErrorFirma::SinAlcance), "bandidos de otra zona");
        let lejos = Situacion { duena: Some(2), plaza: Some(9), en_guerra: true, ..Situacion::default() };
        assert_eq!(firma(g, Trabajo::Atacar, Objetivo::Plaza(9), &lejos), Ok(g), "un enemigo en guerra, esté donde esté");
        let en_paz = Situacion { en_guerra: false, ..lejos };
        assert_eq!(firma(g, Trabajo::Atacar, Objetivo::Plaza(9), &en_paz), Err(ErrorFirma::SinAlcance));
        assert_eq!(firma(Puesto::MaestroCaravanas(4), Trabajo::Atacar, Objetivo::Bandidos(50), &bandidos_en(&[4])), Err(ErrorFirma::SinAlcance), "el Maestro de Caravanas no ataca");
    }

    #[test]
    fn el_general_acompana_solo_lo_que_encabeza_y_el_gobernador_nunca() {
        let suya = Situacion { duena: Some(1), cabeza: Some(7), ..Situacion::default() };
        let ajena = Situacion { duena: Some(1), cabeza: Some(8), ..Situacion::default() };
        assert_eq!(firma(Puesto::General(4), Trabajo::Acompanar, Objetivo::Convocatoria(2), &suya), Ok(Puesto::General(4)));
        assert_eq!(firma(Puesto::General(4), Trabajo::Acompanar, Objetivo::Convocatoria(2), &ajena), Err(ErrorFirma::SinAlcance));
        assert_eq!(firma(Puesto::Gobernador(4), Trabajo::Acompanar, Objetivo::Convocatoria(2), &suya), Err(ErrorFirma::SinAlcance));
        assert_eq!(firma(Puesto::MaestroCaravanas(4), Trabajo::Acompanar, Objetivo::Convocatoria(2), &suya), Err(ErrorFirma::SinAlcance));
    }

    #[test]
    fn el_permanente_solo_lo_firman_el_rey_y_el_embajador() {
        let s = Situacion { duena: Some(1), ..Situacion::default() };
        for puesto in [Puesto::Gobernador(4), Puesto::General(4), Puesto::MaestroCaravanas(4)] {
            assert_eq!(firma(puesto, Trabajo::Permanente, Objetivo::Faccion(1), &s), Err(ErrorFirma::SinAlcance), "{puesto:?}");
        }
    }

    #[test]
    fn firma_el_puesto_de_mas_rango_que_puede_y_con_igual_rango_el_primero() {
        let s = de_la_plaza(4);
        let defender = |puestos: &[Puesto]| puesto_para_firmar(Trabajo::Defender, &Objetivo::Plaza(4), &s, puestos, 7);
        assert_eq!(defender(&[Puesto::Gobernador(4), Puesto::Rey]), Ok(Puesto::Rey));
        assert_eq!(defender(&[Puesto::General(4), Puesto::Gobernador(4)]), Ok(Puesto::General(4)));
        assert_eq!(defender(&[Puesto::Gobernador(4), Puesto::General(4)]), Ok(Puesto::Gobernador(4)));
        // El más alto que no puede no cuenta: el General no escolta, el Maestro sí.
        let escolta = puesto_para_firmar(Trabajo::Escoltar, &Objetivo::Caravana(8), &s, &[Puesto::General(4), Puesto::MaestroCaravanas(4)], 7);
        assert_eq!(escolta, Ok(Puesto::MaestroCaravanas(4)));
        assert_eq!(defender(&[]), Err(ErrorFirma::SinAlcance), "sin cargos no se firma");
    }

    #[test]
    fn el_objetivo_ha_de_ser_de_una_clase_que_admita_el_trabajo() {
        let d = datos();
        let propio = Situacion { duena: Some(1), plaza: Some(4), ..Situacion::default() };
        let vale = |trabajo, objetivo: Objetivo| objetivo_valido(&d, trabajo, &objetivo, 1, &propio);
        assert_eq!(vale(Trabajo::Defender, Objetivo::Plaza(4)), Ok(()));
        assert_eq!(vale(Trabajo::Defender, Objetivo::Caravana(8)), Err(ErrorFirma::ObjetivoInvalido), "no se defiende una caravana");
        assert_eq!(vale(Trabajo::Defender, Objetivo::Punto(Punto::new(0, 0))), Err(ErrorFirma::ObjetivoInvalido), "un punto es solo de `ir_a`");
        assert_eq!(vale(Trabajo::Escoltar, Objetivo::Caravana(8)), Ok(()));
        assert_eq!(vale(Trabajo::Escoltar, Objetivo::Plaza(4)), Err(ErrorFirma::ObjetivoInvalido), "no se escolta una plaza");
        assert_eq!(vale(Trabajo::Permanente, Objetivo::Plaza(4)), Err(ErrorFirma::ObjetivoInvalido), "el permanente es de la Facción");
        assert_eq!(vale(Trabajo::Acompanar, Objetivo::Avanzada(31)), Err(ErrorFirma::ObjetivoInvalido), "se acompaña a una convocatoria o a un ejército");
    }

    #[test]
    fn defender_escoltar_acompanar_y_el_permanente_piden_algo_de_la_propia_faccion() {
        let d = datos();
        let ajeno = Situacion { duena: Some(2), plaza: Some(9), ..Situacion::default() };
        assert_eq!(objetivo_valido(&d, Trabajo::Defender, &Objetivo::Plaza(9), 1, &ajeno), Err(ErrorFirma::ObjetivoInvalido));
        assert_eq!(objetivo_valido(&d, Trabajo::Escoltar, &Objetivo::Caravana(8), 1, &ajeno), Err(ErrorFirma::ObjetivoInvalido));
        assert_eq!(objetivo_valido(&d, Trabajo::Acompanar, &Objetivo::Ejercito(3), 1, &ajeno), Err(ErrorFirma::ObjetivoInvalido));
        assert_eq!(objetivo_valido(&d, Trabajo::Permanente, &Objetivo::Faccion(2), 1, &Situacion { duena: Some(2), ..Situacion::default() }), Err(ErrorFirma::ObjetivoInvalido));
        assert_eq!(objetivo_valido(&d, Trabajo::Permanente, &Objetivo::Faccion(1), 1, &Situacion { duena: Some(1), ..Situacion::default() }), Ok(()));
        assert_eq!(objetivo_valido(&d, Trabajo::Acompanar, &Objetivo::Ejercito(3), 2, &ajeno), Ok(()), "cada una acompaña a lo suyo");
    }

    #[test]
    fn atacar_pide_bandidos_o_una_faccion_en_guerra_pero_nunca_la_propia_ni_una_neutral() {
        let d = datos();
        let plaza = |duena: u32, en_guerra: bool| Situacion { duena: Some(duena), plaza: Some(9), en_guerra, ..Situacion::default() };
        assert_eq!(objetivo_valido(&d, Trabajo::Atacar, &Objetivo::Bandidos(50), 1, &bandidos_en(&[4])), Ok(()));
        assert_eq!(objetivo_valido(&d, Trabajo::Atacar, &Objetivo::Plaza(9), 1, &plaza(2, true)), Ok(()), "una Facción en guerra");
        assert_eq!(objetivo_valido(&d, Trabajo::Atacar, &Objetivo::Plaza(9), 1, &plaza(2, false)), Err(ErrorFirma::ObjetivoInvalido), "sin guerra");
        assert_eq!(objetivo_valido(&d, Trabajo::Atacar, &Objetivo::Plaza(4), 1, &plaza(1, false)), Err(ErrorFirma::ObjetivoInvalido), "la propia");
        assert_eq!(objetivo_valido(&d, Trabajo::Atacar, &Objetivo::Faccion(2), 1, &plaza(2, true)), Err(ErrorFirma::ObjetivoInvalido), "una Facción entera no es un objetivo de ataque");
    }

    #[test]
    fn el_temporal_lleva_uno_de_los_plazos_de_la_tabla_y_el_permanente_ninguno() {
        let d = datos();
        assert_eq!(renombre_del_plazo(&d, Trabajo::Defender, Some(1)), Ok(10));
        assert_eq!(renombre_del_plazo(&d, Trabajo::Escoltar, Some(3)), Ok(25));
        assert_eq!(renombre_del_plazo(&d, Trabajo::Atacar, Some(7)), Ok(50));
        assert_eq!(renombre_del_plazo(&d, Trabajo::Acompanar, Some(2)), Err(ErrorFirma::PlazoInvalido), "dos días no es un plazo");
        assert_eq!(renombre_del_plazo(&d, Trabajo::Defender, None), Err(ErrorFirma::PlazoInvalido), "un temporal sin plazo");
        assert_eq!(renombre_del_plazo(&d, Trabajo::Permanente, Some(7)), Err(ErrorFirma::PlazoInvalido), "un permanente con plazo");
        assert_eq!(renombre_del_plazo(&d, Trabajo::Permanente, None), Ok(0));
    }

    #[test]
    fn la_tarifa_es_valor_por_plazo_por_rango_redondeada_hacia_arriba_a_oro_entero() {
        let d = datos();
        // Nivel 1 sin tropa, 3 días, rango 1: 20 × 35 % = 7 de oro exactos (no se redondea de más).
        assert_eq!(tarifa(&d, Some(3), 1, 0, 1), Ok(7_000));
        // Nivel 3 con 14,5 de oro de tropa: 74,5 × 15 % × 120 % = 13,41 → 14.
        assert_eq!(tarifa(&d, Some(1), 3, 14_500, 2), Ok(14_000));
        // El permanente, ×5: 74,5 × 500 % = 372,5 → 373.
        assert_eq!(tarifa(&d, None, 3, 14_500, 1), Ok(373_000));
        // Un nivel 10 señor de la guerra (×2,2) a 7 días: 200 × 70 % × 220 % = 308.
        assert_eq!(tarifa(&d, Some(7), 10, 0, 5), Ok(308_000));
        // Una fracción de milésima de oro de más sube una unidad entera.
        assert_eq!(tarifa(&d, Some(3), 1, 1, 1), Ok(8_000));
        assert_eq!(tarifa(&d, Some(2), 1, 0, 1), Err(ErrorFirma::PlazoInvalido));
    }

    #[test]
    fn el_contrato_es_de_la_faccion_si_lo_firma_el_rey_o_el_embajador_y_de_la_plaza_si_lo_firma_un_cargo_local() {
        assert_eq!(ambito_de(Puesto::Rey), Ambito::Faccion);
        assert_eq!(ambito_de(Puesto::Embajador), Ambito::Faccion);
        assert_eq!(ambito_de(Puesto::General(4)), Ambito::Plaza(4));
        assert_eq!(ambito_de(Puesto::Gobernador(5)), Ambito::Plaza(5));
        assert_eq!(ambito_de(Puesto::MaestroCaravanas(6)), Ambito::Plaza(6));
    }

    #[test]
    fn un_plazo_de_dias_dura_dias_de_mundo() {
        assert_eq!(duracion(1), 86_400_000);
        assert_eq!(duracion(7), 7 * 86_400_000);
    }

    fn contrato(trabajo: Trabajo, objetivo: Objetivo) -> Contrato {
        Contrato {
            mercenario: 9,
            faccion: 1,
            ambito: Ambito::Plaza(4),
            trabajo,
            objetivo,
            plazo: Some(3),
            firmado_por: 2,
            puesto_firma: Puesto::Gobernador(4),
            firmado_en: 100,
            vence: None,
            campamento_base: 0,
            orden: None,
            fin_aplazado: None,
            renombre: 0,
        }
    }

    #[test]
    fn la_orden_inicial_de_cada_trabajo_sale_de_la_tabla_y_la_da_quien_firmo() {
        let d = datos();
        let t = 5_000;
        let dada = |tipo, objetivo| Orden { tipo, objetivo, dada_por: 2, puesto: Puesto::Gobernador(4), dada_en: t, vence: None, contra: None, unido: false };
        let inicial = |trabajo, objetivo| orden_inicial(&d, &contrato(trabajo, objetivo), None, Some(2), t);
        assert_eq!(inicial(Trabajo::Defender, Objetivo::Pueblo(30)), Some(dada(TipoOrden::Guardar, Objetivo::Pueblo(30))), "guardar el objetivo; la dueña no se guarda");
        assert_eq!(inicial(Trabajo::Escoltar, Objetivo::Caravana(8)), Some(dada(TipoOrden::Guardar, Objetivo::Caravana(8))));
        assert_eq!(inicial(Trabajo::Acompanar, Objetivo::Convocatoria(2)), Some(dada(TipoOrden::Unirse, Objetivo::Convocatoria(2))), "sin haber estado dentro todavía");
        let ataque = Orden { vence: Some(t + d.plazo_orden), contra: Some(2), ..dada(TipoOrden::Atacar, Objetivo::Plaza(9)) };
        assert_eq!(inicial(Trabajo::Atacar, Objetivo::Plaza(9)), Some(ataque), "atacar caduca a las 24 h y recuerda de quién era el objetivo");
    }

    #[test]
    fn el_permanente_guarda_la_plaza_donde_reside_y_sin_plaza_no_tiene_orden() {
        let d = datos();
        let c = contrato(Trabajo::Permanente, Objetivo::Faccion(1));
        let orden = orden_inicial(&d, &c, Some(6), None, 5_000).expect("reside en una plaza");
        assert_eq!((orden.tipo, orden.objetivo, orden.vence), (TipoOrden::Guardar, Objetivo::Plaza(6), None));
        assert_eq!(orden_inicial(&d, &c, None, None, 5_000), None, "residiendo en un campamento no guarda nada");
    }
}
