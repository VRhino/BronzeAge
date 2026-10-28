//! La participación en el núcleo (mecánica 54, `Docs/Mechanics/Participacion.md`): de dónde salen los puntos y la lista de miembros de una Facción. Lo puro (las cifras y la foto) es de
//! `crate::participacion`.
//!
//! Los puntos se suman **en origen**, al resolverse cada combate con números o al aplicarse el resultado de Unity ([`Nucleo::participacion_de_batalla`]).

use super::*;
use militar::escuadra::Escuadra;
use militar::heroe::Residencia;

/// Un ciudadano de una Facción con su participación (Participacion §3): dónde reside, sus cargos y la suma de sus puntos en las plazas que son de la Facción hoy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Miembro {
    pub heroe: u64,
    pub residencia: Option<Residencia>,
    /// `rey`, `embajador` y los locales de su plaza de residencia, como `gobernador@as-3`.
    pub cargos: Vec<String>,
    pub participacion: u64,
}

impl Nucleo {
    /// La participación de cada héroe en la plaza (vacía si no tiene ninguna).
    pub fn participacion_en(&self, plaza: u64) -> Option<&BTreeMap<u64, u64>> {
        self.estado.participacion.get(&plaza)
    }

    /// Los ciudadanos de la Facción, en orden de ingreso, con su participación (Participacion §3).
    pub fn miembros_de(&self, f: u32) -> Vec<Miembro> {
        let e = &self.estado;
        let Some(x) = e.politica.facciones.get(f) else { return vec![] };
        let mut suma: BTreeMap<u64, u64> = BTreeMap::new();
        for (plaza, heroes) in &e.participacion {
            if e.asentamientos.get(plaza).is_some_and(|a| a.faccion == f) {
                heroes.iter().for_each(|(&h, &p)| *suma.entry(h).or_insert(0) += p);
            }
        }
        x.ciudadanos
            .iter()
            .map(|&h| {
                let residencia = self.heroe(h).and_then(|hn| hn.heroe.residencia);
                let mut cargos = Vec::new();
                if x.rey == Some(h) {
                    cargos.push("rey".to_owned());
                }
                if x.embajador == Some(h) {
                    cargos.push("embajador".to_owned());
                }
                if let Some(Residencia::Asentamiento(a)) = residencia {
                    if let Some(plaza) = e.asentamientos.get(&u64::from(a)) {
                        let locales = politica::CARGOS.into_iter().filter(|&c| plaza.cargos.es_titular(c, h));
                        cargos.extend(locales.map(|c| format!("{}@{}", c.clave(), ids::asentamiento(u64::from(a)))));
                    }
                }
                Miembro { heroe: h, residencia, cargos, participacion: suma.get(&h).copied().unwrap_or(0) }
            })
            .collect()
    }

    /// Suma `puntos` a cada héroe en la plaza (si sigue en pie) y toca la plaza y su Facción.
    fn sumar_participacion(&mut self, plaza: u64, heroes: &[u64], puntos: u64) {
        let Some(f) = self.estado.asentamientos.get(&plaza).map(|a| a.faccion) else { return };
        if heroes.is_empty() || puntos == 0 {
            return;
        }
        let de_la_plaza = self.estado.participacion.entry(plaza).or_default();
        for &h in heroes {
            *de_la_plaza.entry(h).or_insert(0) += puntos;
        }
        self.tr.tocar(Clase::Asentamiento, plaza);
        self.tr.tocar(Clase::Faccion, u64::from(f));
    }

    /// Lo que da una batalla con combate (Participacion §2): `defensa` en cada plaza de `defendidas` a cada uno de `defensores`, y `batallaGanada` a cada uno de `ganadores` en la plaza donde
    /// reside (el que reside en un campamento no suma).
    pub(super) fn participacion_de_batalla(&mut self, defendidas: &[u64], defensores: &[u64], ganadores: &[u64]) {
        let r = self.estado.reglas.economia.participacion.clone();
        let mut sin_repetir: Vec<u64> = defensores.to_vec();
        sin_repetir.sort_unstable();
        sin_repetir.dedup();
        let mut plazas = defendidas.to_vec();
        plazas.sort_unstable();
        plazas.dedup();
        for plaza in plazas {
            self.sumar_participacion(plaza, &sin_repetir, r.defensa);
        }
        let mut por_plaza: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
        for &h in ganadores {
            if let Some(Residencia::Asentamiento(a)) = self.heroe(h).and_then(|hn| hn.heroe.residencia) {
                let v = por_plaza.entry(u64::from(a)).or_default();
                if !v.contains(&h) {
                    v.push(h);
                }
            }
        }
        for (plaza, heroes) in por_plaza {
            self.sumar_participacion(plaza, &heroes, r.batalla_ganada);
        }
    }

    /// Los héroes de las escuadras con soldados de una escolta cedida: los que la defienden.
    pub(super) fn heroes_de_escolta(&self, escolta: &[Escuadra]) -> Vec<u64> {
        let mut v: Vec<u64> = escolta.iter().filter(|s| s.con_soldados()).filter_map(|s| self.escuadra(s.id).map(|(h, _)| h.heroe.id)).collect();
        v.sort_unstable();
        v.dedup();
        v
    }
}
