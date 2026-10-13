//! `batch territorio [--mundo <.bin>] [--facciones 400] [--dias 30] [--semilla 42] [--salida batch/salida/territorio] [--decisiones <.json>]`

use batch::{artefacto_en, cargar, carpeta_por_defecto, escribir, fundir_decisiones, resumen_en_texto, territorio, Escenario};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

struct Opciones {
    mundo: Option<PathBuf>,
    escenario: Escenario,
    salida: PathBuf,
    decisiones: Option<PathBuf>,
}

fn opciones(args: &[String]) -> Result<Opciones, String> {
    let mut o = Opciones { mundo: None, escenario: Escenario::default(), salida: PathBuf::from("batch/salida/territorio"), decisiones: None };
    let mut it = args.iter();
    while let Some(clave) = it.next() {
        let valor = it.next().ok_or(format!("falta el valor de {clave}"))?;
        let numero = |v: &String| v.parse::<u64>().map_err(|_| format!("{clave} espera un número, no «{v}»"));
        match clave.as_str() {
            "--mundo" => o.mundo = Some(valor.into()),
            "--facciones" => o.escenario.facciones = numero(valor)? as u32,
            "--dias" => o.escenario.dias = numero(valor)? as u32,
            "--semilla" => o.escenario.semilla = numero(valor)?,
            "--salida" => o.salida = valor.into(),
            "--decisiones" => o.decisiones = Some(valor.into()),
            otro => return Err(format!("opción desconocida: {otro}")),
        }
    }
    Ok(o)
}

fn ejecutar(args: &[String]) -> Result<(), String> {
    let o = opciones(args)?;
    let ruta = match o.mundo {
        Some(r) => r,
        None => artefacto_en(&carpeta_por_defecto())?,
    };
    let t0 = Instant::now();
    println!("Mundo: {}", ruta.display());
    let mundo = cargar(&ruta)?;
    println!("Cargado en {:.1} s; simulando {} Facciones durante {} días...", t0.elapsed().as_secs_f64(), o.escenario.facciones, o.escenario.dias);
    let mut informe = territorio(&mundo, &o.escenario);
    if let Some(d) = o.decisiones {
        let json = std::fs::read_to_string(&d).map_err(|e| format!("no se puede leer {}: {e}", d.display()))?;
        fundir_decisiones(&mut informe, &json)?;
    }
    escribir(&informe, &o.salida)?;
    println!("{}\n", resumen_en_texto(&informe));
    for a in &informe.anomalias {
        println!("ANOMALIA: {}", a.titulo);
    }
    println!("Informe en {} ({:.1} s en total)", o.salida.display(), t0.elapsed().as_secs_f64());
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let resultado = match args.split_first() {
        Some((cmd, resto)) if cmd == "territorio" => ejecutar(resto),
        _ => Err("uso: batch territorio [--mundo <.bin>] [--facciones 400] [--dias 30] [--semilla 42] [--salida <dir>] [--decisiones <.json>]".into()),
    };
    match resultado {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
