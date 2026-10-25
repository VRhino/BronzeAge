//! El ámbito `ADMIN` (4H T3e) por el servidor WebSocket real y el actor del núcleo: la administración ve todo el mundo sin niebla ni redacción (las columnas y los héroes ajenos enteros, cada
//! plaza con su interior, las Facciones, sus relaciones, los campamentos de mercenarios y de bandidos) y todo lo que cambia (la columna que marcha con su plan hasta el destino, la relación que
//! se fija y se quita, el paso de minuto de cada plaza como cambio parcial con máscara, los hechos); solo una sesión de administración abre `ADMIN`; la reanudación por versión (con la secuencia
//! interior de cada plaza) y la foto cuando el registro no alcanza; y la versión y el hash del mundo, iguales con una sesión de administración y sin ninguna. Que nunca le llegue un token de
//! batalla lo prueba `batalla_unity.rs`. Cada prueba que acumula deltas acaba con «foto + deltas = foto nueva».
//!
//! El mundo se prepara en el núcleo con el reloj **parado**: el tiempo solo anda cuando la administración lo avanza (`controlar_reloj avanzar_hasta`). Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`); se omiten si no está.

mod comun;

use base::{Instante, Mil, Punto, MS_POR_MINUTO};
use comun::sesion::*;
use contrato::v1::{self as pb, entidad::Cuerpo, mensaje_de_cliente as mc, mensaje_de_servidor as ms, orden, DecisionReanudacion as D, TipoCambio, TipoCliente, TipoEntidad};
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::proyeccion::propios::{self, Espectador};
use nucleo::proyeccion::{ids, interior};
use nucleo::{AccionReloj, Ajustes, Arranque, Clase, Comando, Nucleo, Peticion, Reglas, Resultado};
use prost_types::{value::Kind, Struct, Value};
use rutas::Rutas;
use servidor::prueba::{hola, ClientePrueba};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

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

/// La plaza de F1 (nivel 3) con `ana` dentro; una columna de F2 (`gus`) a 12 u de su frontera y una de F3 (`nora`) lejos, en guerra entre ellas; `dora`, sin Facción, en el campamento 0; un
/// campamento de bandidos lejos de todos. El reloj, parado.
struct Esc {
    n: Nucleo,
    plaza: u64,
    centro: Punto,
    radio: Mil,
    ana: u64,
    gus: u64,
    nora: u64,
    dora: u64,
    bandidos: u64,
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

fn escenario(m: &M, ajustes: Ajustes) -> Esc {
    let mut reglas = Reglas::default();
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let n = Nucleo::nuevo(Arranque { reglas, ajustes, semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
    let mut x = Esc { n, plaza: 0, centro: Punto::new(0, 0), radio: 0, ana: 0, gus: 0, nora: 0, dora: 0, bandidos: 0, peticiones: 0 };
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
    x.radio = x.n.estado().asentamientos[&plaza].radio;
    x.ana = x.ok("ana", Comando::CrearHeroe { cuenta: "ana".into(), campamento: 0, controlador: Controlador::Humano }).creadas[0];
    let ana = x.ana;
    x.ok("admin", Comando::FijarFaccionDeHeroe { heroe: ana, faccion: Some(1) });
    x.ok("admin", Comando::FijarResidencia { heroe: ana, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    x.ok("admin", Comando::ColocarHeroe { heroe: ana, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
    x.ok("admin", Comando::DarEscuadra { heroe: ana, tropa: Tropa::MiliciaLanceros, cantidad: 10 });
    let (centro, radio) = (x.centro, x.radio);
    x.gus = x.n.crear_ejercito(x.n.ahora(), "gus", 2, en_tierra(&m.0, centro, -(radio + 12 * U), 0), 500, 100_000).unwrap();
    x.nora = x.n.crear_ejercito(x.n.ahora(), "nora", 3, en_tierra(&m.0, centro, 0, radio + 150 * U), 500, 100_000).unwrap();
    x.ok("admin", Comando::FijarRelacion { a: 2, b: 3, relacion: Some(nucleo::Relacion::Guerra) });
    x.dora = x.ok("dora", Comando::CrearHeroe { cuenta: "dora".into(), campamento: 0, controlador: Controlador::Humano }).creadas[0];
    x.bandidos = x.ok("admin", Comando::ColocarBandidos { posicion: en_tierra(&m.0, centro, radio + 300 * U, 0), nivel: 2 }).creadas[0];
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

fn dar_madera(plaza: u64, cantidad: i64) -> orden::Cuerpo {
    let campos = [("asentamiento_id", texto(&ids::asentamiento(plaza))), ("recurso", texto("madera")), ("cantidad", Value { kind: Some(Kind::NumberValue(cantidad as f64)) })];
    orden::Cuerpo::Administracion(pb::OrdenDeAdministracion { codigo: "dar_recursos".into(), parametros: Some(Struct { fields: campos.into_iter().map(|(k, v)| (k.to_owned(), v)).collect() }) })
}

fn marchar(heroe: u64, p: Punto) -> orden::Cuerpo {
    use pb::orden_marchar_a::{objetivo::Tipo, Objetivo};
    orden::Cuerpo::MarcharA(pb::OrdenMarcharA { heroe_id: ids::heroe(heroe), objetivo: Some(Objetivo { tipo: Some(Tipo::Punto(pb::Punto { x: p.x, y: p.y })) }) })
}

fn reloj_hasta(t: Instante) -> orden::Cuerpo {
    orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: "avanzar_hasta".into(), factor_milesimas: None, hasta_ms: Some(t) })
}

fn aceptada(r: &pb::ResultadoOrden) {
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
}

/// Lo que ya estaba en camino hacia la sesión (una orden rechazada por el núcleo sale después de todo lo anterior).
async fn pendiente(c: &mut ClientePrueba) -> Vec<ms::Cuerpo> {
    let (previos, r) = dar_todo(c, "barrera", orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: "ej-999999".into() })).await;
    assert_eq!(rechazo(&r), "ejercito.desconocido");
    previos
}

fn hola_con(cliente: &str, tipo: TipoCliente, conocidas: &[(&str, u64)], interiores: &[(&str, u64)]) -> pb::Hola {
    let mut h = hola(&format!("ok:{cliente}"), tipo);
    h.versiones_conocidas.extend(conocidas.iter().map(|(k, v)| ((*k).to_owned(), *v)));
    h.versiones_interiores_conocidas.extend(interiores.iter().map(|(k, v)| ((*k).to_owned(), *v)));
    h
}

/// La clave del almacén del cliente de una entidad del contrato: su clase y su id.
fn clave(c: &Cuerpo) -> (i32, String) {
    let (t, id) = match c {
        Cuerpo::Ejercito(x) => (TipoEntidad::Ejercito, &x.id),
        Cuerpo::Asentamiento(x) => (TipoEntidad::Asentamiento, &x.id),
        Cuerpo::Faccion(x) => (TipoEntidad::Faccion, &x.id),
        Cuerpo::Tecnologias(x) => (TipoEntidad::TecnologiaFaccion, &x.faccion_id),
        Cuerpo::Heroe(x) => (TipoEntidad::Heroe, &x.id),
        Cuerpo::InformeInspeccion(x) => (TipoEntidad::InformeInspeccion, &x.id),
        Cuerpo::Escuadron(x) => (TipoEntidad::Escuadron, &x.id),
        Cuerpo::Caravana(x) => (TipoEntidad::Caravana, &x.id),
        Cuerpo::Convocatoria(x) => (TipoEntidad::Convocatoria, &x.id),
        Cuerpo::AcuerdoTrueque(x) => (TipoEntidad::AcuerdoTrueque, &x.id),
        Cuerpo::OrdenMercado(x) => (TipoEntidad::OrdenMercado, &x.id),
        Cuerpo::Batalla(x) => (TipoEntidad::Batalla, &x.id),
        Cuerpo::Relacion(x) => (TipoEntidad::RelacionPolitica, &x.id),
        Cuerpo::Pueblo(x) => (TipoEntidad::Pueblo, &x.id),
        Cuerpo::Obra(x) => (TipoEntidad::ObraDeMapa, &x.id),
        Cuerpo::Avanzada(x) => (TipoEntidad::Avanzada, &x.id),
        Cuerpo::CampamentoMercenarios(x) => (TipoEntidad::CampamentoMercenarios, &x.id),
        Cuerpo::CampamentoBandidos(x) => (TipoEntidad::CampamentoBandidos, &x.id),
        Cuerpo::Titulo(x) => (TipoEntidad::Titulo, &x.titulo_id),
        otro => panic!("ADMIN no lleva {otro:?}"),
    };
    (t as i32, id.clone())
}

/// El almacén del cliente de administración: lo que le dejan la `Foto` y las `Delta` de `ADMIN` (la plaza parcial se aplica con su máscara, como hace el cliente).
#[derive(Default, PartialEq, Debug)]
struct Almacen {
    entidades: BTreeMap<(i32, String), Cuerpo>,
    version: u64,
}

impl Almacen {
    /// Aplica lo que llegó de `ADMIN` (lo de otros ámbitos no debe llegar: falla). Devuelve las `Delta` aplicadas.
    fn aplicar<'a>(&mut self, mensajes: &'a [ms::Cuerpo]) -> Vec<&'a pb::Delta> {
        let mut deltas = Vec::new();
        for m in mensajes {
            match m {
                ms::Cuerpo::Foto(f) => {
                    assert_eq!(ambito(&f.ambito), "ADMIN:");
                    self.entidades = f.entidades.iter().filter_map(|e| e.cuerpo.clone()).map(|c| (clave(&c), c)).collect();
                    self.version = f.version;
                }
                ms::Cuerpo::Delta(d) => {
                    assert_eq!(ambito(&d.ambito), "ADMIN:");
                    assert_eq!(d.desde, self.version, "cada delta empieza donde acabó la anterior");
                    for c in &d.cambios {
                        let k = (c.entidad, c.id.clone());
                        let datos = c.datos.clone().and_then(|e| e.cuerpo);
                        if c.tipo == TipoCambio::CambioQuita as i32 {
                            assert!(self.entidades.remove(&k).is_some(), "QUITA de lo que el cliente no tenía: {k:?}");
                        } else if c.reemplaza {
                            self.entidades.insert(k, datos.expect("ACTUALIZA con datos"));
                        } else {
                            let (Some(Cuerpo::Asentamiento(destino)), Some(Cuerpo::Asentamiento(parcial))) = (self.entidades.get_mut(&k), datos) else { panic!("solo la plaza viaja parcial: {k:?}") };
                            interior::aplicar(destino, &parcial, &c.campos);
                        }
                    }
                    self.version = d.hasta;
                    deltas.push(d);
                }
                ms::Cuerpo::Revocado(r) => panic!("la administración no pierde nunca `ADMIN`: {r:?}"),
                _ => {}
            }
        }
        deltas
    }

    fn get(&self, tipo: TipoEntidad, id: &str) -> Option<&Cuerpo> {
        self.entidades.get(&(tipo as i32, id.to_owned()))
    }

    fn de(&self, tipo: TipoEntidad) -> usize {
        self.entidades.keys().filter(|(t, _)| *t == tipo as i32).count()
    }

    /// El id de la relación activa entre `f-a` y `f-b`, en cualquier sentido (desde 5D, `rel-N`).
    fn relacion_entre(&self, a: u32, b: u32) -> Option<String> {
        let (fa, fb) = (ids::faccion(a), ids::faccion(b));
        self.entidades.iter().find_map(|((_, id), c)| match c {
            Cuerpo::Relacion(r) if (r.faccion_a_id == fa && r.faccion_b_id == fb) || (r.faccion_a_id == fb && r.faccion_b_id == fa) => Some(id.clone()),
            _ => None,
        })
    }
}

/// La foto de `ADMIN` que recibe una sesión de administración nueva.
async fn foto_nueva(w: &Servicio, cuenta: &str) -> Almacen {
    let (c, b, mensajes) = w.conectar(hola_con(cuenta, TipoCliente::ClienteAdminWeb, &[], &[])).await;
    assert_eq!(b.ambitos.iter().map(|a| ambito(&a.ambito)).collect::<Vec<_>>(), ["ADMIN:"]);
    let mut a = Almacen::default();
    a.aplicar(&mensajes);
    c.cerrar().await;
    a
}

// ───────────────────────────── 1. Todo, sin niebla ni redacción, y lo que cambia ─────────────────────────────

#[tokio::test]
async fn la_administracion_ve_todo_el_mundo_sin_niebla_ni_redaccion_y_todo_lo_que_cambia() {
    let m = con_mundo!();
    let x = escenario(m, Ajustes::default());
    let (plaza, as_, centro, radio, gus_col, dora, t0) = (x.plaza, ids::asentamiento(x.plaza), x.centro, x.radio, x.gus, x.dora, x.n.ahora());
    let gus = x.n.ejercito(gus_col).unwrap().columna.lider;
    let ej_gus = ids::ejercito(gus_col);
    // Lo que el núcleo proyecta para la administración antes de arrancar (el reloj está parado: nada cambia hasta que se mueva).
    let plaza_entera = interior::asentamiento(&x.n, plaza).unwrap();
    let columnas = [gus_col, x.nora].map(|c| propios::ejercito(&x.n, x.n.ejercito(c).unwrap()));
    let heroes = [x.ana, x.dora, gus].map(|h| propios::heroe(&x.n, x.n.heroe(h).unwrap()));
    let escuadra = x.n.heroe(x.ana).unwrap().heroe.escuadras.iter().map(|s| propios::escuadron(&x.n, s)).next().expect("la escuadra de ana");
    let bandidos = propios::campamento_bandidos(&x.n, x.n.campamento_de_bandidos(x.bandidos).unwrap());
    let campamentos = m.0.campamentos().len();
    let w = arrancar_con(x.n).await;
    let (mut admin_c, b, mensajes) = w.conectar(hola_con("admin", TipoCliente::ClienteAdminWeb, &[], &[])).await;
    assert_eq!(b.ambitos.iter().map(|a| ambito(&a.ambito)).collect::<Vec<_>>(), ["ADMIN:"]);
    assert!(b.heroe_id.is_none() && b.faccion_id.is_none() && b.campamentos_para_elegir.is_empty());
    let mut de_admin = Almacen::default();
    de_admin.aplicar(&mensajes);

    // La plaza con su interior entero, las columnas ajenas con su composición, su Líder y su plan, y los héroes enteros (también el de quien no tiene Facción).
    assert_eq!(de_admin.get(TipoEntidad::Asentamiento, &as_), Some(&Cuerpo::Asentamiento(plaza_entera.clone())));
    assert!(!plaza_entera.almacen.is_empty() && plaza_entera.poblacion.is_some() && plaza_entera.version_interior > 0, "la plaza va con su interior");
    for c in &columnas {
        assert_eq!(de_admin.get(TipoEntidad::Ejercito, &c.id), Some(&Cuerpo::Ejercito(c.clone())));
        assert!(!c.lider_id.is_empty() && c.suministro.is_some() && c.viveres.is_some(), "sin redactar: {c:?}");
    }
    for h in &heroes {
        assert_eq!(de_admin.get(TipoEntidad::Heroe, &h.id), Some(&Cuerpo::Heroe(h.clone())));
        assert!(h.almacen_personal.is_some() && h.atributos_base.is_some() && h.ubicacion.is_some(), "el héroe entero: {h:?}");
    }
    assert_eq!(heroes.map(|h| h.jugador_id), [Some("ana".into()), Some("dora".into()), None], "la cuenta de cada héroe de verdad (el de la columna de gus es de campo)");
    assert_eq!(de_admin.get(TipoEntidad::Escuadron, &escuadra.id), Some(&Cuerpo::Escuadron(escuadra.clone())), "las escuadras de todos");
    // Las tres Facciones con sus tecnologías, la guerra entre F2 y F3, los campamentos de mercenarios (todos, explorados o no) y el de bandidos (aunque nadie lo vea).
    for f in ["f-1", "f-2", "f-3"] {
        assert!(de_admin.get(TipoEntidad::Faccion, f).is_some() && de_admin.get(TipoEntidad::TecnologiaFaccion, f).is_some(), "{f}");
    }
    let Some(Cuerpo::Relacion(rel)) = de_admin.relacion_entre(2, 3).and_then(|id| de_admin.get(TipoEntidad::RelacionPolitica, &id)) else { panic!("la relación de F2 y F3") };
    assert_eq!(rel.tipo, pb::TipoRelacion::RelacionGuerra as i32);
    assert_eq!(de_admin.de(TipoEntidad::CampamentoMercenarios), campamentos);
    assert_eq!(de_admin.get(TipoEntidad::CampamentoBandidos, &bandidos.id), Some(&Cuerpo::CampamentoBandidos(bandidos)));
    assert_eq!(de_admin.de(TipoEntidad::FichaConocida), 0, "sin niebla no hay recuerdos: lo ve todo tal como es");

    // Gus marcha: a la administración le llega su columna entera, con el plan hasta el destino (lo que a un ajeno nunca se le enseña).
    let (mut gus_c, _, _) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[], &[])).await;
    let destino = en_tierra(&m.0, centro, -(radio + 30 * U), 0);
    let (_, r) = dar_todo(&mut gus_c, "g1", marchar(gus, destino)).await;
    aceptada(&r);
    let deltas = de_admin.aplicar(&pendiente(&mut admin_c).await).len();
    assert!(deltas >= 1);
    let Some(Cuerpo::Ejercito(col)) = de_admin.get(TipoEntidad::Ejercito, &ej_gus) else { panic!() };
    let fin = col.plan.as_ref().expect("con su plan").vertices.last().copied().unwrap();
    assert_eq!(fin.p.as_ref().map(|p| (p.x, p.y)), Some((destino.x, destino.y)), "el plan entero, hasta el destino");

    // Una relación que se fija y se quita: ACTUALIZA y QUITA.
    let (previos, r) = dar_todo(&mut admin_c, "a1", admin("fijar_relacion", &[("faccion_a_id", "f-1"), ("faccion_b_id", "f-3"), ("relacion", "alianza")])).await;
    aceptada(&r);
    de_admin.aplicar(&previos);
    let r13 = de_admin.relacion_entre(1, 3).expect("la alianza de F1 y F3");
    let (previos, r) = dar_todo(&mut admin_c, "a2", admin("fijar_relacion", &[("faccion_a_id", "f-1"), ("faccion_b_id", "f-3")])).await;
    aceptada(&r);
    let deltas = de_admin.aplicar(&previos);
    assert!(deltas.iter().flat_map(|d| &d.cambios).any(|c| c.id == r13 && c.tipo == TipoCambio::CambioQuita as i32));

    // Dora entra en la plaza: cambia quién está dentro (el interior, no la ficha) y la administración lo ve.
    let (previos, r) = dar_todo(&mut admin_c, "a3", admin("colocar_heroe", &[("heroe_id", &ids::heroe(dora)), ("ubicacion", &as_)])).await;
    aceptada(&r);
    de_admin.aplicar(&previos);
    let Some(Cuerpo::Asentamiento(p)) = de_admin.get(TipoEntidad::Asentamiento, &as_) else { panic!() };
    assert!(p.presentes_ids.contains(&ids::heroe(dora)), "{:?}", p.presentes_ids);

    // Pasan tres minutos con la columna en marcha: la plaza llega en cada paso como cambio parcial con máscara (su almacén, no su ciudad) y la llegada de gus, con su hecho.
    let (mut parciales, mut llego) = (0, false);
    for (k, t) in [1, 2, 3, 4].map(|k| (k, if k < 4 { t0 + k * MS_POR_MINUTO } else { fin.en_ms.max(t0 + 3 * MS_POR_MINUTO) + 1_000 })) {
        let (previos, r) = dar_todo(&mut admin_c, &format!("t{k}"), reloj_hasta(t)).await;
        aceptada(&r);
        for d in de_admin.aplicar(&previos) {
            for c in d.cambios.iter().filter(|c| c.id == as_ && !c.reemplaza) {
                assert!(c.campos.starts_with(&["version".to_owned(), "version_interior".to_owned()]) && !c.campos.contains(&"trazado".to_owned()), "{:?}", c.campos);
                parciales += 1;
            }
            llego |= d.eventos.iter().any(|e| e.codigo == "ejercito.llega" && e.heroes_ids == [ids::heroe(gus)]);
        }
    }
    assert!(parciales >= 3, "un cambio parcial de la plaza por cada paso de minuto: {parciales}");
    assert!(llego, "los hechos llegan a la administración");
    assert!(de_admin.get(TipoEntidad::Ejercito, &ej_gus).is_some_and(|c| matches!(c, Cuerpo::Ejercito(x) if x.plan.is_none())), "gus ha llegado");

    // Foto + deltas = la foto que recibe una sesión de administración nueva.
    de_admin.aplicar(&pendiente(&mut admin_c).await);
    let nueva = foto_nueva(&w, "admin2").await;
    assert_eq!(de_admin.entidades, nueva.entidades, "foto + deltas = foto nueva");
    w.parar().await;
}

// ───────────────────────────── 2. Solo la administración abre `ADMIN` ─────────────────────────────

#[tokio::test]
async fn solo_una_sesion_de_administracion_abre_admin_y_las_ordenes_de_administracion_siguen_siendo_suyas() {
    let m = con_mundo!();
    let x = escenario(m, Ajustes::default());
    let v = x.n.version();
    let w = arrancar_con(x.n).await;
    let pide_admin = [("ADMIN:", v)];
    // Un jugador de Unity, un bot sin héroe y un servidor de batalla que dicen conocer `ADMIN`: no lo abren ni reciben nada de él.
    for (cuenta, tipo) in [("ok:ana", TipoCliente::ClienteUnity), ("ok:robot", TipoCliente::ClienteBot), ("svc:srv", TipoCliente::ClienteServidorBatalla)] {
        let mut h = hola(cuenta, tipo);
        h.versiones_conocidas.extend(pide_admin.map(|(k, v)| (k.to_owned(), v)));
        let (c, b, mensajes) = w.conectar(h).await;
        assert!(b.ambitos.iter().all(|a| ambito(&a.ambito) != "ADMIN:"), "{cuenta}: {:?}", b.ambitos);
        for m in &mensajes {
            let a = match m {
                ms::Cuerpo::Foto(f) => ambito(&f.ambito),
                ms::Cuerpo::Delta(d) => ambito(&d.ambito),
                _ => String::new(),
            };
            assert_ne!(a, "ADMIN:", "{cuenta}");
        }
        c.cerrar().await;
    }
    // La administración abre solo `ADMIN`, aunque diga conocer ámbitos de jugador.
    let (mut admin_c, b, _) = w.conectar(hola_con("admin", TipoCliente::ClienteAdminWeb, &[("HEROE:heroe-1", v), ("FACCION:f-1", v)], &[])).await;
    assert_eq!(b.ambitos.iter().map(|a| ambito(&a.ambito)).collect::<Vec<_>>(), ["ADMIN:"]);
    // Las órdenes de administración (y el reloj) siguen siendo solo suyas.
    let (mut ana, _, _) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[], &[])).await;
    let (_, r) = dar_todo(&mut ana, "x1", admin("fijar_relacion", &[("faccion_a_id", "f-1"), ("faccion_b_id", "f-3"), ("relacion", "alianza")])).await;
    assert_eq!(rechazo(&r), "no_autorizado");
    let (_, r) = dar_todo(&mut ana, "x2", reloj_hasta(i64::MAX / 2)).await;
    assert_eq!(rechazo(&r), "no_autorizado");
    let (_, r) = dar_todo(&mut admin_c, "x3", admin("fijar_relacion", &[("faccion_a_id", "f-1"), ("faccion_b_id", "f-3"), ("relacion", "alianza")])).await;
    aceptada(&r);
    // Y la administración no tiene vista (ve todo sin pedirla).
    admin_c.enviar(mc::Cuerpo::Vista(pb::CambiarVista { rectangulo: Some(pb::Rectangulo { min: Some(pb::Punto { x: 0, y: 0 }), max: Some(pb::Punto { x: 1, y: 1 }) }) })).await;
    loop {
        match admin_c.esperar().await.cuerpo {
            Some(ms::Cuerpo::Aviso(a)) => break assert_eq!(a.codigo, "vista.sin_derecho"),
            Some(ms::Cuerpo::Delta(d)) => assert_eq!(ambito(&d.ambito), "ADMIN:"),
            o => panic!("llegó {o:?}"),
        }
    }
    w.parar().await;
}

// ───────────────────────────── 3. Reanudación ─────────────────────────────

#[tokio::test]
async fn admin_se_reanuda_sin_novedad_o_por_deltas_con_la_secuencia_interior_de_cada_plaza_y_con_foto_si_el_registro_no_alcanza() {
    let m = con_mundo!();
    let x = escenario(m, Ajustes::default());
    let (plaza, as_, t0) = (x.plaza, ids::asentamiento(x.plaza), x.n.ahora());
    let clave_plaza = format!("ASENTAMIENTO:{as_}");
    let w = arrancar_con(x.n).await;
    let (mut otro, _, _) = w.conectar(hola_con("otro", TipoCliente::ClienteAdminWeb, &[], &[])).await;
    let (admin_c, b, mensajes) = w.conectar(hola_con("admin", TipoCliente::ClienteAdminWeb, &[], &[])).await;
    let mut tenia = Almacen::default();
    tenia.aplicar(&mensajes);
    let v0 = b.ambitos[0].version_actual;
    let Some(Cuerpo::Asentamiento(p)) = tenia.get(TipoEntidad::Asentamiento, &as_) else { panic!() };
    let ki = p.version_interior;
    admin_c.cerrar().await;
    let decision = |b: &pb::Bienvenida| b.ambitos.iter().map(|a| (ambito(&a.ambito), a.decision)).collect::<Vec<_>>();

    // Nada cambió: sin novedad y sin un solo mensaje.
    let (c, b, mensajes) = w.conectar(hola_con("admin", TipoCliente::ClienteAdminWeb, &[("ADMIN:", v0)], &[(&clave_plaza, ki)])).await;
    assert_eq!(decision(&b), [("ADMIN:".to_owned(), D::ReanudaSinNovedad as i32)]);
    assert!(mensajes.is_empty());
    c.cerrar().await;

    // Mientras no está: madera para la plaza, una relación y dos pasos de minuto. Reanuda con una sola delta de v0 a hoy, con la plaza entera (su secuencia interior avanzó).
    aceptada(&dar(&mut otro, "o1", dar_madera(plaza, 1_000)).await.1);
    aceptada(&dar(&mut otro, "o2", admin("fijar_relacion", &[("faccion_a_id", "f-1"), ("faccion_b_id", "f-2"), ("relacion", "guerra")])).await.1);
    aceptada(&dar(&mut otro, "o3", reloj_hasta(t0 + 2 * MS_POR_MINUTO)).await.1);
    pendiente(&mut otro).await;
    let (mut c, b, mensajes) = w.conectar(hola_con("admin", TipoCliente::ClienteAdminWeb, &[("ADMIN:", v0)], &[(&clave_plaza, ki)])).await;
    assert_eq!(decision(&b), [("ADMIN:".to_owned(), D::ReanudaConDeltas as i32)]);
    let ms::Cuerpo::Delta(d) = &mensajes[0] else { panic!("{mensajes:?}") };
    assert_eq!((d.desde, d.hasta, mensajes.len()), (v0, b.ambitos[0].version_actual, 1), "una sola delta de v0 a hoy");
    let cambio = d.cambios.iter().find(|c| c.id == as_).expect("la plaza");
    assert!(cambio.reemplaza, "la plaza entera: no se sabe qué tiene el cliente de ella");
    assert!(d.cambios.iter().any(|c| matches!(c.datos.as_ref().and_then(|e| e.cuerpo.as_ref()), Some(Cuerpo::Relacion(r)) if r.faccion_a_id == "f-1" && r.faccion_b_id == "f-2")));
    tenia.aplicar(&mensajes);
    pendiente(&mut c).await;
    assert_eq!(tenia.entidades, foto_nueva(&w, "tercero").await.entidades, "foto + deltas = foto nueva");
    let Some(Cuerpo::Asentamiento(p)) = tenia.get(TipoEntidad::Asentamiento, &as_) else { panic!() };
    let (v1, ki1) = (tenia.version, p.version_interior);
    assert!(ki1 >= ki + 2);
    c.cerrar().await;

    // Al día pero con una secuencia interior que no es la de hoy (de una copia del servidor más nueva, o ninguna): solo esa plaza, entera.
    for ki_cliente in [ki1 + 5, 0] {
        let (c, b, mensajes) = w.conectar(hola_con("admin", TipoCliente::ClienteAdminWeb, &[("ADMIN:", v1)], &[(&clave_plaza, ki_cliente)])).await;
        assert_eq!(decision(&b), [("ADMIN:".to_owned(), D::ReanudaConDeltas as i32)], "secuencia {ki_cliente}");
        let ms::Cuerpo::Delta(d) = &mensajes[0] else { panic!() };
        assert_eq!(d.cambios.iter().filter(|c| c.entidad == TipoEntidad::Asentamiento as i32).map(|c| (c.id.as_str(), c.reemplaza)).collect::<Vec<_>>(), [(as_.as_str(), true)]);
        c.cerrar().await;
    }

    // Una versión del futuro: foto.
    let (c, b, mensajes) = w.conectar(hola_con("admin", TipoCliente::ClienteAdminWeb, &[("ADMIN:", 999_999)], &[])).await;
    assert_eq!(decision(&b), [("ADMIN:".to_owned(), D::ReanudaConFoto as i32)]);
    assert!(matches!(&mensajes[..], [ms::Cuerpo::Foto(_)]));
    c.cerrar().await;
    w.parar().await;

    // Sin registro que alcance la versión del cliente (se conservan 2 versiones): foto.
    let x = escenario(m, Ajustes { retencion_cambios: 2, ..Ajustes::default() });
    let plaza = x.plaza;
    let w = arrancar_con(x.n).await;
    let (mut otro, _, _) = w.conectar(hola_con("otro", TipoCliente::ClienteAdminWeb, &[], &[])).await;
    let (c, b, _) = w.conectar(hola_con("admin", TipoCliente::ClienteAdminWeb, &[], &[])).await;
    let v0 = b.ambitos[0].version_actual;
    c.cerrar().await;
    for k in 0..4 {
        aceptada(&dar(&mut otro, &format!("r{k}"), dar_madera(plaza, 1_000 + k)).await.1);
    }
    let (_, b, mensajes) = w.conectar(hola_con("admin", TipoCliente::ClienteAdminWeb, &[("ADMIN:", v0)], &[])).await;
    assert_eq!(decision(&b), [("ADMIN:".to_owned(), D::ReanudaConFoto as i32)]);
    assert!(matches!(&mensajes[..], [ms::Cuerpo::Foto(_)]));
    w.parar().await;
}

// ───────────────────────────── 4. La versión y el hash no dependen de la administración ─────────────────────────────

/// Las mismas órdenes, con una sesión de administración que lo mira todo detrás del servidor (y que da las órdenes de administración y mueve el reloj) y aplicadas al núcleo sin sesión
/// ninguna (con las pertenencias que lleva el actor): la misma versión y el mismo hash del mundo.
#[tokio::test]
async fn la_version_y_el_hash_son_los_mismos_con_una_sesion_de_administracion_y_sin_ninguna() {
    let m = con_mundo!();
    let (con, mut sin) = (escenario(m, Ajustes::default()), escenario(m, Ajustes::default()));
    sin.n.seguir_pertenencias();
    assert_eq!((con.n.version(), con.n.hash_estado()), (sin.n.version(), sin.n.hash_estado()), "partida igual");
    let (plaza, t0) = (con.plaza, con.n.ahora());
    // (peticion, cuenta, orden del contrato, comando del núcleo)
    let pasos: Vec<(String, &str, orden::Cuerpo, Comando)> = (1..=4i64)
        .flat_map(|k| {
            let t = t0 + k * MS_POR_MINUTO;
            [
                (format!("m{k}"), "admin", dar_madera(plaza, 1_000 * k), Comando::DarRecursos { asentamiento: plaza, recurso: asentamiento::Recurso::Madera, cantidad: 1_000 * k }),
                (format!("t{k}"), "admin", reloj_hasta(t), Comando::Reloj(AccionReloj::AvanzarHasta(t))),
            ]
        })
        .chain([(
            "v1".to_owned(),
            "ana",
            orden::Cuerpo::AnadirEdificioManualmente(pb::OrdenAnadirEdificioManualmente { asentamiento_id: ids::asentamiento(plaza), cargo: "gobernador".into(), tipo: "vivienda".into() }),
            Comando::AnadirEdificio { asentamiento: plaza, tipo: asentamiento::catalogo::Tipo::Vivienda },
        )])
        .collect();

    // Con la administración delante: un observador más que lo mira todo.
    let w = arrancar_con(con.n).await;
    let (mut obs, _, _) = w.conectar(hola_con("observador", TipoCliente::ClienteAdminWeb, &[], &[])).await;
    let (mut admin_c, _, _) = w.conectar(hola_con("admin", TipoCliente::ClienteAdminWeb, &[], &[])).await;
    let (mut ana_c, _, _) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[], &[])).await;
    for (peticion, cuenta, cuerpo, _) in &pasos {
        let c = if *cuenta == "admin" { &mut admin_c } else { &mut ana_c };
        aceptada(&dar_todo(c, peticion, cuerpo.clone()).await.1);
    }
    let vistos = pendiente(&mut obs).await;
    assert!(vistos.iter().filter(|m| matches!(m, ms::Cuerpo::Delta(_))).count() >= pasos.len(), "el observador lo vio todo pasar");
    let con = w.parar().await;

    // Sin nadie: el núcleo solo, con los mismos comandos y las mismas peticiones.
    for (peticion, cuenta, _, comando) in pasos {
        let p = Peticion { actor: cuenta.into(), admin: cuenta == "admin", peticion_id: peticion, version_esperada: None, comando };
        let t = sin.n.ahora();
        assert!(sin.n.aplicar_en(t, p).aceptada());
    }
    assert_eq!((con.version(), con.hash_estado()), (sin.n.version(), sin.n.hash_estado()), "la administración no cambia el mundo");
    assert_eq!(con.version_interior(plaza), sin.n.version_interior(plaza));
}

// ───────────────────────────── Coste ─────────────────────────────

/// Cuánto cuesta proyectar `ADMIN` en el mundo de las mediciones de T3d (nueve plazas y 300 columnas de 30 Facciones que marchan): la foto entera (proyección y bytes) y lo que cuesta cada versión
/// (proyectar lo que tocó, que es lo que hace la `Delta` de la administración): `cargo test --release -p nucleo --test admin coste -- --ignored --nocapture`.
#[test]
#[ignore = "medición"]
fn coste_de_proyectar_admin_con_300_columnas_y_30_facciones() {
    use prost::Message;
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
    n.seguir_pertenencias();
    let v0 = n.version();
    n.avanzar_hasta(30 * MS_POR_MINUTO);
    let medir = |veces: u32, f: &dyn Fn()| {
        let i0 = std::time::Instant::now();
        for _ in 0..veces {
            f();
        }
        i0.elapsed().as_secs_f64() * 1e6 / f64::from(veces)
    };
    // La foto: cada plaza con su interior y todo lo demás con la proyección de la administración.
    let foto = || {
        let plazas = n.estado().asentamientos.keys().filter_map(|&p| interior::asentamiento(&n, p)).map(|p| pb::Entidad { cuerpo: Some(Cuerpo::Asentamiento(p)) });
        let resto = propios::todo_menos_plazas(&n).into_iter().flat_map(|(clase, id)| propios::proyectar(&n, Espectador::Admin, clase, id).existen).map(|(_, _, c)| pb::Entidad { cuerpo: Some(c) });
        pb::Foto { entidades: plazas.chain(resto).collect(), ..Default::default() }
    };
    let f = foto();
    let t_foto = medir(10, &|| drop(foto()));
    // Cada versión: proyectar lo que tocó (lo que hace la `Delta` de `ADMIN`, sin la máscara de las plazas).
    let registros: Vec<_> = n.cambios_desde(v0).unwrap().collect();
    let tocadas: usize = registros.iter().map(|cv| Clase::TODAS.iter().map(|&c| cv.tocados.de(c).len()).sum::<usize>()).sum();
    let por_version = medir(3, &|| {
        for cv in &registros {
            for c in Clase::TODAS.into_iter().filter(|&c| c != Clase::Interior && c != Clase::Asentamiento) {
                for &id in cv.tocados.de(c) {
                    drop(propios::cambios(&n, Espectador::Admin, c, id, true));
                }
            }
        }
    }) / registros.len() as f64;
    println!(
        "ADMIN: foto de {} entidades ({} plazas) en {:.1} ms, {} KB; {} versiones en 30 min de mundo ({} entidades tocadas): {:.1} µs por versión",
        f.entidades.len(),
        n.estado().asentamientos.len(),
        t_foto / 1000.0,
        f.encoded_len() / 1024,
        registros.len(),
        tocadas,
        por_version
    );
}
