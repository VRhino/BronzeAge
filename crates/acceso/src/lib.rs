//! El servicio de acceso mínimo de la fase 7 (tanda 7B): alta de cuentas con código, cuenta de bot, login con token opaco y el [`Autenticador`] del servidor.
//!
//! Sigue a `Docs/Mechanics/Proyecciones_Y_Autorizacion.md` §2.1 con lo justo para que bots y admin entren por el contrato 05 (el contrato no cambia: el token va en `Hola.token`):
//! - **Alta** (`POST /v1/registro`, `{nick, clave, codigo?}`): el nick se recorta y pasa a minúsculas, y es único. Con `codigo_registro_bots` configurado y coincidente, la cuenta es **de bot**
//!   (sin ningún privilegio: solo abre sesiones `CLIENTE_BOT`, como un jugador); si no, con `codigo_registro` configurado hace falta que coincida; sin él el registro está abierto. No da sesión.
//! - **Login** (`POST /v1/sesiones`, `{nick, clave}`): cada login emite un token nuevo que dura `ACCESO.duracionSesionMs`. Se guarda solo su SHA-256.
//! - **Quién es quién**: la cuenta de bot abre `CLIENTE_BOT`; la humana, `CLIENTE_UNITY`; la que figura en `administradores` (por nick), además `CLIENTE_ADMIN_WEB` con el rol de
//!   administrador global. `Identidad::cliente_id` es el id de la cuenta (`u-N`): el núcleo lo usa como cuenta del héroe.
//! - **Disco**: `cuentas.jsonl` y `sesiones.jsonl` en la carpeta del servicio, una línea por alta o por login, con `sync_data` antes de contestar. Al abrir se leen las dos y se reescribe
//!   `sesiones.jsonl` sin las caducadas: los tokens sobreviven a un reinicio. Las claves, con Argon2id.
//!
//! Lo que falta y no hace falta aún: membresías por partida (moderador y administrador de la partida), deshabilitar cuentas, cerrar sesión y los tokens de los servidores de batalla (fase 8).

use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    future::Future,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use argon2::{
    password_hash::{
        rand_core::{OsRng, RngCore},
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Algorithm, Argon2, Params, Version,
};
use contrato::v1::TipoCliente;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use servidor::{Autenticador, ErrorAcceso, Http, Identidad, PeticionHttp, RespuestaHttp, RolTecnico};
use sha2::{Digest, Sha256};

const PROYECCIONES: &str = include_str!("../../../Docs/Mechanics/Proyecciones_Y_Autorizacion.md");

/// El JSON del bloque `parametros:nombre` de un documento.
fn bloque(doc: &str, nombre: &str) -> Value {
    let marca = format!("<!-- parametros:{nombre} -->");
    let ini = doc.find(&marca).unwrap_or_else(|| panic!("no hay bloque «{nombre}»")) + marca.len();
    let resto = &doc[ini..];
    let fin = resto.find("<!-- /parametros -->").unwrap_or_else(|| panic!("el bloque «{nombre}» no se cierra"));
    let cuerpo = resto[..fin].trim().strip_prefix("```json").and_then(|c| c.strip_suffix("```")).expect("bloque json");
    serde_json::from_str(cuerpo).expect("bloque json válido")
}

/// `ACCESO.duracionSesionMs` (Proyecciones_Y_Autorizacion §2.1).
pub fn duracion_sesion_ms() -> i64 {
    bloque(PROYECCIONES, "ACCESO")["duracionSesionMs"].as_i64().expect("ACCESO.duracionSesionMs")
}

/// El coste de Argon2id: memoria en KiB y pasadas.
#[derive(Clone, Copy, Debug)]
pub struct Coste {
    pub memoria_kib: u32,
    pub pasadas: u32,
}

impl Coste {
    /// Los valores por defecto de Argon2id (19 MiB, 2 pasadas).
    pub const PRODUCCION: Coste = Coste { memoria_kib: Params::DEFAULT_M_COST, pasadas: Params::DEFAULT_T_COST };
    /// Barato, para las pruebas (en depuración el de producción tarda segundos).
    pub const PRUEBAS: Coste = Coste { memoria_kib: 8, pasadas: 1 };
}

#[derive(Clone, Debug)]
pub struct Config {
    /// Código de invitación de las cuentas humanas (`CODIGO_REGISTRO`); `None`, registro abierto.
    pub codigo_registro: Option<String>,
    /// Código que marca la cuenta como bot (`CODIGO_REGISTRO_BOTS`); `None`, no hay cuentas de bot.
    pub codigo_registro_bots: Option<String>,
    /// Nicks de los administradores globales (`ADMINISTRADORES`).
    pub administradores: Vec<String>,
    pub duracion_sesion_ms: i64,
    pub coste: Coste,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            codigo_registro: None,
            codigo_registro_bots: None,
            administradores: Vec::new(),
            duracion_sesion_ms: duracion_sesion_ms(),
            coste: Coste::PRODUCCION,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorRegistro {
    /// Nick o clave fuera de forma (400).
    Invalido(&'static str),
    /// Hace falta un código y no coincide (403).
    CodigoIncorrecto,
    /// El nick ya existe (409).
    NickRepetido,
    /// No se pudo escribir en disco (500).
    Disco,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorLogin {
    /// Nick que no existe o clave que no coincide (401): no se dice cuál.
    Credenciales,
    Disco,
}

/// Una cuenta.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cuenta {
    /// `u-N`.
    pub id: String,
    pub nick: String,
    /// La clave en formato PHC de Argon2id.
    clave: String,
    pub bot: bool,
    pub creada_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SesionGuardada {
    token_sha256: String,
    cuenta: String,
    expira_ms: i64,
}

/// Lo que devuelve un login.
#[derive(Clone, Debug)]
pub struct Sesion {
    pub token: String,
    pub cuenta: Cuenta,
    pub expira_ms: i64,
}

struct Ficheros {
    cuentas: File,
    sesiones: File,
}

#[derive(Default)]
struct Estado {
    cuentas: HashMap<String, Cuenta>,
    por_nick: HashMap<String, String>,
    sesiones: HashMap<String, SesionGuardada>,
    siguiente: u64,
    ficheros: Option<Ficheros>,
    /// El código de invitación vigente: el de `Config::codigo_registro` al abrir; lo cambia un administrador en caliente (no se guarda: un reinicio vuelve al de la configuración).
    codigo_registro: Option<String>,
}

impl Estado {
    fn insertar(&mut self, c: Cuenta) {
        if let Some(n) = c.id.strip_prefix("u-").and_then(|n| n.parse::<u64>().ok()) {
            self.siguiente = self.siguiente.max(n);
        }
        self.por_nick.insert(c.nick.clone(), c.id.clone());
        self.cuentas.insert(c.id.clone(), c);
    }
}

struct Interior {
    config: Config,
    /// La carpeta de los ficheros (`None`: en memoria).
    dir: Option<PathBuf>,
    estado: Mutex<Estado>,
}

/// El servicio. Barato de clonar: lo comparten el [`Autenticador`] y el [`Http`] del servidor.
#[derive(Clone)]
pub struct Acceso {
    interior: Arc<Interior>,
}

pub fn ahora_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

fn sha256(token: &str) -> String {
    Sha256::digest(token.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn normalizar(nick: &str) -> String {
    nick.trim().to_lowercase()
}

/// Forma del nick (ya normalizado) y de la clave. Decisión técnica (2026-10-26): de 3 a 32 caracteres, letras, cifras, `_`, `-` y `.`; la clave, de 8 a 128.
fn validar(nick: &str, clave: &str) -> Result<(), ErrorRegistro> {
    let n = nick.chars().count();
    if !(3..=32).contains(&n) || !nick.chars().all(|c| c.is_alphanumeric() || "_-.".contains(c)) {
        return Err(ErrorRegistro::Invalido("el nick tiene de 3 a 32 caracteres: letras, cifras, «_», «-» y «.»"));
    }
    if !(8..=128).contains(&clave.chars().count()) {
        return Err(ErrorRegistro::Invalido("la clave tiene de 8 a 128 caracteres"));
    }
    Ok(())
}

fn leer_lineas<T: for<'de> Deserialize<'de>>(f: &Path) -> io::Result<Vec<T>> {
    let Ok(abierto) = File::open(f) else { return Ok(Vec::new()) };
    let mut fuera = Vec::new();
    for linea in BufReader::new(abierto).lines() {
        let linea = linea?;
        if linea.trim().is_empty() {
            continue;
        }
        fuera.push(serde_json::from_str(&linea).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", f.display())))?);
    }
    Ok(fuera)
}

/// Quita una última línea sin terminar (el proceso murió escribiéndola), para que la siguiente no se pegue a ella.
fn recortar_cola(f: &Path) -> io::Result<()> {
    let Ok(bytes) = fs::read(f) else { return Ok(()) };
    if bytes.last().is_some_and(|&b| b != b'\n') {
        let largo = bytes.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
        OpenOptions::new().write(true).open(f)?.set_len(largo as u64)?;
    }
    Ok(())
}

fn anadir<T: Serialize>(f: &mut File, v: &T) -> io::Result<()> {
    let mut linea = serde_json::to_vec(v)?;
    linea.push(b'\n');
    f.write_all(&linea)?;
    f.sync_data()
}

impl Acceso {
    /// Sin disco: lo que se registra se pierde al parar.
    pub fn en_memoria(config: Config) -> Self {
        let estado = Estado { codigo_registro: config.codigo_registro.clone(), ..Estado::default() };
        Self { interior: Arc::new(Interior { config, dir: None, estado: Mutex::new(estado) }) }
    }

    /// Abre (o crea) el servicio en `dir`: lee las cuentas y las sesiones aún vigentes y reescribe `sesiones.jsonl` sin las caducadas.
    pub fn abrir(dir: &Path, config: Config) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let (fc, fs_) = (dir.join("cuentas.jsonl"), dir.join("sesiones.jsonl"));
        recortar_cola(&fc)?;
        recortar_cola(&fs_)?;
        let mut e = Estado { codigo_registro: config.codigo_registro.clone(), ..Estado::default() };
        for c in leer_lineas::<Cuenta>(&fc)? {
            e.insertar(c);
        }
        let ahora = ahora_ms();
        for s in leer_lineas::<SesionGuardada>(&fs_)? {
            if s.expira_ms > ahora && e.cuentas.contains_key(&s.cuenta) {
                e.sesiones.insert(s.token_sha256.clone(), s);
            }
        }
        let temporal = dir.join("sesiones.jsonl.nuevo");
        {
            let mut t = File::create(&temporal)?;
            for s in e.sesiones.values() {
                let mut linea = serde_json::to_vec(s)?;
                linea.push(b'\n');
                t.write_all(&linea)?;
            }
            t.sync_all()?;
        }
        fs::rename(&temporal, &fs_)?;
        let abrir = |f: &Path| OpenOptions::new().create(true).append(true).open(f);
        e.ficheros = Some(Ficheros { cuentas: abrir(&fc)?, sesiones: abrir(&fs_)? });
        Ok(Self { interior: Arc::new(Interior { config, dir: Some(dir.to_owned()), estado: Mutex::new(e) }) })
    }

    fn estado(&self) -> std::sync::MutexGuard<'_, Estado> {
        self.interior.estado.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn argon(&self) -> Argon2<'static> {
        let c = self.interior.config.coste;
        Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::new(c.memoria_kib, c.pasadas, 1, None).expect("coste de Argon2 válido"))
    }

    pub fn config(&self) -> &Config {
        &self.interior.config
    }

    pub fn cuenta(&self, id: &str) -> Option<Cuenta> {
        self.estado().cuentas.get(id).cloned()
    }

    pub fn cuentas(&self) -> usize {
        self.estado().cuentas.len()
    }

    /// Cuántas de las cuentas son de bot.
    pub fn bots(&self) -> usize {
        self.estado().cuentas.values().filter(|c| c.bot).count()
    }

    /// El código de invitación de las cuentas humanas vigente (`None`: registro abierto).
    pub fn codigo_registro(&self) -> Option<String> {
        self.estado().codigo_registro.clone()
    }

    /// Cambia el código de invitación en caliente (`None` o vacío: registro abierto). No se guarda: al reiniciar vuelve el de la configuración (`CODIGO_REGISTRO`).
    pub fn cambiar_codigo_registro(&self, codigo: Option<String>) {
        self.estado().codigo_registro = codigo.map(|c| c.trim().to_owned()).filter(|c| !c.is_empty());
    }

    /// La cuenta de un token vigente si es de un administrador global (un nick de `ADMINISTRADORES` que no es bot).
    pub fn administrador(&self, token: &str, ahora: i64) -> Option<Cuenta> {
        let (c, _) = self.sesion(token, ahora)?;
        (!c.bot && self.interior.config.administradores.iter().any(|a| normalizar(a) == c.nick)).then_some(c)
    }

    /// Copia los ficheros a `destino` (respaldo), con el servicio bloqueado mientras copia para que no quede una línea a medias. En memoria no copia nada.
    pub fn respaldar(&self, destino: &Path) -> io::Result<()> {
        let Some(dir) = &self.interior.dir else { return Ok(()) };
        let _e = self.estado();
        fs::create_dir_all(destino)?;
        for f in ["cuentas.jsonl", "sesiones.jsonl"] {
            if dir.join(f).exists() {
                fs::copy(dir.join(f), destino.join(f))?;
            }
        }
        Ok(())
    }

    /// Alta de cuenta (§2.1). El hash de la clave se calcula fuera del hilo del servidor.
    pub async fn registrar(&self, nick: &str, clave: &str, codigo: Option<&str>) -> Result<Cuenta, ErrorRegistro> {
        let nick = normalizar(nick);
        validar(&nick, clave)?;
        let cfg = &self.interior.config;
        let bot = cfg.codigo_registro_bots.as_deref().is_some_and(|c| Some(c) == codigo);
        if !bot && self.codigo_registro().as_deref().is_some_and(|c| Some(c) != codigo) {
            return Err(ErrorRegistro::CodigoIncorrecto);
        }
        if self.estado().por_nick.contains_key(&nick) {
            return Err(ErrorRegistro::NickRepetido);
        }
        let (argon, clave) = (self.argon(), clave.to_owned());
        let hash = tokio::task::spawn_blocking(move || {
            let sal = SaltString::generate(&mut OsRng);
            argon.hash_password(clave.as_bytes(), &sal).map(|h| h.to_string())
        })
        .await
        .map_err(|_| ErrorRegistro::Disco)?
        .map_err(|_| ErrorRegistro::Disco)?;
        let mut e = self.estado();
        if e.por_nick.contains_key(&nick) {
            return Err(ErrorRegistro::NickRepetido);
        }
        let c = Cuenta { id: format!("u-{}", e.siguiente + 1), nick, clave: hash, bot, creada_ms: ahora_ms() };
        if let Some(f) = e.ficheros.as_mut() {
            anadir(&mut f.cuentas, &c).map_err(|_| ErrorRegistro::Disco)?;
        }
        e.insertar(c.clone());
        Ok(c)
    }

    /// Login (§2.1): un token nuevo en cada llamada.
    pub async fn entrar(&self, nick: &str, clave: &str) -> Result<Sesion, ErrorLogin> {
        self.entrar_en(nick, clave, ahora_ms()).await
    }

    /// [`Acceso::entrar`] con el instante dado (para las pruebas de caducidad).
    pub async fn entrar_en(&self, nick: &str, clave: &str, ahora: i64) -> Result<Sesion, ErrorLogin> {
        let cuenta = {
            let e = self.estado();
            e.por_nick.get(&normalizar(nick)).and_then(|id| e.cuentas.get(id)).cloned()
        };
        let Some(cuenta) = cuenta else { return Err(ErrorLogin::Credenciales) };
        let (argon, clave, hash) = (self.argon(), clave.to_owned(), cuenta.clave.clone());
        let bien = tokio::task::spawn_blocking(move || {
            PasswordHash::new(&hash).is_ok_and(|h| argon.verify_password(clave.as_bytes(), &h).is_ok())
        })
        .await
        .unwrap_or(false);
        if !bien {
            return Err(ErrorLogin::Credenciales);
        }
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let s = SesionGuardada { token_sha256: sha256(&token), cuenta: cuenta.id.clone(), expira_ms: ahora + self.interior.config.duracion_sesion_ms };
        let mut e = self.estado();
        if let Some(f) = e.ficheros.as_mut() {
            anadir(&mut f.sesiones, &s).map_err(|_| ErrorLogin::Disco)?;
        }
        let expira_ms = s.expira_ms;
        e.sesiones.insert(s.token_sha256.clone(), s);
        Ok(Sesion { token, cuenta, expira_ms })
    }

    /// La cuenta y la caducidad de un token vigente en `ahora`.
    pub fn sesion(&self, token: &str, ahora: i64) -> Option<(Cuenta, i64)> {
        let e = self.estado();
        let s = e.sesiones.get(&sha256(token)).filter(|s| s.expira_ms > ahora)?;
        Some((e.cuentas.get(&s.cuenta)?.clone(), s.expira_ms))
    }

    /// Quién es el dueño de un token vigente en `ahora`: lo que el servidor necesita para abrir la sesión.
    pub fn identidad(&self, token: &str, ahora: i64) -> Result<Identidad, ErrorAcceso> {
        let (c, _) = self.sesion(token, ahora).ok_or(ErrorAcceso::Invalido)?;
        let admin = self.administrador(token, ahora).is_some();
        let (roles, rol) = if c.bot {
            (vec![TipoCliente::ClienteBot], RolTecnico::Jugador)
        } else if admin {
            (vec![TipoCliente::ClienteUnity, TipoCliente::ClienteAdminWeb], RolTecnico::AdministradorGlobal)
        } else {
            (vec![TipoCliente::ClienteUnity], RolTecnico::Jugador)
        };
        Ok(Identidad { cliente_id: c.id, roles, rol })
    }
}

impl Autenticador for Acceso {
    async fn autenticar(&self, token: &str) -> Result<Identidad, ErrorAcceso> {
        self.identidad(token, ahora_ms())
    }
}

// ───────────────────────────── HTTP ─────────────────────────────

fn error(estado: u16, codigo: &str, mensaje: &str) -> RespuestaHttp {
    RespuestaHttp::json(estado, json!({ "error": codigo, "mensaje": mensaje }).to_string())
}

fn texto<'a>(v: &'a Value, campo: &str) -> Option<&'a str> {
    v.get(campo).and_then(Value::as_str)
}

/// El token de la cabecera `Authorization: sesion <token>` (o `Bearer <token>`).
pub fn token_de(p: &PeticionHttp) -> Option<&str> {
    let v = p.cabecera("authorization")?;
    let (esquema, token) = v.split_once(' ')?;
    (esquema.eq_ignore_ascii_case("sesion") || esquema.eq_ignore_ascii_case("bearer")).then_some(token.trim())
}

impl Acceso {
    async fn atender_http(&self, p: PeticionHttp) -> RespuestaHttp {
        let cuerpo = || serde_json::from_slice::<Value>(&p.cuerpo).ok();
        match (p.metodo.as_str(), p.ruta.as_str()) {
            ("POST", "/v1/registro") => {
                let Some(v) = cuerpo() else { return error(400, "datos_invalidos", "el cuerpo es un JSON con nick y clave") };
                let (Some(nick), Some(clave)) = (texto(&v, "nick"), texto(&v, "clave")) else {
                    return error(400, "datos_invalidos", "faltan nick o clave");
                };
                match self.registrar(nick, clave, texto(&v, "codigo")).await {
                    Ok(c) => RespuestaHttp::json(201, json!({ "cuenta_id": c.id, "nick": c.nick, "bot": c.bot }).to_string()),
                    Err(ErrorRegistro::Invalido(m)) => error(400, "datos_invalidos", m),
                    Err(ErrorRegistro::CodigoIncorrecto) => error(403, "codigo_incorrecto", "el código de registro no coincide"),
                    Err(ErrorRegistro::NickRepetido) => error(409, "nick_repetido", "ese nick ya existe"),
                    Err(ErrorRegistro::Disco) => error(500, "interno", "no se pudo guardar la cuenta"),
                }
            }
            ("POST", "/v1/sesiones") => {
                let Some(v) = cuerpo() else { return error(400, "datos_invalidos", "el cuerpo es un JSON con nick y clave") };
                let (Some(nick), Some(clave)) = (texto(&v, "nick"), texto(&v, "clave")) else {
                    return error(400, "datos_invalidos", "faltan nick o clave");
                };
                match self.entrar(nick, clave).await {
                    Ok(s) => RespuestaHttp::json(
                        200,
                        json!({ "token": s.token, "cuenta_id": s.cuenta.id, "nick": s.cuenta.nick, "bot": s.cuenta.bot, "expira_ms": s.expira_ms }).to_string(),
                    ),
                    Err(ErrorLogin::Credenciales) => error(401, "credenciales_invalidas", "nick o clave incorrectos"),
                    Err(ErrorLogin::Disco) => error(500, "interno", "no se pudo guardar la sesión"),
                }
            }
            ("GET", "/v1/sesiones/actual") => match token_de(&p).and_then(|t| self.sesion(t, ahora_ms())) {
                Some((c, expira_ms)) => RespuestaHttp::json(200, json!({ "cuenta_id": c.id, "nick": c.nick, "bot": c.bot, "expira_ms": expira_ms }).to_string()),
                None => error(401, "token_invalido", "sin sesión o caducada"),
            },
            (_, "/v1/registro" | "/v1/sesiones" | "/v1/sesiones/actual") => error(405, "metodo_no_permitido", "método no permitido"),
            _ => error(404, "no_encontrado", "ruta desconocida"),
        }
    }
}

impl Http for Acceso {
    fn atender(&self, p: PeticionHttp) -> Pin<Box<dyn Future<Output = RespuestaHttp> + Send + '_>> {
        Box::pin(self.atender_http(p))
    }
}
