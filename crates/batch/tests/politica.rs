//! El batch de «política y Facciones» sobre el artefacto real; se omite si no está generado. Un escenario corto (seis plazas, dos días) con los invariantes de la política
//! y del combate, la reproducción del azar y la foto ida y vuelta.

use batch::{artefacto_en, cargar, carpeta_por_defecto, fundir_decisiones, politica, semaforo, Cargado, EscenarioPolitica, Informe, Semaforo};
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

fn corto() -> EscenarioPolitica {
    EscenarioPolitica { plazas: 6, dias: 2, semilla: 42, verificar_determinismo: true, nobleza: true }
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
fn el_escenario_corto_cumple_los_invariantes_de_la_politica_reproduce_el_azar_y_sobrevive_a_la_foto() {
    let c = con_mundo!();
    let i = politica(c, &corto());
    let ids: std::collections::BTreeSet<_> = i.metricas.iter().map(|m| &m.id).collect();
    assert_eq!(ids.len(), i.metricas.len(), "ids repetidos");
    for id in ["inv_ciudadania", "inv_rey", "inv_cargos", "inv_relaciones", "inv_arrastre", "inv_tributo", "inv_reputacion", "inv_traslado", "inv_ids_faccion", "inv_xp_faccion", "inv_rechazos", "inv_foto"] {
        assert!(ids.contains(&id.to_string()), "falta el invariante «{id}»");
    }
    for m in i.metricas.iter().filter(|m| m.id.starts_with("inv_") || m.id.starts_with("determinismo")) {
        assert_eq!(semaforo(m), Semaforo::Verde, "{}: {:?}", m.id, m.medido);
    }
    assert!(i.anomalias.is_empty(), "{:?}", i.anomalias);
    // Las Facciones nacen con su orden, sus héroes piden el ingreso y se mudan, y el Gobernador reparte los cargos.
    for id in ["pol_facciones_creadas", "pol_ingresos", "pol_mudanzas", "pol_cargos", "reclutados"] {
        assert!(medida(&i, id) > 0.0, "«{id}» a 0");
    }
    assert!(i.parametros.iter().any(|p| p.nombre.starts_with("Foto ida y vuelta") && p.valor.starts_with("hash antes")), "la corrida hace la foto a mitad");
    assert!(informe::renderizar(&i).contains("Política y Facciones"));
    // El JSON de decisiones del repositorio se funde sin errores.
    let mut con_decisiones = i.clone();
    fundir_decisiones(&mut con_decisiones, include_str!("../decisiones/politica.json")).expect("decisiones/politica.json es válido");
    assert!(!con_decisiones.decisiones.is_empty());
}

#[test]
fn la_misma_semilla_da_el_mismo_informe_politico_y_otra_otro() {
    let c = con_mundo!();
    let e = EscenarioPolitica { verificar_determinismo: false, ..corto() };
    let (a, b) = (politica(c, &e), politica(c, &e));
    assert_eq!(sin_tiempos(a.clone()), sin_tiempos(b));
    assert_ne!(sin_tiempos(a), sin_tiempos(politica(c, &EscenarioPolitica { semilla: 7, ..e })), "la semilla no cambia nada");
}
