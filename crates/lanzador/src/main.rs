//! `bronzeage-lanzador`: el lanzador de servidores de batalla (ver la cabecera de `lib.rs`). Se configura por entorno:
//!
//! | Variable | Qué | Por defecto |
//! |---|---|---|
//! | `LANZADOR_MOTOR` | dónde escucha el motor (`host:puerto`, el de `BRONZEAGE_DIRECCION`) | `127.0.0.1:7878` |
//! | `LANZADOR_NICK`, `LANZADOR_CLAVE` | la cuenta de servidor de batalla | — (obligatorias) |
//! | `CODIGO_REGISTRO_SERVIDORES` | el mismo código que el del motor: si la cuenta no existe, la da de alta al entrar | — (sin él, la cuenta tiene que existir) |
//! | `LANZADOR_HOST` | la dirección a la que se conectan los jugadores | `127.0.0.1` |
//! | `LANZADOR_PUERTOS` | los puertos de juego, uno por batalla a la vez (`7777-7786`, `7777,7780`) | `7777-7786` |
//! | `LANZADOR_PROCESO` | el ejecutable del servidor de batalla (Conquest como servidor dedicado), o `mentira`: el servidor de batalla de mentira que lleva dentro el lanzador | — (obligatoria) |
//! | `LANZADOR_ARGUMENTOS` | sus argumentos fijos, separados por espacios (p. ej. `-batchmode -nographics`; a la de mentira, `--espera-ms 60000` para que dure un minuto); los de cada batalla van detrás | — |
//!
//! Ctrl+C, o la línea `parar` por la entrada estándar, lo para: mata los procesos de batalla que tenga (el núcleo las vence por su plazo). `bronzeage-lanzador mentira …` no
//! lanza nada: es el servidor de batalla de mentira ([`lanzador::mentira`]), el proceso que arranca el lanzador con `LANZADOR_PROCESO=mentira`.

use std::{env, process::ExitCode};

use lanzador::{Config, Informe};
use tokio::io::{AsyncBufReadExt, BufReader};

fn var(nombre: &str) -> Option<String> {
    env::var(nombre).ok().filter(|v| !v.trim().is_empty())
}

fn config() -> Result<Config, String> {
    let obligatoria = |n: &str| var(n).ok_or_else(|| format!("falta {n}"));
    let proceso = obligatoria("LANZADOR_PROCESO")?;
    let mentira = proceso == "mentira";
    let proceso = if mentira { env::current_exe().map_err(|e| format!("mi propio ejecutable: {e}"))? } else { proceso.into() };
    let mut c = Config::nueva(&var("LANZADOR_MOTOR").unwrap_or_else(|| "127.0.0.1:7878".into()), &obligatoria("LANZADOR_NICK")?, &obligatoria("LANZADOR_CLAVE")?, proceso);
    if let Some(h) = var("LANZADOR_HOST") {
        c.host_publico = h;
    }
    if let Some(p) = var("LANZADOR_PUERTOS") {
        c.puertos = lanzador::puertos(&p).map_err(|e| format!("LANZADOR_PUERTOS: {e}"))?;
    }
    c.codigo = var("CODIGO_REGISTRO_SERVIDORES");
    c.argumentos = mentira.then(|| "mentira".to_owned()).into_iter().chain(var("LANZADOR_ARGUMENTOS").iter().flat_map(|a| a.split_whitespace().map(str::to_owned))).collect();
    Ok(c)
}

async fn parar() {
    let entrada = async {
        let mut lineas = BufReader::new(tokio::io::stdin()).lines();
        while let Ok(Some(l)) = lineas.next_line().await {
            if l.trim() == "parar" {
                return;
            }
        }
        std::future::pending::<()>().await
    };
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = entrada => {}
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    if env::args().nth(1).as_deref() == Some("mentira") {
        lanzador::mentira::correr().await;
        return ExitCode::SUCCESS;
    }
    println!("bronzeage-lanzador {}", env!("CARGO_PKG_VERSION"));
    let c = match config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bronzeage-lanzador: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "motor {} con la cuenta {}; {} puertos ({}-{}) en {}; proceso {}{}",
        c.motor,
        c.nick,
        c.puertos.len(),
        c.puertos[0],
        c.puertos[c.puertos.len() - 1],
        c.host_publico,
        c.proceso.display(),
        if c.argumentos.first().is_some_and(|a| a == "mentira") { " (el servidor de batalla de mentira: nadie muere y gana el defensor)" } else { "" }
    );
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(i) = rx.recv().await {
            println!("{}", texto(&i));
        }
    });
    lanzador::correr(c, Some(tx), parar()).await;
    println!("parado");
    ExitCode::SUCCESS
}

fn texto(i: &Informe) -> String {
    match i {
        Informe::Conectado => "conectado con el motor".into(),
        Informe::Desconectado(m) => format!("sin sesión con el motor: {m}"),
        Informe::Reclama { batalla, revision, puerto, humanos } => format!("{batalla}: reclamo la revisión {revision} en el puerto {puerto} ({} humanos)", humanos.len()),
        Informe::SinPuerto { batalla } => format!("{batalla}: sin puerto libre, no la reclamo"),
        Informe::Asignada { batalla, revision } => format!("{batalla}: asignada (revisión {revision})"),
        Informe::Rechazada { batalla, codigo, .. } => format!("{batalla}: no es mía ({codigo})"),
        Informe::ProcesoArrancado { batalla } => format!("{batalla}: proceso arrancado"),
        Informe::ProcesoConectado { batalla } => format!("{batalla}: proceso conectado"),
        Informe::Reenvia { batalla, mensaje } => format!("{batalla}: {mensaje} al motor"),
        Informe::Acuse { batalla, mensaje, resultado, codigo } => format!("{batalla}: acuse de {mensaje}: {resultado:?}{}", codigo.as_deref().map(|c| format!(" ({c})")).unwrap_or_default()),
        Informe::Descartado { batalla, mensaje } => format!("{batalla}: el proceso mandó {mensaje}, que pone el lanzador: descartado"),
        Informe::Terminada { batalla, con_resultado, motivo } => {
            format!("{batalla}: terminada {} ({motivo})", if *con_resultado { "con resultado" } else { "sin resultado: la vence el núcleo por su plazo" })
        }
    }
}
