//! Mide eventos/s: `cargo run --release -p cola --example medir_cola`.
use base::Rng;
use cola::{Clave, Cola};
use std::time::Instant;

fn main() {
    const N: u64 = 1_000_000;
    let mut r = Rng::nuevo(1);
    let clave = Clave { entidad: 0, generacion: 0 };
    // A: 1M programar (instantes aleatorios) y luego 1M sacar.
    let mut c = Cola::nueva();
    let t = Instant::now();
    for i in 0..N {
        c.programar(r.por_debajo(10_000_000) as i64, 1 + r.por_debajo(10) as u8, clave, i);
    }
    let n = std::iter::from_fn(|| c.siguiente_hasta(i64::MAX, &|_| true)).count();
    let s = t.elapsed().as_secs_f64();
    println!("lote:   {n} eventos programar+sacar en {s:.3} s = {:.2} M eventos/s", N as f64 / s / 1e6);
    // B: régimen estacionario con ~100k eventos en vuelo.
    let mut c = Cola::nueva();
    for i in 0..100_000u64 {
        c.programar(r.por_debajo(1_000_000) as i64, 5, clave, i);
    }
    let t = Instant::now();
    for i in 0..N {
        let e = c.siguiente_hasta(i64::MAX, &|_| true).unwrap();
        c.programar(e.instante + 1 + r.por_debajo(1_000_000) as i64, 1 + r.por_debajo(10) as u8, clave, i);
    }
    let s = t.elapsed().as_secs_f64();
    println!("estac.: {N} sacar+programar con 100k en vuelo en {s:.3} s = {:.2} M eventos/s", N as f64 / s / 1e6);
}
