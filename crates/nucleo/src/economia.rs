//! La economía de las plazas sobre el mundo real: los nodos de piedra y de ganado con su stock finito, las parcelas de bosque y los
//! adaptadores que ponen al [`asentamiento::asentamiento::Asentamiento`] sobre ellos. Lo comparten el núcleo y el batch de población.
//!
//! - [`Mapa`]: lo inmutable del artefacto (posiciones, cantidades, densidades) con una rejilla para buscar por radio, y la elección de
//!   sitio de cada tipo de edificio ([`Mapa::proponer`]).
//! - [`StockNodo`]: el stock vivo de un nodo y su regeneración. Es **estado del núcleo** (entra en el hash y en las fotos); se guarda solo
//!   el de los nodos que alguien ha tocado.
//! - [`FuentesMundo`] y [`SitiosPlaza`]: los `Fuentes` y `Sitios` de una plaza dentro del núcleo (los yacimientos del mapa más la
//!   ciudad de la plaza, [`crate::urbanismo`]).

/// Madera por minuto mínima que debe poder dar el bosque a 10 u de una plaza para que no caiga en ruinas (decidido el 2026-10-18; la
/// cifra es un placeholder a calibrar: el mantenimiento pide ~1,65 al día 1 y ~2,5 con 40 edificios). Se mide como `5 × Σ densidades`.
pub const MADERA_MINIMA_POTENCIAL: Mil = 3_000;

use asentamiento::asentamiento::{Gratuito, Sitios, Urbano};
use asentamiento::catalogo::Tipo;
use asentamiento::edificio::Fuente;
use asentamiento::produccion::Fuentes;
use base::{Instante, Mil, Punto, MS_POR_MINUTO};
use mundo::Mundo;
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use territorio::{Disco, Parcela};

use crate::estado::Asentamiento;
use crate::urbanismo::{Urbanismo, Urbe};

/// Lado de la celda de la rejilla de búsqueda (milésimas): 10 unidades.
const CELDA: i64 = 10_000;
/// Regeneración de un nodo agotado (`REGENERACION_NODOS`): el ganado a los 3 minutos, los minerales a los 10.
pub const REGENERA_GANADO_MIN: i64 = 3;
pub const REGENERA_MINERAL_MIN: i64 = 10;

/// Puntos indexados por celda para preguntar «los que caen en este disco».
struct Rejilla {
    columnas: i64,
    filas: i64,
    celdas: Vec<Vec<u32>>,
}

impl Rejilla {
    fn nueva(ancho: i64, alto: i64, puntos: impl Iterator<Item = (u32, Punto)>) -> Self {
        let (columnas, filas) = (ancho / CELDA + 1, alto / CELDA + 1);
        let mut celdas = vec![Vec::new(); (columnas * filas) as usize];
        for (id, p) in puntos {
            let (c, f) = ((p.x / CELDA).clamp(0, columnas - 1), (p.y / CELDA).clamp(0, filas - 1));
            celdas[(f * columnas + c) as usize].push(id);
        }
        Self { columnas, filas, celdas }
    }

    /// Ids cuyo punto está a `radio` o menos de `centro`, en orden ascendente de id.
    fn en_disco(&self, centro: Punto, radio: Mil, posicion: impl Fn(u32) -> Punto) -> Vec<u32> {
        // Los dos extremos se acotan a la rejilla: un punto fuera del mapa vive en la celda del borde, y un disco que empieza más allá
        // del borde aún puede alcanzarlo.
        let celda = |v: i64, n: i64| v.div_euclid(CELDA).clamp(0, n - 1);
        let (c0, c1) = (celda(centro.x - radio, self.columnas), celda(centro.x + radio, self.columnas));
        let (f0, f1) = (celda(centro.y - radio, self.filas), celda(centro.y + radio, self.filas));
        let mut v = Vec::new();
        for f in f0..=f1 {
            for c in c0..=c1 {
                v.extend(self.celdas[(f * self.columnas + c) as usize].iter().copied().filter(|&id| posicion(id).dist2(centro) <= (radio as i128).pow(2)));
            }
        }
        v.sort_unstable();
        v
    }
}

/// Un nodo finito del mapa.
pub struct NodoMapa {
    pub pos: Punto,
    /// El extractor que lo explota: `Cantera` (piedra), `Corral` (ganado) o una mina (oro, cobre, estaño o hierro: solo con un pueblo extractor).
    pub tipo: Tipo,
    pub inicial: Mil,
}

/// Lo que la economía necesita del artefacto, inmutable.
pub struct Mapa {
    /// Piedra, ganado y minerales en una sola lista: `Fuente::Nodo(i)` indexa aquí.
    pub nodos: Vec<NodoMapa>,
    pub parcelas: Vec<Parcela>,
    pub total_por_masa: Vec<usize>,
    /// Densidad de cada parcela en ppm (la del contrato son milésimas: ×1000).
    pub densidad_ppm: Vec<i64>,
    pub campamentos: Vec<Punto>,
    pub ancho: i64,
    pub alto: i64,
    rej_parcelas: Rejilla,
    rej_piedra: Rejilla,
    rej_ganado: Rejilla,
    rej_minerales: Rejilla,
}

macro_rules! pt {
    ($o:expr) => {
        $o.as_ref().map(|p| Punto::new(p.x, p.y)).unwrap_or_default()
    };
}

impl Mapa {
    pub fn de(m: &Mundo) -> Arc<Mapa> {
        let masas: std::collections::HashMap<&str, usize> = m.bosques().iter().enumerate().map(|(i, b)| (b.id.as_str(), i)).collect();
        let parcelas: Vec<Parcela> = m.parcelas().iter().map(|p| Parcela { bosque: masas[p.bosque_id.as_str()] as u32, centro: pt!(p.centro), densidad: p.densidad_milesimas }).collect();
        let mut total_por_masa = vec![0; masas.len()];
        parcelas.iter().for_each(|p| total_por_masa[p.bosque as usize] += 1);
        let nodos: Vec<NodoMapa> = m
            .nodos()
            .iter()
            .filter_map(|n| {
                let tipo = match n.tipo.as_str() {
                    "piedra" => Tipo::Cantera,
                    "livestock" => Tipo::Corral,
                    "oro" => Tipo::Mina,
                    "cobre" => Tipo::MinaCobre,
                    "estano" => Tipo::MinaEstano,
                    "hierro" => Tipo::MinaHierro,
                    _ => return None,
                };
                Some(NodoMapa { pos: pt!(n.posicion), tipo, inicial: n.cantidad_inicial })
            })
            .collect();
        let (ancho, alto) = m.tamano();
        Self::nuevo(nodos, parcelas, total_por_masa, m.campamentos().iter().map(|c| pt!(c.posicion)).collect(), ancho, alto)
    }

    /// El mapa a partir de sus partes (también para los mundos de pruebas).
    pub fn nuevo(nodos: Vec<NodoMapa>, parcelas: Vec<Parcela>, total_por_masa: Vec<usize>, campamentos: Vec<Punto>, ancho: i64, alto: i64) -> Arc<Mapa> {
        let densidad_ppm = parcelas.iter().map(|p| p.densidad as i64 * 1000).collect();
        let de_tipo = |t: Tipo| nodos.iter().enumerate().filter(move |(_, n)| n.tipo == t).map(|(i, n)| (i as u32, n.pos)).collect::<Vec<_>>();
        let rej_piedra = Rejilla::nueva(ancho, alto, de_tipo(Tipo::Cantera).into_iter());
        let rej_ganado = Rejilla::nueva(ancho, alto, de_tipo(Tipo::Corral).into_iter());
        let minerales = nodos.iter().enumerate().filter(|(_, n)| n.tipo.es_de_pueblo() && n.tipo != Tipo::Cantera).map(|(i, n)| (i as u32, n.pos));
        let rej_minerales = Rejilla::nueva(ancho, alto, minerales);
        let rej_parcelas = Rejilla::nueva(ancho, alto, parcelas.iter().enumerate().map(|(i, p)| (i as u32, p.centro)));
        Arc::new(Mapa { nodos, parcelas, total_por_masa, densidad_ppm, campamentos, ancho, alto, rej_parcelas, rej_piedra, rej_ganado, rej_minerales })
    }

    pub fn parcelas_en(&self, centro: Punto, radio: Mil) -> Vec<u32> {
        self.rej_parcelas.en_disco(centro, radio, |i| self.parcelas[i as usize].centro)
    }

    pub fn piedra_en(&self, centro: Punto, radio: Mil) -> Vec<u32> {
        self.rej_piedra.en_disco(centro, radio, |i| self.nodos[i as usize].pos)
    }

    pub fn ganado_en(&self, centro: Punto, radio: Mil) -> Vec<u32> {
        self.rej_ganado.en_disco(centro, radio, |i| self.nodos[i as usize].pos)
    }

    pub fn minerales_en(&self, centro: Punto, radio: Mil) -> Vec<u32> {
        self.rej_minerales.en_disco(centro, radio, |i| self.nodos[i as usize].pos)
    }

    fn parcela(&self, centro: Punto, radio: Mil, propias: &BTreeSet<u32>, cuota_pct: u32, libre: &dyn Fn(u32) -> bool, en_zona: &dyn Fn(Punto) -> bool) -> Option<u32> {
        // Solo las parcelas del disco (y las propias, por si acaso): el resto no cambia la respuesta de `elegir`.
        let mut locales = self.parcelas_en(centro, radio);
        locales.extend(propias.iter().copied());
        locales.sort_unstable();
        locales.dedup();
        let ps: Vec<Parcela> = locales.iter().map(|&g| self.parcelas[g as usize]).collect();
        let propias_locales: BTreeSet<u32> = locales.iter().enumerate().filter(|(_, g)| propias.contains(g)).map(|(i, _)| i as u32).collect();
        let r2 = (radio as i128).pow(2);
        territorio::elegir_parcela(&ps, &self.total_por_masa, cuota_pct, centro, &propias_locales, |i| libre(locales[i as usize]), |c| c.dist2(centro) <= r2 && en_zona(c))
            .ok()
            .map(|i| locales[i as usize])
    }

    fn mas_cercano(&self, candidatos: Vec<u32>, centro: Punto, libre: &dyn Fn(u32) -> bool, en_zona: &dyn Fn(Punto) -> bool) -> Option<u32> {
        candidatos.into_iter().filter(|&n| libre(n) && en_zona(self.nodos[n as usize].pos)).min_by_key(|&n| (self.nodos[n as usize].pos.dist2(centro), n))
    }

    /// Dónde puede ir un edificio de esa plaza (el contrato de `Sitios::proponer`, sin tomar nada):
    /// Leñera → parcela de bosque libre en la zona, con la cuota por masa; Cantera → el nodo de piedra libre más cercano en la zona; Corral →
    /// el de ganado; minas → `None` (van por pueblo extractor, Doc 4.2.2); el resto, `Some(None)`: este método solo conoce los yacimientos, y que
    /// el edificio **quepa en la ciudad** lo decide el trazado urbano ([`crate::urbanismo::Urbe`]) en las plazas del núcleo; el batch de
    /// población no lo usa y deja que siempre quepa. `en_zona` recorta el disco `(centro, radio)` por las fronteras con otras Facciones.
    #[allow(clippy::too_many_arguments)]
    pub fn proponer(
        &self,
        tipo: Tipo,
        centro: Punto,
        radio: Mil,
        propias: &BTreeSet<u32>,
        cuota_pct: u32,
        parcela_libre: &dyn Fn(u32) -> bool,
        nodo_libre: &dyn Fn(u32) -> bool,
        en_zona: &dyn Fn(Punto) -> bool,
    ) -> Option<Option<Fuente>> {
        match tipo {
            Tipo::Lenera => self.parcela(centro, radio, propias, cuota_pct, parcela_libre, en_zona).map(|p| Some(Fuente::Parcela(p))),
            Tipo::Cantera => self.mas_cercano(self.piedra_en(centro, radio), centro, nodo_libre, en_zona).map(|n| Some(Fuente::Nodo(n))),
            Tipo::Corral => self.mas_cercano(self.ganado_en(centro, radio), centro, nodo_libre, en_zona).map(|n| Some(Fuente::Nodo(n))),
            // Van por pueblo extractor (Doc 4.2.2), no por la ciudad.
            Tipo::Mina | Tipo::MinaCobre | Tipo::MinaEstano | Tipo::MinaHierro => None,
            // Sin yacimiento: el sitio lo decide la ciudad de la plaza, no el mapa.
            _ => Some(None),
        }
    }
}

/// Stock vivo de un nodo: cuánto queda y, si se agotó, desde cuándo. Un nodo agotado vuelve a su cantidad inicial pasado el cooldown de
/// su categoría; es un estado determinista en enteros, sin eventos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StockNodo {
    pub stock: Mil,
    pub vacio_desde: Option<Instante>,
}

/// Cooldown de regeneración de un nodo de ese tipo, en ms.
pub fn regeneracion_ms(tipo: Tipo) -> i64 {
    MS_POR_MINUTO * if tipo == Tipo::Corral { REGENERA_GANADO_MIN } else { REGENERA_MINERAL_MIN }
}

impl StockNodo {
    pub fn nuevo(inicial: Mil) -> Self {
        Self { stock: inicial, vacio_desde: None }
    }

    /// Saca hasta `pedido` en `ahora`: regenera antes si el cooldown venció. Devuelve lo que salió y si con ello se agotó.
    pub fn extraer(&mut self, inicial: Mil, regenera_ms: i64, ahora: Instante, pedido: Mil) -> (Mil, bool) {
        if self.stock == 0 {
            match self.vacio_desde {
                Some(desde) if ahora - desde >= regenera_ms => {
                    self.stock = inicial;
                    self.vacio_desde = None;
                }
                _ => return (0, false),
            }
        }
        let sale = pedido.min(self.stock);
        self.stock -= sale;
        let agotado = self.stock == 0;
        if agotado {
            self.vacio_desde = Some(ahora);
        }
        (sale, agotado)
    }
}

/// Los yacimientos y parcelas que ve una plaza al avanzar su minuto.
pub(crate) struct FuentesMundo<'a> {
    pub mapa: &'a Mapa,
    pub nodos: &'a mut BTreeMap<u32, StockNodo>,
    pub ahora: Instante,
}

impl Fuentes for FuentesMundo<'_> {
    fn densidad_parcela(&self, parcela: u32) -> i64 {
        self.mapa.densidad_ppm.get(parcela as usize).copied().unwrap_or(0)
    }

    fn extraer(&mut self, nodo: u32, pedido: Mil) -> Mil {
        let Some(n) = self.mapa.nodos.get(nodo as usize) else { return 0 };
        self.nodos.entry(nodo).or_insert_with(|| StockNodo::nuevo(n.inicial)).extraer(n.inicial, regeneracion_ms(n.tipo), self.ahora, pedido).0
    }
}

/// El `Sitios` de una plaza dentro del núcleo. La zona es el disco recortado por las fronteras con las plazas de otras Facciones que ya
/// la tocan (`Estado::contactos`); esas vecinas se buscan solo cuando hace falta (la mayoría de los minutos no se pregunta por un sitio).
pub(crate) struct SitiosPlaza<'a> {
    pub mapa: &'a Mapa,
    pub cuota_pct: u32,
    pub id: u64,
    pub yo: Disco,
    pub parcelas: &'a mut BTreeSet<u32>,
    /// La ciudad de la plaza y la tabla de huellas: dónde cae cada edificio dentro de ella.
    pub urbe: &'a mut Urbe,
    pub urbanismo: &'a Urbanismo,
    pub parcelas_ocupadas: &'a mut BTreeMap<u32, u64>,
    pub nodos_reclamados: &'a mut BTreeMap<u32, u64>,
    /// Las demás plazas (la propia no está: el llamante la saca del mapa mientras avanza).
    pub otras: &'a BTreeMap<u64, Asentamiento>,
    pub contactos: &'a BTreeSet<(u64, u64)>,
    pub vecinos: OnceCell<Vec<Disco>>,
}

impl SitiosPlaza<'_> {
    fn vecinos(&self) -> &[Disco] {
        self.vecinos.get_or_init(|| {
            let id = self.id;
            self.contactos
                .iter()
                .filter_map(|&(a, b)| if a == id { Some(b) } else { (b == id).then_some(a) })
                .filter_map(|o| self.otras.get(&o))
                .map(|o| Disco { centro: o.posicion, radio: o.radio, faccion: o.faccion })
                .collect()
        })
    }
}

impl Sitios for SitiosPlaza<'_> {
    /// Hay sitio si lo hay en el mapa (los que ligan a un yacimiento) y en la ciudad (caché). Se mira antes el mapa, que es barato: una
    /// Leñera sin parcela libre o un Corral sin ganado no tienen sitio, y no hace falta barrer la ciudad para saberlo.
    fn proponer(&mut self, tipo: Tipo, u: &Urbano, ids: &mut u32, anclas: &mut Vec<Gratuito>) -> Option<Option<Fuente>> {
        let yo = self.yo;
        // Solo los tipos que ligan a un yacimiento miran la geometría de la zona.
        let ligado = matches!(tipo, Tipo::Lenera | Tipo::Cantera | Tipo::Corral);
        let vecinos: &[Disco] = if ligado { self.vecinos() } else { &[] };
        let (po, nr) = (&*self.parcelas_ocupadas, &*self.nodos_reclamados);
        let fuente = self.mapa.proponer(tipo, yo.centro, yo.radio, self.parcelas, self.cuota_pct, &|i| !po.contains_key(&i), &|n| !nr.contains_key(&n), &|c| territorio::en_zona(c, &yo, vecinos))?;
        let (cabe, nuevas) = self.urbe.proponer(self.urbanismo, tipo, u, ids);
        anclas.extend(nuevas);
        cabe.then_some(fuente)
    }

    /// Coloca el edificio en la ciudad y, solo si cabe, toma su yacimiento. `None` = otro proyecto de la misma pasada se quedó el sitio.
    fn tomar(&mut self, id: u32, tipo: Tipo, fuente: Option<Fuente>, u: &Urbano, ids: &mut u32) -> Option<Vec<Gratuito>> {
        let gratuitos = self.urbe.tomar(self.urbanismo, id, tipo, u, ids)?;
        match fuente {
            Some(Fuente::Parcela(p)) => {
                self.parcelas_ocupadas.insert(p, self.id);
                self.parcelas.insert(p);
            }
            Some(Fuente::Nodo(n)) => {
                self.nodos_reclamados.insert(n, self.id);
            }
            None => {}
        }
        Some(gratuitos)
    }

    fn puestos_de_mercado(&mut self, nivel: u8, u: &Urbano, ids: &mut u32) -> Vec<Gratuito> {
        self.urbe.puestos(self.urbanismo, nivel, u, ids)
    }

    fn hueco_al_mejorar(&mut self, id: u32, tipo: Tipo, nivel: u8, u: &Urbano) -> bool {
        self.urbe.hueco_al_mejorar(self.urbanismo, id, tipo, nivel, u)
    }

    fn reubicar(&mut self, id: u32, tipo: Tipo, nivel: u8, u: &Urbano) -> bool {
        self.urbe.reubicar(self.urbanismo, id, tipo, nivel, u)
    }

    fn reclasificar_torres(&mut self, celdas: &mut [trazado::CeldaMuro], nivel: u8, p: &trazado::ParametrosMuralla) {
        self.urbe.reclasificar_torres(celdas, nivel, p);
    }
}
