//! El lanzador de punta a punta: un núcleo real con su servidor del contrato 05 y su servicio de acceso (la cuenta de servidor de batalla de verdad, con su login), una batalla
//! que abre un humano, y el lanzador, que la reclama, arranca `bronzeage-batalla-de-mentira` (el Unity de mentira) y le devuelve su resultado al núcleo. Sobre una retícula
//! sintética: no hace falta el artefacto de mundo.

use std::{sync::Arc, time::Duration};

use acceso::{Acceso, Config as ConfigAcceso, Coste};
use base::{Punto, MS_POR_MINUTO};
use combate::ciclo::EstadoBatalla;
use contrato::v1::{AristaGrafo, GrafoNavegacion, NodoGrafo, Punto as PuntoPb, Resultado};
use lanzador::{Config, Informe};
use militar::{catalogo::Tropa, heroe::Controlador};
use nucleo::{proyeccion::ids, Ajustes, Arranque, Comando, Nucleo, ObjetivoAtaque, Peticion, Relacion, Reglas};
use rutas::Rutas;
use servidor::{servir_con_http, RelojCompartido};
use tokio::sync::{mpsc, oneshot};

const ESP: i64 = 8_000;
const MIN: i64 = MS_POR_MINUTO;
const CLAVE: &str = "una clave larga";

fn c(x: i64, y: i64) -> Punto {
    Punto::new(x * ESP + ESP / 2, y * ESP + ESP / 2)
}

/// Retícula `w × h` abierta con 8 vecinos (la de las pruebas del núcleo).
fn reticula(w: i64, h: i64) -> Arc<Rutas> {
    let mut g = GrafoNavegacion { espaciado_min: ESP as u32, ..Default::default() };
    for y in 0..h {
        for x in 0..w {
            let p = c(x, y);
            g.nodos.push(NodoGrafo { id: (y * w + x) as u32, posicion: Some(PuntoPb { x: p.x, y: p.y }) });
        }
    }
    for y in 0..h {
        for x in 0..w {
            for (dx, dy) in [(1, 0), (0, 1), (1, 1), (-1, 1)] {
                let (x2, y2) = (x + dx, y + dy);
                if (0..w).contains(&x2) && (0..h).contains(&y2) {
                    g.aristas.push(AristaGrafo { a: (y * w + x) as u32, b: (y2 * w + x2) as u32, coste_milesimas: if dx != 0 && dy != 0 { 1414 } else { 1000 }, vado: false });
                }
            }
        }
    }
    Arc::new(Rutas::desde_grafo(&g))
}

fn admin(n: &mut Nucleo, id: &str, c: Comando) -> Vec<u64> {
    let r = n.aplicar_en(n.ahora(), Peticion::nueva("admin", id, c).de_admin());
    assert!(r.aceptada(), "{r:?}");
    r.creadas
}

/// Ana (F1, humana, espadachines) ataca a la columna de beto (F2, bot, milicia) en guerra: se abre una batalla de Unity sin convocatoria, que espera servidor.
/// Devuelve el núcleo, la batalla, el héroe de ana y sus escuadras.
fn nucleo_con_batalla() -> (Nucleo, u64, u64, Vec<u64>) {
    let arranque = Arranque { reglas: Reglas { batallas_en_unity: true, ..Reglas::default() }, ajustes: Ajustes::default(), semilla: 42, inicio: 0, rutas: reticula(40, 5), mundo_hash: "hash-de-prueba".into(), mundo: None };
    let mut n = Nucleo::nuevo(arranque);
    let columna = |n: &mut Nucleo, cuenta: &str, faccion: u32, p: Punto| {
        let id = n.crear_ejercito(n.ahora(), cuenta, faccion, p, 500, 350_000).unwrap();
        (id, n.ejercito(id).unwrap().columna.lider)
    };
    let ana = columna(&mut n, "ana", 1, c(10, 2));
    let beto = columna(&mut n, "beto", 2, Punto::new(c(10, 2).x + 1_000, c(10, 2).y));
    let completa = |t: Tropa| militar::Reglas::cargar().tropas.tropa(t).unidades;
    let mut suyas = Vec::new();
    for (i, t) in [Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto].into_iter().enumerate() {
        suyas.extend(admin(&mut n, &format!("d{i}"), Comando::DarEscuadra { heroe: ana.1, tropa: t, cantidad: completa(t) }));
    }
    admin(&mut n, "d9", Comando::DarEscuadra { heroe: beto.1, tropa: Tropa::MiliciaLanceros, cantidad: completa(Tropa::MiliciaLanceros) });
    admin(&mut n, "humano", Comando::FijarControlador { heroe: ana.1, controlador: Controlador::Humano });
    admin(&mut n, "guerra", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    let r = n.aplicar_en(10 * MIN, Peticion::nueva("ana", "a1", Comando::Atacar { heroe: ana.1, objetivo: ObjetivoAtaque::Ejercito(beto.0), declara_guerra: false }));
    assert!(r.aceptada(), "{r:?}");
    let id = n.batallas().next().expect("se abrió una batalla").id;
    assert_eq!(n.batallas_pendientes(), vec![id], "espera servidor");
    (n, id, ana.1, suyas)
}

/// El motor sirviendo el núcleo con el acceso real; la cuenta del lanzador registrada con el código de servidores.
struct Motor {
    srv: servidor::Servidor,
    tx: mpsc::Sender<servidor::ComandoSesion>,
    actor: tokio::task::JoinHandle<Nucleo>,
}

async fn motor(n: Nucleo) -> Motor {
    let acceso = Acceso::en_memoria(ConfigAcceso { codigo_registro_servidores: Some("servidores".into()), coste: Coste::PRUEBAS, ..ConfigAcceso::default() });
    acceso.registrar("lanzador-1", CLAVE, Some("servidores")).await.unwrap();
    acceso.registrar("ana", CLAVE, None).await.unwrap();
    let reloj = RelojCompartido::nuevo(n.ahora());
    let (tx, rx) = mpsc::channel(1024);
    let actor = nucleo::sesiones::lanzar(n, reloj.clone(), rx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let srv = servir_con_http(listener, tx.clone(), acceso.clone(), Arc::new(acceso), reloj, servidor::Config::default()).await;
    Motor { srv, tx, actor }
}

impl Motor {
    async fn parar(self) -> Nucleo {
        self.srv.detener().await;
        drop(self.tx);
        self.actor.await.unwrap()
    }
}

/// El lanzador corriendo contra `motor`; devuelve el canal de sus informes y cómo pararlo.
fn lanzar(config: Config) -> (mpsc::UnboundedReceiver<Informe>, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    let (itx, irx) = mpsc::unbounded_channel();
    let (ptx, prx) = oneshot::channel::<()>();
    let h = tokio::spawn(lanzador::correr(config, Some(itx), async {
        let _ = prx.await;
    }));
    (irx, ptx, h)
}

fn config(m: &Motor, nick: &str) -> Config {
    let mut c = Config::nueva(&m.srv.direccion.to_string(), nick, CLAVE, env!("CARGO_BIN_EXE_bronzeage-batalla-de-mentira"));
    c.puertos = vec![41_001];
    c.espera_reconexion = Duration::from_millis(100);
    c.espera_proceso = Duration::from_secs(20);
    c
}

/// Los informes hasta el primero que cumpla `fin` (incluido), o falla a los 30 s.
async fn hasta(rx: &mut mpsc::UnboundedReceiver<Informe>, fin: impl Fn(&Informe) -> bool) -> Vec<Informe> {
    let mut v = Vec::new();
    loop {
        let i = tokio::time::timeout(Duration::from_secs(30), rx.recv()).await.unwrap_or_else(|_| panic!("sin el informe esperado; llegaron {v:#?}")).expect("el lanzador sigue");
        let parar = fin(&i);
        v.push(i);
        if parar {
            return v;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn el_lanzador_reclama_la_batalla_arranca_el_proceso_y_su_resultado_llega_al_nucleo() {
    let (n, id, ana, suyas) = nucleo_con_batalla();
    let m = motor(n).await;
    // Un lanzador nuevo, con el código de servidores: se da de alta solo al entrar (la cuenta u-3, tras lanzador-1 y ana).
    let mut c = config(&m, "lanzador-nuevo");
    c.codigo = Some("servidores".into());
    let (mut rx, parar, h) = lanzar(c);
    let v = hasta(&mut rx, |i| matches!(i, Informe::Terminada { .. })).await;
    let uuid = match &v[1] {
        Informe::Reclama { batalla, revision, puerto, humanos } => {
            assert_eq!((*revision, *puerto, humanos.clone()), (0, 41_001, vec![ids::heroe(ana)]), "con un token para ana, la única humana");
            batalla.clone()
        }
        o => panic!("se esperaba que reclamase, llegó {o:?}; todo: {v:#?}"),
    };
    let aceptado = |mensaje: &str| v.iter().any(|i| matches!(i, Informe::Acuse { mensaje: x, resultado: Resultado::OrdenAceptada, .. } if x == mensaje));
    assert_eq!(v[0], Informe::Conectado);
    assert!(v.contains(&Informe::Asignada { batalla: uuid.clone(), revision: 0 }));
    assert!(v.contains(&Informe::ProcesoArrancado { batalla: uuid.clone() }) && v.contains(&Informe::ProcesoConectado { batalla: uuid.clone() }));
    assert!(aceptado("asignacion") && aceptado("inicio") && aceptado("resultado"), "{v:#?}");
    assert!(matches!(v.last(), Some(Informe::Terminada { con_resultado: true, .. })), "{v:#?}");
    let _ = parar.send(());
    h.await.unwrap();

    // En el núcleo: la batalla se jugó en la instancia del lanzador, el token de ana quedó custodiado y la experiencia del resultado llegó a sus escuadras.
    let n = m.parar().await;
    let b = n.batalla(id).expect("retenida tras cerrarse");
    assert_eq!(b.estado, EstadoBatalla::Aplicada);
    let a = b.asignacion.as_ref().unwrap();
    assert_eq!((a.instancia.host.as_str(), a.instancia.puerto, a.instancia.protocolo.as_str(), a.servidor.as_str()), ("127.0.0.1", 41_001, "udp", "u-3"), "el servidor es la cuenta del lanzador");
    assert!(a.intento.starts_with("lz-") && a.tokens.len() == 1 && a.tokens[0].heroe == ana && a.tokens[0].token.len() == 64);
    for e in suyas {
        assert_eq!(n.heroe(ana).unwrap().heroe.escuadra(e).unwrap().experiencia, 2_000, "los 2 puntos del Unity de mentira");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn un_proceso_que_acaba_sin_resultado_suelta_el_puerto_y_la_batalla_queda_para_su_plazo() {
    let (n, id, ..) = nucleo_con_batalla();
    let m = motor(n).await;
    // Con la de mentira que lleva dentro el propio lanzador (`LANZADOR_PROCESO=mentira`: él mismo con `mentira` delante).
    let mut c = config(&m, "lanzador-1");
    c.proceso = env!("CARGO_BIN_EXE_bronzeage-lanzador").into();
    c.argumentos = vec!["mentira".into(), "--sin-resultado".into()];
    let (mut rx, parar, h) = lanzar(c);
    let v = hasta(&mut rx, |i| matches!(i, Informe::Terminada { .. })).await;
    assert!(matches!(v.last(), Some(Informe::Terminada { con_resultado: false, .. })), "{v:#?}");
    assert!(!v.iter().any(|i| matches!(i, Informe::Reenvia { mensaje, .. } if mensaje == "resultado")));
    let _ = parar.send(());
    h.await.unwrap();
    let n = m.parar().await;
    assert_eq!(n.batalla(id).unwrap().estado, EstadoBatalla::EnCurso, "empezó y espera su resultado hasta el plazo");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sin_cuenta_de_servidor_no_entra_y_un_proceso_que_no_existe_no_deja_la_batalla_cogida() {
    let (n, id, ..) = nucleo_con_batalla();
    let m = motor(n).await;
    // Una cuenta humana no sirve: el lanzador lo dice y sigue intentándolo.
    let (mut rx, parar, h) = lanzar(config(&m, "ana"));
    let v = hasta(&mut rx, |i| matches!(i, Informe::Desconectado(_))).await;
    assert!(matches!(&v[0], Informe::Desconectado(m) if m.contains("no es de servidor de batalla")), "{v:#?}");
    let _ = parar.send(());
    h.await.unwrap();
    // Con la cuenta buena pero un ejecutable que no existe, la reclama, no arranca y suelta el puerto.
    let mut c = config(&m, "lanzador-1");
    c.proceso = "/no/existe/servidor-de-batalla".into();
    let (mut rx, parar, h) = lanzar(c);
    let v = hasta(&mut rx, |i| matches!(i, Informe::Terminada { .. })).await;
    assert!(matches!(v.last(), Some(Informe::Terminada { con_resultado: false, motivo, .. }) if motivo.contains("no arranca")), "{v:#?}");
    let _ = parar.send(());
    h.await.unwrap();
    let n = m.parar().await;
    assert_eq!(n.batalla(id).unwrap().estado, EstadoBatalla::Asignada, "asignada a un proceso que no arrancó: la vence el núcleo por su plazo");
}
