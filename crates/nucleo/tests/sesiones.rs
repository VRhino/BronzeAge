//! Integración: servidor real (WebSocket) + actor del núcleo + `ClientePrueba`.

mod comun;

use base::Punto;
use comun::*;
use contrato::v1::{
    self as pb, mensaje_de_cliente as mc, mensaje_de_servidor as ms, orden, orden_marchar_a, DecisionReanudacion, Entidad, Orden,
    OrdenControlarReloj, OrdenMarcharA, TipoCliente,
};
use nucleo::{sesiones, Nucleo};
use servidor::{
    prueba::{hola, AutenticadorDePrueba, ClientePrueba},
    servir, ComandoSesion, Config, Difusion, RelojCompartido, Servidor,
};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

struct Mundo {
    srv: Servidor,
    tx: mpsc::Sender<ComandoSesion>,
    actor: JoinHandle<Nucleo>,
}

async fn arrancar() -> Mundo {
    let reloj = RelojCompartido::nuevo(0);
    let mut n = nucleo(30, 30);
    n.crear_ejercito(0, "heroe-ana", "ana", 1, c(1, 1), 4_000, 350_000, 350_000).unwrap(); // ej-1
    n.crear_ejercito(0, "heroe-beto", "beto", 2, c(10, 10), 4_000, 350_000, 350_000).unwrap(); // ej-2
    let (tx, rx) = mpsc::channel(1024);
    let actor = sesiones::lanzar(n, reloj.clone(), rx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let srv = servir(listener, tx.clone(), AutenticadorDePrueba, reloj, Config::default()).await;
    Mundo { srv, tx, actor }
}

impl Mundo {
    async fn parar(self) -> Nucleo {
        self.srv.detener().await;
        drop(self.tx);
        self.actor.await.unwrap()
    }
}

fn marchar_a(peticion: &str, heroe: &str, p: Punto) -> mc::Cuerpo {
    mc::Cuerpo::Orden(Orden {
        peticion_id: peticion.into(),
        cuerpo: Some(orden::Cuerpo::MarcharA(OrdenMarcharA {
            heroe_id: heroe.into(),
            objetivo: Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(pb::Punto { x: p.x, y: p.y })) }),
        })),
        ..Default::default()
    })
}

fn reloj_orden(peticion: &str, accion: &str, factor: Option<i64>) -> mc::Cuerpo {
    mc::Cuerpo::Orden(Orden {
        peticion_id: peticion.into(),
        cuerpo: Some(orden::Cuerpo::ControlarReloj(OrdenControlarReloj { accion: accion.into(), factor_milesimas: factor, hasta_ms: None })),
        ..Default::default()
    })
}

fn ejercito(e: &Entidad) -> &pb::Ejercito {
    match e.cuerpo.as_ref() {
        Some(pb::entidad::Cuerpo::Ejercito(x)) => x,
        _ => panic!("se esperaba un Ejercito"),
    }
}

async fn foto(c: &mut ClientePrueba) -> pb::Foto {
    match c.esperar().await.cuerpo {
        Some(ms::Cuerpo::Foto(f)) => f,
        o => panic!("se esperaba Foto, llegó {o:?}"),
    }
}
async fn delta(c: &mut ClientePrueba) -> pb::Delta {
    match c.esperar().await.cuerpo {
        Some(ms::Cuerpo::Delta(d)) => d,
        o => panic!("se esperaba Delta, llegó {o:?}"),
    }
}
async fn resultado(c: &mut ClientePrueba) -> pb::ResultadoOrden {
    match c.esperar().await.cuerpo {
        Some(ms::Cuerpo::Resultado(r)) => r,
        o => panic!("se esperaba Resultado, llegó {o:?}"),
    }
}
async fn silencio(c: &mut ClientePrueba) {
    assert!(tokio::time::timeout(Duration::from_millis(300), c.recibir()).await.is_err(), "no debía llegar nada");
}

#[tokio::test]
async fn conectar_ordenar_marcha_y_ver_llegar() {
    let w = arrancar().await;
    let (mut ana, b) = ClientePrueba::entrar(w.srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    assert_eq!(b.heroe_id.as_deref(), Some("ana"));
    assert_eq!(b.mundo.unwrap().hash, "hash-de-prueba");
    assert_eq!(b.ambitos[0].decision, DecisionReanudacion::ReanudaConFoto as i32);
    let f = foto(&mut ana).await;
    assert_eq!((f.entidades.len(), f.ambito.unwrap().id.as_str()), (1, "ana"), "solo lo suyo");
    let e = ejercito(&f.entidades[0]);
    assert_eq!((e.id.as_str(), e.estado, e.plan.is_none()), ("ej-1", pb::EstadoColumna::ColumnaEstacionada as i32, true));

    let (mut beto, _) = ClientePrueba::entrar(w.srv.direccion, hola("ok:beto", TipoCliente::ClienteUnity)).await;
    foto(&mut beto).await;
    let (mut admin, _) = ClientePrueba::entrar(w.srv.direccion, hola("ok:admin", TipoCliente::ClienteAdminWeb)).await;
    assert!(foto(&mut admin).await.entidades.is_empty());

    // El administrador acelera el reloj ×600: el resultado y luego el aviso a todos.
    admin.enviar(reloj_orden("r1", "fijar_factor", Some(600_000))).await;
    assert_eq!(resultado(&mut admin).await.resultado, pb::Resultado::OrdenAceptada as i32);
    for c in [&mut admin, &mut ana, &mut beto] {
        match c.esperar().await.cuerpo {
            Some(ms::Cuerpo::Aviso(a)) => assert_eq!((a.codigo.as_str(), a.reloj.unwrap().factor_milesimas), ("reloj.cambiado", 600_000)),
            o => panic!("se esperaba el Aviso, llegó {o:?}"),
        }
    }
    // Un jugador no toca el reloj.
    ana.enviar(reloj_orden("r2", "pausar", None)).await;
    let r = resultado(&mut ana).await;
    assert_eq!((r.resultado, r.rechazo.unwrap().codigo.as_str()), (pb::Resultado::OrdenRechazada as i32, "no_autorizado"));
    // Ni manda columnas ajenas.
    ana.enviar(marchar_a("m0", "heroe-beto", c(5, 5))).await;
    assert_eq!(resultado(&mut ana).await.rechazo.unwrap().codigo, "no_autorizado");

    // La orden de marcha: primero la Delta (con el plan), después el resultado, con la causalidad del contrato.
    ana.enviar(marchar_a("m1", "heroe-ana", c(9, 1))).await;
    let d = delta(&mut ana).await;
    let r = resultado(&mut ana).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    assert_eq!(d.desde, f.version, "la delta continúa donde acaba la foto");
    assert!(d.hasta <= r.version_tras);
    let e = ejercito(d.cambios[0].datos.as_ref().unwrap());
    assert_eq!(e.estado, pb::EstadoColumna::ColumnaMarchando as i32);
    let plan = e.plan.as_ref().unwrap();
    assert!(plan.vertices.len() >= 2);
    assert_eq!(plan.vertices.last().unwrap().p.as_ref().map(|p| (p.x, p.y)), Some((c(9, 1).x, c(9, 1).y)));

    // Idempotencia: la misma petición no se ejecuta otra vez. Y la llegada llega sola, sin que nadie pida nada (16 s de
    // mundo ≈ 27 ms con el reloj ×600): puede cruzarse con la respuesta, así que se aceptan en cualquier orden.
    ana.enviar(marchar_a("m1", "heroe-ana", c(1, 9))).await;
    let (mut dup, mut d2) = (None, None);
    while dup.is_none() || d2.is_none() {
        match ana.esperar().await.cuerpo {
            Some(ms::Cuerpo::Resultado(x)) => dup = Some(x),
            Some(ms::Cuerpo::Delta(x)) => d2 = Some(x),
            o => panic!("llegó {o:?}"),
        }
    }
    let (dup, d2) = (dup.unwrap(), d2.unwrap());
    assert_eq!((dup.resultado, dup.version_tras), (pb::Resultado::OrdenDuplicada as i32, r.version_tras));
    assert_eq!(d2.desde, d.hasta);
    let e = ejercito(d2.cambios[0].datos.as_ref().unwrap());
    assert_eq!((e.estado, e.plan.is_none()), (pb::EstadoColumna::ColumnaEstacionada as i32, true));
    assert_eq!(e.posicion.as_ref().map(|p| (p.x, p.y)), Some((c(9, 1).x, c(9, 1).y)));
    assert_eq!(d2.eventos.len(), 1);
    assert_eq!(d2.eventos[0].codigo, "ejercito.llega");
    assert_eq!(d2.eventos[0].heroes_ids, ["heroe-ana"]);

    // Lo de ana no le llega a beto (ni nada en absoluto).
    silencio(&mut beto).await;
    let nucleo = w.parar().await;
    assert_eq!(nucleo.estado().ejercitos[&1].posicion, c(9, 1));
}

#[tokio::test]
async fn reanudacion_sin_novedad_con_deltas_y_con_foto() {
    let w = arrancar().await;
    let (mut admin, _) = ClientePrueba::entrar(w.srv.direccion, hola("ok:admin", TipoCliente::ClienteAdminWeb)).await;
    foto(&mut admin).await;
    let (mut ana, _) = ClientePrueba::entrar(w.srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    let f = foto(&mut ana).await;
    ana.cerrar().await;

    // Cambia el mundo sin tocar nada de ana: beto marcha. Ana, al volver, "sin novedad" aunque la versión global avanzó.
    let (mut beto, _) = ClientePrueba::entrar(w.srv.direccion, hola("ok:beto", TipoCliente::ClienteUnity)).await;
    foto(&mut beto).await;
    beto.enviar(marchar_a("b1", "heroe-beto", c(12, 10))).await;
    delta(&mut beto).await;
    assert!(resultado(&mut beto).await.version_tras > f.version);
    let mut h = hola("ok:ana", TipoCliente::ClienteUnity);
    h.versiones_conocidas.insert("HEROE:ana".into(), f.version);
    let (mut ana, b) = ClientePrueba::entrar(w.srv.direccion, h).await;
    assert_eq!(b.ambitos[0].decision, DecisionReanudacion::ReanudaSinNovedad as i32);
    silencio(&mut ana).await;
    ana.cerrar().await;

    // Ahora el administrador manda la columna de ana mientras está fuera: al volver recibe un delta consolidado.
    admin.enviar(marchar_a("a1", "heroe-ana", c(3, 1))).await;
    assert_eq!(resultado(&mut admin).await.resultado, pb::Resultado::OrdenAceptada as i32); // admin no ve lo de ana
    let mut h = hola("ok:ana", TipoCliente::ClienteUnity);
    h.versiones_conocidas.insert("HEROE:ana".into(), f.version);
    let (mut ana, b) = ClientePrueba::entrar(w.srv.direccion, h).await;
    assert_eq!(b.ambitos[0].decision, DecisionReanudacion::ReanudaConDeltas as i32);
    let d = delta(&mut ana).await;
    assert_eq!(d.desde, f.version);
    assert_eq!(d.hasta, b.ambitos[0].version_actual);
    assert_eq!(d.cambios.len(), 1);

    // Una versión del futuro (el servidor volvió de una copia vieja) o ninguna: foto.
    let mut h = hola("ok:ana", TipoCliente::ClienteUnity);
    h.versiones_conocidas.insert("HEROE:ana".into(), 999_999);
    let (mut ana, b) = ClientePrueba::entrar(w.srv.direccion, h).await;
    assert_eq!(b.ambitos[0].decision, DecisionReanudacion::ReanudaConFoto as i32);
    foto(&mut ana).await;
    w.parar().await;
}

#[tokio::test]
async fn el_cliente_lento_se_descarta_sin_bloquear_el_mundo() {
    let w = arrancar().await;
    // Sesión directa con un canal de capacidad 1 que nadie lee: la Foto lo llena.
    let (dtx, mut drx) = mpsc::channel(1);
    let (rtx, rrx) = oneshot::channel();
    let h = hola("ok:ana", TipoCliente::ClienteUnity);
    let identidad = servidor::Identidad { cliente_id: "ana".into(), roles: vec![TipoCliente::ClienteUnity] };
    w.tx.send(ComandoSesion::Conecta { sesion: 777, identidad, hola: h, difusion: dtx, respuesta: rtx }).await.unwrap();
    rrx.await.unwrap().expect("bienvenida");

    // El administrador mueve la columna de ana: el actor no puede depositar y la descarta; el mundo sigue.
    let (mut admin, _) = ClientePrueba::entrar(w.srv.direccion, hola("ok:admin", TipoCliente::ClienteAdminWeb)).await;
    foto(&mut admin).await;
    admin.enviar(marchar_a("a1", "heroe-ana", c(3, 1))).await;
    assert_eq!(resultado(&mut admin).await.resultado, pb::Resultado::OrdenAceptada as i32);
    admin.enviar(marchar_a("a2", "heroe-ana", c(5, 1))).await;
    assert_eq!(resultado(&mut admin).await.resultado, pb::Resultado::OrdenAceptada as i32);

    assert!(matches!(drx.recv().await, Some(Difusion::Foto(_))), "lo que ya estaba en el canal se entrega");
    assert!(drx.recv().await.is_none(), "después, el canal se cierra: la sesión se descartó");
    w.parar().await;
}

#[tokio::test]
async fn ordenes_no_implementadas_se_rechazan_con_codigo_estable() {
    let w = arrancar().await;
    let (mut ana, _) = ClientePrueba::entrar(w.srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    foto(&mut ana).await;
    ana.enviar(mc::Cuerpo::Orden(Orden {
        peticion_id: "x".into(),
        cuerpo: Some(orden::Cuerpo::Fundar(pb::OrdenFundar {})),
        ..Default::default()
    }))
    .await;
    assert_eq!(resultado(&mut ana).await.rechazo.unwrap().codigo, "orden.no_soportada");
    ana.enviar(marchar_a("y", "nadie", c(1, 1))).await;
    assert_eq!(resultado(&mut ana).await.rechazo.unwrap().codigo, "heroe.desconocido");
    w.parar().await;
}
