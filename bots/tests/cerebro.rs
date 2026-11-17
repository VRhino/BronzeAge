//! El cerebro sin plaza contra el servidor real (7E): seis bots llegan, el líder de los amigos crea su Facción, sus cuatro amigos piden entrar y él los acepta, el sexto (un
//! tardío) entra en ella, y uno sale del campamento a explorar el anillo.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use bronce_age_bots_brain::{
    cerebro,
    runner::{Config, Registro},
    Cerebro, Contexto, Fallo, Proceso,
};
use contrato::v1::entidad::Cuerpo;

mod comun;

/// Lo que se vio de cada héroe al pensar: su Facción, si salió alguna vez del campamento y dónde reside.
type Visto = Arc<Mutex<BTreeMap<String, (Option<String>, bool, Option<String>)>>>;

struct Mira(Visto);

impl Cerebro for Mira {
    type Memoria = cerebro::Memoria;
    type Pizarra = cerebro::Pizarra;
    async fn pensar(&self, ctx: &mut Contexto<'_, Self>) -> Result<(), Fallo> {
        // Mira con el mismo contexto: el cerebro de verdad piensa con sus tipos.
        let r = cerebro::pensar(ctx).await;
        let fuera = ctx.puerto.observar().ambitos.values().flat_map(|a| a.entidades.values()).any(|c| matches!(c, Cuerpo::Ejercito(e) if e.participantes.iter().any(|p| p.heroe_id == ctx.yo)));
        let mut v = self.0.lock().unwrap();
        let e = v.entry(ctx.yo.to_string()).or_default();
        e.0 = ctx.faccion();
        e.1 |= fuera;
        e.2 = ctx.puerto.observar().ambitos.values().flat_map(|a| a.entidades.values()).find_map(|c| match c {
            Cuerpo::Heroe(h) if h.id == ctx.yo => h.residencia_id.clone(),
            _ => None,
        });
        r
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn sin_plaza_forman_faccion_y_salen() {
    let datos = tempfile::tempdir().unwrap();
    let Some((_servidor, dir)) = comun::servidor(datos.path()).await else {
        eprintln!("sin binario del servidor o sin artefacto de mundo: se omite");
        return;
    };
    let ruta = datos.path().join("registro.json");
    let cfg = Config { servidor: dir, codigo_bots: "bots".into(), registro: ruta.clone(), total: 6, dias_llegada: 6.0 / 86_400.0, semilla: 3, cada_ms: 1_000, siempre: true, lugares: Default::default() };
    let visto: Visto = Default::default();
    let p = Proceso::arrancar(cfg, Mira(visto.clone())).await.expect("arrancar");
    let mut listo = false;
    for _ in 0..900 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let Some(r) = Registro::leer(&ruta).unwrap().filter(|r| r.bots.len() == 6) else { continue };
        let v = visto.lock().unwrap();
        let f = |i: usize| v.get(&r.bots[i].heroe_id).and_then(|x| x.0.clone());
        let amigos = f(0).is_some() && (1..5).all(|i| f(i) == f(0));
        // El sexto (con la semilla 3, un tardío) entra en una que ya existe.
        let sexto = f(5).is_some();
        // Sin presa conocida sale uno por campamento a explorar; los demás esperan dentro.
        let salen = r.bots.iter().any(|b| v.get(&b.heroe_id).is_some_and(|x| x.1));
        if amigos && sexto && salen {
            listo = true;
            break;
        }
    }
    let v = visto.lock().unwrap().clone();
    p.parar().await.expect("parar");
    assert!(listo, "{v:#?}\n{:#?}", Registro::leer(&ruta).unwrap());
}

/// A mano: un minuto real con el reloj ×`BOTS_FACTOR` (60: una hora de mundo) y la traza de órdenes (`BOTS_TRAZA=1 cargo test --test cerebro -- --ignored --nocapture`).
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn una_hora_a_x60() {
    let datos = tempfile::tempdir().unwrap();
    let Some((_servidor, dir)) = comun::servidor(datos.path()).await else { return };
    comun::acelerar(&dir, std::env::var("BOTS_FACTOR").ok().and_then(|f| f.parse().ok()).unwrap_or(60)).await;
    let cfg = Config { servidor: dir, codigo_bots: "bots".into(), registro: datos.path().join("registro.json"), total: 12, dias_llegada: 0.01, semilla: 3, cada_ms: 300_000, siempre: true, lugares: Default::default() };
    let p = Proceso::arrancar(cfg, Mira(Default::default())).await.expect("arrancar");
    tokio::time::sleep(Duration::from_secs(60)).await;
    p.parar().await.expect("parar");
}

/// Con el fondo ya en su almacén (dado por la administración, solo para la prueba), un solitario lo aporta, compra la Caravana de Fundación, sale con ella y funda.
#[tokio::test(flavor = "multi_thread")]
async fn con_el_fondo_compra_la_caravana_y_funda() {
    let datos = tempfile::tempdir().unwrap();
    let Some((_servidor, dir)) = comun::servidor(datos.path()).await else {
        eprintln!("sin binario del servidor o sin artefacto de mundo: se omite");
        return;
    };
    comun::acelerar(&dir, 60).await;
    let ruta = datos.path().join("registro.json");
    let cfg = Config { servidor: dir.clone(), codigo_bots: "bots".into(), registro: ruta.clone(), total: 1, dias_llegada: 1.0 / 86_400.0, semilla: 3, cada_ms: 60_000, siempre: true, lugares: Default::default() };
    let visto: Visto = Default::default();
    let p = Proceso::arrancar(cfg, Mira(visto.clone())).await.expect("arrancar");
    let heroe = loop {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if let Some(b) = Registro::leer(&ruta).unwrap().and_then(|r| r.bots.first().cloned()) {
            break b.heroe_id;
        }
    };
    for (recurso, n) in [("madera", 64), ("piedra", 8), ("trigo", 40), ("oro", 40)] {
        comun::dar_almacen(&dir, &heroe, recurso, (n * 1000) as f64).await;
    }
    let mut fundo = false;
    for _ in 0..1200 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if visto.lock().unwrap().get(&heroe).is_some_and(|x| x.2.as_deref().is_some_and(|r| r.starts_with("as-"))) {
            fundo = true;
            break;
        }
    }
    p.parar().await.expect("parar");
    assert!(fundo, "{:#?}", visto.lock().unwrap());
}
