//! Las misiones en el núcleo (mecánica 49, `Docs/Mechanics/Misiones.md`): las cadenas del héroe con sus avances en origen, los contadores de estado de la plaza, reclamar con sus
//! rechazos y la foto. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

use asentamiento::catalogo::Tipo;
use asentamiento::Recurso;
use base::{Punto, MS_POR_MINUTO, UNIDAD};
use militar::catalogo::Tropa;
use militar::columna::Politica;
use militar::heroe::Controlador;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::misiones::{ClaseDueno, Dueno};
use nucleo::{rechazos, Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Respuesta, Resultado};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};

const HORA: i64 = 60 * MS_POR_MINUTO;

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

fn nucleo(m: &M) -> Nucleo {
    Nucleo::nuevo(Arranque { reglas: Reglas::default(), ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) })
}

fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

/// Un sitio verde cerca del campamento `i` donde fundar.
fn sitio(n: &Nucleo, m: &Mundo, i: usize, faccion: u32) -> Punto {
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
    panic!("sin sitio cerca del campamento {i}");
}

struct X {
    n: Nucleo,
    cuenta: u64,
}

impl X {
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
    fn rechazo(&mut self, actor: &str, c: Comando) -> &'static str {
        match self.ord(actor, c).resultado {
            Resultado::Rechazada { codigo, .. } => codigo,
            otro => panic!("se esperaba un rechazo: {otro:?}"),
        }
    }
    fn avanzar(&mut self, ms: i64) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    fn cumplidas(&self, d: Dueno) -> Vec<String> {
        let r = &self.n.estado().reglas.economia.misiones;
        self.n.misiones_de(d).map(|x| x.cumplidas.iter().map(|&i| r.misiones[i as usize].id.clone()).collect()).unwrap_or_default()
    }
    fn hechos(&self, codigo: &str) -> Vec<String> {
        self.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).filter_map(|e| e.valores.iter().find(|(k, _)| *k == "mision").map(|(_, v)| format!("{v:?}"))).collect()
    }
    fn oro_personal(&self, h: u64) -> i64 {
        self.n.heroe(h).unwrap().almacen_personal.get(&Recurso::Oro).copied().unwrap_or(0)
    }
}

fn reclamar(heroe: u64, dueno: Dueno, mision: &str) -> Comando {
    Comando::ReclamarRecompensa { heroe, dueno, mision: mision.into() }
}

#[test]
fn el_heroe_cumple_su_cadena_en_origen_y_reclama_el_oro_una_vez() {
    let m = con_mundo!();
    let mut x = X { n: nucleo(m), cuenta: 0 };
    let h = x.ok("ana", Comando::CrearHeroe { cuenta: "ana".into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0];
    let d = Dueno::Heroe(h);
    // El préstamo antes de salir no cuenta: H2 aún no está activa (pide H1).
    let escuadras = x.ok("ana", Comando::PedirPrestamo { heroe: h, tropas: vec![Tropa::Granjeros] }).creadas;
    assert!(x.cumplidas(d).is_empty());
    assert_eq!(x.n.misiones_de(d), None, "sin misión activa que lo use, el préstamo no deja contador");
    let col = x.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: h, escuadras, carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None }).creadas[0];
    assert_eq!(x.cumplidas(d), ["H1"], "salir cumple H1");
    // Marchar y llegar cumple H3.
    let c0 = campamento(&m.0, 0);
    let llega = (1..40).flat_map(|k| (0..8).map(move |g| (k, g))).any(|(k, g)| {
        let a = (g as f64) * std::f64::consts::FRAC_PI_4;
        let destino = Punto::new(c0.x + (k as f64 * 5_000.0 * a.cos()) as i64, c0.y + (k as f64 * 5_000.0 * a.sin()) as i64);
        x.ord("ana", Comando::MarcharA { ejercito: col, destino }).aceptada()
    });
    assert!(llega, "hay algún destino por tierra cerca del campamento");
    let fin = x.n.ejercito(col).unwrap().plan.fin().unwrap();
    x.n.avanzar_hasta(fin);
    assert_eq!(x.cumplidas(d), ["H1", "H3"]);
    assert_eq!(x.hechos("mision.cumplida").len(), 2);
    // Reclamar: solo su cuenta, una vez y lo cumplido.
    assert_eq!(x.rechazo("beto", reclamar(h, d, "H1")), rechazos::NO_AUTORIZADO);
    assert_eq!(x.rechazo("ana", reclamar(h, d, "H2")), rechazos::MISION_NO_CUMPLIDA);
    assert_eq!(x.rechazo("ana", reclamar(h, d, "A2")), rechazos::MISION_DESCONOCIDA, "A2 es de una plaza, no de un héroe");
    let antes = x.oro_personal(h);
    x.ok("ana", reclamar(h, d, "H1"));
    assert_eq!(x.oro_personal(h), antes + 10 * UNIDAD, "10 de oro al almacén personal");
    assert_eq!(x.rechazo("ana", reclamar(h, d, "H1")), rechazos::MISION_YA_RECLAMADA);
    assert_eq!(x.hechos("mision.reclamada").len(), 1);
    // La proyección (Misiones §5): las activas con su avance, las cumplidas y las reclamadas, en el orden del catálogo; las que esperan a su previa no viajan.
    use contrato::v1 as pb;
    let vistas = nucleo::proyeccion::propios::misiones(&x.n, d);
    let estados: Vec<(&str, i32)> = vistas.iter().map(|v| (v.id.as_str(), v.estado)).collect();
    let (activa, cumplida, reclamada) = (pb::EstadoMision::MisionActiva as i32, pb::EstadoMision::MisionCumplida as i32, pb::EstadoMision::MisionReclamada as i32);
    assert_eq!(estados, [("H1", reclamada), ("H2", activa), ("H3", cumplida), ("H4", activa), ("H5", activa), ("H11", activa), ("H13", activa)]);
    let h3 = &vistas[2];
    assert_eq!((h3.condiciones[0].contador.as_str(), h3.condiciones[0].avance, h3.condiciones[0].umbral), ("llegadas", 1, 1));
    assert_eq!(h3.previa.as_deref(), Some("H1"));
    assert_eq!(h3.recompensa.as_ref().unwrap().por_id.get("oro"), Some(&(10 * UNIDAD)));
    assert_eq!(vistas[1].condiciones[0].avance, 0, "el préstamo de antes no cuenta para H2");
    // La foto lleva las misiones (desde BAN46) y se lee igual.
    let bytes = x.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN48");
    let vuelta = nucleo::Estado::deserializar(&bytes, &Reglas::default(), &x.n.estado().mundo_hash).unwrap();
    assert_eq!(vuelta.misiones, x.n.estado().misiones);
    assert_eq!(vuelta.serializar(), bytes);
}

#[test]
fn la_plaza_cumple_por_estado_al_completar_el_edificio_y_la_cadena_sigue_en_el_mismo_instante() {
    let m = con_mundo!();
    let mut n = nucleo(m);
    let p = sitio(&n, &m.0, 0, 1);
    let e = n.crear_ejercito(0, "ana", 1, p, 500, 100_000).unwrap();
    assert!(n.aplicar_en(0, Peticion::nueva("admin", "c", Comando::DarCaravanaFundacion { ejercito: e }).de_admin()).aceptada());
    let a = n.aplicar_en(0, Peticion::nueva("ana", "f", Comando::Fundar { ejercito: e })).creadas[0];
    let mut x = X { n, cuenta: 0 };
    let d = Dueno::Asentamiento(a);
    // Sin construcción automática, la cola queda libre para lo que pide la prueba.
    x.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
    // Nivel 2 antes de tener Mercado: A4 aún no está activa, así que no se cumple.
    x.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel: 2 });
    x.ok("admin", Comando::DarRecursos { asentamiento: a, recurso: Recurso::Madera, cantidad: 100_000 });
    assert!(x.cumplidas(d).is_empty());
    x.ok("admin", Comando::AnadirEdificio { asentamiento: a, tipo: Tipo::Mercado });
    let activo = |x: &X| x.n.economia(a).unwrap().edificios.iter().any(|e| e.tipo == Tipo::Mercado && e.activo());
    for _ in 0..80 {
        if activo(&x) {
            break;
        }
        x.avanzar(HORA);
    }
    assert!(activo(&x), "el Mercado acaba por levantarse");
    let cumplidas = x.cumplidas(d);
    assert!(cumplidas.contains(&"A2".to_owned()) && cumplidas.contains(&"A4".to_owned()), "A2 por el Mercado y, al activarse, A4 por el nivel que ya tenía: {cumplidas:?}");
    // Sin Gobernador ni residentes, solo la administración la reclama. Los recursos van al almacén de la plaza si caben enteros: con el almacén casi lleno, no caben y siguen por
    // reclamar; con un Almacén más, sí.
    let almacen = |x: &X, r: Recurso| x.n.economia(a).unwrap().almacen.cantidad(r);
    x.ok("admin", Comando::DarRecursos { asentamiento: a, recurso: Recurso::Piedra, cantidad: 100_000 });
    assert!(x.n.economia(a).unwrap().almacen.espacio(Recurso::Piedra) < 300 * UNIDAD);
    assert_eq!(x.rechazo("admin", reclamar(0, d, "A4")), rechazos::MISION_ALMACEN_LLENO);
    x.ok("admin", Comando::DarRecursos { asentamiento: a, recurso: Recurso::Madera, cantidad: 50_000 });
    x.ok("admin", Comando::AnadirEdificio { asentamiento: a, tipo: Tipo::Almacen });
    for _ in 0..20 {
        if x.n.economia(a).unwrap().almacen.espacio(Recurso::Piedra) >= 300 * UNIDAD {
            break;
        }
        x.avanzar(HORA);
    }
    let piedra = almacen(&x, Recurso::Piedra);
    assert!(x.n.economia(a).unwrap().almacen.espacio(Recurso::Piedra) >= 300 * UNIDAD, "el Almacén nuevo deja sitio");
    x.ok("admin", reclamar(0, d, "A4"));
    assert_eq!(almacen(&x, Recurso::Piedra), piedra + 300 * UNIDAD);
    assert_eq!(x.rechazo("admin", reclamar(0, d, "A4")), rechazos::MISION_YA_RECLAMADA);
    // El interior de la plaza las lleva (Misiones §5) y la Facción pública no lleva las suyas.
    let interior = nucleo::proyeccion::interior::asentamiento(&x.n, a).unwrap();
    let a4 = interior.misiones.iter().find(|v| v.id == "A4").unwrap();
    assert_eq!(a4.estado, contrato::v1::EstadoMision::MisionReclamada as i32);
    assert_eq!(a4.recompensa.as_ref().unwrap().por_id.get("piedra"), Some(&(300 * UNIDAD)));
    let a1 = interior.misiones.iter().find(|v| v.id == "A1").expect("la A1 viaja");
    assert_eq!(a1.estado, contrato::v1::EstadoMision::MisionActiva as i32, "activa mientras la plaza no tenga aldea agrícola");
    assert!(nucleo::proyeccion::propios::faccion_publica(&x.n, x.n.estado().asentamientos[&a].faccion).misiones.is_empty());
    // Una foto anterior a las misiones arranca la plaza mirando su estado, sin hecho.
    let r = &x.n.estado().reglas.economia.misiones;
    assert!(r.buscar(ClaseDueno::Asentamiento, "A1").is_some());
}

#[test]
fn reclamar_lo_de_una_plaza_es_de_su_gobernador_y_lo_de_una_faccion_de_su_rey() {
    let m = con_mundo!();
    let mut x = X { n: nucleo(m), cuenta: 0 };
    let h = x.ok("ana", Comando::CrearHeroe { cuenta: "ana".into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0];
    x.ok("ana", Comando::CrearFaccion { heroe: h, nombre: "Micenas".into(), sigilo: None });
    // Crear la Facción hace a su creador ciudadano: avanza la cadena del héroe (H5 pide H1, que aún no está).
    assert!(!x.cumplidas(Dueno::Heroe(h)).contains(&"H5".to_owned()));
    let faccion = x.n.heroe(h).unwrap().faccion.expect("es ciudadano de la que crea");
    assert_eq!(x.rechazo("beto", reclamar(h, Dueno::Faccion(faccion), "F1")), rechazos::NO_AUTORIZADO, "beto no es la cuenta del héroe");
    assert_eq!(x.rechazo("ana", reclamar(h, Dueno::Faccion(faccion), "F1")), rechazos::MISION_NO_CUMPLIDA);
    // La Facción propia las lleva: las primeras de cada cadena, activas y a 0.
    let ids: Vec<String> = nucleo::proyeccion::propios::faccion(&x.n, faccion).misiones.into_iter().map(|v| v.id).collect();
    assert_eq!(ids, ["F1", "F7", "F10"]);
}
