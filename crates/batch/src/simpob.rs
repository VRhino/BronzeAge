//! La población sintética del sistema «población, construcción y mantenimiento»: N asentamientos fundados a la vez sobre el mundo real
//! que viven `dias` días minuto a minuto con la política más simple posible (pedir la subida cada hora si nada la bloquea). Un bucle
//! global por minuto avanza todos, así que compiten de verdad por los nodos de piedra y las parcelas de bosque.

use crate::economia::{Mapa, Reclamos, Recursos, SinSitio, SitiosVilla};
use asentamiento::asentamiento::{Asentamiento, Evento, Exterior};
use asentamiento::ascenso::Bloqueo;
use asentamiento::catalogo::Tipo;
use asentamiento::datos::{Reglas, UNO};
use asentamiento::edificio::Fuente;
use asentamiento::recurso::{Recurso, TODOS};
use base::{Instante, Mil, Punto, Rng, MS_POR_MINUTO};
use mundo::Mundo;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};
use territorio::{Disco, DistanciasDeFundacion, Terreno, Viabilidad};

const MIN_POR_DIA: i64 = 1440;
/// Cada cuántos minutos el «jugador» sintético mira si puede subir de nivel.
pub const CADA_MIN_ASCENSO: i64 = 60;
/// Candidatos por ronda y rondas máximas para colocar una plaza (si ninguno es fundable, otra ronda).
pub const CANDIDATOS: usize = 25;
pub const MAX_RONDAS: usize = 40;
/// Anillo de búsqueda alrededor del campamento (milésimas): 30-120 u.
pub const ANILLO_CAMPAMENTO: (i64, i64) = (30_000, 120_000);

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
    /// Habitantes muertos de hambre, en milésimas.
    pub muertes: Mil,
    pub min_deficit_trigo: u64,
    pub pasos: u64,
    pub nutricion_min: Mil,
    pub medidor_min: Mil,
    /// Cuánto bajó como mucho el nivel operativo respecto del alcanzado (0 = nunca).
    pub degradacion_max: u32,
    pub colapsos: u32,
    pub pesants_max: Mil,
    pub pesants_200: Option<i64>,
    /// Pesants al final de cada día (milésimas), del 0 al último.
    pub pesants_dia: Vec<Mil>,
    pub viviendas_dia: Vec<u32>,
    /// Edificios activos por tipo en cada día de foto (el de la primera foto y el final).
    pub edificios_a: BTreeMap<Tipo, u32>,
    pub edificios_fin: BTreeMap<Tipo, u32>,
    pub sin_sitio: SinSitio,
    /// En cuántas comprobaciones horarias (con el nivel 1) pesó cada bloqueo.
    pub bloqueos_horas: [u32; 7],
    pub comprobaciones_n1: u32,
    /// Minuto en que por primera vez dejó de pesar cada bloqueo.
    pub libre_de: [Option<i64>; 7],
    pub ultimos_bloqueos: Vec<usize>,
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

pub struct Corrida {
    pub villas: Vec<Villa>,
    pub recursos: Recursos,
    pub reclamos: Reclamos,
    pub violaciones: Violaciones,
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
                // No se funda a ciegas (decidido el 2026-10-18): sin una parcela de bosque a 10 u o menos la plaza cae en ruinas.
                if mapa.parcelas_en(p, r.techo(1)).is_empty() {
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
fn paso(v: &mut Villa, indice: u32, minuto: i64, reglas: &Reglas, mapa: &Mapa, recursos: &mut Recursos, reclamos: &mut Reclamos, viol: &mut Violaciones) {
    let t: Instante = minuto * MS_POR_MINUTO;
    let Villa { pos, a, radio, parcelas, reg, .. } = v;
    let sin_tecnologia = |_: &str| false;
    let sin_bonus = |_: Recurso| UNO;
    let mut sitios = SitiosVilla { mapa, reclamos, villa: indice, centro: *pos, radio: *radio, parcelas, sin_sitio: &mut reg.sin_sitio };
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
        recinto_nivel: 0,
        racion_guarnicion: 0,
        felicidad_extra: UNO,
        nobleza_extra: UNO,
        recaudacion_extra: UNO,
    };
    let (nivel_antes, nutricion_antes) = (a.nivel, a.poblacion.nutricion);
    let mut ev = a.avanzar_minuto(t, &mut x);
    reg.pasos += 1;

    // La política del jugador: cada hora, si nada lo bloquea, pide la subida. Cada plaza es de una Facción distinta (cupo ocupado 0).
    if minuto % CADA_MIN_ASCENSO == 0 && a.ascenso.is_none() {
        let bloqueos = a.bloqueos_de_ascenso(reglas, &x, 0, &[]);
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
        if bloqueos.is_empty() && a.solicitar_ascenso(t, reglas, &x, 0, &[], &mut ev).is_ok() {
            reg.ascenso_pedido.get_or_insert(minuto);
        }
    }
    drop(x);

    for e in ev {
        match e {
            Evento::EdificioCompletado { tipo: Tipo::Lenera, .. } => drop(reg.primera_lenera.get_or_insert(minuto)),
            Evento::EdificioCompletado { tipo: Tipo::Cantera, .. } => drop(reg.primera_cantera.get_or_insert(minuto)),
            Evento::EdificioCompletado { tipo: Tipo::Corral, .. } => drop(reg.primera_corral.get_or_insert(minuto)),
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
    if a.poblacion.pesants >= 200_000 {
        reg.pesants_200.get_or_insert(minuto);
    }
    viol.almacen += TODOS.iter().filter(|&&r| a.almacen.cantidad(r) < 0 || a.almacen.cantidad(r) > a.almacen.capacidad(r)).count() as u64;
    viol.poblacion += (a.poblacion.pesants < 0 || a.poblacion.artesanos < 0 || a.poblacion.nobleza < 0 || a.poblacion.nutricion < 0 || a.poblacion.nutricion > 100_000) as u64;
    viol.medidor += !(0..=100_000).contains(&a.medidor.valor) as u64;
    viol.nivel += (a.nivel < nivel_antes || a.medidor.nivel_actual > a.nivel) as u64;
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

/// Corre `plazas` plazas durante `dias` días de mundo.
pub fn simular(mundo: &Mundo, mapa: &Arc<Mapa>, plazas: u32, dias: u32, semilla: u64) -> Corrida {
    let reglas = Reglas::cargar();
    let (puntos, sin_colocar) = colocar(mundo, mapa, plazas, semilla);
        let dia_foto = dias.min(7);
    let mut villas: Vec<Villa> = puntos
        .into_iter()
        .map(|(pos, verde)| {
            let a = Asentamiento::fundar(&reglas, 0, &reglas.materiales_iniciales, reglas.viviendas_iniciales);
            let reg = Registro { nutricion_min: a.poblacion.nutricion, medidor_min: a.medidor.valor, pesants_dia: vec![a.poblacion.pesants], viviendas_dia: vec![reglas.viviendas_iniciales], ..Registro::default() };
            Villa { pos, verde, parcelas_n1: mapa.parcelas_en(pos, mapa.reglas.techo(1)).len(), densidad_n1: mapa.parcelas_en(pos, mapa.reglas.techo(1)).iter().map(|&i| mapa.densidad_ppm[i as usize]).sum(), piedra_n1: mapa.piedra_en(pos, mapa.reglas.techo(1)).len(), a, radio: mapa.reglas.radio_inicial, parcelas: BTreeSet::new(), reg }
        })
        .collect();
    let (mut recursos, mut reclamos, mut viol) = (Recursos::nuevo(mapa), Reclamos::nuevo(mapa), Violaciones::default());
    let t0 = Instant::now();
    for minuto in 1..=dias as i64 * MIN_POR_DIA {
        recursos.ahora = minuto * MS_POR_MINUTO;
        for (i, v) in villas.iter_mut().enumerate().filter(|(_, v)| !v.a.destruido) {
            paso(v, i as u32, minuto, &reglas, mapa, &mut recursos, &mut reclamos, &mut viol);
        }
        if minuto % MIN_POR_DIA == 0 {
            let dia = (minuto / MIN_POR_DIA) as u32;
            for v in villas.iter_mut() {
                v.reg.pesants_dia.push(v.a.poblacion.pesants);
                v.reg.viviendas_dia.push(v.a.edificios.iter().filter(|e| e.activo() && e.tipo == Tipo::Vivienda).count() as u32);
                if dia == dia_foto {
                    v.reg.edificios_a = contar(&v.a);
                }
                if dia == dias {
                    v.reg.edificios_fin = contar(&v.a);
                }
            }
        }
    }
    let pared = t0.elapsed();
    comprobar_fuentes(&villas, &reclamos, &mut viol);
    let pasos = villas.iter().map(|v| v.reg.pasos).sum();
    Corrida { villas, recursos, reclamos, violaciones: viol, sin_colocar, pared, pasos, dia_foto }
}

/// Hash del estado final de todas las plazas y del mapa (para comprobar el determinismo).
pub fn hash_estado(c: &Corrida) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for v in &c.villas {
        let (a, p) = (&v.a, &v.a.poblacion);
        (v.pos.x, v.pos.y, v.radio, a.nivel, a.medidor.valor, a.medidor.nivel_actual, p.pesants, p.artesanos, p.nobleza, p.nutricion, a.destruido, a.ascenso.map(|s| s.completa_en)).hash(&mut h);
        TODOS.iter().for_each(|&r| a.almacen.cantidad(r).hash(&mut h));
        a.edificios.iter().for_each(|e| (e.id, e.tipo, e.estado as u8, e.nivel_interno, e.completa_en, e.fuente.map(|f| format!("{f:?}"))).hash(&mut h));
        v.parcelas.iter().for_each(|p| p.hash(&mut h));
    }
    c.recursos.estado().for_each(|s| s.hash(&mut h));
    c.reclamos.parcela.hash(&mut h);
    c.reclamos.nodo.hash(&mut h);
    h.finish()
}
