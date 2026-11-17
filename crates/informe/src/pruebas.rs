use super::*;

fn metrica(min: Option<f64>, max: Option<f64>, medido: Option<f64>, margen: Option<f64>) -> Metrica {
    Metrica {
        id: "m".into(),
        nombre: "M".into(),
        unidad: "u".into(),
        esperado_min: min,
        esperado_max: max,
        medido,
        origen: String::new(),
        nota: None,
        margen_ambar: margen,
    }
}

fn sem(min: Option<f64>, max: Option<f64>, medido: Option<f64>, margen: Option<f64>) -> Semaforo {
    semaforo(&metrica(min, max, medido, margen))
}

#[test]
fn semaforo_fronteras_exactas_son_verdes() {
    assert_eq!(sem(Some(30.0), None, Some(30.0), None), Semaforo::Verde);
    assert_eq!(sem(None, Some(3.0), Some(3.0), None), Semaforo::Verde);
    assert_eq!(sem(Some(5.0), Some(15.0), Some(15.0), None), Semaforo::Verde);
}

#[test]
fn semaforo_limite_nulo_es_sin_limite() {
    assert_eq!(sem(Some(30.0), None, Some(1e9), None), Semaforo::Verde);
    assert_eq!(sem(None, Some(3.0), Some(-1e9), None), Semaforo::Verde);
}

#[test]
fn semaforo_margen_por_defecto_es_el_20_por_ciento_del_limite() {
    // límite 30 → margen 6: 25 queda a 5 (ámbar), 24 queda a 6 (rojo)
    assert_eq!(sem(Some(30.0), None, Some(25.0), None), Semaforo::Ambar);
    assert_eq!(sem(Some(30.0), None, Some(24.0), None), Semaforo::Rojo);
    // por arriba, con el límite superior
    assert_eq!(sem(None, Some(10.0), Some(11.0), None), Semaforo::Ambar);
    assert_eq!(sem(None, Some(10.0), Some(12.5), None), Semaforo::Rojo);
}

#[test]
fn semaforo_limite_cero_usa_margen_de_medio() {
    assert_eq!(sem(None, Some(0.0), Some(0.4), None), Semaforo::Ambar);
    assert_eq!(sem(None, Some(0.0), Some(0.5), None), Semaforo::Rojo);
}

#[test]
fn semaforo_margen_explicito() {
    assert_eq!(sem(Some(30.0), None, Some(20.0), Some(15.0)), Semaforo::Ambar);
    assert_eq!(sem(Some(30.0), None, Some(29.0), Some(0.5)), Semaforo::Rojo);
}

#[test]
fn semaforo_sin_esperado_o_sin_medido() {
    assert_eq!(sem(None, None, Some(4.0), None), Semaforo::Informativa);
    assert_eq!(sem(Some(1.0), None, None, None), Semaforo::SinDato);
    assert_eq!(sem(None, None, None, None), Semaforo::SinDato);
}

/// El SVG no debe llevar números rotos (NaN o infinitos).
fn sin_numeros_rotos(h: &str) -> bool {
    !["NaN", "inf\"", "inf,", "inf<", "inf "].iter().any(|p| h.contains(p))
}

fn ejemplo() -> Informe {
    serde_json::from_str(include_str!("../ejemplo/informe_ejemplo.json")).unwrap()
}

#[test]
fn render_completo_tiene_todas_las_secciones() {
    let h = renderizar(&ejemplo());
    for t in ["Resumen", "Métricas", "Gráficos", "Mapa", "Anomalías", "Decisiones para ti", "Parámetros usados"] {
        assert!(h.contains(&format!("<h2>{t}</h2>")), "falta la sección {t}");
    }
    assert!(!h.contains("<script"));
    assert!(sin_numeros_rotos(&h));
}

#[test]
fn render_escapa_el_html_del_json() {
    let mut i = ejemplo();
    i.sistema = "<script>alert(1)</script>".into();
    i.metricas[0].nombre = "<script>x</script>".into();
    i.decisiones[0].pregunta = "<img src=x onerror=1>".into();
    i.mapa.as_mut().unwrap().capas[0].color = "red\" onload=\"x".into();
    let h = renderizar(&i);
    assert!(!h.contains("<script"));
    assert!(!h.contains("<img"));
    assert!(!h.contains("onload=\"x"));
    assert!(h.contains("&lt;script&gt;"));
}

#[test]
fn informe_minimo_renderiza() {
    let i: Informe = serde_json::from_str(r#"{"sistema":"S","fecha":"2026-10-13"}"#).unwrap();
    let h = renderizar(&i);
    assert!(h.contains("Informe de batch: S"));
    assert!(h.contains("Decisiones para ti"));
}

fn grafico(tipo: TipoGrafico, series: Vec<Serie>) -> Informe {
    let mut i: Informe = serde_json::from_str(r#"{"sistema":"S","fecha":"f"}"#).unwrap();
    i.series.push(Grafico { id: "g".into(), titulo: "G".into(), tipo, eje_x: "x".into(), eje_y: "y".into(), series });
    i
}

#[test]
fn graficos_degenerados_no_dan_nan() {
    for tipo in [TipoGrafico::Linea, TipoGrafico::Barras, TipoGrafico::Histograma] {
        let casos = [
            vec![],
            vec![Serie { nombre: "a".into(), puntos: vec![] }],
            vec![Serie { nombre: "a".into(), puntos: vec![(3.0, 7.0)] }],
            vec![Serie { nombre: "a".into(), puntos: vec![(0.0, 0.0)] }],
            vec![Serie { nombre: "a".into(), puntos: vec![(1.0, 5.0), (2.0, 5.0)] }],
            vec![Serie { nombre: "a".into(), puntos: vec![(1.0, -5.0), (2.0, 5.0)] }],
        ];
        for series in casos {
            let h = renderizar(&grafico(tipo, series));
            assert!(sin_numeros_rotos(&h), "{tipo:?}: {h}");
        }
    }
}

#[test]
fn el_fondo_del_mapa_solo_se_pinta_si_es_un_bmp_valido() {
    let mut i = ejemplo();
    let uri = bmp_uri(2, 1, &[255, 0, 0, 0, 255, 0]).unwrap();
    i.mapa.as_mut().unwrap().fondo = Some(uri.clone());
    assert!(renderizar(&i).contains(&format!("<image href=\"{uri}\"")));
    i.mapa.as_mut().unwrap().fondo = Some("data:image/bmp;base64,AA\" onload=\"x".into());
    assert!(!renderizar(&i).contains("<image"));
    i.mapa.as_mut().unwrap().fondo = Some("javascript:alert(1)".into());
    assert!(!renderizar(&i).contains("<image"));
}

#[test]
fn mapa_con_tamano_invalido_se_omite() {
    let mut i = ejemplo();
    i.mapa.as_mut().unwrap().ancho = 0.0;
    assert!(!renderizar(&i).contains("<h2>Mapa</h2>"));
}

#[test]
fn numeros_en_espanol() {
    assert_eq!(num(31.2), "31,2");
    assert_eq!(num(30.0), "30");
    assert_eq!(num(-0.0), "0");
    assert_eq!(num(0.05), "0,05");
}

#[test]
fn metricas_rojas_primero() {
    let h = renderizar(&ejemplo());
    assert!(h.find("✖ Falla").unwrap() < h.find("✔ Cumple").unwrap());
}

#[test]
fn el_mapa_usa_punto_decimal() {
    let mut i = ejemplo();
    i.mapa.as_mut().unwrap().capas[0].circulos[0].r = 12.5;
    let h = renderizar(&i);
    assert!(h.contains("r=\"12.5\""));
    assert!(!h.contains("r=\"10,"));
}

#[test]
fn las_lineas_del_mapa_salen_como_polilineas_y_se_omiten_si_no_son_validas() {
    let mut i = ejemplo();
    let p = |x: f64, y: f64| Punto { x, y };
    i.mapa.as_mut().unwrap().capas[0].lineas = vec![vec![p(1.0, 2.0), p(30.5, 40.0), p(50.0, 60.0)], vec![p(1.0, 1.0)], vec![p(0.0, f64::NAN), p(2.0, 2.0)]];
    let h = renderizar(&i);
    assert_eq!(h.matches("<polyline class=\"via\"").count(), 1, "solo la línea de dos o más puntos finitos");
    assert!(h.contains("points=\"1.0,2.0 30.5,40.0 50.0,60.0\""));
}
