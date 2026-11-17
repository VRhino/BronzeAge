//! La crónica entera fuera de la foto (sistema 6, decisión Q2; plan §5.3): se guarda en su diario, sobrevive a un apagado y a un corte sin duplicadas ni perdidas, se lee por páginas y no
//! cambia ni la versión ni el hash del mundo.

mod comun;

use comun::*;
use contrato::v1 as pb;
use nucleo::proyeccion::eventos::evento_dominio;
use nucleo::{Ajustes, Comando, Estado, Nucleo, Peticion, Relacion};
use persistencia::{Almacen, Aviso, Config, Diario, Sincronia};
use std::path::Path;

const SEGMENTO: u64 = 4 << 10;

fn persistente(dir: &Path, foto_cada: u64) -> Nucleo {
    persistente_con(dir, foto_cada, SEGMENTO)
}

fn persistente_con(dir: &Path, foto_cada: u64, max_segmento: u64) -> Nucleo {
    let mut ar = arranque(reticula(8, 8));
    ar.ajustes = Ajustes { foto_cada, latido_ms: 0, ..Ajustes::default() };
    Nucleo::con_persistencia(ar, dir, Config { max_segmento, sincronia: Sincronia::Nunca, fotos_conservadas: 3 }).expect("abrir")
}

/// La orden `i` del guion: guerra (que se canta) o paz, por turnos, entre dos Facciones vecinas.
fn orden(i: u32) -> Peticion {
    let a = 1 + (i / 2) % 4;
    let relacion = i.is_multiple_of(2).then_some(Relacion::Guerra);
    Peticion::nueva("admin", &format!("o{i}"), Comando::FijarRelacion { a, b: a + 1, relacion }).de_admin()
}

fn jugar(n: &mut Nucleo, desde: u32, hasta: u32) {
    for i in desde..hasta {
        let t = i64::from(i) * 1_000;
        let r = n.aplicar_en(t, orden(i));
        assert!(r.aceptada(), "{i}: {r:?}");
        n.avanzar_hasta(t + 500);
    }
}

/// El mismo guion en memoria, sin parar: la crónica que salió en vivo (la de las `Delta`), la versión y el hash.
fn referencia(hasta: u32) -> (Vec<pb::EventoDominio>, u64, [u8; 32]) {
    let mut n = nucleo(8, 8);
    jugar(&mut n, 0, hasta);
    let entradas = n.cambios_desde(0).expect("todo retenido").flat_map(|cv| cv.eventos.iter().filter(|e| e.codigo == "cronica.entrada").map(|e| evento_dominio(&n, e, cv.version))).collect();
    (entradas, n.version(), n.hash_estado())
}

fn pedir(n: &mut Nucleo, desde: Option<u64>, hasta: Option<u64>, limite: u32) -> pb::Cronica {
    let (tx, rx) = std::sync::mpsc::channel();
    n.pedir_cronica(pb::PedirCronica { peticion_id: "p".into(), desde_version: desde, hasta_version: hasta, limite }, Box::new(move |c| tx.send(c).unwrap()));
    rx.recv().unwrap()
}

/// Todo lo guardado, de página en página hacia delante.
fn leer_todo(n: &mut Nucleo, limite: u32) -> Vec<pb::EventoDominio> {
    let (mut todo, mut desde) = (Vec::new(), 0);
    loop {
        let p = pedir(n, Some(desde), None, limite);
        todo.extend(p.entradas.iter().cloned());
        if !p.mas_recientes {
            return todo;
        }
        desde = p.entradas.last().unwrap().version + 1;
    }
}

#[test]
fn se_guarda_entera_y_tras_apagar_y_recuperar_se_lee_igual_sin_cambiar_version_ni_hash() {
    let dir = tempfile::tempdir().unwrap();
    let (r60, v60, h60) = referencia(60);
    assert_eq!(r60.len(), 30, "una guerra cada dos órdenes");
    {
        let mut n = persistente(dir.path(), 9);
        jugar(&mut n, 0, 60);
        assert_eq!((n.version(), n.hash_estado()), (v60, h60), "guardar la crónica no cambia la versión ni el hash");
        assert_eq!(leer_todo(&mut n, 7), r60);
        n.apagar().unwrap();
    }
    assert!(std::fs::read_dir(dir.path().join("cronica")).unwrap().count() > 0, "la crónica vive en su propio directorio");
    let mut n = persistente(dir.path(), 9);
    assert!(n.recuperado() && n.avisos_de_recuperacion().is_empty(), "{:?}", n.avisos_de_recuperacion());
    assert_eq!(leer_todo(&mut n, 7), r60, "tras recuperar, la misma crónica, sin duplicadas");
    jugar(&mut n, 60, 120);
    let (r120, v120, h120) = referencia(120);
    assert_eq!((n.version(), n.hash_estado()), (v120, h120));
    assert_eq!(leer_todo(&mut n, 1_000), r120);
}

/// La versión de la última foto conservada.
fn version_de_la_foto(dir: &Path) -> u64 {
    let foto = Almacen::abrir(dir, Config::default()).unwrap().recuperar().unwrap().foto.expect("hay foto");
    Estado::deserializar(&foto.datos, &reglas(), "hash-de-prueba").unwrap().version
}

/// Tres cortes tras una foto: sin apagar (la crónica va por delante de la foto), con el lote siguiente perdido entero y con el lote a medias. En los tres, al recuperar queda exactamente
/// la crónica de la referencia, también al seguir jugando.
#[test]
fn un_corte_entre_una_foto_y_el_siguiente_lote_no_duplica_ni_pierde() {
    let (r60, _, _) = referencia(60);
    let (r120, v120, h120) = referencia(120);
    for corte in ["sin apagar", "lote perdido", "lote a medias"] {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut n = persistente(dir.path(), 50);
            jugar(&mut n, 0, 60);
            n.esperar_fotos();
        } // se suelta sin `apagar`
        let foto = version_de_la_foto(dir.path());
        assert!(r60.iter().any(|e| e.version > foto) && r60.iter().any(|e| e.version <= foto), "hay crónica a los dos lados de la foto (v{foto})");
        let cronica = dir.path().join("cronica");
        match corte {
            "lote perdido" => drop(Diario::abrir(&cronica, SEGMENTO, foto).unwrap()),
            "lote a medias" => {
                let ultimo = std::fs::read_dir(&cronica).unwrap().map(|e| e.unwrap().path()).max().unwrap();
                let largo = std::fs::metadata(&ultimo).unwrap().len();
                std::fs::OpenOptions::new().write(true).open(&ultimo).unwrap().set_len(largo - 3).unwrap();
            }
            _ => {}
        }
        let mut n = persistente(dir.path(), 50);
        let avisos = n.avisos_de_recuperacion().to_vec();
        assert_eq!(avisos.len(), usize::from(corte == "lote a medias"), "{corte}: {avisos:?}");
        assert!(avisos.iter().all(|a| matches!(a, Aviso::RegistroTruncado { .. })), "{corte}: {avisos:?}");
        assert_eq!(leer_todo(&mut n, 5), r60, "{corte}: ni duplicadas ni perdidas");
        jugar(&mut n, 60, 120);
        assert_eq!((n.version(), n.hash_estado()), (v120, h120), "{corte}");
        assert_eq!(leer_todo(&mut n, 200), r120, "{corte}");
    }
}

#[test]
fn las_paginas_van_de_200_por_defecto_a_1000_como_mucho_y_se_encadenan() {
    let dir = tempfile::tempdir().unwrap();
    let mut n = persistente_con(dir.path(), 10_000, 64 << 20);
    jugar(&mut n, 0, 2_500);
    let (r, v, _) = referencia(2_500);
    assert!(r.len() >= 1_250, "una guerra cada dos órdenes (y algún título): {}", r.len());
    let ultimas = pedir(&mut n, None, None, 0);
    assert_eq!((ultimas.entradas.as_slice(), ultimas.mas_antiguas, ultimas.mas_recientes, ultimas.version), (&r[r.len() - 200..], true, false, v), "las 200 últimas");
    assert_eq!(ultimas.peticion_id, "p");
    let todas = pedir(&mut n, None, None, 5_000);
    assert_eq!(todas.entradas.as_slice(), &r[r.len() - 1_000..], "como mucho 1.000");
    // Hasta una versión, hacia atrás, encadenando con la versión de la primera − 1: todo, sin repetir ni saltar.
    let (mut atras, mut hasta) = (Vec::new(), None);
    loop {
        let p = pedir(&mut n, None, hasta, 0);
        atras.splice(0..0, p.entradas.iter().cloned());
        if !p.mas_antiguas {
            break;
        }
        hasta = Some(p.entradas[0].version - 1);
    }
    assert_eq!(atras, r);
    let medio = r[600].version;
    let tramo = pedir(&mut n, Some(medio), Some(medio + 40), 3);
    assert_eq!((tramo.entradas.as_slice(), tramo.mas_antiguas, tramo.mas_recientes), (&r[600..603], true, true));
    // Sin persistencia no hay crónica guardada: la página va vacía (con la versión).
    let mut m = nucleo(8, 8);
    jugar(&mut m, 0, 4);
    let vacia = pedir(&mut m, None, None, 0);
    assert_eq!((vacia.entradas.len(), vacia.version, vacia.mas_antiguas), (0, m.version(), false));
}

/// Por el WebSocket real: quien llega tarde lee todo el historial; una entrada nueva está en la página en cuanto su orden tiene resultado, igual que la que llega en la `Delta`; y tras parar
/// y recuperar, la misma crónica.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn por_el_websocket_quien_llega_tarde_lee_todo_el_historial() {
    use comun::sesion::*;
    use contrato::v1::{mensaje_de_cliente as mc, mensaje_de_servidor as ms, orden, TipoCliente};
    use prost_types::{value::Kind, Struct, Value};
    use servidor::prueba::{hola, ClientePrueba};

    async fn pagina(c: &mut ClientePrueba, id: &str, hasta: Option<u64>) -> pb::Cronica {
        c.enviar(mc::Cuerpo::PedirCronica(pb::PedirCronica { peticion_id: id.into(), hasta_version: hasta, ..Default::default() })).await;
        loop {
            match c.esperar().await.cuerpo {
                Some(ms::Cuerpo::Cronica(p)) => {
                    assert_eq!(p.peticion_id, id);
                    return p;
                }
                Some(ms::Cuerpo::Delta(_) | ms::Cuerpo::Aviso(_)) => {}
                o => panic!("llegó {o:?}"),
            }
        }
    }
    let texto = |x: &str| Value { kind: Some(Kind::StringValue(x.into())) };

    let dir = tempfile::tempdir().unwrap();
    let mut n = persistente(dir.path(), 9);
    jugar(&mut n, 0, 40);
    let (r40, _, _) = referencia(40);
    let w = arrancar_con(n).await;
    let (mut tarde, _, _) = w.conectar(hola("ok:ana", TipoCliente::ClienteUnity)).await;
    let p = pagina(&mut tarde, "c1", None).await;
    assert_eq!((p.entradas.clone(), p.mas_antiguas), (r40.clone(), false), "las 20 entradas, en una página");

    // Una guerra nueva: la administración la oye en su `Delta` y, en cuanto tiene el resultado, ya está en la página de quien la pida.
    let (mut admin, _, _) = w.conectar(hola("ok:admin", TipoCliente::ClienteAdminWeb)).await;
    let guerra = orden::Cuerpo::Administracion(pb::OrdenDeAdministracion {
        codigo: "fijar_relacion".into(),
        parametros: Some(Struct { fields: [("faccion_a_id", texto("f-1")), ("faccion_b_id", texto("f-3")), ("relacion", texto("guerra"))].into_iter().map(|(k, v)| (k.to_owned(), v)).collect() }),
    });
    let (deltas, r) = dar(&mut admin, "g1", guerra).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    let en_vivo: Vec<_> = deltas.iter().flat_map(|d| d.eventos.iter().filter(|e| e.codigo == "cronica.entrada").cloned()).collect();
    assert_eq!(en_vivo.len(), 1, "la crónica llega en vivo como siempre");
    let p = pagina(&mut tarde, "c2", None).await;
    assert_eq!(p.entradas.last(), en_vivo.last(), "la página trae la misma entrada que la Delta");
    assert!(p.version >= r.version_tras);
    let antes = pagina(&mut tarde, "c3", Some(en_vivo[0].version - 1)).await;
    assert_eq!(antes.entradas, r40, "hasta una versión");
    let mut todo = r40;
    todo.extend(en_vivo);

    let mut n = w.parar().await;
    assert_eq!(leer_todo(&mut n, 200), todo);
    drop(n);
    let mut n = persistente(dir.path(), 9);
    assert_eq!(leer_todo(&mut n, 3), todo, "tras recuperar, la misma");
}

/// 6G por el WebSocket real: un público del sistema 6 (la Era que empieza por plazo) llega en vivo a quien no es de nadie en su `Delta` de `HEROE`, con su hecho tipado, su entrada de
/// crónica y la forma pública del estado de tecnología ya en la Era nueva; y la página de la crónica trae la misma entrada que la `Delta`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn por_el_websocket_la_era_que_empieza_llega_en_vivo_a_heroe_y_a_la_cronica() {
    use comun::sesion::*;
    use contrato::v1::{evento_dominio::Tipado, mensaje_de_cliente as mc, mensaje_de_servidor as ms, orden, TipoCliente};
    use servidor::prueba::hola;

    let dir = tempfile::tempdir().unwrap();
    let mut n = persistente(dir.path(), 1_000);
    // Una columna de la Facción 1 (la hace existir) y la cita de la Era, que se arma con lo primero que pasa.
    n.crear_ejercito(0, "ana", 1, c(2, 1), 500, 100_000).unwrap();
    assert!(n.aplicar_en(0, Peticion::nueva("admin", "t", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["escudos_ligeros".into()] }).de_admin()).aceptada());
    let r = n.estado().reglas.economia.tecnologia.clone();
    let plazo = r.eras[0].plazo;
    let w = arrancar_con(n).await;
    let (mut ana, _, recibido) = w.conectar(hola("ok:ana", TipoCliente::ClienteUnity)).await;
    let era_de = |m: &[ms::Cuerpo]| {
        m.iter()
            .flat_map(|x| match x {
                ms::Cuerpo::Foto(f) => f.entidades.iter().filter_map(|e| e.cuerpo.clone()).collect::<Vec<_>>(),
                ms::Cuerpo::Delta(d) => d.cambios.iter().filter_map(|c| c.datos.as_ref()?.cuerpo.clone()).collect(),
                _ => Vec::new(),
            })
            .filter_map(|c| if let pb::entidad::Cuerpo::EstadoTecnologia(t) = c { Some(t.era) } else { None })
            .last()
    };
    assert_eq!(era_de(&recibido), Some(r.eras[0].id.clone()), "la Era vigente, en su HEROE");
    let (mut admin, _, _) = w.conectar(hola("ok:admin", TipoCliente::ClienteAdminWeb)).await;
    let reloj = |accion: &str, hasta_ms: Option<i64>| orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: accion.into(), hasta_ms, ..Default::default() });
    let (_, x) = dar(&mut admin, "r1", reloj("pausar", None)).await;
    assert_eq!(x.resultado, pb::Resultado::OrdenAceptada as i32);
    let (_, x) = dar(&mut admin, "r2", reloj("avanzar_hasta", Some(plazo))).await;
    assert_eq!(x.resultado, pb::Resultado::OrdenAceptada as i32);
    let d = loop {
        match ana.esperar().await.cuerpo {
            Some(ms::Cuerpo::Delta(d)) if d.eventos.iter().any(|e| e.codigo == "era.comienza") => break d,
            Some(ms::Cuerpo::Delta(_) | ms::Cuerpo::Aviso(_)) => {}
            o => panic!("llegó {o:?}"),
        }
    };
    assert!(contrato::almacen::clave_de_ambito(&d.ambito).starts_with("HEROE:"), "por su HEROE: es pública");
    let era = d.eventos.iter().find(|e| e.codigo == "era.comienza").unwrap();
    assert_eq!(era.tipado, Some(Tipado::EraComenzada(pb::EraComenzada { era_id: r.eras[1].id.clone(), orden: 2, por_plazo: true })));
    assert_eq!(era_de(&[ms::Cuerpo::Delta(d.clone())]), Some(r.eras[1].id.clone()), "la forma pública, en la misma Delta");
    let en_vivo: Vec<pb::EventoDominio> = d.eventos.iter().filter(|e| e.codigo == "cronica.entrada").cloned().collect();
    assert!(!en_vivo.is_empty(), "su entrada de crónica, en vivo");
    ana.enviar(mc::Cuerpo::PedirCronica(pb::PedirCronica { peticion_id: "c1".into(), ..Default::default() })).await;
    let pagina = loop {
        match ana.esperar().await.cuerpo {
            Some(ms::Cuerpo::Cronica(p)) => break p,
            Some(ms::Cuerpo::Delta(_) | ms::Cuerpo::Aviso(_)) => {}
            o => panic!("llegó {o:?}"),
        }
    };
    assert!(en_vivo.iter().all(|e| pagina.entradas.contains(e)), "la página trae las mismas entradas que la Delta");
    drop(w.parar().await);
    drop(dir);
}

/// Un historial largo por el núcleo: 200.000 órdenes que dejan ≈ 100.000 entradas. Mide lo que cuesta escribirlo (el guion persistente; el mismo en memoria, que no guarda la crónica,
/// como comparación), lo que ocupa, recuperar (con el diario) y leer una página. `cargo test --release -p nucleo --test cronica historial_largo -- --ignored --nocapture`.
#[test]
#[ignore]
fn historial_largo_de_100000_entradas() {
    use std::time::Instant;
    let dir = tempfile::tempdir().unwrap();
    let t = Instant::now();
    let mut m = nucleo(8, 8);
    jugar(&mut m, 0, 200_000);
    let en_memoria = t.elapsed();
    {
        use prost::Message;
        let e = m.cambios_desde(m.version() - 10).unwrap().flat_map(|cv| cv.eventos.iter()).find(|e| e.codigo == "cronica.entrada").unwrap().clone();
        let t = Instant::now();
        let b: usize = (0..100_000).map(|i| evento_dominio(&m, &e, i).encode_to_vec().len()).sum();
        println!("proyectar y codificar 100.000 entradas: {:?} ({b})", t.elapsed());
    }
    let t = Instant::now();
    let mut n = persistente_con(dir.path(), 10_000, 64 << 20);
    jugar(&mut n, 0, 200_000);
    println!("solo las órdenes: {:?}", t.elapsed());
    n.apagar().unwrap();
    let con_cronica = t.elapsed();
    let bytes: u64 = std::fs::read_dir(dir.path().join("cronica")).unwrap().map(|e| e.unwrap().metadata().unwrap().len()).sum();
    let total = leer_todo(&mut n, 1_000).len();
    assert_eq!((n.version(), n.hash_estado()), (m.version(), m.hash_estado()));
    drop(n);
    let t = Instant::now();
    let mut n = persistente_con(dir.path(), 10_000, 64 << 20);
    let recuperar = t.elapsed();
    assert_eq!(leer_todo(&mut n, 1_000).len(), total);
    let medir = |n: &mut Nucleo, hasta: Option<u64>, limite| {
        let t = Instant::now();
        let k = (0..100).map(|_| pedir(n, None, hasta, limite).entradas.len()).sum::<usize>() / 100;
        (t.elapsed() / 100, k)
    };
    let ultimas = medir(&mut n, None, 0);
    let mitad_v = n.version() / 2;
    let mitad = medir(&mut n, Some(mitad_v), 1_000);
    println!(
        "{total} entradas, {bytes} bytes ({} por entrada); 200.000 órdenes en memoria {en_memoria:?}, persistentes con la crónica {con_cronica:?}; recuperar {recuperar:?}; \
         página de las 200 últimas {:?} ({}), 1.000 hasta la mitad {:?} ({})",
        bytes / total as u64,
        ultimas.0,
        ultimas.1,
        mitad.0,
        mitad.1
    );
}
