//! Encuentros, persecución e inspección en el núcleo (fase 4C; Movimiento_Ejercitos §5, §7 y §12, Combate §2). La mayoría de las pruebas corren sobre la
//! retícula sintética de `comun` (columnas de administración, sin mundo); las de plazas, sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`) y se omiten si no está.

mod comun;

use asentamiento::Recurso;
use base::{Instante, Punto, Rng, MS_POR_MINUTO};
use comun::{c, marchar, nucleo};
use militar::inspeccion::{Contenido, Objetivo};
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{rechazos, Ajustes, Arranque, Comando, Nucleo, Peticion, Relacion, Reglas, Resultado, Respuesta};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

const MIN: Instante = MS_POR_MINUTO;

type Hecho = (Instante, Vec<u64>, Vec<u64>, Vec<(&'static str, i64)>);

fn hechos(n: &Nucleo, codigo: &str) -> Vec<Hecho> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).map(|e| (e.en, e.ejercitos.clone(), e.asentamientos.clone(), e.datos.clone())).collect()
}

fn dato(h: &Hecho, clave: &str) -> i64 {
    h.3.iter().find(|(k, _)| *k == clave).unwrap_or_else(|| panic!("el hecho no lleva «{clave}»: {h:?}")).1
}

fn codigo(r: &Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

fn admin(id: &str, comando: Comando) -> Peticion {
    Peticion::nueva("admin", id, comando).de_admin()
}

fn heroe(n: &Nucleo, columna: u64) -> u64 {
    n.ejercito(columna).unwrap().columna.lider
}

fn perseguir(n: &mut Nucleo, t: Instante, actor: &str, id: &str, columna: u64, objetivo: Objetivo) -> Respuesta {
    let h = heroe(n, columna);
    n.aplicar_en(t, Peticion::nueva(actor, id, Comando::Perseguir { heroe: h, objetivo }))
}

fn dejar(n: &mut Nucleo, t: Instante, actor: &str, id: &str, columna: u64) -> Respuesta {
    let h = heroe(n, columna);
    n.aplicar_en(t, Peticion::nueva(actor, id, Comando::DejarDePerseguir { heroe: h }))
}

fn inspeccionar(n: &mut Nucleo, t: Instante, actor: &str, id: &str, columna: u64, objetivo: Objetivo) -> Respuesta {
    let h = heroe(n, columna);
    n.aplicar_en(t, Peticion::nueva(actor, id, Comando::Inspeccionar { heroe: h, objetivo }))
}

fn distancia_en(n: &Nucleo, a: u64, b: u64, t: Instante) -> i64 {
    n.ejercito(a).unwrap().posicion_en(t).dist(n.ejercito(b).unwrap().posicion_en(t))
}

// ───────────────────────────── 1. Encuentros entre columnas ─────────────────────────────

/// Dos columnas de Facciones distintas frente a frente en la fila 2 de una retícula de 40 × 5, a 500 milésimas por segundo (30 unidades por minuto).
fn frente_a_frente() -> (Nucleo, u64, u64) {
    let mut n = nucleo(40, 5);
    let a = n.crear_ejercito(0, "ana", 1, c(0, 2), 500, 350_000).unwrap();
    let b = n.crear_ejercito(0, "beto", 2, c(39, 2), 500, 350_000).unwrap();
    (n, a, b)
}

#[test]
fn la_misma_faccion_y_los_aliados_no_se_cruzan_y_la_alianza_se_mira_al_vencer() {
    let correr = |faccion_b: u32, alianza_antes: bool, alianza_a_mitad: bool| {
        let mut n = nucleo(40, 5);
        let a = n.crear_ejercito(0, "ana", 1, c(0, 2), 500, 350_000).unwrap();
        let b = n.crear_ejercito(0, "beto", faccion_b, c(39, 2), 500, 350_000).unwrap();
        if alianza_antes {
            assert!(n.aplicar_en(0, admin("r1", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Alianza) })).aceptada());
        }
        n.aplicar_en(0, marchar("ana", "m1", a, c(39, 2)));
        n.aplicar_en(0, marchar("beto", "m2", b, c(0, 2)));
        if alianza_a_mitad {
            // Se firma después de programar el cruce y antes de que venza: el filtro se aplica con el estado de ese instante.
            assert!(n.aplicar_en(100_000, admin("r2", Comando::FijarRelacion { a: 2, b: 1, relacion: Some(Relacion::Alianza) })).aceptada());
        }
        n.avanzar_hasta(30 * MIN);
        hechos(&n, "ejercito.encuentro").len()
    };
    assert_eq!(correr(2, false, false), 1, "enemigas");
    assert_eq!(correr(1, false, false), 0, "la misma Facción");
    assert_eq!(correr(2, true, false), 0, "aliadas");
    assert_eq!(correr(2, false, true), 0, "aliadas antes de cruzarse");
}

#[test]
fn una_columna_acampada_tambien_se_encuentra_y_una_sola_vez() {
    let (mut n, a, _) = frente_a_frente();
    let quieta = n.crear_ejercito(0, "gus", 3, c(20, 2), 500, 350_000).unwrap();
    assert!(n.aplicar_en(0, marchar("ana", "m1", a, c(39, 2))).aceptada());
    n.avanzar_hasta(30 * MIN);
    let e: Vec<_> = hechos(&n, "ejercito.encuentro").into_iter().filter(|h| h.1.contains(&quieta) && h.1.contains(&a)).collect();
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].0, (c(20, 2).x - c(0, 2).x - 3_000) * 2, "instante exacto: 2 ms por milésima a 500 mil/s");
    // Un cambio de ritmo de la quieta (le llega carga) ni repite el encuentro ni lo pierde.
    let antes = hechos(&n, "ejercito.encuentro").len();
    assert!(n.aplicar_en(n.ahora(), admin("pv", Comando::DarSuministro { ejercito: quieta, recurso: Recurso::Trigo, cantidad: 1_000 })).aceptada());
    n.avanzar_hasta(n.ahora() + 10 * MIN);
    assert_eq!(hechos(&n, "ejercito.encuentro").len(), antes);
}

#[test]
fn solo_la_alianza_es_de_los_tuyos_y_la_relacion_la_fija_la_administracion() {
    let mut n = nucleo(40, 5);
    assert_eq!(n.relacion(1, 2), None);
    assert!(n.aplicar_en(0, admin("r1", Comando::FijarRelacion { a: 2, b: 1, relacion: Some(Relacion::Vasallaje) })).aceptada());
    assert_eq!((n.relacion(1, 2), n.relacion(2, 1)), (Some(Relacion::Vasallaje), Some(Relacion::Vasallaje)));
    assert!(!n.estado().aliadas(Some(1), Some(2)), "el vasallaje no es una alianza");
    assert!(n.aplicar_en(1, admin("r2", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Alianza) })).aceptada());
    assert!(n.estado().aliadas(Some(1), Some(2)) && !n.estado().aliadas(None, Some(2)));
    // Mismo valor: sin cambios; se quita; consigo misma o sin ser administración, no.
    assert_eq!(n.aplicar_en(2, admin("r3", Comando::FijarRelacion { a: 2, b: 1, relacion: Some(Relacion::Alianza) })).resultado, Resultado::SinCambios);
    assert!(n.aplicar_en(3, admin("r4", Comando::FijarRelacion { a: 1, b: 2, relacion: None })).aceptada());
    assert_eq!(n.relacion(1, 2), None);
    assert_eq!(codigo(&n.aplicar_en(4, admin("r5", Comando::FijarRelacion { a: 1, b: 1, relacion: Some(Relacion::Guerra) }))), rechazos::RELACION_CONSIGO_MISMA);
    assert_eq!(codigo(&n.aplicar_en(4, Peticion::nueva("ana", "r6", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) }))), rechazos::NO_AUTORIZADO);
    // Desde 5D la administración crea o rompe relaciones reales (decisión 9): la de r1 tiene a F2 de señora; r2 rompe el vasallaje y crea la alianza; r4 la rompe.
    assert_eq!((hechos(&n, "diplomacia.relacion_aceptada").len(), hechos(&n, "diplomacia.relacion_rota").len()), (2, 2));
    let r1 = n.relacion_politica(n.estado().politica.relaciones.iter().next().unwrap().id).unwrap();
    assert_eq!((r1.a, r1.b, r1.activa()), (2, 1, false), "a es la señora; la rota se conserva");
}

#[test]
fn quien_pide_salir_del_mundo_perseguido_o_en_batalla_no_sale_hasta_que_toca() {
    let fuera = |n: &Nucleo, h: u64| n.heroe(h).unwrap().heroe.presencia.fuera;
    // Sin nadie detrás, sale a los 2 min 30 s.
    let (mut n, _, q) = cazador_y_presa();
    let hq = heroe(&n, q);
    assert!(n.aplicar_en(0, Peticion::nueva("beto", "d", Comando::Desconectarse { heroe: hq })).aceptada());
    n.avanzar_hasta(150_000);
    assert!(fuera(&n, hq));

    // Perseguido (a la misma velocidad: ni lo alcanza ni lo pierde de vista) no sale hasta pedido + 5 min: quien huye desconectándose no gana nada.
    let mut n = nucleo(40, 5);
    let p = n.crear_ejercito(0, "ana", 1, c(0, 2), 500, 3_500_000).unwrap();
    let q = n.crear_ejercito(0, "beto", 2, c(1, 2), 500, 3_500_000).unwrap();
    let hq = heroe(&n, q);
    n.aplicar_en(0, marchar("beto", "m", q, c(39, 2)));
    assert!(perseguir(&mut n, 0, "ana", "p", p, Objetivo::Ejercito(q)).aceptada());
    assert!(n.aplicar_en(0, Peticion::nueva("beto", "d", Comando::Desconectarse { heroe: hq })).aceptada());
    n.avanzar_hasta(299_999);
    assert!(!fuera(&n, hq) && n.persiguiendo(p).is_some(), "sigue perseguido y sigue dentro");
    n.avanzar_hasta(300_000);
    assert!(fuera(&n, hq));
    assert_eq!(hechos(&n, "columna.presa_a_cubierto").len(), 1, "su columna se disuelve y la persecución termina");

    // En batalla espera al fin de la batalla, y entonces sale.
    let (mut n, _, q) = cazador_y_presa();
    let hq = heroe(&n, q);
    assert!(n.aplicar_en(0, Peticion::nueva("beto", "d", Comando::Desconectarse { heroe: hq })).aceptada());
    assert!(n.columna_entra_en_batalla(100_000, q));
    n.avanzar_hasta(10 * MIN);
    assert!(!fuera(&n, hq), "combatiendo no sale");
    assert!(n.columna_sale_de_batalla(10 * MIN, q));
    n.avanzar_hasta(10 * MIN);
    assert!(fuera(&n, hq));
}
#[test]
fn un_herido_y_una_columna_en_batalla_no_tienen_encuentros() {
    let correr = |herir: bool, batalla: bool| {
        let (mut n, a, b) = frente_a_frente();
        n.aplicar_en(0, marchar("ana", "m1", a, c(39, 2)));
        n.aplicar_en(0, marchar("beto", "m2", b, c(0, 2)));
        if herir {
            // Herido a los 250 s, vale 2 minutos: sigue herido en el cruce (309 s).
            let hb = heroe(&n, b);
            assert!(n.aplicar_en(250_000, admin("h", Comando::HerirHeroe { heroe: hb })).aceptada());
        }
        if batalla {
            n.avanzar_hasta(250_000);
            assert!(n.columna_entra_en_batalla(250_000, b));
        }
        n.avanzar_hasta(30 * MIN);
        hechos(&n, "ejercito.encuentro").len()
    };
    assert_eq!(correr(false, false), 1);
    assert_eq!(correr(true, false), 0, "un herido no se cruza");
    assert_eq!(correr(false, true), 0, "una columna apartada no se cruza");
}

#[test]
fn una_columna_en_batalla_sale_del_espacio_no_come_no_admite_ordenes_de_movimiento_y_retoma_su_plan() {
    let (mut n, a, b) = frente_a_frente();
    assert_eq!(n.cosas_en_el_espacio(), 2);
    n.aplicar_en(0, marchar("ana", "m1", a, c(39, 2)));
    n.avanzar_hasta(60_000);
    let donde = n.ejercito(a).unwrap().posicion_en(60_000);
    let (llegada, viveres) = (n.ejercito(a).unwrap().plan.fin().unwrap(), n.ejercito(a).unwrap().columna.viveres.valor_en(60_000));
    assert!(n.columna_entra_en_batalla(60_000, a));
    assert!(!n.columna_entra_en_batalla(60_000, a), "ya estaba apartada");
    assert!(!n.columna_entra_en_batalla(60_000, 9_999));
    assert_eq!((n.cosas_en_el_espacio(), n.en_el_mapa(a), n.en_el_mapa(b)), (1, false, true));
    let e = n.ejercito(a).unwrap();
    assert!(e.en_batalla && !e.marchando() && e.posicion == donde, "se queda donde está");
    // No se la toca con órdenes de movimiento, de persecución ni de inspección.
    let (v, hash) = (n.version(), n.hash_estado());
    assert_eq!(codigo(&n.aplicar_en(60_000, marchar("ana", "m2", a, c(0, 0)))), rechazos::COLUMNA_EN_BATALLA);
    assert_eq!(codigo(&n.aplicar_en(60_000, Peticion::nueva("ana", "e", Comando::Estacionar { ejercito: a }))), rechazos::COLUMNA_EN_BATALLA);
    assert_eq!(codigo(&perseguir(&mut n, 60_000, "ana", "p", a, Objetivo::Ejercito(b))), rechazos::COLUMNA_EN_BATALLA);
    assert_eq!(codigo(&inspeccionar(&mut n, 60_000, "ana", "i", a, Objetivo::Ejercito(b))), rechazos::COLUMNA_EN_BATALLA);
    assert_eq!((n.version(), n.hash_estado()), (v, hash), "los rechazos no dejan rastro");
    // Apartada no come: los víveres no bajan mientras dura.
    assert_eq!(n.ejercito(a).unwrap().columna.viveres.valor_en(90_000), viveres);
    // Al acabar vuelve al espacio y retoma el plan que llevaba, desplazado lo que duró la batalla (30 s).
    assert!(n.columna_sale_de_batalla(90_000, a));
    assert!(!n.columna_sale_de_batalla(90_000, a));
    assert_eq!((n.cosas_en_el_espacio(), n.en_el_mapa(a)), (2, true));
    let e = n.ejercito(a).unwrap();
    assert!(e.marchando() && !e.en_batalla && e.plan_en_pausa.is_none());
    assert_eq!(e.plan.fin(), Some(llegada + 30_000), "llega 30 s más tarde");
    assert_eq!(e.posicion_en(90_000), donde, "retoma desde donde se quedó");
    assert!(e.columna.viveres.valor_en(100_000) < viveres, "y vuelve a comer");
    assert_eq!(hechos(&n, "ejercito.en_batalla").len() + hechos(&n, "ejercito.sale_de_batalla").len(), 2);
    assert!(n.aplicar_en(91_000, marchar("ana", "m3", a, c(39, 2))).aceptada());
}

// ───────────────────────────── 2. Persecución ─────────────────────────────

/// `p` (ana, Facción 1, 1 unidad por segundo) y `q` (beto, Facción 2, a 8 unidades, 0,5 unidades por segundo) en la fila 2.
fn cazador_y_presa() -> (Nucleo, u64, u64) {
    let mut n = nucleo(40, 5);
    let p = n.crear_ejercito(0, "ana", 1, c(0, 2), 1_000, 350_000).unwrap();
    let q = n.crear_ejercito(0, "beto", 2, c(1, 2), 500, 350_000).unwrap();
    (n, p, q)
}

#[test]
fn perseguir_empieza_con_la_presa_a_la_vista_la_alcanza_en_un_instante_exacto_y_no_hay_combate() {
    let (mut n, p, q) = cazador_y_presa();
    assert!(n.aplicar_en(0, marchar("beto", "m", q, c(39, 2))).aceptada());
    let r = perseguir(&mut n, 0, "ana", "p1", p, Objetivo::Ejercito(q));
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(n.persiguiendo(p), Some(Objetivo::Ejercito(q)));
    assert_eq!(n.perseguidores_de(Objetivo::Ejercito(q)), vec![p]);
    assert!(n.ejercito(p).unwrap().marchando(), "sale en marcha hacia la intercepción");
    // El contrato dice adónde va: el punto de intercepción, el final de su plan (paridad P7).
    let x = nucleo::proyeccion::propios::ejercito(&n, n.ejercito(p).unwrap()).persiguiendo.expect("persigue");
    let fin = n.ejercito(p).unwrap().plan.vertices.last().unwrap().p;
    assert_eq!((x.presa_id, x.presa_es_caravana, x.destino.map(|d| (d.x, d.y))), (format!("ej-{q}"), false, Some((fin.x, fin.y))));
    let iniciada = hechos(&n, "columna.persecucion_iniciada");
    assert_eq!((iniciada.len(), iniciada[0].1.clone(), dato(&iniciada[0], "presa")), (1, vec![p], q as i64));
    assert!(iniciada[0].3.iter().all(|(k, _)| *k != "caravana"), "la presa es una columna");
    // Misma orden otra vez: sin cambios.
    assert_eq!(perseguir(&mut n, 0, "ana", "p2", p, Objetivo::Ejercito(q)).resultado, Resultado::SinCambios);

    // Ella va a 1 u/s y él a 0,5: la ventaja de 8 unidades baja a 3 en 10 s, y justo antes todavía la persigue.
    n.avanzar_hasta(9_999);
    assert_eq!(n.persiguiendo(p), Some(Objetivo::Ejercito(q)));
    n.avanzar_hasta(10_000);
    let alcance = hechos(&n, "columna.presa_alcanzada");
    assert_eq!(alcance.len(), 1, "{alcance:?}");
    let t = alcance[0].0;
    assert_eq!((t, alcance[0].1.clone()), (10_000, vec![p, q]), "lo oyen las dos columnas");
    assert!(distancia_en(&n, p, q, t) <= 3_000 + 1 && distancia_en(&n, p, q, t - 1) > 3_000 - 2, "a ≤ radioEncuentro exacto: {}", distancia_en(&n, p, q, t));
    assert_eq!(n.persiguiendo(p), None, "alcanzar termina la persecución");
    assert!(n.perseguidores_de(Objetivo::Ejercito(q)).is_empty());
    // El encuentro hostil también deja su hecho (el mismo instante), pero ningún combate.
    assert!(hechos(&n, "ejercito.encuentro").iter().any(|h| h.0 == t));
    assert!(hechos(&n, "combate.resuelto").is_empty());
    // Quien persigue sigue su plan hasta el final y acampa.
    n.avanzar_hasta(30 * MIN);
    assert!(!n.ejercito(p).unwrap().marchando());
}

#[test]
fn el_perseguidor_rehace_su_plan_cada_vez_que_la_presa_cambia_el_suyo_y_al_llegar_sin_alcanzarla() {
    let (mut n, p, q) = cazador_y_presa();
    n.aplicar_en(0, marchar("beto", "m1", q, c(39, 2)));
    assert!(perseguir(&mut n, 0, "ana", "p", p, Objetivo::Ejercito(q)).aceptada());
    let destino_antes = n.ejercito(p).unwrap().plan.vertices.last().unwrap().p;
    let gen_antes = n.ejercito(p).unwrap().plan.generacion;
    // A los 3 s la presa da media vuelta hacia otra fila: el plan del perseguidor se rehace desde donde está, hacia el punto nuevo.
    n.avanzar_hasta(3_000);
    n.aplicar_en(3_000, marchar("beto", "m2", q, c(1, 0)));
    let e = n.ejercito(p).unwrap();
    assert!(e.plan.generacion > gen_antes);
    assert_ne!(e.plan.vertices.last().unwrap().p, destino_antes);
    assert_eq!(e.plan.vertices.first().unwrap().en, 3_000, "parte de donde está en ese instante");
    assert_eq!(n.persiguiendo(p), Some(Objetivo::Ejercito(q)));
    n.avanzar_hasta(30 * MIN);
    assert_eq!(hechos(&n, "columna.presa_alcanzada").len(), 1);
    // `marcharA` suelta la presa sin avisar, y `dejarDePerseguir` avisa, también sin presa (como el TypeScript, paridad P3).
    let (mut n, p, q) = cazador_y_presa();
    n.aplicar_en(0, marchar("beto", "m1", q, c(39, 2)));
    assert!(dejar(&mut n, 0, "ana", "d0", p).aceptada());
    assert_eq!(hechos(&n, "columna.persecucion_abandonada").len(), 1, "sin presa también avisa");
    perseguir(&mut n, 0, "ana", "p", p, Objetivo::Ejercito(q));
    assert_eq!(codigo(&dejar(&mut n, 0, "beto", "d1", p)), rechazos::NO_AUTORIZADO);
    assert!(dejar(&mut n, 1_000, "ana", "d2", p).aceptada());
    assert_eq!((n.persiguiendo(p), hechos(&n, "columna.persecucion_abandonada").len()), (None, 2));
    perseguir(&mut n, 2_000, "ana", "p3", p, Objetivo::Ejercito(q));
    assert!(n.aplicar_en(3_000, marchar("ana", "m", p, c(0, 4))).aceptada());
    assert_eq!((n.persiguiendo(p), hechos(&n, "columna.persecucion_abandonada").len()), (None, 2), "marcharA suelta sin evento");
    perseguir(&mut n, 4_000, "ana", "p4", p, Objetivo::Ejercito(q));
    assert!(n.aplicar_en(5_000, Peticion::nueva("ana", "e", Comando::Estacionar { ejercito: p })).aceptada());
    assert_eq!(n.persiguiendo(p), None, "una columna parada no persigue");
}

#[test]
fn la_presa_que_sale_del_mapa_se_da_por_puesta_a_cubierto_y_dos_que_se_persiguen_no_se_enredan() {
    let (mut n, p, q) = cazador_y_presa();
    n.aplicar_en(0, marchar("beto", "m", q, c(39, 2)));
    assert!(perseguir(&mut n, 0, "ana", "p", p, Objetivo::Ejercito(q)).aceptada());
    n.avanzar_hasta(2_000);
    assert!(n.columna_entra_en_batalla(2_000, q));
    let cubierta = hechos(&n, "columna.presa_a_cubierto");
    assert_eq!((cubierta.len(), cubierta[0].0, cubierta[0].1.clone()), (1, 2_000, vec![p, q]));
    assert_eq!((n.persiguiendo(p), n.perseguidores_de(Objetivo::Ejercito(q))), (None, vec![]));
    assert_eq!(codigo(&perseguir(&mut n, 2_001, "ana", "p2", p, Objetivo::Ejercito(q))), rechazos::PERSECUCION_A_CUBIERTO, "no se persigue lo que no está en el mapa");

    // Dos columnas que se persiguen entre sí: la cascada de re-planes termina.
    let mut n = nucleo(40, 5);
    let a = n.crear_ejercito(0, "ana", 1, c(0, 2), 800, 350_000).unwrap();
    let b = n.crear_ejercito(0, "beto", 2, c(1, 2), 800, 350_000).unwrap();
    assert!(perseguir(&mut n, 0, "ana", "pa", a, Objetivo::Ejercito(b)).aceptada());
    assert!(perseguir(&mut n, 0, "beto", "pb", b, Objetivo::Ejercito(a)).aceptada());
    n.avanzar_hasta(5 * MIN);
    assert!(n.eventos_en_cola() < 100);
}

#[test]
fn perseguir_se_rechaza_sin_vista_con_los_tuyos_y_con_lo_que_las_reglas_prohiben_sin_dejar_rastro() {
    let (mut n, p, q) = cazador_y_presa();
    let lejos = n.crear_ejercito(0, "gus", 3, c(5, 2), 500, 350_000).unwrap();
    let propia = n.crear_ejercito(0, "hugo", 1, c(1, 3), 500, 350_000).unwrap();
    let hp = heroe(&n, p);
    let orden = |n: &mut Nucleo, actor: &str, objetivo| n.aplicar_en(0, Peticion::nueva(actor, "x", Comando::Perseguir { heroe: hp, objetivo }));
    let (v, hash, ids) = (n.version(), n.hash_estado(), n.estado().siguiente_id);
    assert_eq!(codigo(&orden(&mut n, "ana", Objetivo::Ejercito(lejos))), rechazos::PERSECUCION_FUERA_DE_VISTA, "a 32 unidades, con la vista en 15");
    assert_eq!(codigo(&orden(&mut n, "ana", Objetivo::Ejercito(propia))), rechazos::PERSECUCION_PROPIA_O_ALIADA);
    assert_eq!(codigo(&orden(&mut n, "ana", Objetivo::Ejercito(p))), rechazos::PERSECUCION_A_SI_MISMA);
    assert_eq!(codigo(&orden(&mut n, "ana", Objetivo::Ejercito(9_999))), rechazos::EJERCITO_DESCONOCIDO);
    assert_eq!(codigo(&orden(&mut n, "ana", Objetivo::Caravana(9_999))), rechazos::CARAVANA_DESCONOCIDA);
    assert_eq!(codigo(&orden(&mut n, "ana", Objetivo::Asentamiento(1))), rechazos::OBJETIVO_NO_SOPORTADO);
    assert_eq!(codigo(&orden(&mut n, "beto", Objetivo::Ejercito(q))), rechazos::NO_AUTORIZADO, "la columna no es de beto");
    assert_eq!(codigo(&n.aplicar_en(0, Peticion::nueva("ana", "x", Comando::Perseguir { heroe: 99_999, objetivo: Objetivo::Ejercito(q) }))), rechazos::HEROE_DESCONOCIDO);
    assert_eq!((n.version(), n.hash_estado(), n.estado().siguiente_id), (v, hash, ids), "los rechazos no dejan rastro");
    // Un aliado tampoco: la alianza se mira al ordenar.
    n.aplicar_en(0, admin("r", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Alianza) }));
    assert_eq!(codigo(&orden(&mut n, "ana", Objetivo::Ejercito(q))), rechazos::PERSECUCION_PROPIA_O_ALIADA);
    n.aplicar_en(0, admin("r2", Comando::FijarRelacion { a: 1, b: 2, relacion: None }));
    // Un herido ni persigue ni es perseguido.
    let hq = heroe(&n, q);
    n.aplicar_en(1, admin("h1", Comando::HerirHeroe { heroe: hq }));
    assert_eq!(codigo(&orden(&mut n, "ana", Objetivo::Ejercito(q))), rechazos::PERSECUCION_PRESA_SIN_HEROE_SANO);
    n.aplicar_en(1, admin("h2", Comando::HerirHeroe { heroe: hp }));
    assert_eq!(codigo(&orden(&mut n, "ana", Objetivo::Ejercito(q))), rechazos::PERSECUCION_SIN_HEROE_SANO);
    assert!(n.persiguiendo(p).is_none() && n.perseguidores_de(Objetivo::Ejercito(q)).is_empty());
}

#[test]
fn la_presa_que_sale_de_la_vista_cancela_la_persecucion_en_el_instante_exacto() {
    // Él huye a 1 u/s y ella lo sigue a 0,5: la distancia crece 0,5 u/s desde 8 y supera 15 a los 14 s (en el siguiente milisegundo).
    let mut n = nucleo(40, 5);
    let p = n.crear_ejercito(0, "ana", 1, c(0, 2), 500, 350_000).unwrap();
    let q = n.crear_ejercito(0, "beto", 2, c(1, 2), 1_000, 350_000).unwrap();
    assert!(n.aplicar_en(0, marchar("beto", "m", q, c(39, 2))).aceptada());
    assert!(perseguir(&mut n, 0, "ana", "p", p, Objetivo::Ejercito(q)).aceptada());
    n.avanzar_hasta(14_000);
    assert_eq!(n.persiguiendo(p), Some(Objetivo::Ejercito(q)), "a 14 s está justo a 15 unidades: aún la ve");
    n.avanzar_hasta(14_001);
    assert_eq!(n.persiguiendo(p), None);
    let perdida = hechos(&n, "columna.presa_perdida");
    assert_eq!((perdida.len(), perdida[0].0, perdida[0].1.clone()), (1, 14_001, vec![p, q]));
    assert!(hechos(&n, "columna.presa_alcanzada").is_empty());
    // Quien persigue no se detiene por ello: termina el rumbo que llevaba.
    assert!(n.ejercito(p).unwrap().marchando());
}

// ───────────────────────────── 3. Determinismo y fotos ─────────────────────────────

/// Un escenario con persecuciones, encuentros, inspecciones y relaciones. `avanzar` decide cómo se mueve el reloj entre órdenes.
fn escenario_con_persecuciones(n: &mut Nucleo, avanzar: &dyn Fn(&mut Nucleo, Instante)) {
    let mut rng = Rng::derivado(5, "persecuciones");
    let ids: Vec<u64> = (0..24)
        .map(|i| {
            let p = c(rng.por_debajo(6) as i64, rng.por_debajo(3) as i64); // todas cerca: se ven, se cruzan y se persiguen
            n.crear_ejercito(0, &format!("c{i}"), (i % 3) as u32, p, 400 + rng.por_debajo(700) as i64, 350_000).unwrap()
        })
        .collect();
    n.aplicar_en(0, admin("r", Comando::FijarRelacion { a: 0, b: 1, relacion: Some(Relacion::Alianza) }));
    for paso in 1..400u64 {
        let t = paso as i64 * 2_000;
        avanzar(n, t);
        let (a, b) = (ids[rng.por_debajo(ids.len() as u64) as usize], ids[rng.por_debajo(ids.len() as u64) as usize]);
        let cuenta = format!("c{}", ids.iter().position(|&x| x == a).unwrap());
        match rng.por_debajo(6) {
            0 | 1 => {
                n.aplicar_en(t, marchar(&cuenta, &format!("m{paso}"), a, c(rng.por_debajo(10) as i64, rng.por_debajo(6) as i64)));
            }
            2 | 3 => {
                perseguir(n, t, &cuenta, &format!("p{paso}"), a, Objetivo::Ejercito(b));
            }
            4 => {
                inspeccionar(n, t, &cuenta, &format!("i{paso}"), a, Objetivo::Ejercito(b));
            }
            _ => {
                dejar(n, t, &cuenta, &format!("d{paso}"), a);
            }
        }
    }
}

fn hash_con(avanzar: &dyn Fn(&mut Nucleo, Instante)) -> ([u8; 32], usize, usize) {
    let mut n = nucleo(30, 10);
    escenario_con_persecuciones(&mut n, avanzar);
    n.avanzar_hasta(20 * MIN);
    let persecuciones = hechos(&n, "columna.persecucion_iniciada").len();
    (n.hash_estado(), persecuciones, hechos(&n, "ejercito.encuentro").len())
}

#[test]
fn las_persecuciones_son_deterministas_da_igual_como_se_trocee_el_reloj_y_el_hash_refleja_el_estado() {
    let (a, persecuciones, encuentros) = hash_con(&|n, t| n.avanzar_hasta(t));
    assert!(persecuciones > 5 && encuentros > 3, "escenario poco variado: {persecuciones} persecuciones, {encuentros} encuentros");
    assert_eq!(a, hash_con(&|n, t| n.avanzar_hasta(t)).0, "repetible");
    assert_eq!(a, hash_con(&|_, _| {}).0, "sin avances intermedios");
    assert_eq!(
        a,
        hash_con(&|n, t| {
            let ahora = n.ahora();
            for s in 1..=4 {
                n.avanzar_hasta(ahora + (t - ahora).max(0) * s / 4);
            }
        })
        .0,
        "a trozos"
    );
    // El hash cambia con la persecución: la misma historia sin una orden de perseguir da otro mundo.
    let (mut n, p, q) = cazador_y_presa();
    n.aplicar_en(0, marchar("beto", "m", q, c(39, 2)));
    let sin = n.hash_estado();
    assert!(perseguir(&mut n, 0, "ana", "p", p, Objetivo::Ejercito(q)).aceptada());
    assert_ne!(n.hash_estado(), sin);
    let con = n.hash_estado();
    dejar(&mut n, 0, "ana", "d", p);
    assert_ne!(n.hash_estado(), con, "soltar la presa también es estado");
}

/// El escenario de la foto: una persecución viva con un informe y una relación, a los 4 s de mundo.
fn persecucion_viva(n: &mut Nucleo) -> (u64, u64) {
    let p = n.crear_ejercito(0, "ana", 1, c(0, 2), 1_000, 350_000).unwrap();
    let q = n.crear_ejercito(0, "beto", 2, c(1, 2), 500, 350_000).unwrap();
    n.aplicar_en(0, admin("r", Comando::FijarRelacion { a: 2, b: 7, relacion: Some(Relacion::Guerra) }));
    n.aplicar_en(0, marchar("beto", "m", q, c(39, 2)));
    assert!(perseguir(n, 0, "ana", "p", p, Objetivo::Ejercito(q)).aceptada());
    assert!(inspeccionar(n, 0, "ana", "i", p, Objetivo::Ejercito(q)).aceptada());
    n.avanzar_hasta(4_000);
    (p, q)
}

#[test]
fn la_foto_con_una_persecucion_viva_conserva_la_presa_los_informes_las_relaciones_y_el_espacio() {
    let mut n = nucleo(40, 5);
    let (p, q) = persecucion_viva(&mut n);
    let bytes = n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN48");
    let vuelta = nucleo::Estado::deserializar(&bytes, &comun::reglas(), "hash-de-prueba").expect("deserializa");
    assert_eq!(vuelta.serializar(), bytes, "ida y vuelta idéntica");
    assert_eq!(vuelta.ejercitos[&p].persiguiendo, Some(q));
    assert_eq!(vuelta.militar.heroes[&heroe(&n, p)].informes.len(), 1);
    assert_eq!(vuelta.relacion(7, 2), Some(Relacion::Guerra));

    // Seguir desde una caída da lo mismo que no parar: se rehace el espacio y la persecución acaba igual, una sola vez.
    let alcanzadas = |n: &Nucleo, desde: u64| n.cambios_desde(desde).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == "columna.presa_alcanzada").count();
    let v0 = n.version();
    n.avanzar_hasta(30 * MIN);
    let (continuo, a_continuo) = (n.hash_estado(), alcanzadas(&n, v0));

    let dir = tempfile::tempdir().unwrap();
    let cfg = persistencia::Config { max_segmento: 64 << 10, sincronia: persistencia::Sincronia::Nunca, fotos_conservadas: 3 };
    let abrir = || Nucleo::con_persistencia(Arranque { ajustes: Ajustes { foto_cada: 3, latido_ms: 0, ..Ajustes::default() }, ..comun::arranque(comun::reticula(40, 5)) }, dir.path(), cfg).unwrap();
    {
        let mut n = abrir();
        persecucion_viva(&mut n);
        n.esperar_fotos();
    }
    let mut n = abrir();
    assert!(n.recuperado());
    assert_eq!(n.persiguiendo(p), Some(Objetivo::Ejercito(q)), "la persecución sobrevive a la caída");
    assert_eq!(n.perseguidores_de(Objetivo::Ejercito(q)), vec![p]);
    assert_eq!(n.cosas_en_el_espacio(), 2, "el espacio se reconstruye");
    let v0 = n.version();
    n.avanzar_hasta(30 * MIN);
    assert_eq!(alcanzadas(&n, v0), 1);
    assert_eq!((a_continuo, n.hash_estado()), (1, continuo), "el mundo sigue igual que si no hubiera caído");
}

// ───────────────────────────── 4. Inspeccionar ─────────────────────────────

#[test]
fn inspeccionar_guarda_un_informe_diez_minutos_avisa_al_observado_y_se_comprueba_antes_de_guardar() {
    let mut n = nucleo(40, 5);
    let a = n.crear_ejercito(0, "ana", 1, c(0, 2), 500, 350_000).unwrap();
    let b = n.crear_ejercito(0, "beto", 2, c(1, 2), 500, 350_000).unwrap();
    let lejos = n.crear_ejercito(0, "gus", 3, c(2, 2), 500, 350_000).unwrap();
    let (v, hash) = (n.version(), n.hash_estado());
    assert_eq!(codigo(&inspeccionar(&mut n, 0, "ana", "i1", a, Objetivo::Ejercito(lejos))), rechazos::INSPECCION_LEJOS, "a 16 unidades, con el anillo en 8");
    assert_eq!(codigo(&inspeccionar(&mut n, 0, "ana", "i2", a, Objetivo::Ejercito(a))), rechazos::INSPECCION_LA_TUYA);
    assert_eq!(codigo(&inspeccionar(&mut n, 0, "beto", "i3", a, Objetivo::Ejercito(b))), rechazos::NO_AUTORIZADO);
    assert_eq!(codigo(&inspeccionar(&mut n, 0, "ana", "i4", a, Objetivo::Ejercito(9_999))), rechazos::EJERCITO_DESCONOCIDO);
    assert_eq!(codigo(&inspeccionar(&mut n, 0, "ana", "i5", a, Objetivo::Caravana(9_999))), rechazos::CARAVANA_DESCONOCIDA);
    assert_eq!(codigo(&inspeccionar(&mut n, 0, "ana", "i6", a, Objetivo::Asentamiento(9_999))), rechazos::ASENTAMIENTO_DESCONOCIDO);
    assert_eq!((n.version(), n.hash_estado()), (v, hash), "los rechazos no dejan rastro");

    let ha = heroe(&n, a);
    let r = inspeccionar(&mut n, 1_000, "ana", "i7", a, Objetivo::Ejercito(b));
    assert!(r.aceptada(), "{r:?}");
    let informes = n.informes_de_inspeccion(ha);
    assert_eq!(informes.len(), 1);
    let i = informes[0];
    assert_eq!((i.objetivo, i.visto_en, i.expira_en, i.donde), (Objetivo::Ejercito(b), 1_000, 1_000 + 10 * MIN, c(1, 2)));
    assert_eq!(i.contenido, Contenido::Columna { faccion: Some(2), heroes: vec![heroe(&n, b)], escuadras: vec![] });
    // El aviso es del observado.
    let aviso = hechos(&n, "columna.observada");
    assert_eq!((aviso.len(), aviso[0].1.clone(), dato(&aviso[0], "observador")), (1, vec![b], ha as i64));
    // Mirar otra vez lo renueva (uno por objetivo) y a los diez minutos caduca.
    assert!(inspeccionar(&mut n, 5 * MIN, "ana", "i8", a, Objetivo::Ejercito(b)).aceptada());
    assert_eq!(n.informes_de_inspeccion(ha).len(), 1);
    n.avanzar_hasta(15 * MIN - 1);
    assert_eq!(n.informes_de_inspeccion(ha).len(), 1);
    n.avanzar_hasta(15 * MIN);
    assert!(n.informes_de_inspeccion(ha).is_empty());
    assert_eq!(hechos(&n, "columna.observada").len(), 2);
    // Una columna a cubierto no se mira.
    assert!(n.columna_entra_en_batalla(15 * MIN, b));
    assert_eq!(codigo(&inspeccionar(&mut n, 15 * MIN, "ana", "i9", a, Objetivo::Ejercito(b))), rechazos::INSPECCION_A_CUBIERTO);
}

// ───────────────────────────── 5. Plazas e inspección de plazas (mundo real) ─────────────────────────────

type M = (Arc<Mundo>, Arc<Rutas>);

fn mundo() -> Option<&'static M> {
    static M: OnceLock<Option<M>> = OnceLock::new();
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

/// Un sitio verde con mucho bosque cerca del campamento `i`: una plaza que no cae en ruinas por falta de madera.
fn sitio_rico(n: &Nucleo, m: &Mundo, i: usize, faccion: u32) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    for r in (0..8).map(|k| 50_000 + k * 10_000) {
        for g in 0..360 {
            let a = (g as f64).to_radians();
            let q = Punto::new(p.x + (r as f64 * a.cos()) as i64, p.y + (r as f64 * a.sin()) as i64);
            if n.viabilidad(q, faccion).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6) {
                return q;
            }
        }
    }
    panic!("sin sitio rico cerca del campamento {i}");
}

fn fundar(n: &mut Nucleo, t: Instante, dueno: &str, faccion: u32, p: Punto) -> u64 {
    let e = n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap();
    assert!(n.aplicar_en(t, admin(&format!("c-{e}"), Comando::DarCaravanaFundacion { ejercito: e })).aceptada());
    let r = n.aplicar_en(t, Peticion::nueva(dueno, &format!("f-{e}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

/// Una columna de otra Facción a unos 14 unidades de la plaza, a la que se puede ir por tierra: el primer ángulo con ruta.
fn columna_cerca_de(n: &mut Nucleo, plaza: Punto, faccion: u32, cuenta: &str) -> u64 {
    for g in (0..360).step_by(15) {
        let a = (g as f64).to_radians();
        let p = Punto::new(plaza.x + (14_000.0 * a.cos()) as i64, plaza.y + (14_000.0 * a.sin()) as i64);
        let id = n.crear_ejercito(0, cuenta, faccion, p, 500, 350_000).unwrap();
        if n.aplicar_en(0, marchar(cuenta, &format!("ruta-{id}"), id, plaza)).aceptada() {
            return id;
        }
        n.aplicar_en(0, admin(&format!("fuera-{id}"), Comando::Estacionar { ejercito: id }));
    }
    panic!("sin camino hasta la plaza");
}

#[test]
fn una_plaza_ajena_es_una_cosa_fija_del_espacio_y_una_columna_hostil_que_llega_deja_su_encuentro() {
    let m = con_mundo!();
    let mut n = Nucleo::nuevo(arranque(m, 7));
    let p = sitio_rico(&n, &m.0, 0, 1);
    // Los campamentos de bandidos de las vetas ya están en el espacio desde el arranque.
    let bandidos = n.campamentos_de_bandidos().count();
    assert_eq!(n.cosas_en_el_espacio(), bandidos);
    let plaza = fundar(&mut n, 0, "ana", 1, p);
    // La columna que fundó sigue en el mapa; la plaza se suma al espacio.
    assert_eq!(n.cosas_en_el_espacio(), bandidos + 2);
    // Una de la propia Facción pasa por la puerta sin dejar nada; una hostil, sí.
    let amiga = columna_cerca_de(&mut n, p, 1, "gus");
    let hostil = columna_cerca_de(&mut n, p, 2, "beto");
    n.avanzar_hasta(30 * MIN);
    let e = hechos(&n, "asentamiento.encuentro");
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!((e[0].1.clone(), e[0].2.clone(), dato(&e[0], "faccionA"), dato(&e[0], "faccionB")), (vec![hostil], vec![plaza], 2, 1));
    assert!(n.ejercito(hostil).unwrap().posicion_en(e[0].0).dist(p) <= 3_000 + 1);
    let _ = amiga;
}

#[test]
fn inspeccionar_una_plaza_ajena_guarda_su_defensa_y_avisa_a_la_plaza() {
    let m = con_mundo!();
    let mut n = Nucleo::nuevo(arranque(m, 7));
    let p = sitio_rico(&n, &m.0, 0, 1);
    let plaza = fundar(&mut n, 0, "ana", 1, p);
    let obs = n.crear_ejercito(0, "beto", 2, Punto::new(p.x + 6_000, p.y), 500, 350_000).unwrap();
    let propia = n.crear_ejercito(0, "gus", 1, Punto::new(p.x + 6_000, p.y), 500, 350_000).unwrap();
    let lejos = n.crear_ejercito(0, "hugo", 2, Punto::new(p.x + 12_000, p.y), 500, 350_000).unwrap();
    assert_eq!(codigo(&inspeccionar(&mut n, 0, "gus", "a", propia, Objetivo::Asentamiento(plaza))), rechazos::INSPECCION_PLAZA_PROPIA);
    assert_eq!(codigo(&inspeccionar(&mut n, 0, "hugo", "b", lejos, Objetivo::Asentamiento(plaza))), rechazos::INSPECCION_LEJOS);
    assert!(inspeccionar(&mut n, 0, "beto", "c", obs, Objetivo::Asentamiento(plaza)).aceptada());
    let ho = heroe(&n, obs);
    let informes = n.informes_de_inspeccion(ho);
    assert_eq!(informes.len(), 1);
    assert_eq!(informes[0].contenido, Contenido::Plaza { faccion: 1, heroes: vec![], guarnicion: vec![], recintos: vec![] });
    let aviso = hechos(&n, "asentamiento.observado");
    assert_eq!((aviso.len(), aviso[0].2.clone()), (1, vec![plaza]));
}

// ───────────────────────────── 6. Coste de la rejilla con muchas columnas ─────────────────────────────

/// Mide el coste de la rejilla con muchas columnas a la vez: dar de alta columnas quietas, ordenar la marcha de un tercio (descontando el A\* de la ruta,
/// que se mide aparte con las mismas rutas) y dejar correr 2 h de mundo con sus encuentros.
/// `cargo test --release -p nucleo --test encuentros rendimiento -- --ignored --nocapture`.
fn medir(columnas: u64, lado: i64) {
    let mut n = nucleo(lado, lado);
    let mut rng = Rng::derivado(9, "medir");
    let t0 = std::time::Instant::now();
    let ids: Vec<(u64, Punto)> = (0..columnas)
        .map(|i| {
            let p = c(rng.por_debajo(lado as u64) as i64, rng.por_debajo(lado as u64) as i64);
            (n.crear_ejercito(0, &format!("c{}", i % 50), (i % 8) as u32, p, 300 + rng.por_debajo(500) as i64, 3_500_000).unwrap(), p)
        })
        .collect();
    let altas = t0.elapsed();
    // Un tercio sale a marchar; el resto, acampado: la rejilla lleva las dos cosas.
    let marchan: Vec<(usize, u64, Punto, Punto)> = ids.iter().enumerate().filter(|(k, _)| k % 3 == 0).map(|(k, &(id, p))| (k, id, p, c(rng.por_debajo(lado as u64) as i64, rng.por_debajo(lado as u64) as i64))).collect();
    let t1 = std::time::Instant::now();
    for &(_, _, desde, hasta) in &marchan {
        n.rutas().ruta(desde, hasta, &rutas::Opciones::default());
    }
    let solo_astar = t1.elapsed();
    let (e0, t2) = (n.metricas().eventos, std::time::Instant::now());
    let ordenes = marchan.iter().filter(|&&(k, id, _, hasta)| n.aplicar_en(0, marchar(&format!("c{}", k % 50), &format!("m{k}"), id, hasta)).aceptada()).count() as u64;
    let ordenar = t2.elapsed();
    let t3 = std::time::Instant::now();
    n.avanzar_hasta(2 * 60 * MIN);
    let (correr, eventos) = (t3.elapsed(), n.metricas().eventos - e0);
    let rejilla = ordenar.saturating_sub(solo_astar);
    println!(
        "{columnas} columnas en {lado}×{lado} celdas: altas {:.2} s ({:.1} µs por columna quieta con su rejilla); {ordenes} órdenes de marcha en {:.2} s, de ellas {:.2} s de A* = {:.1} µs por orden de rejilla y encuentros; 2 h de mundo: {eventos} eventos en {:.2} s ({:.1} µs por evento); {} encuentros; {} cosas en el espacio; {} eventos aún en la cola",
        altas.as_secs_f64(),
        altas.as_secs_f64() * 1e6 / columnas as f64,
        ordenar.as_secs_f64(),
        solo_astar.as_secs_f64(),
        rejilla.as_secs_f64() * 1e6 / ordenes.max(1) as f64,
        correr.as_secs_f64(),
        correr.as_secs_f64() * 1e6 / eventos.max(1) as f64,
        hechos(&n, "ejercito.encuentro").len(),
        n.cosas_en_el_espacio(),
        n.eventos_en_cola()
    );
    assert!(ordenes >= columnas / 4);
    assert_eq!(n.cosas_en_el_espacio() as u64, columnas);
}

#[test]
fn muchas_columnas_en_la_rejilla_pequeno() {
    medir(300, 60);
}

#[test]
#[ignore = "mide 5.000 columnas en una retícula de 300 × 300; en release"]
fn rendimiento_de_5000_columnas() {
    medir(5_000, 300);
}
