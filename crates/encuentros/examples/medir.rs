//! Medición: 10.000 móviles con planes de 20 tramos en el mundo real (2.667.000 × 2.000.000 milésimas).
//! `CARGO_TARGET_DIR=... cargo run --release -p encuentros --example medir [lado_zona_en_milesimas]`
use base::{Plan, Punto, Rng, Vertice};
use encuentros::{Entradas, Rejilla};
use std::time::Instant;

fn plan(rng: &mut Rng, zona: (i64, i64), tramos: usize) -> Plan {
    let mut t = rng.por_debajo(600_000) as i64; // arrancan en la próxima hora
    let mut p = Punto::new(rng.por_debajo(zona.0 as u64) as i64, rng.por_debajo(zona.1 as u64) as i64);
    let mut v = vec![Vertice { p, en: t }];
    for _ in 0..tramos {
        // ~0,5 mil/ms (velocidad 30), tramos de 5.000 a 60.000 milésimas en cualquier dirección, sin salirse de la zona
        let (dx, dy) = (rng.por_debajo(110_000) as i64 - 55_000, rng.por_debajo(110_000) as i64 - 55_000);
        p = Punto::new((p.x + dx).clamp(0, zona.0 - 1), (p.y + dy).clamp(0, zona.1 - 1));
        t += (v.last().unwrap().p.dist(p) * 2).max(1);
        v.push(Vertice { p, en: t });
    }
    Plan::nuevo(1, v)
}

fn main() {
    let lado: Option<i64> = std::env::args().nth(1).and_then(|a| a.parse().ok());
    let zona = lado.map_or((2_667_000, 2_000_000), |l| (l, l));
    let mut rng = Rng::derivado(7, "medir");
    let planes: Vec<Plan> = (0..10_000).map(|_| plan(&mut rng, zona, 20)).collect();
    let duracion = planes.iter().map(|p| p.fin().unwrap() - p.inicio().unwrap()).sum::<i64>() / planes.len() as i64;
    println!("zona {}×{} · 10.000 móviles · 20 tramos · duración media del plan {} min", zona.0, zona.1, duracion / 60_000);

    let mut rej = Rejilla::para_mundo_real();
    let t = Instant::now();
    for (e, p) in planes.iter().enumerate() {
        rej.apuntar(e as u64, 1, p);
    }
    println!("indexar los 10.000 planes: {:?} ({:?} por plan)", t.elapsed(), t.elapsed() / 10_000);

    // 1.000 planes nuevos: reapuntar (quitar + meter) y calcular sus encuentros
    let (mut n_ap, mut n_enc, mut cands) = (std::time::Duration::ZERO, std::time::Duration::ZERO, 0usize);
    let mut total_enc = 0usize;
    for e in 0..1_000u64 {
        let nuevo = plan(&mut rng, zona, 20);
        let t = Instant::now();
        rej.apuntar(e, 2, &nuevo);
        n_ap += t.elapsed();
        let t = Instant::now();
        let enc = rej.encuentros_de(e, &nuevo, 3_000, Entradas::Todas);
        n_enc += t.elapsed();
        cands += rej.candidatos(e).len();
        total_enc += enc.len();
    }
    println!("plan nuevo: reapuntar {:?} · encuentros {:?} (media de 1.000) · candidatas {:.1} · encuentros {:.2}",
        n_ap / 1_000, n_enc / 1_000, cands as f64 / 1_000.0, total_enc as f64 / 1_000.0);
}
