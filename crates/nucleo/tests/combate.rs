//! El combate con números en el núcleo (fase 4D; Combate §2-§11, Generacion_Del_Mundo §13): `atacar` con sus objetivos y sus consecuencias, la conquista con
//! su protección y su ocupación, los bandidos en el espacio, el saqueo de un pueblo y los ganchos de la caravana. Las pruebas de la primera mitad corren sobre
//! la retícula sintética de `comun` (columnas de administración con escuadras de `DarEscuadra`, sin mundo); las de la segunda, sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`) y se omiten si no está.

mod comun;

use asentamiento::Recurso::{self, *};
use base::{Instante, Punto, Rng, MS_POR_MINUTO};
use comun::{c, nucleo};
use militar::catalogo::Tropa;
use nucleo::{rechazos, Comando, Nucleo, ObjetivoAtaque, Peticion, Relacion, Resultado, Respuesta};

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

fn mc() -> combate::Reglas {
    combate::Reglas::cargar()
}

/// Una columna de la cuenta en `p`: devuelve (columna, héroe).
fn columna(n: &mut Nucleo, cuenta: &str, faccion: u32, p: Punto) -> (u64, u64) {
    columna_en(n, n.ahora(), cuenta, faccion, p)
}

fn columna_en(n: &mut Nucleo, t: Instante, cuenta: &str, faccion: u32, p: Punto) -> (u64, u64) {
    let id = n.crear_ejercito(t, cuenta, faccion, p, 500, 350_000).unwrap();
    (id, n.ejercito(id).unwrap().columna.lider)
}

/// Le da al héroe una escuadra completa de esa tropa (a bordo de su columna) y devuelve su id.
fn dar(n: &mut Nucleo, id: &str, heroe: u64, tropa: Tropa, cantidad: i64) -> u64 {
    let r = n.aplicar_en(n.ahora(), admin(id, Comando::DarEscuadra { heroe, tropa, cantidad }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

fn completa(tropa: Tropa) -> i64 {
    militar::Reglas::cargar().tropas.tropa(tropa).unidades
}

fn atacar(n: &mut Nucleo, t: Instante, actor: &str, id: &str, heroe: u64, objetivo: ObjetivoAtaque, guerra: bool) -> Respuesta {
    n.aplicar_en(t, Peticion::nueva(actor, id, Comando::Atacar { heroe, objetivo, declara_guerra: guerra }))
}

fn en_guerra(n: &mut Nucleo, a: u32, b: u32) {
    let r = n.aplicar_en(n.ahora(), admin(&format!("guerra-{a}-{b}"), Comando::FijarRelacion { a, b, relacion: Some(Relacion::Guerra) }));
    assert!(r.aceptada(), "{r:?}");
}

fn soldados(n: &Nucleo, heroe: u64, escuadra: u64) -> i64 {
    n.heroe(heroe).unwrap().heroe.escuadra(escuadra).unwrap().cantidad
}

/// El estado del generador de la partida tras `k` tiradas desde el principio (la semilla de `comun::arranque` es 42).
fn rng_tras(k: usize) -> u64 {
    let mut r = Rng::derivado(42, "nucleo");
    (0..k).for_each(|_| {
        r.siguiente();
    });
    r.estado()
}

fn rastro(n: &Nucleo) -> (u64, [u8; 32], u64, u64) {
    (n.version(), n.hash_estado(), n.estado().rng.estado(), n.estado().siguiente_id)
}

/// Dos columnas de Facciones distintas, a menos de `radioEncuentro` una de otra y en guerra. La de «ana» (F1) lleva espadachines y arqueros; la de «beto»
/// (F2), una milicia: gana ana con holgura.
struct Choque {
    n: Nucleo,
    ana: (u64, u64),
    beto: (u64, u64),
    fuerte: Vec<u64>,
    debil: u64,
}

fn choque() -> Choque {
    let mut n = nucleo(40, 5);
    let ana = columna(&mut n, "ana", 1, c(10, 2));
    let beto = columna(&mut n, "beto", 2, Punto::new(c(10, 2).x + 2_000, c(10, 2).y));
    let fuerte = vec![dar(&mut n, "d1", ana.1, Tropa::EspadachinesBronce, completa(Tropa::EspadachinesBronce)), dar(&mut n, "d2", ana.1, Tropa::ArquerosCompuesto, completa(Tropa::ArquerosCompuesto))];
    let debil = dar(&mut n, "d3", beto.1, Tropa::MiliciaLanceros, completa(Tropa::MiliciaLanceros));
    en_guerra(&mut n, 1, 2);
    Choque { n, ana, beto, fuerte, debil }
}

// ───────────────────────────── 1. Columna contra columna ─────────────────────────────

#[test]
fn una_columna_ataca_a_otra_y_el_derrotado_queda_herido_pierde_la_mitad_del_carro_y_el_ganador_cobra_botin_y_chatarra() {
    let mut x = choque();
    let (ana, beto) = (x.ana, x.beto);
    // Beto lleva un carro con trigo y madera; ana, uno vacío.
    for (id, r, q) in [("s1", Trigo, 100_000), ("s2", Madera, 41_001)] {
        assert!(x.n.aplicar_en(x.n.ahora(), admin(id, Comando::DarSuministro { ejercito: beto.0, recurso: r, cantidad: q })).aceptada());
    }
    let (antes_a, antes_d) = (soldados(&x.n, ana.1, x.fuerte[0]), soldados(&x.n, beto.1, x.debil));
    let (v, rng0) = (x.n.version(), x.n.estado().rng.estado());
    assert_eq!(rng0, rng_tras(0), "nada gastó una tirada antes");

    let r = atacar(&mut x.n, 10 * MIN, "ana", "a1", ana.1, ObjetivoAtaque::Ejercito(beto.0), false);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(x.n.version(), v + 1, "una orden aceptada sube la versión una vez");
    assert_eq!(x.n.estado().rng.estado(), rng_tras(2), "un combate entre columnas gasta exactamente dos tiradas");

    let h = &hechos(&x.n, "combate.resuelto");
    assert_eq!(h.len(), 1);
    let h = &h[0];
    assert_eq!((h.0, h.1.clone(), dato(h, "ganaElAtacante")), (10 * MIN, vec![ana.0, beto.0], 1));
    // Las bajas del perdedor van de 30 % a 70 % y el ganador pierde entre el 5 % y el 20 %.
    let (despues_a, despues_d) = (soldados(&x.n, ana.1, x.fuerte[0]), soldados(&x.n, beto.1, x.debil));
    let perdidos_d = antes_d - despues_d;
    assert!((antes_d * 30 / 100..=antes_d * 70 / 100 + 1).contains(&perdidos_d), "el derrotado perdió {perdidos_d} de {antes_d}");
    assert!(antes_a - despues_a <= antes_a * 20 / 100 + 1, "el ganador perdió poco: {} de {antes_a}", antes_a - despues_a);
    assert_eq!((dato(h, "bajasDefensor"), dato(h, "bajasAtacante")), (perdidos_d, antes_a - despues_a + (completa(Tropa::ArquerosCompuesto) - soldados(&x.n, ana.1, x.fuerte[1]))));
    // La experiencia: 1 punto por ganar y medio por perder.
    let xp = |h: u64, e: u64| x.n.heroe(h).unwrap().heroe.escuadra(e).unwrap().experiencia;
    assert_eq!((xp(ana.1, x.fuerte[0]), xp(ana.1, x.fuerte[1]), xp(beto.1, x.debil)), (1_000, 1_000, 500));
    // Todos los héroes del derrotado quedan heridos; el vencedor no.
    assert!(x.n.heroe_herido(beto.1) && !x.n.heroe_herido(ana.1));
    assert_eq!(x.n.heroe(beto.1).unwrap().heroe.herido_hasta, Some(10 * MIN + 2 * MIN), "heridoMinutos = 2");
    // El carro del derrotado pierde la mitad (hacia abajo) de cada recurso y el ganador se la queda con la chatarra de las bajas, hasta donde le cabe.
    let carga = |col: u64| x.n.ejercito(col).unwrap().columna.carga.clone();
    assert_eq!((carga(beto.0)[&Trigo], carga(beto.0)[&Madera]), (50_000, 20_501));
    let ganada = carga(ana.0);
    assert_eq!((ganada[&Trigo], ganada[&Madera]), (50_000, 20_500));
    let chatarra: i64 = ganada.iter().filter(|(r, _)| matches!(r, ChatarraCobre | ChatarraBronce | ChatarraHierro)).map(|(_, n)| n).sum();
    assert_eq!(chatarra, dato(h, "chatarra"), "la chatarra de las bajas de los dos bandos va al carro del ganador");
    assert!(chatarra > 0 && dato(h, "botin") == 70_500 + chatarra);
    // Intocable: sin ningún héroe sano ni se le ataca ni ataca.
    let rastro0 = rastro(&x.n);
    assert_eq!(codigo(&atacar(&mut x.n, 11 * MIN, "ana", "a2", ana.1, ObjetivoAtaque::Ejercito(beto.0), false)), rechazos::COMBATE_SIN_HEROE_SANO);
    assert_eq!(codigo(&atacar(&mut x.n, 11 * MIN, "beto", "b1", beto.1, ObjetivoAtaque::Ejercito(ana.0), false)), rechazos::HEROE_HERIDO);
    assert_eq!(rastro(&x.n).2, rastro0.2, "ninguno de los dos gastó una tirada");
    // Y se cura al cabo de dos minutos, sin evento.
    x.n.avanzar_hasta(12 * MIN);
    assert!(!x.n.heroe_herido(beto.1));
}

#[test]
fn las_ordenes_que_no_valen_se_rechazan_sin_rastro_y_en_su_orden() {
    let mut x = choque();
    let (ana, beto) = (x.ana, x.beto);
    let ataque = |o: ObjetivoAtaque, h: u64| Comando::Atacar { heroe: h, objetivo: o, declara_guerra: false };
    let otra = columna(&mut x.n, "gus", 3, c(10, 2));
    let lejana = columna(&mut x.n, "hugo", 2, c(30, 2));
    let personal_f2 = columna(&mut x.n, "ivan", 2, c(10, 2));
    x.n.avanzar_hasta(20 * MIN);
    let con = |x: &mut Choque, actor: &str, id: &str, c: Comando| {
        let antes = rastro(&x.n);
        let r = x.n.aplicar_en(20 * MIN, Peticion::nueva(actor, id, c));
        assert_eq!(rastro(&x.n), antes, "el rechazo «{id}» dejó rastro");
        codigo(&r)
    };
    // Autorización y existencia.
    assert_eq!(con(&mut x, "beto", "r1", ataque(ObjetivoAtaque::Ejercito(beto.0), ana.1)), rechazos::NO_AUTORIZADO, "el héroe es de otro");
    assert_eq!(con(&mut x, "ana", "r2", ataque(ObjetivoAtaque::Ejercito(beto.0), 9_999)), rechazos::HEROE_DESCONOCIDO);
    assert_eq!(con(&mut x, "ana", "r3", ataque(ObjetivoAtaque::Ejercito(9_999), ana.1)), rechazos::EJERCITO_DESCONOCIDO);
    assert_eq!(con(&mut x, "ana", "r4", ataque(ObjetivoAtaque::Ejercito(ana.0), ana.1)), rechazos::COMBATE_PROPIA, "no se ataca a la propia columna");
    // Alcance: a más de radioEncuentro (3).
    assert_eq!(con(&mut x, "ana", "r5", ataque(ObjetivoAtaque::Ejercito(lejana.0), ana.1)), rechazos::COMBATE_LEJOS);
    // Los míos y los aliados.
    assert_eq!(con(&mut x, "ana", "r6", ataque(ObjetivoAtaque::Ejercito(otra.0), ana.1)), rechazos::COMBATE_SIN_GUERRA, "sin guerra no se ataca a otra Facción");
    assert!(x.n.aplicar_en(20 * MIN, admin("alianza", Comando::FijarRelacion { a: 1, b: 3, relacion: Some(Relacion::Alianza) })).aceptada());
    assert_eq!(con(&mut x, "ana", "r7", Comando::Atacar { heroe: ana.1, objetivo: ObjetivoAtaque::Ejercito(otra.0), declara_guerra: true }), rechazos::COMBATE_ALIADA, "ni con declaración se ataca a un aliado");
    // La clase: un ejército y una columna personal no combaten entre sí (las de administración nacen ejército).
    x.n.aplicar_en(20 * MIN, admin("sale", Comando::Estacionar { ejercito: personal_f2.0 }));
    // Avanzada: todavía no existe ninguna.
    assert_eq!(con(&mut x, "ana", "r8", ataque(ObjetivoAtaque::Avanzada(1), ana.1)), rechazos::COMBATE_AVANZADA_NO_EXISTE);
    // Una caravana, un campamento, una plaza y un pueblo que no existen.
    assert_eq!(con(&mut x, "ana", "r9", ataque(ObjetivoAtaque::Caravana(7), ana.1)), rechazos::CARAVANA_DESCONOCIDA);
    assert_eq!(con(&mut x, "ana", "r10", ataque(ObjetivoAtaque::Campamento(7), ana.1)), rechazos::CAMPAMENTO_DE_BANDIDOS_DESCONOCIDO);
    assert_eq!(con(&mut x, "ana", "r11", ataque(ObjetivoAtaque::Asentamiento(7), ana.1)), rechazos::ASENTAMIENTO_DESCONOCIDO);
    assert_eq!(con(&mut x, "ana", "r12", ataque(ObjetivoAtaque::Pueblo(7), ana.1)), rechazos::PUEBLO_DESCONOCIDO);
    // Una columna apartada por una batalla no se ataca ni ataca.
    assert!(x.n.columna_entra_en_batalla(20 * MIN, beto.0));
    assert_eq!(con(&mut x, "ana", "r13", ataque(ObjetivoAtaque::Ejercito(beto.0), ana.1)), rechazos::COMBATE_A_CUBIERTO);
    assert_eq!(con(&mut x, "beto", "r14", ataque(ObjetivoAtaque::Ejercito(ana.0), beto.1)), rechazos::COLUMNA_EN_BATALLA);
    assert!(x.n.columna_sale_de_batalla(20 * MIN, beto.0));
    // Todo lo que se rechazó dejó la partida como estaba: las tiradas siguen en cero.
    assert_eq!(x.n.estado().rng.estado(), rng_tras(0));
    // Al fin, sí.
    assert!(atacar(&mut x.n, 21 * MIN, "ana", "ok", ana.1, ObjetivoAtaque::Ejercito(beto.0), false).aceptada());
}

#[test]
fn declarar_la_guerra_al_atacar_la_deja_en_guerra_pero_solo_si_la_orden_vale() {
    let mut n = nucleo(40, 5);
    let (ana, beto) = (columna(&mut n, "ana", 1, c(10, 2)), columna(&mut n, "beto", 2, Punto::new(c(10, 2).x + 2_000, c(10, 2).y)));
    dar(&mut n, "d1", ana.1, Tropa::ArquerosCompuesto, completa(Tropa::ArquerosCompuesto));
    assert_eq!(n.relacion(1, 2), None);
    // Una orden que falla por otra cosa no declara nada.
    let lejos = columna(&mut n, "gus", 3, c(30, 2));
    assert_eq!(codigo(&atacar(&mut n, MIN, "ana", "a0", ana.1, ObjetivoAtaque::Ejercito(lejos.0), true)), rechazos::COMBATE_LEJOS);
    assert_eq!(n.relacion(1, 3), None, "no se declaró la guerra");
    // Sin la declaración, no.
    assert_eq!(codigo(&atacar(&mut n, MIN, "ana", "a1", ana.1, ObjetivoAtaque::Ejercito(beto.0), false)), rechazos::COMBATE_SIN_GUERRA);
    // Con ella, ataca y la guerra queda.
    let r = atacar(&mut n, MIN, "ana", "a2", ana.1, ObjetivoAtaque::Ejercito(beto.0), true);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!((n.relacion(1, 2), n.relacion(2, 1)), (Some(Relacion::Guerra), Some(Relacion::Guerra)));
    let g = hechos(&n, "faccion.guerra_declarada");
    assert_eq!((g.len(), dato(&g[0], "atacante"), dato(&g[0], "contra")), (1, 1, 2));
    // La guerra ya existe: los siguientes ataques no la vuelven a declarar.
    assert!(atacar(&mut n, 20 * MIN, "ana", "a3", ana.1, ObjetivoAtaque::Ejercito(beto.0), true).resultado != Resultado::Duplicada);
    assert_eq!(hechos(&n, "faccion.guerra_declarada").len(), 1);
}

#[test]
fn un_encuentro_exacto_de_dos_columnas_enemigas_se_convierte_en_combate_cuando_una_orden_lo_pide() {
    let mut n = nucleo(40, 5);
    let (ana, beto) = (columna(&mut n, "ana", 1, c(0, 2)), columna(&mut n, "beto", 2, c(39, 2)));
    dar(&mut n, "d1", ana.1, Tropa::EspadachinesBronce, completa(Tropa::EspadachinesBronce));
    dar(&mut n, "d2", beto.1, Tropa::MiliciaLanceros, completa(Tropa::MiliciaLanceros));
    assert!(n.aplicar_en(0, comun::marchar("ana", "m1", ana.0, c(39, 2))).aceptada());
    assert!(n.aplicar_en(0, comun::marchar("beto", "m2", beto.0, c(0, 2))).aceptada());
    while hechos(&n, "ejercito.encuentro").is_empty() {
        n.avanzar_hasta(n.ahora() + 1_000);
        assert!(n.ahora() < 30 * MIN, "el encuentro no llegó");
    }
    let e = hechos(&n, "ejercito.encuentro");
    assert_eq!(e.len(), 1, "el encuentro es solo un hecho: nadie ataca por llegar");
    assert_eq!(n.estado().rng.estado(), rng_tras(0));
    // Un segundo después del cruce las dos siguen a ≤ radioEncuentro: la orden lo convierte en combate.
    let t = e[0].0 + 1_000;
    n.avanzar_hasta(t);
    let r = atacar(&mut n, t, "ana", "a1", ana.1, ObjetivoAtaque::Ejercito(beto.0), true);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(hechos(&n, "combate.resuelto").len(), 1);
    assert_eq!(n.estado().rng.estado(), rng_tras(2));
}

// ───────────────────────────── 2. Campamentos de bandidos ─────────────────────────────

fn colocar(n: &mut Nucleo, id: &str, p: Punto, nivel: u8) -> u64 {
    let r = n.aplicar_en(n.ahora(), admin(id, Comando::ColocarBandidos { posicion: p, nivel }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

#[test]
fn atacar_un_campamento_de_bandidos_no_exige_guerra_gasta_una_tirada_y_su_oro_va_al_carro() {
    let mut n = nucleo(40, 5);
    let ana = columna(&mut n, "ana", 1, c(10, 2));
    dar(&mut n, "d1", ana.1, Tropa::ArquerosCompuesto, completa(Tropa::ArquerosCompuesto));
    let cerca = Punto::new(c(10, 2).x + 1_000, c(10, 2).y);
    let b = colocar(&mut n, "b1", cerca, 2);
    assert_eq!(n.campamento_de_bandidos(b).map(|b| (b.nivel, b.posicion)), Some((2, cerca)));
    assert_eq!(n.cosas_en_el_espacio(), 2, "la columna y el campamento, una cosa fija del espacio");
    n.avanzar_hasta(5 * MIN);
    let rastro0 = rastro(&n);
    // El administrador solo; el nivel de 1 a 3.
    assert_eq!(codigo(&n.aplicar_en(5 * MIN, Peticion::nueva("ana", "x1", Comando::ColocarBandidos { posicion: cerca, nivel: 1 }))), rechazos::NO_AUTORIZADO);
    assert_eq!(codigo(&n.aplicar_en(5 * MIN, admin("x2", Comando::ColocarBandidos { posicion: cerca, nivel: 4 }))), rechazos::BANDIDOS_NIVEL_INVALIDO);
    assert_eq!(rastro(&n), rastro0);

    let r = atacar(&mut n, 5 * MIN, "ana", "a1", ana.1, ObjetivoAtaque::Campamento(b), false);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(n.estado().rng.estado(), rng_tras(1), "el ataque a un campamento gasta una tirada");
    let h = &hechos(&n, "combate.campamento_destruido")[0];
    assert_eq!((dato(h, "campamento"), dato(h, "nivel"), dato(h, "heridos")), (b as i64, 2, 0));
    assert!(n.campamento_de_bandidos(b).is_none(), "el campamento cae");
    assert_eq!(n.estado().ejercitos[&ana.0].columna.carga[&Oro], 33_000, "el oro por héroe del nivel 2");
    assert_eq!(dato(h, "oro"), 33_000);
    assert_eq!(n.cosas_en_el_espacio(), 1, "ya no está en el espacio");
    // Ya no existe.
    assert_eq!(codigo(&atacar(&mut n, 6 * MIN, "ana", "a2", ana.1, ObjetivoAtaque::Campamento(b), false)), rechazos::CAMPAMENTO_DE_BANDIDOS_DESCONOCIDO);
}

#[test]
fn un_heroe_solo_ataca_a_los_bandidos_pero_no_basta_para_un_campamento_de_nivel_uno_y_la_derrota_cuesta_heridas_y_la_mitad_del_carro() {
    let mut n = nucleo(40, 5);
    let ana = columna(&mut n, "ana", 1, c(10, 2));
    assert!(n.aplicar_en(0, admin("s", Comando::DarSuministro { ejercito: ana.0, recurso: Piedra, cantidad: 30_001 })).aceptada());
    // 15 de poder con tirada de ±15 %: como mucho 17,25 contra los 20 del nivel 1.
    let b = colocar(&mut n, "b1", Punto::new(c(10, 2).x, c(10, 2).y + 1_000), 1);
    let r = atacar(&mut n, MIN, "ana", "a1", ana.1, ObjetivoAtaque::Campamento(b), false);
    assert!(r.aceptada(), "{r:?}");
    let h = &hechos(&n, "combate.ataque_campamento_fallido")[0];
    assert_eq!((dato(h, "heridos"), dato(h, "carroPerdido"), dato(h, "oro")), (1, 15_000, 0));
    assert!(n.campamento_de_bandidos(b).is_some(), "el campamento aguanta");
    assert!(n.heroe_herido(ana.1));
    assert_eq!(n.estado().ejercitos[&ana.0].columna.carga[&Piedra], 15_001, "pierde la mitad del carro, hacia abajo, y nadie se la queda");
}

#[test]
fn el_oro_de_los_bandidos_cae_con_los_rendimientos_decrecientes_de_cada_heroe() {
    let mut n = nucleo(40, 5);
    let ana = columna(&mut n, "ana", 1, c(10, 2));
    dar(&mut n, "d1", ana.1, Tropa::ArquerosCompuesto, completa(Tropa::ArquerosCompuesto));
    let mut oros = vec![];
    for k in 0..15 {
        let b = colocar(&mut n, &format!("b{k}"), Punto::new(c(10, 2).x + 1_000, c(10, 2).y), 1);
        let antes = n.estado().ejercitos[&ana.0].columna.carga.get(&Oro).copied().unwrap_or(0);
        let r = atacar(&mut n, (k + 1) * MIN, "ana", &format!("a{k}"), ana.1, ObjetivoAtaque::Campamento(b), false);
        assert!(r.aceptada(), "{r:?}");
        oros.push(n.estado().ejercitos[&ana.0].columna.carga.get(&Oro).copied().unwrap_or(0) - antes);
    }
    // Completo las 8 primeras veces en 24 h, luego 15 % menos por cada una más y, desde la 14.ª, solo experiencia.
    assert_eq!(oros, [30_000, 30_000, 30_000, 30_000, 30_000, 30_000, 30_000, 30_000, 25_500, 21_000, 16_500, 12_000, 7_500, 0, 0]);
    // Pasadas 24 h desde las primeras, vuelve a pagar casi completo.
    let b = colocar(&mut n, "bz", Punto::new(c(10, 2).x + 1_000, c(10, 2).y), 1);
    let antes = n.estado().ejercitos[&ana.0].columna.carga[&Oro];
    assert!(atacar(&mut n, 25 * 60 * MIN, "ana", "az", ana.1, ObjetivoAtaque::Campamento(b), false).aceptada());
    assert!(n.estado().ejercitos[&ana.0].columna.carga[&Oro] - antes >= 21_000, "la ventana de 24 h se desliza");
}

#[test]
fn un_campamento_de_bandidos_no_se_ataca_ni_se_coloca_junto_a_otro_combate_prohibido() {
    // Sin mundo no hay campamentos de mercenarios: la protección se prueba en las pruebas del mundo real; aquí, la distancia.
    let mut n = nucleo(40, 5);
    let ana = columna(&mut n, "ana", 1, c(10, 2));
    let lejos = colocar(&mut n, "b1", c(30, 2), 3);
    let r = n.aplicar_en(MIN, Peticion::nueva("ana", "a1", Comando::Atacar { heroe: ana.1, objetivo: ObjetivoAtaque::Campamento(lejos), declara_guerra: false }));
    assert_eq!(codigo(&r), rechazos::COMBATE_LEJOS);
}

// ───────────────────────────── 3. Foto, hash y determinismo ─────────────────────────────

#[test]
fn el_estado_del_combate_va_en_la_foto_y_cualquier_campo_cambia_el_hash() {
    use nucleo::{CampamentoBandidos, GuarnicionDePueblo, OrigenBandidos, Reglas};
    let reglas = Reglas::default();
    let mut base = nucleo::Estado::nuevo(reglas.clone(), "h".into(), 1, 0);
    base.combate.bandidos.insert(1, CampamentoBandidos { id: 1, posicion: Punto::new(10, 20), nivel: 2, origen: OrigenBandidos::Plaza(4), bosque: Some(3) });
    base.combate.bandidos.insert(2, CampamentoBandidos { id: 2, posicion: Punto::new(-10, 20), nivel: 3, origen: OrigenBandidos::Anillo(1), bosque: None });
    base.combate.bandidos.insert(3, CampamentoBandidos { id: 3, posicion: Punto::new(0, 0), nivel: 1, origen: OrigenBandidos::Veta(5), bosque: None });
    base.combate.bandidos.insert(4, CampamentoBandidos { id: 4, posicion: Punto::new(1, 1), nivel: 1, origen: OrigenBandidos::Admin, bosque: None });
    base.combate.siguiente_bandidos = 5;
    base.combate.reaparecen.insert(4, 3_600_000);
    base.combate.anillos.insert(1, 600_000);
    base.combate.guarniciones.insert(9, GuarnicionDePueblo { escuadra: combate::pueblo::guarnicion_de_serie(&mc(), 12, 0), repone_en: Some(7_000) });
    base.combate.destruidos.insert(6, vec![100, 200]);
    let bytes = base.serializar();
    assert_eq!(&bytes[..5], b"BAN12");
    let vuelta = nucleo::Estado::deserializar(&bytes, &reglas, "h").unwrap();
    assert_eq!(vuelta.combate, base.combate);
    assert_eq!(vuelta.serializar(), bytes, "serializar → deserializar → serializar es idéntico");
    let mut vistos = std::collections::HashSet::from([base.hash()]);
    type Cambio = Box<dyn Fn(&mut nucleo::EstadoCombate)>;
    let cambios: Vec<(&str, Cambio)> = vec![
        ("posición del campamento", Box::new(|c| c.bandidos.get_mut(&1).unwrap().posicion = Punto::new(11, 20))),
        ("nivel", Box::new(|c| c.bandidos.get_mut(&1).unwrap().nivel = 3)),
        ("origen", Box::new(|c| c.bandidos.get_mut(&1).unwrap().origen = OrigenBandidos::Plaza(5))),
        ("bosque", Box::new(|c| c.bandidos.get_mut(&1).unwrap().bosque = None)),
        ("un campamento menos", Box::new(|c| drop(c.bandidos.remove(&4)))),
        ("contador de ids", Box::new(|c| c.siguiente_bandidos = 6)),
        ("reaparición de la plaza", Box::new(|c| c.reaparecen.insert(4, 3_600_001).map(drop).unwrap())),
        ("cita del anillo", Box::new(|c| c.anillos.insert(1, 600_001).map(drop).unwrap())),
        ("guarnición del pueblo", Box::new(|c| c.guarniciones.get_mut(&9).unwrap().escuadra.cantidad -= 1)),
        ("reposición de la guarnición", Box::new(|c| c.guarniciones.get_mut(&9).unwrap().repone_en = None)),
        ("héroe que destruyó", Box::new(|c| c.destruidos.get_mut(&6).unwrap().push(300))),
    ];
    for (nombre, f) in cambios {
        let mut s = nucleo::Estado::deserializar(&bytes, &reglas, "h").unwrap();
        f(&mut s.combate);
        assert!(vistos.insert(s.hash()), "el hash no cambió (o coincidió con otro cambio) al tocar {nombre}");
    }
    // Una carga nueva de la cola va y vuelve.
    let mut con_cola = nucleo::Estado::deserializar(&bytes, &reglas, "h").unwrap();
    for (i, s) in [nucleo::SucesoCombate::BandidosDeLaPlaza, nucleo::SucesoCombate::BandidosDelAnillo, nucleo::SucesoCombate::FinProteccion, nucleo::SucesoCombate::FinOcupacion, nucleo::SucesoCombate::ReponeGuarnicion].into_iter().enumerate() {
        con_cola.cola.programar(100 + i as i64, nucleo::PRIO_BANDIDOS, con_cola.generaciones.clave(i as u64 + 1), nucleo::Carga::Combate(s));
    }
    let bytes = con_cola.serializar();
    let otra = nucleo::Estado::deserializar(&bytes, &reglas, "h").unwrap();
    assert_eq!(otra.serializar(), bytes);
    assert_eq!(otra.cola.len(), 5);
    // El formato anterior se rechaza con su motivo.
    let viejo = [b"BAN11".as_slice(), &bytes[5..]].concat();
    let Err(e) = nucleo::Estado::deserializar(&viejo, &reglas, "h") else { panic!("BAN11 no debe admitirse") };
    assert!(e.to_string().contains("BAN11"), "{e}");
}

// ═════════════════════════════ El mundo real ═════════════════════════════

use asentamiento::catalogo::Tipo;
use comercio::caravana::{Animal, EstadoCaravana, TipoCarro};
use militar::escuadra::Contenedor;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{Ajustes, Arranque, Reglas};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

const HORA: Instante = 60 * MIN;

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

fn arranque(m: &M, semilla: u64) -> Arranque {
    Arranque { reglas: Reglas::default(), ajustes: Ajustes::default(), semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

/// El campamento de mercenarios más cercano a `p` (índice del artefacto).
fn campamento_mas_cercano(m: &Mundo, p: Punto) -> u32 {
    (0..m.campamentos().len()).min_by_key(|&i| campamento(m, i).dist2(p)).unwrap() as u32
}

/// Un sitio verde con mucho bosque a 10 u del campamento `i`: una plaza que no cae en ruinas por falta de madera.
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

fn fundar(n: &mut Nucleo, t: Instante, nombre: &str, dueno: &str, faccion: u32, p: Punto) -> u64 {
    let e = n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap();
    assert!(n.aplicar_en(t, admin(&format!("c-{nombre}"), Comando::DarCaravanaFundacion { ejercito: e })).aceptada());
    let r = n.aplicar_en(t, Peticion::nueva(dueno, &format!("f-{nombre}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

/// Dos plazas de Facciones distintas (F1 con «ana» junto al campamento 0, F2 con «beto» junto al 3). El tiempo corre solo cuando se pide.
struct Esc {
    n: Nucleo,
    a: u64,
    b: u64,
    cuenta: u64,
    /// El instante de las órdenes que se dan, si no es el de ahora (los guiones dan sus órdenes en instantes fijos, sea cual sea el modo de avanzar).
    fijo: Option<Instante>,
}

impl Esc {
    fn t(&self) -> Instante {
        self.fijo.unwrap_or(self.n.ahora())
    }
    fn peticion(&mut self, actor: &str, c: Comando) -> Peticion {
        self.cuenta += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.cuenta), c);
        if actor == "admin" {
            p.de_admin()
        } else {
            p
        }
    }
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        let (t, p) = (self.t(), self.peticion(actor, c));
        self.n.aplicar_en(t, p)
    }
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    fn rechazada(&mut self, actor: &str, c: Comando) -> &'static str {
        let r = self.ord(actor, c);
        codigo(&r)
    }
    /// Da las órdenes que siguen en el instante `t`, sin mover el reloj antes.
    fn en(&mut self, t: Instante) -> &mut Self {
        self.fijo = Some(t);
        self
    }
    fn avanzar(&mut self, ms: i64) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    fn pos(&self, plaza: u64) -> Punto {
        self.n.estado().asentamientos[&plaza].posicion
    }
    fn dar(&mut self, plaza: u64, recurso: Recurso, cantidad: i64) {
        let r = self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
    }
    /// Un héroe de la cuenta, ciudadano de `faccion`, que reside en la plaza y está dentro. Nace en el campamento más cercano.
    fn heroe_en(&mut self, m: &Mundo, cuenta: &str, faccion: u32, plaza: u64) -> u64 {
        let campamento = campamento_mas_cercano(m, self.pos(plaza));
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento, controlador: Controlador::Humano }).creadas[0];
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(faccion) });
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
        h
    }
    /// Una escuadra completa de esa tropa para el héroe, en su campamento, y su loadout activo (el «Default») la lleva: defiende en persona.
    fn defensor(&mut self, heroe: u64, tropas: &[Tropa]) -> Vec<u64> {
        let ids: Vec<u64> = tropas.iter().map(|&t| self.ok("admin", Comando::DarEscuadra { heroe, tropa: t, cantidad: completa(t) }).creadas[0]).collect();
        let loadout = self.n.heroe(heroe).unwrap().heroe.loadouts[0].id;
        self.ok("admin", Comando::GuardarLoadout { heroe, loadout: Some(loadout), nombre: "Default".into(), escuadras: ids.clone(), perks: vec![], activo: Some(true) });
        ids
    }
    /// Un ejército de administración de `cuenta`/`faccion` a 2 u de `p`, con las tropas dadas.
    fn ejercito(&mut self, cuenta: &str, faccion: u32, p: Punto, tropas: &[Tropa]) -> (u64, u64) {
        let t = self.t();
        let (col, h) = columna_en(&mut self.n, t, cuenta, faccion, Punto::new(p.x + 2_000, p.y));
        for &t in tropas {
            self.ok("admin", Comando::DarEscuadra { heroe: h, tropa: t, cantidad: completa(t) });
        }
        (col, h)
    }
    fn atacar(&mut self, actor: &str, heroe: u64, objetivo: ObjetivoAtaque, guerra: bool) -> Respuesta {
        let t = self.t();
        self.cuenta += 1;
        atacar(&mut self.n, t, actor, &format!("o{}", self.cuenta), heroe, objetivo, guerra)
    }
}

fn montar(m: &M, semilla: u64, con_mercado: bool) -> Esc {
    montar_sobre(Nucleo::nuevo(arranque(m, semilla)), m, con_mercado)
}

fn montar_sobre(mut n: Nucleo, m: &M, con_mercado: bool) -> Esc {
    let (pa, pb) = (sitio_rico(&n, &m.0, 0, 1), sitio_rico(&n, &m.0, 3, 2));
    let (a, b) = (fundar(&mut n, 0, "a", "ana", 1, pa), fundar(&mut n, 0, "b", "beto", 2, pb));
    let mut e = Esc { n, a, b, cuenta: 0, fijo: None };
    if con_mercado {
        for plaza in [a, b] {
            e.dar(plaza, Madera, 350_000);
            e.ok("admin", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
        }
        e.avanzar(9 * HORA);
        for plaza in [a, b] {
            assert!(e.n.economia(plaza).unwrap().edificios.iter().any(|x| x.tipo == Tipo::Mercado && x.activo()), "el Mercado de as-{plaza} debería estar activo");
        }
    } else {
        e.avanzar(2 * HORA);
    }
    e
}

fn almacen(n: &Nucleo, plaza: u64, r: Recurso) -> i64 {
    n.economia(plaza).unwrap().almacen.cantidad(r)
}

// ───────────────────────────── 4. Asedio y conquista ─────────────────────────────

/// La plaza B con su dueño «beto» dentro, que la defiende en persona con su loadout, y un ejército de «ana» a su puerta.
struct Asedio {
    e: Esc,
    beto: u64,
    ana: (u64, u64),
}

fn asedio(m: &'static M, semilla: u64, defensa: &[Tropa], ataque: &[Tropa]) -> Asedio {
    let mut e = montar(m, semilla, false);
    let b = e.b;
    let beto = e.heroe_en(&m.0, "beto", 2, b);
    if !defensa.is_empty() {
        e.defensor(beto, defensa);
    }
    let ana = e.ejercito("ana", 1, e.pos(b), ataque);
    Asedio { e, beto, ana }
}

#[test]
fn un_asedio_que_gana_conquista_la_plaza_la_deja_protegida_y_ocupada_y_desaloja_a_los_defensores() {
    let m = con_mundo!();
    let mut s = asedio(m, 7, &[Tropa::MiliciaLanceros], &[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto]);
    let (b, beto, ana) = (s.e.b, s.beto, s.ana);
    let vieja_zona = s.e.n.radio_zona(b).unwrap();
    let poblacion = s.e.n.economia(b).unwrap().poblacion.clone();
    let edificios_activos = s.e.n.economia(b).unwrap().edificios.iter().filter(|x| x.activo()).count();
    let t = s.e.n.ahora();
    let (v0, rastro0) = (s.e.n.version(), rastro(&s.e.n));

    // Sin guerra no se asedia; con la declaración, sí.
    assert_eq!(codigo(&s.e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b), false)), rechazos::COMBATE_SIN_GUERRA);
    assert_eq!(rastro(&s.e.n), rastro0, "el rechazo no dejó rastro");
    let rng_antes = s.e.n.estado().rng.estado();
    let r = s.e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b), true);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(s.e.n.version(), v0 + 1);
    let mut espejo = Rng::nuevo(rng_antes);
    (0..2).for_each(|_| {
        espejo.siguiente();
    });
    assert_eq!(s.e.n.estado().rng.estado(), espejo.estado(), "un asedio con defensores gasta dos tiradas");

    // La ciudad cambia de dueño y se saquea (Combate §8).
    let a = &s.e.n.estado().asentamientos[&b];
    assert_eq!(a.faccion, 1);
    assert_eq!((a.protegida_hasta, a.ocupada_hasta), (Some(t + 24 * HORA), Some(t + 90 * MIN)));
    assert!(a.economia.ocupado && s.e.n.plaza_protegida(b));
    assert!(a.economia.poblacion.pesants <= poblacion.pesants * 3 / 4 + 1_000 && a.economia.poblacion.pesants % 1_000 == 0, "pierde el 25 % y queda en personas enteras");
    assert!(a.economia.edificios.iter().any(|x| x.danado), "algún edificio queda dañado");
    assert!(a.economia.edificios.iter().filter(|x| x.activo()).count() < edificios_activos);
    assert_eq!(a.radio, vieja_zona, "la zona no cambia");
    // El hecho del asedio y el de la conquista.
    let h = &hechos(&s.e.n, "combate.asedio_conquista")[0];
    assert_eq!((h.1.clone(), dato(h, "faccionAtacante"), dato(h, "faccionDefensora")), (vec![ana.0], 1, 2));
    let c = &hechos(&s.e.n, "asentamiento.conquistada")[0];
    assert_eq!((c.2.clone(), dato(c, "faccion"), dato(c, "faccionAnterior"), dato(c, "ocupadaHasta")), (vec![b], 1, 2, t + 90 * MIN));
    assert_eq!(hechos(&s.e.n, "faccion.derrotada").iter().map(|f| dato(f, "faccion")).collect::<Vec<_>>(), vec![2], "era la única plaza de la Facción 2");
    // El defensor herido (defendió en persona y la plaza cayó), desalojado: reside en el campamento más cercano, nace una columna personal con lo que le
    // sobrevivió y la Facción de la plaza, a la puerta, y su campamento queda a 0.
    let h = s.e.n.heroe(beto).unwrap();
    assert!(s.e.n.heroe_herido(beto));
    assert_eq!(h.heroe.residencia, Some(Residencia::Campamento(campamento_mas_cercano(&m.0, s.e.pos(b)))));
    let Ubicacion::Columna(salida) = h.heroe.presencia.ubicacion else { panic!("sale a la calle: {:?}", h.heroe.presencia.ubicacion) };
    let col = s.e.n.ejercito(salida).unwrap();
    assert_eq!((col.columna.faccion, col.columna.participantes.len(), col.posicion), (Some(2), 1, s.e.pos(b)));
    assert_eq!(col.columna.escuadras.len(), 1);
    assert!(s.e.n.soldados_de_columna(salida).unwrap() > 0 && col.columna.carga.is_empty());
    assert_eq!(hechos(&s.e.n, "heroe.desalojado").len(), 1);
    // Los plazos son eventos: la ocupación acaba a la hora y media y la protección a las 24 h.
    assert!(s.e.n.plaza_protegida(b));
    s.e.n.avanzar_hasta(t + 90 * MIN - 1);
    assert!(s.e.n.estado().asentamientos[&b].economia.ocupado);
    s.e.n.avanzar_hasta(t + 90 * MIN);
    let a = &s.e.n.estado().asentamientos[&b];
    assert!(!a.economia.ocupado && a.ocupada_hasta.is_none() && a.protegida_hasta.is_some());
    assert_eq!(hechos(&s.e.n, "asentamiento.ocupacion_terminada")[0].0, t + 90 * MIN);
    // Mientras está protegida, un tercero no la asedia (aunque esté en guerra): ni deja rastro ni gasta una tirada.
    let gus = s.e.ejercito("gus", 3, s.e.pos(b), &[Tropa::EspadachinesBronce]);
    s.e.ok("admin", Comando::FijarRelacion { a: 1, b: 3, relacion: Some(Relacion::Guerra) });
    let rastro1 = rastro(&s.e.n);
    assert_eq!(codigo(&s.e.atacar("gus", gus.1, ObjetivoAtaque::Asentamiento(b), false)), rechazos::COMBATE_PLAZA_PROTEGIDA);
    assert_eq!(rastro(&s.e.n), rastro1);
    s.e.n.avanzar_hasta(t + 24 * HORA - 1);
    assert!(s.e.n.plaza_protegida(b));
    s.e.n.avanzar_hasta(t + 24 * HORA);
    assert!(!s.e.n.plaza_protegida(b) && s.e.n.estado().asentamientos[&b].protegida_hasta.is_none());
    assert_eq!(hechos(&s.e.n, "asentamiento.proteccion_terminada")[0].0, t + 24 * HORA);
    // Pasadas las 24 h, sí: y la propia plaza de ana no se asedia.
    assert_eq!(codigo(&s.e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b), false)), rechazos::COMBATE_PLAZA_PROPIA);
}

#[test]
fn un_asedio_que_pierde_hiere_a_todo_el_ejercito_y_la_chatarra_se_queda_en_la_plaza() {
    let m = con_mundo!();
    let mut s = asedio(m, 7, &[Tropa::ArquerosCompuesto], &[Tropa::EspadachinesBronce]);
    let (b, beto, ana) = (s.e.b, s.beto, s.ana);
    // Un segundo héroe en el ejército de ana: también queda herido, hayan entrado en la batalla o no.
    let segundo = {
        let campamento = campamento_mas_cercano(&m.0, s.e.pos(b));
        let h = s.e.ok("carla", Comando::CrearHeroe { cuenta: "carla".into(), campamento, controlador: Controlador::Humano }).creadas[0];
        s.e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
        s.e.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Columna(ana.0) });
        h
    };
    let metal0: i64 = [ChatarraCobre, ChatarraBronce, ChatarraHierro].iter().map(|&r| almacen(&s.e.n, b, r)).sum();
    let r = s.e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b), true);
    assert!(r.aceptada(), "{r:?}");
    let h = &hechos(&s.e.n, "combate.asedio_resistido")[0];
    assert!(dato(h, "poderDefensor") > dato(h, "poderAtacante"), "{h:?}");
    assert_eq!(h.2, vec![b], "se cuenta a la plaza si resiste");
    let a = &s.e.n.estado().asentamientos[&b];
    assert_eq!((a.faccion, a.protegida_hasta, a.ocupada_hasta, a.economia.ocupado), (2, None, None, false), "la plaza no cae");
    assert!(s.e.n.heroe_herido(ana.1) && s.e.n.heroe_herido(segundo), "todos los participantes del ejército quedan heridos");
    assert!(!s.e.n.heroe_herido(beto), "los defensores no");
    assert_eq!(s.e.n.heroe(beto).unwrap().heroe.presencia.ubicacion, Ubicacion::Asentamiento(b as u32));
    // La chatarra de las bajas de los dos bandos va al almacén de la plaza que resiste.
    let metal: i64 = [ChatarraCobre, ChatarraBronce, ChatarraHierro].iter().map(|&r| almacen(&s.e.n, b, r)).sum();
    assert_eq!(metal - metal0, dato(h, "chatarra"));
    assert!(dato(h, "chatarra") > 0, "los espadachines de bronce que caen dejan chatarra");
    assert_eq!(s.e.n.estado().ejercitos[&ana.0].columna.carga.len(), 0);
}

#[test]
fn una_plaza_sin_defensores_con_soldados_cae_sin_combate_ni_tirada_y_sin_heridos() {
    let m = con_mundo!();
    let mut s = asedio(m, 7, &[], &[Tropa::EspadachinesBronce]);
    let (b, beto, ana) = (s.e.b, s.beto, s.ana);
    // Un héroe sin escuadras no basta para defender; uno sin tropa no abre un asedio.
    let sin_soldados = s.e.ejercito("ivan", 1, s.e.pos(b), &[]);
    assert_eq!(codigo(&s.e.atacar("ivan", sin_soldados.1, ObjetivoAtaque::Asentamiento(b), true)), rechazos::COMBATE_SIN_ATACANTES);
    let rng0 = s.e.n.estado().rng.estado();
    assert!(s.e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b), true).aceptada());
    assert_eq!(s.e.n.estado().rng.estado(), rng0, "sin defensores no hay tirada");
    assert_eq!(s.e.n.estado().asentamientos[&b].faccion, 1);
    assert!(!s.e.n.heroe_herido(beto), "sin combate no hiere a nadie");
    assert!(!s.e.n.heroe_herido(ana.1));
    // Beto estaba dentro y sin columna aparcada: sale a la calle sin escuadras.
    assert!(matches!(s.e.n.heroe(beto).unwrap().heroe.presencia.ubicacion, Ubicacion::Columna(_)));
}

// ───────────────────────────── 5. Caravanas: aguante (B) y disputa de carros (A) ─────────────────────────────

impl Esc {
    /// Una caravana de la plaza con `carros` carros con buey, fuera del reparto automático.
    fn caravana(&mut self, actor: &str, plaza: u64, carros: u32) -> u64 {
        let id = self.ok(actor, Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
        self.ok(actor, Comando::ReservarCaravana { caravana: id, reservada: true });
        for k in 0..carros {
            self.ok(actor, Comando::AnadirCarro { caravana: id, tipo: TipoCarro::Basico });
            self.ok(actor, Comando::AnadirAnimal { caravana: id, carro: k, animal: Animal::Buey });
        }
        id
    }
    /// Prepara la caravana de A hacia B con 100 de cobre y avanza hasta que sale. Devuelve el instante de salida.
    fn lanzar(&mut self, caravana: u64) -> Instante {
        let (a, b) = (self.a, self.b);
        self.dar(a, Cobre, 300_000);
        self.ok("ana", Comando::PrepararCaravana { caravana, destino: b, carga: vec![(Cobre, 100_000)], salir_en: None });
        for _ in 0..600 {
            if self.n.caravana(caravana).unwrap().estado == EstadoCaravana::EnTransito {
                return self.n.caravana(caravana).unwrap().plan.as_ref().unwrap().inicio().unwrap();
            }
            self.avanzar(MIN);
        }
        panic!("la caravana no salió");
    }
    fn carga_de_columna(&self, columna: u64, r: Recurso) -> i64 {
        self.n.ejercito(columna).unwrap().columna.carga.get(&r).copied().unwrap_or(0)
    }
}

#[test]
fn una_caravana_atacada_dentro_de_su_zona_es_un_aguante_pierde_el_70_por_ciento_y_la_mitad_de_los_carros_y_su_escolta_vuelve_a_casa_a_cero() {
    let m = con_mundo!();
    let mut e = montar(m, 7, true);
    let a = e.a;
    // Dos carros y una escolta de milicia cedida por un héroe residente.
    let ca = e.caravana("ana", a, 2);
    let carla = e.heroe_en(&m.0, "carla", 1, a);
    let milicia = e.ok("admin", Comando::DarEscuadra { heroe: carla, tropa: Tropa::MiliciaLanceros, cantidad: 25 }).creadas[0];
    e.ok("carla", Comando::AsignarEscolta { caravana: ca, heroe: carla, escuadras: vec![milicia] });
    let t0 = e.lanzar(ca);
    e.n.avanzar_hasta(t0);
    // En el instante de salir está en la puerta de su plaza: zona del dueño.
    assert_eq!(e.n.zona_de_caravana(ca, t0), Some(combate::caravana::Zona::DelDuenoOAliado));
    let atacante = e.ejercito("beto", 2, e.pos(a), &[Tropa::ArquerosCompuesto, Tropa::EspadachinesBronce]);
    let (v0, rng0) = (e.n.version(), e.n.estado().rng.estado());
    let r = e.atacar("beto", atacante.1, ObjetivoAtaque::Caravana(ca), true);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(e.n.version(), v0 + 1);
    let mut espejo = Rng::nuevo(rng0);
    espejo.siguiente();
    assert_eq!(e.n.estado().rng.estado(), espejo.estado(), "el asalto a una caravana gasta una tirada");
    let h = &hechos(&e.n, "combate.caravana_interceptada")[0];
    assert_eq!((dato(h, "caravana"), dato(h, "aguante"), dato(h, "carros"), dato(h, "destruida")), (ca as i64, 1, 1, 0));
    assert!(dato(h, "poderAtacante") > dato(h, "defensa"), "la escolta de milicia, con la ventaja del aguante, no basta");
    assert_eq!(h.2, vec![a]);
    // El dueño pierde la mitad de los carros y el 70 % de la carga; el 30 % se salva; el atacante se lleva el 70 %.
    let k = e.n.caravana(ca).unwrap();
    assert_eq!((k.carros.len(), k.contenido[&Cobre]), (1, 30_000));
    assert_eq!(e.carga_de_columna(atacante.0, Cobre), 70_000);
    assert_eq!(dato(h, "botin"), 70_000);
    // La escolta cae: queda a 0 y vuelve al campamento de su héroe, y la caravana ya no la lleva.
    let s = e.n.heroe(carla).unwrap().heroe.escuadra(milicia).unwrap();
    assert_eq!((s.cantidad, s.contenedor), (0, Contenedor::Campamento));
    assert!(k.escolta.is_empty());
    // El atacante gana: no queda herido.
    assert!(!e.n.heroe_herido(atacante.1));
}

#[test]
fn una_caravana_atacada_fuera_de_toda_zona_es_una_disputa_de_carros_y_sin_carros_que_quedar_se_destruye() {
    let m = con_mundo!();
    let mut e = montar(m, 7, true);
    let (a, b) = (e.a, e.b);
    let ca = e.caravana("ana", a, 2);
    let t0 = e.lanzar(ca);
    let fin = e.n.caravana(ca).unwrap().plan.as_ref().unwrap().fin().unwrap();
    // A mitad de camino, lejos de las dos plazas, no hay zona de nadie.
    e.n.avanzar_hasta(t0 + (fin - t0) / 2);
    let t = e.n.ahora();
    let donde = e.n.posicion_de_caravana(ca, t).unwrap();
    assert_eq!(e.n.zona_de_caravana(ca, t), Some(combate::caravana::Zona::Ninguna), "a mitad de camino no hay zona de nadie");
    let atacante = e.ejercito("beto", 2, donde, &[Tropa::ArquerosCompuesto]);
    let r = e.atacar("beto", atacante.1, ObjetivoAtaque::Caravana(ca), true);
    assert!(r.aceptada(), "{r:?}");
    let h = &hechos(&e.n, "combate.caravana_interceptada")[0];
    assert_eq!((dato(h, "aguante"), dato(h, "carros"), dato(h, "destruida")), (0, 1, 0));
    let k = e.n.caravana(ca).unwrap();
    assert_eq!(k.carros.len(), 1, "pierde la mitad de los carros, con su animal");
    assert_eq!(k.contenido[&Cobre], 50_000, "y la carga que llevaba ese carro");
    assert_eq!(e.carga_de_columna(atacante.0, Cobre), 50_000);
    // Una caravana de un solo carro lo pierde entero: desaparece (sin aguante no se salva nada) con un hecho de pérdida.
    let cb = e.caravana("ana", a, 1);
    e.dar(a, Cobre, 100_000);
    e.ok("ana", Comando::PrepararCaravana { caravana: cb, destino: b, carga: vec![(Cobre, 50_000)], salir_en: None });
    let t1 = loop {
        e.avanzar(MIN);
        if e.n.caravana(cb).unwrap().estado == EstadoCaravana::EnTransito {
            break e.n.ahora();
        }
    };
    let fin = e.n.caravana(cb).unwrap().plan.as_ref().unwrap().fin().unwrap();
    e.n.avanzar_hasta(t1 + (fin - t1) / 2);
    let donde = e.n.posicion_de_caravana(cb, e.n.ahora()).unwrap();
    let otro = e.ejercito("beto", 2, donde, &[Tropa::ArquerosCompuesto]);
    assert!(e.atacar("beto", otro.1, ObjetivoAtaque::Caravana(cb), false).aceptada(), "ya hay guerra");
    assert!(e.n.caravana(cb).is_none());
    assert!(hechos(&e.n, "comercio.caravana_perdida").iter().any(|h| dato(h, "caravana") == cb as i64));
    assert_eq!(e.carga_de_columna(otro.0, Cobre), 50_000);
}

#[test]
fn si_la_caravana_resiste_el_atacante_pierde_el_25_por_ciento_y_sus_heroes_quedan_heridos_y_una_adjunta_o_parada_no_se_ataca() {
    let m = con_mundo!();
    let mut e = montar(m, 7, true);
    let a = e.a;
    let ca = e.caravana("ana", a, 2);
    // Parada en su plaza no es presa: no está en el mapa.
    let solo = e.ejercito("beto", 2, e.pos(a), &[]);
    assert_eq!(codigo(&e.atacar("beto", solo.1, ObjetivoAtaque::Caravana(ca), true)), rechazos::COMBATE_A_CUBIERTO);
    let t0 = e.lanzar(ca);
    e.n.avanzar_hasta(t0);
    // Un héroe solo (15 de poder, 17,25 como mucho) no vence a los carreteros (26).
    let rastro0 = rastro(&e.n);
    assert_eq!(codigo(&e.atacar("beto", solo.1, ObjetivoAtaque::Caravana(ca), false)), rechazos::COMBATE_SIN_GUERRA);
    assert_eq!(rastro(&e.n), rastro0, "sin guerra no se ataca y no deja rastro");
    let r = e.atacar("beto", solo.1, ObjetivoAtaque::Caravana(ca), true);
    assert!(r.aceptada(), "{r:?}");
    let h = &hechos(&e.n, "combate.caravana_resiste")[0];
    assert_eq!((dato(h, "carros"), dato(h, "heridos"), dato(h, "botin")), (0, 1, 0));
    assert!(dato(h, "defensa") >= 26_000 * 125 / 100 || dato(h, "aguante") == 0 || dato(h, "defensa") == 32_500, "la defensa base es 26 y con aguante 32,5: {h:?}");
    assert!(e.n.heroe_herido(solo.1));
    let k = e.n.caravana(ca).unwrap();
    assert_eq!((k.carros.len(), k.contenido[&Cobre]), (2, 100_000), "no se pierde nada");
    // Una adjunta a un ejército se ataca al ejército.
    e.avanzar(2 * HORA);
    let cb = e.caravana("ana", a, 1);
    let col = e.ejercito("ana", 1, e.pos(a), &[Tropa::MiliciaLanceros]);
    e.ok("admin", Comando::AdjuntarCaravana { caravana: cb, ejercito: col.0 });
    let tercero = e.ejercito("gus", 3, e.pos(a), &[Tropa::MiliciaLanceros]);
    assert_eq!(codigo(&e.atacar("gus", tercero.1, ObjetivoAtaque::Caravana(cb), true)), rechazos::COMBATE_CARAVANA_ADJUNTA);
}

// ───────────────────────────── 6. Pueblos extractores ─────────────────────────────

use asentamiento::pueblos::EstadoPueblo;

/// Un sitio fundable, verde y con mucho bosque, a `r` del punto `c`.
fn sitio_junto_a(n: &Nucleo, c: Punto, r: i64) -> Option<Punto> {
    (0..36).map(|g| ((g * 10) as f64).to_radians()).map(|a| Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64)).find(|&p| {
        n.viabilidad(p, 1).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6 && !v.condenada)
    })
}

/// La plaza madre de «ana» (F1, nivel 2) con un pueblo de cobre levantado y activo, y su carro. `Esc.b` no se usa.
fn con_pueblo(m: &M, semilla: u64) -> (Esc, u64) {
    let mut n = Nucleo::nuevo(arranque(m, semilla));
    let mapa = nucleo::economia::Mapa::de(&m.0);
    let (cobre, sitio) = (0..mapa.nodos.len())
        .filter(|&i| mapa.nodos[i].tipo == Tipo::MinaCobre)
        .find_map(|i| sitio_junto_a(&n, mapa.nodos[i].pos, 6_000).map(|p| (i as u32, p)))
        .expect("hay un nodo de cobre junto a un sitio rico");
    let a = fundar(&mut n, 0, "m", "ana", 1, sitio);
    let mut e = Esc { n, a, b: 0, cuenta: 0, fijo: None };
    e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel: 2 });
    for _ in 0..9 {
        e.ok("admin", Comando::AgrandarZona { asentamiento: a });
    }
    let pueblo = e.ok("ana", Comando::MarcarPueblo { asentamiento: a, nodo: cobre }).creadas[0];
    let sitio = e.n.posicion_de_pueblo(pueblo).unwrap();
    let col = e.n.crear_ejercito(0, "ana", 1, sitio, 500, 100_000).unwrap();
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 100_000 });
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 12_000 });
    e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo });
    let EstadoPueblo::EnObra { completa_en } = e.n.pueblo(pueblo).unwrap().1.estado else { panic!("debía estar en obra") };
    e.n.avanzar_hasta(completa_en + MIN);
    assert!(e.n.pueblo(pueblo).unwrap().1.activo());
    (e, pueblo)
}

#[test]
fn saquear_un_pueblo_se_lleva_lo_que_hay_por_encima_de_la_mitad_y_la_guarnicion_de_serie_se_repone_a_las_6_horas() {
    let m = con_mundo!();
    let (mut e, pueblo) = con_pueblo(m, 7);
    let madre = e.a;
    e.avanzar(HORA);
    let sitio = e.n.posicion_de_pueblo(pueblo).unwrap();
    let guardado = e.n.pueblo(pueblo).unwrap().1.almacen.cantidad(Cobre);
    assert!(guardado > 250_000 && guardado < 500_000, "tras una hora hay más de la mitad: {guardado}");
    let saqueador = e.ejercito("beto", 2, sitio, &[Tropa::ArquerosCompuesto, Tropa::EspadachinesBronce]);
    let solo = e.ejercito("gus", 3, sitio, &[]);
    assert_eq!(e.n.estado().combate.guarniciones.len(), 0, "la guarnición de serie nace la primera vez que se la necesita");
    // Un héroe sin soldados no saquea; sin guerra tampoco; el pueblo propio, menos.
    assert_eq!(codigo(&e.atacar("gus", solo.1, ObjetivoAtaque::Pueblo(pueblo), true)), rechazos::COMBATE_SIN_ATACANTES);
    let rastro0 = rastro(&e.n);
    assert_eq!(codigo(&e.atacar("beto", saqueador.1, ObjetivoAtaque::Pueblo(pueblo), false)), rechazos::COMBATE_SIN_GUERRA);
    assert_eq!(codigo(&e.atacar("beto", saqueador.1, ObjetivoAtaque::Pueblo(9_999), true)), rechazos::PUEBLO_DESCONOCIDO);
    assert_eq!(rastro(&e.n), rastro0, "los rechazos no dejan rastro (ni la guarnición de serie)");
    let rng0 = e.n.estado().rng.estado();
    let t = e.n.ahora();
    let r = e.atacar("beto", saqueador.1, ObjetivoAtaque::Pueblo(pueblo), true);
    assert!(r.aceptada(), "{r:?}");
    let mut espejo = Rng::nuevo(rng0);
    (0..2).for_each(|_| {
        espejo.siguiente();
    });
    assert_eq!(e.n.estado().rng.estado(), espejo.estado(), "un saqueo con guarnición gasta dos tiradas");
    // El atacante se lleva solo lo que está por encima de la mitad; el pueblo conserva la mitad protegida; no cambia de dueño.
    let h = &hechos(&e.n, "combate.pueblo_saqueado")[0];
    assert_eq!((dato(h, "pueblo"), h.2.clone()), (pueblo as i64, vec![madre]));
    assert_eq!(dato(h, "saqueado"), guardado - 250_000);
    assert_eq!(e.n.pueblo(pueblo).unwrap().1.almacen.cantidad(Cobre), 250_000);
    assert_eq!(e.carga_de_columna(saqueador.0, Cobre), guardado - 250_000);
    assert_eq!(e.n.estado().asentamientos[&madre].faccion, 1);
    // Los mineros: caen entre un 30 % y un 70 % y se reponen a las 6 horas, sin que nada recurra mientras tanto.
    let g = &e.n.estado().combate.guarniciones[&pueblo];
    assert!((1..30).contains(&g.escuadra.cantidad), "quedan mineros: {}", g.escuadra.cantidad);
    assert_eq!(g.repone_en, Some(t + 6 * HORA));
    e.n.avanzar_hasta(t + 6 * HORA - 1);
    assert!(e.n.estado().combate.guarniciones[&pueblo].escuadra.cantidad < 30);
    e.n.avanzar_hasta(t + 6 * HORA);
    let g = &e.n.estado().combate.guarniciones[&pueblo];
    assert_eq!((g.escuadra.cantidad, g.repone_en), (30, None));
    assert!(g.escuadra.experiencia > 0, "conserva la experiencia que ganó");
    assert_eq!(hechos(&e.n, "pueblo.guarnicion_repuesta")[0].0, t + 6 * HORA);
}

#[test]
fn un_heroe_solo_pierde_contra_los_mineros_y_sin_ellos_el_pueblo_cae_sin_combate() {
    let m = con_mundo!();
    let (mut e, pueblo) = con_pueblo(m, 9);
    e.avanzar(HORA);
    let sitio = e.n.posicion_de_pueblo(pueblo).unwrap();
    let guardado = e.n.pueblo(pueblo).unwrap().1.almacen.cantidad(Cobre);
    // Los mineros (30 de poder, con la ventaja del aguante) ganan siempre a un héroe con una milicia mínima... y a uno solo, sin que se pueda ni intentar.
    let debil = e.ejercito("gus", 3, sitio, &[]);
    e.ok("admin", Comando::DarEscuadra { heroe: debil.1, tropa: Tropa::Lenadores, cantidad: 3 });
    let r = e.atacar("gus", debil.1, ObjetivoAtaque::Pueblo(pueblo), true);
    assert!(r.aceptada(), "{r:?}");
    let h = &hechos(&e.n, "combate.pueblo_resiste")[0];
    assert!(dato(h, "poderDefensor") > dato(h, "poderAtacante"), "{h:?}");
    assert_eq!((dato(h, "saqueado"), dato(h, "heridos")), (0, 1));
    assert!(e.n.heroe_herido(debil.1));
    assert_eq!(e.n.pueblo(pueblo).unwrap().1.almacen.cantidad(Cobre), guardado, "no se llevó nada");
    // Un ejército fuerte los va deshaciendo; con la guarnición a 0 esperando su reposición, el pueblo se saquea sin combate ni tirada.
    let fuerte = e.ejercito("beto", 2, sitio, &[Tropa::ArquerosCompuesto, Tropa::EspadachinesBronce, Tropa::Hequetai]);
    for _ in 0..30 {
        if e.n.estado().combate.guarniciones[&pueblo].escuadra.cantidad == 0 {
            break;
        }
        assert!(e.atacar("beto", fuerte.1, ObjetivoAtaque::Pueblo(pueblo), true).aceptada());
    }
    assert_eq!(e.n.estado().combate.guarniciones[&pueblo].escuadra.cantidad, 0, "los mineros acaban cayendo");
    let (rng0, saqueos) = (e.n.estado().rng.estado(), hechos(&e.n, "combate.pueblo_saqueado").len());
    assert!(e.atacar("beto", fuerte.1, ObjetivoAtaque::Pueblo(pueblo), true).aceptada());
    assert_eq!(e.n.estado().rng.estado(), rng0, "sin defensores no hay tirada");
    assert_eq!(hechos(&e.n, "combate.pueblo_saqueado").len(), saqueos + 1);
}

// ───────────────────────────── 7. Bandidos en el espacio ─────────────────────────────

fn de_combate_pendientes(n: &Nucleo) -> Vec<(Instante, nucleo::SucesoCombate)> {
    let e = n.estado();
    e.cola.volcar().into_iter().filter(|x| e.generaciones.es_vigente(x.clave)).filter_map(|x| if let nucleo::Carga::Combate(s) = x.carga { Some((x.instante, s)) } else { None }).collect()
}

#[test]
fn la_partida_nace_con_los_campamentos_de_las_vetas_y_uno_en_el_anillo_de_cada_campamento_de_mercenarios() {
    let m = con_mundo!();
    let mut n = Nucleo::nuevo(arranque(m, 11));
    let del_mundo = m.0.bandidos_iniciales();
    assert!(!del_mundo.is_empty());
    assert_eq!(n.campamentos_de_bandidos().count(), del_mundo.len(), "los de las vetas nacen con la partida");
    for (i, b) in del_mundo.iter().enumerate() {
        let p = b.posicion.as_ref().unwrap();
        assert!(n.campamentos_de_bandidos().any(|c| c.posicion == Punto::new(p.x, p.y) && c.origen == nucleo::OrigenBandidos::Veta(i as u32) && c.nivel as u32 == b.nivel.clamp(1, 3)));
    }
    assert_eq!(de_combate_pendientes(&n).len(), m.0.campamentos().len(), "una cita por anillo, y nada más");
    n.avanzar_hasta(1);
    // Un campamento de bandidos en el anillo de cada campamento de mercenarios (30 a 50 u), en tierra firme y fuera de toda zona.
    for c in 0..m.0.campamentos().len() {
        let suyos: Vec<_> = n.campamentos_de_bandidos().filter(|b| b.origen == nucleo::OrigenBandidos::Anillo(c as u32)).collect();
        assert_eq!(suyos.len(), 1, "sin residentes sin plaza, el mínimo es uno (campamento {c})");
        let d = suyos[0].posicion.dist(campamento(&m.0, c));
        assert!((30_000..=50_000).contains(&d), "{d}");
        assert!(!n.en_proteccion_de_campamento(suyos[0].posicion));
    }
    // Cada cita se renueva a los 10 minutos para mirar si aún falta alguno, y se detiene cuando no falta ninguno.
    assert!(de_combate_pendientes(&n).iter().all(|&(t, s)| t == 10 * MIN && s == nucleo::SucesoCombate::BandidosDelAnillo));
    n.avanzar_hasta(11 * MIN);
    assert!(de_combate_pendientes(&n).is_empty(), "satisfechos los anillos, no queda nada pendiente: nada recurre por cosa ociosa");
    assert_eq!(n.campamentos_de_bandidos().count(), del_mundo.len() + m.0.campamentos().len());
    // Misma semilla, mismos campamentos.
    let mut otro = Nucleo::nuevo(arranque(m, 11));
    otro.avanzar_hasta(11 * MIN);
    assert_eq!(otro.hash_estado(), n.hash_estado());
    let mut distinto = Nucleo::nuevo(arranque(m, 12));
    distinto.avanzar_hasta(11 * MIN);
    assert_ne!(distinto.hash_estado(), n.hash_estado(), "otra semilla, otros niveles y otros puntos");
}

#[test]
fn toda_plaza_tiene_su_campamento_en_el_bosque_libre_mas_cercano_y_destruido_reaparece_a_los_60_minutos() {
    let m = con_mundo!();
    let mut e = montar(m, 7, false);
    let (a, b) = (e.a, e.b);
    let suyo = |e: &Esc, plaza: u64| e.n.campamentos_de_bandidos().find(|c| c.origen == nucleo::OrigenBandidos::Plaza(plaza)).cloned();
    let (ca, cb) = (suyo(&e, a).expect("la plaza A tiene su campamento"), suyo(&e, b).expect("la plaza B tiene el suyo"));
    assert_ne!(ca.bosque, cb.bosque, "un campamento por bosque");
    for (p, c) in [(a, &ca), (b, &cb)] {
        let bosque = &m.0.bosques()[c.bosque.unwrap() as usize];
        let centro = Punto::new(bosque.centro.as_ref().unwrap().x, bosque.centro.as_ref().unwrap().y);
        assert_eq!(c.posicion, centro);
        for q in e.n.estado().asentamientos.values() {
            assert!(q.posicion.dist(centro) >= q.radio + 10_000, "el campamento de as-{p} está a ≥ 10 u de la zona de as-{}", q.id);
        }
        assert!(!e.n.en_zona(a, c.posicion) && !e.n.en_zona(b, c.posicion));
    }
    let apariciones = hechos(&e.n, "bandidos.campamento_aparece");
    assert!(apariciones.iter().any(|h| h.2 == vec![a] && dato(h, "campamento") == ca.id as i64 && dato(h, "nivel") == ca.nivel as i64));
    assert!(de_combate_pendientes(&e.n).is_empty(), "plazas y anillos satisfechos: no queda ninguna cita pendiente");
    let (cola0, ahora0) = (e.n.eventos_en_cola(), e.n.ahora());
    e.avanzar(10 * HORA);
    assert!(e.n.eventos_en_cola() <= cola0 + 2, "los campamentos quietos no programan nada ({cola0} → {})", e.n.eventos_en_cola());
    let _ = ahora0;

    // Destruido (por quien sea), le reaparece uno al cabo de 60 minutos, exactos, y no antes.
    let atacante = e.ejercito("gus", 3, ca.posicion, &[Tropa::ArquerosCompuesto, Tropa::EspadachinesBronce]);
    let t = e.n.ahora();
    assert!(e.atacar("gus", atacante.1, ObjetivoAtaque::Campamento(ca.id), false).aceptada());
    assert!(suyo(&e, a).is_none());
    assert_eq!(e.n.estado().combate.reaparecen[&a], t + 60 * MIN);
    assert_eq!(de_combate_pendientes(&e.n), vec![(t + 60 * MIN, nucleo::SucesoCombate::BandidosDeLaPlaza)]);
    e.n.avanzar_hasta(t + 60 * MIN - 1);
    assert!(suyo(&e, a).is_none());
    e.n.avanzar_hasta(t + 60 * MIN);
    let nuevo = suyo(&e, a).expect("reaparece junto a su plaza");
    assert!(nuevo.id != ca.id && e.n.estado().combate.reaparecen.get(&a).is_none());
    assert_eq!(hechos(&e.n, "bandidos.campamento_aparece").iter().filter(|h| h.2 == vec![a]).map(|h| h.0).collect::<Vec<_>>(), vec![0, t + 60 * MIN]);
    assert!(de_combate_pendientes(&e.n).is_empty());
}

#[test]
fn nadie_inicia_un_combate_junto_a_un_campamento_de_mercenarios() {
    let m = con_mundo!();
    let mut e = montar(m, 7, false);
    let c0 = campamento(&m.0, 0);
    let ana = e.ejercito("gus", 1, c0, &[Tropa::EspadachinesBronce]);
    let beto = e.ejercito("hugo", 2, Punto::new(c0.x + 3_000, c0.y), &[Tropa::MiliciaLanceros]);
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    assert!(e.n.en_proteccion_de_campamento(c0));
    assert_eq!(codigo(&e.atacar("gus", ana.1, ObjetivoAtaque::Ejercito(beto.0), false)), rechazos::COMBATE_PROTECCION_DE_CAMPAMENTO, "ni desde allí");
    // Ni contra lo que está allí: un atacante a 12 u del campamento contra uno que está dentro de su radio.
    let fuera = e.ejercito("ivan", 1, Punto::new(c0.x + 9_500, c0.y), &[Tropa::EspadachinesBronce]);
    let dentro = columna(&mut e.n, "jose", 2, Punto::new(c0.x + 9_500, c0.y));
    assert_eq!(codigo(&e.atacar("ivan", fuera.1, ObjetivoAtaque::Ejercito(dentro.0), false)), rechazos::COMBATE_PROTECCION_DE_CAMPAMENTO, "ni contra lo que esté allí");
    // Ni se colocan bandidos allí.
    assert_eq!(e.rechazada("admin", Comando::ColocarBandidos { posicion: Punto::new(c0.x + 5_000, c0.y), nivel: 1 }), rechazos::BANDIDOS_SOBRE_UNA_ZONA);
    // Ni se colocan dentro de una zona de influencia.
    let a = e.a;
    assert_eq!(e.rechazada("admin", Comando::ColocarBandidos { posicion: e.pos(a), nivel: 1 }), rechazos::BANDIDOS_SOBRE_UNA_ZONA);
}

/// Un campamento de bandidos de ese nivel en la ruta de la caravana, al 40 %, fuera de toda zona. Devuelve su id y el instante exacto en que el plan de la
/// caravana entra por primera vez a ≤ 7 u de él.
fn bandidos_en_la_ruta(e: &mut Esc, caravana: u64, nivel: u8) -> (u64, Instante) {
    let (t0, fin) = {
        let p = e.n.caravana(caravana).unwrap().plan.as_ref().unwrap();
        (p.inicio().unwrap(), p.fin().unwrap())
    };
    let donde = e.n.caravana(caravana).unwrap().plan.as_ref().unwrap().posicion_en(t0 + (fin - t0) * 4 / 10).unwrap();
    let id = e.ok("admin", Comando::ColocarBandidos { posicion: donde, nivel }).creadas[0];
    let radio = mc().bandidos.radio_ataque_caravana;
    let mut t = e.n.ahora();
    let llegada = loop {
        if e.n.caravana(caravana).unwrap().plan.as_ref().unwrap().posicion_en(t).unwrap().dist(donde) <= radio {
            break t;
        }
        t += 1_000;
    };
    (id, llegada)
}

#[test]
fn una_caravana_que_cruza_el_radio_de_un_campamento_de_bandidos_es_acosada_en_ese_instante_exacto() {
    let m = con_mundo!();
    // Nivel 3 (poder 120, 102 como mínimo) contra los carreteros (26): la caravana desaparece con su carga.
    let mut e = montar(m, 7, true);
    let (a, _) = (e.a, e.b);
    let ca = e.caravana("ana", a, 2);
    e.lanzar(ca);
    let (b3, llegada) = bandidos_en_la_ruta(&mut e, ca, 3);
    let suyos = |e: &Esc, codigo: &str, b: u64| -> Vec<Hecho> { hechos(&e.n, codigo).into_iter().filter(|h| dato(h, "campamento") == b as i64).collect() };
    e.n.avanzar_hasta(llegada - 1_000);
    assert!(e.n.caravana(ca).is_some(), "aún no ha llegado al radio");
    assert!(suyos(&e, "bandidos.caravana_interceptada", b3).is_empty());
    e.n.avanzar_hasta(llegada + 1_000);
    let h = suyos(&e, "bandidos.caravana_interceptada", b3);
    assert_eq!(h.len(), 1, "{h:?}");
    assert!(h[0].0 > llegada - 1_000 && h[0].0 <= llegada, "el acoso es el cruce exacto del plan con el radio: {} frente a {llegada}", h[0].0);
    assert_eq!((dato(&h[0], "campamento"), dato(&h[0], "caravana")), (b3 as i64, ca as i64));
    assert!(e.n.caravana(ca).is_none(), "la caravana desaparece");
    assert_eq!(hechos(&e.n, "comercio.caravana_perdida").len(), 1);
    assert!(e.n.campamento_de_bandidos(b3).is_some(), "el campamento sigue donde estaba");

    // Nivel 1 (poder 20, 23 como mucho) contra los mismos carreteros: la caravana escapa y sigue su camino.
    let mut e = montar(m, 7, true);
    let ca = e.caravana("ana", a, 2);
    e.lanzar(ca);
    let (b1, llegada) = bandidos_en_la_ruta(&mut e, ca, 1);
    e.n.avanzar_hasta(llegada + 1_000);
    let h = suyos(&e, "bandidos.caravana_escapa", b1);
    assert_eq!(h.len(), 1);
    assert!(e.n.caravana(ca).is_some_and(|k| k.estado == EstadoCaravana::EnTransito && k.carros.len() == 2));
    // Y no vuelve a ser acosada por ese campamento durante ese mismo paso.
    let fin = e.n.caravana(ca).unwrap().plan.as_ref().unwrap().fin().unwrap();
    e.n.avanzar_hasta(fin + MIN);
    assert_eq!(suyos(&e, "bandidos.caravana_escapa", b1).len(), 1);
}

#[test]
fn una_caravana_adjunta_a_un_ejercito_se_defiende_con_el_poder_de_la_columna_que_no_sufre_bajas() {
    let m = con_mundo!();
    for (tropas, cae) in [(&[Tropa::MiliciaLanceros][..], true), (&[Tropa::ArquerosCompuesto][..], false)] {
        let mut e = montar(m, 7, true);
        let (a, b) = (e.a, e.b);
        let ca = e.caravana("ana", a, 1);
        let col = e.ejercito("ana", 1, e.pos(a), tropas);
        e.ok("admin", Comando::AdjuntarCaravana { caravana: ca, ejercito: col.0 });
        let soldados0 = e.n.soldados_de_columna(col.0).unwrap();
        e.ok("ana", Comando::MarcharA { ejercito: col.0, destino: e.pos(b) });
        let (t0, fin) = {
            let p = &e.n.ejercito(col.0).unwrap().plan;
            (p.inicio().unwrap(), p.fin().unwrap())
        };
        let donde = e.n.ejercito(col.0).unwrap().plan.posicion_en(t0 + (fin - t0) * 4 / 10).unwrap();
        let mio = e.ok("admin", Comando::ColocarBandidos { posicion: donde, nivel: 3 }).creadas[0];
        e.n.avanzar_hasta(fin);
        let suyos = |codigo: &str| -> Vec<Hecho> { hechos(&e.n, codigo).into_iter().filter(|h| dato(h, "campamento") == mio as i64).collect() };
        let (perdida, escapa) = (suyos("bandidos.caravana_interceptada"), suyos("bandidos.caravana_escapa"));
        assert_eq!((perdida.len(), escapa.len()), (cae as usize, !cae as usize), "{tropas:?}: {perdida:?} {escapa:?}");
        assert_eq!(e.n.caravana(ca).is_none(), cae);
        assert_eq!(e.n.soldados_de_columna(col.0).unwrap(), soldados0, "la columna no sufre bajas por el acoso");
        assert!(!e.n.ejercito(col.0).unwrap().caravanas_adjuntas.contains(&ca) || !cae);
    }
}

// ───────────────────────────── 8. Conquista con visitantes, foto y determinismo ─────────────────────────────

#[test]
fn el_visitante_amigo_defiende_con_su_columna_aparcada_y_si_la_plaza_cae_vuelve_a_ella() {
    let m = con_mundo!();
    let mut s = asedio(m, 7, &[Tropa::MiliciaLanceros], &[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto]);
    let (b, beto, ana) = (s.e.b, s.beto, s.ana);
    // Un héroe de la Facción 2 llega de visita con su columna y la deja aparcada a la puerta; defiende con ella, en el cupo que dejan los residentes.
    let visita = s.e.ejercito("carla", 2, s.e.pos(b), &[Tropa::MiliciaLanceros]);
    s.e.ok("carla", Comando::Guarnecer { asentamiento: b, heroe: visita.1 });
    let hq = s.e.n.heroe(visita.1).unwrap().heroe.escuadras[0].id;
    assert_eq!(s.e.n.heroe(visita.1).unwrap().heroe.presencia.ubicacion, Ubicacion::Asentamiento(b as u32));
    assert!(!s.e.n.en_el_mapa(visita.0), "la columna aparcada no está en el mapa");
    let antes = s.e.n.heroe(visita.1).unwrap().heroe.escuadra(hq).unwrap().cantidad;

    assert!(s.e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b), true).aceptada());
    let h = &hechos(&s.e.n, "combate.asedio_conquista");
    assert_eq!(h.len(), 1, "la plaza cae pese a la ayuda");
    let tras = s.e.n.heroe(visita.1).unwrap();
    assert!(tras.heroe.escuadra(hq).unwrap().cantidad < antes, "la escuadra de la columna aparcada sufrió bajas: defendió");
    assert!(s.e.n.heroe_herido(visita.1) && s.e.n.heroe_herido(beto), "los dos defensores en persona quedan heridos");
    // Vuelve a su columna aparcada, que ya está otra vez en el mapa, y no es residente: no cambia de casa.
    assert_eq!(tras.heroe.presencia.ubicacion, Ubicacion::Columna(visita.0));
    assert!(s.e.n.en_el_mapa(visita.0));
    assert_ne!(tras.heroe.residencia, s.e.n.heroe(beto).unwrap().heroe.residencia);
    assert_eq!(s.e.n.ejercito(visita.0).unwrap().columna.faccion, Some(2));
}

/// El mismo guion de dos asedios con bandidos y un anillo a su alrededor, con las órdenes en instantes fijos: `avanzar` solo decide cómo se mueve el reloj entre
/// ellas.
fn primera_mitad(e: &mut Esc, m: &'static M) -> (u64, (u64, u64)) {
    let (a, b) = (e.a, e.b);
    let base = e.n.ahora();
    e.en(base + MIN);
    let beto = e.heroe_en(&m.0, "beto", 2, b);
    e.defensor(beto, &[Tropa::MiliciaLanceros]);
    let ana = e.ejercito("ana", 1, e.pos(b), &[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto]);
    e.en(base + 10 * MIN);
    assert!(e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b), true).aceptada());
    e.fijo = None;
    let _ = a;
    (beto, ana)
}

fn segunda_mitad(e: &mut Esc, m: &'static M, beto: u64, avanzar: &dyn Fn(&mut Esc, Instante)) {
    let a = e.a;
    let _ = (m, beto);
    let base = 2 * HORA;
    avanzar(e, base + 2 * HORA);
    let suyo = e.n.campamentos_de_bandidos().find(|c| c.origen == nucleo::OrigenBandidos::Plaza(a)).cloned().expect("la plaza A tiene su campamento");
    e.en(base + 2 * HORA);
    let gus = e.ejercito("gus", 3, suyo.posicion, &[Tropa::ArquerosCompuesto, Tropa::EspadachinesBronce]);
    assert!(e.atacar("gus", gus.1, ObjetivoAtaque::Campamento(suyo.id), false).aceptada());
    avanzar(e, base + 4 * HORA);
    e.en(base + 4 * HORA);
    let hugo = e.ejercito("hugo", 3, e.pos(e.b), &[Tropa::EspadachinesBronce]);
    e.ok("admin", Comando::FijarRelacion { a: 3, b: 1, relacion: Some(Relacion::Guerra) });
    assert_eq!(codigo(&e.atacar("hugo", hugo.1, ObjetivoAtaque::Asentamiento(e.b), false)), rechazos::COMBATE_PLAZA_PROTEGIDA);
    avanzar(e, base + 30 * HORA);
    e.fijo = None;
    e.n.avanzar_hasta(base + 30 * HORA);
}

fn guion(m: &'static M, semilla: u64, avanzar: &dyn Fn(&mut Esc, Instante)) -> Esc {
    let mut e = montar(m, semilla, false);
    let (beto, _) = primera_mitad(&mut e, m);
    segunda_mitad(&mut e, m, beto, avanzar);
    e
}

#[test]
fn mismas_ordenes_dan_el_mismo_hash_normal_acelerado_y_a_trozos() {
    let m = con_mundo!();
    let normal = guion(m, 3, &|e, t| e.n.avanzar_hasta(t));
    assert_eq!(normal.n.hash_estado(), guion(m, 3, &|e, t| e.n.avanzar_hasta(t)).n.hash_estado(), "repetible");
    assert_eq!(normal.n.hash_estado(), guion(m, 3, &|_, _| {}).n.hash_estado(), "acelerado de un golpe");
    let a_trozos = guion(m, 3, &|e, t| {
        let ahora = e.n.ahora();
        for s in 1..=5 {
            e.n.avanzar_hasta(ahora + (t - ahora).max(0) * s * s / 25);
        }
    });
    assert_eq!(normal.n.hash_estado(), a_trozos.n.hash_estado(), "en pasos pequeños e irregulares, como el modo en vivo");
    assert_ne!(normal.n.hash_estado(), guion(m, 4, &|e, t| e.n.avanzar_hasta(t)).n.hash_estado(), "otra semilla, otro mundo");
    // El guion hizo de verdad lo que se espera.
    for h in ["combate.asedio_conquista", "combate.campamento_destruido", "asentamiento.conquistada", "asentamiento.ocupacion_terminada", "asentamiento.proteccion_terminada", "bandidos.campamento_aparece"] {
        assert!(!hechos(&normal.n, h).is_empty(), "el guion no produjo «{h}»");
    }
    // Pasadas 28 horas desde el guion no queda ninguna cita de combate pendiente que no sea la reaparición del campamento destruido (ya vencida).
    assert!(de_combate_pendientes(&normal.n).is_empty());
}

#[test]
fn una_foto_con_la_conquista_en_proteccion_recupera_el_mismo_mundo_y_los_plazos_vencen_a_su_hora() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9) };
    let sin_parar = {
        let mut e = montar(m, 9, false);
        let (beto, _) = primera_mitad(&mut e, m);
        e.n.avanzar_hasta(e.n.ahora() + 30 * MIN);
        segunda_mitad(&mut e, m, beto, &|e, t| e.n.avanzar_hasta(t));
        (e.n.hash_estado(), e.n.estado().combate.clone())
    };
    let (a_mitad, beto, cuenta, fijo) = {
        let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        let mut e = montar_sobre(n, m, false);
        let (beto, _) = primera_mitad(&mut e, m);
        assert!(e.n.metricas().ordenes_aceptadas > 8, "las órdenes de combate entran al registro");
        // La foto se hace con la plaza protegida y ocupada y sus dos plazos en la cola.
        e.n.avanzar_hasta(e.n.ahora() + 30 * MIN);
        assert!(e.n.plaza_protegida(e.b) && e.n.ocupada_hasta(e.b).is_some());
        e.n.foto_ahora();
        e.n.avanzar_hasta(e.n.ahora() + 5 * MIN);
        e.n.apagar().unwrap();
        (e.n.hash_estado(), beto, e.cuenta, e.fijo)
    };
    let n = Nucleo::con_persistencia(con(ajustes), dir.path(), Default::default()).unwrap();
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), a_mitad, "foto + registro reconstruyen el mismo mundo, con la protección y la ocupación a medias");
    let (a, b) = (*n.estado().asentamientos.keys().next().unwrap(), *n.estado().asentamientos.keys().last().unwrap());
    let mut e = Esc { n, a, b, cuenta, fijo };
    assert!(e.n.plaza_protegida(b), "sigue protegida");
    let t_ocupacion = e.n.ocupada_hasta(b).expect("sigue ocupada");
    e.n.avanzar_hasta(t_ocupacion - 1);
    assert!(e.n.estado().asentamientos[&b].economia.ocupado);
    e.n.avanzar_hasta(t_ocupacion);
    assert!(!e.n.estado().asentamientos[&b].economia.ocupado, "la ocupación vence a su hora aunque se recuperara de una foto");
    segunda_mitad(&mut e, m, beto, &|e, t| e.n.avanzar_hasta(t));
    let (tras, antes) = (&e.n.estado().combate, &sin_parar.1);
    for (id, b) in &antes.bandidos {
        assert_eq!(tras.bandidos.get(id), Some(b), "el campamento {id} recuperado es el del mundo que no se interrumpió");
    }
    assert_eq!((tras.bandidos.len(), tras.siguiente_bandidos), (antes.bandidos.len(), antes.siguiente_bandidos));
    assert_eq!((&tras.reaparecen, &tras.anillos, &tras.destruidos), (&antes.reaparecen, &antes.anillos, &antes.destruidos), "los plazos y la historia de los bandidos coinciden");
    assert_eq!(tras.guarniciones, antes.guarniciones);
    assert_eq!(e.n.hash_estado(), sin_parar.0);
    e.n.apagar().unwrap();
}
