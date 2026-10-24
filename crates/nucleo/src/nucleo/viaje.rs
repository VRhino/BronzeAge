//! El viaje de las caravanas comerciales dentro del núcleo (Comercio_Caravanas §4.3, §6, §7, §10): el trazado de la ruta sobre el mapa real,
//! el plan con el coste del terreno y la red de caminos, los peajes, la llegada, el regreso, el reparto automático a los trueques y la red
//! de caminos con su poda. Las reglas puras viven en el crate `comercio`; el A\* es de `crates/rutas`.
//!
//! # Una caravana es un plan
//! Al salir se calcula su plan de una vez (vértices con su instante de paso). **Su generación** es la de su entidad en la tabla de la cola
//! (`entidad_caravana`): cada plan nuevo la sube y los eventos programados con el plan anterior (llegada, peaje, fin de preparación) se
//! descartan solos; el mismo número viaja en `Plan.generacion`, que es lo que ve la rejilla de encuentros. Eventos de prioridad 6
//! ([`SucesoComercio`]): la llegada al último vértice, un peaje por cada ciudad ajena que cruza (el siguiente se programa al cobrar el
//! anterior) y el pulso del reparto.
//!
//! # El trazado
//! `trazarRutaComercial` (§7.2): un A\* de `origen` a `destino` con la atracción de la red; si el trazo pisa la zona de una plaza de otra
//! Facción (que no sea ni el origen ni el destino) la ruta pasa antes por esa ciudad —un A\* hasta ella y otro desde ella— y deja un peaje
//! pendiente. El plan reparte el tiempo por el coste de cada tramo y lo divide a la mitad sobre las aristas de la red de caminos.
//! Los A\* se memorizan por `(extremos, versión de la red)`: la memoria no es estado (se rehace al recuperar) y su contenido es función pura
//! del estado, así que no afecta al hash. Un par de plazas sin ruta por tierra se recuerda aparte (el terreno no cambia).
//!
//! # La red de caminos
//! Cada lanzamiento registra su ruta (aunque luego se cancele). La poda es **perezosa**: al arrancar una ruta se quitan las que llevan más de
//! `caducidadMinutos` sin lanzarse; las consultas ven la red ya podada sin tocarla. Las aristas que entran por primera vez en la red hacen
//! que las caravanas en viaje por ellas rehagan su plan desde donde están (el tramo restante, más rápido); la caravana que acaba de
//! registrar esa ruta no se rehace (su primer viaje la construye: «más rápido la segunda vez»).
//!
//! # Sin coste por plaza ociosa
//! Nada recurre por plaza: el reparto se reevalúa al aceptar un trueque y al quedar libre una caravana, y solo si queda un lado sin cargar
//! (hay caravana libre pero falta mercancía) se programa un pulso de un minuto en esa plaza, que se detiene cuando ya no hay nada pendiente.

use super::*;
use crate::estado::SucesoComercio;
use crate::estado_comercio::entidad_caravana;
use crate::nucleo::flota::rechazo_de_caravana;
use crate::nucleo::mercado::id32;
use asentamiento::datos::UNO;
use base::{Vertice, MS_POR_MINUTO};
use comercio::caravana::{candidatas_de_reparto, Caravana, EstadoCaravana, Llegada, PerdidaCarros, TipoCaravana};
use comercio::evento::Evento as EventoComercio;
use comercio::red_caminos::{Arista, RedCaminos};
use comercio::reparto::{asignar_origen, lados_pendientes, EntornoReparto, LadoPendiente};
use comercio::viaje::{longitud_ruta, peajes_de_paradas, CobroPeaje};
use comercio::{total, Lado};
use rutas::{a_plan_por_minuto, clave_arista, Atraccion, Camino};
use std::collections::{BTreeSet, HashMap, HashSet};
use territorio::Disco;

/// Rutas memorizadas como máximo; al pasarse se vacía la memoria entera.
const MAX_CAMINOS_EN_MEMORIA: usize = 4096;

/// Una ruta comercial trazada (§7.2): la polilínea, el coste de cada tramo y las ciudades ajenas por las que pasa con la distancia
/// recorrida hasta cada una.
#[derive(Clone, Debug)]
pub(super) struct Trazado {
    pub puntos: Vec<Punto>,
    pub costes: Vec<i64>,
    pub paradas: Vec<(u32, Mil)>,
}

/// Memoria del viaje (no es estado): rutas ya calculadas y la atracción de la red mientras sigue valiendo.
#[derive(Default)]
pub(super) struct CacheViaje {
    /// Sube cada vez que cambian las rutas de la red (una ruta nueva, la poda): invalida la atracción memorizada.
    version_red: u64,
    /// Sube cada vez que se recalcula la atracción (cambió la red o caducó una ruta con el tiempo): invalida las rutas memorizadas con la anterior.
    epoca: u64,
    atraccion: Option<AtraccionMemo>,
    /// `(origen, destino, con atracción, época)` → ruta.
    caminos: HashMap<(Punto, Punto, bool, u64), Option<Camino>>,
    /// Pares de plazas sin ruta por tierra.
    inalcanzables: HashSet<(u64, u64)>,
}

/// La atracción de la red calculada para una versión, vigente hasta que caduca la primera de las rutas que la forman.
struct AtraccionMemo {
    version: u64,
    valida_hasta: Instante,
    atraccion: Option<Arc<Atraccion>>,
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Dónde está una caravana en `t`: sobre su plan si viaja, con la columna si va adjunta, y donde quedó en los demás casos.
    pub fn posicion_de_caravana(&self, id: u64, t: Instante) -> Option<Punto> {
        let c = self.estado.comercio.caravana(id)?;
        Some(match c.estado {
            EstadoCaravana::Adjunta => self.estado.ejercitos.values().find(|e| e.caravanas_adjuntas.contains(&id)).map_or(c.posicion, |e| e.posicion_en(t)),
            EstadoCaravana::EnBatalla => c.posicion,
            _ => c.posicion_en(t),
        })
    }

    /// La red de caminos tal como está guardada (puede conservar rutas caducadas hasta el próximo lanzamiento; véase [`Nucleo::pesos_de_red`]).
    pub fn red_de_caminos(&self) -> &RedCaminos {
        &self.estado.comercio.red
    }

    /// La red tal como la verían las rutas de ahora: sin las caducadas (poda perezosa, sin tocar el estado).
    fn red_vigente(&self) -> RedCaminos {
        let mut red = self.estado.comercio.red.clone();
        let asentamientos = &self.estado.asentamientos;
        red.podar(&self.estado.reglas.economia.comercio.red, self.estado.ahora, &|id| asentamientos.contains_key(&(id as u64)));
        red
    }

    /// Los pesos vigentes de la red (cuántas rutas pisan cada arista), sin las rutas caducadas.
    pub fn pesos_de_red(&self) -> std::collections::BTreeMap<Arista, u32> {
        comercio::red_caminos::pesos(self.rutas_vigentes())
    }

    /// Las rutas comerciales vigentes (sin las caducadas ni las de una plaza que ya no existe), sin tocar el estado.
    pub fn rutas_vigentes(&self) -> impl Iterator<Item = &comercio::red_caminos::RutaComercial> {
        let (r, ahora, plazas) = (&self.estado.reglas.economia.comercio.red, self.estado.ahora, &self.estado.asentamientos);
        self.estado.comercio.red.rutas().filter(move |x| x.vigente(r, ahora, &|id| plazas.contains_key(&u64::from(id))))
    }

    /// `caminoCompartidoAbierto` (§7.4): ¿hay una arista fuera de toda zona que pisen ≥ 10 rutas de ≥ 3 Facciones? (la consulta del logro
    /// `logistica_campana`, Era IV, aún sin quien la llame).
    pub fn camino_compartido_abierto(&self) -> bool {
        let red = self.red_vigente();
        let discos: Vec<Disco> = self.discos().into_iter().map(|(_, d)| d).collect();
        red.camino_compartido_abierto(&self.estado.reglas.economia.comercio.red, &|p| discos.iter().any(|d| territorio::en_zona(p, d, discos.iter())))
    }

    /// Capacidad de carga de una columna (Movimiento_Ejercitos §9): el carro de cada uno de sus participantes más la capacidad efectiva («Carga
    /// Ampliada» del origen) de las caravanas adjuntas.
    pub fn capacidad_de_carga(&self, ejercito: u64) -> Option<Mil> {
        let e = self.estado.ejercitos.get(&ejercito)?;
        let r = &self.estado.reglas.economia.comercio.flota;
        let adjuntas: Mil = e
            .caravanas_adjuntas
            .iter()
            .filter_map(|&c| self.estado.comercio.caravana(c))
            .map(|c| c.capacidad_efectiva(r, self.estado.asentamientos.get(&(c.origen as u64)).map_or(UNO, |a| self.factores_de(a).capacidad)))
            .sum();
        Some(e.columna.capacidad_de_carga(&self.estado.reglas.economia.militar, adjuntas))
    }

    // ───────────────────────────── Red de caminos y rutas ─────────────────────────────

    /// Poda perezosa de la red (§7.4): al registrar una ruta se quitan las que llevan más de `caducidadMinutos` sin lanzarse. Solo se llama
    /// en una orden o un evento aceptados (una orden rechazada no deja rastro): las rutas para el trazado leen la red ya podada sin tocarla.
    fn podar_red(&mut self, t: Instante) {
        let Estado { comercio, asentamientos, reglas, .. } = &mut self.estado;
        let existe = |id: u32| asentamientos.contains_key(&(id as u64));
        if comercio.red.podar(&reglas.economia.comercio.red, t, &existe) {
            self.cache.version_red += 1;
        }
    }

    /// La atracción de la red para las rutas que se calculen en `t`: la de las rutas vigentes (las caducadas no cuentan aunque aún no se hayan
    /// podado). `None` si no queda ninguna. Memorizada hasta que caduque la primera ruta o cambie la red; no toca el estado.
    fn atraccion_de_red(&mut self, t: Instante) -> Option<Arc<Atraccion>> {
        if let Some(m) = &self.cache.atraccion {
            if m.version == self.cache.version_red && t < m.valida_hasta {
                return m.atraccion.clone();
            }
        }
        let (nav, caducidad) = (&self.estado.reglas.economia.comercio.navegacion, self.estado.reglas.economia.comercio.red.caducidad);
        let asentamientos = &self.estado.asentamientos;
        let mut pesos: HashMap<rutas::ClaveArista, u32> = HashMap::new();
        let mut valida_hasta = Instante::MAX;
        for r in self.estado.comercio.red.rutas() {
            let vence = r.ultimo_lanzamiento + caducidad;
            if t >= vence || !asentamientos.contains_key(&(r.origen as u64)) || !asentamientos.contains_key(&(r.destino as u64)) {
                continue;
            }
            valida_hasta = valida_hasta.min(vence);
            for a in &r.aristas {
                *pesos.entry(*a).or_insert(0) += 1;
            }
        }
        let atraccion = (!pesos.is_empty()).then(|| Arc::new(Atraccion { pesos, por_peso: nav.atraccion_por_peso, minimo: nav.atraccion_min }));
        self.cache.epoca += 1;
        self.cache.atraccion = Some(AtraccionMemo { version: self.cache.version_red, valida_hasta, atraccion: atraccion.clone() });
        atraccion
    }

    /// Un A\* memorizado entre dos puntos, con la atracción de la red si se da.
    fn camino(&mut self, origen: Punto, destino: Punto, atraccion: Option<&Arc<Atraccion>>) -> Option<Camino> {
        let clave = (origen, destino, atraccion.is_some(), if atraccion.is_some() { self.cache.epoca } else { 0 });
        if let Some(c) = self.cache.caminos.get(&clave) {
            return c.clone();
        }
        self.metricas.rutas_calculadas += 1;
        let opciones = Opciones { atraccion: atraccion.cloned(), ..Opciones::default() };
        let c = self.rutas.ruta_con(&mut self.escritorio, origen, destino, &opciones);
        if self.cache.caminos.len() >= MAX_CAMINOS_EN_MEMORIA {
            self.cache.caminos.clear();
        }
        self.cache.caminos.insert(clave, c.clone());
        c
    }

    /// Un A\* sin atracción de la red (el regreso de una aparcada con carga y el de una caravana cuya plaza cae).
    pub(super) fn camino_sin_red(&mut self, origen: Punto, destino: Punto) -> Option<Camino> {
        self.camino(origen, destino, None)
    }

    /// `trazarRutaComercial` (§7.2) entre dos plazas. `None` si no hay ruta por tierra.
    pub(super) fn trazar_ruta_comercial(&mut self, origen: u64, destino: u64, t: Instante) -> Option<Trazado> {
        let (o, d) = (self.estado.asentamientos.get(&origen)?, self.estado.asentamientos.get(&destino)?);
        let (po, pd, faccion) = (o.posicion, d.posicion, o.faccion);
        self.trazar_ruta(origen, destino, po, pd, faccion, t)
    }

    /// El trazado de §7.2 entre dos puntos de la Facción `faccion`. `origen` y `destino` son las claves de lo que une la ruta (dos plazas, o un
    /// pueblo extractor y su madre: el carro de acarreo): ni una ni otra cuentan como zona ajena por la que pagar peaje, y el par sin ruta por
    /// tierra se recuerda con ellas.
    pub(super) fn trazar_ruta(&mut self, origen: u64, destino: u64, po: Punto, pd: Punto, faccion: u32, t: Instante) -> Option<Trazado> {
        if self.cache.inalcanzables.contains(&(origen, destino)) {
            return None;
        }
        let atraccion = self.atraccion_de_red(t);
        // Las zonas de las plazas de otra Facción que ni salen ni llegan: por ellas hay que pasar (peaje).
        let todos: Vec<(u64, Disco)> = self.discos();
        let mut pendientes: Vec<(u64, Disco)> = todos.iter().copied().filter(|&(id, d)| id != origen && id != destino && d.faccion != faccion).collect();
        let discos: Vec<Disco> = todos.iter().map(|&(_, d)| d).collect();
        let (mut puntos, mut costes, mut paradas, mut actual) = (vec![po], Vec::<i64>::new(), Vec::<(u32, Mil)>::new(), po);
        let extender = |puntos: &mut Vec<Punto>, costes: &mut Vec<i64>, c: &Camino| {
            puntos.extend_from_slice(&c.puntos[1..]);
            costes.extend_from_slice(&c.costes);
        };
        let mut primero = true;
        loop {
            let Some(tramo) = self.camino(actual, pd, atraccion.as_ref()) else {
                if primero {
                    self.cache.inalcanzables.insert((origen, destino));
                }
                return None;
            };
            primero = false;
            let zona = tramo.puntos.iter().skip(1).find_map(|&p| pendientes.iter().position(|(_, d)| territorio::en_zona(p, d, discos.iter())));
            let Some(zona) = zona else {
                extender(&mut puntos, &mut costes, &tramo);
                break;
            };
            let (ciudad, disco) = pendientes.remove(zona);
            let Some(hasta) = self.camino(actual, disco.centro, atraccion.as_ref()) else {
                extender(&mut puntos, &mut costes, &tramo);
                break;
            };
            extender(&mut puntos, &mut costes, &hasta);
            paradas.push((id32(ciudad), longitud_ruta(&puntos)));
            actual = disco.centro;
        }
        if puntos.len() < 2 {
            // Origen y destino en el mismo punto: un viaje de duración cero.
            puntos.push(pd);
            costes.push(0);
        }
        Some(Trazado { puntos, costes, paradas })
    }

    /// Los costes de un trazado con el camino más rápido: sobre una arista de la red el coste se multiplica por `factorCamino`.
    fn costes_efectivos(&self, puntos: &[Punto], costes: &[i64]) -> Vec<i64> {
        let factor = self.estado.reglas.economia.comercio.navegacion.factor_camino;
        let red = &self.estado.comercio.red;
        puntos.windows(2).zip(costes).map(|(w, &c)| if red.contiene(&clave_arista(w[0], w[1])) { c * factor / UNO } else { c }).collect()
    }

    /// El plan de una caravana sobre una polilínea con sus costes, saliendo en `salida` (`generacion` la pone [`Nucleo::instalar_plan`]).
    pub(super) fn plan_de(&self, c: &Caravana, puntos: &[Punto], costes: &[i64], salida: Instante) -> Plan {
        let origen = c.origen as u64;
        let factor = self.estado.asentamientos.get(&origen).map_or(UNO, |a| self.factores_de(a).velocidad);
        let velocidad = c.velocidad_efectiva(&self.estado.reglas.economia.comercio.flota, factor);
        a_plan_por_minuto(puntos, &self.costes_efectivos(puntos, costes), self.rutas.espaciado(), salida, velocidad, 0)
    }

    /// La red de rutas cambió por fuera de un lanzamiento (una plaza desapareció): la atracción memorizada deja de valer.
    pub(super) fn invalidar_red(&mut self) {
        self.cache.version_red += 1;
    }

    /// Registra la ruta de un lanzamiento en la red (§7.4) y rehace el plan de las caravanas en viaje por aristas que acaban de pasar a ser
    /// camino. Las `excepto` (las que acaban de salir por esa ruta) no se rehacen.
    pub(super) fn registrar_ruta_en_red(&mut self, origen: u64, destino: u64, ruta: &[Punto], t: Instante, excepto: &[u64]) {
        let Some(faccion) = self.estado.asentamientos.get(&origen).map(|a| a.faccion) else { return };
        self.podar_red(t);
        let cambio = self.estado.comercio.red.registrar_ruta(id32(origen), id32(destino), faccion, ruta, t);
        if cambio.pesos_cambiados {
            self.cache.version_red += 1;
        }
        if !cambio.nuevas.is_empty() {
            self.rehacer_planes_por_camino(&cambio.nuevas.into_iter().collect(), t, excepto);
        }
    }

    /// Las caravanas en viaje cuyo tramo restante pisa una arista recién abierta rehacen su plan desde su posición (más rápido sobre el camino).
    /// Se conservan el pasado del plan y los peajes pendientes (su avance es por distancia, que no cambia).
    fn rehacer_planes_por_camino(&mut self, nuevas: &BTreeSet<Arista>, t: Instante, excepto: &[u64]) {
        let factor = self.estado.reglas.economia.comercio.navegacion.factor_camino;
        let rehacer: Vec<(u64, Plan)> = self
            .estado
            .comercio
            .flota
            .iter()
            .filter(|c| matches!(c.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando) && !excepto.contains(&c.id))
            .filter_map(|c| Some((c.id, replanificar_con_camino(c.plan.as_ref()?, t, nuevas, factor)?)))
            .collect();
        for (id, plan) in rehacer {
            self.instalar_plan(id, plan);
        }
    }

    // ───────────────────────────── El plan de una caravana ─────────────────────────────

    /// Pone `plan` a la caravana: sube su generación (descarta lo programado con el plan anterior), lo guarda con esa generación, programa su
    /// llegada y su próximo peaje y avisa a los generadores (la rejilla de encuentros).
    pub(super) fn instalar_plan(&mut self, id: u64, mut plan: Plan) {
        let entidad = entidad_caravana(id);
        let g = self.estado.generaciones.subir(entidad);
        plan.generacion = g;
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return };
        let c = &mut self.estado.comercio.flota[i];
        if let Some(v) = plan.vertices.first() {
            c.posicion = v.p;
        }
        c.plan = Some(plan.clone());
        self.tr.tocar(Clase::Caravana, id);
        self.programar_viaje(id);
        self.avisar_plan_entidad(entidad, Some(plan));
    }

    /// Programa la llegada y el próximo peaje de la caravana con su generación vigente.
    fn programar_viaje(&mut self, id: u64) {
        let Some(c) = self.estado.comercio.caravana(id) else { return };
        let (fin, peaje) = (c.llegada_en(), c.proximo_peaje());
        let ahora = self.estado.ahora;
        let entidad = entidad_caravana(id);
        if let Some(fin) = fin {
            self.programar_comercio(fin.max(ahora), entidad, SucesoComercio::LlegadaCaravana);
        }
        if let Some(p) = peaje {
            self.programar_comercio(p.max(ahora), entidad, SucesoComercio::PeajeCaravana);
        }
    }

    /// Cuenta al espacio y a los generadores que la entidad tiene plan nuevo (`Some`) o ya no se mueve (`None`).
    pub(super) fn avisar_plan_entidad(&mut self, entidad: u64, plan: Option<Plan>) {
        self.anunciar_caravana(entidad, plan.clone());
        if self.generadores.is_empty() {
            return;
        }
        let mut gens = std::mem::take(&mut self.generadores);
        for (i, g) in gens.iter_mut().enumerate() {
            let mut cx = Cx { estado: &mut self.estado, tr: &mut self.tr, generador: i as u16 };
            match &plan {
                Some(p) => g.plan_cambiado(&mut cx, entidad, p),
                None => g.plan_retirado(&mut cx, entidad),
            }
        }
        self.generadores = gens;
    }

    /// La caravana deja de moverse (se pierde, llega a casa o se aparta): sus eventos pendientes se descartan y la rejilla la olvida.
    pub(super) fn retirar_plan(&mut self, id: u64) {
        let entidad = entidad_caravana(id);
        self.estado.generaciones.subir(entidad);
        self.avisar_plan_entidad(entidad, None);
    }

    /// Plan de regreso de `desde` a la plaza de origen por un A\* sin atracción de la red (el de una aparcada con carga y el de una caravana
    /// cuyo destino cae). `None` si no hay ruta por tierra.
    pub(super) fn plan_de_regreso(&mut self, id: u64, desde: Punto, t: Instante) -> Option<(Plan, Camino)> {
        let c = self.estado.comercio.caravana(id)?;
        let hasta = self.estado.asentamientos.get(&(c.origen as u64))?.posicion;
        let camino = self.camino_sin_red(desde, hasta)?;
        let c = self.estado.comercio.caravana(id)?;
        let plan = self.plan_de(c, &camino.puntos, &camino.costes, t);
        Some((plan, camino))
    }

    // ───────────────────────────── Salir ─────────────────────────────

    /// El fin del tiempo de preparación (§5.6): la caravana sale con su plan, calculado desde este instante sobre la ruta que guardó.
    pub(super) fn salida_de_preparacion(&mut self, id: u64, t: Instante) {
        let Some(c) = self.estado.comercio.caravana(id) else { return };
        if c.estado != EstadoCaravana::Preparando || c.prepara_hasta.is_none_or(|h| h > t) {
            return;
        }
        let (origen, destino) = (c.origen as u64, c.destino.map(u64::from));
        let plan = self.plan_de(c, &c.ruta, &c.costes, t);
        let i = self.estado.comercio.posicion_caravana(id).expect("existe");
        if self.estado.comercio.flota[i].salir(t, plan.clone()).is_err() {
            return;
        }
        self.instalar_plan(id, plan);
        self.tr.tocar(Clase::Asentamiento, origen);
        self.emitir_comercio(t, EventoComercio::CaravanaSale { caravana: id }, [Some(origen), destino].into_iter().flatten().collect(), &[]);
    }

    // ───────────────────────────── Peajes y llegada ─────────────────────────────

    /// Cobra los peajes alcanzados (§7.3): el 2 % de lo que lleva en ese momento a cada ciudad ajena que acaba de cruzar. La ciudad que ya no
    /// existe no cobra (la caravana se queda con lo suyo).
    pub(super) fn cobrar_peajes(&mut self, id: u64, t: Instante) {
        let tasa = self.estado.reglas.economia.comercio.peaje;
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return };
        let cobros = self.estado.comercio.flota[i].cobrar_peajes(t, tasa);
        if !cobros.is_empty() {
            self.tr.tocar(Clase::Caravana, id);
        }
        for CobroPeaje { ciudad, cobrado } in cobros {
            let ciudad_id = ciudad as u64;
            let Some(plaza) = self.estado.asentamientos.get_mut(&ciudad_id) else {
                for (r, n) in cobrado {
                    *self.estado.comercio.flota[i].contenido.entry(r).or_insert(0) += n;
                }
                continue;
            };
            for (&r, &n) in &cobrado {
                plaza.economia.almacen.agregar(r, n);
            }
            self.tr.tocar(Clase::Asentamiento, ciudad_id);
            self.emitir_comercio(t, EventoComercio::PeajePaso { caravana: id, ciudad, cobrado }, vec![ciudad_id], &[]);
        }
    }

    /// El plan cruza una ciudad ajena: cobra el peaje y programa el siguiente.
    pub(super) fn peaje_de_caravana(&mut self, id: u64, t: Instante) {
        self.cobrar_peajes(id, t);
        let Some(c) = self.estado.comercio.caravana(id) else { return };
        if let Some(p) = c.proximo_peaje() {
            // Defensa: un peaje que no se pudo cobrar en su instante (redondeo del avance) se reintenta en el siguiente milisegundo.
            self.programar_comercio(p.max(t + 1), entidad_caravana(id), SucesoComercio::PeajeCaravana);
        }
    }

    /// El plan llegó a su último vértice (§6). Antes se cobran los peajes que aún queden (el último puede caer en el mismo instante).
    pub(super) fn llegada_de_caravana(&mut self, id: u64, t: Instante) {
        let Some(c) = self.estado.comercio.caravana(id) else { return };
        if c.llegada_en().is_none_or(|f| f > t) {
            return;
        }
        let acarreo = c.tipo == TipoCaravana::Acarreo;
        self.cobrar_peajes(id, t);
        let i = self.estado.comercio.posicion_caravana(id).expect("existe");
        let origen = self.estado.comercio.flota[i].origen as u64;
        let Ok(llegada) = self.estado.comercio.flota[i].llegar(t) else { return };
        self.tr.tocar(Clase::Caravana, id);
        match llegada {
            Llegada::Entrega { destino, contenido, origen_acuerdo } => {
                // El regreso: la misma ruta invertida, con una generación nueva.
                if let Some(plan) = self.estado.comercio.flota[i].plan.clone() {
                    self.instalar_plan(id, plan);
                }
                let destino = destino as u64;
                let Some(entrega) = self.liquidar_entrega(t, origen, destino, &contenido) else {
                    // Una de las dos plazas ya no existe: la caravana se pierde con su carga (la ruina la habría resuelto antes).
                    self.perder_caravana(t, id);
                    return;
                };
                if let Some((acuerdo, lado)) = origen_acuerdo {
                    self.abonar_a_trueque(t, acuerdo, lado, &contenido);
                }
                let ev = EventoComercio::CaravanaLlega { caravana: id, destino: id32(destino), comision: entrega.comision, recargo: entrega.recargo };
                self.emitir_comercio(t, ev, vec![origen, destino], &[("carga", total(&contenido)), ("perdido", total(&entrega.perdido))]);
                // El trueque pudo quedar a medias: otra caravana libre puede llevar lo que falta.
                self.reparto_de_plaza(origen, t);
            }
            Llegada::Acarreo { destino, contenido } => {
                if let Some(plan) = self.estado.comercio.flota[i].plan.clone() {
                    self.instalar_plan(id, plan); // la vuelta: la misma ruta invertida, con una generación nueva
                }
                self.acarreo_llega(t, id, destino as u64, &contenido);
            }
            Llegada::Casa { .. } if acarreo => {
                self.retirar_plan(id);
                self.acarreo_en_casa(t, origen, id);
            }
            Llegada::Casa { contenido, escolta } => {
                self.devolver_escolta(&escolta);
                self.retirar_plan(id);
                let volcado = self.estado.asentamientos.get_mut(&origen).map_or(0, |a| contenido.iter().map(|(&r, &n)| a.economia.almacen.agregar(r, n)).sum::<Mil>());
                self.tr.tocar(Clase::Asentamiento, origen);
                self.emitir_propio(t, "comercio.caravana_en_casa", vec![origen], vec![("caravana", id as i64), ("volcado", volcado), ("perdido", total(&contenido) - volcado)]);
                self.reparto_de_plaza(origen, t);
            }
            Llegada::Perdida { escolta, .. } => {
                self.devolver_escolta(&escolta);
                self.perder_caravana(t, id);
            }
        }
    }

    // ───────────────────────────── Reparto automático (§4.3) ─────────────────────────────

    /// Los lados pendientes de los trueques cuyo origen es `origen` (solo `activo` vivo, con algo por entregar, sin cargamento ya en ruta y con
    /// destino existente), con su puntuación.
    fn lados_pendientes_de(&self, origen: u64, t: Instante) -> Vec<LadoPendiente> {
        let asentamientos = &self.estado.asentamientos;
        let flota = &self.estado.comercio.flota;
        let distancia = |a: u32, b: u32| match (asentamientos.get(&(a as u64)), asentamientos.get(&(b as u64))) {
            (Some(x), Some(y)) => x.posicion.dist(y.posicion),
            _ => 0,
        };
        let existe = |id: u32| asentamientos.contains_key(&(id as u64));
        let en_ruta = |acuerdo: u64, lado: Lado| flota.iter().any(|c| c.estado == EstadoCaravana::EnTransito && c.origen_acuerdo == Some((acuerdo, lado)));
        let entorno = EntornoReparto { ahora: t, distancia: &distancia, existe: &existe, en_ruta: &en_ruta };
        lados_pendientes(&self.estado.reglas.economia.comercio.asignacion, &self.estado.comercio.acuerdos, &entorno).into_iter().filter(|p| p.origen as u64 == origen).collect()
    }

    /// Reevalúa el reparto de las caravanas libres de `origen` a los lados pendientes de sus trueques (`asignarCaravanasATrueque`, §4.3). Sale con
    /// cualquier carga positiva. Si queda un lado sin cargar y una caravana libre (falta mercancía), arma el pulso de un minuto; si no, nada.
    pub(super) fn reparto_de_plaza(&mut self, origen: u64, t: Instante) {
        let Some(plaza) = self.estado.asentamientos.get(&origen) else { return };
        let cr = self.estado.reglas.economia.comercio.clone();
        let factores = self.factores_de(plaza);
        let mut almacen = plaza.economia.almacen.clone();
        let candidatas = candidatas_de_reparto(&self.estado.comercio.flota, &cr.flota, id32(origen), factores.capacidad);
        if candidatas.is_empty() {
            return;
        }
        let pendientes = self.lados_pendientes_de(origen, t);
        if pendientes.is_empty() {
            return;
        }
        let salidas = asignar_origen(id32(origen), &pendientes, &candidatas, &mut almacen, &mut |o, d| self.trazar_ruta_comercial(o as u64, d as u64, t));
        if !salidas.is_empty() {
            self.estado.asentamientos.get_mut(&origen).expect("existe").economia.almacen = almacen;
            self.tr.tocar(Clase::Asentamiento, origen);
            let lanzadas: Vec<u64> = salidas.iter().map(|s| s.caravana).collect();
            for s in salidas {
                let (id, destino, tz) = (s.caravana, s.destino as u64, s.trazado);
                let Some(i) = self.estado.comercio.posicion_caravana(id) else { continue };
                let plan = self.plan_de(&self.estado.comercio.flota[i], &tz.puntos, &tz.costes, t);
                let peajes = peajes_de_paradas(&tz.paradas, longitud_ruta(&tz.puntos));
                let c = &mut self.estado.comercio.flota[i];
                if c.salir_por_reparto(s.destino, s.carga, s.acuerdo, s.lado, tz.puntos.clone(), peajes, plan.clone()).is_err() {
                    continue;
                }
                c.costes = tz.costes.clone();
                self.instalar_plan(id, plan);
                self.emitir_comercio(t, EventoComercio::CaravanaSale { caravana: id }, vec![origen, destino], &[("acuerdo", s.acuerdo as i64), ("lado", (s.lado == Lado::B) as i64)]);
                self.registrar_ruta_en_red(origen, destino, &tz.puntos, t, &lanzadas);
            }
        }
        // ¿Queda un lado sin cargar con una caravana libre? Entonces falta mercancía (o el almacén cambia solo): se vuelve a mirar en un minuto.
        let quedan = candidatas_de_reparto(&self.estado.comercio.flota, &cr.flota, id32(origen), factores.capacidad);
        let sigue = !quedan.is_empty() && self.lados_pendientes_de(origen, t).iter().any(|p| !self.cache.inalcanzables.contains(&(origen, p.destino as u64)));
        if sigue {
            self.armar_pulso_de_reparto(origen, t);
        }
    }

    /// Programa el pulso del reparto de la plaza dentro de un minuto, si no tiene ya uno.
    fn armar_pulso_de_reparto(&mut self, plaza: u64, t: Instante) {
        if self.estado.comercio.reparto.contains_key(&plaza) {
            return;
        }
        let en = t + MS_POR_MINUTO;
        self.estado.comercio.reparto.insert(plaza, en);
        self.programar_comercio(en, plaza, SucesoComercio::PulsoReparto);
    }

    /// Vence el pulso del reparto de una plaza: reevalúa (y se vuelve a armar solo si sigue habiendo algo sin cargar).
    pub(super) fn pulso_de_reparto(&mut self, plaza: u64, t: Instante) {
        self.estado.comercio.reparto.remove(&plaza);
        self.reparto_de_plaza(plaza, t);
    }

    // ───────────────────────────── Batalla (hook del sistema militar) ─────────────────────────────

    /// La caravana entra en una batalla activa (Reloj_Y_Eventos §6, Doc 3.10): su plan queda en pausa, deja de ser perseguible y ya no cambia
    /// mientras dura. `false` si no existe o no está en un estado que pueda entrar (en el mapa: en tránsito, retornando o adjunta). Procesa los
    /// eventos hasta `t` y cierra la versión. La llama el sistema militar; no deja entrada en el registro de peticiones.
    pub fn caravana_entra_en_batalla(&mut self, t: Instante, id: u64) -> bool {
        let t = t.max(self.estado.ahora);
        self.procesar_hasta(t);
        let entra = self.apartar_caravana(t, id);
        self.cerrar_version(t, entra);
        entra
    }

    /// La batalla acabó: la caravana vuelve a lo que hacía con su plan desplazado lo que duró la pausa (ni un metro de más ni de menos).
    pub fn caravana_sale_de_batalla(&mut self, t: Instante, id: u64) -> bool {
        let t = t.max(self.estado.ahora);
        self.procesar_hasta(t);
        let sale = self.reponer_caravana(t, id);
        self.cerrar_version(t, sale);
        sale
    }

    /// `caravana_entra_en_batalla` dentro de una transición ya abierta (no procesa eventos ni cierra la versión).
    pub(super) fn apartar_caravana(&mut self, t: Instante, id: u64) -> bool {
        let Some(posicion) = self.posicion_de_caravana(id, t) else { return false };
        let i = self.estado.comercio.posicion_caravana(id).expect("existe");
        let c = &mut self.estado.comercio.flota[i];
        if c.entrar_en_batalla().is_err() {
            return false;
        }
        c.pausada_desde = Some(t);
        c.posicion = posicion;
        let origen = c.origen as u64;
        self.tr.tocar(Clase::Caravana, id);
        self.retirar_plan(id);
        self.emitir_propio(t, "comercio.caravana_en_batalla", vec![origen], vec![("caravana", id as i64)]);
        true
    }

    /// `caravana_sale_de_batalla` dentro de una transición ya abierta.
    pub(super) fn reponer_caravana(&mut self, t: Instante, id: u64) -> bool {
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return false };
        let c = &mut self.estado.comercio.flota[i];
        let Some(desde) = c.pausada_desde else { return false };
        let (aqui, origen) = (c.posicion, c.origen as u64);
        if c.salir_de_batalla().is_err() {
            return false;
        }
        c.pausada_desde = None;
        self.tr.tocar(Clase::Caravana, id);
        let plan = match (&c.plan, c.estado) {
            (Some(p), EstadoCaravana::EnTransito | EstadoCaravana::Retornando) => Some(pausar_plan(p, desde, t, aqui)),
            _ => None,
        };
        if let Some(plan) = plan {
            self.instalar_plan(id, plan);
        }
        self.emitir_propio(t, "comercio.caravana_sale_de_batalla", vec![origen], vec![("caravana", id as i64)]);
        true
    }

    /// El sistema militar declara los carros que se pierden en una batalla (`aplicar_perdida_de_carros`, Doc 3.10): se van con su animal y la
    /// parte proporcional de la carga; sin carros, la caravana se destruye. `None` si la caravana no existe o algún índice no es válido. Si se
    /// fue el animal más lento, el plan en pausa se reajusta a la velocidad nueva ([`Nucleo::reajustar_plan_en_pausa`]).
    pub fn aplicar_perdida_de_carros(&mut self, t: Instante, id: u64, perdidos: &[usize]) -> Option<PerdidaCarros> {
        let t = t.max(self.estado.ahora);
        self.procesar_hasta(t);
        let i = self.estado.comercio.posicion_caravana(id)?;
        let flota = self.estado.reglas.economia.comercio.flota.clone();
        let (origen, antes) = (self.estado.comercio.flota[i].origen as u64, self.estado.comercio.flota[i].velocidad(&flota));
        let perdida = self.estado.comercio.flota[i].aplicar_perdida_de_carros(&flota, perdidos).ok()?;
        self.tr.tocar(Clase::Caravana, id);
        if perdida.destruida {
            self.perder_caravana(t, id);
        } else {
            self.reajustar_plan_en_pausa(id, antes);
            self.emitir_propio(t, "comercio.carros_perdidos", vec![origen], vec![("caravana", id as i64), ("carros", perdida.carros_perdidos as i64), ("carga", total(&perdida.carga_perdida))]);
        }
        self.cerrar_version(t, true);
        Some(perdida)
    }

    /// La caravana perdió carros en una batalla y su velocidad era `antes`: si se fue el animal más lento, lo que le queda al plan en pausa desde el
    /// instante de la pausa dura en proporción inversa a la velocidad nueva (decisión del autor, 2026-10-23; Comercio_Caravanas, nota de la 3B-2). Al salir de la
    /// batalla, [`pausar_plan`] lo desplaza como siempre. Sin plan en pausa o sin cambio de velocidad, no toca nada.
    pub(super) fn reajustar_plan_en_pausa(&mut self, id: u64, antes: Mil) {
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return };
        let despues = self.estado.comercio.flota[i].velocidad(&self.estado.reglas.economia.comercio.flota);
        let c = &mut self.estado.comercio.flota[i];
        let (Some(desde), Some(plan)) = (c.pausada_desde, &c.plan) else { return };
        if antes == despues || antes <= 0 || despues <= 0 {
            return;
        }
        c.plan = Some(reescalar_plan(plan, desde, c.posicion, antes, despues));
        self.tr.tocar(Clase::Caravana, id);
    }

    // ───────────────────────────── Aparcada y adjunta (Movimiento_Ejercitos §9) ─────────────────────────────

    /// Escala el plan de una columna en marcha cuando cambia su velocidad (se engancha o se suelta una caravana, cambia quién va): el tiempo que le
    /// queda al plan es inversamente proporcional a la velocidad. Sube su generación y reprograma llegada, umbrales, hambre y encuentros. Devuelve si
    /// lo hizo (si no marcha o la velocidad no cambió, no toca nada).
    pub(super) fn reescalar_marcha(&mut self, id: u64, t: Instante, antes: Mil, despues: Mil) -> bool {
        if antes == despues || antes <= 0 || despues <= 0 {
            return false;
        }
        let Some(e) = self.estado.ejercitos.get_mut(&id) else { return false };
        if !e.marchando() {
            return false;
        }
        let g = self.estado.generaciones.subir(id);
        let e = self.estado.ejercitos.get_mut(&id).expect("existe");
        let pos = e.posicion_en(t);
        let nuevos: Vec<Vertice> = e.plan.vertices.iter().map(|v| if v.en > t { Vertice { p: v.p, en: t + (v.en - t) * antes / despues } } else { *v }).collect();
        e.posicion = pos;
        e.plan = Plan { generacion: g, vertices: nuevos };
        self.tr.tocar(Clase::Ejercito, id);
        self.programar_ejercito(id, t);
        self.avisar_plan(id, true);
        self.anunciar_columna(id, Motivo::Plan);
        true
    }

    /// La administración salta la comprobación de la puerta (andamiaje de las pruebas y de la operación: coloca una caravana donde haga falta).
    ///
    /// `adjuntarCaravana` (§9): una caravana `disponible` (en su origen) o `aparcada` (en su anfitriona) se engancha a una columna de su Facción
    /// que está a la puerta de esa plaza. La columna pasa a viajar a la velocidad del más lento.
    pub(super) fn adjuntar_caravana(&mut self, t: Instante, p: &Peticion, caravana: u64, ejercito: u64) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let e = self.estado.ejercitos.get(&ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(ejercito)))?;
        let i = self.estado.comercio.posicion_caravana(caravana).ok_or_else(|| rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(caravana)))?;
        if !self.manda_columna(p, e, false) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya".into()));
        }
        if !Self::es_ejercito(e) {
            return Err(rechazo(rechazos::COLUMNA_PERSONAL_SIN_CARAVANAS, "solo un ejército lleva caravanas: quien quiere llevar una sale ya como ejército".into()));
        }
        let c = &self.estado.comercio.flota[i];
        if c.tipo != TipoCaravana::Comercial {
            return Err(rechazo(rechazos::CARAVANA_NO_ES_COMERCIAL, "solo se engancha la caravana comercial".into()));
        }
        if !matches!(c.estado, EstadoCaravana::Disponible | EstadoCaravana::Aparcada) {
            return Err(rechazo(rechazos::CARAVANA_ESTADO_INVALIDO, "solo se engancha una caravana disponible o aparcada".into()));
        }
        let lugar = if c.estado == EstadoCaravana::Aparcada { c.anfitriona.map(u64::from) } else { Some(c.origen as u64) };
        let sitio = lugar.and_then(|l| self.estado.asentamientos.get(&l)).ok_or_else(|| rechazo(rechazos::ASENTAMIENTO_DESCONOCIDO, "la plaza de la caravana ya no existe".into()))?;
        if self.estado.asentamientos.get(&(c.origen as u64)).map(|o| o.faccion) != e.columna.faccion {
            return Err(rechazo(rechazos::CARAVANA_FACCION_AJENA, "la caravana es de otra Facción".into()));
        }
        if !p.admin && e.posicion_en(t).dist(sitio.posicion) > self.radio_puerta() {
            return Err(rechazo(rechazos::CARAVANA_FUERA_DE_LA_PUERTA, "la columna no está en la puerta de la plaza donde está la caravana".into()));
        }
        let antes = self.velocidad_de_columna(e);
        self.estado.comercio.flota[i].adjuntar().map_err(|er| {
            let (c, m) = rechazo_de_caravana(er);
            rechazo(c, m.into())
        })?;
        let e = self.estado.ejercitos.get_mut(&ejercito).expect("existe");
        if let Err(k) = e.caravanas_adjuntas.binary_search(&caravana) {
            e.caravanas_adjuntas.insert(k, caravana);
        }
        e.version = v;
        let despues = self.velocidad_de_columna(&self.estado.ejercitos[&ejercito]);
        self.reescalar_marcha(ejercito, t, antes, despues);
        self.tr.tocar(Clase::Ejercito, ejercito);
        self.evento("ejercito.caravana_adjuntada", t, ejercito, None, vec![("caravana", caravana as i64)]);
        Ok(None)
    }

    /// La columna entra en una plaza de la Facción de la caravana y la deja aparcada allí (Movimiento_Ejercitos §11).
    pub(super) fn aparcar_caravana(&mut self, t: Instante, p: &Peticion, caravana: u64, plaza: u64) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let i = self.estado.comercio.posicion_caravana(caravana).ok_or_else(|| rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(caravana)))?;
        let (ejercito, e) = self
            .estado
            .ejercitos
            .iter()
            .find(|(_, e)| e.caravanas_adjuntas.contains(&caravana))
            .map(|(&id, e)| (id, e))
            .ok_or_else(|| rechazo(rechazos::CARAVANA_NO_ESTA_ADJUNTA, "la caravana no va enganchada a ninguna columna".into()))?;
        if !self.manda_columna(p, e, false) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya".into()));
        }
        let anfitriona = self.plaza_o_rechazo(plaza)?;
        let origen = self.estado.comercio.flota[i].origen as u64;
        if self.estado.asentamientos.get(&origen).map(|o| o.faccion) != Some(anfitriona.faccion) {
            return Err(rechazo(rechazos::CARAVANA_PLAZA_AJENA, "una caravana solo se aparca en una plaza de su Facción".into()));
        }
        if !p.admin && e.posicion_en(t).dist(anfitriona.posicion) > self.radio_puerta() {
            return Err(rechazo(rechazos::CARAVANA_FUERA_DE_LA_PUERTA, "la columna no está en la puerta de la plaza".into()));
        }
        let (antes, posicion) = (self.velocidad_de_columna(e), anfitriona.posicion);
        self.estado.comercio.flota[i].aparcar(id32(plaza), posicion).map_err(|er| {
            let (c, m) = rechazo_de_caravana(er);
            rechazo(c, m.into())
        })?;
        let e = self.estado.ejercitos.get_mut(&ejercito).expect("existe");
        e.caravanas_adjuntas.retain(|&c| c != caravana);
        e.version = v;
        let despues = self.velocidad_de_columna(&self.estado.ejercitos[&ejercito]);
        self.reescalar_marcha(ejercito, t, antes, despues);
        self.tr.tocar(Clase::Ejercito, ejercito);
        self.evento("ejercito.caravana_aparcada", t, ejercito, None, vec![("caravana", caravana as i64), ("plaza", plaza as i64)]);
        Ok(None)
    }

    /// Saca la caravana de la lista de caravanas adjuntas de cualquier columna (cuando desaparece estando enganchada).
    pub(super) fn soltar_de_columnas(&mut self, caravana: u64) {
        let afectadas: Vec<u64> = self.estado.ejercitos.iter().filter(|(_, e)| e.caravanas_adjuntas.contains(&caravana)).map(|(&id, _)| id).collect();
        for id in afectadas {
            let ahora = self.estado.ahora;
            let antes = self.velocidad_de_columna(&self.estado.ejercitos[&id]);
            self.estado.ejercitos.get_mut(&id).expect("existe").caravanas_adjuntas.retain(|&c| c != caravana);
            let despues = self.velocidad_de_columna(&self.estado.ejercitos[&id]);
            self.reescalar_marcha(id, ahora, antes, despues);
            self.tr.tocar(Clase::Ejercito, id);
        }
    }

}

/// Rehace un plan desde `t` si su tramo restante pisa alguna arista de `nuevas` (que acaban de pasar a ser camino): el pasado se conserva, se
/// inserta la posición de `t` y a cada tramo restante que es camino se le divide el tiempo por el `factor` de camino (ppm del coste). `None`
/// si el plan no cambia.
fn replanificar_con_camino(plan: &Plan, t: Instante, nuevas: &BTreeSet<Arista>, factor: i64) -> Option<Plan> {
    let v = &plan.vertices;
    let k = v.partition_point(|x| x.en <= t); // primer vértice por llegar
    if k == 0 || k >= v.len() {
        return None;
    }
    if !(k - 1..v.len() - 1).any(|j| nuevas.contains(&clave_arista(v[j].p, v[j + 1].p))) {
        return None;
    }
    let aqui = plan.posicion_en(t)?;
    let mut nuevo: Vec<Vertice> = v[..k].to_vec();
    if nuevo.last().is_none_or(|u| u.en < t || u.p != aqui) {
        nuevo.push(Vertice { p: aqui, en: t });
    }
    let (mut en, mut antes) = (t, t);
    for j in k..v.len() {
        let viejo = v[j].en - antes;
        let dur = if nuevas.contains(&clave_arista(v[j - 1].p, v[j].p)) { viejo * factor / UNO } else { viejo };
        en += dur;
        antes = v[j].en;
        nuevo.push(Vertice { p: v[j].p, en });
    }
    Some(Plan::nuevo(0, nuevo))
}

/// El plan con lo que le queda desde `desde` (donde está en `aqui`) recorrido a la velocidad `despues` en vez de `antes`: cada vértice posterior tarda
/// `antes / despues` veces lo que tardaba.
fn reescalar_plan(plan: &Plan, desde: Instante, aqui: Punto, antes: Mil, despues: Mil) -> Plan {
    let mut vertices: Vec<Vertice> = plan.vertices.iter().copied().filter(|v| v.en <= desde).collect();
    vertices.push(Vertice { p: aqui, en: desde });
    vertices.extend(plan.vertices.iter().filter(|v| v.en > desde).map(|v| Vertice { p: v.p, en: desde + (v.en - desde) * antes / despues }));
    vertices.dedup();
    Plan::nuevo(plan.generacion, vertices)
}

/// El plan de una caravana que estuvo apartada de `desde` a `hasta` en `aqui`: lo vivido se conserva, se queda quieta lo que duró la pausa
/// y lo que le faltaba se desplaza en esa misma cantidad.
pub(super) fn pausar_plan(plan: &Plan, desde: Instante, hasta: Instante, aqui: Punto) -> Plan {
    let pausa = (hasta - desde).max(0);
    let mut vertices: Vec<Vertice> = plan.vertices.iter().copied().filter(|v| v.en <= desde).collect();
    vertices.push(Vertice { p: aqui, en: desde });
    vertices.push(Vertice { p: aqui, en: hasta });
    vertices.extend(plan.vertices.iter().filter(|v| v.en > desde).map(|v| Vertice { p: v.p, en: v.en + pausa }));
    vertices.dedup();
    Plan::nuevo(plan.generacion, vertices)
}
