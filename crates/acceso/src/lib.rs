//! El servicio de acceso mínimo de la fase 7 (tanda 7B): alta de cuentas con código, cuenta de bot, login con token opaco y el [`Autenticador`] del servidor.
//!
//! Sigue a `Docs/Mechanics/Proyecciones_Y_Autorizacion.md` §2.1 con lo justo para que bots y admin entren por el contrato 05 (el contrato no cambia: el token va en `Hola.token`):
//! - **Alta** (`POST /v1/registro`, `{nick, clave, codigo?}`): el nick se recorta y pasa a minúsculas, y es único. Con `codigo_registro_bots` configurado y coincidente, la cuenta es **de bot**
//!   (sin ningún privilegio: solo abre sesiones `CLIENTE_BOT`, como un jugador); si no, con `codigo_registro` configurado hace falta que coincida; sin él el registro está abierto. No da sesión.
//! - **Login** (`POST /v1/sesiones`, `{nick, clave}`): cada login emite un token nuevo que dura `ACCESO.duracionSesionMs`. Se guarda solo su SHA-256.
//! - **Cerrar sesión** (`DELETE /v1/sesiones/actual` con `Authorization: sesion <token>`): anula ese token (`204`; `401 token_invalido` si no hay sesión). Los demás tokens de la cuenta siguen valiendo.
//! - **Argon2 acotado**: cada hash o verificación gasta ~19 MiB, así que solo corren `Config::argon_a_la_vez` a la vez; quien no consigue turno en `Config::argon_espera` recibe `503 ocupado`.
//! - **Quién es quién**: la cuenta de bot abre `CLIENTE_BOT`; la de servidor de batalla, solo `CLIENTE_SERVIDOR_BATALLA`; la humana, `CLIENTE_UNITY`; la humana con la marca `admin`,
//!   además `CLIENTE_ADMIN_WEB` con el rol de administrador global. `Identidad::cliente_id` es el id de la cuenta (`u-N`): el núcleo lo usa como cuenta del héroe (y como el servidor que
//!   tiene una batalla). Ni un bot ni un servidor de batalla son nunca administradores.
//! - **Servidores de batalla** (fase 8, 2026-11-11): con `codigo_registro_servidores` configurado y coincidente, el alta es de una cuenta **de servidor de batalla** (la del lanzador de
//!   Conquest, Batalla_Unity_Ciclo §8): entra con nick y clave como cualquiera y su token solo abre `CLIENTE_SERVIDOR_BATALLA`. Sin ese código no hay ninguna.
//! - **Administradores**: la marca vive en la cuenta. Si al abrir no hay ninguna cuenta admin, se genera un código de un solo uso, solo en memoria ([`Acceso::codigo_admin`]): quien
//!   se registra con él nace administrador y lo gasta. Después se da y se quita la marca con [`Acceso::fijar_admin`] (nunca se queda la partida sin administradores).
//! - **Disco**: `cuentas.jsonl` y `sesiones.jsonl` en la carpeta del servicio, una línea por alta, por cambio de la marca admin, por login o por cierre de sesión (la línea del token con `expira_ms: 0`), con `sync_data` antes
//!   de contestar. Al abrir se leen las dos (de una cuenta manda su última línea; de un token, también) y se reescribe `sesiones.jsonl` sin las caducadas ni las anuladas: los tokens sobreviven a un reinicio. Las claves, con Argon2id.
//!
//! Lo que falta y no hace falta aún: membresías por partida (moderador y administrador de la partida) y deshabilitar cuentas.

use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    future::Future,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
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
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

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
    /// Código que marca la cuenta como servidor de batalla (`CODIGO_REGISTRO_SERVIDORES`); `None`, no hay cuentas de servidor de batalla.
    pub codigo_registro_servidores: Option<String>,
    /// Nombres de máquina desde los que una página puede llamar al servidor, además de esta máquina y de la IP del servidor (`BRONZEAGE_ORIGENES`).
    pub origenes: Vec<String>,
    /// Las IPs desde las que se puede administrar además de esta máquina (`BRONZEAGE_IPS_ADMIN`); el proceso las pasa al servidor y a las rutas de administración.
    pub ips_admin: Vec<std::net::IpAddr>,
    pub duracion_sesion_ms: i64,
    pub coste: Coste,
    /// Cuántos cálculos de Argon2 (altas y logins) pueden correr a la vez; cada uno usa `coste.memoria_kib`. Por defecto, los núcleos disponibles (mínimo 1).
    pub argon_a_la_vez: usize,
    /// Lo que espera un alta o un login por un turno de Argon2 antes de rendirse con `Ocupado`.
    pub argon_espera: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            codigo_registro: None,
            codigo_registro_bots: None,
            codigo_registro_servidores: None,
            origenes: Vec::new(),
            ips_admin: Vec::new(),
            duracion_sesion_ms: duracion_sesion_ms(),
            coste: Coste::PRODUCCION,
            argon_a_la_vez: std::thread::available_parallelism().map_or(1, usize::from),
            argon_espera: Duration::from_secs(2),
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
    /// Todos los turnos de Argon2 siguen ocupados pasada `Config::argon_espera` (503).
    Ocupado,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorAdmin {
    /// No hay cuenta con ese nick (404).
    NoExiste,
    /// Un bot no puede ser administrador (400).
    EsBot,
    /// Un servidor de batalla no puede ser administrador (400).
    EsServidor,
    /// Quitar al último administrador dejaría la partida sin ninguno (409).
    UltimoAdministrador,
    /// No se pudo escribir en disco (500).
    Disco,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorLogin {
    /// Nick que no existe o clave que no coincide (401): no se dice cuál.
    Credenciales,
    Disco,
    /// Todos los turnos de Argon2 siguen ocupados pasada `Config::argon_espera` (503).
    Ocupado,
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
    /// Administrador global (nunca un bot ni un servidor de batalla). Cambiarla añade la línea actualizada de la cuenta a `cuentas.jsonl`.
    #[serde(default)]
    pub admin: bool,
    /// Servidor de batalla: solo abre `CLIENTE_SERVIDOR_BATALLA` (2026-11-11).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub servidor: bool,
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

/// La clase de una cuenta nueva, según el código del alta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Clase {
    Humana,
    Admin,
    Bot,
    Servidor,
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
    /// El código de un solo uso que hace administrador a quien se registra con él: solo existe mientras no haya ninguna cuenta admin, y solo en memoria.
    codigo_admin: Option<String>,
}

impl Estado {
    fn insertar(&mut self, c: Cuenta) {
        if let Some(n) = c.id.strip_prefix("u-").and_then(|n| n.parse::<u64>().ok()) {
            self.siguiente = self.siguiente.max(n);
        }
        self.por_nick.insert(c.nick.clone(), c.id.clone());
        self.cuentas.insert(c.id.clone(), c);
    }

    fn administradores(&self) -> impl Iterator<Item = &Cuenta> {
        self.cuentas.values().filter(|c| c.admin && !c.bot && !c.servidor)
    }

    /// Sin ningún administrador hay código de un solo uso; con alguno, no.
    fn preparar_codigo_admin(&mut self) {
        let sin_admin = self.administradores().next().is_none();
        self.codigo_admin = sin_admin.then(generar_codigo_admin);
    }
}

struct Interior {
    config: Config,
    /// La carpeta de los ficheros (`None`: en memoria).
    dir: Option<PathBuf>,
    estado: Mutex<Estado>,
    /// Los turnos de Argon2: tantos como `Config::argon_a_la_vez`.
    argon: Arc<Semaphore>,
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

/// Un código de administrador: 4 grupos de 4 caracteres de un alfabeto de 32 sin ambiguos (sin `I`, `O`, `0` ni `1`), 80 bits, separados por `-`.
fn generar_codigo_admin() -> String {
    const ALFABETO: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    let grupos: Vec<String> = bytes.chunks(4).map(|g| g.iter().map(|b| ALFABETO[(b & 31) as usize] as char).collect()).collect();
    grupos.join("-")
}

/// Igualdad de textos sin atajos de tiempo: recorre todos los bytes acumulando las diferencias (la longitud no es secreta).
fn iguales(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0, |dif, (x, y)| dif | (x ^ y)) == 0
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
        let mut estado = Estado { codigo_registro: config.codigo_registro.clone(), ..Estado::default() };
        estado.preparar_codigo_admin();
        let argon = Arc::new(Semaphore::new(config.argon_a_la_vez.max(1)));
        Self { interior: Arc::new(Interior { config, dir: None, estado: Mutex::new(estado), argon }) }
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
        e.preparar_codigo_admin();
        let ahora = ahora_ms();
        for s in leer_lineas::<SesionGuardada>(&fs_)? {
            // La última línea de un token manda: una caducada (o la de un cierre de sesión, con `expira_ms: 0`) quita la anterior.
            if s.expira_ms > ahora && e.cuentas.contains_key(&s.cuenta) {
                e.sesiones.insert(s.token_sha256.clone(), s);
            } else {
                e.sesiones.remove(&s.token_sha256);
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
        let argon = Arc::new(Semaphore::new(config.argon_a_la_vez.max(1)));
        Ok(Self { interior: Arc::new(Interior { config, dir: Some(dir.to_owned()), estado: Mutex::new(e), argon }) })
    }

    fn estado(&self) -> std::sync::MutexGuard<'_, Estado> {
        self.interior.estado.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn argon(&self) -> Argon2<'static> {
        let c = self.interior.config.coste;
        Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::new(c.memoria_kib, c.pasadas, 1, None).expect("coste de Argon2 válido"))
    }

    /// Un turno de Argon2, con la espera máxima de `Config::argon_espera`. Se suelta al soltar el permiso: va dentro del cálculo, que sigue gastando memoria aunque la petición se cancele.
    async fn turno_argon(&self) -> Option<OwnedSemaphorePermit> {
        tokio::time::timeout(self.interior.config.argon_espera, self.interior.argon.clone().acquire_owned()).await.ok()?.ok()
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

    /// Cuántas de las cuentas son de servidor de batalla.
    pub fn servidores(&self) -> usize {
        self.estado().cuentas.values().filter(|c| c.servidor).count()
    }

    /// El código de invitación de las cuentas humanas vigente (`None`: registro abierto).
    pub fn codigo_registro(&self) -> Option<String> {
        self.estado().codigo_registro.clone()
    }

    /// Cambia el código de invitación en caliente (`None` o vacío: registro abierto). No se guarda: al reiniciar vuelve el de la configuración (`CODIGO_REGISTRO`).
    pub fn cambiar_codigo_registro(&self, codigo: Option<String>) {
        self.estado().codigo_registro = codigo.map(|c| c.trim().to_owned()).filter(|c| !c.is_empty());
    }

    /// La cuenta de un token vigente si es de un administrador global (con la marca `admin` y no bot).
    pub fn administrador(&self, token: &str, ahora: i64) -> Option<Cuenta> {
        let (c, _) = self.sesion(token, ahora)?;
        (c.admin && !c.bot && !c.servidor).then_some(c)
    }

    /// Los nicks de los administradores globales, ordenados.
    pub fn administradores(&self) -> Vec<String> {
        let mut nicks: Vec<String> = self.estado().administradores().map(|c| c.nick.clone()).collect();
        nicks.sort();
        nicks
    }

    /// El código de administrador de un solo uso, si sigue vivo (solo mientras no haya ningún administrador). Registrarse con él da la marca y lo gasta.
    pub fn codigo_admin(&self) -> Option<String> {
        self.estado().codigo_admin.clone()
    }

    /// Da o quita la marca de administrador a la cuenta `nick`. Sin cambios (y sin escribir) si ya está así. Con administrador, el código de un solo uso deja de valer.
    pub fn fijar_admin(&self, nick: &str, admin: bool) -> Result<Cuenta, ErrorAdmin> {
        let mut e = self.estado();
        let mut c = e.por_nick.get(&normalizar(nick)).and_then(|id| e.cuentas.get(id)).cloned().ok_or(ErrorAdmin::NoExiste)?;
        if c.bot {
            return Err(ErrorAdmin::EsBot);
        }
        if c.servidor {
            return Err(ErrorAdmin::EsServidor);
        }
        if c.admin == admin {
            return Ok(c);
        }
        if !admin && e.administradores().count() == 1 {
            return Err(ErrorAdmin::UltimoAdministrador);
        }
        c.admin = admin;
        if let Some(f) = e.ficheros.as_mut() {
            anadir(&mut f.cuentas, &c).map_err(|_| ErrorAdmin::Disco)?;
        }
        e.insertar(c.clone());
        if admin {
            e.codigo_admin = None;
        }
        Ok(c)
    }

    /// Qué cuenta crea un alta con `codigo`. El código de bots manda; luego el de servidores de batalla; luego el de administrador (si sigue vivo); si no, hace falta el de
    /// invitación, si lo hay.
    fn clase_del_alta(&self, e: &Estado, codigo: Option<&str>) -> Result<Clase, ErrorRegistro> {
        let coincide = |vigente: Option<&str>| matches!((vigente, codigo), (Some(v), Some(c)) if iguales(v, c));
        if coincide(self.interior.config.codigo_registro_bots.as_deref()) {
            return Ok(Clase::Bot);
        }
        if coincide(self.interior.config.codigo_registro_servidores.as_deref()) {
            return Ok(Clase::Servidor);
        }
        if coincide(e.codigo_admin.as_deref()) {
            return Ok(Clase::Admin);
        }
        if e.codigo_registro.is_some() && !coincide(e.codigo_registro.as_deref()) {
            return Err(ErrorRegistro::CodigoIncorrecto);
        }
        Ok(Clase::Humana)
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
        {
            let e = self.estado();
            self.clase_del_alta(&e, codigo)?;
            if e.por_nick.contains_key(&nick) {
                return Err(ErrorRegistro::NickRepetido);
            }
        }
        let turno = self.turno_argon().await.ok_or(ErrorRegistro::Ocupado)?;
        let (argon, clave) = (self.argon(), clave.to_owned());
        let hash = tokio::task::spawn_blocking(move || {
            let _turno = turno;
            let sal = SaltString::generate(&mut OsRng);
            argon.hash_password(clave.as_bytes(), &sal).map(|h| h.to_string())
        })
        .await
        .map_err(|_| ErrorRegistro::Disco)?
        .map_err(|_| ErrorRegistro::Disco)?;
        let mut e = self.estado();
        // Con el bloqueo tomado otra vez: si el código de administrador lo gastó otro alta mientras se hacía el hash, ya no vale.
        let clase = self.clase_del_alta(&e, codigo)?;
        if e.por_nick.contains_key(&nick) {
            return Err(ErrorRegistro::NickRepetido);
        }
        let (bot, admin, servidor) = (clase == Clase::Bot, clase == Clase::Admin, clase == Clase::Servidor);
        let c = Cuenta { id: format!("u-{}", e.siguiente + 1), nick, clave: hash, bot, admin, servidor, creada_ms: ahora_ms() };
        if let Some(f) = e.ficheros.as_mut() {
            anadir(&mut f.cuentas, &c).map_err(|_| ErrorRegistro::Disco)?;
        }
        e.insertar(c.clone());
        if admin {
            e.codigo_admin = None;
        }
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
        let turno = self.turno_argon().await.ok_or(ErrorLogin::Ocupado)?;
        let (argon, clave, hash) = (self.argon(), clave.to_owned(), cuenta.clave.clone());
        let bien = tokio::task::spawn_blocking(move || {
            let _turno = turno;
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

    /// Cierra la sesión de `token`: `true` si estaba vigente y queda anulado, `false` si no había sesión. Se guarda la línea del token con `expira_ms: 0`, que al abrir quita la anterior.
    /// Las sesiones WebSocket ya abiertas con ese token siguen hasta que se cierren: solo se anula para abrir otras.
    pub fn cerrar_sesion(&self, token: &str) -> Result<bool, ErrorLogin> {
        let hash = sha256(token);
        let mut e = self.estado();
        let Some(cuenta) = e.sesiones.get(&hash).filter(|s| s.expira_ms > ahora_ms()).map(|s| s.cuenta.clone()) else { return Ok(false) };
        if let Some(f) = e.ficheros.as_mut() {
            anadir(&mut f.sesiones, &SesionGuardada { token_sha256: hash.clone(), cuenta, expira_ms: 0 }).map_err(|_| ErrorLogin::Disco)?;
        }
        e.sesiones.remove(&hash);
        Ok(true)
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
        let (roles, rol) = if c.bot {
            (vec![TipoCliente::ClienteBot], RolTecnico::Jugador)
        } else if c.servidor {
            (vec![TipoCliente::ClienteServidorBatalla], RolTecnico::Jugador)
        } else if c.admin {
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

fn ocupado() -> RespuestaHttp {
    error(503, "ocupado", "el servidor está ocupado; reinténtalo en unos segundos")
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
                    Ok(c) => RespuestaHttp::json(201, json!({ "cuenta_id": c.id, "nick": c.nick, "bot": c.bot, "admin": c.admin, "servidor": c.servidor }).to_string()),
                    Err(ErrorRegistro::Invalido(m)) => error(400, "datos_invalidos", m),
                    Err(ErrorRegistro::CodigoIncorrecto) => error(403, "codigo_incorrecto", "el código de registro no coincide"),
                    Err(ErrorRegistro::NickRepetido) => error(409, "nick_repetido", "ese nick ya existe"),
                    Err(ErrorRegistro::Disco) => error(500, "interno", "no se pudo guardar la cuenta"),
                    Err(ErrorRegistro::Ocupado) => ocupado(),
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
                        json!({ "token": s.token, "cuenta_id": s.cuenta.id, "nick": s.cuenta.nick, "bot": s.cuenta.bot, "admin": s.cuenta.admin, "servidor": s.cuenta.servidor, "expira_ms": s.expira_ms }).to_string(),
                    ),
                    Err(ErrorLogin::Credenciales) => error(401, "credenciales_invalidas", "nick o clave incorrectos"),
                    Err(ErrorLogin::Disco) => error(500, "interno", "no se pudo guardar la sesión"),
                    Err(ErrorLogin::Ocupado) => ocupado(),
                }
            }
            ("GET", "/v1/sesiones/actual") => match token_de(&p).and_then(|t| self.sesion(t, ahora_ms())) {
                Some((c, expira_ms)) => RespuestaHttp::json(200, json!({ "cuenta_id": c.id, "nick": c.nick, "bot": c.bot, "admin": c.admin, "servidor": c.servidor, "expira_ms": expira_ms }).to_string()),
                None => error(401, "token_invalido", "sin sesión o caducada"),
            },
            ("DELETE", "/v1/sesiones/actual") => match token_de(&p).map(|t| self.cerrar_sesion(t)) {
                Some(Ok(true)) => RespuestaHttp::texto(204, ""),
                Some(Ok(false)) | None => error(401, "token_invalido", "sin sesión o caducada"),
                Some(Err(_)) => error(500, "interno", "no se pudo guardar el cierre de sesión"),
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
