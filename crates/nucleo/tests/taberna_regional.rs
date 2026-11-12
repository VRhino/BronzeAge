//! La taberna regional (tanda 69; Niebla_De_Guerra, Comercio_Caravanas §5.8 y Mercenarios_Por_Contrato): comprar el informe de las plazas o de las órdenes de la región del relieve, tomar a distancia una orden de mercado de
//! esas plazas con un envío de caravana, y ver y contratar a los mercenarios de la región desde la taberna. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`), que es el que
//! tiene regiones; las pruebas se omiten si no está. Las cifras salen de las reglas (`INTEL.region`, la tarifa del tablón) o de lo que la propia prueba coloca.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::Recurso::{self, *};
use asentamiento::Cargo;
use base::{Instante, Punto, MS_POR_MINUTO};
use comercio::caravana::{Animal, EstadoCaravana, TipoCarro};
use comun::mercenarios as mc;
use comun::real::{fundar, sitio_rico, M};
use contrato::v1::TipoEntidad;
use mercenarios::Objetivo;
use militar::heroe::Controlador;
use militar::presencia::Ubicacion;
use mundo::Mundo;
use nucleo::memoria::{Sitio, TipoInformeRegion};
use nucleo::proyeccion::vista::{Mirada, Perspectiva};
use nucleo::proyeccion::{ids, interior, propios};
use nucleo::{rechazos, Comando, Estado, Nucleo, PagarCon, Relacion};
use std::collections::{BTreeMap, BTreeSet};
use vision::Rect;

const MIN: Instante = MS_POR_MINUTO;
const HORA: Instante = 60 * MIN;
/// Un oro por unidad, el precio de las órdenes que colocan las pruebas (en ppm de oro por milésima de unidad; las cantidades van en milésimas: 50_000 son 50 unidades).
const UN_ORO: i64 = 1_000_000;

macro_rules! con_mundo {
    () => {
        match comun::real::mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

/// Un sitio verde donde `faccion` puede fundar ahora, en la `region` del relieve, alrededor del campamento `i` y a 90 u o más de cada punto de `lejos`: la taberna vende lo que cae lejos de su vista.
fn sitio_en(n: &Nucleo, m: &Mundo, i: usize, faccion: u32, region: u8, lejos: &[Punto]) -> Punto {
    let c = comun::real::campamento(m, i);
    for r in (0..16).map(|k| 40_000 + k * 10_000) {
        for g in (0..360).step_by(10) {
            let a = f64::from(g).to_radians();
            let p = Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64);
            let libre = n.viabilidad(p, faccion).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde);
            if m.region_en(p.x, p.y) == Some(region) && libre && lejos.iter().all(|q| q.dist(p) >= 90_000) {
                return p;
            }
        }
    }
    panic!("sin sitio en la región {region} cerca del campamento {i}");
}

/// La Facción 1 (Rey: eva) con su plaza `p1`; en su región, `p2` (F2) y `p3` (F3, en guerra con F1), y en otra, `p4` (F2). Todas con Mercado y `p1` con Taberna si se pide.
/// Dentro de `p1` están eva (Rey), `gob` (su Gobernador) y `civ` (un ciudadano sin cargo).
struct E {
    n: Nucleo,
    eva: u64,
    gob: u64,
    civ: u64,
    p1: u64,
    p2: u64,
    p3: u64,
    p4: u64,
    region: u8,
}

fn escenario(m: &M, taberna: bool) -> E {
    escenario_con(m, taberna, true)
}

/// Como [`escenario`], con o sin Mercado en `p1`.
fn escenario_con(m: &M, taberna: bool, mercado_en_p1: bool) -> E {
    let mut n = mc::nucleo(m);
    // Algunas órdenes de montaje ya valen lo que piden (la plaza ya está en el nivel, la construcción ya está pausada): «sin cambios» también vale.
    let admin = |n: &mut Nucleo, c: Comando| {
        let r = mc::dar(n, "admin", c);
        assert!(r.aceptada() || r.resultado == nucleo::Resultado::SinCambios, "{r:?}");
    };
    // El primer ciudadano de la Facción 1 es su Rey.
    let eva = mc::heroe_en(&mut n, "eva", 0, Controlador::Humano);
    admin(&mut n, Comando::FijarFaccionDeHeroe { heroe: eva, faccion: Some(1) });
    let s1 = sitio_rico(&n, &m.0, 0, 1);
    let region = m.0.region_en(s1.x, s1.y).expect("la plaza cae en una región");
    let p1 = fundar(&mut n, "fundador", 1, s1);
    let s2 = sitio_en(&n, &m.0, 0, 2, region, &[s1]);
    let p2 = fundar(&mut n, "gus", 2, s2);
    let s3 = sitio_en(&n, &m.0, 0, 3, region, &[s1, s2]);
    let p3 = fundar(&mut n, "carlos", 3, s3);
    // Con el nivel 2 de la Facción 2 caben dos plazas (y se desbloquea el Mercado).
    admin(&mut n, Comando::FijarNivelFaccion { faccion: 2, nivel: 2 });
    let s4 = sitio_rico(&n, &m.0, 1, 2);
    assert_ne!(m.0.region_en(s4.x, s4.y), Some(region), "la cuarta plaza cae en otra región");
    let p4 = fundar(&mut n, "gus2", 2, s4);
    admin(&mut n, Comando::FijarRelacion { a: 1, b: 3, relacion: Some(Relacion::Guerra) });

    for f in [1, 3] {
        admin(&mut n, Comando::FijarNivelFaccion { faccion: f, nivel: 2 });
    }
    admin(&mut n, Comando::FijarNivelAsentamiento { asentamiento: p1, nivel: 2 });
    for plaza in [p1, p2, p3, p4] {
        admin(&mut n, Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
        admin(&mut n, Comando::DarRecursos { asentamiento: plaza, recurso: Madera, cantidad: 350_000 });
        if plaza != p1 || mercado_en_p1 {
            admin(&mut n, Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
        }
    }
    if taberna {
        for r in [Piedra, Oro, Trigo] {
            admin(&mut n, Comando::DarRecursos { asentamiento: p1, recurso: r, cantidad: 400_000 });
        }
        admin(&mut n, Comando::AnadirEdificio { asentamiento: p1, tipo: Tipo::Taberna });
    }
    let levantado = |n: &Nucleo, plaza: u64, tipo: Tipo| n.economia(plaza).unwrap().edificios.iter().any(|b| b.tipo == tipo && b.activo());
    for _ in 0..40 {
        n.avanzar_hasta(n.ahora() + HORA);
        let listo = [p2, p3, p4].iter().all(|&p| levantado(&n, p, Tipo::Mercado)) && (!mercado_en_p1 || levantado(&n, p1, Tipo::Mercado)) && (!taberna || levantado(&n, p1, Tipo::Taberna));
        if listo {
            break;
        }
        for r in [Madera, Piedra, Trigo] {
            admin(&mut n, Comando::DarRecursos { asentamiento: p1, recurso: r, cantidad: 100_000 });
        }
    }
    assert!((!mercado_en_p1 || levantado(&n, p1, Tipo::Mercado)) && (!taberna || levantado(&n, p1, Tipo::Taberna)), "el Mercado y la Taberna acaban por levantarse");
    admin(&mut n, Comando::DarRecursos { asentamiento: p1, recurso: Oro, cantidad: 400_000 });
    admin(&mut n, Comando::DarRecursos { asentamiento: p2, recurso: Oro, cantidad: 400_000 });

    // Dentro de la plaza: el Rey, su Gobernador y un ciudadano sin cargo.
    let gob = mc::heroe_en(&mut n, "gob", 0, Controlador::Humano);
    let civ = mc::heroe_en(&mut n, "civ", 0, Controlador::Humano);
    for h in [gob, civ] {
        admin(&mut n, Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
    }
    mc::con_cargo(&mut n, "eva", eva, p1, Cargo::Gobernador, gob);
    for h in [eva, gob, civ] {
        admin(&mut n, Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(p1 as u32) });
    }
    E { n, eva, gob, civ, p1, p2, p3, p4, region }
}

impl E {
    fn oro(&self, plaza: u64) -> i64 {
        self.n.economia(plaza).unwrap().almacen.cantidad(Oro)
    }

    fn cantidad(&self, plaza: u64, r: Recurso) -> i64 {
        self.n.economia(plaza).unwrap().almacen.cantidad(r)
    }

    fn comprar(&mut self, heroe: u64, tipo: TipoInformeRegion) -> nucleo::Respuesta {
        let (cuenta, p1) = (self.n.heroe(heroe).unwrap().cuenta.clone(), self.p1);
        mc::dar(&mut self.n, &cuenta, Comando::ComprarInformeRegion { heroe, asentamiento: p1, tipo })
    }

    fn comprar_ok(&mut self, tipo: TipoInformeRegion) {
        let eva = self.eva;
        let r = self.comprar(eva, tipo);
        assert!(r.aceptada(), "{r:?}");
    }

    fn comprar_rechazado(&mut self, heroe: u64, tipo: TipoInformeRegion) -> &'static str {
        let (cuenta, p1) = (self.n.heroe(heroe).unwrap().cuenta.clone(), self.p1);
        mc::rechazo(&mut self.n, &cuenta, Comando::ComprarInformeRegion { heroe, asentamiento: p1, tipo })
    }

    /// Coloca en la plaza `plaza` una orden de venta o de compra de `cantidad` del recurso a un oro por unidad y devuelve su id (la de venta, con el stock que pide).
    fn orden(&mut self, plaza: u64, venta: bool, recurso: Recurso, cantidad: i64) -> u64 {
        if venta {
            mc::ok(&mut self.n, "admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad });
        }
        mc::ok(&mut self.n, "admin", Comando::ColocarOrdenMercado { asentamiento: plaza, venta, recurso, cantidad, precio: Some(UN_ORO) }).creadas[0]
    }

    fn intel(&self) -> militar::datos::ReglasIntel {
        self.n.estado().reglas.economia.militar.intel.clone()
    }
}

fn todo() -> Rect {
    Rect { min: Punto::new(0, 0), max: Punto::new(2_667_000, 2_000_000) }
}

/// Las órdenes de mercado que la `VISTA` de la Facción 1 lleva ahora.
fn ordenes_en_la_vista(n: &Nucleo) -> BTreeSet<u64> {
    let m = Mirada::de(n, Perspectiva::Faccion(1));
    m.contenido(None, todo(), &BTreeMap::new()).into_iter().filter(|x| x.clave.0 == TipoEntidad::OrdenMercado as i32).filter_map(|x| ids::parse_orden_de_mercado(&x.clave.1)).collect()
}

// ───────────────────────────── 1. Comprar el informe de la región ─────────────────────────────

#[test]
fn sin_taberna_o_sin_el_cargo_no_se_compra_ningun_informe_de_region() {
    let m = con_mundo!();
    let mut sin = escenario(m, false);
    let eva = sin.eva;
    for tipo in [TipoInformeRegion::Plazas, TipoInformeRegion::Ordenes] {
        assert_eq!(sin.comprar_rechazado(eva, tipo), rechazos::INTEL_SIN_TABERNA, "la plaza no tiene Taberna");
    }
    let mut e = escenario(m, true);
    let civ = e.civ;
    assert_eq!(e.comprar_rechazado(civ, TipoInformeRegion::Plazas), rechazos::NO_AUTORIZADO, "un ciudadano sin cargo no compra intel");
    // Fuera de la plaza tampoco, aunque sea el Rey.
    let eva = e.eva;
    mc::ok(&mut e.n, "admin", Comando::ColocarHeroe { heroe: eva, ubicacion: Ubicacion::Mercenarios(0) });
    assert_eq!(e.comprar_rechazado(eva, TipoInformeRegion::Ordenes), rechazos::NO_AUTORIZADO);
    assert!(e.n.estado().memoria.informes_region.is_empty());
}

#[test]
fn los_dos_informes_cuestan_lo_que_dice_intel_region_aunque_la_plaza_sea_de_nivel_2_y_se_cobran_del_almacen_de_la_plaza() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (p1, eva) = (e.p1, e.eva);
    let precio = e.intel().oro_region;
    assert_eq!(e.n.estado().asentamientos[&p1].nivel(), 2, "el precio es fijo: no depende del nivel de la plaza");
    let personal = mc::oro_personal(&e.n, eva);
    for tipo in [TipoInformeRegion::Plazas, TipoInformeRegion::Ordenes] {
        let antes = e.oro(p1);
        e.comprar_ok(tipo);
        assert_eq!(antes - e.oro(p1), precio, "{tipo:?}: el oro sale del almacén de la plaza");
        assert_eq!(e.n.estado().memoria.informes_region[&(1, e.region, tipo)], e.n.ahora());
    }
    assert_eq!(mc::oro_personal(&e.n, eva), personal, "y no del héroe");
}

#[test]
fn el_mismo_tipo_de_la_misma_region_espera_su_enfriamiento_y_el_otro_tipo_no() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (eva, enfriamiento) = (e.eva, e.intel().enfriamiento_region);
    e.comprar_ok(TipoInformeRegion::Plazas);
    let comprado = e.n.ahora();
    assert_eq!(e.comprar_rechazado(eva, TipoInformeRegion::Plazas), rechazos::INTEL_INFORME_RECIENTE);
    e.comprar_ok(TipoInformeRegion::Ordenes);
    e.n.avanzar_hasta(comprado + enfriamiento - 1);
    assert_eq!(e.comprar_rechazado(eva, TipoInformeRegion::Plazas), rechazos::INTEL_INFORME_RECIENTE, "un instante antes");
    e.n.avanzar_hasta(comprado + enfriamiento);
    e.comprar_ok(TipoInformeRegion::Plazas);
    // Al comprar se limpia lo que ya pasó su enfriamiento: el de órdenes, comprado antes, también.
    let guardados: Vec<_> = e.n.estado().memoria.informes_region.keys().map(|&(_, _, tipo)| tipo).collect();
    assert_eq!(guardados, vec![TipoInformeRegion::Plazas], "queda solo el recién comprado");
    // Ni la compra ni el enfriamiento avisan a nadie.
    let avisos = e.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|ev| ev.codigo.contains("informe")).count();
    assert_eq!(avisos, 0);
}

#[test]
fn los_informes_de_region_viajan_en_la_foto_y_en_la_faccion_propia_pero_no_en_la_publica() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    e.comprar_ok(TipoInformeRegion::Plazas);
    e.comprar_ok(TipoInformeRegion::Ordenes);
    let (t, intel) = (e.n.ahora(), e.intel());
    let bytes = e.n.estado().serializar();
    let vuelta = Estado::deserializar(&bytes, &e.n.estado().reglas.clone(), m.0.hash()).unwrap();
    assert_eq!(vuelta.memoria.informes_region, e.n.estado().memoria.informes_region);
    assert_eq!(vuelta.serializar(), bytes);
    let x = propios::faccion(&e.n, 1);
    let vistos: Vec<(u32, &str, i64, Option<i64>, i64)> = x.informes_region.iter().map(|i| (i.region, i.tipo.as_str(), i.comprado_en_ms, i.caduca_en_ms, i.libre_en_ms)).collect();
    let region = u32::from(e.region);
    assert_eq!(vistos, vec![(region, "plazas", t, None, t + intel.enfriamiento_region), (region, "ordenes", t, Some(t + intel.caducidad_region), t + intel.enfriamiento_region)]);
    assert!(propios::faccion_publica(&e.n, 1).informes_region.is_empty(), "las demás Facciones no ven lo que compró");
}

#[test]
fn el_informe_de_plazas_graba_las_fichas_de_la_region_sin_las_propias_ni_las_de_una_faccion_en_guerra_ni_las_de_otra_region() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (p1, p2, p3, p4, eva) = (e.p1, e.p2, e.p3, e.p4, e.eva);
    let fichas = |e: &E| e.n.estado().memoria.fichas.get(&1).cloned().unwrap_or_default();
    assert!(!fichas(&e).contains_key(&Sitio::Plaza(p2)), "antes de comprar no conoce la plaza lejana");
    let informe_de_plaza = |e: &mut E| mc::dar(&mut e.n, "eva", Comando::ComprarInformePlaza { heroe: eva, origen: nucleo::memoria::OrigenIntel::Plaza(p1), asentamiento: p2 });
    let r = informe_de_plaza(&mut e);
    assert!(format!("{:?}", r.resultado).contains(rechazos::INTEL_PLAZA_DESCONOCIDA), "{r:?}");

    e.comprar_ok(TipoInformeRegion::Plazas);
    let f = fichas(&e);
    let ficha = f[&Sitio::Plaza(p2)].clone();
    assert_eq!((ficha.conocido_en, ficha.datos.faccion, ficha.datos.posicion), (e.n.ahora(), 2, e.n.estado().asentamientos[&p2].posicion), "fechada en la compra, como la grabaría una columna");
    // Con su nombre de entonces (2026-11-10, pedido del cliente: la lista enseñaba solo el id), que la `VISTA` lleva en la ficha conocida.
    let nombre = e.n.estado().asentamientos[&p2].nombre.clone();
    assert!(!nombre.is_empty() && ficha.datos.nombre == nombre, "la ficha guarda el nombre de la plaza");
    let v = Mirada::de(&e.n, Perspectiva::Faccion(1));
    let conocida = v.contenido(None, todo(), &BTreeMap::new()).into_iter().find(|x| x.clave == (TipoEntidad::FichaConocida as i32, ids::asentamiento(p2))).and_then(|x| x.cuerpo());
    let Some(contrato::v1::entidad::Cuerpo::FichaConocida(conocida)) = conocida else { panic!("la VISTA recuerda la plaza: {conocida:?}") };
    assert_eq!(conocida.nombre.as_deref(), Some(nombre.as_str()));
    for (plaza, motivo) in [(p1, "la propia"), (p3, "la de la Facción en guerra"), (p4, "la de otra región")] {
        assert!(!f.contains_key(&Sitio::Plaza(plaza)), "{motivo} no entra");
    }
    // Y como ya la conoce, puede comprar su Informe de plaza.
    let r = informe_de_plaza(&mut e);
    assert!(r.aceptada(), "{r:?}");
}

#[test]
fn una_faccion_aliada_o_vasalla_entra_en_el_informe_y_una_en_guerra_no() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (p2, p3) = (e.p2, e.p3);
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Alianza) });
    let (o2, o3) = (e.orden(p2, true, Madera, 50_000), e.orden(p3, true, Piedra, 50_000));
    e.comprar_ok(TipoInformeRegion::Ordenes);
    assert_eq!(ordenes_en_la_vista(&e.n), BTreeSet::from([o2]), "la aliada entra; la enemiga, nunca");
    // Al terminar la guerra con F3 (a neutral), su plaza entra: el informe ya comprado vale para toda plaza que entre hoy.
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 3, relacion: None });
    assert_eq!(ordenes_en_la_vista(&e.n), BTreeSet::from([o2, o3]));
}

// ───────────────────────────── 2. Las órdenes en la VISTA ─────────────────────────────

#[test]
fn el_informe_de_ordenes_lleva_las_de_la_region_en_la_vista_mientras_vale_y_nunca_las_enemigas_ni_las_de_otra_region() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (p1, p2, p3, p4) = (e.p1, e.p2, e.p3, e.p4);
    let (venta, compra) = (e.orden(p2, true, Madera, 50_000), e.orden(p2, false, Cobre, 40_000));
    let (propia, enemiga, lejana) = (e.orden(p1, true, Trigo, 10_000), e.orden(p3, true, Piedra, 50_000), e.orden(p4, true, Trigo, 50_000));
    assert!(ordenes_en_la_vista(&e.n).is_empty(), "sin informe, ni una");
    // El informe de plazas no da las órdenes.
    e.comprar_ok(TipoInformeRegion::Plazas);
    assert!(ordenes_en_la_vista(&e.n).is_empty());

    e.comprar_ok(TipoInformeRegion::Ordenes);
    let (comprado, caducidad) = (e.n.ahora(), e.intel().caducidad_region);
    assert_eq!(ordenes_en_la_vista(&e.n), BTreeSet::from([venta, compra]), "las de la plaza neutral de la región; no la propia, ni la enemiga, ni la de otra región");
    for (orden, quien) in [(propia, "propia"), (enemiga, "enemiga"), (lejana, "de otra región")] {
        assert!(!ordenes_en_la_vista(&e.n).contains(&orden), "la {quien} no llega");
    }
    e.n.avanzar_hasta(comprado + caducidad - 1);
    assert_eq!(ordenes_en_la_vista(&e.n).len(), 2, "vale hasta justo antes de caducar");
    e.n.avanzar_hasta(comprado + caducidad);
    assert!(ordenes_en_la_vista(&e.n).is_empty(), "al caducar dejan de llegar, sin aviso");
}

// ───────────────────────────── 3. Tomar una orden a distancia ─────────────────────────────

impl E {
    /// Una caravana de la plaza `p1` con `carros` carros con buey.
    fn caravana(&mut self, carros: u32) -> u64 {
        let p1 = self.p1;
        // Crear una caravana tiene su enfriamiento (`CARAVANA_COOLDOWN`, 10 minutos): se deja pasar antes de cada una.
        self.n.avanzar_hasta(self.n.ahora() + 10 * MIN);
        let id = mc::ok(&mut self.n, "admin", Comando::CrearCaravana { asentamiento: p1 }).creadas[0];
        for k in 0..carros {
            mc::ok(&mut self.n, "admin", Comando::AnadirCarro { caravana: id, tipo: TipoCarro::Basico });
            mc::ok(&mut self.n, "admin", Comando::AnadirAnimal { caravana: id, carro: k, animal: Animal::Buey });
        }
        id
    }

    fn tomar(&mut self, orden: u64, cantidad: i64) -> nucleo::Respuesta {
        let p1 = self.p1;
        mc::dar(&mut self.n, "gob", Comando::TomarOrdenADistancia { asentamiento: p1, orden, cantidad })
    }

    fn tomar_rechazada(&mut self, actor: &str, orden: u64, cantidad: i64) -> &'static str {
        let p1 = self.p1;
        mc::rechazo(&mut self.n, actor, Comando::TomarOrdenADistancia { asentamiento: p1, orden, cantidad })
    }

    /// Avanza hasta que la caravana vuelve a estar libre en su origen (el envío acaba al volver), hasta un día.
    fn hasta_que_vuelva(&mut self, car: u64) {
        for _ in 0..24 * 6 {
            let k = self.n.caravana(car).unwrap();
            if k.estado == EstadoCaravana::Disponible && k.encargo.is_none() {
                return;
            }
            self.n.avanzar_hasta(self.n.ahora() + 10 * MIN);
        }
        panic!("la caravana no volvió en un día: {:?}", self.n.caravana(car).unwrap().estado);
    }

    fn oro_a_bordo(&self, car: u64) -> i64 {
        self.n.caravana(car).unwrap().contenido.get(&Oro).copied().unwrap_or(0)
    }
}

/// Lo que cobró o pagó la caravana en las plazas ajenas, según los hechos del mercado: el oro que se movió.
fn oro_movido(n: &Nucleo) -> i64 {
    mc::hechos(n, "mercado.comercio_en_plaza").iter().map(|h| h.dato("oroMovido").expect("el hecho lleva el oro movido")).sum()
}

#[test]
fn tomar_una_orden_a_distancia_comprueba_en_su_orden_la_plaza_el_mercado_la_orden_la_cantidad_el_informe_y_la_caravana() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (p2, p3, p4, p1) = (e.p2, e.p3, e.p4, e.p1);
    let venta = e.orden(p2, true, Madera, 50_000);
    let (enemiga, lejana, propia) = (e.orden(p3, true, Piedra, 50_000), e.orden(p4, true, Trigo, 50_000), e.orden(p1, true, Trigo, 10_000));
    // La matriz de la plaza: el Gobernador, el Tesorero o el Maestro de Caravanas; un ciudadano sin cargo, no.
    assert_eq!(e.tomar_rechazada("civ", venta, 10_000), rechazos::NO_AUTORIZADO);
    assert_eq!(e.tomar_rechazada("gob", 9_999_999, 10_000), rechazos::ORDEN_DESCONOCIDA);
    assert_eq!(e.tomar_rechazada("gob", venta, 0), rechazos::ORDEN_CANTIDAD_NO_POSITIVA);
    assert_eq!(e.tomar_rechazada("gob", venta, 10_000), rechazos::INTEL_SIN_INFORME_DE_ORDENES, "sin informe de la región");
    // Un informe de plazas no vale para las órdenes.
    e.comprar_ok(TipoInformeRegion::Plazas);
    assert_eq!(e.tomar_rechazada("gob", venta, 10_000), rechazos::INTEL_SIN_INFORME_DE_ORDENES);

    e.comprar_ok(TipoInformeRegion::Ordenes);
    for (orden, quien) in [(enemiga, "enemiga"), (lejana, "de otra región"), (propia, "propia")] {
        assert_eq!(e.tomar_rechazada("gob", orden, 10_000), rechazos::INTEL_SIN_INFORME_DE_ORDENES, "la orden {quien}");
    }
    assert_eq!(e.tomar_rechazada("gob", venta, 10_000), rechazos::CARAVANA_NO_DISPONIBLE, "no hay caravana libre");
    // Una orden que ya no está activa.
    let cancelada = e.orden(p2, true, Cobre, 10_000);
    mc::ok(&mut e.n, "admin", Comando::CancelarOrdenMercado { orden: cancelada });
    assert_eq!(e.tomar_rechazada("gob", cancelada, 10_000), rechazos::ORDEN_NO_ACTIVA);
    // Con una caravana libre pero sin capacidad para la cantidad, tampoco hay caravana.
    let car = e.caravana(1);
    assert_eq!(e.tomar_rechazada("gob", venta, 100_000_000), rechazos::CARAVANA_NO_DISPONIBLE, "ninguna llega a esa cantidad");
    assert!(e.n.caravana(car).unwrap().encargo.is_none());
}

#[test]
fn tomar_una_orden_a_distancia_exige_un_mercado_activo_en_la_plaza_que_toma() {
    let m = con_mundo!();
    let mut e = escenario_con(m, true, false);
    assert_eq!(e.tomar_rechazada("gob", 1, 10_000), rechazos::MERCADO_SIN_MERCADO, "antes que mirar la orden");
}

#[test]
fn tomar_una_orden_de_venta_envia_la_caravana_que_vuelve_con_lo_comprado_y_el_oro_sale_de_la_capital() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (p1, p2) = (e.p1, e.p2);
    let venta = e.orden(p2, true, Hierro, 50_000);
    e.comprar_ok(TipoInformeRegion::Ordenes);
    let (primera, segunda) = (e.caravana(1), e.caravana(1));
    assert!(primera < segunda);
    let (hierro, oro_p1, hierro_p2) = (e.cantidad(p1, Hierro), e.oro(p1), e.cantidad(p2, Hierro));

    let r = e.tomar(venta, 50_000);
    assert!(r.aceptada(), "{r:?}");
    let k = e.n.caravana(primera).unwrap();
    assert!(e.n.caravana(segunda).unwrap().encargo.is_none(), "la primera por id");
    let encargo = k.encargo.clone().expect("la primera tiene el encargo");
    assert_eq!((encargo.modo, u64::from(encargo.destino), encargo.salir_en), (comercio::encargo::Modo::Envio, p2, None), "un envío a la plaza de la orden que sale ya");
    assert!(encargo.en_origen.recoger.is_empty() && encargo.en_origen.dejar.is_empty());
    let linea = &encargo.en_destino.recoger[0];
    assert_eq!((linea.recurso, linea.cantidad, linea.precio_max), (Hierro, 50_000, Some(UN_ORO)), "compra allí al precio de la orden como tope");
    assert_eq!(e.oro_a_bordo(primera), 0, "la caravana no lleva oro");

    e.hasta_que_vuelva(primera);
    assert_eq!(e.oro_a_bordo(primera), 0, "ni vuelve con él");
    assert_eq!(e.cantidad(p1, Hierro) - hierro, 50_000, "trae lo comprado y lo deja en el almacén de la plaza");
    assert_eq!(hierro_p2 - e.cantidad(p2, Hierro), 50_000, "la orden se vació");
    let (pagado, movido, plazas) = (oro_p1 - e.oro(p1), oro_movido(&e.n), e.n.estado().asentamientos.len() as i64);
    // La capital (la única plaza de la Facción) paga lo movido; en el viaje cada plaza recauda hasta una unidad.
    assert!(movido > 0 && (pagado - movido).abs() <= 1_000 * plazas, "movido {movido}, pagado por la capital {pagado}");
}

#[test]
fn tomar_una_orden_de_compra_lleva_la_mercancia_de_la_plaza_la_vende_y_el_oro_entra_en_la_capital() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (p1, p2) = (e.p1, e.p2);
    let compra = e.orden(p2, false, Cobre, 50_000);
    mc::ok(&mut e.n, "admin", Comando::DarRecursos { asentamiento: p1, recurso: Cobre, cantidad: 50_000 });
    e.comprar_ok(TipoInformeRegion::Ordenes);
    let car = e.caravana(1);
    let (cobre_p1, cobre_p2, oro_p1) = (e.cantidad(p1, Cobre), e.cantidad(p2, Cobre), e.oro(p1));

    let r = e.tomar(compra, 50_000);
    assert!(r.aceptada(), "{r:?}");
    let encargo = e.n.caravana(car).unwrap().encargo.clone().expect("tiene el encargo");
    let (recoge, deja) = (&encargo.en_origen.recoger[0], &encargo.en_destino.dejar[0]);
    assert_eq!((recoge.recurso, recoge.cantidad, recoge.precio_max), (Cobre, 50_000, None), "en el origen toma de su almacén lo que vende");
    assert_eq!((deja.recurso, deja.tope, deja.precio_min), (Cobre, Some(50_000), Some(UN_ORO)), "y lo vende allí con el precio de la orden como suelo");
    e.n.avanzar_hasta(e.n.ahora() + MIN);
    assert_eq!(e.oro_a_bordo(car), 0, "no lleva oro");
    assert!(e.cantidad(p1, Cobre) < cobre_p1, "la mercancía ha salido del almacén de la plaza");

    e.hasta_que_vuelva(car);
    assert_eq!(cobre_p1 - e.cantidad(p1, Cobre), 50_000);
    assert_eq!(e.cantidad(p2, Cobre) - cobre_p2, 50_000, "la plaza compradora la recibe");
    let (cobrado, movido, plazas) = (e.oro(p1) - oro_p1, oro_movido(&e.n), e.n.estado().asentamientos.len() as i64);
    assert!(movido > 0 && (cobrado - movido).abs() <= 1_000 * plazas, "movido {movido}, cobrado por la capital {cobrado}");
    assert_eq!(e.oro_a_bordo(car), 0);
}

// ───────────────────────────── 4. Los mercenarios de la región ─────────────────────────────

/// Tres bots ofrecidos: uno en el campamento 0 y otro en el 6 (de la región de `p1`), y otro en el 1 (de otra).
fn tres_bots(e: &mut E) -> (u64, u64, u64) {
    let region_de = |c: usize| comun::real::mundo().and_then(|m| m.0.region_en(comun::real::campamento(&m.0, c).x, comun::real::campamento(&m.0, c).y));
    assert_eq!((region_de(0), region_de(6)), (Some(e.region), Some(e.region)), "los campamentos 0 y 6 caen en la región de la plaza");
    assert_ne!(region_de(1), Some(e.region), "el 1, en otra");
    (mc::bot_ofrecido(&mut e.n, "bot-a", 0), mc::bot_ofrecido(&mut e.n, "bot-b", 6), mc::bot_ofrecido(&mut e.n, "bot-c", 1))
}

#[test]
fn la_taberna_enseña_gratis_el_tablon_de_los_campamentos_de_su_region_y_una_plaza_sin_taberna_no() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (a, b, c) = tres_bots(&mut e);
    let plaza = interior::asentamiento(&e.n, e.p1).expect("la plaza existe");
    let ofertas: Vec<(&str, &str)> = plaza.mercenarios_region.iter().map(|o| (o.heroe_id.as_str(), o.campamento_id.as_str())).collect();
    assert_eq!(ofertas, vec![(ids::heroe(a).as_str(), ids::campamento(0).as_str()), (ids::heroe(b).as_str(), ids::campamento(6).as_str())], "los de su región, por héroe; no el de otra");
    assert!(!ofertas.iter().any(|(h, _)| *h == ids::heroe(c)));
    assert!(plaza.mercenarios_region.iter().all(|o| !o.tarifas.is_empty() && o.ficha.is_some()), "con la forma del tablón del campamento");
    let del_campamento = interior::campamento_por_dentro(&e.n, 0, propios::Espectador::Admin).unwrap();
    assert_eq!(del_campamento.tablon[0].campamento_id, ids::campamento(0), "el tablón del campamento también dice el suyo");
    assert_eq!(del_campamento.tablon[0].tarifas, plaza.mercenarios_region[0].tarifas, "las mismas tarifas");

    let sin = escenario(m, false);
    let mut sin = sin;
    tres_bots(&mut sin);
    assert!(interior::asentamiento(&sin.n, sin.p1).unwrap().mercenarios_region.is_empty(), "sin Taberna activa, nada");
}

#[test]
fn contratar_desde_la_taberna_paga_el_almacen_de_la_plaza_o_las_monedas_y_el_mercenario_se_pone_en_camino() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (a, b, _) = tres_bots(&mut e);
    let (p1, eva) = (e.p1, e.eva);
    let plaza = interior::asentamiento(&e.n, p1).unwrap();
    let tarifa = |heroe: u64| plaza.mercenarios_region.iter().find(|o| o.heroe_id == ids::heroe(heroe)).unwrap().tarifas.iter().find(|t| t.plazo_dias == Some(1)).map(|t| (t.oro, t.monedas)).unwrap();
    let firma = |mercenario, pagar_con| Comando::ContratarMercenario { heroe: eva, mercenario, trabajo: mercenarios::Trabajo::Defender, objetivo: Objetivo::Plaza(p1), plazo: Some(1), residencia: None, pagar_con };

    // Sin `pagar_con`, el almacén de la plaza: el oro sale de él y va al almacén personal del mercenario.
    let (oro_a, _) = tarifa(a);
    let (antes, del_bot, monedas) = (e.oro(p1), mc::oro_personal(&e.n, a), e.n.heroe(eva).unwrap().heroe.monedas);
    mc::ok(&mut e.n, "eva", firma(a, None));
    assert_eq!((antes - e.oro(p1), mc::oro_personal(&e.n, a) - del_bot, e.n.heroe(eva).unwrap().heroe.monedas), (oro_a, oro_a, monedas), "paga la plaza, no las monedas del héroe");
    let contrato = mc::contrato(&e.n, a).expect("contratado");
    assert_eq!((contrato.faccion, contrato.firmado_por), (1, eva));
    assert!(contrato.orden.is_some(), "se pone en camino solo, con la orden de su trabajo");
    assert!(!mc::ofrecido(&e.n, a), "sale del tablón");
    assert!(interior::asentamiento(&e.n, p1).unwrap().mercenarios_region.iter().all(|o| o.heroe_id != ids::heroe(a)));

    // Con monedas: salen del héroe y van a las del mercenario; el almacén de la plaza no se toca.
    let (_, monedas_b) = tarifa(b);
    let (antes, del_bot, monedas) = (e.oro(p1), e.n.heroe(b).unwrap().heroe.monedas, e.n.heroe(eva).unwrap().heroe.monedas);
    mc::ok(&mut e.n, "eva", firma(b, Some(PagarCon::Monedas)));
    assert_eq!((e.oro(p1), e.n.heroe(b).unwrap().heroe.monedas - del_bot, monedas - e.n.heroe(eva).unwrap().heroe.monedas), (antes, monedas_b, monedas_b));
}

#[test]
fn desde_la_taberna_un_campamento_de_otra_region_un_pago_que_no_es_de_la_plaza_o_quien_no_firma_se_rechazan() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (a, _, c) = tres_bots(&mut e);
    let (p1, eva, civ, gob) = (e.p1, e.eva, e.civ, e.gob);
    let firma = |heroe, mercenario, pagar_con| Comando::ContratarMercenario { heroe, mercenario, trabajo: mercenarios::Trabajo::Defender, objetivo: Objetivo::Plaza(p1), plazo: Some(1), residencia: None, pagar_con };
    assert_eq!(mc::rechazo(&mut e.n, "eva", firma(eva, c, None)), rechazos::MERCENARIO_FUERA_DE_LA_REGION, "el campamento 1 cae en otra región");
    assert_eq!(mc::rechazo(&mut e.n, "eva", firma(eva, c, Some(PagarCon::Monedas))), rechazos::MERCENARIO_FUERA_DE_LA_REGION);
    // Fuera del campamento solo valen el almacén de la plaza y las monedas.
    for forma in [PagarCon::AlmacenPersonal, PagarCon::Carro] {
        assert_eq!(mc::rechazo(&mut e.n, "eva", firma(eva, a, Some(forma))), rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA, "{forma:?}");
    }
    // Con el almacén de la plaza firman el Rey, el Embajador o el Gobernador; con otro cargo no hay almacén, y sin cargo las reglas puras no le dan alcance.
    assert_eq!(mc::rechazo(&mut e.n, "civ", firma(civ, a, None)), rechazos::NO_AUTORIZADO);
    assert_eq!(mc::rechazo(&mut e.n, "civ", firma(civ, a, Some(PagarCon::Monedas))), rechazos::MERCENARIO_SIN_ALCANCE);
    let r = mc::dar(&mut e.n, "gob", firma(gob, a, None));
    assert!(!r.aceptada() || mc::contrato(&e.n, a).is_some_and(|c| c.firmado_por == gob), "el Gobernador de la plaza también firma con su almacén: {r:?}");
    // En el campamento el almacén de la plaza no paga, y quien no está en el campamento ni en una plaza suya con Taberna no llega a la puerta.
    mc::ok(&mut e.n, "admin", Comando::ColocarHeroe { heroe: civ, ubicacion: Ubicacion::Mercenarios(1) });
    assert_eq!(mc::rechazo(&mut e.n, "civ", firma(civ, c, Some(PagarCon::AlmacenPlaza))), rechazos::ORDEN_FORMA_INVALIDA);
    let mut sin = escenario(m, false);
    let (a, _, _) = tres_bots(&mut sin);
    let (p1, eva) = (sin.p1, sin.eva);
    let c = Comando::ContratarMercenario { heroe: eva, mercenario: a, trabajo: mercenarios::Trabajo::Defender, objetivo: Objetivo::Plaza(p1), plazo: Some(1), residencia: None, pagar_con: None };
    assert_eq!(mc::rechazo(&mut sin.n, "eva", c), rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA, "una plaza sin Taberna no sirve");
}

/// La tarifa de un día de `heroe` en el tablón de la taberna de `p1`, en oro y en monedas.
fn tarifa_de_un_dia(e: &E, heroe: u64) -> (i64, i64) {
    let plaza = interior::asentamiento(&e.n, e.p1).unwrap();
    plaza.mercenarios_region.iter().find(|o| o.heroe_id == ids::heroe(heroe)).unwrap().tarifas.iter().find(|t| t.plazo_dias == Some(1)).map(|t| (t.oro, t.monedas)).unwrap()
}

/// El fondo de monedas de la plaza (el del impuesto de su forja).
fn fondo(n: &Nucleo, plaza: u64) -> i64 {
    n.estado().asentamientos[&plaza].forja.fondo
}

/// Con `pagar_con = fondoPlaza` (Forja_En_Plazas §6) el Rey paga desde la taberna la tarifa en monedas con el fondo de la plaza, que se llena a mano (ninguna forja ha pagado aún impuesto
/// aquí): las monedas salen del fondo y pasan a las del mercenario, y ni el almacén de la plaza ni las monedas del héroe se tocan.
#[test]
fn con_el_fondo_de_la_plaza_se_paga_desde_la_taberna_la_tarifa_en_monedas_y_pasan_al_mercenario() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (a, _, _) = tres_bots(&mut e);
    let (p1, eva) = (e.p1, e.eva);
    let (_, monedas) = tarifa_de_un_dia(&e, a);
    let (n, _dir) = mc::retocado(m, &e.n, |s| s.asentamientos.get_mut(&p1).unwrap().forja.fondo = monedas + 7);
    e.n = n;
    let firma = Comando::ContratarMercenario { heroe: eva, mercenario: a, trabajo: mercenarios::Trabajo::Defender, objetivo: Objetivo::Plaza(p1), plazo: Some(1), residencia: None, pagar_con: Some(PagarCon::FondoPlaza) };
    let (oro, del_bot, de_eva) = (e.oro(p1), e.n.heroe(a).unwrap().heroe.monedas, e.n.heroe(eva).unwrap().heroe.monedas);
    mc::ok(&mut e.n, "eva", firma);
    assert_eq!(fondo(&e.n, p1), 7, "la tarifa en monedas sale del fondo");
    assert_eq!(e.n.heroe(a).unwrap().heroe.monedas - del_bot, monedas, "y pasa a las monedas del mercenario");
    assert_eq!((e.oro(p1), mc::oro_personal(&e.n, a), e.n.heroe(eva).unwrap().heroe.monedas), (oro, 0, de_eva), "ni el almacén de la plaza, ni oro al mercenario, ni las monedas del Rey");
    assert_eq!(mc::contrato(&e.n, a).map(|c| c.firmado_por), Some(eva));
    let h = mc::hechos(&e.n, "mercenarios.contratado");
    assert_eq!((h.len(), h[0].dato("monedas"), h[0].dato("oro")), (1, Some(monedas), Some(0)), "el hecho lleva las monedas");
}

/// Con el fondo de la plaza: si no llega a la tarifa, `mercenarios.sin_fondo` sin cambiar nada; quien no firmaría con el almacén de la plaza tampoco con su fondo (`no_autorizado`); y en el
/// campamento o al prorrogar el fondo no paga (`orden.forma_invalida`), como el almacén de la plaza.
#[test]
fn el_fondo_de_la_plaza_corto_quien_no_firma_por_la_plaza_el_campamento_y_la_prorroga_se_rechazan() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (a, _, c) = tres_bots(&mut e);
    let (p1, eva, civ) = (e.p1, e.eva, e.civ);
    let (_, monedas) = tarifa_de_un_dia(&e, a);
    let (n, _dir) = mc::retocado(m, &e.n, |s| s.asentamientos.get_mut(&p1).unwrap().forja.fondo = monedas - 1);
    e.n = n;
    let firma = |heroe, mercenario| Comando::ContratarMercenario { heroe, mercenario, trabajo: mercenarios::Trabajo::Defender, objetivo: Objetivo::Plaza(p1), plazo: Some(1), residencia: None, pagar_con: Some(PagarCon::FondoPlaza) };

    let del_bot = e.n.heroe(a).unwrap().heroe.monedas;
    assert_eq!(mc::rechazo(&mut e.n, "eva", firma(eva, a)), rechazos::MERCENARIOS_SIN_FONDO, "al fondo le falta una moneda");
    assert_eq!((fondo(&e.n, p1), e.n.heroe(a).unwrap().heroe.monedas, mc::ofrecido(&e.n, a)), (monedas - 1, del_bot, true), "y no cambia nada");
    assert_eq!(mc::rechazo(&mut e.n, "civ", firma(civ, a)), rechazos::NO_AUTORIZADO, "con el fondo firman el Rey, el Embajador o el Gobernador");

    mc::ok(&mut e.n, "admin", Comando::ColocarHeroe { heroe: civ, ubicacion: Ubicacion::Mercenarios(1) });
    assert_eq!(mc::rechazo(&mut e.n, "civ", firma(civ, c)), rechazos::ORDEN_FORMA_INVALIDA, "en el campamento el fondo de la plaza no paga");

    // Contratado con el almacén de la plaza, la prórroga es en persona, en su campamento base, y allí el fondo no la paga.
    mc::ok(&mut e.n, "eva", Comando::ContratarMercenario { heroe: eva, mercenario: a, trabajo: mercenarios::Trabajo::Defender, objetivo: Objetivo::Plaza(p1), plazo: Some(1), residencia: None, pagar_con: None });
    mc::ok(&mut e.n, "admin", Comando::ColocarHeroe { heroe: eva, ubicacion: Ubicacion::Mercenarios(0) });
    assert_eq!(mc::rechazo(&mut e.n, "eva", Comando::ProrrogarContrato { heroe: eva, mercenario: a, plazo: 1, pagar_con: Some(PagarCon::FondoPlaza) }), rechazos::ORDEN_FORMA_INVALIDA);
    assert_eq!(fondo(&e.n, p1), monedas - 1, "el fondo sigue igual");
}

// ───────────────────────────── Rumores: las vetas de estaño (2026-11-10) ─────────────────────────────

/// Los yacimientos de estaño de la `region` y si la Facción 1 tiene explorado cada uno.
fn estano_de(e: &E, region: u8) -> Vec<(Punto, bool)> {
    let m = comun::real::mundo().expect("hay mundo");
    let mem = &e.n.estado().memoria;
    (m.0.nodos().iter())
        .filter(|x| x.tipo == "estano")
        .filter_map(|x| x.posicion.map(|p| Punto::new(p.x, p.y)))
        .filter(|p| m.0.region_en(p.x, p.y) == Some(region))
        .map(|p| (p, mem.exploracion.get(&1).is_some_and(|x| x.explorado_en(p))))
        .collect()
}

/// 2026-11-10 (decisión del autor): el rumor de las vetas de estaño deja explorado para la Facción el terreno alrededor de cada yacimiento de estaño de la región de la taberna, por
/// `INTEL.rumores.vetasEstano.oro` (80) del almacén de la plaza; lo compra quien compra la intel (el Rey, el Embajador o el Gobernador, dentro) y, si ya los conoce todos, se rechaza sin
/// cobrar.
#[test]
fn el_rumor_de_las_vetas_de_estano_explora_los_yacimientos_de_la_region_y_no_se_cobra_dos_veces() {
    let m = con_mundo!();
    let mut e = escenario(m, true);
    let (p1, eva, civ, region) = (e.p1, e.eva, e.civ, e.region);
    let antes = estano_de(&e, region);
    assert!(antes.iter().any(|&(_, explorado)| !explorado), "la región tiene estaño que la Facción no conoce: {antes:?}");
    let rumor = |heroe| Comando::ComprarRumor { heroe, origen: nucleo::memoria::OrigenIntel::Plaza(p1), tipo: nucleo::memoria::TipoRumor::VetasEstano };
    assert_eq!(mc::rechazo(&mut e.n, "civ", rumor(civ)), rechazos::NO_AUTORIZADO, "sin cargo no se compra");
    let (oro0, precio) = (e.oro(p1), e.intel().oro_rumor_estano);
    assert_eq!(precio, 80_000, "INTEL.rumores.vetasEstano.oro");
    mc::ok(&mut e.n, "eva", rumor(eva));
    assert_eq!(oro0 - e.oro(p1), precio, "paga el almacén de la plaza");
    let despues = estano_de(&e, region);
    assert!(despues.iter().all(|&(_, explorado)| explorado), "todos los yacimientos de la región quedan explorados: {despues:?}");
    // Lo de otras regiones no: el rumor es de la región de la taberna.
    let otra = e.n.estado().asentamientos[&e.p4].posicion;
    let otra = m.0.region_en(otra.x, otra.y).unwrap();
    let fuera = estano_de(&e, otra);
    assert!(fuera.iter().any(|&(_, explorado)| !explorado), "el estaño de otra región sigue sin explorar: {fuera:?}");
    // Ya no hay nada nuevo que contar: se rechaza sin cobrar.
    assert_eq!(mc::rechazo(&mut e.n, "eva", rumor(eva)), rechazos::RUMOR_SIN_NOVEDAD);
    // Viaja en la foto: es lo explorado de la Facción.
    let vuelta = Estado::deserializar(&e.n.estado().serializar(), &e.n.estado().reglas, &e.n.estado().mundo_hash).unwrap();
    assert_eq!(vuelta.memoria.exploracion.get(&1), e.n.estado().memoria.exploracion.get(&1));
}

/// En la taberna de un campamento lo compra cualquier héroe con Facción que esté dentro, con el oro de su almacén personal si reside allí; el rumor es de la región del campamento.
#[test]
fn el_rumor_tambien_se_compra_en_la_taberna_de_un_campamento_con_el_oro_del_heroe() {
    let m = con_mundo!();
    let mut e = escenario(m, false);
    // Un campamento de una región con estaño que la Facción 1 aún no conoce.
    let region_de = |i: usize| {
        let c = comun::real::campamento(&m.0, i);
        m.0.region_en(c.x, c.y)
    };
    let (i, region) = (0..m.0.campamentos().len())
        .filter_map(|i| Some((i, region_de(i)?)))
        .find(|&(_, r)| estano_de(&e, r).iter().any(|&(_, explorado)| !explorado))
        .expect("algún campamento cae en una región con estaño por conocer");
    let zoe = mc::heroe_en(&mut e.n, "zoe", i as u32, Controlador::Humano);
    let rumor = Comando::ComprarRumor { heroe: zoe, origen: nucleo::memoria::OrigenIntel::Campamento(i as u32), tipo: nucleo::memoria::TipoRumor::VetasEstano };
    assert_eq!(mc::rechazo(&mut e.n, "zoe", rumor.clone()), rechazos::INTEL_SIN_FACCION, "la intel es para una Facción");
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: zoe, faccion: Some(1) });
    assert_eq!(mc::rechazo(&mut e.n, "zoe", rumor.clone()), rechazos::INTEL_SIN_ORO, "sin oro en su almacén personal");
    mc::ok(&mut e.n, "admin", Comando::DarAlmacenPersonal { heroe: zoe, recurso: Oro, cantidad: 100_000 });
    mc::ok(&mut e.n, "zoe", rumor);
    let oro = e.n.heroe(zoe).unwrap().almacen_personal.get(&Oro).copied().unwrap_or(0);
    assert_eq!(oro, 100_000 - e.intel().oro_rumor_estano, "paga su almacén personal");
    assert!(estano_de(&e, region).iter().all(|&(_, explorado)| explorado));
}
