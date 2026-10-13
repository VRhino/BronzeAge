//! Demo del núcleo con el artefacto de mundo real: N columnas salen de los campamentos, marchan a destinos aleatorios
//! deterministas (rutas A* sobre el grafo del mundo), y al llegar reciben una orden nueva; 1 hora de mundo en modo
//! acelerado. Escribe un `demo.json` para `demo/visor` (formato en `demo/visor/FORMATO.md`) y mide el núcleo.
//!
//!   cargo run --release -p nucleo --example demo -- <artefacto.bin> <salida.json> [n_ejercitos=1000] [semilla=42]
//!
//! Cada 30 s de mundo el conductor lee del registro de cambios las llegadas y da órdenes de marcha (en lote, con las rutas
//! en paralelo); además redirige a mitad de camino a un 0,5 % de las columnas en marcha por paso (órdenes desde la posición
//! real). Todo entra por el mismo camino que una orden de un jugador: `Nucleo::marchas_en_lote`.

use base::{Instante, Punto, Rng};
use mundo::{Mundo, OpcionesValidacion, TAMANO_CELDA};
use nucleo::espacial::{GeneradorEncuentros, GeneradorVision};
use nucleo::{Ajustes, Arranque, Comando, EventoNucleo, Nucleo, Peticion, Reglas};
use rutas::Rutas;
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Instant as Cronometro;

const DURACION: Instante = 3_600_000;
const PASO: Instante = 30_000;
/// 18, 24, 30 y 42 unidades por minuto (pesada, media, ligera, montada) en milésimas por segundo.
const VELOCIDADES: [i64; 4] = [300, 400, 500, 700];
const COLORES: [&str; 4] = ["#3b82f6", "#ef4444", "#22a35a", "#d97706"];
const MAX_EVENTOS_JSON: usize = 6_000;
const RADIO: i64 = 300_000;
const RADIO_MIN: i64 = 60_000;

/// Punto transitable a entre 60 y 300 unidades de `desde` (rechazo con enteros: nada de coma flotante).
fn destino(mundo: &Mundo, rng: &mut Rng, desde: Punto) -> Punto {
    let (ancho, alto) = mundo.tamano();
    for _ in 0..500 {
        let (dx, dy) = (rng.por_debajo(2 * RADIO as u64 + 1) as i64 - RADIO, rng.por_debajo(2 * RADIO as u64 + 1) as i64 - RADIO);
        let d2 = dx * dx + dy * dy;
        let p = Punto::new(desde.x + dx, desde.y + dy);
        if (RADIO_MIN * RADIO_MIN..=RADIO * RADIO).contains(&d2) && p.x >= 0 && p.y >= 0 && p.x < ancho && p.y < alto && mundo.transitable_en(p.x, p.y) {
            return p;
        }
    }
    desde
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

fn escapar(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn texto_del_evento(e: &EventoNucleo) -> String {
    let nombre = format!("Ejército {}", e.ejercitos.first().copied().unwrap_or(0));
    match e.codigo.as_ref() {
        "ejercito.llega" => format!("{nombre} llega a su destino"),
        "ejercito.sin_viveres" => format!("{nombre} se queda sin víveres"),
        "ejercito.encuentro" => format!("Ejército {} y Ejército {} se encuentran", e.ejercitos[0], e.ejercitos.get(1).copied().unwrap_or(0)),
        "vision.contacto" => format!("La facción f{} ve por primera vez a la f{} ({nombre})", e.datos.first().map_or(0, |d| d.1) + 1, e.datos.get(1).map_or(0, |d| d.1) + 1),
        "ejercito.viveres_bajos" => format!("{nombre}: víveres bajos ({:.1})", e.datos.first().map_or(0, |d| d.1) as f64 / 1000.0),
        otro => format!("{nombre}: {otro}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("uso: demo <artefacto.bin> <salida.json> [n_ejercitos=1000] [semilla=42]");
        std::process::exit(2);
    }
    let n: usize = args.get(2).map_or(1000, |s| s.parse().expect("n_ejercitos"));
    let semilla: u64 = args.get(3).map_or(42, |s| s.parse().expect("semilla"));

    // ── Mundo y rutas ──
    let t = Cronometro::now();
    let bytes = std::fs::read(&args[0]).expect("leer el artefacto");
    let mundo = Mundo::cargar(&bytes, &OpcionesValidacion::default()).expect("artefacto válido");
    let t_carga = t.elapsed();
    let t = Cronometro::now();
    let rutas = Arc::new(Rutas::desde_mundo(&mundo));
    println!("mundo {}: cargar y validar {:?}; rutas ({} nodos, espaciado {}): {:?}", &mundo.hash()[..12], t_carga, rutas.nodos(), rutas.espaciado(), t.elapsed());

    // ── El núcleo y las columnas ──
    let ajustes = Ajustes { retencion_cambios: 1 << 20, muestreo_ms: 250, ..Ajustes::default() };
    let mut nucleo = Nucleo::nuevo(Arranque { reglas: Reglas::default(), ajustes, semilla, inicio: 0, rutas, mundo_hash: mundo.hash().to_owned(), mundo: None });
    // Encuentros exactos entre columnas y visión muestreada cada 250 ms (generadores espaciales del núcleo).
    nucleo.agregar_generador(Box::new(GeneradorEncuentros::nuevo()));
    nucleo.agregar_generador(Box::new(GeneradorVision::nuevo()));
    let campamentos: Vec<(String, Punto)> =
        mundo.campamentos().iter().map(|c| (c.id.clone(), c.posicion.map(|p| Punto::new(p.x, p.y)).expect("campamento con posición"))).collect();
    assert!(!campamentos.is_empty(), "el mundo no tiene campamentos");
    let mut rng = Rng::derivado(semilla, "demo");
    let mut ids = Vec::with_capacity(n);
    // Planes grabados para el visor: vértices (x, y, t) por columna.
    let mut grabado: Vec<Vec<(i64, i64, i64)>> = Vec::with_capacity(n);
    for i in 0..n {
        let (_, pos) = &campamentos[i % campamentos.len()];
        let velocidad = VELOCIDADES[rng.por_debajo(4) as usize];
        let viveres = 20_000 + rng.por_debajo(330_000) as i64; // algunas se quedan sin víveres durante la hora
        let id = nucleo
            .crear_ejercito(0, &format!("heroe-{i}"), &format!("bot-{i}"), (i % 4) as u32, *pos, velocidad, viveres, 350_000)
            .expect("alta");
        ids.push(id);
        grabado.push(vec![(pos.x, pos.y, 0)]);
    }

    // ── Conducción ──
    let (mut t_ordenes, mut t_bucle) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    let (mut ordenes, mut rechazadas, mut redirigidas) = (0u64, 0u64, 0u64);
    let mut eventos: Vec<EventoNucleo> = Vec::new();
    let mut visto = nucleo.version();
    let mut a_ordenar: Vec<u64> = ids.clone();
    for paso in 0..=DURACION / PASO {
        let t_mundo = paso * PASO;
        if paso > 0 {
            let c = Cronometro::now();
            nucleo.avanzar_hasta(t_mundo);
            t_bucle += c.elapsed();
            a_ordenar.clear();
            for cv in nucleo.cambios_desde(visto).expect("registro retenido") {
                for e in &cv.eventos {
                    if e.codigo == "ejercito.llega" {
                        a_ordenar.push(e.ejercitos[0]);
                    }
                    eventos.push(e.clone());
                }
            }
            visto = nucleo.version();
            // Redirecciones a mitad de camino.
            for &id in &ids {
                if nucleo.estado().ejercitos[&id].marchando() && rng.por_debajo(200) == 0 && !a_ordenar.contains(&id) {
                    a_ordenar.push(id);
                    redirigidas += 1;
                }
            }
        }
        if a_ordenar.is_empty() {
            continue;
        }
        let peticiones: Vec<Peticion> = a_ordenar
            .iter()
            .map(|&id| {
                let desde = nucleo.estado().ejercitos[&id].posicion_en(t_mundo);
                let comando = Comando::MarcharA { ejercito: id, destino: destino(&mundo, &mut rng, desde) };
                Peticion::nueva("demo", &format!("d{paso}-{id}"), comando).de_admin()
            })
            .collect();
        let c = Cronometro::now();
        let respuestas = nucleo.marchas_en_lote(t_mundo, peticiones);
        t_ordenes += c.elapsed();
        for (&id, r) in a_ordenar.iter().zip(&respuestas) {
            ordenes += 1;
            if !r.aceptada() {
                rechazadas += 1;
                continue;
            }
            let plan = &nucleo.estado().ejercitos[&id].plan;
            let g = &mut grabado[(id - 1) as usize];
            let t0 = plan.vertices[0].en;
            g.retain(|v| v.2 < t0); // lo que quedaba del plan anterior más allá de la orden ya no ocurre
            g.extend(plan.vertices.iter().map(|v| (v.p.x, v.p.y, v.en)));
        }
    }
    nucleo.avanzar_hasta(DURACION);
    for cv in nucleo.cambios_desde(visto).expect("registro retenido") {
        eventos.extend(cv.eventos.iter().cloned());
    }
    let m = nucleo.metricas().clone();

    // ── Informe ──
    let segundos = t_bucle.as_secs_f64();
    println!("{n} columnas, {} h de mundo en modo acelerado (semilla {semilla})", DURACION / 3_600_000);
    println!("órdenes: {ordenes} ({rechazadas} rechazadas), {redirigidas} redirecciones a mitad de camino; {} rutas A*, {:?} en total", m.rutas_calculadas, t_ordenes);
    println!(
        "bucle de eventos: {} eventos vigentes ({} descartados por invalidación) en {:.1} ms → {:.0} eventos/s; {} versiones de mundo",
        m.eventos,
        nucleo.eventos_descartados(),
        segundos * 1000.0,
        m.eventos as f64 / segundos.max(1e-9),
        nucleo.version()
    );
    println!("estado final: {} bytes serializados, hash {}", nucleo.estado().serializar().len(), nucleo::hex(&nucleo.hash_estado()));

    // ── demo.json ──
    // El visor lleva la cuenta de los vértices de cada plan: se recorta a la duración de la demo.
    let recortar = |v: &[(i64, i64, i64)]| -> Vec<(i64, i64, i64)> {
        let plan = base::Plan::nuevo(0, v.iter().map(|&(x, y, t)| base::Vertice { p: Punto::new(x, y), en: t }).collect());
        let mut r: Vec<_> = v.iter().copied().filter(|p| p.2 < DURACION).collect();
        let p = plan.posicion_en(DURACION).expect("plan no vacío");
        r.push((p.x, p.y, DURACION));
        r
    };
    let (ancho, alto) = mundo.tamano();
    let (cols, filas) = mundo.rejilla();
    let mut terreno = vec![0u8; cols * filas];
    let t = TAMANO_CELDA as i64;
    for f in 0..filas {
        for c in 0..cols {
            terreno[f * cols + c] = mundo.terreno_en(c as i64 * t + t / 2, f as i64 * t + t / 2).map_or(0, |k| k as i32 as u8);
        }
    }
    let mut j = String::new();
    write!(j, "{{\"titulo\":\"Núcleo Rust: {n} columnas, 1 h de mundo (modo acelerado)\",\"semilla\":{semilla},\"duracion_ms\":{DURACION},").unwrap();
    write!(j, "\"mundo\":{{\"ancho\":{ancho},\"alto\":{alto},\"terreno\":{{\"columnas\":{cols},\"filas\":{filas},\"base64\":\"{}\"}}}},", base64(&terreno)).unwrap();
    j.push_str("\"campamentos\":[");
    for (i, (id, p)) in campamentos.iter().enumerate() {
        write!(j, "{}{{\"id\":\"{}\",\"x\":{},\"y\":{}}}", if i > 0 { "," } else { "" }, escapar(id), p.x, p.y).unwrap();
    }
    j.push_str("],\"entidades\":[");
    for (i, &id) in ids.iter().enumerate() {
        let e = &nucleo.estado().ejercitos[&id];
        write!(j, "{}{{\"id\":\"ej-{id}\",\"nombre\":\"Ejército {id}\",\"faccion\":\"f{}\",\"color\":\"{}\",\"radio_vision\":30000,\"plan\":[", if i > 0 { "," } else { "" }, e.faccion + 1, COLORES[e.faccion as usize % 4]).unwrap();
        for (k, (x, y, t)) in recortar(&grabado[i]).iter().enumerate() {
            write!(j, "{}{{\"x\":{x},\"y\":{y},\"t\":{t}}}", if k > 0 { "," } else { "" }).unwrap();
        }
        j.push_str("]}");
    }
    // Se conservan todos los avisos de víveres y se muestrean las llegadas hasta no pasar de MAX_EVENTOS_JSON.
    let muestreable = |c: &str| c == "ejercito.llega" || c == "ejercito.encuentro";
    let llegadas = eventos.iter().filter(|e| muestreable(&e.codigo)).count();
    let resto = eventos.len() - llegadas;
    let cada = if eventos.len() > MAX_EVENTOS_JSON { llegadas.div_ceil(MAX_EVENTOS_JSON.saturating_sub(resto).max(1)) } else { 1 };
    j.push_str("],\"eventos\":[");
    let (mut k, mut escritos) = (0usize, 0usize);
    for e in &eventos {
        if muestreable(&e.codigo) {
            k += 1;
            if (k - 1) % cada != 0 {
                continue;
            }
        }
        let (x, y) = e.punto.map_or((0, 0), |p| (p.x, p.y));
        let quien = e.ejercitos.iter().map(|id| format!("\"ej-{id}\"")).collect::<Vec<_>>().join(",");
        write!(j, "{}{{\"t\":{},\"tipo\":\"{}\",\"texto\":\"{}\",\"x\":{x},\"y\":{y},\"entidades\":[{quien}]}}", if escritos > 0 { "," } else { "" }, e.en, escapar(&e.codigo), escapar(&texto_del_evento(e))).unwrap();
        escritos += 1;
    }
    j.push_str("]}");
    std::fs::write(&args[1], &j).expect("escribir demo.json");
    println!("{}: {:.1} MB, {} entidades, {escritos} de {} eventos (llegadas 1 de cada {cada})", args[1], j.len() as f64 / 1e6, ids.len(), eventos.len());
}
