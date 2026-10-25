//! La niebla en el núcleo (4H T3d, Niebla_De_Guerra §2-§5): lo que graba cada ojo en la memoria de su Facción al cerrar las versiones (lo explorado por una columna, las fichas de lo ajeno con
//! el último instante en que se vio, lo de un héroe sin bandera), la foto (`BAN36`) y que todo es lo mismo con sesiones y sin ellas; y lo que proyecta la vista de una Facción
//! (`proyeccion::vista::Mirada`): lo propio entero, lo ajeno redactado, lo recordado como ficha, lo que ven los aliados y el tramo ajeno que se renueva. Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;
use base::{Instante, Mil, Plan, Punto, MS_POR_MINUTO};
use contrato::v1::{self as pb, entidad::Cuerpo, TipoEntidad};
use militar::heroe::Controlador;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::memoria::Sitio;
use nucleo::proyeccion::vista::{Mirada, Perspectiva, Sello};
use nucleo::{Ajustes, Arranque, Comando, Estado, Nucleo, Peticion, Relacion, Reglas, Resultado, Respuesta};
use rutas::Rutas;
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};
use vision::{Mascara, Rect};

const MIN: Instante = MS_POR_MINUTO;
const U: Mil = 1_000;

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

fn reglas() -> Reglas {
    let mut r = Reglas::default();
    let b = &mut Arc::make_mut(&mut r.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    r
}

fn nucleo(m: &M) -> Nucleo {
    Nucleo::nuevo(Arranque { reglas: reglas(), ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) })
}

/// Un sitio verde y rico a unas decenas de unidades del campamento `i`.
fn sitio_rico(n: &Nucleo, m: &Mundo, i: usize, faccion: u32) -> Punto {
    let c = m.campamentos()[i].posicion.unwrap();
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

/// Un punto en tierra junto a `centro + (dx, dy)` (busca en espiral, de 2 en 2 unidades).
fn en_tierra(m: &Mundo, centro: Punto, dx: Mil, dy: Mil) -> Punto {
    let p = Punto::new(centro.x + dx, centro.y + dy);
    (0..20i64).flat_map(|r| (0..8i64).map(move |k| (r, k))).map(|(r, k)| {
        let a = (k as f64 * 45.0).to_radians();
        Punto::new(p.x + (r as f64 * 2_000.0 * a.cos()) as i64, p.y + (r as f64 * 2_000.0 * a.sin()) as i64)
    }).find(|q| m.transitable_en(q.x, q.y)).expect("hay tierra cerca")
}

struct Esc {
    n: Nucleo,
    peticiones: u64,
}

impl Esc {
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.peticiones += 1;
        let p = Peticion::nueva(actor, &format!("p{}", self.peticiones), c);
        let t = self.n.ahora();
        // 5I (5E-10): sin el andamiaje de `comun::mandaba`; cada cuenta da sus órdenes de plaza con la matriz real.
        self.n.aplicar_en(t, if actor == "admin" { p.de_admin() } else { p })
    }

    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
        r
    }

    /// Una plaza de `faccion` fundada por la cuenta `cuenta` en `p`, con la construcción automática parada.
    fn plaza(&mut self, cuenta: &str, faccion: u32, p: Punto) -> u64 {
        let t = self.n.ahora();
        let col = self.n.crear_ejercito(t, cuenta, faccion, p, 500, 100_000).unwrap();
        self.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
        let plaza = self.ok(cuenta, Comando::Fundar { ejercito: col }).creadas[0];
        self.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
        plaza
    }

    /// Una columna de `faccion` en `p` (a 0,5 u/s).
    fn columna(&mut self, cuenta: &str, faccion: u32, p: Punto) -> u64 {
        let t = self.n.ahora();
        self.n.crear_ejercito(t, cuenta, faccion, p, 500, 100_000).unwrap()
    }

    fn marchar(&mut self, cuenta: &str, ejercito: u64, destino: Punto) -> Plan {
        self.ok(cuenta, Comando::MarcharA { ejercito, destino });
        self.n.ejercito(ejercito).unwrap().plan.clone()
    }

    fn avanzar(&mut self, ms: Instante) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }

    fn explorado(&self, f: u32) -> Mascara {
        self.n.estado().memoria.exploracion.get(&f).cloned().unwrap_or_else(|| self.n.estado().memoria.vacia().unwrap())
    }
}

/// El último instante de `[desde, hasta]` en que el plan estaba a `≤ radio` de `p`, muestreando cada 50 ms (la referencia de fuerza bruta).
fn ultimo_visto(plan: &Plan, quieto: Punto, desde: Instante, hasta: Instante, p: Punto, radio: Mil) -> Option<Instante> {
    (0..=(hasta - desde) / 50).map(|k| hasta - k * 50).find(|&t| {
        let q = plan.posicion_en(t).unwrap_or(quieto);
        let (dx, dy) = ((q.x - p.x) as i128, (q.y - p.y) as i128);
        dx * dx + dy * dy <= radio as i128 * radio as i128
    })
}

/// Lo que se ve desde cada punto del plan entre dos instantes (muestreado cada 100 ms), con el alcance dado.
fn discos(m: &Mascara, plan: &Plan, desde: Instante, hasta: Instante, radio: Mil) -> Mascara {
    let mut d = Mascara::nueva(m.columnas(), m.filas(), m.tamano());
    for t in (desde..=hasta).step_by(100) {
        d.marcar_vision(plan.posicion_en(t).unwrap(), radio);
    }
    d
}

fn contiene(grande: &Mascara, pequena: &Mascara) -> bool {
    let mut u = grande.clone();
    u.fundir(pequena);
    &u == grande
}

/// Todo el mapa.
fn todo() -> Rect {
    Rect { min: Punto::new(0, 0), max: Punto::new(2_667_000, 2_000_000) }
}

/// Las entidades de la vista de `p` desde `viewer` (con lo que ya tiene el cliente), proyectadas: `(clase, id, cuerpo)`.
fn vista(n: &Nucleo, p: Perspectiva, viewer: Option<u64>, rect: Rect) -> Vec<(TipoEntidad, String, Cuerpo)> {
    let m = Mirada::de(n, p);
    m.contenido(viewer, rect, &BTreeMap::new()).into_iter().filter_map(|x| Some((TipoEntidad::try_from(x.clave.0).ok()?, x.clave.1.clone(), x.cuerpo()?))).collect()
}

fn de_vista(v: &[(TipoEntidad, String, Cuerpo)], tipo: TipoEntidad, id: &str) -> Option<Cuerpo> {
    v.iter().find(|(t, i, _)| *t == tipo && i == id).map(|(_, _, c)| c.clone())
}

// ───────────────────────────── 1. Lo que explora una columna ─────────────────────────────

#[test]
fn la_columna_graba_lo_que_barrio_su_plan_al_tocarla_y_hasta_entonces_la_vista_lo_suma_sin_grabarlo() {
    let m = con_mundo!();
    let mut e = Esc { n: nucleo(m), peticiones: 0 };
    let inicio = sitio_rico(&e.n, &m.0, 0, 1);
    let col = e.columna("ana", 1, inicio);
    let plan = e.marchar("ana", col, en_tierra(&m.0, inicio, 80 * U, 0));
    let (t0, fin) = (plan.vertices[0].en, plan.fin().unwrap());
    assert_eq!(e.n.estado().memoria.vigias[&col].desde, t0, "marchar toca la columna: graba desde ahí");
    let quieta = e.explorado(1);

    // A media marcha nada ha tocado la columna: la memoria sigue como estaba, pero la vista ya suma lo andado.
    e.avanzar((fin - t0) / 2);
    let mitad = e.n.ahora();
    assert_eq!(e.explorado(1), quieta, "lo andado sin tocarla no se graba");
    let celdas = |n: &Nucleo| {
        let f = Mirada::de(n, Perspectiva::Faccion(1)).niebla().cloned().expect("con rejilla");
        Mascara::desde_bytes(f.columnas as usize, f.filas as usize, i64::from(f.tamano_celda), &f.celdas)
    };
    let vista_mitad = celdas(&e.n);
    let radio = e.n.alcance_de_vista(col).unwrap();
    assert!(contiene(&vista_mitad, &discos(&quieta, &plan, t0, mitad, radio)), "la vista ya enseña lo que lleva andado");

    // Al llegar (la llegada la toca) queda grabado justo lo que barrió: lo que se veía desde cada punto, y nada a más de su alcance.
    e.avanzar(fin - mitad + MIN);
    let grabado = e.explorado(1);
    assert!(contiene(&grabado, &discos(&grabado, &plan, t0, fin, radio)), "todo lo visto por el camino queda explorado");
    assert!(contiene(&discos(&grabado, &plan, t0, fin, radio + 200), &grabado), "y nada que no se viera");
    assert!(contiene(&grabado, &vista_mitad), "lo que la vista sumaba es lo que se grabó");
    assert_eq!(celdas(&e.n), grabado, "ya grabado, la vista no suma nada más (la columna está quieta)");
    assert_eq!(e.n.estado().memoria.vigias[&col].desde, fin);
}

// ───────────────────────────── 2. Las fichas de lo ajeno ─────────────────────────────

#[test]
fn la_ficha_de_una_plaza_ajena_es_lo_ultimo_que_se_vio_de_ella_y_cuando_y_nunca_lo_que_cambio_despues() {
    let m = con_mundo!();
    let mut e = Esc { n: nucleo(m), peticiones: 0 };
    let sitio = sitio_rico(&e.n, &m.0, 0, 2);
    let plaza = e.plaza("gus", 2, sitio);
    let nivel_antes = e.n.estado().asentamientos[&plaza].nivel();
    // Una columna de F1 (un héroe solo: ve 15) pasa a unas 10 u de la plaza de F2 y se aleja.
    let col = e.columna("ana", 1, en_tierra(&m.0, sitio, -60 * U, 10 * U));
    e.avanzar(MIN);
    let plan = e.marchar("ana", col, en_tierra(&m.0, sitio, 70 * U, 10 * U));
    let (t0, fin) = (plan.vertices[0].en, plan.fin().unwrap());
    let radio = e.n.alcance_de_vista(col).unwrap();
    let visto = ultimo_visto(&plan, plan.vertices[0].p, t0, fin, sitio, radio).expect("el camino pasa a la vista de la plaza");
    assert!(visto < fin, "y la deja atrás");
    e.n.avanzar_hasta(visto + 30_000);

    // Mientras la columna anda, la plaza se ve primero (entera, como ficha avistada) y luego se recuerda: la vista la da como `FichaConocida` con lo que la columna vio, sin grabarlo aún.
    let as_ = nucleo::proyeccion::ids::asentamiento(plaza);
    let v = vista(&e.n, Perspectiva::Faccion(1), None, todo());
    assert!(de_vista(&v, TipoEntidad::Asentamiento, &as_).is_none(), "fuera del alcance ya no se ve");
    let Some(Cuerpo::FichaConocida(f)) = de_vista(&v, TipoEntidad::FichaConocida, &as_) else { panic!("se recuerda: {v:?}") };
    assert!((f.conocido_en_ms - visto).abs() <= 60, "vista por última vez al salir de su alcance: {} frente a {visto}", f.conocido_en_ms);
    assert_eq!((f.faccion_id.as_str(), f.nivel, f.tipo), ("f-2", nivel_antes, TipoEntidad::Asentamiento as i32));
    assert!(!e.n.estado().memoria.fichas.contains_key(&1), "lo andado no está grabado: nada tocó la columna");

    // La plaza sube de nivel sin que nadie de F1 la vea: al grabarse lo que vio la columna, la ficha conserva el nivel de antes.
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 3 });
    let ficha = e.n.estado().memoria.fichas[&1][&Sitio::Plaza(plaza)];
    assert_eq!(ficha.datos.nivel, nivel_antes, "lo que cambió después no se ve");
    assert!((ficha.conocido_en - visto).abs() <= 60, "{} frente a {visto}", ficha.conocido_en);
    assert_eq!(ficha.conocido_en, f.conocido_en_ms, "es lo que la vista enseñaba");
    let v = vista(&e.n, Perspectiva::Faccion(1), None, todo());
    let Some(Cuerpo::FichaConocida(f)) = de_vista(&v, TipoEntidad::FichaConocida, &as_) else { panic!() };
    assert_eq!(f.nivel, nivel_antes);

    // Al llegar, la llegada toca la columna y nada cambia en la ficha (no volvió a ver la plaza). Volver a pasar sí la pone al día.
    e.n.avanzar_hasta(fin + MIN);
    assert_eq!(e.n.estado().memoria.fichas[&1][&Sitio::Plaza(plaza)], ficha);
    let plan = e.marchar("ana", col, en_tierra(&m.0, sitio, 0, 12 * U));
    e.n.avanzar_hasta(plan.fin().unwrap() + 1_000);
    let v = vista(&e.n, Perspectiva::Faccion(1), None, todo());
    let Some(Cuerpo::Asentamiento(a)) = de_vista(&v, TipoEntidad::Asentamiento, &as_) else { panic!("a la vista otra vez") };
    assert_eq!(a.nivel, 3);
    assert!(de_vista(&v, TipoEntidad::FichaConocida, &as_).is_none(), "lo que se ve gana a lo que se recuerda: nunca las dos");
    assert_eq!(e.n.estado().memoria.fichas[&1][&Sitio::Plaza(plaza)].datos.nivel, 3, "la llegada grabó lo que vio");
}

// ───────────────────────────── 3. Sin bandera ─────────────────────────────

#[test]
fn sin_bandera_lo_explorado_es_del_heroe_y_pasa_a_su_faccion_cuando_entra_en_una() {
    let m = con_mundo!();
    let mut e = Esc { n: nucleo(m), peticiones: 0 };
    let h = e.ok("eva", Comando::CrearHeroe { cuenta: "eva".into(), campamento: 0, controlador: Controlador::Humano }).creadas[0];
    let col = e.ok("eva", Comando::SalirDelCampamento { campamento: 0, heroe: h, escuadras: vec![], carga: vec![], politica: militar::columna::Politica::Rechazar }).creadas[0];
    assert_eq!(e.n.ejercito(col).unwrap().columna.faccion, None, "un héroe sin Facción anda sin bandera");
    let p = e.n.ejercito(col).unwrap().posicion;
    let plan = e.marchar("eva", col, en_tierra(&m.0, p, 40 * U, 10 * U));
    e.n.avanzar_hasta(plan.fin().unwrap() + 1_000);
    let mem = &e.n.estado().memoria;
    let personal = mem.personal.get(&h).cloned().expect("lo explorado sin bandera es del héroe");
    assert!(personal.contar() > 0 && mem.exploracion.is_empty(), "y de ninguna Facción");
    let f = Mirada::de(&e.n, Perspectiva::Heroe(h)).niebla().cloned().unwrap();
    assert_eq!(f.celdas, personal.bytes(), "su vista es su niebla personal");

    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(3) });
    let mem = &e.n.estado().memoria;
    assert!(!mem.personal.contains_key(&h), "al entrar en una Facción se le borra lo suyo...");
    assert!(contiene(&mem.exploracion[&3], &personal), "...porque pasa a su Facción");
}

// ───────────────────────────── 4. Aliados ─────────────────────────────

#[test]
fn lo_que_ven_los_aliados_se_ve_pero_no_se_explora_y_desaparece_al_romper_la_alianza() {
    let m = con_mundo!();
    let mut e = Esc { n: nucleo(m), peticiones: 0 };
    let sitio = sitio_rico(&e.n, &m.0, 0, 1);
    e.plaza("ana", 1, sitio);
    // Lejos de F1, una columna de F2 (aliada) junto a una de F3.
    let lejos = en_tierra(&m.0, sitio, 300 * U, 0);
    let de_f2 = e.columna("beto", 2, lejos);
    let de_f3 = e.columna("gus", 3, en_tierra(&m.0, lejos, 8 * U, 0));
    e.avanzar(1_000);
    let ver = |n: &Nucleo| vista(n, Perspectiva::Faccion(1), None, todo());
    let ajenas = [nucleo::proyeccion::ids::ejercito(de_f2), nucleo::proyeccion::ids::ejercito(de_f3)];
    let ids_ = |v: &[(TipoEntidad, String, Cuerpo)]| v.iter().filter(|(t, i, _)| *t == TipoEntidad::Ejercito && ajenas.contains(i)).map(|(_, i, _)| i.clone()).collect::<Vec<_>>();
    assert!(ids_(&ver(&e.n)).is_empty(), "sin alianza, F1 no ve nada de eso");
    let explorado = e.explorado(1);

    e.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Alianza) });
    let v = ver(&e.n);
    let mut esperado = vec![nucleo::proyeccion::ids::ejercito(de_f2), nucleo::proyeccion::ids::ejercito(de_f3)];
    esperado.sort();
    let mut vistos = ids_(&v);
    vistos.sort();
    assert_eq!(vistos, esperado, "con alianza ve lo que ven los ojos aliados (y a ellos)");
    let Some(Cuerpo::Ejercito(x)) = de_vista(&v, TipoEntidad::Ejercito, &nucleo::proyeccion::ids::ejercito(de_f2)) else { panic!() };
    assert!(x.escuadron_ids.is_empty() && x.suministro.is_none(), "lo del aliado también es ajeno: redactado");
    let f = Mirada::de(&e.n, Perspectiva::Faccion(1)).niebla().cloned().unwrap();
    let visibles = Mascara::desde_bytes(f.columnas as usize, f.filas as usize, i64::from(f.tamano_celda), &f.visibles);
    assert!(visibles.explorado_en(lejos) && !Mascara::desde_bytes(f.columnas as usize, f.filas as usize, i64::from(f.tamano_celda), &f.celdas).explorado_en(lejos), "se ve pero no se explora");
    assert_eq!(e.explorado(1), explorado, "la memoria no aprende nada de los aliados");

    e.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: None });
    assert!(ids_(&ver(&e.n)).is_empty(), "al romperse, desaparece sin dejar recuerdo");
    e.ok("admin", Comando::FijarRelacion { a: 2, b: 1, relacion: Some(Relacion::Vasallaje) });
    assert_eq!(ids_(&ver(&e.n)).len(), 2, "el vasallaje también comparte la visión, en cualquier dirección");
}

// ───────────────────────────── 5. Lo ajeno, redactado ─────────────────────────────

#[test]
fn lo_ajeno_se_ve_redactado_y_el_tramo_ajeno_se_renueva_solo_cuando_le_quedan_menos_de_10_s_sin_revelar_el_destino() {
    let m = con_mundo!();
    let mut e = Esc { n: nucleo(m), peticiones: 0 };
    let sitio = sitio_rico(&e.n, &m.0, 0, 1);
    let plaza = e.plaza("ana", 1, sitio);
    let radio = e.n.estado().asentamientos[&plaza].radio;
    // Una columna de F2 cruza la zona de F1 de punta a punta: la plaza la ve mientras está en su disco.
    let col = e.columna("gus", 2, en_tierra(&m.0, sitio, -radio + 2 * U, 0));
    let h_gus = e.n.ejercito(col).unwrap().columna.lider;
    e.ok("admin", Comando::DarEscuadra { heroe: h_gus, tropa: militar::catalogo::Tropa::MiliciaLanceros, cantidad: 10 });
    let destino = en_tierra(&m.0, sitio, 200 * U, 0);
    let plan = e.marchar("gus", col, destino);
    e.avanzar(5_000);
    let t = e.n.ahora();
    let mirada = Mirada::de(&e.n, Perspectiva::Faccion(1));
    let contenido = mirada.contenido(None, todo(), &BTreeMap::new());
    let id = nucleo::proyeccion::ids::ejercito(col);
    let x = contenido.iter().find(|v| v.clave == (TipoEntidad::Ejercito as i32, id.clone())).expect("la plaza la ve");
    let Some(Cuerpo::Ejercito(av)) = x.cuerpo() else { panic!() };
    assert!(av.escuadron_ids.is_empty() && av.suministro.is_none() && av.viveres.is_none() && av.lider_id.is_empty() && av.persiguiendo.is_none(), "sin composición, carro, víveres ni Líder: {av:?}");
    assert_eq!((av.faccion_id.as_str(), av.participantes.len()), ("f-2", 1));
    let tramo = &av.plan.as_ref().expect("con su tramo").vertices;
    let hasta = tramo.last().unwrap().en_ms;
    assert!(hasta <= t + vision::radios::HORIZONTE_MS && hasta > t, "el tramo acaba en el horizonte: {hasta} desde {t}");
    assert!(tramo.iter().all(|v| v.p.as_ref().unwrap().x < destino.x - 100 * U), "y no revela el destino");
    assert_eq!(x.sello, Sello(e.n.ejercito(col).unwrap().version, hasta));
    // El héroe que va en ella, solo con lo público: sus escuadras (tropa, cantidad y nivel), no si es bot ni su campamento.
    let Some(Cuerpo::Heroe(hp)) = contenido.iter().find(|v| v.clave == (TipoEntidad::Heroe as i32, nucleo::proyeccion::ids::heroe(h_gus))).and_then(|v| v.cuerpo()) else { panic!("el héroe ajeno se ve") };
    assert_eq!((hp.jugador_id.clone(), hp.ubicacion.clone(), hp.escuadrones_ids.len(), hp.escuadras_que_lleva.len()), (None, None, 0, 1));
    assert_eq!(hp.escuadras_que_lleva[0].cantidad, 10);

    // Con el tramo que tiene el cliente aún le quedan más de 10 s: no se renueva (mismo sello). Cuando le quedan menos, sí, y sigue sin enseñar el destino.
    let tiene = BTreeMap::from([(x.clave.clone(), x.sello)]);
    drop(contenido);
    e.n.avanzar_hasta(hasta - vision::radios::RENOVAR_MS - 1);
    let mirada = Mirada::de(&e.n, Perspectiva::Faccion(1));
    let sello = |m: &Mirada<'_>| m.contenido(None, todo(), &tiene).into_iter().find(|v| v.clave.1 == id).map(|v| v.sello);
    assert_eq!(sello(&mirada), Some(Sello(e.n.ejercito(col).unwrap().version, hasta)), "le quedan más de 10 s: sigue con el que tiene");
    e.n.avanzar_hasta(hasta - vision::radios::RENOVAR_MS + 1);
    let mirada = Mirada::de(&e.n, Perspectiva::Faccion(1));
    let nuevo = sello(&mirada).expect("sigue a la vista");
    assert!(nuevo.1 > hasta && nuevo.1 <= e.n.ahora() + vision::radios::HORIZONTE_MS, "el tramo nuevo empieza ahora y llega al horizonte: {nuevo:?}");
    assert!(nuevo.1 < plan.fin().unwrap());

    // La plaza ajena y la caravana ajena: lo que se ve desde fuera, nada más.
    let v = vista(&e.n, Perspectiva::Faccion(2), None, todo());
    assert!(de_vista(&v, TipoEntidad::Asentamiento, &nucleo::proyeccion::ids::asentamiento(plaza)).is_some_and(|c| matches!(c, Cuerpo::Asentamiento(a) if a.almacen.is_empty() && a.poblacion.is_none() && a.pueblos_ids.is_empty() && a.trazado.is_none() && a.edificios.iter().all(|b| b.estado == pb::EstadoEdificio::EdificioActivo as i32) && !a.zona.is_empty())), "la columna de F2 ve la plaza de F1 como ficha con su zona: {v:?}");
}

#[test]
fn el_tramo_ajeno_se_recorta_al_alcance_de_los_ojos_y_no_al_rectangulo_de_la_vista() {
    let m = con_mundo!();
    let mut e = Esc { n: nucleo(m), peticiones: 0 };
    let sitio = sitio_rico(&e.n, &m.0, 0, 1);
    let plaza = e.plaza("ana", 1, sitio);
    let radio = e.n.estado().asentamientos[&plaza].radio;
    // Una columna de F2 junto al borde de lo que ve F1, saliendo hacia fuera: deja de verse mucho antes del horizonte.
    let col = e.columna("gus", 2, en_tierra(&m.0, sitio, radio - 2 * U, 0));
    e.marchar("gus", col, en_tierra(&m.0, sitio, radio + 100 * U, 0));
    e.avanzar(500);
    let t = e.n.ahora();
    let id = nucleo::proyeccion::ids::ejercito(col);
    let Some(Cuerpo::Ejercito(av)) = de_vista(&vista(&e.n, Perspectiva::Faccion(1), None, todo()), TipoEntidad::Ejercito, &id) else { panic!("F1 la ve") };
    let fin = av.plan.as_ref().expect("con su tramo").vertices.last().cloned().unwrap();
    let (p, plan) = (fin.p.unwrap(), e.n.ejercito(col).unwrap().plan.clone());
    let mirada = Mirada::de(&e.n, Perspectiva::Faccion(1));
    assert!(fin.en_ms > t && fin.en_ms < t + vision::radios::HORIZONTE_MS, "acaba antes del horizonte, aunque el rectángulo sea el mapa entero: {} desde {t}", fin.en_ms);
    assert!(mirada.ve(Punto::new(p.x, p.y)), "lo último del tramo aún se ve: {p:?}");
    assert!(!mirada.ve(plan.posicion_en(fin.en_ms + 1_000).unwrap()), "un segundo después del tramo, la columna ya está fuera del alcance de todos los ojos");
}

// ───────────────────────────── 6. La foto y el determinismo ─────────────────────────────

/// Un escenario con marchas, una plaza ajena y un héroe sin bandera, con las mismas órdenes a las mismas horas.
fn partida(m: &M, sesiones: bool, trozos: i64) -> Nucleo {
    let mut e = Esc { n: nucleo(m), peticiones: 0 };
    if sesiones {
        e.n.seguir_pertenencias();
    }
    let sitio = sitio_rico(&e.n, &m.0, 0, 2);
    e.plaza("gus", 2, sitio);
    let col = e.columna("ana", 1, en_tierra(&m.0, sitio, -60 * U, 10 * U));
    let h = e.ok("eva", Comando::CrearHeroe { cuenta: "eva".into(), campamento: 0, controlador: Controlador::Humano }).creadas[0];
    let sola = e.ok("eva", Comando::SalirDelCampamento { campamento: 0, heroe: h, escuadras: vec![], carga: vec![], politica: militar::columna::Politica::Rechazar }).creadas[0];
    let p = e.n.ejercito(sola).unwrap().posicion;
    for (k, destino) in [(0, en_tierra(&m.0, sitio, 70 * U, 10 * U)), (1, en_tierra(&m.0, sitio, 0, -40 * U)), (2, en_tierra(&m.0, sitio, -60 * U, 0))].into_iter() {
        e.marchar("ana", col, destino);
        e.marchar("eva", sola, en_tierra(&m.0, p, (k + 1) * 20 * U, 0));
        let fin = e.n.ahora() + 10 * MIN;
        for s in 1..=trozos {
            let t = e.n.ahora() + (fin - e.n.ahora()) * s / trozos;
            e.n.avanzar_hasta(t);
        }
    }
    e.n
}

#[test]
fn la_memoria_es_la_misma_con_sesiones_y_sin_ellas_y_avanzando_a_trozos_y_viaja_en_la_foto_ban23() {
    let m = con_mundo!();
    let a = partida(m, false, 1);
    let (b, c) = (partida(m, true, 1), partida(m, false, 7));
    let rastro = |n: &Nucleo| (n.version(), n.hash_estado());
    assert_eq!(rastro(&a), rastro(&b), "con sesiones, la misma versión y el mismo hash");
    assert_eq!(rastro(&a), rastro(&c), "avanzando a trozos, también");
    let mem = &a.estado().memoria;
    assert!(mem.exploracion.contains_key(&1) && mem.fichas.contains_key(&1) && !mem.personal.is_empty() && !mem.vigias.is_empty(), "la partida graba de todo");

    let bytes = a.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN40");
    let vuelta = Estado::deserializar(&bytes, &reglas(), m.0.hash()).expect("la foto vuelve");
    assert_eq!(vuelta.memoria, *mem);
    assert_eq!(vuelta.serializar(), bytes);
    let vieja = [b"BAN22".as_slice(), &bytes[5..]].concat();
    let Err(e) = Estado::deserializar(&vieja, &reglas(), m.0.hash()) else { panic!("BAN22 no debe admitirse") };
    assert!(e.to_string().contains("BAN22") && e.to_string().contains("niebla"), "{e}");
    let vieja = [b"BAN23".as_slice(), &bytes[5..]].concat();
    let Err(e) = Estado::deserializar(&vieja, &reglas(), m.0.hash()) else { panic!("BAN23 no debe admitirse") };
    assert!(e.to_string().contains("BAN23") && e.to_string().contains("LAYOUT_VERSION 3"), "{e}");
    let vieja = [b"BAN25".as_slice(), &bytes[5..]].concat();
    let Err(e) = Estado::deserializar(&vieja, &reglas(), m.0.hash()) else { panic!("BAN25 no debe admitirse") };
    assert!(e.to_string().contains("BAN25") && e.to_string().contains("LAYOUT_VERSION 4"), "{e}");
    let vieja = [b"BAN28".as_slice(), &bytes[5..]].concat();
    let Err(e) = Estado::deserializar(&vieja, &reglas(), m.0.hash()) else { panic!("BAN28 no debe admitirse") };
    assert!(e.to_string().contains("BAN28") && e.to_string().contains("LAYOUT_VERSION 5"), "{e}");
}

#[test]
fn recuperar_la_foto_y_el_registro_reproduce_la_memoria_aunque_las_columnas_anden_sin_grabar() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let arranque = || Arranque { reglas: reglas(), ajustes: Ajustes { foto_cada: 3, ..Ajustes::default() }, semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) };
    let n = Nucleo::con_persistencia(arranque(), dir.path(), Default::default()).unwrap();
    let sitio = sitio_rico(&n, &m.0, 0, 2);
    let mut e = Esc { n, peticiones: 0 };
    e.plaza("gus", 2, sitio);
    let col = e.columna("ana", 1, en_tierra(&m.0, sitio, -60 * U, 10 * U));
    e.marchar("ana", col, en_tierra(&m.0, sitio, 70 * U, 10 * U));
    e.avanzar(2 * MIN);
    // A media marcha: lo andado no está grabado; tras recuperar, la columna sigue sin grabarlo y lo graba igual al tocarla.
    let mut n = e.n;
    let (t, antes) = (n.ahora(), (n.version(), n.hash_estado()));
    n.apagar().unwrap();
    drop(n);
    let mut r = Nucleo::con_persistencia(arranque(), dir.path(), Default::default()).unwrap();
    assert_eq!((r.version(), r.hash_estado(), r.ahora()), (antes.0, antes.1, t), "la recuperación deja el mismo estado");
    assert!(r.aplicar_en(t, Peticion::nueva("ana", "parar", Comando::Estacionar { ejercito: col })).aceptada());
    let viva = partida_hasta_estacionar(m, sitio, col);
    assert_eq!((r.version(), r.hash_estado()), (viva.version(), viva.hash_estado()), "grabar tras recuperar da lo mismo que en el proceso vivo");
    assert!(r.estado().memoria.fichas[&1].contains_key(&Sitio::Plaza(viva.estado().asentamientos.keys().copied().next().unwrap())), "y graba la plaza que vio");
}

/// La misma partida que la de la recuperación, sin parar el proceso: hasta estacionar la columna en el mismo instante.
fn partida_hasta_estacionar(m: &M, sitio: Punto, col: u64) -> Nucleo {
    let dir = tempfile::tempdir().unwrap();
    let arranque = Arranque { reglas: reglas(), ajustes: Ajustes { foto_cada: 3, ..Ajustes::default() }, semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) };
    let mut e = Esc { n: Nucleo::con_persistencia(arranque, dir.path(), Default::default()).unwrap(), peticiones: 0 };
    e.plaza("gus", 2, sitio);
    let c = e.columna("ana", 1, en_tierra(&m.0, sitio, -60 * U, 10 * U));
    assert_eq!(c, col);
    e.marchar("ana", col, en_tierra(&m.0, sitio, 70 * U, 10 * U));
    e.avanzar(2 * MIN);
    let t = e.n.ahora();
    assert!(e.n.aplicar_en(t, Peticion::nueva("ana", "parar", Comando::Estacionar { ejercito: col })).aceptada());
    e.n
}

// ───────────────────────────── 6. Lo que el núcleo ya sabía (4H T7) ─────────────────────────────

/// El área (con signo, en milésimas al cuadrado) de un lazo.
fn area(puntos: &[pb::Punto]) -> i128 {
    (0..puntos.len()).map(|i| (puntos[i].x as i128) * (puntos[(i + 1) % puntos.len()].y as i128) - (puntos[i].y as i128) * (puntos[(i + 1) % puntos.len()].x as i128)).sum::<i128>() / 2
}

/// La vista de una Facción (4H T7) lleva su zona **fundida** (dos plazas suyas que se solapan dan un solo lazo, más pequeño que la suma de sus zonas), de quién es el suelo bajo cada columna
/// propia (`territorio_faccion_id`: la Facción de la zona que la contiene aunque no se vea su plaza, o vacío en tierra de nadie) y, al héroe cuya columna está a la puerta de una plaza propia, el
/// almacén de esa plaza (lo demás de su interior, no).
#[test]
fn la_vista_lleva_la_zona_fundida_el_suelo_bajo_cada_columna_propia_y_el_almacen_de_la_plaza_a_cuya_puerta_se_esta() {
    let m = con_mundo!();
    let mut e = Esc { n: nucleo(m), peticiones: 0 };
    let sa = sitio_rico(&e.n, &m.0, 0, 1);
    let a = e.plaza("ana", 1, sa);
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 5 });
    // La segunda plaza de F1, en el sitio fundable más cercano; las dos zonas crecen hasta solaparse.
    let sb = (1..60).flat_map(|r| (0..36).map(move |g| (r, g))).map(|(r, g)| {
        let ang = ((g * 10) as f64).to_radians();
        Punto::new(sa.x + (r as f64 * 1_000.0 * ang.cos()) as i64, sa.y + (r as f64 * 1_000.0 * ang.sin()) as i64)
    }).find(|&p| e.n.viabilidad(p, 1).is_some_and(|v| v.motivo.is_none())).expect("hay un sitio fundable cerca");
    let b = e.plaza("beto", 1, sb);
    for x in [a, b] {
        e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: x, nivel: 4 });
    }
    let solapan = |n: &Nucleo| n.estado().asentamientos[&a].radio + n.estado().asentamientos[&b].radio > sa.dist(sb) + 2 * U;
    for _ in 0..60 {
        if solapan(&e.n) {
            break;
        }
        e.ok("admin", Comando::AgrandarZona { asentamiento: a });
        e.ok("admin", Comando::AgrandarZona { asentamiento: b });
    }
    let (ra, rb) = (e.n.estado().asentamientos[&a].radio, e.n.estado().asentamientos[&b].radio);
    assert!(solapan(&e.n), "las dos zonas de F1 se solapan: {ra} + {rb} frente a {}", sa.dist(sb));
    let c = e.plaza("gus", 2, sitio_rico(&e.n, &m.0, 3, 2));
    let sc = e.n.estado().asentamientos[&c].posicion;
    // Tres columnas de F1: a la puerta de A, dentro de la zona de C (de F2) y en tierra de nadie.
    let puerta = e.columna("ana", 1, en_tierra(&m.0, sa, U, 0));
    let en_f2 = e.columna("ana", 1, en_tierra(&m.0, sc, 3 * U, 0));
    let lejos = (0..m.0.campamentos().len()).map(|i| sitio_rico(&e.n, &m.0, i, 1)).find(|&p| e.n.faccion_del_suelo(&[p])[0].is_none()).expect("un sitio de nadie");
    let nadie = e.columna("ana", 1, lejos);
    let heroe_de = |n: &Nucleo, col: u64| n.ejercito(col).unwrap().columna.lider;

    let v = vista(&e.n, Perspectiva::Faccion(1), Some(heroe_de(&e.n, puerta)), todo());
    let Some(Cuerpo::ZonaFaccion(z)) = de_vista(&v, TipoEntidad::ZonaFaccion, "f-1") else { panic!("la zona de F1") };
    let (za, zb) = (e.n.poligono_zona(a), e.n.poligono_zona(b));
    let pb_ = |v: Vec<Punto>| v.into_iter().map(|p| pb::Punto { x: p.x, y: p.y }).collect::<Vec<_>>();
    let (area_a, area_b) = (area(&pb_(za)).abs(), area(&pb_(zb)).abs());
    assert_eq!(z.contornos.len(), 1, "un solo lazo");
    let union = area(&z.contornos[0].puntos);
    assert!(union > area_a.max(area_b) && union < area_a + area_b, "la unión ({union}) es más que cada zona ({area_a}, {area_b}) y menos que su suma");
    let suelo = |col: u64| match de_vista(&v, TipoEntidad::Ejercito, &nucleo::proyeccion::ids::ejercito(col)) {
        Some(Cuerpo::Ejercito(x)) => x.territorio_faccion_id,
        otro => panic!("{otro:?}"),
    };
    assert_eq!((suelo(puerta), suelo(en_f2), suelo(nadie)), (Some("f-1".into()), Some("f-2".into()), Some(String::new())));
    let as_a = nucleo::proyeccion::ids::asentamiento(a);
    let Some(Cuerpo::Asentamiento(ficha)) = de_vista(&v, TipoEntidad::Asentamiento, &as_a) else { panic!("la plaza A") };
    assert_eq!(ficha.almacen.len(), asentamiento::recurso::TODOS.len(), "a la puerta, su almacén");
    assert!(ficha.edificios.iter().all(|x| x.estado == pb::EstadoEdificio::EdificioActivo as i32) && ficha.trazado.is_none(), "y nada más del interior");
    let otro = vista(&e.n, Perspectiva::Faccion(1), Some(heroe_de(&e.n, en_f2)), todo());
    let Some(Cuerpo::Asentamiento(ficha)) = de_vista(&otro, TipoEntidad::Asentamiento, &as_a) else { panic!() };
    assert!(ficha.almacen.is_empty(), "lejos de la puerta, solo la ficha");
    // Columnas ajenas: sin suelo.
    let de_f2 = vista(&e.n, Perspectiva::Faccion(2), None, todo());
    assert!(de_f2.iter().all(|(t, _, c)| *t != TipoEntidad::Ejercito || !matches!(c, Cuerpo::Ejercito(x) if x.faccion_id == "f-1" && x.territorio_faccion_id.is_some())));
}
