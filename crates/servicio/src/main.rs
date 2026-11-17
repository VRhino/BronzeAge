//! `bronzeage-servidor`: el proceso del servidor (ver la cabecera de `lib.rs`). Se configura por entorno:
//!
//! | Variable | Qué | Por defecto |
//! |---|---|---|
//! | `BRONZEAGE_DIRECCION` | dónde escucha (WebSocket y HTTP en el mismo puerto) | `127.0.0.1:7878` |
//! | `PORT` | sin `BRONZEAGE_DIRECCION`, escucha en `0.0.0.0:<PORT>`: es lo que pone un alojamiento de contenedores (Render) | — |
//! | `BRONZEAGE_MUNDO` | el artefacto de mundo, o una carpeta con un `mundo-*.bin`; su paquete de relieve (`relieve-<hash>.bin`), si está al lado, también se sirve | `generador/salida/mundo` |
//! | `BRONZEAGE_DATOS` | la carpeta de datos (`mundo/` y `acceso/`) | `datos` |
//! | `BRONZEAGE_SEMILLA` | la semilla de la partida (solo cuenta la primera vez) | `42` |
//! | `BRONZEAGE_BATALLAS_EN_UNITY` | `1`: las batallas las juegan los servidores de batalla (`bronzeage-lanzador`), no se resuelven con números. Es una regla de la partida: arrancar una partida ya creada con el otro valor falla | — (con números) |
//! | `CODIGO_REGISTRO` | código de invitación de las cuentas humanas; sin él, registro abierto | — |
//! | `CODIGO_REGISTRO_BOTS` | código que marca la cuenta como bot; sin él, no hay bots | — |
//! | `CODIGO_REGISTRO_SERVIDORES` | código que marca la cuenta como servidor de batalla (la del lanzador, `bronzeage-lanzador`); sin él, no hay ninguna | — |
//! | `BRONZEAGE_IPS_ADMIN` | IPs, separadas por comas, desde las que se puede administrar (WebSocket de `CLIENTE_ADMIN_WEB` y `/v1/admin/`) además de esta máquina; siempre hace falta además la cuenta admin | solo esta máquina |
//! | `ALMACEN_URL` | una base libSQL (Turso, `libsql://…`) donde se copia la carpeta de datos cada pocos segundos y al parar, y de donde se baja al arrancar con la carpeta vacía (`servicio::espejo`): para alojar el servidor donde el disco no dura | — (solo disco) |
//! | `ALMACEN_TOKEN` | el token de esa base | — |
//! | `BRONZEAGE_LLAVE_ADMIN` | una llave (24 caracteres o más) con la que se administra desde cualquier IP: va en la cabecera `X-Llave-Admin` o en la consulta (`?llave=…`). Para un servidor detrás de un proxy, donde la IP que se ve es la del proxy. Siempre hace falta además la cuenta admin | — (solo por IP) |
//! | `BRONZEAGE_ORIGENES` | nombres de máquina, separados por comas, desde los que una página puede llamar al servidor (además de esta máquina y de la IP del servidor) | — |
//!
//! Sin ninguna cuenta administradora, al arrancar se imprime un código de administrador de un solo uso: quien se registra con él lo es. Ctrl+C, la señal de terminar (SIGTERM, la
//! que manda un alojamiento de contenedores antes de apagar) o la línea `parar` por la entrada
//! estándar (la usa el panel de arranque, `scripts/panel.mjs`, porque en Windows no puede mandar Ctrl+C a un proceso hijo), para el proceso: cierra las sesiones y apaga el núcleo
//! con su foto. Por la misma entrada, `admin dar <nick>` y `admin quitar <nick>` dan y quitan la marca de administrador.

use std::{env, net::{IpAddr, SocketAddr}, path::PathBuf, process::ExitCode};

use acceso::ErrorAdmin;
use tokio::io::{AsyncBufReadExt, BufReader};

fn var(nombre: &str) -> Option<String> {
    env::var(nombre).ok().filter(|v| !v.trim().is_empty())
}

/// Atiende la entrada estándar: `admin dar|quitar <nick>` contesta una línea y vuelve a esperar; `parar` vuelve. Si la entrada se cierra (un proceso sin consola), espera para
/// siempre: solo para Ctrl+C.
async fn parar_por_la_entrada(acceso: &acceso::Acceso) {
    let mut lineas = BufReader::new(tokio::io::stdin()).lines();
    while let Ok(Some(l)) = lineas.next_line().await {
        let palabras: Vec<&str> = l.split_whitespace().collect();
        match palabras[..] {
            ["parar"] => return,
            ["admin", orden @ ("dar" | "quitar"), nick] => match acceso.fijar_admin(nick, orden == "dar") {
                Ok(c) => println!("{} {} administrador", c.nick, if c.admin { "es" } else { "ya no es" }),
                Err(e) => {
                    let motivo = match e {
                        ErrorAdmin::NoExiste => "no existe esa cuenta",
                        ErrorAdmin::EsBot => "es una cuenta de bot",
                        ErrorAdmin::EsServidor => "es una cuenta de servidor de batalla",
                        ErrorAdmin::UltimoAdministrador => "es el último administrador",
                        ErrorAdmin::Disco => "no se pudo guardar",
                    };
                    println!("admin {orden} {nick}: {motivo}");
                }
            },
            _ => {}
        }
    }
    std::future::pending::<()>().await
}

/// La señal de terminar (SIGTERM): la manda un alojamiento de contenedores antes de apagar el proceso, y hay que parar con la foto igual que con Ctrl+C. En Windows no existe:
/// espera para siempre.
async fn terminar() {
    #[cfg(unix)]
    if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        s.recv().await;
        return;
    }
    std::future::pending::<()>().await
}

#[tokio::main]
async fn main() -> ExitCode {
    println!("bronzeage-servidor {} (núcleo {})", env!("CARGO_PKG_VERSION"), nucleo::VERSION);
    match correr().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("bronzeage-servidor: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn correr() -> Result<(), String> {
    let direccion: SocketAddr = match (var("BRONZEAGE_DIRECCION"), var("PORT")) {
        (Some(d), _) => d.parse().map_err(|e| format!("BRONZEAGE_DIRECCION: {e}"))?,
        (None, Some(p)) => SocketAddr::from(([0, 0, 0, 0], p.trim().parse::<u16>().map_err(|e| format!("PORT: {e}"))?)),
        (None, None) => SocketAddr::from(([127, 0, 0, 1], 7878)),
    };
    let mundo = PathBuf::from(var("BRONZEAGE_MUNDO").unwrap_or_else(|| "generador/salida/mundo".into()));
    let datos = PathBuf::from(var("BRONZEAGE_DATOS").unwrap_or_else(|| "datos".into()));
    let semilla: u64 = var("BRONZEAGE_SEMILLA").map_or(Ok(42), |s| s.parse()).map_err(|e| format!("BRONZEAGE_SEMILLA: {e}"))?;
    let en_unity = match var("BRONZEAGE_BATALLAS_EN_UNITY").as_deref().map(str::trim) {
        None | Some("0" | "no" | "false") => false,
        Some("1" | "si" | "sí" | "true") => true,
        Some(o) => return Err(format!("BRONZEAGE_BATALLAS_EN_UNITY: «{o}» no es 1 ni 0")),
    };
    let ips_admin = var("BRONZEAGE_IPS_ADMIN")
        .map(|v| v.split(',').map(str::trim).filter(|s| !s.is_empty()).map(|s| s.parse::<IpAddr>().map_err(|e| format!("BRONZEAGE_IPS_ADMIN: «{s}» no es una IP ({e})"))).collect::<Result<Vec<_>, _>>())
        .transpose()?
        .unwrap_or_default();
    let llave_admin = var("BRONZEAGE_LLAVE_ADMIN").map(|l| l.trim().to_owned());
    if llave_admin.as_ref().is_some_and(|l| l.len() < servidor::MIN_LLAVE_ADMIN) {
        return Err(format!("BRONZEAGE_LLAVE_ADMIN: tiene que tener {} caracteres o más", servidor::MIN_LLAVE_ADMIN));
    }
    let con_llave = llave_admin.is_some();
    let config = acceso::Config {
        codigo_registro: var("CODIGO_REGISTRO"),
        codigo_registro_bots: var("CODIGO_REGISTRO_BOTS"),
        codigo_registro_servidores: var("CODIGO_REGISTRO_SERVIDORES"),
        origenes: var("BRONZEAGE_ORIGENES").map(|v| v.split(',').map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect()).unwrap_or_default(),
        ips_admin,
        llave_admin,
        ..acceso::Config::default()
    };
    let fichero = servicio::buscar_mundo(&mundo).map_err(|e| {
        let ruta = std::path::absolute(&mundo).unwrap_or(mundo.clone());
        format!(
            "no encuentro el artefacto de mundo en {} ({e}). Genéralo con «generar-mundo» en el panel o con `node scripts/generar-mundo.mjs` (unos 2 min), o pon \
             BRONZEAGE_MUNDO con la ruta de un mundo-*.bin o de su carpeta",
            ruta.display()
        )
    })?;
    println!("mundo: {}", fichero.display());
    let mut a = servicio::arranque_de(&fichero, semilla).map_err(|e| format!("artefacto de mundo: {e}"))?;
    a.arranque.reglas.batallas_en_unity = en_unity;
    // El paquete de relieve de los clientes 3D: si falta o no vale, el servidor arranca igual y no lo anuncia.
    let relieve = match servicio::relieve_junto_a(&fichero, &a.arranque) {
        Ok(Some(r)) => {
            println!("relieve: {} ({} bytes)", r.hash, r.bytes.len());
            Some(r)
        }
        Ok(None) => {
            println!("sin paquete de relieve junto al artefacto: los clientes 3D no tendrán la altura fina (se genera con el mundo)");
            None
        }
        Err(e) => {
            eprintln!("el paquete de relieve no vale y no se sirve: {e}");
            None
        }
    };
    // La copia remota va antes que nada: si la carpeta está vacía, la partida se baja de allí; si no se llega a la base, no se arranca (sería empezar otra partida y pisar la guardada).
    let espejo = match var("ALMACEN_URL") {
        Some(url) => {
            let d = datos.clone();
            let (e, apertura) = tokio::task::block_in_place(|| servicio::espejo::Espejo::abrir(&d, Box::new(servicio::espejo::Turso::abrir(&url, var("ALMACEN_TOKEN"))?)))
                .map_err(|e| format!("copia remota (ALMACEN_URL): {e}"))?;
            println!(
                "copia remota: {}",
                match apertura {
                    servicio::espejo::Apertura::Vacio => "vacía, como la carpeta de datos: partida nueva".to_owned(),
                    servicio::espejo::Apertura::Bajada { ficheros } => format!("partida bajada a {} ({ficheros} ficheros)", datos.display()),
                    servicio::espejo::Apertura::Local => "manda la carpeta de datos, que ya tenía la partida".to_owned(),
                }
            );
            Some(e)
        }
        None => None,
    };
    let (s, i) = servicio::arrancar(a.arranque, Some(a.bytes), relieve, &datos, direccion, config.clone()).await.map_err(|e| format!("datos en {}: {e}", datos.display()))?;
    println!(
        "escuchando en {} (WebSocket del contrato 05 y HTTP de acceso); mundo {}; versión {}, instante {} ms{}; {} cuentas; registro {}; bots {}; servidores de batalla {}; batallas {}; {} administradores",
        s.direccion,
        i.mundo_hash,
        i.version,
        i.ahora,
        if i.recuperado { ", recuperado" } else { ", partida nueva" },
        i.cuentas,
        if config.codigo_registro.is_some() { "con código" } else { "abierto" },
        if config.codigo_registro_bots.is_some() { "permitidos" } else { "sin código: no hay" },
        if config.codigo_registro_servidores.is_some() { "permitidos" } else { "sin código: no hay" },
        if en_unity { "en Unity (las juegan los lanzadores)" } else { "con números" },
        s.acceso.administradores().len(),
    );
    if con_llave {
        println!("administración: también desde cualquier IP con la llave de administración");
    }
    let espejo = espejo.map(|e| servicio::espejo::EspejoVivo::lanzar(e, std::time::Duration::from_secs(5)));
    if let Some(codigo) = &i.codigo_admin {
        println!("código de administrador (un solo uso, regístrate con él): {codigo}");
    }
    tokio::select! {
        r = tokio::signal::ctrl_c() => r.map_err(|e| format!("señal: {e}"))?,
        _ = terminar() => {}
        _ = parar_por_la_entrada(&s.acceso) => {}
    }
    println!("parando...");
    let n = s.parar().await;
    println!("parado en la versión {}, instante {} ms", n.version(), n.ahora());
    drop(n);
    if let Some(e) = espejo {
        let v = tokio::task::block_in_place(|| e.parar()).map_err(|e| format!("la última copia remota falló, lo escrito desde la anterior no está en la base: {e}"))?;
        println!("copia remota al día ({} trozos en la última vuelta)", v.trozos);
    }
    Ok(())
}
