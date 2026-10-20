//! La economía de las plazas dentro del núcleo (Economia_Construccion): cada plaza fundada vive sola, un minuto de mundo por evento,
//! se guarda en las fotos y reacciona a las órdenes. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las
//! pruebas que lo necesitan se omiten si no está.

use asentamiento::catalogo::Tipo;
use asentamiento::{Cargo, Recurso};
use base::{Instante, Punto, MS_POR_MINUTO};
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{rechazos, Ajustes, Arranque, Comando, Estado, Nucleo, Peticion, Reglas, Resultado};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;
const DIA: i64 = 24 * HORA;

fn mundo() -> Option<&'static (Arc<Mundo>, Arc<Rutas>)> {
    static M: OnceLock<Option<(Arc<Mundo>, Arc<Rutas>)>> = OnceLock::new();
    M.get_or_init(|| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generador/salida/mundo");
        let f = std::fs::read_dir(dir).ok()?.flatten().find(|e| e.file_name().to_string_lossy().starts_with("mundo-"))?;
        let m = Mundo::cargar(&std::fs::read(f.path()).ok()?, &OpcionesValidacion::default()).expect("artefacto válido");
        let r = Arc::new(Rutas::desde_mundo(&m));
        Some((Arc::new(m), r))
    })
    .as_ref()
}

macro_rules! con_mundo {
    () => {
        match mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

type M = (Arc<Mundo>, Arc<Rutas>);

/// Las reglas por defecto sin acoso ni anillo de bandidos: estos tests miden otra cosa y se calibraron sin ellos (los de `combate.rs` miden los bandidos).
fn sin_bandidos() -> Reglas {
    let mut r = Reglas::default();
    let b = &mut std::sync::Arc::make_mut(&mut r.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    r
}

fn arranque(m: &M, semilla: u64) -> Arranque {
    Arranque { reglas: sin_bandidos(), ajustes: Ajustes::default(), semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn nuevo(m: &M) -> Nucleo {
    Nucleo::nuevo(arranque(m, 7))
}

fn admin(id: &str, c: Comando) -> Peticion {
    Peticion::nueva("admin", id, c).de_admin()
}

fn ana(id: &str, c: Comando) -> Peticion {
    Peticion::nueva("ana", id, c)
}

fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

/// Primer punto fundable en anillos alrededor del campamento `i` (verde: con bosque y piedra en la zona inicial).
fn sitio(n: &Nucleo, m: &Mundo, i: usize, r: i64, grado0: i64, faccion: u32, exigir_verde: bool) -> Punto {
    let c = campamento(m, i);
    for r in (0..8).map(|k| r + k * 10_000) {
        for g in 0..360 {
            let a = ((grado0 + g) as f64).to_radians();
            let p = Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64);
            if let Some(v) = n.viabilidad(p, faccion) {
                if v.motivo.is_none() && (!exigir_verde || v.color == territorio::Viabilidad::Verde) {
                    return p;
                }
            }
        }
    }
    panic!("sin sitio fundable cerca del campamento {i}");
}

/// Un sitio verde con mucho bosque a 10 u (al menos 6 parcelas): una plaza que no cae en ruinas por falta de madera.
fn sitio_rico(n: &Nucleo, m: &Mundo, i: usize, faccion: u32) -> Punto {
    let c = campamento(m, i);
    for r in (0..8).map(|k| 50_000 + k * 10_000) {
        for g in 0..360 {
            let a = (g as f64).to_radians();
            let p = Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64);
            if n.viabilidad(p, faccion).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6) {
                return p;
            }
        }
    }
    panic!("sin sitio rico cerca del campamento {i}");
}

/// Una columna de `faccion` en `p` con caravana, que funda en `t`; devuelve el id de la plaza.
fn fundar_en(n: &mut Nucleo, t: Instante, nombre: &str, faccion: u32, p: Punto) -> u64 {
    let e = n.crear_ejercito(t, "ana", faccion, p, 500, 100_000).unwrap();
    assert!(n.aplicar_en(t, admin(&format!("c-{nombre}"), Comando::DarCaravanaFundacion { ejercito: e })).aceptada());
    let r = n.aplicar_en(t, ana(&format!("f-{nombre}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

fn codigo(r: &nucleo::Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

/// Eventos de crónica con ese código: `(instante, plazas, datos)`.
type Hecho = (Instante, Vec<u64>, Vec<(&'static str, i64)>);

fn eventos(n: &Nucleo, codigo: &str) -> Vec<Hecho> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).map(|e| (e.en, e.asentamientos.clone(), e.datos.clone())).collect()
}

fn tipo_idx(t: Tipo) -> i64 {
    t as u8 as i64
}

/// Un «jugador» que cada hora pide la subida de nivel de la plaza (si algo lo bloquea, la orden se rechaza y no deja rastro).
fn hora_de_jugador(n: &mut Nucleo, id: u64, hora: i64) {
    let t = hora * HORA;
    n.avanzar_hasta(t);
    n.aplicar_en(t, ana(&format!("asc{id}-{hora}"), Comando::SolicitarAscenso { asentamiento: id }));
}

// ───────────────────────────── 1. La plaza vive sola ─────────────────────────────

#[test]
fn una_plaza_vive_sola_construye_crece_y_sube_al_nivel_2() {
    let m = con_mundo!();
    let mut n = nuevo(m);
    let p = sitio(&n, &m.0, 0, 50_000, 0, 1, true);
    let fundada = 90_123; // una fase que no es múltiplo de minuto: los pasos caen en `fundado_en + k·60 s`
    let id = fundar_en(&mut n, fundada, "a", 1, p);
    for hora in 2..=12 * 24 {
        hora_de_jugador(&mut n, id, hora);
    }
    let eco = n.economia(id).expect("la plaza sigue viva");
    let completados = eventos(&n, "construccion.edificio_completado");
    let de_tipo = |t: Tipo| completados.iter().filter(|(_, _, d)| d.contains(&("tipo", tipo_idx(t)))).count();
    assert!(de_tipo(Tipo::Lenera) >= 1 && de_tipo(Tipo::Cantera) >= 1, "construye su Leñera y su Cantera sola: {completados:?}");
    assert!(eventos(&n, "asentamiento.nivel_subio").iter().any(|(_, _, d)| d.contains(&("nivel", 2))), "sube al nivel 2 pidiéndolo");
    assert_eq!((eco.nivel, eco.nivel_actual()), (2, 2));
    assert!(eco.poblacion.pesants > 20_000, "la población crece: {}", eco.poblacion.pesants);
    assert!(eco.almacen.cantidad(Recurso::Madera) >= 0);
    // Los pasos caen en `fundado_en + k·60 s`, mirando o no.
    for codigo in ["construccion.edificio_completado", "construccion.necesidad_detectada", "asentamiento.nivel_subio"] {
        for (en, ..) in eventos(&n, codigo) {
            assert_eq!((en - fundada) % MIN, 0, "{codigo} en {en}");
        }
    }
    // La zona creció 1 unidad por edificio completado, sin pasar del techo del nivel alcanzado; el tiempo solo no la agranda.
    let r = &n.estado().reglas.territorio;
    let esperado = (r.radio_inicial + completados.len() as i64 * r.crecimiento_por_edificio).min(r.techo(2));
    assert_eq!(n.radio_zona(id), Some(esperado));
    // Cada edificio ligado a un yacimiento tiene tomado su sitio: parcelas y nodos reclamados cuadran con las fuentes.
    use asentamiento::edificio::Fuente;
    let a = &n.estado().asentamientos[&id];
    let parcelas: std::collections::BTreeSet<u32> = eco.edificios.iter().filter_map(|e| if let Some(Fuente::Parcela(p)) = e.fuente { Some(p) } else { None }).collect();
    assert_eq!(a.parcelas, parcelas);
    assert!(n.estado().nodos_reclamados.values().all(|&dueno| dueno == id) && !n.estado().nodos_reclamados.is_empty());
    // El stock de la piedra extraída es estado del núcleo.
    assert!(!n.estado().yacimientos.is_empty());
}

#[test]
fn un_paso_sin_novedades_no_sube_la_version() {
    let m = con_mundo!();
    let mut n = nuevo(m);
    let p = sitio(&n, &m.0, 0, 50_000, 0, 1, true);
    let id = fundar_en(&mut n, 0, "a", 1, p);
    // Con la auto-construcción pausada y sin obras, los primeros minutos solo mueven ritmos (población, comida, almacén).
    assert!(n.aplicar_en(0, admin("pausa", Comando::AlternarAutoConstruccion { asentamiento: id, pausada: true })).aceptada());
    let (v, h) = (n.version(), n.hash_estado());
    n.avanzar_hasta(5 * MIN);
    assert_eq!(n.version(), v, "los ritmos no cierran versión");
    assert_ne!(n.hash_estado(), h, "pero el estado sí cambia (población, comida, mantenimiento)");
    // Las citas iniciales de bandidos (una por campamento de mercenarios y una por plaza) se agotan en los primeros minutos; después queda un paso por minuto.
    let antes = n.metricas().eventos;
    n.avanzar_hasta(10 * MIN);
    assert_eq!(n.metricas().eventos - antes, 5, "un paso por minuto");
}

// ───────────────────────────── 2. Determinismo ─────────────────────────────

/// Dos plazas, tres días, órdenes a horas fijas; `avanzar` decide cómo se trocea el avance entre órdenes.
fn correr(m: &M, semilla: u64, avanzar: &dyn Fn(&mut Nucleo, Instante)) -> Nucleo {
    let mut n = Nucleo::nuevo(arranque(m, semilla));
    let (pa, pb) = (sitio(&n, &m.0, 0, 50_000, 0, 1, true), sitio(&n, &m.0, 3, 50_000, 0, 2, true));
    let (a, b) = (fundar_en(&mut n, 0, "a", 1, pa), fundar_en(&mut n, 17 * MIN + 5, "b", 2, pb));
    for hora in 1..=3 * 24 {
        let t = hora * HORA;
        avanzar(&mut n, t);
        for (id, x) in [(a, "a"), (b, "b")] {
            n.aplicar_en(t, ana(&format!("asc-{x}-{hora}"), Comando::SolicitarAscenso { asentamiento: id }));
        }
        if hora == 5 {
            n.aplicar_en(t, ana("viv", Comando::AnadirEdificio { asentamiento: a, tipo: Tipo::Vivienda }));
            n.aplicar_en(t, ana("rac", Comando::ActivarPolitica { asentamiento: b, politica: "racionamiento".into(), cargo: Cargo::Sacerdote }));
        }
        if hora == 30 {
            n.aplicar_en(t, ana("res", Comando::CalibrarReserva { asentamiento: b, recurso: Recurso::Madera, valor: 20_000 }));
        }
    }
    n.avanzar_hasta(3 * DIA + 7 * MIN);
    n
}

#[test]
fn mismas_ordenes_dan_el_mismo_hash_y_da_igual_como_se_trocee_el_avance() {
    let m = con_mundo!();
    let normal = correr(m, 3, &|n, t| n.avanzar_hasta(t));
    assert_eq!(normal.hash_estado(), correr(m, 3, &|n, t| n.avanzar_hasta(t)).hash_estado(), "repetible");
    // Sin avances intermedios (solo el que hace cada orden al entrar): el modo acelerado de un golpe.
    assert_eq!(normal.hash_estado(), correr(m, 3, &|_, _| {}).hash_estado());
    // En pasos pequeños e irregulares (como el modo en vivo con pulsos).
    let a_trozos = correr(m, 3, &|n, t| {
        let ahora = n.ahora();
        for s in 1..=5 {
            n.avanzar_hasta(ahora + (t - ahora).max(0) * s * s / 25);
        }
    });
    assert_eq!(normal.hash_estado(), a_trozos.hash_estado());
    assert_ne!(normal.hash_estado(), correr(m, 4, &|n, t| n.avanzar_hasta(t)).hash_estado(), "otra semilla, otro mundo");
}

// ───────────────────────────── 3. Recuperación desde foto ─────────────────────────────

#[test]
fn una_simulacion_de_dias_con_fotos_y_recuperacion_a_mitad_da_el_mismo_hash() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9) };
    // Las mismas órdenes, con y sin interrupción.
    let tramo = |n: &mut Nucleo, ids: (u64, u64), desde: i64, hasta: i64| {
        for hora in desde..=hasta {
            let t = hora * HORA;
            n.avanzar_hasta(t);
            for (id, x) in [(ids.0, "a"), (ids.1, "b")] {
                n.aplicar_en(t, ana(&format!("asc-{x}-{hora}"), Comando::SolicitarAscenso { asentamiento: id }));
            }
            if hora % 17 == 0 {
                n.aplicar_en(t, ana(&format!("viv-{hora}"), Comando::AnadirEdificio { asentamiento: ids.0, tipo: Tipo::Vivienda }));
                n.aplicar_en(t, ana(&format!("mv-{hora}"), Comando::AlternarAutoConstruccion { asentamiento: ids.1, pausada: hora % 34 != 0 }));
            }
        }
    };
    let fundar_dos = |n: &mut Nucleo| {
        let (pa, pb) = (sitio(n, &m.0, 0, 50_000, 0, 1, true), sitio(n, &m.0, 3, 50_000, 0, 2, true));
        (fundar_en(n, 0, "a", 1, pa), fundar_en(n, 5 * MIN, "b", 2, pb))
    };

    // Sin interrumpir, en memoria.
    let sin_parar = {
        let mut n = Nucleo::nuevo(con(ajustes.clone()));
        let ids = fundar_dos(&mut n);
        tramo(&mut n, ids, 1, 4 * 24);
        tramo(&mut n, ids, 4 * 24 + 1, 8 * 24);
        n.hash_estado()
    };

    // Con foto a mitad y caída: se recupera de la foto + el registro y se sigue.
    let (ids, a_mitad) = {
        let mut n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        let ids = fundar_dos(&mut n);
        tramo(&mut n, ids, 1, 2 * 24);
        assert!(n.metricas().ordenes_aceptadas > 8, "las órdenes de economía entran al registro: {}", n.metricas().ordenes_aceptadas);
        n.foto_ahora(); // la foto lleva economías, yacimientos, reclamos y los pasos de minuto de la cola
        tramo(&mut n, ids, 2 * 24 + 1, 4 * 24);
        n.apagar().unwrap();
        (ids, n.hash_estado())
    };
    let mut n = Nucleo::con_persistencia(con(ajustes), dir.path(), Default::default()).unwrap();
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), a_mitad, "foto + registro reconstruyen el mismo mundo");
    tramo(&mut n, ids, 4 * 24 + 1, 8 * 24);
    assert_eq!(n.hash_estado(), sin_parar, "recuperar a mitad no cambia el mundo");
    n.apagar().unwrap();
}

// ───────────────────────────── 4. Las órdenes y sus rechazos ─────────────────────────────

#[test]
fn las_ordenes_de_economia_y_sus_rechazos() {
    let m = con_mundo!();
    let mut n = nuevo(m);
    let p = sitio(&n, &m.0, 0, 50_000, 0, 1, true);
    let id = fundar_en(&mut n, 0, "a", 1, p);
    let t = 0;
    let orden = |n: &mut Nucleo, i: &str, c: Comando| n.aplicar_en(t, ana(i, c));
    let madera0 = n.economia(id).unwrap().almacen.cantidad(Recurso::Madera);

    // Autorización y existencia.
    let casi = |actor: &str, i: &str, c: Comando| Peticion::nueva(actor, i, c);
    assert_eq!(codigo(&n.aplicar_en(t, casi("luis", "x1", Comando::AlternarAutoConstruccion { asentamiento: id, pausada: true }))), rechazos::NO_AUTORIZADO);
    assert_eq!(codigo(&orden(&mut n, "x2", Comando::SolicitarAscenso { asentamiento: 999 })), rechazos::ASENTAMIENTO_DESCONOCIDO);
    assert!(n.aplicar_en(t, casi("luis", "x3", Comando::AlternarAutoConstruccion { asentamiento: id, pausada: true }).de_admin()).aceptada(), "la administración manda en todas");
    assert_eq!(orden(&mut n, "x4", Comando::AlternarAutoConstruccion { asentamiento: id, pausada: true }).resultado, Resultado::SinCambios);
    assert!(orden(&mut n, "x5", Comando::AlternarAutoConstruccion { asentamiento: id, pausada: false }).aceptada());

    // Añadir, reordenar, quitar (con reembolso) y los errores de obra.
    let r1 = orden(&mut n, "v1", Comando::AnadirEdificio { asentamiento: id, tipo: Tipo::Vivienda });
    assert!(r1.aceptada(), "{r1:?}");
    let v1 = r1.creadas[0] as u32;
    let r2 = orden(&mut n, "v2", Comando::AnadirEdificio { asentamiento: id, tipo: Tipo::Vivienda });
    assert!(r2.aceptada(), "{r2:?}");
    let g = r2.creadas[0] as u32;
    assert!(n.economia(id).unwrap().almacen.cantidad(Recurso::Madera) < madera0, "se paga al encolar");
    assert_eq!(orden(&mut n, "m1", Comando::MoverEnCola { asentamiento: id, edificio: g, arriba: false }).resultado, Resultado::SinCambios, "dos manuales con la misma prioridad: intercambiarlas no cambia nada");
    assert_eq!(codigo(&orden(&mut n, "m3", Comando::MoverEnCola { asentamiento: id, edificio: 9_999, arriba: true })), rechazos::OBRA_NO_ESTA_EN_COLA);
    assert!(orden(&mut n, "q1", Comando::QuitarDeCola { asentamiento: id, edificio: v1 }).aceptada());
    assert!(n.economia(id).unwrap().edificio(v1).is_none());
    assert_eq!(codigo(&orden(&mut n, "q2", Comando::QuitarDeCola { asentamiento: id, edificio: v1 })), rechazos::EDIFICIO_DESCONOCIDO);
    assert_eq!(codigo(&orden(&mut n, "a1", Comando::AnadirEdificio { asentamiento: id, tipo: Tipo::CentroUrbano })), rechazos::OBRA_TIPO_NO_MANUAL);
    assert_eq!(codigo(&orden(&mut n, "a2", Comando::AnadirEdificio { asentamiento: id, tipo: Tipo::Palacio })), rechazos::OBRA_NIVEL_INSUFICIENTE);
    assert_eq!(codigo(&orden(&mut n, "a3", Comando::AnadirEdificio { asentamiento: id, tipo: Tipo::Maravilla })), rechazos::OBRA_ERA_INSUFICIENTE);
    assert_eq!(codigo(&orden(&mut n, "a4", Comando::AnadirEdificio { asentamiento: id, tipo: Tipo::MinaCobre })), rechazos::OBRA_SOLO_POR_PUEBLO, "las minas van por pueblo extractor");
    assert_eq!(codigo(&orden(&mut n, "u1", Comando::MejorarEdificio { asentamiento: id, edificio: 9_999 })), rechazos::EDIFICIO_DESCONOCIDO);
    assert_eq!(codigo(&orden(&mut n, "u2", Comando::MejorarEdificio { asentamiento: id, edificio: g })), rechazos::OBRA_NO_MEJORABLE, "aún no está activo");

    // Un Corral/Cantera toma un nodo; al quitarlo de la cola lo devuelve; la Leñera, una parcela.
    let r3 = orden(&mut n, "c1", Comando::AnadirEdificio { asentamiento: id, tipo: Tipo::Cantera });
    assert!(r3.aceptada(), "{r3:?}");
    assert_eq!(n.estado().nodos_reclamados.len(), 1);
    assert!(orden(&mut n, "c2", Comando::QuitarDeCola { asentamiento: id, edificio: r3.creadas[0] as u32 }).aceptada());
    assert!(n.estado().nodos_reclamados.is_empty(), "quitar de la cola libera el nodo");
    let r4 = orden(&mut n, "l1", Comando::AnadirEdificio { asentamiento: id, tipo: Tipo::Lenera });
    assert!(r4.aceptada(), "{r4:?}");
    assert_eq!(n.estado().asentamientos[&id].parcelas.len(), 1);
    assert!(orden(&mut n, "l2", Comando::QuitarDeCola { asentamiento: id, edificio: r4.creadas[0] as u32 }).aceptada());
    assert!(n.estado().asentamientos[&id].parcelas.is_empty() && n.estado().parcelas_ocupadas.is_empty(), "quitar de la cola libera la parcela");

    // Recetas y reserva.
    assert!(orden(&mut n, "r1", Comando::AlternarReceta { asentamiento: id, recurso: Recurso::LingoteBronce, pausada: true }).aceptada());
    assert_eq!(orden(&mut n, "r2", Comando::AlternarReceta { asentamiento: id, recurso: Recurso::LingoteBronce, pausada: true }).resultado, Resultado::SinCambios);
    assert!(orden(&mut n, "k1", Comando::CalibrarReserva { asentamiento: id, recurso: Recurso::Madera, valor: 5_000 }).aceptada());
    assert_eq!(n.economia(id).unwrap().reserva_manual[Recurso::Madera.indice()], 5_000);
    assert_eq!(orden(&mut n, "k2", Comando::CalibrarReserva { asentamiento: id, recurso: Recurso::Madera, valor: 5_000 }).resultado, Resultado::SinCambios);

    // Ascenso: todos los bloqueos a la vez, con el código del primero.
    let r = orden(&mut n, "s1", Comando::SolicitarAscenso { asentamiento: id });
    assert_eq!(codigo(&r), rechazos::ASCENSO_FALTA_POBLACION);
    let Resultado::Rechazada { mensaje, .. } = &r.resultado else { unreachable!() };
    assert!(mensaje.contains("faltan edificios") && mensaje.contains("faltan recursos"), "{mensaje}");

    // Políticas.
    let pol = |i: &str, pol: &str, cargo| ana(i, Comando::ActivarPolitica { asentamiento: id, politica: pol.into(), cargo });
    assert_eq!(codigo(&n.aplicar_en(t, pol("p1", "no_existe", Cargo::Gobernador))), rechazos::POLITICA_DESCONOCIDA);
    assert_eq!(codigo(&n.aplicar_en(t, pol("p2", "via_rapida", Cargo::Sacerdote))), rechazos::POLITICA_FUERA_DE_POOL);
    assert!(n.aplicar_en(t, pol("p3", "racionamiento", Cargo::Sacerdote)).aceptada());
    assert_eq!(codigo(&n.aplicar_en(t, pol("p4", "racionamiento", Cargo::Sacerdote))), rechazos::POLITICA_YA_ACTIVA);
    assert_eq!(codigo(&n.aplicar_en(t, pol("p5", "culto_fertilidad", Cargo::Sacerdote))), rechazos::POLITICA_SIN_SLOT);
    assert_eq!(n.economia(id).unwrap().politicas.activas().len(), 1);
    // Caduca sola a su hora (150 min de la duración por defecto), con precisión de un minuto.
    n.avanzar_hasta(151 * MIN);
    assert!(n.economia(id).unwrap().politicas.activas().is_empty());
    assert_eq!(eventos(&n, "politica.expirada").len(), 1);

    // Una orden rechazada o sin cambios no deja rastro; las aceptadas, sí (versión y plaza tocada).
    let v = n.version();
    let h = n.hash_estado();
    assert_eq!(codigo(&n.aplicar_en(151 * MIN, ana("z1", Comando::QuitarDeCola { asentamiento: id, edificio: 9_999 }))), rechazos::EDIFICIO_DESCONOCIDO);
    assert_eq!((n.version(), n.hash_estado()), (v, h));
}

#[test]
fn la_cola_se_reordena_intercambiando_prioridades() {
    let m = con_mundo!();
    let mut n = nuevo(m);
    let p = sitio(&n, &m.0, 0, 50_000, 0, 1, true);
    let id = fundar_en(&mut n, 0, "a", 1, p);
    n.avanzar_hasta(MIN); // el primer paso compromete la Leñera y la Cantera (auto-construcción) y quedan en cola
    let cola = |n: &Nucleo| {
        let mut v: Vec<(i64, u32)> = n.economia(id).unwrap().edificios.iter().filter(|e| e.estado == asentamiento::edificio::Estado::EnCola).map(|e| (e.prioridad, e.id)).collect();
        v.sort_by_key(|&(p, _)| std::cmp::Reverse(p));
        v
    };
    let antes = cola(&n);
    assert!(antes.len() >= 2 && antes[0].0 != antes[1].0, "{antes:?}");
    let v = n.version();
    let r = n.aplicar_en(MIN, ana("sube", Comando::MoverEnCola { asentamiento: id, edificio: antes[1].1, arriba: true }));
    assert!(r.aceptada() && r.version_tras > v, "{r:?}");
    let despues = cola(&n);
    assert_eq!((despues[0].1, despues[1].1), (antes[1].1, antes[0].1), "el segundo pasó al primer puesto");
    assert_eq!(n.estado().asentamientos[&id].version, r.version_tras, "la plaza queda tocada en esa versión");
    assert_eq!(n.aplicar_en(MIN, ana("sube2", Comando::MoverEnCola { asentamiento: id, edificio: antes[1].1, arriba: true })).resultado, Resultado::SinCambios, "ya es el primero");
}

// ───────────────────────────── 5 y 6. Ruinas ─────────────────────────────

#[test]
fn una_plaza_que_se_arruina_libera_sus_parcelas_y_nodos_y_su_evento_de_minuto_se_invalida() {
    let m = con_mundo!();
    let mut n = nuevo(m);
    let p = sitio(&n, &m.0, 0, 50_000, 0, 1, true);
    let id = fundar_en(&mut n, 0, "a", 1, p);
    let pb = sitio_rico(&n, &m.0, 3, 2);
    let otra = fundar_en(&mut n, 0, "b", 2, pb);
    // Sin Leñera no hay madera para el mantenimiento: la auto-construcción pausada, una parcela por la vía de administración y una
    // Cantera a mano (toma un nodo). Los dos yacimientos quedan reclamados por la plaza condenada.
    assert!(n.aplicar_en(0, admin("pausa", Comando::AlternarAutoConstruccion { asentamiento: id, pausada: true })).aceptada());
    let parcela = n.aplicar_en(0, admin("parcela", Comando::ReclamarParcela { asentamiento: id }));
    assert!(parcela.aceptada(), "{parcela:?}");
    assert!(n.aplicar_en(0, ana("cantera", Comando::AnadirEdificio { asentamiento: id, tipo: Tipo::Cantera })).aceptada());
    assert_eq!((n.estado().parcelas_ocupadas.len(), n.estado().nodos_reclamados.len()), (1, 1));
    let gen_antes = n.estado().generaciones.actual(id);
    let descartados_antes = n.eventos_descartados();

    let mut dia = 0;
    while n.economia(id).is_some() {
        dia += 1;
        assert!(dia <= 60, "debería haber caído en ruinas");
        n.avanzar_hasta(dia * DIA);
    }
    let ruinas = eventos(&n, "asentamiento.ruinas");
    assert_eq!(ruinas.len(), 1);
    assert_eq!(ruinas[0].1, vec![id]);
    assert!(ruinas[0].2[0].1 != 0, "lleva los recursos que no pudo pagar");

    // La plaza desapareció y liberó todo lo suyo.
    assert!(!n.estado().asentamientos.contains_key(&id));
    assert!(n.estado().parcelas_ocupadas.values().all(|&d| d != id));
    assert!(n.estado().nodos_reclamados.values().all(|&d| d != id));
    // El paso de minuto ya programado se invalidó (generación nueva) y sale descartado de la cola; la otra plaza sigue viva.
    assert!(n.estado().generaciones.actual(id) > gen_antes);
    n.avanzar_hasta(n.ahora() + 2 * MIN);
    assert_eq!(n.eventos_descartados(), descartados_antes + 1, "el Minuto siguiente de la plaza arruinada se descartó");
    let eventos_antes = n.metricas().eventos;
    n.avanzar_hasta(n.ahora() + 10 * MIN);
    assert_eq!(n.metricas().eventos - eventos_antes, 10, "solo queda el reloj de la otra plaza: un paso por minuto");
    assert!(n.economia(otra).is_some(), "la otra plaza sigue viva");
    // Y el mundo arruinado sobrevive a serializar y recuperar.
    let bytes = n.estado().serializar();
    let e2 = Estado::deserializar(&bytes, &Reglas::default(), m.0.hash()).unwrap();
    assert_eq!(e2.hash(), n.hash_estado());
}

// ───────────────────────────── La economía en el estado (sin mundo) ─────────────────────────────

fn estado_con_una_plaza() -> Estado {
    let mut s = Estado::nuevo(Reglas::default(), "h".into(), 1, 0);
    let eco = &s.reglas.economia.datos;
    let economia = asentamiento::asentamiento::Asentamiento::fundar(eco, 0, &eco.materiales_iniciales, eco.viviendas_iniciales);
    let urbe = nucleo::urbanismo::Urbe::fundar(&s.reglas.economia, 7, 1);
    s.asentamientos.insert(1, nucleo::Asentamiento { id: 1, faccion: 1, posicion: Punto::new(1000, 2000), fundado_en: 0, radio: 6_000, parcelas: Default::default(), version: 0, economia, urbe, protegida_hasta: None, ocupada_hasta: None });
    s
}

#[test]
fn la_economia_se_serializa_de_ida_y_vuelta_y_cualquier_campo_cambia_el_hash() {
    use asentamiento::asentamiento::Ascenso;
    use asentamiento::edificio::{Edificio, Estado as EstadoEdificio, Fuente, Mejora};
    use asentamiento::politicas::Activa;
    let reglas = Reglas::default();
    let base = estado_con_una_plaza();
    let bytes = base.serializar();
    let vuelta = Estado::deserializar(&bytes, &reglas, "h").unwrap();
    assert_eq!(vuelta.serializar(), bytes, "serializar → deserializar → serializar es idéntico");
    assert_eq!(vuelta.asentamientos, base.asentamientos);
    // Una economía rica de verdad (todos los campos en uso) también da la vuelta.
    let mut rica = Estado::deserializar(&bytes, &reglas, "h").unwrap();
    {
        let a = rica.asentamientos.get_mut(&1).unwrap();
        a.parcelas.insert(4);
        let e = &mut a.economia;
        let mut ed = Edificio::nuevo(40, Tipo::Cantera, EstadoEdificio::EnConstruccion);
        ed.completa_en = Some(123_456);
        ed.fuente = Some(Fuente::Nodo(8));
        ed.mejora = Some(Mejora { nivel_objetivo: 2, completa_en: 99 });
        e.edificios.push(ed);
        e.politicas = asentamiento::Politicas::desde_activas(vec![Activa { id: "racionamiento".into(), cargo: Cargo::Sacerdote, expira_en: 777 }]);
        e.ascenso = Some(Ascenso { nivel_objetivo: 2, iniciado_en: 5, completa_en: 6 });
        e.recetas_pausadas = vec![Recurso::Cuero, Recurso::ArmaBronce];
        e.extractor_minutos_sin_cupo = vec![(Tipo::Corral, 3)];
        e.reserva_manual[3] = 42;
    }
    rica.yacimientos.insert(3, nucleo::economia::StockNodo { stock: 5, vacio_desde: Some(77) });
    let rb = rica.serializar();
    let rica2 = Estado::deserializar(&rb, &reglas, "h").unwrap();
    assert_eq!(rica2.serializar(), rb);
    assert_eq!(rica2.asentamientos, rica.asentamientos);
    assert_eq!(rica2.nodos_reclamados.get(&8), Some(&1), "los reclamos de nodos se reconstruyen de las fuentes de los edificios");
    assert_eq!(rica2.yacimientos, rica.yacimientos);

    // Cada campo de la economía (y del stock) cambia el hash.
    type Cambio = (&'static str, Box<dyn Fn(&mut Estado)>);
    let eco = |f: fn(&mut asentamiento::asentamiento::Asentamiento)| -> Box<dyn Fn(&mut Estado)> { Box::new(move |s| f(&mut s.asentamientos.get_mut(&1).unwrap().economia)) };
    let cambios: Vec<Cambio> = vec![
        ("fundado_en", eco(|e| e.fundado_en += 1)),
        ("nivel", eco(|e| e.nivel += 1)),
        ("medidor.valor", eco(|e| e.medidor.valor -= 1)),
        ("medidor.racha", eco(|e| e.medidor.racha += 1)),
        ("medidor.nivel_actual", eco(|e| e.medidor.nivel_actual += 1)),
        ("poblacion.pesants", eco(|e| e.poblacion.pesants += 1)),
        ("poblacion.artesanos", eco(|e| e.poblacion.artesanos += 1)),
        ("poblacion.nobleza", eco(|e| e.poblacion.nobleza += 1)),
        ("poblacion.nutricion", eco(|e| e.poblacion.nutricion -= 1)),
        ("almacen.cantidad", eco(|e| e.almacen.poner(Recurso::Hierro, 1))),
        ("almacen.capacidad", eco(|e| e.almacen.ampliar(Recurso::Hierro, 1))),
        ("politicas", eco(|e| e.politicas = asentamiento::Politicas::desde_activas(vec![Activa { id: "x".into(), cargo: Cargo::General, expira_en: 1 }]))),
        ("edificio.estado", eco(|e| e.edificios[1].estado = EstadoEdificio::EnCola)),
        ("edificio.completa_en", eco(|e| e.edificios[1].completa_en = Some(5))),
        ("edificio.prioridad", eco(|e| e.edificios[1].prioridad += 1)),
        ("edificio.nivel_interno", eco(|e| e.edificios[1].nivel_interno += 1)),
        ("edificio.mejora", eco(|e| e.edificios[1].mejora = Some(Mejora { nivel_objetivo: 2, completa_en: 1 }))),
        ("edificio.danado", eco(|e| e.edificios[1].danado = true)),
        ("edificio.fuente", eco(|e| e.edificios[1].fuente = Some(Fuente::Parcela(2)))),
        ("edificio.pausado_por_almacen", eco(|e| e.edificios[1].pausado_por_almacen = true)),
        ("edificio.tipo", eco(|e| e.edificios[1].tipo = Tipo::Almacen)),
        ("edificio.nuevo", eco(|e| e.edificios.push(Edificio::nuevo(77, Tipo::Granja, EstadoEdificio::EnCola)))),
        ("siguiente_edificio", eco(|e| e.siguiente_edificio += 1)),
        ("ascenso", eco(|e| e.ascenso = Some(Ascenso { nivel_objetivo: 2, iniciado_en: 1, completa_en: 2 }))),
        ("recetas_pausadas", eco(|e| e.recetas_pausadas.push(Recurso::Cuero))),
        ("auto_pausada", eco(|e| e.auto_pausada = true)),
        ("reserva_manual", eco(|e| e.reserva_manual[7] = 1)),
        ("ocupado", eco(|e| e.ocupado = true)),
        ("extractor_minutos_sin_cupo", eco(|e| e.extractor_minutos_sin_cupo.push((Tipo::Cantera, 1)))),
        ("completados_pendientes", eco(|e| e.completados_pendientes += 1)),
        ("destruido", eco(|e| e.destruido = true)),
        ("ultimo_minuto", eco(|e| e.ultimo_minuto += 1)),
        ("yacimientos", Box::new(|s| {
            s.yacimientos.insert(0, nucleo::economia::StockNodo { stock: 1, vacio_desde: None });
        })),
    ];
    let h0 = base.hash();
    let mut vistos = std::collections::HashSet::from([h0]);
    for (nombre, f) in cambios {
        let mut s = Estado::deserializar(&bytes, &reglas, "h").unwrap();
        f(&mut s);
        assert!(vistos.insert(s.hash()), "el hash no cambió (o coincidió con otro cambio) al tocar {nombre}");
    }
}

#[test]
fn las_fotos_se_niegan_con_otras_reglas_de_economia() {
    let base = estado_con_una_plaza();
    let bytes = base.serializar();
    let mut otras = Reglas::default();
    // Las reglas se identifican por el documento del que salen: una huella distinta se rechaza.
    let mut sucias = bytes.clone();
    let huella = otras.economia.huella().as_bytes().to_vec();
    let pos = sucias.windows(huella.len()).position(|w| w == &huella[..]).expect("la foto lleva la huella");
    sucias[pos] = if sucias[pos] == b'0' { b'1' } else { b'0' };
    assert!(Estado::deserializar(&sucias, &otras, "h").is_err());
    otras.viveres_bajos_pct += 1;
    assert!(Estado::deserializar(&bytes, &otras, "h").is_err());
}

// ───────────────────────────── Rendimiento ─────────────────────────────

/// Funda hasta `plazas` plazas (una Facción cada una) en una rejilla de 31 u; devuelve cuántas se fundaron.
fn poblar(n: &mut Nucleo, m: &Mundo, plazas: usize) -> usize {
    let (ancho, alto) = m.tamano();
    let paso = 31_000;
    let mut fundadas = 0;
    'fuera: for y in (paso..alto).step_by(paso as usize) {
        for x in (paso..ancho).step_by(paso as usize) {
            let faccion = fundadas as u32 + 1;
            let e = n.crear_ejercito(0, "ana", faccion, Punto::new(x, y), 500, 100_000).unwrap();
            n.aplicar_en(0, admin(&format!("c{x}-{y}"), Comando::DarCaravanaFundacion { ejercito: e }));
            if n.aplicar_en(0, ana(&format!("f{x}-{y}"), Comando::Fundar { ejercito: e })).aceptada() {
                fundadas += 1;
                if fundadas == plazas {
                    break 'fuera;
                }
            }
        }
    }
    fundadas
}

/// Mide el coste de un paso de minuto con muchas plazas. En `cargo test` corre pequeño; el de 1.000 plazas × 3 días es
/// `cargo test --release -p nucleo --test asentamientos rendimiento -- --ignored --nocapture`.
fn medir(plazas: usize, dias: i64) {
    let m = con_mundo!();
    let mut n = nuevo(m);
    let fundadas = poblar(&mut n, &m.0, plazas);
    assert!(fundadas >= plazas.min(30), "se fundaron {fundadas}");
    let (e0, t0) = (n.metricas().eventos, std::time::Instant::now());
    n.avanzar_hasta(dias * DIA);
    let (pasos, pared) = (n.metricas().eventos - e0, t0.elapsed());
    let vivas = n.estado().asentamientos.len();
    println!("{fundadas} plazas, {dias} días: {pasos} pasos en {:.2} s = {:.1} µs por paso; {vivas} siguen vivas; versión {}", pared.as_secs_f64(), pared.as_secs_f64() * 1e6 / pasos as f64, n.version());
    assert!(pasos >= (vivas as u64) * (dias as u64 * 1440).saturating_sub(1) / 2);
}

#[test]
fn rendimiento_de_muchas_plazas_pequeno() {
    medir(30, 1);
}

#[test]
#[ignore = "mide 1.000 plazas × 3 días; en release"]
fn rendimiento_de_mil_plazas_tres_dias() {
    medir(1_000, 3);
}

