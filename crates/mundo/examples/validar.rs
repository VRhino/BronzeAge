//! Valida un artefacto de mundo con las reglas del servidor, y su paquete de relieve si está al lado (`relieve-<hash>.bin`):
//! `cargo run -p mundo --example validar -- ruta/mundo-<hash>.bin`
use mundo::{validar_relieve, Mundo, OpcionesValidacion};

fn main() {
    let ruta = std::path::PathBuf::from(std::env::args().nth(1).expect("uso: validar <artefacto.bin>"));
    let bytes = std::fs::read(&ruta).expect("no se pudo leer el artefacto");
    let m = match Mundo::cargar(&bytes, &OpcionesValidacion::default()) {
        Ok(m) => m,
        Err(e) => { eprintln!("RECHAZADO: {e}"); std::process::exit(1); }
    };
    println!(
        "OK {} ({} bytes): {} nodos de recurso, {} parcelas, {} campamentos, {} bandidos, {} alijos",
        m.hash(), bytes.len(), m.nodos().len(), m.parcelas().len(), m.campamentos().len(), m.bandidos_iniciales().len(), m.alijos().len()
    );
    let junto = ruta.with_file_name(format!("relieve-{}.bin", m.hash()));
    let Ok(relieve) = std::fs::read(&junto) else { return println!("sin paquete de relieve al lado ({})", junto.display()) };
    match validar_relieve(&relieve, m.hash(), m.tamano()) {
        Ok(h) => println!("OK relieve {h} ({} bytes)", relieve.len()),
        Err(e) => { eprintln!("RELIEVE RECHAZADO: {e}"); std::process::exit(1); }
    }
}
