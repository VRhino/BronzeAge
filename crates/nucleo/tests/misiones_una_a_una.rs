//! Las misiones de hoy, una a una (mecánica 59, `Docs/Mechanics/Misiones.md`): cada misión del catálogo se cumple haciendo lo que dice su texto
//! (`Misiones_Textos.md`). Las cadenas y sus previas ya las prueba `tests/misiones.rs`; aquí el catálogo va **sin previas**, para que cada misión esté activa desde
//! el principio y cada prueba mire solo la suya: sin cumplir antes del hecho y cumplida después. Sobre el artefacto de mundo real; las pruebas se omiten si no está.

mod comun;
use asentamiento::catalogo::Tipo;
use asentamiento::edificio::Edificios;
use asentamiento::Recurso;
use base::{Punto, MS_POR_MINUTO};
use comun::real::{self, campamento, pisable, sitio_rico, M};
use militar::catalogo::Tropa;
use militar::columna::Politica;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::misiones::Dueno;
use nucleo::{Ajustes, Arranque, Comando, Nucleo, ObjetivoAtaque, Peticion, Reglas, Respuesta};
use std::sync::Arc;
use tecnologia::datos::{Capitulo, DefinicionEpica, HechoEpico, Logro};
use tecnologia::Contador;

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;

macro_rules! con_mundo {
    () => {
        match real::mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

/// Las reglas de siempre con el catálogo de misiones sin previas, y sin bandidos que acosen o aparezcan solos (no son de estas pruebas).
fn reglas() -> Reglas {
    let mut r = Reglas::default();
    for x in &mut Arc::make_mut(&mut r.economia.misiones).misiones {
        x.previa = None;
    }
    let b = &mut Arc::make_mut(&mut r.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    // Almacenes grandes (las plazas con la construcción parada viven de lo que da la administración) y la nobleza con un solo ciudadano (A10).
    let d = Arc::make_mut(&mut r.economia.datos);
    d.capacidad_inicial = 100_000_000;
    d.poblacion.min_ciudadanos_nobleza = 1;
    // La cantería aparece con una plaza de nivel 2 (F5) y tiene una épica (F12), como en `tests/aedas_residentes.rs`.
    let t = Arc::make_mut(&mut r.economia.tecnologia);
    let c = t.buscar("canteria").unwrap();
    t.catalogo[c as usize].logro = Some(Logro { contador: Contador::PlazasEnNivel2, umbral: 1 });
    t.catalogo[c as usize].epica = Some(DefinicionEpica {
        nombre: "Canto de la piedra".into(),
        capitulos: vec![Capitulo { hecho: HechoEpico::Obra, cantidad: 1, edificio: Some(Tipo::Almacen) }, Capitulo { hecho: HechoEpico::Obra, cantidad: 1, edificio: None }],
    });
    t.indexar();
    r
}

struct X {
    n: Nucleo,
    cuenta: u64,
}

impl X {
    fn nuevo(m: &M) -> X {
        let n = Nucleo::nuevo(Arranque { reglas: reglas(), ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
        X { n, cuenta: 0 }
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
    fn avanzar(&mut self, ms: i64) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    /// Avanza de `paso` en `paso`, como mucho `veces`, hasta que se cumple `listo`.
    fn hasta(&mut self, paso: i64, veces: u32, listo: impl Fn(&X) -> bool) -> bool {
        for _ in 0..veces {
            if listo(self) {
                return true;
            }
            self.avanzar(paso);
        }
        listo(self)
    }
    fn cumplida(&self, d: Dueno, id: &str) -> bool {
        let r = &self.n.estado().reglas.economia.misiones;
        self.n.misiones_de(d).is_some_and(|x| x.cumplidas.iter().any(|&i| r.misiones[i as usize].id == id))
    }
    /// La misión `id` de `d` no está cumplida antes de `hecho` y sí después.
    fn cumple(&mut self, d: Dueno, id: &str, hecho: impl FnOnce(&mut X)) {
        assert!(!self.cumplida(d, id), "{id} ya estaba cumplida antes de su hecho");
        hecho(self);
        assert!(self.cumplida(d, id), "{id} no se cumple con su hecho: {:?}", self.n.misiones_de(d));
    }
    fn heroe(&mut self, cuenta: &str, campamento: u32) -> u64 {
        self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento, controlador: Controlador::Humano, nombre: None }).creadas[0]
    }
    /// Una columna de administración de la cuenta en `p`: (columna, héroe).
    fn columna(&mut self, cuenta: &str, faccion: u32, p: Punto, viveres: i64) -> (u64, u64) {
        let t = self.n.ahora();
        let col = self.n.crear_ejercito(t, cuenta, faccion, p, 500, viveres).unwrap();
        (col, self.n.ejercito(col).unwrap().columna.lider)
    }
    /// Una plaza de la Facción `faccion` fundada por `cuenta` junto al campamento `i`.
    fn plaza(&mut self, m: &M, cuenta: &str, faccion: u32, i: usize) -> u64 {
        let p = sitio_rico(&self.n, &m.0, i, faccion);
        real::fundar(&mut self.n, cuenta, faccion, p)
    }
    /// Un héroe de la cuenta, ciudadano de la Facción de la plaza, que reside en ella y está dentro.
    fn heroe_en(&mut self, cuenta: &str, plaza: u64) -> u64 {
        let h = self.heroe(cuenta, 0);
        let f = self.n.estado().asentamientos[&plaza].faccion;
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(f) });
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
        h
    }
    /// De todo para semanas: sin auto-construcción la plaza no se sostiene sola.
    fn llenar(&mut self, plaza: u64) {
        for r in asentamiento::recurso::TODOS {
            let _ = self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso: r, cantidad: 20_000_000 });
        }
    }
    /// Una plaza de la Facción 1 de «ana» con la construcción automática parada y el nivel dado (la administración lo fija).
    fn plaza_quieta(&mut self, m: &M, nivel: u32) -> u64 {
        let a = self.plaza(m, "ana", 1, 0);
        self.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
        if nivel > 1 {
            self.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel });
        }
        self.llenar(a);
        a
    }
    /// Encarga el edificio y espera a que esté activo.
    fn levantar(&mut self, plaza: u64, tipo: Tipo) {
        self.llenar(plaza);
        self.ok("admin", Comando::AnadirEdificio { asentamiento: plaza, tipo });
        let listo = self.hasta(HORA, 24 * 6, |x| x.n.economia(plaza).is_some_and(|e| e.edificios.activos(tipo) > 0));
        assert!(listo, "{tipo:?} no se levantó a tiempo");
    }
    fn rey(&self, cuenta: &str) -> u64 {
        self.n.heroes().find(|h| h.cuenta == cuenta).unwrap().heroe.id
    }
    fn posicion(&self, plaza: u64) -> Punto {
        self.n.estado().asentamientos[&plaza].posicion
    }
}

// ───────────────────────────── Héroe ─────────────────────────────

#[test]
fn h1_salir_al_mundo() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let h = x.heroe("ana", 0);
    x.cumple(Dueno::Heroe(h), "H1", |x| {
        x.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: h, escuadras: vec![], carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None });
    });
}

#[test]
fn h2_pedir_tropa_prestada() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let h = x.heroe("ana", 0);
    x.cumple(Dueno::Heroe(h), "H2", |x| {
        x.ok("ana", Comando::PedirPrestamo { heroe: h, tropas: vec![Tropa::Granjeros] });
    });
}

#[test]
fn h3_llegar_al_final_del_camino() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let h = x.heroe("ana", 0);
    let col = x.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: h, escuadras: vec![], carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None }).creadas[0];
    let c0 = campamento(&m.0, 0);
    let marcha = (1..40).flat_map(|k| (0..8).map(move |g| (k, g))).any(|(k, g)| {
        let a = (g as f64) * std::f64::consts::FRAC_PI_4;
        let destino = Punto::new(c0.x + (k as f64 * 5_000.0 * a.cos()) as i64, c0.y + (k as f64 * 5_000.0 * a.sin()) as i64);
        x.ord("ana", Comando::MarcharA { ejercito: col, destino }).aceptada()
    });
    assert!(marcha, "hay algún destino por tierra cerca del campamento");
    let fin = x.n.ejercito(col).unwrap().plan.fin().unwrap();
    x.cumple(Dueno::Heroe(h), "H3", |x| x.n.avanzar_hasta(fin));
}

#[test]
fn h4_repostar_en_una_plaza() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza(m, "ana", 1, 0);
    x.ok("admin", Comando::DarRecursos { asentamiento: a, recurso: Recurso::Trigo, cantidad: 1_000_000 });
    let (_, h) = x.columna("ana", 1, x.posicion(a), 50_000);
    x.cumple(Dueno::Heroe(h), "H4", |x| {
        x.ok("ana", Comando::Repostar { heroe: h, asentamiento: a });
    });
}

#[test]
fn h5_entrar_en_una_faccion() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let h = x.heroe("ana", 0);
    x.cumple(Dueno::Heroe(h), "H5", |x| {
        x.ok("ana", Comando::CrearFaccion { heroe: h, nombre: "Micenas".into(), sigilo: None });
    });
    // Y entrando en una que ya existe.
    let b = x.heroe("beto", 0);
    let f = x.n.heroe(h).unwrap().faccion.unwrap();
    x.cumple(Dueno::Heroe(b), "H5", |x| {
        x.ok("beto", Comando::SolicitarIngreso { heroe: b, faccion: f });
        x.ok("ana", Comando::ResponderSolicitud { heroe: h, faccion: f, solicitante: b, aceptar: true });
    });
}

#[test]
fn h6_reclutar_una_escuadra_nueva() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza(m, "ana", 1, 0);
    x.avanzar(24 * HORA); // población que no haga falta para producir
    let h = x.heroe_en("ana", a);
    x.cumple(Dueno::Heroe(h), "H6", |x| {
        x.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::MiliciaLanceros });
    });
}

#[test]
fn h7_reclutar_un_ejercito_de_verdad() {
    // 50 de infantería (milicia y granjeros, del Centro Urbano), 20 de tiro (honderos, de la Galería de tiro) y 10 de caballería (exploradores, de las Caballerizas).
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza_quieta(m, 2);
    x.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["hostigamiento_tribal".into(), "cria_caballar".into()] });
    for t in [Tipo::GaleriaDeTiro, Tipo::Caballerizas] {
        x.levantar(a, t);
    }
    // Población que no haga falta para producir: la plaza vuelve a construir sola (Viviendas) unos días.
    x.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: false });
    for _ in 0..6 {
        x.avanzar(24 * HORA);
        x.llenar(a);
    }
    let h = x.heroe_en("ana", a);
    x.cumple(Dueno::Heroe(h), "H7", |x| {
        for tropa in [Tropa::MiliciaLanceros, Tropa::Granjeros, Tropa::Honderos, Tropa::ExploradoresCaballo] {
            let r = x.ord("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa });
            assert!(r.aceptada(), "{tropa:?}: {r:?} con {:?}", x.n.economia(a).unwrap().poblacion);
        }
    });
}

#[test]
fn h8_unirse_a_un_ejercito() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    x.plaza(m, "ana", 1, 0);
    let (a, b) = (x.heroe("ana", 0), x.heroe("beto", 0));
    for h in [a, b] {
        x.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
    }
    let ej = x.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: a, escuadras: vec![], carga: vec![], politica: Politica::Aceptar, caravana_fundacion: None }).creadas[0];
    x.ok("beto", Comando::SalirDelCampamento { campamento: 0, heroe: b, escuadras: vec![], carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None });
    x.cumple(Dueno::Heroe(b), "H8", |x| {
        x.ok("beto", Comando::UnirseEnCampo { heroe: b, ejercito: ej });
    });
}

/// Un campamento de bandidos de nivel 1 junto a una columna de `cuenta` con una escuadra completa de arqueros; devuelve (campamento, héroe).
fn bandidos_y_arqueros(x: &mut X, m: &M, cuenta: &str, faccion: u32, donde: Punto) -> (u64, u64) {
    let (_, h) = x.columna(cuenta, faccion, donde, 350_000);
    let unidades = militar::Reglas::cargar().tropas.tropa(Tropa::ArquerosCompuesto).unidades;
    x.ok("admin", Comando::DarEscuadra { heroe: h, tropa: Tropa::ArquerosCompuesto, cantidad: unidades });
    let cerca = pisable(&m.0, donde, 1_000, 0);
    let b = x.ok("admin", Comando::ColocarBandidos { posicion: cerca, nivel: 1 }).creadas[0];
    (b, h)
}

/// Lejos de los campamentos de mercenarios y de las plazas: un sitio para los bandidos.
fn descampado(m: &M, k: i64) -> Punto {
    pisable(&m.0, campamento(&m.0, 0), 40_000 + k * 8_000, 30_000)
}

#[test]
fn h9_combatir_en_una_batalla() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (b, h) = bandidos_y_arqueros(&mut x, m, "ana", 1, descampado(m, 0));
    x.cumple(Dueno::Heroe(h), "H9", |x| {
        x.ok("ana", Comando::Atacar { heroe: h, objetivo: ObjetivoAtaque::Campamento(b), declara_guerra: false });
    });
}

#[test]
fn h10_destruir_un_campamento_de_bandidos() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (b, h) = bandidos_y_arqueros(&mut x, m, "ana", 1, descampado(m, 0));
    x.cumple(Dueno::Heroe(h), "H10", |x| {
        x.ok("ana", Comando::Atacar { heroe: h, objetivo: ObjetivoAtaque::Campamento(b), declara_guerra: false });
        assert!(x.n.campamento_de_bandidos(b).is_none(), "el campamento cae");
    });
}

#[test]
fn h11_abrir_un_alijo() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let puerta = campamento(&m.0, 0);
    let (alijo, sitio) = m.0.alijos().iter().enumerate().map(|(i, a)| (i as u32, Punto::new(a.posicion.unwrap().x, a.posicion.unwrap().y))).min_by_key(|(_, p)| p.dist2(puerta)).expect("hay alijos");
    let (_, h) = x.columna("ana", 1, sitio, 100_000);
    x.cumple(Dueno::Heroe(h), "H11", |x| {
        x.ok("ana", Comando::AbrirAlijo { heroe: h, alijo });
    });
}

#[test]
fn h12_entregar_materiales_a_una_obra() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza(m, "ana", 1, 0);
    // La aldea agrícola marcada en el sitio del panel; una columna de «beto», de la Facción, a su puerta con el material.
    let punto = x.n.sitio_de_aldea(a).expect("hay sitio de aldea");
    let pueblo = x.ok("admin", Comando::MarcarAldea { asentamiento: a, punto }).creadas[0];
    let sitio = x.n.posicion_de_pueblo(pueblo).unwrap();
    let (col, h) = x.columna("beto", 1, sitio, 100_000);
    for (recurso, cantidad) in x.n.pueblo(pueblo).unwrap().1.falta() {
        x.ok("admin", Comando::DarSuministro { ejercito: col, recurso, cantidad });
    }
    x.cumple(Dueno::Heroe(h), "H12", |x| {
        x.ok("admin", Comando::EntregarMateriales { ejercito: col, pueblo });
    });
}

#[test]
fn h13_repartir_los_puntos_de_atributo() {
    // Hoy el héroe no gana puntos de atributo (el servidor no sube su nivel: `tests/heroes.rs`), así que H13 no tiene camino en la partida (parada hasta que haya
    // Unity para los perks, decisión del autor del 2026-10-28). Lo que sí se prueba: sin
    // puntos, repartir se rechaza y no cuenta.
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let h = x.heroe("ana", 0);
    assert!(!x.ord("ana", Comando::RepartirPuntos { heroe: h, fuerza: 1, destreza: 0, armadura: 0, vitalidad: 0 }).aceptada());
    assert!(!x.cumplida(Dueno::Heroe(h), "H13"));
}

// ───────────────────────────── Plaza ─────────────────────────────

#[test]
fn a1_fundar_una_aldea_agricola() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza(m, "ana", 1, 0);
    let d = Dueno::Asentamiento(a);
    x.cumple(d, "A1", |x| {
        comun::levantar_aldea(&mut x.n, "ana", a).expect("hay sitio de aldea");
        assert!(x.hasta(MIN, 24 * 60, |x| x.cumplida(d, "A1")), "la aldea acaba su obra");
    });
}

#[test]
fn a2_a3_un_mercado_y_un_barracon_activos() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza_quieta(m, 2);
    let d = Dueno::Asentamiento(a);
    x.cumple(d, "A2", |x| x.levantar(a, Tipo::Mercado));
    x.cumple(d, "A3", |x| x.levantar(a, Tipo::Barracon));
}

#[test]
fn a4_subir_la_plaza_a_nivel_2_pidiendolo() {
    // Como `tests/asentamientos.rs`: la plaza vive sola, el jugador pide el ascenso cada hora y levanta su aldea agrícola el primer día.
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza(m, "ana", 1, 0);
    let d = Dueno::Asentamiento(a);
    x.cumple(d, "A4", |x| {
        for hora in 1..=12 * 24 {
            x.avanzar(HORA);
            if hora == 24 {
                comun::levantar_aldea(&mut x.n, "ana", a).expect("hay sitio fértil para la aldea");
            }
            let _ = x.ord("admin", Comando::SolicitarAscenso { asentamiento: a });
            if x.cumplida(d, "A4") {
                break;
            }
        }
        assert_eq!(x.n.economia(a).unwrap().nivel, 2);
    });
}

#[test]
fn a9_subir_la_plaza_a_nivel_3() {
    // El nivel 3 por la administración (el ascenso de verdad lo prueba A4): también lo mira.
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza_quieta(m, 2);
    x.cumple(Dueno::Asentamiento(a), "A9", |x| {
        x.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel: 3 });
    });
}

#[test]
fn a5_completar_un_pueblo() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza_quieta(m, 2);
    for _ in 0..9 {
        x.ok("admin", Comando::AgrandarZona { asentamiento: a });
    }
    let mapa = nucleo::economia::Mapa::de(&m.0);
    let nodos: Vec<u32> = (0..mapa.nodos.len() as u32).filter(|&i| x.n.en_zona(a, mapa.nodos[i as usize].pos)).collect();
    let pueblo = nodos
        .into_iter()
        .find_map(|nodo| {
            let r = x.ord("admin", Comando::MarcarPueblo { asentamiento: a, nodo });
            r.aceptada().then(|| r.creadas[0])
        })
        .expect("hay un nodo en la zona donde marcar un pueblo");
    let sitio = x.n.posicion_de_pueblo(pueblo).unwrap();
    let (col, _) = x.columna("ana", 1, sitio, 100_000);
    for (recurso, cantidad) in x.n.pueblo(pueblo).unwrap().1.falta() {
        x.ok("admin", Comando::DarSuministro { ejercito: col, recurso, cantidad });
    }
    let d = Dueno::Asentamiento(a);
    x.cumple(d, "A5", |x| {
        x.ok("admin", Comando::EntregarMateriales { ejercito: col, pueblo });
        assert!(x.hasta(MIN, 24 * 60, |x| x.cumplida(d, "A5")), "el pueblo acaba su obra");
    });
}

#[test]
fn a6_tener_artesanos() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza_quieta(m, 2);
    let d = Dueno::Asentamiento(a);
    x.cumple(d, "A6", |x| {
        x.levantar(a, Tipo::Carpinteria);
        assert!(x.hasta(HORA, 48, |x| x.cumplida(d, "A6")), "llegan los primeros artesanos: {:?}", x.n.economia(a).unwrap().poblacion);
    });
}

#[test]
fn a7_completar_un_recinto_de_la_muralla() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza_quieta(m, 2);
    let d = Dueno::Asentamiento(a);
    x.cumple(d, "A7", |x| {
        x.ok("admin", Comando::ComprometerRecinto { asentamiento: a, cargo: asentamiento::Cargo::MaestroObras, nivel: 1 });
        assert!(x.hasta(HORA, 48, |x| x.cumplida(d, "A7")), "la empalizada se cierra");
    });
}

#[test]
fn a8_completar_una_avanzada() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza_quieta(m, 2);
    for _ in 0..9 {
        x.ok("admin", Comando::AgrandarZona { asentamiento: a });
    }
    let sitio = real::en_tierra(&m.0, x.posicion(a), 8_000, 0);
    let (col, h) = x.columna("ana", 1, sitio, 100_000);
    comun::coloca_avanzadas(&mut x.n, a, h);
    let av = x.ok("ana", Comando::ColocarAvanzada { asentamiento: a, heroe: h, posicion: sitio }).creadas[0];
    let d = Dueno::Asentamiento(a);
    x.cumple(d, "A8", |x| {
        for _ in 0..40 {
            let falta = x.n.avanzada(av).unwrap().1.obra.as_ref().map(|o| o.falta()).unwrap_or_default();
            if falta.is_empty() {
                break;
            }
            for (recurso, cantidad) in falta {
                let _ = x.ord("admin", Comando::DarSuministro { ejercito: col, recurso, cantidad });
            }
            x.ok("ana", Comando::EntregarMaterialesAAvanzada { ejercito: col, avanzada: av });
        }
        assert!(x.hasta(HORA, 48, |x| x.cumplida(d, "A8")), "la avanzada acaba su obra");
    });
}

#[test]
fn a10_tener_nobles() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza_quieta(m, 2);
    x.levantar(a, Tipo::Almacen);
    let d = Dueno::Asentamiento(a);
    x.cumple(d, "A10", |x| {
        x.levantar(a, Tipo::Palacio);
        assert!(x.hasta(MIN, 120, |x| x.cumplida(d, "A10")), "nace la nobleza");
    });
}

/// Dos plazas de Facciones distintas: la de «ana» (F1) junto al campamento 0 y la de «beto» (F2) junto al 3.
fn dos_plazas(x: &mut X, m: &M) -> (u64, u64) {
    let a = x.plaza(m, "ana", 1, 0);
    let b = x.plaza(m, "beto", 2, 3);
    for p in [a, b] {
        x.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: p, pausada: true });
        x.llenar(p);
    }
    (a, b)
}

/// Un asedio de «ana» a la plaza B con lo que lleve de ataque; «beto» la defiende dentro con lo que lleve de defensa.
fn asediar(x: &mut X, m: &M, b: u64, ataque: &[Tropa], defensa: &[Tropa]) {
    let h = x.heroe("beto", 3);
    x.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(2) });
    x.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(b as u32)) });
    x.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(b as u32) });
    let unidades = |t: Tropa| militar::Reglas::cargar().tropas.tropa(t).unidades;
    if !defensa.is_empty() {
        let ids: Vec<u64> = defensa.iter().map(|&t| x.ok("admin", Comando::DarEscuadra { heroe: h, tropa: t, cantidad: unidades(t) }).creadas[0]).collect();
        let loadout = x.n.heroe(h).unwrap().heroe.loadouts[0].id;
        x.ok("admin", Comando::GuardarLoadout { heroe: h, loadout: Some(loadout), nombre: "Default".into(), escuadras: ids, perks: vec![], activo: Some(true) });
    }
    let pb = x.posicion(b);
    let (_, ana) = x.columna("ana", 1, Punto::new(pb.x + 2_000, pb.y), 350_000);
    for &t in ataque {
        x.ok("admin", Comando::DarEscuadra { heroe: ana, tropa: t, cantidad: unidades(t) });
    }
    let _ = m;
    x.ok("ana", Comando::Atacar { heroe: ana, objetivo: ObjetivoAtaque::Asentamiento(b), declara_guerra: true });
    let fin = x.n.batallas_activas().filter_map(|b| b.convocatoria_hasta).max();
    if let Some(fin) = fin {
        x.n.avanzar_hasta(fin);
    }
}

#[test]
fn a11_resistir_un_asedio() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (_, b) = dos_plazas(&mut x, m);
    x.avanzar(25 * HORA);
    let d = Dueno::Asentamiento(b);
    x.cumple(d, "A11", |x| {
        asediar(x, m, b, &[Tropa::MiliciaLanceros], &[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto]);
        assert_eq!(x.n.estado().asentamientos[&b].faccion, 2, "la plaza resiste");
    });
}

// ───────────────────────────── Facción ─────────────────────────────

#[test]
fn f1_la_primera_plaza_es_la_capital() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let h = x.heroe("ana", 0);
    x.ok("ana", Comando::CrearFaccion { heroe: h, nombre: "Micenas".into(), sigilo: None });
    let f = x.n.heroe(h).unwrap().faccion.unwrap();
    x.cumple(Dueno::Faccion(f), "F1", |x| {
        let p = sitio_rico(&x.n, &m.0, 0, f);
        real::fundar(&mut x.n, "ana", f, p);
    });
}

#[test]
fn f2_nombrar_un_embajador() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza(m, "ana", 1, 0);
    let rey = x.rey("ana");
    let bea = x.heroe_en("bea", a);
    x.cumple(Dueno::Faccion(1), "F2", |x| {
        x.ok("ana", Comando::AsignarEmbajador { heroe: rey, faccion: 1, designado: bea });
    });
}

#[test]
fn f3_f4_fundar_dos_y_cinco_plazas() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let d = Dueno::Faccion(1);
    // El cupo de plazas de la Facción sube con su nivel (Diplomacia_Facciones §3.5): la administración lo fija.
    let fundar = |x: &mut X, i: usize, cuenta: &str| {
        let p = sitio_rico(&x.n, &m.0, i, 1);
        real::fundar(&mut x.n, cuenta, 1, p);
    };
    fundar(&mut x, 0, "ana");
    x.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 10 });
    x.cumple(d, "F3", |x| fundar(x, 1, "ana2"));
    for (i, cuenta) in [(2, "ana3"), (3, "ana4")] {
        fundar(&mut x, i, cuenta);
    }
    x.cumple(d, "F4", |x| fundar(x, 4, "ana5"));
}

#[test]
fn f5_adoptar_una_tecnologia() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza(m, "ana", 1, 0);
    let rey = x.rey("ana");
    x.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel: 2 });
    let canteria = x.n.estado().reglas.economia.tecnologia.buscar("canteria").unwrap();
    let aparecida = |x: &X| x.n.tecnologia().de(&x.n.estado().reglas.economia.tecnologia, 1).aparecidas.contiene(canteria);
    assert!(x.hasta(HORA, 24 * 5, aparecida), "aparece la cantería");
    x.ok("ana", Comando::Guarnecer { asentamiento: a, heroe: rey });
    x.llenar(a);
    x.cumple(Dueno::Faccion(1), "F5", |x| {
        x.ok("ana", Comando::AdoptarTecnologia { heroe: rey, faccion: 1, tecnologia: canteria });
    });
}

#[test]
fn f6_activar_una_politica() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza(m, "ana", 1, 0);
    x.cumple(Dueno::Faccion(1), "F6", |x| {
        x.ok("admin", Comando::ActivarPolitica { asentamiento: a, politica: "racionamiento".into(), cargo: asentamiento::Cargo::Sacerdote });
    });
}

/// Un trueque de cobre por estaño entre A y B, con una caravana en cada plaza: se propone, se acepta y se espera a que se cumpla.
fn trueque(x: &mut X, a: u64, b: u64) {
    for (p, r) in [(a, Recurso::Cobre), (b, Recurso::Estano)] {
        x.ok("admin", Comando::DarRecursos { asentamiento: p, recurso: r, cantidad: 100_000 });
    }
    let tr = x.ok("admin", Comando::ProponerTrueque { a, b, lineas_a: vec![(Recurso::Cobre, 10_000)], lineas_b: vec![(Recurso::Estano, 10_000)] }).creadas[0];
    x.ok("admin", Comando::AceptarTrueque { acuerdo: tr });
    let cumplido = x.hasta(HORA, 24 * 10, |x| x.n.acuerdo(tr).is_none_or(|t| t.estado == comercio::trueque::EstadoTrueque::Cumplido));
    assert!(cumplido, "el trueque se cumple: {:?}", x.n.acuerdo(tr));
}

#[test]
fn f7_f8_cumplir_uno_y_cinco_trueques_con_otra_faccion() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (a, b) = dos_plazas(&mut x, m);
    for p in [a, b] {
        use comercio::caravana::{Animal, TipoCarro};
        x.levantar(p, Tipo::Mercado);
        let c = x.ok("admin", Comando::CrearCaravana { asentamiento: p }).creadas[0];
        x.ok("admin", Comando::AnadirCarro { caravana: c, tipo: TipoCarro::Basico });
        x.ok("admin", Comando::AnadirAnimal { caravana: c, carro: 0, animal: Animal::Buey });
    }
    let d = Dueno::Faccion(1);
    x.cumple(d, "F7", |x| trueque(x, a, b));
    assert!(x.cumplida(Dueno::Faccion(2), "F7"), "cuenta para las dos");
    for _ in 0..3 {
        trueque(&mut x, a, b);
    }
    x.cumple(d, "F8", |x| trueque(x, a, b));
}

#[test]
fn f9_aceptar_una_alianza() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    dos_plazas(&mut x, m);
    let (ra, rb) = (x.rey("ana"), x.rey("beto"));
    x.cumple(Dueno::Faccion(1), "F9", |x| {
        let p = x.ok("ana", Comando::ProponerRelacion { heroe: ra, a: 1, b: 2, vasallaje: false, tributo: None }).creadas[0];
        x.ok("beto", Comando::ResponderRelacion { heroe: rb, propuesta: p, aceptar: true });
    });
    assert!(x.cumplida(Dueno::Faccion(2), "F9"), "cuenta para las dos");
}

#[test]
fn f10_destruir_cinco_campamentos_de_bandidos() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    x.plaza(m, "ana", 1, 0);
    let d = Dueno::Faccion(1);
    let destruir = |x: &mut X, k: i64| {
        let (b, h) = bandidos_y_arqueros(x, m, &format!("h{k}"), 1, descampado(m, k));
        x.ok(&format!("h{k}"), Comando::Atacar { heroe: h, objetivo: ObjetivoAtaque::Campamento(b), declara_guerra: false });
        assert!(x.n.campamento_de_bandidos(b).is_none());
    };
    for k in 0..4 {
        destruir(&mut x, k);
    }
    x.cumple(d, "F10", |x| destruir(x, 4));
}

#[test]
fn f11_conquistar_una_plaza() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (_, b) = dos_plazas(&mut x, m);
    x.avanzar(25 * HORA);
    x.cumple(Dueno::Faccion(1), "F11", |x| {
        asediar(x, m, b, &[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto], &[]);
        assert_eq!(x.n.estado().asentamientos[&b].faccion, 1, "la plaza cae");
    });
}

#[test]
fn f12_empezar_una_epica() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza_quieta(m, 2);
    let rey = x.rey("ana");
    x.levantar(a, Tipo::Almacen);
    x.levantar(a, Tipo::Palacio);
    x.ok("admin", Comando::Guarnecer { asentamiento: a, heroe: rey });
    assert!(x.hasta(HORA, 24 * 3, |x| !x.n.residentes().aedas.is_empty()), "llega el primer Aeda");
    let canteria = x.n.estado().reglas.economia.tecnologia.buscar("canteria").unwrap();
    x.cumple(Dueno::Faccion(1), "F12", |x| {
        x.ok("ana", Comando::EmpezarEpica { heroe: rey, asentamiento: a, aeda: 1, tecnologia: canteria });
    });
}

/// Las misiones que prueba este archivo, una a una: todo el catálogo. Una misión nueva sin su prueba aquí la hace fallar.
const PROBADAS: &[&str] = &[
    "H1", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9", "H10", "H11", "H12", "H13", "A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A9", "A10", "A11", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8",
    "F9", "F10", "F11", "F12",
];

#[test]
fn cada_mision_del_catalogo_tiene_su_prueba() {
    let ids: Vec<String> = nucleo::misiones::Reglas::cargar().misiones.into_iter().map(|m| m.id).collect();
    assert_eq!(ids, PROBADAS);
}
