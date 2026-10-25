//! La población sintética del sistema «economía y comercio»: Facciones de varias plazas fundadas a la vez sobre el mundo real que viven
//! `dias` días sobre el núcleo (economía real de cada plaza, trueques, flota que viaja por el mapa, peajes, red de caminos, mercenarios y
//! pueblos extractores). Un planificador propio (cola de acciones por instante) avanza el núcleo con `avanzar_hasta` y aplica cada orden
//! con `aplicar_en`; los hechos se leen del registro de cambios cada media hora de mundo (`Vigia`).
//!
//! # El «jugador» mínimo (hipótesis del batch, deterministas)
//! Cada plaza tiene su columna parada en la puerta (fundó con ella) y su propia Facción-actor. Cada hora: pide la subida de nivel; con 150
//! de madera sobrante construye el Mercado; con Mercado monta una caravana (2 carros con buey); con la flota lista propone un trueque a una
//! plaza vecina (la mitad de las veces de otra Facción) de lo que le sobra por lo que le falta; y de nivel 2 en adelante marca un pueblo
//! extractor y manda a su columna a comprar los materiales al campamento de mercenarios más cercano y a entregarlos en la obra. La otra
//! parte contesta a los diez minutos: acepta si puede cumplir su lado y rechaza si no.
//!
//! # Andamiaje de administración (provisional hasta el sistema de héroes)
//! El oro que lleva el carro de una columna (`DarSuministro`), la residencia y el almacén personal de los sondeos de cupo (`FijarResidencia`,
//! `DarAlmacenPersonal`, del héroe que va en la columna) y la Facción de nivel 8 (`FijarNivelFaccion`, nivel 8: cap de fundación 6 y cupo de 4 plazas de nivel 2) salen de la administración. Todo el oro
//! inyectado se suma en `oro_inyectado` y se declara en el informe.

use crate::auditoria::{auditar_cola, Violaciones};
use crate::geo::{Cargado, Geo};
use crate::sim::{cuenta_de_gobernador as gobernador, gobernador_de_oficio, nucleo_nuevo, ANILLO_CAMPAMENTO, ANILLO_PLAZA, ASCENSO_CADA_MIN, CANDIDATOS, VELOCIDAD, VIVERES};
use crate::vigia::Vigia;
use asentamiento::catalogo::Tipo;

use asentamiento::recurso::TODOS;
use asentamiento::Recurso;
use base::{Instante, Punto, Rng, MS_POR_MINUTO};
use comercio::caravana::{Animal, EstadoCaravana, TipoCarro};
use comercio::red_caminos::Arista;
use comercio::trueque::EstadoTrueque;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::{Comando, Nucleo, Peticion, Resultado};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use std::time::{Duration, Instant};
use territorio::Viabilidad;

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;
const DIA: i64 = 24 * HORA;

// ── Supuestos del batch (hipótesis sintéticas, se anotan en `parametros` del informe) ──
/// Plazas de cada Facción (bajo el cap de fundación de una Facción de nivel 8, que es 6).
pub const PLAZAS_POR_FACCION: u32 = 4;
pub const NIVEL_FACCION: u32 = 8;
pub const TURNO_MIN: i64 = 60;
pub const OBSERVAR_MIN: i64 = 30;
/// El «jugador» responde a una propuesta a los diez minutos.
pub const RESPUESTA_MIN: i64 = 10;
/// Madera que debe sobrar (por encima del coste del Mercado) para que se construya.
pub const RESERVA_MADERA: i64 = 50_000;
pub const COSTE_MERCADO: i64 = 100_000;
pub const CARROS_POR_CARAVANA: usize = 2;
/// Vecinas candidatas para un trueque y radio máximo (milésimas).
pub const VECINAS: usize = 10;
pub const RADIO_TRUEQUE: i64 = 300_000;
/// Probabilidad (%) de proponer en una hora cualquiera con la flota lista.
pub const PROPONE_PCT: u64 = 20;
/// Probabilidad (%) de buscar la pareja en otra Facción.
pub const OTRA_FACCION_PCT: u64 = 60;
/// Cantidad máxima que se ofrece en un trueque (milésimas) y fracción mínima del almacén que debe quedar.
pub const OFERTA_MAX: i64 = 80_000;
/// Un mineral cuenta como excedente desde esta cantidad (milésimas) y como carencia por debajo.
pub const MINERAL_MIN: i64 = 5_000;
/// Un recurso «sobra» por encima de este porcentaje de su capacidad y «falta» por debajo de este otro.
pub const SOBRA_PCT: i64 = 45;
pub const FALTA_PCT: i64 = 25;
pub const PUEBLOS_OBJETIVO: usize = 2;
/// Los campamentos reponen sus pilas cada 3 h: una obra sin materiales espera esa reposición para volver a comprar.
pub const REINTENTO_OBRA_H: i64 = 3;
/// Plazas (por Facción, la primera) cuyas columnas hacen el sondeo del cupo diario y el viaje de los minerales.
pub const FACCIONES_SONDEO: u32 = 6;
pub const FACCIONES_MINERALES: u32 = 12;
pub const DIA_SONDEO: i64 = 3;
pub const DIA_MINERALES: i64 = 2;

/// Lo que todas las plazas tienen lleno (no se trueca) y los minerales (escasos: solo los hay si se compran a los mercenarios).
const BASICOS: [Recurso; 4] = [Recurso::Madera, Recurso::Piedra, Recurso::Trigo, Recurso::Oro];
const MINERALES: [Recurso; 3] = [Recurso::Cobre, Recurso::Estano, Recurso::Hierro];

/// Qué se anota de cada tipo de orden: aceptadas, sin cambios y rechazos por código.
#[derive(Clone, Debug, Default)]
pub struct Cuenta {
    pub aceptadas: u64,
    pub sin_cambios: u64,
    pub rechazos: BTreeMap<&'static str, u64>,
}

/// Lo que se sabe de cada plaza fundada.
#[derive(Clone, Debug)]
pub struct Colocada {
    pub faccion: u32,
    pub pos: Punto,
    pub columna: u64,
    /// El héroe que va en la columna (sin residencia: visitante en todos los campamentos salvo durante el sondeo del cupo) y su cuenta.
    pub heroe: u64,
    pub cuenta: String,
    /// Campamento de mercenarios más cercano (índice del artefacto).
    pub campamento: u32,
}

/// Una foto diaria de lo que se vigila en el tiempo.
#[derive(Clone, Debug, Default)]
pub struct Muestra {
    pub dia: u32,
    /// Reputación de cada Facción que alguna vez comerció (ppm de punto).
    pub reputacion: Vec<i64>,
    pub aristas: usize,
    pub rutas: usize,
    /// Caravanas comerciales por estado, en el orden de `ESTADOS`.
    pub caravanas: [u32; 6],
    pub plazas_vivas: usize,
    pub con_mercado: usize,
    pub con_flota: usize,
    /// Ocupación media del almacén de las plazas (% de la capacidad) de madera, piedra, trigo, ganado y oro.
    pub llenado: [i64; 5],
}

pub const LLENADO: [Recurso; 5] = [Recurso::Madera, Recurso::Piedra, Recurso::Trigo, Recurso::Livestock, Recurso::Oro];

pub const ESTADOS: [(EstadoCaravana, &str); 6] = [
    (EstadoCaravana::Disponible, "disponible"),
    (EstadoCaravana::Preparando, "preparando"),
    (EstadoCaravana::EnTransito, "en tránsito"),
    (EstadoCaravana::Retornando, "retornando"),
    (EstadoCaravana::Aparcada, "aparcada"),
    (EstadoCaravana::Adjunta, "adjunta"),
];

pub struct Corrida {
    pub nucleo: Nucleo,
    pub vigia: Vigia,
    pub plazas: Vec<Colocada>,
    pub facciones: u32,
    pub ordenes: BTreeMap<&'static str, Cuenta>,
    pub muestras: Vec<Muestra>,
    pub oro_inyectado: i64,
    pub violaciones: Violaciones,
    /// Mayor peso que tuvo cada arista de la red en las fotos diarias (cuántas rutas vigentes la pisaban).
    pub pico_red: BTreeMap<Arista, u32>,
    /// Cobre, estaño y hierro: lo que debe haber (lo inicial más lo comprado a los mercenarios) y lo que hay en todos los contenedores al final.
    pub balance_minerales: Vec<(Recurso, i64, i64)>,
    /// Viajes de compra que necesitó cada obra de pueblo que llegó a completarse.
    pub viajes_por_obra: Vec<u32>,
    /// Misiones de columna (pueblo, cupo, minerales) lanzadas y terminadas.
    pub misiones: BTreeMap<&'static str, (u32, u32)>,
    /// La vía de juego de la tecnología (6H): turnos de una capital con algo aparecido sin adoptar y el Rey fuera de ella.
    pub adopcion_rey_fuera: u64,
    pub pared: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Clase {
    Pueblo,
    Cupo,
    Minerales,
}

impl Clase {
    fn nombre(self) -> &'static str {
        match self {
            Clase::Pueblo => "pueblo",
            Clase::Cupo => "cupo",
            Clase::Minerales => "minerales",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Paso {
    Comprar,
    Entregar,
    Vender,
    Fin,
}

#[derive(Clone, Copy, Debug)]
struct Mision {
    tipo: Clase,
    pueblo: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
enum Accion {
    Turno(u64),
    Responder(u64),
    Observar,
    Mision { plaza: u64, paso: Paso },
    Sondeo { plaza: u64, tipo: Clase },
}

struct Pj {
    info: Colocada,
    vecinas: Vec<u64>,
    mision: Option<Mision>,
    /// Trueque que esta plaza propuso y sigue abierto.
    propuesta: Option<u64>,
    obra_pendiente: Option<u64>,
    /// Viajes de compra de la obra pendiente y no antes de cuándo se vuelve a intentar.
    viajes_obra: u32,
    proxima_obra: Instante,
}

struct Sim<'a> {
    n: Nucleo,
    geo: &'a Geo,
    rng: Rng,
    fin: Instante,
    control: bool,
    cola: BinaryHeap<Reverse<(Instante, usize)>>,
    acciones: Vec<Accion>,
    pjs: BTreeMap<u64, Pj>,
    facciones: Vec<Vec<u64>>,
    vigia: Vigia,
    faccion_de: BTreeMap<u64, u32>,
    pico_red: BTreeMap<Arista, u32>,
    /// Cobre, estaño y hierro que hay en el mundo al empezar.
    minerales0: Vec<(Recurso, i64)>,
    ordenes: BTreeMap<&'static str, Cuenta>,
    muestras: Vec<Muestra>,
    contador: u64,
    oro_inyectado: i64,
    violaciones: Violaciones,
    viajes_por_obra: Vec<u32>,
    misiones: BTreeMap<&'static str, (u32, u32)>,
    adopcion_rey_fuera: u64,
}

fn jugador(f: u32) -> String {
    format!("f{f}")
}

fn punto_en_anillo(rng: &mut Rng, c: Punto, (rmin, rmax): (i64, i64)) -> Punto {
    let r = (rmin + rng.por_debajo((rmax - rmin) as u64) as i64) as f64;
    let a = (rng.por_debajo(360_000) as f64 / 1000.0).to_radians();
    Punto::new(c.x + (r * a.cos()) as i64, c.y + (r * a.sin()) as i64)
}

/// Un sitio fundable para la Facción `f` en el anillo: el primer Verde, o el mejor Ámbar; nunca uno condenado a ruinas por falta de madera. Lo usan
/// `batch comercio` y `batch militar`.
pub(crate) fn sitio(n: &Nucleo, rng: &mut Rng, f: u32, centro: Punto, anillo: (i64, i64)) -> Option<Punto> {
    let mut ambar: Vec<((u8, usize), Punto)> = Vec::new();
    for _ in 0..CANDIDATOS {
        let p = punto_en_anillo(rng, centro, anillo);
        let v = n.viabilidad(p, f).expect("el núcleo tiene mundo");
        if v.motivo.is_some() || v.condenada {
            continue;
        }
        if v.color == Viabilidad::Verde {
            return Some(p);
        }
        ambar.push((((v.parcelas_libres > 0) as u8 + (v.piedra > 0) as u8, v.parcelas_libres + v.piedra), p));
    }
    ambar.into_iter().max_by_key(|&(k, _)| k).map(|(_, p)| p)
}

/// Corre `plazas` plazas (en Facciones de `PLAZAS_POR_FACCION`) durante `dias` días de mundo. Con `control` solo funda y asciende: no comercia
/// (sirve para medir lo que cuesta el comercio).
pub fn simular(c: &Cargado, geo: &Geo, plazas: u32, dias: u32, semilla: u64, control: bool, paso_normal: bool) -> Corrida {
    let t0 = Instant::now();
    let mut s = Sim {
        n: nucleo_nuevo(c, semilla),
        geo,
        rng: Rng::derivado(semilla, "batch-comercio"),
        fin: dias as i64 * DIA,
        control,
        cola: BinaryHeap::new(),
        acciones: Vec::new(),
        pjs: BTreeMap::new(),
        facciones: Vec::new(),
        vigia: Vigia::default(),
        faccion_de: BTreeMap::new(),
        pico_red: BTreeMap::new(),
        minerales0: Vec::new(),
        ordenes: BTreeMap::new(),
        muestras: Vec::new(),
        contador: 0,
        oro_inyectado: 0,
        violaciones: Violaciones::default(),
        viajes_por_obra: Vec::new(),
        misiones: BTreeMap::new(),
        adopcion_rey_fuera: 0,
    };
    s.fundar_todas(plazas);
    s.arrancar();
    s.correr(paso_normal);
    let balance_minerales = s.balance_minerales();
    // El cobre se funde y los lingotes se gastan en mejoras (la de la Armería): solo el estaño y el hierro cuadran al milésimo.
    s.violaciones.conservacion += balance_minerales.iter().filter(|(r, debe, hay)| *r != Recurso::Cobre && debe != hay).count() as u64;
    s.violaciones.registro += s.vigia.huecos;
    let f = s.facciones.len() as u32;
    Corrida {
        plazas: s.pjs.values().map(|p| p.info.clone()).collect(),
        nucleo: s.n,
        vigia: s.vigia,
        facciones: f,
        ordenes: s.ordenes,
        muestras: s.muestras,
        oro_inyectado: s.oro_inyectado,
        violaciones: s.violaciones,
        pico_red: s.pico_red,
        balance_minerales,
        viajes_por_obra: s.viajes_por_obra,
        misiones: s.misiones,
        adopcion_rey_fuera: s.adopcion_rey_fuera,
        pared: t0.elapsed(),
    }
}

impl Sim<'_> {
    fn programar(&mut self, t: Instante, a: Accion) {
        if t <= self.fin {
            self.acciones.push(a);
            self.cola.push(Reverse((t, self.acciones.len() - 1)));
        }
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    fn peticion(&mut self, actor: &str, c: Comando) -> Peticion {
        self.contador += 1;
        let p = Peticion::nueva(actor, &format!("b{}", self.contador), c);
        if actor == "admin" {
            p.de_admin()
        } else {
            p
        }
    }

    /// Aplica una orden y la anota en las cuentas de su tipo.
    fn orden(&mut self, t: Instante, actor: &str, etiqueta: &'static str, c: Comando) -> nucleo::Respuesta {
        let p = self.peticion(actor, c);
        let r = self.n.aplicar_en(t, p);
        let cuenta = self.ordenes.entry(etiqueta).or_default();
        match &r.resultado {
            Resultado::Aceptada => cuenta.aceptadas += 1,
            Resultado::SinCambios => cuenta.sin_cambios += 1,
            Resultado::Rechazada { codigo, .. } => *cuenta.rechazos.entry(codigo).or_insert(0) += 1,
            Resultado::Duplicada => unreachable!("los ids de petición no se repiten"),
        }
        r
    }

    fn admin(&mut self, t: Instante, etiqueta: &'static str, c: Comando) -> nucleo::Respuesta {
        self.orden(t, "admin", etiqueta, c)
    }

    fn vive(&self, id: u64) -> bool {
        self.n.estado().asentamientos.contains_key(&id)
    }

    // ───────────────────────────── Fundación ─────────────────────────────

    fn campamento_mas_cercano(&self, p: Punto) -> u32 {
        self.geo.campamentos.iter().enumerate().min_by_key(|(_, c)| p.dist(**c)).map_or(0, |(i, _)| i as u32)
    }

    /// Funda `plazas` plazas repartidas en Facciones de `PLAZAS_POR_FACCION`: la primera en el anillo de un campamento (en orden, uno por
    /// Facción), las demás junto a las que ya tiene. Una columna por plaza, nacida en el sitio, que funda con su Caravana de Fundación.
    fn fundar_todas(&mut self, plazas: u32) {
        let facciones = plazas.div_ceil(PLAZAS_POR_FACCION);
        for f in 1..=facciones {
            self.facciones.push(Vec::new());
            self.admin(0, "FijarNivelFaccion", Comando::FijarNivelFaccion { faccion: f, nivel: NIVEL_FACCION });
        }
        let mut minuto = 0;
        for k in 0..plazas {
            let f = k % facciones + 1;
            let propias = self.facciones[f as usize - 1].clone();
            let sitio = if propias.is_empty() {
                let centro = self.geo.campamentos[f as usize % self.geo.campamentos.len()];
                sitio(&self.n, &mut self.rng, f, centro, ANILLO_CAMPAMENTO)
            } else {
                let madre = propias[self.rng.por_debajo(propias.len() as u64) as usize];
                let centro = self.n.estado().asentamientos[&madre].posicion;
                sitio(&self.n, &mut self.rng, f, centro, ANILLO_PLAZA)
            };
            let Some(p) = sitio else { continue };
            minuto += 1;
            let t = minuto * MIN;
            self.n.avanzar_hasta(t);
            let Ok(columna) = self.n.crear_ejercito(t, &jugador(f), f, p, VELOCIDAD, VIVERES) else { continue };
            self.admin(t, "DarCaravanaFundacion", Comando::DarCaravanaFundacion { ejercito: columna });
            let r = self.orden(t, &jugador(f), "Fundar", Comando::Fundar { ejercito: columna });
            let Some(&id) = r.creadas.first().filter(|_| r.aceptada()) else { continue };
            // 5E: las órdenes de la plaza las da su Gobernador de oficio, residente y dentro (`gobernador_de_oficio`, con su cuenta propia).
            gobernador_de_oficio(&mut self.n, t, id).expect("el Gobernador de oficio de la plaza");
            // Su héroe nace en el campamento más cercano y va en la columna; sin residencia, es un visitante de todos los campamentos.
            let (campamento, cuenta) = (self.campamento_mas_cercano(p), format!("{}-{k}", jugador(f)));
            let h = self.admin(t, "CrearHeroe", Comando::CrearHeroe { cuenta: cuenta.clone(), campamento, controlador: Controlador::Humano });
            let Some(&heroe) = h.creadas.first().filter(|_| h.aceptada()) else { continue };
            self.admin(t, "FijarResidencia", Comando::FijarResidencia { heroe, residencia: None });
            self.admin(t, "ColocarHeroe", Comando::ColocarHeroe { heroe, ubicacion: Ubicacion::Columna(columna) });
            self.facciones[f as usize - 1].push(id);
            self.faccion_de.insert(id, f);
            let info = Colocada { faccion: f, pos: p, columna, heroe, cuenta, campamento };
            self.pjs.insert(id, Pj { info, vecinas: Vec::new(), mision: None, propuesta: None, obra_pendiente: None, viajes_obra: 0, proxima_obra: 0 });
        }
        // Las vecinas de cada plaza: las más cercanas de las demás, hasta el radio del trueque.
        let todas: Vec<(u64, Punto)> = self.pjs.iter().map(|(&i, p)| (i, p.info.pos)).collect();
        for (id, pos) in &todas {
            let mut v: Vec<(i64, u64)> = todas.iter().filter(|(o, _)| o != id).map(|&(o, q)| (pos.dist(q), o)).filter(|&(d, _)| d <= RADIO_TRUEQUE).collect();
            v.sort_unstable();
            self.pjs.get_mut(id).expect("existe").vecinas = v.into_iter().take(VECINAS).map(|(_, o)| o).collect();
        }
    }

    // ───────────────────────────── Bucle ─────────────────────────────

    fn arrancar(&mut self) {
        self.minerales0 = MINERALES.iter().map(|&r| (r, self.mineral_en_el_mundo(r))).collect();
        let t0 = self.n.ahora();
        let ids: Vec<u64> = self.pjs.keys().copied().collect();
        for (k, id) in ids.iter().enumerate() {
            // Los turnos se reparten por los minutos de la hora para no apilar toda la población en el mismo instante.
            self.programar(t0 + (k as i64 % TURNO_MIN + 1) * MIN, Accion::Turno(*id));
        }
        self.programar(t0 + OBSERVAR_MIN * MIN, Accion::Observar);
        if self.control {
            return;
        }
        for f in 1..=self.facciones.len() as u32 {
            let Some(&plaza) = self.facciones[f as usize - 1].first() else { continue };
            if f <= FACCIONES_SONDEO {
                self.programar(DIA_SONDEO * DIA + f as i64 * 7 * MIN, Accion::Sondeo { plaza, tipo: Clase::Cupo });
            }
            if f <= FACCIONES_MINERALES {
                self.programar(DIA_MINERALES * DIA + f as i64 * 5 * MIN, Accion::Sondeo { plaza, tipo: Clase::Minerales });
            }
        }
    }

    fn correr(&mut self, paso_normal: bool) {
        while let Some(Reverse((t, i))) = self.cola.pop() {
            if paso_normal {
                // «Modo normal»: el núcleo avanza a ritmo de reloj (cada 7 min de mundo) y no de un salto hasta la orden.
                let mut m = self.n.ahora();
                while m + 7 * MIN < t {
                    m += 7 * MIN;
                    self.n.avanzar_hasta(m);
                }
            }
            self.n.avanzar_hasta(t);
            match self.acciones[i] {
                Accion::Turno(id) => self.turno(t, id),
                Accion::Responder(acuerdo) => self.responder(t, acuerdo),
                Accion::Observar => self.observar(t),
                Accion::Mision { plaza, paso } => self.mision(t, plaza, paso),
                Accion::Sondeo { plaza, tipo } => self.sondeo(t, plaza, tipo),
            }
        }
        self.n.avanzar_hasta(self.fin);
        self.leer();
        self.muestrear(self.fin / DIA);
        self.comprobar_final();
    }

    fn leer(&mut self) {
        let f = &self.faccion_de;
        self.vigia.leer(&self.n, &|p| f.get(&p).copied());
    }

    fn observar(&mut self, t: Instante) {
        self.programar(t + OBSERVAR_MIN * MIN, Accion::Observar);
        self.leer();
        if t % DIA < OBSERVAR_MIN * MIN {
            self.muestrear(t / DIA);
        }
        if t % (6 * HORA) < OBSERVAR_MIN * MIN {
            self.comprobar_periodico();
        }
    }

    // ───────────────────────────── Turno de una plaza ─────────────────────────────

    fn turno(&mut self, t: Instante, id: u64) {
        if !self.vive(id) {
            return;
        }
        self.programar(t + TURNO_MIN * MIN, Accion::Turno(id));
        // Como en `batch territorio`: el «jugador» pide el ascenso cada hora; el núcleo exige los requisitos reales.
        if t % (ASCENSO_CADA_MIN * MIN) < TURNO_MIN * MIN {
            self.orden(t, &gobernador(id), "SolicitarAscenso", Comando::SolicitarAscenso { asentamiento: id });
        }
        self.adoptar(t, id);
        if self.control {
            return;
        }
        self.flota(t, id);
        self.proponer(t, id);
        self.pueblo(t, id);
    }

    /// La vía de juego de la tecnología (6H, como en `batch militar`): si la plaza es la capital efectiva de su Facción y le ha aparecido algo sin adoptar, su Rey lo adopta con la orden
    /// real, dentro de la capital y con la tarifa de su almacén. Aquí el Rey (el héroe de la fundación) va en la columna de la plaza, a la puerta, y la necesita para sus misiones: la primera
    /// vez le pasa la corona (`AsignarRey`) al Gobernador de oficio de la capital, que reside y está dentro. Si tampoco así hay Rey dentro, se cuenta y se espera.
    fn adoptar(&mut self, t: Instante, p: u64) {
        let f = self.n.estado().asentamientos[&p].faccion;
        if self.n.capital_de(f) != Some(p) {
            return;
        }
        let r = self.n.estado().reglas.economia.tecnologia.clone();
        let propias = self.n.tecnologia().de(&r, f);
        let pendientes: Vec<tecnologia::TecId> = propias.aparecidas.iter().filter(|&x| !propias.adoptadas.contiene(x)).collect();
        let rey_de = |n: &Nucleo| n.faccion(f).and_then(|x| x.rey).and_then(|h| n.heroe(h).map(|x| (h, x.cuenta.clone())));
        let Some((mut rey, mut cuenta)) = rey_de(&self.n).filter(|_| !pendientes.is_empty()) else { return };
        let dentro = |n: &Nucleo, h: u64| n.heroe(h).map(|x| x.heroe.presencia.ubicacion) == Some(Ubicacion::Asentamiento(p as u32));
        if !dentro(&self.n, rey) {
            let gobernador = self.n.estado().asentamientos[&p].cargos.titular(asentamiento::Cargo::Gobernador).filter(|&g| dentro(&self.n, g));
            let paso = gobernador.is_some_and(|designado| self.orden(t, &cuenta, "PasarLaCorona", Comando::AsignarRey { heroe: rey, faccion: f, designado }).aceptada());
            match rey_de(&self.n).filter(|&(h, _)| paso && dentro(&self.n, h)) {
                Some(nuevo) => (rey, cuenta) = nuevo,
                None => {
                    self.adopcion_rey_fuera += 1;
                    return;
                }
            }
        }
        for x in pendientes {
            if self.n.estado().asentamientos[&p].economia.almacen.tiene(r.tarifa(x)) {
                self.orden(t, &cuenta, "AdoptarTecnologia", Comando::AdoptarTecnologia { heroe: rey, faccion: f, tecnologia: x });
            }
        }
    }

    /// Construye el Mercado cuando sobra madera y monta la caravana de la plaza (2 carros con buey) cuando el Mercado está activo.
    fn flota(&mut self, t: Instante, id: u64) {
        let (madera, mercado, activo) = {
            let a = &self.n.estado().asentamientos[&id];
            let ed = &a.economia.edificios;
            (a.economia.almacen.cantidad(Recurso::Madera), ed.iter().any(|e| e.tipo == Tipo::Mercado), ed.iter().any(|e| e.tipo == Tipo::Mercado && e.activo()))
        };
        let actor = gobernador(id);
        if !mercado {
            if madera >= COSTE_MERCADO + RESERVA_MADERA {
                self.orden(t, &actor, "AnadirEdificio(Mercado)", Comando::AnadirEdificio { asentamiento: id, tipo: Tipo::Mercado });
            }
            return;
        }
        if !activo {
            return;
        }
        let flota: Vec<(u64, usize, usize)> = self.n.flota_de(id).iter().map(|c| (c.id, c.carros.len(), c.carros.iter().filter(|k| k.animal.is_some()).count())).collect();
        let Some(&(caravana, carros, con_animal)) = flota.first() else {
            self.orden(t, &actor, "CrearCaravana", Comando::CrearCaravana { asentamiento: id });
            return;
        };
        if carros < CARROS_POR_CARAVANA {
            self.orden(t, &actor, "AnadirCarro", Comando::AnadirCarro { caravana, tipo: TipoCarro::Basico });
        }
        for k in con_animal..carros {
            self.orden(t, &actor, "AnadirAnimal", Comando::AnadirAnimal { caravana, carro: k as u32, animal: Animal::Buey });
        }
    }

    /// ¿Tiene la plaza una caravana con al menos un buey, libre para el reparto?
    fn flota_lista(&self, id: u64) -> bool {
        self.n.flota_de(id).iter().any(|c| c.estado == EstadoCaravana::Disponible && c.carros.iter().any(|k| k.animal.is_some()))
    }

    // ───────────────────────────── Trueques ─────────────────────────────

    fn precio(&self, r: Recurso) -> i64 {
        self.n.estado().reglas.economia.comercio.precio.base_de(r).unwrap_or(0)
    }

    /// Cantidades y capacidades de un almacén.
    fn almacen_de(&self, id: u64) -> ([i64; 29], [i64; 29]) {
        let (c, k, _) = self.n.estado().asentamientos[&id].economia.almacen.partes();
        (*c, *k)
    }

    /// Lo que la plaza `a` daría a `b` y lo que le pediría a cambio, si hay un par de recursos que a una le sobra y a la otra le falta
    /// (y viceversa): la doble coincidencia de deseos del trueque. Los recursos básicos y el oro no entran (todas las plazas los tienen
    /// llenos). Nada que no quepa en el almacén que lo recibe.
    fn pareja(&self, a: u64, b: u64) -> Option<(Recurso, i64, Recurso, i64)> {
        let ((ca, ka), (cb, kb)) = (self.almacen_de(a), self.almacen_de(b));
        let ofrece = |c: &[i64; 29], k: &[i64; 29], r: Recurso| !BASICOS.contains(&r) && if MINERALES.contains(&r) { c[r.indice()] >= MINERAL_MIN } else { 100 * c[r.indice()] >= SOBRA_PCT * k[r.indice()] && k[r.indice()] > 0 };
        let quiere = |c: &[i64; 29], k: &[i64; 29], r: Recurso| !BASICOS.contains(&r) && if MINERALES.contains(&r) { c[r.indice()] < MINERAL_MIN } else { 100 * c[r.indice()] <= FALTA_PCT * k[r.indice()] && k[r.indice()] > 0 };
        // Lo que sobra por encima de la reserva (un mineral deja 5 u).
        let libre = |c: &[i64; 29], k: &[i64; 29], r: Recurso| c[r.indice()] - if MINERALES.contains(&r) { 5_000 } else { k[r.indice()] * SOBRA_PCT / 100 };
        for ro in TODOS.iter().copied().filter(|&r| ofrece(&ca, &ka, r) && quiere(&cb, &kb, r)) {
            let q_ro = libre(&ca, &ka, ro).clamp(0, OFERTA_MAX);
            if q_ro < 1_000 || kb[ro.indice()] - cb[ro.indice()] < q_ro {
                continue;
            }
            for rn in TODOS.iter().copied().filter(|&r| r != ro && ofrece(&cb, &kb, r) && quiere(&ca, &ka, r)) {
                let (p_ro, p_rn) = (self.precio(ro), self.precio(rn));
                let q_rn = if p_ro > 0 && p_rn > 0 { (q_ro as i128 * p_ro as i128 / p_rn as i128) as i64 } else { q_ro };
                if q_rn >= 1_000 && q_rn <= libre(&cb, &kb, rn) && q_rn <= ka[rn.indice()] - ca[rn.indice()] {
                    return Some((ro, q_ro, rn, q_rn));
                }
            }
        }
        None
    }

    fn proponer(&mut self, t: Instante, id: u64) {
        if let Some(ac) = self.pjs.get(&id).and_then(|p| p.propuesta) {
            if self.n.acuerdo(ac).is_some_and(|a| !a.estado.terminado()) {
                return;
            }
        }
        if !self.flota_lista(id) || self.rng.por_debajo(100) >= PROPONE_PCT {
            return;
        }
        // La pareja: la más cercana de las vecinas (de otra Facción las más de las veces) con la que haya doble coincidencia.
        let mi_faccion = self.pjs[&id].info.faccion;
        let otra = self.rng.por_debajo(100) < OTRA_FACCION_PCT;
        let hallada = self.pjs[&id].vecinas.iter().copied().find_map(|v| {
            let q = self.pjs.get(&v)?;
            (self.vive(v) && (q.info.faccion != mi_faccion) == otra && self.flota_lista(v)).then(|| self.pareja(id, v).map(|o| (v, o)))?
        });
        let Some((socia, (da, q_da, quiere, q_quiere))) = hallada else { return };
        let r = self.orden(t, &gobernador(id), "ProponerTrueque", Comando::ProponerTrueque { a: id, b: socia, lineas_a: vec![(da, q_da)], lineas_b: vec![(quiere, q_quiere)] });
        if let Some(&ac) = r.creadas.first().filter(|_| r.aceptada()) {
            self.pjs.get_mut(&id).expect("existe").propuesta = Some(ac);
            self.programar(t + RESPUESTA_MIN * MIN, Accion::Responder(ac));
        }
    }

    /// La parte B contesta: acepta si tiene flota y puede cumplir su lado; si no, rechaza.
    fn responder(&mut self, t: Instante, acuerdo: u64) {
        let Some(a) = self.n.acuerdo(acuerdo).filter(|a| a.estado == EstadoTrueque::Propuesto) else { return };
        let b = a.b as u64;
        let lineas: Vec<(Recurso, i64)> = a.lineas_b.iter().map(|l| (l.recurso, l.cantidad_total)).collect();
        let entra: Vec<(Recurso, i64)> = a.lineas_a.iter().map(|l| (l.recurso, l.cantidad_total)).collect();
        if !self.vive(b) {
            return;
        }
        let almacen = &self.n.estado().asentamientos[&b].economia.almacen;
        let puede = self.flota_lista(b) && lineas.iter().all(|&(r, n)| almacen.cantidad(r) >= n) && entra.iter().all(|&(r, n)| almacen.espacio(r) >= n);
        let (etiqueta, c) = if puede { ("AceptarTrueque", Comando::AceptarTrueque { acuerdo }) } else { ("RechazarTrueque", Comando::RechazarTrueque { acuerdo }) };
        self.orden(t, &gobernador(b), etiqueta, c);
    }

    // ───────────────────────────── Pueblos y misiones de columna ─────────────────────────────

    /// Marca un pueblo extractor (de nivel 2 en adelante) y manda a la columna a comprar los materiales al campamento más cercano.
    fn pueblo(&mut self, t: Instante, id: u64) {
        let Some(p) = self.pjs.get(&id) else { return };
        if p.mision.is_some() {
            return;
        }
        let a = &self.n.estado().asentamientos[&id];
        let nivel = a.economia.nivel_actual();
        if let Some(pueblo) = p.obra_pendiente {
            // Una obra a medias (el campamento no tenía todos los materiales): otro viaje cuando se reponga.
            if t >= p.proxima_obra {
                self.empezar(t, id, Mision { tipo: Clase::Pueblo, pueblo: Some(pueblo) });
            }
            return;
        }
        if nivel < 2 || self.n.pueblos_de(id).len() >= PUEBLOS_OBJETIVO {
            return;
        }
        let Some(&(nodo, _)) = self.n.candidatos_de_pueblo(id).iter().find(|(_, tipo)| *tipo == Tipo::Cantera) else { return };
        let r = self.orden(t, &gobernador(id), "MarcarPueblo", Comando::MarcarPueblo { asentamiento: id, nodo });
        let Some(&pueblo) = r.creadas.first().filter(|_| r.aceptada()) else { return };
        self.pjs.get_mut(&id).expect("existe").obra_pendiente = Some(pueblo);
        self.empezar(t, id, Mision { tipo: Clase::Pueblo, pueblo: Some(pueblo) });
    }

    /// Lo que falta entregar a la obra de un pueblo (vacío si ya no está pendiente).
    fn falta_del_pueblo(&self, id: u64, pueblo: u64) -> Vec<(Recurso, i64)> {
        self.n.pueblo(pueblo).filter(|&(plaza, _)| plaza == id).map(|(_, p)| p.falta()).unwrap_or_default()
    }

    /// Da a la columna el oro del viaje y la manda al campamento. El oro es andamiaje de administración (`DarSuministro`).
    fn empezar(&mut self, t: Instante, id: u64, m: Mision) {
        let (columna, heroe, camp) = (self.pjs[&id].info.columna, self.pjs[&id].info.heroe, self.pjs[&id].info.campamento);
        let oro = match m.tipo {
            Clase::Pueblo => {
                let falta = self.falta_del_pueblo(id, m.pueblo.expect("lleva pueblo"));
                let (de_oro, resto): (Vec<(Recurso, i64)>, Vec<(Recurso, i64)>) = falta.iter().partition(|(r, _)| *r == Recurso::Oro);
                let lleva = self.n.suministro(columna).get(&Recurso::Oro).copied().unwrap_or(0);
                (de_oro.iter().map(|x| x.1).sum::<i64>() + resto.iter().map(|x| x.1).sum::<i64>() * 3 / 2 + 10_000 - lleva).max(0)
            }
            Clase::Cupo => 0,
            Clase::Minerales => 400_000,
        };
        self.pjs.get_mut(&id).expect("existe").mision = Some(m);
        self.misiones.entry(m.tipo.nombre()).or_default().0 += 1;
        if oro > 0 {
            let r = self.admin(t, "DarSuministro(oro)", Comando::DarSuministro { ejercito: columna, recurso: Recurso::Oro, cantidad: oro });
            if r.aceptada() {
                self.oro_inyectado += oro;
            }
        }
        if m.tipo == Clase::Cupo {
            self.admin(t, "FijarResidencia", Comando::FijarResidencia { heroe, residencia: Some(Residencia::Campamento(camp)) });
            let r = self.admin(t, "DarAlmacenPersonal", Comando::DarAlmacenPersonal { heroe, recurso: Recurso::Oro, cantidad: 400_000 });
            if r.aceptada() {
                self.oro_inyectado += 400_000;
            }
        }
        let puerta = self.geo.campamentos[camp as usize];
        self.ir(t, id, puerta, Paso::Comprar);
    }

    /// Manda a la columna a un punto y programa el siguiente paso para su llegada. Si no hay ruta, la misión se abandona.
    fn ir(&mut self, t: Instante, id: u64, destino: Punto, luego: Paso) {
        let (columna, faccion) = (self.pjs[&id].info.columna, self.pjs[&id].info.faccion);
        let r = self.orden(t, &jugador(faccion), "MarcharA", Comando::MarcharA { ejercito: columna, destino });
        let llega = match r.resultado {
            Resultado::Aceptada => self.n.estado().ejercitos[&columna].plan.fin().unwrap_or(t),
            Resultado::SinCambios => t,
            _ => return self.terminar(t, id, false),
        };
        self.programar(llega.max(t), Accion::Mision { plaza: id, paso: luego });
    }

    fn terminar(&mut self, t: Instante, id: u64, bien: bool) {
        let Some(p) = self.pjs.get_mut(&id) else { return };
        let Some(m) = p.mision.take() else { return };
        let heroe = p.info.heroe;
        self.misiones.entry(m.tipo.nombre()).or_default().1 += bien as u32;
        if m.tipo == Clase::Pueblo {
            let p = self.pjs.get_mut(&id).expect("existe");
            p.viajes_obra += 1;
            if p.obra_pendiente.is_none() {
                self.viajes_por_obra.push(p.viajes_obra);
                p.viajes_obra = 0;
            } else {
                p.proxima_obra = t + REINTENTO_OBRA_H * HORA;
            }
        }
        if m.tipo == Clase::Cupo {
            self.admin(t, "FijarResidencia", Comando::FijarResidencia { heroe, residencia: None });
        }
    }

    fn mision(&mut self, t: Instante, id: u64, paso: Paso) {
        let Some(&Mision { tipo, pueblo, .. }) = self.pjs.get(&id).and_then(|p| p.mision.as_ref()) else { return };
        if !self.vive(id) {
            return self.terminar(t, id, false);
        }
        let (columna, heroe, camp, faccion, casa) = {
            let i = &self.pjs[&id].info;
            (i.columna, i.heroe, i.campamento, i.faccion, i.pos)
        };
        let (actor, cuenta) = (jugador(faccion), self.pjs[&id].info.cuenta.clone());
        match (tipo, paso) {
            (_, Paso::Comprar) => {
                self.orden(t, &cuenta, "RepostarEnCampamento", Comando::RepostarEnCampamento { heroe, campamento: camp });
                let compras: Vec<(Recurso, i64)> = match tipo {
                    Clase::Pueblo => self.falta_del_pueblo(id, pueblo.expect("lleva pueblo")).into_iter().filter(|(r, _)| *r != Recurso::Oro).collect(),
                    Clase::Cupo => vec![(Recurso::Madera, 30_000), (Recurso::Madera, 30_000), (Recurso::Madera, 30_000), (Recurso::Piedra, 6_000), (Recurso::Piedra, 6_000), (Recurso::Piedra, 6_000)],
                    Clase::Minerales => vec![(Recurso::Cobre, 15_000), (Recurso::Estano, 8_000)],
                };
                for (recurso, cantidad) in compras {
                    self.orden(t, &cuenta, "ComprarEnCampamento", Comando::ComprarEnCampamento { heroe, campamento: camp, recurso, cantidad });
                }
                match tipo {
                    Clase::Pueblo => {
                        let Some(pos) = self.n.posicion_de_pueblo(pueblo.expect("lleva pueblo")) else { return self.terminar(t, id, false) };
                        self.ir(t, id, pos, Paso::Entregar);
                    }
                    Clase::Cupo => self.ir(t, id, casa, Paso::Fin),
                    Clase::Minerales => self.ir(t, id, casa, Paso::Vender),
                }
            }
            (Clase::Pueblo, Paso::Entregar) => {
                let pueblo = pueblo.expect("lleva pueblo");
                let r = self.orden(t, &actor, "EntregarMateriales", Comando::EntregarMateriales { ejercito: columna, pueblo });
                let completa = r.aceptada() && self.falta_del_pueblo(id, pueblo).is_empty();
                if completa {
                    self.pjs.get_mut(&id).expect("existe").obra_pendiente = None;
                }
                self.ir(t, id, self.pjs[&id].info.pos, Paso::Fin);
            }
            (Clase::Minerales, Paso::Vender) => {
                for (recurso, cantidad) in [(Recurso::Cobre, 15_000), (Recurso::Estano, 8_000)] {
                    let r = self.orden(t, &gobernador(id), "ColocarOrdenMercado", Comando::ColocarOrdenMercado { asentamiento: id, venta: false, recurso, cantidad, precio: None });
                    if let Some(&orden) = r.creadas.first().filter(|_| r.aceptada()) {
                        self.orden(t, &actor, "ComerciarEnPlaza", Comando::ComerciarEnPlaza { ejercito: columna, orden, cantidad });
                    }
                }
                self.terminar(t, id, true);
            }
            (_, Paso::Fin) => self.terminar(t, id, true),
            _ => self.terminar(t, id, false),
        }
    }

    fn sondeo(&mut self, t: Instante, id: u64, tipo: Clase) {
        if !self.vive(id) {
            return;
        }
        if self.pjs[&id].mision.is_some() {
            return self.programar(t + HORA, Accion::Sondeo { plaza: id, tipo });
        }
        self.empezar(t, id, Mision { tipo, pueblo: None });
    }

    // ───────────────────────────── Medidas y comprobaciones ─────────────────────────────

    fn muestrear(&mut self, dia: i64) {
        let e = self.n.estado();
        let mut caravanas = [0u32; 6];
        for c in e.comercio.flota.iter().filter(|c| c.tipo == comercio::caravana::TipoCaravana::Comercial) {
            if let Some(i) = ESTADOS.iter().position(|(s, _)| *s == c.estado) {
                caravanas[i] += 1;
            }
        }
        let con = |f: &dyn Fn(&nucleo::Asentamiento) -> bool| e.asentamientos.values().filter(|a| f(a)).count();
        let pesos = self.n.pesos_de_red();
        for (a, &p) in &pesos {
            let pico = self.pico_red.entry(*a).or_insert(0);
            *pico = (*pico).max(p);
        }
        let m = Muestra {
            dia: dia as u32,
            reputacion: (1..=self.facciones.len() as u32).map(|f| self.n.reputacion(f)).collect(),
            aristas: self.n.red_de_caminos().aristas().count(),
            rutas: self.n.red_de_caminos().rutas().count(),
            caravanas,
            plazas_vivas: e.asentamientos.len(),
            con_mercado: con(&|a| a.economia.edificios.iter().any(|x| x.tipo == Tipo::Mercado && x.activo())),
            con_flota: con(&|a| self.n.flota_de(a.id).iter().any(|c| c.carros.iter().any(|k| k.animal.is_some()))),
            llenado: LLENADO.map(|r| {
                let v: Vec<i64> = e.asentamientos.values().map(|a| 100 * a.economia.almacen.cantidad(r) / a.economia.almacen.capacidad(r).max(1)).collect();
                v.iter().sum::<i64>() / v.len().max(1) as i64
            }),
        };
        self.muestras.push(m);
    }

    fn comprobar_periodico(&mut self) {
        self.violaciones.sumar(&auditar_cola(&self.n));
        self.violaciones.sumar(&crate::auditoria::auditar_contenedores(&self.n));
    }

    fn comprobar_final(&mut self) {
        self.comprobar_periodico();
    }

    /// Lo que hay de un recurso en todos los contenedores del mundo: almacenes, carros de las columnas, caravanas, almacenes personales y el carro que un héroe lleva
    /// sin columna (el de su residencia, Entrar con todo).
    fn en_el_mundo(&self, r: Recurso) -> i64 {
        let e = self.n.estado();
        e.asentamientos.values().map(|a| a.economia.almacen.disponible(r)).sum::<i64>()
            + e.ejercitos.values().map(|c| c.columna.carga.get(&r).copied().unwrap_or(0)).sum::<i64>()
            + e.comercio.flota.iter().map(|c| c.contenido.get(&r).copied().unwrap_or(0)).sum::<i64>()
            + self.n.heroes().map(|h| h.almacen_personal.get(&r).copied().unwrap_or(0) + h.carro.get(&r).copied().unwrap_or(0)).sum::<i64>()
    }

    /// Lo que hay de un mineral en el mundo contando lo ya fundido: la Fundición convierte el mineral en lingote sin tecnología (el cobre), y un
    /// lingote vale lo que consumió su receta.
    fn mineral_en_el_mundo(&self, mineral: Recurso) -> i64 {
        let lingote = match mineral {
            Recurso::Cobre => Recurso::LingoteCobre,
            Recurso::Estano => Recurso::LingoteEstano,
            _ => Recurso::LingoteHierro,
        };
        let recetas = &self.n.estado().reglas.economia.datos.catalogo.tipo(Tipo::Fundicion).niveles;
        let por_unidad = recetas.iter().flat_map(|n| &n.recetas).find(|r| r.produce == lingote).and_then(|r| r.consume_por_unidad.iter().find(|(x, _)| *x == mineral)).map_or(0, |&(_, n)| n);
        self.en_el_mundo(mineral) + self.en_el_mundo(lingote) * por_unidad / 1000
    }

    /// Por cada mineral: (recurso, lo que debe haber, lo que hay). Debe haber lo inicial más lo comprado a los mercenarios; ninguna otra
    /// economía de plaza lo produce ni, sin tecnologías, lo gasta más que fundiéndolo.
    fn balance_minerales(&self) -> Vec<(Recurso, i64, i64)> {
        self.minerales0.iter().map(|&(r, inicial)| (r, inicial + self.vigia.compras.get(&r.indice()).map_or(0, |c| c.unidades), self.mineral_en_el_mundo(r))).collect()
    }
}

/// Huella del estado final del núcleo.
pub fn huella(c: &Corrida) -> [u8; 32] {
    c.nucleo.hash_estado()
}
