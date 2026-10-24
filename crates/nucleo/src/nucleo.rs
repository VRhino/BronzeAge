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

use crate::comando::{rechazos, AccionReloj, Autoriza, Comando, Peticion, Resultado, Respuesta};
use crate::estado::{Carga, CambioVersion, Clase, Ejercito, Tocados, Estado, EventoNucleo, Reglas, PRIO_LLEGADA, PRIO_UMBRAL};
use combate::ciclo::Batalla;
use crate::estado_militar::HeroeNucleo;
use mundo::Mundo;
use crate::generadores::{Cx, GeneradorDeEventos, Transicion};
use crate::proyeccion::ids;
use base::{Instante, Mil, Plan, Punto};
use cola::Evento;
use comercio::caravana::EstadoCaravana;
use persistencia::{Almacen, Foto, Fotos};
use rutas::{Camino, Escritorio, Opciones, Peticion as PeticionRuta, Rutas};
use servidor::RelojCompartido;
use std::borrow::Cow;
use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;

mod auditoria;
mod avanzadas;
mod bandidos;
mod batallas;
mod columnas;
mod combates;
mod conquistas;
mod construccion;
mod convocatorias;
mod cronica;
mod dentro;
mod espacio;
mod experiencia;
mod exploracion;
mod flota;
mod heroes;
mod inspecciones;
mod interiores;
mod mercado;
mod mercenarios;
mod murallas;
mod persecuciones;
mod pertenencias;
mod ciudadania;
mod pueblos;
mod salidas;
mod taberna;
mod uniones;
mod viaje;
mod zonas;

use espacio::{Cosa, Motivo};
pub(crate) use exploracion::cerca;
pub use batallas::TokenDeJugador;
pub use murallas::{ErrorPresupuesto, Presupuesto, RecintoEnPie};
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
    /// Comprueba en cada versión que ninguna entidad cambia sin quedar en el registro de cambios (`nucleo/auditoria.rs`). Caro: solo para pruebas.
    /// Por defecto, desactivado; en compilaciones de depuración lo activa la variable de entorno `BRONZEAGE_AUDITAR_CAMBIOS`.
    pub auditar_cambios: bool,
}

impl Default for Ajustes {
    fn default() -> Self {
        Self { retencion_cambios: 65_536, muestreo_ms: 0, latido_ms: 10_000, foto_cada: 10_000, auditar_cambios: cfg!(debug_assertions) && std::env::var_os("BRONZEAGE_AUDITAR_CAMBIOS").is_some() }
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
    /// Versión en que cambió por última vez cada entidad que no la lleva en su estado (todas salvo columnas y plazas). No se persiste: tras un reinicio una entidad
    /// no tocada desde entonces vale `version_de_arranque`, que no puede ser menor que la que un cliente conserva.
    versiones: BTreeMap<(Clase, u64), u64>,
    /// La versión de mundo con la que arrancó el proceso (la recuperada de la foto y el registro, o 0 en un mundo nuevo).
    version_de_arranque: u64,
    /// La huella de cada entidad en la última comprobación de `Ajustes::auditar_cambios`.
    huellas_auditadas: Option<auditoria::Huellas>,
    /// A qué ámbitos de jugador pertenece cada entidad que viaja a ellos (`pertenencias.rs`). Solo se lleva si hay sesiones o el núcleo es persistente (`seguir_pertenencias`); no se persiste: se rehace del estado.
    pertenencias: Option<pertenencias::Mapa>,
    /// Dónde está cada héroe y cada caravana respecto de las plazas (`interiores.rs`). Solo se lleva si hay sesiones o se audita; no se persiste.
    interiores: Option<interiores::Interiores>,
    /// Los ojos y los sitios como estaban al cerrar la última versión (`exploracion.rs`): con ellos se graba lo que vieron. No se persiste: entre versiones es lo que dice el estado. Sin rejilla de niebla, nada.
    ojos: Option<exploracion::Ojos>,
    /// Desde qué versión llevan pertenencias los registros de cambios (la de `seguir_pertenencias`; en el núcleo persistente, la de su foto): las anteriores no dicen a qué ámbito iba cada entidad y no
    /// se pueden reanudar por deltas.
    pertenencias_desde: u64,
    /// Lo último que el núcleo estaba aplicando (una orden o un evento), para decir en la auditoría qué cambió algo sin marcarlo.
    causa_auditada: String,
    persist: Option<Persistencia>,
    proxima_muestra: Option<Instante>,
    metricas: Metricas,
    avisos: Vec<persistencia::Aviso>,
    recuperado: bool,
    /// Memos del viaje de las caravanas (rutas y atracción de la red): funciones puras del estado, no entran en el hash ni en las fotos.
    cache: viaje::CacheViaje,
    /// El espacio del mundo (rejilla de encuentros y perseguidores): se reconstruye del estado, no entra en el hash ni en las fotos.
    espacio: espacio::Espacio,
    /// Las columnas que están rehaciendo su plan por una persecución en este instante (para no repetir una en la misma cascada). Vacío entre órdenes.
    siguiendo: Vec<u64>,
    /// Las batallas que salieron del estado al cumplir su retención y aún no recogió quien las persiste (`tomar_batallas_retiradas`): no entra en el hash ni
    /// en las fotos.
    retiradas: Vec<Batalla>,
}

impl Nucleo {
    /// Mundo nuevo, solo en memoria.
    pub fn nuevo(a: Arranque) -> Nucleo {
        let estado = Estado::nuevo(a.reglas.clone(), a.mundo_hash.clone(), a.semilla, a.inicio);
        let mut n = Self::construir(a, estado);
        n.sembrar_el_mundo();
        n.rebasar_auditoria();
        n
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
        if !recuperado {
            n.sembrar_el_mundo();
        }
        n.rebasar_auditoria();
        n.avisos = rec.avisos;
        // El registro que se repite lleva las pertenencias, como lo llevaba el proceso vivo (son una función pura del estado): un cliente que conserva una versión desde la foto reanuda por
        // deltas tras el reinicio (4H T6). El mundo persistente es el del servidor, que siempre tiene sesiones.
        n.seguir_pertenencias();
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
            versiones: BTreeMap::new(),
            version_de_arranque: base_registro,
            huellas_auditadas: None,
            pertenencias: None,
            interiores: None,
            ojos: None,
            pertenencias_desde: 0,
            causa_auditada: String::new(),
            persist: None,
            proxima_muestra: None,
            metricas: Metricas::default(),
            avisos: Vec::new(),
            recuperado: false,
            cache: viaje::CacheViaje::default(),
            espacio: espacio::Espacio::nuevo(),
            siguiendo: Vec::new(),
            retiradas: Vec::new(),
        };
        n.reconstruir_espacio();
        n.reiniciar_muestreo();
        // La rejilla de lo explorado sale de los límites del mapa (`EXPLORACION.tamanoCelda`); sin artefacto de mundo no se graba nada.
        if let (None, Some(g)) = (n.estado.memoria.rejilla, &n.geografia) {
            let t = n.estado.reglas.economia.militar.vision.tamano_celda;
            let (ancho, alto) = g.tamano();
            let (columnas, filas) = vision::rejilla(ancho, alto, t);
            n.estado.memoria.rejilla = Some((columnas, filas, t));
        }
        n.rehacer_ojos();
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
        if self.ajustes.auditar_cambios {
            self.causa_auditada = format!("el evento {:?} de la entidad {id}", ev.carga);
        }
        match ev.carga {
            Carga::Llegada => self.llegada(id, t),
            Carga::Umbral(objetivo) => self.umbral(id, objetivo, t),
            Carga::Minuto => self.minuto_de_plaza(id, t),
            Carga::FinDeObras => self.fines_de_plaza(id, t),
            Carga::CaducaPolitica => self.caduca_politica(id, t),
            Carga::Comercio { suceso, aplazos } => self.comercio_vence(ev.clave, suceso, aplazos, t),
            Carga::Reputacion(cita) => self.cita_de_reputacion(id, cita, t),
            Carga::SalidaDelMundo => self.salida_del_mundo(id, t),
            Carga::Militar(suceso) => self.suceso_militar(id, suceso, t),
            Carga::Combate(suceso) => self.suceso_de_combate(id, suceso, t),
            Carga::Batalla(suceso) => self.suceso_de_batalla(crate::estado_comercio::id_de_entidad(id), suceso, t),
            Carga::Encuentro { otra, otra_generacion } => self.encuentro_vence(id, otra, otra_generacion, t),
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
        // Quien entra, sale, se muda o aparca marca el interior de su plaza (solo con sesiones).
        self.marcar_interiores();
        // Las escuadras de un héroe tocado viajan con él.
        let escuadras: Vec<u64> = self.tr.tocados.de(Clase::Heroe).iter().filter_map(|h| self.estado.militar.heroes.get(h)).flat_map(|h| h.heroe.escuadras.iter().map(|e| e.id)).collect();
        self.tr.tocar_todos(Clase::Escuadra, escuadras);
        self.auditar_cambios();
        if self.tr.tocados.esta_vacio() && self.tr.eventos.is_empty() && !forzar {
            return false;
        }
        // Lo que vieron los ojos que la transición tocó queda en la memoria de su Facción.
        self.explorar(t);
        self.estado.version += 1;
        let v = self.estado.version;
        let mut tocados = std::mem::take(&mut self.tr.tocados);
        tocados.cerrar();
        let pertenencias = self.cerrar_pertenencias(&tocados);
        for id in tocados.de(Clase::Ejercito) {
            if let Some(e) = self.estado.ejercitos.get_mut(id) {
                e.version = v;
            }
        }
        for id in tocados.de(Clase::Asentamiento) {
            if let Some(a) = self.estado.asentamientos.get_mut(id) {
                a.version = v;
            }
        }
        for clase in Clase::TODAS.into_iter().filter(|c| !matches!(c, Clase::Ejercito | Clase::Asentamiento)) {
            self.versiones.extend(tocados.de(clase).iter().map(|&id| ((clase, id), v)));
        }
        let mut eventos = std::mem::take(&mut self.tr.eventos);
        self.tr.arruinadas.clear();
        // La crónica es una vista de los hechos de la versión (Tecnologia_Aedas §8): sus entradas públicas van con ellos.
        let cronica = self.cronica(&eventos);
        eventos.extend(cronica);
        self.registro.push_back(CambioVersion { version: v, en: t, tocados, pertenencias, eventos });
        if self.registro.len() > self.ajustes.retencion_cambios {
            if let Some(c) = self.registro.pop_front() {
                self.base_registro = c.version;
            }
        }
        self.rebasar_auditoria();
        true
    }

    /// La versión de mundo en que cambió por última vez una entidad (la de su estado, si lo lleva; si no, la del registro de cambios). Una entidad que no ha cambiado desde
    /// que arrancó el proceso vale la versión de arranque: nunca es menor que la que un cliente conserva de antes.
    pub fn version_de(&self, clase: Clase, id: u64) -> u64 {
        match clase {
            Clase::Ejercito => self.estado.ejercitos.get(&id).map(|e| e.version),
            // La ficha de la plaza y su interior (lo que cambia con el paso de minuto) son una sola entidad en el contrato: vale lo último que cambió de las dos.
            Clase::Asentamiento => self.estado.asentamientos.get(&id).map(|a| a.version.max(self.versiones.get(&(Clase::Interior, id)).copied().unwrap_or(0))),
            _ => self.versiones.get(&(clase, id)).copied(),
        }
        .unwrap_or(self.version_de_arranque)
    }

    /// La secuencia propia del interior de la plaza `id`: cuántos pasos de minuto lleva dados (`(ultimo_minuto - fundado_en) / 60 s`, porque los pasos caen en `fundado_en + k·60 s`). Lo que cambia el paso de
    /// minuto dentro de la plaza (almacén, población, medidor, nutrición, edificios parados, tendencia y almacén local de sus pueblos) **no sube la versión del mundo**: sube esta secuencia, que viaja
    /// en `Asentamiento.version_interior` y sirve a la reanudación de `ASENTAMIENTO` (4H T3c, decisión del autor 2026-10-23). Es función del estado que ya se persiste (no añade nada a la foto) y vale
    /// lo mismo con sesiones y sin ellas, así que el contador `version` y el hash tampoco dependen de si alguien escucha. `None` si la plaza no existe.
    pub fn version_interior(&self, id: u64) -> Option<u64> {
        let eco = &self.estado.asentamientos.get(&id)?.economia;
        Some(((eco.ultimo_minuto - eco.fundado_en) / base::MS_POR_MINUTO).max(0) as u64)
    }

    // ───────────────────────────── Reglas de las columnas ─────────────────────────────

    fn evento(&mut self, codigo: &'static str, en: Instante, id: u64, punto: Option<Punto>, datos: Vec<(&'static str, i64)>) {
        self.tr.emitir(&self.estado, EventoNucleo { codigo: Cow::Borrowed(codigo), en, ejercitos: vec![id], asentamientos: vec![], punto, datos, ..Default::default() });
    }

    /// Programa la llegada (si hay plan), los umbrales de víveres y, si la columna pasa hambre, su próxima deserción, con la generación vigente. Llamar tras subirla.
    fn programar_ejercito(&mut self, id: u64, t: Instante) {
        let Some(e) = self.estado.ejercitos.get(&id) else { return };
        let (fin, viveres) = (e.plan.fin(), e.columna.viveres);
        let cap = viveres.maximo.unwrap_or(0);
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
        self.programar_desercion(id, t);
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

    /// La columna deja de moverse donde está en `t`: sin plan, víveres al ritmo de acampada, generación nueva. Una columna quieta no persigue.
    fn detener(&mut self, id: u64, t: Instante) {
        let Some(e) = self.estado.ejercitos.get_mut(&id) else { return };
        e.posicion = e.posicion_en(t);
        let g = self.estado.generaciones.subir(id);
        e.plan = Plan { generacion: g, vertices: Vec::new() };
        self.tr.tocar(Clase::Ejercito, id);
        self.fijar_persecucion(id, None);
        self.poner_marcha(id, t, militar::racion::Estado::Estacionada);
        self.programar_ejercito(id, t);
        self.avisar_plan(id, false);
        self.anunciar_columna(id, Motivo::Plan);
    }

    /// La columna ha puesto en marcha `camino` desde donde está en `t`, a la velocidad que le da su composición: plan nuevo, generación nueva y todo lo
    /// que dependía del plan anterior, reprogramado.
    fn poner_ruta(&mut self, id: u64, t: Instante, camino: Camino) {
        let e = &self.estado.ejercitos[&id];
        let (origen, velocidad) = (e.posicion_en(t), self.velocidad_de_columna(e).max(1));
        let g = self.estado.generaciones.subir(id);
        let e = self.estado.ejercitos.get_mut(&id).expect("existe");
        e.plan = camino.a_plan(t, velocidad, g);
        e.posicion = origen;
        self.tr.tocar(Clase::Ejercito, id);
        self.poner_marcha(id, t, militar::racion::Estado::Marchando);
        self.programar_ejercito(id, t);
        self.avisar_plan(id, true);
        self.anunciar_columna(id, Motivo::Plan);
    }

    fn llegada(&mut self, id: u64, t: Instante) {
        let Some(e) = self.estado.ejercitos.get(&id) else { return };
        let Some(ultimo) = e.plan.vertices.last().copied() else { return };
        // Quien persigue no acampa al llegar al último punto donde vio a su presa: la sigue, a no ser que ya no tenga a quién.
        if e.persiguiendo.is_some() && self.seguir_presa(id) {
            return;
        }
        self.detener(id, t);
        self.evento("ejercito.llega", t, id, Some(ultimo.p), vec![]);
    }

    fn umbral(&mut self, id: u64, objetivo: Mil, t: Instante) {
        let Some(e) = self.estado.ejercitos.get(&id) else { return };
        let valor = e.columna.viveres.valor_en(t);
        if valor > objetivo {
            return; // defensivo: el evento no corresponde al ritmo vigente
        }
        let punto = e.posicion_en(t);
        if objetivo <= 0 {
            // Sin víveres la tropa pasa hambre desde ahora (la moral de cada escuadra baja y, a 0, deserta): la columna no se detiene.
            self.evento("ejercito.sin_viveres", t, id, Some(punto), vec![]);
            self.reprogramar_columna(id, t);
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

    /// Alta de una columna por la vía de administración, con su héroe de campo (cuenta `cuenta`). Devuelve su id. La velocidad es la fija de la columna
    /// (30 unidades por minuto = 500) y los víveres, los que lleva al nacer (topados a la capacidad de un héroe).
    pub fn crear_ejercito(&mut self, t: Instante, cuenta: &str, faccion: u32, posicion: Punto, velocidad: Mil, viveres: Mil) -> Result<u64, Respuesta> {
        let comando = Comando::CrearEjercito { cuenta: cuenta.into(), faccion, posicion, velocidad, viveres };
        let r = self.aplicar_en(t, Peticion::nueva("admin", &format!("crear:{}", self.estado.siguiente_id), comando).de_admin());
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
            return Respuesta::duplicada(v);
        }
        let t = t.max(self.estado.ahora);
        self.procesar_hasta(t);
        if self.ajustes.auditar_cambios {
            self.causa_auditada = format!("la orden {:?}", p.comando);
        }
        let r = match p.version_esperada {
            Some(v) if self.estado.version > v => Respuesta::rechazo(
                rechazos::VERSION_OBSOLETA,
                format!("el mundo está en la versión {}, se esperaba {v}", self.estado.version),
                self.estado.version,
            ),
            _ => self.ejecutar(t, &p, precalculado),
        };
        // Una orden que no se acepta no deja rastro: lo que marcó antes de rechazarse no cuenta como cambio.
        if r.resultado != Resultado::Aceptada {
            self.tr.tocados = Tocados::default();
        }
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
        let sin_cambios = || Respuesta::sin_cambios(v);
        let mut creadas = Vec::new();
        if !p.admin && p.comando.autoriza() == Autoriza::Administracion {
            return Respuesta::rechazo(rechazos::NO_AUTORIZADO, "solo administración", v);
        }
        if let Some(r) = self.rechazo_por_batalla(t, p) {
            return r;
        }
        match &p.comando {
            Comando::CrearEjercito { cuenta, faccion, posicion, velocidad, viveres } => {
                // Andamiaje de administración: una columna con un héroe «de campo» (sin residencia ni loadout) de la cuenta dada, a la que obedece.
                let (id, heroe) = (self.nuevo_id(), self.nuevo_id());
                let mr = self.estado.reglas.economia.militar.clone();
                let mut h = militar::heroe::Heroe::nuevo(&mr, heroe, militar::heroe::Controlador::Bot, 0, heroe);
                h.residencia = None;
                h.loadouts.clear();
                h.presencia = militar::presencia::Presencia::nueva(militar::presencia::Ubicacion::Columna(id));
                self.estado.militar.insertar(HeroeNucleo { sintetico: true, ..HeroeNucleo::nuevo(h, cuenta, Some(*faccion)) });
                self.otorgar_de_administracion(heroe, *faccion);
                self.tr.tocar(Clase::Heroe, heroe);
                let columna = militar::columna::Columna::salir(&mr, id, Some(*faccion), heroe, militar::columna::TipoColumna::Ejercito, militar::columna::Politica::Rechazar, vec![], Default::default(), *viveres, t, &|_| 0);
                let mut e = self.columna_suelta(columna, *posicion);
                e.velocidad_fija = Some(*velocidad);
                self.alta_de_columna(t, e);
                creadas.push(id);
            }
            Comando::MarcharA { ejercito, destino } => {
                let Some(e) = self.estado.ejercitos.get(ejercito) else {
                    return Respuesta::rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(*ejercito), v);
                };
                if let Err(r) = self.autorizar_movimiento(p, e) {
                    return r;
                }
                let (id, origen, marchando, lider) = (e.id, e.posicion_en(t), e.marchando(), e.columna.lider);
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
                    // Elegir destino es dejar de ir detrás de nadie, y sin avisar.
                    self.fijar_persecucion(id, None);
                    self.poner_ruta(id, t, camino);
                    // Presencia_Y_Convocatoria §6: `rectifica` si la columna ya marchaba; el objetivo es el punto de destino (el contrato lleva a una plaza como su posición).
                    let datos = vec![("heroe", lider as i64), ("rectifica", marchando as i64), ("objetivoX", destino.x), ("objetivoY", destino.y)];
                    self.evento("jugador.marcha_fijada", t, id, Some(origen), datos);
                }
            }
            Comando::Estacionar { ejercito } => {
                let Some(e) = self.estado.ejercitos.get(ejercito) else {
                    return Respuesta::rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(*ejercito), v);
                };
                if let Err(r) = self.autorizar_movimiento(p, e) {
                    return r;
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
            Comando::ComprometerRecinto { .. } | Comando::MejorarRecinto { .. } | Comando::AbandonarRecinto { .. } => match self.ejecutar_murallas(t, p) {
                Ok(Some(creada)) => creadas.push(creada),
                Ok(None) => {}
                Err(r) => return r,
            },
            Comando::ComprarMirada { .. } | Comando::ComprarInformePlaza { .. } => match self.ejecutar_taberna(t, p) {
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
            | Comando::ColocarHeroe { .. }
            | Comando::HerirHeroe { .. } => match self.ejecutar_heroes(t, p) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::SalirAlMundo { .. }
            | Comando::MovilizarEjercito { .. }
            | Comando::SalirDelCampamento { .. }
            | Comando::EntrarEnAsentamiento { .. }
            | Comando::SalirDeAsentamiento { .. }
            | Comando::Guarnecer { .. }
            | Comando::FijarAcceso { .. }
            | Comando::VaciarCarro { .. }
            | Comando::EntrarEnCampamento { .. } => match self.ejecutar_salidas(t, p) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::EntrarEnPueblo { .. } | Comando::SalirDePueblo { .. } | Comando::EntrarEnAvanzada { .. } | Comando::SalirDeAvanzada { .. } => match self.ejecutar_dentro(t, p) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::UnirseAEjercito { .. }
            | Comando::UnirseEnCampo { .. }
            | Comando::ResponderPeticionDeUnion { .. }
            | Comando::SepararseDelEjercito { .. }
            | Comando::CederLiderazgo { .. }
            | Comando::OrganizarEjercito { .. }
            | Comando::CancelarFormacion { .. }
            | Comando::Repostar { .. } => match self.ejecutar_uniones(t, p) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::ConvocarEjercito { .. }
            | Comando::UnirseAConvocatoria { .. }
            | Comando::ResponderPeticionDeConvocatoria { .. }
            | Comando::CambiarSeleccionDeConvocatoria { .. }
            | Comando::SepararseDeConvocatoria { .. }
            | Comando::CancelarConvocatoria { .. }
            | Comando::PartirConvocatoria { .. } => match self.ejecutar_convocatorias(t, p) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::CrearFaccion { .. }
            | Comando::SolicitarIngreso { .. }
            | Comando::ResponderSolicitud { .. }
            | Comando::DejarFaccion { .. }
            | Comando::AsignarRey { .. }
            | Comando::AsignarEmbajador { .. }
            | Comando::CambiarResidencia { .. }
            | Comando::ResidirEnCampamento { .. }
            | Comando::DejarResidencia { .. }
            | Comando::FijarFaccionDeHeroe { .. } => match self.ejecutar_politica(t, p) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::DesignarCapital { .. } | Comando::RenombrarAsentamiento { .. } => match self.ejecutar_capital_y_nombre(t, p) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::Perseguir { .. } | Comando::DejarDePerseguir { .. } => match self.ejecutar_persecuciones(t, p) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::Inspeccionar { heroe, objetivo } => match self.inspeccionar(t, p, *heroe, *objetivo) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::FijarRelacion { a, b, relacion } => match self.fijar_relacion(t, *a, *b, *relacion) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::Atacar { heroe, objetivo, declara_guerra } => match self.atacar(t, p, *heroe, *objetivo, *declara_guerra) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::FijarControlador { .. }
            | Comando::UnirseABatalla { .. }
            | Comando::CancelarBatalla { .. }
            | Comando::RegistrarAsignacion { .. }
            | Comando::ConfirmarInicio { .. }
            | Comando::RegistrarTokens { .. }
            | Comando::RegistrarSalida { .. }
            | Comando::AplicarResultado { .. } => match self.ejecutar_batallas(t, p) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::ColocarBandidos { posicion, nivel } => match self.colocar_bandidos(t, *posicion, *nivel) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::DarEscuadra { heroe, tropa, cantidad } => match self.dar_escuadra(t, *heroe, *tropa, *cantidad) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::MarcarPueblo { .. } | Comando::EntregarMateriales { .. } | Comando::DestruirPueblo { .. } | Comando::ReponerCarro { .. } => match self.ejecutar_pueblos(t, p) {
                Ok(Some(creada)) => creadas.push(creada),
                Ok(None) => {}
                Err(r) => return r,
            },
            Comando::ColocarAvanzada { .. }
            | Comando::EntregarMaterialesAAvanzada { .. }
            | Comando::MejorarAvanzada { .. }
            | Comando::RepararAvanzada { .. }
            | Comando::DestruirAvanzada { .. }
            | Comando::CancelarObraDeAvanzada { .. }
            | Comando::ElegirTropaDeAvanzada { .. }
            | Comando::GuarnecerAvanzada { .. }
            | Comando::RetirarDeAvanzada { .. }
            | Comando::RepostarEnAvanzada { .. }
            | Comando::DepositarEnAvanzada { .. } => match self.ejecutar_avanzadas(t, p) {
                Ok(ids) => creadas.extend(ids),
                Err(r) => return r,
            },
            Comando::Reloj(accion) => {
                if let Err(r) = self.controlar_reloj(*accion) {
                    return r.unwrap_or_else(sin_cambios);
                }
            }
        }
        let tipo = p.comando.tipo_creado();
        debug_assert!(creadas.is_empty() || tipo.is_some(), "{:?} crea {creadas:?} sin declarar qué (`Comando::tipo_creado`)", p.comando);
        if let Some(tipo) = tipo {
            tipo.tocadas(&creadas).into_iter().for_each(|(clase, id)| self.tr.tocar(clase, id));
        }
        p.comando.nombradas().into_iter().for_each(|(clase, id)| self.tr.tocar(clase, id));
        self.cerrar_version(t, true);
        Respuesta::aceptada_con(self.estado.version, creadas, tipo)
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
