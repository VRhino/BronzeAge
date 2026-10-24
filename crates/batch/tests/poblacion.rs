//! El batch de «población, construcción y mantenimiento» sobre el artefacto real; se omite si no está generado.

use batch::{artefacto_en, cargar, carpeta_por_defecto, fundir_decisiones, poblacion, poblacion_y_ciudad, semaforo, Cargado, EscenarioPoblacion, Informe, Semaforo};
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

fn pequeno() -> EscenarioPoblacion {
    EscenarioPoblacion { plazas: 20, dias: 4, semilla: 42, verificar_determinismo: true, murallas: true }
}

fn medida(i: &Informe, id: &str) -> f64 {
    i.metricas.iter().find(|m| m.id == id).unwrap_or_else(|| panic!("falta «{id}»")).medido.unwrap_or_else(|| panic!("«{id}» sin medir"))
}

/// El informe sin lo que depende del reloj de pared (fecha y tiempos), para comparar dos ejecuciones.
fn sin_tiempos(mut i: Informe) -> String {
    i.fecha.clear();
    for m in i.metricas.iter_mut().filter(|m| ["pared_total", "us_por_paso"].contains(&m.id.as_str())) {
        m.medido = None;
    }
    serde_json::to_string(&i).unwrap()
}

#[test]
fn el_escenario_pequeno_produce_todas_las_metricas() {
    let c = con_mundo!();
    let i = poblacion(c, &pequeno());
    assert!(i.metricas.len() >= 40);
    for m in &i.metricas {
        let v = m.medido.unwrap_or_else(|| panic!("«{}» sin medir", m.id));
        assert!(v.is_finite(), "{} = {v}", m.id);
    }
    let ids: std::collections::BTreeSet<_> = i.metricas.iter().map(|m| &m.id).collect();
    assert_eq!(ids.len(), i.metricas.len(), "ids repetidos");
    for id in ["nivel2_pct", "nivel2_mediana_dias", "ruinas_pct", "hambre_pct", "muertes_totales", "primera_lenera_horas", "sin_lenera_pct", "sin_cantera_pct", "pesants_fin", "pesants_200_pct", "viviendas_fin", "edificios_fin", "deficit_trigo_pct", "nutricion_min", "medidor_bajo50_pct", "pared_total", "pasos", "us_por_paso", "determinismo", "invariantes", "invariantes_ciudad", "ciudad_sin_sitio_vivienda_pct", "ciudad_sin_sitio_almacen_pct", "ciudad_sin_sitio_granero_pct", "ciudad_sin_sitio_lenera_pct", "piezas_a", "piezas_fin", "anclas_plazas_pct", "ancla_primera_dia", "mejora_espera_pct", "mejora_espera_max_h", "cuadrilla_retenida_24h", "radio_ciudad_medio", "radio_tope_pct", "puestos_mercado_total"] {
        assert!(i.metricas.iter().any(|m| m.id == id), "falta la métrica «{id}»");
    }
    assert_eq!(i.series.len(), 6);
    // En 4 días nadie llega al nivel 2: solo el histograma de ese día puede estar vacío.
    assert!(i.series.iter().filter(|g| g.id != "dia_nivel2").all(|g| g.series.iter().any(|s| !s.puntos.is_empty())), "algún gráfico sin datos");
    let mapa = i.mapa.as_ref().unwrap();
    for nombre in ["Plazas de nivel 1", "Plazas de nivel 2", "Plazas de nivel 3 o más", "Campamentos de mercenarios"] {
        assert!(mapa.capas.iter().any(|c| c.nombre == nombre), "falta la capa «{nombre}»");
    }
    assert!(mapa.fondo.as_deref().is_some_and(|f| f.starts_with("data:image/bmp;base64,")), "el mapa no lleva fondo");
    let html = informe::renderizar(&i);
    assert!(html.contains("Población, construcción y mantenimiento") && html.contains("Plazas que llegan al nivel 2"));
    assert!(medida(&i, "pasos") > 0.0);
    // El gráfico de pesants va en habitantes enteros, como la métrica (no en milésimas, como antes de la población entera).
    let pesants = i.series.iter().find(|g| g.id == "pesants_por_dia").unwrap();
    let mediana = &pesants.series.iter().find(|s| s.nombre == "Mediana").unwrap().puntos;
    assert_eq!(mediana.last().unwrap().1, medida(&i, "pesants_fin"));
    // Cada plaza lleva su ciudad: al menos el Centro Urbano, las 3 Viviendas y la Granja iniciales.
    assert!(medida(&i, "piezas_a") >= 5.0 && medida(&i, "radio_ciudad_medio") >= 30.0);
}

#[test]
fn los_invariantes_y_el_determinismo_se_cumplen() {
    let c = con_mundo!();
    let i = poblacion(c, &pequeno());
    for id in ["invariantes", "invariantes_ciudad", "determinismo"] {
        let m = i.metricas.iter().find(|m| m.id == id).unwrap();
        assert_eq!(semaforo(m), Semaforo::Verde, "{id}: {:?}", m.medido);
    }
    assert!(i.anomalias.is_empty());
}

#[test]
fn la_ciudad_de_ejemplo_sale_como_svg() {
    let c = con_mundo!();
    let (_, svgs) = poblacion_y_ciudad(c, &EscenarioPoblacion { verificar_determinismo: false, dias: 2, ..pequeno() });
    let svg = &svgs.iter().find(|(n, _)| *n == "ciudad_ejemplo.svg").expect("queda alguna plaza en pie").1;
    assert!(svg.starts_with("<svg") && svg.trim_end().ends_with("</svg>") && svg.contains("<title>centroUrbano #0</title>"));
}

#[test]
fn el_mismo_escenario_da_el_mismo_informe() {
    let c = con_mundo!();
    // Sin la repetición interna del determinismo (cada corrida ya son tres simulaciones en debug): aquí se comparan las dos corridas.
    let e = EscenarioPoblacion { verificar_determinismo: false, ..pequeno() };
    let (a, b) = (poblacion(c, &e), poblacion(c, &e));
    assert_eq!(sin_tiempos(a), sin_tiempos(b));
}

#[test]
fn otra_semilla_coloca_las_plazas_en_otro_sitio_y_las_decisiones_se_funden() {
    let c = con_mundo!();
    let e = EscenarioPoblacion { verificar_determinismo: false, dias: 2, ..pequeno() };
    let mut i = poblacion(c, &e);
    let otra = poblacion(c, &EscenarioPoblacion { semilla: 7, ..e });
    assert_ne!(sin_tiempos(i.clone()), sin_tiempos(otra), "la semilla no cambia nada");
    fundir_decisiones(&mut i, r#"{"resumen":"Leído a mano","decisiones":[{"id":"d1","pregunta":"¿Qué?","opciones":[{"texto":"Esto","recomendada":true}]}]}"#).unwrap();
    assert_eq!((i.resumen.as_str(), i.decisiones.len()), ("Leído a mano", 1));
}
