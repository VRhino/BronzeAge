//! Visión muestreada cada 250 ms por Facción, en paralelo (fase 5; `Docs/Mechanics/Niebla_De_Guerra.md`).
//!
//! La visión es **informativa**: no hace falta el instante exacto en que algo entra en la vista (Arquitectura/13 §3).
//! El núcleo llama a [`Vision::muestrear`] con las posiciones que dan los planes en el instante `t`, compara con la
//! muestra anterior ([`diff`]) y de ahí salen los `Delta` de la sesión. Todo en enteros y con salida ordenada: el
//! resultado no depende del número de hilos de `rayon`.
//!
//! # Reglas del diseño que se respetan (secciones de `Niebla_De_Guerra.md`)
//!
//! - §3 Los ojos: plaza = `radioPotencial` (margen 0: **ve solo su zona**, contra el disco); avanzada = 12; columna con
//!   soldados = 30; columna sin soldados = 15; Mirada = su radio. Ver es `distancia ≤ radio` (**no estricto**).
//! - §3 / §5.4: una caravana «grande» llama la atención si hay un ojo propio, aliado o prestado a `≤ 60` de ella
//!   aunque no entre en su radio normal → [`Movil::atencion`].
//! - §5.1 / §5.4: lo **propio** se ve siempre (las columnas propias viajan enteras); lo ajeno, solo si cae en un ojo.
//! - §5.2: la visión se **comparte** con alianza o vasallaje activos (cualquier dirección) → [`Vision::con_alianzas`].
//!   Los aliados suman a lo visible, no a lo explorado.
//! - §4.1: la exploración de una Facción la graban sus ojos **propios**; los aliados y las Miradas no. La memoria solo
//!   crece. No se muestrea: se **barre** el recorrido exacto de cada ojo entre dos instantes ([`Mascara::barrer`],
//!   [`recorrido`]), así que lo explorado no depende de cada cuánto se mire (lo hace el núcleo al cerrar sus versiones).
//! - §4.4: la visión prestada nunca se graba. §2.2: una celda cuenta como vista cuando lo está su centro, salvo la del
//!   propio ojo, que siempre. §2.3: la máscara que viaja es `celdas` (explorado ∪ ojos propios ahora) y `visibles`
//!   ([`niebla_proyectada`]).
//! - §5.4 Ejércitos ajenos / Doc 13 §5: de lo ajeno solo viaja el tramo dentro de la vista y del horizonte
//!   corto ([`tramo_visible`]).
//!
//! # Suposiciones (el documento no las fija)
//!
//! 1. La rejilla de la niebla tiene la celda del bloque `EXPLORACION` (5 unidades) y la deriva de los límites del mapa quien
//!    la usa ([`rejilla`]).
//! 2. La alianza no es transitiva: una Facción comparte con sus aliados directos.
//! 3. Quién es observador (plaza, avanzada, columna según tropa, Mirada vigente) y qué móviles se omiten (en batalla,
//!    caravana propia con destino propio, etc.) lo decide el núcleo al armar las listas; aquí solo se mide.
//! 4. Un [`FaccionId`] distinto por cada héroe huérfano (su `exploracionPersonal`) lo asigna el llamante.
//! 5. `tramo_visible` devuelve solo el primer pedazo contiguo dentro del rectángulo; si el plan sale y vuelve a entrar
//!    en menos de 30 s, el resto llega en el siguiente muestreo.

mod niebla;
mod tramo;

pub use niebla::{niebla_proyectada, recorrido, rejilla, ultima_vez, Mascara};
pub use tramo::{tramo_visible, Rect};

use base::{Instante, Mil, Plan, Punto};
use rayon::prelude::*;

pub type FaccionId = u32;
pub type EntidadId = u64;

/// Radios de visión vigentes, en milésimas de unidad (Niebla_De_Guerra §7).
pub mod radios {
    use base::Mil;
    pub const EJERCITO: Mil = 30_000;
    pub const JUGADOR_SOLO: Mil = 15_000;
    pub const AVANZADA: Mil = 12_000;
    pub const MIRADA: Mil = 30_000;
    pub const CARAVANA_GRANDE: Mil = 60_000;
    /// Horizonte corto del plan ajeno (contrato 05).
    pub const HORIZONTE_MS: i64 = 30_000;
    /// Cuánto tramo ajeno le queda al cliente cuando el muestreo le manda el siguiente (decisión 6 del autor, 2026-10-22).
    pub const RENOVAR_MS: i64 = 10_000;
    /// Columna con soldados en pie ve `EJERCITO`; sin ellos, `JUGADOR_SOLO` (§3).
    pub fn columna(con_soldados: bool) -> Mil {
        if con_soldados { EJERCITO } else { JUGADOR_SOLO }
    }
}

/// Dónde está algo: fijo, o siguiendo un plan (por referencia: el núcleo no clona planes en cada muestreo).
#[derive(Clone, Copy, Debug)]
pub enum Donde<'a> {
    Fijo(Punto),
    Plan(&'a Plan),
}

impl Donde<'_> {
    /// Posición en `t`; `None` si el plan no tiene vértices.
    pub fn en(&self, t: Instante) -> Option<Punto> {
        match self {
            Donde::Fijo(p) => Some(*p),
            Donde::Plan(plan) => plan.posicion_en(t),
        }
    }
}

/// Un ojo: lo que ve `faccion` (y sus aliados) en un círculo de `radio` alrededor de `donde`.
#[derive(Clone, Copy, Debug)]
pub struct Observador<'a> {
    pub faccion: FaccionId,
    pub radio: Mil,
    pub donde: Donde<'a>,
}

impl<'a> Observador<'a> {
    pub fn fijo(faccion: FaccionId, centro: Punto, radio: Mil) -> Self {
        Self { faccion, radio, donde: Donde::Fijo(centro) }
    }
    pub fn movil(faccion: FaccionId, plan: &'a Plan, radio: Mil) -> Self {
        Self { faccion, radio, donde: Donde::Plan(plan) }
    }
}

/// Algo que se puede ver: columna, caravana, plaza ajena, campamento…
#[derive(Clone, Copy, Debug)]
pub struct Movil<'a> {
    pub entidad: EntidadId,
    /// Dueña. Siempre ve lo suyo; las demás, solo si cae en un ojo.
    pub faccion: FaccionId,
    pub donde: Donde<'a>,
    /// Caravana grande: además de por el radio del ojo, se ve desde cualquier ojo a `≤ atencion` (§5.4).
    pub atencion: Option<Mil>,
}

impl<'a> Movil<'a> {
    pub fn new(entidad: EntidadId, faccion: FaccionId, donde: Donde<'a>) -> Self {
        Self { entidad, faccion, donde, atencion: None }
    }
}

/// Qué ve cada Facción en un instante: ids de entidad ordenados. Una Facción con ojos o con móviles propios aparece
/// siempre, aunque no vea nada.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Muestra {
    pub t: Instante,
    pub por_faccion: Vec<(FaccionId, Vec<EntidadId>)>,
}

impl Muestra {
    pub fn de(&self, faccion: FaccionId) -> &[EntidadId] {
        match self.por_faccion.binary_search_by_key(&faccion, |(f, _)| *f) {
            Ok(i) => &self.por_faccion[i].1,
            Err(_) => &[],
        }
    }
}

/// Lo que entra y sale de la vista de una Facción entre dos muestras (ordenados).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CambioVista {
    pub entran: Vec<EntidadId>,
    pub salen: Vec<EntidadId>,
}

/// Cambios por Facción; solo aparecen las que cambiaron.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cambios {
    pub por_faccion: Vec<(FaccionId, CambioVista)>,
}

/// Diferencia entre dos muestras: lo que alimenta los `Delta` de la sesión.
pub fn diff(anterior: &Muestra, nueva: &Muestra) -> Cambios {
    let mut fs: Vec<FaccionId> = anterior.por_faccion.iter().chain(&nueva.por_faccion).map(|(f, _)| *f).collect();
    fs.sort_unstable();
    fs.dedup();
    let por_faccion = fs
        .into_iter()
        .filter_map(|f| {
            let (antes, ahora) = (anterior.de(f), nueva.de(f));
            let c = CambioVista { entran: restar(ahora, antes), salen: restar(antes, ahora) };
            (!c.entran.is_empty() || !c.salen.is_empty()).then_some((f, c))
        })
        .collect();
    Cambios { por_faccion }
}

/// `a \ b` sobre listas ordenadas.
fn restar(a: &[EntidadId], b: &[EntidadId]) -> Vec<EntidadId> {
    let mut out = Vec::new();
    let mut j = 0;
    for &x in a {
        while j < b.len() && b[j] < x {
            j += 1;
        }
        if j >= b.len() || b[j] != x {
            out.push(x);
        }
    }
    out
}

/// Configuración de la visión: qué Facciones comparten ojos.
#[derive(Clone, Debug, Default)]
pub struct Vision {
    /// Pares `(a, b)` con `a < b`, ordenados y sin repetir.
    alianzas: Vec<(FaccionId, FaccionId)>,
}

/// Ojo ya resuelto a una posición.
#[derive(Clone, Copy)]
struct Ojo {
    faccion: FaccionId,
    p: Punto,
    radio: Mil,
}

/// Móvil ya resuelto a una posición: `(entidad, dueña, posición, atención)`.
type Pos = (EntidadId, FaccionId, Punto, Option<Mil>);

/// Lado de la rejilla espacial (milésimas): del orden del radio de un ejército.
const LADO: i64 = 30_000;

impl Vision {
    pub fn new() -> Self {
        Self::default()
    }

    /// Alianzas y vasallajes `activos` (cualquier dirección): comparten visión (§5.2).
    pub fn con_alianzas(pares: impl IntoIterator<Item = (FaccionId, FaccionId)>) -> Self {
        let mut alianzas: Vec<_> = pares.into_iter().filter(|(a, b)| a != b).map(|(a, b)| (a.min(b), a.max(b))).collect();
        alianzas.sort_unstable();
        alianzas.dedup();
        Self { alianzas }
    }

    /// Facciones que comparten visión con `f`, ordenadas.
    pub fn aliados(&self, f: FaccionId) -> Vec<FaccionId> {
        let mut v: Vec<_> =
            self.alianzas.iter().filter_map(|&(a, b)| if a == f { Some(b) } else if b == f { Some(a) } else { None }).collect();
        v.sort_unstable();
        v
    }

    /// Qué ve cada Facción en `t`. Determinista: el orden de entrada y los hilos no cambian el resultado.
    pub fn muestrear(&self, t: Instante, observadores: &[Observador], moviles: &[Movil]) -> Muestra {
        // 1. Posiciones en t (en paralelo; `collect` conserva el orden).
        let mut ojos: Vec<Ojo> =
            observadores.par_iter().filter_map(|o| Some(Ojo { faccion: o.faccion, p: o.donde.en(t)?, radio: o.radio })).collect();
        let mov: Vec<Pos> = moviles.par_iter().filter_map(|m| Some((m.entidad, m.faccion, m.donde.en(t)?, m.atencion))).collect();

        // 2. Rejilla espacial de móviles (CSR) y atajos por Facción.
        let rejilla = Rejilla::nueva(&mov);
        let mut por_dueno: Vec<u32> = (0..mov.len() as u32).collect();
        por_dueno.sort_by_key(|&i| mov[i as usize].1);
        let atenciones: Vec<u32> = (0..mov.len() as u32).filter(|&i| mov[i as usize].3.is_some()).collect();
        ojos.sort_by_key(|o| o.faccion);

        let mut facciones: Vec<FaccionId> = observadores.iter().map(|o| o.faccion).chain(moviles.iter().map(|m| m.faccion)).collect();
        facciones.sort_unstable();
        facciones.dedup();

        // 3. Por Facción, en paralelo.
        let por_faccion = facciones
            .par_iter()
            .map(|&f| {
                let mut vistos: Vec<EntidadId> = Vec::new();
                // Lo propio, siempre.
                let a = por_dueno.partition_point(|&i| mov[i as usize].1 < f);
                for &i in por_dueno[a..].iter().take_while(|&&i| mov[i as usize].1 == f) {
                    vistos.push(mov[i as usize].0);
                }
                // Ojos propios y de los aliados.
                let mut grupo = self.aliados(f);
                grupo.push(f);
                for g in grupo {
                    let a = ojos.partition_point(|o| o.faccion < g);
                    for ojo in ojos[a..].iter().take_while(|o| o.faccion == g) {
                        rejilla.en_disco(&mov, ojo.p, ojo.radio, |i| vistos.push(mov[i as usize].0));
                        // ponytail: lista lineal de caravanas grandes; rejilla propia si llegan a miles.
                        for &i in &atenciones {
                            let (id, _, p, at) = mov[i as usize];
                            if dentro(ojo.p, p, at.unwrap_or(0)) {
                                vistos.push(id);
                            }
                        }
                    }
                }
                vistos.sort_unstable();
                vistos.dedup();
                (f, vistos)
            })
            .collect();
        Muestra { t, por_faccion }
    }
}

fn dentro(centro: Punto, p: Punto, radio: Mil) -> bool {
    let (dx, dy) = (centro.x - p.x, centro.y - p.y);
    dx * dx + dy * dy <= radio * radio
}

/// Rejilla espacial de móviles en formato CSR: los de la celda `c` son `items[inicio[c]..inicio[c + 1]]`.
struct Rejilla {
    min: Punto,
    cols: i64,
    filas: i64,
    inicio: Vec<u32>,
    items: Vec<u32>,
}

impl Rejilla {
    fn nueva(mov: &[Pos]) -> Self {
        let (mut min, mut max) = (Punto::new(0, 0), Punto::new(0, 0));
        if let Some(&(_, _, p, _)) = mov.first() {
            (min, max) = (p, p);
        }
        for &(_, _, p, _) in mov {
            min = Punto::new(min.x.min(p.x), min.y.min(p.y));
            max = Punto::new(max.x.max(p.x), max.y.max(p.y));
        }
        let cols = (max.x - min.x) / LADO + 1;
        let filas = (max.y - min.y) / LADO + 1;
        let mut r = Rejilla { min, cols, filas, inicio: vec![0; (cols * filas) as usize + 1], items: vec![0; mov.len()] };
        for &(_, _, p, _) in mov {
            let c = r.celda(p);
            r.inicio[c + 1] += 1;
        }
        for c in 0..(cols * filas) as usize {
            r.inicio[c + 1] += r.inicio[c];
        }
        let mut relleno = r.inicio.clone();
        for (i, &(_, _, p, _)) in mov.iter().enumerate() {
            let c = r.celda(p);
            r.items[relleno[c] as usize] = i as u32;
            relleno[c] += 1;
        }
        r
    }

    fn celda(&self, p: Punto) -> usize {
        (((p.y - self.min.y) / LADO) * self.cols + (p.x - self.min.x) / LADO) as usize
    }

    /// Llama a `f(índice)` por cada móvil a `≤ radio` de `centro` (no estricto).
    fn en_disco(&self, mov: &[Pos], centro: Punto, radio: Mil, mut f: impl FnMut(u32)) {
        let c0 = (centro.x - radio - self.min.x).div_euclid(LADO).max(0);
        let c1 = (centro.x + radio - self.min.x).div_euclid(LADO).min(self.cols - 1);
        let f0 = (centro.y - radio - self.min.y).div_euclid(LADO).max(0);
        let f1 = (centro.y + radio - self.min.y).div_euclid(LADO).min(self.filas - 1);
        for fila in f0..=f1 {
            for col in c0..=c1 {
                let c = (fila * self.cols + col) as usize;
                for &i in &self.items[self.inicio[c] as usize..self.inicio[c + 1] as usize] {
                    if dentro(centro, mov[i as usize].2, radio) {
                        f(i);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
