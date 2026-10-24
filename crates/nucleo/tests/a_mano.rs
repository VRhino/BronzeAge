//! El almacén personal y las caravanas a mano (5G-a, 2026-10-24; Movimiento_Ejercitos §3.3 y §9, Heroe_Y_Tropa §4.4): `guardarEnAlmacenPersonal`, `sacarDelAlmacenPersonal`, `cargarCaravana`,
//! `soltarCaravana` y `reclamarCaravana`, cada una con sus rechazos y con la conservación (lo que sale de un sitio entra en el otro, en unidades enteras). Portadas de
//! `residenciaMercenaria.test.ts` («almacén personal»), `comandosMercenarios.test.ts` y `avanzarEjercitos.test.ts` (`cargarCaravanaAdjunta`, `soltarCaravana`) del TypeScript. Sobre el artefacto
//! de mundo real (`generador/salida/mundo/mundo-*.bin`); se omiten si no está. Las pruebas por el WebSocket real están al final.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::Recurso::{self, *};
use base::{Punto, MS_POR_MINUTO};
use comercio::caravana::{Animal, EstadoCaravana, TipoCarro};
use comun::real::{campamento, en_tierra, mundo, sitio_rico, M};
use comun::sesion::*;
use contrato::v1::{self as pb, orden, TipoCliente};
use militar::catalogo::Tropa;
use militar::columna::Politica;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::{rechazos, Ajustes, Arranque, Comando, Dato, Nucleo, Peticion, Reglas, Resultado, Respuesta};

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

/// Los datos enteros de un hecho y su texto `sentido`, si lo lleva.
type Hecho = (Vec<(&'static str, i64)>, Option<String>);

/// Cada hecho `codigo`.
fn hechos(n: &Nucleo, codigo: &str) -> Vec<Hecho> {
    n.cambios_desde(0)
        .unwrap()
        .flat_map(|c| c.eventos.iter())
        .filter(|e| e.codigo == codigo)
        .map(|e| (e.datos.clone(), e.valores.iter().find_map(|(k, v)| if *k == "sentido" { if let Dato::Texto(s) = v { Some(s.clone()) } else { None } } else { None })))
        .collect()
}

fn dato(datos: &[(&'static str, i64)], clave: &str) -> i64 {
    datos.iter().find(|(k, _)| *k == clave).unwrap_or_else(|| panic!("falta «{clave}» en {datos:?}")).1
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
        self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento, controlador: Controlador::Humano }).creadas[0]
    }
    fn personal(&self, h: u64, r: Recurso) -> i64 {
        self.n.heroe(h).unwrap().almacen_personal.get(&r).copied().unwrap_or(0)
    }
    fn carro(&self, col: u64, r: Recurso) -> i64 {
        self.n.ejercito(col).unwrap().columna.carga.get(&r).copied().unwrap_or(0)
    }
    fn almacen(&self, plaza: u64, r: Recurso) -> i64 {
        self.n.economia(plaza).unwrap().almacen.cantidad(r)
    }
    fn contenido(&self, k: u64, r: Recurso) -> i64 {
        self.n.caravana(k).unwrap().contenido.get(&r).copied().unwrap_or(0)
    }
    /// La columna marcha hasta `destino` y llega.
    fn llevar(&mut self, cuenta: &str, col: u64, destino: Punto) {
        self.ok(cuenta, Comando::MarcharA { ejercito: col, destino });
        let fin = self.n.ejercito(col).unwrap().plan.fin().unwrap();
        self.n.avanzar_hasta(fin);
    }
    fn dar(&mut self, plaza: u64, recurso: Recurso, cantidad: i64) {
        let r = self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
    }
}

/// Funda en `p` una plaza de `faccion` con una columna de administración de `dueno`.
fn fundar(e: &mut Esc, dueno: &str, faccion: u32, p: Punto) -> u64 {
    let t = e.n.ahora();
    let col = e.n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap();
    e.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
    e.ok(dueno, Comando::Fundar { ejercito: col }).creadas[0]
}

// ───────────────────────────── 1. El almacén personal, en un campamento ─────────────────────────────

#[test]
fn el_almacen_personal_se_llena_y_se_vacia_con_el_carro_a_la_puerta_de_la_residencia_sin_crear_ni_perder_nada() {
    let m = con_mundo!();
    let mut e = Esc::nuevo(m);
    let ana = e.heroe("ana", 0);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: ana, faccion: Some(1) });
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: ana, recurso: Madera, cantidad: 300_000 });
    // Dentro del campamento no lleva columna: el almacén se toca con el carro (dentro está `vaciarCarro`).
    assert_eq!(e.no("ana", Comando::GuardarEnAlmacenPersonal { heroe: ana, recurso: Madera, cantidad: 10_000 }), rechazos::HEROE_SIN_COLUMNA);
    let col = e.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: ana, escuadras: vec![], carga: vec![(Madera, 200_000)], politica: Politica::Aceptar }).creadas[0];
    let madera = |e: &Esc| e.personal(ana, Madera) + e.carro(col, Madera);
    assert_eq!((e.personal(ana, Madera), e.carro(col, Madera), madera(&e)), (100_000, 200_000, 300_000));

    // Guarda lo pedido en unidades enteras (50,7 → 50) y lo saca hasta lo que cabe en el carro.
    e.ok("ana", Comando::GuardarEnAlmacenPersonal { heroe: ana, recurso: Madera, cantidad: 50_700 });
    assert_eq!((e.personal(ana, Madera), e.carro(col, Madera), madera(&e)), (150_000, 150_000, 300_000));
    let cap = e.n.capacidad_de_carga(col).unwrap();
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Piedra, cantidad: cap - 150_000 - 40_000 });
    e.ok("ana", Comando::SacarDelAlmacenPersonal { heroe: ana, recurso: Madera, cantidad: 999_999_000 });
    assert_eq!((e.personal(ana, Madera), e.carro(col, Madera), madera(&e)), (110_000, 190_000, 300_000), "solo cabían 40 en el carro");
    let h = hechos(&e.n, "heroe.almacen_personal");
    assert_eq!(h.iter().map(|(d, s)| (dato(d, "heroe") as u64, dato(d, "cantidad"), s.clone().unwrap())).collect::<Vec<_>>(), vec![(ana, 50_000, "guarda".into()), (ana, 40_000, "saca".into())]);
    assert_eq!(dato(&h[0].0, "recurso"), dato(&h[1].0, "recurso"));
    // Con el carro lleno no se saca; sin nada guardado de un recurso, tampoco.
    assert_eq!(e.no("ana", Comando::SacarDelAlmacenPersonal { heroe: ana, recurso: Madera, cantidad: 1_000 }), rechazos::CARRO_LLENO);
    assert_eq!(e.no("ana", Comando::SacarDelAlmacenPersonal { heroe: ana, recurso: Oro, cantidad: 1_000 }), rechazos::ALMACEN_SIN_EXISTENCIAS);

    // El tope del almacén es en total, de cualquier recurso: con 10 de hueco entran 10 de piedra y luego nada.
    let hueco = 1_000_000 - 110_000 - 10_000;
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: ana, recurso: Oro, cantidad: hueco });
    let piedra = |e: &Esc| e.personal(ana, Piedra) + e.carro(col, Piedra);
    let antes = piedra(&e);
    e.ok("ana", Comando::GuardarEnAlmacenPersonal { heroe: ana, recurso: Piedra, cantidad: 100_000 });
    assert_eq!((e.personal(ana, Piedra), piedra(&e)), (10_000, antes));
    assert_eq!(e.n.heroe(ana).unwrap().almacen_personal.values().sum::<i64>(), 1_000_000);
    assert_eq!(e.no("ana", Comando::GuardarEnAlmacenPersonal { heroe: ana, recurso: Piedra, cantidad: 1_000 }), rechazos::ALMACEN_PERSONAL_LLENO);
    assert_eq!(e.no("ana", Comando::GuardarEnAlmacenPersonal { heroe: ana, recurso: Estano, cantidad: 1_000 }), rechazos::CARRO_NO_LLEVA);
    assert_eq!(e.no("ana", Comando::GuardarEnAlmacenPersonal { heroe: ana, recurso: Madera, cantidad: 999 }), rechazos::CANTIDAD_NO_POSITIVA, "menos de una unidad");
    assert_eq!(e.no("ana", Comando::GuardarEnAlmacenPersonal { heroe: ana, recurso: Madera, cantidad: -5_000 }), rechazos::CANTIDAD_NO_POSITIVA);

    // Solo el Líder (el carro es común): beto se une en campo y no puede.
    let beto = e.heroe("beto", 0);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: beto, faccion: Some(1) });
    e.ok("beto", Comando::SalirDelCampamento { campamento: 0, heroe: beto, escuadras: vec![], carga: vec![], politica: Politica::Rechazar });
    e.ok("beto", Comando::UnirseEnCampo { heroe: beto, ejercito: col });
    assert_eq!(e.n.heroe(beto).unwrap().heroe.presencia.ubicacion, Ubicacion::Columna(col));
    assert_eq!(e.no("beto", Comando::GuardarEnAlmacenPersonal { heroe: beto, recurso: Madera, cantidad: 1_000 }), rechazos::COLUMNA_SOLO_EL_LIDER);
    assert_eq!(e.no("beto", Comando::SacarDelAlmacenPersonal { heroe: ana, recurso: Oro, cantidad: 1_000 }), rechazos::NO_AUTORIZADO, "el héroe no es suyo");

    // Lejos de la puerta de su residencia, no; y sin residencia no hay almacén.
    let lejos = en_tierra(&m.0, campamento(&m.0, 0), 30_000, 0);
    e.llevar("ana", col, lejos);
    assert_eq!(e.no("ana", Comando::SacarDelAlmacenPersonal { heroe: ana, recurso: Oro, cantidad: 1_000 }), rechazos::ALMACEN_PERSONAL_FUERA_DE_TU_RESIDENCIA);
    assert_eq!(e.no("ana", Comando::GuardarEnAlmacenPersonal { heroe: ana, recurso: Madera, cantidad: 1_000 }), rechazos::ALMACEN_PERSONAL_FUERA_DE_TU_RESIDENCIA);
    e.ok("admin", Comando::FijarResidencia { heroe: ana, residencia: None });
    assert_eq!(e.no("ana", Comando::GuardarEnAlmacenPersonal { heroe: ana, recurso: Madera, cantidad: 1_000 }), rechazos::HEROE_SIN_RESIDENCIA);
}

#[test]
fn si_reside_en_una_plaza_su_almacen_personal_esta_a_la_puerta_de_esa_plaza() {
    let m = con_mundo!();
    let mut e = Esc::nuevo(m);
    let p = sitio_rico(&e.n, &m.0, 0, 1);
    let a = fundar(&mut e, "ana", 1, p);
    let ana = e.heroe("ana", 0);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: ana, faccion: Some(1) });
    e.ok("admin", Comando::FijarResidencia { heroe: ana, residencia: Some(Residencia::Asentamiento(a as u32)) });
    e.ok("admin", Comando::ColocarHeroe { heroe: ana, ubicacion: Ubicacion::Asentamiento(a as u32) });
    e.dar(a, Madera, 20_000);
    let col = e.ok("ana", Comando::SalirAlMundo { asentamiento: a, heroe: ana, escuadras: vec![], carga: vec![(Madera, 20_000)] }).creadas[0];
    e.ok("ana", Comando::GuardarEnAlmacenPersonal { heroe: ana, recurso: Madera, cantidad: 15_000 });
    assert_eq!((e.personal(ana, Madera), e.carro(col, Madera)), (15_000, 5_000));
    // La puerta del campamento donde nació ya no es su residencia.
    e.llevar("ana", col, campamento(&m.0, 0));
    assert_eq!(e.no("ana", Comando::SacarDelAlmacenPersonal { heroe: ana, recurso: Madera, cantidad: 1_000 }), rechazos::ALMACEN_PERSONAL_FUERA_DE_TU_RESIDENCIA);
}

// ───────────────────────────── 2. Cargar, soltar y reclamar caravanas ─────────────────────────────

/// Una plaza de la Facción 1 con Mercado activo, «ana» residente con una milicia y su ejército a la puerta con una caravana de la plaza enganchada; y otra plaza de la Facción 1 lejos.
struct ConCaravana {
    e: Esc,
    a: u64,
    c: u64,
    ana: u64,
    ej: u64,
    k: u64,
}

fn con_caravana(m: &M) -> ConCaravana {
    let mut e = Esc::nuevo(m);
    let pa = sitio_rico(&e.n, &m.0, 0, 1);
    let a = fundar(&mut e, "ana", 1, pa);
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 });
    let pc = sitio_rico(&e.n, &m.0, 2, 1);
    let c = fundar(&mut e, "ana", 1, pc);
    e.avanzar(24 * HORA);
    e.dar(a, Madera, 350_000);
    e.ok("admin", Comando::AnadirEdificio { asentamiento: a, tipo: Tipo::Mercado });
    e.avanzar(9 * HORA);
    e.dar(a, Cobre, 100_000);
    e.dar(a, Madera, 100_000);
    let k = e.ok("ana", Comando::CrearCaravana { asentamiento: a }).creadas[0];
    e.ok("ana", Comando::ReservarCaravana { caravana: k, reservada: true });
    e.ok("ana", Comando::AnadirCarro { caravana: k, tipo: TipoCarro::Basico });
    e.ok("ana", Comando::AnadirAnimal { caravana: k, carro: 0, animal: Animal::Buey });
    let ana = e.heroe("ana", 0);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: ana, faccion: Some(1) });
    e.ok("admin", Comando::FijarResidencia { heroe: ana, residencia: Some(Residencia::Asentamiento(a as u32)) });
    e.ok("admin", Comando::ColocarHeroe { heroe: ana, ubicacion: Ubicacion::Asentamiento(a as u32) });
    let milicia = e.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: ana, tropa: Tropa::MiliciaLanceros }).creadas[0];
    let ej = e.ok("ana", Comando::MovilizarEjercito { asentamiento: a, heroe: ana, escuadras: vec![milicia], carga: vec![], politica: Politica::Aceptar }).creadas[0];
    ConCaravana { e, a, c, ana, ej, k }
}

#[test]
fn cargar_una_caravana_adjunta_saca_del_almacen_de_la_plaza_a_cuya_puerta_esta_lo_que_le_cabe() {
    let m = con_mundo!();
    let ConCaravana { mut e, a, c, ej, k, .. } = con_caravana(m);
    let cargar = |recurso, cantidad, asentamiento| Comando::CargarCaravana { ejercito: ej, caravana: k, asentamiento, recurso, cantidad };
    assert_eq!(e.no("ana", cargar(Piedra, 5_000, a)), rechazos::CARAVANA_NO_ESTA_ADJUNTA, "aún no va con la columna");
    e.ok("ana", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });

    e.dar(a, Piedra, 50_000);
    let piedra = |e: &Esc| e.almacen(a, Piedra) + e.contenido(k, Piedra);
    let antes = (e.almacen(a, Piedra), piedra(&e));
    e.ok("ana", cargar(Piedra, 20_400, a));
    assert_eq!((e.almacen(a, Piedra), e.contenido(k, Piedra), piedra(&e)), (antes.0 - 20_000, 20_000, antes.1), "en unidades enteras, nada se crea ni se pierde");
    let h = hechos(&e.n, "ejercito.caravana_cargada");
    assert_eq!((dato(&h[0].0, "caravana") as u64, dato(&h[0].0, "plaza") as u64, dato(&h[0].0, "cargado")), (k, a, 20_000));

    // Rechazos: menos de una unidad, sin existencias, otra plaza lejos, otra Facción y quien no va en la columna.
    assert_eq!(e.no("ana", cargar(Piedra, 500, a)), rechazos::CANTIDAD_NO_POSITIVA);
    assert_eq!(e.almacen(a, LingoteBronce), 0);
    assert_eq!(e.no("ana", cargar(LingoteBronce, 5_000, a)), rechazos::ALMACEN_SIN_EXISTENCIAS);
    assert_eq!(e.no("ana", cargar(Piedra, 5_000, c)), rechazos::ASENTAMIENTO_FUERA_DE_LA_PUERTA);
    let pz = sitio_rico(&e.n, &m.0, 1, 2);
    let ajena = fundar(&mut e, "zed", 2, pz);
    e.dar(ajena, Piedra, 50_000);
    assert_eq!(e.no("admin", cargar(Piedra, 5_000, ajena)), rechazos::CARAVANA_PLAZA_AJENA, "una plaza de otra Facción no le abre su almacén");
    assert_eq!(e.no("zed", cargar(Piedra, 5_000, a)), rechazos::NO_AUTORIZADO);
    assert_eq!(e.no("ana", Comando::CargarCaravana { ejercito: 999, caravana: k, asentamiento: a, recurso: Piedra, cantidad: 5_000 }), rechazos::EJERCITO_DESCONOCIDO);
    assert_eq!(e.no("ana", Comando::CargarCaravana { ejercito: ej, caravana: 999, asentamiento: a, recurso: Piedra, cantidad: 5_000 }), rechazos::CARAVANA_DESCONOCIDA);

    // Se llena hasta su capacidad y entonces se rechaza; lo cargado sigue siendo lo que salió del almacén.
    let mut llena = false;
    for _ in 0..40 {
        e.dar(a, Piedra, 500_000);
        let r = e.ord("ana", cargar(Piedra, 999_000_000, a));
        if !r.aceptada() {
            assert_eq!(codigo(&r), rechazos::CARAVANA_SIN_CAPACIDAD);
            llena = true;
            break;
        }
    }
    assert!(llena, "la caravana tiene un tope");
    let cargado: i64 = hechos(&e.n, "ejercito.caravana_cargada").iter().map(|(d, _)| dato(d, "cargado")).sum();
    assert_eq!(e.contenido(k, Piedra), cargado);
}

#[test]
fn soltar_una_caravana_la_aparca_en_la_plaza_de_su_faccion_donde_esta_la_columna_y_en_mundo_abierto_no() {
    let m = con_mundo!();
    let ConCaravana { mut e, a, ana, ej, k, .. } = con_caravana(m);
    let soltar = |heroe| Comando::SoltarCaravana { ejercito: ej, caravana: k, heroe };
    assert_eq!(e.no("ana", soltar(ana)), rechazos::CARAVANA_NO_ESTA_ADJUNTA);
    e.ok("ana", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });
    e.dar(a, Piedra, 30_000);
    e.ok("ana", Comando::CargarCaravana { ejercito: ej, caravana: k, asentamiento: a, recurso: Piedra, cantidad: 30_000 });
    let beto = e.heroe("beto", 0);
    assert_eq!(e.no("ana", soltar(beto)), rechazos::NO_AUTORIZADO, "el héroe de otro");
    assert_eq!(e.no("beto", soltar(beto)), rechazos::NO_AUTORIZADO, "quien no va en la columna");

    // Lejos de toda plaza de su Facción, no se suelta.
    let pos = e.n.estado().asentamientos[&a].posicion;
    let lejos = en_tierra(&m.0, pos, 30_000, 0);
    e.llevar("ana", ej, lejos);
    assert_eq!(e.no("ana", soltar(ana)), rechazos::CARAVANA_FUERA_DE_LA_PUERTA);
    // A la puerta de la plaza, sí: queda aparcada allí con lo que llevaba, y la columna sigue sin ella.
    e.llevar("ana", ej, pos);
    e.ok("ana", soltar(ana));
    let kk = e.n.caravana(k).unwrap();
    assert_eq!((kk.estado, kk.anfitriona.map(u64::from), e.contenido(k, Piedra)), (EstadoCaravana::Aparcada, Some(a), 30_000));
    assert!(e.n.ejercito(ej).unwrap().caravanas_adjuntas.is_empty());
    let h = hechos(&e.n, "ejercito.caravana_soltada");
    assert_eq!((h.len(), dato(&h[0].0, "caravana") as u64, dato(&h[0].0, "plaza") as u64, dato(&h[0].0, "heroe") as u64), (1, k, a, ana));
    assert!(hechos(&e.n, "ejercito.caravana_aparcada").is_empty(), "soltar tiene su propio hecho");
    assert_eq!(e.no("ana", soltar(ana)), rechazos::CARAVANA_NO_ESTA_ADJUNTA);
}

#[test]
fn reclamar_solo_vale_para_la_caravana_de_fundacion_que_vuelve_sola() {
    let m = con_mundo!();
    let ConCaravana { mut e, ana, k, .. } = con_caravana(m);
    let reclamar = |heroe, caravana| Comando::ReclamarCaravana { heroe, caravana, politica: Politica::Aceptar };
    assert_eq!(e.no("ana", reclamar(ana, k)), rechazos::CARAVANA_NO_RECLAMABLE, "una caravana comercial no se reclama");
    assert_eq!(e.no("ana", reclamar(ana, 999)), rechazos::CARAVANA_DESCONOCIDA);
    assert_eq!(e.no("beto", reclamar(ana, k)), rechazos::NO_AUTORIZADO);
    let beto = e.heroe("beto", 0);
    assert_eq!(e.no("beto", reclamar(beto, k)), rechazos::HEROE_SIN_COLUMNA, "dentro de un campamento no se reclama nada");
}

// ───────────────────────────── 3. Por el WebSocket real ─────────────────────────────

fn heroes(entidades: impl IntoIterator<Item = Option<pb::Entidad>>) -> Vec<pb::Heroe> {
    entidades.into_iter().flatten().filter_map(|e| if let Some(pb::entidad::Cuerpo::Heroe(h)) = e.cuerpo { Some(h) } else { None }).collect()
}

fn madera_de(h: &pb::Heroe) -> i64 {
    h.almacen_personal.as_ref().and_then(|c| c.por_id.get("madera")).copied().unwrap_or(0)
}

#[tokio::test]
async fn por_la_sesion_el_almacen_personal_se_guarda_y_se_saca_y_el_heroe_lo_ve_con_su_capacidad() {
    let Some(m) = mundo() else {
        eprintln!("sin artefacto de mundo real: se omite");
        return;
    };
    let mut e = Esc::nuevo(m);
    let ana = e.heroe("ana", 0);
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: ana, recurso: Madera, cantidad: 100_000 });
    e.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: ana, escuadras: vec![], carga: vec![(Madera, 80_000)], politica: Politica::Rechazar });
    let w = arrancar_con(e.n).await;
    let (mut c, f) = w.entrar("ana", TipoCliente::ClienteUnity).await;
    let h = heroes(f.entidades.into_iter().map(Some));
    assert_eq!((h.len(), madera_de(&h[0]), h[0].capacidad_almacen_personal), (1, 20_000, 1_000_000), "el héroe propio con su almacén y su capacidad");

    let guardar = orden::Cuerpo::GuardarEnAlmacenPersonal(pb::OrdenGuardarEnAlmacenPersonal { recurso: "madera".into(), cantidad: 50_000 });
    let (deltas, r) = dar(&mut c, "g1", guardar).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    let d = del_heroe(&deltas);
    assert_eq!(heroes(d.iter().flat_map(|d| d.cambios.iter().map(|x| x.datos.clone()))).iter().map(madera_de).collect::<Vec<_>>(), vec![70_000]);
    assert!(d.iter().flat_map(|d| &d.eventos).any(|x| x.codigo == "heroe.almacen_personal"), "el hecho llega al héroe");

    let sacar = orden::Cuerpo::SacarDelAlmacenPersonal(pb::OrdenSacarDelAlmacenPersonal { recurso: "madera".into(), cantidad: 30_000 });
    let (deltas, r) = dar(&mut c, "s1", sacar).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    assert_eq!(heroes(del_heroe(&deltas).iter().flat_map(|d| d.cambios.iter().map(|x| x.datos.clone()))).iter().map(madera_de).collect::<Vec<_>>(), vec![40_000]);

    let piedra = orden::Cuerpo::GuardarEnAlmacenPersonal(pb::OrdenGuardarEnAlmacenPersonal { recurso: "piedra".into(), cantidad: 1_000 });
    let (_, r) = dar(&mut c, "g2", piedra).await;
    assert_eq!(rechazo(&r), "carro.no_lleva");
    let reclamar = orden::Cuerpo::ReclamarCaravana(pb::OrdenReclamarCaravana { heroe_id: format!("heroe-{ana}"), caravana_id: "car-9".into(), politica_de_union: None });
    let (_, r) = dar(&mut c, "r1", reclamar).await;
    assert_eq!(rechazo(&r), "caravana.desconocida", "ya no es `orden.no_disponible`");
    let n = w.parar().await;
    assert_eq!(n.heroe(ana).unwrap().almacen_personal.get(&Madera), Some(&40_000));
}

#[tokio::test]
async fn por_la_sesion_se_carga_y_se_suelta_una_caravana_adjunta() {
    let Some(m) = mundo() else {
        eprintln!("sin artefacto de mundo real: se omite");
        return;
    };
    let ConCaravana { mut e, a, ana, ej, k, .. } = con_caravana(m);
    e.ok("ana", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });
    e.dar(a, Piedra, 40_000);
    let w = arrancar_con(e.n).await;
    let (mut c, _) = w.entrar("ana", TipoCliente::ClienteUnity).await;
    let (ejs, ks, as_) = (format!("ej-{ej}"), format!("car-{k}"), format!("as-{a}"));
    let cargar = orden::Cuerpo::CargarCaravana(pb::OrdenCargarCaravana { ejercito_id: ejs.clone(), caravana_id: ks.clone(), asentamiento_id: as_, recurso: "piedra".into(), cantidad: 25_000 });
    let (deltas, r) = dar(&mut c, "c1", cargar).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    let caravanas: Vec<pb::Caravana> = deltas.iter().flat_map(|d| &d.cambios).filter_map(|x| x.datos.as_ref()).filter_map(|e| if let Some(pb::entidad::Cuerpo::Caravana(k)) = &e.cuerpo { Some(k.clone()) } else { None }).collect();
    assert!(!caravanas.is_empty(), "la caravana cargada llega a quien la lleva");
    assert!(deltas.iter().flat_map(|d| &d.eventos).any(|x| x.codigo == "ejercito.caravana_cargada"));
    let soltar = orden::Cuerpo::SoltarCaravana(pb::OrdenSoltarCaravana { ejercito_id: ejs, caravana_id: ks, heroe_id: format!("heroe-{ana}") });
    let (deltas, r) = dar(&mut c, "c2", soltar).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    assert!(deltas.iter().flat_map(|d| &d.eventos).any(|x| x.codigo == "ejercito.caravana_soltada"));
    let n = w.parar().await;
    let kk = n.caravana(k).unwrap();
    assert_eq!((kk.estado, kk.contenido.get(&Piedra).copied()), (EstadoCaravana::Aparcada, Some(25_000)));
}
