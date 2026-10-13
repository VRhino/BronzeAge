//! Mide el muestreo con 10.000 móviles y 200 Facciones en el mundo real (2.667 × 2.000 u):
//! `cargo run --release -p vision --example rendimiento [-- ruta/mundo.bin]`
//! Con ruta, la rejilla de la niebla sale del artefacto; sin ella, 667 × 500 (las del mundo real).
use base::{Plan, Punto, Rng, Vertice};
use std::time::Instant;
use vision::{diff, radios, Donde, Movil, Niebla, Observador, Vision};

fn main() {
    let mut niebla = match std::env::args().nth(1) {
        Some(ruta) => {
            let m = mundo::Mundo::cargar(&std::fs::read(ruta).unwrap(), &mundo::OpcionesValidacion::default()).unwrap();
            Niebla::para_mundo(&m)
        }
        None => Niebla::nueva(667, 500),
    };
    let (n_mov, n_fac, n_plazas) = (10_000usize, 200u32, 1_000usize);
    let mut rng = Rng::nuevo(42);
    let plan_al_azar = |rng: &mut Rng| {
        let (mut x, mut y, mut t) = (rng.por_debajo(2_667_000) as i64, rng.por_debajo(2_000_000) as i64, 0i64);
        let vs: Vec<_> = (0..4)
            .map(|_| {
                let v = Vertice { p: Punto::new(x, y), en: t };
                x = (x + rng.por_debajo(200_000) as i64 - 100_000).clamp(0, 2_667_000);
                y = (y + rng.por_debajo(200_000) as i64 - 100_000).clamp(0, 2_000_000);
                t += 600_000;
                v
            })
            .collect();
        Plan::nuevo(1, vs)
    };
    let planes: Vec<Plan> = (0..n_mov).map(|_| plan_al_azar(&mut rng)).collect();
    let plazas: Vec<Punto> = (0..n_plazas).map(|_| Punto::new(rng.por_debajo(2_667_000) as i64, rng.por_debajo(2_000_000) as i64)).collect();
    // Peor caso: cada móvil es además un ojo de ejército de su Facción; más 1.000 plazas.
    let mut obs: Vec<Observador> = planes.iter().enumerate().map(|(i, p)| Observador::movil(i as u32 % n_fac, p, radios::EJERCITO)).collect();
    obs.extend(plazas.iter().enumerate().map(|(i, &p)| Observador::fijo(i as u32 % n_fac, p, 20_000)));
    let mov: Vec<Movil> = planes.iter().enumerate().map(|(i, p)| Movil::new(i as u64, i as u32 % n_fac, Donde::Plan(p))).collect();
    let v = Vision::con_alianzas((0..n_fac / 2).map(|i| (i * 2, i * 2 + 1)));

    println!("hilos de rayon: {}", rayon::current_num_threads());
    let mut anterior = v.muestrear(0, &obs, &mov);
    for (nombre, f) in [
        ("muestrear (11.000 ojos × 10.000 móviles × 200 Facciones)", 0),
        ("grabar niebla (11.000 ojos)", 1),
        ("muestrear + diff", 2),
    ] {
        let mut ms = vec![];
        for k in 1..=10i64 {
            let t = k * 250;
            let i0 = Instant::now();
            match f {
                0 => drop(v.muestrear(t, &obs, &mov)),
                1 => niebla.grabar(t, &obs),
                _ => {
                    let m = v.muestrear(t, &obs, &mov);
                    let c = diff(&anterior, &m);
                    anterior = m;
                    std::hint::black_box(c);
                }
            }
            ms.push(i0.elapsed().as_secs_f64() * 1000.0);
        }
        ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("{nombre}: mediana {:.2} ms, máx {:.2} ms", ms[ms.len() / 2], ms[ms.len() - 1]);
    }
    let i0 = Instant::now();
    let p = v.niebla_proyectada(&niebla, 0, 2_500, &obs);
    println!("niebla_proyectada de una Facción: {:.2} ms ({} bytes de celdas)", i0.elapsed().as_secs_f64() * 1000.0, p.celdas.len());
}
