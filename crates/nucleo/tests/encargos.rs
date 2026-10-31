//! Los encargos de caravana: rutas y envíos automáticos entre plazas (Comercio_Caravanas §5.8; `Rutas_De_Caravanas.md` §3 y §4). Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está. Cada prueba mide una regla: el ciclo (llega, deja, recoge y sale), los tiempos por carro, la espera de
//! existencias, el sobrante, los peajes, cancelar y cambiar, el envío a una plaza ajena (sin Mercado, y comerciando con sus órdenes), los rechazos, quién da el encargo, la rejilla y la foto.

mod comun;

use asentamiento::{Cargo, Recurso::{self, *}};
use asentamiento::datos::UNO;
use asentamiento::catalogo::Tipo;
use base::{Instante, Punto, MS_POR_MINUTO};
use comercio::caravana::{Animal, Caravana, EstadoCaravana, TipoCarro};
use comercio::encargo::{Encargo, Fase, LineaDejar, LineaRecoger, Modo, Parada, Punta};
use comercio::orden_mercado::EstadoOrden;
use comercio::precio::{comision_de_entrega, ContextoComision};
use comercio::Carga as Mercancia;
use comun::real::{fundar, sitio_rico, M};
use militar::catalogo::Tropa;
use militar::escuadra::Contenedor;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::{rechazos, Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Resultado, Respuesta};
use rutas::Opciones;

const PUNTO: i64 = UNO;
const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;
const DIA: i64 = 24 * HORA;

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
    e.asentamientos.values().map(|a| a.economia.almacen.cantidad(r)).sum::<i64>() + e.comercio.flota.iter().map(|c| c.contenido.get(&r).copied().unwrap_or(0)).sum::<i64>()
}

fn cantidad(n: &Nucleo, plaza: u64, r: Recurso) -> i64 {
    n.economia(plaza).unwrap().almacen.cantidad(r)
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

fn cambiar(caravana: u64, destino: u64, salir_en: Option<Instante>, en_origen: Parada, en_destino: Parada) -> Comando {
    Comando::CambiarEncargo { caravana, destino, salir_en, en_origen, en_destino }
}

/// Dos plazas de la Facción 1 con Mercado (`a` y `b`, de campamentos distintos), con el tiempo parado en el instante en que el Mercado de `a` ya está activo.
struct Escenario {
    n: Nucleo,
    a: u64,
    b: u64,
    cuenta: u64,
}

impl Escenario {
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
    /// Una orden que se rechaza, sin tocar el estado: ni su hash ni su versión cambian.
    fn rechazada(&mut self, actor: &str, c: Comando) -> &'static str {
        let (hash, version) = (self.n.hash_estado(), self.n.version());
        let r = self.ord(actor, c);
        let codigo = codigo(&r);
        assert_eq!((self.n.hash_estado(), self.n.version()), (hash, version), "un rechazo ({codigo}) no cambia el estado");
        codigo
    }
    fn admin(&mut self, c: Comando) -> Respuesta {
        self.ok("admin", c)
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
    /// Una caravana de `carros` carros con buey en la plaza.
    fn caravana(&mut self, plaza: u64, carros: u32) -> u64 {
        let id = self.admin(Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
        for k in 0..carros {
            self.admin(Comando::AnadirCarro { caravana: id, tipo: TipoCarro::Basico });
            self.admin(Comando::AnadirAnimal { caravana: id, carro: k, animal: Animal::Buey });
        }
        id
    }
    fn k(&self, id: u64) -> &Caravana {
        self.n.caravana(id).expect("existe la caravana")
    }
    fn encargo(&self, id: u64) -> &Encargo {
        self.k(id).encargo.as_ref().expect("tiene encargo")
    }
    fn a_bordo(&self, id: u64, r: Recurso) -> i64 {
        self.k(id).contenido.get(&r).copied().unwrap_or(0)
    }
    fn fin_del_plan(&self, id: u64) -> Instante {
        self.k(id).plan.as_ref().and_then(|p| p.fin()).expect("la caravana tiene plan")
    }
    fn tiempo_carga(&self) -> i64 {
        self.n.estado().reglas.economia.comercio.encargo.tiempo_carga
    }
    fn tiempo_descarga(&self) -> i64 {
        self.n.estado().reglas.economia.comercio.encargo.tiempo_descarga
    }
    /// (estado, punta, fase) de la caravana con encargo.
    fn situacion(&self, id: u64) -> (EstadoCaravana, Punta, Fase) {
        let e = self.encargo(id);
        (self.k(id).estado, e.punta, e.fase)
    }
    /// Avanza hasta justo antes del fin del plan (sigue en camino) y hasta el fin (llegó). Devuelve el instante de llegada.
    fn viajar(&mut self, id: u64) -> Instante {
        assert!(matches!(self.k(id).estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando), "no va de camino: {:?}", self.k(id).estado);
        let fin = self.fin_del_plan(id);
        self.avanzar_hasta(fin - 1);
        assert!(matches!(self.k(id).estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando), "aún no ha llegado");
        self.avanzar_hasta(fin);
        assert_eq!(self.k(id).estado, EstadoCaravana::Esperando, "llegó a su parada");
        fin
    }
    /// La caravana vuelve a su origen, descarga y el envío termina. Devuelve el instante en que termina.
    fn viajar_y_terminar(&mut self, id: u64) -> Instante {
        let llegada = self.viajar(id);
        let fin = llegada + self.tiempo_descarga();
        self.avanzar_hasta(fin);
        fin
    }
    /// Un héroe de la Facción 1, residente en `plaza` y dentro de ella.
    fn heroe_residente(&mut self, cuenta: &str, plaza: u64) -> u64 {
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0];
        self.admin(Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
        self.admin(Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
        self.admin(Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
        h
    }
}

/// Funda las dos plazas de la Facción 1 (`a` con el Rey de la Facción) y les construye el Mercado.
fn escenario_de(n: Nucleo, m: &M) -> Escenario {
    let mut n = n;
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

fn escenario(m: &M, semilla: u64) -> Escenario {
    escenario_de(Nucleo::nuevo(arranque(m, semilla)), m)
}

/// Funda una plaza de la Facción 2 junto al campamento 1: una plaza ajena a la que ir.
fn con_ajena(e: &mut Escenario, m: &M) -> u64 {
    let p = sitio_rico(&e.n, &m.0, 1, 2);
    fundar(&mut e.n, "beto", 2, p)
}

/// Funda en el trayecto de A a B una plaza de la Facción 3 y la deja en el camino.
fn plaza_en_el_camino(e: &mut Escenario, m: &M) -> u64 {
    let (pa, pb) = (pos(&e.n, e.a), pos(&e.n, e.b));
    let camino = m.1.ruta(pa, pb, &Opciones::default()).unwrap();
    let (desde, hasta) = (camino.puntos.len() * 35 / 100, camino.puntos.len() * 65 / 100);
    let p = camino.puntos[desde..hasta]
        .iter()
        .copied()
        .find(|&p| p.dist(pa) > 40_000 && p.dist(pb) > 40_000 && e.n.viabilidad(p, 3).is_some_and(|v| v.motivo.is_none()))
        .expect("un sitio fundable en el camino");
    fundar(&mut e.n, "carlos", 3, p)
}

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
    e.ok("admin", Comando::ColocarOrdenMercado { asentamiento: plaza, venta, recurso, cantidad, precio: Some(precio) }).creadas[0]
}

/// Una línea de la parada de una plaza ajena: deja `recurso` (hasta `tope`) a precio mínimo `precio_min`.
fn vende(recurso: Recurso, tope: Option<i64>, precio_min: i64) -> LineaDejar {
    LineaDejar { recurso, tope, precio_min: Some(precio_min) }
}

/// Una línea de la parada de una plaza ajena: recoge `cantidad` de `recurso` a precio máximo `precio_max`.
fn compra(recurso: Recurso, cantidad: i64, precio_max: i64) -> LineaRecoger {
    LineaRecoger { recurso, cantidad, precio_max: Some(precio_max) }
}

/// Lanza ya un envío de `car` a `destino`, espera a que cargue en el origen y llega: devuelve el instante de llegada, con el destino ya tratado hasta ese instante.
fn enviar_y_llegar(e: &mut Escenario, car: u64, destino: u64, en_origen: Parada, en_destino: Parada) -> Instante {
    e.admin(encargar(car, destino, Modo::Envio, None, en_origen, en_destino));
    e.avanzar_hasta(e.encargo(car).fin_paso.expect("carga en el origen"));
    e.viajar(car)
}

/// Los hechos `mercado.comercio_en_plaza` de la caravana, en orden: (orden, cantidad).
fn servidas(e: &Escenario, car: u64) -> Vec<(i64, i64)> {
    let todos = hechos(&e.n, "mercado.comercio_en_plaza");
    assert!(todos.iter().all(|h| dato(h, "entreFacciones") == 1 && dato(h, "caravana") == car as i64 && h.2.iter().all(|(k, _)| *k != "ejercito")), "{todos:?}");
    todos.iter().map(|h| (dato(h, "orden"), dato(h, "cantidad"))).collect()
}

/// La comisión que debe acuñar la entrega de `carga` de `origen` a `destino` con el estado de ahora.
fn comision_esperada(n: &Nucleo, origen: u64, destino: u64, carga: &[(Recurso, i64)]) -> i64 {
    let r = comercio::Reglas::cargar();
    let e = n.estado();
    let precios = n.tabla_de_precios();
    let c: Mercancia = carga.iter().copied().collect();
    let (po, pd) = (&e.asentamientos[&origen], &e.asentamientos[&destino]);
    comision_de_entrega(&r.comision, precios.valor(&c), &ContextoComision { mismo_asentamiento: false, misma_faccion: po.faccion == pd.faccion, factor_comision_externa: UNO, distancia_recta: po.posicion.dist(pd.posicion) })
}

// ───────────────────────────── 1. El ciclo ─────────────────────────────

#[test]
fn una_ruta_entre_dos_plazas_propias_da_varias_vueltas_sin_comision() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 300_000);
    e.dar(b, Estano, 300_000);
    let car = e.caravana(a, 1);
    let (cobre, estano) = (mundo_de(&e.n, Cobre), mundo_de(&e.n, Estano));
    let (cobre_a, cobre_b, estano_a, estano_b) = (cantidad(&e.n, a, Cobre), cantidad(&e.n, b, Cobre), cantidad(&e.n, a, Estano), cantidad(&e.n, b, Estano));
    let (carga, descarga, t0) = (e.tiempo_carga(), e.tiempo_descarga(), e.n.ahora());

    e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 100_000)], &[Estano]), parada(&[(Estano, 50_000)], &[Cobre])));
    // Sin hora de salida empieza ya a cargar en el origen: toma del almacén y sube a bordo; sale al acabar el tiempo de carga de su único carro.
    assert_eq!(e.situacion(car), (EstadoCaravana::Esperando, Punta::Origen, Fase::Carga));
    assert_eq!((e.encargo(car).fin_paso, e.a_bordo(car, Cobre), cantidad(&e.n, a, Cobre)), (Some(t0 + carga), 100_000, cobre_a - 100_000));
    let dado = hechos(&e.n, "comercio.encargo_dado");
    assert_eq!((dado.len(), dato(&dado[0], "caravana"), dato(&dado[0], "destino"), dato(&dado[0], "modo")), (1, car as i64, b as i64, 0));
    let h = &hechos(&e.n, "comercio.caravana_carga")[0];
    assert_eq!((h.0, h.1.clone(), dato(h, "cobre"), dato(h, "carros"), dato(h, "finPaso")), (t0, vec![a], 100_000, 1, t0 + carga));
    e.avanzar_hasta(t0 + carga - 1);
    assert_eq!(e.k(car).estado, EstadoCaravana::Esperando);
    e.avanzar_hasta(t0 + carga);
    assert_eq!((e.k(car).estado, e.encargo(car).punta, e.k(car).destino), (EstadoCaravana::EnTransito, Punta::Destino, Some(b as u32)));
    assert_eq!(hechos(&e.n, "comercio.caravana_sale").len(), 1);

    // Llega al destino: deja el cobre, recoge estaño y sale de vuelta cargada.
    let llegada = e.viajar(car);
    assert_eq!(e.situacion(car), (EstadoCaravana::Esperando, Punta::Destino, Fase::Descarga));
    assert_eq!((e.encargo(car).fin_paso, e.a_bordo(car, Cobre), cantidad(&e.n, b, Cobre) - cobre_b), (Some(llegada + descarga), 0, 100_000));
    let h = &hechos(&e.n, "comercio.caravana_descarga")[0];
    assert_eq!((h.0, h.1.clone(), dato(h, "cobre"), dato(h, "tirado"), dato(h, "aBordo"), dato(h, "carros"), dato(h, "finPaso"), dato(h, "comision")), (llegada, vec![b], 100_000, 0, 0, 1, llegada + descarga, 0));
    e.avanzar_hasta(llegada + descarga);
    assert_eq!((e.situacion(car).2, e.a_bordo(car, Estano), cantidad(&e.n, b, Estano)), (Fase::Carga, 50_000, estano_b - 50_000));
    e.avanzar_hasta(llegada + descarga + carga);
    assert_eq!((e.k(car).estado, e.encargo(car).punta, e.encargo(car).vueltas), (EstadoCaravana::Retornando, Punta::Origen, 0));

    // Vuelve al origen: suma una vuelta, deja el estaño y recoge cobre otra vez.
    let vuelta = e.viajar(car);
    assert_eq!((e.encargo(car).vueltas, e.situacion(car)), (1, (EstadoCaravana::Esperando, Punta::Origen, Fase::Descarga)));
    assert_eq!((e.a_bordo(car, Estano), cantidad(&e.n, a, Estano) - estano_a), (0, 50_000));
    e.avanzar_hasta(vuelta + descarga);
    assert_eq!((e.situacion(car).2, e.a_bordo(car, Cobre)), (Fase::Carga, 100_000));
    e.avanzar_hasta(vuelta + descarga + carga);
    assert_eq!((e.k(car).estado, e.encargo(car).vueltas), (EstadoCaravana::EnTransito, 1));

    // Una segunda vuelta entera, ya por el camino que abrió la primera.
    e.viajar(car);
    e.avanzar_hasta(e.n.ahora() + descarga + carga);
    e.viajar(car);
    assert_eq!(e.encargo(car).vueltas, 2);
    assert_eq!(cantidad(&e.n, b, Cobre) - cobre_b, 200_000, "en cada ida deja 100 de cobre");
    // Ni se acuña ni se pierde nada, y ninguna descarga cobró comisión.
    let carga_total = e.a_bordo(car, Cobre) + e.a_bordo(car, Estano);
    assert_eq!((mundo_de(&e.n, Cobre), mundo_de(&e.n, Estano)), (cobre, estano), "{carga_total} a bordo");
    assert!(hechos(&e.n, "comercio.caravana_descarga").iter().all(|h| dato(h, "comision") == 0));
    assert!(hechos(&e.n, "comercio.caravana_llega").is_empty());
}

#[test]
fn un_envio_programado_a_otra_plaza_propia_espera_su_hora_deja_con_comision_vuelve_y_termina_cumplido() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    e.dar(b, Estano, 100_000);
    let car = e.caravana(a, 1);
    // Una escolta cedida antes del encargo viaja con ella.
    let carla = e.heroe_residente("carla", a);
    let milicia = e.admin(Comando::DarEscuadra { heroe: carla, tropa: Tropa::MiliciaLanceros, cantidad: 25 }).creadas[0];
    e.ok("carla", Comando::AsignarEscolta { caravana: car, heroe: carla, escuadras: vec![milicia] });
    let contenedor = |e: &Escenario| e.n.heroe(carla).unwrap().heroe.escuadra(milicia).unwrap().contenedor;
    assert!(matches!(contenedor(&e), Contenedor::Escolta(_)));
    let (cobre_a, cobre_b, estano_a) = (cantidad(&e.n, a, Cobre), cantidad(&e.n, b, Cobre), cantidad(&e.n, a, Estano));
    let (carga, descarga, salir) = (e.tiempo_carga(), e.tiempo_descarga(), e.n.ahora() + 2 * HORA);

    e.admin(encargar(car, b, Modo::Envio, Some(salir), parada(&[(Cobre, 50_000)], &[Estano]), parada(&[(Estano, 20_000)], &[Cobre])));
    // Espera su hora: aún no ha tomado nada del almacén.
    assert_eq!((e.situacion(car), e.encargo(car).fin_paso, e.k(car).contenido.is_empty()), ((EstadoCaravana::Esperando, Punta::Origen, Fase::Espera), None, true));
    assert_eq!((cantidad(&e.n, a, Cobre), e.k(car).escolta.len()), (cobre_a, 1));
    assert_eq!(hechos(&e.n, "comercio.encargo_dado").iter().map(|h| dato(h, "modo")).collect::<Vec<_>>(), vec![1]);
    e.avanzar_hasta(salir - 1);
    assert_eq!((e.situacion(car).2, e.k(car).contenido.is_empty()), (Fase::Espera, true));
    e.avanzar_hasta(salir);
    assert_eq!((e.situacion(car).2, e.a_bordo(car, Cobre), e.encargo(car).fin_paso), (Fase::Carga, 50_000, Some(salir + carga)));
    e.avanzar_hasta(salir + carga);
    assert_eq!(e.k(car).estado, EstadoCaravana::EnTransito);

    // En el destino deja con la comisión de una entrega.
    let llegada = e.fin_del_plan(car);
    e.avanzar_hasta(llegada - 1);
    let (esperada, oro_b) = (comision_esperada(&e.n, a, b, &[(Cobre, 50_000)]), cantidad(&e.n, b, Oro));
    e.avanzar_hasta(llegada);
    let h = hechos(&e.n, "comercio.caravana_descarga").into_iter().find(|h| h.1 == vec![b]).expect("descarga en el destino");
    let comision = dato(&h, "comision");
    assert!(comision > 0 && comision <= esperada && esperada - comision < 1_000, "comisión {comision} contra {esperada}: se acuña en unidades enteras, hacia abajo");
    assert_eq!(cantidad(&e.n, b, Cobre) - cobre_b, 50_000);
    assert!((cantidad(&e.n, b, Oro) - oro_b - comision).abs() <= 1_000, "la comisión se acuña en el destino");
    e.avanzar_hasta(llegada + descarga + carga);
    assert_eq!((e.k(car).estado, e.a_bordo(car, Estano)), (EstadoCaravana::Retornando, 20_000));

    // Vuelve, deja y termina cumplido: queda disponible y la escolta vuelve al campamento.
    let vuelta = e.viajar(car);
    assert_eq!(e.situacion(car), (EstadoCaravana::Esperando, Punta::Origen, Fase::Descarga));
    assert_eq!(e.encargo(car).fin_paso, Some(vuelta + descarga));
    e.avanzar_hasta(vuelta + descarga - 1);
    assert_eq!(e.k(car).estado, EstadoCaravana::Esperando);
    e.avanzar_hasta(vuelta + descarga);
    let k = e.k(car);
    assert_eq!((k.estado, k.encargo.is_none(), k.contenido.is_empty(), k.escolta.is_empty(), k.destino), (EstadoCaravana::Disponible, true, true, true, None));
    assert_eq!(contenedor(&e), Contenedor::Campamento);
    assert_eq!(cantidad(&e.n, a, Estano) - estano_a, 20_000);
    let fin = hechos(&e.n, "comercio.encargo_terminado");
    assert_eq!((fin.len(), fin[0].0, dato(&fin[0], "motivo"), dato(&fin[0], "volcado"), dato(&fin[0], "perdido")), (1, vuelta + descarga, 0, 0, 0));
    // Libre otra vez: no se mueve más.
    e.avanzar(HORA);
    assert_eq!((e.k(car).estado, hechos(&e.n, "comercio.caravana_sale").len()), (EstadoCaravana::Disponible, 2));
}

// ───────────────────────────── 2. Esperar existencias ─────────────────────────────

#[test]
fn esperar_existencias_espera_con_su_pulso_de_un_minuto_y_carga_en_cuanto_hay_todo() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Estano, 20_000);
    let car = e.caravana(a, 1);
    let hay = cantidad(&e.n, a, Estano);
    let t0 = e.n.ahora();
    let origen = Parada { esperar_existencias: true, ..parada(&[(Estano, hay + 30_000)], &[]) };
    e.admin(encargar(car, b, Modo::Ruta, None, origen, Parada::default()));
    // Espera sin tomar nada, y el hecho dice qué falta.
    assert_eq!((e.situacion(car), e.encargo(car).fin_paso, e.k(car).contenido.is_empty(), cantidad(&e.n, a, Estano)), ((EstadoCaravana::Esperando, Punta::Origen, Fase::Espera), None, true, hay));
    let espera = hechos(&e.n, "comercio.caravana_espera");
    assert_eq!((espera.len(), espera[0].0, espera[0].1.clone(), dato(&espera[0], "estano")), (1, t0, vec![a], 30_000));
    // Cada minuto repite la carga con el almacén de entonces, y el hecho no se repite.
    e.avanzar_hasta(t0 + 4 * MIN + MIN / 2);
    assert_eq!((e.situacion(car).2, hechos(&e.n, "comercio.caravana_espera").len()), (Fase::Espera, 1));
    e.dar(a, Estano, 30_000);
    e.avanzar_hasta(t0 + 5 * MIN - 1);
    assert_eq!((e.situacion(car).2, e.k(car).contenido.is_empty()), (Fase::Espera, true), "no carga hasta su pulso");
    e.avanzar_hasta(t0 + 5 * MIN);
    assert_eq!((e.situacion(car).2, e.a_bordo(car, Estano), cantidad(&e.n, a, Estano)), (Fase::Carga, hay + 30_000, 0));
    let carga = hechos(&e.n, "comercio.caravana_carga");
    assert_eq!((carga.len(), carga[0].0, dato(&carga[0], "estano")), (1, t0 + 5 * MIN, hay + 30_000));
    // Y ya no hay más pulsos: el siguiente tramo sale a su hora.
    e.avanzar_hasta(t0 + 5 * MIN + e.tiempo_carga());
    assert_eq!(e.k(car).estado, EstadoCaravana::EnTransito);
}

#[test]
fn sin_esperar_existencias_sale_con_lo_que_haya() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Estano, 20_000);
    let car = e.caravana(a, 1);
    let hay = cantidad(&e.n, a, Estano);
    let t0 = e.n.ahora();
    e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Estano, hay + 30_000)], &[]), Parada::default()));
    assert_eq!((e.situacion(car), e.a_bordo(car, Estano), cantidad(&e.n, a, Estano)), ((EstadoCaravana::Esperando, Punta::Origen, Fase::Carga), hay, 0));
    assert!(hechos(&e.n, "comercio.caravana_espera").is_empty());
    e.avanzar_hasta(t0 + e.tiempo_carga());
    assert_eq!((e.k(car).estado, e.a_bordo(car, Estano)), (EstadoCaravana::EnTransito, hay));
}

// ───────────────────────────── 3. Tiempos por carro ─────────────────────────────

#[test]
fn dos_carros_que_mueven_algo_tardan_el_doble_que_uno_y_una_parada_sin_nada_que_mover_no_tarda() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 350_000);
    e.dar(a, Estano, 350_000);
    let car = e.caravana(a, 2);
    let (carga, descarga, t0) = (e.tiempo_carga(), e.tiempo_descarga(), e.n.ahora());
    // Cada carro con buey carga 500: 100 de cobre ocupan un carro y 600 (de dos recursos: un almacén no cabe tanto de uno) los dos.
    e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 100_000)], &[]), Parada::default()));
    let h = &hechos(&e.n, "comercio.caravana_carga")[0];
    assert_eq!((dato(h, "carros"), e.encargo(car).fin_paso), (1, Some(t0 + carga)));
    e.admin(Comando::CancelarEncargo { caravana: car });
    e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 300_000), (Estano, 300_000)], &[]), parada(&[], &[Cobre, Estano])));
    let h = &hechos(&e.n, "comercio.caravana_carga")[1];
    assert_eq!((dato(h, "carros"), e.encargo(car).fin_paso), (2, Some(t0 + 2 * carga)), "dos carros: el doble de tiempo");
    e.avanzar_hasta(t0 + 2 * carga);
    // La descarga de los 600 también ocupa los dos carros; luego no recoge nada y sale en el mismo instante en que acaba.
    let llegada = e.viajar(car);
    let h = hechos(&e.n, "comercio.caravana_descarga").into_iter().find(|h| h.1 == vec![b]).unwrap();
    assert_eq!((dato(&h, "carros"), dato(&h, "finPaso"), e.encargo(car).fin_paso), (2, llegada + 2 * descarga, Some(llegada + 2 * descarga)));
    e.avanzar_hasta(llegada + 2 * descarga);
    assert_eq!((e.k(car).estado, e.k(car).contenido.is_empty()), (EstadoCaravana::Retornando, true), "recoger nada no tarda");
    assert!(hechos(&e.n, "comercio.caravana_carga").iter().all(|h| h.1 != vec![b]));
    // En el origen no deja nada: ni espera ni descarga; pasa derecho a cargar lo que quedaba (50 de cada recurso, un carro).
    let vuelta = e.viajar(car);
    let h = hechos(&e.n, "comercio.caravana_descarga").into_iter().find(|h| h.1 == vec![a]).unwrap();
    assert_eq!((h.0, dato(&h, "carros"), dato(&h, "finPaso")), (vuelta, 0, 0));
    assert_eq!((e.situacion(car).2, e.encargo(car).fin_paso, e.a_bordo(car, Cobre)), (Fase::Carga, Some(vuelta + carga), 50_000));
    assert_eq!(e.a_bordo(car, Estano), 50_000);
}

// ───────────────────────────── 4. El sobrante ─────────────────────────────

#[test]
fn desechar_sobrante_con_el_almacen_lleno_tira_y_sin_desechar_se_queda_a_bordo() {
    let m = con_mundo!();
    for desechar in [true, false] {
        let mut e = escenario(m, 7);
        let (a, b) = (e.a, e.b);
        e.dar(a, Cobre, 100_000);
        let hueco = e.n.economia(b).unwrap().almacen.espacio(Cobre);
        e.dar(b, Cobre, hueco - 30_000);
        let cobre = mundo_de(&e.n, Cobre);
        let car = e.caravana(a, 1);
        let destino = Parada { desechar_sobrante: desechar, ..parada(&[], &[Cobre]) };
        e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 100_000)], &[]), destino));
        let (cobre_b, cobre_mundo) = (cantidad(&e.n, b, Cobre), mundo_de(&e.n, Cobre));
        assert_eq!(cobre_mundo, cobre);
        e.avanzar(e.tiempo_carga());
        e.viajar(car);
        // Entran 30 (lo que cabe); el resto de 70 se tira o sigue a bordo.
        let h = hechos(&e.n, "comercio.caravana_descarga").into_iter().find(|h| h.1 == vec![b]).unwrap();
        assert_eq!((dato(&h, "cobre"), cantidad(&e.n, b, Cobre) - cobre_b), (30_000, 30_000));
        if desechar {
            assert_eq!((dato(&h, "tirado"), dato(&h, "aBordo"), e.a_bordo(car, Cobre)), (70_000, 0, 0));
            assert_eq!(mundo_de(&e.n, Cobre), cobre - 70_000, "lo tirado desaparece");
        } else {
            assert_eq!((dato(&h, "tirado"), dato(&h, "aBordo"), e.a_bordo(car, Cobre)), (0, 70_000, 70_000));
            assert_eq!(mundo_de(&e.n, Cobre), cobre, "lo que sigue a bordo no se pierde");
        }
        assert_eq!(dato(&h, "carros"), 1, "el sobrante también se mueve");
    }
}

// ───────────────────────────── 5. Peajes ─────────────────────────────

#[test]
fn una_plaza_ajena_en_el_camino_cobra_peaje_a_la_ida_y_a_la_vuelta_cargada() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    let c = plaza_en_el_camino(&mut e, m);
    e.dar(a, Cobre, 100_000);
    e.dar(b, Estano, 100_000);
    let car = e.caravana(a, 1);
    let (cobre_c, estano_c) = (cantidad(&e.n, c, Cobre), cantidad(&e.n, c, Estano));
    e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 100_000)], &[Estano]), parada(&[(Estano, 100_000)], &[Cobre])));
    e.avanzar(e.tiempo_carga());
    assert_eq!((e.k(car).estado, e.k(car).peajes.len(), e.k(car).peajes[0].ciudad), (EstadoCaravana::EnTransito, 1, c as u32), "la ruta pasa por la ciudad ajena");
    // A la ida cobra el 2 % de lo que lleva.
    e.viajar(car);
    let peajes = hechos(&e.n, "comercio.peaje_paso");
    assert_eq!((peajes.len(), peajes[0].1.clone(), dato(&peajes[0], "cobre")), (1, vec![c], 2_000));
    assert_eq!(cantidad(&e.n, c, Cobre) - cobre_c, 2_000);
    // Descarga lo que queda, recoge estaño y a la vuelta, cargada, paga otra vez.
    e.avanzar(e.tiempo_descarga() + e.tiempo_carga());
    assert_eq!((e.k(car).estado, e.a_bordo(car, Estano), e.k(car).peajes.len()), (EstadoCaravana::Retornando, 100_000, 1));
    e.viajar(car);
    let peajes = hechos(&e.n, "comercio.peaje_paso");
    assert_eq!((peajes.len(), peajes[1].1.clone(), dato(&peajes[1], "estano")), (2, vec![c], 2_000), "la vuelta cargada paga");
    assert_eq!(cantidad(&e.n, c, Estano) - estano_c, 2_000);
    let h = hechos(&e.n, "comercio.caravana_descarga").into_iter().find(|h| h.1 == vec![a]).unwrap();
    assert_eq!(dato(&h, "estano"), 98_000, "y deja en el origen lo que queda");
}

// ───────────────────────────── 6. Cancelar ─────────────────────────────

#[test]
fn cancelar_esperando_en_el_origen_termina_ya_y_la_carga_vuelve_al_almacen() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    let car = e.caravana(a, 1);
    let cobre = cantidad(&e.n, a, Cobre);
    let t0 = e.n.ahora();
    e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 100_000)], &[]), Parada::default()));
    assert_eq!((e.a_bordo(car, Cobre), cantidad(&e.n, a, Cobre)), (100_000, cobre - 100_000));
    e.avanzar(MIN);
    e.admin(Comando::CancelarEncargo { caravana: car });
    let k = e.k(car);
    assert_eq!((k.estado, k.encargo.is_none(), k.contenido.is_empty(), cantidad(&e.n, a, Cobre)), (EstadoCaravana::Disponible, true, true, cobre));
    let fin = hechos(&e.n, "comercio.encargo_terminado");
    assert_eq!((fin.len(), fin[0].0, dato(&fin[0], "motivo"), dato(&fin[0], "volcado"), dato(&fin[0], "perdido")), (1, t0 + MIN, 1, 100_000, 0));
    // Sus eventos pendientes se descartaron: no sale al acabar la carga.
    e.avanzar(HORA);
    assert_eq!((e.k(car).estado, hechos(&e.n, "comercio.caravana_sale").len()), (EstadoCaravana::Disponible, 0));
    // Ya no tiene encargo que cancelar.
    assert_eq!(e.rechazada("admin", Comando::CancelarEncargo { caravana: car }), rechazos::ENCARGO_NO_TIENE);
}


#[test]
fn cancelar_en_ruta_llega_sin_dejar_ni_recoger_vuelve_y_termina_en_el_origen() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    e.dar(b, Estano, 100_000);
    let car = e.caravana(a, 1);
    let (cobre_a, cobre_b, estano_b) = (cantidad(&e.n, a, Cobre), cantidad(&e.n, b, Cobre), cantidad(&e.n, b, Estano));
    e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 100_000)], &[]), parada(&[(Estano, 50_000)], &[Cobre])));
    e.avanzar(e.tiempo_carga());
    let mitad = e.n.ahora() + (e.fin_del_plan(car) - e.n.ahora()) / 2;
    e.avanzar_hasta(mitad);
    e.admin(Comando::CancelarEncargo { caravana: car });
    assert_eq!((e.k(car).estado, e.encargo(car).cancelado), (EstadoCaravana::EnTransito, true), "acaba el tramo en curso");
    let (hash, r) = (e.n.hash_estado(), e.ord("admin", Comando::CancelarEncargo { caravana: car }));
    assert_eq!((r.resultado, e.n.hash_estado()), (Resultado::SinCambios, hash), "cancelar lo ya cancelado no cambia nada");

    // Al llegar al destino no deja ni recoge: da la vuelta en el acto con lo que lleva.
    let llegada = e.fin_del_plan(car);
    e.avanzar_hasta(llegada);
    assert_eq!((e.k(car).estado, e.a_bordo(car, Cobre), cantidad(&e.n, b, Cobre), cantidad(&e.n, b, Estano)), (EstadoCaravana::Retornando, 100_000, cobre_b, estano_b));
    assert!(hechos(&e.n, "comercio.caravana_descarga").is_empty() && hechos(&e.n, "comercio.caravana_carga").iter().all(|h| h.1 != vec![b]));
    // Y al llegar al origen termina, y su carga vuelve al almacén.
    let vuelta = e.fin_del_plan(car);
    e.avanzar_hasta(vuelta);
    let k = e.k(car);
    assert_eq!((k.estado, k.encargo.is_none(), k.contenido.is_empty(), cantidad(&e.n, a, Cobre)), (EstadoCaravana::Disponible, true, true, cobre_a));
    let fin = hechos(&e.n, "comercio.encargo_terminado");
    assert_eq!((fin.len(), fin[0].0, dato(&fin[0], "motivo"), dato(&fin[0], "volcado")), (1, vuelta, 1, 100_000));
}

#[test]
fn cancelar_esperando_en_el_destino_sale_ya_de_vuelta_sin_dejar_ni_recoger_mas() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    let car = e.caravana(a, 1);
    let (cobre_a, cobre_b, hay) = (cantidad(&e.n, a, Cobre), cantidad(&e.n, b, Cobre), cantidad(&e.n, b, Estano));
    // En el destino no deja nada y espera un estaño que no hay.
    let destino = Parada { esperar_existencias: true, ..parada(&[(Estano, hay + 50_000)], &[]) };
    e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 100_000)], &[]), destino));
    e.avanzar(e.tiempo_carga());
    let llegada = e.viajar(car);
    assert_eq!((e.situacion(car), e.a_bordo(car, Cobre)), ((EstadoCaravana::Esperando, Punta::Destino, Fase::Espera), 100_000));
    e.avanzar_hasta(llegada + 3 * MIN);
    e.admin(Comando::CancelarEncargo { caravana: car });
    assert_eq!((e.k(car).estado, e.encargo(car).cancelado, e.encargo(car).punta, e.a_bordo(car, Cobre)), (EstadoCaravana::Retornando, true, Punta::Origen, 100_000));
    // Aunque ahora hubiera estaño, ya no espera: sus pulsos se descartaron.
    e.dar(b, Estano, 50_000);
    let vuelta = e.fin_del_plan(car);
    e.avanzar_hasta(vuelta);
    assert_eq!((e.k(car).estado, cantidad(&e.n, a, Cobre), cantidad(&e.n, b, Cobre)), (EstadoCaravana::Disponible, cobre_a, cobre_b));
    assert_eq!(cantidad(&e.n, b, Estano), hay + 50_000, "no recogió nada");
    let fin = hechos(&e.n, "comercio.encargo_terminado");
    assert_eq!((fin.len(), dato(&fin[0], "motivo"), dato(&fin[0], "volcado")), (1, 1, 100_000));
}

// ───────────────────────────── 7. Cambiar ─────────────────────────────

#[test]
fn cambiar_las_paradas_valen_desde_el_paso_siguiente_y_no_tocan_el_paso_en_curso() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 300_000);
    e.dar(b, Estano, 300_000);
    let car = e.caravana(a, 1);
    let t0 = e.n.ahora();
    e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 100_000)], &[]), Parada::default()));
    // Mientras carga cambia las dos paradas: la carga en curso sigue como era; lo nuevo vale desde el paso siguiente.
    e.avanzar(MIN);
    let antes = e.encargo(car).clone();
    e.admin(cambiar(car, b, None, parada(&[(Cobre, 20_000)], &[Estano]), parada(&[(Estano, 50_000)], &[Cobre])));
    let ahora = e.encargo(car);
    assert_eq!((ahora.vueltas, ahora.punta, ahora.fase, ahora.fin_paso, ahora.cancelado), (antes.vueltas, antes.punta, antes.fase, antes.fin_paso, antes.cancelado));
    assert_eq!((ahora.en_origen.recoger.len(), ahora.en_destino.dejar.len(), e.a_bordo(car, Cobre), ahora.fin_paso), (1, 1, 100_000, Some(t0 + e.tiempo_carga())));
    e.avanzar_hasta(t0 + e.tiempo_carga());
    e.viajar(car);
    // En el destino ya rige la parada nueva: deja el cobre y recoge estaño.
    assert_eq!(e.a_bordo(car, Cobre), 0);
    e.avanzar(e.tiempo_descarga());
    assert_eq!(e.a_bordo(car, Estano), 50_000);
    e.avanzar(e.tiempo_carga());
    // Y en el origen, la nueva: deja el estaño y recoge solo 20 de cobre.
    e.viajar(car);
    e.avanzar(e.tiempo_descarga());
    assert_eq!((e.a_bordo(car, Estano), e.a_bordo(car, Cobre)), (0, 20_000));
}

#[test]
fn cambiar_una_espera_de_existencias_se_recoge_en_su_proximo_pulso() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Estano, 20_000);
    let car = e.caravana(a, 1);
    let hay = cantidad(&e.n, a, Estano);
    let t0 = e.n.ahora();
    e.admin(encargar(car, b, Modo::Ruta, None, Parada { esperar_existencias: true, ..parada(&[(Estano, hay + 30_000)], &[]) }, Parada::default()));
    e.avanzar_hasta(t0 + MIN + MIN / 2);
    // Ahora pide lo que hay: no carga en el acto, sino en el pulso siguiente.
    e.admin(cambiar(car, b, None, Parada { esperar_existencias: true, ..parada(&[(Estano, hay)], &[]) }, Parada::default()));
    assert_eq!((e.situacion(car).2, e.k(car).contenido.is_empty()), (Fase::Espera, true));
    e.avanzar_hasta(t0 + 2 * MIN - 1);
    assert!(e.k(car).contenido.is_empty());
    e.avanzar_hasta(t0 + 2 * MIN);
    assert_eq!((e.situacion(car).2, e.a_bordo(car, Estano)), (Fase::Carga, hay));
}

#[test]
fn cambiar_el_destino_solo_se_puede_esperando_en_el_origen() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    let c = con_ajena(&mut e, m);
    e.dar(a, Cobre, 100_000);
    let car = e.caravana(a, 1);
    let con_precios = |recurso, cantidad| Parada { recoger: vec![LineaRecoger { recurso, cantidad, precio_max: Some(3_000_000) }], ..Parada::default() };
    let salir = e.n.ahora() + 2 * HORA;
    e.admin(encargar(car, b, Modo::Envio, Some(salir), parada(&[(Cobre, 50_000)], &[]), Parada::default()));
    // Esperando en el origen sí: el envío pasa a una plaza ajena (con precios en esa parada).
    assert_eq!(e.rechazada("admin", cambiar(car, c, None, Parada::default(), parada(&[(Estano, 1_000)], &[]))), rechazos::ENCARGO_FALTA_PRECIO);
    e.admin(cambiar(car, c, Some(salir), Parada::default(), con_precios(Estano, 1_000)));
    assert_eq!((e.encargo(car).destino, e.encargo(car).modo), (c as u32, Modo::Envio));
    // En ruta ya no: el destino es el que lleva, pero las paradas sí se pueden cambiar.
    e.avanzar_hasta(salir + e.tiempo_carga());
    assert_eq!(e.k(car).estado, EstadoCaravana::EnTransito);
    assert_eq!(e.rechazada("admin", cambiar(car, b, None, Parada::default(), con_precios(Estano, 1_000))), rechazos::CARAVANA_NO_DISPONIBLE);
    e.admin(cambiar(car, c, None, Parada::default(), con_precios(Estano, 2_000)));
    assert_eq!(e.encargo(car).en_destino.recoger[0].cantidad, 2_000);
}

#[test]
fn cambiar_la_hora_de_un_envio_que_espera_la_reprograma_y_despues_ya_no_cuenta() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    let car = e.caravana(a, 1);
    let (t0, carga) = (e.n.ahora(), e.tiempo_carga());
    let origen = parada(&[(Cobre, 50_000)], &[]);
    e.admin(encargar(car, b, Modo::Envio, Some(t0 + 2 * HORA), origen.clone(), Parada::default()));
    // Una hora fuera del máximo se rechaza; dentro, se reprograma: a la hora vieja no pasa nada.
    assert_eq!(e.rechazada("admin", cambiar(car, b, Some(t0 + 4 * DIA), origen.clone(), Parada::default())), rechazos::CARAVANA_SALIDA_INVALIDA);
    e.admin(cambiar(car, b, Some(t0 + 4 * HORA), origen.clone(), Parada::default()));
    assert_eq!(e.encargo(car).salir_en, Some(t0 + 4 * HORA));
    e.avanzar_hasta(t0 + 2 * HORA + carga);
    assert_eq!((e.situacion(car).2, e.k(car).contenido.is_empty()), (Fase::Espera, true));
    e.avanzar_hasta(t0 + 4 * HORA);
    assert_eq!((e.situacion(car).2, e.a_bordo(car, Cobre)), (Fase::Carga, 50_000));
    // Ya cargando, la hora no cuenta: se conserva la que había y solo cambia lo demás.
    let nueva = parada(&[(Cobre, 30_000)], &[]);
    e.admin(cambiar(car, b, Some(t0 + 6 * HORA), nueva.clone(), Parada::default()));
    assert_eq!((e.encargo(car).salir_en, e.encargo(car).fase), (Some(t0 + 4 * HORA), Fase::Carga));
    assert_eq!(e.ord("admin", cambiar(car, b, Some(t0 + 7 * HORA), nueva, Parada::default())).resultado, Resultado::SinCambios, "sin cambios reales");
}

#[test]
fn cambiar_un_envio_que_espera_sin_hora_empieza_ya_la_carga() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    let car = e.caravana(a, 1);
    let origen = parada(&[(Cobre, 50_000)], &[]);
    e.admin(encargar(car, b, Modo::Envio, Some(e.n.ahora() + 2 * HORA), origen.clone(), Parada::default()));
    e.admin(cambiar(car, b, None, origen, Parada::default()));
    assert_eq!((e.situacion(car).2, e.a_bordo(car, Cobre), e.encargo(car).salir_en), (Fase::Carga, 50_000, None));
    e.avanzar(e.tiempo_carga());
    assert_eq!(e.k(car).estado, EstadoCaravana::EnTransito);
    e.avanzar(2 * HORA);
    assert_eq!(hechos(&e.n, "comercio.caravana_carga").len(), 1, "la hora vieja no la hace cargar otra vez");
    assert_eq!((e.k(car).estado, hechos(&e.n, "comercio.encargo_terminado").len()), (EstadoCaravana::Disponible, 1));
}

// ───────────────────────────── 8. Un envío a una plaza ajena ─────────────────────────────

#[test]
fn un_envio_a_una_plaza_ajena_sin_mercado_activo_no_comercia_y_vuelve_con_lo_que_llevaba() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let a = e.a;
    let c = con_ajena(&mut e, m);
    e.dar(a, Cobre, 100_000);
    let car = e.caravana(a, 1);
    let (cobre_a, cobre_c) = (cantidad(&e.n, a, Cobre), cantidad(&e.n, c, Cobre));
    let en_c = Parada {
        recoger: vec![LineaRecoger { recurso: Estano, cantidad: 10_000, precio_max: Some(3_000_000) }],
        dejar: vec![LineaDejar { recurso: Cobre, tope: None, precio_min: Some(1_000_000) }],
        ..Parada::default()
    };
    e.admin(encargar(car, c, Modo::Envio, None, parada(&[(Cobre, 50_000)], &[Cobre]), en_c));
    e.avanzar(e.tiempo_carga());
    let llegada = e.fin_del_plan(car);
    e.avanzar_hasta(llegada);
    assert_eq!((e.k(car).estado, e.a_bordo(car, Cobre), cantidad(&e.n, c, Cobre)), (EstadoCaravana::Retornando, 50_000, cobre_c), "da la vuelta en el acto con lo que lleva");
    assert!(hechos(&e.n, "comercio.caravana_descarga").is_empty() && hechos(&e.n, "mercado.comercio_en_plaza").is_empty());
    // En el origen deja lo que traía y el envío queda cumplido.
    let vuelta = e.viajar_y_terminar(car);
    assert_eq!(cantidad(&e.n, a, Cobre), cobre_a, "lo que llevaba vuelve al almacén");
    let fin = hechos(&e.n, "comercio.encargo_terminado");
    assert_eq!((fin.len(), fin[0].0, dato(&fin[0], "motivo")), (1, vuelta, 0));
    assert_eq!(e.k(car).estado, EstadoCaravana::Disponible);
}

#[test]
fn un_envio_a_una_plaza_ajena_con_mercado_vende_a_sus_ordenes_de_compra_y_con_lo_cobrado_compra_en_el_mismo_viaje() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let a = e.a;
    let c = ajena_con_mercado(&mut e, m);
    e.dar(a, Cobre, 300_000);
    e.dar(a, Estano, 300_000);
    e.dar(c, Madera, 100_000);
    let (compra_cobre, compra_estano) = (orden(&mut e, c, false, Cobre, 300_000, PUNTO / 2), orden(&mut e, c, false, Estano, 300_000, PUNTO / 2));
    let venta_madera = orden(&mut e, c, true, Madera, 100_000, PUNTO);
    let car = e.caravana(a, 2);
    // 600 de mercancía ocupan los dos carros; a cambio compra 100 de madera, que ocupa uno.
    let en_origen = parada(&[(Cobre, 300_000), (Estano, 300_000)], &[Madera, Oro]);
    let en_destino = Parada { dejar: vec![vende(Cobre, None, PUNTO / 2), vende(Estano, None, PUNTO / 2)], recoger: vec![compra(Madera, 100_000, 2 * PUNTO)], ..Parada::default() };
    let (madera_a, madera_c, oro_c) = (cantidad(&e.n, a, Madera), cantidad(&e.n, c, Madera), cantidad(&e.n, c, Oro));
    let llegada = enviar_y_llegar(&mut e, car, c, en_origen, en_destino);

    // Vende: la plaza recibe la mercancía y paga su oro, y los dos carros tardan lo suyo.
    let d = hechos(&e.n, "comercio.caravana_descarga").into_iter().find(|h| h.1 == vec![c]).expect("descarga en la plaza ajena");
    assert_eq!((d.0, dato(&d, "cobre"), dato(&d, "estano"), dato(&d, "tirado"), dato(&d, "comision"), dato(&d, "carros")), (llegada, 300_000, 300_000, 0, 0, 2));
    assert_eq!((dato(&d, "finPaso"), e.encargo(car).fin_paso), (llegada + 2 * e.tiempo_descarga(), Some(llegada + 2 * e.tiempo_descarga())));
    assert_eq!((cantidad(&e.n, c, Cobre), cantidad(&e.n, c, Estano)), (300_000, 300_000));
    let oro_cobrado = e.a_bordo(car, Oro);
    assert!(oro_cobrado > 0 && oro_c - cantidad(&e.n, c, Oro) == oro_cobrado, "el oro que paga la plaza es el que cobra la caravana");
    assert_eq!((e.a_bordo(car, Cobre), e.a_bordo(car, Estano)), (0, 0));
    assert_eq!(servidas(&e, car).len(), 2);

    // Compra con lo cobrado, sin esperar existencias: un carro de madera.
    let fin_descarga = llegada + 2 * e.tiempo_descarga();
    e.avanzar_hasta(fin_descarga);
    let k = hechos(&e.n, "comercio.caravana_carga").into_iter().find(|h| h.1 == vec![c]).expect("carga en la plaza ajena");
    assert_eq!((k.0, dato(&k, "madera"), dato(&k, "carros"), dato(&k, "finPaso")), (fin_descarga, 100_000, 1, fin_descarga + e.tiempo_carga()));
    assert_eq!((e.situacion(car).2, e.encargo(car).fin_paso), (Fase::Carga, Some(fin_descarga + e.tiempo_carga())));
    assert_eq!((e.a_bordo(car, Madera), madera_c - cantidad(&e.n, c, Madera)), (100_000, 100_000));
    assert!(e.a_bordo(car, Oro) < oro_cobrado, "pagó la madera con lo cobrado");
    assert_eq!(servidas(&e, car).len(), 3);
    for orden in [compra_cobre, compra_estano, venta_madera] {
        assert!(e.n.libro_ordenes().orden(orden).is_none() && e.n.libro_ordenes().buscar(orden).is_some_and(|o| o.estado == EstadoOrden::Cumplida), "cumplida entera: pasa al historial");
    }
    e.avanzar_hasta(fin_descarga + e.tiempo_carga());
    assert_eq!(e.k(car).estado, EstadoCaravana::Retornando);

    // Vuelve con el oro y la madera y los deja en el origen: el envío queda cumplido.
    let a_bordo_oro = e.a_bordo(car, Oro);
    e.viajar_y_terminar(car);
    let en_a = hechos(&e.n, "comercio.caravana_descarga").into_iter().rfind(|h| h.1 == vec![a]).expect("descarga en el origen");
    // El almacén del origen tiene poco sitio para el oro: lo que cabe entra y el resto se queda a bordo (sin desechar) hasta que el envío termina y lo vuelca.
    assert_eq!(cantidad(&e.n, a, Madera) - madera_a, 100_000);
    assert_eq!(dato(&en_a, "oro") + dato(&en_a, "aBordo"), a_bordo_oro, "todo el oro cobrado llegó al origen o sigue a bordo");
    let fin = hechos(&e.n, "comercio.encargo_terminado");
    assert_eq!((fin.len(), dato(&fin[0], "motivo"), e.k(car).estado), (1, 0, EstadoCaravana::Disponible));
}

#[test]
fn en_una_plaza_ajena_vende_de_la_orden_que_mejor_paga_a_la_que_peor_y_respeta_precios_y_topes_y_cumple_a_medias_o_entera() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let a = e.a;
    let c = ajena_con_mercado(&mut e, m);
    e.dar(a, Cobre, 150_000);
    e.dar(a, Estano, 100_000);
    e.dar(c, Madera, 200_000);
    // Compran cobre: la de 3 y la de 2 (se cumplen), la de 1 (bajo el mínimo de 1,5) no se toca aunque falten 20 para el tope. Compran estaño: solo 30 de 100 por el tope.
    let (cobre_1, cobre_3, cobre_2) = (orden(&mut e, c, false, Cobre, 100_000, PUNTO), orden(&mut e, c, false, Cobre, 50_000, 3 * PUNTO), orden(&mut e, c, false, Cobre, 50_000, 2 * PUNTO));
    let estano = orden(&mut e, c, false, Estano, 100_000, 2 * PUNTO);
    // Venden madera: se toma la de 1 y la de 1,5; la de 3 pasa del máximo de 2.
    let (madera_15, madera_3, madera_1) = (orden(&mut e, c, true, Madera, 40_000, PUNTO * 3 / 2), orden(&mut e, c, true, Madera, 40_000, 3 * PUNTO), orden(&mut e, c, true, Madera, 40_000, PUNTO));
    let car = e.caravana(a, 1);
    let en_destino = Parada {
        dejar: vec![vende(Cobre, Some(120_000), PUNTO * 3 / 2), vende(Estano, Some(30_000), PUNTO)],
        recoger: vec![compra(Madera, 100_000, 2 * PUNTO)],
        ..Parada::default()
    };
    let llegada = enviar_y_llegar(&mut e, car, c, parada(&[(Cobre, 150_000), (Estano, 100_000)], &[Madera, Oro]), en_destino);
    e.avanzar_hasta(llegada + e.tiempo_descarga() + e.tiempo_carga());

    // Primero el cobre de la que mejor paga a la que peor, luego el estaño, luego la madera de la más barata a la más cara.
    assert_eq!(
        servidas(&e, car),
        vec![(cobre_3 as i64, 50_000), (cobre_2 as i64, 50_000), (estano as i64, 30_000), (madera_1 as i64, 40_000), (madera_15 as i64, 40_000)]
    );
    assert_eq!((e.a_bordo(car, Cobre), e.a_bordo(car, Estano), e.a_bordo(car, Madera)), (50_000, 70_000, 80_000), "lo que no se vende sigue a bordo");
    let l = e.n.libro_ordenes();
    let estado = |id: u64| l.buscar(id).map(|o| (o.estado, o.cantidad_cumplida)).unwrap();
    assert_eq!((estado(cobre_3), estado(cobre_2)), ((EstadoOrden::Cumplida, 50_000), (EstadoOrden::Cumplida, 50_000)));
    assert!(l.historial.iter().any(|o| o.id == cobre_3) && l.orden(cobre_3).is_none(), "una orden cumplida pasa al historial");
    assert_eq!((estado(cobre_1), estado(estano)), ((EstadoOrden::Activa, 0), (EstadoOrden::Activa, 30_000)), "la del precio bajo no se toca y la del tope queda a medias");
    assert_eq!((estado(madera_3), estado(madera_15)), ((EstadoOrden::Activa, 0), (EstadoOrden::Cumplida, 40_000)));
}

#[test]
fn en_una_plaza_ajena_sin_ordenes_que_casen_no_comercia_vuelve_con_lo_que_lleva_y_no_tira_nada_aunque_desechar_sobrante_este_puesto() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let a = e.a;
    let c = ajena_con_mercado(&mut e, m);
    e.dar(a, Cobre, 100_000);
    // Solo hay una orden de compra por debajo del mínimo y ninguna de venta; el almacén de la plaza no tiene sitio para el cobre que traen.
    orden(&mut e, c, false, Cobre, 100_000, PUNTO / 2);
    let hueco = e.n.economia(c).unwrap().almacen.espacio(Cobre);
    e.dar(c, Cobre, hueco);
    let cobre = mundo_de(&e.n, Cobre);
    let car = e.caravana(a, 1);
    let en_destino = Parada { dejar: vec![vende(Cobre, None, PUNTO)], recoger: vec![compra(Estano, 10_000, 3 * PUNTO)], desechar_sobrante: true, ..Parada::default() };
    e.admin(encargar(car, c, Modo::Envio, None, parada(&[(Cobre, 100_000)], &[Cobre]), en_destino));
    e.avanzar_hasta(e.encargo(car).fin_paso.expect("carga en el origen"));
    let llegada = e.fin_del_plan(car);
    e.avanzar_hasta(llegada);
    let d = hechos(&e.n, "comercio.caravana_descarga").into_iter().find(|h| h.1 == vec![c]).expect("descarga en la plaza ajena");
    assert_eq!((d.0, dato(&d, "carros"), dato(&d, "tirado"), dato(&d, "aBordo"), e.a_bordo(car, Cobre)), (llegada, 0, 0, 100_000, 100_000));
    assert_eq!((e.k(car).estado, e.k(car).contenido.len()), (EstadoCaravana::Retornando, 1), "sale en el acto: no compra nada y no hay carga");
    assert!(hechos(&e.n, "comercio.caravana_carga").iter().all(|h| h.1 != vec![c]) && hechos(&e.n, "mercado.comercio_en_plaza").is_empty());
    e.viajar_y_terminar(car);
    assert_eq!(mundo_de(&e.n, Cobre), cobre, "no se tiró nada");
    assert_eq!(e.k(car).estado, EstadoCaravana::Disponible);
}

#[test]
fn la_plaza_de_una_faccion_aliada_se_trata_como_cualquier_ajena() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let a = e.a;
    let c = ajena_con_mercado(&mut e, m);
    let (rey_1, rey_2) = (e.n.faccion(1).unwrap().rey.unwrap(), e.n.faccion(2).unwrap().rey.unwrap());
    let propuesta = e.ok("rey", Comando::ProponerRelacion { heroe: rey_1, a: 1, b: 2, vasallaje: false, tributo: None }).creadas[0];
    e.ok("beto", Comando::ResponderRelacion { heroe: rey_2, propuesta, aceptar: true });
    assert_eq!(e.n.relacion(1, 2), Some(nucleo::Relacion::Alianza));
    e.dar(a, Cobre, 100_000);
    let compra_cobre = orden(&mut e, c, false, Cobre, 100_000, PUNTO);
    let car = e.caravana(a, 1);
    let en_destino = Parada { dejar: vec![vende(Cobre, None, PUNTO)], ..Parada::default() };
    enviar_y_llegar(&mut e, car, c, parada(&[(Cobre, 100_000)], &[Oro]), en_destino);
    assert_eq!(servidas(&e, car), vec![(compra_cobre as i64, 100_000)], "vende como a cualquier plaza de otra Facción, con la comisión de fuera");
    let h = &hechos(&e.n, "mercado.comercio_en_plaza")[0];
    assert!(dato(h, "comision") > 0 && dato(h, "oroMovido") < dato(h, "valor"));
}

#[test]
fn el_oro_se_conserva_lo_que_paga_la_plaza_es_lo_que_cobra_la_caravana_y_la_comision_se_queda_sin_acunar_nada() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let a = e.a;
    let c = ajena_con_mercado(&mut e, m);
    e.dar(a, Cobre, 300_000);
    e.dar(c, Madera, 50_000);
    orden(&mut e, c, false, Cobre, 100_000, 2 * PUNTO);
    orden(&mut e, c, false, Cobre, 100_000, PUNTO);
    orden(&mut e, c, true, Madera, 50_000, PUNTO);
    let car = e.caravana(a, 1);
    let en_destino = Parada { dejar: vec![vende(Cobre, None, PUNTO)], recoger: vec![compra(Madera, 50_000, 2 * PUNTO)], ..Parada::default() };
    e.admin(encargar(car, c, Modo::Envio, None, parada(&[(Cobre, 200_000)], &[Madera, Oro]), en_destino));
    e.avanzar_hasta(e.encargo(car).fin_paso.unwrap());
    let llegada = e.fin_del_plan(car);
    e.avanzar_hasta(llegada - 1);
    let (oro_mundo, oro_c) = (mundo_de(&e.n, Oro), cantidad(&e.n, c, Oro));
    e.avanzar_hasta(llegada);
    // La venta: lo que sale del almacén de la plaza es lo que entra a la caravana, y la comisión no se paga: valor − comisión = oro movido (a la unidad entera).
    let ventas = hechos(&e.n, "mercado.comercio_en_plaza");
    let (valor, comision, movido) = (ventas.iter().map(|h| dato(h, "valor")).sum::<i64>(), ventas.iter().map(|h| dato(h, "comision")).sum::<i64>(), ventas.iter().map(|h| dato(h, "oroMovido")).sum::<i64>());
    assert!(comision > 0 && (valor - comision - movido).abs() < 1_000 * ventas.len() as i64, "valor {valor}, comisión {comision}, movido {movido}");
    assert_eq!((oro_c - cantidad(&e.n, c, Oro), e.a_bordo(car, Oro)), (movido, movido));
    assert_eq!(mundo_de(&e.n, Oro), oro_mundo, "no se acuña oro al vender");
    // La compra paga con ese oro y el mundo sigue igual.
    e.avanzar_hasta(llegada + e.tiempo_descarga());
    assert!(e.a_bordo(car, Oro) < movido && e.a_bordo(car, Madera) == 50_000);
    assert_eq!(mundo_de(&e.n, Oro), oro_mundo, "ni al comprar");
}

// ───────────────────────────── 9. Rechazos y autorización ─────────────────────────────

#[test]
fn cada_rechazo_del_encargo_sale_con_su_codigo_y_no_cambia_el_estado() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    let c = con_ajena(&mut e, m);
    e.dar(a, Cobre, 100_000);
    let car = e.caravana(a, 1);
    let ahora = e.n.ahora();
    let vacia = Parada::default;
    // Sin encargo no hay qué cambiar ni cancelar.
    assert_eq!(e.rechazada("admin", Comando::CancelarEncargo { caravana: car }), rechazos::ENCARGO_NO_TIENE);
    assert_eq!(e.rechazada("admin", cambiar(car, b, None, vacia(), vacia())), rechazos::ENCARGO_NO_TIENE);
    // Lo que valida el crate, en su orden.
    assert_eq!(e.rechazada("admin", encargar(car, a, Modo::Ruta, None, vacia(), vacia())), rechazos::ENCARGO_MISMO_ORIGEN);
    assert_eq!(e.rechazada("admin", encargar(car, c, Modo::Ruta, None, vacia(), vacia())), rechazos::ENCARGO_DESTINO_AJENO);
    assert_eq!(e.rechazada("admin", encargar(car, b, Modo::Ruta, Some(ahora + HORA), vacia(), vacia())), rechazos::CARAVANA_SALIDA_INVALIDA, "una ruta sale en cuanto se le da");
    assert_eq!(e.rechazada("admin", encargar(car, b, Modo::Envio, Some(ahora), vacia(), vacia())), rechazos::CARAVANA_SALIDA_INVALIDA);
    assert_eq!(e.rechazada("admin", encargar(car, b, Modo::Envio, Some(ahora + 4 * DIA), vacia(), vacia())), rechazos::CARAVANA_SALIDA_INVALIDA);
    assert_eq!(e.rechazada("admin", encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 500)], &[]), vacia())), rechazos::ENCARGO_LINEA_INVALIDA);
    assert_eq!(e.rechazada("admin", encargar(car, c, Modo::Envio, None, vacia(), parada(&[(Estano, 1_000)], &[]))), rechazos::ENCARGO_FALTA_PRECIO);
    assert_eq!(e.rechazada("admin", encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 400_000), (Estano, 200_000)], &[]), vacia())), rechazos::ENCARGO_SUPERA_CAPACIDAD);
    // Una caravana que no está disponible no recibe encargo.
    e.avanzar(11 * MIN);
    let preparando = e.caravana(a, 1);
    e.admin(Comando::PrepararCaravana { caravana: preparando, destino: b, carga: vec![(Cobre, 1_000)], salir_en: Some(e.n.ahora() + HORA), escolta: None });
    assert_eq!(e.rechazada("admin", encargar(preparando, b, Modo::Ruta, None, vacia(), vacia())), rechazos::CARAVANA_NO_DISPONIBLE);
    // Con un encargo dado, otro no.
    e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 10_000)], &[]), vacia()));
    assert_eq!(e.rechazada("admin", encargar(car, b, Modo::Ruta, None, vacia(), vacia())), rechazos::ENCARGO_YA_TIENE);
    // Cambiar se valida igual que encargar, con el modo de siempre.
    assert_eq!(e.rechazada("admin", cambiar(car, c, None, vacia(), vacia())), rechazos::ENCARGO_DESTINO_AJENO);
    assert_eq!(e.rechazada("admin", cambiar(car, a, None, vacia(), vacia())), rechazos::ENCARGO_MISMO_ORIGEN);
    assert_eq!(e.rechazada("admin", cambiar(car, b, None, parada(&[(Cobre, 500)], &[]), vacia())), rechazos::ENCARGO_LINEA_INVALIDA);
    assert_eq!(e.rechazada("admin", cambiar(car, b, None, parada(&[(Cobre, 600_000)], &[]), vacia())), rechazos::ENCARGO_SUPERA_CAPACIDAD);
}

#[test]
fn el_maestro_de_caravanas_da_el_encargo_y_el_sacerdote_no() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    let rey = e.n.faccion(1).and_then(|f| f.rey).expect("F1 tiene Rey");
    let (ana, bea, eli) = (e.heroe_residente("ana", a), e.heroe_residente("bea", a), e.heroe_residente("eli", a));
    e.ok("rey", Comando::AsignarCargoLocal { heroe: rey, asentamiento: a, cargo: Cargo::Gobernador, designado: ana });
    e.ok("ana", Comando::AsignarCargoLocal { heroe: ana, asentamiento: a, cargo: Cargo::MaestroCaravanas, designado: eli });
    e.ok("ana", Comando::AsignarCargoLocal { heroe: ana, asentamiento: a, cargo: Cargo::Sacerdote, designado: bea });
    e.dar(a, Cobre, 100_000);
    let car = e.caravana(a, 1);
    let ruta = || encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 10_000)], &[]), Parada::default());
    assert_eq!(e.rechazada("bea", ruta()), rechazos::NO_AUTORIZADO, "el Sacerdote no");
    assert_eq!(e.rechazada("bea", Comando::CancelarEncargo { caravana: car }), rechazos::NO_AUTORIZADO);
    e.ok("eli", ruta());
    assert_eq!(e.k(car).estado, EstadoCaravana::Esperando, "el Maestro de Caravanas da el encargo");
    assert_eq!(e.rechazada("bea", cambiar(car, b, None, Parada::default(), Parada::default())), rechazos::NO_AUTORIZADO);
    e.ok("eli", Comando::CancelarEncargo { caravana: car });
    assert_eq!(e.k(car).estado, EstadoCaravana::Disponible, "y también lo cancela");
}

// ───────────────────────────── 10. Dentro de la plaza ─────────────────────────────

#[test]
fn una_caravana_esperando_no_esta_en_la_rejilla_de_encuentros() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    e.dar(b, Estano, 100_000);
    let car = e.caravana(a, 1);
    let en_la_rejilla = e.n.cosas_en_el_espacio();
    // Esperando en el origen, igual que `disponible`: no está.
    e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 10_000)], &[Estano]), parada(&[(Estano, 10_000)], &[Cobre])));
    assert_eq!((e.k(car).estado, e.n.cosas_en_el_espacio()), (EstadoCaravana::Esperando, en_la_rejilla));
    // En camino sí está; esperando en el destino, otra vez no.
    e.avanzar(e.tiempo_carga());
    assert_eq!((e.k(car).estado, e.n.cosas_en_el_espacio()), (EstadoCaravana::EnTransito, en_la_rejilla + 1));
    e.viajar(car);
    assert_eq!(e.n.cosas_en_el_espacio(), en_la_rejilla);
    e.avanzar(e.tiempo_descarga() + e.tiempo_carga());
    assert_eq!((e.k(car).estado, e.n.cosas_en_el_espacio()), (EstadoCaravana::Retornando, en_la_rejilla + 1));
}

// ───────────────────────────── 11. La foto ─────────────────────────────

#[test]
fn recuperar_desde_foto_con_una_caravana_a_mitad_de_encargo_sigue_el_ciclo_igual() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9) };
    // Una ruta cuya caravana acaba de llegar al destino: en plena descarga, con su fin pendiente en la cola.
    let primera_mitad = |e: &mut Escenario| {
        let (a, b) = (e.a, e.b);
        e.dar(a, Cobre, 100_000);
        e.dar(b, Estano, 100_000);
        let car = e.caravana(a, 1);
        e.admin(encargar(car, b, Modo::Ruta, None, parada(&[(Cobre, 100_000)], &[Estano]), parada(&[(Estano, 50_000)], &[Cobre])));
        e.avanzar(e.tiempo_carga());
        e.viajar(car);
        e.avanzar(MIN / 2);
        assert_eq!(e.situacion(car), (EstadoCaravana::Esperando, Punta::Destino, Fase::Descarga));
    };
    let segunda_mitad = |e: &mut Escenario| e.avanzar(10 * HORA);

    let sin_parar = {
        let mut e = escenario(m, 9);
        primera_mitad(&mut e);
        e.avanzar(10 * MIN);
        segunda_mitad(&mut e);
        (e.n.hash_estado(), e.n.estado().comercio.clone())
    };

    let (a_mitad, cuenta) = {
        let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        let mut e = escenario_de(n, m);
        primera_mitad(&mut e);
        e.n.foto_ahora();
        e.avanzar(10 * MIN);
        e.n.apagar().unwrap();
        (e.n.hash_estado(), e.cuenta)
    };
    let n = Nucleo::con_persistencia(con(ajustes), dir.path(), Default::default()).unwrap();
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), a_mitad, "foto + registro reconstruyen el mismo mundo, con el encargo a mitad");
    let ids: Vec<u64> = n.estado().asentamientos.keys().copied().collect();
    let mut e = Escenario { n, a: ids[0], b: ids[1], cuenta };
    segunda_mitad(&mut e);
    assert_eq!(e.n.estado().comercio, sin_parar.1, "el comercio recuperado sigue igual que el que no se interrumpió");
    assert_eq!(e.n.hash_estado(), sin_parar.0);
    assert!(e.n.estado().comercio.flota[0].encargo.as_ref().unwrap().vueltas >= 2, "siguió dando vueltas");
    e.n.apagar().unwrap();
}
