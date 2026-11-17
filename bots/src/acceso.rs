//! El servicio de acceso del servidor (`Docs/Coordinacion/08_Consumo_del_Contrato.md` del motor, «El servidor y el acceso»): alta y login por HTTP/1.1 en el mismo puerto que el
//! WebSocket. El servidor contesta y cierra, así que basta un `POST` a mano.

use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::Fallo;

/// Con lo que entra una cuenta: el nick (el servidor lo recorta y pasa a minúsculas) y la clave.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Credenciales {
    pub nick: String,
    pub clave: String,
}

/// La dirección de un servidor tal como se guarda: `host:puerto` a pelo (el de casa, sin cifrar) o `https://host[:puerto]` (uno publicado detrás de TLS, como Render).
/// Admite lo que le pasan el panel y la consola (`http://…`, `ws://…`, `https://…`, `wss://…`, con o sin `/` o `/v1` al final).
pub fn normalizar(s: &str) -> String {
    let s = s.trim().trim_end_matches('/').trim_end_matches("/v1").trim_end_matches('/');
    match s.strip_prefix("https://").or_else(|| s.strip_prefix("wss://")) {
        Some(resto) => format!("https://{resto}"),
        None => s.trim_start_matches("http://").trim_start_matches("ws://").into(),
    }
}

/// (cifrado, anfitrión, `host:puerto` al que conectar) de una dirección ya normalizada.
fn partes(dir: &str) -> (bool, &str, String) {
    match dir.strip_prefix("https://") {
        Some(resto) => (true, resto.split(':').next().unwrap_or(resto), if resto.contains(':') { resto.into() } else { format!("{resto}:443") }),
        None => (false, dir.split(':').next().unwrap_or(dir), dir.into()),
    }
}

/// La URL del WebSocket del contrato de ese servidor.
pub(crate) fn url_ws(dir: &str) -> String {
    match dir.strip_prefix("https://") {
        Some(resto) => format!("wss://{resto}/"),
        None => format!("ws://{dir}/"),
    }
}

/// La configuración TLS de cliente (raíces de Mozilla más las de `BOTS_RAIZ_TLS`, proveedor `ring`): la misma para el WebSocket y para el HTTP a mano.
pub(crate) fn tls() -> std::sync::Arc<rustls::ClientConfig> {
    static C: std::sync::OnceLock<std::sync::Arc<rustls::ClientConfig>> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        let mut raices = rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
        // `BOTS_RAIZ_TLS`: un fichero PEM con raíces de más (un servidor con certificado de una autoridad propia, o las pruebas).
        if let Ok(ruta) = std::env::var("BOTS_RAIZ_TLS") {
            use rustls::pki_types::pem::PemObject;
            match rustls::pki_types::CertificateDer::pem_file_iter(&ruta) {
                Ok(certs) => certs.flatten().for_each(|c| drop(raices.add(c))),
                Err(e) => eprintln!("BOTS_RAIZ_TLS ({ruta}): {e}"),
            }
        }
        let c = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .expect("versiones de TLS")
            .with_root_certificates(raices)
            .with_no_client_auth();
        std::sync::Arc::new(c)
    })
    .clone()
}

/// Manda la petición (cabecera y cuerpo ya montados, con `Connection: close`) y lee hasta que el servidor cierra: (estado, cuerpo). Un proxy por delante puede
/// trocear el cuerpo (`Transfer-Encoding: chunked`): se recompone.
async fn pedir(dir: &str, ruta: &str, peticion: impl FnOnce(&str) -> Vec<u8>) -> Result<(u16, Vec<u8>), Fallo> {
    let red = |e: std::io::Error| Fallo::Red(format!("{ruta}: {e}"));
    let (cifrado, anfitrion, destino) = partes(dir);
    let p = peticion(if cifrado { anfitrion } else { dir });
    let mut tcp = TcpStream::connect(&destino).await.map_err(red)?;
    let mut r = Vec::new();
    if cifrado {
        let nombre = rustls::pki_types::ServerName::try_from(anfitrion.to_string()).map_err(|e| Fallo::Red(format!("{ruta}: {e}")))?;
        let mut s = tokio_rustls::TlsConnector::from(tls()).connect(nombre, tcp).await.map_err(red)?;
        s.write_all(&p).await.map_err(red)?;
        // Un servidor que cierra sin el aviso de TLS da `UnexpectedEof` con la respuesta ya entera: vale lo leído.
        if let Err(e) = s.read_to_end(&mut r).await {
            if e.kind() != std::io::ErrorKind::UnexpectedEof || r.is_empty() {
                return Err(red(e));
            }
        }
    } else {
        tcp.write_all(&p).await.map_err(red)?;
        tcp.read_to_end(&mut r).await.map_err(red)?;
    }
    let fin = r.windows(4).position(|w| w == b"\r\n\r\n").ok_or_else(|| Fallo::Red(format!("{ruta}: respuesta sin cabecera")))?;
    let cabecera = String::from_utf8_lossy(&r[..fin]).to_ascii_lowercase();
    let estado: u16 = cabecera.split(' ').nth(1).and_then(|s| s.parse().ok()).ok_or_else(|| Fallo::Red(format!("{ruta}: estado ilegible")))?;
    let troceado = cabecera.lines().any(|l| l.starts_with("transfer-encoding:") && l.contains("chunked"));
    let cuerpo = r.split_off(fin + 4);
    Ok((estado, if troceado { destrocear(&cuerpo).ok_or_else(|| Fallo::Red(format!("{ruta}: cuerpo troceado ilegible")))? } else { cuerpo }))
}

/// Recompone un cuerpo `chunked`: `<tamaño en hex>[;ext]\r\n<datos>\r\n` hasta el trozo de tamaño 0.
fn destrocear(mut b: &[u8]) -> Option<Vec<u8>> {
    let mut salida = Vec::with_capacity(b.len());
    loop {
        let fin = b.windows(2).position(|w| w == b"\r\n")?;
        let n = usize::from_str_radix(std::str::from_utf8(&b[..fin]).ok()?.split(';').next()?.trim(), 16).ok()?;
        b = &b[fin + 2..];
        if n == 0 {
            return Some(salida);
        }
        salida.extend_from_slice(b.get(..n)?);
        b = b.get(n + 2..)?;
    }
}

async fn post(dir: &str, ruta: &str, v: Value) -> Result<(u16, Value), Fallo> {
    let cuerpo = v.to_string();
    let (estado, r) = pedir(dir, ruta, |host| {
        format!("POST {ruta} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cuerpo}", cuerpo.len()).into_bytes()
    })
    .await?;
    Ok((estado, serde_json::from_slice(&r).unwrap_or(Value::Null)))
}

fn rechazo(estado: u16, r: &Value) -> Fallo {
    Fallo::Acceso { estado, codigo: r["error"].as_str().unwrap_or("").into() }
}

/// Un `GET` del mismo servidor (el artefacto de mundo): el cuerpo, o el estado si no es 200.
pub(crate) async fn get(dir: &str, ruta: &str) -> Result<Vec<u8>, Fallo> {
    let (estado, r) = pedir(dir, ruta, |host| format!("GET {ruta} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").into_bytes()).await?;
    if estado != 200 {
        return Err(Fallo::Acceso { estado, codigo: ruta.into() });
    }
    Ok(r)
}

/// `POST /v1/registro`. Con el código de bots la cuenta nace de bot (sin él, una cuenta de bot no abre `CLIENTE_BOT`: `rol_no_permitido`). Un nick ya dado es `Fallo::Acceso{409, nick_repetido}`.
pub async fn registrar(dir: &str, c: &Credenciales, codigo: &str) -> Result<(), Fallo> {
    let (estado, r) = post(dir, "/v1/registro", json!({ "nick": c.nick, "clave": c.clave, "codigo": codigo })).await?;
    if estado != 201 {
        return Err(rechazo(estado, &r));
    }
    Ok(())
}

/// `POST /v1/sesiones`: un token nuevo (dura 12 h).
pub async fn iniciar_sesion(dir: &str, c: &Credenciales) -> Result<String, Fallo> {
    let (estado, r) = post(dir, "/v1/sesiones", json!({ "nick": c.nick, "clave": c.clave })).await?;
    match r["token"].as_str() {
        Some(t) if estado == 200 => Ok(t.into()),
        _ => Err(rechazo(estado, &r)),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_direccion_se_normaliza_y_conserva_el_cifrado() {
        assert_eq!(normalizar("http://localhost:7878/"), "localhost:7878");
        assert_eq!(normalizar("ws://127.0.0.1:7878"), "127.0.0.1:7878");
        assert_eq!(normalizar("127.0.0.1:7878"), "127.0.0.1:7878");
        assert_eq!(normalizar("https://motor.ejemplo.com/v1/"), "https://motor.ejemplo.com");
        assert_eq!(normalizar("wss://motor.ejemplo.com"), "https://motor.ejemplo.com");
        assert_eq!(partes("https://motor.ejemplo.com"), (true, "motor.ejemplo.com", "motor.ejemplo.com:443".to_string()));
        assert_eq!(partes("https://motor.ejemplo.com:8443"), (true, "motor.ejemplo.com", "motor.ejemplo.com:8443".to_string()));
        assert_eq!(partes("localhost:7878"), (false, "localhost", "localhost:7878".to_string()));
        assert_eq!(url_ws("https://motor.ejemplo.com"), "wss://motor.ejemplo.com/");
        assert_eq!(url_ws("localhost:7878"), "ws://localhost:7878/");
    }

    #[test]
    fn el_cuerpo_troceado_se_recompone() {
        assert_eq!(destrocear(b"4\r\nhola\r\n6;x=1\r\n mundo\r\n0\r\n\r\n").as_deref(), Some(&b"hola mundo"[..]));
        assert_eq!(destrocear(b"4\r\nho"), None);
    }
}
