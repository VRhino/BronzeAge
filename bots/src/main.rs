//! El proceso de bots contra un servidor. Por entorno: `BOTS_SERVIDOR` (127.0.0.1:7878; con `https://…` entra cifrado a uno publicado, y `BOTS_RAIZ_TLS` añade raíces de un PEM), `CODIGO_REGISTRO_BOTS`, `BOTS_REGISTRO` (bots-registro.json), `BOTS_TOTAL` (30),
//! `BOTS_DIAS_LLEGADA` (3), `BOTS_SEMILLA` (1), `BOTS_CADA_MS` (300000, de mundo), `BOTS_HORARIO` (`por-semilla` o `siempre`), `BOTS_CAMPAMENTOS` (`camp-3,camp-7`; sin ella, todos) y
//! `BOTS_POR_CAMPAMENTO` (0). Ctrl+C los desconecta y para.
//!
//! Con `BOTS_TOKEN`, en cambio, es el servicio de bots de la consola de administración: escucha el canal de control en `BOTS_PUERTO` (0.0.0.0:4000) y arranca
//! y para los bots desde allí (`control`); los registros van a `BOTS_REGISTROS` (.): dentro de la carpeta de cada partida, si existe (`partidas/<nombre>/bots.json` del motor).
//!
//! `BOTS_POBLACION=mercenarios` lleva a los mercenarios en vez de a los bots libres: otro proceso, con su puerto (por defecto 4001), su registro y su plan de llegadas, que empieza en 0.

use bronce_age_bots_brain::{
    cerebro::Bot,
    control::{Ajustes, Poblacion, Servicio},
    mercenario::Mercenario,
    runner::{Config, Lugares},
    Proceso,
};

#[tokio::main]
async fn main() -> Result<(), String> {
    let var = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.into());
    let num = |k: &str, d: &str| var(k, d).parse::<f64>().map_err(|e| format!("{k}: {e}"));
    let mercenarios = var("BOTS_POBLACION", "libres") == "mercenarios";
    let poblacion = if mercenarios { Poblacion::Mercenarios } else { Poblacion::Libres };
    let cfg = Config {
        // El panel del motor la pasa como URL (`http://localhost:7878`), como al TypeScript.
        servidor: bronce_age_bots_brain::acceso::normalizar(&var("BOTS_SERVIDOR", "127.0.0.1:7878")),
        codigo_bots: var("CODIGO_REGISTRO_BOTS", ""),
        registro: var("BOTS_REGISTRO", if mercenarios { "mercs-registro.json" } else { "bots-registro.json" }).into(),
        total: num("BOTS_TOTAL", if mercenarios { "0" } else { "30" })? as u32,
        dias_llegada: num("BOTS_DIAS_LLEGADA", "3")?,
        semilla: num("BOTS_SEMILLA", "1")? as u64,
        cada_ms: num("BOTS_CADA_MS", "300000")? as i64,
        siempre: mercenarios || var("BOTS_HORARIO", "por-semilla") == "siempre",
        lugares: Lugares::del_entorno(),
    };
    let token = var("BOTS_TOKEN", "");
    if !token.is_empty() {
        let por_defecto = serde_json::json!({
            "servidor": if cfg.servidor.starts_with("https://") { cfg.servidor.clone() } else { format!("http://{}", cfg.servidor) }, "total": cfg.total, "diasLlegada": cfg.dias_llegada, "semilla": cfg.semilla,
            "cadaMs": cfg.cada_ms, "horario": if cfg.siempre { "siempre" } else { "por-semilla" },
            "campamentos": cfg.lugares.campamentos, "porCampamento": cfg.lugares.por_campamento,
        });
        let s = Servicio::new(Ajustes { token, poblacion, codigo_bots: cfg.codigo_bots, registros: var("BOTS_REGISTROS", ".").into(), cada_ms: cfg.cada_ms, por_defecto });
        // El panel pasa solo el número de puerto.
        let dir = var("BOTS_PUERTO", if mercenarios { "0.0.0.0:4001" } else { "0.0.0.0:4000" });
        let dir = if dir.contains(':') { dir } else { format!("0.0.0.0:{dir}") };
        println!("servicio de bots en {dir}/control");
        s.retomar().await;
        return s.escuchar(&dir).await.map_err(|e| e.to_string());
    }
    let p = if mercenarios { Proceso::arrancar(cfg, Mercenario).await } else { Proceso::arrancar(cfg, Bot).await }.map_err(|f| format!("arrancar: {f:?}"))?;
    println!("bots en marcha; Ctrl+C para parar");
    tokio::signal::ctrl_c().await.map_err(|e| e.to_string())?;
    p.parar().await.map_err(|f| format!("parar: {f:?}"))
}
