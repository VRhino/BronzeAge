//! A qué ámbitos de jugador pertenece cada entidad (4H T3b): el núcleo recuerda la pertenencia de cada una y, al cerrar cada versión, deja en el registro de cambios la de las que se
//! tocaron (`Pertenencia`: a qué ámbitos pertenecen ahora y de cuáles salieron). Con ello la sesión sabe a quién contar cada cambio, y a quién decirle `QUITA` cuando una entidad
//! desaparece (una columna disuelta, una caravana destruida) o deja de ser suya (una caravana que se aparca, una plaza conquistada). La regla de qué ámbito recibe qué es
//! [`audiencia`](crate::proyeccion::audiencia).
//!
//! Solo se lleva si alguien lo va a leer: [`Nucleo::seguir_pertenencias`] lo activa (lo hace el actor de sesiones) y el batch y las pruebas del motor no pagan nada.
//!
//! Hay cambios que arrastran a otras entidades sin tocarlas: si una plaza cambia de dueño, sus pueblos, avanzadas, trueques, órdenes y caravanas cambian de Facción; si un héroe pasa a
//! ser el de su sesión (se crea el héroe de verdad de una cuenta que tenía columnas de campo), sus columnas, escuadras y caravanas cambian de ámbito. En esos casos se revisa toda la
//! clase afectada y solo se anotan las que cambiaron de ámbito.
//!
//! DUDA: cuesta ≈ 1 µs por versión con 500 columnas de héroes de campo (el peor caso: cada participante busca el héroe de la sesión de su cuenta; con héroes de verdad es una consulta). Un pueblo, una avanzada
//! o una caravana tocados buscan su plaza o su columna recorriéndolas; el reparto a 500 sesiones es de T6.

use super::*;
use crate::estado::Pertenencia;
use crate::proyeccion::ambito::Ambito;
use crate::proyeccion::audiencia::{self, CLASES};
use std::collections::BTreeSet;

pub(super) type Mapa = BTreeMap<(Clase, u64), Vec<Ambito>>;

/// La pertenencia de las escuadras de un héroe: la de su dueño.
fn escuadras_de(e: &Estado, heroe: u64) -> impl Iterator<Item = (u64, Vec<Ambito>)> + '_ {
    let ambitos = audiencia::de(e, Clase::Heroe, heroe);
    e.militar.heroes.get(&heroe).into_iter().flat_map(|h| &h.heroe.escuadras).map(move |s| (s.id, ambitos.clone()))
}

/// La pertenencia de una escuadra por su id: la de su dueño, o ninguna si ya no existe.
fn de_escuadra(e: &Estado, escuadra: u64) -> Vec<Ambito> {
    e.militar.heroes.values().find(|h| h.heroe.escuadra(escuadra).is_some()).map(|h| audiencia::de(e, Clase::Heroe, h.heroe.id)).unwrap_or_default()
}

/// La pertenencia de todas las entidades, desde cero.
fn de_todo(e: &Estado) -> Mapa {
    let mut m = Mapa::new();
    for clase in CLASES {
        let ahora: Vec<(u64, Vec<Ambito>)> = if clase == Clase::Escuadra {
            e.militar.heroes.keys().flat_map(|&h| escuadras_de(e, h)).collect()
        } else {
            audiencia::todas(e, clase).into_iter().map(|id| (id, audiencia::de(e, clase, id))).collect()
        };
        m.extend(ahora.into_iter().filter(|(_, a)| !a.is_empty()).map(|(id, a)| ((clase, id), a)));
    }
    m
}

impl Nucleo {
    /// Empieza a llevar la pertenencia de las entidades a los ámbitos de jugador y a dejarla en el registro de cambios (lo hace el actor de sesiones al arrancar).
    pub fn seguir_pertenencias(&mut self) {
        self.pertenencias = Some(de_todo(&self.estado));
        self.pertenencias_desde = self.estado.version;
    }

    /// La primera versión cuyo registro de cambios lleva pertenencias: un cliente que conserve una anterior (de antes de arrancar las sesiones, o de la repetición del registro al recuperar) no puede reanudar por deltas.
    pub fn pertenencias_desde(&self) -> u64 {
        self.pertenencias_desde
    }

    /// Las entidades que pertenecen a un ámbito de jugador, por clase e id (vacío si no se llevan las pertenencias).
    pub fn entidades_de(&self, ambito: Ambito) -> Vec<(Clase, u64)> {
        self.pertenencias.iter().flatten().filter(|(_, a)| a.contains(&ambito)).map(|(&k, _)| k).collect()
    }

    /// Calcula la pertenencia de lo que cambió en esta versión y la compara con la que tenía; devuelve lo que hay que decir y deja el mapa al día.
    pub(super) fn cerrar_pertenencias(&mut self, tocados: &Tocados) -> Vec<Pertenencia> {
        let Some(mut mapa) = self.pertenencias.take() else { return Vec::new() };
        let e = &self.estado;
        let mut ahora: Mapa = BTreeMap::new();
        for clase in CLASES.into_iter().filter(|&c| c != Clase::Escuadra) {
            ahora.extend(tocados.de(clase).iter().map(|&id| ((clase, id), audiencia::de(e, clase, id))));
        }
        // Las caravanas que lleva una columna tocada y las escuadras de un héroe tocado (que `cerrar_version` ya dio por tocadas) pertenecen con su columna y con su dueño.
        for x in tocados.de(Clase::Ejercito).iter().filter_map(|id| e.ejercitos.get(id)) {
            for &c in &x.caravanas_adjuntas {
                ahora.entry((Clase::Caravana, c)).or_insert_with(|| audiencia::de(e, Clase::Caravana, c));
            }
        }
        for &h in tocados.de(Clase::Heroe) {
            ahora.extend(escuadras_de(e, h).map(|(s, a)| ((Clase::Escuadra, s), a)));
        }
        for &s in tocados.de(Clase::Escuadra) {
            ahora.entry((Clase::Escuadra, s)).or_insert_with(|| de_escuadra(e, s));
        }
        // Una plaza que cambia de dueño o un héroe que deja de ser el de su sesión arrastran a otras clases enteras (las que nacen sin dependientes no cuentan).
        let antes_de = |c: Clase, id: u64| mapa.get(&(c, id)).map_or(&[][..], Vec::as_slice);
        let cambia = |clase: Clase, relevante: &dyn Fn(u64, &[Ambito]) -> bool| ahora.iter().any(|(&(c, id), a)| c == clase && a.as_slice() != antes_de(c, id) && relevante(id, antes_de(c, id)));
        let con_dependientes = |_: u64, antes: &[Ambito]| !antes.is_empty();
        let cuenta_con_varios = |id: u64, antes: &[Ambito]| !antes.is_empty() || e.militar.heroes.get(&id).is_some_and(|h| e.militar.heroes.values().filter(|o| o.cuenta == h.cuenta).count() > 1);
        let mut revisar: BTreeSet<Clase> = BTreeSet::new();
        if cambia(Clase::Asentamiento, &con_dependientes) {
            revisar.extend([Clase::Pueblo, Clase::Avanzada, Clase::OrdenMercado, Clase::Acuerdo, Clase::Caravana]);
        }
        if cambia(Clase::Heroe, &cuenta_con_varios) {
            revisar.extend([Clase::Ejercito, Clase::Escuadra, Clase::Caravana, Clase::Batalla]);
        }
        for clase in revisar {
            let conocidas: Vec<u64> = mapa.range((clase, 0)..=(clase, u64::MAX)).map(|(&(_, id), _)| id).collect();
            if clase == Clase::Escuadra {
                for &h in e.militar.heroes.keys() {
                    for (s, a) in escuadras_de(e, h) {
                        ahora.entry((clase, s)).or_insert(a);
                    }
                }
                for s in conocidas {
                    ahora.entry((clase, s)).or_insert_with(|| de_escuadra(e, s));
                }
            } else {
                for id in audiencia::todas(e, clase).into_iter().chain(conocidas) {
                    ahora.entry((clase, id)).or_insert_with(|| audiencia::de(e, clase, id));
                }
            }
        }
        let mut registro = Vec::new();
        for ((clase, id), a) in ahora {
            let antes = if a.is_empty() { mapa.remove(&(clase, id)) } else { mapa.insert((clase, id), a.clone()) }.unwrap_or_default();
            let quitan: Vec<Ambito> = antes.iter().filter(|x| !a.contains(x)).copied().collect();
            let anotar = if tocados.contiene(clase, id) { !a.is_empty() || !quitan.is_empty() } else { a != antes };
            if anotar {
                registro.push(Pertenencia { clase, id, ahora: a, quitan });
            }
        }
        self.pertenencias = Some(mapa);
        registro
    }
}
