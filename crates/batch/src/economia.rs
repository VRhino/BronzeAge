//! Los adaptadores que ponen al asentamiento (crate `asentamiento`) sobre el mundo real: los nodos de piedra y de ganado con su stock
//! finito, las parcelas de bosque y los reclamos compartidos entre todas las plazas. Cada nodo o parcela lo explota una sola plaza.
//!
//! - [`Mapa`]: lo inmutable del artefacto (posiciones, cantidades, densidades) con una rejilla para buscar por radio.
//! - [`Recursos`]: el stock vivo de los nodos; implementa `Fuentes` (extraer y regenerar).
//! - [`Reclamos`]: quién tiene qué parcela o nodo.
//! - [`SitiosVilla`]: el `Sitios` de una plaza, con su disco de zona.

use asentamiento::catalogo::Tipo;
use asentamiento::edificio::Fuente;
use asentamiento::produccion::Fuentes;
use asentamiento::asentamiento::Sitios;
use base::{Instante, Mil, Punto, MS_POR_MINUTO};
use mundo::Mundo;
use std::collections::BTreeSet;
use std::sync::Arc;
use territorio::{Parcela, SinParcela};

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
        let (c0, c1) = (((centro.x - radio) / CELDA).max(0), ((centro.x + radio) / CELDA).min(self.columnas - 1));
        let (f0, f1) = (((centro.y - radio) / CELDA).max(0), ((centro.y + radio) / CELDA).min(self.filas - 1));
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
    pub tipo: Tipo,
    pub inicial: Mil,
}

/// Lo que el batch necesita del artefacto para la economía, inmutable.
pub struct Mapa {
    /// Piedra y ganado en una sola lista: `Fuente::Nodo(i)` indexa aquí.
    pub nodos: Vec<NodoMapa>,
    pub parcelas: Vec<Parcela>,
    pub total_por_masa: Vec<usize>,
    /// Densidad de cada parcela en ppm (la del contrato son milésimas: ×1000).
    pub densidad_ppm: Vec<i64>,
    pub campamentos: Vec<Punto>,
    /// Reglas de territorio del núcleo (cuota de parcelas por masa, techos de la zona, crecimiento por edificio).
    pub reglas: nucleo::ReglasTerritorio,
    pub ancho: i64,
    pub alto: i64,
    rej_parcelas: Rejilla,
    rej_piedra: Rejilla,
    rej_ganado: Rejilla,
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
        let rej_parcelas = Rejilla::nueva(ancho, alto, parcelas.iter().enumerate().map(|(i, p)| (i as u32, p.centro)));
        Arc::new(Mapa { nodos, parcelas, total_por_masa, densidad_ppm, campamentos, reglas: nucleo::ReglasTerritorio::default(), ancho, alto, rej_parcelas, rej_piedra, rej_ganado })
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
}

/// El stock vivo de los nodos. Un nodo agotado vuelve a su cantidad inicial pasado el cooldown de su categoría.
pub struct Recursos {
    mapa: Arc<Mapa>,
    stock: Vec<Mil>,
    vacio_desde: Vec<Option<Instante>>,
    /// El instante del minuto en curso (lo fija el bucle antes de cada paso).
    pub ahora: Instante,
    /// Veces que un nodo se agotó (informativo).
    pub agotamientos: u64,
}

impl Recursos {
    pub fn nuevo(mapa: &Arc<Mapa>) -> Self {
        Self { mapa: mapa.clone(), stock: mapa.nodos.iter().map(|n| n.inicial).collect(), vacio_desde: vec![None; mapa.nodos.len()], ahora: 0, agotamientos: 0 }
    }

    fn cooldown(&self, nodo: usize) -> i64 {
        MS_POR_MINUTO * if self.mapa.nodos[nodo].tipo == Tipo::Corral { REGENERA_GANADO_MIN } else { REGENERA_MINERAL_MIN }
    }

    /// Stock de todos los nodos y su calendario (para el hash de estado).
    pub fn estado(&self) -> impl Iterator<Item = (Mil, Option<Instante>)> + '_ {
        self.stock.iter().copied().zip(self.vacio_desde.iter().copied())
    }
}

impl Fuentes for Recursos {
    fn densidad_parcela(&self, parcela: u32) -> i64 {
        self.mapa.densidad_ppm.get(parcela as usize).copied().unwrap_or(0)
    }

    fn extraer(&mut self, nodo: u32, pedido: Mil) -> Mil {
        let n = nodo as usize;
        if self.stock[n] == 0 {
            match self.vacio_desde[n] {
                Some(desde) if self.ahora - desde >= self.cooldown(n) => {
                    self.stock[n] = self.mapa.nodos[n].inicial;
                    self.vacio_desde[n] = None;
                }
                _ => return 0,
            }
        }
        let sale = pedido.min(self.stock[n]);
        self.stock[n] -= sale;
        if self.stock[n] == 0 {
            self.vacio_desde[n] = Some(self.ahora);
            self.agotamientos += 1;
        }
        sale
    }
}

/// Quién explota qué: una parcela o un nodo lo tiene una sola plaza de todo el mundo.
pub struct Reclamos {
    /// Plaza (índice) que tiene cada parcela.
    pub parcela: Vec<Option<u32>>,
    pub nodo: Vec<Option<u32>>,
}

impl Reclamos {
    pub fn nuevo(m: &Mapa) -> Self {
        Self { parcela: vec![None; m.parcelas.len()], nodo: vec![None; m.nodos.len()] }
    }
}

/// Cuántas veces un `proponer` devolvió `None` por falta de sitio, por extractor ligado a un yacimiento (0 Leñera, 1 Cantera, 2 Corral).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SinSitio {
    /// Consultas denegadas mientras la plaza no tenía ningún extractor de ese tipo: la plaza se queda sin él por falta de sitio.
    pub sin_ninguno: [u64; 3],
    /// Todas las consultas denegadas (la auto-construcción repite la pregunta cada minuto).
    pub total: [u64; 3],
    /// La plaza ya comprometió al menos uno de ese tipo.
    pub tiene: [bool; 3],
}

impl SinSitio {
    fn denegar(&mut self, k: usize) {
        self.total[k] += 1;
        self.sin_ninguno[k] += !self.tiene[k] as u64;
    }
}

/// El `Sitios` de una plaza: disco de zona sin recorte por vecinas (simplificación). Lenera → parcela de bosque con la cuota por masa;
/// Cantera → el nodo de piedra libre más cercano; Corral → el de ganado; minas → ninguno (van por pueblo extractor); el resto, siempre cabe.
pub struct SitiosVilla<'a> {
    pub mapa: &'a Mapa,
    pub reclamos: &'a mut Reclamos,
    pub villa: u32,
    pub centro: Punto,
    pub radio: Mil,
    pub parcelas: &'a mut BTreeSet<u32>,
    pub sin_sitio: &'a mut SinSitio,
}

impl SitiosVilla<'_> {
    fn parcela(&self) -> Result<u32, SinParcela> {
        // Solo las parcelas del disco (y las propias, por si acaso): el resto no cambia la respuesta de `elegir`.
        let mut locales = self.mapa.parcelas_en(self.centro, self.radio);
        locales.extend(self.parcelas.iter().copied());
        locales.sort_unstable();
        locales.dedup();
        let ps: Vec<Parcela> = locales.iter().map(|&g| self.mapa.parcelas[g as usize]).collect();
        let propias: BTreeSet<u32> = locales.iter().enumerate().filter(|(_, g)| self.parcelas.contains(g)).map(|(i, _)| i as u32).collect();
        let r2 = (self.radio as i128).pow(2);
        territorio::elegir_parcela(&ps, &self.mapa.total_por_masa, self.mapa.reglas.cuota_parcelas_pct, self.centro, &propias, |i| self.reclamos.parcela[locales[i as usize] as usize].is_none(), |c| c.dist2(self.centro) <= r2)
            .map(|i| locales[i as usize])
    }

    fn el_mas_cercano(&self, candidatos: Vec<u32>) -> Option<u32> {
        candidatos.into_iter().filter(|&n| self.reclamos.nodo[n as usize].is_none()).min_by_key(|&n| (self.mapa.nodos[n as usize].pos.dist2(self.centro), n))
    }
}

impl Sitios for SitiosVilla<'_> {
    fn proponer(&mut self, tipo: Tipo) -> Option<Option<Fuente>> {
        match tipo {
            Tipo::Lenera => match self.parcela() {
                Ok(p) => Some(Some(Fuente::Parcela(p))),
                Err(_) => {
                    self.sin_sitio.denegar(0);
                    None
                }
            },
            Tipo::Cantera => match self.el_mas_cercano(self.mapa.piedra_en(self.centro, self.radio)) {
                Some(n) => Some(Some(Fuente::Nodo(n))),
                None => {
                    self.sin_sitio.denegar(1);
                    None
                }
            },
            Tipo::Corral => match self.el_mas_cercano(self.mapa.ganado_en(self.centro, self.radio)) {
                Some(n) => Some(Some(Fuente::Nodo(n))),
                None => {
                    self.sin_sitio.denegar(2);
                    None
                }
            },
            // Van por pueblo extractor (Doc 4.2.2), que este batch no implementa.
            Tipo::Mina | Tipo::MinaCobre | Tipo::MinaEstano | Tipo::MinaHierro => None,
            // El trazado urbano real (crate `trazado`) no está integrado: ponemos que siempre hay sitio.
            _ => Some(None),
        }
    }

    fn tomar(&mut self, tipo: Tipo, fuente: Option<Fuente>) {
        match (tipo, fuente) {
            (Tipo::Lenera, Some(Fuente::Parcela(p))) => {
                self.reclamos.parcela[p as usize] = Some(self.villa);
                self.parcelas.insert(p);
                self.sin_sitio.tiene[0] = true;
            }
            (Tipo::Cantera | Tipo::Corral, Some(Fuente::Nodo(n))) => {
                self.reclamos.nodo[n as usize] = Some(self.villa);
                self.sin_sitio.tiene[(tipo == Tipo::Corral) as usize + 1] = true;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn nodo(x: i64, tipo: Tipo) -> NodoMapa {
        NodoMapa { pos: Punto::new(x * 1000, 0), tipo, inicial: 100_000 }
    }

    fn parcela(x: i64, bosque: u32) -> Parcela {
        Parcela { bosque, centro: Punto::new(x * 1000, 0), densidad: 500 }
    }

    /// Piedra a 5 y 8 u, ganado a 5 u; una masa de 2 parcelas (3 y 4 u) y otra de 4 (50..53 u, lejos).
    fn mapa() -> Arc<Mapa> {
        let nodos = vec![nodo(5, Tipo::Cantera), nodo(8, Tipo::Cantera), nodo(5, Tipo::Corral)];
        let parcelas = vec![parcela(3, 0), parcela(4, 0), parcela(50, 1), parcela(51, 1), parcela(52, 1), parcela(53, 1)];
        Mapa::nuevo(nodos, parcelas, vec![2, 4], vec![], 100_000, 100_000)
    }

    fn sitios<'a>(m: &'a Mapa, r: &'a mut Reclamos, villa: u32, radio: Mil, parcelas: &'a mut BTreeSet<u32>, s: &'a mut SinSitio) -> SitiosVilla<'a> {
        SitiosVilla { mapa: m, reclamos: r, villa, centro: Punto::new(0, 0), radio, parcelas, sin_sitio: s }
    }

    #[test]
    fn la_rejilla_devuelve_solo_lo_que_cae_en_el_disco() {
        let m = mapa();
        assert_eq!(m.piedra_en(Punto::new(0, 0), 6_000), vec![0]);
        assert_eq!(m.piedra_en(Punto::new(0, 0), 8_000), vec![0, 1]);
        assert_eq!(m.parcelas_en(Punto::new(0, 0), 10_000), vec![0, 1]);
        assert!(m.ganado_en(Punto::new(0, 0), 4_000).is_empty());
    }

    #[test]
    fn un_nodo_agotado_vuelve_a_su_cantidad_tras_su_cooldown() {
        let m = mapa();
        let mut r = Recursos::nuevo(&m);
        r.ahora = 0;
        assert_eq!(r.extraer(0, 70_000), 70_000);
        assert_eq!(r.extraer(0, 70_000), 30_000, "solo sale lo que queda");
        r.ahora = 9 * MS_POR_MINUTO;
        assert_eq!(r.extraer(0, 10_000), 0, "la piedra tarda 10 min");
        r.ahora = 10 * MS_POR_MINUTO;
        assert_eq!(r.extraer(0, 10_000), 10_000, "ya se regeneró");
        // El ganado, a los 3.
        r.ahora = 0;
        assert_eq!(r.extraer(2, 100_000), 100_000);
        r.ahora = 3 * MS_POR_MINUTO;
        assert_eq!(r.extraer(2, 1), 1);
        assert_eq!(r.agotamientos, 2);
    }

    #[test]
    fn un_nodo_o_una_parcela_los_tiene_una_sola_plaza() {
        let m = mapa();
        let (mut rec, mut propias, mut otras) = (Reclamos::nuevo(&m), BTreeSet::new(), BTreeSet::new());
        let (mut ss1, mut ss2) = (SinSitio::default(), SinSitio::default());
        let p = sitios(&m, &mut rec, 0, 10_000, &mut propias, &mut ss1).proponer(Tipo::Cantera);
        assert_eq!(p, Some(Some(Fuente::Nodo(0))), "la más cercana");
        sitios(&m, &mut rec, 0, 10_000, &mut propias, &mut ss1).tomar(Tipo::Cantera, Some(Fuente::Nodo(0)));
        // Otra plaza con el mismo disco recibe la segunda, y luego ninguna.
        let mut s2 = sitios(&m, &mut rec, 1, 10_000, &mut otras, &mut ss2);
        assert_eq!(s2.proponer(Tipo::Cantera), Some(Some(Fuente::Nodo(1))));
        s2.tomar(Tipo::Cantera, Some(Fuente::Nodo(1)));
        assert_eq!(s2.proponer(Tipo::Cantera), None);
        assert_eq!((ss2.sin_ninguno[1], ss2.total[1]), (0, 1), "ya tenía una: la denegación no cuenta como «sin ninguno»");
    }

    #[test]
    fn la_lenera_respeta_la_cuota_de_la_masa_y_las_minas_van_por_pueblo() {
        let m = mapa();
        let (mut rec, mut propias, mut ss) = (Reclamos::nuevo(&m), BTreeSet::new(), SinSitio::default());
        let mut s = sitios(&m, &mut rec, 0, 10_000, &mut propias, &mut ss);
        // Masa de 2 parcelas, cuota 50 % = 1: una Leñera y no más.
        let Some(Some(Fuente::Parcela(p))) = s.proponer(Tipo::Lenera) else { panic!("hay parcela") };
        s.tomar(Tipo::Lenera, Some(Fuente::Parcela(p)));
        assert_eq!(s.proponer(Tipo::Lenera), None, "cupo de la masa agotado");
        assert_eq!(s.proponer(Tipo::MinaEstano), None);
        assert_eq!(s.proponer(Tipo::Vivienda), Some(None), "el resto siempre cabe");
        assert_eq!((ss.sin_ninguno[0], ss.total[0], ss.tiene[0]), (0, 1, true));
        assert_eq!(rec.parcela.iter().flatten().count(), 1);
    }
}
