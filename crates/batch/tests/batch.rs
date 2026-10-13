//! El batch de «mundo y territorio» sobre el artefacto real; se omite si no está generado.

use batch::{artefacto_en, cargar, carpeta_por_defecto, fundir_decisiones, semaforo, territorio, Cargado, Escenario, Informe, Semaforo};
use std::sync::OnceLock;

fn mundo() -> Option<&'static Cargado> {
    static M: OnceLock<Option<Cargado>> = OnceLock::new();
    M.get_or_init(|| cargar(&artefacto_en(&carpeta_por_defecto()).ok()?).ok()).as_ref()
}

macro_rules! con_mundo {
    () => {
        match mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

fn pequeno() -> Escenario {
    Escenario { facciones: 20, dias: 2, semilla: 42, verificar_determinismo: true }
}

/// El informe sin lo que depende del reloj de pared (fecha y tiempos), para comparar dos ejecuciones.
fn sin_tiempos(mut i: Informe) -> String {
    i.fecha.clear();
    for m in i.metricas.iter_mut().filter(|m| ["pared_total", "pared_por_dia", "viabilidad_tiempo"].contains(&m.id.as_str())) {
        m.medido = None;
    }
    serde_json::to_string(&i).unwrap()
}

#[test]
fn el_escenario_pequeno_produce_todas_las_metricas() {
    let c = con_mundo!();
    let i = territorio(c, &pequeno());
    assert!(i.metricas.len() >= 30);
    for m in &i.metricas {
        let v = m.medido.unwrap_or_else(|| panic!("«{}» sin medir", m.id));
        assert!(v.is_finite(), "{} = {v}", m.id);
    }
    let ids: std::collections::BTreeSet<_> = i.metricas.iter().map(|m| &m.id).collect();
    assert_eq!(ids.len(), i.metricas.len(), "ids repetidos");
    assert_eq!(i.series.len(), 5);
    assert_eq!(i.mapa.as_ref().unwrap().capas.len(), 3);
    let html = informe::renderizar(&i);
    assert!(html.contains("Mundo y territorio") && html.contains("Distancia mínima entre plazas"));
}

#[test]
fn los_invariantes_se_cumplen() {
    let c = con_mundo!();
    let i = territorio(c, &pequeno());
    for id in ["separacion_min", "plazas_en_zona_ajena", "plazas_cerca_campamento", "cuota_max_masa", "determinismo"] {
        let m = i.metricas.iter().find(|m| m.id == id).unwrap();
        assert_eq!(semaforo(m), Semaforo::Verde, "{id}: {:?}", m.medido);
    }
    assert!(i.anomalias.is_empty());
}

#[test]
fn el_mismo_escenario_da_el_mismo_informe() {
    let c = con_mundo!();
    let (a, b) = (territorio(c, &pequeno()), territorio(c, &pequeno()));
    assert_eq!(sin_tiempos(a), sin_tiempos(b));
}

#[test]
fn las_decisiones_se_funden_en_el_informe() {
    let c = con_mundo!();
    let mut i = territorio(c, &Escenario { verificar_determinismo: false, ..pequeno() });
    let json = r#"{"resumen":"Leído a mano","anomalias":[{"titulo":"A","causa":"diseno"}],"decisiones":[{"id":"d1","pregunta":"¿Qué?","opciones":[{"texto":"Esto","recomendada":true}]}]}"#;
    fundir_decisiones(&mut i, json).unwrap();
    assert_eq!((i.resumen.as_str(), i.anomalias.len(), i.decisiones.len()), ("Leído a mano", 1, 1));
    assert!(fundir_decisiones(&mut i, "no es json").is_err());
}
