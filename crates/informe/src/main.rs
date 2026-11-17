//! `informe <entrada.json> <salida.html>`

use std::process::ExitCode;

fn ejecutar(entrada: &str, salida: &str) -> Result<(), String> {
    let texto = std::fs::read_to_string(entrada).map_err(|e| format!("no se pudo leer {entrada}: {e}"))?;
    let informe: informe::Informe =
        serde_json::from_str(&texto).map_err(|e| format!("JSON no válido en {entrada}: {e}"))?;
    std::fs::write(salida, informe::renderizar(&informe))
        .map_err(|e| format!("no se pudo escribir {salida}: {e}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [entrada, salida] = args.as_slice() else {
        eprintln!("uso: informe <entrada.json> <salida.html>");
        return ExitCode::from(2);
    };
    match ejecutar(entrada, salida) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
