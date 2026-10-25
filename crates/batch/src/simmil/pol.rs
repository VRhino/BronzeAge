//! El batch político (sistema 5, tanda 5I): la población de héroes bots del batch militar ([`super`]) con la política en manos de **bots de diplomacia** mínimos y
//! deterministas por semilla, que solo dan órdenes del contrato (la cuenta de su héroe; el Rey o el Embajador cuando la orden lo pide).
//!
//! # Lo que cambia frente al batch militar
//! - **Las Facciones nacen con `CrearFaccion`**: un héroe del campamento de su primera plaza la crea y es su Rey; se muda a esa plaza (`CambiarResidencia`).
//! - **Los cuatro héroes de cada plaza** piden el ingreso (`SolicitarIngreso`), el Rey los acepta (`ResponderSolicitud`) y se mudan (`CambiarResidencia`). El Rey nombra al Guardián
//!   Gobernador, y este al Intendente Tesorero, al Capitán General, a sí mismo Maestro de Obras y al Cazador Sacerdote; el Rey nombra Embajador al Intendente de su primera plaza.
//! - **No hay parejas aliadas de salida**: cada `DIPLOMACIA_H` horas el Rey (o, la mitad de las veces que la orden lo deja, el Embajador) de cada Facción contesta las solicitudes
//!   y las propuestas que le llegan, firma la paz, se rebela si es vasalla y más fuerte que su señora, designa capital cuando una plaza suya tiene Palacio, y toma como mucho una
//!   iniciativa: declarar la guerra (las belicosas, contra quien tiene un título o la vecina más cercana), proponer alianza, vasallaje (con tributo) o fusión, anexionarse a una
//!   Facción sin plazas o a una vasalla pequeña (las ambiciosas), romper una alianza, liberar a una vasalla, traicionar a una aliada (las traicioneras: sus Capitanes la atacan con
//!   `declara_guerra`, Q2), pasar la corona o cambiar la admisión de otras Facciones en sus ataques.
//! - **Los Capitanes y Cazadores** solo atacan a quien está en guerra con su Facción (o a la aliada que van a traicionar); quien conquista se muda con `CambiarResidencia`.
//! - **El Gobernador** pide además el Palacio (nivel 2) y la Gran Fundición (en la capital, con la Facción en el nivel que la pide), repone los cargos vacíos con residentes,
//!   abre o cierra la puerta a los neutrales, veta y levanta vetos, y abre el almacén a las aliadas.
//! - **Colonos**: cada `COLONOS_CADA_H` horas nace un héroe que pide entrar en dos Facciones con plazas; admitido, se muda a la plaza suya con menos residentes. Los desalojados de
//!   una plaza perdida también se mudan a otra de su Facción y vuelven a casa.
//! - **Secesión**: una vez al día, con el `SECESION_PCT` %, un Cazador sin cargos de una Facción grande la deja (`DejarFaccion`); pasado el cooldown de creación crea la suya
//!   (`CrearFaccion`), que nace sin plazas.
//! - **Refundación**: una Facción sin plazas (derrotada o recién creada) junta el fondo en el campamento donde reside su Rey (`AportarARefundacion` del almacén personal, y
//!   `ComprarEnCampamento` con su oro lo que falte), compra la Caravana de Refundación, la engancha a un ejército de uno de los suyos (`SalirDelCampamento`, `AdjuntarCaravana`),
//!   marcha a un sitio y funda; los suyos que residen en campamentos se mudan a la plaza nueva.
//!
//! # Andamiaje de administración que queda
//! El nivel de partida de cada Facción (`FijarNivelFaccion` a `NIVEL_INICIAL`: «Facciones con historia», luego sube sola con la experiencia), la colonia de cada plaza
//! (`crear_ejercito` + `DarCaravanaFundacion`, como en el militar) y la colocación de cada héroe dentro de su plaza al nacer (`ColocarHeroe`). Decisión 9 del plan del sistema 5.
//! Las tecnologías van por la vía de juego desde 6H (el Rey adopta en su capital lo que aparece: `Sim::adoptar`).
//!
//! # Invariantes (todos a 0)
//! Ciudadanía única y la misma en el héroe y en la Facción; Rey (y Embajador) entre los ciudadanos; cargos locales de residentes ciudadanos de la dueña; una relación activa por
//! par y ninguna vasalla con dos señoras; arrastre de cada guerra (el de `guerra.confirmar`) y en guerra con todo el bando; el tributo de cada plaza vasalla (pagado ≥ recibido,
//! de un vasallaje con tributo, sin pasar de lo pactado); la reputación solo se mueve con sus sucesos o sus alianzas; la anexión y la fusión no pierden ni duplican recursos,
//! tropas, héroes, ciudadanos, plazas ni caravanas; ningún id de Facción se reutiliza; la experiencia de una Facción no baja.

use super::*;
use asentamiento::edificio::Edificios;
use comercio::reputacion::{Reputacion, Suceso};
use nucleo::{Dato, LadoDelFondo, SituacionFundacion};
use politica::propuestas::{Clase, Pacto};
use politica::relaciones::TipoRelacion;
use politica::tipos::{Grupo, Grupos, CARGOS};

// ── Supuestos del batch político (hipótesis sintéticas, se anotan en `parametros` del informe) ──
/// Nivel de partida de cada Facción (cap de fundación 3: sus tres plazas; cupo de nivel 2: una).
pub const NIVEL_INICIAL: u32 = 3;
pub const DIPLOMACIA_H: i64 = 4;
pub const COLONOS_CADA_H: i64 = 12;
/// El Rey admite este porcentaje de las solicitudes de ingreso.
pub const ADMITE_PCT: u64 = 80;
pub const TRAICIONERA_PCT: u64 = 20;
pub const AMBICIOSA_PCT: u64 = 35;
/// Una guerra no se cierra con una paz propuesta antes de estas horas (la contestación, sí).
pub const PAZ_TRAS_H: i64 = 48;
/// El tributo que pide la señora: milésimas de trigo por minuto, repartidas entre las plazas de la vasalla.
pub const TRIBUTO_POR_MINUTO: i64 = 500;
pub const SECESION_PCT: u64 = 30;
pub const SECESION_MIN_CIUDADANOS: usize = 9;
pub const INTENTOS_DE_FUNDAR: u32 = 3;
/// Las aliadas se proponen a una de las vecinas más cercanas; la traición dura estos días (los Capitanes atacan a la aliada si la tienen a tiro).
pub const VECINAS: usize = 4;
pub const TRAICION_DIAS: i64 = 3;
/// Lo que se tolera (ppm de punto) entre la reputación que sigue el ritmo de la observación anterior y la de ahora sin ningún suceso: el redondeo de los reanclajes.
const TOLERANCIA_REPUTACION: i64 = 10;

/// Lo que se puede variar del escenario político sin tocar ninguna regla.
#[derive(Clone, Copy, Debug)]
pub struct Ajustes {
    /// `false`: experimento del riesgo R1 (ninguna plaza llega a los residentes que pide la nobleza).
    pub nobleza: bool,
}

impl Default for Ajustes {
    fn default() -> Self {
        Self { nobleza: true }
    }
}

/// Los invariantes de la política (todos deben quedar a 0).
#[derive(Clone, Copy, Debug, Default)]
pub struct ViolacionesPoliticas {
    /// Un héroe en dos Facciones, o cuya Facción no es la que dice la Facción.
    pub ciudadania: u64,
    /// Una Facción con ciudadanos sin Rey, o con un Rey o un Embajador que no es su ciudadano.
    pub rey: u64,
    /// Un cargo local de quien no reside en la plaza o no es ciudadano de su dueña.
    pub cargos: u64,
    /// Dos relaciones activas en un par, o una vasalla con dos señoras.
    pub relaciones: u64,
    /// Una guerra sin el arrastre de `guerra.confirmar`, o con alguien del bando que no quedó en guerra.
    pub arrastre: u64,
    /// Un pago de tributo incoherente (recibe más de lo que paga, sin vasallaje con tributo, o más de lo pactado).
    pub tributo: u64,
    /// Una reputación que se movió sin suceso ni alianza, o un suceso mayor que cualquiera de los del bloque.
    pub reputacion: u64,
    /// Una anexión o una fusión que pierde o duplica algo.
    pub traslado: u64,
    /// El id de una Facción desaparecida que vuelve a aparecer.
    pub ids: u64,
    /// La experiencia de una Facción que baja.
    pub experiencia: u64,
}

/// Lo que se cuenta de la política a lo largo de la corrida (lo demás está en `Medidas::hechos` y en las cuentas de órdenes).
#[derive(Clone, Debug, Default)]
pub struct MedidasPoliticas {
    pub alianzas: u64,
    pub vasallajes: u64,
    pub alianzas_rotas: u64,
    pub vasallajes_rotos: u64,
    pub traiciones: u64,
    pub traiciones_planeadas: u64,
    pub arrastradas: u64,
    /// Unidades de tributo pagadas y recibidas, y pagos.
    pub tributo_pagado: i64,
    pub tributo_recibido: i64,
    pub tributos: u64,
    tributo_por_relacion: BTreeMap<u64, i64>,
    tributo_excedido: BTreeSet<u64>,
    /// (instante, absorbente, absorbida, plazas trasladadas).
    pub anexiones: Vec<(Instante, u32, u32, usize)>,
    /// (instante, A, B, C, plazas).
    pub fusiones: Vec<(Instante, u32, u32, u32, usize)>,
    /// (instante, Facción, por quién: 0 = colapso).
    pub derrotas: Vec<(Instante, u32, u32)>,
    /// (instante, Facción, plaza, cuánto estuvo sin plazas).
    pub refundaciones: Vec<(Instante, u32, u64, Instante)>,
    sin_plazas_desde: BTreeMap<u32, Instante>,
    /// Título → (nace, cambia de manos).
    pub titulos: BTreeMap<String, (u64, u64)>,
    /// (instante, plaza, distancia a su capital en la última observación, milésimas).
    pub ruinas: Vec<(Instante, u64, i64)>,
    pub secesiones: u64,
    pub colonos: u64,
    pub tareas: u64,
    pub tareas_fallidas: u64,
    pub sin_fondo: u64,
    /// Salidas a cazar bandidos del anillo para el fondo, y el oro que se aportó desde el carro al volver.
    pub cazas: u64,
    pub oro_de_caza: i64,
    pub rep_cambios: u64,
}

/// Una foto diaria de la política.
#[derive(Clone, Debug, Default)]
pub struct MuestraPolitica {
    pub dia: u32,
    pub facciones: usize,
    pub con_plazas: usize,
    pub niveles: Vec<u32>,
    /// Reputación de cada Facción (ppm de punto).
    pub reputaciones: Vec<i64>,
    pub alianzas: usize,
    pub vasallajes: usize,
    pub guerras: usize,
    pub nobles: i64,
    pub con_nobleza: usize,
    pub con_palacio: usize,
    /// El multiplicador del mantenimiento por la distancia (ppm) de cada plaza que no es su capital.
    pub multiplicadores: Vec<i64>,
    pub residentes: usize,
    pub poblacion: i64,
    pub plazas: usize,
}

/// Una refundación en marcha: la columna de `titular` lleva la Caravana de Refundación a un sitio o, con `caza`, sale a por el oro de un campamento de bandidos del anillo de
/// su campamento (Cal-5: solo lo cobran los héroes sin plaza) para el fondo, y vuelve a aportarlo desde el carro.
#[derive(Clone, Debug)]
pub struct Tarea {
    pub faccion: u32,
    pub titular: u64,
    pub columna: u64,
    pub caravana: u64,
    pub intentos: u32,
    pub viva: bool,
    pub caza: Option<u64>,
    pub campamento: u32,
    /// En la caza: 0 = va hacia los bandidos, 1 = vuelve a la puerta del campamento.
    pub fase: u8,
}

/// Lo que se suma antes y después de una anexión o una fusión: tiene que ser lo mismo.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Totales {
    heroes: usize,
    escuadras: usize,
    soldados: i64,
    plazas: usize,
    caravanas: usize,
    recursos: BTreeMap<Recurso, i64>,
}

pub struct Pol {
    pub ajustes: Ajustes,
    pub traicioneras: BTreeSet<u32>,
    pub ambiciosas: BTreeSet<u32>,
    /// Facción → (la aliada que sus Capitanes atacarán con `declara_guerra`, hasta cuándo).
    traicion: BTreeMap<u32, (u32, Instante)>,
    /// Bot → hasta cuándo está apartado de su turno militar por una tarea política.
    ocupados: BTreeMap<usize, Instante>,
    reyes_iniciales: BTreeMap<u32, u64>,
    pub colonos: Vec<u64>,
    /// Héroe → cuándo dejó su Facción para crear la suya.
    disidentes: BTreeMap<u64, Instante>,
    pub tareas: Vec<Tarea>,
    pub v: ViolacionesPoliticas,
    pub m: MedidasPoliticas,
    pub muestras: Vec<MuestraPolitica>,
    /// La reputación de cada Facción en la observación anterior, y cuándo fue.
    reputacion: BTreeMap<u32, Reputacion>,
    rep_desde: Instante,
    /// Las Facciones cuya reputación pudo moverse desde la observación anterior (un suceso o una relación suya); con una anexión o una fusión, todas.
    rep_tocadas: BTreeSet<u32>,
    rep_todas: bool,
    experiencia: BTreeMap<u32, i64>,
    /// Las Facciones que han existido y las que desaparecieron.
    pub vistas: BTreeSet<u32>,
    pub desaparecidas: BTreeSet<u32>,
    /// Plaza → su distancia a la capital (milésimas) en la última observación.
    distancia: BTreeMap<u64, i64>,
}

impl Pol {
    pub fn nuevo(ajustes: Ajustes) -> Self {
        Pol {
            ajustes,
            traicioneras: BTreeSet::new(),
            ambiciosas: BTreeSet::new(),
            traicion: BTreeMap::new(),
            ocupados: BTreeMap::new(),
            reyes_iniciales: BTreeMap::new(),
            colonos: Vec::new(),
            disidentes: BTreeMap::new(),
            tareas: Vec::new(),
            v: ViolacionesPoliticas::default(),
            m: MedidasPoliticas::default(),
            muestras: Vec::new(),
            reputacion: BTreeMap::new(),
            rep_desde: 0,
            rep_tocadas: BTreeSet::new(),
            rep_todas: false,
            experiencia: BTreeMap::new(),
            vistas: BTreeSet::new(),
            desaparecidas: BTreeSet::new(),
            distancia: BTreeMap::new(),
        }
    }

    /// ¿Atacan los Capitanes de `yo` a su aliada `otra` (la traición planeada)?
    pub fn traiciona(&self, yo: u32, otra: u32, t: Instante) -> bool {
        self.traicion.get(&yo).is_some_and(|&(g, hasta)| g == otra && t < hasta)
    }

    pub fn ocupado(&self, bot: usize, t: Instante) -> bool {
        self.ocupados.get(&bot).is_some_and(|&hasta| t < hasta)
    }
}

fn dato(datos: &[(&'static str, i64)], k: &str) -> Option<i64> {
    datos.iter().find(|(c, _)| *c == k).map(|&(_, v)| v)
}

/// Las Facciones de una lista de un hecho (`f-N`).
fn facciones_de(valores: &[(&'static str, Dato)], k: &str) -> Vec<u32> {
    match valores.iter().find(|(c, _)| *c == k) {
        Some((_, Dato::Lista(l))) => l.iter().filter_map(|d| if let Dato::Texto(s) = d { s.strip_prefix("f-")?.parse().ok() } else { None }).collect(),
        _ => vec![],
    }
}

fn sumar<'a>(r: &mut BTreeMap<Recurso, i64>, m: impl IntoIterator<Item = (&'a Recurso, &'a i64)>) {
    for (&k, &q) in m {
        *r.entry(k).or_insert(0) += q;
    }
}

enum Movimiento {
    Nada,
    Guerra(u32),
    Alianza(u32),
    Vasallaje(u32),
    Anexion(u32),
    Fusion(u32),
    Romper(u64),
    Liberar(u64),
    Traicion(u32),
    Corona,
    Admitir,
}

impl Sim<'_> {
    fn p(&mut self) -> &mut Pol {
        self.pol.as_mut().expect("batch político")
    }

    fn cuenta_de(&self, h: u64) -> String {
        self.n.heroe(h).map_or_else(String::new, |x| x.cuenta.clone())
    }

    fn rey_de(&self, f: u32) -> Option<(u64, String)> {
        let rey = self.n.faccion(f)?.rey?;
        Some((rey, self.cuenta_de(rey)))
    }

    /// Quien habla por la Facción esta vez: la mitad de las veces su Embajador (si lo tiene), si no su Rey.
    fn autoridad(&mut self, f: u32) -> Option<(u64, String)> {
        let x = self.n.faccion(f)?;
        let (rey, embajador) = (x.rey?, x.embajador);
        let h = match embajador {
            Some(e) if self.rng.por_debajo(2) == 0 => e,
            _ => rey,
        };
        Some((h, self.cuenta_de(h)))
    }

    fn plazas_n(&self, f: u32) -> usize {
        self.n.plazas_de(f).count()
    }

    /// Los soldados de todos sus ciudadanos, estén donde estén (lo que mira el título «Ejército más grande»).
    fn poder_de(&self, f: u32) -> i64 {
        self.n.faccion(f).map_or(0, |x| x.ciudadanos.iter().filter_map(|&h| self.n.heroe(h)).flat_map(|h| h.heroe.escuadras.iter()).map(|s| s.cantidad).sum())
    }

    fn bot_de(&self, h: u64) -> Option<usize> {
        self.bots.iter().position(|b| b.heroe == h)
    }

    // ───────────────────────────── El nacimiento ─────────────────────────────

    /// Las Facciones nacen con la orden real: un héroe del campamento de su primera plaza da `CrearFaccion` y es su Rey. Luego, el andamiaje: el nivel de partida.
    pub(super) fn pol_crear_facciones(&mut self) {
        for k in 1..=self.facciones {
            let campamento = (k as usize % self.geo.campamentos.len()) as u32;
            let cuenta = format!("{}-rey", jugador(k));
            let r = self.segura(0, &cuenta, "CrearHeroe", Comando::CrearHeroe { cuenta: cuenta.clone(), campamento, controlador: Controlador::Humano });
            let Some(&rey) = r.creadas.first().filter(|_| r.aceptada()) else { continue };
            let r = self.segura(0, &cuenta, "CrearFaccion", Comando::CrearFaccion { heroe: rey, nombre: format!("Casa {k}"), sigilo: None });
            // La Facción nueva toma el id siguiente al mayor del mundo: la k-ésima es la `f{k}` del reparto de plazas.
            if r.creadas.first() != Some(&u64::from(k)) {
                continue;
            }
            self.admin(0, "FijarNivelFaccion", Comando::FijarNivelFaccion { faccion: k, nivel: NIVEL_INICIAL });
            if self.rng.por_debajo(100) < BELICOSA_PCT {
                self.belicosas.insert(k);
            }
            let (traicionera, ambiciosa) = (self.rng.por_debajo(100) < TRAICIONERA_PCT, self.rng.por_debajo(100) < AMBICIOSA_PCT);
            let p = self.p();
            if traicionera {
                p.traicioneras.insert(k);
            }
            if ambiciosa {
                p.ambiciosas.insert(k);
            }
            p.reyes_iniciales.insert(k, rey);
        }
    }

    /// El Rey se muda a la primera plaza de su Facción y entra en ella.
    pub(super) fn pol_rey_en_su_plaza(&mut self, t: Instante, f: u32, plaza: u64) {
        let Some(&rey) = self.p().reyes_iniciales.get(&f) else { return };
        let cuenta = self.cuenta_de(rey);
        self.segura(t, &cuenta, "CambiarResidencia", Comando::CambiarResidencia { heroe: rey, asentamiento: plaza });
        self.admin(t, "ColocarHeroe", Comando::ColocarHeroe { heroe: rey, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
    }

    /// Un héroe recién nacido pide el ingreso, el Rey le acepta y se muda a su plaza; el andamiaje le coloca dentro.
    pub(super) fn pol_ingresar(&mut self, t: Instante, f: u32, heroe: u64, cuenta: &str, plaza: u64) {
        self.segura(t, cuenta, "SolicitarIngreso", Comando::SolicitarIngreso { heroe, faccion: f });
        if let Some((rey, rc)) = self.rey_de(f) {
            self.segura(t, &rc, "ResponderSolicitud", Comando::ResponderSolicitud { heroe: rey, faccion: f, solicitante: heroe, aceptar: true });
        }
        self.segura(t, cuenta, "CambiarResidencia", Comando::CambiarResidencia { heroe, asentamiento: plaza });
        self.admin(t, "ColocarHeroe", Comando::ColocarHeroe { heroe, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
    }

    /// El Gobernador (el Guardián) reparte los otros cuatro cargos de la plaza; el Rey nombra Embajador al Intendente de su primera plaza.
    pub(super) fn pol_cargos_iniciales(&mut self, t: Instante, plaza: u64) {
        let de = |s: &Self, rol: Rol| s.bots.iter().rposition(|b| b.plaza == plaza && b.rol == rol);
        let Some(g) = de(self, Rol::Guardian) else { return };
        let gobernador = self.bots[g].heroe;
        if self.n.estado().asentamientos[&plaza].cargos.titular(Cargo::Gobernador) != Some(gobernador) {
            return;
        }
        for (cargo, rol) in [(Cargo::Tesorero, Rol::Intendente), (Cargo::General, Rol::Capitan), (Cargo::MaestroObras, Rol::Guardian), (Cargo::Sacerdote, Rol::Cazador)] {
            if let Some(j) = de(self, rol) {
                let designado = self.bots[j].heroe;
                self.esperada(t, g, "AsignarCargoLocal", Comando::AsignarCargoLocal { heroe: gobernador, asentamiento: plaza, cargo, designado });
            }
        }
        let f = self.n.estado().asentamientos[&plaza].faccion;
        let sin_embajador = self.n.faccion(f).is_some_and(|x| x.embajador.is_none());
        if let (true, Some((rey, rc)), Some(j)) = (sin_embajador, self.rey_de(f), de(self, Rol::Intendente)) {
            let designado = self.bots[j].heroe;
            self.segura(t, &rc, "AsignarEmbajador", Comando::AsignarEmbajador { heroe: rey, faccion: f, designado });
        }
    }

    pub(super) fn pol_arrancar(&mut self, t0: Instante) {
        let facciones: Vec<u32> = self.n.facciones().map(|f| f.id).collect();
        for (k, f) in facciones.into_iter().enumerate() {
            self.programar(t0 + HORA + (k as i64 % 24) * 10 * MIN, Accion::Diplomacia { faccion: f });
        }
        self.programar(t0 + COLONOS_CADA_H * HORA, Accion::Colonos);
    }

    // ───────────────────────────── El turno de diplomacia ─────────────────────────────

    pub(super) fn pol_diplomacia(&mut self, t: Instante, f: u32) {
        // Una Facción que desapareció (absorbida o fundida) no vuelve a tener turno.
        if !self.n.estado().politica.facciones.existe(f) {
            return;
        }
        self.programar(t + DIPLOMACIA_H * HORA, Accion::Diplomacia { faccion: f });
        self.p().traicion.retain(|_, &mut (_, hasta)| hasta > t);
        if self.rey_de(f).is_none() {
            return;
        }
        self.pol_solicitudes(t, f);
        self.pol_responder(t, f);
        if !self.n.estado().politica.facciones.existe(f) || self.rey_de(f).is_none() {
            return;
        }
        if self.plazas_n(f) == 0 {
            self.pol_refundar(t, f);
        }
        self.pol_paz_y_rebelion(t, f);
        self.pol_capital_y_embajador(t, f);
        self.pol_iniciativa(t, f);
    }

    fn pol_solicitudes(&mut self, t: Instante, f: u32) {
        let Some((rey, rc)) = self.rey_de(f) else { return };
        let solicitudes = self.n.faccion(f).map(|x| x.solicitudes.clone()).unwrap_or_default();
        for solicitante in solicitudes {
            let aceptar = self.rng.por_debajo(100) < ADMITE_PCT;
            let c = Comando::ResponderSolicitud { heroe: rey, faccion: f, solicitante, aceptar };
            // Denegar se puede siempre; aceptar a quien va en un ejército, no (5B-6).
            if aceptar {
                self.orden(t, &rc, "AceptarSolicitud", c);
            } else {
                self.segura(t, &rc, "DenegarSolicitud", c);
            }
        }
    }

    /// Las propuestas vigentes que le llegan: las contesta su Rey.
    fn pol_responder(&mut self, t: Instante, f: u32) {
        let pendientes: Vec<(u64, Clase)> = self.n.estado().politica.propuestas.iter().filter(|p| p.vigente(t) && p.clase.par().1 == f).map(|p| (p.id, p.clase.clone())).collect();
        for (id, clase) in pendientes {
            let Some((rey, rc)) = self.rey_de(f) else { return };
            if self.n.estado().politica.propuestas.get(id).is_none() {
                continue;
            }
            let (a, mias) = (clase.par().0, self.plazas_n(f));
            let pct = match &clase {
                Clase::Relacion { pacto: Pacto::Alianza, .. } => 70,
                Clase::Relacion { pacto: Pacto::Vasallaje, .. } if mias == 0 || mias < self.plazas_n(a) && self.poder_de(f) < self.poder_de(a) => 75,
                Clase::Relacion { .. } => 10,
                // Sin plazas prefiere refundar: acepta una de cada cuatro.
                Clase::Anexion { .. } if mias == 0 => 25,
                Clase::Anexion { absorbente, .. } if self.n.estado().politica.relaciones.senora_de(f) == Some(*absorbente) => 50,
                Clase::Anexion { .. } if mias < self.plazas_n(a) => 20,
                Clase::Anexion { .. } => 3,
                Clase::Fusion { .. } => 30,
            };
            let aceptar = self.rng.por_debajo(100) < pct;
            let (etiqueta, c) = match &clase {
                Clase::Relacion { .. } => ("ResponderRelacion", Comando::ResponderRelacion { heroe: rey, propuesta: id, aceptar }),
                Clase::Anexion { .. } => ("ResponderAnexion", Comando::ResponderAnexion { heroe: rey, propuesta: id, aceptar }),
                Clase::Fusion { .. } => ("ResponderFusion", Comando::ResponderFusion { heroe: rey, propuesta: id, aceptar }),
            };
            // Rechazar una propuesta vigente se puede siempre; aceptarla revalida todo (y una batalla abierta lo impide).
            if !aceptar {
                self.segura(t, &rc, etiqueta, c);
            } else if matches!(clase, Clase::Relacion { .. }) {
                self.orden(t, &rc, etiqueta, c);
            } else {
                self.pol_union(t, &rc, etiqueta, c, &clase);
            }
            if !self.n.estado().politica.facciones.existe(f) {
                return;
            }
        }
    }

    /// Lo que se suma de todo el mundo: recursos de plazas, pueblos, almacenes personales, carros, columnas, flota, Caravanas de Fundación y fondos; héroes, escuadras, soldados,
    /// plazas y caravanas.
    fn totales(&self) -> Totales {
        let e = self.n.estado();
        let mut recursos = BTreeMap::new();
        for a in e.asentamientos.values() {
            let de = |alm: &asentamiento::almacen::Almacen| asentamiento::recurso::TODOS.iter().map(|&r| (r, alm.cantidad(r))).collect::<BTreeMap<_, _>>();
            sumar(&mut recursos, &de(&a.economia.almacen));
            for p in &a.economia.pueblos {
                sumar(&mut recursos, &de(&p.almacen));
            }
        }
        let (mut escuadras, mut soldados) = (0, 0);
        for h in self.n.heroes() {
            sumar(&mut recursos, &h.almacen_personal);
            sumar(&mut recursos, &h.carro);
            escuadras += h.heroe.escuadras.len();
            soldados += h.heroe.escuadras.iter().map(|s| s.cantidad).sum::<i64>();
        }
        for c in e.ejercitos.values() {
            sumar(&mut recursos, &c.columna.carga);
        }
        for k in &e.comercio.flota {
            sumar(&mut recursos, &k.contenido);
        }
        for k in e.fundacion.caravanas.values() {
            sumar(&mut recursos, &k.contenido);
        }
        for f in e.fundacion.fondos.values() {
            for m in f.values() {
                sumar(&mut recursos, m);
            }
        }
        recursos.retain(|_, q| *q != 0);
        Totales { heroes: self.n.heroes().count(), escuadras, soldados, plazas: e.asentamientos.len(), caravanas: e.comercio.flota.len() + e.fundacion.caravanas.len(), recursos }
    }

    /// Acepta una anexión o una fusión y comprueba que el traslado no pierde ni duplica nada: los totales del mundo, los ciudadanos y las plazas de la que queda (la unión de los de
    /// las dos), la Facción de cada ciudadano y que la que desaparece ya no exista.
    fn pol_union(&mut self, t: Instante, rc: &str, etiqueta: &'static str, c: Comando, clase: &Clase) {
        let (a, b) = clase.par();
        self.n.avanzar_hasta(t);
        let ciudadanos = |s: &Self, f: u32| s.n.faccion(f).map_or_else(BTreeSet::new, |x| x.ciudadanos.iter().copied().collect::<BTreeSet<u64>>());
        let plazas = |s: &Self, f: u32| s.n.plazas_de(f).map(|x| x.id).collect::<BTreeSet<u64>>();
        let antes = self.totales();
        let (ca, cb, pa, pb) = (ciudadanos(self, a), ciudadanos(self, b), plazas(self, a), plazas(self, b));
        let r = self.orden(t, rc, etiqueta, c);
        if !r.aceptada() {
            return;
        }
        let fusion = matches!(clase, Clase::Fusion { .. });
        let destino = if fusion { r.creadas.first().map_or(0, |&x| x as u32) } else { a };
        let despues = self.totales();
        let (cd, pd) = (ciudadanos(self, destino), plazas(self, destino));
        let (union_c, union_p): (BTreeSet<u64>, BTreeSet<u64>) = (ca.union(&cb).copied().collect(), pa.union(&pb).copied().collect());
        let mut malas = (antes != despues) as u64 + (cd != union_c) as u64 + (pd != union_p) as u64;
        malas += cd.iter().filter(|&&h| self.n.heroe(h).and_then(|x| x.faccion) != Some(destino)).count() as u64;
        let fuera: Vec<u32> = if fusion { vec![a, b] } else { vec![b] };
        malas += fuera.iter().filter(|&&f| self.n.estado().politica.facciones.existe(f)).count() as u64;
        let reusado = fusion && self.p().vistas.contains(&destino);
        let p = self.p();
        p.v.traslado += malas;
        p.v.ids += reusado as u64;
        if fusion {
            p.m.fusiones.push((t, a, b, destino, union_p.len()));
        } else {
            p.m.anexiones.push((t, a, b, pb.len()));
        }
    }

    /// La paz (propone pasadas `PAZ_TRAS_H` horas; si la propuso la otra, firma) y la rebelión de la vasalla más fuerte que su señora.
    fn pol_paz_y_rebelion(&mut self, t: Instante, f: u32) {
        let guerras: Vec<(u64, u32, Instante, Option<u32>)> =
            self.n.estado().politica.relaciones.activas().filter(|r| r.tipo == TipoRelacion::Guerra && r.incluye(f)).map(|r| (r.id, r.otra(f), r.creada_en, r.paz_propuesta_por)).collect();
        for (relacion, otra, desde, paz) in guerras {
            let pct = match paz {
                Some(o) if o == otra => 85,
                None if t >= desde + PAZ_TRAS_H * HORA => 25,
                _ => 0,
            };
            if pct > 0 && self.rng.por_debajo(100) < pct {
                let Some((heroe, cuenta)) = self.autoridad(f) else { return };
                self.segura(t, &cuenta, "ProponerPaz", Comando::ProponerPaz { heroe, relacion, faccion: f });
            }
        }
        let Some((relacion, senora)) = self.n.estado().politica.relaciones.vasallaje_de(f).map(|r| (r.id, r.a)) else { return };
        let pct = if self.poder_de(f) > self.poder_de(senora) { 30 } else { 2 };
        if self.rng.por_debajo(100) < pct {
            let Some((heroe, cuenta)) = self.autoridad(f) else { return };
            let version = self.n.version();
            if self.orden(t, &cuenta, "RebelionVasallo", Comando::RebelionVasallo { heroe, relacion }).aceptada() {
                self.pol_tras_guerra_obligada(version, None);
            }
        }
    }

    /// La capital (la plaza con Palacio activo, si ninguna lleva la marca y no hay cooldown) y el Embajador que falte.
    fn pol_capital_y_embajador(&mut self, t: Instante, f: u32) {
        let Some((rey, rc)) = self.rey_de(f) else { return };
        let cooldown = self.n.estado().reglas.economia.politica.cooldown_capital;
        let Some(x) = self.n.faccion(f) else { return };
        let (designada_en, embajador, otro) = (x.capital_designada_en, x.embajador, x.ciudadanos.iter().copied().find(|&h| h != rey));
        let marcada = self.n.plazas_de(f).any(|a| a.capital_de == Some(f));
        // La que ya es la capital efectiva (la más antigua de la Facción) no hace falta designarla (`capital.ya_es_la_capital`).
        let efectiva = self.n.capital_de(f);
        let con_palacio = self.n.plazas_de(f).find(|a| Some(a.id) != efectiva && a.economia.edificios.iter().any(|e| e.tipo == Tipo::Palacio && e.activo())).map(|a| a.id);
        if let Some(p) = con_palacio.filter(|_| !marcada && designada_en.is_none_or(|d| t >= d + cooldown)) {
            self.segura(t, &rc, "DesignarCapital", Comando::DesignarCapital { heroe: rey, faccion: f, asentamiento: p });
        }
        if let (None, Some(designado)) = (embajador, otro) {
            self.segura(t, &rc, "AsignarEmbajador", Comando::AsignarEmbajador { heroe: rey, faccion: f, designado });
        }
    }

    /// El blanco de una guerra: la mitad de las veces una Facción con un título (si no tiene relación con ella), si no la más cercana con plazas.
    fn objetivo_de_guerra(&mut self, f: u32, libres: &[u32]) -> Option<u32> {
        let con_plazas: Vec<u32> = libres.iter().copied().filter(|&g| self.plazas_n(g) > 0).collect();
        let titulada = self.n.titulos().iter().map(|x| x.poseedor).find(|g| con_plazas.contains(g));
        if let Some(g) = titulada.filter(|_| self.rng.por_debajo(2) == 0) {
            return Some(g);
        }
        // La mitad de las veces, una que está en un vasallaje (la guerra arrastra a su bando: §5.4).
        let rels = &self.n.estado().politica.relaciones;
        let en_vasallaje = con_plazas.iter().copied().find(|&g| rels.senora_de(g).is_some() || !rels.vasallas_de(g).is_empty());
        if let Some(g) = en_vasallaje.filter(|_| self.rng.por_debajo(2) == 0) {
            return Some(g);
        }
        let casa = self.n.plazas_de(f).next()?.posicion;
        con_plazas.into_iter().min_by_key(|&g| (self.n.plazas_de(g).map(|a| a.posicion.dist(casa)).min().unwrap_or(i64::MAX), g))
    }

    /// Como mucho una iniciativa por turno, sorteada con pesos entre las que tiene a mano.
    fn pol_iniciativa(&mut self, t: Instante, f: u32) {
        let rels = &self.n.estado().politica.relaciones;
        let libres: Vec<u32> = self.n.facciones().map(|x| x.id).filter(|&g| g != f && rels.activa_entre(f, g).is_none() && self.n.faccion(g).is_some_and(|x| x.rey.is_some())).collect();
        let aliadas: Vec<(u64, u32)> = rels.activas().filter(|r| r.tipo == TipoRelacion::Alianza && r.incluye(f)).map(|r| (r.id, r.otra(f))).collect();
        let vasallas: Vec<(u64, u32)> = rels.activas().filter(|r| r.tipo == TipoRelacion::Vasallaje && r.a == f).map(|r| (r.id, r.b)).collect();
        let en_guerra = rels.activas().any(|r| r.tipo == TipoRelacion::Guerra && r.incluye(f));
        let mias = self.plazas_n(f);
        let sin_plazas: Vec<u32> = libres.iter().copied().filter(|&g| self.plazas_n(g) == 0).collect();
        let debiles: Vec<u32> = libres.iter().copied().filter(|&g| (1..mias).contains(&self.plazas_n(g)) && rels.senora_de(g).is_none()).collect();
        let pol = self.pol.as_ref().expect("batch político");
        let (belicosa, ambiciosa, traicionera) = (self.belicosas.contains(&f), pol.ambiciosas.contains(&f), pol.traicioneras.contains(&f));
        let mut opciones: Vec<(u64, Movimiento)> = vec![(40, Movimiento::Nada), (1, Movimiento::Corona), (2, Movimiento::Admitir)];
        if belicosa && !en_guerra && mias > 0 {
            if let Some(g) = self.objetivo_de_guerra(f, &libres) {
                opciones.push((20, Movimiento::Guerra(g)));
            }
        }
        // Las aliadas se buscan entre las cuatro vecinas más cercanas con plazas.
        let casa = self.n.plazas_de(f).next().map(|a| a.posicion);
        let mut con_plazas: Vec<u32> = libres.iter().copied().filter(|&g| self.plazas_n(g) > 0).collect();
        if let Some(casa) = casa {
            con_plazas.sort_by_key(|&g| (self.n.plazas_de(g).map(|a| a.posicion.dist(casa)).min().unwrap_or(i64::MAX), g));
            con_plazas.truncate(VECINAS);
        }
        if aliadas.len() < 2 && !con_plazas.is_empty() {
            let g = con_plazas[self.rng.por_debajo(con_plazas.len() as u64) as usize];
            opciones.push((25, Movimiento::Alianza(g)));
        }
        if mias >= 2 && !debiles.is_empty() {
            let g = debiles[self.rng.por_debajo(debiles.len() as u64) as usize];
            opciones.push((if ambiciosa { 15 } else { 6 }, Movimiento::Vasallaje(g)));
        }
        if ambiciosa {
            if !sin_plazas.is_empty() {
                let g = sin_plazas[self.rng.por_debajo(sin_plazas.len() as u64) as usize];
                opciones.push((30, Movimiento::Anexion(g)));
            } else if let Some(&(_, v)) = vasallas.first() {
                opciones.push((8, Movimiento::Anexion(v)));
            }
        }
        if let Some(&(id, g)) = aliadas.first() {
            opciones.extend([(1, Movimiento::Fusion(g)), (3, Movimiento::Romper(id))]);
            if traicionera && !en_guerra {
                opciones.push((5, Movimiento::Traicion(g)));
            }
        }
        if let Some(&(id, _)) = vasallas.first() {
            opciones.push((3, Movimiento::Liberar(id)));
        }
        let total: u64 = opciones.iter().map(|o| o.0).sum();
        let mut tirada = self.rng.por_debajo(total);
        let Some(k) = opciones.iter().position(|o| {
            let dentro = tirada < o.0;
            tirada = tirada.saturating_sub(o.0);
            dentro
        }) else {
            return;
        };
        let movimiento = opciones.swap_remove(k).1;
        let Some((rey, rc)) = self.rey_de(f) else { return };
        let Some((heroe, cuenta)) = self.autoridad(f) else { return };
        match movimiento {
            Movimiento::Nada => {}
            Movimiento::Guerra(g) => {
                let arrastre = self.n.estado().politica.relaciones.arrastre(f, g);
                let version = self.n.version();
                if self.orden(t, &cuenta, "DeclararGuerra", Comando::DeclararGuerra { heroe, a: f, b: g }).aceptada() {
                    self.pol_tras_guerra_obligada(version, Some((f, g, arrastre)));
                }
            }
            Movimiento::Alianza(g) => drop(self.orden(t, &cuenta, "ProponerAlianza", Comando::ProponerRelacion { heroe, a: f, b: g, vasallaje: false, tributo: None })),
            Movimiento::Vasallaje(g) => {
                let tributo = Some((Recurso::Trigo, TRIBUTO_POR_MINUTO));
                self.orden(t, &cuenta, "ProponerVasallaje", Comando::ProponerRelacion { heroe, a: f, b: g, vasallaje: true, tributo });
            }
            Movimiento::Anexion(g) => drop(self.orden(t, &cuenta, "ProponerAnexion", Comando::ProponerAnexion { heroe, absorbente: f, absorbida: g })),
            Movimiento::Fusion(g) => drop(self.orden(t, &rc, "ProponerFusion", Comando::ProponerFusion { heroe: rey, a: f, b: g, nombre: format!("Union {f}-{g}"), rey })),
            Movimiento::Romper(relacion) => drop(self.orden(t, &cuenta, "RomperAlianza", Comando::RomperRelacion { heroe, relacion, iniciador: f })),
            Movimiento::Liberar(relacion) => drop(self.orden(t, &cuenta, "LiberarVasalla", Comando::RomperRelacion { heroe, relacion, iniciador: f })),
            Movimiento::Traicion(g) => {
                let p = self.p();
                p.traicion.insert(f, (g, t + TRAICION_DIAS * DIA));
                p.m.traiciones_planeadas += 1;
            }
            Movimiento::Corona => {
                let designado = self.n.faccion(f).and_then(|x| x.embajador.filter(|&e| e != rey).or_else(|| x.ciudadanos.iter().copied().find(|&h| h != rey)));
                if let Some(designado) = designado {
                    self.orden(t, &rc, "AsignarRey", Comando::AsignarRey { heroe: rey, faccion: f, designado });
                }
            }
            Movimiento::Admitir => {
                let admitir = !self.n.faccion(f).is_some_and(|x| x.admite_otras_en_ataques);
                self.orden(t, &rc, "AdmitirOtrasFacciones", Comando::AdmitirOtrasFacciones { heroe: rey, faccion: f, admitir });
            }
        }
    }

    /// Tras una orden aceptada que declara una guerra (`DeclararGuerra`, `RebelionVasallo`): tiene que haber un `diplomacia.guerra_declarada` coherente.
    fn pol_tras_guerra_obligada(&mut self, version: u64, esperado: Option<(u32, u32, Vec<u32>)>) {
        if self.pol_tras_guerra(version, esperado) == 0 {
            self.p().v.arrastre += 1;
        }
    }

    /// Las guerras declaradas desde `version` (justo tras la orden): nadie del bando es el atacante, todo el bando (el objetivo y las arrastradas) queda en guerra con él y, si se
    /// sabe, el arrastre es el que `guerra.confirmar` enseñaba antes de la orden. Devuelve cuántas guerras miró.
    pub(super) fn pol_tras_guerra(&mut self, version: u64, esperado: Option<(u32, u32, Vec<u32>)>) -> usize {
        let guerras: Vec<(u32, u32, Vec<u32>)> = match self.n.cambios_desde(version) {
            Some(c) => c
                .flat_map(|c| c.eventos.iter())
                .filter(|e| e.codigo == "diplomacia.guerra_declarada")
                .map(|e| (dato(&e.datos, "atacante").unwrap_or(0) as u32, dato(&e.datos, "contra").unwrap_or(0) as u32, facciones_de(&e.valores, "arrastradas")))
                .collect(),
            None => vec![],
        };
        let mut malas = 0;
        for (a, b, arrastradas) in &guerras {
            malas += (a == b || arrastradas.contains(a) || arrastradas.contains(b)) as u64;
            malas += std::iter::once(b).chain(arrastradas).filter(|&&x| self.n.relacion(*a, x) != Some(Relacion::Guerra)).count() as u64;
            if let Some((ea, eb, lista)) = &esperado {
                malas += ((*a, *b) == (*ea, *eb) && arrastradas != lista) as u64;
            }
        }
        self.p().v.arrastre += malas;
        guerras.len()
    }

    // ───────────────────────────── La plaza ─────────────────────────────

    /// Lo político del Gobernador en su plaza, cada hora que gobierna: Palacio, Gran Fundición, cargos vacíos, puerta, vetos y almacén para las aliadas.
    pub(super) fn pol_plaza(&mut self, t: Instante, i: usize, p: u64) {
        let heroe = self.bots[i].heroe;
        let (f, nivel, palacio, gran, cargos, permite, vetados) = {
            let a = &self.n.estado().asentamientos[&p];
            let ed = &a.economia.edificios;
            (a.faccion, a.economia.nivel_actual(), ed.en_cualquier_estado(Tipo::Palacio) > 0, ed.en_cualquier_estado(Tipo::GranFundicion) > 0, a.cargos, a.permite_reabastecer_aliados, a.puerta.vetados.clone())
        };
        if nivel >= 2 && !palacio && t % (6 * HORA) < HORA {
            self.suya(t, i, "AnadirPalacio", Comando::AnadirEdificio { asentamiento: p, tipo: Tipo::Palacio });
        }
        if t % (12 * HORA) < HORA && !gran && self.n.capital_de(f) == Some(p) && self.n.nivel_de_faccion(f) >= 6 {
            self.suya(t, i, "AnadirGranFundicion", Comando::AnadirEdificio { asentamiento: p, tipo: Tipo::GranFundicion });
        }
        // Los cargos que quedaron vacíos (una conquista, una mudanza): a residentes de la plaza, el bot del papel que toca primero.
        let residentes: Vec<u64> = self.n.estado().militar.residentes_de(p).filter(|h| h.faccion == Some(f)).map(|h| h.heroe.id).collect();
        for (cargo, rol) in [(Cargo::Tesorero, Rol::Intendente), (Cargo::General, Rol::Capitan), (Cargo::MaestroObras, Rol::Guardian), (Cargo::Sacerdote, Rol::Cazador)] {
            if cargos.ocupado(cargo) || residentes.is_empty() {
                continue;
            }
            let del_papel = residentes.iter().copied().find(|&h| self.bot_de(h).is_some_and(|j| self.bots[j].rol == rol));
            let designado = del_papel.unwrap_or(residentes[0]);
            self.esperada(t, i, "AsignarCargoLocal", Comando::AsignarCargoLocal { heroe, asentamiento: p, cargo, designado });
        }
        if t % (12 * HORA) < HORA {
            // La puerta: abre a los neutrales o vuelve a cerrarles (la de por defecto cierra a neutrales y enemigos).
            if self.rng.por_debajo(100) < 15 {
                let abierta = self.n.puerta_cerrada_a(p).is_some_and(|g| !g.contiene(Grupo::Neutrales));
                let cerrada_a = if abierta { Grupos::de([Grupo::Neutrales, Grupo::Enemigos]) } else { Grupos::de([Grupo::Enemigos]) };
                self.suya(t, i, "FijarPuerta", Comando::FijarPuerta { heroe, asentamiento: p, cerrada_a });
            }
            // Los vetos: levanta uno de cada tres veces; si no, veta (el 10 %) a un héroe de una Facción en guerra con la suya que no resida aquí.
            if let Some(&vetado) = vetados.first().filter(|_| self.rng.por_debajo(100) < 35) {
                self.suya(t, i, "LevantarVeto", Comando::VetarJugador { heroe, asentamiento: p, vetado, vetar: false });
            } else if self.rng.por_debajo(100) < 10 {
                let reside = Some(Residencia::Asentamiento(p as u32));
                let enemigo = self.n.heroes().find(|h| h.faccion.is_some_and(|g| self.n.relacion(f, g) == Some(Relacion::Guerra)) && h.heroe.residencia != reside).map(|h| h.heroe.id);
                if let Some(vetado) = enemigo {
                    self.suya(t, i, "VetarJugador", Comando::VetarJugador { heroe, asentamiento: p, vetado, vetar: true });
                }
            }
        }
        if !permite && self.n.estado().politica.relaciones.alianzas_de(f) > 0 {
            self.suya(t, i, "AlternarReabastecerAliados", Comando::AlternarReabastecerAliados { asentamiento: p, permitido: true });
        }
    }

    /// Un héroe que reside en una plaza y está en un campamento (se mudó desde él): sale y vuelve a casa.
    pub(super) fn pol_salir_del_campamento(&mut self, t: Instante, i: usize, campamento: u32) {
        let heroe = self.bots[i].heroe;
        let r = self.suya(t, i, "SalirDelCampamento", Comando::SalirDelCampamento { campamento, heroe, escuadras: vec![], carga: vec![], politica: Politica::Rechazar });
        if let Some(&c) = r.creadas.first().filter(|_| r.aceptada()) {
            self.volver(t, i, c);
        }
    }

    // ───────────────────────────── Colonos, mudanzas y secesiones ─────────────────────────────

    pub(super) fn pol_colonos(&mut self, t: Instante) {
        self.programar(t + COLONOS_CADA_H * HORA, Accion::Colonos);
        // Nace un colono.
        let cuenta = format!("colono-{}", self.p().m.colonos);
        let campamento = self.rng.por_debajo(self.geo.campamentos.len() as u64) as u32;
        let r = self.segura(t, &cuenta, "CrearHeroe", Comando::CrearHeroe { cuenta: cuenta.clone(), campamento, controlador: Controlador::Humano });
        if let Some(&h) = r.creadas.first().filter(|_| r.aceptada()) {
            let p = self.p();
            p.colonos.push(h);
            p.m.colonos += 1;
        }
        // Los colonos sin Facción ni solicitud pendiente piden entrar en dos con plazas.
        let con_plazas: Vec<u32> = self.n.facciones().map(|f| f.id).filter(|&f| self.plazas_n(f) > 0).collect();
        for h in self.p().colonos.clone() {
            let pide = self.n.heroe(h).is_some_and(|x| x.faccion.is_none()) && !self.n.facciones().any(|f| f.solicitudes.contains(&h));
            if !pide || con_plazas.is_empty() {
                continue;
            }
            let cuenta = self.cuenta_de(h);
            for _ in 0..2 {
                let faccion = con_plazas[self.rng.por_debajo(con_plazas.len() as u64) as usize];
                self.orden(t, &cuenta, "SolicitarIngreso", Comando::SolicitarIngreso { heroe: h, faccion });
            }
        }
        // Los ciudadanos que residen en un campamento y cuya Facción tiene plazas se mudan a la suya con menos residentes (los colonos admitidos y los desalojados).
        let mut quienes: Vec<u64> = self.p().colonos.clone();
        let pol = self.pol.as_ref().expect("batch político");
        quienes.extend(self.bots.iter().enumerate().filter(|(j, b)| !pol.ocupado(*j, t) && !pol.disidentes.contains_key(&b.heroe)).map(|(_, b)| b.heroe));
        for h in quienes {
            let Some(x) = self.n.heroe(h) else { continue };
            let (Some(f), Some(Residencia::Campamento(_))) = (x.faccion, x.heroe.residencia) else { continue };
            let destino = self.n.plazas_de(f).map(|a| a.id).min_by_key(|&a| (self.n.estado().militar.residentes_de(a).count(), a));
            if let Some(asentamiento) = destino {
                let cuenta = x.cuenta.clone();
                self.orden(t, &cuenta, "CambiarResidencia", Comando::CambiarResidencia { heroe: h, asentamiento });
            }
        }
        if t % DIA < COLONOS_CADA_H * HORA && self.rng.por_debajo(100) < SECESION_PCT {
            self.pol_secesion(t);
        }
        // Los disidentes crean su Facción en cuanto pasa el cooldown de creación (si no entraron en otra).
        let cooldown = self.n.estado().reglas.economia.politica.cooldown_creacion;
        let listos: Vec<u64> = self.p().disidentes.iter().filter(|(_, &salio)| t >= salio + cooldown).map(|(&h, _)| h).collect();
        for h in listos {
            if self.n.heroe(h).is_some_and(|x| x.faccion.is_none()) {
                let cuenta = self.cuenta_de(h);
                let r = self.orden(t, &cuenta, "CrearFaccion", Comando::CrearFaccion { heroe: h, nombre: format!("Disidentes de {h}"), sigilo: None });
                let Some(&g) = r.creadas.first().filter(|_| r.aceptada()) else { continue };
                self.programar(t + HORA, Accion::Diplomacia { faccion: g as u32 });
            }
            self.p().disidentes.remove(&h);
        }
    }

    /// Un Cazador de una Facción grande, en casa y que no es su Rey ni su Embajador, la deja para crear la suya (sus cargos locales se vacían al dejarla).
    fn pol_secesion(&mut self, t: Instante) {
        let grandes: Vec<u32> = self.n.facciones().filter(|f| f.ciudadanos.len() >= SECESION_MIN_CIUDADANOS).map(|f| f.id).collect();
        if grandes.is_empty() {
            return;
        }
        let f = grandes[self.rng.por_debajo(grandes.len() as u64) as usize];
        let Some(x) = self.n.faccion(f) else { return };
        let (rey, embajador) = (x.rey, x.embajador);
        let candidato = (0..self.bots.len()).find(|&i| {
            let h = self.bots[i].heroe;
            let en_casa = self.casa(i).is_some_and(|p| self.heroe(i).is_some_and(|x| x.heroe.presencia.ubicacion == Ubicacion::Asentamiento(p as u32)));
            self.bots[i].rol == Rol::Cazador && self.faccion_bot(i) == f && en_casa && Some(h) != rey && Some(h) != embajador && !self.pol.as_ref().is_some_and(|p| p.ocupado(i, t))
        });
        let Some(i) = candidato else { return };
        let heroe = self.bots[i].heroe;
        if self.suya(t, i, "DejarFaccion", Comando::DejarFaccion { heroe }).aceptada() {
            let p = self.p();
            p.disidentes.insert(heroe, t);
            p.m.secesiones += 1;
        }
    }

    // ───────────────────────────── La refundación ─────────────────────────────

    /// Una Facción sin plazas junta el fondo en el campamento de su Rey (o de la mayoría de los suyos), compra la Caravana de Refundación y sale a fundar.
    fn pol_refundar(&mut self, t: Instante, f: u32) {
        if self.p().tareas.iter().any(|x| x.viva && x.faccion == f) {
            return;
        }
        let Some(x) = self.n.faccion(f) else { return };
        let (ciudadanos, rey) = (x.ciudadanos.clone(), x.rey);
        let reside = |s: &Self, h: u64| match s.n.heroe(h)?.heroe.residencia {
            Some(Residencia::Campamento(k)) => Some(k),
            _ => None,
        };
        let mut por_campamento: BTreeMap<u32, usize> = BTreeMap::new();
        ciudadanos.iter().filter_map(|&h| reside(self, h)).for_each(|k| *por_campamento.entry(k).or_insert(0) += 1);
        let Some(k) = rey.and_then(|r| reside(self, r)).or_else(|| por_campamento.iter().max_by_key(|&(&k, &n)| (n, std::cmp::Reverse(k))).map(|(&k, _)| k)) else { return };
        let dentro: Vec<u64> = ciudadanos
            .iter()
            .copied()
            .filter(|&h| reside(self, h) == Some(k) && self.n.heroe(h).is_some_and(|x| x.heroe.presencia.ubicacion == Ubicacion::Mercenarios(k) && !x.heroe.esta_herido(t)))
            .collect();
        if dentro.is_empty() {
            return;
        }
        let ya = self.n.estado().fundacion.caravanas.values().find(|c| c.faccion == Some(f) && c.situacion == SituacionFundacion::Disponible).map(|c| (c.id, c.titular));
        let (caravana, titular) = match ya {
            Some(c) => c,
            None => {
                let costo = self.n.costo_refundacion();
                // Con el almacén personal lleno no cabe lo que se compra ni el oro de la caza: lo que no es oro se guarda en el fondo (lo suyo, se puede retirar).
                let cabe = self.n.estado().reglas.economia.comercio.mercenarios.capacidad_almacen_personal;
                for &h in &dentro {
                    let almacen = self.n.heroe(h).map(|x| x.almacen_personal.clone()).unwrap_or_default();
                    if militar::total(&almacen) * 2 <= cabe {
                        continue;
                    }
                    let cuenta = self.cuenta_de(h);
                    for (recurso, q) in almacen.into_iter().filter(|&(r, q)| r != Recurso::Oro && q >= base::UNIDAD) {
                        let cantidad = q / base::UNIDAD * base::UNIDAD;
                        self.orden(t, &cuenta, "AportarParaHacerSitio", Comando::AportarARefundacion { heroe: h, recurso, cantidad, lado: LadoDelFondo::Almacen });
                    }
                }
                for (&r, &q) in &costo {
                    for &h in &dentro {
                        let falta = q - self.n.fondo_de_faccion(k, f).get(&r).copied().unwrap_or(0);
                        if falta <= 0 {
                            break;
                        }
                        let cuenta = self.cuenta_de(h);
                        let tiene = |s: &Self, r: Recurso| s.n.heroe(h).and_then(|x| x.almacen_personal.get(&r).copied()).unwrap_or(0) / base::UNIDAD * base::UNIDAD;
                        // Lo que no tiene lo compra en el mercado del campamento con su oro.
                        if tiene(self, r) < falta && r != Recurso::Oro && tiene(self, Recurso::Oro) > 0 {
                            self.orden(t, &cuenta, "ComprarEnCampamento", Comando::ComprarEnCampamento { heroe: h, campamento: k, recurso: r, cantidad: falta - tiene(self, r) });
                        }
                        let pon = falta.min(tiene(self, r));
                        if pon > 0 {
                            self.orden(t, &cuenta, "AportarARefundacion", Comando::AportarARefundacion { heroe: h, recurso: r, cantidad: pon, lado: LadoDelFondo::Almacen });
                        }
                    }
                }
                let fondo = self.n.fondo_de_faccion(k, f);
                if costo.iter().any(|(r, &q)| fondo.get(r).copied().unwrap_or(0) < q) {
                    self.p().m.sin_fondo += 1;
                    // Sin oro con que comprar lo que falta: uno de los suyos con tropa sale a por los bandidos del anillo.
                    let cazador = dentro.iter().copied().find(|&h| self.bot_de(h).is_some() && self.n.heroe(h).is_some_and(|x| x.heroe.escuadras.iter().any(|s| s.contenedor == Contenedor::Campamento && s.cantidad > 0)));
                    if let Some(h) = cazador {
                        self.pol_salir_a_cazar(t, f, k, h);
                    }
                    return;
                }
                // El titular: uno de los bots militares de dentro (sale con su tropa), si no el primero.
                let titular = dentro.iter().copied().find(|&h| self.bot_de(h).is_some()).unwrap_or(dentro[0]);
                let cuenta = self.cuenta_de(titular);
                let r = self.segura(t, &cuenta, "ComprarCaravanaDeRefundacion", Comando::ComprarCaravanaDeRefundacion { heroe: titular });
                let Some(&id) = r.creadas.first().filter(|_| r.aceptada()) else { return };
                (id, titular)
            }
        };
        if dentro.contains(&titular) {
            self.pol_partir_a_fundar(t, f, k, caravana, titular);
        }
    }

    fn pol_partir_a_fundar(&mut self, t: Instante, faccion: u32, k: u32, caravana: u64, titular: u64) {
        let cuenta = self.cuenta_de(titular);
        let escuadras: Vec<u64> = self.n.heroe(titular).map(|h| h.heroe.escuadras.iter().filter(|s| s.contenedor == Contenedor::Campamento && !s.en_guarnicion && s.cantidad > 0).map(|s| s.id).collect()).unwrap_or_default();
        let r = self.orden(t, &cuenta, "SalirDelCampamento", Comando::SalirDelCampamento { campamento: k, heroe: titular, escuadras, carga: vec![], politica: Politica::Aceptar });
        let Some(&columna) = r.creadas.first().filter(|_| r.aceptada()) else { return };
        self.orden(t, &cuenta, "RepostarEnCampamento", Comando::RepostarEnCampamento { heroe: titular, campamento: k });
        if !self.segura(t, &cuenta, "AdjuntarCaravana", Comando::AdjuntarCaravana { caravana, ejercito: columna }).aceptada() {
            return;
        }
        let puerta = self.n.puerta_de_campamento(k).unwrap_or_default();
        let p = self.p();
        p.tareas.push(Tarea { faccion, titular, columna, caravana, intentos: 0, viva: true, caza: None, campamento: k, fase: 0 });
        p.m.tareas += 1;
        let tarea = p.tareas.len() - 1;
        self.pol_marchar_a_fundar(t, tarea, puerta, ANILLO_CAMPAMENTO);
    }

    fn pol_marchar_a_fundar(&mut self, t: Instante, tarea: usize, centro: Punto, anillo: (i64, i64)) {
        let Tarea { faccion, titular, columna, .. } = self.p().tareas[tarea].clone();
        let destino = sitio(&self.n, &mut self.rng, faccion, centro, anillo);
        let cuenta = self.cuenta_de(titular);
        let marcha = destino.is_some_and(|d| self.orden(t, &cuenta, "MarcharAFundar", Comando::MarcharA { ejercito: columna, destino: d }).aceptada());
        if !marcha {
            return self.pol_tarea_fallida(tarea);
        }
        let llega = self.n.ejercito(columna).and_then(|e| e.plan.fin()).unwrap_or(t).max(t + 1);
        if let Some(i) = self.bot_de(titular) {
            self.p().ocupados.insert(i, llega + HORA);
        }
        self.programar(llega, Accion::Refundar { tarea });
    }

    /// Sale del campamento como ejército con su tropa hacia el campamento de bandidos del anillo más cercano a la puerta.
    fn pol_salir_a_cazar(&mut self, t: Instante, faccion: u32, k: u32, titular: u64) {
        let Some(puerta) = self.n.puerta_de_campamento(k) else { return };
        let objetivo = self.n.campamentos_de_bandidos().filter(|b| b.origen == nucleo::OrigenBandidos::Anillo(k)).min_by_key(|b| (b.posicion.dist(puerta), b.id)).map(|b| (b.id, b.posicion));
        let Some((bandidos, sitio)) = objetivo else { return };
        let cuenta = self.cuenta_de(titular);
        let escuadras: Vec<u64> = self.n.heroe(titular).map(|h| h.heroe.escuadras.iter().filter(|s| s.contenedor == Contenedor::Campamento && !s.en_guarnicion && s.cantidad > 0).map(|s| s.id).collect()).unwrap_or_default();
        let r = self.orden(t, &cuenta, "SalirACazar", Comando::SalirDelCampamento { campamento: k, heroe: titular, escuadras, carga: vec![], politica: Politica::Aceptar });
        let Some(&columna) = r.creadas.first().filter(|_| r.aceptada()) else { return };
        let p = self.p();
        p.tareas.push(Tarea { faccion, titular, columna, caravana: 0, intentos: 0, viva: true, caza: Some(bandidos), campamento: k, fase: 0 });
        p.m.cazas += 1;
        let tarea = p.tareas.len() - 1;
        self.pol_marchar(t, tarea, sitio);
    }

    fn pol_marchar(&mut self, t: Instante, tarea: usize, destino: Punto) {
        let Tarea { titular, columna, .. } = self.p().tareas[tarea].clone();
        let cuenta = self.cuenta_de(titular);
        if !self.orden(t, &cuenta, "MarcharACazar", Comando::MarcharA { ejercito: columna, destino }).aceptada() {
            return self.pol_tarea_fallida(tarea);
        }
        let llega = self.n.ejercito(columna).and_then(|e| e.plan.fin()).unwrap_or(t).max(t + 1);
        if let Some(i) = self.bot_de(titular) {
            self.p().ocupados.insert(i, llega + HORA);
        }
        self.programar(llega, Accion::Refundar { tarea });
    }

    /// La caza para el fondo: al llegar ataca (con las comprobaciones de toda orden de ataque del batch) y vuelve; en la puerta aporta desde el carro lo que trae y entra.
    fn pol_llega_de_caza(&mut self, t: Instante, tarea: usize, bandidos: u64) {
        let Tarea { titular, columna, campamento, fase, .. } = self.p().tareas[tarea].clone();
        let (Some(e), Some(i)) = (self.n.ejercito(columna), self.bot_de(titular)) else { return self.pol_tarea_fallida(tarea) };
        if let Some(fin) = e.plan.fin().filter(|&fin| fin > t) {
            return self.programar(fin, Accion::Refundar { tarea });
        }
        let puerta = self.n.puerta_de_campamento(campamento).unwrap_or_default();
        if fase == 0 {
            if self.n.campamentos_de_bandidos().any(|b| b.id == bandidos) && !self.n.heroe_herido(titular) {
                self.atacar(t, i, columna, ObjetivoAtaque::Campamento(bandidos));
            }
            self.p().tareas[tarea].fase = 1;
            return self.pol_marchar(t, tarea, puerta);
        }
        // Entra y vacía el carro en su almacén personal: con ese oro compra en el mercado del campamento lo que le falte al fondo y lo aporta en el turno siguiente.
        let cuenta = self.cuenta_de(titular);
        self.p().m.oro_de_caza += self.n.ejercito(columna).and_then(|e| e.columna.carga.get(&Recurso::Oro).copied()).unwrap_or(0);
        self.orden(t, &cuenta, "EntrarEnCampamento", Comando::EntrarEnCampamento { campamento, heroe: titular });
        self.orden(t, &cuenta, "VaciarCarroDeCaza", Comando::VaciarCarro { heroe: titular });
        let p = self.p();
        p.tareas[tarea].viva = false;
        p.ocupados.remove(&i);
    }

    pub(super) fn pol_llega_a_refundar(&mut self, t: Instante, tarea: usize) {
        let Tarea { faccion, titular, columna, caravana, intentos, viva, caza, .. } = self.p().tareas[tarea].clone();
        if !viva {
            return;
        }
        if let Some(bandidos) = caza {
            return self.pol_llega_de_caza(t, tarea, bandidos);
        }
        let la_lleva = self.n.estado().fundacion.caravanas.get(&caravana).is_some_and(|c| c.situacion == SituacionFundacion::Adjunta(columna));
        let Some(e) = self.n.ejercito(columna).filter(|_| la_lleva) else { return self.pol_tarea_fallida(tarea) };
        if let Some(fin) = e.plan.fin().filter(|&fin| fin > t) {
            return self.programar(fin, Accion::Refundar { tarea });
        }
        let aqui = e.posicion_en(t);
        let cuenta = self.cuenta_de(titular);
        let r = self.orden(t, &cuenta, "FundarRefundacion", Comando::Fundar { ejercito: columna });
        if let Some(&plaza) = r.creadas.first().filter(|_| r.aceptada()) {
            self.p().tareas[tarea].viva = false;
            if let Some(i) = self.bot_de(titular) {
                self.p().ocupados.remove(&i);
            }
            // Los suyos que residen en un campamento se mudan a la plaza nueva.
            let suyos: Vec<u64> = self.n.faccion(faccion).map(|x| x.ciudadanos.clone()).unwrap_or_default();
            for h in suyos {
                if self.n.heroe(h).is_some_and(|x| matches!(x.heroe.residencia, Some(Residencia::Campamento(_)))) {
                    let cuenta = self.cuenta_de(h);
                    self.orden(t, &cuenta, "CambiarResidencia", Comando::CambiarResidencia { heroe: h, asentamiento: plaza });
                }
            }
            return;
        }
        if intentos + 1 >= INTENTOS_DE_FUNDAR {
            return self.pol_tarea_fallida(tarea);
        }
        self.p().tareas[tarea].intentos += 1;
        self.pol_marchar_a_fundar(t, tarea, aqui, (20_000, 60_000));
    }

    fn pol_tarea_fallida(&mut self, tarea: usize) {
        let titular = self.p().tareas[tarea].titular;
        let i = self.bot_de(titular);
        let p = self.p();
        p.tareas[tarea].viva = false;
        p.m.tareas_fallidas += 1;
        if let Some(i) = i {
            p.ocupados.remove(&i);
        }
    }

    // ───────────────────────────── Hechos e invariantes ─────────────────────────────

    pub(super) fn pol_hecho(&mut self, codigo: &str, t: Instante, plazas: &[u64], datos: &[(&'static str, i64)], valores: &[(&'static str, Dato)]) {
        let d = |k: &str| dato(datos, k);
        let rr = self.n.estado().reglas.economia.comercio.reputacion.clone();
        let relacion = d("relacion").and_then(|id| self.n.relacion_politica(id as u64).cloned());
        let distancia = plazas.first().and_then(|p| self.p().distancia.get(p).copied()).unwrap_or(0);
        let p = self.p();
        if codigo.starts_with("reputacion.") || codigo.starts_with("diplomacia.") {
            p.rep_tocadas.extend(["faccion", "faccionA", "faccionB", "atacante", "contra", "faccionAnterior"].iter().filter_map(|k| d(k)).map(|x| x as u32));
            p.rep_tocadas.extend(facciones_de(valores, "arrastradas"));
            p.rep_todas |= matches!(codigo, "diplomacia.anexion" | "diplomacia.fusion");
        }
        match codigo {
            "reputacion.cambio" => {
                p.m.rep_cambios += 1;
                let maximo = [
                    Suceso::TruequeCumplido,
                    Suceso::TruequeIncumplido { fraccion: asentamiento::datos::UNO },
                    Suceso::LiberarVasalloVoluntario,
                    Suceso::RebelionParaSenora,
                    Suceso::RomperAlianza,
                    Suceso::AtacarAliado,
                    Suceso::DefenderVasallo,
                    Suceso::NoDefenderVasallo,
                    Suceso::GuerraTrasRomperAlianza,
                ]
                .iter()
                .map(|s| s.delta(&rr).abs())
                .max()
                .unwrap_or(0);
                p.v.reputacion += (d("delta").unwrap_or(0).abs() > maximo) as u64;
            }
            "diplomacia.relacion_aceptada" => match d("tipo") {
                Some(0) => p.m.alianzas += 1,
                Some(1) => p.m.vasallajes += 1,
                _ => {}
            },
            "diplomacia.relacion_rota" => match d("tipo") {
                Some(0) => p.m.alianzas_rotas += 1,
                Some(1) => p.m.vasallajes_rotos += 1,
                _ => {}
            },
            "diplomacia.guerra_declarada" => {
                p.m.traiciones += (d("traicion") == Some(1)) as u64;
                p.m.arrastradas += facciones_de(valores, "arrastradas").len() as u64;
            }
            "diplomacia.tributo_pagado" => {
                let (pagadas, recibidas) = (d("pagadas").unwrap_or(0), d("recibidas").unwrap_or(0));
                p.m.tributo_pagado += pagadas;
                p.m.tributo_recibido += recibidas;
                p.m.tributos += 1;
                let id = d("relacion").unwrap_or(0) as u64;
                *p.m.tributo_por_relacion.entry(id).or_insert(0) += pagadas;
                // Un vasallaje con tributo de esa vasalla (la señora puede haber cambiado después, con una anexión).
                let coherente = relacion.is_some_and(|r| r.tipo == TipoRelacion::Vasallaje && Some(i64::from(r.b)) == d("faccionB") && r.tributo.is_some());
                p.v.tributo += (pagadas <= 0 || recibidas < 0 || recibidas > pagadas || !coherente) as u64;
            }
            "faccion.creada" if t > 0 => drop(p.m.sin_plazas_desde.insert(d("faccion").unwrap_or(0) as u32, t)),
            "faccion.derrotada" => {
                let f = d("faccion").unwrap_or(0) as u32;
                p.m.derrotas.push((t, f, d("por").unwrap_or(0) as u32));
                p.m.sin_plazas_desde.entry(f).or_insert(t);
            }
            "fundacion.asentamiento_fundado" => {
                let f = d("faccion").unwrap_or(0) as u32;
                if let (Some(desde), Some(&plaza)) = (p.m.sin_plazas_desde.remove(&f), plazas.first()) {
                    p.m.refundaciones.push((t, f, plaza, t - desde));
                }
            }
            "titulo.nace" | "titulo.cambia_manos" => {
                let clave = match valores.iter().find(|(k, _)| *k == "titulo") {
                    Some((_, Dato::Texto(s))) => s.clone(),
                    _ => "?".into(),
                };
                let e = p.m.titulos.entry(clave).or_insert((0, 0));
                if codigo == "titulo.nace" {
                    e.0 += 1;
                } else {
                    e.1 += 1;
                }
            }
            "asentamiento.ruinas" => {
                if let Some(&plaza) = plazas.first() {
                    p.m.ruinas.push((t, plaza, distancia));
                }
            }
            _ => {}
        }
    }

    /// Los invariantes del estado político en cada observación, la distancia de cada plaza a su capital y la foto diaria.
    pub(super) fn pol_observar(&mut self, t: Instante) {
        let n = &self.n;
        let e = n.estado();
        let mut v = ViolacionesPoliticas::default();
        // Ciudadanía: cada héroe en una sola Facción, la misma que dice el héroe; Rey y Embajador entre los ciudadanos.
        let mut ciudadania: BTreeMap<u64, u32> = BTreeMap::new();
        for f in n.facciones() {
            for &c in &f.ciudadanos {
                v.ciudadania += ciudadania.insert(c, f.id).is_some() as u64;
                v.ciudadania += (n.heroe(c).map(|h| h.faccion) != Some(Some(f.id))) as u64;
            }
            v.rey += (!f.ciudadanos.is_empty() && !f.rey.is_some_and(|r| f.ciudadanos.contains(&r))) as u64;
            v.rey += f.embajador.is_some_and(|x| !f.ciudadanos.contains(&x)) as u64;
        }
        v.ciudadania += n.heroes().filter(|h| h.faccion.is_some_and(|f| ciudadania.get(&h.heroe.id) != Some(&f))).count() as u64;
        // Cargos locales: de residentes ciudadanos de la dueña.
        for a in e.asentamientos.values() {
            for c in CARGOS {
                if let Some(h) = a.cargos.titular(c) {
                    let ok = n.heroe(h).is_some_and(|x| x.heroe.residencia == Some(Residencia::Asentamiento(a.id as u32)) && x.faccion == Some(a.faccion));
                    v.cargos += !ok as u64;
                }
            }
        }
        // Relaciones: una activa por par; ninguna vasalla con dos señoras.
        let (mut pares, mut senoras): (BTreeMap<(u32, u32), u32>, BTreeMap<u32, u32>) = Default::default();
        for r in e.politica.relaciones.activas() {
            *pares.entry((r.a.min(r.b), r.a.max(r.b))).or_insert(0) += 1;
            if r.tipo == TipoRelacion::Vasallaje {
                *senoras.entry(r.b).or_insert(0) += 1;
            }
            v.relaciones += (r.a == r.b || e.politica.relaciones.activa_entre(r.a, r.b).map(|x| x.id) != Some(r.id)) as u64;
        }
        v.relaciones += pares.values().filter(|&&k| k > 1).count() as u64 + senoras.values().filter(|&&k| k > 1).count() as u64;
        // Experiencia monótona.
        let experiencias: Vec<(u32, i64)> = n.facciones().map(|f| (f.id, f.experiencia)).collect();
        let actuales: BTreeSet<u32> = experiencias.iter().map(|x| x.0).collect();
        // La reputación: sin suceso ni relación suya en la ventana (ni cambio de régimen al cruzar el 0), su valor sigue el ritmo de la observación anterior.
        let rr = &e.reglas.economia.comercio.reputacion;
        let pol = self.pol.as_mut().expect("batch político");
        for &(f, x) in &experiencias {
            v.experiencia += pol.experiencia.insert(f, x).is_some_and(|antes| x < antes) as u64;
        }
        for (&f, cur) in &e.comercio.reputacion {
            v.reputacion += (cur.valor_en(t).abs() > comercio::reputacion::MAXIMO) as u64;
            if pol.rep_todas || pol.rep_tocadas.contains(&f) {
                continue;
            }
            match pol.reputacion.get(&f) {
                None => v.reputacion += 1,
                Some(prev) => {
                    let regimen = prev.proximo_cambio(pol.rep_desde).is_some_and(|c| c <= t);
                    // Reanclar el ritmo fuera de un minuto entero (la cita del umbral o del cruce) redondea: se tolera una millonésima de punto por reanclaje.
                    let mala = !regimen && ((prev.valor_en(t) - cur.valor_en(t)).abs() > TOLERANCIA_REPUTACION || prev.alianzas() != cur.alianzas());
                    v.reputacion += mala as u64;
                }
            }
        }
        let _ = rr;
        pol.reputacion = e.comercio.reputacion.clone();
        pol.rep_desde = t;
        pol.rep_tocadas.clear();
        pol.rep_todas = false;
        // Ids: una Facción que desapareció no vuelve.
        v.ids += actuales.iter().filter(|f| pol.desaparecidas.contains(f)).count() as u64;
        let idas: Vec<u32> = pol.vistas.iter().copied().filter(|f| !actuales.contains(f)).collect();
        pol.desaparecidas.extend(idas);
        pol.vistas.extend(actuales.iter().copied());
        // El tributo no pasa de lo pactado (una unidad de margen por plaza por el resto oculto; se cuenta una vez por relación).
        for (&id, &pagado) in &pol.m.tributo_por_relacion {
            let Some(r) = e.politica.relaciones.get(id) else { continue };
            let fin = r.rota_en.unwrap_or(t).min(t);
            let tope = r.tributo.map_or(0, |x| x.cantidad_por_minuto) * ((fin - r.creada_en) / MIN + 1) / base::UNIDAD + 8;
            if pagado > tope && pol.m.tributo_excedido.insert(id) {
                v.tributo += 1;
            }
        }
        // La distancia de cada plaza a la capital de su Facción.
        for a in e.asentamientos.values() {
            let d = n.capital_de(a.faccion).filter(|&c| c != a.id && self.n.estado().asentamientos.contains_key(&c)).map_or(0, |c| a.posicion.dist(e.asentamientos[&c].posicion));
            pol.distancia.insert(a.id, d);
        }
        let s = &mut pol.v;
        s.ciudadania += v.ciudadania;
        s.rey += v.rey;
        s.cargos += v.cargos;
        s.relaciones += v.relaciones;
        s.reputacion += v.reputacion;
        s.ids += v.ids;
        s.experiencia += v.experiencia;
        s.tributo += v.tributo;
        let dia = (t / DIA) as u32;
        if t % DIA < OBSERVAR_MIN * MIN && pol.muestras.last().is_none_or(|m| m.dia != dia) {
            self.pol_muestrear(t, dia);
        }
    }

    fn pol_muestrear(&mut self, t: Instante, dia: u32) {
        let n = &self.n;
        let e = n.estado();
        let escala = e.reglas.economia.datos.mantenimiento.escala_distancia.max(1);
        let relaciones = |tipo| e.politica.relaciones.activas().filter(|r| r.tipo == tipo).count();
        let palacio = |a: &nucleo::Asentamiento| a.economia.edificios.iter().any(|x| x.tipo == Tipo::Palacio && x.activo());
        let pob = |a: &nucleo::Asentamiento| a.economia.poblacion.enteros();
        let muestra = MuestraPolitica {
            dia,
            facciones: n.facciones().count(),
            con_plazas: n.facciones().filter(|f| n.plazas_de(f.id).next().is_some()).count(),
            niveles: n.facciones().map(|f| n.nivel_de_faccion(f.id)).collect(),
            reputaciones: n.facciones().map(|f| e.comercio.reputacion.get(&f.id).map_or(0, |r| r.valor_en(t))).collect(),
            alianzas: relaciones(TipoRelacion::Alianza),
            vasallajes: relaciones(TipoRelacion::Vasallaje),
            guerras: relaciones(TipoRelacion::Guerra),
            nobles: e.asentamientos.values().map(|a| pob(a).2).sum(),
            con_nobleza: e.asentamientos.values().filter(|a| pob(a).2 > 0).count(),
            con_palacio: e.asentamientos.values().filter(|a| palacio(a)).count(),
            multiplicadores: self.pol.as_ref().expect("batch político").distancia.values().filter(|&&d| d > 0).map(|&d| 1_000_000 + d * 1_000_000 / escala).collect(),
            residentes: e.asentamientos.keys().map(|&p| e.militar.residentes_de(p).count()).sum(),
            poblacion: e.asentamientos.values().map(|a| pob(a).0 + pob(a).1 + pob(a).2).sum(),
            plazas: e.asentamientos.len(),
        };
        self.p().muestras.push(muestra);
    }
}
