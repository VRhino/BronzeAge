//! El puerto contra el servidor real (7C): un bot se da de alta, entra, crea su héroe, mira el mapa y, tras un corte, reanuda sin perder nada y sin foto.
//!
//! Necesita el binario del servidor (`cargo build --release -p servicio` en el motor; o `BRONZEAGE_SERVIDOR_BIN`) y un artefacto de mundo (`BRONZEAGE_MUNDO`, por defecto
//! `../BronzeAgeFase0/generador/salida/mundo`). Sin ellos se omite.

use bronce_age_bots_brain::{acceso, Credenciales, Puerto};
use contrato::v1::{self as pb, orden::Cuerpo as O, DecisionReanudacion, Resultado, TipoEntidad};

mod comun;
use comun::servidor;

#[tokio::test(flavor = "multi_thread")]
async fn un_bot_juega_por_el_contrato_y_reanuda_tras_un_corte() {
    let datos = tempfile::tempdir().unwrap();
    let Some((_servidor, dir)) = servidor(datos.path()).await else {
        eprintln!("sin binario del servidor o sin artefacto de mundo: se omite");
        return;
    };
    let c = Credenciales { nick: "bot-1".into(), clave: "clave de bot".into() };
    acceso::registrar(&dir, &c, "bots").await.expect("alta");
    let mut bot = Puerto::entrar(&dir, c.clone()).await.expect("entrar");
    assert!(bot.bienvenida.heroe_id.is_none(), "una cuenta nueva aún no tiene héroe");

    // Nace en un campamento, como cualquier jugador.
    let camp = bot.bienvenida.campamentos_para_elegir.first().expect("un campamento para nacer").clone();
    let r = bot.actuar(O::CrearHeroe(pb::OrdenCrearHeroe { display_name: "Bot 1".into(), campamento_id: camp.campamento_id, class_definition_id: "dagas".into(), ..Default::default() })).await.expect("crear_heroe");
    assert_eq!(r.resultado, Resultado::OrdenAceptada as i32, "{r:?}");
    let heroe = r.creadas.iter().find(|c| c.entidad == TipoEntidad::Heroe as i32).expect("nació un héroe").id.clone();
    assert!(bot.observar().ambitos.contains_key(&format!("HEROE:{heroe}")), "la respuesta causal ya trae su ámbito");

    // Mira alrededor del campamento.
    let p = camp.posicion.unwrap();
    let (min, max) = (pb::Punto { x: p.x - 20_000, y: p.y - 20_000 }, pb::Punto { x: p.x + 20_000, y: p.y + 20_000 });
    bot.mirar(pb::Rectangulo { min: Some(min), max: Some(max) }).await.expect("mirar");
    let antes = bot.observar().contenido();
    assert!(antes.keys().any(|k| k.starts_with("VISTA:")), "{:?}", antes.keys());

    // Corte de red: la siguiente orden reconecta con lo que conserva y el servidor no manda ninguna foto.
    bot.cortar();
    let r = bot.actuar(O::Atacar(Default::default())).await.expect("una orden tras el corte");
    assert_ne!(r.resultado, Resultado::OrdenDuplicada as i32);
    assert_eq!(bot.conexiones, 2);
    assert_eq!(bot.bienvenida.heroe_id.as_deref(), Some(heroe.as_str()));
    assert!(bot.bienvenida.ambitos.iter().all(|a| a.decision != DecisionReanudacion::ReanudaConFoto as i32), "{:?}", bot.bienvenida.ambitos);
    assert!(bot.observar().ambitos.keys().any(|k| k.starts_with("VISTA:")), "la VISTA se vuelve a abrir");

    // Lo acumulado es lo que daría una conexión nueva, sin nada guardado.
    let mut ahora = bot.observar().contenido();
    let mut nuevo = Puerto::entrar(&dir, c).await.expect("entrar de nuevo");
    let mut limpio = nuevo.observar().contenido();
    nuevo.cerrar().await;
    ahora.retain(|k, _| !k.starts_with("VISTA:"));
    limpio.retain(|k, _| !k.starts_with("VISTA:"));
    assert_eq!(ahora, limpio);
}
