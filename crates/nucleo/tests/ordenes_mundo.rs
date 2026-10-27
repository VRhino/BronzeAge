//! Las órdenes del contrato a través de la sesión sobre el artefacto de mundo real (4H, T2): presencia, columnas, economía, acceso, murallas, pueblos, avanzadas, comercio y batalla de
//! Unity. Cada prueba prepara el mundo directamente en el núcleo (lo que aún no tiene orden de jugador o sería largo de jugar), lo pone detrás del servidor y da las órdenes por el
//! contrato como lo haría un cliente: el resultado es el del núcleo, con su respuesta causal (las `Delta` que la causan llegan antes del resultado, que lleva lo creado con su clase y
//! su id del contrato). El artefacto es `generador/salida/mundo/mundo-*.bin`; las pruebas se omiten si no está.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::Recurso::{self, *};
use base::{Instante, Punto, MS_POR_MINUTO};
use comun::sesion::*;
use contrato::v1::{self as pb, orden, Resultado as R, TipoCliente, TipoEntidad};
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::economia::Mapa;
use nucleo::proyeccion::ids;
use nucleo::{Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Resultado, Respuesta};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

const MIN: Instante = MS_POR_MINUTO;
const HORA: Instante = 60 * MIN;

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

/// Las reglas por defecto sin acoso ni anillo de bandidos (estas pruebas miden otra cosa) y, si se pide, con servidores de batalla.
fn arranque(m: &M, semilla: u64, unity: bool) -> Arranque {
    let mut reglas = Reglas { batallas_en_unity: unity, ..Reglas::default() };
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    Arranque { reglas, ajustes: Ajustes::default(), semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

/// Un sitio verde con mucho bosque a 10 u (al menos 6 parcelas) cerca del campamento `i`: una plaza que no cae en ruinas por falta de madera.
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

/// Un sitio fundable, verde y con mucho bosque, a `r` del punto `c`, lejos de la protección de un campamento de mercenarios (donde nadie combate).
fn sitio_junto_a(n: &Nucleo, c: Punto, r: i64) -> Option<Punto> {
    (0..36).map(|g| ((g * 10) as f64).to_radians()).map(|a| Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64)).find(|&p| {
        let libre = |q: Punto| !n.en_proteccion_de_campamento(q);
        libre(p) && libre(Punto::new(p.x + 12_000, p.y)) && libre(Punto::new(p.x + 9_000, p.y))
            && n.viabilidad(p, 1).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6 && !v.condenada)
    })
}

/// El núcleo antes de ponerlo detrás del servidor: lo que no se juega por el contrato se prepara aquí.
struct Prep {
    n: Nucleo,
    cuenta: u64,
}

impl Prep {
    fn nuevo(m: &M, unity: bool) -> Self {
        Prep { n: Nucleo::nuevo(arranque(m, 7, unity)), cuenta: 0 }
    }
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.cuenta += 1;
        let p = Peticion::nueva(actor, &format!("prep{}", self.cuenta), c);
        let t = self.n.ahora();
        // 5I (5E-10): sin el andamiaje de `comun::mandaba`; cada cuenta da sus órdenes de plaza con la matriz real.
        self.n.aplicar_en(t, if actor == "admin" { p.de_admin() } else { p })
    }
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
        r
    }
    fn avanzar(&mut self, ms: Instante) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    /// Una columna de administración de la cuenta (con su héroe de campo): (columna, héroe).
    fn columna(&mut self, cuenta: &str, faccion: u32, p: Punto) -> (u64, u64) {
        let t = self.n.ahora();
        let id = self.n.crear_ejercito(t, cuenta, faccion, p, 500, 100_000).unwrap();
        (id, self.n.ejercito(id).unwrap().columna.lider)
    }
    /// Funda una plaza de la cuenta en `p` con una columna de fundación, y la cuenta tiene en ella un Gobernador de oficio (5E: sus órdenes de plaza pasan la matriz real).
    fn plaza(&mut self, cuenta: &str, faccion: u32, p: Punto) -> u64 {
        let (col, _) = self.columna(cuenta, faccion, p);
        self.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
        let plaza = self.ok(cuenta, Comando::Fundar { ejercito: col }).creadas[0];
        comun::gobernador_de_oficio(&mut self.n, cuenta, plaza);
        plaza
    }
    fn dar(&mut self, plaza: u64, recurso: Recurso, cantidad: i64) {
        self.ok("admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad });
    }
}

/// Lo que la orden creó: `(clase, id)`.
fn creadas(r: &pb::ResultadoOrden) -> Vec<(TipoEntidad, String)> {
    r.creadas.iter().map(|c| (TipoEntidad::try_from(c.entidad).unwrap(), c.id.clone())).collect()
}

fn aceptada(r: &pb::ResultadoOrden) {
    assert_eq!(r.resultado, R::OrdenAceptada as i32, "{r:?}");
}

/// Los ids de las columnas de una `Delta`.
fn ejercitos_de(d: &pb::Delta) -> Vec<String> {
    columnas_de(d).iter().map(|e| e.id.clone()).collect()
}

fn id(x: u64, prefijo: &str) -> String {
    format!("{prefijo}-{x}")
}

fn punto_pb(p: Punto) -> Option<pb::Punto> {
    Some(pb::Punto { x: p.x, y: p.y })
}

fn marchar_a(heroe: &str, p: Punto) -> orden::Cuerpo {
    use pb::orden_marchar_a::{objetivo::Tipo, Objetivo};
    orden::Cuerpo::MarcharA(pb::OrdenMarcharA { heroe_id: heroe.into(), objetivo: Some(Objetivo { tipo: Some(Tipo::Punto(pb::Punto { x: p.x, y: p.y })) }) })
}

fn s(x: &str) -> String {
    x.into()
}

// ───────────────────────────── Presencia y columnas ─────────────────────────────

#[tokio::test]
async fn un_jugador_nuevo_crea_su_heroe_pide_tropa_sale_del_campamento_y_marcha_por_el_contrato() {
    let m = con_mundo!();
    let w = arrancar_con(Prep::nuevo(m, false).n).await;
    let (mut carla, f) = w.entrar("carla", TipoCliente::ClienteUnity).await;
    assert!(f.entidades.is_empty());

    // Presencia: el héroe nace en un campamento (ids con la forma del contrato); uno por cuenta.
    let (_, r) = dar(&mut carla, "o1", orden::Cuerpo::CrearHeroe(pb::OrdenCrearHeroe { display_name: s("Carla"), campamento_id: s("camp-0"), class_definition_id: s("guerrero"), genero: s("femenino"), avatar: None })).await;
    aceptada(&r);
    let heroe = creadas(&r)[0].clone();
    assert_eq!(heroe.0, TipoEntidad::Heroe);
    let (_, r) = dar(&mut carla, "o2", orden::Cuerpo::CrearHeroe(pb::OrdenCrearHeroe { campamento_id: s("camp-0"), ..Default::default() })).await;
    assert_eq!(rechazo(&r), "heroe.ya_existe");
    let (_, r) = dar(&mut carla, "o3", orden::Cuerpo::PedirPrestamo(pb::OrdenPedirPrestamo { tropa_ids: vec![s("milicia_lanceros")] })).await;
    aceptada(&r);
    let escuadra = creadas(&r)[0].clone();
    assert_eq!(escuadra.0, TipoEntidad::Escuadron);

    // Columnas: sale del campamento con la escuadra; la columna llega en la `Delta` que precede al resultado. Vuelve a entrar y sale otra vez.
    let salir = |peticion: &str, escuadras: Vec<String>| (peticion.to_owned(), orden::Cuerpo::SalirDelCampamento(pb::OrdenSalirDelCampamento { campamento_id: s("camp-0"), heroe_id: heroe.1.clone(), escuadron_ids: escuadras, ..Default::default() }));
    let (p, cuerpo) = salir("o4", vec![escuadra.1.clone()]);
    let (deltas, r) = dar(&mut carla, &p, cuerpo).await;
    aceptada(&r);
    let columna = creadas(&r)[0].clone();
    assert_eq!(columna.0, TipoEntidad::Ejercito);
    assert_eq!((del_heroe(&deltas).len(), ejercitos_de(del_heroe(&deltas)[0])), (1, vec![columna.1.clone()]));
    let (_, r) = dar(&mut carla, "o5", orden::Cuerpo::EntrarEnCampamento(pb::OrdenEntrarEnCampamento { campamento_id: s("camp-0"), heroe_id: heroe.1.clone() })).await;
    aceptada(&r);
    // Dentro del campamento puede pedir salir del mundo y arrepentirse.
    let (_, r) = dar(&mut carla, "o6", orden::Cuerpo::Desconectarse(pb::OrdenDesconectarse { heroe_id: heroe.1.clone() })).await;
    aceptada(&r);
    let (_, r) = dar(&mut carla, "o7", orden::Cuerpo::Conectarse(pb::OrdenConectarse { heroe_id: heroe.1.clone() })).await;
    aceptada(&r);
    let (p, cuerpo) = salir("o8", vec![escuadra.1.clone()]);
    let (_, r) = dar(&mut carla, &p, cuerpo).await;
    aceptada(&r);
    let columna = creadas(&r)[0].clone();

    // `marchar_a` y `estacionar_ejercito`: el plan llega en la `Delta`. `repostar_en_campamento` sin campamento busca el que tiene a la puerta.
    let otro_campamento = campamento(&m.0, 1);
    let (deltas, r) = dar(&mut carla, "o9", marchar_a(&heroe.1, otro_campamento)).await;
    aceptada(&r);
    assert_eq!(columnas_de(del_heroe(&deltas).last().unwrap())[0].estado, pb::EstadoColumna::ColumnaMarchando as i32);
    let (_, r) = dar(&mut carla, "o10", orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: columna.1.clone() })).await;
    aceptada(&r);
    let (_, r) = dar(&mut carla, "o11", orden::Cuerpo::RepostarEnCampamento(pb::OrdenRepostarEnCampamento { campamento_id: None })).await;
    assert_eq!(rechazo(&r), "mercenarios.viveres_llenos", "sin campamento en la orden, el que tiene a la puerta (la marcha acaba de empezar): el núcleo dice que los víveres ya están llenos");
    let n = w.parar().await;
    assert_eq!(n.heroe_de_cuenta("carla").map(|h| ids::heroe(h.heroe.id)), Some(heroe.1));
}

// ───────────────────────────── Economía, acceso y murallas ─────────────────────────────

/// La plaza de ana (F1, nivel 2) junto a un nodo de cobre, con la auto-construcción parada y la zona agrandada hasta el techo del nivel. Devuelve el núcleo preparado, la plaza, su
/// centro y el índice del nodo.
fn plaza_de_ana(m: &M, unity: bool) -> (Prep, u64, Punto, u32) {
    let mut p = Prep::nuevo(m, unity);
    let mapa = Mapa::de(&m.0);
    let (cobre, sitio) = (0..mapa.nodos.len())
        .filter(|&i| mapa.nodos[i].tipo == Tipo::MinaCobre)
        .find_map(|i| sitio_junto_a(&p.n, mapa.nodos[i].pos, 6_000).map(|s| (i as u32, s)))
        .expect("hay un nodo de cobre junto a un sitio rico, lejos de los campamentos");
    let plaza = p.plaza("ana", 1, sitio);
    p.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
    p.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 2 });
    for _ in 0..14 {
        p.ok("admin", Comando::AgrandarZona { asentamiento: plaza });
    }
    (p, plaza, sitio, cobre)
}

#[tokio::test]
async fn la_economia_el_acceso_y_las_murallas_de_una_plaza_se_ordenan_por_el_contrato_con_la_autorizacion_del_nucleo() {
    let m = con_mundo!();
    let (mut p, plaza, _, _) = plaza_de_ana(m, false);
    for r in [Madera, Piedra, Oro, Trigo] {
        p.dar(plaza, r, 300_000);
    }
    p.columna("beto", 2, Punto::new(1_000, 1_000));
    let as_ = id(plaza, "as");
    // El Rey de F1 es el héroe de campo que fundó la plaza: fija su puerta sin estar dentro (5E).
    let rey = ids::heroe(p.n.faccion(1).and_then(|f| f.rey).expect("F1 tiene Rey"));
    let w = arrancar_con(p.n).await;
    let (mut ana, _) = w.entrar("ana", TipoCliente::ClienteUnity).await;
    let (mut beto, _) = w.entrar("beto", TipoCliente::ClienteUnity).await;

    // Economía: encola una Vivienda (el resultado lleva el edificio con la plaza), pausa y reanuda la auto-construcción y calibra la reserva. Beto no manda en la plaza.
    let (_, r) = dar(&mut ana, "e1", orden::Cuerpo::AnadirEdificioManualmente(pb::OrdenAnadirEdificioManualmente { asentamiento_id: as_.clone(), cargo: s("gobernador"), tipo: s("vivienda") })).await;
    aceptada(&r);
    let edificio = creadas(&r);
    assert_eq!((edificio.len(), edificio[0].0), (1, TipoEntidad::Edificio));
    assert!(edificio[0].1.starts_with(&format!("ed-{plaza}-")), "{edificio:?}");
    let (_, r) = dar(&mut beto, "e2", orden::Cuerpo::AnadirEdificioManualmente(pb::OrdenAnadirEdificioManualmente { asentamiento_id: as_.clone(), cargo: s("gobernador"), tipo: s("vivienda") })).await;
    assert_eq!(rechazo(&r), "no_autorizado");
    let (_, r) = dar(&mut ana, "e3", orden::Cuerpo::AlternarAutoConstruccion(pb::OrdenAlternarAutoConstruccion { asentamiento_id: as_.clone(), pausada: false })).await;
    aceptada(&r);
    let (_, r) = dar(&mut ana, "e4", orden::Cuerpo::CalibrarReservaManual(pb::OrdenCalibrarReservaManual { asentamiento_id: as_.clone(), recurso: s("trigo"), valor: 50_000 })).await;
    aceptada(&r);
    // Un edificio de otra plaza, un tipo fuera del catálogo y una dirección que no existe se rechazan sin llegar al núcleo o con el código de la entidad.
    let (_, r) = dar(&mut ana, "e5", orden::Cuerpo::QuitarDeCola(pb::OrdenQuitarDeCola { asentamiento_id: as_.clone(), cargo: s("gobernador"), edificio_id: s("ed-999-1") })).await;
    assert_eq!(rechazo(&r), "edificio.desconocido");
    let (_, r) = dar(&mut ana, "e6", orden::Cuerpo::AnadirEdificioManualmente(pb::OrdenAnadirEdificioManualmente { asentamiento_id: as_.clone(), cargo: s("gobernador"), tipo: s("castillo") })).await;
    assert_eq!(rechazo(&r), "orden.forma_invalida");

    // Puerta: el Rey de ana la cierra solo a los neutrales (por defecto estaba cerrada a neutrales y enemigos; `cerrada_a` lo dice todo, así que ahora se abre a los enemigos).
    let (_, r) = dar(&mut ana, "a1", orden::Cuerpo::FijarPuerta(pb::OrdenFijarPuerta { asentamiento_id: as_.clone(), heroe_id: rey, cerrada_a: vec![s("neutrales")] })).await;
    aceptada(&r);

    // Murallas: comprometer una empalizada devuelve el recinto con la plaza en su id; el núcleo decide si se mejora (la plaza de nivel 2, no) y, sin cerrar, se abandona. Uno que no existe se rechaza con el código de la entidad.
    let (_, r) = dar(&mut ana, "r1", orden::Cuerpo::ComprometerRecinto(pb::OrdenComprometerRecinto { asentamiento_id: as_.clone(), cargo: s("gobernador"), nivel: s("1") })).await;
    aceptada(&r);
    assert_eq!(creadas(&r), [(TipoEntidad::Recinto, format!("recinto-{plaza}-1"))]);
    let (_, r) = dar(&mut ana, "r2", orden::Cuerpo::MejorarRecinto(pb::OrdenMejorarRecinto { asentamiento_id: as_.clone(), cargo: s("maestroObras"), recinto_id: format!("recinto-{plaza}-1") })).await;
    assert_eq!(rechazo(&r), "murallas.nivel_de_plaza", "la plaza de nivel 2 no mejora aún su empalizada");
    let (_, r) = dar(&mut ana, "r3", orden::Cuerpo::MejorarRecinto(pb::OrdenMejorarRecinto { asentamiento_id: as_.clone(), cargo: s("maestroObras"), recinto_id: format!("recinto-{plaza}-77") })).await;
    assert_eq!(rechazo(&r), "murallas.recinto_desconocido");
    let (_, r) = dar(&mut ana, "r4", orden::Cuerpo::AbandonarRecinto(pb::OrdenAbandonarRecinto { asentamiento_id: as_.clone(), recinto_id: format!("recinto-{plaza}-1") })).await;
    aceptada(&r);
    let n = w.parar().await;
    assert_eq!(n.puerta_cerrada_a(plaza), Some(politica::Grupos::de([politica::Grupo::Neutrales])), "la orden fija la puerta entera: solo se cierra lo que dice `cerrada_a`");
    assert!(n.economia(plaza).unwrap().edificios.iter().any(|e| e.tipo == Tipo::Vivienda && e.id == edificio_n(&edificio[0].1)));
}

/// El número de un `ed-<plaza>-<n>`.
fn edificio_n(id: &str) -> u32 {
    ids::parse_edificio(id).expect("un edificio").1
}

// ───────────────────────────── Pueblos y avanzadas ─────────────────────────────

#[tokio::test]
async fn un_pueblo_se_marca_se_entrega_y_se_destruye_por_el_contrato_con_la_obra_como_obra_de_mapa() {
    let m = con_mundo!();
    let (mut p, plaza, _, cobre) = plaza_de_ana(m, false);
    // La columna de ana, a la puerta del nodo con el material de la obra (100 de madera y 12 de oro).
    let sitio = Mapa::de(&m.0).nodos[cobre as usize].pos;
    let (col, heroe) = p.columna("ana", 1, sitio);
    p.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 100_000 });
    p.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 12_000 });
    let (as_, h) = (id(plaza, "as"), ids::heroe(heroe));
    let w = arrancar_con(p.n).await;
    let (mut ana, _) = w.entrar("ana", TipoCliente::ClienteUnity).await;

    let (_, r) = dar(&mut ana, "p1", orden::Cuerpo::MarcarPueblo(pb::OrdenMarcarPueblo { asentamiento_id: as_.clone(), heroe_id: h.clone(), nodo_id: ids::nodo(cobre) })).await;
    aceptada(&r);
    let obra = creadas(&r)[0].clone();
    assert_eq!(obra.0, TipoEntidad::ObraDeMapa, "mientras no se levanta, el pueblo es una obra de mapa");
    // Entregar los materiales a esa obra: el resultado llega con la `Delta` de la columna, que ya no los lleva.
    let (deltas, r) = dar(&mut ana, "p2", orden::Cuerpo::EntregarEnObra(pb::OrdenEntregarEnObra { heroe_id: h.clone(), obra_id: obra.1.clone(), materiales: None })).await;
    aceptada(&r);
    assert_eq!(del_heroe(&deltas).len(), 1);
    assert!(columnas_de(del_heroe(&deltas)[0])[0].suministro.as_ref().unwrap().por_id.is_empty(), "la columna entregó todo lo que llevaba");
    // La obra de un pueblo no se cancela con `cancelar_obra`; se destruye.
    let (_, r) = dar(&mut ana, "p3", orden::Cuerpo::CancelarObra(pb::OrdenCancelarObra { heroe_id: h.clone(), obra_id: obra.1.clone() })).await;
    assert_eq!(rechazo(&r), "objetivo.no_soportado");
    let pueblo = id(ids::parse_obra(&obra.1).unwrap(), "pueblo");
    let (_, r) = dar(&mut ana, "p4", orden::Cuerpo::DestruirPueblo(pb::OrdenDestruirPueblo { heroe_id: h.clone(), pueblo_id: pueblo.clone() })).await;
    aceptada(&r);
    let (_, r) = dar(&mut ana, "p5", orden::Cuerpo::DestruirPueblo(pb::OrdenDestruirPueblo { heroe_id: h, pueblo_id: pueblo })).await;
    assert_eq!(rechazo(&r), "pueblo.desconocido");
    let n = w.parar().await;
    assert!(n.pueblos_de(plaza).is_empty());
}

#[tokio::test]
async fn una_avanzada_se_coloca_se_entrega_y_se_destruye_por_el_contrato() {
    let m = con_mundo!();
    let (mut p, plaza, centro, _) = plaza_de_ana(m, false);
    let sitio = Punto::new(centro.x + 8_000, centro.y);
    let (col, heroe) = p.columna("ana", 1, sitio);
    for (r, n) in [(Madera, 150_000), (Piedra, 150_000), (Oro, 50_000)] {
        p.ok("admin", Comando::DarSuministro { ejercito: col, recurso: r, cantidad: n });
    }
    let (as_, h) = (id(plaza, "as"), ids::heroe(heroe));
    // 5E-1: la coloca el Maestro de Obras de la madre, residente, con su columna en el sitio.
    comun::coloca_avanzadas(&mut p.n, plaza, heroe);
    // Cancelar y destruir son del panel de la plaza madre (5E): los da su Gobernador, dentro de ella (el de oficio de ana).
    let gobernador = ids::heroe(p.n.estado().asentamientos[&plaza].cargos.titular(asentamiento::Cargo::Gobernador).expect("tiene Gobernador"));
    let w = arrancar_con(p.n).await;
    let (mut ana, _) = w.entrar("ana", TipoCliente::ClienteUnity).await;

    let (_, r) = dar(&mut ana, "v1", orden::Cuerpo::ColocarAvanzada(pb::OrdenColocarAvanzada { asentamiento_id: as_, heroe_id: h.clone(), posicion: punto_pb(sitio) })).await;
    aceptada(&r);
    let obra = creadas(&r)[0].clone();
    assert_eq!(obra.0, TipoEntidad::ObraDeMapa);
    let (_, r) = dar(&mut ana, "v2", orden::Cuerpo::EntregarEnObra(pb::OrdenEntregarEnObra { heroe_id: h.clone(), obra_id: obra.1.clone(), materiales: None })).await;
    aceptada(&r);
    // La primera obra no se cancela (se destruye la avanzada): el código es el del núcleo, que encontró la avanzada tras `obra-N`.
    let (_, r) = dar(&mut ana, "v3", orden::Cuerpo::CancelarObra(pb::OrdenCancelarObra { heroe_id: h.clone(), obra_id: obra.1.clone() })).await;
    assert_eq!(rechazo(&r), "no_autorizado", "el héroe de la columna no es su Gobernador ni está dentro de la madre");
    let (_, r) = dar(&mut ana, "v3b", orden::Cuerpo::CancelarObra(pb::OrdenCancelarObra { heroe_id: gobernador.clone(), obra_id: obra.1.clone() })).await;
    assert_eq!(rechazo(&r), "avanzada.nada_que_cancelar");
    let avanzada = id(ids::parse_obra(&obra.1).unwrap(), "av");
    let (_, r) = dar(&mut ana, "v4", orden::Cuerpo::DestruirAvanzada(pb::OrdenDestruirAvanzada { heroe_id: gobernador, avanzada_id: avanzada.clone() })).await;
    aceptada(&r);
    let (_, r) = dar(&mut ana, "v5", orden::Cuerpo::MejorarAvanzada(pb::OrdenMejorarAvanzada { heroe_id: s("heroe-1"), avanzada_id: avanzada })).await;
    assert_eq!(rechazo(&r), "avanzada.desconocida");
    let n = w.parar().await;
    assert!(n.avanzadas_de(plaza).is_empty());
}

// ───────────────────────────── Comercio ─────────────────────────────

#[tokio::test]
async fn el_comercio_se_ordena_por_el_contrato_entre_dos_plazas_con_mercado() {
    let m = con_mundo!();
    let mut p = Prep::nuevo(m, false);
    let (sa, sb) = (sitio_rico(&p.n, &m.0, 0, 1), sitio_rico(&p.n, &m.0, 3, 2));
    let (a, b) = (p.plaza("ana", 1, sa), p.plaza("beto", 2, sb));
    for plaza in [a, b] {
        p.dar(plaza, Madera, 350_000);
        p.ok("admin", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
    }
    p.avanzar(9 * HORA);
    let (as_a, as_b) = (id(a, "as"), id(b, "as"));
    let w = arrancar_con(p.n).await;
    let (mut ana, _) = w.entrar("ana", TipoCliente::ClienteUnity).await;
    let (mut beto, _) = w.entrar("beto", TipoCliente::ClienteUnity).await;

    // Una orden de mercado se coloca y se retira (la orden 145 del contrato).
    let (_, r) = dar(&mut ana, "c1", orden::Cuerpo::ColocarOrdenMercado(pb::OrdenColocarOrdenMercado { asentamiento_id: as_a.clone(), tipo: s("venta"), recurso: s("madera"), cantidad: 10_000, precio: None })).await;
    aceptada(&r);
    let orden_de_mercado = creadas(&r)[0].clone();
    assert_eq!(orden_de_mercado.0, TipoEntidad::OrdenMercado);
    let (_, r) = dar(&mut beto, "c2", orden::Cuerpo::CancelarOrdenMercado(pb::OrdenCancelarOrdenMercado { orden_id: orden_de_mercado.1.clone() })).await;
    assert_eq!(rechazo(&r), "no_autorizado", "solo quien manda en la plaza retira su orden");
    let (_, r) = dar(&mut ana, "c3", orden::Cuerpo::CancelarOrdenMercado(pb::OrdenCancelarOrdenMercado { orden_id: orden_de_mercado.1 })).await;
    aceptada(&r);

    // Un trueque propuesto por ana lo acepta o rechaza beto (la plaza B).
    let trueque = |recurso: &str, cantidad: i64| vec![(recurso.to_owned(), cantidad)];
    let lineas_a: Vec<_> = trueque("madera", 10_000).into_iter().map(|(recurso, cantidad)| pb::orden_proponer_trueque::LineasA { recurso, cantidad }).collect();
    let lineas_b: Vec<_> = trueque("piedra", 5_000).into_iter().map(|(recurso, cantidad)| pb::orden_proponer_trueque::LineasB { recurso, cantidad }).collect();
    let (_, r) = dar(&mut ana, "c4", orden::Cuerpo::ProponerTrueque(pb::OrdenProponerTrueque { asentamiento_a_id: as_a.clone(), lineas_a, asentamiento_b_id: as_b, lineas_b })).await;
    aceptada(&r);
    let acuerdo = creadas(&r)[0].clone();
    assert_eq!(acuerdo.0, TipoEntidad::AcuerdoTrueque);
    let (_, r) = dar(&mut ana, "c5", orden::Cuerpo::RechazarTrueque(pb::OrdenRechazarTrueque { acuerdo_id: acuerdo.1.clone() })).await;
    assert_eq!(rechazo(&r), "no_autorizado", "el trueque lo responde la plaza B");
    let (_, r) = dar(&mut beto, "c6", orden::Cuerpo::RechazarTrueque(pb::OrdenRechazarTrueque { acuerdo_id: acuerdo.1 })).await;
    aceptada(&r);

    // Una caravana se crea, se le monta un carro con su buey y se reserva (queda fuera del reparto automático).
    let (_, r) = dar(&mut ana, "c7", orden::Cuerpo::CrearCaravana(pb::OrdenCrearCaravana { asentamiento_id: as_a })).await;
    aceptada(&r);
    let caravana = creadas(&r)[0].clone();
    assert_eq!(caravana.0, TipoEntidad::Caravana);
    let (_, r) = dar(&mut ana, "c8", orden::Cuerpo::AgregarCarroCaravana(pb::OrdenAgregarCarroCaravana { caravana_id: caravana.1.clone(), tipo_carro: s("basico") })).await;
    aceptada(&r);
    let (_, r) = dar(&mut ana, "c9", orden::Cuerpo::ComprarAnimalCaravana(pb::OrdenComprarAnimalCaravana { caravana_id: caravana.1.clone(), carro_indice: 0, tipo_animal: s("buey") })).await;
    aceptada(&r);
    let (_, r) = dar(&mut ana, "c10", orden::Cuerpo::ReservarCaravana(pb::OrdenReservarCaravana { caravana_id: caravana.1.clone(), reservada: true })).await;
    aceptada(&r);
    let (_, r) = dar(&mut ana, "c11", orden::Cuerpo::CancelarCaravana(pb::OrdenCancelarCaravana { caravana_id: caravana.1 })).await;
    assert_eq!(rechazo(&r), "caravana.no_preparando", "solo se cancela una caravana que se está preparando");
    let n = w.parar().await;
    assert_eq!(n.flota_de(a).len(), 1);
}

// ───────────────────────────── Batalla de Unity ─────────────────────────────

#[tokio::test]
async fn quien_esta_dentro_de_una_plaza_asediada_se_une_a_la_batalla_de_unity_por_su_uuid() {
    let m = con_mundo!();
    let mut p = Prep::nuevo(m, true);
    let (sa, sb) = (sitio_rico(&p.n, &m.0, 0, 1), sitio_rico(&p.n, &m.0, 3, 2));
    let (_, b) = (p.plaza("ana", 1, sa), p.plaza("beto", 2, sb));
    p.avanzar(24 * HORA);
    // Pasado el día de protección con que nace la plaza (Doc 1.2), beto (humano, F2) defiende en persona dentro de su plaza con una escuadra; ana (humana, F1) la asedia con un ejército a su puerta.
    let campamento = (0..m.0.campamentos().len()).min_by_key(|&i| campamento(&m.0, i).dist2(sb)).unwrap() as u32;
    let beto = p.ok("beto", Comando::CrearHeroe { cuenta: "beto".into(), campamento, controlador: Controlador::Humano, nombre: None }).creadas[0];
    p.ok("admin", Comando::FijarFaccionDeHeroe { heroe: beto, faccion: Some(2) });
    p.ok("admin", Comando::FijarResidencia { heroe: beto, residencia: Some(Residencia::Asentamiento(b as u32)) });
    p.ok("admin", Comando::ColocarHeroe { heroe: beto, ubicacion: Ubicacion::Asentamiento(b as u32) });
    let cantidad = |t: Tropa| militar::Reglas::cargar().tropas.tropa(t).unidades;
    let defensa = p.ok("admin", Comando::DarEscuadra { heroe: beto, tropa: Tropa::MiliciaLanceros, cantidad: cantidad(Tropa::MiliciaLanceros) }).creadas[0];
    let loadout = p.n.heroe(beto).unwrap().heroe.loadouts[0].id;
    p.ok("admin", Comando::GuardarLoadout { heroe: beto, loadout: Some(loadout), nombre: "Default".into(), escuadras: vec![defensa], perks: vec![], activo: Some(true) });
    let puerta = p.n.estado().asentamientos[&b].posicion;
    let (_, ana) = p.columna("ana", 1, Punto::new(puerta.x + 2_000, puerta.y));
    for t in [Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto] {
        p.ok("admin", Comando::DarEscuadra { heroe: ana, tropa: t, cantidad: cantidad(t) });
    }
    p.ok("admin", Comando::FijarControlador { heroe: ana, controlador: Controlador::Humano });
    let t = p.n.ahora();
    let r = p.n.aplicar_en(t, Peticion::nueva("ana", "asedio", Comando::Atacar { heroe: ana, objetivo: nucleo::ObjetivoAtaque::Asentamiento(b), declara_guerra: true }));
    assert!(r.aceptada(), "{r:?}");
    let uuid = p.n.uuid_de_batalla(p.n.batallas().next().expect("se abrió la batalla del asedio").id);
    let w = arrancar_con(p.n).await;
    let (mut c_beto, _) = w.entrar("beto", TipoCliente::ClienteUnity).await;

    // Un UUID que no es de esta partida no existe; el de la batalla del asedio sí, y el residente que está dentro se une a la defensa (sin lado: es el defensor).
    let unirse = |battle_id: &str| orden::Cuerpo::UnirseABatalla(pb::OrdenUnirseABatalla { heroe_id: ids::heroe(beto), battle_id: battle_id.into(), lado: None });
    let (_, r) = dar(&mut c_beto, "u1", unirse("00000000-0000-0000-0000-000000000000")).await;
    assert_eq!(rechazo(&r), "batalla.no_existe");
    let (_, r) = dar(&mut c_beto, "u2", unirse(&uuid)).await;
    aceptada(&r);
    // Quien ya está en la batalla no puede cancelarla: solo quien la inició (y ana no es la cuenta de este héroe).
    let (_, r) = dar(&mut c_beto, "u3", orden::Cuerpo::CancelarBatalla(pb::OrdenCancelarBatalla { battle_id: uuid })).await;
    assert_eq!(rechazo(&r), "batalla.invalida");
    let n = w.parar().await;
    assert!(n.heroe_en_batalla(beto), "beto defiende en la batalla del asedio");
}

// ───────────────────────────── Repostar sin decir dónde ─────────────────────────────

#[tokio::test]
async fn repostar_sin_lugar_busca_la_plaza_a_la_puerta_de_la_columna_y_sin_ninguna_lo_dice() {
    let m = con_mundo!();
    let (mut p, plaza, centro, _) = plaza_de_ana(m, false);
    p.dar(plaza, Trigo, 400_000);
    let (cerca, h_cerca) = p.columna("ana", 1, Punto::new(centro.x + 1_000, centro.y));
    let (_, h_lejos) = p.columna("ana", 1, Punto::new(centro.x + 12_000, centro.y));
    let viveres = |p: &Prep, col: u64| p.n.ejercito(col).unwrap().columna.viveres.valor_en(p.n.ahora());
    let antes = viveres(&p, cerca);
    let w = arrancar_con(p.n).await;
    let (mut ana, _) = w.entrar("ana", TipoCliente::ClienteUnity).await;

    let repostar = |heroe: u64, lugar: Option<&str>| orden::Cuerpo::Repostar(pb::OrdenRepostar { heroe_id: ids::heroe(heroe), lugar_id: lugar.map(String::from) });
    let (_, r) = dar(&mut ana, "r1", repostar(h_lejos, None)).await;
    assert_eq!(rechazo(&r), "repostar.sin_lugar", "a 12 u de la plaza no hay ninguna a la puerta");
    let (deltas, r) = dar(&mut ana, "r2", repostar(h_cerca, None)).await;
    aceptada(&r);
    assert_eq!(del_heroe(&deltas).len(), 1, "la columna llega con más víveres antes del resultado");
    // Con el lugar dicho vale lo mismo (la columna ya está llena: lo dice el núcleo), y una avanzada que no existe no se inventa.
    let (_, r) = dar(&mut ana, "r3", repostar(h_cerca, Some(&id(plaza, "as")))).await;
    assert_eq!(rechazo(&r), "columna.viveres_llenos");
    let (_, r) = dar(&mut ana, "r4", repostar(h_cerca, Some("av-999"))).await;
    assert_eq!(rechazo(&r), "avanzada.desconocida");
    let n = w.parar().await;
    assert!(n.ejercito(cerca).unwrap().columna.viveres.valor_en(n.ahora()) > antes);
}
