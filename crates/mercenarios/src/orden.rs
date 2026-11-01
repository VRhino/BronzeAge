//! Las órdenes de mando sin mirar el mundo (diseño §6.1, §6.4 y §6.5): alcance de cada cargo, qué admite cada trabajo y qué orden sustituye a cuál.

use crate::datos::{Admite, Datos};
use crate::tipos::{Ambito, Contrato, Objetivo, Orden, Puesto, Situacion, TipoOrden, Trabajo};
use base::Mil;

/// Por qué no vale una orden. Cada variante tiene su rechazo `mercenario.*` en el núcleo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorOrden {
    /// El objetivo no es de la clase que pide la orden o no es de quien debe ser (§6.1): `mercenario.objetivo_invalido`.
    ObjetivoInvalido,
    /// El trabajo del contrato no admite esa orden (§6.4): `mercenario.orden_fuera_de_contrato`.
    FueraDeContrato,
    /// Ningún cargo de quien ordena alcanza a ese mercenario con esa orden (§6.5): `mercenario.sin_alcance`.
    SinAlcance,
}

/// ¿Vale el objetivo para esta orden? (§6.1) `faccion` es la del contrato, que es también la de quien ordena.
fn objetivo_valido(faccion: u32, tipo: TipoOrden, objetivo: &Objetivo, s: &Situacion) -> bool {
    let propio = s.duena == Some(faccion);
    match (tipo, objetivo) {
        (TipoOrden::Guardar, Objetivo::Plaza(_) | Objetivo::Pueblo(_) | Objetivo::Avanzada(_) | Objetivo::Caravana(_)) => propio,
        (TipoOrden::IrA, Objetivo::Punto(_)) => true,
        (TipoOrden::Atacar, Objetivo::Bandidos(_)) => true,
        (TipoOrden::Atacar, Objetivo::Ejercito(_) | Objetivo::Caravana(_) | Objetivo::Plaza(_) | Objetivo::Pueblo(_) | Objetivo::Avanzada(_)) => s.en_guerra && !propio,
        (TipoOrden::Unirse, Objetivo::Convocatoria(_) | Objetivo::Ejercito(_)) => propio,
        _ => false,
    }
}

/// ¿Alcanza este puesto a este mercenario? (§6.5, «A quién manda»)
fn alcanza(puesto: Puesto, c: &Contrato, residencia: Option<u64>) -> bool {
    let de_la_plaza = |p: u64| c.ambito == Ambito::Plaza(p);
    match puesto {
        Puesto::Rey | Puesto::Embajador => true,
        Puesto::General(p) | Puesto::Gobernador(p) => de_la_plaza(p) || (c.trabajo == Trabajo::Permanente && residencia == Some(p)),
        Puesto::MaestroCaravanas(p) => de_la_plaza(p) && c.trabajo == Trabajo::Escoltar,
    }
}

/// ¿Puede este puesto dar esta orden sobre este objetivo? (§6.5, «Qué órdenes»). `ordenante` es quien la da, para la convocatoria o el ejército que encabeza.
fn puede_dar(puesto: Puesto, tipo: TipoOrden, objetivo: &Objetivo, s: &Situacion, ordenante: u64) -> bool {
    let de_la_plaza = |p: u64| s.plaza == Some(p);
    match (puesto, tipo) {
        (Puesto::Rey | Puesto::Embajador, _) | (_, TipoOrden::IrA) => true,
        (Puesto::General(p) | Puesto::Gobernador(p), TipoOrden::Guardar) => de_la_plaza(p),
        (Puesto::MaestroCaravanas(p), TipoOrden::Guardar) => matches!(objetivo, Objetivo::Caravana(_)) && de_la_plaza(p),
        (Puesto::General(_), TipoOrden::Atacar) => true,
        (Puesto::Gobernador(_), TipoOrden::Atacar) => matches!(objetivo, Objetivo::Bandidos(_)),
        (Puesto::General(_), TipoOrden::Unirse) => s.cabeza == Some(ordenante),
        (Puesto::Gobernador(_), TipoOrden::Unirse) | (Puesto::MaestroCaravanas(_), TipoOrden::Atacar | TipoOrden::Unirse) => false,
    }
}

/// El puesto de más rango de `puestos` que alcanza a este mercenario y puede dar esta orden (§6.5). Antes mira que el objetivo valga para la orden (§6.1).
/// `residencia`: la plaza donde reside el mercenario, si reside en una. `ordenante`: quien da la orden y tiene esos `puestos`.
pub fn puesto_para_ordenar(c: &Contrato, residencia: Option<u64>, puestos: &[Puesto], tipo: TipoOrden, objetivo: &Objetivo, s: &Situacion, ordenante: u64) -> Result<Puesto, ErrorOrden> {
    if !objetivo_valido(c.faccion, tipo, objetivo, s) {
        return Err(ErrorOrden::ObjetivoInvalido);
    }
    puestos.iter().copied().filter(|&p| alcanza(p, c, residencia) && puede_dar(p, tipo, objetivo, s, ordenante)).max_by_key(|p| p.rango()).ok_or(ErrorOrden::SinAlcance)
}

/// ¿Admite el trabajo del contrato esta orden? (§6.4, tabla TRABAJOS_MERCENARIOS).
/// `distancia`: del objetivo de la orden al del contrato, para `atacar_en_radio`; sin ella, esa orden no se admite.
pub fn admite(d: &Datos, c: &Contrato, tipo: TipoOrden, objetivo: &Objetivo, distancia: Option<Mil>) -> Result<(), ErrorOrden> {
    let vale = |a: &Admite| match a {
        Admite::Guardar => tipo == TipoOrden::Guardar,
        Admite::GuardarObjetivo => tipo == TipoOrden::Guardar && *objetivo == c.objetivo,
        Admite::IrA => tipo == TipoOrden::IrA,
        Admite::Atacar => tipo == TipoOrden::Atacar,
        Admite::AtacarEnRadio => tipo == TipoOrden::Atacar && distancia.is_some_and(|x| x <= d.radio_defensa),
        Admite::Unirse => tipo == TipoOrden::Unirse,
    };
    if d.trabajo(c.trabajo).admite.iter().any(vale) {
        Ok(())
    } else {
        Err(ErrorOrden::FueraDeContrato)
    }
}

/// Una orden nueva sustituye a la vigente si su puesto tiene igual o más rango que el de quien dio la vigente (§6.5). Sin orden vigente, siempre.
/// General y Gobernador tienen el mismo rango: manda la última. Si no la sustituye, el núcleo rechaza con `mercenario.orden_de_mas_rango`.
pub fn sustituye(vigente: Option<&Orden>, puesto: Puesto) -> bool {
    vigente.is_none_or(|o| puesto.rango() >= o.puesto.rango())
}

#[cfg(test)]
mod pruebas {
    //! Una prueba por fila de las tablas de §6.4 y §6.5, en positivo y en negativo, y las clases de objetivo de §6.1.
    use super::*;
    use base::Punto;

    /// La Facción de los contratos y de quien ordena.
    const NUESTRA: u32 = 1;
    const AJENA: u32 = 2;
    /// Dos plazas de la Facción.
    const P: u64 = 3;
    const Q: u64 = 4;
    /// Quien ordena.
    const YO: u64 = 10;
    const OTRO: u64 = 11;

    fn contrato(trabajo: Trabajo, objetivo: Objetivo, ambito: Ambito) -> Contrato {
        Contrato {
            mercenario: 50,
            faccion: NUESTRA,
            ambito,
            trabajo,
            objetivo,
            plazo: Some(3),
            firmado_por: YO,
            puesto_firma: Puesto::Rey,
            firmado_en: 0,
            vence: Some(1_000),
            campamento_base: 7,
            orden: None,
            fin_aplazado: None,
            renombre: 0,
        }
    }

    /// Un escolta de una caravana, contratado por la plaza `p`.
    fn escolta(p: u64) -> Contrato {
        contrato(Trabajo::Escoltar, Objetivo::Caravana(90), Ambito::Plaza(p))
    }

    /// Un defensor de la plaza `p`, contratado por esa plaza.
    fn defensor(p: u64) -> Contrato {
        contrato(Trabajo::Defender, Objetivo::Plaza(p), Ambito::Plaza(p))
    }

    /// Un permanente de la Facción.
    fn permanente() -> Contrato {
        contrato(Trabajo::Permanente, Objetivo::Faccion(NUESTRA), Ambito::Faccion)
    }

    /// Un objetivo de la propia Facción que cuelga de la plaza `plaza`.
    fn propio(plaza: u64) -> Situacion {
        Situacion { duena: Some(NUESTRA), plaza: Some(plaza), ..Situacion::default() }
    }

    /// Un objetivo de otra Facción, en guerra con la nuestra.
    fn enemigo() -> Situacion {
        Situacion { duena: Some(AJENA), plaza: Some(80), en_guerra: true, ..Situacion::default() }
    }

    /// Una convocatoria o un ejército de la propia Facción, encabezado por `cabeza`.
    fn encabezado_por(cabeza: u64) -> Situacion {
        Situacion { duena: Some(NUESTRA), cabeza: Some(cabeza), ..Situacion::default() }
    }

    fn punto() -> Objetivo {
        Objetivo::Punto(Punto::new(1_000, 2_000))
    }

    /// Ordena `YO` con estos `puestos` a un mercenario que no reside en una plaza.
    fn ordenar(c: &Contrato, puestos: &[Puesto], tipo: TipoOrden, objetivo: Objetivo, s: &Situacion) -> Result<Puesto, ErrorOrden> {
        puesto_para_ordenar(c, None, puestos, tipo, &objetivo, s, YO)
    }

    fn orden_de(puesto: Puesto) -> Orden {
        Orden { tipo: TipoOrden::IrA, objetivo: punto(), dada_por: YO, puesto, dada_en: 0, vence: None, contra: None, unido: false }
    }

    #[test]
    fn guardar_pide_plaza_pueblo_avanzada_o_caravana_de_la_propia_faccion() {
        let c = escolta(P);
        for objetivo in [Objetivo::Plaza(1), Objetivo::Pueblo(2), Objetivo::Avanzada(3), Objetivo::Caravana(4)] {
            assert_eq!(ordenar(&c, &[Puesto::Rey], TipoOrden::Guardar, objetivo, &propio(P)), Ok(Puesto::Rey), "{objetivo:?} propio");
            assert_eq!(ordenar(&c, &[Puesto::Rey], TipoOrden::Guardar, objetivo, &enemigo()), Err(ErrorOrden::ObjetivoInvalido), "{objetivo:?} ajeno");
        }
        for objetivo in [Objetivo::Bandidos(1), Objetivo::Ejercito(2), Objetivo::Convocatoria(3), Objetivo::Faccion(NUESTRA), punto()] {
            assert_eq!(ordenar(&c, &[Puesto::Rey], TipoOrden::Guardar, objetivo, &propio(P)), Err(ErrorOrden::ObjetivoInvalido), "{objetivo:?}");
        }
    }

    #[test]
    fn ir_a_pide_un_punto_y_nada_mas() {
        let c = escolta(P);
        assert_eq!(ordenar(&c, &[Puesto::Rey], TipoOrden::IrA, punto(), &Situacion::default()), Ok(Puesto::Rey));
        for objetivo in [Objetivo::Plaza(1), Objetivo::Pueblo(2), Objetivo::Avanzada(3), Objetivo::Caravana(4), Objetivo::Bandidos(5), Objetivo::Ejercito(6), Objetivo::Convocatoria(7)] {
            assert_eq!(ordenar(&c, &[Puesto::Rey], TipoOrden::IrA, objetivo, &propio(P)), Err(ErrorOrden::ObjetivoInvalido), "{objetivo:?}");
        }
    }

    #[test]
    fn atacar_pide_bandidos_o_algo_de_una_faccion_en_guerra_nunca_de_la_propia() {
        let c = defensor(P);
        let rey = |objetivo: Objetivo, s: &Situacion| ordenar(&c, &[Puesto::Rey], TipoOrden::Atacar, objetivo, s);
        assert_eq!(rey(Objetivo::Bandidos(1), &Situacion::default()), Ok(Puesto::Rey));
        for objetivo in [Objetivo::Ejercito(2), Objetivo::Caravana(3), Objetivo::Plaza(4), Objetivo::Pueblo(5), Objetivo::Avanzada(6)] {
            assert_eq!(rey(objetivo, &enemigo()), Ok(Puesto::Rey), "{objetivo:?} en guerra");
            let sin_guerra = Situacion { en_guerra: false, ..enemigo() };
            assert_eq!(rey(objetivo, &sin_guerra), Err(ErrorOrden::ObjetivoInvalido), "{objetivo:?} sin guerra");
            let propio_en_guerra = Situacion { en_guerra: true, ..propio(P) };
            assert_eq!(rey(objetivo, &propio_en_guerra), Err(ErrorOrden::ObjetivoInvalido), "{objetivo:?} propio");
        }
        for objetivo in [Objetivo::Convocatoria(7), Objetivo::Faccion(AJENA), punto()] {
            assert_eq!(rey(objetivo, &enemigo()), Err(ErrorOrden::ObjetivoInvalido), "{objetivo:?}");
        }
    }

    #[test]
    fn unirse_pide_convocatoria_o_ejercito_de_la_propia_faccion() {
        let c = contrato(Trabajo::Acompanar, Objetivo::Convocatoria(1), Ambito::Faccion);
        for objetivo in [Objetivo::Convocatoria(1), Objetivo::Ejercito(2)] {
            assert_eq!(ordenar(&c, &[Puesto::Rey], TipoOrden::Unirse, objetivo, &encabezado_por(OTRO)), Ok(Puesto::Rey), "{objetivo:?} propio");
            assert_eq!(ordenar(&c, &[Puesto::Rey], TipoOrden::Unirse, objetivo, &enemigo()), Err(ErrorOrden::ObjetivoInvalido), "{objetivo:?} ajeno");
        }
        for objetivo in [Objetivo::Plaza(1), Objetivo::Caravana(2), Objetivo::Bandidos(3), Objetivo::Faccion(NUESTRA), punto()] {
            assert_eq!(ordenar(&c, &[Puesto::Rey], TipoOrden::Unirse, objetivo, &propio(P)), Err(ErrorOrden::ObjetivoInvalido), "{objetivo:?}");
        }
    }

    #[test]
    fn un_objetivo_invalido_se_rechaza_antes_que_el_alcance() {
        let c = defensor(P);
        let ajena = Situacion { duena: Some(AJENA), ..encabezado_por(YO) };
        assert_eq!(ordenar(&c, &[Puesto::Gobernador(P)], TipoOrden::Unirse, Objetivo::Convocatoria(1), &ajena), Err(ErrorOrden::ObjetivoInvalido));
        assert_eq!(ordenar(&c, &[Puesto::Gobernador(P)], TipoOrden::Unirse, Objetivo::Convocatoria(1), &encabezado_por(YO)), Err(ErrorOrden::SinAlcance));
    }

    #[test]
    fn el_rey_y_el_embajador_mandan_a_todos_con_todas_las_ordenes() {
        let contratos = [defensor(Q), escolta(Q), permanente(), contrato(Trabajo::Acompanar, Objetivo::Ejercito(5), Ambito::Faccion)];
        for puesto in [Puesto::Rey, Puesto::Embajador] {
            for c in &contratos {
                assert_eq!(puesto_para_ordenar(c, None, &[puesto], TipoOrden::Guardar, &Objetivo::Plaza(P), &propio(P), YO), Ok(puesto), "{puesto:?} guardar {:?}", c.trabajo);
                assert_eq!(puesto_para_ordenar(c, None, &[puesto], TipoOrden::IrA, &punto(), &Situacion::default(), YO), Ok(puesto), "{puesto:?} ir_a {:?}", c.trabajo);
                assert_eq!(puesto_para_ordenar(c, None, &[puesto], TipoOrden::Atacar, &Objetivo::Plaza(80), &enemigo(), YO), Ok(puesto), "{puesto:?} atacar {:?}", c.trabajo);
                assert_eq!(
                    puesto_para_ordenar(c, None, &[puesto], TipoOrden::Unirse, &Objetivo::Convocatoria(1), &encabezado_por(OTRO), YO),
                    Ok(puesto),
                    "{puesto:?} unirse {:?}",
                    c.trabajo
                );
            }
        }
    }

    #[test]
    fn general_y_gobernador_alcanzan_los_contratos_de_su_plaza_y_los_permanentes_que_residen_en_ella() {
        for puesto in [Puesto::General(P), Puesto::Gobernador(P)] {
            let da = |c: &Contrato, residencia: Option<u64>| puesto_para_ordenar(c, residencia, &[puesto], TipoOrden::IrA, &punto(), &Situacion::default(), YO);
            assert_eq!(da(&defensor(P), None), Ok(puesto), "contrato de su plaza");
            assert_eq!(da(&defensor(Q), None), Err(ErrorOrden::SinAlcance), "contrato de otra plaza");
            assert_eq!(da(&permanente(), Some(P)), Ok(puesto), "permanente que reside en su plaza");
            assert_eq!(da(&permanente(), Some(Q)), Err(ErrorOrden::SinAlcance), "permanente que reside en otra plaza");
            assert_eq!(da(&permanente(), None), Err(ErrorOrden::SinAlcance), "permanente que no reside en una plaza");
            let de_la_faccion = contrato(Trabajo::Acompanar, Objetivo::Ejercito(5), Ambito::Faccion);
            assert_eq!(da(&de_la_faccion, Some(P)), Err(ErrorOrden::SinAlcance), "temporal de la Facción: no es permanente");
        }
    }

    #[test]
    fn el_maestro_de_caravanas_solo_alcanza_las_escoltas_de_su_plaza() {
        let da = |c: &Contrato, residencia: Option<u64>| puesto_para_ordenar(c, residencia, &[Puesto::MaestroCaravanas(P)], TipoOrden::IrA, &punto(), &Situacion::default(), YO);
        assert_eq!(da(&escolta(P), None), Ok(Puesto::MaestroCaravanas(P)));
        assert_eq!(da(&escolta(Q), None), Err(ErrorOrden::SinAlcance), "escolta de otra plaza");
        assert_eq!(da(&defensor(P), None), Err(ErrorOrden::SinAlcance), "contrato de su plaza que no es escolta");
        assert_eq!(da(&permanente(), Some(P)), Err(ErrorOrden::SinAlcance), "permanente que reside en su plaza");
        let escolta_de_la_faccion = contrato(Trabajo::Escoltar, Objetivo::Caravana(90), Ambito::Faccion);
        assert_eq!(da(&escolta_de_la_faccion, None), Err(ErrorOrden::SinAlcance), "escolta de la Facción");
    }

    #[test]
    fn el_general_guarda_objetivos_de_su_plaza_ataca_lo_que_sea_y_se_une_a_lo_que_encabeza() {
        let general = [Puesto::General(P)];
        let c = defensor(P);
        let ok = Ok(Puesto::General(P));
        assert_eq!(ordenar(&c, &general, TipoOrden::Guardar, Objetivo::Pueblo(5), &propio(P)), ok);
        assert_eq!(ordenar(&c, &general, TipoOrden::Guardar, Objetivo::Pueblo(6), &propio(Q)), Err(ErrorOrden::SinAlcance), "objetivo de otra plaza");
        assert_eq!(ordenar(&c, &general, TipoOrden::Atacar, Objetivo::Bandidos(7), &Situacion::default()), ok, "bandidos");
        assert_eq!(ordenar(&c, &general, TipoOrden::Atacar, Objetivo::Plaza(80), &enemigo()), ok, "enemigo en guerra");
        assert_eq!(ordenar(&c, &general, TipoOrden::IrA, punto(), &Situacion::default()), ok);
        assert_eq!(ordenar(&c, &general, TipoOrden::Unirse, Objetivo::Convocatoria(8), &encabezado_por(YO)), ok, "su convocatoria");
        assert_eq!(ordenar(&c, &general, TipoOrden::Unirse, Objetivo::Ejercito(9), &encabezado_por(YO)), ok, "su ejército");
        assert_eq!(ordenar(&c, &general, TipoOrden::Unirse, Objetivo::Convocatoria(8), &encabezado_por(OTRO)), Err(ErrorOrden::SinAlcance), "la de otro");
        assert_eq!(ordenar(&c, &general, TipoOrden::Unirse, Objetivo::Convocatoria(8), &propio(P)), Err(ErrorOrden::SinAlcance), "sin cabeza");
    }

    #[test]
    fn el_gobernador_guarda_objetivos_de_su_plaza_ataca_solo_bandidos_y_no_se_une() {
        let gobernador = [Puesto::Gobernador(P)];
        let c = defensor(P);
        let ok = Ok(Puesto::Gobernador(P));
        assert_eq!(ordenar(&c, &gobernador, TipoOrden::Guardar, Objetivo::Avanzada(5), &propio(P)), ok);
        assert_eq!(ordenar(&c, &gobernador, TipoOrden::Guardar, Objetivo::Avanzada(6), &propio(Q)), Err(ErrorOrden::SinAlcance), "objetivo de otra plaza");
        assert_eq!(ordenar(&c, &gobernador, TipoOrden::Atacar, Objetivo::Bandidos(7), &Situacion::default()), ok, "bandidos");
        assert_eq!(ordenar(&c, &gobernador, TipoOrden::Atacar, Objetivo::Plaza(80), &enemigo()), Err(ErrorOrden::SinAlcance), "enemigo en guerra");
        assert_eq!(ordenar(&c, &gobernador, TipoOrden::IrA, punto(), &Situacion::default()), ok);
        assert_eq!(ordenar(&c, &gobernador, TipoOrden::Unirse, Objetivo::Convocatoria(8), &encabezado_por(YO)), Err(ErrorOrden::SinAlcance), "ni a la suya");
    }

    #[test]
    fn el_maestro_de_caravanas_guarda_caravanas_de_su_plaza_y_va_a_un_punto() {
        let maestro = [Puesto::MaestroCaravanas(P)];
        let c = escolta(P);
        let ok = Ok(Puesto::MaestroCaravanas(P));
        assert_eq!(ordenar(&c, &maestro, TipoOrden::Guardar, Objetivo::Caravana(90), &propio(P)), ok);
        assert_eq!(ordenar(&c, &maestro, TipoOrden::Guardar, Objetivo::Caravana(91), &propio(Q)), Err(ErrorOrden::SinAlcance), "caravana de otra plaza");
        assert_eq!(ordenar(&c, &maestro, TipoOrden::Guardar, Objetivo::Plaza(P), &propio(P)), Err(ErrorOrden::SinAlcance), "no es una caravana");
        assert_eq!(ordenar(&c, &maestro, TipoOrden::IrA, punto(), &Situacion::default()), ok);
        assert_eq!(ordenar(&c, &maestro, TipoOrden::Atacar, Objetivo::Bandidos(7), &Situacion::default()), Err(ErrorOrden::SinAlcance));
        assert_eq!(ordenar(&c, &maestro, TipoOrden::Unirse, Objetivo::Convocatoria(8), &encabezado_por(YO)), Err(ErrorOrden::SinAlcance));
    }

    #[test]
    fn con_varios_cargos_se_usa_el_de_mas_rango_que_alcanza_y_puede() {
        let c = escolta(P);
        let guardar = |puestos: &[Puesto]| ordenar(&c, puestos, TipoOrden::Guardar, Objetivo::Caravana(90), &propio(P));
        assert_eq!(guardar(&[Puesto::MaestroCaravanas(P), Puesto::Gobernador(P), Puesto::Rey]), Ok(Puesto::Rey));
        assert_eq!(guardar(&[Puesto::Rey, Puesto::Embajador]), Ok(Puesto::Rey));
        assert_eq!(guardar(&[Puesto::Embajador, Puesto::Rey]), Ok(Puesto::Rey));
        assert_eq!(guardar(&[Puesto::MaestroCaravanas(P), Puesto::Gobernador(P)]), Ok(Puesto::Gobernador(P)));
        assert_eq!(guardar(&[Puesto::MaestroCaravanas(P), Puesto::General(Q)]), Ok(Puesto::MaestroCaravanas(P)), "el General de otra plaza no cuenta");
        assert_eq!(guardar(&[Puesto::General(Q), Puesto::Gobernador(Q)]), Err(ErrorOrden::SinAlcance));
        assert_eq!(guardar(&[]), Err(ErrorOrden::SinAlcance), "sin cargos");
        let ataque = ordenar(&c, &[Puesto::MaestroCaravanas(P), Puesto::Gobernador(P)], TipoOrden::Atacar, Objetivo::Bandidos(7), &Situacion::default());
        assert_eq!(ataque, Ok(Puesto::Gobernador(P)), "el Maestro no puede atacar, el Gobernador sí");
    }

    /// Qué admite el trabajo de `c`, por este orden: guardar su objetivo, guardar otro, ir_a, atacar a un radio de defensa y una milésima, atacar a un radio justo y unirse.
    fn admitidas(c: &Contrato) -> [bool; 6] {
        let d = Datos::cargar();
        let otro = Objetivo::Plaza(99);
        let ok = |tipo, objetivo: &Objetivo, distancia| admite(&d, c, tipo, objetivo, distancia).is_ok();
        [
            ok(TipoOrden::Guardar, &c.objetivo, None),
            ok(TipoOrden::Guardar, &otro, None),
            ok(TipoOrden::IrA, &punto(), None),
            ok(TipoOrden::Atacar, &otro, Some(d.radio_defensa + 1)),
            ok(TipoOrden::Atacar, &otro, Some(d.radio_defensa)),
            ok(TipoOrden::Unirse, &otro, None),
        ]
    }

    #[test]
    fn defender_admite_guardar_su_objetivo_ir_a_y_atacar_lo_que_este_en_el_radio() {
        assert_eq!(admitidas(&defensor(P)), [true, false, true, false, true, false]);
        let d = Datos::cargar();
        let atacar = |distancia| admite(&d, &defensor(P), TipoOrden::Atacar, &Objetivo::Bandidos(7), distancia);
        assert_eq!(atacar(Some(0)), Ok(()));
        assert_eq!(atacar(Some(d.radio_defensa)), Ok(()), "justo en el radio");
        assert_eq!(atacar(Some(d.radio_defensa + 1)), Err(ErrorOrden::FueraDeContrato));
        assert_eq!(atacar(None), Err(ErrorOrden::FueraDeContrato), "sin distancia no hay radio");
    }

    #[test]
    fn escoltar_admite_guardar_su_objetivo_e_ir_a() {
        assert_eq!(admitidas(&escolta(P)), [true, false, true, false, false, false]);
    }

    #[test]
    fn atacar_admite_atacar_lo_que_sea_e_ir_a() {
        let c = contrato(Trabajo::Atacar, Objetivo::Bandidos(7), Ambito::Faccion);
        assert_eq!(admitidas(&c), [false, false, true, true, true, false]);
    }

    #[test]
    fn acompanar_admite_unirse_atacar_ir_a_y_guardar() {
        // Su `guardar` vale para cualquier objetivo, no solo el del contrato.
        let c = contrato(Trabajo::Acompanar, Objetivo::Convocatoria(5), Ambito::Faccion);
        assert_eq!(admitidas(&c), [true, true, true, true, true, true]);
    }

    #[test]
    fn el_permanente_admite_todas() {
        assert_eq!(admitidas(&permanente()), [true, true, true, true, true, true]);
    }

    #[test]
    fn una_orden_sustituye_a_la_vigente_de_igual_o_menos_rango() {
        assert!(sustituye(None, Puesto::MaestroCaravanas(P)), "sin orden vigente");
        let vigente = |puesto| Some(orden_de(puesto));
        assert!(sustituye(vigente(Puesto::Gobernador(P)).as_ref(), Puesto::Rey));
        assert!(sustituye(vigente(Puesto::Gobernador(P)).as_ref(), Puesto::Embajador));
        assert!(sustituye(vigente(Puesto::MaestroCaravanas(P)).as_ref(), Puesto::Gobernador(P)));
        assert!(sustituye(vigente(Puesto::Rey).as_ref(), Puesto::Rey));
        assert!(sustituye(vigente(Puesto::MaestroCaravanas(P)).as_ref(), Puesto::MaestroCaravanas(P)));
    }

    #[test]
    fn una_orden_no_sustituye_a_la_vigente_de_mas_rango() {
        let vigente = |puesto| Some(orden_de(puesto));
        assert!(!sustituye(vigente(Puesto::Rey).as_ref(), Puesto::Embajador));
        assert!(!sustituye(vigente(Puesto::Embajador).as_ref(), Puesto::General(P)));
        assert!(!sustituye(vigente(Puesto::General(P)).as_ref(), Puesto::MaestroCaravanas(P)));
        assert!(!sustituye(vigente(Puesto::Gobernador(P)).as_ref(), Puesto::MaestroCaravanas(P)));
    }

    #[test]
    fn general_y_gobernador_empatan_y_manda_la_ultima() {
        let del_general = orden_de(Puesto::General(P));
        let del_gobernador = orden_de(Puesto::Gobernador(P));
        assert!(sustituye(Some(&del_general), Puesto::Gobernador(P)));
        assert!(sustituye(Some(&del_gobernador), Puesto::General(P)));
        let de_otra_plaza = orden_de(Puesto::General(Q));
        assert!(sustituye(Some(&de_otra_plaza), Puesto::Gobernador(P)), "el rango no mira la plaza");
    }
}
