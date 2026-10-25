//! Batch de validación del sistema «mundo y territorio» (fase 6): una población sintética de Facciones juega semanas de
//! mundo sobre el núcleo en modo acelerado y las medidas resultantes se comparan con lo que el diseño espera.
//!
//! - [`territorio`] corre el escenario de «mundo y territorio» y devuelve el [`Informe`].
//! - [`poblacion`] corre el de «población, construcción y mantenimiento» (plazas del crate `asentamiento` sobre el mundo real, cada una con
//!   su ciudad real del trazado urbano).
//! - [`comercio`] corre el de «economía y comercio» (Facciones de varias plazas que comercian sobre el núcleo: trueques, caravanas que viajan por
//!   el mapa, peajes, red de caminos, mercenarios y pueblos extractores con acarreo).
//! - [`militar`] corre el de «militar y combate» (héroes bots que reclutan, salen, comen, persiguen, convocan y combaten con números contra columnas, caravanas,
//!   bandidos, pueblos, avanzadas y plazas, con los invariantes del combate comprobados orden a orden).
//! - [`politica`] corre el de «política y Facciones» (la población del militar con Facciones, ciudadanía, residencia, cargos, puerta, relaciones, tributo, capital, anexión,
//!   fusión, refundación y títulos en manos de bots de diplomacia que solo dan órdenes del contrato, con los invariantes de la política comprobados en cada observación).
//! - [`tecnologia`] corre el de «tecnología y Aedas» (la población del político con bots de tecnología que construyen hacia los hitos, adoptan, compran a los Aedas, encargan épicas, abren
//!   alijos y reclutan en los campamentos, con los invariantes del sistema 6 comprobados orden a orden y la crónica guardada).
//! - [`escribir`] deja `informe.json` e `informe.html` en una carpeta.
//! - [`fundir_decisiones`] añade a mano el resumen, las anomalías y las decisiones tras leer los resultados.

mod armar;
mod armarcom;
mod armarmil;
mod armarpob;
mod armarpol;
mod armartec;
mod auditmil;
mod auditoria;
mod economia;
mod fondo;
mod geo;
mod medidas;
mod sim;
mod simcom;
mod simmil;
mod simpob;
mod svgciudad;
mod vigia;

pub use geo::{artefacto_en, cargar, carpeta_por_defecto, Cargado};
pub use informe::{semaforo, Informe, Semaforo};

use std::path::Path;
use std::time::Instant;

/// Parámetros de un escenario del batch.
#[derive(Clone, Debug)]
pub struct Escenario {
    pub facciones: u32,
    pub dias: u32,
    pub semilla: u64,
    /// Repite un escenario reducido dos veces y compara los hashes del estado (unos segundos).
    pub verificar_determinismo: bool,
}

impl Default for Escenario {
    fn default() -> Self {
        Self { facciones: 400, dias: 30, semilla: 42, verificar_determinismo: true }
    }
}

/// Corre el escenario sobre el mundo y monta el informe.
pub fn territorio(c: &Cargado, e: &Escenario) -> Informe {
    let t0 = Instant::now();
    let geo = geo::Geo::de(&c.mundo);
    let malla = medidas::sondear_malla(c, &geo, e.semilla);
    let corrida = sim::simular(c, &geo, e.facciones, e.dias, e.semilla);
    let determinismo = e.verificar_determinismo.then(|| es_determinista(c, &geo, e.semilla));
    armar::armar(&armar::Entrada { c, geo: &geo, e, corrida: &corrida, malla: &malla, determinismo, pared_total: t0.elapsed() })
}

/// Parámetros del escenario de población: plazas fundadas a la vez, simuladas minuto a minuto.
#[derive(Clone, Debug)]
pub struct EscenarioPoblacion {
    pub plazas: u32,
    pub dias: u32,
    pub semilla: u64,
    /// Repite un escenario reducido dos veces y compara los hashes del estado.
    pub verificar_determinismo: bool,
    /// El jugador compromete la empalizada en cuanto el nivel de la plaza lo permite (paso 7 del plan de Murallas).
    pub murallas: bool,
}

impl Default for EscenarioPoblacion {
    fn default() -> Self {
        Self { plazas: 300, dias: 30, semilla: 42, verificar_determinismo: true, murallas: true }
    }
}

/// Corre el escenario de población sobre el mundo y monta el informe.
pub fn poblacion(c: &Cargado, e: &EscenarioPoblacion) -> Informe {
    poblacion_y_ciudad(c, e).0
}

/// Lo mismo que [`poblacion`] y, además, los SVG de la ciudad (nombre del fichero y contenido): el de una plaza de ejemplo (`ciudad_ejemplo.svg`), si queda alguna en
/// pie, y el de una plaza amurallada que ya no halla sitio para Viviendas (`ciudad_bloqueada.svg`), si la hay.
pub fn poblacion_y_ciudad(c: &Cargado, e: &EscenarioPoblacion) -> (Informe, Vec<(&'static str, String)>) {
    let t0 = Instant::now();
    let mapa = economia::Mapa::de(&c.mundo);
    let corrida = simpob::simular(&c.mundo, &mapa, e.plazas, e.dias, e.semilla, e.murallas);
    let determinismo = e.verificar_determinismo.then(|| {
        let (a, b) = (simpob::simular(&c.mundo, &mapa, 20, 3, e.semilla, e.murallas), simpob::simular(&c.mundo, &mapa, 20, 3, e.semilla, e.murallas));
        simpob::hash_estado(&a) == simpob::hash_estado(&b)
    });
    let re = nucleo::ReglasEconomia::default();
    let svg: Vec<(&'static str, String)> = [("ciudad_ejemplo.svg", svgciudad::ciudad_ejemplo(&corrida, &re.datos)), ("ciudad_bloqueada.svg", svgciudad::ciudad_bloqueada(&corrida, &re.datos))].into_iter().filter_map(|(n, s)| Some((n, s?))).collect();
    (armarpob::armar(&armarpob::Entrada { c, mapa: &mapa, e, corrida: &corrida, reglas: &re.datos, urb: &re.urbanismo, determinismo, pared_total: t0.elapsed() }), svg)
}

/// Parámetros del escenario de comercio: plazas fundadas a la vez (en Facciones de cuatro) que comercian sobre el núcleo.
#[derive(Clone, Debug)]
pub struct EscenarioComercio {
    pub plazas: u32,
    pub dias: u32,
    pub semilla: u64,
    /// Repite un escenario reducido y compara las huellas del núcleo (dos veces con la misma semilla; una más con el reloj a pasos).
    pub verificar_determinismo: bool,
    /// Repite el escenario sin comercio para medir lo que cuesta.
    pub medir_coste: bool,
}

impl Default for EscenarioComercio {
    fn default() -> Self {
        Self { plazas: 160, dias: 30, semilla: 42, verificar_determinismo: true, medir_coste: true }
    }
}

/// Corre el escenario de comercio sobre el mundo y monta el informe.
pub fn comercio(c: &Cargado, e: &EscenarioComercio) -> Informe {
    let t0 = Instant::now();
    let geo = geo::Geo::de(&c.mundo);
    let corrida = simcom::simular(c, &geo, e.plazas, e.dias, e.semilla, false, false);
    let control = e.medir_coste.then(|| simcom::simular(c, &geo, e.plazas, e.dias, e.semilla, true, false));
    let determinismo = e.verificar_determinismo.then(|| {
        let reducida = |paso_normal| simcom::huella(&simcom::simular(c, &geo, 20, 4, e.semilla, false, paso_normal));
        let (a, b, normal) = (reducida(false), reducida(false), reducida(true));
        (a == b, a == normal)
    });
    armarcom::armar(&armarcom::Entrada { c, geo: &geo, e, corrida: &corrida, control: control.as_ref(), determinismo, pared_total: t0.elapsed() })
}

/// Parámetros del escenario militar: plazas fundadas a la vez (en Facciones de tres) con cuatro héroes bots cada una.
#[derive(Clone, Debug)]
pub struct EscenarioMilitar {
    pub plazas: u32,
    pub dias: u32,
    pub semilla: u64,
    /// Repite un escenario reducido (9 plazas y 2 días, o menos si el escenario es menor) dos veces (mismo hash) y una más con la foto ida y vuelta a mitad (mismo
    /// hash que sin ella).
    pub verificar_determinismo: bool,
}

impl Default for EscenarioMilitar {
    fn default() -> Self {
        Self { plazas: 60, dias: 30, semilla: 42, verificar_determinismo: true }
    }
}

/// Corre el escenario militar sobre el mundo y monta el informe. La corrida principal hace también la foto ida y vuelta a mitad de camino.
pub fn militar(c: &Cargado, e: &EscenarioMilitar) -> Informe {
    let t0 = Instant::now();
    let geo = geo::Geo::de(&c.mundo);
    let corrida = simmil::simular(c, &geo, e.plazas, e.dias, e.semilla, true);
    let determinismo = e.verificar_determinismo.then(|| {
        let reducida = |foto| simmil::huella(&simmil::simular(c, &geo, e.plazas.min(9), e.dias.min(2), e.semilla, foto));
        let (a, b, foto) = (reducida(false), reducida(false), reducida(true));
        (a == b, a == foto)
    });
    armarmil::armar(&armarmil::Entrada { c, geo: &geo, e, corrida: &corrida, determinismo, pared_total: t0.elapsed() })
}

/// Parámetros del escenario político: los del militar y, como experimento del riesgo R1, la nobleza.
#[derive(Clone, Debug)]
pub struct EscenarioPolitica {
    pub plazas: u32,
    pub dias: u32,
    pub semilla: u64,
    /// Repite un escenario reducido (9 plazas y 2 días, o menos) dos veces (mismo hash) y una más con la foto ida y vuelta a mitad (mismo hash que sin ella).
    pub verificar_determinismo: bool,
    /// `false`: experimento sin nobleza (riesgo R1); las reglas no cambian.
    pub nobleza: bool,
}

impl Default for EscenarioPolitica {
    fn default() -> Self {
        Self { plazas: 60, dias: 30, semilla: 42, verificar_determinismo: true, nobleza: true }
    }
}

/// Corre el escenario político sobre el mundo y monta el informe. La corrida principal hace también la foto ida y vuelta a mitad de camino.
pub fn politica(c: &Cargado, e: &EscenarioPolitica) -> Informe {
    let t0 = Instant::now();
    let geo = geo::Geo::de(&c.mundo);
    let ajustes = simmil::pol::Ajustes { nobleza: e.nobleza };
    let corrida = simmil::simular_politica(c, &geo, e.plazas, e.dias, e.semilla, true, ajustes);
    let determinismo = e.verificar_determinismo.then(|| {
        let reducida = |foto| simmil::huella(&simmil::simular_politica(c, &geo, e.plazas.min(9), e.dias.min(2), e.semilla, foto, ajustes));
        let (a, b, foto) = (reducida(false), reducida(false), reducida(true));
        (a == b, a == foto)
    });
    armarpol::armar(&armarpol::Entrada { c, geo: &geo, e, corrida: &corrida, determinismo, pared_total: t0.elapsed() })
}

/// Parámetros del escenario tecnológico: los del político y el experimento de los umbrales.
#[derive(Clone, Debug)]
pub struct EscenarioTecnologia {
    pub plazas: u32,
    pub dias: u32,
    pub semilla: u64,
    /// Repite un escenario reducido (9 plazas y 2 días, o menos) dos veces (mismo hash) y una más con la foto ida y vuelta a mitad (mismo hash que sin ella).
    pub verificar_determinismo: bool,
    /// 1 = los umbrales de los logros del TypeScript (Q3). Mayor: cada uno dividido por ella (experimento para ejercitar las reglas; el bloque no cambia).
    pub escala_logros: u64,
}

impl Default for EscenarioTecnologia {
    fn default() -> Self {
        Self { plazas: 60, dias: 42, semilla: 42, verificar_determinismo: true, escala_logros: 1 }
    }
}

/// Corre el escenario tecnológico sobre el mundo y monta el informe. La corrida principal hace también la foto ida y vuelta a mitad de camino (en el directorio de la partida).
pub fn tecnologia(c: &Cargado, e: &EscenarioTecnologia) -> Informe {
    let t0 = Instant::now();
    let geo = geo::Geo::de(&c.mundo);
    let ajustes = simmil::tec::Ajustes { escala_logros: e.escala_logros.max(1) };
    let corrida = simmil::simular_tecnologia(c, &geo, e.plazas, e.dias, e.semilla, true, ajustes);
    let determinismo = e.verificar_determinismo.then(|| {
        let reducida = |foto| simmil::huella(&simmil::simular_tecnologia(c, &geo, e.plazas.min(9), e.dias.min(2), e.semilla, foto, ajustes));
        let (a, b, foto) = (reducida(false), reducida(false), reducida(true));
        (a == b, a == foto)
    });
    armartec::armar(&armartec::Entrada { c, geo: &geo, e, corrida: &corrida, determinismo, pared_total: t0.elapsed() })
}

/// 25 Facciones y 2 días, dos veces: ¿mismo `hash_estado()`?
fn es_determinista(c: &Cargado, geo: &geo::Geo, semilla: u64) -> bool {
    let a = sim::simular(c, geo, 25, 2, semilla);
    let b = sim::simular(c, geo, 25, 2, semilla);
    a.nucleo.hash_estado() == b.nucleo.hash_estado()
}

/// Funde el JSON de decisiones (campos opcionales `resumen`, `anomalias`, `decisiones`) en el informe: el resumen y las
/// decisiones sustituyen a los actuales; las anomalías se añaden a las automáticas.
pub fn fundir_decisiones(i: &mut Informe, json: &str) -> Result<(), String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("JSON de decisiones no válido: {e}"))?;
    let campo = |nombre: &str| v.get(nombre).filter(|x| !x.is_null()).cloned();
    if let Some(r) = campo("resumen") {
        i.resumen = serde_json::from_value(r).map_err(|e| format!("«resumen»: {e}"))?;
    }
    if let Some(a) = campo("anomalias") {
        let mut nuevas: Vec<informe::Anomalia> = serde_json::from_value(a).map_err(|e| format!("«anomalias»: {e}"))?;
        i.anomalias.append(&mut nuevas);
    }
    if let Some(d) = campo("decisiones") {
        i.decisiones = serde_json::from_value(d).map_err(|e| format!("«decisiones»: {e}"))?;
    }
    Ok(())
}

/// Escribe `informe.json` e `informe.html` en `dir` (se crea si no existe).
pub fn escribir(i: &Informe, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("no se puede crear {}: {e}", dir.display()))?;
    let json = serde_json::to_string_pretty(i).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("informe.json"), json).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("informe.html"), informe::renderizar(i)).map_err(|e| e.to_string())
}

/// Resumen de una línea por métrica, con su semáforo.
pub fn resumen_en_texto(i: &Informe) -> String {
    let marca = |s| match s {
        Semaforo::Verde => "VERDE   ",
        Semaforo::Ambar => "AMBAR   ",
        Semaforo::Rojo => "ROJO    ",
        Semaforo::Informativa => "info    ",
        Semaforo::SinDato => "sin dato",
    };
    i.metricas
        .iter()
        .map(|m| format!("[{}] {}: {} {}", marca(semaforo(m)), m.nombre, m.medido.map_or("—".into(), |v| v.to_string()), m.unidad))
        .collect::<Vec<_>>()
        .join("\n")
}
