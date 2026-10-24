//! Reanudación y causalidad de la sesión (4H T6) por el servidor WebSocket real y el actor del núcleo, sobre el artefacto de mundo real (se omiten si no está).
//!
//! **La propiedad «foto + deltas ≡ foto final»**, en los siete ámbitos a la vez (`HEROE`, `FACCION`, `ASENTAMIENTO`, `CAMPAMENTO`, `VISTA`, `ADMIN` y `BATALLA`) con mundos y órdenes
//! aleatorios de semilla fija: cada jugador acumula lo que le llega en su almacén ([`Almacen`]) y, cada pocos pasos, (1) lo acumulado es lo que daría una foto nueva; (2) reanudar con lo que
//! conserva (todos sus ámbitos a la vez) lo deja igual y no le repite ni un hecho; y (3) reanudar con lo que tenía en la comprobación anterior —como un cliente que perdió todo lo de en medio:
//! un hueco— también lo deja igual. Además: la retención del registro (la versión más vieja que aún se reanuda por deltas, y una más, foto), `version_esperada`, la idempotencia de una orden
//! repetida (también tras reconectar) y que un hecho no se cuente dos veces al reanudar ni se pierda cuando el ámbito que lo oye se acaba de abrir.
//!
//! El reloj de mundo está **parado** y solo anda cuando la administración lo avanza, así que lo que llega a cada sesión depende solo de las órdenes; lo que ya iba hacia una sesión se recoge con
//! una orden rechazada (sale después de todo lo anterior).

mod comun;

use base::{Instante, Mil, Punto, Rng, MS_POR_MINUTO};
use contrato::almacen::{clave_de_ambito, Almacen};
use comun::real::{self, en_tierra, fundar, sitio_rico, M};
use comun::sesion::*;
use contrato::v1::{
    self as pb, mensaje_de_cliente as mc, mensaje_de_servidor as ms, mensaje_de_servidor_de_batalla as de_srv, mensaje_para_servidor_de_batalla as para, orden, TipoCliente,
};
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::proyeccion::ids;
use nucleo::{AccionReloj, Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Resultado};
use prost_types::{value::Kind, Struct, Value};
use servidor::prueba::{hola, ClientePrueba};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

const U: Mil = 1_000;

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

// ───────────────────────────── El mundo ─────────────────────────────

/// Las reglas por defecto con servidores de batalla, almacenes grandes y sin bandidos.
fn reglas() -> Reglas {
    let mut economia = nucleo::ReglasEconomia::cargar();
    Arc::make_mut(&mut economia.datos).capacidad_inicial = 100_000_000;
    let mut r = Reglas { economia, batallas_en_unity: true, ..Reglas::default() };
    let b = &mut Arc::make_mut(&mut r.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    r
}

/// Dos plazas, A de F1 y B de F2, en campamentos que elige la semilla; `ana` (F1) reside en A y está dentro, `beto` (F2) en B; `dora`, sin Facción, en el campamento de mercenarios más cercano a A;
/// `nora` (F3) y `fede` (F1), columnas de campo a 2 u de B. El reloj, parado.
struct Esc {
    n: Nucleo,
    a: u64,
    b: u64,
    campamento: u32,
    ana: u64,
    escuadra_de_ana: u64,
    beto: u64,
    dora: u64,
    nora: u64,
    fede: u64,
    peticiones: u64,
}

impl Esc {
    fn ok(&mut self, actor: &str, c: Comando) -> nucleo::Respuesta {
        self.peticiones += 1;
        let p = Peticion::nueva(actor, &format!("prep{}", self.peticiones), c);
        let t = self.n.ahora();
        let r = self.n.aplicar_en(t, if actor == "admin" { p.de_admin() } else { p });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
        r
    }

    /// Un héroe humano de `cuenta`, ciudadano de `faccion`, que reside en `plaza` y está dentro, con una escuadra de milicia en su loadout activo: (héroe, escuadra).
    fn residente(&mut self, cuenta: &str, faccion: u32, plaza: u64, nacer_en: u32) -> (u64, u64) {
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: nacer_en, controlador: Controlador::Humano }).creadas[0];
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(faccion) });
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
        let escuadra = self.ok("admin", Comando::DarEscuadra { heroe: h, tropa: Tropa::MiliciaLanceros, cantidad: 10 }).creadas[0];
        let loadout = self.n.heroe(h).unwrap().heroe.loadouts[0].id;
        self.ok("admin", Comando::GuardarLoadout { heroe: h, loadout: Some(loadout), nombre: "Default".into(), escuadras: vec![escuadra], perks: vec![], activo: Some(true) });
        (h, escuadra)
    }

    /// Una columna de campo de `cuenta`/`faccion` a 2 u de la plaza B, con una escuadra de milicia.
    fn columna_junto_a_b(&mut self, m: &M, cuenta: &str, faccion: u32, dy: Mil) -> u64 {
        let p = en_tierra(&m.0, self.n.estado().asentamientos[&self.b].posicion, 2 * U, dy);
        let t = self.n.ahora();
        let col = self.n.crear_ejercito(t, cuenta, faccion, p, 500, 350_000).unwrap();
        let h = self.n.ejercito(col).unwrap().columna.lider;
        self.ok("admin", Comando::DarEscuadra { heroe: h, tropa: Tropa::MiliciaLanceros, cantidad: 10 });
        col
    }
}

fn arranque(m: &'static M, semilla: u64, ajustes: Ajustes) -> Arranque {
    Arranque { reglas: reglas(), ajustes, semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

/// El escenario en memoria o, con `dir`, persistente (registro y fotos en `dir`).
fn escenario(m: &'static M, semilla: u64, ajustes: Ajustes, dir: Option<&std::path::Path>) -> Esc {
    let a = arranque(m, semilla, ajustes);
    let n = match dir {
        Some(d) => Nucleo::con_persistencia(a, d, Default::default()).expect("almacén nuevo"),
        None => Nucleo::nuevo(a),
    };
    let total = m.0.campamentos().len();
    let (ia, ib) = (semilla as usize % total, (semilla as usize + 3) % total);
    let mut x = Esc { n, a: 0, b: 0, campamento: ia as u32, ana: 0, escuadra_de_ana: 0, beto: 0, dora: 0, nora: 0, fede: 0, peticiones: 0 };
    let (pa, pb_) = (sitio_rico(&x.n, &m.0, ia, 1), sitio_rico(&x.n, &m.0, ib, 2));
    x.a = fundar(&mut x.n, "fundador_a", 1, pa);
    x.b = fundar(&mut x.n, "fundador_b", 2, pb_);
    for plaza in [x.a, x.b] {
        x.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
        for r in asentamiento::recurso::TODOS {
            x.ok("admin", Comando::DarRecursos { asentamiento: plaza, recurso: r, cantidad: 5_000_000 });
        }
    }
    (x.ana, x.escuadra_de_ana) = x.residente("ana", 1, x.a, ia as u32);
    x.beto = x.residente("beto", 2, x.b, ib as u32).0;
    x.dora = x.ok("dora", Comando::CrearHeroe { cuenta: "dora".into(), campamento: ia as u32, controlador: Controlador::Humano }).creadas[0];
    x.ok("admin", Comando::DarAlmacenPersonal { heroe: x.dora, recurso: asentamiento::Recurso::Oro, cantidad: 1_000_000 });
    x.nora = x.columna_junto_a_b(m, "nora", 3, 0);
    x.fede = x.columna_junto_a_b(m, "fede", 1, 2 * U);
    // Pasado el día de protección con que nacen las plazas (Doc 1.2): nora puede asediar B desde la primera orden.
    let t = x.n.protegida_hasta(x.b).unwrap_or(0).max(x.n.ahora());
    x.n.avanzar_hasta((t / MS_POR_MINUTO + 1) * MS_POR_MINUTO + 1_000);
    x.ok("admin", Comando::Reloj(AccionReloj::Pausar));
    x
}

// ───────────────────────────── Órdenes del contrato ─────────────────────────────

fn texto(x: &str) -> Value {
    Value { kind: Some(Kind::StringValue(x.into())) }
}

fn de_admin(codigo: &str, parametros: &[(&str, Value)]) -> orden::Cuerpo {
    orden::Cuerpo::Administracion(pb::OrdenDeAdministracion { codigo: codigo.into(), parametros: Some(Struct { fields: parametros.iter().map(|(k, v)| ((*k).to_owned(), v.clone())).collect() }) })
}

fn reloj_hasta(t: Instante) -> orden::Cuerpo {
    orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: "avanzar_hasta".into(), factor_milesimas: None, hasta_ms: Some(t) })
}

fn marchar(heroe: u64, p: Punto) -> orden::Cuerpo {
    use pb::orden_marchar_a::{objetivo::Tipo, Objetivo};
    orden::Cuerpo::MarcharA(pb::OrdenMarcharA { heroe_id: ids::heroe(heroe), objetivo: Some(Objetivo { tipo: Some(Tipo::Punto(pb::Punto { x: p.x, y: p.y })) }) })
}

fn atacar(heroe: u64, plaza: u64) -> orden::Cuerpo {
    use pb::orden_atacar::{objetivo::Tipo, Objetivo};
    orden::Cuerpo::Atacar(pb::OrdenAtacar { heroe_id: ids::heroe(heroe), objetivo: Some(Objetivo { tipo: Some(Tipo::AsentamientoId(ids::asentamiento(plaza))) }), declara_guerra: Some(true) })
}

fn vista(r: (Punto, Punto)) -> mc::Cuerpo {
    mc::Cuerpo::Vista(pb::CambiarVista { rectangulo: Some(pb::Rectangulo { min: Some(pb::Punto { x: r.0.x, y: r.0.y }), max: Some(pb::Punto { x: r.1.x, y: r.1.y }) }) })
}

fn alrededor(p: Punto, r: Mil) -> (Punto, Punto) {
    (Punto::new(p.x - r, p.y - r), Punto::new(p.x + r, p.y + r))
}

/// Lo que ya iba hacia la sesión: una orden rechazada (por no existir su columna o, a quien combate, por `batalla.bloqueado`) sale después de todo lo anterior.
async fn pendiente(c: &mut ClientePrueba) -> Vec<ms::Cuerpo> {
    let (previos, r) = dar_todo(c, "barrera", orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: "ej-999999".into() })).await;
    assert!(["ejercito.desconocido", "batalla.bloqueado"].contains(&rechazo(&r)), "{r:?}");
    previos
}

// ───────────────────────────── Los clientes ─────────────────────────────

/// Un cliente de la prueba: su conexión, lo que tiene y lo que tenía en la comprobación anterior.
struct Jugador {
    cuenta: &'static str,
    tipo: TipoCliente,
    vista: Option<(Punto, Punto)>,
    c: Option<ClientePrueba>,
    almacen: Almacen,
    anterior: Almacen,
}

/// Cuántos mensajes de cada tipo de ámbito llegaron (`Foto` o `Delta`) y cuántas nieblas enteras y parciales: que la propiedad no se cumpla en el vacío.
type Cuenta = BTreeMap<String, usize>;

fn contar(cuenta: &mut Cuenta, mensajes: &[ms::Cuerpo]) {
    for m in mensajes {
        if let ms::Cuerpo::Delta(pb::Delta { niebla: Some(x), .. }) = m {
            *cuenta.entry(if x.parcial { "niebla parcial" } else { "niebla entera" }.to_owned()).or_default() += 1;
        }
        let a = match m {
            ms::Cuerpo::Delta(d) => &d.ambito,
            ms::Cuerpo::Foto(f) => &f.ambito,
            _ => continue,
        };
        *cuenta.entry(clave_de_ambito(a).split(':').next().unwrap().to_owned()).or_default() += 1;
    }
}

/// Conecta a `cuenta` reanudando con lo que conserva `desde` (vacío: sin nada, todo foto) y, si tiene vista, la vuelve a pedir. Devuelve la conexión, lo que tiene al acabar, lo que llegó y la
/// `Bienvenida`.
/// El cliente descarta los ámbitos que no le vuelven (la `Bienvenida` dice cuáles tiene) y la `VISTA` (no se reanuda).
async fn conectar(w: &Servicio, cuenta: &str, tipo: TipoCliente, rect: Option<(Punto, Punto)>, desde: &Almacen) -> (ClientePrueba, Almacen, Vec<ms::Cuerpo>, pb::Bienvenida) {
    let (mut c, b, mut mensajes) = w.conectar(desde.hola(&format!("ok:{cuenta}"), tipo)).await;
    let vuelven: BTreeSet<String> = b.ambitos.iter().map(|a| clave_de_ambito(&a.ambito)).collect();
    let mut a = desde.clone();
    a.ambitos.retain(|k, _| vuelven.contains(k));
    a.hechos.clear();
    if let Some(r) = rect {
        c.enviar(vista(r)).await;
        loop {
            let m = c.esperar().await.cuerpo.expect("un mensaje con cuerpo");
            let es_la_foto = matches!(&m, ms::Cuerpo::Foto(f) if clave_de_ambito(&f.ambito).starts_with("VISTA:"));
            mensajes.push(m);
            if es_la_foto {
                break;
            }
        }
    }
    a.aplicar_todo(&mensajes);
    (c, a, mensajes, b)
}

/// Las reanudaciones por deltas de cada tipo de ámbito (`deltas:TIPO`): que las comprobaciones de reanudar no sean todas con foto.
fn contar_reanudaciones(cuenta: &mut Cuenta, b: &pb::Bienvenida) {
    for a in b.ambitos.iter().filter(|a| a.decision == pb::DecisionReanudacion::ReanudaConDeltas as i32) {
        *cuenta.entry(format!("deltas:{}", clave_de_ambito(&a.ambito).split(':').next().unwrap())).or_default() += 1;
    }
}

impl Jugador {
    async fn nuevo(w: &Servicio, cuenta: &'static str, tipo: TipoCliente, rect: Option<(Punto, Punto)>, cuenta_: &mut Cuenta) -> Jugador {
        let (c, almacen, mensajes, _) = conectar(w, cuenta, tipo, rect, &Almacen::default()).await;
        contar(cuenta_, &mensajes);
        Jugador { cuenta, tipo, vista: rect, c: Some(c), anterior: almacen.clone(), almacen }
    }

    /// Da una orden y aplica lo que llegó antes de su resultado.
    async fn dar(&mut self, peticion: &str, cuerpo: orden::Cuerpo, cuenta: &mut Cuenta) -> pb::ResultadoOrden {
        let (previos, r) = dar_todo(self.c.as_mut().expect("conectado"), peticion, cuerpo).await;
        contar(cuenta, &previos);
        self.almacen.aplicar_todo(&previos);
        r
    }

    /// Las tres comprobaciones: lo acumulado es lo que daría una foto nueva; reanudar con lo que tiene lo deja igual sin repetirle un hecho; reanudar con lo de la comprobación anterior
    /// (perdido todo lo de en medio) también. Se queda conectado con esto último.
    async fn comprobar(&mut self, w: &Servicio, donde: &str, cuenta: &mut Cuenta) {
        let mut vieja = self.c.take().expect("conectado");
        let previos = pendiente(&mut vieja).await;
        contar(cuenta, &previos);
        self.almacen.aplicar_todo(&previos);
        vieja.cerrar().await;
        let (cuenta_, tipo, rect) = (self.cuenta, self.tipo, self.vista);
        let (c, al_dia, mensajes, b) = conectar(w, cuenta_, tipo, rect, &self.almacen).await;
        contar_reanudaciones(cuenta, &b);
        let repetidos: Vec<&str> = mensajes.iter().filter_map(|m| if let ms::Cuerpo::Delta(d) = m { Some(d) } else { None }).flat_map(|d| d.eventos.iter().map(|e| e.codigo.as_str())).collect();
        assert!(repetidos.is_empty(), "{donde}: {cuenta_} reanuda al día y le llegan otra vez hechos que ya tenía: {repetidos:?}");
        c.cerrar().await;
        let (c, nueva, _, _) = conectar(w, cuenta_, tipo, rect, &Almacen::default()).await;
        c.cerrar().await;
        let esperado = nueva.contenido();
        assert_eq!(self.almacen.contenido(), esperado, "{donde}: lo acumulado por {cuenta_} (foto + deltas) no es la foto nueva");
        assert_eq!(al_dia.contenido(), esperado, "{donde}: {cuenta_} reanuda con lo que tiene y no queda como la foto nueva");
        let (c, desde_antes, _, b) = conectar(w, cuenta_, tipo, rect, &self.anterior).await;
        contar_reanudaciones(cuenta, &b);
        assert_eq!(desde_antes.contenido(), esperado, "{donde}: {cuenta_} reanuda con lo de la comprobación anterior (un hueco) y no queda como la foto nueva");
        self.c = Some(c);
        (self.almacen, self.anterior) = (desde_antes.clone(), desde_antes);
    }
}

// ───────────────────────────── El servidor de batalla de prueba ─────────────────────────────

/// Toma las batallas que le llegan y las lleva (asignación, inicio, resultado) cuando le toca jugar.
struct Svc {
    c: ClientePrueba,
    tickets: BTreeMap<String, pb::BattleTicket>,
    fase: BTreeMap<String, u8>,
    intentos: u64,
}

impl Svc {
    async fn leer_lo_que_haya(&mut self) {
        while let Ok(Some(m)) = tokio::time::timeout(Duration::from_millis(40), self.c.recibir()).await {
            if let Some(ms::Cuerpo::Batalla(pb::MensajeParaServidorDeBatalla { cuerpo: Some(para::Cuerpo::Ticket(t)) })) = m.cuerpo {
                self.fase.insert(t.battle_id.clone(), 0);
                self.tickets.insert(t.battle_id.clone(), t);
            }
        }
    }

    /// Da el siguiente paso de una de sus batallas (la que elige `rng`) y espera su acuse.
    async fn jugar(&mut self, rng: &mut Rng) {
        self.leer_lo_que_haya().await;
        let vivas: Vec<String> = self.fase.iter().filter(|(_, &f)| f < 3).map(|(b, _)| b.clone()).collect();
        if vivas.is_empty() {
            return;
        }
        let b = vivas[rng.por_debajo(vivas.len() as u64) as usize].clone();
        let t = self.tickets[&b].clone();
        let intento = format!("i{}", self.intentos);
        let cuerpo = match self.fase[&b] {
            0 => {
                self.intentos += 1;
                de_srv::Cuerpo::Asignacion(pb::BattleServerAssignment {
                    schema_version: 1,
                    battle_id: b.clone(),
                    ticket_revision: t.ticket_revision,
                    intento_asignacion_id: format!("i{}", self.intentos),
                    instancia: Some(pb::InstanciaDeBatalla { host: "unity.test".into(), puerto: 7777, protocolo: "udp".into() }),
                    tokens_participante: vec![],
                    ..Default::default()
                })
            }
            1 => de_srv::Cuerpo::Inicio(pb::InicioBatalla { schema_version: 1, battle_id: b.clone(), ticket_revision: t.ticket_revision, intento_asignacion_id: intento }),
            _ => resultado_de(&t, &intento, if rng.por_debajo(2) == 0 { pb::LadoId::Atacante } else { pb::LadoId::Defensor }),
        };
        self.c.enviar(mc::Cuerpo::Batalla(pb::MensajeDeServidorDeBatalla { cuerpo: Some(cuerpo) })).await;
        loop {
            match self.c.esperar().await.cuerpo {
                Some(ms::Cuerpo::Batalla(pb::MensajeParaServidorDeBatalla { cuerpo: Some(para::Cuerpo::Acuse(a)) })) => {
                    if a.resultado == pb::Resultado::OrdenAceptada as i32 {
                        *self.fase.get_mut(&b).unwrap() += 1;
                    }
                    return;
                }
                Some(ms::Cuerpo::Batalla(pb::MensajeParaServidorDeBatalla { cuerpo: Some(para::Cuerpo::Ticket(t)) })) => {
                    self.fase.insert(t.battle_id.clone(), 0);
                    self.tickets.insert(t.battle_id.clone(), t);
                }
                _ => {}
            }
        }
    }
}

/// El resultado de una partida en la que cada escuadra del ticket pierde un soldado.
fn resultado_de(t: &pb::BattleTicket, intento: &str, ganador: pb::LadoId) -> de_srv::Cuerpo {
    let bandos = [t.atacante.as_ref().unwrap(), t.defensor.as_ref().unwrap()];
    let participantes: Vec<&pb::BattleParticipantSnapshot> = bandos.iter().flat_map(|b| &b.participantes).collect();
    let escuadras = participantes.iter().flat_map(|p| &p.escuadras).chain(bandos.iter().flat_map(|b| &b.escuadras_sin_heroe));
    de_srv::Cuerpo::Resultado(pb::BattleResult {
        schema_version: 1,
        battle_id: t.battle_id.clone(),
        result_id: format!("res-{intento}"),
        ticket_revision: t.ticket_revision,
        intento_asignacion_id: intento.into(),
        inicio_ms: 1_000,
        fin_ms: 61_000,
        ganador: ganador as i32,
        razon: pb::RazonVictoria::Aniquilacion as i32,
        objetivos: vec![],
        por_escuadra: escuadras.map(|e| pb::ResultadoEscuadra { squad_id: e.squad_id.clone(), desplegados: e.efectivos_autorizados, supervivientes_al_cierre: e.efectivos_autorizados.saturating_sub(1), muertos: e.efectivos_autorizados.min(1), xp_ganada: 2 }).collect(),
        por_heroe: participantes.iter().map(|p| pb::ResultadoHeroe { heroe_id: p.heroe_id.clone(), participo: true, sobrevivio_al_cierre: true, xp_ganada: 3, derrotado: None, botin: None }).collect(),
        version_servidor: "v1".into(),
        carros_perdidos: vec![],
    })
}

// ───────────────────────────── 1. La propiedad, con órdenes aleatorias ─────────────────────────────

const PASOS: u64 = 48;
const CADA: u64 = 12;

/// Un mundo y una serie de órdenes de la semilla dada; cada `CADA` pasos, las tres comprobaciones con cada cliente. Devuelve cuántos mensajes llegaron de cada tipo de ámbito.
async fn caso(m: &'static M, semilla: u64) -> Cuenta {
    let x = escenario(m, semilla, Ajustes::default(), None);
    let (a, b, camp, ana, esc_ana, beto, dora) = (x.a, x.b, x.campamento, x.ana, x.escuadra_de_ana, x.beto, x.dora);
    let (pos_a, pos_b) = (x.n.estado().asentamientos[&a].posicion, x.n.estado().asentamientos[&b].posicion);
    let (nora_col, fede_col) = (x.nora, x.fede);
    let (nora, fede) = (x.n.ejercito(nora_col).unwrap().columna.lider, x.n.ejercito(fede_col).unwrap().columna.lider);
    let todo = (Punto::new(0, 0), Punto::new(2_667_000, 2_000_000));
    let mut t = x.n.ahora();
    let w = arrancar_con(x.n).await;
    let mut cuenta = Cuenta::new();
    let mut admin = Jugador::nuevo(&w, "admin", TipoCliente::ClienteAdminWeb, None, &mut cuenta).await;
    let mut jugadores = vec![
        Jugador::nuevo(&w, "ana", TipoCliente::ClienteUnity, Some(alrededor(pos_a, 150 * U)), &mut cuenta).await,
        Jugador::nuevo(&w, "beto", TipoCliente::ClienteUnity, None, &mut cuenta).await,
        Jugador::nuevo(&w, "dora", TipoCliente::ClienteUnity, Some(todo), &mut cuenta).await,
        Jugador::nuevo(&w, "nora", TipoCliente::ClienteBot, Some(alrededor(pos_b, 150 * U)), &mut cuenta).await,
        Jugador::nuevo(&w, "fede", TipoCliente::ClienteBot, None, &mut cuenta).await,
    ];
    let (c, bienvenida) = ClientePrueba::entrar(w.srv.direccion, hola("svc:srv", TipoCliente::ClienteServidorBatalla)).await;
    assert!(bienvenida.ambitos.is_empty());
    let mut svc = Svc { c, tickets: BTreeMap::new(), fase: BTreeMap::new(), intentos: 0 };
    let mut rng = Rng::derivado(semilla, "ordenes");
    let cerca = |rng: &mut Rng, p: Punto| en_tierra(&m.0, p, rng.por_debajo(40 * U as u64) as Mil - 20 * U, rng.por_debajo(40 * U as u64) as Mil - 20 * U);
    let recursos = ["madera", "piedra", "trigo", "oro"];
    for paso in 0..PASOS {
        let id = format!("s{semilla}p{paso}");
        // El primer paso abre siempre una batalla (nora asedia B); el resto, al azar.
        let accion = if paso == 0 { 11 } else { rng.por_debajo(14) };
        match accion {
            0..=2 => {
                t += 1_000 + rng.por_debajo(89_000) as Instante;
                let r = admin.dar(&id, reloj_hasta(t), &mut cuenta).await;
                assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
            }
            3 => {
                let plaza = if rng.por_debajo(2) == 0 { a } else { b };
                let recurso = recursos[rng.por_debajo(4) as usize];
                let cantidad = (1_000 + rng.por_debajo(50_000)) as f64;
                admin.dar(&id, admin_dar_recursos(plaza, recurso, cantidad), &mut cuenta).await;
            }
            4 => {
                let (fa, fb) = [(1, 2), (1, 3), (2, 3)][rng.por_debajo(3) as usize];
                let mut p = vec![("faccion_a_id", texto(&ids::faccion(fa))), ("faccion_b_id", texto(&ids::faccion(fb)))];
                p.extend([None, Some("alianza"), Some("guerra")][rng.por_debajo(3) as usize].map(|r| ("relacion", texto(r))));
                admin.dar(&id, de_admin("fijar_relacion", &p), &mut cuenta).await;
            }
            5 => {
                let mut p = vec![("heroe_id", texto(&ids::heroe(dora)))];
                p.extend([None, Some(1), Some(2)][rng.por_debajo(3) as usize].map(|f| ("faccion_id", texto(&ids::faccion(f)))));
                admin.dar(&id, de_admin("fijar_faccion_de_heroe", &p), &mut cuenta).await;
            }
            6 => {
                let sitio = [ids::campamento(camp), ids::asentamiento(a), ids::asentamiento(b)][rng.por_debajo(3) as usize].clone();
                admin.dar(&id, de_admin("colocar_heroe", &[("heroe_id", texto(&ids::heroe(dora))), ("ubicacion", texto(&sitio))]), &mut cuenta).await;
            }
            7 => {
                let cuerpo = match rng.por_debajo(4) {
                    0 => orden::Cuerpo::SalirAlMundo(pb::OrdenSalirAlMundo { asentamiento_id: ids::asentamiento(a), heroe_id: ids::heroe(ana), escuadron_ids: vec![ids::escuadra(esc_ana)], carga: None }),
                    1 => orden::Cuerpo::EntrarEnAsentamiento(pb::OrdenEntrarEnAsentamiento { asentamiento_id: ids::asentamiento(a), heroe_id: ids::heroe(ana) }),
                    2 => marchar(ana, pos_a),
                    _ => marchar(ana, cerca(&mut rng, pos_a)),
                };
                jugadores[0].dar(&id, cuerpo, &mut cuenta).await;
            }
            8 => {
                let tipo = ["vivienda", "granja", "lenera"][rng.por_debajo(3) as usize];
                let cuerpo = orden::Cuerpo::AnadirEdificioManualmente(pb::OrdenAnadirEdificioManualmente { asentamiento_id: ids::asentamiento(b), cargo: "gobernador".into(), tipo: tipo.into() });
                jugadores[1].dar(&id, cuerpo, &mut cuenta).await;
            }
            9 => {
                let cerrada_a = ["neutrales", "enemigos"].into_iter().filter(|_| rng.por_debajo(2) == 0).map(String::from).collect();
                jugadores[1].dar(&id, orden::Cuerpo::FijarPuerta(pb::OrdenFijarPuerta { asentamiento_id: ids::asentamiento(b), heroe_id: ids::heroe(beto), cerrada_a }), &mut cuenta).await;
            }
            10 => {
                let cuerpo = match rng.por_debajo(3) {
                    0 => orden::Cuerpo::ComprarEnCampamento(pb::OrdenComprarEnCampamento { recurso: "trigo".into(), cantidad: 1_000 + rng.por_debajo(20_000) as i64, campamento_id: None }),
                    1 => orden::Cuerpo::SalirDelCampamento(pb::OrdenSalirDelCampamento { campamento_id: ids::campamento(camp), heroe_id: ids::heroe(dora), ..Default::default() }),
                    _ => orden::Cuerpo::EntrarEnCampamento(pb::OrdenEntrarEnCampamento { campamento_id: ids::campamento(camp), heroe_id: ids::heroe(dora) }),
                };
                jugadores[2].dar(&id, cuerpo, &mut cuenta).await;
            }
            11 => {
                let cuerpo = match rng.por_debajo(3) {
                    _ if paso == 0 => atacar(nora, b),
                    0 => atacar(nora, b),
                    1 => marchar(nora, cerca(&mut rng, pos_b)),
                    _ => orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: ids::ejercito(nora_col) }),
                };
                let r = jugadores[3].dar(&id, cuerpo, &mut cuenta).await;
                if paso == 0 {
                    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "nora asedia B: {r:?}");
                }
            }
            12 => {
                let cuerpo = if rng.por_debajo(2) == 0 { marchar(fede, cerca(&mut rng, pos_b)) } else { orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: ids::ejercito(fede_col) }) };
                jugadores[4].dar(&id, cuerpo, &mut cuenta).await;
            }
            _ => svc.jugar(&mut rng).await,
        }
        if (paso + 1) % CADA == 0 {
            let donde = format!("semilla {semilla}, paso {paso}");
            admin.comprobar(&w, &donde, &mut cuenta).await;
            for j in &mut jugadores {
                j.comprobar(&w, &donde, &mut cuenta).await;
            }
        }
    }
    w.parar().await;
    cuenta
}

fn admin_dar_recursos(plaza: u64, recurso: &str, cantidad: f64) -> orden::Cuerpo {
    de_admin("dar_recursos", &[("asentamiento_id", texto(&ids::asentamiento(plaza))), ("recurso", texto(recurso)), ("cantidad", Value { kind: Some(Kind::NumberValue(cantidad)) })])
}

/// Cada semilla monta su mundo y su serie; todas juntas tienen que haber movido los siete ámbitos.
#[tokio::test]
async fn foto_mas_deltas_es_la_foto_final_en_los_siete_ambitos_con_ordenes_aleatorias_y_reanudar_lo_deja_igual() {
    let m = con_mundo!();
    let mut total = Cuenta::new();
    for semilla in 1..=4 {
        for (k, v) in caso(m, semilla).await {
            *total.entry(k).or_default() += v;
        }
    }
    for tipo in ["HEROE", "FACCION", "ASENTAMIENTO", "CAMPAMENTO", "VISTA", "ADMIN", "BATALLA"] {
        assert!(total.get(tipo).copied().unwrap_or(0) >= 4, "el ámbito {tipo} apenas recibió nada: {total:?}");
    }
    assert!(total.get("niebla parcial").copied().unwrap_or(0) >= 1, "la niebla viajó siempre entera: {total:?}");
    // Y se reanudaron por deltas, varios a la vez, todos los que se reanudan (`VISTA` no: se vuelve a pedir).
    for tipo in ["HEROE", "FACCION", "ASENTAMIENTO", "CAMPAMENTO", "ADMIN", "BATALLA"] {
        assert!(total.get(&format!("deltas:{tipo}")).copied().unwrap_or(0) >= 1, "el ámbito {tipo} no se reanudó nunca por deltas: {total:?}");
    }
    eprintln!("mensajes por tipo de ámbito: {total:?}");
}

// ───────────────────────────── 2. Un hecho, una vez ─────────────────────────────

/// Duda 8 de T5: cada hecho llega a cada sesión por un solo ámbito, el primero de los abiertos que lo oye. (a) Si ese ámbito se acaba de abrir con su `Foto` en el mismo reparto (entrar en la plaza abre
/// `ASENTAMIENTO`), el hecho no se pierde: va por el siguiente que lo oye (`HEROE`, que lo nombra). (b) Al reanudar, un hecho que ya llegó por un ámbito que el cliente ya no tiene (la plaza de la que lo
/// sacaron) no vuelve a llegar por otro que lo oye y vuelve de una copia vieja (`FACCION`): solo se cuentan los de versiones posteriores a la mayor que el cliente declara.
#[tokio::test]
async fn un_hecho_llega_una_vez_aunque_el_ambito_que_lo_oye_se_acabe_de_abrir_y_al_reanudar_no_se_repite_aunque_el_que_lo_trajo_ya_no_este() {
    let m = con_mundo!();
    let x = escenario(m, 1, Ajustes::default(), None);
    let (a, ana, esc, camp) = (x.a, x.ana, x.escuadra_de_ana, x.campamento);
    let w = arrancar_con(x.n).await;
    let mut cuenta = Cuenta::new();
    let mut admin = Jugador::nuevo(&w, "admin", TipoCliente::ClienteAdminWeb, None, &mut cuenta).await;
    let mut j = Jugador::nuevo(&w, "ana", TipoCliente::ClienteUnity, None, &mut cuenta).await;
    let aceptada = |r: &pb::ResultadoOrden| assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    let heroe = format!("HEROE:{}", ids::heroe(ana));
    // Sale al mundo y vuelve a entrar: entrar abre `ASENTAMIENTO` con su `Foto` en el mismo reparto que el hecho de la entrada, que le llega por `HEROE`.
    aceptada(&j.dar("salir", orden::Cuerpo::SalirAlMundo(pb::OrdenSalirAlMundo { asentamiento_id: ids::asentamiento(a), heroe_id: ids::heroe(ana), escuadron_ids: vec![ids::escuadra(esc)], carga: None }), &mut cuenta).await);
    aceptada(&j.dar("entrar", orden::Cuerpo::EntrarEnAsentamiento(pb::OrdenEntrarEnAsentamiento { asentamiento_id: ids::asentamiento(a), heroe_id: ids::heroe(ana) }), &mut cuenta).await);
    let llegados = |j: &Jugador, codigo: &str| j.almacen.hechos.iter().filter(|(_, e)| e.codigo == codigo).map(|(k, _)| k.clone()).collect::<Vec<_>>();
    assert_eq!(llegados(&j, "jugador.entra_en_asentamiento"), [heroe.as_str()], "el hecho de entrar no se pierde aunque la plaza se abra en ese mismo reparto");
    assert_eq!(llegados(&j, "jugador.sale_al_mundo"), [heroe.as_str()]);
    // Un hecho de la plaza que no nombra a nadie: lo oyen `ASENTAMIENTO` (ana está dentro) y `FACCION`; le llega por la plaza. Después la administración la saca al campamento: pierde la plaza y su
    // `FACCION` no cambia.
    aceptada(&j.dar("auto", orden::Cuerpo::AlternarAutoConstruccion(pb::OrdenAlternarAutoConstruccion { asentamiento_id: ids::asentamiento(a), pausada: false }), &mut cuenta).await);
    let v_auto = j.almacen.hechos.iter().find(|(_, e)| e.codigo == "construccion.auto_alternada").map(|(_, e)| e.version).expect("el hecho de la plaza");
    assert_eq!(llegados(&j, "construccion.auto_alternada"), [format!("ASENTAMIENTO:{}", ids::asentamiento(a))]);
    aceptada(&admin.dar("fuera", de_admin("colocar_heroe", &[("heroe_id", texto(&ids::heroe(ana))), ("ubicacion", texto(&ids::campamento(camp)))]), &mut cuenta).await);
    let mut c = j.c.take().unwrap();
    j.almacen.aplicar_todo(&pendiente(&mut c).await);
    c.cerrar().await;
    assert!(!j.almacen.ambitos.contains_key(&format!("ASENTAMIENTO:{}", ids::asentamiento(a))), "la plaza se le revocó");
    // Al reanudar no se repite ningún hecho: tampoco si `FACCION`, que también oye el de la plaza, vuelve de una copia vieja (la de antes de todo) y la plaza ya no es suya.
    let faccion = format!("FACCION:{}", ids::faccion(1));
    let mut mezcla = j.almacen.clone();
    mezcla.ambitos.insert(faccion.clone(), j.anterior.ambitos[&faccion].clone());
    assert!(mezcla.ambitos[&faccion].version < v_auto, "el caso exige que FACCION vuelva de antes del hecho");
    let (c, al_dia, mensajes, _) = conectar(&w, "ana", TipoCliente::ClienteUnity, None, &mezcla).await;
    let hechos = |mensajes: &[ms::Cuerpo]| mensajes.iter().filter_map(|m| if let ms::Cuerpo::Delta(d) = m { Some(d) } else { None }).flat_map(|d| d.eventos.iter().map(|e| e.codigo.clone())).collect::<Vec<_>>();
    assert_eq!(hechos(&mensajes), Vec::<String>::new(), "al reanudar no se repite ningún hecho de antes de la mayor versión que declara");
    assert_eq!(al_dia.contenido(), j.almacen.contenido());
    c.cerrar().await;
    // Quien reanuda con lo de antes de todo (un cliente que lo perdió) sí recibe cada hecho, una vez.
    let (c, _, mensajes, _) = conectar(&w, "ana", TipoCliente::ClienteUnity, None, &j.anterior).await;
    let todos = hechos(&mensajes);
    for codigo in ["jugador.sale_al_mundo", "jugador.entra_en_asentamiento", "construccion.auto_alternada"] {
        assert_eq!(todos.iter().filter(|c| *c == codigo).count(), 1, "{codigo} una vez: {todos:?}");
    }
    c.cerrar().await;
    w.parar().await;
}

// ───────────────────────────── 3. Retención, `version_esperada` e idempotencia ─────────────────────────────

/// La retención del registro (`Ajustes::retencion_cambios`, 65.536 versiones por defecto): quien conserva la versión más vieja que el registro aún alcanza reanuda por deltas; una más vieja,
/// con foto; y de los dos modos el cliente queda igual que con una foto nueva. Aquí la retención es de 8 para no generar 65.536 versiones.
#[tokio::test]
async fn el_registro_reanuda_por_deltas_hasta_la_version_mas_vieja_que_retiene_y_mas_alla_con_foto() {
    assert_eq!(Ajustes::default().retencion_cambios, 65_536);
    let m = con_mundo!();
    let x = escenario(m, 2, Ajustes { retencion_cambios: 8, ..Ajustes::default() }, None);
    let a = x.a;
    let w = arrancar_con(x.n).await;
    let mut cuenta = Cuenta::new();
    let mut admin = Jugador::nuevo(&w, "admin", TipoCliente::ClienteAdminWeb, None, &mut cuenta).await;
    let mut ana = Jugador::nuevo(&w, "ana", TipoCliente::ClienteUnity, None, &mut cuenta).await;
    // Cada `dar_recursos` cierra una versión, que el cliente de administración recibe (y ana, en su plaza).
    let mut copias = vec![(admin.almacen.clone(), ana.almacen.clone())];
    for i in 0..9 {
        let r = admin.dar(&format!("dar{i}"), admin_dar_recursos(a, "madera", 1_000.0), &mut cuenta).await;
        assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
        let mut c = ana.c.take().unwrap();
        ana.almacen.aplicar_todo(&pendiente(&mut c).await);
        ana.c = Some(c);
        copias.push((admin.almacen.clone(), ana.almacen.clone()));
    }
    let hoy = admin.almacen.ambitos["ADMIN:"].version;
    assert_eq!(copias[0].0.ambitos["ADMIN:"].version + 9, hoy, "una versión por orden");
    for j in [&mut admin, &mut ana] {
        j.c.take().unwrap().cerrar().await;
    }
    let (_, nueva_admin, _, _) = conectar(&w, "admin", TipoCliente::ClienteAdminWeb, None, &Almacen::default()).await;
    let (_, nueva_ana, _, _) = conectar(&w, "ana", TipoCliente::ClienteUnity, None, &Almacen::default()).await;
    // Con 9 versiones nuevas y 8 retenidas, la copia de hace 8 aún se reanuda por deltas y la de hace 9 ya no.
    let decision = |b: &pb::Bienvenida, ambito: &str| b.ambitos.iter().find(|x| clave_de_ambito(&x.ambito) == ambito).map(|x| x.decision).expect("el ámbito vuelve");
    let plaza = format!("ASENTAMIENTO:{}", ids::asentamiento(a));
    for (i, esperada) in [(0, pb::DecisionReanudacion::ReanudaConFoto), (1, pb::DecisionReanudacion::ReanudaConDeltas), (8, pb::DecisionReanudacion::ReanudaConDeltas)] {
        let (c, b, mensajes) = w.conectar(copias[i].0.hola("ok:admin", TipoCliente::ClienteAdminWeb)).await;
        assert_eq!(decision(&b, "ADMIN:"), esperada as i32, "ADMIN desde la versión {}", copias[i].0.ambitos["ADMIN:"].version);
        let mut almacen = copias[i].0.clone();
        almacen.aplicar_todo(&mensajes);
        assert_eq!(almacen.contenido(), nueva_admin.contenido(), "la administración reanuda desde la copia {i} y no queda como la foto nueva");
        c.cerrar().await;
        let (c, b, mensajes) = w.conectar(copias[i].1.hola("ok:ana", TipoCliente::ClienteUnity)).await;
        assert_eq!(decision(&b, &plaza), esperada as i32, "la plaza de ana desde la versión {}", copias[i].1.ambitos[&plaza].version);
        let mut almacen = copias[i].1.clone();
        almacen.aplicar_todo(&mensajes);
        assert_eq!(almacen.contenido(), nueva_ana.contenido(), "ana reanuda desde la copia {i} y no queda como la foto nueva");
        c.cerrar().await;
    }
    w.parar().await;
}

/// `version_esperada`: si el mundo ya pasó de ella, `version.obsoleta` sin ningún cambio; si es la de hoy, la orden se aplica. Idempotencia: la misma `peticion_id` otra vez es
/// `ORDEN_DUPLICADA` con la versión de entonces, sin `Delta` ni hecho nuevos, también desde una sesión nueva tras reconectar.
#[tokio::test]
async fn version_esperada_obsoleta_se_rechaza_y_una_orden_repetida_no_se_aplica_dos_veces_ni_tras_reconectar() {
    let m = con_mundo!();
    let x = escenario(m, 3, Ajustes::default(), None);
    let a = x.a;
    let w = arrancar_con(x.n).await;
    let mut cuenta = Cuenta::new();
    let mut ana = Jugador::nuevo(&w, "ana", TipoCliente::ClienteUnity, None, &mut cuenta).await;
    let auto = |pausada| orden::Cuerpo::AlternarAutoConstruccion(pb::OrdenAlternarAutoConstruccion { asentamiento_id: ids::asentamiento(a), pausada });
    let con_version = |id: &str, version: u64, cuerpo| mc::Cuerpo::Orden(pb::Orden { peticion_id: id.into(), version_esperada: Some(version), cuerpo: Some(cuerpo) });
    let hoy = ana.almacen.ambitos.values().map(|x| x.version).max().unwrap();
    // Una versión que el mundo ya pasó: rechazo sin cambios.
    let c = ana.c.as_mut().unwrap();
    c.enviar(con_version("vieja", hoy - 1, auto(false))).await;
    let r = resultado(c).await;
    assert_eq!((rechazo(&r), r.version_tras), ("version.obsoleta", hoy));
    // La de hoy: se aplica.
    c.enviar(con_version("al_dia", hoy, auto(false))).await;
    let mut previos = Vec::new();
    let r = loop {
        match c.esperar().await.cuerpo.unwrap() {
            ms::Cuerpo::Resultado(r) => break r,
            m => previos.push(m),
        }
    };
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    assert!(!previos.is_empty(), "la orden aceptada trae sus Delta antes del resultado");
    ana.almacen.aplicar_todo(&previos);
    let v = r.version_tras;
    // La misma petición otra vez: duplicada, con la versión de entonces y nada antes.
    let (previos, r) = dar_todo(ana.c.as_mut().unwrap(), "al_dia", auto(true)).await;
    assert_eq!((r.resultado, r.version_tras, previos.len()), (pb::Resultado::OrdenDuplicada as i32, v, 0), "{r:?}");
    // Tras reconectar, desde una sesión nueva: también duplicada, y la plaza sigue como la dejó la primera.
    ana.c.take().unwrap().cerrar().await;
    let (mut c, al_dia, mensajes, _) = conectar(&w, "ana", TipoCliente::ClienteUnity, None, &ana.almacen).await;
    assert!(mensajes.is_empty(), "reanuda sin novedad: {mensajes:?}");
    let (previos, r) = dar_todo(&mut c, "al_dia", auto(true)).await;
    assert_eq!((r.resultado, r.version_tras, previos.len()), (pb::Resultado::OrdenDuplicada as i32, v, 0), "{r:?}");
    let plaza = &al_dia.ambitos[&format!("ASENTAMIENTO:{}", ids::asentamiento(a))].entidades[&(pb::TipoEntidad::Asentamiento as i32, ids::asentamiento(a))];
    assert!(matches!(plaza, pb::entidad::Cuerpo::Asentamiento(p) if !p.auto_construccion_pausada), "la repetida no la pausó");
    c.cerrar().await;
    w.parar().await;
}

// ───────────────────────────── 4. Tras un reinicio ─────────────────────────────

/// Duda 5 de T3b (y 4 de T3c): el registro que el núcleo persistente repite al recuperarse lleva las pertenencias, así que quien conserva una versión de antes del reinicio (y no anterior a la última
/// foto) reanuda por deltas todos sus ámbitos, y queda como con una foto nueva; antes, toda versión anterior a la recuperada pedía foto.
#[tokio::test]
async fn tras_un_reinicio_quien_conserva_una_version_de_antes_reanuda_por_deltas() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let x = escenario(m, 4, ajustes.clone(), Some(dir.path()));
    let a = x.a;
    let mut t = x.n.ahora();
    let w = arrancar_con(x.n).await;
    let mut cuenta = Cuenta::new();
    let mut admin = Jugador::nuevo(&w, "admin", TipoCliente::ClienteAdminWeb, None, &mut cuenta).await;
    let mut ana = Jugador::nuevo(&w, "ana", TipoCliente::ClienteUnity, None, &mut cuenta).await;
    // Lo que ana conserva a mitad; después el mundo sigue (una orden suya, recursos, la relación de su Facción, pasos de minuto) y se para.
    let mut c = ana.c.take().unwrap();
    ana.almacen.aplicar_todo(&pendiente(&mut c).await);
    ana.c = Some(c);
    let de_antes = ana.almacen.clone();
    let aceptada = |r: &pb::ResultadoOrden| assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    aceptada(&ana.dar("auto", orden::Cuerpo::AlternarAutoConstruccion(pb::OrdenAlternarAutoConstruccion { asentamiento_id: ids::asentamiento(a), pausada: false }), &mut cuenta).await);
    aceptada(&admin.dar("dar", admin_dar_recursos(a, "trigo", 9_000.0), &mut cuenta).await);
    aceptada(&admin.dar("rel", de_admin("fijar_relacion", &[("faccion_a_id", texto(&ids::faccion(1))), ("faccion_b_id", texto(&ids::faccion(2))), ("relacion", texto("alianza"))]), &mut cuenta).await);
    t += 3 * MS_POR_MINUTO;
    aceptada(&admin.dar("reloj", reloj_hasta(t), &mut cuenta).await);
    for j in [&mut admin, &mut ana] {
        j.c.take().unwrap().cerrar().await;
    }
    let vivo = w.parar().await;
    let (version, hash) = (vivo.version(), vivo.hash_estado());
    drop(vivo);

    // Se recupera (sin foto: el registro entero) y ana vuelve con lo de a mitad.
    let recuperado = Nucleo::con_persistencia(arranque(m, 4, ajustes), dir.path(), Default::default()).expect("se recupera");
    assert!(recuperado.recuperado());
    assert_eq!((recuperado.version(), recuperado.hash_estado()), (version, hash), "el mismo mundo");
    let w = arrancar_con(recuperado).await;
    let (c, nueva, _, _) = conectar(&w, "ana", TipoCliente::ClienteUnity, None, &Almacen::default()).await;
    c.cerrar().await;
    let (c, reanudada, _, b) = conectar(&w, "ana", TipoCliente::ClienteUnity, None, &de_antes).await;
    let decisiones: Vec<(String, i32)> = b.ambitos.iter().map(|x| (clave_de_ambito(&x.ambito), x.decision)).collect();
    assert!(decisiones.iter().all(|(_, d)| *d == pb::DecisionReanudacion::ReanudaConDeltas as i32), "todo por deltas tras el reinicio: {decisiones:?}");
    assert_eq!(reanudada.contenido(), nueva.contenido(), "reanudar tras el reinicio deja lo mismo que una foto nueva");
    c.cerrar().await;
    w.parar().await;
}
