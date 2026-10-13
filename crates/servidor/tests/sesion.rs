//! Pruebas de integración: servidor real + núcleo falso + cliente `tokio-tungstenite` en el mismo proceso.

use std::{collections::HashSet, time::Duration};

use contrato::v1::{
    mensaje_de_cliente as mc, mensaje_de_servidor as ms, AmbitoTipo, CambiarVista, DecisionReanudacion, Punto,
    Rectangulo, Resultado, TipoCliente,
};
use servidor::{
    prueba::{hola, AutenticadorDePrueba, ClientePrueba, NucleoFalso},
    servir, Config, RelojCompartido, Servidor,
};

async fn arrancar(config: Config) -> (Servidor, NucleoFalso) {
    let reloj = RelojCompartido::nuevo(1_000_000);
    let nucleo = NucleoFalso::arrancar(reloj.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    (servir(listener, nucleo.comandos.clone(), AutenticadorDePrueba, reloj, config).await, nucleo)
}

async fn esperar_sesiones(nucleo: &NucleoFalso, n: usize) {
    for _ in 0..200 {
        if nucleo.sesiones_activas() == n {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("sesiones activas: {} (se esperaban {n})", nucleo.sesiones_activas());
}

/// Envía `h` (que debe fallar), y comprueba el `ErrorSesion` fatal y el cierre posterior.
async fn debe_fallar(srv: &Servidor, primero: mc::Cuerpo, codigo: &str) {
    let mut c = ClientePrueba::conectar(srv.direccion).await;
    c.enviar(primero).await;
    let Some(ms::Cuerpo::Error(e)) = c.esperar().await.cuerpo else { panic!("se esperaba ErrorSesion") };
    assert_eq!((e.codigo.as_str(), e.fatal), (codigo, true));
    assert!(c.recibir().await.is_none(), "tras un error fatal el servidor cierra");
}

#[tokio::test]
async fn handshake_feliz() {
    let (srv, _) = arrancar(Config::default()).await;
    let (mut c, b) = ClientePrueba::entrar(srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    assert!(!b.sesion_id.is_empty());
    assert_eq!(b.heroe_id.as_deref(), Some("ana"));
    assert_eq!(b.maximo_ordenes_pendientes, Config::default().max_ordenes_pendientes);
    assert_eq!(b.ambitos[0].decision, DecisionReanudacion::ReanudaConFoto as i32);
    assert!(b.reloj.unwrap().mundo_ms >= 1_000_000);
    // La Bienvenida fue secuencia 1 (lo comprueba el cliente); la Foto, 2.
    assert!(matches!(c.esperar().await.cuerpo, Some(ms::Cuerpo::Foto(f)) if f.entidades.len() == 1));
}

#[tokio::test]
async fn versiones_de_protocolo_erroneas() {
    let (srv, _) = arrancar(Config::default()).await;
    for v in [0, 2] {
        let mut h = hola("ok:ana", TipoCliente::ClienteUnity);
        h.version_protocolo = v;
        debe_fallar(&srv, mc::Cuerpo::Hola(h), "version_protocolo").await;
    }
}

#[tokio::test]
async fn token_invalido_rol_y_acceso_caido() {
    let (srv, _) = arrancar(Config::default()).await;
    debe_fallar(&srv, mc::Cuerpo::Hola(hola("falso", TipoCliente::ClienteUnity)), "token_invalido").await;
    debe_fallar(&srv, mc::Cuerpo::Hola(hola("bot:b1", TipoCliente::ClienteUnity)), "rol_no_permitido").await;
    debe_fallar(&srv, mc::Cuerpo::Hola(hola("ok:ana", TipoCliente::NoEspecificado)), "rol_no_permitido").await;
    debe_fallar(&srv, mc::Cuerpo::Hola(hola("caido", TipoCliente::ClienteUnity)), "interno").await;
}

#[tokio::test]
async fn el_primer_mensaje_debe_ser_hola() {
    let (srv, _) = arrancar(Config::default()).await;
    debe_fallar(&srv, mc::Cuerpo::Ping(contrato::v1::Ping { cliente_envio_ms: 1 }), "hola_esperado").await;
}

#[tokio::test]
async fn el_nucleo_puede_rechazar_la_entrada() {
    let (srv, _) = arrancar(Config::default()).await;
    let mut h = hola("ok:ana", TipoCliente::ClienteUnity);
    h.partida_id = "inexistente".into();
    debe_fallar(&srv, mc::Cuerpo::Hola(h), "partida_no_disponible").await;
}

#[tokio::test]
async fn ping_pong_con_reloj() {
    let (srv, _) = arrancar(Config::default()).await;
    let (mut c, _) = ClientePrueba::entrar(srv.direccion, hola("ok:ana", TipoCliente::ClienteBot)).await;
    c.esperar().await; // Foto
    c.ping(777).await;
    let Some(ms::Cuerpo::Pong(p)) = c.esperar().await.cuerpo else { panic!("se esperaba Pong") };
    let r = p.reloj.unwrap();
    assert_eq!(p.cliente_envio_ms, 777);
    assert!(r.mundo_ms >= 1_000_000 && r.factor_milesimas == 1000 && !r.pausado);
    assert!(p.servidor_recibio_ms <= r.servidor_ms);
}

#[tokio::test]
async fn orden_con_respuesta_causal() {
    let (srv, _) = arrancar(Config::default()).await;
    let (mut c, _) = ClientePrueba::entrar(srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    c.esperar().await;
    c.orden("o1").await;
    let Some(ms::Cuerpo::Delta(d)) = c.esperar().await.cuerpo else { panic!("la Delta va antes del resultado") };
    let Some(ms::Cuerpo::Resultado(r)) = c.esperar().await.cuerpo else { panic!("se esperaba el resultado") };
    assert_eq!((r.peticion_id.as_str(), r.resultado), ("o1", Resultado::OrdenAceptada as i32));
    assert_eq!((d.desde, d.hasta, r.version_tras), (0, 1, 1));

    c.orden("o1").await; // idempotencia (la decide el núcleo; el servidor solo la transporta)
    let Some(ms::Cuerpo::Resultado(r)) = c.esperar().await.cuerpo else { panic!() };
    assert_eq!(r.resultado, Resultado::OrdenDuplicada as i32);

    c.orden("rechaza1").await;
    let Some(ms::Cuerpo::Resultado(r)) = c.esperar().await.cuerpo else { panic!() };
    assert_eq!(r.rechazo.unwrap().codigo, "no_autorizado");
}

#[tokio::test]
async fn secuencias_sin_huecos_y_causalidad_con_trafico_mezclado() {
    let (srv, nucleo) = arrancar(Config::default()).await;
    let (mut c, _) = ClientePrueba::entrar(srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    c.esperar().await;
    for i in 0..30 {
        c.orden(&format!("o{i}")).await;
        c.ping(i).await;
        nucleo.difundir_delta();
    }
    // 30 × (Delta + Resultado + Pong + Delta difundida); `recibir` falla si hay un hueco de secuencia.
    let mut deltas_vistas = HashSet::new();
    for _ in 0..120 {
        match c.esperar().await.cuerpo {
            Some(ms::Cuerpo::Delta(d)) => {
                deltas_vistas.insert(d.hasta);
            }
            Some(ms::Cuerpo::Resultado(r)) => {
                assert!(deltas_vistas.contains(&r.version_tras), "el resultado llegó antes que su Delta");
            }
            Some(ms::Cuerpo::Pong(_)) => {}
            otro => panic!("inesperado: {otro:?}"),
        }
    }
}

#[tokio::test]
async fn reanudacion_sin_novedad_con_deltas_y_con_foto() {
    let (srv, nucleo) = arrancar(Config::default()).await;
    let (mut c, _) = ClientePrueba::entrar(srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    c.esperar().await;
    for i in 0..3 {
        c.orden(&format!("o{i}")).await;
        c.esperar().await;
        c.esperar().await;
    }
    c.cerrar().await;

    let reanudar = |v: u64| {
        let mut h = hola("ok:ana", TipoCliente::ClienteUnity);
        h.versiones_conocidas.insert("HEROE:ana".into(), v);
        h
    };
    // Al día: sin novedad y sin Foto (lo siguiente que llega es el Pong).
    let (mut c, b) = ClientePrueba::entrar(srv.direccion, reanudar(3)).await;
    assert_eq!(b.ambitos[0].decision, DecisionReanudacion::ReanudaSinNovedad as i32);
    c.ping(1).await;
    assert!(matches!(c.esperar().await.cuerpo, Some(ms::Cuerpo::Pong(_))));
    c.cerrar().await;

    // Dos versiones de retraso: el tramo de deltas.
    nucleo.difundir_delta();
    nucleo.difundir_delta();
    let (mut c, b) = ClientePrueba::entrar(srv.direccion, reanudar(3)).await;
    assert_eq!(b.ambitos[0].decision, DecisionReanudacion::ReanudaConDeltas as i32);
    assert_eq!(b.ambitos[0].version_actual, 5);
    let Some(ms::Cuerpo::Delta(d)) = c.esperar().await.cuerpo else { panic!("se esperaba la Delta de reanudación") };
    assert_eq!((d.desde, d.hasta), (3, 5));

    // Reanudar es reconectar: un segundo Hola en la misma sesión es un error no fatal y la sesión sigue viva.
    c.enviar(mc::Cuerpo::Hola(reanudar(0))).await;
    let Some(ms::Cuerpo::Error(e)) = c.esperar().await.cuerpo else { panic!("se esperaba hola_repetido") };
    assert_eq!((e.codigo.as_str(), e.fatal), ("hola_repetido", false));
    c.enviar(mc::Cuerpo::Ping(contrato::v1::Ping { cliente_envio_ms: 7 })).await;
    assert!(matches!(c.esperar().await.cuerpo, Some(ms::Cuerpo::Pong(p)) if p.cliente_envio_ms == 7));
    c.cerrar().await;

    // Demasiado viejo: foto.
    for _ in 0..10 {
        nucleo.difundir_delta();
    }
    let (mut c, b) = ClientePrueba::entrar(srv.direccion, reanudar(3)).await;
    assert_eq!(b.ambitos[0].decision, DecisionReanudacion::ReanudaConFoto as i32);
    assert!(matches!(c.esperar().await.cuerpo, Some(ms::Cuerpo::Foto(f)) if f.version == 15));
}

#[tokio::test]
async fn compresion_zstd_negociada() {
    let (srv, _) = arrancar(Config { umbral_compresion: 64, ..Config::default() }).await;
    let larga = "a".repeat(4000);

    let mut h = hola("ok:ana", TipoCliente::ClienteUnity);
    h.compresiones = vec!["gzip".into(), "zstd".into()];
    let (mut c, _) = ClientePrueba::entrar(srv.direccion, h).await;
    assert_eq!(c.tramas_zstd, 0, "la Bienvenida nunca va comprimida");
    c.esperar().await;
    c.orden(&larga).await; // el cliente también manda comprimido
    c.esperar().await; // Delta
    let Some(ms::Cuerpo::Resultado(r)) = c.esperar().await.cuerpo else { panic!() };
    assert_eq!(r.peticion_id, larga);
    assert!(c.tramas_zstd >= 1, "el resultado largo debía llegar comprimido");

    // Sin oferta de zstd, nada se comprime aunque sea grande.
    let (mut c, _) = ClientePrueba::entrar(srv.direccion, hola("ok:beto", TipoCliente::ClienteUnity)).await;
    c.esperar().await;
    c.orden(&"b".repeat(4000)).await; // otra petición: la repetida sería un duplicado sin Delta
    c.esperar().await;
    c.esperar().await;
    assert_eq!(c.tramas_zstd, 0);
}

#[tokio::test]
async fn desconexion_limpia() {
    let (srv, nucleo) = arrancar(Config::default()).await;
    let (c, _) = ClientePrueba::entrar(srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    esperar_sesiones(&nucleo, 1).await;
    c.cerrar().await;
    esperar_sesiones(&nucleo, 0).await;
}

#[tokio::test]
async fn una_sola_sesion_por_cliente() {
    let (srv, nucleo) = arrancar(Config::default()).await;
    let (mut a, _) = ClientePrueba::entrar(srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    let (mut b, _) = ClientePrueba::entrar(srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    assert!(matches!(a.esperar().await.cuerpo, Some(ms::Cuerpo::Foto(_))));
    let Some(ms::Cuerpo::Aviso(av)) = a.esperar().await.cuerpo else { panic!("se esperaba el aviso") };
    assert_eq!(av.codigo, "sesion.reemplazada");
    assert!(a.recibir().await.is_none());
    esperar_sesiones(&nucleo, 1).await;
    b.esperar().await;
    b.orden("o1").await;
    b.esperar().await;
    assert!(matches!(b.esperar().await.cuerpo, Some(ms::Cuerpo::Resultado(_))));
    // Otro cliente distinto no molesta.
    let (_c, _) = ClientePrueba::entrar(srv.direccion, hola("ok:beto", TipoCliente::ClienteUnity)).await;
    esperar_sesiones(&nucleo, 2).await;
}

#[tokio::test]
async fn limite_de_ordenes_pendientes() {
    let (srv, nucleo) = arrancar(Config { max_ordenes_pendientes: 3, ..Config::default() }).await;
    let (mut c, b) = ClientePrueba::entrar(srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    assert_eq!(b.maximo_ordenes_pendientes, 3);
    c.esperar().await;
    for i in 0..4 {
        c.orden(&format!("lenta{i}")).await; // el núcleo falso no las contesta
    }
    let Some(ms::Cuerpo::Error(e)) = c.esperar().await.cuerpo else { panic!("se esperaba ErrorSesion") };
    assert_eq!((e.codigo.as_str(), e.fatal), ("demasiadas_ordenes", false));
    c.ping(5).await; // la sesión sigue viva
    assert!(matches!(c.esperar().await.cuerpo, Some(ms::Cuerpo::Pong(_))));
    assert_eq!(nucleo.ordenes_recibidas(), 3, "la cuarta no llegó al núcleo");

    // Con respuesta, el hueco se libera: muchas órdenes seguidas con el límite al mínimo.
    let (srv2, _) = arrancar(Config { max_ordenes_pendientes: 1, ..Config::default() }).await;
    let (mut c, _) = ClientePrueba::entrar(srv2.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    c.esperar().await;
    for i in 0..10 {
        c.orden(&format!("o{i}")).await;
        c.esperar().await;
        assert!(matches!(c.esperar().await.cuerpo, Some(ms::Cuerpo::Resultado(_))));
    }
}

#[tokio::test]
async fn vista_y_batalla() {
    let (srv, _) = arrancar(Config::default()).await;
    let (mut c, _) = ClientePrueba::entrar(srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    c.esperar().await;
    let rect = Rectangulo { min: Some(Punto::default()), max: Some(Punto { x: 10, y: 10 }) };
    c.enviar(mc::Cuerpo::Vista(CambiarVista { rectangulo: Some(rect) })).await;
    let Some(ms::Cuerpo::Foto(f)) = c.esperar().await.cuerpo else { panic!() };
    assert_eq!(f.ambito.unwrap().tipo, AmbitoTipo::AmbitoVista as i32);

    // Un cliente normal no puede mandar mensajes de servidor de batalla.
    c.enviar(mc::Cuerpo::Batalla(Default::default())).await;
    let Some(ms::Cuerpo::Error(e)) = c.esperar().await.cuerpo else { panic!() };
    assert_eq!((e.codigo.as_str(), e.fatal), ("rol_no_permitido", false));
    // Y un mensaje vacío es fatal.
    c.enviar_vacio().await;
    let Some(ms::Cuerpo::Error(e)) = c.esperar().await.cuerpo else { panic!() };
    assert_eq!((e.codigo.as_str(), e.fatal), ("mensaje_invalido", true));
    assert!(c.recibir().await.is_none());
}

#[tokio::test]
async fn la_parada_avisa_y_cierra() {
    let (srv, _) = arrancar(Config::default()).await;
    let (mut c, _) = ClientePrueba::entrar(srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    c.esperar().await;
    let parada = tokio::spawn(srv.detener());
    let Some(ms::Cuerpo::Aviso(a)) = c.esperar().await.cuerpo else { panic!() };
    assert_eq!(a.codigo, "servidor.reiniciando");
    assert!(c.recibir().await.is_none());
    parada.await.unwrap();
}

#[tokio::test]
async fn rendimiento_con_200_sesiones() {
    let m = servidor::prueba::medir_rendimiento(200, 50).await;
    println!("{m:?} → {:.0} mensajes/s", m.mensajes_por_s());
    assert_eq!(m.mensajes, 200 * 50 * 2);
}
