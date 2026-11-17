//! El runner de bots (`Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md` §3 y §13 del motor): un proceso aparte que maneja a todos los bots de una partida por su [`Puerto`], como el
//! `ProcesoDeBots` de TypeScript.
//!
//! - **Altas**: el plan de llegadas (D56, D57) cuenta desde que el proceso empezó en la partida; cada bot es una cuenta de bot nueva que nace en un campamento, el de menos residentes,
//!   y los que llegan juntos van al mismo.
//! - **Registro** (JSON): las cuentas de sus bots, su héroe y su perfil, y cuántas llegadas lleva, para volver a entrar con los mismos héroes tras un reinicio. Es lo único que guarda.
//! - **Horario** (D55): fuera de sus horas el bot da `desconectarse` y cierra la conexión, como quien cierra el cliente; al volver, `conectarse`.
//! - **Ritmo**: piensa cada [`Config::cada_ms`] de mundo con un desfase propio, y antes si le llega un hecho que lo nombra.
//! - **Memoria desechable** y **pizarra de Facción**: las define el [`Cerebro`]; viven en el proceso y se pierden al reiniciar (el bot rehace su plan mirando el mundo).
//!
//! Cada bot es una tarea con su conexión: un fallo de uno se avisa y no para a los demás. El reloj lo lleva una cuenta sin héroe, el «reloj».

use std::{
    collections::HashMap,
    future::Future,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use contrato::v1::{orden::Cuerpo as O, EventoDominio, OrdenConectarse, OrdenCrearHeroe, OrdenDesconectarse, Reloj, Resultado, ResultadoOrden, TipoEntidad};
use serde::{Deserialize, Serialize};
use tokio::{sync::watch, task::JoinSet};

use crate::{
    acceso,
    azar::{hash, Azar},
    llegadas::{self, Llegada, Perfil, TipoPerfil},
    puerto::{self, Puerto},
    Credenciales, Fallo,
};

/// Lo que decide un bot. Su memoria y la pizarra de su Facción son suyas: el runner solo las guarda.
pub trait Cerebro: Send + Sync + 'static {
    type Memoria: Default + Send + 'static;
    type Pizarra: Default + Send + 'static;
    fn pensar(&self, ctx: &mut Contexto<'_, Self>) -> impl Future<Output = Result<(), Fallo>> + Send;
    /// Su memoria para el panel (`volcarMemoria`), en el formato del servicio TypeScript. Se toma al acabar de pensar.
    fn volcar(_memoria: &Self::Memoria) -> serde_json::Value {
        serde_json::json!({})
    }
    /// Lo que la pizarra de una Facción guarda en el registro, para sobrevivir a los reinicios.
    fn guardar(_pizarra: &Self::Pizarra) -> serde_json::Value {
        serde_json::Value::Null
    }
    /// La pizarra que vuelve del registro.
    fn cargar(_guardado: &serde_json::Value) -> Self::Pizarra {
        Default::default()
    }
    /// Lo que la Facción publica para todos: su estado respecto a las demás.
    fn publico(_pizarra: &Self::Pizarra, _ahora_ms: i64) -> serde_json::Value {
        serde_json::Value::Null
    }
    /// Cuándo llegan: cada población tiene su plan.
    fn plan(semilla: u64, total: u32, dias: f64) -> Vec<Llegada> {
        llegadas::plan_de_llegadas(semilla, total, dias)
    }
    /// Lo que hace un bot al retirarlo desde el panel.
    fn despedida() -> Option<O> {
        None
    }
    /// Un hecho que oye un bot de la Facción, en cuanto llega: lo apunta en la pizarra y dice a qué compañeros despertar (se adelanta su turno de pensar).
    fn oir(_hecho: &EventoDominio, _pizarra: &mut Self::Pizarra) -> Vec<String> {
        Vec::new()
    }
    /// Lo que el panel ve de una pizarra en cada estado (cada 2 s): ligero (`PizarraInfo` del contrato de control).
    fn resumen(_pizarra: &Self::Pizarra, _ahora_ms: i64) -> serde_json::Value {
        serde_json::Value::Null
    }
    /// La pizarra entera, para el panel a petición (`volcarPizarra`).
    fn detalle(_pizarra: &Self::Pizarra, _ahora_ms: i64) -> serde_json::Value {
        serde_json::Value::Null
    }
}

/// Los eventos de las Facciones de bots, en vivo, para el panel: `{faccionId, heroeId, tipo, texto, datos, ms, en}` (`ms`, de mundo; `en`, de pared).
pub fn eventos() -> &'static tokio::sync::broadcast::Sender<serde_json::Value> {
    static EVENTOS: std::sync::OnceLock<tokio::sync::broadcast::Sender<serde_json::Value>> = std::sync::OnceLock::new();
    EVENTOS.get_or_init(|| tokio::sync::broadcast::channel(256).0)
}

/// Cómo ve el panel las pizarras: sin clave, el resumen de todas; con una, esa entera. Lo pone el proceso que las tiene.
pub type VerPizarras = Arc<dyn Fn(Option<&str>, i64) -> serde_json::Value + Send + Sync>;

/// Lo que recibe un cerebro al pensar.
pub struct Contexto<'a, C: Cerebro + ?Sized> {
    /// Su héroe.
    pub yo: &'a str,
    pub perfil: &'a Perfil,
    pub puerto: &'a mut Puerto,
    pub memoria: &'a mut C::Memoria,
    pub azar: &'a mut Azar,
    /// Los hechos que llegaron desde la última vez que pensó.
    pub hechos: Vec<EventoDominio>,
    /// El instante de mundo en que piensa.
    pub ahora_ms: i64,
    esperas: &'a mut HashMap<String, i64>,
    pizarras: &'a Mutex<HashMap<String, C::Pizarra>>,
    refuerzos: &'a Mutex<Vec<String>>,
    avisos: &'a Mutex<std::collections::HashSet<String>>,
}

/// Lo que espera un bot tras un rechazo, por defecto: media hora de mundo.
pub const ESPERA_TRAS_RECHAZO_MS: i64 = 30 * 60_000;

impl<C: Cerebro + ?Sized> Contexto<'_, C> {
    /// Su Facción: la del ámbito `FACCION` que tiene abierto.
    pub fn faccion(&self) -> Option<String> {
        self.puerto.observar().ambitos.keys().find_map(|k| k.strip_prefix("FACCION:").map(String::from))
    }

    /// La pizarra de su Facción (o la suya, si aún no tiene): lo que se cuentan sus bots, como amigos por el chat de voz.
    pub fn con_pizarra<R>(&self, f: impl FnOnce(&mut C::Pizarra) -> R) -> R {
        let clave = self.faccion().unwrap_or_else(|| format!("sin-faccion:{}", self.yo));
        self.con_pizarra_en(&clave, f)
    }

    /// Una pizarra por su nombre, para lo que comparten bots que aún no son de la misma Facción (unos amigos que acaban de llegar).
    pub fn con_pizarra_en<R>(&self, clave: &str, f: impl FnOnce(&mut C::Pizarra) -> R) -> R {
        f(self.pizarras.lock().unwrap().entry(clave.into()).or_default())
    }

    /// Una plaza nueva de `faccion`: el runner da de alta [`REFUERZOS`] cuentas que piden entrar en ella (§16).
    pub fn pedir_refuerzos(&self, faccion: &str) {
        self.refuerzos.lock().unwrap().push(faccion.into());
    }

    /// Manda al panel un evento de su Facción, en vivo ([`eventos`]).
    pub fn evento(&self, tipo: &str, texto: String, datos: serde_json::Value) {
        let _ = eventos().send(serde_json::json!({"faccionId": self.faccion(), "heroeId": self.yo, "tipo": tipo, "texto": texto, "datos": datos, "ms": self.ahora_ms, "en": ms_epoca()}));
    }

    /// Despierta a unos compañeros: piensan en cuanto pueden, sin esperar su turno (un evento de la Facción: una reunión en la puerta, una plaza atacada).
    pub fn avisar(&self, heroes: impl IntoIterator<Item = String>) {
        self.avisos.lock().unwrap().extend(heroes.into_iter().filter(|h| h != self.yo));
    }

    /// ¿Sigue `clave` en su espera?
    pub fn en_espera(&self, clave: &str) -> bool {
        self.esperas.get(clave).is_some_and(|&hasta| hasta > self.ahora_ms)
    }

    /// No intenta `clave` hasta pasados `espera_ms` de mundo, aunque lo último saliera bien (una orden de mercado que ya está puesta).
    pub fn esperar(&mut self, clave: &str, espera_ms: i64) {
        self.esperas.insert(clave.into(), self.ahora_ms + espera_ms);
    }

    /// Como [`Puerto::actuar`], pero tras un rechazo no se reintenta la misma `clave` hasta pasados `espera_ms` de mundo (`None`: se dejó sin intentar). El rechazo es información.
    /// ¿Espera aún la orden `clave` tras un rechazo?
    pub fn esperando(&self, clave: &str) -> bool {
        self.esperas.get(clave).is_some_and(|&hasta| hasta > self.ahora_ms)
    }

    pub async fn intentar(&mut self, clave: &str, cuerpo: O, espera_ms: i64) -> Result<Option<ResultadoOrden>, Fallo> {
        if self.esperas.get(clave).is_some_and(|&hasta| hasta > self.ahora_ms) {
            return Ok(None);
        }
        let r = self.puerto.actuar(cuerpo).await?;
        if r.resultado == Resultado::OrdenRechazada as i32 {
            self.esperas.insert(clave.into(), self.ahora_ms + espera_ms);
        } else {
            self.esperas.remove(clave);
        }
        Ok(Some(r))
    }
}

#[derive(Clone)]
pub struct Config {
    /// `127.0.0.1:7878`.
    pub servidor: String,
    pub codigo_bots: String,
    /// Dónde guarda su registro.
    pub registro: PathBuf,
    pub total: u32,
    pub dias_llegada: f64,
    pub semilla: u64,
    /// Cada cuánto piensa un bot, en ms de mundo. ponytail: 5 min como en TypeScript, a calibrar con batch.
    pub cada_ms: i64,
    /// Sin horario: conectados todo el día (pruebas).
    pub siempre: bool,
    pub lugares: Lugares,
}

/// Dónde nacen los bots de una población. Para condensar una prueba en pocos campamentos, aunque el resto del mapa quede vacío.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Lugares {
    /// Los campamentos permitidos, en orden. Vacío: todos.
    #[serde(default)]
    pub campamentos: Vec<String>,
    /// Cuántos residentes llenan uno antes de pasar al siguiente. Con 0, se reparten entre los de la lista.
    #[serde(default)]
    pub por_campamento: u32,
}

impl Lugares {
    /// `BOTS_CAMPAMENTOS` (`camp-3,camp-7`) y `BOTS_POR_CAMPAMENTO`.
    pub fn del_entorno() -> Self {
        let campamentos = std::env::var("BOTS_CAMPAMENTOS").unwrap_or_default().split(',').map(str::trim).filter(|c| !c.is_empty()).map(String::from).collect();
        Self { campamentos, por_campamento: std::env::var("BOTS_POR_CAMPAMENTO").ok().and_then(|n| n.parse().ok()).unwrap_or(0) }
    }

    /// El campamento donde nace el siguiente: el primero de la lista que no esté lleno y, llenos todos, el que menos tenga: nadie se queda sin nacer.
    fn elegir(&self, c: &[contrato::v1::CampamentoParaElegir]) -> Option<String> {
        let mut posibles: Vec<_> = c.iter().filter(|k| self.campamentos.is_empty() || self.campamentos.contains(&k.campamento_id)).collect();
        if self.por_campamento > 0 {
            posibles.sort_by_key(|k| self.campamentos.iter().position(|x| *x == k.campamento_id));
            if let Some(k) = posibles.iter().find(|k| k.residentes < self.por_campamento) {
                return Some(k.campamento_id.clone());
            }
        }
        posibles.iter().min_by_key(|k| (k.residentes, k.campamento_id.clone())).map(|k| k.campamento_id.clone())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BotRegistrado {
    pub cuenta: Credenciales,
    pub heroe_id: String,
    pub perfil: Perfil,
    /// Retirado desde el panel: ya no juega ni vuelve tras un reinicio.
    #[serde(default)]
    pub retirado: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Registro {
    /// El mundo de la partida: con otro, los héroes del registro no existen.
    pub mundo_hash: String,
    /// Hace únicos los nicks de este registro.
    pub serie: String,
    /// El instante de mundo en que el proceso empezó en la partida: las llegadas cuentan desde ahí.
    pub inicio_ms: i64,
    pub llegadas_hechas: usize,
    pub reloj: Credenciales,
    pub bots: Vec<BotRegistrado>,
    /// El plan de llegadas de esta partida: lo fija el primer arranque y los reinicios lo respetan (un registro de antes no lo trae: se queda con el de su siguiente arranque).
    #[serde(default)]
    pub plan: Option<PlanLlegadas>,
    /// Lo que cada Facción de bots guarda de su pizarra: su libro de agravios.
    #[serde(default)]
    pub libros: std::collections::BTreeMap<String, serde_json::Value>,
    /// Altas que fallaron a medias: su nick ya está cogido y no se repite.
    #[serde(default)]
    pub fallidas: usize,
}

/// Lo que define las llegadas de una partida: `llegadas_hechas` es un índice de este plan, y `semilla` da además el horario de cada bot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanLlegadas {
    #[serde(default)]
    pub lugares: Lugares,
    pub total: u32,
    pub dias_llegada: f64,
    pub semilla: u64,
}

impl Registro {
    pub fn leer(ruta: &std::path::Path) -> Result<Option<Self>, Fallo> {
        match std::fs::read_to_string(ruta) {
            Ok(t) => serde_json::from_str(&t).map(Some).map_err(|e| Fallo::Registro(format!("{}: {e}", ruta.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Fallo::Registro(format!("{}: {e}", ruta.display()))),
        }
    }

    /// Escribe al lado y renombra: un corte a mitad no deja un registro roto.
    fn guardar(&self, ruta: &std::path::Path) -> Result<(), Fallo> {
        let tmp = ruta.with_extension("tmp");
        let fallo = |e: std::io::Error| Fallo::Registro(format!("{}: {e}", ruta.display()));
        std::fs::write(&tmp, serde_json::to_string_pretty(self).unwrap()).map_err(fallo)?;
        std::fs::rename(&tmp, ruta).map_err(fallo)
    }
}

/// Cómo se maneja a un bot desde el canal de control (12_NPC §14; `control/contrato.ts` del TypeScript): `auto` sigue su horario; `conectado`/`desconectado` lo fuerzan;
/// `congelado` sigue su horario de conexión pero no piensa.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modo {
    #[default]
    Auto,
    Conectado,
    Desconectado,
    Congelado,
}

/// Lo que el panel ve de un bot (`BotInfo` del contrato de control).
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoBot {
    pub heroe_id: String,
    pub nombre: String,
    pub perfil: Option<Perfil>,
    /// `sin-plaza`, `en-casa`, `en-columna` o `sin-datos`.
    pub fase: String,
    pub conectado: bool,
    pub modo: Modo,
    pub retirado: bool,
    pub faccion_id: Option<String>,
    pub residencia_id: Option<String>,
    pub sesiones: Vec<(i64, i64)>,
    pub pensamientos: u64,
    pub vista_en: Option<i64>,
    pub errores: u64,
    pub ultimo_error: Option<String>,
    /// Su plan en curso, en corto («cazar band-3»).
    pub plan: Option<String>,
    /// La columna en la que va.
    pub columna_id: Option<String>,
}

/// Lo que el canal de control toca mientras el proceso corre, y lo que lee de él.
#[derive(Default)]
pub struct Control {
    /// Pausado: los bots siguen conectados pero no piensan ni llegan más.
    pub pausado: std::sync::atomic::AtomicBool,
    pub modos: Mutex<HashMap<String, Modo>>,
    /// Los que hay que retirar (se desconectan y no vuelven; queda en el registro).
    pub retirar: Mutex<Vec<String>>,
    /// Los que piensan ya, sin esperar su turno.
    pub pensar_ya: Mutex<std::collections::HashSet<String>>,
    /// Los que despierta un evento de su Facción ([`Contexto::avisar`], [`Cerebro::oir`]): piensan antes de su turno, salvo en pausa.
    pub avisos: Mutex<std::collections::HashSet<String>>,
    /// Las pizarras para el panel, mientras corre un proceso.
    pub pizarras: Mutex<Option<VerPizarras>>,
    /// Llegadas forzadas desde el panel, fuera del plan.
    pub forzadas: Mutex<Vec<TipoPerfil>>,
    pub bots: Mutex<std::collections::BTreeMap<String, InfoBot>>,
    /// Llegadas del plan hechas, y extras.
    pub llegadas: Mutex<(usize, usize)>,
    /// El instante de mundo en la última vuelta del bucle.
    pub mundo_ms: std::sync::atomic::AtomicI64,
    /// La memoria de cada bot la última vez que pensó, con sus esperas (`volcarMemoria`).
    pub memorias: Mutex<HashMap<String, serde_json::Value>>,
    /// El estado de cada Facción de bots respecto a las demás: es lo que se publica en `GET /relaciones`.
    pub relaciones: Mutex<serde_json::Value>,
}

type Pizarras<C> = Arc<Mutex<HashMap<String, <C as Cerebro>::Pizarra>>>;
type RelojCompartido = Arc<Mutex<(Reloj, Instant)>>;
type Refuerzos = Arc<Mutex<Vec<String>>>;

/// Los héroes nuevos que trae cada fundación de un bot (§16; pregunta 5 de la fase 7: altas del runner, que llegan por el camino de cualquier jugador).
pub const REFUERZOS: u32 = 5;

/// El proceso en marcha.
pub struct Proceso {
    parar: watch::Sender<bool>,
    tarea: tokio::task::JoinHandle<Result<(), Fallo>>,
    /// El plan de llegadas que sigue: el del registro, si ya había uno.
    pub plan: PlanLlegadas,
}

impl Proceso {
    /// Recupera el registro (o lo crea, con su cuenta del reloj), vuelve a poner a jugar a sus bots y sigue con las llegadas de su plan: reconectar no configura llegadas nuevas.
    pub async fn arrancar<C: Cerebro>(cfg: Config, cerebro: C) -> Result<Self, Fallo> {
        Self::arrancar_con(cfg, cerebro, Default::default()).await
    }

    /// Como [`Proceso::arrancar`], con el control que maneja el canal de control.
    pub async fn arrancar_con<C: Cerebro>(mut cfg: Config, cerebro: C, control: Arc<Control>) -> Result<Self, Fallo> {
        let guardado = Registro::leer(&cfg.registro)?;
        let nuevo = guardado.is_none();
        let serie = guardado.as_ref().map_or_else(|| format!("{:x}", ms_epoca() % 0xff_ffff), |r| r.serie.clone());
        let cuenta_reloj = match &guardado {
            Some(r) => r.reloj.clone(),
            None => {
                let c = Credenciales { nick: format!("b{serie}-reloj"), clave: clave_nueva(&serie) };
                acceso::registrar(&cfg.servidor, &c, &cfg.codigo_bots).await?;
                c
            }
        };
        // Las cuentas de un registro solo existen en el servidor de su partida: con otra, el servidor no las conoce.
        let reloj = Puerto::entrar(&cfg.servidor, cuenta_reloj.clone()).await.map_err(|f| match f {
            Fallo::Acceso { estado: 401, .. } if !nuevo => Fallo::Registro(format!("las cuentas de {} no existen en este servidor: es de otra partida; usa iniciar con la partida que está abierta", cfg.registro.display())),
            f => f,
        })?;
        let hash_mundo = reloj.bienvenida.mundo.as_ref().map(|m| m.hash.clone()).unwrap_or_default();
        let mut registro = match guardado {
            Some(r) if r.mundo_hash != hash_mundo => {
                return Err(Fallo::Registro(format!("{} es del mundo {} y el servidor tiene {hash_mundo}: si la partida se regeneró, bórralo", cfg.registro.display(), r.mundo_hash)))
            }
            Some(r) => r,
            None => Registro { mundo_hash: hash_mundo, serie, inicio_ms: reloj.mundo_ms(), llegadas_hechas: 0, reloj: cuenta_reloj, bots: Vec::new(), plan: None, libros: Default::default(), fallidas: 0 },
        };
        let plan = registro.plan.clone().unwrap_or(PlanLlegadas { lugares: cfg.lugares.clone(), total: cfg.total, dias_llegada: cfg.dias_llegada, semilla: cfg.semilla });
        (cfg.total, cfg.dias_llegada, cfg.semilla, cfg.lugares) = (plan.total, plan.dias_llegada, plan.semilla, plan.lugares.clone());
        if nuevo || registro.plan.is_none() {
            registro.plan = Some(plan.clone());
            registro.guardar(&cfg.registro)?;
        }
        let (parar, rx) = watch::channel(false);
        let tarea = tokio::spawn(bucle(Arc::new(cfg), registro, reloj, Arc::new(cerebro), control, rx));
        Ok(Self { parar, tarea, plan })
    }

    /// Para el bucle y a los bots (cada uno da `desconectarse` y cierra) y espera a que acaben.
    pub async fn parar(self) -> Result<(), Fallo> {
        let _ = self.parar.send(true);
        self.tarea.await.unwrap_or_else(|e| Err(Fallo::Registro(format!("el runner cayó: {e}"))))
    }
}

async fn bucle<C: Cerebro>(cfg: Arc<Config>, mut registro: Registro, mut reloj: Puerto, cerebro: Arc<C>, control: Arc<Control>, mut parar: watch::Receiver<bool>) -> Result<(), Fallo> {
    let pizarras: Pizarras<C> = Arc::new(Mutex::new(registro.libros.iter().map(|(f, g)| (f.clone(), C::cargar(g))).collect()));
    let vistas = pizarras.clone();
    *control.pizarras.lock().unwrap() = Some(Arc::new(move |clave: Option<&str>, t: i64| {
        let pz = vistas.lock().unwrap();
        match clave {
            None => serde_json::Value::Array(
                pz.iter()
                    .map(|(k, x)| {
                        let mut v = C::resumen(x, t);
                        if v.is_object() {
                            v["id"] = serde_json::json!(k);
                        }
                        v
                    })
                    .filter(|v| v.is_object())
                    .collect(),
            ),
            Some(k) => pz.get(k).map_or(serde_json::Value::Null, |x| C::detalle(x, t)),
        }
    }));
    let mut guardado_en = Instant::now();
    let compartido: RelojCompartido = Arc::new(Mutex::new(reloj.reloj()));
    let refuerzos: Refuerzos = Default::default();
    let mut bots = JoinSet::new();
    for b in registro.bots.iter().filter(|b| !b.retirado) {
        bots.spawn(jugar(cfg.clone(), b.clone(), None, cerebro.clone(), pizarras.clone(), compartido.clone(), refuerzos.clone(), control.clone(), parar.clone()));
    }
    let plan = C::plan(cfg.semilla, cfg.total, cfg.dias_llegada);
    let mut extras = 0;
    let resultado = loop {
        tokio::select! {
            _ = parar.changed() => break Ok(()),
            r = reloj.atender(Duration::from_millis(250)) => if let Err(f) = r { break Err(f) },
        }
        *compartido.lock().unwrap() = reloj.reloj();
        let ahora = reloj.mundo_ms();
        control.mundo_ms.store(ahora, std::sync::atomic::Ordering::Relaxed);
        // Retirados desde el panel: quedan así en el registro (su tarea se despide sola).
        let retirados: Vec<String> = std::mem::take(&mut *control.retirar.lock().unwrap());
        if !retirados.is_empty() {
            registro.bots.iter_mut().filter(|b| retirados.contains(&b.heroe_id)).for_each(|b| b.retirado = true);
            control.bots.lock().unwrap().iter_mut().filter(|(id, _)| retirados.contains(id)).for_each(|(_, i)| i.retirado = true);
            registro.guardar(&cfg.registro)?;
        }
        let pausado = control.pausado.load(std::sync::atomic::Ordering::Relaxed);
        // Las llegadas forzadas desde el panel, fuera del plan.
        let forzadas: Vec<TipoPerfil> = std::mem::take(&mut *control.forzadas.lock().unwrap());
        for perfil in forzadas {
            let cuantos = if perfil == TipoPerfil::Amigos { llegadas::AMIGOS_POR_GRUPO } else { 1 };
            for (b, p) in alta(&cfg, &mut registro, &Llegada { en_ms: 0, grupo: 0, perfil, cuantos }).await {
                bots.spawn(jugar(cfg.clone(), b, Some(p), cerebro.clone(), pizarras.clone(), compartido.clone(), refuerzos.clone(), control.clone(), parar.clone()));
            }
            extras += 1;
        }
        while let Some(l) = plan.get(registro.llegadas_hechas).filter(|l| !pausado && registro.inicio_ms + l.en_ms <= ahora) {
            for (b, p) in alta(&cfg, &mut registro, l).await {
                bots.spawn(jugar(cfg.clone(), b, Some(p), cerebro.clone(), pizarras.clone(), compartido.clone(), refuerzos.clone(), control.clone(), parar.clone()));
            }
            registro.llegadas_hechas += 1;
            registro.guardar(&cfg.registro)?;
        }
        *control.llegadas.lock().unwrap() = (registro.llegadas_hechas, extras);
        if guardado_en.elapsed() >= GUARDAR_LIBROS {
            guardado_en = Instant::now();
            guardar_libros::<C>(&cfg, &mut registro, &pizarras, &control, ahora)?;
        }
        // ponytail: una petición de refuerzos que llega justo antes de parar se pierde; guardarla en el registro si importa.
        let pedidos: Vec<String> = std::mem::take(&mut *refuerzos.lock().unwrap());
        for faccion in pedidos {
            let l = Llegada { en_ms: 0, grupo: 0, perfil: TipoPerfil::Refuerzo(faccion), cuantos: REFUERZOS };
            for (b, p) in alta(&cfg, &mut registro, &l).await {
                bots.spawn(jugar(cfg.clone(), b, Some(p), cerebro.clone(), pizarras.clone(), compartido.clone(), refuerzos.clone(), control.clone(), parar.clone()));
            }
        }
        while let Some(r) = bots.try_join_next() {
            match r {
                Ok(Err(f)) => eprintln!("bots: un bot dejó de jugar: {f:?}"),
                Err(e) => eprintln!("bots: un bot cayó: {e}"),
                Ok(Ok(())) => {}
            }
        }
    };
    let _ = parar.borrow_and_update();
    while bots.join_next().await.is_some() {}
    *control.pizarras.lock().unwrap() = None;
    let ahora = reloj.mundo_ms();
    guardar_libros::<C>(&cfg, &mut registro, &pizarras, &control, ahora)?;
    reloj.cerrar().await;
    resultado
}

/// Cada cuánto se guardan los libros y se refresca lo publicado, en tiempo real.
const GUARDAR_LIBROS: Duration = Duration::from_secs(30);

/// Guarda en el registro lo que las Facciones recuerdan y refresca lo que publican. Solo las pizarras de Facción: las de un héroe suelto no tienen relaciones.
fn guardar_libros<C: Cerebro>(cfg: &Config, registro: &mut Registro, pizarras: &Pizarras<C>, control: &Control, ahora: i64) -> Result<(), Fallo> {
    let p = pizarras.lock().unwrap();
    let de_faccion = || p.iter().filter(|(f, _)| f.starts_with("f-"));
    registro.libros = de_faccion().map(|(f, x)| (f.clone(), C::guardar(x))).collect();
    *control.relaciones.lock().unwrap() = serde_json::Value::Object(de_faccion().map(|(f, x)| (f.clone(), C::publico(x, ahora))).collect());
    drop(p);
    registro.guardar(&cfg.registro)
}

/// Da de alta una llegada: una cuenta de bot y su héroe por cada uno, todos en el campamento que elige el primero (el de menos residentes). Cada uno queda en el registro en cuanto
/// existe; uno que falla se avisa y el grupo sigue sin él.
async fn alta(cfg: &Config, registro: &mut Registro, l: &Llegada) -> Vec<(BotRegistrado, Puerto)> {
    let mut creados: Vec<(BotRegistrado, Puerto)> = Vec::new();
    let mut campamento: Option<String> = None;
    for _ in 0..l.cuantos {
        let n = registro.bots.len() + registro.fallidas + 1;
        let cuenta = Credenciales { nick: format!("b{}-{n}", registro.serie), clave: clave_nueva(&registro.serie) };
        let nacer = async {
            acceso::registrar(&cfg.servidor, &cuenta, &cfg.codigo_bots).await?;
            let mut p = Puerto::entrar(&cfg.servidor, cuenta.clone()).await?;
            let elegido = campamento.clone().or_else(|| cfg.lugares.elegir(&p.bienvenida.campamentos_para_elegir));
            let elegido = elegido.ok_or_else(|| Fallo::Registro("no hay campamentos donde nacer".into()))?;
            let clase = crate::datos::de(&p).await?.clase(n);
            // El nombre es único en todo el servidor: si otro (un jugador, la otra población) ya lo lleva, prueba con un apellido de número.
            let mut r = None;
            for intento in 0..5 {
                let base = crate::libro::nombre(l.grupo, n);
                let display_name = if intento == 0 { base } else { format!("{base} {}", n * 10 + intento) };
                let x = p.actuar(O::CrearHeroe(OrdenCrearHeroe { display_name, campamento_id: elegido.clone(), class_definition_id: clase.clone(), ..Default::default() })).await?;
                let repetido = x.rechazo.as_ref().is_some_and(|e| e.codigo == "heroe.nombre_en_uso");
                r = Some(x);
                if !repetido {
                    break;
                }
            }
            let r = r.expect("al menos un intento");
            let heroe = r.creadas.iter().find(|c| c.entidad == TipoEntidad::Heroe as i32).map(|c| c.id.clone());
            let heroe = heroe.ok_or_else(|| Fallo::Registro(format!("crear_heroe: {r:?}")))?;
            Ok::<_, Fallo>((p, heroe, elegido))
        };
        match nacer.await {
            Ok((p, heroe_id, elegido)) => {
                campamento = Some(elegido);
                let perfil = match &l.perfil {
                    TipoPerfil::Amigos => Perfil::Amigos { lider: creados.first().map_or(heroe_id.clone(), |(b, _)| b.heroe_id.clone()) },
                    TipoPerfil::Solitario => Perfil::Solitario,
                    TipoPerfil::Tardio => Perfil::Tardio,
                    TipoPerfil::Refuerzo(f) => Perfil::Refuerzo { faccion: f.clone() },
                    TipoPerfil::Mercenario => Perfil::Mercenario,
                };
                let b = BotRegistrado { cuenta: cuenta.clone(), heroe_id, perfil, retirado: false };
                registro.bots.push(b.clone());
                if let Err(f) = registro.guardar(&cfg.registro) {
                    eprintln!("bots: {f:?}");
                }
                creados.push((b, p));
            }
            Err(f) => {
                registro.fallidas += 1;
                eprintln!("bots: no nació {}: {f:?}", cuenta.nick);
            }
        }
    }
    creados
}

/// La vida de un bot: se conecta en sus horas, piensa a su ritmo o cuando un hecho lo nombra, y atiende su sesión mientras tanto.
async fn jugar<C: Cerebro>(
    cfg: Arc<Config>,
    b: BotRegistrado,
    puerto: Option<Puerto>,
    cerebro: Arc<C>,
    pizarras: Pizarras<C>,
    reloj: RelojCompartido,
    refuerzos: Refuerzos,
    control: Arc<Control>,
    mut parar: watch::Receiver<bool>,
) -> Result<(), Fallo> {
    // Uno recién nacido trae la conexión en la que creó su héroe (su `Bienvenida` aún es la de antes); uno del registro vuelve a entrar, y su héroe tiene que seguir ahí.
    let mut puerto = match puerto {
        Some(p) => p,
        None => {
            let p = Puerto::entrar(&cfg.servidor, b.cuenta.clone()).await?;
            if p.bienvenida.heroe_id.as_deref() != Some(b.heroe_id.as_str()) {
                return Err(Fallo::Registro(format!("{} ya no tiene el héroe {} (¿otra partida?)", b.cuenta.nick, b.heroe_id)));
            }
            p
        }
    };
    let mut azar = Azar::new(cfg.semilla ^ hash(&b.cuenta.nick));
    let sesiones = if cfg.siempre { vec![(0, 24 * 60)] } else { llegadas::horario(&mut azar) };
    let ahora = || puerto::mundo_ms(&reloj.lock().unwrap());
    let desfase = (hash(&b.heroe_id) % cfg.cada_ms as u64) as i64;
    let mut proximo = ahora() - ahora().rem_euclid(cfg.cada_ms) + desfase;
    let (mut memoria, mut esperas, mut hechos, mut conectado) = (C::Memoria::default(), HashMap::new(), Vec::<EventoDominio>::new(), false);
    let pausa = || tokio::time::sleep(Duration::from_secs(5));
    let apuntar = |f: &mut dyn FnMut(&mut InfoBot)| {
        let mut bots = control.bots.lock().unwrap();
        let i = bots.entry(b.heroe_id.clone()).or_insert_with(|| InfoBot {
            heroe_id: b.heroe_id.clone(),
            nombre: b.cuenta.nick.clone(),
            perfil: Some(b.perfil.clone()),
            fase: "sin-datos".into(),
            sesiones: sesiones.clone(),
            ..Default::default()
        });
        f(i);
    };
    apuntar(&mut |_| {});
    while !*parar.borrow() {
        let t = ahora();
        // Lo que dice el panel: retirado, modo, pausa.
        if control.retirar.lock().unwrap().contains(&b.heroe_id) || control.bots.lock().unwrap().get(&b.heroe_id).is_some_and(|i| i.retirado) {
            apuntar(&mut |i| i.retirado = true);
            if let Some(o) = C::despedida() {
                let _ = puerto.actuar(o).await;
            }
            break;
        }
        let modo = control.modos.lock().unwrap().get(&b.heroe_id).copied().unwrap_or_default();
        let en_horas = match modo {
            Modo::Conectado => true,
            Modo::Desconectado => false,
            _ => llegadas::en_sesion(&sesiones, t),
        };
        apuntar(&mut |i| {
            i.modo = modo;
            i.conectado = conectado;
        });
        if !en_horas {
            if conectado {
                let _ = puerto.actuar(O::Desconectarse(OrdenDesconectarse { heroe_id: b.heroe_id.clone() })).await;
                puerto.cerrar().await;
                conectado = false;
            }
            let _ = tokio::time::timeout(Duration::from_secs(1), parar.changed()).await;
            continue;
        }
        if !conectado {
            if let Err(f) = puerto.actuar(O::Conectarse(OrdenConectarse { heroe_id: b.heroe_id.clone() })).await {
                eprintln!("bots: {} no se conecta: {f:?}", b.cuenta.nick);
                pausa().await;
                continue;
            }
            conectado = true;
        }
        let nombrado = hechos.iter().any(|h| h.heroes_ids.contains(&b.heroe_id));
        let ya = control.pensar_ya.lock().unwrap().remove(&b.heroe_id);
        let avisado = control.avisos.lock().unwrap().remove(&b.heroe_id);
        let quieto = modo == Modo::Congelado || control.pausado.load(std::sync::atomic::Ordering::Relaxed);
        if ya || (!quieto && (t >= proximo || nombrado || avisado)) {
            while proximo <= t {
                proximo += cfg.cada_ms;
            }
            let mut ctx = Contexto::<C> {
                yo: &b.heroe_id,
                perfil: &b.perfil,
                puerto: &mut puerto,
                memoria: &mut memoria,
                azar: &mut azar,
                hechos: std::mem::take(&mut hechos),
                ahora_ms: t,
                esperas: &mut esperas,
                pizarras: &pizarras,
                refuerzos: &refuerzos,
                avisos: &control.avisos,
            };
            let r = cerebro.pensar(&mut ctx).await;
            let mut volcado = C::volcar(&memoria);
            volcado["esperas"] = serde_json::json!(esperas.iter().filter(|(_, h)| **h > t).collect::<Vec<_>>());
            // Su plan en corto: el tipo y, si lo lleva, a qué («cazar band-3»).
            let plan = volcado.get("plan").map(|p| {
                let tipo = p["tipo"].as_str().unwrap_or_default();
                match p.as_object().and_then(|o| o.iter().find(|(k, v)| *k != "tipo" && v.is_string())).and_then(|(_, v)| v.as_str()) {
                    Some(a) => format!("{tipo} {a}"),
                    None => tipo.to_string(),
                }
            });
            control.memorias.lock().unwrap().insert(b.heroe_id.clone(), volcado);
            let fase = info_de(&puerto, &b.heroe_id);
            let columna = crate::cerebro::columna(puerto.observar(), &b.heroe_id).map(|c| c.id);
            apuntar(&mut |i| {
                i.pensamientos += 1;
                i.vista_en = Some(t);
                (i.fase, i.faccion_id, i.residencia_id) = fase.clone();
                (i.plan, i.columna_id) = (plan.clone(), columna.clone());
                if let Err(f) = &r {
                    i.errores += 1;
                    i.ultimo_error = Some(format!("{f:?}"));
                }
            });
            if let Err(f) = r {
                eprintln!("bots: {} al pensar: {f:?}", b.cuenta.nick);
                pausa().await;
            }
            continue;
        }
        // Hasta su turno, atendiendo la sesión; como mucho 100 ms, para ver el horario, si hay que parar y si un compañero o un hecho le despierta.
        let (r, _) = reloj.lock().unwrap().clone();
        let real = if r.pausado || r.factor_milesimas == 0 { 1000 } else { (proximo - t) * 1000 / r.factor_milesimas as i64 };
        if let Err(f) = puerto.atender(Duration::from_millis(real.clamp(1, 100) as u64)).await {
            eprintln!("bots: {}: {f:?}", b.cuenta.nick);
            pausa().await;
        }
        let nuevos: Vec<EventoDominio> = puerto.hechos().into_iter().map(|(_, h)| h).collect();
        // Lo que oye, en cuanto llega, pasa por la pizarra de su Facción: puede despertar a compañeros.
        if let Some(f) = puerto.observar().ambitos.keys().find_map(|k| k.strip_prefix("FACCION:")).filter(|_| !nuevos.is_empty()) {
            let despiertan: Vec<(String, Vec<String>)> = {
                let mut pz = pizarras.lock().unwrap();
                let p = pz.entry(f.to_string()).or_default();
                nuevos.iter().map(|h| (h.codigo.clone(), C::oir(h, p))).filter(|(_, d)| !d.is_empty()).collect()
            };
            for (codigo, d) in despiertan {
                let texto = format!("{codigo}: despierta a {}", d.len());
                let _ = eventos().send(serde_json::json!({"faccionId": f, "heroeId": b.heroe_id, "tipo": "despertar", "texto": texto, "datos": {"hecho": codigo, "despertados": d}, "ms": ahora(), "en": ms_epoca()}));
                control.avisos.lock().unwrap().extend(d);
            }
        }
        hechos.extend(nuevos);
    }
    if conectado {
        let _ = puerto.actuar(O::Desconectarse(OrdenDesconectarse { heroe_id: b.heroe_id.clone() })).await;
    }
    puerto.cerrar().await;
    Ok(())
}

/// La fase de un bot para el panel (`sin-plaza`, `en-casa`, `en-columna`), su Facción y su residencia, con lo que ve.
fn info_de(puerto: &Puerto, yo: &str) -> (String, Option<String>, Option<String>) {
    let a = puerto.observar();
    let todo = || a.ambitos.values().flat_map(|x| x.entidades.values());
    let heroe = todo().find_map(|c| match c {
        contrato::v1::entidad::Cuerpo::Heroe(h) if h.id == yo => Some(h.clone()),
        _ => None,
    });
    let faccion = a.ambitos.keys().find_map(|k| k.strip_prefix("FACCION:").map(String::from));
    let Some(h) = heroe else { return ("sin-datos".into(), faccion, None) };
    let en_columna = matches!(h.ubicacion.as_ref().and_then(|u| u.donde.as_ref()), Some(contrato::v1::ubicacion_heroe::Donde::EjercitoId(_)));
    let fase = match (&h.residencia_id, en_columna) {
        (Some(r), false) if r.starts_with("as-") => "en-casa",
        (_, true) => "en-columna",
        _ => "sin-plaza",
    };
    (fase.into(), faccion, h.residencia_id)
}

fn ms_epoca() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}

fn clave_nueva(serie: &str) -> String {
    static CUENTAS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = CUENTAS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut a = Azar::new(ms_epoca() ^ hash(serie) ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    format!("{:016x}{:016x}", a.siguiente(), a.siguiente())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn un_registro_sin_plan_se_sigue_leyendo_y_el_plan_guardado_vuelve() {
        let viejo = r#"{"mundo_hash":"m","serie":"s","inicio_ms":1,"llegadas_hechas":2,"reloj":{"nick":"r","clave":"c"},"bots":[]}"#;
        let mut r: Registro = serde_json::from_str(viejo).unwrap();
        assert_eq!(r.plan, None);
        r.plan = Some(PlanLlegadas { lugares: Lugares { campamentos: vec!["camp-3".into()], por_campamento: 4 }, total: 12, dias_llegada: 1.5, semilla: 7 });
        let otra: Registro = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(otra.plan, r.plan);
    }

    #[test]
    fn llena_los_campamentos_en_orden_y_deja_vacios_los_demas() {
        let k = |id: &str, residentes| contrato::v1::CampamentoParaElegir { campamento_id: id.into(), residentes, ..Default::default() };
        let c = [k("camp-1", 0), k("camp-3", 5), k("camp-7", 2)];
        assert_eq!(Lugares::default().elegir(&c).as_deref(), Some("camp-1"), "sin lista, el que menos tiene");
        let l = |por_campamento| Lugares { campamentos: vec!["camp-3".into(), "camp-7".into()], por_campamento };
        assert_eq!(l(6).elegir(&c).as_deref(), Some("camp-3"), "el primero hasta llenarlo");
        assert_eq!(l(5).elegir(&c).as_deref(), Some("camp-7"), "lleno, el siguiente");
        assert_eq!(l(0).elegir(&c).as_deref(), Some("camp-7"), "sin tope, se reparten entre los de la lista");
        assert_eq!(l(2).elegir(&c).as_deref(), Some("camp-7"), "todos llenos, el que menos tiene");
    }
}
