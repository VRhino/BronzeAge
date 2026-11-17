//! El batch «partida» (7F): una partida nueva contra el servidor real con el reloj acelerado, con todos los cerebros y sin andamiaje. Arranca el servidor con una carpeta de datos
//! nueva, lo acelera con la cuenta de administración (lo único que hace la administración: los bots juegan como jugadores), deja jugar a los bots `PARTIDA_DIAS` días de mundo
//! y escribe el informe: una fila por día con lo que ve `ADMIN` y, al final, las órdenes de los bots por resultado.
//!
//! Por entorno: `BRONZEAGE_SERVIDOR_BIN` (`target/motor/release/bronzeage-servidor`), `BRONZEAGE_MUNDO` (`../BronzeAgeFase0/generador/salida/mundo`), `PARTIDA_DIAS` (3),
//! `PARTIDA_FACTOR` (1440: un día de mundo por minuto), `BOTS_TOTAL` (30), `BOTS_DIAS_LLEGADA` (1), `BOTS_SEMILLA` (1), `BOTS_CAMPAMENTOS` y `BOTS_POR_CAMPAMENTO` (donde nacen), `PARTIDA_MERCENARIOS` (0: cuántos mercenarios llegan, en los mismos
//! campamentos y días), `PARTIDA_SALIDA` (`partidas`). Escribe además `campamentos.json`, con dónde está cada campamento, para elegir una región.

use std::{collections::BTreeMap, fmt::Write as _, process::Stdio, time::Duration};

use bronce_age_bots_brain::{
    acceso,
    cerebro::Bot,
    mapa,
    mercenario::Mercenario,
    puerto,
    runner::{Config, Lugares},
    Credenciales, Proceso, Puerto,
};
use contrato::v1::{entidad::Cuerpo, orden::Cuerpo as O, OrdenControlarReloj, TipoCliente};
use serde_json::json;

fn var(k: &str, d: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| d.into())
}

fn num(k: &str, d: &str) -> Result<f64, String> {
    var(k, d).parse().map_err(|e| format!("{k}: {e}"))
}

/// Cuánto trabajo repiten las Facciones: de los viajes (una columna que sale a un objetivo, y sus órdenes de seguir hacia él en las 2 h siguientes cuentan como el mismo),
/// cuántos salen a algo a lo que otra columna de su Facción salió en las últimas 2 h. Por tipo de objetivo. De los viajes sin tropa a un punto (`ver`, la exploración),
/// además, cuántas casillas distintas pisaron y cuántos fueron a la casilla de al lado de la de otra columna de su Facción en las últimas 2 h (`cerca`).
fn repetidos() -> serde_json::Value {
    const VENTANA: i64 = 2 * 3_600_000;
    let mut m = puerto::marchas();
    m.sort_by_key(|x| x.3);
    let mut ultimo: BTreeMap<(String, String, String), i64> = BTreeMap::new();
    let mut por_objetivo: BTreeMap<(String, String), BTreeMap<String, i64>> = BTreeMap::new();
    let mut tipos: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    let (mut casillas, mut cerca, mut vistas): (std::collections::BTreeSet<(String, String)>, u32, Vec<(String, String, (i64, i64), i64)>) = Default::default();
    for (f, col, clave, t) in m {
        if let Some((x, y)) = clave.strip_prefix("ver:").and_then(|c| c.split_once(',')).and_then(|(x, y)| Some((x.parse::<i64>().ok()?, y.parse::<i64>().ok()?))) {
            casillas.insert((f.clone(), clave.clone()));
            cerca += vistas.iter().any(|(g, c, (a, b), u)| *g == f && *c != col && t - u < VENTANA && (a - x).abs() <= 1 && (b - y).abs() <= 1) as u32;
            vistas.push((f.clone(), col.clone(), (x, y), t));
        }
        let nuevo = ultimo.get(&(f.clone(), col.clone(), clave.clone())).is_none_or(|u| t - u >= VENTANA);
        ultimo.insert((f.clone(), col.clone(), clave.clone()), t);
        let otros = por_objetivo.entry((f, clave.clone())).or_default();
        if nuevo {
            let e = tipos.entry(clave.split(':').next().unwrap_or("").into()).or_default();
            e.0 += 1;
            e.1 += otros.iter().any(|(c, u)| *c != col && t - u < VENTANA) as u32;
        }
        otros.insert(col, t);
    }
    let mut j: serde_json::Map<_, _> = tipos.into_iter().map(|(k, (v, r))| (k, json!({"viajes": v, "repetidos": r}))).collect();
    if let Some(v) = j.get_mut("ver") {
        v["casillas"] = json!(casillas.len());
        v["cerca"] = json!(cerca);
    }
    json!(j)
}

/// La cadena del bronce en un momento: minas de estaño y de cobre, lo que guardan las plazas (mineral y lingotes, en unidades), fundiciones (y cuántas a nivel 2 o más),
/// plazas con un yacimiento de estaño en su zona y cuántas Facciones tienen la aleación de bronce aparecida y adoptada.
fn cadena_del_bronce(admin: &Puerto, estano: &[contrato::v1::Punto], dia: u32) -> serde_json::Value {
    let todo = || admin.observar().ambitos.get("ADMIN:").into_iter().flat_map(|a| a.entidades.values());
    let t = admin.mundo_ms();
    let cantidad = |e: &contrato::v1::Existencia| {
        e.cantidad.as_ref().map_or(0, |r| {
            let v = if r.en_escalones { r.valor_en_t0 } else { r.valor_en_t0 + r.por_minuto * (t - r.t0_ms).max(0) / 60_000 };
            v.min(r.maximo.unwrap_or(i64::MAX)).clamp(r.minimo.unwrap_or(0), e.capacidad.max(0)) / 1000
        })
    };
    let (mut minas_estano, mut minas_cobre, mut fundiciones, mut fundiciones_2, mut junto_al_estano, mut aparecida, mut adoptada) = (0, 0, 0, 0, 0, 0, 0);
    let mut almacen: BTreeMap<&str, i64> = BTreeMap::new();
    for c in todo() {
        match c {
            Cuerpo::Pueblo(x) if x.tipo_extractor == "minaEstano" => minas_estano += 1,
            Cuerpo::Pueblo(x) if x.tipo_extractor == "minaCobre" => minas_cobre += 1,
            Cuerpo::Asentamiento(p) => {
                for r in ["cobre", "estano", "lingoteCobre", "lingoteEstano", "lingoteBronce"] {
                    *almacen.entry(r).or_default() += p.almacen.get(r).map_or(0, cantidad);
                }
                let f: Vec<_> = p.edificios.iter().filter(|e| e.tipo == "fundicion").collect();
                fundiciones += f.len();
                fundiciones_2 += f.iter().filter(|e| e.nivel_interno.unwrap_or(1) >= 2).count();
                junto_al_estano += p.posicion.as_ref().is_some_and(|c| estano.iter().any(|q| ((q.x - c.x) as f64).hypot((q.y - c.y) as f64) <= p.radio_potencial as f64)) as u32;
            }
            Cuerpo::Tecnologias(x) => {
                aparecida += x.aparecidas.iter().any(|t| t == "aleacion_bronce") as u32;
                adoptada += x.adoptadas.iter().any(|t| t == "aleacion_bronce") as u32;
            }
            _ => {}
        }
    }
    json!({
        "dia": dia, "minas_estano": minas_estano, "minas_cobre": minas_cobre, "almacen": almacen, "fundiciones": fundiciones, "fundiciones_nivel_2": fundiciones_2,
        "plazas_junto_al_estano": junto_al_estano, "bronce_aparecida": aparecida, "bronce_adoptada": adoptada,
    })
}

/// Lo que ve `ADMIN` en un momento: una fila del informe.
fn foto(admin: &Puerto, dia: u32) -> serde_json::Value {
    let todo = || admin.observar().ambitos.get("ADMIN:").into_iter().flat_map(|a| a.entidades.values());
    let mut niveles: BTreeMap<u32, u32> = BTreeMap::new();
    let (mut facciones, mut plazas, mut heroes, mut con_plaza, mut escuadras, mut soldados, mut batallas, mut caravanas) = (0, 0, 0, 0, 0, 0u64, 0, 0);
    for c in todo() {
        match c {
            Cuerpo::Faccion(_) => facciones += 1,
            Cuerpo::Asentamiento(p) => {
                plazas += 1;
                *niveles.entry(p.nivel_actual).or_default() += 1;
            }
            Cuerpo::Heroe(h) => {
                heroes += 1;
                con_plaza += h.residencia_id.as_deref().is_some_and(|r| r.starts_with("as-")) as u32;
            }
            Cuerpo::Escuadron(e) if e.prestada_por_campamento_id.is_none() => {
                escuadras += 1;
                soldados += e.cantidad as u64;
            }
            Cuerpo::Batalla(_) => batallas += 1,
            Cuerpo::Caravana(_) => caravanas += 1,
            _ => {}
        }
    }
    json!({
        "dia": dia, "facciones": facciones, "plazas": plazas, "niveles": niveles, "heroes": heroes, "heroes_con_plaza": con_plaza,
        "escuadras": escuadras, "soldados": soldados, "batallas": batallas, "caravanas": caravanas,
    })
}

/// Lo que se mide al final para la tabla «Para la fase 7» (`Docs/Mecanicas a desarrollar.md` del motor): tropas por tipo (¿se reclutan las de Barracón y Galería?),
/// reputación de cada Facción, recintos de cada plaza (la empalizada), tecnologías adoptadas y residentes por plaza.
fn mediciones(admin: &Puerto) -> serde_json::Value {
    let todo = || admin.observar().ambitos.get("ADMIN:").into_iter().flat_map(|a| a.entidades.values());
    let mut tropas: BTreeMap<String, (u32, u64)> = BTreeMap::new();
    let mut reputacion: BTreeMap<String, f64> = BTreeMap::new();
    let mut recintos: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    let mut tecnologias: BTreeMap<String, usize> = BTreeMap::new();
    let mut residentes: BTreeMap<String, usize> = BTreeMap::new();
    let mut personalidades: BTreeMap<String, String> = BTreeMap::new();
    let mut plazas = Vec::new();
    let mut relaciones = Vec::new();
    let mut minas_estano = Vec::new();
    for c in todo() {
        match c {
            Cuerpo::Escuadron(e) if e.prestada_por_campamento_id.is_none() => {
                let t = tropas.entry(e.tropa_id.clone()).or_default();
                t.0 += 1;
                t.1 += e.cantidad as u64;
            }
            Cuerpo::Faccion(f) => {
                reputacion.insert(f.id.clone(), f.reputacion.as_ref().map_or(0.0, |r| r.valor_en_t0 as f64 / 1000.0));
                personalidades.insert(f.id.clone(), format!("{:?}", bronce_age_bots_brain::plaza::Personalidad::de(&f.id)));
            }
            Cuerpo::Asentamiento(p) => {
                recintos.insert(p.id.clone(), p.recintos.iter().map(|r| r.nivel).collect());
                residentes.insert(p.id.clone(), p.casas_compradas.len());
                let clase = |r: &Option<contrato::v1::Ritmo>| r.as_ref().map_or(0, |x| x.valor_en_t0 / 1000);
                let (pe, ar) = p.poblacion.as_ref().map_or((0, 0), |x| (clase(&x.pesants), clase(&x.artesanos)));
                let mut edificios: BTreeMap<String, u32> = BTreeMap::new();
                p.edificios.iter().for_each(|e| *edificios.entry(e.tipo.clone()).or_default() += 1);
                plazas.push(json!({"id": p.id, "faccion": p.faccion_id, "pos": p.posicion.as_ref().map(|x| (x.x, x.y)), "nivel": p.nivel_actual, "pesants": pe, "artesanos": ar, "residentes": p.casas_compradas.len(), "edificios": edificios, "almacen": p.almacen.iter().map(|(r, e)| (r.clone(), clase(&e.cantidad))).collect::<BTreeMap<_, _>>(), "estano": p.almacen.get("estano").map_or(0, |e| clase(&e.cantidad)), "bronce": p.almacen.get("lingoteBronce").map_or(0, |e| clase(&e.cantidad))}));
            }
            Cuerpo::Tecnologias(t) => {
                tecnologias.insert(t.faccion_id.clone(), t.adoptadas.len());
            }
            Cuerpo::Pueblo(x) if x.tipo_extractor == "minaEstano" => minas_estano.push(x.asentamiento_madre_id.clone()),
            Cuerpo::Relacion(r) if r.estado == contrato::v1::EstadoRelacion::RelacionActiva as i32 => relaciones.push(json!({"tipo": r.tipo, "a": r.faccion_a_id, "b": r.faccion_b_id})),
            _ => {}
        }
    }
    let tropas: BTreeMap<_, _> = tropas.into_iter().map(|(t, (n, s))| (t, json!({"escuadras": n, "soldados": s}))).collect();
    json!({ "tropas": tropas, "personalidades": personalidades, "reputacion": reputacion, "recintos": recintos, "tecnologias_adoptadas": tecnologias, "residentes": residentes, "plazas": plazas, "relaciones": relaciones, "minas_estano": minas_estano })
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let dias = num("PARTIDA_DIAS", "3")? as u32;
    let factor = num("PARTIDA_FACTOR", "1440")? as i64;
    let semilla = num("BOTS_SEMILLA", "1")? as u64;
    let salida = std::path::PathBuf::from(var("PARTIDA_SALIDA", "partidas"));
    std::fs::create_dir_all(&salida).map_err(|e| e.to_string())?;
    let datos = salida.join(format!("datos-{semilla}"));
    let _ = std::fs::remove_dir_all(&datos);

    // El servidor, con una partida nueva.
    let dir = std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()).map_err(|e| e.to_string())?.to_string();
    let bin = var("BRONZEAGE_SERVIDOR_BIN", &format!("target/motor/release/bronzeage-servidor{}", std::env::consts::EXE_SUFFIX));
    let mut servidor = tokio::process::Command::new(&bin)
        .env("BRONZEAGE_DIRECCION", &dir)
        .env("BRONZEAGE_MUNDO", var("BRONZEAGE_MUNDO", "../BronzeAgeFase0/generador/salida/mundo"))
        .env("BRONZEAGE_DATOS", &datos)
        .env("BRONZEAGE_SEMILLA", semilla.to_string())
        .env("CODIGO_REGISTRO_BOTS", "bots")
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("{bin}: {e}"))?;
    // El servidor imprime al arrancar el código de administrador de un solo uso; se sigue leyendo su salida para que no se llene la tubería.
    let (codigo_tx, codigo_rx) = tokio::sync::oneshot::channel::<String>();
    {
        use tokio::io::AsyncBufReadExt;
        let mut lineas = tokio::io::BufReader::new(servidor.stdout.take().ok_or("sin salida del servidor")?).lines();
        tokio::spawn(async move {
            let mut tx = Some(codigo_tx);
            while let Ok(Some(l)) = lineas.next_line().await {
                if let Some(c) = l.split("regístrate con él): ").nth(1) {
                    if let Some(tx) = tx.take() {
                        let _ = tx.send(c.trim().to_string());
                    }
                }
            }
        });
    }
    while tokio::net::TcpStream::connect(&dir).await.is_err() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // La administración: solo acelera el reloj y mira.
    let c = Credenciales { nick: "admin".into(), clave: "clave de admin".into() };
    let codigo = tokio::time::timeout(Duration::from_secs(30), codigo_rx).await.map_err(|_| "el servidor no dio el código de administrador".to_string())?.map_err(|e| e.to_string())?;
    acceso::registrar(&dir, &c, &codigo).await.map_err(|f| format!("{f:?}"))?;
    let mut admin = Puerto::entrar_como(&dir, c, TipoCliente::ClienteAdminWeb).await.map_err(|f| format!("{f:?}"))?;
    admin
        .actuar(O::ControlarReloj(OrdenControlarReloj { accion: "fijar_factor".into(), factor_milesimas: Some(factor * 1000), hasta_ms: None }))
        .await
        .map_err(|f| format!("{f:?}"))?;

    let cfg = Config {
        servidor: dir.clone(),
        codigo_bots: "bots".into(),
        registro: salida.join(format!("registro-{semilla}.json")),
        total: num("BOTS_TOTAL", "30")? as u32,
        dias_llegada: num("BOTS_DIAS_LLEGADA", "1")?,
        semilla,
        cada_ms: 300_000,
        siempre: false,
        lugares: Lugares::del_entorno(),
    };
    let _ = std::fs::remove_file(&cfg.registro);
    let m = mapa::de(&admin).await.map_err(|f| format!("{f:?}"))?;
    let campamentos: Vec<_> = m.campamentos.iter().enumerate().map(|(i, p)| json!({"id": format!("camp-{i}"), "x": p.x, "y": p.y})).collect();
    std::fs::write(salida.join("campamentos.json"), serde_json::to_string_pretty(&campamentos).unwrap()).map_err(|e| e.to_string())?;
    let mercenarios = num("PARTIDA_MERCENARIOS", "0")? as u32;
    let mut cfg_mercs = cfg.clone();
    let p = Proceso::arrancar(cfg, Bot).await.map_err(|f| format!("{f:?}"))?;
    let pm = if mercenarios > 0 {
        cfg_mercs.registro = salida.join(format!("mercs-{semilla}.json"));
        (cfg_mercs.total, cfg_mercs.siempre) = (mercenarios, true);
        let _ = std::fs::remove_file(&cfg_mercs.registro);
        Some(Proceso::arrancar(cfg_mercs, Mercenario).await.map_err(|f| format!("{f:?}"))?)
    } else {
        None
    };

    puerto::MEDIR_MARCHAS.store(true, std::sync::atomic::Ordering::Relaxed);
    let inicio = admin.mundo_ms();
    let mut filas = vec![foto(&admin, 0)];
    let estano: Vec<contrato::v1::Punto> = mapa::de(&admin).await.map(|m| m.nodos.iter().filter(|(_, tipo, _)| tipo == "estano").map(|(_, _, p)| p.clone()).collect()).unwrap_or_default();
    let mut bronce = vec![cadena_del_bronce(&admin, &estano, 0)];
    let mut hechos: BTreeMap<String, u64> = BTreeMap::new();
    let mut acosos = Vec::new();
    for dia in 1..=dias {
        while admin.mundo_ms() < inicio + dia as i64 * 86_400_000 {
            admin.atender(Duration::from_secs(1)).await.map_err(|f| format!("{f:?}"))?;
            for (_, h) in admin.hechos() {
                if matches!(h.codigo.as_str(), "bandidos.caravana_interceptada" | "bandidos.caravana_escapa" | "fundacion.caravana_perdida") {
                    acosos.push(json!({"codigo": h.codigo, "en": h.en_ms, "plaza": h.asentamiento_id, "datos": format!("{:?}", h.payload)}));
                }
                *hechos.entry(h.codigo).or_default() += 1;
            }
        }
        let f = foto(&admin, dia);
        eprintln!("partida: {f}");
        filas.push(f);
        bronce.push(cadena_del_bronce(&admin, &estano, dia));
    }
    let medido = mediciones(&admin);
    p.parar().await.map_err(|f| format!("{f:?}"))?;
    if let Some(pm) = pm {
        pm.parar().await.map_err(|f| format!("{f:?}"))?;
    }
    admin.cerrar().await;
    let _ = servidor.kill().await;

    // El informe.
    let ordenes: Vec<_> = puerto::ordenes_dadas().into_iter().filter(|((_, como), _)| !como.is_empty()).map(|((que, como), n)| json!({"orden": que, "resultado": como, "veces": n})).collect();
    let informe = json!({ "semilla": semilla, "dias": dias, "factor": factor, "filas": filas, "hechos": hechos, "ordenes": ordenes, "mediciones": medido, "acosos": acosos, "repetidos": repetidos(), "bronce": bronce });
    let base = salida.join(format!("partida-{semilla}"));
    std::fs::write(base.with_extension("json"), serde_json::to_string_pretty(&informe).unwrap()).map_err(|e| e.to_string())?;
    let mut md = format!("# Partida con bots (semilla {semilla}, {dias} días a ×{factor})\n\n| Día | Facciones | Plazas | Niveles | Héroes (con plaza) | Escuadras (soldados) | Batallas en curso | Caravanas |\n|---|---|---|---|---|---|---|---|\n");
    for f in &informe["filas"].as_array().unwrap().clone() {
        let _ = writeln!(md, "| {} | {} | {} | {} | {} ({}) | {} ({}) | {} | {} |", f["dia"], f["facciones"], f["plazas"], f["niveles"], f["heroes"], f["heroes_con_plaza"], f["escuadras"], f["soldados"], f["batallas"], f["caravanas"]);
    }
    let m = &informe["mediciones"];
    md.push_str("\n## Mediciones al final\n\n| Tropa | Escuadras | Soldados |\n|---|---|---|\n");
    for (t, v) in m["tropas"].as_object().into_iter().flatten() {
        let _ = writeln!(md, "| {t} | {} | {} |", v["escuadras"], v["soldados"]);
    }
    let _ = writeln!(md, "\n- Personalidad por Facción: {}", m["personalidades"]);
    let _ = writeln!(md, "- Reputación por Facción: {}", m["reputacion"]);
    let _ = writeln!(md, "- Recintos por plaza (niveles): {}", m["recintos"]);
    let _ = writeln!(md, "- Tecnologías adoptadas por Facción: {}", m["tecnologias_adoptadas"]);
    let _ = writeln!(md, "- Residentes por plaza: {}", m["residentes"]);
    md.push_str("\n## La cadena del bronce\n\n| Día | Minas de estaño | Minas de cobre | Cobre | Estaño | Lingote de cobre | Lingote de estaño | Lingote de bronce | Fundiciones (nivel 2+) | Plazas junto al estaño | Aleación (aparecida / adoptada) |\n|---|---|---|---|---|---|---|---|---|---|---|\n");
    for b in informe["bronce"].as_array().into_iter().flatten() {
        let a = &b["almacen"];
        let _ = writeln!(md, "| {} | {} | {} | {} | {} | {} | {} | {} | {} ({}) | {} | {} / {} |", b["dia"], b["minas_estano"], b["minas_cobre"], a["cobre"], a["estano"], a["lingoteCobre"], a["lingoteEstano"], a["lingoteBronce"], b["fundiciones"], b["fundiciones_nivel_2"], b["plazas_junto_al_estano"], b["bronce_aparecida"], b["bronce_adoptada"]);
    }
    md.push_str("\n## Viajes repetidos\n\nUn viaje es repetido si otra columna de la misma Facción salió al mismo objetivo en las 2 h anteriores (`band`: campamento de bandidos, `as`: plaza ajena, `pueblo`, `celda`: un punto, en casillas de 20 u).\n\n| Objetivo | Viajes | Repetidos |\n|---|---|---|\n");
    for (k, v) in informe["repetidos"].as_object().into_iter().flatten() {
        let _ = writeln!(md, "| {k} | {} | {} |", v["viajes"], v["repetidos"]);
    }
    if let Some(v) = informe["repetidos"].get("ver") {
        let _ = writeln!(md, "\nExploración (`ver`): {} casillas distintas; {} órdenes a la casilla de al lado de la de otra columna de su Facción en las 2 h anteriores.", v["casillas"], v["cerca"]);
    }
    md.push_str("\n## Órdenes de los bots\n\n| Orden | Resultado | Veces |\n|---|---|---|\n");
    for ((que, como), n) in puerto::ordenes_dadas() {
        let _ = writeln!(md, "| {que} | {como} | {n} |");
    }
    md.push_str("\n## Hechos\n\n| Hecho | Veces |\n|---|---|\n");
    for (h, n) in &hechos {
        let _ = writeln!(md, "| {h} | {n} |");
    }
    std::fs::write(base.with_extension("md"), md).map_err(|e| e.to_string())?;
    println!("informe en {}", base.with_extension("md").display());
    Ok(())
}
