//! Aedas itinerantes (TA §6.2): cuántos hay, dónde nacen, cuánto se quedan y adónde van después. Sin azar: el reparto sale del orden de ids y el destino
//! de la distancia. El viaje es un `base::Plan` por la red de caminos que traza el núcleo con el planificador de las columnas a `AEDAS.velocidad` (6D);
//! cada Aeda tiene **un solo evento pendiente** ([`AedaItinerante::siguiente_evento`]): la llegada o el fin de la estancia.
//!
//! Qué plazas los aceptan lo decide la puerta de la plaza (grupo `aedas`, `politica::puerta`; el cierre por defecto no los incluye): el núcleo pasa
//! solo esas. Q5 del autor: cerrarles la puerta con un Aeda dentro lo hace salir al instante hacia su siguiente destino (el núcleo llama a
//! [`AedaItinerante::candidatos`] y [`AedaItinerante::salir`]); lo que ya reveló queda revelado.

use crate::datos::Reglas;
use crate::AsentamientoId;
use base::{Instante, Plan, Punto};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AedaItinerante {
    /// El N de `aeda-N` (decisión 7).
    pub id: u32,
    /// Donde se detuvo por última vez (o de donde salió). En marcha, la posición sale del plan.
    pub posicion: Punto,
    /// La plaza donde está detenido.
    pub en: Option<AsentamientoId>,
    /// Fin de la estancia (o de la espera de un Aeda varado, sin camino a ninguna plaza).
    pub hasta: Option<Instante>,
    pub destino: Option<AsentamientoId>,
    pub plan: Option<Plan>,
    /// Las últimas `memoriaVisitas` plazas visitadas, de la más vieja a la más reciente.
    pub recientes: Vec<AsentamientoId>,
}

/// `itinerantesObjetivo`: `max(minimo, ⌈vivas / faccionesPorAeda⌉)` con alguna Facción viva; 0 sin ninguna. Nunca se eliminan si baja.
pub fn objetivo(r: &Reglas, vivas: u32) -> u32 {
    if vivas == 0 {
        return 0;
    }
    r.aedas.minimo_itinerantes.max(vivas.div_ceil(r.aedas.facciones_por_aeda))
}

/// Los que faltan hasta el objetivo: `aeda-<i+1>` para `i` desde `existentes`, cada uno ya llegado (con su estancia) a la plaza `i mod n` de las que los
/// aceptan, **ordenadas por id**. Sin plazas que los acepten, ninguno.
pub fn nacer(r: &Reglas, existentes: u32, vivas: u32, aceptan: &[(AsentamientoId, Punto)], ahora: Instante) -> Vec<AedaItinerante> {
    let mut plazas = aceptan.to_vec();
    plazas.sort_by_key(|p| p.0);
    if plazas.is_empty() {
        return Vec::new();
    }
    (existentes..objetivo(r, vivas))
        .map(|i| {
            let (plaza, pos) = plazas[i as usize % plazas.len()];
            let mut a = AedaItinerante { id: i + 1, posicion: pos, en: None, hasta: None, destino: None, plan: None, recientes: Vec::new() };
            a.llegar(r, plaza, pos, ahora);
            a
        })
        .collect()
}

/// `aedaEn`: el Aeda detenido en esa plaza, si lo hay (el único al que se le compra).
pub fn aeda_en(aedas: &[AedaItinerante], plaza: AsentamientoId) -> Option<&AedaItinerante> {
    aedas.iter().find(|a| a.en == Some(plaza))
}

impl AedaItinerante {
    /// `llegado`: detenido en la plaza el tiempo de su estancia; la plaza pasa a la cola de `recientes`.
    pub fn llegar(&mut self, r: &Reglas, plaza: AsentamientoId, pos: Punto, ahora: Instante) {
        self.posicion = pos;
        self.en = Some(plaza);
        self.hasta = Some(ahora + r.aedas.estancia);
        self.destino = None;
        self.plan = None;
        self.recientes.retain(|&p| p != plaza);
        self.recientes.push(plaza);
        let sobran = self.recientes.len().saturating_sub(r.aedas.memoria_visitas);
        self.recientes.drain(..sobran);
    }

    /// Los destinos posibles, en el orden en que se prueban: las plazas que los aceptan distintas de la actual, primero las que no ha visitado hace
    /// poco (si no queda ninguna, todas), de cerca a lejos y, a igual distancia, por id. El núcleo sale hacia la primera a la que haya camino.
    pub fn candidatos(&self, aceptan: &[(AsentamientoId, Punto)]) -> Vec<AsentamientoId> {
        let otras: Vec<(AsentamientoId, Punto)> = aceptan.iter().copied().filter(|&(p, _)| Some(p) != self.en).collect();
        let frescas: Vec<(AsentamientoId, Punto)> = otras.iter().copied().filter(|(p, _)| !self.recientes.contains(p)).collect();
        let mut lista = if frescas.is_empty() { otras } else { frescas };
        lista.sort_by_key(|&(p, pos)| (self.posicion.dist2(pos), p));
        lista.into_iter().map(|(p, _)| p).collect()
    }

    /// Sale hacia `destino` por `plan` (que empieza ahora en su posición).
    pub fn salir(&mut self, destino: AsentamientoId, plan: Plan) {
        self.en = None;
        self.hasta = None;
        self.destino = Some(destino);
        self.plan = Some(plan);
    }

    /// Sin camino a ninguna candidata: se queda donde está otra estancia antes de volver a buscar ruta.
    pub fn esperar(&mut self, r: &Reglas, ahora: Instante) {
        self.hasta = Some(ahora + r.aedas.estancia);
    }

    /// Su destino desapareció en marcha (ruina): se para donde va y elige otro desde ahí, sin plaza actual.
    pub fn perder_destino(&mut self, ahora: Instante) {
        if let Some(p) = self.plan.as_ref().and_then(|p| p.posicion_en(ahora)) {
            self.posicion = p;
        }
        self.destino = None;
        self.plan = None;
        self.en = None;
        self.hasta = None;
    }

    /// Su único evento pendiente: la llegada (fin del plan) o el fin de la estancia. `None` solo si está parado sin plazo (recién perdido el destino:
    /// el núcleo elige otro en la misma transición).
    pub fn siguiente_evento(&self) -> Option<Instante> {
        self.plan.as_ref().and_then(Plan::fin).or(self.hasta)
    }
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/aedas.test.ts` («cuántos itinerantes hay», «recorrido»; la parte de reglas: el viaje por la red y
    //! la puerta son del núcleo, 6D).
    use super::*;
    use base::Vertice;

    fn p(x: i64) -> Punto {
        Punto::new(x * 1000, 0)
    }

    #[test]
    fn ninguno_sin_plazas_con_plazas_uno_por_cada_tres_facciones_y_minimo_tres() {
        let r = Reglas::default();
        assert_eq!(objetivo(&r, 0), 0);
        assert_eq!(objetivo(&r, 2), 3);
        assert_eq!(objetivo(&r, 10), 4);
        assert_eq!(objetivo(&r, 12), 4);
    }

    #[test]
    fn nacen_el_minimo_repartidos_por_las_plazas_y_no_nacen_mas() {
        let r = Reglas::default();
        let plazas = [(7, p(70)), (3, p(30))];
        let nuevos = nacer(&r, 0, 2, &plazas, 0);
        assert_eq!(nuevos.iter().map(|a| (a.id, a.en)).collect::<Vec<_>>(), vec![(1, Some(3)), (2, Some(7)), (3, Some(3))], "por id de plaza");
        assert!(nuevos.iter().all(|a| a.hasta == Some(r.aedas.estancia) && a.siguiente_evento() == Some(r.aedas.estancia)));
        assert!(nacer(&r, 3, 2, &plazas, 1).is_empty());
        assert!(nacer(&r, 0, 2, &[], 0).is_empty(), "sin plazas que los acepten, ninguno");
        assert_eq!(nacer(&r, 3, 12, &plazas, 5).iter().map(|a| a.id).collect::<Vec<_>>(), vec![4], "el que falta, con el siguiente número");
    }

    #[test]
    fn sale_hacia_la_mas_cercana_que_no_ha_visitado_y_si_no_queda_ninguna_a_cualquiera() {
        let r = Reglas::default();
        let plazas = [(1, p(0)), (2, p(10)), (3, p(20)), (4, p(30))];
        let mut a = nacer(&r, 0, 1, &plazas[..1], 0).remove(0);
        assert_eq!(a.candidatos(&plazas), vec![2, 3, 4]);
        a.recientes = vec![2];
        assert_eq!(a.candidatos(&plazas), vec![3, 4], "las frescas primero");
        a.recientes = vec![1, 2, 3, 4];
        assert_eq!(a.candidatos(&plazas), vec![2, 3, 4], "sin frescas, todas menos la actual");
        // A igual distancia, por id.
        a.recientes.clear();
        assert_eq!(a.candidatos(&[(9, p(-10)), (2, p(10))]), vec![2, 9]);
    }

    #[test]
    fn recorre_las_plazas_dejando_constancia_de_por_donde_ha_pasado() {
        let r = Reglas::default();
        let plazas = [(1, p(0)), (2, p(10))];
        let mut a = nacer(&r, 0, 1, &plazas[..1], 0).remove(0);
        let fin = a.hasta.unwrap();
        let destino = a.candidatos(&plazas)[0];
        a.salir(destino, Plan::nuevo(1, vec![Vertice { p: p(0), en: fin }, Vertice { p: p(10), en: fin + 500 }]));
        assert_eq!((a.en, a.siguiente_evento()), (None, Some(fin + 500)));
        a.llegar(&r, destino, p(10), fin + 500);
        assert_eq!((a.en, a.recientes.clone(), a.posicion), (Some(2), vec![1, 2], p(10)));
        assert_eq!(a.candidatos(&plazas), vec![1], "solo queda la otra");
        for x in 3..10 {
            a.llegar(&r, x, p(x as i64), 0);
        }
        assert_eq!(a.recientes, vec![6, 7, 8, 9], "memoria de 4");
        a.llegar(&r, 7, p(7), 0);
        assert_eq!(a.recientes, vec![6, 8, 9, 7]);
    }

    #[test]
    fn sin_camino_espera_otra_estancia_y_sin_destino_elige_desde_donde_esta() {
        let r = Reglas::default();
        let mut a = nacer(&r, 0, 1, &[(1, p(0))], 0).remove(0);
        a.esperar(&r, 100);
        assert_eq!(a.siguiente_evento(), Some(100 + r.aedas.estancia));
        a.salir(2, Plan::nuevo(1, vec![Vertice { p: p(0), en: 0 }, Vertice { p: p(10), en: 1000 }]));
        a.perder_destino(500);
        assert_eq!((a.posicion, a.en, a.destino, a.siguiente_evento()), (p(5), None, None, None));
        assert_eq!(a.candidatos(&[(1, p(0)), (3, p(6))]), vec![3], "la fresca, desde donde quedó");
        a.recientes.clear();
        assert_eq!(a.candidatos(&[(1, p(0)), (3, p(6))]), vec![3, 1], "sin plaza actual que excluir, de cerca a lejos");
    }

    #[test]
    fn el_aeda_de_una_plaza() {
        let r = Reglas::default();
        let aedas = nacer(&r, 0, 1, &[(1, p(0)), (2, p(1))], 0);
        assert_eq!(aeda_en(&aedas, 2).map(|a| a.id), Some(2));
        assert!(aeda_en(&aedas, 9).is_none());
    }
}
