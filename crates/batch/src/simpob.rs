//! La población sintética del sistema «población, construcción y mantenimiento»: N asentamientos fundados a la vez sobre el mundo real
//! que viven `dias` días minuto a minuto con la política más simple posible (pedir la subida cada hora si nada la bloquea). Un bucle
//! global por minuto avanza todos, así que compiten de verdad por los nodos de piedra y las parcelas de bosque.

use crate::economia::{Mapa, Reclamos, Recursos, SinSitio, SinSitioCiudad, SitiosVilla};
use asentamiento::asentamiento::{Asentamiento, Evento, Exterior};
use asentamiento::ascenso::Bloqueo;
use asentamiento::catalogo::{Tipo, TIPOS};
use asentamiento::datos::UNO;
use asentamiento::edificio::Fuente;
use asentamiento::recurso::{Recurso, TODOS};
use base::{Instante, Mil, Punto, Rng, MS_POR_MINUTO};
use mundo::Mundo;
use nucleo::urbanismo::Urbe;
use nucleo::ReglasEconomia;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};
use territorio::{Disco, DistanciasDeFundacion, Terreno, Viabilidad};
use trazado::{Categoria, Rol};

const MIN_POR_DIA: i64 = 1440;
/// Cada cuántos minutos el «jugador» sintético mira si puede subir de nivel.
pub const CADA_MIN_ASCENSO: i64 = 60;
/// Candidatos por ronda y rondas máximas para colocar una plaza (si ninguno es fundable, otra ronda).
pub const CANDIDATOS: usize = 25;
pub const MAX_RONDAS: usize = 40;
/// Anillo de búsqueda alrededor del campamento (milésimas): 30-120 u.
pub const ANILLO_CAMPAMENTO: (i64, i64) = (30_000, 120_000);
/// El recinto que compromete el jugador: la empalizada (nivel 1), en cuanto el nivel operativo de la plaza lo permite (Murallas §4, gate de nivel 2), como el bot
/// del TypeScript (`bots/cerebro/gobierno.ts:197`). La mejora a piedra pide el nivel 3, al que este batch no llega.
pub const NIVEL_RECINTO: u8 = 1;

fn clase(b: &Bloqueo) -> usize {
    match b {
        Bloqueo::NivelMaximo => 0,
        Bloqueo::AscensoEnCurso => 1,
        Bloqueo::FaltaPoblacion => 2,
        Bloqueo::FaltanEdificios => 3,
        Bloqueo::SinCupoDeFaccion => 4,
        Bloqueo::RecursosInsuficientes => 5,
        Bloqueo::Insolvente(_) => 6,
    }
}

/// Lo que se mide de cada plaza mientras vive.
#[derive(Clone, Debug, Default)]
pub struct Registro {
    /// Minuto de mundo en que se completó la primera Leñera, Cantera y Corral.
    pub primera_lenera: Option<i64>,
    pub primera_cantera: Option<i64>,
    pub primera_corral: Option<i64>,
    pub nivel2: Option<i64>,
    pub nivel3: Option<i64>,
    pub ascenso_pedido: Option<i64>,
    pub ruinas: Option<i64>,
    /// Recursos que no pudo pagar al caer en ruinas.
    pub ruinas_faltantes: Vec<Recurso>,
    /// Habitantes muertos de hambre.
    pub muertes: i64,
    pub min_deficit_trigo: u64,
    pub pasos: u64,
    pub nutricion_min: Mil,
    pub medidor_min: Mil,
    /// Cuánto bajó como mucho el nivel operativo respecto del alcanzado (0 = nunca).
    pub degradacion_max: u32,
    pub colapsos: u32,
    pub pesants_max: i64,
    pub pesants_200: Option<i64>,
    /// Pesants al final de cada día, del 0 al último.
    pub pesants_dia: Vec<i64>,
    pub viviendas_dia: Vec<u32>,
    /// Edificios activos por tipo en cada día de foto (el de la primera foto y el final).
    pub edificios_a: BTreeMap<Tipo, u32>,
    pub edificios_fin: BTreeMap<Tipo, u32>,
    pub sin_sitio: SinSitio,
    /// Lo que la ciudad le negó (por tipo) y las mejoras que esperaron hueco.
    pub ciudad: SinSitioCiudad,
    /// Piezas de la ciudad en la foto intermedia y al final.
    pub piezas_a: usize,
    pub piezas_fin: usize,
    /// Minuto en que el trazado creó la primera ancla de saturación.
    pub primera_ancla: Option<i64>,
    /// En cuántas comprobaciones horarias (con el nivel 1) pesó cada bloqueo.
    pub bloqueos_horas: [u32; 7],
    pub comprobaciones_n1: u32,
    /// Minuto en que por primera vez dejó de pesar cada bloqueo.
    pub libre_de: [Option<i64>; 7],
    pub ultimos_bloqueos: Vec<usize>,
    /// La muralla (paso 7 del plan de Murallas): minuto en que se comprometió la empalizada y en que se cerró, celdas y puertas del anillo, lo que cuesta
    /// levantarlo entero y, si nunca se pudo trazar, el último motivo.
    pub recinto_comprometido: Option<i64>,
    pub recinto_completado: Option<i64>,
    pub recinto_celdas: usize,
    pub recinto_puertas: usize,
    pub recinto_coste: asentamiento::Costo,
    pub recinto_error: Option<trazado::ErrorMuralla>,
}

/// Una plaza viva.
pub struct Villa {
    pub pos: Punto,
    /// Nació en un sitio Verde (parcelas libres y piedra en la zona inicial).
    pub verde: bool,
    /// Parcelas de bosque y nodos de piedra en el disco del techo del nivel 1 al fundar (lo que el sitio ofrece como máximo).
    pub parcelas_n1: usize,
    /// Suma de las densidades (ppm, 1,0 = 1.000.000) de esas parcelas: la madera que da el bosque a 10 u si se explotase entero.
    pub densidad_n1: i64,
    pub piedra_n1: usize,
    pub a: Asentamiento,
    /// La ciudad de la plaza: dónde está cada uno de sus edificios (Trazado_Urbano).
    pub urbe: Urbe,
    /// Radio de la zona (milésimas): crece 1 u por edificio completado hasta el techo de su nivel.
    pub radio: Mil,
    pub parcelas: BTreeSet<u32>,
    pub reg: Registro,
}

/// Violaciones de los invariantes de §15 (todas deben ser 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Violaciones {
    pub almacen: u64,
    pub poblacion: u64,
    pub medidor: u64,
    pub nivel: u64,
    pub duplicados: u64,
    pub reclamos: u64,
}

impl Violaciones {
    pub fn total(&self) -> u64 {
        self.almacen + self.poblacion + self.medidor + self.nivel + self.duplicados + self.reclamos
    }
}

/// Violaciones de los invariantes de la ciudad (Trazado_Urbano §13) y de su acuerdo con la economía; todas deben ser 0. Se comprueban
/// al final en todas las plazas que siguen en pie.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViolacionesCiudad {
    /// Plazas comprobadas.
    pub plazas: u32,
    /// Pares de piezas que se pisan.
    pub solape: u64,
    /// Piezas sobre una celda de calle o de camino.
    pub suelo: u64,
    /// Genéricos fuera del disco urbano o, en las afueras, piezas fuera de su banda (anclas y satélites no miran el disco: Trazado_Urbano §6.5, §7.2).
    pub disco: u64,
    /// Piezas sin ninguna celda de calle o camino pegada, en plazas sin recintos.
    pub sin_salida: u64,
    /// Lo mismo en las plazas con algún recinto: la red se recalcula con el muro puesto y una pieza anterior puede quedarse sin corredor dentro del
    /// cap (Trazado_Urbano §5.1). Se cuenta aparte, fuera de [`Self::total`]: no es un fallo del código sino un hueco de las reglas (ver el informe).
    pub sin_salida_muralla: u64,
    /// Piezas sin su edificio (mismo id y tipo) o edificios con huella en la rejilla sin pieza.
    pub economia: u64,
    /// Piezas sobre una celda de muro o torre de un recinto, o ciudades cuyos recintos no son la proyección de los de la economía (Murallas §3).
    pub muro: u64,
}

impl ViolacionesCiudad {
    pub fn total(&self) -> u64 {
        self.solape + self.suelo + self.disco + self.sin_salida + self.economia + self.muro
    }
}

pub struct Corrida {
    pub villas: Vec<Villa>,
    pub recursos: Recursos,
    pub reclamos: Reclamos,
    pub violaciones: Violaciones,
    pub ciudades: ViolacionesCiudad,
    /// Plazas que se pidieron y no encontraron ningún punto fundable.
    pub sin_colocar: u32,
    pub pared: Duration,
    pub pasos: u64,
    /// Día de la foto de edificios intermedia (7, o el último si el escenario es más corto).
    pub dia_foto: u32,
}

fn terreno(m: &Mundo, p: Punto) -> Terreno {
    match m.terreno_en(p.x, p.y) {
        None => Terreno::FueraDelMapa,
        Some(c) if c as i32 == 1 => Terreno::Agua,
        Some(c) if c as i32 == 6 => Terreno::Cima,
        Some(_) if !m.transitable_en(p.x, p.y) => Terreno::NoTransitable,
        Some(_) => Terreno::Firme,
    }
}

/// Madera por minuto (milésimas) que daría el bosque a 10 u de `p` si se explotase entero: 5 × la suma de densidades de sus parcelas.
fn madera_potencial(mapa: &Mapa, p: Punto) -> Mil {
    5_000 * mapa.parcelas_en(p, mapa.reglas.techo(1)).iter().map(|&i| mapa.densidad_ppm[i as usize]).sum::<i64>() / 1_000_000
}

/// Reparte las plazas entre los campamentos por igual y elige el sitio de cada una (ver la cabecera del módulo).
fn colocar(mundo: &Mundo, mapa: &Mapa, plazas: u32, semilla: u64) -> (Vec<(Punto, bool)>, u32) {
    let mut rng = Rng::derivado(semilla, "poblacion");
    let r = &mapa.reglas;
    let dist = DistanciasDeFundacion { separacion_plazas: r.separacion_plazas, exclusion_campamento: r.exclusion_campamento };
    let (mut discos, mut puntos, mut sin_colocar) = (Vec::<Disco>::new(), Vec::new(), 0);
    for i in 0..plazas {
        let centro = mapa.campamentos[i as usize % mapa.campamentos.len()];
        let mut elegido: Option<(Punto, bool)> = None;
        'rondas: for _ in 0..MAX_RONDAS {
            let mut ambar: Vec<((u8, usize), Punto)> = Vec::new();
            for _ in 0..CANDIDATOS {
                let radio = (ANILLO_CAMPAMENTO.0 + rng.por_debajo((ANILLO_CAMPAMENTO.1 - ANILLO_CAMPAMENTO.0) as u64) as i64) as f64;
                let ang = (rng.por_debajo(360_000) as f64 / 1000.0).to_radians();
                let p = Punto::new(centro.x + (radio * ang.cos()) as i64, centro.y + (radio * ang.sin()) as i64);
                if territorio::validar(p, terreno(mundo, p), &discos, mapa.campamentos.iter().copied(), &dist).is_err() {
                    continue;
                }
                // No se funda a ciegas (decidido el 2026-10-18): sin madera potencial suficiente a 10 u (`MADERA_MINIMA_POTENCIAL`) la plaza cae en ruinas.
                if madera_potencial(mapa, p) < nucleo::economia::MADERA_MINIMA_POTENCIAL {
                    continue;
                }
                let (parcelas, piedra) = (mapa.parcelas_en(p, r.radio_inicial).len(), mapa.piedra_en(p, r.radio_inicial).len());
                if territorio::viabilidad(true, parcelas, piedra) == Viabilidad::Verde {
                    elegido = Some((p, true));
                    break 'rondas;
                }
                ambar.push((((parcelas > 0) as u8 + (piedra > 0) as u8, parcelas + piedra), p));
            }
            if let Some(&(_, p)) = ambar.iter().max_by_key(|(k, _)| *k) {
                elegido = Some((p, false));
                break;
            }
        }
        match elegido {
            Some((p, verde)) => {
                discos.push(Disco { centro: p, radio: r.radio_inicial, faccion: i + 1 });
                puntos.push((p, verde));
            }
            None => sin_colocar += 1,
        }
    }
    (puntos, sin_colocar)
}

fn contar(a: &Asentamiento) -> BTreeMap<Tipo, u32> {
    let mut m = BTreeMap::new();
    a.edificios.iter().filter(|e| e.activo()).for_each(|e| *m.entry(e.tipo).or_insert(0) += 1);
    m
}

/// Un minuto de una plaza: el paso del asentamiento, la política del jugador, la zona y los invariantes.
#[allow(clippy::too_many_arguments)]
fn paso(v: &mut Villa, indice: u32, minuto: i64, re: &ReglasEconomia, mapa: &Mapa, recursos: &mut Recursos, reclamos: &mut Reclamos, viol: &mut Violaciones, murallas: bool) {
    let t: Instante = minuto * MS_POR_MINUTO;
    if murallas && minuto % CADA_MIN_ASCENSO == 0 {
        comprometer_muralla(v, t, minuto, re);
    }
    let Villa { pos, a, radio, parcelas, reg, urbe, .. } = v;
    let (reglas, urb) = (&*re.datos, &*re.urbanismo);
    let sin_tecnologia = |_: &str| false;
    let sin_bonus = |_: Recurso| UNO;
    let mut sitios = SitiosVilla { mapa, reclamos, villa: indice, centro: *pos, radio: *radio, parcelas, sin_sitio: &mut reg.sin_sitio, urbe, urbanismo: urb, ciudad: &mut reg.ciudad, minuto };
    let mut x = Exterior {
        reglas,
        fuentes: recursos,
        sitios: &mut sitios,
        fertilidad: UNO,
        tiene_tecnologia: &sin_tecnologia,
        factor_tecnologia: &sin_bonus,
        nivel_faccion: 1,
        orden_era: 1,
        es_capital: true,
        faccion_tiene_gran_fundicion: false,
        ciudadanos: 1,
        distancia_capital: 0,
        racion_guarnicion: 0,
        felicidad_extra: UNO,
        nobleza_extra: UNO,
        recaudacion_extra: UNO,
    };
    let (nivel_antes, nutricion_antes) = (a.nivel, a.poblacion.nutricion);
    let mut ev = a.avanzar_hasta(t, &mut x);
    reg.pasos += 1;

    // La política del jugador: cada hora, si nada lo bloquea, pide la subida. Cada plaza es de una Facción distinta (cupo ocupado 0).
    if minuto % CADA_MIN_ASCENSO == 0 && a.ascenso.is_none() {
        let bloqueos = a.bloqueos_de_ascenso(t, reglas, &x, 0);
        if a.nivel == 1 {
            let clases: Vec<usize> = bloqueos.iter().map(clase).collect();
            reg.comprobaciones_n1 += 1;
            for k in 0..7 {
                if clases.contains(&k) {
                    reg.bloqueos_horas[k] += 1;
                } else {
                    reg.libre_de[k].get_or_insert(minuto);
                }
            }
            reg.ultimos_bloqueos = clases;
        }
        if bloqueos.is_empty() && a.solicitar_ascenso(t, reglas, &x, 0, &mut ev).is_ok() {
            reg.ascenso_pedido.get_or_insert(minuto);
        }
    }
    drop(x);

    for e in ev {
        match e {
            Evento::EdificioCompletado { tipo: Tipo::Lenera, .. } => drop(reg.primera_lenera.get_or_insert(minuto)),
            Evento::EdificioCompletado { tipo: Tipo::Cantera, .. } => drop(reg.primera_cantera.get_or_insert(minuto)),
            Evento::EdificioCompletado { tipo: Tipo::Corral, .. } => drop(reg.primera_corral.get_or_insert(minuto)),
            Evento::EdificioGratuito { tipo, .. } if urb.clase(tipo).is_some_and(|c| c.rol == Rol::AnclaSaturacion) => drop(reg.primera_ancla.get_or_insert(minuto)),
            Evento::RecintoCompletado { .. } => {
                reg.recinto_completado.get_or_insert(minuto);
                sincronizar_recintos(a, urbe);
            }
            Evento::NivelSubio { nivel: 2 } => drop(reg.nivel2.get_or_insert(minuto)),
            Evento::NivelSubio { nivel: 3 } => drop(reg.nivel3.get_or_insert(minuto)),
            Evento::Hambruna { pesants, artesanos, nobleza } => reg.muertes += pesants + artesanos + nobleza,
            Evento::MantenimientoColapsado { .. } => reg.colapsos += 1,
            Evento::Ruinas { faltantes } => {
                reg.ruinas.get_or_insert(minuto);
                reg.ruinas_faltantes = faltantes;
            }
            _ => {}
        }
    }

    // La zona crece 1 u por cada edificio completado, con el techo del nivel alcanzado.
    let hechos = a.tomar_completados() as i64;
    *radio = (*radio + hechos * mapa.reglas.crecimiento_por_edificio).min(mapa.reglas.techo(a.nivel));

    // Medidas y invariantes.
    reg.min_deficit_trigo += (a.poblacion.nutricion < nutricion_antes) as u64;
    reg.nutricion_min = reg.nutricion_min.min(a.poblacion.nutricion);
    reg.medidor_min = reg.medidor_min.min(a.medidor.valor);
    reg.degradacion_max = reg.degradacion_max.max(a.nivel.saturating_sub(a.medidor.nivel_actual));
    reg.pesants_max = reg.pesants_max.max(a.poblacion.pesants);
    if a.poblacion.pesants >= 200 {
        reg.pesants_200.get_or_insert(minuto);
    }
    // Fuera de [0, capacidad], o con una fracción (C3b: el almacén guarda unidades enteras y un resto oculto en (−1000, 1000)).
    let (cantidad, _, resto) = a.almacen.partes();
    viol.almacen += TODOS.iter().filter(|&&r| a.almacen.cantidad(r) < 0 || a.almacen.disponible(r) > a.almacen.capacidad(r) || cantidad[r.indice()] % 1000 != 0 || !(-999..1000).contains(&resto[r.indice()]) || a.almacen.disponible(r) < 0).count() as u64;
    viol.poblacion += (a.poblacion.pesants < 0 || a.poblacion.artesanos < 0 || a.poblacion.nobleza < 0 || a.poblacion.nutricion < 0 || a.poblacion.nutricion > 100_000) as u64;
    viol.medidor += !(0..=100_000).contains(&a.medidor.valor) as u64;
    viol.nivel += (a.nivel < nivel_antes || a.medidor.nivel_actual > a.nivel) as u64;
}

/// La política de murallas del jugador, en su turno horario y **antes** del paso del minuto (la primera celda se levanta en ese mismo paso, como en el núcleo, que la
/// levanta en el instante de la orden): sin recintos y con el nivel operativo que pide la empalizada, la traza con la ciudad de la plaza y la compromete. Comprometer
/// es gratis; cada celda se paga al levantarla y respeta la reserva (Murallas §5).
fn comprometer_muralla(v: &mut Villa, t: Instante, minuto: i64, re: &ReglasEconomia) {
    let (a, reg) = (&mut v.a, &mut v.reg);
    let m = &re.datos.muralla;
    if !a.recintos.is_empty() || a.destruido || a.comprobar_comprometer(&re.datos, NIVEL_RECINTO).is_err() {
        return;
    }
    match trazado::trazar_recinto(v.urbe.ciudad(), &m.parametros_de_trazo(), NIVEL_RECINTO) {
        Ok(trazo) => {
            reg.recinto_coste = m.coste_de_trazo(&trazo.celdas, NIVEL_RECINTO);
            reg.recinto_celdas = trazo.celdas.len();
            reg.recinto_puertas = trazo.puertas();
            reg.recinto_comprometido = Some(minuto);
            a.comprometer_recinto(t, NIVEL_RECINTO, trazo.celdas, trazo.red_de_puertas, trazo.tras_pieza, &mut vec![]);
            sincronizar_recintos(a, &mut v.urbe);
        }
        Err(e) => reg.recinto_error = Some(e),
    }
}

/// La ciudad deriva sus recintos de los de la economía (`asentamiento::recintos::proyeccion`), como `Nucleo::sincronizar_recintos`: al comprometer y al cerrarse el anillo.
fn sincronizar_recintos(a: &Asentamiento, urbe: &mut Urbe) {
    let proyeccion = asentamiento::recintos::proyeccion(&a.recintos);
    if urbe.ciudad().recintos() != proyeccion.as_slice() {
        urbe.set_recintos(proyeccion);
    }
}

/// Edificios con fuente de todas las plazas: ¿alguna parcela o nodo aparece dos veces? ¿Cuadran con los reclamos?
fn comprobar_fuentes(villas: &[Villa], reclamos: &Reclamos, viol: &mut Violaciones) {
    let mut vistas = HashSet::new();
    for v in villas {
        for e in &v.a.edificios {
            if let Some(f) = e.fuente {
                if !vistas.insert(match f {
                    Fuente::Parcela(p) => (0u8, p),
                    Fuente::Nodo(n) => (1u8, n),
                }) {
                    viol.duplicados += 1;
                }
            }
        }
    }
    let reclamados = reclamos.parcela.iter().flatten().count() + reclamos.nodo.iter().flatten().count();
    viol.reclamos += reclamados.abs_diff(vistas.len()) as u64;
}

/// Semidiagonal de una huella en milésimas de unidad local (la de `trazado`, que no la exporta).
fn media_diagonal_mil(r: &trazado::Rect) -> i64 {
    let (a, b) = (r.ancho as i128, r.alto as i128);
    base::isqrt((a * a + b * b) * 1500 * 1500) as i64
}

/// Los invariantes de §13 de `Trazado_Urbano.md` y el acuerdo ciudad-economía, en todas las plazas que siguen en pie.
fn comprobar_ciudades(villas: &[Villa], re: &ReglasEconomia, out: &mut ViolacionesCiudad) {
    for v in villas.iter().filter(|v| !v.a.destruido) {
        out.plazas += 1;
        let c = v.urbe.ciudad();
        let prm = c.params();
        let radio = v.urbe.radio_ul(&v.a.urbano(&re.datos)) as i64;
        let (afueras_min, banda) = (prm.radio_afueras_min_ul as i64, prm.ancho_banda_afueras_ul as i64);
        let ps = c.piezas();
        let muros: HashSet<trazado::Celda> = c.recintos().iter().flat_map(|r| r.muros.iter().copied()).collect();
        out.muro += (c.recintos() != asentamiento::recintos::proyeccion(&v.a.recintos).as_slice()) as u64;
        for (k, p) in ps.iter().enumerate() {
            out.muro += p.rect.celdas().any(|cl| muros.contains(&cl)) as u64;
            out.solape += ps[..k].iter().filter(|q| q.rect.solapa(&p.rect)).count() as u64;
            out.suelo += p.rect.celdas().any(|cl| c.es_calle(cl) || c.es_camino(cl)) as u64;
            let centro2 = p.rect.dist2_centro_al_origen();
            let dentro = match p.categoria {
                // T §6.3: el campo vive en la banda de las afueras, a >= `radioAfuerasMin` del centro (borde) y hasta `max(radio, 60 + 36)` más la semidiagonal.
                Categoria::Afueras => {
                    let tope = radio.max(afueras_min + banda) * 1000 + media_diagonal_mil(&p.rect);
                    centro2 <= tope * tope && p.rect.dist2_borde_al_origen() >= (afueras_min * 1000).pow(2)
                }
                Categoria::Generico => centro2 <= (radio * 1000).pow(2),
                _ => true,
            };
            out.disco += !dentro as u64;
            let sin_salida = !p.rect.adyacentes().into_iter().any(|cl| c.es_calle(cl) || c.es_camino(cl)) as u64;
            if c.recintos().is_empty() {
                out.sin_salida += sin_salida;
            } else {
                out.sin_salida_muralla += sin_salida;
            }
            out.economia += !v.a.edificio(p.id).is_some_and(|e| e.tipo == TIPOS[p.tipo as usize]) as u64;
        }
        out.economia += v.a.edificios.iter().filter(|e| re.urbanismo.clase(e.tipo).is_some() && c.pieza(e.id).is_none()).count() as u64;
    }
}

/// Corre `plazas` plazas durante `dias` días de mundo.
/// Con `murallas`, el jugador compromete la empalizada en cuanto puede ([`NIVEL_RECINTO`]).
pub fn simular(mundo: &Mundo, mapa: &Arc<Mapa>, plazas: u32, dias: u32, semilla: u64, murallas: bool) -> Corrida {
    let re = ReglasEconomia::default();
    let reglas = &*re.datos;
    let (puntos, sin_colocar) = colocar(mundo, mapa, plazas, semilla);
    let dia_foto = dias.min(7);
    let mut villas: Vec<Villa> = puntos
        .into_iter()
        .enumerate()
        .map(|(i, (pos, verde))| {
            let a = Asentamiento::fundar(reglas, 0, &reglas.materiales_iniciales, reglas.viviendas_iniciales);
            // La ciudad de cada plaza: semilla determinista del escenario y del indice de la plaza, con los mismos edificios iniciales que la economia.
            let urbe = Urbe::fundar(&re, semilla, i as u64);
            let reg = Registro { nutricion_min: a.poblacion.nutricion, medidor_min: a.medidor.valor, pesants_dia: vec![a.poblacion.pesants], viviendas_dia: vec![reglas.viviendas_iniciales], ..Registro::default() };
            Villa { pos, verde, parcelas_n1: mapa.parcelas_en(pos, mapa.reglas.techo(1)).len(), densidad_n1: mapa.parcelas_en(pos, mapa.reglas.techo(1)).iter().map(|&i| mapa.densidad_ppm[i as usize]).sum(), piedra_n1: mapa.piedra_en(pos, mapa.reglas.techo(1)).len(), a, urbe, radio: mapa.reglas.radio_inicial, parcelas: BTreeSet::new(), reg }
        })
        .collect();
    let (mut recursos, mut reclamos, mut viol) = (Recursos::nuevo(mapa), Reclamos::nuevo(mapa), Violaciones::default());
    let t0 = Instant::now();
    for minuto in 1..=dias as i64 * MIN_POR_DIA {
        recursos.ahora = minuto * MS_POR_MINUTO;
        for (i, v) in villas.iter_mut().enumerate().filter(|(_, v)| !v.a.destruido) {
            paso(v, i as u32, minuto, &re, mapa, &mut recursos, &mut reclamos, &mut viol, murallas);
        }
        if minuto % MIN_POR_DIA == 0 {
            let dia = (minuto / MIN_POR_DIA) as u32;
            for v in villas.iter_mut() {
                v.reg.pesants_dia.push(v.a.poblacion.pesants);
                v.reg.viviendas_dia.push(v.a.edificios.iter().filter(|e| e.activo() && e.tipo == Tipo::Vivienda).count() as u32);
                if dia == dia_foto {
                    v.reg.edificios_a = contar(&v.a);
                    v.reg.piezas_a = v.urbe.ciudad().piezas().len();
                }
                if dia == dias {
                    v.reg.edificios_fin = contar(&v.a);
                    v.reg.piezas_fin = v.urbe.ciudad().piezas().len();
                }
            }
        }
    }
    let pared = t0.elapsed();
    comprobar_fuentes(&villas, &reclamos, &mut viol);
    let mut ciudades = ViolacionesCiudad::default();
    comprobar_ciudades(&villas, &re, &mut ciudades);
    let pasos = villas.iter().map(|v| v.reg.pasos).sum();
    Corrida { villas, recursos, reclamos, violaciones: viol, ciudades, sin_colocar, pared, pasos, dia_foto }
}

/// Hash del estado final de todas las plazas y del mapa (para comprobar el determinismo).
pub fn hash_estado(c: &Corrida) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for v in &c.villas {
        let (a, p) = (&v.a, &v.a.poblacion);
        (v.pos.x, v.pos.y, v.radio, a.nivel, a.medidor.valor, a.medidor.nivel_actual, p.pesants, p.artesanos, p.nobleza, p.nutricion, a.destruido, a.ascenso.map(|s| s.completa_en)).hash(&mut h);
        (p.restos, p.recuperando).hash(&mut h);
        a.recintos.iter().for_each(|r| (r.id, r.nivel, r.avance, r.siguiente_celda_en, r.completado_en, r.celdas.len()).hash(&mut h));
        a.almacen.partes().hash(&mut h);
        a.edificios.iter().for_each(|e| (e.id, e.tipo, e.estado as u8, e.nivel_interno, e.completa_en, e.fuente.map(|f| format!("{f:?}"))).hash(&mut h));
        v.parcelas.iter().for_each(|p| p.hash(&mut h));
        v.urbe.ciudad().piezas().iter().for_each(|p| (p.id, p.tipo, p.rect.col, p.rect.row, p.rect.ancho, p.rect.alto, p.rotado).hash(&mut h));
    }
    c.recursos.estado().for_each(|s| s.hash(&mut h));
    c.reclamos.parcela.hash(&mut h);
    c.reclamos.nodo.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use asentamiento::edificio::{Edificio, Estado};
    use trazado::{Ciudad, Pieza, Rect};

    fn villa(re: &ReglasEconomia) -> Villa {
        let r = &re.datos;
        let a = Asentamiento::fundar(r, 0, &r.materiales_iniciales, r.viviendas_iniciales);
        Villa { pos: Punto::new(0, 0), verde: true, parcelas_n1: 0, densidad_n1: 0, piedra_n1: 0, a, urbe: Urbe::fundar(re, 42, 0), radio: 6_000, parcelas: BTreeSet::new(), reg: Registro::default() }
    }

    fn comprobar(v: &Villa, re: &ReglasEconomia) -> ViolacionesCiudad {
        let mut out = ViolacionesCiudad::default();
        comprobar_ciudades(std::slice::from_ref(v), re, &mut out);
        out
    }

    #[test]
    fn los_invariantes_de_la_ciudad_detectan_lo_que_se_tuerce() {
        let re = ReglasEconomia::default();
        let mut v = villa(&re);
        assert_eq!(comprobar(&v, &re), ViolacionesCiudad { plazas: 1, ..Default::default() }, "una plaza recién fundada cumple todo");
        // Un Almacén (genérico) en la economía y en la ciudad, pero lejos del disco y de cualquier calle.
        let c = v.urbe.ciudad();
        let mut piezas = c.piezas().to_vec();
        let pieza = |id, col| Pieza { id, tipo: Tipo::Almacen as u8 as u16, categoria: Categoria::Generico, rol: Rol::Normal, rect: Rect::new(col, 0, 1, 1), rotado: false, ancla_llena: false, semilla_saturada: false };
        piezas.push(pieza(50, 60));
        v.urbe = Urbe::desde_ciudad(Ciudad::desde_partes(c.semilla(), *c.params(), trazado::LAYOUT_VERSION, piezas, vec![]).unwrap());
        v.a.edificios.push(Edificio::nuevo(50, Tipo::Almacen, Estado::Activo));
        let o = comprobar(&v, &re);
        assert_eq!((o.disco, o.sin_salida, o.solape, o.suelo, o.economia), (1, 1, 0, 0, 0), "{o:?}");
        // Sin su edificio, la pieza desacuerda con la economía.
        v.a.edificios.pop();
        assert_eq!(comprobar(&v, &re).economia, 1);
    }

    #[test]
    fn el_jugador_compromete_la_empalizada_en_el_nivel_2_y_la_ciudad_la_proyecta() {
        let re = ReglasEconomia::default();
        let mut v = villa(&re);
        comprometer_muralla(&mut v, 60_000, 1, &re);
        assert!(v.a.recintos.is_empty() && v.reg.recinto_comprometido.is_none(), "en el nivel 1 no se puede");
        (v.a.nivel, v.a.medidor.nivel_actual) = (2, 2);
        comprometer_muralla(&mut v, 60_000, 1, &re);
        assert_eq!((v.a.recintos.len(), v.reg.recinto_comprometido), (1, Some(1)));
        assert!(v.reg.recinto_celdas > 0 && v.reg.recinto_puertas > 0 && !v.reg.recinto_coste.is_empty());
        assert_eq!(v.urbe.ciudad().recintos(), asentamiento::recintos::proyeccion(&v.a.recintos).as_slice());
        assert_eq!(comprobar(&v, &re).total(), 0);
        comprometer_muralla(&mut v, 120_000, 2, &re);
        assert_eq!(v.a.recintos.len(), 1, "con un recinto en obra no se compromete otro");
        // Una ciudad que no lleva los recintos de la economía se detecta.
        v.urbe.set_recintos(vec![]);
        assert_eq!(comprobar(&v, &re).muro, 1);
    }
}
