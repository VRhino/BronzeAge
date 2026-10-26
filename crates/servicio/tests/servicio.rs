//! El proceso del servidor de punta a punta (7B): una cuenta de bot se da de alta y entra por HTTP, abre su sesión del contrato 05 con el token y, tras un reinicio con la misma
//! carpeta de datos, la partida y el token siguen. Con el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`; se omite si no está) el bot además crea su héroe.

use std::{net::SocketAddr, path::Path, sync::Arc};

use contrato::almacen::Almacen;
use contrato::v1::{self as pb, mensaje_de_cliente as mc, mensaje_de_servidor as ms, orden::Cuerpo as O, AristaGrafo, GrafoNavegacion, NodoGrafo, TipoCliente, TipoEntidad};
use nucleo::{Ajustes, Arranque, Reglas};
use serde_json::{json, Value};
use servidor::prueba::{hola, ClienteDeEjemplo, ClientePrueba};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn acceso() -> acceso::Config {
    acceso::Config { codigo_registro_bots: Some("bots".into()), coste: acceso::Coste::PRUEBAS, ..acceso::Config::default() }
}

fn libre() -> SocketAddr {
    "127.0.0.1:0".parse().unwrap()
}

/// Un mundo mínimo sin artefacto: cuatro nodos.
fn arranque_sintetico() -> Arranque {
    let mut g = GrafoNavegacion { espaciado_min: 8_000, ..Default::default() };
    for (i, (x, y)) in [(4_000, 4_000), (12_000, 4_000), (4_000, 12_000), (12_000, 12_000)].into_iter().enumerate() {
        g.nodos.push(NodoGrafo { id: i as u32, posicion: Some(pb::Punto { x, y }) });
    }
    for (a, b) in [(0, 1), (0, 2), (1, 3), (2, 3)] {
        g.aristas.push(AristaGrafo { a, b, coste_milesimas: 1000, vado: false });
    }
    Arranque {
        reglas: Reglas::default(),
        ajustes: Ajustes::default(),
        semilla: 42,
        inicio: 0,
        rutas: Arc::new(rutas::Rutas::desde_grafo(&g)),
        mundo_hash: "hash-de-prueba".into(),
        mundo: None,
    }
}

async fn post(dir: SocketAddr, ruta: &str, v: Value) -> (u16, Value) {
    let mut tcp = tokio::net::TcpStream::connect(dir).await.unwrap();
    let cuerpo = v.to_string();
    let p = format!("POST {ruta} HTTP/1.1\r\nHost: prueba\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{cuerpo}", cuerpo.len());
    tcp.write_all(p.as_bytes()).await.unwrap();
    let mut r = String::new();
    tcp.read_to_string(&mut r).await.unwrap();
    let (cabecera, cuerpo) = r.split_once("\r\n\r\n").unwrap();
    (cabecera.split(' ').nth(1).unwrap().parse().unwrap(), serde_json::from_str(cuerpo).unwrap_or(Value::Null))
}

/// Un `GET` a mano; devuelve el estado y los bytes del cuerpo.
async fn get(dir: SocketAddr, ruta: &str) -> (u16, Vec<u8>) {
    let mut tcp = tokio::net::TcpStream::connect(dir).await.unwrap();
    tcp.write_all(format!("GET {ruta} HTTP/1.1\r\nHost: prueba\r\n\r\n").as_bytes()).await.unwrap();
    let mut r = Vec::new();
    tcp.read_to_end(&mut r).await.unwrap();
    let fin = r.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let cabecera = String::from_utf8_lossy(&r[..fin]).into_owned();
    (cabecera.split(' ').nth(1).unwrap().parse().unwrap(), r[fin + 4..].to_vec())
}

/// Alta (si `alta`) y login de un bot; devuelve su token.
async fn token_de_bot(dir: SocketAddr, nick: &str, alta: bool) -> String {
    let datos = json!({ "nick": nick, "clave": "clave de bot", "codigo": "bots" });
    if alta {
        let (e, r) = post(dir, "/v1/registro", datos.clone()).await;
        assert_eq!((e, r["bot"].as_bool()), (201, Some(true)), "{r}");
    }
    let (e, s) = post(dir, "/v1/sesiones", datos).await;
    assert_eq!(e, 200, "{s}");
    s["token"].as_str().unwrap().to_owned()
}

async fn bienvenida(dir: SocketAddr, token: &str) -> pb::Bienvenida {
    let mut c = ClientePrueba::conectar(dir).await;
    c.enviar(mc::Cuerpo::Hola(hola(token, TipoCliente::ClienteBot))).await;
    let Some(ms::Cuerpo::Bienvenida(b)) = c.esperar().await.cuerpo else { panic!("se esperaba la Bienvenida") };
    c.cerrar().await;
    b
}

#[tokio::test(flavor = "multi_thread")]
async fn un_bot_entra_por_el_acceso_real_y_sigue_tras_un_reinicio() {
    let datos = tempfile::tempdir().unwrap();
    let (s, i) = servicio::arrancar(arranque_sintetico(), None, datos.path(), libre(), acceso()).await.unwrap();
    assert!(!i.recuperado);
    let dir = s.direccion;
    let token = token_de_bot(dir, "bot-1", true).await;
    let b = bienvenida(dir, &token).await;
    assert!(b.heroe_id.is_none(), "una cuenta nueva aún no tiene héroe");
    // Los datos de juego: anunciados en la `Bienvenida` y servidos por versión (y el vigente, sin versión).
    let info = b.mundo.expect("InfoMundo");
    assert_eq!(info.url_datos_de_juego, format!("/v1/datos/{}", info.version_datos_de_juego));
    let (estado, cuerpo) = get(dir, &info.url_datos_de_juego).await;
    assert_eq!(estado, 200);
    assert_eq!(get(dir, "/v1/datos").await, (200, cuerpo.clone()));
    assert_eq!(get(dir, "/v1/datos/otra").await.0, 404);
    let v: Value = serde_json::from_slice(&cuerpo).unwrap();
    assert!(v["documentos"]["Fundacion_Y_Refundacion"]["FUNDACION"].is_object(), "los bloques de Mechanics, por documento");
    assert!(v["documentos"]["Proyecciones_Y_Autorizacion"]["ACCESO"]["duracionSesionMs"].is_number());
    assert_eq!(v["reglas"]["territorio"]["separacion_plazas"], json!(30_000));
    assert_eq!(v["huella_reglas"].as_str().map(str::len), Some(64));
    // Los costes derivados, por recurso en milésimas: los mismos que cobra el núcleo.
    let r = Reglas::default();
    for (clave, coste) in [("costo_caravana_fundacion", r.costo_caravana_fundacion()), ("costo_refundacion", r.costo_refundacion())] {
        let esperado: serde_json::Map<String, Value> = coste.iter().map(|(x, &n)| (x.nombre().to_owned(), json!(n))).collect();
        assert_eq!(v["derivados"][clave], Value::Object(esperado), "{clave}");
    }
    assert_eq!(v["derivados"]["costo_caravana_fundacion"]["madera"], json!(160_000));
    assert_eq!(v["derivados"]["costo_refundacion"]["madera"], json!(64_000));
    // Un token que no es de este servicio no entra.
    let mut c = ClientePrueba::conectar(dir).await;
    c.enviar(mc::Cuerpo::Hola(hola("ok:bot-1", TipoCliente::ClienteBot))).await;
    let Some(ms::Cuerpo::Error(e)) = c.esperar().await.cuerpo else { panic!("se esperaba el error") };
    assert_eq!(e.codigo, "token_invalido");
    let version = s.parar().await.version();

    let (s, i) = servicio::arrancar(arranque_sintetico(), None, datos.path(), libre(), acceso()).await.unwrap();
    assert!(i.recuperado && i.version == version && i.cuentas == 1, "{i:?}");
    bienvenida(s.direccion, &token).await;
    s.parar().await;
}

fn mundo_real() -> Option<std::path::PathBuf> {
    servicio::buscar_mundo(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generador/salida/mundo")).ok()
}

#[tokio::test(flavor = "multi_thread")]
async fn con_el_mundo_real_el_bot_crea_su_heroe_y_lo_conserva() {
    let Some(f) = mundo_real() else {
        eprintln!("sin artefacto de mundo real: se omite");
        return;
    };
    let datos = tempfile::tempdir().unwrap();
    let a = servicio::arranque_de(&f, 42).unwrap();
    let bytes = a.bytes.clone();
    let (s, _) = servicio::arrancar(a.arranque, Some(a.bytes.clone()), datos.path(), libre(), acceso()).await.unwrap();
    let token = token_de_bot(s.direccion, "bot-1", true).await;
    let mut bot = ClienteDeEjemplo::entrar(s.direccion, hola(&token, TipoCliente::ClienteBot), Almacen::default()).await;
    // La `Bienvenida` dice dónde bajar el artefacto, y el mismo puerto lo sirve tal cual.
    let info = bot.bienvenida.mundo.clone().expect("InfoMundo");
    assert_eq!(info.url_artefacto, format!("/v1/mundo/{}", info.hash));
    let (estado, cuerpo) = get(s.direccion, &info.url_artefacto).await;
    assert_eq!((estado, cuerpo.len()), (200, bytes.len()));
    assert!(cuerpo == *bytes, "el artefacto servido es el del disco");
    assert_eq!(get(s.direccion, "/v1/mundo/otro-hash").await.0, 404);
    let camp = bot.bienvenida.campamentos_para_elegir.first().expect("un campamento para nacer").campamento_id.clone();
    let r = bot.aceptada(O::CrearHeroe(pb::OrdenCrearHeroe { display_name: "Bot 1".into(), campamento_id: camp, ..Default::default() })).await;
    let heroe = r.creadas.iter().find(|c| c.entidad == TipoEntidad::Heroe as i32).expect("nació un héroe").id.clone();
    bot.sesion.cerrar().await;
    s.parar().await;

    let a = servicio::arranque_de(&f, 42).unwrap();
    let (s, i) = servicio::arrancar(a.arranque, Some(a.bytes), datos.path(), libre(), acceso()).await.unwrap();
    assert!(i.recuperado);
    let token = token_de_bot(s.direccion, "bot-1", false).await;
    assert_eq!(bienvenida(s.direccion, &token).await.heroe_id.as_deref(), Some(heroe.as_str()));
    s.parar().await;
}

/// El archivo de datos de juego lleva todos los documentos de Mechanics con bloques de parámetros (menos el README).
#[test]
fn los_datos_de_juego_llevan_todos_los_documentos_con_bloques() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Docs/Mechanics");
    let mut con_bloques: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
        .filter(|e| std::fs::read_to_string(e.path()).unwrap().contains("<!-- parametros:"))
        .map(|e| e.path().file_stem().unwrap().to_string_lossy().into_owned())
        .filter(|n| n != "README")
        .collect();
    con_bloques.sort();
    let d = servicio::datos::datos_de_juego(&Reglas::default());
    let v: Value = serde_json::from_slice(&d.json).unwrap();
    let incluidos: Vec<String> = v["documentos"].as_object().unwrap().keys().cloned().collect();
    assert_eq!(incluidos, con_bloques, "un documento de Mechanics con bloques nuevo va en servicio::datos::DOCUMENTOS");
    assert_eq!(d.version, servicio::datos::datos_de_juego(&Reglas::default()).version, "la versión es estable");
}
