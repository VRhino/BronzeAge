//! Lo que cada uno ve de los mercenarios por contrato (entrada 48; Mercenarios_Por_Contrato §10 y la duda 3 del plan): el mercenario, por su `HEROE` y no por `FACCION` (su contrato, su orden, su
//! ficha entera y lo que necesita de su objetivo); los ciudadanos de la Facción, sus contratos y la marca de cada miembro; todos, la marca y la ficha reducida; quien está dentro del campamento,
//! el tablón con las tarifas que cobra la firma; el directorio, que no cuenta a los marcados; y la memoria de mapa de un marcado, que no se mezcla con la de la Facción. Las proyecciones se
//! prueban directamente y los ámbitos por el servidor WebSocket real. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::{Cargo, Recurso};
use base::{Instante, Mil, Punto, MS_POR_MINUTO};
use comun::mercenarios as mc;
use comun::real::{campamento, en_tierra, fundar, sitio_rico, M};
use comun::sesion::{arrancar_con, dar_todo, rechazo};
use contrato::v1::{self as pb, entidad::Cuerpo, mensaje_de_cliente as cliente, mensaje_de_servidor as ms, orden, TipoCliente, TipoEntidad};
use mercenarios::contrato::duracion;
use mercenarios::{Datos, Medalla, Objetivo, Trabajo};
use militar::catalogo::Tropa;
use militar::heroe::Controlador;
use nucleo::proyeccion::ambito::Ambito;
use nucleo::proyeccion::propios::{self, Espectador};
use nucleo::proyeccion::vista::{Mirada, Perspectiva};
use nucleo::proyeccion::{audiencia, ids, interior, vista};
use nucleo::{Clase, Comando, Nucleo, Relacion};
use servidor::prueba::{hola, ClientePrueba};
use std::collections::BTreeMap;
use vision::Rect;

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

const U: Mil = 1_000;

/// Lo que vale por nivel un héroe sin tropa, en oro, y la fracción del plazo: con el héroe de nivel 1 y rango «a sueldo», 1 día son 3 de oro (15 % de 20), 3 días 7 (35 %), 7 días 14 (70 %) y
/// el permanente 100 (500 %).
const TARIFAS_SIN_TROPA: [(Option<u32>, Mil); 4] = [(Some(1), 1_000), (Some(3), 2_000), (Some(7), 4_000), (None, 25_000)];

/// La Facción 1 (humana) con su Rey «rey», que espera con 500 de oro en el carro de su columna a la puerta del campamento 0; su plaza, fundada por «fundador», y la plaza de la Facción 2 (de «gus»).
/// Los mercenarios los pone cada prueba.
struct E {
    n: Nucleo,
    rey: u64,
    plaza: u64,
    ajena: u64,
}

impl E {
    fn nuevo(m: &M) -> Self {
        let mut n = mc::nucleo(m);
        let rey = mc::firmante(&mut n, "rey", 1, 0, mc::ORO, Controlador::Humano);
        let (sa, sb) = (sitio_rico(&n, &m.0, 3, 1), sitio_rico(&n, &m.0, 0, 2));
        let plaza = fundar(&mut n, "fundador", 1, sa);
        let ajena = fundar(&mut n, "gus", 2, sb);
        E { n, rey, plaza, ajena }
    }

    /// Un bot en el tablón del campamento 0.
    fn bot(&mut self, cuenta: &str) -> u64 {
        mc::bot_ofrecido(&mut self.n, cuenta, 0)
    }

    /// Un Gobernador de la plaza de la Facción 1, nombrado por el Rey, a la puerta del campamento 0 con 500 de oro.
    fn gobernador(&mut self, cuenta: &str) -> u64 {
        let g = mc::firmante(&mut self.n, cuenta, 1, 0, mc::ORO, Controlador::Humano);
        let (rey, plaza) = (self.rey, self.plaza);
        mc::con_cargo(&mut self.n, "rey", rey, plaza, Cargo::Gobernador, g);
        g
    }

    /// El Rey contrata a `mercenario` (exige que se acepte).
    fn contratar(&mut self, mercenario: u64, trabajo: Trabajo, objetivo: Objetivo, plazo: Option<u8>) {
        let residencia = (trabajo == Trabajo::Permanente).then_some(self.plaza);
        let r = mc::contratar(&mut self.n, self.rey, mercenario, trabajo, objetivo, plazo, residencia);
        assert!(r.aceptada(), "{r:?}");
    }

    /// El mundo detrás del servidor: las plazas con la construcción parada y el reloj recién entrado en un minuto, para que el paso de minuto no mande nada por su cuenta.
    fn quieto(mut self) -> Self {
        for plaza in [self.plaza, self.ajena] {
            mc::ok(&mut self.n, "admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
        }
        let ahora = self.n.ahora();
        self.n.avanzar_hasta((ahora / MS_POR_MINUTO + 1) * MS_POR_MINUTO + 1_000);
        self
    }
}

/// El `HEROE` del propio héroe, como lo recibe él.
fn su_heroe(n: &Nucleo, id: u64) -> pb::Heroe {
    match propios::proyectar(n, Espectador::Heroe(Some(id)), Clase::Heroe, id).existen.into_iter().next() {
        Some((_, _, Cuerpo::Heroe(h))) => h,
        otro => panic!("se esperaba el héroe: {otro:?}"),
    }
}

/// El tablón del campamento `c`, como lo ve quien está dentro.
fn tablon(n: &Nucleo, c: u32, quien: Espectador) -> Vec<pb::OfertaMercenario> {
    match propios::proyectar(n, quien, Clase::Campamento, u64::from(c)).existen.into_iter().next() {
        Some((_, _, Cuerpo::CampamentoMercenarios(x))) => x.tablon,
        otro => panic!("se esperaba el campamento: {otro:?}"),
    }
}

/// El objetivo de un contrato o de una orden que es una plaza.
fn plaza(id: u64) -> Option<pb::ObjetivoMercenario> {
    Some(pb::ObjetivoMercenario { tipo: Some(pb::objetivo_mercenario::Tipo::AsentamientoId(ids::asentamiento(id))) })
}

/// Las tarifas de una oferta como `(plazo en días, oro)`.
fn tarifas(o: &pb::OfertaMercenario) -> Vec<(Option<u32>, Mil)> {
    o.tarifas.iter().map(|t| (t.plazo_dias, t.oro)).collect()
}

// ───────────────────────────── 1. El mercenario: su HEROE ─────────────────────────────

#[test]
fn el_mercenario_ve_en_su_heroe_su_contrato_su_orden_y_su_ficha_entera() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, gob, plaza_id) = (e.bot("bot-1"), e.gobernador("gob"), e.plaza);
    let datos = Datos::cargar();

    // Ofrecido y sin contrato: la ficha entera (nació con el ofrecimiento), pero ni contrato ni marca.
    let h = su_heroe(&e.n, bot);
    assert_eq!((h.contrato, h.contratado), (None, None));
    let f = h.ficha_mercenario.expect("la ficha nace con el primer ofrecimiento");
    assert_eq!((f.rango, f.rango_id.as_str(), f.renombre, f.renombre_siguiente_rango), (1, "a_sueldo", 0, Some(datos.rangos[1].renombre)));
    assert_eq!((f.historial.map(|h| h.firmados), f.premios), (Some(0), 0));

    // El Gobernador de la plaza le contrata su defensa: el contrato es de su plaza y lleva su cargo.
    let t = e.n.ahora();
    let r = mc::contratar(&mut e.n, gob, bot, Trabajo::Defender, Objetivo::Plaza(plaza_id), Some(3), None);
    assert!(r.aceptada(), "{r:?}");
    let h = su_heroe(&e.n, bot);
    assert_eq!(h.contratado.as_deref(), Some("temporal"));

    let (as_, puesto) = (ids::asentamiento(plaza_id), format!("gobernador@{}", ids::asentamiento(plaza_id)));
    let c = h.contrato.expect("tiene contrato");
    assert_eq!((c.mercenario_id.as_str(), c.faccion_id.as_str(), c.trabajo.as_str(), c.plazo_dias), (ids::heroe(bot).as_str(), "f-1", "defender", Some(3)));
    assert_eq!((c.asentamiento_id, c.objetivo), (Some(as_.clone()), plaza(plaza_id)));
    assert_eq!((c.firmado_por_id, c.puesto_firma.as_str(), c.firmado_en_ms, c.vence_en_ms), (ids::heroe(gob), puesto.as_str(), t, Some(t + duracion(3))));
    assert_eq!((c.campamento_base_id, c.fin_aplazado, c.renombre), (ids::campamento(0), None, mc::contrato(&e.n, bot).unwrap().renombre));
    // La orden inicial la da quien firma, pero sin cargo: cualquier cargo que alcance el contrato la sustituye.
    let o = c.orden.expect("nace con su orden");
    assert_eq!((o.tipo.as_str(), o.objetivo, o.dada_por_id, o.puesto), ("guardar", plaza(plaza_id), ids::heroe(gob), None));
    assert_eq!((o.dada_en_ms, o.vence_en_ms, o.contra_faccion_id, o.unido), (t, None, None, false));

    // La ficha, con lo que falta para el siguiente rango y para el grado siguiente de cada medalla.
    let f = h.ficha_mercenario.expect("la ficha");
    assert_eq!((f.rango, f.renombre, f.renombre_siguiente_rango, f.premios), (1, 0, Some(datos.rangos[1].renombre), 0));
    assert_eq!(f.historial.map(|h| (h.firmados, h.terminados, h.despidos, h.buenas, h.malas)), Some((1, 0, 0, 0, 0)));
    let ids_de_medallas: Vec<&str> = f.medallas.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids_de_medallas, Medalla::TODAS.map(Medalla::id));
    for (medalla, de_ficha) in Medalla::TODAS.into_iter().zip(&f.medallas) {
        assert_eq!((de_ficha.grado, de_ficha.cuenta, de_ficha.siguiente_umbral), (0, 0, Some(datos.umbrales(medalla)[0])), "{medalla:?}");
    }
}

#[test]
fn un_heroe_sin_ficha_no_lleva_nada_de_mercenario() {
    let m = con_mundo!();
    let e = E::nuevo(m);
    let h = su_heroe(&e.n, e.rey);
    assert_eq!((h.contrato, h.ficha_mercenario, h.contratado), (None, None, None));
    let h = vista::heroe_publico(&e.n, e.rey);
    assert_eq!((h.contrato, h.ficha_mercenario, h.contratado), (None, None, None));
}

// ───────────────────────────── 2. El público: la marca y la ficha reducida ─────────────────────────────

#[test]
fn el_publico_ve_la_marca_el_rango_y_el_grado_de_las_medallas_pero_ni_el_historial_ni_el_contrato() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, plaza_id) = (e.bot("bot-1"), e.plaza);

    // En el tablón, sin contrato: ya tiene ficha pública y todavía no marca.
    let h = vista::heroe_publico(&e.n, bot);
    assert_eq!((h.contrato, h.contratado), (None, None));
    assert!(h.ficha_mercenario.is_some());

    e.contratar(bot, Trabajo::Defender, Objetivo::Plaza(plaza_id), Some(1));
    let h = vista::heroe_publico(&e.n, bot);
    assert_eq!((h.contrato, h.contratado.as_deref()), (None, Some("temporal")), "el contrato es del mercenario y de su Facción");
    let f = h.ficha_mercenario.expect("la ficha pública");
    assert_eq!((f.rango, f.rango_id.as_str(), f.renombre), (1, "a_sueldo", 0));
    assert_eq!((f.renombre_siguiente_rango, f.historial, f.premios), (None, None, 0), "ni lo que le falta, ni su historial, ni sus premios");
    assert_eq!(f.medallas.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), Medalla::TODAS.map(Medalla::id));
    assert!(f.medallas.iter().all(|m| (m.grado, m.cuenta, m.siguiente_umbral) == (0, 0, None)), "el grado de cada medalla, sin su cuenta ni su umbral");
}

// ───────────────────────────── 3. La Facción: sus contratos y sus miembros ─────────────────────────────

#[test]
fn los_ciudadanos_ven_los_contratos_de_la_faccion_y_la_marca_de_cada_miembro() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (temporal, permanente, rey, plaza_id) = (e.bot("bot-1"), e.bot("bot-2"), e.rey, e.plaza);
    assert!(propios::faccion(&e.n, 1).contratos.is_empty());
    e.contratar(temporal, Trabajo::Defender, Objetivo::Plaza(plaza_id), Some(3));
    e.contratar(permanente, Trabajo::Permanente, Objetivo::Faccion(1), None);

    let f = propios::faccion(&e.n, 1);
    let contratos: Vec<(String, &str, Option<u32>)> = f.contratos.iter().map(|c| (c.mercenario_id.clone(), c.trabajo.as_str(), c.plazo_dias)).collect();
    assert_eq!(contratos, [(ids::heroe(temporal), "defender", Some(3)), (ids::heroe(permanente), "permanente", None)]);
    assert_eq!(f.contratos[1].asentamiento_id, None, "el permanente es de la Facción entera, no de una plaza");
    let marca = |h: u64| f.miembros.iter().find(|x| x.heroe_id == ids::heroe(h)).unwrap_or_else(|| panic!("{h} es miembro")).marca.as_deref();
    assert_eq!((marca(temporal), marca(permanente), marca(rey)), (Some("temporal"), Some("permanente"), None));

    // De otra Facción se ve su forma pública: ni miembros ni contratos.
    let publica = propios::faccion_publica(&e.n, 1);
    assert!(publica.contratos.is_empty() && publica.miembros.is_empty());
}

#[test]
fn un_marcado_recibe_la_faccion_de_quien_le_paga_en_su_forma_publica_y_un_ciudadano_la_entera() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, plaza_id, rey) = (e.bot("bot-1"), e.plaza, e.rey);
    e.contratar(bot, Trabajo::Defender, Objetivo::Plaza(plaza_id), Some(3));

    let cuerpos = |quien| propios::proyectar(&e.n, Espectador::Heroe(Some(quien)), Clase::Faccion, 1).existen.into_iter().map(|(_, _, c)| c).collect::<Vec<_>>();
    let marcado = cuerpos(bot);
    let [Cuerpo::Faccion(f)] = marcado.as_slice() else { panic!("al marcado solo le llega la Facción: {marcado:?}") };
    assert!(f.contratos.is_empty() && f.miembros.is_empty(), "sin los contratos de los demás ni la lista de miembros");

    let ciudadano = cuerpos(rey);
    assert!(matches!(ciudadano.as_slice(), [Cuerpo::Faccion(f), Cuerpo::Tecnologias(_)] if f.contratos.len() == 1), "{ciudadano:?}");
}

#[test]
fn el_directorio_no_cuenta_a_los_marcados() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, plaza_id) = (e.bot("bot-1"), e.plaza);
    let ciudadanos = |n: &Nucleo| propios::directorio_facciones(n).facciones.into_iter().find(|f| f.id == "f-1").expect("la Facción 1").ciudadanos;
    let (antes, miembros) = (ciudadanos(&e.n), propios::faccion(&e.n, 1).miembros.len());
    assert_eq!(antes as usize, miembros);

    e.contratar(bot, Trabajo::Defender, Objetivo::Plaza(plaza_id), Some(3));
    assert_eq!(propios::faccion(&e.n, 1).miembros.len(), miembros + 1, "el marcado es miembro de la Facción");
    assert_eq!(ciudadanos(&e.n), antes, "pero no cuenta en el directorio");
}

// ───────────────────────────── 4. Lo que necesita de su objetivo ─────────────────────────────

#[test]
fn la_entidad_del_objetivo_del_contrato_y_de_la_orden_llega_al_heroe_del_mercenario() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (defensor, acompanante, escolta, permanente, atacante, libre) = (e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3"), e.bot("bot-4"), e.bot("bot-5"), e.bot("bot-6"));
    let (plaza_id, ajena) = (e.plaza, e.ajena);
    let t = e.n.ahora();
    let ejercito = e.n.crear_ejercito(t, "capi", 1, campamento(&m.0, 1), 500, 100_000).unwrap();
    // Una caravana comercial de la plaza necesita un Mercado en pie.
    for recurso in [Recurso::Madera, Recurso::Piedra, Recurso::Oro, Recurso::Trigo] {
        mc::ok(&mut e.n, "admin", Comando::DarRecursos { asentamiento: plaza_id, recurso, cantidad: 350_000 });
    }
    mc::ok(&mut e.n, "admin", Comando::AnadirEdificio { asentamiento: plaza_id, tipo: Tipo::Mercado });
    e.n.avanzar_hasta(t + 9 * 60 * MS_POR_MINUTO);
    let caravana = mc::ok(&mut e.n, "admin", Comando::CrearCaravana { asentamiento: plaza_id }).creadas[0];
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });

    e.contratar(defensor, Trabajo::Defender, Objetivo::Plaza(plaza_id), Some(1));
    e.contratar(acompanante, Trabajo::Acompanar, Objetivo::Ejercito(ejercito), Some(1));
    e.contratar(escolta, Trabajo::Escoltar, Objetivo::Caravana(caravana), Some(1));
    // El permanente es de la Facción, pero su orden es guardar la plaza donde reside.
    e.contratar(permanente, Trabajo::Permanente, Objetivo::Faccion(1), None);
    e.contratar(atacante, Trabajo::Atacar, Objetivo::Plaza(ajena), Some(1));

    let quienes = |clase, id| -> Vec<u64> {
        let de = audiencia::de(e.n.estado(), clase, id);
        [defensor, acompanante, escolta, permanente, atacante, libre].into_iter().filter(|&h| de.contains(&Ambito::Heroe(h))).collect()
    };
    assert_eq!(quienes(Clase::Asentamiento, plaza_id), [defensor, permanente], "la plaza que defiende y la que guarda por su orden");
    assert_eq!(quienes(Clase::Ejercito, ejercito), [acompanante]);
    assert_eq!(quienes(Clase::Caravana, caravana), [escolta]);
    // Lo ajeno no se le cuenta por el contrato: la plaza que ataca es de otra Facción y su ficha no es de su `HEROE` (Mercenarios_Por_Contrato §10: lo que ve de ella es lo que alcance su niebla).
    assert!(quienes(Clase::Asentamiento, ajena).is_empty());
    // Y la Facción sigue recibiendo lo suyo.
    assert!(audiencia::de(e.n.estado(), Clase::Asentamiento, plaza_id).contains(&Ambito::Faccion(1)));

    // La pertenencia que lleva el núcleo (la de la foto de `HEROE`) es la misma.
    e.n.seguir_pertenencias();
    assert!(e.n.entidades_de(Ambito::Heroe(defensor)).contains(&(Clase::Asentamiento, plaza_id)));
    assert!(e.n.entidades_de(Ambito::Heroe(escolta)).contains(&(Clase::Caravana, caravana)));
    assert!(!e.n.entidades_de(Ambito::Heroe(libre)).contains(&(Clase::Asentamiento, plaza_id)));
}

// ───────────────────────────── 5. El tablón ─────────────────────────────

#[test]
fn el_tablon_lleva_a_cada_ofrecido_del_campamento_con_su_ficha_su_tropa_su_herida_y_una_tarifa_por_plazo() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (con_tropa, sin_tropa, en_otro) = (e.bot("bot-1"), e.bot("bot-2"), mc::bot_ofrecido(&mut e.n, "bot-3", 1));
    mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: con_tropa, tropa: Tropa::MiliciaLanceros, cantidad: 10 });
    mc::ok(&mut e.n, "admin", Comando::HerirHeroe { heroe: con_tropa });
    let herido_hasta = e.n.heroe(con_tropa).unwrap().heroe.herido_hasta;
    assert!(herido_hasta.is_some());

    let dentro = tablon(&e.n, 0, Espectador::Heroe(Some(e.rey)));
    assert_eq!(dentro.iter().map(|o| o.heroe_id.clone()).collect::<Vec<_>>(), [ids::heroe(con_tropa), ids::heroe(sin_tropa)], "los de ese campamento, por héroe");
    assert_eq!(tablon(&e.n, 0, Espectador::Admin), dentro, "la administración ve el mismo");
    assert_eq!(tablon(&e.n, 1, Espectador::Admin).iter().map(|o| o.heroe_id.clone()).collect::<Vec<_>>(), [ids::heroe(en_otro)]);
    let (a, b) = (&dentro[0], &dentro[1]);

    // Su ficha entera (con el historial) y desde cuándo está ofrecido.
    assert_eq!(a.desde_ms, e.n.estado().contratos.ofertas[&con_tropa].desde);
    assert_eq!(a.ficha, su_heroe(&e.n, con_tropa).ficha_mercenario);
    assert!(a.ficha.as_ref().is_some_and(|f| f.historial.is_some()));
    // Su tropa, la suya, y si está herido.
    let tropa_id = militar::Reglas::cargar().tropas.tropa(Tropa::MiliciaLanceros).nombre.clone();
    assert_eq!(a.tropa, [pb::EscuadraQueLleva { tropa_id, cantidad: 10, nivel: 1 }]);
    assert_eq!((b.tropa.len(), b.herido_hasta_ms), (0, None));
    assert_eq!(a.herido_hasta_ms, herido_hasta);
    // Una tarifa por cada plazo y la del permanente: sin tropa, las del nivel 1; con ella, más.
    assert_eq!(tarifas(b), TARIFAS_SIN_TROPA);
    // La tarifa en monedas es la de oro, ya reducida, a `monedasPorOro` monedas por cada oro entero.
    let por_oro = i64::from(e.n.estado().reglas.economia.contratos.monedas_por_oro);
    assert!(a.tarifas.iter().chain(&b.tarifas).all(|t| t.monedas == t.oro / base::UNIDAD * por_oro && t.monedas > 0), "{:?}", b.tarifas);
    assert_eq!(a.tarifas.iter().map(|t| t.plazo_dias).collect::<Vec<_>>(), [Some(1), Some(3), Some(7), None]);
    assert!(a.tarifas.iter().zip(&b.tarifas).all(|(con, sin)| con.oro > sin.oro), "la tropa se paga: {:?}", tarifas(a));
}

#[test]
fn el_tablon_cobra_lo_que_dice_y_el_contratado_sale_de_el() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza_id) = (e.rey, e.plaza);
    let (con_tropa, permanente) = (e.bot("bot-1"), e.bot("bot-2"));
    mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: con_tropa, tropa: Tropa::MiliciaLanceros, cantidad: 10 });
    let oferta = |n: &Nucleo, h: u64, plazo: Option<u32>| tablon(n, 0, Espectador::Admin).into_iter().find(|o| o.heroe_id == ids::heroe(h)).and_then(|o| o.tarifas.into_iter().find(|t| t.plazo_dias == plazo)).map(|t| t.oro);

    let (tres_dias, para_siempre) = (oferta(&e.n, con_tropa, Some(3)).unwrap(), oferta(&e.n, permanente, None).unwrap());
    let carro = mc::oro_del_carro(&e.n, rey);
    e.contratar(con_tropa, Trabajo::Defender, Objetivo::Plaza(plaza_id), Some(3));
    assert_eq!(carro - mc::oro_del_carro(&e.n, rey), tres_dias, "la firma cobra la tarifa del tablón");
    let carro = mc::oro_del_carro(&e.n, rey);
    e.contratar(permanente, Trabajo::Permanente, Objetivo::Faccion(1), None);
    assert_eq!((carro - mc::oro_del_carro(&e.n, rey), para_siempre), (25_000, 25_000));

    assert!(tablon(&e.n, 0, Espectador::Admin).is_empty(), "el contratado ya no está ofrecido");
}

#[test]
fn desde_fuera_del_campamento_no_se_ve_el_tablon() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    e.bot("bot-1");
    assert_eq!(tablon(&e.n, 0, Espectador::Admin).len(), 1);
    // `interior::campamento` es lo de fuera: ni tablón ni lo que cobra.
    assert!(interior::campamento(&e.n, 0).expect("el campamento").tablon.is_empty());
    // Y es lo que lleva la `VISTA` de una Facción que mira todo el mapa.
    let todo = Rect { min: Punto::new(0, 0), max: Punto::new(2_667_000, 2_000_000) };
    let mirada = Mirada::de(&e.n, Perspectiva::Faccion(1));
    let vistos = mirada.contenido(Some(e.rey), todo, &BTreeMap::new());
    let campamento = vistos.into_iter().find(|x| x.clave.0 == TipoEntidad::CampamentoMercenarios as i32 && x.clave.1 == ids::campamento(0));
    let Some(Cuerpo::CampamentoMercenarios(c)) = campamento.and_then(|x| x.cuerpo()) else { panic!("la vista lleva el campamento") };
    assert!(c.tablon.is_empty());
}

// ───────────────────────────── 6. La memoria de mapa del marcado ─────────────────────────────

/// La columna del héroe marcha a `destino` y llega.
fn andar(n: &mut Nucleo, cuenta: &str, heroe: u64, destino: Punto) {
    let ejercito = n.estado().ejercito_de_heroe(heroe).expect("va en una columna");
    mc::ok(n, cuenta, Comando::MarcharA { ejercito, destino });
    let fin: Instante = n.ejercito(ejercito).expect("la columna existe").plan.fin().expect("marcha");
    n.avanzar_hasta(fin + 1_000);
}

#[test]
fn la_memoria_personal_del_marcado_no_pasa_a_la_faccion_y_sigue_creciendo_con_sus_ojos() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (puerta, plaza_id) = (campamento(&m.0, 0), e.plaza);
    let (este, lejos) = (en_tierra(&m.0, puerta, 40 * U, 0), en_tierra(&m.0, puerta, 90 * U, 0));
    let sitio_de_la_plaza = e.n.estado().asentamientos[&plaza_id].posicion;
    let sabe = |n: &Nucleo, f: u32, p: Punto| n.estado().memoria.exploracion.get(&f).is_some_and(|x| x.explorado_en(p));
    let suya = |n: &Nucleo, h: u64, p: Punto| n.estado().memoria.personal.get(&h).is_some_and(|x| x.explorado_en(p));

    // Sin bandera, lo que ve un bot es solo suyo: sale, anda 40 u al este, vuelve y se ofrece.
    let bot = mc::heroe_en(&mut e.n, "bot-1", 0, Controlador::Bot);
    mc::salir_con_oro(&mut e.n, "bot-1", bot, 0, 0);
    andar(&mut e.n, "bot-1", bot, este);
    andar(&mut e.n, "bot-1", bot, puerta);
    mc::ok(&mut e.n, "bot-1", Comando::EntrarEnCampamento { campamento: 0, heroe: bot });
    mc::ok(&mut e.n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });
    assert!(suya(&e.n, bot, este) && !sabe(&e.n, 1, este) && !sabe(&e.n, 1, lejos));

    // Al firmar, su memoria personal no pasa a la de la Facción (a un ciudadano sí, al entrar en ella) y sigue siendo suya.
    e.contratar(bot, Trabajo::Defender, Objetivo::Plaza(plaza_id), Some(7));
    assert!(suya(&e.n, bot, este), "conserva lo suyo");
    assert!(!sabe(&e.n, 1, este), "y no se lo da a la Facción");

    // Marcado, sus ojos alimentan las dos: la columna con la que sale lleva la bandera de quien le paga y lo que ve es de ella y también suyo.
    mc::salir_con_oro(&mut e.n, "bot-1", bot, 0, 0);
    let columna = e.n.estado().ejercito_de_heroe(bot).expect("sale en su columna");
    assert_eq!(e.n.ejercito(columna).unwrap().columna.faccion, Some(1));
    assert!(!suya(&e.n, bot, lejos));
    andar(&mut e.n, "bot-1", bot, lejos);
    assert!(suya(&e.n, bot, lejos), "sigue creciendo su memoria personal");
    assert!(sabe(&e.n, 1, lejos), "y la Facción recibe lo que ve");
    assert!(suya(&e.n, bot, este));

    // Al irse de la Facción conserva su memoria personal y no se lleva la de ella.
    assert!(sabe(&e.n, 1, sitio_de_la_plaza) && !suya(&e.n, bot, sitio_de_la_plaza));
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: bot, faccion: None });
    assert!(suya(&e.n, bot, este) && suya(&e.n, bot, lejos), "conserva su memoria personal");
    assert!(!suya(&e.n, bot, sitio_de_la_plaza), "y no se lleva la de la Facción");
    assert!(sabe(&e.n, 1, sitio_de_la_plaza) && sabe(&e.n, 1, lejos), "que sigue como estaba");
}

// ───────────────────────────── 7. Los ámbitos de la sesión ─────────────────────────────

fn ambito(a: &Option<pb::Ambito>) -> String {
    let a = a.as_ref().expect("un ámbito");
    format!("{}:{}", pb::AmbitoTipo::try_from(a.tipo).unwrap().as_str_name().trim_start_matches("AMBITO_"), a.id)
}

/// Un mensaje en una línea: `foto HEROE:heroe-3`, `delta CAMPAMENTO:camp-0`...
fn etiqueta(m: &ms::Cuerpo) -> String {
    match m {
        ms::Cuerpo::Foto(f) => format!("foto {}", ambito(&f.ambito)),
        ms::Cuerpo::Delta(d) => format!("delta {}", ambito(&d.ambito)),
        ms::Cuerpo::Revocado(r) => format!("revocado {}", ambito(&r.ambito)),
        otro => format!("{otro:?}"),
    }
}

fn etiquetas(mensajes: &[ms::Cuerpo]) -> Vec<String> {
    mensajes.iter().map(etiqueta).collect()
}

/// Lo que ya estaba en camino hacia la sesión: el actor contesta a una orden que rechaza después de repartir todo lo anterior, y el servidor entrega en orden.
async fn pendiente(c: &mut ClientePrueba) -> Vec<ms::Cuerpo> {
    let (previos, r) = dar_todo(c, "barrera", orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: "ej-999999".into() })).await;
    assert_eq!(rechazo(&r), "ejercito.desconocido");
    previos
}

fn foto_de(m: &ms::Cuerpo) -> &pb::Foto {
    let ms::Cuerpo::Foto(f) = m else { panic!("se esperaba una Foto: {m:?}") };
    f
}

/// Lo que actualizan las `Delta` del ámbito `a` entre los mensajes.
fn actualizado<'a>(mensajes: &'a [ms::Cuerpo], a: &str) -> Vec<&'a Cuerpo> {
    let deltas = mensajes.iter().filter_map(|m| if let ms::Cuerpo::Delta(d) = m { Some(d) } else { None });
    deltas.filter(|d| ambito(&d.ambito) == a).flat_map(|d| &d.cambios).filter_map(|c| c.datos.as_ref()?.cuerpo.as_ref()).collect()
}

/// Los códigos de los hechos de las `Delta` del ámbito `a` entre los mensajes.
fn hechos<'a>(mensajes: &'a [ms::Cuerpo], a: &str) -> Vec<&'a str> {
    let deltas = mensajes.iter().filter_map(|m| if let ms::Cuerpo::Delta(d) = m { Some(d) } else { None });
    deltas.filter(|d| ambito(&d.ambito) == a).flat_map(|d| &d.eventos).map(|e| e.codigo.as_str()).collect()
}

fn cuerpos(f: &pb::Foto) -> Vec<&Cuerpo> {
    f.entidades.iter().filter_map(|e| e.cuerpo.as_ref()).collect()
}

fn el_heroe<'a>(cuerpos: &[&'a Cuerpo], id: &str) -> &'a pb::Heroe {
    cuerpos.iter().find_map(|c| if let Cuerpo::Heroe(h) = c { Some(h).filter(|h| h.id == id) } else { None }).unwrap_or_else(|| panic!("el héroe {id}"))
}

fn la_faccion<'a>(cuerpos: &[&'a Cuerpo]) -> &'a pb::Faccion {
    cuerpos.iter().find_map(|c| if let Cuerpo::Faccion(f) = c { Some(f) } else { None }).expect("la Facción")
}

fn el_tablon<'a>(cuerpos: &[&'a Cuerpo]) -> &'a [pb::OfertaMercenario] {
    cuerpos.iter().find_map(|c| if let Cuerpo::CampamentoMercenarios(x) = c { Some(x.tablon.as_slice()) } else { None }).expect("el campamento")
}

#[tokio::test]
async fn el_marcado_abre_su_heroe_y_el_campamento_pero_no_su_faccion_y_su_vista_es_la_personal() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, libre, plaza_id, rey) = (e.bot("bot-1"), e.bot("bot-2"), e.plaza, e.rey);
    e.contratar(bot, Trabajo::Defender, Objetivo::Plaza(plaza_id), Some(3));
    let (h_bot, h_libre, h_rey, as_, camp) = (ids::heroe(bot), ids::heroe(libre), ids::heroe(rey), ids::asentamiento(plaza_id), ids::campamento(0));
    let w = arrancar_con(e.quieto().n).await;

    // El marcado: su héroe y el campamento donde está; la Facción de quien le paga no es suya.
    let (mut marcado, _, mensajes) = w.conectar(hola("ok:bot-1", TipoCliente::ClienteBot)).await;
    assert_eq!(etiquetas(&mensajes), [format!("foto HEROE:{h_bot}"), format!("foto CAMPAMENTO:{camp}")]);
    let suyo = cuerpos(foto_de(&mensajes[0]));
    let h = el_heroe(&suyo, &h_bot);
    let c = h.contrato.as_ref().expect("su contrato");
    assert_eq!((h.contratado.as_deref(), c.orden.as_ref().map(|o| o.tipo.as_str()), c.objetivo.clone()), (Some("temporal"), Some("guardar"), plaza(plaza_id)));
    assert!(h.ficha_mercenario.as_ref().is_some_and(|f| f.historial.is_some()), "su ficha entera");
    assert!(suyo.iter().any(|c| matches!(c, Cuerpo::Asentamiento(a) if a.id == as_)), "y la plaza que defiende, por su HEROE");
    let f = la_faccion(&suyo);
    assert_eq!((f.id.as_str(), f.contratos.len(), f.miembros.len()), ("f-1", 0, 0), "la Facción, en su forma pública");
    assert!(!suyo.iter().any(|c| matches!(c, Cuerpo::Tecnologias(_))));
    // El tablón del campamento ya no lo lleva a él.
    let tablon: Vec<&str> = el_tablon(&cuerpos(foto_de(&mensajes[1]))).iter().map(|o| o.heroe_id.as_str()).collect();
    assert_eq!(tablon, [h_libre.as_str()]);

    // Su vista es la personal.
    let p = campamento(&m.0, 0);
    marcado.enviar(cliente::Cuerpo::Vista(pb::CambiarVista { rectangulo: Some(pb::Rectangulo { min: Some(pb::Punto { x: p.x - 10 * U, y: p.y - 10 * U }), max: Some(pb::Punto { x: p.x + 10 * U, y: p.y + 10 * U }) }) })).await;
    let ms::Cuerpo::Foto(f) = marcado.esperar().await.cuerpo.unwrap() else { panic!("se esperaba la Foto de la vista") };
    assert_eq!(ambito(&f.ambito), format!("VISTA:{h_bot}"));

    // El Rey sí tiene su Facción, con el contrato y la marca del miembro, y todo lo suyo; y, con su columna parada a la puerta del campamento 0, también el campamento (2026-11-10).
    let (_, _, mensajes) = w.conectar(hola("ok:rey", TipoCliente::ClienteUnity)).await;
    assert_eq!(etiquetas(&mensajes), [format!("foto HEROE:{h_rey}"), "foto FACCION:f-1".to_owned(), format!("foto CAMPAMENTO:{camp}")]);
    let cuerpos_de_faccion = cuerpos(foto_de(&mensajes[1]));
    let f = la_faccion(&cuerpos_de_faccion);
    assert_eq!(f.contratos.iter().map(|c| c.mercenario_id.as_str()).collect::<Vec<_>>(), [h_bot.as_str()]);
    assert_eq!(f.miembros.iter().find(|x| x.heroe_id == h_bot).and_then(|x| x.marca.as_deref()), Some("temporal"));
    assert!(cuerpos_de_faccion.iter().any(|c| matches!(c, Cuerpo::Tecnologias(_))));
    w.parar().await;
}

#[tokio::test]
async fn al_firmar_el_mercenario_recibe_su_contrato_por_su_heroe_y_la_faccion_el_suyo_y_el_tablon_pierde_la_oferta() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, otro, plaza_id) = (e.bot("bot-1"), e.bot("bot-2"), e.plaza);
    // Alguien dentro del campamento que mira el tablón: un ciudadano.
    let dentro = mc::heroe_en(&mut e.n, "beto", 0, Controlador::Humano);
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: dentro, faccion: Some(1) });
    let (h_bot, h_otro, h_rey) = (ids::heroe(bot), ids::heroe(otro), ids::heroe(e.rey));
    let w = arrancar_con(e.quieto().n).await;
    let (mut rey, _, _) = w.conectar(hola("ok:rey", TipoCliente::ClienteUnity)).await;
    let (mut marcado, _, _) = w.conectar(hola("ok:bot-1", TipoCliente::ClienteBot)).await;
    let (mut beto, _, mensajes) = w.conectar(hola("ok:beto", TipoCliente::ClienteUnity)).await;
    assert_eq!(el_tablon(&cuerpos(foto_de(&mensajes[2]))).iter().map(|o| o.heroe_id.as_str()).collect::<Vec<_>>(), [h_bot.as_str(), h_otro.as_str()]);

    let firma = orden::Cuerpo::ContratarMercenario(pb::OrdenContratarMercenario {
        mercenario_id: h_bot.clone(),
        trabajo: "defender".into(),
        objetivo: plaza(plaza_id),
        plazo_dias: Some(3),
        residencia_asentamiento_id: None,
        pagar_con: None,
    });
    let (previos, r) = dar_todo(&mut rey, "firma", firma).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");

    // La Facción recibe el contrato y la marca, y el hecho de la firma.
    let faccion = actualizado(&previos, "FACCION:f-1");
    let f = la_faccion(&faccion);
    assert_eq!(f.contratos.iter().map(|c| c.mercenario_id.as_str()).collect::<Vec<_>>(), [h_bot.as_str()]);
    assert_eq!(f.miembros.iter().find(|x| x.heroe_id == h_bot).and_then(|x| x.marca.as_deref()), Some("temporal"));
    assert_eq!(hechos(&previos, "FACCION:f-1"), ["mercenarios.contratado"]);
    assert!(etiquetas(&previos).contains(&format!("delta HEROE:{h_rey}")), "el Rey, además, pagó con el carro");

    // El mercenario, el suyo y el del campamento (sin él en el tablón), y nada de la Facción: el hecho le llega una sola vez, por su HEROE.
    let recibido = pendiente(&mut marcado).await;
    assert!(recibido.iter().all(|m| !etiqueta(m).contains("FACCION")), "{:?}", etiquetas(&recibido));
    let suyo = actualizado(&recibido, &format!("HEROE:{h_bot}"));
    let h = el_heroe(&suyo, &h_bot);
    assert_eq!((h.contratado.as_deref(), h.contrato.as_ref().map(|c| c.trabajo.as_str())), (Some("temporal"), Some("defender")));
    assert_eq!(hechos(&recibido, &format!("HEROE:{h_bot}")), ["mercenarios.contratado"]);
    assert_eq!(el_tablon(&actualizado(&recibido, &format!("CAMPAMENTO:{}", ids::campamento(0)))).iter().map(|o| o.heroe_id.as_str()).collect::<Vec<_>>(), [h_otro.as_str()]);

    // Quien mira el tablón desde dentro lo ve sin él.
    let visto = pendiente(&mut beto).await;
    assert_eq!(el_tablon(&actualizado(&visto, &format!("CAMPAMENTO:{}", ids::campamento(0)))).iter().map(|o| o.heroe_id.as_str()).collect::<Vec<_>>(), [h_otro.as_str()]);
    assert!(hechos(&visto, &format!("HEROE:{}", ids::heroe(dentro))).is_empty());
    w.parar().await;
}

#[tokio::test]
async fn la_plaza_que_defiende_le_llega_en_cuanto_firma_y_sale_de_su_heroe_al_acabar_el_contrato() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, plaza_id) = (e.bot("bot-1"), e.plaza);
    let (h_bot, as_, suyo) = (ids::heroe(bot), ids::asentamiento(plaza_id), format!("HEROE:{}", ids::heroe(bot)));
    let w = arrancar_con(e.quieto().n).await;
    let (mut rey, _, _) = w.conectar(hola("ok:rey", TipoCliente::ClienteUnity)).await;
    let (mut marcado, _, mensajes) = w.conectar(hola("ok:bot-1", TipoCliente::ClienteBot)).await;
    let es_la_plaza = |c: &&Cuerpo| matches!(c, Cuerpo::Asentamiento(a) if a.id == as_);
    assert!(!cuerpos(foto_de(&mensajes[0])).iter().any(es_la_plaza), "sin contrato no la ve");

    // Firmar no cambia la plaza, pero ahora es de su contrato y de su orden: le llega sin esperar a que cambie.
    let firma = orden::Cuerpo::ContratarMercenario(pb::OrdenContratarMercenario {
        mercenario_id: h_bot.clone(),
        trabajo: "defender".into(),
        objetivo: plaza(plaza_id),
        plazo_dias: Some(3),
        residencia_asentamiento_id: None,
        pagar_con: None,
    });
    assert_eq!(dar_todo(&mut rey, "firma", firma).await.1.resultado, pb::Resultado::OrdenAceptada as i32);
    assert!(actualizado(&pendiente(&mut marcado).await, &suyo).iter().any(es_la_plaza), "la plaza que defiende, en cuanto firma");

    // Despedido, sale de su HEROE.
    let despido = orden::Cuerpo::DespedirMercenario(pb::OrdenDespedirMercenario { mercenario_id: h_bot.clone() });
    assert_eq!(dar_todo(&mut rey, "despido", despido).await.1.resultado, pb::Resultado::OrdenAceptada as i32);
    let deltas: Vec<pb::Delta> = pendiente(&mut marcado).await.into_iter().filter_map(|m| if let ms::Cuerpo::Delta(d) = m { Some(d) } else { None }).collect();
    let quitadas: Vec<String> = deltas
        .into_iter()
        .filter(|d| ambito(&d.ambito) == suyo)
        .flat_map(|d| d.cambios)
        .filter(|c| c.tipo == pb::TipoCambio::CambioQuita as i32 && c.entidad == TipoEntidad::Asentamiento as i32)
        .map(|c| c.id)
        .collect();
    assert_eq!(quitadas, [as_]);
    w.parar().await;
}

#[tokio::test]
async fn quien_se_ofrece_se_pone_en_el_tablon_de_quien_mira_desde_dentro() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let dentro = mc::heroe_en(&mut e.n, "beto", 0, Controlador::Humano);
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: dentro, faccion: Some(1) });
    let bot = mc::heroe_en(&mut e.n, "bot-1", 0, Controlador::Bot);
    let (h_bot, camp) = (ids::heroe(bot), format!("CAMPAMENTO:{}", ids::campamento(0)));
    let w = arrancar_con(e.quieto().n).await;
    let (mut beto, _, mensajes) = w.conectar(hola("ok:beto", TipoCliente::ClienteUnity)).await;
    assert!(el_tablon(&cuerpos(foto_de(&mensajes[2]))).is_empty());
    let (mut bot_c, _, _) = w.conectar(hola("ok:bot-1", TipoCliente::ClienteBot)).await;

    let (_, r) = dar_todo(&mut bot_c, "oferta", orden::Cuerpo::OfrecerseComoMercenario(pb::OrdenOfrecerseComoMercenario {})).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    let visto = pendiente(&mut beto).await;
    let oferta = actualizado(&visto, &camp);
    let tablon = el_tablon(&oferta);
    assert_eq!(tablon.iter().map(|o| o.heroe_id.as_str()).collect::<Vec<_>>(), [h_bot.as_str()]);
    assert_eq!(tarifas(&tablon[0]), TARIFAS_SIN_TROPA);

    // Y se retira.
    let (_, r) = dar_todo(&mut bot_c, "retira", orden::Cuerpo::RetirarOferta(pb::OrdenRetirarOferta {})).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    let visto = pendiente(&mut beto).await;
    assert!(el_tablon(&actualizado(&visto, &camp)).is_empty());
    w.parar().await;
}

#[test]
fn el_loadout_de_una_partida_vieja_no_sirve_la_prestada_que_ya_se_retiro_y_se_vuelve_a_guardar() {
    let m = con_mundo!();
    let mut n = mc::nucleo(m);
    let h = mc::heroe_en(&mut n, "ana", 0, Controlador::Humano);
    let propia = mc::ok(&mut n, "admin", Comando::DarEscuadra { heroe: h, tropa: Tropa::MiliciaLanceros, cantidad: 10 }).creadas[0];
    // Antes del 2026-11-03 la prestada retirada al mudarse se quedaba en el loadout: aquí, la 99 999, que ya no es suya.
    let (mut n, _dir) = mc::retocado(m, &n, |e| e.militar.heroes.get_mut(&h).expect("existe").heroe.loadouts[0].escuadras = vec![99_999, propia]);
    let (id, nombre) = {
        let hn = n.heroe(h).expect("existe");
        let l = &propios::heroe(&n, hn).loadouts[0];
        assert_eq!(l.squad_ids, vec![ids::escuadra(propia)], "la VISTA solo sirve las escuadras que aún son suyas");
        (hn.heroe.loadouts[0].id, l.display_name.clone())
    };
    // El cliente reenvía lo que ve, y se guarda.
    mc::ok(&mut n, "ana", Comando::GuardarLoadout { heroe: h, loadout: Some(id), nombre, escuadras: vec![propia], perks: vec![], activo: None });
    assert_eq!(n.heroe(h).unwrap().heroe.loadouts[0].escuadras, vec![propia]);
}
