//! El canal de control contra el servidor real: sin token no entra; con él, ve el estado, arranca los bots, los pausa, fuerza una llegada y los para.

use std::time::Duration;

use bronce_age_bots_brain::control::{Ajustes, Poblacion, Servicio};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;

mod comun;

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn recibir(ws: &mut Ws) -> Value {
    loop {
        match tokio::time::timeout(Duration::from_secs(10), ws.next()).await.expect("sin mensajes") {
            Some(Ok(Message::Text(t))) => return serde_json::from_str(&t).unwrap(),
            Some(Ok(_)) => continue,
            otro => panic!("se cerró: {otro:?}"),
        }
    }
}

/// Espera el estado que cumple `cond`, saltando lo demás.
async fn estado(ws: &mut Ws, que: &str, cond: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..300 {
        let m = recibir(ws).await;
        if m["tipo"] == "estado" && cond(&m["estado"]) {
            return m["estado"].clone();
        }
    }
    panic!("no llegó: {que}");
}

async fn mandar(ws: &mut Ws, v: Value) -> Value {
    let id = v["id"].clone();
    ws.send(Message::Text(v.to_string().into())).await.unwrap();
    loop {
        let m = recibir(ws).await;
        if m["tipo"] == "respuesta" && m["id"] == id {
            return m;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn el_panel_arranca_pausa_y_para_los_bots() {
    let datos = tempfile::tempdir().unwrap();
    let Some((_servidor, dir)) = comun::servidor(datos.path()).await else { return };
    let registros = tempfile::tempdir().unwrap();
    let puerto = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
    let s = Servicio::new(Ajustes { token: "secreto".into(), poblacion: Poblacion::Libres, codigo_bots: "bots".into(), registros: registros.path().into(), cada_ms: 500, por_defecto: json!({}) });
    tokio::spawn(s.escuchar(Box::leak(puerto.to_string().into_boxed_str())));
    tokio::time::sleep(Duration::from_millis(200)).await;

    // GET /salud.
    let salud = {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut c = tokio::net::TcpStream::connect(puerto).await.unwrap();
        c.write_all(b"GET /salud HTTP/1.1
host: x

").await.unwrap();
        let mut r = String::new();
        c.read_to_string(&mut r).await.unwrap();
        r
    };
    assert!(salud.contains("inactivo"), "{salud}");

    // Sin el token, fuera.
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{puerto}/control")).await.unwrap();
    let r = mandar(&mut ws, json!({"id": 1, "accion": "autenticar", "token": "otro"})).await;
    assert_eq!(r["ok"], false);

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{puerto}/control")).await.unwrap();
    assert_eq!(mandar(&mut ws, json!({"id": 1, "accion": "autenticar", "token": "secreto"})).await["ok"], true);
    let h = recibir(&mut ws).await;
    assert!(h["tipo"] == "historial" && h["eventos"].is_array(), "{h}");
    estado(&mut ws, "inactivo", |e| e["fase"] == "inactivo" && e["codigoEnEntorno"] == true).await;

    let config = json!({"servidor": format!("http://{dir}"), "partida": "prueba", "total": 2, "diasLlegada": 2.0 / 86_400.0, "semilla": 3, "cadaMs": 1000, "horario": "siempre"});
    assert_eq!(mandar(&mut ws, json!({"id": 2, "accion": "iniciar", "config": config})).await["ok"], true);
    let e = estado(&mut ws, "dos bots pensando", |e| e["fase"] == "corriendo" && e["bots"].as_array().is_some_and(|b| b.len() >= 2 && b.iter().all(|b| b["pensamientos"].as_u64() > Some(0)))).await;
    assert!(registros.path().join("bots-prueba.json").exists());
    assert!(e["llegadas"]["plan"].as_array().is_some_and(|p| !p.is_empty()));

    // Las pizarras salen con sus bots y una se vuelca entera, con su libro; la salud cuenta las órdenes del último minuto.
    let con_bots = |p: &serde_json::Value| p["bots"].as_array().is_some_and(|b| !b.is_empty());
    let e = estado(&mut ws, "pizarras con bots", |e| e["pizarras"].as_array().is_some_and(|p| p.iter().any(con_bots)) && e["salud"]["peticionesMin"].as_u64() > Some(0)).await;
    let id = e["pizarras"].as_array().unwrap().iter().find(|p| con_bots(p)).unwrap()["id"].as_str().unwrap().to_string();
    let v = mandar(&mut ws, json!({"id": 20, "accion": "volcarPizarra", "faccionId": id})).await;
    assert!(v["ok"] == true && v["datos"]["libro"].is_object() && v["datos"]["tablero"].is_array(), "{v}");

    assert_eq!(mandar(&mut ws, json!({"id": 3, "accion": "pausar"})).await["ok"], true);
    estado(&mut ws, "pausado", |e| e["fase"] == "pausado").await;
    assert_eq!(mandar(&mut ws, json!({"id": 4, "accion": "reanudar"})).await["ok"], true);
    assert_eq!(mandar(&mut ws, json!({"id": 5, "accion": "forzarLlegada", "perfil": "solitario"})).await["ok"], true);
    estado(&mut ws, "llega uno más", |e| e["llegadas"]["extras"] == 1 && e["bots"].as_array().is_some_and(|b| b.len() >= 3)).await;
    let heroe = e["bots"][0]["heroeId"].as_str().unwrap().to_string();
    assert_eq!(mandar(&mut ws, json!({"id": 6, "accion": "retirarBot", "heroeId": heroe})).await["ok"], true);
    estado(&mut ws, "retirado", |e| e["bots"].as_array().is_some_and(|b| b.iter().any(|b| b["heroeId"] == heroe.as_str() && b["retirado"] == true))).await;
    assert_eq!(mandar(&mut ws, json!({"id": 7, "accion": "parar"})).await["ok"], true);
    estado(&mut ws, "inactivo otra vez", |e| e["fase"] == "inactivo").await;
    assert_eq!(mandar(&mut ws, json!({"id": 8, "accion": "reiniciarRegistro"})).await["ok"], true);
    assert!(!registros.path().join("bots-prueba.json").exists());
}

/// Un servicio escuchando y autenticado.
async fn panel(registros: &std::path::Path, poblacion: Poblacion) -> Ws {
    let puerto = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
    let s = Servicio::new(Ajustes { token: "secreto".into(), poblacion, codigo_bots: "bots".into(), registros: registros.into(), cada_ms: 500, por_defecto: json!({}) });
    tokio::spawn(s.escuchar(Box::leak(puerto.to_string().into_boxed_str())));
    tokio::time::sleep(Duration::from_millis(200)).await;
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{puerto}/control")).await.unwrap();
    assert_eq!(mandar(&mut ws, json!({"id": 1, "accion": "autenticar", "token": "secreto"})).await["ok"], true);
    ws
}

#[tokio::test(flavor = "multi_thread")]
async fn los_mercenarios_se_arrancan_y_se_paran_sin_tocar_a_los_libres() {
    let datos = tempfile::tempdir().unwrap();
    let Some((_servidor, dir)) = comun::servidor(datos.path()).await else { return };
    let registros = tempfile::tempdir().unwrap();
    let mut libres = panel(registros.path(), Poblacion::Libres).await;
    let mut mercs = panel(registros.path(), Poblacion::Mercenarios).await;
    let config = json!({"servidor": format!("http://{dir}"), "partida": "prueba", "total": 1, "diasLlegada": 1.0 / 86_400.0, "semilla": 3, "cadaMs": 1000, "horario": "por-semilla"});

    assert_eq!(mandar(&mut libres, json!({"id": 2, "accion": "iniciar", "config": config})).await["ok"], true);
    estado(&mut libres, "el libre en marcha", |e| e["fase"] == "corriendo" && e["bots"].as_array().is_some_and(|b| b.len() == 1)).await;
    assert_eq!(mandar(&mut mercs, json!({"id": 2, "accion": "iniciar", "config": config})).await["ok"], true);
    let e = estado(&mut mercs, "el mercenario en marcha", |e| e["fase"] == "corriendo" && e["bots"].as_array().is_some_and(|b| b.len() == 1 && b[0]["conectado"] == true)).await;
    assert_eq!(e["poblacion"], "mercs");
    assert_eq!(e["llegadas"]["plan"][0]["perfil"], "mercenario");
    assert!(registros.path().join("bots-prueba.json").exists() && registros.path().join("mercs-prueba.json").exists());

    assert_eq!(mandar(&mut mercs, json!({"id": 3, "accion": "forzarLlegada"})).await["ok"], true);
    estado(&mut mercs, "llega otro mercenario", |e| e["bots"].as_array().is_some_and(|b| b.len() == 2)).await;
    estado(&mut libres, "los libres no se enteran", |e| e["bots"].as_array().is_some_and(|b| b.len() == 1)).await;

    assert_eq!(mandar(&mut mercs, json!({"id": 4, "accion": "parar"})).await["ok"], true);
    estado(&mut mercs, "mercenarios parados", |e| e["fase"] == "inactivo").await;
    estado(&mut libres, "los libres siguen", |e| e["fase"] == "corriendo").await;
    assert_eq!(mandar(&mut libres, json!({"id": 3, "accion": "parar"})).await["ok"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn el_registro_vive_en_la_carpeta_de_su_partida() {
    let datos = tempfile::tempdir().unwrap();
    let Some((_servidor, dir)) = comun::servidor(datos.path()).await else { return };
    let registros = tempfile::tempdir().unwrap();
    std::fs::create_dir(registros.path().join("tercios")).unwrap();
    // El admin manda el id de la partida; la carpeta lleva su nombre y el id dentro.
    std::fs::create_dir(registros.path().join("otra")).unwrap();
    std::fs::write(registros.path().join("otra").join("partida.id"), "p-1234\n").unwrap();
    let mut ws = panel(registros.path(), Poblacion::Mercenarios).await;
    let config = json!({"servidor": format!("http://{dir}"), "partida": "tercios", "total": 1, "diasLlegada": 1.0 / 86_400.0, "semilla": 3, "cadaMs": 1000, "horario": "siempre"});
    assert_eq!(mandar(&mut ws, json!({"id": 2, "accion": "iniciar", "config": config})).await["ok"], true);
    estado(&mut ws, "en marcha", |e| e["fase"] == "corriendo").await;
    assert!(registros.path().join("tercios").join("mercs.json").exists());
    let mut libres = panel(registros.path(), Poblacion::Libres).await;
    let por_id = json!({"servidor": format!("http://{dir}"), "partida": "p-1234", "total": 1, "diasLlegada": 1.0 / 86_400.0, "semilla": 3, "cadaMs": 1000, "horario": "siempre"});
    assert_eq!(mandar(&mut libres, json!({"id": 2, "accion": "iniciar", "config": por_id})).await["ok"], true);
    estado(&mut libres, "en marcha por id", |e| e["fase"] == "corriendo").await;
    assert!(registros.path().join("otra").join("bots.json").exists());
    assert_eq!(mandar(&mut libres, json!({"id": 3, "accion": "parar"})).await["ok"], true);
    assert_eq!(mandar(&mut ws, json!({"id": 3, "accion": "parar"})).await["ok"], true);
    estado(&mut ws, "parado", |e| e["fase"] == "inactivo").await;

    // Borrada la partida, un servicio nuevo no la retoma.
    std::fs::write(registros.path().join("servicio-mercs.json"), config.to_string()).unwrap();
    std::fs::remove_dir_all(registros.path().join("tercios")).unwrap();
    let s = Servicio::new(Ajustes { token: "secreto".into(), poblacion: Poblacion::Mercenarios, codigo_bots: "bots".into(), registros: registros.path().into(), cada_ms: 500, por_defecto: json!({}) });
    s.retomar().await;
    assert!(!registros.path().join("servicio-mercs.json").exists());
}
