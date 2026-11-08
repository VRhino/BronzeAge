//! Las rutas de administración del proceso (fase 7, a petición del admin web): solo desde la máquina del servidor o una IP de `BRONZEAGE_IPS_ADMIN` (si no, `403 ip_no_permitida`;
//! detrás de un proxy inverso en la misma máquina todas las conexiones parecen locales y la protección es solo la cuenta) y con el token de una cuenta con la marca de administrador
//! (`Authorization: sesion <token>`; la marca se da con el código de un solo uso del arranque, con estas rutas o con `admin dar <nick>` por la entrada estándar). Sin token o con uno
//! caducado, `401 token_invalido`; con el de otra cuenta, `403 no_autorizado`.
//!
//! | Ruta | Qué |
//! |---|---|
//! | `GET /v1/admin/partida` | la partida que sirve el proceso: carpeta, id (`partida.id`, el `gameId` del contrato de batalla), hash del mundo, semilla, versión e instante del núcleo, cuentas y bots, código de registro |
//! | `GET /v1/admin/registro/codigo` | `{ codigo }`: el código de invitación vigente (`null`: registro abierto) |
//! | `PUT /v1/admin/registro/codigo` | `{ codigo }` (texto, o `null` para abrir el registro): lo cambia en caliente, hasta el próximo reinicio (luego vuelve `CODIGO_REGISTRO`) |
//! | `POST /v1/admin/respaldos` | respaldo ahora: foto y copia de `mundo/` y `acceso/` en `<datos>/respaldos/<id>/`; `201` con el respaldo |
//! | `GET /v1/admin/respaldos` | `{ respaldos: [...] }`, del más nuevo al más viejo |
//! | `GET /v1/admin/carga` | la medida de carga del actor: `{ activo, detalle, cada_s, ultima }`, con la última ventana de `cada_s` segundos (ver `Docs/Arquitectura/Medida_De_Carga.md`) |
//! | `POST /v1/admin/carga` | `{ activo?, detalle? }` (booleanos): la enciende o la apaga en caliente, sin reiniciar; contesta como el `GET` |
//! | `GET /v1/admin/misiones/catalogo` | el catálogo **vigente** de las misiones recurrentes (Misiones_Recurrentes §6), en el JSON que acepta la orden de administración `fijar_catalogo_misiones`: `{ asentamiento: [...], faccion: [...] }` |
//! | `GET /v1/admin/administradores` | `{ administradores: [nick, ...] }`, ordenados |
//! | `PUT /v1/admin/administradores` | `{ nick, admin }`: da (`true`) o quita (`false`) la marca de administrador a la cuenta; `200 { nick, admin }`. `404 no_encontrado` si no existe, `400 es_bot` si es de bot, `409 ultimo_administrador` si se quita al único |
//!
//! Un respaldo es `{ id, creado_ms, version, instante, mundo_hash, semilla, bytes }` (también en su `respaldo.json`). Restaurarlo es copiar sus `mundo/` y `acceso/` sobre los de la carpeta
//! de datos con el servidor parado: lo hace el panel de arranque (`scripts/panel.mjs`), que es quien para y arranca el proceso.

use std::{
    io,
    net::IpAddr,
    path::{Path, PathBuf},
};

use acceso::{ahora_ms, token_de, Acceso, ErrorAdmin};
use serde_json::{json, Value};
use servidor::{ip_de_administracion, ComandoSesion, EstadoNucleo, PeticionAdmin, PeticionHttp, RespuestaHttp};
use tokio::sync::{mpsc, oneshot};

/// El prefijo de las rutas de administración.
pub const RUTA_ADMIN: &str = "/v1/admin/";
/// La subcarpeta de los respaldos dentro de la carpeta de datos.
pub const RESPALDOS: &str = "respaldos";

/// Lo que las rutas necesitan del proceso.
pub(crate) struct Admin {
    pub acceso: Acceso,
    pub nucleo: mpsc::Sender<ComandoSesion>,
    pub carpeta: PathBuf,
    pub mundo_hash: String,
    pub semilla: u64,
    /// El `gameId` de la partida (`partida.id`).
    pub id_partida: String,
    /// Las IPs desde las que se administra además de esta máquina (`BRONZEAGE_IPS_ADMIN`).
    pub ips_admin: Vec<IpAddr>,
}

fn error(estado: u16, codigo: &str, mensaje: &str) -> RespuestaHttp {
    RespuestaHttp::json(estado, json!({ "error": codigo, "mensaje": mensaje }).to_string())
}

fn tamano(dir: &Path) -> u64 {
    std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| if e.path().is_dir() { tamano(&e.path()) } else { e.metadata().map_or(0, |m| m.len()) }).sum()
}

impl Admin {
    async fn estado_nucleo(&self) -> Option<EstadoNucleo> {
        let (tx, rx) = oneshot::channel();
        self.nucleo.send(ComandoSesion::Admin(PeticionAdmin::Estado { respuesta: tx })).await.ok()?;
        rx.await.ok()
    }

    fn respaldos(&self) -> Vec<Value> {
        let mut v: Vec<Value> = std::fs::read_dir(self.carpeta.join(RESPALDOS))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| serde_json::from_slice::<Value>(&std::fs::read(e.path().join("respaldo.json")).ok()?).ok())
            .collect();
        v.sort_by_key(|r| std::cmp::Reverse(r["creado_ms"].as_i64().unwrap_or(0)));
        v
    }

    async fn respaldar(&self) -> io::Result<Value> {
        let creado_ms = ahora_ms();
        let base = self.carpeta.join(RESPALDOS);
        let (mut id, mut n) = (format!("r-{creado_ms}"), 1);
        while base.join(&id).exists() {
            n += 1;
            id = format!("r-{creado_ms}-{n}");
        }
        let destino = base.join(&id);
        let (tx, rx) = oneshot::channel();
        let pedir = PeticionAdmin::Respaldo { destino: destino.join("mundo"), respuesta: tx };
        self.nucleo.send(ComandoSesion::Admin(pedir)).await.map_err(|_| io::Error::other("el núcleo no responde"))?;
        let e = rx.await.map_err(|_| io::Error::other("el núcleo no responde"))??;
        let (acceso, dir) = (self.acceso.clone(), destino.clone());
        tokio::task::spawn_blocking(move || acceso.respaldar(&dir.join("acceso"))).await.map_err(io::Error::other)??;
        let r = json!({
            "id": id, "creado_ms": creado_ms, "version": e.version, "instante": e.instante,
            "mundo_hash": self.mundo_hash, "semilla": self.semilla, "bytes": tamano(&destino),
        });
        std::fs::write(destino.join("respaldo.json"), r.to_string())?;
        Ok(r)
    }

    pub async fn atender(&self, p: PeticionHttp) -> RespuestaHttp {
        if !p.ip.is_some_and(|ip| ip_de_administracion(ip, &self.ips_admin)) {
            return error(403, "ip_no_permitida", "la administración solo se atiende desde la máquina del servidor o una IP de BRONZEAGE_IPS_ADMIN");
        }
        let Some(token) = token_de(&p) else { return error(401, "token_invalido", "falta Authorization: sesion <token>") };
        let ahora = ahora_ms();
        if self.acceso.administrador(token, ahora).is_none() {
            return match self.acceso.sesion(token, ahora) {
                Some(_) => error(403, "no_autorizado", "solo para administradores"),
                None => error(401, "token_invalido", "sin sesión o caducada"),
            };
        }
        let ruta = &p.ruta[RUTA_ADMIN.len()..];
        match (p.metodo.as_str(), ruta) {
            ("GET", "partida") => {
                let Some(e) = self.estado_nucleo().await else { return error(503, "interno", "el núcleo no responde") };
                let carpeta = std::path::absolute(&self.carpeta).unwrap_or(self.carpeta.clone());
                RespuestaHttp::json(
                    200,
                    json!({
                        "carpeta": carpeta.display().to_string(), "id": self.id_partida, "mundo_hash": self.mundo_hash, "semilla": self.semilla,
                        "version": e.version, "instante": e.instante, "cuentas": self.acceso.cuentas(), "bots": self.acceso.bots(),
                        "codigo_registro": self.acceso.codigo_registro(), "respaldos": self.respaldos().len(),
                    })
                    .to_string(),
                )
            }
            ("GET", "registro/codigo") => RespuestaHttp::json(200, json!({ "codigo": self.acceso.codigo_registro() }).to_string()),
            ("PUT", "registro/codigo") => {
                let Some(v) = serde_json::from_slice::<Value>(&p.cuerpo).ok().filter(|v| v.get("codigo").is_some_and(|c| c.is_string() || c.is_null())) else {
                    return error(400, "datos_invalidos", "el cuerpo es { \"codigo\": texto o null }");
                };
                self.acceso.cambiar_codigo_registro(v["codigo"].as_str().map(str::to_owned));
                RespuestaHttp::json(200, json!({ "codigo": self.acceso.codigo_registro() }).to_string())
            }
            ("GET", "administradores") => RespuestaHttp::json(200, json!({ "administradores": self.acceso.administradores() }).to_string()),
            ("PUT", "administradores") => {
                let v = serde_json::from_slice::<Value>(&p.cuerpo).unwrap_or_default();
                let (Some(nick), Some(admin)) = (v["nick"].as_str(), v["admin"].as_bool()) else {
                    return error(400, "datos_invalidos", "el cuerpo es { \"nick\": texto, \"admin\": booleano }");
                };
                match self.acceso.fijar_admin(nick, admin) {
                    Ok(c) => RespuestaHttp::json(200, json!({ "nick": c.nick, "admin": c.admin }).to_string()),
                    Err(ErrorAdmin::NoExiste) => error(404, "no_encontrado", "no hay ninguna cuenta con ese nick"),
                    Err(ErrorAdmin::EsBot) => error(400, "es_bot", "una cuenta de bot no puede ser administradora"),
                    Err(ErrorAdmin::UltimoAdministrador) => error(409, "ultimo_administrador", "no se puede quitar al último administrador"),
                    Err(ErrorAdmin::Disco) => error(500, "interno", "no se pudo guardar la cuenta"),
                }
            }
            ("GET", "misiones/catalogo") => {
                let (tx, rx) = oneshot::channel();
                if self.nucleo.send(ComandoSesion::Admin(PeticionAdmin::CatalogoMisiones { respuesta: tx })).await.is_err() {
                    return error(503, "interno", "el núcleo no responde");
                }
                match rx.await {
                    Ok(json) => RespuestaHttp::json(200, json),
                    Err(_) => error(503, "interno", "el núcleo no responde"),
                }
            }
            (metodo @ ("GET" | "POST"), "carga") => {
                let (activo, detalle) = if metodo == "POST" {
                    let Some(v) = serde_json::from_slice::<Value>(&p.cuerpo).ok().filter(|v| v.is_object() && ["activo", "detalle"].iter().all(|k| v.get(k).is_none_or(Value::is_boolean))) else {
                        return error(400, "datos_invalidos", "el cuerpo es { \"activo\"?: booleano, \"detalle\"?: booleano }");
                    };
                    (v.get("activo").and_then(Value::as_bool), v.get("detalle").and_then(Value::as_bool))
                } else {
                    (None, None)
                };
                let (tx, rx) = oneshot::channel();
                if self.nucleo.send(ComandoSesion::Admin(PeticionAdmin::Carga { activo, detalle, respuesta: tx })).await.is_err() {
                    return error(503, "interno", "el núcleo no responde");
                }
                match rx.await {
                    Ok(json) => RespuestaHttp::json(200, json),
                    Err(_) => error(503, "interno", "el núcleo no responde"),
                }
            }
            ("GET", "respaldos") => RespuestaHttp::json(200, json!({ "respaldos": self.respaldos() }).to_string()),
            ("POST", "respaldos") => match self.respaldar().await {
                Ok(r) => RespuestaHttp::json(201, r.to_string()),
                Err(e) => error(500, "interno", &format!("no se pudo respaldar: {e}")),
            },
            (_, "partida" | "registro/codigo" | "respaldos" | "misiones/catalogo" | "carga" | "administradores") => error(405, "metodo_no_permitido", "método no permitido"),
            _ => error(404, "no_encontrado", "ruta desconocida"),
        }
    }
}
