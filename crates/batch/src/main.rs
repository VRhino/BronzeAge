//! `batch territorio [--mundo <.bin>] [--facciones 400] [--dias 30] [--semilla 42] [--salida batch/salida/territorio] [--decisiones <.json>]`
//! `batch poblacion  [--mundo <.bin>] [--plazas 300]    [--dias 30] [--semilla 42] [--salida batch/salida/poblacion]  [--decisiones <.json>]`

use batch::{artefacto_en, cargar, carpeta_por_defecto, escribir, fundir_decisiones, poblacion_y_ciudad, resumen_en_texto, territorio, Escenario, EscenarioPoblacion, Informe};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq)]
enum Sistema {
    Territorio,
    Poblacion,
}

struct Opciones {
    mundo: Option<PathBuf>,
    /// Facciones (territorio) o plazas (población).
    cantidad: u32,
    dias: u32,
    semilla: u64,
    salida: PathBuf,
    decisiones: Option<PathBuf>,
}

fn opciones(sistema: Sistema, args: &[String]) -> Result<Opciones, String> {
    let (cantidad, salida) = match sistema {
        Sistema::Territorio => (Escenario::default().facciones, "batch/salida/territorio"),
        Sistema::Poblacion => (EscenarioPoblacion::default().plazas, "batch/salida/poblacion"),
    };
    let nombre_cantidad = if sistema == Sistema::Territorio { "--facciones" } else { "--plazas" };
    let mut o = Opciones { mundo: None, cantidad, dias: 30, semilla: 42, salida: PathBuf::from(salida), decisiones: None };
    let mut it = args.iter();
    while let Some(clave) = it.next() {
        let valor = it.next().ok_or(format!("falta el valor de {clave}"))?;
        let numero = |v: &String| v.parse::<u64>().map_err(|_| format!("{clave} espera un número, no «{v}»"));
        match clave.as_str() {
            "--mundo" => o.mundo = Some(valor.into()),
            c if c == nombre_cantidad => o.cantidad = numero(valor)? as u32,
            "--dias" => o.dias = numero(valor)? as u32,
            "--semilla" => o.semilla = numero(valor)?,
            "--salida" => o.salida = valor.into(),
            "--decisiones" => o.decisiones = Some(valor.into()),
            otro => return Err(format!("opción desconocida: {otro}")),
        }
    }
    Ok(o)
}

fn ejecutar(sistema: Sistema, args: &[String]) -> Result<(), String> {
    let o = opciones(sistema, args)?;
    let ruta = match o.mundo {
        Some(r) => r,
        None => artefacto_en(&carpeta_por_defecto())?,
    };
    let t0 = Instant::now();
    println!("Mundo: {}", ruta.display());
    let mundo = cargar(&ruta)?;
    let mut ciudad_svg = None;
    let informe: Informe = match sistema {
        Sistema::Territorio => {
            println!("Cargado en {:.1} s; simulando {} Facciones durante {} días...", t0.elapsed().as_secs_f64(), o.cantidad, o.dias);
            territorio(&mundo, &Escenario { facciones: o.cantidad, dias: o.dias, semilla: o.semilla, ..Escenario::default() })
        }
        Sistema::Poblacion => {
            println!("Cargado en {:.1} s; simulando {} plazas durante {} días...", t0.elapsed().as_secs_f64(), o.cantidad, o.dias);
            let (informe, svg) = poblacion_y_ciudad(&mundo, &EscenarioPoblacion { plazas: o.cantidad, dias: o.dias, semilla: o.semilla, ..EscenarioPoblacion::default() });
            ciudad_svg = svg;
            informe
        }
    };
    let mut informe = informe;
    if let Some(d) = o.decisiones {
        let json = std::fs::read_to_string(&d).map_err(|e| format!("no se puede leer {}: {e}", d.display()))?;
        fundir_decisiones(&mut informe, &json)?;
    }
    escribir(&informe, &o.salida)?;
    if let Some(svg) = ciudad_svg {
        std::fs::write(o.salida.join("ciudad_ejemplo.svg"), svg).map_err(|e| e.to_string())?;
    }
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
        Some((cmd, resto)) if cmd == "territorio" => ejecutar(Sistema::Territorio, resto),
        Some((cmd, resto)) if cmd == "poblacion" => ejecutar(Sistema::Poblacion, resto),
        _ => Err("uso: batch territorio|poblacion [--mundo <.bin>] [--facciones|--plazas N] [--dias 30] [--semilla 42] [--salida <dir>] [--decisiones <.json>]".into()),
    };
    match resultado {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
