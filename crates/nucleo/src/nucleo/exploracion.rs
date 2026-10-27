//! Lo que graban los ojos (Niebla_De_Guerra §3-§4, 4H T3d): el núcleo pone al día la memoria de la niebla (`memoria::Memoria`) al cerrar cada versión, con lo que la transición tocó.
//!
//! - **Columnas**: una columna tocada barre su recorrido desde su [`Vigia`] hasta ahora (con el plan, la Facción y los héroes que tenía, que no cambian sin tocarla) y graba lo explorado en su
//!   Facción (sin bandera, en la de cada héroe o en lo suyo) y la ficha de cada sitio ajeno que vio, con el **último instante** en que lo vio. Lo que una columna no tocada lleva andado no está en
//!   la memoria todavía: lo deriva quien proyecta ([`Nucleo::recorrido_pendiente`], [`Nucleo::avistados_pendientes`]) y es exactamente lo que se grabará.
//! - **Ojos fijos** (la zona de una plaza, que ve su disco entero, y una avanzada operativa, 12): se graban al cambiar (nacer, crecer, cambiar de dueño, caer); mientras no cambian ven siempre lo
//!   mismo.
//! - **Sitios** (plazas, pueblos y avanzadas, también las ruinas de una ciudadela sin dueño): cuando cambia lo que se ve de uno (su dueño, su nivel o su etapa, su radio, su estado —activo, en
//!   obra, en ruinas—, que exista), primero se graba con lo de **antes** lo que cada ojo ajeno vio de él
//!   desde la última vez que se grabó, y desde ese instante su ficha vale lo de ahora (`Memoria::sitios`): así ninguna ficha guarda algo que su Facción no llegó a ver. Quien lo ve en ese mismo
//!   instante se lleva lo de ahora; si desapareció y alguien mira su sitio, la ficha se olvida (§4.1).
//!
//! Lo que el núcleo recuerda de cada ojo y de cada sitio al cerrar la última versión ([`Ojos`]) no se persiste: entre versiones es lo mismo que dice el estado, y se rehace de él al arrancar o
//! recuperar. Sin artefacto de mundo (`Memoria::rejilla` vacía) no se graba nada.
//!
//! El alcance con que se barre lo andado es el que la columna tenía al empezar a grabar (su vigía, Niebla_De_Guerra §4.1): si pierde los soldados sin que nada la toque, sigue grabando 30 hasta el
//! próximo cambio.

use super::*;
use crate::memoria::{DatosSitio, EstadoSitio, Ficha, Memoria, Sitio, Vigia};
use asentamiento::avanzadas::{Avanzada, Estado as EstadoAvanzada};
use base::Vertice;
use militar::columna;
use std::collections::BTreeSet;
use vision::{recorrido, ultima_vez};

/// Un ojo que anda (una columna) tal como estaba al cerrar la última versión, con la caja de todo su plan (para no rehacer su recorrido si un sitio cae lejos).
#[derive(Clone)]
struct Andante {
    faccion: Option<u32>,
    heroes: Vec<u64>,
    plan: Plan,
    quieto: Punto,
    caja: (Punto, Punto),
}

/// Un ojo fijo: la zona de una plaza o una avanzada operativa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fijo {
    pub faccion: u32,
    pub centro: Punto,
    pub radio: Mil,
}

/// Lado de las celdas del índice de sitios y ojos fijos (milésimas): no menor que el alcance de ningún ojo fijo (una plaza ve hasta 30).
const LADO: Mil = 32_000;

/// Sitios por celda de [`LADO`]: para no recorrer el mapa entero cada vez que algo cambia.
#[derive(Default)]
struct Rejilla(BTreeMap<(i64, i64), Vec<Sitio>>);

impl Rejilla {
    fn celda(p: Punto) -> (i64, i64) {
        (p.x.div_euclid(LADO), p.y.div_euclid(LADO))
    }

    fn poner(&mut self, s: Sitio, p: Punto) {
        self.0.entry(Self::celda(p)).or_default().push(s);
    }

    fn quitar(&mut self, s: Sitio, p: Punto) {
        let c = Self::celda(p);
        if let Some(v) = self.0.get_mut(&c) {
            v.retain(|x| *x != s);
            if v.is_empty() {
                self.0.remove(&c);
            }
        }
    }

    /// Los sitios de las celdas que tocan la caja `[a, b]` (pueden quedar fuera de ella: hay que filtrar).
    fn en_caja(&self, a: Punto, b: Punto) -> impl Iterator<Item = Sitio> + '_ {
        let ((c0, f0), (c1, f1)) = (Self::celda(a), Self::celda(b));
        (f0..=f1).flat_map(move |f| self.0.range((c0, f)..=(c1, f)).filter(move |((_, g), _)| *g == f)).flat_map(|(_, v)| v.iter().copied())
    }
}

/// Los ojos y los sitios como estaban al cerrar la última versión, con su índice por celdas.
pub(super) struct Ojos {
    columnas: BTreeMap<u64, Andante>,
    fijos: BTreeMap<Sitio, Fijo>,
    sitios: BTreeMap<Sitio, DatosSitio>,
    /// Los pueblos y las avanzadas de cada plaza: cambian de dueño con ella, y desaparecen con ella.
    hijos: BTreeMap<u64, Vec<Sitio>>,
    por_sitio: Rejilla,
    por_fijo: Rejilla,
}

impl Ojos {
    fn sitio(&mut self, s: Sitio, ahora: Option<DatosSitio>) {
        if let Some(d) = self.sitios.remove(&s) {
            self.por_sitio.quitar(s, d.posicion);
        }
        if let Some(d) = ahora {
            self.sitios.insert(s, d);
            self.por_sitio.poner(s, d.posicion);
        }
    }

    fn fijo(&mut self, s: Sitio, ahora: Option<Fijo>) {
        if let Some(o) = self.fijos.remove(&s) {
            self.por_fijo.quitar(s, o.centro);
        }
        if let Some(o) = ahora {
            self.fijos.insert(s, o);
            self.por_fijo.poner(s, o.centro);
        }
    }

    /// Los sitios en la caja `[a, b]`.
    fn sitios_en(&self, a: Punto, b: Punto) -> impl Iterator<Item = (Sitio, DatosSitio)> + '_ {
        self.por_sitio.en_caja(a, b).map(|s| (s, self.sitios[&s])).filter(move |(_, d)| (a.x..=b.x).contains(&d.posicion.x) && (a.y..=b.y).contains(&d.posicion.y))
    }

    /// Los sitios a `≤ radio` de `centro`.
    fn sitios_cerca(&self, centro: Punto, radio: Mil) -> Vec<(Sitio, DatosSitio)> {
        self.sitios_en(Punto::new(centro.x - radio, centro.y - radio), Punto::new(centro.x + radio, centro.y + radio)).filter(|(_, d)| cerca(centro, d.posicion, radio)).collect()
    }

    /// Los ojos fijos que ven `p`.
    fn fijos_que_ven(&self, p: Punto) -> Vec<Fijo> {
        self.por_fijo.en_caja(Punto::new(p.x - LADO, p.y - LADO), Punto::new(p.x + LADO, p.y + LADO)).map(|s| self.fijos[&s]).filter(|o| cerca(o.centro, p, o.radio)).collect()
    }
}

fn andante(e: &Ejercito) -> Andante {
    let puntos: Vec<Vertice> = e.plan.vertices.iter().copied().chain(std::iter::once(Vertice { p: e.posicion, en: 0 })).collect();
    Andante { faccion: e.columna.faccion, heroes: e.columna.participantes.iter().map(|p| p.heroe).collect(), plan: e.plan.clone(), quieto: e.posicion, caja: caja(&puntos, 0) }
}

/// ¿Está `p` en la caja agrandada por `radio`?
fn en_caja(c: (Punto, Punto), p: Punto, radio: Mil) -> bool {
    (c.0.x - radio..=c.1.x + radio).contains(&p.x) && (c.0.y - radio..=c.1.y + radio).contains(&p.y)
}

pub(crate) fn cerca(a: Punto, b: Punto, radio: Mil) -> bool {
    let (x, y) = ((a.x - b.x) as i128, (a.y - b.y) as i128);
    x * x + y * y <= radio as i128 * radio as i128
}

/// La caja de un recorrido, agrandada por el alcance: un sitio fuera de ella no se pudo ver.
fn caja(r: &[Vertice], radio: Mil) -> (Punto, Punto) {
    let (mut a, mut b) = (Punto::new(Mil::MAX, Mil::MAX), Punto::new(Mil::MIN, Mil::MIN));
    for v in r {
        a = Punto::new(a.x.min(v.p.x), a.y.min(v.p.y));
        b = Punto::new(b.x.max(v.p.x), b.y.max(v.p.y));
    }
    (Punto::new(a.x.saturating_sub(radio), a.y.saturating_sub(radio)), Punto::new(b.x.saturating_add(radio), b.y.saturating_add(radio)))
}

/// Lo que vio un ojo que anduvo `r` con alcance `radio`: de cada sitio, el último instante en que lo vio sin bajar de desde cuándo vale lo que se ve de él (`desde`: lo de antes ya está grabado).
fn avistados<'a>(r: &'a [Vertice], radio: Mil, sitios: impl Iterator<Item = (Sitio, DatosSitio)> + 'a, desde: &'a BTreeMap<Sitio, Instante>) -> impl Iterator<Item = (Sitio, DatosSitio, Instante)> + 'a {
    let c = caja(r, radio);
    sitios.filter(move |(_, d)| (c.0.x..=c.1.x).contains(&d.posicion.x) && (c.0.y..=c.1.y).contains(&d.posicion.y)).filter_map(move |(s, d)| {
        let tope = desde.get(&s).copied().unwrap_or(Instante::MIN);
        let tramo: Vec<Vertice> = match r.iter().position(|v| v.en >= tope) {
            Some(0) => r.to_vec(),
            Some(i) => std::iter::once(Vertice { p: base::interpolar(r[i - 1], r[i], tope), en: tope }).chain(r[i..].iter().copied()).collect(),
            None => return None,
        };
        ultima_vez(&tramo, d.posicion, radio).map(|t| (s, d, t))
    })
}

/// Lo que se ve de una plaza.
fn datos_de_plaza(a: &crate::estado::Asentamiento) -> DatosSitio {
    DatosSitio { faccion: a.faccion, posicion: a.posicion, nivel: a.nivel(), radio: a.radio, estado: EstadoSitio::Activo }
}

/// Lo que se ve de un pueblo de la Facción `faccion` en `posicion`: activo o, mientras espera materiales o se levanta, en obra.
fn datos_de_pueblo(faccion: u32, posicion: Punto, p: &asentamiento::pueblos::Pueblo) -> DatosSitio {
    DatosSitio { faccion, posicion, nivel: 0, radio: 0, estado: if p.activo() { EstadoSitio::Activo } else { EstadoSitio::EnObra } }
}

/// Lo que se ve de una avanzada de la Facción `faccion` (0: unas ruinas sin dueño): su etapa y, con una obra en marcha (levantarla, mejorarla o repararla), en obra; si no, en ruinas o activa.
fn datos_de_avanzada(faccion: u32, x: &Avanzada) -> DatosSitio {
    let estado = if x.obra.is_some() || x.estado == EstadoAvanzada::EnConstruccion {
        EstadoSitio::EnObra
    } else if x.en_ruinas() {
        EstadoSitio::EnRuinas
    } else {
        EstadoSitio::Activo
    };
    DatosSitio { faccion, posicion: x.posicion, nivel: x.etapa.indice() as u32 + 1, radio: 0, estado }
}

impl Nucleo {
    /// El alcance de vista de una columna ahora: 30 con soldados en pie, 15 sin ellos (§3).
    pub(crate) fn radio_de_vista(&self, e: &Ejercito) -> Mil {
        columna::alcance_de_vista(&self.estado.reglas.economia.militar, self.escuadras_de_columna(e.id))
    }

    /// Lo que se ve desde fuera de un sitio (su ficha): `None` si ya no existe. Unas ruinas de ciudadela sin dueño son de la Facción 0.
    pub(crate) fn datos_de_sitio(&self, s: Sitio) -> Option<DatosSitio> {
        let e = &self.estado;
        match s {
            Sitio::Plaza(id) => e.asentamientos.get(&id).map(datos_de_plaza),
            Sitio::Pueblo(id) => {
                let (madre, p) = self.pueblo(id)?;
                Some(datos_de_pueblo(e.asentamientos.get(&madre)?.faccion, self.posicion_de_pueblo(id)?, p))
            }
            Sitio::Avanzada(id) => {
                let (madre, v) = self.avanzada(id)?;
                let faccion = match madre {
                    Some(m) => e.asentamientos.get(&m)?.faccion,
                    None => 0,
                };
                Some(datos_de_avanzada(faccion, v))
            }
        }
    }

    /// Una plaza y sus pueblos y avanzadas, con lo que se ve de cada uno.
    fn sitios_con_datos(&self, a: &crate::estado::Asentamiento) -> Vec<(Sitio, DatosSitio)> {
        let nodos = self.geografia.as_ref().map_or(&[][..], |g| &g.mapa.nodos[..]);
        let mut v = vec![(Sitio::Plaza(a.id), datos_de_plaza(a))];
        v.extend(a.economia.pueblos.iter().filter_map(|p| Some((Sitio::Pueblo(p.id), datos_de_pueblo(a.faccion, crate::economia::posicion_de_pueblo(nodos, p)?, p)))));
        v.extend(a.economia.avanzadas.iter().map(|x| (Sitio::Avanzada(x.id), datos_de_avanzada(a.faccion, x))));
        v
    }

    /// Los ojos fijos de una plaza: su zona y sus avanzadas operativas.
    fn fijos_de(a: &crate::estado::Asentamiento) -> Vec<(Sitio, Fijo)> {
        let mut v = vec![(Sitio::Plaza(a.id), Fijo { faccion: a.faccion, centro: a.posicion, radio: a.radio })];
        v.extend(a.economia.avanzadas.iter().filter(|x| x.operativa()).map(|x| (Sitio::Avanzada(x.id), Fijo { faccion: a.faccion, centro: x.posicion, radio: vision::radios::AVANZADA })));
        v
    }

    /// ¿Existe el sitio? (Para la vista: de lo que ya no existe se olvida lo recordado al volver a mirar.)
    pub(crate) fn existe_sitio(&self, s: Sitio) -> bool {
        match &self.ojos {
            Some(o) => o.sitios.contains_key(&s),
            None => self.datos_de_sitio(s).is_some(),
        }
    }

    /// Lo que la columna lleva andado sin grabar: su recorrido desde su vigía hasta `hasta` y el alcance con que se grabará. `None` sin vigía (sin rejilla).
    pub(crate) fn recorrido_pendiente(&self, e: &Ejercito, hasta: Instante) -> Option<(Vec<Vertice>, Mil)> {
        let v = self.estado.memoria.vigias.get(&e.id)?;
        Some((recorrido(&e.plan, e.posicion, v.desde, hasta), v.radio))
    }

    /// Lo que se vio de cada sitio en un recorrido aún sin grabar (de [`Nucleo::recorrido_pendiente`]): el sitio, lo que se ve de él y el último instante en que se vio. Es lo que se grabará.
    pub(crate) fn avistados_pendientes(&self, r: &[Vertice], radio: Mil) -> Vec<(Sitio, DatosSitio, Instante)> {
        let Some(o) = &self.ojos else { return Vec::new() };
        let c = caja(r, radio);
        avistados(r, radio, o.sitios_en(c.0, c.1), &self.estado.memoria.sitios).collect()
    }

    /// Los pueblos y las avanzadas de la plaza `a`.
    fn hijos_de(&self, a: u64) -> Vec<Sitio> {
        self.estado.asentamientos.get(&a).map_or_else(Vec::new, |p| p.economia.pueblos.iter().map(|x| Sitio::Pueblo(x.id)).chain(p.economia.avanzadas.iter().map(|x| Sitio::Avanzada(x.id))).collect())
    }

    /// La plaza madre de un pueblo o una avanzada, si existe.
    fn madre_de(&self, s: Sitio) -> Option<u64> {
        match s {
            Sitio::Pueblo(id) => self.pueblo(id).map(|(m, _)| m),
            Sitio::Avanzada(id) => self.avanzada(id).and_then(|(m, _)| m),
            Sitio::Plaza(_) => None,
        }
    }

    /// Empieza a llevar la cuenta de los ojos y los sitios (al construir el núcleo y tras sembrar el mundo). Sin rejilla no hay nada que grabar. Toda columna lleva su vigía (las de una foto ya lo
    /// traen; una sin él graba desde ahora).
    pub(super) fn rehacer_ojos(&mut self) {
        if self.estado.memoria.rejilla.is_none() {
            self.ojos = None;
            return;
        }
        let mut ojos = Ojos {
            columnas: self.estado.ejercitos.values().map(|e| (e.id, andante(e))).collect(),
            fijos: BTreeMap::new(),
            sitios: BTreeMap::new(),
            hijos: self.estado.asentamientos.keys().map(|&a| (a, self.hijos_de(a))).collect(),
            por_sitio: Rejilla::default(),
            por_fijo: Rejilla::default(),
        };
        for a in self.estado.asentamientos.values() {
            self.sitios_con_datos(a).into_iter().for_each(|(s, d)| ojos.sitio(s, Some(d)));
            Self::fijos_de(a).into_iter().for_each(|(s, o)| ojos.fijo(s, Some(o)));
        }
        for x in self.estado.combate.huerfanas.values() {
            ojos.sitio(Sitio::Avanzada(x.id), Some(datos_de_avanzada(0, x)));
        }
        self.ojos = Some(ojos);
        let t = self.estado.ahora;
        let nuevas: Vec<(u64, Mil)> = self.estado.ejercitos.values().filter(|e| !self.estado.memoria.vigias.contains_key(&e.id)).map(|e| (e.id, self.radio_de_vista(e))).collect();
        for (id, radio) in nuevas {
            self.estado.memoria.vigias.insert(id, Vigia { desde: t, radio });
        }
    }

    /// Pone al día la memoria con lo que tocó la transición (se llama al cerrar la versión `t`, antes de subirla).
    pub(super) fn explorar(&mut self, t: Instante) {
        let tocados = &self.tr.tocados;
        if [Clase::Ejercito, Clase::Asentamiento, Clase::Pueblo, Clase::Avanzada, Clase::Heroe].iter().all(|&c| tocados.de(c).is_empty()) {
            return;
        }
        let Some(mut viejos) = self.ojos.take() else { return };
        let mut mem = std::mem::take(&mut self.estado.memoria);
        let (columnas_rejilla, filas_rejilla, lado) = mem.rejilla.expect("hay ojos solo con rejilla");
        let vacia = || vision::Mascara::nueva(columnas_rejilla, filas_rejilla, lado);

        // Qué sitios y qué ojos fijos pueden haber cambiado: los de las plazas tocadas (la plaza, sus pueblos y sus avanzadas, de antes y de ahora: cambian de dueño y desaparecen con ella) y los
        // pueblos y avanzadas tocados, que se miran con su plaza madre.
        let mut plazas: BTreeSet<u64> = tocados.de(Clase::Asentamiento).iter().copied().collect();
        let sueltos: Vec<Sitio> = tocados.de(Clase::Pueblo).iter().map(|&p| Sitio::Pueblo(p)).chain(tocados.de(Clase::Avanzada).iter().map(|&v| Sitio::Avanzada(v))).collect();
        plazas.extend(sueltos.iter().filter_map(|&s| self.madre_de(s)));
        let (mut nuevos_sitios, mut nuevos_fijos): (BTreeMap<Sitio, DatosSitio>, BTreeMap<Sitio, Fijo>) = (BTreeMap::new(), BTreeMap::new());
        let mut sitios: BTreeSet<Sitio> = sueltos.into_iter().collect();
        for &a in &plazas {
            sitios.insert(Sitio::Plaza(a));
            sitios.extend(viejos.hijos.get(&a).into_iter().flatten().copied());
            match self.estado.asentamientos.get(&a) {
                Some(p) => {
                    nuevos_sitios.extend(self.sitios_con_datos(p));
                    nuevos_fijos.extend(Self::fijos_de(p));
                    viejos.hijos.insert(a, self.hijos_de(a));
                }
                None => {
                    viejos.hijos.remove(&a);
                }
            }
        }
        sitios.extend(nuevos_sitios.keys().copied());
        // Lo que ya no es de ninguna plaza tocada (las ruinas de la ciudadela de una plaza que cae, que quedan sin dueño) se mira suelto.
        for &s in &sitios {
            if !nuevos_sitios.contains_key(&s) {
                if let Some(d) = self.datos_de_sitio(s) {
                    nuevos_sitios.insert(s, d);
                }
            }
        }
        let cambiados: Vec<(Sitio, Option<DatosSitio>, Option<DatosSitio>)> =
            sitios.iter().map(|&s| (s, viejos.sitios.get(&s).copied(), nuevos_sitios.get(&s).copied())).filter(|(_, antes, ahora)| antes != ahora).collect();
        let fijos: Vec<(Sitio, Option<Fijo>, Option<Fijo>)> =
            sitios.iter().filter(|s| !matches!(s, Sitio::Pueblo(_))).map(|&s| (s, viejos.fijos.get(&s).copied(), nuevos_fijos.get(&s).copied())).filter(|(_, antes, ahora)| antes != ahora).collect();

        // 1. Lo que cada ojo ajeno vio de un sitio que cambia, con lo de antes; desde ahora su ficha vale lo nuevo.
        for &(s, antes, ahora) in &cambiados {
            if let Some(d) = antes {
                let tope = mem.sitios.get(&s).copied().unwrap_or(Instante::MIN);
                for (id, ojo) in viejos.columnas.iter().filter(|(_, o)| o.faccion.is_some_and(|f| f != d.faccion)) {
                    let (Some(f), Some(v)) = (ojo.faccion, mem.vigias.get(id)) else { continue };
                    if !en_caja(ojo.caja, d.posicion, v.radio) {
                        continue;
                    }
                    if let Some(en) = ultima_vez(&recorrido(&ojo.plan, ojo.quieto, v.desde.max(tope), t), d.posicion, v.radio) {
                        mem.anotar(f, s, d, en);
                    }
                }
                for o in viejos.fijos_que_ven(d.posicion) {
                    mem.anotar(o.faccion, s, d, t);
                }
            }
            match ahora {
                Some(_) => mem.sitios.insert(s, t),
                None => mem.sitios.remove(&s),
            };
            viejos.sitio(s, ahora);
        }

        // 2. Las columnas tocadas barren lo que anduvieron con lo que eran (también la que desaparece) y ven los sitios de su recorrido.
        let columnas: Vec<u64> = tocados.de(Clase::Ejercito).to_vec();
        for &id in &columnas {
            let (Some(ojo), Some(v)) = (viejos.columnas.get(&id), mem.vigias.get(&id).copied()) else { continue };
            let r = recorrido(&ojo.plan, ojo.quieto, v.desde, t);
            match ojo.faccion {
                Some(f) => {
                    mem.exploracion.entry(f).or_insert_with(vacia).barrer(&r, v.radio);
                    let c = caja(&r, v.radio);
                    let vistos: Vec<_> = avistados(&r, v.radio, viejos.sitios_en(c.0, c.1), &mem.sitios).collect();
                    for (s, d, en) in vistos {
                        mem.anotar(f, s, d, en);
                    }
                    // Lo recordado de un sitio que ya no existe (su ficha, su informe) se olvida si se vuelve a mirar su sitio después.
                    let olvidar: Vec<Sitio> =
                        mem.recordado(f).into_iter().filter(|&(s, p, en)| !viejos.sitios.contains_key(&s) && ultima_vez(&r, p, v.radio).is_some_and(|visto| visto > en)).map(|(s, _, _)| s).collect();
                    for s in olvidar {
                        mem.olvidar(f, s);
                    }
                }
                // Sin bandera, cada héroe lo guarda en su Facción si la tiene y si no en lo suyo (§4.2).
                None => {
                    for &h in &ojo.heroes {
                        let m = match self.estado.militar.heroes.get(&h).and_then(|x| x.faccion) {
                            Some(f) => mem.exploracion.entry(f).or_insert_with(vacia),
                            None => mem.personal.entry(h).or_insert_with(vacia),
                        };
                        m.barrer(&r, v.radio);
                    }
                }
            }
        }

        // 3. El ojo fijo que cambia (nace, crece, cambia de dueño o cae) vio hasta ahora todo lo de su disco de antes; el de ahora explora su disco.
        for &(s, antes, ahora) in &fijos {
            if let Some(o) = antes {
                for (x, d) in viejos.sitios_cerca(o.centro, o.radio) {
                    mem.anotar(o.faccion, x, d, t);
                }
            }
            if let Some(o) = ahora {
                mem.exploracion.entry(o.faccion).or_insert_with(vacia).marcar_vision(o.centro, o.radio);
            }
            viejos.fijo(s, ahora);
        }

        // 4. Lo que se ve en este instante: de un sitio que cambió, lo de ahora (o se olvida, si desapareció); desde un ojo fijo nuevo, lo que cae en su disco.
        if !cambiados.is_empty() || !fijos.is_empty() {
            let columnas_ven: Vec<(u32, Punto, Mil)> =
                if cambiados.is_empty() { Vec::new() } else { self.estado.ejercitos.values().filter_map(|e| Some((e.columna.faccion?, e.posicion_en(t), self.radio_de_vista(e)))).collect() };
            for &(s, antes, ahora) in &cambiados {
                let Some(p) = ahora.or(antes).map(|d| d.posicion) else { continue };
                let fijos_ven = viejos.fijos_que_ven(p).into_iter().map(|o| (o.faccion, o.centro, o.radio));
                for (f, _, _) in columnas_ven.iter().copied().filter(|(_, c, r)| cerca(*c, p, *r)).chain(fijos_ven) {
                    match ahora {
                        Some(d) => mem.anotar(f, s, d, t),
                        None => mem.olvidar(f, s),
                    }
                }
                // Un sitio que pasa a ser de la Facción ya no es ajeno: su ficha sobra.
                if let Some(d) = ahora {
                    mem.olvidar(d.faccion, s);
                }
            }
            for o in fijos.iter().filter_map(|&(_, _, ahora)| ahora) {
                for (x, d) in viejos.sitios_cerca(o.centro, o.radio) {
                    mem.anotar(o.faccion, x, d, t);
                }
                let olvidar: Vec<Sitio> = mem.recordado(o.faccion).into_iter().filter(|&(x, p, _)| !viejos.sitios.contains_key(&x) && cerca(o.centro, p, o.radio)).map(|(x, _, _)| x).collect();
                for x in olvidar {
                    mem.olvidar(o.faccion, x);
                }
            }
        }

        // 5. Cada columna tocada graba desde ahora, con el alcance que tiene; la que desaparece deja de tener vigía.
        for &id in &columnas {
            match self.estado.ejercitos.get(&id) {
                Some(e) => {
                    mem.vigias.insert(id, Vigia { desde: t, radio: self.radio_de_vista(e) });
                    viejos.columnas.insert(id, andante(e));
                }
                None => {
                    mem.vigias.remove(&id);
                    viejos.columnas.remove(&id);
                }
            }
        }

        // 6. Un héroe que entra en una Facción le da lo que exploró sin bandera (§4.3); el que ya no existe se lo lleva.
        for &h in tocados.de(Clase::Heroe) {
            match self.estado.militar.heroes.get(&h).map(|x| x.faccion) {
                Some(Some(f)) => {
                    if let Some(p) = mem.personal.remove(&h) {
                        mem.exploracion.entry(f).or_insert_with(vacia).fundir(&p);
                    }
                }
                Some(None) => {}
                None => {
                    mem.personal.remove(&h);
                }
            }
        }

        self.estado.memoria = mem;
        self.ojos = Some(viejos);
    }
}

impl Memoria {
    /// Anota en la Facción `faccion` la ficha del sitio `s` vista en `en`, si es ajeno y no tiene ya una más reciente (en un mismo instante gana la última que se anota).
    pub(crate) fn anotar(&mut self, faccion: u32, s: Sitio, datos: DatosSitio, en: Instante) {
        if datos.faccion == faccion {
            return;
        }
        let de = self.fichas.entry(faccion).or_default();
        if de.get(&s).is_none_or(|f| en >= f.conocido_en) {
            de.insert(s, Ficha { datos, conocido_en: en });
        }
    }

    /// Borra lo que la Facción `faccion` recuerda del sitio `s`: su ficha y, de una plaza, su Informe.
    pub(crate) fn olvidar(&mut self, faccion: u32, s: Sitio) {
        if let Some(de) = self.fichas.get_mut(&faccion) {
            de.remove(&s);
            if de.is_empty() {
                self.fichas.remove(&faccion);
            }
        }
        if let (Sitio::Plaza(id), Some(de)) = (s, self.informes.get_mut(&faccion)) {
            de.remove(&id);
            if de.is_empty() {
                self.informes.remove(&faccion);
            }
        }
    }

    /// Lo que la Facción `f` recuerda de cada sitio, con su posición y desde cuándo: sus fichas y sus Informes de plaza (§4.1: los dos se olvidan si se vuelve a mirar el sitio y ya no está).
    pub(crate) fn recordado(&self, f: u32) -> Vec<(Sitio, Punto, Instante)> {
        let fichas = self.fichas.get(&f).into_iter().flatten().map(|(&s, x)| (s, x.datos.posicion, x.conocido_en));
        fichas.chain(self.informes.get(&f).into_iter().flatten().map(|(&id, i)| (Sitio::Plaza(id), i.posicion, i.conocido_en))).collect()
    }
}
