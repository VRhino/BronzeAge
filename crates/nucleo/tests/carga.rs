//! La carga de las sesiones (4H T6): 500 sesiones contra el actor del núcleo, sin WebSocket (los mensajes se cuentan al salir del actor, codificados como los mandaría el servidor y comprimidos
//! con zstd como él, que comprime toda trama de 256 bytes o más). Mide el coste del reparto (el actor con 500 sesiones frente al actor con la sola administración que mueve el reloj y frente al
//! núcleo sin actor), el muestreo de `VISTA` y los bytes por minuto de cada clase de sesión con la economía en escalones.
//!
//! `cargo test --release -p nucleo --test carga -- --ignored --nocapture`. Sobre el artefacto de mundo real (se omite si no está).

mod comun;

use base::{Instante, Mil, Punto, Rng, MS_POR_MINUTO};
use comun::real::{self, campamento, en_tierra, fundar, sitio_rico};
use contrato::v1::{self as pb, mensaje_de_servidor as ms, orden, TipoCliente};
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::proyeccion::ids;
use nucleo::{sesiones, AccionReloj, Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas};
use prost::Message;
use servidor::{ComandoSesion, Difusion, Identidad, RelojCompartido};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant as Pared;
use tokio::sync::{mpsc, oneshot};

const U: Mil = 1_000;
const FACCIONES: u32 = 30;
const EN_PLAZA: usize = 45;
const EN_EL_MAPA: usize = 450;
const ADMINS: usize = 5;
const MINUTOS: i64 = 5;
const PASO_MS: i64 = 250;

/// El mundo de la medición: una plaza por Facción (30, con auto-construcción y recursos: crecen y su almacén cambia cada minuto), 45 héroes dentro de su plaza y 450 columnas de campo que marchan
/// a ±300 u de su plaza (una cuenta por columna). El reloj, parado.
struct Mundo500 {
    n: Nucleo,
    /// La cuenta de cada jugador del mapa, su columna y el héroe que la lleva.
    mapa: Vec<(String, u64, u64)>,
    /// La cuenta de cada jugador dentro de su plaza.
    plaza: Vec<String>,
}

fn montar() -> Option<Mundo500> {
    let m = real::mundo()?;
    let mut reglas = Reglas::default();
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let mut n = Nucleo::nuevo(Arranque { reglas, ajustes: Ajustes::default(), semilla: 5, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
    let total = m.0.campamentos().len();
    let mut plazas = Vec::new();
    for f in 1..=FACCIONES {
        let i = (f as usize - 1) % total;
        let p = sitio_rico(&n, &m.0, i, f);
        let a = fundar(&mut n, &format!("fundador{f}"), f, p);
        for r in [asentamiento::Recurso::Madera, asentamiento::Recurso::Piedra, asentamiento::Recurso::Trigo, asentamiento::Recurso::Oro] {
            assert!(n.aplicar_en(0, Peticion::nueva("admin", &format!("r{f}{r:?}"), Comando::DarRecursos { asentamiento: a, recurso: r, cantidad: 2_000_000 }).de_admin()).aceptada());
        }
        plazas.push((a, p, i as u32));
    }
    let ok = |n: &mut Nucleo, actor: &str, id: String, c: Comando| {
        let p = Peticion::nueva(actor, &id, c);
        // Andamiaje (5E): una orden de plaza de quien mandaba en ella antes del sistema 5 se da como administración (`comun::mandaba`).
        let r = n.aplicar_en(0, if actor == "admin" || comun::mandaba(&n, actor, &p.comando) { p.de_admin() } else { p });
        assert!(r.aceptada(), "{r:?}");
        r
    };
    let mut en_plaza = Vec::new();
    for k in 0..EN_PLAZA {
        let (a, _, i) = plazas[k % plazas.len()];
        let cuenta = format!("p{k}");
        let h = ok(&mut n, &cuenta, format!("h{k}"), Comando::CrearHeroe { cuenta: cuenta.clone(), campamento: i, controlador: Controlador::Humano, nombre: None }).creadas[0];
        ok(&mut n, "admin", format!("hf{k}"), Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some((k % plazas.len()) as u32 + 1) });
        ok(&mut n, "admin", format!("hr{k}"), Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(a as u32)) });
        ok(&mut n, "admin", format!("hc{k}"), Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(a as u32) });
        en_plaza.push(cuenta);
    }
    let mut rng = Rng::nuevo(17);
    let mut mapa = Vec::new();
    for k in 0..EN_EL_MAPA {
        let (_, centro, _) = plazas[k % plazas.len()];
        let cuenta = format!("j{k}");
        let desde = en_tierra(&m.0, centro, rng.por_debajo(200 * U as u64) as Mil - 100 * U, rng.por_debajo(200 * U as u64) as Mil - 100 * U);
        let col = n.crear_ejercito(0, &cuenta, (k % plazas.len()) as u32 + 1, desde, 500, 350_000).unwrap();
        let h = n.ejercito(col).unwrap().columna.lider;
        ok(&mut n, "admin", format!("e{k}"), Comando::DarEscuadra { heroe: h, tropa: Tropa::MiliciaLanceros, cantidad: 10 });
        mapa.push((cuenta, col, h));
    }
    // Diez minutos para que las plazas arranquen y, después, cada columna a un destino a ±300 u de su plaza: marchan durante toda la medición.
    n.avanzar_hasta(10 * MS_POR_MINUTO + 1_000);
    let t = n.ahora();
    for (k, (cuenta, col, _)) in mapa.iter().enumerate() {
        let (_, centro, _) = plazas[k % plazas.len()];
        // Un destino en tierra al que se llegue por tierra (los rechazados no dejan rastro).
        for intento in 0.. {
            assert!(intento < 100, "sin destino para la columna {k}");
            let hacia = Punto::new(centro.x + rng.por_debajo(600 * U as u64) as Mil - 300 * U, centro.y + rng.por_debajo(600 * U as u64) as Mil - 300 * U);
            if m.0.transitable_en(hacia.x, hacia.y) && n.aplicar_en(t, Peticion::nueva(cuenta, &format!("m{k}-{intento}"), Comando::MarcharA { ejercito: *col, destino: hacia })).aceptada() {
                break;
            }
        }
    }
    ok(&mut n, "admin", "pausa".into(), Comando::Reloj(AccionReloj::Pausar));
    assert_eq!(t, n.ahora());
    Some(Mundo500 { n, mapa, plaza: en_plaza })
}

// ───────────────────────────── Las sesiones, sin red ─────────────────────────────

/// Lo que llegó a una sesión: bytes por tipo de ámbito (`Foto` y `Delta`), en crudo y comprimidos como los mandaría el servidor, y mensajes; la niebla aparte.
#[derive(Default, Clone)]
struct Contador {
    crudo: BTreeMap<String, usize>,
    zstd: BTreeMap<String, usize>,
    mensajes: usize,
}

fn tipo_de(d: &Difusion) -> String {
    let a = match d {
        Difusion::Foto(f) => &f.ambito,
        Difusion::Delta(x) => &x.ambito,
        _ => return "otros".into(),
    };
    let a = a.as_ref().expect("un ámbito");
    let tipo = pb::AmbitoTipo::try_from(a.tipo).unwrap().as_str_name().trim_start_matches("AMBITO_").to_owned();
    if matches!(d, Difusion::Foto(_)) { format!("foto {tipo}") } else { tipo }
}

fn cuerpo(d: Difusion) -> Option<ms::Cuerpo> {
    Some(match d {
        Difusion::Foto(f) => ms::Cuerpo::Foto(f),
        Difusion::Delta(x) => ms::Cuerpo::Delta(x),
        Difusion::Resultado(r) => ms::Cuerpo::Resultado(r),
        Difusion::Revocado(r) => ms::Cuerpo::Revocado(r),
        Difusion::Aviso(a) => ms::Cuerpo::Aviso(a),
        Difusion::Batalla(b) => ms::Cuerpo::Batalla(b),
        Difusion::Cronica(c) => ms::Cuerpo::Cronica(c),
        Difusion::Cerrar(_) => return None,
    })
}

/// Lee todo lo que llega a una sesión y lo cuenta; los resultados de las órdenes van además a `resultados` (la administración que mueve el reloj los espera).
async fn drenar(mut rx: mpsc::Receiver<Difusion>, cuenta: Arc<Mutex<Contador>>, resultados: Option<mpsc::UnboundedSender<pb::ResultadoOrden>>) {
    let mut secuencia = 1;
    while let Some(d) = rx.recv().await {
        // Los avisos del reloj son de la medición (la administración lo avanza a pasos de 250 ms): en vivo no los hay.
        if matches!(&d, Difusion::Aviso(a) if a.codigo == "reloj.cambiado") {
            continue;
        }
        let tipo = tipo_de(&d);
        let es_niebla = matches!(&d, Difusion::Delta(x) if x.niebla.is_some());
        let Some(c) = cuerpo(d) else { continue };
        if let (Some(tx), ms::Cuerpo::Resultado(r)) = (&resultados, &c) {
            let _ = tx.send(r.clone());
        }
        secuencia += 1;
        let bytes = pb::MensajeDeServidor { secuencia, cuerpo: Some(c), ..Default::default() }.encode_to_vec();
        let z = if bytes.len() >= 256 { zstd::bulk::compress(&bytes, 3).unwrap().len() } else { bytes.len() };
        let mut k = cuenta.lock().unwrap();
        k.mensajes += 1;
        *k.crudo.entry(tipo.clone()).or_default() += bytes.len();
        *k.zstd.entry(tipo.clone()).or_default() += z;
        if es_niebla {
            *k.crudo.entry(format!("{tipo} con niebla")).or_default() += bytes.len();
            *k.zstd.entry(format!("{tipo} con niebla")).or_default() += z;
        }
    }
}

struct Sesion {
    clase: &'static str,
    cuenta: Arc<Mutex<Contador>>,
}

/// El actor con sus sesiones: la administración que mueve el reloj (la sesión 1) y las demás.
struct Banco {
    tx: mpsc::Sender<ComandoSesion>,
    actor: tokio::task::JoinHandle<Nucleo>,
    sesiones: Vec<Sesion>,
    resultados: mpsc::UnboundedReceiver<pb::ResultadoOrden>,
    peticiones: u64,
}

impl Banco {
    async fn conectar(&mut self, cuenta: &str, tipo: TipoCliente, clase: &'static str, rect: Option<(Punto, Punto)>, resultados: Option<mpsc::UnboundedSender<pb::ResultadoOrden>>) {
        let sesion = self.sesiones.len() as u64 + 1;
        let (dtx, drx) = mpsc::channel(1 << 16);
        let contador = Arc::new(Mutex::new(Contador::default()));
        tokio::spawn(drenar(drx, contador.clone(), resultados));
        let (rtx, rrx) = oneshot::channel();
        let identidad = Identidad { cliente_id: cuenta.into(), roles: vec![tipo], rol: servidor::RolTecnico::AdministradorPartida };
        self.tx.send(ComandoSesion::Conecta { sesion, identidad, hola: servidor::prueba::hola(&format!("ok:{cuenta}"), tipo), difusion: dtx, respuesta: rtx }).await.unwrap();
        rrx.await.unwrap().expect("bienvenida");
        if let Some((a, b)) = rect {
            let rect = pb::Rectangulo { min: Some(pb::Punto { x: a.x, y: a.y }), max: Some(pb::Punto { x: b.x, y: b.y }) };
            self.tx.send(ComandoSesion::Vista { sesion, rect }).await.unwrap();
        }
        self.sesiones.push(Sesion { clase, cuenta: contador });
    }

    /// Una orden de la sesión `sesion` (sin esperar su resultado, salvo la de la administración, que es la 1).
    async fn orden(&mut self, sesion: u64, cuerpo: orden::Cuerpo) {
        self.peticiones += 1;
        let orden = pb::Orden { peticion_id: format!("c{}", self.peticiones), cuerpo: Some(cuerpo), ..Default::default() };
        self.tx.send(ComandoSesion::Orden { sesion, orden }).await.unwrap();
        if sesion == 1 {
            let r = self.resultados.recv().await.unwrap();
            assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
        }
    }

    async fn reloj_hasta(&mut self, t: Instante) {
        self.orden(1, orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: "avanzar_hasta".into(), factor_milesimas: None, hasta_ms: Some(t) })).await;
    }

    /// Lo que llegó a las sesiones de cada clase hasta ahora, y la vuelve a cero.
    fn recoger(&self) -> BTreeMap<&'static str, (usize, Contador)> {
        let mut r: BTreeMap<&'static str, (usize, Contador)> = BTreeMap::new();
        for s in &self.sesiones {
            let k = std::mem::take(&mut *s.cuenta.lock().unwrap());
            let e = r.entry(s.clase).or_default();
            e.0 += 1;
            e.1.mensajes += k.mensajes;
            for (t, b) in k.crudo {
                *e.1.crudo.entry(t).or_default() += b;
            }
            for (t, b) in k.zstd {
                *e.1.zstd.entry(t).or_default() += b;
            }
        }
        r
    }
}

async fn arrancar(n: Nucleo) -> Banco {
    let reloj = RelojCompartido::nuevo(n.ahora());
    let (tx, rx) = mpsc::channel(1 << 16);
    let actor = sesiones::lanzar(n, reloj, rx);
    let (rtx, resultados) = mpsc::unbounded_channel();
    let mut b = Banco { tx, actor, sesiones: Vec::new(), resultados, peticiones: 0 };
    b.conectar("admin", TipoCliente::ClienteAdminWeb, "administración", None, Some(rtx)).await;
    b
}

/// Espera a que el actor haya atendido todo lo anterior (un `controlar_reloj` sin avance no se puede: se usa una orden de administración rechazada).
async fn barrera(b: &mut Banco) {
    b.peticiones += 1;
    let orden = pb::Orden { peticion_id: format!("barrera{}", b.peticiones), cuerpo: Some(orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: "ej-999999".into() })), ..Default::default() };
    b.tx.send(ComandoSesion::Orden { sesion: 1, orden }).await.unwrap();
    b.resultados.recv().await.unwrap();
}

fn imprimir(titulo: &str, recogido: &BTreeMap<&'static str, (usize, Contador)>, minutos: f64) {
    println!("{titulo}");
    for (clase, (cuantas, k)) in recogido {
        let total: usize = k.zstd.iter().filter(|(t, _)| !t.ends_with("con niebla")).map(|(_, b)| b).sum();
        let crudo: usize = k.crudo.iter().filter(|(t, _)| !t.ends_with("con niebla")).map(|(_, b)| b).sum();
        let por = |b: usize| b as f64 / *cuantas as f64 / minutos / 1024.0;
        let desglose: Vec<String> = k.zstd.iter().map(|(t, b)| format!("{t} {:.1}", por(*b))).collect();
        println!(
            "  {clase} ({cuantas} sesiones): {:.1} KB/min por sesión con zstd ({:.1} KB/min en crudo), {:.0} mensajes/min por sesión; por tipo (KB/min con zstd): {}",
            por(total),
            por(crudo),
            k.mensajes as f64 / *cuantas as f64 / minutos,
            desglose.join(", ")
        );
    }
}

/// El mismo guion para todos: `MINUTOS` minutos de mundo en pasos de 250 ms (lo que el actor despierta en vivo con alguna `VISTA` abierta), con lo que el mundo hace solo (las plazas crecen,
/// las columnas marchan y llegan). Devuelve los segundos de pared.
async fn guion(b: &mut Banco, desde: Instante) -> f64 {
    let i0 = Pared::now();
    let mut t = desde;
    while t < desde + MINUTOS * MS_POR_MINUTO {
        t += PASO_MS;
        b.reloj_hasta(t).await;
    }
    i0.elapsed().as_secs_f64()
}

/// Conecta las 499 sesiones que acompañan a la administración que mueve el reloj: 4 más de administración, 45 jugadores en su plaza (`HEROE`, `FACCION`, `ASENTAMIENTO`) y 450 en el mapa (`HEROE`,
/// `FACCION` y `VISTA` de 600 × 600 u alrededor del campamento junto al que está su plaza). Devuelve los segundos de pared.
async fn conectar_499(b: &mut Banco, mapa: &[(String, u64, u64)], en_plaza: &[String]) -> f64 {
    let m = real::mundo().unwrap();
    let i0 = Pared::now();
    for k in 1..ADMINS {
        b.conectar(&format!("admin{k}"), TipoCliente::ClienteAdminWeb, "administración", None, None).await;
    }
    for cuenta in en_plaza {
        b.conectar(cuenta, TipoCliente::ClienteUnity, "jugador en su plaza", None, None).await;
    }
    for (k, (cuenta, _, _)) in mapa.iter().enumerate() {
        let c = campamento(&m.0, (k % FACCIONES as usize) % m.0.campamentos().len());
        let rect = (Punto::new(c.x - 300 * U, c.y - 300 * U), Punto::new(c.x + 300 * U, c.y + 300 * U));
        b.conectar(cuenta, TipoCliente::ClienteUnity, "jugador en el mapa", Some(rect), None).await;
    }
    barrera(b).await;
    i0.elapsed().as_secs_f64()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "medición"]
async fn carga_de_500_sesiones_coste_del_reparto_muestreo_de_la_vista_y_bytes_por_minuto() {
    let Some(base) = montar() else {
        eprintln!("sin artefacto de mundo real: se omite");
        return;
    };
    let t0 = base.n.ahora();
    let (mapa, en_plaza) = (base.mapa.clone(), base.plaza.clone());
    println!("mundo: {FACCIONES} plazas, {EN_PLAZA} héroes en su plaza, {EN_EL_MAPA} columnas que marchan; versión {}", base.n.version());

    // 1. El núcleo solo (sin actor ni pertenencias), los mismos pasos.
    let mut n = montar().unwrap().n;
    let i0 = Pared::now();
    let mut t = t0;
    while t < t0 + MINUTOS * MS_POR_MINUTO {
        t += PASO_MS;
        n.avanzar_hasta(t);
    }
    let solo = i0.elapsed().as_secs_f64();

    // 2. El actor con la sola administración que mueve el reloj.
    let mut b = arrancar(montar().unwrap().n).await;
    barrera(&mut b).await;
    b.recoger();
    let un_admin = guion(&mut b, t0).await;
    barrera(&mut b).await;
    let solo_admin = b.recoger();
    drop(b.tx);
    let n1 = b.actor.await.unwrap();

    // 3. El actor con 500 sesiones, el mismo guion: la misma versión y el mismo hash.
    let mut b = arrancar(base.n).await;
    let conexion = conectar_499(&mut b, &mapa, &en_plaza).await;
    let fotos = b.recoger();
    let quinientas = guion(&mut b, t0).await;
    barrera(&mut b).await;
    let en_marcha = b.recoger();
    drop(b.tx);
    let n500 = b.actor.await.unwrap();
    assert_eq!((n500.version(), n500.hash_estado()), (n1.version(), n1.hash_estado()), "la versión y el hash no dependen de las sesiones");
    let versiones = n500.version();

    // 4. Una orden por jugador del mapa (cambia de destino) con las 500 conectadas: con el reloj parado (todas las vistas al momento tras cada orden) y en marcha a ×1 (al momento solo la
    // de quien la da; las demás, en el muestreo de 250 ms: decisión del autor del 2026-10-24).
    let m = real::mundo().unwrap();
    let mut b = arrancar(n500).await;
    conectar_499(&mut b, &mapa, &en_plaza).await;
    b.recoger();
    let mut rng = Rng::nuevo(23);
    let mut ronda = || {
        let ordenes: Vec<(u64, orden::Cuerpo)> = mapa
            .iter()
            .enumerate()
            .map(|(k, (_, _, h))| {
                let c = campamento(&m.0, (k % FACCIONES as usize) % m.0.campamentos().len());
                let destino = en_tierra(&m.0, c, rng.por_debajo(300 * U as u64) as Mil - 150 * U, rng.por_debajo(300 * U as u64) as Mil - 150 * U);
                use pb::orden_marchar_a::{objetivo::Tipo, Objetivo};
                (k as u64 + 1 + ADMINS as u64 + EN_PLAZA as u64, orden::Cuerpo::MarcharA(pb::OrdenMarcharA { heroe_id: ids::heroe(*h), objetivo: Some(Objetivo { tipo: Some(Tipo::Punto(pb::Punto { x: destino.x, y: destino.y })) }) }))
            })
            .collect();
        ordenes
    };
    let i_ordenes = Pared::now();
    for (sesion, cuerpo) in ronda() {
        b.orden(sesion, cuerpo).await;
    }
    barrera(&mut b).await;
    let ordenes = i_ordenes.elapsed().as_secs_f64();
    let por_orden = b.recoger();
    b.orden(1, orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: "reanudar".into(), factor_milesimas: None, hasta_ms: None })).await;
    let i_en_marcha = Pared::now();
    for (sesion, cuerpo) in ronda() {
        b.orden(sesion, cuerpo).await;
    }
    barrera(&mut b).await;
    let ordenes_en_marcha = i_en_marcha.elapsed().as_secs_f64();
    b.recoger();
    drop(b.tx);
    b.actor.await.unwrap();

    println!("pared, {MINUTOS} min de mundo en pasos de {PASO_MS} ms: núcleo solo {solo:.2} s; actor con 1 administración {un_admin:.2} s; actor con 500 sesiones {quinientas:.2} s");
    println!("versiones al acabar el guion: {versiones} (iguales con 1 sesión y con 500, y el hash)");
    println!("conectar las 499 (Bienvenida, fotos y vistas): {conexion:.2} s; {} órdenes de marcha con las 500 conectadas: {ordenes:.2} s ({:.0} µs por orden) con el reloj parado, {ordenes_en_marcha:.2} s ({:.0} µs por orden, con los muestreos de 250 ms) en marcha a ×1", mapa.len(), ordenes * 1e6 / mapa.len() as f64, ordenes_en_marcha * 1e6 / mapa.len() as f64);
    imprimir("fotos al conectar (KB por sesión, no por minuto):", &fotos, 1.0);
    imprimir("con 1 administración:", &solo_admin, MINUTOS as f64);
    imprimir("con 500 sesiones:", &en_marcha, MINUTOS as f64);
    imprimir("las órdenes de marcha (KB por sesión en total):", &por_orden, 1.0);
}
