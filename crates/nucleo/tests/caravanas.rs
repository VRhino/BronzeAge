//! Las caravanas comerciales viajan de verdad por el mapa (fase 3B-2, Comercio_Caravanas §4.3, §6, §7, §9, §10): trazado y plan sobre el
//! mundo real, peajes, llegada y regreso, reparto automático a los trueques, red de caminos, mercado de mercenarios, ruina y conquista con
//! caravanas en ruta, hook de batalla, determinismo y recuperación desde foto. Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

use asentamiento::catalogo::Tipo;
use asentamiento::datos::UNO;
use asentamiento::Recurso::{self, *};
use base::{Instante, Punto, MS_POR_MINUTO};
use comercio::caravana::{Animal, EstadoCaravana, TipoCarro};
use comercio::precio::{comision_de_entrega, ContextoComision, TablaPrecios};
use comercio::trueque::EstadoTrueque;
use comercio::{Carga as Mercancia, Lado};
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{rechazos, Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Resultado, Respuesta};
use rutas::{Opciones, Rutas};
use std::sync::{Arc, OnceLock};

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;
const DIA: i64 = 24 * HORA;
const PUNTO: i64 = 1_000_000;

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

fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

/// Un sitio verde con mucho bosque a 10 u cerca del campamento `i`: una plaza que no cae en ruinas por falta de madera.
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

type Hecho = (Instante, Vec<u64>, Vec<(&'static str, i64)>);

fn hechos(n: &Nucleo, codigo: &str) -> Vec<Hecho> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).map(|e| (e.en, e.asentamientos.clone(), e.datos.clone())).collect()
}

fn dato(h: &Hecho, clave: &str) -> i64 {
    h.2.iter().find(|(k, _)| *k == clave).unwrap_or_else(|| panic!("el hecho no lleva «{clave}»: {h:?}")).1
}

fn mundo_de(n: &Nucleo, r: Recurso) -> i64 {
    let e = n.estado();
    e.asentamientos.values().map(|a| a.economia.almacen.cantidad(r)).sum::<i64>()
        + e.ejercitos.values().map(|c| c.columna.carga.get(&r).copied().unwrap_or(0)).sum::<i64>()
        + e.comercio.flota.iter().map(|c| c.contenido.get(&r).copied().unwrap_or(0)).sum::<i64>()
}

fn cantidad(n: &Nucleo, plaza: u64, r: Recurso) -> i64 {
    n.economia(plaza).unwrap().almacen.cantidad(r)
}

fn pos(n: &Nucleo, plaza: u64) -> Punto {
    n.estado().asentamientos[&plaza].posicion
}

/// Dos plazas de Facciones distintas (F1 con «ana», F2 con «beto») con Mercado activo; el tiempo corre solo cuando se pide.
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
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    fn ok_en(&mut self, t: Instante, actor: &str, c: Comando) -> Respuesta {
        let p = self.peticion(actor, c);
        let r = self.n.aplicar_en(t, p);
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
    fn dar(&mut self, plaza: u64, recurso: Recurso, cantidad: i64) {
        let r = self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
    }
    /// Una caravana de `carros` carros con buey en la plaza, reservada (fuera del reparto) si se pide.
    fn caravana(&mut self, actor: &str, plaza: u64, carros: u32, reservada: bool) -> u64 {
        let id = self.ok(actor, Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
        if reservada {
            self.ok(actor, Comando::ReservarCaravana { caravana: id, reservada: true });
        }
        for k in 0..carros {
            self.ok(actor, Comando::AnadirCarro { caravana: id, tipo: TipoCarro::Basico });
            self.ok(actor, Comando::AnadirAnimal { caravana: id, carro: k, animal: Animal::Buey });
        }
        id
    }
    fn trueque(&mut self, a: &[(Recurso, i64)], b: &[(Recurso, i64)]) -> u64 {
        let (pa, pb) = (self.a, self.b);
        let id = self.ok("ana", Comando::ProponerTrueque { a: pa, b: pb, lineas_a: a.to_vec(), lineas_b: b.to_vec() }).creadas[0];
        self.ok("beto", Comando::AceptarTrueque { acuerdo: id });
        id
    }
    fn caravana_de(&self, id: u64) -> &comercio::caravana::Caravana {
        self.n.caravana(id).expect("existe la caravana")
    }
    /// Un héroe de la cuenta, residente en el campamento `casa`, que va en la columna `columna` (la administración lo coloca: aún no hay `salirAlMundo`).
    fn heroe_en_columna(&mut self, cuenta: &str, casa: u32, columna: u64) -> u64 {
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: casa, controlador: militar::heroe::Controlador::Humano }).creadas[0];
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: militar::presencia::Ubicacion::Columna(columna) });
        h
    }
}

fn fundar(n: &mut Nucleo, t: Instante, nombre: &str, dueno: &str, faccion: u32, p: Punto) -> u64 {
    let e = n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap();
    assert!(n.aplicar_en(t, Peticion::nueva("admin", &format!("c-{nombre}"), Comando::DarCaravanaFundacion { ejercito: e }).de_admin()).aceptada());
    let r = n.aplicar_en(t, Peticion::nueva(dueno, &format!("f-{nombre}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

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

fn escenario_con(m: &M, semilla: u64, madera: i64, pausar_a: bool) -> Escenario {
    montar(Nucleo::nuevo(arranque(m, semilla)), m, true, madera, pausar_a)
}

fn escenario(m: &M, semilla: u64) -> Escenario {
    escenario_con(m, semilla, 350_000, false)
}



/// La comisión que debe acuñar la entrega de `carga` de `origen` a `destino` con el estado de ahora (precios con el stock de ahora).
fn comision_esperada(n: &Nucleo, origen: u64, destino: u64, carga: &[(Recurso, i64)]) -> i64 {
    let r = comercio::Reglas::cargar();
    let e = n.estado();
    let precios = TablaPrecios::de(&r.precio, e.asentamientos.values().map(|a| &a.economia.almacen));
    let c: Mercancia = carga.iter().copied().collect();
    let (po, pd) = (&e.asentamientos[&origen], &e.asentamientos[&destino]);
    comision_de_entrega(&r.comision, precios.valor(&c), &ContextoComision { mismo_asentamiento: false, misma_faccion: po.faccion == pd.faccion, factor_comision_externa: UNO, distancia_recta: po.posicion.dist(pd.posicion) })
}

/// Duración de un viaje por tierra entre dos plazas con el A* sin red: coste de terreno × espaciado / velocidad (milésimas por minuto).
fn duracion_esperada(m: &M, desde: Punto, hasta: Punto, velocidad: i64) -> i64 {
    let c = m.1.ruta(desde, hasta, &Opciones::default()).expect("hay ruta");
    (c.coste_total() as i128 * m.1.espaciado() as i128 * 60 + velocidad as i128 / 2).div_euclid(velocidad as i128) as i64
}

fn fin_del_plan(e: &Escenario, c: u64) -> Instante {
    e.caravana_de(c).plan.as_ref().and_then(|p| p.fin()).expect("la caravana tiene plan")
}

// ───────────────────────────── 1. Lanzar, viajar, llegar y volver ─────────────────────────────

#[test]
fn un_trueque_se_cumple_por_reparto_automatico_con_comision_reputacion_y_regreso() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    e.dar(b, Estano, 100_000);
    let ca = e.caravana("ana", a, 1, false);
    let cb = e.caravana("beto", b, 1, false);
    let (cobre_a0, estano_b0, cobre_b0, estano_a0) = (cantidad(&e.n, a, Cobre), cantidad(&e.n, b, Estano), cantidad(&e.n, b, Cobre), cantidad(&e.n, a, Estano));
    let (cobre_mundo, estano_mundo) = (mundo_de(&e.n, Cobre), mundo_de(&e.n, Estano));
    // Mientras no hay trueque, las caravanas libres esperan en su ciudad.
    assert_eq!((e.caravana_de(ca).estado, e.caravana_de(cb).estado), (EstadoCaravana::Disponible, EstadoCaravana::Disponible));

    let t0 = e.n.ahora();
    let tr = e.trueque(&[(Cobre, 60_000)], &[(Estano, 30_000)]);
    // Al aceptar se reevalúa el reparto: cada plaza carga lo suyo y manda su caravana.
    let (c_a, c_b) = (e.caravana_de(ca).clone(), e.caravana_de(cb).clone());
    assert_eq!((c_a.estado, c_a.origen_acuerdo, c_a.destino, c_a.contenido[&Cobre]), (EstadoCaravana::EnTransito, Some((tr, Lado::A)), Some(b as u32), 60_000));
    assert_eq!((c_b.estado, c_b.origen_acuerdo, c_b.destino, c_b.contenido[&Estano]), (EstadoCaravana::EnTransito, Some((tr, Lado::B)), Some(a as u32), 30_000));
    assert!(c_a.peajes.is_empty() && c_b.peajes.is_empty(), "no hay ciudades ajenas por el camino");
    assert_eq!(cantidad(&e.n, a, Cobre), cobre_a0 - 60_000, "la carga sale ya del almacén");
    assert_eq!(mundo_de(&e.n, Cobre), cobre_mundo, "y no se crea ni se pierde nada");
    let sale = hechos(&e.n, "comercio.caravana_sale");
    assert_eq!(sale.iter().map(|h| (h.0, h.1.clone())).collect::<Vec<_>>(), vec![(t0, vec![a, b]), (t0, vec![b, a])]);
    // El plan es el de la caravana: empieza en su plaza, acaba en la otra y tarda lo que dicen la velocidad (buey, 24) y el terreno. La de A
    // sale primero y su ruta entra en la red; la de B, que sale justo después, ya va por camino (la mitad del coste sobre las mismas aristas).
    for (c, desde, hasta) in [(&c_a, a, b), (&c_b, b, a)] {
        let plan = c.plan.as_ref().unwrap();
        assert_eq!((plan.vertices.first().unwrap().p, plan.vertices.last().unwrap().p), (pos(&e.n, desde), pos(&e.n, hasta)));
        assert!(plan.vertices.first().unwrap().en == t0 && plan.generacion > 0);
    }
    let esperada = duracion_esperada(m, pos(&e.n, a), pos(&e.n, b), 24_000);
    assert_eq!(c_a.plan.as_ref().unwrap().fin().unwrap() - t0, esperada);
    let d_b = c_b.plan.as_ref().unwrap().fin().unwrap() - t0;
    assert!(d_b * 100 > esperada * 48 && d_b * 100 < esperada * 56, "la segunda ruta va por el camino de la primera: {d_b} contra {esperada}");
    // El reparto no vuelve a mandar nada: un cargamento por lado.
    e.avanzar(MIN);
    assert_eq!(hechos(&e.n, "comercio.caravana_sale").len(), 2);

    // Llegan en su instante exacto: entregan, cobran la comisión y salen de vuelta.
    let mut llegadas = vec![(fin_del_plan(&e, ca), ca, a, b, Cobre, 60_000), (fin_del_plan(&e, cb), cb, b, a, Estano, 30_000)];
    llegadas.sort();
    for (fin, c, origen, destino, recurso, cuanto) in llegadas {
        e.avanzar_hasta(fin - 1);
        assert_eq!(e.caravana_de(c).estado, EstadoCaravana::EnTransito, "aún no ha llegado");
        let esperada = comision_esperada(&e.n, origen, destino, &[(recurso, cuanto)]);
        let (antes, oro_antes) = (cantidad(&e.n, destino, recurso), cantidad(&e.n, destino, Oro));
        e.avanzar_hasta(fin);
        let h = hechos(&e.n, "comercio.caravana_llega").into_iter().find(|h| dato(h, "caravana") == c as i64).expect("llegó");
        assert_eq!((h.0, h.1.clone(), dato(&h, "perdido")), (fin, vec![origen, destino], 0));
        let comision = dato(&h, "comision");
        assert!(comision > 0 && comision <= esperada && esperada - comision < 1_000, "comisión {comision} contra {esperada}: se acuña en unidades enteras, hacia abajo");
        assert_eq!(cantidad(&e.n, destino, recurso) - antes, cuanto, "la mercancía llega entera");
        assert!((cantidad(&e.n, destino, Oro) - oro_antes - comision).abs() <= 1_000, "la comisión se acuña en el destino");
        let c = e.caravana_de(c);
        assert_eq!((c.estado, c.contenido.is_empty(), c.origen_acuerdo, c.posicion), (EstadoCaravana::Retornando, true, None, pos(&e.n, destino)));
        // El regreso desanda la ida con las mismas duraciones.
        let plan = c.plan.as_ref().unwrap();
        assert_eq!(plan.inicio(), Some(fin));
        assert_eq!(plan.fin().unwrap() - fin, fin - t0);
    }
    assert_eq!((cantidad(&e.n, b, Cobre) - cobre_b0, cantidad(&e.n, a, Estano) - estano_a0), (60_000, 30_000));
    assert_eq!((cantidad(&e.n, a, Cobre), cantidad(&e.n, b, Estano)), (cobre_a0 - 60_000, estano_b0 - 30_000));
    // Cumplido: reputación y experiencia entre Facciones, y lo comerciado alimenta la reposición de los mercenarios.
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Cumplido);
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (5 * PUNTO, 5 * PUNTO));
    assert_eq!(hechos(&e.n, "faccion.experiencia").iter().filter(|h| dato(h, "motivo") == 1).count(), 2, "la del trueque (las demás son de edificios)");
    let cont = &e.n.mercenarios().mercado.contadores;
    assert_eq!((cont[&Cobre], cont[&Estano]), (60_000, 30_000));
    assert_eq!(mundo_de(&e.n, Cobre), cobre_mundo);
    assert_eq!(mundo_de(&e.n, Estano), estano_mundo);

    // El regreso desanda la ida con las mismas duraciones y nadie cobra peaje; al llegar a casa queda disponible y limpia.
    let mut vueltas: Vec<(Instante, u64, Punto)> = [ca, cb]
        .into_iter()
        .filter_map(|c| {
            let car = e.caravana_de(c);
            // La que fue por el camino puede haber vuelto ya a casa mientras la otra entregaba.
            let Some(plan) = car.plan.as_ref() else {
                assert_eq!(car.estado, EstadoCaravana::Disponible);
                return None;
            };
            assert_eq!(plan.vertices.last().unwrap().p, pos(&e.n, car.origen as u64));
            Some((plan.fin().unwrap(), c, pos(&e.n, car.origen as u64)))
        })
        .collect();
    vueltas.sort_by_key(|v| (v.0, v.1));
    for (fin, c, casa) in vueltas {
        e.avanzar_hasta(fin);
        let car = e.caravana_de(c);
        assert_eq!((car.estado, car.destino, car.plan.is_none(), car.ruta.is_empty(), car.posicion), (EstadoCaravana::Disponible, None, true, true, casa));
    }
    assert_eq!(hechos(&e.n, "comercio.caravana_en_casa").len(), 2);
    for c in [ca, cb] {
        assert_eq!(e.caravana_de(c).estado, EstadoCaravana::Disponible);
    }
    // El cooldown de creación (10 min, compartido con la Fundación) cuenta desde la creación, no desde el regreso.
    let r = e.ok("ana", Comando::CrearCaravana { asentamiento: a });
    assert_eq!(r.creadas.len(), 1);
    assert_eq!(e.rechazada("ana", Comando::CrearCaravana { asentamiento: a }), rechazos::CARAVANA_EN_COOLDOWN);
    // Nada se repite: el trueque cumplido no manda más caravanas y sus eventos no vuelven a salir.
    e.avanzar(DIA);
    assert_eq!((hechos(&e.n, "comercio.caravana_sale").len(), hechos(&e.n, "comercio.caravana_llega").len()), (2, 2));
}

#[test]
fn un_lado_sin_mercancia_arma_un_pulso_de_un_minuto_que_se_detiene_solo() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    let ca = e.caravana("ana", a, 1, false);
    // A no tiene estaño: el trueque nace con la caravana libre pero sin nada que cargar.
    assert_eq!(cantidad(&e.n, a, Estano), 0);
    let t0 = e.n.ahora();
    let tr = e.trueque(&[(Estano, 30_000)], &[(Cobre, 1_000)]);
    assert_eq!(e.caravana_de(ca).estado, EstadoCaravana::Disponible);
    assert_eq!(e.n.estado().comercio.reparto.get(&a), Some(&(t0 + MIN)), "pulso de un minuto en la plaza de origen con un lado sin cargar");
    // El pulso se repite mientras falte mercancía y no hace nada más: ni hechos ni versión.
    let hechos_antes = |e: &Escenario| e.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|ev| ev.codigo.starts_with("comercio.")).count();
    let h = hechos_antes(&e);
    e.avanzar(5 * MIN);
    assert_eq!(hechos_antes(&e), h);
    assert_eq!(e.n.estado().comercio.reparto.get(&a), Some(&(t0 + 6 * MIN)));
    // Llega estaño al almacén y el siguiente pulso lo carga con cualquier cantidad positiva.
    e.dar(a, Estano, 7_000);
    e.avanzar_hasta(t0 + 6 * MIN);
    let c = e.caravana_de(ca);
    assert_eq!((c.estado, c.contenido[&Estano], c.origen_acuerdo), (EstadoCaravana::EnTransito, 7_000, Some((tr, Lado::A))));
    assert!(e.n.estado().comercio.reparto.is_empty(), "ya no hay nada pendiente sin cargar: el pulso se detiene");
    let eventos = e.n.eventos_en_cola();
    let sin_pulsos = e.n.estado().cola.volcar().iter().filter(|ev| matches!(ev.carga, nucleo::Carga::Comercio { suceso: nucleo::SucesoComercio::PulsoReparto, .. })).count();
    assert_eq!(sin_pulsos, 0, "{eventos} eventos en cola");
    // Una plaza sin trueques no programa ni un pulso.
    assert!(!e.n.estado().comercio.reparto.contains_key(&b));
}

// ───────────────────────────── 2. El viaje sobre el mapa real ─────────────────────────────

#[test]
fn el_tiempo_de_viaje_sale_de_la_velocidad_del_animal_y_del_coste_del_terreno() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    e.dar(a, Oro, 300_000);
    let (pa, pb) = (pos(&e.n, a), pos(&e.n, b));
    let buey = e.caravana("ana", a, 1, true);
    e.avanzar(11 * MIN);
    let caballo = e.ok("ana", Comando::CrearCaravana { asentamiento: a }).creadas[0];
    e.ok("ana", Comando::AnadirCarro { caravana: caballo, tipo: TipoCarro::Basico });
    e.ok("ana", Comando::AnadirAnimal { caravana: caballo, carro: 0, animal: Animal::Caballo });
    // Salida inmediata (un solo carro y sin hora de salida): en tránsito desde el comando, sin `caravana_sale`. La primera ruta no tiene camino:
    // el caballo (36) tarda lo que dicen el terreno y su velocidad; el buey (24) sale después, ya por el camino que el caballo abrió.
    let t = e.n.ahora();
    for c in [caballo, buey] {
        e.ok("ana", Comando::PrepararCaravana { caravana: c, destino: b, carga: vec![(Cobre, 10_000)], salir_en: None });
        assert_eq!(e.caravana_de(c).estado, EstadoCaravana::EnTransito);
        assert_eq!(e.caravana_de(c).prepara_hasta, None);
    }
    assert!(hechos(&e.n, "comercio.caravana_sale").is_empty());
    let (d_buey, d_caballo) = (fin_del_plan(&e, buey) - t, fin_del_plan(&e, caballo) - t);
    assert_eq!(d_caballo, duracion_esperada(m, pa, pb, 36_000), "caballo: 36 unidades por minuto sobre el coste del terreno");
    let sin_camino = duracion_esperada(m, pa, pb, 24_000);
    assert!(d_buey * 100 > sin_camino * 48 && d_buey * 100 < sin_camino * 56, "buey: 24 y por el camino, la mitad: {d_buey} contra {sin_camino}");
    assert!((duracion_esperada(m, pa, pb, 24_000) * 2 - duracion_esperada(m, pa, pb, 36_000) * 3).abs() <= 3, "el caballo es 1,5 veces más rápido");
    // El coste del terreno pesa de verdad: más que la recta a la velocidad del caballo.
    let recta = pa.dist(pb) * MIN / 36_000;
    assert!(d_caballo > recta, "el terreno y los rodeos alargan el viaje: {d_caballo} ms contra {recta} ms en línea recta");
    // Mientras viaja, su posición es función del plan: a mitad de camino no está ni en origen ni en destino.
    e.avanzar_hasta(t + d_caballo / 2);
    let medio = e.n.posicion_de_caravana(caballo, e.n.ahora()).unwrap();
    assert!(medio != pa && medio != pb && medio.dist(pa) < pa.dist(pb) + 1_000);
    // Un rechazo: sin ruta por tierra o a una plaza que no existe no sale ni gasta nada.
    assert_eq!(e.rechazada("ana", Comando::PrepararCaravana { caravana: caballo, destino: b, carga: vec![(Cobre, 1_000)], salir_en: None }), rechazos::CARAVANA_NO_DISPONIBLE);
}

/// Funda en el trayecto de A a B una plaza de `faccion` y la deja en el camino. Devuelve su id.
fn plaza_en_el_camino(e: &mut Escenario, m: &M, faccion: u32) -> u64 {
    let (pa, pb) = (pos(&e.n, e.a), pos(&e.n, e.b));
    let camino = m.1.ruta(pa, pb, &Opciones::default()).unwrap();
    let (desde, hasta) = (camino.puntos.len() * 35 / 100, camino.puntos.len() * 65 / 100);
    let p = camino.puntos[desde..hasta]
        .iter()
        .copied()
        .find(|&p| p.dist(pa) > 40_000 && p.dist(pb) > 40_000 && e.n.viabilidad(p, faccion).is_some_and(|v| v.motivo.is_none()))
        .expect("un sitio fundable en el camino");
    let t = e.n.ahora();
    if faccion == 1 {
        e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 });
    }
    fundar(&mut e.n, t, "c", if faccion == 1 { "ana" } else { "carlos" }, faccion, p)
}

#[test]
fn una_ciudad_ajena_en_el_camino_cobra_el_peaje_a_la_ida_y_no_al_regreso() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    let c = plaza_en_el_camino(&mut e, m, 3);
    e.dar(a, Cobre, 200_000);
    let car = e.caravana("ana", a, 1, true);
    let cobre_c0 = cantidad(&e.n, c, Cobre);
    let t = e.n.ahora();
    e.ok("ana", Comando::PrepararCaravana { caravana: car, destino: b, carga: vec![(Cobre, 100_000)], salir_en: None });
    let lanzada = e.caravana_de(car).clone();
    // El trazado pasa por la ciudad ajena: un peaje fijado al lanzar, en el avance en que el plan llega a su centro.
    assert_eq!(lanzada.peajes.len(), 1);
    assert_eq!(lanzada.peajes[0].ciudad, c as u32);
    assert!(lanzada.ruta.contains(&pos(&e.n, c)), "la ruta pasa por el centro de la ciudad ajena");
    let plan = lanzada.plan.as_ref().unwrap();
    let cruce = comercio::viaje::instante_de_progreso(plan, lanzada.peajes[0].progreso).unwrap();
    assert!(cruce > t && cruce < plan.fin().unwrap());
    e.avanzar_hasta(cruce - 1);
    assert!(hechos(&e.n, "comercio.peaje_paso").is_empty());
    assert_eq!(e.caravana_de(car).contenido[&Cobre], 100_000);
    e.avanzar_hasta(cruce);
    let peaje = hechos(&e.n, "comercio.peaje_paso");
    assert_eq!((peaje.len(), peaje[0].0, peaje[0].1.clone(), dato(&peaje[0], "ciudad"), dato(&peaje[0], "cobre")), (1, cruce, vec![c], c as i64, 2_000), "el 2 % de lo que lleva");
    assert_eq!(cantidad(&e.n, c, Cobre) - cobre_c0, 2_000, "el peaje es de la ciudad cruzada");
    assert_eq!((e.caravana_de(car).contenido[&Cobre], e.caravana_de(car).peajes.len()), (98_000, 0));
    // El destino recibe lo que queda y el regreso, vacío, no paga nada.
    let cobre_b0 = cantidad(&e.n, b, Cobre);
    let fin = fin_del_plan(&e, car);
    e.avanzar_hasta(fin);
    assert_eq!(cantidad(&e.n, b, Cobre) - cobre_b0, 98_000);
    assert_eq!(e.caravana_de(car).estado, EstadoCaravana::Retornando);
    let vuelta = fin_del_plan(&e, car);
    e.avanzar_hasta(vuelta + MIN);
    assert_eq!((hechos(&e.n, "comercio.peaje_paso").len(), cantidad(&e.n, c, Cobre) - cobre_c0), (1, 2_000));
    assert_eq!(e.caravana_de(car).estado, EstadoCaravana::Disponible);
}

#[test]
fn una_ciudad_de_la_propia_faccion_en_el_camino_no_cobra_peaje() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    let c = plaza_en_el_camino(&mut e, m, 1);
    e.dar(a, Cobre, 100_000);
    let car = e.caravana("ana", a, 1, true);
    e.ok("ana", Comando::PrepararCaravana { caravana: car, destino: b, carga: vec![(Cobre, 100_000)], salir_en: None });
    assert!(e.caravana_de(car).peajes.is_empty());
    let directa = m.1.ruta(pos(&e.n, a), pos(&e.n, b), &Opciones::default()).unwrap();
    assert_eq!(e.caravana_de(car).ruta, directa.puntos, "sin paso forzado: la ruta es la del A* sin más, y {c} es solo una plaza que queda en el camino");
}

/// En la `VISTA` de la Facción la red llega con el escalón de cada arista, en el orden de `aristas`, y con sus rutas vigentes (4H T7): la que acaba de entrar, con sus aristas y su lanzamiento.
#[test]
fn la_vista_lleva_la_red_con_el_escalon_de_cada_arista_y_las_rutas_propias() {
    use contrato::v1::entidad::Cuerpo;
    use nucleo::proyeccion::vista::{Mirada, Perspectiva};
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    let car = e.caravana("ana", a, 2, true);
    let t = e.n.ahora();
    e.ok("ana", Comando::PrepararCaravana { caravana: car, destino: b, carga: vec![(Cobre, 50_000)], salir_en: Some(t + HORA) });
    let faccion = e.n.estado().asentamientos[&a].faccion;
    let rect = vision::Rect { min: Punto::new(0, 0), max: Punto::new(3_000_000, 3_000_000) };
    let mirada = Mirada::de(&e.n, Perspectiva::Faccion(faccion));
    let red = mirada.contenido(None, rect, &Default::default()).into_iter().find_map(|v| if let Some(Cuerpo::RedCaminos(r)) = v.cuerpo() { Some(r) } else { None }).expect("la red se ve");
    let sendero = u32::from(comercio::red_caminos::escalon_de_peso(&e.n.estado().reglas.economia.comercio.red, 1));
    assert!(!red.aristas.is_empty() && red.escalones.len() == red.aristas.len() && red.escalones.iter().all(|&s| s == sendero), "una ruta: peso 1 en cada arista");
    assert_eq!(red.rutas.len(), 1);
    let r = &red.rutas[0];
    assert_eq!((r.id.clone(), r.origen_id.clone(), r.destino_id.clone(), r.ultimo_lanzamiento_ms), (format!("ruta-{a}-{b}"), format!("as-{a}"), format!("as-{b}"), t));
    assert!(red.aristas.iter().all(|x| r.aristas.contains(x)), "las aristas que se ven son de la ruta");
}

// ───────────────────────────── 3. Preparar, cancelar e invalidar ─────────────────────────────

#[test]
fn cancelar_la_preparacion_deja_el_camino_y_sus_eventos_no_vuelven() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    let car = e.caravana("ana", a, 2, true);
    let t = e.n.ahora();
    e.ok("ana", Comando::PrepararCaravana { caravana: car, destino: b, carga: vec![(Cobre, 50_000)], salir_en: Some(t + HORA) });
    // La ruta entra en la red ya, aunque luego se cancele.
    let pesos = e.n.pesos_de_red();
    assert!(!pesos.is_empty() && pesos.values().all(|&p| p == 1));
    assert_eq!(e.n.red_de_caminos().rutas().count(), 1);
    assert_eq!(e.caravana_de(car).estado, EstadoCaravana::Preparando);
    assert!(e.caravana_de(car).plan.is_none(), "una preparando está en su ciudad: sin plan");
    e.avanzar(30 * MIN);
    e.ok("ana", Comando::CancelarCaravana { caravana: car });
    assert_eq!(e.caravana_de(car).estado, EstadoCaravana::Disponible);
    assert_eq!(e.n.pesos_de_red(), pesos, "el camino se queda");
    e.avanzar(3 * HORA);
    assert!(hechos(&e.n, "comercio.caravana_sale").is_empty(), "el fin de la preparación cancelada no sale");
    // Ya en ruta no se cancela.
    e.ok("ana", Comando::PrepararCaravana { caravana: car, destino: b, carga: vec![(Cobre, 10_000)], salir_en: None });
    assert_eq!(e.caravana_de(car).estado, EstadoCaravana::Preparando, "dos carros: 2 min de preparación");
    e.avanzar(2 * MIN);
    assert_eq!(e.caravana_de(car).estado, EstadoCaravana::EnTransito);
    assert_eq!(e.rechazada("ana", Comando::CancelarCaravana { caravana: car }), rechazos::CARAVANA_NO_PREPARANDO);
    assert_eq!(hechos(&e.n, "comercio.caravana_sale").len(), 1);
}


// ───────────────────────────── 4. Ruina y conquista con caravanas en ruta ─────────────────────────────

/// Las dos plazas con muy poca madera de más (la de A caerá en ruinas) y una caravana de dos carros por plaza.
fn escenario_ruina(m: &M) -> (Escenario, u64, u64) {
    let mut e = escenario_con(m, 7, 200_000, true);
    let (a, b) = (e.a, e.b);
    let ca = e.caravana("ana", a, 2, true);
    let cb = e.caravana("beto", b, 2, true);
    (e, ca, cb)
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
fn la_ruina_del_destino_hace_dar_la_vuelta_a_la_caravana_y_la_del_origen_la_pierde() {
    let m = con_mundo!();
    let ruina = hora_de_la_ruina(m);
    let (mut e, ca, cb) = escenario_ruina(m);
    let (a, b) = (e.a, e.b);
    e.avanzar_hasta(ruina - 40 * MIN);
    for plaza in [a, b] {
        e.dar(plaza, Cobre, 300_000);
    }
    // Una de A hacia B (se perderá con su plaza), una de B hacia A (dará la vuelta) y otra de B que sale justo al caer A: dos carros → 2 min de
    // preparación; la tercera queda `preparando` hasta el mismo instante de la ruina.
    let cb2 = e.caravana("beto", b, 1, true);
    let cobre_b0 = cantidad(&e.n, b, Cobre);
    e.ok("ana", Comando::PrepararCaravana { caravana: ca, destino: b, carga: vec![(Cobre, 100_000)], salir_en: None });
    e.ok("beto", Comando::PrepararCaravana { caravana: cb, destino: a, carga: vec![(Cobre, 50_000)], salir_en: None });
    e.ok("beto", Comando::PrepararCaravana { caravana: cb2, destino: a, carga: vec![(Cobre, 20_000)], salir_en: Some(ruina) });
    e.avanzar_hasta(ruina - 1);
    assert!(e.n.economia(a).is_some(), "la ruina llega exactamente a su hora");
    assert_eq!((e.caravana_de(ca).estado, e.caravana_de(cb).estado, e.caravana_de(cb2).estado), (EstadoCaravana::EnTransito, EstadoCaravana::EnTransito, EstadoCaravana::Preparando));
    let donde_estaba = e.n.posicion_de_caravana(cb, ruina).unwrap();
    let (fin_antes, pb) = (fin_del_plan(&e, cb), pos(&e.n, b));
    assert!(donde_estaba != pb && donde_estaba != pos(&e.n, a), "a mitad de camino");

    // En el mismo instante (ruina, prioridad 5, antes que el comercio, 6): la preparando hacia A no llega a salir.
    e.avanzar_hasta(ruina);
    assert!(e.n.economia(a).is_none());
    assert!(hechos(&e.n, "comercio.caravana_sale").iter().all(|h| h.0 < ruina - 20 * MIN), "nada sale en el instante de la ruina");
    let vuelve = hechos(&e.n, "comercio.caravana_vuelve");
    assert_eq!(vuelve.iter().map(|h| (h.0, dato(h, "caravana"))).collect::<Vec<_>>(), vec![(ruina, cb as i64), (ruina, cb2 as i64)]);
    assert_eq!(e.caravana_de(cb2).estado, EstadoCaravana::Disponible);
    assert_eq!(cantidad(&e.n, b, Cobre), cobre_b0 - 50_000, "la carga de la preparando volvió al almacén; la de cb sigue en el camino");
    // La de A se pierde con su carga; la de B da la vuelta desde donde está, con su carga, por un plan nuevo hasta su casa.
    assert!(e.n.caravana(ca).is_none());
    assert_eq!(hechos(&e.n, "comercio.caravana_perdida").iter().map(|h| dato(h, "caravana")).collect::<Vec<_>>(), vec![ca as i64]);
    let vuelta = e.caravana_de(cb).clone();
    assert_eq!((vuelta.estado, vuelta.destino, vuelta.contenido[&Cobre], vuelta.origen_acuerdo), (EstadoCaravana::Retornando, Some(a as u32), 50_000, None));
    let plan = vuelta.plan.as_ref().unwrap();
    assert_eq!((plan.vertices.first().unwrap().p, plan.vertices.last().unwrap().p, plan.inicio()), (donde_estaba, pb, Some(ruina)), "parte de la posición real");
    // Vuelve por donde vino, a la misma velocidad: tarda lo que llevaba andando (salió 38 minutos antes, tras 2 de preparación).
    let andado = plan.fin().unwrap() - ruina;
    assert!((andado - 38 * MIN).abs() < 3 * MIN && ruina + (fin_antes - ruina) > plan.fin().unwrap() - 60 * MIN, "regreso de {} min", andado / MIN);
    // Llega a casa y vuelca su carga; lo cerrado no vuelve a salir.
    let fin = plan.fin().unwrap();
    e.avanzar_hasta(fin);
    assert_eq!(e.caravana_de(cb).estado, EstadoCaravana::Disponible);
    assert_eq!(cantidad(&e.n, b, Cobre), cobre_b0, "la carga vuelve al almacén de B");
    let casa = hechos(&e.n, "comercio.caravana_en_casa");
    assert_eq!((casa.len(), dato(&casa[0], "volcado")), (1, 50_000));
    e.avanzar(2 * DIA);
    assert!(hechos(&e.n, "comercio.caravana_llega").is_empty(), "ni la perdida ni la que dio la vuelta entregan nada");
    assert_eq!(hechos(&e.n, "comercio.caravana_sale").len(), 2);
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

#[test]
fn la_conquista_pasa_la_flota_hace_volver_a_las_que_iban_y_destruye_la_aparcada() {
    let m = con_mundo!();
    // A es más antigua que B (o igual y de id menor), pero es de F2 solo desde la conquista: la capital de F2 sigue siendo B (5C-1). Sin mantenimiento por distancia, porque A, a 1.500 u de
    // esa capital, se arruinaría en horas y se llevaría su flota; aquí se mide la flota, no la distancia (su escala se calibra en 5I, 5C-2).
    let mut reglas = sin_bandidos();
    std::sync::Arc::make_mut(&mut reglas.economia.datos).mantenimiento.escala_distancia = i64::MAX / 4;
    let mut e = montar(Nucleo::nuevo(Arranque { reglas, ..arranque(m, 7) }), m, true, 350_000, false);
    let (a, b) = (e.a, e.b);
    // A nace protegida un día (Doc 1.2): el guion empieza cuando ya se la puede asediar.
    e.avanzar_hasta(e.n.protegida_hasta(a).unwrap());
    let c = tercera_plaza(&mut e, m);
    e.dar(a, Cobre, 200_000);
    e.dar(b, Cobre, 200_000);
    let col_a = {
        let p = pos(&e.n, a);
        let t = e.n.ahora();
        e.n.crear_ejercito(t, "ana", 1, Punto::new(p.x + 1_000, p.y), 500, 100_000).unwrap()
    };
    // Una de A hacia B, una de B hacia A, y una de C aparcada en A con madera (la deja la columna de Ana, que entra en A).
    let ca = e.caravana("ana", a, 1, true);
    let cb = e.caravana("beto", b, 1, true);
    let cc = e.caravana("ana", c, 1, true);
    e.ok("admin", Comando::AdjuntarCaravana { caravana: cc, ejercito: col_a });
    e.ok("admin", Comando::AparcarCaravana { caravana: cc, asentamiento: a }); // la administración la deja dentro (5G-a-5: solo se aparca dentro)
    e.dar(a, Madera, 10_000);
    e.ok("ana", Comando::MoverCargaAparcada { caravana: cc, recurso: Madera, cantidad: 5_000, cargar: true });
    let tr = e.trueque(&[(Cobre, 1_000)], &[(Estano, 1_000)]);
    let cobre_b0 = cantidad(&e.n, b, Cobre);
    e.ok("ana", Comando::PrepararCaravana { caravana: ca, destino: b, carga: vec![(Cobre, 100_000)], salir_en: None });
    e.ok("beto", Comando::PrepararCaravana { caravana: cb, destino: a, carga: vec![(Cobre, 50_000)], salir_en: None });
    e.avanzar(20 * MIN);
    let ahora = e.n.ahora();
    // El asedio se resuelve al acabar su convocatoria de 30 s: ahí es donde la pilla la conquista.
    let donde = e.n.posicion_de_caravana(cb, ahora + 30_000).unwrap();
    // La conquista de verdad: el ejército de Gus (F2) asedia la plaza A, que no tiene a nadie que la defienda, y cae sin combate.
    let p = pos(&e.n, a);
    let gus = e.n.crear_ejercito(ahora, "gus", 2, Punto::new(p.x + 2_000, p.y), 500, 100_000).unwrap();
    let heroe_gus = e.n.ejercito(gus).unwrap().columna.lider;
    let arqueros = militar::catalogo::Tropa::ArquerosCompuesto;
    e.ok("admin", Comando::DarEscuadra { heroe: heroe_gus, tropa: arqueros, cantidad: militar::Reglas::cargar().tropas.tropa(arqueros).unidades });
    e.ok("gus", Comando::Atacar { heroe: heroe_gus, objetivo: nucleo::ObjetivoAtaque::Asentamiento(a), declara_guerra: true });
    tras_la_convocatoria(&mut e.n);
    assert_eq!(e.n.estado().asentamientos[&a].faccion, 2, "la plaza cambió de dueño");
    let conquista = hechos(&e.n, "asentamiento.conquistada")[0].0;
    assert!(a < b && e.n.estado().asentamientos[&a].fundado_en <= e.n.estado().asentamientos[&b].fundado_en, "por su fundación, A ganaría (la misma, y menor id)");
    assert_eq!((e.n.estado().asentamientos[&a].de_la_faccion_desde, e.n.capital_de(2)), (conquista, Some(b)), "conquistar no cambia la capital (5C-1)");

    // El trueque con la plaza se anula sin penalizar; la plaza sigue y su flota pasa con ella: la suya sigue su camino.
    assert_eq!(e.n.acuerdo(tr).unwrap().estado, EstadoTrueque::Expirado);
    assert_eq!((e.n.reputacion(1), e.n.reputacion(2)), (0, 0));
    assert_eq!(e.caravana_de(ca).estado, EstadoCaravana::EnTransito);
    // La que iba hacia la plaza regresa con su carga desde donde está; la aparcada en la plaza se destruye con su carga, sin captura (Doc 3.13.7).
    let vuelta = e.caravana_de(cb);
    assert_eq!((vuelta.estado, vuelta.destino, vuelta.contenido[&Cobre]), (EstadoCaravana::Retornando, Some(a as u32), 50_000));
    assert_eq!(vuelta.plan.as_ref().unwrap().vertices.first().unwrap().p, donde);
    assert!(e.n.caravana(cc).is_none(), "la aparcada en la plaza conquistada se destruye");
    assert_eq!(hechos(&e.n, "comercio.caravana_perdida").iter().map(|h| dato(h, "caravana")).collect::<Vec<_>>(), vec![cc as i64]);
    // La que regresó llega a su casa y vuelca lo que llevaba; la de A, que no era destino de nadie, entrega en B.
    e.avanzar(DIA);
    assert_eq!(e.caravana_de(cb).estado, EstadoCaravana::Disponible);
    assert_eq!(cantidad(&e.n, b, Cobre), cobre_b0 + 100_000, "la de B volvió con sus 50 y la de A, que seguía su camino, le trajo sus 100");
    let llegadas = hechos(&e.n, "comercio.caravana_llega");
    assert_eq!((llegadas.len(), dato(&llegadas[0], "caravana")), (1, ca as i64));
    assert_eq!(hechos(&e.n, "comercio.caravana_en_casa").len(), 2, "la que dio la vuelta y la que entregó y volvió");
}

// ───────────────────────────── 5. La red de caminos ─────────────────────────────

#[test]
fn la_ruta_va_el_doble_de_rapido_la_segunda_vez_y_la_red_se_poda_a_los_4320_minutos() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 300_000);
    e.dar(b, Cobre, 300_000);
    let ca = e.caravana("ana", a, 1, true);
    let cb = e.caravana("beto", b, 1, true);
    assert!(e.n.pesos_de_red().is_empty() && e.n.red_de_caminos().rutas().count() == 0);

    let lanzar = |e: &mut Escenario| {
        let t = e.n.ahora();
        e.ok("ana", Comando::PrepararCaravana { caravana: ca, destino: b, carga: vec![(Cobre, 10_000)], salir_en: None });
        let ida = fin_del_plan(e, ca) - t;
        e.avanzar_hasta(fin_del_plan(e, ca));
        e.avanzar_hasta(fin_del_plan(e, ca));
        assert_eq!(e.caravana_de(ca).estado, EstadoCaravana::Disponible);
        (t, ida)
    };
    let (_, primera) = lanzar(&mut e);
    let pesos = e.n.pesos_de_red();
    assert!(!pesos.is_empty() && pesos.values().all(|&p| p == 1));
    let aristas = e.n.red_de_caminos().aristas().count();
    assert_eq!(primera, duracion_esperada(m, pos(&e.n, a), pos(&e.n, b), 24_000), "el primer viaje abre el camino: no va más rápido");
    // La segunda vez va por el camino: el coste de cada arista de la red se divide por dos (los tramos de enganche no).
    let (t2, segunda) = lanzar(&mut e);
    assert!(segunda * 100 > primera * 48 && segunda * 100 < primera * 56, "{segunda} contra {primera}");
    assert_eq!(e.n.red_de_caminos().rutas().count(), 1, "la ruta del mismo par se sustituye");
    assert!(e.n.red_de_caminos().aristas().count() >= aristas, "las aristas no salen nunca");

    // Poda perezosa a los 4.320 minutos desde el último lanzamiento: las consultas ya no ven la ruta, el estado la guarda hasta el próximo uso.
    e.avanzar_hasta(t2 + 4_320 * MIN - 1);
    assert!(!e.n.pesos_de_red().is_empty());
    e.avanzar_hasta(t2 + 4_320 * MIN);
    assert!(e.n.pesos_de_red().is_empty(), "a los 4.320 min exactos caduca");
    assert_eq!(e.n.red_de_caminos().rutas().count(), 1, "sin tick: sigue guardada hasta que alguien arranque otra ruta");
    // Arrancar otra ruta la poda; las aristas del camino viejo siguen como sendero y siguen siendo más rápidas.
    let t3 = e.n.ahora();
    e.ok("beto", Comando::PrepararCaravana { caravana: cb, destino: a, carga: vec![(Cobre, 10_000)], salir_en: None });
    let red = e.n.red_de_caminos();
    assert_eq!((red.rutas().count(), red.ruta(a as u32, b as u32).is_none(), red.ruta(b as u32, a as u32).is_some()), (1, true, true));
    assert!(red.aristas().count() >= aristas);
    let vuelta = fin_del_plan(&e, cb) - t3;
    assert!(vuelta * 100 < primera * 56, "las aristas viejas siguen siendo camino aunque ninguna ruta las pese: {vuelta} contra {primera}");
}

/// Un sitio fundable cerca del campamento `i` (no hace falta bosque: estas plazas solo sirven para lanzar caravanas).
fn sitio_cerca(n: &Nucleo, m: &Mundo, i: usize, faccion: u32) -> Punto {
    let c = campamento(m, i);
    for r in (0..10).map(|k| 40_000 + k * 6_000) {
        for g in (0..360).step_by(3) {
            let a = (g as f64).to_radians();
            let p = Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64);
            if n.viabilidad(p, faccion).is_some_and(|v| v.motivo.is_none()) {
                return p;
            }
        }
    }
    panic!("sin sitio fundable cerca del campamento {i}");
}

#[test]
fn un_camino_compartido_abierto_pide_diez_rutas_de_tres_facciones_fuera_de_las_zonas() {
    let m = con_mundo!();
    let mut n = Nucleo::nuevo(arranque(m, 11));
    for f in [1, 2, 3] {
        n.aplicar_en(0, Peticion::nueva("admin", &format!("n{f}"), Comando::FijarNivelFaccion { faccion: f, nivel: 2 }).de_admin());
    }
    // Tres plazas (una por Facción) cerca de cada uno de dos campamentos lejanos: 18 rutas ordenadas entre los dos grupos comparten corredor.
    let mut plazas = Vec::new();
    for (grupo, campamento) in [0usize, 3].into_iter().enumerate() {
        for f in [1u32, 2, 3] {
            let p = sitio_cerca(&n, &m.0, campamento, f);
            plazas.push((fundar(&mut n, 0, &format!("p{grupo}{f}"), &format!("dueno{f}"), f, p), f, grupo));
        }
    }
    let mut e = Escenario { n, a: plazas[0].0, b: plazas[3].0, cuenta: 0 };
    for &(plaza, f, _) in &plazas {
        e.dar(plaza, Madera, 350_000);
        e.dar(plaza, Cobre, 50_000);
        e.ok("admin", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
        let _ = f;
    }
    e.avanzar(9 * HORA);
    assert!(!e.n.camino_compartido_abierto(), "sin rutas no hay camino compartido");
    let t = e.n.ahora();
    for &(plaza, f, _) in &plazas {
        assert!(e.n.economia(plaza).is_some(), "la plaza sigue viva");
        let actor = format!("dueno{f}");
        let c = e.caravana(&actor, plaza, 2, true);
        for &(destino, _, grupo_d) in &plazas {
            if plazas.iter().find(|p| p.0 == plaza).unwrap().2 == grupo_d {
                continue;
            }
            // Lanzar y cancelar: la ruta entra en la red al lanzar, aunque la caravana no llegue a salir.
            e.ok(&actor, Comando::PrepararCaravana { caravana: c, destino, carga: vec![(Cobre, 1_000)], salir_en: Some(t + HORA) });
            e.ok(&actor, Comando::CancelarCaravana { caravana: c });
        }
    }
    let pesos = e.n.pesos_de_red();
    let max = pesos.values().copied().max().unwrap();
    assert!(max >= 10, "el corredor entre los dos grupos lo pisan al menos 10 rutas (máximo {max})");
    assert_eq!(e.n.red_de_caminos().rutas().count(), 18);
    assert!(e.n.camino_compartido_abierto(), "10 rutas de 3 Facciones sobre una arista fuera de toda zona");
    // Pasado el plazo, las rutas caducan y el logro deja de cumplirse.
    e.avanzar_hasta(t + HORA + 4_320 * MIN);
    assert!(!e.n.camino_compartido_abierto());
}


// ───────────────────────────── 6. Mercado de mercenarios ─────────────────────────────

#[test]
fn el_mercado_de_mercenarios_vende_reposta_y_se_repone_solo_si_algo_cambio() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    // En reposo el mercado no cuesta nada: ni evento programado, ni stock guardado (el de un campamento intacto es el inicial).
    assert!(e.n.mercenarios().pulso.is_none() && e.n.mercenarios().stock.is_empty());
    assert!(e.n.estado().cola.volcar().iter().all(|ev| !matches!(ev.carga, nucleo::Carga::Comercio { suceso: nucleo::SucesoComercio::ReponeMercenarios, .. })));
    e.avanzar(MIN);
    let t = e.n.ahora();
    let puerta = campamento(&m.0, 0);
    let col = e.n.crear_ejercito(t, "ana", 1, Punto::new(puerta.x + 1_000, puerta.y), 500, 100_000).unwrap();
    let lejos = e.n.crear_ejercito(t, "lejana", 1, Punto::new(puerta.x + 10_000, puerta.y), 500, 100_000).unwrap();
    // «ana» vive en el campamento 1: en el 0 es una visitante con su columna a la puerta. La «lejana» va en una columna lejos.
    let ana = e.heroe_en_columna("ana", 1, col);
    let lejana = e.heroe_en_columna("lejana", 1, lejos);
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 300_000 });
    let comprar = |heroe, campamento, recurso, cantidad| Comando::ComprarEnCampamento { heroe, campamento, recurso, cantidad };
    // Rechazos, cada uno con su código.
    assert_eq!(e.rechazada("ana", comprar(ana, 9_999, Cobre, 10_000)), rechazos::CAMPAMENTO_DESCONOCIDO);
    assert_eq!(e.rechazada("lejana", comprar(lejana, 0, Cobre, 10_000)), rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA);
    assert_eq!(e.rechazada("beto", comprar(ana, 0, Cobre, 10_000)), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", comprar(99, 0, Cobre, 10_000)), rechazos::HEROE_DESCONOCIDO);
    assert_eq!(e.rechazada("ana", comprar(ana, 0, Trigo, 10_000)), rechazos::MERCENARIOS_TRIGO_NO_VA_AL_CARRO, "el trigo nunca va al carro");
    assert_eq!(e.rechazada("ana", comprar(ana, 0, Oro, 10_000)), rechazos::MERCENARIOS_CANTIDAD_INVALIDA);
    assert_eq!(e.rechazada("ana", comprar(ana, 0, Cobre, 900)), rechazos::MERCENARIOS_CANTIDAD_INVALIDA, "menos de una unidad");
    assert_eq!(e.rechazada("lejana", Comando::RepostarEnCampamento { heroe: lejana, campamento: 0 }), rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA);
    assert!(e.n.mercenarios().pulso.is_none(), "un rechazo no deja rastro");

    // Visitante: paga con el oro de su carro, el oro cobrado se destruye y lo comprado va al carro; sin cupo diario.
    let (oro0, cobre0) = (mundo_de(&e.n, Oro), mundo_de(&e.n, Cobre));
    e.ok("ana", comprar(ana, 0, Cobre, 10_900));
    let h = hechos(&e.n, "mercenarios.comprado");
    let cobrado = dato(&h[0], "oro");
    assert_eq!((h.len(), dato(&h[0], "cantidad"), dato(&h[0], "residente")), (1, 10_000, 0), "solo se sirven unidades enteras");
    assert!(cobrado >= 10_000 && cobrado % 1_000 == 0, "ceil(10 × precio) en oro entero: {cobrado}");
    assert_eq!(oro0 - mundo_de(&e.n, Oro), cobrado, "el oro cobrado se destruye");
    assert_eq!(mundo_de(&e.n, Cobre) - cobre0, 10_000);
    assert_eq!((e.n.suministro(col)[&Cobre], e.n.suministro(col)[&Oro]), (10_000, 300_000 - cobrado));
    assert_eq!(e.n.stock_de_campamento(0).unwrap().de(Cobre), 90_000);
    assert_eq!(e.n.stock_de_campamento(1).unwrap().de(Cobre), 100_000, "otro campamento intacto");
    assert!(e.n.mercenarios().pulso.is_none(), "el cobre no es una pila: no hay nada que reponer sin comercio");

    // Repostar: gratis, hasta el tope de la columna, sin gastar el stock.
    let stock0 = e.n.stock_de_campamento(0).unwrap();
    // Dos héroes en la columna (su Líder de escenario y «ana»): la capacidad de los víveres es la suma de la de los dos.
    let antes = e.n.estado().ejercitos[&col].columna.viveres.valor_en(e.n.ahora());
    e.ok("ana", Comando::RepostarEnCampamento { heroe: ana, campamento: 0 });
    let r = hechos(&e.n, "mercenarios.repostado");
    assert_eq!((r.len(), dato(&r[0], "cantidad")), (1, 700_000 - antes));
    assert_eq!(e.n.estado().ejercitos[&col].columna.viveres.valor_en(e.n.ahora()), 700_000);
    assert_eq!(e.rechazada("ana", Comando::RepostarEnCampamento { heroe: ana, campamento: 0 }), rechazos::MERCENARIOS_VIVERES_LLENOS);
    assert_eq!(e.n.stock_de_campamento(0).unwrap(), stock0);

    // Residente: paga de su almacén personal, compra con cupo diario (madera 60) y el cupo se renueva al día siguiente.
    let residir = |residencia| Comando::FijarResidencia { heroe: ana, residencia };
    assert_eq!(e.rechazada("ana", residir(Some(militar::heroe::Residencia::Campamento(0)))), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("admin", residir(Some(militar::heroe::Residencia::Campamento(9_999)))), rechazos::CAMPAMENTO_DESCONOCIDO);
    e.ok("admin", residir(None));
    assert_eq!(e.rechazada("admin", Comando::DarAlmacenPersonal { heroe: ana, recurso: Oro, cantidad: 1_000 }), rechazos::HEROE_SIN_RESIDENCIA, "sin residencia no hay almacén personal");
    e.ok("admin", residir(Some(militar::heroe::Residencia::Campamento(0))));
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: ana, recurso: Oro, cantidad: 500_000 });
    e.ok("ana", comprar(ana, 0, Madera, 50_000));
    assert_eq!(e.n.heroe(ana).unwrap().almacen_personal[&Madera], 50_000);
    assert!(!e.n.suministro(col).contains_key(&Madera), "lo del residente no va al carro");
    e.ok("ana", comprar(ana, 0, Madera, 20_000));
    assert_eq!(e.n.heroe(ana).unwrap().almacen_personal[&Madera], 60_000, "el cupo diario de madera son 60: solo caben 10 de los 20");
    assert_eq!(e.rechazada("ana", comprar(ana, 0, Madera, 1_000)), rechazos::MERCENARIOS_CUPO_AGOTADO);
    // La madera es una pila: bajó de 300 y toca reponerla en la próxima fase de 3 horas.
    let cada = 3 * HORA;
    let proxima = (e.n.ahora() / cada + 1) * cada;
    assert_eq!(e.n.mercenarios().pulso, Some(proxima), "una sola cita, a la fase de siempre");
    assert_eq!(e.n.stock_de_campamento(0).unwrap().de(Madera), 240_000);

    // Lo comerciado entre Facciones alimenta la reposición: un trueque entero por reparto automático.
    e.dar(a, Cobre, 100_000);
    e.dar(b, Estano, 100_000);
    e.caravana("ana", a, 1, false);
    e.caravana("beto", b, 1, false);
    e.trueque(&[(Cobre, 60_000)], &[(Estano, 30_000)]);
    e.avanzar_hasta(proxima - 1);
    let cont = e.n.mercenarios().mercado.contadores.clone();
    assert_eq!((cont[&Cobre], cont[&Estano]), (60_000, 30_000), "el trueque ya se cumplió");
    assert!(hechos(&e.n, "mercenarios.repuesto").is_empty());
    e.avanzar_hasta(proxima);
    // Las pilas vuelven a su tope; cada bien comerciado sube el 3 % de cada comercio (en todos los campamentos), en unidades enteras hacia abajo y como mínimo una (EE-5: 1,8 de cobre
    // suben 1, y 0,9 de estaño, el mínimo de 1); los contadores vuelven a cero.
    assert_eq!(e.n.stock_de_campamento(0).unwrap().de(Madera), 300_000);
    assert_eq!((e.n.stock_de_campamento(0).unwrap().de(Cobre), e.n.stock_de_campamento(1).unwrap().de(Cobre)), (91_000, 101_000));
    assert_eq!(e.n.stock_de_campamento(1).unwrap().de(Estano), 101_000);
    assert!(e.n.mercenarios().mercado.contadores.is_empty());
    assert_eq!((hechos(&e.n, "mercenarios.repuesto").len(), e.n.mercenarios().mercado.repone_en), (1, proxima + cada));
    // Con todo repuesto no hay más citas: el evento no recurre mientras nada cambie.
    assert!(e.n.mercenarios().pulso.is_none());
    e.avanzar(3 * DIA);
    assert_eq!(hechos(&e.n, "mercenarios.repuesto").len(), 1);
    // Al día siguiente el cupo vuelve a 60.
    e.ok("ana", comprar(ana, 0, Madera, 40_000));
    assert_eq!(e.n.heroe(ana).unwrap().almacen_personal[&Madera], 100_000);
}

// ───────────────────────────── 7. Adjuntas, aparcadas y columnas ─────────────────────────────

#[test]
fn una_caravana_adjunta_viaja_con_la_columna_a_su_paso_y_entrega_a_la_puerta_del_destino() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    let (pa, pb) = (pos(&e.n, a), pos(&e.n, b));
    let t = e.n.ahora();
    let col = e.n.crear_ejercito(t, "ana", 1, Punto::new(pa.x + 1_000, pa.y), 500, 400_000).unwrap();
    let sola = e.n.crear_ejercito(t, "ana", 1, Punto::new(pa.x + 1_000, pa.y + 500), 500, 400_000).unwrap();
    let col_beto = e.n.crear_ejercito(t, "beto", 2, Punto::new(pa.x, pa.y + 1_000), 500, 400_000).unwrap();
    let col_lejos = e.n.crear_ejercito(t, "ana", 1, Punto::new(pa.x + 20_000, pa.y), 500, 400_000).unwrap();
    e.dar(a, Cobre, 100_000);
    let c = e.caravana("ana", a, 1, true);
    let adjuntar = |ejercito| Comando::AdjuntarCaravana { caravana: c, ejercito };
    // Quien engancha: el dueño de la columna, a la puerta del lugar donde está la caravana (su origen) y de su Facción.
    assert_eq!(e.rechazada("beto", adjuntar(col)), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("beto", adjuntar(col_beto)), rechazos::CARAVANA_FACCION_AJENA);
    assert_eq!(e.rechazada("ana", adjuntar(col_lejos)), rechazos::CARAVANA_FUERA_DE_LA_PUERTA);
    assert_eq!(e.rechazada("ana", adjuntar(999)), rechazos::EJERCITO_DESCONOCIDO);
    e.ok("ana", adjuntar(col));
    assert_eq!(e.caravana_de(c).estado, EstadoCaravana::Adjunta);
    assert_eq!(e.n.estado().ejercitos[&col].caravanas_adjuntas, vec![c]);
    assert_eq!(e.rechazada("ana", adjuntar(sola)), rechazos::CARAVANA_ESTADO_INVALIDO, "ya va enganchada");
    // Aparcarla en la plaza, cargarla contra su almacén y volver a engancharla.
    assert_eq!(e.rechazada("ana", Comando::AparcarCaravana { caravana: c, asentamiento: a }), rechazos::CARAVANA_SOLTAR_SOLO_DENTRO, "a la puerta se está fuera (5G-a-5)");
    e.ok("admin", Comando::AparcarCaravana { caravana: c, asentamiento: a });
    assert_eq!((e.caravana_de(c).estado, e.caravana_de(c).anfitriona), (EstadoCaravana::Aparcada, Some(a as u32)));
    assert!(e.n.estado().ejercitos[&col].caravanas_adjuntas.is_empty());
    e.ok("ana", Comando::MoverCargaAparcada { caravana: c, recurso: Cobre, cantidad: 60_000, cargar: true });
    e.ok("ana", adjuntar(col));
    assert_eq!(e.n.capacidad_de_carga(col), Some(500_000 + 500_000 - 60_000), "el carro de un jugador más el hueco libre de la caravana adjunta, que lleva 60 de cobre (5G-a-4)");
    let tr = e.trueque(&[(Cobre, 60_000)], &[(Estano, 1_000)]);

    // La columna marcha a la puerta de B a la velocidad del más lento (el buey, 24; la columna sola va a 30).
    let destino = Punto::new(pb.x + 1_000, pb.y);
    let t0 = e.n.ahora();
    e.ok("ana", Comando::MarcharA { ejercito: col, destino });
    e.ok("ana", Comando::MarcharA { ejercito: sola, destino });
    let (d_con, d_sin) = (e.n.estado().ejercitos[&col].plan.fin().unwrap() - t0, e.n.estado().ejercitos[&sola].plan.fin().unwrap() - t0);
    assert!((d_con * 4 - d_sin * 5).abs() * 1_000 < d_sin * 5, "con la caravana 24 en vez de 30: {d_con} contra {d_sin}");
    e.avanzar(d_con / 2);
    let medio = e.n.estado().ejercitos[&col].posicion_en(e.n.ahora());
    assert_eq!(e.n.posicion_de_caravana(c, e.n.ahora()), Some(medio), "la adjunta va donde va la columna");
    assert_eq!(e.rechazada("ana", Comando::AparcarCaravana { caravana: c, asentamiento: a }), rechazos::CARAVANA_SOLTAR_SOLO_DENTRO, "solo dentro de una plaza");
    e.avanzar_hasta(t0 + d_con);
    // Entrega manual a la puerta del que recibe: la comisión de siempre y el trueque se abona.
    let cobre_b0 = cantidad(&e.n, b, Cobre);
    assert_eq!(e.rechazada("ana", Comando::EntregarDeCaravana { ejercito: sola, caravana: c, acuerdo: tr }), rechazos::CARAVANA_NO_ESTA_ADJUNTA);
    e.ok("ana", Comando::EntregarDeCaravana { ejercito: col, caravana: c, acuerdo: tr });
    assert_eq!(cantidad(&e.n, b, Cobre) - cobre_b0, 60_000);
    assert_eq!(e.n.acuerdo(tr).unwrap().pendiente(Lado::A), vec![]);
    assert_eq!(e.caravana_de(c).estado, EstadoCaravana::Adjunta, "sigue enganchada");
    // Soltarla (aparcarla) en la plaza de otra Facción no se puede; y la columna recupera su velocidad si estaba en marcha.
    assert_eq!(e.rechazada("ana", Comando::AparcarCaravana { caravana: c, asentamiento: b }), rechazos::CARAVANA_PLAZA_AJENA);
}


#[test]
fn una_personal_no_lleva_caravanas_el_ejercito_que_guarnece_entra_con_ellas_y_sale_con_ellas_y_el_que_se_queda_sin_nadie_las_suelta_a_volver() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let a = e.a;
    let h = e.ok("ana", Comando::CrearHeroe { cuenta: "ana".into(), campamento: 0, controlador: militar::heroe::Controlador::Humano }).creadas[0];
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
    e.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(militar::heroe::Residencia::Asentamiento(a as u32)) });
    e.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: militar::presencia::Ubicacion::Asentamiento(a as u32) });
    let milicia = e.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: militar::catalogo::Tropa::MiliciaLanceros }).creadas[0];
    e.dar(a, Cobre, 100_000);
    let c = e.caravana("ana", a, 1, true);
    let adjuntar = |ejercito| Comando::AdjuntarCaravana { caravana: c, ejercito };
    // Una columna personal va a lo suyo: no lleva caravanas.
    let personal = e.ok("ana", Comando::SalirAlMundo { asentamiento: a, heroe: h, escuadras: vec![], carga: vec![] }).creadas[0];
    assert_eq!(e.rechazada("ana", adjuntar(personal)), rechazos::COLUMNA_PERSONAL_SIN_CARAVANAS);
    e.ok("ana", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: h });
    // Un ejército sí. Al guarnecer entra con todo (Entrar con todo, 2026-10-22): la caravana ya no se queda aparcada en la plaza; sigue enganchada y la columna del
    // Líder, un ejército mientras la lleve, se queda dentro (sin tropa: la milicia vuelve a su campamento).
    let movilizar = Comando::MovilizarEjercito { asentamiento: a, heroe: h, escuadras: vec![milicia], carga: vec![], politica: militar::columna::Politica::Aceptar };
    let ej = e.ok("ana", movilizar.clone()).creadas[0];
    e.ok("ana", adjuntar(ej));
    assert_eq!(e.n.estado().ejercitos[&ej].caravanas_adjuntas, vec![c]);
    e.ok("ana", Comando::Guarnecer { asentamiento: a, heroe: h });
    assert!(e.n.ejercito(ej).is_some() && !e.n.en_el_mapa(ej), "la columna sigue, dentro, para llevar su caravana");
    assert_eq!((e.caravana_de(c).estado, e.n.estado().ejercitos[&ej].caravanas_adjuntas.clone()), (EstadoCaravana::Adjunta, vec![c]));
    assert_eq!(e.n.ejercito(ej).unwrap().columna.tipo, militar::columna::TipoColumna::Ejercito);
    assert_eq!(dato(&hechos(&e.n, "ejercito.guarnece")[0], "caravanas"), 1);
    // Sale con todo: la misma columna, con su tropa (sacada de su campamento) y su caravana.
    let r = e.ok("ana", movilizar);
    assert!(r.creadas.is_empty(), "sale con la columna con la que entró");
    assert_eq!((e.n.estado().ejercitos[&ej].caravanas_adjuntas.clone(), e.n.ejercito(ej).unwrap().columna.escuadras.clone()), (vec![c], vec![milicia]));
    assert!(e.n.en_el_mapa(ej));
    // Aparcar la caravana en su plaza sigue siendo una orden explícita (Doc 3.13.7), solo dentro (5G-a-5), y se puede volver a enganchar.
    assert_eq!(e.rechazada("ana", Comando::AparcarCaravana { caravana: c, asentamiento: a }), rechazos::CARAVANA_SOLTAR_SOLO_DENTRO, "fuera, a la puerta");
    e.ok("admin", Comando::AparcarCaravana { caravana: c, asentamiento: a });
    assert_eq!((e.caravana_de(c).estado, e.caravana_de(c).anfitriona), (EstadoCaravana::Aparcada, Some(a as u32)));
    e.ok("ana", adjuntar(ej));
    // Un ejército cuyo último héroe sale del mundo se disuelve y sus caravanas vuelven a su origen por su cuenta.
    e.ok("ana", Comando::Desconectarse { heroe: h });
    e.avanzar(150_000);
    assert!(e.n.ejercito(ej).is_none());
    assert_eq!(e.caravana_de(c).estado, EstadoCaravana::Disponible, "volvió sola: la puerta de su plaza está a un paso");
    assert_eq!(hechos(&e.n, "ejercito.disuelto").len(), 1);
    assert_eq!(e.rechazada("ana", adjuntar(ej)), rechazos::EJERCITO_DESCONOCIDO);
}
#[test]
fn una_arista_que_acaba_de_ser_camino_acelera_a_la_caravana_que_ya_iba_por_ella() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let a = e.a;
    let c = tercera_plaza(&mut e, m);
    let col_a = {
        let p = pos(&e.n, a);
        let t = e.n.ahora();
        e.n.crear_ejercito(t, "ana", 1, Punto::new(p.x + 1_000, p.y), 500, 100_000).unwrap()
    };
    e.dar(a, Madera, 10_000);
    e.dar(a, Cobre, 10_000);
    // Una de C, aparcada en A con madera, vuelve a su origen: su camino no es de nadie (el regreso no registra ruta).
    let cc = e.caravana("ana", c, 1, true);
    e.ok("admin", Comando::AdjuntarCaravana { caravana: cc, ejercito: col_a });
    e.ok("admin", Comando::AparcarCaravana { caravana: cc, asentamiento: a }); // la administración la deja dentro (5G-a-5: solo se aparca dentro)
    e.ok("ana", Comando::MoverCargaAparcada { caravana: cc, recurso: Madera, cantidad: 5_000, cargar: true });
    let madera_c0 = cantidad(&e.n, c, Madera);
    e.ok("ana", Comando::EnviarCaravanaAlOrigen { caravana: cc });
    assert!(e.n.pesos_de_red().is_empty());
    let (fin0, gen0) = (fin_del_plan(&e, cc), e.caravana_de(cc).plan.as_ref().unwrap().generacion);
    // Diez minutos después otra caravana sale de A hacia C por el mismo camino: sus aristas pasan a ser camino y la que vuelve rehace su plan.
    e.avanzar(10 * MIN);
    let ahora = e.n.ahora();
    let ca = e.caravana("ana", a, 1, true);
    e.ok("ana", Comando::PrepararCaravana { caravana: ca, destino: c, carga: vec![(Cobre, 1_000)], salir_en: None });
    let car = e.caravana_de(cc);
    let (fin1, gen1) = (car.plan.as_ref().unwrap().fin().unwrap(), car.plan.as_ref().unwrap().generacion);
    assert!(gen1 > gen0, "plan nuevo, generación nueva");
    let (viejo, nuevo) = (fin0 - ahora, fin1 - ahora);
    assert!(nuevo * 100 > viejo * 48 && nuevo * 100 < viejo * 58, "lo que le quedaba se reparte sobre el camino: {nuevo} contra {viejo}");
    assert_eq!(car.plan.as_ref().unwrap().vertices.iter().find(|v| v.en == ahora).map(|v| v.p), e.n.posicion_de_caravana(cc, ahora), "parte de donde está");
    // Llega en el instante nuevo, una sola vez: los eventos del plan viejo se descartan.
    let descartados = e.n.eventos_descartados();
    e.avanzar_hasta(fin1 - 1);
    assert_eq!(e.caravana_de(cc).estado, EstadoCaravana::Retornando);
    e.avanzar_hasta(fin1);
    assert_eq!(e.caravana_de(cc).estado, EstadoCaravana::Disponible);
    e.avanzar_hasta(fin0 + MIN);
    let casa = hechos(&e.n, "comercio.caravana_en_casa");
    assert_eq!(casa.iter().filter(|h| dato(h, "caravana") == cc as i64).count(), 1);
    // Vuelca en su casa lo que llevaba (lo que no cabe en el almacén de C, que produce madera, se pierde).
    let h = casa.iter().find(|h| dato(h, "caravana") == cc as i64).unwrap();
    assert_eq!(dato(h, "volcado") + dato(h, "perdido"), 5_000);
    assert!(cantidad(&e.n, c, Madera) >= madera_c0);
    assert!(e.n.eventos_descartados() > descartados, "el evento de llegada del plan viejo salió y se descartó");
}

// ───────────────────────────── 8. Batalla, encuentros y visión ─────────────────────────────

#[test]
fn dos_caravanas_que_se_cruzan_dejan_su_encuentro_y_una_en_batalla_no_cambia_mientras_dura() {
    let m = con_mundo!();
    let n = Nucleo::nuevo(arranque(m, 7));
    let mut e = montar(n, m, true, 350_000, false);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    e.dar(b, Cobre, 100_000);
    let ca = e.caravana("ana", a, 1, true);
    let cb = e.caravana("beto", b, 1, true);
    let t0 = e.n.ahora();
    e.ok("ana", Comando::PrepararCaravana { caravana: ca, destino: b, carga: vec![(Cobre, 10_000)], salir_en: None });
    e.ok("beto", Comando::PrepararCaravana { caravana: cb, destino: a, carga: vec![(Cobre, 10_000)], salir_en: None });
    // Van una contra otra por el mismo camino: se encuentran en el instante exacto en que se acercan a 3 unidades (un evento, no un sondeo).
    let (fin_a, fin_b) = (fin_del_plan(&e, ca), fin_del_plan(&e, cb));
    let (plan_a, plan_b) = (e.caravana_de(ca).plan.clone().unwrap(), e.caravana_de(cb).plan.clone().unwrap());
    e.avanzar_hasta(fin_b.min(fin_a));
    // Una columna hostil junto a la plaza de destino también deja su encuentro con la caravana que llega (`caravana` sola): aquí cuentan las dos caravanas.
    let enc: Vec<_> = hechos(&e.n, "caravana.encuentro").into_iter().filter(|h| h.2.iter().any(|(k, _)| *k == "caravanaB")).collect();
    assert_eq!(enc.len(), 1, "un solo encuentro");
    let (t_enc, ids) = (enc[0].0, [dato(&enc[0], "caravana"), dato(&enc[0], "caravanaB")]);
    assert!(t_enc > t0 && t_enc < fin_a.min(fin_b));
    assert!(ids == [ca as i64, cb as i64] || ids == [cb as i64, ca as i64], "{ids:?}");
    let separacion = |t| plan_a.posicion_en(t).unwrap().dist(plan_b.posicion_en(t).unwrap());
    assert!(separacion(t_enc) <= 3_000 + 50, "en el instante del encuentro están a ≤ 3 unidades ({} milésimas)", separacion(t_enc));
    assert!(separacion(t_enc - 1_000) > 2_900, "un segundo antes aún no");
    // La batalla la decide el sistema militar: la caravana queda apartada, sin plan que cambie ni llegada, y vuelve donde estaba.
    let ahora = e.n.ahora();
    let fin_ca = fin_del_plan(&e, ca);
    assert!(e.n.caravana_entra_en_batalla(ahora, ca));
    assert!(!e.n.caravana_entra_en_batalla(ahora, ca), "ya está en batalla");
    assert!(!e.n.caravana_entra_en_batalla(ahora, 999));
    let quieta = e.n.posicion_de_caravana(ca, ahora).unwrap();
    assert_eq!(e.caravana_de(ca).estado, EstadoCaravana::EnBatalla);
    e.avanzar(10 * MIN);
    assert_eq!(e.n.posicion_de_caravana(ca, e.n.ahora()), Some(quieta));
    assert!(e.n.caravana_sale_de_batalla(ahora + 10 * MIN, ca));
    assert!(!e.n.caravana_sale_de_batalla(ahora + 10 * MIN, ca));
    assert_eq!(e.caravana_de(ca).estado, EstadoCaravana::EnTransito);
    assert_eq!(fin_del_plan(&e, ca), fin_ca + 10 * MIN, "la pausa desplaza lo que le quedaba, ni un metro más ni menos");
    assert_eq!(e.n.posicion_de_caravana(ca, ahora + 10 * MIN), Some(quieta));
    e.avanzar_hasta(fin_ca + 10 * MIN - 1);
    assert!(hechos(&e.n, "comercio.caravana_llega").iter().all(|h| dato(h, "caravana") != ca as i64));
    e.avanzar_hasta(fin_ca + 10 * MIN);
    let llegada = hechos(&e.n, "comercio.caravana_llega").into_iter().find(|h| dato(h, "caravana") == ca as i64).expect("llegó");
    assert_eq!(llegada.0, fin_ca + 10 * MIN);
}

#[test]
fn los_carros_perdidos_en_batalla_se_llevan_su_parte_de_la_carga_y_sin_carros_no_queda_caravana() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 200_000);
    let car = e.caravana("ana", a, 2, true);
    e.ok("ana", Comando::PrepararCaravana { caravana: car, destino: b, carga: vec![(Cobre, 100_000)], salir_en: None });
    e.avanzar(5 * MIN);
    let t = e.n.ahora();
    assert!(e.n.caravana_entra_en_batalla(t, car));
    assert!(e.n.aplicar_perdida_de_carros(t, car, &[5]).is_none(), "un índice que no existe no cambia nada");
    let p = e.n.aplicar_perdida_de_carros(t, car, &[1]).expect("existe");
    assert_eq!((p.carros_perdidos, p.carga_perdida[&Cobre], p.destruida), (1, 50_000, false), "la mitad de la carga: viajaba en el carro perdido");
    assert_eq!((e.caravana_de(car).carros.len(), e.caravana_de(car).contenido[&Cobre]), (1, 50_000));
    assert!(e.n.caravana_sale_de_batalla(t + MIN, car));
    let cobre_b0 = cantidad(&e.n, b, Cobre);
    e.avanzar_hasta(fin_del_plan(&e, car));
    assert_eq!(cantidad(&e.n, b, Cobre) - cobre_b0, 50_000, "llega lo que quedó");
    // Sin carros se destruye con su carga.
    let c2 = e.caravana("ana", a, 1, true);
    e.ok("ana", Comando::PrepararCaravana { caravana: c2, destino: b, carga: vec![(Cobre, 10_000)], salir_en: None });
    let t = e.n.ahora();
    assert!(e.n.caravana_entra_en_batalla(t, c2));
    let p = e.n.aplicar_perdida_de_carros(t, c2, &[0]).unwrap();
    assert!(p.destruida);
    assert!(e.n.caravana(c2).is_none());
    assert_eq!(hechos(&e.n, "comercio.caravana_perdida").iter().map(|h| dato(h, "caravana")).collect::<Vec<_>>(), vec![c2 as i64]);
    e.avanzar(DIA);
    assert_eq!(hechos(&e.n, "comercio.caravana_llega").len(), 1, "la destruida no llega");
}

#[test]
fn si_en_la_batalla_se_pierde_el_animal_mas_lento_el_plan_en_pausa_sigue_a_la_velocidad_nueva() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 200_000);
    e.dar(a, Oro, 300_000);
    // Un carro con buey (24) y otro con caballo (36): la caravana va a 24.
    let car = e.caravana("ana", a, 1, true);
    e.ok("ana", Comando::AnadirCarro { caravana: car, tipo: TipoCarro::Basico });
    e.ok("ana", Comando::AnadirAnimal { caravana: car, carro: 1, animal: Animal::Caballo });
    e.ok("ana", Comando::PrepararCaravana { caravana: car, destino: b, carga: vec![(Cobre, 10_000)], salir_en: None });
    e.avanzar(5 * MIN);
    let t = e.n.ahora();
    let queda = fin_del_plan(&e, car) - t;
    assert!(e.n.caravana_entra_en_batalla(t, car));
    // Pierde el carro del buey: ahora va a 36, y lo que le quedaba dura 24 / 36.
    assert!(!e.n.aplicar_perdida_de_carros(t, car, &[0]).unwrap().destruida);
    assert_eq!(e.caravana_de(car).velocidad(&Reglas::default().economia.comercio.flota), 36_000);
    assert!(e.n.caravana_sale_de_batalla(t + MIN, car));
    let nuevo = fin_del_plan(&e, car) - (t + MIN);
    assert!((nuevo - queda * 24 / 36).abs() <= 1, "le quedaban {queda} ms a 24; a 36 son {nuevo}");
    e.avanzar_hasta(fin_del_plan(&e, car));
    assert!(hechos(&e.n, "comercio.caravana_llega").iter().any(|h| dato(h, "caravana") == car as i64), "llega a su hora nueva");
}

/// Un generador de visión que cuenta lo que ve (el de `espacial` guarda sus contadores dentro de la caja).
struct Espia(nucleo::espacial::GeneradorVision, Arc<std::sync::Mutex<(u64, u64)>>);

impl nucleo::GeneradorDeEventos for Espia {
    fn muestrear(&mut self, cx: &mut nucleo::Cx<'_>) {
        self.0.muestrear(cx);
        *self.1.lock().unwrap() = (self.0.entradas, self.0.salidas);
    }
}

#[test]
fn una_caravana_grande_llama_la_atencion_a_60_unidades_y_una_pequena_no() {
    let m = con_mundo!();
    let correr = |carros: u32| -> u64 {
        let ajustes = Ajustes { muestreo_ms: 250, ..Ajustes::default() };
        let mut e = montar(Nucleo::nuevo(Arranque { ajustes, ..arranque(m, 7) }), m, true, 350_000, false);
        let (a, b) = (e.a, e.b);
        let (pa, pb) = (pos(&e.n, a), pos(&e.n, b));
        e.dar(a, Cobre, 100_000);
        let cuenta = Arc::new(std::sync::Mutex::new((0, 0)));
        e.n.agregar_generador(Box::new(Espia(nucleo::espacial::GeneradorVision::nuevo(), cuenta.clone())));
        // Un observador de otra Facción a 45 unidades del camino: fuera de su radio (30) pero dentro de los 60 de una caravana grande.
        let camino = m.1.ruta(pa, pb, &Opciones::default()).unwrap();
        let v = camino.puntos[camino.puntos.len() / 2];
        let t = e.n.ahora();
        e.n.crear_ejercito(t, "beto", 2, Punto::new(v.x + 45_000, v.y), 500, 100_000).unwrap();
        // Las dos plazas no cuentan como observadoras: solo la columna del vigía.
        let car = e.caravana("ana", a, carros, true);
        e.ok("ana", Comando::PrepararCaravana { caravana: car, destino: b, carga: vec![(Cobre, 10_000)], salir_en: None });
        e.avanzar(2 * carros as i64 * MIN); // la preparación de 2 min por carro de más
        let fin = fin_del_plan(&e, car);
        e.avanzar_hasta(fin);
        let (entradas, _) = *cuenta.lock().unwrap();
        entradas
    };
    let pequena = correr(1);
    let grande = correr(3);
    assert!(grande > pequena, "la grande entra además en la vista del vigía ({grande} contra {pequena})");
}

// ───────────────────────────── 9. Determinismo y recuperación ─────────────────────────────

/// Un guion de viaje: reparto automático, lanzamiento manual cancelado y relanzado, compra en un campamento, red de caminos y llegadas.
fn guion(e: &mut Escenario, m: &M, avanzar: &dyn Fn(&mut Escenario, Instante)) {
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    e.dar(b, Estano, 100_000);
    let base = e.n.ahora();
    let col = {
        let p = campamento(&m.0, 0);
        e.n.crear_ejercito(base, "ana", 1, Punto::new(p.x + 1_000, p.y), 500, 100_000).unwrap()
    };
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 50_000 });
    let ana = e.heroe_en_columna("ana", 1, col);
    let c1 = e.caravana("ana", a, 1, false);
    let _c2 = e.caravana("beto", b, 1, false);
    // El cooldown de creación es de 10 minutos por plaza.
    e.avanzar(11 * MIN);
    let c3 = e.caravana("ana", a, 2, true);
    e.trueque(&[(Cobre, 60_000)], &[(Estano, 30_000)]);
    avanzar(e, base + 20 * MIN);
    e.ok_en(base + 20 * MIN, "ana", Comando::ComprarEnCampamento { heroe: ana, campamento: 0, recurso: Cobre, cantidad: 5_000 });
    e.ok_en(base + 20 * MIN, "ana", Comando::PrepararCaravana { caravana: c3, destino: b, carga: vec![(Cobre, 20_000)], salir_en: Some(base + HORA) });
    avanzar(e, base + 30 * MIN);
    e.ok_en(base + 30 * MIN, "ana", Comando::CancelarCaravana { caravana: c3 });
    e.ok_en(base + 30 * MIN, "ana", Comando::PrepararCaravana { caravana: c3, destino: b, carga: vec![(Cobre, 15_000)], salir_en: None });
    avanzar(e, base + 90 * MIN);
    e.ok_en(base + 90 * MIN, "ana", Comando::RepostarEnCampamento { heroe: ana, campamento: 0 });
    avanzar(e, base + 5 * HORA);
    avanzar(e, base + 7 * HORA);
    e.n.avanzar_hasta(base + 7 * HORA);
    assert_eq!(e.n.acuerdo(1).unwrap().estado, EstadoTrueque::Cumplido);
    let _ = c1;
}

fn correr(m: &M, semilla: u64, avanzar: &dyn Fn(&mut Escenario, Instante)) -> Nucleo {
    let mut e = escenario(m, semilla);
    guion(&mut e, m, avanzar);
    e.n
}

#[test]
fn mismas_ordenes_dan_el_mismo_hash_normal_acelerado_y_a_trozos() {
    let m = con_mundo!();
    let normal = correr(m, 3, &|e, t| e.avanzar_hasta(t));
    assert_eq!(normal.hash_estado(), correr(m, 3, &|e, t| e.avanzar_hasta(t)).hash_estado(), "repetible");
    assert_eq!(normal.hash_estado(), correr(m, 3, &|_, _| {}).hash_estado(), "acelerado de un golpe");
    let a_trozos = correr(m, 3, &|e, t| {
        let ahora = e.n.ahora();
        for s in 1..=5 {
            e.avanzar_hasta(ahora + (t - ahora).max(0) * s * s / 25);
        }
    });
    assert_eq!(normal.hash_estado(), a_trozos.hash_estado(), "en pasos pequeños e irregulares, como el modo en vivo");
    assert_ne!(normal.hash_estado(), correr(m, 4, &|e, t| e.avanzar_hasta(t)).hash_estado(), "otra semilla, otro mundo");
    // El guion hizo de verdad lo que se espera.
    assert!(normal.estado().comercio.red.rutas().count() >= 2);
    assert!(hechos(&normal, "comercio.caravana_llega").len() >= 3 && hechos(&normal, "mercenarios.comprado").len() == 1);
}

#[test]
fn recuperar_desde_foto_con_una_caravana_a_medio_camino_da_el_mismo_mundo() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9) };
    // Dos caravanas por reparto (una con peaje por una plaza ajena en el camino) y un lanzamiento manual aún preparando.
    let primera_mitad = |e: &mut Escenario| {
        let (a, b) = (e.a, e.b);
        e.dar(a, Cobre, 100_000);
        e.dar(b, Estano, 100_000);
        e.caravana("ana", a, 1, false);
        e.caravana("beto", b, 1, false);
        e.trueque(&[(Cobre, 60_000)], &[(Estano, 30_000)]);
        e.avanzar(11 * MIN);
        let manual = e.caravana("beto", b, 2, true);
        let salir = e.n.ahora() + 6 * HORA;
        e.ok("beto", Comando::PrepararCaravana { caravana: manual, destino: a, carga: vec![(Estano, 5_000)], salir_en: Some(salir) });
    };
    let segunda_mitad = |e: &mut Escenario| e.avanzar(10 * HORA);

    let sin_parar = {
        let mut e = escenario(m, 9);
        primera_mitad(&mut e);
        e.avanzar(30 * MIN);
        segunda_mitad(&mut e);
        (e.n.hash_estado(), e.n.estado().comercio.clone())
    };

    let (a_mitad, cuenta) = {
        let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        let mut e = montar(n, m, true, 350_000, false);
        primera_mitad(&mut e);
        e.avanzar(30 * MIN);
        // A los 41 minutos de viaje las dos van por el camino y la manual espera: la foto las pilla en mitad.
        let en_ruta = e.n.estado().comercio.flota.iter().filter(|c| c.estado == EstadoCaravana::EnTransito).count();
        assert_eq!(en_ruta, 2);
        assert!(e.n.estado().comercio.flota.iter().any(|c| c.estado == EstadoCaravana::Preparando));
        e.n.foto_ahora();
        e.avanzar(10 * MIN);
        e.n.apagar().unwrap();
        (e.n.hash_estado(), e.cuenta)
    };
    let n = Nucleo::con_persistencia(con(ajustes), dir.path(), Default::default()).unwrap();
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), a_mitad, "foto + registro reconstruyen el mismo mundo, con las caravanas en el camino");
    let ids: Vec<u64> = n.estado().asentamientos.keys().copied().collect();
    let mut e = Escenario { n, a: ids[0], b: ids[1], cuenta };
    e.avanzar(10 * HORA - 10 * MIN);
    assert_eq!(e.n.estado().comercio, sin_parar.1, "el comercio recuperado sigue igual que el que no se interrumpió");
    assert_eq!(e.n.hash_estado(), sin_parar.0);
    assert_eq!(e.n.acuerdo(1).unwrap().estado, EstadoTrueque::Cumplido);
    e.n.apagar().unwrap();
}

// ───────────────────────────── 10. Rendimiento ─────────────────────────────

/// Funda `plazas` plazas de una Facción cada una en una rejilla de 31 u, con Mercado, y les da dos caravanas de un buey. Devuelve
/// `(plazas fundadas, caravanas)` y deja el reloj tras construir los Mercados.
fn poblar_con_caravanas(n: &mut Nucleo, m: &Mundo, plazas: usize) -> Vec<(u64, u64, u64)> {
    let (ancho, alto) = m.tamano();
    let paso = 31_000;
    let mut ids = Vec::new();
    'fuera: for y in (paso..alto).step_by(paso as usize) {
        for x in (paso..ancho).step_by(paso as usize) {
            let faccion = ids.len() as u32 + 1;
            let e = n.crear_ejercito(0, "ana", faccion, Punto::new(x, y), 500, 100_000).unwrap();
            n.aplicar_en(0, Peticion::nueva("admin", &format!("c{x}-{y}"), Comando::DarCaravanaFundacion { ejercito: e }).de_admin());
            let r = n.aplicar_en(0, Peticion::nueva("ana", &format!("f{x}-{y}"), Comando::Fundar { ejercito: e }));
            if r.aceptada() {
                ids.push((r.creadas[0], 0, 0));
                if ids.len() == plazas {
                    break 'fuera;
                }
            }
        }
    }
    let mut cuenta = 0;
    let mut p = |n: &mut Nucleo, t: Instante, c: Comando| {
        cuenta += 1;
        let r = n.aplicar_en(t, Peticion::nueva("admin", &format!("x{cuenta}"), c).de_admin());
        r
    };
    for &(plaza, ..) in &ids {
        p(n, 0, Comando::DarRecursos { asentamiento: plaza, recurso: Madera, cantidad: 350_000 });
        p(n, 0, Comando::DarRecursos { asentamiento: plaza, recurso: Cobre, cantidad: 100_000 });
        p(n, 0, Comando::DarRecursos { asentamiento: plaza, recurso: Oro, cantidad: 100_000 });
        p(n, 0, Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
    }
    n.avanzar_hasta(9 * HORA);
    let mut caravanas = Vec::new();
    for k in 0..2 {
        for (i, &(plaza, ..)) in ids.iter().enumerate() {
            let t = 9 * HORA + k * 11 * MIN; // el cooldown de creación es de 10 min por plaza
            let c = p(n, t, Comando::CrearCaravana { asentamiento: plaza });
            if !c.aceptada() {
                continue;
            }
            let id = c.creadas[0];
            // Una plaza sin madera u oro para el carro y el buey deja su caravana vacía: no cuenta.
            let carro = p(n, t, Comando::AnadirCarro { caravana: id, tipo: TipoCarro::Basico }).aceptada();
            if carro && p(n, t, Comando::AnadirAnimal { caravana: id, carro: 0, animal: Animal::Buey }).aceptada() {
                caravanas.push((plaza, id, i as u64));
            }
        }
    }
    caravanas
}

/// Mide el coste de tener muchas caravanas viajando a la vez. `cargo test --release -p nucleo --test caravanas rendimiento -- --ignored --nocapture`.
fn medir_caravanas(plazas: usize) {
    let m = con_mundo!();
    let lanzar = |con_caravanas: bool| {
        let mut n = Nucleo::nuevo(arranque(m, 5));
        let caravanas = poblar_con_caravanas(&mut n, &m.0, plazas);
        let ids: Vec<u64> = n.estado().asentamientos.keys().copied().collect();
        let inicio = 9 * HORA + 30 * MIN;
        n.avanzar_hasta(inicio);
        if !con_caravanas {
            println!("{} plazas fundadas, {} siguen vivas a las 9 h, {} caravanas montadas", ids.len(), n.estado().asentamientos.len(), caravanas.len());
        }
        let mut lanzadas = 0;
        let (e0, t0) = (n.metricas().eventos, std::time::Instant::now());
        if con_caravanas {
            for (k, &(plaza, id, i)) in caravanas.iter().enumerate() {
                let destino = ids[(i as usize + 1 + k % 3) % ids.len()];
                if destino == plaza || !n.estado().asentamientos.contains_key(&plaza) {
                    continue;
                }
                let r = n.aplicar_en(inicio, Peticion::nueva("ana", &format!("l{k}"), Comando::PrepararCaravana { caravana: id, destino, carga: vec![(Cobre, 1_000)], salir_en: None }));
                lanzadas += r.aceptada() as u32;
            }
        }
        let lanzamiento = t0.elapsed();
        n.avanzar_hasta(inicio + 12 * HORA);
        (n.metricas().eventos - e0, t0.elapsed(), lanzamiento, lanzadas, n.estado().comercio.flota.iter().filter(|c| c.estado == EstadoCaravana::Disponible).count())
    };
    let (ev_base, t_base, ..) = lanzar(false);
    let (ev_car, t_car, t_lanzar, lanzadas, en_casa) = lanzar(true);
    let eventos = ev_car.saturating_sub(ev_base).max(1);
    let extra = t_car.saturating_sub(t_base).saturating_sub(t_lanzar);
    println!(
        "{plazas} plazas, {lanzadas} caravanas lanzadas en {:.2} s ({:.0} µs por lanzamiento con su A*); {eventos} eventos de caravana en 12 h de mundo (llegadas, peajes, regresos y los pasos de minuto de más de las plazas que siguen vivas gracias a ellas): {:.2} s de más = {:.1} µs por evento; {en_casa} disponibles al final",
        t_lanzar.as_secs_f64(),
        t_lanzar.as_secs_f64() * 1e6 / lanzadas.max(1) as f64,
        extra.as_secs_f64(),
        extra.as_secs_f64() * 1e6 / eventos as f64
    );
    assert!(lanzadas as usize >= plazas, "salieron {lanzadas}");
}

#[test]
fn muchas_caravanas_viajando_a_la_vez_pequeno() {
    medir_caravanas(12);
}

#[test]
#[ignore = "mide 100 plazas con 200 caravanas viajando a la vez; en release"]
fn rendimiento_de_200_caravanas() {
    medir_caravanas(100);
}

/// Un asedio con números se resuelve al acabar su convocatoria de 30 s (calibración del sistema 4, M8): deja pasar las que haya abiertas.
fn tras_la_convocatoria(n: &mut Nucleo) {
    let fin = n.batallas_activas().filter_map(|b| b.convocatoria_hasta).max();
    if let Some(h) = fin {
        n.avanzar_hasta(h);
    }
}
