//! La red de caminos (Comercio_Caravanas §7.4): las aristas que pisan las rutas comerciales. Una arista entra cuando alguna ruta la pisa
//! y **no sale nunca** (queda como sendero); las rutas vigentes dan el peso de cada arista, y el peso da el escalón (sendero, camino,
//! calzada). Sobre cualquier arista de la red una caravana avanza más rápido (eso lo aplica el plan, en `crates/rutas`).

use crate::datos::ReglasRed;
use crate::tipos::{AsentamientoId, FaccionId};
use base::{Instante, Punto};
use std::collections::{BTreeMap, BTreeSet};

/// Clave canónica de una arista: los dos extremos **redondeados a unidad entera** y ordenados.
pub type Arista = ((i64, i64), (i64, i64));

fn entero(v: i64) -> i64 {
    (v + 500).div_euclid(1000)
}

/// `claveArista(a, b)`.
pub fn clave_arista(a: Punto, b: Punto) -> Arista {
    let (p, q) = ((entero(a.x), entero(a.y)), (entero(b.x), entero(b.y)));
    if p <= q {
        (p, q)
    } else {
        (q, p)
    }
}

/// Las aristas de una ruta, sin repetir (`aristasDeTrazado`), en el orden en que se pisan.
// DUDA: un par de puntos consecutivos que redondean al mismo entero no es un tramo: se omite (en TypeScript daba una clave degenerada).
pub fn aristas_de_trazado(ruta: &[Punto]) -> Vec<Arista> {
    let mut v: Vec<Arista> = Vec::new();
    for w in ruta.windows(2) {
        let a = clave_arista(w[0], w[1]);
        if a.0 != a.1 && !v.contains(&a) {
            v.push(a);
        }
    }
    v
}

/// Punto medio de una arista, en milésimas.
pub fn punto_medio(a: &Arista) -> Punto {
    Punto::new((a.0 .0 + a.1 .0) * 500, (a.0 .1 + a.1 .1) * 500)
}

/// Una ruta comercial: una por par ordenado `origen > destino`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RutaComercial {
    pub origen: AsentamientoId,
    pub destino: AsentamientoId,
    /// La Facción del origen al lanzar.
    pub faccion: FaccionId,
    pub aristas: Vec<Arista>,
    pub ultimo_lanzamiento: Instante,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RedCaminos {
    aristas: BTreeSet<Arista>,
    rutas: BTreeMap<(AsentamientoId, AsentamientoId), RutaComercial>,
}

/// Lo que cambió al registrar una ruta: las aristas que entran por primera vez en la red (ya son camino) y si cambiaron los pesos
/// (una ruta nueva para el par, o con otras aristas que la anterior).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CambioRed {
    pub nuevas: Vec<Arista>,
    pub pesos_cambiados: bool,
}

impl RedCaminos {
    /// Reconstruye la red desde sus piezas (la recuperación de una foto).
    pub fn desde_partes(aristas: impl IntoIterator<Item = Arista>, rutas: impl IntoIterator<Item = RutaComercial>) -> Self {
        Self { aristas: aristas.into_iter().collect(), rutas: rutas.into_iter().map(|r| ((r.origen, r.destino), r)).collect() }
    }

    pub fn vacia(&self) -> bool {
        self.aristas.is_empty() && self.rutas.is_empty()
    }

    /// `registrarRuta` (en cada lanzamiento, a mano o por reparto): la ruta anterior del mismo par se **sustituye**; las aristas nuevas se
    /// añaden y las que la ruta nueva ya no pisa **siguen** en la red. Una ruta sin tramos no se registra.
    pub fn registrar_ruta(&mut self, origen: AsentamientoId, destino: AsentamientoId, faccion: FaccionId, ruta: &[Punto], ahora: Instante) -> CambioRed {
        let aristas = aristas_de_trazado(ruta);
        let nuevas: Vec<Arista> = aristas.iter().filter(|a| !self.aristas.contains(a)).copied().collect();
        let pesos_cambiados = self.rutas.get(&(origen, destino)).is_none_or(|x| x.aristas != aristas);
        self.aristas.extend(aristas.iter().copied());
        self.rutas.insert((origen, destino), RutaComercial { origen, destino, faccion, aristas, ultimo_lanzamiento: ahora });
        CambioRed { nuevas, pesos_cambiados }
    }

    /// `podarRutas`: quita las rutas que ya no son vigentes ([`RutaComercial::vigente`]). Devuelve `true` si cambió algo (si no, la red queda idéntica). Las aristas no se retiran nunca.
    pub fn podar(&mut self, r: &ReglasRed, ahora: Instante, existe: &dyn Fn(AsentamientoId) -> bool) -> bool {
        let antes = self.rutas.len();
        self.rutas.retain(|_, x| x.vigente(r, ahora, existe));
        self.rutas.len() != antes
    }

    /// Cuándo caduca la ruta más vieja (para agendar la poda como evento de baja frecuencia).
    pub fn proxima_caducidad(&self, r: &ReglasRed) -> Option<Instante> {
        self.rutas.values().map(|x| x.ultimo_lanzamiento + r.caducidad).min()
    }

    pub fn rutas(&self) -> impl Iterator<Item = &RutaComercial> {
        self.rutas.values()
    }

    pub fn ruta(&self, origen: AsentamientoId, destino: AsentamientoId) -> Option<&RutaComercial> {
        self.rutas.get(&(origen, destino))
    }

    /// `aristasDeRed`: todas las aristas, con o sin peso; sobre cualquiera la caravana avanza más rápido.
    pub fn aristas(&self) -> impl Iterator<Item = &Arista> {
        self.aristas.iter()
    }

    pub fn contiene(&self, a: &Arista) -> bool {
        self.aristas.contains(a)
    }

    /// `pesosDeRed`: para cada arista, cuántas rutas vigentes la contienen (las de peso 0 no aparecen).
    pub fn pesos(&self) -> BTreeMap<Arista, u32> {
        pesos(self.rutas.values())
    }

    /// `caminoCompartidoAbierto(red, zonas)`: ¿hay una arista, con su **punto medio fuera de toda zona**, que pisen `≥ logroRutas` rutas
    /// de `≥ logroFacciones` Facciones? Es la consulta del logro `logistica_campana` (Era IV, aún sin quien la llame).
    pub fn camino_compartido_abierto(&self, r: &ReglasRed, dentro_de_alguna_zona: &dyn Fn(Punto) -> bool) -> bool {
        self.pesos().into_iter().filter(|&(_, p)| p >= r.logro_rutas).any(|(a, _)| {
            if dentro_de_alguna_zona(punto_medio(&a)) {
                return false;
            }
            let facciones: BTreeSet<FaccionId> = self.rutas.values().filter(|x| x.aristas.contains(&a)).map(|x| x.faccion).collect();
            facciones.len() as u32 >= r.logro_facciones
        })
    }
}

impl RutaComercial {
    /// ¿Sigue vigente? Caduca a `caducidad` de su último lanzamiento o cuando uno de sus extremos deja de existir.
    pub fn vigente(&self, r: &ReglasRed, ahora: Instante, existe: &dyn Fn(AsentamientoId) -> bool) -> bool {
        ahora - self.ultimo_lanzamiento < r.caducidad && existe(self.origen) && existe(self.destino)
    }
}

/// Cuántas de `rutas` pisan cada arista (las de peso 0 no aparecen).
pub fn pesos<'a>(rutas: impl IntoIterator<Item = &'a RutaComercial>) -> BTreeMap<Arista, u32> {
    let mut p: BTreeMap<Arista, u32> = BTreeMap::new();
    for a in rutas.into_iter().flat_map(|x| &x.aristas) {
        *p.entry(*a).or_insert(0) += 1;
    }
    p
}

/// `escalonDePeso(p)`: `p ≥ escalonCalzada → 2` (calzada); `p ≥ escalonCamino → 1` (camino); si no `0` (sendero).
pub fn escalon_de_peso(r: &ReglasRed, peso: u32) -> u8 {
    if peso >= r.escalon_calzada {
        2
    } else if peso >= r.escalon_camino {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::Reglas;

    const MIN: i64 = 60_000;

    fn r() -> ReglasRed {
        Reglas::cargar().red
    }

    fn p(x: i64, y: i64) -> Punto {
        Punto::new(x * 1000, y * 1000)
    }

    #[test]
    fn la_clave_redondea_a_entero_y_no_depende_del_sentido() {
        let a = clave_arista(Punto::new(10_400, 20_600), Punto::new(30_000, 40_499));
        assert_eq!(a, ((10, 21), (30, 40)));
        assert_eq!(clave_arista(Punto::new(30_000, 40_499), Punto::new(10_400, 20_600)), a);
        assert_eq!(clave_arista(Punto::new(500, 0), Punto::new(0, 0)), ((0, 0), (1, 0)), "0,5 redondea hacia arriba");
    }

    #[test]
    fn registrar_una_ruta_anade_aristas_sin_repetir() {
        let mut red = RedCaminos::default();
        red.registrar_ruta(1, 2, 7, &[p(0, 0), p(10, 0), p(10, 10), p(0, 0), p(10, 0)], 0);
        assert_eq!(red.aristas().count(), 3, "(0,0)-(10,0) se pisa dos veces y cuenta una");
        assert_eq!(red.ruta(1, 2).unwrap().aristas.len(), 3);
        assert_eq!(red.pesos().values().copied().collect::<Vec<_>>(), vec![1, 1, 1]);
    }

    #[test]
    fn la_ruta_del_mismo_par_se_sustituye_pero_las_aristas_viejas_siguen_como_sendero() {
        let mut red = RedCaminos::default();
        red.registrar_ruta(1, 2, 7, &[p(0, 0), p(10, 0)], 0);
        red.registrar_ruta(1, 2, 7, &[p(0, 0), p(0, 10)], 5);
        assert_eq!(red.rutas().count(), 1);
        assert_eq!(red.aristas().count(), 2, "la arista vieja sigue en la red");
        let pesos = red.pesos();
        assert_eq!(pesos.len(), 1, "pero ya no pesa: ninguna ruta vigente la pisa");
        assert!(red.contiene(&clave_arista(p(0, 0), p(10, 0))) && !pesos.contains_key(&clave_arista(p(0, 0), p(10, 0))));
        assert_eq!(red.ruta(1, 2).unwrap().ultimo_lanzamiento, 5);
    }

    #[test]
    fn dos_pares_comparten_arista_y_suman_peso() {
        let mut red = RedCaminos::default();
        red.registrar_ruta(1, 2, 7, &[p(0, 0), p(10, 0)], 0);
        red.registrar_ruta(2, 1, 8, &[p(10, 0), p(0, 0)], 0);
        red.registrar_ruta(3, 2, 8, &[p(5, 5), p(10, 0)], 0);
        let pesos = red.pesos();
        assert_eq!(pesos[&clave_arista(p(0, 0), p(10, 0))], 2, "una por par ordenado, en sentidos opuestos");
        assert_eq!(pesos[&clave_arista(p(5, 5), p(10, 0))], 1);
        assert_eq!(red.rutas().count(), 3);
    }

    #[test]
    fn la_poda_quita_rutas_caducadas_o_sin_extremo_y_es_idempotente() {
        let r = r();
        let mut red = RedCaminos::default();
        red.registrar_ruta(1, 2, 7, &[p(0, 0), p(10, 0)], 0);
        red.registrar_ruta(1, 3, 7, &[p(0, 0), p(0, 10)], 1_000 * MIN);
        red.registrar_ruta(4, 2, 7, &[p(0, 0), p(5, 5)], 1_000 * MIN);
        let existe = |x| x != 4;
        assert_eq!(red.proxima_caducidad(&r), Some(4_320 * MIN));
        assert!(!red.podar(&r, 4_320 * MIN - 1, &|_| true), "nada que quitar: la misma red");
        assert!(red.podar(&r, 4_320 * MIN - 1, &existe), "la de un extremo inexistente sale");
        assert_eq!(red.rutas().count(), 2);
        assert!(red.podar(&r, 4_320 * MIN, &existe), "a los 4.320 min exactos caduca (>=)");
        assert_eq!(red.rutas().map(|x| (x.origen, x.destino)).collect::<Vec<_>>(), vec![(1, 3)]);
        assert!(!red.podar(&r, 4_320 * MIN, &existe), "idempotente");
        assert_eq!(red.aristas().count(), 3, "las aristas no salen nunca");
        assert_eq!(red.pesos().len(), 1);
    }

    #[test]
    fn escalones_sendero_camino_calzada() {
        let r = r();
        let e = |p| escalon_de_peso(&r, p);
        assert_eq!((e(0), e(1), e(2), e(5), e(6), e(40)), (0, 0, 1, 1, 2, 2));
    }

    #[test]
    fn camino_compartido_abierto_pide_diez_rutas_de_tres_facciones_fuera_de_las_zonas() {
        let r = r();
        let sin_zonas = |_p: Punto| false;
        let montar = |facciones: &[u32], rutas: u32| {
            let mut red = RedCaminos::default();
            for i in 0..rutas {
                // Todas pisan la arista (100,100)-(110,100); cada una tiene su propio origen.
                red.registrar_ruta(i, 1000 + i, facciones[i as usize % facciones.len()], &[p(100, 100), p(110, 100)], 0);
            }
            red
        };
        assert!(montar(&[1, 2, 3], 10).camino_compartido_abierto(&r, &sin_zonas));
        assert!(!montar(&[1, 2, 3], 9).camino_compartido_abierto(&r, &sin_zonas), "9 rutas no bastan");
        assert!(!montar(&[1, 2], 12).camino_compartido_abierto(&r, &sin_zonas), "dos Facciones no bastan");
        assert!(!montar(&[1, 2, 3], 10).camino_compartido_abierto(&r, &|punto| punto == p(105, 100)), "el punto medio está en una zona");
    }

    #[test]
    fn registrar_dice_que_aristas_abre_y_si_cambian_los_pesos() {
        let mut red = RedCaminos::default();
        let c1 = red.registrar_ruta(1, 2, 7, &[p(0, 0), p(10, 0)], 0);
        assert_eq!((c1.nuevas.len(), c1.pesos_cambiados), (1, true), "una ruta nueva abre su arista y cambia los pesos");
        let c2 = red.registrar_ruta(1, 2, 7, &[p(0, 0), p(10, 0)], 9);
        assert_eq!((c2.nuevas.len(), c2.pesos_cambiados), (0, false), "lanzar otra vez la misma ruta no abre nada ni cambia los pesos");
        let c3 = red.registrar_ruta(1, 3, 7, &[p(0, 0), p(10, 0)], 9);
        assert_eq!((c3.nuevas.len(), c3.pesos_cambiados), (0, true), "otro par por las mismas aristas: pesan más");
        let c4 = red.registrar_ruta(1, 2, 7, &[p(0, 0), p(0, 10)], 9);
        assert_eq!((c4.nuevas.len(), c4.pesos_cambiados), (1, true));
        // Reconstruirla desde sus piezas la deja igual.
        let copia = RedCaminos::desde_partes(red.aristas().copied(), red.rutas().cloned());
        assert_eq!(copia, red);
    }

    #[test]
    fn registrar_es_determinista() {
        let hacer = || {
            let mut red = RedCaminos::default();
            red.registrar_ruta(2, 1, 7, &[p(3, 3), p(9, 1), p(9, 9)], 10);
            red.registrar_ruta(1, 2, 7, &[p(0, 0), p(3, 3)], 20);
            red
        };
        assert_eq!(hacer(), hacer());
        assert_eq!(hacer().pesos(), hacer().pesos());
    }
}
