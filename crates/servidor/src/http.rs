//! Lo poco de HTTP que comparte el puerto del WebSocket (7B): el servicio de acceso (`POST /v1/registro`, `POST /v1/sesiones`) y la salud. El contrato 05 no lo usa.
//!
//! Cada conexión empieza leyendo la cabecera de la petición. Si pide `Upgrade: websocket`, lo leído se devuelve al flujo ([`Rebobinado`]) y sigue el WebSocket de siempre; si no,
//! se contesta una sola petición HTTP/1.1 y se cierra (`Connection: close`). `OPTIONS` se contesta aquí (CORS abierto: el admin web llama desde el navegador).

use std::{
    future::Future,
    io,
    pin::Pin,
    task::{Context, Poll},
};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf},
    net::TcpStream,
};

/// Tamaño máximo de la cabecera de una petición.
pub const MAX_CABECERA: usize = 16 << 10;
/// Tamaño máximo del cuerpo de una petición HTTP.
pub const MAX_CUERPO: usize = 64 << 10;

/// Una petición HTTP ya leída.
#[derive(Clone, Debug)]
pub struct PeticionHttp {
    pub metodo: String,
    /// La ruta sin la consulta (`/v1/registro`).
    pub ruta: String,
    /// Las cabeceras con el nombre en minúsculas.
    pub cabeceras: Vec<(String, String)>,
    pub cuerpo: Vec<u8>,
}

impl PeticionHttp {
    pub fn cabecera(&self, nombre: &str) -> Option<&str> {
        self.cabeceras.iter().find(|(k, _)| k == nombre).map(|(_, v)| v.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RespuestaHttp {
    pub estado: u16,
    pub tipo: &'static str,
    pub cuerpo: Vec<u8>,
}

impl RespuestaHttp {
    pub fn json(estado: u16, cuerpo: impl Into<Vec<u8>>) -> Self {
        Self { estado, tipo: "application/json", cuerpo: cuerpo.into() }
    }
    pub fn texto(estado: u16, cuerpo: impl Into<Vec<u8>>) -> Self {
        Self { estado, tipo: "text/plain; charset=utf-8", cuerpo: cuerpo.into() }
    }
}

/// Quien atiende las peticiones HTTP del puerto. Con la caja del futuro para poder guardarlo como `dyn`.
pub trait Http: Send + Sync + 'static {
    fn atender(&self, p: PeticionHttp) -> Pin<Box<dyn Future<Output = RespuestaHttp> + Send + '_>>;
}

/// Sin servicio HTTP: solo `GET /salud`; el resto, 404.
pub struct SinHttp;

impl Http for SinHttp {
    fn atender(&self, _: PeticionHttp) -> Pin<Box<dyn Future<Output = RespuestaHttp> + Send + '_>> {
        Box::pin(async { RespuestaHttp::texto(404, "no encontrado") })
    }
}

/// Lo que empezó la conexión.
pub(crate) enum Inicio {
    WebSocket(Rebobinado),
    Http(TcpStream, PeticionHttp),
    /// Mal formada, demasiado grande o cortada: se cierra sin más.
    Nada,
}

/// Lee la cabecera (y, si no es un WebSocket, el cuerpo) de la primera petición.
pub(crate) async fn leer_inicio(mut tcp: TcpStream) -> Inicio {
    let mut buf = Vec::with_capacity(1024);
    let fin = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        if buf.len() > MAX_CABECERA {
            return Inicio::Nada;
        }
        let mut trozo = [0u8; 2048];
        match tcp.read(&mut trozo).await {
            Ok(0) | Err(_) => return Inicio::Nada,
            Ok(n) => buf.extend_from_slice(&trozo[..n]),
        }
    };
    let Ok(cabecera) = std::str::from_utf8(&buf[..fin]) else { return Inicio::Nada };
    let mut lineas = cabecera.split("\r\n");
    let mut primera = lineas.next().unwrap_or("").split(' ');
    let (Some(metodo), Some(objetivo)) = (primera.next(), primera.next()) else { return Inicio::Nada };
    let ruta = objetivo.split('?').next().unwrap_or("").to_owned();
    let cabeceras: Vec<(String, String)> = lineas
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    let es_ws = cabeceras.iter().any(|(k, v)| k == "upgrade" && v.eq_ignore_ascii_case("websocket"));
    if es_ws {
        return Inicio::WebSocket(Rebobinado { previo: buf, pos: 0, tcp });
    }
    let largo = cabeceras.iter().find(|(k, _)| k == "content-length").and_then(|(_, v)| v.parse::<usize>().ok()).unwrap_or(0);
    if largo > MAX_CUERPO {
        return Inicio::Nada;
    }
    let mut cuerpo = buf[fin..].to_vec();
    while cuerpo.len() < largo {
        let mut trozo = vec![0u8; largo - cuerpo.len()];
        match tcp.read(&mut trozo).await {
            Ok(0) | Err(_) => return Inicio::Nada,
            Ok(n) => cuerpo.extend_from_slice(&trozo[..n]),
        }
    }
    cuerpo.truncate(largo);
    Inicio::Http(tcp, PeticionHttp { metodo: metodo.to_owned(), ruta, cabeceras, cuerpo })
}

fn razon(estado: u16) -> &'static str {
    match estado {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    }
}

/// Atiende la petición (`OPTIONS` y `GET /salud` aquí; el resto, `http`) y cierra.
pub(crate) async fn contestar(mut tcp: TcpStream, p: PeticionHttp, http: &dyn Http) {
    let r = match (p.metodo.as_str(), p.ruta.as_str()) {
        ("OPTIONS", _) => RespuestaHttp { estado: 204, tipo: "text/plain", cuerpo: Vec::new() },
        ("GET", "/salud") => RespuestaHttp::texto(200, "ok"),
        _ => http.atender(p).await,
    };
    let cabecera = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\nAccess-Control-Allow-Headers: content-type, authorization\r\nConnection: close\r\n\r\n",
        r.estado,
        razon(r.estado),
        r.tipo,
        r.cuerpo.len()
    );
    let _ = tcp.write_all(cabecera.as_bytes()).await;
    let _ = tcp.write_all(&r.cuerpo).await;
    let _ = tcp.shutdown().await;
}

/// El flujo TCP con lo ya leído delante: el handshake del WebSocket vuelve a leer la cabecera.
pub struct Rebobinado {
    previo: Vec<u8>,
    pos: usize,
    tcp: TcpStream,
}

impl AsyncRead for Rebobinado {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        if self.pos < self.previo.len() {
            let n = (self.previo.len() - self.pos).min(buf.remaining());
            let desde = self.pos;
            buf.put_slice(&self.previo[desde..desde + n]);
            self.pos += n;
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut self.tcp).poll_read(cx, buf)
    }
}

impl AsyncWrite for Rebobinado {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.tcp).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.tcp).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.tcp).poll_shutdown(cx)
    }
}
