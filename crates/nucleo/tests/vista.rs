//! El ámbito `VISTA` con niebla (4H T3d) por el servidor WebSocket real y el actor del núcleo: la `Foto` con su `NieblaProyectada`, lo propio entero y lo ajeno redactado, la columna ajena que
//! entra en la vista, se ve con su tramo de 30 s (que el muestreo renueva cuando le quedan menos de 10, sin enseñar nunca el destino) y sale; la plaza que se ve y luego se recuerda como ficha; lo
//! que ven los aliados, que aparece y desaparece con la relación; el filtro del rectángulo; la vista sin Facción (la niebla personal del héroe) y la reanudación (la vista no se reanuda: se abre
//! con `CambiarVista`). Cada prueba acaba con «foto + deltas = lo que proyecta el núcleo».
//!
//! El mundo se prepara en el núcleo con el reloj **parado**: el tiempo solo anda cuando la administración lo avanza (`controlar_reloj avanzar_hasta`), así que cada paso se puede contar; la prueba del
//! muestreo pone el reloj en marcha, acelerado, y espera a que el actor renueve el tramo por su cuenta. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); se omiten si no está.

mod comun;

use base::{Instante, Mil, Punto, MS_POR_MINUTO};
use comun::sesion::*;
use contrato::v1::{self as pb, entidad::Cuerpo, mensaje_de_cliente as mc, mensaje_de_servidor as ms, orden, TipoCliente, TipoEntidad};
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::proyeccion::ids;
use nucleo::proyeccion::vista::{self, Mirada, Perspectiva};
use nucleo::{AccionReloj, Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Resultado};
use prost_types::{value::Kind, Struct, Value};
use rutas::Rutas;
use servidor::prueba::{hola, ClientePrueba};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};
use vision::{Mascara, Rect};

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

/// Un punto en tierra junto a `centro + (dx, dy)`.
fn en_tierra(m: &Mundo, centro: Punto, dx: Mil, dy: Mil) -> Punto {
    let p = Punto::new(centro.x + dx, centro.y + dy);
    (0..100i64)
        .flat_map(|r| (0..8i64).map(move |k| (r, k)))
        .map(|(r, k)| {
            let a = (k as f64 * 45.0).to_radians();
            Punto::new(p.x + (r as f64 * 2_000.0 * a.cos()) as i64, p.y + (r as f64 * 2_000.0 * a.sin()) as i64)
        })
        .find(|q| m.transitable_en(q.x, q.y))
        .expect("hay tierra cerca")
}

/// La plaza de F1 (de nivel 3, con su zona de 20 u) con `ana` dentro; una columna de F2 (`gus`) a 12 u de su frontera; una columna de F3 (`nora`) lejos; `dora`, sin Facción, en el campamento
/// 0. El reloj, parado.
struct Esc {
    n: Nucleo,
    plaza: u64,
    centro: Punto,
    radio: Mil,
    ana: u64,
    gus: u64,
    nora: u64,
    dora: u64,
    peticiones: u64,
}

impl Esc {
    fn ok(&mut self, actor: &str, c: Comando) -> nucleo::Respuesta {
        self.peticiones += 1;
        let p = Peticion::nueva(actor, &format!("prep{}", self.peticiones), c);
        let t = self.n.ahora();
        let r = self.n.aplicar_en(t, if actor == "admin" { p.de_admin() } else { p });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
        r
    }
}

fn escenario(m: &M) -> Esc {
    let mut reglas = Reglas::default();
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let n = Nucleo::nuevo(Arranque { reglas, ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
    let mut x = Esc { n, plaza: 0, centro: Punto::new(0, 0), radio: 0, ana: 0, gus: 0, nora: 0, dora: 0, peticiones: 0 };
    x.centro = sitio_rico(&x.n, &m.0, 0, 1);
    let col = x.n.crear_ejercito(0, "fundador", 1, x.centro, 500, 100_000).unwrap();
    x.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
    x.plaza = x.ok("fundador", Comando::Fundar { ejercito: col }).creadas[0];
    let plaza = x.plaza;
    x.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
    for r in [asentamiento::Recurso::Madera, asentamiento::Recurso::Piedra, asentamiento::Recurso::Oro, asentamiento::Recurso::Trigo] {
        x.ok("admin", Comando::DarRecursos { asentamiento: plaza, recurso: r, cantidad: 300_000 });
    }
    x.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 3 });
    for _ in 0..14 {
        x.ok("admin", Comando::AgrandarZona { asentamiento: plaza });
    }
    x.radio = x.n.estado().asentamientos[&plaza].radio;
    x.ana = x.ok("ana", Comando::CrearHeroe { cuenta: "ana".into(), campamento: 0, controlador: Controlador::Humano }).creadas[0];
    let ana = x.ana;
    x.ok("admin", Comando::FijarFaccionDeHeroe { heroe: ana, faccion: Some(1) });
    x.ok("admin", Comando::FijarResidencia { heroe: ana, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    x.ok("admin", Comando::ColocarHeroe { heroe: ana, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
    let (centro, radio) = (x.centro, x.radio);
    x.gus = x.n.crear_ejercito(x.n.ahora(), "gus", 2, en_tierra(&m.0, centro, -(radio + 12 * U), 0), 500, 100_000).unwrap();
    x.nora = x.n.crear_ejercito(x.n.ahora(), "nora", 3, en_tierra(&m.0, centro, 0, radio + 150 * U), 500, 100_000).unwrap();
    x.dora = x.ok("dora", Comando::CrearHeroe { cuenta: "dora".into(), campamento: 0, controlador: Controlador::Humano }).creadas[0];
    let t = x.n.ahora();
    x.n.avanzar_hasta((t / MS_POR_MINUTO + 1) * MS_POR_MINUTO + 1_000);
    x.ok("admin", Comando::Reloj(AccionReloj::Pausar));
    x
}

// ───────────────────────────── Lectura de lo que llega ─────────────────────────────

fn ambito(a: &Option<pb::Ambito>) -> String {
    let a = a.as_ref().expect("un ámbito");
    format!("{}:{}", pb::AmbitoTipo::try_from(a.tipo).unwrap().as_str_name().trim_start_matches("AMBITO_"), a.id)
}

fn texto(x: &str) -> Value {
    Value { kind: Some(Kind::StringValue(x.into())) }
}

fn admin(codigo: &str, parametros: &[(&str, &str)]) -> orden::Cuerpo {
    orden::Cuerpo::Administracion(pb::OrdenDeAdministracion { codigo: codigo.into(), parametros: Some(Struct { fields: parametros.iter().map(|(k, v)| ((*k).to_owned(), texto(v))).collect() }) })
}

fn rect(a: Punto, b: Punto) -> mc::Cuerpo {
    mc::Cuerpo::Vista(pb::CambiarVista { rectangulo: Some(pb::Rectangulo { min: Some(pb::Punto { x: a.x, y: a.y }), max: Some(pb::Punto { x: b.x, y: b.y }) }) })
}

fn todo() -> Rect {
    Rect { min: Punto::new(0, 0), max: Punto::new(2_667_000, 2_000_000) }
}

fn marchar(heroe: u64, p: Punto) -> orden::Cuerpo {
    use pb::orden_marchar_a::{objetivo::Tipo, Objetivo};
    orden::Cuerpo::MarcharA(pb::OrdenMarcharA { heroe_id: ids::heroe(heroe), objetivo: Some(Objetivo { tipo: Some(Tipo::Punto(pb::Punto { x: p.x, y: p.y })) }) })
}

/// Lo que ya estaba en camino hacia la sesión (una orden rechazada por su forma sale después de todo lo anterior).
async fn pendiente(c: &mut ClientePrueba) -> Vec<ms::Cuerpo> {
    let (previos, r) = dar_todo(c, "barrera", orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: "ej-999999".into() })).await;
    assert_eq!(rechazo(&r), "ejercito.desconocido");
    previos
}

/// El almacén del cliente de un ámbito: lo que le dejan la `Foto` y las `Delta`, por `(clase, id)`, y la última niebla.
#[derive(Default)]
struct Cliente {
    entidades: BTreeMap<(i32, String), Cuerpo>,
    niebla: Option<pb::NieblaProyectada>,
    version: u64,
}

impl Cliente {
    /// Aplica lo que llegó del ámbito `buscado` (lo de los demás ámbitos se ignora). Devuelve las `Delta` aplicadas.
    fn aplicar<'a>(&mut self, mensajes: &'a [ms::Cuerpo], buscado: &str) -> Vec<&'a pb::Delta> {
        let mut deltas = Vec::new();
        for m in mensajes {
            match m {
                ms::Cuerpo::Foto(f) if ambito(&f.ambito) == buscado => {
                    self.entidades = f.entidades.iter().filter_map(|e| e.cuerpo.clone()).map(|c| ((tipo_de(&c) as i32, id_de(&c)), c)).collect();
                    self.niebla = f.niebla.clone();
                    self.version = f.version;
                }
                ms::Cuerpo::Delta(d) if ambito(&d.ambito) == buscado => {
                    assert_eq!(d.desde, self.version, "cada delta empieza donde acabó la anterior");
                    for c in &d.cambios {
                        let clave = (c.entidad, c.id.clone());
                        if c.tipo == pb::TipoCambio::CambioQuita as i32 {
                            assert!(self.entidades.remove(&clave).is_some(), "QUITA de lo que el cliente no tenía: {clave:?}");
                        } else {
                            assert!(c.reemplaza, "en la vista todo viaja completo");
                            self.entidades.insert(clave, c.datos.clone().and_then(|e| e.cuerpo).expect("ACTUALIZA con datos"));
                        }
                    }
                    if let Some(x) = &d.niebla {
                        vista::aplicar_niebla(&mut self.niebla, x);
                    }
                    self.version = d.hasta;
                    deltas.push(d);
                }
                _ => {}
            }
        }
        deltas
    }

    fn get(&self, tipo: TipoEntidad, id: &str) -> Option<&Cuerpo> {
        self.entidades.get(&(tipo as i32, id.to_owned()))
    }
}

fn tipo_de(c: &Cuerpo) -> TipoEntidad {
    match c {
        Cuerpo::Ejercito(_) => TipoEntidad::Ejercito,
        Cuerpo::Caravana(_) => TipoEntidad::Caravana,
        Cuerpo::Asentamiento(_) => TipoEntidad::Asentamiento,
        Cuerpo::Pueblo(_) => TipoEntidad::Pueblo,
        Cuerpo::Avanzada(_) => TipoEntidad::Avanzada,
        Cuerpo::Obra(_) => TipoEntidad::ObraDeMapa,
        Cuerpo::FichaConocida(_) => TipoEntidad::FichaConocida,
        Cuerpo::CampamentoBandidos(_) => TipoEntidad::CampamentoBandidos,
        Cuerpo::CampamentoMercenarios(_) => TipoEntidad::CampamentoMercenarios,
        Cuerpo::Batalla(_) => TipoEntidad::Batalla,
        Cuerpo::Heroe(_) => TipoEntidad::Heroe,
        Cuerpo::ZonaFaccion(_) => TipoEntidad::ZonaFaccion,
        Cuerpo::RedCaminos(_) => TipoEntidad::RedCaminos,
        otro => panic!("la vista no lleva {otro:?}"),
    }
}

fn id_de(c: &Cuerpo) -> String {
    match c {
        Cuerpo::Ejercito(x) => x.id.clone(),
        Cuerpo::Caravana(x) => x.id.clone(),
        Cuerpo::Asentamiento(x) => x.id.clone(),
        Cuerpo::Pueblo(x) => x.id.clone(),
        Cuerpo::Avanzada(x) => x.id.clone(),
        Cuerpo::Obra(x) => x.id.clone(),
        Cuerpo::FichaConocida(x) => x.asentamiento_id.clone(),
        Cuerpo::CampamentoBandidos(x) => x.id.clone(),
        Cuerpo::CampamentoMercenarios(x) => x.id.clone(),
        Cuerpo::Batalla(x) => x.id.clone(),
        Cuerpo::Heroe(x) => x.id.clone(),
        Cuerpo::ZonaFaccion(x) => x.faccion_id.clone(),
        Cuerpo::RedCaminos(_) => "red".into(),
        otro => panic!("la vista no lleva {otro:?}"),
    }
}

/// Lo que el núcleo proyecta para esa vista ahora mismo: debe ser lo que el cliente acumuló. De lo ajeno que anda, el tramo y el sitio son los del instante en que se mandó (el cliente sigue con
/// ese tramo mientras le dure), así que no se comparan.
fn proyeccion(n: &Nucleo, p: Perspectiva, viewer: u64, r: Rect) -> (BTreeMap<(i32, String), Cuerpo>, Option<pb::NieblaProyectada>) {
    let m = Mirada::de(n, p);
    let entidades = m.contenido(Some(viewer), r, &BTreeMap::new()).into_iter().filter_map(|x| Some((x.clave.clone(), x.cuerpo()?))).collect();
    (sin_tramos(entidades), m.niebla().cloned())
}

fn sin_tramos(mut e: BTreeMap<(i32, String), Cuerpo>) -> BTreeMap<(i32, String), Cuerpo> {
    for c in e.values_mut() {
        match c {
            Cuerpo::Ejercito(x) if x.lider_id.is_empty() => (x.plan, x.posicion) = (None, None),
            Cuerpo::Caravana(x) if x.contenido.is_none() => (x.plan, x.posicion) = (None, None),
            _ => {}
        }
    }
    e
}

impl Cliente {
    /// Lo que tiene, para compararlo con la proyección.
    fn ahora(&self) -> (BTreeMap<(i32, String), Cuerpo>, Option<pb::NieblaProyectada>) {
        (sin_tramos(self.entidades.clone()), self.niebla.clone())
    }
}

fn mascara(f: &pb::NieblaProyectada, visibles: bool) -> Mascara {
    Mascara::desde_bytes(f.columnas as usize, f.filas as usize, i64::from(f.tamano_celda), if visibles { &f.visibles } else { &f.celdas })
}

/// Avanza el mundo (parado) hasta `t` con la administración.
async fn avanzar(admin_c: &mut ClientePrueba, n: &mut u64, t: Instante) {
    *n += 1;
    let (_, r) = dar_todo(admin_c, &format!("reloj{n}"), orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: "avanzar_hasta".into(), factor_milesimas: None, hasta_ms: Some(t) })).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
}

// ───────────────────────────── 1. Una columna ajena entra en la vista, anda con su tramo y sale ─────────────────────────────

#[tokio::test]
async fn la_columna_ajena_entra_en_la_vista_redactada_con_un_tramo_que_se_renueva_sin_revelar_el_destino_y_sale_y_la_plaza_vista_queda_como_ficha() {
    let m = con_mundo!();
    let x = escenario(m);
    let (plaza, centro, radio, ana, gus_col, t0) = (x.plaza, x.centro, x.radio, x.ana, x.gus, x.n.ahora());
    let gus = x.n.ejercito(gus_col).unwrap().columna.lider;
    let as_ = ids::asentamiento(plaza);
    let ej = ids::ejercito(gus_col);
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (mut ana_c, _, _) = w.conectar(hola("ok:ana", TipoCliente::ClienteUnity)).await;
    let (mut gus_c, _, _) = w.conectar(hola("ok:gus", TipoCliente::ClienteUnity)).await;

    // La foto de la vista de F1: su plaza (como en su Facción), su zona y la niebla (la plaza ha explorado su disco); nada de F2, que está fuera de todo ojo de F1.
    ana_c.enviar(rect(Punto::new(0, 0), Punto::new(2_667_000, 2_000_000))).await;
    let mut de_ana = Cliente::default();
    de_ana.aplicar(&[ana_c.esperar().await.cuerpo.unwrap()], "VISTA:f-1");
    assert!(de_ana.get(TipoEntidad::Asentamiento, &as_).is_some() && de_ana.get(TipoEntidad::ZonaFaccion, "f-1").is_some());
    assert!(de_ana.get(TipoEntidad::Ejercito, &ej).is_none(), "la columna de F2 está fuera de la vista de F1");
    let niebla = de_ana.niebla.clone().expect("la foto de la vista trae la niebla");
    assert!(mascara(&niebla, false).explorado_en(centro) && mascara(&niebla, true).explorado_en(Punto::new(centro.x + radio - 1_000, centro.y)));
    assert!(!mascara(&niebla, true).explorado_en(Punto::new(centro.x - radio - 12 * U, centro.y)), "donde está gus no se ve");
    gus_c.enviar(rect(Punto::new(0, 0), Punto::new(2_667_000, 2_000_000))).await;
    let mut de_gus = Cliente::default();
    de_gus.aplicar(&[gus_c.esperar().await.cuerpo.unwrap()], "VISTA:f-2");
    assert!(de_gus.get(TipoEntidad::Ejercito, &ej).is_some(), "su columna, entera");
    assert!(de_gus.get(TipoEntidad::Asentamiento, &as_).is_none() && de_gus.get(TipoEntidad::FichaConocida, &as_).is_none(), "nunca vio la plaza de F1");

    // Gus cruza la zona de F1 de oeste a este y sigue 40 u más allá: el mundo avanza a pasos de 2 s.
    let destino = en_tierra(&m.0, centro, radio + 40 * U, 0);
    let (previos, r) = dar_todo(&mut gus_c, "g1", marchar(gus, destino)).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    de_gus.aplicar(&previos, "VISTA:f-2");
    let (mut t, mut pasos) = (t0, 0);
    let (mut entro, mut salio, mut tramos) = (None::<Instante>, None::<Instante>, Vec::<(Instante, Instante)>::new());
    let (mut gus_vio, mut gus_recuerda) = (false, false);
    while t < t0 + 10 * MS_POR_MINUTO && !(salio.is_some() && gus_recuerda) {
        t += 2_000;
        avanzar(&mut admin_c, &mut pasos, t).await;
        for d in de_ana.aplicar(&pendiente(&mut ana_c).await, "VISTA:f-1") {
            for c in d.cambios.iter().filter(|c| c.id == ej) {
                if c.tipo == pb::TipoCambio::CambioQuita as i32 {
                    salio.get_or_insert(t);
                    continue;
                }
                let Some(Cuerpo::Ejercito(av)) = c.datos.as_ref().and_then(|e| e.cuerpo.as_ref()) else { panic!() };
                entro.get_or_insert(t);
                assert!(av.escuadron_ids.is_empty() && av.suministro.is_none() && av.viveres.is_none() && av.lider_id.is_empty() && av.peticiones_de_union.is_empty(), "redactada: {av:?}");
                let plan = av.plan.as_ref().expect("una columna que anda trae su tramo");
                let (desde, hasta) = (plan.vertices[0].en_ms, plan.vertices.last().unwrap().en_ms);
                assert!(desde >= t - 2_000 && hasta <= t + vision::radios::HORIZONTE_MS, "el tramo empieza ahora y no pasa del horizonte: [{desde}, {hasta}] en {t}");
                assert!(plan.vertices.iter().all(|v| v.p.as_ref().unwrap().x < destino.x), "nunca el destino");
                tramos.push((desde, hasta));
            }
        }
        for d in de_gus.aplicar(&pendiente(&mut gus_c).await, "VISTA:f-2") {
            gus_vio |= d.cambios.iter().any(|c| c.id == as_ && c.entidad == TipoEntidad::Asentamiento as i32 && c.tipo == pb::TipoCambio::CambioActualiza as i32);
            gus_recuerda |= gus_vio && d.cambios.iter().any(|c| c.id == as_ && c.entidad == TipoEntidad::FichaConocida as i32);
        }
    }
    let (entro, salio) = (entro.expect("la columna entra en la vista de F1"), salio.expect("y sale"));
    assert!(entro < salio && tramos.len() >= 2, "anduvo por la vista el tiempo de renovar su tramo: {tramos:?}");
    for w in tramos.windows(2) {
        assert!(w[1].0 >= w[0].1 - vision::radios::RENOVAR_MS && w[1].0 <= w[0].1, "cada tramo se renueva cuando al anterior le quedan menos de 10 s: {tramos:?}");
    }
    assert!(gus_vio && gus_recuerda, "gus vio la plaza de F1 al cruzarla y, al dejarla atrás, la recuerda como ficha");
    let Some(Cuerpo::FichaConocida(f)) = de_gus.get(TipoEntidad::FichaConocida, &as_) else { panic!() };
    assert!(de_gus.get(TipoEntidad::Asentamiento, &as_).is_none(), "nunca las dos a la vez");
    assert_eq!((f.faccion_id.as_str(), f.tipo), ("f-1", TipoEntidad::Asentamiento as i32));
    let explorado = mascara(de_gus.niebla.as_ref().unwrap(), false);
    assert!(explorado.explorado_en(centro), "lo que anduvo queda explorado en la niebla de gus");

    // Foto + deltas = lo que proyecta el núcleo, para los dos.
    pendiente(&mut ana_c).await;
    pendiente(&mut gus_c).await;
    let n = w.parar().await;
    let rect_todo = todo();
    assert_eq!(de_ana.ahora(), proyeccion(&n, Perspectiva::Faccion(1), ana, rect_todo), "ana: foto + deltas = proyección");
    assert_eq!(de_gus.ahora(), proyeccion(&n, Perspectiva::Faccion(2), gus, rect_todo), "gus: foto + deltas = proyección");
}

// ───────────────────────────── 2. El muestreo de 250 ms renueva el tramo solo ─────────────────────────────

#[tokio::test]
async fn con_el_reloj_en_marcha_el_muestreo_renueva_el_tramo_ajeno_sin_ninguna_orden() {
    let m = con_mundo!();
    let mut x = escenario(m);
    let (centro, radio, gus_col) = (x.centro, x.radio, x.gus);
    // Gus ya dentro de la zona de F1, a punto de cruzarla despacio de oeste a este.
    let destino = en_tierra(&m.0, centro, radio + 40 * U, 0);
    x.ok("gus", Comando::MarcharA { ejercito: gus_col, destino });
    let entra = x.n.ejercito(gus_col).unwrap().plan.vertices.iter().find(|v| (v.p.x - centro.x).abs() < radio - 3 * U).map(|v| v.en).expect("cruza la zona");
    x.n.avanzar_hasta(entra);
    let ej = ids::ejercito(gus_col);
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (mut ana_c, _, _) = w.conectar(hola("ok:ana", TipoCliente::ClienteUnity)).await;
    ana_c.enviar(rect(Punto::new(0, 0), Punto::new(2_667_000, 2_000_000))).await;
    let mut de_ana = Cliente::default();
    de_ana.aplicar(&[ana_c.esperar().await.cuerpo.unwrap()], "VISTA:f-1");
    let Some(Cuerpo::Ejercito(av)) = de_ana.get(TipoEntidad::Ejercito, &ej).cloned() else { panic!("la columna de gus está a la vista") };
    let primero = av.plan.unwrap().vertices.last().unwrap().en_ms;

    // El reloj en marcha, veinte veces más deprisa: sin orden ninguna, el actor manda el tramo siguiente antes de que se acabe este.
    let (_, r) = dar_todo(&mut admin_c, "factor", orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: "fijar_factor".into(), factor_milesimas: Some(20_000), hasta_ms: None })).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let (_, r) = dar_todo(&mut admin_c, "marcha", orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: "reanudar".into(), factor_milesimas: None, hasta_ms: None })).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let renovado = loop {
        let m = ana_c.esperar().await.cuerpo.expect("un mensaje");
        let deltas = de_ana.aplicar(std::slice::from_ref(&m), "VISTA:f-1");
        let nuevo = deltas.iter().flat_map(|d| &d.cambios).filter(|c| c.id == ej).find_map(|c| match c.datos.as_ref().and_then(|e| e.cuerpo.as_ref()) {
            Some(Cuerpo::Ejercito(x)) => x.plan.as_ref().map(|p| (p.vertices[0].en_ms, p.vertices.last().unwrap().en_ms)),
            _ => None,
        });
        if let Some(t) = nuevo {
            break t;
        }
    };
    assert!(renovado.1 > primero && renovado.0 >= primero - vision::radios::RENOVAR_MS - 250 && renovado.0 <= primero, "el tramo nuevo sale cuando al viejo le quedan menos de 10 s (± un muestreo): {renovado:?} tras {primero}");
    assert!(renovado.1 - renovado.0 <= vision::radios::HORIZONTE_MS);
    w.parar().await;
}

/// Lee lo que llegue durante `ms` de pared y lo descarta (avisos del reloj y lo que el muestreo tuviera pendiente).
async fn vaciar(c: &mut ClientePrueba, ms: u64) {
    while tokio::time::timeout(std::time::Duration::from_millis(ms), c.recibir()).await.is_ok() {}
}

#[tokio::test]
async fn tras_una_orden_solo_la_vista_de_quien_la_da_se_pone_al_dia_y_las_demas_en_el_muestreo() {
    let m = con_mundo!();
    let mut x = escenario(m);
    let (centro, radio, gus_col) = (x.centro, x.radio, x.gus);
    let gus = x.n.ejercito(gus_col).unwrap().columna.lider;
    // Gus dentro de la zona de F1: Ana lo ve.
    let dentro = en_tierra(&m.0, centro, -radio + 5 * U, 0);
    x.ok("gus", Comando::MarcharA { ejercito: gus_col, destino: dentro });
    let llega = x.n.ejercito(gus_col).unwrap().plan.fin().unwrap();
    x.n.avanzar_hasta(llega);
    let ej = ids::ejercito(gus_col);
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (mut ana_c, _, _) = w.conectar(hola("ok:ana", TipoCliente::ClienteUnity)).await;
    let (mut gus_c, _, _) = w.conectar(hola("ok:gus", TipoCliente::ClienteUnity)).await;
    let todo = rect(Punto::new(0, 0), Punto::new(2_667_000, 2_000_000));
    for c in [&mut ana_c, &mut gus_c] {
        c.enviar(todo.clone()).await;
        c.esperar().await;
    }
    // El reloj en marcha, pero tan despacio (×0,001) que no llega ningún muestreo de 250 ms de mundo durante la prueba.
    for (id, accion, factor) in [("lento", "fijar_factor", Some(1)), ("marcha", "reanudar", None)] {
        let (_, r) = dar_todo(&mut admin_c, id, orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: accion.into(), factor_milesimas: factor, hasta_ms: None })).await;
        assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    }
    vaciar(&mut ana_c, 300).await;
    vaciar(&mut gus_c, 300).await;
    // Gus cambia de destino: su vista se pone al día antes del resultado de su orden...
    let (previos, r) = dar_todo(&mut gus_c, "g1", marchar(gus, en_tierra(&m.0, centro, radio + 40 * U, 0))).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let es_la_columna = |d: &pb::Delta| d.cambios.iter().any(|c| c.id == ej);
    assert!(previos.iter().any(|m| matches!(m, ms::Cuerpo::Delta(d) if ambito(&d.ambito) == "VISTA:f-2" && es_la_columna(d))), "la vista de quien da la orden, al momento");
    // ...y la de Ana espera al muestreo: con el reloj casi quieto, nada llega.
    silencio(&mut ana_c).await;
    // Con el reloj a ×1, el muestreo siguiente le trae la columna con su plan nuevo.
    let (_, r) = dar_todo(&mut admin_c, "normal", orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: "fijar_factor".into(), factor_milesimas: Some(1_000), hasta_ms: None })).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    loop {
        match ana_c.esperar().await.cuerpo {
            Some(ms::Cuerpo::Delta(d)) if ambito(&d.ambito) == "VISTA:f-1" && es_la_columna(&d) => break,
            Some(_) => {}
            None => panic!("sin mensaje"),
        }
    }
    w.parar().await;
}

// ───────────────────────────── 3. Aliados, rectángulo, sin Facción y reanudación ─────────────────────────────

#[tokio::test]
async fn la_vision_de_los_aliados_aparece_y_desaparece_con_la_relacion_y_solo_llega_lo_del_rectangulo() {
    let m = con_mundo!();
    let x = escenario(m);
    let (ana, nora_col, gus_col, x_plaza) = (x.ana, x.nora, x.gus, x.plaza);
    let nora_pos = x.n.ejercito(nora_col).unwrap().posicion;
    let (ej_nora, ej_gus) = (ids::ejercito(nora_col), ids::ejercito(gus_col));
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (mut ana_c, _, _) = w.conectar(hola("ok:ana", TipoCliente::ClienteUnity)).await;
    // Ana mira solo los alrededores de nora: nada que ver.
    let alrededor = (Punto::new(nora_pos.x - 50 * U, nora_pos.y - 50 * U), Punto::new(nora_pos.x + 50 * U, nora_pos.y + 50 * U));
    ana_c.enviar(rect(alrededor.0, alrededor.1)).await;
    let mut de_ana = Cliente::default();
    de_ana.aplicar(&[ana_c.esperar().await.cuerpo.unwrap()], "VISTA:f-1");
    assert!(de_ana.get(TipoEntidad::Ejercito, &ej_nora).is_none());
    let antes = de_ana.niebla.clone().unwrap();

    // La intel de la taberna llega por el contrato: en su plaza no hay Taberna.
    let mirada = orden::Cuerpo::ComprarMirada(pb::OrdenComprarMirada { origen: Some(pb::orden_comprar_mirada::Origen { tipo: "asentamiento".into(), id: ids::asentamiento(x_plaza) }), centro: Some(pb::Punto { x: nora_pos.x, y: nora_pos.y }) });
    let (_, r) = dar_todo(&mut ana_c, "mirada", mirada).await;
    assert_eq!(rechazo(&r), "intel.sin_taberna");

    // F1 y F3 se alían: a ana le llega la columna de nora (lo que ven sus ojos) y su vista crece sin explorar nada.
    let (previos, r) = dar_todo(&mut admin_c, "alianza", admin("fijar_relacion", &[("faccion_a_id", "f-1"), ("faccion_b_id", "f-3"), ("relacion", "alianza")])).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{previos:?}");
    de_ana.aplicar(&pendiente(&mut ana_c).await, "VISTA:f-1");
    let Some(Cuerpo::Ejercito(av)) = de_ana.get(TipoEntidad::Ejercito, &ej_nora) else { panic!("la columna aliada se ve") };
    assert!(av.escuadron_ids.is_empty(), "redactada: es ajena");
    let ahora = de_ana.niebla.clone().unwrap();
    assert_eq!(ahora.celdas, antes.celdas, "lo que ven los aliados no se explora");
    assert!(mascara(&ahora, true).explorado_en(nora_pos) && !mascara(&antes, true).explorado_en(nora_pos), "pero se ve");
    assert!(de_ana.get(TipoEntidad::Ejercito, &ej_gus).is_none(), "gus cae fuera del rectángulo de ana");

    // Se rompe la alianza: desaparece sin dejar recuerdo.
    let (_, r) = dar_todo(&mut admin_c, "ruptura", admin("fijar_relacion", &[("faccion_a_id", "f-1"), ("faccion_b_id", "f-3")])).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let llegado = pendiente(&mut ana_c).await;
    let deltas = de_ana.aplicar(&llegado, "VISTA:f-1");
    assert!(deltas.iter().flat_map(|d| &d.cambios).any(|c| c.id == ej_nora && c.tipo == pb::TipoCambio::CambioQuita as i32));
    assert_eq!(de_ana.niebla.as_ref().unwrap().visibles, antes.visibles);
    pendiente(&mut ana_c).await;
    let n = w.parar().await;
    assert_eq!(de_ana.ahora(), proyeccion(&n, Perspectiva::Faccion(1), ana, Rect { min: alrededor.0, max: alrededor.1 }));
}

#[tokio::test]
async fn sin_faccion_la_vista_es_la_del_heroe_con_su_niebla_personal_y_al_reconectar_no_se_reanuda_se_vuelve_a_pedir() {
    let m = con_mundo!();
    let x = escenario(m);
    let dora = x.dora;
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (mut dora_c, b, _) = w.conectar(hola("ok:dora", TipoCliente::ClienteUnity)).await;
    assert!(b.faccion_id.is_none(), "dora no tiene Facción");
    let (mut t, mut pasos) = (b.reloj.unwrap().mundo_ms, 0);
    dora_c.enviar(rect(Punto::new(0, 0), Punto::new(2_667_000, 2_000_000))).await;
    let mut de_dora = Cliente::default();
    let vista_dora = format!("VISTA:{}", ids::heroe(dora));
    let m0 = dora_c.esperar().await.cuerpo.unwrap();
    de_dora.aplicar(std::slice::from_ref(&m0), &vista_dora);
    assert_eq!(de_dora.version, if let ms::Cuerpo::Foto(f) = &m0 { f.version } else { panic!("se esperaba la foto de {vista_dora}: {m0:?}") });
    assert!(de_dora.get(TipoEntidad::CampamentoMercenarios, &ids::campamento(0)).is_some(), "ve el campamento donde está");
    assert!(mascara(de_dora.niebla.as_ref().unwrap(), false).contar() == 0, "sin bandera no hay memoria de Facción y aún no ha andado");

    // Sale al mapa y anda: su niebla personal crece, y es suya (ninguna Facción la tiene).
    let (previos, r) = dar_todo(&mut dora_c, "salir", orden::Cuerpo::SalirDelCampamento(pb::OrdenSalirDelCampamento { campamento_id: ids::campamento(0), heroe_id: ids::heroe(dora), ..Default::default() })).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    de_dora.aplicar(&previos, &vista_dora);
    let col = r.creadas.first().map(|c| c.id.clone()).expect("su columna");
    let puerta = m.0.campamentos()[0].posicion.unwrap();
    let (previos, r) = dar_todo(&mut dora_c, "andar", marchar(dora, en_tierra(&m.0, Punto::new(puerta.x, puerta.y), 30 * U, 0))).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    de_dora.aplicar(&previos, &vista_dora);
    for _ in 0..30 {
        t += 2_000;
        avanzar(&mut admin_c, &mut pasos, t).await;
    }
    de_dora.aplicar(&pendiente(&mut dora_c).await, &vista_dora);
    assert!(de_dora.get(TipoEntidad::Ejercito, &col).is_some(), "su columna, entera, en su vista");

    // Al reconectar, la vista no se reanuda (ni aunque diga conocerla): la vuelve a pedir y la foto nueva es lo que tenía.
    drop(dora_c);
    let mut h = hola("ok:dora", TipoCliente::ClienteUnity);
    h.versiones_conocidas.insert(vista_dora.clone(), de_dora.version);
    let (mut dora_c, b, _) = w.conectar(h).await;
    assert!(b.ambitos.iter().all(|a| ambito(&a.ambito) != vista_dora), "la vista no viaja en la bienvenida");
    dora_c.enviar(rect(Punto::new(0, 0), Punto::new(2_667_000, 2_000_000))).await;
    let mut otra = Cliente::default();
    otra.aplicar(&[dora_c.esperar().await.cuerpo.unwrap()], &vista_dora);
    assert_eq!(otra.ahora(), de_dora.ahora(), "la foto nueva es lo que acumuló");
    pendiente(&mut dora_c).await;
    let n = w.parar().await;
    assert_eq!(otra.ahora(), proyeccion(&n, Perspectiva::Heroe(dora), dora, todo()));
    assert!(n.estado().memoria.personal.contains_key(&dora) || n.estado().memoria.vigias.values().any(|v| v.desde > 0), "lo que explora es suyo (grabado o pendiente de su columna)");
}

// ───────────────────────────── Coste ─────────────────────────────

/// Cuánto cuesta lo que hace el muestreo de las vistas (una `Mirada` por perspectiva y el contenido de cada sesión) en el mundo real con nueve plazas que crecen y 300 columnas de 30 Facciones
/// que marchan: `cargo test --release -p nucleo --test vista coste -- --ignored --nocapture`.
#[test]
#[ignore = "medición"]
fn coste_de_la_vista_con_300_columnas_y_30_facciones() {
    let m = con_mundo!();
    let mut reglas = Reglas::default();
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let mut n = Nucleo::nuevo(Arranque { reglas, ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
    let mut rng = base::Rng::nuevo(11);
    let campamentos: Vec<Punto> = m.0.campamentos().iter().map(|c| c.posicion.map(|p| Punto::new(p.x, p.y)).unwrap()).collect();
    for i in 0..campamentos.len() {
        let f = i as u32 + 1;
        let sitio = sitio_rico(&n, &m.0, i, f);
        let col = n.crear_ejercito(0, &format!("c{}", f - 1), f, sitio, 500, 100_000).unwrap();
        assert!(n.aplicar_en(0, Peticion::nueva("admin", &format!("fc{i}"), Comando::DarCaravanaFundacion { ejercito: col }).de_admin()).aceptada());
        assert!(n.aplicar_en(0, Peticion::nueva(&format!("c{}", f - 1), &format!("f{i}"), Comando::Fundar { ejercito: col })).aceptada());
    }
    let mut cols = Vec::new();
    for i in 0..300u32 {
        let c = campamentos[i as usize % campamentos.len()];
        let p = loop {
            let p = Punto::new(c.x + rng.por_debajo(300_000) as i64 - 150_000, c.y + rng.por_debajo(300_000) as i64 - 150_000);
            if m.0.transitable_en(p.x, p.y) {
                break p;
            }
        };
        cols.push((n.crear_ejercito(0, &format!("c{}", i % 30), i % 30 + 1, p, 500, 100_000).unwrap(), c, format!("c{}", i % 30)));
    }
    for (k, (col, c, cuenta)) in cols.iter().enumerate() {
        let d = Punto::new(c.x + rng.por_debajo(300_000) as i64 - 150_000, c.y + rng.por_debajo(300_000) as i64 - 150_000);
        n.aplicar_en(0, Peticion::nueva(cuenta, &format!("o{k}"), Comando::MarcharA { ejercito: *col, destino: d }));
    }
    n.avanzar_hasta(30 * MS_POR_MINUTO);
    let rect = todo();
    let medir = |veces: u32, f: &dyn Fn()| {
        let i0 = std::time::Instant::now();
        for _ in 0..veces {
            f();
        }
        i0.elapsed().as_secs_f64() * 1e6 / f64::from(veces)
    };
    let de = medir(20, &|| drop(Mirada::de(&n, Perspectiva::Faccion(1))));
    let mirada = Mirada::de(&n, Perspectiva::Faccion(1));
    let vacio = BTreeMap::new();
    let sellos = medir(50, &|| drop(mirada.contenido(None, rect, &vacio)));
    let todo_ = medir(20, &|| drop(mirada.contenido(None, rect, &vacio).iter().filter_map(|x| x.cuerpo()).collect::<Vec<_>>()));
    let cuantas = mirada.contenido(None, rect, &vacio).len();
    let ronda = medir(3, &|| (1..=30).for_each(|f| drop(Mirada::de(&n, Perspectiva::Faccion(f)).contenido(None, rect, &vacio))));
    println!("Mirada::de (una Facción): {de:.0} µs; contenido con sellos: {sellos:.0} µs ({cuantas} entidades); con cuerpos: {todo_:.0} µs; una ronda de 30 perspectivas: {:.1} ms", ronda / 1000.0);
}
