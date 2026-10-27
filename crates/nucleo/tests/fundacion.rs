//! La refundación y la Caravana de Fundación de verdad (sistema 5, tanda 5G-b; Fundacion_Y_Refundacion §3-§9, Movimiento_Ejercitos §9): `lanzarCaravanaFundacion`, `desarmarCaravanaFundacion`,
//! `aportarARefundacion`, `retirarDeRefundacion`, `comprarCaravanaDeRefundacion`, engancharla (`adjuntarCaravana`), fundar con ella, que caduque suelta, que vuelva sola sin su titular y que otro
//! la reclame (`reclamarCaravana`). Cada orden con sus rechazos y la conservación de lo que cuesta. Portadas de `engine/__tests__/{caravanaFundacion,refundacion,fundacionCampamento}.test.ts` y
//! `session/__tests__/comandosExpansion.test.ts` del TypeScript. Sobre el artefacto de mundo real; se omiten si no está. La prueba por el WebSocket real está al final.

mod comun;

use asentamiento::Recurso::{self, *};
use base::{Punto, MS_POR_MINUTO};
use comun::real::{campamento, en_tierra, mundo, sitio_rico, M};
use comun::sesion::*;
use contrato::v1::{self as pb, orden, TipoCliente};
use militar::catalogo::Tropa;
use militar::columna::{Politica, TipoColumna};
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::{rechazos, Ajustes, Arranque, Carga, Comando, Detalle, Estado, LadoDelFondo, Nucleo, ObjetivoAtaque, OrigenFundacion, Peticion, Reglas, Respuesta, Resultado, SituacionFundacion};
use std::collections::BTreeMap;

const HORA: i64 = 60 * MS_POR_MINUTO;

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

fn arranque(m: &M) -> Arranque {
    Arranque { reglas: Reglas::default(), ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn codigo(r: &Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

/// Los datos enteros de cada hecho `codigo`.
fn hechos(n: &Nucleo, codigo: &str) -> Vec<Vec<(&'static str, i64)>> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).map(|e| e.datos.clone()).collect()
}

fn dato(datos: &[(&'static str, i64)], clave: &str) -> i64 {
    datos.iter().find(|(k, _)| *k == clave).unwrap_or_else(|| panic!("falta «{clave}» en {datos:?}")).1
}

fn unidades(v: &[(Recurso, i64)]) -> BTreeMap<Recurso, i64> {
    v.iter().map(|&(r, n)| (r, n * 1_000)).collect()
}

struct Esc {
    n: Nucleo,
    cuenta: u64,
}

impl Esc {
    fn nuevo(m: &M) -> Self {
        Esc { n: Nucleo::nuevo(arranque(m)), cuenta: 0 }
    }
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.cuenta += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.cuenta), c);
        // 5I (5E-10): sin el andamiaje de `comun::mandaba`; cada cuenta da sus órdenes de plaza con la matriz real.
        let p = if actor == "admin" { p.de_admin() } else { p };
        let t = self.n.ahora();
        self.n.aplicar_en(t, p)
    }
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    fn no(&mut self, actor: &str, c: Comando) -> &'static str {
        // Lo que vence en este mismo instante se procesa antes: la versión de después solo puede cambiar por la orden.
        let t = self.n.ahora();
        self.n.avanzar_hasta(t);
        let antes = self.n.estado().version;
        let r = self.ord(actor, c);
        assert_eq!(self.n.estado().version, antes, "un rechazo no versiona");
        codigo(&r)
    }
    fn avanzar(&mut self, ms: i64) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    fn heroe(&mut self, cuenta: &str, campamento: u32) -> u64 {
        self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento, controlador: Controlador::Humano, nombre: None }).creadas[0]
    }
    fn almacen(&self, plaza: u64, r: Recurso) -> i64 {
        self.n.economia(plaza).unwrap().almacen.cantidad(r)
    }
    fn dar(&mut self, plaza: u64, recurso: Recurso, cantidad: i64) {
        let r = self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
    }
    fn llevar(&mut self, cuenta: &str, col: u64, destino: Punto) {
        self.ok(cuenta, Comando::MarcharA { ejercito: col, destino });
        let fin = self.n.ejercito(col).unwrap().plan.fin().unwrap();
        self.n.avanzar_hasta(fin);
    }
    fn caravana(&self, id: u64) -> Option<&nucleo::CaravanaFundacion> {
        self.n.estado().fundacion.caravanas.get(&id)
    }
    fn personal(&self, h: u64, r: Recurso) -> i64 {
        self.n.heroe(h).unwrap().almacen_personal.get(&r).copied().unwrap_or(0)
    }
    fn fondo(&self, campamento: u32, h: u64, r: Recurso) -> i64 {
        self.n.estado().fundacion.fondos.get(&campamento).and_then(|f| f.get(&h)).and_then(|m| m.get(&r)).copied().unwrap_or(0)
    }
    /// Los plazos de las Caravanas de Fundación que siguen vigentes en la cola.
    fn plazos(&self) -> usize {
        let e = self.n.estado();
        e.cola.volcar().into_iter().filter(|x| matches!(x.carga, Carga::Fundacion(_)) && e.generaciones.es_vigente(x.clave)).count()
    }
}

/// Funda con la administración una plaza de `faccion` en `p`.
fn fundar(e: &mut Esc, dueno: &str, faccion: u32, p: Punto) -> u64 {
    let t = e.n.ahora();
    let col = e.n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap();
    e.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
    e.ok(dueno, Comando::Fundar { ejercito: col }).creadas[0]
}

/// Un héroe de la cuenta, ciudadano de `faccion`, residente y dentro de la plaza `a`.
fn residente(e: &mut Esc, cuenta: &str, faccion: u32, a: u64) -> u64 {
    let h = e.heroe(cuenta, 0);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(faccion) });
    e.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(a as u32)) });
    e.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(a as u32) });
    h
}

/// La plaza `a` de la Facción 1, a nivel 2, con la Facción a nivel 2 (cap 2) y de todo en el almacén; «ana» reside y está dentro.
struct ConPlaza {
    e: Esc,
    a: u64,
    ana: u64,
}

fn con_plaza(m: &M) -> ConPlaza {
    con_plaza_en(m, Esc::nuevo(m))
}

/// Como [`con_plaza`], sin bandidos que acosen caravanas (desde P2 también acosan a la columna que lleva la de Fundación): para las pruebas de viajes largos.
fn con_plaza_sin_acoso(m: &M) -> ConPlaza {
    let mut a = arranque(m);
    std::sync::Arc::make_mut(&mut a.reglas.economia.combate).bandidos.radio_ataque_caravana = 0;
    con_plaza_en(m, Esc { n: Nucleo::nuevo(a), cuenta: 0 })
}

fn con_plaza_en(m: &M, mut e: Esc) -> ConPlaza {
    let p = sitio_rico(&e.n, &m.0, 0, 1);
    let a = fundar(&mut e, "dueno", 1, p);
    let ana = residente(&mut e, "ana", 1, a);
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel: 2 });
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 });
    for r in [Madera, Piedra, Trigo, Oro] {
        e.dar(a, r, 300_000);
    }
    ConPlaza { e, a, ana }
}

/// Un ejército de `h` (residente y dentro de `a`), movilizado con una milicia: queda a la puerta.
fn movilizar(e: &mut Esc, cuenta: &str, h: u64, a: u64) -> u64 {
    let esc = e.ok("admin", Comando::DarEscuadra { heroe: h, tropa: Tropa::MiliciaLanceros, cantidad: 10 }).creadas[0];
    e.ok(cuenta, Comando::MovilizarEjercito { asentamiento: a, heroe: h, escuadras: vec![esc], carga: vec![], politica: Politica::Aceptar }).creadas[0]
}

// ───────────────────────────── 1. El coste ─────────────────────────────

#[test]
fn la_caravana_de_fundacion_cuesta_lo_de_una_plaza_nueva_y_la_de_refundacion_el_cuarenta_por_ciento_redondeado_hacia_arriba() {
    let m = con_mundo!();
    let e = Esc::nuevo(m);
    assert_eq!(e.n.costo_caravana_fundacion(), unidades(&[(Madera, 160), (Piedra, 20), (Trigo, 100), (Oro, 100)]));
    assert_eq!(e.n.costo_refundacion(), unidades(&[(Madera, 64), (Piedra, 8), (Trigo, 40), (Oro, 40)]));
}

// ───────────────────────────── 2. Lanzarla desde una plaza y desarmarla ─────────────────────────────

#[test]
fn lanzar_desde_la_plaza_cobra_el_coste_reserva_el_cap_y_desarmarla_lo_devuelve_entero() {
    let m = con_mundo!();
    let ConPlaza { mut e, a, ana } = con_plaza(m);
    let lanzar = |heroe| Comando::LanzarCaravanaFundacion { heroe, asentamiento: a };
    // Rechazos, en su orden: plaza, ciudadanía, residencia y presencia, nivel, cooldown, cap y coste.
    assert_eq!(e.no("ana", Comando::LanzarCaravanaFundacion { heroe: ana, asentamiento: 999 }), rechazos::ASENTAMIENTO_DESCONOCIDO);
    let zed = e.heroe("zed", 0);
    assert_eq!(e.no("zed", lanzar(zed)), rechazos::FUNDACION_NO_CIUDADANO);
    assert_eq!(e.no("zed", lanzar(ana)), rechazos::NO_AUTORIZADO, "el héroe de otra cuenta");
    let beto = e.heroe("beto", 0);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: beto, faccion: Some(1) });
    assert_eq!(e.no("beto", lanzar(beto)), rechazos::FUNDACION_NO_RESIDES, "ciudadano pero no reside en la plaza");
    e.ok("admin", Comando::FijarResidencia { heroe: beto, residencia: Some(Residencia::Asentamiento(a as u32)) });
    assert_eq!(e.no("beto", lanzar(beto)), rechazos::FUNDACION_NO_RESIDES, "reside pero no está dentro");
    e.ok("admin", Comando::ColocarHeroe { heroe: beto, ubicacion: Ubicacion::Asentamiento(a as u32) });
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 1 });
    assert_eq!(e.no("ana", lanzar(ana)), rechazos::CAP_ALCANZADO, "con cap 1 y una plaza, no cabe otra");
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 });

    // Lanzarla: el almacén paga el coste entero, nace parada en la plaza con su titular, y caduca en 48 h.
    let costo = e.n.costo_caravana_fundacion();
    let antes: BTreeMap<Recurso, i64> = costo.keys().map(|&r| (r, e.almacen(a, r))).collect();
    let t0 = e.n.ahora();
    let r = e.ok("ana", lanzar(ana));
    let k = r.creadas[0];
    for (&rec, &n) in &costo {
        assert_eq!(e.almacen(a, rec), antes[&rec] - n, "{rec:?}");
    }
    let c = e.caravana(k).unwrap().clone();
    let pos = e.n.estado().asentamientos[&a].posicion;
    assert_eq!((c.faccion, c.titular, c.origen, c.situacion.clone(), c.posicion, c.caduca_en), (Some(1), ana, OrigenFundacion::Plaza(a), SituacionFundacion::Disponible, pos, Some(t0 + 48 * HORA)));
    assert_eq!(c.contenido, costo);
    assert_eq!(e.plazos(), 1, "solo su caducidad");
    let h = hechos(&e.n, "expansion.caravana_lanzada");
    assert_eq!((h.len(), dato(&h[0], "caravana") as u64, dato(&h[0], "heroe") as u64, dato(&h[0], "faccion")), (1, k, ana, 1));
    // El cooldown de caravanas de la plaza (compartido con las comerciales) y, pasado, el cap: la caravana viva ya cuenta.
    assert_eq!(e.no("beto", lanzar(beto)), rechazos::CARAVANA_EN_COOLDOWN);
    e.avanzar(11 * MS_POR_MINUTO);
    assert_eq!(e.no("beto", lanzar(beto)), rechazos::CAP_ALCANZADO, "una plaza y una caravana ya son el cap 2");
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 3 });
    // Sin el coste no sale: una plaza recién fundada solo tiene sus materiales iniciales (50 de madera).
    let pb = sitio_rico(&e.n, &m.0, 2, 1);
    let b = fundar(&mut e, "dueno", 1, pb);
    let carlos = residente(&mut e, "carlos", 1, b);
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: b, nivel: 2 });
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 5 });
    assert_eq!(e.no("carlos", Comando::LanzarCaravanaFundacion { heroe: carlos, asentamiento: b }), rechazos::CARAVANA_SIN_RECURSOS);

    // Desarmar: solo su titular y solo suelta; devuelve lo que costó a la plaza.
    assert_eq!(e.no("beto", Comando::DesarmarCaravanaFundacion { heroe: beto, caravana: k }), rechazos::FUNDACION_NO_TITULAR);
    assert_eq!(e.no("ana", Comando::DesarmarCaravanaFundacion { heroe: ana, caravana: 999 }), rechazos::CARAVANA_DESCONOCIDA);
    let antes: BTreeMap<Recurso, i64> = costo.keys().map(|&r| (r, e.almacen(a, r))).collect();
    let entra: BTreeMap<Recurso, i64> = costo.iter().map(|(&r, &n)| (r, n.min(e.n.economia(a).unwrap().almacen.espacio(r)))).collect();
    e.ok("ana", Comando::DesarmarCaravanaFundacion { heroe: ana, caravana: k });
    for &rec in costo.keys() {
        assert_eq!(e.almacen(a, rec), antes[&rec] + entra[&rec], "{rec:?}: vuelve hasta donde cabe");
    }
    let esperado: i64 = entra.values().sum();
    assert!(e.caravana(k).is_none());
    assert_eq!(e.plazos(), 0, "ninguna caravana, ningún plazo");
    let h = hechos(&e.n, "expansion.caravana_desarmada");
    assert_eq!((h.len(), dato(&h[0], "devuelto")), (1, esperado));
}

#[test]
fn una_plaza_de_nivel_1_no_lanza_y_la_caravana_suelta_caduca_a_las_48_horas_y_devuelve_el_coste() {
    let m = con_mundo!();
    let ConPlaza { mut e, a, ana } = con_plaza(m);
    let p = sitio_rico(&e.n, &m.0, 2, 1);
    let b = fundar(&mut e, "dueno", 1, p);
    let bea = residente(&mut e, "bea", 1, b);
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 5 });
    assert_eq!(e.no("bea", Comando::LanzarCaravanaFundacion { heroe: bea, asentamiento: b }), rechazos::FUNDACION_NIVEL_INSUFICIENTE);

    let k = e.ok("ana", Comando::LanzarCaravanaFundacion { heroe: ana, asentamiento: a }).creadas[0];
    e.avanzar(48 * HORA - 1);
    assert!(e.caravana(k).is_some(), "aún no");
    e.avanzar(1);
    assert!(e.caravana(k).is_none(), "caduca");
    // Vuelve al almacén hasta donde quepa (en 48 h la plaza ha ido llenándolo): lo demás se pierde.
    let h = hechos(&e.n, "fundacion.caravana_devuelta");
    let devuelto = dato(&h[0], "devuelto");
    assert_eq!((h.len(), dato(&h[0], "caravana") as u64), (1, k));
    assert!(devuelto > 0 && devuelto <= e.n.costo_caravana_fundacion().values().sum::<i64>(), "{devuelto}");
    assert_eq!(e.plazos(), 0);
}

// ───────────────────────────── 3. Engancharla, fundar, volver sola y reclamarla ─────────────────────────────

#[test]
fn su_titular_la_engancha_a_un_ejercito_a_la_puerta_y_funda_con_ella_donde_llega() {
    let m = con_mundo!();
    let ConPlaza { mut e, a, ana } = con_plaza_sin_acoso(m);
    let k = e.ok("ana", Comando::LanzarCaravanaFundacion { heroe: ana, asentamiento: a }).creadas[0];
    // Una columna personal no lleva caravanas; un ejército sin su titular, tampoco.
    let beto = residente(&mut e, "beto", 1, a);
    let suyo = movilizar(&mut e, "beto", beto, a);
    assert_eq!(e.no("beto", Comando::AdjuntarCaravana { caravana: k, ejercito: suyo }), rechazos::FUNDACION_NO_TITULAR);
    let col = e.ok("ana", Comando::SalirAlMundo { asentamiento: a, heroe: ana, escuadras: vec![], carga: vec![] }).creadas[0];
    assert_eq!(e.no("ana", Comando::AdjuntarCaravana { caravana: k, ejercito: col }), rechazos::COLUMNA_PERSONAL_SIN_CARAVANAS);
    e.ok("ana", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: ana });
    let ej = movilizar(&mut e, "ana", ana, a);
    assert_eq!(e.no("ana", Comando::Fundar { ejercito: ej }), rechazos::SIN_CARAVANA, "aún no la lleva");
    e.ok("ana", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });
    assert_eq!((e.caravana(k).unwrap().situacion.clone(), e.caravana(k).unwrap().caduca_en), (SituacionFundacion::Adjunta(ej), None), "enganchada no caduca");
    assert_eq!(e.plazos(), 0);
    assert_eq!(e.no("ana", Comando::DesarmarCaravanaFundacion { heroe: ana, caravana: k }), rechazos::CARAVANA_NO_DISPONIBLE, "enganchada no se desarma");
    // Se la lleva a otro sitio y funda: la caravana se gasta y ana pasa a residir en la plaza nueva.
    let sitio = sitio_rico(&e.n, &m.0, 2, 1);
    e.llevar("ana", ej, sitio);
    let nueva = e.ok("ana", Comando::Fundar { ejercito: ej }).creadas[0];
    assert!(e.caravana(k).is_none(), "se gasta");
    assert_eq!(e.n.heroe(ana).unwrap().heroe.residencia, Some(Residencia::Asentamiento(nueva as u32)));
    let h = hechos(&e.n, "fundacion.asentamiento_fundado");
    assert_eq!(dato(h.last().unwrap(), "caravana") as u64, k);
    assert_eq!(e.no("ana", Comando::Fundar { ejercito: ej }), rechazos::SIN_CARAVANA);
}

#[test]
fn sin_su_titular_vuelve_sola_otro_de_su_faccion_la_reclama_y_al_llegar_se_desarma() {
    let m = con_mundo!();
    let ConPlaza { mut e, a, ana } = con_plaza(m);
    let k = e.ok("ana", Comando::LanzarCaravanaFundacion { heroe: ana, asentamiento: a }).creadas[0];
    let beto = residente(&mut e, "beto", 1, a);
    let ej = movilizar(&mut e, "beto", beto, a);
    let suya = e.ok("admin", Comando::DarEscuadra { heroe: ana, tropa: Tropa::MiliciaLanceros, cantidad: 10 }).creadas[0];
    e.ok("ana", Comando::UnirseAEjercito { asentamiento: a, heroe: ana, ejercito: ej, escuadras: vec![suya], carga: vec![] });
    e.ok("beto", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });
    let pos = e.n.estado().asentamientos[&a].posicion;
    let lejos = en_tierra(&m.0, pos, 40_000, 0);
    e.llevar("beto", ej, lejos);
    assert_eq!(e.caravana(k).unwrap().situacion, SituacionFundacion::Adjunta(ej), "va con su titular");

    // Su titular se separa: la caravana se suelta y vuelve sola a la plaza.
    let suya = e.ok("ana", Comando::SepararseDelEjercito { heroe: ana }).creadas[0];
    let t = e.n.ahora();
    let c = e.caravana(k).unwrap().clone();
    let SituacionFundacion::Retornando(plan) = &c.situacion else { panic!("vuelve sola: {c:?}") };
    let llega = plan.fin().unwrap();
    assert!(llega > t);
    assert_eq!(e.plazos(), 1, "solo su llegada");
    assert_eq!(hechos(&e.n, "fundacion.caravana_vuelve").len(), 1);

    // Reclamarla: de otra Facción no; lejos no; una caravana comercial nunca. Ana, en el sitio, sí: su columna personal pasa a ser un ejército.
    let zcol = e.n.crear_ejercito(t, "zed", 2, e.n.ejercito(suya).unwrap().posicion_en(t), 500, 100_000).unwrap();
    let zed = e.n.ejercito(zcol).unwrap().columna.lider;
    assert_eq!(e.no("zed", Comando::ReclamarCaravana { heroe: zed, caravana: k, politica: Politica::Aceptar }), rechazos::CARAVANA_FACCION_AJENA);
    let lcol = e.n.crear_ejercito(t, "luis", 1, en_tierra(&m.0, lejos, 0, 30_000), 500, 100_000).unwrap();
    let luis = e.n.ejercito(lcol).unwrap().columna.lider;
    assert_eq!(e.no("luis", Comando::ReclamarCaravana { heroe: luis, caravana: k, politica: Politica::Aceptar }), rechazos::CARAVANA_LEJOS);
    assert_eq!(e.n.ejercito(suya).unwrap().columna.tipo, TipoColumna::Personal);
    e.ok("ana", Comando::ReclamarCaravana { heroe: ana, caravana: k, politica: Politica::Preguntar });
    let col = &e.n.ejercito(suya).unwrap().columna;
    assert_eq!((col.tipo, col.politica, col.lider), (TipoColumna::Ejercito, Politica::Preguntar, ana), "ejército con ella de Líder");
    assert_eq!((e.caravana(k).unwrap().situacion.clone(), e.caravana(k).unwrap().titular), (SituacionFundacion::Adjunta(suya), ana));
    assert_eq!(e.plazos(), 0, "la llegada se descarta");
    assert_eq!(e.no("ana", Comando::ReclamarCaravana { heroe: ana, caravana: k, politica: Politica::Aceptar }), rechazos::CARAVANA_NO_RECLAMABLE, "ya no vuelve sola");

    // Deja de ser ciudadana: vuelve sola otra vez y, al llegar, se desarma y devuelve el coste a la plaza.
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: ana, faccion: None });
    let SituacionFundacion::Retornando(plan) = &e.caravana(k).unwrap().situacion else { panic!("vuelve sola") };
    let llega = plan.fin().unwrap();
    // A mitad de camino, la foto la guarda tal cual.
    let reglas = Reglas::default();
    let bytes = e.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN45");
    let s = Estado::deserializar(&bytes, &reglas, m.0.hash()).unwrap();
    assert_eq!((s.fundacion.clone(), s.hash()), (e.n.estado().fundacion.clone(), e.n.hash_estado()));
    let vieja = [b"BAN30".as_slice(), &bytes[5..]].concat();
    let Err(err) = Estado::deserializar(&vieja, &reglas, m.0.hash()) else { panic!("BAN30 no debe admitirse") };
    assert!(err.to_string().contains("BAN30") && err.to_string().contains("Caravana de Fundación"), "{err}");

    let costo = e.n.costo_caravana_fundacion();
    e.n.avanzar_hasta(llega - 1);
    assert!(e.caravana(k).is_some());
    e.n.avanzar_hasta(llega);
    assert!(e.caravana(k).is_none(), "llega y se desarma");
    let h = hechos(&e.n, "fundacion.caravana_devuelta");
    let devuelto = dato(&h[0], "devuelto");
    assert_eq!((h.len(), dato(&h[0], "caravana") as u64), (1, k));
    assert!(devuelto > 0 && devuelto <= costo.values().sum::<i64>(), "hasta donde quepa en el almacén: {devuelto}");
    assert_eq!(e.plazos(), 0);
}

#[test]
fn la_de_la_administracion_va_con_su_columna_sea_quien_sea_su_titular() {
    let m = con_mundo!();
    let mut e = Esc::nuevo(m);
    let p = sitio_rico(&e.n, &m.0, 0, 1);
    let col = e.n.crear_ejercito(0, "ana", 1, p, 500, 100_000).unwrap();
    e.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
    assert_eq!(e.ord("admin", Comando::DarCaravanaFundacion { ejercito: col }).resultado, Resultado::SinCambios, "ya lleva una");
    let k = e.n.estado().fundacion.de_columna(col).unwrap();
    let c = e.caravana(k).unwrap().clone();
    let ana = e.n.ejercito(col).unwrap().columna.lider;
    assert_eq!((c.origen, c.titular, c.faccion, c.contenido.is_empty()), (OrigenFundacion::Ninguno, ana, Some(1), true));
    // Andamiaje: su titular puede dejar de ser ciudadano y la caravana sigue con la columna.
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: ana, faccion: Some(2) });
    assert_eq!(e.caravana(k).unwrap().situacion, SituacionFundacion::Adjunta(col));
    assert!(hechos(&e.n, "fundacion.caravana_vuelve").is_empty() && hechos(&e.n, "fundacion.caravana_perdida").is_empty());
}

// ───────────────────────────── 4. La refundación ─────────────────────────────

/// Una Facción nueva sin plazas («Troya») con «ana» de Rey, residente en el campamento 0 y dentro de él.
fn faccion_sin_plazas(e: &mut Esc) -> (u32, u64) {
    let ana = e.heroe("ana", 0);
    let f = e.ok("ana", Comando::CrearFaccion { heroe: ana, nombre: "Troya".into(), sigilo: None }).creadas[0] as u32;
    (f, ana)
}

fn ingresar(e: &mut Esc, cuenta: &str, f: u32, rey: u64, campamento: u32) -> u64 {
    let h = e.heroe(cuenta, campamento);
    e.ok(cuenta, Comando::SolicitarIngreso { heroe: h, faccion: f });
    let rey_cuenta = e.n.heroe(rey).unwrap().cuenta.clone();
    e.ok(&rey_cuenta, Comando::ResponderSolicitud { heroe: rey, faccion: f, solicitante: h, aceptar: true });
    h
}

#[test]
fn el_fondo_se_aporta_y_se_retira_con_el_almacen_o_el_carro_y_la_caravana_de_refundacion_se_paga_por_orden_de_ciudadania() {
    let m = con_mundo!();
    let mut e = Esc::nuevo(m);
    let (f, ana) = faccion_sin_plazas(&mut e);
    let beto = ingresar(&mut e, "beto", f, ana, 0);
    let aportar = |heroe, recurso, cantidad, lado| Comando::AportarARefundacion { heroe, recurso, cantidad, lado };
    let retirar = |heroe, recurso, cantidad, lado| Comando::RetirarDeRefundacion { heroe, recurso, cantidad, lado };
    use LadoDelFondo::{Almacen as Alm, Carro};

    // Rechazos: sin nada en el almacén, sin columna para el carro, menos de una unidad, sin Facción, y residiendo en otro campamento.
    assert_eq!(e.no("ana", aportar(ana, Oro, 10_000, Alm)), rechazos::ALMACEN_SIN_EXISTENCIAS);
    assert_eq!(e.no("ana", aportar(ana, Oro, 10_000, Carro)), rechazos::HEROE_SIN_COLUMNA);
    assert_eq!(e.no("ana", aportar(ana, Oro, 999, Alm)), rechazos::CANTIDAD_NO_POSITIVA);
    let zed = e.heroe("zed", 0);
    assert_eq!(e.no("zed", aportar(zed, Oro, 10_000, Alm)), rechazos::FACCION_NO_PERTENECE);
    let carla = ingresar(&mut e, "carla", f, ana, 1);
    e.ok("admin", Comando::ColocarHeroe { heroe: carla, ubicacion: Ubicacion::Mercenarios(0) });
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: carla, recurso: Oro, cantidad: 10_000 });
    assert_eq!(e.no("carla", aportar(carla, Oro, 10_000, Alm)), rechazos::REFUNDACION_NO_RESIDES, "su almacén está en el campamento 1");

    // Aportar mueve min(pedido, lo que hay), en unidades enteras; nada se crea ni se pierde.
    for (h, recurso, n) in [(ana, Oro, 30_000), (ana, Madera, 100_000), (beto, Oro, 50_000), (beto, Piedra, 8_000), (beto, Trigo, 40_000)] {
        e.ok("admin", Comando::DarAlmacenPersonal { heroe: h, recurso, cantidad: n });
    }
    let oro = |e: &Esc| e.personal(ana, Oro) + e.personal(beto, Oro) + e.fondo(0, ana, Oro) + e.fondo(0, beto, Oro);
    let total0 = oro(&e);
    e.ok("ana", aportar(ana, Oro, 999_999_000, Alm));
    e.ok("beto", aportar(beto, Oro, 20_700, Alm));
    assert_eq!((e.fondo(0, ana, Oro), e.fondo(0, beto, Oro), e.personal(beto, Oro), oro(&e)), (30_000, 20_000, 30_000, total0));
    assert_eq!(e.n.fondo_de_faccion(0, f).get(&Oro), Some(&50_000));
    // Retirar lo suyo, hasta lo aportado; lo que no aportó, no.
    assert_eq!(e.no("beto", retirar(beto, Madera, 1_000, Alm)), rechazos::REFUNDACION_SIN_APORTE);
    e.ok("beto", retirar(beto, Oro, 999_000_000, Alm));
    assert_eq!((e.fondo(0, beto, Oro), e.personal(beto, Oro), oro(&e)), (0, 50_000, total0));
    let h = hechos(&e.n, "mercenarios.fondo_refundacion");
    assert_eq!(h.len(), 3);
    assert_eq!((dato(&h[2], "heroe") as u64, dato(&h[2], "cantidad"), dato(&h[2], "faccion"), dato(&h[2], "campamento")), (beto, 20_000, f as i64, 0));

    // Comprar: el fondo tiene que cubrir el coste.
    assert_eq!(e.no("ana", Comando::ComprarCaravanaDeRefundacion { heroe: ana }), rechazos::REFUNDACION_FONDO_CORTO);
    // El rechazo dice cuánto falta de cada recurso (`detalle.faltan`, fase 7): el coste menos el fondo, solo lo que no llega.
    let r = e.ord("ana", Comando::ComprarCaravanaDeRefundacion { heroe: ana });
    let Resultado::Rechazada { detalle: Some(Detalle::Faltan { recursos }), .. } = &r.resultado else { panic!("{r:?}") };
    let fondo = e.n.fondo_de_faccion(0, f);
    let esperado: Vec<(Recurso, i64)> =
        e.n.costo_refundacion().into_iter().map(|(r, n)| (r, n - fondo.get(&r).copied().unwrap_or(0))).filter(|&(_, n)| n > 0).collect();
    assert_eq!(esperado, [(Madera, 64_000), (Piedra, 8_000), (Trigo, 40_000), (Oro, 10_000)]);
    assert_eq!(*recursos, esperado);
    e.ok("ana", aportar(ana, Madera, 100_000, Alm));
    for (recurso, n) in [(Oro, 50_000), (Piedra, 8_000), (Trigo, 40_000)] {
        e.ok("beto", aportar(beto, recurso, n, Alm));
    }
    let costo = e.n.costo_refundacion();
    let fondo_antes = e.n.fondo_de_faccion(0, f);
    let k = e.ok("beto", Comando::ComprarCaravanaDeRefundacion { heroe: beto }).creadas[0];
    let c = e.caravana(k).unwrap().clone();
    assert_eq!((c.titular, c.origen, c.situacion.clone(), c.faccion), (beto, OrigenFundacion::Campamento(0), SituacionFundacion::Disponible, Some(f)));
    assert_eq!(c.contenido, costo);
    // Por orden de ciudadanía: primero lo de ana (30 de oro), luego lo de beto (10); el resto se queda en el fondo.
    assert_eq!(c.aportes[&ana], unidades(&[(Madera, 64), (Oro, 30)]));
    assert_eq!(c.aportes[&beto], unidades(&[(Oro, 10), (Piedra, 8), (Trigo, 40)]));
    let fondo_despues = e.n.fondo_de_faccion(0, f);
    for (&r, &n) in &fondo_antes {
        assert_eq!(fondo_despues.get(&r).copied().unwrap_or(0) + costo.get(&r).copied().unwrap_or(0), n, "{r:?}: lo gastado está en la caravana");
    }
    assert_eq!((e.fondo(0, ana, Madera), e.fondo(0, beto, Oro)), (36_000, 40_000));
    assert_eq!(e.no("ana", Comando::ComprarCaravanaDeRefundacion { heroe: ana }), rechazos::REFUNDACION_YA_TIENE_CARAVANA);
    assert_eq!(hechos(&e.n, "mercenarios.caravana_refundacion").len(), 1);

    // Quien deja la Facción pierde lo suyo: desarmada, lo de ana vuelve al fondo a su nombre y lo de beto, que se fue, se pierde.
    e.ok("beto", Comando::DejarFaccion { heroe: beto });
    assert_eq!(e.n.fondo_de_faccion(0, f).get(&Oro), None, "lo de beto ya no cuenta para la Facción");
    assert_eq!(e.no("ana", Comando::DesarmarCaravanaFundacion { heroe: ana, caravana: k }), rechazos::FUNDACION_NO_TITULAR);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: beto, faccion: Some(f) });
    // (beto vuelve: es el titular y la desarma; lo que puso él vuelve también, porque vuelve a ser ciudadano)
    let (m0, o0) = (e.fondo(0, ana, Madera), e.fondo(0, beto, Oro));
    e.ok("beto", Comando::DesarmarCaravanaFundacion { heroe: beto, caravana: k });
    assert_eq!((e.fondo(0, ana, Madera), e.fondo(0, ana, Oro), e.fondo(0, beto, Oro), e.fondo(0, beto, Trigo)), (m0 + 64_000, 30_000, o0 + 10_000, 40_000));
    assert!(e.caravana(k).is_none() && e.plazos() == 0);

    // Con plazas no se refunda.
    let p = sitio_rico(&e.n, &m.0, 1, f);
    fundar(&mut e, "ana", f, p);
    assert_eq!(e.no("ana", aportar(ana, Madera, 1_000, Alm)), rechazos::REFUNDACION_CON_PLAZAS);
    assert_eq!(e.no("ana", Comando::ComprarCaravanaDeRefundacion { heroe: ana }), rechazos::REFUNDACION_CON_PLAZAS);
    // Retirar lo suyo sí, siempre.
    e.ok("ana", retirar(ana, Madera, 10_000, Alm));
}

#[test]
fn dejar_la_faccion_pierde_lo_aportado_al_devolver_la_caravana_y_lejos_del_campamento_no_se_aporta() {
    let m = con_mundo!();
    let mut e = Esc::nuevo(m);
    let (f, ana) = faccion_sin_plazas(&mut e);
    let beto = ingresar(&mut e, "beto", f, ana, 0);
    for (h, recurso, n) in [(ana, Madera, 64_000), (ana, Piedra, 8_000), (ana, Trigo, 40_000), (beto, Oro, 40_000)] {
        e.ok("admin", Comando::DarAlmacenPersonal { heroe: h, recurso, cantidad: n });
        let cuenta = e.n.heroe(h).unwrap().cuenta.clone();
        e.ok(&cuenta, Comando::AportarARefundacion { heroe: h, recurso, cantidad: n, lado: LadoDelFondo::Almacen });
    }
    let k = e.ok("ana", Comando::ComprarCaravanaDeRefundacion { heroe: ana }).creadas[0];
    e.ok("beto", Comando::DejarFaccion { heroe: beto });
    e.ok("ana", Comando::DesarmarCaravanaFundacion { heroe: ana, caravana: k });
    assert_eq!((e.fondo(0, ana, Madera), e.fondo(0, ana, Trigo), e.fondo(0, beto, Oro)), (64_000, 40_000, 0), "lo de beto se pierde");
    let h = hechos(&e.n, "expansion.caravana_desarmada");
    assert_eq!(dato(&h[0], "devuelto"), 112_000);
    // Lejos del campamento no se aporta.
    let col = e.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: ana, escuadras: vec![], carga: vec![], politica: Politica::Rechazar }).creadas[0];
    e.llevar("ana", col, en_tierra(&m.0, campamento(&m.0, 0), 30_000, 0));
    assert_eq!(e.no("ana", Comando::RetirarDeRefundacion { heroe: ana, recurso: Madera, cantidad: 1_000, lado: LadoDelFondo::Carro }), rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA);
}

/// El ciclo entero de una Facción sin plazas (Doc 1.9b): sus héroes, en el campamento, ganan oro destruyendo bandidos del anillo (solo lo cobran los sin plaza, Cal-5), lo aportan desde el carro,
/// los demás recursos desde el almacén, compran la Caravana de Refundación, la enganchan, la llevan y fundan. El oro se conserva en cada paso.
#[test]
fn una_faccion_sin_plazas_refunda_con_el_oro_de_los_bandidos_del_anillo() {
    let m = con_mundo!();
    let mut e = Esc::nuevo(m);
    let (f, ana) = faccion_sin_plazas(&mut e);
    let beto = ingresar(&mut e, "beto", f, ana, 0);
    e.avanzar(HORA);
    let bandidos = e.n.campamentos_de_bandidos().find(|b| b.origen == nucleo::OrigenBandidos::Anillo(0)).cloned().expect("el anillo del campamento 0 tiene bandidos");

    // Ana sale como ejército, recibe tropa a bordo y destruye el campamento de bandidos: el oro va a su carro.
    let ej = e.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: ana, escuadras: vec![], carga: vec![], politica: Politica::Aceptar }).creadas[0];
    for t in [Tropa::ArquerosCompuesto, Tropa::EspadachinesBronce, Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto] {
        let n = militar::Reglas::cargar().tropas.tropa(t).unidades;
        e.ok("admin", Comando::DarEscuadra { heroe: ana, tropa: t, cantidad: n });
    }
    e.llevar("ana", ej, en_tierra(&m.0, bandidos.posicion, 2_000, 0));
    let oro_carro = |e: &Esc| e.n.ejercito(ej).unwrap().columna.carga.get(&Oro).copied().unwrap_or(0);
    let antes = oro_carro(&e);
    assert!(e.ok("ana", Comando::Atacar { heroe: ana, objetivo: ObjetivoAtaque::Campamento(bandidos.id), declara_guerra: false }).aceptada());
    let ganado = oro_carro(&e) - antes;
    let destruido = hechos(&e.n, "combate.campamento_destruido");
    assert_eq!(dato(destruido.last().unwrap(), "oro"), ganado);
    assert!(ganado > 0, "sin plaza, el anillo paga oro (Cal-5)");

    // Vuelve a la puerta del campamento y lo aporta desde el carro (es la Líder); beto pone el resto desde su almacén.
    e.llevar("ana", ej, campamento(&m.0, 0));
    let total0 = oro_carro(&e);
    e.ok("ana", Comando::AportarARefundacion { heroe: ana, recurso: Oro, cantidad: total0, lado: LadoDelFondo::Carro });
    assert_eq!((oro_carro(&e), e.fondo(0, ana, Oro)), (0, total0), "del carro al fondo, entero");
    let costo = e.n.costo_refundacion();
    for (&r, &n) in &costo {
        let falta = n - e.n.fondo_de_faccion(0, f).get(&r).copied().unwrap_or(0);
        if falta > 0 {
            e.ok("admin", Comando::DarAlmacenPersonal { heroe: beto, recurso: r, cantidad: falta });
            e.ok("beto", Comando::AportarARefundacion { heroe: beto, recurso: r, cantidad: falta, lado: LadoDelFondo::Almacen });
        }
    }
    // Compra, la engancha a su ejército a la puerta y funda lejos del campamento.
    let k = e.ok("ana", Comando::ComprarCaravanaDeRefundacion { heroe: ana }).creadas[0];
    assert_eq!(e.caravana(k).unwrap().aportes[&ana].get(&Oro).copied(), Some(total0.min(costo[&Oro])), "su oro de bandidos, primero");
    e.ok("ana", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });
    let sitio = sitio_rico(&e.n, &m.0, 0, f);
    e.llevar("ana", ej, sitio);
    let plaza = e.ok("ana", Comando::Fundar { ejercito: ej }).creadas[0];
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, f);
    assert!(e.caravana(k).is_none() && e.plazos() == 0);
    assert_eq!(e.n.heroe(ana).unwrap().heroe.residencia, Some(Residencia::Asentamiento(plaza as u32)), "funda y reside en ella");
    // Ya con plaza, la Facción no refunda.
    assert_eq!(e.no("beto", Comando::ComprarCaravanaDeRefundacion { heroe: beto }), rechazos::REFUNDACION_CON_PLAZAS);
}

// ───────────────────────────── 4b. Las decisiones del autor sobre las dudas de 5G-b (2026-10-24) ─────────────────────────────

/// Una columna de la Facción 2 («zed») en `p`, con tropa de sobra para ganar un asalto; devuelve la columna y su héroe.
fn enemigo(e: &mut Esc, p: Punto) -> (u64, u64) {
    let t = e.n.ahora();
    let col = e.n.crear_ejercito(t, "zed", 2, p, 500, 350_000).unwrap();
    let zed = e.n.ejercito(col).unwrap().columna.lider;
    for tropa in [Tropa::ArquerosCompuesto, Tropa::EspadachinesBronce, Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto] {
        let n = militar::Reglas::cargar().tropas.tropa(tropa).unidades;
        e.ok("admin", Comando::DarEscuadra { heroe: zed, tropa, cantidad: n });
    }
    (col, zed)
}

/// Lanza la caravana de ana, la engancha a un ejército de beto en el que va ana, lo lleva a `dx` de la plaza y ana se separa: la caravana vuelve sola desde ahí.
fn volviendo_desde(m: &M, dx: i64) -> (Esc, u64, u64, u64) {
    let ConPlaza { mut e, a, ana } = con_plaza(m);
    let k = e.ok("ana", Comando::LanzarCaravanaFundacion { heroe: ana, asentamiento: a }).creadas[0];
    let beto = residente(&mut e, "beto", 1, a);
    let ej = movilizar(&mut e, "beto", beto, a);
    let suya = e.ok("admin", Comando::DarEscuadra { heroe: ana, tropa: Tropa::MiliciaLanceros, cantidad: 10 }).creadas[0];
    e.ok("ana", Comando::UnirseAEjercito { asentamiento: a, heroe: ana, ejercito: ej, escuadras: vec![suya], carga: vec![] });
    e.ok("beto", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });
    let pos = e.n.estado().asentamientos[&a].posicion;
    e.llevar("beto", ej, en_tierra(&m.0, pos, dx, 0));
    e.ok("ana", Comando::SepararseDelEjercito { heroe: ana });
    assert!(matches!(e.caravana(k).unwrap().situacion, SituacionFundacion::Retornando(_)));
    (e, a, ana, k)
}

#[test]
fn decision_5gb4_se_suelta_a_mano_solo_dentro_de_su_origen_y_vuelve_a_caducar() {
    let m = con_mundo!();
    let ConPlaza { mut e, a, ana } = con_plaza(m);
    let k = e.ok("ana", Comando::LanzarCaravanaFundacion { heroe: ana, asentamiento: a }).creadas[0];
    let ej = movilizar(&mut e, "ana", ana, a);
    e.ok("ana", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });
    let soltar = || Comando::SoltarCaravana { ejercito: ej, caravana: k, heroe: ana };
    assert_eq!(e.no("ana", soltar()), rechazos::CARAVANA_SOLTAR_SOLO_DENTRO, "a la puerta se está fuera");
    let pos = e.n.estado().asentamientos[&a].posicion;
    e.llevar("ana", ej, en_tierra(&m.0, pos, 30_000, 0));
    assert_eq!(e.no("ana", soltar()), rechazos::CARAVANA_SOLTAR_SOLO_DENTRO, "en mundo abierto");
    // Entra con ella en su origen, que es su residencia: la columna sigue dentro para llevarla (como con una comercial).
    e.llevar("ana", ej, pos);
    e.ok("ana", Comando::Guarnecer { asentamiento: a, heroe: ana });
    assert!(e.n.ejercito(ej).is_some() && !e.n.en_el_mapa(ej), "dentro, con la caravana");
    assert_eq!(e.caravana(k).unwrap().situacion, SituacionFundacion::Adjunta(ej), "entrar no la suelta");
    let t = e.n.ahora();
    e.ok("ana", soltar());
    let c = e.caravana(k).unwrap();
    assert_eq!((c.situacion.clone(), c.posicion, c.caduca_en), (SituacionFundacion::Disponible, pos, Some(t + 48 * HORA)), "suelta en su origen, vuelve a caducar");
    assert_eq!(e.plazos(), 1);
    assert!(e.n.ejercito(ej).is_none() && e.n.heroe(ana).unwrap().columna.is_none(), "la columna que conservaba para llevarla se deshace");
    assert_eq!(hechos(&e.n, "ejercito.caravana_soltada").len(), 1);
}

#[test]
fn decision_5gb4_la_de_un_campamento_se_suelta_dentro_del_campamento() {
    let m = con_mundo!();
    let mut e = Esc::nuevo(m);
    let (f, ana) = faccion_sin_plazas(&mut e);
    for (&r, &n) in &e.n.costo_refundacion() {
        e.ok("admin", Comando::DarAlmacenPersonal { heroe: ana, recurso: r, cantidad: n });
        e.ok("ana", Comando::AportarARefundacion { heroe: ana, recurso: r, cantidad: n, lado: LadoDelFondo::Almacen });
    }
    let k = e.ok("ana", Comando::ComprarCaravanaDeRefundacion { heroe: ana }).creadas[0];
    let ej = e.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: ana, escuadras: vec![], carga: vec![], politica: Politica::Aceptar }).creadas[0];
    e.ok("ana", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });
    assert_eq!(e.no("ana", Comando::SoltarCaravana { ejercito: ej, caravana: k, heroe: ana }), rechazos::CARAVANA_SOLTAR_SOLO_DENTRO);
    e.ok("ana", Comando::EntrarEnCampamento { campamento: 0, heroe: ana });
    assert_eq!(e.caravana(k).unwrap().situacion, SituacionFundacion::Adjunta(ej));
    e.ok("ana", Comando::SoltarCaravana { ejercito: ej, caravana: k, heroe: ana });
    assert_eq!((e.caravana(k).unwrap().situacion.clone(), e.caravana(k).unwrap().faccion), (SituacionFundacion::Disponible, Some(f)));
    assert!(e.n.heroe(ana).unwrap().columna.is_none());
}

#[test]
fn decision_5gb5_la_que_vuelve_sola_esta_en_el_mapa_se_ve_y_fuera_de_su_zona_se_intercepta_entera() {
    let m = con_mundo!();
    let (mut e, _a, _ana, k) = volviendo_desde(m, 40_000);
    let t = e.n.ahora();
    let donde = e.caravana(k).unwrap().posicion_en(t);
    // En la rejilla de encuentros: una columna enemiga que aparece a su lado se la encuentra.
    let (col, zed) = enemigo(&mut e, donde);
    e.avanzar(1);
    let encuentros = hechos(&e.n, "caravana.encuentro");
    assert!(encuentros.iter().any(|d| dato(d, "caravana") as u64 == k), "{encuentros:?}");
    // La ve la Facción enemiga (con su columna al lado), avistada y sin cantidades; la suya, entera.
    let todo = vision::Rect { min: Punto::new(0, 0), max: Punto::new(2_667_000, 2_000_000) };
    let ver = |n: &Nucleo, p| {
        let mirada = nucleo::proyeccion::vista::Mirada::de(n, p);
        let encontrada = mirada
            .contenido(None, todo, &BTreeMap::new())
            .into_iter()
            .filter_map(|v| v.cuerpo())
            .find_map(|c| if let pb::entidad::Cuerpo::Caravana(x) = c { (x.id == format!("car-{k}")).then_some(x) } else { None });
        encontrada
    };
    let ajena = ver(&e.n, nucleo::proyeccion::vista::Perspectiva::Faccion(2)).expect("la ve la Facción enemiga");
    assert_eq!((ajena.tipo, ajena.faccion_id.clone(), ajena.contenido.is_none(), ajena.titular_id.is_none()), (pb::TipoCaravana::CaravanaDeFundacion as i32, Some("f-1".into()), true, true));
    assert!(!ajena.recursos_avistados.is_empty());
    let propia = ver(&e.n, nucleo::proyeccion::vista::Perspectiva::Faccion(1)).expect("la ve la suya");
    assert!(propia.contenido.is_some() && propia.titular_id.is_some());

    // Fuera de la zona de su dueño es una disputa (Combate §7.1): quien la gana se la lleva entera y la caravana se pierde.
    let costo: i64 = e.n.costo_caravana_fundacion().values().sum();
    let antes: i64 = e.n.ejercito(col).unwrap().columna.carga.values().sum();
    e.ok("zed", Comando::Atacar { heroe: zed, objetivo: ObjetivoAtaque::Caravana(k), declara_guerra: true });
    assert!(e.caravana(k).is_none(), "interceptada, se pierde");
    let h = hechos(&e.n, "combate.caravana_interceptada");
    assert_eq!((dato(&h[0], "caravana") as u64, dato(&h[0], "aguante"), dato(&h[0], "destruida")), (k, 0, 1));
    let cogido: i64 = e.n.ejercito(col).unwrap().columna.carga.values().sum::<i64>() - antes;
    assert_eq!(cogido, dato(&h[0], "botin"));
    assert!(cogido > 0 && cogido <= costo, "carga lo que le cabe de lo que costó: {cogido}");
    assert_eq!(hechos(&e.n, "fundacion.caravana_perdida").len(), 1);
    assert_eq!(e.plazos(), 0);
    assert_eq!(e.no("zed", Comando::Atacar { heroe: zed, objetivo: ObjetivoAtaque::Caravana(k), declara_guerra: false }), rechazos::CARAVANA_DESCONOCIDA);
}

#[test]
fn decision_5gb5_en_la_zona_de_su_duenio_aguanta_y_sigue_volviendo_con_el_treinta_por_ciento() {
    let m = con_mundo!();
    let (mut e, a, _ana, k) = volviendo_desde(m, 4_000);
    let t = e.n.ahora();
    let donde = e.caravana(k).unwrap().posicion_en(t);
    assert!(e.n.en_zona(a, donde), "a 4 de su plaza, dentro de su zona");
    let (_, zed) = enemigo(&mut e, donde);
    let costo = e.n.costo_caravana_fundacion();
    e.ok("zed", Comando::Atacar { heroe: zed, objetivo: ObjetivoAtaque::Caravana(k), declara_guerra: true });
    let h = hechos(&e.n, "combate.caravana_interceptada");
    assert_eq!((dato(&h[0], "aguante"), dato(&h[0], "destruida")), (1, 0));
    let c = e.caravana(k).expect("un aguante no la destruye").clone();
    for (&r, &n) in &costo {
        let perdido = n * 7 / 10 / 1_000 * 1_000;
        assert_eq!(c.contenido.get(&r).copied().unwrap_or(0), n - perdido, "{r:?}: el 70 % redondeado hacia abajo se pierde");
    }
    assert!(matches!(c.situacion, SituacionFundacion::Retornando(_)), "sigue volviendo");
    // Llega con lo que le queda y lo devuelve a su plaza.
    let SituacionFundacion::Retornando(plan) = &c.situacion else { unreachable!() };
    e.n.avanzar_hasta(plan.fin().unwrap());
    assert!(e.caravana(k).is_none());
    let d = hechos(&e.n, "fundacion.caravana_devuelta");
    assert!(dato(&d[0], "devuelto") <= c.contenido.values().sum::<i64>());
}

#[test]
fn decision_5gb7_en_el_campamento_cada_sesion_ve_solo_el_fondo_de_su_faccion() {
    let m = con_mundo!();
    let mut e = Esc::nuevo(m);
    let (f, ana) = faccion_sin_plazas(&mut e);
    let zoe = e.heroe("zoe", 0);
    let g = e.ok("zoe", Comando::CrearFaccion { heroe: zoe, nombre: "Micenas".into(), sigilo: None }).creadas[0] as u32;
    for (h, cuenta, n) in [(ana, "ana", 10_000), (zoe, "zoe", 20_000)] {
        e.ok("admin", Comando::DarAlmacenPersonal { heroe: h, recurso: Oro, cantidad: n });
        e.ok(cuenta, Comando::AportarARefundacion { heroe: h, recurso: Oro, cantidad: n, lado: LadoDelFondo::Almacen });
    }
    use nucleo::proyeccion::propios::{proyectar, Espectador};
    let fondos = |quien| -> BTreeMap<String, i64> {
        let p = proyectar(&e.n, quien, nucleo::Clase::Campamento, 0);
        let Some((_, _, pb::entidad::Cuerpo::CampamentoMercenarios(c))) = p.existen.into_iter().next() else { panic!("el campamento") };
        c.fondos.into_iter().map(|(k, v)| (k, v.por_id.get("oro").copied().unwrap_or(0))).collect()
    };
    assert_eq!(fondos(Espectador::Heroe(Some(ana))), BTreeMap::from([(format!("f-{f}"), 10_000)]), "ana, solo el de Troya");
    assert_eq!(fondos(Espectador::Heroe(Some(zoe))), BTreeMap::from([(format!("f-{g}"), 20_000)]), "zoe, solo el de Micenas");
    assert_eq!(fondos(Espectador::Admin), BTreeMap::from([(format!("f-{f}"), 10_000), (format!("f-{g}"), 20_000)]), "la administración, todos");
    assert!(fondos(Espectador::Heroe(None)).is_empty());
}

#[test]
fn decision_5gb11_la_caravana_suelta_en_una_plaza_conquistada_se_destruye_con_ella() {
    let m = con_mundo!();
    let ConPlaza { mut e, a, ana } = con_plaza(m);
    let k = e.ok("ana", Comando::LanzarCaravanaFundacion { heroe: ana, asentamiento: a }).creadas[0];
    // Ana sale (nadie defiende la plaza) y pasa el día de protección con que nació.
    e.ok("admin", Comando::ColocarHeroe { heroe: ana, ubicacion: Ubicacion::Mercenarios(0) });
    e.avanzar(25 * HORA);
    assert!(e.caravana(k).is_some());
    let pos = e.n.estado().asentamientos[&a].posicion;
    let (_, zed) = enemigo(&mut e, Punto::new(pos.x + 2_000, pos.y));
    e.ok("zed", Comando::Atacar { heroe: zed, objetivo: ObjetivoAtaque::Asentamiento(a), declara_guerra: true });
    let fin = e.n.batallas_activas().filter_map(|b| b.convocatoria_hasta).max();
    if let Some(h) = fin {
        e.n.avanzar_hasta(h);
    }
    assert_eq!(e.n.estado().asentamientos[&a].faccion, 2, "conquistada");
    assert!(e.caravana(k).is_none(), "la suelta se destruye con la plaza");
    let h = hechos(&e.n, "fundacion.caravana_perdida");
    assert_eq!((h.len(), dato(&h[0], "plaza") as u64), (1, a));
    assert!(hechos(&e.n, "fundacion.caravana_devuelta").is_empty(), "lo que costó se pierde");
    assert_eq!(e.plazos(), 0);
}

// ───────────────────────────── 4c. Paridad con el TypeScript (P2, 2026-10-27) ─────────────────────────────

/// La velocidad de la Caravana de Fundación (`CARAVANA_CATALOGO.construccion.velocidad`), por segundo.
fn velocidad_de_fundacion(e: &Esc) -> i64 {
    e.n.estado().reglas.economia.comercio.flota.construccion.velocidad / 60
}

#[test]
fn p2_la_columna_que_lleva_la_caravana_de_fundacion_marcha_a_su_paso_y_lo_recupera_al_perderla() {
    let m = con_mundo!();
    let ConPlaza { mut e, a, ana } = con_plaza(m);
    let k = e.ok("ana", Comando::LanzarCaravanaFundacion { heroe: ana, asentamiento: a }).creadas[0];
    let ej = movilizar(&mut e, "ana", ana, a);
    let sola = e.n.velocidad_de_ejercito(ej).unwrap();
    let vf = velocidad_de_fundacion(&e);
    assert!(vf < sola, "la de fundación es más lenta que la milicia: {vf} / {sola}");
    e.ok("ana", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });
    assert_eq!(e.n.velocidad_de_ejercito(ej), Some(vf), "va al paso de la caravana");
    // En marcha, el plan ya va a ese paso; si la caravana se pierde (bandidos), la columna recompone su marcha.
    let pos = e.n.estado().asentamientos[&a].posicion;
    e.ok("ana", Comando::MarcharA { ejercito: ej, destino: en_tierra(&m.0, pos, 60_000, 0) });
    let plan = e.n.ejercito(ej).unwrap().plan.clone();
    let (t0, fin) = (plan.inicio().unwrap(), plan.fin().unwrap());
    let donde = plan.posicion_en(t0 + (fin - t0) * 7 / 10).unwrap();
    e.ok("admin", Comando::ColocarBandidos { posicion: donde, nivel: 3 });
    e.n.avanzar_hasta(fin);
    let interceptada = hechos(&e.n, "bandidos.caravana_interceptada").into_iter().filter(|d| dato(d, "caravana") as u64 == k).count();
    let escapa = hechos(&e.n, "bandidos.caravana_escapa").into_iter().filter(|d| dato(d, "caravana") as u64 == k).count();
    assert_eq!(interceptada + escapa, 1, "los bandidos acosan a la columna por la caravana que lleva");
    if interceptada == 1 {
        assert!(e.caravana(k).is_none(), "se la llevan entera");
        assert_eq!(e.n.velocidad_de_ejercito(ej), Some(sola), "sin ella, la columna vuelve a su paso");
        assert_eq!(hechos(&e.n, "fundacion.caravana_perdida").len(), 1);
    } else {
        assert_eq!(e.caravana(k).unwrap().situacion, SituacionFundacion::Adjunta(ej));
    }
}

#[test]
fn p2_la_que_vuelve_sola_la_acosan_los_bandidos_y_se_la_llevan_entera() {
    let m = con_mundo!();
    let (mut e, _a, _ana, k) = volviendo_desde(m, 40_000);
    let SituacionFundacion::Retornando(plan) = e.caravana(k).unwrap().situacion.clone() else { unreachable!() };
    let (t0, fin) = (plan.inicio().unwrap(), plan.fin().unwrap());
    let donde = plan.posicion_en(t0 + (fin - t0) / 10).unwrap();
    // Nivel 3 contra los carreteros: gana siempre el bandido.
    e.ok("admin", Comando::ColocarBandidos { posicion: donde, nivel: 3 });
    e.n.avanzar_hasta(fin);
    let h = hechos(&e.n, "bandidos.caravana_interceptada");
    assert_eq!((h.len(), dato(&h[0], "caravana") as u64), (1, k), "{h:?}");
    assert!(e.caravana(k).is_none(), "se pierde con todo lo que costó");
    let perdida = hechos(&e.n, "fundacion.caravana_perdida");
    assert_eq!((perdida.len(), dato(&perdida[0], "interceptada")), (1, 1));
    assert!(hechos(&e.n, "fundacion.caravana_devuelta").is_empty());
    assert_eq!(e.plazos(), 0);
}

#[test]
fn p2_una_columna_personal_persigue_a_la_que_vuelve_sola_pero_no_a_la_adjunta() {
    use militar::inspeccion::Objetivo;
    let m = con_mundo!();
    let ConPlaza { mut e, a, ana } = con_plaza(m);
    let pos = e.n.estado().asentamientos[&a].posicion;
    // Zed, de otra Facción, sale de su plaza con su columna personal y espera a 45 u al este de la de ana.
    let sitio = sitio_rico(&e.n, &m.0, 2, 1);
    let b = fundar(&mut e, "zedd", 2, sitio);
    let zed = residente(&mut e, "zed", 2, b);
    let cz = e.ok("zed", Comando::SalirAlMundo { asentamiento: b, heroe: zed, escuadras: vec![], carga: vec![] }).creadas[0];
    e.llevar("zed", cz, en_tierra(&m.0, pos, 45_000, 0));
    // Ana lanza su caravana (el almacén se repone de lo que gastó la plaza mientras zed caminaba), beto la lleva a 40 u con ana dentro, y ana se separa: vuelve sola.
    for r in [Madera, Piedra, Trigo, Oro] {
        e.dar(a, r, 300_000);
    }
    let k = e.ok("ana", Comando::LanzarCaravanaFundacion { heroe: ana, asentamiento: a }).creadas[0];
    let beto = residente(&mut e, "beto", 1, a);
    let ej = movilizar(&mut e, "beto", beto, a);
    let suya = e.ok("admin", Comando::DarEscuadra { heroe: ana, tropa: Tropa::MiliciaLanceros, cantidad: 10 }).creadas[0];
    e.ok("ana", Comando::UnirseAEjercito { asentamiento: a, heroe: ana, ejercito: ej, escuadras: vec![suya], carga: vec![] });
    e.ok("beto", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });
    e.llevar("beto", ej, en_tierra(&m.0, pos, 40_000, 0));
    assert_eq!(e.no("zed", Comando::Perseguir { heroe: zed, objetivo: Objetivo::Caravana(k) }), rechazos::PERSECUCION_CARAVANA_ADJUNTA, "adjunta, se persigue a su columna");
    e.ok("ana", Comando::SepararseDelEjercito { heroe: ana });
    assert!(matches!(e.caravana(k).unwrap().situacion, SituacionFundacion::Retornando(_)));
    e.ok("zed", Comando::Perseguir { heroe: zed, objetivo: Objetivo::Caravana(k) });
    assert_eq!(e.n.persiguiendo(cz), Some(Objetivo::Caravana(k)));
    assert_eq!(e.n.perseguidores_de(Objetivo::Caravana(k)), vec![cz]);
    let iniciada = hechos(&e.n, "columna.persecucion_iniciada");
    assert_eq!(dato(iniciada.last().unwrap(), "caravana") as u64, k);
    // La alcanza antes de que llegue a su plaza.
    let SituacionFundacion::Retornando(plan) = e.caravana(k).unwrap().situacion.clone() else { unreachable!() };
    e.n.avanzar_hasta(plan.fin().unwrap());
    let alcanzada = hechos(&e.n, "columna.presa_alcanzada");
    assert!(alcanzada.iter().any(|d| dato(d, "caravana") as u64 == k), "{alcanzada:?}");
    assert_eq!(e.n.persiguiendo(cz), None);
}

// ───────────────────────────── 4d. Paridad con el TypeScript (P3, 2026-10-27) ─────────────────────────────

#[test]
fn p3_la_caravana_de_fundacion_se_inspecciona_y_el_informe_lista_sus_recursos_por_nombre() {
    use militar::inspeccion::{Contenido, Objetivo};
    let m = con_mundo!();
    let (mut e, _a, _ana, k) = volviendo_desde(m, 40_000);
    let t = e.n.ahora();
    let donde = e.caravana(k).unwrap().posicion_en(t);
    let (_, zed) = enemigo(&mut e, donde);
    e.ok("zed", Comando::Inspeccionar { heroe: zed, objetivo: Objetivo::Caravana(k) });
    let informe = e.n.informes_de_inspeccion(zed).last().copied().cloned().expect("un informe");
    let Contenido::Caravana { escoltada, recursos } = informe.contenido else { panic!("{informe:?}") };
    assert!(!escoltada, "vuelve sola");
    let nombres: Vec<&str> = recursos.iter().map(|r| r.nombre()).collect();
    let mut ordenados = nombres.clone();
    ordenados.sort();
    assert_eq!(nombres, ordenados, "por nombre, como el TypeScript");
    assert_eq!(recursos.len(), e.caravana(k).unwrap().contenido.values().filter(|&&n| n > 0).count());
    assert_eq!(hechos(&e.n, "caravana.observada").len(), 1);
}

#[test]
fn p3_una_columna_dentro_de_un_lugar_ni_ve_ni_se_ve_en_el_muestreo() {
    use nucleo::espacial::GeneradorVision;
    let m = con_mundo!();
    let mut a0 = arranque(m);
    a0.ajustes.muestreo_ms = 250;
    let mut e = Esc { n: Nucleo::nuevo(a0), cuenta: 0 };
    e.n.agregar_generador(Box::new(GeneradorVision::nuevo()));
    let ConPlaza { mut e, a, ana } = con_plaza_en(m, e);
    let pos = e.n.estado().asentamientos[&a].posicion;
    // La columna de la colonia (la que fundó) se aleja: así solo cuenta la de ana.
    let lejos = en_tierra(&m.0, pos, -70_000, 0);
    let otras: Vec<u64> = e.n.estado().ejercitos.keys().copied().collect();
    for c in otras {
        e.ok("admin", Comando::MarcharA { ejercito: c, destino: lejos });
    }
    e.avanzar(6 * HORA);
    // Ana entra en su plaza con su ejército y la Caravana de Fundación: la columna queda dentro, fuera del mapa.
    let k = e.ok("ana", Comando::LanzarCaravanaFundacion { heroe: ana, asentamiento: a }).creadas[0];
    let ej = movilizar(&mut e, "ana", ana, a);
    e.ok("ana", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });
    e.ok("ana", Comando::Guarnecer { asentamiento: a, heroe: ana });
    assert!(e.n.ejercito(ej).is_some() && !e.n.en_el_mapa(ej));
    let contacto = |e: &Esc, ve: i64, vista: i64| hechos(&e.n, "vision.contacto").iter().any(|h| dato(h, "faccionQueVe") == ve && dato(h, "faccionVista") == vista);
    let (_, _zed) = enemigo(&mut e, Punto::new(pos.x + 2_000, pos.y));
    e.avanzar(60_000);
    assert!(!contacto(&e, 2, 1), "la columna de dentro no se ve");
    assert!(!contacto(&e, 1, 2), "ni ve");
    // Otra columna de la Facción a la puerta, en el mapa: se ven.
    let beto = residente(&mut e, "beto", 1, a);
    movilizar(&mut e, "beto", beto, a);
    e.avanzar(60_000);
    assert!(contacto(&e, 2, 1) && contacto(&e, 1, 2), "a la puerta, sí");
}

// ───────────────────────────── 5. Por el WebSocket real ─────────────────────────────

fn entidades(deltas: &[pb::Delta]) -> Vec<pb::entidad::Cuerpo> {
    deltas.iter().flat_map(|d| &d.cambios).filter_map(|x| x.datos.as_ref()).filter_map(|e| e.cuerpo.clone()).collect()
}

#[tokio::test]
async fn por_la_sesion_se_aporta_se_compra_la_caravana_de_refundacion_y_se_desarma() {
    let Some(m) = mundo() else {
        eprintln!("sin artefacto de mundo real: se omite");
        return;
    };
    let mut e = Esc::nuevo(m);
    let (f, ana) = faccion_sin_plazas(&mut e);
    for (&r, &n) in &e.n.costo_refundacion() {
        e.ok("admin", Comando::DarAlmacenPersonal { heroe: ana, recurso: r, cantidad: n });
    }
    let w = arrancar_con(e.n).await;
    let (mut c, _) = w.entrar("ana", TipoCliente::ClienteUnity).await;
    for (i, r) in ["madera", "piedra", "trigo", "oro"].into_iter().enumerate() {
        let aportar = orden::Cuerpo::AportarARefundacion(pb::OrdenAportarARefundacion { recurso: r.into(), cantidad: 100_000, lado: "almacen".into() });
        let (deltas, res) = dar(&mut c, &format!("a{i}"), aportar).await;
        assert_eq!(res.resultado, pb::Resultado::OrdenAceptada as i32, "{res:?}");
        if r == "oro" {
            let fondos: Vec<pb::CampamentoMercenarios> = entidades(&deltas).into_iter().filter_map(|x| if let pb::entidad::Cuerpo::CampamentoMercenarios(k) = x { Some(k) } else { None }).collect();
            let suyo = fondos.last().and_then(|k| k.fondos.get(&format!("f-{f}"))).expect("el campamento lleva el fondo de la Facción");
            assert_eq!(suyo.por_id.get("oro"), Some(&40_000));
            assert!(deltas.iter().flat_map(|d| &d.eventos).any(|x| x.codigo == "mercenarios.fondo_refundacion"));
        }
    }
    let mal = orden::Cuerpo::AportarARefundacion(pb::OrdenAportarARefundacion { recurso: "oro".into(), cantidad: 1_000, lado: "bolsa".into() });
    assert_eq!(rechazo(&dar(&mut c, "m1", mal).await.1), "orden.forma_invalida");

    let (deltas, res) = dar(&mut c, "c1", orden::Cuerpo::ComprarCaravanaDeRefundacion(pb::OrdenComprarCaravanaDeRefundacion {})).await;
    assert_eq!(res.resultado, pb::Resultado::OrdenAceptada as i32, "{res:?}");
    let caravanas: Vec<pb::Caravana> = entidades(&deltas).into_iter().filter_map(|x| if let pb::entidad::Cuerpo::Caravana(k) = x { Some(k) } else { None }).collect();
    let k = caravanas.last().expect("la caravana llega a su titular");
    assert_eq!((k.tipo, k.titular_id.clone(), k.origen_campamento_id.clone(), k.faccion_id.clone(), k.estado), (pb::TipoCaravana::CaravanaDeFundacion as i32, Some(format!("heroe-{ana}")), Some("camp-0".into()), Some(format!("f-{f}")), pb::EstadoCaravana::CaravanaDisponible as i32));
    assert_eq!(k.aportes.get(&format!("heroe-{ana}")).and_then(|x| x.por_id.get("madera")), Some(&64_000));
    assert!(k.caduca_en_ms.is_some());
    assert!(deltas.iter().flat_map(|d| &d.eventos).any(|x| x.codigo == "mercenarios.caravana_refundacion"));

    let desarmar = orden::Cuerpo::DesarmarCaravanaFundacion(pb::OrdenDesarmarCaravanaFundacion { caravana_id: k.id.clone() });
    let (deltas, res) = dar(&mut c, "d1", desarmar).await;
    assert_eq!(res.resultado, pb::Resultado::OrdenAceptada as i32, "{res:?}");
    assert!(deltas.iter().flat_map(|d| &d.eventos).any(|x| x.codigo == "expansion.caravana_desarmada"));
    let lanzar = orden::Cuerpo::LanzarCaravanaFundacion(pb::OrdenLanzarCaravanaFundacion { origen_asentamiento_id: "as-999".into() });
    assert_eq!(rechazo(&dar(&mut c, "l1", lanzar).await.1), "asentamiento.desconocido", "ya no es `orden.no_disponible`");
    let n = w.parar().await;
    assert!(n.estado().fundacion.caravanas.is_empty());
    assert_eq!(n.fondo_de_faccion(0, f), n.costo_refundacion(), "el fondo vuelve a ser lo aportado");
}
