//! `bronzeage-servidor`: el proceso del servidor (ver la cabecera de `lib.rs`). Se configura por entorno:
//!
//! | Variable | Qué | Por defecto |
//! |---|---|---|
//! | `BRONZEAGE_DIRECCION` | dónde escucha (WebSocket y HTTP en el mismo puerto) | `127.0.0.1:7878` |
//! | `BRONZEAGE_MUNDO` | el artefacto de mundo, o una carpeta con un `mundo-*.bin` | `generador/salida/mundo` |
//! | `BRONZEAGE_DATOS` | la carpeta de datos (`mundo/` y `acceso/`) | `datos` |
//! | `BRONZEAGE_SEMILLA` | la semilla de la partida (solo cuenta la primera vez) | `42` |
//! | `CODIGO_REGISTRO` | código de invitación de las cuentas humanas; sin él, registro abierto | — |
//! | `CODIGO_REGISTRO_BOTS` | código que marca la cuenta como bot; sin él, no hay bots | — |
//! | `ADMINISTRADORES` | nicks de los administradores globales, separados por comas | — |
//!
//! Ctrl+C para: cierra las sesiones y apaga el núcleo con su foto.

use std::{env, net::SocketAddr, path::PathBuf, process::ExitCode};

fn var(nombre: &str) -> Option<String> {
    env::var(nombre).ok().filter(|v| !v.trim().is_empty())
}

#[tokio::main]
async fn main() -> ExitCode {
    match correr().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("bronzeage-servidor: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn correr() -> Result<(), String> {
    let direccion: SocketAddr = var("BRONZEAGE_DIRECCION").unwrap_or_else(|| "127.0.0.1:7878".into()).parse().map_err(|e| format!("BRONZEAGE_DIRECCION: {e}"))?;
    let mundo = PathBuf::from(var("BRONZEAGE_MUNDO").unwrap_or_else(|| "generador/salida/mundo".into()));
    let datos = PathBuf::from(var("BRONZEAGE_DATOS").unwrap_or_else(|| "datos".into()));
    let semilla: u64 = var("BRONZEAGE_SEMILLA").map_or(Ok(42), |s| s.parse()).map_err(|e| format!("BRONZEAGE_SEMILLA: {e}"))?;
    let config = acceso::Config {
        codigo_registro: var("CODIGO_REGISTRO"),
        codigo_registro_bots: var("CODIGO_REGISTRO_BOTS"),
        administradores: var("ADMINISTRADORES").map(|v| v.split(',').map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect()).unwrap_or_default(),
        ..acceso::Config::default()
    };
    let fichero = servicio::buscar_mundo(&mundo).map_err(|e| format!("artefacto de mundo: {e}"))?;
    println!("mundo: {}", fichero.display());
    let arranque = servicio::arranque_de(&fichero, semilla).map_err(|e| format!("artefacto de mundo: {e}"))?;
    let (s, i) = servicio::arrancar(arranque, &datos, direccion, config.clone()).await.map_err(|e| format!("datos en {}: {e}", datos.display()))?;
    println!(
        "escuchando en {} (WebSocket del contrato 05 y HTTP de acceso); mundo {}; versión {}, instante {} ms{}; {} cuentas; registro {}; bots {}; {} administradores",
        s.direccion,
        i.mundo_hash,
        i.version,
        i.ahora,
        if i.recuperado { ", recuperado" } else { ", partida nueva" },
        i.cuentas,
        if config.codigo_registro.is_some() { "con código" } else { "abierto" },
        if config.codigo_registro_bots.is_some() { "permitidos" } else { "sin código: no hay" },
        config.administradores.len(),
    );
    tokio::signal::ctrl_c().await.map_err(|e| format!("señal: {e}"))?;
    println!("parando...");
    let n = s.parar().await;
    println!("parado en la versión {}, instante {} ms", n.version(), n.ahora());
    Ok(())
}
