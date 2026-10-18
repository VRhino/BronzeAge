//! Batch de validación del sistema «mundo y territorio» (fase 6): una población sintética de Facciones juega semanas de
//! mundo sobre el núcleo en modo acelerado y las medidas resultantes se comparan con lo que el diseño espera.
//!
//! - [`territorio`] corre el escenario y devuelve el [`Informe`].
//! - [`escribir`] deja `informe.json` e `informe.html` en una carpeta.
//! - [`fundir_decisiones`] añade a mano el resumen, las anomalías y las decisiones tras leer los resultados.

mod armar;
mod fondo;
mod geo;
mod medidas;
mod sim;

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

/// 50 Facciones y 3 días, dos veces: ¿mismo `hash_estado()`?
fn es_determinista(c: &Cargado, geo: &geo::Geo, semilla: u64) -> bool {
    let a = sim::simular(c, geo, 50, 3, semilla);
    let b = sim::simular(c, geo, 50, 3, semilla);
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
