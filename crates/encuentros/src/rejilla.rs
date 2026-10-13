//! Rejilla espacio-temporal: cada tramo se apunta en las celdas (espacio × ventana de tiempo) que toca, ensanchadas medio
//! `margen`, de modo que dos tramos a distancia ≤ `margen` en algún instante comparten celda. Solo los pares que
//! comparten celda llegan a la cuadrática.
//!
//! Los tramos **quietos y largos** (un campamento «aparcado» hasta el horizonte) no se trocean en ventanas: irían a miles
//! de celdas. Van a un índice solo espacial (`fijos`) y se cruzan con el índice espacial de todos los demás (`espacial`).

use crate::{primer_acercamiento, Tramo};
use base::{Instante, Mil, Plan, Punto};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// Hash rápido (FxHash) para claves enteras de celdas; el orden de iteración nunca influye en un resultado.
#[derive(Default, Clone, Copy)]
struct Fx(u64);

impl Hasher for Fx {
    fn write(&mut self, bytes: &[u8]) {
        bytes.iter().for_each(|&b| self.write_u64(b as u64));
    }
    fn write_u32(&mut self, i: u32) {
        self.write_u64(i as u64)
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_i32(&mut self, i: i32) {
        self.write_u64(i as u32 as u64)
    }
    fn write_i64(&mut self, i: i64) {
        self.write_u64(i as u64)
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

type Mapa<K, V> = HashMap<K, V, BuildHasherDefault<Fx>>;
type Celda = (i32, i32);
type CeldaVentana = (i32, i32, i64);

/// Una entidad que se acerca a otra, con el instante exacto del primer acercamiento a ≤ radio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Encuentro {
    pub otra: u64,
    /// Generación del plan de `otra` con el que se calculó: si cambia, el encuentro ya no vale.
    pub otra_generacion: u32,
    pub instante: Instante,
    /// Punto medio entre ambas en ese instante (simétrico: lo mismo desde cualquiera de las dos).
    pub punto: Punto,
}

#[derive(Default)]
struct Indice {
    ventanas: Vec<CeldaVentana>,
    espaciales: Vec<Celda>,
    fijas: Vec<Celda>,
}

struct Apunte {
    generacion: u32,
    tramos: Vec<Tramo>,
    indice: Indice,
}

pub struct Rejilla {
    celda: i64,
    ventana: i64,
    margen: Mil,
    ventanas: Mapa<CeldaVentana, Vec<u64>>,
    espacial: Mapa<Celda, Vec<u64>>,
    fijos: Mapa<Celda, Vec<u64>>,
    apuntes: Mapa<u64, Apunte>,
}

/// Un tramo quieto de más de estas ventanas va al índice solo espacial.
const VENTANAS_MAX_QUIETO: i64 = 8;

impl Rejilla {
    /// `celda`: lado en milésimas; `ventana`: ms; `margen`: el mayor radio con el que se consultará.
    pub fn nueva(celda: Mil, ventana: Instante, margen: Mil) -> Self {
        assert!(celda > 0 && ventana > 0 && margen >= 0);
        Self { celda, ventana, margen, ventanas: Mapa::default(), espacial: Mapa::default(), fijos: Mapa::default(), apuntes: Mapa::default() }
    }

    /// Valores pensados para el mundo real: celdas de 50 unidades, ventanas de 5 minutos, radios hasta 15 unidades.
    pub fn para_mundo_real() -> Self {
        Self::nueva(50_000, 300_000, 15_000)
    }

    pub fn len(&self) -> usize {
        self.apuntes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.apuntes.is_empty()
    }
    pub fn contiene(&self, entidad: u64) -> bool {
        self.apuntes.contains_key(&entidad)
    }

    /// Apunta (o reapunta, quitando antes lo anterior) los tramos del plan de la entidad.
    pub fn apuntar(&mut self, entidad: u64, generacion: u32, plan: &Plan) {
        self.quitar(entidad);
        let tramos = tramos_de(plan);
        let indice = self.indexar(&tramos);
        for &k in &indice.ventanas {
            self.ventanas.entry(k).or_default().push(entidad);
        }
        for &k in &indice.espaciales {
            self.espacial.entry(k).or_default().push(entidad);
        }
        for &k in &indice.fijas {
            self.fijos.entry(k).or_default().push(entidad);
        }
        self.apuntes.insert(entidad, Apunte { generacion, tramos, indice });
    }

    pub fn quitar(&mut self, entidad: u64) {
        let Some(Apunte { indice, .. }) = self.apuntes.remove(&entidad) else { return };
        sacar(&mut self.ventanas, &indice.ventanas, entidad);
        sacar(&mut self.espacial, &indice.espaciales, entidad);
        sacar(&mut self.fijos, &indice.fijas, entidad);
    }

    /// Entidades que comparten celda y ventana con `entidad` (ordenadas, sin ella). Vacío si no está apuntada.
    pub fn candidatos(&self, entidad: u64) -> Vec<u64> {
        self.apuntes.get(&entidad).map_or_else(Vec::new, |a| self.buscar(entidad, &a.indice))
    }

    /// Primer acercamiento a ≤ `radio` de `entidad` (con el plan dado, esté o no apuntado ya) con cada candidata, por
    /// instante y luego por entidad. `radio` no puede pasar del `margen` de la rejilla.
    pub fn encuentros_de(&self, entidad: u64, plan: &Plan, radio: Mil) -> Vec<Encuentro> {
        assert!(radio <= self.margen, "radio {radio} mayor que el margen de la rejilla ({})", self.margen);
        let mios = tramos_de(plan);
        let mut v: Vec<Encuentro> = self
            .buscar(entidad, &self.indexar(&mios))
            .into_iter()
            .filter_map(|otra| {
                let a = &self.apuntes[&otra];
                let instante = primer_cruce(&mios, &a.tramos, radio)?;
                let (pa, pb) = (posicion(&mios, instante), posicion(&a.tramos, instante));
                Some(Encuentro { otra, otra_generacion: a.generacion, instante, punto: Punto::new((pa.x + pb.x) >> 1, (pa.y + pb.y) >> 1) })
            })
            .collect();
        v.sort_unstable_by_key(|e| (e.instante, e.otra));
        v
    }

    fn buscar(&self, yo: u64, ix: &Indice) -> Vec<u64> {
        let mut r: Vec<u64> = Vec::new();
        let mut tomar = |v: Option<&Vec<u64>>| r.extend(v.into_iter().flatten().copied().filter(|&e| e != yo));
        for &(x, y, w) in &ix.ventanas {
            tomar(self.ventanas.get(&(x, y, w)));
            tomar(self.fijos.get(&(x, y)));
        }
        for k in &ix.fijas {
            tomar(self.espacial.get(k));
            tomar(self.fijos.get(k));
        }
        r.sort_unstable();
        r.dedup();
        r
    }

    fn indexar(&self, tramos: &[Tramo]) -> Indice {
        let (v, e) = (self.ventana, (self.margen + 1) / 2 + 1);
        let celda = |c: Mil| c.div_euclid(self.celda) as i32;
        let mut ix = Indice::default();
        for t in tramos {
            let (w0, w1) = (t.a.en.div_euclid(v), t.b.en.div_euclid(v));
            if t.es_quieto() && w1 - w0 > VENTANAS_MAX_QUIETO {
                let (p, mut cs) = (t.a.p, Vec::new());
                for cx in celda(p.x - e)..=celda(p.x + e) {
                    cs.extend((celda(p.y - e)..=celda(p.y + e)).map(|cy| (cx, cy)));
                }
                ix.fijas.extend(cs);
                continue;
            }
            for w in w0..=w1 {
                let (pa, pb) = (t.posicion(t.a.en.max(w * v)), t.posicion(t.b.en.min(w * v + v - 1)));
                let (x0, x1) = (pa.x.min(pb.x) - e, pa.x.max(pb.x) + e);
                let (y0, y1) = (pa.y.min(pb.y) - e, pa.y.max(pb.y) + e);
                for cx in celda(x0)..=celda(x1) {
                    for cy in celda(y0)..=celda(y1) {
                        ix.ventanas.push((cx, cy, w));
                        ix.espaciales.push((cx, cy));
                    }
                }
            }
        }
        for lista in [&mut ix.ventanas as &mut dyn Ordenar, &mut ix.espaciales, &mut ix.fijas] {
            lista.ordenar_y_unir();
        }
        ix
    }
}

trait Ordenar {
    fn ordenar_y_unir(&mut self);
}
impl<T: Ord> Ordenar for Vec<T> {
    fn ordenar_y_unir(&mut self) {
        self.sort_unstable();
        self.dedup();
    }
}

fn sacar<K: std::hash::Hash + Eq>(mapa: &mut Mapa<K, Vec<u64>>, claves: &[K], entidad: u64) {
    for k in claves {
        if let Some(v) = mapa.get_mut(k) {
            if let Some(i) = v.iter().position(|&e| e == entidad) {
                v.swap_remove(i);
            }
            if v.is_empty() {
                mapa.remove(k);
            }
        }
    }
}

/// Tramos del plan; un plan de un solo vértice no tiene tramos (para estar quieto, un tramo quieto hasta el horizonte).
fn tramos_de(plan: &Plan) -> Vec<Tramo> {
    Tramo::del_plan(plan).collect()
}

/// Posición en `t` según una lista de tramos contiguos (anclada a los extremos).
fn posicion(tramos: &[Tramo], t: Instante) -> Punto {
    let i = tramos.partition_point(|x| x.b.en < t).min(tramos.len() - 1);
    tramos[i].posicion(t)
}

/// Primer instante en que dos secuencias de tramos contiguos quedan a ≤ `radio`. Barrido con dos punteros: las ventanas
/// de los pares sucesivos son consecutivas, así que el primer acierto es el más temprano.
fn primer_cruce(a: &[Tramo], b: &[Tramo], radio: Mil) -> Option<Instante> {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if let Some(t) = primer_acercamiento(a[i], b[j], radio) {
            return Some(t);
        }
        match a[i].b.en.cmp(&b[j].b.en) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => (i, j) = (i + 1, j + 1),
        }
    }
    None
}
