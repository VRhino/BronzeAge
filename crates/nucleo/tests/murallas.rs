//! Los recintos de muralla en el núcleo (Docs/Mechanics/Murallas.md, fase de Murallas, segunda tanda): comprometer, levantar celda a celda con fines
//! exactos, la falta de materiales y el reintento del paso de minuto, la mejora sin perder defensa, la ampliación con arrabal, abandonar, los rechazos
//! sin rastro, la foto a mitad de obra, el determinismo y el hash. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas
//! se omiten si no está.

mod comun;
use asentamiento::catalogo::Tipo;
use asentamiento::recintos::{self, Mejora, Recinto};
use asentamiento::{Cargo, Costo, Recurso};
use base::{Instante, Punto, MS_POR_MINUTO};
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{rechazos, Ajustes, Arranque, Carga, Comando, ErrorPresupuesto, Estado, Nucleo, Peticion, Presupuesto, Reglas, ReglasEconomia, Resultado, Respuesta};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};
use trazado::{CeldaMuro, ClaseCelda};

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;
const UNO: i64 = 1_000_000;

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

/// Las reglas por defecto sin bandidos (miden otra cosa). Con `rico`, un almacén de 100.000 por recurso: un anillo entero cuesta más de lo que cabe en el de
/// partida (400). `ajustar` toca las cifras de la economía.
fn reglas(rico: bool, ajustar: fn(&mut asentamiento::Reglas)) -> Reglas {
    let mut economia = ReglasEconomia::cargar();
    let datos = Arc::make_mut(&mut economia.datos);
    if rico {
        datos.capacidad_inicial = 100_000_000;
    }
    ajustar(datos);
    let mut r = Reglas { economia, ..Reglas::default() };
    let b = &mut Arc::make_mut(&mut r.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    r
}

fn arranque(m: &M, semilla: u64, rico: bool) -> Arranque {
    Arranque { reglas: reglas(rico, |_| {}), ajustes: Ajustes::default(), semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn sitio_rico(n: &Nucleo, m: &Mundo, i: usize, faccion: u32) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    for r in (0..8).map(|k| 50_000 + k * 10_000) {
        for g in 0..360 {
            let a = (g as f64).to_radians();
            let q = Punto::new(p.x + (r as f64 * a.cos()) as i64, p.y + (r as f64 * a.sin()) as i64);
            if n.viabilidad(q, faccion).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6) {
                return q;
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

fn fundar(n: &mut Nucleo, t: Instante, nombre: &str, dueno: &str, faccion: u32, p: Punto) -> u64 {
    let e = n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap();
    assert!(n.aplicar_en(t, Peticion::nueva("admin", &format!("c-{nombre}"), Comando::DarCaravanaFundacion { ejercito: e }).de_admin()).aceptada());
    let r = n.aplicar_en(t, Peticion::nueva(dueno, &format!("f-{nombre}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

/// Una plaza viva de «ana» (Facción 1) sin auto-construcción: solo se construye lo que se pide.
struct Escenario {
    n: Nucleo,
    a: u64,
    cuenta: u64,
}

impl Escenario {
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.cuenta += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.cuenta), c);
        // Andamiaje (5E): una orden de plaza de quien mandaba en ella antes del sistema 5 se da como administración (`comun::mandaba`).
        let p = if actor == "admin" || comun::mandaba(&self.n, actor, &p.comando) { p.de_admin() } else { p };
        let t = self.n.ahora();
        self.n.aplicar_en(t, p)
    }
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    /// Una orden que se rechaza: devuelve su código y comprueba que no dejó rastro (ni versión ni estado).
    fn rechazada(&mut self, actor: &str, c: Comando) -> &'static str {
        let (hash, version) = (self.n.hash_estado(), self.n.version());
        let r = self.ord(actor, c);
        let c = codigo(&r);
        assert_eq!((self.n.hash_estado(), self.n.version()), (hash, version), "un rechazo no deja rastro ({c})");
        c
    }
    fn avanzar(&mut self, ms: i64) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    fn hasta(&mut self, t: Instante) {
        self.n.avanzar_hasta(t);
    }
    fn dar(&mut self, recurso: Recurso, cantidad: i64) {
        let a = self.a;
        let r = self.ord("admin", Comando::DarRecursos { asentamiento: a, recurso, cantidad });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
    }
    /// Nivel de la plaza (la administración lo fija; sube también el operativo).
    fn nivel(&mut self, nivel: u32) {
        let a = self.a;
        self.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel });
    }
    fn eco(&self) -> &asentamiento::asentamiento::Asentamiento {
        self.n.economia(self.a).unwrap()
    }
    fn recintos(&self) -> &[Recinto] {
        &self.eco().recintos
    }
    fn almacen(&self, r: Recurso) -> i64 {
        self.eco().almacen.cantidad(r)
    }
    fn muralla(&self) -> asentamiento::recintos::ReglasMuralla {
        self.n.estado().reglas.economia.datos.muralla.clone()
    }
    fn comprometer(&mut self, nivel: u8) -> Respuesta {
        let a = self.a;
        self.ord("ana", Comando::ComprometerRecinto { asentamiento: a, cargo: Cargo::MaestroObras, nivel })
    }
    fn mejorar(&mut self, recinto: u32) -> Respuesta {
        let a = self.a;
        self.ord("ana", Comando::MejorarRecinto { asentamiento: a, cargo: Cargo::MaestroObras, recinto })
    }
    fn presupuesto(&self, nivel: u8) -> Result<Presupuesto, ErrorPresupuesto> {
        self.n.presupuesto_de_recinto(self.a, nivel)
    }
    /// Los recintos que esperan una celda con su cita en el futuro (uno parado por falta de materiales conserva el instante que le tocaba, ya pasado, y no tiene cita).
    fn citas_de_recintos(&self) -> usize {
        let ahora = self.n.ahora();
        self.recintos().iter().filter(|r| r.siguiente_celda_en.is_some_and(|f| f > ahora)).count()
    }
    /// Las citas de fin de obras de la plaza que hay en la cola son exactamente las de lo que está en curso: las obras, las mejoras, el ascenso y la celda que espera
    /// cada recinto. Ni falta ninguna ni sobra una (un recinto parado por falta de materiales no deja cita).
    fn las_citas_cuadran(&self) {
        let (e, ahora, eco) = (self.n.estado(), self.n.ahora(), self.eco());
        let mut esperadas: Vec<Instante> = eco.edificios.iter().filter_map(|x| x.completa_en.filter(|_| x.estado == asentamiento::edificio::Estado::EnConstruccion)).collect();
        esperadas.extend(eco.edificios.iter().filter_map(|x| x.mejora.map(|m| m.completa_en)));
        esperadas.extend(eco.ascenso.map(|s| s.completa_en));
        esperadas.extend(eco.recintos.iter().filter_map(|r| r.siguiente_celda_en));
        esperadas.retain(|&f| f > ahora);
        esperadas.sort_unstable();
        esperadas.dedup();
        let mut citas: Vec<Instante> = e.cola.volcar().iter().filter(|ev| matches!(ev.carga, Carga::FinDeObras) && ev.clave.entidad == self.a && e.generaciones.es_vigente(ev.clave)).map(|ev| ev.instante).collect();
        citas.sort_unstable();
        citas.dedup();
        assert_eq!(citas, esperadas, "las citas de fin de la cola no cuadran con lo que está en curso");
    }
    /// Avanza hasta que el recinto `i` (índice) se cierra.
    fn terminar(&mut self, i: usize) {
        for _ in 0..5_000 {
            if self.recintos()[i].completo() && self.recintos()[i].mejora.is_none() {
                return;
            }
            let t = self.recintos()[i].siguiente_celda_en.unwrap_or(self.n.ahora() + MIN);
            self.hasta(t);
        }
        panic!("el recinto {i} no termina");
    }
    /// Añade edificios (de uno en uno, esperando a que se levanten) hasta que `parar` lo diga con el presupuesto de la empalizada.
    fn crecer_hasta(&mut self, parar: impl Fn(&Presupuesto) -> bool) -> Presupuesto {
        let a = self.a;
        for k in 0..60 {
            let tipo = [Tipo::Vivienda, Tipo::Almacen, Tipo::Vivienda, Tipo::Lenera][k % 4];
            self.ok("ana", Comando::AnadirEdificio { asentamiento: a, tipo });
            self.avanzar(3 * HORA);
            let p = self.presupuesto(1).expect("la ciudad se puede amurallar");
            if parar(&p) {
                return p;
            }
        }
        panic!("la ciudad no llega a la condición");
    }
}

/// Una plaza nueva de «ana». Con `regalo`, sus almacenes caben el coste de un anillo y se le dan ese número de unidades de madera, piedra y oro.
fn escenario(m: &M, semilla: u64, regalo: Option<i64>) -> Escenario {
    let mut n = Nucleo::nuevo(arranque(m, semilla, regalo.is_some()));
    let p = sitio_rico(&n, &m.0, 0, 1);
    let a = fundar(&mut n, 0, "a", "ana", 1, p);
    let mut e = Escenario { n, a, cuenta: 0 };
    e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
    for r in regalo.map_or(vec![], |_| vec![Recurso::Madera, Recurso::Piedra, Recurso::Oro]) {
        e.dar(r, regalo.unwrap() * 1_000);
    }
    e
}

/// El coste de una celda en `nivel`, sumado como números para comparar con lo que bajó un almacén.
fn gasto(c: &Costo, r: Recurso) -> i64 {
    c.iter().find(|&&(x, _)| x == r).map_or(0, |&(_, n)| n)
}

fn posiciones(celdas: &[CeldaMuro]) -> Vec<(i32, i32)> {
    celdas.iter().map(|c| (c.celda.col, c.celda.row)).collect()
}

// ───────────────────────────── El presupuesto ─────────────────────────────

#[test]
fn el_presupuesto_ensena_el_trazo_y_su_coste_sin_tocar_nada() {
    let m = con_mundo!();
    let e = escenario(m, 7, None);
    let (hash, version) = (e.n.hash_estado(), e.n.version());
    let muralla = e.muralla();
    let mut anterior: Option<Presupuesto> = None;
    for nivel in 1..=3u8 {
        let p = e.presupuesto(nivel).expect("la ciudad de partida se puede amurallar");
        let (celdas, puertas, torres) = (p.trazo.celdas.len(), p.trazo.puertas(), p.trazo.torres());
        assert_eq!(p.nivel, nivel);
        assert!(puertas >= 1 && p.trazo.area_encerrada > 0, "{celdas} celdas, {puertas} puertas");
        assert_eq!((p.extramuros, p.envuelve), (0, true), "sin recintos no hay arrabal ni nada que envolver");
        // El coste es la tarifa acumulada del nivel × (muros + 4 por puerta + 3 por torre).
        let unidades = (celdas - puertas - torres + 4 * puertas + 3 * torres) as i64;
        assert_eq!(p.coste, muralla.coste_de_trazo(&p.trazo.celdas, nivel));
        for &(r, n) in &muralla.tarifa_por_celda[nivel as usize - 1] {
            assert_eq!(gasto(&p.coste, r), n * unidades, "{r:?}");
        }
        assert_eq!(torres > 0, nivel > 1, "la empalizada no tiene torres");
        if let Some(a) = &anterior {
            // El mismo anillo y las mismas puertas en todos los niveles: solo cambian las torres y el coste.
            assert_eq!(posiciones(&a.trazo.celdas), posiciones(&p.trazo.celdas));
            assert!(p.coste.iter().all(|&(r, n)| n > gasto(&a.coste, r)), "cada nivel cuesta más");
        }
        assert_eq!(e.presupuesto(nivel), Ok(p.clone()), "determinista");
        anterior = Some(p);
    }
    assert_eq!(e.presupuesto(0), Err(ErrorPresupuesto::NivelInexistente));
    assert_eq!(e.presupuesto(4), Err(ErrorPresupuesto::NivelInexistente));
    assert_eq!(e.n.presupuesto_de_recinto(9_999, 1), Err(ErrorPresupuesto::SinPlaza));
    assert_eq!((e.n.hash_estado(), e.n.version()), (hash, version), "preguntar no toca nada");
}

// ───────────────────────────── Comprometer y levantar ─────────────────────────────

#[test]
fn comprometer_es_gratis_congela_el_trazo_ocupa_suelo_y_levanta_la_primera_celda_en_la_orden() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, Some(5_000));
    e.nivel(2);
    e.hasta(HORA + 30_000); // fuera de un paso de minuto
    let a = e.a;
    let p = e.presupuesto(1).unwrap();
    let c = e.n.ciudad(a).unwrap();
    assert!(p.trazo.celdas.iter().filter(|c| c.clase != ClaseCelda::Puerta).all(|x| !c.esta_ocupada(x.celda)), "antes de comprometer el suelo del anillo está libre");
    let t0 = e.n.ahora();
    let antes = [e.almacen(Recurso::Madera), e.almacen(Recurso::Piedra)];
    e.las_citas_cuadran();

    let r = e.comprometer(1);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(r.creadas, vec![1], "creadas lleva el id del recinto");
    let rec = &e.recintos()[0];
    assert_eq!((rec.id, rec.nivel, rec.avance, rec.comprometido_en, rec.completado_en, rec.mejora), (1, 1, 0, t0, None, None));
    assert_eq!(rec.celdas, p.trazo.celdas, "el anillo se congela tal como lo enseñó el presupuesto");
    assert_eq!(rec.siguiente_celda_en, Some(t0 + 15 * MIN));
    // La primera celda es la puerta principal (×4) y se paga en el instante de la orden; comprometer en sí no cobra nada.
    assert_eq!(rec.celdas[0].clase, ClaseCelda::Puerta);
    let coste = e.muralla().coste_de_celda(ClaseCelda::Puerta, 1);
    assert_eq!(antes[0] - e.almacen(Recurso::Madera), gasto(&coste, Recurso::Madera));
    assert_eq!(antes[1] - e.almacen(Recurso::Piedra), gasto(&coste, Recurso::Piedra));
    assert_eq!(gasto(&coste, Recurso::Madera), 80_000);
    // Desde el compromiso las celdas de muro (levantadas o no) ocupan suelo, y la ciudad guarda la proyección.
    let c = e.n.ciudad(a).unwrap();
    assert_eq!(c.recintos(), recintos::proyeccion(e.recintos()));
    assert!(rec.celdas.iter().filter(|x| x.clase != ClaseCelda::Puerta).all(|x| c.esta_ocupada(x.celda)));
    assert!(!c.recintos()[0].completo);
    // Una sola cita viva, la de la siguiente celda, y sus hechos.
    assert_eq!(e.citas_de_recintos(), 1);
    e.las_citas_cuadran();
    let comprometido = hechos(&e.n, "construccion.recinto_comprometido");
    assert_eq!(comprometido.len(), 1);
    assert_eq!((comprometido[0].0, dato(&comprometido[0], "recinto"), dato(&comprometido[0], "nivel")), (t0, 1, 1));
    assert_eq!((dato(&comprometido[0], "celdas"), dato(&comprometido[0], "puertas")), (p.trazo.celdas.len() as i64, p.trazo.puertas() as i64));
    let celdas = hechos(&e.n, "construccion.recinto_celda");
    assert_eq!((celdas.len(), dato(&celdas[0], "levantadas"), dato(&celdas[0], "siguienteEn")), (1, 1, t0 + 15 * MIN));
}

#[test]
fn cada_celda_se_levanta_en_su_cita_exacta_y_cobra_su_clase_hasta_cerrar_el_anillo() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, Some(5_000));
    e.nivel(2);
    e.hasta(HORA + 30_000);
    let a = e.a;
    let muralla = e.muralla();
    let t0 = e.n.ahora();
    let base = e.eco().coste_mantenimiento(&e.n.estado().reglas.economia.datos, 0);
    e.ok("ana", Comando::ComprometerRecinto { asentamiento: a, cargo: Cargo::Gobernador, nivel: 1 });
    let celdas = e.recintos()[0].celdas.clone();
    let n = celdas.len();
    for k in 1..n {
        let cita = t0 + k as i64 * 15 * MIN;
        e.hasta(cita - 1);
        assert_eq!(e.recintos()[0].avance, k as i32 - 1, "antes de su cita la celda {k} no está");
        let antes = [e.almacen(Recurso::Madera), e.almacen(Recurso::Piedra)];
        e.hasta(cita);
        let rec = &e.recintos()[0];
        assert_eq!(rec.avance, k as i32, "la celda {k} sale en su cita, al milisegundo");
        let coste = muralla.coste_de_celda(celdas[k].clase, 1);
        assert_eq!(antes[0] - e.almacen(Recurso::Madera), gasto(&coste, Recurso::Madera), "celda {k} ({:?})", celdas[k].clase);
        assert_eq!(antes[1] - e.almacen(Recurso::Piedra), gasto(&coste, Recurso::Piedra));
        // El mantenimiento cuenta las celdas en pie: 0,02 de madera por cada una.
        let actual = e.eco().coste_mantenimiento(&e.n.estado().reglas.economia.datos, 0);
        assert_eq!(gasto(&actual, Recurso::Madera) - gasto(&base, Recurso::Madera), 20 * (k as i64 + 1));
        assert_eq!(e.citas_de_recintos(), usize::from(k < n - 1), "una sola cita viva mientras quedan celdas, ninguna al cerrar");
        e.las_citas_cuadran();
    }
    let rec = &e.recintos()[0];
    assert!(rec.completo());
    assert_eq!((rec.completado_en, rec.siguiente_celda_en, rec.nivel), (Some(t0 + (n as i64 - 1) * 15 * MIN), None, 1));
    assert_eq!(recintos::nivel_del_mejor_completo(e.recintos()), 1);
    assert!(e.n.ciudad(a).unwrap().recintos()[0].completo, "la ciudad ve el anillo completo");
    let hecho = hechos(&e.n, "construccion.recinto_completado");
    assert_eq!(hecho.len(), 1);
    assert_eq!((hecho[0].0, dato(&hecho[0], "recinto"), dato(&hecho[0], "celdas"), dato(&hecho[0], "puertas")), (t0 + (n as i64 - 1) * 15 * MIN, 1, n as i64, 1));
    assert_eq!(hechos(&e.n, "construccion.recinto_celda").len(), n);
    // Cerrado, ya no hay obra: ni citas ni nada que reintentar.
    e.avanzar(5 * HORA);
    e.las_citas_cuadran();
    assert_eq!((e.citas_de_recintos(), e.recintos()[0].avance), (0, n as i32 - 1));
}

#[test]
fn sin_materiales_no_avanza_ni_programa_nada_y_el_paso_de_minuto_reintenta() {
    let m = con_mundo!();
    // La partida trae 50 de madera y 20 de piedra: no alcanza ni para la puerta (80 de madera), y todavía hay que dejar la reserva del mantenimiento.
    let mut e = escenario(m, 7, None);
    e.nivel(2);
    e.hasta(30_000);
    let r = e.comprometer(1);
    assert!(r.aceptada(), "comprometer no cobra: {r:?}");
    assert_eq!((e.recintos()[0].avance, e.recintos()[0].siguiente_celda_en), (-1, None), "no avanza ni se cancela");
    e.las_citas_cuadran(); // y no deja ninguna cita
    e.avanzar(3 * HORA);
    assert_eq!(e.recintos()[0].avance, -1, "tres horas de pasos de minuto sin materiales: nada");
    e.las_citas_cuadran();
    // Llegan los materiales a mitad de minuto: la celda sale en el paso de minuto siguiente, no antes.
    let t_dar = e.n.ahora() + 20_000;
    e.hasta(t_dar);
    e.dar(Recurso::Madera, 1_000_000);
    e.dar(Recurso::Piedra, 1_000_000);
    let t_minuto = (t_dar / MIN + 1) * MIN;
    e.hasta(t_minuto - 1);
    assert_eq!(e.recintos()[0].avance, -1);
    e.hasta(t_minuto);
    assert_eq!(e.recintos()[0].avance, 0);
    assert_eq!(e.recintos()[0].siguiente_celda_en, Some(t_minuto + 15 * MIN), "la siguiente cita cuenta desde cuando se levantó de verdad");
    assert_eq!(e.citas_de_recintos(), 1);
    e.las_citas_cuadran();
}

#[test]
fn la_reserva_de_mantenimiento_frena_la_obra_y_al_bajarla_el_paso_de_minuto_la_retoma() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, Some(1_000));
    e.nivel(2);
    e.hasta(HORA + 30_000);
    let a = e.a;
    let t0 = e.n.ahora();
    e.comprometer(1);
    e.hasta(t0 + 5 * 15 * MIN);
    assert_eq!(e.recintos()[0].avance, 5);
    // El Tesorero reserva más madera de la que queda: la celda que toca no se paga y no se programa otra cita.
    e.ok("ana", Comando::CalibrarReserva { asentamiento: a, recurso: Recurso::Madera, valor: 999_000 });
    assert!(e.almacen(Recurso::Madera) < 999_000);
    e.hasta(t0 + 6 * 15 * MIN);
    assert_eq!(e.recintos()[0].avance, 5, "bajo la reserva no se paga");
    assert_eq!(e.citas_de_recintos(), 0, "ni se espera");
    e.las_citas_cuadran();
    let madera = e.almacen(Recurso::Madera);
    e.avanzar(3 * HORA);
    assert_eq!((e.recintos()[0].avance, e.almacen(Recurso::Madera)), (5, madera), "no se cancela ni cobra nada");
    // Se baja la reserva en mitad de un minuto: la obra sigue en el paso de minuto siguiente y la cadencia de 15 minutos cuenta desde ahí.
    let t_baja = e.n.ahora() + 45_000;
    e.hasta(t_baja);
    e.ok("ana", Comando::CalibrarReserva { asentamiento: a, recurso: Recurso::Madera, valor: 0 });
    let t_minuto = (t_baja / MIN + 1) * MIN;
    e.hasta(t_minuto - 1);
    assert_eq!(e.recintos()[0].avance, 5);
    e.hasta(t_minuto);
    assert_eq!((e.recintos()[0].avance, e.recintos()[0].siguiente_celda_en), (6, Some(t_minuto + 15 * MIN)));
    e.las_citas_cuadran();
    e.hasta(t_minuto + 15 * MIN);
    assert_eq!(e.recintos()[0].avance, 7, "y de ahí en adelante, por cita");
}

// ───────────────────────────── Mejorar ─────────────────────────────

#[test]
fn mejorar_no_quita_multiplicador_ni_cupo_ni_requisito_y_cobra_la_diferencia() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, Some(50_000));
    let a = e.a;
    e.nivel(3);
    e.hasta(HORA + 30_000);
    let muralla = e.muralla();
    // Un Barracón (cupo de guarnición de 7) y la empalizada (+10 al completarla).
    e.ok("admin", Comando::AnadirEdificio { asentamiento: a, tipo: Tipo::Barracon });
    e.ok("ana", Comando::ComprometerRecinto { asentamiento: a, cargo: Cargo::MaestroObras, nivel: 1 });
    e.terminar(0);
    for _ in 0..40 {
        if e.n.cupo_de_guarnicion(a) == Some(17) {
            break;
        }
        e.avanzar(HORA);
    }
    assert_eq!(e.n.cupo_de_guarnicion(a), Some(7 + 10), "Barracón de nivel 1 y empalizada completa");
    let puertas = e.recintos()[0].puertas() as i64;
    let multiplicador = |e: &Escenario| recintos::multiplicador_defensivo(&e.muralla(), e.recintos());
    assert_eq!(multiplicador(&e), UNO + 300_000 / puertas, "bono de la empalizada (1,3) repartido entre sus puertas");
    let (mult0, upkeep0, proyeccion0) = (multiplicador(&e), recintos::upkeep(&muralla, e.recintos()), e.n.ciudad(a).unwrap().recintos().to_vec());
    let coste0 = e.eco().coste_mantenimiento(&e.n.estado().reglas.economia.datos, 0);

    // Mejorar a piedra: el anillo en pie sigue defendiendo con su nivel de antes.
    e.hasta(e.n.ahora() + 17_000);
    let (t1, madera, piedra) = (e.n.ahora(), e.almacen(Recurso::Madera), e.almacen(Recurso::Piedra));
    let r = e.mejorar(1);
    assert!(r.aceptada(), "{r:?}");
    let rec = &e.recintos()[0];
    assert_eq!((rec.nivel, rec.completo(), rec.mejora), (1, true, Some(Mejora { nivel: 2, avance: 0 })));
    assert_eq!(rec.siguiente_celda_en, Some(t1 + 30 * MIN), "30 minutos por celda al nivel de piedra");
    let diferencia = muralla.coste_de_mejora_de_celda(ClaseCelda::Puerta, 2);
    assert_eq!((madera - e.almacen(Recurso::Madera), piedra - e.almacen(Recurso::Piedra)), (gasto(&diferencia, Recurso::Madera), gasto(&diferencia, Recurso::Piedra)), "la primera celda cuesta la diferencia de tarifas, ×4 por ser puerta");
    assert_eq!(gasto(&diferencia, Recurso::Madera), 20_000);
    assert_eq!(gasto(&diferencia, Recurso::Piedra), 100_000);
    let intacto = |e: &Escenario| {
        assert_eq!(multiplicador(e), mult0, "el multiplicador es el del anillo en pie");
        assert_eq!(e.n.cupo_de_guarnicion(a), Some(17), "el cupo de guarnición sigue con el nivel de antes");
        assert_eq!(recintos::nivel_del_mejor_completo(e.recintos()), 1, "y el requisito del nivel 4");
        assert_eq!(recintos::upkeep(&e.muralla(), e.recintos()), upkeep0, "el mantenimiento cuenta las celdas en pie");
        assert_eq!(e.eco().coste_mantenimiento(&e.n.estado().reglas.economia.datos, 0), coste0);
        assert_eq!(e.n.ciudad(a).unwrap().recintos(), &proyeccion0[..], "la ciudad ni se entera: no se recalcula nada al mejorar");
        assert!(e.recintos()[0].completo() && e.recintos()[0].nivel == 1);
    };
    intacto(&e);
    e.hasta(t1 + 10 * 30 * MIN);
    assert_eq!(e.recintos()[0].mejora.map(|m| m.avance), Some(10));
    intacto(&e);
    assert_eq!(e.rechazada("ana", Comando::MejorarRecinto { asentamiento: a, cargo: Cargo::MaestroObras, recinto: 1 }), rechazos::MURALLAS_YA_MEJORANDO);
    // Todos los anillos están completos, así que comprometer otro pasa a pedir el arrabal.
    assert_eq!(e.rechazada("ana", Comando::ComprometerRecinto { asentamiento: a, cargo: Cargo::MaestroObras, nivel: 1 }), rechazos::MURALLAS_ARRABAL);

    // Al cerrarse la mejora: nivel nuevo, torres recalculadas sobre el mismo anillo y el cupo y el mantenimiento del nivel nuevo.
    e.terminar(0);
    let rec = &e.recintos()[0];
    assert_eq!((rec.nivel, rec.mejora, rec.siguiente_celda_en, rec.avance), (2, None, None, rec.celdas.len() as i32 - 1));
    let directo = e.presupuesto(2).unwrap();
    assert_eq!(rec.celdas, directo.trazo.celdas, "mejorar recalcula las torres sobre las celdas de muro y da exactamente el trazo directo del nivel");
    assert!(rec.celdas.iter().any(|c| c.clase == ClaseCelda::Torre));
    assert_eq!(e.n.cupo_de_guarnicion(a), Some(7 + 15), "piedra: +15");
    assert_eq!(recintos::upkeep(&muralla, e.recintos()), vec![(Recurso::Piedra, 20 * rec.celdas.len() as i64)]);
    assert_eq!(e.n.ciudad(a).unwrap().recintos(), &proyeccion0[..], "muro y torre ocupan igual: la proyección no cambió");
    e.las_citas_cuadran();
    assert_eq!(e.citas_de_recintos(), 0);
    let mejorado = hechos(&e.n, "construccion.recinto_mejorado");
    assert_eq!((mejorado.len(), dato(&mejorado[0], "recinto"), dato(&mejorado[0], "nivel")), (1, 1, 2));
    assert_eq!(hechos(&e.n, "construccion.recinto_mejora_iniciada").len(), 1);
    // A adarve hace falta una plaza de nivel 4.
    assert_eq!(e.rechazada("ana", Comando::MejorarRecinto { asentamiento: a, cargo: Cargo::MaestroObras, recinto: 1 }), rechazos::MURALLAS_NIVEL_DE_PLAZA);
}

// ───────────────────────────── Ampliar ─────────────────────────────

#[test]
fn ampliar_exige_arrabal_y_envolver_y_mientras_el_nuevo_esta_en_obra_defiende_el_anterior() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, Some(50_000));
    let a = e.a;
    e.nivel(3);
    e.hasta(HORA + 30_000);
    e.comprometer(1);
    e.terminar(0);
    let interior = e.recintos()[0].clone();
    let (mult0, upkeep_interior) = (recintos::multiplicador_defensivo(&e.muralla(), e.recintos()), recintos::upkeep(&e.muralla(), e.recintos()));
    // Con solo el anillo no hay ampliación posible: ni arrabal ni nada que envolver.
    assert_eq!(e.rechazada("ana", Comando::ComprometerRecinto { asentamiento: a, cargo: Cargo::MaestroObras, nivel: 1 }), rechazos::MURALLAS_ARRABAL);

    // La ciudad crece (primero dentro del anillo, luego fuera): con pocos edificios extramuros todavía no, con 6 y envolviendo sí.
    let p = e.crecer_hasta(|p| p.extramuros >= 3);
    assert!((3..6).contains(&p.extramuros), "{} extramuros", p.extramuros);
    assert_eq!(e.rechazada("ana", Comando::ComprometerRecinto { asentamiento: a, cargo: Cargo::MaestroObras, nivel: 1 }), rechazos::MURALLAS_ARRABAL);
    let p = e.crecer_hasta(|p| p.extramuros >= 6 && p.envuelve);
    assert!(p.extramuros >= 6);
    let ahora = e.n.ahora();
    let r = e.comprometer(1);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(r.creadas, vec![2], "el segundo recinto");
    let (viejo, nuevo) = (&e.recintos()[0], &e.recintos()[1]);
    assert_eq!(viejo, &interior, "el anillo interior no se toca");
    assert_eq!((nuevo.id, nuevo.nivel, nuevo.comprometido_en), (2, 1, ahora));
    assert!(trazado::contiene_a(&nuevo.celdas, &viejo.celdas), "el anillo nuevo envuelve estrictamente al viejo");
    // Q1: mientras el nuevo está en obra, defiende el interior (multiplicador, cupo y requisito), y el mantenimiento suma las celdas de los dos.
    assert!(!nuevo.completo() && nuevo.levantadas() >= 1 && nuevo.completado_en.is_none());
    assert_eq!(recintos::de_referencia(e.recintos()).map(|r| r.id), Some(1));
    assert_eq!(recintos::multiplicador_defensivo(&e.muralla(), e.recintos()), mult0);
    assert_eq!(recintos::nivel_del_mejor_completo(e.recintos()), 1);
    let por_celda = 20 * (viejo.celdas.len() as i64 + nuevo.levantadas() as i64);
    assert_eq!(recintos::upkeep(&e.muralla(), e.recintos()), vec![(Recurso::Madera, por_celda)]);
    assert_eq!(upkeep_interior, vec![(Recurso::Madera, 20 * viejo.celdas.len() as i64)]);
    let c = e.n.ciudad(a).unwrap();
    assert_eq!((c.recintos().len(), c.recintos()[0].completo, c.recintos()[1].completo), (2, true, false));
    // Uno en obra bloquea comprometer otro; abandonarlo es posible porque nunca se cerró; el interior, completo, no.
    assert_eq!(e.rechazada("ana", Comando::ComprometerRecinto { asentamiento: a, cargo: Cargo::MaestroObras, nivel: 1 }), rechazos::MURALLAS_EN_OBRA);
    assert_eq!(e.rechazada("ana", Comando::AbandonarRecinto { asentamiento: a, recinto: 1 }), rechazos::MURALLAS_YA_CERRADO);
    // Al cerrarse, el nuevo pasa a ser el que defiende.
    e.terminar(1);
    assert!(e.recintos()[1].completo() && e.recintos()[1].completado_en.is_some());
    assert_eq!(recintos::de_referencia(e.recintos()).map(|r| r.id), Some(2));
    let puertas2 = e.recintos()[1].puertas() as i64;
    assert_eq!(recintos::multiplicador_defensivo(&e.muralla(), e.recintos()), UNO + 300_000 / puertas2);
    assert_eq!(hechos(&e.n, "construccion.recinto_completado").len(), 2);
    assert!(e.n.ciudad(a).unwrap().recintos().iter().all(|r| r.completo));
    // Con los dos anillos completos y sin edificios nuevos fuera del segundo, ya no hay arrabal para un tercero.
    assert_eq!(e.rechazada("ana", Comando::ComprometerRecinto { asentamiento: a, cargo: Cargo::MaestroObras, nivel: 1 }), rechazos::MURALLAS_ARRABAL);
}

#[test]
fn una_ampliacion_que_no_envuelve_al_anillo_viejo_se_rechaza_sin_rastro() {
    let m = con_mundo!();
    let mut e = escenario_con_arrabal(m);
    let a = e.a;
    e.nivel(3);
    e.hasta(HORA + 30_000);
    e.comprometer(1);
    e.terminar(0);
    // Con un solo edificio fuera basta de arrabal, pero el anillo nuevo todavía coincide con el viejo por algún lado.
    let p = e.crecer_hasta(|p| p.extramuros >= 1 && !p.envuelve);
    assert!(!p.envuelve);
    assert_eq!(e.rechazada("ana", Comando::ComprometerRecinto { asentamiento: a, cargo: Cargo::MaestroObras, nivel: 1 }), rechazos::MURALLAS_NO_ENVUELVE);
    assert_eq!(e.recintos().len(), 1);
}

/// Como [`escenario`] rico, pero con un solo edificio extramuros de arrabal para ampliar (la cifra del bloque es 6): así se llega al rechazo de «no envuelve».
fn escenario_con_arrabal(m: &M) -> Escenario {
    let arranque = Arranque { reglas: reglas(true, |d| d.muralla.arrabal_minimo = 1), ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) };
    let mut n = Nucleo::nuevo(arranque);
    let p = sitio_rico(&n, &m.0, 0, 1);
    let a = fundar(&mut n, 0, "a", "ana", 1, p);
    let mut e = Escenario { n, a, cuenta: 0 };
    e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
    for r in [Recurso::Madera, Recurso::Piedra, Recurso::Oro] {
        e.dar(r, 50_000_000);
    }
    e
}

// ───────────────────────────── Abandonar ─────────────────────────────

#[test]
fn abandonar_borra_un_recinto_que_nunca_se_cerro_sin_devolver_nada_y_libera_el_suelo() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, Some(5_000));
    let a = e.a;
    e.nivel(2);
    e.hasta(HORA + 30_000);
    e.comprometer(1);
    let celdas = e.recintos()[0].celdas.clone();
    e.avanzar(3 * 15 * MIN);
    assert_eq!(e.recintos()[0].avance, 3);
    assert!(celdas.iter().filter(|c| c.clase != ClaseCelda::Puerta).all(|c| e.n.ciudad(a).unwrap().esta_ocupada(c.celda)));
    let (madera, piedra) = (e.almacen(Recurso::Madera), e.almacen(Recurso::Piedra));
    assert_eq!(e.rechazada("ana", Comando::AbandonarRecinto { asentamiento: a, recinto: 7 }), rechazos::MURALLAS_RECINTO_DESCONOCIDO);
    assert_eq!(e.rechazada("beto", Comando::AbandonarRecinto { asentamiento: a, recinto: 1 }), rechazos::NO_AUTORIZADO);
    let r = e.ok("ana", Comando::AbandonarRecinto { asentamiento: a, recinto: 1 });
    assert!(r.creadas.is_empty());
    assert!(e.recintos().is_empty());
    assert_eq!((e.almacen(Recurso::Madera), e.almacen(Recurso::Piedra)), (madera, piedra), "sin devolución");
    let c = e.n.ciudad(a).unwrap();
    assert!(c.recintos().is_empty(), "la ciudad deja de tener el anillo");
    assert!(celdas.iter().filter(|c| c.clase != ClaseCelda::Puerta).all(|x| !c.esta_ocupada(x.celda)), "el suelo queda libre");
    assert_eq!(hechos(&e.n, "construccion.recinto_abandonado").len(), 1);
    assert_eq!(recintos::upkeep(&e.muralla(), e.recintos()), vec![]);
    // La cita que quedaba en la cola no encuentra nada que cumplir.
    e.avanzar(2 * HORA);
    assert!(e.recintos().is_empty());
    // Y el id queda libre para el siguiente.
    let r = e.comprometer(1);
    assert_eq!(r.creadas, vec![1]);
    e.terminar(0);
    assert_eq!(e.rechazada("ana", Comando::AbandonarRecinto { asentamiento: a, recinto: 1 }), rechazos::MURALLAS_YA_CERRADO, "uno que se cerró no se abandona");
}

// ───────────────────────────── Rechazos ─────────────────────────────

#[test]
fn los_rechazos_tienen_su_codigo_y_no_dejan_rastro() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, Some(50_000));
    let a = e.a;
    let comprometer = |cargo, nivel| Comando::ComprometerRecinto { asentamiento: a, cargo, nivel };
    let mejorar = |recinto| Comando::MejorarRecinto { asentamiento: a, cargo: Cargo::Gobernador, recinto };
    // En el orden en que se comprueban: la plaza, la autorización, el cargo, el nivel de recinto, el nivel de plaza.
    assert_eq!(e.rechazada("ana", Comando::ComprometerRecinto { asentamiento: 9_999, cargo: Cargo::Gobernador, nivel: 1 }), rechazos::ASENTAMIENTO_DESCONOCIDO);
    assert_eq!(e.rechazada("beto", comprometer(Cargo::Gobernador, 1)), rechazos::NO_AUTORIZADO);
    for cargo in [Cargo::Tesorero, Cargo::General, Cargo::Sacerdote] {
        assert_eq!(e.rechazada("ana", comprometer(cargo, 1)), rechazos::MURALLAS_SIN_CARGO, "{cargo:?}");
        assert_eq!(e.rechazada("ana", Comando::MejorarRecinto { asentamiento: a, cargo, recinto: 1 }), rechazos::MURALLAS_SIN_CARGO);
    }
    for nivel in [0, 4, 200] {
        assert_eq!(e.rechazada("ana", comprometer(Cargo::Gobernador, nivel)), rechazos::MURALLAS_NIVEL_INEXISTENTE, "nivel {nivel}");
    }
    assert_eq!(e.rechazada("ana", comprometer(Cargo::Gobernador, 1)), rechazos::MURALLAS_NIVEL_DE_PLAZA, "la empalizada pide una plaza de nivel 2");
    e.nivel(2);
    assert_eq!(e.rechazada("ana", comprometer(Cargo::Gobernador, 2)), rechazos::MURALLAS_NIVEL_DE_PLAZA, "el muro de piedra, de nivel 3");
    assert_eq!(e.rechazada("ana", comprometer(Cargo::Gobernador, 3)), rechazos::MURALLAS_NIVEL_DE_PLAZA, "y el adarve, de nivel 4");
    // Mejorar lo que no existe, lo que no está completo y lo que no cabe.
    assert_eq!(e.rechazada("ana", mejorar(1)), rechazos::MURALLAS_RECINTO_DESCONOCIDO);
    e.nivel(3);
    e.hasta(HORA + 30_000);
    e.ok("ana", comprometer(Cargo::MaestroObras, 1));
    assert_eq!(e.rechazada("ana", mejorar(1)), rechazos::MURALLAS_INCOMPLETO, "primero se cierra el anillo");
    assert_eq!(e.rechazada("ana", comprometer(Cargo::Gobernador, 1)), rechazos::MURALLAS_EN_OBRA);
    e.terminar(0);
    assert_eq!(e.rechazada("ana", Comando::AbandonarRecinto { asentamiento: a, recinto: 1 }), rechazos::MURALLAS_YA_CERRADO);
    // Ya mejorando, y en el máximo (un adarve directo, con una plaza de nivel 4).
    e.ok("ana", mejorar(1));
    assert_eq!(e.rechazada("ana", mejorar(1)), rechazos::MURALLAS_YA_MEJORANDO);
    e.terminar(0);
    e.nivel(4);
    e.ok("ana", mejorar(1));
    e.terminar(0);
    assert_eq!(e.recintos()[0].nivel, 3);
    assert_eq!(e.rechazada("ana", mejorar(1)), rechazos::MURALLAS_EN_EL_MAXIMO);
    // Sin arrabal tampoco se amplía.
    assert_eq!(e.rechazada("ana", comprometer(Cargo::Gobernador, 3)), rechazos::MURALLAS_ARRABAL);
    assert_eq!(hechos(&e.n, "construccion.recinto_mejorado").len(), 2);
}

// ───────────────────────────── Foto, recuperación, determinismo ─────────────────────────────

#[test]
fn la_foto_a_mitad_de_obra_conserva_la_cita_y_se_rechaza_la_del_formato_anterior() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, Some(50_000));
    let a = e.a;
    e.nivel(3);
    e.hasta(HORA + 30_000);
    e.comprometer(1);
    e.terminar(0);
    e.mejorar(1);
    e.hasta(e.n.ahora() + 4 * HORA + 17_000); // a mitad de la mejora
    let mitad = e.recintos()[0].mejora.unwrap().avance;
    assert!(mitad > 0 && (mitad as usize) < e.recintos()[0].celdas.len() - 1);
    e.las_citas_cuadran();

    let reglas = reglas(true, |_| {});
    let bytes = e.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN41");
    let vuelta = Estado::deserializar(&bytes, &reglas, &m.0.hash().to_string()).unwrap();
    assert_eq!(vuelta.serializar(), bytes, "ida y vuelta idéntica");
    assert_eq!(vuelta.asentamientos, e.n.estado().asentamientos);
    assert_eq!(vuelta.asentamientos[&a].economia.recintos, e.recintos());
    assert_eq!(vuelta.asentamientos[&a].urbe.ciudad().recintos(), recintos::proyeccion(e.recintos()), "la ciudad se deriva de los recintos al leerla");
    assert_eq!(vuelta.hash(), e.n.hash_estado());
    // La cola vuelve con la cita de la celda siguiente.
    let siguiente = e.recintos()[0].siguiente_celda_en.unwrap();
    assert!(vuelta.cola.volcar().iter().any(|ev| ev.instante == siguiente && matches!(ev.carga, Carga::FinDeObras) && ev.clave.entidad == a));

    // El formato anterior (los recintos dentro de la ciudad, sin nivel, avance ni mejora) se rechaza con su motivo.
    let viejo = [b"BAN19".as_slice(), &bytes[5..]].concat();
    let Err(err) = Estado::deserializar(&viejo, &reglas, &m.0.hash().to_string()) else { panic!("BAN19 no debe admitirse") };
    assert!(err.to_string().contains("BAN19") && err.to_string().contains("recintos"), "{err}");
    // Un recinto con el avance fuera del anillo es una foto corrupta.
    let mut mala = Estado::deserializar(&bytes, &reglas, &m.0.hash().to_string()).unwrap();
    mala.asentamientos.get_mut(&a).unwrap().economia.recintos[0].avance = 9_999;
    assert!(Estado::deserializar(&mala.serializar(), &reglas, &m.0.hash().to_string()).is_err());
}

#[test]
fn recuperar_a_mitad_de_obra_de_una_muralla_no_cambia_el_mundo() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let con = |ajustes: Ajustes| Arranque { ajustes, ..arranque(m, 9, true) };
    // Las mismas órdenes en instantes que no son de minuto: una muralla de empalizada que se cierra y se mejora a piedra; la foto cae a mitad de la mejora.
    let arrancar = |n: &mut Nucleo| {
        let p = sitio_rico(n, &m.0, 0, 1);
        let id = fundar(n, 0, "a", "ana", 1, p);
        let admin = |i: &str, c| Peticion::nueva("admin", i, c).de_admin();
        assert!(n.aplicar_en(0, admin("pausa", Comando::AlternarAutoConstruccion { asentamiento: id, pausada: true })).aceptada());
        n.avanzar_hasta(4 * MIN + 55_000);
        for r in [Recurso::Madera, Recurso::Piedra] {
            n.aplicar_en(4 * MIN + 55_000, admin(&format!("dar-{r:?}"), Comando::DarRecursos { asentamiento: id, recurso: r, cantidad: 60_000_000 }));
        }
        assert!(n.aplicar_en(4 * MIN + 55_000, admin("nivel", Comando::FijarNivelAsentamiento { asentamiento: id, nivel: 3 })).aceptada());
        let orden = |i: &str, c| comun::como_dueno(Peticion::nueva("ana", i, c));
        assert!(n.aplicar_en(5 * MIN + 1_000, orden("muralla", Comando::ComprometerRecinto { asentamiento: id, cargo: Cargo::MaestroObras, nivel: 1 })).aceptada());
        id
    };
    let mejora = |n: &mut Nucleo, id: u64| {
        let t = 10 * HORA + 13_000;
        n.avanzar_hasta(t);
        assert!(n.aplicar_en(t, comun::como_dueno(Peticion::nueva("ana", "mejora", Comando::MejorarRecinto { asentamiento: id, cargo: Cargo::MaestroObras, recinto: 1 }))).aceptada());
    };
    let final_ = 40 * HORA;
    let sin_parar = {
        let mut n = Nucleo::nuevo(con(ajustes.clone()));
        let id = arrancar(&mut n);
        mejora(&mut n, id);
        n.avanzar_hasta(final_);
        (n.hash_estado(), n.economia(id).unwrap().recintos.clone())
    };
    assert!(sin_parar.1[0].completo() && sin_parar.1[0].nivel == 2 && sin_parar.1[0].mejora.is_none(), "la mejora acaba antes del final");
    let (id, a_mitad, siguiente) = {
        let mut n = Nucleo::con_persistencia(con(ajustes.clone()), dir.path(), Default::default()).unwrap();
        let id = arrancar(&mut n);
        mejora(&mut n, id);
        n.avanzar_hasta(16 * HORA + 17_000);
        let rec = n.economia(id).unwrap().recintos[0].clone();
        assert!(rec.mejora.is_some_and(|m| m.avance > 2), "a mitad de la mejora: {rec:?}");
        n.foto_ahora();
        n.apagar().unwrap();
        (id, n.hash_estado(), rec.siguiente_celda_en)
    };
    let mut n = Nucleo::con_persistencia(con(ajustes), dir.path(), Default::default()).unwrap();
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), a_mitad, "foto + registro reconstruyen el mismo mundo");
    assert_eq!(n.economia(id).unwrap().recintos[0].siguiente_celda_en, siguiente);
    n.avanzar_hasta(final_);
    assert_eq!((n.hash_estado(), n.economia(id).unwrap().recintos.clone()), sin_parar, "recuperar a mitad de la mejora no cambia el mundo");
    n.apagar().unwrap();
}

/// Una partida de muralla con las órdenes en instantes que no son de minuto: se cierra la empalizada y se intenta mejorarla a piedra cada hora. Con `escaso`, la plaza
/// solo tiene lo justo, el Tesorero reserva más madera de la que queda a la hora 3 y la libera a la 5 (la obra se frena y la retoma el paso de minuto) y la partida acaba
/// antes de que termine la gracia del mantenimiento (24 h); sin él, la plaza es rica y llega a la mejora completa. Devuelve el resumen que tiene que ser igual en los tres
/// modos de avanzar y, aparte, el avance del anillo en las horas 4 a 6 (solo comparable en el modo normal).
fn correr(m: &M, escaso: bool, avanzar: &dyn Fn(&mut Nucleo, Instante)) -> (Resumen, Vec<i32>) {
    let mut n = Nucleo::nuevo(arranque(m, 5, true));
    let p = sitio_rico(&n, &m.0, 0, 1);
    let id = fundar(&mut n, 31_000, "a", "ana", 1, p);
    let admin = |i: &str, c| Peticion::nueva("admin", i, c).de_admin();
    let ana = |i: &str, c| comun::como_dueno(Peticion::nueva("ana", i, c));
    let t = HORA + 13_579; // nunca coincide con un paso de minuto de la plaza
    avanzar(&mut n, t);
    n.aplicar_en(t, admin("nivel", Comando::FijarNivelAsentamiento { asentamiento: id, nivel: 3 }));
    n.aplicar_en(t, admin("pausa", Comando::AlternarAutoConstruccion { asentamiento: id, pausada: true }));
    let regalo = if escaso { 800_000 } else { 60_000_000 };
    for r in [Recurso::Madera, Recurso::Piedra] {
        n.aplicar_en(t, admin(&format!("dar-{r:?}"), Comando::DarRecursos { asentamiento: id, recurso: r, cantidad: regalo }));
    }
    let horas = if escaso { 22 } else { 40 };
    let mut avances = vec![];
    for hora in 1..=horas {
        let t = hora * HORA + 13_579;
        avanzar(&mut n, t);
        if matches!(hora, 4..=6) {
            avances.push(n.economia(id).unwrap().recintos[0].avance);
        }
        match (hora, escaso) {
            (1, _) => drop(n.aplicar_en(t + 1, ana("muralla", Comando::ComprometerRecinto { asentamiento: id, cargo: Cargo::MaestroObras, nivel: 1 }))),
            (3, true) => drop(n.aplicar_en(t + 2, ana("reserva", Comando::CalibrarReserva { asentamiento: id, recurso: Recurso::Madera, valor: 999_000 }))),
            (5, true) => drop(n.aplicar_en(t + 2, ana("sin-reserva", Comando::CalibrarReserva { asentamiento: id, recurso: Recurso::Madera, valor: 0 }))),
            _ => {}
        }
        // Cada hora lo intenta: se rechaza hasta que el anillo está completo y sin mejora, y entonces empieza la mejora.
        n.aplicar_en(t + 3, ana(&format!("mejora-{hora}"), Comando::MejorarRecinto { asentamiento: id, cargo: Cargo::MaestroObras, recinto: 1 }));
    }
    avanzar(&mut n, (horas + 1) * HORA);
    n.avanzar_hasta((horas + 2) * HORA);
    let rec = n.economia(id).expect("la plaza sigue en pie").recintos[0].clone();
    ((n.hash_estado(), rec.nivel, rec.completo(), hechos(&n, "construccion.recinto_celda").len()), avances)
}

/// El hash del mundo, el nivel del recinto, si está completo y cuántas celdas se levantaron.
type Resumen = ([u8; 32], u8, bool, usize);

#[test]
fn con_ordenes_fuera_de_minuto_el_modo_normal_el_acelerado_y_a_trozos_dan_el_mismo_mundo() {
    let m = con_mundo!();
    for escaso in [false, true] {
        let (normal, avances) = correr(m, escaso, &|n, t| n.avanzar_hasta(t));
        if escaso {
            // Entre la hora 3 y la 5 la obra está parada (la reserva pide más madera de la que queda); al bajarla, el paso de minuto la retoma.
            let [h4, h5, h6] = avances[..] else { panic!("{avances:?}") };
            assert!(h4 > 0 && h4 == h5 && h6 > h5, "parada de la hora 3 a la 5 y retomada después: {avances:?}");
            assert!(normal.2 && normal.3 >= 28, "el anillo se cierra: {normal:?}");
        } else {
            assert!(normal.1 == 2 && normal.2 && normal.3 > 40, "se cerró y se mejoró: {normal:?}");
        }
        assert_eq!(normal, correr(m, escaso, &|n, t| n.avanzar_hasta(t)).0, "repetible");
        assert_eq!(normal, correr(m, escaso, &|_, _| {}).0, "acelerado de un golpe: solo avanza lo que pide cada orden al entrar");
        let a_trozos = correr(m, escaso, &|n, t| {
            let ahora = n.ahora();
            for s in 1..=7 {
                n.avanzar_hasta(ahora + (t - ahora).max(0) * s * s / 49);
            }
        });
        assert_eq!(normal, a_trozos.0);
    }
}

// ───────────────────────────── Rendimiento ─────────────────────────────

#[test]
#[ignore = "mide el coste de set_recintos y del trazo sobre una ciudad crecida; en release"]
fn rendimiento_de_set_recintos_y_del_trazo() {
    let m = con_mundo!();
    let mut e = escenario(m, 7, Some(20_000));
    let a = e.a;
    e.nivel(3);
    e.hasta(HORA + 30_000);
    e.comprometer(1);
    e.terminar(0);
    e.crecer_hasta(|p| p.extramuros >= 6 && p.envuelve);
    let urbe = e.n.estado().asentamientos[&a].urbe.clone();
    let proyeccion = recintos::proyeccion(e.recintos());
    let piezas = urbe.ciudad().piezas().len();
    let medir = |nombre: &str, veces: u32, mut f: Box<dyn FnMut()>| {
        let t = std::time::Instant::now();
        for _ in 0..veces {
            f();
        }
        println!("{nombre}: {:.1} µs por llamada ({veces} llamadas, {piezas} piezas)", t.elapsed().as_secs_f64() * 1e6 / veces as f64);
    };
    let mut u = urbe.clone();
    let p = proyeccion.clone();
    medir("Urbe::set_recintos con 1 recinto", 200, Box::new(move || u.set_recintos(p.clone())));
    let mut u = urbe.clone();
    medir("Urbe::set_recintos sin recintos", 200, Box::new(move || u.set_recintos(vec![])));
    let nucleo = &e.n;
    let t = std::time::Instant::now();
    for _ in 0..20 {
        nucleo.presupuesto_de_recinto(a, 1).unwrap();
    }
    println!("presupuesto_de_recinto (trazo + extramuros): {:.1} µs por llamada", t.elapsed().as_secs_f64() * 1e6 / 20.0);
}
