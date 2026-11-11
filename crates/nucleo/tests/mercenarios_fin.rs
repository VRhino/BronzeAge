//! El fin del contrato (BAN54, entrada 48; Mercenarios_Por_Contrato §7): el plazo que vence, el despido (con su espera de recontrato y lo que queda por valorar), el que se aplaza hasta que
//! acaba la batalla, y lo que acaba el mundo (la plaza que firmó se pierde, la Facción se queda sin ciudadanos sin marca, la administración, una anexión o una fusión le quitan al
//! mercenario la Facción). Lo que deja cada fin en el estado: el héroe fuera de la Facción y fuera de su ejército o su convocatoria, el permanente en su campamento base, la ficha, lo que
//! queda por valorar, el hecho `mercenarios.terminado`. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;

use asentamiento::Cargo;
use base::{Instante, Punto, MS_POR_MINUTO};
use combate::ciclo::{Asignacion, Instancia};
use combate::resolucion::Lado;
use combate::resultado::{Razon, Resultado as ResultadoDeBatalla, ResultadoEscuadra, ResultadoHeroe};
use comun::mercenarios as mc;
use comun::real::{campamento, fundar, pisable, sitio_rico, M};
use mercenarios::contrato::duracion;
use mercenarios::{Fin, Objetivo, Terminado, Trabajo};
use militar::catalogo::Tropa;
use militar::columna::Politica;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::{rechazos, Comando, Dato, EventoNucleo, Nucleo, ObjetivoAtaque, Peticion, Relacion};
use politica::Marca;

const MIN: Instante = MS_POR_MINUTO;

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

/// La Facción 1 (humana) con su Rey «rey», que espera con 500 de oro en el carro de su columna a la puerta del campamento 0; su plaza, junto al campamento 3, fundada por «fundador»
/// (Gobernador de oficio, fuera, en su columna), y la plaza de la Facción 2 (de «gus»). Los mercenarios y los demás firmantes los pone cada prueba.
struct E {
    n: Nucleo,
    rey: u64,
    plaza: u64,
    ajena: u64,
}

impl E {
    fn nuevo(m: &M) -> Self {
        Self::sobre(m, mc::nucleo(m))
    }

    fn sobre(m: &M, mut n: Nucleo) -> Self {
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

    /// Un Rey humano de la Facción 3 (vacía hasta ahora), con 500 de oro en el carro a la puerta del campamento 0, y la guerra de su Facción con la 2: puede contratar ataques a su plaza.
    fn rey_de_la_3(&mut self) -> u64 {
        let rey = mc::firmante(&mut self.n, "rey3", 3, 0, mc::ORO, Controlador::Humano);
        mc::ok(&mut self.n, "admin", Comando::FijarRelacion { a: 3, b: 2, relacion: Some(Relacion::Guerra) });
        rey
    }

    /// `firmante` contrata, con su cuenta, un trabajo temporal (sin plaza donde residir).
    fn contratar(&mut self, firmante: u64, mercenario: u64, trabajo: Trabajo, objetivo: Objetivo, plazo: Option<u8>) -> nucleo::Respuesta {
        mc::contratar(&mut self.n, firmante, mercenario, trabajo, objetivo, plazo, None)
    }

    /// Lo mismo, exigiendo un rechazo que no deje rastro: devuelve su código.
    fn rechazado(&mut self, firmante: u64, mercenario: u64, trabajo: Trabajo, objetivo: Objetivo, plazo: Option<u8>) -> &'static str {
        mc::contratar_rechazado(&mut self.n, firmante, mercenario, trabajo, objetivo, plazo, None)
    }

    /// `heroe` despide, con su cuenta, a `mercenario`: devuelve la respuesta tal cual.
    fn despedir(&mut self, heroe: u64, mercenario: u64) -> nucleo::Respuesta {
        let cuenta = self.n.heroe(heroe).expect("quien despide existe").cuenta.clone();
        mc::dar(&mut self.n, &cuenta, Comando::DespedirMercenario { heroe, mercenario })
    }
}

/// El motivo que lleva un hecho `mercenarios.terminado`.
fn motivo(h: &EventoNucleo) -> &str {
    h.valores.iter().find_map(|(k, v)| match v {
        Dato::Texto(t) if *k == "motivo" => Some(t.as_str()),
        _ => None,
    }).expect("el hecho lleva su motivo")
}

/// Cómo acabaron los contratos, en el orden en que se dieron los hechos: (mercenario, motivo).
fn fines(n: &Nucleo) -> Vec<(u64, String)> {
    mc::hechos(n, "mercenarios.terminado").iter().map(|h| (h.dato("mercenario").expect("nombra al mercenario") as u64, motivo(h).to_owned())).collect()
}

/// Una escuadra completa de esa tropa.
fn completa(tropa: Tropa) -> i64 {
    militar::Reglas::cargar().tropas.tropa(tropa).unidades
}

/// Un mensaje del servidor de batalla: se acepta.
fn del_servidor(n: &mut Nucleo, t: Instante, id: &str, comando: Comando) {
    let r = n.aplicar_en(t, Peticion::de_servidor_de_batalla("srv", id, comando));
    assert!(r.aceptada(), "{r:?}");
}

/// Juega la batalla `batalla` sin humanos y la cierra un minuto después, con la victoria del atacante y sin bajas: la asigna a un servidor, la inicia y aplica el resultado.
fn cerrar_batalla(n: &mut Nucleo, batalla: u64) {
    let t = n.ahora() + MIN;
    let revision = n.batalla(batalla).unwrap().ticket.revision;
    let instancia = Instancia { host: "10.0.0.1".into(), puerto: 7777, protocolo: "udp".into() };
    let asignacion = Asignacion { intento: "a1".into(), servidor: "ignorado: lo pone el actor".into(), revision, instancia, tokens: vec![] };
    del_servidor(n, t, "asignacion", Comando::RegistrarAsignacion { batalla, asignacion });
    del_servidor(n, t, "inicio", Comando::ConfirmarInicio { batalla, intento: "a1".into(), revision });
    let b = n.batalla(batalla).unwrap();
    let por_escuadra = b.escuadras_de().map(|e| ResultadoEscuadra { escuadra: e.id, desplegados: e.efectivos, supervivientes_al_cierre: e.efectivos, muertos: 0, xp_ganada: 0 }).collect();
    let por_heroe = b.heroes_que_combaten().map(|h| ResultadoHeroe { heroe: h, participo: true, sobrevivio_al_cierre: true, xp_ganada: 0, derrotado: None, botin: None }).collect();
    let resultado = ResultadoDeBatalla {
        resultado_id: format!("res-{batalla}"),
        revision,
        intento: "a1".into(),
        inicio: 1_000,
        fin: 61_000,
        ganador: Lado::Atacante,
        razon: Razon::ObjetivosCapturados,
        objetivos: vec![("bandera".into(), Some(Lado::Atacante))],
        por_escuadra,
        por_heroe,
        version_servidor: "v1".into(),
        carros_perdidos: vec![],
    };
    del_servidor(n, t, "resultado", Comando::AplicarResultado { batalla, resultado });
}

/// El campamento de mercenarios más cercano a `p`.
fn campamento_mas_cercano(m: &M, p: Punto) -> usize {
    (0..m.0.campamentos().len()).min_by_key(|&i| campamento(&m.0, i).dist2(p)).unwrap()
}

/// El mercenario, sin Facción ni contrato ni marca, y de la Facción `faccion` ya no es ciudadano.
fn libre(n: &Nucleo, mercenario: u64, faccion: u32) {
    assert!(mc::contrato(n, mercenario).is_none(), "sigue con contrato");
    assert_eq!(n.heroe(mercenario).unwrap().faccion, None);
    assert_eq!(n.estado().politica.facciones.marca_de(mercenario), None);
    assert!(n.faccion(faccion).is_none_or(|f| !f.ciudadanos.contains(&mercenario)));
}

#[test]
fn al_vencer_el_plazo_el_temporal_sale_de_la_faccion_y_sigue_en_su_campamento() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, gob, plaza) = (e.bot("bot-1"), e.gobernador("gob"), e.plaza);
    assert!(e.contratar(gob, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    let c = mc::contrato(&e.n, bot).unwrap();
    let (vence, paga, carro) = (c.vence.unwrap(), mc::oro_personal(&e.n, bot), mc::oro_del_carro(&e.n, gob));

    // Un instante antes sigue vigente, con su marca y sin hechos.
    e.n.avanzar_hasta(vence - 1);
    assert_eq!(mc::contrato(&e.n, bot), Some(c.clone()));
    assert_eq!(e.n.estado().politica.facciones.marca_de(bot), Some(Marca::Temporal));
    assert!(mc::hechos(&e.n, "mercenarios.terminado").is_empty());

    // A su hora exacta acaba: fuera de la Facción, sin contrato, dentro de su campamento y sin cooldown de nada.
    e.n.avanzar_hasta(vence);
    libre(&e.n, bot, 1);
    let h = e.n.heroe(bot).unwrap();
    assert_eq!((h.heroe.residencia, h.heroe.presencia.ubicacion, h.residencia_cambiada_en), (Some(Residencia::Campamento(0)), Ubicacion::Mercenarios(0), None));
    // La cuenta de la ficha, sin espera de recontrato (solo el despido la pone) y con lo que queda por valorar.
    let f = &e.n.estado().contratos.fichas[&bot];
    assert_eq!((f.historial.firmados, f.historial.terminados, f.historial.despidos), (1, 1, 0));
    assert!(e.n.estado().contratos.esperas.is_empty());
    let valoracion = e.n.estado().reglas.economia.contratos.valoracion;
    assert_eq!(e.n.estado().contratos.terminados[&(bot, 1)], Terminado { firmado_por: gob, renombre: c.renombre, palabra: true, valorable_hasta: vence + valoracion, valorado: false });
    // El hecho llega a la Facción y al mercenario, con su motivo.
    let hechos = mc::hechos(&e.n, "mercenarios.terminado");
    assert_eq!(hechos.len(), 1);
    let h = &hechos[0];
    assert_eq!((h.en, h.heroes.clone(), h.facciones.clone(), h.dato("faccion"), h.dato("mercenario"), motivo(h)), (vence, vec![bot], vec![1], Some(1), Some(bot as i64), "plazo"));
    // No se devuelve nada: el mercenario se queda con lo cobrado y el firmante no recupera su oro.
    assert_eq!((mc::oro_personal(&e.n, bot), mc::oro_del_carro(&e.n, gob)), (paga, carro));

    // Vuelve a ofrecerse y, sin espera, el Rey lo contrata otra vez.
    mc::ok(&mut e.n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });
    let rey = e.rey;
    assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    assert_eq!(e.n.estado().contratos.fichas[&bot].historial.firmados, 2);
}

#[test]
fn una_prorroga_deja_vieja_la_cita_del_plazo_anterior() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    let primero = mc::contrato(&e.n, bot).unwrap().vence.unwrap();
    mc::ok(&mut e.n, "rey", Comando::ProrrogarContrato { heroe: rey, mercenario: bot, plazo: 1, pagar_con: None });
    let segundo = primero + duracion(1);
    assert_eq!(mc::contrato(&e.n, bot).unwrap().vence, Some(segundo));

    // La cita del primer vencimiento llega y no acaba nada.
    e.n.avanzar_hasta(primero);
    assert!(mc::contrato(&e.n, bot).is_some() && fines(&e.n).is_empty());
    // La del segundo, sí, una sola vez.
    e.n.avanzar_hasta(segundo);
    libre(&e.n, bot, 1);
    assert_eq!(fines(&e.n), vec![(bot, "plazo".to_owned())]);
    assert_eq!(e.n.estado().contratos.fichas[&bot].historial.terminados, 1);
}

#[test]
fn despedir_acaba_el_contrato_sin_devolver_oro_y_no_se_recontrata_hasta_que_pasa_la_espera() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7)).aceptada());
    let (paga, carro) = (mc::oro_personal(&e.n, bot), mc::oro_del_carro(&e.n, rey));

    let r = e.despedir(rey, bot);
    assert!(r.aceptada(), "{r:?}");
    let t = e.n.ahora();
    libre(&e.n, bot, 1);
    let h = e.n.heroe(bot).unwrap();
    assert_eq!((h.heroe.residencia, h.heroe.presencia.ubicacion), (Some(Residencia::Campamento(0)), Ubicacion::Mercenarios(0)));
    let f = &e.n.estado().contratos.fichas[&bot];
    assert_eq!((f.historial.firmados, f.historial.terminados, f.historial.despidos), (1, 0, 1));
    // La espera de recontrato corre desde el despido, solo con esa Facción, y el contrato queda por valorar con lo que apuntó el despido: nada.
    let d = &e.n.estado().reglas.economia.contratos;
    let (espera, valoracion) = (d.espera_recontrato, d.valoracion);
    assert_eq!(e.n.estado().contratos.esperas.iter().map(|(&k, &v)| (k, v)).collect::<Vec<_>>(), vec![((bot, 1), t + espera)]);
    assert_eq!(e.n.estado().contratos.terminados[&(bot, 1)], Terminado { firmado_por: rey, renombre: 0, palabra: false, valorable_hasta: t + valoracion, valorado: false });
    let h = mc::hechos(&e.n, "mercenarios.terminado");
    assert_eq!((h.len(), h[0].en, h[0].heroes.clone(), h[0].facciones.clone(), motivo(&h[0])), (1, t, vec![bot], vec![1], "despido"));
    assert_eq!((mc::oro_personal(&e.n, bot), mc::oro_del_carro(&e.n, rey)), (paga, carro), "no se devuelve nada");

    // Vuelve a ofrecerse, pero esa Facción no lo recontrata hasta que pasa la espera: ni un instante antes.
    mc::ok(&mut e.n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });
    assert_eq!(e.rechazado(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)), rechazos::MERCENARIO_RECONTRATO_EN_ESPERA);
    e.n.avanzar_hasta(t + espera - 1);
    assert_eq!(e.rechazado(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)), rechazos::MERCENARIO_RECONTRATO_EN_ESPERA);
    e.n.avanzar_hasta(t + espera);
    assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    assert_eq!(e.n.estado().contratos.fichas[&bot].historial.firmados, 2);
}

#[test]
fn el_despedido_lo_contrata_de_inmediato_otra_faccion_y_el_fin_da_la_vuelta_por_la_foto() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza, ajena) = (e.bot("bot-1"), e.rey, e.plaza, e.ajena);
    let rey3 = e.rey_de_la_3();
    assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(3)).aceptada());
    assert!(e.despedir(rey, bot).aceptada());
    mc::ok(&mut e.n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });

    // La espera es de la Facción que despidió: la 3 lo contrata ahora mismo.
    assert!(e.contratar(rey3, bot, Trabajo::Atacar, Objetivo::Plaza(ajena), Some(1)).aceptada());
    let c = mc::contrato(&e.n, bot).unwrap();
    assert_eq!((c.faccion, e.n.heroe(bot).unwrap().faccion), (3, Some(3)));
    assert_eq!(e.n.estado().contratos.esperas.keys().copied().collect::<Vec<_>>(), vec![(bot, 1)], "la espera con la Facción 1 sigue en pie");

    let bytes = e.n.estado().serializar();
    let vuelta = nucleo::Estado::deserializar(&bytes, &e.n.estado().reglas, m.0.hash()).expect("la foto vuelve");
    assert_eq!(vuelta.contratos, e.n.estado().contratos);
    assert_eq!((vuelta.contratos.esperas.len(), vuelta.contratos.terminados.len()), (1, 1));
    assert_eq!(vuelta.serializar(), bytes);
}

#[test]
fn despedir_pide_un_contrato_de_su_faccion_y_un_cargo_que_podria_firmarlo_hoy() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza, ajena) = (e.bot("bot-1"), e.rey, e.plaza, e.ajena);
    let (gob, sin_cargo) = (e.gobernador("gob"), e.ciudadano("ana"));
    let despedir = |heroe, mercenario| Comando::DespedirMercenario { heroe, mercenario };

    // Sin contrato; con el de otra Facción.
    assert_eq!(mc::rechazo(&mut e.n, "rey", despedir(rey, bot)), rechazos::MERCENARIO_SIN_CONTRATO);
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    assert!(e.contratar(rey, bot, Trabajo::Atacar, Objetivo::Plaza(ajena), Some(1)).aceptada());
    let de_otra = mc::firmante(&mut e.n, "otro", 2, 0, mc::ORO, Controlador::Humano);
    assert_eq!(mc::rechazo(&mut e.n, "otro", despedir(de_otra, bot)), rechazos::MERCENARIO_SIN_CONTRATO);
    // La cuenta ajena no despide por el Rey; el Gobernador no alcanza un ataque a otra Facción y quien no tiene cargo, nada.
    assert_eq!(mc::rechazo(&mut e.n, "ana", despedir(rey, bot)), rechazos::NO_AUTORIZADO);
    assert_eq!(mc::rechazo(&mut e.n, "gob", despedir(gob, bot)), rechazos::MERCENARIO_SIN_ALCANCE);
    assert_eq!(mc::rechazo(&mut e.n, "ana", despedir(sin_cargo, bot)), rechazos::MERCENARIO_SIN_ALCANCE);
    assert!(mc::contrato(&e.n, bot).is_some_and(|c| c.fin_aplazado.is_none()), "ningún rechazo toca el contrato");

    // El Embajador despide a distancia, desde la puerta de otro campamento; el Gobernador, la defensa de su plaza aunque la firmara el Rey.
    let lejos = mc::firmante(&mut e.n, "lejos", 1, 3, mc::ORO, Controlador::Humano);
    mc::ok(&mut e.n, "rey", Comando::AsignarEmbajador { heroe: rey, faccion: 1, designado: lejos });
    assert!(e.despedir(lejos, bot).aceptada());
    libre(&e.n, bot, 1);
    let otro = e.bot("bot-2");
    assert!(e.contratar(rey, otro, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    assert!(e.despedir(gob, otro).aceptada());
    libre(&e.n, otro, 1);
    assert_eq!(fines(&e.n), vec![(bot, "despido".to_owned()), (otro, "despido".to_owned())]);
    // Y ya sin contrato, no hay a quien despedir.
    assert_eq!(mc::rechazo(&mut e.n, "rey", despedir(rey, otro)), rechazos::MERCENARIO_SIN_CONTRATO);
}

#[test]
fn despedir_a_quien_combate_se_acepta_y_el_contrato_acaba_al_cerrarse_su_batalla() {
    let m = con_mundo!();
    let mut e = E::sobre(m, mc::nucleo_con(m, |r| r.batallas_en_unity = true));
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7)).aceptada());

    // Sale de su campamento con su tropa, en un ejército, y un ejército de la Facción 2 lo ataca a la puerta: una batalla de Unity entre bots.
    let escuadra = mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: bot, tropa: Tropa::MiliciaLanceros, cantidad: completa(Tropa::MiliciaLanceros) }).creadas[0];
    mc::ok(&mut e.n, "bot-1", Comando::SalirDelCampamento { campamento: 0, heroe: bot, escuadras: vec![escuadra], carga: vec![], politica: Politica::Aceptar, caravana_fundacion: None });
    let columna = e.n.heroe(bot).unwrap().columna.unwrap();
    // Junto a un campamento nadie inicia un combate: el ejército se aleja un poco por donde hay tierra y allí lo espera el de la Facción 2.
    let lejos = pisable(&m.0, campamento(&m.0, 0), 20_000, 0);
    assert!(!e.n.en_proteccion_de_campamento(lejos));
    mc::ok(&mut e.n, "bot-1", Comando::MarcharA { ejercito: columna, destino: lejos });
    let t = e.n.ahora();
    let atacante = e.n.crear_ejercito(t, "otro", 2, Punto::new(lejos.x + 1_000, lejos.y), 500, 350_000).unwrap();
    let lider = e.n.ejercito(atacante).unwrap().columna.lider;
    mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: lider, tropa: Tropa::EspadachinesBronce, cantidad: completa(Tropa::EspadachinesBronce) });
    e.n.avanzar_hasta(t + 10 * MIN);
    mc::ok(&mut e.n, "otro", Comando::Atacar { heroe: lider, objetivo: ObjetivoAtaque::Ejercito(columna), declara_guerra: false });
    let batalla = e.n.batallas().next().expect("se abrió una batalla").id;
    assert!(e.n.heroe_en_batalla(bot));

    // El despido se acepta, pero no se cumple: el contrato y la marca siguen y nadie se entera todavía.
    let r = e.despedir(rey, bot);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(mc::contrato(&e.n, bot).map(|c| c.fin_aplazado), Some(Some(Fin::Despido)));
    assert_eq!((e.n.estado().politica.facciones.marca_de(bot), e.n.heroe(bot).unwrap().faccion), (Some(Marca::Temporal), Some(1)));
    assert!(fines(&e.n).is_empty() && e.n.estado().contratos.esperas.is_empty());
    // Despedir otra vez no cambia nada.
    mc::sin_cambios(&mut e.n, "rey", Comando::DespedirMercenario { heroe: rey, mercenario: bot });

    // Al cerrarse la batalla se cumple, en ese instante: de ahí corre la espera de recontrato.
    cerrar_batalla(&mut e.n, batalla);
    let t = e.n.ahora();
    libre(&e.n, bot, 1);
    assert_eq!(fines(&e.n), vec![(bot, "despido".to_owned())]);
    assert_eq!(mc::hechos(&e.n, "mercenarios.terminado")[0].en, t);
    let espera = e.n.estado().reglas.economia.contratos.espera_recontrato;
    assert_eq!(e.n.estado().contratos.esperas.get(&(bot, 1)), Some(&(t + espera)));
    assert_eq!(e.n.estado().contratos.fichas[&bot].historial.despidos, 1);
}

#[test]
fn si_cae_la_plaza_que_firmo_el_contrato_acaba_y_el_permanente_vuelve_a_su_campamento() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    // La plaza nace protegida un día: pasado, se puede asediar.
    let (rey, plaza) = (e.rey, e.plaza);
    e.n.avanzar_hasta(e.n.protegida_hasta(plaza).expect("la fundación la protege"));
    let (de_plaza, permanente, del_rey, gob) = (e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3"), e.gobernador("gob"));
    assert!(e.contratar(gob, de_plaza, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7)).aceptada());
    assert!(e.contratar(rey, del_rey, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7)).aceptada());
    mc::ok(&mut e.n, "rey", Comando::ContratarMercenario { heroe: rey, mercenario: permanente, trabajo: Trabajo::Permanente, objetivo: Objetivo::Faccion(1), plazo: None, residencia: Some(plaza), pagar_con: None });
    assert_eq!(e.n.heroe(permanente).unwrap().heroe.residencia, Some(Residencia::Asentamiento(plaza as u32)));
    assert_ne!(campamento_mas_cercano(m, e.n.estado().asentamientos[&plaza].posicion), 0, "la plaza queda más cerca de otro campamento que el base del permanente");

    // La Facción 2 la conquista.
    let pos = e.n.estado().asentamientos[&plaza].posicion;
    let t = e.n.ahora();
    let atacante = e.n.crear_ejercito(t, "otro", 2, Punto::new(pos.x + 1_000, pos.y), 500, 350_000).unwrap();
    let lider = e.n.ejercito(atacante).unwrap().columna.lider;
    for tropa in [Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto] {
        mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: lider, tropa, cantidad: completa(tropa) });
    }
    mc::ok(&mut e.n, "otro", Comando::Atacar { heroe: lider, objetivo: ObjetivoAtaque::Asentamiento(plaza), declara_guerra: true });
    let convocatoria = e.n.batallas_activas().filter_map(|b| b.convocatoria_hasta).max().expect("un asedio convoca");
    assert!(mc::contrato(&e.n, de_plaza).is_some(), "mientras se asedia, sigue");
    e.n.avanzar_hasta(convocatoria);
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, 2, "la plaza cambió de dueño");

    // El contrato de la plaza acaba en el instante de la conquista; el firmado por el Rey (ámbito: la Facción) sigue.
    libre(&e.n, de_plaza, 1);
    assert_eq!(fines(&e.n), vec![(de_plaza, "plaza_perdida".to_owned())]);
    let h = &mc::hechos(&e.n, "mercenarios.terminado")[0];
    assert_eq!((h.en, h.heroes.clone(), h.facciones.clone()), (convocatoria, vec![de_plaza], vec![1]));
    assert!(mc::contrato(&e.n, del_rey).is_some());
    // El permanente ya no tiene plaza de su Facción donde residir: vuelve a su campamento base (no al más cercano de la plaza perdida) y conserva el contrato.
    let h = e.n.heroe(permanente).unwrap();
    assert_eq!((h.faccion, h.heroe.residencia), (Some(1), Some(Residencia::Campamento(0))));
    assert_eq!(mc::contrato(&e.n, permanente).map(|c| c.fin_aplazado), Some(None));
}

#[test]
fn cuando_el_ultimo_ciudadano_sin_marca_deja_la_faccion_caen_todos_sus_contratos() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza, ajena, rey3) = (e.rey, e.plaza, e.ajena, e.rey_de_la_3());
    let ciudadano = mc::heroe_en(&mut e.n, "ciu3", 0, Controlador::Humano);
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: ciudadano, faccion: Some(3) });
    let (a, b, de_la_1) = (e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3"));
    for bot in [a, b] {
        assert!(e.contratar(rey3, bot, Trabajo::Atacar, Objetivo::Plaza(ajena), Some(1)).aceptada());
    }
    assert!(e.contratar(rey, de_la_1, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());

    // Mientras quede un ciudadano sin marca, los contratos siguen: el que se va primero no los toca.
    mc::ok(&mut e.n, "ciu3", Comando::DejarFaccion { heroe: ciudadano });
    assert!(mc::contrato(&e.n, a).is_some() && mc::contrato(&e.n, b).is_some() && fines(&e.n).is_empty());

    // El último, que además es el Rey, se va y se llevan por delante a los marcados, que no heredan el trono ni sostienen la Facción.
    mc::ok(&mut e.n, "rey3", Comando::DejarFaccion { heroe: rey3 });
    assert_eq!(fines(&e.n), vec![(a, "sin_ciudadanos".to_owned()), (b, "sin_ciudadanos".to_owned())]);
    for bot in [a, b] {
        libre(&e.n, bot, 3);
        assert_eq!(e.n.heroe(bot).unwrap().heroe.residencia, Some(Residencia::Campamento(0)));
    }
    let f = e.n.faccion(3).expect("la Facción sigue, vacía");
    assert_eq!((f.rey, f.ciudadanos.is_empty()), (None, true));
    let h = &mc::hechos(&e.n, "mercenarios.terminado")[0];
    assert_eq!((h.heroes.clone(), h.facciones.clone()), (vec![a], vec![3]));
    // Los de otra Facción no se enteran.
    assert!(mc::contrato(&e.n, de_la_1).is_some());
    // Como en toda Facción sin nadie, el primero que reciba la ciudadanía será su Rey.
    let nuevo = mc::heroe_en(&mut e.n, "nuevo", 0, Controlador::Humano);
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: nuevo, faccion: Some(3) });
    assert_eq!(e.n.faccion(3).and_then(|f| f.rey), Some(nuevo));
}

#[test]
fn el_permanente_despedido_vuelve_a_su_campamento_base_sin_cooldown() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    assert_ne!(campamento_mas_cercano(m, e.n.estado().asentamientos[&plaza].posicion), 0, "la plaza queda más cerca de otro campamento que el base del permanente");
    mc::ok(&mut e.n, "rey", Comando::ContratarMercenario { heroe: rey, mercenario: bot, trabajo: Trabajo::Permanente, objetivo: Objetivo::Faccion(1), plazo: None, residencia: Some(plaza), pagar_con: None });
    assert_eq!(e.n.heroe(bot).unwrap().heroe.residencia, Some(Residencia::Asentamiento(plaza as u32)));

    assert!(e.despedir(rey, bot).aceptada());
    libre(&e.n, bot, 1);
    let h = e.n.heroe(bot).unwrap();
    assert_eq!((h.heroe.residencia, h.residencia_cambiada_en), (Some(Residencia::Campamento(0)), None), "a su campamento base, sin cooldown");
    assert_eq!(fines(&e.n), vec![(bot, "despido".to_owned())]);
    // Sigue siendo un bot libre: puede ofrecerse otra vez desde su campamento.
    mc::ok(&mut e.n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });
}

#[test]
fn el_permanente_que_deja_de_residir_en_una_plaza_de_su_faccion_vuelve_a_su_campamento_y_conserva_el_contrato() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    mc::ok(&mut e.n, "rey", Comando::ContratarMercenario { heroe: rey, mercenario: bot, trabajo: Trabajo::Permanente, objetivo: Objetivo::Faccion(1), plazo: None, residencia: Some(plaza), pagar_con: None });

    // La administración lo muda a otro campamento: sin plaza de su Facción donde residir, vuelve al suyo.
    mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe: bot, residencia: Some(Residencia::Campamento(3)) });
    let h = e.n.heroe(bot).unwrap();
    assert_eq!((h.faccion, h.heroe.residencia), (Some(1), Some(Residencia::Campamento(0))));
    assert_eq!(e.n.estado().politica.facciones.marca_de(bot), Some(Marca::Permanente));
    assert!(mc::contrato(&e.n, bot).is_some_and(|c| c.fin_aplazado.is_none()) && fines(&e.n).is_empty());

    // Y si sin ser nada de eso queda sin residencia, también.
    mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe: bot, residencia: None });
    assert_eq!(e.n.heroe(bot).unwrap().heroe.residencia, Some(Residencia::Campamento(0)));
    // En cambio, una plaza de su Facción vale: no se le arrastra.
    mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe: bot, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    assert_eq!(e.n.heroe(bot).unwrap().heroe.residencia, Some(Residencia::Asentamiento(plaza as u32)));
}

/// Un ciudadano de la Facción 1 que sale del campamento 0 al mando de un ejército que acepta a todos: devuelve (héroe, ejército).
fn al_mando_de_un_ejercito(e: &mut E, cuenta: &str) -> (u64, u64) {
    let capi = mc::heroe_en(&mut e.n, cuenta, 0, Controlador::Humano);
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: capi, faccion: Some(1) });
    let salida = Comando::SalirDelCampamento { campamento: 0, heroe: capi, escuadras: vec![], carga: vec![], politica: Politica::Aceptar, caravana_fundacion: None };
    (capi, mc::ok(&mut e.n, cuenta, salida).creadas[0])
}

/// Los héroes que van en la columna, en el orden de su unión.
fn va_en(n: &Nucleo, columna: u64) -> Vec<u64> {
    n.ejercito(columna).expect("la columna existe").columna.participantes.iter().map(|p| p.heroe).collect()
}

#[test]
fn la_plaza_veta_a_un_permanente_residente_y_este_vuelve_a_su_campamento_con_el_contrato() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    // El Gobernador de oficio, dentro de su plaza (entró al fundarla), es quien veta.
    let (rey, plaza) = (e.rey, e.plaza);
    let gobernador = e.n.faccion(1).unwrap().ciudadanos.iter().copied().find(|&h| h != rey).expect("el fundador");
    let (bot, ciudadano) = (e.bot("bot-1"), e.ciudadano("ana"));
    mc::ok(&mut e.n, "rey", Comando::ContratarMercenario { heroe: rey, mercenario: bot, trabajo: Trabajo::Permanente, objetivo: Objetivo::Faccion(1), plazo: None, residencia: Some(plaza), pagar_con: None });
    mc::ok(&mut e.n, "admin", Comando::ColocarHeroe { heroe: bot, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
    mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe: ciudadano, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    let vetar = |vetado| Comando::VetarJugador { heroe: gobernador, asentamiento: plaza, vetado, vetar: true };

    // A un residente sin marca no se le veta.
    assert_eq!(mc::rechazo(&mut e.n, "fundador", vetar(ciudadano)), rechazos::PUERTA_RESIDENTE);
    // Al permanente sí: perder la residencia es el veto. Vuelve a su campamento base con el contrato vigente y, vetado, la puerta le saca al mapa.
    mc::ok(&mut e.n, "fundador", vetar(bot));
    let h = e.n.heroe(bot).unwrap();
    assert_eq!((h.faccion, h.heroe.residencia, h.residencia_cambiada_en), (Some(1), Some(Residencia::Campamento(0)), None));
    assert!(matches!(h.heroe.presencia.ubicacion, Ubicacion::Columna(_)), "{:?}", h.heroe.presencia.ubicacion);
    assert!(mc::contrato(&e.n, bot).is_some_and(|c| c.fin_aplazado.is_none()) && fines(&e.n).is_empty());
    assert_eq!(e.n.estado().politica.facciones.marca_de(bot), Some(Marca::Permanente));
    let veto = mc::hechos(&e.n, "asentamiento.veto");
    assert_eq!((veto.len(), veto[0].dato("heroe"), veto[0].dato("vetado")), (1, Some(bot as i64), Some(1)));
    // Vetarlo otra vez no cambia nada.
    mc::sin_cambios(&mut e.n, "fundador", vetar(bot));
}

#[test]
fn el_marcado_que_va_en_un_ejercito_se_separa_en_el_sitio_con_lo_suyo() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, bot) = (e.rey, e.bot("bot-1"));
    let (capi, ejercito) = al_mando_de_un_ejercito(&mut e, "capi");
    // Acompañar al ejército le da la orden de unirse a él.
    assert!(e.contratar(rey, bot, Trabajo::Acompanar, Objetivo::Ejercito(ejercito), Some(1)).aceptada());
    mc::salir_con_oro(&mut e.n, "bot-1", bot, 0, 0);
    mc::ok(&mut e.n, "bot-1", Comando::UnirseEnCampo { heroe: bot, ejercito });
    assert_eq!(va_en(&e.n, ejercito), vec![capi, bot]);

    assert!(e.despedir(rey, bot).aceptada());
    libre(&e.n, bot, 1);
    // El ejército sigue con su Líder, de la misma Facción; el mercenario queda en su propia columna, en el mismo punto, ya sin Facción.
    let suya = e.n.heroe(bot).unwrap().columna.expect("sigue en el mapa, en su columna");
    assert_ne!(suya, ejercito);
    assert_eq!((va_en(&e.n, ejercito), e.n.ejercito(ejercito).unwrap().columna.lider, e.n.ejercito(ejercito).unwrap().columna.faccion), (vec![capi], capi, Some(1)));
    let (suya, del_ejercito) = (e.n.ejercito(suya).unwrap(), e.n.ejercito(ejercito).unwrap());
    assert_eq!((va_en(&e.n, suya.id), suya.columna.faccion, suya.posicion), (vec![bot], None, del_ejercito.posicion));
    assert_eq!(fines(&e.n), vec![(bot, "despido".to_owned())]);
}

/// Un mercenario con contrato de la Facción 1 que sale del campamento 0 al mando de un ejército que acepta a todos: devuelve el ejército.
fn mercenario_al_mando(e: &mut E, cuenta: &str, mercenario: u64) -> u64 {
    let (rey, plaza) = (e.rey, e.plaza);
    assert!(e.contratar(rey, mercenario, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7)).aceptada());
    let salida = Comando::SalirDelCampamento { campamento: 0, heroe: mercenario, escuadras: vec![], carga: vec![], politica: Politica::Aceptar, caravana_fundacion: None };
    mc::ok(&mut e.n, cuenta, salida).creadas[0]
}

#[test]
fn el_marcado_que_encabeza_un_ejercito_cede_el_mando_a_quien_no_tiene_marca_aunque_sea_mas_reciente() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, bot, otro) = (e.rey, e.bot("bot-1"), e.bot("bot-2"));
    let ejercito = mercenario_al_mando(&mut e, "bot-1", bot);
    // Otro mercenario se une primero y después un ciudadano sin marca: el mando es de este, aunque sea más reciente.
    assert!(e.contratar(rey, otro, Trabajo::Acompanar, Objetivo::Ejercito(ejercito), Some(1)).aceptada());
    mc::salir_con_oro(&mut e.n, "bot-2", otro, 0, 0);
    mc::ok(&mut e.n, "bot-2", Comando::UnirseEnCampo { heroe: otro, ejercito });
    let capi = e.ciudadano("capi");
    mc::ok(&mut e.n, "capi", Comando::UnirseEnCampo { heroe: capi, ejercito });
    assert_eq!((va_en(&e.n, ejercito), e.n.ejercito(ejercito).unwrap().columna.lider), (vec![bot, otro, capi], bot));

    assert!(e.despedir(rey, bot).aceptada());
    libre(&e.n, bot, 1);
    assert_eq!((va_en(&e.n, ejercito), e.n.ejercito(ejercito).unwrap().columna.lider), (vec![otro, capi], capi));
    let h = mc::hechos(&e.n, "columna.liderazgo_cedido");
    assert_eq!((h.len(), h[0].dato("anterior"), h[0].dato("sucesor")), (1, Some(bot as i64), Some(capi as i64)));
}

#[test]
fn si_a_quien_encabeza_el_ejercito_solo_le_quedan_marcados_el_mando_es_del_mas_antiguo() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, primero, segundo) = (e.rey, e.bot("bot-1"), e.bot("bot-2"));
    let ejercito = mercenario_al_mando(&mut e, "bot-1", primero);
    // El segundo acompaña a ese ejército: solo hay marcados en él.
    assert!(e.contratar(rey, segundo, Trabajo::Acompanar, Objetivo::Ejercito(ejercito), Some(1)).aceptada());
    mc::salir_con_oro(&mut e.n, "bot-2", segundo, 0, 0);
    mc::ok(&mut e.n, "bot-2", Comando::UnirseEnCampo { heroe: segundo, ejercito });
    assert_eq!(va_en(&e.n, ejercito), vec![primero, segundo]);

    assert!(e.despedir(rey, primero).aceptada());
    assert_eq!((va_en(&e.n, ejercito), e.n.ejercito(ejercito).unwrap().columna.lider), (vec![segundo], segundo));
    libre(&e.n, primero, 1);
    assert_eq!(e.n.estado().politica.facciones.marca_de(segundo), Some(Marca::Temporal));
}

#[test]
fn el_marcado_que_va_en_una_convocatoria_se_separa_de_ella() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, bot) = (e.rey, e.bot("bot-1"));
    // «capi» convoca un ejército en el campamento 0, con una escuadra; el mercenario lo acompaña.
    let capi = mc::heroe_en(&mut e.n, "capi", 0, Controlador::Humano);
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: capi, faccion: Some(1) });
    let tropa = |e: &mut E, heroe| mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe, tropa: Tropa::MiliciaLanceros, cantidad: completa(Tropa::MiliciaLanceros) }).creadas[0];
    let (de_capi, del_bot) = (tropa(&mut e, capi), tropa(&mut e, bot));
    let convocar = Comando::ConvocarEjercito { heroe: capi, escuadras: vec![de_capi], carga: vec![], politica: Politica::Aceptar };
    let convocatoria = mc::ok(&mut e.n, "capi", convocar).creadas[0];
    assert!(e.contratar(rey, bot, Trabajo::Acompanar, Objetivo::Convocatoria(convocatoria), Some(1)).aceptada());
    mc::ok(&mut e.n, "bot-1", Comando::UnirseAConvocatoria { heroe: bot, convocatoria, escuadras: vec![del_bot], carga: vec![] });
    let integrantes = |e: &E| e.n.convocatoria(convocatoria).unwrap().integrantes.iter().map(|i| i.heroe).collect::<Vec<_>>();
    assert_eq!(integrantes(&e), vec![capi, bot]);

    assert!(e.despedir(rey, bot).aceptada());
    libre(&e.n, bot, 1);
    assert_eq!(integrantes(&e), vec![capi]);
    assert!(e.n.estado().militar.convocatoria_de(bot).is_none());
    let h = mc::hechos(&e.n, "convocatoria.separado");
    assert_eq!((h.len(), h[0].dato("heroe")), (1, Some(bot as i64)));
    // Sigue dentro de su campamento, con su escuadra: nada salió.
    let h = e.n.heroe(bot).unwrap();
    assert_eq!(h.heroe.presencia.ubicacion, Ubicacion::Mercenarios(0));
    assert!(h.heroe.escuadra(del_bot).is_some());
}

#[test]
fn si_la_administracion_le_cambia_la_faccion_al_marcado_el_contrato_acaba_y_no_se_le_saca_de_la_nueva() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza) = (e.rey, e.plaza);
    let (sin_faccion, de_otra) = (e.bot("bot-1"), e.bot("bot-2"));
    for bot in [sin_faccion, de_otra] {
        assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7)).aceptada());
    }

    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: sin_faccion, faccion: None });
    libre(&e.n, sin_faccion, 1);
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: de_otra, faccion: Some(2) });
    assert!(mc::contrato(&e.n, de_otra).is_none());
    assert_eq!(e.n.heroe(de_otra).unwrap().faccion, Some(2));
    assert!(e.n.faccion(2).unwrap().ciudadanos.contains(&de_otra), "no se le saca de la Facción nueva");
    assert!(e.n.faccion(1).unwrap().ciudadanos.iter().all(|&h| h != sin_faccion && h != de_otra));

    // Es un fin de la Facción, no un despido: sin espera de recontrato, con lo suyo por valorar.
    assert_eq!(fines(&e.n), vec![(sin_faccion, "faccion_perdida".to_owned()), (de_otra, "faccion_perdida".to_owned())]);
    let (esperas, terminados) = (&e.n.estado().contratos.esperas, &e.n.estado().contratos.terminados);
    assert!(esperas.is_empty() && terminados.contains_key(&(sin_faccion, 1)) && terminados.contains_key(&(de_otra, 1)));
    let f = &e.n.estado().contratos.fichas[&de_otra];
    assert_eq!((f.historial.terminados, f.historial.despidos), (1, 0));
}

#[test]
fn una_anexion_acaba_los_contratos_de_la_faccion_absorbida_y_no_los_de_la_que_absorbe() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza, ajena, rey3) = (e.rey, e.plaza, e.ajena, e.rey_de_la_3());
    let (temporal, permanente, de_la_3) = (e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3"));
    assert!(e.contratar(rey, temporal, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7)).aceptada());
    mc::ok(&mut e.n, "rey", Comando::ContratarMercenario { heroe: rey, mercenario: permanente, trabajo: Trabajo::Permanente, objetivo: Objetivo::Faccion(1), plazo: None, residencia: Some(plaza), pagar_con: None });
    assert!(e.contratar(rey3, de_la_3, Trabajo::Atacar, Objetivo::Plaza(ajena), Some(7)).aceptada());

    // La Facción 3 absorbe a la 1: los marcados de la 1 no se trasladan (la anexión solo mueve ciudadanos sin marca).
    let propuesta = mc::ok(&mut e.n, "rey3", Comando::ProponerAnexion { heroe: rey3, absorbente: 3, absorbida: 1 }).creadas[0];
    mc::ok(&mut e.n, "rey", Comando::ResponderAnexion { heroe: rey, propuesta, aceptar: true });
    assert!(e.n.faccion(1).is_none());
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, 3, "la plaza pasó a la absorbente");
    for bot in [temporal, permanente] {
        assert!(mc::contrato(&e.n, bot).is_none());
        assert_eq!((e.n.heroe(bot).unwrap().faccion, e.n.estado().politica.facciones.faccion_de(bot)), (None, None), "su copia en el héroe está al día");
        assert_eq!(e.n.estado().politica.facciones.marca_de(bot), None);
        assert!(e.n.faccion(3).unwrap().ciudadanos.iter().all(|&h| h != bot));
    }
    assert_eq!(fines(&e.n), vec![(temporal, "faccion_perdida".to_owned()), (permanente, "faccion_perdida".to_owned())]);
    // El permanente reside en su campamento base, no en una plaza que ya no es de su Facción.
    assert_eq!(e.n.heroe(permanente).unwrap().heroe.residencia, Some(Residencia::Campamento(0)));
    // El de la Facción que absorbe sigue con su contrato y su marca.
    assert!(mc::contrato(&e.n, de_la_3).is_some_and(|c| c.faccion == 3));
    assert_eq!((e.n.estado().politica.facciones.marca_de(de_la_3), e.n.heroe(de_la_3).unwrap().faccion), (Some(Marca::Temporal), Some(3)));
}

#[test]
fn una_fusion_acaba_los_contratos_de_las_dos_facciones() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza, ajena, rey3) = (e.rey, e.plaza, e.ajena, e.rey_de_la_3());
    let (de_la_1, permanente, de_la_3) = (e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3"));
    assert!(e.contratar(rey, de_la_1, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7)).aceptada());
    mc::ok(&mut e.n, "rey", Comando::ContratarMercenario { heroe: rey, mercenario: permanente, trabajo: Trabajo::Permanente, objetivo: Objetivo::Faccion(1), plazo: None, residencia: Some(plaza), pagar_con: None });
    assert!(e.contratar(rey3, de_la_3, Trabajo::Atacar, Objetivo::Plaza(ajena), Some(7)).aceptada());

    let propuesta = mc::ok(&mut e.n, "rey", Comando::ProponerFusion { heroe: rey, a: 1, b: 3, nombre: "Argos".into(), rey }).creadas[0];
    let nueva = mc::ok(&mut e.n, "rey3", Comando::ResponderFusion { heroe: rey3, propuesta, aceptar: true }).creadas[0] as u32;
    assert!(e.n.faccion(1).is_none() && e.n.faccion(3).is_none());
    let ciudadanos = e.n.faccion(nueva).unwrap().ciudadanos.clone();
    for bot in [de_la_1, permanente, de_la_3] {
        assert!(mc::contrato(&e.n, bot).is_none());
        assert_eq!(e.n.heroe(bot).unwrap().faccion, None);
        assert!(!ciudadanos.contains(&bot), "los marcados no pasan a la Facción nueva");
    }
    assert!(ciudadanos.contains(&rey) && ciudadanos.contains(&rey3));
    assert_eq!(fines(&e.n).into_iter().map(|(_, motivo)| motivo).collect::<Vec<_>>(), vec!["faccion_perdida"; 3]);
    assert_eq!(e.n.heroe(permanente).unwrap().heroe.residencia, Some(Residencia::Campamento(0)));
}

#[test]
fn quien_esta_dentro_de_una_plaza_que_ya_no_le_admite_al_acabar_su_contrato_sale_al_mapa() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    mc::ok(&mut e.n, "rey", Comando::ContratarMercenario { heroe: rey, mercenario: bot, trabajo: Trabajo::Permanente, objetivo: Objetivo::Faccion(1), plazo: None, residencia: Some(plaza), pagar_con: None });
    // Entra en su plaza, donde reside: mientras sea de la Facción, está en su casa.
    mc::ok(&mut e.n, "admin", Comando::ColocarHeroe { heroe: bot, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
    assert_eq!(e.n.heroe(bot).unwrap().heroe.presencia.ubicacion, Ubicacion::Asentamiento(plaza as u32));

    // Despedido, ya no reside allí ni es de la Facción: la puerta no le deja estar dentro y sale al mapa con su columna.
    assert!(e.despedir(rey, bot).aceptada());
    libre(&e.n, bot, 1);
    let h = e.n.heroe(bot).unwrap();
    assert_eq!(h.heroe.residencia, Some(Residencia::Campamento(0)));
    assert!(matches!(h.heroe.presencia.ubicacion, Ubicacion::Columna(_)), "{:?}", h.heroe.presencia.ubicacion);
    assert_eq!(h.columna.and_then(|c| e.n.ejercito(c)).map(|c| (c.columna.faccion, c.columna.participantes.len())), Some((None, 1)));
}
