//! Las rutas de administración del proceso (fase 7, a petición del admin web): solo con el token de una cuenta de administrador (`Authorization: sesion <token>` de un nick de
//! `ADMINISTRADORES`). Sin token o con uno caducado, `401 token_invalido`; con el de otra cuenta, `403 no_autorizado`.
//!
//! | Ruta | Qué |
//! |---|---|
//! | `GET /v1/admin/partida` | la partida que sirve el proceso: carpeta, id (`partida.id`, el `gameId` del contrato de batalla), hash del mundo, semilla, versión e instante del núcleo, cuentas y bots, código de registro |
//! | `GET /v1/admin/registro/codigo` | `{ codigo }`: el código de invitación vigente (`null`: registro abierto) |
//! | `PUT /v1/admin/registro/codigo` | `{ codigo }` (texto, o `null` para abrir el registro): lo cambia en caliente, hasta el próximo reinicio (luego vuelve `CODIGO_REGISTRO`) |
//! | `POST /v1/admin/respaldos` | respaldo ahora: foto y copia de `mundo/` y `acceso/` en `<datos>/respaldos/<id>/`; `201` con el respaldo |
//! | `GET /v1/admin/respaldos` | `{ respaldos: [...] }`, del más nuevo al más viejo |
//!
//! Un respaldo es `{ id, creado_ms, version, instante, mundo_hash, semilla, bytes }` (también en su `respaldo.json`). Restaurarlo es copiar sus `mundo/` y `acceso/` sobre los de la carpeta
//! de datos con el servidor parado: lo hace el panel de arranque (`scripts/panel.mjs`), que es quien para y arranca el proceso.

use std::{
    io,
    path::{Path, PathBuf},
};

use acceso::{ahora_ms, token_de, Acceso};
use serde_json::{json, Value};
use servidor::{ComandoSesion, EstadoNucleo, PeticionAdmin, PeticionHttp, RespuestaHttp};
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
            ("GET", "respaldos") => RespuestaHttp::json(200, json!({ "respaldos": self.respaldos() }).to_string()),
            ("POST", "respaldos") => match self.respaldar().await {
                Ok(r) => RespuestaHttp::json(201, r.to_string()),
                Err(e) => error(500, "interno", &format!("no se pudo respaldar: {e}")),
            },
            (_, "partida" | "registro/codigo" | "respaldos") => error(405, "metodo_no_permitido", "método no permitido"),
            _ => error(404, "no_encontrado", "ruta desconocida"),
        }
    }
}
