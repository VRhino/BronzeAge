//! Las órdenes de mando y mudar al permanente (BAN54, entrada 48; Mercenarios_Por_Contrato §6): un cargo da la misma orden a entre 1 y 16 mercenarios de su Facción con el alcance y
//! el rango de §6.5, el trabajo del contrato limita qué admite (§6.4) y la orden acaba cumplida, caducada o sustituida (§6.3) con la vuelta de la inicial. Lo que cada orden deja en el
//! estado, los hechos con sus destinatarios y que ningún rechazo deja rastro. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;

use asentamiento::Cargo;
use base::{Instante, Punto, MS_POR_MINUTO};
use combate::ciclo::{Asignacion, EstadoBatalla, Instancia, Token};
use combate::resolucion::Lado;
use combate::resultado::{Razon, Resultado as ResultadoDeBatalla, ResultadoEscuadra, ResultadoHeroe};
use comun::mercenarios as mc;
use comun::real::{campamento, en_tierra, fundar, pisable, sitio_rico, M};
use mercenarios::{Objetivo, Orden, Puesto, TipoOrden, Trabajo};
use militar::catalogo::Tropa;
use militar::columna::Politica;
use militar::heroe::{Controlador, Residencia};
use nucleo::proyeccion::ids;
use nucleo::{rechazos, Comando, Dato, EventoNucleo, Nucleo, ObjetivoAtaque, Peticion, Relacion, Respuesta, Resultado};

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

/// La Facción 1 (humana) con su Rey «rey», que espera con 500 de oro en el carro de su columna a la puerta del campamento 0; su plaza, fundada por «fundador» (su Gobernador de oficio, dentro
/// de ella con su ejército), y la plaza de la Facción 2 (de «gus»). Los mercenarios y los demás cargos los pone cada prueba.
struct E {
    n: Nucleo,
    rey: u64,
    fundador: u64,
    plaza: u64,
    ajena: u64,
}

impl E {
    fn nuevo(m: &M) -> Self {
        Self::sobre(mc::nucleo(m), m)
    }

    /// Lo mismo sobre el núcleo dado.
    fn sobre(mut n: Nucleo, m: &M) -> Self {
        // El primer ciudadano de la Facción 1 es su Rey.
        let rey = mc::firmante(&mut n, "rey", 1, 0, mc::ORO, Controlador::Humano);
        let (sa, sb) = (sitio_rico(&n, &m.0, 3, 1), sitio_rico(&n, &m.0, 0, 2));
        let plaza = fundar(&mut n, "fundador", 1, sa);
        let ajena = fundar(&mut n, "gus", 2, sb);
        let fundador = n.faccion(1).and_then(|f| f.ciudadanos.iter().copied().find(|&h| h != rey)).expect("el fundador es ciudadano");
        // El fundador ya está dentro con su ejército: la columna que funda entra en la plaza (Fundacion_Y_Refundacion §5.5).
        E { n, rey, fundador, plaza, ajena }
    }

    /// Una segunda plaza de la Facción 1 (sube su nivel para que el cap de fundación lo permita), fundada por «fundador-2».
    fn otra_plaza(&mut self, m: &M) -> u64 {
        mc::ok(&mut self.n, "admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 });
        let sitio = sitio_rico(&self.n, &m.0, 1, 1);
        fundar(&mut self.n, "fundador-2", 1, sitio)
    }

    /// Un bot en el tablón del campamento 0.
    fn bot(&mut self, cuenta: &str) -> u64 {
        mc::bot_ofrecido(&mut self.n, cuenta, 0)
    }

    /// Un ciudadano de la Facción 1 con 500 de oro en el carro a la puerta del campamento 0, sin cargo.
    fn ciudadano(&mut self, cuenta: &str) -> u64 {
        mc::firmante(&mut self.n, cuenta, 1, 0, mc::ORO, Controlador::Humano)
    }

    /// Un Gobernador de la plaza, nombrado por el Rey (en lugar del Gobernador de oficio), a la puerta del campamento 0 con 500 de oro.
    fn gobernador(&mut self, cuenta: &str) -> u64 {
        let g = self.ciudadano(cuenta);
        let (rey, plaza) = (self.rey, self.plaza);
        mc::con_cargo(&mut self.n, "rey", rey, plaza, Cargo::Gobernador, g);
        g
    }

    /// Un residente de la plaza, a la puerta del campamento 0 con 500 de oro, con un cargo local que le da el Gobernador de oficio.
    fn con_cargo(&mut self, cuenta: &str, cargo: Cargo) -> u64 {
        let h = self.ciudadano(cuenta);
        let (fundador, plaza) = (self.fundador, self.plaza);
        mc::con_cargo(&mut self.n, "fundador", fundador, plaza, cargo, h);
        h
    }

    /// `firmante` contrata, con su cuenta, un trabajo temporal (sin plaza donde residir).
    fn contratar(&mut self, firmante: u64, mercenario: u64, trabajo: Trabajo, objetivo: Objetivo, plazo: Option<u8>) -> Respuesta {
        mc::contratar(&mut self.n, firmante, mercenario, trabajo, objetivo, plazo, None)
    }

    /// El Rey contrata a un permanente que reside en la plaza.
    fn permanente(&mut self, mercenario: u64, plaza: u64) {
        let rey = self.rey;
        let r = mc::contratar(&mut self.n, rey, mercenario, Trabajo::Permanente, Objetivo::Faccion(1), None, Some(plaza));
        assert!(r.aceptada(), "{r:?}");
    }

    /// `quien` da la orden con su cuenta.
    fn ordenar(&mut self, quien: u64, mercenarios: &[u64], tipo: TipoOrden, objetivo: Objetivo) -> Respuesta {
        let cuenta = self.n.heroe(quien).expect("quien ordena existe").cuenta.clone();
        mc::dar(&mut self.n, &cuenta, Comando::OrdenarMercenario { heroe: quien, mercenarios: mercenarios.to_vec(), tipo, objetivo })
    }

    /// Lo mismo, exigiendo que se acepte.
    fn ordena(&mut self, quien: u64, mercenarios: &[u64], tipo: TipoOrden, objetivo: Objetivo) {
        let r = self.ordenar(quien, mercenarios, tipo, objetivo);
        assert!(r.aceptada(), "{r:?}");
    }

    /// Lo mismo, exigiendo un rechazo que no deje rastro: devuelve su código y su mensaje.
    fn rechazada(&mut self, quien: u64, mercenarios: &[u64], tipo: TipoOrden, objetivo: Objetivo) -> (&'static str, String) {
        self.n.avanzar_hasta(self.n.ahora());
        let antes = (self.n.version(), self.n.hash_estado());
        let r = self.ordenar(quien, mercenarios, tipo, objetivo);
        assert_eq!((self.n.version(), self.n.hash_estado()), antes, "el rechazo {r:?} dejó rastro");
        match r.resultado {
            Resultado::Rechazada { codigo, mensaje, .. } => (codigo, mensaje),
            otro => panic!("se esperaba un rechazo: {otro:?}"),
        }
    }

    /// El código del rechazo, sin más.
    fn codigo(&mut self, quien: u64, mercenarios: &[u64], tipo: TipoOrden, objetivo: Objetivo) -> &'static str {
        self.rechazada(quien, mercenarios, tipo, objetivo).0
    }

    /// La orden vigente del mercenario.
    fn orden(&self, mercenario: u64) -> Option<Orden> {
        mc::contrato(&self.n, mercenario).expect("tiene contrato").orden
    }

    /// El tipo y el objetivo de su orden vigente.
    fn que_hace(&self, mercenario: u64) -> Option<(TipoOrden, Objetivo)> {
        self.orden(mercenario).map(|o| (o.tipo, o.objetivo))
    }
}

/// Lo que dura una orden `ir_a` o `atacar`: `plazoOrdenHoras`.
fn plazo_orden(n: &Nucleo) -> Instante {
    n.estado().reglas.economia.contratos.plazo_orden
}

/// Un punto cualquiera del mapa, en tierra, a `dx` unidades al este del campamento 0.
fn punto(m: &M, dx: i64) -> Objetivo {
    Objetivo::Punto(en_tierra(&m.0, campamento(&m.0, 0), dx * 1_000, 0))
}

/// Los héroes que oyen un hecho, ordenados como los deja el núcleo.
fn oyen(ids: &[u64]) -> Vec<u64> {
    let mut v = ids.to_vec();
    v.sort_unstable();
    v
}

/// Un hecho de una orden lo oyen el mercenario y quien la dio, y nadie más: ni una Facción, ni una plaza, ni una columna.
fn solo_ellos(h: &EventoNucleo, mercenario: u64, ordenante: u64) {
    assert_eq!((h.dato("mercenario"), h.dato("ordenante")), (Some(mercenario as i64), Some(ordenante as i64)));
    assert_eq!((h.heroes.clone(), h.facciones.clone(), h.asentamientos.clone(), h.ejercitos.clone()), (oyen(&[mercenario, ordenante]), vec![], vec![], vec![]), "{h:?}");
}

/// El motivo de un `orden_caducada`.
fn motivo(h: &EventoNucleo) -> Option<&str> {
    h.valores.iter().find_map(|(k, v)| if let (&"motivo", Dato::Texto(s)) = (k, v) { Some(s.as_str()) } else { None })
}

/// Un campamento de bandidos de ese nivel lo más cerca que se pueda de `centro` sin caer en una zona de influencia ni junto a un campamento de mercenarios.
fn bandidos(e: &mut E, m: &M, centro: Punto, nivel: u8) -> u64 {
    for radio in (0..40).map(|k| 4_000 + 2_000 * k) {
        for grado in (0..360).step_by(45) {
            let a = f64::from(grado).to_radians();
            let p = pisable(&m.0, Punto::new(centro.x + (radio as f64 * a.cos()) as i64, centro.y + (radio as f64 * a.sin()) as i64), 0, 0);
            let r = mc::dar(&mut e.n, "admin", Comando::ColocarBandidos { posicion: p, nivel });
            if r.aceptada() {
                return r.creadas[0];
            }
        }
    }
    panic!("sin sitio para unos bandidos cerca de {centro:?}");
}

/// El dato `tipo` de los hechos de una orden `ir_a`.
const IR_A: i64 = 1;
/// El dato `tipo` de los hechos de una orden `atacar`.
const ATACAR: i64 = 2;
/// El dato `tipo` de los hechos de una orden `unirse`.
const UNIRSE: i64 = 3;

#[test]
fn una_orden_ir_a_lleva_quien_la_dio_con_que_puesto_y_cuando_y_al_vencer_vuelve_la_inicial() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, gob, rey, plaza) = (e.bot("bot-1"), e.gobernador("gob"), e.rey, e.plaza);
    assert!(e.contratar(gob, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(3)).aceptada());
    let inicial = e.orden(bot).expect("nace con orden");
    assert_eq!((inicial.tipo, inicial.dada_por, inicial.puesto), (TipoOrden::Guardar, gob, None), "la inicial la firmó el Gobernador, pero no tiene rango");
    let plazo = plazo_orden(&e.n);
    assert_eq!(plazo, 24 * HORA);

    // El Rey, a distancia y una hora después, lo manda a un punto.
    e.n.avanzar_hasta(e.n.ahora() + HORA);
    let (t, destino) = (e.n.ahora(), punto(m, 30));
    e.ordena(rey, &[bot], TipoOrden::IrA, destino);
    assert_eq!(e.orden(bot), Some(Orden { tipo: TipoOrden::IrA, objetivo: destino, dada_por: rey, puesto: Some(Puesto::Rey), dada_en: t, vence: Some(t + plazo), contra: None, unido: false }));
    let h = mc::hechos(&e.n, "mercenarios.ordenado");
    assert_eq!((h.len(), h[0].dato("tipo")), (1, Some(IR_A)));
    solo_ellos(&h[0], bot, rey);

    // Hasta un instante antes sigue; al llegar su hora caduca y vuelve la inicial del contrato, tal como la dio quien lo firmó.
    e.n.avanzar_hasta(t + plazo - 1);
    assert_eq!(e.que_hace(bot), Some((TipoOrden::IrA, destino)));
    assert!(mc::hechos(&e.n, "mercenarios.orden_caducada").is_empty());
    e.n.avanzar_hasta(t + plazo);
    assert_eq!(e.orden(bot), Some(Orden { dada_en: t + plazo, ..inicial }), "la inicial, dada de nuevo por quien firmó");
    let h = mc::hechos(&e.n, "mercenarios.orden_caducada");
    assert_eq!((h.len(), motivo(&h[0]), h[0].dato("tipo")), (1, Some("plazo"), Some(IR_A)));
    solo_ellos(&h[0], bot, rey);
    assert!(mc::hechos(&e.n, "mercenarios.orden_cumplida").is_empty());
}

#[test]
fn una_orden_nueva_deja_vieja_la_cita_de_la_anterior() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(3)).aceptada());
    let (t0, plazo) = (e.n.ahora(), plazo_orden(&e.n));
    e.ordena(rey, &[bot], TipoOrden::IrA, punto(m, 30));
    // Una hora después, otro punto: su plazo es el de esta orden, no el de la primera.
    e.n.avanzar_hasta(t0 + HORA);
    let segundo = punto(m, 40);
    e.ordena(rey, &[bot], TipoOrden::IrA, segundo);
    assert_eq!(e.orden(bot).unwrap().vence, Some(t0 + HORA + plazo));
    e.n.avanzar_hasta(t0 + plazo);
    assert_eq!(e.que_hace(bot), Some((TipoOrden::IrA, segundo)), "la cita de la primera orden ya no vale");
    assert!(mc::hechos(&e.n, "mercenarios.orden_caducada").is_empty());
    e.n.avanzar_hasta(t0 + HORA + plazo);
    assert_eq!(e.que_hace(bot), Some((TipoOrden::Guardar, Objetivo::Plaza(plaza))));
    assert_eq!(mc::hechos(&e.n, "mercenarios.orden_caducada").len(), 1);

    // Una orden que no caduca (guardar) tampoco deja citas: dada encima de un ir_a, el plazo de éste pasa sin tocarla.
    let t1 = e.n.ahora();
    e.ordena(rey, &[bot], TipoOrden::IrA, punto(m, 30));
    e.ordena(rey, &[bot], TipoOrden::Guardar, Objetivo::Plaza(plaza));
    assert_eq!(e.orden(bot).map(|o| (o.tipo, o.vence)), Some((TipoOrden::Guardar, None)));
    e.n.avanzar_hasta(t1 + plazo + HORA);
    assert_eq!(e.que_hace(bot), Some((TipoOrden::Guardar, Objetivo::Plaza(plaza))));
    assert_eq!(mc::hechos(&e.n, "mercenarios.orden_caducada").len(), 1);
}

#[test]
fn una_orden_a_tres_mercenarios_entra_para_los_tres_con_un_hecho_para_cada_uno() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza) = (e.rey, e.plaza);
    let bots = [e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3")];
    for b in bots {
        assert!(e.contratar(rey, b, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    }
    let (t, destino) = (e.n.ahora(), punto(m, 30));
    e.ordena(rey, &bots, TipoOrden::IrA, destino);
    for b in bots {
        assert_eq!(e.orden(b), Some(Orden { tipo: TipoOrden::IrA, objetivo: destino, dada_por: rey, puesto: Some(Puesto::Rey), dada_en: t, vence: Some(t + plazo_orden(&e.n)), contra: None, unido: false }));
    }
    let h = mc::hechos(&e.n, "mercenarios.ordenado");
    assert_eq!(h.len(), 3);
    for (h, b) in h.iter().zip(bots) {
        solo_ellos(h, b, rey);
    }
}

#[test]
fn con_un_mercenario_fuera_de_alcance_la_orden_no_entra_para_ninguno_y_el_rechazo_lo_nombra() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, gob, plaza) = (e.rey, e.gobernador("gob"), e.plaza);
    let (a, b, fuera) = (e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3"));
    // Los dos primeros, de la plaza (los firma el Gobernador); el tercero, de la Facción (lo firma el Rey): el Gobernador no lo alcanza.
    assert!(e.contratar(gob, a, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    assert!(e.contratar(gob, b, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    assert!(e.contratar(rey, fuera, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    let antes = [e.orden(a), e.orden(b), e.orden(fuera)];

    let (codigo, mensaje) = e.rechazada(gob, &[a, fuera, b], TipoOrden::IrA, punto(m, 30));
    assert_eq!(codigo, rechazos::MERCENARIO_SIN_ALCANCE);
    assert!(mensaje.contains(&ids::heroe(fuera)) && !mensaje.contains(&ids::heroe(a)) && !mensaje.contains(&ids::heroe(b)), "el rechazo nombra a quien falla: {mensaje}");
    assert_eq!([e.orden(a), e.orden(b), e.orden(fuera)], antes, "no entra ninguna");
    assert!(mc::hechos(&e.n, "mercenarios.ordenado").is_empty());
    // Sin él, entra para los dos.
    e.ordena(gob, &[a, b], TipoOrden::IrA, punto(m, 30));
    assert_eq!(mc::hechos(&e.n, "mercenarios.ordenado").len(), 2);
}

#[test]
fn la_lista_lleva_de_uno_a_dieciseis_mercenarios_sin_repetir() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza) = (e.rey, e.plaza);
    let bots: Vec<u64> = (0..17).map(|i| e.bot(&format!("bot-{i}"))).collect();
    for &b in &bots {
        assert!(e.contratar(rey, b, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    }
    let (ir, destino) = (TipoOrden::IrA, punto(m, 30));
    let lista = rechazos::MERCENARIO_LISTA_INVALIDA;
    assert_eq!(e.codigo(rey, &[], ir, destino), lista, "vacía");
    assert_eq!(e.codigo(rey, &bots, ir, destino), lista, "de diecisiete");
    assert_eq!(e.codigo(rey, &[bots[0], bots[1], bots[0]], ir, destino), lista, "con un repetido");
    assert_eq!(e.codigo(rey, &[bots[2], bots[2]], ir, destino), lista, "con todos repetidos");
    assert!(bots.iter().all(|&b| e.que_hace(b) == Some((TipoOrden::Guardar, Objetivo::Plaza(plaza)))), "ninguna lista mala deja orden");

    // De uno, entra; de dieciséis, también.
    e.ordena(rey, &bots[..1], ir, destino);
    assert_eq!(mc::hechos(&e.n, "mercenarios.ordenado").len(), 1);
    e.ordena(rey, &bots[1..], ir, destino);
    assert_eq!(mc::hechos(&e.n, "mercenarios.ordenado").len(), 17);
}

#[test]
fn quien_ya_tiene_esa_orden_se_queda_como_esta_y_si_la_tienen_todos_no_cambia_nada() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza) = (e.rey, e.plaza);
    let (a, b) = (e.bot("bot-1"), e.bot("bot-2"));
    for x in [a, b] {
        assert!(e.contratar(rey, x, Trabajo::Defender, Objetivo::Plaza(plaza), Some(3)).aceptada());
    }
    let destino = punto(m, 30);
    e.ordena(rey, &[a], TipoOrden::IrA, destino);
    let de_a = e.orden(a);
    // Una hora después se la da a los dos: «a» no se toca (ni su instante ni su plazo), «b» la recibe.
    e.n.avanzar_hasta(e.n.ahora() + HORA);
    e.ordena(rey, &[a, b], TipoOrden::IrA, destino);
    assert_eq!(e.orden(a), de_a);
    assert_eq!(e.orden(b).map(|o| (o.tipo, o.objetivo, o.dada_en)), Some((TipoOrden::IrA, destino, e.n.ahora())));
    let h = mc::hechos(&e.n, "mercenarios.ordenado");
    assert_eq!(h.len(), 2, "uno por mercenario que cambió: «a» la primera vez y «b» la segunda");
    solo_ellos(&h[1], b, rey);
    // Si ya la tienen los dos, no cambia nada: ni versión ni hecho.
    let cuenta = e.n.heroe(rey).unwrap().cuenta.clone();
    mc::sin_cambios(&mut e.n, &cuenta, Comando::OrdenarMercenario { heroe: rey, mercenarios: vec![a, b], tipo: TipoOrden::IrA, objetivo: destino });
    // Otro punto es otra orden; el mismo punto con otro tipo, también.
    e.ordena(rey, &[a, b], TipoOrden::IrA, punto(m, 40));
    assert_eq!(mc::hechos(&e.n, "mercenarios.ordenado").len(), 4);
}

#[test]
fn ordenar_pide_un_contrato_con_la_faccion_de_quien_ordena_y_un_objetivo_que_exista_y_valga_para_esa_orden() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza, ajena) = (e.rey, e.plaza, e.ajena);
    let (bot, sin_contrato, de_otra) = (e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3"));
    assert!(e.contratar(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    // La Facción 3 contrata a su mercenario: acompañar a un ejército suyo.
    let rey3 = mc::firmante(&mut e.n, "rey-3", 3, 0, mc::ORO, Controlador::Humano);
    let aqui = campamento(&m.0, 1);
    let suyo = e.n.crear_ejercito(e.n.ahora(), "capi-3", 3, aqui, 500, 100_000).unwrap();
    assert!(mc::contratar(&mut e.n, rey3, de_otra, Trabajo::Acompanar, Objetivo::Ejercito(suyo), Some(1), None).aceptada());

    let sin = rechazos::MERCENARIO_SIN_CONTRATO;
    assert_eq!(e.codigo(rey, &[sin_contrato], TipoOrden::IrA, punto(m, 30)), sin, "en el tablón, sin contrato");
    assert_eq!(e.codigo(rey, &[de_otra], TipoOrden::IrA, punto(m, 30)), sin, "con el contrato de otra Facción");
    assert_eq!(e.codigo(rey, &[999_999], TipoOrden::IrA, punto(m, 30)), sin, "que no existe");

    let invalido = rechazos::MERCENARIO_OBJETIVO_INVALIDO;
    let (guardar, ir_a, atacar) = (TipoOrden::Guardar, TipoOrden::IrA, TipoOrden::Atacar);
    assert_eq!(e.codigo(rey, &[bot], guardar, Objetivo::Plaza(999_999)), invalido, "que no existe");
    assert_eq!(e.codigo(rey, &[bot], guardar, Objetivo::Plaza(ajena)), invalido, "guardar lo de otra Facción");
    assert_eq!(e.codigo(rey, &[bot], guardar, Objetivo::Bandidos(1)), invalido, "guardar una clase que no se guarda");
    assert_eq!(e.codigo(rey, &[bot], ir_a, Objetivo::Plaza(plaza)), invalido, "ir_a pide un punto");
    assert_eq!(e.codigo(rey, &[bot], atacar, Objetivo::Plaza(ajena)), invalido, "una Facción en paz no se ataca");
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    assert_eq!(e.codigo(rey, &[bot], atacar, Objetivo::Plaza(plaza)), invalido, "lo propio tampoco, aunque haya guerra");
    assert_eq!(e.codigo(rey, &[bot], TipoOrden::Unirse, Objetivo::Plaza(plaza)), invalido, "unirse pide una convocatoria o un ejército");
    // La misma Facción, el mismo mercenario: todo lo anterior no dejó orden ni hecho.
    assert_eq!(e.que_hace(bot), Some((TipoOrden::Guardar, Objetivo::Plaza(plaza))));
    assert!(mc::hechos(&e.n, "mercenarios.ordenado").is_empty());
}

#[test]
fn el_alcance_de_cada_cargo_sigue_la_tabla_de_la_seccion_6_5() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, fundador, plaza, ajena) = (e.rey, e.fundador, e.plaza, e.ajena);
    let otra = e.otra_plaza(m);
    let general = e.con_cargo("gen", Cargo::General);
    let maestro = e.con_cargo("maestro", Cargo::MaestroCaravanas);
    let sin_cargo = e.ciudadano("ana");
    let (de_la_plaza, de_la_faccion, permanente) = (e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3"));
    // Un defensor de la plaza (lo firma el General: contrato de la plaza), otro de la Facción (lo firma el Rey) y un permanente que reside en la plaza.
    assert!(e.contratar(general, de_la_plaza, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    assert!(e.contratar(rey, de_la_faccion, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    e.permanente(permanente, plaza);
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    let (sin_alcance, fuera) = (rechazos::MERCENARIO_SIN_ALCANCE, rechazos::MERCENARIO_ORDEN_FUERA_DE_CONTRATO);
    let (ir_a, guardar, atacar) = (TipoOrden::IrA, TipoOrden::Guardar, TipoOrden::Atacar);

    // Quien no tiene cargo, nada.
    assert_eq!(e.codigo(sin_cargo, &[de_la_plaza], ir_a, punto(m, 30)), sin_alcance);
    // El General y el Gobernador no alcanzan a un contrato de la Facción, aunque defienda su plaza...
    for quien in [general, fundador] {
        assert_eq!(e.codigo(quien, &[de_la_faccion], ir_a, punto(m, 30)), sin_alcance, "un contrato de la Facción es del Rey");
    }
    // ...ni, con un permanente que reside en su plaza, a guardar una plaza que no es la suya.
    for quien in [general, fundador] {
        assert_eq!(e.codigo(quien, &[permanente], guardar, Objetivo::Plaza(otra)), sin_alcance, "fuera de su plaza");
    }
    // El Gobernador solo ataca bandidos; el General ataca lo enemigo, y entonces solo lo frena lo que admite el contrato (una plaza lejos de la que se defiende).
    assert_eq!(e.codigo(fundador, &[de_la_plaza], atacar, Objetivo::Plaza(ajena)), sin_alcance, "el Gobernador no ataca lo que no son bandidos");
    assert_eq!(e.codigo(general, &[de_la_plaza], atacar, Objetivo::Plaza(ajena)), fuera, "el General sí, pero el defensor no sale de su radio");
    let sitio = e.n.estado().asentamientos[&plaza].posicion;
    let bandidos = bandidos(&mut e, m, sitio, 1);
    e.ordena(fundador, &[de_la_plaza], atacar, Objetivo::Bandidos(bandidos));
    assert_eq!(e.orden(de_la_plaza).map(|o| (o.tipo, o.puesto)), Some((atacar, Some(Puesto::Gobernador(plaza)))));
    e.ordena(general, &[de_la_plaza], guardar, Objetivo::Plaza(plaza));
    // El Maestro de caravanas solo alcanza a los escoltas: ni un defensor ni un permanente.
    for destino in [de_la_plaza, de_la_faccion, permanente] {
        assert_eq!(e.codigo(maestro, &[destino], ir_a, punto(m, 30)), sin_alcance);
    }
    // El Rey, a todos.
    e.ordena(rey, &[de_la_plaza, de_la_faccion, permanente], ir_a, punto(m, 30));
    assert_eq!(mc::hechos(&e.n, "mercenarios.ordenado").len(), 2 + 3);
}

#[test]
fn el_trabajo_del_contrato_limita_las_ordenes_y_atacar_en_radio_mide_la_distancia_al_objetivo_del_contrato() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza, ajena) = (e.rey, e.plaza, e.ajena);
    let otra = e.otra_plaza(m);
    let (defensor, atacante, escolta_de_ejercito) = (e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3"));
    assert!(e.contratar(rey, defensor, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    assert!(e.contratar(rey, atacante, Trabajo::Atacar, Objetivo::Plaza(ajena), Some(1)).aceptada());
    let aqui = campamento(&m.0, 1);
    let ejercito = e.n.crear_ejercito(e.n.ahora(), "capi", 1, aqui, 500, 100_000).unwrap();
    assert!(e.contratar(rey, escolta_de_ejercito, Trabajo::Acompanar, Objetivo::Ejercito(ejercito), Some(1)).aceptada());
    let fuera = rechazos::MERCENARIO_ORDEN_FUERA_DE_CONTRATO;

    // Un defensor guarda solo lo que defiende, va a donde le manden y ataca lo que cae en su radio; nada de unirse.
    assert_eq!(e.codigo(rey, &[defensor], TipoOrden::Guardar, Objetivo::Plaza(otra)), fuera, "guardar otra plaza");
    assert_eq!(e.codigo(rey, &[defensor], TipoOrden::Unirse, Objetivo::Ejercito(ejercito)), fuera, "unirse");
    assert_eq!(e.codigo(rey, &[defensor], TipoOrden::Atacar, Objetivo::Plaza(ajena)), fuera, "atacar lo que está lejos de su plaza");
    // Bandidos justo fuera de la zona de la plaza, a menos de `radioDefensa`; y otros lejos, junto al campamento 0.
    let (sitio, radio_defensa) = (e.n.estado().asentamientos[&plaza].posicion, e.n.estado().reglas.economia.contratos.radio_defensa);
    let (cerca, lejos) = (bandidos(&mut e, m, sitio, 1), bandidos(&mut e, m, campamento(&m.0, 0), 1));
    let donde = |e: &E, b: u64| e.n.campamento_de_bandidos(b).unwrap().posicion;
    assert!(donde(&e, cerca).dist(sitio) <= radio_defensa, "unos bandidos junto a la plaza");
    assert!(donde(&e, lejos).dist(sitio) > radio_defensa, "y otros lejos de ella");
    assert_eq!(e.codigo(rey, &[defensor], TipoOrden::Atacar, Objetivo::Bandidos(lejos)), fuera, "bandidos fuera de su radio");
    e.ordena(rey, &[defensor], TipoOrden::Atacar, Objetivo::Bandidos(cerca));
    assert_eq!(e.que_hace(defensor), Some((TipoOrden::Atacar, Objetivo::Bandidos(cerca))));
    e.ordena(rey, &[defensor], TipoOrden::IrA, punto(m, 30));

    // El que ataca por contrato: atacar lo que quiera el Rey, ir_a, y nada más.
    assert_eq!(e.codigo(rey, &[atacante], TipoOrden::Guardar, Objetivo::Plaza(plaza)), fuera);
    assert_eq!(e.codigo(rey, &[atacante], TipoOrden::Unirse, Objetivo::Ejercito(ejercito)), fuera);
    e.ordena(rey, &[atacante], TipoOrden::Atacar, Objetivo::Bandidos(lejos));
    // El que acompaña: unirse, atacar, ir_a y guardar.
    for (tipo, objetivo) in [(TipoOrden::Atacar, Objetivo::Bandidos(lejos)), (TipoOrden::IrA, punto(m, 30)), (TipoOrden::Guardar, Objetivo::Plaza(plaza))] {
        e.ordena(rey, &[escolta_de_ejercito], tipo, objetivo);
    }
    e.ordena(rey, &[escolta_de_ejercito], TipoOrden::Unirse, Objetivo::Ejercito(ejercito));
    assert_eq!(e.que_hace(escolta_de_ejercito), Some((TipoOrden::Unirse, Objetivo::Ejercito(ejercito))));
}

#[test]
fn el_gobernador_no_sustituye_la_orden_del_rey_y_el_general_si_la_del_gobernador() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, fundador, plaza) = (e.rey, e.fundador, e.plaza);
    let general = e.con_cargo("gen", Cargo::General);
    let bot = e.bot("bot-1");
    // Lo contrata el General, así que es un contrato de la plaza: lo alcanzan el Gobernador, el General y el Rey.
    assert!(e.contratar(general, bot, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7)).aceptada());
    let (uno, dos, tres, cuatro) = (punto(m, 20), punto(m, 30), punto(m, 40), punto(m, 50));
    let mas_rango = rechazos::MERCENARIO_ORDEN_DE_MAS_RANGO;

    e.ordena(fundador, &[bot], TipoOrden::IrA, uno);
    assert_eq!(e.orden(bot).map(|o| (o.dada_por, o.puesto)), Some((fundador, Some(Puesto::Gobernador(plaza)))));
    // El General (mismo rango) la sustituye: manda la última. Y el Gobernador, la suya.
    e.ordena(general, &[bot], TipoOrden::IrA, dos);
    assert_eq!(e.orden(bot).map(|o| (o.objetivo, o.dada_por, o.puesto)), Some((dos, general, Some(Puesto::General(plaza)))));
    e.ordena(fundador, &[bot], TipoOrden::IrA, tres);
    assert_eq!(e.orden(bot).map(|o| (o.objetivo, o.puesto)), Some((tres, Some(Puesto::Gobernador(plaza)))));

    // El Rey sustituye a ambos. Después, ni el Gobernador ni el General la sustituyen, y el rechazo nombra al mercenario.
    e.ordena(rey, &[bot], TipoOrden::IrA, cuatro);
    let del_rey = e.orden(bot);
    for quien in [fundador, general] {
        let (codigo, mensaje) = e.rechazada(quien, &[bot], TipoOrden::IrA, uno);
        assert_eq!(codigo, mas_rango);
        assert!(mensaje.contains(&ids::heroe(bot)), "{mensaje}");
        assert_eq!(e.codigo(quien, &[bot], TipoOrden::Guardar, Objetivo::Plaza(plaza)), mas_rango, "ni con otra orden");
    }
    assert_eq!(e.orden(bot), del_rey);
    // Si la orden del Rey ya es esa misma, no hay nada que sustituir: no cambia nada y no se queja del rango.
    let cuenta = e.n.heroe(fundador).unwrap().cuenta.clone();
    mc::sin_cambios(&mut e.n, &cuenta, Comando::OrdenarMercenario { heroe: fundador, mercenarios: vec![bot], tipo: TipoOrden::IrA, objetivo: cuatro });
    // Y vencida la del Rey, vuelve la inicial del contrato, que no tiene rango: el Gobernador ya puede.
    e.n.avanzar_hasta(del_rey.unwrap().vence.unwrap());
    assert_eq!(e.orden(bot).map(|o| (o.tipo, o.puesto)), Some((TipoOrden::Guardar, None)));
    e.ordena(fundador, &[bot], TipoOrden::IrA, uno);
}

#[test]
fn el_general_de_la_plaza_manda_al_permanente_que_firmo_el_rey_y_reside_en_ella_y_el_gobernador_de_otra_plaza_no_lo_alcanza() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, fundador, plaza) = (e.rey, e.fundador, e.plaza);
    // El fundador de la otra plaza es su Gobernador de oficio, y ya está dentro (entró al fundarla).
    let otra = e.otra_plaza(m);
    let de_otra = e.n.faccion(1).and_then(|f| f.ciudadanos.iter().copied().find(|&h| h != rey && h != fundador)).expect("quien fundó la otra plaza");
    assert!(e.n.estado().asentamientos[&otra].cargos.es_titular(Cargo::Gobernador, de_otra));
    let general = e.con_cargo("gen", Cargo::General);
    let bot = e.bot("bot-1");
    e.permanente(bot, plaza);
    let inicial = e.orden(bot).expect("nace con orden");
    assert_eq!((inicial.dada_por, inicial.puesto), (rey, None), "lo firmó el Rey y su orden inicial no tiene rango");

    // El General de la plaza donde reside lo alcanza aunque lo firmara el Rey, y la orden inicial no le gana en rango.
    let (t, destino) = (e.n.ahora(), punto(m, 30));
    e.ordena(general, &[bot], TipoOrden::IrA, destino);
    assert_eq!(e.orden(bot), Some(Orden { tipo: TipoOrden::IrA, objetivo: destino, dada_por: general, puesto: Some(Puesto::General(plaza)), dada_en: t, vence: Some(t + plazo_orden(&e.n)), contra: None, unido: false }));
    let h = mc::hechos(&e.n, "mercenarios.ordenado");
    assert_eq!(h.len(), 1);
    solo_ellos(&h[0], bot, general);

    // El Gobernador de otra plaza no lo alcanza, ni con la orden del General vigente ni con la inicial, ni para guardar la suya.
    let sin_alcance = rechazos::MERCENARIO_SIN_ALCANCE;
    assert_eq!(e.codigo(de_otra, &[bot], TipoOrden::IrA, punto(m, 40)), sin_alcance);
    assert_eq!(e.codigo(de_otra, &[bot], TipoOrden::Guardar, Objetivo::Plaza(otra)), sin_alcance);
    e.n.avanzar_hasta(t + plazo_orden(&e.n));
    assert_eq!(e.orden(bot).map(|o| (o.tipo, o.puesto)), Some((TipoOrden::Guardar, None)), "vuelve la inicial, sin rango");
    assert_eq!(e.codigo(de_otra, &[bot], TipoOrden::IrA, punto(m, 40)), sin_alcance);
    // Mudado a la otra plaza, es él quien lo alcanza, y ya no el General de la primera.
    mc::ok(&mut e.n, "rey", Comando::MudarMercenario { heroe: rey, mercenario: bot, plaza: otra });
    assert_eq!(e.codigo(general, &[bot], TipoOrden::IrA, punto(m, 40)), sin_alcance);
    e.ordena(de_otra, &[bot], TipoOrden::IrA, punto(m, 40));
    assert_eq!(e.orden(bot).map(|o| (o.dada_por, o.puesto)), Some((de_otra, Some(Puesto::Gobernador(otra)))));
}

#[test]
fn la_orden_de_un_gobernador_destituido_sigue_vigente_hasta_que_acaba_o_la_sustituye_otra() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza) = (e.rey, e.plaza);
    let (bot, otro, gob) = (e.bot("bot-1"), e.bot("bot-2"), e.gobernador("gob"));
    for b in [bot, otro] {
        assert!(e.contratar(gob, b, Trabajo::Defender, Objetivo::Plaza(plaza), Some(7)).aceptada());
    }
    let t = e.n.ahora();
    e.ordena(gob, &[bot, otro], TipoOrden::IrA, punto(m, 30));
    let suya = e.orden(bot).unwrap();
    assert_eq!((suya.dada_por, suya.puesto), (gob, Some(Puesto::Gobernador(plaza))));

    // El Rey nombra a otro Gobernador: «gob» deja de serlo y ya no ordena a nadie.
    let nuevo = e.ciudadano("gob-2");
    mc::con_cargo(&mut e.n, "rey", rey, plaza, Cargo::Gobernador, nuevo);
    assert!(!e.n.estado().asentamientos[&plaza].cargos.es_titular(Cargo::Gobernador, gob));
    assert_eq!(e.codigo(gob, &[bot], TipoOrden::IrA, punto(m, 40)), rechazos::MERCENARIO_SIN_ALCANCE);
    // Su orden sigue vigente, con el puesto con el que se dio: nada la anuló ni se avisó de nada.
    assert_eq!((e.orden(bot), e.orden(otro)), (Some(suya), Some(suya)));
    assert!(mc::hechos(&e.n, "mercenarios.orden_caducada").is_empty());
    // El nuevo Gobernador, del mismo rango y una hora después, sustituye una...
    e.n.avanzar_hasta(t + HORA);
    e.ordena(nuevo, &[bot], TipoOrden::IrA, punto(m, 40));
    assert_eq!(e.orden(bot).map(|o| (o.dada_por, o.puesto)), Some((nuevo, Some(Puesto::Gobernador(plaza)))));
    // ...y la otra sigue hasta su hora, cuando vuelve la inicial del contrato, que firmó «gob» y no tiene rango.
    e.n.avanzar_hasta(t + plazo_orden(&e.n));
    assert_eq!(e.orden(otro).map(|o| (o.tipo, o.dada_por, o.puesto)), Some((TipoOrden::Guardar, gob, None)));
    let h = mc::hechos(&e.n, "mercenarios.orden_caducada");
    assert_eq!((h.len(), motivo(&h[0])), (1, Some("plazo")));
    solo_ellos(&h[0], otro, gob);
    assert_eq!(e.que_hace(bot), Some((TipoOrden::IrA, punto(m, 40))), "la del nuevo Gobernador sigue hasta la suya");
}

#[test]
fn el_rey_muda_a_un_permanente_de_plaza_sin_cooldown_y_su_guardar_la_sigue() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza) = (e.rey, e.plaza);
    let otra = e.otra_plaza(m);
    let bot = e.bot("bot-1");
    e.permanente(bot, plaza);
    let inicial = e.orden(bot).expect("nace con orden");
    assert_eq!((inicial.tipo, inicial.objetivo, inicial.dada_por, inicial.puesto), (TipoOrden::Guardar, Objetivo::Plaza(plaza), rey, None));
    let reside = |e: &E| e.n.heroe(bot).unwrap().heroe.residencia;
    assert_eq!(reside(&e), Some(Residencia::Asentamiento(plaza as u32)));

    // A distancia y sin esperar: reside en la otra plaza y guarda la otra plaza, con la misma orden de antes (quién la dio y cuándo).
    mc::ok(&mut e.n, "rey", Comando::MudarMercenario { heroe: rey, mercenario: bot, plaza: otra });
    assert_eq!(reside(&e), Some(Residencia::Asentamiento(otra as u32)));
    assert_eq!(e.n.heroe(bot).unwrap().residencia_cambiada_en, None, "mudarlo no estrena el cooldown de residencia");
    assert_eq!(e.orden(bot), Some(Orden { objetivo: Objetivo::Plaza(otra), ..inicial }));
    let h = mc::hechos(&e.n, "mercenarios.mudado");
    assert_eq!((h.len(), h[0].dato("plaza")), (1, Some(otra as i64)));
    solo_ellos(&h[0], bot, rey);
    assert!(mc::hechos(&e.n, "mercenarios.ordenado").is_empty(), "mudarlo no es dar una orden");
    // Otra vez, de vuelta, en el acto.
    mc::sin_cambios(&mut e.n, "rey", Comando::MudarMercenario { heroe: rey, mercenario: bot, plaza: otra });
    mc::ok(&mut e.n, "rey", Comando::MudarMercenario { heroe: rey, mercenario: bot, plaza });
    assert_eq!((reside(&e), e.que_hace(bot)), (Some(Residencia::Asentamiento(plaza as u32)), Some((TipoOrden::Guardar, Objetivo::Plaza(plaza)))));

    // Si su orden es otra, no se toca; y al acabar vuelve su inicial en la plaza donde reside ahora.
    let t = e.n.ahora();
    e.ordena(rey, &[bot], TipoOrden::IrA, punto(m, 30));
    let ir_a = e.orden(bot);
    mc::ok(&mut e.n, "rey", Comando::MudarMercenario { heroe: rey, mercenario: bot, plaza: otra });
    assert_eq!(e.orden(bot), ir_a);
    e.n.avanzar_hasta(t + plazo_orden(&e.n));
    assert_eq!(e.que_hace(bot), Some((TipoOrden::Guardar, Objetivo::Plaza(otra))));
    // El Embajador también muda.
    let embajador = e.ciudadano("emba");
    mc::ok(&mut e.n, "rey", Comando::AsignarEmbajador { heroe: rey, faccion: 1, designado: embajador });
    mc::ok(&mut e.n, "emba", Comando::MudarMercenario { heroe: embajador, mercenario: bot, plaza });
    assert_eq!(e.que_hace(bot), Some((TipoOrden::Guardar, Objetivo::Plaza(plaza))));
    assert_eq!(mc::hechos(&e.n, "mercenarios.mudado").len(), 4);
}

#[test]
fn mudar_pide_un_contrato_de_la_faccion_un_cargo_de_alcance_un_permanente_y_una_plaza_de_la_faccion() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, fundador, plaza, ajena) = (e.rey, e.fundador, e.plaza, e.ajena);
    let otra = e.otra_plaza(m);
    let (permanente, temporal, sin_contrato) = (e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3"));
    e.permanente(permanente, plaza);
    assert!(e.contratar(rey, temporal, Trabajo::Defender, Objetivo::Plaza(plaza), Some(1)).aceptada());
    let general = e.con_cargo("gen", Cargo::General);
    let mudar = |heroe, mercenario, plaza| Comando::MudarMercenario { heroe, mercenario, plaza };

    assert_eq!(mc::rechazo(&mut e.n, "rey", mudar(rey, sin_contrato, otra)), rechazos::MERCENARIO_SIN_CONTRATO);
    assert_eq!(mc::rechazo(&mut e.n, "rey", mudar(rey, 999_999, otra)), rechazos::MERCENARIO_SIN_CONTRATO);
    // Solo el Rey y el Embajador: ni el Gobernador ni el General de la plaza donde reside.
    assert_eq!(mc::rechazo(&mut e.n, "fundador", mudar(fundador, permanente, otra)), rechazos::MERCENARIO_SIN_ALCANCE);
    assert_eq!(mc::rechazo(&mut e.n, "gen", mudar(general, permanente, otra)), rechazos::MERCENARIO_SIN_ALCANCE);
    // Solo al permanente.
    assert_eq!(mc::rechazo(&mut e.n, "rey", mudar(rey, temporal, otra)), rechazos::MERCENARIO_TIPO_DE_CONTRATO);
    // A una plaza de la Facción que exista.
    assert_eq!(mc::rechazo(&mut e.n, "rey", mudar(rey, permanente, ajena)), rechazos::MERCENARIO_OBJETIVO_INVALIDO, "una plaza de otra Facción");
    assert_eq!(mc::rechazo(&mut e.n, "rey", mudar(rey, permanente, 999_999)), rechazos::MERCENARIO_OBJETIVO_INVALIDO, "una plaza que no existe");
    // A la misma en la que reside, nada cambia.
    mc::sin_cambios(&mut e.n, "rey", mudar(rey, permanente, plaza));
    assert!(mc::hechos(&e.n, "mercenarios.mudado").is_empty());
}

#[test]
fn un_atacar_caduca_con_la_paz_o_pasado_su_plazo_sin_atacar_y_vuelve_la_inicial_si_la_hay() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza, ajena) = (e.rey, e.plaza, e.ajena);
    let (permanente, a_sueldo) = (e.bot("bot-1"), e.bot("bot-2"));
    e.permanente(permanente, plaza);
    let guerra = |e: &mut E, relacion| mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion });
    guerra(&mut e, Some(Relacion::Guerra));
    // Un «Atacar un objetivo» contra la plaza enemiga y un permanente al que el Rey manda atacarla.
    assert!(e.contratar(rey, a_sueldo, Trabajo::Atacar, Objetivo::Plaza(ajena), Some(7)).aceptada());
    let inicial = e.orden(permanente).unwrap();
    let t = e.n.ahora();
    e.ordena(rey, &[permanente], TipoOrden::Atacar, Objetivo::Plaza(ajena));
    let plazo = plazo_orden(&e.n);
    assert_eq!(e.orden(permanente), Some(Orden { tipo: TipoOrden::Atacar, objetivo: Objetivo::Plaza(ajena), dada_por: rey, puesto: Some(Puesto::Rey), dada_en: t, vence: Some(t + plazo), contra: Some(2), unido: false }));
    assert_eq!(e.orden(a_sueldo).map(|o| (o.tipo, o.contra, o.vence)), Some((TipoOrden::Atacar, Some(2), Some(t + plazo))), "su inicial recuerda contra quién va");

    // La paz anula los dos ataques: el permanente vuelve a guardar su plaza; el otro no tiene a qué volver y espera donde esté.
    guerra(&mut e, None);
    assert_eq!(e.orden(permanente), Some(Orden { dada_en: t, ..inicial }));
    assert_eq!(e.orden(a_sueldo), None);
    let h = mc::hechos(&e.n, "mercenarios.orden_caducada");
    assert_eq!(h.len(), 2);
    for (h, m, quien) in h.iter().zip([permanente, a_sueldo]).map(|(h, m)| (h, m, rey)) {
        assert_eq!((motivo(h), h.dato("tipo")), (Some("paz"), Some(ATACAR)));
        solo_ellos(h, m, quien);
    }
    // Sin orden vigente no queda nada en la cola: la guerra de nuevo y el paso del tiempo no resucitan nada.
    guerra(&mut e, Some(Relacion::Guerra));
    e.n.avanzar_hasta(t + 2 * plazo);
    assert_eq!(e.orden(a_sueldo), None);

    // Con guerra, el ataque vence a las 24 horas si no se hizo: el permanente vuelve a guardar y el hecho lo cuenta.
    let t = e.n.ahora();
    e.ordena(rey, &[permanente], TipoOrden::Atacar, Objetivo::Plaza(ajena));
    e.n.avanzar_hasta(t + plazo - 1);
    assert_eq!(e.que_hace(permanente), Some((TipoOrden::Atacar, Objetivo::Plaza(ajena))));
    e.n.avanzar_hasta(t + plazo);
    assert_eq!(e.orden(permanente), Some(Orden { dada_en: t + plazo, ..inicial }));
    let h = mc::hechos(&e.n, "mercenarios.orden_caducada");
    assert_eq!((h.len(), motivo(&h[2])), (3, Some("plazo")));
    solo_ellos(&h[2], permanente, rey);
    assert!(mc::hechos(&e.n, "mercenarios.orden_cumplida").is_empty(), "caducar no es cumplir");
}

#[test]
fn unirse_se_apunta_al_verle_dentro_y_caduca_si_sale_o_si_la_convocatoria_se_cancela() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, bot) = (e.rey, e.bot("bot-1"));
    // Un Líder con su convocatoria en el campamento 0, abierta a quien quiera.
    let lider = mc::heroe_en(&mut e.n, "capi", 0, Controlador::Humano);
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: lider, faccion: Some(1) });
    let escuadra = mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: lider, tropa: Tropa::MiliciaLanceros, cantidad: 10 }).creadas[0];
    let convocar = |e: &mut E| mc::ok(&mut e.n, "capi", Comando::ConvocarEjercito { heroe: lider, escuadras: vec![escuadra], carga: vec![], politica: Politica::Aceptar }).creadas[0];
    let conv = convocar(&mut e);
    assert!(e.contratar(rey, bot, Trabajo::Acompanar, Objetivo::Convocatoria(conv), Some(7)).aceptada());
    let inicial = e.orden(bot).unwrap();
    assert_eq!((inicial.tipo, inicial.objetivo, inicial.dada_por, inicial.vence, inicial.unido), (TipoOrden::Unirse, Objetivo::Convocatoria(conv), rey, None, false));

    // Mientras no entre, la orden espera sin caducar; al verle dentro se apunta.
    e.n.avanzar_hasta(e.n.ahora() + 2 * plazo_orden(&e.n));
    assert_eq!(e.orden(bot), Some(inicial));
    let unirse = Comando::UnirseAConvocatoria { heroe: bot, convocatoria: conv, escuadras: vec![], carga: vec![] };
    mc::ok(&mut e.n, "bot-1", unirse.clone());
    assert_eq!(e.orden(bot), Some(Orden { unido: true, ..inicial }));
    assert!(mc::hechos(&e.n, "mercenarios.orden_caducada").is_empty());

    // Si sale, caduca; la inicial vuelve, otra vez sin haber entrado.
    mc::ok(&mut e.n, "bot-1", Comando::SepararseDeConvocatoria { heroe: bot });
    assert_eq!(e.orden(bot), Some(Orden { dada_en: e.n.ahora(), ..inicial }));
    let h = mc::hechos(&e.n, "mercenarios.orden_caducada");
    assert_eq!((h.len(), motivo(&h[0]), h[0].dato("tipo")), (1, Some("salida"), Some(UNIRSE)));
    solo_ellos(&h[0], bot, rey);

    // Entra otra vez, y el Líder cancela la convocatoria: el objetivo desaparece, y como la inicial era ese objetivo, queda sin orden.
    mc::ok(&mut e.n, "bot-1", unirse);
    assert_eq!(e.orden(bot).map(|o| o.unido), Some(true));
    mc::ok(&mut e.n, "capi", Comando::CancelarConvocatoria { heroe: lider });
    assert_eq!(e.orden(bot), None);
    let h = mc::hechos(&e.n, "mercenarios.orden_caducada");
    assert_eq!((h.len(), motivo(&h[1])), (2, Some("objetivo")));
    solo_ellos(&h[1], bot, rey);
}

#[test]
fn si_la_convocatoria_parte_el_contrato_y_la_orden_unirse_siguen_con_el_ejercito_nuevo_hasta_que_sale() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, dentro, fuera) = (e.rey, e.bot("bot-1"), e.bot("bot-2"));
    let lider = mc::heroe_en(&mut e.n, "capi", 0, Controlador::Humano);
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: lider, faccion: Some(1) });
    let escuadra = mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: lider, tropa: Tropa::MiliciaLanceros, cantidad: 10 }).creadas[0];
    let conv = mc::ok(&mut e.n, "capi", Comando::ConvocarEjercito { heroe: lider, escuadras: vec![escuadra], carga: vec![], politica: Politica::Aceptar }).creadas[0];
    // Los dos acompañan a la convocatoria; «dentro» se apunta antes de que parta y «fuera» no.
    for b in [dentro, fuera] {
        assert!(e.contratar(rey, b, Trabajo::Acompanar, Objetivo::Convocatoria(conv), Some(7)).aceptada());
    }
    mc::ok(&mut e.n, "bot-1", Comando::UnirseAConvocatoria { heroe: dentro, convocatoria: conv, escuadras: vec![], carga: vec![] });
    let (antes_dentro, antes_fuera) = (e.orden(dentro).unwrap(), e.orden(fuera).unwrap());
    assert_eq!((antes_dentro.unido, antes_fuera.unido), (true, false));

    // La convocatoria parte: el ejército que nace es aquello que acompañaban. Siguen el contrato y la orden, ahora sobre él, y la orden conserva si ya estaba dentro; no caduca nada.
    let ejercito = mc::ok(&mut e.n, "capi", Comando::PartirConvocatoria { heroe: lider, caravana_fundacion: None }).creadas[0];
    let nuevo = Objetivo::Ejercito(ejercito);
    for b in [dentro, fuera] {
        assert_eq!(mc::contrato(&e.n, b).map(|c| (c.trabajo, c.objetivo)), Some((Trabajo::Acompanar, nuevo)), "el contrato sigue, sobre el ejército");
    }
    assert_eq!((e.orden(dentro), e.orden(fuera)), (Some(Orden { objetivo: nuevo, ..antes_dentro }), Some(Orden { objetivo: nuevo, ..antes_fuera })));
    assert!(mc::hechos(&e.n, "mercenarios.orden_caducada").is_empty());
    assert!(e.n.ejercito(ejercito).is_some_and(|x| x.columna.participa(dentro)), "«dentro» partió con él");

    // Al separarse después del ejército, la orden caduca por salida y vuelve la inicial, que acompaña al ejército y espera a que vuelva a entrar.
    mc::ok(&mut e.n, "bot-1", Comando::SepararseDelEjercito { heroe: dentro });
    let h = mc::hechos(&e.n, "mercenarios.orden_caducada");
    assert_eq!((h.len(), motivo(&h[0]), h[0].dato("tipo")), (1, Some("salida"), Some(UNIRSE)));
    solo_ellos(&h[0], dentro, rey);
    assert_eq!(e.orden(dentro), Some(Orden { objetivo: nuevo, dada_en: e.n.ahora(), unido: false, ..antes_dentro }));
    assert_eq!(e.orden(fuera), Some(Orden { objetivo: nuevo, ..antes_fuera }), "quien no había entrado no sale");
}

/// Los soldados de una escuadra completa de esa tropa.
fn completa(tropa: Tropa) -> i64 {
    militar::Reglas::cargar().tropas.tropa(tropa).unidades
}

/// Un claro cerca del campamento 0, a salvo de su protección (junto a un campamento de mercenarios nadie inicia un combate).
fn claro(e: &E, m: &M) -> Punto {
    pisable(&m.0, campamento(&m.0, 0), e.n.estado().reglas.economia.militar.mercenarios.radio_proteccion + 8_000, 0)
}

/// El Rey contrata a un mercenario («bot-1») para atacar `objetivo`; éste trae después sus tropas (para no encarecer la tarifa), forma su ejército con una convocatoria de uno (un
/// ejército solo combate con otro ejército), parte y marcha hasta junto a `destino`. Devuelve su id.
fn enviar_a_atacar(e: &mut E, m: &M, objetivo: Objetivo, tropas: &[Tropa], destino: Punto) -> u64 {
    let (rey, bot) = (e.rey, e.bot("bot-1"));
    assert!(e.contratar(rey, bot, Trabajo::Atacar, objetivo, Some(7)).aceptada());
    let escuadras: Vec<u64> = tropas.iter().map(|&t| mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: bot, tropa: t, cantidad: completa(t) }).creadas[0]).collect();
    mc::ok(&mut e.n, "bot-1", Comando::ConvocarEjercito { heroe: bot, escuadras, carga: vec![], politica: Politica::Aceptar });
    let columna = mc::ok(&mut e.n, "bot-1", Comando::PartirConvocatoria { heroe: bot, caravana_fundacion: None }).creadas[0];
    mc::ok(&mut e.n, "bot-1", Comando::MarcharA { ejercito: columna, destino: pisable(&m.0, destino, 1_000, 0) });
    let llegada = e.n.ejercito(columna).unwrap().plan.fin().expect("marcha");
    e.n.avanzar_hasta(llegada);
    bot
}

/// La Facción 1 en guerra con la 2, un ejército de la 2 («rival», con las tropas que se digan) en un claro, y un mercenario de la 1 con las suyas, contratado por el Rey para atacar
/// ese ejército, junto a él.
struct Choque {
    e: E,
    bot: u64,
    ejercito: u64,
    rival: u64,
}

impl Choque {
    fn nuevo(mut e: E, m: &M, mercenario: &[Tropa], rival: &[Tropa]) -> Self {
        mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
        let donde = claro(&e, m);
        let ejercito = e.n.crear_ejercito(e.n.ahora(), "beto", 2, donde, 500, 100_000).unwrap();
        let rival_h = e.n.ejercito(ejercito).unwrap().columna.lider;
        for &t in rival {
            mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: rival_h, tropa: t, cantidad: completa(t) });
        }
        let bot = enviar_a_atacar(&mut e, m, Objetivo::Ejercito(ejercito), mercenario, donde);
        Choque { e, bot, ejercito, rival: rival_h }
    }

    /// El mercenario ataca al ejército con su cuenta.
    fn atacar(&mut self) -> Respuesta {
        mc::dar(&mut self.e.n, "bot-1", Comando::Atacar { heroe: self.bot, objetivo: ObjetivoAtaque::Ejercito(self.ejercito), declara_guerra: false })
    }

    /// Pasa el tiempo que tardan en curarse los heridos.
    fn curar(&mut self) {
        self.e.n.avanzar_hasta(self.e.n.ahora() + 3 * MS_POR_MINUTO);
    }

    /// ¿Perdió el mercenario? Quien pierde queda herido.
    fn herido(&self) -> bool {
        self.e.n.heroe(self.bot).unwrap().heroe.herido_hasta.is_some_and(|h| h > self.e.n.ahora())
    }

    /// ¿Cuántas órdenes cumplidas lleva el mercenario? Cada una la oyen él y el Rey.
    fn cumplidas(&self) -> usize {
        let h = mc::hechos(&self.e.n, "mercenarios.orden_cumplida");
        h.iter().for_each(|h| solo_ellos(h, self.bot, self.e.rey));
        h.len()
    }
}

#[test]
fn un_atacar_se_cumple_con_el_primer_ataque_gane_o_pierda_y_no_se_repite_sin_una_orden_nueva() {
    let m = con_mundo!();
    // Contra un ejército sin tropa gana el mercenario; contra uno de tres escuadras, con una sola de lanceros, pierde.
    for (rival, pierde) in [(vec![], false), (vec![Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto, Tropa::EspadachinesBronce], true)] {
        let mut c = Choque::nuevo(E::nuevo(m), m, &[Tropa::MiliciaLanceros], &rival);
        let (bot, rey, ejercito) = (c.bot, c.e.rey, c.ejercito);
        let t = c.e.n.ahora();
        let inicial = Orden { tipo: TipoOrden::Atacar, objetivo: Objetivo::Ejercito(ejercito), dada_por: rey, puesto: None, dada_en: t, vence: Some(t + plazo_orden(&c.e.n)), contra: Some(2), unido: false };
        assert_eq!(c.e.orden(bot).map(|o| Orden { dada_en: t, vence: inicial.vence, ..o }), Some(inicial));

        let r = c.atacar();
        assert!(r.aceptada(), "{r:?}");
        assert_eq!(c.herido(), pierde, "quien pierde queda herido");
        assert_eq!(c.cumplidas(), 1, "{rival:?}");
        let h = mc::hechos(&c.e.n, "mercenarios.orden_cumplida");
        assert_eq!(h[0].dato("tipo"), Some(ATACAR));
        // Era un «Atacar un objetivo»: tras su ataque no tiene a qué volver y espera donde está.
        assert_eq!(c.e.orden(bot), None);
        assert!(mc::hechos(&c.e.n, "mercenarios.orden_caducada").is_empty());

        // Atacar otra vez, sin orden, ni se puede (la guarda del contrato) ni cumple nada.
        c.curar();
        let r = c.atacar();
        assert!(matches!(r.resultado, Resultado::Rechazada { codigo, .. } if codigo == rechazos::MERCENARIO_SIN_ORDEN), "{r:?}");
        assert_eq!((c.cumplidas(), c.e.orden(bot)), (1, None));
        // Con una orden nueva, el ataque siguiente la cumple.
        c.curar();
        c.e.ordena(rey, &[bot], TipoOrden::Atacar, Objetivo::Ejercito(ejercito));
        assert_eq!(c.e.orden(bot).map(|o| o.contra), Some(Some(2)));
        let r = c.atacar();
        assert!(r.aceptada(), "{r:?}");
        assert_eq!((c.cumplidas(), c.e.orden(bot)), (2, None));
    }
}

#[test]
fn un_ataque_a_un_campamento_de_bandidos_cumple_la_orden_se_destruya_o_no() {
    let m = con_mundo!();
    // Una escuadra de lanceros destruye un campamento de nivel 1 y no puede con el de nivel 3.
    for (nivel, destruye) in [(1, true), (3, false)] {
        let mut e = E::nuevo(m);
        let rey = e.rey;
        let donde = claro(&e, m);
        let bandidos = bandidos(&mut e, m, donde, nivel);
        let sitio = e.n.campamento_de_bandidos(bandidos).unwrap().posicion;
        let bot = enviar_a_atacar(&mut e, m, Objetivo::Bandidos(bandidos), &[Tropa::MiliciaLanceros], sitio);
        let orden = e.orden(bot).unwrap();
        assert_eq!((orden.tipo, orden.objetivo, orden.contra), (TipoOrden::Atacar, Objetivo::Bandidos(bandidos), None), "los bandidos no son de nadie");

        let r = mc::dar(&mut e.n, "bot-1", Comando::Atacar { heroe: bot, objetivo: ObjetivoAtaque::Campamento(bandidos), declara_guerra: false });
        assert!(r.aceptada(), "{r:?}");
        assert_eq!(e.n.campamento_de_bandidos(bandidos).is_none(), destruye);
        assert_eq!(mc::hechos(&e.n, "combate.campamento_destruido").len(), usize::from(destruye));
        let h = mc::hechos(&e.n, "mercenarios.orden_cumplida");
        assert_eq!(h.len(), 1, "nivel {nivel}");
        solo_ellos(&h[0], bot, rey);
        assert_eq!(e.orden(bot), None);
        assert!(mc::hechos(&e.n, "mercenarios.orden_caducada").is_empty(), "si el campamento cae en el mismo ataque, la orden se cumplió, no caducó");
    }
}

#[test]
fn un_atacar_caduca_si_su_objetivo_desaparece_antes_de_atacarlo() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, plaza, ajena) = (e.rey, e.plaza, e.ajena);
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    // Un ejército de la Facción 2 a la puerta de su plaza.
    let sitio = e.n.estado().asentamientos[&ajena].posicion;
    let ejercito = e.n.crear_ejercito(e.n.ahora(), "beto", 2, sitio, 500, 100_000).unwrap();
    let rival = e.n.ejercito(ejercito).unwrap().columna.lider;
    let (a_sueldo, permanente) = (e.bot("bot-1"), e.bot("bot-2"));
    assert!(e.contratar(rey, a_sueldo, Trabajo::Atacar, Objetivo::Ejercito(ejercito), Some(7)).aceptada());
    e.permanente(permanente, plaza);
    e.ordena(rey, &[permanente], TipoOrden::Atacar, Objetivo::Ejercito(ejercito));
    assert!(e.n.ejercito(ejercito).is_some());

    // El ejército entra entero en su plaza y deja de serlo antes de que nadie lo ataque: queda un héroe dentro, sin ejército al que atacar.
    mc::ok(&mut e.n, "beto", Comando::Guarnecer { asentamiento: ajena, heroe: rival });
    e.n.avanzar_hasta(e.n.ahora());
    assert_eq!(e.orden(a_sueldo), None, "no tiene a qué volver");
    assert_eq!(e.que_hace(permanente), Some((TipoOrden::Guardar, Objetivo::Plaza(plaza))), "vuelve su inicial");
    let h = mc::hechos(&e.n, "mercenarios.orden_caducada");
    assert_eq!(h.len(), 2);
    for (h, m) in h.iter().zip([a_sueldo, permanente]) {
        assert_eq!((motivo(h), h.dato("tipo")), (Some("objetivo"), Some(ATACAR)));
        solo_ellos(h, m, rey);
    }
    assert!(mc::hechos(&e.n, "mercenarios.orden_cumplida").is_empty());
}

// ───────────────────────────── Con servidores de batalla (Unity) ─────────────────────────────

/// Un mensaje del servidor de batalla `srv`.
fn msg(n: &mut Nucleo, t: Instante, id: &str, comando: Comando) -> Respuesta {
    n.aplicar_en(t, Peticion::de_servidor_de_batalla("srv", id, comando))
}

/// La asignación `a1` de la batalla en su revisión, con un token para cada humano.
fn asignacion(revision: u32, humanos: &[u64]) -> Asignacion {
    Asignacion {
        intento: "a1".into(),
        servidor: "ignorado: lo pone el actor".into(),
        revision,
        instancia: Instancia { host: "10.0.0.1".into(), puerto: 7777, protocolo: "udp".into() },
        tokens: humanos.iter().map(|&heroe| Token { heroe, token: format!("t{heroe}"), expira_en: 9_999_999 }).collect(),
    }
}

/// Un resultado válido de la batalla `id`: de cada escuadra mueren dos soldados, gana `ganador` y todos los héroes participan.
fn resultado(n: &Nucleo, id: u64, ganador: Lado) -> ResultadoDeBatalla {
    let b = n.batalla(id).unwrap();
    ResultadoDeBatalla {
        resultado_id: format!("res-{id}"),
        revision: b.ticket.revision,
        intento: "a1".into(),
        inicio: 1_000,
        fin: 61_000,
        ganador,
        razon: Razon::ObjetivosCapturados,
        objetivos: vec![("bandera".into(), Some(ganador))],
        por_escuadra: b
            .escuadras_de()
            .map(|e| {
                let m = 2.min(e.efectivos);
                ResultadoEscuadra { escuadra: e.id, desplegados: e.efectivos, supervivientes_al_cierre: e.efectivos - m, muertos: m, xp_ganada: 4 }
            })
            .collect(),
        por_heroe: b.heroes_que_combaten().map(|h| ResultadoHeroe { heroe: h, participo: true, sobrevivio_al_cierre: true, xp_ganada: 7, derrotado: None, botin: None }).collect(),
        version_servidor: "v1".into(),
        carros_perdidos: vec![],
    }
}

#[test]
fn con_servidores_de_batalla_el_ataque_se_cumple_al_aplicarse_el_resultado_gane_quien_gane() {
    let m = con_mundo!();
    for ganador in [Lado::Atacante, Lado::Defensor] {
        let mut c = Choque::nuevo(E::sobre(mc::nucleo_con(m, |r| r.batallas_en_unity = true), m), m, &[Tropa::EspadachinesBronce], &[Tropa::MiliciaLanceros]);
        let (bot, rey, rival) = (c.bot, c.e.rey, c.rival);
        // Con un humano entre los que combaten, atacar abre una batalla en lugar de tirar con números.
        mc::ok(&mut c.e.n, "admin", Comando::FijarControlador { heroe: rival, controlador: Controlador::Humano });
        let r = c.atacar();
        assert!(r.aceptada(), "{r:?}");
        let id = c.e.n.batallas().next().expect("se abrió una batalla").id;
        assert_eq!(c.cumplidas(), 0, "mientras se juega, la orden sigue");
        assert_eq!(c.e.que_hace(bot), Some((TipoOrden::Atacar, Objetivo::Ejercito(c.ejercito))));

        let t = c.e.n.ahora() + MS_POR_MINUTO;
        let revision = c.e.n.batalla(id).unwrap().ticket.revision;
        assert!(msg(&mut c.e.n, t, "as", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion(revision, &[rival]) }).aceptada());
        assert!(msg(&mut c.e.n, t, "in", Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision }).aceptada());
        assert_eq!(c.e.n.batalla(id).unwrap().estado, EstadoBatalla::EnCurso);
        let res = resultado(&c.e.n, id, ganador);
        let r = msg(&mut c.e.n, t + MS_POR_MINUTO, "res", Comando::AplicarResultado { batalla: id, resultado: res });
        assert!(r.aceptada(), "{r:?}");
        assert_eq!((c.cumplidas(), c.e.orden(bot)), (1, None), "{ganador:?}");
        assert_eq!(mc::hechos(&c.e.n, "mercenarios.orden_cumplida")[0].heroes, oyen(&[bot, rey]));
    }
}
