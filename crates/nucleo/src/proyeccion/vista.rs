//! El ámbito `VISTA` (contrato 05 §3, 4H T3d): el mundo que ve una Facción —o un héroe sin bandera— dentro del rectángulo de `CambiarVista`, filtrado por la niebla (Niebla_De_Guerra §3-§5).
//!
//! - [`Mirada::de`] reúne lo que se ve desde una perspectiva en un instante: los ojos propios y los de los aliados (alianza o vasallaje, cualquier dirección: §5.2), las dos máscaras de la
//!   niebla (`celdas`, lo explorado con lo que lleva andado cada columna propia sin grabar; `visibles`, lo que alcanzan ahora todos los ojos) y las fichas de lo que se recuerda (las de la memoria
//!   más las que dejará lo andado).
//! - [`Mirada::contenido`] dice qué entidades tiene que tener el cliente, cada una con su [`Sello`] (lo que, si cambia, obliga a mandarla otra vez) y su cuerpo, que solo se calcula si hace falta.
//!   Lo **propio** (las columnas de la Facción y en las que va la cuenta, las caravanas de sus plazas o con destino en ellas, sus plazas, pueblos, avanzadas y obras) viaja como en `FACCION` y `HEROE`;
//!   lo **ajeno** que se ve, redactado (§5.4, D15): la columna sin composición y solo con el tramo del plan al alcance de los ojos que la ven y dentro de un horizonte de 30 s (que el muestreo
//!   renueva cuando le quedan menos de 10 s, decisión 6; nunca el destino), la caravana sin cantidades, la plaza como ficha con su zona recortada y sus edificios activos, el pueblo y la avanzada sin almacén ni guarnición,
//!   el héroe solo con lo público (`HeroePublico`); lo **recordado** (un sitio ajeno que se vio y ya no se ve), como `FichaConocida`. Además: los campamentos de bandidos que se ven, los de
//!   mercenarios explorados (o donde reside o está), las batallas que se ven o en que está su Facción (como las ve él: `propios::batalla`), la zona de la Facción, los caminos explorados y la
//!   intel de la taberna: las Miradas de la Facción (abiertas o enfriando su zona) y sus Informes de plaza. Una Mirada abierta, suya o de un aliado, es un ojo prestado: ve, no explora.
//!
//! El tramo de lo ajeno se recorta a la unión de los discos de los ojos (decisión del autor, 2026-10-23: no al rectángulo de la vista, que enseñaría camino fuera del alcance); una caravana
//! grande que solo se ve por llamar la atención, a sus 60 u. La columna se retira en el muestreo siguiente a que deje de verse.

use super::propios::{self, Proyectada};
use super::{ficha_de_asentamiento, ids, interior, plan, punto};
use crate::estado::{Clase, Ejercito};
use crate::estado_militar::Relacion;
use crate::memoria::{DatosSitio, EstadoSitio, Ficha, InformeDePlaza, Mirada as MiradaIntel, OrigenIntel, Sitio};
use crate::nucleo::Nucleo;
use asentamiento::edificio::Estado as EstadoEdificio;
use base::{Instante, Mil, Plan, Punto, Vertice};
use comercio::caravana::{Caravana, EstadoCaravana};
use contrato::v1 as pb;
use militar::escuadra::Contenedor;
use militar::presencia::Ubicacion;
use pb::entidad::Cuerpo;
use std::collections::{BTreeMap, BTreeSet};
use vision::{radios, tramo_visible, Mascara, Rect};

/// Un ojo: su centro y su alcance.
type Ojo = (Punto, Mil);

/// Desde dónde se mira: la Facción del héroe o, sin bandera, el héroe solo (su niebla personal). Es el id del ámbito (`VISTA:f-N`, `VISTA:heroe-N`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Perspectiva {
    Faccion(u32),
    Heroe(u64),
}

/// La identidad de una entidad en el almacén del cliente: su clase del contrato y su id.
pub type Clave = (i32, String);

/// Lo que decide si una entidad que el cliente ya tiene hay que mandarla otra vez: su versión y un dato propio de la vista (dónde acaba el tramo que se le mandó de una columna ajena, cuándo
/// se vio un sitio recordado...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sello(pub u64, pub i64);

/// Una entidad de la vista: su clave, su sello y su cuerpo (que solo se calcula si hay que mandarla).
pub struct Visible<'a> {
    pub clave: Clave,
    pub sello: Sello,
    cuerpo: Box<dyn Fn() -> Option<Cuerpo> + 'a>,
}

impl Visible<'_> {
    pub fn cuerpo(&self) -> Option<Cuerpo> {
        (self.cuerpo)()
    }
}

fn visible<'a>(tipo: pb::TipoEntidad, id: String, sello: Sello, cuerpo: impl Fn() -> Option<Cuerpo> + 'a) -> Visible<'a> {
    Visible { clave: (tipo as i32, id), sello, cuerpo: Box::new(cuerpo) }
}

/// Lo que se ve desde una perspectiva en un instante.
pub struct Mirada<'n> {
    n: &'n Nucleo,
    pub perspectiva: Perspectiva,
    faccion: Option<u32>,
    t: Instante,
    /// Todas las columnas con su posición en `t`: se calcula una vez por perspectiva y no una por sesión (4H T6).
    columnas: Vec<(&'n Ejercito, Punto)>,
    /// Los ojos propios y los de los aliados en `t`: centro y alcance.
    ojos: Vec<Ojo>,
    /// Las columnas propias: las de la Facción o, sin bandera, en las que va el héroe.
    propias: BTreeSet<u64>,
    /// Lo explorado (con lo que lleva andado cada columna propia). `None` sin rejilla de niebla.
    celdas: Option<Mascara>,
    /// Las dos máscaras que viajan: lo explorado y lo visible ahora.
    niebla: Option<pb::NieblaProyectada>,
    /// Lo que se recuerda de cada sitio ajeno (memoria más lo andado sin grabar).
    fichas: BTreeMap<Sitio, Ficha>,
    /// Los Informes de plaza que conserva la Facción (sin los de plazas que ya no existen y se han vuelto a mirar).
    informes: BTreeMap<u64, &'n InformeDePlaza>,
}

/// Las Facciones que comparten visión con `f` (alianza o vasallaje, en cualquier dirección: §5.2).
fn aliados(n: &Nucleo, f: u32) -> Vec<u32> {
    n.estado().militar.relaciones.iter().filter(|(_, r)| matches!(r, Relacion::Alianza | Relacion::Vasallaje)).filter_map(|(&(a, b), _)| if a == f { Some(b) } else if b == f { Some(a) } else { None }).collect()
}

/// La cuenta de `viewer`, el héroe de la sesión de su cuenta (`audiencia::heroe_de_sesion`).
fn cuenta_de(n: &Nucleo, viewer: u64) -> Option<&str> {
    n.estado().militar.heroes.get(&viewer).map(|h| h.cuenta.as_str())
}

/// ¿Va algún héroe de la `cuenta` en la columna? Para el héroe de la sesión de esa cuenta es lo mismo que preguntar si él es el héroe de la sesión de algún participante (`audiencia::principal`),
/// sin buscarlo: el de un héroe de campo se busca entre todos los héroes, y esto se pregunta por cada columna ajena del rectángulo en cada muestreo de cada sesión (4H T6).
fn va_en(n: &Nucleo, e: &Ejercito, cuenta: &str) -> bool {
    let est = n.estado();
    e.columna.participantes.iter().filter_map(|p| est.militar.heroes.get(&p.heroe)).any(|h| h.cuenta == cuenta)
}

/// Anota una ficha si no hay una más reciente (en un mismo instante gana la última), como `Memoria::anotar`.
fn anotar(fichas: &mut BTreeMap<Sitio, Ficha>, faccion: u32, s: Sitio, datos: DatosSitio, en: Instante) {
    if datos.faccion != faccion && fichas.get(&s).is_none_or(|f| en >= f.conocido_en) {
        fichas.insert(s, Ficha { datos, conocido_en: en });
    }
}

impl<'n> Mirada<'n> {
    pub fn de(n: &'n Nucleo, perspectiva: Perspectiva) -> Self {
        let (e, t) = (n.estado(), n.ahora());
        let mem = &e.memoria;
        let faccion = match perspectiva {
            Perspectiva::Faccion(f) => Some(f),
            Perspectiva::Heroe(_) => None,
        };
        let propias: BTreeSet<u64> = e
            .ejercitos
            .values()
            .filter(|x| match perspectiva {
                Perspectiva::Faccion(f) => x.columna.faccion == Some(f),
                Perspectiva::Heroe(h) => cuenta_de(n, h).is_some_and(|c| va_en(n, x, c)),
            })
            .map(|x| x.id)
            .collect();
        let grupo: Vec<u32> = faccion.into_iter().chain(faccion.map(|f| aliados(n, f)).unwrap_or_default()).collect();
        let columnas: Vec<(&Ejercito, Punto)> = e.ejercitos.values().map(|x| (x, x.posicion_en(t))).collect();
        let mut ojos: Vec<Ojo> = columnas.iter().filter(|(x, _)| propias.contains(&x.id) || x.columna.faccion.is_some_and(|f| grupo.contains(&f))).map(|&(x, p)| (p, n.radio_de_vista(x))).collect();
        for a in e.asentamientos.values().filter(|a| grupo.contains(&a.faccion)) {
            ojos.push((a.posicion, a.radio));
            ojos.extend(a.economia.avanzadas.iter().filter(|v| v.operativa()).map(|v| (v.posicion, radios::AVANZADA)));
        }
        // Las Miradas abiertas de la Facción y de sus aliados: ven, pero lo que ven no se explora (no entran en `celdas`).
        let prestados = ojos.len();
        ojos.extend(mem.miradas.iter().filter(|m| grupo.contains(&m.faccion) && m.activa(t)).map(|m| (m.centro, m.radio)));

        // Lo explorado: la memoria de la Facción (o la del héroe sin bandera) más lo que lleva andado cada columna propia, que es lo que se grabará.
        let explorado = match perspectiva {
            Perspectiva::Faccion(f) => mem.exploracion.get(&f),
            Perspectiva::Heroe(h) => mem.personal.get(&h),
        };
        let celdas = mem.vacia().map(|vacia| {
            let mut m = explorado.cloned().unwrap_or(vacia);
            for x in propias.iter().filter_map(|id| e.ejercitos.get(id)) {
                if let Some((r, radio)) = n.recorrido_pendiente(x, t) {
                    m.barrer(&r, radio);
                }
            }
            m
        });
        let visibles = mem.vacia().map(|mut m| {
            for &(p, r) in &ojos {
                m.marcar_vision(p, r);
            }
            m
        });

        // Las fichas: las de la memoria más lo que las columnas propias vieron sin grabar; lo recordado de un sitio que ya no existe se olvida si se ha vuelto a mirar su sitio (o se mira ahora).
        let mut fichas = faccion.and_then(|f| mem.fichas.get(&f)).cloned().unwrap_or_default();
        let mut informes: BTreeMap<u64, &InformeDePlaza> = faccion.and_then(|f| mem.informes.get(&f)).into_iter().flatten().map(|(&id, i)| (id, i)).collect();
        if let Some(f) = faccion {
            let olvidado = |s: Sitio, p: Punto, en: Instante, r: &[Vertice], radio: Mil| !n.existe_sitio(s) && vision::ultima_vez(r, p, radio).is_some_and(|visto| visto > en);
            for x in propias.iter().filter_map(|id| e.ejercitos.get(id)) {
                let Some((r, radio)) = n.recorrido_pendiente(x, t) else { continue };
                for (s, d, en) in n.avistados_pendientes(&r, radio) {
                    anotar(&mut fichas, f, s, d, en);
                }
                fichas.retain(|&s, x| !olvidado(s, x.datos.posicion, x.conocido_en, &r, radio));
                informes.retain(|&id, i| !olvidado(Sitio::Plaza(id), i.posicion, i.conocido_en, &r, radio));
            }
            // Mirar ahora el sitio de lo que ya no existe también lo olvida (los ojos prestados no: la visión prestada no deja ni quita memoria).
            let propios_ojos = &ojos[..prestados];
            let a_la_vista = |p: Punto| propios_ojos.iter().any(|&(c, r)| crate::nucleo::cerca(c, p, r));
            fichas.retain(|&s, x| n.existe_sitio(s) || !a_la_vista(x.datos.posicion));
            informes.retain(|&id, i| n.existe_sitio(Sitio::Plaza(id)) || !a_la_vista(i.posicion));
        }
        let niebla = celdas.as_ref().zip(visibles.as_ref()).map(|(c, v)| vision::niebla_proyectada(c, v));
        Mirada { n, perspectiva, faccion, t, columnas, ojos, propias, celdas, niebla, fichas, informes }
    }

    /// ¿Se ve ahora el punto? (alguno de los ojos a `≤` su alcance, no estricto).
    pub fn ve(&self, p: Punto) -> bool {
        self.ojos.iter().any(|&(c, r)| crate::nucleo::cerca(c, p, r))
    }

    /// Las dos máscaras de la niebla (`None` sin rejilla).
    pub fn niebla(&self) -> Option<&pb::NieblaProyectada> {
        self.niebla.as_ref()
    }

    /// Lo que el cliente de `viewer` (el héroe de la sesión) tiene que tener con su rectángulo. `tiene` es lo que ya tiene (con su sello): una columna ajena cuyo tramo aún le dura 10 s o más
    /// conserva el que tiene, de modo que no se le manda nada.
    pub fn contenido(&self, viewer: Option<u64>, rect: Rect, tiene: &BTreeMap<Clave, Sello>) -> Vec<Visible<'_>> {
        let (n, t) = (self.n, self.t);
        let e = n.estado();
        let mut v: Vec<Visible<'_>> = Vec::new();
        let cuenta = viewer.and_then(|h| cuenta_de(n, h));
        let propia = |x: &Ejercito| self.propias.contains(&x.id) || cuenta.is_some_and(|c| va_en(n, x, c));

        // Columnas: las propias, enteras; las ajenas que se ven, con su tramo (sin las que una batalla aparta: la batalla las sustituye).
        let mut heroes: BTreeSet<u64> = BTreeSet::new();
        for &(x, p) in &self.columnas {
            if !rect.contiene(p) {
                continue;
            }
            let id = ids::ejercito(x.id);
            if propia(x) {
                v.push(visible(pb::TipoEntidad::Ejercito, id, Sello(x.version, 0), move || Some(Cuerpo::Ejercito(propios::ejercito(n, x)))));
            } else if !x.en_batalla && self.ve(p) {
                let tramo = self.tramo(&x.plan, x.version, tiene.get(&(pb::TipoEntidad::Ejercito as i32, id.clone())), &self.ojos);
                let sello = Sello(x.version, tramo.as_ref().map_or_else(|v| *v, |t| t.last().map_or(0, |v| v.en)));
                v.push(visible(pb::TipoEntidad::Ejercito, id, sello, move || Some(Cuerpo::Ejercito(self.ejercito_avistado(x)))));
            } else {
                continue;
            }
            heroes.extend(x.columna.participantes.iter().map(|q| q.heroe));
        }

        // Caravanas que no están en casa ni en una batalla: las propias (de una plaza de la Facción o con destino en ella), enteras; las ajenas, si se ven o si una grande llama la atención.
        let flota = &e.reglas.economia.comercio.flota;
        let de_la_faccion = |plaza: Option<u32>| plaza.and_then(|p| e.asentamientos.get(&u64::from(p))).is_some_and(|a| Some(a.faccion) == self.faccion);
        for c in e.comercio.flota.iter().filter(|c| !matches!(c.estado, EstadoCaravana::Disponible | EstadoCaravana::EnBatalla)) {
            let Some(p) = n.posicion_de_caravana(c.id, t) else { continue };
            if !rect.contiene(p) {
                continue;
            }
            let id = ids::caravana(c.id);
            let version = n.version_de(Clase::Caravana, c.id);
            if de_la_faccion(Some(c.origen)) || de_la_faccion(c.destino) || propias_adjunta(e, c, &self.propias) {
                v.push(visible(pb::TipoEntidad::Caravana, id, Sello(version, 0), move || Some(Cuerpo::Caravana(propios::caravana(n, c)))));
                continue;
            }
            let atencion = c.es_grande(flota) && matches!(c.estado, EstadoCaravana::Preparando | EstadoCaravana::EnTransito) && self.ojos.iter().any(|&(o, _)| crate::nucleo::cerca(o, p, radios::CARAVANA_GRANDE));
            if self.ve(p) || atencion {
                let plan = c.plan.clone().filter(|_| matches!(c.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando)).unwrap_or_default();
                // La caravana grande llama la atención a 60 u: hasta ahí se ve su tramo.
                let grande: Option<Vec<Ojo>> = atencion.then(|| self.ojos.iter().map(|&(o, r)| (o, r.max(radios::CARAVANA_GRANDE))).collect());
                let tramo = self.tramo(&plan, version, tiene.get(&(pb::TipoEntidad::Caravana as i32, id.clone())), grande.as_deref().unwrap_or(&self.ojos));
                let sello = Sello(version, tramo.as_ref().map_or_else(|v| *v, |t| t.last().map_or(0, |v| v.en)));
                v.push(visible(pb::TipoEntidad::Caravana, id, sello, move || Some(Cuerpo::Caravana(self.caravana_avistada(c, p, &plan, grande.as_deref().unwrap_or(&self.ojos))))));
            }
        }

        // Los sitios: lo propio como en `FACCION`; lo ajeno que se ve, redactado; lo ajeno que no se ve y se recuerda, como ficha.
        let mut vistos: BTreeSet<Sitio> = BTreeSet::new();
        for a in e.asentamientos.values() {
            let duena = Some(a.faccion) == self.faccion;
            if rect.contiene(a.posicion) && (duena || self.ve(a.posicion)) {
                vistos.insert(Sitio::Plaza(a.id));
                let id = ids::asentamiento(a.id);
                if duena {
                    v.extend(self.propias_de(Clase::Asentamiento, a.id, a.version));
                } else {
                    v.push(visible(pb::TipoEntidad::Asentamiento, id, Sello(a.version, 0), move || Some(Cuerpo::Asentamiento(plaza_avistada(n, a.id)))));
                }
            }
            for p in &a.economia.pueblos {
                let Some(pos) = n.posicion_de_pueblo(p.id) else { continue };
                if rect.contiene(pos) && (duena || self.ve(pos)) {
                    vistos.insert(Sitio::Pueblo(p.id));
                    let version = n.version_de(Clase::Pueblo, p.id);
                    v.extend(if duena { self.propias_de(Clase::Pueblo, p.id, version) } else { self.pueblo_avistado(a.id, p, pos, version) });
                }
            }
            for x in &a.economia.avanzadas {
                if rect.contiene(x.posicion) && (duena || self.ve(x.posicion)) {
                    vistos.insert(Sitio::Avanzada(x.id));
                    let version = n.version_de(Clase::Avanzada, x.id);
                    v.extend(if duena { self.propias_de(Clase::Avanzada, x.id, version) } else { self.avanzada_avistada(Some(a.id), x, version) });
                }
            }
        }
        for x in n.avanzadas_huerfanas().filter(|x| rect.contiene(x.posicion) && self.ve(x.posicion)) {
            vistos.insert(Sitio::Avanzada(x.id));
            v.extend(self.avanzada_avistada(None, x, n.version_de(Clase::Avanzada, x.id)));
        }
        for (&s, &f) in self.fichas.iter().filter(|(s, f)| !vistos.contains(s) && rect.contiene(f.datos.posicion)) {
            let id = match s {
                Sitio::Plaza(x) => ids::asentamiento(x),
                Sitio::Pueblo(x) => ids::pueblo(x),
                Sitio::Avanzada(x) => ids::avanzada(x),
            };
            let d = f.datos;
            let sello = Sello(u64::from(d.faccion) << 32 | u64::from(d.nivel) << 2 | d.estado as u64, f.conocido_en ^ d.radio.rotate_left(20));
            v.push(visible(pb::TipoEntidad::FichaConocida, id.clone(), sello, move || Some(Cuerpo::FichaConocida(ficha_conocida(s, id.clone(), f)))));
        }

        // La intel de la taberna: las Miradas de la Facción mientras enfrían su zona (las de los aliados solo prestan sus ojos) y sus Informes de plaza, con su fecha.
        if let Some(f) = self.faccion {
            for m in e.memoria.miradas.iter().filter(|m| m.faccion == f && m.libre_en > t && rect.contiene(m.centro)) {
                v.push(visible(pb::TipoEntidad::MiradaIntel, ids::mirada(m.id), Sello(m.id, m.libre_en), move || Some(Cuerpo::Mirada(mirada_intel(m)))));
            }
            for (&id, &i) in self.informes.iter().filter(|(_, i)| rect.contiene(i.posicion)) {
                v.push(visible(pb::TipoEntidad::InformePlaza, ids::asentamiento(id), Sello(0, i.conocido_en), move || Some(Cuerpo::Informe(informe_de_plaza(id, i)))));
            }
        }

        // Campamentos de bandidos que se ven ahora (sin memoria: aparecen y desaparecen).
        for b in e.combate.bandidos.values().filter(|b| rect.contiene(b.posicion) && self.ve(b.posicion)) {
            v.push(visible(pb::TipoEntidad::CampamentoBandidos, ids::bandidos(b.id), Sello(n.version_de(Clase::Bandidos, b.id), 0), move || Some(Cuerpo::CampamentoBandidos(propios::campamento_bandidos(n, b)))));
        }

        // Campamentos de mercenarios explorados, o donde el héroe reside o está.
        let suyo = |c: u32| viewer.and_then(|h| n.heroe(h)).is_some_and(|h| h.heroe.presencia.ubicacion == Ubicacion::Mercenarios(c) || h.heroe.residencia == Some(militar::heroe::Residencia::Campamento(c)));
        for (c, puerta) in n.puertas_de_campamentos() {
            if rect.contiene(puerta) && (suyo(c) || self.celdas.as_ref().is_some_and(|m| m.explorado_en(puerta))) {
                v.push(visible(pb::TipoEntidad::CampamentoMercenarios, ids::campamento(c), Sello(n.version_de(Clase::Campamento, u64::from(c)), 0), move || interior::campamento(n, c).map(Cuerpo::CampamentoMercenarios)));
            }
        }

        // Batallas activas que se ven o en que está su Facción, como las ve él (entera si combate; si no, resumida con el lado al que podría unirse).
        for b in n.batallas_activas() {
            let suya = self.faccion.is_some_and(|f| [b.ticket.atacante.faccion, b.ticket.defensor.faccion].contains(&Some(f)));
            if rect.contiene(b.punto) && (suya || self.ve(b.punto)) {
                v.push(visible(pb::TipoEntidad::Batalla, n.uuid_de_batalla(b.id), Sello(n.version_de(Clase::Batalla, b.id), 0), move || Some(Cuerpo::Batalla(propios::batalla(n, b, propios::Espectador::Heroe(viewer))))));
            }
        }

        // Héroes ajenos: los que van en una columna de la vista y los que están dentro de la plaza donde está él; solo lo público. El suyo está entero en `HEROE`.
        if let Some(Ubicacion::Asentamiento(a)) = viewer.and_then(|h| n.heroe(h)).map(|h| h.heroe.presencia.ubicacion) {
            heroes.extend(n.heroes().filter(|h| h.heroe.presencia.ubicacion == Ubicacion::Asentamiento(a)).map(|h| h.heroe.id));
        }
        for h in heroes.into_iter().filter(|&h| Some(h) != viewer).filter_map(|h| n.heroe(h)) {
            let version = h.heroe.escuadras.iter().map(|s| n.version_de(Clase::Escuadra, s.id)).fold(n.version_de(Clase::Heroe, h.heroe.id), u64::max);
            v.push(visible(pb::TipoEntidad::Heroe, ids::heroe(h.heroe.id), Sello(version, 0), move || Some(Cuerpo::Heroe(heroe_publico(n, h.heroe.id)))));
        }

        // La zona de la Facción: el polígono recortado de cada una de sus plazas.
        if let Some(f) = self.faccion {
            let plazas: Vec<&crate::estado::Asentamiento> = n.plazas_de(f).collect();
            if !plazas.is_empty() {
                let version = plazas.iter().map(|a| a.version).max().unwrap_or(0);
                v.push(visible(pb::TipoEntidad::ZonaFaccion, ids::faccion(f), Sello(version, plazas.len() as i64), move || {
                    Some(Cuerpo::ZonaFaccion(pb::ZonaFaccion { faccion_id: ids::faccion(f), contornos: n.plazas_de(f).map(|a| pb::ContornoZona { puntos: n.poligono_zona(a.id).into_iter().map(punto).collect() }).collect() }))
                }));
            }
        }

        // Los caminos de la red con algún extremo explorado y alguno dentro del rectángulo (las aristas no salen nunca de la red y lo explorado solo crece: cuántas son dice si cambió).
        if let Some(m) = &self.celdas {
            let aristas: Vec<String> = e.comercio.red.aristas().filter(|a| arista_en(a, |p| m.explorado_en(p)) && arista_en(a, |p| rect.contiene(p))).map(id_de_arista).collect();
            if !aristas.is_empty() {
                let cuantas = aristas.len() as i64;
                v.push(visible(pb::TipoEntidad::RedCaminos, "red".into(), Sello(0, cuantas), move || Some(Cuerpo::RedCaminos(pb::RedCaminos { version: 0, aristas: aristas.clone(), rutas: Vec::new() }))));
            }
        }
        v
    }

    /// El tramo de un plan ajeno que toca mandar: el que el cliente ya tiene (`Err(dónde acaba)`) si es de la misma versión y le dura aún [`radios::RENOVAR_MS`] o más, o el plan llega a su fin
    /// dentro de él; si no, el tramo nuevo al alcance de los ojos y dentro del horizonte de 30 s.
    fn tramo(&self, plan: &Plan, version: u64, tiene: Option<&Sello>, ojos: &[Ojo]) -> Result<Vec<Vertice>, i64> {
        if plan.vertices.is_empty() {
            return Ok(Vec::new());
        }
        if let Some(&Sello(v, hasta)) = tiene {
            if v == version && (hasta - self.t >= radios::RENOVAR_MS || plan.fin().is_some_and(|f| f <= hasta)) {
                return Err(hasta);
            }
        }
        Ok(tramo_visible(plan, self.t, radios::HORIZONTE_MS, ojos))
    }

    /// Lo propio de una clase (las mismas entidades que en `FACCION`: la ficha de la plaza; el pueblo, la avanzada y sus obras).
    fn propias_de(&self, clase: Clase, id: u64, version: u64) -> Vec<Visible<'_>> {
        let n = self.n;
        propios::proyectar(n, propios::Espectador::Heroe(None), clase, id).existen.into_iter().map(|(tipo, eid, cuerpo)| visible(tipo, eid, Sello(version, 0), move || Some(cuerpo.clone()))).collect()
    }

    /// La columna ajena (`EjercitoAvistado`): quién es, de quién, de qué tipo, quiénes van (los rombos), si está en formación, dónde está y el tramo visible de su plan; `te_sigue` si persigue a una
    /// columna propia. Ni composición, ni carro, ni víveres, ni Líder, ni destino.
    fn ejercito_avistado(&self, x: &Ejercito) -> pb::Ejercito {
        let tramo = tramo_visible(&x.plan, self.t, radios::HORIZONTE_MS, &self.ojos);
        pb::Ejercito {
            id: ids::ejercito(x.id),
            version: x.version,
            faccion_id: x.columna.faccion.map_or(String::new(), ids::faccion),
            participantes: x.columna.participantes.iter().map(|p| pb::Participante { heroe_id: ids::heroe(p.heroe), unido_en_ms: 0 }).collect(),
            tipo: if x.columna.tipo == militar::columna::TipoColumna::Personal { pb::TipoColumna::ColumnaPersonal } else { pb::TipoColumna::ColumnaEjercito } as i32,
            formacion_expira_en_ms: x.columna.formacion_expira_en,
            estado: if x.marchando() { pb::EstadoColumna::ColumnaMarchando } else { pb::EstadoColumna::ColumnaEstacionada } as i32,
            posicion: Some(punto(x.posicion_en(self.t))),
            plan: (!tramo.is_empty()).then(|| plan(&Plan::nuevo(x.plan.generacion, tramo))),
            te_sigue: x.persiguiendo.is_some_and(|presa| self.propias.contains(&presa)),
            ..Default::default()
        }
    }

    /// La caravana ajena (`CaravanaAvistada`): de qué Facción (la de su plaza de origen), dónde está, el tramo visible de su plan, si va escoltada y qué recursos lleva, sin cantidades.
    fn caravana_avistada(&self, c: &Caravana, p: Punto, plan_: &Plan, ojos: &[Ojo]) -> pb::Caravana {
        let e = self.n.estado();
        let tramo = if plan_.vertices.is_empty() { Vec::new() } else { tramo_visible(plan_, self.t, radios::HORIZONTE_MS, ojos) };
        let mut recursos: Vec<String> = c.contenido.iter().filter(|(_, &q)| q > 0).map(|(r, _)| r.nombre().to_owned()).collect();
        recursos.sort();
        pb::Caravana {
            id: ids::caravana(c.id),
            version: self.n.version_de(Clase::Caravana, c.id),
            faccion_id: e.asentamientos.get(&u64::from(c.origen)).map(|a| ids::faccion(a.faccion)),
            posicion: Some(punto(p)),
            plan: (!tramo.is_empty()).then(|| plan(&Plan::nuevo(plan_.generacion, tramo))),
            escoltada: c.estado == EstadoCaravana::Adjunta || !c.escolta.is_empty(),
            recursos_avistados: recursos,
            ..Default::default()
        }
    }

    /// El pueblo ajeno que se ve: de qué plaza es, dónde, qué extrae y si lo asedian; sin almacén, carro ni guarnición. Mientras es una obra, la obra sin lo pedido ni lo entregado.
    fn pueblo_avistado(&self, madre: u64, p: &asentamiento::pueblos::Pueblo, pos: Punto, version: u64) -> Vec<Visible<'_>> {
        let n = self.n;
        let (pid, madre_id) = (p.id, ids::asentamiento(madre));
        if p.activo() {
            let tipo = p.tipo.nombre().to_owned();
            let nodo = ids::nodo(p.nodo);
            return vec![visible(pb::TipoEntidad::Pueblo, ids::pueblo(pid), Sello(version, 0), move || {
                Some(Cuerpo::Pueblo(pb::Pueblo {
                    id: ids::pueblo(pid),
                    version,
                    asentamiento_madre_id: madre_id.clone(),
                    nodo_id: nodo.clone(),
                    tipo_extractor: tipo.clone(),
                    posicion: Some(punto(pos)),
                    batalla_id: propios::batalla_sobre(n, Ubicacion::Pueblo(pid)),
                    ..Default::default()
                }))
            })];
        }
        let nodo = Some(ids::nodo(p.nodo));
        vec![visible(pb::TipoEntidad::ObraDeMapa, ids::obra(pid), Sello(version, 0), move || {
            Some(Cuerpo::Obra(pb::ObraDeMapa { id: ids::obra(pid), version, asentamiento_madre_id: madre_id.clone(), tipo: pb::TipoObra::ObraPueblo as i32, posicion: Some(punto(pos)), nodo_id: nodo.clone(), ..Default::default() }))
        })]
    }

    /// La avanzada ajena que se ve: de qué plaza, su etapa, si está en ruinas, su vista y si la asedian; sin almacén ni guarnición ni tropas elegidas. Con una obra en marcha, la obra sin lo pedido ni lo
    /// entregado.
    fn avanzada_avistada(&self, madre: Option<u64>, x: &asentamiento::avanzadas::Avanzada, version: u64) -> Vec<Visible<'_>> {
        let n = self.n;
        let (vid, madre_id, pos) = (x.id, madre.map(ids::asentamiento).unwrap_or_default(), x.posicion);
        let mut v = Vec::new();
        if x.estado != asentamiento::avanzadas::Estado::EnConstruccion {
            let (etapa, ruinas) = (x.etapa.indice() as u32 + 1, x.en_ruinas());
            let invulnerable = match x.estado {
                asentamiento::avanzadas::Estado::EnRuinas { invulnerable_hasta } => invulnerable_hasta,
                _ => None,
            };
            let madre_id = madre_id.clone();
            v.push(visible(pb::TipoEntidad::Avanzada, ids::avanzada(vid), Sello(version, 0), move || {
                Some(Cuerpo::Avanzada(pb::Avanzada {
                    id: ids::avanzada(vid),
                    version,
                    asentamiento_madre_id: madre_id.clone(),
                    etapa,
                    posicion: Some(punto(pos)),
                    estado: if ruinas { pb::EstadoAvanzada::AvanzadaEnRuinas } else { pb::EstadoAvanzada::AvanzadaActiva } as i32,
                    radio_vision: radios::AVANZADA,
                    invulnerable_hasta_ms: invulnerable,
                    batalla_id: propios::batalla_sobre(n, Ubicacion::Avanzada(vid)),
                    ..Default::default()
                }))
            }));
        }
        if let Some(o) = &x.obra {
            let tipo = match o.tipo {
                asentamiento::avanzadas::TipoObra::Construir => pb::TipoObra::ObraAvanzada,
                asentamiento::avanzadas::TipoObra::Mejorar => pb::TipoObra::ObraMejoraAvanzada,
                asentamiento::avanzadas::TipoObra::Reparar => pb::TipoObra::ObraReparacion,
            } as i32;
            let destino = (o.tipo != asentamiento::avanzadas::TipoObra::Construir).then(|| ids::avanzada(vid));
            v.push(visible(pb::TipoEntidad::ObraDeMapa, ids::obra(vid), Sello(version, 0), move || {
                Some(Cuerpo::Obra(pb::ObraDeMapa { id: ids::obra(vid), version, asentamiento_madre_id: madre_id.clone(), tipo, posicion: Some(punto(pos)), destino_id: destino.clone(), ..Default::default() }))
            }));
        }
        v
    }
}

/// ¿Va la caravana adjunta a una columna propia?
fn propias_adjunta(e: &crate::estado::Estado, c: &Caravana, propias: &BTreeSet<u64>) -> bool {
    c.estado == EstadoCaravana::Adjunta && propias.iter().filter_map(|id| e.ejercitos.get(id)).any(|x| x.caravanas_adjuntas.contains(&c.id))
}

/// La plaza ajena que se ve (`AsentamientoAvistado`): la ficha (Facción, sitio, nivel, zona real recortada, asedio) con sus edificios **activos** (tipo, sitio y huella); nunca su almacén, su
/// población, sus obras, su cola, sus pueblos y avanzadas, sus políticas ni su trazado.
pub fn plaza_avistada(n: &Nucleo, id: u64) -> pb::Asentamiento {
    let a = &n.estado().asentamientos[&id];
    let mut p = ficha_de_asentamiento(a, &n.poligono_zona(id), propios::batalla_sobre(n, Ubicacion::Asentamiento(id as u32)));
    p.pueblos_ids.clear();
    p.avanzadas_ids.clear();
    let ciudad = n.vista_ciudad(id);
    p.edificios = a
        .economia
        .edificios
        .iter()
        .filter(|b| b.estado == EstadoEdificio::Activo)
        .map(|b| {
            let pieza = ciudad.as_ref().and_then(|c| c.piezas.iter().find(|x| x.id == b.id));
            pb::Edificio {
                id: ids::edificio(id, b.id),
                tipo: b.tipo.nombre().to_owned(),
                posicion: pieza.map(|x| punto(x.centro)),
                estado: pb::EstadoEdificio::EdificioActivo as i32,
                ambito: pb::AmbitoEdificio::Asentamiento as i32,
                nivel_interno: Some(u32::from(b.nivel_interno)),
                rotado: pieza.is_some_and(|x| x.rotado),
                ancho: pieza.map_or(0, |x| x.rect.ancho.max(0) as u32),
                alto: pieza.map_or(0, |x| x.rect.alto.max(0) as u32),
                ..Default::default()
            }
        })
        .collect();
    p
}

/// El héroe ajeno (`HeroePublico`): nombre, clase, nivel, hasta cuándo está herido y las escuadras que lleva en su columna. Nada de su campamento, sus puntos, su Liderazgo ni si es humano o bot.
pub fn heroe_publico(n: &Nucleo, id: u64) -> pb::Heroe {
    let h = n.heroe(id).expect("el héroe existe");
    let tropas = &n.estado().reglas.economia.militar.tropas;
    pb::Heroe {
        id: ids::heroe(id),
        version: h.heroe.escuadras.iter().map(|s| n.version_de(Clase::Escuadra, s.id)).fold(n.version_de(Clase::Heroe, id), u64::max),
        display_name: propios::nombre_de_heroe(id),
        class_definition_id: propios::CLASE_POR_DEFECTO.into(),
        avatar: Some(pb::Avatar::default()),
        nivel: h.heroe.nivel,
        herido_hasta_ms: h.heroe.herido_hasta,
        escuadras_que_lleva: h
            .heroe
            .escuadras
            .iter()
            .filter(|s| matches!(s.contenedor, Contenedor::Ejercito(_)))
            .map(|s| pb::EscuadraQueLleva { tropa_id: tropas.tropa(s.tropa).nombre.clone(), cantidad: u32::try_from(s.cantidad.max(0)).unwrap_or(u32::MAX), nivel: u32::from(s.nivel) })
            .collect(),
        ..Default::default()
    }
}

/// Una Mirada de la Facción, abierta o enfriando su zona.
fn mirada_intel(m: &MiradaIntel) -> pb::MiradaIntel {
    pb::MiradaIntel {
        id: ids::mirada(m.id),
        version: 0,
        faccion_id: ids::faccion(m.faccion),
        origen_id: match m.origen {
            OrigenIntel::Plaza(a) => ids::asentamiento(a),
            OrigenIntel::Campamento(c) => ids::campamento(c),
        },
        centro: Some(punto(m.centro)),
        radio: m.radio,
        comprada_en_ms: m.comprada_en,
        expira_en_ms: m.expira_en,
        libre_en_ms: m.libre_en,
    }
}

/// El Informe de una plaza ajena que conserva la Facción: lo que había el día que se compró (edificios levantados y en obra, recintos, guarnición y quién estaba dentro). Nunca el almacén.
fn informe_de_plaza(id: u64, i: &InformeDePlaza) -> pb::InformePlaza {
    let militar::inspeccion::Contenido::Plaza { faccion, heroes, guarnicion, recintos } = &i.defensa else { unreachable!("un Informe guarda la defensa de una plaza") };
    pb::InformePlaza {
        asentamiento_id: ids::asentamiento(id),
        version: 0,
        faccion_id: ids::faccion(*faccion),
        nombre: None,
        nivel: i.nivel,
        conocido_en_ms: i.conocido_en,
        posicion: Some(punto(i.posicion)),
        edificios: i
            .edificios
            .iter()
            .map(|b| pb::EdificioInforme {
                tipo: b.tipo.nombre().to_owned(),
                posicion: b.sitio.map(punto),
                estado: if b.en_obra { pb::EstadoEdificio::EdificioEnConstruccion } else { pb::EstadoEdificio::EdificioActivo } as i32,
                nivel_interno: Some(u32::from(b.nivel_interno)),
                ambito: pb::AmbitoEdificio::Asentamiento as i32,
            })
            .collect(),
        recintos: recintos.iter().map(|r| pb::RecintoInforme { nivel: u32::from(r.nivel), celdas: propios::celdas(&r.celdas), avance: r.avance }).collect(),
        guarnicion: propios::avistadas(guarnicion),
        heroes_ids: heroes.iter().map(|&h| ids::heroe(h)).collect(),
    }
}

/// La ficha de lo que se recuerda de un sitio ajeno.
fn ficha_conocida(s: Sitio, id: String, f: Ficha) -> pb::FichaConocida {
    let tipo = match s {
        Sitio::Plaza(_) => pb::TipoEntidad::Asentamiento,
        Sitio::Pueblo(_) => pb::TipoEntidad::Pueblo,
        Sitio::Avanzada(_) => pb::TipoEntidad::Avanzada,
    };
    let estado = match f.datos.estado {
        EstadoSitio::Activo => pb::EstadoSitio::SitioActivo,
        EstadoSitio::EnObra => pb::EstadoSitio::SitioEnObra,
        EstadoSitio::EnRuinas => pb::EstadoSitio::SitioEnRuinas,
    };
    pb::FichaConocida {
        asentamiento_id: id,
        nombre: None,
        faccion_id: (f.datos.faccion > 0).then(|| ids::faccion(f.datos.faccion)).unwrap_or_default(),
        posicion: Some(punto(f.datos.posicion)),
        nivel: f.datos.nivel,
        conocido_en_ms: f.conocido_en,
        radio_potencial: matches!(s, Sitio::Plaza(_)).then_some(f.datos.radio),
        tipo: tipo as i32,
        estado: estado as i32,
    }
}

/// ¿Cumple algún extremo de la arista la condición?
fn arista_en(a: &comercio::red_caminos::Arista, f: impl Fn(Punto) -> bool) -> bool {
    [a.0, a.1].into_iter().any(|(x, y)| f(Punto::new(x * 1000, y * 1000)))
}

/// El id de una arista de la red en el contrato: sus dos extremos en unidades enteras, `x1,y1:x2,y2` (los de la clave canónica, ordenados).
fn id_de_arista(a: &comercio::red_caminos::Arista) -> String {
    format!("{},{}:{},{}", a.0 .0, a.0 .1, a.1 .0, a.1 .1)
}

/// Las entidades de una vista ya proyectadas (la `Foto`), en el orden de `contenido`.
pub fn proyectar_todo(visibles: &[Visible<'_>]) -> Vec<Proyectada> {
    visibles.iter().filter_map(|x| Some((pb::TipoEntidad::try_from(x.clave.0).ok()?, x.clave.1.clone(), x.cuerpo()?))).collect()
}

/// Bytes iguales que se mandan dentro de un tramo antes que abrir otro (un tramo cuesta unos 5 bytes de cabecera).
const HUECO_EN_UN_TRAMO: usize = 8;

/// Los tramos de bytes de `ahora` que difieren de `antes` (de la misma longitud), juntando los separados por menos de [`HUECO_EN_UN_TRAMO`] bytes iguales.
fn tramos(antes: &[u8], ahora: &[u8]) -> Vec<pb::TramoDeNiebla> {
    let mut v: Vec<pb::TramoDeNiebla> = Vec::new();
    let mut i = 0;
    while i < ahora.len() {
        if antes[i] == ahora[i] {
            i += 1;
            continue;
        }
        let mut fin = i + 1;
        while fin < ahora.len() && ahora[fin..].iter().zip(&antes[fin..]).take(HUECO_EN_UN_TRAMO).any(|(a, b)| a != b) {
            fin += 1;
        }
        v.push(pb::TramoDeNiebla { desde: i as u32, datos: ahora[i..fin].to_vec() });
        i = fin;
    }
    v
}

/// La niebla que se manda a quien tiene `antes` y le toca `ahora` (4H T6): un cambio parcial con los tramos de bytes que cambiaron de cada máscara, o la niebla entera si cambiaron las dimensiones
/// o los tramos no ahorran nada. La máscara de lo visible cambia en casi cada muestreo de 250 ms (los ojos andan), pero solo en el borde de sus discos: unos cientos de bytes de los 26,7 KB de cada
/// máscara del mapa real.
pub fn cambio_de_niebla(antes: &pb::NieblaProyectada, ahora: &pb::NieblaProyectada) -> pb::NieblaProyectada {
    let dimensiones = |x: &pb::NieblaProyectada| (x.tamano_celda, x.columnas, x.filas, x.celdas.len(), x.visibles.len());
    if dimensiones(antes) != dimensiones(ahora) || antes.parcial || ahora.parcial {
        return ahora.clone();
    }
    let (tramos_celdas, tramos_visibles) = (tramos(&antes.celdas, &ahora.celdas), tramos(&antes.visibles, &ahora.visibles));
    let bytes: usize = tramos_celdas.iter().chain(&tramos_visibles).map(|t| t.datos.len() + 5).sum();
    if bytes >= ahora.celdas.len() + ahora.visibles.len() {
        return ahora.clone();
    }
    pb::NieblaProyectada { tamano_celda: ahora.tamano_celda, columnas: ahora.columnas, filas: ahora.filas, parcial: true, tramos_celdas, tramos_visibles, ..Default::default() }
}

/// Lo que hace el cliente con la niebla de una `Delta` o de una `Foto`: la entera sustituye a la que tiene; la parcial sobrescribe los bytes de sus tramos en la que tiene.
pub fn aplicar_niebla(destino: &mut Option<pb::NieblaProyectada>, llega: &pb::NieblaProyectada) {
    if !llega.parcial {
        *destino = Some(llega.clone());
        return;
    }
    let d = destino.as_mut().expect("una niebla parcial llega sobre la que el cliente ya tiene");
    for (mascara, tramos) in [(&mut d.celdas, &llega.tramos_celdas), (&mut d.visibles, &llega.tramos_visibles)] {
        for t in tramos {
            mascara[t.desde as usize..t.desde as usize + t.datos.len()].copy_from_slice(&t.datos);
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn niebla(celdas: &[u8], visibles: &[u8]) -> pb::NieblaProyectada {
        pb::NieblaProyectada { tamano_celda: 5_000, columnas: 8 * celdas.len() as u32, filas: 1, celdas: celdas.to_vec(), visibles: visibles.to_vec(), ..Default::default() }
    }

    #[test]
    fn la_niebla_parcial_lleva_solo_los_tramos_que_cambiaron_y_aplicada_da_la_nueva() {
        let mut antes = vec![0u8; 400];
        antes[10] = 0xFF;
        let (mut celdas, mut visibles) = (antes.clone(), antes.clone());
        celdas[11] = 1;
        visibles[100] = 3;
        visibles[104] = 3; // a menos de 8 bytes del anterior: el mismo tramo
        visibles[300] = 7;
        let (a, b) = (niebla(&antes, &antes), niebla(&celdas, &visibles));
        let c = cambio_de_niebla(&a, &b);
        assert!(c.parcial && c.celdas.is_empty() && c.visibles.is_empty());
        assert_eq!(c.tramos_celdas.iter().map(|t| (t.desde, t.datos.len())).collect::<Vec<_>>(), [(11, 1)]);
        assert_eq!(c.tramos_visibles.iter().map(|t| (t.desde, t.datos.len())).collect::<Vec<_>>(), [(100, 5), (300, 1)]);
        let mut cliente = Some(a.clone());
        aplicar_niebla(&mut cliente, &c);
        assert_eq!(cliente.as_ref(), Some(&b));
        // Sin cambios: ningún tramo. Todo cambiado, o con otras dimensiones: la niebla entera.
        assert!(cambio_de_niebla(&b, &b).tramos_celdas.is_empty() && cambio_de_niebla(&b, &b).tramos_visibles.is_empty());
        let todo = niebla(&[0xAA; 400], &[0x55; 400]);
        assert_eq!(cambio_de_niebla(&a, &todo), todo);
        assert_eq!(cambio_de_niebla(&a, &niebla(&[1; 10], &[1; 10])), niebla(&[1; 10], &[1; 10]));
        let mut cliente = Some(a);
        aplicar_niebla(&mut cliente, &todo);
        assert_eq!(cliente, Some(todo));
    }
}
