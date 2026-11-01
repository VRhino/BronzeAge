//! La proyección del encargo de una caravana (Comercio_Caravanas §5.8.4, §5.8.8 y §5.8.9; `Proyecciones_Y_Autorizacion.md`): lo que lleva la caravana propia y qué caravanas salen en el mapa. Sobre el
//! artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::datos::UNO;
use asentamiento::Recurso::{self, *};
use base::{Instante, Punto, MS_POR_MINUTO};
use comercio::caravana::{Animal, EstadoCaravana, TipoCarro};
use comercio::encargo::{Fase, LineaDejar, LineaRecoger, Modo, Parada};
use comercio::orden_mercado::EstadoOrden;
use comun::real::{fundar, sitio_rico, M};
use contrato::v1::{self as pb, entidad::Cuerpo, TipoEntidad};
use nucleo::proyeccion::ambito::Ambito;
use nucleo::proyeccion::ids;
use nucleo::proyeccion::propios::{self, Espectador};
use nucleo::proyeccion::vista::{Mirada, Perspectiva};
use nucleo::{Ajustes, Arranque, Clase, Comando, Nucleo, Peticion, Reglas, Resultado, Respuesta};
use std::collections::BTreeMap;
use vision::Rect;

const PUNTO: i64 = UNO;
const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;

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

/// Las reglas por defecto sin acoso ni anillo de bandidos: estas pruebas miden otra cosa.
fn arranque(m: &M, semilla: u64) -> Arranque {
    let mut reglas = Reglas::default();
    let b = &mut std::sync::Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    Arranque { reglas, ajustes: Ajustes::default(), semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn pos(n: &Nucleo, plaza: u64) -> Punto {
    n.estado().asentamientos[&plaza].posicion
}

/// Lo que recoge (`recurso`, `cantidad`) y los recursos que deja sin tope, en una parada de plaza propia.
fn parada(recoger: &[(Recurso, i64)], dejar: &[Recurso]) -> Parada {
    Parada {
        recoger: recoger.iter().map(|&(recurso, cantidad)| LineaRecoger { recurso, cantidad, precio_max: None }).collect(),
        dejar: dejar.iter().map(|&recurso| LineaDejar { recurso, tope: None, precio_min: None }).collect(),
        ..Parada::default()
    }
}

fn encargar(caravana: u64, destino: u64, modo: Modo, salir_en: Option<Instante>, en_origen: Parada, en_destino: Parada) -> Comando {
    Comando::EncargarCaravana { caravana, destino, modo, salir_en, en_origen, en_destino }
}

/// Dos plazas de la Facción 1 con Mercado (`a` y `b`, de campamentos distintos), con el tiempo parado en el instante en que el Mercado de `a` ya está activo.
struct Escenario {
    n: Nucleo,
    a: u64,
    b: u64,
    cuenta: u64,
}

impl Escenario {
    fn admin(&mut self, c: Comando) -> Respuesta {
        self.cuenta += 1;
        let p = Peticion::nueva("admin", &format!("o{}", self.cuenta), c).de_admin();
        let t = self.n.ahora();
        let r = self.n.aplicar_en(t, p);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    fn avanzar(&mut self, ms: i64) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    fn dar(&mut self, plaza: u64, recurso: Recurso, cantidad: i64) {
        self.cuenta += 1;
        let p = Peticion::nueva("admin", &format!("o{}", self.cuenta), Comando::DarRecursos { asentamiento: plaza, recurso, cantidad }).de_admin();
        let t = self.n.ahora();
        let r = self.n.aplicar_en(t, p);
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
    }
    /// Una caravana de un carro con buey en la plaza.
    fn caravana(&mut self, plaza: u64) -> u64 {
        let id = self.admin(Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
        self.admin(Comando::AnadirCarro { caravana: id, tipo: TipoCarro::Basico });
        self.admin(Comando::AnadirAnimal { caravana: id, carro: 0, animal: Animal::Buey });
        id
    }
    fn estado(&self, id: u64) -> EstadoCaravana {
        self.n.caravana(id).expect("existe la caravana").estado
    }
    fn tiempo_carga(&self) -> i64 {
        self.n.estado().reglas.economia.comercio.encargo.tiempo_carga
    }
    fn tiempo_descarga(&self) -> i64 {
        self.n.estado().reglas.economia.comercio.encargo.tiempo_descarga
    }
    /// Avanza hasta el fin del plan de la caravana, que llega y queda `Esperando`. Devuelve el instante de llegada.
    fn viajar(&mut self, id: u64) -> Instante {
        let fin = self.n.caravana(id).and_then(|c| c.plan.as_ref()).and_then(|p| p.fin()).expect("la caravana tiene plan");
        self.n.avanzar_hasta(fin);
        assert_eq!(self.estado(id), EstadoCaravana::Esperando, "llegó a su parada");
        fin
    }
    /// La caravana tal como la proyecta la Facción propia (`propios::caravana`, la de `FACCION`).
    fn proyectada(&self, id: u64) -> pb::Caravana {
        let p = propios::proyectar(&self.n, Espectador::Heroe(None), Clase::Caravana, id);
        match p.existen.into_iter().next() {
            Some((_, _, Cuerpo::Caravana(c))) => c,
            otro => panic!("se esperaba la caravana: {otro:?}"),
        }
    }
}

/// Funda las dos plazas de la Facción 1 (`a` con el Rey de la Facción) y les construye el Mercado.
fn escenario(m: &M, semilla: u64) -> Escenario {
    let mut n = Nucleo::nuevo(arranque(m, semilla));
    let pa = sitio_rico(&n, &m.0, 0, 1);
    let a = fundar(&mut n, "rey", 1, pa);
    let mut e = Escenario { n, a, b: 0, cuenta: 0 };
    e.admin(Comando::FijarNivelFaccion { faccion: 1, nivel: 2 });
    let pb = sitio_rico(&e.n, &m.0, 3, 1);
    e.b = fundar(&mut e.n, "rey_b", 1, pb);
    for plaza in [e.a, e.b] {
        e.admin(Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
        e.dar(plaza, Madera, 350_000);
        e.admin(Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
    }
    e.avanzar(9 * HORA);
    for plaza in [e.a, e.b] {
        assert!(e.n.economia(plaza).unwrap().edificios.iter().any(|x| x.tipo == Tipo::Mercado && x.activo()), "el Mercado de as-{plaza} debería estar activo");
    }
    e
}

/// Funda una plaza de la Facción 2 junto al campamento 1: una plaza ajena a la que ir.
fn con_ajena(e: &mut Escenario, m: &M) -> u64 {
    let p = sitio_rico(&e.n, &m.0, 1, 2);
    fundar(&mut e.n, "beto", 2, p)
}

fn texto_recoger(recurso: &str, cantidad: i64, precio_max: Option<i64>) -> pb::LineaRecoger {
    pb::LineaRecoger { recurso: recurso.into(), cantidad, precio_max }
}

fn texto_dejar(recurso: &str, tope: Option<i64>, precio_min: Option<i64>) -> pb::LineaDejar {
    pb::LineaDejar { recurso: recurso.into(), tope, precio_min }
}

// ───────────────────────────── La proyección propia ─────────────────────────────

#[test]
fn la_proyeccion_propia_lleva_el_encargo_con_sus_textos_y_cambia_de_parada_y_fase_a_lo_largo_del_ciclo() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 300_000);
    e.dar(b, Estano, 300_000);
    let car = e.caravana(a);
    assert_eq!(e.proyectada(car).encargo, None, "sin encargo no hay nada que contar");
    let (carga, descarga, t0) = (e.tiempo_carga(), e.tiempo_descarga(), e.n.ahora());
    let en_origen = Parada { desechar_sobrante: true, ..parada(&[(Cobre, 100_000)], &[Estano]) };
    let en_destino = Parada { esperar_existencias: true, ..parada(&[(Estano, 50_000)], &[Cobre]) };
    e.admin(encargar(car, b, Modo::Ruta, None, en_origen, en_destino));

    // Recién dado: esperando en el origen, cargando; las dos paradas, campo a campo y con el id de catálogo de cada recurso.
    let k = e.proyectada(car);
    assert_eq!(k.estado, pb::EstadoCaravana::CaravanaEsperando as i32);
    let x = k.encargo.expect("la caravana propia lleva su encargo");
    assert_eq!((x.modo.as_str(), x.destino_asentamiento_id.as_str(), x.salir_en_ms, x.vueltas, x.cancelado), ("ruta", ids::asentamiento(b).as_str(), None, 0, false));
    assert_eq!(
        x.parada_origen,
        Some(pb::ParadaEncargo { recoger: vec![texto_recoger("cobre", 100_000, None)], esperar_existencias: false, dejar: vec![texto_dejar("estano", None, None)], desechar_sobrante: true })
    );
    assert_eq!(
        x.parada_destino,
        Some(pb::ParadaEncargo { recoger: vec![texto_recoger("estano", 50_000, None)], esperar_existencias: true, dejar: vec![texto_dejar("cobre", None, None)], desechar_sobrante: false })
    );
    assert_eq!((x.parada_actual.as_str(), x.fase.as_str(), x.fin_paso_ms), ("origen", "carga", Some(t0 + carga)));

    // Sale hacia el destino.
    e.n.avanzar_hasta(t0 + carga);
    let k = e.proyectada(car);
    assert_eq!((k.estado, k.encargo.expect("sigue con su encargo").parada_actual.as_str()), (pb::EstadoCaravana::CaravanaEnTransito as i32, "destino"));

    // Llega: descarga en el destino.
    let llegada = e.viajar(car);
    let x = e.proyectada(car).encargo.unwrap();
    assert_eq!((x.parada_actual.as_str(), x.fase.as_str(), x.fin_paso_ms), ("destino", "descarga", Some(llegada + descarga)));
    e.n.avanzar_hasta(llegada + descarga);
    let x = e.proyectada(car).encargo.unwrap();
    assert_eq!((x.parada_actual.as_str(), x.fase.as_str(), x.fin_paso_ms), ("destino", "carga", Some(llegada + descarga + carga)));

    // Vuelve al origen: suma una vuelta y descarga allí.
    e.n.avanzar_hasta(llegada + descarga + carga);
    let k = e.proyectada(car);
    assert_eq!((k.estado, k.encargo.expect("sigue con su encargo").parada_actual.as_str()), (pb::EstadoCaravana::CaravanaRetornando as i32, "origen"));
    e.viajar(car);
    let x = e.proyectada(car).encargo.unwrap();
    assert_eq!((x.vueltas, x.parada_actual.as_str(), x.fase.as_str()), (1, "origen", "descarga"));

    // Cancelada en ruta, lo dice.
    e.n.avanzar_hasta(e.n.ahora() + descarga + carga);
    assert_eq!(e.estado(car), EstadoCaravana::EnTransito);
    e.admin(Comando::CancelarEncargo { caravana: car });
    assert!(e.proyectada(car).encargo.unwrap().cancelado);
}

#[test]
fn un_envio_programado_a_una_plaza_ajena_lleva_su_hora_sus_precios_y_la_fase_de_espera() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let a = e.a;
    let c = con_ajena(&mut e, m);
    e.dar(a, Cobre, 100_000);
    let car = e.caravana(a);
    let salida = e.n.ahora() + 30 * MIN;
    let en_c = Parada {
        recoger: vec![LineaRecoger { recurso: Estano, cantidad: 10_000, precio_max: Some(3_000_000) }],
        dejar: vec![LineaDejar { recurso: Cobre, tope: Some(5_000), precio_min: Some(1_000_000) }],
        ..Parada::default()
    };
    e.admin(encargar(car, c, Modo::Envio, Some(salida), parada(&[(Cobre, 50_000)], &[Cobre]), en_c));
    let x = e.proyectada(car).encargo.expect("el envío");
    assert_eq!((x.modo.as_str(), x.destino_asentamiento_id.as_str(), x.salir_en_ms), ("envio", ids::asentamiento(c).as_str(), Some(salida)));
    assert_eq!((x.parada_actual.as_str(), x.fase.as_str(), x.fin_paso_ms), ("origen", "espera", None), "espera su hora");
    assert_eq!(
        x.parada_destino,
        Some(pb::ParadaEncargo {
            recoger: vec![texto_recoger("estano", 10_000, Some(3_000_000))],
            esperar_existencias: false,
            dejar: vec![texto_dejar("cobre", Some(5_000), Some(1_000_000))],
            desechar_sobrante: false,
        })
    );
}

// ───────────────────────────── El mapa ─────────────────────────────

fn todo() -> Rect {
    Rect { min: Punto::new(-1_000_000, -1_000_000), max: Punto::new(10_000_000, 10_000_000) }
}

/// Lo que la Facción `f` tiene en su vista: cada entidad con su cuerpo.
fn vista_de(n: &Nucleo, f: u32) -> BTreeMap<(i32, String), Cuerpo> {
    Mirada::de(n, Perspectiva::Faccion(f)).contenido(None, todo(), &BTreeMap::new()).into_iter().filter_map(|v| Some((v.clave.clone(), v.cuerpo()?))).collect()
}

#[test]
fn una_caravana_esperando_no_sale_en_el_mapa_de_nadie_y_en_transito_si_pero_su_faccion_la_recibe_entera() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    let _ = con_ajena(&mut e, m);
    e.n.seguir_pertenencias();
    e.dar(a, Cobre, 100_000);
    let car = e.caravana(a);
    let clave = (TipoEntidad::Caravana as i32, ids::caravana(car));
    // Una columna de la Facción 2 sobre la plaza de origen: sus ojos alcanzan a la caravana que espera allí.
    let t = e.n.ahora();
    e.n.crear_ejercito(t, "espia", 2, pos(&e.n, a), 500, 100_000).unwrap();
    assert!(vista_de(&e.n, 2).contains_key(&(TipoEntidad::Asentamiento as i32, ids::asentamiento(a))), "los ojos de la Facción 2 están sobre la plaza");
    assert!(!vista_de(&e.n, 1).contains_key(&clave) && !vista_de(&e.n, 2).contains_key(&clave), "una `Disponible` está en casa");

    e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 100_000)], &[]), parada(&[], &[Cobre])));
    assert_eq!(e.estado(car), EstadoCaravana::Esperando);
    let (propia, ajena) = (vista_de(&e.n, 1), vista_de(&e.n, 2));
    assert!(!propia.contains_key(&clave), "esperando en el origen no sale en el mapa de su Facción");
    assert!(!ajena.contains_key(&clave), "ni en el de otra Facción con ojos sobre la plaza");
    // Pero su Facción la tiene entera donde tiene sus caravanas: el ámbito `FACCION`, que reparte por pertenencias.
    assert!(e.n.entidades_de(Ambito::Faccion(1)).contains(&(Clase::Caravana, car)));
    assert!(!e.n.entidades_de(Ambito::Faccion(2)).contains(&(Clase::Caravana, car)));
    let k = e.proyectada(car);
    assert_eq!(k.estado, pb::EstadoCaravana::CaravanaEsperando as i32);
    assert!(k.encargo.is_some());

    // En tránsito sí sale: entera para la suya y avistada, sin encargo, para quien la ve.
    let t = e.n.ahora() + e.tiempo_carga();
    e.n.avanzar_hasta(t);
    assert_eq!(e.estado(car), EstadoCaravana::EnTransito);
    let donde = e.n.posicion_de_caravana(car, t).expect("en marcha");
    e.n.crear_ejercito(t, "espia2", 2, donde, 500, 100_000).unwrap();
    let (propia, ajena) = (vista_de(&e.n, 1), vista_de(&e.n, 2));
    let Some(Cuerpo::Caravana(k)) = propia.get(&clave) else { panic!("en tránsito sale en el mapa de su Facción") };
    assert!(k.encargo.is_some());
    let Some(Cuerpo::Caravana(k)) = ajena.get(&clave) else { panic!("y en el de quien la ve") };
    assert_eq!(k.encargo, None, "la proyección ajena no lleva el encargo");

    // Llega al destino y vuelve a no salir.
    e.viajar(car);
    assert!(!vista_de(&e.n, 1).contains_key(&clave));
}

// ───────────────────────────── Las órdenes de una plaza ajena ─────────────────────────────

/// Una plaza de la Facción 2 con su Mercado activo y su oro, para comerciar con sus órdenes (rutas de caravanas §6).
fn ajena_con_mercado(e: &mut Escenario, m: &M) -> u64 {
    let c = con_ajena(e, m);
    e.admin(Comando::FijarNivelFaccion { faccion: 2, nivel: 2 });
    e.admin(Comando::AlternarAutoConstruccion { asentamiento: c, pausada: true });
    e.dar(c, Madera, 350_000);
    e.admin(Comando::AnadirEdificio { asentamiento: c, tipo: Tipo::Mercado });
    e.avanzar(9 * HORA);
    assert!(e.n.economia(c).unwrap().edificios.iter().any(|x| x.tipo == Tipo::Mercado && x.activo()), "el Mercado de as-{c} debería estar activo");
    e.dar(c, Oro, 400_000);
    c
}

/// Coloca en la plaza `plaza` (de la Facción 2) una orden de venta o de compra de `cantidad` del recurso a `precio` por unidad.
fn orden(e: &mut Escenario, plaza: u64, venta: bool, recurso: Recurso, cantidad: i64, precio: i64) -> u64 {
    e.admin(Comando::ColocarOrdenMercado { asentamiento: plaza, venta, recurso, cantidad, precio: Some(precio) }).creadas[0]
}

/// Los ids de las órdenes de mercado que la Facción `f` tiene en su vista con el rectángulo `rect`.
fn ordenes_en_vista(n: &Nucleo, f: u32, rect: Rect) -> Vec<String> {
    let mirada = Mirada::de(n, Perspectiva::Faccion(f));
    let v = mirada.contenido(None, rect, &BTreeMap::new());
    v.into_iter().filter(|x| x.clave.0 == TipoEntidad::OrdenMercado as i32).map(|x| x.clave.1).collect()
}

#[test]
fn un_envio_que_espera_en_una_plaza_ajena_hace_llegar_a_su_faccion_las_ordenes_activas_de_esa_plaza_hasta_que_se_va() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let a = e.a;
    let c = ajena_con_mercado(&mut e, m);
    e.dar(a, Cobre, 100_000);
    // La plaza compra el cobre que trae la caravana (se cumple) y vende madera (queda activa mientras la caravana espera).
    let compra_cobre = orden(&mut e, c, false, Cobre, 100_000, PUNTO / 2);
    let venta_madera = ids::orden_de_mercado(orden(&mut e, c, true, Madera, 100_000, PUNTO));
    let car = e.caravana(a);
    let en_destino = Parada { dejar: vec![LineaDejar { recurso: Cobre, tope: None, precio_min: Some(PUNTO / 2) }], ..Parada::default() };
    // Un rectángulo que no cubre la plaza: la regla no depende de él.
    let lejos = Rect { min: Punto::new(0, 0), max: Punto::new(1, 1) };
    assert!(!lejos.contiene(pos(&e.n, c)));

    e.admin(encargar(car, c, Modo::Envio, None, parada(&[(Cobre, 100_000)], &[]), en_destino));
    let t = e.n.ahora() + e.tiempo_carga();
    e.n.avanzar_hasta(t);
    assert_eq!(e.estado(car), EstadoCaravana::EnTransito);
    for f in [1, 3] {
        assert!(ordenes_en_vista(&e.n, f, todo()).is_empty(), "antes de llegar, la Facción {f} no recibe las órdenes");
    }

    // Llega y espera dentro mientras descarga: vende a la orden de compra y su Facción recibe las activas, con cualquier rectángulo.
    e.viajar(car);
    assert_eq!(e.n.caravana(car).unwrap().encargo.as_ref().map(|x| x.fase), Some(Fase::Descarga));
    assert!(e.n.libro_ordenes().buscar(compra_cobre).is_some_and(|o| o.estado == EstadoOrden::Cumplida), "vendió a la orden de compra");
    for rect in [todo(), lejos] {
        let propias = ordenes_en_vista(&e.n, 1, rect);
        assert_eq!(propias, vec![venta_madera.clone()], "solo las activas de la plaza ajena");
    }
    assert!(ordenes_en_vista(&e.n, 3, todo()).is_empty(), "una Facción sin nada dentro no las recibe");

    // Sale de vuelta: dejan de llegar.
    let fin = e.n.caravana(car).unwrap().encargo.as_ref().and_then(|x| x.fin_paso).expect("descarga en curso");
    e.n.avanzar_hasta(fin);
    assert_eq!(e.estado(car), EstadoCaravana::Retornando);
    for f in [1, 3] {
        assert!(ordenes_en_vista(&e.n, f, todo()).is_empty() && ordenes_en_vista(&e.n, f, lejos).is_empty(), "al irse la caravana, la Facción {f} no las recibe");
    }
}
