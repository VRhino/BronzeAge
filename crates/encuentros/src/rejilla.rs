//! Rejilla espacio-temporal: cada tramo se apunta en las celdas (espacio × ventana de tiempo) que toca, ensanchadas medio
//! `margen`, de modo que dos tramos a distancia ≤ `margen` en algún instante comparten celda. Solo los pares que
//! comparten celda llegan a la cuadrática.
//!
//! Los tramos **quietos y largos** (un campamento «aparcado» hasta el horizonte) no se trocean en ventanas: irían a miles
//! de celdas. Van a un índice solo espacial (`fijos`) y se cruzan con el índice espacial de todos los demás (`espacial`): así caben en la misma
//! rejilla las columnas acampadas y lo que nunca se mueve (las plazas, apuntadas con [`crate::plan_quieto`]).

use crate::{intervalo_de_acercamiento, Tramo};
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

/// Qué cuenta como encuentro cuando una entidad estrena plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entradas {
    /// También estar ya a ≤ radio al empezar el plan (una entidad que nace junto a otra): el encuentro es ese instante.
    Todas,
    /// Solo las **entradas** en el radio: si ya estaban juntas cuando el plan empieza, ese rato no cuenta (el encuentro ocurrió antes) y el
    /// siguiente es la próxima vez que se acerquen, si se alejan y vuelven.
    Nuevas,
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
    pub fn encuentros_de(&self, entidad: u64, plan: &Plan, radio: Mil, entradas: Entradas) -> Vec<Encuentro> {
        assert!(radio <= self.margen, "radio {radio} mayor que el margen de la rejilla ({})", self.margen);
        let mios = tramos_de(plan);
        let mut v: Vec<Encuentro> = self
            .buscar(entidad, &self.indexar(&mios))
            .into_iter()
            .filter_map(|otra| {
                let a = &self.apuntes[&otra];
                let instante = primer_cruce(&mios, &a.tramos, radio, entradas)?;
                let (pa, pb) = (posicion(&mios, instante), posicion(&a.tramos, instante));
                Some(Encuentro { otra, otra_generacion: a.generacion, instante, punto: Punto::new((pa.x + pb.x) >> 1, (pa.y + pb.y) >> 1) })
            })
            .collect();
        v.sort_unstable_by_key(|e| (e.instante, e.otra));
        v
    }

    /// Primer instante `>= desde` en que `a` y `b` (ya apuntadas) quedan a **más** de `radio`, estando a ≤ radio en `desde`: «la presa sale de la
    /// vista». Si ya no lo están en `desde`, `desde`. `None` si no se separan mientras duran sus planes, o si alguna no está apuntada.
    pub fn primera_salida(&self, a: u64, b: u64, radio: Mil, desde: Instante) -> Option<Instante> {
        primera_salida(&self.apuntes.get(&a)?.tramos, &self.apuntes.get(&b)?.tramos, radio, desde)
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

/// ¿Están las cajas de los dos tramos a más de `radio` en alguno de los ejes? Entonces no pueden acercarse a ≤ `radio` en ningún instante: un descarte
/// barato (cuatro comparaciones) antes de la cuadrática, que importa cuando un plan largo se cruza con muchas cosas quietas.
fn cajas_separadas(a: &Tramo, b: &Tramo, radio: Mil) -> bool {
    let caja = |t: &Tramo| (t.a.p.x.min(t.b.p.x), t.a.p.x.max(t.b.p.x), t.a.p.y.min(t.b.p.y), t.a.p.y.max(t.b.p.y));
    let ((ax0, ax1, ay0, ay1), (bx0, bx1, by0, by1)) = (caja(a), caja(b));
    ax0 - bx1 > radio || bx0 - ax1 > radio || ay0 - by1 > radio || by0 - ay1 > radio
}

/// Ventana de tiempo que comparten dos tramos, si la hay.
fn ventana(a: &Tramo, b: &Tramo) -> Option<(Instante, Instante)> {
    let (w0, w1) = (a.a.en.max(b.a.en), a.b.en.min(b.b.en));
    (w0 <= w1).then_some((w0, w1))
}

/// Avanza los dos punteros del barrido: los pares sucesivos tienen ventanas consecutivas.
fn avanzar(a: &[Tramo], b: &[Tramo], i: &mut usize, j: &mut usize) {
    match a[*i].b.en.cmp(&b[*j].b.en) {
        std::cmp::Ordering::Less => *i += 1,
        std::cmp::Ordering::Greater => *j += 1,
        std::cmp::Ordering::Equal => (*i, *j) = (*i + 1, *j + 1),
    }
}

/// Primer instante en que dos secuencias de tramos contiguos entran a ≤ `radio`. Barrido con dos punteros: las ventanas
/// de los pares sucesivos son consecutivas, así que el primer acierto es el más temprano. Con [`Entradas::Nuevas`], si en el instante en que
/// empieza `a` ya estaban a ≤ radio (y `b` ya existía), ese rato y los pares que lo prolongan no cuentan: el primer encuentro es la siguiente entrada.
fn primer_cruce(a: &[Tramo], b: &[Tramo], radio: Mil, entradas: Entradas) -> Option<Instante> {
    let (inicio, existia) = (a.first()?.a.en, b.first()?.a.en <= a.first()?.a.en);
    let (mut i, mut j) = (0, 0);
    // ¿Termina el par anterior dentro del radio? Entonces seguir dentro en el siguiente no es una entrada.
    let mut dentro = false;
    while i < a.len() && j < b.len() {
        match ventana(&a[i], &b[j]).zip((!cajas_separadas(&a[i], &b[j], radio)).then(|| intervalo_de_acercamiento(a[i], b[j], radio)).flatten()) {
            Some(((w0, w1), (primero, ultimo))) => {
                let ya_juntas = dentro || (entradas == Entradas::Nuevas && existia && w0 == inicio);
                if !(ya_juntas && primero == w0) {
                    return Some(primero);
                }
                dentro = ultimo == w1;
            }
            None => dentro = false,
        }
        avanzar(a, b, &mut i, &mut j);
    }
    None
}

/// Primer instante `>= desde` en que dos secuencias de tramos contiguos, a ≤ `radio` en `desde`, quedan a más de `radio`. Si en `desde` no
/// estaban a ≤ radio, `desde`. `None` si no se separan mientras duran los planes.
fn primera_salida(a: &[Tramo], b: &[Tramo], radio: Mil, desde: Instante) -> Option<Instante> {
    let (mut i, mut j) = (0, 0);
    // El instante en que tienen que seguir juntas: `desde`, y luego el comienzo de cada ventana (los pares son consecutivos).
    let mut juntas_en = desde;
    while i < a.len() && j < b.len() {
        if let Some((w0, w1)) = ventana(&a[i], &b[j]).filter(|&(_, w1)| w1 >= desde) {
            let en = juntas_en.max(w0);
            match intervalo_de_acercamiento(a[i], b[j], radio) {
                Some((primero, ultimo)) if primero <= en && en <= ultimo => {
                    if ultimo < w1 {
                        return Some(ultimo + 1);
                    }
                    juntas_en = w1;
                }
                _ => return Some(en),
            }
        }
        avanzar(a, b, &mut i, &mut j);
    }
    None
}
