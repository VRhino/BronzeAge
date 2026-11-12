//! Los bandidos en el espacio (Doc 1.9, Generacion_Del_Mundo §13, Combate §7.2, fase 4D). Un campamento de bandidos es una entidad **fija** de la rejilla de
//! encuentros, igual que una plaza: no tiene plan, pero cuenta para el cruce con las caravanas.
//!
//! # Cuándo aparecen
//! Todo es un evento de prioridad 7 (`PRIO_BANDIDOS`: los bandidos ven a las columnas tal como estaban antes de moverse), uno por cosa que espera y ninguno por
//! cosa ociosa:
//! - **Hasta `porPlaza` (10) por plaza, de golpe** (`SucesoCombate::BandidosDeLaPlaza`; 2026-11-02, decisión del autor: antes uno y a las 6 h): al fundarla llegan
//!   todos, y al destruirse uno sin oleada pendiente se programa la siguiente a `respawnMinutos` (2 h), que repone de golpe los que falten entonces. Cada uno en el
//!   bosque no reclamado más cercano a ella, sin campamento encima y con el centro a ≥ `margenZonaPlaza` (5 u) del borde de toda zona, con su tirada de nivel. Sin
//!   bosque que valga no gasta tirada y espera: cuando un bosque queda libre o válido (se va un campamento de bandidos que lo ocupaba o una plaza cae en ruinas y su
//!   zona desaparece), las plazas que esperan los suyos los vuelven a buscar en ese instante, por orden de id (el TypeScript volvía a mirar cada tick).
//! - **En el anillo de cada campamento de mercenarios** (`SucesoCombate::BandidosDelAnillo`): `clamp(residentes sin plaza × 4, 5, 10)` campamentos. Por debajo
//!   del mínimo (5) llegan **de golpe** los que falten hasta él (la partida nace con 5 en cada anillo; 2026-11-03, decisión del autor); por encima, uno cada
//!   `reaparicionMinutos` mientras falten. La cita arranca con el mundo, se renueva al crear uno o al destruirse uno del anillo, se **rehace al cambiar la demanda**
//!   (llega o se va un residente del campamento, o una Facción se queda sin plazas; [`Nucleo::reprogramar_anillo`]) y se detiene cuando no falta ninguno (el
//!   TypeScript miraba la demanda cada tick). Sin sitio válido tras `INTENTOS_ANILLO` intentos (dos tiradas cada uno) reintenta a los `reaparicionMinutos`.
//! - **Los del mundo**: el artefacto trae los campamentos de las vetas de estaño y oro (`bandidos_iniciales`); nacen con la partida, no reaparecen.
//! - **Los de administración** (`ColocarBandidos`): para escenarios y pruebas.
//!
//! # Cómo acosan
//! A una caravana que pasa a ≤ `radioAtaqueCaravana` de un campamento: el cruce es un evento exacto del mismo tipo que los encuentros
//! (`Carga::Encuentro`), pero de prioridad 7. Se programa cuando la caravana estrena plan o cuando el campamento aparece, y al vencer se mira con el estado de ese
//! instante: una caravana parada en una plaza, dentro de cualquier zona o en la protección de un campamento de mercenarios no es presa. Una tirada: gana el
//! bandido si `poder × tirada > defensa` (la escolta cedida, o los carreteros); la caravana desaparece con su carga y su escolta queda a 0 y vuelve al
//! campamento de su héroe. Una caravana que se salva no vuelve a ser acosada por ese campamento hasta que cambie de plan. Las caravanas adjuntas a un ejército
//! se defienden con el poder de la columna, que no sufre bajas: el cruce lo programa la columna en marcha que las lleva.
//!
//! # Con servidores de batalla (quinta ronda de Entrar con todo, 2026-10-22)
//! Los bandidos no son bots ni jugadores. Si el acoso es a una columna que lleva caravanas adjuntas y **algún héroe sano** y el proceso tiene servidores de batalla, no se tira: se abre una
//! batalla de caravana ([`Nucleo::apertura_de_acoso`]) con **todas** las caravanas adjuntas que son presa (decisión del autor, 2026-10-22), en la que atacan los bandidos y defienden los héroes de la columna, y Unity decide (haya humanos o no). Una caravana sin héroes
//! —suelta, con escolta de tropa cedida o sin ella, o adjunta a una columna sin héroes sanos— se resuelve **siempre con números**, haya servidores o no. Un campamento no vuelve a
//! acosar a la misma columna mientras se conserve su batalla ([`Nucleo::acoso_ya_jugado`]).

use super::contratos::Desenlace;
use super::*;
use crate::estado::{SucesoCombate, PRIO_BANDIDOS};
use crate::estado_combate::{entidad_anillo, entidad_bandidos, es_entidad_bandidos, CampamentoBandidos, OrigenBandidos};
use crate::estado_comercio::es_entidad_caravana;
use crate::estado_fundacion::{es_entidad_fundacion, Origen as OrigenFundacion, Situacion as SituacionFundacion};
use ::encuentros::{plan_quieto, Entradas};
use ::mercenarios::Objetivo;
use combate::bandidos::{campamentos_que_tocan, cuantos_crear_en_el_anillo, faltan_junto_a_plaza, nivel_al_azar, proxima_aparicion_en_el_anillo, punto_del_anillo, puede_acosar, reaparece_en, resolver_acoso, resolver_emboscada};
use combate::resolucion::Bando;
use super::combates::caidos;
use asentamiento::Recurso;
use militar::Carga as Mercancia;

/// A dónde va el botín de unos bandidos vencidos al defenderse de ellos.
#[derive(Clone, Copy)]
pub(super) enum Botin {
    Columna(u64),
    Caravana(u64),
}
use combate::caravana::{Defensa, Zona};
use militar::escuadra::Escuadra;
use std::collections::BTreeSet;

/// Lo cobrado de un campamento destruido, tal como lo llevan sus hechos: madera, piedra y trigo (lo que cupo en el carro) y las monedas.
pub(super) fn datos_del_botin(cogido: &Mercancia, monedas: i64) -> [(&'static str, i64); 4] {
    let de = |r| cogido.get(&r).copied().unwrap_or(0);
    [("madera", de(Recurso::Madera)), ("piedra", de(Recurso::Piedra)), ("trigo", de(Recurso::Trigo)), ("monedas", monedas)]
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Un campamento de bandidos por id.
    pub fn campamento_de_bandidos(&self, id: u64) -> Option<&CampamentoBandidos> {
        self.estado.combate.bandidos.get(&id)
    }

    /// Todos los campamentos de bandidos, por id.
    pub fn campamentos_de_bandidos(&self) -> impl Iterator<Item = &CampamentoBandidos> {
        self.estado.combate.bandidos.values()
    }

    // ───────────────────────────── Alta y baja ─────────────────────────────

    /// La partida nace: los campamentos que trae el artefacto de mundo (junto a las vetas) y la primera cita de cada anillo de mercenarios. Solo en un estado
    /// nuevo; una foto ya lo trae todo.
    pub(super) fn sembrar_el_mundo(&mut self) {
        let Some(geo) = &self.geografia else { return };
        let (t, n) = (self.estado.ahora, geo.mapa.campamentos.len() as u32);
        let del_artefacto: Vec<(Punto, u8)> = geo.bandidos_del_artefacto();
        for (i, (p, nivel)) in del_artefacto.into_iter().enumerate() {
            self.poner_bandidos(p, nivel, OrigenBandidos::Veta(i as u32), None, t);
        }
        for c in 0..n {
            self.programar_anillo(c, t);
        }
    }

    /// Pone un campamento de bandidos en el mapa: entra en la rejilla y programa el acoso a las caravanas que ya pasan por su radio.
    fn poner_bandidos(&mut self, posicion: Punto, nivel: u8, origen: OrigenBandidos, bosque: Option<u32>, t: Instante) -> u64 {
        let id = self.estado.combate.siguiente_bandidos;
        self.estado.combate.siguiente_bandidos += 1;
        self.estado.combate.bandidos.insert(id, CampamentoBandidos { id, posicion, nivel, origen, bosque });
        self.tr.tocar(Clase::Bandidos, id);
        let entidad = entidad_bandidos(id);
        let plan = plan_quieto(posicion, t, 0);
        self.espacio.rejilla.apuntar(entidad, self.estado.generaciones.actual(entidad), &plan);
        self.programar_acosos(entidad, &plan, Entradas::Todas);
        id
    }

    /// Destruye un campamento: sale del mapa y del espacio, sus citas pendientes mueren con su generación y, según de dónde saliera, se agenda el siguiente.
    pub(super) fn destruir_bandidos(&mut self, id: u64, t: Instante) {
        let Some(b) = self.estado.combate.bandidos.remove(&id) else { return };
        self.tr.tocar(Clase::Bandidos, id);
        let entidad = entidad_bandidos(id);
        self.estado.generaciones.subir(entidad);
        self.espacio.rejilla.quitar(entidad);
        match b.origen {
            // La oleada que repone: la primera baja la programa; las que caigan antes de que llegue las repone la misma.
            OrigenBandidos::Plaza(plaza) if !self.estado.combate.reaparecen.contains_key(&plaza) => {
                let en = reaparece_en(&self.estado.reglas.economia.combate, t);
                self.estado.combate.reaparecen.insert(plaza, en);
                self.tr.tocar(Clase::Asentamiento, plaza);
                self.programar_bandidos_de_plaza(plaza, en);
            }
            OrigenBandidos::Plaza(_) => {}
            OrigenBandidos::Anillo(c) => {
                let desde = self.estado.combate.anillos.get(&c).copied().unwrap_or(t).max(t);
                self.programar_anillo(c, desde);
            }
            OrigenBandidos::Veta(_) | OrigenBandidos::Admin => {}
        }
        if b.bosque.is_some() {
            self.plazas_sin_bosque_buscan(t);
        }
    }

    /// La plaza cae en ruinas: su campamento de bandidos desaparece con ella (sin reaparecer, `bandidos.campamento_disperso`, Comercio_Caravanas §10) y su plazo de reaparición también.
    pub(super) fn bandidos_de_plaza_arruinada(&mut self, plaza: u64, t: Instante) {
        let suyos: Vec<(u64, Punto)> = self.estado.combate.bandidos.values().filter(|b| b.origen == OrigenBandidos::Plaza(plaza)).map(|b| (b.id, b.posicion)).collect();
        for (id, punto) in suyos {
            let datos = vec![("campamento", id as i64), ("asentamientoArruinado", plaza as i64)];
            self.tr.emitir(&self.estado, EventoNucleo { asentamientos: vec![plaza], punto: Some(punto), ..EventoNucleo::nuevo("bandidos.campamento_disperso", t, datos) });
            self.estado.combate.bandidos.remove(&id);
            self.tr.tocar(Clase::Bandidos, id);
            self.estado.generaciones.subir(entidad_bandidos(id));
            self.espacio.rejilla.quitar(entidad_bandidos(id));
        }
        self.estado.combate.reaparecen.remove(&plaza);
        // Su zona ya no está (y quizá tampoco su campamento): algún bosque puede valer ahora para otra plaza.
        self.plazas_sin_bosque_buscan(t);
    }

    // ───────────────────────────── Las citas ─────────────────────────────

    /// Programa la aparición del campamento de una plaza en `en`.
    pub(super) fn programar_bandidos_de_plaza(&mut self, plaza: u64, en: Instante) {
        let clave = self.estado.generaciones.clave(plaza);
        self.estado.cola.programar(en, PRIO_BANDIDOS, clave, Carga::Combate(SucesoCombate::BandidosDeLaPlaza));
    }

    /// Programa la cita del anillo del campamento de mercenarios `c` en `en`.
    fn programar_anillo(&mut self, c: u32, en: Instante) {
        let clave = self.estado.generaciones.clave(entidad_anillo(c));
        self.estado.cola.programar(en, PRIO_BANDIDOS, clave, Carga::Combate(SucesoCombate::BandidosDelAnillo));
    }

    /// La demanda del anillo de `c` cambió: su cita se rehace (la pendiente muere con la generación). Si con la demanda de ahora falta algún campamento,
    /// vence en cuanto lo permita su plazo (`bandidosEn`); si no, no queda ninguna.
    pub(super) fn reprogramar_anillo(&mut self, c: u32, t: Instante) {
        self.estado.generaciones.subir(entidad_anillo(c));
        if self.faltan_en_el_anillo(c) {
            let desde = self.estado.combate.anillos.get(&c).copied().unwrap_or(t).max(t);
            self.programar_anillo(c, desde);
        }
    }

    /// Una Facción se quedó sin plazas: sus residentes en los campamentos de mercenarios pasan a pedir bandidos.
    pub(super) fn reprogramar_anillos(&mut self, t: Instante) {
        let n = self.geografia.as_ref().map_or(0, |g| g.mapa.campamentos.len() as u32);
        for c in 0..n {
            self.reprogramar_anillo(c, t);
        }
    }

    /// Cuentan los residentes cuya Facción no tiene plazas o que no tienen Facción: son los que necesitan bandidos que los acosen.
    fn residentes_sin_plaza(&self, c: u32) -> i64 {
        self.heroes().filter(|h| h.heroe.residencia == Some(militar::heroe::Residencia::Campamento(c)) && self.sin_plaza(h)).count() as i64
    }

    /// ¿Es un héroe **sin plaza**? Reside en un campamento de mercenarios y su Facción no tiene plazas, o no tiene Facción: el mismo criterio que la demanda
    /// del anillo (Cal-5, 2026-10-24).
    fn sin_plaza(&self, h: &crate::estado_militar::HeroeNucleo) -> bool {
        matches!(h.heroe.residencia, Some(militar::heroe::Residencia::Campamento(_))) && h.faccion.is_none_or(|f| self.plazas_de(f).next().is_none())
    }

    /// El botín que gana el héroe `heroe` al destruir un campamento de ese `origen` y ese `nivel`, con los rendimientos decrecientes de su ventana: devuelve
    /// los recursos (para el carro) y las monedas, que ya suma a las del héroe. **El botín de los bandidos del anillo de un campamento de mercenarios solo lo
    /// cobran los héroes sin plaza** (Cal-5, 2026-10-24: el anillo existe para el jugador nuevo); los demás ganan solo experiencia, y esa destrucción no
    /// cuenta en su ventana. Los de plaza, vetas y administración pagan a todos.
    pub(super) fn botin_de_bandidos_para(&mut self, heroe: u64, origen: OrigenBandidos, nivel: u8, t: Instante) -> (Mercancia, i64) {
        if matches!(origen, OrigenBandidos::Anillo(_)) && !self.heroe(heroe).is_some_and(|h| self.sin_plaza(h)) {
            return (Mercancia::new(), 0);
        }
        let mut recientes = self.estado.combate.destruidos.get(&heroe).cloned().unwrap_or_default();
        let (recursos, monedas) = combate::bandidos::botin_de_bandidos(&self.estado.reglas.economia.combate, &mut recientes, nivel, t);
        self.estado.combate.destruidos.insert(heroe, recientes);
        if self.heroe(heroe).is_some() {
            let h = &mut self.heroe_mut(heroe).heroe;
            h.monedas = h.monedas.saturating_add(monedas);
        }
        (recursos, monedas)
    }

    /// El botín de los héroes `heroes` al destruir un campamento: todos los recursos juntos (para el carro) y el total de monedas (ya en cada héroe).
    pub(super) fn botin_de_los_heroes(&mut self, heroes: impl IntoIterator<Item = u64>, origen: OrigenBandidos, nivel: u8, t: Instante) -> (Mercancia, i64) {
        let (mut recursos, mut monedas) = (Mercancia::new(), 0);
        for h in heroes {
            let (r, m) = self.botin_de_bandidos_para(h, origen, nivel, t);
            r.into_iter().for_each(|(recurso, n)| militar::sumar(&mut recursos, recurso, n));
            monedas += m;
        }
        (recursos, monedas)
    }

    /// Los campamentos que tiene ahora el anillo de `c`.
    fn en_el_anillo(&self, c: u32) -> i64 {
        self.estado.combate.bandidos.values().filter(|b| b.origen == OrigenBandidos::Anillo(c)).count() as i64
    }

    /// ¿Le faltan campamentos al anillo de `c` con la demanda de ahora?
    fn faltan_en_el_anillo(&self, c: u32) -> bool {
        self.en_el_anillo(c) < campamentos_que_tocan(&self.estado.reglas.economia.combate, self.residentes_sin_plaza(c))
    }

    /// Los campamentos que tiene ahora la plaza.
    fn bandidos_de(&self, plaza: u64) -> i64 {
        self.estado.combate.bandidos.values().filter(|b| b.origen == OrigenBandidos::Plaza(plaza)).count() as i64
    }

    /// Un bosque pudo quedar libre o válido: la primera plaza (por id) que esperaba los suyos —le faltan y su oleada venció o no tiene— los vuelve a buscar en este
    /// instante. Si los pone todos, su aparición llama otra vez aquí y busca la siguiente; la que no encuentra bosque sigue esperando, sin gastar tirada.
    fn plazas_sin_bosque_buscan(&mut self, t: Instante) {
        let cr = &self.estado.reglas.economia.combate;
        let primera = self.estado.asentamientos.keys().copied().find(|&p| faltan_junto_a_plaza(cr, self.bandidos_de(p), self.estado.combate.reaparecen.get(&p).copied(), t) > 0);
        if let Some(plaza) = primera {
            self.programar_bandidos_de_plaza(plaza, t);
        }
    }

    /// El bosque no reclamado más cercano a `desde`: sin campamento de bandidos encima y con el centro fuera de la zona de toda plaza (más el margen). Devuelve su
    /// índice y su centro. No gasta tiradas.
    fn bosque_libre_mas_cercano(&self, desde: Punto) -> Option<(u32, Punto)> {
        let geo = self.geografia.as_ref()?;
        let mut candidatos: Vec<(i128, u32, Punto)> = geo.bosques().iter().enumerate().filter_map(|(i, b)| b.centro.as_ref().map(|c| (Punto::new(c.x, c.y).dist2(desde), i as u32, Punto::new(c.x, c.y)))).collect();
        candidatos.sort_by_key(|&(d, i, _)| (d, i));
        let margen = self.estado.reglas.economia.combate.bandidos.margen_zona_plaza;
        let ocupados: BTreeSet<u32> = self.estado.combate.bandidos.values().filter_map(|b| b.bosque).collect();
        candidatos.into_iter().find(|&(_, i, centro)| !ocupados.contains(&i) && !self.estado.asentamientos.values().any(|a| a.posicion.dist(centro) < a.radio + margen)).map(|(_, i, c)| (i, c))
    }

    /// Vence la cita de la oleada de la plaza: llegan de golpe los que le falten, cada uno en el bosque libre más cercano que quede.
    pub(super) fn bandidos_de_la_plaza(&mut self, plaza: u64, t: Instante) {
        let Some(posicion) = self.estado.asentamientos.get(&plaza).map(|a| a.posicion) else { return };
        let cr = self.estado.reglas.economia.combate.clone();
        let faltan = faltan_junto_a_plaza(&cr, self.bandidos_de(plaza), self.estado.combate.reaparecen.get(&plaza).copied(), t);
        if faltan == 0 {
            return;
        }
        if self.estado.combate.reaparecen.remove(&plaza).is_some() {
            self.tr.tocar(Clase::Asentamiento, plaza);
        }
        for _ in 0..faltan {
            // Sin bosque que valga se queda esperando con los que tenga (sin oleada pendiente): ya no queda bosque para ninguna otra.
            let Some((bosque, centro)) = self.bosque_libre_mas_cercano(posicion) else { return };
            let nivel = nivel_al_azar(&cr, &mut self.estado.rng);
            let id = self.poner_bandidos(centro, nivel, OrigenBandidos::Plaza(plaza), Some(bosque), t);
            self.aparecio(id, t, vec![plaza]);
        }
        // Quizá quedaba bosque para la siguiente que espera (una ruina pudo dejar válidos varios).
        self.plazas_sin_bosque_buscan(t);
    }

    /// Vence la cita del anillo del campamento de mercenarios `c`.
    pub(super) fn bandidos_del_anillo(&mut self, c: u32, t: Instante) {
        let Some(centro) = self.geografia.as_ref().and_then(|g| g.mapa.campamentos.get(c as usize).copied()) else { return };
        if self.estado.combate.anillos.get(&c).is_some_and(|&desde| t < desde) {
            return;
        }
        let cr = self.estado.reglas.economia.combate.clone();
        // Por debajo del mínimo llegan de golpe los que falten hasta él; por encima, uno cada `reaparicionMinutos` mientras pida la demanda.
        let cuantos = cuantos_crear_en_el_anillo(&cr, self.en_el_anillo(c), self.residentes_sin_plaza(c));
        if cuantos == 0 {
            return;
        }
        let valido = |s: &Self, p: Punto| s.geografia.as_ref().is_some_and(|g| g.es_tierra_firme(p)) && s.zona_en(p, None) == Zona::Ninguna && !s.en_proteccion_de_campamento(p);
        let mut creados = vec![];
        for _ in 0..cuantos {
            // El sorteo del punto presta el generador de la partida: se saca del estado mientras se mira el mapa y se devuelve con lo que gastó.
            let mut rng = std::mem::replace(&mut self.estado.rng, base::Rng::nuevo(0));
            let punto = punto_del_anillo(&cr, centro, &mut rng, |p| valido(self, p));
            self.estado.rng = rng;
            // Sin sitio: las tiradas de los intentos se gastaron igual (el TypeScript también las gastaba) y se reintenta más tarde.
            let Some(punto) = punto else { break };
            let nivel = nivel_al_azar(&cr, &mut self.estado.rng);
            creados.push(self.poner_bandidos(punto, nivel, OrigenBandidos::Anillo(c), None, t));
        }
        let siguiente = proxima_aparicion_en_el_anillo(&cr, t);
        if !creados.is_empty() {
            self.estado.combate.anillos.insert(c, siguiente);
            self.tr.tocar(Clase::Campamento, u64::from(c));
        }
        self.programar_anillo(c, siguiente);
        for id in creados {
            self.aparecio(id, t, vec![]);
        }
    }

    fn aparecio(&mut self, id: u64, t: Instante, plazas: Vec<u64>) {
        let b = &self.estado.combate.bandidos[&id];
        let (punto, nivel) = (b.posicion, b.nivel);
        self.tr.emitir(&self.estado, EventoNucleo { codigo: Cow::Borrowed("bandidos.campamento_aparece"), en: t, ejercitos: vec![], asentamientos: plazas, punto: Some(punto), datos: vec![("campamento", id as i64), ("nivel", nivel as i64)], ..Default::default() });
    }

    // ───────────────────────────── El acoso ─────────────────────────────

    /// Programa los acosos de `entidad` (un campamento de bandidos o una caravana) con su `plan`: cada cruce a ≤ `radioAtaqueCaravana` con una caravana (si
    /// es un campamento) o con un campamento (si es una caravana) es un evento exacto de prioridad 7.
    pub(super) fn programar_acosos(&mut self, entidad: u64, plan: &Plan, entradas: Entradas) {
        let (t, radio) = (self.estado.ahora, self.estado.reglas.economia.combate.bandidos.radio_ataque_caravana);
        let es_campamento = es_entidad_bandidos(entidad);
        let clave = cola::Clave { entidad, generacion: self.estado.generaciones.actual(entidad) };
        for e in self.espacio.rejilla.encuentros_de(entidad, plan, radio, entradas) {
            let pareja = if es_campamento { self.atrae_bandidos(e.otra) } else { es_entidad_bandidos(e.otra) };
            if pareja {
                self.estado.cola.programar(e.instante.max(t), PRIO_BANDIDOS, clave, Carga::Encuentro { otra: e.otra, otra_generacion: e.otra_generacion });
            }
        }
    }

    /// ¿Atrae a los bandidos? Una caravana que viaja suelta (también la de Fundación que vuelve sola) y una columna que lleva caravanas adjuntas o una de Fundación.
    pub(super) fn atrae_bandidos(&self, entidad: u64) -> bool {
        match self.cosa(entidad) {
            Some(Cosa::Caravana(k)) => matches!(k.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando),
            Some(Cosa::Fundacion(k)) => matches!(k.situacion, SituacionFundacion::Retornando(_)),
            Some(Cosa::Columna(e)) => !e.caravanas_adjuntas.is_empty() || self.estado.fundacion.de_columna(e.id).is_some(),
            _ => false,
        }
    }

    /// Vence el cruce de un campamento con una caravana suelta o con una columna que lleva caravanas (`entidad` y `otra` son los dos, en cualquier orden).
    pub(super) fn acoso_vence(&mut self, entidad: u64, otra: u64, otra_generacion: u32, t: Instante) {
        if self.estado.generaciones.actual(otra) != otra_generacion {
            return;
        }
        let (camp, presa) = if es_entidad_bandidos(entidad) { (entidad, otra) } else { (otra, entidad) };
        let idc = crate::estado_comercio::id_de_entidad(camp);
        if !es_entidad_bandidos(camp) || !self.estado.combate.bandidos.contains_key(&idc) {
            return;
        }
        if es_entidad_caravana(presa) {
            let idk = crate::estado_comercio::id_de_entidad(presa);
            self.acosar(idc, idk, t);
        } else if es_entidad_fundacion(presa) {
            self.acosar_fundacion(idc, crate::estado_comercio::id_de_entidad(presa), t);
        } else if self.estado.ejercitos.get(&presa).is_some_and(|e| self.columna_en_el_mapa(e) && !self.acoso_ya_jugado(idc, e.id)) {
            self.acosar_columna(idc, presa, t);
        }
    }

    /// ¿Es presa de los bandidos una caravana en ese estado y en ese punto? No si está parada en una plaza (`disponible`, `preparando`, `aparcada`), ni dentro de cualquier zona, ni en la
    /// protección de un campamento de mercenarios. La distancia ya la garantiza el evento (exacto); la de las posiciones redondeadas podría pasarse de una milésima.
    pub(super) fn es_presa_de_bandidos(&self, estado: EstadoCaravana, donde: Punto) -> bool {
        puede_acosar(&self.estado.reglas.economia.combate, estado, 0, self.zona_en(donde, None) != Zona::Ninguna, self.en_proteccion_de_campamento(donde))
    }

    /// El campamento `idc` acosa a la columna `columna`, que lleva caravanas adjuntas. Con servidores de batalla y algún héroe sano a bordo, los bandidos y la columna se juegan **todas** las
    /// caravanas adjuntas en una sola batalla de Unity (quinta ronda de Entrar con todo, 2026-10-22; la columna queda apartada y ahí acaba el cruce). Si no, cada caravana adjunta tiene su
    /// propia tirada con números, en orden de id, y se defiende con el poder de la columna. Los bandidos no son bots ni jugadores: sin héroes, siempre con números.
    fn acosar_columna(&mut self, idc: u64, columna: u64, t: Instante) {
        if self.estado.reglas.batallas_en_unity {
            let donde = self.estado.ejercitos[&columna].posicion_en(t);
            if let Some(ap) = self.apertura_de_acoso(t, idc, columna, donde) {
                self.abrir_batalla(t, ap);
                return;
            }
        }
        self.emboscar_columna(idc, columna, t);
    }

    /// La emboscada con números (2026-11-06, pedido del autor): los bandidos atacan a la columna y es un combate de verdad, el ataque a un campamento al revés
    /// ([`combate::bandidos::resolver_emboscada`]). Defiende la tropa de la columna con sus héroes, que sufre bajas gane o pierda. Si ganan los bandidos se pierden **todas** las
    /// caravanas adjuntas que son presa (y la de Fundación), como en la batalla de Unity; si gana la columna, el campamento cae y da su oro y su experiencia como si lo hubiera atacado.
    fn emboscar_columna(&mut self, idc: u64, columna: u64, t: Instante) {
        let Some(nivel) = self.estado.combate.bandidos.get(&idc).map(|b| b.nivel) else { return };
        let donde = self.estado.ejercitos[&columna].posicion_en(t);
        let caravanas: Vec<u64> =
            self.estado.ejercitos[&columna].caravanas_adjuntas.iter().copied().filter(|&k| self.estado.comercio.caravana(k).is_some_and(|c| self.es_presa_de_bandidos(c.estado, donde))).collect();
        let fundacion = self.estado.fundacion.de_columna(columna).filter(|_| self.es_presa_de_bandidos(EstadoCaravana::Adjunta, donde));
        if caravanas.is_empty() && fundacion.is_none() {
            return;
        }
        let cr = self.estado.reglas.economia.combate.clone();
        let mut tropa = self.tropa_de_columna(columna, t, cr.batalla.capacidad_resto);
        let columnas = self.columnas_de(&tropa.escuadras);
        let participantes = Self::participantes_de(&self.estado.ejercitos[&columna]);
        let e = resolver_emboscada(&cr, nivel, Bando { escuadras: &mut tropa.escuadras, heroes: &tropa.heroes }, &mut self.estado.rng);
        self.experiencia_de_bando(t, &mut tropa.escuadras, &e.bajas, &tropa.heroes, !e.gana_el_bandido);
        self.volcar_escuadras(&tropa.escuadras);
        self.recomponer_tras_bajas(&columnas, t);
        let origenes: Vec<u64> = caravanas.iter().filter_map(|&k| self.estado.comercio.caravana(k)).map(|k| k.origen as u64).collect();
        // La participación (Participacion §2): la defensa a los de la columna; las misiones cuentan la batalla.
        self.participacion_de_batalla(t, &origenes, &participantes, &[]);
        self.avance_de_heroes(t, &participantes, "batallas", 1);
        let atacados: Vec<Objetivo> = caravanas.iter().map(|&k| Objetivo::Caravana(k)).collect();
        self.tras_batalla(t, &Desenlace { atacados: &atacados, atacantes: &[], defensores: &participantes, gana_atacante: e.gana_el_bandido, nivel_bandidos: None });
        let codigo = if e.gana_el_bandido { "bandidos.caravana_interceptada" } else { "bandidos.caravana_escapa" };
        for (&k, &origen) in caravanas.iter().zip(&origenes) {
            let plazas = vec![origen].into_iter().filter(|o| self.estado.asentamientos.contains_key(o)).collect();
            let datos = vec![("campamento", idc as i64), ("caravana", k as i64), ("poderBandido", e.poder_bandido), ("defensa", e.defensa), ("bajasEscolta", caidos(&e.bajas))];
            self.tr.emitir(&self.estado, EventoNucleo { codigo: Cow::Borrowed(codigo), en: t, ejercitos: vec![columna], asentamientos: plazas, punto: Some(donde), datos, ..Default::default() });
        }
        if let Some(x) = fundacion {
            let plazas = match self.estado.fundacion.caravanas.get(&x).map(|k| k.origen) {
                Some(OrigenFundacion::Plaza(a)) if self.estado.asentamientos.contains_key(&a) => vec![a],
                _ => vec![],
            };
            let datos = vec![("campamento", idc as i64), ("caravana", x as i64), ("poderBandido", e.poder_bandido), ("defensa", e.defensa), ("bajasEscolta", caidos(&e.bajas))];
            self.tr.emitir(&self.estado, EventoNucleo { codigo: Cow::Borrowed(codigo), en: t, ejercitos: vec![columna], asentamientos: plazas, punto: Some(donde), datos, ..Default::default() });
        }
        if e.gana_el_bandido {
            for k in caravanas {
                self.contar(t, tecnologia::contador::Delta::uno(tecnologia::Contador::CaravanasDestruidasOCapturadas));
                self.perder_caravana(t, k);
            }
            if let Some(x) = fundacion {
                self.contar(t, tecnologia::contador::Delta::uno(tecnologia::Contador::CaravanasDestruidasOCapturadas));
                let antes = self.velocidad_antes(columna);
                self.caer_fundacion(t, x, combate::caravana::Modalidad::Disputa);
                self.recomponer_por_fundacion(columna, t, antes);
            }
        } else {
            let faccion = self.estado.ejercitos[&columna].columna.faccion;
            self.bandidos_vencidos(t, idc, &tropa.heroes, Botin::Columna(columna), faccion, donde);
        }
    }

    /// Un campamento que ataca y pierde cae (2026-11-06, pedido del autor), con el premio de haberlo destruido: el botín de cada héroe que lo venció (a la columna que
    /// defendía o a la caravana escoltada, lo que quepa), la experiencia de bandidos de su Facción, el contador de campamentos destruidos y las misiones de bandidos.
    pub(super) fn bandidos_vencidos(&mut self, t: Instante, idc: u64, heroes: &[u64], botin: Botin, faccion: Option<u32>, donde: Punto) {
        let Some((nivel, origen)) = self.estado.combate.bandidos.get(&idc).map(|b| (b.nivel, b.origen)) else { return };
        let (recursos, monedas) = self.botin_de_los_heroes(heroes.iter().copied(), origen, nivel, t);
        let cogido = match botin {
            Botin::Columna(c) => self.cargar_botin_en(c, &recursos),
            Botin::Caravana(k) => self.cargar_botin_en_caravana(k, &recursos),
        };
        self.destruir_bandidos(idc, t);
        self.experiencia_de_bandidos(t, faccion);
        self.contar(t, tecnologia::contador::Delta::uno(tecnologia::Contador::BandidosCampamentosDestruidos));
        self.avance_de_heroes(t, heroes, "bandidos", 1);
        if let Some(f) = faccion {
            self.avance(t, Dueno::Faccion(f), "bandidos", 1);
        }
        let ejercitos = match botin {
            Botin::Columna(c) => vec![c],
            Botin::Caravana(_) => vec![],
        };
        self.tr.emitir(&self.estado, EventoNucleo { codigo: Cow::Borrowed("bandidos.campamento_vencido"), en: t, ejercitos, asentamientos: vec![], punto: Some(donde), datos: [("campamento", idc as i64), ("nivel", nivel as i64)].into_iter().chain(datos_del_botin(&cogido, monedas)).collect(), ..Default::default() });
    }

    /// El botín de unos bandidos vencidos en la caravana que los venció con su escolta: lo que quepa en su capacidad.
    fn cargar_botin_en_caravana(&mut self, k: u64, botin: &Mercancia) -> Mercancia {
        let flota = self.estado.reglas.economia.comercio.flota.clone();
        let Some(i) = self.estado.comercio.posicion_caravana(k) else { return Mercancia::new() };
        let c = &mut self.estado.comercio.flota[i];
        let capacidad = c.capacidad(&flota);
        let cogido = combate::botin::cargar_botin(&mut c.contenido, botin, capacidad);
        self.tr.tocar(Clase::Caravana, k);
        cogido
    }

    /// El campamento `idc` acosa con una tirada con números a la Caravana de Fundación `x` que vuelve sola (paridad con el TypeScript, P2): se defiende con los carreteros
    /// (`defensaBaseCaravana`: no lleva escolta). Si gana el bandido se la llevan entera, como en una disputa ([`Nucleo::caer_fundacion`]). La que va adjunta a una columna cae o se
    /// salva con la emboscada de la columna ([`Nucleo::emboscar_columna`]).
    fn acosar_fundacion(&mut self, idc: u64, x: u64, t: Instante) {
        let (Some(b), Some(k)) = (self.estado.combate.bandidos.get(&idc), self.estado.fundacion.caravanas.get(&x)) else { return };
        let (nivel, origen) = (b.nivel, k.origen);
        let SituacionFundacion::Retornando(_) = k.situacion else { return };
        let donde = k.posicion_en(t);
        if !self.es_presa_de_bandidos(EstadoCaravana::Retornando, donde) {
            return;
        }
        let cr = self.estado.reglas.economia.combate.clone();
        let acoso = resolver_acoso(&cr, nivel, Defensa::Base, &mut self.estado.rng);
        let plazas = match origen {
            OrigenFundacion::Plaza(a) if self.estado.asentamientos.contains_key(&a) => vec![a],
            _ => vec![],
        };
        let datos = vec![("campamento", idc as i64), ("caravana", x as i64), ("poderBandido", acoso.poder_bandido), ("defensa", acoso.defensa), ("bajasEscolta", 0)];
        self.tr.emitir(&self.estado, EventoNucleo { codigo: Cow::Borrowed(if acoso.gana_el_bandido { "bandidos.caravana_interceptada" } else { "bandidos.caravana_escapa" }), en: t, ejercitos: vec![], asentamientos: plazas, punto: Some(donde), datos, ..Default::default() });
        if acoso.gana_el_bandido {
            self.contar(t, tecnologia::contador::Delta::uno(tecnologia::Contador::CaravanasDestruidasOCapturadas));
            self.caer_fundacion(t, x, combate::caravana::Modalidad::Disputa);
        }
    }

    /// El campamento `idc` acosa con una tirada con números a la caravana suelta `idk`, que se defiende con su escolta cedida (que sufre bajas) o con los carreteros. Si la escolta
    /// gana, el campamento cae (2026-11-06, pedido del autor): su botín, para los héroes de la escolta, va en la caravana. Contra los carreteros solo se salva la caravana.
    fn acosar(&mut self, idc: u64, idk: u64, t: Instante) {
        let (Some(b), Some(k)) = (self.estado.combate.bandidos.get(&idc), self.estado.comercio.caravana(idk)) else { return };
        let (nivel, origen, estado) = (b.nivel, k.origen as u64, k.estado);
        let donde = self.posicion_de_caravana(idk, t).unwrap_or(k.posicion);
        if !self.es_presa_de_bandidos(estado, donde) {
            return;
        }
        let cr = self.estado.reglas.economia.combate.clone();
        let mut escolta: Vec<Escuadra> = k.escolta.iter().filter_map(|c| self.escuadra(c.id).map(|(_, s)| s.clone())).collect();
        // La participación (Participacion §2): solo da la defensa a los héroes de la escolta cedida.
        let defensores = self.heroes_de_escolta(&escolta);
        let escoltada = escolta.iter().any(Escuadra::con_soldados);
        let defensa = if escoltada { Defensa::Escolta(&mut escolta) } else { Defensa::Base };
        let acoso = resolver_acoso(&cr, nivel, defensa, &mut self.estado.rng);
        self.experiencia_de_bando(t, &mut escolta, &acoso.bajas_escolta, &[], !acoso.gana_el_bandido);
        self.participacion_de_batalla(t, &[origen], &defensores, &[]);
        self.tras_batalla(t, &Desenlace { atacados: &[Objetivo::Caravana(idk)], atacantes: &[], defensores: &defensores, gana_atacante: acoso.gana_el_bandido, nivel_bandidos: None });
        self.volcar_escuadras(&escolta);
        let plazas = vec![origen].into_iter().filter(|o| self.estado.asentamientos.contains_key(o)).collect();
        let datos = vec![("campamento", idc as i64), ("caravana", idk as i64), ("poderBandido", acoso.poder_bandido), ("defensa", acoso.defensa), ("bajasEscolta", acoso.bajas_escolta.iter().map(|b| b.antes - b.despues).sum())];
        self.tr.emitir(&self.estado, EventoNucleo { codigo: Cow::Borrowed(if acoso.gana_el_bandido { "bandidos.caravana_interceptada" } else { "bandidos.caravana_escapa" }), en: t, ejercitos: vec![], asentamientos: plazas, punto: Some(donde), datos, ..Default::default() });
        if acoso.gana_el_bandido {
            self.contar(t, tecnologia::contador::Delta::uno(tecnologia::Contador::CaravanasDestruidasOCapturadas));
            // La caravana desaparece con su carga; su escolta (a 0) vuelve al campamento de su héroe al perderse.
            self.perder_caravana(t, idk);
        } else if escoltada {
            let faccion = self.estado.asentamientos.get(&origen).map(|a| a.faccion);
            self.bandidos_vencidos(t, idc, &defensores, Botin::Caravana(idk), faccion, donde);
        }
    }

    // ───────────────────────────── La orden de administración ─────────────────────────────

    /// `colocarBandidos` (escenarios y pruebas): un campamento de ese nivel, fuera de toda zona y de la protección de un campamento de mercenarios, donde no
    /// aparecen bandidos de verdad.
    pub(super) fn colocar_bandidos(&mut self, t: Instante, posicion: Punto, nivel: u8) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        if !(1..=3).contains(&nivel) {
            return Err(Respuesta::rechazo(rechazos::BANDIDOS_NIVEL_INVALIDO, "el nivel de un campamento de bandidos va de 1 a 3", v));
        }
        if self.zona_en(posicion, None) != Zona::Ninguna || self.en_proteccion_de_campamento(posicion) {
            return Err(Respuesta::rechazo(rechazos::BANDIDOS_SOBRE_UNA_ZONA, "los bandidos no actúan dentro de una zona de influencia ni junto a un campamento de mercenarios", v));
        }
        let id = self.poner_bandidos(posicion, nivel, OrigenBandidos::Admin, None, t);
        self.aparecio(id, t, vec![]);
        Ok(vec![id])
    }
}
