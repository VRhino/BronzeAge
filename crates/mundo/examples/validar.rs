//! Valida un artefacto de mundo con las reglas del servidor: `cargo run -p mundo --example validar -- ruta/mundo-<hash>.bin`
use mundo::{Mundo, OpcionesValidacion};

fn main() {
    let ruta = std::env::args().nth(1).expect("uso: validar <artefacto.bin>");
    let bytes = std::fs::read(&ruta).expect("no se pudo leer el artefacto");
    match Mundo::cargar(&bytes, &OpcionesValidacion::default()) {
        Ok(m) => println!(
            "OK {} ({} bytes): {} nodos de recurso, {} parcelas, {} campamentos, {} bandidos, {} alijos",
            m.hash(), bytes.len(), m.nodos().len(), m.parcelas().len(), m.campamentos().len(), m.bandidos_iniciales().len(), m.alijos().len()
        ),
        Err(e) => { eprintln!("RECHAZADO: {e}"); std::process::exit(1); }
    }
}
