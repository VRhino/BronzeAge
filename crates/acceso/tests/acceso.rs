//! El servicio de acceso (7B) por el puerto real: HTTP para el alta y el login, el WebSocket del contrato 05 con el token, y lo que sobrevive a un reinicio.

use std::net::SocketAddr;

use acceso::{Acceso, Config, Coste, ErrorLogin, ErrorRegistro};
use contrato::v1::{mensaje_de_cliente as mc, mensaje_de_servidor as ms, TipoCliente};
use serde_json::{json, Value};
use servidor::{
    prueba::{hola, ClientePrueba, NucleoFalso},
    servir_con_http, Autenticador, ErrorAcceso, RelojCompartido, RolTecnico, Servidor,
};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn config() -> Config {
    Config {
        codigo_registro: Some("humanos".into()),
        codigo_registro_bots: Some("bots".into()),
        administradores: vec!["Víctor".into()],
        coste: Coste::PRUEBAS,
        ..Config::default()
    }
}

async fn arrancar(acceso: Acceso) -> (Servidor, NucleoFalso) {
    let reloj = RelojCompartido::nuevo(0);
    let nucleo = NucleoFalso::arrancar(reloj.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let srv = servir_con_http(listener, nucleo.comandos.clone(), acceso.clone(), Arc::new(acceso), reloj, Default::default()).await;
    (srv, nucleo)
}

/// Una petición HTTP/1.1 a mano; devuelve el estado y el cuerpo.
async fn http(dir: SocketAddr, metodo: &str, ruta: &str, cabeceras: &[(&str, &str)], cuerpo: &str) -> (u16, String) {
    let mut tcp = tokio::net::TcpStream::connect(dir).await.unwrap();
    let mut p = format!("{metodo} {ruta} HTTP/1.1\r\nHost: prueba\r\nContent-Length: {}\r\n", cuerpo.len());
    for (k, v) in cabeceras {
        p.push_str(&format!("{k}: {v}\r\n"));
    }
    p.push_str("\r\n");
    p.push_str(cuerpo);
    tcp.write_all(p.as_bytes()).await.unwrap();
    let mut r = String::new();
    tcp.read_to_string(&mut r).await.unwrap();
    let (cabecera, cuerpo) = r.split_once("\r\n\r\n").expect("respuesta con cabecera");
    let estado = cabecera.split(' ').nth(1).unwrap().parse().unwrap();
    assert!(cabecera.contains("Access-Control-Allow-Origin: *"), "{cabecera}");
    (estado, cuerpo.to_owned())
}

async fn post(dir: SocketAddr, ruta: &str, v: Value) -> (u16, Value) {
    let (e, c) = http(dir, "POST", ruta, &[("Content-Type", "application/json")], &v.to_string()).await;
    (e, serde_json::from_str(&c).unwrap_or(Value::Null))
}

async fn registrar_y_entrar(dir: SocketAddr, nick: &str, codigo: &str) -> Value {
    let (e, r) = post(dir, "/v1/registro", json!({ "nick": nick, "clave": "una clave larga", "codigo": codigo })).await;
    assert_eq!(e, 201, "{r}");
    let (e, s) = post(dir, "/v1/sesiones", json!({ "nick": nick, "clave": "una clave larga" })).await;
    assert_eq!(e, 200, "{s}");
    s
}

/// Abre una sesión del contrato 05 y devuelve el código de error de sesión, si lo hubo.
async fn abrir(dir: SocketAddr, token: &str, tipo: TipoCliente) -> Result<(), String> {
    let mut c = ClientePrueba::conectar(dir).await;
    c.enviar(mc::Cuerpo::Hola(hola(token, tipo))).await;
    let r = match c.esperar().await.cuerpo {
        Some(ms::Cuerpo::Bienvenida(_)) => Ok(()),
        Some(ms::Cuerpo::Error(e)) => Err(e.codigo),
        o => panic!("llegó {o:?}"),
    };
    c.cerrar().await;
    r
}

#[tokio::test]
async fn alta_login_y_sesion_por_el_mismo_puerto() {
    let (srv, _n) = arrancar(Acceso::en_memoria(config())).await;
    let dir = srv.direccion;
    assert_eq!(http(dir, "GET", "/salud", &[], "").await, (200, "ok".into()));
    assert_eq!(http(dir, "OPTIONS", "/v1/registro", &[], "").await.0, 204);

    // Un bot: rol de jugador, solo `CLIENTE_BOT`.
    let s = registrar_y_entrar(dir, "  Bot-001 ", "bots").await;
    assert_eq!((s["nick"].as_str(), s["bot"].as_bool(), s["cuenta_id"].as_str()), (Some("bot-001"), Some(true), Some("u-1")));
    let token = s["token"].as_str().unwrap();
    assert_eq!(token.len(), 64);
    abrir(dir, token, TipoCliente::ClienteBot).await.unwrap();
    assert_eq!(abrir(dir, token, TipoCliente::ClienteAdminWeb).await, Err("rol_no_permitido".into()));
    assert_eq!(abrir(dir, token, TipoCliente::ClienteUnity).await, Err("rol_no_permitido".into()));
    assert_eq!(abrir(dir, "no-es-un-token", TipoCliente::ClienteBot).await, Err("token_invalido".into()));

    // Un humano con el código de invitación, y el administrador global (por nick).
    let h = registrar_y_entrar(dir, "ana", "humanos").await;
    assert_eq!(h["bot"], json!(false));
    abrir(dir, h["token"].as_str().unwrap(), TipoCliente::ClienteUnity).await.unwrap();
    assert_eq!(abrir(dir, h["token"].as_str().unwrap(), TipoCliente::ClienteBot).await, Err("rol_no_permitido".into()));
    let a = registrar_y_entrar(dir, "VÍCTOR", "humanos").await;
    abrir(dir, a["token"].as_str().unwrap(), TipoCliente::ClienteAdminWeb).await.unwrap();

    // La sesión actual por cabecera.
    let (e, c) = http(dir, "GET", "/v1/sesiones/actual", &[("Authorization", &format!("sesion {token}"))], "").await;
    assert_eq!(e, 200);
    assert_eq!(serde_json::from_str::<Value>(&c).unwrap()["cuenta_id"], json!("u-1"));
    assert_eq!(http(dir, "GET", "/v1/sesiones/actual", &[("Authorization", "sesion nada")], "").await.0, 401);

    // Los errores del alta y del login.
    let mal = |nick: &str, clave: &str, codigo: &str| json!({ "nick": nick, "clave": clave, "codigo": codigo });
    assert_eq!(post(dir, "/v1/registro", mal("ana", "otra clave larga", "humanos")).await.0, 409);
    assert_eq!(post(dir, "/v1/registro", mal("luis", "otra clave larga", "otro")).await.0, 403);
    assert_eq!(post(dir, "/v1/registro", mal("luis", "corta", "humanos")).await.0, 400);
    assert_eq!(post(dir, "/v1/registro", mal("l", "otra clave larga", "humanos")).await.0, 400);
    assert_eq!(post(dir, "/v1/registro", mal("l uis", "otra clave larga", "humanos")).await.0, 400);
    assert_eq!(post(dir, "/v1/registro", json!({ "nick": "luis" })).await.0, 400);
    assert_eq!(http(dir, "POST", "/v1/registro", &[], "no es json").await.0, 400);
    let (e, r) = post(dir, "/v1/sesiones", json!({ "nick": "ana", "clave": "no es la clave" })).await;
    assert_eq!((e, r["error"].as_str()), (401, Some("credenciales_invalidas")));
    assert_eq!(post(dir, "/v1/sesiones", json!({ "nick": "nadie", "clave": "una clave larga" })).await.0, 401);
    assert_eq!(http(dir, "GET", "/v1/registro", &[], "").await.0, 405);
    assert_eq!(http(dir, "GET", "/v1/otra", &[], "").await.0, 404);
    srv.detener().await;
}

#[tokio::test]
async fn sin_codigo_de_bots_no_hay_bots_y_sin_codigo_humano_el_registro_esta_abierto() {
    let a = Acceso::en_memoria(Config { coste: Coste::PRUEBAS, ..Config::default() });
    let c = a.registrar("cualquiera", "una clave larga", Some("bots")).await.unwrap();
    assert!(!c.bot);
    assert!(!a.registrar("otro", "una clave larga", None).await.unwrap().bot);
    let a = Acceso::en_memoria(Config { codigo_registro: Some("x".into()), coste: Coste::PRUEBAS, ..Config::default() });
    assert_eq!(a.registrar("otro", "una clave larga", None).await.unwrap_err(), ErrorRegistro::CodigoIncorrecto);
}

#[tokio::test]
async fn el_token_caduca_y_cada_login_da_uno_nuevo() {
    let a = Acceso::en_memoria(config());
    a.registrar("bot-1", "una clave larga", Some("bots")).await.unwrap();
    let dura = a.config().duracion_sesion_ms;
    assert_eq!(dura, 43_200_000, "ACCESO.duracionSesionMs de Proyecciones §2.1");
    let s1 = a.entrar_en("bot-1", "una clave larga", 1_000).await.unwrap();
    let s2 = a.entrar_en("BOT-1", "una clave larga", 2_000).await.unwrap();
    assert_ne!(s1.token, s2.token);
    assert_eq!(s1.expira_ms, 1_000 + dura);
    let i = a.identidad(&s1.token, 1_000 + dura - 1).unwrap();
    assert_eq!((i.cliente_id.as_str(), i.roles.as_slice(), i.rol), ("u-1", &[TipoCliente::ClienteBot][..], RolTecnico::Jugador));
    assert_eq!(a.identidad(&s1.token, 1_000 + dura).unwrap_err(), ErrorAcceso::Invalido);
    assert!(a.identidad(&s2.token, 1_000 + dura).is_ok());
    assert_eq!(a.entrar_en("bot-1", "otra", 0).await.unwrap_err(), ErrorLogin::Credenciales);
}

#[tokio::test]
async fn cuentas_y_tokens_sobreviven_a_un_reinicio() {
    let dir = tempfile::tempdir().unwrap();
    let token = {
        let a = Acceso::abrir(dir.path(), config()).unwrap();
        a.registrar("bot-1", "una clave larga", Some("bots")).await.unwrap();
        a.registrar("ana", "una clave larga", Some("humanos")).await.unwrap();
        // Uno caducado (se descarta al abrir) y uno vigente.
        a.entrar_en("ana", "una clave larga", 0).await.unwrap();
        a.entrar("bot-1", "una clave larga").await.unwrap().token
    };
    let a = Acceso::abrir(dir.path(), config()).unwrap();
    assert_eq!(a.cuentas(), 2);
    let i = a.autenticar(&token).await.unwrap();
    assert_eq!(i.cliente_id, "u-1");
    let sesiones = std::fs::read_to_string(dir.path().join("sesiones.jsonl")).unwrap();
    assert_eq!(sesiones.lines().count(), 1, "la caducada se quitó al abrir");
    assert!(!sesiones.contains(&token), "solo se guarda el hash del token");
    assert_eq!(a.registrar("ana", "una clave larga", Some("humanos")).await.unwrap_err(), ErrorRegistro::NickRepetido);
    // El id sigue la numeración y la clave se comprueba contra el hash guardado.
    assert_eq!(a.registrar("luis", "una clave larga", Some("humanos")).await.unwrap().id, "u-3");
    assert!(a.entrar("ana", "una clave larga").await.is_ok());

    // Una última línea cortada (el proceso murió escribiendo) no impide abrir.
    drop(a);
    let mut f = std::fs::OpenOptions::new().append(true).open(dir.path().join("cuentas.jsonl")).unwrap();
    std::io::Write::write_all(&mut f, b"{\"id\":\"u-4\",\"ni").unwrap();
    drop(f);
    let a = Acceso::abrir(dir.path(), config()).unwrap();
    assert_eq!(a.cuentas(), 3);
    assert_eq!(a.registrar("eva", "una clave larga", Some("humanos")).await.unwrap().id, "u-4");
    drop(a);
    assert_eq!(Acceso::abrir(dir.path(), config()).unwrap().cuentas(), 4, "la línea nueva no se pegó a la cortada");
}
