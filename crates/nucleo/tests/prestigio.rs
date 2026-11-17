//! El prestigio de la Facción (tanda 69, Diplomacia_Facciones §3.4 `PRESTIGIO`): lo que suma cada fuente (victoria contra héroes de otra Facción en un
//! combate de columnas y en una batalla de Unity, conquista, fundación, subida de nivel, trueque y orden de mercado), lo que no suma (bandidos, la misma Facción, varios héroes ganadores)
//! y lo que se desbloquea por umbrales (el tope de alianzas, la Maravilla y las estatuas de la capital). Las cuantías y los umbrales salen del bloque `PRESTIGIO` (`comun::reglas()`).
//! Las pruebas de la primera mitad corren sobre la retícula sintética de `comun`; las de la segunda, sobre el artefacto de mundo real y se omiten si no está.

mod comun;

use asentamiento::Recurso::{self, *};
use asentamiento::Tipo;
use comun::real::{self, M};
use comercio::caravana::{Animal, TipoCarro};
use base::{Instante, Punto, MS_POR_MINUTO};
use combate::ciclo::{Asignacion, EstadoBatalla, Instancia, Token};
use combate::resolucion::Lado;
use combate::resultado::{Razon, Resultado as ResultadoDeBatalla, ResultadoEscuadra, ResultadoHeroe};
use comun::{c, nucleo};
use militar::catalogo::Tropa;
use militar::heroe::Controlador;
use militar::presencia::Ubicacion;
use nucleo::{rechazos, Ajustes, Arranque, Comando, Nucleo, ObjetivoAtaque, Peticion, Reglas, Relacion, Resultado, Respuesta};
use politica::datos::ReglasPrestigio;
use std::sync::Arc;

const MIN: Instante = MS_POR_MINUTO;
const HORA: Instante = 60 * MIN;

/// Los `motivo` del hecho `faccion.prestigio`.
mod motivo {
    pub const VICTORIA: i64 = 1;
    pub const CONQUISTA: i64 = 2;
    pub const FUNDACION: i64 = 3;
    pub const ASCENSO: i64 = 4;
    pub const COMERCIO: i64 = 5;
}

fn prestigio() -> ReglasPrestigio {
    comun::reglas().economia.politica.prestigio.clone()
}

/// Los hechos `faccion.prestigio` de ese motivo, en orden: (Facción, ganado, total).
fn ganado(n: &Nucleo, motivo: i64) -> Vec<(i64, i64, i64)> {
    let todos = n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == "faccion.prestigio");
    let dato = |e: &nucleo::EventoNucleo, k: &str| e.datos.iter().find(|(c, _)| *c == k).unwrap_or_else(|| panic!("el hecho no lleva «{k}»: {e:?}")).1;
    todos.filter(|e| dato(e, "motivo") == motivo).map(|e| (dato(e, "faccion"), dato(e, "ganado"), dato(e, "total"))).collect()
}

fn total(n: &Nucleo, faccion: u32) -> i64 {
    n.faccion(faccion).unwrap().prestigio
}

fn codigo(r: &Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

fn admin(id: &str, comando: Comando) -> Peticion {
    Peticion::nueva("admin", id, comando).de_admin()
}

/// Unas reglas donde `ajusta` cambia el bloque `PRESTIGIO`.
fn reglas_con(ajusta: impl FnOnce(&mut ReglasPrestigio)) -> Reglas {
    let mut r = comun::reglas();
    ajusta(&mut Arc::make_mut(&mut r.economia.politica).prestigio);
    r
}

// ───────────────────────────── 1. La victoria contra héroes de otra Facción ─────────────────────────────

/// Una columna de la cuenta en `p`: (columna, héroe).
fn columna(n: &mut Nucleo, cuenta: &str, faccion: u32, p: Punto) -> (u64, u64) {
    let id = n.crear_ejercito(n.ahora(), cuenta, faccion, p, 500, 350_000).unwrap();
    (id, n.ejercito(id).unwrap().columna.lider)
}

/// Le da al héroe una escuadra completa de esa tropa y devuelve su id.
fn dar(n: &mut Nucleo, id: &str, heroe: u64, tropa: Tropa) -> u64 {
    let unidades = militar::Reglas::cargar().tropas.tropa(tropa).unidades;
    let r = n.aplicar_en(n.ahora(), admin(id, Comando::DarEscuadra { heroe, tropa, cantidad: unidades }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

fn atacar(n: &mut Nucleo, t: Instante, actor: &str, id: &str, heroe: u64, objetivo: ObjetivoAtaque) -> Respuesta {
    n.aplicar_en(t, Peticion::nueva(actor, id, Comando::Atacar { heroe, objetivo, declara_guerra: false }))
}

/// Dos columnas de Facciones distintas, a menos de `radioEncuentro` una de otra y en guerra. La de «ana» (F1) lleva espadachines y arqueros; la de «beto» (F2), una milicia: gana ana.
struct Choque {
    n: Nucleo,
    ana: (u64, u64),
    beto: (u64, u64),
}

fn choque_con(n: Nucleo) -> Choque {
    let mut n = n;
    let ana = columna(&mut n, "ana", 1, c(10, 2));
    let beto = columna(&mut n, "beto", 2, Punto::new(c(10, 2).x + 1_000, c(10, 2).y));
    dar(&mut n, "d1", ana.1, Tropa::EspadachinesBronce);
    dar(&mut n, "d2", ana.1, Tropa::ArquerosCompuesto);
    dar(&mut n, "d3", beto.1, Tropa::MiliciaLanceros);
    let r = n.aplicar_en(n.ahora(), admin("guerra", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) }));
    assert!(r.aceptada(), "{r:?}");
    Choque { n, ana, beto }
}

fn choque() -> Choque {
    choque_con(nucleo(40, 5))
}

#[test]
fn ganar_un_combate_de_columnas_a_otra_faccion_suma_la_victoria_a_la_ganadora_con_su_total() {
    let mut x = choque();
    let r = atacar(&mut x.n, 10 * MIN, "ana", "a1", x.ana.1, ObjetivoAtaque::Ejercito(x.beto.0));
    assert!(r.aceptada(), "{r:?}");
    let victoria = prestigio().victoria;
    assert_eq!(ganado(&x.n, motivo::VICTORIA), vec![(1, victoria, victoria)], "gana ana (F1): una vez, con su total");
    assert_eq!((total(&x.n, 1), total(&x.n, 2)), (victoria, 0), "la perdedora no suma ni pierde");
}

#[test]
fn contra_un_heroe_sin_faccion_no_suma_nada() {
    // Como los bandidos, que no tienen ninguna; la misma Facción no se puede atacar, y su filtro lo prueba la unidad de `nucleo::prestigio`.
    let mut x = choque();
    let r = x.n.aplicar_en(x.n.ahora(), admin("sf", Comando::FijarFaccionDeHeroe { heroe: x.beto.1, faccion: None }));
    assert!(r.aceptada(), "{r:?}");
    let r = atacar(&mut x.n, 10 * MIN, "ana", "a1", x.ana.1, ObjetivoAtaque::Ejercito(x.beto.0));
    assert!(r.aceptada(), "{r:?}");
    assert!(ganado(&x.n, motivo::VICTORIA).is_empty() && total(&x.n, 1) == 0);
}

// ───────────────────────────── 2. La batalla de Unity ─────────────────────────────

fn asignacion(revision: u32, tokens: &[u64]) -> Asignacion {
    Asignacion {
        intento: "a1".into(),
        servidor: "ignorado: lo pone el actor".into(),
        revision,
        instancia: Instancia { host: "10.0.0.1".into(), puerto: 7777, protocolo: "udp".into() },
        tokens: tokens.iter().map(|&heroe| Token { heroe, token: format!("t{heroe}"), expira_en: 9_999_999 }).collect(),
    }
}

/// Un mensaje del servidor de batalla.
fn msg(n: &mut Nucleo, t: Instante, id: &str, comando: Comando) -> Respuesta {
    let r = n.aplicar_en(t, Peticion::de_servidor_de_batalla("srv", id, comando));
    assert!(r.aceptada(), "{r:?}");
    r
}

/// Un resultado válido de la batalla `id`: gana el atacante, mueren 5 de cada escuadra y todos los héroes participan.
fn resultado(n: &Nucleo, id: u64) -> ResultadoDeBatalla {
    let b = n.batalla(id).unwrap();
    ResultadoDeBatalla {
        resultado_id: format!("res-{id}"),
        revision: b.ticket.revision,
        intento: "a1".into(),
        inicio: 1_000,
        fin: 61_000,
        ganador: Lado::Atacante,
        razon: Razon::ObjetivosCapturados,
        objetivos: vec![("bandera".into(), Some(Lado::Atacante))],
        por_escuadra: b.escuadras_de().map(|e| ResultadoEscuadra { escuadra: e.id, desplegados: e.efectivos, supervivientes_al_cierre: e.efectivos - 5.min(e.efectivos), muertos: 5.min(e.efectivos) }).collect(),
        por_heroe: b.heroes_que_combaten().map(|h| ResultadoHeroe { heroe: h, participo: true, sobrevivio_al_cierre: true, derrotado: None }).collect(),
        version_servidor: "v1".into(),
        carros_perdidos: vec![],
    }
}

#[test]
fn ganar_una_batalla_de_unity_a_otra_faccion_suma_la_victoria_una_vez() {
    let mut a = comun::arranque(comun::reticula(40, 5));
    a.reglas.batallas_en_unity = true;
    let mut x = choque_con(Nucleo::nuevo(a));
    let r = x.n.aplicar_en(x.n.ahora(), admin("humano", Comando::FijarControlador { heroe: x.ana.1, controlador: Controlador::Humano }));
    assert!(r.aceptada(), "{r:?}");
    let r = atacar(&mut x.n, 10 * MIN, "ana", "a1", x.ana.1, ObjetivoAtaque::Ejercito(x.beto.0));
    assert!(r.aceptada(), "{r:?}");
    let id = x.n.batallas().next().unwrap().id;
    assert!(ganado(&x.n, motivo::VICTORIA).is_empty(), "abrir la batalla no da nada: falta el resultado");
    let revision = x.n.batalla(id).unwrap().ticket.revision;
    msg(&mut x.n, 11 * MIN, "as", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion(revision, &[x.ana.1]) });
    msg(&mut x.n, 11 * MIN, "in", Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision });
    assert_eq!(x.n.batalla(id).unwrap().estado, EstadoBatalla::EnCurso);
    let res = resultado(&x.n, id);
    msg(&mut x.n, 20 * MIN, "res", Comando::AplicarResultado { batalla: id, resultado: res });
    let victoria = prestigio().victoria;
    assert_eq!(ganado(&x.n, motivo::VICTORIA), vec![(1, victoria, victoria)], "gana ana (F1)");
    assert_eq!((total(&x.n, 1), total(&x.n, 2)), (victoria, 0));
}

// ───────────────────────────── 3. El tope de alianzas ─────────────────────────────

/// Propone una alianza de `a` a `b` (la propone el Rey de `a`) y devuelve el código del rechazo o el id de la propuesta.
fn proponer(n: &mut Nucleo, id: &str, cuenta: &str, a: u32, b: u32) -> Result<u64, &'static str> {
    let heroe = n.faccion(a).unwrap().rey.unwrap();
    let r = n.aplicar_en(n.ahora(), Peticion::nueva(cuenta, id, Comando::ProponerRelacion { heroe, a, b, vasallaje: false, tributo: None }));
    if r.aceptada() {
        Ok(r.creadas[0])
    } else {
        Err(codigo(&r))
    }
}

/// El Rey de `b` contesta la propuesta que le hizo otra Facción.
fn aceptar(n: &mut Nucleo, id: &str, cuenta: &str, b: u32, propuesta: u64) -> Result<(), &'static str> {
    let heroe = n.faccion(b).unwrap().rey.unwrap();
    let r = n.aplicar_en(n.ahora(), Peticion::nueva(cuenta, id, Comando::ResponderRelacion { heroe, propuesta, aceptar: true }));
    if r.aceptada() {
        Ok(())
    } else {
        Err(codigo(&r))
    }
}

#[test]
fn el_tope_de_alianzas_crece_con_el_prestigio_y_se_mira_en_las_dos_facciones_al_proponer_y_al_aceptar() {
    // Para tener un prestigio exacto sin pelear tantas veces, una victoria vale lo que una alianza más (`alianzas.unaMasCada`) en estas reglas.
    let cada = prestigio().alianza_cada;
    let mut a = comun::arranque(comun::reticula(40, 5));
    a.reglas = reglas_con(|p| p.victoria = cada);
    let mut x = choque_con(Nucleo::nuevo(a));
    // Tres Facciones más, cada una con su Rey: la 3 («carlos»), la 4 («dani») y la 5 («eva»).
    for (cuenta, faccion, y) in [("carlos", 3, 0), ("dani", 4, 1), ("eva", 5, 3)] {
        columna(&mut x.n, cuenta, faccion, c(30, y));
    }
    let n = &mut x.n;
    // Sin prestigio, una sola alianza.
    let p = proponer(n, "p1", "ana", 1, 3).unwrap();
    aceptar(n, "r1", "carlos", 3, p).unwrap();
    assert_eq!(proponer(n, "p2", "ana", 1, 4), Err(rechazos::DIPLOMACIA_TOPE_ALIANZAS), "la que propone ya está en su tope");
    assert_eq!(proponer(n, "p3", "eva", 5, 3), Err(rechazos::DIPLOMACIA_TOPE_ALIANZAS), "y la otra también cuenta: la 3 está en su tope aunque la 5 esté libre");
    // Con `alianza_cada` de prestigio (una victoria en estas reglas) caben dos. Se abren dos propuestas con una sola alianza activa; la segunda en aceptarse ya no cabe.
    let r = atacar(n, 10 * MIN, "ana", "a1", x.ana.1, ObjetivoAtaque::Ejercito(x.beto.0));
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(total(n, 1), cada, "100 de prestigio: dos alianzas");
    let (p4, p5) = (proponer(n, "p4", "ana", 1, 4).unwrap(), proponer(n, "p5", "ana", 1, 5).unwrap());
    aceptar(n, "r4", "dani", 4, p4).unwrap();
    assert_eq!(aceptar(n, "r5", "eva", 5, p5), Err(rechazos::DIPLOMACIA_TOPE_ALIANZAS), "al aceptar se vuelve a mirar: la 1 ya tiene sus dos");
    // Con el doble (200) entra la tercera.
    let r = atacar(n, 13 * MIN, "ana", "a2", x.ana.1, ObjetivoAtaque::Ejercito(x.beto.0));
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(total(n, 1), 2 * cada, "200 de prestigio: tres alianzas");
    aceptar(n, "r6", "eva", 5, p5).unwrap();
}

// ───────────────────────────── 4. Con plazas: fundar, conquistar, ascender y comerciar ─────────────────────────────

macro_rules! con_mundo {
    () => {
        match real::mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

fn arranque(m: &M, semilla: u64, reglas: Reglas) -> Arranque {
    Arranque { reglas, ajustes: Ajustes::default(), semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

/// Unas reglas sin acoso ni anillo de bandidos: estas pruebas miden otra cosa.
fn sin_bandidos(mut r: Reglas) -> Reglas {
    let b = &mut Arc::make_mut(&mut r.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    r
}

/// Dos plazas, la A de F1 («ana») y la B de F2 («beto»), con las reglas dadas.
struct Esc {
    n: Nucleo,
    a: u64,
    b: u64,
    k: u64,
}

impl Esc {
    fn peticion(&mut self, actor: &str, c: Comando) -> Peticion {
        self.k += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.k), c);
        // Andamiaje (5E): una orden de plaza de quien mandaba en ella antes del sistema 5 se da como administración (`comun::mandaba`).
        if actor == "admin" || comun::mandaba(&self.n, actor, &p.comando) {
            p.de_admin()
        } else {
            p
        }
    }
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        let (t, p) = (self.n.ahora(), self.peticion(actor, c));
        self.n.aplicar_en(t, p)
    }
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    fn avanzar(&mut self, ms: Instante) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    fn pos(&self, plaza: u64) -> Punto {
        self.n.estado().asentamientos[&plaza].posicion
    }
    fn dar(&mut self, plaza: u64, recurso: Recurso, cantidad: i64) {
        let r = self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
    }
    /// Una columna de la cuenta a una unidad de la puerta de la plaza.
    fn columna_en(&mut self, cuenta: &str, faccion: u32, plaza: u64) -> (u64, u64) {
        let p = self.pos(plaza);
        columna(&mut self.n, cuenta, faccion, Punto::new(p.x + 1_000, p.y))
    }
}

/// Funda la A (F1) junto al campamento 0 y la B (F2) junto al `campamento_b`.
fn montar(m: &M, reglas: Reglas, semilla: u64, campamento_b: usize) -> Esc {
    let mut n = Nucleo::nuevo(arranque(m, semilla, reglas));
    let pa = real::sitio_rico(&n, &m.0, 0, 1);
    let a = real::fundar(&mut n, "ana", 1, pa);
    let pb = real::sitio_rico(&n, &m.0, campamento_b, 2);
    let b = real::fundar(&mut n, "beto", 2, pb);
    Esc { n, a, b, k: 0 }
}

/// Como [`montar`], con un Mercado activo en cada plaza (nueve horas de obra).
fn montar_con_mercado(m: &M, semilla: u64) -> Esc {
    let mut e = montar(m, sin_bandidos(comun::reglas()), semilla, 3);
    for plaza in [e.a, e.b] {
        e.dar(plaza, Madera, 350_000);
        e.ok("admin", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
    }
    e.avanzar(9 * HORA);
    for plaza in [e.a, e.b] {
        assert!(e.n.economia(plaza).unwrap().edificios.iter().any(|x| x.tipo == Tipo::Mercado && x.activo()), "el Mercado de as-{plaza} debería estar activo");
    }
    e
}

#[test]
fn fundar_una_plaza_suma_la_fundacion_a_su_faccion() {
    let m = con_mundo!();
    let e = montar(m, comun::reglas(), 7, 0);
    let f = prestigio().fundacion;
    assert_eq!(ganado(&e.n, motivo::FUNDACION), vec![(1, f, f), (2, f, f)], "cada Facción, la suya (también la primera plaza)");
    assert_eq!((total(&e.n, 1), total(&e.n, 2)), (f, f));
}

#[test]
fn conquistar_una_plaza_defendida_suma_la_conquista_y_una_sola_victoria_aunque_ganen_dos_heroes() {
    let m = con_mundo!();
    // Con el anillo de bandidos quieto: la caída de la B suma demanda al campamento 0 y crea uno en el mismo instante (no es lo que mide esta prueba).
    let mut e = montar(m, sin_bandidos(comun::reglas()), 7, 0);
    // Pasa el día de protección con que nacen las plazas (Doc 1.2).
    for plaza in [e.a, e.b] {
        if let Some(h) = e.n.protegida_hasta(plaza) {
            e.n.avanzar_hasta(h.max(e.n.ahora()));
        }
    }
    // Beto defiende la B en persona, con una milicia.
    let campamento = 0;
    let h = e.ok("beto", Comando::CrearHeroe { cuenta: "beto".into(), campamento, controlador: Controlador::Humano, nombre: None, clase: "espada_escudo_mediano".into(), aspecto: Default::default() }).creadas[0];
    let b = e.b;
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(2) });
    e.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(militar::heroe::Residencia::Asentamiento(b as u32)) });
    e.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(b as u32) });
    let milicia = e.ok("admin", Comando::DarEscuadra { heroe: h, tropa: Tropa::MiliciaLanceros, cantidad: militar::Reglas::cargar().tropas.tropa(Tropa::MiliciaLanceros).unidades }).creadas[0];
    let loadout = e.n.heroe(h).unwrap().heroe.loadouts[0].id;
    e.ok("admin", Comando::GuardarLoadout { heroe: h, loadout: Some(loadout), nombre: "Default".into(), escuadras: vec![milicia], activo: Some(true) });
    // El ejército de ana a la puerta, con dos héroes de F1: el segundo se coloca en la columna del primero.
    let ana = e.columna_en("ana", 1, b);
    for tropa in [Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto] {
        dar(&mut e.n, &format!("d-{tropa:?}"), ana.1, tropa);
    }
    let luis = e.ok("luis", Comando::CrearHeroe { cuenta: "luis".into(), campamento, controlador: Controlador::Humano, nombre: None, clase: "espada_escudo_mediano".into(), aspecto: Default::default() }).creadas[0];
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: luis, faccion: Some(1) });
    e.ok("admin", Comando::ColocarHeroe { heroe: luis, ubicacion: Ubicacion::Columna(ana.0) });
    dar(&mut e.n, "d-luis", luis, Tropa::EspadachinesBronce);
    let monedas0 = (e.n.heroe(ana.1).unwrap().heroe.monedas, e.n.heroe(luis).unwrap().heroe.monedas);
    let t = e.n.ahora();
    let r = e.n.aplicar_en(t, Peticion::nueva("ana", "asedio", Comando::Atacar { heroe: ana.1, objetivo: ObjetivoAtaque::Asentamiento(b), declara_guerra: true }));
    assert!(r.aceptada(), "{r:?}");
    // El asedio se resuelve al acabar su convocatoria.
    let fin = e.n.batallas_activas().filter_map(|b| b.convocatoria_hasta).max();
    e.n.avanzar_hasta(fin.expect("hay convocatoria"));
    assert_eq!(e.n.estado().asentamientos[&b].faccion, 1, "la plaza cambió de dueño");
    assert!(e.n.heroe(ana.1).unwrap().heroe.monedas > monedas0.0 && e.n.heroe(luis).unwrap().heroe.monedas > monedas0.1, "los dos héroes ganaron el combate (las monedas de combate los cuentan a los dos)");
    let p = prestigio();
    assert_eq!(ganado(&e.n, motivo::VICTORIA), vec![(1, p.victoria, p.fundacion + p.victoria)], "una vez, aunque ganaron ana y luis");
    assert_eq!(ganado(&e.n, motivo::CONQUISTA), vec![(1, p.conquista, p.fundacion + p.victoria + p.conquista)]);
    assert_eq!(total(&e.n, 2), p.fundacion, "la derrotada no gana ni pierde");
}

#[test]
fn subir_una_plaza_de_nivel_suma_el_prestigio_por_nivel_por_el_nivel_nuevo() {
    let m = con_mundo!();
    let mut n = Nucleo::nuevo(arranque(m, 7, sin_bandidos(comun::reglas())));
    let p = real::sitio_rico(&n, &m.0, 0, 1);
    let plaza = real::fundar(&mut n, "ana", 1, p);
    // Un «jugador» que cada hora pide la subida de nivel; levanta la cantera y la aldea que pide el nivel 2 (como en `tests/asentamientos.rs`).
    for hora in 2..=12 * 24 {
        let t = hora * HORA;
        n.avanzar_hasta(t);
        n.aplicar_en(t, comun::como_dueno(Peticion::nueva("ana", &format!("asc{hora}"), Comando::SolicitarAscenso { asentamiento: plaza })));
        if hora == 2 {
            comun::levantar_cantera(&mut n, "ana", plaza).expect("hay piedra en la zona");
        }
        if hora == 24 {
            comun::levantar_aldea(&mut n, "ana", plaza).expect("hay sitio fértil para la aldea");
        }
    }
    assert_eq!(n.estado().asentamientos[&plaza].nivel(), 2, "sube al nivel 2");
    let p = prestigio();
    let por_nivel = p.ascenso_por_nivel * 2;
    assert_eq!(ganado(&n, motivo::ASCENSO), vec![(1, por_nivel, p.fundacion + por_nivel)], "`ascensoPorNivel` × el nivel nuevo, una vez");
}

/// Hasta que el trueque `id` se cumple (como mucho tres días): las caravanas llevan lo suyo de una plaza a la otra.
fn hasta_cumplido(e: &mut Esc, id: u64) {
    for _ in 0..72 {
        if e.n.acuerdo(id).unwrap().estado == comercio::trueque::EstadoTrueque::Cumplido {
            return;
        }
        e.avanzar(HORA);
    }
    panic!("el trueque no se cumplió en tres días: {:?}", e.n.acuerdo(id));
}

#[test]
fn un_trueque_cumplido_entre_facciones_distintas_suma_el_comercio_a_las_dos() {
    let m = con_mundo!();
    let mut e = montar_con_mercado(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    e.dar(b, Estano, 100_000);
    for (cuenta, plaza) in [("ana", a), ("beto", b)] {
        let id = e.ok(cuenta, Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
        e.ok(cuenta, Comando::AnadirCarro { caravana: id, tipo: TipoCarro::Basico });
        e.ok(cuenta, Comando::AnadirAnimal { caravana: id, carro: 0, animal: Animal::Buey });
    }
    let acuerdo = e.ok("ana", Comando::ProponerTrueque { a, b, lineas_a: vec![(Cobre, 60_000)], lineas_b: vec![(Estano, 30_000)] }).creadas[0];
    e.ok("beto", Comando::AceptarTrueque { acuerdo });
    assert!(ganado(&e.n, motivo::COMERCIO).is_empty(), "proponer y aceptar no es cumplir");
    hasta_cumplido(&mut e, acuerdo);
    let p = prestigio();
    assert_eq!(ganado(&e.n, motivo::COMERCIO), vec![(1, p.comercio, p.fundacion + p.comercio), (2, p.comercio, p.fundacion + p.comercio)], "a cada una de las dos");
}

#[test]
fn tomar_una_orden_de_mercado_de_otra_faccion_suma_el_comercio_a_las_dos_y_de_la_propia_no_suma() {
    let m = con_mundo!();
    let mut e = montar_con_mercado(m, 7);
    let a = e.a;
    e.dar(a, Madera, 100_000);
    let orden = e.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Madera, cantidad: 50_000, precio: Some(2_000_000) }).creadas[0];
    // Una columna de ana, de la misma Facción de la plaza, toma un poco: nada.
    let propia = e.columna_en("ana", 1, a).0;
    e.ok("admin", Comando::DarSuministro { ejercito: propia, recurso: Oro, cantidad: 300_000 });
    e.ok("ana", Comando::ComerciarEnPlaza { ejercito: propia, orden, cantidad: 10_000 });
    assert!(ganado(&e.n, motivo::COMERCIO).is_empty(), "de la propia Facción no suma");
    // Una columna de beto (F2) toma otro poco: suma a la que toma y a la dueña de la plaza.
    let ajena = e.columna_en("beto", 2, a).0;
    e.ok("admin", Comando::DarSuministro { ejercito: ajena, recurso: Oro, cantidad: 300_000 });
    e.ok("beto", Comando::ComerciarEnPlaza { ejercito: ajena, orden, cantidad: 20_000 });
    let p = prestigio();
    assert_eq!(ganado(&e.n, motivo::COMERCIO), vec![(2, p.comercio, p.fundacion + p.comercio), (1, p.comercio, p.fundacion + p.comercio)]);
}

// ───────────────────────────── 5. La Maravilla y las estatuas ─────────────────────────────

#[test]
fn la_maravilla_pide_el_umbral_de_prestigio_de_la_faccion_en_lugar_de_la_era() {
    let m = con_mundo!();
    let maravilla = |e: &mut Esc| {
        let a = e.a;
        let r = e.ord("ana", Comando::AnadirEdificio { asentamiento: a, tipo: Tipo::Maravilla });
        codigo(&r)
    };
    // F1 tiene la `fundacion` de su primera plaza, mucho menos que el umbral de la Maravilla.
    let mut e = montar(m, comun::reglas(), 7, 0);
    assert!(total(&e.n, 1) < prestigio().maravilla);
    assert_eq!(maravilla(&mut e), rechazos::OBRA_PRESTIGIO_INSUFICIENTE);
    // Con el umbral alcanzado (aquí, el de la fundación) el prestigio ya no la frena: la frena el nivel 5 de plaza que también pide.
    let fundacion = prestigio().fundacion;
    let mut e = montar(m, reglas_con(|p| p.maravilla = fundacion), 7, 0);
    assert_eq!(maravilla(&mut e), rechazos::OBRA_NIVEL_INSUFICIENTE);
}

#[test]
fn la_estatua_solo_se_levanta_en_la_capital_y_tantas_como_umbrales_alcanzados() {
    let m = con_mundo!();
    let estatua = |e: &mut Esc, plaza: u64| {
        let r = e.ord("ana", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Estatua });
        if r.aceptada() {
            "aceptada"
        } else {
            codigo(&r)
        }
    };
    // Sin ningún umbral alcanzado (la `fundacion` de la primera plaza no llega al primero): no hay estatua.
    let mut e = montar(m, comun::reglas(), 7, 0);
    assert!(total(&e.n, 1) < prestigio().estatuas[0]);
    let a = e.a;
    e.dar(a, Piedra, 100_000);
    assert_eq!(estatua(&mut e, a), rechazos::OBRA_PRESTIGIO_INSUFICIENTE);
    // Fundar vale aquí el primer umbral: F1 tiene uno. La segunda plaza de F1 (nivel de Facción 3 para que quepa) no es la capital: la más antigua lo es.
    let primero = prestigio().estatuas[0];
    let mut e = montar(m, reglas_con(|p| p.fundacion = primero), 7, 0);
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 3 });
    let (a, p2) = (e.a, real::sitio_rico(&e.n, &m.0, 0, 1));
    let a2 = real::fundar(&mut e.n, "ana2", 1, p2);
    assert_eq!(e.n.capital_de(1), Some(a), "la capital es la plaza más antigua");
    // Dos fundaciones valen 2 × el primer umbral: con los umbrales por defecto (200, 500, 1.000) sigue siendo uno solo.
    assert_eq!(total(&e.n, 1), 2 * primero);
    assert_eq!(prestigio().estatuas.iter().filter(|&&u| total(&e.n, 1) >= u).count(), 1);
    e.dar(a, Piedra, 100_000);
    e.dar(a2, Piedra, 100_000);
    assert_eq!(estatua(&mut e, a2), rechazos::OBRA_SOLO_EN_CAPITAL, "fuera de la capital, ni con prestigio");
    assert_eq!(estatua(&mut e, a), "aceptada", "la primera, en la capital");
    assert_eq!(estatua(&mut e, a), rechazos::OBRA_PRESTIGIO_INSUFICIENTE, "la que está en obra ya cuenta contra el tope");
}
