//! Las columnas en el núcleo (fase 4B-2, Movimiento_Ejercitos, Presencia_Y_Convocatoria): salir al mundo y entrar en un lugar, comer y pasar hambre con el
//! consumo calculado de la composición, la velocidad derivada, unir en campo y en la plaza, separar, el mando, formar un ejército, repostar, las caravanas
//! adjuntas, la desconexión con la columna, las convocatorias, y determinismo y recuperación desde foto. Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;
use asentamiento::Recurso::{self, *};
use base::{Instante, Punto, MS_POR_MINUTO};
use militar::catalogo::Tropa;
use militar::escuadra::Contenedor;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use militar::columna::{Politica, TipoColumna};
use militar::convocatoria::Lugar;
use militar::racion::{self, Estado as Marcha};
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
        self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento, controlador: Controlador::Humano, nombre: None }).creadas[0]
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


fn fundar(n: &mut Nucleo, t: Instante, dueno: &str, faccion: u32, p: Punto) -> u64 {
    let e = n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap();
    assert!(n.aplicar_en(t, Peticion::nueva("admin", &format!("c-{e}"), Comando::DarCaravanaFundacion { ejercito: e }).de_admin()).aceptada());
    let r = n.aplicar_en(t, Peticion::nueva(dueno, &format!("f-{e}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

fn escenario(m: &M, semilla: u64) -> Escenario {
    let mut n = Nucleo::nuevo(arranque(m, semilla));
    let p = sitio_rico(&n, &m.0, 0, 1);
    let a = fundar(&mut n, 0, "ana", 1, p);
    Escenario { n, a, cuenta: 0 }
}

impl Escenario {
    /// Un héroe de la cuenta ciudadano de la Facción 1 que reside en la plaza, está dentro y tiene una escuadra de milicia (25 lanceros) en su campamento.
    fn con_milicia(&mut self, cuenta: &str) -> (u64, u64) {
        let h = self.heroe_en_la_plaza(cuenta);
        let r = self.ok(cuenta, Comando::ReclutarTropa { asentamiento: self.a, heroe: h, tropa: Tropa::MiliciaLanceros });
        (h, r.creadas[0])
    }
    /// Un héroe de la cuenta, ciudadano de la Facción 1, residente del campamento `c` con dos escuadras prestadas (30 granjeros y 30 mineros).
    fn con_prestamo(&mut self, cuenta: &str, c: u32) -> (u64, Vec<u64>) {
        let h = self.heroe(cuenta, c);
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
        let ids = self.ok(cuenta, Comando::PedirPrestamo { heroe: h, tropas: vec![Tropa::Granjeros, Tropa::Mineros] }).creadas;
        (h, ids)
    }
    /// Sube la plaza a nivel 2 y espera a que levante un Barracón (cupo de guarnición de 7 por héroe residente).
    fn con_barracon(&mut self) {
        let a = self.a;
        self.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel: 2 });
        self.dar(a, Madera, 1_000_000);
        self.ok("admin", Comando::AnadirEdificio { asentamiento: a, tipo: asentamiento::catalogo::Tipo::Barracon });
        let activo = |e: &Escenario| e.n.economia(a).unwrap().edificios.iter().any(|x| x.tipo == asentamiento::catalogo::Tipo::Barracon && x.activo());
        for _ in 0..80 {
            if activo(self) {
                break;
            }
            self.avanzar(HORA);
        }
        assert!(activo(self), "el Barracón acaba por levantarse");
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
    fn escuadra(&self, h: u64, id: u64) -> militar::escuadra::Escuadra {
        self.n.heroe(h).unwrap().heroe.escuadra(id).unwrap().clone()
    }
}

fn mr() -> militar::Reglas {
    militar::Reglas::cargar()
}

// ───────────────────────────── 1. Salir al mundo ─────────────────────────────

#[test]
fn salir_al_mundo_valida_en_orden_carga_el_carro_llena_los_viveres_y_un_rechazo_no_deja_rastro() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let (h, milicia) = e.con_milicia("ana");
    let a = e.a;
    let salir = |heroe, escuadras: &[u64], carga: &[(Recurso, i64)]| Comando::SalirAlMundo { asentamiento: a, heroe, escuadras: escuadras.to_vec(), carga: carga.to_vec() };
    let beto = e.heroe("beto", 0);
    let (v, hash, ids) = (e.n.version(), e.n.hash_estado(), e.n.estado().siguiente_id);
    assert_eq!(e.rechazada("beto", salir(h, &[], &[])), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", Comando::SalirAlMundo { asentamiento: 9_999, heroe: h, escuadras: vec![], carga: vec![] }), rechazos::ASENTAMIENTO_DESCONOCIDO);
    assert_eq!(e.rechazada("ana", salir(99, &[], &[])), rechazos::HEROE_DESCONOCIDO);
    assert_eq!(e.rechazada("beto", salir(beto, &[], &[])), rechazos::SALIDA_SOLO_DE_TU_RESIDENCIA, "solo se sale al mundo desde la propia residencia");
    assert_eq!(e.rechazada("ana", salir(h, &[99_999], &[])), rechazos::ESCUADRA_AJENA);
    assert_eq!(e.rechazada("ana", salir(h, &[milicia, milicia], &[])), rechazos::ESCUADRA_REPETIDA);
    assert_eq!(e.rechazada("ana", salir(h, &[], &[(Madera, -1)])), rechazos::CARGA_NEGATIVA);
    assert_eq!(e.rechazada("ana", salir(h, &[], &[(Madera, 400_000), (Piedra, 101_000)])), rechazos::CARRO_LLENO, "el tope es del total del carro: 500");
    assert_eq!(e.rechazada("ana", salir(h, &[], &[(Hierro, 1_000)])), rechazos::ALMACEN_SIN_EXISTENCIAS);
    assert_eq!((e.n.version(), e.n.hash_estado(), e.n.estado().siguiente_id), (v, hash, ids), "los rechazos no dejan rastro");

    // La salida: columna personal quieta en la plaza, con la milicia, el carro y los víveres llenos de la plaza.
    let (madera0, trigo0) = (e.almacen(Madera), e.almacen(Trigo));
    let r = e.ok("ana", salir(h, &[milicia], &[(Madera, 30_000)]));
    let col = r.creadas[0];
    let c = &e.ejercito(col).columna;
    assert_eq!((c.tipo, c.lider, c.politica, c.estado, c.faccion), (TipoColumna::Personal, h, Politica::Rechazar, Marcha::Estacionada, Some(1)));
    assert_eq!((c.participantes.len(), c.escuadras.clone(), c.carga[&Madera]), (1, vec![milicia], 30_000));
    assert_eq!(e.ejercito(col).posicion, e.plaza());
    assert!(!e.ejercito(col).marchando());
    assert_eq!(madera0 - e.almacen(Madera), 30_000, "la carga sale del almacén");
    let llenado = trigo0 - e.almacen(Trigo);
    assert_eq!(e.viveres(col), llenado, "los víveres se llenan de la plaza sin bajar de su reserva");
    assert!(llenado > 0 && llenado <= 350_000);
    assert_eq!(e.escuadra(h, milicia).contenedor, Contenedor::Ejercito(col));
    let hn = e.n.heroe(h).unwrap();
    assert_eq!((hn.columna, hn.heroe.presencia.ubicacion, hn.heroe.viveres), (Some(col), Ubicacion::Columna(col), 0));
    // Come lo que dice su composición (25 soldados y un héroe, acampada), y va a la velocidad de su escuadra más lenta.
    assert_eq!(e.n.consumo_de_columna(col), Some(racion::racion_columna(&mr(), 25, 1, Marcha::Estacionada)));
    assert_eq!(e.n.velocidad_de_ejercito(col), Some(mr().tropas.tropa(Tropa::MiliciaLanceros).velocidad / 60));
    assert_eq!(e.n.alcance_de_vista(col), Some(30_000), "con soldados en pie ve a 30");
    assert_eq!(e.n.soldados_de_columna(col), Some(25));
    let sale = hechos(&e.n, "jugador.sale_al_mundo");
    assert_eq!((sale.len(), sale[0].1.clone(), dato(&sale[0], "escuadras"), dato(&sale[0], "cargaTotal")), (1, vec![a], 1, 30_000));
    assert_eq!(e.rechazada("ana", salir(h, &[], &[])), rechazos::HEROE_YA_EN_COLUMNA, "no se sale dos veces");
}

/// Al salir al mundo desde su plaza el héroe se queda con la foto de su interior (Niebla_De_Guerra §4.5), ya sin la tropa ni la carga que se lleva: el almacén, la cola (lo que no está activo) y
/// la guarnición. Viaja en su `Heroe.plazas_recordadas`, no cambia mientras está fuera aunque la plaza siga viviendo, la siguiente salida la sustituye y vuelve de la foto (`BAN25`).
#[test]
fn salir_al_mundo_deja_al_heroe_la_foto_del_interior_de_su_plaza() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let (h, milicia) = e.con_milicia("ana");
    let a = e.a;
    let lenadores = e.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::Lenadores }).creadas[0];
    e.con_barracon();
    e.ok("ana", Comando::AsignarGuarnicion { heroe: h, escuadra: lenadores });
    e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
    let vivienda = e.ok("ana", Comando::AnadirEdificio { asentamiento: a, tipo: asentamiento::catalogo::Tipo::Vivienda }).creadas[0] as u32;
    let (t, madera) = (e.n.ahora(), e.almacen(Madera));
    e.ok("ana", Comando::SalirAlMundo { asentamiento: a, heroe: h, escuadras: vec![milicia], carga: vec![(Madera, 40_000)] });
    let recuerdo = e.n.heroe(h).unwrap().plazas_recordadas[&(a as u32)].clone();
    assert_eq!((recuerdo.visto_en, recuerdo.guarnicion.clone()), (t, vec![lenadores]));
    // Lo que deja atrás, sin la carga que se lleva, tal como se ve (con la unidad que se está consumiendo, EE-2).
    assert_eq!(e.almacen(Madera), madera - 40_000);
    assert_eq!(recuerdo.almacen[&Madera], e.n.economia(a).unwrap().almacen.visible(Madera));
    assert!(recuerdo.cola.iter().any(|b| b.id == vivienda && b.completa_en.is_none()), "la vivienda en cola");
    assert!(e.n.economia(a).unwrap().edificios.iter().filter(|b| b.activo()).all(|b| recuerdo.cola.iter().all(|x| x.id != b.id)), "lo activo no se recuerda: viaja vivo en la ficha");
    let p = nucleo::proyeccion::propios::heroe(&e.n, e.n.heroe(h).unwrap());
    let pr = &p.plazas_recordadas[&format!("as-{a}")];
    assert_eq!((pr.visto_en_ms, pr.almacen["madera"].cantidad.as_ref().unwrap().valor_en_t0, pr.guarnicion_escuadrones_ids.clone()), (t, recuerdo.almacen[&Madera], vec![format!("esc-{lenadores}")]));
    assert!(pr.cola.iter().any(|b| b.id == format!("ed-{a}-{vivienda}") && b.estado == contrato::v1::EstadoEdificio::EdificioEnCola as i32));
    // Fuera, la plaza sigue viviendo y la foto no cambia; la foto del estado la conserva.
    e.dar(a, Madera, 7_000);
    assert_eq!(e.n.heroe(h).unwrap().plazas_recordadas[&(a as u32)], recuerdo);
    let bytes = e.n.estado().serializar();
    let vuelta = nucleo::Estado::deserializar(&bytes, &Reglas::default(), &e.n.estado().mundo_hash).unwrap();
    assert_eq!(vuelta.militar.heroes[&h].plazas_recordadas[&(a as u32)], recuerdo);
    // La siguiente salida la sustituye.
    e.ok("ana", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: h });
    e.ok("ana", Comando::SalirAlMundo { asentamiento: a, heroe: h, escuadras: vec![], carga: vec![] });
    assert_eq!(e.n.heroe(h).unwrap().plazas_recordadas[&(a as u32)].almacen[&Madera], e.n.economia(a).unwrap().almacen.visible(Madera));
    // Salir como ejército también la toma (decisión del autor del 2026-10-27).
    e.ok("ana", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: h });
    e.dar(a, Madera, 3_000);
    e.avanzar(MIN);
    let t = e.n.ahora();
    e.ok("ana", Comando::MovilizarEjercito { asentamiento: a, heroe: h, escuadras: vec![milicia], carga: vec![], politica: Politica::Aceptar, caravana_fundacion: None });
    let recuerdo = &e.n.heroe(h).unwrap().plazas_recordadas[&(a as u32)];
    assert_eq!((recuerdo.visto_en, recuerdo.almacen[&Madera]), (t, e.n.economia(a).unwrap().almacen.visible(Madera)));
}

#[test]
fn salir_sin_tropa_es_legitimo_y_la_escuadra_en_la_guarnicion_no_sale() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let (h, milicia) = e.con_milicia("ana");
    let a = e.a;
    let lenadores = e.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::Lenadores }).creadas[0];
    e.con_barracon();
    let salir = |escuadras: &[u64]| Comando::SalirAlMundo { asentamiento: a, heroe: h, escuadras: escuadras.to_vec(), carga: vec![] };
    e.ok("ana", Comando::AsignarGuarnicion { heroe: h, escuadra: lenadores });
    assert_eq!(e.rechazada("ana", salir(&[lenadores])), rechazos::ESCUADRA_EN_LA_GUARNICION);
    let col = e.ok("ana", salir(&[])).creadas[0];
    // Un héroe solo: el viajero, más rápido que cualquier tropa, y ve a 15.
    assert_eq!(e.n.velocidad_de_ejercito(col), Some(66_000 / 60));
    assert_eq!(e.n.alcance_de_vista(col), Some(15_000));
    assert_eq!(e.n.consumo_de_columna(col), Some(racion::racion_columna(&mr(), 0, 1, Marcha::Estacionada)));
    assert_eq!(e.escuadra(h, milicia).contenedor, Contenedor::Campamento, "lo que no se llevó se queda en el campamento");
}

#[test]
fn movilizar_exige_una_escuadra_y_el_ejercito_nace_quieto_con_su_politica() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let (h, milicia) = e.con_milicia("ana");
    let a = e.a;
    let movilizar = |escuadras: &[u64], politica| Comando::MovilizarEjercito { asentamiento: a, heroe: h, escuadras: escuadras.to_vec(), carga: vec![], politica, caravana_fundacion: None };
    assert_eq!(e.rechazada("ana", movilizar(&[], Politica::Aceptar)), rechazos::SALIDA_SIN_ESCUADRAS, "movilizar no sale con las manos vacías");
    let col = e.ok("ana", movilizar(&[milicia], Politica::Preguntar)).creadas[0];
    let c = &e.ejercito(col).columna;
    assert_eq!((c.tipo, c.politica, c.estado, c.lider), (TipoColumna::Ejercito, Politica::Preguntar, Marcha::Estacionada, h));
    assert_eq!(e.ejercito(col).posicion, e.plaza(), "sale quieto en la puerta: su Líder lo dirige con marcharA");
    assert_eq!(hechos(&e.n, "ejercito.movilizado").len(), 1);
}

// ───────────────────────────── 2. Comer, pasar hambre y desertar ─────────────────────────────

#[test]
fn el_hambre_de_una_columna_baja_la_moral_y_la_tropa_deserta_en_su_fase_hasta_que_vuelve_la_racion() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (h, ids) = e.con_prestamo("ana", 0);
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: h, recurso: Trigo, cantidad: 100_000 });
    let t0 = e.n.ahora();
    let salir = Comando::SalirDelCampamento { campamento: 0, heroe: h, escuadras: ids.clone(), carga: vec![(Trigo, 100_000)], politica: Politica::Rechazar, caravana_fundacion: None };
    let col = e.ok("ana", salir).creadas[0];
    // La ración gratis del campamento llena los víveres y estampa la ración; la carga sale del almacén personal.
    assert_eq!(e.viveres(col), 350_000);
    assert_eq!(e.n.heroe(h).unwrap().heroe.racion_en, Some(t0));
    assert!(e.n.heroe(h).unwrap().almacen_personal.is_empty());
    assert_eq!(e.ejercito(col).columna.carga[&Trigo], 100_000);
    // 60 soldados y un héroe, acampados: (60 × 0,15 + 0,5) × ¼ × ⅒.
    let consumo = racion::racion_columna(&mr(), 60, 1, Marcha::Estacionada);
    assert_eq!(consumo, 238);
    assert_eq!(e.n.consumo_de_columna(col), Some(consumo));
    let cero = e.ejercito(col).columna.viveres.instante_de_valor(0, t0).unwrap();

    // Con ración la moral sube y no se programa ninguna deserción.
    e.n.avanzar_hasta(cero - 1);
    assert!(hechos(&e.n, "ejercito.sin_viveres").is_empty());
    assert!(e.n.estado().cola.volcar().iter().all(|ev| !matches!(ev.carga, Carga::Militar(SucesoMilitar::Desercion))));
    assert_eq!(e.escuadra(h, ids[0]).moral.por_minuto, mr().militar.regeneracion_moral);
    // En el instante exacto en que los víveres llegan a 0 la tropa pasa hambre: la moral baja 20 por minuto y llega a 0 a los 5 minutos.
    e.n.avanzar_hasta(cero);
    assert_eq!(hechos(&e.n, "ejercito.sin_viveres").len(), 1);
    assert_eq!(e.escuadra(h, ids[0]).moral.por_minuto, -mr().militar.degradacion_moral);
    let m0 = cero + 5 * MIN;
    let desercion = |e: &Escenario| e.n.estado().cola.volcar().iter().filter(|ev| e.n.estado().generaciones.es_vigente(ev.clave)).find(|ev| matches!(ev.carga, Carga::Militar(SucesoMilitar::Desercion))).map(|ev| ev.instante);
    assert_eq!(desercion(&e), Some(m0), "una sola cita: la próxima deserción de cualquiera de sus escuadras");
    e.n.avanzar_hasta(m0 - 1);
    assert_eq!((e.escuadra(h, ids[0]).cantidad, e.escuadra(h, ids[1]).cantidad), (30, 30));
    // Al llegar la moral a 0 desertan ceil(5 %) de 30 = 2 de cada escuadra, y otros 2 cada minuto.
    e.n.avanzar_hasta(m0);
    assert_eq!((e.escuadra(h, ids[0]).cantidad, e.escuadra(h, ids[1]).cantidad), (28, 28));
    assert_eq!(desercion(&e), Some(m0 + MIN));
    e.n.avanzar_hasta(m0 + MIN);
    assert_eq!((e.escuadra(h, ids[0]).cantidad, e.escuadra(h, ids[1]).cantidad), (26, 26));
    let bajas = hechos(&e.n, "tropas.desercion");
    assert_eq!((bajas.len(), dato(&bajas[0], "desertores"), dato(&bajas[0], "heroe")), (4, 2, h as i64));
    assert_eq!(e.n.soldados_de_columna(col), Some(52));
    assert_eq!(e.n.consumo_de_columna(col), Some(racion::racion_columna(&mr(), 52, 1, Marcha::Estacionada)), "el consumo sigue a la tropa que queda");

    // Pasar trigo del carro a los víveres devuelve la ración: la moral sube y la deserción ya no ocurre.
    e.n.avanzar_hasta(m0 + MIN + 30_000);
    assert_eq!(e.rechazada("ana", Comando::PasarAViveres { heroe: h, cantidad: 0 }), rechazos::CANTIDAD_NO_POSITIVA);
    e.ok("ana", Comando::PasarAViveres { heroe: h, cantidad: 100_000 });
    assert_eq!(e.escuadra(h, ids[0]).moral.por_minuto, mr().militar.regeneracion_moral);
    assert!(desercion(&e).is_none());
    e.avanzar(10 * MIN);
    assert_eq!((e.escuadra(h, ids[0]).cantidad, e.escuadra(h, ids[1]).cantidad), (26, 26));
    assert!(e.escuadra(h, ids[0]).moral_en(e.n.ahora()) > 0);
}

#[test]
fn marchar_come_un_cuarto_de_racion_la_velocidad_es_la_de_la_tropa_mas_lenta_y_al_llegar_acampa() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (h, ids) = e.con_prestamo("ana", 0);
    let col = e.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: h, escuadras: ids.clone(), carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None }).creadas[0];
    let (lento, granjeros) = (mr().tropas.tropa(Tropa::Mineros).velocidad.min(mr().tropas.tropa(Tropa::Granjeros).velocidad), mr().tropas.tropa(Tropa::Granjeros).velocidad);
    assert_eq!(e.n.velocidad_de_ejercito(col), Some(lento / 60));
    assert!(granjeros >= lento);
    let destino = e.plaza();
    assert_eq!(e.rechazada("beto", Comando::MarcharA { ejercito: col, destino }), rechazos::NO_AUTORIZADO, "solo su Líder la dirige");
    e.ok("ana", Comando::MarcharA { ejercito: col, destino });
    assert_eq!(e.n.consumo_de_columna(col), Some(racion::racion_columna(&mr(), 60, 1, Marcha::Marchando)));
    assert!(e.ejercito(col).marchando());
    let fin = e.ejercito(col).plan.fin().unwrap();
    e.n.avanzar_hasta(fin);
    assert!(!e.ejercito(col).marchando());
    assert_eq!(e.ejercito(col).posicion, destino);
    assert_eq!(e.n.consumo_de_columna(col), Some(racion::racion_columna(&mr(), 60, 1, Marcha::Estacionada)), "al llegar acampa y come una décima parte");
    // Estacionar a mitad de camino es del Líder y deja la columna donde está.
    e.ok("ana", Comando::MarcharA { ejercito: col, destino: campamento(&m.0, 0) });
    e.avanzar(30_000);
    assert_eq!(e.rechazada("beto", Comando::Estacionar { ejercito: col }), rechazos::NO_AUTORIZADO);
    e.ok("ana", Comando::Estacionar { ejercito: col });
    assert!(!e.ejercito(col).marchando());
}

#[test]
fn la_marcha_forzada_va_un_cincuenta_por_ciento_mas_deprisa_come_el_doble_solo_marchando_y_va_en_la_foto() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (h, ids) = e.con_prestamo("ana", 0);
    let col = e.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: h, escuadras: ids, carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None }).creadas[0];
    let normal = e.n.velocidad_de_ejercito(col).unwrap();
    let forzada = |activa| Comando::MarchaForzada { ejercito: col, activa };
    assert_eq!(e.rechazada("beto", forzada(true)), rechazos::NO_AUTORIZADO, "solo su Líder la pone");
    assert_eq!(e.rechazada("ana", Comando::MarchaForzada { ejercito: 999_999, activa: true }), rechazos::EJERCITO_DESCONOCIDO);
    assert_eq!(e.ord("ana", forzada(false)).resultado, Resultado::SinCambios, "quitarla sin tenerla no cambia nada");
    // Parada, con la marcha forzada puesta: va más deprisa cuando ande, pero come lo de siempre.
    let v = e.ejercito(col).version;
    e.ok("ana", forzada(true));
    assert!(e.ejercito(col).columna.forzada);
    assert!(e.ejercito(col).version > v, "la columna cambia de versión");
    assert_eq!(e.ord("ana", forzada(true)).resultado, Resultado::SinCambios, "ponerla dos veces no cambia nada");
    let factor = mr().movimiento.factor_velocidad_forzada;
    assert_eq!(factor, 1_500_000, "factorVelocidadMarchaForzada es 1,5");
    assert_eq!(mr().movimiento.factor_consumo_forzada, 2_000_000, "factorConsumoMarchaForzada es 2");
    assert_eq!(e.n.velocidad_de_ejercito(col), Some(normal * 60 * factor / 1_000_000 / 60));
    assert_eq!(e.n.consumo_de_columna(col), Some(racion::racion_columna(&mr(), 60, 1, Marcha::Estacionada)), "parada come lo de siempre");
    assert!(nucleo::proyeccion::propios::ejercito(&e.n, e.ejercito(col)).marcha_forzada, "su dueño la ve");
    // Marchando come el doble.
    let destino = e.plaza();
    e.ok("ana", Comando::MarcharA { ejercito: col, destino });
    assert_eq!(e.n.consumo_de_columna(col), Some(racion::racion_columna(&mr(), 60, 1, Marcha::Marchando) * 2));
    assert_eq!(e.n.consumo_de_columna(col), Some(racion::racion_columna_forzada(&mr(), 60, 1, Marcha::Marchando, true)));
    // Quitarla a mitad de camino retrasa la llegada y vuelve a la ración normal; ponerla otra vez la adelanta.
    e.avanzar(30_000);
    let fin_forzada = e.ejercito(col).plan.fin().unwrap();
    e.ok("ana", forzada(false));
    let fin_normal = e.ejercito(col).plan.fin().unwrap();
    assert!(fin_normal > fin_forzada, "sin forzar llega más tarde: {fin_normal} vs {fin_forzada}");
    assert_eq!(e.n.consumo_de_columna(col), Some(racion::racion_columna(&mr(), 60, 1, Marcha::Marchando)));
    assert!(!nucleo::proyeccion::propios::ejercito(&e.n, e.ejercito(col)).marcha_forzada);
    e.ok("ana", forzada(true));
    assert!(e.ejercito(col).plan.fin().unwrap() < fin_normal);
    // Va en la foto.
    let bytes = e.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN49");
    let vuelta = nucleo::Estado::deserializar(&bytes, &Reglas::default(), &e.n.estado().mundo_hash).unwrap();
    assert!(vuelta.ejercitos[&col].columna.forzada, "la marcha forzada va en la foto");
    assert_eq!(vuelta.serializar(), bytes);
    // Al llegar acampa con la marcha forzada puesta y come lo de siempre.
    let fin = e.ejercito(col).plan.fin().unwrap();
    e.n.avanzar_hasta(fin);
    assert!(!e.ejercito(col).marchando());
    assert!(e.ejercito(col).columna.forzada, "sigue puesta hasta que se quita");
    assert_eq!(e.n.consumo_de_columna(col), Some(racion::racion_columna(&mr(), 60, 1, Marcha::Estacionada)));
}

// ───────────────────────────── 3. Entrar en un lugar ─────────────────────────────

#[test]
fn entrar_en_la_propia_residencia_deja_el_carro_con_el_heroe_y_en_otra_plaza_la_columna_entra_con_el_visitante() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let (h, milicia) = e.con_milicia("ana");
    let a = e.a;
    let madera0 = e.almacen(Madera);
    let col = e.ok("ana", Comando::SalirAlMundo { asentamiento: a, heroe: h, escuadras: vec![milicia], carga: vec![(Madera, 40_000)] }).creadas[0];
    let viveres = e.viveres(col);
    let entrar = Comando::EntrarEnAsentamiento { asentamiento: a, heroe: h };
    // Entrar en la propia residencia: la columna se deshace dentro, la tropa vuelve al campamento y el carro NO se vacía: se queda con el héroe (Entrar con todo).
    e.ok("ana", entrar.clone());
    assert!(e.n.ejercito(col).is_none(), "la columna desaparece");
    let hn = e.n.heroe(h).unwrap();
    assert_eq!((hn.columna, hn.heroe.presencia.ubicacion, hn.heroe.viveres), (None, Ubicacion::Asentamiento(a as u32), viveres), "los víveres siguen con el héroe");
    assert_eq!(hn.carro[&Madera], 40_000, "el carro entra con él");
    assert_eq!(e.escuadra(h, milicia).contenedor, Contenedor::Campamento);
    assert_eq!(e.almacen(Madera), madera0 - 40_000, "el almacén de la plaza no recibe el carro (antes se vaciaba solo)");
    assert_eq!(hechos(&e.n, "jugador.entra_en_asentamiento").len(), 1);
    assert_eq!(e.rechazada("ana", entrar.clone()), rechazos::HEROE_SIN_COLUMNA, "dentro de la plaza ya no va en ninguna columna");

    // Volver a salir: los víveres que lleva el héroe y el carro que trajo pasan a la columna nueva.
    let col2 = e.ok("ana", Comando::SalirAlMundo { asentamiento: a, heroe: h, escuadras: vec![], carga: vec![] }).creadas[0];
    assert!(e.viveres(col2) >= viveres.min(350_000));
    assert_eq!((e.ejercito(col2).columna.carga[&Madera], e.n.heroe(h).unwrap().carro.is_empty()), (40_000, true), "sale con el carro que trajo");
    // Entrar de nuevo y vaciarlo con la orden explícita: el almacén personal lo recibe.
    e.ok("ana", entrar.clone());
    e.ok("ana", Comando::VaciarCarro { heroe: h });
    let hn = e.n.heroe(h).unwrap();
    assert_eq!((hn.almacen_personal[&Madera], hn.carro.is_empty()), (40_000, true));
    assert_eq!(e.ord("ana", Comando::VaciarCarro { heroe: h }).resultado, Resultado::SinCambios, "con el carro vacío no hay nada que hacer");
    let col2 = e.ok("ana", Comando::SalirAlMundo { asentamiento: a, heroe: h, escuadras: vec![], carga: vec![] }).creadas[0];
    assert!(e.ejercito(col2).columna.carga.is_empty());

    // De visita: un ciudadano de la misma Facción que reside en un campamento entra con su columna, que queda dentro.
    let (b, ids) = e.con_prestamo("beto", 0);
    let cb = e.ok("beto", Comando::SalirDelCampamento { campamento: 0, heroe: b, escuadras: ids.clone(), carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None }).creadas[0];
    let destino = e.plaza();
    assert_eq!(e.rechazada("beto", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: b }), rechazos::ASENTAMIENTO_FUERA_DE_LA_PUERTA, "hay que estar a la puerta");
    e.ok("beto", Comando::MarcharA { ejercito: cb, destino });
    let fin = e.ejercito(cb).plan.fin().unwrap();
    e.n.avanzar_hasta(fin);
    // Un extranjero (Facción 2): la puerta por defecto está cerrada a neutrales y enemigos (Q1, 5E), así que no entra.
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: b, faccion: Some(2) });
    assert_eq!(e.rechazada("beto", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: b }), rechazos::PUERTA_CERRADA);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: b, faccion: Some(1) });
    e.ok("beto", Comando::EntrarEnAsentamiento { asentamiento: a, heroe: b });
    let hb = e.n.heroe(b).unwrap();
    assert_eq!((hb.columna, hb.heroe.presencia.ubicacion), (Some(cb), Ubicacion::Asentamiento(a as u32)), "su columna sigue siendo suya: entra con él, dentro, fuera del mapa");
    assert!(e.n.ejercito(cb).is_some() && !e.n.en_el_mapa(cb));
    assert_eq!(e.rechazada("beto", Comando::MarcharA { ejercito: cb, destino: campamento(&m.0, 0) }), rechazos::HEROE_DENTRO_DE_UN_LUGAR, "dentro de un lugar no se dirige la columna");
    assert_eq!(e.escuadra(b, ids[0]).contenedor, Contenedor::Ejercito(cb), "la tropa no se mueve de su columna");
    // Salir con todo lo que entró: desde la plaza ajena, sin elegir nada; desde la propia residencia no (allí se sale al mundo).
    assert_eq!(e.rechazada("ana", Comando::SalirDeAsentamiento { asentamiento: a, heroe: h }), rechazos::SALIDA_DESDE_TU_RESIDENCIA, "desde la propia residencia se sale al mundo");
    e.ok("beto", Comando::SalirDeAsentamiento { asentamiento: a, heroe: b });
    assert_eq!(e.n.heroe(b).unwrap().heroe.presencia.ubicacion, Ubicacion::Columna(cb));
    assert_eq!(e.rechazada("beto", Comando::SalirDeAsentamiento { asentamiento: a, heroe: b }), rechazos::HEROE_NO_ESTA_EN_LA_PLAZA);
}

// ───────────────────────────── 4. Salir de un campamento y entrar en él ─────────────────────────────

#[test]
fn salir_del_campamento_da_la_racion_gratis_cada_media_hora_y_entrar_en_el_deja_el_carro_con_el_heroe() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (h, ids) = e.con_prestamo("ana", 0);
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: h, recurso: Oro, cantidad: 50_000 });
    let salir = |escuadras: &[u64], carga: &[(Recurso, i64)], politica| Comando::SalirDelCampamento { campamento: 0, heroe: h, escuadras: escuadras.to_vec(), carga: carga.to_vec(), politica, caravana_fundacion: None };
    let (v, hash) = (e.n.version(), e.n.hash_estado());
    assert_eq!(e.rechazada("beto", salir(&[], &[], Politica::Rechazar)), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", Comando::SalirDelCampamento { campamento: 9_999, heroe: h, escuadras: vec![], carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None }), rechazos::CAMPAMENTO_DESCONOCIDO);
    assert_eq!(e.rechazada("ana", salir(&[], &[(Oro, 51_000)], Politica::Rechazar)), rechazos::ALMACEN_SIN_EXISTENCIAS, "solo se carga lo que hay en el almacén personal");
    assert_eq!(e.rechazada("ana", salir(&[], &[(Oro, 600_000)], Politica::Rechazar)), rechazos::CARRO_LLENO);
    assert_eq!(e.rechazada("ana", salir(&[ids[0], ids[0]], &[], Politica::Rechazar)), rechazos::ESCUADRA_REPETIDA);
    assert_eq!((e.n.version(), e.n.hash_estado()), (v, hash));
    let t = e.n.ahora();
    let col = e.ok("ana", salir(&ids, &[(Oro, 20_000)], Politica::Aceptar)).creadas[0];
    // Con política distinta de rechazar sale como ejército; el almacén personal pierde lo cargado.
    assert_eq!((e.ejercito(col).columna.tipo, e.ejercito(col).columna.politica), (TipoColumna::Ejercito, Politica::Aceptar));
    assert_eq!(e.n.heroe(h).unwrap().almacen_personal[&Oro], 30_000);
    assert_eq!((e.viveres(col), e.ejercito(col).posicion), (350_000, campamento(&m.0, 0)));
    assert_eq!(hechos(&e.n, "jugador.sale_de_campamento").len(), 1);

    // Entrar de nuevo en su campamento: la tropa vuelve a él y los víveres siguen con el héroe, pero el carro NO se vuelca en el almacén personal: se queda con él
    // (Entrar con todo). `vaciarCarro` lo pasa al almacén.
    e.ok("ana", Comando::EntrarEnCampamento { campamento: 0, heroe: h });
    assert!(e.n.ejercito(col).is_none());
    let hn = e.n.heroe(h).unwrap();
    assert_eq!((hn.almacen_personal[&Oro], hn.carro[&Oro], hn.heroe.viveres, hn.heroe.presencia.ubicacion, hn.columna), (30_000, 20_000, 350_000, Ubicacion::Mercenarios(0), None));
    assert_eq!(e.escuadra(h, ids[0]).contenedor, Contenedor::Campamento);
    // La ración gratis es cada 30 minutos y no se acumula: si no ha pasado el plazo, sale con los víveres que lleva (los del héroe) y con el carro que trajo.
    let col2 = e.ok("ana", salir(&ids, &[], Politica::Rechazar)).creadas[0];
    assert_eq!((e.viveres(col2), e.ejercito(col2).columna.carga[&Oro]), (350_000, 20_000), "los víveres que traía el héroe y el carro con el que entró");
    assert_eq!(e.n.heroe(h).unwrap().heroe.racion_en, Some(t), "la ración no se renovó: no pasaron 30 minutos");

    // Lo que no cabe en el almacén personal se queda en el carro, que sale con el héroe: el almacén queda a 10 000 de su tope (1 000 000) y el carro lleva 20 000.
    e.ok("ana", Comando::EntrarEnCampamento { campamento: 0, heroe: h });
    e.ok("admin", Comando::DarAlmacenPersonal { heroe: h, recurso: Oro, cantidad: 960_000 });
    assert_eq!(e.n.heroe(h).unwrap().almacen_personal[&Oro], 990_000);
    e.ok("ana", Comando::VaciarCarro { heroe: h });
    let h0 = e.n.heroe(h).unwrap();
    assert_eq!((h0.almacen_personal[&Oro], h0.carro[&Oro]), (1_000_000, 10_000), "lleno: lo que no cabe se queda en el carro");
    assert_eq!(e.rechazada("ana", Comando::VaciarCarro { heroe: h }), rechazos::ALMACEN_PERSONAL_LLENO);
    let col3 = e.ok("ana", salir(&ids, &[], Politica::Rechazar)).creadas[0];
    assert_eq!((e.ejercito(col3).columna.escuadras.clone(), e.ejercito(col3).columna.carga[&Oro]), (ids.clone(), 10_000), "se lleva lo que no cupo");
    assert_eq!(e.n.heroe(h).unwrap().heroe.presencia.ubicacion, Ubicacion::Columna(col3));
}

// ───────────────────────────── 5. Unirse en campo, separarse y el mando ─────────────────────────────

/// Dos héroes de la Facción 1 a la puerta del campamento 0: «ana» al mando de un ejército (política dada) y «beto» con su columna personal.
fn ejercito_y_viajero(e: &mut Escenario, politica: Politica) -> ((u64, u64, Vec<u64>), (u64, u64, Vec<u64>)) {
    let (a, ia) = e.con_prestamo("ana", 0);
    let (b, ib) = e.con_prestamo("beto", 0);
    let ejercito = e.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: a, escuadras: ia.clone(), carga: vec![], politica, caravana_fundacion: None }).creadas[0];
    let personal = e.ok("beto", Comando::SalirDelCampamento { campamento: 0, heroe: b, escuadras: ib.clone(), carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None }).creadas[0];
    ((a, ejercito, ia), (b, personal, ib))
}

#[test]
fn unirse_en_campo_funde_la_columna_con_lo_que_lleva_y_cada_condicion_tiene_su_rechazo() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let ((a, ej, ia), (b, per, ib)) = ejercito_y_viajero(&mut e, Politica::Aceptar);
    let unirse = |heroe, ejercito| Comando::UnirseEnCampo { heroe, ejercito };
    assert_eq!(e.rechazada("ana", unirse(b, ej)), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("beto", unirse(b, 9_999)), rechazos::EJERCITO_DESCONOCIDO);
    assert_eq!(e.rechazada("beto", unirse(b, per)), rechazos::COLUMNA_YA_VAS_EN_ELLA);
    // Una columna de otra Facción tampoco se une, ni siquiera siendo aliada.
    let (c, ic) = e.con_prestamo("carla", 0);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: c, faccion: Some(2) });
    e.ok("carla", Comando::SalirDelCampamento { campamento: 0, heroe: c, escuadras: ic, carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None });
    assert_eq!(e.rechazada("carla", unirse(c, ej)), rechazos::COLUMNA_OTRA_FACCION);
    // Una columna personal no recoge a nadie, tampoco a un ejército.
    assert_eq!(e.rechazada("ana", unirse(a, per)), rechazos::COLUMNA_NO_ADMITE, "una personal rechaza a todos");
    let (v0, v1) = (e.viveres(ej), e.viveres(per));
    let antes_v = e.n.version();
    e.ok("beto", unirse(b, ej));
    assert!(e.n.ejercito(per).is_none(), "la columna que se une desaparece");
    let c = &e.ejercito(ej).columna;
    assert_eq!((c.participantes.iter().map(|p| p.heroe).collect::<Vec<_>>(), c.escuadras.clone()), (vec![a, b], [ia.clone(), ib.clone()].concat()));
    assert_eq!(c.viveres.maximo, Some(700_000), "la capacidad es la suma de la de sus héroes");
    assert_eq!(e.viveres(ej), v0 + v1);
    assert_eq!(e.n.consumo_de_columna(ej), Some(racion::racion_columna(&mr(), 120, 2, Marcha::Estacionada)));
    assert_eq!(e.n.capacidad_de_carga(ej), Some(1_000_000), "un carro por héroe");
    let hb = e.n.heroe(b).unwrap();
    assert_eq!((hb.columna, hb.heroe.presencia.ubicacion), (Some(ej), Ubicacion::Columna(ej)));
    assert_eq!(e.escuadra(b, ib[0]).contenedor, Contenedor::Ejercito(ej));
    assert!(e.n.version() > antes_v);
    assert_eq!(hechos(&e.n, "columna.union_en_campo").len(), 1);
    // Marcha a la velocidad del más lento y solo la dirige su Líder.
    assert_eq!(e.rechazada("beto", Comando::MarcharA { ejercito: ej, destino: e.plaza() }), rechazos::NO_AUTORIZADO);
}

#[test]
fn con_preguntar_la_peticion_vive_diez_segundos_y_el_silencio_es_un_no() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let ((a, ej, _), (b, per, _)) = ejercito_y_viajero(&mut e, Politica::Preguntar);
    let t = e.n.ahora();
    e.ok("beto", Comando::UnirseEnCampo { heroe: b, ejercito: ej });
    assert!(e.n.ejercito(per).is_some(), "preguntar no une todavía");
    assert_eq!(e.ejercito(ej).columna.peticiones.len(), 1);
    assert_eq!(e.ejercito(ej).columna.peticiones[0].expira_en, t + 10_000);
    assert_eq!(hechos(&e.n, "columna.union_pedida").len(), 1);
    // Solo el Líder contesta.
    assert_eq!(e.rechazada("beto", Comando::ResponderPeticionDeUnion { heroe: b, ejercito: ej, solicitante: b, aceptar: true }), rechazos::COLUMNA_SOLO_EL_LIDER);
    assert_eq!(e.rechazada("ana", Comando::ResponderPeticionDeUnion { heroe: a, ejercito: ej, solicitante: 99, aceptar: true }), rechazos::COLUMNA_NO_HAY_PETICION);
    // Rechazar la retira y avisa; pedir otra vez la renueva.
    e.ok("ana", Comando::ResponderPeticionDeUnion { heroe: a, ejercito: ej, solicitante: b, aceptar: false });
    assert!(e.ejercito(ej).columna.peticiones.is_empty());
    assert_eq!(hechos(&e.n, "columna.union_rechazada").len(), 1);
    e.ok("beto", Comando::UnirseEnCampo { heroe: b, ejercito: ej });
    // Caduca a los diez segundos, por un evento exacto.
    let t2 = e.n.ahora();
    e.n.avanzar_hasta(t2 + 9_999);
    assert_eq!(e.ejercito(ej).columna.peticiones.len(), 1);
    e.n.avanzar_hasta(t2 + 10_000);
    assert!(e.ejercito(ej).columna.peticiones.is_empty());
    assert_eq!(hechos(&e.n, "columna.peticion_caducada").len(), 1);
    assert_eq!(e.rechazada("ana", Comando::ResponderPeticionDeUnion { heroe: a, ejercito: ej, solicitante: b, aceptar: true }), rechazos::COLUMNA_NO_HAY_PETICION);
    // Contestar a tiempo y aceptar une (la geometría se mira al aceptar).
    e.ok("beto", Comando::UnirseEnCampo { heroe: b, ejercito: ej });
    e.ok("ana", Comando::ResponderPeticionDeUnion { heroe: a, ejercito: ej, solicitante: b, aceptar: true });
    assert!(e.n.ejercito(per).is_none() && e.ejercito(ej).columna.participa(b));
}

#[test]
fn separarse_te_lleva_lo_tuyo_el_lider_cede_antes_y_el_mando_pasa_al_mas_antiguo() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let ((a, ej, _), (b, _, ib)) = ejercito_y_viajero(&mut e, Politica::Aceptar);
    e.ok("beto", Comando::UnirseEnCampo { heroe: b, ejercito: ej });
    e.avanzar(10 * MIN);
    // El Líder no se separa; el último tampoco; solo el Líder cede.
    assert_eq!(e.rechazada("ana", Comando::SepararseDelEjercito { heroe: a }), rechazos::COLUMNA_EL_LIDER_NO_PUEDE);
    assert_eq!(e.rechazada("beto", Comando::CederLiderazgo { heroe: b, sucesor: a }), rechazos::COLUMNA_SOLO_EL_LIDER);
    assert_eq!(e.rechazada("ana", Comando::CederLiderazgo { heroe: a, sucesor: a }), rechazos::COLUMNA_YA_ERES_EL_LIDER);
    assert_eq!(e.rechazada("ana", Comando::CederLiderazgo { heroe: a, sucesor: 99 }), rechazos::COLUMNA_SUCESOR_FUERA);
    let antes_viveres = e.viveres(ej);
    let ids = e.n.estado().siguiente_id;
    let nueva = e.ok("beto", Comando::SepararseDelEjercito { heroe: b }).creadas[0];
    assert_eq!(nueva, ids, "la columna nueva sale del contador común");
    let n = e.ejercito(nueva);
    assert_eq!((n.columna.tipo, n.columna.lider, n.columna.politica, n.posicion), (TipoColumna::Personal, b, Politica::Rechazar, e.ejercito(ej).posicion));
    assert_eq!(n.columna.escuadras, ib, "se lleva sus escuadras");
    assert_eq!(n.columna.viveres.maximo, Some(350_000));
    // Su parte igual de los víveres: víveres × su capacidad / la capacidad total, en unidades enteras hacia abajo (EE-3); el resto oculto se queda.
    assert_eq!(e.viveres(nueva), base::entero(antes_viveres / 2));
    assert_eq!(e.viveres(ej), antes_viveres - base::entero(antes_viveres / 2));
    assert_eq!(e.ejercito(ej).columna.viveres.maximo, Some(350_000));
    assert_eq!(e.n.consumo_de_columna(ej), Some(racion::racion_columna(&mr(), 60, 1, Marcha::Estacionada)));
    assert_eq!(e.ejercito(ej).columna.tipo, TipoColumna::Ejercito, "un ejército reducido a uno sigue siéndolo");
    assert_eq!(e.rechazada("ana", Comando::SepararseDelEjercito { heroe: a }), rechazos::COLUMNA_EL_LIDER_NO_PUEDE);
    assert_eq!(e.rechazada("beto", Comando::SepararseDelEjercito { heroe: b }), rechazos::COLUMNA_DE_UNA_PERSONAL_NO_TE_SEPARAS);
    assert_eq!(hechos(&e.n, "columna.separacion").len(), 1);
    // Volver a unirse y ceder el mando: entonces el Líder anterior sí puede irse.
    e.ok("beto", Comando::UnirseEnCampo { heroe: b, ejercito: ej });
    e.ok("ana", Comando::CederLiderazgo { heroe: a, sucesor: b });
    assert_eq!(e.ejercito(ej).columna.lider, b);
    assert_eq!(hechos(&e.n, "columna.liderazgo_cedido").len(), 1);
    e.ok("ana", Comando::SepararseDelEjercito { heroe: a });
}

// ───────────────────────────── 6. Formar un ejército en campo ─────────────────────────────

#[test]
fn organizar_un_ejercito_espera_tres_diez_minutos_y_si_no_llega_se_deshace() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, ia) = e.con_prestamo("ana", 0);
    let (b, ib) = e.con_prestamo("beto", 0);
    let (c, ic) = e.con_prestamo("carla", 0);
    let personal = |e: &mut Escenario, cuenta, h, ids: &[u64]| e.ok(cuenta, Comando::SalirDelCampamento { campamento: 0, heroe: h, escuadras: ids.to_vec(), carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None }).creadas[0];
    let ca = personal(&mut e, "ana", a, &ia);
    personal(&mut e, "beto", b, &ib);
    personal(&mut e, "carla", c, &ic);
    // Solo una personal sola organiza, con aceptar o preguntar; una formación no se mueve.
    assert_eq!(e.rechazada("ana", Comando::OrganizarEjercito { heroe: a, politica: Politica::Rechazar }), rechazos::COLUMNA_POLITICA_INVALIDA);
    let t = e.n.ahora();
    e.ok("ana", Comando::OrganizarEjercito { heroe: a, politica: Politica::Aceptar });
    assert_eq!(e.ejercito(ca).columna.formacion_expira_en, Some(t + 10 * MIN));
    assert_eq!(e.rechazada("ana", Comando::OrganizarEjercito { heroe: a, politica: Politica::Aceptar }), rechazos::COLUMNA_YA_ORGANIZANDO);
    assert_eq!(e.rechazada("ana", Comando::MarcharA { ejercito: ca, destino: e.plaza() }), rechazos::COLUMNA_EN_FORMACION);
    // Con dos sigue siendo una formación (para el combate, personal); con tres es un ejército y se cancela el plazo.
    e.ok("beto", Comando::UnirseEnCampo { heroe: b, ejercito: ca });
    assert!(e.ejercito(ca).columna.es_formacion() && e.ejercito(ca).columna.tipo == TipoColumna::Personal);
    e.ok("carla", Comando::UnirseEnCampo { heroe: c, ejercito: ca });
    assert_eq!((e.ejercito(ca).columna.tipo, e.ejercito(ca).columna.es_formacion()), (TipoColumna::Ejercito, false));
    assert_eq!(hechos(&e.n, "columna.formacion_completada").len(), 1);
    e.avanzar(30 * MIN);
    assert_eq!(e.ejercito(ca).columna.participantes.len(), 3, "el plazo ya no deshace nada");
    assert_eq!(e.ejercito(ca).columna.tipo, TipoColumna::Ejercito);

    // Otra formación que no llega a tres se deshace sola a los diez minutos: cada uno vuelve a su columna personal con lo suyo.
    let (d, id_) = e.con_prestamo("dana", 0);
    let (f, if_) = e.con_prestamo("fede", 0);
    let cd = personal(&mut e, "dana", d, &id_);
    let cf = personal(&mut e, "fede", f, &if_);
    let t = e.n.ahora();
    e.ok("dana", Comando::OrganizarEjercito { heroe: d, politica: Politica::Preguntar });
    e.ok("fede", Comando::UnirseEnCampo { heroe: f, ejercito: cd });
    assert!(e.n.ejercito(cf).is_some(), "preguntar: aún no está dentro");
    e.ok("dana", Comando::ResponderPeticionDeUnion { heroe: d, ejercito: cd, solicitante: f, aceptar: true });
    assert_eq!(e.ejercito(cd).columna.participantes.len(), 2);
    e.n.avanzar_hasta(t + 10 * MIN - 1);
    assert!(e.ejercito(cd).columna.es_formacion());
    e.n.avanzar_hasta(t + 10 * MIN);
    assert!(!e.ejercito(cd).columna.es_formacion());
    assert_eq!((e.ejercito(cd).columna.tipo, e.ejercito(cd).columna.participantes.len()), (TipoColumna::Personal, 1));
    let suelta = e.n.heroe(f).unwrap().columna.unwrap();
    assert_ne!(suelta, cd);
    assert_eq!(e.ejercito(suelta).columna.escuadras, if_, "se lleva lo suyo");
    assert_eq!(hechos(&e.n, "columna.formacion_disuelta").len(), 1);
    // El Líder también puede cancelar la suya antes de tiempo.
    let (g, ig) = e.con_prestamo("gema", 0);
    let cg = personal(&mut e, "gema", g, &ig);
    e.ok("gema", Comando::OrganizarEjercito { heroe: g, politica: Politica::Aceptar });
    assert_eq!(e.rechazada("beto", Comando::CancelarFormacion { heroe: b }), rechazos::COLUMNA_NO_ES_UNA_FORMACION);
    e.ok("gema", Comando::CancelarFormacion { heroe: g });
    assert!(!e.ejercito(cg).columna.es_formacion());
    e.avanzar(30 * MIN);
}

// ───────────────────────────── 7. Desconectarse con la columna ─────────────────────────────

#[test]
fn quien_se_desconecta_en_campo_se_lleva_lo_suyo_el_mando_pasa_al_mas_antiguo_y_al_volver_nace_su_columna() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let ((a, ej, ia), (b, _, _)) = ejercito_y_viajero(&mut e, Politica::Aceptar);
    e.ok("beto", Comando::UnirseEnCampo { heroe: b, ejercito: ej });
    e.avanzar(5 * MIN);
    let punto = e.ejercito(ej).posicion;
    // Conectarse a tiempo cancela la salida.
    e.ok("ana", Comando::Desconectarse { heroe: a });
    e.avanzar(60_000);
    e.ok("ana", Comando::Conectarse { heroe: a });
    e.avanzar(10 * MIN);
    assert!(hechos(&e.n, "heroe.sale_del_mundo").is_empty());
    let t = e.n.ahora();
    e.ok("ana", Comando::Desconectarse { heroe: a });
    e.n.avanzar_hasta(t + 150_000 - 1);
    assert!(!e.n.heroe(a).unwrap().heroe.presencia.fuera);
    e.n.avanzar_hasta(t + 150_000);
    // Sale con sus escuadras y su parte de los víveres; era el Líder, así que el mando pasa a quien queda.
    let hn = e.n.heroe(a).unwrap();
    assert!(hn.heroe.presencia.fuera && hn.columna.is_none());
    assert_eq!(hn.heroe.presencia.ubicacion, Ubicacion::Desconectado(punto));
    assert!(ia.iter().all(|&s| e.escuadra(a, s).contenedor == Contenedor::Fuera));
    assert!(hn.heroe.viveres > 0);
    let c = &e.ejercito(ej).columna;
    assert_eq!((c.lider, c.participantes.len(), c.tipo), (b, 1, TipoColumna::Ejercito));
    assert_eq!(c.viveres.maximo, Some(350_000));
    assert_eq!(hechos(&e.n, "columna.liderazgo_cedido").len(), 1);
    // Fuera del mundo solo se puede conectar.
    assert_eq!(e.rechazada("ana", Comando::Desconectarse { heroe: a }), rechazos::HEROE_FUERA_DEL_MUNDO);
    // Al conectarse reaparece en su punto con una columna personal nueva, quieta, con sus escuadras y sus víveres.
    let nueva = e.ok("ana", Comando::Conectarse { heroe: a }).creadas[0];
    let n = e.ejercito(nueva);
    assert_eq!((n.posicion, n.columna.tipo, n.columna.lider, n.columna.escuadras.clone()), (punto, TipoColumna::Personal, a, ia.clone()));
    assert_eq!(e.escuadra(a, ia[0]).contenedor, Contenedor::Ejercito(nueva));
    assert_eq!(e.n.heroe(a).unwrap().columna, Some(nueva));
    assert_eq!(e.n.heroe(a).unwrap().heroe.viveres, 0);

    // Si el que se va es el último, la columna se disuelve.
    e.ok("beto", Comando::Desconectarse { heroe: b });
    e.avanzar(150_000);
    assert!(e.n.ejercito(ej).is_none());
    assert!(e.n.heroe(b).unwrap().heroe.presencia.fuera);
}

// ───────────────────────────── 8. Guarnecer, entrar en el campamento y unirse desde la plaza ─────────────────────────────

#[test]
fn guarnecer_desarma_el_ejercito_en_la_puerta_el_residente_entra_con_su_carro_y_el_de_visita_con_su_columna_dentro() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let (a, milicia) = e.con_milicia("ana");
    let pl = e.a;
    let ej = e.ok("ana", Comando::MovilizarEjercito { asentamiento: pl, heroe: a, escuadras: vec![milicia], carga: vec![(Madera, 20_000)], politica: Politica::Aceptar, caravana_fundacion: None }).creadas[0];
    // «beto», de la misma Facción pero residente de un campamento, llega a la plaza y se une en campo.
    let (b, ib) = e.con_prestamo("beto", 0);
    let cb = e.ok("beto", Comando::SalirDelCampamento { campamento: 0, heroe: b, escuadras: ib.clone(), carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None }).creadas[0];
    e.ok("beto", Comando::MarcharA { ejercito: cb, destino: e.plaza() });
    let fin = e.ejercito(cb).plan.fin().unwrap();
    e.n.avanzar_hasta(fin);
    e.ok("beto", Comando::UnirseEnCampo { heroe: b, ejercito: ej });
    assert_eq!(e.ejercito(ej).columna.participantes.len(), 2);
    let madera0 = e.almacen(Madera);

    let guarnecer = |heroe| Comando::Guarnecer { asentamiento: pl, heroe };
    assert_eq!(e.rechazada("beto", guarnecer(b)), rechazos::COLUMNA_SOLO_EL_LIDER, "lo ordena su Líder");
    assert_eq!(e.rechazada("ana", Comando::Guarnecer { asentamiento: 9_999, heroe: a }), rechazos::ASENTAMIENTO_DESCONOCIDO);
    let (v, hash) = (e.n.version(), e.n.hash_estado());
    e.ok("ana", guarnecer(a));
    assert_ne!((v, hash), (e.n.version(), e.n.hash_estado()));
    // «ana» reside aquí: la tropa vuelve al campamento y la columna desaparece, pero el carro no se vacía: se queda con ella.
    let hn = e.n.heroe(a).unwrap();
    assert_eq!((hn.columna, hn.heroe.presencia.ubicacion), (None, Ubicacion::Asentamiento(pl as u32)));
    assert_eq!(e.escuadra(a, milicia).contenedor, Contenedor::Campamento);
    assert!(e.n.ejercito(ej).is_none());
    assert_eq!((e.almacen(Madera), e.n.heroe(a).unwrap().carro[&Madera]), (madera0, 10_000), "el carro se reparte entre los dos héroes y el de ana se queda con ella");
    // «beto» entra de visita: su columna personal queda dentro con su tropa.
    let hb = e.n.heroe(b).unwrap();
    let suya = hb.columna.expect("su columna sigue siendo suya");
    assert_eq!(e.ejercito(suya).columna.carga[&Madera], 10_000);
    assert_eq!(hb.heroe.presencia.ubicacion, Ubicacion::Asentamiento(pl as u32));
    assert_eq!((e.ejercito(suya).columna.tipo, e.ejercito(suya).columna.escuadras.clone()), (TipoColumna::Personal, ib));
    assert_eq!(hechos(&e.n, "ejercito.guarnece").len(), 1);
    assert!(!e.n.en_el_mapa(suya), "su columna está dentro, fuera del mapa");
    assert_eq!(e.rechazada("ana", guarnecer(a)), rechazos::HEROE_SIN_COLUMNA);

    // Una personal no guarnece: entra con `entrarEnAsentamiento`.
    e.ok("ana", Comando::SalirAlMundo { asentamiento: pl, heroe: a, escuadras: vec![], carga: vec![] });
    assert_eq!(e.rechazada("ana", guarnecer(a)), rechazos::COLUMNA_NO_ES_EJERCITO);
}

#[test]
fn un_ejercito_entra_entero_en_su_campamento_con_todo() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let ((a, ej, ia), (b, _, ib)) = ejercito_y_viajero(&mut e, Politica::Aceptar);
    e.ok("beto", Comando::UnirseEnCampo { heroe: b, ejercito: ej });
    let entrar = |heroe| Comando::EntrarEnCampamento { campamento: 0, heroe };
    assert_eq!(e.rechazada("beto", entrar(b)), rechazos::COLUMNA_SOLO_EL_LIDER);
    assert_eq!(e.rechazada("ana", Comando::EntrarEnCampamento { campamento: 9_999, heroe: a }), rechazos::CAMPAMENTO_DESCONOCIDO);
    e.ok("ana", entrar(a));
    assert!(e.n.ejercito(ej).is_none());
    for (h, ids) in [(a, &ia), (b, &ib)] {
        let hn = e.n.heroe(h).unwrap();
        assert_eq!((hn.columna, hn.heroe.presencia.ubicacion), (None, Ubicacion::Mercenarios(0)));
        assert!(ids.iter().all(|&s| e.escuadra(h, s).contenedor == Contenedor::Campamento));
    }
    assert_eq!(hechos(&e.n, "ejercito.entra_en_campamento").len(), 1);
}

#[test]
fn unirse_a_un_ejercito_desde_la_plaza_saca_tropa_fresca_y_la_politica_rechazar_cierra_la_puerta() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(96 * HORA);
    let (a, milicia) = e.con_milicia("ana");
    let (c, milicia_c) = e.con_milicia("carla");
    let pl = e.a;
    let ej = e.ok("ana", Comando::MovilizarEjercito { asentamiento: pl, heroe: a, escuadras: vec![milicia], carga: vec![], politica: Politica::Aceptar, caravana_fundacion: None }).creadas[0];
    let unirse = |heroe, escuadras: &[u64]| Comando::UnirseAEjercito { asentamiento: pl, heroe, ejercito: ej, escuadras: escuadras.to_vec(), carga: vec![] };
    assert_eq!(e.rechazada("beto", unirse(c, &[milicia_c])), rechazos::NO_AUTORIZADO);
    let (b, ib) = e.con_prestamo("beto", 0);
    assert_eq!(e.rechazada("beto", unirse(b, &ib)), rechazos::SALIDA_SOLO_DE_TU_RESIDENCIA, "solo un residente saca tropas de la plaza");
    assert_eq!(e.rechazada("carla", unirse(c, &[])), rechazos::SALIDA_SIN_ESCUADRAS);
    let viveres0 = e.viveres(ej);
    e.ok("carla", unirse(c, &[milicia_c]));
    let col = &e.ejercito(ej).columna;
    assert_eq!((col.participantes.iter().map(|p| p.heroe).collect::<Vec<_>>(), col.escuadras.clone()), (vec![a, c], vec![milicia, milicia_c]));
    assert_eq!(col.viveres.maximo, Some(700_000));
    assert!(e.viveres(ej) > viveres0, "el que se une trae sus víveres, llenados de la plaza");
    assert_eq!(e.escuadra(c, milicia_c).contenedor, Contenedor::Ejercito(ej));
    assert_eq!(e.n.heroe(c).unwrap().heroe.presencia.ubicacion, Ubicacion::Columna(ej));
    assert_eq!(hechos(&e.n, "ejercito.refuerzo").len(), 1);
    assert_eq!(e.n.consumo_de_columna(ej), Some(racion::racion_columna(&mr(), 50, 2, Marcha::Estacionada)));
    // Una columna personal no admite a nadie, tampoco desde la plaza.
    let (d, _) = e.con_milicia("dana");
    let sola = e.ok("dana", Comando::SalirAlMundo { asentamiento: pl, heroe: d, escuadras: vec![], carga: vec![] }).creadas[0];
    let (f, milicia_f) = e.con_milicia("fede");
    let a_ella = Comando::UnirseAEjercito { asentamiento: pl, heroe: f, ejercito: sola, escuadras: vec![milicia_f], carga: vec![] };
    assert_eq!(e.rechazada("fede", a_ella), rechazos::COLUMNA_NO_ES_EJERCITO);
}

// ───────────────────────────── 9. Repostar ─────────────────────────────

#[test]
fn repostar_llena_los_viveres_en_una_plaza_propia_sin_bajar_de_su_reserva_y_solo_narra_si_iba_por_debajo_de_la_mitad() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    e.avanzar(24 * HORA);
    let (a, milicia) = e.con_milicia("ana");
    let pl = e.a;
    let col = e.ok("ana", Comando::MovilizarEjercito { asentamiento: pl, heroe: a, escuadras: vec![milicia], carga: vec![], politica: Politica::Aceptar, caravana_fundacion: None }).creadas[0];
    let repostar = Comando::Repostar { heroe: a, asentamiento: pl };
    e.avanzar(5 * HORA);
    let llevaba = e.viveres(col);
    assert!(llevaba < 350_000 && llevaba > 175_000);
    assert_eq!(e.rechazada("beto", repostar.clone()), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", Comando::Repostar { heroe: a, asentamiento: 9_999 }), rechazos::ASENTAMIENTO_DESCONOCIDO);
    let trigo0 = e.almacen(Trigo);
    e.ok("ana", repostar.clone());
    // Del almacén sale trigo en unidades enteras (C3b): los víveres quedan a menos de una unidad del tope, y eso ya es estar llenos.
    assert!(e.viveres(col) > 349_000 && e.viveres(col) <= 350_000, "{}", e.viveres(col));
    assert_eq!(trigo0 - e.almacen(Trigo), e.viveres(col) - llevaba, "sale del almacén de la plaza");
    assert_eq!((trigo0 - e.almacen(Trigo)) % 1_000, 0, "en unidades enteras");
    assert!(hechos(&e.n, "ejercito.reabastecido").is_empty(), "el goteo no es noticia");
    assert_eq!(e.rechazada("ana", repostar), rechazos::COLUMNA_VIVERES_LLENOS);
    // Con casi nada, sí: una columna de administración con 50 de víveres.
    let seca = e.n.crear_ejercito(e.n.ahora(), "carla", 1, e.plaza(), 500, 50_000).unwrap();
    let lider = e.ejercito(seca).columna.lider;
    e.ok("carla", Comando::Repostar { heroe: lider, asentamiento: pl });
    assert_eq!(e.viveres(seca), 350_000);
    assert_eq!(hechos(&e.n, "ejercito.reabastecido").len(), 1);
}

#[test]
fn una_columna_personal_de_la_faccion_reposta_sola_en_la_zona_de_lo_que_sobra_y_algo_mas_rapido_de_lo_que_come() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (pl, centro) = (e.a, e.plaza());
    e.avanzar(24 * HORA);
    e.dar(pl, Trigo, 5_000_000);
    // Dos columnas personales con su milicia que salen llenas: una se queda a la puerta, en la zona; la otra marcha fuera de ella.
    let (a, milicia_a) = e.con_milicia("ana");
    let (b, milicia_b) = e.con_milicia("beto");
    let quieta = e.ok("ana", Comando::SalirAlMundo { asentamiento: pl, heroe: a, escuadras: vec![milicia_a], carga: vec![] }).creadas[0];
    let fuera = e.ok("beto", Comando::SalirAlMundo { asentamiento: pl, heroe: b, escuadras: vec![milicia_b], carga: vec![] }).creadas[0];
    let destino = comun::real::en_tierra(&m.0, centro, 150_000, 0);
    e.ok("beto", Comando::MarcharA { ejercito: fuera, destino });
    // Un ejército de la Facción en la zona no reposta solo (lo hace con la orden), aunque lleve poco.
    let ejercito = e.n.crear_ejercito(e.n.ahora(), "carla", 1, Punto::new(centro.x + 2_000, centro.y), 500, 50_000).unwrap();
    assert_eq!(e.ejercito(ejercito).columna.tipo, TipoColumna::Ejercito);
    let tope = e.ejercito(quieta).columna.viveres.maximo.unwrap();
    e.avanzar(10 * HORA);
    assert!(!e.n.en_zona(pl, e.ejercito(fuera).posicion_en(e.n.ahora())), "la otra acabó fuera de la zona");
    let come = -e.ejercito(quieta).columna.viveres.por_minuto;
    assert!(come * 600 > 50_000, "en diez horas se comería más de 50 unidades: {come}");
    // La de la zona recibe cada minuto 1,25 veces lo que come (en unidades enteras hacia arriba): sigue casi llena. La de fuera y el ejército, no.
    assert!(tope - e.viveres(quieta) < 2_000, "{} de {tope}", e.viveres(quieta));
    assert!(tope - e.viveres(fuera) > 40_000, "{} de {tope}", e.viveres(fuera));
    assert!(e.viveres(ejercito) < 50_000);
    assert!(hechos(&e.n, "ejercito.reabastecido").is_empty(), "el goteo no es noticia");
}

// ───────────────────────────── 10. Convocatorias ─────────────────────────────

fn convocar(e: &mut Escenario, cuenta: &str, h: u64, ids: &[u64], politica: Politica) -> u64 {
    e.ok(cuenta, Comando::ConvocarEjercito { heroe: h, escuadras: ids.to_vec(), carga: vec![], politica }).creadas[0]
}

fn unirse_a(h: u64, convocatoria: u64, ids: &[u64]) -> Comando {
    Comando::UnirseAConvocatoria { heroe: h, convocatoria, escuadras: ids.to_vec(), carga: vec![] }
}

#[test]
fn convocar_y_unirse_piden_el_mismo_lugar_la_misma_faccion_y_una_salida_posible_y_no_dejan_rastro_al_rechazar() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, ia) = e.con_prestamo("ana", 0);
    let (b, ib) = e.con_prestamo("beto", 0);
    let (d, id_) = e.con_prestamo("dana", 0);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: d, faccion: Some(2) });
    let c = e.heroe_en_la_plaza("carla");
    let convocar_ana = |politica, ids: &[u64]| Comando::ConvocarEjercito { heroe: a, escuadras: ids.to_vec(), carga: vec![], politica };
    let (v, hash, ids) = (e.n.version(), e.n.hash_estado(), e.n.estado().siguiente_id);
    assert_eq!(e.rechazada("beto", convocar_ana(Politica::Aceptar, &ia)), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazada("ana", convocar_ana(Politica::Rechazar, &ia)), rechazos::CONVOCATORIA_POLITICA_INVALIDA);
    assert_eq!(e.rechazada("ana", convocar_ana(Politica::Aceptar, &[99_999])), rechazos::ESCUADRA_AJENA);
    assert_eq!((e.n.version(), e.n.hash_estado(), e.n.estado().siguiente_id), (v, hash, ids), "un rechazo no deja rastro");
    let conv = convocar(&mut e, "ana", a, &ia, Politica::Aceptar);
    assert_eq!(e.rechazada("ana", convocar_ana(Politica::Aceptar, &ia)), rechazos::CONVOCATORIA_YA_EN_ELLA);
    let k = e.n.convocatoria(conv).unwrap();
    assert_eq!((k.lider, k.faccion, k.lugar, k.integrantes.len()), (a, 1, Lugar::Campamento(0), 1));
    assert_eq!(e.n.convocatorias_del_lugar(Lugar::Campamento(0)).len(), 1);
    // Mientras se prepara, nadie se va por otra puerta.
    assert_eq!(e.rechazada("ana", Comando::SalirDelCampamento { campamento: 0, heroe: a, escuadras: ia.clone(), carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None }), rechazos::HEROE_EN_CONVOCATORIA);

    // Unirse: otro lugar, otra Facción, una selección que no puede salir; luego sí.
    assert_eq!(e.rechazada("carla", unirse_a(c, conv, &[])), rechazos::CONVOCATORIA_OTRO_LUGAR);
    assert_eq!(e.rechazada("dana", unirse_a(d, conv, &id_)), rechazos::CONVOCATORIA_OTRA_FACCION);
    assert_eq!(e.rechazada("beto", unirse_a(b, 9_999, &ib)), rechazos::CONVOCATORIA_DESCONOCIDA);
    assert_eq!(e.rechazada("beto", unirse_a(b, conv, &ia)), rechazos::ESCUADRA_AJENA);
    e.ok("beto", unirse_a(b, conv, &ib));
    assert_eq!(e.n.convocatoria(conv).unwrap().integrantes.iter().map(|i| i.heroe).collect::<Vec<_>>(), vec![a, b]);
    assert_eq!(e.rechazada("beto", unirse_a(b, conv, &ib)), rechazos::CONVOCATORIA_YA_EN_ELLA);
    assert_eq!(hechos(&e.n, "convocatoria.unido").len(), 1);
    // Cambiar lo elegido, separarse (el Líder no: cancela) y cancelar (solo el Líder).
    e.ok("beto", Comando::CambiarSeleccionDeConvocatoria { heroe: b, escuadras: vec![ib[0]], carga: vec![] });
    assert_eq!(e.n.convocatoria(conv).unwrap().integrantes[1].seleccion.escuadras, vec![ib[0]]);
    assert_eq!(e.rechazada("beto", Comando::CancelarConvocatoria { heroe: b }), rechazos::CONVOCATORIA_SOLO_EL_LIDER);
    assert_eq!(e.rechazada("ana", Comando::SepararseDeConvocatoria { heroe: a }), rechazos::CONVOCATORIA_EL_LIDER_CANCELA);
    assert_eq!(e.rechazada("carla", Comando::SepararseDeConvocatoria { heroe: c }), rechazos::CONVOCATORIA_NO_ESTAS);
    e.ok("beto", Comando::SepararseDeConvocatoria { heroe: b });
    assert_eq!(e.n.convocatoria(conv).unwrap().integrantes.len(), 1);
    e.ok("beto", unirse_a(b, conv, &ib));
    e.ok("ana", Comando::CancelarConvocatoria { heroe: a });
    assert!(e.n.convocatoria(conv).is_none());
    assert_eq!(hechos(&e.n, "convocatoria.cancelada").len(), 1);
    // Nada salió: las escuadras siguen en el campamento y el héroe sigue dentro.
    assert!(ia.iter().all(|&s| e.escuadra(a, s).contenedor == Contenedor::Campamento));
    assert_eq!(e.n.heroe(b).unwrap().heroe.presencia.ubicacion, Ubicacion::Mercenarios(0));
}

#[test]
fn con_preguntar_la_peticion_a_la_convocatoria_vive_diez_segundos_y_el_silencio_es_un_no() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, ia) = e.con_prestamo("ana", 0);
    let (b, ib) = e.con_prestamo("beto", 0);
    let conv = convocar(&mut e, "ana", a, &ia, Politica::Preguntar);
    let t = e.n.ahora();
    e.ok("beto", unirse_a(b, conv, &ib));
    assert_eq!(e.n.convocatoria(conv).unwrap().integrantes.len(), 1, "preguntar no mete todavía");
    assert_eq!(e.n.convocatoria(conv).unwrap().peticiones[0].expira_en, t + 10_000);
    assert_eq!(hechos(&e.n, "convocatoria.union_pedida").len(), 1);
    let responder = |aceptar| Comando::ResponderPeticionDeConvocatoria { heroe: a, convocatoria: conv, solicitante: b, aceptar };
    assert_eq!(e.rechazada("beto", Comando::ResponderPeticionDeConvocatoria { heroe: b, convocatoria: conv, solicitante: b, aceptar: true }), rechazos::CONVOCATORIA_SOLO_EL_LIDER);
    e.ok("ana", responder(false));
    assert!(e.n.convocatoria(conv).unwrap().peticiones.is_empty());
    assert_eq!(hechos(&e.n, "convocatoria.union_rechazada").len(), 1);
    // Sin contestar, caduca por un evento exacto a los diez segundos.
    let t2 = e.n.ahora();
    e.ok("beto", unirse_a(b, conv, &ib));
    e.n.avanzar_hasta(t2 + 9_999);
    assert_eq!(e.n.convocatoria(conv).unwrap().peticiones.len(), 1);
    e.n.avanzar_hasta(t2 + 10_000);
    assert!(e.n.convocatoria(conv).unwrap().peticiones.is_empty());
    assert_eq!(hechos(&e.n, "convocatoria.peticion_caducada").len(), 1);
    assert_eq!(e.rechazada("ana", responder(true)), rechazos::CONVOCATORIA_NO_HAY_PETICION);
    // Contestada a tiempo y aceptada, entra; el aviso de caducidad que quedaba en la cola ya no vale.
    e.ok("beto", unirse_a(b, conv, &ib));
    e.ok("ana", responder(true));
    assert_eq!(e.n.convocatoria(conv).unwrap().integrantes.len(), 2);
    e.avanzar(MIN);
    assert_eq!(hechos(&e.n, "convocatoria.peticion_caducada").len(), 1);
    assert_eq!(hechos(&e.n, "convocatoria.unido").len(), 1);
}

#[test]
fn partir_es_todo_o_nada_y_sale_un_ejercito_con_lo_que_cada_uno_eligio() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, ia) = e.con_prestamo("ana", 0);
    let (b, ib) = e.con_prestamo("beto", 0);
    let conv = convocar(&mut e, "ana", a, &ia, Politica::Preguntar);
    e.ok("beto", unirse_a(b, conv, &ib));
    e.ok("ana", Comando::ResponderPeticionDeConvocatoria { heroe: a, convocatoria: conv, solicitante: b, aceptar: true });
    assert_eq!(e.rechazada("beto", Comando::PartirConvocatoria { heroe: b, caravana_fundacion: None }), rechazos::CONVOCATORIA_SOLO_EL_LIDER);
    // «beto» deja de residir en el campamento: el préstamo que eligió ya no es suyo y nadie sale.
    e.ok("admin", Comando::FijarResidencia { heroe: b, residencia: None });
    let (v, hash) = (e.n.version(), e.n.hash_estado());
    assert_eq!(e.rechazada("ana", Comando::PartirConvocatoria { heroe: a, caravana_fundacion: None }), rechazos::CONVOCATORIA_NO_PUEDE_SALIR);
    assert_eq!((e.n.version(), e.n.hash_estado()), (v, hash), "todo o nada: ni el Líder sale");
    assert!(e.n.convocatoria(conv).is_some());
    // Se separa de la convocatoria y el Líder parte solo.
    e.ok("beto", Comando::SepararseDeConvocatoria { heroe: b });
    let col = e.ok("ana", Comando::PartirConvocatoria { heroe: a, caravana_fundacion: None }).creadas[0];
    let c = &e.ejercito(col).columna;
    assert_eq!((c.tipo, c.politica, c.lider, c.participantes.iter().map(|p| p.heroe).collect::<Vec<_>>(), c.escuadras.clone()), (TipoColumna::Ejercito, Politica::Preguntar, a, vec![a], ia));
    assert!(e.n.convocatoria(conv).is_none());
    assert_eq!(e.n.heroe(a).unwrap().heroe.presencia.ubicacion, Ubicacion::Columna(col));
    assert_eq!(hechos(&e.n, "convocatoria.partio").len(), 1);
    assert_eq!(e.ejercito(col).columna.viveres.maximo, Some(350_000));
    assert_eq!(e.rechazada("ana", Comando::PartirConvocatoria { heroe: a, caravana_fundacion: None }), rechazos::CONVOCATORIA_NO_ESTAS);
}

#[test]
fn quien_sale_del_lugar_por_su_cuenta_se_queda_fuera_de_la_convocatoria_al_partir() {
    let m = con_mundo!();
    let mut e = escenario(m, 7);
    let (a, ia) = e.con_prestamo("ana", 0);
    let (b, ib) = e.con_prestamo("beto", 0);
    let (c, ic) = e.con_prestamo("carla", 0);
    let conv = convocar(&mut e, "ana", a, &ia, Politica::Aceptar);
    e.ok("beto", unirse_a(b, conv, &ib));
    e.ok("carla", unirse_a(c, conv, &ic));
    // «beto» se desconecta y sale del mundo; «carla» sigue dentro.
    e.ok("beto", Comando::Desconectarse { heroe: b });
    e.avanzar(150_000);
    let col = e.ok("ana", Comando::PartirConvocatoria { heroe: a, caravana_fundacion: None }).creadas[0];
    let c1 = &e.ejercito(col).columna;
    assert_eq!(c1.participantes.iter().map(|p| p.heroe).collect::<Vec<_>>(), vec![a, c], "el que se fue no parte");
    assert_eq!(c1.escuadras.len(), ia.len() + ic.len());
    // Si el Líder ya no está dentro, no se parte.
    let (d, id_) = e.con_prestamo("dana", 0);
    convocar(&mut e, "dana", d, &id_, Politica::Aceptar);
    e.ok("dana", Comando::Desconectarse { heroe: d });
    e.avanzar(150_000);
    assert!(e.n.heroe(d).unwrap().heroe.presencia.fuera);
    assert_eq!(e.rechazada("dana", Comando::PartirConvocatoria { heroe: d, caravana_fundacion: None }), rechazos::HEROE_FUERA_DEL_MUNDO);
}

// ───────────────────────────── 11. Determinismo y recuperación ─────────────────────────────

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
    /// Un héroe de la Facción 1 en el campamento 0 con un préstamo de 30 granjeros y 30 mineros, creado en el instante `t`.
    fn leva_en(&mut self, t: Instante, cuenta: &str) -> (u64, Vec<u64>) {
        let h = self.ok_en(t, cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0];
        self.ok_en(t, "admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
        let ids = self.ok_en(t, cuenta, Comando::PedirPrestamo { heroe: h, tropas: vec![Tropa::Granjeros, Tropa::Mineros] }).creadas;
        (h, ids)
    }
}

/// Tres héroes forman un ejército por convocatoria y se les junta uno en campo: lo que ocurre hasta tener el ejército en marcha.
fn formar(e: &mut Escenario, base: Instante, avanzar: &dyn Fn(&mut Escenario, Instante)) -> (u64, u64, u64, u64) {
    let (ana, ia) = e.leva_en(base, "ana");
    let (beto, ib) = e.leva_en(base, "beto");
    let (carla, ic) = e.leva_en(base, "carla");
    let conv = e.ok_en(base, "ana", Comando::ConvocarEjercito { heroe: ana, escuadras: ia, carga: vec![], politica: Politica::Preguntar }).creadas[0];
    e.ok_en(base, "beto", unirse_a(beto, conv, &ib));
    avanzar(e, base + 5_000);
    e.ok_en(base + 5_000, "ana", Comando::ResponderPeticionDeConvocatoria { heroe: ana, convocatoria: conv, solicitante: beto, aceptar: true });
    avanzar(e, base + MIN);
    let col = e.ok_en(base + MIN, "ana", Comando::PartirConvocatoria { heroe: ana, caravana_fundacion: None }).creadas[0];
    e.ok_en(base + MIN, "carla", Comando::SalirDelCampamento { campamento: 0, heroe: carla, escuadras: ic, carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None });
    (ana, beto, carla, col)
}

/// Un guion de columnas: convocatoria, unión con petición que caduca y otra aceptada, una desconexión, ceder el mando, separarse, hambre y desertar,
/// y el ejército que entra entero en su campamento. Todas las órdenes llevan su instante, así que da igual cuándo avance el reloj entre una y otra.
fn guion(e: &mut Escenario, avanzar: &dyn Fn(&mut Escenario, Instante)) {
    let base = 24 * HORA;
    avanzar(e, base);
    let (ana, beto, carla, col) = formar(e, base, avanzar);
    e.ok_en(base + MIN, "carla", Comando::UnirseEnCampo { heroe: carla, ejercito: col });
    avanzar(e, base + 2 * MIN);
    e.ok_en(base + 2 * MIN, "carla", Comando::UnirseEnCampo { heroe: carla, ejercito: col });
    e.ok_en(base + 2 * MIN, "ana", Comando::ResponderPeticionDeUnion { heroe: ana, ejercito: col, solicitante: carla, aceptar: true });
    avanzar(e, base + 10 * MIN);
    e.ok_en(base + 10 * MIN, "beto", Comando::Desconectarse { heroe: beto });
    avanzar(e, base + 20 * MIN);
    e.ok_en(base + 20 * MIN, "beto", Comando::Conectarse { heroe: beto });
    avanzar(e, base + 30 * MIN);
    e.ok_en(base + 30 * MIN, "ana", Comando::CederLiderazgo { heroe: ana, sucesor: carla });
    e.ok_en(base + 30 * MIN, "ana", Comando::SepararseDelEjercito { heroe: ana });
    avanzar(e, base + 31 * MIN);
    e.ok_en(base + 31 * MIN, "ana", Comando::EntrarEnCampamento { campamento: 0, heroe: ana });
    // Un día y medio acampados sin repostar: se acaban los víveres y la tropa deserta.
    avanzar(e, base + 40 * HORA);
    e.ok_en(base + 40 * HORA, "carla", Comando::EntrarEnCampamento { campamento: 0, heroe: carla });
    avanzar(e, base + 42 * HORA);
    e.n.avanzar_hasta(base + 42 * HORA);
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
    assert_eq!(ana.heroe.presencia.ubicacion, Ubicacion::Mercenarios(0));
    let carla = normal.heroe_de_cuenta("carla").unwrap();
    assert_eq!((carla.columna, carla.heroe.presencia.ubicacion), (None, Ubicacion::Mercenarios(0)), "el ejército entró entero");
    let beto = normal.heroe_de_cuenta("beto").unwrap();
    assert!(beto.columna.is_some_and(|c| normal.ejercito(c).is_some()), "la columna de «beto» sigue en el mapa");
    for h in ["columna.peticion_caducada", "columna.separacion", "columna.liderazgo_cedido", "ejercito.entra_en_campamento", "tropas.desercion", "convocatoria.partio"] {
        assert!(!hechos(&normal, h).is_empty(), "el guion no produjo «{h}»");
    }
    // Como mucho un evento de deserción vigente por columna con hambre: nada recurrente por la que está quieta y harta.
    let vigentes = normal.estado().cola.volcar().iter().filter(|ev| normal.estado().generaciones.es_vigente(ev.clave) && matches!(ev.carga, Carga::Militar(SucesoMilitar::Desercion))).count();
    assert!(vigentes <= normal.estado().ejercitos.len(), "{vigentes} deserciones vigentes");
    let mut sin_guion = escenario(m, 3);
    sin_guion.avanzar(42 * HORA);
    assert_ne!(normal.hash_estado(), sin_guion.n.hash_estado());
}

#[test]
fn recuperar_desde_foto_con_una_convocatoria_ya_partida_y_una_peticion_viva_da_el_mismo_mundo() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9) };
    let base = 24 * HORA;
    // Antes de la foto: el ejército ya partió y una petición de «carla» está viva. Después: caduca, se une, marcha, el hambre y desertar.
    let primera_mitad = |e: &mut Escenario| {
        e.avanzar_hasta(base);
        let (ana, beto, carla, col) = formar(e, base, &|e, t| e.avanzar_hasta(t));
        e.ok_en(base + MIN, "carla", Comando::UnirseEnCampo { heroe: carla, ejercito: col });
        (ana, beto, carla, col)
    };
    let segunda_mitad = |e: &mut Escenario, ana: u64, carla: u64, col: u64| {
        e.avanzar_hasta(base + 3 * MIN);
        assert!(e.ejercito(col).columna.peticiones.is_empty(), "la petición venció tras recuperar");
        e.ok_en(base + 3 * MIN, "carla", Comando::UnirseEnCampo { heroe: carla, ejercito: col });
        e.ok_en(base + 3 * MIN, "ana", Comando::ResponderPeticionDeUnion { heroe: ana, ejercito: col, solicitante: carla, aceptar: true });
        e.ok_en(base + 3 * MIN, "ana", Comando::MarcharA { ejercito: col, destino: e.plaza() });
        e.avanzar_hasta(base + 40 * HORA);
    };

    let sin_parar = {
        let mut e = escenario(m, 9);
        let (ana, _, carla, col) = primera_mitad(&mut e);
        e.avanzar_hasta(base + 2 * MIN);
        segunda_mitad(&mut e, ana, carla, col);
        (e.n.hash_estado(), e.n.estado().serializar())
    };
    let (a_mitad, ana, carla, col, cuenta) = {
        let mut n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        let p = sitio_rico(&n, &m.0, 0, 1);
        let a = fundar(&mut n, 0, "ana", 1, p);
        let mut e = Escenario { n, a, cuenta: 0 };
        let (ana, _, carla, col) = primera_mitad(&mut e);
        assert_eq!(e.ejercito(col).columna.peticiones.len(), 1);
        // La foto a mitad de camino se lee tal cual: serializar → deserializar → serializar es idéntico.
        let bytes = e.n.estado().serializar();
        let vuelta = nucleo::Estado::deserializar(&bytes, &Reglas::default(), &e.n.estado().mundo_hash).unwrap();
        assert_eq!((vuelta.serializar(), vuelta.hash()), (bytes, e.n.estado().hash()));
        e.n.foto_ahora();
        e.avanzar_hasta(base + 2 * MIN);
        e.n.apagar().unwrap();
        (e.n.hash_estado(), ana, carla, col, e.cuenta)
    };
    let n = Nucleo::con_persistencia(con(ajustes), dir.path(), Default::default()).unwrap();
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), a_mitad, "foto + registro reconstruyen el mismo mundo, con la columna y la petición viva");
    let a = *n.estado().asentamientos.keys().next().unwrap();
    let mut e = Escenario { n, a, cuenta };
    segunda_mitad(&mut e, ana, carla, col);
    assert_eq!(e.n.hash_estado(), sin_parar.0);
    assert_eq!(e.n.estado().serializar(), sin_parar.1, "el estado recuperado sigue igual que el que no se interrumpió");
    e.n.apagar().unwrap();
}
