//! El batch de «economía y comercio» sobre el artefacto real; se omite si no está generado.

use batch::{artefacto_en, cargar, carpeta_por_defecto, comercio, fundir_decisiones, semaforo, Cargado, EscenarioComercio, Informe, Semaforo};
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

fn pequeno() -> EscenarioComercio {
    EscenarioComercio { plazas: 16, dias: 3, semilla: 42, verificar_determinismo: true, medir_coste: true }
}

fn medida(i: &Informe, id: &str) -> f64 {
    i.metricas.iter().find(|m| m.id == id).unwrap_or_else(|| panic!("falta «{id}»")).medido.unwrap_or_else(|| panic!("«{id}» sin medir"))
}

/// El informe sin lo que depende del reloj de pared (fecha y tiempos), para comparar dos ejecuciones.
fn sin_tiempos(mut i: Informe) -> String {
    i.fecha.clear();
    for m in i.metricas.iter_mut().filter(|m| ["pared_total", "pared_corrida", "pared_control", "us_por_evento_de_comercio"].contains(&m.id.as_str())) {
        m.medido = None;
    }
    serde_json::to_string(&i).unwrap()
}

#[test]
fn el_escenario_pequeno_produce_el_informe_completo() {
    let c = con_mundo!();
    let i = comercio(c, &pequeno());
    let ids: std::collections::BTreeSet<_> = i.metricas.iter().map(|m| &m.id).collect();
    assert_eq!(ids.len(), i.metricas.len(), "ids repetidos");
    for id in [
        "colocadas", "mercado_pct", "flota_pct", "trueques_propuestos", "trueques_cumplimiento_pct", "viajes_por_trueque", "caravanas_lanzadas", "caravanas_perdidas", "peajes_cobros", "rep_max_alcanzada", "red_aristas", "red_aceleracion", "merc_compras", "merc_cupo_madera_max",
        "pueblos_marcados", "acarreo_perdido_pct", "inv_almacen", "inv_carga", "inv_trueque", "inv_sin_plan", "inv_cola", "inv_plaza_ociosa", "inv_minuto", "inv_registro", "inv_conservacion", "pared_total", "eventos", "us_por_evento_de_comercio", "determinismo", "determinismo_normal",
    ] {
        assert!(i.metricas.iter().any(|m| m.id == id), "falta la métrica «{id}»");
    }
    assert_eq!(i.series.len(), 11);
    let mapa = i.mapa.as_ref().unwrap();
    for nombre in ["Zonas de nivel 1", "Rutas vigentes al final", "Pueblos extractores", "Campamentos de mercenarios"] {
        assert!(mapa.capas.iter().any(|c| c.nombre == nombre), "falta la capa «{nombre}»");
    }
    assert!(mapa.fondo.as_deref().is_some_and(|f| f.starts_with("data:image/bmp;base64,")), "el mapa no lleva fondo");
    assert!(informe::renderizar(&i).contains("Economía y comercio"));
    assert!(medida(&i, "colocadas") > 0.0 && medida(&i, "eventos") > 0.0);
}

#[test]
fn los_invariantes_y_el_determinismo_se_cumplen() {
    let c = con_mundo!();
    let i = comercio(c, &pequeno());
    for id in ["inv_almacen", "inv_carga", "inv_trueque", "inv_sin_plan", "inv_cola", "inv_plaza_ociosa", "inv_minuto", "inv_registro", "inv_conservacion", "determinismo", "determinismo_normal"] {
        let m = i.metricas.iter().find(|m| m.id == id).unwrap();
        assert_eq!(semaforo(m), Semaforo::Verde, "{id}: {:?}", m.medido);
    }
    assert!(i.anomalias.is_empty());
}

#[test]
fn el_mismo_escenario_da_el_mismo_informe_y_otra_semilla_otro() {
    let c = con_mundo!();
    let e = EscenarioComercio { verificar_determinismo: false, medir_coste: false, ..pequeno() };
    let (a, b) = (comercio(c, &e), comercio(c, &e));
    assert_eq!(sin_tiempos(a.clone()), sin_tiempos(b));
    assert_ne!(sin_tiempos(a), sin_tiempos(comercio(c, &EscenarioComercio { semilla: 7, ..e })), "la semilla no cambia nada");
}

#[test]
fn con_comercio_hay_trueques_caravanas_y_pueblos_y_cuadran_las_cuentas() {
    let c = con_mundo!();
    let i = comercio(c, &EscenarioComercio { plazas: 40, dias: 12, semilla: 42, verificar_determinismo: false, medir_coste: false });
    assert!(medida(&i, "trueques_cumplidos") > 0.0, "ningún trueque en 12 días");
    assert_eq!(medida(&i, "caravanas_lanzadas"), medida(&i, "caravanas_entregadas"), "toda caravana lanzada entrega");
    assert!(medida(&i, "pueblos_completados") > 0.0 && medida(&i, "acarreo_viajes") > 0.0, "ningún pueblo con acarreo");
    assert!(medida(&i, "merc_compras") > 0.0 && medida(&i, "red_aristas") > 0.0);
    for id in ["inv_almacen", "inv_carga", "inv_trueque", "inv_sin_plan", "inv_cola", "inv_plaza_ociosa", "inv_minuto", "inv_registro", "inv_conservacion"] {
        assert_eq!(medida(&i, id), 0.0, "{id}");
    }
}

#[test]
fn las_decisiones_del_informe_se_funden() {
    let c = con_mundo!();
    let mut i = comercio(c, &EscenarioComercio { verificar_determinismo: false, medir_coste: false, ..pequeno() });
    fundir_decisiones(&mut i, include_str!("../decisiones/comercio.json")).unwrap();
    assert!(!i.resumen.is_empty() && i.decisiones.len() >= 3 && i.decisiones.iter().all(|d| d.opciones.iter().filter(|o| o.recomendada).count() == 1));
}
