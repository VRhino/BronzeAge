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

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use base::{Instante, Mil, Plan, Punto, Vertice};
use contrato::v1::GrafoNavegacion;
use mundo::{Mundo, TAMANO_CELDA};
use rayon::prelude::*;

/// Cuántos nodos de enganche se prueban en cada extremo.
pub const ENGANCHES: usize = 4;
/// Desplazamiento de la heurística en punto fijo.
const DESPLAZ: u32 = 20;
const SIN_PADRE: u32 = u32::MAX;

/// Opciones de una petición de ruta.
#[derive(Clone, Debug, Default)]
pub struct Opciones {
    /// Coste añadido (en milésimas de espaciado, como `coste_milesimas`) a cada arista vado en la búsqueda. No entra en los
    /// tiempos del plan: solo desvía la ruta.
    pub penalizacion_vado: i64,
    /// Ids de nodo (`NodoGrafo.id`) por los que no se puede pasar ni engancharse.
    pub bloqueados: Vec<u32>,
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

/// Memoria de trabajo reutilizable de una búsqueda (una por hilo): evita reservar y poner a cero 53.000 entradas por ruta.
pub struct Escritorio {
    g: Vec<i64>,
    padre: Vec<u32>,
    padre_arista: Vec<u32>,
    /// `visto[i] == marca` ⇒ `g[i]` es de esta búsqueda.
    visto: Vec<u32>,
    /// `bloqueado[i] == marca` ⇒ nodo bloqueado en esta búsqueda.
    bloqueado: Vec<u32>,
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

/// Copia a bits de la transitabilidad del mundo (una celda = `TAMANO_CELDA`).
struct Transitable {
    cols: i64,
    filas: i64,
    bits: Vec<u64>,
}

impl Transitable {
    fn de(m: &Mundo) -> Self {
        let (c, f) = m.rejilla();
        let mut bits = vec![0u64; (c * f).div_ceil(64)];
        let t = TAMANO_CELDA as i64;
        for fila in 0..f {
            for col in 0..c {
                if m.transitable_en(col as i64 * t + t / 2, fila as i64 * t + t / 2) {
                    let i = fila * c + col;
                    bits[i >> 6] |= 1 << (i & 63);
                }
            }
        }
        Self { cols: c as i64, filas: f as i64, bits }
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
        self.bits[i >> 6] >> (i & 63) & 1 == 1
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
            ids, indice_de, pos, inicio, destino, coste, vado, espaciado, ratio_min: ratio_min as i64,
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

    /// Coste de un tramo recto en llano, en milésimas de espaciado (redondeo al más cercano).
    fn coste_recto(&self, a: Punto, b: Punto) -> i64 {
        (a.dist(b) * 1000 + self.espaciado / 2) / self.espaciado
    }

    fn h(&self, desde: Punto, destino: Punto) -> i64 {
        ((desde.dist(destino) as i128 * self.ratio_min as i128) >> DESPLAZ) as i64
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
        let directo = origen.dist(destino) <= self.espaciado && self.transitable.as_ref().is_none_or(|t| t.libre(origen, destino));
        if directo {
            return Some(recta(vec![self.coste_recto(origen, destino)], vec![origen, destino]));
        }

        // Marca de búsqueda; al dar la vuelta se limpian los sellos.
        e.marca = e.marca.wrapping_add(1);
        if e.marca == 0 {
            e.visto.fill(0);
            e.bloqueado.fill(0);
            e.marca = 1;
        }
        let marca = e.marca;
        for id in &opciones.bloqueados {
            if let Some(&i) = self.indice_de.get(id) {
                e.bloqueado[i as usize] = marca;
            }
        }
        let esta_bloqueado = |i: u32| e.bloqueado[i as usize] == marca;
        let salidas = self.enganches(origen, &esta_bloqueado);
        let llegadas = self.enganches(destino, &esta_bloqueado);
        if salidas.is_empty() || llegadas.is_empty() {
            return None;
        }

        e.cola.clear();
        for &(s, c) in &salidas {
            let (s_, h) = (s as usize, self.h(self.pos[s as usize], destino));
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
            let hu = self.h(self.pos[u as usize], destino);
            if f > gu + hu {
                continue; // entrada vieja
            }
            if let Some(&(_, c)) = llegadas.iter().find(|&&(n, _)| n == u) {
                e.cola.push(Reverse((gu + c, 0, u, true)));
            }
            for k in self.inicio[u as usize] as usize..self.inicio[u as usize + 1] as usize {
                let v = self.destino[k];
                if esta_bloqueado(v) {
                    continue;
                }
                let paso = self.coste[k] as i64 + if self.vado[k] { opciones.penalizacion_vado } else { 0 };
                let g2 = gu + paso;
                let vi = v as usize;
                if e.visto[vi] != marca || g2 < e.g[vi] {
                    (e.visto[vi], e.g[vi], e.padre[vi], e.padre_arista[vi]) = (marca, g2, u, k as u32);
                    let h = self.h(self.pos[vi], destino);
                    e.cola.push(Reverse((g2 + h, h, v, false)));
                }
            }
        }
        let ultimo = final_desde?;

        // Reconstrucción hacia atrás: nodos y coste real (sin penalización) de cada arista.
        let mut rev: Vec<(u32, i64)> = Vec::new(); // (nodo, coste de la arista que llega a él; 0 para el primero)
        let mut n = ultimo;
        loop {
            let p = e.padre[n as usize];
            if p == SIN_PADRE {
                rev.push((n, 0));
                break;
            }
            rev.push((n, self.coste[e.padre_arista[n as usize] as usize] as i64));
            n = p;
        }
        rev.reverse();
        let primero = rev[0].0;
        let c_ini = salidas.iter().find(|&&(s, _)| s == primero).expect("el origen del camino es un enganche").1;
        let c_fin = llegadas.iter().find(|&&(s, _)| s == ultimo).expect("el final del camino es un enganche").1;

        let mut puntos = vec![origen];
        let mut costes: Vec<i64> = Vec::new();
        let mut pendiente = 0i64; // coste de tramos de longitud cero, que se pasa al siguiente
        let mut nodos = Vec::with_capacity(rev.len());
        let mut empujar = |p: Punto, c: i64, puntos: &mut Vec<Punto>| {
            if puntos.last() == Some(&p) {
                pendiente += c;
            } else {
                puntos.push(p);
                costes.push(c + std::mem::take(&mut pendiente));
            }
        };
        for (i, &(nodo, c)) in rev.iter().enumerate() {
            nodos.push(self.ids[nodo as usize]);
            empujar(self.pos[nodo as usize], if i == 0 { c_ini } else { c }, &mut puntos);
        }
        empujar(destino, c_fin, &mut puntos);
        if pendiente != 0 {
            *costes.last_mut().expect("hay al menos un tramo") += pendiente;
        }
        Some(Camino { puntos, costes, nodos, espaciado: self.espaciado })
    }
}

#[cfg(test)]
mod pruebas;
