//! Los pueblos extractores y su carro de acarreo (fase 3C, Economia_Construccion §14, Doc 4.2.2; Comercio_Caravanas segunda §12): la obra de
//! mapa que se marca y se levanta con materiales entregados en el sitio, la extracción contra el almacén local, el carro que sale lleno hacia
//! la plaza madre, la ruina y la conquista de la madre, determinismo y recuperación desde foto. Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;
use asentamiento::catalogo::Tipo;
use asentamiento::pueblos::{EstadoPueblo, IngresoAcarreo};
use asentamiento::Recurso::{self, *};
use base::{Instante, Punto, MS_POR_MINUTO};
use comercio::caravana::{EstadoCaravana, TipoCaravana};
use mundo::{Mundo, OpcionesValidacion};
use nucleo::economia::Mapa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::{rechazos, Ajustes, Arranque, Carga, Comando, Nucleo, Peticion, Reglas, Resultado, Respuesta, SucesoMilitar};
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

/// Un sitio fundable, verde y con mucho bosque, a `r` del punto `c`: una plaza que no cae en ruinas por falta de madera.
/// Con tierra de cultivo en la zona inicial (algo de fertilidad a 6 u o menos): sin ella las Granjas no dan nada y la plaza se queda sin gente.
fn fertil(m: &Mundo, p: Punto) -> bool {
    (0..40).map(|i| (i as f64 / 40.0 * std::f64::consts::TAU, 6_000.0 * (i % 5) as f64 / 5.0)).any(|(a, r)| m.fertilidad_en(p.x + (r * a.cos()) as i64, p.y + (r * a.sin()) as i64).is_some_and(|f| f >= 77))
}

fn sitio_junto_a(n: &Nucleo, m: &Mundo, c: Punto, r: i64) -> Option<Punto> {
    (0..36).map(|g| ((g * 10) as f64).to_radians()).map(|a| Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64)).find(|&p| {
        n.viabilidad(p, 1).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6 && !v.condenada) && fertil(m, p)
    })
}

/// La plaza madre de Ana (F1) a 6 u de un nodo de cobre, ya de nivel 2, con la auto-construcción parada y la zona agrandada para que el nodo
/// quede dentro. El tiempo corre solo cuando se pide.
struct Escenario {
    n: Nucleo,
    plaza: u64,
    /// Índice (en `Mapa::nodos`) del nodo de cobre junto a la plaza.
    cobre: u32,
    mapa: Arc<Mapa>,
    cuenta: u64,
}

impl Escenario {
    fn peticion(&mut self, actor: &str, c: Comando) -> Peticion {
        self.cuenta += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.cuenta), c);
        // Andamiaje (5E): una orden de plaza de quien mandaba en ella antes del sistema 5 se da como administración (`comun::mandaba`).
        if actor == "admin" || comun::mandaba(&self.n, actor, &p.comando) {
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
    fn dar(&mut self, recurso: Recurso, cantidad: i64) {
        let plaza = self.plaza;
        let r = self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
    }
    fn almacen(&self, r: Recurso) -> i64 {
        self.n.economia(self.plaza).unwrap().almacen.cantidad(r)
    }
    /// Una columna de `faccion` en `p`.
    fn columna(&mut self, dueno: &str, faccion: u32, p: Punto) -> u64 {
        let t = self.n.ahora();
        self.n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap()
    }
    /// Marca un pueblo de cobre en el nodo de la plaza.
    fn marcar(&mut self) -> u64 {
        let (plaza, nodo) = (self.plaza, self.cobre);
        self.ok("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo }).creadas[0]
    }
    fn pueblo(&self, id: u64) -> &asentamiento::pueblos::Pueblo {
        self.n.pueblo(id).expect("el pueblo existe").1
    }
    /// Una columna de Ana en la puerta del pueblo con todo el material de su obra (100 de madera y 12 de oro).
    fn llevar_materiales(&mut self, pueblo: u64) -> u64 {
        let sitio = self.n.posicion_de_pueblo(pueblo).unwrap();
        let col = self.columna("ana", 1, sitio);
        self.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 100_000 });
        self.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 12_000 });
        col
    }
    /// Marca un pueblo, lo levanta con todo el material y espera a que acabe la obra. Devuelve su id.
    fn pueblo_levantado(&mut self) -> u64 {
        let id = self.marcar();
        let col = self.llevar_materiales(id);
        self.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
        let EstadoPueblo::EnObra { completa_en } = self.pueblo(id).estado else { panic!("debía estar en obra") };
        self.n.avanzar_hasta(completa_en + MIN);
        assert!(self.pueblo(id).activo());
        id
    }
}

fn montar(m: &M, semilla: u64, nivel: u32) -> Escenario {
    montar_con(Nucleo::nuevo(arranque(m, semilla)), m, nivel)
}

fn montar_con(n: Nucleo, m: &M, nivel: u32) -> Escenario {
    montar_donde(n, m, nivel, |_| true)
}

/// Como [`montar_con`], con una condición más sobre el sitio de la plaza.
fn montar_donde(n: Nucleo, m: &M, nivel: u32, vale: impl Fn(Punto) -> bool) -> Escenario {
    let mapa = Mapa::de(&m.0);
    let (cobre, sitio) = (0..mapa.nodos.len())
        .filter(|&i| mapa.nodos[i].tipo == Tipo::MinaCobre)
        .find_map(|i| sitio_junto_a(&n, &m.0, mapa.nodos[i].pos, 6_000).filter(|&p| vale(p)).map(|p| (i as u32, p)))
        .expect("hay un nodo de cobre junto a un sitio rico");
    montar_en(n, mapa, nivel, cobre, sitio)
}

/// La plaza madre de Ana fundada en `sitio`, con el nodo `nodo` como el de su pueblo (ver [`montar`]).
fn montar_en(mut n: Nucleo, mapa: Arc<Mapa>, nivel: u32, cobre: u32, sitio: Punto) -> Escenario {
    let e = n.crear_ejercito(0, "ana", 1, sitio, 500, 100_000).unwrap();
    assert!(n.aplicar_en(0, Peticion::nueva("admin", "c", Comando::DarCaravanaFundacion { ejercito: e }).de_admin()).aceptada());
    let r = n.aplicar_en(0, Peticion::nueva("ana", "f", Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    let plaza = r.creadas[0];
    let mut esc = Escenario { n, plaza, cobre, mapa, cuenta: 0 };
    esc.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
    // La puerta de antes del sistema 5 (abierta también a los neutrales, que por defecto ya no entran: Q1, 5E): en estas pruebas los neutrales entran.
    esc.ok("admin", comun::acceso(plaza, true, true, false));
    if nivel > 1 {
        esc.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel });
        // El techo de la zona sube con el nivel: se agranda hasta el del nivel 2 (15 u) para tener nodos con los que elegir.
        for _ in 0..9 {
            esc.ok("admin", Comando::AgrandarZona { asentamiento: plaza });
        }
    }
    assert!(esc.n.en_zona(plaza, esc.mapa.nodos[cobre as usize].pos) || nivel == 1, "el nodo de cobre cae en la zona");
    esc
}

#[test]
fn un_pueblo_hereda_el_acceso_de_su_plaza_madre() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let pueblo = e.marcar();
    let sitio = e.n.posicion_de_pueblo(pueblo).unwrap();
    // Un héroe de cada relación con la Facción 1: la propia, una aliada, una sin relación y una en guerra.
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(nucleo::Relacion::Alianza) });
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 4, relacion: Some(nucleo::Relacion::Guerra) });
    let mut heroes = vec![];
    for (c, f) in [("pro", 1), ("ali", 2), ("neu", 3), ("ene", 4)] {
        let col = e.columna(c, f, sitio);
        heroes.push(e.n.ejercito(col).unwrap().columna.lider);
    }
    let del_pueblo = |e: &Escenario| heroes.iter().map(|&h| e.n.puede_entrar_en_pueblo(h, pueblo)).collect::<Vec<_>>();
    let de_la_madre = |e: &Escenario| heroes.iter().map(|&h| Some(e.n.puede_entrar_en_plaza(h, plaza))).collect::<Vec<_>>();
    // Por defecto entran todos menos los enemigos, y el pueblo dice lo mismo que su plaza madre.
    assert_eq!(del_pueblo(&e), vec![Some(true), Some(true), Some(true), Some(false)]);
    assert_eq!(del_pueblo(&e), de_la_madre(&e));
    // Lo que el dueño de la plaza decide cambia a la vez a la plaza y a su pueblo.
    e.ok("admin", comun::acceso(plaza, true, false, true));
    assert_eq!(del_pueblo(&e), vec![Some(true), Some(true), Some(false), Some(true)]);
    assert_eq!(del_pueblo(&e), de_la_madre(&e));
    e.ok("admin", comun::acceso(plaza, false, false, false));
    assert_eq!(del_pueblo(&e), vec![Some(true), Some(false), Some(false), Some(false)], "la misma Facción entra siempre");
    assert_eq!(e.n.puede_entrar_en_pueblo(heroes[0], 9_999), None, "un pueblo que no existe no tiene puerta");
}

#[test]
fn el_almacen_local_del_pueblo_es_del_tamano_del_carro_de_acarreo() {
    let r = Reglas::default().economia;
    assert_eq!(r.datos.pueblos.capacidad_almacen, comercio::acarreo::capacidad(&r.comercio.flota), "con un almacén menor que el carro, «sale lleno» no se dispararía nunca");
}

// ───────────────────────────── 1. La obra de mapa ─────────────────────────────

#[test]
fn una_obra_de_mapa_se_marca_ocupa_cupo_y_no_pasa_por_la_cola_ni_cuesta_nada_a_la_plaza() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, cobre) = (e.plaza, e.cobre);
    assert!(e.n.candidatos_de_pueblo(plaza).contains(&(cobre, Tipo::MinaCobre)), "el panel ofrece el nodo de cobre de la zona");
    let (madera, edificios) = (e.almacen(Madera), e.n.economia(plaza).unwrap().edificios.len());
    e.columna("beto", 2, Punto::new(1_000, 1_000));
    assert_eq!(e.rechazada("beto", Comando::MarcarPueblo { asentamiento: plaza, nodo: cobre }), rechazos::NO_AUTORIZADO);

    let id = e.marcar();
    let (madre, p) = e.n.pueblo(id).unwrap();
    assert_eq!((madre, p.tipo, p.nodo(), p.carro), (plaza, Tipo::MinaCobre, Some(cobre), None));
    let EstadoPueblo::Pendiente { requerido, entregado } = &p.estado else { panic!("pendiente de materiales") };
    assert_eq!(requerido, &vec![(Madera, 100_000), (Oro, 12_000)], "su extractor (30) + 50 + el carro (20) y el buey (12 de oro)");
    assert!(entregado.iter().all(|&(_, n)| n == 0));
    let eco = e.n.economia(plaza).unwrap();
    assert_eq!((e.almacen(Madera), eco.edificios.len(), eco.pueblos.len()), (madera, edificios, 1), "no pasa por la cola de la ciudad ni paga del almacén");
    assert!(!e.n.candidatos_de_pueblo(plaza).iter().any(|&(n, _)| n == cobre), "el nodo ya está reclamado");
    let h = hechos(&e.n, "pueblo.marcado");
    assert_eq!((h.len(), dato(&h[0], "pueblo"), dato(&h[0], "nodo")), (1, id as i64, cobre as i64));

    // Un nodo ocupado, uno que no existe, uno de ganado y uno fuera de la zona se rechazan.
    assert_eq!(e.rechazada("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: cobre }), rechazos::PUEBLO_NODO_OCUPADO);
    assert_eq!(e.rechazada("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: u32::MAX }), rechazos::PUEBLO_NODO_DESCONOCIDO);
    // Los mundos de antes de las aldeas agrícolas (2026-10-27) aún traen nodos de ganado: no admiten pueblo.
    if let Some(ganado) = (0..e.mapa.nodos.len()).find(|&i| e.mapa.nodos[i].tipo == Tipo::Corral) {
        assert_eq!(e.rechazada("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: ganado as u32 }), rechazos::PUEBLO_NODO_NO_EXTRAIBLE);
    }
    let lejos = (0..e.mapa.nodos.len()).find(|&i| e.mapa.nodos[i].tipo == Tipo::MinaEstano && !e.n.en_zona(plaza, e.mapa.nodos[i].pos)).unwrap() as u32;
    assert_eq!(e.rechazada("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: lejos }), rechazos::PUEBLO_FUERA_DE_LA_ZONA);
}

#[test]
fn un_nodo_sin_ruta_por_tierra_hasta_la_madre_no_admite_pueblo() {
    let m = con_mundo!();
    let n = Nucleo::nuevo(arranque(m, 7));
    let mapa = Mapa::de(&m.0);
    // Un nodo de pueblo y un sitio fundable a unas unidades sin camino por tierra entre los dos (un río o un barranco de por medio).
    let sin_ruta = |i: usize| {
        let nodo = mapa.nodos[i].pos;
        (5..=11).step_by(2).flat_map(|d| (0..24).map(move |g| (d * 1_000, g))).find_map(|(d, g)| {
            let a = ((g * 15) as f64).to_radians();
            let p = Punto::new(nodo.x + (d as f64 * a.cos()) as i64, nodo.y + (d as f64 * a.sin()) as i64);
            (m.1.ruta(nodo, p, &rutas::Opciones::default()).is_none() && n.viabilidad(p, 1).is_some_and(|v| v.motivo.is_none())).then_some(p)
        })
    };
    let (nodo, sitio) = (0..mapa.nodos.len()).filter(|&i| mapa.nodos[i].tipo.es_de_pueblo()).find_map(|i| sin_ruta(i).map(|p| (i as u32, p))).expect("algún nodo sin ruta hasta un sitio fundable");
    let mut e = montar_en(n, mapa, 2, nodo, sitio);
    let plaza = e.plaza;
    assert!(e.n.en_zona(plaza, e.mapa.nodos[nodo as usize].pos), "el nodo cae en la zona");
    let rastro = e.n.hash_estado();
    assert_eq!(e.rechazada("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo }), rechazos::PUEBLO_SIN_RUTA, "su carro no podría acarrear nunca");
    assert_eq!(e.n.hash_estado(), rastro, "el rechazo no deja rastro");
    assert!(e.n.pueblos_de(plaza).is_empty());
}

#[test]
fn exige_madre_de_nivel_2_y_respeta_el_cupo_que_libera_destruirlo_a_mano() {
    let m = con_mundo!();
    let mut uno = montar(m, 7, 1);
    let (plaza, cobre) = (uno.plaza, uno.cobre);
    assert_eq!(uno.rechazada("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: cobre }), rechazos::PUEBLO_NIVEL_INSUFICIENTE);

    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let primero = e.marcar();
    // Tres pueblos en el nivel 2, la aldea agrícola incluida: el segundo, de lo que ofrezca el panel; con la aldea, el cuarto se rechaza aunque haya candidatos.
    let candidatos = e.n.candidatos_de_pueblo(plaza);
    assert!(candidatos.len() >= 2, "la zona de nivel 2 ofrece nodos para otros dos pueblos: {candidatos:?}");
    assert!(candidatos.iter().all(|c| c.1 != Tipo::MinaHierro), "el hierro pide una tecnología que la Facción no tiene: no es candidato");
    let segundo = e.ok("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: candidatos[0].0 }).creadas[0];
    comun::levantar_aldea(&mut e.n, "ana", plaza).expect("la aldea ocupa el tercer hueco");
    assert_eq!(e.rechazada("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: candidatos[1].0 }), rechazos::PUEBLO_SIN_CUPO);
    // Destruirlo a mano (aunque esté pendiente) libera el cupo y el nodo, y no devuelve nada.
    assert_eq!(e.rechazada("beto", Comando::DestruirPueblo { pueblo: primero }), rechazos::NO_AUTORIZADO);
    e.ok("ana", Comando::DestruirPueblo { pueblo: primero });
    assert!(e.n.pueblo(primero).is_none() && e.n.pueblo(segundo).is_some());
    assert_eq!(e.rechazada("ana", Comando::DestruirPueblo { pueblo: primero }), rechazos::PUEBLO_DESCONOCIDO);
    assert_eq!(hechos(&e.n, "pueblo.destruido").len(), 1);
    e.ok("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: candidatos[1].0 });
    assert_eq!(e.n.pueblos_de(plaza).len(), 3);
}

#[test]
fn los_minerales_y_la_segunda_cantera_ya_no_se_construyen_dentro_de_la_ciudad() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    e.dar(Madera, 300_000);
    for tipo in [Tipo::MinaCobre, Tipo::MinaEstano, Tipo::Mina] {
        assert_eq!(e.rechazada("ana", Comando::AnadirEdificio { asentamiento: plaza, tipo }), rechazos::OBRA_SOLO_POR_PUEBLO, "{tipo:?}");
    }
    e.ok("ana", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Cantera });
    assert_eq!(e.rechazada("ana", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Cantera }), rechazos::OBRA_SOLO_POR_PUEBLO, "la primera es directa; las demás, por pueblo");
}

// ───────────────────────────── 2. Los materiales y la obra ─────────────────────────────

#[test]
fn los_materiales_se_entregan_en_el_sitio_con_una_columna_de_la_faccion_y_con_todo_entregado_arranca_la_obra() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let id = e.marcar();
    let sitio = e.n.posicion_de_pueblo(id).unwrap();
    let col = e.columna("ana", 1, sitio);
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 58_000 }); // con los 2 que entregan abajo carla y ana, 60
    // La puerta, la Facción y el propietario de la columna se comprueban.
    let lejos = e.columna("ana", 1, Punto::new(sitio.x + 20_000, sitio.y));
    e.ok("admin", Comando::DarSuministro { ejercito: lejos, recurso: Madera, cantidad: 10_000 });
    assert_eq!(e.rechazada("ana", Comando::EntregarMateriales { ejercito: lejos, pueblo: id }), rechazos::PUEBLO_FUERA_DE_LA_PUERTA);
    let ajena = e.columna("beto", 2, sitio);
    e.ok("admin", Comando::DarSuministro { ejercito: ajena, recurso: Madera, cantidad: 10_000 });
    assert_eq!(e.rechazada("beto", Comando::EntregarMateriales { ejercito: ajena, pueblo: id }), rechazos::PUEBLO_FACCION_AJENA);
    assert_eq!(e.rechazada("beto", Comando::EntregarMateriales { ejercito: col, pueblo: id }), rechazos::NO_AUTORIZADO);
    let vacia = e.columna("ana", 1, sitio);
    assert_eq!(e.rechazada("ana", Comando::EntregarMateriales { ejercito: vacia, pueblo: id }), rechazos::PUEBLO_NADA_QUE_ENTREGAR);
    // Entregan los ciudadanos de la Facción (Doc 4.2.2), sean o no el Líder: carla va en una columna de ana y entrega lo de ella;
    // al dejar de ser ciudadana (la columna conserva la bandera de su Líder) ya no.
    let de_ana = e.columna("ana", 1, sitio);
    e.ok("admin", Comando::DarSuministro { ejercito: de_ana, recurso: Madera, cantidad: 1_000 });
    let carla = e.ok("carla", Comando::CrearHeroe { cuenta: "carla".into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0];
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: carla, faccion: Some(1) });
    e.ok("admin", Comando::ColocarHeroe { heroe: carla, ubicacion: Ubicacion::Columna(de_ana) });
    e.ok("carla", Comando::EntregarMateriales { ejercito: de_ana, pueblo: id });
    e.ok("admin", Comando::DarSuministro { ejercito: de_ana, recurso: Madera, cantidad: 1_000 });
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: carla, faccion: None });
    assert_eq!(e.n.ejercito(de_ana).unwrap().columna.faccion, Some(1));
    assert_eq!(e.rechazada("carla", Comando::EntregarMateriales { ejercito: de_ana, pueblo: id }), rechazos::PUEBLO_FACCION_AJENA);
    e.ok("ana", Comando::EntregarMateriales { ejercito: de_ana, pueblo: id });
    assert_eq!(e.pueblo(id).falta(), vec![(Madera, 98_000), (Oro, 12_000)]);

    // Una entrega parcial no arranca nada: se anota lo entregado y el carro de la columna queda vacío.
    e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
    assert_eq!(e.pueblo(id).falta(), vec![(Madera, 40_000), (Oro, 12_000)]);
    assert!(e.n.suministro(col).is_empty());
    // Se completa con una segunda columna; lo que sobra se queda en el carro.
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 50_000 });
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 12_000 });
    let t = e.n.ahora();
    e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
    assert_eq!(e.n.suministro(col)[&Madera], 10_000);
    assert_eq!(e.pueblo(id).estado, EstadoPueblo::EnObra { completa_en: t + 6 * HORA }, "la obra de una mina de cobre dura 6 h");
    assert_eq!(e.rechazada("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id }), rechazos::PUEBLO_NO_ESTA_PENDIENTE);
    assert_eq!((hechos(&e.n, "pueblo.materiales_entregados").len(), hechos(&e.n, "pueblo.obra_iniciada").len()), (4, 1));
    // Mientras se levanta no extrae ni tiene carro; acaba en el primer paso de minuto de la plaza que alcanza su hora (precisión de un minuto).
    e.n.avanzar_hasta(t + 6 * HORA - 1);
    assert!(!e.pueblo(id).activo());
    e.n.avanzar_hasta(t + 6 * HORA + MIN);
    let fin = hechos(&e.n, "pueblo.completado");
    assert_eq!(fin.len(), 1);
    assert!(fin[0].0 >= t + 6 * HORA && fin[0].0 < t + 6 * HORA + MIN, "acaba dentro del minuto de su hora");
    // Nace su carro de acarreo, parado en el pueblo, de la flota de la madre pero sin contar contra sus cupos.
    let carro = e.pueblo(id).carro.expect("nace con su carro");
    let c = e.n.caravana(carro).unwrap();
    assert_eq!((c.tipo, c.estado, c.origen as u64, c.posicion), (TipoCaravana::Acarreo, EstadoCaravana::Disponible, e.plaza, sitio));
    let flota = &e.n.estado().comercio.flota;
    assert_eq!((comercio::caravana::caravanas_comerciales_de(flota, e.plaza as u32), comercio::caravana::carros_en_uso(flota, e.plaza as u32)), (0, 0), "el carro de acarreo no cuenta contra los cupos del Mercado");
}

#[test]
fn una_caravana_adjunta_tambien_lleva_materiales() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    // Un Mercado para tener caravanas; la gracia de mantenimiento de un día cubre las 9 h de obra.
    e.dar(Madera, 350_000);
    e.ok("ana", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
    e.avanzar(9 * HORA);
    e.dar(Madera, 350_000);
    e.dar(Oro, 100_000);
    let car = e.ok("ana", Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
    e.ok("ana", Comando::AnadirCarro { caravana: car, tipo: comercio::caravana::TipoCarro::Basico });
    e.ok("ana", Comando::AnadirAnimal { caravana: car, carro: 0, animal: comercio::caravana::Animal::Buey });
    e.ok("ana", Comando::ReservarCaravana { caravana: car, reservada: true });
    // La columna la engancha, entra en la plaza y la deja aparcada para cargarla del almacén; luego la vuelve a enganchar.
    let pos = e.n.estado().asentamientos[&plaza].posicion;
    let col = e.columna("ana", 1, Punto::new(pos.x + 1_000, pos.y));
    e.ok("ana", Comando::AdjuntarCaravana { caravana: car, ejercito: col });
    e.ok("admin", Comando::AparcarCaravana { caravana: car, asentamiento: plaza }); // la administración la deja dentro (5G-a-5: solo se aparca dentro)
    e.ok("ana", Comando::MoverCargaAparcada { caravana: car, recurso: Madera, cantidad: 100_000, cargar: true });
    e.ok("ana", Comando::MoverCargaAparcada { caravana: car, recurso: Oro, cantidad: 12_000, cargar: true });
    e.ok("ana", Comando::AdjuntarCaravana { caravana: car, ejercito: col });
    // Marca el pueblo; la columna marcha a su puerta y entrega lo que lleva en la caravana (su carro está vacío).
    let id = e.marcar();
    let sitio = e.n.posicion_de_pueblo(id).unwrap();
    e.ok("ana", Comando::MarcharA { ejercito: col, destino: sitio });
    let fin = e.n.estado().ejercitos[&col].plan.fin().expect("marcha");
    e.n.avanzar_hasta(fin);
    e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
    assert!(matches!(e.pueblo(id).estado, EstadoPueblo::EnObra { .. }), "la caravana adjunta puso los 100 de madera y los 12 de oro");
    assert!(e.n.caravana(car).unwrap().contenido.is_empty());
}

// ───────────────────────────── 3. Extraer, llenar y acarrear ─────────────────────────────

#[test]
fn el_pueblo_extrae_contra_su_almacen_y_el_carro_sale_lleno_vuelca_en_la_madre_y_vuelve_vacio() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, cobre) = (e.plaza, e.cobre);
    let id = e.pueblo_levantado();
    let activo_desde = hechos(&e.n, "pueblo.completado")[0].0;
    let carro = e.pueblo(id).carro.unwrap();
    // Extrae contra el almacén local (no el de la plaza): 5 por minuto, también el minuto en que se completó.
    e.n.avanzar_hasta(activo_desde + 10 * MIN);
    assert_eq!(e.pueblo(id).almacen.cantidad(Cobre), 55_000);
    assert_eq!(e.almacen(Cobre), 0, "no entra en el almacén de la madre hasta que lo traiga el carro");
    // El logro `extraido.cobre` (6B) cuenta en origen lo que sale del nodo (D17): lo del pueblo, al extraerlo.
    let extraido = |e: &Escenario| e.n.tecnologia().contador(tecnologia::Contador::ExtraidoCobre);
    assert_eq!(extraido(&e), 55);
    // Hasta llenarse: el nodo se agota y espera a regenerarse sin que el pueblo se mude.
    let mut t = activo_desde + 10 * MIN;
    while hechos(&e.n, "acarreo.vuelve").is_empty() {
        t += 10 * MIN;
        assert!(t < activo_desde + 12 * HORA, "el carro debería haber vuelto");
        e.n.avanzar_hasta(t);
        assert!(e.pueblo(id).almacen.cantidad(Cobre) <= 500_000);
    }
    assert_eq!(e.pueblo(id).nodo(), Some(cobre));
    let sale = hechos(&e.n, "acarreo.sale");
    let (sale_en, carga) = (sale[0].0, dato(&sale[0], "carga"));
    assert_eq!(carga, 500_000, "solo sale con el carro lleno");
    assert_eq!(hechos(&e.n, "pueblo.almacen_lleno")[0].0, sale_en, "sale en el mismo minuto en que se llena");
    // El viaje es el de una caravana: plan por el mapa real a 24 u/min (el buey) hasta la plaza madre, sin la ventaja del camino aunque su ruta
    // ya esté en la red (decisión del autor del 2026-10-29: con ella iba al doble).
    let llega = hechos(&e.n, "acarreo.llega");
    let (llega_en, ingresado, perdido) = (llega[0].0, dato(&llega[0], "carga"), dato(&llega[0], "perdido"));
    let sitio = e.n.posicion_de_pueblo(id).unwrap();
    let camino = m.1.ruta(sitio, e.n.estado().asentamientos[&plaza].posicion, &rutas::Opciones::default()).unwrap();
    let esperado = (camino.coste_total() as i128 * m.1.espaciado() as i128 * 60 / 24_000) as i64;
    assert!(e.n.red_de_caminos().ruta(plaza as u32, id as u32).is_some(), "su ruta está en la red de caminos");
    assert!((llega_en - sale_en - esperado).abs() < 1_000, "tarda lo que su ruta a 24 u/min: {} frente a {}", llega_en - sale_en, esperado);
    // El almacén de la madre admite 400 de cada recurso: lo que no cabe se pierde.
    assert_eq!((ingresado, perdido), (400_000, 100_000));
    // Lo que el carro vuelca en la madre no se cuenta otra vez: el contador es lo que salió del nodo, en los carros y en el almacén del pueblo.
    let salido: i64 = hechos(&e.n, "acarreo.sale").iter().map(|h| dato(h, "carga")).sum::<i64>() + e.pueblo(id).almacen.cantidad(Cobre);
    assert_eq!(extraido(&e) as i64 * 1_000, salido);
    // La ventana de la solvencia guarda lo que cupo, no lo que traía el carro, en la hora de la llegada.
    let eco = e.n.economia(plaza).unwrap();
    assert_eq!(eco.ingresos_acarreo, vec![IngresoAcarreo { recurso: Cobre, hora: llega_en / HORA, cantidad: 400_000 }]);
    let vuelve = hechos(&e.n, "acarreo.vuelve");
    assert_eq!((dato(&vuelve[0], "caravana"), vuelve[0].0 - llega_en), (carro as i64, llega_en - sale_en), "vuelve vacío por la misma ruta, en el mismo tiempo");
    let c = e.n.caravana(carro).unwrap();
    assert_eq!((c.tipo, c.estado, c.posicion), (TipoCaravana::Acarreo, EstadoCaravana::Disponible, sitio));
    assert!(e.n.caravana(carro).unwrap().contenido.is_empty());
    // Con el carro en casa y el almacén vacío, todo vuelve a empezar; mientras tanto, el almacén de la madre guarda lo que entró.
    assert!(e.almacen(Cobre) >= 400_000 || hechos(&e.n, "acarreo.sale").len() > 1);
}

#[test]
fn con_el_almacen_de_la_madre_lleno_el_acarreo_no_ingresa_nada_ni_cuenta_para_la_solvencia() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    e.dar(Cobre, 400_000);
    assert_eq!(e.almacen(Cobre), 400_000, "el almacén de la madre admite 400 de cada recurso y queda lleno");
    let id = e.pueblo_levantado();
    let activo_desde = hechos(&e.n, "pueblo.completado")[0].0;
    let mut t = activo_desde;
    while hechos(&e.n, "acarreo.llega").is_empty() {
        t += 10 * MIN;
        assert!(t < activo_desde + 12 * HORA, "el carro debería haber llegado");
        e.n.avanzar_hasta(t);
    }
    let llega = &hechos(&e.n, "acarreo.llega")[0];
    assert_eq!((dato(llega, "carga"), dato(llega, "perdido")), (0, 500_000), "todo se pierde contra un almacén lleno");
    assert!(e.n.economia(plaza).unwrap().ingresos_acarreo.is_empty(), "y no cuenta como ingreso: la plaza no se hace solvente con un pueblo que se pierde");
    assert!(e.pueblo(id).carro.is_some());
}

#[test]
fn con_el_nodo_agotado_el_pueblo_espera_a_que_se_regenere_y_no_se_muda() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let cobre = e.cobre;
    let id = e.pueblo_levantado();
    let desde = hechos(&e.n, "pueblo.completado")[0].0;
    let inicial = e.mapa.nodos[cobre as usize].inicial;
    // Hasta agotar el nodo extrae 5 por minuto; el stock del nodo es lo que hay (101-299 unidades): se agota antes de llenar el almacén.
    let minutos_en_agotar = inicial / 5_000;
    e.n.avanzar_hasta(desde + minutos_en_agotar * MIN + MIN);
    assert_eq!(e.pueblo(id).almacen.cantidad(Cobre), inicial, "sacó todo el nodo");
    assert_eq!(e.n.estado().yacimientos[&cobre].stock, 0);
    // Espera los 10 minutos del cooldown (el nodo sigue siendo el suyo)...
    e.n.avanzar_hasta(desde + minutos_en_agotar * MIN + 5 * MIN);
    assert_eq!(e.pueblo(id).almacen.cantidad(Cobre), inicial, "agotado, espera");
    // ...y vuelve a extraer cuando se regenera.
    e.n.avanzar_hasta(desde + minutos_en_agotar * MIN + 12 * MIN);
    assert!(e.pueblo(id).almacen.cantidad(Cobre) > inicial, "regeneró y siguió");
    assert_eq!(e.pueblo(id).nodo(), Some(cobre));
}

/// Pierde el carro del pueblo una y otra vez (cada reposición automática cuesta 20 de madera a la madre, que no produce) hasta que la madre ya no
/// puede pagar otro: el pueblo queda sin carro.
fn perder_el_carro_hasta_que_no_se_repone(e: &mut Escenario, pueblo: u64) {
    for _ in 0..10 {
        let Some(carro) = e.pueblo(pueblo).carro else { return };
        let t = e.n.ahora();
        assert!(e.n.aplicar_perdida_de_carros(t, carro, &[0]).unwrap().destruida);
        e.avanzar(MIN);
    }
    panic!("la madre no deja de reponer el carro");
}

#[test]
fn sin_carro_el_pueblo_acumula_hasta_llenar_el_almacen_y_para_hasta_que_se_repone() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, cobre) = (e.plaza, e.cobre);
    let id = e.pueblo_levantado();
    let carro = e.pueblo(id).carro.unwrap();
    // Un ataque destruye el carro (el hook del sistema militar) con el pueblo recién levantado.
    let t = e.n.ahora();
    assert!(e.n.aplicar_perdida_de_carros(t, carro, &[0]).unwrap().destruida);
    assert!(e.n.caravana(carro).is_none() && e.pueblo(id).carro.is_none());
    // Reponer a mano cuesta el carro y el buey (20 de madera y 12 de oro) de la madre, solo si el pueblo no tiene ninguno y solo a quien manda.
    assert_eq!(e.rechazada("beto", Comando::ReponerCarro { pueblo: id }), rechazos::NO_AUTORIZADO);
    let (madera, oro) = (e.almacen(Madera), e.almacen(Oro));
    e.ok("ana", Comando::ReponerCarro { pueblo: id });
    assert_eq!((madera - e.almacen(Madera), oro - e.almacen(Oro)), (20_000, 12_000));
    let nuevo = e.pueblo(id).carro.expect("tiene carro otra vez");
    assert_ne!(nuevo, carro);
    assert_eq!(e.rechazada("ana", Comando::ReponerCarro { pueblo: id }), rechazos::PUEBLO_YA_TIENE_CARRO);
    // Sin la orden, la madre lo repone sola en su siguiente paso de minuto, mientras pueda pagarlo (2026-10-27).
    let t = e.n.ahora();
    e.n.aplicar_perdida_de_carros(t, nuevo, &[0]).unwrap();
    e.avanzar(MIN);
    let solo = e.pueblo(id).carro.expect("repuesto solo");
    assert_eq!((e.almacen(Madera), hechos(&e.n, "acarreo.carro_repuesto").len()), (madera - 40_000, 2));
    assert_ne!(solo, nuevo);
    // Con 10 de madera ya no puede: sin carro no sale nada, acumula, se llena una sola vez y la extracción para (el nodo no se gasta de más).
    perder_el_carro_hasta_que_no_se_repone(&mut e, id);
    assert!(e.almacen(Madera) < 20_000);
    e.avanzar(8 * HORA);
    assert!(e.pueblo(id).carro.is_none());
    assert_eq!(e.pueblo(id).almacen.cantidad(Cobre), 500_000);
    assert!(e.pueblo(id).lleno);
    assert_eq!((hechos(&e.n, "pueblo.almacen_lleno").len(), hechos(&e.n, "acarreo.sale").len()), (1, 0));
    let stock_nodo = e.n.estado().yacimientos.get(&cobre).map(|s| s.stock);
    e.avanzar(30 * MIN);
    assert_eq!(e.n.estado().yacimientos.get(&cobre).map(|s| s.stock), stock_nodo, "lleno, no gasta el yacimiento");
    // En cuanto tiene la madera, lo repone en el siguiente paso de minuto y el carro sale al instante con el almacén lleno.
    e.dar(Madera, 20_000);
    e.avanzar(MIN);
    let ultimo = e.pueblo(id).carro.expect("repuesto en cuanto pudo");
    assert_eq!(e.n.caravana(ultimo).unwrap().estado, EstadoCaravana::EnTransito);
    assert_eq!((e.pueblo(id).almacen.cantidad(Cobre), e.n.caravana(ultimo).unwrap().contenido[&Cobre]), (0, 500_000));
    // Un pueblo aún sin levantar no tiene carro que reponer.
    let otro = e.n.candidatos_de_pueblo(plaza).into_iter().find(|c| c.1 == Tipo::Cantera).unwrap().0;
    let pendiente = e.ok("ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: otro }).creadas[0];
    assert_eq!(e.rechazada("ana", Comando::ReponerCarro { pueblo: pendiente }), rechazos::PUEBLO_NO_ESTA_ACTIVO);
}

#[test]
fn el_pueblo_paga_mantenimiento_a_la_madre_y_no_deja_eventos_recurrentes_por_pueblo_ocioso() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, datos) = (e.plaza, Reglas::default().economia.datos);
    let sin_pueblos = e.n.economia(plaza).unwrap().coste_mantenimiento(&datos, 0);
    let id = e.marcar();
    assert_eq!(e.n.economia(plaza).unwrap().coste_mantenimiento(&datos, 0), sin_pueblos, "pendiente o en obra, aún no cuenta");
    let col = e.llevar_materiales(id);
    let en_cola_base = e.n.eventos_en_cola(); // el paso de minuto de la plaza y los umbrales de víveres de la columna
    e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
    let EstadoPueblo::EnObra { completa_en } = e.pueblo(id).estado else { panic!("en obra") };
    e.n.avanzar_hasta(completa_en + MIN);
    assert!(e.n.economia(plaza).unwrap().coste_mantenimiento(&datos, 0)[0].1 > sin_pueblos[0].1, "activo, cuenta como un edificio más de la madre");
    // Sin carro y con el almacén lleno el pueblo no programa nada: solo quedan el paso de minuto de la plaza y lo que ya había.
    perder_el_carro_hasta_que_no_se_repone(&mut e, id);
    e.avanzar(8 * HORA);
    assert!(e.pueblo(id).lleno);
    assert_eq!(e.n.eventos_en_cola(), en_cola_base, "ni el pueblo ocioso ni el carro ausente tienen eventos propios");
}

#[test]
fn la_planta_del_pueblo_es_una_funcion_de_su_id_y_no_se_guarda() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let id = e.pueblo_levantado();
    let hash = e.n.hash_estado();
    let planta = e.n.planta_de_pueblo(id).expect("la planta del pueblo");
    assert_eq!(planta, e.n.planta_de_pueblo(id).unwrap());
    assert_eq!(planta.piezas[0].tipo, Tipo::MinaCobre, "la mina es el ancla");
    assert!(planta.piezas.iter().any(|p| p.tipo == Tipo::Almacen) && planta.piezas.iter().any(|p| p.tipo == Tipo::Corral));
    assert_eq!(e.n.hash_estado(), hash, "es presentación: no toca el estado");
    assert!(e.n.planta_de_pueblo(u64::MAX).is_none());
    // La guarnición de serie la pone el sistema militar; su dato (30 mineros, uno cada 12 min) está en las reglas.
    let p = Reglas::default().economia.datos.pueblos.clone();
    assert_eq!((p.mineros, p.minutos_por_minero), (30, 12));
}

// ───────────────────────────── 4. Destruirlo, la ruina y la conquista de la madre ─────────────────────────────

/// Avanza de minuto en minuto hasta que el carro del pueblo esté en camino a la madre.
fn hasta_que_el_carro_vaya_cargado(e: &mut Escenario, pueblo: u64) -> u64 {
    let carro = e.pueblo(pueblo).carro.unwrap();
    let limite = e.n.ahora() + 12 * HORA;
    while e.n.caravana(carro).unwrap().estado != EstadoCaravana::EnTransito {
        e.avanzar(MIN);
        assert!(e.n.ahora() < limite, "el carro debería haber salido");
    }
    carro
}

#[test]
fn la_ruta_del_carro_se_traza_una_vez_al_levantarse_entra_en_la_red_y_se_olvida_con_el_pueblo() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let id = e.pueblo_levantado();
    // Al levantarse se traza y se guarda, y entra en la red de caminos como una ruta madre → pueblo (2026-10-27).
    let guardada = e.n.estado().comercio.rutas_acarreo.get(&id).expect("ruta guardada al levantarse").clone();
    assert_eq!(guardada.faccion, 1);
    assert_eq!((guardada.trazado.puntos.first(), guardada.trazado.puntos.last()), (e.n.posicion_de_pueblo(id).as_ref(), Some(&e.n.estado().asentamientos[&plaza].posicion)));
    assert!(e.n.rutas_vigentes().any(|r| (u64::from(r.origen), u64::from(r.destino)) == (plaza, id)));
    let aristas = e.n.red_de_caminos().aristas().count();
    assert!(aristas > 0);
    // Cada viaje va por la ruta guardada, sin trazarla otra vez, y renueva la ruta en la red.
    let carro = hasta_que_el_carro_vaya_cargado(&mut e, id);
    assert_eq!(e.n.caravana(carro).unwrap().ruta, guardada.trazado.puntos);
    let lanzada = e.n.red_de_caminos().ruta(plaza as u32, id as u32).unwrap().ultimo_lanzamiento;
    assert_eq!(lanzada, e.n.ahora());
    assert_eq!(e.n.estado().comercio.rutas_acarreo[&id], guardada);
    // La foto la guarda.
    let bytes = e.n.estado().serializar();
    let vuelta = nucleo::Estado::deserializar(&bytes, &Reglas::default(), &e.n.estado().mundo_hash).unwrap();
    assert_eq!(vuelta.comercio.rutas_acarreo, e.n.estado().comercio.rutas_acarreo);
    // Destruido el pueblo, su ruta se olvida y sale de la red (las aristas, que ya son camino, se quedan).
    e.ok("ana", Comando::DestruirPueblo { pueblo: id });
    assert!(e.n.estado().comercio.rutas_acarreo.is_empty());
    assert!(!e.n.rutas_vigentes().any(|r| u64::from(r.destino) == id));
    assert_eq!(e.n.red_de_caminos().aristas().count(), aristas);
}

#[test]
fn un_residente_de_la_madre_cede_a_mano_escolta_al_carro_de_acarreo_y_viaja_con_el() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    // El cupo de escolta de la madre es el de su Mercado (como en cualquier caravana suya): sin Mercado, ninguno.
    e.dar(Madera, 350_000);
    e.ok("ana", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
    e.avanzar(9 * HORA);
    let id = e.pueblo_levantado();
    let carro = e.pueblo(id).carro.unwrap();
    // Carla reside en la madre y está dentro: cede una escuadra de su campamento al carro parado en el pueblo. No hay escolta automática.
    assert!(e.n.caravana(carro).unwrap().escolta.is_empty());
    let carla = e.ok("carla", Comando::CrearHeroe { cuenta: "carla".into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0];
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: carla, faccion: Some(1) });
    e.ok("admin", Comando::FijarResidencia { heroe: carla, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    e.ok("admin", Comando::ColocarHeroe { heroe: carla, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
    let tropa = militar::catalogo::Tropa::MiliciaLanceros;
    let escuadra = e.ok("admin", Comando::DarEscuadra { heroe: carla, tropa, cantidad: 25 }).creadas[0];
    e.ok("carla", Comando::AsignarEscolta { caravana: carro, heroe: carla, escuadras: vec![escuadra] });
    assert_eq!(e.n.caravana(carro).unwrap().escolta.iter().map(|x| x.id).collect::<Vec<_>>(), vec![escuadra]);
    assert_eq!(hechos(&e.n, "comercio.caravana_escolta_cedida").len(), 1);
    // Sale lleno con su escolta, vuelca en la madre y vuelve con ella.
    hasta_que_el_carro_vaya_cargado(&mut e, id);
    assert_eq!(e.n.caravana(carro).unwrap().escolta.len(), 1);
    let limite = e.n.ahora() + 2 * HORA;
    while hechos(&e.n, "acarreo.vuelve").is_empty() {
        e.avanzar(MIN);
        assert!(e.n.ahora() < limite, "el carro debería haber vuelto");
    }
    assert_eq!(e.n.caravana(carro).unwrap().escolta.iter().map(|x| x.id).collect::<Vec<_>>(), vec![escuadra], "la escolta sigue con el carro al viaje siguiente");
    assert_eq!(e.n.heroe(carla).unwrap().heroe.escuadra(escuadra).unwrap().contenedor, militar::escuadra::Contenedor::Escolta(carro));
    // Con el carro en casa (su pueblo), Carla se la quita y vuelve a su campamento.
    e.ok("carla", Comando::QuitarEscolta { caravana: carro, heroe: carla, escuadras: None });
    assert!(e.n.caravana(carro).unwrap().escolta.is_empty());
    assert_eq!(e.n.heroe(carla).unwrap().heroe.escuadra(escuadra).unwrap().contenedor, militar::escuadra::Contenedor::Campamento);
}

#[test]
fn destruir_a_mano_un_pueblo_con_el_carro_en_camino_pierde_el_carro_su_carga_y_libera_el_nodo() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, cobre) = (e.plaza, e.cobre);
    let id = e.pueblo_levantado();
    let carro = hasta_que_el_carro_vaya_cargado(&mut e, id);
    let cobre_madre = e.almacen(Cobre);
    e.ok("ana", Comando::DestruirPueblo { pueblo: id });
    assert!(e.n.pueblo(id).is_none() && e.n.caravana(carro).is_none(), "se pierde el pueblo y su carro con lo que llevaba");
    assert!(!e.n.estado().nodos_reclamados.contains_key(&cobre), "el nodo vuelve a estar libre");
    e.avanzar(3 * HORA);
    assert_eq!(e.almacen(Cobre), cobre_madre, "lo que iba en el carro no llegó nunca");
    assert!(hechos(&e.n, "acarreo.llega").is_empty());
    assert!(e.n.candidatos_de_pueblo(plaza).iter().any(|&(n, _)| n == cobre), "y vuelve a ser candidato");
}

#[test]
fn la_ruina_de_la_madre_se_lleva_sus_pueblos_y_sus_carros_y_libera_los_nodos() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let (plaza, cobre) = (e.plaza, e.cobre);
    let id = e.pueblo_levantado();
    let carro = e.pueblo(id).carro.unwrap();
    // Sin auto-construcción ni madera que repongan el mantenimiento, la plaza acaba cayendo en ruinas.
    for dia in 1..=150 {
        let t = e.n.ahora().max(dia * 24 * HORA);
        e.n.avanzar_hasta(t);
        if e.n.economia(plaza).is_none() {
            break;
        }
    }
    assert!(e.n.economia(plaza).is_none(), "la plaza cayó en ruinas");
    assert!(e.n.pueblo(id).is_none() && e.n.pueblos_de(plaza).is_empty());
    assert!(e.n.caravana(carro).is_none(), "su carro, con la madre por origen, se pierde en la cascada de comercio");
    assert!(hechos(&e.n, "comercio.caravana_perdida").iter().any(|h| dato(h, "caravana") == carro as i64));
    assert!(!e.n.estado().nodos_reclamados.contains_key(&cobre), "el nodo queda libre");
    assert!(e.n.estado().comercio.flota.iter().all(|c| c.tipo != TipoCaravana::Acarreo));
}

#[test]
fn la_conquista_de_la_madre_deja_los_pueblos_con_la_plaza_y_el_carro_en_camino_sigue() {
    let m = con_mundo!();
    // Sin el día de protección con que nace la plaza (Doc 1.2): lo que se mide es la conquista con el carro aún en camino.
    let montar_sin_proteccion = || {
        let mut reglas = sin_bandidos();
        std::sync::Arc::make_mut(&mut reglas.economia.combate).ocupacion.proteccion = 0;
        let mut e = montar_con(Nucleo::nuevo(Arranque { reglas, ..arranque(m, 7) }), m, 2);
        let id = e.pueblo_levantado();
        (e, id)
    };
    // El asedio cae al acabar su convocatoria de 30 s y el viaje del carro es más corto: una primera pasada dice cuándo sale, y en la segunda (el mismo mundo) el asedio se
    // abre 28 s antes.
    let sale = {
        let (mut e, id) = montar_sin_proteccion();
        hasta_que_el_carro_vaya_cargado(&mut e, id);
        e.n.ahora()
    };
    let (mut e, id) = montar_sin_proteccion();
    let plaza = e.plaza;
    e.n.avanzar_hasta(sale - 28_000);
    // La conquista de verdad: el ejército de Beto (F2) asedia la madre, que no tiene quien la defienda, y cae sin combate.
    let p = e.n.estado().asentamientos[&plaza].posicion;
    let beto = e.columna("beto", 2, Punto::new(p.x + 2_000, p.y));
    let heroe = e.n.ejercito(beto).unwrap().columna.lider;
    let arqueros = militar::catalogo::Tropa::ArquerosCompuesto;
    e.ok("admin", Comando::DarEscuadra { heroe, tropa: arqueros, cantidad: militar::Reglas::cargar().tropas.tropa(arqueros).unidades });
    e.ok("beto", Comando::Atacar { heroe, objetivo: nucleo::ObjetivoAtaque::Asentamiento(plaza), declara_guerra: true });
    tras_la_convocatoria(&mut e.n);
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, 2);
    let carro = e.pueblo(id).carro.unwrap();
    // Los pueblos son de la plaza: pasan con ella (y su zona) al conquistador, y la flota traspasada, también el carro de acarreo.
    assert!(e.n.pueblo(id).is_some_and(|(madre, p)| madre == plaza && p.carro == Some(carro)));
    assert_eq!(e.n.caravana(carro).unwrap().estado, EstadoCaravana::EnTransito);
    assert_eq!(hechos(&e.n, "comercio.flota_traspasada").len(), 1);
    e.avanzar(HORA);
    assert!(!hechos(&e.n, "acarreo.llega").is_empty(), "el carro entrega en la plaza ya conquistada");
}

// ───────────────────────────── 5. Determinismo y recuperación ─────────────────────────────

/// Un guion con pueblos: marca, entrega, se levanta, pierde el carro (el primero de la flota), la madre lo repone sola en su paso de minuto y marca otro pueblo. Las órdenes
/// llevan su instante: `avanzar` solo decide cómo se mueve el reloj entre ellas.
fn guion(e: &mut Escenario, avanzar: &dyn Fn(&mut Escenario, Instante)) {
    let plaza = e.plaza;
    let id = e.marcar();
    let otro = e.n.candidatos_de_pueblo(plaza)[0].0;
    let col = e.llevar_materiales(id);
    avanzar(e, 20 * MIN);
    e.ok_en(20 * MIN, "ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
    avanzar(e, 8 * HORA);
    // `car-2`: el 1 es la Caravana de Fundación con que la administración fundó la plaza (5G-b: toma su id del contador de las caravanas).
    e.n.aplicar_perdida_de_carros(8 * HORA, 2, &[0]).expect("el carro del pueblo");
    avanzar(e, 12 * HORA);
    e.ok_en(12 * HORA, "ana", Comando::MarcarPueblo { asentamiento: plaza, nodo: otro });
    avanzar(e, 20 * HORA);
}

fn correr(m: &M, semilla: u64, avanzar: &dyn Fn(&mut Escenario, Instante)) -> Nucleo {
    let mut e = montar(m, semilla, 2);
    guion(&mut e, avanzar);
    e.n.avanzar_hasta(20 * HORA);
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
    assert_ne!(normal.hash_estado(), correr(m, 4, &|e, t| e.n.avanzar_hasta(t)).hash_estado(), "otra semilla, otro mundo");
    // El guion hizo de verdad lo que se espera.
    assert!(!hechos(&normal, "acarreo.llega").is_empty() && hechos(&normal, "pueblo.completado").len() == 1 && hechos(&normal, "acarreo.carro_repuesto").len() == 1);
}

#[test]
fn una_foto_a_mitad_de_obra_y_otra_con_el_carro_en_camino_recuperan_el_mismo_mundo() {
    let m = con_mundo!();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9) };
    // Dos cortes: con la obra a medias (3 h de 6) y con el carro cargado y en camino.
    for (nombre, carro_en_camino) in [("a mitad de obra", false), ("con el carro en camino", true)] {
        let dir = tempfile::tempdir().unwrap();
        // Sin parar: la referencia.
        let referencia = {
            let mut e = montar(m, 9, 2);
            let id = e.marcar();
            let col = e.llevar_materiales(id);
            e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
            e.avanzar(10 * HORA);
            e.avanzar(10 * HORA);
            (e.n.hash_estado(), e.n.estado().comercio.clone())
        };
        let (a_mitad, cuenta, plaza, id, ahora) = {
            let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
            let mut e = montar_con(n, m, 2);
            let id = e.marcar();
            let col = e.llevar_materiales(id);
            e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
            if carro_en_camino {
                e.avanzar(6 * HORA + MIN);
                let carro = hasta_que_el_carro_vaya_cargado(&mut e, id);
                assert_eq!(e.n.caravana(carro).unwrap().estado, EstadoCaravana::EnTransito, "la foto pilla el viaje en mitad");
            } else {
                e.avanzar(3 * HORA);
                assert!(matches!(e.pueblo(id).estado, EstadoPueblo::EnObra { .. }), "la foto pilla la obra en mitad");
            }
            e.n.foto_ahora();
            e.avanzar(10 * MIN);
            e.n.apagar().unwrap();
            (e.n.hash_estado(), e.cuenta, e.plaza, id, e.n.ahora())
        };
        let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        assert!(n.recuperado());
        assert_eq!(n.hash_estado(), a_mitad, "{nombre}: foto + registro reconstruyen el mismo mundo");
        let cobre = n.pueblo(id).unwrap().1.nodo().unwrap();
        let mut e = Escenario { n, plaza, cobre, mapa: Mapa::de(&m.0), cuenta };
        e.avanzar(20 * HORA - ahora);
        assert_eq!(e.n.hash_estado(), referencia.0, "{nombre}: el mundo recuperado sigue igual que el que no se interrumpió");
        assert_eq!(e.n.estado().comercio, referencia.1);
        e.n.apagar().unwrap();
    }
}

// ───────────────────────────── 6. Entrar en un pueblo (Entrar con todo, 2026-10-22) ─────────────────────────────

impl Escenario {
    /// Una columna de esa Facción (un ejército de un héroe) en la puerta del pueblo: (columna, héroe).
    fn visita(&mut self, cuenta: &str, faccion: u32, pueblo: u64) -> (u64, u64) {
        let sitio = self.n.posicion_de_pueblo(pueblo).unwrap();
        let col = self.columna(cuenta, faccion, sitio);
        (col, self.n.ejercito(col).unwrap().columna.lider)
    }
    fn ubicacion(&self, heroe: u64) -> Ubicacion {
        self.n.heroe(heroe).unwrap().heroe.presencia.ubicacion
    }
    fn viveres(&self, columna: u64) -> i64 {
        self.n.ejercito(columna).unwrap().columna.viveres.valor_en(self.n.ahora())
    }
    fn carga(&self, columna: u64) -> std::collections::BTreeMap<Recurso, i64> {
        self.n.ejercito(columna).unwrap().columna.carga.clone()
    }
}

#[test]
fn se_entra_en_un_pueblo_con_todo_con_el_acceso_de_la_madre_se_come_de_los_viveres_y_se_sale_con_todo() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let pueblo = e.marcar();
    let sitio = e.n.posicion_de_pueblo(pueblo).unwrap();
    let (c0, h0) = e.visita("ana", 1, pueblo);
    // Una obra no tiene puerta: hasta que se levanta no se entra.
    assert_eq!(e.rechazada("ana", Comando::EntrarEnPueblo { pueblo, heroe: h0 }), rechazos::PUEBLO_NO_ESTA_ACTIVO);
    let materiales = e.llevar_materiales(pueblo);
    e.ok("ana", Comando::EntregarMateriales { ejercito: materiales, pueblo });
    let EstadoPueblo::EnObra { completa_en } = e.pueblo(pueblo).estado else { panic!("debía estar en obra") };
    e.n.avanzar_hasta(completa_en + MIN);
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(nucleo::Relacion::Alianza) });
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 4, relacion: Some(nucleo::Relacion::Guerra) });

    // Los rechazos, en su orden y sin dejar rastro.
    let lejos = e.columna("lia", 1, Punto::new(sitio.x + 5_000, sitio.y));
    let lia = e.n.ejercito(lejos).unwrap().columna.lider;
    let (_, enemigo) = e.visita("eno", 4, pueblo);
    e.avanzar(0); // lo que ya vencía en este instante, procesado: así «sin rastro» mide solo las órdenes
    let (v, hash) = (e.n.version(), e.n.hash_estado());
    assert_eq!(e.rechazada("ana", Comando::EntrarEnPueblo { pueblo: 9_999, heroe: h0 }), rechazos::PUEBLO_DESCONOCIDO);
    assert_eq!(e.rechazada("beto", Comando::EntrarEnPueblo { pueblo, heroe: h0 }), rechazos::NO_AUTORIZADO, "el héroe es de otro");
    assert_eq!(e.rechazada("lia", Comando::EntrarEnPueblo { pueblo, heroe: lia }), rechazos::PUEBLO_FUERA_DE_LA_PUERTA, "a más de radioPuerta no se entra");
    assert_eq!(e.rechazada("eno", Comando::EntrarEnPueblo { pueblo, heroe: enemigo }), rechazos::PUERTA_CERRADA, "la plaza madre no abre su puerta a los enemigos");
    assert_eq!(e.rechazada("ana", Comando::SalirDePueblo { pueblo, heroe: h0 }), rechazos::HEROE_NO_ESTA_EN_EL_PUEBLO);
    assert_eq!(e.rechazada("ana", Comando::SalirDePueblo { pueblo: 9_999, heroe: h0 }), rechazos::PUEBLO_DESCONOCIDO);
    assert_eq!((e.n.version(), e.n.hash_estado()), (v, hash));

    // Entra con todo lo suyo: tropa, carro y víveres quedan en su columna, que sigue existiendo pero fuera del mapa.
    e.ok("admin", Comando::DarSuministro { ejercito: c0, recurso: Madera, cantidad: 30_000 });
    let escuadra = e.ok("admin", Comando::DarEscuadra { heroe: h0, tropa: militar::catalogo::Tropa::MiliciaLanceros, cantidad: 25 }).creadas[0];
    let (carga0, viveres0) = (e.carga(c0), e.viveres(c0));
    let t = e.n.ahora();
    e.ok("ana", Comando::EntrarEnPueblo { pueblo, heroe: h0 });
    assert_eq!(e.ubicacion(h0), Ubicacion::Pueblo(pueblo));
    assert!(!e.n.en_el_mapa(c0), "dentro no está en el mapa: nadie la ve ni la ataca");
    assert_eq!((e.carga(c0), e.n.ejercito(c0).unwrap().columna.escuadras.clone()), (carga0.clone(), vec![escuadra]));
    let h = &hechos(&e.n, "jugador.entra_en_pueblo")[0];
    assert_eq!((h.0, h.1.clone(), dato(h, "heroe"), dato(h, "pueblo"), dato(h, "ejercito")), (t, vec![plaza], h0 as i64, pueblo as i64, c0 as i64));
    e.avanzar(HORA);
    assert!(e.viveres(c0) < viveres0, "come de sus propios víveres, como un ejército acampado");
    let dentro = e.viveres(c0);
    e.ok("ana", Comando::SalirDePueblo { pueblo, heroe: h0 });
    assert_eq!(e.ubicacion(h0), Ubicacion::Columna(c0));
    assert!(e.n.en_el_mapa(c0));
    assert_eq!((e.carga(c0), e.n.ejercito(c0).unwrap().posicion, e.viveres(c0)), (carga0, sitio, dentro), "sale con todo, a la puerta");
    assert_eq!(dato(&hechos(&e.n, "jugador.sale_de_pueblo")[0], "pueblo"), pueblo as i64);
    assert_eq!(e.rechazada("ana", Comando::SalirDePueblo { pueblo, heroe: h0 }), rechazos::HEROE_NO_ESTA_EN_EL_PUEBLO);

    // El aliado y quien no tiene relación entran (por defecto la puerta solo se cierra a los enemigos); un ejército entero entra por orden de su Líder y se desarma.
    let (_, aliado) = e.visita("alia", 2, pueblo);
    let (cn, neutral) = e.visita("neu", 3, pueblo);
    let segundo = e.ok("bia", Comando::CrearHeroe { cuenta: "bia".into(), campamento: 0, controlador: militar::heroe::Controlador::Humano, nombre: None }).creadas[0];
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: segundo, faccion: Some(3) });
    e.ok("admin", Comando::ColocarHeroe { heroe: segundo, ubicacion: Ubicacion::Columna(cn) });
    e.ok("alia", Comando::EntrarEnPueblo { pueblo, heroe: aliado });
    assert_eq!(e.rechazada("bia", Comando::EntrarEnPueblo { pueblo, heroe: segundo }), rechazos::COLUMNA_SOLO_EL_LIDER, "un ejército entra por orden de su Líder");
    e.ok("neu", Comando::EntrarEnPueblo { pueblo, heroe: neutral });
    assert_eq!((e.ubicacion(aliado), e.ubicacion(neutral), e.ubicacion(segundo)), (Ubicacion::Pueblo(pueblo), Ubicacion::Pueblo(pueblo), Ubicacion::Pueblo(pueblo)));
    let ej = hechos(&e.n, "ejercito.entra_en_pueblo").into_iter().find(|h| dato(h, "ejercito") == cn as i64).expect("el ejército de «neu»");
    assert_eq!((dato(&ej, "heroes"), dato(&ej, "pueblo")), (2, pueblo as i64));
    let c2 = e.n.heroe(segundo).unwrap().columna.expect("al desarmarse cada héroe conserva su columna");
    assert!(c2 != cn && !e.n.en_el_mapa(c2) && !e.n.en_el_mapa(cn));
    e.ok("bia", Comando::SalirDePueblo { pueblo, heroe: segundo });
    assert!(e.n.en_el_mapa(c2) && !e.n.en_el_mapa(cn), "cada uno sale por su cuenta");
}

#[test]
fn cerrar_la_puerta_de_la_plaza_saca_tambien_de_su_pueblo_a_quien_ya_no_puede_entrar_y_destruirlo_saca_a_los_demas() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let pueblo = e.pueblo_levantado();
    let sitio = e.n.posicion_de_pueblo(pueblo).unwrap();
    let centro = e.n.estado().asentamientos[&plaza].posicion;
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(nucleo::Relacion::Alianza) });
    // Dentro del pueblo: un neutral (F3), un aliado (F2), uno de la propia Facción (F1) y un residente de la plaza (F1). Dentro de la plaza: otro neutral.
    let mut dentro = std::collections::BTreeMap::new();
    for (cuenta, faccion) in [("neu", 3), ("alia", 2), ("pro", 1), ("res", 1)] {
        let (col, h) = e.visita(cuenta, faccion, pueblo);
        e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 10_000 });
        if cuenta == "res" {
            e.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
        }
        e.ok(cuenta, Comando::EntrarEnPueblo { pueblo, heroe: h });
        dentro.insert(cuenta, (col, h));
    }
    let colp = e.columna("nep", 3, centro);
    let nep = e.n.ejercito(colp).unwrap().columna.lider;
    e.ok("nep", Comando::Guarnecer { asentamiento: plaza, heroe: nep });
    assert_eq!(e.ubicacion(nep), Ubicacion::Asentamiento(plaza as u32));

    // Se cierra a los neutrales: salen el del pueblo y el de la plaza, cada uno con su columna y su carga y a la puerta de su sitio; los demás siguen.
    let t = e.n.ahora();
    e.ok("ana", comun::acceso(plaza, true, false, false));
    let (col_n, neu) = dentro["neu"];
    assert_eq!((e.ubicacion(neu), e.n.en_el_mapa(col_n), e.n.ejercito(col_n).unwrap().posicion, e.carga(col_n).get(&Madera).copied()), (Ubicacion::Columna(col_n), true, sitio, Some(10_000)));
    assert_eq!((e.ubicacion(nep), e.n.en_el_mapa(colp), e.n.ejercito(colp).unwrap().posicion), (Ubicacion::Columna(colp), true, centro));
    for cuenta in ["alia", "pro", "res"] {
        assert_eq!(e.ubicacion(dentro[cuenta].1), Ubicacion::Pueblo(pueblo), "{cuenta} sigue dentro");
    }
    let echados = hechos(&e.n, "heroe.expulsado");
    assert_eq!(echados.iter().map(|h| (dato(h, "heroe"), dato(h, "lugar"), dato(h, "sitio"), dato(h, "motivo"), h.0, h.1.clone())).collect::<Vec<_>>(), vec![(nep as i64, 0, plaza as i64, 0, t, vec![plaza]), (neu as i64, 1, pueblo as i64, 0, t, vec![plaza])]);

    // Se cierra también a los aliados: sale «alia»; la misma Facción y el residente se quedan.
    e.ok("ana", comun::acceso(plaza, false, false, false));
    assert_eq!(e.ubicacion(dentro["alia"].1), Ubicacion::Columna(dentro["alia"].0));
    assert_eq!((e.ubicacion(dentro["pro"].1), e.ubicacion(dentro["res"].1)), (Ubicacion::Pueblo(pueblo), Ubicacion::Pueblo(pueblo)));

    // Destruir el pueblo a mano saca a los que quedan con lo suyo, a su puerta.
    e.ok("ana", Comando::DestruirPueblo { pueblo });
    assert!(e.n.pueblo(pueblo).is_none());
    for cuenta in ["pro", "res"] {
        let (col, h) = dentro[cuenta];
        assert_eq!((e.ubicacion(h), e.n.en_el_mapa(col), e.n.ejercito(col).unwrap().posicion, e.carga(col).get(&Madera).copied()), (Ubicacion::Columna(col), true, sitio, Some(10_000)), "{cuenta}");
    }
    let ultimos: Vec<_> = hechos(&e.n, "heroe.expulsado").into_iter().filter(|h| dato(h, "motivo") == 3).collect();
    assert_eq!(ultimos.iter().map(|h| (dato(h, "lugar"), dato(h, "sitio"))).collect::<Vec<_>>(), vec![(1, pueblo as i64); 2], "el lugar se destruye");
}

#[test]
fn si_la_madre_cae_en_ruinas_salen_todos_de_su_pueblo_y_si_es_conquistada_solo_los_que_el_acceso_heredado_ya_no_deja_entrar() {
    let m = con_mundo!();
    // Ruina: sin auto-construcción ni madera que repongan el mantenimiento, la plaza acaba cayendo en ruinas.
    let mut e = montar(m, 7, 2);
    let (plaza, cobre) = (e.plaza, e.cobre);
    let pueblo = e.pueblo_levantado();
    let sitio = e.n.posicion_de_pueblo(pueblo).unwrap();
    let (col, h) = e.visita("neu", 3, pueblo);
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 10_000 });
    e.ok("neu", Comando::EntrarEnPueblo { pueblo, heroe: h });
    for dia in 1..=150 {
        let t = e.n.ahora().max(dia * 24 * HORA);
        e.n.avanzar_hasta(t);
        if e.n.economia(plaza).is_none() {
            break;
        }
    }
    assert!(e.n.economia(plaza).is_none() && e.n.pueblo(pueblo).is_none(), "la plaza cayó en ruinas y se llevó su pueblo");
    assert!(!e.n.estado().nodos_reclamados.contains_key(&cobre));
    assert_eq!((e.ubicacion(h), e.n.en_el_mapa(col), e.n.ejercito(col).unwrap().posicion, e.carga(col).get(&Madera).copied()), (Ubicacion::Columna(col), true, sitio, Some(10_000)));
    let echado = &hechos(&e.n, "heroe.expulsado")[0];
    assert_eq!((dato(echado, "motivo"), dato(echado, "lugar"), dato(echado, "sitio"), echado.1.clone()), (1, 1, pueblo as i64, vec![]), "el hecho no nombra a la plaza que ya no está");

    // Conquista: el pueblo pasa al conquistador (F2) con la puerta heredada, que vuelve a la de por defecto (Q1, 5E: cerrada a neutrales y enemigos). Dentro hay un neutral (F3, al que la
    // madre abrió la puerta), un aliado del conquistador (F5) y un enemigo suyo (F4, en guerra con F2): salen los que ya no podrían entrar, el neutral y el enemigo.
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let pueblo = e.pueblo_levantado();
    e.ok("admin", Comando::FijarRelacion { a: 2, b: 5, relacion: Some(nucleo::Relacion::Alianza) });
    e.ok("admin", Comando::FijarRelacion { a: 2, b: 4, relacion: Some(nucleo::Relacion::Guerra) });
    let mut visitas = vec![];
    for (cuenta, faccion) in [("neu", 3), ("alia", 5), ("ene", 4)] {
        let (col, h) = e.visita(cuenta, faccion, pueblo);
        e.ok(cuenta, Comando::EntrarEnPueblo { pueblo, heroe: h });
        visitas.push((col, h));
    }
    let p = e.n.estado().asentamientos[&plaza].posicion;
    let beto = e.columna("beto", 2, Punto::new(p.x + 2_000, p.y));
    let heroe = e.n.ejercito(beto).unwrap().columna.lider;
    let arqueros = militar::catalogo::Tropa::ArquerosCompuesto;
    e.ok("admin", Comando::DarEscuadra { heroe, tropa: arqueros, cantidad: militar::Reglas::cargar().tropas.tropa(arqueros).unidades });
    sin_proteccion(&mut e.n, plaza);
    e.ok("beto", Comando::Atacar { heroe, objetivo: nucleo::ObjetivoAtaque::Asentamiento(plaza), declara_guerra: true });
    tras_la_convocatoria(&mut e.n);
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, 2);
    assert!(e.n.pueblo(pueblo).is_some_and(|(madre, _)| madre == plaza));
    let [(col_neu, neu), (col_alia, alia), (col_ene, ene)] = visitas[..] else { unreachable!() };
    assert_eq!(e.ubicacion(alia), Ubicacion::Pueblo(pueblo), "el aliado del conquistador se queda dentro");
    assert!(!e.n.en_el_mapa(col_alia));
    assert_eq!((e.ubicacion(ene), e.n.en_el_mapa(col_ene)), (Ubicacion::Columna(col_ene), true), "el enemigo del conquistador sale al mapa con la columna con la que entró");
    assert_eq!((e.ubicacion(neu), e.n.en_el_mapa(col_neu)), (Ubicacion::Columna(col_neu), true), "y el neutral, con la puerta de por defecto");
    let echados = hechos(&e.n, "heroe.expulsado");
    assert_eq!(echados.len(), 2);
    for x in &echados {
        assert_eq!((dato(x, "motivo"), dato(x, "lugar"), x.1.clone()), (2, 1, vec![plaza]), "cambia de dueño");
    }
}

/// Al conquistar la plaza su puerta vuelve a la de por defecto (Q1, 5E: cerrada a neutrales y enemigos, respecto al dueño nuevo): lo que decidía el dueño viejo ya no cuenta, el aliado suyo que
/// estaba dentro del pueblo es ahora un neutral y sale, y el dueño nuevo la cambia luego cuando quiere (abrirla a los neutrales le deja volver a entrar).
#[test]
fn al_conquistar_la_madre_su_acceso_vuelve_al_de_por_defecto_y_el_pueblo_lo_hereda() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let pueblo = e.pueblo_levantado();
    // F3 es aliada del dueño (F1): entra aunque se cierre a los neutrales; con F2, que no tiene relación con ella, es neutral.
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 3, relacion: Some(nucleo::Relacion::Alianza) });
    e.ok("ana", comun::acceso(plaza, true, false, false));
    let (col, h) = e.visita("alia", 3, pueblo);
    e.ok("alia", Comando::EntrarEnPueblo { pueblo, heroe: h });
    assert_eq!(e.ubicacion(h), Ubicacion::Pueblo(pueblo));
    let p = e.n.estado().asentamientos[&plaza].posicion;
    let beto = e.columna("beto", 2, Punto::new(p.x + 2_000, p.y));
    let heroe = e.n.ejercito(beto).unwrap().columna.lider;
    let arqueros = militar::catalogo::Tropa::ArquerosCompuesto;
    e.ok("admin", Comando::DarEscuadra { heroe, tropa: arqueros, cantidad: militar::Reglas::cargar().tropas.tropa(arqueros).unidades });
    sin_proteccion(&mut e.n, plaza);
    e.ok("beto", Comando::Atacar { heroe, objetivo: nucleo::ObjetivoAtaque::Asentamiento(plaza), declara_guerra: true });
    tras_la_convocatoria(&mut e.n);
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, 2);
    assert_eq!(e.n.estado().asentamientos[&plaza].puerta, Default::default(), "la puerta vuelve a la de por defecto");
    assert_eq!((e.ubicacion(h), e.n.en_el_mapa(col)), (Ubicacion::Columna(col), true), "con la puerta de por defecto el neutral (respecto al dueño nuevo) sale al mapa con lo suyo");
    // El dueño nuevo la cambia cuando quiere: abierta a los neutrales, puede volver a entrar.
    assert_eq!(e.n.puede_entrar_en_pueblo(h, pueblo), Some(false));
    e.ok("beto", comun::acceso(plaza, true, true, false));
    assert_eq!(e.n.puede_entrar_en_pueblo(h, pueblo), Some(true));
}

/// El acceso reiniciado sigue cerrando la puerta a los enemigos del dueño nuevo: el visitante del pueblo que está en guerra con el conquistador sale al mapa con lo suyo.
#[test]
fn al_conquistar_la_madre_con_el_acceso_por_defecto_salen_del_pueblo_los_enemigos_del_conquistador() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let pueblo = e.pueblo_levantado();
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 3, relacion: Some(nucleo::Relacion::Alianza) });
    e.ok("admin", Comando::FijarRelacion { a: 2, b: 3, relacion: Some(nucleo::Relacion::Guerra) });
    let (col, h) = e.visita("alia", 3, pueblo);
    e.ok("alia", Comando::EntrarEnPueblo { pueblo, heroe: h });
    assert_eq!(e.ubicacion(h), Ubicacion::Pueblo(pueblo));
    let p = e.n.estado().asentamientos[&plaza].posicion;
    let beto = e.columna("beto", 2, Punto::new(p.x + 2_000, p.y));
    let heroe = e.n.ejercito(beto).unwrap().columna.lider;
    let arqueros = militar::catalogo::Tropa::ArquerosCompuesto;
    e.ok("admin", Comando::DarEscuadra { heroe, tropa: arqueros, cantidad: militar::Reglas::cargar().tropas.tropa(arqueros).unidades });
    sin_proteccion(&mut e.n, plaza);
    e.ok("beto", Comando::Atacar { heroe, objetivo: nucleo::ObjetivoAtaque::Asentamiento(plaza), declara_guerra: true });
    tras_la_convocatoria(&mut e.n);
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, 2);
    assert_eq!((e.ubicacion(h), e.n.en_el_mapa(col)), (Ubicacion::Columna(col), true), "el acceso por defecto no deja entrar a los enemigos del dueño nuevo: sale con lo suyo");
}

// ───────────────────────────── 7. Un guion con héroes dentro: determinismo y foto ─────────────────────────────

/// Un guion con un pueblo y gente dentro: se levanta, un aliado y un neutral entran con su columna, se cierra la puerta a los neutrales (sale uno) y el aliado sale por su cuenta.
/// Cada orden lleva su instante: `avanzar` solo decide cómo se mueve el reloj entre ellas.
fn guion_con_gente(e: &mut Escenario, avanzar: &dyn Fn(&mut Escenario, Instante)) {
    let plaza = e.plaza;
    let id = e.marcar();
    let col = e.llevar_materiales(id);
    e.ok_en(0, "ana", Comando::EntregarMateriales { ejercito: col, pueblo: id });
    e.ok_en(0, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(nucleo::Relacion::Alianza) });
    // Las columnas de los visitantes nacen al principio, en el mismo instante sea cual sea el modo de avanzar.
    let (_, aliado) = e.visita("alia", 2, id);
    let (_, neutral) = e.visita("neu", 3, id);
    avanzar(e, 8 * HORA);
    e.ok_en(8 * HORA, "alia", Comando::EntrarEnPueblo { pueblo: id, heroe: aliado });
    e.ok_en(8 * HORA, "neu", Comando::EntrarEnPueblo { pueblo: id, heroe: neutral });
    avanzar(e, 9 * HORA);
    e.ok_en(9 * HORA, "ana", comun::acceso(plaza, true, false, false));
    avanzar(e, 10 * HORA);
    e.ok_en(10 * HORA, "alia", Comando::SalirDePueblo { pueblo: id, heroe: aliado });
    avanzar(e, 20 * HORA);
}

fn correr_con_gente(m: &M, semilla: u64, avanzar: &dyn Fn(&mut Escenario, Instante)) -> Nucleo {
    let mut e = montar(m, semilla, 2);
    guion_con_gente(&mut e, avanzar);
    e.n.avanzar_hasta(20 * HORA);
    e.n
}

#[test]
fn entrar_y_salir_de_un_pueblo_da_el_mismo_hash_normal_acelerado_y_a_trozos() {
    let m = con_mundo!();
    let normal = correr_con_gente(m, 3, &|e, t| e.n.avanzar_hasta(t));
    assert_eq!(normal.hash_estado(), correr_con_gente(m, 3, &|e, t| e.n.avanzar_hasta(t)).hash_estado(), "repetible");
    assert_eq!(normal.hash_estado(), correr_con_gente(m, 3, &|_, _| {}).hash_estado(), "acelerado de un golpe");
    let a_trozos = correr_con_gente(m, 3, &|e, t| {
        let ahora = e.n.ahora();
        for s in 1..=5 {
            e.n.avanzar_hasta(ahora + (t - ahora).max(0) * s * s / 25);
        }
    });
    assert_eq!(normal.hash_estado(), a_trozos.hash_estado(), "en pasos pequeños e irregulares, como el modo en vivo");
    // El guion hizo de verdad lo que se espera.
    assert_eq!(hechos(&normal, "jugador.entra_en_pueblo").len(), 2);
    assert_eq!(hechos(&normal, "heroe.expulsado").len(), 1);
    assert_eq!(hechos(&normal, "jugador.sale_de_pueblo").len(), 1);
    // Nada recurre por una entidad ociosa: como mucho un evento de deserción vigente por columna, esté dentro de un pueblo o en el mapa.
    let vigentes = normal.estado().cola.volcar().iter().filter(|ev| normal.estado().generaciones.es_vigente(ev.clave) && matches!(ev.carga, Carga::Militar(SucesoMilitar::Desercion))).count();
    assert!(vigentes <= normal.estado().ejercitos.len(), "{vigentes} deserciones vigentes");
}

#[test]
fn una_foto_con_un_heroe_dentro_de_un_pueblo_se_recupera_y_la_anterior_se_rechaza() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9) };
    let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
    let mut e = montar_con(n, m, 2);
    let pueblo = e.pueblo_levantado();
    let (col, h) = e.visita("neu", 3, pueblo);
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 10_000 });
    e.ok("neu", Comando::EntrarEnPueblo { pueblo, heroe: h });
    let bytes = e.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN50");
    let vuelta = nucleo::Estado::deserializar(&bytes, &Reglas::default(), &e.n.estado().mundo_hash).unwrap();
    assert_eq!(vuelta.militar.heroes[&h].heroe.presencia.ubicacion, Ubicacion::Pueblo(pueblo));
    assert_eq!((vuelta.serializar(), vuelta.hash()), (bytes.clone(), e.n.estado().hash()));
    let viejo = [b"BAN17".as_slice(), &bytes[5..]].concat();
    assert!(nucleo::Estado::deserializar(&viejo, &Reglas::default(), &e.n.estado().mundo_hash).is_err(), "BAN17 no traía héroes dentro de un pueblo");
    // Foto + registro: se recupera con el héroe dentro y sigue igual que el que no se interrumpió.
    e.n.foto_ahora();
    e.avanzar(HORA);
    let (hash, ubicacion, plaza, cuenta) = (e.n.hash_estado(), e.ubicacion(h), e.plaza, e.cuenta);
    e.n.apagar().unwrap();
    assert_eq!(ubicacion, Ubicacion::Pueblo(pueblo));
    let n = Nucleo::con_persistencia(con(ajustes), dir.path(), Default::default()).unwrap();
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), hash, "foto + registro reconstruyen el mismo mundo, con el héroe dentro del pueblo");
    let cobre = n.pueblo(pueblo).unwrap().1.nodo().unwrap();
    let mut e = Escenario { n, plaza, cobre, mapa: Mapa::de(&m.0), cuenta };
    assert_eq!(e.ubicacion(h), Ubicacion::Pueblo(pueblo));
    e.ok("neu", Comando::SalirDePueblo { pueblo, heroe: h });
    assert!(e.n.en_el_mapa(col));
    e.n.apagar().unwrap();
}

// ───────────────────────────── La niebla: el pueblo ajeno se recuerda «como una ciudad» (4H T3d) ─────────────────────────────

#[test]
fn el_pueblo_ajeno_que_se_vio_en_obra_se_recuerda_en_obra_aunque_despues_se_termine() {
    use nucleo::memoria::{EstadoSitio, Sitio};
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let pueblo = e.marcar();
    let col = e.llevar_materiales(pueblo);
    e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo });
    let EstadoPueblo::EnObra { completa_en } = e.pueblo(pueblo).estado else { panic!("debía estar en obra") };
    let sitio = e.n.posicion_de_pueblo(pueblo).unwrap();
    // Una columna de F2 lo ve en obra al pasar y se va lejos antes de que acabe.
    let tierra = |r: i64| (0..36).map(|g| ((g * 10) as f64).to_radians()).map(|a| Punto::new(sitio.x + (r as f64 * a.cos()) as i64, sitio.y + (r as f64 * a.sin()) as i64)).find(|q| m.0.transitable_en(q.x, q.y)).expect("tierra");
    let (junto, lejos) = (tierra(10_000), tierra(60_000));
    let beto = e.columna("beto", 2, junto);
    e.avanzar(MIN);
    e.ok("beto", Comando::MarcharA { ejercito: beto, destino: lejos });
    let ficha = |n: &Nucleo| n.estado().memoria.fichas[&2][&Sitio::Pueblo(pueblo)].datos.estado;
    assert_eq!(ficha(&e.n), EstadoSitio::EnObra, "lo vio en obra");
    let fin = e.n.ejercito(beto).unwrap().plan.fin().unwrap();
    e.n.avanzar_hasta(fin.max(completa_en) + MIN);
    assert!(e.pueblo(pueblo).activo());
    assert_eq!(ficha(&e.n), EstadoSitio::EnObra, "se recuerda como estaba cuando se vio, no como está");
}

#[test]
fn el_pueblo_ajeno_se_recuerda_como_una_ciudad_y_se_olvida_cuando_se_vuelve_a_mirar_su_sitio_y_ya_no_esta() {
    use contrato::v1::{entidad::Cuerpo, TipoEntidad};
    use nucleo::memoria::Sitio;
    use nucleo::proyeccion::vista::{Mirada, Perspectiva};
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let pueblo = e.pueblo_levantado();
    let sitio = e.n.posicion_de_pueblo(pueblo).unwrap();
    // Una columna de F2 (un héroe solo: ve 15) espera a 10 u del pueblo y se va lejos.
    let tierra = |r: i64| (0..36).map(|g| ((g * 10) as f64).to_radians()).map(|a| Punto::new(sitio.x + (r as f64 * a.cos()) as i64, sitio.y + (r as f64 * a.sin()) as i64)).find(|q| m.0.transitable_en(q.x, q.y)).expect("tierra");
    let (junto, lejos) = (tierra(10_000), tierra(60_000));
    let col = e.columna("beto", 2, junto);
    e.avanzar(MIN);
    e.ok("beto", Comando::MarcharA { ejercito: col, destino: lejos });
    let fin = e.n.ejercito(col).unwrap().plan.fin().unwrap();
    e.n.avanzar_hasta(fin + MIN);
    let ficha = e.n.estado().memoria.fichas[&2][&Sitio::Pueblo(pueblo)];
    assert_eq!((ficha.datos.faccion, ficha.datos.posicion), (1, sitio), "lo vio al pasar y lo recuerda");
    let id = nucleo::proyeccion::ids::pueblo(pueblo);
    let recordado = |n: &Nucleo| {
        let rect = vision::Rect { min: Punto::new(0, 0), max: Punto::new(2_667_000, 2_000_000) };
        Mirada::de(n, Perspectiva::Faccion(2)).contenido(None, rect, &Default::default()).into_iter().find(|v| v.clave == (TipoEntidad::FichaConocida as i32, id.clone())).and_then(|v| v.cuerpo())
    };
    let Some(Cuerpo::FichaConocida(f)) = recordado(&e.n) else { panic!("la vista lo recuerda") };
    assert_eq!((f.tipo, f.faccion_id.as_str(), f.nivel, f.radio_potencial, f.estado), (TipoEntidad::Pueblo as i32, "f-1", 0, None, contrato::v1::EstadoSitio::SitioActivo as i32));

    // Ana lo destruye: mientras F2 no vuelva a mirar el sitio, la foto vieja se queda.
    e.ok("ana", Comando::DestruirPueblo { pueblo });
    assert!(e.n.estado().memoria.fichas[&2].contains_key(&Sitio::Pueblo(pueblo)));
    assert!(recordado(&e.n).is_some());
    // La columna vuelve a su sitio: ya no está, y se olvida (una ciudad que no existe no es vieja, es falsa).
    e.ok("beto", Comando::MarcharA { ejercito: col, destino: junto });
    let fin = e.n.ejercito(col).unwrap().plan.fin().unwrap();
    e.n.avanzar_hasta(fin + MIN);
    assert!(e.n.estado().memoria.fichas.get(&2).is_none_or(|f| !f.contains_key(&Sitio::Pueblo(pueblo))), "olvidado");
    assert!(recordado(&e.n).is_none());
}

/// Un asedio con números se resuelve al acabar su convocatoria de 30 s (calibración del sistema 4, M8): deja pasar las que haya abiertas.
fn tras_la_convocatoria(n: &mut Nucleo) {
    let fin = n.batallas_activas().filter_map(|b| b.convocatoria_hasta).max();
    if let Some(h) = fin {
        n.avanzar_hasta(h);
    }
}

/// Deja pasar el día de protección con que nace la plaza (Doc 1.2): después se la puede asediar.
fn sin_proteccion(n: &mut Nucleo, plaza: u64) {
    if let Some(h) = n.protegida_hasta(plaza) {
        let t = h.max(n.ahora());
        n.avanzar_hasta(t);
    }
}


// ───────────────────────────── 8. La aldea agrícola y su rebaño (mecánicas 50 y 51, 2026-10-27) ─────────────────────────────

impl Escenario {
    /// Una columna de Ana en la puerta del pueblo con lo que le falta a su obra (o a su mejora), que lo entrega.
    fn entregar_lo_que_falta(&mut self, pueblo: u64) {
        let sitio = self.n.posicion_de_pueblo(pueblo).unwrap();
        let col = self.columna("ana", 1, sitio);
        for (recurso, cantidad) in self.pueblo(pueblo).falta() {
            self.ok("admin", Comando::DarSuministro { ejercito: col, recurso, cantidad });
        }
        self.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo });
    }
}

#[test]
fn la_aldea_se_marca_en_tierra_fertil_nace_con_su_rebano_y_su_carro_lleva_trigo_y_ganado_a_la_madre() {
    let m = con_mundo!();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let dir = tempfile::tempdir().unwrap();
    let n = Nucleo::con_persistencia(Arranque { ajustes: ajustes.clone(), ..arranque(m, 7) }, dir.path(), Default::default()).unwrap();
    // Un sitio con tierra pobre en algún punto de su zona de nivel 2 (de 3,5 a 15 u), para probar el rechazo.
    let pobre_cerca = |c: Punto| (0..24).flat_map(|k| (0..24).map(move |g| (3_500 + k * 500, g))).any(|(d, g)| {
        let a = ((g * 15) as f64).to_radians();
        let q = Punto::new(c.x + (d as f64 * a.cos()) as i64, c.y + (d as f64 * a.sin()) as i64);
        m.0.transitable_en(q.x, q.y) && m.0.fertilidad_en(q.x, q.y).is_some_and(|f| f <= 76)
    });
    let mut e = montar_donde(n, m, 2, pobre_cerca);
    let plaza = e.plaza;
    let centro = e.n.estado().asentamientos[&plaza].posicion;
    // Junto a la plaza, fuera de la zona, o en tierra pobre: no.
    assert_eq!(e.rechazada("ana", Comando::MarcarAldea { asentamiento: plaza, punto: Punto::new(centro.x + 1_000, centro.y) }), rechazos::PUEBLO_DEMASIADO_CERCA);
    assert_eq!(e.rechazada("ana", Comando::MarcarAldea { asentamiento: plaza, punto: Punto::new(centro.x + 400_000, centro.y) }), rechazos::PUEBLO_FUERA_DE_LA_ZONA);
    let pobre = (0..24).flat_map(|k| (0..24).map(move |g| (3_500 + k * 500, g))).any(|(d, g)| {
        let a = ((g * 15) as f64).to_radians();
        let punto = Punto::new(centro.x + (d as f64 * a.cos()) as i64, centro.y + (d as f64 * a.sin()) as i64);
        // Solo donde el artefacto dice que la tierra es pobre: en uno fértil se marcaría la aldea.
        if !m.0.transitable_en(punto.x, punto.y) || m.0.fertilidad_en(punto.x, punto.y).is_none_or(|f| f > 76) {
            return false;
        }
        let r = e.ord("ana", Comando::MarcarAldea { asentamiento: plaza, punto });
        matches!(r.resultado, Resultado::Rechazada { codigo, .. } if codigo == rechazos::PUEBLO_FERTILIDAD_INSUFICIENTE)
    });
    assert!(pobre, "en la tierra pobre de la zona no se marca la aldea");
    assert!(e.n.pueblos_de(plaza).is_empty(), "los rechazos no dejan rastro");

    let id = comun::levantar_aldea(&mut e.n, "ana", plaza).expect("hay tierra fértil en la zona");
    // (Con persistencia, la crónica no se lee desde el principio: se miran los estados.)
    let p = e.pueblo(id);
    assert!(p.es_aldea() && p.nodo().is_none() && p.rebano.is_none());
    let EstadoPueblo::EnObra { completa_en } = p.estado else { panic!("con todo entregado, en obra") };
    e.n.avanzar_hasta(completa_en + MIN);
    let p = e.pueblo(id);
    assert!(p.activo());
    assert_eq!(p.rebano.as_ref().map(|r| (r.especie.as_str(), r.cabezas / 1_000)), Some(("vacuno", 20)), "nace con el pie de cría de su corral");
    assert!(e.n.economia(plaza).unwrap().aldeas_activas() == 1);

    // Su carro sale con trigo y, cuando el rebaño pasa del pie de cría, también con ganado; los dos entran en el almacén de la madre.
    let limite = e.n.ahora() + 12 * HORA;
    let trae = |e: &Escenario, r: Recurso| e.n.economia(plaza).unwrap().ingresos_acarreo.iter().any(|i| i.recurso == r);
    while !(trae(&e, Trigo) && trae(&e, Livestock)) {
        e.avanzar(10 * MIN);
        assert!(e.n.ahora() < limite, "el carro debería haber traído trigo y ganado: {:?}", e.n.economia(plaza).unwrap().ingresos_acarreo);
    }

    // Sube de nivel con otra obra de mapa: sigue produciendo mientras espera los materiales.
    e.ok("ana", Comando::MejorarAldea { pueblo: id });
    assert!(matches!(e.pueblo(id).mejora, Some(EstadoPueblo::Pendiente { .. })));
    assert_eq!(e.rechazada("ana", Comando::MejorarAldea { pueblo: id }), rechazos::PUEBLO_MEJORA_EN_CURSO);
    assert!(e.pueblo(id).activo());
    e.entregar_lo_que_falta(id);
    let Some(EstadoPueblo::EnObra { completa_en }) = e.pueblo(id).mejora.clone() else { panic!("la mejora, en obra") };

    // Una foto con la mejora en obra y el rebaño criando recupera el mismo mundo.
    e.n.foto_ahora();
    e.avanzar(10 * MIN);
    let (hash, cuenta, ahora) = (e.n.hash_estado(), e.cuenta, e.n.ahora());
    e.n.apagar().unwrap();
    let n = Nucleo::con_persistencia(Arranque { ajustes, ..arranque(m, 7) }, dir.path(), Default::default()).unwrap();
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), hash, "foto + registro reconstruyen la aldea, su mejora y su rebaño");
    let mut e = Escenario { n, plaza, cobre: e.cobre, mapa: e.mapa.clone(), cuenta };
    assert_eq!(e.n.ahora(), ahora);
    e.n.avanzar_hasta(completa_en);
    assert_eq!(e.pueblo(id).nivel, 2);
    assert_eq!(e.pueblo(id).mejora, None);
    assert_eq!(e.n.pueblo(id).map(|(madre, _)| madre), Some(plaza));
    e.n.apagar().unwrap();
}

// ───────────────────────────── 9. Dentro del pueblo: guarnición, almacén local, escolta y mejora (§14.0.1, 2026-10-29) ─────────────────────────────

use militar::catalogo::Tropa;
use militar::escuadra::Contenedor;

/// Liderazgo de una escuadra de `tropa`.
fn coste(tropa: Tropa) -> i64 {
    militar::liderazgo::coste(&militar::Reglas::cargar(), tropa)
}

impl Escenario {
    /// Una orden sin el andamiaje de [`Escenario::peticion`]: pasa la autorización real aunque la cuenta tenga héroes de la Facción dueña.
    fn real(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.cuenta += 1;
        let (t, p) = (self.n.ahora(), Peticion::nueva(actor, &format!("o{}", self.cuenta), c));
        self.n.aplicar_en(t, p)
    }
    fn milicia(&mut self, heroe: u64) -> u64 {
        self.ok("admin", Comando::DarEscuadra { heroe, tropa: Tropa::MiliciaLanceros, cantidad: 25 }).creadas[0]
    }
    fn contenedor(&self, heroe: u64, escuadra: u64) -> Contenedor {
        self.n.heroe(heroe).unwrap().heroe.escuadra(escuadra).unwrap().contenedor
    }
    fn en_columna(&self, columna: u64) -> Vec<u64> {
        self.n.ejercito(columna).unwrap().columna.escuadras.clone()
    }
}

#[test]
fn dentro_un_ciudadano_de_la_faccion_deja_escuadras_de_su_columna_de_guarnicion_con_su_cupo_y_las_recoge() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let pueblo = e.pueblo_levantado();
    let cupo = e.n.cupo_de_guarnicion_de_pueblo(pueblo).unwrap();
    assert_eq!(cupo, 30, "un extractor es siempre de nivel 1");
    let caben = (cupo / coste(Tropa::MiliciaLanceros)) as usize;
    assert!(caben >= 1);
    let (c0, h0) = e.visita("ana", 1, pueblo);
    let escuadras: Vec<u64> = (0..=caben).map(|_| e.milicia(h0)).collect();
    let (_, neutral) = e.visita("neu", 3, pueblo);
    let suya = e.milicia(neutral);
    e.ok("neu", Comando::EntrarEnPueblo { pueblo, heroe: neutral });

    // Los rechazos, sin rastro.
    e.avanzar(0);
    let (v, hash) = (e.n.version(), e.n.hash_estado());
    let guarnecer = |escuadras: &[u64]| Comando::GuarnecerPueblo { heroe: h0, pueblo, escuadras: escuadras.to_vec() };
    assert_eq!(e.rechazada("ana", guarnecer(&escuadras[..1])), rechazos::HEROE_NO_ESTA_EN_EL_PUEBLO, "desde fuera, no");
    assert_eq!(e.rechazada("neu", Comando::GuarnecerPueblo { heroe: neutral, pueblo, escuadras: vec![suya] }), rechazos::PUEBLO_FACCION_AJENA, "solo los ciudadanos de la Facción de la madre");
    assert_eq!((e.n.version(), e.n.hash_estado()), (v, hash));
    e.ok("ana", Comando::EntrarEnPueblo { pueblo, heroe: h0 });
    e.avanzar(0);
    let (v, hash) = (e.n.version(), e.n.hash_estado());
    assert_eq!(e.rechazada("ana", guarnecer(&escuadras)), rechazos::GUARNICION_SUPERA_EL_CUPO);
    assert_eq!(e.rechazada("ana", guarnecer(&[])), rechazos::SALIDA_SIN_ESCUADRAS);
    assert_eq!(e.rechazada("ana", guarnecer(&[escuadras[0], escuadras[0]])), rechazos::ESCUADRA_REPETIDA);
    assert_eq!(e.rechazada("ana", guarnecer(&[suya])), rechazos::ESCUADRA_AJENA);
    assert_eq!(e.rechazada("ana", Comando::RetirarDePueblo { heroe: h0, pueblo, escuadras: vec![escuadras[0]] }), rechazos::PUEBLO_ESCUADRA_NO_ESTA);
    assert_eq!((e.n.version(), e.n.hash_estado()), (v, hash));

    // Las que caben en su cupo salen de la columna a la guarnición; la que sobra sigue en la columna.
    e.ok("ana", guarnecer(&escuadras[..caben]));
    assert!(escuadras[..caben].iter().all(|&s| e.contenedor(h0, s) == Contenedor::Pueblo(pueblo)));
    assert_eq!(e.en_columna(c0), vec![escuadras[caben]]);
    assert_eq!(e.n.guarnicion_de_jugadores_de_pueblo(pueblo).len(), caben);
    let h = &hechos(&e.n, "pueblo.guarnecido")[0];
    assert_eq!((dato(h, "pueblo"), dato(h, "heroe"), dato(h, "escuadras")), (pueblo as i64, h0 as i64, caben as i64));
    assert_eq!(e.rechazada("ana", guarnecer(&escuadras[caben..])), rechazos::GUARNICION_SUPERA_EL_CUPO, "el cupo es de todo lo suyo en el pueblo");
    assert_eq!(e.rechazada("ana", guarnecer(&escuadras[..1])), rechazos::PUEBLO_ESCUADRA_FUERA_DE_TU_COLUMNA);

    // El cupo es por héroe: otro ciudadano deja la suya aunque el de Ana esté lleno.
    let (_, lia) = e.visita("lia", 1, pueblo);
    let de_lia = e.milicia(lia);
    e.ok("lia", Comando::EntrarEnPueblo { pueblo, heroe: lia });
    e.ok("lia", Comando::GuarnecerPueblo { heroe: lia, pueblo, escuadras: vec![de_lia] });
    assert_eq!(e.n.guarnicion_de_jugadores_de_pueblo(pueblo).len(), caben + 1);

    // Recoge una a la columna con la que entró; la guarnición se queda aunque su héroe salga.
    e.ok("ana", Comando::RetirarDePueblo { heroe: h0, pueblo, escuadras: vec![escuadras[0]] });
    assert_eq!(e.contenedor(h0, escuadras[0]), Contenedor::Ejercito(c0));
    assert_eq!(e.en_columna(c0), vec![escuadras[caben], escuadras[0]]);
    assert_eq!(dato(&hechos(&e.n, "pueblo.guarnicion_retirada")[0], "escuadras"), 1);
    e.ok("ana", Comando::SalirDePueblo { pueblo, heroe: h0 });
    e.ok("lia", Comando::SalirDePueblo { pueblo, heroe: lia });
    assert_eq!(e.n.guarnicion_de_jugadores_de_pueblo(pueblo).len(), caben);

    // Si su héroe deja la Facción de la madre, su guarnición vuelve intacta a su campamento; la de los demás se queda.
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: lia, faccion: Some(2) });
    assert_eq!(e.contenedor(lia, de_lia), Contenedor::Campamento);
    assert_eq!(e.n.guarnicion_de_jugadores_de_pueblo(pueblo).len(), caben - 1);
    // Y si el pueblo desaparece, también.
    e.ok("ana", Comando::DestruirPueblo { pueblo });
    assert!(escuadras[1..caben].iter().all(|&s| e.contenedor(h0, s) == Contenedor::Campamento));
    assert!(e.n.heroe(h0).unwrap().heroe.escuadras.iter().all(|s| s.cantidad == 25), "intactas");
}

#[test]
fn dentro_se_deja_en_el_almacen_local_solo_lo_que_produce_el_pueblo_y_se_carga_lo_que_haya_hasta_llenar_el_carro() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let pueblo = e.pueblo_levantado();
    let carro = e.pueblo(pueblo).carro.unwrap();
    let capacidad = e.n.estado().reglas.economia.datos.pueblos.capacidad_almacen;
    let (c0, h0) = e.visita("ana", 1, pueblo);
    e.ok("admin", Comando::DarSuministro { ejercito: c0, recurso: Cobre, cantidad: capacidad });
    let dejar = |recurso, cantidad| Comando::DejarEnPueblo { heroe: h0, pueblo, recurso, cantidad };
    let cargar = |recurso, cantidad| Comando::CargarDePueblo { heroe: h0, pueblo, recurso, cantidad };
    assert_eq!(e.rechazada("ana", dejar(Cobre, 1_000)), rechazos::HEROE_NO_ESTA_EN_EL_PUEBLO);
    e.ok("ana", Comando::EntrarEnPueblo { pueblo, heroe: h0 });
    e.avanzar(0);
    let (v, hash) = (e.n.version(), e.n.hash_estado());
    assert_eq!(e.rechazada("ana", dejar(Madera, 1_000)), rechazos::PUEBLO_RECURSO_AJENO, "solo se deja lo que produce el pueblo");
    assert_eq!(e.rechazada("ana", dejar(Cobre, 0)), rechazos::CANTIDAD_NO_POSITIVA);
    assert_eq!(e.rechazada("ana", cargar(Madera, 1_000)), rechazos::PUEBLO_NADA_QUE_CARGAR);
    assert_eq!((e.n.version(), e.n.hash_estado()), (v, hash));

    // Con el carro de acarreo en casa, dejar hasta llenar el almacén lo hace salir con todo.
    let habia = e.pueblo(pueblo).almacen.cantidad(Cobre);
    assert_eq!(e.n.caravana(carro).unwrap().estado, EstadoCaravana::Disponible, "en casa");
    e.ok("ana", dejar(Cobre, capacidad));
    let h = &hechos(&e.n, "pueblo.dejado")[0];
    assert_eq!((dato(h, "cantidad"), dato(h, "heroe")), (capacidad - habia, h0 as i64));
    assert_eq!(e.carga(c0).get(&Cobre).copied().unwrap_or(0), habia, "lo que no cupo se queda en el carro");
    assert_eq!(e.n.caravana(carro).unwrap().estado, EstadoCaravana::EnTransito, "el almacén lleno hace salir el carro");
    assert_eq!(e.pueblo(pueblo).almacen.cantidad(Cobre), 0);

    // Sin el carro en casa se llena y se queda lleno.
    let falta = capacidad - habia;
    e.ok("admin", Comando::DarSuministro { ejercito: c0, recurso: Cobre, cantidad: falta });
    e.ok("ana", dejar(Cobre, 2 * capacidad));
    assert_eq!(e.pueblo(pueblo).almacen.cantidad(Cobre), capacidad);
    assert!(e.carga(c0).is_empty());
    assert_eq!(e.rechazada("ana", dejar(Cobre, 1_000)), rechazos::PUEBLO_NADA_QUE_DEJAR);
    e.ok("admin", Comando::DarSuministro { ejercito: c0, recurso: Cobre, cantidad: 1_000 });
    assert_eq!(e.rechazada("ana", dejar(Cobre, 1_000)), rechazos::PUEBLO_ALMACEN_SIN_SITIO);

    // Se carga lo que haya hasta donde quepa en el carro de la columna.
    e.ok("ana", cargar(Cobre, 2 * capacidad));
    assert_eq!(e.carga(c0)[&Cobre], capacidad, "el carro de la columna admite lo mismo que el almacén");
    assert_eq!(e.pueblo(pueblo).almacen.cantidad(Cobre), 1_000);
    assert_eq!(dato(&hechos(&e.n, "pueblo.cargado")[0], "cantidad"), capacidad - 1_000);
    assert_eq!(e.rechazada("ana", cargar(Cobre, 1_000)), rechazos::CARRO_LLENO);
    // Y sale con ello.
    e.ok("ana", Comando::SalirDePueblo { pueblo, heroe: h0 });
    assert_eq!(e.carga(c0)[&Cobre], capacidad);
}

#[test]
fn dentro_un_ciudadano_de_la_faccion_escolta_el_carro_de_acarreo_con_escuadras_de_su_columna_y_se_las_lleva_al_quitarla() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    // El cupo de escolta es el del Mercado de la madre, como desde la plaza.
    e.dar(Madera, 350_000);
    e.ok("ana", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
    e.avanzar(9 * HORA);
    let pueblo = e.pueblo_levantado();
    let carro = e.pueblo(pueblo).carro.unwrap();
    // Carla no reside en la madre: le basta con ser de su Facción y estar dentro con su columna.
    let (c0, carla) = e.visita("carla", 1, pueblo);
    let escuadra = e.milicia(carla);
    let (_, neutral) = e.visita("neu", 3, pueblo);
    let suya = e.milicia(neutral);
    e.ok("neu", Comando::EntrarEnPueblo { pueblo, heroe: neutral });
    assert_eq!(e.rechazada("neu", Comando::AsignarEscolta { caravana: carro, heroe: neutral, escuadras: vec![suya] }), rechazos::PUEBLO_FACCION_AJENA);
    e.ok("carla", Comando::EntrarEnPueblo { pueblo, heroe: carla });
    e.ok("carla", Comando::AsignarEscolta { caravana: carro, heroe: carla, escuadras: vec![escuadra] });
    assert_eq!(e.n.caravana(carro).unwrap().escolta.iter().map(|x| x.id).collect::<Vec<_>>(), vec![escuadra]);
    assert_eq!(e.contenedor(carla, escuadra), Contenedor::Escolta(carro));
    assert!(e.en_columna(c0).is_empty());
    // Al quitarla desde dentro vuelve a la columna con la que entró, no a un campamento.
    e.ok("carla", Comando::QuitarEscolta { caravana: carro, heroe: carla, escuadras: None });
    assert!(e.n.caravana(carro).unwrap().escolta.is_empty());
    assert_eq!(e.contenedor(carla, escuadra), Contenedor::Ejercito(c0));
    assert_eq!(e.en_columna(c0), vec![escuadra]);
}

#[test]
fn el_gobernador_o_el_maestro_de_obras_de_la_madre_mejora_la_aldea_desde_dentro_y_sube_su_cupo_de_guarnicion() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2);
    let plaza = e.plaza;
    let aldea = comun::levantar_aldea(&mut e.n, "ana", plaza).expect("hay tierra fértil en la zona");
    let EstadoPueblo::EnObra { completa_en } = e.pueblo(aldea).estado else { panic!("con todo entregado, en obra") };
    e.n.avanzar_hasta(completa_en + MIN);
    assert_eq!(e.n.cupo_de_guarnicion_de_pueblo(aldea), Some(30));
    // Mara reside en la madre y es su Maestra de Obras; fuera de la plaza solo puede mandarlo dentro de la aldea.
    let (_, mara) = e.visita("mara", 1, aldea);
    e.ok("admin", Comando::FijarResidencia { heroe: mara, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    let mejorar = Comando::MejorarAldea { pueblo: aldea };
    assert!(!e.real("mara", mejorar.clone()).aceptada(), "sin cargo, no");
    e.ok("mara", Comando::EntrarEnPueblo { pueblo: aldea, heroe: mara });
    assert!(!e.real("mara", mejorar.clone()).aceptada(), "dentro y sin cargo, tampoco");
    e.ok("mara", Comando::SalirDePueblo { pueblo: aldea, heroe: mara });
    e.ok("admin", Comando::AsignarCargoLocal { heroe: mara, asentamiento: plaza, cargo: asentamiento::Cargo::MaestroObras, designado: mara });
    assert!(!e.real("mara", mejorar.clone()).aceptada(), "con cargo pero fuera de la plaza y de la aldea, no");
    e.ok("mara", Comando::EntrarEnPueblo { pueblo: aldea, heroe: mara });
    let r = e.real("mara", mejorar);
    assert!(r.aceptada(), "{r:?}");
    assert!(matches!(e.pueblo(aldea).mejora, Some(EstadoPueblo::Pendiente { .. })));
    // En la aldea se deja trigo (o ganado), no lo que no produce.
    let col = e.n.heroe(mara).unwrap().columna.unwrap();
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Trigo, cantidad: 10_000 });
    assert_eq!(e.rechazada("mara", Comando::DejarEnPueblo { heroe: mara, pueblo: aldea, recurso: Cobre, cantidad: 1_000 }), rechazos::PUEBLO_RECURSO_AJENO);
    e.ok("mara", Comando::DejarEnPueblo { heroe: mara, pueblo: aldea, recurso: Trigo, cantidad: 10_000 });
    // Con la mejora hecha, el cupo de guarnición de cada héroe sube 15.
    e.entregar_lo_que_falta(aldea);
    let Some(EstadoPueblo::EnObra { completa_en }) = e.pueblo(aldea).mejora.clone() else { panic!("la mejora, en obra") };
    e.n.avanzar_hasta(completa_en);
    assert_eq!(e.pueblo(aldea).nivel, 2);
    assert_eq!(e.n.cupo_de_guarnicion_de_pueblo(aldea), Some(45));
}
