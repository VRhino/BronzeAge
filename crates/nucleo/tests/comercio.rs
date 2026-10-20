//! El comercio dentro del núcleo (fase 3B-1, Comercio_Caravanas): trueques, órdenes de mercado y mostrador contra el almacén real de las
//! plazas, flota de caravanas (sin movimiento), reputación entre Facciones, plazos como eventos de la cola, cierre por ruina y conquista,
//! determinismo y recuperación desde foto. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas que lo
//! necesitan se omiten si no está.
//!
//! Los almacenes caben 400 de cada recurso (`ALMACEN.capacidadInicialPorRecurso`), así que las cantidades de las pruebas son pequeñas.

use asentamiento::catalogo::Tipo;
use asentamiento::datos::UNO;
use asentamiento::Recurso::{self, *};
use base::{Instante, Punto, MS_POR_MINUTO};
use comercio::caravana::{Animal, EstadoCaravana, TipoCarro};
use comercio::orden_mercado::EstadoOrden;
use comercio::precio::{comision_de_entrega, recargo_por_reputacion, ContextoComision, TablaPrecios};
use comercio::trueque::EstadoTrueque;
use comercio::{Carga as Mercancia, Lado};
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{rechazos, Ajustes, Arranque, Comando, Estado, Nucleo, Peticion, Reglas, Resultado, Respuesta};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;
const DIA: i64 = 24 * HORA;
const PUNTO: i64 = 1_000_000;

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

fn arranque(m: &M, semilla: u64) -> Arranque {
    Arranque { reglas: Reglas::default(), ajustes: Ajustes::default(), semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

/// Un sitio verde con mucho bosque a 10 u (al menos 6 parcelas) cerca del campamento `i`: una plaza que no cae en ruinas por falta de madera.
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

fn codigo(r: &Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

/// Hechos de crónica con ese código: `(instante, plazas, datos)`.
type Hecho = (Instante, Vec<u64>, Vec<(&'static str, i64)>);

fn hechos(n: &Nucleo, codigo: &str) -> Vec<Hecho> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).map(|e| (e.en, e.asentamientos.clone(), e.datos.clone())).collect()
}

fn dato(h: &Hecho, clave: &str) -> i64 {
    h.2.iter().find(|(k, _)| *k == clave).unwrap_or_else(|| panic!("el hecho no lleva «{clave}»: {h:?}")).1
}

/// Todo el oro del mundo: almacenes de las plazas, carros de las columnas y lo que llevan las caravanas.
fn oro(n: &Nucleo) -> i64 {
    mundo_de(n, Oro)
}

/// Todo lo que hay de un recurso: almacenes, carros de las columnas y caravanas.
fn mundo_de(n: &Nucleo, r: Recurso) -> i64 {
    let e = n.estado();
    e.asentamientos.values().map(|a| a.economia.almacen.cantidad(r)).sum::<i64>()
        + e.ejercitos.values().map(|c| c.suministro.get(&r).copied().unwrap_or(0)).sum::<i64>()
        + e.comercio.flota.iter().map(|c| c.contenido.get(&r).copied().unwrap_or(0)).sum::<i64>()
}

fn cantidad(n: &Nucleo, plaza: u64, r: Recurso) -> i64 {
    n.economia(plaza).unwrap().almacen.cantidad(r)
}

fn pos(n: &Nucleo, plaza: u64) -> Punto {
    n.estado().asentamientos[&plaza].posicion
}

/// Dos plazas de Facciones distintas (F1 con «ana», F2 con «beto»), la auto-construcción pausada y, si se pide, un Mercado activo. El
/// tiempo corre solo cuando se pide (`avanzar`); las órdenes se dan en el instante actual salvo las `*_en`.
struct Escenario {
    n: Nucleo,
    a: u64,
    b: u64,
    cuenta: u64,
}

impl Escenario {
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
        let (t, p) = (self.n.ahora(), self.peticion(actor, c));
        self.n.aplicar_en(t, p)
    }
    /// La orden en un instante concreto (el núcleo procesa los eventos hasta él antes de aplicarla).
    fn ok_en(&mut self, t: Instante, actor: &str, c: Comando) -> Respuesta {
        let p = self.peticion(actor, c);
        let r = self.n.aplicar_en(t, p);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    /// Da la orden (como administración si `actor` es «admin») y exige que se acepte.
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    fn rechazada(&mut self, actor: &str, c: Comando) -> &'static str {
        let r = self.ord(actor, c);
        codigo(&r)
    }
    fn avanzar(&mut self, ms: i64) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    fn avanzar_hasta(&mut self, t: Instante) {
        self.n.avanzar_hasta(t);
    }
    /// Una columna a una unidad de la puerta de la plaza (dentro del radio de la puerta).
    fn columna_en(&mut self, nombre: &str, dueno: &str, faccion: u32, plaza: u64) -> u64 {
        let p = pos(&self.n, plaza);
        let t = self.n.ahora();
        self.n.crear_ejercito(t, nombre, dueno, faccion, Punto::new(p.x + 1_000, p.y), 500, 100_000, 100_000).unwrap()
    }
    /// Añade recursos al almacén de la plaza (si ya está lleno no cambia nada, y no importa).
    fn dar(&mut self, plaza: u64, recurso: Recurso, cantidad: i64) {
        let r = self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
    }
    /// Crea en `origen` una caravana con `carros` carros con buey, la engancha a `columna` y la deja aparcada en `anfitriona`.
    fn caravana_aparcada_en(&mut self, actor: &str, origen: u64, columna: u64, anfitriona: u64, carros: u32) -> u64 {
        let id = self.ok(actor, Comando::CrearCaravana { asentamiento: origen }).creadas[0];
        // Reservada: el reparto automático no se la lleva mientras se monta.
        self.ok(actor, Comando::ReservarCaravana { caravana: id, reservada: true });
        for k in 0..carros {
            self.ok(actor, Comando::AnadirCarro { caravana: id, tipo: TipoCarro::Basico });
            self.ok(actor, Comando::AnadirAnimal { caravana: id, carro: k, animal: Animal::Buey });
        }
        self.ok("admin", Comando::AdjuntarCaravana { caravana: id, ejercito: columna });
        self.ok("admin", Comando::AparcarCaravana { caravana: id, asentamiento: anfitriona });
        id
    }
    /// Como `caravana_aparcada_en`, aparcada en su propia plaza.
    fn caravana_aparcada(&mut self, actor: &str, plaza: u64, columna: u64, carros: u32) -> u64 {
        self.caravana_aparcada_en(actor, plaza, columna, plaza, carros)
    }
    fn cargar(&mut self, actor: &str, caravana: u64, recurso: Recurso, cantidad: i64) {
        self.ok(actor, Comando::MoverCargaAparcada { caravana, recurso, cantidad, cargar: true });
    }
    /// Un trueque de A a B propuesto y aceptado.
    fn trueque(&mut self, a: &[(Recurso, i64)], b: &[(Recurso, i64)]) -> u64 {
        let id = self.trueque_sin_aceptar_con(a, b);
        self.ok("beto", Comando::AceptarTrueque { acuerdo: id });
        id
    }
    fn trueque_sin_aceptar_con(&mut self, a: &[(Recurso, i64)], b: &[(Recurso, i64)]) -> u64 {
        let (pa, pb) = (self.a, self.b);
        self.ok("ana", Comando::ProponerTrueque { a: pa, b: pb, lineas_a: a.to_vec(), lineas_b: b.to_vec() }).creadas[0]
    }
    fn trueque_sin_aceptar(&mut self) -> u64 {
        self.trueque_sin_aceptar_con(&[(Madera, 100_000), (Piedra, 50_000)], &[(Cobre, 30_000)])
    }
}

fn fundar(n: &mut Nucleo, t: Instante, nombre: &str, dueno: &str, faccion: u32, p: Punto) -> u64 {
    let e = n.crear_ejercito(t, nombre, dueno, faccion, p, 500, 100_000, 100_000).unwrap();
    assert!(n.aplicar_en(t, Peticion::nueva("admin", &format!("c-{nombre}"), Comando::DarCaravanaFundacion { ejercito: e }).de_admin()).aceptada());
    let r = n.aplicar_en(t, Peticion::nueva(dueno, &format!("f-{nombre}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

/// Funda las dos plazas en `n` y las deja, si se pide, con su Mercado construido. `madera` es lo que se le da a cada plaza (además de lo que
/// ya trae). Las plazas construyen solas su Leñera (sin ella, el mantenimiento se come la madera y caen en ruinas a las 26 horas, poco más
/// de un día de vida); con `pausar_a` la plaza A no construye nada y es la que cae.
fn montar(mut n: Nucleo, m: &M, con_mercado: bool, madera: i64, pausar_a: bool) -> Escenario {
    let (pa, pb) = (sitio_rico(&n, &m.0, 0, 1), sitio_rico(&n, &m.0, 3, 2));
    let (a, b) = (fundar(&mut n, 0, "a", "ana", 1, pa), fundar(&mut n, 0, "b", "beto", 2, pb));
    let mut e = Escenario { n, a, b, cuenta: 0 };
    if pausar_a {
        e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
    }
    if con_mercado {
        for (plaza, madera) in [(a, madera), (b, 350_000)] {
            e.dar(plaza, Madera, madera);
            e.ok("admin", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
        }
        e.avanzar(9 * HORA);
        for plaza in [a, b] {
            assert!(e.n.economia(plaza).unwrap().edificios.iter().any(|x| x.tipo == Tipo::Mercado && x.activo()), "el Mercado de as-{plaza} debería estar activo");
        }
    }
    e
}

fn escenario_con(m: &M, semilla: u64, con_mercado: bool, madera: i64, pausar_a: bool) -> Escenario {
    montar(Nucleo::nuevo(arranque(m, semilla)), m, con_mercado, madera, pausar_a)
}

fn escenario(m: &M, semilla: u64, con_mercado: bool) -> Escenario {
    escenario_con(m, semilla, con_mercado, 350_000, false)
}

/// Una tercera plaza de F1 (con Mercado construido) cerca del campamento 1.
fn tercera_plaza(e: &mut Escenario, m: &M) -> u64 {
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 });
    let pc = sitio_rico(&e.n, &m.0, 1, 1);
    let t = e.n.ahora();
    let c = fundar(&mut e.n, t, "c", "ana", 1, pc);
    e.dar(c, Madera, 350_000);
    e.ok("admin", Comando::AnadirEdificio { asentamiento: c, tipo: Tipo::Mercado });
    e.avanzar(9 * HORA);
    c
}

/// La comisión que debe acuñar la entrega de `carga` de `origen` a `destino` con el estado de ahora (precios con el stock de ahora).
fn comision_esperada(n: &Nucleo, origen: u64, destino: u64, carga: &[(Recurso, i64)]) -> i64 {
    let r = comercio::Reglas::cargar();
    let e = n.estado();
    let precios = TablaPrecios::de(&r.precio, e.asentamientos.values().map(|a| &a.economia.almacen));
    let c: Mercancia = carga.iter().copied().collect();
    let (po, pd) = (&e.asentamientos[&origen], &e.asentamientos[&destino]);
    comision_de_entrega(
        &r.comision,
        precios.valor(&c),
        &ContextoComision { mismo_asentamiento: false, misma_faccion: po.faccion == pd.faccion, factor_comision_externa: UNO, distancia_recta: po.posicion.dist(pd.posicion) },
    )
}

/// Lo que debe acuñar y cobrar de más una entrega: `(comisión, recargo)` efectivos, con el tope de oro del almacén de destino (la comisión
/// se topa por su capacidad y el recargo por lo que deja la comisión: §15) y el oro del origen.
fn entrega_esperada(n: &Nucleo, origen: u64, destino: u64, carga: &[(Recurso, i64)]) -> (i64, i64) {
    let r = comercio::Reglas::cargar();
    let calculada = comision_esperada(n, origen, destino, carga);
    let (po, pd) = (&n.estado().asentamientos[&origen], &n.estado().asentamientos[&destino]);
    let espacio = pd.economia.almacen.espacio(Oro);
    let comision = calculada.min(espacio);
    let debido = recargo_por_reputacion(&r.reputacion, calculada, po.faccion == pd.faccion, n.reputacion(po.faccion), po.economia.almacen.cantidad(Oro));
    (comision, debido.min(espacio - comision))
}

// ───────────────────────────── 1. Un trueque entero con entrega manual ─────────────────────────────

#[test]
fn un_trueque_entero_entre_facciones_con_entrega_manual_se_cumple_con_comision_y_reputacion() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, true);
    let (a, b) = (e.a, e.b);
    // Las columnas que hacen la entrega: la de Ana a la puerta de B (recibe lo de A) y la de Beto a la puerta de A.
    let col_ana_en_b = e.columna_en("ana-en-b", "ana", 1, b);
    let col_beto_en_a = e.columna_en("beto-en-a", "beto", 2, a);
    let col_ana_en_a = e.columna_en("ana-en-a", "ana", 1, a);
    e.dar(a, Cobre, 100_000);
    e.dar(b, Estano, 100_000);
    let ca = e.caravana_aparcada("ana", a, col_ana_en_b, 1);
    e.cargar("ana", ca, Cobre, 100_000);
    let cb = e.caravana_aparcada("beto", b, col_beto_en_a, 1);
    e.cargar("beto", cb, Estano, 30_000);

    // Proponer: solo quien manda en A (no el de B ni un extraño); aceptar: solo quien manda en B.
    let propuesta = Comando::ProponerTrueque { a, b, lineas_a: vec![(Cobre, 100_000)], lineas_b: vec![(Estano, 30_000)] };
    assert_eq!(e.rechazada("beto", propuesta.clone()), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("luis", propuesta.clone()), rechazos::NO_AUTORIZADO);
    let tr = e.ok("ana", propuesta).creadas[0];
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Propuesto);
    assert_eq!(e.rechazada("ana", Comando::AceptarTrueque { acuerdo: tr }), rechazos::NO_AUTORIZADO, "el sí es de la plaza B");
    assert_eq!(e.rechazada("beto", Comando::AceptarTrueque { acuerdo: 99 }), rechazos::TRUEQUE_DESCONOCIDO);
    e.ok("beto", Comando::AceptarTrueque { acuerdo: tr });
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Activo);
    assert_eq!(e.n.acuerdo(tr).unwrap().aceptado_en, Some(e.n.ahora()));
    assert_eq!(e.rechazada("beto", Comando::RechazarTrueque { acuerdo: tr }), rechazos::TRUEQUE_NO_ESTA_PROPUESTO, "se contesta una vez");
    assert_eq!(hechos(&e.n, "comercio.trueque_propuesto").len(), 1);
    assert_eq!(hechos(&e.n, "comercio.trueque_aceptado").len(), 1);

    // La entrega manual de A: antes, los rechazos.
    let entrega = |ejercito, caravana| Comando::EntregarDeCaravana { ejercito, caravana, acuerdo: tr };
    assert_eq!(e.rechazada("ana", entrega(col_ana_en_b, ca)), rechazos::CARAVANA_NO_ESTA_ADJUNTA, "la caravana está aparcada");
    e.ok("admin", Comando::AdjuntarCaravana { caravana: ca, ejercito: col_ana_en_a });
    assert_eq!(e.rechazada("ana", entrega(col_ana_en_a, ca)), rechazos::ENTREGA_FUERA_DE_LA_PUERTA, "la columna está en la puerta de A, y A es quien envía");
    e.ok("admin", Comando::AparcarCaravana { caravana: ca, asentamiento: a });
    e.ok("admin", Comando::AdjuntarCaravana { caravana: ca, ejercito: col_ana_en_b });
    assert_eq!(e.rechazada("beto", entrega(col_ana_en_b, ca)), rechazos::NO_AUTORIZADO, "la columna es de Ana");
    assert_eq!(e.rechazada("ana", entrega(col_ana_en_a, ca)), rechazos::CARAVANA_NO_ESTA_ADJUNTA, "va con otra columna");
    assert_eq!(e.rechazada("ana", Comando::EntregarDeCaravana { ejercito: col_ana_en_b, caravana: ca, acuerdo: 99 }), rechazos::TRUEQUE_DESCONOCIDO);

    let (oro0, cobre_b0) = (oro(&e.n), cantidad(&e.n, b, Cobre));
    let (comision_a, recargo_a) = entrega_esperada(&e.n, a, b, &[(Cobre, 100_000)]);
    assert!(comision_a > 0 && recargo_a == 0);
    e.ok("ana", entrega(col_ana_en_b, ca));
    assert_eq!(oro(&e.n) - oro0, comision_a, "la comisión se acuña: es lo único que cambia el oro del mundo");
    assert_eq!(cantidad(&e.n, b, Cobre) - cobre_b0, 100_000, "el cobre llega entero al almacén de B");
    assert!(e.n.caravana(ca).unwrap().contenido.is_empty());
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Activo, "falta lo de B");
    assert_eq!(e.n.acuerdo(tr).unwrap().pendiente(Lado::A), vec![]);
    let escoltada = hechos(&e.n, "comercio.entrega_escoltada");
    assert_eq!((escoltada.len(), dato(&escoltada[0], "comision")), (1, comision_a));
    assert_eq!(e.rechazada("ana", entrega(col_ana_en_b, ca)), rechazos::ENTREGA_NO_DEBE, "ya pagó su lado");

    // La entrega de B cierra el trueque.
    e.ok("admin", Comando::AdjuntarCaravana { caravana: cb, ejercito: col_beto_en_a });
    let (oro1, estano_a0) = (oro(&e.n), cantidad(&e.n, a, Estano));
    let (comision_b, recargo_b) = entrega_esperada(&e.n, b, a, &[(Estano, 30_000)]);
    assert!(comision_b > 0 && recargo_b == 0);
    e.ok("beto", Comando::EntregarDeCaravana { ejercito: col_beto_en_a, caravana: cb, acuerdo: tr });
    assert_eq!(oro(&e.n) - oro1, comision_b);
    assert_eq!(cantidad(&e.n, a, Estano) - estano_a0, 30_000);
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Cumplido);
    let cumplidos = hechos(&e.n, "comercio.trueque_cumplido");
    assert_eq!(cumplidos.len(), 1);
    assert_eq!((dato(&cumplidos[0], "entreFacciones"), dato(&cumplidos[0], "cobre"), dato(&cumplidos[0], "estano"), dato(&cumplidos[0], "trasVencer")), (1, 100_000, 30_000, 0));
    // Entre Facciones: +5 de reputación y la experiencia para las dos.
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (5 * PUNTO, 5 * PUNTO));
    let xp: Vec<i64> = hechos(&e.n, "faccion.experiencia").iter().map(|h| dato(h, "faccion")).collect();
    assert_eq!(xp, vec![1, 2]);
    // La reputación decae despacio: 0,002 por minuto.
    e.avanzar(1_000 * MIN);
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (3 * PUNTO, 3 * PUNTO));
    // El plazo del trueque ya no caduca nada: su evento se invalidó al cerrarse.
    e.avanzar(2 * DIA);
    assert!(hechos(&e.n, "comercio.trueque_expirado").is_empty());
    assert_eq!(e.n.acuerdos_de(a).len(), 1);
}

#[test]
fn un_trueque_entre_plazas_de_la_misma_faccion_cobra_el_3_por_ciento_y_no_toca_la_reputacion() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, true);
    let a = e.a;
    let c = tercera_plaza(&mut e, m);
    let (col_en_c, col_en_a) = (e.columna_en("ana-en-c", "ana", 1, c), e.columna_en("ana-en-a", "ana", 1, a));
    e.dar(c, Piedra, 100_000);
    let tr = e.ok("ana", Comando::ProponerTrueque { a, b: c, lineas_a: vec![(Madera, 40_000)], lineas_b: vec![(Piedra, 20_000)] }).creadas[0];
    e.ok("ana", Comando::AceptarTrueque { acuerdo: tr });
    // Lo de A llega a la puerta de C con la columna que está allí; lo de C, a la puerta de A: la Facción debe los dos lados y manda el primero
    // con algo pendiente (A antes que B).
    let ca = e.caravana_aparcada("ana", a, col_en_c, 1);
    e.cargar("ana", ca, Madera, 40_000);
    let cc = e.caravana_aparcada("ana", c, col_en_a, 1);
    e.cargar("ana", cc, Piedra, 20_000);
    let (comision_1, _) = entrega_esperada(&e.n, a, c, &[(Madera, 40_000)]);
    assert!(comision_1 > 0);
    let oro0 = oro(&e.n);
    e.ok("admin", Comando::AdjuntarCaravana { caravana: ca, ejercito: col_en_c });
    e.ok("ana", Comando::EntregarDeCaravana { ejercito: col_en_c, caravana: ca, acuerdo: tr });
    assert_eq!(oro(&e.n) - oro0, comision_1);
    let r = comercio::Reglas::cargar();
    // Misma Facción: 3 % del valor de la madera por el bonus de distancia, no el 8 % de la tasa externa.
    let valor = {
        let e_ = e.n.estado();
        let t = TablaPrecios::de(&r.precio, e_.asentamientos.values().map(|p| &p.economia.almacen));
        t.valor(&[(Madera, 40_000)].into_iter().collect())
    };
    assert!(comision_1 < valor * 5 / 100 + 2, "{comision_1} {valor}");
    e.ok("admin", Comando::AdjuntarCaravana { caravana: cc, ejercito: col_en_a });
    e.ok("ana", Comando::EntregarDeCaravana { ejercito: col_en_a, caravana: cc, acuerdo: tr });
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Cumplido);
    let cumplidos = hechos(&e.n, "comercio.trueque_cumplido");
    assert_eq!((cumplidos.len(), dato(&cumplidos[0], "entreFacciones")), (1, 0), "el hecho sale igual, con entreFacciones falso");
    assert_eq!((e.n.reputacion(1), hechos(&e.n, "faccion.experiencia").len()), (0, 0), "la reputación y la experiencia son entre Facciones");
}

// ───────────────────────────── 2. Caducidad por evento ─────────────────────────────

#[test]
fn una_propuesta_sin_contestar_caduca_a_su_instante_sin_penalizar() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, true);
    let t0 = e.n.ahora();
    let tr = e.trueque_sin_aceptar();
    let vence = t0 + DIA;
    assert_eq!(e.n.acuerdo(tr).unwrap().expira_en, vence);
    e.avanzar_hasta(vence - 1);
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Propuesto);
    e.avanzar_hasta(vence);
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Expirado);
    let ex = hechos(&e.n, "comercio.trueque_expirado");
    assert_eq!((ex.len(), ex[0].0, dato(&ex[0], "acuerdo"), dato(&ex[0], "incumplido")), (1, vence, tr as i64, 0));
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (0, 0), "una propuesta que nadie contesta no penaliza");
    // Una propuesta vencida ya no se acepta; contestarla a tiempo cambia el plazo (se cuenta desde el sí).
    assert_eq!(e.rechazada("beto", Comando::AceptarTrueque { acuerdo: tr }), rechazos::TRUEQUE_NO_ESTA_PROPUESTO);
    let t1 = e.n.ahora();
    let tr2 = e.trueque_sin_aceptar();
    e.avanzar(DIA - MIN);
    e.ok("beto", Comando::AceptarTrueque { acuerdo: tr2 });
    assert_eq!(e.n.acuerdo(tr2).unwrap().expira_en, t1 + DIA - MIN + DIA, "el plazo del sí es de un día entero");
    e.avanzar_hasta(t1 + DIA + HORA);
    assert_eq!(e.n.acuerdo(tr2).unwrap().estado, EstadoTrueque::Activo, "el evento de la propuesta se invalidó al aceptarla");
    // Rechazar deja el acuerdo (no se borra) y tampoco penaliza.
    let tr3 = e.trueque_sin_aceptar();
    e.ok("beto", Comando::RechazarTrueque { acuerdo: tr3 });
    assert_eq!(e.n.acuerdo(tr3).unwrap().estado, EstadoTrueque::Rechazado);
    e.avanzar(2 * DIA);
    assert_eq!(hechos(&e.n, "comercio.trueque_expirado").len(), 2, "el rechazado no vuelve a caducar");
}

#[test]
fn un_trueque_activo_que_vence_penaliza_en_proporcion_a_lo_no_entregado_y_la_reputacion_se_recupera_sola() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, true);
    let (a, b) = (e.a, e.b);
    let col = e.columna_en("ana-en-b", "ana", 1, b);
    let ca = e.caravana_aparcada("ana", a, col, 1);
    e.cargar("ana", ca, Madera, 100_000);
    let tr = e.trueque_sin_aceptar();
    e.ok("beto", Comando::AceptarTrueque { acuerdo: tr });
    let vence = e.n.acuerdo(tr).unwrap().expira_en;
    // A entrega la madera (100 de 150 pedidos en su lado), B no entrega nada.
    let t = e.n.ahora();
    e.n.entregar_contenido(t, ca, b, Some((tr, Lado::A))).expect("existe");
    assert_eq!(e.n.acuerdo(tr).unwrap().pendiente(Lado::A), vec![(Piedra, 50_000)]);
    e.avanzar_hasta(vence - 1);
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (0, 0));
    e.avanzar_hasta(vence);
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Expirado);
    // A debe 1/3 (333.333 ppm) y B debe todo: −8 × fracción, a cada Facción, al instante del vencimiento.
    assert_eq!(e.n.reputacion(1), -2_666_664);
    assert_eq!(e.n.reputacion(2), -8 * PUNTO);
    let ex = hechos(&e.n, "comercio.trueque_expirado");
    assert_eq!((ex.len(), dato(&ex[0], "incumplido")), (1, 1));
    let cambios = hechos(&e.n, "reputacion.cambio");
    assert_eq!(cambios.iter().map(|h| (dato(h, "faccion"), dato(h, "delta"))).collect::<Vec<_>>(), vec![(1, -2_666_664), (2, -8 * PUNTO)]);
    // Sube sola 0,002 por minuto y, al cruzar el 0, la cita reancla el ritmo (cuatro mil minutos para los −8).
    e.avanzar_hasta(vence + 3_999 * MIN);
    assert_eq!(e.n.reputacion(2), -2_000, "falta un minuto para el cero");
    e.avanzar_hasta(vence + 4_000 * MIN);
    assert_eq!(e.n.reputacion(2), 0);
    assert_eq!(e.n.estado().comercio.reputacion[&2].ancla(), (0, vence + 4_000 * MIN), "la cita de cruce de cero reanclaba el ritmo");
    assert_eq!(e.n.estado().comercio.reputacion[&1].ancla(), (0, vence + 2_666_664 * 30), "la de la Facción 1 cruzó antes");
}

#[test]
fn rareza_un_trueque_expirado_que_se_salda_pasa_a_cumplido_y_reparte_la_recompensa() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, true);
    let (a, b) = (e.a, e.b);
    let (col_a, col_b) = (e.columna_en("ana-en-a", "ana", 1, a), e.columna_en("beto-en-b", "beto", 2, b));
    e.dar(b, Cobre, 100_000);
    let ca = e.caravana_aparcada("ana", a, col_a, 1);
    e.cargar("ana", ca, Madera, 100_000);
    let cb = e.caravana_aparcada("beto", b, col_b, 1);
    e.cargar("beto", cb, Cobre, 30_000);
    let tr = e.trueque(&[(Madera, 100_000)], &[(Cobre, 30_000)]);
    let vence = e.n.acuerdo(tr).unwrap().expira_en;
    e.avanzar_hasta(vence);
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Expirado);
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (-8 * PUNTO, -8 * PUNTO));
    // Las caravanas llegan tarde, en el mismo instante: abonan aunque ya venció (§15) y al saldarse pasa a cumplido.
    e.n.entregar_contenido(vence, ca, b, Some((tr, Lado::A))).unwrap();
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Expirado);
    e.n.entregar_contenido(vence, cb, a, Some((tr, Lado::B))).unwrap();
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Cumplido);
    let cumplidos = hechos(&e.n, "comercio.trueque_cumplido");
    assert_eq!((cumplidos.len(), dato(&cumplidos[0], "trasVencer")), (1, 1));
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (-3 * PUNTO, -3 * PUNTO), "−8 por vencer y +5 por cumplir, aunque tarde");
    // Y no se reabre: otra llegada sobre lo cumplido no vuelve a repartir nada.
    e.cargar("ana", ca, Madera, 1_000);
    e.n.entregar_contenido(vence, ca, b, Some((tr, Lado::A))).unwrap();
    assert_eq!(hechos(&e.n, "comercio.trueque_cumplido").len(), 1);
}

// ───────────────────────────── 3. Órdenes de mercado y mostrador ─────────────────────────────

#[test]
fn las_ordenes_y_el_mostrador_mueven_el_almacen_real_y_conservan_el_oro() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, true);
    let (a, b) = (e.a, e.b);
    let col = e.columna_en("beto-en-a", "beto", 2, a);
    e.dar(a, Madera, 100_000);
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 300_000 });
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Cobre, cantidad: 50_000 });
    let t0 = e.n.ahora();

    // Colocar: solo quien manda en la plaza, con Mercado, cantidad positiva y (si vende) existencias.
    let venta = |cantidad, precio| Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Madera, cantidad, precio };
    assert_eq!(e.rechazada("luis", venta(100_000, Some(2 * PUNTO))), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("beto", venta(100_000, Some(2 * PUNTO))), rechazos::NO_AUTORIZADO, "una columna de otra Facción no manda en A");
    assert_eq!(e.rechazada("ana", venta(0, Some(2 * PUNTO))), rechazos::ORDEN_CANTIDAD_NO_POSITIVA);
    assert_eq!(e.rechazada("ana", venta(10_000_000, Some(2 * PUNTO))), rechazos::ORDEN_SIN_STOCK);
    assert_eq!(e.rechazada("ana", Comando::ColocarOrdenMercado { asentamiento: 999, venta: true, recurso: Madera, cantidad: 1, precio: None }), rechazos::ASENTAMIENTO_DESCONOCIDO);
    let v = e.ok("ana", venta(50_000, Some(2 * PUNTO))).creadas[0];
    let c = e.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: false, recurso: Cobre, cantidad: 50_000, precio: Some(5 * PUNTO) }).creadas[0];
    let sin_precio = e.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: false, recurso: Piedra, cantidad: 10_000, precio: None }).creadas[0];
    assert!(e.n.libro_ordenes().orden(sin_precio).unwrap().precio_unitario > 0, "sin precio, el de referencia");
    assert_eq!(hechos(&e.n, "mercado.orden_colocada").len(), 3);
    assert_eq!(e.n.escaparate(a, false).len(), 0, "el escaparate solo lo ve quien está en la puerta");
    assert_eq!(e.n.escaparate(a, true).len(), 3);

    // El mostrador: la plaza vende. Facciones distintas: tasa externa del 8 % → 2 × 1,08 = 2,16 por unidad.
    let comerciar = |orden, cantidad| Comando::ComerciarEnPlaza { ejercito: col, orden, cantidad };
    assert_eq!(e.rechazada("luis", comerciar(v, 1_000)), rechazos::MOSTRADOR_NO_ES_LIDER);
    assert_eq!(e.rechazada("beto", comerciar(999, 1_000)), rechazos::ORDEN_DESCONOCIDA);
    assert_eq!(e.rechazada("beto", comerciar(v, 0)), rechazos::MOSTRADOR_CANTIDAD_NO_POSITIVA);
    let lejos = e.columna_en("beto-en-b", "beto", 2, b);
    assert_eq!(e.rechazada("beto", Comando::ComerciarEnPlaza { ejercito: lejos, orden: v, cantidad: 1_000 }), rechazos::MOSTRADOR_FUERA_DE_LA_PUERTA);
    let (oro0, madera_mundo0, madera_a0, oro_a0, carro0) = (oro(&e.n), mundo_de(&e.n, Madera), cantidad(&e.n, a, Madera), cantidad(&e.n, a, Oro), e.n.suministro(col));
    e.ok("beto", comerciar(v, 20_000));
    assert_eq!(cantidad(&e.n, a, Madera), madera_a0 - 20_000);
    assert_eq!(cantidad(&e.n, a, Oro), oro_a0 + 43_200, "20 × 2,16 = 43,2: la comisión se queda en la plaza");
    let carro = e.n.suministro(col);
    assert_eq!((carro[&Oro], carro[&Madera]), (carro0[&Oro] - 43_200, 20_000));
    assert_eq!((oro(&e.n), mundo_de(&e.n, Madera)), (oro0, madera_mundo0), "el oro y la madera del mundo no cambian");
    let o = e.n.libro_ordenes().orden(v).unwrap();
    assert_eq!((o.cantidad_cumplida, o.estado), (20_000, EstadoOrden::Activa));
    let cp = hechos(&e.n, "mercado.comercio_en_plaza");
    assert_eq!((cp.len(), dato(&cp[0], "cantidad"), dato(&cp[0], "entreFacciones"), dato(&cp[0], "oroMovido"), dato(&cp[0], "comision")), (1, 20_000, 1, 43_200, 3_200));

    // Lo que queda de la orden: se sirve lo pendiente (30) aunque se pida más, y la orden sale al historial sin evento.
    e.ok("beto", comerciar(v, 400_000));
    assert_eq!(e.n.suministro(col)[&Madera], 50_000);
    assert!(e.n.libro_ordenes().orden(v).is_none());
    assert_eq!(e.n.libro_ordenes().buscar(v).unwrap().estado, EstadoOrden::Cumplida);
    assert_eq!(e.rechazada("beto", comerciar(v, 1_000)), rechazos::ORDEN_NO_ACTIVA);
    assert_eq!(oro(&e.n), oro0);
    assert_eq!(e.rechazada("ana", Comando::CancelarOrdenMercado { orden: v }), rechazos::ORDEN_NO_ACTIVA);

    // La plaza compra: 5 × 0,92 = 4,6 por unidad → 50 de cobre son 230 de oro.
    let (oro_a1, cobre_a1) = (cantidad(&e.n, a, Oro), cantidad(&e.n, a, Cobre));
    e.ok("beto", comerciar(c, 50_000));
    assert_eq!((cantidad(&e.n, a, Oro), cantidad(&e.n, a, Cobre)), (oro_a1 - 230_000, cobre_a1 + 50_000));
    assert_eq!(e.n.suministro(col).get(&Cobre), None, "las entradas a cero se borran del carro");
    assert_eq!(oro(&e.n), oro0);

    // Una orden sin precio positivo o sobre el oro no se liquida.
    let gratis = e.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Madera, cantidad: 1_000, precio: Some(0) }).creadas[0];
    assert_eq!(e.rechazada("beto", comerciar(gratis, 1_000)), rechazos::MOSTRADOR_PRECIO_NO_POSITIVO);
    let oro_orden = e.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Oro, cantidad: 1_000, precio: Some(PUNTO) }).creadas[0];
    assert_eq!(e.rechazada("beto", comerciar(oro_orden, 1_000)), rechazos::MOSTRADOR_RECURSO_ORO);
    // Con el carro sin piedra, la compra de piedra no tiene nada que servir.
    assert_eq!(e.rechazada("beto", comerciar(sin_precio, 1_000)), rechazos::MOSTRADOR_NADA_QUE_SERVIR);

    // Retirar una orden: solo quien manda en la plaza; sale al historial y su plazo ya no vence.
    let retirable = e.ok("ana", venta(1_000, Some(PUNTO))).creadas[0];
    assert_eq!(e.rechazada("beto", Comando::CancelarOrdenMercado { orden: retirable }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", Comando::CancelarOrdenMercado { orden: 999 }), rechazos::ORDEN_DESCONOCIDA);
    e.ok("ana", Comando::CancelarOrdenMercado { orden: retirable });
    assert_eq!(e.n.libro_ordenes().buscar(retirable).unwrap().estado, EstadoOrden::Expirada);
    assert_eq!(hechos(&e.n, "mercado.orden_cancelada").len(), 1);

    // Caducidad por evento: a los 200 minutos justos de colocarse, y no antes.
    assert!(e.n.libro_ordenes().orden(c).is_none(), "la de cobre ya se cumplió");
    let vence = e.n.libro_ordenes().orden(gratis).unwrap().expira_en;
    assert_eq!(vence, t0 + 200 * MIN);
    e.avanzar_hasta(vence - 1);
    assert!(e.n.libro_ordenes().orden(gratis).is_some() && !e.n.escaparate(a, true).is_empty());
    e.avanzar_hasta(vence);
    assert!(e.n.libro_ordenes().ordenes.is_empty(), "todas las activas vencieron a la vez");
    assert_eq!(e.n.libro_ordenes().buscar(gratis).unwrap().estado, EstadoOrden::Expirada);
    let ex = hechos(&e.n, "mercado.orden_expirada");
    let mut ids: Vec<i64> = ex.iter().map(|h| dato(h, "orden")).collect();
    ids.sort();
    assert_eq!(ids, vec![sin_precio as i64, gratis as i64, oro_orden as i64], "una vez cada una, y las cumplidas o retiradas no");
    assert!(ex.iter().all(|h| h.0 == vence && h.1 == vec![a]));
    assert!(e.n.escaparate(a, true).is_empty());
    assert_eq!(e.rechazada("beto", comerciar(gratis, 1_000)), rechazos::ORDEN_NO_ACTIVA);
    // Sin Mercado no se coloca ninguna orden.
    let mut sin = escenario(m, 7, false);
    let a2 = sin.a;
    assert_eq!(sin.rechazada("ana", Comando::ColocarOrdenMercado { asentamiento: a2, venta: false, recurso: Piedra, cantidad: 1, precio: Some(PUNTO) }), rechazos::MERCADO_SIN_MERCADO);
}

#[test]
fn el_mostrador_topa_por_el_oro_de_quien_compra_y_el_espacio_del_carro() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, true);
    let a = e.a;
    let col = e.columna_en("ana-en-a", "ana", 1, a);
    e.dar(a, Madera, 100_000);
    let v = e.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Madera, cantidad: 200_000, precio: Some(PUNTO) }).creadas[0];
    // Misma Facción: 3 %. El carro solo tiene 10 de oro: se sirven 9,708 unidades (10 / 1,03) y nada más.
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 10_000 });
    let oro0 = oro(&e.n);
    e.ok("ana", Comando::ComerciarEnPlaza { ejercito: col, orden: v, cantidad: 200_000 });
    assert_eq!(e.n.suministro(col)[&Madera], 9_708);
    assert_eq!(e.n.suministro(col).get(&Oro), None, "se gastó todo el oro del carro");
    assert_eq!(oro(&e.n), oro0);
    // El carro lleno (500) no admite nada más: DarSuministro lo recorta y repetirlo no cambia nada.
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Piedra, cantidad: 10_000_000 });
    assert_eq!(e.n.suministro(col).values().sum::<i64>(), 500_000);
    assert_eq!(e.ord("admin", Comando::DarSuministro { ejercito: col, recurso: Piedra, cantidad: 1 }).resultado, Resultado::SinCambios);
    assert_eq!(e.rechazada("ana", Comando::DarSuministro { ejercito: col, recurso: Piedra, cantidad: 1 }), rechazos::NO_AUTORIZADO, "es andamiaje de administración");
    assert_eq!(e.rechazada("admin", Comando::DarRecursos { asentamiento: a, recurso: Piedra, cantidad: 0 }), rechazos::CANTIDAD_NO_POSITIVA);
}

// ───────────────────────────── 4. La flota y sus cupos ─────────────────────────────

#[test]
fn la_flota_crece_hasta_los_cupos_del_mercado_y_cada_rechazo_tiene_su_codigo() {
    let m = con_mundo!();
    // Sin Mercado el cupo es 0.
    let mut sin = escenario(m, 7, false);
    let a0 = sin.a;
    assert_eq!(sin.n.cupos_de_flota(a0).map(|c| (c.caravanas, c.carros, c.escolta)), Some((0, 0, 0)));
    assert_eq!(sin.rechazada("ana", Comando::CrearCaravana { asentamiento: a0 }), rechazos::CARAVANA_SIN_MERCADO);

    let mut e = escenario(m, 7, true);
    let (a, b) = (e.a, e.b);
    assert_eq!(e.n.cupos_de_flota(a).map(|c| (c.caravanas, c.carros, c.escolta)), Some((2, 3, 100)), "Mercado de nivel 1: 2 caravanas, 3 carros, 100 de escolta");
    assert_eq!(e.rechazada("luis", Comando::CrearCaravana { asentamiento: a }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("beto", Comando::CrearCaravana { asentamiento: a }), rechazos::NO_AUTORIZADO, "es de la Facción de A");
    assert_eq!(e.rechazada("ana", Comando::CrearCaravana { asentamiento: 999 }), rechazos::ASENTAMIENTO_DESCONOCIDO);
    let t0 = e.n.ahora();
    let c1 = e.ok("ana", Comando::CrearCaravana { asentamiento: a }).creadas[0];
    let casco = e.n.caravana(c1).unwrap();
    assert_eq!((casco.estado, casco.carros.len(), casco.origen as u64, casco.posicion), (EstadoCaravana::Disponible, 0, a, pos(&e.n, a)));
    assert_eq!(e.n.estado().comercio.ultima_caravana[&a], t0);
    // El cooldown de 10 minutos es de la plaza (el cupo aún no se alcanzó).
    assert_eq!(e.rechazada("ana", Comando::CrearCaravana { asentamiento: a }), rechazos::CARAVANA_EN_COOLDOWN);
    e.avanzar(10 * MIN - 1);
    assert_eq!(e.rechazada("ana", Comando::CrearCaravana { asentamiento: a }), rechazos::CARAVANA_EN_COOLDOWN);
    e.avanzar(1);
    let c2 = e.ok("ana", Comando::CrearCaravana { asentamiento: a }).creadas[0];
    e.avanzar(10 * MIN);
    assert_eq!(e.rechazada("ana", Comando::CrearCaravana { asentamiento: a }), rechazos::CARAVANA_CUPO_DE_CARAVANAS);
    assert_eq!(e.n.flota_de(a).iter().map(|c| c.id).collect::<Vec<_>>(), vec![c1, c2]);
    // Los carros cuestan 20 de madera del almacén y la flota entera tiene un cupo de 3. El reforzado, además, una Carpintería activa.
    assert_eq!(e.rechazada("ana", Comando::AnadirCarro { caravana: c1, tipo: TipoCarro::Reforzado }), rechazos::CARAVANA_SIN_CARPINTERIA);
    let madera0 = cantidad(&e.n, a, Madera);
    e.ok("ana", Comando::AnadirCarro { caravana: c1, tipo: TipoCarro::Basico });
    assert_eq!(cantidad(&e.n, a, Madera), madera0 - 20_000);
    e.ok("ana", Comando::AnadirCarro { caravana: c1, tipo: TipoCarro::Basico });
    e.ok("ana", Comando::AnadirCarro { caravana: c2, tipo: TipoCarro::Basico });
    assert_eq!(e.rechazada("ana", Comando::AnadirCarro { caravana: c2, tipo: TipoCarro::Basico }), rechazos::CARAVANA_CUPO_DE_CARROS);
    assert_eq!(e.rechazada("beto", Comando::AnadirCarro { caravana: c2, tipo: TipoCarro::Basico }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", Comando::AnadirCarro { caravana: 999, tipo: TipoCarro::Basico }), rechazos::CARAVANA_DESCONOCIDA);
    // Los animales cuestan oro, que se destruye: es un sumidero del diseño.
    let (oro_a0, oro_mundo0) = (cantidad(&e.n, a, Oro), oro(&e.n));
    e.ok("ana", Comando::AnadirAnimal { caravana: c1, carro: 0, animal: Animal::Buey });
    assert_eq!((cantidad(&e.n, a, Oro), oro(&e.n)), (oro_a0 - 12_000, oro_mundo0 - 12_000));
    assert_eq!(e.rechazada("ana", Comando::AnadirAnimal { caravana: c1, carro: 0, animal: Animal::Caballo }), rechazos::CARAVANA_CARRO_YA_TIENE_ANIMAL);
    assert_eq!(e.rechazada("ana", Comando::AnadirAnimal { caravana: c1, carro: 7, animal: Animal::Buey }), rechazos::CARAVANA_CARRO_INEXISTENTE);
    // Mover un carro con su animal entre caravanas del mismo origen no toca el cupo; entre orígenes distintos no se puede.
    e.ok("ana", Comando::MoverCarro { de: c1, a: c2, carro: 0 });
    assert_eq!((e.n.caravana(c1).unwrap().carros.len(), e.n.caravana(c2).unwrap().carros.len()), (1, 2));
    assert_eq!(e.n.caravana(c2).unwrap().carros[1].animal, Some(Animal::Buey), "el carro se lleva su animal");
    assert_eq!(e.rechazada("ana", Comando::MoverCarro { de: c1, a: c1, carro: 0 }), rechazos::CARAVANA_ORIGEN_DISTINTO);
    assert_eq!(e.rechazada("ana", Comando::MoverCarro { de: c1, a: c2, carro: 9 }), rechazos::CARAVANA_CARRO_INEXISTENTE);
    let cb = e.ok("beto", Comando::CrearCaravana { asentamiento: b }).creadas[0];
    assert_eq!(e.rechazada("ana", Comando::MoverCarro { de: c1, a: cb, carro: 0 }), rechazos::CARAVANA_ORIGEN_DISTINTO);
    // Reservar saca la caravana del reparto automático; repetirlo no cambia nada.
    e.ok("ana", Comando::ReservarCaravana { caravana: c1, reservada: true });
    assert!(e.n.caravana(c1).unwrap().reservada_manual);
    assert_eq!(e.ord("ana", Comando::ReservarCaravana { caravana: c1, reservada: true }).resultado, Resultado::SinCambios);
    assert_eq!(hechos(&e.n, "comercio.caravana_construida").len(), 3);
    assert_eq!(hechos(&e.n, "comercio.caravana_carro_agregado").len(), 3);
    assert_eq!(hechos(&e.n, "comercio.caravana_animal_comprado").len(), 1);
    assert_eq!(hechos(&e.n, "comercio.caravana_carro_movido").len(), 1);
    assert_eq!(hechos(&e.n, "comercio.caravana_reserva").len(), 1);
}

// ───────────────────────────── 5. Preparar y cancelar ─────────────────────────────

#[test]
fn preparar_reserva_la_carga_registra_el_fin_y_cancelar_la_devuelve_topada_por_la_capacidad() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, true);
    let (a, b) = (e.a, e.b);
    // Un casco sin animal no tiene capacidad; con un solo carro no hay preparación y la salida sería inmediata (3B-2).
    let c = e.ok("ana", Comando::CrearCaravana { asentamiento: a }).creadas[0];
    let preparar = |carga: Vec<(Recurso, i64)>, salir_en| Comando::PrepararCaravana { caravana: c, destino: b, carga, salir_en };
    assert_eq!(e.rechazada("ana", preparar(vec![(Madera, 1_000)], None)), rechazos::CARAVANA_SIN_CAPACIDAD);
    e.ok("ana", Comando::AnadirCarro { caravana: c, tipo: TipoCarro::Basico });
    e.ok("ana", Comando::AnadirAnimal { caravana: c, carro: 0, animal: Animal::Buey });
    let t = e.n.ahora();
    assert_eq!(e.rechazada("ana", preparar(vec![(Madera, 1_000)], Some(t))), rechazos::CARAVANA_SALIDA_INVALIDA);
    assert_eq!(e.rechazada("ana", preparar(vec![(Madera, 1_000)], Some(t + 3 * DIA + 1))), rechazos::CARAVANA_SALIDA_INVALIDA);
    assert_eq!(e.rechazada("ana", preparar(vec![(Madera, 600_000)], Some(t + HORA))), rechazos::CARAVANA_CARGA_INVALIDA, "pasa de la capacidad de un carro con buey (500)");
    assert_eq!(e.rechazada("ana", preparar(vec![], Some(t + HORA))), rechazos::CARAVANA_CARGA_INVALIDA);
    assert_eq!(e.rechazada("ana", preparar(vec![(Madera, -5)], Some(t + HORA))), rechazos::CARAVANA_CARGA_INVALIDA);
    assert_eq!(e.rechazada("ana", preparar(vec![(Estano, 1_000)], Some(t + HORA))), rechazos::CARAVANA_SIN_STOCK);
    assert_eq!(e.rechazada("beto", preparar(vec![(Madera, 1_000)], Some(t + HORA))), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", Comando::PrepararCaravana { caravana: c, destino: 999, carga: vec![(Madera, 1_000)], salir_en: Some(t + HORA) }), rechazos::ASENTAMIENTO_DESCONOCIDO);
    assert_eq!(e.rechazada("ana", Comando::PrepararCaravana { caravana: 999, destino: b, carga: vec![(Madera, 1_000)], salir_en: Some(t + HORA) }), rechazos::CARAVANA_DESCONOCIDA);
    assert_eq!(e.n.caravana(c).unwrap().estado, EstadoCaravana::Disponible, "un rechazo no cambia nada");
    // Con dos carros, la preparación cuenta (2 min por carro de más) y la carga se reserva ya.
    e.ok("ana", Comando::AnadirCarro { caravana: c, tipo: TipoCarro::Basico });
    e.ok("ana", Comando::AnadirAnimal { caravana: c, carro: 1, animal: Animal::Buey });
    let (madera_a0, madera_mundo0) = (cantidad(&e.n, a, Madera), mundo_de(&e.n, Madera));
    let (v0, h0) = (e.n.version(), e.n.hash_estado());
    let r = e.ok("ana", preparar(vec![(Madera, 150_000), (Madera, 50_000)], None));
    assert!(r.version_tras > v0 && e.n.hash_estado() != h0);
    let prep = e.n.caravana(c).unwrap();
    assert_eq!((prep.estado, prep.contenido[&Madera], prep.destino, prep.prepara_hasta), (EstadoCaravana::Preparando, 200_000, Some(b as u32), Some(t + 2 * MIN)));
    assert!(prep.ruta.len() >= 2 && prep.ruta[0] == pos(&e.n, a) && *prep.ruta.last().unwrap() == pos(&e.n, b), "la ruta por tierra de crates/rutas");
    assert_eq!(cantidad(&e.n, a, Madera), madera_a0 - 200_000);
    assert_eq!(mundo_de(&e.n, Madera), madera_mundo0, "la madera pasa del almacén a la caravana: no se crea ni se pierde");
    let ev = hechos(&e.n, "comercio.caravana_preparando");
    assert_eq!((ev.len(), ev[0].1.clone(), dato(&ev[0], "saleEn")), (1, vec![a, b], t + 2 * MIN));
    // Ya no está disponible: no admite más carros, ni otra preparación.
    assert_eq!(e.rechazada("ana", Comando::AnadirCarro { caravana: c, tipo: TipoCarro::Basico }), rechazos::CARAVANA_NO_DISPONIBLE);
    assert_eq!(e.rechazada("ana", preparar(vec![(Madera, 1_000)], Some(t + HORA))), rechazos::CARAVANA_NO_DISPONIBLE);
    assert_eq!(e.rechazada("ana", Comando::CancelarCaravana { caravana: 999 }), rechazos::CARAVANA_DESCONOCIDA);
    assert_eq!(e.rechazada("beto", Comando::CancelarCaravana { caravana: c }), rechazos::NO_AUTORIZADO);
    // El fin de la preparación es un evento a su instante: hasta entonces sigue preparando (en su ciudad, sin plan).
    e.avanzar_hasta(t + 2 * MIN - 1);
    assert!(hechos(&e.n, "comercio.caravana_sale").is_empty());
    assert_eq!(e.n.caravana(c).unwrap().estado, EstadoCaravana::Preparando);
    assert!(e.n.caravana(c).unwrap().plan.is_none());
    // Cancelar: la carga vuelve al almacén, pero solo cabe lo que deja la capacidad (el resto se pierde).
    let espacio = e.n.economia(a).unwrap().almacen.espacio(Madera);
    assert!(espacio >= 200_000, "{espacio}");
    e.dar(a, Madera, espacio - 100_000);
    let (madera_antes, mundo_antes) = (cantidad(&e.n, a, Madera), mundo_de(&e.n, Madera));
    e.ok("ana", Comando::CancelarCaravana { caravana: c });
    let casco = e.n.caravana(c).unwrap();
    assert_eq!((casco.estado, casco.contenido.is_empty(), casco.destino, casco.prepara_hasta, casco.ruta.len(), casco.carros.len()), (EstadoCaravana::Disponible, true, None, None, 0, 2));
    assert_eq!(cantidad(&e.n, a, Madera), madera_antes + 100_000, "solo cabían 100");
    assert_eq!(mundo_de(&e.n, Madera), mundo_antes - 100_000, "lo que no cupo se pierde");
    let cc = hechos(&e.n, "comercio.caravana_cancelada");
    assert_eq!((cc.len(), dato(&cc[0], "devuelto"), dato(&cc[0], "perdido")), (1, 100_000, 100_000));
    assert_eq!(e.rechazada("ana", Comando::CancelarCaravana { caravana: c }), rechazos::CARAVANA_NO_PREPARANDO);
    // El fin de la preparación cancelada no vuelve a salir, y una caravana que se prepara de nuevo tiene su propio plazo.
    e.avanzar(10 * MIN);
    assert!(hechos(&e.n, "comercio.caravana_sale").is_empty());
    e.ok("ana", Comando::PrepararCaravana { caravana: c, destino: b, carga: vec![(Madera, 10_000)], salir_en: None });
    let t2 = e.n.ahora();
    e.avanzar(2 * MIN);
    assert_eq!(hechos(&e.n, "comercio.caravana_sale").iter().map(|h| (h.0, dato(h, "caravana"))).collect::<Vec<_>>(), vec![(t2 + 2 * MIN, c as i64)]);
    assert_eq!(e.n.caravana(c).unwrap().estado, EstadoCaravana::EnTransito);
}

#[test]
fn una_aparcada_se_carga_se_descarga_y_vuelve_vacia_al_origen() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, true);
    let (a, b) = (e.a, e.b);
    e.dar(a, Piedra, 400_000);
    let col = e.columna_en("ana-en-a", "ana", 1, a);
    let c = e.caravana_aparcada("ana", a, col, 1);
    assert_eq!(e.n.caravana(c).unwrap().anfitriona, Some(a as u32));
    // Cargar: min(pedido, capacidad − lo cargado, lo que hay en el almacén). La capacidad de un carro con buey es 500.
    let madera0 = cantidad(&e.n, a, Madera);
    assert!(madera0 > 100_000 && madera0 < 400_000);
    e.cargar("ana", c, Madera, 600_000);
    assert_eq!((e.n.caravana(c).unwrap().contenido[&Madera], cantidad(&e.n, a, Madera)), (madera0, 0), "se llevó todo lo que había");
    let piedra0 = cantidad(&e.n, a, Piedra);
    e.cargar("ana", c, Piedra, 600_000);
    assert_eq!(e.n.caravana(c).unwrap().contenido[&Piedra], 500_000 - madera0, "topada por lo que le queda de capacidad");
    assert_eq!(cantidad(&e.n, a, Piedra), piedra0 - (500_000 - madera0));
    assert_eq!(e.rechazada("ana", Comando::MoverCargaAparcada { caravana: c, recurso: Piedra, cantidad: 1, cargar: true }), rechazos::CARAVANA_NADA_QUE_MOVER, "ya está llena");
    assert_eq!(e.rechazada("beto", Comando::MoverCargaAparcada { caravana: c, recurso: Madera, cantidad: 1, cargar: false }), rechazos::NO_AUTORIZADO);
    e.ok("ana", Comando::MoverCargaAparcada { caravana: c, recurso: Madera, cantidad: 100_000, cargar: false });
    assert_eq!((e.n.caravana(c).unwrap().contenido[&Madera], cantidad(&e.n, a, Madera)), (madera0 - 100_000, 100_000));
    // Vacía vuelve al instante (con carga viaja: véase `tests/caravanas.rs`).
    for r in [Madera, Piedra] {
        let n = e.n.caravana(c).unwrap().contenido[&r];
        e.ok("ana", Comando::MoverCargaAparcada { caravana: c, recurso: r, cantidad: n, cargar: false });
    }
    assert!(e.n.caravana(c).unwrap().contenido.is_empty());
    assert_eq!(cantidad(&e.n, a, Madera), madera0);
    e.ok("ana", Comando::EnviarCaravanaAlOrigen { caravana: c });
    let casa = e.n.caravana(c).unwrap();
    assert_eq!((casa.estado, casa.anfitriona, casa.posicion), (EstadoCaravana::Disponible, None, pos(&e.n, a)));
    assert_eq!(e.rechazada("ana", Comando::EnviarCaravanaAlOrigen { caravana: c }), rechazos::CARAVANA_NO_APARCADA);
    assert_eq!(e.rechazada("ana", Comando::MoverCargaAparcada { caravana: c, recurso: Madera, cantidad: 1, cargar: true }), rechazos::CARAVANA_NO_APARCADA);
    // Aparcar solo en una plaza de la Facción de su origen.
    e.ok("admin", Comando::AdjuntarCaravana { caravana: c, ejercito: col });
    assert_eq!(e.rechazada("admin", Comando::AparcarCaravana { caravana: c, asentamiento: b }), rechazos::CARAVANA_PLAZA_AJENA);
    assert_eq!(e.rechazada("beto", Comando::AparcarCaravana { caravana: c, asentamiento: a }), rechazos::NO_AUTORIZADO, "la columna es de Ana");
    assert_eq!(e.rechazada("admin", Comando::AdjuntarCaravana { caravana: c, ejercito: 999 }), rechazos::EJERCITO_DESCONOCIDO);
    assert_eq!(e.rechazada("admin", Comando::AdjuntarCaravana { caravana: c, ejercito: col }), rechazos::CARAVANA_ESTADO_INVALIDO, "ya está adjunta");
}

// ───────────────────────────── 6. Reputación baja: umbral, recargo y recuperación ─────────────────────────────

#[test]
fn con_mala_reputacion_la_entrega_paga_recargo_del_oro_del_origen_y_la_reputacion_sale_sola_del_umbral() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, true);
    let (a, b) = (e.a, e.b);
    // Cinco trueques que vencen sin cumplirse: −8 por trueque a cada Facción, −40 en total, justo el umbral.
    let t0 = e.n.ahora();
    for _ in 0..5 {
        e.trueque(&[(Madera, 1_000)], &[(Cobre, 1_000)]);
    }
    let vence = t0 + DIA;
    e.avanzar_hasta(vence - 1);
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (0, 0));
    e.avanzar_hasta(vence);
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (-40 * PUNTO, -40 * PUNTO));
    let entra: Vec<(i64, i64)> = hechos(&e.n, "reputacion.umbral_bajo").iter().map(|h| (dato(h, "faccion"), dato(h, "entra"))).collect();
    assert_eq!(entra, vec![(1, 1), (2, 1)], "al llegar a −40 entra en el umbral bajo, una vez por Facción");

    // Una entrega de B a A con la reputación de B en −40: la comisión externa es del 12 % (8 % × 1,5 de distancia máxima) y B paga
    // además el 50 % de esa comisión de su propio oro, que va al almacén de A (se conserva).
    let col_b_en_a = e.columna_en("beto-en-a", "beto", 2, a);
    // Pasado un día de impuestos, el oro de A está en su tope y no cabría la comisión: A le compra cobre a la columna de Beto (el oro pasa
    // al carro y se conserva) para hacer sitio.
    e.ok("admin", Comando::DarSuministro { ejercito: col_b_en_a, recurso: Cobre, cantidad: 100_000 });
    let compra = e.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: false, recurso: Cobre, cantidad: 100_000, precio: Some(5 * PUNTO) }).creadas[0];
    let oro_antes = oro(&e.n);
    e.ok("beto", Comando::ComerciarEnPlaza { ejercito: col_b_en_a, orden: compra, cantidad: 100_000 });
    assert_eq!(oro(&e.n), oro_antes);
    assert!(e.n.economia(a).unwrap().almacen.espacio(Oro) > 100_000);
    e.dar(b, Cobre, 100_000);
    let cb = e.caravana_aparcada("beto", b, col_b_en_a, 1);
    e.cargar("beto", cb, Cobre, 30_000);
    let tr = e.trueque(&[(Madera, 100_000)], &[(Cobre, 30_000)]);
    e.ok("admin", Comando::AdjuntarCaravana { caravana: cb, ejercito: col_b_en_a });
    let (comision, recargo) = entrega_esperada(&e.n, b, a, &[(Cobre, 30_000)]);
    assert!(comision > 0 && recargo > 0, "{comision} {recargo}");
    let (oro0, oro_a0, oro_b0) = (oro(&e.n), cantidad(&e.n, a, Oro), cantidad(&e.n, b, Oro));
    e.ok("beto", Comando::EntregarDeCaravana { ejercito: col_b_en_a, caravana: cb, acuerdo: tr });
    assert_eq!(cantidad(&e.n, b, Oro), oro_b0 - recargo, "el recargo sale del oro del almacén de origen");
    assert_eq!(cantidad(&e.n, a, Oro), oro_a0 + comision + recargo, "y va al destino junto a la comisión acuñada");
    assert_eq!(oro(&e.n) - oro0, comision, "el oro del mundo solo crece por la comisión acuñada");
    let escoltada = hechos(&e.n, "comercio.entrega_escoltada");
    assert_eq!((dato(&escoltada[0], "comision"), dato(&escoltada[0], "recargo")), (comision, recargo));

    // La reputación sube sola 0,002 por minuto y sale del umbral en cuanto pasa de −40 (a los 30 ms), con su aviso.
    e.avanzar_hasta(vence + 29);
    assert!(hechos(&e.n, "reputacion.umbral_bajo").iter().all(|h| dato(h, "entra") == 1));
    e.avanzar_hasta(vence + 30);
    let sale: Vec<(i64, i64, i64)> = hechos(&e.n, "reputacion.umbral_bajo").iter().filter(|h| dato(h, "entra") == 0).map(|h| (h.0, dato(h, "faccion"), dato(h, "valor"))).collect();
    assert_eq!(sale, vec![(vence + 30, 1, -40 * PUNTO + 1), (vence + 30, 2, -40 * PUNTO + 1)]);
}

// ───────────────────────────── 7. El orden dentro de la prioridad 6 ─────────────────────────────

#[test]
fn en_el_mismo_instante_el_fin_de_preparacion_sale_antes_que_las_caducidades() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, true);
    let (a, b) = (e.a, e.b);
    let c = e.ok("ana", Comando::CrearCaravana { asentamiento: a }).creadas[0];
    for k in 0..2 {
        e.ok("ana", Comando::AnadirCarro { caravana: c, tipo: TipoCarro::Basico });
        e.ok("ana", Comando::AnadirAnimal { caravana: c, carro: k, animal: Animal::Buey });
    }
    let t0 = e.n.ahora();
    let vence = t0 + DIA;
    // La caducidad del trueque se programa primero; el fin de preparación y la orden, después, todos para `vence`.
    let tr = e.ok("ana", Comando::ProponerTrueque { a, b, lineas_a: vec![(Madera, 1_000)], lineas_b: vec![(Cobre, 1_000)] }).creadas[0];
    e.ok("ana", Comando::PrepararCaravana { caravana: c, destino: b, carga: vec![(Madera, 10_000)], salir_en: Some(vence) });
    e.avanzar_hasta(vence - 200 * MIN);
    let orden = e.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Madera, cantidad: 1_000, precio: Some(PUNTO) }).creadas[0];
    assert_eq!((e.n.acuerdo(tr).unwrap().expira_en, e.n.caravana(c).unwrap().prepara_hasta, e.n.libro_ordenes().orden(orden).unwrap().expira_en), (vence, Some(vence), vence));
    e.avanzar_hasta(vence);
    let salida: Vec<String> = e
        .n
        .cambios_desde(0)
        .unwrap()
        .flat_map(|c| c.eventos.iter())
        .filter(|ev| ev.en == vence && (ev.codigo.starts_with("comercio.") || ev.codigo.starts_with("mercado.")))
        .map(|ev| ev.codigo.to_string())
        .collect();
    assert_eq!(salida, vec!["comercio.caravana_sale", "comercio.trueque_expirado", "mercado.orden_expirada"], "fin de preparación, luego caducidades en el orden en que se programaron");
}

// ───────────────────────────── 8. Ruina y conquista ─────────────────────────────

/// Las dos plazas con muy poca madera de más (la de A caerá en ruinas) y una caravana de dos carros por plaza.
fn escenario_ruina(m: &M) -> (Escenario, u64, u64) {
    let mut e = escenario_con(m, 7, true, 200_000, true);
    let (a, b) = (e.a, e.b);
    let mut caravanas = [0, 0];
    for (i, (actor, plaza)) in [("ana", a), ("beto", b)].into_iter().enumerate() {
        caravanas[i] = e.ok(actor, Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
        for k in 0..2 {
            e.ok(actor, Comando::AnadirCarro { caravana: caravanas[i], tipo: TipoCarro::Basico });
            e.ok(actor, Comando::AnadirAnimal { caravana: caravanas[i], carro: k, animal: Animal::Buey });
        }
    }
    (e, caravanas[0], caravanas[1])
}

/// El instante en que la plaza A cae en ruinas en `escenario_ruina` (un ensayo previo: el mundo es determinista).
fn hora_de_la_ruina(m: &M) -> Instante {
    let (mut e, ..) = escenario_ruina(m);
    let a = e.a;
    for dia in 1..=150 {
        let t = e.n.ahora().max(dia * DIA);
        e.avanzar_hasta(t);
        if e.n.economia(a).is_none() {
            return hechos(&e.n, "asentamiento.ruinas")[0].0;
        }
    }
    panic!("la plaza A no cayó en ruinas en 150 días");
}

#[test]
fn una_plaza_que_cae_en_ruinas_cierra_sus_trueques_sin_penalizar_pierde_su_flota_y_hace_volver_las_que_iban() {
    let m = con_mundo!();
    let ruina = hora_de_la_ruina(m);
    let (mut e, ca, cb) = escenario_ruina(m);
    let (a, b) = (e.a, e.b);
    // Hasta una hora antes de la ruina: nada se ha tocado y la plaza A sigue viva. Entonces se montan los trueques y las caravanas en vuelo
    // con cobre (la ruina depende de la madera: no cambia).
    e.avanzar_hasta(ruina - HORA);
    assert!(e.n.economia(a).is_some());
    for plaza in [a, b] {
        e.dar(plaza, Cobre, 200_000);
    }
    let salir = e.n.ahora() + 2 * DIA;
    e.ok("ana", Comando::PrepararCaravana { caravana: ca, destino: b, carga: vec![(Cobre, 100_000)], salir_en: Some(salir) });
    e.ok("beto", Comando::PrepararCaravana { caravana: cb, destino: a, carga: vec![(Cobre, 50_000)], salir_en: Some(salir) });
    let t1 = e.ok("ana", Comando::ProponerTrueque { a, b, lineas_a: vec![(Cobre, 1_000)], lineas_b: vec![(Piedra, 1_000)] }).creadas[0];
    e.ok("beto", Comando::AceptarTrueque { acuerdo: t1 });
    let t2 = e.ok("beto", Comando::ProponerTrueque { a: b, b: a, lineas_a: vec![(Piedra, 1_000)], lineas_b: vec![(Cobre, 1_000)] }).creadas[0];
    let propia = e.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Cobre, cantidad: 1_000, precio: Some(PUNTO) }).creadas[0];
    let cerrada = e.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Cobre, cantidad: 1_000, precio: Some(PUNTO) }).creadas[0];
    e.ok("ana", Comando::CancelarOrdenMercado { orden: cerrada });
    let ajena = e.ok("beto", Comando::ColocarOrdenMercado { asentamiento: b, venta: true, recurso: Cobre, cantidad: 1_000, precio: Some(PUNTO) }).creadas[0];
    e.avanzar_hasta(ruina - 1);
    assert!(e.n.economia(a).is_some(), "la ruina llega exactamente a su hora");
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (0, 0));
    let b_cobre_antes = cantidad(&e.n, b, Cobre);
    assert_eq!(e.n.libro_ordenes().ordenes.len(), 2);

    e.avanzar_hasta(ruina);
    assert!(e.n.economia(a).is_none());
    // Los trueques de la plaza: expirados sin penalizar a nadie, con aviso a la socia.
    assert_eq!((e.n.acuerdo(t1).unwrap().estado, e.n.acuerdo(t2).unwrap().estado), (EstadoTrueque::Expirado, EstadoTrueque::Expirado));
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (0, 0));
    assert!(hechos(&e.n, "reputacion.cambio").is_empty());
    let cancelados = hechos(&e.n, "comercio.trueque_cancelado");
    assert_eq!(cancelados.iter().map(|h| (h.0, h.1.clone(), dato(h, "acuerdo"), dato(h, "socia"))).collect::<Vec<_>>(), vec![(ruina, vec![b], t1 as i64, b as i64), (ruina, vec![b], t2 as i64, b as i64)]);
    // La caravana de A se pierde con su carga; la de B que iba hacia A vuelve a casa con la suya.
    assert!(e.n.caravana(ca).is_none());
    assert_eq!(hechos(&e.n, "comercio.caravana_perdida").iter().map(|h| dato(h, "caravana")).collect::<Vec<_>>(), vec![ca as i64]);
    let vuelta = e.n.caravana(cb).unwrap();
    assert_eq!((vuelta.estado, vuelta.contenido.is_empty(), vuelta.destino, vuelta.carros.len()), (EstadoCaravana::Disponible, true, None, 2));
    assert_eq!(cantidad(&e.n, b, Cobre), b_cobre_antes + 50_000, "la carga volvió al almacén de B");
    let vuelve = hechos(&e.n, "comercio.caravana_vuelve");
    assert_eq!((vuelve.len(), dato(&vuelve[0], "caravana"), dato(&vuelve[0], "devuelto"), dato(&vuelve[0], "perdido")), (1, cb as i64, 50_000, 0));
    // Las órdenes de la plaza (en pie y cerradas) desaparecen; las de B siguen.
    let libro = e.n.libro_ordenes();
    assert!(libro.buscar(propia).is_none() && libro.buscar(cerrada).is_none());
    assert!(libro.orden(ajena).is_some());
    assert!(e.n.flota_de(a).is_empty() && !e.n.estado().comercio.ultima_caravana.contains_key(&a));
    // Nada de lo cerrado vuelve a caducar ni a prepararse: sus eventos se invalidaron con la entidad.
    e.avanzar(4 * DIA);
    assert!(hechos(&e.n, "comercio.trueque_expirado").is_empty());
    assert!(hechos(&e.n, "comercio.caravana_sale").is_empty());
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (0, 0));
    assert_eq!(e.n.libro_ordenes().buscar(ajena).unwrap().estado, EstadoOrden::Expirada, "la de B caducó por su plazo");
}

#[test]
fn la_conquista_pasa_la_flota_anula_los_trueques_devuelve_las_caravanas_y_destruye_las_aparcadas() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, true);
    let (a, b) = (e.a, e.b);
    let c = tercera_plaza(&mut e, m);
    let col_a = e.columna_en("ana-en-a", "ana", 1, a);
    // Una de A (se queda con la plaza), una de B en preparación hacia A, y dos de C aparcadas en A (una vacía y otra con carga).
    let ca = e.ok("ana", Comando::CrearCaravana { asentamiento: a }).creadas[0];
    let cb = e.ok("beto", Comando::CrearCaravana { asentamiento: b }).creadas[0];
    for k in 0..2 {
        e.ok("beto", Comando::AnadirCarro { caravana: cb, tipo: TipoCarro::Basico });
        e.ok("beto", Comando::AnadirAnimal { caravana: cb, carro: k, animal: Animal::Buey });
    }
    e.dar(b, Cobre, 100_000);
    let salir = e.n.ahora() + 2 * DIA;
    e.ok("beto", Comando::PrepararCaravana { caravana: cb, destino: a, carga: vec![(Cobre, 20_000)], salir_en: Some(salir) });
    let cc1 = e.caravana_aparcada_en("ana", c, col_a, a, 1);
    e.avanzar(11 * MIN);
    let cc2 = e.caravana_aparcada_en("ana", c, col_a, a, 1);
    e.dar(a, Madera, 10_000);
    e.ok("ana", Comando::MoverCargaAparcada { caravana: cc2, recurso: Madera, cantidad: 5_000, cargar: true });
    let tr = e.trueque(&[(Madera, 1_000)], &[(Cobre, 1_000)]);
    let orden = e.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Madera, cantidad: 1_000, precio: Some(PUNTO) }).creadas[0];

    let ahora = e.n.ahora();
    assert!(e.n.cerrar_comercio_por_conquista(ahora, a));
    assert!(!e.n.cerrar_comercio_por_conquista(ahora, 999));
    // La plaza sigue (con su flota y su mercado); el trueque se anula sin penalizar y avisa a la socia.
    assert!(e.n.economia(a).is_some());
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Expirado);
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (0, 0));
    let cancelados = hechos(&e.n, "comercio.trueque_cancelado");
    assert_eq!((cancelados.len(), cancelados[0].1.clone(), dato(&cancelados[0], "socia")), (1, vec![b], b as i64));
    assert!(e.n.caravana(ca).is_some(), "la flota pasa al conquistador: no se pierde ninguna");
    let flota = hechos(&e.n, "comercio.flota_traspasada");
    assert_eq!((flota.len(), flota[0].1.clone(), dato(&flota[0], "caravanas")), (1, vec![a], 1));
    assert!(e.n.libro_ordenes().orden(orden).is_some(), "las órdenes del Mercado se quedan");
    // La de B que iba hacia A vuelve con su carga; las dos de C aparcadas en A (vacía o con carga) se destruyen, sin captura (Doc 3.13.7).
    let vuelta = e.n.caravana(cb).unwrap();
    assert_eq!((vuelta.estado, vuelta.contenido.is_empty()), (EstadoCaravana::Disponible, true));
    assert!(e.n.caravana(cc1).is_none() && e.n.caravana(cc2).is_none(), "las aparcadas en la plaza conquistada se destruyen");
    let perdidas = hechos(&e.n, "comercio.caravana_perdida");
    assert_eq!(perdidas.iter().map(|h| dato(h, "caravana")).collect::<Vec<_>>(), vec![cc1 as i64, cc2 as i64]);
    // Lo anulado no vuelve a caducar ni a salir de la preparación.
    e.avanzar(3 * DIA);
    assert!(hechos(&e.n, "comercio.trueque_expirado").is_empty());
    assert!(hechos(&e.n, "comercio.caravana_sale").is_empty());
}

// ───────────────────────────── 9. Determinismo y recuperación ─────────────────────────────

/// Un guion de comercio (trueques, órdenes, mostrador, flota, caducidades) a horas fijas; `avanzar` decide cómo se trocea el avance entre
/// órdenes (las órdenes llevan su instante, así que sin avances intermedios el mundo es el mismo).
fn guion(e: &mut Escenario, avanzar: &dyn Fn(&mut Escenario, Instante)) {
    let (a, b) = (e.a, e.b);
    let col = e.columna_en("beto-en-a", "beto", 2, a);
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 200_000 });
    let base = e.n.ahora();
    let c = e.ok("ana", Comando::CrearCaravana { asentamiento: a }).creadas[0];
    for k in 0..2 {
        e.ok("ana", Comando::AnadirCarro { caravana: c, tipo: TipoCarro::Basico });
        e.ok("ana", Comando::AnadirAnimal { caravana: c, carro: k, animal: Animal::Buey });
    }
    // Reservada: el reparto automático no se la lleva y la lanza el guion a mano.
    e.ok("ana", Comando::ReservarCaravana { caravana: c, reservada: true });
    let t1 = e.trueque(&[(Madera, 10_000)], &[(Cobre, 10_000)]);
    e.ok("beto", Comando::ProponerTrueque { a: b, b: a, lineas_a: vec![(Piedra, 5_000)], lineas_b: vec![(Madera, 5_000)] });
    let v = e.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Madera, cantidad: 50_000, precio: Some(2 * PUNTO) }).creadas[0];
    avanzar(e, base + 10 * MIN);
    e.ok_en(base + 10 * MIN, "beto", Comando::ComerciarEnPlaza { ejercito: col, orden: v, cantidad: 20_000 });
    e.ok_en(base + 10 * MIN, "ana", Comando::PrepararCaravana { caravana: c, destino: b, carga: vec![(Madera, 100_000)], salir_en: Some(base + 3 * HORA) });
    avanzar(e, base + HORA);
    let v2 = e.ok_en(base + HORA, "ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: false, recurso: Cobre, cantidad: 5_000, precio: None }).creadas[0];
    e.ok_en(base + HORA, "ana", Comando::CancelarOrdenMercado { orden: v2 });
    avanzar(e, base + 5 * HORA);
    avanzar(e, base + DIA + HORA);
    avanzar(e, base + 3 * DIA);
    e.n.avanzar_hasta(base + 3 * DIA);
    assert_eq!(e.n.acuerdo(t1).unwrap().estado, EstadoTrueque::Expirado, "venció sin cumplirse: penaliza a las dos Facciones");
}

fn correr(m: &M, semilla: u64, avanzar: &dyn Fn(&mut Escenario, Instante)) -> Nucleo {
    let mut e = escenario(m, semilla, true);
    guion(&mut e, avanzar);
    e.n
}

#[test]
fn mismas_ordenes_dan_el_mismo_hash_y_da_igual_como_se_trocee_el_avance() {
    let m = con_mundo!();
    let normal = correr(m, 3, &|e, t| e.avanzar_hasta(t));
    assert_eq!(normal.hash_estado(), correr(m, 3, &|e, t| e.avanzar_hasta(t)).hash_estado(), "repetible");
    // El modo acelerado de un golpe: sin avances intermedios, solo el que hace cada orden al entrar.
    assert_eq!(normal.hash_estado(), correr(m, 3, &|_, _| {}).hash_estado());
    // En pasos pequeños e irregulares, como el modo en vivo con pulsos.
    let a_trozos = correr(m, 3, &|e, t| {
        let ahora = e.n.ahora();
        for s in 1..=5 {
            e.avanzar_hasta(ahora + (t - ahora).max(0) * s * s / 25);
        }
    });
    assert_eq!(normal.hash_estado(), a_trozos.hash_estado());
    assert_ne!(normal.hash_estado(), correr(m, 4, &|e, t| e.avanzar_hasta(t)).hash_estado(), "otra semilla, otro mundo");
    // El guion hizo de verdad lo que se espera.
    assert!(normal.estado().comercio.acuerdos.len() == 2 && normal.libro_ordenes().historial.len() >= 2);
    assert!(normal.reputacion(1) < 0 && normal.reputacion(2) < 0);
}

#[test]
fn recuperar_desde_foto_en_mitad_de_un_trueque_da_el_mismo_mundo() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9) };
    // El trueque se propone y acepta antes de la foto; vence después de recuperar. La caravana se prepara a mano y se cancela al final.
    let primera_mitad = |e: &mut Escenario| {
        let (a, b) = (e.a, e.b);
        let col = e.columna_en("beto-en-a", "beto", 2, a);
        e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 100_000 });
        e.trueque(&[(Madera, 10_000)], &[(Cobre, 10_000)]);
        let c = e.ok("ana", Comando::CrearCaravana { asentamiento: a }).creadas[0];
        e.ok("ana", Comando::ReservarCaravana { caravana: c, reservada: true });
        for k in 0..2 {
            e.ok("ana", Comando::AnadirCarro { caravana: c, tipo: TipoCarro::Basico });
            e.ok("ana", Comando::AnadirAnimal { caravana: c, carro: k, animal: Animal::Buey });
        }
        let salir = e.n.ahora() + 12 * HORA;
        e.ok("ana", Comando::PrepararCaravana { caravana: c, destino: b, carga: vec![(Madera, 50_000)], salir_en: Some(salir) });
        let v = e.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Madera, cantidad: 30_000, precio: Some(PUNTO) }).creadas[0];
        e.ok("beto", Comando::ComerciarEnPlaza { ejercito: col, orden: v, cantidad: 10_000 });
        c
    };
    let segunda_mitad = |e: &mut Escenario, c: u64| {
        e.avanzar(7 * HORA);
        e.ok("ana", Comando::CancelarCaravana { caravana: c });
        e.avanzar(2 * DIA);
    };

    // Sin interrumpir, en memoria.
    let sin_parar = {
        let mut e = escenario(m, 9, true);
        let c = primera_mitad(&mut e);
        e.avanzar(3 * HORA);
        segunda_mitad(&mut e, c);
        (e.n.hash_estado(), e.n.estado().comercio.clone())
    };

    // Con foto en mitad del trueque (activo, con su caducidad en la cola) y caída: se recupera de la foto + el registro y se sigue.
    let (a_mitad, caravana, cuenta) = {
        let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        let mut e = montar(n, m, true, 350_000, false);
        let c = primera_mitad(&mut e);
        assert!(e.n.metricas().ordenes_aceptadas > 20, "las órdenes de comercio entran al registro");
        e.n.foto_ahora();
        e.avanzar(3 * HORA);
        e.n.apagar().unwrap();
        (e.n.hash_estado(), c, e.cuenta)
    };
    let n = Nucleo::con_persistencia(con(ajustes), dir.path(), Default::default()).unwrap();
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), a_mitad, "foto + registro reconstruyen el mismo mundo, con el trueque activo en mitad");
    let ids: Vec<u64> = n.estado().asentamientos.keys().copied().collect();
    let mut e = Escenario { n, a: ids[0], b: ids[1], cuenta };
    assert_eq!(e.n.acuerdo(1).unwrap().estado, EstadoTrueque::Activo);
    segunda_mitad(&mut e, caravana);
    assert_eq!(e.n.estado().comercio, sin_parar.1, "el comercio recuperado sigue igual que el que no se interrumpió");
    assert_eq!(e.n.hash_estado(), sin_parar.0);
    assert_eq!(e.n.acuerdo(1).unwrap().estado, EstadoTrueque::Expirado, "venció tras recuperar");
    e.n.apagar().unwrap();
}

// ───────────────────────────── 10. El comercio en el estado (sin mundo) ─────────────────────────────

fn estado_con_comercio() -> Estado {
    use comercio::caravana::{Caravana, Carro, Escuadra};
    use comercio::orden_mercado::{OrdenMercado, TipoOrden};
    use comercio::reputacion::Reputacion;
    use comercio::trueque::AcuerdoTrueque;
    use comercio::viaje::{plan_uniforme, Peaje};
    let mut s = Estado::nuevo(Reglas::default(), "h".into(), 1, 0);
    let cr = s.reglas.economia.comercio.clone();
    let mut ac = AcuerdoTrueque::proponer(&cr.trueque, 4, 1, 2, &[(Madera, 100_000), (Oro, 5)], &[(Cobre, 30_000)], 10, true).unwrap();
    ac.aceptar(&cr.trueque, 99, true).unwrap();
    ac.lineas_a[0].cantidad_entregada = 7;
    s.comercio.acuerdos.push(ac);
    s.comercio.acuerdos.push(AcuerdoTrueque::proponer(&cr.trueque, 5, 2, 1, &[(Piedra, 1)], &[(Trigo, 2)], 11, true).unwrap());
    let mut o = OrdenMercado::colocar(&cr.mercado, 3, 1, TipoOrden::Venta, Madera, 9_000, Some(-4), 0, 20, true, true, 9_000).unwrap();
    o.cantidad_cumplida = 100;
    s.comercio.libro.colocar(o.clone());
    o.id = 2;
    o.estado = EstadoOrden::Cumplida;
    s.comercio.libro.historial.push(o);
    let mut c = Caravana::nueva(6, 1, Punto::new(5, 6));
    c.carros = vec![Carro { tipo: TipoCarro::Reforzado, animal: Some(Animal::Camello) }, Carro { tipo: TipoCarro::Basico, animal: None }];
    c.contenido.insert(Cobre, 123);
    c.destino = Some(2);
    c.estado = EstadoCaravana::Preparando;
    c.ruta = vec![Punto::new(5, 6), Punto::new(50, 60), Punto::new(500, 600)];
    c.plan = plan_uniforme(&c.ruta, 1_000, 24_000, 3);
    c.peajes = vec![Peaje { ciudad: 2, progreso: 300_000 }];
    c.origen_acuerdo = Some((4, Lado::B));
    c.prepara_hasta = Some(77_777);
    c.escolta = vec![Escuadra { id: 9, heroe: 8, liderazgo: 14 }];
    c.reservada_manual = true;
    c.anfitriona = Some(1);
    c.antes_de_batalla = Some(EstadoCaravana::Retornando);
    c.de_campamento = true;
    s.comercio.flota.push(c);
    s.comercio.reputacion.insert(1, Reputacion::desde(&cr.reputacion, -12 * PUNTO, 500, 2));
    s.comercio.reputacion.insert(7, Reputacion::desde(&cr.reputacion, 40 * PUNTO, 900, 0));
    s.comercio.ultima_caravana.insert(1, 42);
    let mut col = nucleo::Ejercito {
        id: 3,
        lider: "h".into(),
        propietario: "p".into(),
        faccion: 1,
        velocidad: 500,
        posicion: Punto::new(1, 2),
        plan: Default::default(),
        viveres: base::Ritmo::constante(10, 0),
        capacidad_viveres: 100,
        caravana_fundacion: false,
        suministro: [(Oro, 5), (Piedra, 6)].into_iter().collect(),
        caravanas_adjuntas: vec![6],
        version: 0,
    };
    col.caravanas_adjuntas.sort();
    s.ejercitos.insert(3, col);
    s.comercio.red.registrar_ruta(1, 2, 1, &[Punto::new(1_000, 1_000), Punto::new(9_000, 1_000), Punto::new(9_000, 9_000)], 5);
    s.comercio.red.registrar_ruta(2, 1, 2, &[Punto::new(9_000, 9_000), Punto::new(1_000, 1_000)], 6);
    s.comercio.reparto.insert(1, 60_000);
    s.comercio.mercenarios.mercado.contadores.insert(Cobre, 7_000);
    s.comercio.mercenarios.mercado.repone_en = 123;
    s.comercio.mercenarios.stock.insert(2, comercio::mercenarios::StockCampamento::nuevo(&cr));
    s.comercio.mercenarios.pulso = Some(999);
    s.comercio.flota[0].costes = vec![7, 8];
    s.comercio.flota[0].pausada_desde = Some(55);
    s.comercio.siguiente_acuerdo = 6;
    s.comercio.siguiente_orden = 4;
    s.comercio.siguiente_caravana = 7;
    s
}

#[test]
fn el_comercio_se_serializa_de_ida_y_vuelta_y_cualquier_campo_cambia_el_hash() {
    use comercio::reputacion::Reputacion;
    let reglas = Reglas::default();
    let rep = reglas.economia.comercio.reputacion.clone();
    let base = estado_con_comercio();
    let bytes = base.serializar();
    assert_eq!(&bytes[..4], b"BAN9");
    let vuelta = Estado::deserializar(&bytes, &reglas, "h").unwrap();
    assert_eq!(vuelta.serializar(), bytes, "serializar → deserializar → serializar es idéntico");
    assert_eq!(vuelta.comercio, base.comercio);
    // El formato anterior (BAN8) se rechaza con su motivo, y uno más viejo o ajeno también.
    let mut viejo = bytes.clone();
    viejo[3] = b'8';
    let Err(e) = Estado::deserializar(&viejo, &reglas, "h") else { panic!("BAN8 no debe admitirse") };
    assert!(e.to_string().contains("BAN8"), "{e}");
    viejo[3] = b'7';
    assert!(Estado::deserializar(&viejo, &reglas, "h").is_err());
    assert!(Estado::deserializar(b"XXXX", &reglas, "h").is_err());
    // Cada campo del comercio cambia el hash (y ninguno coincide con otro cambio).
    type Cambio = (&'static str, Box<dyn Fn(&mut nucleo::EstadoComercio)>);
    let cambios: Vec<Cambio> = vec![
        ("acuerdo.estado", Box::new(|c| c.acuerdos[0].estado = EstadoTrueque::Cumplido)),
        ("acuerdo.aceptado_en", Box::new(|c| c.acuerdos[0].aceptado_en = Some(1))),
        ("acuerdo.expira_en", Box::new(|c| c.acuerdos[0].expira_en += 1)),
        ("acuerdo.creado_en", Box::new(|c| c.acuerdos[0].creado_en += 1)),
        ("acuerdo.entregada", Box::new(|c| c.acuerdos[0].lineas_a[0].cantidad_entregada += 1)),
        ("acuerdo.total", Box::new(|c| c.acuerdos[0].lineas_b[0].cantidad_total += 1)),
        ("acuerdo.lado", Box::new(|c| c.acuerdos[1].b = 3)),
        ("acuerdo.todos", Box::new(|c| c.acuerdos.clear())),
        ("orden.cumplida", Box::new(|c| c.libro.ordenes[0].cantidad_cumplida += 1)),
        ("orden.precio", Box::new(|c| c.libro.ordenes[0].precio_unitario += 1)),
        ("orden.expira", Box::new(|c| c.libro.ordenes[0].expira_en += 1)),
        ("orden.estado", Box::new(|c| c.libro.ordenes[0].estado = EstadoOrden::Expirada)),
        ("historial", Box::new(|c| c.libro.historial.clear())),
        ("caravana.contenido", Box::new(|c| *c.flota[0].contenido.get_mut(&Cobre).unwrap() += 1)),
        ("caravana.carros", Box::new(|c| c.flota[0].carros[1].animal = Some(Animal::Buey))),
        ("caravana.estado", Box::new(|c| c.flota[0].estado = EstadoCaravana::Disponible)),
        ("caravana.posicion", Box::new(|c| c.flota[0].posicion = Punto::new(1, 1))),
        ("caravana.ruta", Box::new(|c| {
            c.flota[0].ruta.pop();
        })),
        ("caravana.plan", Box::new(|c| c.flota[0].plan = None)),
        ("caravana.peajes", Box::new(|c| c.flota[0].peajes[0].progreso += 1)),
        ("caravana.acuerdo", Box::new(|c| c.flota[0].origen_acuerdo = Some((4, Lado::A)))),
        ("caravana.prepara_hasta", Box::new(|c| c.flota[0].prepara_hasta = None)),
        ("caravana.escolta", Box::new(|c| c.flota[0].escolta[0].liderazgo += 1)),
        ("caravana.reservada", Box::new(|c| c.flota[0].reservada_manual = false)),
        ("caravana.anfitriona", Box::new(|c| c.flota[0].anfitriona = None)),
        ("caravana.antes_de_batalla", Box::new(|c| c.flota[0].antes_de_batalla = None)),
        ("caravana.de_campamento", Box::new(|c| c.flota[0].de_campamento = false)),
        ("reputacion.valor", Box::new({
            let r = rep.clone();
            move |c| {
                c.reputacion.insert(1, Reputacion::desde(&r, -11 * PUNTO, 500, 2));
            }
        })),
        ("reputacion.t0", Box::new({
            let r = rep.clone();
            move |c| {
                c.reputacion.insert(1, Reputacion::desde(&r, -12 * PUNTO, 501, 2));
            }
        })),
        ("reputacion.alianzas", Box::new({
            let r = rep.clone();
            move |c| {
                c.reputacion.insert(1, Reputacion::desde(&r, -12 * PUNTO, 500, 1));
            }
        })),
        ("reputacion.otra_faccion", Box::new(|c| {
            c.reputacion.insert(8, Reputacion::nueva(0));
        })),
        ("ultima_caravana", Box::new(|c| *c.ultima_caravana.get_mut(&1).unwrap() += 1)),
        ("caravana.costes", Box::new(|c| c.flota[0].costes[0] += 1)),
        ("caravana.pausada_desde", Box::new(|c| c.flota[0].pausada_desde = None)),
        ("red.ruta", Box::new(|c| {
            c.red.registrar_ruta(1, 2, 1, &[Punto::new(1_000, 1_000), Punto::new(9_000, 1_000)], 5);
        })),
        ("red.ultimo_lanzamiento", Box::new(|c| {
            c.red.registrar_ruta(2, 1, 2, &[Punto::new(9_000, 9_000), Punto::new(1_000, 1_000)], 7);
        })),
        ("red.arista_nueva", Box::new(|c| {
            c.red.registrar_ruta(3, 4, 1, &[Punto::new(20_000, 20_000), Punto::new(30_000, 20_000)], 6);
        })),
        ("reparto", Box::new(|c| *c.reparto.get_mut(&1).unwrap() += 1)),
        ("mercenarios.contadores", Box::new(|c| *c.mercenarios.mercado.contadores.get_mut(&Cobre).unwrap() += 1)),
        ("mercenarios.repone_en", Box::new(|c| c.mercenarios.mercado.repone_en += 1)),
        ("mercenarios.stock", Box::new(|c| *c.mercenarios.stock.get_mut(&2).unwrap().stock.get_mut(&Cobre).unwrap() += 1)),
        ("mercenarios.pulso", Box::new(|c| c.mercenarios.pulso = None)),
        ("siguiente_acuerdo", Box::new(|c| c.siguiente_acuerdo += 1)),
        ("siguiente_orden", Box::new(|c| c.siguiente_orden += 1)),
        ("siguiente_caravana", Box::new(|c| c.siguiente_caravana += 1)),
    ];
    let mut vistos = std::collections::HashSet::from([base.hash()]);
    for (nombre, f) in cambios {
        let mut s = Estado::deserializar(&bytes, &reglas, "h").unwrap();
        f(&mut s.comercio);
        assert!(vistos.insert(s.hash()), "el hash no cambió (o coincidió con otro cambio) al tocar {nombre}");
    }
    // El carro y las caravanas adjuntas viajan con la columna.
    type CambioColumna = (&'static str, Box<dyn Fn(&mut nucleo::Ejercito)>);
    let columna: Vec<CambioColumna> = vec![("suministro", Box::new(|e| *e.suministro.get_mut(&Oro).unwrap() += 1)), ("adjuntas", Box::new(|e| e.caravanas_adjuntas.push(9)))];
    for (nombre, f) in columna {
        let mut s = Estado::deserializar(&bytes, &reglas, "h").unwrap();
        f(s.ejercitos.get_mut(&3).unwrap());
        assert!(vistos.insert(s.hash()), "el hash no cambió (o coincidió con otro cambio) al tocar {nombre} de la columna");
    }
}

#[test]
fn la_huella_de_las_reglas_incluye_los_bloques_de_comercio_y_militares() {
    use sha2::{Digest, Sha256};
    let r = Reglas::default();
    let sin_comercio = asentamiento::datos::texto_de_parametros() + &asentamiento::datos::texto_de_parametros_de(nucleo::urbanismo::DOC_TRAZADO);
    let con_comercio = sin_comercio.clone() + &comercio::datos::texto_de_parametros();
    let con_militar = con_comercio.clone() + &militar::datos::texto_de_parametros();
    assert_eq!(r.economia.huella(), nucleo::hex(&Sha256::digest(con_militar).into()));
    assert_ne!(r.economia.huella(), nucleo::hex(&Sha256::digest(con_comercio).into()), "los bloques militares entran en la huella");
    assert_ne!(r.economia.huella(), nucleo::hex(&Sha256::digest(sin_comercio).into()));
    // Una foto con otra huella (otras cifras de comercio) se niega.
    let bytes = estado_con_comercio().serializar();
    let huella = r.economia.huella().as_bytes().to_vec();
    let mut sucia = bytes.clone();
    let pos = sucia.windows(huella.len()).position(|w| w == &huella[..]).expect("la foto lleva la huella");
    sucia[pos] = if sucia[pos] == b'0' { b'1' } else { b'0' };
    assert!(Estado::deserializar(&sucia, &r, "h").is_err());
}

// ───────────────────────────── 11. Rendimiento ─────────────────────────────

/// El comercio no añade coste por plaza al paso de un minuto (la medida de 1.000 plazas × 3 días es
/// `cargo test --release -p nucleo --test asentamientos rendimiento -- --ignored --nocapture`, ver `tests/asentamientos.rs`). Aquí se
/// comprueba que, sin órdenes de comercio, los pasos de las plazas no tocan su estado ni programan eventos de comercio.
#[test]
fn los_pasos_de_las_plazas_no_tocan_el_estado_de_comercio_ni_programan_eventos() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, false);
    let antes = e.n.estado().comercio.clone();
    let eventos0 = e.n.metricas().eventos;
    e.avanzar(3 * DIA);
    assert!(e.n.metricas().eventos - eventos0 >= 2 * 3 * 1440 - 4, "corren los pasos de minuto de las dos plazas");
    assert_eq!(e.n.estado().comercio, antes);
    assert!(e.n.estado().cola.volcar().iter().all(|ev| !matches!(ev.carga, nucleo::Carga::Comercio { .. } | nucleo::Carga::Reputacion(_))));
}

