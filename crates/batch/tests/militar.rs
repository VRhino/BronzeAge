//! El batch de «militar y combate» sobre el artefacto real; se omite si no está generado. Un escenario corto (seis plazas, un día) con los invariantes del
//! combate comprobados orden a orden, la reproducción del azar y la foto ida y vuelta.

use batch::{artefacto_en, cargar, carpeta_por_defecto, fundir_decisiones, militar, semaforo, Cargado, EscenarioMilitar, Informe, Semaforo};
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

fn corto() -> EscenarioMilitar {
    EscenarioMilitar { plazas: 6, dias: 1, semilla: 42, verificar_determinismo: true }
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
fn el_escenario_corto_cumple_los_invariantes_reproduce_el_azar_y_sobrevive_a_la_foto() {
    let c = con_mundo!();
    let i = militar(c, &corto());
    let ids: std::collections::BTreeSet<_> = i.metricas.iter().map(|m| &m.id).collect();
    assert_eq!(ids.len(), i.metricas.len(), "ids repetidos");
    for m in i.metricas.iter().filter(|m| m.id.starts_with("inv_") || m.id.starts_with("determinismo")) {
        assert_eq!(semaforo(m), Semaforo::Verde, "{}: {:?}", m.id, m.medido);
    }
    assert!(i.anomalias.is_empty(), "{:?}", i.anomalias);
    // Los bots hacen de todo lo que da tiempo en un día: reclutan, salen, combaten (al menos contra los bandidos de casa) y convocan.
    for id in ["reclutados", "salidas", "combates", "combates_campamento", "convocatorias", "heroes"] {
        assert!(medida(&i, id) > 0.0, "«{id}» a 0");
    }
    assert!(i.parametros.iter().any(|p| p.nombre.starts_with("Foto ida y vuelta") && p.valor.starts_with("hash antes")), "la corrida hace la foto a mitad");
    assert_eq!(i.series.len(), 5);
    assert!(informe::renderizar(&i).contains("Militar y combate"));
    // El JSON de decisiones del repositorio se funde sin errores.
    let mut con_decisiones = i.clone();
    fundir_decisiones(&mut con_decisiones, include_str!("../decisiones/militar.json")).expect("decisiones/militar.json es válido");
    assert!(!con_decisiones.decisiones.is_empty());
}

#[test]
fn la_misma_semilla_da_el_mismo_informe_y_otra_otro() {
    let c = con_mundo!();
    let e = EscenarioMilitar { verificar_determinismo: false, ..corto() };
    let (a, b) = (militar(c, &e), militar(c, &e));
    assert_eq!(sin_tiempos(a.clone()), sin_tiempos(b));
    assert_ne!(sin_tiempos(a), sin_tiempos(militar(c, &EscenarioMilitar { semilla: 7, ..e })), "la semilla no cambia nada");
}
