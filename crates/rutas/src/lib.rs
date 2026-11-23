//! Rutas: A* sobre el grafo de navegación del artefacto de mundo (la ÚNICA autoridad de rutas) y conversión del camino
//! en un `base::Plan` con el instante de paso por cada vértice (Movimiento_Ejercitos §4.2-§4.3).
//!
//! Todo en enteros y determinista: mismo grafo y misma petición → el mismo camino bit a bit.
//!
//! # Unidades del coste
//! `AristaGrafo.coste_milesimas` es el coste TOTAL de recorrer la arista, medido en «espaciados»: 1000 = un espaciado en
//! llano (la diagonal en llano ya vale 1414, por la √2). Por eso el tiempo de una arista es
//! `coste × espaciado / 1000 / velocidad`, no `longitud × coste`. Los tramos de enganche (del punto real al nodo más
//! cercano y del último nodo al destino real) cuestan `longitud × 1000 / espaciado`, es decir, terreno llano.
//!
//! # Enganche
//! El origen y el destino se enganchan a los (hasta [`ENGANCHES`]) nodos más cercanos cuyo tramo recto es libre. Con un
//! `Mundo` (`desde_mundo`) «libre» significa transitable a lo largo del tramo (un nodo al otro lado de un río no vale) y
//! origen/destino deben ser transitables; con solo el grafo (`desde_grafo`) no se puede saber qué hay entre medias, así
//! que se engancha al único nodo más cercano. Si origen y destino están a
//! menos de un espaciado y el tramo es libre, la ruta es la recta directa.
//!
//! # Suavizado (`Opciones::suavizar`)
//! Con la capa `coste` del mundo, la ruta del A* se estira desde el origen: desde cada vértice salta al más lejano (hasta
//! [`MAX_SALTO`] vértices) al que se llega en recta libre, sin saltarse una arista de vado y sin que la recta cueste más que
//! el tramo que sustituye, medidos los dos con la capa. Los tramos nuevos llevan el coste de la capa; los que quedan, el
//! suyo. Lo piden las columnas (marchar, perseguir); las caravanas no, porque van por la red de caminos.
//!
//! # Estorbos (`Opciones::estorbos`)
//! Discos que la ruta rodea: las plazas, para las columnas (Tamaños y distancias del mapa, bloque E, 2026-11-23). Un tramo que se mete en uno
//! (que pasa a menos de su radio del centro, con [`HOLGURA_ESTORBO`] de margen) cuesta en la búsqueda `factor` veces lo suyo: es un paso muy caro,
//! no cerrado, así que solo se cruza por dentro si no hay otro camino. Vale para las aristas, los enganches, la recta directa y el suavizado. Un
//! disco que tiene dentro el origen o el destino no cuenta. Como el vado, no entra en los costes del camino: solo lo desvía.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;

use base::{Instante, Mil, Plan, Punto, Vertice, MS_POR_MINUTO};
use contrato::v1::GrafoNavegacion;
use mundo::{Mundo, TAMANO_CELDA};
use rayon::prelude::*;

/// Cuántos nodos de enganche se prueban en cada extremo.
pub const ENGANCHES: usize = 4;
/// Cuántos vértices puede saltarse de una vez el suavizado (acota el coste de estirar rutas largas).
pub const MAX_SALTO: usize = 64;
/// Desplazamiento de la heurística en punto fijo.
const DESPLAZ: u32 = 20;
const SIN_PADRE: u32 = u32::MAX;
/// Uno, en ppm.
const UNO: i64 = 1_000_000;
/// Lo que un tramo puede rozar un estorbo sin contar como dentro (milésimas): el punto de salida de una plaza, en su contorno, puede quedar una
/// milésima dentro por el redondeo.
pub const HOLGURA_ESTORBO: i64 = 100;

/// Clave de una arista de la red de caminos: los dos extremos **redondeados a unidad entera** y ordenados (la de
/// `comercio::red_caminos::Arista`, que este crate no conoce).
pub type ClaveArista = ((i64, i64), (i64, i64));

fn entero(v: i64) -> i64 {
    (v + 500).div_euclid(1000)
}

/// `claveArista(a, b)`: la misma clave sea cual sea el sentido.
pub fn clave_arista(a: Punto, b: Punto) -> ClaveArista {
    let (p, q) = ((entero(a.x), entero(a.y)), (entero(b.x), entero(b.y)));
    if p <= q {
        (p, q)
    } else {
        (q, p)
    }
}

/// La atracción de la red de caminos (Comercio_Caravanas §7.1): el coste de una arista se multiplica por `factorAtraccion(peso)`, con
/// `factorAtraccion(p) = p > 0 ? max(minimo, 1 − por_peso × p) : 1`. Solo desvía la ruta; no entra en los tiempos del plan.
#[derive(Clone, Debug, Default)]
pub struct Atraccion {
    /// Peso de cada arista (cuántas rutas vigentes la pisan); las de peso 0 no aparecen.
    pub pesos: HashMap<ClaveArista, u32>,
    /// `NAVEGACION.atraccionPorPeso` en ppm.
    pub por_peso: i64,
    /// `NAVEGACION.atraccionMin` en ppm.
    pub minimo: i64,
}

/// Los discos que una ruta rodea (ver «Estorbos» arriba).
#[derive(Clone, Debug, Default)]
pub struct Estorbos {
    /// Centro y radio de cada disco.
    pub discos: Vec<(Punto, Mil)>,
    /// Cuántas veces su coste cuesta en la búsqueda un tramo que se mete en uno (1 o menos: no estorban).
    pub factor: i64,
}

/// ¿Pasa el tramo `a → b` a menos de `r − HOLGURA_ESTORBO` de `c`? El punto del tramo más cercano a `c` se redondea a la milésima.
fn tramo_en_disco(a: Punto, b: Punto, c: Punto, r: Mil) -> bool {
    // Descarte rápido: el disco fuera de la caja del tramo.
    if c.x + r < a.x.min(b.x) || c.x - r > a.x.max(b.x) || c.y + r < a.y.min(b.y) || c.y - r > a.y.max(b.y) {
        return false;
    }
    let lim = (r - HOLGURA_ESTORBO).max(0) as i128;
    let (dx, dy) = ((b.x - a.x) as i128, (b.y - a.y) as i128);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0 { 0 } else { ((c.x - a.x) as i128 * dx + (c.y - a.y) as i128 * dy).clamp(0, l2) };
    let (qx, qy) = if l2 == 0 { (a.x as i128, a.y as i128) } else { (a.x as i128 + dx * t / l2, a.y as i128 + dy * t / l2) };
    let (ex, ey) = (qx - c.x as i128, qy - c.y as i128);
    ex * ex + ey * ey < lim * lim
}

impl Atraccion {
    /// El factor (ppm) de una arista de peso `p`.
    pub fn factor(&self, p: u32) -> i64 {
        if p > 0 {
            (UNO - self.por_peso * p as i64).max(self.minimo)
        } else {
            UNO
        }
    }
    /// El menor factor que puede aparecer (para que la heurística siga siendo admisible).
    fn factor_minimo(&self) -> i64 {
        self.pesos.values().map(|&p| self.factor(p)).min().unwrap_or(UNO).min(UNO)
    }
}

/// Opciones de una petición de ruta.
#[derive(Clone, Debug, Default)]
pub struct Opciones {
    /// Coste añadido (en milésimas de espaciado, como `coste_milesimas`) a cada arista vado en la búsqueda. No entra en los
    /// tiempos del plan: solo desvía la ruta.
    pub penalizacion_vado: i64,
    /// Ids de nodo (`NodoGrafo.id`) por los que no se puede pasar ni engancharse.
    pub bloqueados: Vec<u32>,
    /// La red de caminos que atrae a la ruta (las caravanas comerciales); `None` o sin pesos = sin atracción.
    pub atraccion: Option<Arc<Atraccion>>,
    /// Estirar la ruta en rectas que no cuestan más (ver «Suavizado» arriba). Solo hace algo si el mundo trae la capa
    /// `coste`; sin ella la ruta es la del A* tal cual.
    pub suavizar: bool,
    /// Discos que la ruta rodea (las plazas, para las columnas); `None` = nada estorba.
    pub estorbos: Option<Arc<Estorbos>>,
}

impl Opciones {
    /// Las de una columna que marcha o persigue: suavizada y sin más (las caravanas van por la red de caminos y no suavizan).
    pub fn de_columna() -> Self {
        Self { suavizar: true, ..Self::default() }
    }
}

/// Una petición de un lote.
#[derive(Clone, Debug)]
pub struct Peticion {
    pub origen: Punto,
    pub destino: Punto,
    pub opciones: Opciones,
}

/// Camino resultante: del origen real al destino real pasando por nodos del grafo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Camino {
    /// Origen, nodos y destino (sin repetidos consecutivos).
    pub puntos: Vec<Punto>,
    /// Coste real (sin penalizaciones) de cada tramo `puntos[i] → puntos[i+1]`, en milésimas de espaciado.
    pub costes: Vec<i64>,
    /// Ids de los nodos recorridos, en orden.
    pub nodos: Vec<u32>,
    /// Espaciado del grafo (milésimas) con el que se midieron los costes.
    pub espaciado: i64,
}

impl Camino {
    /// Coste total en milésimas de espaciado.
    pub fn coste_total(&self) -> i64 {
        self.costes.iter().sum()
    }

    /// Plan de marcha: `salida` es el instante en el primer punto; `velocidad` en milésimas de unidad por segundo.
    pub fn a_plan(&self, salida: Instante, velocidad: Mil, generacion: u32) -> Plan {
        a_plan(self, salida, velocidad, generacion)
    }

    /// El camino hasta donde entra por primera vez en el disco de `centro` y `radio` (el contorno de un lugar del mapa: Tamaños y distancias del
    /// mapa, bloque C, 2026-11-23): el último tramo se corta en el punto del contorno por donde llega, con su coste en proporción. Un origen ya dentro
    /// del disco (a `radio` o menos) deja solo el origen. Sin entrar en el disco, el camino tal cual. Los `nodos` se quedan con los de antes del corte.
    pub fn hasta_el_contorno(mut self, centro: Punto, radio: Mil) -> Camino {
        let dentro = |p: Punto| p.dist2(centro) <= (radio as i128) * (radio as i128);
        let Some(&origen) = self.puntos.first() else { return self };
        if dentro(origen) {
            return Camino { puntos: vec![origen], costes: vec![], nodos: vec![], espaciado: self.espaciado };
        }
        let Some(k) = (0..self.puntos.len().saturating_sub(1)).find(|&k| dentro(self.puntos[k + 1])) else { return self };
        let (a, b) = (self.puntos[k], self.puntos[k + 1]);
        // Bisección sobre la fracción del tramo, en millonésimas: `a` queda fuera y `b` dentro.
        let en = |f: i64| Punto::new(a.x + ((b.x - a.x) as i128 * f as i128 / 1_000_000) as i64, a.y + ((b.y - a.y) as i128 * f as i128 / 1_000_000) as i64);
        let (mut fuera, mut entra) = (0i64, 1_000_000i64);
        while entra - fuera > 1 {
            let m = (fuera + entra) / 2;
            if dentro(en(m)) { entra = m } else { fuera = m }
        }
        let corte = en(entra);
        let coste = (self.costes.get(k).copied().unwrap_or(0) as i128 * entra as i128 / 1_000_000) as i64;
        self.puntos.truncate(k + 1);
        self.costes.truncate(k);
        if corte != a {
            self.puntos.push(corte);
            self.costes.push(coste);
        }
        self.nodos.truncate(k);
        self
    }
}

/// Convierte un camino en plan. El instante de cada vértice es `salida + redondeo(coste_acumulado × espaciado / velocidad)`
/// (ms; mitad hacia arriba): el redondeo es sobre el acumulado, así que no se acumula error y los instantes son monótonos.
/// `velocidad <= 0` se trata como 1 (nunca divide por cero).
pub fn a_plan(camino: &Camino, salida: Instante, velocidad: Mil, generacion: u32) -> Plan {
    let v = velocidad.max(1) as i128;
    let mut acum: i128 = 0;
    let mut vertices = Vec::with_capacity(camino.puntos.len());
    for (i, &p) in camino.puntos.iter().enumerate() {
        if i > 0 {
            acum += camino.costes[i - 1] as i128;
        }
        let ms = (acum * camino.espaciado as i128 + v / 2) / v;
        vertices.push(Vertice { p, en: salida + ms as Instante });
    }
    Plan::nuevo(generacion, vertices)
}

/// Plan sobre una polilínea con el coste de cada tramo dado (`costes[i]` es el de `puntos[i] → puntos[i+1]`, en milésimas de
/// espaciado) y la velocidad en **milésimas de unidad por minuto** (la de las caravanas: 24.000 = 24 unidades por minuto). El instante
/// de cada vértice es `salida + redondeo(coste_acumulado × espaciado × 60 / velocidad)` ms. `velocidad <= 0` se trata como 1.
pub fn a_plan_por_minuto(puntos: &[Punto], costes: &[i64], espaciado: i64, salida: Instante, velocidad: Mil, generacion: u32) -> Plan {
    debug_assert!(costes.len() + 1 >= puntos.len(), "falta el coste de algún tramo");
    let v = velocidad.max(1) as i128;
    let por_milesima = (MS_POR_MINUTO / 1000) as i128;
    let mut acum: i128 = 0;
    let mut vertices = Vec::with_capacity(puntos.len());
    for (i, &p) in puntos.iter().enumerate() {
        if i > 0 {
            acum += costes.get(i - 1).copied().unwrap_or(0) as i128;
        }
        let ms = (acum * espaciado as i128 * por_milesima + v / 2) / v;
        vertices.push(Vertice { p, en: salida + ms as Instante });
    }
    Plan::nuevo(generacion, vertices)
}

/// Memoria de trabajo reutilizable de una búsqueda (una por hilo): evita reservar y poner a cero 53.000 entradas por ruta.
pub struct Escritorio {
    g: Vec<i64>,
    padre: Vec<u32>,
    padre_arista: Vec<u32>,
    /// `visto[i] == marca` ⇒ `g[i]` es de esta búsqueda.
    visto: Vec<u32>,
    /// `bloqueado[i] == marca` ⇒ nodo bloqueado en esta búsqueda.
    bloqueado: Vec<u32>,
    /// `cerca[i] == marca` ⇒ el nodo está lo bastante cerca de un estorbo para que una arista suya pueda meterse en él.
    cerca: Vec<u32>,
    /// El estorbo (índice en los de esta búsqueda) del que está cerca, si `cerca` es de esta búsqueda. Las plazas están a 60 u unas de otras: un nodo
    /// no está cerca de dos.
    disco: Vec<u32>,
    marca: u32,
    cola: BinaryHeap<Reverse<(i64, i64, u32, bool)>>,
}

impl Escritorio {
    pub fn nuevo(nodos: usize) -> Self {
        Self {
            g: vec![0; nodos],
            padre: vec![SIN_PADRE; nodos],
            padre_arista: vec![0; nodos],
            visto: vec![0; nodos],
            bloqueado: vec![0; nodos],
            cerca: vec![0; nodos],
            disco: vec![0; nodos],
            marca: 0,
            cola: BinaryHeap::new(),
        }
    }
}

/// Grafo compacto + índice espacial + (opcional) la transitabilidad del mundo para validar los enganches.
pub struct Rutas {
    ids: Vec<u32>,
    indice_de: HashMap<u32, u32>,
    pos: Vec<Punto>,
    // CSR: aristas de `i` en `inicio[i]..inicio[i+1]`
    inicio: Vec<u32>,
    destino: Vec<u32>,
    coste: Vec<u32>,
    vado: Vec<bool>,
    espaciado: i64,
    /// La arista más larga (milésimas): un tramo que se mete en un disco tiene los dos extremos a `radio + arista_max` o menos de su centro.
    arista_max: i64,
    /// Coste mínimo por milésima de longitud, en punto fijo `<< DESPLAZ` (suelo): heurística admisible.
    ratio_min: i64,
    // Índice espacial (CSR por celdas)
    min: Punto,
    lado: i64,
    cols: i64,
    filas: i64,
    celda_inicio: Vec<u32>,
    celda_nodos: Vec<u32>,
    transitable: Option<Transitable>,
}

/// Copia a bits de la transitabilidad del mundo (una celda = `TAMANO_CELDA`) y, si la trae, su capa de coste. Con los puntos de los
/// vados: el cauce va cerrado entero y se pisa solo a [`mundo::RADIO_VADO`] del punto de un vado (2026-11-18).
struct Transitable {
    cols: i64,
    filas: i64,
    bits: Vec<u64>,
    vados: Vec<Punto>,
    /// Coste por celda en milésimas (1000 = llano, 0 = no transitable); vacío si el mundo no trae la capa.
    coste: Vec<u16>,
}

impl Transitable {
    fn de(m: &Mundo) -> Self {
        let (c, f) = m.rejilla();
        let mut bits = vec![0u64; (c * f).div_ceil(64)];
        let t = TAMANO_CELDA as i64;
        let mut coste = if m.tiene_coste() { vec![0u16; c * f] } else { Vec::new() };
        for fila in 0..f {
            for col in 0..c {
                let (x, y, i) = (col as i64 * t + t / 2, fila as i64 * t + t / 2, fila * c + col);
                if m.transitable_en(x, y) {
                    bits[i >> 6] |= 1 << (i & 63);
                }
                if !coste.is_empty() {
                    coste[i] = m.coste_en(x, y).unwrap_or(0);
                }
            }
        }
        let vados = m.rios().iter().flat_map(|r| &r.vados).map(|v| Punto::new(v.x, v.y)).collect();
        Self { cols: c as i64, filas: f as i64, bits, vados, coste }
    }

    /// Para las pruebas: `coste(col, fila)` por celda, en milésimas; 0 = no transitable.
    #[cfg(test)]
    fn de_celdas(cols: i64, filas: i64, coste: impl Fn(i64, i64) -> u16) -> Self {
        let n = (cols * filas) as usize;
        let (mut bits, mut c) = (vec![0u64; n.div_ceil(64)], vec![0u16; n]);
        for i in 0..n {
            c[i] = coste(i as i64 % cols, i as i64 / cols);
            if c[i] > 0 {
                bits[i >> 6] |= 1 << (i & 63);
            }
        }
        Self { cols, filas, bits, vados: Vec::new(), coste: c }
    }

    fn tiene_coste(&self) -> bool {
        !self.coste.is_empty()
    }

    /// Coste de la celda de `p` en milésimas; 0 fuera del mundo, en lo que no se pisa o sin capa de coste.
    fn coste_en(&self, p: Punto) -> u16 {
        if p.x < 0 || p.y < 0 || self.coste.is_empty() {
            return 0;
        }
        let (c, f) = (p.x / TAMANO_CELDA as i64, p.y / TAMANO_CELDA as i64);
        if c >= self.cols || f >= self.filas {
            return 0;
        }
        self.coste[(f * self.cols + c) as usize]
    }

    /// Coste de la recta `a → b` con la capa, en milésimas de espaciado (como `coste_milesimas`): la media de la capa en el
    /// centro de cada trozo de media celda × largo / espaciado, redondeada al más cercano. `None` si pisa una celda no
    /// transitable.
    fn coste_recto(&self, a: Punto, b: Punto, espaciado: i64) -> Option<i64> {
        let largo = a.dist(b);
        let n = largo / (TAMANO_CELDA as i64 / 2) + 1;
        let mut suma: i64 = 0;
        for k in 0..n {
            let q = Punto::new(a.x + (b.x - a.x) * (2 * k + 1) / (2 * n), a.y + (b.y - a.y) * (2 * k + 1) / (2 * n));
            match self.coste_en(q) {
                0 => return None,
                c => suma += c as i64,
            }
        }
        let den = n as i128 * espaciado as i128;
        Some(((suma as i128 * largo as i128 + den / 2) / den) as i64)
    }
    fn en(&self, p: Punto) -> bool {
        if p.x < 0 || p.y < 0 {
            return false;
        }
        let (c, f) = (p.x / TAMANO_CELDA as i64, p.y / TAMANO_CELDA as i64);
        if c >= self.cols || f >= self.filas {
            return false;
        }
        let i = (f * self.cols + c) as usize;
        self.bits[i >> 6] >> (i & 63) & 1 == 1 || self.vados.iter().any(|&v| v.dist2(p) <= (mundo::RADIO_VADO as i128).pow(2))
    }
    /// Segmento transitable, muestreado cada media celda (incluye los extremos).
    fn libre(&self, a: Punto, b: Punto) -> bool {
        let paso = TAMANO_CELDA as i64 / 2;
        let n = a.dist(b) / paso + 1;
        (0..=n).all(|i| self.en(Punto::new(a.x + (b.x - a.x) * i / n, a.y + (b.y - a.y) * i / n)))
    }
}

impl Rutas {
    /// Construye desde el grafo solo (todo enganche recto se considera libre).
    pub fn desde_grafo(g: &GrafoNavegacion) -> Rutas {
        Self::construir(g, None)
    }

    /// Construye desde un mundo cargado: además valida enganches y extremos con su raster de transitabilidad.
    pub fn desde_mundo(m: &Mundo) -> Rutas {
        Self::construir(m.navegacion(), Some(Transitable::de(m)))
    }

    /// Para las pruebas: el grafo con un terreno sintético de `cols × filas` celdas (`coste` por celda; 0 = no transitable).
    #[cfg(test)]
    fn con_terreno(g: &GrafoNavegacion, cols: i64, filas: i64, coste: impl Fn(i64, i64) -> u16) -> Rutas {
        Self::construir(g, Some(Transitable::de_celdas(cols, filas, coste)))
    }

    fn construir(g: &GrafoNavegacion, transitable: Option<Transitable>) -> Rutas {
        let n = g.nodos.len();
        let ids: Vec<u32> = g.nodos.iter().map(|x| x.id).collect();
        let indice_de: HashMap<u32, u32> = ids.iter().enumerate().map(|(i, &id)| (id, i as u32)).collect();
        let pos: Vec<Punto> =
            g.nodos.iter().map(|x| x.posicion.map_or(Punto::default(), |p| Punto::new(p.x, p.y))).collect();

        // Aristas válidas (ambos nodos existen), no dirigidas → dos entradas.
        let aristas: Vec<(u32, u32, u32, bool)> = g
            .aristas
            .iter()
            .filter_map(|e| Some((*indice_de.get(&e.a)?, *indice_de.get(&e.b)?, e.coste_milesimas, e.vado)))
            .collect();

        // Espaciado: el declarado, o la arista más corta.
        let espaciado = if g.espaciado_min > 0 {
            g.espaciado_min as i64
        } else {
            aristas.iter().map(|&(a, b, ..)| pos[a as usize].dist(pos[b as usize])).filter(|&l| l > 0).min().unwrap_or(1000)
        };

        // CSR (conteo → prefijos → relleno). El orden de relleno sigue el de `g.aristas`: determinista.
        let mut inicio = vec![0u32; n + 1];
        for &(a, b, ..) in &aristas {
            inicio[a as usize + 1] += 1;
            inicio[b as usize + 1] += 1;
        }
        for i in 0..n {
            inicio[i + 1] += inicio[i];
        }
        let total = inicio[n] as usize;
        let (mut destino, mut coste, mut vado) = (vec![0u32; total], vec![0u32; total], vec![false; total]);
        let mut cursor = inicio.clone();
        for &(a, b, c, v) in &aristas {
            for (de, a_) in [(a, b), (b, a)] {
                let k = cursor[de as usize] as usize;
                cursor[de as usize] += 1;
                (destino[k], coste[k], vado[k]) = (a_, c, v);
            }
        }

        // Heurística: min sobre aristas de coste/longitud, y el llano de los enganches (1000/espaciado); suelo en punto fijo.
        let mut ratio_min = (1000i128 << DESPLAZ) / espaciado as i128;
        for &(a, b, c, _) in &aristas {
            let l = pos[a as usize].dist(pos[b as usize]) as i128 + 1; // +1: la longitud real es >= el suelo, el ratio nunca sube
            ratio_min = ratio_min.min(((c as i128) << DESPLAZ) / l);
        }

        let arista_max = aristas.iter().map(|&(a, b, ..)| pos[a as usize].dist(pos[b as usize])).max().unwrap_or(0);

        // Índice espacial: celdas de 2 espaciados.
        let (mut min, mut max) = (Punto::new(i64::MAX, i64::MAX), Punto::new(i64::MIN, i64::MIN));
        for p in &pos {
            (min.x, min.y, max.x, max.y) = (min.x.min(p.x), min.y.min(p.y), max.x.max(p.x), max.y.max(p.y));
        }
        if n == 0 {
            (min, max) = (Punto::default(), Punto::default());
        }
        let lado = (2 * espaciado).max(1);
        let (cols, filas) = ((max.x - min.x) / lado + 1, (max.y - min.y) / lado + 1);
        let celda = |p: Punto| ((p.y - min.y) / lado * cols + (p.x - min.x) / lado) as usize;
        let mut celda_inicio = vec![0u32; (cols * filas) as usize + 1];
        for &p in &pos {
            celda_inicio[celda(p) + 1] += 1;
        }
        for i in 0..celda_inicio.len() - 1 {
            celda_inicio[i + 1] += celda_inicio[i];
        }
        let mut cursor = celda_inicio.clone();
        let mut celda_nodos = vec![0u32; n];
        for (i, &p) in pos.iter().enumerate() {
            let c = celda(p);
            celda_nodos[cursor[c] as usize] = i as u32;
            cursor[c] += 1;
        }

        Rutas {
            ids, indice_de, pos, inicio, destino, coste, vado, espaciado, arista_max, ratio_min: ratio_min as i64,
            min, lado, cols, filas, celda_inicio, celda_nodos, transitable,
        }
    }

    pub fn nodos(&self) -> usize {
        self.ids.len()
    }
    pub fn espaciado(&self) -> i64 {
        self.espaciado
    }

    /// Ruta con memoria propia (cómodo; para muchas rutas usa `ruta_con` o `rutas_en_paralelo`).
    pub fn ruta(&self, origen: Punto, destino: Punto, opciones: &Opciones) -> Option<Camino> {
        self.ruta_con(&mut Escritorio::nuevo(self.nodos()), origen, destino, opciones)
    }

    /// Lote en paralelo (rayon); un `Escritorio` por hilo. El resultado conserva el orden de las peticiones.
    pub fn rutas_en_paralelo(&self, peticiones: &[Peticion]) -> Vec<Option<Camino>> {
        peticiones
            .par_iter()
            .map_init(|| Escritorio::nuevo(self.nodos()), |e, p| self.ruta_con(e, p.origen, p.destino, &p.opciones))
            .collect()
    }

    /// Hasta `ENGANCHES` nodos cercanos a `p` con tramo libre: `(nodo, coste del tramo recto)`, del más cercano al más lejano
    /// (empates por índice). El radio de búsqueda se duplica desde 2 espaciados hasta 32.
    fn enganches(&self, p: Punto, bloqueado: &dyn Fn(u32) -> bool) -> Vec<(u32, i64)> {
        let mut radio = 2 * self.espaciado;
        for _ in 0..5 {
            let (cx0, cx1) = (((p.x - radio - self.min.x).div_euclid(self.lado)).max(0), ((p.x + radio - self.min.x).div_euclid(self.lado)).min(self.cols - 1));
            let (cy0, cy1) = (((p.y - radio - self.min.y).div_euclid(self.lado)).max(0), ((p.y + radio - self.min.y).div_euclid(self.lado)).min(self.filas - 1));
            let mut cerca: Vec<(i128, u32)> = Vec::new();
            for cy in cy0..=cy1 {
                for cx in cx0..=cx1 {
                    let c = (cy * self.cols + cx) as usize;
                    for &i in &self.celda_nodos[self.celda_inicio[c] as usize..self.celda_inicio[c + 1] as usize] {
                        let d2 = p.dist2(self.pos[i as usize]);
                        if d2 <= (radio as i128).pow(2) && !bloqueado(i) {
                            cerca.push((d2, i));
                        }
                    }
                }
            }
            cerca.sort_unstable();
            let libres: Vec<(u32, i64)> = cerca
                .into_iter()
                .filter(|&(_, i)| self.transitable.as_ref().is_none_or(|t| t.libre(p, self.pos[i as usize])))
                .take(if self.transitable.is_some() { ENGANCHES } else { 1 })
                .map(|(_, i)| (i, self.coste_recto(p, self.pos[i as usize])))
                .collect();
            if !libres.is_empty() {
                return libres;
            }
            radio *= 2;
        }
        Vec::new()
    }

    /// Llama a `f` con cada nodo a `radio` o menos de `p`.
    fn para_nodos_a(&self, p: Punto, radio: i64, mut f: impl FnMut(u32)) {
        if self.pos.is_empty() {
            return;
        }
        let (cx0, cx1) = (((p.x - radio - self.min.x).div_euclid(self.lado)).max(0), ((p.x + radio - self.min.x).div_euclid(self.lado)).min(self.cols - 1));
        let (cy0, cy1) = (((p.y - radio - self.min.y).div_euclid(self.lado)).max(0), ((p.y + radio - self.min.y).div_euclid(self.lado)).min(self.filas - 1));
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                let c = (cy * self.cols + cx) as usize;
                for &i in &self.celda_nodos[self.celda_inicio[c] as usize..self.celda_inicio[c + 1] as usize] {
                    if p.dist2(self.pos[i as usize]) <= (radio as i128).pow(2) {
                        f(i);
                    }
                }
            }
        }
    }

    /// Coste de un tramo recto en llano, en milésimas de espaciado (redondeo al más cercano).
    fn coste_recto(&self, a: Punto, b: Punto) -> i64 {
        (a.dist(b) * 1000 + self.espaciado / 2) / self.espaciado
    }

    fn h(&self, desde: Punto, destino: Punto, factor: i64) -> i64 {
        ((desde.dist(destino) as i128 * self.ratio_min as i128 * factor as i128 / UNO as i128) >> DESPLAZ) as i64
    }

    /// A* con la memoria de trabajo `e`.
    pub fn ruta_con(&self, e: &mut Escritorio, origen: Punto, destino: Punto, opciones: &Opciones) -> Option<Camino> {
        if let Some(t) = &self.transitable {
            if !t.en(origen) || !t.en(destino) {
                return None;
            }
        }
        let recta = |costes: Vec<i64>, puntos: Vec<Punto>| Camino { puntos, costes, nodos: Vec::new(), espaciado: self.espaciado };
        if origen == destino {
            return Some(recta(Vec::new(), vec![origen]));
        }
        // Los estorbos que cuentan: los que no tienen dentro el origen ni el destino.
        let factor = opciones.estorbos.as_deref().map_or(1, |x| x.factor);
        let fuera = |p: Punto, c: Punto, r: Mil| p.dist2(c) >= ((r - HOLGURA_ESTORBO).max(0) as i128).pow(2);
        let discos: Vec<(Punto, Mil)> = match opciones.estorbos.as_deref() {
            Some(x) if factor > 1 => x.discos.iter().copied().filter(|&(c, r)| fuera(origen, c, r) && fuera(destino, c, r)).collect(),
            _ => Vec::new(),
        };
        let cruza = |a: Punto, b: Punto| discos.iter().any(|&(c, r)| tramo_en_disco(a, b, c, r));
        let caro = |a: Punto, b: Punto, coste: i64| if !discos.is_empty() && cruza(a, b) { coste * factor } else { coste };
        let directo = origen.dist(destino) <= self.espaciado && self.transitable.as_ref().is_none_or(|t| t.libre(origen, destino)) && !cruza(origen, destino);
        if directo {
            return Some(recta(vec![self.coste_recto(origen, destino)], vec![origen, destino]));
        }

        // Marca de búsqueda; al dar la vuelta se limpian los sellos.
        e.marca = e.marca.wrapping_add(1);
        if e.marca == 0 {
            e.visto.fill(0);
            e.bloqueado.fill(0);
            e.cerca.fill(0);
            e.marca = 1;
        }
        let marca = e.marca;
        for id in &opciones.bloqueados {
            if let Some(&i) = self.indice_de.get(id) {
                e.bloqueado[i as usize] = marca;
            }
        }
        for (k, &(c, r)) in discos.iter().enumerate() {
            self.para_nodos_a(c, r + self.arista_max, |i| (e.cerca[i as usize], e.disco[i as usize]) = (marca, k as u32));
        }
        // Solo los estorbos de los extremos de la arista (los de los nodos cercanos): con cientos de plazas, mirarlas todas en cada arista es caro.
        let cruza_arista = |e: &Escritorio, u: usize, v: usize| {
            [u, v].iter().filter(|&&n| e.cerca[n] == marca).any(|&n| {
                let (c, r) = discos[e.disco[n] as usize];
                tramo_en_disco(self.pos[u], self.pos[v], c, r)
            })
        };
        let esta_bloqueado = |i: u32| e.bloqueado[i as usize] == marca;
        // La atracción de la red solo abarata aristas: la heurística se afloja al menor factor que puede aparecer (sigue admisible).
        let atraccion = opciones.atraccion.as_deref().filter(|a| !a.pesos.is_empty());
        let fh = atraccion.map_or(UNO, Atraccion::factor_minimo);
        let salidas = self.enganches(origen, &esta_bloqueado);
        let llegadas = self.enganches(destino, &esta_bloqueado);
        if salidas.is_empty() || llegadas.is_empty() {
            return None;
        }

        e.cola.clear();
        for &(s, c) in &salidas {
            let c = caro(origen, self.pos[s as usize], c);
            let (s_, h) = (s as usize, self.h(self.pos[s as usize], destino, fh));
            if e.visto[s_] != marca || c < e.g[s_] {
                (e.visto[s_], e.g[s_], e.padre[s_]) = (marca, c, SIN_PADRE);
                e.cola.push(Reverse((c + h, h, s, false)));
            }
        }

        // Búsqueda. Una entrada «meta» (`true`) es «llegar al destino real desde este nodo»: al sacarla, el coste es óptimo.
        // Sin conjunto cerrado (la heurística en punto fijo puede perder consistencia por un redondeo): se descartan las
        // entradas con g peor que el conocido.
        let mut final_desde: Option<u32> = None;
        while let Some(Reverse((f, _, u, meta))) = e.cola.pop() {
            if meta {
                final_desde = Some(u);
                break;
            }
            let gu = e.g[u as usize];
            let hu = self.h(self.pos[u as usize], destino, fh);
            if f > gu + hu {
                continue; // entrada vieja
            }
            if let Some(&(_, c)) = llegadas.iter().find(|&&(n, _)| n == u) {
                e.cola.push(Reverse((gu + caro(self.pos[u as usize], destino, c), 0, u, true)));
            }
            for k in self.inicio[u as usize] as usize..self.inicio[u as usize + 1] as usize {
                let v = self.destino[k];
                if esta_bloqueado(v) {
                    continue;
                }
                let mut coste = self.coste[k] as i64;
                if let Some(a) = atraccion {
                    if let Some(&peso) = a.pesos.get(&clave_arista(self.pos[u as usize], self.pos[v as usize])) {
                        coste = coste * a.factor(peso) / UNO;
                    }
                }
                let mut paso = coste + if self.vado[k] { opciones.penalizacion_vado } else { 0 };
                if !discos.is_empty() && cruza_arista(e, u as usize, v as usize) {
                    paso += coste * (factor - 1);
                }
                let g2 = gu + paso;
                let vi = v as usize;
                if e.visto[vi] != marca || g2 < e.g[vi] {
                    (e.visto[vi], e.g[vi], e.padre[vi], e.padre_arista[vi]) = (marca, g2, u, k as u32);
                    let h = self.h(self.pos[vi], destino, fh);
                    e.cola.push(Reverse((g2 + h, h, v, false)));
                }
            }
        }
        let ultimo = final_desde?;

        // Reconstrucción hacia atrás: nodos y coste real (sin penalización) de cada arista.
        let mut rev: Vec<(u32, i64, bool)> = Vec::new(); // (nodo, coste de la arista que llega a él, ¿es vado?; 0 y no para el primero)
        let mut n = ultimo;
        loop {
            let p = e.padre[n as usize];
            if p == SIN_PADRE {
                rev.push((n, 0, false));
                break;
            }
            let k = e.padre_arista[n as usize] as usize;
            rev.push((n, self.coste[k] as i64, self.vado[k]));
            n = p;
        }
        rev.reverse();
        let primero = rev[0].0;
        let c_ini = salidas.iter().find(|&&(s, _)| s == primero).expect("el origen del camino es un enganche").1;
        let c_fin = llegadas.iter().find(|&&(s, _)| s == ultimo).expect("el final del camino es un enganche").1;

        let mut puntos = vec![origen];
        let mut costes: Vec<i64> = Vec::new();
        let mut vados: Vec<bool> = Vec::new(); // ¿el tramo i es una arista de vado? (los de longitud cero son enganches)
        let mut pendiente = 0i64; // coste de tramos de longitud cero, que se pasa al siguiente
        let mut nodos = Vec::with_capacity(rev.len());
        let mut empujar = |p: Punto, c: i64, v: bool, puntos: &mut Vec<Punto>, vados: &mut Vec<bool>| {
            if puntos.last() == Some(&p) {
                pendiente += c;
            } else {
                puntos.push(p);
                costes.push(c + std::mem::take(&mut pendiente));
                vados.push(v);
            }
        };
        for (i, &(nodo, c, v)) in rev.iter().enumerate() {
            nodos.push(self.ids[nodo as usize]);
            empujar(self.pos[nodo as usize], if i == 0 { c_ini } else { c }, v, &mut puntos, &mut vados);
        }
        empujar(destino, c_fin, false, &mut puntos, &mut vados);
        if pendiente != 0 {
            *costes.last_mut().expect("hay al menos un tramo") += pendiente;
        }
        let mut camino = Camino { puntos, costes, nodos, espaciado: self.espaciado };
        if opciones.suavizar {
            if let Some(t) = self.transitable.as_ref().filter(|t| t.tiene_coste()) {
                self.estirar(t, &mut camino, &vados, &caro);
            }
        }
        Some(camino)
    }

    /// El suavizado por coste (ver «Suavizado» arriba). `vados[i]` dice si el tramo `i` es una arista de vado: esos tramos
    /// se quedan, y no se salta por encima de ellos.
    fn estirar(&self, t: &Transitable, camino: &mut Camino, vados: &[bool], caro: &dyn Fn(Punto, Punto, i64) -> i64) {
        let p = &camino.puntos;
        let n = p.len();
        if n < 3 {
            return;
        }
        // Coste de capa de cada tramo de la ruta del A* (lo que no se puede pisar cuenta como infinito).
        // Un tramo que se mete en un estorbo cuenta con su recargo, también la recta que lo sustituiría.
        let tramo: Vec<i64> = (0..n - 1).map(|k| t.coste_recto(p[k], p[k + 1], self.espaciado).map_or(i64::MAX / 4, |c| caro(p[k], p[k + 1], c))).collect();
        let (mut puntos, mut costes) = (vec![p[0]], Vec::with_capacity(n - 1));
        let mut i = 0;
        while i + 1 < n {
            // Se puede aterrizar como mucho en el arranque del primer vado que venga.
            let tope = (i + MAX_SALTO).min(n - 1).min(vados[i..].iter().position(|&v| v).map_or(usize::MAX, |k| i + k));
            let mut salto = (i + 1, camino.costes[i]);
            for j in (i + 2..=tope).rev() {
                if !t.libre(p[i], p[j]) {
                    continue;
                }
                let sustituye = tramo[i..j].iter().fold(0i64, |s, &c| s.saturating_add(c));
                if let Some(c) = t.coste_recto(p[i], p[j], self.espaciado).filter(|&c| caro(p[i], p[j], c) <= sustituye) {
                    salto = (j, c);
                    break;
                }
            }
            puntos.push(p[salto.0]);
            costes.push(salto.1);
            i = salto.0;
        }
        if puntos.len() == n {
            return;
        }
        camino.nodos.retain(|id| self.indice_de.get(id).is_some_and(|&k| puntos.contains(&self.pos[k as usize])));
        (camino.puntos, camino.costes) = (puntos, costes);
    }
}

#[cfg(test)]
mod pruebas;
