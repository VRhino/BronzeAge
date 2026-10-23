//! Entrar con todo (decidido por el autor el 2026-10-22; `Consideraciones/Entrar_Con_Todo_Definicion.md`, Presencia_Y_Convocatoria §4-§5, Movimiento_Ejercitos §11):
//! quien entra en un lugar lleva consigo tropa, carro, víveres y todas sus caravanas adjuntas, sean del origen que sean, y al salir se lo lleva; la tropa de un
//! visitante come de sus víveres; entrar en la residencia no vacía el carro (`vaciarCarro` lo hace); el dueño decide por relación quién entra (`fijarAcceso`); y si
//! el lugar cae quien estaba dentro sale al mapa con lo suyo. La defensa de quien está dentro (aliados sí, neutrales y enemigos no) está en `combate.rs`; la herencia
//! del acceso a los pueblos y a las avanzadas, en `pueblos.rs` y `avanzadas.rs`. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas
//! se omiten si no está.

use asentamiento::catalogo::Tipo;
use asentamiento::Recurso::{self, *};
use base::{Instante, Punto, MS_POR_MINUTO};
use comercio::caravana::{Animal, EstadoCaravana, TipoCarro};
use militar::catalogo::Tropa;
use militar::columna::{Politica, TipoColumna};
use militar::escuadra::Contenedor;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use militar::racion::{self, Estado as Marcha};
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{rechazos, Acceso, Ajustes, Arranque, Carga, Comando, Nucleo, Peticion, Relacion, Reglas, Resultado, Respuesta, SucesoMilitar};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;

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

fn mr() -> militar::Reglas {
    militar::Reglas::cargar()
}

fn fundar(n: &mut Nucleo, t: Instante, dueno: &str, faccion: u32, p: Punto) -> u64 {
    let e = n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap();
    assert!(n.aplicar_en(t, Peticion::nueva("admin", &format!("c-{e}"), Comando::DarCaravanaFundacion { ejercito: e }).de_admin()).aceptada());
    let r = n.aplicar_en(t, Peticion::nueva(dueno, &format!("f-{e}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

struct Escenario {
    n: Nucleo,
    /// La plaza de «ana» (Facción 1), junto al campamento 0.
    a: u64,
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
    fn dar(&mut self, plaza: u64, recurso: Recurso, cantidad: i64) {
        let r = self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
    }
    fn almacen(&self, recurso: Recurso) -> i64 {
        self.n.economia(self.a).unwrap().almacen.cantidad(recurso)
    }
    fn plaza(&self) -> Punto {
        self.n.estado().asentamientos[&self.a].posicion
    }
    fn ejercito(&self, id: u64) -> &nucleo::Ejercito {
        self.n.ejercito(id).expect("la columna existe")
    }
    fn viveres(&self, id: u64) -> i64 {
        self.ejercito(id).columna.viveres.valor_en(self.n.ahora())
    }
    /// Un héroe de la cuenta, nacido en el campamento `campamento`.
    fn heroe(&mut self, cuenta: &str, campamento: u32) -> u64 {
        self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento, controlador: Controlador::Humano }).creadas[0]
    }
    /// Un héroe de la cuenta ciudadano de la Facción 1, residente y presente en la plaza, con una escuadra de milicia (25 lanceros) en su campamento.
    fn con_milicia(&mut self, cuenta: &str) -> (u64, u64) {
        let h = self.heroe(cuenta, 0);
        let a = self.a as u32;
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(a)) });
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(a) });
        let r = self.ok(cuenta, Comando::ReclutarTropa { asentamiento: self.a, heroe: h, tropa: Tropa::MiliciaLanceros });
        (h, r.creadas[0])
    }
    /// Un héroe de la cuenta ciudadano de la Facción 1, residente y presente en la plaza, sin tropa.
    fn residente(&mut self, cuenta: &str) -> u64 {
        let h = self.heroe(cuenta, 0);
        let a = self.a as u32;
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(a)) });
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(a) });
        h
    }
    /// Un héroe de la cuenta, de la Facción dada, residente del campamento 0 con dos escuadras prestadas (30 granjeros y 30 mineros).
    fn con_prestamo(&mut self, cuenta: &str, faccion: u32) -> (u64, Vec<u64>) {
        let h = self.heroe(cuenta, 0);
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(faccion) });
        let ids = self.ok(cuenta, Comando::PedirPrestamo { heroe: h, tropas: vec![Tropa::Granjeros, Tropa::Mineros] }).creadas;
        (h, ids)
    }
    /// La columna sale del campamento 0 con lo dado y marcha hasta la puerta de la plaza.
    fn viaja_a_la_plaza(&mut self, cuenta: &str, h: u64, escuadras: &[u64], carga: &[(Recurso, i64)], politica: Politica) -> u64 {
        let col = self.ok(cuenta, Comando::SalirDelCampamento { campamento: 0, heroe: h, escuadras: escuadras.to_vec(), carga: carga.to_vec(), politica }).creadas[0];
        self.llega(cuenta, col);
        col
    }
    fn llega(&mut self, cuenta: &str, col: u64) {
        let destino = self.plaza();
        self.ok(cuenta, Comando::MarcharA { ejercito: col, destino });
        let fin = self.ejercito(col).plan.fin().unwrap();
        self.n.avanzar_hasta(fin);
    }
    fn escuadra(&self, h: u64, id: u64) -> militar::escuadra::Escuadra {
        self.n.heroe(h).unwrap().heroe.escuadra(id).unwrap().clone()
    }
}

fn escenario(m: &M, semilla: u64) -> Escenario {
    let mut n = Nucleo::nuevo(arranque(m, semilla));
    let p = sitio_rico(&n, &m.0, 0, 1);
    let a = fundar(&mut n, 0, "ana", 1, p);
    Escenario { n, a, cuenta: 0 }
}

/// Dos plazas de la Facción 1: la de «ana» junto al campamento 0 y otra junto al 2.
fn dos_plazas(m: &M, semilla: u64) -> (Escenario, u64) {
    let mut n = Nucleo::nuevo(arranque(m, semilla));
    let pa = sitio_rico(&n, &m.0, 0, 1);
    let a = fundar(&mut n, 0, "ana", 1, pa);
    assert!(n.aplicar_en(0, Peticion::nueva("admin", "nivel", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 }).de_admin()).aceptada());
    let pc = sitio_rico(&n, &m.0, 2, 1);
    let c = fundar(&mut n, 0, "ana", 1, pc);
    (Escenario { n, a, cuenta: 0 }, c)
}

// ───────────────────────────── 1. El visitante entra con todo y sale con todo ─────────────────────────────

#[test]
fn el_visitante_entra_con_todo_come_de_sus_viveres_y_sale_con_todo() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let a = e.a;
    let (b, ids) = e.con_prestamo("beto", 1);
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: b, recurso: Oro, cantidad: 30_000 });
    let cb = e.viaja_a_la_plaza("beto", b, &ids, &[(Oro, 20_000)], Politica::Rechazar);
    let soldados_de_la_plaza = e.n.soldados_del_campamento(a);
    let carga0 = e.ejercito(cb).columna.carga.clone();
    let viveres0 = e.viveres(cb);

    e.ok("beto", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: b });
    // Todo lo suyo entra con él: la columna sigue siendo suya, dentro, fuera del mapa, con su tropa, su carro y sus víveres.
    let hb = e.n.heroe(b).unwrap();
    assert_eq!((hb.columna, hb.heroe.presencia.ubicacion), (Some(cb), Ubicacion::Asentamiento(a as u32)));
    assert!(!e.n.en_el_mapa(cb), "dentro no está en el mapa: nadie la ve ni la ataca");
    assert_eq!((e.ejercito(cb).columna.escuadras.clone(), e.ejercito(cb).columna.carga.clone(), e.viveres(cb)), (ids.clone(), carga0.clone(), viveres0));
    assert!(ids.iter().all(|&s| e.escuadra(b, s).contenedor == Contenedor::Ejercito(cb)), "la tropa sigue en su columna: no pasa a ninguna guarnición");
    assert_eq!(e.n.soldados_del_campamento(a), soldados_de_la_plaza, "la plaza no cuenta a la tropa de su visita");

    // Come de sus víveres, como un ejército acampado.
    let consumo = e.n.consumo_de_columna(cb).unwrap();
    let soldados = e.n.soldados_de_columna(cb).unwrap();
    assert_eq!(consumo, racion::racion_columna(&mr(), soldados, 1, Marcha::Estacionada));
    let t0 = e.viveres(cb);
    e.avanzar(2 * HORA);
    assert!(e.viveres(cb) < t0 && (t0 - e.viveres(cb) - consumo * 120).abs() <= 2, "gastó lo de dos horas de acampada: {} frente a {}", t0 - e.viveres(cb), consumo * 120);
    assert_eq!(e.n.soldados_del_campamento(a), soldados_de_la_plaza, "y la plaza sigue sin contarla");
    let dentro = e.viveres(cb);

    // Sale con todo lo que entró.
    e.ok("beto", Comando::SalirDeAsentamiento { asentamiento: a, heroe: b });
    assert_eq!(e.n.heroe(b).unwrap().heroe.presencia.ubicacion, Ubicacion::Columna(cb));
    assert!(e.n.en_el_mapa(cb));
    assert_eq!((e.ejercito(cb).columna.escuadras.clone(), e.ejercito(cb).columna.carga.clone(), e.ejercito(cb).posicion), (ids, carga0, e.plaza()));
    assert_eq!(e.viveres(cb), dentro, "los víveres que quedaban");
}

#[test]
fn quien_entra_solo_sale_solo_y_el_rechazo_de_la_columna_aparcada_ya_no_existe() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let a = e.a;
    // Un héroe colocado por la administración dentro de una plaza ajena a su residencia no trajo columna: sale solo, con sus víveres.
    let h = e.ciudadano("beto", Some(1));
    e.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(a as u32) });
    assert_eq!(e.n.heroe(h).unwrap().columna, None);
    e.ok("beto", Comando::SalirDeAsentamiento { asentamiento: a, heroe: h });
    let col = e.n.heroe(h).unwrap().columna.expect("nace una columna personal");
    assert_eq!((e.n.heroe(h).unwrap().heroe.presencia.ubicacion, e.ejercito(col).posicion, e.ejercito(col).columna.tipo), (Ubicacion::Columna(col), e.plaza(), TipoColumna::Personal));
    // Lo mismo en un campamento donde no reside: sale solo; y a quien no trajo nada no se le deja sacar tropa que no tiene aquí (su campamento está en su residencia).
    let z = e.ciudadano("zed", Some(1));
    e.ok("admin", Comando::FijarResidencia { heroe: z, residencia: None });
    assert_eq!(e.rechazada("zed", Comando::SalirDelCampamento { campamento: 0, heroe: z, escuadras: vec![5], carga: vec![], politica: Politica::Rechazar }), rechazos::SALIDA_SOLO_DE_TU_RESIDENCIA);
    assert_eq!(e.rechazada("zed", Comando::SalirDelCampamento { campamento: 0, heroe: z, escuadras: vec![], carga: vec![(Oro, 1)], politica: Politica::Rechazar }), rechazos::SALIDA_SOLO_DE_TU_RESIDENCIA);
    let cz = e.ok("zed", Comando::SalirDelCampamento { campamento: 0, heroe: z, escuadras: vec![], carga: vec![], politica: Politica::Rechazar }).creadas[0];
    assert!(e.n.en_el_mapa(cz));
}

// ───────────────────────────── 2. Las caravanas entran y salen con él, sean de donde sean ─────────────────────────────

impl Escenario {
    /// Una caravana de un carro con buey en `plaza`, reservada (fuera del reparto).
    fn caravana(&mut self, plaza: u64) -> u64 {
        self.dar(plaza, Cobre, 100_000);
        self.dar(plaza, Madera, 100_000);
        let id = self.ok("ana", Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
        self.ok("ana", Comando::ReservarCaravana { caravana: id, reservada: true });
        self.ok("ana", Comando::AnadirCarro { caravana: id, tipo: TipoCarro::Basico });
        self.ok("ana", Comando::AnadirAnimal { caravana: id, carro: 0, animal: Animal::Buey });
        id
    }
    fn estado_de(&self, caravana: u64) -> EstadoCaravana {
        self.n.caravana(caravana).expect("existe la caravana").estado
    }
}

#[test]
fn las_caravanas_de_otro_origen_entran_y_salen_con_el_ejercito_en_una_plaza_y_en_un_campamento() {
    let m = con_mundo!();
    let (mut e, c) = dos_plazas(m, 7);
    e.avanzar(24 * HORA);
    let a = e.a;
    let (ana, milicia) = e.con_milicia("ana");
    // La plaza C levanta un Mercado (sin él no se crean caravanas).
    e.dar(c, Madera, 350_000);
    e.ok("admin", Comando::AnadirEdificio { asentamiento: c, tipo: Tipo::Mercado });
    e.avanzar(9 * HORA);
    assert!(e.n.economia(c).unwrap().edificios.iter().any(|x| x.tipo == Tipo::Mercado && x.activo()), "el Mercado de C debería estar activo");

    // Un residente: su ejército sale de A llevando una caravana de C (otra plaza, de su misma Facción).
    let k = e.caravana(c);
    let mov = Comando::MovilizarEjercito { asentamiento: a, heroe: ana, escuadras: vec![milicia], carga: vec![], politica: Politica::Aceptar };
    let ej = e.ok("ana", mov.clone()).creadas[0];
    e.ok("admin", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });
    e.ok("admin", Comando::DarSuministro { ejercito: ej, recurso: Madera, cantidad: 30_000 });
    assert_eq!(e.n.caravana(k).unwrap().origen as u64, c, "la caravana es de C, no de la plaza donde va a entrar");
    // Antes de Entrar con todo esto se rechazaba (`caravana.ajena_al_lugar`); ahora entra con ella.
    e.ok("ana", Comando::Guarnecer { asentamiento: a, heroe: ana });
    assert_eq!((e.estado_de(k), e.ejercito(ej).caravanas_adjuntas.clone()), (EstadoCaravana::Adjunta, vec![k]), "no se aparca: sigue enganchada");
    assert!(!e.n.en_el_mapa(ej), "dentro");
    assert_eq!(e.escuadra(ana, milicia).contenedor, Contenedor::Campamento, "la milicia, a su campamento (reside aquí)");
    assert_eq!(dato(&hechos(&e.n, "ejercito.guarnece")[0], "caravanas"), 1);
    // Con la columna dentro, el carro se queda en ella (su capacidad es la de la columna con sus caravanas); `vaciarCarro` también lo vacía.
    assert_eq!((e.ejercito(ej).columna.carga.get(&Madera).copied(), e.n.heroe(ana).unwrap().carro.is_empty()), (Some(30_000), true));
    e.ok("ana", Comando::VaciarCarro { heroe: ana });
    assert_eq!((e.ejercito(ej).columna.carga.is_empty(), e.n.heroe(ana).unwrap().almacen_personal.get(&Madera).copied()), (true, Some(30_000)));
    // Sale con todo: la misma columna, con la caravana y la tropa que elija.
    let r = e.ok("ana", mov);
    assert!(r.creadas.is_empty(), "sale con la columna con la que entró");
    assert_eq!((e.ejercito(ej).caravanas_adjuntas.clone(), e.ejercito(ej).columna.escuadras.clone(), e.n.en_el_mapa(ej)), (vec![k], vec![milicia], true));
    assert_eq!(e.estado_de(k), EstadoCaravana::Adjunta);

    // Un visitante: el ejército de «beto» (Facción 1, residente del campamento 0) lleva otra caravana de C y entra de visita en A.
    e.avanzar(11 * MIN); // el cooldown de creación de caravanas de C es de 10 minutos
    let k2 = e.caravana(c);
    let (b, ids) = e.con_prestamo("beto", 1);
    let eb = e.ok("beto", Comando::SalirDelCampamento { campamento: 0, heroe: b, escuadras: ids.clone(), carga: vec![], politica: Politica::Aceptar }).creadas[0];
    e.ok("admin", Comando::AdjuntarCaravana { caravana: k2, ejercito: eb });
    // Entra en su campamento con ella enganchada (antes se rechazaba): la columna sigue dentro para llevarla y vuelve a salir con todo.
    e.ok("beto", Comando::EntrarEnCampamento { campamento: 0, heroe: b });
    assert_eq!((e.estado_de(k2), e.ejercito(eb).caravanas_adjuntas.clone(), e.n.heroe(b).unwrap().heroe.presencia.ubicacion), (EstadoCaravana::Adjunta, vec![k2], Ubicacion::Mercenarios(0)));
    assert_eq!(ids.iter().map(|&s| e.escuadra(b, s).contenedor).collect::<Vec<_>>(), vec![Contenedor::Campamento; 2]);
    let r = e.ok("beto", Comando::SalirDelCampamento { campamento: 0, heroe: b, escuadras: ids.clone(), carga: vec![], politica: Politica::Aceptar });
    assert!(r.creadas.is_empty() && e.ejercito(eb).caravanas_adjuntas == vec![k2], "retoma la misma columna con su caravana");
    e.llega("beto", eb);
    e.ok("beto", Comando::Guarnecer { asentamiento: a, heroe: b });
    let hb = e.n.heroe(b).unwrap();
    assert_eq!((hb.columna, hb.heroe.presencia.ubicacion), (Some(eb), Ubicacion::Asentamiento(a as u32)), "de visita todo queda en su columna");
    assert_eq!((e.estado_de(k2), e.ejercito(eb).caravanas_adjuntas.clone(), e.ejercito(eb).columna.tipo), (EstadoCaravana::Adjunta, vec![k2], TipoColumna::Ejercito), "sigue siendo un ejército mientras lleve la caravana");
    assert!(!e.n.en_el_mapa(eb));
    e.ok("beto", Comando::SalirDeAsentamiento { asentamiento: a, heroe: b });
    assert!(e.n.en_el_mapa(eb));
    assert_eq!((e.ejercito(eb).caravanas_adjuntas.clone(), e.estado_de(k2)), (vec![k2], EstadoCaravana::Adjunta), "salió con la caravana enganchada");
}

// ───────────────────────────── 3. El carro de la residencia ─────────────────────────────

#[test]
fn el_residente_no_vacia_el_carro_al_entrar_el_carro_ocupa_su_capacidad_y_vaciarlo_deja_lo_que_no_cabe() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let a = e.a;
    let (h, milicia) = e.con_milicia("ana");
    let sacar = |carga: Vec<(Recurso, i64)>| Comando::SalirAlMundo { asentamiento: a, heroe: h, escuadras: vec![milicia], carga };
    let col = e.ok("ana", sacar(vec![(Madera, 40_000)])).creadas[0];
    let madera_en_la_plaza = e.almacen(Madera);
    e.ok("ana", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: h });
    let hn = e.n.heroe(h).unwrap();
    assert_eq!((hn.carro.get(&Madera).copied(), hn.columna, hn.almacen_personal.is_empty()), (Some(40_000), None, true), "el carro entra con él y el almacén personal no lo recibe");
    assert_eq!(e.almacen(Madera), madera_en_la_plaza, "el almacén de la plaza tampoco");
    assert!(e.n.ejercito(col).is_none());
    assert_eq!(dato(&hechos(&e.n, "jugador.entra_en_asentamiento")[0], "carro"), 40_000);

    // El carro nunca lleva más de su capacidad (500 000): lo que ya trae cuenta al volver a salir.
    assert_eq!(e.rechazada("ana", sacar(vec![(Madera, 460_001)])), rechazos::CARRO_LLENO);
    // Lo que no cabe en el almacén personal (1 000 000 de capacidad) se queda en el carro: aquí solo caben 20 000.
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: h, recurso: Oro, cantidad: 980_000 });
    e.ok("ana", Comando::VaciarCarro { heroe: h });
    let hn = e.n.heroe(h).unwrap();
    assert_eq!((hn.almacen_personal.get(&Madera).copied(), hn.carro.get(&Madera).copied()), (Some(20_000), Some(20_000)));
    let v = hechos(&e.n, "heroe.vacia_el_carro");
    assert_eq!((dato(&v[0], "movido"), dato(&v[0], "resto"), v[0].1.clone()), (20_000, 20_000, vec![a]));
    assert_eq!(e.rechazada("ana", Comando::VaciarCarro { heroe: h }), rechazos::ALMACEN_PERSONAL_LLENO);
    // Se llevaba lo que no cupo, y sale con ello.
    let col2 = e.ok("ana", sacar(vec![])).creadas[0];
    assert_eq!(e.ejercito(col2).columna.carga.get(&Madera).copied(), Some(20_000));
    assert!(e.n.heroe(h).unwrap().carro.is_empty());

    // Quién puede vaciar un carro: solo el dueño del héroe, y solo dentro de su residencia.
    e.ok("ana", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: h });
    assert_eq!(e.rechazada("beto", Comando::VaciarCarro { heroe: h }), rechazos::NO_AUTORIZADO);
    // Un visitante tiene su carro en su columna, no en su residencia.
    let (b, ids) = e.con_prestamo("beto", 1);
    e.viaja_a_la_plaza("beto", b, &ids, &[], Politica::Rechazar);
    assert_eq!(e.rechazada("beto", Comando::VaciarCarro { heroe: b }), rechazos::VACIAR_SOLO_EN_TU_RESIDENCIA, "a la puerta, en el mapa");
    e.ok("beto", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: b });
    assert_eq!(e.rechazada("beto", Comando::VaciarCarro { heroe: b }), rechazos::VACIAR_SOLO_EN_TU_RESIDENCIA, "de visita");
}

#[test]
fn una_convocatoria_parte_con_el_carro_del_residente_y_con_la_columna_entera_del_visitante() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let a = e.a;
    let (ana, milicia) = e.con_milicia("ana");
    let col = e.ok("ana", Comando::SalirAlMundo { asentamiento: a, heroe: ana, escuadras: vec![], carga: vec![] }).creadas[0];
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 40_000 });
    e.ok("ana", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: ana });
    let (b, ids) = e.con_prestamo("beto", 1);
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: b, recurso: Oro, cantidad: 30_000 });
    let cb = e.viaja_a_la_plaza("beto", b, &ids, &[(Oro, 20_000)], Politica::Rechazar);
    e.ok("beto", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: b });

    // «ana» convoca con su milicia y «beto», de visita, se une sin elegir nada (sale con lo que trajo).
    let conv = e.ok("ana", Comando::ConvocarEjercito { heroe: ana, escuadras: vec![milicia], carga: vec![], politica: Politica::Aceptar }).creadas[0];
    e.ok("beto", Comando::UnirseAConvocatoria { heroe: b, convocatoria: conv, escuadras: vec![], carga: vec![] });
    let ejercito = e.ok("ana", Comando::PartirConvocatoria { heroe: ana }).creadas[0];
    let c = &e.ejercito(ejercito).columna;
    assert_eq!(c.participantes.iter().map(|p| p.heroe).collect::<Vec<_>>(), vec![ana, b]);
    assert_eq!((c.carga.get(&Madera).copied(), c.carga.get(&Oro).copied()), (Some(40_000), Some(20_000)), "el carro del residente y el del visitante, juntos");
    let mut esperadas = vec![milicia];
    esperadas.extend(&ids);
    assert_eq!(c.escuadras.len(), esperadas.len());
    assert!(e.n.ejercito(cb).is_none(), "la columna del visitante se fundió con el ejército");
    assert!(e.n.heroe(ana).unwrap().carro.is_empty() && e.n.en_el_mapa(ejercito));
    assert_eq!((e.n.heroe(ana).unwrap().heroe.presencia.ubicacion, e.n.heroe(b).unwrap().heroe.presencia.ubicacion), (Ubicacion::Columna(ejercito), Ubicacion::Columna(ejercito)));
}

// ───────────────────────────── 4. Quién entra: el acceso por relación ─────────────────────────────

impl Escenario {
    /// Un héroe de la cuenta con la Facción dada (o ninguna), sin columna.
    fn ciudadano(&mut self, cuenta: &str, faccion: Option<u32>) -> u64 {
        let h = self.heroe(cuenta, 0);
        let r = self.ord("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
        h
    }
}

#[test]
fn el_duenio_decide_por_relacion_quien_entra_la_misma_faccion_y_los_residentes_siempre() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let a = e.a;
    for (x, y, r) in [(1, 2, Relacion::Alianza), (1, 3, Relacion::Vasallaje), (1, 5, Relacion::Guerra)] {
        e.ok("admin", Comando::FijarRelacion { a: x, b: y, relacion: Some(r) });
    }
    let propio = e.ciudadano("p", Some(1));
    let (aliado, vasallo, neutral, enemigo, sin_faccion) = (e.ciudadano("al", Some(2)), e.ciudadano("va", Some(3)), e.ciudadano("ne", Some(4)), e.ciudadano("en", Some(5)), e.ciudadano("sf", None));
    let (residente, _) = e.con_milicia("res");
    let puede = |e: &Escenario| [propio, aliado, vasallo, neutral, enemigo, sin_faccion, residente].map(|h| e.n.puede_entrar_en_plaza(h, a));
    // Por defecto, hasta el sistema político: entran todos menos los enemigos.
    assert_eq!(e.n.acceso_de_plaza(a), Some(Acceso { aliados: true, neutrales: true, enemigos: false }));
    assert_eq!(puede(&e), [true, true, true, true, false, true, true]);

    // Solo manda quien manda en la plaza: un desconocido no, ni siquiera con una orden válida; la administración sí.
    let (v, hash) = (e.n.version(), e.n.hash_estado());
    assert_eq!(e.rechazada("zoe", Comando::FijarAcceso { asentamiento: a, aliados: false, neutrales: false, enemigos: false }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("p", Comando::FijarAcceso { asentamiento: 9_999, aliados: false, neutrales: false, enemigos: false }), rechazos::ASENTAMIENTO_DESCONOCIDO);
    assert_eq!((e.n.version(), e.n.hash_estado()), (v, hash), "un rechazo no deja rastro");
    assert_eq!(e.ord("p", Comando::FijarAcceso { asentamiento: a, aliados: true, neutrales: true, enemigos: false }).resultado, Resultado::SinCambios);

    // Aliados y vasallos van juntos.
    e.ok("p", Comando::FijarAcceso { asentamiento: a, aliados: false, neutrales: true, enemigos: false });
    assert_eq!(puede(&e), [true, false, false, true, false, true, true]);
    // Los neutrales (también quien no tiene Facción).
    e.ok("p", Comando::FijarAcceso { asentamiento: a, aliados: true, neutrales: false, enemigos: false });
    assert_eq!(puede(&e), [true, true, true, false, false, false, true]);
    // Abrir la puerta al enemigo.
    e.ok("p", Comando::FijarAcceso { asentamiento: a, aliados: true, neutrales: true, enemigos: true });
    assert_eq!(puede(&e), [true; 7]);
    // Cerrada a todos: la misma Facción y los residentes siempre pueden entrar.
    e.ok("admin", Comando::FijarAcceso { asentamiento: a, aliados: false, neutrales: false, enemigos: false });
    assert_eq!(puede(&e), [true, false, false, false, false, false, true]);
    let h = hechos(&e.n, "asentamiento.acceso");
    assert_eq!((h.len(), h[0].1.clone(), dato(&h[3], "aliados"), dato(&h[3], "enemigos")), (4, vec![a], 0, 0));
}

#[test]
fn la_orden_de_entrar_la_respeta_un_enemigo_a_la_puerta_no_entra_hasta_que_se_le_abre_y_un_ciudadano_siempre() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let a = e.a;
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 5, relacion: Some(Relacion::Guerra) });
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Alianza) });
    let (en, ie) = e.con_prestamo("en", 5);
    let ce = e.viaja_a_la_plaza("en", en, &ie, &[], Politica::Rechazar);
    let (al, ia) = e.con_prestamo("al", 2);
    let ca = e.viaja_a_la_plaza("al", al, &ia, &[], Politica::Rechazar);
    let (ci, ic) = e.con_prestamo("ci", 1);
    let cc = e.viaja_a_la_plaza("ci", ci, &ic, &[], Politica::Rechazar);
    assert_eq!(e.rechazada("en", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: en }), rechazos::PUERTA_CERRADA);
    let (v, hash) = (e.n.version(), e.n.hash_estado());
    assert_eq!(e.rechazada("en", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: en }), rechazos::PUERTA_CERRADA);
    assert_eq!((e.n.version(), e.n.hash_estado()), (v, hash), "el rechazo no deja rastro");
    assert!(e.n.en_el_mapa(ce), "se queda fuera, en el mapa");
    // El aliado y el ciudadano entran; cerrar la puerta a los aliados echa al mapa, al instante, al que ya estaba dentro (cambia desde el 2026-10-22: antes sólo decidía quién
    // entraba) y el siguiente ya no entra. El ciudadano, que es de la misma Facción, no se echa nunca.
    e.ok("al", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: al });
    assert!(!e.n.en_el_mapa(ca));
    e.ok("ci", Comando::FijarAcceso { asentamiento: a, aliados: false, neutrales: true, enemigos: false });
    assert_eq!(e.n.heroe(al).unwrap().heroe.presencia.ubicacion, Ubicacion::Columna(ca), "el que estaba dentro sale al mapa con su columna");
    assert!(e.n.en_el_mapa(ca));
    assert_eq!(e.rechazada("al", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: al }), rechazos::PUERTA_CERRADA, "y ya no vuelve a entrar");
    e.ok("ci", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: ci });
    assert!(e.n.en_el_mapa(ca) && !e.n.en_el_mapa(cc));
    // Se abre la puerta a los enemigos y el enemigo entra.
    e.ok("ci", Comando::FijarAcceso { asentamiento: a, aliados: true, neutrales: true, enemigos: true });
    e.ok("en", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: en });
    assert!(!e.n.en_el_mapa(ce));
}

#[test]
fn una_foto_con_el_acceso_cambiado_se_recupera_y_la_anterior_se_rechaza() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(HORA);
    e.ok("admin", Comando::FijarAcceso { asentamiento: e.a, aliados: false, neutrales: true, enemigos: true });
    let bytes = e.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN21");
    let vuelta = nucleo::Estado::deserializar(&bytes, &Reglas::default(), &e.n.estado().mundo_hash).unwrap();
    assert_eq!(vuelta.asentamientos[&e.a].acceso, Acceso { aliados: false, neutrales: true, enemigos: true });
    assert_eq!((vuelta.serializar(), vuelta.hash()), (bytes.clone(), e.n.estado().hash()));
    let viejo = [b"BAN16".as_slice(), &bytes[5..]].concat();
    let Err(err) = nucleo::Estado::deserializar(&viejo, &Reglas::default(), &e.n.estado().mundo_hash) else { panic!("BAN16 no debe admitirse") };
    assert!(err.to_string().contains("BAN16") && err.to_string().contains("acceso"), "{err}");
    // BAN17 (sin héroes dentro de pueblos ni avanzadas) tampoco se admite, y dice por qué.
    let viejo = [b"BAN17".as_slice(), &bytes[5..]].concat();
    let Err(err) = nucleo::Estado::deserializar(&viejo, &Reglas::default(), &e.n.estado().mundo_hash) else { panic!("BAN17 no debe admitirse") };
    assert!(err.to_string().contains("BAN17") && err.to_string().contains("pueblo") && err.to_string().contains("avanzada"), "{err}");
}

// ───────────────────────────── 5. Cuando el lugar cae en ruinas ─────────────────────────────

#[test]
fn al_arruinarse_la_plaza_el_visitante_sale_al_mapa_con_su_columna_y_su_carga_sin_perder_nada() {
    let m = con_mundo!();
    let (mut e, c) = dos_plazas(m, 7);
    let a = e.a;
    let pa = e.plaza();
    // La plaza A no construye nada: sin madera para su mantenimiento acabará en ruinas.
    e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
    let ana = e.residente("ana");
    // «beto», de visita, con su columna, su tropa y 20 000 de oro en el carro; «ana», residente, con el suyo.
    let (b, ids) = e.con_prestamo("beto", 1);
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: b, recurso: Oro, cantidad: 30_000 });
    let cb = e.viaja_a_la_plaza("beto", b, &ids, &[(Oro, 20_000)], Politica::Rechazar);
    e.ok("beto", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: b });
    let col = e.ok("ana", Comando::SalirAlMundo { asentamiento: a, heroe: ana, escuadras: vec![], carga: vec![] }).creadas[0];
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 40_000 });
    e.ok("ana", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: ana });
    assert!(e.n.ejercito(col).is_none());
    assert_eq!(e.n.heroe(ana).unwrap().carro.get(&Madera).copied(), Some(40_000));

    let mut ruina = None;
    for dia in 2..=150 {
        e.n.avanzar_hasta(dia * 24 * HORA);
        if e.n.economia(a).is_none() {
            ruina = Some(hechos(&e.n, "asentamiento.ruinas")[0].0);
            break;
        }
    }
    let ruina = ruina.expect("la plaza A cae en ruinas");
    // «beto» sale al mapa con la misma columna y su carga; nadie queda en un campamento ni sin sitio.
    let hb = e.n.heroe(b).unwrap();
    assert_eq!((hb.columna, hb.heroe.presencia.ubicacion), (Some(cb), Ubicacion::Columna(cb)));
    assert!(e.n.en_el_mapa(cb));
    assert_eq!((e.ejercito(cb).columna.carga.get(&Oro).copied(), e.ejercito(cb).posicion), (Some(20_000), pa));
    // «ana» (residente) sale con una columna nueva que lleva su carro; su residencia pasa a la plaza hermana.
    let ha = e.n.heroe(ana).unwrap();
    let Ubicacion::Columna(nueva) = ha.heroe.presencia.ubicacion else { panic!("«ana» sale al mapa") };
    assert_eq!((ha.heroe.residencia, e.ejercito(nueva).columna.carga.get(&Madera).copied(), e.ejercito(nueva).posicion, ha.carro.is_empty()), (Some(Residencia::Asentamiento(c as u32)), Some(40_000), pa, true));
    let reubicados = hechos(&e.n, "heroe.reubicado");
    assert!(reubicados.len() >= 2 && reubicados.iter().all(|r| r.0 == ruina && dato(r, "alMapa") == 1));
}

// ───────────────────────────── 6. Foto y determinismo ─────────────────────────────

impl Escenario {
    /// Como `con_milicia`, pero con cada orden en el instante `t`.
    fn milicia_en(&mut self, t: Instante, cuenta: &str) -> (u64, u64) {
        let a = self.a as u32;
        let h = self.ok_en(t, cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano }).creadas[0];
        self.ok_en(t, "admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
        self.ok_en(t, "admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(a)) });
        self.ok_en(t, "admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(a) });
        let r = self.ok_en(t, cuenta, Comando::ReclutarTropa { asentamiento: self.a, heroe: h, tropa: Tropa::MiliciaLanceros });
        (h, r.creadas[0])
    }
    /// Como `con_prestamo`, pero con cada orden en el instante `t`.
    fn prestamo_en(&mut self, t: Instante, cuenta: &str, faccion: u32) -> (u64, Vec<u64>) {
        let h = self.ok_en(t, cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano }).creadas[0];
        self.ok_en(t, "admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(faccion) });
        let ids = self.ok_en(t, cuenta, Comando::PedirPrestamo { heroe: h, tropas: vec![Tropa::Granjeros, Tropa::Mineros] }).creadas;
        (h, ids)
    }
}

/// Entrar con todo, fijar el acceso, vaciar el carro y salir, con cada orden en su instante.
fn guion(e: &mut Escenario, avanzar: &dyn Fn(&mut Escenario, Instante)) {
    let base = 24 * HORA;
    avanzar(e, base);
    let a = e.a;
    let (ana, milicia) = e.milicia_en(base, "ana");
    let col = e.ok_en(base, "ana", Comando::SalirAlMundo { asentamiento: a, heroe: ana, escuadras: vec![milicia], carga: vec![] }).creadas[0];
    e.ok_en(base, "admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 40_000 });
    let (b, ids) = e.prestamo_en(base, "beto", 1);
    e.ok_en(base, "admin", Comando::DarAlmacenPersonal { heroe: b, recurso: Oro, cantidad: 30_000 });
    let cb = e.ok_en(base, "beto", Comando::SalirDelCampamento { campamento: 0, heroe: b, escuadras: ids, carga: vec![(Oro, 20_000)], politica: Politica::Rechazar }).creadas[0];
    let destino = e.plaza();
    e.ok_en(base, "beto", Comando::MarcharA { ejercito: cb, destino });
    let fin = e.ejercito(cb).plan.fin().unwrap();
    avanzar(e, fin);
    e.ok_en(fin, "ana", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: ana });
    e.ok_en(fin, "beto", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: b });
    avanzar(e, fin + 10 * MIN);
    e.ok_en(fin + 10 * MIN, "ana", Comando::FijarAcceso { asentamiento: a, aliados: false, neutrales: true, enemigos: false });
    avanzar(e, fin + 2 * HORA);
    e.ok_en(fin + 2 * HORA, "ana", Comando::VaciarCarro { heroe: ana });
    e.ok_en(fin + 2 * HORA, "beto", Comando::SalirDeAsentamiento { asentamiento: a, heroe: b });
    avanzar(e, fin + 3 * HORA);
    e.ok_en(fin + 3 * HORA, "ana", Comando::SalirAlMundo { asentamiento: a, heroe: ana, escuadras: vec![milicia], carga: vec![] });
    avanzar(e, fin + 5 * HORA);
    e.n.avanzar_hasta(fin + 5 * HORA);
}

fn correr(m: &M, semilla: u64, avanzar: &dyn Fn(&mut Escenario, Instante)) -> Nucleo {
    let mut e = escenario(m, semilla);
    guion(&mut e, avanzar);
    e.n
}

#[test]
fn mismas_ordenes_dan_el_mismo_hash_normal_acelerado_y_a_trozos() {
    let m = con_mundo!();
    let normal = correr(m, 3, &|e, t| e.n.avanzar_hasta(t));
    assert_eq!(normal.hash_estado(), correr(m, 3, &|e, t| e.n.avanzar_hasta(t)).hash_estado(), "repetible");
    assert_eq!(normal.hash_estado(), correr(m, 3, &|_, _| {}).hash_estado(), "acelerado de un golpe");
    let a_trozos = correr(m, 3, &|e, t| {
        let ahora = e.n.ahora();
        for s in 1..=5 {
            e.n.avanzar_hasta(ahora + (t - ahora).max(0) * s * s / 25);
        }
    });
    assert_eq!(normal.hash_estado(), a_trozos.hash_estado(), "en pasos pequeños e irregulares, como el modo en vivo");
    // El guion hizo de verdad lo que se espera.
    for h in ["jugador.entra_en_asentamiento", "asentamiento.acceso", "heroe.vacia_el_carro", "jugador.sale_de_asentamiento"] {
        assert!(!hechos(&normal, h).is_empty(), "el guion no produjo «{h}»");
    }
    let ana = normal.heroe_de_cuenta("ana").unwrap();
    assert_eq!(ana.carro.len(), 0, "vació y volvió a salir");
    assert_eq!(ana.almacen_personal.get(&Madera).copied(), Some(40_000));
    // Nada recurre por una entidad ociosa: como mucho un evento de deserción vigente por columna.
    let vigentes = normal.estado().cola.volcar().iter().filter(|ev| normal.estado().generaciones.es_vigente(ev.clave) && matches!(ev.carga, Carga::Militar(SucesoMilitar::Desercion))).count();
    assert!(vigentes <= normal.estado().ejercitos.len(), "{vigentes} deserciones vigentes");
    let mut sin_guion = escenario(m, 3);
    sin_guion.avanzar(29 * HORA);
    assert_ne!(normal.hash_estado(), sin_guion.n.hash_estado());
}

#[test]
fn una_foto_con_un_visitante_dentro_y_un_carro_en_la_residencia_recupera_el_mismo_mundo() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9) };
    // Antes de la foto: «ana» entró con su carro y «beto» de visita con su columna. Después: el acceso, vaciar, salir y el hambre del visitante dentro.
    let primera = |e: &mut Escenario| {
        e.avanzar(24 * HORA);
        let a = e.a;
        let (ana, milicia) = e.con_milicia("ana");
        let col = e.ok("ana", Comando::SalirAlMundo { asentamiento: a, heroe: ana, escuadras: vec![milicia], carga: vec![] }).creadas[0];
        e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 40_000 });
        let (b, ids) = e.con_prestamo("beto", 1);
        let cb = e.viaja_a_la_plaza("beto", b, &ids, &[], Politica::Rechazar);
        e.ok("ana", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: ana });
        e.ok("beto", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: b });
        (ana, b, cb)
    };
    let segunda = |e: &mut Escenario, ana: u64, b: u64| {
        let a = e.a;
        e.avanzar(3 * HORA);
        e.ok("ana", Comando::FijarAcceso { asentamiento: a, aliados: true, neutrales: false, enemigos: true });
        e.ok("ana", Comando::VaciarCarro { heroe: ana });
        e.avanzar(40 * HORA);
        e.ok("beto", Comando::SalirDeAsentamiento { asentamiento: a, heroe: b });
        e.avanzar(HORA);
    };
    let sin_parar = {
        let mut e = escenario(m, 9);
        let (ana, b, _) = primera(&mut e);
        e.avanzar(HORA);
        segunda(&mut e, ana, b);
        (e.n.hash_estado(), e.n.estado().serializar())
    };
    let (a_mitad, ana, b, cuenta) = {
        let mut n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        let p = sitio_rico(&n, &m.0, 0, 1);
        let a = fundar(&mut n, 0, "ana", 1, p);
        let mut e = Escenario { n, a, cuenta: 0 };
        let (ana, b, cb) = primera(&mut e);
        assert!(e.n.ejercito(cb).is_some() && !e.n.en_el_mapa(cb));
        let bytes = e.n.estado().serializar();
        let vuelta = nucleo::Estado::deserializar(&bytes, &Reglas::default(), &e.n.estado().mundo_hash).unwrap();
        assert_eq!((vuelta.serializar(), vuelta.hash()), (bytes, e.n.estado().hash()));
        e.n.foto_ahora();
        e.avanzar(HORA);
        e.n.apagar().unwrap();
        (e.n.hash_estado(), ana, b, e.cuenta)
    };
    let n = Nucleo::con_persistencia(con(ajustes), dir.path(), Default::default()).unwrap();
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), a_mitad, "foto + registro reconstruyen el mismo mundo, con el carro de «ana» y la columna de «beto» dentro");
    let a = *n.estado().asentamientos.keys().next().unwrap();
    let mut e = Escenario { n, a, cuenta };
    segunda(&mut e, ana, b);
    assert_eq!(e.n.hash_estado(), sin_parar.0);
    assert_eq!(e.n.estado().serializar(), sin_parar.1, "el estado recuperado sigue igual que el que no se interrumpió");
    e.n.apagar().unwrap();
}

// ───────────────────────────── 7. Cerrar la puerta expulsa ─────────────────────────────

#[test]
fn cerrar_la_puerta_saca_al_mapa_al_instante_y_con_todo_lo_suyo_a_quien_ya_no_puede_entrar_pero_no_a_la_misma_faccion_ni_a_los_residentes() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let a = e.a;
    for (x, y, r) in [(1, 2, Relacion::Alianza), (1, 5, Relacion::Guerra)] {
        e.ok("admin", Comando::FijarRelacion { a: x, b: y, relacion: Some(r) });
    }
    // Con la puerta abierta a todos entran de visita, con su columna, un aliado (F2), un neutral (F4), un enemigo (F5) y uno de la propia Facción (F1).
    e.ok("admin", Comando::FijarAcceso { asentamiento: a, aliados: true, neutrales: true, enemigos: true });
    let mut visitas = vec![];
    for (cuenta, faccion) in [("al", 2), ("ne", 4), ("en", 5), ("ci", 1)] {
        let (h, ids) = e.con_prestamo(cuenta, faccion);
        e.ok("admin", Comando::DarAlmacenPersonal { heroe: h, recurso: Oro, cantidad: 30_000 });
        let col = e.viaja_a_la_plaza(cuenta, h, &ids, &[(Oro, 20_000)], Politica::Rechazar);
        e.ok(cuenta, Comando::EntrarEnAsentamiento { asentamiento: a, heroe: h });
        visitas.push((cuenta, h, col));
    }
    let [(_, al, cal), (_, ne, cne), (_, en, cen), (_, ci, cci)] = visitas[..] else { unreachable!() };
    // Un residente dentro y un neutral que la administración colocó dentro sin columna (no trajo nada).
    let (res, _) = e.con_milicia("res");
    let sf = e.ciudadano("sf", Some(4));
    e.ok("admin", Comando::ColocarHeroe { heroe: sf, ubicacion: Ubicacion::Asentamiento(a as u32) });
    let foto = |e: &Escenario, col: u64| (e.ejercito(col).columna.carga.clone(), e.ejercito(col).columna.escuadras.clone(), e.viveres(col));
    let (f_ne, f_en) = (foto(&e, cne), foto(&e, cen));
    let dentro = |e: &Escenario, h: u64| e.n.heroe(h).unwrap().heroe.presencia.ubicacion == Ubicacion::Asentamiento(a as u32);
    assert!([al, ne, en, ci, sf, res].iter().all(|&h| dentro(&e, h)));

    // Se cierra a neutrales y enemigos: salen los dos visitantes con su columna y el que no trajo ninguna con una nueva; los demás siguen dentro.
    let (t, v) = (e.n.ahora(), e.n.version());
    e.ok("admin", Comando::FijarAcceso { asentamiento: a, aliados: true, neutrales: false, enemigos: false });
    assert_eq!(e.n.version(), v + 1, "una sola mutación");
    assert!([al, ci, res].iter().all(|&h| dentro(&e, h)), "el aliado, el de la propia Facción y el residente siguen");
    assert!(!e.n.en_el_mapa(cal) && !e.n.en_el_mapa(cci));
    for (h, col, antes) in [(ne, cne, f_ne), (en, cen, f_en)] {
        assert_eq!(e.n.heroe(h).unwrap().heroe.presencia.ubicacion, Ubicacion::Columna(col), "sale con la columna con la que entró");
        assert!(e.n.en_el_mapa(col));
        assert_eq!((foto(&e, col), e.ejercito(col).posicion), (antes, e.plaza()), "con la tropa, el carro y los víveres de antes, a la puerta");
    }
    let hs = e.n.heroe(sf).unwrap();
    let Ubicacion::Columna(nueva) = hs.heroe.presencia.ubicacion else { panic!("sale al mapa: {:?}", hs.heroe.presencia.ubicacion) };
    assert_eq!((hs.columna, e.ejercito(nueva).columna.faccion, e.ejercito(nueva).columna.tipo, e.ejercito(nueva).posicion), (Some(nueva), Some(4), TipoColumna::Personal, e.plaza()));
    let echados = hechos(&e.n, "heroe.expulsado");
    assert_eq!(echados.iter().map(|h| dato(h, "heroe")).collect::<Vec<_>>(), vec![ne as i64, en as i64, sf as i64], "por id, en el mismo instante");
    assert!(echados.iter().all(|h| (h.0, h.1.clone(), dato(h, "lugar"), dato(h, "id"), dato(h, "motivo")) == (t, vec![a], 0, a as i64, 0)));
    // Quien salió no vuelve a entrar.
    assert_eq!(e.rechazada("ne", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: ne }), rechazos::PUERTA_CERRADA);

    // Se cierra también a los aliados: sale «al»; la misma Facción y el residente nunca.
    e.ok("admin", Comando::FijarAcceso { asentamiento: a, aliados: false, neutrales: false, enemigos: false });
    assert_eq!(e.n.heroe(al).unwrap().heroe.presencia.ubicacion, Ubicacion::Columna(cal));
    assert!(e.n.en_el_mapa(cal) && dentro(&e, ci) && dentro(&e, res) && !e.n.en_el_mapa(cci));
    assert_eq!(hechos(&e.n, "heroe.expulsado").len(), 4);

    // Abrir la puerta no saca a nadie, y repetir lo mismo no cambia nada.
    e.ok("admin", Comando::FijarAcceso { asentamiento: a, aliados: true, neutrales: true, enemigos: true });
    assert_eq!(e.ord("admin", Comando::FijarAcceso { asentamiento: a, aliados: true, neutrales: true, enemigos: true }).resultado, Resultado::SinCambios);
    assert_eq!((hechos(&e.n, "heroe.expulsado").len(), dentro(&e, ci), dentro(&e, res)), (4, true, true));
}

// ───────────────────────────── 8. Cuando el lugar cae por conquista (Doc 5 §5.4 y §5.15.5, Combate §9) ─────────────────────────────

impl Escenario {
    /// Un ejército de «gus» (Facción 2) a 2 u de la puerta de la plaza `a`, con esas tropas: (columna, héroe).
    fn atacante(&mut self, tropas: &[Tropa]) -> (u64, u64) {
        let (t, p) = (self.n.ahora(), self.plaza());
        let col = self.n.crear_ejercito(t, "gus", 2, Punto::new(p.x + 2_000, p.y), 500, 100_000).unwrap();
        let h = self.ejercito(col).columna.lider;
        for &tropa in tropas {
            self.ok("admin", Comando::DarEscuadra { heroe: h, tropa, cantidad: mr().tropas.tropa(tropa).unidades });
        }
        (col, h)
    }
    /// El loadout activo del héroe lleva esas escuadras: las que defienden en persona.
    fn loadout(&mut self, h: u64, escuadras: Vec<u64>) {
        let loadout = self.n.heroe(h).unwrap().heroe.loadouts[0].id;
        self.ok("admin", Comando::GuardarLoadout { heroe: h, loadout: Some(loadout), nombre: "Default".into(), escuadras, perks: vec![], activo: Some(true) });
    }
    /// Un Barracón en la plaza (que sube a nivel 2): el cupo de 7 de Liderazgo para la guarnición.
    fn con_barracon(&mut self) {
        let a = self.a;
        self.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel: 2 });
        self.dar(a, Madera, 1_000_000);
        self.ok("admin", Comando::AnadirEdificio { asentamiento: a, tipo: Tipo::Barracon });
        let activo = |e: &Escenario| e.n.economia(a).unwrap().edificios.iter().any(|x| x.tipo == Tipo::Barracon && x.activo());
        for _ in 0..80 {
            if activo(self) {
                break;
            }
            self.avanzar(HORA);
        }
        assert!(activo(self), "el Barracón acaba por levantarse");
    }
    /// El héroe sale con la carga dada y vuelve a entrar en su residencia sin vaciarla: su carro se queda con él.
    fn con_carro(&mut self, cuenta: &str, h: u64, carga: &[(Recurso, i64)]) {
        let a = self.a;
        let col = self.ok(cuenta, Comando::SalirAlMundo { asentamiento: a, heroe: h, escuadras: vec![], carga: carga.to_vec() }).creadas[0];
        self.ok(cuenta, Comando::EntrarEnAsentamiento { asentamiento: a, heroe: h });
        assert!(self.n.ejercito(col).is_none());
    }
}

#[test]
fn el_desalojo_de_una_conquista_cumple_el_canon_el_defensor_sale_con_lo_que_uso_el_resto_a_cero_a_la_plaza_mas_cercana_y_el_residente_pierde_la_mitad_del_carro() {
    let m = con_mundo!();
    let (mut e, c) = dos_plazas(m, 7);
    e.avanzar(24 * HORA);
    let a = e.a;
    e.con_barracon();
    // «ana», residente: una milicia que defiende (en su loadout), unos peltastas que se quedan en el campamento, unos leñadores en la guarnición y 40 001 de madera y 10 000 de piedra en el carro.
    let (ana, milicia) = e.con_milicia("ana");
    let peltastas = e.ok("admin", Comando::DarEscuadra { heroe: ana, tropa: Tropa::Peltastas, cantidad: 10 }).creadas[0];
    let lenadores = e.ok("admin", Comando::DarEscuadra { heroe: ana, tropa: Tropa::Lenadores, cantidad: 30 }).creadas[0];
    e.ok("ana", Comando::AsignarGuarnicion { heroe: ana, escuadra: lenadores });
    e.loadout(ana, vec![milicia]);
    e.dar(a, Madera, 100_000);
    e.dar(a, Piedra, 100_000);
    e.con_carro("ana", ana, &[(Madera, 40_001), (Piedra, 10_000)]);
    // «bea», residente que está fuera (en el mapa, con su milicia en la columna): solo cambia de casa. «cai», residente dentro pero herida: no defiende y sale con su carro entero.
    let bea = e.residente("bea");
    let mb = e.ok("admin", Comando::DarEscuadra { heroe: bea, tropa: Tropa::MiliciaLanceros, cantidad: 25 }).creadas[0];
    let cb = e.ok("bea", Comando::SalirAlMundo { asentamiento: a, heroe: bea, escuadras: vec![mb], carga: vec![] }).creadas[0];
    let cai = e.residente("cai");
    let mc = e.ok("admin", Comando::DarEscuadra { heroe: cai, tropa: Tropa::MiliciaLanceros, cantidad: 25 }).creadas[0];
    e.loadout(cai, vec![mc]);
    e.con_carro("cai", cai, &[(Madera, 30_000)]);
    // «vis», de otra Facción, de visita con su columna y 20 000 de oro: sale con ella sin perder nada.
    let (vis, ids) = e.con_prestamo("vis", 3);
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: vis, recurso: Oro, cantidad: 30_000 });
    let cv = e.viaja_a_la_plaza("vis", vis, &ids, &[(Oro, 20_000)], Politica::Rechazar);
    e.ok("vis", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: vis });
    e.ok("admin", Comando::HerirHeroe { heroe: cai });
    let personal = e.n.heroe(ana).unwrap().almacen_personal.clone();
    let (carro_ana, carro_cai, carga_vis) = (e.n.heroe(ana).unwrap().carro.clone(), e.n.heroe(cai).unwrap().carro.clone(), e.ejercito(cv).columna.carga.clone());
    assert_eq!(carro_ana.get(&Madera), Some(&40_001));

    // Un ejército de F2 asedia la plaza y gana.
    let (cg, gus) = e.atacante(&[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto, Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto]);
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    let t = e.n.ahora();
    e.ok("gus", Comando::Atacar { heroe: gus, objetivo: nucleo::ObjetivoAtaque::Asentamiento(a), declara_guerra: false });
    let conquista = &hechos(&e.n, "asentamiento.conquistada")[0];
    assert_eq!((conquista.0, dato(conquista, "faccion")), (t, 2), "la plaza cae");

    // 1. Cada héroe defensor queda fuera, en el mundo, con las escuadras que usó en la batalla (las que sobrevivieron), en su propia columna y con su carro.
    let ha = e.n.heroe(ana).unwrap();
    let Ubicacion::Columna(sale) = ha.heroe.presencia.ubicacion else { panic!("«ana» queda fuera: {:?}", ha.heroe.presencia.ubicacion) };
    assert_eq!((e.ejercito(sale).columna.escuadras.clone(), e.ejercito(sale).posicion, e.ejercito(sale).columna.faccion), (vec![milicia], e.plaza(), Some(1)));
    let usada = e.escuadra(ana, milicia);
    assert!(usada.cantidad > 0 && usada.cantidad < 25 && usada.contenedor == Contenedor::Ejercito(sale), "sobrevivió con bajas y va en su columna: {usada:?}");
    assert!(e.n.heroe_herido(ana), "los defensores en persona quedan heridos");
    // 2. Su residencia pasa a la plaza más cercana de su Facción (la otra plaza F1), con todo su campamento a 0: lo que no usó y la guarnición, sin captura.
    assert_eq!(ha.heroe.residencia, Some(Residencia::Asentamiento(c as u32)), "el asentamiento más cercano de su Facción");
    for (id, que) in [(peltastas, "los peltastas del campamento"), (lenadores, "los leñadores de la guarnición")] {
        let s = e.escuadra(ana, id);
        assert_eq!((s.cantidad, s.contenedor, s.en_guarnicion), (0, Contenedor::Campamento, false), "{que} quedan a 0 y vuelven a su campamento: no hay captura");
    }
    assert_eq!(ha.almacen_personal, personal, "su almacén personal viaja con ella");
    // 3. El carro del residente que defendió: pierde la mitad (hacia abajo, recurso a recurso) y es del vencedor; la regla del derrotado, que el canon no desmiente.
    let carga_sale = e.ejercito(sale).columna.carga.clone();
    assert_eq!((carga_sale.get(&Madera).copied(), carga_sale.get(&Piedra).copied()), (Some(40_001 - 20_000), Some(5_000)), "40 001 de madera → 20 001; 10 000 de piedra → 5 000");
    assert!(e.n.heroe(ana).unwrap().carro.is_empty(), "el carro de su residencia pasó a la columna con la que sale");
    let robado = 20_000 + 5_000;
    assert_eq!(dato(conquista, "carroPerdido"), robado);
    let del_vencedor = e.ejercito(cg).columna.carga.clone();
    assert_eq!((del_vencedor.get(&Piedra).copied().unwrap_or(0) >= 5_000, del_vencedor.get(&Madera).copied().unwrap_or(0) >= 20_000), (true, true), "el vencedor carga lo robado (y la chatarra): {del_vencedor:?}");
    // 4. «bea», fuera, solo se muda: su columna y su milicia no se tocan; su campamento, a 0. «cai», herida, no defendió: sale con su carro entero.
    assert_eq!(e.n.heroe(bea).unwrap().heroe.residencia, Some(Residencia::Asentamiento(c as u32)));
    assert_eq!((e.n.heroe(bea).unwrap().heroe.presencia.ubicacion, e.escuadra(bea, mb).cantidad), (Ubicacion::Columna(cb), 25));
    let hc = e.n.heroe(cai).unwrap();
    let Ubicacion::Columna(sale_cai) = hc.heroe.presencia.ubicacion else { panic!("«cai» sale al mapa") };
    assert_eq!((e.ejercito(sale_cai).columna.carga.clone(), e.ejercito(sale_cai).columna.escuadras.is_empty(), hc.heroe.residencia), (carro_cai, true, Some(Residencia::Asentamiento(c as u32))), "sale sola con su carro entero");
    assert_eq!(e.escuadra(cai, mc).cantidad, 0, "su milicia, que no combatió, corre la suerte del resto del campamento");
    // 5. El visitante sale con la columna con la que entró, sin perder nada ni cambiar de casa.
    assert_eq!((e.n.heroe(vis).unwrap().heroe.presencia.ubicacion, e.ejercito(cv).columna.carga.clone(), e.n.en_el_mapa(cv)), (Ubicacion::Columna(cv), carga_vis, true));
    assert_eq!(e.n.heroe(vis).unwrap().heroe.residencia, Some(Residencia::Campamento(0)));
    // Nadie queda dentro de la plaza conquistada y se cuenta a cada uno.
    assert!(e.n.heroes().all(|h| h.heroe.presencia.ubicacion != Ubicacion::Asentamiento(a as u32)));
    assert_eq!(hechos(&e.n, "heroe.desalojado").len(), 4, "ana, bea, cai y vis");
}

#[test]
fn un_residente_que_conservaba_su_columna_para_las_caravanas_y_defiende_sale_con_ella_y_con_las_escuadras_que_uso() {
    let m = con_mundo!();
    let (mut e, c) = dos_plazas(m, 7);
    e.avanzar(24 * HORA);
    let a = e.a;
    let (ana, milicia) = e.con_milicia("ana");
    e.loadout(ana, vec![milicia]);
    // La plaza C levanta un Mercado (sin él no hay caravanas) y «ana» entra en A con una caravana de C: su columna sigue (sin tropa) para llevarla.
    e.dar(c, Madera, 350_000);
    e.ok("admin", Comando::AnadirEdificio { asentamiento: c, tipo: Tipo::Mercado });
    e.avanzar(9 * HORA);
    let k = e.caravana(c);
    let ej = e.ok("ana", Comando::MovilizarEjercito { asentamiento: a, heroe: ana, escuadras: vec![milicia], carga: vec![], politica: Politica::Aceptar }).creadas[0];
    e.ok("admin", Comando::AdjuntarCaravana { caravana: k, ejercito: ej });
    e.ok("admin", Comando::DarSuministro { ejercito: ej, recurso: Madera, cantidad: 30_000 });
    e.ok("ana", Comando::Guarnecer { asentamiento: a, heroe: ana });
    assert_eq!((e.n.heroe(ana).unwrap().columna, e.escuadra(ana, milicia).contenedor), (Some(ej), Contenedor::Campamento), "la milicia vuelve al campamento y defiende desde allí");

    let (_, gus) = e.atacante(&[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto, Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto]);
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    e.ok("gus", Comando::Atacar { heroe: gus, objetivo: nucleo::ObjetivoAtaque::Asentamiento(a), declara_guerra: false });
    assert_eq!(e.n.estado().asentamientos[&a].faccion, 2, "la plaza cae");
    // Sale con la columna que conservaba —la caravana sigue enganchada— y con la milicia que defendió y sobrevivió; la mitad del carro, para el vencedor.
    let ha = e.n.heroe(ana).unwrap();
    assert_eq!((ha.heroe.presencia.ubicacion, ha.columna, ha.heroe.residencia), (Ubicacion::Columna(ej), Some(ej), Some(Residencia::Asentamiento(c as u32))));
    assert!(e.n.en_el_mapa(ej));
    assert_eq!((e.ejercito(ej).caravanas_adjuntas.clone(), e.estado_de(k)), (vec![k], EstadoCaravana::Adjunta));
    let s = e.escuadra(ana, milicia);
    assert!(s.cantidad > 0 && s.cantidad < 25 && s.contenedor == Contenedor::Ejercito(ej), "la milicia que defendió va en su columna: {s:?}");
    assert_eq!(e.ejercito(ej).columna.escuadras, vec![milicia]);
    assert_eq!(e.ejercito(ej).columna.carga.get(&Madera).copied(), Some(15_000), "la mitad de los 30 000 del carro");
    assert!(e.n.heroe(ana).unwrap().carro.is_empty());
}
