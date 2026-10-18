//! Del estado final del núcleo y los registros de la simulación a las medidas del informe.

use crate::geo::{Cargado, Geo};
use crate::sim::{self, Corrida};
use base::{Punto, Rng};
use nucleo::Nucleo;
use std::collections::BTreeMap;

const MIN_MS: f64 = 60_000.0;
/// Plazas que el canon estima para un foco de 150 u (Doc 1.9c).
pub const PLAZAS_POR_FOCO: f64 = 45.0;

/// Una plaza al final de la simulación, con lo que las medidas necesitan de ella.
pub struct Plaza {
    pub id: u64,
    pub faccion: u32,
    pub pos: Punto,
    pub nivel: u32,
    /// Radio de la zona al final, en milésimas.
    pub radio: i64,
    pub anillo: usize,
    pub campamento: usize,
    pub parcelas: Vec<u32>,
}

pub fn plazas_finales(n: &Nucleo, geo: &Geo) -> Vec<Plaza> {
    n.estado()
        .asentamientos
        .values()
        .map(|a| {
            let (anillo, campamento) = geo.anillo(a.posicion);
            Plaza { id: a.id, faccion: a.faccion, pos: a.posicion, nivel: a.nivel(), radio: a.radio, anillo, campamento, parcelas: a.parcelas.iter().copied().collect() }
        })
        .collect()
}

pub fn pct(a: usize, b: usize) -> Option<f64> {
    (b > 0).then(|| 100.0 * a as f64 / b as f64)
}

/// Distancia mínima entre plazas (en u) y plazas cuyo centro cae en la zona real de una plaza de otra Facción.
pub fn separacion_y_zonas_ajenas(n: &Nucleo, plazas: &[Plaza]) -> (Option<f64>, usize) {
    let (mut minima, mut dentro) = (i64::MAX, 0);
    for (i, a) in plazas.iter().enumerate() {
        let mut esta_dentro = false;
        for (j, b) in plazas.iter().enumerate() {
            if i < j {
                minima = minima.min(a.pos.dist(b.pos));
            }
            // Solo las discos que contienen el centro merecen la consulta exacta (con el recorte de las fronteras).
            if i != j && a.faccion != b.faccion && a.pos.dist2(b.pos) <= (b.radio as i128).pow(2) && n.en_zona(b.id, a.pos) {
                esta_dentro = true;
            }
        }
        dentro += esta_dentro as usize;
    }
    ((minima != i64::MAX).then(|| minima as f64 / 1000.0), dentro)
}

pub fn cerca_de_campamento(plazas: &[Plaza], geo: &Geo, exclusion: i64) -> usize {
    plazas.iter().filter(|p| geo.campamentos.iter().any(|c| p.pos.dist2(*c) < (exclusion as i128).pow(2))).count()
}

/// (mayor fracción de una masa que ocupa un asentamiento, en %; masas con todas sus parcelas ocupadas)
pub fn cuota_y_masas_llenas(n: &Nucleo, plazas: &[Plaza], geo: &Geo) -> (Option<f64>, usize) {
    let mut mayor: Option<f64> = None;
    for p in plazas {
        let mut por_masa: BTreeMap<usize, usize> = BTreeMap::new();
        p.parcelas.iter().for_each(|&i| *por_masa.entry(geo.parcelas[i as usize].1).or_insert(0) += 1);
        for (masa, k) in por_masa.into_iter().filter(|&(m, _)| geo.total_por_masa[m] >= 2) {
            let f = 100.0 * k as f64 / geo.total_por_masa[masa] as f64;
            mayor = Some(mayor.map_or(f, |m| m.max(f)));
        }
    }
    let mut ocupadas = vec![0usize; geo.total_por_masa.len()];
    n.estado().parcelas_ocupadas.keys().for_each(|&i| ocupadas[geo.parcelas[i as usize].1] += 1);
    let llenas = ocupadas.iter().zip(&geo.total_por_masa).filter(|(o, t)| **t > 0 && o == t).count();
    (mayor, llenas)
}

/// (plazas sin piedra, plazas sin parcela de bosque) en el disco del techo de su nivel actual.
pub fn sin_recursos_en_zona_maxima(n: &Nucleo, plazas: &[Plaza], geo: &Geo) -> (usize, usize) {
    let reglas = &n.estado().reglas.territorio;
    let dentro = |p: &Plaza, c: Punto, r: i64| p.pos.dist2(c) <= (r as i128).pow(2);
    let (mut sin_piedra, mut sin_bosque) = (0, 0);
    for p in plazas {
        let r = reglas.techo(p.nivel);
        sin_piedra += !geo.piedra.iter().any(|&c| dentro(p, c, r)) as usize;
        sin_bosque += !geo.parcelas.iter().any(|&(c, _)| dentro(p, c, r)) as usize;
    }
    (sin_piedra, sin_bosque)
}

/// Contactos de frontera por plaza (media y máximo) a partir de los pares que ya se han tocado.
pub fn contactos(n: &Nucleo, plazas: &[Plaza]) -> (Option<f64>, usize) {
    let mut por_plaza: BTreeMap<u64, usize> = BTreeMap::new();
    for &(a, b) in &n.estado().contactos {
        *por_plaza.entry(a).or_insert(0) += 1;
        *por_plaza.entry(b).or_insert(0) += 1;
    }
    let total: usize = por_plaza.values().sum();
    ((!plazas.is_empty()).then(|| total as f64 / plazas.len() as f64), por_plaza.values().copied().max().unwrap_or(0))
}

/// Área de un polígono (fórmula del cordón).
fn area(poligono: &[Punto]) -> f64 {
    let n = poligono.len();
    let suma: f64 = (0..n).map(|i| (poligono[i].x as f64) * (poligono[(i + 1) % n].y as f64) - (poligono[(i + 1) % n].x as f64) * (poligono[i].y as f64)).sum();
    suma.abs() / 2.0
}

/// % medio y máximo del área de la zona perdido por los recortes, frente al polígono de 48 lados sin recortar.
pub fn area_perdida(n: &Nucleo, plazas: &[Plaza]) -> (Option<f64>, Option<f64>) {
    let completo = |r: f64| 0.5 * 48.0 * r * r * (std::f64::consts::TAU / 48.0).sin();
    let perdidas: Vec<f64> = plazas
        .iter()
        .filter(|p| p.radio > 0)
        .map(|p| (100.0 * (1.0 - area(&n.poligono_zona(p.id)) / completo(p.radio as f64))).max(0.0))
        .collect();
    let media = (!perdidas.is_empty()).then(|| perdidas.iter().sum::<f64>() / perdidas.len() as f64);
    (media, perdidas.iter().copied().reduce(f64::max))
}

/// Por tipo (0 estaño, 1 oro) y anillo: % de nodos cubiertos por la zona real de alguna plaza; y nodos por anillo.
pub fn cobertura_de_metales(n: &Nucleo, plazas: &[Plaza], geo: &Geo) -> ([[Option<f64>; 3]; 2], [usize; 3]) {
    let (mut total, mut cubiertos, mut por_anillo) = ([[0usize; 3]; 2], [[0usize; 3]; 2], [0usize; 3]);
    for m in &geo.metales {
        total[m.tipo][m.anillo] += 1;
        por_anillo[m.anillo] += 1;
        let cubierto = plazas.iter().any(|p| m.p.dist2(p.pos) <= (p.radio as i128).pow(2) && n.en_zona(p.id, m.p));
        cubiertos[m.tipo][m.anillo] += cubierto as usize;
    }
    let pc = |t: usize, a: usize| pct(cubiertos[t][a], total[t][a]);
    ([[pc(0, 0), pc(0, 1), pc(0, 2)], [pc(1, 0), pc(1, 1), pc(1, 2)]], por_anillo)
}

/// Lo que mide el mundo antes de fundar nada: de los puntos fundables, cuántos nacerían verdes, por anillo.
pub struct Malla {
    pub fundables: [usize; 3],
    pub verdes: [usize; 3],
}

pub const PUNTOS_POR_ANILLO: usize = 2000;
const INTENTOS_MALLA: usize = 150_000;

pub fn sondear_malla(c: &Cargado, geo: &Geo, semilla: u64) -> Malla {
    let n = sim::nucleo_nuevo(c, semilla);
    let mut rng = Rng::derivado(semilla, "batch-territorio/malla");
    let mut m = Malla { fundables: [0; 3], verdes: [0; 3] };
    for _ in 0..INTENTOS_MALLA {
        if m.fundables.iter().all(|&k| k >= PUNTOS_POR_ANILLO) {
            break;
        }
        let p = Punto::new(rng.por_debajo(geo.ancho as u64) as i64, rng.por_debajo(geo.alto as u64) as i64);
        let anillo = geo.anillo(p).0;
        // Tierra firme: lo que el mundo deja pisar; el resto lo descarta la viabilidad.
        if m.fundables[anillo] >= PUNTOS_POR_ANILLO || !c.mundo.transitable_en(p.x, p.y) {
            continue;
        }
        let v = n.viabilidad(p, 1).expect("el núcleo tiene mundo");
        if v.motivo.is_none() {
            m.fundables[anillo] += 1;
            m.verdes[anillo] += (v.color == territorio::Viabilidad::Verde) as usize;
        }
    }
    m
}

/// Medias de los tiempos de la primera plaza (min) y plazas nacidas en verde, de los registros.
pub fn tiempo_primera_plaza_min(c: &Corrida) -> Option<f64> {
    let t: Vec<f64> = c.reg.fundaciones.iter().filter(|f| f.primera).map(|f| f.tras_ms as f64 / MIN_MS).collect();
    (!t.is_empty()).then(|| t.iter().sum::<f64>() / t.len() as f64)
}
