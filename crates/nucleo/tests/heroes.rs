//! Los héroes y su tropa en el núcleo (fase 4B-1, Heroe_Y_Tropa, Campamentos_Mercenarios, Presencia_Y_Convocatoria §7): crear al héroe, reclutar
//! en una plaza y en un campamento, la tropa prestada, loadouts, guarnición con su ración, escolta de caravanas, herido, conectarse y
//! desconectarse, la ruina de la plaza donde se reside, y determinismo y recuperación desde foto. Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

use asentamiento::Recurso::{self, *};
use base::{Instante, Punto, MS_POR_MINUTO};
use militar::catalogo::Tropa;
use militar::escuadra::Contenedor;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{rechazos, Ajustes, Arranque, Comando, Nucleo, PagarCon, Peticion, Reglas, Resultado, Respuesta};
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

/// Eventos en la cola salvo las citas de fin de obra, mejora, ascenso y política de las plazas, que nacen y se cumplen solas mientras el mundo avanza.
fn en_cola(n: &Nucleo) -> usize {
    n.estado().cola.volcar().iter().filter(|e| !matches!(e.carga, nucleo::Carga::FinDeObras | nucleo::Carga::CaducaPolitica)).count()
}

fn hechos(n: &Nucleo, codigo: &str) -> Vec<Hecho> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).map(|e| (e.en, e.asentamientos.clone(), e.datos.clone())).collect()
}

fn dato(h: &Hecho, clave: &str) -> i64 {
    h.2.iter().find(|(k, _)| *k == clave).unwrap_or_else(|| panic!("el hecho no lleva «{clave}»: {h:?}")).1
}

struct Escenario {
    n: Nucleo,
    /// La plaza de «ana» (Facción 1).
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
    /// El héroe de la cuenta, nacido en el campamento `campamento`.
    fn heroe(&mut self, cuenta: &str, campamento: u32) -> u64 {
        self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento, controlador: Controlador::Humano }).creadas[0]
    }
    /// Un héroe de la cuenta ya ciudadano de la Facción 1, residente y presente en la plaza.
    fn heroe_en_la_plaza(&mut self, cuenta: &str) -> u64 {
        let h = self.heroe(cuenta, 0);
        let a = self.a as u32;
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(a)) });
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(a) });
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

fn escenario(m: &M, semilla: u64) -> Escenario {
    let mut n = Nucleo::nuevo(arranque(m, semilla));
    let p = sitio_rico(&n, &m.0, 0, 1);
    let a = fundar(&mut n, 0, "a", "ana", 1, p);
    Escenario { n, a, cuenta: 0 }
}

// ───────────────────────────── 1. Crear al héroe ─────────────────────────────

#[test]
fn el_heroe_nace_en_un_campamento_con_su_progresion_inicial_y_cada_rechazo_tiene_su_codigo() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let crear = |cuenta: &str, campamento, controlador| Comando::CrearHeroe { cuenta: cuenta.into(), campamento, controlador };
    // Pasada la aparición de los campamentos de bandidos de las plazas, nada más se mueve por sí solo.
    e.avanzar(HORA);
    let (v0, id0) = (e.n.version(), e.n.estado().siguiente_id);
    assert_eq!(e.rechazada("ana", crear("beto", 0, Controlador::Humano)), rechazos::NO_AUTORIZADO, "nadie crea el héroe de otro");
    assert_eq!(e.rechazada("ana", crear("ana", 0, Controlador::Bot)), rechazos::NO_AUTORIZADO, "el controlador «bot» solo lo pone la administración");
    assert_eq!(e.rechazada("ana", crear("ana", 9_999, Controlador::Humano)), rechazos::HEROE_CAMPAMENTO_DESCONOCIDO);
    assert_eq!((e.n.version(), e.n.estado().siguiente_id), (v0, id0), "un rechazo no deja rastro: ni versión ni ids");

    let h = e.heroe("ana", 2);
    let hn = e.n.heroe(h).unwrap();
    assert_eq!((hn.cuenta.as_str(), hn.faccion, hn.heroe.controlador), ("ana", None, Controlador::Humano));
    assert_eq!((hn.heroe.nivel, hn.heroe.monedas.bronce, hn.heroe.liderazgo_base), (1, 500, 100));
    assert_eq!(hn.heroe.residencia, Some(Residencia::Campamento(2)), "nace dentro de un campamento, donde reside");
    assert_eq!(hn.heroe.presencia.ubicacion, Ubicacion::Mercenarios(2));
    assert!(hn.heroe.escuadras.is_empty() && hn.almacen_personal.is_empty());
    assert_eq!(hn.heroe.loadouts.len(), 1);
    assert_eq!((hn.heroe.loadouts[0].nombre.as_str(), hn.heroe.loadouts[0].activo, hn.heroe.loadouts[0].id), ("Default", true, h + 1), "el id del héroe y el de su loadout salen del contador común");
    assert_eq!(e.n.estado().siguiente_id, id0 + 2);
    assert_eq!(e.n.heroe_de_cuenta("ana").map(|x| x.heroe.id), Some(h));
    assert!(!e.n.heroe_herido(h));
    assert_eq!(e.rechazada("ana", crear("ana", 0, Controlador::Humano)), rechazos::HEROE_YA_EXISTE, "uno por cuenta y partida");

    // La administración crea los bots, y la cuenta ajena.
    let bot = e.ok("admin", crear("bot-1", 1, Controlador::Bot)).creadas[0];
    assert_eq!(e.n.heroe(bot).unwrap().heroe.controlador, Controlador::Bot);
    let creados = hechos(&e.n, "heroe.creado");
    assert_eq!((creados.len(), dato(&creados[0], "heroe"), dato(&creados[0], "campamento")), (2, h as i64, 2));
    // El héroe no tiene columna ni plaza: no hay nada que mover en el mapa.
    assert_eq!(e.n.estado().ejercito_de_heroe(h), None);
}

// ───────────────────────────── 2. Reclutar en una plaza ─────────────────────────────

#[test]
fn reclutar_en_una_plaza_valida_en_orden_paga_del_almacen_y_resta_poblacion() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let (a, h) = (e.a, e.heroe("ana", 0));
    let reclutar = |heroe, tropa| Comando::ReclutarTropa { asentamiento: a, heroe, tropa };
    assert_eq!(e.rechazada("beto", reclutar(h, Tropa::MiliciaLanceros)), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", reclutar(99, Tropa::MiliciaLanceros)), rechazos::HEROE_DESCONOCIDO);
    assert_eq!(e.rechazada("ana", Comando::ReclutarTropa { asentamiento: 9_999, heroe: h, tropa: Tropa::MiliciaLanceros }), rechazos::ASENTAMIENTO_DESCONOCIDO);
    assert_eq!(e.rechazada("ana", reclutar(h, Tropa::MiliciaLanceros)), rechazos::RECLUTAMIENTO_NO_PUEDES, "ni reside ni es de la Facción de la plaza");
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
    assert_eq!(e.rechazada("ana", reclutar(h, Tropa::MiliciaLanceros)), rechazos::HEROE_NO_ESTA_EN_LA_PLAZA, "hay que estar dentro de la plaza");
    e.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(a as u32) });
    assert_eq!(e.rechazada("ana", reclutar(h, Tropa::MiliciaLanceros)), rechazos::RECLUTAMIENTO_SOLO_REPONER, "ciudadano sin residir: solo repone lo que ya tiene aquí");
    e.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(a as u32)) });

    // La tecnología se mira en la Facción de la plaza (antes que la población, el edificio o el coste).
    assert_eq!(e.rechazada("ana", reclutar(h, Tropa::LancerosMimbre)), rechazos::RECLUTAMIENTO_SIN_TECNOLOGIA);
    e.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["escudos_ligeros".into()] });
    assert_eq!(e.rechazada("ana", reclutar(h, Tropa::LancerosMimbre)), rechazos::RECLUTAMIENTO_SIN_EDIFICIO, "la plaza no tiene Barracón");

    // Milicia del Centro Urbano: 25 pesants y 2 de madera por soldado; la leva de arranque no pide tecnología.
    let (madera0, pesants0, ids0, cola0) = (e.almacen(Madera), e.n.economia(a).unwrap().poblacion.pesants, e.n.estado().siguiente_id, en_cola(&e.n));
    let r = e.ok("ana", reclutar(h, Tropa::MiliciaLanceros));
    assert_eq!(r.creadas.len(), 1);
    let esc = e.n.heroe(h).unwrap().heroe.escuadra(r.creadas[0]).unwrap().clone();
    assert_eq!((esc.tropa, esc.cantidad, esc.nivel, esc.contenedor, esc.en_guarnicion, esc.plaza_origen), (Tropa::MiliciaLanceros, 25, 1, Contenedor::Campamento, false, Some(a as u32)));
    assert_eq!(esc.moral_en(e.n.ahora()), 100_000);
    assert_eq!(madera0 - e.almacen(Madera), 50_000, "2 de madera por soldado");
    assert_eq!(pesants0 - e.n.economia(a).unwrap().poblacion.pesants, 25, "los soldados salen de la población de su clase");
    assert_eq!(e.n.estado().siguiente_id, ids0 + 1, "una escuadra nueva gasta un id");
    assert_eq!(en_cola(&e.n), cola0, "reclutar no programa ningún evento");
    let h1 = hechos(&e.n, "tropas.reclutadas");
    assert_eq!((h1.len(), h1[0].1.clone(), dato(&h1[0], "reclutados"), dato(&h1[0], "nueva")), (1, vec![a], 25, 1));
    assert_eq!(e.n.soldados_del_campamento(a), 25);
    assert_eq!(e.rechazada("ana", reclutar(h, Tropa::MiliciaLanceros)), rechazos::RECLUTAMIENTO_AL_TOPE, "una escuadra por tropa: ya está completa");
    assert_eq!(e.n.estado().siguiente_id, ids0 + 1, "un rechazo no gasta ids");

    // «Leva Forzosa» (General): el equipo cuesta el 70 %.
    e.ok("admin", Comando::ActivarPolitica { asentamiento: a, politica: "leva_forzosa".into(), cargo: asentamiento::Cargo::General });
    let madera1 = e.almacen(Madera);
    e.ok("ana", reclutar(h, Tropa::Lenadores));
    assert_eq!(madera1 - e.almacen(Madera), 21_000, "30 soldados × 1 de madera × 0,7");
}

// ───────────────────────────── 3. Reclutar en un campamento de mercenarios ─────────────────────────────

/// La huella del campamento (Generacion_Del_Mundo §11) y, con ella, su edificio militar: 0 Barracón, 1 Galería de tiro, 2 Caballerizas.
fn edificio_militar(m: &Mundo, c: usize) -> i64 {
    let p = campamento(m, c);
    (p.x.div_euclid(1_000) * 31 + p.y.div_euclid(1_000) * 17 + c as i64).abs() % 3
}

#[test]
fn el_edificio_militar_de_un_campamento_sale_de_su_posicion_y_con_el_la_tropa_que_ofrece() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let n = m.0.campamentos().len();
    // Con las tecnologías de arranque solo se ofrecen los Honderos y los Escaramuzadores de la Galería de tiro.
    let ofrecen = |e: &Escenario, tropa| (0..n).filter(|&c| e.n.tropas_del_campamento(c as u32).unwrap().contains(&tropa)).collect::<Vec<_>>();
    let galerias: Vec<usize> = (0..n).filter(|&c| edificio_militar(&m.0, c) == 1).collect();
    assert!(!galerias.is_empty() && galerias.len() < n);
    assert_eq!(ofrecen(&e, Tropa::Honderos), galerias);
    assert!((0..n).all(|c| !e.n.tropas_del_campamento(c as u32).unwrap().contains(&Tropa::MiliciaLanceros)), "la leva de arranque no se recluta en un campamento");
    assert!(e.n.tropas_del_campamento(n as u32).is_none());
    // Una tecnología que adopta la mitad de las Facciones vivas (aquí la única) la desbloquea en todos los campamentos que tienen su edificio.
    assert!(ofrecen(&e, Tropa::LancerosMimbre).is_empty());
    e.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["escudos_ligeros".into()] });
    let barracones: Vec<usize> = (0..n).filter(|&c| edificio_militar(&m.0, c) == 0).collect();
    assert_eq!(ofrecen(&e, Tropa::LancerosMimbre), barracones);
    assert_eq!(e.rechazada("ana", Comando::FijarTecnologias { faccion: 1, tecnologias: vec![] }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["no_existe".into()] }), rechazos::TECNOLOGIA_DESCONOCIDA);
    assert_eq!(e.ord("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["escudos_ligeros".into()] }).resultado, Resultado::SinCambios);
    assert!(e.n.tiene_tecnologia(1, "escudos_ligeros") && e.n.tiene_tecnologia(2, "leva_comunal") && !e.n.tiene_tecnologia(2, "escudos_ligeros"));
}

#[test]
fn reclutar_en_un_campamento_cobra_oro_del_almacen_personal_o_del_carro_y_gasta_reclutas() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let n = m.0.campamentos().len();
    let camp = (0..n).find(|&c| edificio_militar(&m.0, c) == 1).unwrap() as u32;
    let mr = militar::Reglas::cargar();
    let (h, beto) = (e.heroe("ana", camp), e.heroe("beto", camp));
    let honderos = |heroe, pagar_con| Comando::ReclutarEnCampamento { heroe, tropa: Tropa::Honderos, pagar_con };
    let personal = PagarCon::AlmacenPersonal;
    assert_eq!(e.rechazada("beto", honderos(h, personal)), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", Comando::ReclutarEnCampamento { heroe: h, tropa: Tropa::MiliciaLanceros, pagar_con: personal }), rechazos::MERCENARIOS_TROPA_NO_DISPONIBLE);
    assert_eq!(e.rechazada("ana", honderos(h, personal)), rechazos::MERCENARIOS_SIN_ORO);
    assert_eq!(e.rechazada("ana", honderos(h, PagarCon::Carro)), rechazos::HEROE_SIN_COLUMNA, "para pagar con el carro hay que ir en una columna");
    assert_eq!(e.n.reclutas_de_campamento(camp), Some(100_000), "un campamento nace lleno: 10 viviendas × 10");

    // Con el oro en el almacén personal: precio = (2 de escalón + 1 de arma de madera × 8) × 1,5 = 15 por soldado, 20 soldados.
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: h, recurso: Oro, cantidad: 1_000_000 });
    let precio = militar::mercenarios::precio(&mr, mr.tropas.tropa(Tropa::Honderos), 20, asentamiento::datos::UNO, false);
    assert_eq!(precio, 300_000);
    let r = e.ok("ana", honderos(h, personal));
    assert_eq!(r.creadas.len(), 1);
    let hn = e.n.heroe(h).unwrap();
    let esc = hn.heroe.escuadra(r.creadas[0]).unwrap();
    assert_eq!((esc.tropa, esc.cantidad, esc.contenedor, esc.plaza_origen, esc.prestada), (Tropa::Honderos, 20, Contenedor::Campamento, None, None));
    assert_eq!(hn.almacen_personal[&Oro], 1_000_000 - precio);
    assert_eq!(e.n.reclutas_de_campamento(camp), Some(80_000));
    let reclutado = hechos(&e.n, "mercenarios.reclutado");
    assert_eq!((reclutado.len(), dato(&reclutado[0], "cantidad"), dato(&reclutado[0], "oro")), (1, 20, precio));
    assert_eq!(e.rechazada("ana", honderos(h, personal)), rechazos::MERCENARIOS_AL_TOPE);
    // Los reclutas se recuperan a 10 por hora, sin ningún evento.
    let cola = en_cola(&e.n);
    e.avanzar(HORA);
    assert_eq!(e.n.reclutas_de_campamento(camp), Some(90_000));
    assert!(en_cola(&e.n) <= cola, "reponer los reclutas no programa nada");

    // Con el oro del carro de su columna, a la puerta del campamento.
    let puerta = campamento(&m.0, camp as usize);
    let t = e.n.ahora();
    let col = e.n.crear_ejercito(t, "beto", 2, Punto::new(puerta.x + 1_000, puerta.y), 500, 100_000).unwrap();
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 400_000 });
    e.ok("admin", Comando::ColocarHeroe { heroe: beto, ubicacion: Ubicacion::Columna(col) });
    e.ok("beto", honderos(beto, PagarCon::Carro));
    assert_eq!(e.n.suministro(col)[&Oro], 400_000 - precio);
    assert_eq!(e.n.reclutas_de_campamento(camp), Some(70_000));
    assert_eq!(e.n.soldados_del_campamento(e.a), 0, "los reclutas de un campamento no comen de ninguna plaza");

    // Lejos del campamento no se recluta, y solo donde se reside.
    e.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(e.a as u32) });
    assert_eq!(e.rechazada("ana", Comando::ReclutarEnCampamento { heroe: h, tropa: Tropa::Honderos, pagar_con: personal }), rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA);
    e.ok("admin", Comando::FijarResidencia { heroe: h, residencia: None });
    assert_eq!(e.rechazada("ana", honderos(h, personal)), rechazos::MERCENARIOS_NO_RESIDES);
}

// ───────────────────────────── 4. La tropa prestada ─────────────────────────────

#[test]
fn la_tropa_prestada_es_gratis_de_leva_comunal_y_se_retira_al_dejar_de_residir() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let h = e.heroe("ana", 0);
    let pedir = |tropas: &[Tropa]| Comando::PedirPrestamo { heroe: h, tropas: tropas.to_vec() };
    assert_eq!(e.rechazada("ana", pedir(&[])), rechazos::MERCENARIOS_PRESTAMO_SIN_TROPAS);
    assert_eq!(e.rechazada("ana", pedir(&[Tropa::Honderos])), rechazos::MERCENARIOS_NO_SE_PRESTA, "solo la leva comunal se presta");
    assert_eq!(e.rechazada("ana", pedir(&[Tropa::Granjeros, Tropa::Granjeros])), rechazos::MERCENARIOS_PRESTAMO_REPETIDA);
    assert!(e.n.heroe(h).unwrap().heroe.escuadras.is_empty(), "un rechazo no crea nada");
    let ids0 = e.n.estado().siguiente_id;
    let r = e.ok("ana", pedir(&[Tropa::Granjeros, Tropa::Mineros]));
    assert_eq!(r.creadas.len(), 2);
    assert_eq!(e.n.estado().siguiente_id, ids0 + 2);
    for (id, tropa) in r.creadas.iter().zip([Tropa::Granjeros, Tropa::Mineros]) {
        let esc = e.n.heroe(h).unwrap().heroe.escuadra(*id).unwrap();
        assert_eq!((esc.tropa, esc.cantidad, esc.prestada, esc.contenedor, esc.experiencia), (tropa, 30, Some(0), Contenedor::Campamento, 0));
    }
    assert_eq!(e.rechazada("ana", pedir(&[Tropa::Lenadores, Tropa::Granjeros])), rechazos::MERCENARIOS_YA_PRESTADA);
    assert_eq!(e.rechazada("ana", Comando::ReponerPrestamo { heroe: h }), rechazos::MERCENARIOS_NADA_QUE_REPONER, "ya están completas");
    // Una prestada no cuenta para «una escuadra por tropa»: reclutar la propia de esa tropa no choca con ella.
    assert!(e.n.heroe(h).unwrap().heroe.escuadra_de(Tropa::Granjeros).is_none());
    // Fuera del campamento no se pide, y la tropa prestada solo es del campamento donde se reside.
    e.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(e.a as u32) });
    assert_eq!(e.rechazada("ana", pedir(&[Tropa::Lenadores])), rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA);
    e.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Mercenarios(0) });
    // Mudarse de campamento la retira, de donde esté, y lo cuenta.
    e.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Campamento(1)) });
    assert!(e.n.heroe(h).unwrap().heroe.escuadras.is_empty());
    let retiro = hechos(&e.n, "mercenarios.prestamo_retirado");
    assert_eq!((retiro.len(), dato(&retiro[0], "escuadras")), (1, 2));
    assert_eq!(hechos(&e.n, "mercenarios.prestamo").len(), 1);
}

// ───────────────────────────── 5. Loadouts, orden de las escuadras y puntos ─────────────────────────────

#[test]
fn los_loadouts_el_orden_de_las_escuadras_y_el_reparto_de_puntos_validan_cada_uno_lo_suyo() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let h = e.heroe("ana", 0);
    let ids = e.ok("ana", Comando::PedirPrestamo { heroe: h, tropas: vec![Tropa::Granjeros, Tropa::Lenadores, Tropa::MiliciaLanceros, Tropa::Mineros] }).creadas;
    let (s1, s2, s3, s4) = (ids[0], ids[1], ids[2], ids[3]);
    let guardar = |loadout, nombre: &str, escuadras: &[u64], activo| Comando::GuardarLoadout { heroe: h, loadout, nombre: nombre.into(), escuadras: escuadras.to_vec(), perks: vec![], activo };
    assert_eq!(e.rechazada("ana", guardar(None, "  ", &[s1], None)), rechazos::LOADOUT_SIN_NOMBRE);
    assert_eq!(e.rechazada("ana", guardar(Some(55), "x", &[s1], None)), rechazos::LOADOUT_DESCONOCIDO);
    assert_eq!(e.rechazada("ana", guardar(None, "x", &[s1, s1], None)), rechazos::ESCUADRA_REPETIDA);
    assert_eq!(e.rechazada("ana", guardar(None, "x", &[s1, 99_999], None)), rechazos::ESCUADRA_AJENA);
    assert_eq!(e.rechazada("ana", Comando::GuardarLoadout { heroe: h, loadout: None, nombre: "x".into(), escuadras: vec![s1], perks: vec![7], activo: None }), rechazos::LOADOUT_PERK_NO_DESBLOQUEADO);
    assert_eq!(e.n.heroe(h).unwrap().heroe.loadouts.len(), 1, "los rechazos no dejan rastro");

    // Uno nuevo se añade al final, inactivo, y su id sale del contador común.
    let ids0 = e.n.estado().siguiente_id;
    let asalto = e.ok("ana", guardar(None, "Asalto", &[s1, s2], None)).creadas[0];
    assert_eq!(asalto, ids0);
    let lo = &e.n.heroe(h).unwrap().heroe.loadouts;
    assert_eq!((lo.len(), lo[1].id, lo[1].nombre.as_str(), lo[1].activo, lo[0].activo), (2, asalto, "Asalto", false, true));
    // Reescribirlo por id conserva el id; marcarlo activo desactiva los demás.
    let r = e.ok("ana", guardar(Some(asalto), "Asalto 2", &[s3], Some(true)));
    assert!(r.creadas.is_empty());
    let lo = &e.n.heroe(h).unwrap().heroe.loadouts;
    assert_eq!((lo.len(), lo[1].nombre.as_str(), lo[1].escuadras.clone(), lo[1].activo, lo[0].activo), (2, "Asalto 2", vec![s3], true, false));
    assert_eq!(e.n.heroe(h).unwrap().heroe.escuadras_del_loadout_activo(), &[s3]);
    e.ok("ana", Comando::BorrarLoadout { heroe: h, loadout: asalto });
    assert_eq!(e.rechazada("ana", Comando::BorrarLoadout { heroe: h, loadout: asalto }), rechazos::LOADOUT_DESCONOCIDO);
    assert!(e.n.heroe(h).unwrap().heroe.escuadras_del_loadout_activo().is_empty(), "borrar el activo no activa otro");

    // El orden: las pedidas primero y el resto como estaban; repetir el mismo orden no cambia nada.
    let orden = |h: &nucleo::HeroeNucleo| h.heroe.escuadras.iter().map(|x| x.id).collect::<Vec<_>>();
    e.ok("ana", Comando::OrdenarEscuadras { heroe: h, escuadras: vec![s3, s1] });
    assert_eq!(orden(e.n.heroe(h).unwrap()), vec![s3, s1, s2, s4]);
    assert_eq!(e.ord("ana", Comando::OrdenarEscuadras { heroe: h, escuadras: vec![s3, s1] }).resultado, Resultado::SinCambios);
    assert_eq!(e.rechazada("ana", Comando::OrdenarEscuadras { heroe: h, escuadras: vec![s2, s2] }), rechazos::ESCUADRA_REPETIDA);
    assert_eq!(e.rechazada("ana", Comando::OrdenarEscuadras { heroe: h, escuadras: vec![s2, 99_999] }), rechazos::ESCUADRA_AJENA);
    assert_eq!(orden(e.n.heroe(h).unwrap()), vec![s3, s1, s2, s4]);

    // Los puntos: el servidor no sube el nivel, así que no hay puntos sin gastar que repartir.
    let reparto = |fuerza, destreza| Comando::RepartirPuntos { heroe: h, fuerza, destreza, armadura: 0, vitalidad: 0 };
    assert_eq!(e.rechazada("ana", reparto(0, 0)), rechazos::HEROE_NINGUN_PUNTO);
    assert_eq!(e.rechazada("ana", reparto(-1, 1)), rechazos::HEROE_PUNTOS_NEGATIVOS);
    assert_eq!(e.rechazada("ana", reparto(2, 0)), rechazos::HEROE_PUNTOS_INSUFICIENTES);
    assert_eq!(e.rechazada("beto", reparto(1, 0)), rechazos::NO_AUTORIZADO);
}

// ───────────────────────────── 6. Herido y presencia ─────────────────────────────

#[test]
fn un_heroe_herido_lo_esta_dos_minutos_y_se_lee_al_mirar_sin_ningun_evento() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let h = e.heroe("ana", 0);
    assert_eq!(e.rechazada("ana", Comando::HerirHeroe { heroe: h }), rechazos::NO_AUTORIZADO, "herir es del combate: hoy, de la administración");
    assert_eq!(e.rechazada("admin", Comando::HerirHeroe { heroe: 99 }), rechazos::HEROE_DESCONOCIDO);
    e.avanzar(10 * MIN);
    let t = e.n.ahora();
    let cola = en_cola(&e.n);
    e.ok("admin", Comando::HerirHeroe { heroe: h });
    assert_eq!(e.n.heroe(h).unwrap().heroe.herido_hasta, Some(t + 2 * MIN));
    assert!(e.n.heroe_herido(h));
    assert_eq!(en_cola(&e.n), cola, "curarse no programa nada: se mira al leer");
    assert_eq!(e.ord("admin", Comando::HerirHeroe { heroe: h }).resultado, Resultado::SinCambios, "a un herido no se le alarga");
    e.n.avanzar_hasta(t + 2 * MIN - 1);
    assert!(e.n.heroe_herido(h));
    e.n.avanzar_hasta(t + 2 * MIN);
    assert!(!e.n.heroe_herido(h), "vence en el instante exacto");
    e.ok("admin", Comando::HerirHeroe { heroe: h });
    assert_eq!(hechos(&e.n, "heroe.herido").len(), 2);
}

#[test]
fn desconectarse_programa_la_salida_a_dos_minutos_y_medio_y_conectarse_la_cancela_o_vuelve() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let h = e.heroe("ana", 0);
    let t = e.n.ahora();
    let cola = en_cola(&e.n);
    e.ok("ana", Comando::Desconectarse { heroe: h });
    assert_eq!(e.n.heroe(h).unwrap().heroe.presencia.desconecta_en, Some(t + 150_000));
    assert_eq!(en_cola(&e.n), cola + 1, "un solo evento, con prioridad 1");
    assert!(e.n.estado().cola.volcar().iter().any(|ev| matches!(ev.carga, nucleo::Carga::SalidaDelMundo) && ev.prioridad == nucleo::PRIO_SALIDA && ev.instante == t + 150_000));
    assert_eq!(e.ord("ana", Comando::Desconectarse { heroe: h }).resultado, Resultado::SinCambios, "pedirlo dos veces no cambia nada");
    // Conectarse antes de que venza la cancela, sin eventos, y el evento viejo no hace nada.
    e.ok("ana", Comando::Conectarse { heroe: h });
    assert_eq!(e.n.heroe(h).unwrap().heroe.presencia.desconecta_en, None);
    assert_eq!(e.ord("ana", Comando::Conectarse { heroe: h }).resultado, Resultado::SinCambios);
    e.avanzar(10 * MIN);
    assert!(!e.n.heroe(h).unwrap().heroe.presencia.fuera && hechos(&e.n, "heroe.sale_del_mundo").is_empty());

    // Si vence, sale: fuera del mundo solo se puede conectar.
    let t = e.n.ahora();
    e.ok("ana", Comando::Desconectarse { heroe: h });
    e.n.avanzar_hasta(t + 150_000 - 1);
    assert!(!e.n.heroe(h).unwrap().heroe.presencia.fuera);
    e.n.avanzar_hasta(t + 150_000);
    assert!(e.n.heroe(h).unwrap().heroe.presencia.fuera);
    let sale = hechos(&e.n, "heroe.sale_del_mundo");
    assert_eq!((sale.len(), sale[0].0), (1, t + 150_000));
    assert_eq!(e.rechazada("ana", Comando::PedirPrestamo { heroe: h, tropas: vec![Tropa::Granjeros] }), rechazos::HEROE_FUERA_DEL_MUNDO);
    assert_eq!(e.rechazada("ana", Comando::Desconectarse { heroe: h }), rechazos::HEROE_FUERA_DEL_MUNDO);
    assert_eq!(e.rechazada("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Mercenarios(1) }), rechazos::HEROE_FUERA_DEL_MUNDO);
    e.ok("ana", Comando::Conectarse { heroe: h });
    assert!(!e.n.heroe(h).unwrap().heroe.presencia.fuera);
    assert_eq!(hechos(&e.n, "heroe.vuelve_al_mundo").len(), 1);
    e.ok("ana", Comando::PedirPrestamo { heroe: h, tropas: vec![Tropa::Granjeros] });

    // En campo (andamiaje de administración: el resto de las columnas está en `columnas.rs`): una columna admite a varios héroes, y cada uno va en una.
    let puerta = campamento(&m.0, 0);
    let t = e.n.ahora();
    let col = e.n.crear_ejercito(t, "ana", 1, puerta, 500, 100_000).unwrap();
    assert_eq!(e.rechazada("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Desconectado(puerta) }), rechazos::HEROE_UBICACION_INVALIDA);
    e.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Columna(col) });
    assert_eq!(e.ord("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Columna(col) }).resultado, Resultado::SinCambios);
    assert_eq!(e.n.heroe(h).unwrap().columna, Some(col));
    let otra = e.n.crear_ejercito(t, "beto", 1, puerta, 500, 100_000).unwrap();
    assert_eq!(e.rechazada("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Columna(otra) }), rechazos::HEROE_YA_EN_COLUMNA);
    assert_eq!(e.rechazada("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Mercenarios(1) }), rechazos::HEROE_EN_COLUMNA, "de una columna se sale entrando en un lugar");
    let otro = e.heroe("beto", 1);
    e.ok("admin", Comando::ColocarHeroe { heroe: otro, ubicacion: Ubicacion::Columna(col) });
    assert_eq!(e.n.ejercito(col).unwrap().columna.participantes.len(), 3, "el héroe de campo de la administración y los dos");
    assert_eq!(e.rechazada("admin", Comando::ColocarHeroe { heroe: otro, ubicacion: Ubicacion::Columna(9_999) }), rechazos::EJERCITO_DESCONOCIDO);
    assert_eq!(e.rechazada("admin", Comando::ColocarHeroe { heroe: otro, ubicacion: Ubicacion::Asentamiento(9_999) }), rechazos::ASENTAMIENTO_DESCONOCIDO);
    assert_eq!(e.rechazada("admin", Comando::ColocarHeroe { heroe: otro, ubicacion: Ubicacion::Mercenarios(9_999) }), rechazos::CAMPAMENTO_DESCONOCIDO);
}

// ───────────────────────────── 7. Guarnición y ración ─────────────────────────────

/// Un escenario con la ración de un soldado fijada (en milésimas de trigo por minuto; el valor vigente es 150): solo se toca la cifra, no la huella.
fn escenario_con_racion(m: &M, semilla: u64, racion: i64) -> Escenario {
    let mut a = arranque(m, semilla);
    std::sync::Arc::make_mut(&mut a.reglas.economia.militar).militar.racion_por_soldado = racion;
    let mut n = Nucleo::nuevo(a);
    let p = sitio_rico(&n, &m.0, 0, 1);
    let a = fundar(&mut n, 0, "a", "ana", 1, p);
    Escenario { n, a, cuenta: 0 }
}

#[test]
fn la_guarnicion_respeta_el_cupo_de_sus_edificios_y_se_retira_sin_gastar_liderazgo() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let a = e.a;
    e.avanzar(24 * HORA);
    let h = e.heroe_en_la_plaza("ana");
    let milicia = e.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::MiliciaLanceros }).creadas[0];
    let lenadores = e.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::Lenadores }).creadas[0];
    let asignar = |escuadra| Comando::AsignarGuarnicion { heroe: h, escuadra };
    // Sin Barracón, Galería ni Caballerizas no hay guarnición.
    assert_eq!(e.n.cupo_de_guarnicion(a), Some(0));
    assert_eq!(e.rechazada("ana", asignar(lenadores)), rechazos::GUARNICION_SUPERA_EL_CUPO);
    // Un Barracón (la plaza sube al nivel 2 por la administración): cupo de 7 por héroe residente.
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel: 2 });
    e.dar(a, Madera, 1_000_000);
    e.ok("admin", Comando::AnadirEdificio { asentamiento: a, tipo: asentamiento::catalogo::Tipo::Barracon });
    // Espera su turno en la cola de obras y tarda 12 h en levantarse.
    let activo = |e: &Escenario| e.n.economia(a).unwrap().edificios.iter().any(|x| x.tipo == asentamiento::catalogo::Tipo::Barracon && x.activo());
    for _ in 0..80 {
        if activo(&e) {
            break;
        }
        e.avanzar(HORA);
    }
    assert!(activo(&e), "el Barracón acaba por levantarse");
    assert_eq!(e.n.cupo_de_guarnicion(a), Some(7), "Barracón de nivel interno 1");
    assert_eq!(e.rechazada("ana", asignar(milicia)), rechazos::GUARNICION_SUPERA_EL_CUPO, "una escuadra de escalón 2 cuesta 14 de Liderazgo");
    assert_eq!(e.rechazada("ana", asignar(99_999)), rechazos::ESCUADRA_AJENA);
    e.ok("ana", asignar(lenadores));
    assert!(e.n.escuadra(lenadores).unwrap().1.en_guarnicion);
    assert_eq!(e.rechazada("ana", asignar(lenadores)), rechazos::GUARNICION_YA_ASIGNADA);
    // «Levas de guarnición» (+14 al cupo de cada residente) mientras dure la política.
    e.ok("admin", Comando::ActivarPolitica { asentamiento: a, politica: "levas_guarnicion".into(), cargo: asentamiento::Cargo::General });
    assert_eq!(e.n.cupo_de_guarnicion(a), Some(21));
    e.ok("ana", asignar(milicia));
    // Retirar una escuadra la devuelve al campamento; una que no está en la guarnición no se retira.
    assert_eq!(e.rechazada("ana", Comando::RetirarGuarnicion { heroe: h, escuadra: 99_999 }), rechazos::GUARNICION_NO_ASIGNADA);
    e.ok("ana", Comando::RetirarGuarnicion { heroe: h, escuadra: milicia });
    assert_eq!(e.rechazada("ana", Comando::RetirarGuarnicion { heroe: h, escuadra: milicia }), rechazos::GUARNICION_NO_ASIGNADA);
    // Dejar de residir suelta la guarnición.
    e.ok("admin", Comando::FijarResidencia { heroe: h, residencia: None });
    assert!(e.n.heroe(h).unwrap().heroe.escuadras.iter().all(|x| !x.en_guarnicion));
    assert_eq!(e.n.soldados_del_campamento(a), 0, "sin residencia la tropa no come de la plaza");
}

#[test]
fn el_campamento_de_la_plaza_come_del_almacen_justo_despues_de_la_poblacion_y_sin_eventos_nuevos() {
    let m = con_mundo!();
    // Dos mundos idénticos salvo la ración (2 y 4 de trigo por soldado y minuto): lo que el segundo se come de más es exactamente la diferencia.
    let hasta = |racion| {
        let mut e = escenario_con_racion(m, 7, racion);
        e.avanzar(24 * HORA);
        e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: e.a, pausada: true });
        let h = e.heroe_en_la_plaza("ana");
        let a = e.a;
        e.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::MiliciaLanceros });
        e.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::Lenadores });
        e
    };
    let (mut poco, mut mucho) = (hasta(2_000), hasta(4_000));
    assert_eq!(poco.n.soldados_del_campamento(poco.a), 55);
    let (cola, antes) = (en_cola(&mucho.n), poco.almacen(Trigo));
    assert_eq!(antes, mucho.almacen(Trigo), "hasta reclutar son idénticos");
    for k in 1..=8 {
        poco.avanzar(MIN);
        mucho.avanzar(MIN);
        assert_eq!(poco.almacen(Trigo) - mucho.almacen(Trigo), k * 55 * 2_000, "la guarnición come 55 soldados × ración por minuto, ni más ni menos ({k} min)");
    }
    assert_eq!(en_cola(&mucho.n), cola, "comer no programa ningún evento: lo hace el paso de minuto que ya existe");
    assert_eq!(mucho.n.economia(mucho.a).unwrap().poblacion, poco.n.economia(poco.a).unwrap().poblacion, "la población come antes y no nota la ración");
}

#[test]
fn sin_trigo_baja_la_moral_y_con_la_moral_a_cero_desertan_los_soldados_hasta_que_vuelve_la_ración() {
    let m = con_mundo!();
    // Una ración de 4 de trigo por soldado y minuto (220 por minuto con 55 soldados) vacía el almacén en un cuarto de hora.
    let mut e = escenario_con_racion(m, 7, 4_000);
    let a = e.a;
    e.avanzar(24 * HORA);
    e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
    let h = e.heroe_en_la_plaza("ana");
    let milicia = e.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::MiliciaLanceros }).creadas[0];
    let lenadores = e.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::Lenadores }).creadas[0];
    let moral = |e: &Escenario, id| e.n.escuadra(id).unwrap().1.moral_en(e.n.ahora());
    let cantidad = |e: &Escenario, id| e.n.escuadra(id).unwrap().1.cantidad;
    // Mientras hay trigo para toda la ración, la moral no se mueve.
    let mut hambre = None;
    for _ in 0..200 {
        e.avanzar(MIN);
        if moral(&e, milicia) < 100_000 {
            hambre = Some((e.n.ahora(), moral(&e, milicia)));
            break;
        }
        assert_eq!(cantidad(&e, milicia), 25);
    }
    // El primer paso sin ración entera solo pierde en proporción a lo que faltó; mientras la ración sea mínima (lo que da la Granja), casi 20 puntos
    // por minuto. Hasta que la moral llega a cero no deserta nadie.
    let (t0, m0) = hambre.expect("el almacén se acaba");
    assert!((80_000..100_000).contains(&m0), "una ración incompleta quita menos que una ausente: {m0}");
    let (mut anterior, mut k) = (m0, 0);
    while anterior > 0 {
        k += 1;
        assert!(k <= 8, "con unos 20 puntos por minuto llega a cero en seis pasos");
        e.n.avanzar_hasta(t0 + k * MIN);
        let m = moral(&e, milicia);
        assert!(m < anterior && anterior - m <= 20_000 && (m == 0 || anterior - m > 10_000), "minuto {k} de hambre: de {anterior} a {m}");
        anterior = m;
        let desertaron = if m == 0 { (23, 28) } else { (25, 30) };
        assert_eq!((cantidad(&e, milicia), cantidad(&e, lenadores)), desertaron, "minuto {k}: la moral es {m}");
    }
    // Al llegar a cero, ese mismo minuto desertan ceil(5 %) de cada escuadra: ceil(25 × 0,05) = 2 y ceil(30 × 0,05) = 2.
    let des = hechos(&e.n, "tropas.desercion");
    assert_eq!(des.len(), 2);
    assert!(des.iter().all(|d| d.0 == t0 + k * MIN && d.1 == vec![a] && dato(d, "desertores") == 2 && dato(d, "heroe") == h as i64));
    e.n.avanzar_hasta(t0 + (k + 1) * MIN);
    assert_eq!((cantidad(&e, milicia), cantidad(&e, lenadores)), (21, 26), "cada minuto con la moral a cero deserta otro 5 %");
    // Vuelve el trigo: la moral sube 5 por minuto y no deserta nadie más.
    e.dar(a, Trigo, 3_000_000);
    let (c_milicia, desertados) = (cantidad(&e, milicia), hechos(&e.n, "tropas.desercion").len());
    e.avanzar(4 * MIN);
    assert_eq!(moral(&e, milicia), 20_000, "5 por minuto desde el minuto siguiente a la ración");
    assert_eq!((cantidad(&e, milicia), hechos(&e.n, "tropas.desercion").len()), (c_milicia, desertados));
}

// ───────────────────────────── 8. Escolta de caravanas ─────────────────────────────

/// La plaza A con su Mercado (cupo de escolta 100) y una plaza B de otra Facción, 72 horas después de fundarse (ya hay población de sobra para reclutar).
fn escenario_de_escolta(m: &M) -> (Escenario, u64) {
    let mut e = escenario(m, 7);
    let a = e.a;
    e.dar(a, Madera, 350_000);
    e.ok("admin", Comando::AnadirEdificio { asentamiento: a, tipo: asentamiento::catalogo::Tipo::Mercado });
    let mercado = |e: &Escenario| e.n.economia(a).unwrap().edificios.iter().any(|x| x.tipo == asentamiento::catalogo::Tipo::Mercado && x.activo());
    for _ in 0..80 {
        if mercado(&e) && e.n.ahora() >= 72 * HORA {
            break;
        }
        e.avanzar(HORA);
    }
    assert!(mercado(&e), "el Mercado acaba por levantarse");
    let t = e.n.ahora();
    let pb = sitio_rico(&e.n, &m.0, 3, 2);
    let b = fundar(&mut e.n, t, "b", "beto", 2, pb);
    (e, b)
}

#[test]
fn la_escolta_se_cede_con_la_caravana_parada_en_su_origen_y_vuelve_al_campamento_por_cada_camino() {
    let m = con_mundo!();
    let (mut e, b) = escenario_de_escolta(m);
    let a = e.a;
    let h = e.heroe_en_la_plaza("ana");
    let milicia = e.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::MiliciaLanceros }).creadas[0];
    let lenadores = e.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::Lenadores }).creadas[0];
    let caravana = |e: &mut Escenario| {
        let c = e.ok("ana", Comando::CrearCaravana { asentamiento: a }).creadas[0];
        e.ok("ana", Comando::AnadirCarro { caravana: c, tipo: comercio::caravana::TipoCarro::Basico });
        e.ok("ana", Comando::AnadirAnimal { caravana: c, carro: 0, animal: comercio::caravana::Animal::Buey });
        e.ok("ana", Comando::ReservarCaravana { caravana: c, reservada: true });
        c
    };
    let c = caravana(&mut e);
    assert_eq!(e.n.cupos_de_flota(a).unwrap().escolta, 100);
    let asignar = |caravana, escuadras: &[u64]| Comando::AsignarEscolta { caravana, heroe: h, escuadras: escuadras.to_vec() };
    let contenedor = |e: &Escenario, id| e.n.escuadra(id).unwrap().1.contenedor;

    // Rechazos, cada uno con su código y sin dejar rastro.
    assert_eq!(e.rechazada("beto", asignar(c, &[milicia])), rechazos::NO_AUTORIZADO, "cada residente cede lo suyo");
    assert_eq!(e.rechazada("ana", asignar(99, &[milicia])), rechazos::CARAVANA_DESCONOCIDA);
    assert_eq!(e.rechazada("ana", asignar(c, &[])), rechazos::CARAVANA_ESCOLTA_INVALIDA, "al menos una escuadra");
    assert_eq!(e.rechazada("ana", asignar(c, &[milicia, milicia])), rechazos::CARAVANA_ESCOLTA_INVALIDA, "sin repetidas");
    assert_eq!(e.rechazada("ana", asignar(c, &[99_999])), rechazos::CARAVANA_ESCOLTA_INVALIDA, "tiene que ser suya");
    e.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Campamento(0)) });
    assert_eq!(e.rechazada("ana", asignar(c, &[milicia])), rechazos::CARAVANA_ESCOLTA_INVALIDA, "el héroe tiene que residir en el origen de la caravana");
    e.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(a as u32)) });
    assert!(e.n.caravana(c).unwrap().escolta.is_empty() && contenedor(&e, milicia) == Contenedor::Campamento);

    // Se ceden enteras, no gastan el Liderazgo de quien las presta y salen del campamento (de la ración de la plaza).
    e.ok("ana", asignar(c, &[milicia, lenadores]));
    assert_eq!((contenedor(&e, milicia), contenedor(&e, lenadores)), (Contenedor::Escolta(c), Contenedor::Escolta(c)));
    assert_eq!(e.n.caravana(c).unwrap().escolta.iter().map(|x| (x.id, x.heroe, x.liderazgo)).collect::<Vec<_>>(), vec![(milicia, h, 14), (lenadores, h, 7)]);
    assert_eq!(hechos(&e.n, "comercio.caravana_escolta_cedida").len(), 1);
    assert_eq!(e.n.soldados_del_campamento(a), 0, "la escolta no consume la ración de la plaza");
    assert_eq!(e.rechazada("ana", asignar(c, &[milicia])), rechazos::CARAVANA_ESCOLTA_INVALIDA, "ya no está en el campamento");
    assert_eq!(e.rechazada("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::MiliciaLanceros }), rechazos::RECLUTAMIENTO_YA_LA_TIENES_FUERA, "una escuadra en escolta no se repone");

    // Cada héroe retira solo las suyas; las retiradas vuelven al campamento.
    assert_eq!(e.rechazada("beto", Comando::QuitarEscolta { caravana: c, heroe: h, escuadras: None }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", Comando::QuitarEscolta { caravana: c, heroe: h, escuadras: Some(vec![99_999]) }), rechazos::CARAVANA_ESCOLTA_INVALIDA);
    e.ok("ana", Comando::QuitarEscolta { caravana: c, heroe: h, escuadras: Some(vec![milicia]) });
    assert_eq!((contenedor(&e, milicia), contenedor(&e, lenadores)), (Contenedor::Campamento, Contenedor::Escolta(c)));
    e.ok("ana", Comando::QuitarEscolta { caravana: c, heroe: h, escuadras: None });
    assert_eq!(contenedor(&e, lenadores), Contenedor::Campamento);
    assert_eq!(e.ord("ana", Comando::QuitarEscolta { caravana: c, heroe: h, escuadras: None }).resultado, Resultado::SinCambios);
    assert_eq!(hechos(&e.n, "comercio.caravana_escolta_retirada").len(), 2);

    // Cancelar la preparación la devuelve.
    e.dar(a, Cobre, 100_000);
    e.ok("ana", asignar(c, &[milicia]));
    let salir = e.n.ahora() + HORA;
    e.ok("ana", Comando::PrepararCaravana { caravana: c, destino: b, carga: vec![(Cobre, 20_000)], salir_en: Some(salir) });
    assert_eq!(contenedor(&e, milicia), Contenedor::Escolta(c));
    assert_eq!(e.rechazada("ana", asignar(c, &[lenadores])), rechazos::CARAVANA_NO_DISPONIBLE, "solo con la caravana parada");
    e.ok("ana", Comando::CancelarCaravana { caravana: c });
    assert_eq!(contenedor(&e, milicia), Contenedor::Campamento);

    // Viaja con la caravana y vuelve al campamento cuando la caravana llega a casa.
    e.ok("ana", asignar(c, &[milicia, lenadores]));
    e.avanzar(11 * MIN);
    e.ok("ana", Comando::PrepararCaravana { caravana: c, destino: b, carga: vec![(Cobre, 20_000)], salir_en: None });
    assert_eq!(e.n.caravana(c).unwrap().estado, comercio::caravana::EstadoCaravana::EnTransito);
    let ida = e.n.caravana(c).unwrap().plan.as_ref().unwrap().fin().unwrap();
    e.n.avanzar_hasta(ida - 1);
    assert_eq!(contenedor(&e, milicia), Contenedor::Escolta(c));
    e.n.avanzar_hasta(ida);
    let vuelta = e.n.caravana(c).unwrap().plan.as_ref().unwrap().fin().unwrap();
    assert_eq!((e.n.caravana(c).unwrap().estado, contenedor(&e, milicia)), (comercio::caravana::EstadoCaravana::Retornando, Contenedor::Escolta(c)), "durante la vuelta sigue cedida");
    e.n.avanzar_hasta(vuelta);
    assert_eq!(e.n.caravana(c).unwrap().estado, comercio::caravana::EstadoCaravana::Disponible);
    assert_eq!((contenedor(&e, milicia), contenedor(&e, lenadores)), (Contenedor::Campamento, Contenedor::Campamento));
    assert!(e.n.caravana(c).unwrap().escolta.is_empty());

    // Si la caravana se pierde, la escolta vuelve a su campamento.
    e.ok("ana", asignar(c, &[milicia]));
    let t = e.n.ahora();
    e.n.aplicar_perdida_de_carros(t, c, &[0]).expect("pierde su único carro");
    assert!(e.n.caravana(c).is_none());
    assert_eq!(contenedor(&e, milicia), Contenedor::Campamento);
}

// ───────────────────────────── 9. La ruina de la plaza donde se reside ─────────────────────────────

#[test]
fn al_arruinarse_la_plaza_sus_residentes_pasan_a_la_plaza_hermana_mas_cercana_o_al_campamento_y_los_de_visita_salen() {
    let m = con_mundo!();
    let mut n = Nucleo::nuevo(arranque(m, 7));
    let pa = sitio_rico(&n, &m.0, 0, 1);
    let a = fundar(&mut n, 0, "a", "ana", 1, pa);
    assert!(n.aplicar_en(0, Peticion::nueva("admin", "nivel", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 }).de_admin()).aceptada());
    let pc = sitio_rico(&n, &m.0, 2, 1);
    let c = fundar(&mut n, 0, "c", "ana", 1, pc);
    let mut e = Escenario { n, a, cuenta: 0 };
    // La plaza A no construye nada: sin madera para su mantenimiento acabará en ruinas.
    e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
    let (ana, carla, beto) = (e.heroe_en_la_plaza("ana"), e.heroe("carla", 0), e.heroe("beto", 3));
    e.ok("admin", Comando::FijarResidencia { heroe: carla, residencia: Some(Residencia::Asentamiento(a as u32)) });
    e.ok("admin", Comando::ColocarHeroe { heroe: carla, ubicacion: Ubicacion::Asentamiento(a as u32) });
    e.ok("admin", Comando::ColocarHeroe { heroe: beto, ubicacion: Ubicacion::Asentamiento(a as u32) });
    assert_eq!(e.n.estado().militar.residentes_de(a).map(|h| h.heroe.id).collect::<Vec<_>>(), vec![ana, carla]);

    let mut ruina = None;
    for dia in 1..=150 {
        e.n.avanzar_hasta(dia * 24 * HORA);
        if e.n.economia(a).is_none() {
            ruina = Some(hechos(&e.n, "asentamiento.ruinas")[0].0);
            break;
        }
    }
    let ruina = ruina.expect("la plaza A cae en ruinas");
    // «ana» es de la Facción 1 y queda la plaza C: reside allí. «carla» no tiene Facción: al campamento más cercano a la ruina.
    let cercano = (0..m.0.campamentos().len()).min_by_key(|&i| (campamento(&m.0, i).dist2(pa), i)).unwrap() as u32;
    let h = |id| e.n.heroe(id).unwrap().heroe.clone();
    assert_eq!(h(ana).residencia, Some(Residencia::Asentamiento(c as u32)));
    assert_eq!(h(carla).residencia, Some(Residencia::Campamento(cercano)));
    assert_eq!(h(beto).residencia, Some(Residencia::Campamento(3)), "el que estaba de visita conserva su casa");
    // Los que estaban dentro quedan en el campamento más cercano (no hay plaza a la que volver).
    for id in [ana, carla, beto] {
        assert_eq!(h(id).presencia.ubicacion, Ubicacion::Mercenarios(cercano));
    }
    assert!(e.n.estado().militar.residentes_de(a).next().is_none());
    assert_eq!(e.n.estado().militar.residentes_de(c).map(|x| x.heroe.id).collect::<Vec<_>>(), vec![ana]);
    let reubicados = hechos(&e.n, "heroe.reubicado");
    assert_eq!(reubicados.len(), 3);
    assert!(reubicados.iter().all(|r| r.0 == ruina && dato(r, "ruina") == a as i64));
}

// ───────────────────────────── 10. Las tecnologías de la administración y quién manda en la plaza ─────────────────────────────

/// Un sitio fundable, verde y con mucho bosque, a `r` del punto `c`.
fn sitio_junto_a(n: &Nucleo, c: Punto, r: i64) -> Option<Punto> {
    (0..36).map(|g| ((g * 10) as f64).to_radians()).map(|a| Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64)).find(|&p| {
        n.viabilidad(p, 1).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6 && !v.condenada)
    })
}

#[test]
fn las_tecnologias_de_la_administracion_abren_las_obras_que_las_piden_y_un_heroe_ciudadano_manda_en_su_plaza() {
    use asentamiento::catalogo::Tipo;
    let m = con_mundo!();
    let mapa = nucleo::economia::Mapa::de(&m.0);
    let mut n = Nucleo::nuevo(arranque(m, 7));
    let (hierro, sitio) = (0..mapa.nodos.len())
        .filter(|&i| mapa.nodos[i].tipo == Tipo::MinaHierro)
        .find_map(|i| sitio_junto_a(&n, mapa.nodos[i].pos, 6_000).map(|p| (i as u32, p)))
        .expect("hay un nodo de hierro junto a un sitio rico");
    let a = fundar(&mut n, 0, "a", "ana", 1, sitio);
    let mut e = Escenario { n, a, cuenta: 0 };
    e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel: 2 });
    for _ in 0..9 {
        e.ok("admin", Comando::AgrandarZona { asentamiento: a });
    }
    assert!(e.n.en_zona(a, mapa.nodos[hierro as usize].pos));
    // El hierro pide «forja_hierro_temprana»: sin ella no es candidato ni se marca.
    assert!(!e.n.candidatos_de_pueblo(a).contains(&(hierro, Tipo::MinaHierro)));
    assert_eq!(e.rechazada("ana", Comando::MarcarPueblo { asentamiento: a, nodo: hierro }), rechazos::PUEBLO_TECNOLOGIA_FALTANTE);
    e.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["forja_hierro_temprana".into()] });
    assert!(e.n.candidatos_de_pueblo(a).contains(&(hierro, Tipo::MinaHierro)), "con la tecnología de la Facción ya lo es");
    e.ok("ana", Comando::MarcarPueblo { asentamiento: a, nodo: hierro });
    // La tecnología es de la Facción: otra Facción sigue sin ella.
    assert!(!e.n.tiene_tecnologia(2, "forja_hierro_temprana"));

    // Quien no tiene columna ni héroe de la Facción no manda en la plaza; con un héroe ciudadano sí (la orden cae después, en las reglas del comercio).
    let carla = e.heroe("carla", 0);
    assert_eq!(e.rechazada("carla", Comando::CrearCaravana { asentamiento: a }), rechazos::NO_AUTORIZADO);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: carla, faccion: Some(2) });
    assert_eq!(e.rechazada("carla", Comando::CrearCaravana { asentamiento: a }), rechazos::NO_AUTORIZADO, "ciudadana de otra Facción");
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: carla, faccion: Some(1) });
    assert_eq!(e.rechazada("carla", Comando::CrearCaravana { asentamiento: a }), rechazos::CARAVANA_SIN_MERCADO, "mandar, manda: lo que falta es el Mercado");
    assert_eq!(e.ord("admin", Comando::FijarFaccionDeHeroe { heroe: carla, faccion: Some(1) }).resultado, Resultado::SinCambios);
}

// ───────────────────────────── 11. El estado militar en la foto y en el hash ─────────────────────────────

fn estado_con_heroes() -> nucleo::Estado {
    use comercio::mercenarios::CupoDiario;
    use militar::escuadra::Escuadra;
    use militar::heroe::{Heroe, Loadout};
    use militar::mercenarios::Reclutas;
    let mut s = nucleo::Estado::nuevo(Reglas::default(), "h".into(), 1, 0);
    let mr = s.reglas.economia.militar.clone();
    let mut h = Heroe::nuevo(&mr, 5, Controlador::Humano, 3, 6);
    h.residencia = Some(Residencia::Asentamiento(2));
    h.presencia.ubicacion = Ubicacion::Asentamiento(2);
    h.presencia.desconecta_en = Some(150_000);
    h.nivel = 3;
    h.experiencia = 11;
    h.puntos_de_atributo = 4;
    h.puntos_de_perk = 2;
    h.atributos.armadura = 7;
    h.perks_desbloqueados = vec![1, 4];
    h.loadouts.push(Loadout { id: 9, nombre: "Asalto".into(), escuadras: vec![10, 11], perks: vec![4], activo: false });
    h.monedas.plata = 12;
    h.herido_hasta = Some(120_000);
    h.viveres = 33_000;
    h.racion_en = Some(40_000);
    for (i, (tropa, contenedor)) in [(Tropa::MiliciaLanceros, Contenedor::Campamento), (Tropa::Arqueros, Contenedor::Ejercito(4)), (Tropa::Honderos, Contenedor::Escolta(7)), (Tropa::Hequetai, Contenedor::Fuera), (Tropa::Granjeros, Contenedor::Campamento)].into_iter().enumerate() {
        let mut e = Escuadra::nueva(&mr, 10 + i as u64, 5, tropa, 5 + i as i64, Some(2), 0);
        e.contenedor = contenedor;
        h.escuadras.push(e);
    }
    h.escuadras[0].en_guarnicion = true;
    h.escuadras[0].reserva_batalla = Some(77);
    h.escuadras[0].experiencia = 5_500;
    h.escuadras[0].nivel = 2;
    h.escuadras[0].moral = base::Ritmo { valor_en_t0: 60_000, t0: 10, por_minuto: -20_000, maximo: Some(100_000), minimo: Some(0) };
    h.escuadras[4].prestada = Some(1);
    h.escuadras[4].plaza_origen = None;
    s.militar.insertar(nucleo::HeroeNucleo { almacen_personal: [(Oro, 8)].into_iter().collect(), cupo: CupoDiario { dia: 4, comprado: [(Madera, 9)].into_iter().collect() }, columna: Some(77), carro_fuera: [(Cobre, 3)].into_iter().collect(), ..nucleo::HeroeNucleo::nuevo(h, "ana", Some(1)) });
    let mut otro = Heroe::nuevo(&mr, 20, Controlador::Bot, 1, 21);
    otro.presencia.ubicacion = Ubicacion::Desconectado(Punto::new(5, -7));
    otro.presencia.fuera = true;
    s.militar.insertar(nucleo::HeroeNucleo::nuevo(otro, "bot", None));
    s.militar.reclutas.insert(2, militar::mercenarios::Reclutas { poblacion: 40_000, en: 1_234 });
    s.militar.convocatorias.insert(
        7,
        militar::convocatoria::Convocatoria {
            id: 7,
            faccion: 1,
            lugar: militar::convocatoria::Lugar::Asentamiento(2),
            lider: 5,
            politica: militar::columna::Politica::Preguntar,
            integrantes: vec![militar::convocatoria::Integrante { heroe: 5, unido_en: 10, seleccion: militar::convocatoria::Seleccion { escuadras: vec![11, 10], carga: [(Oro, 5)].into_iter().collect() } }],
            peticiones: vec![militar::convocatoria::Peticion { heroe: 20, pedido_en: 1, expira_en: 11, seleccion: Default::default() }],
            creada_en: 3,
        },
    );
    s.militar.tecnologias.insert(1, ["arco_compuesto".to_owned(), "leva_comunal".to_owned()].into_iter().collect());
    let clave = s.generaciones.clave(5);
    s.cola.programar(150_000, nucleo::PRIO_SALIDA, clave, nucleo::Carga::SalidaDelMundo);
    // Los cinco plazos militares viajan en la cola.
    for (i, suceso) in [nucleo::SucesoMilitar::Desercion, nucleo::SucesoMilitar::CaducaFormacion, nucleo::SucesoMilitar::CaducaPeticion { columna: 4 }, nucleo::SucesoMilitar::CaducaPeticionDeConvocatoria { convocatoria: 7 }, nucleo::SucesoMilitar::PresaFueraDeVista { presa: 9, presa_generacion: 3 }].into_iter().enumerate() {
        let clave = s.generaciones.clave(40 + i as u64);
        s.cola.programar(200_000 + i as i64, nucleo::PRIO_UMBRAL, clave, nucleo::Carga::Militar(suceso));
    }
    let _ = Reclutas::llenos;
    s
}

fn h5(m: &mut nucleo::EstadoMilitar) -> &mut militar::heroe::Heroe {
    &mut m.heroes.get_mut(&5).unwrap().heroe
}

#[test]
fn el_estado_militar_se_serializa_de_ida_y_vuelta_y_cualquier_campo_cambia_el_hash() {
    use nucleo::EstadoMilitar;
    let reglas = Reglas::default();
    let base = estado_con_heroes();
    let bytes = base.serializar();
    assert_eq!(&bytes[..5], b"BAN16");
    let vuelta = nucleo::Estado::deserializar(&bytes, &reglas, "h").unwrap();
    assert_eq!(vuelta.serializar(), bytes, "serializar → deserializar → serializar es idéntico");
    assert_eq!(vuelta.militar, base.militar, "incluido el índice de residentes, que se reconstruye");
    assert_eq!(vuelta.militar.residentes_de(2).map(|h| h.heroe.id).collect::<Vec<_>>(), vec![5]);
    assert!(vuelta.cola.volcar().iter().any(|ev| matches!(ev.carga, nucleo::Carga::SalidaDelMundo) && ev.instante == 150_000 && ev.prioridad == nucleo::PRIO_SALIDA), "la salida pendiente viaja en la cola");
    // Una foto cortada o con basura detrás no se admite.
    assert!(nucleo::Estado::deserializar(&bytes[..bytes.len() - 3], &reglas, "h").is_err());
    assert!(nucleo::Estado::deserializar(&[&bytes[..], &[0u8][..]].concat(), &reglas, "h").is_err());

    type Cambio = (&'static str, Box<dyn Fn(&mut EstadoMilitar)>);
    let cambios: Vec<Cambio> = vec![
        ("cuenta", Box::new(|m| m.heroes.get_mut(&5).unwrap().cuenta = "otra".into())),
        ("faccion", Box::new(|m| m.heroes.get_mut(&5).unwrap().faccion = Some(2))),
        ("faccion.ninguna", Box::new(|m| m.heroes.get_mut(&5).unwrap().faccion = None)),
        ("almacen_personal", Box::new(|m| *m.heroes.get_mut(&5).unwrap().almacen_personal.get_mut(&Oro).unwrap() += 1)),
        ("cupo.dia", Box::new(|m| m.heroes.get_mut(&5).unwrap().cupo.dia += 1)),
        ("cupo.comprado", Box::new(|m| *m.heroes.get_mut(&5).unwrap().cupo.comprado.get_mut(&Madera).unwrap() += 1)),
        ("controlador", Box::new(|m| h5(m).controlador = Controlador::Bot)),
        ("liderazgo", Box::new(|m| h5(m).liderazgo_base += 1)),
        ("ubicacion", Box::new(|m| h5(m).presencia.ubicacion = Ubicacion::Columna(2))),
        ("desconecta_en", Box::new(|m| h5(m).presencia.desconecta_en = None)),
        ("fuera", Box::new(|m| h5(m).presencia.fuera = true)),
        ("residencia", Box::new(|m| m.mudar(5, Some(Residencia::Campamento(2))))),
        ("residencia.ninguna", Box::new(|m| m.mudar(5, None))),
        ("nivel", Box::new(|m| h5(m).nivel += 1)),
        ("experiencia", Box::new(|m| h5(m).experiencia += 1)),
        ("puntos_atributo", Box::new(|m| h5(m).puntos_de_atributo += 1)),
        ("puntos_perk", Box::new(|m| h5(m).puntos_de_perk += 1)),
        ("atributos", Box::new(|m| h5(m).atributos.fuerza += 1)),
        ("perks", Box::new(|m| h5(m).perks_desbloqueados.push(8))),
        ("loadout.nombre", Box::new(|m| h5(m).loadouts[1].nombre.push('!'))),
        ("loadout.escuadras", Box::new(|m| h5(m).loadouts[1].escuadras.reverse())),
        ("loadout.perks", Box::new(|m| h5(m).loadouts[1].perks.clear())),
        ("loadout.activo", Box::new(|m| h5(m).loadouts[1].activo = true)),
        ("monedas", Box::new(|m| h5(m).monedas.oro += 1)),
        ("herido_hasta", Box::new(|m| h5(m).herido_hasta = None)),
        ("viveres", Box::new(|m| h5(m).viveres += 1)),
        ("racion_en", Box::new(|m| h5(m).racion_en = None)),
        ("columna", Box::new(|m| m.heroes.get_mut(&5).unwrap().columna = None)),
        ("carro_fuera", Box::new(|m| *m.heroes.get_mut(&5).unwrap().carro_fuera.get_mut(&Cobre).unwrap() += 1)),
        ("sintetico", Box::new(|m| m.heroes.get_mut(&5).unwrap().sintetico = true)),
        ("convocatoria.politica", Box::new(|m| m.convocatorias.get_mut(&7).unwrap().politica = militar::columna::Politica::Aceptar)),
        ("convocatoria.lugar", Box::new(|m| m.convocatorias.get_mut(&7).unwrap().lugar = militar::convocatoria::Lugar::Campamento(2))),
        ("convocatoria.seleccion.orden", Box::new(|m| m.convocatorias.get_mut(&7).unwrap().integrantes[0].seleccion.escuadras.reverse())),
        ("convocatoria.seleccion.carga", Box::new(|m| m.convocatorias.get_mut(&7).unwrap().integrantes[0].seleccion.carga.clear())),
        ("convocatoria.peticion", Box::new(|m| m.convocatorias.get_mut(&7).unwrap().peticiones[0].expira_en += 1)),
        ("convocatoria.peticion.seleccion", Box::new(|m| m.convocatorias.get_mut(&7).unwrap().peticiones[0].seleccion.escuadras.push(1))),
        ("convocatoria.cancelada", Box::new(|m| {
            m.convocatorias.clear();
        })),
        ("convocatoria.creada_en", Box::new(|m| m.convocatorias.get_mut(&7).unwrap().creada_en += 1)),
        ("escuadra.cantidad", Box::new(|m| h5(m).escuadras[1].cantidad += 1)),
        ("escuadra.nivel", Box::new(|m| h5(m).escuadras[1].nivel += 1)),
        ("escuadra.experiencia", Box::new(|m| h5(m).escuadras[1].experiencia += 1)),
        ("escuadra.moral.valor", Box::new(|m| h5(m).escuadras[0].moral.valor_en_t0 += 1)),
        ("escuadra.moral.t0", Box::new(|m| h5(m).escuadras[0].moral.t0 += 1)),
        ("escuadra.moral.ritmo", Box::new(|m| h5(m).escuadras[0].moral.por_minuto += 1)),
        ("escuadra.contenedor", Box::new(|m| h5(m).escuadras[1].contenedor = Contenedor::Ejercito(5))),
        ("escuadra.en_guarnicion", Box::new(|m| h5(m).escuadras[0].en_guarnicion = false)),
        ("escuadra.reserva", Box::new(|m| h5(m).escuadras[0].reserva_batalla = None)),
        ("escuadra.prestada", Box::new(|m| h5(m).escuadras[4].prestada = Some(2))),
        ("escuadra.plaza_origen", Box::new(|m| h5(m).escuadras[1].plaza_origen = Some(9))),
        ("escuadra.orden", Box::new(|m| h5(m).escuadras.swap(0, 1))),
        ("otro.ubicacion", Box::new(|m| m.heroes.get_mut(&20).unwrap().heroe.presencia.ubicacion = Ubicacion::Desconectado(Punto::new(5, 7)))),
        ("heroe.nuevo", Box::new(|m| {
            let mr = militar::Reglas::cargar();
            m.insertar(nucleo::HeroeNucleo::nuevo(militar::heroe::Heroe::nuevo(&mr, 30, Controlador::Humano, 0, 31), "c", None));
        })),
        ("reclutas.poblacion", Box::new(|m| m.reclutas.get_mut(&2).unwrap().poblacion += 1)),
        ("reclutas.en", Box::new(|m| m.reclutas.get_mut(&2).unwrap().en += 1)),
        ("reclutas.otro", Box::new(|m| {
            m.reclutas.insert(3, militar::mercenarios::Reclutas { poblacion: 1, en: 0 });
        })),
        ("tecnologias.una_menos", Box::new(|m| {
            m.tecnologias.get_mut(&1).unwrap().remove("arco_compuesto");
        })),
        ("tecnologias.faccion", Box::new(|m| {
            m.tecnologias.insert(2, ["arco_compuesto".to_owned()].into_iter().collect());
        })),
    ];
    let mut vistos = std::collections::HashSet::from([base.hash()]);
    for (nombre, f) in cambios {
        let mut s = nucleo::Estado::deserializar(&bytes, &reglas, "h").unwrap();
        f(&mut s.militar);
        assert!(vistos.insert(s.hash()), "el hash no cambió (o coincidió con otro cambio) al tocar {nombre}");
    }
}

// ───────────────────────────── 12. Determinismo y recuperación ─────────────────────────────

impl Escenario {
    fn ok_en(&mut self, t: Instante, actor: &str, c: Comando) -> Respuesta {
        let p = self.peticion(actor, c);
        let r = self.n.aplicar_en(t, p);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    fn avanzar_hasta(&mut self, t: Instante) {
        self.n.avanzar_hasta(t);
    }
}

/// Un guion militar: dos héroes, tropa en la plaza y prestada, loadout, una salida pendiente, un herido, volver y mudarse. Todas las órdenes llevan
/// su instante (como en el registro), de modo que da igual cuándo avance el reloj entre una y otra.
fn guion(e: &mut Escenario, avanzar: &dyn Fn(&mut Escenario, Instante)) {
    let (a, base) = (e.a, 24 * HORA);
    avanzar(e, base);
    let crear = |e: &mut Escenario, cuenta: &str, camp| e.ok_en(base, cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: camp, controlador: Controlador::Humano }).creadas[0];
    let (ana, beto) = (crear(e, "ana", 0), crear(e, "beto", 0));
    e.ok_en(base, "admin", Comando::FijarFaccionDeHeroe { heroe: ana, faccion: Some(1) });
    e.ok_en(base, "admin", Comando::FijarResidencia { heroe: ana, residencia: Some(Residencia::Asentamiento(a as u32)) });
    e.ok_en(base, "admin", Comando::ColocarHeroe { heroe: ana, ubicacion: Ubicacion::Asentamiento(a as u32) });
    e.ok_en(base, "admin", Comando::DarAlmacenPersonal { heroe: beto, recurso: Oro, cantidad: 500_000 });
    let milicia = e.ok_en(base, "ana", Comando::ReclutarTropa { asentamiento: a, heroe: ana, tropa: Tropa::MiliciaLanceros }).creadas[0];
    let prestadas = e.ok_en(base, "beto", Comando::PedirPrestamo { heroe: beto, tropas: vec![Tropa::Granjeros, Tropa::Mineros] }).creadas;
    e.ok_en(base, "beto", Comando::GuardarLoadout { heroe: beto, loadout: None, nombre: "Leva".into(), escuadras: prestadas.clone(), perks: vec![], activo: Some(true) });
    e.ok_en(base, "beto", Comando::OrdenarEscuadras { heroe: beto, escuadras: vec![prestadas[1]] });
    avanzar(e, base + 10 * MIN);
    e.ok_en(base + 10 * MIN, "beto", Comando::Desconectarse { heroe: beto });
    e.ok_en(base + 10 * MIN, "admin", Comando::HerirHeroe { heroe: ana });
    avanzar(e, base + 30 * MIN);
    e.ok_en(base + 30 * MIN, "ana", Comando::Desconectarse { heroe: ana });
    avanzar(e, base + 31 * MIN);
    e.ok_en(base + 31 * MIN, "ana", Comando::Conectarse { heroe: ana });
    avanzar(e, base + 40 * MIN);
    e.ok_en(base + 40 * MIN, "beto", Comando::Conectarse { heroe: beto });
    e.ok_en(base + 40 * MIN, "admin", Comando::FijarResidencia { heroe: beto, residencia: Some(Residencia::Campamento(1)) });
    avanzar(e, base + 3 * HORA);
    e.n.avanzar_hasta(base + 3 * HORA);
    assert_eq!(e.n.escuadra(milicia).unwrap().1.cantidad, 25);
}

fn correr(m: &M, semilla: u64, avanzar: &dyn Fn(&mut Escenario, Instante)) -> Nucleo {
    let mut e = escenario(m, semilla);
    guion(&mut e, avanzar);
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
    let ana = normal.heroe_de_cuenta("ana").unwrap();
    let beto = normal.heroe_de_cuenta("beto").unwrap();
    assert_eq!(ana.heroe.escuadras.len(), 1);
    assert_eq!((beto.heroe.escuadras.len(), beto.heroe.residencia), (0, Some(Residencia::Campamento(1))), "la prestada se retiró al mudarse");
    assert!(!hechos(&normal, "heroe.sale_del_mundo").is_empty() || !hechos(&normal, "heroe.vuelve_al_mundo").is_empty());
    // El hash se mueve con el estado militar: el mismo mundo sin el guion da otro.
    let mut sin_guion = escenario(m, 3);
    sin_guion.avanzar(27 * HORA);
    assert_ne!(normal.hash_estado(), sin_guion.n.hash_estado());
}

#[test]
fn recuperar_desde_foto_con_tropa_guarnicion_herido_y_una_salida_pendiente_da_el_mismo_mundo() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9) };
    // Antes de la foto: tropa propia y prestada, un herido y una desconexión a medias. Después: el hambre de la guarnición, la salida y volver.
    let primera_mitad = |e: &mut Escenario| {
        let a = e.a;
        e.avanzar(24 * HORA);
        let ana = e.heroe_en_la_plaza("ana");
        let beto = e.heroe("beto", 0);
        e.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: ana, tropa: Tropa::MiliciaLanceros });
        e.ok("beto", Comando::PedirPrestamo { heroe: beto, tropas: vec![Tropa::Granjeros, Tropa::Lenadores] });
        e.ok("admin", Comando::HerirHeroe { heroe: ana });
        e.avanzar(MIN);
        e.ok("beto", Comando::Desconectarse { heroe: beto });
        (ana, beto)
    };
    let segunda_mitad = |e: &mut Escenario, ana: u64, beto: u64| {
        e.avanzar(4 * HORA);
        assert!(!e.n.heroe_herido(ana) && e.n.heroe(beto).unwrap().heroe.presencia.fuera, "la salida pendiente venció tras recuperar");
        e.ok("beto", Comando::Conectarse { heroe: beto });
        e.ok("admin", Comando::FijarResidencia { heroe: beto, residencia: Some(Residencia::Campamento(2)) });
        e.avanzar(2 * HORA);
    };

    let sin_parar = {
        let mut e = escenario(m, 9);
        let (ana, beto) = primera_mitad(&mut e);
        e.avanzar(30 * MIN);
        segunda_mitad(&mut e, ana, beto);
        (e.n.hash_estado(), e.n.estado().militar.clone())
    };
    let (a_mitad, ana, beto, cuenta) = {
        let n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        let mut n = n;
        let p = sitio_rico(&n, &m.0, 0, 1);
        let a = fundar(&mut n, 0, "a", "ana", 1, p);
        let mut e = Escenario { n, a, cuenta: 0 };
        let (ana, beto) = primera_mitad(&mut e);
        assert!(e.n.metricas().ordenes_aceptadas > 8, "las órdenes militares entran al registro");
        e.n.foto_ahora();
        e.avanzar(30 * MIN);
        e.n.apagar().unwrap();
        (e.n.hash_estado(), ana, beto, e.cuenta)
    };
    let n = Nucleo::con_persistencia(con(ajustes), dir.path(), Default::default()).unwrap();
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), a_mitad, "foto + registro reconstruyen el mismo mundo, con la tropa, el herido y la salida a medias");
    let a = *n.estado().asentamientos.keys().next().unwrap();
    let mut e = Escenario { n, a, cuenta };
    assert!(e.n.heroe_herido(ana) || e.n.ahora() >= e.n.heroe(ana).unwrap().heroe.herido_hasta.unwrap());
    segunda_mitad(&mut e, ana, beto);
    assert_eq!(e.n.estado().militar, sin_parar.1, "el estado militar recuperado sigue igual que el que no se interrumpió");
    assert_eq!(e.n.hash_estado(), sin_parar.0);
    e.n.apagar().unwrap();
}
