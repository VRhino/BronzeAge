//! Pruebas del núcleo en modo acelerado: órdenes, eventos, invalidación, idempotencia, determinismo y recuperación.

mod comun;

use base::{Instante, Punto, Ritmo};
use comun::*;
use nucleo::{rechazos, AccionReloj, Ajustes, Comando, Estado, Nucleo, Peticion, Reglas, Resultado};
use persistencia::{Config, Sincronia};
use std::sync::{Arc, Mutex};

fn eventos(n: &Nucleo) -> Vec<(String, Instante, u64)> {
    n.cambios_desde(0)
        .expect("registro completo")
        // Los hechos de alguna columna (los títulos de prestigio, 5F, no son de ninguna).
        .flat_map(|cv| cv.eventos.iter().filter_map(|e| Some((e.codigo.to_string(), e.en, *e.ejercitos.first()?))))
        .collect()
}

fn alta(n: &mut Nucleo, dueno: &str, p: Punto, velocidad: i64, viveres: i64) -> u64 {
    n.crear_ejercito(0, dueno, 1, p, velocidad, viveres).expect("alta")
}

#[test]
fn marcha_y_llegada_exactas() {
    let mut n = nucleo(12, 12);
    let id = alta(&mut n, "ana", c(1, 1), 4_000, 350_000); // 8.000 milésimas = 2.000 ms
    assert_eq!(n.version(), 1, "el alta es la versión 1");
    let r = n.aplicar_en(0, marchar("ana", "m1", id, c(9, 1)));
    assert!(r.aceptada());
    assert_eq!(r.version_tras, 2);
    let e = &n.estado().ejercitos[&id];
    assert_eq!(e.plan.fin(), Some(16_000), "8 espaciados a 2 s");
    assert_eq!(e.plan.generacion, 1);

    n.avanzar_hasta(15_999);
    assert!(n.estado().ejercitos[&id].marchando(), "aún no llegó");
    assert_eq!(n.version(), 2);
    n.avanzar_hasta(16_000);
    let e = &n.estado().ejercitos[&id];
    assert!(!e.marchando());
    assert_eq!(e.posicion, c(9, 1));
    assert_eq!(n.version(), 3, "la llegada es un cambio de versión");
    assert_eq!(eventos(&n), vec![("jugador.marcha_fijada".to_string(), 0, id), ("ejercito.llega".to_string(), 16_000, id)], "la orden cuenta la marcha y el reloj la llegada");
    // Los víveres pasaron a ritmo de acampada en el instante de la llegada, sin saltos.
    let v = e.columna.viveres;
    assert_eq!((v.t0, v.por_minuto), (16_000, -13), "acampada, un héroe solo come 0,5 × ¼ × ⅒ por minuto");
    assert_eq!(v.valor_en(16_000), Ritmo { valor_en_t0: 350_000, t0: 0, por_minuto: -125, maximo: None, minimo: None }.valor_en(16_000), "en marcha come 0,5 × ¼ por minuto");
}

#[test]
fn una_orden_a_mitad_de_camino_parte_de_la_posicion_real_e_invalida_la_llegada_vieja() {
    let mut n = nucleo(12, 12);
    let id = alta(&mut n, "ana", c(1, 1), 4_000, 350_000);
    n.aplicar_en(0, marchar("ana", "m1", id, c(9, 1)));
    let plan_viejo = n.estado().ejercitos[&id].plan.clone();
    let t = 7_000;
    let donde = plan_viejo.posicion_en(t).unwrap();
    assert!(donde.x > c(1, 1).x && donde.x < c(9, 1).x);

    let r = n.aplicar_en(t, marchar("ana", "m2", id, c(5, 9)));
    assert!(r.aceptada());
    let nuevo = &n.estado().ejercitos[&id].plan;
    assert_eq!((nuevo.vertices[0].p, nuevo.vertices[0].en), (donde, t), "el plan nuevo empieza donde está la columna");
    assert_eq!(nuevo.generacion, 2);
    let llegada = nuevo.fin().unwrap();

    n.avanzar_hasta(plan_viejo.fin().unwrap() + 1);
    assert!(n.estado().ejercitos[&id].marchando(), "la llegada vieja no se aplicó");
    assert!(n.eventos_descartados() >= 1);
    n.avanzar_hasta(llegada);
    assert_eq!(n.estado().ejercitos[&id].posicion, c(5, 9));
    assert_eq!(eventos(&n).iter().filter(|e| e.0 == "ejercito.llega").count(), 1);
}

#[test]
fn estacionar_detiene_donde_esta() {
    let mut n = nucleo(12, 12);
    let id = alta(&mut n, "ana", c(1, 1), 4_000, 350_000);
    n.aplicar_en(0, marchar("ana", "m1", id, c(9, 1)));
    let esperado = n.estado().ejercitos[&id].plan.posicion_en(5_000).unwrap();
    let r = n.aplicar_en(5_000, Peticion::nueva("ana", "e1", Comando::Estacionar { ejercito: id }));
    assert!(r.aceptada());
    let e = &n.estado().ejercitos[&id];
    assert_eq!((e.marchando(), e.posicion), (false, esperado));
    n.avanzar_hasta(60_000);
    assert_eq!(n.estado().ejercitos[&id].posicion, esperado, "la llegada vieja no la mueve");
    let otra = n.aplicar_en(60_000, Peticion::nueva("ana", "e2", Comando::Estacionar { ejercito: id }));
    assert_eq!(otra.resultado, Resultado::SinCambios, "ya estaba quieta");
}

#[test]
fn los_viveres_son_un_ritmo_con_eventos_de_umbral() {
    let mut ar = arranque(reticula(60, 4));
    ar.reglas.viveres_bajos_pct = 1; // aviso a 3,5 unidades (el 1 % de los 350 que caben en un héroe)
    let mut n = Nucleo::nuevo(ar);
    let id = n.crear_ejercito(0, "ana", 1, c(1, 1), 100, 5_000).unwrap(); // 5 unidades de víveres, 80 s por espaciado
    let r = n.aplicar_en(0, marchar("ana", "m1", id, c(50, 1)));
    assert!(r.aceptada());
    let plan = n.estado().ejercitos[&id].plan.clone();
    let ritmo = Ritmo { valor_en_t0: 5_000, t0: 0, por_minuto: -125, maximo: Some(350_000), minimo: Some(0) };
    let (bajo, cero) = (ritmo.instante_de_valor(3_500, 0).unwrap(), ritmo.instante_de_valor(0, 0).unwrap());
    assert!(cero < plan.fin().unwrap(), "se queda sin víveres antes de llegar");

    n.avanzar_hasta(bajo - 1);
    // Lo que hace el reloj: sin el hecho de la orden de marchar.
    let del_reloj = |n: &Nucleo| eventos(n).into_iter().filter(|e| e.0 != "jugador.marcha_fijada").collect::<Vec<_>>();
    assert!(del_reloj(&n).is_empty(), "nada ocurre entre eventos: solo hay que evaluar la fórmula");
    assert_eq!(n.estado().ejercitos[&id].columna.viveres.valor_en(bajo - 1), 3_501);
    n.avanzar_hasta(bajo);
    assert_eq!(del_reloj(&n), vec![("ejercito.viveres_bajos".to_string(), bajo, id)]);
    n.avanzar_hasta(cero);
    assert_eq!(del_reloj(&n).last().unwrap(), &("ejercito.sin_viveres".to_string(), cero, id));
    let e = &n.estado().ejercitos[&id];
    assert_eq!(e.columna.viveres.valor_en(cero), 0);
    assert_eq!(e.columna.viveres.valor_en(cero + 600_000), 0, "el suelo se respeta");
    assert!(e.marchando(), "la columna no se detiene por hambre (sin tropa no hay a quién desertar)");
    // Cada umbral ocurre una sola vez: tras llegar, la columna acampa con 0 y la cola queda vacía.
    n.avanzar_hasta(plan.fin().unwrap() + 10_000);
    assert_eq!(del_reloj(&n).iter().map(|e| e.0.as_str()).collect::<Vec<_>>(), ["ejercito.viveres_bajos", "ejercito.sin_viveres", "ejercito.llega"]);
    // Los umbrales que el alta programó con el ritmo de acampada quedaron invalidados; salen de la cola (descartados) cuando les toca.
    n.avanzar_hasta(30_000_000);
    // Solo queda la cita global del fin del plazo de la Era (6B), que no es de ninguna entidad ociosa.
    let cola: Vec<nucleo::Carga> = n.estado().cola.volcar().into_iter().map(|e| e.carga).collect();
    assert_eq!(cola, vec![nucleo::Carga::Era]);
}

#[test]
fn idempotencia_version_esperada_y_autorizacion() {
    let mut n = nucleo(12, 12);
    let id = alta(&mut n, "ana", c(1, 1), 4_000, 350_000);
    let primera = n.aplicar_en(0, marchar("ana", "m1", id, c(9, 1)));
    let hash = n.hash_estado();
    let version = n.version();
    let otra = n.aplicar_en(100, marchar("ana", "m1", id, c(1, 9)));
    assert_eq!(otra.resultado, Resultado::Duplicada);
    assert_eq!(otra.version_tras, primera.version_tras);
    assert_eq!((n.version(), n.hash_estado()), (version, hash), "una duplicada no toca nada");
    // Otro actor con el mismo peticion_id no es la misma petición (y no es el dueño).
    let ajeno = n.aplicar_en(100, marchar("beto", "m1", id, c(1, 9)));
    assert!(matches!(ajeno.resultado, Resultado::Rechazada { codigo: rechazos::NO_AUTORIZADO, .. }));
    // version_esperada
    let mut p = marchar("ana", "m3", id, c(3, 3));
    p.version_esperada = Some(version - 1);
    let obsoleta = n.aplicar_en(100, p);
    assert!(matches!(obsoleta.resultado, Resultado::Rechazada { codigo: rechazos::VERSION_OBSOLETA, .. }));
    let mut p = marchar("ana", "m4", id, c(3, 3));
    p.version_esperada = Some(n.version());
    assert!(n.aplicar_en(100, p).aceptada());
}

#[test]
fn rechazadas_y_sin_cambios_no_dejan_rastro() {
    let mut n = nucleo(12, 12);
    let id = alta(&mut n, "ana", c(1, 1), 4_000, 350_000);
    n.aplicar_en(0, marchar("ana", "m1", id, c(9, 1)));
    n.avanzar_hasta(1_000);
    let (v, h, sig) = (n.version(), n.hash_estado(), n.estado().siguiente_id);
    let rng_antes = n.estado().rng.estado();

    let lejos = Punto::new(50_000_000, 50_000_000);
    let casos = [
        marchar("beto", "x1", id, c(3, 3)),                                             // no es suya
        marchar("ana", "x2", 999, c(3, 3)),                                             // no existe
        marchar("ana", "x3", id, lejos),                                                // sin ruta
        Peticion::nueva("ana", "x4", Comando::Reloj(AccionReloj::Pausar)),              // no es admin
        Peticion::nueva("ana", "x5", Comando::CrearEjercito { cuenta: "ana".into(), faccion: 1, posicion: c(2, 2), velocidad: 1, viveres: 1 }),
    ];
    for p in casos {
        let r = n.aplicar_en(1_000, p);
        assert!(matches!(r.resultado, Resultado::Rechazada { .. }), "{r:?}");
        assert_eq!(r.version_tras, v);
    }
    let sin = n.aplicar_en(1_000, Peticion::nueva("admin", "x6", Comando::Reloj(AccionReloj::Reanudar)).de_admin());
    assert_eq!(sin.resultado, Resultado::SinCambios);
    // Nada de lo anterior dejó rastro: ni versión, ni ids gastados, ni dados, ni cambio en el hash.
    assert_eq!((n.version(), n.hash_estado(), n.estado().siguiente_id, n.estado().rng.estado()), (v, h, sig, rng_antes));
}

#[test]
fn control_del_reloj_solo_para_administracion() {
    let mut n = nucleo(4, 4);
    let admin = |id: &str, a| Peticion::nueva("admin", id, Comando::Reloj(a)).de_admin();
    assert!(n.aplicar_en(0, admin("r1", AccionReloj::FijarFactor(60_000))).aceptada());
    assert_eq!(n.estado().factor_milesimas, 60_000);
    assert_eq!(n.aplicar_en(0, admin("r2", AccionReloj::FijarFactor(60_000))).resultado, Resultado::SinCambios);
    assert!(matches!(n.aplicar_en(0, admin("r3", AccionReloj::FijarFactor(0))).resultado, Resultado::Rechazada { codigo: rechazos::RELOJ_INVALIDO, .. }));
    assert!(n.aplicar_en(0, admin("r4", AccionReloj::Pausar)).aceptada());
    assert!(n.estado().pausado);
    assert!(n.aplicar_en(0, admin("r5", AccionReloj::Reanudar)).aceptada());
    assert!(n.aplicar_en(0, admin("r6", AccionReloj::AvanzarHasta(10_000))).aceptada());
    assert_eq!(n.ahora(), 10_000);
    assert!(matches!(n.aplicar_en(10_000, admin("r7", AccionReloj::AvanzarHasta(9_000))).resultado, Resultado::Rechazada { .. }));
}

#[test]
fn el_estado_se_serializa_y_se_recupera_igual() {
    let mut n = nucleo(30, 30);
    let esc = Escenario { w: 30, h: 30, n: 40, semilla: 7 };
    let ids = esc.crear(&mut n);
    esc.ordenes(&mut n, &ids, 0, 120, &|n, t| n.avanzar_hasta(t));
    let bytes = n.estado().serializar();
    let e2 = Estado::deserializar(&bytes, &reglas(), "hash-de-prueba").expect("deserializa");
    assert_eq!(e2.hash(), n.hash_estado());
    assert_eq!(e2.serializar(), bytes);
    let otras = Reglas { viveres_bajos_pct: 2, ..reglas() };
    assert!(Estado::deserializar(&bytes, &otras, "hash-de-prueba").is_err(), "reglas distintas: se niega");
    assert!(Estado::deserializar(&bytes, &reglas(), "otro-mundo").is_err(), "otro mundo: se niega");
    assert!(Estado::deserializar(&bytes[..bytes.len() - 3], &reglas(), "hash-de-prueba").is_err(), "truncado");
}

/// Corre el escenario completo y devuelve el hash final.
fn hash_de(semilla_esc: u64, avanzar: &dyn Fn(&mut Nucleo, Instante)) -> [u8; 32] {
    let mut n = nucleo(30, 30);
    let esc = Escenario { w: 30, h: 30, n: 40, semilla: semilla_esc };
    let ids = esc.crear(&mut n);
    esc.ordenes(&mut n, &ids, 0, 300, avanzar);
    n.avanzar_hasta(300 * 3_000 + 600_000);
    n.hash_estado()
}

#[test]
fn misma_semilla_y_mismas_ordenes_dan_el_mismo_hash_y_da_igual_como_se_trocee_el_avance() {
    let a = hash_de(11, &|n, t| n.avanzar_hasta(t));
    assert_eq!(a, hash_de(11, &|n, t| n.avanzar_hasta(t)), "repetible");
    // Sin avances intermedios (solo el que hace cada orden al entrar)...
    assert_eq!(a, hash_de(11, &|_, _| {}), "el troceado del avance no cambia el mundo");
    // ...o en pasos pequeños.
    assert_eq!(
        a,
        hash_de(11, &|n, t| {
            let ahora = n.ahora();
            for s in 1..=3 {
                n.avanzar_hasta(ahora + (t - ahora).max(0) * s / 3);
            }
        })
    );
    assert_ne!(a, hash_de(12, &|n, t| n.avanzar_hasta(t)), "otras órdenes, otro mundo");
}

#[test]
fn un_lote_con_rutas_en_paralelo_es_idéntico_a_aplicar_una_a_una() {
    let crear = |n: &mut Nucleo| -> Vec<u64> { (0..60).map(|i| alta_en(n, i)).collect() };
    fn alta_en(n: &mut Nucleo, i: i64) -> u64 {
        n.crear_ejercito(0, "ana", 1, c(i % 20, i / 20), 3_000, 100_000).unwrap()
    }
    let pet = |ids: &[u64]| -> Vec<Peticion> {
        let mut v: Vec<Peticion> = ids.iter().map(|&id| marchar("ana", &format!("l{id}"), id, c((id as i64 * 7) % 25, (id as i64 * 11) % 25))).collect();
        v.push(marchar("ana", "repetida", ids[0], c(3, 3))); // el mismo ejército otra vez: se calcula en línea
        v
    };
    let (mut a, mut b) = (nucleo(30, 30), nucleo(30, 30));
    let (ia, ib) = (crear(&mut a), crear(&mut b));
    let ra = a.marchas_en_lote(5_000, pet(&ia));
    let rb: Vec<_> = pet(&ib).into_iter().map(|p| b.aplicar_en(5_000, p)).collect();
    assert_eq!(ra, rb);
    assert!(ra.iter().all(|r| r.aceptada()));
    assert_eq!(a.hash_estado(), b.hash_estado());
    a.avanzar_hasta(2_000_000);
    b.avanzar_hasta(2_000_000);
    assert_eq!(a.hash_estado(), b.hash_estado());
}

fn persistencia_en(dir: &std::path::Path, foto_cada: u64) -> Nucleo {
    let mut ar = arranque(reticula(30, 30));
    ar.ajustes = Ajustes { foto_cada, latido_ms: 0, ..Ajustes::default() };
    let cfg = Config { max_segmento: 64 << 10, sincronia: Sincronia::Nunca, fotos_conservadas: 3 };
    Nucleo::con_persistencia(ar, dir, cfg).expect("abrir")
}

fn fotos_en(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir).unwrap().filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().starts_with("foto-")).count()
}

/// Referencia: todo el escenario en memoria, sin parar.
fn referencia() -> [u8; 32] {
    let mut n = nucleo(30, 30);
    let esc = Escenario { w: 30, h: 30, n: 40, semilla: 7 };
    let ids = esc.crear(&mut n);
    esc.ordenes(&mut n, &ids, 0, 300, &|n, t| n.avanzar_hasta(t));
    n.avanzar_hasta(300 * 3_000 + 600_000);
    n.hash_estado()
}

/// Apaga y recupera a mitad del escenario (con o sin fotos), sigue y debe llegar al mismo hash que la referencia.
fn con_caida(foto_cada: u64) {
    let dir = tempfile::tempdir().unwrap();
    let esc = Escenario { w: 30, h: 30, n: 40, semilla: 7 };
    let (antes, ids) = {
        let mut n = persistencia_en(dir.path(), foto_cada);
        assert!(!n.recuperado());
        let ids = esc.crear(&mut n);
        esc.ordenes(&mut n, &ids, 0, 150, &|n, t| n.avanzar_hasta(t));
        n.avanzar_hasta(150 * 3_000);
        n.esperar_fotos();
        (n.hash_estado(), ids)
    }; // se suelta sin `apagar`: como una caída tras escribir
    if foto_cada > 0 {
        assert!(fotos_en(dir.path()) > 0, "hubo fotos");
    }
    let mut n = persistencia_en(dir.path(), foto_cada);
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), antes, "recuperar da exactamente el mismo estado");
    assert_eq!(n.ahora(), 150 * 3_000);
    esc.ordenes(&mut n, &ids, 150, 300, &|n, t| n.avanzar_hasta(t));
    n.avanzar_hasta(300 * 3_000 + 600_000);
    assert_eq!(n.hash_estado(), referencia(), "tras recuperar el mundo sigue igual que si no hubiera caído");
    n.apagar().unwrap();
}

#[test]
fn recuperar_solo_con_el_registro() {
    con_caida(0);
}

#[test]
fn recuperar_desde_foto_y_registro() {
    con_caida(25);
}

#[test]
fn recuperar_dos_veces_seguidas_da_lo_mismo() {
    let dir = tempfile::tempdir().unwrap();
    let hash = {
        let mut n = persistencia_en(dir.path(), 10);
        let esc = Escenario { w: 30, h: 30, n: 20, semilla: 3 };
        let ids = esc.crear(&mut n);
        esc.ordenes(&mut n, &ids, 0, 60, &|n, t| n.avanzar_hasta(t));
        n.avanzar_hasta(200_000);
        n.hash_estado()
    };
    for _ in 0..2 {
        let n = persistencia_en(dir.path(), 10);
        assert_eq!(n.hash_estado(), hash);
    }
}

struct Espia {
    planes: Arc<Mutex<Vec<(u64, usize)>>>,
}

impl nucleo::GeneradorDeEventos for Espia {
    fn plan_cambiado(&mut self, cx: &mut nucleo::Cx<'_>, ejercito: u64, plan: &base::Plan) {
        self.planes.lock().unwrap().push((ejercito, plan.vertices.len()));
        let (a, b) = (plan.inicio().unwrap(), plan.fin().unwrap());
        cx.programar((a + b) / 2, 5, ejercito, 1, 0, 0); // «a mitad de camino»
    }
    fn plan_retirado(&mut self, _cx: &mut nucleo::Cx<'_>, ejercito: u64) {
        self.planes.lock().unwrap().push((ejercito, 0));
    }
    fn al_vencer(&mut self, cx: &mut nucleo::Cx<'_>, entidad: u64, tipo: u16, _a: u64, _b: u64) {
        assert_eq!(tipo, 1);
        let punto = cx.estado().ejercitos[&entidad].plan.posicion_en(cx.ahora());
        cx.emitir(nucleo::EventoNucleo { codigo: "espia.medio".into(), en: cx.ahora(), ejercitos: vec![entidad], asentamientos: vec![], punto, datos: vec![], ..Default::default() });
    }
}

#[test]
fn un_generador_programa_eventos_propios_que_se_invalidan_con_el_plan() {
    let mut n = nucleo(12, 12);
    let planes = Arc::new(Mutex::new(Vec::new()));
    n.agregar_generador(Box::new(Espia { planes: planes.clone() }));
    let (a, b) = (alta(&mut n, "ana", c(1, 1), 4_000, 350_000), alta(&mut n, "ana", c(1, 5), 4_000, 350_000));
    n.aplicar_en(0, marchar("ana", "m1", a, c(9, 1))); // mitad en 8.000
    n.aplicar_en(0, marchar("ana", "m2", b, c(9, 5)));
    n.aplicar_en(4_000, marchar("ana", "m3", b, c(1, 11))); // cambia de plan antes de su mitad: la vieja se invalida
    n.avanzar_hasta(100_000);
    let e = eventos(&n);
    assert_eq!(e.iter().filter(|x| x.0 == "espia.medio" && x.2 == a).count(), 1);
    assert_eq!(e.iter().filter(|x| x.0 == "espia.medio" && x.2 == b).count(), 1, "solo el del plan vigente");
    let mitad_a = e.iter().find(|x| x.0 == "espia.medio" && x.2 == a).unwrap().1;
    assert_eq!(mitad_a, 8_000);
    let vistos = planes.lock().unwrap();
    assert!(vistos.iter().filter(|(id, k)| *id == a && *k == 0).count() == 1, "avisó de que dejó de moverse");
    assert_eq!(vistos.iter().filter(|(_, k)| *k > 0).count(), 3);
}

#[test]
fn los_generadores_reciben_los_planes_vigentes_al_registrarse() {
    let mut n = nucleo(12, 12);
    let a = alta(&mut n, "ana", c(1, 1), 4_000, 350_000);
    n.aplicar_en(0, marchar("ana", "m1", a, c(9, 1)));
    struct Vigente(Arc<Mutex<Vec<u64>>>);
    impl nucleo::GeneradorDeEventos for Vigente {
        fn plan_vigente(&mut self, ejercito: u64, _plan: &base::Plan) {
            self.0.lock().unwrap().push(ejercito);
        }
    }
    let v = Arc::new(Mutex::new(vec![]));
    n.agregar_generador(Box::new(Vigente(v.clone())));
    assert_eq!(*v.lock().unwrap(), vec![a]);
}

#[test]
fn el_muestreo_es_periodico_en_tiempo_de_mundo() {
    struct Cuenta(Arc<Mutex<Vec<Instante>>>);
    impl nucleo::GeneradorDeEventos for Cuenta {
        fn muestrear(&mut self, cx: &mut nucleo::Cx<'_>) {
            self.0.lock().unwrap().push(cx.ahora());
        }
    }
    let mut ar = arranque(reticula(4, 4));
    ar.ajustes.muestreo_ms = 250;
    let mut n = Nucleo::nuevo(ar);
    let v = Arc::new(Mutex::new(vec![]));
    n.agregar_generador(Box::new(Cuenta(v.clone())));
    n.avanzar_hasta(1_000);
    n.avanzar_hasta(1_600);
    assert_eq!(*v.lock().unwrap(), vec![250, 500, 750, 1_000, 1_250, 1_500]);
    assert_eq!(n.version(), 0, "muestrear sin emitir nada no cambia el mundo");
}

#[test]
fn el_orden_de_los_eventos_del_mismo_instante_respeta_la_prioridad() {
    // Se queda sin víveres (prioridad 3) en el mismo instante en que llega (prioridad 8): primero los víveres.
    let mut ar = arranque(reticula(12, 12));
    ar.reglas.viveres_bajos_pct = 0;
    let mut n = Nucleo::nuevo(ar);
    let id = n.crear_ejercito(0, "ana", 1, c(1, 1), 1_000, 50).unwrap(); // 50 milésimas de víveres a 125 por minuto: se acaban a los 24 s; el plan de 3 espaciados a 1 u/s dura 24 s
    n.aplicar_en(0, marchar("ana", "m1", id, c(4, 1)));
    assert_eq!(n.estado().ejercitos[&id].plan.fin(), Some(24_000));
    n.avanzar_hasta(30_000);
    assert_eq!(eventos(&n).iter().map(|e| (e.0.as_str(), e.1)).collect::<Vec<_>>(), [("jugador.marcha_fijada", 0), ("ejercito.sin_viveres", 24_000), ("ejercito.llega", 24_000)]);
    assert_eq!(n.estado().ejercitos[&id].posicion, c(4, 1));
}

#[test]
fn el_escenario_aleatorio_ejercita_todos_los_caminos() {
    let mut n = nucleo(30, 30);
    let esc = Escenario { w: 30, h: 30, n: 40, semilla: 7 };
    let ids = esc.crear(&mut n);
    esc.ordenes(&mut n, &ids, 0, 300, &|n, t| n.avanzar_hasta(t));
    n.avanzar_hasta(300 * 3_000 + 600_000);
    let m = n.metricas();
    assert!(m.ordenes_aceptadas > 200, "{m:?}");
    assert!(m.eventos > 150, "{m:?}");
    assert!(n.eventos_descartados() > 10, "se invalidaron planes a mitad de camino");
    let codigos: std::collections::BTreeSet<_> = eventos(&n).into_iter().map(|e| e.0).collect();
    assert_eq!(codigos.into_iter().collect::<Vec<_>>(), ["ejercito.encuentro", "ejercito.estacionado", "ejercito.llega", "ejercito.sin_viveres", "ejercito.viveres_bajos", "jugador.marcha_fijada"]);
}

#[test]
fn la_memoria_de_idempotencia_es_acotada() {
    let mut ar = arranque(reticula(12, 12));
    ar.reglas.limite_idempotencia = 3;
    let mut n = Nucleo::nuevo(ar);
    let id = alta(&mut n, "ana", c(1, 1), 4_000, 350_000);
    for i in 0..4 {
        assert!(n.aplicar_en(i, marchar("ana", &format!("p{i}"), id, c(2 + i, 5))).aceptada());
    }
    // `p0` ya salió de la memoria (se recuerdan 3): se vuelve a ejecutar; `p3` sigue recordada.
    assert_eq!(n.aplicar_en(10, marchar("ana", "p3", id, c(9, 9))).resultado, Resultado::Duplicada);
    assert!(n.aplicar_en(10, marchar("ana", "p0", id, c(9, 9))).aceptada());
}

#[test]
fn la_memoria_de_idempotencia_es_de_cada_cuenta() {
    let mut ar = arranque(reticula(12, 12));
    ar.reglas.limite_idempotencia = 3;
    let mut n = Nucleo::nuevo(ar);
    let (ana, beto) = (alta(&mut n, "ana", c(1, 1), 4_000, 350_000), alta(&mut n, "beto", c(1, 3), 4_000, 350_000));
    assert!(n.aplicar_en(0, marchar("ana", "a0", ana, c(2, 5))).aceptada());
    // Muchas órdenes de otra cuenta no le quitan a Ana las suyas: cada cuenta recuerda sus últimas 3.
    for i in 0..10 {
        assert!(n.aplicar_en(i, marchar("beto", &format!("b{i}"), beto, c(2 + i % 8, 7))).aceptada());
    }
    assert_eq!(n.aplicar_en(20, marchar("ana", "a0", ana, c(9, 9))).resultado, Resultado::Duplicada, "el reintento de Ana sigue siendo un duplicado");
    assert_eq!(n.aplicar_en(20, marchar("beto", "b9", beto, c(9, 9))).resultado, Resultado::Duplicada);
    assert!(n.aplicar_en(20, marchar("beto", "b0", beto, c(9, 9))).aceptada(), "la de Beto más vieja ya salió de su memoria");
    // Y la memoria va en la foto, por cuenta.
    let vuelta = nucleo::Estado::deserializar(&n.estado().serializar(), &n.estado().reglas, &n.estado().mundo_hash).unwrap();
    assert_eq!((vuelta.visto("ana", "a0"), vuelta.visto("beto", "b1")), (n.estado().visto("ana", "a0"), None));
    assert!(vuelta.visto("ana", "a0").is_some());
}

#[test]
fn modo_en_vivo_con_el_reloj_compartido() {
    use servidor::RelojCompartido;
    use std::{thread::sleep, time::Duration};
    let reloj = RelojCompartido::nuevo(0);
    let mut n = nucleo(12, 12);
    n.conectar_reloj(reloj.clone());
    let id = alta(&mut n, "ana", c(1, 1), 4_000, 350_000);
    let admin = |id: &str, a| Peticion::nueva("admin", id, Comando::Reloj(a)).de_admin();

    assert!(n.aplicar(admin("r1", AccionReloj::FijarFactor(3_600_000))).aceptada()); // ×3600: 1 ms de pared = 3,6 s de mundo
    assert_eq!(reloj.reloj().factor_milesimas, 3_600_000);
    assert!(n.aplicar(marchar("ana", "m1", id, c(9, 1))).aceptada(), "la orden lleva el instante de mundo del reloj");
    sleep(Duration::from_millis(60));
    n.pulso();
    assert!(!n.estado().ejercitos[&id].marchando(), "16 s de mundo pasaron en un pulso");
    assert!(n.ahora() >= 16_000);

    // Pausa: el reloj se congela y el pulso no avanza nada.
    assert!(n.aplicar(admin("r2", AccionReloj::Pausar)).aceptada());
    let congelado = reloj.mundo_ahora();
    sleep(Duration::from_millis(20));
    assert_eq!(reloj.mundo_ahora(), congelado);
    n.pulso();
    assert_eq!(n.ahora(), congelado);
    // `avanzar_hasta` adelanta el mundo y el reloj con él.
    assert!(n.aplicar(admin("r3", AccionReloj::AvanzarHasta(congelado + 1_000_000))).aceptada());
    assert_eq!((n.ahora(), reloj.mundo_ahora()), (congelado + 1_000_000, congelado + 1_000_000));
    assert!(n.aplicar(admin("r4", AccionReloj::Reanudar)).aceptada());
    sleep(Duration::from_millis(20));
    n.pulso();
    assert!(n.ahora() > congelado + 1_000_000);
    assert!(n.estado().factor_milesimas == 3_600_000 && !n.estado().pausado);
}

#[test]
fn recuperar_con_otro_mundo_se_niega_y_el_reloj_vuelve_como_estaba() {
    use servidor::RelojCompartido;
    let dir = tempfile::tempdir().unwrap();
    let con_hash = |hash: &str, foto_cada| {
        let mut ar = arranque(reticula(30, 30));
        ar.mundo_hash = hash.into();
        ar.ajustes = Ajustes { foto_cada, latido_ms: 0, ..Ajustes::default() };
        Nucleo::con_persistencia(ar, dir.path(), Config { max_segmento: 64 << 10, sincronia: Sincronia::Nunca, fotos_conservadas: 3 })
    };
    {
        let mut n = con_hash("mundo-a", 0).unwrap();
        let admin = |id: &str, a| Peticion::nueva("admin", id, Comando::Reloj(a)).de_admin();
        n.aplicar_en(0, admin("r1", AccionReloj::FijarFactor(60_000)));
        n.aplicar_en(0, admin("r2", AccionReloj::AvanzarHasta(5_000_000)));
        n.aplicar_en(5_000_000, admin("r3", AccionReloj::Pausar));
        n.apagar().unwrap();
    }
    assert!(con_hash("mundo-b", 0).is_err(), "el registro es de otro mundo");
    let mut n = con_hash("mundo-a", 0).unwrap();
    assert_eq!(n.ahora(), 5_000_000);
    let reloj = RelojCompartido::nuevo(123_456_789); // el reloj nuevo no sabe nada: se alinea con el estado recuperado
    n.conectar_reloj(reloj.clone());
    let r = reloj.reloj();
    assert_eq!((r.mundo_ms, r.factor_milesimas, r.pausado), (5_000_000, 60_000, true), "el mundo estuvo congelado mientras estaba apagado");
    // Con foto, también.
    n.foto_ahora();
    n.esperar_fotos();
    drop(n);
    assert!(con_hash("mundo-b", 0).is_err(), "la foto es de otro mundo");
    assert!(con_hash("mundo-a", 0).is_ok());
}

#[test]
fn una_cola_del_registro_cortada_se_recupera_y_avisa() {
    let dir = tempfile::tempdir().unwrap();
    let esc = Escenario { w: 30, h: 30, n: 20, semilla: 5 };
    let hash = {
        let mut n = persistencia_en(dir.path(), 0);
        let ids = esc.crear(&mut n);
        esc.ordenes(&mut n, &ids, 0, 50, &|n, t| n.avanzar_hasta(t));
        n.avanzar_hasta(50 * 3_000);
        n.hash_estado()
    };
    // Un corte a medio escribir: bytes sueltos al final del último segmento.
    let seg = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|x| x == "log")).max().unwrap();
    use std::io::Write;
    std::fs::OpenOptions::new().append(true).open(seg).unwrap().write_all(&[9, 0, 0, 0, 1, 2]).unwrap();
    let n = persistencia_en(dir.path(), 0);
    assert!(!n.avisos_de_recuperacion().is_empty());
    assert_eq!(n.hash_estado(), hash, "lo escrito entero se recupera igual");
}
