//! Los adaptadores que ponen al asentamiento (crate `asentamiento`) sobre el mundo real para el batch de población: el stock finito de los
//! nodos de piedra y de ganado, los reclamos de parcelas y nodos compartidos entre todas las plazas. Cada nodo o parcela lo explota una
//! sola plaza. La geometría (rejilla de búsqueda) y la elección de sitio son las del núcleo (`nucleo::economia`), que las comparte.
//!
//! - [`Mapa`]: el mapa del núcleo más las reglas de territorio del batch.
//! - [`Recursos`]: el stock vivo de los nodos; implementa `Fuentes` (extraer y regenerar).
//! - [`Reclamos`]: quién tiene qué parcela o nodo.
//! - [`SitiosVilla`]: el `Sitios` de una plaza, con su disco de zona y su ciudad real ([`nucleo::urbanismo::Urbe`], la misma del núcleo).

use asentamiento::asentamiento::{Gratuito, Sitios, Urbano};
use asentamiento::catalogo::Tipo;
use asentamiento::edificio::Fuente;
use asentamiento::produccion::Fuentes;
use base::{Instante, Mil, Punto};
use mundo::Mundo;
use nucleo::economia::{regeneracion_ms, StockNodo};
use nucleo::urbanismo::{Urbanismo, Urbe};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;
use std::sync::Arc;
#[cfg(test)]
use territorio::Parcela;

#[cfg(test)]
pub use nucleo::economia::NodoMapa;
pub use nucleo::economia::{REGENERA_GANADO_MIN, REGENERA_MINERAL_MIN};

/// Lo que el batch necesita del artefacto para la economía, inmutable: el mapa del núcleo y las reglas de territorio (cuota de parcelas por
/// masa, techos de la zona, crecimiento por edificio). Se usa como el mapa del núcleo (`Deref`).
pub struct Mapa {
    base: Arc<nucleo::economia::Mapa>,
    pub reglas: nucleo::ReglasTerritorio,
}

impl Deref for Mapa {
    type Target = nucleo::economia::Mapa;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl Mapa {
    pub fn de(m: &Mundo) -> Arc<Mapa> {
        Arc::new(Mapa { base: nucleo::economia::Mapa::de(m), reglas: nucleo::ReglasTerritorio::default() })
    }

    /// El mapa a partir de sus partes (los mundos de pruebas).
    #[cfg(test)]
    pub fn nuevo(nodos: Vec<NodoMapa>, parcelas: Vec<Parcela>, total_por_masa: Vec<usize>, campamentos: Vec<Punto>, ancho: i64, alto: i64) -> Arc<Mapa> {
        Arc::new(Mapa { base: nucleo::economia::Mapa::nuevo(nodos, parcelas, total_por_masa, campamentos, ancho, alto), reglas: nucleo::ReglasTerritorio::default() })
    }
}

/// El stock vivo de los nodos. Un nodo agotado vuelve a su cantidad inicial pasado el cooldown de su categoría.
pub struct Recursos {
    mapa: Arc<Mapa>,
    nodos: Vec<StockNodo>,
    /// El instante del minuto en curso (lo fija el bucle antes de cada paso).
    pub ahora: Instante,
    /// Veces que un nodo se agotó (informativo).
    pub agotamientos: u64,
}

impl Recursos {
    pub fn nuevo(mapa: &Arc<Mapa>) -> Self {
        Self { mapa: mapa.clone(), nodos: mapa.nodos.iter().map(|n| StockNodo::nuevo(n.inicial)).collect(), ahora: 0, agotamientos: 0 }
    }

    /// Stock de todos los nodos y su calendario (para el hash de estado).
    pub fn estado(&self) -> impl Iterator<Item = (Mil, Option<Instante>)> + '_ {
        self.nodos.iter().map(|n| (n.stock, n.vacio_desde))
    }
}

impl Fuentes for Recursos {
    fn densidad_parcela(&self, parcela: u32) -> i64 {
        self.mapa.densidad_ppm.get(parcela as usize).copied().unwrap_or(0)
    }

    fn extraer(&mut self, nodo: u32, pedido: Mil) -> Mil {
        let n = &self.mapa.nodos[nodo as usize];
        let (sale, agotado) = self.nodos[nodo as usize].extraer(n.inicial, regeneracion_ms(n.tipo), self.ahora, pedido);
        self.agotamientos += agotado as u64;
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

/// Lo que la ciudad de una plaza le negó o le hizo esperar (Trazado_Urbano §6, §6.7, §7.1).
#[derive(Clone, Debug, Default)]
pub struct SinSitioCiudad {
    /// Consultas `proponer` denegadas por falta de sitio **en la ciudad** (el mapa sí tenía el yacimiento), por tipo. La auto-construcción
    /// repite la pregunta cada minuto mientras quiere el edificio.
    pub denegadas: BTreeMap<Tipo, u64>,
    /// `tomar` que ya no halló sitio (otro proyecto de la misma pasada se lo quedó), por tipo.
    pub tomar_negado: BTreeMap<Tipo, u64>,
    /// Mejoras terminadas que esperan hueco para mudarse (la Granja): id del edificio → minuto en que empezó a esperar.
    pub esperando: BTreeMap<u32, i64>,
    /// Esperas ya resueltas, en minutos.
    pub esperas: Vec<i64>,
}

/// El `Sitios` de una plaza: disco de zona sin recorte por vecinas (simplificación) y su ciudad real, la del núcleo. Lenera → parcela de
/// bosque con la cuota por masa; Cantera → el nodo de piedra libre más cercano; Corral → el de ganado; minas → ninguno (van por pueblo
/// extractor); y todo lo que tiene huella en la rejilla, además, solo si **cabe en la ciudad** (anclas, puestos y mudanza de la Granja
/// incluidos: es el mismo adaptador que [`nucleo::economia`] usa en las plazas del núcleo, sin recorte de zona por vecinas).
pub struct SitiosVilla<'a> {
    pub mapa: &'a Mapa,
    pub reclamos: &'a mut Reclamos,
    pub villa: u32,
    pub centro: Punto,
    pub radio: Mil,
    pub parcelas: &'a mut BTreeSet<u32>,
    pub sin_sitio: &'a mut SinSitio,
    pub urbe: &'a mut Urbe,
    pub urbanismo: &'a Urbanismo,
    pub ciudad: &'a mut SinSitioCiudad,
    /// El minuto de mundo en curso (para medir cuánto espera una mejora sin hueco).
    pub minuto: i64,
}

impl Sitios for SitiosVilla<'_> {
    fn proponer(&mut self, tipo: Tipo, u: &Urbano, ids: &mut u32, anclas: &mut Vec<Gratuito>) -> Option<Option<Fuente>> {
        let reclamos = &*self.reclamos;
        let propuesta = self.mapa.proponer(
            tipo,
            self.centro,
            self.radio,
            self.parcelas,
            self.mapa.reglas.cuota_parcelas_pct,
            &|i| reclamos.parcela[i as usize].is_none(),
            &|n| reclamos.nodo[n as usize].is_none(),
            &|_| true,
        );
        let Some(fuente) = propuesta else {
            match tipo {
                Tipo::Lenera => self.sin_sitio.denegar(0),
                Tipo::Cantera => self.sin_sitio.denegar(1),
                Tipo::Corral => self.sin_sitio.denegar(2),
                _ => {}
            }
            return None;
        };
        let (cabe, nuevas) = self.urbe.proponer(self.urbanismo, tipo, u, ids);
        anclas.extend(nuevas);
        if cabe {
            Some(fuente)
        } else {
            *self.ciudad.denegadas.entry(tipo).or_default() += 1;
            None
        }
    }

    fn tomar(&mut self, id: u32, tipo: Tipo, fuente: Option<Fuente>, u: &Urbano, ids: &mut u32) -> Option<Vec<Gratuito>> {
        let Some(gratuitos) = self.urbe.tomar(self.urbanismo, id, tipo, u, ids) else {
            *self.ciudad.tomar_negado.entry(tipo).or_default() += 1;
            return None;
        };
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
        Some(gratuitos)
    }

    fn puestos_de_mercado(&mut self, nivel: u8, u: &Urbano, ids: &mut u32) -> Vec<Gratuito> {
        self.urbe.puestos(self.urbanismo, nivel, u, ids)
    }

    fn hueco_al_mejorar(&mut self, id: u32, tipo: Tipo, nivel: u8, u: &Urbano) -> bool {
        self.urbe.hueco_al_mejorar(self.urbanismo, id, tipo, nivel, u)
    }

    /// La mudanza al terminar la mejora; si no hay hueco, la mejora espera y aquí se mide cuánto.
    fn reubicar(&mut self, id: u32, tipo: Tipo, nivel: u8, u: &Urbano) -> bool {
        let cabe = self.urbe.reubicar(self.urbanismo, id, tipo, nivel, u);
        if cabe {
            if let Some(desde) = self.ciudad.esperando.remove(&id) {
                self.ciudad.esperas.push(self.minuto - desde);
            }
        } else {
            self.ciudad.esperando.entry(id).or_insert(self.minuto);
        }
        cabe
    }

    /// Al terminar la mejora de un recinto (Murallas §3), como el núcleo.
    fn reclasificar_torres(&mut self, celdas: &mut [trazado::CeldaMuro], nivel: u8, p: &trazado::ParametrosMuralla) {
        self.urbe.reclasificar_torres(celdas, nivel, p);
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use base::MS_POR_MINUTO;

    /// El contexto urbano no importa a este adaptador.
    const U: Urbano = Urbano { nivel: 1, nivel_operativo: 1, completados: 0, perfil: None, siguiente_id: 0 };

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

    /// Una plaza de pruebas: su ciudad recién fundada y los registros que el `Sitios` rellena.
    struct Plaza {
        re: nucleo::ReglasEconomia,
        urbe: Urbe,
        ciudad: SinSitioCiudad,
    }

    impl Plaza {
        fn nueva() -> Self {
            let re = nucleo::ReglasEconomia::default();
            let urbe = Urbe::fundar(&re, 1, 0);
            Self { re, urbe, ciudad: SinSitioCiudad::default() }
        }
    }

    fn sitios<'a>(m: &'a Mapa, r: &'a mut Reclamos, villa: u32, radio: Mil, parcelas: &'a mut BTreeSet<u32>, s: &'a mut SinSitio, p: &'a mut Plaza) -> SitiosVilla<'a> {
        let Plaza { re, urbe, ciudad } = p;
        SitiosVilla { mapa: m, reclamos: r, villa, centro: Punto::new(0, 0), radio, parcelas, sin_sitio: s, urbe, urbanismo: &re.urbanismo, ciudad, minuto: 0 }
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
        let (mut p1, mut p2) = (Plaza::nueva(), Plaza::nueva());
        let p = sitios(&m, &mut rec, 0, 10_000, &mut propias, &mut ss1, &mut p1).proponer(Tipo::Cantera, &U, &mut 100, &mut vec![]);
        assert_eq!(p, Some(Some(Fuente::Nodo(0))), "la más cercana");
        sitios(&m, &mut rec, 0, 10_000, &mut propias, &mut ss1, &mut p1).tomar(0, Tipo::Cantera, Some(Fuente::Nodo(0)), &U, &mut 1);
        // Otra plaza con el mismo disco recibe la segunda, y luego ninguna.
        let mut s2 = sitios(&m, &mut rec, 1, 10_000, &mut otras, &mut ss2, &mut p2);
        assert_eq!(s2.proponer(Tipo::Cantera, &U, &mut 100, &mut vec![]), Some(Some(Fuente::Nodo(1))));
        s2.tomar(0, Tipo::Cantera, Some(Fuente::Nodo(1)), &U, &mut 1);
        assert_eq!(s2.proponer(Tipo::Cantera, &U, &mut 100, &mut vec![]), None);
        assert_eq!((ss2.sin_ninguno[1], ss2.total[1]), (0, 1), "ya tenía una: la denegación no cuenta como «sin ninguno»");
    }

    #[test]
    fn lo_que_la_ciudad_niega_se_cuenta_por_tipo() {
        let m = mapa();
        let (mut rec, mut propias, mut ss, mut plaza) = (Reclamos::nuevo(&m), BTreeSet::new(), SinSitio::default(), Plaza::nueva());
        let mut s = sitios(&m, &mut rec, 0, 10_000, &mut propias, &mut ss, &mut plaza);
        // El Centro Urbano no se coloca nunca: el mapa no se opone (no liga a un yacimiento) pero la ciudad dice que no cabe.
        assert_eq!(s.proponer(Tipo::CentroUrbano, &U, &mut 100, &mut vec![]), None);
        assert_eq!(s.proponer(Tipo::CentroUrbano, &U, &mut 100, &mut vec![]), None);
        assert_eq!(plaza.ciudad.denegadas.get(&Tipo::CentroUrbano), Some(&2));
        assert_eq!((ss.total, plaza.ciudad.denegadas.len()), ([0; 3], 1), "no se confunde con la falta de yacimiento");
    }

    #[test]
    fn la_lenera_respeta_la_cuota_de_la_masa_y_las_minas_van_por_pueblo() {
        let m = mapa();
        let (mut rec, mut propias, mut ss, mut plaza) = (Reclamos::nuevo(&m), BTreeSet::new(), SinSitio::default(), Plaza::nueva());
        let mut s = sitios(&m, &mut rec, 0, 10_000, &mut propias, &mut ss, &mut plaza);
        // Masa de 2 parcelas, cuota 50 % = 1: una Leñera y no más.
        let Some(Some(Fuente::Parcela(p))) = s.proponer(Tipo::Lenera, &U, &mut 100, &mut vec![]) else { panic!("hay parcela") };
        s.tomar(100, Tipo::Lenera, Some(Fuente::Parcela(p)), &U, &mut 101);
        assert_eq!(s.proponer(Tipo::Lenera, &U, &mut 100, &mut vec![]), None, "cupo de la masa agotado");
        assert_eq!(s.proponer(Tipo::MinaEstano, &U, &mut 100, &mut vec![]), None);
        assert_eq!(s.proponer(Tipo::Vivienda, &U, &mut 100, &mut vec![]), Some(None), "en una ciudad recién fundada cabe una Vivienda");
        assert_eq!((ss.sin_ninguno[0], ss.total[0], ss.tiene[0]), (0, 1, true));
        assert_eq!(rec.parcela.iter().flatten().count(), 1);
    }
}
