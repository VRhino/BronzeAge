//! Ofrecerse, contratar y prorrogar (BAN54, entrada 48; Mercenarios_Por_Contrato §3 a §6.2): el bot residente y dentro de su campamento se pone en el tablón y se quita; un cargo de una Facción
//! humana, en persona y con el oro de su carro, lo contrata (el Rey y el Embajador todo; el Gobernador y el General su plaza; quien no tiene cargo, nada) y lo prorroga. Lo que cada firma deja en el estado
//! (la marca de ciudadanía, el contrato con su orden inicial, el pago en el almacén del mercenario, los hechos) y que cada rechazo no deja rastro. Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;

use asentamiento::{Cargo, Recurso};
use base::Mil;
use comun::mercenarios as mc;
use comun::real::{fundar, sitio_rico, M};
use mercenarios::contrato::duracion;
use mercenarios::{Ambito, Objetivo, Puesto, TipoOrden, Trabajo};
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::{rechazos, Comando, Nucleo, PagarCon, Relacion};
use politica::Marca;

macro_rules! con_mundo {
    () => {
        match comun::real::mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

/// Lo que vale por nivel un héroe sin tropa, en oro, y la fracción del plazo, con la cuarta parte de `factorOroPct` (tanda 69): con el héroe de nivel 1 y rango «a sueldo», 1 día son
/// ⌈20 × 15 % × 25 %⌉ = 1 de oro, 3 días ⌈20 × 35 % × 25 %⌉ = 2, 7 días ⌈20 × 70 % × 25 %⌉ = 4 y el permanente 20 × 500 % × 25 % = 25.
const UN_DIA: Mil = 1_000;
const TRES_DIAS: Mil = 2_000;
const SIETE_DIAS: Mil = 4_000;
const PERMANENTE: Mil = 25_000;

/// La Facción 1 (humana) con su Rey «rey», que espera con 500 de oro en el carro de su columna a la puerta del campamento 0; su plaza, fundada por «fundador» (Gobernador de oficio, fuera,
/// en su columna), y la plaza de la Facción 2 (de «gus»). Los mercenarios y los demás firmantes los pone cada prueba.
struct E {
    n: Nucleo,
    rey: u64,
    plaza: u64,
    ajena: u64,
}

impl E {
    fn nuevo(m: &M) -> Self {
        let mut n = mc::nucleo(m);
        // El primer ciudadano de la Facción 1 es su Rey.
        let rey = mc::firmante(&mut n, "rey", 1, 0, mc::ORO, Controlador::Humano);
        assert_eq!(n.faccion(1).and_then(|f| f.rey), Some(rey));
        let (sa, sb) = (sitio_rico(&n, &m.0, 3, 1), sitio_rico(&n, &m.0, 0, 2));
        let plaza = fundar(&mut n, "fundador", 1, sa);
        let ajena = fundar(&mut n, "gus", 2, sb);
        E { n, rey, plaza, ajena }
    }

    /// Un bot en el tablón del campamento 0.
    fn bot(&mut self, cuenta: &str) -> u64 {
        mc::bot_ofrecido(&mut self.n, cuenta, 0)
    }

    /// Un ciudadano de la Facción 1 con 500 de oro en el carro a la puerta del campamento 0, sin cargo.
    fn ciudadano(&mut self, cuenta: &str) -> u64 {
        mc::firmante(&mut self.n, cuenta, 1, 0, mc::ORO, Controlador::Humano)
    }

    /// Un Gobernador de la plaza de la Facción 1, nombrado por el Rey, a la puerta del campamento 0 con 500 de oro.
    fn gobernador(&mut self, cuenta: &str) -> u64 {
        let g = self.ciudadano(cuenta);
        let (rey, plaza) = (self.rey, self.plaza);
        mc::con_cargo(&mut self.n, "rey", rey, plaza, Cargo::Gobernador, g);
        g
    }

    /// `firmante` contrata, con su cuenta, un trabajo temporal (sin plaza donde residir).
    fn contratar(&mut self, firmante: u64, mercenario: u64, trabajo: Trabajo, objetivo: Objetivo, plazo: Option<u8>) -> nucleo::Respuesta {
        mc::contratar(&mut self.n, firmante, mercenario, trabajo, objetivo, plazo, None)
    }

    /// Lo mismo, exigiendo un rechazo que no deje rastro: devuelve su código.
    fn rechazado(&mut self, firmante: u64, mercenario: u64, trabajo: Trabajo, objetivo: Objetivo, plazo: Option<u8>) -> &'static str {
        mc::contratar_rechazado(&mut self.n, firmante, mercenario, trabajo, objetivo, plazo, None)
    }
}

#[test]
fn un_bot_residente_y_dentro_se_ofrece_en_el_tablon_y_se_retira() {
    let m = con_mundo!();
    let mut n = mc::nucleo(m);
    let bot = mc::heroe_en(&mut n, "bot-1", 2, Controlador::Bot);
    assert!(!mc::ofrecido(&n, bot) && !n.estado().contratos.fichas.contains_key(&bot));

    mc::ok(&mut n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });
    let oferta = n.estado().contratos.ofertas[&bot];
    assert_eq!((oferta.campamento, oferta.desde), (2, n.ahora()));
    assert!(n.estado().contratos.fichas.contains_key(&bot), "la ficha nace con el primer ofrecimiento");
    let hechos = mc::hechos(&n, "mercenarios.ofrecido");
    assert_eq!((hechos.len(), hechos[0].dato("heroe"), hechos[0].dato("campamento")), (1, Some(bot as i64), Some(2)));
    assert_eq!(hechos[0].heroes, vec![bot], "lo oye el héroe");

    // Ofrecerse otra vez no cambia nada.
    mc::sin_cambios(&mut n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });

    mc::ok(&mut n, "bot-1", Comando::RetirarOferta { heroe: bot });
    assert!(!mc::ofrecido(&n, bot));
    assert!(n.estado().contratos.fichas.contains_key(&bot), "la ficha es del héroe, no de la oferta");
    let hechos = mc::hechos(&n, "mercenarios.oferta_retirada");
    assert_eq!((hechos.len(), hechos[0].dato("heroe"), hechos[0].dato("campamento")), (1, Some(bot as i64), Some(2)));
    assert_eq!(hechos[0].heroes, vec![bot], "lo oye el héroe");
    mc::sin_cambios(&mut n, "bot-1", Comando::RetirarOferta { heroe: bot });

    // Y puede volver a ofrecerse.
    mc::ok(&mut n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });
    assert!(mc::ofrecido(&n, bot));
    assert_eq!(mc::hechos(&n, "mercenarios.ofrecido").len(), 2);
}

#[test]
fn solo_se_ofrece_un_bot_sin_faccion_que_reside_en_un_campamento_y_esta_dentro() {
    let m = con_mundo!();
    let mut n = mc::nucleo(m);
    let oferta = |h: u64| Comando::OfrecerseComoMercenario { heroe: h };

    let humano = mc::heroe_en(&mut n, "ana", 0, Controlador::Humano);
    assert_eq!(mc::rechazo(&mut n, "ana", oferta(humano)), rechazos::MERCENARIO_SOLO_BOTS);

    let con_faccion = mc::heroe_en(&mut n, "bot-1", 0, Controlador::Bot);
    mc::ok(&mut n, "admin", Comando::FijarFaccionDeHeroe { heroe: con_faccion, faccion: Some(1) });
    assert_eq!(mc::rechazo(&mut n, "bot-1", oferta(con_faccion)), rechazos::FACCION_YA_PERTENECE);

    let sin_casa = mc::heroe_en(&mut n, "bot-2", 0, Controlador::Bot);
    mc::ok(&mut n, "admin", Comando::FijarResidencia { heroe: sin_casa, residencia: None });
    assert_eq!(mc::rechazo(&mut n, "bot-2", oferta(sin_casa)), rechazos::MERCENARIOS_NO_RESIDES);

    let fuera = mc::heroe_en(&mut n, "bot-3", 0, Controlador::Bot);
    mc::salir_con_oro(&mut n, "bot-3", fuera, 0, 0);
    assert_eq!(mc::rechazo(&mut n, "bot-3", oferta(fuera)), rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA);

    // Dentro, pero de otro campamento que el suyo.
    let de_visita = mc::heroe_en(&mut n, "bot-4", 0, Controlador::Bot);
    mc::ok(&mut n, "admin", Comando::ColocarHeroe { heroe: de_visita, ubicacion: Ubicacion::Mercenarios(1) });
    assert_eq!(mc::rechazo(&mut n, "bot-4", oferta(de_visita)), rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA);

    // Ni la cuenta ajena lo ofrece.
    let ajeno = mc::heroe_en(&mut n, "bot-5", 0, Controlador::Bot);
    assert!(!mc::dar(&mut n, "ana", oferta(ajeno)).aceptada() && !mc::ofrecido(&n, ajeno));
    assert!(n.estado().contratos.ofertas.is_empty());
}

#[test]
fn la_oferta_se_retira_sola_cuando_el_bot_se_muda_de_residencia() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let bot = e.bot("bot-1");
    assert!(mc::ofrecido(&e.n, bot));
    let plaza = e.plaza as u32;
    mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe: bot, residencia: Some(Residencia::Asentamiento(plaza)) });
    assert!(!mc::ofrecido(&e.n, bot), "la oferta es del campamento donde se reside");
    let h = mc::hechos(&e.n, "mercenarios.oferta_retirada");
    assert_eq!((h.len(), h[0].dato("heroe"), h[0].dato("campamento")), (1, Some(bot as i64), Some(0)));
    assert_eq!(h[0].heroes, vec![bot], "lo oye el héroe");
}

#[test]
fn el_gobernador_contrata_la_defensa_de_su_plaza_y_paga_con_el_oro_de_su_carro() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, gob, plaza) = (e.bot("bot-1"), e.gobernador("gob"), e.plaza);
    assert_eq!(e.n.heroe(bot).unwrap().heroe.nivel, 1, "las tarifas de esta prueba son las del nivel 1");
    let (t, carro) = (e.n.ahora(), mc::oro_del_carro(&e.n, gob));

    let r = e.contratar(gob, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(3));
    assert!(r.aceptada(), "{r:?}");

    // Paga la tarifa del carro, que entra en el almacén del mercenario.
    assert_eq!(mc::oro_del_carro(&e.n, gob), carro - TRES_DIAS);
    assert_eq!(mc::oro_personal(&e.n, bot), TRES_DIAS);
    // Entra en la Facción con su marca y sale del tablón; sigue residiendo en su campamento.
    let h = e.n.heroe(bot).unwrap();
    assert_eq!((h.faccion, h.heroe.residencia), (Some(1), Some(Residencia::Campamento(0))));
    assert_eq!(e.n.estado().politica.facciones.marca_de(bot), Some(Marca::Temporal));
    assert!(!mc::ofrecido(&e.n, bot));
    // El contrato, con su primera orden: guardar la plaza, dada por quien firmó.
    let c = mc::contrato(&e.n, bot).expect("tiene contrato");
    assert_eq!((c.mercenario, c.faccion, c.ambito), (bot, 1, Ambito::Plaza(plaza)));
    assert_eq!((c.trabajo, c.objetivo, c.plazo), (Trabajo::Defender, Objetivo::Plaza(plaza), Some(3)));
    assert_eq!((c.firmado_por, c.puesto_firma, c.firmado_en), (gob, Puesto::Gobernador(plaza), t));
    assert_eq!((c.vence, c.campamento_base, c.fin_aplazado, c.renombre), (Some(t + duracion(3)), 0, None, 25));
    let o = c.orden.expect("nace con orden");
    assert_eq!((o.tipo, o.objetivo, o.dada_por, o.puesto, o.dada_en), (TipoOrden::Guardar, Objetivo::Plaza(plaza), gob, None, t));
    assert_eq!((o.vence, o.contra, o.unido), (None, None, false));
    assert_eq!(e.n.estado().contratos.fichas[&bot].historial.firmados, 1);
    // El hecho llega al mercenario y a la Facción.
    let hechos = mc::hechos(&e.n, "mercenarios.contratado");
    assert_eq!(hechos.len(), 1);
    let h = &hechos[0];
    assert_eq!((h.heroes.clone(), h.facciones.clone()), (vec![bot], vec![1]));
    assert_eq!((h.dato("faccion"), h.dato("mercenario"), h.dato("firmante"), h.dato("plazo"), h.dato("oro"), h.dato("campamento")), (Some(1), Some(bot as i64), Some(gob as i64), Some(3), Some(TRES_DIAS), Some(0)));
}

#[test]
fn el_rey_contrata_un_permanente_que_pasa_a_residir_en_la_plaza_y_su_orden_es_guardarla() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    let (t, carro) = (e.n.ahora(), mc::oro_del_carro(&e.n, rey));

    mc::ok(&mut e.n, "rey", Comando::ContratarMercenario { heroe: rey, mercenario: bot, trabajo: Trabajo::Permanente, objetivo: Objetivo::Faccion(1), plazo: None, residencia: Some(plaza), pagar_con: None });

    assert_eq!(mc::oro_del_carro(&e.n, rey), carro - PERMANENTE);
    assert_eq!(mc::oro_personal(&e.n, bot), PERMANENTE);
    let h = e.n.heroe(bot).unwrap();
    assert_eq!((h.faccion, h.heroe.residencia), (Some(1), Some(Residencia::Asentamiento(plaza as u32))), "reside en la plaza");
    assert_eq!(h.residencia_cambiada_en, None, "sin cooldown");
    assert_eq!(e.n.estado().politica.facciones.marca_de(bot), Some(Marca::Permanente));
    assert!(!mc::ofrecido(&e.n, bot));
    let c = mc::contrato(&e.n, bot).unwrap();
    assert_eq!((c.ambito, c.trabajo, c.objetivo, c.plazo, c.vence), (Ambito::Faccion, Trabajo::Permanente, Objetivo::Faccion(1), None, None));
    assert_eq!((c.firmado_por, c.puesto_firma, c.firmado_en, c.renombre), (rey, Puesto::Rey, t, 0));
    let o = c.orden.unwrap();
    assert_eq!((o.tipo, o.objetivo, o.dada_por, o.puesto, o.vence), (TipoOrden::Guardar, Objetivo::Plaza(plaza), rey, None, None));
    // Un permanente no se prorroga.
    assert_eq!(mc::rechazo(&mut e.n, "rey", Comando::ProrrogarContrato { heroe: rey, mercenario: bot, plazo: 3, pagar_con: None }), rechazos::MERCENARIO_TIPO_DE_CONTRATO);
}

#[test]
fn el_permanente_pide_una_plaza_de_la_faccion_y_solo_el_permanente_reside_en_una() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza, ajena) = (e.bot("bot-1"), e.rey, e.plaza, e.ajena);
    let c = |trabajo, objetivo, plazo, residencia| Comando::ContratarMercenario { heroe: rey, mercenario: bot, trabajo, objetivo, plazo, residencia, pagar_con: None };
    let malo = rechazos::MERCENARIO_OBJETIVO_INVALIDO;
    assert_eq!(mc::rechazo(&mut e.n, "rey", c(Trabajo::Permanente, Objetivo::Faccion(1), None, None)), malo, "sin plaza donde residir");
    assert_eq!(mc::rechazo(&mut e.n, "rey", c(Trabajo::Permanente, Objetivo::Faccion(1), None, Some(ajena))), malo, "una plaza de otra Facción");
    assert_eq!(mc::rechazo(&mut e.n, "rey", c(Trabajo::Permanente, Objetivo::Faccion(1), None, Some(999_999))), malo, "una plaza que no existe");
    assert_eq!(mc::rechazo(&mut e.n, "rey", c(Trabajo::Permanente, Objetivo::Faccion(2), None, Some(plaza))), malo, "el permanente es de la propia Facción");
    assert_eq!(mc::rechazo(&mut e.n, "rey", c(Trabajo::Defender, Objetivo::Plaza(plaza), Some(1), Some(plaza))), malo, "el temporal no reside en la plaza");
    assert_eq!(mc::rechazo(&mut e.n, "rey", c(Trabajo::Permanente, Objetivo::Faccion(1), Some(3), Some(plaza))), rechazos::MERCENARIO_PLAZO_INVALIDO);
    assert!(mc::ofrecido(&e.n, bot) && mc::contrato(&e.n, bot).is_none());
}

#[test]
fn el_plazo_ha_de_ser_uno_de_la_tabla_y_el_permanente_no_lleva_plazo() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    let malo = rechazos::MERCENARIO_PLAZO_INVALIDO;
    for plazo in [None, Some(0), Some(2), Some(30)] {
        assert_eq!(e.rechazado(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), plazo), malo, "{plazo:?}");
    }
    // Los tres de la tabla valen: 1, 3 y 7 días, cada uno con su tarifa.
    for (dias, tarifa) in [(1, UN_DIA), (3, TRES_DIAS), (7, SIETE_DIAS)] {
        let bot = e.bot(&format!("plazo-{dias}"));
        let carro = mc::oro_del_carro(&e.n, rey);
        assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(dias)).aceptada());
        assert_eq!(carro - mc::oro_del_carro(&e.n, rey), tarifa, "{dias} días");
        assert_eq!(mc::contrato(&e.n, bot).unwrap().vence, Some(e.n.ahora() + duracion(dias)));
    }
}

#[test]
fn el_objetivo_ha_de_existir_y_ser_de_una_clase_que_admita_el_trabajo() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza, ajena) = (e.bot("bot-1"), e.rey, e.plaza, e.ajena);
    let malo = rechazos::MERCENARIO_OBJETIVO_INVALIDO;
    for (trabajo, objetivo) in [
        (Trabajo::Defender, Objetivo::Plaza(999_999)),
        (Trabajo::Defender, Objetivo::Faccion(1)),
        (Trabajo::Defender, Objetivo::Plaza(ajena)),
        (Trabajo::Escoltar, Objetivo::Plaza(plaza)),
        (Trabajo::Acompanar, Objetivo::Plaza(plaza)),
        (Trabajo::Atacar, Objetivo::Punto(base::Punto::new(0, 0))),
        (Trabajo::Atacar, Objetivo::Plaza(plaza)),
        (Trabajo::Atacar, Objetivo::Plaza(ajena)),
    ] {
        assert_eq!(e.rechazado(rey, bot, trabajo, objetivo, Some(1)), malo, "{trabajo:?} {objetivo:?}");
    }
}

#[test]
fn el_rey_ataca_una_plaza_en_guerra_y_su_orden_inicial_caduca_y_recuerda_a_quien_se_ataca() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, gob, ajena) = (e.bot("bot-1"), e.rey, e.gobernador("gob"), e.ajena);
    assert_eq!(e.rechazado(rey, bot, Trabajo::Atacar, Objetivo::Plaza(ajena), Some(1)), rechazos::MERCENARIO_OBJETIVO_INVALIDO, "en paz no se ataca");
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    // Un Gobernador solo ataca bandidos de su zona.
    assert_eq!(e.rechazado(gob, bot, Trabajo::Atacar, Objetivo::Plaza(ajena), Some(1)), rechazos::MERCENARIO_SIN_ALCANCE);

    let t = e.n.ahora();
    assert!(e.contratar(rey, bot, Trabajo::Atacar, Objetivo::Plaza(ajena), Some(1)).aceptada());
    let c = mc::contrato(&e.n, bot).unwrap();
    assert_eq!((c.ambito, c.puesto_firma, c.renombre), (Ambito::Faccion, Puesto::Rey, 10));
    let o = c.orden.unwrap();
    assert_eq!((o.tipo, o.objetivo, o.dada_por, o.contra), (TipoOrden::Atacar, Objetivo::Plaza(ajena), rey, Some(2)));
    assert_eq!(o.vence, Some(t + e.n.estado().reglas.economia.contratos.plazo_orden));
}

#[test]
fn el_rey_contrata_acompanar_a_un_ejercito_de_su_faccion_y_no_al_de_otra() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey) = (e.bot("bot-1"), e.rey);
    let aqui = comun::real::campamento(&m.0, 1);
    let (t, propio, ajeno) = (e.n.ahora(), e.n.crear_ejercito(e.n.ahora(), "capi", 1, aqui, 500, 100_000).unwrap(), e.n.crear_ejercito(e.n.ahora(), "otro", 2, aqui, 500, 100_000).unwrap());
    assert_eq!(e.rechazado(rey, bot, Trabajo::Acompanar, Objetivo::Ejercito(ajeno), Some(1)), rechazos::MERCENARIO_OBJETIVO_INVALIDO);
    assert_eq!(e.rechazado(rey, bot, Trabajo::Acompanar, Objetivo::Ejercito(999_999), Some(1)), rechazos::MERCENARIO_OBJETIVO_INVALIDO);

    assert!(e.contratar(rey, bot, Trabajo::Acompanar, Objetivo::Ejercito(propio), Some(7)).aceptada());
    let o = mc::contrato(&e.n, bot).unwrap().orden.unwrap();
    assert_eq!((o.tipo, o.objetivo, o.dada_por, o.dada_en, o.vence, o.contra, o.unido), (TipoOrden::Unirse, Objetivo::Ejercito(propio), rey, t, None, None, false));
}

#[test]
fn quien_no_tiene_cargo_o_lo_tiene_de_otra_plaza_o_de_otro_trabajo_no_firma() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, plaza) = (e.bot("bot-1"), e.plaza);
    let sin_cargo = e.ciudadano("ana");
    assert_eq!(e.rechazado(sin_cargo, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)), rechazos::MERCENARIO_SIN_ALCANCE);
    // El Gobernador firma la defensa de su plaza, pero no un permanente ni acompañar.
    let gob = e.gobernador("gob");
    assert_eq!(mc::contratar_rechazado(&mut e.n, gob, bot, Trabajo::Permanente, Objetivo::Faccion(1), None, Some(plaza)), rechazos::MERCENARIO_SIN_ALCANCE);
    let aqui = comun::real::campamento(&m.0, 1);
    let ejercito = e.n.crear_ejercito(e.n.ahora(), "capi", 1, aqui, 500, 100_000).unwrap();
    assert_eq!(e.rechazado(gob, bot, Trabajo::Acompanar, Objetivo::Ejercito(ejercito), Some(1)), rechazos::MERCENARIO_SIN_ALCANCE);
    // Un Gobernador que ya no reside en su plaza no ejerce.
    mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe: gob, residencia: Some(Residencia::Campamento(0)) });
    assert_eq!(e.rechazado(gob, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)), rechazos::MERCENARIO_SIN_ALCANCE);
    assert!(mc::ofrecido(&e.n, bot));
}

#[test]
fn el_general_defiende_su_plaza_y_ataca_a_un_enemigo_en_guerra_pero_no_firma_un_permanente() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, otro, plaza, ajena) = (e.bot("bot-1"), e.bot("bot-2"), e.plaza, e.ajena);
    // El Gobernador de oficio (quien fundó) ya está dentro de su plaza (entró al fundarla) y nombra General a un residente.
    let fundador = e.n.faccion(1).unwrap().ciudadanos.iter().copied().find(|&h| h != e.rey).expect("el fundador");
    let general = e.ciudadano("gen");
    mc::con_cargo(&mut e.n, "fundador", fundador, plaza, Cargo::General, general);

    assert!(e.contratar(general, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    let c = mc::contrato(&e.n, bot).unwrap();
    assert_eq!((c.firmado_por, c.puesto_firma, c.ambito), (general, Puesto::General(plaza), Ambito::Plaza(plaza)));
    // Ataca a un enemigo en guerra, no a una Facción en paz; el permanente no es suyo.
    assert_eq!(e.rechazado(general, otro, Trabajo::Atacar, Objetivo::Plaza(ajena), Some(1)), rechazos::MERCENARIO_OBJETIVO_INVALIDO, "en paz no se ataca");
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    assert_eq!(mc::contratar_rechazado(&mut e.n, general, otro, Trabajo::Permanente, Objetivo::Faccion(1), None, Some(plaza)), rechazos::MERCENARIO_SIN_ALCANCE);
    assert!(e.contratar(general, otro, Trabajo::Atacar, Objetivo::Plaza(ajena), Some(1)).aceptada());
    let c = mc::contrato(&e.n, otro).unwrap();
    assert_eq!((c.puesto_firma, c.ambito, c.orden.unwrap().contra), (Puesto::General(plaza), Ambito::Plaza(plaza), Some(2)));
}

#[test]
fn no_se_contrata_a_quien_no_esta_en_el_tablon_ni_a_quien_ya_es_de_una_faccion() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza) = (e.rey, e.plaza);
    let humano = mc::heroe_en(&mut e.n, "ana", 0, Controlador::Humano);
    assert_eq!(e.rechazado(rey, humano, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)), rechazos::MERCENARIO_NO_OFRECIDO);
    assert_eq!(e.rechazado(rey, 999_999, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)), rechazos::MERCENARIO_NO_OFRECIDO);

    let bot = e.bot("bot-1");
    assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    // Ya contratado, deja el tablón: nadie más lo contrata.
    assert_eq!(e.rechazado(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)), rechazos::MERCENARIO_NO_OFRECIDO);

    // Un ofrecido al que la administración dio Facción por otra vía sigue en el tablón, pero no se contrata.
    let otro = e.bot("bot-2");
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: otro, faccion: Some(2) });
    assert_eq!(e.rechazado(rey, otro, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)), rechazos::FACCION_YA_PERTENECE);
}

#[test]
fn una_faccion_de_bots_no_contrata_y_quien_firma_dentro_paga_con_lo_que_tiene_alli() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza) = (e.rey, e.plaza);
    let bot = e.bot("bot-1");
    // La Facción 3 tiene un Rey bot.
    let rey_bot = mc::firmante(&mut e.n, "bot-rey", 3, 0, mc::ORO, Controlador::Bot);
    assert_eq!(e.rechazado(rey_bot, bot, Trabajo::Permanente, Objetivo::Faccion(3), None), rechazos::MERCENARIO_FACCION_DE_BOTS);

    // Sin Facción no se firma.
    let sin = mc::heroe_en(&mut e.n, "sin", 0, Controlador::Humano);
    assert_eq!(e.rechazado(sin, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)), rechazos::MERCENARIO_SIN_ALCANCE);

    // Un Embajador dentro del campamento donde reside, sin columna (2026-11-03): con el carro vacío no paga; sin decir con qué, paga con su almacén personal.
    let dentro = mc::heroe_en(&mut e.n, "dentro", 0, Controlador::Humano);
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: dentro, faccion: Some(1) });
    mc::ok(&mut e.n, "rey", Comando::AsignarEmbajador { heroe: rey, faccion: 1, designado: dentro });
    mc::ok(&mut e.n, "admin", Comando::DarAlmacenPersonal { heroe: dentro, recurso: Recurso::Oro, cantidad: mc::ORO });
    let con_carro = Comando::ContratarMercenario { heroe: dentro, mercenario: bot, trabajo: Trabajo::Defender, objetivo: Objetivo::Plaza(plaza), plazo: Some(1), residencia: None, pagar_con: Some(PagarCon::Carro) };
    assert_eq!(mc::rechazo(&mut e.n, "dentro", con_carro), rechazos::MERCENARIOS_SIN_ORO);
    assert!(mc::ofrecido(&e.n, bot));
    assert!(e.contratar(dentro, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    assert_eq!(mc::oro_personal(&e.n, dentro), mc::ORO - UN_DIA);
}

#[test]
fn hay_que_estar_en_el_campamento_del_mercenario_y_con_oro_bastante_en_el_carro() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    // Un firmante a la puerta de otro campamento.
    let lejos = mc::firmante(&mut e.n, "lejos", 1, 3, mc::ORO, Controlador::Humano);
    mc::ok(&mut e.n, "rey", Comando::AsignarEmbajador { heroe: rey, faccion: 1, designado: lejos });
    assert_eq!(e.rechazado(lejos, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)), rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA);
    // Uno sin oro bastante: 1 de oro no paga 2 (tres días).
    let pobre = mc::firmante(&mut e.n, "pobre", 1, 0, UN_DIA, Controlador::Humano);
    mc::ok(&mut e.n, "rey", Comando::AsignarEmbajador { heroe: rey, faccion: 1, designado: pobre });
    assert_eq!(e.rechazado(pobre, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(3)), rechazos::MERCENARIOS_SIN_ORO);
    assert_eq!(mc::oro_del_carro(&e.n, pobre), UN_DIA);
    // Con el oro justo, sí: una tarifa de 1 de oro.
    let justo = mc::firmante(&mut e.n, "justo", 1, 0, UN_DIA, Controlador::Humano);
    mc::ok(&mut e.n, "rey", Comando::AsignarEmbajador { heroe: rey, faccion: 1, designado: justo });
    assert!(e.contratar(justo, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    assert_eq!(mc::oro_del_carro(&e.n, justo), 0);
}

#[test]
fn la_tarifa_suma_lo_que_costaria_reclutar_su_tropa() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    let tropa = militar::catalogo::Tropa::MiliciaLanceros;
    mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: bot, tropa, cantidad: 10 });
    let mr = &e.n.estado().reglas.economia.militar;
    let valor_tropa = militar::mercenarios::precio(mr, mr.tropas.tropa(tropa), 10, asentamiento::datos::UNO, false);
    assert!(valor_tropa > 0);
    // (20 de oro por nivel + la tropa) × 35 % de tres días × 25 % de `factorOroPct`, redondeado hacia arriba a oro entero.
    let esperado = ((20_000 + valor_tropa) * 35 * 25 + 9_999_999) / 10_000_000 * 1_000;
    let carro = mc::oro_del_carro(&e.n, rey);
    assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(3)).aceptada());
    assert_eq!(carro - mc::oro_del_carro(&e.n, rey), esperado);
    assert!(esperado > TRES_DIAS);
}

#[test]
fn lo_que_no_cabe_en_el_almacen_del_mercenario_desaparece_y_no_impide_la_firma() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza) = (e.rey, e.plaza);
    let capacidad = e.n.estado().reglas.economia.comercio.mercenarios.capacidad_almacen_personal;
    // Un bot con el almacén casi lleno: cabe 1 de oro más y la tarifa es de 2.
    let bot = mc::heroe_en(&mut e.n, "bot-1", 0, Controlador::Bot);
    mc::ok(&mut e.n, "admin", Comando::DarAlmacenPersonal { heroe: bot, recurso: Recurso::Oro, cantidad: capacidad - UN_DIA });
    mc::ok(&mut e.n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });
    let carro = mc::oro_del_carro(&e.n, rey);
    assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(3)).aceptada());
    assert_eq!(mc::oro_personal(&e.n, bot), capacidad, "se llena y no pasa de ahí");
    assert_eq!(mc::oro_del_carro(&e.n, rey), carro - TRES_DIAS, "el firmante paga la tarifa entera");
    // Con el almacén lleno, el pago desaparece del todo.
    let otro = mc::heroe_en(&mut e.n, "bot-2", 0, Controlador::Bot);
    mc::ok(&mut e.n, "admin", Comando::DarAlmacenPersonal { heroe: otro, recurso: Recurso::Oro, cantidad: capacidad });
    mc::ok(&mut e.n, "bot-2", Comando::OfrecerseComoMercenario { heroe: otro });
    let carro = mc::oro_del_carro(&e.n, rey);
    assert!(e.contratar(rey, otro, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    assert_eq!((mc::oro_personal(&e.n, otro), mc::oro_del_carro(&e.n, rey)), (capacidad, carro - UN_DIA));
}

#[test]
fn prorrogar_alarga_el_contrato_cobra_la_tarifa_del_plazo_y_suma_renombre() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, gob, plaza) = (e.bot("bot-1"), e.gobernador("gob"), e.plaza);
    assert!(e.contratar(gob, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    let c0 = mc::contrato(&e.n, bot).unwrap();
    let (carro, personal) = (mc::oro_del_carro(&e.n, gob), mc::oro_personal(&e.n, bot));

    mc::ok(&mut e.n, "gob", Comando::ProrrogarContrato { heroe: gob, mercenario: bot, plazo: 3, pagar_con: None });
    let c = mc::contrato(&e.n, bot).unwrap();
    assert_eq!(c.vence, Some(c0.vence.unwrap() + duracion(3)), "se suma al vencimiento, no a hoy");
    assert_eq!(c.renombre, c0.renombre + 25);
    assert_eq!((c.plazo, c.firmado_en, c.firmado_por, c.orden), (c0.plazo, c0.firmado_en, c0.firmado_por, c0.orden), "lo demás no cambia");
    assert_eq!((mc::oro_del_carro(&e.n, gob), mc::oro_personal(&e.n, bot)), (carro - TRES_DIAS, personal + TRES_DIAS));
    let h = mc::hechos(&e.n, "mercenarios.prorrogado");
    assert_eq!((h.len(), h[0].heroes.clone(), h[0].facciones.clone()), (1, vec![bot], vec![1]));
    assert_eq!((h[0].dato("plazo"), h[0].dato("oro"), h[0].dato("firmante")), (Some(3), Some(TRES_DIAS), Some(gob as i64)));
    // Se puede prorrogar otra vez.
    mc::ok(&mut e.n, "gob", Comando::ProrrogarContrato { heroe: gob, mercenario: bot, plazo: 7, pagar_con: None });
    assert_eq!(mc::contrato(&e.n, bot).unwrap().vence, Some(c0.vence.unwrap() + duracion(3) + duracion(7)));
}

#[test]
fn prorrogar_pide_un_contrato_de_su_faccion_un_plazo_de_la_tabla_estar_en_persona_y_oro() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, gob, plaza) = (e.bot("bot-1"), e.rey, e.gobernador("gob"), e.plaza);
    let prorrogar = |heroe, mercenario, plazo| Comando::ProrrogarContrato { heroe, mercenario, plazo, pagar_con: None };
    // Sin contrato.
    assert_eq!(mc::rechazo(&mut e.n, "rey", prorrogar(rey, bot, 1)), rechazos::MERCENARIO_SIN_CONTRATO);
    assert!(e.contratar(gob, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    // Un firmante de otra Facción no puede.
    let de_otra = mc::firmante(&mut e.n, "otro", 2, 0, mc::ORO, Controlador::Humano);
    assert_eq!(mc::rechazo(&mut e.n, "otro", prorrogar(de_otra, bot, 1)), rechazos::MERCENARIO_SIN_CONTRATO);
    // Un ciudadano sin cargo, tampoco.
    let sin_cargo = e.ciudadano("ana");
    assert_eq!(mc::rechazo(&mut e.n, "ana", prorrogar(sin_cargo, bot, 1)), rechazos::MERCENARIO_SIN_ALCANCE);
    // Un plazo que no es de la tabla.
    assert_eq!(mc::rechazo(&mut e.n, "gob", prorrogar(gob, bot, 2)), rechazos::MERCENARIO_PLAZO_INVALIDO);
    // En persona: el Rey, que puede firmarlo, a la puerta de otro campamento, no.
    let c = mc::contrato(&e.n, bot).unwrap();
    let lejos = mc::firmante(&mut e.n, "lejos", 1, 3, mc::ORO, Controlador::Humano);
    mc::ok(&mut e.n, "rey", Comando::AsignarEmbajador { heroe: rey, faccion: 1, designado: lejos });
    assert_eq!(mc::rechazo(&mut e.n, "lejos", prorrogar(lejos, bot, 1)), rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA);
    // Sin oro.
    let pobre = mc::firmante(&mut e.n, "pobre", 1, 0, UN_DIA, Controlador::Humano);
    mc::ok(&mut e.n, "rey", Comando::AsignarEmbajador { heroe: rey, faccion: 1, designado: pobre });
    assert_eq!(mc::rechazo(&mut e.n, "pobre", prorrogar(pobre, bot, 3)), rechazos::MERCENARIOS_SIN_ORO);
    assert_eq!(mc::contrato(&e.n, bot).unwrap(), c, "ningún rechazo toca el contrato");
    // El Rey, a la puerta del campamento base, sí.
    mc::ok(&mut e.n, "rey", prorrogar(rey, bot, 1));
    assert_eq!(mc::contrato(&e.n, bot).unwrap().vence, Some(c.vence.unwrap() + duracion(1)));
}

/// El oro del carro que el héroe tiene consigo dentro del campamento donde reside, sin columna.
fn oro_del_carro_propio(n: &Nucleo, heroe: u64) -> Mil {
    n.heroe(heroe).expect("el héroe existe").carro.get(&Recurso::Oro).copied().unwrap_or(0)
}

#[test]
fn dentro_del_campamento_donde_reside_se_paga_con_el_almacen_personal_o_con_el_carro() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (uno, dos, tres, rey, plaza) = (e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3"), e.rey, e.plaza);
    // El Rey entra en el campamento 0, donde reside: su columna se deshace y el oro del carro se queda con él. En el almacén personal, 1 de oro.
    mc::ok(&mut e.n, "rey", Comando::EntrarEnCampamento { campamento: 0, heroe: rey });
    assert_eq!(e.n.estado().ejercito_de_heroe(rey), None, "dentro y sin columna");
    mc::ok(&mut e.n, "admin", Comando::DarAlmacenPersonal { heroe: rey, recurso: Recurso::Oro, cantidad: UN_DIA });
    let (personal, carro) = (mc::oro_personal(&e.n, rey), oro_del_carro_propio(&e.n, rey));
    assert_eq!((personal, carro), (UN_DIA, mc::ORO));
    let contratar = |mercenario: u64, plazo: u8, pagar_con: Option<PagarCon>| Comando::ContratarMercenario {
        heroe: rey,
        mercenario,
        trabajo: Trabajo::Defender,
        objetivo: Objetivo::Plaza(plaza),
        plazo: Some(plazo),
        residencia: None,
        pagar_con,
    };

    // Sin decir con qué, paga con el almacén personal, y el pago entra en el del mercenario.
    mc::ok(&mut e.n, "rey", contratar(uno, 1, None));
    assert_eq!((mc::oro_personal(&e.n, rey), oro_del_carro_propio(&e.n, rey)), (personal - UN_DIA, carro));
    assert_eq!(mc::oro_personal(&e.n, uno), UN_DIA);
    assert_eq!(mc::hechos(&e.n, "mercenarios.contratado")[0].dato("oro"), Some(UN_DIA));
    // Se queda sin oro en el almacén: no paga otra tarifa de 1, ni por defecto ni pedido a propósito.
    assert_eq!(mc::rechazo(&mut e.n, "rey", contratar(dos, 1, None)), rechazos::MERCENARIOS_SIN_ORO);
    assert_eq!(mc::rechazo(&mut e.n, "rey", contratar(dos, 1, Some(PagarCon::AlmacenPersonal))), rechazos::MERCENARIOS_SIN_ORO);
    // Con el carro que trajo al entrar, sí.
    mc::ok(&mut e.n, "rey", contratar(dos, 1, Some(PagarCon::Carro)));
    assert_eq!((mc::oro_personal(&e.n, rey), oro_del_carro_propio(&e.n, rey)), (personal - UN_DIA, carro - UN_DIA));
    assert_eq!(mc::oro_personal(&e.n, dos), UN_DIA);
    // Y el almacén personal pedido a propósito, en cuanto vuelve a tener oro bastante.
    mc::ok(&mut e.n, "admin", Comando::DarAlmacenPersonal { heroe: rey, recurso: Recurso::Oro, cantidad: TRES_DIAS });
    mc::ok(&mut e.n, "rey", contratar(tres, 3, Some(PagarCon::AlmacenPersonal)));
    assert_eq!((mc::oro_personal(&e.n, rey), oro_del_carro_propio(&e.n, rey)), (personal - UN_DIA, carro - UN_DIA));
    assert_eq!(mc::contrato(&e.n, tres).map(|c| c.plazo), Some(Some(3)));
}

#[test]
fn el_almacen_personal_solo_paga_en_el_campamento_donde_se_reside() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (uno, dos, rey, plaza) = (e.bot("bot-1"), e.bot("bot-2"), e.rey, e.plaza);
    let contratar = |heroe: u64, mercenario: u64, pagar_con: Option<PagarCon>| Comando::ContratarMercenario {
        heroe,
        mercenario,
        trabajo: Trabajo::Defender,
        objetivo: Objetivo::Plaza(plaza),
        plazo: Some(1),
        residencia: None,
        pagar_con,
    };
    // Un Embajador con su columna a la puerta del campamento 0 que reside en el 3: allí no tiene su almacén personal.
    let visita = e.ciudadano("visita");
    mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe: visita, residencia: Some(Residencia::Campamento(3)) });
    mc::ok(&mut e.n, "admin", Comando::DarAlmacenPersonal { heroe: visita, recurso: Recurso::Oro, cantidad: mc::ORO });
    mc::ok(&mut e.n, "rey", Comando::AsignarEmbajador { heroe: rey, faccion: 1, designado: visita });
    assert_eq!(mc::rechazo(&mut e.n, "visita", contratar(visita, uno, Some(PagarCon::AlmacenPersonal))), rechazos::ALMACEN_PERSONAL_FUERA_DE_TU_RESIDENCIA);
    // Sin decir con qué, paga con el carro de su columna, como siempre.
    let carro = mc::oro_del_carro(&e.n, visita);
    mc::ok(&mut e.n, "visita", contratar(visita, uno, None));
    assert_eq!(mc::oro_del_carro(&e.n, visita), carro - UN_DIA);
    // El Rey reside en el campamento 0: con su columna a la puerta paga por defecto con el carro y, si lo pide, con el almacén personal.
    mc::ok(&mut e.n, "admin", Comando::DarAlmacenPersonal { heroe: rey, recurso: Recurso::Oro, cantidad: UN_DIA });
    let (personal, carro) = (mc::oro_personal(&e.n, rey), mc::oro_del_carro(&e.n, rey));
    mc::ok(&mut e.n, "rey", contratar(rey, dos, Some(PagarCon::AlmacenPersonal)));
    assert_eq!((mc::oro_personal(&e.n, rey), mc::oro_del_carro(&e.n, rey)), (personal - UN_DIA, carro));
}

#[test]
fn prorrogar_desde_dentro_del_campamento_base_paga_con_el_almacen_personal_o_con_el_carro() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    let c0 = mc::contrato(&e.n, bot).unwrap();
    // El Rey entra en el campamento base del mercenario, donde reside, con 2 de oro (tres días) en el almacén personal.
    mc::ok(&mut e.n, "rey", Comando::EntrarEnCampamento { campamento: 0, heroe: rey });
    mc::ok(&mut e.n, "admin", Comando::DarAlmacenPersonal { heroe: rey, recurso: Recurso::Oro, cantidad: TRES_DIAS });
    let (personal, carro) = (mc::oro_personal(&e.n, rey), oro_del_carro_propio(&e.n, rey));
    assert_eq!(personal, TRES_DIAS);
    let prorrogar = |plazo: u8, pagar_con: Option<PagarCon>| Comando::ProrrogarContrato { heroe: rey, mercenario: bot, plazo, pagar_con };
    // Sin decir con qué, con el almacén personal; con el carro, si lo pide.
    mc::ok(&mut e.n, "rey", prorrogar(3, None));
    assert_eq!((mc::oro_personal(&e.n, rey), oro_del_carro_propio(&e.n, rey)), (0, carro));
    mc::ok(&mut e.n, "rey", prorrogar(1, Some(PagarCon::Carro)));
    assert_eq!((mc::oro_personal(&e.n, rey), oro_del_carro_propio(&e.n, rey)), (0, carro - UN_DIA));
    assert_eq!(mc::contrato(&e.n, bot).unwrap().vence, Some(c0.vence.unwrap() + duracion(3) + duracion(1)));
    // Con el almacén personal ya vacío, no.
    assert_eq!(mc::rechazo(&mut e.n, "rey", prorrogar(1, None)), rechazos::MERCENARIOS_SIN_ORO);
}

/// Las monedas del héroe.
fn monedas(n: &Nucleo, heroe: u64) -> i64 {
    n.heroe(heroe).expect("el héroe existe").heroe.monedas
}

#[test]
fn el_contrato_pagado_en_monedas_las_saca_del_firmante_y_las_lleva_al_mercenario() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    let por_oro = i64::from(e.n.estado().reglas.economia.contratos.monedas_por_oro);
    // La tarifa en monedas es la de oro, ya a un cuarto, a `monedasPorOro` monedas por oro: 3 días son 2 de oro.
    let tarifa = TRES_DIAS / base::UNIDAD * por_oro;
    let (del_rey, del_bot, carro) = (monedas(&e.n, rey), monedas(&e.n, bot), mc::oro_del_carro(&e.n, rey));
    let c = Comando::ContratarMercenario { heroe: rey, mercenario: bot, trabajo: Trabajo::Defender, objetivo: Objetivo::Plaza(plaza), plazo: Some(3), residencia: None, pagar_con: Some(PagarCon::Monedas) };
    mc::ok(&mut e.n, "rey", c);
    assert_eq!((monedas(&e.n, rey), monedas(&e.n, bot)), (del_rey - tarifa, del_bot + tarifa));
    assert_eq!(mc::oro_del_carro(&e.n, rey), carro, "el oro no se toca");
    assert_eq!(mc::oro_personal(&e.n, bot), 0, "ni el almacén del mercenario");
    let h = &mc::hechos(&e.n, "mercenarios.contratado")[0];
    assert_eq!((h.dato("oro"), h.dato("monedas")), (Some(0), Some(tarifa)));

    // Prorrogar también admite las monedas, junto al mercenario o a la puerta, y cobra la tarifa del plazo.
    let (del_rey, del_bot) = (monedas(&e.n, rey), monedas(&e.n, bot));
    mc::ok(&mut e.n, "rey", Comando::ProrrogarContrato { heroe: rey, mercenario: bot, plazo: 7, pagar_con: Some(PagarCon::Monedas) });
    assert_eq!((monedas(&e.n, rey), monedas(&e.n, bot), mc::oro_del_carro(&e.n, rey)), (del_rey - SIETE_DIAS / base::UNIDAD * por_oro, del_bot + SIETE_DIAS / base::UNIDAD * por_oro, carro));
}

#[test]
fn sin_monedas_bastantes_no_se_contrata_ni_se_prorroga_y_no_deja_rastro() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza) = (e.rey, e.plaza);
    let (bots, ultimo) = ([e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3")], e.bot("bot-4"));
    let por_oro = i64::from(e.n.estado().reglas.economia.contratos.monedas_por_oro);
    let (inicial, siete_dias) = (militar::Reglas::cargar().heroe.monedas_iniciales, SIETE_DIAS / base::UNIDAD * por_oro);
    // Con las monedas de salida (`HEROE.monedasIniciales`) caben tres contratos de siete días y no un cuarto; el permanente (25 de oro) no cabe ni uno, aunque el carro lleve oro de sobra.
    assert!(3 * siete_dias <= inicial && inicial < 4 * siete_dias && PERMANENTE / base::UNIDAD * por_oro > inicial && mc::oro_del_carro(&e.n, rey) > PERMANENTE);
    let con_monedas = |mercenario, trabajo, objetivo, plazo, residencia| Comando::ContratarMercenario { heroe: rey, mercenario, trabajo, objetivo, plazo, residencia, pagar_con: Some(PagarCon::Monedas) };
    assert_eq!(mc::rechazo(&mut e.n, "rey", con_monedas(ultimo, Trabajo::Permanente, Objetivo::Faccion(1), None, Some(plaza))), rechazos::HEROE_SIN_MONEDAS);
    for bot in bots {
        mc::ok(&mut e.n, "rey", con_monedas(bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7), None));
    }
    assert_eq!(monedas(&e.n, rey), inicial - 3 * siete_dias);
    // Le quedan menos monedas que siete días: ni contratar a otro ni prorrogar uno por siete días, y el rechazo no deja rastro (`mc::rechazo` lo comprueba).
    assert_eq!(mc::rechazo(&mut e.n, "rey", con_monedas(ultimo, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7), None)), rechazos::HEROE_SIN_MONEDAS);
    let prorroga = Comando::ProrrogarContrato { heroe: rey, mercenario: bots[0], plazo: 7, pagar_con: Some(PagarCon::Monedas) };
    assert_eq!(mc::rechazo(&mut e.n, "rey", prorroga), rechazos::HEROE_SIN_MONEDAS);
    assert!(mc::ofrecido(&e.n, ultimo));
}

#[test]
fn lo_que_deja_la_firma_y_la_prorroga_da_la_vuelta_por_la_foto() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, otro, rey, plaza) = (e.bot("bot-1"), e.bot("bot-2"), e.rey, e.plaza);
    assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    mc::ok(&mut e.n, "rey", Comando::ProrrogarContrato { heroe: rey, mercenario: bot, plazo: 3, pagar_con: None });
    mc::ok(&mut e.n, "rey", Comando::ContratarMercenario { heroe: rey, mercenario: otro, trabajo: Trabajo::Permanente, objetivo: Objetivo::Faccion(1), plazo: None, residencia: Some(plaza), pagar_con: None });
    let bytes = e.n.estado().serializar();
    let vuelta = nucleo::Estado::deserializar(&bytes, &e.n.estado().reglas, m.0.hash()).expect("la foto vuelve");
    assert_eq!(vuelta.contratos, e.n.estado().contratos);
    assert_eq!(vuelta.contratos.contratos.len(), 2);
    assert_eq!(vuelta.politica.facciones.marca_de(bot), Some(Marca::Temporal));
    assert_eq!(vuelta.politica.facciones.marca_de(otro), Some(Marca::Permanente));
    assert_eq!(vuelta.serializar(), bytes);
}
