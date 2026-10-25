//! El batch de «tecnología y Aedas» sobre el artefacto real; se omite si no está generado. Un escenario corto (nueve plazas, cuatro días) con los umbrales de los logros divididos por mil
//! (para que en pocos días haya logros, apariciones, adopciones, compras y épicas), los invariantes del sistema 6 y los del político y el militar, la reproducción del azar, la foto ida y
//! vuelta y la crónica guardada.

use batch::{artefacto_en, cargar, carpeta_por_defecto, fundir_decisiones, semaforo, tecnologia, Cargado, EscenarioTecnologia, Informe, Semaforo};
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

fn corto() -> EscenarioTecnologia {
    EscenarioTecnologia { plazas: 9, dias: 4, semilla: 42, verificar_determinismo: true, escala_logros: 1000 }
}

fn medida(i: &Informe, id: &str) -> f64 {
    i.metricas.iter().find(|m| m.id == id).unwrap_or_else(|| panic!("falta «{id}»")).medido.unwrap_or_else(|| panic!("«{id}» sin medir"))
}

/// El informe sin lo que depende del reloj de pared (fecha y tiempos), para comparar dos ejecuciones.
fn sin_tiempos(mut i: Informe) -> String {
    i.fecha.clear();
    for m in i.metricas.iter_mut().filter(|m| ["pared_total", "pared_corrida", "cpu_por_dia"].contains(&m.id.as_str())) {
        m.medido = None;
    }
    serde_json::to_string(&i).unwrap()
}

#[test]
fn el_escenario_corto_cumple_los_invariantes_de_la_tecnologia_reproduce_el_azar_y_guarda_la_cronica() {
    let c = con_mundo!();
    let i = tecnologia(c, &corto());
    let ids: std::collections::BTreeSet<_> = i.metricas.iter().map(|m| &m.id).collect();
    assert_eq!(ids.len(), i.metricas.len(), "ids repetidos");
    for id in ["inv_logro", "inv_contador", "inv_adoptadas", "inv_era_cerrada", "inv_era", "inv_sin_logro", "inv_epica", "inv_doble", "inv_itinerantes", "inv_puerta_aedas", "inv_alijo", "inv_cronica", "inv_rechazos", "inv_foto", "inv_ciudadania"] {
        assert!(ids.contains(&id.to_string()), "falta el invariante «{id}»");
    }
    for m in i.metricas.iter().filter(|m| m.id.starts_with("inv_") || m.id.starts_with("determinismo")) {
        assert_eq!(semaforo(m), Semaforo::Verde, "{}: {:?}", m.id, m.medido);
    }
    assert!(i.anomalias.is_empty(), "{:?}", i.anomalias);
    // Hay logros, apariciones, adopciones por la orden, Aedas, alijos abiertos y reclutas de campamento.
    for id in ["tec_adopciones", "tec_aparecidas", "tec_itinerantes", "tec_alijos", "tec_reclutas_campamento"] {
        assert!(medida(&i, id) > 0.0, "«{id}» a 0");
    }
    assert!(i.metricas.iter().any(|m| m.id.starts_with("dia_logro_") && m.medido.is_some()), "algún logro con su día");
    assert!(i.parametros.iter().any(|p| p.nombre.starts_with("Foto ida y vuelta") && p.valor.starts_with("hash antes")), "la corrida hace la foto a mitad");
    let cronica = i.parametros.iter().find(|p| p.nombre == "Crónica").expect("la crónica en los parámetros");
    assert!(!cronica.valor.contains(": 0 entradas emitidas"), "hay crónica: {}", cronica.valor);
    assert!(informe::renderizar(&i).contains("Tecnología y Aedas"));
    let mut con_decisiones = i.clone();
    fundir_decisiones(&mut con_decisiones, include_str!("../decisiones/tecnologia.json")).expect("decisiones/tecnologia.json es válido");
    assert!(!con_decisiones.decisiones.is_empty());
}

#[test]
fn la_misma_semilla_da_el_mismo_informe_tecnologico_y_otra_otro() {
    let c = con_mundo!();
    let e = EscenarioTecnologia { verificar_determinismo: false, ..corto() };
    let (a, b) = (tecnologia(c, &e), tecnologia(c, &e));
    assert_eq!(sin_tiempos(a.clone()), sin_tiempos(b));
    assert_ne!(sin_tiempos(a), sin_tiempos(tecnologia(c, &EscenarioTecnologia { semilla: 7, ..e })), "la semilla no cambia nada");
}
