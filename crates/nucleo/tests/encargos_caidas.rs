//! Las plazas que caen y las caravanas destruidas con encargo (Comercio_Caravanas §10 y §12): qué pasa con una caravana de ruta o de envío cuando
//! una de sus dos plazas cae en ruinas o es conquistada, y cuando la caravana se destruye. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas
//! se omiten si no está.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::Recurso::{self, *};
use base::{Instante, Punto, MS_POR_MINUTO};
use comercio::caravana::{Animal, Caravana, EstadoCaravana, TipoCarro};
use comercio::encargo::{Encargo, LineaDejar, LineaRecoger, Modo, Parada, Punta};
use comun::real::{fundar, sitio_rico, M};
use militar::catalogo::Tropa;
use militar::escuadra::Contenedor;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::{Ajustes, Arranque, Comando, Nucleo, ObjetivoAtaque, Peticion, Reglas, Resultado, Respuesta};

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

/// Sin acoso ni anillo de bandidos y sin mantenimiento por distancia: la plaza conquistada, lejos de la capital del conquistador, no debe arruinarse por ello.
fn arranque(m: &M, semilla: u64) -> Arranque {
    let mut reglas = Reglas::default();
    let b = &mut std::sync::Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    std::sync::Arc::make_mut(&mut reglas.economia.datos).mantenimiento.escala_distancia = i64::MAX / 4;
    Arranque { reglas, ajustes: Ajustes::default(), semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

type Hecho = (Instante, Vec<u64>, Vec<(&'static str, i64)>);

fn hechos(n: &Nucleo, codigo: &str) -> Vec<Hecho> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).map(|e| (e.en, e.asentamientos.clone(), e.datos.clone())).collect()
}

fn dato(h: &Hecho, clave: &str) -> i64 {
    h.2.iter().find(|(k, _)| *k == clave).unwrap_or_else(|| panic!("el hecho no lleva «{clave}»: {h:?}")).1
}

fn cantidad(n: &Nucleo, plaza: u64, r: Recurso) -> i64 {
    n.economia(plaza).unwrap().almacen.cantidad(r)
}

fn pos(n: &Nucleo, plaza: u64) -> Punto {
    n.estado().asentamientos[&plaza].posicion
}

/// Lo que recoge (`recurso`, `cantidad`) en una parada de plaza propia; `esperar` es esperar existencias.
fn recoger(lineas: &[(Recurso, i64)], esperar: bool) -> Parada {
    Parada { recoger: lineas.iter().map(|&(recurso, cantidad)| LineaRecoger { recurso, cantidad, precio_max: None }).collect(), esperar_existencias: esperar, ..Parada::default() }
}

/// Lo que deja sin tope en una parada de plaza propia.
fn dejar(recursos: &[Recurso]) -> Parada {
    Parada { dejar: recursos.iter().map(|&recurso| LineaDejar { recurso, tope: None, precio_min: None }).collect(), ..Parada::default() }
}

fn encargar(caravana: u64, destino: u64, modo: Modo, salir_en: Option<Instante>, en_origen: Parada, en_destino: Parada) -> Comando {
    Comando::EncargarCaravana { caravana, destino, modo, salir_en, en_origen, en_destino }
}

/// Dos plazas de la Facción 1 con Mercado (`a`, con el Rey de la Facción, y `b`, de campamentos distintos), con el tiempo parado en el instante en que los dos Mercados ya están activos.
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
    /// Un héroe de la Facción 1 con una escuadra de milicia asignada de escolta a la caravana; devuelve (héroe, escuadra).
    fn escoltar(&mut self, cuenta: &str, plaza: u64, caravana: u64) -> (u64, u64) {
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano, nombre: None, clase: "espada_escudo_mediano".into(), aspecto: Default::default() }).creadas[0];
        self.admin(Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
        self.admin(Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
        self.admin(Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
        let escuadra = self.admin(Comando::DarEscuadra { heroe: h, tropa: Tropa::MiliciaLanceros, cantidad: 25 }).creadas[0];
        self.ok(cuenta, Comando::AsignarEscolta { caravana, heroe: h, escuadras: vec![escuadra] });
        (h, escuadra)
    }
    fn contenedor(&self, heroe: u64, escuadra: u64) -> Contenedor {
        self.n.heroe(heroe).unwrap().heroe.escuadra(escuadra).unwrap().contenedor
    }
    /// Los `encargo_terminado` de la caravana: (instante, motivo, volcado, perdido).
    fn terminados(&self, caravana: u64) -> Vec<(Instante, i64, i64, i64)> {
        hechos(&self.n, "comercio.encargo_terminado").iter().filter(|h| dato(h, "caravana") == caravana as i64).map(|h| (h.0, dato(h, "motivo"), dato(h, "volcado"), dato(h, "perdido"))).collect()
    }
    fn perdidas(&self) -> Vec<i64> {
        hechos(&self.n, "comercio.caravana_perdida").iter().map(|h| dato(h, "caravana")).collect()
    }
    /// El ejército de la Facción 2 asedia `plaza` (sin defensores) y la conquista al acabar la convocatoria. Devuelve el instante de la conquista.
    fn conquistar(&mut self, plaza: u64) -> Instante {
        let ahora = self.n.ahora();
        let p = pos(&self.n, plaza);
        let gus = self.n.crear_ejercito(ahora, "gus", 2, Punto::new(p.x + 1_000, p.y), 500, 100_000).unwrap();
        let heroe = self.n.ejercito(gus).unwrap().columna.lider;
        let arqueros = Tropa::ArquerosCompuesto;
        self.admin(Comando::DarEscuadra { heroe, tropa: arqueros, cantidad: militar::Reglas::cargar().tropas.tropa(arqueros).unidades });
        self.ok("gus", Comando::Atacar { heroe, objetivo: ObjetivoAtaque::Asentamiento(plaza), declara_guerra: true });
        let fin = self.n.batallas_activas().filter_map(|b| b.convocatoria_hasta).max().expect("hay convocatoria");
        self.n.avanzar_hasta(fin);
        assert_eq!(self.n.estado().asentamientos[&plaza].faccion, 2, "la plaza cambió de dueño");
        hechos(&self.n, "asentamiento.conquistada")[0].0
    }
}

/// Funda las dos plazas de la Facción 1 y les construye el Mercado. Con `ruina`, la plaza `a` queda con poca madera y sin auto-construcción: acabará en ruinas.
fn escenario(m: &M, ruina: bool) -> Escenario {
    let mut n = Nucleo::nuevo(arranque(m, 7));
    let pa = sitio_rico(&n, &m.0, 0, 1);
    let a = fundar(&mut n, "rey", 1, pa);
    let mut e = Escenario { n, a, b: 0, cuenta: 0 };
    e.admin(Comando::FijarNivelFaccion { faccion: 1, nivel: 2 });
    let pb = sitio_rico(&e.n, &m.0, 3, 1);
    e.b = fundar(&mut e.n, "rey_b", 1, pb);
    for (plaza, madera) in [(e.a, if ruina { 200_000 } else { 350_000 }), (e.b, 350_000)] {
        e.admin(Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
        e.dar(plaza, Madera, madera);
        e.admin(Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
    }
    e.avanzar(9 * HORA);
    for plaza in [e.a, e.b] {
        assert!(e.n.economia(plaza).unwrap().edificios.iter().any(|x| x.tipo == Tipo::Mercado && x.activo()), "el Mercado de as-{plaza} debería estar activo");
    }
    e
}

/// El escenario de la ruina de `a`, con una caravana de un carro en cada plaza (las dos plazas cuestan lo mismo en el ensayo que en la prueba).
fn escenario_ruina(m: &M) -> (Escenario, u64, u64) {
    let mut e = escenario(m, true);
    let (a, b) = (e.a, e.b);
    let (ca, cb) = (e.caravana(a, 1), e.caravana(b, 1));
    (e, ca, cb)
}

/// El instante en que la plaza `a` cae en ruinas en `escenario_ruina` (un ensayo previo: el mundo es determinista).
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

/// El escenario de la ruina de `a`, parado `antes` antes de que caiga, con la hora de la ruina y las caravanas de `a` y de `b`.
fn antes_de_la_ruina(m: &M, antes: i64) -> (Escenario, Instante, u64, u64) {
    let ruina = hora_de_la_ruina(m);
    let (mut e, ca, cb) = escenario_ruina(m);
    e.avanzar_hasta(ruina - antes);
    (e, ruina, ca, cb)
}

/// Deja que la plaza `a` caiga: avanza hasta el instante de la ruina, que llega a su hora.
fn arruinar(e: &mut Escenario, ruina: Instante) {
    e.avanzar_hasta(ruina - 1);
    assert!(e.n.economia(e.a).is_some(), "la ruina llega exactamente a su hora");
    e.avanzar_hasta(ruina);
    assert!(e.n.economia(e.a).is_none(), "la plaza cayó en ruinas");
}

/// Da a la caravana `car` de la plaza `b` un encargo hacia la `a` que carga 50 de cobre en el origen.
fn de_b_a_a(e: &mut Escenario, car: u64, modo: Modo, en_destino: Parada) {
    let (a, b) = (e.a, e.b);
    e.dar(b, Cobre, 100_000);
    e.admin(encargar(car, a, modo, None, recoger(&[(Cobre, 50_000)], false), en_destino));
}

// ───────────────────────────── 1. El destino cae en ruinas ─────────────────────────────

#[test]
fn la_plaza_destino_cae_en_ruinas_con_la_caravana_esperando_dentro_y_se_destruye_con_todo() {
    let m = con_mundo!();
    let (mut e, ruina, ca, car) = antes_de_la_ruina(m, 3 * HORA);
    let (a, b) = (e.a, e.b);
    assert!(cantidad(&e.n, a, Estano) < 1_000, "A no tiene el estaño que la caravana espera");
    // Llega a A, deja su cobre y espera allí un estaño que no hay (el viaje dura menos de dos horas).
    de_b_a_a(&mut e, car, Modo::Ruta, Parada { dejar: dejar(&[Cobre]).dejar, ..recoger(&[(Estano, 10_000)], true) });
    let (h, escuadra) = e.escoltar("carla", b, car);
    e.avanzar_hasta(ruina - 30 * MIN);
    assert_eq!((e.k(car).estado, e.encargo(car).punta), (EstadoCaravana::Esperando, Punta::Destino), "espera dentro de A");
    assert_eq!(e.a_bordo(car, Cobre), 0, "dejó su carga en A");
    arruinar(&mut e, ruina);

    assert!(e.n.caravana(car).is_none(), "la que esperaba en el destino se destruye con la plaza");
    assert_eq!(e.perdidas(), vec![ca as i64, car as i64], "la de A se pierde por salir de A y la de B, por esperar dentro");
    assert_eq!(e.terminados(car), vec![(ruina, 3, 0, 0)], "encargo terminado, motivo destruida");
    assert!(e.terminados(ca).is_empty(), "la de A no tenía encargo");
    assert_eq!(e.contenedor(h, escuadra), Contenedor::Campamento, "su escolta vuelve al campamento");
    e.avanzar(DIA);
    assert_eq!(hechos(&e.n, "comercio.encargo_terminado").len(), 1, "nada más termina");
}

#[test]
fn la_plaza_destino_cae_en_ruinas_con_la_caravana_en_camino_regresa_vuelca_y_termina_con_destino_perdido() {
    let m = con_mundo!();
    let (mut e, ruina, ca, car) = antes_de_la_ruina(m, 20 * MIN);
    let (a, b) = (e.a, e.b);
    de_b_a_a(&mut e, car, Modo::Ruta, Parada::default());
    let (h, escuadra) = e.escoltar("carla", b, car);
    let cobre_b = cantidad(&e.n, b, Cobre);
    e.avanzar_hasta(ruina - 1);
    assert_eq!((e.k(car).estado, e.a_bordo(car, Cobre)), (EstadoCaravana::EnTransito, 50_000), "va hacia A cargada");
    arruinar(&mut e, ruina);

    let k = e.k(car);
    assert_eq!((k.estado, k.destino, k.encargo.as_ref().map(|x| (x.punta, x.cancelado))), (EstadoCaravana::Retornando, Some(a as u32), Some((Punta::Origen, true))));
    assert!(e.terminados(car).is_empty() && e.perdidas() == vec![ca as i64], "da la vuelta con su carga; solo se pierde la de A");
    let llegada = e.fin_del_plan(car);
    e.avanzar_hasta(llegada - 1);
    assert_eq!(e.k(car).estado, EstadoCaravana::Retornando);
    e.avanzar_hasta(llegada);
    let k = e.k(car);
    assert_eq!((k.estado, k.encargo.is_none(), k.contenido.is_empty(), k.escolta.is_empty(), k.destino), (EstadoCaravana::Disponible, true, true, true, None), "disponible y limpia");
    assert_eq!(e.contenedor(h, escuadra), Contenedor::Campamento, "la escolta vuelve");
    assert_eq!(cantidad(&e.n, b, Cobre), cobre_b + 50_000, "lo que llevaba vuelve al almacén del origen");
    assert_eq!(e.terminados(car), vec![(llegada, 2, 50_000, 0)], "destino perdido gana a cancelado");
    e.avanzar(DIA);
    assert_eq!(hechos(&e.n, "comercio.encargo_terminado").len(), 1);
}

// ───────────────────────────── 2. El destino es conquistado ─────────────────────────────

/// Escenario sin ruina con la plaza `a` ya atacable, una caravana de `b` a `a` en camino, y la conquista de `a`. Devuelve (escenario, caravana, instante de la conquista).
fn conquista_del_destino(m: &M, modo: Modo) -> (Escenario, u64, Instante) {
    let mut e = escenario(m, false);
    let a = e.a;
    e.avanzar_hasta(e.n.protegida_hasta(a).unwrap());
    let car = e.caravana(e.b, 1);
    de_b_a_a(&mut e, car, modo, Parada::default());
    e.avanzar(10 * MIN);
    assert_eq!((e.k(car).estado, e.a_bordo(car, Cobre)), (EstadoCaravana::EnTransito, 50_000), "va hacia A cargada");
    let conquista = e.conquistar(a);
    (e, car, conquista)
}

#[test]
fn la_plaza_destino_de_una_ruta_es_conquistada_con_la_caravana_en_camino_regresa_y_termina_con_destino_perdido() {
    let m = con_mundo!();
    let (mut e, car, conquista) = conquista_del_destino(m, Modo::Ruta);
    let (a, b) = (e.a, e.b);
    let cobre_b = cantidad(&e.n, b, Cobre);
    let k = e.k(car);
    assert_eq!((k.estado, k.destino, k.encargo.as_ref().map(|x| (x.punta, x.cancelado))), (EstadoCaravana::Retornando, Some(a as u32), Some((Punta::Origen, true))));
    assert!(e.terminados(car).is_empty());
    let llegada = e.fin_del_plan(car);
    assert!(llegada > conquista);
    e.avanzar_hasta(llegada);
    assert_eq!((e.k(car).estado, e.k(car).encargo.is_none()), (EstadoCaravana::Disponible, true));
    assert_eq!(cantidad(&e.n, b, Cobre), cobre_b + 50_000);
    assert_eq!(e.terminados(car), vec![(llegada, 2, 50_000, 0)], "la plaza ya no es de la Facción del origen");
}

#[test]
fn la_plaza_destino_de_un_envio_es_conquistada_con_la_caravana_en_camino_regresa_y_termina_cancelado() {
    let m = con_mundo!();
    let (mut e, car, _) = conquista_del_destino(m, Modo::Envio);
    let b = e.b;
    let cobre_b = cantidad(&e.n, b, Cobre);
    assert_eq!(e.k(car).estado, EstadoCaravana::Retornando);
    let llegada = e.fin_del_plan(car);
    e.avanzar_hasta(llegada);
    assert_eq!((e.k(car).estado, e.k(car).encargo.is_none()), (EstadoCaravana::Disponible, true));
    assert_eq!(cantidad(&e.n, b, Cobre), cobre_b + 50_000);
    assert_eq!(e.terminados(car), vec![(llegada, 1, 50_000, 0)], "el envío a una plaza que cambió de dueño sigue existiendo: termina cancelado");
}

// ───────────────────────────── 3. El origen cae ─────────────────────────────

#[test]
fn la_plaza_origen_es_conquistada_con_la_caravana_esperando_dentro_pasa_al_conquistador_sin_encargo_y_vuelca_en_el_almacen() {
    let m = con_mundo!();
    let mut e = escenario(m, false);
    let (a, b) = (e.a, e.b);
    // Da una vuelta: carga en A todo el cobre que hay, lo lleva a B y vuelve con él; en A espera (más cobre del que queda) con su carga a bordo.
    // La vuelta se da mientras A está protegida: sin Leñera, a A se le acaba la madera unas horas después de que deja de estarlo.
    e.dar(a, Cobre, 50_000);
    let cobre = cantidad(&e.n, a, Cobre);
    assert_eq!(cobre, 50_000, "todo el cobre de A cabe en un viaje");
    let car = e.caravana(a, 2);
    let (h, escuadra) = e.escoltar("carla", a, car);
    e.admin(encargar(car, b, Modo::Ruta, None, recoger(&[(Cobre, cobre)], true), Parada::default()));
    // Va, no deja nada y vuelve con todo (en B no hay nada que hacer): una vuelta después espera en A.
    while e.encargo(car).vueltas == 0 {
        e.avanzar(MIN);
    }
    e.avanzar(MIN);
    assert_eq!((e.k(car).estado, e.encargo(car).punta, e.a_bordo(car, Cobre)), (EstadoCaravana::Esperando, Punta::Origen, cobre), "espera en A con su cobre a bordo");
    let (almacen, escolta) = (cantidad(&e.n, a, Cobre), e.k(car).escolta.len());
    assert_eq!((almacen < cobre, escolta, e.contenedor(h, escuadra)), (true, 1, Contenedor::Escolta(car)));
    e.avanzar_hasta(e.n.ahora().max(e.n.protegida_hasta(a).unwrap()));
    assert_eq!((e.k(car).estado, e.a_bordo(car, Cobre), cantidad(&e.n, a, Cobre)), (EstadoCaravana::Esperando, cobre, almacen), "sigue esperando, igual");

    let conquista = e.conquistar(a);
    let k = e.k(car);
    assert_eq!((k.estado, k.encargo.is_none(), k.contenido.is_empty(), k.escolta.is_empty(), k.destino), (EstadoCaravana::Disponible, true, true, true, None), "disponible, sin encargo");
    assert_eq!(e.n.estado().asentamientos[&a].faccion, 2, "la plaza es del conquistador");
    assert_eq!(cantidad(&e.n, a, Cobre), almacen + cobre, "lo de a bordo entra en el almacén de la plaza");
    assert_eq!(e.contenedor(h, escuadra), Contenedor::Campamento, "la escolta vuelve al campamento");
    assert_eq!(e.terminados(car), vec![(conquista, 1, cobre, 0)], "termina cancelado");
    assert!(e.perdidas().is_empty());
    assert_eq!(hechos(&e.n, "comercio.flota_traspasada").len(), 1);
}

#[test]
fn la_plaza_origen_cae_en_ruinas_y_la_caravana_se_pierde_con_destruida() {
    let m = con_mundo!();
    let (mut e, ruina, car, _) = antes_de_la_ruina(m, 20 * MIN);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 100_000);
    // Un envío hacia B, ya cargado y en camino cuando A cae.
    e.admin(encargar(car, b, Modo::Envio, None, recoger(&[(Cobre, 50_000)], false), Parada::default()));
    e.avanzar(5 * MIN);
    assert_eq!((e.k(car).estado, e.a_bordo(car, Cobre)), (EstadoCaravana::EnTransito, 50_000), "va hacia B cargada");
    arruinar(&mut e, ruina);

    assert!(e.n.caravana(car).is_none());
    assert_eq!(e.perdidas(), vec![car as i64]);
    assert_eq!(e.terminados(car), vec![(ruina, 3, 0, 50_000)], "destruida: volcado 0 y perdido lo que llevaba");
}

// ───────────────────────────── 4. La caravana se destruye ─────────────────────────────

#[test]
fn una_caravana_con_encargo_que_pierde_todos_sus_carros_termina_destruida_y_una_sin_encargo_no_emite_nada() {
    let m = con_mundo!();
    let mut e = escenario(m, false);
    let (a, b) = (e.a, e.b);
    e.dar(a, Cobre, 200_000);
    let con = e.caravana(a, 1);
    e.avanzar(HORA);
    let sin = e.caravana(a, 1);
    e.admin(encargar(con, b, Modo::Ruta, None, recoger(&[(Cobre, 50_000)], false), Parada::default()));
    e.ok("admin", Comando::PrepararCaravana { caravana: sin, destino: b, carga: vec![(Cobre, 10_000)], salir_en: None, escolta: None });
    e.avanzar(5 * MIN);
    assert_eq!((e.k(con).estado, e.a_bordo(con, Cobre), e.k(sin).estado), (EstadoCaravana::EnTransito, 50_000, EstadoCaravana::EnTransito));
    let t = e.n.ahora();
    for car in [con, sin] {
        assert!(e.n.caravana_entra_en_batalla(t, car));
        assert!(e.n.aplicar_perdida_de_carros(t, car, &[0]).expect("existe").destruida);
        assert!(e.n.caravana(car).is_none());
    }
    assert_eq!(e.perdidas(), vec![con as i64, sin as i64]);
    assert_eq!(e.terminados(con).len(), 1);
    let (_, motivo, volcado, _) = e.terminados(con)[0];
    assert_eq!((motivo, volcado), (3, 0));
    assert!(e.terminados(sin).is_empty(), "una caravana sin encargo no emite encargo_terminado");
    assert_eq!(hechos(&e.n, "comercio.encargo_terminado").len(), 1);
}
