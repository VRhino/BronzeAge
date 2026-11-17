//! La sesión con el motor: el login por HTTP (`POST /v1/sesiones` con la cuenta de servidor de batalla, que se da de alta sola si trae el código) y la sesión `CLIENTE_SERVIDOR_BATALLA` del contrato 05 por WebSocket
//! (05_Contrato_de_Sesion §9). Si se cae, vuelve a entrar con un login nuevo tras una espera; el motor repite al reconectar el ticket de las batallas que esperan servidor y
//! de las que ya tiene este.

use std::time::Duration;

use contrato::v1::{mensaje_de_cliente as mc, mensaje_de_servidor as ms, Hola, MensajeDeCliente, MensajeDeServidor, MensajeDeServidorDeBatalla, TipoCliente};
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
    time::timeout,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::Evento;

/// Un `POST` con un cuerpo JSON al HTTP del motor: el estado y el cuerpo de la respuesta.
async fn post(motor: &str, ruta: &str, cuerpo: serde_json::Value) -> Result<(u16, serde_json::Value), String> {
    let cuerpo = cuerpo.to_string();
    let mut tcp = timeout(Duration::from_secs(10), TcpStream::connect(motor)).await.map_err(|_| "el motor no contesta".to_owned())?.map_err(|e| format!("conectar con {motor}: {e}"))?;
    let peticion = format!("POST {ruta} HTTP/1.1\r\nHost: {motor}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cuerpo}", cuerpo.len());
    tcp.write_all(peticion.as_bytes()).await.map_err(|e| format!("{ruta}: {e}"))?;
    let mut respuesta = Vec::new();
    timeout(Duration::from_secs(30), tcp.read_to_end(&mut respuesta)).await.map_err(|_| format!("{ruta}: sin respuesta"))?.map_err(|e| format!("{ruta}: {e}"))?;
    let respuesta = String::from_utf8_lossy(&respuesta);
    let (cabecera, cuerpo) = respuesta.split_once("\r\n\r\n").ok_or_else(|| format!("{ruta}: respuesta sin cabecera"))?;
    let estado: u16 = cabecera.split(' ').nth(1).and_then(|e| e.parse().ok()).ok_or_else(|| format!("{ruta}: respuesta sin estado"))?;
    Ok((estado, serde_json::from_str(cuerpo).unwrap_or_default()))
}

/// Login en el servicio de acceso del motor: devuelve el token de la sesión. Con `codigo` (el `CODIGO_REGISTRO_SERVIDORES` del motor), si la cuenta no existe la da de
/// alta antes: así un lanzador nuevo no necesita que nadie le cree la cuenta.
pub async fn entrar(motor: &str, nick: &str, clave: &str, codigo: Option<&str>) -> Result<String, String> {
    let login = serde_json::json!({ "nick": nick, "clave": clave });
    let (mut estado, mut v) = post(motor, "/v1/sesiones", login.clone()).await?;
    if estado == 401 {
        if let Some(codigo) = codigo {
            let (alta, r) = post(motor, "/v1/registro", serde_json::json!({ "nick": nick, "clave": clave, "codigo": codigo })).await?;
            match alta {
                201 => (estado, v) = post(motor, "/v1/sesiones", login).await?,
                409 => return Err(format!("la cuenta «{nick}» ya existe y la clave no es la suya")),
                _ => return Err(format!("alta: {alta} {}", r["error"].as_str().unwrap_or(""))),
            }
        }
    }
    if estado != 200 {
        return Err(format!("login: {estado} {}", v["error"].as_str().unwrap_or("")));
    }
    if v["servidor"].as_bool() != Some(true) {
        return Err(format!("la cuenta «{nick}» no es de servidor de batalla (se registra con CODIGO_REGISTRO_SERVIDORES)"));
    }
    v["token"].as_str().map(str::to_owned).ok_or_else(|| "login: sin token".to_owned())
}

/// Una sesión con el motor, hasta que se cierre: manda `MotorConectado` con el canal de salida, cada mensaje de batalla que llega y, al cerrarse, nada (lo dice quien la llama).
async fn sesion(motor: &str, token: &str, eventos: &mpsc::UnboundedSender<Evento>) -> Result<(), String> {
    let (mut ws, _) = timeout(Duration::from_secs(10), connect_async(format!("ws://{motor}"))).await.map_err(|_| "el WebSocket no contesta".to_owned())?.map_err(|e| format!("WebSocket: {e}"))?;
    let hola = Hola {
        version_protocolo: 1,
        cliente: TipoCliente::ClienteServidorBatalla as i32,
        version_cliente: format!("bronzeage-lanzador {}", env!("CARGO_PKG_VERSION")),
        token: token.to_owned(),
        ..Default::default()
    };
    ws.send(Message::Binary(MensajeDeCliente { cuerpo: Some(mc::Cuerpo::Hola(hola)) }.encode_to_vec().into())).await.map_err(|e| format!("Hola: {e}"))?;
    let (tx, mut rx) = mpsc::channel::<MensajeDeServidorDeBatalla>(256);
    let mut bienvenida = false;
    loop {
        tokio::select! {
            m = ws.next() => {
                let bytes = match m {
                    Some(Ok(Message::Binary(b))) => b,
                    Some(Ok(Message::Close(_))) | None => return Err("el motor cerró la sesión".into()),
                    Some(Err(e)) => return Err(format!("WebSocket: {e}")),
                    Some(Ok(_)) => continue,
                };
                let m = MensajeDeServidor::decode(&bytes[..]).map_err(|e| format!("mensaje del motor: {e}"))?;
                match m.cuerpo {
                    Some(ms::Cuerpo::Bienvenida(_)) if !bienvenida => {
                        bienvenida = true;
                        let _ = eventos.send(Evento::MotorConectado(tx.clone()));
                    }
                    Some(ms::Cuerpo::Error(e)) => return Err(format!("el motor rechazó la sesión: {} ({})", e.codigo, e.mensaje)),
                    Some(ms::Cuerpo::Batalla(b)) => {
                        if let Some(c) = b.cuerpo {
                            let _ = eventos.send(Evento::Motor(Box::new(c)));
                        }
                    }
                    _ => {}
                }
            }
            Some(m) = rx.recv() => {
                let bytes = MensajeDeCliente { cuerpo: Some(mc::Cuerpo::Batalla(m)) }.encode_to_vec();
                ws.send(Message::Binary(bytes.into())).await.map_err(|e| format!("WebSocket: {e}"))?;
            }
        }
    }
}

/// Entra y mantiene la sesión con el motor para siempre: tras cada caída, `MotorCaido` y otra vez, pasada `espera`.
pub async fn mantener(motor: String, nick: String, clave: String, codigo: Option<String>, espera: Duration, eventos: mpsc::UnboundedSender<Evento>) {
    loop {
        let motivo = match entrar(&motor, &nick, &clave, codigo.as_deref()).await {
            Ok(token) => sesion(&motor, &token, &eventos).await.err().unwrap_or_default(),
            Err(e) => e,
        };
        if eventos.send(Evento::MotorCaido(motivo)).is_err() {
            return;
        }
        tokio::time::sleep(espera).await;
    }
}
