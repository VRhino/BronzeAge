//! Arranca el servidor real del motor para las pruebas. Necesita su binario (`cargo build --release -p servicio --target-dir ../BronceAgeBotsBrain/target/motor` en el motor; o `BRONZEAGE_SERVIDOR_BIN`) y un artefacto de mundo
//! (`BRONZEAGE_MUNDO`, por defecto `../BronzeAgeFase0/generador/salida/mundo`). Sin ellos, la prueba se omite.

use std::{path::PathBuf, process::Stdio, time::Duration};

fn existe(var: &str, defecto: &str) -> Option<PathBuf> {
    let p = std::env::var(var).map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(defecto));
    p.exists().then_some(p)
}

/// El código de administrador de un solo uso que imprime cada servidor de prueba al arrancar, por su dirección.
static CODIGOS_ADMIN: std::sync::Mutex<Vec<(String, String)>> = std::sync::Mutex::new(Vec::new());

/// Lee la salida del servidor (sin leerla se llenaría la tubería) y apunta su código de administrador.
fn leer_salida(dir: String, salida: tokio::process::ChildStdout) {
    use tokio::io::AsyncBufReadExt;
    tokio::spawn(async move {
        let mut lineas = tokio::io::BufReader::new(salida).lines();
        while let Ok(Some(l)) = lineas.next_line().await {
            if let Some(c) = l.split("regístrate con él): ").nth(1) {
                CODIGOS_ADMIN.lock().unwrap().push((dir.clone(), c.trim().to_string()));
            }
        }
    });
}

/// Arranca el servidor en un puerto libre con una carpeta de datos nueva y espera a que conteste.
pub async fn servidor(datos: &std::path::Path) -> Option<(tokio::process::Child, String)> {
    let bin = existe("BRONZEAGE_SERVIDOR_BIN", &format!("target/motor/release/bronzeage-servidor{}", std::env::consts::EXE_SUFFIX))?;
    let mundo = existe("BRONZEAGE_MUNDO", "../BronzeAgeFase0/generador/salida/mundo")?;
    let dir = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().to_string();
    let hijo = tokio::process::Command::new(bin)
        .env("BRONZEAGE_DIRECCION", &dir)
        .env("BRONZEAGE_MUNDO", mundo)
        .env("BRONZEAGE_DATOS", datos)
        .env("CODIGO_REGISTRO_BOTS", "bots")
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("arrancar el servidor");
    let mut hijo = hijo;
    leer_salida(dir.clone(), hijo.stdout.take().expect("su salida"));
    for _ in 0..600 {
        if tokio::net::TcpStream::connect(&dir).await.is_ok() {
            return Some((hijo, dir));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("el servidor no abrió {dir}");
}


/// Da una orden con la cuenta de administración `admin` (solo para preparar una prueba: los bots nunca la usan).
#[allow(dead_code)]
pub async fn administrar(dir: &str, orden: contrato::v1::orden::Cuerpo) {
    use bronce_age_bots_brain::{acceso, Credenciales, Puerto};
    use contrato::v1::{Resultado, TipoCliente};
    let c = Credenciales { nick: "admin".into(), clave: "clave de admin".into() };
    // La primera vez, con el código de un solo uso; después la cuenta ya existe y esto falla sin más.
    let codigo = CODIGOS_ADMIN.lock().unwrap().iter().find(|(d, _)| d == dir).map(|(_, c)| c.clone()).unwrap_or_default();
    let _ = acceso::registrar(dir, &c, &codigo).await;
    let mut p = Puerto::entrar_como(dir, c, TipoCliente::ClienteAdminWeb).await.expect("entrar como administración");
    let r = p.actuar(orden).await.unwrap();
    assert_eq!(r.resultado, Resultado::OrdenAceptada as i32, "{r:?}");
    p.cerrar().await;
}

/// Pone el reloj de mundo a `factor` × tiempo real.
#[allow(dead_code)]
pub async fn acelerar(dir: &str, factor: i64) {
    use contrato::v1::{orden::Cuerpo as O, OrdenControlarReloj};
    administrar(dir, O::ControlarReloj(OrdenControlarReloj { accion: "fijar_factor".into(), factor_milesimas: Some(factor * 1000), hasta_ms: None })).await;
}

/// Mete `cantidad` milésimas de `recurso` en el almacén personal de `heroe` (andamiaje de administración).
#[allow(dead_code)]
pub async fn dar_almacen(dir: &str, heroe: &str, recurso: &str, cantidad: f64) {
    use contrato::v1::{orden::Cuerpo as O, OrdenDeAdministracion};
    use prost_types::{value::Kind, Struct, Value};
    let v = |k: Kind| Value { kind: Some(k) };
    let fields = [("heroe_id", v(Kind::StringValue(heroe.into()))), ("recurso", v(Kind::StringValue(recurso.into()))), ("cantidad", v(Kind::NumberValue(cantidad)))];
    let parametros = Some(Struct { fields: fields.into_iter().map(|(k, x)| (k.to_string(), x)).collect() });
    administrar(dir, O::Administracion(OrdenDeAdministracion { codigo: "dar_almacen_personal".into(), parametros })).await;
}
