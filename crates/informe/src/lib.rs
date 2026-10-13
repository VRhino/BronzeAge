//! Informe de batch de un sistema (fase 6): del JSON medido a una página HTML autónoma.
//!
//! Uso: `informe <entrada.json> <salida.html>` o, desde código, `renderizar(&informe)`.

mod graficos;
mod render;

use serde::{Deserialize, Serialize};

pub use render::renderizar;

/// Informe completo: lo que la simulación midió frente a lo esperado.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Informe {
    pub sistema: String,
    pub fecha: String,
    #[serde(default)]
    pub mundo_hash: String,
    #[serde(default)]
    pub semilla: u64,
    #[serde(default)]
    pub dias_simulados: f64,
    #[serde(default)]
    pub resumen: String,
    #[serde(default)]
    pub parametros: Vec<Parametro>,
    #[serde(default)]
    pub metricas: Vec<Metrica>,
    #[serde(default)]
    pub series: Vec<Grafico>,
    #[serde(default)]
    pub mapa: Option<Mapa>,
    #[serde(default)]
    pub anomalias: Vec<Anomalia>,
    #[serde(default)]
    pub decisiones: Vec<Decision>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Parametro {
    pub nombre: String,
    pub valor: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metrica {
    pub id: String,
    pub nombre: String,
    #[serde(default)]
    pub unidad: String,
    #[serde(default)]
    pub esperado_min: Option<f64>,
    #[serde(default)]
    pub esperado_max: Option<f64>,
    #[serde(default)]
    pub medido: Option<f64>,
    #[serde(default)]
    pub origen: String,
    #[serde(default)]
    pub nota: Option<String>,
    #[serde(default)]
    pub margen_ambar: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TipoGrafico {
    Linea,
    Barras,
    Histograma,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Grafico {
    #[serde(default)]
    pub id: String,
    pub titulo: String,
    pub tipo: TipoGrafico,
    #[serde(default)]
    pub eje_x: String,
    #[serde(default)]
    pub eje_y: String,
    #[serde(default)]
    pub series: Vec<Serie>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Serie {
    pub nombre: String,
    #[serde(default)]
    pub puntos: Vec<(f64, f64)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Mapa {
    pub ancho: f64,
    pub alto: f64,
    #[serde(default)]
    pub capas: Vec<Capa>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Capa {
    pub nombre: String,
    #[serde(default = "color_por_defecto")]
    pub color: String,
    #[serde(default)]
    pub circulos: Vec<Circulo>,
    #[serde(default)]
    pub puntos: Vec<Punto>,
}

fn color_por_defecto() -> String {
    "#3b82f6".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Circulo {
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Punto {
    pub x: f64,
    pub y: f64,
}

/// Qué clase de problema es una anomalía: del código, del diseño o de la medición.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Causa {
    Codigo,
    Diseno,
    Medicion,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Anomalia {
    pub titulo: String,
    #[serde(default)]
    pub detalle: String,
    #[serde(default)]
    pub causa: Option<Causa>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Decision {
    pub id: String,
    pub pregunta: String,
    #[serde(default)]
    pub contexto: String,
    #[serde(default)]
    pub opciones: Vec<Opcion>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Opcion {
    pub texto: String,
    #[serde(default)]
    pub recomendada: bool,
    #[serde(default)]
    pub efecto: Option<String>,
}

/// Estado de una métrica frente a lo esperado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Semaforo {
    Verde,
    Ambar,
    Rojo,
    Informativa,
    SinDato,
}

/// Verde dentro del rango; ámbar si se queda fuera por menos que el margen
/// (por defecto el 20 % del límite incumplido, o 0,5 si ese límite es 0); rojo si no.
pub fn semaforo(m: &Metrica) -> Semaforo {
    let Some(v) = m.medido else {
        return Semaforo::SinDato;
    };
    if m.esperado_min.is_none() && m.esperado_max.is_none() {
        return Semaforo::Informativa;
    }
    // (distancia al rango, límite incumplido)
    let (dist, limite) = match (m.esperado_min, m.esperado_max) {
        (Some(lo), _) if v < lo => (lo - v, lo),
        (_, Some(hi)) if v > hi => (v - hi, hi),
        _ => return Semaforo::Verde,
    };
    let margen = m
        .margen_ambar
        .unwrap_or(if limite == 0.0 { 0.5 } else { 0.2 * limite.abs() });
    if dist < margen {
        Semaforo::Ambar
    } else {
        Semaforo::Rojo
    }
}

/// Escapa el HTML de cualquier texto que venga del JSON.
pub(crate) fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            _ => o.push(c),
        }
    }
    o
}

/// Número en español: coma decimal, hasta 3 decimales, sin ceros sobrantes.
pub(crate) fn num(v: f64) -> String {
    if !v.is_finite() {
        return "—".to_string();
    }
    let mut s = format!("{v:.3}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s == "-0" {
        s = "0".to_string();
    }
    s.replace('.', ",")
}

#[cfg(test)]
mod pruebas;
