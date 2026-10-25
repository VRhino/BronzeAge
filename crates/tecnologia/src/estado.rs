//! El estado de tecnología del servidor (TA §1-§3): la Era, los contadores, los logros fijos, las descubridoras (`primeros`), lo que los Aedas ya
//! cantaron y las tecnologías de cada Facción. Sin tick (decisión técnica 2): el núcleo suma al ocurrir el hecho ([`EstadoTecnologia::aplicar`]), mira
//! la Era al fijar un logro y en la cita de fin de plazo ([`EstadoTecnologia::fin_del_plazo`]) y evalúa la aparición con los disparadores del plan §5.1
//! (`hito::aparecer_por_hito`).

use crate::contador::{Contador, Delta};
use crate::datos::Reglas;
use crate::{Conjunto, FaccionId, TecId};
use base::Instante;
use std::collections::BTreeMap;

/// La primera Facción que desbloqueó una tecnología (la descubridora), por hito, compra o épica.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Primero {
    pub faccion: FaccionId,
    pub en: Instante,
}

/// Las tecnologías de una Facción: oculta → revelada → aparecida → adoptada. `adoptadas ⊆ aparecidas`; `reveladas` y `aparecidas` no se cruzan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TecnologiasFaccion {
    pub aparecidas: Conjunto,
    pub adoptadas: Conjunto,
    /// Un Aeda se las mostró y aún no le han aparecido.
    pub reveladas: Conjunto,
}

impl TecnologiasFaccion {
    /// La de una Facción recién nacida: las de arranque, aparecidas y adoptadas.
    pub fn de_arranque(r: &Reglas) -> Self {
        TecnologiasFaccion { aparecidas: r.de_arranque, adoptadas: r.de_arranque, reveladas: Conjunto::VACIO }
    }
}

/// Empieza una Era nueva (`era.comienza`, público).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CambioEra {
    /// Índice de la Era nueva en `Reglas::eras`.
    pub era: u8,
    /// Llegó por plazo y no por cumplir todos los logros de la anterior.
    pub por_plazo: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EstadoTecnologia {
    /// Índice de la Era vigente en `Reglas::eras`.
    pub era: u8,
    pub era_desde: Instante,
    /// Por [`Contador::indice`]. Nunca bajan.
    pub contadores: [u64; 24],
    /// Por [`TecId`]: cuándo se cumplió el logro. Fijo para siempre.
    pub logros: Vec<Option<Instante>>,
    /// Por [`TecId`].
    pub primeros: Vec<Option<Primero>>,
    /// Descubrimientos que los Aedas ya han cantado.
    pub cantadas: Conjunto,
    /// Una Facción sin entrada solo tiene las de arranque.
    pub por_faccion: BTreeMap<FaccionId, TecnologiasFaccion>,
}

impl EstadoTecnologia {
    /// `estadoTecnologiaInicial`: el servidor arranca en la Era I, sin logros.
    pub fn nuevo(r: &Reglas, desde: Instante) -> Self {
        let n = r.catalogo.len();
        EstadoTecnologia { era: 0, era_desde: desde, contadores: [0; 24], logros: vec![None; n], primeros: vec![None; n], cantadas: Conjunto::VACIO, por_faccion: BTreeMap::new() }
    }

    /// `tecnologiasDe`.
    pub fn de(&self, r: &Reglas, f: FaccionId) -> TecnologiasFaccion {
        self.por_faccion.get(&f).copied().unwrap_or_else(|| TecnologiasFaccion::de_arranque(r))
    }

    /// La entrada de la Facción para cambiarla (la crea con las de arranque).
    pub fn de_mut(&mut self, r: &Reglas, f: FaccionId) -> &mut TecnologiasFaccion {
        self.por_faccion.entry(f).or_insert_with(|| TecnologiasFaccion::de_arranque(r))
    }

    /// `tieneTecnologia`: ¿la tiene **adoptada**? Es lo que miran las cuatro puertas (TA §4.1).
    pub fn tiene(&self, r: &Reglas, f: FaccionId, t: TecId) -> bool {
        self.de(r, f).adoptadas.contiene(t)
    }

    pub fn contador(&self, c: Contador) -> u64 {
        self.contadores[c.indice()]
    }

    pub fn logro_cumplido(&self, t: TecId) -> bool {
        self.logros[t as usize].is_some()
    }

    /// ¿Ha empezado ya la Era de la tecnología?
    pub fn era_abierta(&self, r: &Reglas, t: TecId) -> bool {
        r.orden_era(t) <= r.eras[self.era as usize].orden
    }

    /// El umbral del siguiente logro pendiente del contador (`None` si ya no le queda ninguno). Los logros de un contador vienen ordenados por umbral
    /// y son a lo sumo un par: mirarlo es O(1) en cada suma.
    pub fn umbral_pendiente(&self, r: &Reglas, c: Contador) -> Option<u64> {
        r.por_contador[c.indice()].iter().find(|(_, t)| !self.logro_cumplido(*t)).map(|&(u, _)| u)
    }

    /// Suma `n` a un contador y fija los logros que cruzan su umbral (`tecnologia.logro { contador, umbral }`, público). Devuelve los fijados, por umbral.
    /// Un récord no se suma: [`EstadoTecnologia::subir_record`].
    pub fn sumar(&mut self, r: &Reglas, c: Contador, n: u64, ahora: Instante) -> Vec<TecId> {
        debug_assert!(!c.es_record(), "{} es un récord", c.nombre());
        if n == 0 {
            return Vec::new();
        }
        self.contadores[c.indice()] += n;
        self.fijar_logros(r, c, ahora)
    }

    /// Suma un [`Delta`] entero.
    pub fn aplicar(&mut self, r: &Reglas, d: &Delta, ahora: Instante) -> Vec<TecId> {
        d.iter().flat_map(|(c, n)| self.sumar(r, c, n, ahora)).collect()
    }

    /// `plazasEnNivel.N`: guarda `valor` si supera el récord (una plaza que sube, baja y vuelve a subir no cuenta dos veces).
    pub fn subir_record(&mut self, r: &Reglas, c: Contador, valor: u64, ahora: Instante) -> Vec<TecId> {
        debug_assert!(c.es_record());
        if valor <= self.contadores[c.indice()] {
            return Vec::new();
        }
        self.contadores[c.indice()] = valor;
        self.fijar_logros(r, c, ahora)
    }

    fn fijar_logros(&mut self, r: &Reglas, c: Contador, ahora: Instante) -> Vec<TecId> {
        let valor = self.contadores[c.indice()];
        let mut fijados = Vec::new();
        for &(umbral, t) in &r.por_contador[c.indice()] {
            if umbral > valor {
                break;
            }
            if self.logros[t as usize].is_none() {
                self.logros[t as usize] = Some(ahora);
                fijados.push(t);
            }
        }
        fijados
    }

    /// ¿Tienen su logro todas las tecnologías con logro de la Era vigente?
    pub fn todos_los_logros_de_la_era(&self, r: &Reglas) -> bool {
        r.catalogo.iter().enumerate().filter(|(_, d)| d.era == self.era && d.logro.is_some()).all(|(t, _)| self.logros[t].is_some())
    }

    /// Cuándo acaba el plazo de la Era vigente (la cita global, prioridad 10). `None` en la última (decisión 15: sin los bloques de las Eras IV y V,
    /// la III es la última y no tiene cita).
    pub fn fin_del_plazo(&self, r: &Reglas) -> Option<Instante> {
        (self.era as usize + 1 < r.eras.len()).then(|| self.era_desde + r.eras[self.era as usize].plazo)
    }

    /// TA §3 paso 2: la siguiente Era empieza si se cumplen todos los logros de la vigente o se agota su plazo. **Una sola Era por llamada**, aunque se
    /// cumplieran las de dos. Si llega por plazo, los logros pendientes de la anterior siguen abiertos.
    pub fn avanzar_era(&mut self, r: &Reglas, ahora: Instante) -> Option<CambioEra> {
        let fin = self.fin_del_plazo(r)?;
        let todos = self.todos_los_logros_de_la_era(r);
        if !todos && ahora < fin {
            return None;
        }
        self.era += 1;
        self.era_desde = ahora;
        Some(CambioEra { era: self.era, por_plazo: !todos })
    }

    /// Las candidatas a aparecer a la Facción, en el orden del catálogo: no de arranque, no aparecidas, con el logro cumplido y su Era abierta.
    pub fn candidatas<'a>(&'a self, r: &'a Reglas, f: FaccionId) -> impl Iterator<Item = TecId> + 'a {
        let propias = self.de(r, f);
        (0..r.catalogo.len() as TecId).filter(move |&t| !r.def(t).de_arranque && !propias.aparecidas.contiene(t) && self.logro_cumplido(t) && self.era_abierta(r, t))
    }

    /// `hacerAparecer`: la tecnología pasa a aparecida (y sale de las reveladas) sin pasar por el hito, por cualquier vía (hito, compra, épica).
    /// Si nadie la tenía, la Facción queda como descubridora. Devuelve `None` si ya le había aparecido, y si no, si es la primera del mundo.
    pub fn hacer_aparecer(&mut self, r: &Reglas, f: FaccionId, t: TecId, ahora: Instante) -> Option<bool> {
        let propias = self.de_mut(r, f);
        if propias.aparecidas.contiene(t) {
            return None;
        }
        propias.aparecidas.poner(t);
        propias.reveladas.quitar(t);
        let primera = self.primeros[t as usize].is_none();
        if primera {
            self.primeros[t as usize] = Some(Primero { faccion: f, en: ahora });
        }
        Some(primera)
    }

    /// Desde cuándo la conocen los Aedas, si tiene logro y descubridora: `primeros.en + retrasoConocimiento` (la cita de conocimiento; la Era se mira
    /// aparte con [`EstadoTecnologia::conocida_por_aedas`]).
    pub fn conocida_desde(&self, r: &Reglas, t: TecId) -> Option<Instante> {
        let p = self.primeros[t as usize]?;
        self.logro_cumplido(t).then_some(p.en + r.aedas.retraso_conocimiento)
    }

    /// `conocidaPorAedas` (TA §6.1): logro cumplido, descubridora hace al menos el retraso y su Era abierta.
    pub fn conocida_por_aedas(&self, r: &Reglas, t: TecId, ahora: Instante) -> bool {
        self.conocida_desde(r, t).is_some_and(|d| d <= ahora) && self.era_abierta(r, t)
    }

    /// TA §3 paso 4: los descubrimientos que los Aedas ya conocen y aún no han cantado (`aedas.canta_descubrimiento { tecnologia, descubridora }`,
    /// público). Los marca como cantados.
    pub fn cantar(&mut self, r: &Reglas, ahora: Instante) -> Vec<(TecId, FaccionId)> {
        let nuevas: Vec<(TecId, FaccionId)> = (0..r.catalogo.len() as TecId)
            .filter(|&t| !self.cantadas.contiene(t) && self.conocida_por_aedas(r, t, ahora))
            .map(|t| (t, self.primeros[t as usize].expect("conocida").faccion))
            .collect();
        nuevas.iter().for_each(|&(t, _)| self.cantadas.poner(t));
        nuevas
    }
}

#[cfg(test)]
pub(crate) mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/tecnologia.test.ts` («estado de tecnología», el logro y la Era de «avance de tecnología»).
    use super::*;
    use crate::contador::Delta;
    use crate::datos::SEMANA;

    pub fn r() -> Reglas {
        Reglas::default()
    }

    pub fn t(r: &Reglas, id: &str) -> TecId {
        r.buscar(id).unwrap_or_else(|| panic!("{id}"))
    }

    #[test]
    fn el_servidor_arranca_en_la_era_i_y_una_faccion_sin_entrada_solo_tiene_las_de_arranque() {
        let r = r();
        let e = EstadoTecnologia::nuevo(&r, 0);
        assert_eq!(r.eras[e.era as usize].id, "reinos_palaciales");
        assert_eq!(e.de(&r, 1).adoptadas, r.de_arranque);
        assert!(e.tiene(&r, 1, t(&r, "leva_comunal")));
        assert!(!e.tiene(&r, 1, t(&r, "metalurgia_cobre")));
    }

    #[test]
    fn el_logro_se_fija_al_cruzar_el_umbral_una_vez_y_para_siempre() {
        let r = r();
        let cobre = t(&r, "metalurgia_cobre");
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        assert_eq!(e.umbral_pendiente(&r, Contador::ExtraidoCobre), Some(41_000));
        assert!(e.sumar(&r, Contador::ExtraidoCobre, 40_999, 5).is_empty());
        assert_eq!(e.sumar(&r, Contador::ExtraidoCobre, 1, 7), vec![cobre]);
        assert_eq!(e.logros[cobre as usize], Some(7));
        assert_eq!(e.umbral_pendiente(&r, Contador::ExtraidoCobre), None);
        assert!(e.sumar(&r, Contador::ExtraidoCobre, 1_000_000, 9).is_empty(), "fijo: no se vuelve a fijar");
        assert_eq!(e.logros[cobre as usize], Some(7));
        assert_eq!(e.contador(Contador::ExtraidoCobre), 1_041_000, "ningún contador baja ni se recorta");
    }

    #[test]
    fn plazas_en_nivel_es_un_record() {
        let r = r();
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        assert!(e.subir_record(&r, Contador::PlazasEnNivel2, 11, 1).is_empty());
        assert!(e.subir_record(&r, Contador::PlazasEnNivel2, 3, 2).is_empty());
        assert_eq!(e.contador(Contador::PlazasEnNivel2), 11, "bajar no resta");
        assert_eq!(e.subir_record(&r, Contador::PlazasEnNivel2, 12, 3), vec![t(&r, "armamento_palacial")]);
    }

    #[test]
    fn la_era_avanza_al_agotar_el_plazo_o_al_cumplir_todos_sus_logros_lo_que_llegue_antes() {
        let r = r();
        let inicial = EstadoTecnologia::nuevo(&r, 0);
        assert_eq!(inicial.fin_del_plazo(&r), Some(5 * SEMANA));
        let mut e = inicial.clone();
        assert_eq!(e.avanzar_era(&r, 5 * SEMANA - 1), None);
        assert_eq!(e.avanzar_era(&r, 5 * SEMANA), Some(CambioEra { era: 1, por_plazo: true }));
        assert_eq!(e.era_desde, 5 * SEMANA);

        let mut todos = Delta::default();
        let mut e = inicial.clone();
        for d in r.catalogo.iter().filter(|d| d.era == 0) {
            if let Some(l) = d.logro {
                if l.contador.es_record() {
                    e.subir_record(&r, l.contador, l.umbral, 10);
                } else {
                    todos = todos.con(l.contador, l.umbral);
                }
            }
        }
        e.aplicar(&r, &todos, 10);
        assert!(e.todos_los_logros_de_la_era(&r));
        assert_eq!(e.avanzar_era(&r, 10), Some(CambioEra { era: 1, por_plazo: false }));
        assert_eq!(e.avanzar_era(&r, 10), None, "una sola Era por evaluación");
    }

    #[test]
    fn la_ultima_era_no_tiene_plazo() {
        let r = r();
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        e.era = 2;
        assert_eq!(e.fin_del_plazo(&r), None);
        assert_eq!(e.avanzar_era(&r, 100 * SEMANA), None);
    }

    #[test]
    fn un_logro_de_una_era_posterior_queda_fijado_pero_no_es_candidata_hasta_que_empieza() {
        let r = r();
        let civicas = t(&r, "instituciones_civicas");
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        assert_eq!(e.subir_record(&r, Contador::PlazasEnNivel3, 99, 1), vec![civicas]);
        assert!(!e.candidatas(&r, 1).any(|x| x == civicas));
        e.era = 2;
        assert!(e.candidatas(&r, 1).any(|x| x == civicas));
    }

    #[test]
    fn hacer_aparecer_registra_la_descubridora_una_vez_y_saca_de_las_reveladas() {
        let r = r();
        let cobre = t(&r, "metalurgia_cobre");
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        e.de_mut(&r, 2).reveladas.poner(cobre);
        assert_eq!(e.hacer_aparecer(&r, 2, cobre, 5), Some(true));
        assert_eq!(e.hacer_aparecer(&r, 2, cobre, 6), None);
        assert_eq!(e.hacer_aparecer(&r, 3, cobre, 7), Some(false));
        assert_eq!(e.primeros[cobre as usize], Some(Primero { faccion: 2, en: 5 }));
        let f2 = e.de(&r, 2);
        assert!(f2.aparecidas.contiene(cobre) && !f2.adoptadas.contiene(cobre) && !f2.reveladas.contiene(cobre));
    }

    #[test]
    fn los_aedas_cantan_cada_descubrimiento_una_vez_tras_el_retraso() {
        let r = r();
        let cobre = t(&r, "metalurgia_cobre");
        let mut e = EstadoTecnologia::nuevo(&r, 0);
        e.sumar(&r, Contador::ExtraidoCobre, 41_000, 0);
        e.hacer_aparecer(&r, 1, cobre, 0);
        let retraso = r.aedas.retraso_conocimiento;
        assert_eq!(e.conocida_desde(&r, cobre), Some(retraso));
        assert!(e.cantar(&r, retraso - 1).is_empty());
        assert_eq!(e.cantar(&r, retraso), vec![(cobre, 1)]);
        assert!(e.cantar(&r, retraso + 1).is_empty());
    }
}
