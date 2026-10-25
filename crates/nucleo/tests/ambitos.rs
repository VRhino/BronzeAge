//! Los ámbitos de la sesión (4H, T3a) por el servidor WebSocket real y el actor del núcleo: qué ámbitos abre cada sesión y a quién se los niega, `Revocado` y foto nueva cuando cambia el
//! derecho, reanudación por versión con y sin registro, deltas a varias sesiones en orden causal y sin filtrarse a quien no tiene el ámbito, y `CambiarVista`.
//!
//! Lo que mira el reloj de mundo por su cuenta (el paso de minuto de una plaza, el encuentro de dos columnas que se ven) llega a las sesiones como una `Delta` suelta en cualquier momento: las
//! pruebas de plaza dejan el mundo recién entrado en un minuto y la plaza con la construcción automática parada, las de columnas ponen las columnas a más de 30 u unas de otras, y donde se
//! cuenta lo que llegó se cuenta tras una orden que el actor rechaza (`pendiente`), que sale después de todo lo que ya estaba en camino. Las de plaza y campamento van sobre el artefacto de
//! mundo real (`generador/salida/mundo/mundo-*.bin`) y se omiten si no está.

mod comun;

use asentamiento::Recurso::*;
use base::{Instante, Punto, MS_POR_MINUTO};
use comun::sesion::*;
use comun::*;
use contrato::v1::{self as pb, mensaje_de_cliente as mc, mensaje_de_servidor as ms, orden, DecisionReanudacion as D, TipoCliente};
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::proyeccion::ids;
use nucleo::{Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Resultado};
use prost_types::{value::Kind, Struct, Value};
use rutas::Rutas;
use servidor::prueba::{hola, ClientePrueba};
use std::sync::{Arc, OnceLock};

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

// ───────────────────────────── Un mundo con una plaza y cuatro héroes ─────────────────────────────

/// Un sitio verde con mucho bosque a 10 u del campamento 0: una plaza que no cae en ruinas por falta de madera.
fn sitio_rico(n: &Nucleo, m: &Mundo, faccion: u32) -> Punto {
    let p0 = m.campamentos()[0].posicion.unwrap();
    for r in (0..8).map(|k| 50_000 + k * 10_000) {
        for g in 0..360 {
            let a = (g as f64).to_radians();
            let p = Punto::new(p0.x + (r as f64 * a.cos()) as i64, p0.y + (r as f64 * a.sin()) as i64);
            if n.viabilidad(p, faccion).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6) {
                return p;
            }
        }
    }
    panic!("sin sitio rico cerca del campamento 0");
}

/// El mundo antes de ponerlo detrás del servidor: lo que no se juega por el contrato se prepara aquí.
struct Aldea {
    n: Nucleo,
    plaza: u64,
    /// Ciudadanos de la Facción 1, residentes y dentro de la plaza.
    ana: u64,
    dora: u64,
    /// Ciudadano de la Facción 1 dentro del campamento 0, y de la Facción 2 dentro del mismo.
    beto: u64,
    gus: u64,
    peticiones: u64,
}

impl Aldea {
    fn ok(&mut self, actor: &str, c: Comando) -> nucleo::Respuesta {
        self.peticiones += 1;
        let p = Peticion::nueva(actor, &format!("prep{}", self.peticiones), c);
        let t = self.n.ahora();
        // 5I (5E-10): sin el andamiaje de `comun::mandaba`; cada cuenta da sus órdenes de plaza con la matriz real.
        let r = self.n.aplicar_en(t, if actor == "admin" { p.de_admin() } else { p });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
        r
    }

    fn heroe(&mut self, cuenta: &str, faccion: u32, plaza: Option<u64>) -> u64 {
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano }).creadas[0];
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(faccion) });
        if let Some(a) = plaza {
            self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(a as u32)) });
            self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(a as u32) });
            // 5E: el primer residente que está dentro gobierna la plaza (sus órdenes de plaza pasan la matriz real).
            if comun::sin_gobernador_dentro(&self.n, a) {
                comun::gobierna(&mut self.n, a, h);
            }
        }
        h
    }
}

/// Con `provisiones`, la plaza tiene de todo en el almacén; sin ellas, sin madera para su mantenimiento, acaba en ruinas.
fn aldea(m: &M, ajustes: Ajustes, provisiones: bool) -> Aldea {
    let mut reglas = Reglas::default();
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let n = Nucleo::nuevo(Arranque { reglas, ajustes, semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
    let mut x = Aldea { n, plaza: 0, ana: 0, dora: 0, beto: 0, gus: 0, peticiones: 0 };
    // La plaza la funda una cuenta con una columna de fundación; las cuentas de los héroes de la prueba no tienen ninguna columna.
    let sitio = sitio_rico(&x.n, &m.0, 1);
    let columna = x.n.crear_ejercito(0, "fundador", 1, sitio, 500, 100_000).unwrap();
    x.ok("admin", Comando::DarCaravanaFundacion { ejercito: columna });
    x.plaza = x.ok("fundador", Comando::Fundar { ejercito: columna }).creadas[0];
    x.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: x.plaza, pausada: true });
    for r in [Madera, Piedra, Oro, Trigo].into_iter().filter(|_| provisiones) {
        x.ok("admin", Comando::DarRecursos { asentamiento: x.plaza, recurso: r, cantidad: 300_000 });
    }
    let plaza = x.plaza;
    x.ana = x.heroe("ana", 1, Some(plaza));
    x.dora = x.heroe("dora", 1, Some(plaza));
    x.beto = x.heroe("beto", 1, None);
    x.gus = x.heroe("gus", 2, None);
    // El mundo, recién entrado en un minuto: el paso de minuto de la plaza queda a 59 s de reloj de pared.
    let ahora = x.n.ahora();
    x.n.avanzar_hasta((ahora / MS_POR_MINUTO + 1) * MS_POR_MINUTO + 1_000);
    x
}

// ───────────────────────────── Lectura de lo que llega ─────────────────────────────

fn ambito(a: &Option<pb::Ambito>) -> String {
    let a = a.as_ref().expect("un ámbito");
    format!("{}:{}", pb::AmbitoTipo::try_from(a.tipo).unwrap().as_str_name().trim_start_matches("AMBITO_"), a.id)
}

/// Un mensaje en una línea: `foto HEROE:heroe-3`, `delta ASENTAMIENTO:as-7`, `revocado CAMPAMENTO:camp-0`...
fn etiqueta(m: &ms::Cuerpo) -> String {
    match m {
        ms::Cuerpo::Foto(f) => format!("foto {}", ambito(&f.ambito)),
        ms::Cuerpo::Delta(d) => format!("delta {}", ambito(&d.ambito)),
        ms::Cuerpo::Revocado(r) => format!("revocado {}", ambito(&r.ambito)),
        ms::Cuerpo::Resultado(_) => "resultado".into(),
        ms::Cuerpo::Aviso(a) => format!("aviso {}", a.codigo),
        otro => format!("{otro:?}"),
    }
}

fn etiquetas(ms: &[ms::Cuerpo]) -> Vec<String> {
    ms.iter().map(etiqueta).collect()
}

/// Los `n` mensajes siguientes.
async fn siguientes(c: &mut ClientePrueba, n: usize) -> Vec<ms::Cuerpo> {
    let mut v = Vec::new();
    for _ in 0..n {
        v.push(c.esperar().await.cuerpo.expect("un mensaje con cuerpo"));
    }
    v
}

/// Lo que ya estaba en camino hacia la sesión: el actor contesta a una orden que rechaza después de repartir todo lo anterior, y el servidor entrega en orden.
async fn pendiente(c: &mut ClientePrueba) -> Vec<ms::Cuerpo> {
    let (previos, r) = dar_todo(c, "barrera", orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: "ej-999999".into() })).await;
    assert_eq!(rechazo(&r), "ejercito.desconocido");
    previos
}

fn foto_de(m: &ms::Cuerpo) -> &pb::Foto {
    let ms::Cuerpo::Foto(f) = m else { panic!("se esperaba una Foto: {m:?}") };
    f
}

fn delta_de(m: &ms::Cuerpo) -> &pb::Delta {
    let ms::Cuerpo::Delta(d) = m else { panic!("se esperaba una Delta: {m:?}") };
    d
}

/// Los ids de las plazas de una `Foto` o `Delta`.
fn plazas(entidades: impl Iterator<Item = Option<pb::entidad::Cuerpo>>) -> Vec<String> {
    entidades.filter_map(|e| if let Some(pb::entidad::Cuerpo::Asentamiento(a)) = e { Some(a.id) } else { None }).collect()
}

fn plazas_de_foto(f: &pb::Foto) -> Vec<String> {
    plazas(f.entidades.iter().map(|e| e.cuerpo.clone()))
}

fn plazas_de_delta(d: &pb::Delta) -> Vec<String> {
    plazas(d.cambios.iter().filter_map(|c| c.datos.clone()).map(|e| e.cuerpo))
}

fn hola_con(cliente: &str, tipo: TipoCliente, conocidas: &[(&str, u64)]) -> pb::Hola {
    let mut h = hola(&format!("ok:{cliente}"), tipo);
    h.versiones_conocidas.extend(conocidas.iter().map(|(k, v)| ((*k).to_owned(), *v)));
    h
}

fn texto(x: &str) -> Value {
    Value { kind: Some(Kind::StringValue(x.into())) }
}

/// Una orden de administración con sus parámetros de texto.
fn admin(codigo: &str, parametros: &[(&str, &str)]) -> orden::Cuerpo {
    orden::Cuerpo::Administracion(pb::OrdenDeAdministracion { codigo: codigo.into(), parametros: Some(Struct { fields: parametros.iter().map(|(k, v)| ((*k).to_owned(), texto(v))).collect() }) })
}

fn colocar(heroe: u64, sitio: &str) -> orden::Cuerpo {
    admin("colocar_heroe", &[("heroe_id", &ids::heroe(heroe)), ("ubicacion", sitio)])
}

fn calibrar_reserva(plaza: u64, valor: i64) -> orden::Cuerpo {
    orden::Cuerpo::CalibrarReservaManual(pb::OrdenCalibrarReservaManual { asentamiento_id: ids::asentamiento(plaza), recurso: "trigo".into(), valor })
}

fn dar_madera(plaza: u64, cantidad: i64) -> orden::Cuerpo {
    let campos = [("asentamiento_id", texto(&ids::asentamiento(plaza))), ("recurso", texto("madera")), ("cantidad", Value { kind: Some(Kind::NumberValue(cantidad as f64)) })];
    orden::Cuerpo::Administracion(pb::OrdenDeAdministracion { codigo: "dar_recursos".into(), parametros: Some(Struct { fields: campos.into_iter().map(|(k, v)| (k.to_owned(), v)).collect() }) })
}

// ───────────────────────────── 1. Quién abre qué ─────────────────────────────

#[tokio::test]
async fn cada_sesion_abre_los_ambitos_que_le_corresponden_y_ninguno_mas() {
    let m = con_mundo!();
    let x = aldea(m, Ajustes::default(), true);
    let (plaza, as_, camp) = (x.plaza, ids::asentamiento(x.plaza), ids::campamento(0));
    let (h_ana, h_dora, h_beto, h_gus) = (ids::heroe(x.ana), ids::heroe(x.dora), ids::heroe(x.beto), ids::heroe(x.gus));
    let w = arrancar_con(x.n).await;

    // Un ciudadano dentro de su plaza: el héroe, su Facción y la plaza, con foto de cada uno.
    let (_, b, mensajes) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    assert_eq!((b.heroe_id.as_deref(), b.faccion_id.as_deref()), (Some(h_ana.as_str()), Some("f-1")));
    assert_eq!(etiquetas(&mensajes), [format!("foto HEROE:{h_ana}"), "foto FACCION:f-1".into(), format!("foto ASENTAMIENTO:{as_}")]);
    assert!(b.ambitos.iter().all(|a| a.decision == D::ReanudaConFoto as i32));
    assert_eq!(plazas_de_foto(foto_de(&mensajes[2])), [as_.as_str()]);
    let (_, b, mensajes) = w.conectar(hola_con("dora", TipoCliente::ClienteBot, &[])).await;
    assert_eq!((b.heroe_id.as_deref(), etiquetas(&mensajes)), (Some(h_dora.as_str()), vec![format!("foto HEROE:{h_dora}"), "foto FACCION:f-1".into(), format!("foto ASENTAMIENTO:{as_}")]), "un bot, igual que un jugador");

    // Dentro de un campamento: el del campamento, no el de una plaza. Cada uno con su Facción.
    let (_, _, mensajes) = w.conectar(hola_con("beto", TipoCliente::ClienteUnity, &[])).await;
    assert_eq!(etiquetas(&mensajes), [format!("foto HEROE:{h_beto}"), "foto FACCION:f-1".into(), format!("foto CAMPAMENTO:{camp}")]);
    let (_, _, mensajes) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;
    assert_eq!(etiquetas(&mensajes), [format!("foto HEROE:{h_gus}"), "foto FACCION:f-2".into(), format!("foto CAMPAMENTO:{camp}")]);

    // La administración: solo ADMIN, con todo lo que hay. Un servidor de batalla, nada.
    let (_, b, mensajes) = w.conectar(hola_con("admin", TipoCliente::ClienteAdminWeb, &[])).await;
    assert_eq!((b.heroe_id, etiquetas(&mensajes)), (None, vec!["foto ADMIN:".to_owned()]));
    assert!(plazas_de_foto(foto_de(&mensajes[0])).contains(&as_));
    let (_, b, mensajes) = w.conectar(hola("svc:srv", TipoCliente::ClienteServidorBatalla)).await;
    assert!(b.ambitos.is_empty() && mensajes.is_empty() && b.heroe_id.is_none());

    // Un jugador sin héroe no tiene ámbitos: ve dónde puede nacer, con cuántos héroes viven ya en cada campamento (beto y gus, en el 0).
    let (_, b, mensajes) = w.conectar(hola_con("nueva", TipoCliente::ClienteUnity, &[])).await;
    assert!(b.heroe_id.is_none() && b.faccion_id.is_none() && b.ambitos.is_empty() && mensajes.is_empty());
    let cero = b.campamentos_para_elegir.iter().find(|c| c.campamento_id == camp).expect("el campamento 0");
    assert_eq!((cero.residentes, cero.eligieron_como_inicial), (2, 0));
    assert_eq!(b.campamentos_para_elegir.len(), mundo().unwrap().0.campamentos().len());

    // Lo que un cliente diga conocer de un ámbito que no le corresponde no le abre nada: ni la plaza, ni la administración, ni una Facción ajena.
    let ajenos = [(format!("ASENTAMIENTO:{as_}"), 1), ("ADMIN:".to_owned(), 1), ("FACCION:f-1".to_owned(), 1), (format!("HEROE:{h_ana}"), 1)];
    let conocidas: Vec<(&str, u64)> = ajenos.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    let (_, b, mensajes) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &conocidas)).await;
    let abiertos: Vec<String> = b.ambitos.iter().map(|a| ambito(&a.ambito)).collect();
    assert_eq!(abiertos, [format!("HEROE:{h_gus}"), "FACCION:f-2".into(), format!("CAMPAMENTO:{camp}")]);
    assert!(b.ambitos.iter().all(|a| a.decision == D::ReanudaConFoto as i32) && mensajes.len() == 3);
    let n = w.parar().await;
    assert!(n.estado().asentamientos.contains_key(&plaza));
}

// ───────────────────────────── 2. Un héroe nuevo ─────────────────────────────

#[tokio::test]
async fn el_heroe_nuevo_abre_su_ambito_en_caliente_y_gana_la_faccion_cuando_se_la_dan() {
    let m = con_mundo!();
    let w = arrancar_con(aldea(m, Ajustes::default(), true).n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (mut nueva, b, _) = w.conectar(hola_con("nueva", TipoCliente::ClienteUnity, &[])).await;
    assert!(b.ambitos.is_empty());

    // Al crear su héroe nace en el campamento: se le abren el suyo y el del campamento, antes del resultado (sin Facción aún no hay ámbito de Facción ni vista).
    let crear = orden::Cuerpo::CrearHeroe(pb::OrdenCrearHeroe { display_name: "Nueva".into(), campamento_id: ids::campamento(0), class_definition_id: "guerrero".into(), genero: "femenino".into(), avatar: None });
    let (previos, r) = dar_todo(&mut nueva, "c1", crear).await;
    let heroe = r.creadas[0].id.clone();
    assert_eq!(etiquetas(&previos), [format!("foto HEROE:{heroe}"), format!("foto CAMPAMENTO:{}", ids::campamento(0))]);
    assert!(
        matches!(foto_de(&previos[0]).entidades[..], [pb::Entidad { cuerpo: Some(pb::entidad::Cuerpo::Heroe(_)) }, pb::Entidad { cuerpo: Some(pb::entidad::Cuerpo::EstadoTecnologia(_)) }]),
        "su ámbito es su héroe recién nacido, sin Facción ni escuadras, y lo público de la tecnología (6G)"
    );

    // La administración le da Facción: llega la foto de su ámbito. Y quien no administra no da Facción a nadie.
    let (_, r) = dar(&mut nueva, "c2", admin("fijar_faccion_de_heroe", &[("heroe_id", &heroe), ("faccion_id", "f-1")])).await;
    assert_eq!(rechazo(&r), "no_autorizado");
    let (_, r) = dar(&mut admin_c, "c3", admin("fijar_faccion_de_heroe", &[("heroe_id", &heroe), ("faccion_id", "f-1")])).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    assert_eq!(etiquetas(&siguientes(&mut nueva, 2).await), ["foto FACCION:f-1".to_owned(), format!("delta HEROE:{heroe}")]);
    assert!(pendiente(&mut nueva).await.is_empty());
    w.parar().await;
}

// ───────────────────────────── 3. Perder y ganar derechos ─────────────────────────────

#[tokio::test]
async fn al_perder_un_derecho_llega_revocado_y_al_ganar_otro_su_foto_con_las_ordenes_del_contrato_y_las_de_administracion() {
    let m = con_mundo!();
    let x = aldea(m, Ajustes::default(), true);
    let (plaza, as_, camp, h_ana) = (x.plaza, ids::asentamiento(x.plaza), ids::campamento(0), ids::heroe(x.ana));
    let ana_id = x.ana;
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (mut ana, _, mensajes) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    assert_eq!(mensajes.len(), 3);

    // Sale de la plaza para entrar en un campamento: pierde la plaza, gana el campamento (primero el Revocado, después la Foto).
    let (_, r) = dar(&mut admin_c, "a1", colocar(ana_id, &camp)).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let recibidos = siguientes(&mut ana, 3).await;
    assert_eq!(etiquetas(&recibidos), [format!("revocado ASENTAMIENTO:{as_}"), format!("foto CAMPAMENTO:{camp}"), format!("delta HEROE:{h_ana}")], "el héroe cambió de sitio: su ficha viaja tras los derechos");
    let ms::Cuerpo::Revocado(rev) = &recibidos[0] else { unreachable!() };
    assert_eq!(rev.motivo, "sin_derecho");
    assert!(pendiente(&mut ana).await.is_empty());

    // Y vuelve: pierde el campamento y recupera la plaza, otra vez con su foto entera.
    let (_, r) = dar(&mut admin_c, "a2", colocar(ana_id, &as_)).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let recibidos = siguientes(&mut ana, 3).await;
    assert_eq!(etiquetas(&recibidos), [format!("revocado CAMPAMENTO:{camp}"), format!("foto ASENTAMIENTO:{as_}"), format!("delta HEROE:{h_ana}")]);
    assert_eq!(plazas_de_foto(foto_de(&recibidos[1])), [as_.as_str()]);

    // Por una orden suya: salir al mundo desde su residencia le cuesta la plaza (y le nace la columna); entrar otra vez se la devuelve.
    let salir = orden::Cuerpo::SalirAlMundo(pb::OrdenSalirAlMundo { asentamiento_id: as_.clone(), heroe_id: h_ana.clone(), escuadron_ids: vec![], carga: None });
    let (previos, r) = dar_todo(&mut ana, "s1", salir).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    assert_eq!(etiquetas(&previos), [format!("revocado ASENTAMIENTO:{as_}"), format!("delta HEROE:{h_ana}"), "delta FACCION:f-1".to_owned()], "la plaza, tocada porque su guarnición sale, viaja en resumen a su Facción");
    let entrar = orden::Cuerpo::EntrarEnAsentamiento(pb::OrdenEntrarEnAsentamiento { asentamiento_id: as_.clone(), heroe_id: h_ana.clone() });
    let (previos, r) = dar_todo(&mut ana, "s2", entrar).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    assert!(etiquetas(&previos).contains(&format!("foto ASENTAMIENTO:{as_}")), "{:?}", etiquetas(&previos));

    // Cambia de Facción: deja de ser ciudadano de la plaza, así que pierde la Facción y la plaza juntas; gana la otra Facción.
    let (_, r) = dar(&mut admin_c, "a3", admin("fijar_faccion_de_heroe", &[("heroe_id", &h_ana), ("faccion_id", "f-2")])).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let recibidos = siguientes(&mut ana, 4).await;
    assert_eq!(etiquetas(&recibidos), ["revocado FACCION:f-1".to_owned(), format!("revocado ASENTAMIENTO:{as_}"), "foto FACCION:f-2".to_owned(), format!("delta HEROE:{h_ana}")]);
    assert!(pendiente(&mut ana).await.is_empty());
    let n = w.parar().await;
    assert_eq!(n.estado().asentamientos[&plaza].faccion, 1);
}

// ───────────────────────────── 4. Reanudación por versión ─────────────────────────────

#[tokio::test]
async fn al_reconectar_cada_ambito_se_reanuda_sin_novedad_con_deltas_o_con_foto() {
    let m = con_mundo!();
    let x = aldea(m, Ajustes::default(), true);
    let (plaza, as_, h_ana, ana_id) = (x.plaza, ids::asentamiento(x.plaza), ids::heroe(x.ana), x.ana);
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (ana, b, _) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let (k_heroe, k_faccion, k_plaza) = (format!("HEROE:{h_ana}"), "FACCION:f-1".to_owned(), format!("ASENTAMIENTO:{as_}"));
    let v = b.ambitos[0].version_actual;
    assert!(b.ambitos.iter().all(|a| a.version_actual == v));
    // Un cliente de la plaza conserva también la secuencia interior de la plaza (4H T3c): `version_interior_actual` de su ámbito.
    let vi = b.ambitos[2].version_interior_actual;
    assert!(b.ambitos[..2].iter().all(|a| a.version_interior_actual == 0), "solo ASENTAMIENTO lleva secuencia interior");
    ana.cerrar().await;
    let conoce = |v: u64| {
        let mut h = hola_con("ana", TipoCliente::ClienteUnity, &[(&k_heroe, v), (&k_faccion, v), (&k_plaza, v)]);
        h.versiones_interiores_conocidas.insert(k_plaza.clone(), vi);
        h
    };
    let decisiones = |b: &pb::Bienvenida| b.ambitos.iter().map(|a| a.decision).collect::<Vec<_>>();

    // Nada cambió: los tres sin novedad y sin un solo mensaje.
    let (mut ana, b, mensajes) = w.conectar(conoce(v)).await;
    assert_eq!(decisiones(&b), [D::ReanudaSinNovedad as i32; 3]);
    assert!(mensajes.is_empty());
    silencio(&mut ana).await;
    ana.cerrar().await;

    // Cambia la plaza y nada más: el ámbito de la plaza se reanuda con la delta consolidada, y el de la Facción también (la plaza viaja a ella en resumen); el del héroe, sin novedad.
    let (_, r) = dar(&mut admin_c, "r1", dar_madera(plaza, 1_000)).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let (_, r) = dar(&mut admin_c, "r2", dar_madera(plaza, 2_000)).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let (ana, b, mensajes) = w.conectar(conoce(v)).await;
    assert_eq!(decisiones(&b), [D::ReanudaSinNovedad as i32, D::ReanudaConDeltas as i32, D::ReanudaConDeltas as i32]);
    assert_eq!(etiquetas(&mensajes), [format!("delta {k_faccion}"), format!("delta {k_plaza}")]);
    assert_eq!(plazas_de_delta(delta_de(&mensajes[0])), [as_.clone()]);
    let d = delta_de(&mensajes[1]);
    assert_eq!((ambito(&d.ambito), d.desde, d.hasta, plazas_de_delta(d)), (k_plaza.clone(), v, b.ambitos[2].version_actual, vec![as_.clone()]), "una sola delta de v a hoy, con la plaza una vez");
    assert!(d.hasta >= r.version_tras);
    ana.cerrar().await;

    // Un cliente que cree tener más de lo que hay (el servidor volvió de una copia vieja): foto de todo.
    let (ana, b, mensajes) = w.conectar(conoce(999_999)).await;
    assert_eq!(decisiones(&b), [D::ReanudaConFoto as i32; 3]);
    assert!(mensajes.iter().all(|m| matches!(m, ms::Cuerpo::Foto(_))) && mensajes.len() == 3);
    ana.cerrar().await;

    // Si el héroe salió de la plaza y volvió mientras tanto, los tres siguen por deltas: el registro dice qué cambió en cada ámbito en cada versión (desde T3c ya no se pide foto de la plaza por eso).
    let (_, r) = dar(&mut admin_c, "r3", colocar(ana_id, &ids::campamento(0))).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let (_, r) = dar(&mut admin_c, "r4", colocar(ana_id, &as_)).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let (_, b, mensajes) = w.conectar(conoce(v)).await;
    assert_eq!(decisiones(&b), [D::ReanudaConDeltas as i32; 3]);
    assert_eq!(etiquetas(&mensajes), [format!("delta {k_heroe}"), format!("delta {k_faccion}"), format!("delta {k_plaza}")]);
    w.parar().await;
}

#[tokio::test]
async fn sin_registro_que_alcance_la_version_del_cliente_la_reanudacion_es_con_foto() {
    let m = con_mundo!();
    // El registro solo conserva las dos últimas versiones.
    let x = aldea(m, Ajustes { retencion_cambios: 2, ..Ajustes::default() }, true);
    let (plaza, as_, h_ana) = (x.plaza, ids::asentamiento(x.plaza), ids::heroe(x.ana));
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (ana, b, _) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let v = b.ambitos[0].version_actual;
    ana.cerrar().await;
    let conoce = |v: u64| hola_con("ana", TipoCliente::ClienteUnity, &[(&format!("HEROE:{h_ana}"), v), (&format!("ASENTAMIENTO:{as_}"), v)]);

    // Con una sola versión por delante el registro todavía la alcanza: la plaza se reanuda por deltas.
    let (_, r) = dar(&mut admin_c, "r1", dar_madera(plaza, 1_000)).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let (ana, b, _) = w.conectar(conoce(v)).await;
    assert_eq!(b.ambitos.iter().map(|a| a.decision).collect::<Vec<_>>(), [D::ReanudaSinNovedad as i32, D::ReanudaConFoto as i32, D::ReanudaConDeltas as i32]);
    ana.cerrar().await;

    // Con cuatro, la versión del cliente ya se olvidó: foto de todo, también del héroe, que no había cambiado.
    for (i, cantidad) in [2_000, 3_000, 4_000].into_iter().enumerate() {
        let (_, r) = dar(&mut admin_c, &format!("s{i}"), dar_madera(plaza, cantidad)).await;
        assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    }
    let (_, b, mensajes) = w.conectar(conoce(v)).await;
    assert_eq!(b.ambitos.iter().map(|a| a.decision).collect::<Vec<_>>(), [D::ReanudaConFoto as i32; 3]);
    assert_eq!(mensajes.len(), 3);
    w.parar().await;
}

// ───────────────────────────── 5. Reparto en orden causal y sin filtraciones ─────────────────────────────

#[tokio::test]
async fn el_cambio_de_una_plaza_llega_a_todos_los_que_la_tienen_abierta_en_orden_y_a_nadie_mas() {
    let m = con_mundo!();
    let x = aldea(m, Ajustes::default(), true);
    let (plaza, as_) = (x.plaza, ids::asentamiento(x.plaza));
    let w = arrancar_con(x.n).await;
    let (mut admin_c, f_admin) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (mut ana, _, _) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let (mut dora, b_dora, _) = w.conectar(hola_con("dora", TipoCliente::ClienteBot, &[])).await;
    let (mut beto, _, _) = w.conectar(hola_con("beto", TipoCliente::ClienteUnity, &[])).await;
    let (mut gus, _, _) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;

    // Tres órdenes de ana sobre la plaza: cada una deja una delta de la plaza en ana antes del resultado, con la causalidad del contrato.
    let mut de_ana = Vec::new();
    for (i, valor) in [50_000, 60_000, 70_000].into_iter().enumerate() {
        let (previos, r) = dar_todo(&mut ana, &format!("o{i}"), calibrar_reserva(plaza, valor)).await;
        assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
        let deltas: Vec<&pb::Delta> = previos.iter().filter_map(|m| if let ms::Cuerpo::Delta(d) = m { Some(d) } else { None }).filter(|d| ambito(&d.ambito) == format!("ASENTAMIENTO:{as_}")).collect();
        assert_eq!(deltas.len(), 1, "{:?}", etiquetas(&previos));
        assert_eq!((plazas_de_delta(deltas[0]), deltas[0].hasta <= r.version_tras), (vec![as_.clone()], true));
        de_ana.push((deltas[0].desde, deltas[0].hasta));
    }
    assert!(de_ana.windows(2).all(|p| p[0].1 == p[1].0), "cada delta empieza donde acabó la anterior: {de_ana:?}");

    // Los demás que tienen la plaza, dora y la administración, reciben las mismas versiones, encadenadas desde su foto.
    let versiones_de = |previos: &[ms::Cuerpo], ambito_buscado: String| -> Vec<(u64, u64)> {
        previos.iter().filter_map(|m| if let ms::Cuerpo::Delta(d) = m { Some(d) } else { None }).filter(|d| ambito(&d.ambito) == ambito_buscado).map(|d| (d.desde, d.hasta)).collect()
    };
    let de_dora = versiones_de(&pendiente(&mut dora).await, format!("ASENTAMIENTO:{as_}"));
    let de_admin = versiones_de(&pendiente(&mut admin_c).await, "ADMIN:".into());
    for (quien, recibidas, inicio) in [("dora", de_dora, b_dora.ambitos[2].version_actual), ("admin", de_admin, f_admin.version)] {
        let hastas: Vec<u64> = recibidas.iter().map(|v| v.1).collect();
        assert!(de_ana.iter().all(|v| hastas.contains(&v.1)), "{quien} recibe las versiones de ana: {recibidas:?} frente a {de_ana:?}");
        assert_eq!(recibidas[0].0, inicio, "{quien} empieza donde acabó su foto");
        assert!(recibidas.windows(2).all(|p| p[0].1 == p[1].0), "{quien}: cadena rota {recibidas:?}");
    }

    // Quien no tiene el ámbito de la plaza, ni de su Facción (beto, en el campamento) ni de otra (gus), no recibe su interior: beto, de la misma Facción, solo la ve en resumen en el ámbito de su Facción, y gus no recibe nada.
    assert!(etiquetas(&pendiente(&mut beto).await).iter().all(|e| e == "delta FACCION:f-1"));
    assert!(pendiente(&mut gus).await.is_empty());
    w.parar().await;
}

// ───────────────────────────── 6. CambiarVista ─────────────────────────────

fn rect(a: Punto, b: Punto) -> mc::Cuerpo {
    mc::Cuerpo::Vista(pb::CambiarVista { rectangulo: Some(pb::Rectangulo { min: Some(pb::Punto { x: a.x, y: a.y }), max: Some(pb::Punto { x: b.x, y: b.y }) }) })
}

/// Las columnas de una foto de la vista (el resto —los héroes que van en ellas, la niebla...— no es de esta prueba).
fn ids_de_columnas(f: &[pb::Entidad]) -> Vec<&str> {
    columnas(f).into_iter().map(|e| e.id.as_str()).collect()
}

fn marchar(heroe: &str, p: Punto) -> orden::Cuerpo {
    use pb::orden_marchar_a::{objetivo::Tipo, Objetivo};
    orden::Cuerpo::MarcharA(pb::OrdenMarcharA { heroe_id: heroe.into(), objetivo: Some(Objetivo { tipo: Some(Tipo::Punto(pb::Punto { x: p.x, y: p.y })) }) })
}

#[tokio::test]
async fn cambiar_vista_abre_la_vista_de_la_faccion_con_lo_que_cae_en_el_rectangulo_y_la_rehace_al_cambiarlo() {
    // Tres columnas a más de 30 u unas de otras (nadie se ve): ana y beto de la Facción 1, gus de la 2.
    let mut n = nucleo(30, 30);
    n.crear_ejercito(0, "ana", 1, c(2, 2), 4_000, 350_000).unwrap(); // ej-1, héroe 2
    n.crear_ejercito(0, "beto", 1, c(12, 2), 4_000, 350_000).unwrap(); // ej-3, héroe 4
    n.crear_ejercito(0, "gus", 2, c(2, 25), 4_000, 350_000).unwrap(); // ej-5, héroe 6
    let w = arrancar_con(n).await;
    let (mut ana, b, mensajes) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    assert_eq!(etiquetas(&mensajes), ["foto HEROE:heroe-2", "foto FACCION:f-1"], "la vista no se abre hasta que el cliente dice qué mira");
    assert_eq!(b.ambitos.len(), 2);
    let (mut beto, _, _) = w.conectar(hola_con("beto", TipoCliente::ClienteUnity, &[])).await;
    let (mut gus, _, _) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;

    // Ana mira la esquina de su columna: la vista de su Facción trae la suya y no la de beto, que es de su Facción pero cae fuera. Gus mira todo el mapa: solo ve lo de su Facción.
    ana.enviar(rect(c(1, 1), c(4, 4))).await;
    let ms::Cuerpo::Foto(f) = ana.esperar().await.cuerpo.unwrap() else { panic!("se esperaba la Foto de la vista") };
    assert_eq!((ambito(&f.ambito), ids_de_columnas(&f.entidades)), ("VISTA:f-1".to_owned(), vec!["ej-1"]));
    gus.enviar(rect(c(0, 0), c(29, 29))).await;
    let ms::Cuerpo::Foto(f) = gus.esperar().await.cuerpo.unwrap() else { panic!("se esperaba la Foto de la vista") };
    assert_eq!((ambito(&f.ambito), ids_de_columnas(&f.entidades)), ("VISTA:f-2".to_owned(), vec!["ej-5"]));

    // Beto marcha: su columna, de la Facción de ana, anda fuera del rectángulo de ana, así que a ana no le llega nada (la vista no arrastra lo que cae fuera: desde T3d ni siquiera un QUITA de lo que nunca
    // tuvo); gus, nada. Beto anda sin salir del rectángulo que ana mirará después.
    let (_, r) = dar(&mut beto, "b1", marchar("heroe-4", c(13, 2))).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    assert!(pendiente(&mut ana).await.is_empty(), "lo que cae fuera del rectángulo no llega a la vista");
    assert!(pendiente(&mut gus).await.is_empty(), "lo de otra Facción no llega a su vista");

    // Ana marcha sin salir de su rectángulo: su columna llega entera a su héroe y a su vista; gus sigue sin recibir nada.
    let (previos, r) = dar_todo(&mut ana, "a1", marchar("heroe-2", c(3, 3))).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    assert_eq!(etiquetas(&previos), ["delta HEROE:heroe-2", "delta VISTA:f-1"]);
    let cambios = &delta_de(&previos[1]).cambios;
    assert_eq!((cambios.len(), cambios[0].tipo, cambios[0].reemplaza), (1, pb::TipoCambio::CambioActualiza as i32, true));
    assert!(pendiente(&mut gus).await.is_empty());

    // Ana cambia de rectángulo: foto nueva de la vista (la anterior se descarta) con lo que cae en el nuevo.
    ana.enviar(rect(c(11, 1), c(14, 4))).await;
    let ms::Cuerpo::Foto(f) = ana.esperar().await.cuerpo.unwrap() else { panic!("se esperaba la Foto de la vista") };
    assert_eq!((ambito(&f.ambito), ids_de_columnas(&f.entidades)), ("VISTA:f-1".to_owned(), vec!["ej-3"]));
    assert!(pendiente(&mut ana).await.is_empty(), "y no se revoca ni se repite nada");

    // Lo que no se puede: un rectángulo al revés o sin esquinas, la vista de quien no tiene Facción y la de una sesión de administración.
    ana.enviar(rect(c(4, 4), c(1, 1))).await;
    assert_eq!(aviso(&mut ana).await.codigo, "vista.invalida");
    ana.enviar(mc::Cuerpo::Vista(pb::CambiarVista { rectangulo: Some(pb::Rectangulo { min: None, max: None }) })).await;
    assert_eq!(aviso(&mut ana).await.codigo, "vista.invalida");
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    admin_c.enviar(rect(c(0, 0), c(29, 29))).await;
    assert_eq!(aviso(&mut admin_c).await.codigo, "vista.sin_derecho");
    let (mut nadie, _, _) = w.conectar(hola_con("nadie", TipoCliente::ClienteUnity, &[])).await;
    nadie.enviar(rect(c(0, 0), c(29, 29))).await;
    assert_eq!(aviso(&mut nadie).await.codigo, "vista.sin_derecho");
    w.parar().await;
}

// ───────────────────────────── 7. Una plaza que desaparece ─────────────────────────────

/// Cuándo cae en ruinas la plaza de la aldea sin provisiones: la misma partida (misma semilla, mismas órdenes) avanzada día a día hasta que desaparece.
fn instante_de_la_ruina(m: &M) -> Instante {
    let mut x = aldea(m, Ajustes { retencion_cambios: 100_000_000, ..Ajustes::default() }, false);
    let desde = x.n.version();
    let plaza = x.plaza;
    for dia in 1..=150 {
        x.n.avanzar_hasta(dia * 24 * 60 * MS_POR_MINUTO);
        if !x.n.estado().asentamientos.contains_key(&plaza) {
            return x.n.cambios_desde(desde).expect("el registro lo alcanza").flat_map(|c| c.eventos.iter()).find(|e| e.codigo == "asentamiento.ruinas").expect("el hecho de la ruina").en;
        }
    }
    panic!("la plaza no cae en ruinas en 150 días");
}

#[tokio::test]
async fn la_plaza_que_cae_en_ruinas_se_revoca_con_ya_no_existe_y_su_gente_sale_al_mapa() {
    let m = con_mundo!();
    let ruina = instante_de_la_ruina(m);
    // La misma partida, a unos segundos de la caída, con el reloj acelerado para que llegue sin esperar.
    let mut x = aldea(m, Ajustes::default(), false);
    x.n.avanzar_hasta(ruina - 3_000);
    let (as_, h_ana) = (ids::asentamiento(x.plaza), ids::heroe(x.ana));
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (mut ana, _, mensajes) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    assert_eq!(etiquetas(&mensajes)[2], format!("foto ASENTAMIENTO:{as_}"));
    let reloj = orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: "fijar_factor".into(), factor_milesimas: Some(200_000), hasta_ms: None });
    let (_, r) = dar(&mut admin_c, "t1", reloj).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);

    // La plaza desaparece: ana pierde su ámbito con el motivo de que ya no existe, y su héroe sale al mapa con una columna (un cambio de su ámbito).
    let mut vistos = Vec::new();
    let motivo = loop {
        match ana.esperar().await.cuerpo.expect("un mensaje") {
            ms::Cuerpo::Revocado(r) => break r.motivo,
            otro => vistos.push(etiqueta(&otro)),
        }
    };
    assert_eq!(motivo, "ya_no_existe", "antes: {vistos:?}");
    let siguiente = loop {
        let m = ana.esperar().await.cuerpo.expect("un mensaje");
        if !matches!(m, ms::Cuerpo::Aviso(_)) {
            break etiqueta(&m);
        }
    };
    assert_eq!(siguiente, format!("delta HEROE:{h_ana}"));
    w.parar().await;
}
