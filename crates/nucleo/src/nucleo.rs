//! El bucle de un solo hilo: cola de eventos + órdenes, en `(instante, prioridad, secuencia)`.
//!
//! # Modos
//! - **Acelerado**: [`Nucleo::avanzar_hasta`] procesa la cola sin esperar a ningún reloj; las órdenes llevan su instante
//!   ([`Nucleo::aplicar_en`]). Es lo que usan el batch, las pruebas y la demo.
//! - **En vivo**: con un [`RelojCompartido`] conectado, [`Nucleo::pulso`] avanza hasta `reloj.mundo_ahora()` y
//!   [`Nucleo::aplicar`] estampa la orden con el instante de mundo de ese momento. Es lo que hace el actor de sesiones.
//!
//! Los dos modos son la misma regla: el reloj de pared solo convierte «ahora» en un instante; no entra en el estado.
//!
//! # Persistencia (event sourcing)
//! Entran al registro, con su instante, las **peticiones aceptadas** (las rechazadas y las que no cambian nada no dejan
//! rastro), una entrada `Inicio` al crear el registro (mundo, semilla e instante: recuperar con otro mundo se niega) y, cada
//! `Ajustes::latido_ms` de mundo, un `Avance` para no perder tiempo al recuperar. Cada `Ajustes::foto_cada`
//! entradas se serializa el estado en el núcleo (rápido) y otro hilo lo comprime y lo escribe. Recuperar = última foto +
//! repetir las entradas posteriores: `hash_estado` sale igual. Un fallo al escribir el registro **aborta** (panic): seguir
//! con un estado que no se puede reproducir sería peor que parar.
//!
//! # Rutas
//! `marchar_a` calcula la ruta en línea (función pura del grafo, el origen y el destino: la repetición del registro la
//! reconstruye idéntica, por eso no se escribe). [`Nucleo::marchas_en_lote`] calcula varias en paralelo con `rayon` y las
//! aplica después en orden: mismo resultado, menos tiempo.

use crate::comando::{rechazos, AccionReloj, Comando, Peticion, Resultado, Respuesta};
use crate::estado::{Carga, CambioVersion, Ejercito, Estado, EventoNucleo, Reglas, PRIO_LLEGADA, PRIO_UMBRAL};
use mundo::Mundo;
use crate::generadores::{Cx, GeneradorDeEventos, Transicion};
use base::{Instante, Mil, Plan, Punto, Ritmo};
use cola::Evento;
use comercio::caravana::EstadoCaravana;
use persistencia::{Almacen, Foto, Fotos};
use rutas::{Camino, Escritorio, Opciones, Peticion as PeticionRuta, Rutas};
use servidor::RelojCompartido;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;

mod construccion;
mod flota;
mod heroes;
mod mercado;
mod mercenarios;
mod pueblos;
mod viaje;
mod zonas;

pub use zonas::InformeViabilidad;

/// Tipos de entrada del registro.
const ENTRADA_PETICION: u16 = 1;
const ENTRADA_AVANCE: u16 = 2;
/// Primera entrada de un registro nuevo: con qué mundo, semilla e instante arrancó (se comprueba al recuperar).
const ENTRADA_INICIO: u16 = 3;

/// Ajustes de ejecución: no son reglas de juego, no entran en el hash.
#[derive(Clone, Debug)]
pub struct Ajustes {
    /// Versiones de mundo cuyo registro de cambios se conserva en memoria (alimenta deltas y reanudación).
    pub retencion_cambios: usize,
    /// Cada cuántos ms de mundo se llama a `GeneradorDeEventos::muestrear` (0 = nunca).
    pub muestreo_ms: i64,
    /// Cada cuántos ms de mundo sin entradas se escribe un `Avance` en el registro (0 = en cada avance).
    pub latido_ms: i64,
    /// Entradas del registro entre fotos (0 = sin fotos automáticas).
    pub foto_cada: u64,
}

impl Default for Ajustes {
    fn default() -> Self {
        Self { retencion_cambios: 65_536, muestreo_ms: 0, latido_ms: 10_000, foto_cada: 10_000 }
    }
}

/// Todo lo necesario para arrancar un mundo.
pub struct Arranque {
    pub reglas: Reglas,
    pub ajustes: Ajustes,
    /// Semilla del generador aleatorio de la partida.
    pub semilla: u64,
    /// Instante de mundo inicial (se ignora al recuperar de una foto).
    pub inicio: Instante,
    pub rutas: Arc<Rutas>,
    /// Hash del artefacto de mundo (`InfoMundo.hash`).
    pub mundo_hash: String,
    /// El artefacto cargado: sin él no hay fundación ni parcelas (el núcleo genérico de las pruebas de movimiento no lo usa).
    pub mundo: Option<Arc<Mundo>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metricas {
    /// Eventos vigentes procesados.
    pub eventos: u64,
    pub ordenes_aceptadas: u64,
    pub ordenes_rechazadas: u64,
    pub ordenes_sin_cambios: u64,
    pub ordenes_duplicadas: u64,
    pub rutas_calculadas: u64,
}

enum MsgFoto {
    Foto { secuencia: u64, instante: Instante, datos: Vec<u8> },
    Esperar(mpsc::Sender<()>),
}

struct Persistencia {
    almacen: Almacen,
    fotos: mpsc::Sender<MsgFoto>,
    hilo: Option<JoinHandle<()>>,
    pendiente: Vec<(Instante, u16, Vec<u8>)>,
    desde_foto: u64,
    ultimo_instante_log: Instante,
}

fn hilo_de_fotos(fotos: Fotos, rx: mpsc::Receiver<MsgFoto>) {
    for m in rx {
        match m {
            MsgFoto::Foto { secuencia, instante, datos } => {
                if let Err(e) = fotos.guardar(&Foto::nueva(secuencia, instante, datos)) {
                    eprintln!("nucleo: no se pudo escribir la foto {secuencia}: {e}");
                }
            }
            MsgFoto::Esperar(r) => {
                let _ = r.send(());
            }
        }
    }
}

fn invalido(e: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

pub struct Nucleo {
    estado: Estado,
    ajustes: Ajustes,
    rutas: Arc<Rutas>,
    escritorio: Escritorio,
    mundo_hash: String,
    semilla: u64,
    generadores: Vec<Box<dyn GeneradorDeEventos>>,
    geografia: Option<zonas::Geografia>,
    reloj: Option<RelojCompartido>,
    tr: Transicion,
    /// Un registro por versión, contiguos: el de la versión `v` está en la posición `v - base_registro - 1`.
    registro: VecDeque<CambioVersion>,
    base_registro: u64,
    persist: Option<Persistencia>,
    proxima_muestra: Option<Instante>,
    metricas: Metricas,
    avisos: Vec<persistencia::Aviso>,
    recuperado: bool,
    /// Memos del viaje de las caravanas (rutas y atracción de la red): funciones puras del estado, no entran en el hash ni en las fotos.
    cache: viaje::CacheViaje,
}

impl Nucleo {
    /// Mundo nuevo, solo en memoria.
    pub fn nuevo(a: Arranque) -> Nucleo {
        let estado = Estado::nuevo(a.reglas.clone(), a.mundo_hash.clone(), a.semilla, a.inicio);
        Self::construir(a, estado)
    }

    /// Abre (o crea) el almacén en `dir`, recupera el mundo (última foto + registro) y sigue escribiendo en él.
    /// Si el directorio está vacío arranca un mundo nuevo. Falla si el registro no se puede reproducir.
    pub fn con_persistencia(a: Arranque, dir: &Path, config: persistencia::Config) -> io::Result<Nucleo> {
        let almacen = Almacen::abrir(dir, config)?;
        let rec = almacen.recuperar()?;
        let foto_instante = rec.foto.as_ref().map(|f| f.instante);
        let estado = match &rec.foto {
            Some(f) => Estado::deserializar(&f.datos, &a.reglas, &a.mundo_hash).map_err(invalido)?,
            None => Estado::nuevo(a.reglas.clone(), a.mundo_hash.clone(), a.semilla, a.inicio),
        };
        let recuperado = rec.foto.is_some();
        let mut n = Self::construir(a, estado);
        n.avisos = rec.avisos;
        let (mut repetidas, mut ultimo) = (0u64, foto_instante.unwrap_or(n.estado.ahora));
        for e in rec.entradas_posteriores {
            match e.tipo {
                ENTRADA_PETICION => {
                    let p = Peticion::decodificar(&e.datos).map_err(invalido)?;
                    let r = n.aplicar_interno(e.instante, p, None);
                    if !r.aceptada() {
                        return Err(invalido(format!("la entrada {} no se reproduce: {:?}", e.secuencia, r.resultado)));
                    }
                }
                ENTRADA_AVANCE => n.procesar_hasta(e.instante),
                ENTRADA_INICIO => {
                    if e.datos != n.datos_de_inicio(e.instante) {
                        return Err(invalido("el registro es de otro mundo, otra semilla u otro instante inicial"));
                    }
                }
                t => return Err(invalido(format!("tipo de entrada desconocido: {t}"))),
            }
            repetidas += 1;
            ultimo = e.instante;
        }
        n.recuperado = recuperado || repetidas > 0;
        let nuevo = !n.recuperado;
        let (fotos, rx) = mpsc::channel();
        let escritor = almacen.fotos();
        let hilo = std::thread::Builder::new().name("nucleo-fotos".into()).spawn(move || hilo_de_fotos(escritor, rx))?;
        n.persist = Some(Persistencia { almacen, fotos, hilo: Some(hilo), pendiente: Vec::new(), desde_foto: repetidas, ultimo_instante_log: ultimo });
        if nuevo {
            n.registrar(n.estado.ahora, ENTRADA_INICIO, n.datos_de_inicio(n.estado.ahora));
            n.volcar_log();
        }
        Ok(n)
    }

    /// Contenido de la entrada de inicio: hash del mundo y semilla (la semilla vive en el azar del estado inicial).
    fn datos_de_inicio(&self, inicio: Instante) -> Vec<u8> {
        let mut w = crate::codec::Escritor::default();
        w.texto(&self.mundo_hash);
        w.u64(self.semilla);
        w.i64(inicio);
        w.0
    }

    fn construir(a: Arranque, estado: Estado) -> Nucleo {
        let base_registro = estado.version;
        let escritorio = Escritorio::nuevo(a.rutas.nodos());
        let mut n = Nucleo {
            estado,
            ajustes: a.ajustes,
            rutas: a.rutas,
            escritorio,
            mundo_hash: a.mundo_hash,
            semilla: a.semilla,
            generadores: Vec::new(),
            geografia: a.mundo.map(zonas::Geografia::de),
            reloj: None,
            tr: Transicion::default(),
            registro: VecDeque::new(),
            base_registro,
            persist: None,
            proxima_muestra: None,
            metricas: Metricas::default(),
            avisos: Vec::new(),
            recuperado: false,
            cache: viaje::CacheViaje::default(),
        };
        n.reiniciar_muestreo();
        n
    }

    fn reiniciar_muestreo(&mut self) {
        let paso = self.ajustes.muestreo_ms;
        self.proxima_muestra = (paso > 0).then(|| (self.estado.ahora.div_euclid(paso) + 1) * paso);
    }

    // ───────────────────────────── Consultas ─────────────────────────────

    pub fn estado(&self) -> &Estado {
        &self.estado
    }
    pub fn ahora(&self) -> Instante {
        self.estado.ahora
    }
    pub fn version(&self) -> u64 {
        self.estado.version
    }
    pub fn mundo_hash(&self) -> &str {
        &self.mundo_hash
    }
    pub fn metricas(&self) -> &Metricas {
        &self.metricas
    }
    /// Eventos que siguen en la cola (incluye los invalidados que aún no salieron).
    pub fn eventos_en_cola(&self) -> usize {
        self.estado.cola.len()
    }
    /// Eventos invalidados que ya se descartaron al salir de la cola.
    pub fn eventos_descartados(&self) -> u64 {
        self.estado.cola.descartados()
    }
    /// sha256 del estado canónico: igual semilla + mismas entradas en los mismos instantes ⇒ mismo hash.
    pub fn hash_estado(&self) -> [u8; 32] {
        self.estado.hash()
    }
    /// Instante del próximo evento en la cola (puede ser de uno que saldrá descartado).
    pub fn proximo_instante(&self) -> Option<Instante> {
        self.estado.cola.proximo_instante()
    }
    /// Avisos de la recuperación (registro truncado, foto dañada...).
    pub fn avisos_de_recuperacion(&self) -> &[persistencia::Aviso] {
        &self.avisos
    }
    /// ¿Arrancó recuperando algo del almacén?
    pub fn recuperado(&self) -> bool {
        self.recuperado
    }
    pub fn rutas(&self) -> &Rutas {
        &self.rutas
    }

    /// Registro de cambios de las versiones `> desde`, en orden. `None` si `desde` es más viejo que lo retenido o mayor
    /// que la versión actual (el llamante debe mandar una foto).
    pub fn cambios_desde(&self, desde: u64) -> Option<impl Iterator<Item = &CambioVersion>> {
        if desde < self.base_registro || desde > self.estado.version {
            return None;
        }
        Some(self.registro.range((desde - self.base_registro) as usize..))
    }

    // ───────────────────────────── Montaje ─────────────────────────────

    /// Registra un generador de eventos (`encuentros`, `vision`...). Recibe `plan_vigente` de cada columna en marcha.
    pub fn agregar_generador(&mut self, mut g: Box<dyn GeneradorDeEventos>) {
        for e in self.estado.ejercitos.values().filter(|e| e.marchando()) {
            g.plan_vigente(e.id, &e.plan);
        }
        for c in &self.estado.comercio.flota {
            if let (Some(plan), EstadoCaravana::EnTransito | EstadoCaravana::Retornando) = (&c.plan, c.estado) {
                g.plan_vigente(crate::estado_comercio::entidad_caravana(c.id), plan);
            }
        }
        self.generadores.push(g);
    }

    /// Conecta el reloj de mundo compartido (modo en vivo): lo deja en el instante, factor y pausa del estado.
    pub fn conectar_reloj(&mut self, reloj: RelojCompartido) {
        let (ahora, factor, pausado) = (self.estado.ahora, self.estado.factor_milesimas, self.estado.pausado);
        reloj.con(|r, pared| {
            *r = base::RelojMundo::nuevo(pared, ahora);
            r.fijar_factor(pared, factor);
            if pausado {
                r.pausar(pared);
            }
        });
        self.reloj = Some(reloj);
    }

    // ───────────────────────────── Bucle ─────────────────────────────

    /// Modo acelerado: procesa la cola hasta `t` (inclusive) sin esperar a ningún reloj.
    pub fn avanzar_hasta(&mut self, t: Instante) {
        self.procesar_hasta(t);
        let t = self.estado.ahora;
        let toca = self.persist.as_ref().is_some_and(|p| t > p.ultimo_instante_log && t - p.ultimo_instante_log >= self.ajustes.latido_ms);
        if toca {
            self.registrar(t, ENTRADA_AVANCE, Vec::new());
        }
        self.volcar_log();
    }

    /// Modo en vivo: avanza hasta el instante de mundo del reloj conectado (no hace nada sin reloj).
    pub fn pulso(&mut self) {
        if let Some(r) = &self.reloj {
            let t = r.mundo_ahora();
            if t > self.estado.ahora {
                self.avanzar_hasta(t);
            }
        }
    }

    /// Procesa eventos vigentes con `instante <= limite` y deja `ahora = limite`. Nunca retrocede.
    fn procesar_hasta(&mut self, limite: Instante) {
        if limite < self.estado.ahora {
            return;
        }
        loop {
            let tope = self.proxima_muestra.filter(|&m| m <= limite).unwrap_or(limite);
            while let Some(ev) = self.siguiente(tope) {
                self.manejar(ev);
            }
            match self.proxima_muestra.filter(|&m| m <= limite) {
                Some(m) => {
                    self.muestrear(m);
                    self.proxima_muestra = Some(m + self.ajustes.muestreo_ms);
                }
                None => break,
            }
        }
        self.estado.ahora = limite;
    }

    fn siguiente(&mut self, tope: Instante) -> Option<Evento<Carga>> {
        let Estado { cola, generaciones, .. } = &mut self.estado;
        cola.siguiente_hasta(tope, &|c| generaciones.es_vigente(c))
    }

    fn manejar(&mut self, ev: Evento<Carga>) {
        let t = ev.instante.max(self.estado.ahora);
        self.estado.ahora = t;
        self.metricas.eventos += 1;
        let id = ev.clave.entidad;
        match ev.carga {
            Carga::Llegada => self.llegada(id, t),
            Carga::Umbral(objetivo) => self.umbral(id, objetivo, t),
            Carga::Minuto => self.minuto_de_plaza(id, t),
            Carga::Comercio { suceso, aplazos } => self.comercio_vence(ev.clave, suceso, aplazos, t),
            Carga::Reputacion(cita) => self.cita_de_reputacion(id, cita, t),
            Carga::SalidaDelMundo => self.salida_del_mundo(id, t),
            Carga::Externa { generador, tipo, a, b } => {
                let mut gens = std::mem::take(&mut self.generadores);
                if let Some(g) = gens.get_mut(generador as usize) {
                    let mut cx = Cx { estado: &mut self.estado, tr: &mut self.tr, generador };
                    g.al_vencer(&mut cx, id, tipo, a, b);
                }
                self.generadores = gens;
            }
        }
        self.cerrar_version(t, false);
    }

    fn muestrear(&mut self, t: Instante) {
        self.estado.ahora = t;
        let mut gens = std::mem::take(&mut self.generadores);
        for (i, g) in gens.iter_mut().enumerate() {
            let mut cx = Cx { estado: &mut self.estado, tr: &mut self.tr, generador: i as u16 };
            g.muestrear(&mut cx);
        }
        self.generadores = gens;
        self.cerrar_version(t, false);
    }

    /// La ÚNICA función que sube la versión: si la transición cambió algo (o se fuerza), cierra un registro de cambios.
    fn cerrar_version(&mut self, t: Instante, forzar: bool) -> bool {
        if self.tr.tocados.is_empty() && self.tr.asentamientos.is_empty() && self.tr.eventos.is_empty() && !forzar {
            return false;
        }
        self.estado.version += 1;
        let v = self.estado.version;
        let mut ids = std::mem::take(&mut self.tr.tocados);
        ids.sort_unstable();
        ids.dedup();
        for id in &ids {
            if let Some(e) = self.estado.ejercitos.get_mut(id) {
                e.version = v;
            }
        }
        let mut asentamientos = std::mem::take(&mut self.tr.asentamientos);
        asentamientos.sort_unstable();
        asentamientos.dedup();
        for id in &asentamientos {
            if let Some(a) = self.estado.asentamientos.get_mut(id) {
                a.version = v;
            }
        }
        let eventos = std::mem::take(&mut self.tr.eventos);
        self.registro.push_back(CambioVersion { version: v, en: t, ejercitos: ids, asentamientos, eventos });
        if self.registro.len() > self.ajustes.retencion_cambios {
            if let Some(c) = self.registro.pop_front() {
                self.base_registro = c.version;
            }
        }
        true
    }

    // ───────────────────────────── Reglas de las columnas ─────────────────────────────

    fn consumo(&self, marchando: bool) -> Mil {
        let r = &self.estado.reglas;
        -(if marchando { r.consumo_marcha } else { r.consumo_estacionado })
    }

    fn evento(&mut self, codigo: &'static str, en: Instante, id: u64, punto: Option<Punto>, datos: Vec<(&'static str, i64)>) {
        self.tr.eventos.push(EventoNucleo { codigo: Cow::Borrowed(codigo), en, ejercitos: vec![id], asentamientos: vec![], punto, datos });
    }

    /// Programa la llegada (si hay plan) y los umbrales de víveres con la generación vigente. Llamar tras subirla.
    fn programar_ejercito(&mut self, id: u64, t: Instante) {
        let Some(e) = self.estado.ejercitos.get(&id) else { return };
        let (fin, viveres, cap) = (e.plan.fin(), e.viveres, e.capacidad_viveres);
        let clave = self.estado.generaciones.clave(id);
        if let Some(fin) = fin {
            self.estado.cola.programar(fin.max(t), PRIO_LLEGADA, clave, Carga::Llegada);
        }
        let bajo = cap * self.estado.reglas.viveres_bajos_pct as i64 / 100;
        // Solo se programa lo que aún no se cruzó; `bajo == 0` (sin aviso) no duplica el umbral de cero.
        for objetivo in [bajo, 0].into_iter().skip(usize::from(bajo <= 0)) {
            if viveres.valor_en(t) > objetivo {
                if let Some(ti) = viveres.instante_de_valor(objetivo, t) {
                    self.estado.cola.programar(ti, PRIO_UMBRAL, clave, Carga::Umbral(objetivo));
                }
            }
        }
    }

    fn avisar_plan(&mut self, id: u64, nuevo: bool) {
        if self.generadores.is_empty() {
            return;
        }
        let plan = self.estado.ejercitos[&id].plan.clone();
        let mut gens = std::mem::take(&mut self.generadores);
        for (i, g) in gens.iter_mut().enumerate() {
            let mut cx = Cx { estado: &mut self.estado, tr: &mut self.tr, generador: i as u16 };
            if nuevo {
                g.plan_cambiado(&mut cx, id, &plan);
            } else {
                g.plan_retirado(&mut cx, id);
            }
        }
        self.generadores = gens;
    }

    /// La columna deja de moverse donde está en `t`: sin plan, víveres al ritmo de acampada, generación nueva.
    fn detener(&mut self, id: u64, t: Instante) {
        let consumo = self.consumo(false);
        let Some(e) = self.estado.ejercitos.get_mut(&id) else { return };
        e.posicion = e.posicion_en(t);
        e.viveres = e.viveres.reanclado(t, consumo);
        let g = self.estado.generaciones.subir(id);
        e.plan = Plan { generacion: g, vertices: Vec::new() };
        self.tr.tocados.push(id);
        self.programar_ejercito(id, t);
        self.avisar_plan(id, false);
    }

    fn llegada(&mut self, id: u64, t: Instante) {
        let Some(e) = self.estado.ejercitos.get(&id) else { return };
        let Some(ultimo) = e.plan.vertices.last().copied() else { return };
        self.detener(id, t);
        self.evento("ejercito.llega", t, id, Some(ultimo.p), vec![]);
    }

    fn umbral(&mut self, id: u64, objetivo: Mil, t: Instante) {
        let Some(e) = self.estado.ejercitos.get(&id) else { return };
        let valor = e.viveres.valor_en(t);
        if valor > objetivo {
            return; // defensivo: el evento no corresponde al ritmo vigente
        }
        // Aquí solo se narra el hecho. Qué pasa con la moral y la deserción cuando faltan víveres (Movimiento_Ejercitos §6.3)
        // es una regla de la fase 6: la columna no se detiene por hambre.
        let punto = e.posicion_en(t);
        if objetivo <= 0 {
            self.evento("ejercito.sin_viveres", t, id, Some(punto), vec![]);
        } else {
            self.evento("ejercito.viveres_bajos", t, id, Some(punto), vec![("viveres", valor)]);
        }
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// Aplica una petición con el instante dado (modo acelerado, repetición del registro y pruebas).
    pub fn aplicar_en(&mut self, t: Instante, p: Peticion) -> Respuesta {
        let r = self.aplicar_interno(t, p, None);
        self.volcar_log();
        r
    }

    /// Aplica una petición con el instante de mundo del reloj conectado (modo en vivo).
    pub fn aplicar(&mut self, p: Peticion) -> Respuesta {
        let t = self.reloj.as_ref().map_or(self.estado.ahora, |r| r.mundo_ahora());
        self.aplicar_en(t, p)
    }

    /// Alta de una columna por la vía de administración. Devuelve su id.
    #[allow(clippy::too_many_arguments)]
    pub fn crear_ejercito(&mut self, t: Instante, lider: &str, propietario: &str, faccion: u32, posicion: Punto, velocidad: Mil, viveres: Mil, capacidad_viveres: Mil) -> Result<u64, Respuesta> {
        let comando = Comando::CrearEjercito { lider: lider.into(), propietario: propietario.into(), faccion, posicion, velocidad, viveres, capacidad_viveres };
        let r = self.aplicar_en(t, Peticion::nueva("admin", &format!("crear:{lider}"), comando).de_admin());
        match r.creadas.first() {
            Some(&id) if r.aceptada() => Ok(id),
            _ => Err(r),
        }
    }

    /// Varias peticiones en el mismo instante. Las rutas de las `MarcharA` se calculan en paralelo; el resultado es
    /// idéntico a aplicarlas una a una, en orden. Un ejército repetido en el lote se calcula en línea.
    pub fn marchas_en_lote(&mut self, t: Instante, peticiones: Vec<Peticion>) -> Vec<Respuesta> {
        let t = t.max(self.estado.ahora);
        self.procesar_hasta(t);
        let mut vistos = std::collections::HashSet::new();
        let mut candidatas: Vec<(usize, PeticionRuta)> = Vec::new();
        for (i, p) in peticiones.iter().enumerate() {
            if let Comando::MarcharA { ejercito, destino } = &p.comando {
                if let (Some(e), true) = (self.estado.ejercitos.get(ejercito), vistos.insert(*ejercito)) {
                    if self.estado.visto(&p.actor, &p.peticion_id).is_none() {
                        candidatas.push((i, PeticionRuta { origen: e.posicion_en(t), destino: *destino, opciones: Opciones::default() }));
                    }
                }
            }
        }
        let reqs: Vec<PeticionRuta> = candidatas.iter().map(|(_, r)| r.clone()).collect();
        let mut caminos: Vec<Option<Option<Camino>>> = (0..peticiones.len()).map(|_| None).collect();
        for ((i, _), c) in candidatas.iter().zip(self.rutas.rutas_en_paralelo(&reqs)) {
            caminos[*i] = Some(c);
        }
        self.metricas.rutas_calculadas += reqs.len() as u64;
        let r = peticiones.into_iter().zip(caminos).map(|(p, c)| self.aplicar_interno(t, p, c)).collect();
        self.volcar_log();
        r
    }

    fn aplicar_interno(&mut self, t: Instante, p: Peticion, precalculado: Option<Option<Camino>>) -> Respuesta {
        if let Some(v) = self.estado.visto(&p.actor, &p.peticion_id) {
            self.metricas.ordenes_duplicadas += 1;
            return Respuesta { resultado: Resultado::Duplicada, version_tras: v, creadas: vec![] };
        }
        let t = t.max(self.estado.ahora);
        self.procesar_hasta(t);
        let r = match p.version_esperada {
            Some(v) if self.estado.version > v => Respuesta::rechazo(
                rechazos::VERSION_OBSOLETA,
                format!("el mundo está en la versión {}, se esperaba {v}", self.estado.version),
                self.estado.version,
            ),
            _ => self.ejecutar(t, &p, precalculado),
        };
        match &r.resultado {
            Resultado::Aceptada => {
                self.metricas.ordenes_aceptadas += 1;
                self.estado.recordar(&p.actor, &p.peticion_id, r.version_tras);
                self.registrar(t, ENTRADA_PETICION, p.codificar());
            }
            Resultado::Rechazada { .. } => self.metricas.ordenes_rechazadas += 1,
            Resultado::SinCambios => self.metricas.ordenes_sin_cambios += 1,
            Resultado::Duplicada => unreachable!("las duplicadas salen antes"),
        }
        r
    }

    /// Valida y ejecuta. Una orden rechazada o sin cambios no deja rastro: ni versión, ni ids, ni eventos.
    fn ejecutar(&mut self, t: Instante, p: &Peticion, precalculado: Option<Option<Camino>>) -> Respuesta {
        let v = self.estado.version;
        let sin_cambios = || Respuesta { resultado: Resultado::SinCambios, version_tras: v, creadas: vec![] };
        let mut creadas = Vec::new();
        match &p.comando {
            Comando::CrearEjercito { lider, propietario, faccion, posicion, velocidad, viveres, capacidad_viveres } => {
                if !p.admin {
                    return Respuesta::rechazo(rechazos::NO_AUTORIZADO, "solo administración da de alta columnas", v);
                }
                if self.estado.ejercito_de_lider(lider).is_some() {
                    return Respuesta::rechazo(rechazos::LIDER_REPETIDO, format!("{lider} ya dirige una columna"), v);
                }
                let id = self.estado.siguiente_id;
                self.estado.siguiente_id += 1;
                let viveres = Ritmo { valor_en_t0: *viveres, t0: t, por_minuto: self.consumo(false), maximo: Some(*capacidad_viveres), minimo: Some(0) };
                self.estado.insertar(Ejercito {
                    id,
                    lider: lider.clone(),
                    propietario: propietario.clone(),
                    faccion: *faccion,
                    velocidad: *velocidad,
                    posicion: *posicion,
                    plan: Plan::default(),
                    viveres,
                    capacidad_viveres: *capacidad_viveres,
                    caravana_fundacion: false,
                    suministro: Default::default(),
                    caravanas_adjuntas: Vec::new(),
                    version: 0,
                });
                self.tr.tocados.push(id);
                self.programar_ejercito(id, t);
                creadas.push(id);
            }
            Comando::MarcharA { ejercito, destino } => {
                let Some(e) = self.estado.ejercitos.get(ejercito) else {
                    return Respuesta::rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{ejercito}"), v);
                };
                if !p.admin && e.propietario != p.actor {
                    return Respuesta::rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya", v);
                }
                let (id, origen, velocidad, marchando) = (e.id, e.posicion_en(t), self.velocidad_de_columna(e), e.marchando());
                let camino = precalculado.unwrap_or_else(|| {
                    self.metricas.rutas_calculadas += 1;
                    self.rutas.ruta_con(&mut self.escritorio, origen, *destino, &Opciones::default())
                });
                let Some(camino) = camino else {
                    return Respuesta::rechazo(rechazos::RUTA_IMPOSIBLE, "no hay ruta por tierra hasta el destino", v);
                };
                if camino.puntos.len() < 2 {
                    // Ya está en el destino: queda quieta, o no cambia nada.
                    if !marchando {
                        return sin_cambios();
                    }
                    self.detener(id, t);
                    self.evento("ejercito.estacionado", t, id, Some(origen), vec![]);
                } else {
                    let consumo = self.consumo(true);
                    let g = self.estado.generaciones.subir(id);
                    let e = self.estado.ejercitos.get_mut(&id).expect("existe");
                    e.plan = camino.a_plan(t, velocidad, g);
                    e.posicion = origen;
                    e.viveres = e.viveres.reanclado(t, consumo);
                    self.tr.tocados.push(id);
                    self.programar_ejercito(id, t);
                    self.avisar_plan(id, true);
                }
            }
            Comando::Estacionar { ejercito } => {
                let Some(e) = self.estado.ejercitos.get(ejercito) else {
                    return Respuesta::rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{ejercito}"), v);
                };
                if !p.admin && e.propietario != p.actor {
                    return Respuesta::rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya", v);
                }
                if !e.marchando() {
                    return sin_cambios();
                }
                let (id, punto) = (e.id, e.posicion_en(t));
                self.detener(id, t);
                self.evento("ejercito.estacionado", t, id, Some(punto), vec![]);
            }
            Comando::FijarNivelFaccion { .. }
            | Comando::DarCaravanaFundacion { .. }
            | Comando::Fundar { .. }
            | Comando::FijarNivelAsentamiento { .. }
            | Comando::AgrandarZona { .. }
            | Comando::ReclamarParcela { .. }
            | Comando::SoltarParcela { .. } => match self.ejecutar_territorio(t, p) {
                Ok(Some(creada)) => creadas.push(creada),
                Ok(None) => {}
                Err(r) => return r,
            },
            Comando::AnadirEdificio { .. }
            | Comando::QuitarDeCola { .. }
            | Comando::MoverEnCola { .. }
            | Comando::MejorarEdificio { .. }
            | Comando::AlternarAutoConstruccion { .. }
            | Comando::AlternarReceta { .. }
            | Comando::CalibrarReserva { .. }
            | Comando::SolicitarAscenso { .. }
            | Comando::ActivarPolitica { .. } => match self.ejecutar_economia(t, p) {
                Ok(Some(creada)) => creadas.push(creada),
                Ok(None) => {}
                Err(r) => return r,
            },
            Comando::ProponerTrueque { .. }
            | Comando::AceptarTrueque { .. }
            | Comando::RechazarTrueque { .. }
            | Comando::ColocarOrdenMercado { .. }
            | Comando::CancelarOrdenMercado { .. }
            | Comando::ComerciarEnPlaza { .. }
            | Comando::DarRecursos { .. }
            | Comando::DarSuministro { .. } => match self.ejecutar_mercado(t, p) {
                Ok(Some(creada)) => creadas.push(creada),
                Ok(None) => {}
                Err(r) => return r,
            },
            Comando::CrearCaravana { .. }
            | Comando::AnadirCarro { .. }
            | Comando::AnadirAnimal { .. }
            | Comando::MoverCarro { .. }
            | Comando::ReservarCaravana { .. }
            | Comando::PrepararCaravana { .. }
            | Comando::CancelarCaravana { .. }
            | Comando::MoverCargaAparcada { .. }
            | Comando::EnviarCaravanaAlOrigen { .. }
            | Comando::EntregarDeCaravana { .. }
            | Comando::AdjuntarCaravana { .. }
            | Comando::AparcarCaravana { .. } => match self.ejecutar_flota(t, p) {
                Ok(Some(creada)) => creadas.push(creada),
                Ok(None) => {}
                Err(r) => return r,
            },
            Comando::ComprarEnCampamento { .. }
            | Comando::RepostarEnCampamento { .. }
            | Comando::FijarResidencia { .. }
            | Comando::DarAlmacenPersonal { .. }
            | Comando::ReclutarEnCampamento { .. }
            | Comando::PedirPrestamo { .. }
            | Comando::ReponerPrestamo { .. } => match self.ejecutar_mercenarios(t, p) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::CrearHeroe { .. }
            | Comando::ReclutarTropa { .. }
            | Comando::GuardarLoadout { .. }
            | Comando::BorrarLoadout { .. }
            | Comando::RepartirPuntos { .. }
            | Comando::OrdenarEscuadras { .. }
            | Comando::AsignarGuarnicion { .. }
            | Comando::RetirarGuarnicion { .. }
            | Comando::PasarAViveres { .. }
            | Comando::AsignarEscolta { .. }
            | Comando::QuitarEscolta { .. }
            | Comando::Desconectarse { .. }
            | Comando::Conectarse { .. }
            | Comando::FijarTecnologias { .. }
            | Comando::FijarFaccionDeHeroe { .. }
            | Comando::ColocarHeroe { .. }
            | Comando::HerirHeroe { .. } => match self.ejecutar_heroes(t, p) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::MarcarPueblo { .. } | Comando::EntregarMateriales { .. } | Comando::DestruirPueblo { .. } | Comando::ReponerCarro { .. } => match self.ejecutar_pueblos(t, p) {
                Ok(Some(creada)) => creadas.push(creada),
                Ok(None) => {}
                Err(r) => return r,
            },
            Comando::Reloj(accion) => {
                if !p.admin {
                    return Respuesta::rechazo(rechazos::NO_AUTORIZADO, "solo administración controla el reloj", v);
                }
                if let Err(r) = self.controlar_reloj(*accion) {
                    return r.unwrap_or_else(sin_cambios);
                }
            }
        }
        self.cerrar_version(t, true);
        Respuesta { resultado: Resultado::Aceptada, version_tras: self.estado.version, creadas }
    }

    /// `Err(None)` = sin cambios; `Err(Some(r))` = rechazo.
    fn controlar_reloj(&mut self, accion: AccionReloj) -> Result<(), Option<Respuesta>> {
        let v = self.estado.version;
        let invalido = |m: &str| Err(Some(Respuesta::rechazo(rechazos::RELOJ_INVALIDO, m, v)));
        match accion {
            AccionReloj::Pausar if self.estado.pausado => return Err(None),
            AccionReloj::Reanudar if !self.estado.pausado => return Err(None),
            AccionReloj::FijarFactor(f) if f == 0 || f > 3_600_000 => return invalido("el factor debe estar en 1..=3.600.000"),
            AccionReloj::FijarFactor(f) if f == self.estado.factor_milesimas => return Err(None),
            AccionReloj::AvanzarHasta(h) if h <= self.estado.ahora => return invalido("no se puede avanzar hacia atrás"),
            _ => {}
        }
        match accion {
            AccionReloj::Pausar => self.estado.pausado = true,
            AccionReloj::Reanudar => self.estado.pausado = false,
            AccionReloj::FijarFactor(f) => self.estado.factor_milesimas = f,
            AccionReloj::AvanzarHasta(h) => self.procesar_hasta(h),
        }
        if let Some(reloj) = &self.reloj {
            let (ahora, factor, pausado) = (self.estado.ahora, self.estado.factor_milesimas, self.estado.pausado);
            reloj.con(|r, pared| {
                if let AccionReloj::AvanzarHasta(_) = accion {
                    *r = base::RelojMundo::nuevo(pared, ahora);
                    r.fijar_factor(pared, factor);
                    if pausado {
                        r.pausar(pared);
                    }
                }
                match accion {
                    AccionReloj::Pausar => r.pausar(pared),
                    AccionReloj::Reanudar => r.reanudar(pared),
                    AccionReloj::FijarFactor(f) => r.fijar_factor(pared, f),
                    AccionReloj::AvanzarHasta(_) => {}
                }
            });
        }
        Ok(())
    }

    // ───────────────────────────── Persistencia ─────────────────────────────

    fn registrar(&mut self, t: Instante, tipo: u16, datos: Vec<u8>) {
        if let Some(p) = &mut self.persist {
            p.pendiente.push((t, tipo, datos));
            p.ultimo_instante_log = t;
        }
    }

    /// Escribe las entradas pendientes con un solo volcado y, si toca, pide una foto.
    fn volcar_log(&mut self) {
        let Some(p) = &mut self.persist else { return };
        if p.pendiente.is_empty() {
            return;
        }
        let n = p.pendiente.len() as u64;
        p.almacen
            .registro()
            .anadir_lote(p.pendiente.iter().map(|(t, k, d)| (*t, *k, d.as_slice())))
            .expect("fallo de persistencia: no se puede escribir el registro");
        p.pendiente.clear();
        p.desde_foto += n;
        if self.ajustes.foto_cada > 0 && p.desde_foto >= self.ajustes.foto_cada {
            self.foto_ahora();
        }
    }

    /// Serializa el estado en el núcleo y deja que otro hilo lo comprima y lo escriba. Sin persistencia no hace nada.
    pub fn foto_ahora(&mut self) {
        self.volcar_log();
        let Some(p) = &mut self.persist else { return };
        let secuencia = p.almacen.registro().siguiente_secuencia() - 1;
        let datos = self.estado.serializar();
        p.desde_foto = 0;
        let _ = p.fotos.send(MsgFoto::Foto { secuencia, instante: self.estado.ahora, datos });
    }

    /// Espera a que el hilo de fotos termine las que tiene pendientes.
    pub fn esperar_fotos(&mut self) {
        if let Some(p) = &self.persist {
            let (tx, rx) = mpsc::channel();
            if p.fotos.send(MsgFoto::Esperar(tx)).is_ok() {
                let _ = rx.recv();
            }
        }
    }

    /// Parada limpia: escribe un `Avance` final, sincroniza el registro y espera a las fotos.
    pub fn apagar(&mut self) -> io::Result<()> {
        let t = self.estado.ahora;
        if self.persist.as_ref().is_some_and(|p| p.ultimo_instante_log < t) {
            self.registrar(t, ENTRADA_AVANCE, Vec::new());
        }
        self.volcar_log();
        self.esperar_fotos();
        if let Some(p) = &mut self.persist {
            p.almacen.registro().sincronizar()?;
            p.almacen.compactar()?;
        }
        Ok(())
    }
}

impl Drop for Nucleo {
    fn drop(&mut self) {
        if let Some(mut p) = self.persist.take() {
            drop(p.fotos); // cierra el canal: el hilo termina tras sus pendientes
            if let Some(h) = p.hilo.take() {
                let _ = h.join();
            }
        }
    }
}
