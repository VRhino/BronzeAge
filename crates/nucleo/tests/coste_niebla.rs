//! Cuánto cuesta que los ojos graben lo que ven (4H T3d): el mundo real con nueve plazas y 300 columnas de 30 Facciones que marchan sin parar durante dos horas de mundo, con órdenes a cada rato
//! (cada orden y cada llegada tocan la columna y la hacen barrer lo andado). Solo usa la API del núcleo de antes de T3d, para medirse igual contra la base:
//! `cargo test --release -p nucleo --test coste_niebla -- --ignored --nocapture`. Se omite sin el artefacto de mundo real.

use base::{Punto, Rng, MS_POR_MINUTO};
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas};
use rutas::Rutas;
use std::sync::Arc;
use std::time::Instant;

fn mundo() -> Option<(Arc<Mundo>, Arc<Rutas>)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generador/salida/mundo");
    let f = std::fs::read_dir(dir).ok()?.flatten().find(|e| e.file_name().to_string_lossy().starts_with("mundo-"))?;
    let m = Mundo::cargar(&std::fs::read(f.path()).ok()?, &OpcionesValidacion::default()).expect("artefacto válido");
    let r = Arc::new(Rutas::desde_mundo(&m));
    Some((Arc::new(m), r))
}

/// Un punto en tierra al azar dentro de `r` milésimas de `c`.
fn en_tierra(m: &Mundo, rng: &mut Rng, c: Punto, r: i64) -> Punto {
    loop {
        let p = Punto::new(c.x + rng.por_debajo(2 * r as u64) as i64 - r, c.y + rng.por_debajo(2 * r as u64) as i64 - r);
        if m.transitable_en(p.x, p.y) {
            return p;
        }
    }
}

#[test]
#[ignore = "medición: cargo test --release -p nucleo --test coste_niebla -- --ignored --nocapture"]
fn coste_de_avanzar_dos_horas_con_300_columnas_que_marchan() {
    let Some(m) = mundo() else {
        eprintln!("sin artefacto de mundo real: se omite");
        return;
    };
    let mut reglas = Reglas::default();
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let mut n = Nucleo::nuevo(Arranque { reglas, ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
    let mut rng = Rng::nuevo(11);
    let campamentos: Vec<Punto> = m.0.campamentos().iter().map(|c| c.posicion.map(|p| Punto::new(p.x, p.y)).unwrap()).collect();
    // Una plaza por campamento, con su construcción automática en marcha (crecen, y con ellas su zona: lo que se ve de ellas cambia).
    for (i, &c) in campamentos.iter().enumerate() {
        let f = i as u32 + 1;
        let sitio = (0..8).map(|k| 50_000 + k * 10_000).flat_map(|r| (0..36).map(move |g| (r, g))).map(|(r, g): (i64, i64)| {
            let a = ((g * 10) as f64).to_radians();
            Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64)
        }).find(|&p| n.viabilidad(p, f).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6));
        let Some(sitio) = sitio else { continue };
        let col = n.crear_ejercito(0, &format!("c{}", f - 1), f, sitio, 500, 100_000).unwrap();
        assert!(n.aplicar_en(0, Peticion::nueva("admin", &format!("fc{i}"), Comando::DarCaravanaFundacion { ejercito: col }).de_admin()).aceptada());
        let plaza = n.aplicar_en(0, Peticion::nueva(&format!("c{}", f - 1), &format!("f{i}"), Comando::Fundar { ejercito: col })).creadas[0];
        for (k, r) in [asentamiento::Recurso::Madera, asentamiento::Recurso::Piedra, asentamiento::Recurso::Oro, asentamiento::Recurso::Trigo].into_iter().enumerate() {
            n.aplicar_en(0, Peticion::nueva("admin", &format!("d{i}-{k}"), Comando::DarRecursos { asentamiento: plaza, recurso: r, cantidad: 300_000 }).de_admin());
        }
    }
    let mut columnas = Vec::new();
    for i in 0..300u32 {
        let c = campamentos[i as usize % campamentos.len()];
        let p = en_tierra(&m.0, &mut rng, c, 150_000);
        columnas.push((n.crear_ejercito(0, &format!("c{}", i % 30), i % 30 + 1, p, 500, 100_000).unwrap(), c, format!("c{}", i % 30)));
    }
    let inicio = Instant::now();
    let mut ordenes = 0;
    for paso in 0..24i64 {
        let t = paso * 5 * MS_POR_MINUTO;
        n.avanzar_hasta(t);
        for (k, (col, c, cuenta)) in columnas.iter().enumerate().filter(|(k, _)| (*k as i64 + paso) % 3 == 0) {
            let destino = en_tierra(&m.0, &mut rng, *c, 150_000);
            ordenes += n.aplicar_en(t, Peticion::nueva(cuenta, &format!("o{paso}-{k}"), Comando::MarcharA { ejercito: *col, destino })).aceptada() as usize;
        }
    }
    n.avanzar_hasta(2 * 60 * MS_POR_MINUTO);
    let ms = inicio.elapsed().as_secs_f64() * 1000.0;
    println!("2 h de mundo con 300 columnas y {ordenes} órdenes: {ms:.0} ms; versión {}, hash {:02x?}", n.version(), &n.hash_estado()[..4]);
}
