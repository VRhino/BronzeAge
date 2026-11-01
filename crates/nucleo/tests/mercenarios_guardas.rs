//! Lo que un ofrecido y un mercenario con marca no hacen (BAN54, entrada 48; Mercenarios_Por_Contrato §2, §3, §6.1 y §11): el ofrecido no crea Facción, no pide ingreso ni funda; el
//! contratado no lo hace nunca, ni declara la guerra, ni convoca ejércitos, ni acepta las misiones del tablón, y solo ataca, persigue, se une a un ejército, a una convocatoria o a una
//! batalla, escolta o guarnece una avanzada con la orden que se lo permite. Tampoco recibe el mando de un ejército ni gana Participación, y su Facción no le quita la marca por su cuenta.
//! Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::{Cargo, Recurso};
use base::{Punto, MS_POR_MINUTO};
use combate::resolucion::Lado;
use comercio::caravana::{Animal, TipoCarro};
use comun::mercenarios as mc;
use comun::real::{campamento, en_tierra, fundar, pisable, sitio_rico, M};
use mercenarios::{Objetivo, Trabajo};
use militar::catalogo::Tropa;
use militar::columna::Politica;
use militar::escuadra::Contenedor;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::misiones::Dueno;
use nucleo::recurrentes::{Catalogo, Progreso, Situacion};
use nucleo::{rechazos, Comando, Nucleo, ObjetivoAtaque, Relacion, Resultado};
use std::collections::BTreeMap;
use std::sync::Arc;

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

/// La Facción 1 (humana) con su Rey «rey», que espera con 500 de oro en el carro de su columna a la puerta del campamento 0; su plaza, junto a ese campamento y fundada por «fundador»
/// (Gobernador de oficio, fuera, en su columna), y la plaza de la Facción 2 (de «gus»), junto al mismo campamento. Los mercenarios los pone cada prueba, siempre en el campamento 0.
struct E {
    n: Nucleo,
    rey: u64,
    plaza: u64,
    ajena: u64,
}

impl E {
    fn nuevo(m: &M) -> Self {
        Self::con(m, mc::nucleo(m))
    }

    fn con(m: &M, mut n: Nucleo) -> Self {
        // El primer ciudadano de la Facción 1 es su Rey.
        let rey = mc::firmante(&mut n, "rey", 1, 0, mc::ORO, Controlador::Humano);
        let sitio = sitio_rico(&n, &m.0, 0, 1);
        let plaza = fundar(&mut n, "fundador", 1, sitio);
        let sitio = sitio_rico(&n, &m.0, 0, 2);
        let ajena = fundar(&mut n, "gus", 2, sitio);
        E { n, rey, plaza, ajena }
    }

    /// Un mercenario que el Rey contrata siete días para ese trabajo: sigue en su campamento, dentro, con la orden inicial del trabajo.
    fn contratado(&mut self, cuenta: &str, trabajo: Trabajo, objetivo: Objetivo) -> u64 {
        let bot = mc::bot_ofrecido(&mut self.n, cuenta, 0);
        let r = mc::contratar(&mut self.n, self.rey, bot, trabajo, objetivo, Some(7), None);
        assert!(r.aceptada(), "{trabajo:?} {objetivo:?}: {r:?}");
        bot
    }

    /// Un ciudadano humano de la Facción 1 en el campamento 0, dentro y sin cargo.
    fn ciudadano(&mut self, cuenta: &str) -> u64 {
        let heroe = mc::heroe_en(&mut self.n, cuenta, 0, Controlador::Humano);
        mc::ok(&mut self.n, "admin", Comando::FijarFaccionDeHeroe { heroe, faccion: Some(1) });
        heroe
    }

    /// Un ejército de la Facción 1 que acepta a quien se le una, al mando de `cuenta`, a la puerta del campamento 0: devuelve (héroe, columna).
    fn ejercito(&mut self, cuenta: &str) -> (u64, u64) {
        let heroe = self.ciudadano(cuenta);
        let columna = mc::ok(&mut self.n, cuenta, Comando::SalirDelCampamento { campamento: 0, heroe, escuadras: vec![], carga: vec![], politica: Politica::Aceptar, caravana_fundacion: None }).creadas[0];
        (heroe, columna)
    }

    /// El mercenario `bot`, de la cuenta `cuenta`, sale de su campamento como ejército y marcha a `destino`: devuelve su columna, ya allí.
    fn sale_y_llega(&mut self, cuenta: &str, bot: u64, destino: Punto) -> u64 {
        let columna = mc::ok(&mut self.n, cuenta, Comando::SalirDelCampamento { campamento: 0, heroe: bot, escuadras: vec![], carga: vec![], politica: Politica::Aceptar, caravana_fundacion: None }).creadas[0];
        mc::ok(&mut self.n, cuenta, Comando::MarcharA { ejercito: columna, destino });
        let fin = self.n.ejercito(columna).unwrap().plan.fin().expect("marcha");
        self.n.avanzar_hasta(fin);
        columna
    }

    /// Deja pasar el día de protección con que nacen las plazas.
    fn sin_proteccion(&mut self) {
        for plaza in [self.plaza, self.ajena] {
            if let Some(h) = self.n.protegida_hasta(plaza) {
                self.n.avanzar_hasta(h.max(self.n.ahora()));
            }
        }
    }

    /// Un punto de tierra a la puerta de la plaza `de`, a menos de `radioEncuentro` de ella.
    fn puerta_de(&self, m: &M, de: u64) -> Punto {
        pisable(&m.0, self.n.estado().asentamientos[&de].posicion, 500, 0)
    }

    /// Un ejército de la Facción `faccion` y la cuenta dada, con una escuadra, en `donde`: devuelve (columna, héroe).
    fn sitiador(&mut self, cuenta: &str, faccion: u32, donde: Punto) -> (u64, u64) {
        let columna = self.n.crear_ejercito(self.n.ahora(), cuenta, faccion, donde, 500, 100_000).unwrap();
        let heroe = self.n.ejercito(columna).unwrap().columna.lider;
        mc::ok(&mut self.n, "admin", Comando::DarEscuadra { heroe, tropa: Tropa::MiliciaLanceros, cantidad: 20 });
        (columna, heroe)
    }

    /// Las caravanas de la plaza de la Facción 1, con un carro con buey cada una y fuera del reparto automático: levanta antes el Mercado y espera a que se construya.
    fn caravanas(&mut self, cuantas: usize) -> Vec<u64> {
        let plaza = self.plaza;
        mc::ok(&mut self.n, "admin", Comando::DarRecursos { asentamiento: plaza, recurso: Recurso::Madera, cantidad: 350_000 });
        mc::ok(&mut self.n, "admin", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
        self.n.avanzar_hasta(self.n.ahora() + 9 * 60 * MS_POR_MINUTO);
        mc::ok(&mut self.n, "admin", Comando::DarRecursos { asentamiento: plaza, recurso: Recurso::Cobre, cantidad: 100_000 * cuantas as i64 });
        mc::ok(&mut self.n, "admin", Comando::DarRecursos { asentamiento: plaza, recurso: Recurso::Madera, cantidad: 100_000 * cuantas as i64 });
        (0..cuantas)
            .map(|_| {
                // El cooldown de creación es de diez minutos por plaza.
                self.n.avanzar_hasta(self.n.ahora() + 11 * MS_POR_MINUTO);
                let k = mc::ok(&mut self.n, "admin", Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
                mc::ok(&mut self.n, "admin", Comando::ReservarCaravana { caravana: k, reservada: true });
                mc::ok(&mut self.n, "admin", Comando::AnadirCarro { caravana: k, tipo: TipoCarro::Basico });
                mc::ok(&mut self.n, "admin", Comando::AnadirAnimal { caravana: k, carro: 0, animal: Animal::Buey });
                k
            })
            .collect()
    }

    /// Una atalaya de la plaza de la Facción 1 levantada 8 u al este de ella, con la zona de la plaza agrandada hasta el techo del nivel 2: la coloca su Maestro de Obras (andamiaje de la
    /// administración) y la obra acaba con los materiales que ésta le da. Devuelve (avanzada, sitio).
    fn atalaya(&mut self) -> (u64, Punto) {
        let plaza = self.plaza;
        mc::ok(&mut self.n, "admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 2 });
        for _ in 0..14 {
            mc::dar(&mut self.n, "admin", Comando::AgrandarZona { asentamiento: plaza });
        }
        mc::ok(&mut self.n, "admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
        let centro = self.n.estado().asentamientos[&plaza].posicion;
        let sitio = Punto::new(centro.x + 8_000, centro.y);
        let columna = self.n.crear_ejercito(self.n.ahora(), "obrero", 1, sitio, 500, 100_000).unwrap();
        let maestro = self.n.ejercito(columna).unwrap().columna.lider;
        mc::ok(&mut self.n, "admin", Comando::FijarResidencia { heroe: maestro, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
        mc::ok(&mut self.n, "admin", Comando::AsignarCargoLocal { heroe: maestro, asentamiento: plaza, cargo: Cargo::MaestroObras, designado: maestro });
        let av = mc::ok(&mut self.n, "admin", Comando::ColocarAvanzada { asentamiento: plaza, heroe: maestro, posicion: sitio }).creadas[0];
        // La obra pide varios viajes (el carro admite 500): la administración llena el carro y la columna entrega, hasta que no falta nada.
        while let Some(falta) = self.n.avanzada(av).and_then(|(_, a)| a.obra.as_ref().map(|o| o.falta())).filter(|f| !f.is_empty()) {
            for (recurso, cantidad) in falta {
                mc::dar(&mut self.n, "admin", Comando::DarSuministro { ejercito: columna, recurso, cantidad });
            }
            mc::ok(&mut self.n, "admin", Comando::EntregarMaterialesAAvanzada { ejercito: columna, avanzada: av });
        }
        let fin = self.n.avanzada(av).and_then(|(_, a)| a.obra.as_ref().and_then(|o| o.completa_en)).expect("la obra arrancó");
        self.hasta(fin);
        (av, sitio)
    }

    /// Avanza hasta `fin` reponiendo a la plaza sus materiales a cada hora: sin auto-construcción ni comercio caería en ruinas a mitad de una espera larga.
    fn hasta(&mut self, fin: i64) {
        let plaza = self.plaza;
        while self.n.ahora() < fin {
            self.n.avanzar_hasta(fin.min(self.n.ahora() + 60 * MS_POR_MINUTO));
            for recurso in [Recurso::Madera, Recurso::Piedra, Recurso::Oro, Recurso::Trigo] {
                mc::dar(&mut self.n, "admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad: 400_000 });
            }
        }
    }

    /// La cuenta pide a `t` desconectar a su héroe: saldrá del mundo a los 2 min 30 s.
    fn se_desconecta(&mut self, cuenta: &str, heroe: u64, t: i64) {
        self.n.avanzar_hasta(t);
        mc::ok(&mut self.n, cuenta, Comando::Desconectarse { heroe });
    }

    /// El mercenario `bot` entra en la plaza.
    fn mete_en(&mut self, bot: u64, plaza: u64) {
        mc::ok(&mut self.n, "admin", Comando::ColocarHeroe { heroe: bot, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
    }
}

#[test]
fn un_ofrecido_no_crea_faccion_ni_pide_ingreso_ni_funda_y_al_retirar_la_oferta_vuelve_a_ser_libre() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let bot = mc::bot_ofrecido(&mut e.n, "bot-1", 0);
    let crear = Comando::CrearFaccion { heroe: bot, nombre: "Libres".into(), sigilo: None };
    let pedir = Comando::SolicitarIngreso { heroe: bot, faccion: 1 };
    // Fundar lo hace desde su columna, con una Caravana de Fundación: sale del campamento (la oferta sigue puesta) y la recibe.
    let columna = mc::ok(&mut e.n, "bot-1", Comando::SalirDelCampamento { campamento: 0, heroe: bot, escuadras: vec![], carga: vec![], politica: Politica::Aceptar, caravana_fundacion: None }).creadas[0];
    mc::ok(&mut e.n, "admin", Comando::DarCaravanaFundacion { ejercito: columna });
    let fundar = Comando::Fundar { ejercito: columna };

    let ofrecido = rechazos::FACCION_MERCENARIO_OFRECIDO;
    assert_eq!(mc::rechazo(&mut e.n, "bot-1", crear.clone()), ofrecido);
    assert_eq!(mc::rechazo(&mut e.n, "bot-1", pedir.clone()), ofrecido);
    assert_eq!(mc::rechazo(&mut e.n, "bot-1", fundar.clone()), ofrecido);

    // Retirada la oferta, es un bot libre como cualquier otro: crea su Facción.
    mc::ok(&mut e.n, "bot-1", Comando::RetirarOferta { heroe: bot });
    mc::ok(&mut e.n, "bot-1", crear);
    assert!(e.n.heroe(bot).unwrap().faccion.is_some_and(|f| f > 2));
}

#[test]
fn un_marcado_no_crea_faccion_ni_la_pide_ni_funda_ni_declara_la_guerra_ni_acepta_misiones_del_tablon() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (plaza, rey) = (e.plaza, e.rey);
    let bot = e.contratado("bot-1", Trabajo::Defender, Objetivo::Plaza(plaza));
    let prohibido = rechazos::MERCENARIO_PROHIBIDO;
    for comando in [
        Comando::CrearFaccion { heroe: bot, nombre: "Libres".into(), sigilo: None },
        Comando::SolicitarIngreso { heroe: bot, faccion: 2 },
        Comando::DeclararGuerra { heroe: bot, a: 1, b: 2 },
        Comando::AceptarMision { heroe: bot, dueno: Dueno::Faccion(1), mision: "FG".into() },
        Comando::AceptarMision { heroe: bot, dueno: Dueno::Asentamiento(plaza), mision: "G1".into() },
    ] {
        assert_eq!(mc::rechazo(&mut e.n, "bot-1", comando.clone()), prohibido, "{comando:?}");
    }
    // Fundar, desde su columna con una Caravana de Fundación.
    let columna = mc::ok(&mut e.n, "bot-1", Comando::SalirDelCampamento { campamento: 0, heroe: bot, escuadras: vec![], carga: vec![], politica: Politica::Aceptar, caravana_fundacion: None }).creadas[0];
    mc::ok(&mut e.n, "admin", Comando::DarCaravanaFundacion { ejercito: columna });
    assert_eq!(mc::rechazo(&mut e.n, "bot-1", Comando::Fundar { ejercito: columna }), prohibido);

    // Las misiones de su propio héroe no son del tablón: esa guarda no las toca.
    assert_eq!(mc::rechazo(&mut e.n, "bot-1", Comando::AceptarMision { heroe: bot, dueno: Dueno::Heroe(bot), mision: "ZZ".into() }), rechazos::TABLON_DESCONOCIDO);
    // La administración no lleva guardas (sus órdenes son andamiaje): la rechaza la política, no el contrato.
    let r = mc::dar(&mut e.n, "admin", Comando::CrearFaccion { heroe: bot, nombre: "Libres".into(), sigilo: None });
    assert!(matches!(r.resultado, Resultado::Rechazada { codigo, .. } if codigo == rechazos::FACCION_YA_PERTENECE), "{r:?}");
    // Y quien no es un mercenario hace lo mismo sin trabas: el Rey declara la guerra.
    mc::ok(&mut e.n, "rey", Comando::DeclararGuerra { heroe: rey, a: 1, b: 2 });
    assert_eq!(e.n.estado().relacion(1, 2), Some(Relacion::Guerra));
}

#[test]
fn un_marcado_solo_convoca_con_una_orden_atacar_y_a_su_convocatoria_solo_se_unen_mercenarios() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (plaza, ajena) = (e.plaza, e.ajena);
    let convocar = |heroe| Comando::ConvocarEjercito { heroe, escuadras: vec![], carga: vec![], politica: Politica::Aceptar };
    // Sin una orden atacar, no convoca.
    let defensor = e.contratado("bot-1", Trabajo::Defender, Objetivo::Plaza(plaza));
    assert_eq!(mc::rechazo(&mut e.n, "bot-1", convocar(defensor)), rechazos::MERCENARIO_SIN_ORDEN);
    // Con ella, sí: forma el ejército con el que cumplirla.
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    let atacante = e.contratado("bot-2", Trabajo::Atacar, Objetivo::Plaza(ajena));
    let c = mc::ok(&mut e.n, "bot-2", convocar(atacante)).creadas[0];
    let unirse = |heroe| Comando::UnirseAConvocatoria { heroe, convocatoria: c, escuadras: vec![], carga: vec![] };
    // Un ciudadano sin marca no se une a ella; un mercenario con la orden de unirse a ella, sí.
    let carla = e.ciudadano("carla");
    assert_eq!(mc::rechazo(&mut e.n, "carla", unirse(carla)), rechazos::MERCENARIO_CONVOCATORIA_DE_MERCENARIOS);
    let companero = e.contratado("bot-3", Trabajo::Acompanar, Objetivo::Convocatoria(c));
    mc::ok(&mut e.n, "bot-3", unirse(companero));
    assert!(e.n.convocatoria(c).unwrap().integrantes.iter().any(|i| i.heroe == companero));
}

#[test]
fn atacar_y_perseguir_piden_la_orden_atacar_de_ese_objetivo_y_atacar_declarando_la_guerra_se_rechaza_siempre() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (plaza, ajena) = (e.plaza, e.ajena);
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    // Dos ejércitos enemigos a menos de `radioEncuentro` (1 u) de un sitio en campo abierto, lejos de la protección de los campamentos.
    let aqui = en_tierra(&m.0, campamento(&m.0, 0), 40_000, 0);
    let t = e.n.ahora();
    let (x1, x2) = (e.n.crear_ejercito(t, "gus", 2, Punto::new(aqui.x + 500, aqui.y), 500, 100_000).unwrap(), e.n.crear_ejercito(t, "gus-2", 2, Punto::new(aqui.x - 500, aqui.y), 500, 100_000).unwrap());
    let (atacante, perseguidor, defensor) = (e.contratado("bot-a", Trabajo::Atacar, Objetivo::Ejercito(x1)), e.contratado("bot-b", Trabajo::Atacar, Objetivo::Ejercito(x1)), e.contratado("bot-c", Trabajo::Defender, Objetivo::Plaza(plaza)));
    e.sale_y_llega("bot-a", atacante, aqui);
    e.sale_y_llega("bot-b", perseguidor, aqui);
    let atacar = |heroe, objetivo| Comando::Atacar { heroe, objetivo: ObjetivoAtaque::Ejercito(objetivo), declara_guerra: false };
    let perseguir = |heroe, objetivo| Comando::Perseguir { heroe, objetivo: militar::inspeccion::Objetivo::Ejercito(objetivo) };
    let sin_orden = rechazos::MERCENARIO_SIN_ORDEN;

    // Con la orden de atacar a x1, a ningún otro: ni otro ejército, ni otra clase de objetivo, ni perseguirlo.
    assert_eq!(mc::rechazo(&mut e.n, "bot-a", atacar(atacante, x2)), sin_orden);
    assert_eq!(mc::rechazo(&mut e.n, "bot-a", Comando::Atacar { heroe: atacante, objetivo: ObjetivoAtaque::Asentamiento(ajena), declara_guerra: false }), sin_orden);
    assert_eq!(mc::rechazo(&mut e.n, "bot-a", perseguir(atacante, x2)), sin_orden);
    assert_eq!(mc::rechazo(&mut e.n, "bot-b", perseguir(perseguidor, x2)), sin_orden);
    // Una orden de guardar no da permiso para atacar.
    assert_eq!(mc::rechazo(&mut e.n, "bot-c", atacar(defensor, x1)), sin_orden);
    // Y declarar la guerra al atacar se rechaza siempre, también con la orden de atacar ese mismo objetivo.
    assert_eq!(mc::rechazo(&mut e.n, "bot-a", Comando::Atacar { heroe: atacante, objetivo: ObjetivoAtaque::Ejercito(x1), declara_guerra: true }), rechazos::MERCENARIO_PROHIBIDO);

    // Con su orden, persigue y ataca: el combate se resuelve con números.
    mc::ok(&mut e.n, "bot-b", perseguir(perseguidor, x1));
    mc::ok(&mut e.n, "bot-a", atacar(atacante, x1));
    assert_eq!(mc::hechos(&e.n, "combate.resuelto").len(), 1);
}

#[test]
fn unirse_a_un_ejercito_o_a_una_convocatoria_pide_la_orden_unirse_de_ese_y_de_ningun_otro() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (plaza, rey) = (e.plaza, e.rey);
    let ((_, ej1), (_, ej2)) = (e.ejercito("ana"), e.ejercito("beto"));
    let sin_orden = rechazos::MERCENARIO_SIN_ORDEN;

    // Acompañar al ejército de ana: se une en campo a ese y a ningún otro.
    let bot = e.contratado("bot-1", Trabajo::Acompanar, Objetivo::Ejercito(ej1));
    mc::ok(&mut e.n, "bot-1", Comando::SalirDelCampamento { campamento: 0, heroe: bot, escuadras: vec![], carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None });
    assert_eq!(mc::rechazo(&mut e.n, "bot-1", Comando::UnirseEnCampo { heroe: bot, ejercito: ej2 }), sin_orden);
    mc::ok(&mut e.n, "bot-1", Comando::UnirseEnCampo { heroe: bot, ejercito: ej1 });
    assert!(e.n.ejercito(ej1).unwrap().columna.participa(bot));

    // Un permanente que guarda su plaza no se une a un ejército desde ella.
    let perm = mc::bot_ofrecido(&mut e.n, "bot-2", 0);
    mc::ok(&mut e.n, "rey", Comando::ContratarMercenario { heroe: rey, mercenario: perm, trabajo: Trabajo::Permanente, objetivo: Objetivo::Faccion(1), plazo: None, residencia: Some(plaza) });
    let unirse = Comando::UnirseAEjercito { asentamiento: plaza, heroe: perm, ejercito: ej2, escuadras: vec![], carga: vec![] };
    assert_eq!(mc::rechazo(&mut e.n, "bot-2", unirse), sin_orden);

    // Acompañar a una convocatoria (la de carla, dentro del campamento): a esa, no a un ejército ni a otra cosa.
    let carla = e.ciudadano("carla");
    let c1 = mc::ok(&mut e.n, "carla", Comando::ConvocarEjercito { heroe: carla, escuadras: vec![], carga: vec![], politica: Politica::Aceptar }).creadas[0];
    let acompanante = e.contratado("bot-3", Trabajo::Acompanar, Objetivo::Convocatoria(c1));
    let con_ejercito = e.contratado("bot-4", Trabajo::Acompanar, Objetivo::Ejercito(ej2));
    let unirse = |heroe| Comando::UnirseAConvocatoria { heroe, convocatoria: c1, escuadras: vec![], carga: vec![] };
    assert_eq!(mc::rechazo(&mut e.n, "bot-4", unirse(con_ejercito)), sin_orden);
    mc::ok(&mut e.n, "bot-3", unirse(acompanante));
    assert!(e.n.convocatoria(c1).unwrap().integrantes.iter().any(|i| i.heroe == acompanante));
}

#[test]
fn unirse_a_una_batalla_en_defensa_pide_guardar_lo_que_se_defiende_o_atacar_a_quien_lo_asedia() {
    let m = con_mundo!();
    let mut e = E::con(m, mc::nucleo_con(m, |r| r.batallas_en_unity = true));
    let plaza = e.plaza;
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    e.sin_proteccion();
    let puerta = e.puerta_de(m, plaza);
    let (x, lider) = e.sitiador("gus", 2, puerta);
    let (otro, _) = e.sitiador("hugo", 2, puerta);
    let (_, de_ana) = e.ejercito("ana");
    let guarda = mc::bot_ofrecido(&mut e.n, "bot-g", 0);
    let r = mc::contratar(&mut e.n, e.rey, guarda, Trabajo::Permanente, Objetivo::Faccion(1), None, Some(plaza));
    assert!(r.aceptada(), "{r:?}");
    let caza = e.contratado("bot-c", Trabajo::Atacar, Objetivo::Ejercito(x));
    let caza_otro = e.contratado("bot-d", Trabajo::Atacar, Objetivo::Ejercito(otro));
    let acompana = e.contratado("bot-e", Trabajo::Acompanar, Objetivo::Ejercito(de_ana));
    for bot in [guarda, caza, caza_otro, acompana] {
        e.mete_en(bot, plaza);
    }
    mc::ok(&mut e.n, "gus", Comando::Atacar { heroe: lider, objetivo: ObjetivoAtaque::Asentamiento(plaza), declara_guerra: false });
    let batalla = e.n.batallas().next().expect("se abrió el asedio").id;
    let une = |heroe, lado| Comando::UnirseABatalla { heroe, batalla, lado };
    let sin_orden = rechazos::MERCENARIO_SIN_ORDEN;

    // Ni acompañar a un ejército ni atacar a otro que no está en la batalla dan permiso para defender la plaza.
    assert_eq!(mc::rechazo(&mut e.n, "bot-e", une(acompana, None)), sin_orden);
    assert_eq!(mc::rechazo(&mut e.n, "bot-d", une(caza_otro, None)), sin_orden);
    // El permiso no cambia el bando que le toca: dentro de la plaza solo se defiende, y eso lo dice la batalla, no el contrato.
    assert_eq!(mc::rechazo(&mut e.n, "bot-g", une(guarda, Some(Lado::Atacante))), rechazos::BATALLA_INVALIDA);

    // Con `guardar` de la plaza asediada, defiende; con `atacar` de quien la asedia, también: pelea contra él, que está en el otro bando.
    mc::ok(&mut e.n, "bot-g", une(guarda, None));
    mc::ok(&mut e.n, "bot-c", une(caza, Some(Lado::Defensor)));
    let defensores: Vec<u64> = e.n.batalla(batalla).unwrap().ticket.defensor.participantes.iter().map(|p| p.heroe).collect();
    assert_eq!(defensores, vec![guarda, caza]);
}

#[test]
fn unirse_a_una_batalla_en_ataque_pide_atacar_lo_que_se_asedia() {
    let m = con_mundo!();
    let mut e = E::con(m, mc::nucleo_con(m, |r| r.batallas_en_unity = true));
    let (plaza, ajena) = (e.plaza, e.ajena);
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    e.sin_proteccion();
    let puerta = e.puerta_de(m, ajena);
    let (_, de_ana) = e.ejercito("ana");
    let atacante = e.contratado("bot-a", Trabajo::Atacar, Objetivo::Plaza(ajena));
    let guarda = e.contratado("bot-b", Trabajo::Defender, Objetivo::Plaza(plaza));
    let acompana = e.contratado("bot-c", Trabajo::Acompanar, Objetivo::Ejercito(de_ana));
    for (cuenta, bot) in [("bot-a", atacante), ("bot-b", guarda), ("bot-c", acompana)] {
        e.sale_y_llega(cuenta, bot, puerta);
    }
    let (_, lider) = e.sitiador("sitia", 1, puerta);
    mc::ok(&mut e.n, "sitia", Comando::Atacar { heroe: lider, objetivo: ObjetivoAtaque::Asentamiento(ajena), declara_guerra: false });
    let batalla = e.n.batallas().next().expect("se abrió el asedio").id;
    let une = |heroe| Comando::UnirseABatalla { heroe, batalla, lado: None };

    // A la puerta, con la columna a su alcance, tiene el bando de los que asedian; pero solo quien tiene `atacar` de la plaza asediada lo ocupa.
    assert_eq!(mc::rechazo(&mut e.n, "bot-b", une(guarda)), rechazos::MERCENARIO_SIN_ORDEN);
    assert_eq!(mc::rechazo(&mut e.n, "bot-c", une(acompana)), rechazos::MERCENARIO_SIN_ORDEN);
    mc::ok(&mut e.n, "bot-a", une(atacante));
    let atacantes: Vec<u64> = e.n.batalla(batalla).unwrap().ticket.atacante.participantes.iter().map(|p| p.heroe).collect();
    assert!(atacantes.contains(&atacante) && !atacantes.contains(&guarda) && !atacantes.contains(&acompana), "{atacantes:?}");
}

#[test]
fn adjuntar_una_caravana_o_escoltarla_pide_la_orden_guardar_de_esa_caravana() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let plaza = e.plaza;
    let [k, otra]: [u64; 2] = e.caravanas(2).try_into().unwrap();
    let puerta = e.puerta_de(m, plaza);
    let sin_orden = rechazos::MERCENARIO_SIN_ORDEN;

    // Tres que van a la puerta de la plaza al mando de su columna: el que escolta `k`, el que escolta `otra` y el que defiende la plaza.
    let (escolta, de_otra, defensor) = (e.contratado("bot-a", Trabajo::Escoltar, Objetivo::Caravana(k)), e.contratado("bot-b", Trabajo::Escoltar, Objetivo::Caravana(otra)), e.contratado("bot-c", Trabajo::Defender, Objetivo::Plaza(plaza)));
    let columnas: Vec<u64> = [("bot-a", escolta), ("bot-b", de_otra), ("bot-c", defensor)].into_iter().map(|(cuenta, bot)| e.sale_y_llega(cuenta, bot, puerta)).collect();
    let adjuntar = |caravana, ejercito| Comando::AdjuntarCaravana { caravana, ejercito };

    // La orden es de una caravana: ni la de otra, ni guardar una plaza, dan permiso.
    assert_eq!(mc::rechazo(&mut e.n, "bot-a", adjuntar(otra, columnas[0])), sin_orden);
    assert_eq!(mc::rechazo(&mut e.n, "bot-b", adjuntar(k, columnas[1])), sin_orden);
    assert_eq!(mc::rechazo(&mut e.n, "bot-c", adjuntar(k, columnas[2])), sin_orden);
    mc::ok(&mut e.n, "bot-a", adjuntar(k, columnas[0]));
    assert!(e.n.ejercito(columnas[0]).unwrap().caravanas_adjuntas.contains(&k));

    // Escoltar la cede con sus escuadras, y el héroe ha de residir en la plaza de la caravana: la administración muda a la residencia al que escolta `otra` y le da su milicia.
    let cede = e.contratado("bot-d", Trabajo::Escoltar, Objetivo::Caravana(otra));
    mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe: cede, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    e.mete_en(cede, plaza);
    let milicia = mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: cede, tropa: Tropa::MiliciaLanceros, cantidad: 20 }).creadas[0];
    let escoltar = |caravana, heroe| Comando::AsignarEscolta { caravana, heroe, escuadras: vec![milicia] };
    assert_eq!(mc::rechazo(&mut e.n, "bot-d", escoltar(k, cede)), sin_orden, "la orden es de otra caravana");
    assert_eq!(mc::rechazo(&mut e.n, "bot-c", escoltar(otra, defensor)), sin_orden, "guardar una plaza no es escoltar");
    mc::ok(&mut e.n, "bot-d", escoltar(otra, cede));
    assert_eq!(e.n.heroe(cede).unwrap().heroe.escuadra(milicia).unwrap().contenedor, Contenedor::Escolta(otra));
}

#[test]
fn guarnecer_una_avanzada_pide_guardarla_y_quien_la_guarda_no_necesita_residir_en_su_madre() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (plaza, rey) = (e.plaza, e.rey);
    let (avanzada, sitio) = e.atalaya();
    let sin_orden = rechazos::MERCENARIO_SIN_ORDEN;
    let guarnecer = |heroe, escuadra| Comando::GuarnecerAvanzada { heroe, avanzada, escuadras: vec![escuadra] };

    // Un mercenario de la plaza que la guarda, uno que reside en ella pero guarda otra cosa y un ciudadano suyo que no reside: todos a la puerta de la avanzada con una milicia.
    let guarda = e.contratado("bot-a", Trabajo::Defender, Objetivo::Avanzada(avanzada));
    let residente = mc::bot_ofrecido(&mut e.n, "bot-b", 0);
    mc::ok(&mut e.n, "rey", Comando::ContratarMercenario { heroe: rey, mercenario: residente, trabajo: Trabajo::Permanente, objetivo: Objetivo::Faccion(1), plazo: None, residencia: Some(plaza) });
    let visitante = e.ciudadano("carla");
    let mut milicias = vec![];
    for (cuenta, heroe) in [("bot-a", guarda), ("bot-b", residente), ("carla", visitante)] {
        e.sale_y_llega(cuenta, heroe, sitio);
        milicias.push(mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe, tropa: Tropa::MiliciaLanceros, cantidad: 20 }).creadas[0]);
    }
    assert_eq!(e.n.heroe(guarda).unwrap().heroe.residencia, Some(Residencia::Campamento(0)), "no reside en la plaza madre");

    // Residir en la madre no basta a un marcado, y sin residir en ella solo se guarnece con la orden de guardar esa avanzada; a un ciudadano sin marca le sigue valiendo solo residir.
    assert_eq!(mc::rechazo(&mut e.n, "bot-b", guarnecer(residente, milicias[1])), sin_orden);
    assert_eq!(mc::rechazo(&mut e.n, "carla", guarnecer(visitante, milicias[2])), rechazos::AVANZADA_NO_RESIDES);
    mc::ok(&mut e.n, "bot-a", guarnecer(guarda, milicias[0]));
    assert_eq!(e.n.heroe(guarda).unwrap().heroe.escuadra(milicias[0]).unwrap().contenedor, Contenedor::Avanzada(avanzada));
}

#[test]
fn el_mando_de_un_ejercito_no_se_cede_a_un_mercenario_y_la_sucesion_automatica_lo_deja_para_el_final() {
    /// Lo que tarda en salir del mundo quien se desconecta.
    const SALE_A_LOS: i64 = 150_000;
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (ana, ej) = e.ejercito("ana");
    // Tres mercenarios que acompañan a ese ejército y, después de ellos, dos ciudadanos: el orden del mando es el de llegada.
    let bots: Vec<u64> = (1..=3).map(|k| e.contratado(&format!("bot-{k}"), Trabajo::Acompanar, Objetivo::Ejercito(ej))).collect();
    let (beto, carla) = (e.ciudadano("beto"), e.ciudadano("carla"));
    for (cuenta, heroe) in [("bot-1", bots[0]), ("bot-2", bots[1]), ("bot-3", bots[2]), ("beto", beto), ("carla", carla)] {
        mc::ok(&mut e.n, cuenta, Comando::SalirDelCampamento { campamento: 0, heroe, escuadras: vec![], carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None });
        mc::ok(&mut e.n, cuenta, Comando::UnirseEnCampo { heroe, ejercito: ej });
    }
    let lider = |e: &E| e.n.ejercito(ej).unwrap().columna.lider;
    assert_eq!(lider(&e), ana);

    // Primera etapa: ana, la Líder, se desconecta y el mando pasa al primer ciudadano conectado, aunque los tres mercenarios llegaron antes.
    let t0 = e.n.ahora();
    e.se_desconecta("ana", ana, t0);
    e.n.avanzar_hasta(t0 + SALE_A_LOS);
    assert_eq!(lider(&e), beto);
    // Ni el Líder cede el mando a un mercenario; a un ciudadano sí.
    assert_eq!(mc::rechazo(&mut e.n, "beto", Comando::CederLiderazgo { heroe: beto, sucesor: bots[0] }), rechazos::MERCENARIO_MARCADO);
    mc::ok(&mut e.n, "beto", Comando::CederLiderazgo { heroe: beto, sucesor: carla });
    assert_eq!(lider(&e), carla);

    // Segunda: carla, la Líder, se va cuando solo queda beto sin marca, y ya desconectándose (como bot-1): manda él antes que los mercenarios conectados.
    let t1 = e.n.ahora();
    e.se_desconecta("carla", carla, t1);
    e.se_desconecta("beto", beto, t1 + 60_000);
    e.se_desconecta("bot-1", bots[0], t1 + 120_000);
    e.n.avanzar_hasta(t1 + SALE_A_LOS);
    assert_eq!(lider(&e), beto);
    // Tercera: se va beto y quedan solo mercenarios: manda el conectado, no el más antiguo, que ya se está yendo.
    e.n.avanzar_hasta(t1 + 60_000 + SALE_A_LOS);
    assert_eq!(lider(&e), bots[1]);
    // Cuarta: cuando se va ese mercenario Líder no queda ninguno conectado (bot-1 ya salió, bot-3 se va): cualquiera es mejor que una columna sin mando.
    e.se_desconecta("bot-2", bots[1], t1 + 215_000);
    e.se_desconecta("bot-3", bots[2], t1 + 220_000);
    e.n.avanzar_hasta(t1 + 215_000 + SALE_A_LOS);
    assert_eq!(lider(&e), bots[2]);
}

#[test]
fn un_marcado_no_recluta_en_una_plaza_con_el_almacen_cerrado_a_ciudadanos_y_un_ciudadano_si() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (plaza, rey) = (e.plaza, e.rey);
    // La plaza necesita gente que no produzca para reclutar: unos días.
    e.hasta(e.n.ahora() + 48 * 60 * MS_POR_MINUTO);
    // Un permanente de la plaza y un ciudadano suyo, los dos residentes y dentro.
    let bot = mc::bot_ofrecido(&mut e.n, "bot-1", 0);
    mc::ok(&mut e.n, "rey", Comando::ContratarMercenario { heroe: rey, mercenario: bot, trabajo: Trabajo::Permanente, objetivo: Objetivo::Faccion(1), plazo: None, residencia: Some(plaza) });
    let carla = e.ciudadano("carla");
    mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe: carla, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    for heroe in [bot, carla] {
        e.mete_en(heroe, plaza);
    }
    let reclutar = |heroe| Comando::ReclutarTropa { asentamiento: plaza, heroe, tropa: Tropa::MiliciaLanceros };

    // Con el almacén abierto recluta; cerrado, no: la plaza decide cuánto gastan los ciudadanos sin tener que vetar a los mercenarios.
    mc::ok(&mut e.n, "bot-1", reclutar(bot));
    mc::ok(&mut e.n, "admin", Comando::CerrarAlmacenACiudadanos { asentamiento: plaza, cerrado: true });
    assert_eq!(mc::rechazo(&mut e.n, "bot-1", reclutar(bot)), rechazos::ALMACEN_CERRADO);
    mc::ok(&mut e.n, "carla", reclutar(carla));
}

#[test]
fn la_politica_no_da_a_un_marcado_trono_embajada_ni_cargo_ni_le_deja_irse_y_la_sucesion_del_trono_lo_salta() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (plaza, rey) = (e.plaza, e.rey);
    // Quien fundó la plaza ingresó antes que nadie: sale de la Facción para que, tras el Rey, el orden de ingreso sea el del mercenario y el de carla.
    let fundador = e.n.faccion(1).unwrap().ciudadanos[1];
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: fundador, faccion: None });
    let bot = mc::bot_ofrecido(&mut e.n, "bot-1", 0);
    mc::ok(&mut e.n, "rey", Comando::ContratarMercenario { heroe: rey, mercenario: bot, trabajo: Trabajo::Permanente, objetivo: Objetivo::Faccion(1), plazo: None, residencia: Some(plaza) });
    let carla = e.ciudadano("carla");
    assert_eq!(e.n.faccion(1).unwrap().ciudadanos, vec![rey, bot, carla]);

    let marcado = rechazos::MERCENARIO_MARCADO;
    assert_eq!(mc::rechazo(&mut e.n, "bot-1", Comando::DejarFaccion { heroe: bot }), marcado);
    assert_eq!(mc::rechazo(&mut e.n, "rey", Comando::AsignarRey { heroe: rey, faccion: 1, designado: bot }), marcado);
    assert_eq!(mc::rechazo(&mut e.n, "rey", Comando::AsignarEmbajador { heroe: rey, faccion: 1, designado: bot }), marcado);
    assert_eq!(mc::rechazo(&mut e.n, "rey", Comando::AsignarCargoLocal { heroe: rey, asentamiento: plaza, cargo: Cargo::Gobernador, designado: bot }), marcado);
    // Y un ciudadano cualquiera recibe lo mismo sin trabas.
    mc::ok(&mut e.n, "rey", Comando::AsignarEmbajador { heroe: rey, faccion: 1, designado: carla });
    mc::con_cargo(&mut e.n, "rey", rey, plaza, Cargo::Gobernador, carla);

    // Si se va el Rey, el trono pasa al primero sin marca y salta al mercenario, que ingresó antes; si solo quedan marcados, la Facción se queda sin Rey y, sin ningún ciudadano sin marca,
    // los contratos caen (Mercenarios_Por_Contrato §7): el mercenario sale y la Facción se queda vacía.
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: rey, faccion: None });
    assert_eq!(e.n.faccion(1).unwrap().rey, Some(carla));
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: carla, faccion: None });
    assert_eq!(e.n.faccion(1).unwrap().rey, None);
    assert_eq!(e.n.faccion(1).unwrap().ciudadanos, Vec::<u64>::new());
    assert_eq!(mc::contrato(&e.n, bot), None);
}

/// Un catálogo pequeño: en la plaza, una individual (I1) y dos grupales (G1 y G2, esta con 20 de Participación) diarias; en la Facción, una grupal semanal (FG). Todas piden salidas del
/// campamento o de la plaza.
const CATALOGO: &str = r#"{
  "asentamiento": [
    {"id":"I1","duracion":"dia","tipo":"individual","limiteSemanal":3,"condiciones":{"salidas":1},"xp":1,"titulo":"t","texto":"t","explicacion":"e"},
    {"id":"G1","duracion":"dia","tipo":"grupal","condiciones":{"salidas":9},"xp":1,"titulo":"t","texto":"t","explicacion":"e"},
    {"id":"G2","duracion":"dia","tipo":"grupal","condiciones":{"salidas":2},"xp":1,"participacion":20,"titulo":"t","texto":"t","explicacion":"e"}
  ],
  "faccion": [
    {"id":"FG","duracion":"semana","tipo":"grupal","condiciones":{"salidas":9},"xp":1,"titulo":"t","texto":"t","explicacion":"e"}
  ]
}"#;

/// Lo que ha aportado cada héroe a la grupal `id` del tablón de `d`.
fn aportes(n: &Nucleo, d: Dueno, id: &str) -> BTreeMap<u64, u64> {
    let t = n.ahora();
    match &n.tablon_de(d).unwrap().misiones.iter().find(|m| m.def.id == id && m.vigente(t)).unwrap().progreso {
        Progreso::Grupal { aportes, .. } => aportes.clone(),
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn un_marcado_no_avanza_las_misiones_individuales_y_cuenta_en_las_grupales_de_su_faccion_y_de_la_plaza_de_su_contrato_o_su_residencia_sin_repetir() {
    let m = con_mundo!();
    let mut e = E::con(m, mc::nucleo_con(m, |r| r.economia.recurrentes = Arc::new(Catalogo::desde_texto(CATALOGO, &[]).unwrap())));
    let (plaza, rey) = (e.plaza, e.rey);
    let (pa, f1) = (Dueno::Asentamiento(plaza), Dueno::Faccion(1));
    // Quien firma por la plaza es su Gobernador: el contrato es de la plaza, no de la Facción.
    let gob = mc::firmante(&mut e.n, "gob", 1, 0, mc::ORO, Controlador::Humano);
    mc::con_cargo(&mut e.n, "rey", rey, plaza, Cargo::Gobernador, gob);

    // «bot-1» aceptó la individual siendo ciudadano de la plaza; luego dejó la Facción y se ofreció, y ahora lo contrata el Gobernador. Su aceptación sigue en el tablón.
    let bot = mc::heroe_en(&mut e.n, "bot-1", 0, Controlador::Bot);
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: bot, faccion: Some(1) });
    mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe: bot, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    mc::ok(&mut e.n, "admin", Comando::ColocarHeroe { heroe: bot, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
    mc::ok(&mut e.n, "bot-1", Comando::AceptarMision { heroe: bot, dueno: pa, mision: "I1".into() });
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: bot, faccion: None });
    mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe: bot, residencia: Some(Residencia::Campamento(0)) });
    mc::ok(&mut e.n, "admin", Comando::ColocarHeroe { heroe: bot, ubicacion: Ubicacion::Mercenarios(0) });
    mc::ok(&mut e.n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });
    let r = mc::contratar(&mut e.n, gob, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7), None);
    assert!(r.aceptada(), "{r:?}");
    // «bot-2» tiene el mismo contrato de la plaza y además reside en ella: la plaza del contrato y la de su residencia son la misma.
    let bot2 = mc::bot_ofrecido(&mut e.n, "bot-2", 0);
    let r = mc::contratar(&mut e.n, gob, bot2, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7), None);
    assert!(r.aceptada(), "{r:?}");
    mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe: bot2, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    // «bot-3» lo contrata el Rey, y no reside en la plaza: es solo de la Facción.
    let bot3 = e.contratado("bot-3", Trabajo::Defender, Objetivo::Plaza(plaza));
    let individual = |e: &E| match &e.n.tablon_de(pa).unwrap().misiones.iter().find(|m| m.def.id == "I1").unwrap().progreso {
        Progreso::Individual(a) => a[&bot].situacion,
        otro => panic!("{otro:?}"),
    };
    assert_eq!(individual(&e), Situacion::Aceptada);

    // Cada uno sale de su campamento una vez: una salida cada uno.
    for (cuenta, heroe) in [("bot-1", bot), ("bot-2", bot2), ("bot-3", bot3)] {
        mc::ok(&mut e.n, cuenta, Comando::SalirDelCampamento { campamento: 0, heroe, escuadras: vec![], carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None });
    }
    assert_eq!(individual(&e), Situacion::Aceptada, "la individual no avanza con un mercenario, aunque la aceptara antes");
    // La grupal de la plaza cuenta a los que tienen la plaza por contrato o por residencia, a cada uno una vez; la de la Facción, a los tres.
    assert_eq!(aportes(&e.n, pa, "G1"), BTreeMap::from([(bot, 1), (bot2, 1)]));
    let de_la_faccion = aportes(&e.n, f1, "FG");
    assert_eq!([bot, bot2, bot3].map(|h| de_la_faccion.get(&h).copied()), [Some(1); 3]);
}

#[test]
fn un_marcado_no_gana_participacion_pero_sus_victorias_cuentan_para_las_misiones() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let plaza = e.plaza;
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    // Dos combates iguales a campo abierto y lejos uno del otro: gana el ejército de un ciudadano que reside en la plaza y gana el de un mercenario que también reside en ella.
    let (aqui, alla) = (en_tierra(&m.0, campamento(&m.0, 0), 40_000, 0), en_tierra(&m.0, campamento(&m.0, 0), 40_000, 20_000));
    let (_, ciudadano) = e.sitiador("ctl", 1, aqui);
    let (x1, _) = e.sitiador("gus", 2, Punto::new(aqui.x + 500, aqui.y));
    let (x2, _) = e.sitiador("hugo", 2, Punto::new(alla.x + 500, alla.y));
    let bot = e.contratado("bot-1", Trabajo::Atacar, Objetivo::Ejercito(x2));
    e.sale_y_llega("bot-1", bot, alla);
    for heroe in [ciudadano, bot] {
        mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe, tropa: Tropa::EspadachinesBronce, cantidad: 18 });
        mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    }
    mc::ok(&mut e.n, "ctl", Comando::Atacar { heroe: ciudadano, objetivo: ObjetivoAtaque::Ejercito(x1), declara_guerra: false });
    mc::ok(&mut e.n, "bot-1", Comando::Atacar { heroe: bot, objetivo: ObjetivoAtaque::Ejercito(x2), declara_guerra: false });
    assert_eq!(mc::hechos(&e.n, "combate.resuelto").len(), 2);

    // La batalla ganada (5) es solo del ciudadano; la victoria de los dos cuenta en la misión de la Facción que las pide.
    assert_eq!(e.n.participacion_en(plaza).cloned().unwrap_or_default(), BTreeMap::from([(ciudadano, 5)]));
    assert_eq!(aportes(&e.n, Dueno::Faccion(1), "FS1").into_iter().filter(|&(h, _)| h == ciudadano || h == bot).collect::<Vec<_>>(), vec![(ciudadano, 1), (bot, 1)]);
}

#[test]
fn al_reclamar_una_grupal_la_participacion_se_reparte_solo_entre_quienes_la_ganan() {
    let m = con_mundo!();
    let mut e = E::con(m, mc::nucleo_con(m, |r| r.economia.recurrentes = Arc::new(Catalogo::desde_texto(CATALOGO, &[]).unwrap())));
    let (plaza, rey) = (e.plaza, e.rey);
    let pa = Dueno::Asentamiento(plaza);
    // Un ciudadano y un mercenario, los dos residentes de la plaza, salen una vez cada uno: aportan a la grupal G2, que pide dos salidas.
    let carla = e.ciudadano("carla");
    let bot = e.contratado("bot-1", Trabajo::Defender, Objetivo::Plaza(plaza));
    for (cuenta, heroe) in [("carla", carla), ("bot-1", bot)] {
        mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
        mc::ok(&mut e.n, cuenta, Comando::SalirDelCampamento { campamento: 0, heroe, escuadras: vec![], carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None });
    }
    assert_eq!(aportes(&e.n, pa, "G2"), BTreeMap::from([(carla, 1), (bot, 1)]), "los dos aportan");

    // Al reclamarla, los 20 puntos no se parten con quien no los cobra: son enteros del ciudadano.
    mc::ok(&mut e.n, "admin", Comando::ReclamarRecompensa { heroe: rey, dueno: pa, mision: "G2".into() });
    assert_eq!(e.n.participacion_en(plaza).cloned().unwrap_or_default(), BTreeMap::from([(carla, 20)]));
}
