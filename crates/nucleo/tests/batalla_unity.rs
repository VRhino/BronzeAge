//! La batalla de Unity de extremo a extremo por el contrato (4H, T4): un servidor de batalla de prueba y los jugadores hablan con el servidor WebSocket real y el actor del núcleo.
//!
//! Lo que se prueba: el ámbito `BATALLA` de los combatientes, el ticket completo y congelado (con la plaza de un asedio, sus edificios y sus murallas en pie), su publicación a todos los servidores
//! de batalla conectados y la primera asignación que gana, los acuses de cada mensaje, el token propio de cada combatiente, las incorporaciones, el resultado aplicado con sus deltas, una batalla
//! de bandidos sin humanos y sin iniciador, el asedio visible (`batalla_id`, `lado_para_unirse`) y el informe de inspección de una columna, una caravana y una plaza con sus recintos.
//!
//! El asedio tiene 30 segundos de convocatoria: las pruebas aceleran el reloj de mundo con la orden `controlar_reloj` de la administración hasta que el ticket llega a los servidores y lo devuelven
//! a su ritmo. Lo que se cuenta de lo que llega a una sesión se lee con «leer hasta» (un mensaje que cumple una condición): el reloj de mundo reparte `Delta` por su cuenta. Van sobre el artefacto
//! de mundo real (`generador/salida/mundo/mundo-*.bin`) y se omiten si no está.

mod comun;

use asentamiento::recintos::Recinto;
use asentamiento::Cargo;
use comercio::caravana::{Animal, TipoCarro};
use base::{Instante, Punto, MS_POR_MINUTO};
use comun::sesion::*;
use contrato::v1::{self as pb, entidad::Cuerpo, mensaje_de_cliente as mc, mensaje_de_servidor as ms, mensaje_de_servidor_de_batalla as de_srv, mensaje_para_servidor_de_batalla as para, orden, TipoCliente};
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::proyeccion::ids;
use nucleo::{Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Resultado, Respuesta};
use rutas::Rutas;
use servidor::prueba::{hola, ClientePrueba};
use std::sync::{Arc, OnceLock};

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;

type M = (Arc<Mundo>, Arc<Rutas>);

fn mundo() -> Option<&'static M> {
    static M: OnceLock<Option<M>> = OnceLock::new();
    M.get_or_init(|| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generador/salida/mundo");
        let f = std::fs::read_dir(dir).ok()?.flatten().find(|e| e.file_name().to_string_lossy().starts_with("mundo-"))?;
        let m = Mundo::cargar(&std::fs::read(f.path()).ok()?, &OpcionesValidacion::default()).expect("artefacto válido");
        let r = Arc::new(Rutas::desde_mundo(&m));
        Some((Arc::new(m), r))
    })
    .as_ref()
}

macro_rules! con_mundo {
    () => {
        match mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

// ───────────────────────────── El mundo antes de ponerlo detrás del servidor ─────────────────────────────

/// Las reglas por defecto con servidores de batalla, almacenes de 100.000 por recurso (un anillo entero cuesta más de lo que cabe en el de partida) y sin bandidos salvo que se pidan.
fn reglas(bandidos: bool) -> Reglas {
    let mut economia = asentamiento_reglas();
    Arc::make_mut(&mut economia.datos).capacidad_inicial = 100_000_000;
    let mut r = Reglas { economia, batallas_en_unity: true, ..Reglas::default() };
    if !bandidos {
        let b = &mut Arc::make_mut(&mut r.economia.combate).bandidos;
        (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    }
    r
}

fn asentamiento_reglas() -> nucleo::ReglasEconomia {
    nucleo::ReglasEconomia::cargar()
}

fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

fn campamento_mas_cercano(m: &Mundo, p: Punto) -> u32 {
    (0..m.campamentos().len()).min_by_key(|&i| campamento(m, i).dist2(p)).unwrap() as u32
}

/// Un sitio verde con mucho bosque a 10 u o más del campamento `i`: una plaza que no cae en ruinas por falta de madera.
fn sitio_rico(n: &Nucleo, m: &Mundo, i: usize, faccion: u32) -> Punto {
    let c = campamento(m, i);
    for r in (0..8).map(|k| 50_000 + k * 10_000) {
        for g in 0..360 {
            let a = (g as f64).to_radians();
            let p = Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64);
            if n.viabilidad(p, faccion).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6) {
                return p;
            }
        }
    }
    panic!("sin sitio rico cerca del campamento {i}");
}

fn completa(tropa: Tropa) -> i64 {
    militar::Reglas::cargar().tropas.tropa(tropa).unidades
}

fn fundar(n: &mut Nucleo, nombre: &str, dueno: &str, faccion: u32, p: Punto) -> u64 {
    let e = n.crear_ejercito(0, dueno, faccion, p, 500, 100_000).unwrap();
    assert!(n.aplicar_en(0, Peticion::nueva("admin", &format!("c-{nombre}"), Comando::DarCaravanaFundacion { ejercito: e }).de_admin()).aceptada());
    let r = n.aplicar_en(0, Peticion::nueva(dueno, &format!("f-{nombre}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

/// Dos plazas de Facciones distintas (A de F1, B de F2), sin auto-construcción; la B a nivel 2, rica y con una empalizada completa.
struct Esc {
    n: Nucleo,
    a: u64,
    b: u64,
    cuenta: u64,
}

impl Esc {
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.cuenta += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.cuenta), c);
        let p = if actor == "admin" { p.de_admin() } else { p };
        let t = self.n.ahora();
        self.n.aplicar_en(t, p)
    }

    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
        r
    }

    fn hasta(&mut self, t: Instante) {
        self.n.avanzar_hasta(t);
    }

    fn pos(&self, plaza: u64) -> Punto {
        self.n.estado().asentamientos[&plaza].posicion
    }

    fn recintos_de_b(&self) -> &[Recinto] {
        &self.n.economia(self.b).unwrap().recintos
    }

    /// Avanza hasta que el recinto `i` de la plaza B está cerrado.
    fn terminar(&mut self, i: usize) {
        for _ in 0..5_000 {
            let r = &self.recintos_de_b()[i];
            if r.completo() && r.mejora.is_none() {
                return;
            }
            let ahora = self.n.ahora();
            let t = r.siguiente_celda_en.filter(|&s| s > ahora).unwrap_or(ahora + MIN);
            self.hasta(t);
        }
        panic!("el recinto {i} no termina");
    }

    /// Un héroe humano de la cuenta, ciudadano de F2, que reside en la plaza B y está dentro, con su loadout activo de una escuadra de milicia.
    fn residente(&mut self, m: &Mundo, cuenta: &str) -> u64 {
        let campamento = campamento_mas_cercano(m, self.pos(self.b));
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento, controlador: Controlador::Humano }).creadas[0];
        let b = self.b as u32;
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(2) });
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(b)) });
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(b) });
        let escuadra = self.ok("admin", Comando::DarEscuadra { heroe: h, tropa: Tropa::MiliciaLanceros, cantidad: completa(Tropa::MiliciaLanceros) }).creadas[0];
        let loadout = self.n.heroe(h).unwrap().heroe.loadouts[0].id;
        self.ok("admin", Comando::GuardarLoadout { heroe: h, loadout: Some(loadout), nombre: "Default".into(), escuadras: vec![escuadra], perks: vec![], activo: Some(true) });
        h
    }

    /// Un ejército de administración de `cuenta`/`faccion` a 2 u de la plaza B con esas tropas y su héroe humano (o bot): (columna, héroe).
    fn ejercito(&mut self, cuenta: &str, faccion: u32, tropas: &[Tropa], controlador: Controlador) -> (u64, u64) {
        let p = self.pos(self.b);
        let t = self.n.ahora();
        let col = self.n.crear_ejercito(t, cuenta, faccion, Punto::new(p.x + 2_000, p.y), 500, 350_000).unwrap();
        let h = self.n.ejercito(col).unwrap().columna.lider;
        for &tropa in tropas {
            self.ok("admin", Comando::DarEscuadra { heroe: h, tropa, cantidad: completa(tropa) });
        }
        self.ok("admin", Comando::FijarControlador { heroe: h, controlador });
        (col, h)
    }
}

/// Las dos plazas con la B a nivel 2 y una empalizada completa, a las 25 horas. Los fundadores son cuentas aparte (`fundador_a`, `fundador_b`): los jugadores de las pruebas no tienen más héroe
/// que el que se les da.
fn montar(m: &M, bandidos: bool) -> Esc {
    let a = Arranque { reglas: reglas(bandidos), ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) };
    let mut n = Nucleo::nuevo(a);
    let (pa, pb) = (sitio_rico(&n, &m.0, 0, 1), sitio_rico(&n, &m.0, 3, 2));
    let (ia, ib) = (fundar(&mut n, "a", "fundador_a", 1, pa), fundar(&mut n, "b", "fundador_b", 2, pb));
    let mut e = Esc { n, a: ia, b: ib, cuenta: 0 };
    for plaza in [ia, ib] {
        e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
        // De todo: sin auto-construcción nadie produce, y una plaza sin comida ni mantenimiento caería en ruinas a las pocas decenas de horas.
        for r in asentamiento::recurso::TODOS {
            e.ok("admin", Comando::DarRecursos { asentamiento: plaza, recurso: r, cantidad: 5_000_000 });
        }
        e.ok("admin", Comando::AnadirEdificio { asentamiento: plaza, tipo: asentamiento::catalogo::Tipo::Mercado });
    }
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: ib, nivel: 2 });
    e.hasta(HORA + 30_000);
    e.ok("fundador_b", Comando::ComprometerRecinto { asentamiento: ib, cargo: Cargo::MaestroObras, nivel: 1 });
    e.terminar(0);
    e.hasta(25 * HORA + 30_000);
    e
}

// ───────────────────────────── Lo que llega ─────────────────────────────

fn ambito(a: &Option<pb::Ambito>) -> String {
    let a = a.as_ref().expect("un ámbito");
    format!("{}:{}", pb::AmbitoTipo::try_from(a.tipo).unwrap().as_str_name().trim_start_matches("AMBITO_"), a.id)
}

/// Lee mensajes hasta que llega uno que cumple `f`; devuelve todo lo leído, el último incluido.
async fn leer_hasta(c: &mut ClientePrueba, f: impl Fn(&ms::Cuerpo) -> bool) -> Vec<ms::Cuerpo> {
    let mut leidos = Vec::new();
    loop {
        let m = c.esperar().await.cuerpo.expect("un mensaje con cuerpo");
        let fin = f(&m);
        leidos.push(m);
        if fin {
            return leidos;
        }
    }
}

/// Las batallas de un ámbito `BATALLA:<uuid>` (o de cualquier otro) que traen estos mensajes, en el orden en que llegaron.
fn batallas_de<'a>(mensajes: &'a [ms::Cuerpo], ambito_buscado: &str) -> Vec<&'a pb::Batalla> {
    let entidades = |ambito_de: &Option<pb::Ambito>, es: Vec<&'a pb::Entidad>| if ambito(ambito_de) == ambito_buscado { es } else { Vec::new() };
    mensajes
        .iter()
        .flat_map(|m| match m {
            ms::Cuerpo::Foto(f) => entidades(&f.ambito, f.entidades.iter().collect()),
            ms::Cuerpo::Delta(d) => entidades(&d.ambito, d.cambios.iter().filter_map(|c| c.datos.as_ref()).collect()),
            _ => Vec::new(),
        })
        .filter_map(|e| if let Some(Cuerpo::Batalla(b)) = e.cuerpo.as_ref() { Some(b) } else { None })
        .collect()
}

/// Lee hasta que llega una versión de la batalla del ámbito dado que cumple `f` y la devuelve.
async fn batalla_hasta(c: &mut ClientePrueba, ambito_buscado: &str, f: impl Fn(&pb::Batalla) -> bool) -> pb::Batalla {
    loop {
        let m = leer_hasta(c, |m| batallas_de(std::slice::from_ref(m), ambito_buscado).iter().any(|b| f(b))).await;
        if let Some(b) = batallas_de(&m, ambito_buscado).into_iter().find(|b| f(b)) {
            return b.clone();
        }
    }
}

/// Lo que ya estaba en camino hacia la sesión: el actor contesta a una orden que rechaza (aquí, por su forma, antes del candado de batalla) después de repartir todo lo anterior, y el servidor entrega en orden.
async fn pendiente(c: &mut ClientePrueba) -> Vec<ms::Cuerpo> {
    let (previos, r) = dar_todo(c, "barrera", orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: "basura".into() })).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenRechazada as i32, "{r:?}");
    previos
}

fn aceptada(r: &pb::ResultadoOrden) {
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
}

fn lado_de(l: i32) -> pb::LadoId {
    pb::LadoId::try_from(l).unwrap()
}

// ───────────────────────────── El servidor de batalla de prueba ─────────────────────────────

struct Svc {
    c: ClientePrueba,
}

impl Svc {
    async fn entrar(w: &Servicio, nombre: &str) -> Svc {
        let (c, b) = ClientePrueba::entrar(w.srv.direccion, hola(&format!("svc:{nombre}"), TipoCliente::ClienteServidorBatalla)).await;
        assert!(b.ambitos.is_empty() && b.heroe_id.is_none(), "un servidor de batalla no tiene ámbitos");
        Svc { c }
    }

    /// El siguiente mensaje de batalla (los avisos del reloj no cuentan).
    async fn siguiente(&mut self) -> para::Cuerpo {
        loop {
            match self.c.esperar().await.cuerpo {
                Some(ms::Cuerpo::Batalla(m)) => return m.cuerpo.expect("un mensaje de batalla con cuerpo"),
                Some(ms::Cuerpo::Aviso(_)) => {}
                o => panic!("a un servidor de batalla solo le llegan mensajes de batalla: {o:?}"),
            }
        }
    }

    async fn ticket(&mut self) -> pb::BattleTicket {
        match self.siguiente().await {
            para::Cuerpo::Ticket(t) => t,
            o => panic!("se esperaba un ticket, llegó {o:?}"),
        }
    }

    async fn incorporacion(&mut self) -> pb::IncorporacionBatalla {
        match self.siguiente().await {
            para::Cuerpo::Incorporacion(i) => i,
            o => panic!("se esperaba una incorporación, llegó {o:?}"),
        }
    }

    async fn acuse(&mut self) -> pb::AcuseBatalla {
        match self.siguiente().await {
            para::Cuerpo::Acuse(a) => a,
            o => panic!("se esperaba un acuse, llegó {o:?}"),
        }
    }

    async fn enviar(&mut self, cuerpo: de_srv::Cuerpo) {
        self.c.enviar(mc::Cuerpo::Batalla(pb::MensajeDeServidorDeBatalla { cuerpo: Some(cuerpo) })).await;
    }

    /// Envía un mensaje y devuelve su acuse.
    async fn mandar(&mut self, cuerpo: de_srv::Cuerpo) -> pb::AcuseBatalla {
        self.enviar(cuerpo).await;
        self.acuse().await
    }

    /// No llega ningún mensaje de batalla en un rato (los avisos del reloj no cuentan).
    async fn sin_nada(&mut self) {
        let fin = tokio::time::Instant::now() + std::time::Duration::from_millis(300);
        while let Ok(m) = tokio::time::timeout_at(fin, self.c.recibir()).await {
            assert!(matches!(m.and_then(|m| m.cuerpo), Some(ms::Cuerpo::Aviso(_))), "no debía llegar ningún mensaje de batalla");
        }
    }
}

fn aceptado(a: &pb::AcuseBatalla, mensaje: &str) {
    assert_eq!((a.mensaje.as_str(), a.resultado), (mensaje, pb::Resultado::OrdenAceptada as i32), "{a:?}");
}

fn rechazado<'a>(a: &'a pb::AcuseBatalla, mensaje: &str) -> &'a str {
    assert_eq!((a.mensaje.as_str(), a.resultado), (mensaje, pb::Resultado::OrdenRechazada as i32), "{a:?}");
    a.rechazo.as_ref().expect("un rechazo trae su código").codigo.as_str()
}

fn asignacion(uuid: &str, intento: &str, revision: u32, humanos: &[&str]) -> de_srv::Cuerpo {
    de_srv::Cuerpo::Asignacion(pb::BattleServerAssignment {
        schema_version: 1,
        battle_id: uuid.into(),
        ticket_revision: revision,
        intento_asignacion_id: intento.into(),
        instancia: Some(pb::InstanciaDeBatalla { host: format!("unity-{intento}.test"), puerto: 7777, protocolo: "udp".into() }),
        tokens_participante: humanos.iter().map(|h| pb::TokenParticipante { heroe_id: (*h).into(), token: format!("tok-{h}"), expira_en_ms: 4_000_000_000_000 }).collect(),
        ..Default::default()
    })
}

fn inicio(uuid: &str, intento: &str) -> de_srv::Cuerpo {
    de_srv::Cuerpo::Inicio(pb::InicioBatalla { schema_version: 1, battle_id: uuid.into(), ticket_revision: 0, intento_asignacion_id: intento.into() })
}

/// El resultado de una partida en la que cada escuadra pierde un soldado, con todo lo del ticket y las incorporaciones.
fn resultado_de(ticket: &pb::BattleTicket, incorporaciones: &[pb::IncorporacionBatalla], intento: &str, ganador: pb::LadoId) -> de_srv::Cuerpo {
    let participantes: Vec<&pb::BattleParticipantSnapshot> = [ticket.atacante.as_ref().unwrap(), ticket.defensor.as_ref().unwrap()]
        .into_iter()
        .flat_map(|b| &b.participantes)
        .chain(incorporaciones.iter().filter_map(|i| i.participante.as_ref()))
        .collect();
    let sin_heroe = [ticket.atacante.as_ref().unwrap(), ticket.defensor.as_ref().unwrap()].into_iter().flat_map(|b| &b.escuadras_sin_heroe);
    let escuadras = participantes.iter().flat_map(|p| &p.escuadras).chain(sin_heroe);
    de_srv::Cuerpo::Resultado(pb::BattleResult {
        schema_version: 1,
        battle_id: ticket.battle_id.clone(),
        result_id: format!("res-{intento}"),
        ticket_revision: ticket.ticket_revision,
        intento_asignacion_id: intento.into(),
        inicio_ms: 1_000,
        fin_ms: 61_000,
        ganador: ganador as i32,
        razon: pb::RazonVictoria::ObjetivosCapturados as i32,
        objetivos: vec![pb::ObjetivoResultado { objetivo_id: "bandera".into(), capturado_por: Some(ganador as i32) }],
        por_escuadra: escuadras.map(|e| pb::ResultadoEscuadra { squad_id: e.squad_id.clone(), desplegados: e.efectivos_autorizados, supervivientes_al_cierre: e.efectivos_autorizados - 1, muertos: 1, xp_ganada: 2 }).collect(),
        por_heroe: participantes.iter().map(|p| pb::ResultadoHeroe { heroe_id: p.heroe_id.clone(), participo: true, sobrevivio_al_cierre: true, xp_ganada: 3, derrotado: None, botin: None }).collect(),
        version_servidor: "v1".into(),
        carros_perdidos: vec![],
    })
}

async fn fijar_factor(admin: &mut ClientePrueba, peticion: &str, factor_milesimas: i64) {
    let r = dar_todo(admin, peticion, orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: "fijar_factor".into(), factor_milesimas: Some(factor_milesimas), hasta_ms: None })).await.1;
    aceptada(&r);
}

// ───────────────────────────── El asedio listo para asignar ─────────────────────────────

/// Un asedio de F1 a la plaza B de F2 (con su empalizada) con el ticket ya publicado: `ana` (F1) lo abre con su ejército, `beto` (F2, residente) y `carla` (F2, residente) esperan dentro, `nora` (F3, neutral)
/// y `fede` (F1) esperan fuera, a 2 u. Se unen durante la convocatoria `beto` (defiende desde dentro) y `nora` (ataca desde fuera); `carla` y `fede` quedan sin unirse. Los servidores `a` y `b` están conectados
/// y han recibido el ticket.
struct Asedio {
    w: Servicio,
    uuid: String,
    plaza: u64,
    ana: ClientePrueba,
    beto: ClientePrueba,
    carla: ClientePrueba,
    nora: ClientePrueba,
    fede: ClientePrueba,
    admin: ClientePrueba,
    a: Svc,
    b: Svc,
    ticket: pb::BattleTicket,
    /// Los ids del contrato de los héroes: `ana`, `beto`, `carla`, `nora`, `fede`.
    h: [String; 5],
    /// Todo lo que llegó a cada jugador (en el orden de `h`) desde que se conectó hasta que el ticket llegó a los servidores.
    vistas: Vec<Vec<ms::Cuerpo>>,
}

async fn asedio_listo(m: &'static M) -> Asedio {
    let mut e = montar(m, false);
    let b = e.b;
    let beto = e.residente(&m.0, "beto");
    let carla = e.residente(&m.0, "carla");
    let ana = e.ejercito("ana", 1, &[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto], Controlador::Humano);
    let nora = e.ejercito("nora", 3, &[Tropa::MiliciaLanceros], Controlador::Humano);
    let fede = e.ejercito("fede", 1, &[Tropa::MiliciaLanceros], Controlador::Humano);
    let h = [ana.1, beto, carla, nora.1, fede.1].map(ids::heroe);
    let w = arrancar_con(e.n).await;
    let (mut admin, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let conectar = |cuenta: &'static str| w.conectar(hola(&format!("ok:{cuenta}"), TipoCliente::ClienteUnity));
    let ((mut ana_c, ..), (mut beto_c, ..), (mut carla_c, ..), (mut nora_c, ..), (mut fede_c, ..)) = (conectar("ana").await, conectar("beto").await, conectar("carla").await, conectar("nora").await, conectar("fede").await);
    let (mut a, mut sb) = (Svc::entrar(&w, "srva").await, Svc::entrar(&w, "srvb").await);

    // ana asedia la plaza B declarando la guerra: se abre la batalla, que le llega en su ámbito propio.
    let atacar = orden::Cuerpo::Atacar(pb::OrdenAtacar {
        heroe_id: h[0].clone(),
        objetivo: Some(pb::orden_atacar::Objetivo { tipo: Some(pb::orden_atacar::objetivo::Tipo::AsentamientoId(ids::asentamiento(b))) }),
        declara_guerra: Some(true),
    });
    let (previos, r) = dar_todo(&mut ana_c, "asedio", atacar).await;
    aceptada(&r);
    let uuid = previos.iter().find_map(|m| if let ms::Cuerpo::Foto(f) = m { Some(ambito(&f.ambito)).filter(|a| a.starts_with("BATALLA:")) } else { None }).expect("la foto de su batalla").trim_start_matches("BATALLA:").to_owned();

    // Durante la convocatoria se unen beto, desde dentro, y nora, desde fuera.
    let unirse = |heroe: &str| orden::Cuerpo::UnirseABatalla(pb::OrdenUnirseABatalla { heroe_id: heroe.into(), battle_id: uuid.clone(), lado: None });
    let (de_beto, r) = dar_todo(&mut beto_c, "u-beto", unirse(&h[1])).await;
    aceptada(&r);
    let (de_nora, r) = dar_todo(&mut nora_c, "u-nora", unirse(&h[3])).await;
    aceptada(&r);

    // Se acelera el reloj de mundo hasta que acaba la convocatoria y el ticket llega a los dos servidores; después, a su ritmo.
    fijar_factor(&mut admin, "r1", 60_000).await;
    let (ticket, ticket_b) = (a.ticket().await, sb.ticket().await);
    fijar_factor(&mut admin, "r2", 1_000).await;
    assert_eq!(ticket, ticket_b, "el ticket se publica a todos los servidores conectados, el mismo");
    let mut vistas: Vec<Vec<ms::Cuerpo>> = vec![previos, de_beto, Vec::new(), de_nora, Vec::new()];
    for (i, c) in [&mut ana_c, &mut beto_c, &mut carla_c, &mut nora_c, &mut fede_c].into_iter().enumerate() {
        vistas[i].extend(pendiente(c).await);
    }
    Asedio { w, uuid, plaza: b, ana: ana_c, beto: beto_c, carla: carla_c, nora: nora_c, fede: fede_c, admin, a, b: sb, ticket, h, vistas }
}

#[tokio::test]
async fn el_ticket_del_asedio_es_completo_congelado_y_va_a_todos_los_servidores_de_batalla() {
    let m = con_mundo!();
    let mut x = asedio_listo(m).await;
    let t = x.ticket.clone();
    let [h_ana, h_beto, _, h_nora, _] = x.h.clone();

    // Lo que dice el ticket.
    assert_eq!((t.schema_version, t.battle_id.as_str(), t.ticket_revision), (1, x.uuid.as_str(), 0));
    assert!(t.game_id.contains(m.0.hash()), "el gameId sale del hash del mundo y la semilla");
    let Some(pb::contexto_estrategico::Tipo::Asedio(c)) = t.contexto_estrategico.as_ref().and_then(|c| c.tipo.clone()) else { panic!("un asedio") };
    assert_eq!(c.asentamiento_id, ids::asentamiento(x.plaza));
    let (atacante, defensor) = (t.atacante.as_ref().unwrap(), t.defensor.as_ref().unwrap());
    assert_eq!((atacante.faccion_id.as_deref(), atacante.capacidad_maxima, defensor.faccion_id.as_deref(), defensor.capacidad_maxima), (Some("f-1"), 15, Some("f-2"), 15));
    assert_eq!(atacante.participantes.iter().map(|p| p.heroe_id.clone()).collect::<Vec<_>>(), [h_ana.clone(), h_nora.clone()], "ana, que abrió, y nora, que se unió desde fuera en la convocatoria");
    assert_eq!(defensor.participantes.iter().map(|p| p.heroe_id.clone()).collect::<Vec<_>>(), std::slice::from_ref(&h_beto), "beto, que se unió desde dentro; carla no se unió y no entra de oficio");
    assert!(defensor.escuadras_sin_heroe.is_empty());
    let beto = &defensor.participantes[0];
    let hs = beto.heroe.as_ref().unwrap();
    assert_eq!((beto.faccion_id.as_deref(), beto.es_bot, hs.nivel, hs.class_definition_id.as_str(), hs.femenino, hs.casillas_inventario_libres), (Some("f-2"), false, 1, "default", false, 72));
    assert!(hs.display_name.starts_with("Héroe ") && hs.atributos_efectivos.is_some() && hs.equipamiento.is_some() && hs.avatar.is_some(), "valores por defecto de nombre, clase, género y avatar");
    let milicia = &beto.escuadras;
    assert_eq!(milicia.len(), 1);
    assert_eq!((milicia[0].tropa_id.as_str(), milicia[0].efectivos_autorizados as i64, milicia[0].nivel, milicia[0].experiencia, milicia[0].moral, milicia[0].heroe_id.as_deref()), ("milicia_lanceros", completa(Tropa::MiliciaLanceros), 1, 0, 100, Some(h_beto.as_str())));
    let reglas = t.reglas.as_ref().unwrap();
    assert_eq!((reglas.duracion_maxima_segundos, reglas.version_catalogo_heroe.as_str(), reglas.version_catalogo_objetos.as_str()), (1_800, "conquest-heroes-1", "conquest-items-1"));

    // La plaza como estaba al abrir: sus edificios internos y su empalizada con las celdas en pie.
    let Some(pb::battle_ticket::Mapa::MapaAsentamiento(p)) = t.mapa.clone() else { panic!("un asedio lleva el mapa de la plaza") };
    assert_eq!((p.settlement_id.as_str(), p.layout_version, p.unidades_por_celda), (ids::asentamiento(x.plaza).as_str(), trazado::LAYOUT_VERSION, trazado::CELDA_UL as u32));
    assert!(!p.edificios.is_empty() && p.edificios.iter().all(|e| e.edificio_id.starts_with(&format!("ed-{}-", x.plaza)) && e.ancho > 0 && e.alto > 0 && e.posicion.is_some()));
    assert!(p.edificios.iter().any(|e| e.estado == pb::EstadoEdificio::EdificioActivo as i32 && e.nivel_interno == Some(1)), "{:?}", p.edificios.iter().map(|e| (&e.tipo, e.estado)).collect::<Vec<_>>());
    assert_eq!(p.recintos.len(), 1, "la empalizada");
    let r = &p.recintos[0];
    assert_eq!((r.recinto_id.starts_with(&format!("recinto-{}-", x.plaza)), r.nivel), (true, 1));
    assert!(!r.celdas.is_empty() && r.celdas.iter().any(|c| c.clase == pb::ClaseCelda::CeldaPuerta as i32));

    // Congelado: la plaza cambia después (se añade una vivienda) y un servidor que llega tarde recibe el mismo ticket.
    let (_, r) = dar_todo(&mut x.admin, "viv", orden::Cuerpo::AnadirEdificioManualmente(pb::OrdenAnadirEdificioManualmente { asentamiento_id: ids::asentamiento(x.plaza), cargo: "gobernador".into(), tipo: "vivienda".into() })).await;
    aceptada(&r);
    let mut c = Svc::entrar(&x.w, "srvc").await;
    assert_eq!(c.ticket().await, t, "el ticket está congelado: lo construido después no lo cambia");

    // Los combatientes lo ven en su ámbito, con el ticket y su bando.
    let ambito_b = format!("BATALLA:{}", x.uuid);
    for (i, lado) in [(0, pb::LadoId::Atacante), (1, pb::LadoId::Defensor), (3, pb::LadoId::Atacante)] {
        let ultima = batallas_de(&x.vistas[i], &ambito_b).into_iter().last().unwrap_or_else(|| panic!("el jugador {i} no recibió su batalla"));
        assert_eq!((ultima.lado_propio, ultima.ticket.is_some(), ultima.lado_para_unirse), (Some(lado as i32), true, None), "jugador {i}");
        assert_eq!(ultima.ticket.as_ref().unwrap().ticket_revision, 0);
    }
    // carla (de F2, dentro) y fede (de F1, fuera) no combaten: ven la batalla resumida por su Facción, sin ticket ni bando propio, con el bando al que podrían unirse.
    for (i, faccion, puede) in [(2, "f-2", pb::LadoId::Defensor), (4, "f-1", pb::LadoId::Atacante)] {
        assert!(batallas_de(&x.vistas[i], &ambito_b).is_empty(), "no combate: no tiene su ámbito");
        let resumen = batallas_de(&x.vistas[i], &format!("FACCION:{faccion}"));
        let primera = resumen.first().expect("la batalla llega a la Facción de los dos bandos");
        assert_eq!((primera.id.as_str(), primera.ticket.is_none(), primera.lado_propio, primera.lado_para_unirse), (x.uuid.as_str(), true, None, Some(puede as i32)), "jugador {i}");
    }
    // El asedio se ve desde la plaza: su ficha lleva la batalla que se juega contra ella (a quien está dentro y a su Facción).
    let ficha = |msgs: &[ms::Cuerpo], ambito_buscado: &str| {
        msgs.iter()
            .filter_map(|m| if let ms::Cuerpo::Delta(d) = m { Some(d).filter(|d| ambito(&d.ambito) == ambito_buscado) } else { None })
            // Un cambio parcial de la plaza solo trae los campos de su máscara: `batalla_id` cuenta si la máscara lo nombra (o si la plaza viene entera).
            .flat_map(|d| d.cambios.iter().filter(|c| c.reemplaza || c.campos.iter().any(|campo| campo == "batalla_id")).filter_map(|c| c.datos.as_ref()))
            .filter_map(|e| if let Some(Cuerpo::Asentamiento(p)) = e.cuerpo.as_ref() { Some(p.batalla_id.clone()) } else { None })
            .next_back()
    };
    assert_eq!(ficha(&x.vistas[2], &format!("ASENTAMIENTO:{}", ids::asentamiento(x.plaza))), Some(Some(x.uuid.clone())), "carla, dentro de la plaza asediada");
    assert_eq!(ficha(&x.vistas[2], "FACCION:f-2"), Some(Some(x.uuid.clone())), "y la Facción dueña");
    x.w.parar().await;
}

fn tokens(uuid: &str, intento: &str, humanos: &[&str]) -> de_srv::Cuerpo {
    de_srv::Cuerpo::Tokens(pb::TokensBatalla {
        schema_version: 1,
        battle_id: uuid.into(),
        intento_asignacion_id: intento.into(),
        tokens_participante: humanos.iter().map(|h| pb::TokenParticipante { heroe_id: (*h).into(), token: format!("tok-{h}"), expira_en_ms: 4_000_000_000_000 }).collect(),
        ..Default::default()
    })
}

fn unirse(uuid: &str, heroe: &str) -> orden::Cuerpo {
    orden::Cuerpo::UnirseABatalla(pb::OrdenUnirseABatalla { heroe_id: heroe.into(), battle_id: uuid.into(), lado: None })
}

/// El token que dice tener un combatiente para conectar con el servidor de batalla: el token, el host, el puerto y cuándo caduca.
fn token_propio(b: &pb::Batalla) -> Option<(&str, &str, u32, i64)> {
    b.asignacion_propia.as_ref().map(|a| {
        let i = a.instancia.as_ref().expect("la instancia");
        (a.token.as_str(), i.host.as_str(), i.puerto, a.expira_en_ms)
    })
}

#[tokio::test]
async fn dos_servidores_compiten_por_la_asignacion_y_cada_combatiente_recibe_su_token_con_acuses_incorporaciones_y_resultado() {
    let m = con_mundo!();
    let Asedio { w, uuid, plaza, mut ana, mut beto, mut carla, mut nora, mut fede, a, b, ticket, h, .. } = asedio_listo(m).await;
    let [h_ana, h_beto, h_carla, h_nora, h_fede] = h;
    let ambito_b = format!("BATALLA:{uuid}");
    let humanos = [h_ana.as_str(), h_beto.as_str(), h_nora.as_str()];

    // Los dos servidores piden la batalla a la vez: gana la primera asignación y la otra recibe `batalla.ya_asignada`.
    let (mut sa, mut sb) = (a, b);
    sa.enviar(asignacion(&uuid, "intento-a", 0, &humanos)).await;
    sb.enviar(asignacion(&uuid, "intento-b", 0, &humanos)).await;
    let (acuse_a, acuse_b) = (sa.acuse().await, sb.acuse().await);
    let gana_a = acuse_a.resultado == pb::Resultado::OrdenAceptada as i32;
    let (mut ganador, mut perdedor, intento, acuse_g, acuse_p) = if gana_a { (sa, sb, "intento-a", acuse_a, acuse_b) } else { (sb, sa, "intento-b", acuse_b, acuse_a) };
    aceptado(&acuse_g, "asignacion");
    assert_eq!(acuse_g.battle_id, uuid);
    assert_eq!(rechazado(&acuse_p, "asignacion"), "batalla.ya_asignada");

    // Cada combatiente recibe su propio token, el de su héroe y ningún otro, con la instancia del servidor ganador.
    for (c, heroe) in [(&mut ana, &h_ana), (&mut beto, &h_beto), (&mut nora, &h_nora)] {
        let b = batalla_hasta(c, &ambito_b, |b| b.estado == pb::EstadoBatalla::BatallaAsignada as i32).await;
        let (token, host, puerto, expira) = token_propio(&b).expect("su token");
        assert_eq!((token, host, puerto, expira), (format!("tok-{heroe}").as_str(), format!("unity-{intento}.test").as_str(), 7777, 4_000_000_000_000));
    }
    // Quien no combate no recibe ninguno: carla y fede ven la batalla asignada desde su Facción, sin ticket ni token.
    for (c, faccion) in [(&mut carla, "f-2"), (&mut fede, "f-1")] {
        let b = batalla_hasta(c, &format!("FACCION:{faccion}"), |b| b.estado == pb::EstadoBatalla::BatallaAsignada as i32).await;
        assert!(b.asignacion_propia.is_none() && b.ticket.is_none() && b.lado_para_unirse.is_some(), "{b:?}");
    }

    // Después de la asignación se unen carla (defensora, dentro) y fede (atacante, desde fuera): su incorporación va solo al servidor que tiene la batalla, que responde con sus tokens.
    aceptada(&dar_todo(&mut carla, "u-carla", unirse(&uuid, &h_carla)).await.1);
    aceptada(&dar_todo(&mut fede, "u-fede", unirse(&uuid, &h_fede)).await.1);
    let (i1, i2) = (ganador.incorporacion().await, ganador.incorporacion().await);
    assert_eq!((i1.secuencia, lado_de(i1.lado), i1.participante.as_ref().unwrap().heroe_id.as_str()), (1, pb::LadoId::Defensor, h_carla.as_str()));
    assert_eq!((i2.secuencia, lado_de(i2.lado), i2.participante.as_ref().unwrap().heroe_id.as_str()), (2, pb::LadoId::Atacante, h_fede.as_str()));
    assert_eq!(i1.battle_id, uuid);
    perdedor.sin_nada().await;
    let dos = [h_carla.as_str(), h_fede.as_str()];
    assert_eq!(rechazado(&perdedor.mandar(tokens(&uuid, intento, &dos)).await, "tokens"), "batalla.invalida", "no es el servidor de la asignación");
    aceptado(&ganador.mandar(tokens(&uuid, intento, &dos)).await, "tokens");
    assert_eq!(ganador.mandar(tokens(&uuid, intento, &dos)).await.resultado, pb::Resultado::OrdenDuplicada as i32, "repetir los mismos tokens");
    for (c, heroe, lado) in [(&mut carla, &h_carla, pb::LadoId::Defensor), (&mut fede, &h_fede, pb::LadoId::Atacante)] {
        let b = batalla_hasta(c, &ambito_b, |b| b.asignacion_propia.is_some()).await;
        assert_eq!((token_propio(&b).unwrap().0, b.lado_propio), (format!("tok-{heroe}").as_str(), Some(lado as i32)));
        assert_eq!(b.incorporaciones.iter().map(|i| i.secuencia).collect::<Vec<_>>(), [1, 2], "ya combaten: ven el ticket y las incorporaciones");
    }

    // El inicio: solo el servidor de la asignación; repetirlo es un duplicado.
    assert_eq!(rechazado(&perdedor.mandar(inicio(&uuid, "intento-p")).await, "inicio"), "batalla.invalida");
    aceptado(&ganador.mandar(inicio(&uuid, intento)).await, "inicio");
    assert_eq!(ganador.mandar(inicio(&uuid, intento)).await.resultado, pb::Resultado::OrdenDuplicada as i32);
    for c in [&mut ana, &mut beto] {
        batalla_hasta(c, &ambito_b, |b| b.estado == pb::EstadoBatalla::BatallaEnCurso as i32).await;
    }

    // El resultado se aplica: gana el defensor, los atacantes quedan heridos, sus columnas vuelven al mundo y la plaza deja de estar en asedio.
    let resultado = resultado_de(&ticket, &[i1, i2], intento, pb::LadoId::Defensor);
    aceptado(&ganador.mandar(resultado.clone()).await, "resultado");
    assert_eq!(ganador.mandar(resultado).await.resultado, pb::Resultado::OrdenDuplicada as i32, "el mismo resultado otra vez");
    // A ana le llegan, en el orden que sea, la batalla aplicada (en su ámbito), su héroe herido y su columna libre otra vez (en el del héroe).
    let (mut aplicada, mut herido, mut libre) = (None, false, false);
    let heroe_ana = format!("HEROE:{h_ana}");
    while aplicada.is_none() || !herido || !libre {
        let m = ana.esperar().await.cuerpo.expect("un mensaje con cuerpo");
        aplicada = aplicada.or_else(|| batallas_de(std::slice::from_ref(&m), &ambito_b).into_iter().find(|b| b.estado == pb::EstadoBatalla::BatallaAplicada as i32).cloned());
        if let ms::Cuerpo::Delta(d) = &m {
            for e in d.cambios.iter().filter(|_| ambito(&d.ambito) == heroe_ana).filter_map(|c| c.datos.as_ref()) {
                match &e.cuerpo {
                    Some(Cuerpo::Heroe(x)) => herido |= x.herido_hasta_ms.is_some(),
                    Some(Cuerpo::Ejercito(x)) => libre |= !x.en_batalla && x.batalla_id.is_none(),
                    _ => {}
                }
            }
        }
    }
    let aplicada = aplicada.unwrap();
    let r = aplicada.resultado.as_ref().expect("el resultado aplicado");
    assert_eq!((lado_de(r.ganador), r.por_heroe.len(), r.result_id.as_str(), r.carros_perdidos.is_empty()), (pb::LadoId::Defensor, 5, format!("res-{intento}").as_str(), true));
    assert!(aplicada.cerrada_en_ms.is_some() && aplicada.aplicada_en_ms.is_some());
    let plaza_id = ids::asentamiento(plaza);
    let sin_asedio = leer_hasta(&mut beto, |m| matches!(m, ms::Cuerpo::Delta(d) if ambito(&d.ambito) == format!("ASENTAMIENTO:{plaza_id}") && d.cambios.iter().filter_map(|c| c.datos.as_ref()).any(|e| matches!(&e.cuerpo, Some(Cuerpo::Asentamiento(p)) if p.batalla_id.is_none())))).await;
    assert!(!sin_asedio.is_empty(), "la plaza ya no está en asedio");
    // Foto + deltas = foto final: una sesión nueva de ana ve la misma batalla que ella fue reconstruyendo.
    let ultima = batallas_de(&pendiente(&mut ana).await, &ambito_b).into_iter().last().cloned().unwrap_or(aplicada);
    let (_, _, nueva) = w.conectar(hola("ok:ana", TipoCliente::ClienteUnity)).await;
    assert_eq!(batallas_de(&nueva, &ambito_b).last().copied(), Some(&ultima));
    let n = w.parar().await;
    let id = n.id_de_batalla(&uuid).unwrap();
    assert!(n.batalla_sobre(Ubicacion::Asentamiento(plaza as u32)).is_none() && n.batalla(id).unwrap().estado == combate::ciclo::EstadoBatalla::Aplicada);
    assert!(n.batalla(id).unwrap().ticket.plaza.as_ref().is_some_and(|p| !p.edificios.is_empty() && p.recintos.len() == 1));
    la_foto_vuelve_igual(&n, m);
}

// ───────────────────────────── Bandidos: una batalla sin humanos y sin iniciador ─────────────────────────────

impl Esc {
    /// Una caravana de la plaza con un carro con buey, fuera del reparto automático.
    fn caravana(&mut self, actor: &str, plaza: u64) -> u64 {
        let id = self.ok(actor, Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
        self.ok(actor, Comando::ReservarCaravana { caravana: id, reservada: true });
        self.ok(actor, Comando::AnadirCarro { caravana: id, tipo: TipoCarro::Basico });
        self.ok(actor, Comando::AnadirAnimal { caravana: id, carro: 0, animal: Animal::Buey });
        id
    }
}

/// El primer instante (en pasos de un segundo) en que el plan está a ≤ `radioAtaqueCaravana` de `donde`.
fn llegada_al_radio(plan: &base::Plan, donde: Punto, desde: Instante) -> Instante {
    let radio = combate::Reglas::cargar().bandidos.radio_ataque_caravana;
    (desde..).step_by(1_000).find(|&t| plan.posicion_en(t).unwrap().dist(donde) <= radio).expect("el plan cruza el radio")
}

/// Un trozo de tierra libre cerca de A donde marchar 60 u hacia el este sin acercarse a ningún campamento de bandidos de los que ya hay (los de las plazas y los de las vetas también acosan:
/// la prueba solo quiere el suyo). Devuelve su inicio y su fin.
fn tramo_sin_bandidos(e: &Esc) -> (Punto, Punto) {
    let ya: Vec<Punto> = e.n.campamentos_de_bandidos().map(|b| b.posicion).collect();
    let libre = |p: Punto| e.n.viabilidad(p, 1).is_some_and(|v| v.motivo.is_none()) && ya.iter().all(|b| b.dist(p) > 45_000);
    let centro = e.pos(e.a);
    for r in (80..=240).step_by(20).map(|k| k * 1_000) {
        for g in (0..360).step_by(15) {
            let a = (g as f64).to_radians();
            let p = Punto::new(centro.x + (r as f64 * a.cos()) as i64, centro.y + (r as f64 * a.sin()) as i64);
            let (medio, fin) = (Punto::new(p.x + 30_000, p.y), Punto::new(p.x + 60_000, p.y));
            if [p, medio, fin].into_iter().all(libre) {
                return (p, fin);
            }
        }
    }
    panic!("sin un tramo libre de bandidos cerca de A");
}

/// La administración (`ADMIN`, 4H T3e) ve la batalla entera —el ticket, las incorporaciones y la asignación que ganó, con su instancia— y **nunca** un token: ni el de la asignación, ni el de las
/// incorporaciones, ni el propio de nadie. Los jugadores, al revés: su token y no la asignación.
#[tokio::test]
async fn la_administracion_ve_la_batalla_entera_con_su_asignacion_y_nunca_un_token() {
    use prost::Message;
    let m = con_mundo!();
    let Asedio { w, uuid, mut ana, mut carla, mut admin, a, h, .. } = asedio_listo(m).await;
    let [h_ana, h_beto, h_carla, h_nora, _] = &h;
    let mut sa = a;
    aceptado(&sa.mandar(asignacion(&uuid, "intento-a", 0, &[h_ana, h_beto, h_nora])).await, "asignacion");
    aceptada(&dar_todo(&mut carla, "u-carla", unirse(&uuid, h_carla)).await.1);
    assert_eq!(sa.incorporacion().await.participante.unwrap().heroe_id, *h_carla);
    aceptado(&sa.mandar(tokens(&uuid, "intento-a", &[h_carla])).await, "tokens");
    let suya = batalla_hasta(&mut ana, &format!("BATALLA:{uuid}"), |b| b.asignacion_propia.is_some()).await;
    assert!(suya.asignacion.is_none(), "un jugador ve su token, nunca la asignación");

    // Todo lo que le llegó a la administración desde la asignación, y la foto de una sesión de administración nueva.
    let mut recibido = pendiente(&mut admin).await;
    let ultima = batallas_de(&recibido, "ADMIN:").last().copied().cloned().expect("la batalla llega a la administración");
    let (_, _, foto) = w.conectar(hola("ok:admin2", TipoCliente::ClienteAdminWeb)).await;
    assert_eq!(batallas_de(&foto, "ADMIN:"), [&ultima], "foto + deltas = foto nueva");
    recibido.extend(foto);
    let asignada = ultima.asignacion.as_ref().expect("con la asignación que ganó");
    assert_eq!((asignada.intento_asignacion_id.as_str(), asignada.instancia.as_ref().unwrap().host.as_str(), asignada.tokens_participante.len()), ("intento-a", "unity-intento-a.test", 0));
    assert!(ultima.ticket.is_some() && ultima.incorporaciones.len() == 1 && ultima.asignacion_propia.is_none() && ultima.lado_propio.is_none() && ultima.lado_para_unirse.is_none(), "{ultima:?}");
    let bytes: Vec<u8> = recibido.into_iter().flat_map(|c| pb::MensajeDeServidor { cuerpo: Some(c), ..Default::default() }.encode_to_vec()).collect();
    assert!(!bytes.windows(4).any(|w| w == b"tok-"), "ningún token de batalla viaja a la administración");
    w.parar().await;
}

#[tokio::test]
async fn los_bandidos_que_acosan_a_una_columna_de_bots_juegan_una_batalla_sin_humanos_y_sin_iniciador_que_el_servidor_toma_sin_tokens() {
    let m = con_mundo!();
    let mut e = montar(m, true);
    let caravana = e.caravana("fundador_a", e.a);
    // La columna de una cuenta bot, con su caravana enganchada, marcha por un tramo libre de bandidos y un campamento de nivel 3 la espera al 40 % del camino.
    let (origen, fin_del_tramo) = tramo_sin_bandidos(&e);
    let t = e.n.ahora();
    let col = e.n.crear_ejercito(t, "ana", 1, origen, 500, 350_000).unwrap();
    let h_ana = e.n.ejercito(col).unwrap().columna.lider;
    e.ok("admin", Comando::DarEscuadra { heroe: h_ana, tropa: Tropa::EspadachinesBronce, cantidad: completa(Tropa::EspadachinesBronce) });
    e.ok("admin", Comando::AdjuntarCaravana { caravana, ejercito: col });
    e.ok("ana", Comando::MarcharA { ejercito: col, destino: fin_del_tramo });
    let plan = e.n.ejercito(col).unwrap().plan.clone();
    let (t0, fin) = (plan.inicio().unwrap(), plan.fin().unwrap());
    let donde = plan.posicion_en(t0 + (fin - t0) * 4 / 10).unwrap();
    let campamento = e.ok("admin", Comando::ColocarBandidos { posicion: donde, nivel: 3 }).creadas[0];
    let (ahora, llegada) = (e.n.ahora(), llegada_al_radio(&plan, donde, e.n.ahora()));
    assert!(e.n.batallas().next().is_none());
    let w = arrancar_con(e.n).await;
    let (mut admin, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let mut svc = Svc::entrar(&w, "srv").await;

    // El mundo corre hasta que la columna cruza el radio del campamento: sin tirada con números, se abre una batalla y el servidor recibe su ticket.
    let factor = ((llegada - ahora + 5_000) * 1_000 / 1_500).max(1_000);
    fijar_factor(&mut admin, "r1", factor).await;
    let ticket = svc.ticket().await;
    fijar_factor(&mut admin, "r2", 1_000).await;
    let Some(pb::contexto_estrategico::Tipo::Caravana(c)) = ticket.contexto_estrategico.as_ref().and_then(|c| c.tipo.clone()) else { panic!("una batalla de caravana") };
    let id_caravana = ids::caravana(caravana);
    assert_eq!((c.caravana_id.as_str(), c.modalidad), (id_caravana.as_str(), pb::ModalidadCaravana::ModalidadDisputa as i32), "el acoso nunca ocurre dentro de una zona");
    assert_eq!(c.caravanas.len(), 1);
    assert_eq!((c.caravanas[0].caravana_id.as_str(), c.caravanas[0].carros.iter().map(|k| k.carro_id.clone()).collect::<Vec<_>>()), (id_caravana.as_str(), vec![format!("{id_caravana}:0")]), "los carros se nombran con su caravana y su posición");
    let (atacante, defensor) = (ticket.atacante.as_ref().unwrap(), ticket.defensor.as_ref().unwrap());
    assert_eq!((atacante.faccion_id.clone(), atacante.participantes.len(), atacante.escuadras_sin_heroe.len()), (None, 0, 1), "los bandidos no son de nadie ni llevan héroes");
    let bandidos = &atacante.escuadras_sin_heroe[0];
    assert_eq!((bandidos.tropa_id.as_str(), bandidos.heroe_id.clone(), bandidos.nivel, bandidos.moral, bandidos.efectivos_autorizados as i64), ("milicia_lanceros", None, 1, 100, combate::Reglas::cargar().bandidos.niveles[2].unidades));
    assert_eq!((defensor.faccion_id.as_deref(), defensor.participantes.iter().map(|p| (p.heroe_id.clone(), p.es_bot)).collect::<Vec<_>>()), (Some("f-1"), vec![(ids::heroe(h_ana), true)]));
    assert!(matches!(ticket.mapa, Some(pb::battle_ticket::Mapa::MapaReferencia(_))), "fuera de un asedio el terreno es el artefacto de mundo");

    // Una batalla sin humanos se asigna sin tokens, se inicia y se resuelve igual; el servidor recibe un acuse de cada mensaje.
    let uuid = ticket.battle_id.clone();
    aceptado(&svc.mandar(asignacion(&uuid, "intento", 0, &[])).await, "asignacion");
    aceptado(&svc.mandar(inicio(&uuid, "intento")).await, "inicio");
    aceptado(&svc.mandar(resultado_de(&ticket, &[], "intento", pb::LadoId::Defensor)).await, "resultado");
    let n = w.parar().await;
    let b = n.batallas().next().expect("la batalla sigue en el estado, cerrada");
    assert_eq!((b.iniciada_por, b.estado, b.ticket.atacante.faccion, b.bloqueo.campamento), (0, combate::ciclo::EstadoBatalla::Aplicada, None, Some(campamento as u32)), "nadie la inició");
    assert!(n.caravana(caravana).is_some(), "ganaron los defensores: la caravana sigue");
    assert!(n.campamento_de_bandidos(campamento).is_some(), "y el campamento, donde estaba");
}

// ───────────────────────────── Informe de inspección ─────────────────────────────

#[tokio::test]
async fn el_informe_de_inspeccion_de_una_columna_una_caravana_y_una_plaza_con_sus_recintos_llega_solo_a_quien_inspecciono() {
    let m = con_mundo!();
    let mut e = montar(m, false);
    let b = e.b;
    let ana = e.ejercito("ana", 1, &[Tropa::EspadachinesBronce], Controlador::Humano);
    let gus = e.ejercito("gus", 2, &[Tropa::MiliciaLanceros], Controlador::Humano);
    let caravana = e.caravana("fundador_b", b);
    e.ok("admin", Comando::AdjuntarCaravana { caravana, ejercito: gus.0 });
    let pos_b = e.pos(b);
    let recintos = e.recintos_de_b().to_vec();
    let w = arrancar_con(e.n).await;
    let (mut ana_c, ..) = w.conectar(hola("ok:ana", TipoCliente::ClienteUnity)).await;
    let (mut gus_c, ..) = w.conectar(hola("ok:gus", TipoCliente::ClienteUnity)).await;
    let h_ana = ids::heroe(ana.1);
    let informes = |previos: &[pb::Delta]| -> Vec<pb::InformeInspeccion> {
        previos
            .iter()
            .filter(|d| ambito(&d.ambito) == format!("HEROE:{h_ana}"))
            .flat_map(|d| d.cambios.iter().filter_map(|c| c.datos.as_ref()))
            .filter_map(|e| if let Some(Cuerpo::InformeInspeccion(i)) = e.cuerpo.as_ref() { Some(i.clone()) } else { None })
            .collect()
    };
    let mirar = |objetivo: pb::orden_inspeccionar::objetivo::Tipo| orden::Cuerpo::Inspeccionar(pb::OrdenInspeccionar { heroe_id: h_ana.clone(), objetivo: Some(pb::orden_inspeccionar::Objetivo { tipo: Some(objetivo) }) });
    use pb::orden_inspeccionar::objetivo::Tipo as Objetivo;

    // La plaza de F2: su empalizada completa, con el anillo entero y las celdas levantadas.
    let (previos, r) = dar(&mut ana_c, "i-plaza", mirar(Objetivo::AsentamientoId(ids::asentamiento(b)))).await;
    aceptada(&r);
    let [plaza] = <[_; 1]>::try_from(informes(&previos)).expect("el informe de la plaza");
    assert_eq!((plaza.id.as_str(), plaza.objetivo_id.as_str(), plaza.heroe_id.as_str(), plaza.expira_en_ms - plaza.visto_en_ms), (format!("insp-{}-{}", ana.1, ids::asentamiento(b)).as_str(), ids::asentamiento(b).as_str(), h_ana.as_str(), 10 * MIN));
    assert_eq!(plaza.donde.as_ref().map(|p| (p.x, p.y)), Some((pos_b.x, pos_b.y)));
    let Some(pb::informe_inspeccion::Contenido::Plaza(p)) = plaza.contenido.clone() else { panic!("la defensa de una plaza") };
    assert_eq!((p.faccion_id.as_str(), p.heroes_ids.is_empty(), p.guarnicion.is_empty(), p.recintos.len()), ("f-2", true, true, 1));
    let anillo = &recintos[0];
    assert_eq!((p.recintos[0].nivel, p.recintos[0].avance as usize, p.recintos[0].celdas.len()), (1, anillo.levantadas(), anillo.celdas.len()));
    assert!(p.recintos[0].celdas.iter().any(|c| c.clase == pb::ClaseCelda::CeldaPuerta as i32));

    // La columna de gus y su caravana, de la plaza B (enganchada: va escoltada).
    let (previos, r) = dar(&mut ana_c, "i-columna", mirar(Objetivo::EjercitoId(ids::ejercito(gus.0)))).await;
    aceptada(&r);
    let todos = informes(&previos);
    assert_eq!(todos.len(), 2, "el de la plaza sigue vigente y llega con el nuevo");
    let Some(pb::informe_inspeccion::Contenido::Columna(c)) = todos.iter().find(|i| i.objetivo_id == ids::ejercito(gus.0)).and_then(|i| i.contenido.clone()) else { panic!("una columna") };
    assert_eq!((c.faccion_id.as_deref(), c.heroes_ids, c.escuadras.len()), (Some("f-2"), vec![ids::heroe(gus.1)], 1));
    assert_eq!((c.escuadras[0].tropa_id.as_str(), c.escuadras[0].cantidad as i64, c.escuadras[0].heroe_id.as_str()), ("milicia_lanceros", completa(Tropa::MiliciaLanceros), ids::heroe(gus.1).as_str()));
    let (previos, r) = dar(&mut ana_c, "i-caravana", mirar(Objetivo::CaravanaId(ids::caravana(caravana)))).await;
    aceptada(&r);
    let todos = informes(&previos);
    assert_eq!(todos.len(), 3);
    let Some(pb::informe_inspeccion::Contenido::Caravana(k)) = todos.iter().find(|i| i.objetivo_id == ids::caravana(caravana)).and_then(|i| i.contenido.clone()) else { panic!("una caravana") };
    assert!(k.escoltada && k.recursos.is_empty(), "si va escoltada y qué lleva, nunca cuánto");

    // Foto + deltas = foto final, y solo lo ve quien lo hizo: una sesión nueva de ana ve los tres en su ámbito; gus, ninguno.
    let (_, _, nueva) = w.conectar(hola("ok:ana", TipoCliente::ClienteUnity)).await;
    let foto = nueva.iter().find_map(|m| if let ms::Cuerpo::Foto(f) = m { Some(f).filter(|f| ambito(&f.ambito) == format!("HEROE:{h_ana}")) } else { None }).expect("la foto de su héroe");
    let en_foto: Vec<&pb::InformeInspeccion> = foto.entidades.iter().filter_map(|e| if let Some(Cuerpo::InformeInspeccion(i)) = e.cuerpo.as_ref() { Some(i) } else { None }).collect();
    assert_eq!(en_foto.len(), 3);
    assert!(en_foto.iter().all(|i| todos.contains(i)), "los mismos informes que fueron llegando por deltas");
    let de_gus = pendiente(&mut gus_c).await;
    assert!(de_gus.iter().all(|m| !matches!(m, ms::Cuerpo::Delta(d) if d.cambios.iter().filter_map(|c| c.datos.as_ref()).any(|e| matches!(&e.cuerpo, Some(Cuerpo::InformeInspeccion(_)))))), "es de quien lo hizo");
    let n = w.parar().await;
    assert!(n.informes_de_inspeccion(ana.1).iter().any(|i| matches!(&i.contenido, militar::inspeccion::Contenido::Plaza { recintos, .. } if recintos.len() == 1)));
    la_foto_vuelve_igual(&n, m);
}

#[tokio::test]
async fn quien_no_llega_a_conectarse_reabre_la_batalla_con_un_ticket_nuevo_para_todos_los_servidores_y_pierde_su_ambito() {
    let m = con_mundo!();
    let Asedio { w, uuid, mut beto, a, b, ticket, h, .. } = asedio_listo(m).await;
    let [h_ana, h_beto, _, h_nora, _] = h;
    let ambito_b = format!("BATALLA:{uuid}");
    let humanos = [h_ana.as_str(), h_beto.as_str(), h_nora.as_str()];
    let (mut sa, mut sb) = (a, b);
    aceptado(&sa.mandar(asignacion(&uuid, "intento-a", 0, &humanos)).await, "asignacion");

    // beto no llega a conectarse: sale sin castigo antes de empezar, la revisión sube y la batalla vuelve a esperar servidor, con un ticket nuevo que reciben los dos servidores.
    let salida = de_srv::Cuerpo::Salida(pb::SalidaBatalla {
        schema_version: 1,
        battle_id: uuid.clone(),
        ticket_revision: 0,
        intento_asignacion_id: "intento-a".into(),
        heroe_id: h_beto.clone(),
        motivo: pb::MotivoSalida::SalidaNoConectado as i32,
        derrotado: false,
        escuadras: vec![],
    });
    sa.enviar(salida).await;
    let (nuevo_a, nuevo_b) = (sa.ticket().await, sb.ticket().await);
    aceptado(&sa.acuse().await, "salida");
    assert_eq!(nuevo_a, nuevo_b);
    assert_eq!(nuevo_a.ticket_revision, 1);
    assert!(nuevo_a.defensor.unwrap().participantes.is_empty(), "su participación se quita del ticket y no entra nadie en su lugar");
    assert_ne!(nuevo_a.ticket_revision, ticket.ticket_revision);
    let revocado = leer_hasta(&mut beto, |m| matches!(m, ms::Cuerpo::Revocado(r) if ambito(&r.ambito) == ambito_b)).await;
    assert!(matches!(revocado.last(), Some(ms::Cuerpo::Revocado(r)) if r.motivo == "sin_derecho"), "ya no combate: pierde el ámbito de la batalla");

    // El segundo servidor puede tomarla ahora (la revisión vigente es la 1) y el primero ya no la tiene.
    aceptado(&sb.mandar(asignacion(&uuid, "intento-b", 1, &[h_ana.as_str(), h_nora.as_str()])).await, "asignacion");
    assert_eq!(rechazado(&sa.mandar(inicio(&uuid, "intento-a")).await, "inicio"), "batalla.invalida");
    w.parar().await;
}

/// La foto de un núcleo con un ticket de asedio congelado (plaza, edificios, recintos) y con un informe de inspección de una plaza vuelve igual; la de antes del cambio, no.
fn la_foto_vuelve_igual(n: &Nucleo, m: &M) {
    let bytes = n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN30");
    let vuelta = nucleo::Estado::deserializar(&bytes, &reglas(false), &m.0.hash().to_string()).expect("la foto vuelve");
    assert_eq!(vuelta.serializar(), bytes);
    let viejo = [b"BAN20".as_slice(), &bytes[5..]].concat();
    let Err(err) = nucleo::Estado::deserializar(&viejo, &reglas(false), &m.0.hash().to_string()) else { panic!("BAN20 no debe admitirse") };
    assert!(err.to_string().contains("BAN20") && err.to_string().contains("plaza"), "{err}");
}
