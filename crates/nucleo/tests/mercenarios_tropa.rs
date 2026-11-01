//! El rango y la tropa del mercenario (entrada 48; Mercenarios_Por_Contrato §8.1 y §8.3): la leva propia que recibe el bot la primera vez que se ofrece y que repone pagando, la escuadra de
//! premio de cada rango y la licencia de reclutar. Ningún comando da todavía renombre: el rango se pone con `mc::retocado`, que pasa el estado por la foto. Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;

use asentamiento::datos::UNO;
use asentamiento::{Recurso, Tipo};
use base::{Instante, MS_POR_MINUTO};
use comun::mercenarios as mc;
use comun::real::{fundar, sitio_rico, M};
use mercenarios::{Objetivo, Trabajo};
use militar::catalogo::Tropa;
use militar::columna::Politica;
use militar::escuadra::Contenedor;
use militar::heroe::{Controlador, Residencia};
use militar::mercenarios::precio;
use militar::presencia::Ubicacion;
use nucleo::{rechazos, Comando, Nucleo, PagarCon};
use tempfile::TempDir;

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

const HORA: Instante = 60 * MS_POR_MINUTO;

/// La leva propia, en el orden de `levaPropia`.
const LEVA: [Tropa; 4] = [Tropa::MiliciaLanceros, Tropa::Granjeros, Tropa::Mineros, Tropa::Lenadores];

/// Un campamento cuyo edificio militar es el barracón (donde se reclutan los lanceros de mimbre y los espadachines).
fn barracon(n: &Nucleo, m: &M) -> u32 {
    (0..m.0.campamentos().len() as u32).find(|&c| n.edificio_militar(c) == Some(Tipo::Barracon)).expect("el mundo tiene un campamento con barracón")
}

/// Un campamento que no es el `c`.
fn otro(m: &M, c: u32) -> u32 {
    (0..m.0.campamentos().len() as u32).find(|&x| x != c).expect("el mundo tiene más de un campamento")
}

/// Las escuadras del héroe como `(tropa, soldados)`, en orden de creación.
fn escuadras(n: &Nucleo, heroe: u64) -> Vec<(Tropa, i64)> {
    n.heroe(heroe).unwrap().heroe.escuadras.iter().map(|e| (e.tropa, e.cantidad)).collect()
}

/// Los soldados de una escuadra completa de `tropa`.
fn completa(tropa: Tropa) -> i64 {
    militar::Reglas::cargar().tropas.tropa(tropa).unidades
}

/// La tecnología ha sido descubierta (aparecida para la Facción 1) y nadie la tiene adoptada: el campamento no la abre hasta pasadas 72 horas.
fn descubierta(n: &mut Nucleo, tecnologia: &str) {
    mc::ok(n, "admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec![tecnologia.into()] });
    mc::ok(n, "admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec![] });
}

/// `n` con el rango alcanzado por `bot` ganando justo el renombre que pide (y, con él, una escuadra de premio por rango): lo único que da hoy un rango.
fn con_rango(m: &M, n: &Nucleo, bots: &[(u64, u8)]) -> (Nucleo, TempDir) {
    mc::retocado(m, n, |e| {
        let d = e.reglas.economia.contratos.clone();
        for &(bot, rango) in bots {
            e.contratos.fichas.get_mut(&bot).unwrap().sumar_renombre(&d, d.rangos[usize::from(rango) - 1].renombre);
        }
    })
}

/// El bot reclutando `tropa` con el oro de su almacén personal.
fn reclutar(bot: u64, tropa: Tropa) -> Comando {
    Comando::ReclutarEnCampamento { heroe: bot, tropa, pagar_con: PagarCon::AlmacenPersonal }
}

fn reclamar(bot: u64, tropa: Tropa) -> Comando {
    Comando::ReclamarPremioDeRango { heroe: bot, tropa }
}

// ───────────────────────────── La leva propia ─────────────────────────────

#[test]
fn la_leva_propia_llega_al_primer_ofrecimiento_y_no_al_segundo() {
    let m = con_mundo!();
    let mut n = mc::nucleo_con_leva(m);
    let bot = mc::heroe_en(&mut n, "bot-1", 0, Controlador::Bot);
    assert!(escuadras(&n, bot).is_empty());

    let r = mc::ok(&mut n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });
    // Una escuadra completa de cada tropa de la leva, suya (no prestada, ni de ninguna plaza) y en su campamento; la orden las devuelve.
    assert_eq!(escuadras(&n, bot), LEVA.map(|t| (t, completa(t))));
    assert_eq!(r.creadas, n.heroe(bot).unwrap().heroe.escuadras.iter().map(|e| e.id).collect::<Vec<_>>());
    for e in &n.heroe(bot).unwrap().heroe.escuadras {
        assert_eq!((e.contenedor, e.prestada, e.plaza_origen, e.nivel), (Contenedor::Campamento, None, None, 1), "{:?}", e.tropa);
    }
    assert!(n.estado().contratos.fichas[&bot].leva_recibida);
    let hechos = mc::hechos(&n, "mercenarios.leva_recibida");
    assert_eq!((hechos.len(), hechos[0].dato("heroe"), hechos[0].dato("escuadras")), (1, Some(bot as i64), Some(4)));
    assert_eq!((hechos[0].heroes.clone(), hechos[0].facciones.clone()), (vec![bot], vec![]), "lo oye el mercenario");

    // Retirarse y volver a ofrecerse no da otra: es una sola vez.
    mc::ok(&mut n, "bot-1", Comando::RetirarOferta { heroe: bot });
    assert!(mc::ok(&mut n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot }).creadas.is_empty());
    assert_eq!(escuadras(&n, bot), LEVA.map(|t| (t, completa(t))));
    assert_eq!(mc::hechos(&n, "mercenarios.leva_recibida").len(), 1);

    // No es prestada: al mudarse de residencia se queda con ella (la tropa prestada se retira).
    mc::firmante(&mut n, "rey", 1, 0, mc::ORO, Controlador::Humano);
    let sitio = sitio_rico(&n, &m.0, 3, 1);
    let plaza = fundar(&mut n, "fundador", 1, sitio);
    mc::ok(&mut n, "admin", Comando::FijarResidencia { heroe: bot, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    assert_eq!(escuadras(&n, bot), LEVA.map(|t| (t, completa(t))));
}

#[test]
fn quien_ya_tenia_una_escuadra_de_la_leva_la_recibe_completa_y_no_otra_mas() {
    let m = con_mundo!();
    let mut n = mc::nucleo_con_leva(m);
    let bot = mc::heroe_en(&mut n, "bot-1", 0, Controlador::Bot);
    mc::ok(&mut n, "admin", Comando::DarEscuadra { heroe: bot, tropa: Tropa::Granjeros, cantidad: 10 });

    mc::ok(&mut n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });
    // Una escuadra por tropa en toda la partida: la que tenía se completa, y solo las otras tres son nuevas.
    let mut tropas = escuadras(&n, bot);
    tropas.sort();
    let mut esperadas = LEVA.map(|t| (t, completa(t)));
    esperadas.sort();
    assert_eq!(tropas, esperadas);
    assert_eq!(mc::hechos(&n, "mercenarios.leva_recibida")[0].dato("escuadras"), Some(3));
}

#[test]
fn la_leva_propia_se_repone_pagando_en_el_campamento_aunque_no_tenga_centro_urbano() {
    let m = con_mundo!();
    let mut n = mc::nucleo_con_leva(m);
    let (bot, otro_bot) = (mc::bot_ofrecido(&mut n, "bot-1", 0), mc::bot_ofrecido(&mut n, "bot-2", 0));
    assert!(!n.tropas_del_campamento(0).unwrap().contains(&Tropa::Granjeros), "el campamento no la ofrece");
    // Una escuadra de granjeros herida y a otra le faltan todos los soldados de leñadores (se perdió): la leva se recibe una sola vez.
    let (mut n, _dir) = mc::retocado(m, &n, |e| {
        let escuadras = &mut e.militar.heroes.get_mut(&bot).unwrap().heroe.escuadras;
        escuadras.iter_mut().find(|s| s.tropa == Tropa::Granjeros).unwrap().cantidad = 12;
        escuadras.retain(|s| s.tropa != Tropa::Lenadores);
    });
    mc::ok(&mut n, "admin", Comando::DarAlmacenPersonal { heroe: bot, recurso: Recurso::Oro, cantidad: 100_000 });
    let mr = militar::Reglas::cargar();
    let coste = precio(&mr, mr.tropas.tropa(Tropa::Granjeros), 18, UNO, false);
    let reclutas = n.reclutas_de_campamento(0).unwrap();

    mc::ok(&mut n, "bot-1", reclutar(bot, Tropa::Granjeros));
    assert_eq!(escuadras(&n, bot).iter().find(|e| e.0 == Tropa::Granjeros), Some(&(Tropa::Granjeros, 30)));
    assert_eq!(mc::oro_personal(&n, bot), 100_000 - coste);
    assert_eq!(n.reclutas_de_campamento(0), Some(reclutas - 18_000));
    let hecho = &mc::hechos(&n, "mercenarios.reclutado")[0];
    assert_eq!((hecho.dato("cantidad"), hecho.dato("oro"), hecho.dato("nueva")), (Some(18), Some(coste), Some(0)));
    // Completa, ya no hay qué reponer.
    assert_eq!(mc::rechazo(&mut n, "bot-1", reclutar(bot, Tropa::Granjeros)), rechazos::MERCENARIOS_AL_TOPE);
    // Solo se repone lo que se tiene: la leva no se compra otra vez ni se recibe otra vez, y quien no es mercenario no la repone.
    assert_eq!(mc::rechazo(&mut n, "bot-1", reclutar(bot, Tropa::Lenadores)), rechazos::MERCENARIOS_TROPA_NO_DISPONIBLE);
    mc::ok(&mut n, "bot-1", Comando::RetirarOferta { heroe: bot });
    mc::ok(&mut n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });
    assert!(escuadras(&n, bot).iter().all(|e| e.0 != Tropa::Lenadores));
    mc::ok(&mut n, "admin", Comando::DarAlmacenPersonal { heroe: otro_bot, recurso: Recurso::Oro, cantidad: 100_000 });
    let ajeno = mc::heroe_en(&mut n, "bot-3", 0, Controlador::Bot);
    mc::ok(&mut n, "admin", Comando::DarEscuadra { heroe: ajeno, tropa: Tropa::Granjeros, cantidad: 10 });
    assert_eq!(mc::rechazo(&mut n, "bot-3", reclutar(ajeno, Tropa::Granjeros)), rechazos::MERCENARIOS_TROPA_NO_DISPONIBLE);
}

// ───────────────────────────── La escuadra de premio ─────────────────────────────

#[test]
fn el_premio_se_reclama_con_premio_en_el_campamento_base_y_con_una_tropa_de_la_licencia() {
    let m = con_mundo!();
    let mut n = mc::nucleo(m);
    let camp = barracon(&n, m);
    let bot = mc::bot_ofrecido(&mut n, "bot-1", camp);
    // Sin rango no hay premio.
    assert_eq!(mc::rechazo(&mut n, "bot-1", reclamar(bot, Tropa::LancerosMimbre)), rechazos::MERCENARIO_SIN_PREMIO);
    descubierta(&mut n, "escudos_ligeros");
    descubierta(&mut n, "aleacion_bronce");
    let (mut n, _dir) = con_rango(m, &n, &[(bot, 2)]);
    assert_eq!(n.estado().contratos.fichas[&bot].premios, 1);

    // Fuera de su campamento base no se reclama.
    let lejos = otro(m, camp);
    mc::ok(&mut n, "admin", Comando::ColocarHeroe { heroe: bot, ubicacion: Ubicacion::Mercenarios(lejos) });
    assert_eq!(mc::rechazo(&mut n, "bot-1", reclamar(bot, Tropa::LancerosMimbre)), rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA);
    mc::ok(&mut n, "admin", Comando::ColocarHeroe { heroe: bot, ubicacion: Ubicacion::Mercenarios(camp) });

    // Ni una tropa que su licencia no abre: la de otro edificio, la de su edificio y escalón cuya tecnología nadie descubrió, la de su edificio y tecnología descubierta pero de un escalón
    // por encima del suyo (rango 2: escalón 2) y la de la leva, que no es de ningún edificio del campamento.
    let fuera = rechazos::MERCENARIO_FUERA_DE_LICENCIA;
    for tropa in [Tropa::Honderos, Tropa::EspadachinesCobre, Tropa::EspadachinesBronce, Tropa::MiliciaLanceros] {
        assert_eq!(mc::rechazo(&mut n, "bot-1", reclamar(bot, tropa)), fuera, "{tropa:?}");
    }

    // Con la columna a la puerta basta: una escuadra completa y suya, nacida en el campamento, y el premio gastado.
    mc::salir_con_oro(&mut n, "bot-1", bot, camp, 0);
    let r = mc::ok(&mut n, "bot-1", reclamar(bot, Tropa::LancerosMimbre));
    assert_eq!(escuadras(&n, bot), vec![(Tropa::LancerosMimbre, completa(Tropa::LancerosMimbre))]);
    let e = &n.heroe(bot).unwrap().heroe.escuadras[0];
    assert_eq!(r.creadas, vec![e.id], "la orden devuelve la escuadra nueva");
    assert_eq!((e.contenedor, e.prestada, e.plaza_origen), (Contenedor::Campamento, None, None));
    assert_eq!(n.estado().contratos.fichas[&bot].premios, 0);
    let hecho = &mc::hechos(&n, "mercenarios.premio_reclamado")[0];
    assert_eq!((hecho.heroes.clone(), hecho.facciones.clone()), (vec![bot], vec![]), "lo oye el mercenario");
    assert_eq!((hecho.dato("campamento"), hecho.dato("escuadra"), hecho.dato("nueva")), (Some(i64::from(camp)), Some(e.id as i64), Some(1)));
    assert_eq!(mc::rechazo(&mut n, "bot-1", reclamar(bot, Tropa::LancerosMimbre)), rechazos::MERCENARIO_SIN_PREMIO);
}

#[test]
fn si_ya_tiene_la_escuadra_el_premio_se_la_repone_entera_y_si_esta_completa_no_se_gasta() {
    let m = con_mundo!();
    let mut n = mc::nucleo(m);
    let camp = barracon(&n, m);
    let lanceros = Tropa::LancerosMimbre;
    let (herido, en_columna, entero) = (mc::bot_ofrecido(&mut n, "bot-1", camp), mc::bot_ofrecido(&mut n, "bot-2", camp), mc::bot_ofrecido(&mut n, "bot-3", camp));
    for (bot, soldados) in [(herido, 10), (en_columna, 10), (entero, completa(lanceros))] {
        mc::ok(&mut n, "admin", Comando::DarEscuadra { heroe: bot, tropa: lanceros, cantidad: soldados });
    }
    descubierta(&mut n, "escudos_ligeros");
    let (mut n, _dir) = con_rango(m, &n, &[(herido, 2), (en_columna, 2), (entero, 2)]);
    let escuadra = |n: &Nucleo, bot| n.heroe(bot).unwrap().heroe.escuadras[0].clone();
    let (antes_herido, antes_en_columna) = (escuadra(&n, herido).id, escuadra(&n, en_columna).id);

    // La que está en el campamento se repone en él.
    assert!(mc::ok(&mut n, "bot-1", reclamar(herido, lanceros)).creadas.is_empty(), "no crea ninguna");
    let e = escuadra(&n, herido);
    assert_eq!((n.heroe(herido).unwrap().heroe.escuadras.len(), e.id, e.cantidad, e.contenedor), (1, antes_herido, completa(lanceros), Contenedor::Campamento), "la misma escuadra, entera");
    assert_eq!(n.estado().contratos.fichas[&herido].premios, 0);
    assert_eq!(mc::hechos(&n, "mercenarios.premio_reclamado")[0].dato("nueva"), Some(0));

    // La que va en la columna a la puerta se repone en ella.
    mc::ok(&mut n, "bot-2", Comando::SalirDelCampamento { campamento: camp, heroe: en_columna, escuadras: vec![antes_en_columna], carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None });
    let columna = n.estado().ejercito_de_heroe(en_columna).unwrap();
    let come = |n: &Nucleo| n.ejercito(columna).unwrap().columna.viveres.por_minuto;
    let come_antes = come(&n);
    mc::ok(&mut n, "bot-2", reclamar(en_columna, lanceros));
    let e = escuadra(&n, en_columna);
    assert_eq!((e.id, e.cantidad, e.contenedor), (antes_en_columna, completa(lanceros), Contenedor::Ejercito(columna)));
    assert!(come(&n) < come_antes, "con más soldados la columna come más: {} y {come_antes}", come(&n));

    // Una completa no gana nada con el premio: se rechaza y el premio queda.
    assert_eq!(mc::rechazo(&mut n, "bot-3", reclamar(entero, lanceros)), rechazos::MERCENARIOS_AL_TOPE);
    assert_eq!(n.estado().contratos.fichas[&entero].premios, 1);
}

#[test]
fn el_permanente_reclama_su_premio_en_el_campamento_de_su_contrato_aunque_resida_en_una_plaza() {
    let m = con_mundo!();
    let mut n = mc::nucleo(m);
    let camp = barracon(&n, m);
    let rey = mc::firmante(&mut n, "rey", 1, camp, mc::ORO, Controlador::Humano);
    let sitio = sitio_rico(&n, &m.0, 3, 1);
    let plaza = fundar(&mut n, "fundador", 1, sitio);
    let bot = mc::bot_ofrecido(&mut n, "bot-1", camp);
    assert!(mc::contratar(&mut n, rey, bot, Trabajo::Permanente, Objetivo::Faccion(1), None, Some(plaza)).aceptada());
    assert_eq!(n.heroe(bot).unwrap().heroe.residencia, Some(Residencia::Asentamiento(plaza as u32)));
    descubierta(&mut n, "escudos_ligeros");
    let (mut n, _dir) = con_rango(m, &n, &[(bot, 2)]);

    mc::ok(&mut n, "bot-1", reclamar(bot, Tropa::LancerosMimbre));
    assert_eq!(escuadras(&n, bot), vec![(Tropa::LancerosMimbre, completa(Tropa::LancerosMimbre))]);
}

// ───────────────────────────── La licencia de reclutar ─────────────────────────────

#[test]
fn la_licencia_deja_reclutar_una_tropa_descubierta_hace_una_hora_hasta_su_escalon() {
    let m = con_mundo!();
    let mut n = mc::nucleo(m);
    let camp = barracon(&n, m);
    let (sin_rango, lanza, capitan) = (mc::bot_ofrecido(&mut n, "bot-1", camp), mc::bot_ofrecido(&mut n, "bot-2", camp), mc::bot_ofrecido(&mut n, "bot-3", camp));
    let (mut n, _dir) = con_rango(m, &n, &[(lanza, 2), (capitan, 3)]);
    for bot in [sin_rango, lanza, capitan] {
        mc::ok(&mut n, "admin", Comando::DarAlmacenPersonal { heroe: bot, recurso: Recurso::Oro, cantidad: 900_000 });
    }
    // Dos tecnologías de su edificio descubiertas hace una hora y, por tanto, aún cerradas a todos los campamentos: lanceros de mimbre (escalón 2) y espadachines de bronce (escalón 3).
    descubierta(&mut n, "escudos_ligeros");
    descubierta(&mut n, "aleacion_bronce");
    let descubiertas = n.ahora();
    n.avanzar_hasta(descubiertas + HORA);
    assert!(![Tropa::LancerosMimbre, Tropa::EspadachinesBronce].iter().any(|t| n.tropas_del_campamento(camp).unwrap().contains(t)), "el campamento no las ofrece aún");

    let no = rechazos::MERCENARIOS_TROPA_NO_DISPONIBLE;
    assert_eq!(mc::rechazo(&mut n, "bot-1", reclutar(sin_rango, Tropa::LancerosMimbre)), no, "sin licencia, no");
    // Rango 2: escalón 2. Los lanceros sí; los espadachines (escalón 3), no; ni los de una tecnología que nadie descubrió, ni los de otro edificio.
    mc::ok(&mut n, "bot-2", reclutar(lanza, Tropa::LancerosMimbre));
    assert_eq!(escuadras(&n, lanza), vec![(Tropa::LancerosMimbre, completa(Tropa::LancerosMimbre))]);
    for tropa in [Tropa::EspadachinesBronce, Tropa::EspadachinesCobre, Tropa::Honderos] {
        assert_eq!(mc::rechazo(&mut n, "bot-2", reclutar(lanza, tropa)), no, "{tropa:?}");
    }
    // Rango 3: escalón 3.
    mc::ok(&mut n, "bot-3", reclutar(capitan, Tropa::EspadachinesBronce));
    assert_eq!(escuadras(&n, capitan), vec![(Tropa::EspadachinesBronce, completa(Tropa::EspadachinesBronce))]);
    assert_eq!(mc::rechazo(&mut n, "bot-3", reclutar(capitan, Tropa::EspadachinesCobre)), no, "la Era no se salta: nadie descubrió su tecnología");

    // A las 72 horas el campamento abre la tecnología a todos y el que no tenía licencia ya recluta.
    n.avanzar_hasta(descubiertas + 72 * HORA);
    mc::ok(&mut n, "bot-1", reclutar(sin_rango, Tropa::LancerosMimbre));
}
