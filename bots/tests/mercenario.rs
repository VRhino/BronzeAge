//! El cerebro mercenario contra el servidor real, con un Rey humano guionizado que le contrata y le manda. Cada prueba, una orden: atacar, unirse, guardar e ir a, y la ronda sin contrato.
//! Necesita el servidor con los mercenarios por contrato (entrada 48 del motor).

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use bronce_age_bots_brain::{
    acceso, datos,
    llegadas::{Llegada, Perfil},
    mapa, mercenario,
    runner::{Config, Lugares, Registro},
    Cerebro, Contexto, Credenciales, Fallo, Proceso, Puerto,
};
use contrato::v1::{self as pb, entidad::Cuerpo, objetivo_mercenario::Tipo as Objetivo, orden::Cuerpo as O, orden_marchar_a, ubicacion_heroe::Donde, OrdenDeAdministracion, Punto, Resultado, TipoCliente, TipoEntidad};
use prost_types::{value::Kind, Struct, Value};

mod comun;

#[derive(Clone, Debug, Default)]
struct Visto {
    residencia: Option<String>,
    contratado: bool,
    /// Tuvo contrato y se quedó sin orden: cumplió su ataque.
    cumplio: bool,
    en_el_tablon: bool,
    /// Salió al mapa sin contrato.
    de_ronda: bool,
    dentro_de: Option<String>,
    columna: Option<(String, Punto)>,
    fallos: Vec<String>,
}

type Vistos = Arc<Mutex<BTreeMap<String, Visto>>>;

struct Mira(Vistos);

impl Cerebro for Mira {
    type Memoria = mercenario::Memoria;
    type Pizarra = ();
    fn plan(semilla: u64, total: u32, dias: f64) -> Vec<Llegada> {
        mercenario::plan_de_llegadas(semilla, total, dias)
    }
    async fn pensar(&self, ctx: &mut Contexto<'_, Self>) -> Result<(), Fallo> {
        let r = mercenario::pensar(ctx).await;
        let a = ctx.puerto.observar();
        let h = a.ambitos.get(&format!("HEROE:{}", ctx.yo)).and_then(|a| {
            a.entidades.values().find_map(|c| match c {
                Cuerpo::Heroe(h) if h.id == ctx.yo => Some(h.clone()),
                _ => None,
            })
        });
        let tablon = a.ambitos.values().flat_map(|a| a.entidades.values()).any(|c| matches!(c, Cuerpo::CampamentoMercenarios(k) if k.tablon.iter().any(|o| o.heroe_id == ctx.yo)));
        let col = bronce_age_bots_brain::cerebro::columna(a, ctx.yo).map(|e| (e.id.clone(), bronce_age_bots_brain::cerebro::posicion(&e, ctx.ahora_ms)));
        let mut v = self.0.lock().unwrap();
        let e = v.entry(ctx.yo.to_string()).or_default();
        e.en_el_tablon = tablon;
        e.residencia = h.as_ref().and_then(|h| h.residencia_id.clone());
        e.contratado = h.as_ref().is_some_and(|h| h.contrato.is_some());
        e.cumplio |= h.as_ref().and_then(|h| h.contrato.as_ref()).is_some_and(|c| c.orden.is_none());
        e.dentro_de = h.as_ref().and_then(|h| h.ubicacion.as_ref()).and_then(|u| u.donde.as_ref()).and_then(|d| match d {
            Donde::AsentamientoId(p) => Some(p.clone()),
            _ => None,
        });
        e.de_ronda |= !e.contratado && col.is_some();
        e.columna = col;
        if let Err(f) = &r {
            e.fallos.push(format!("{f:?}"));
        }
        r
    }
}

/// Un mundo con un mercenario ya en el tablón.
struct Prueba {
    _datos: tempfile::TempDir,
    _servidor: tokio::process::Child,
    dir: String,
    vistos: Vistos,
    proceso: Proceso,
    merc: String,
    campamento: String,
}

impl Prueba {
    async fn nueva() -> Option<Self> {
        let datos = tempfile::tempdir().unwrap();
        let Some((servidor, dir)) = comun::servidor(datos.path()).await else {
            eprintln!("sin binario del servidor o sin artefacto de mundo: se omite");
            return None;
        };
        let ruta = datos.path().join("registro.json");
        let cfg = Config { servidor: dir.clone(), codigo_bots: "bots".into(), registro: ruta.clone(), total: 1, dias_llegada: 1.0 / 86_400.0, semilla: 1, cada_ms: 1_000, siempre: true, lugares: Default::default() };
        let vistos: Vistos = Default::default();
        let proceso = Proceso::arrancar(cfg, Mira(vistos.clone())).await.expect("arrancar");
        let mut merc = None;
        hasta(
            "llega el mercenario",
            60,
            || {
                merc = Registro::leer(&ruta).unwrap().and_then(|r| r.bots.iter().find(|b| b.perfil == Perfil::Mercenario).map(|b| b.heroe_id.clone()));
                merc.is_some()
            },
            &vistos,
        )
        .await;
        let merc = merc.unwrap();
        hasta("se ofrece", 60, || vistos.lock().unwrap().get(&merc).is_some_and(|x| x.en_el_tablon), &vistos).await;
        let campamento = vistos.lock().unwrap()[&merc].residencia.clone().expect("reside en su campamento");
        Some(Self { _datos: datos, _servidor: servidor, dir, vistos, proceso, merc, campamento })
    }

    fn visto(&self) -> Visto {
        self.vistos.lock().unwrap()[&self.merc].clone()
    }

    async fn hasta(&self, que: &str, segundos: u64, f: impl Fn(&Visto) -> bool) {
        let v = self.vistos.clone();
        let m = self.merc.clone();
        hasta(que, segundos, || v.lock().unwrap().get(&m).is_some_and(&f), &self.vistos).await;
    }

    async fn acabar(self) {
        let v = self.visto();
        self.proceso.parar().await.expect("parar");
        assert!(v.fallos.is_empty(), "{v:#?}");
    }

    /// Un Rey humano en el campamento del mercenario, con su columna a la puerta y oro en el carro. Devuelve su puerto, su héroe, su columna y dónde está el campamento.
    async fn rey(&self) -> (Puerto, String, String, Punto) {
        let c = Credenciales { nick: "rey-humano".into(), clave: "clave del rey".into() };
        acceso::registrar(&self.dir, &c, "").await.expect("alta");
        let mut rey = Puerto::entrar_como(&self.dir, c, TipoCliente::ClienteUnity).await.expect("entrar");
        let casa = rey.bienvenida.campamentos_para_elegir.iter().find(|k| k.campamento_id == self.campamento).and_then(|k| k.posicion.clone()).expect("su campamento");
        let r = aceptada(rey.actuar(O::CrearHeroe(pb::OrdenCrearHeroe { display_name: "Rey".into(), campamento_id: self.campamento.clone(), class_definition_id: "dagas".into(), ..Default::default() })).await.unwrap());
        let yo = r.creadas.iter().find(|c| c.entidad == TipoEntidad::Heroe as i32).unwrap().id.clone();
        aceptada(rey.actuar(O::CrearFaccion(pb::OrdenCrearFaccion { nombre: "Humanos".into(), sigilo: None })).await.unwrap());
        comun::dar_almacen(&self.dir, &yo, "oro", 5_000_000.0).await;
        let r = aceptada(rey.actuar(O::PedirPrestamo(pb::OrdenPedirPrestamo { tropa_ids: vec!["milicia_lanceros".into()] })).await.unwrap());
        let escuadron_ids = r.creadas.iter().filter(|c| c.entidad == TipoEntidad::Escuadron as i32).map(|c| c.id.clone()).collect();
        let carga = Some(pb::Cantidades { por_id: [("oro".to_string(), 450_000)].into_iter().collect() });
        let politica_de_union = Some("aceptar".to_string());
        aceptada(rey.actuar(O::SalirDelCampamento(pb::OrdenSalirDelCampamento { campamento_id: self.campamento.clone(), heroe_id: yo.clone(), escuadron_ids, carga, politica_de_union, ..Default::default() })).await.unwrap());
        let col = bronce_age_bots_brain::cerebro::columna(rey.observar(), &yo).expect("su columna").id;
        (rey, yo, col, casa)
    }

    /// Le contrata para `trabajo` sobre `objetivo`, un día: el de tres no cabe en un carro.
    async fn contratar(&self, rey: &mut Puerto, trabajo: &str, objetivo: Objetivo) {
        let objetivo = Some(pb::ObjetivoMercenario { tipo: Some(objetivo) });
        aceptada(rey.actuar(O::ContratarMercenario(pb::OrdenContratarMercenario { mercenario_id: self.merc.clone(), trabajo: trabajo.into(), objetivo, plazo_dias: Some(1), residencia_asentamiento_id: None, pagar_con: None })).await.unwrap());
    }
}

fn aceptada(r: pb::ResultadoOrden) -> pb::ResultadoOrden {
    assert_eq!(r.resultado, Resultado::OrdenAceptada as i32, "{r:?}");
    r
}

async fn hasta(que: &str, segundos: u64, mut f: impl FnMut() -> bool, vistos: &Vistos) {
    for _ in 0..segundos * 10 {
        if f() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("no pasó: {que}\n{:#?}", vistos.lock().unwrap());
}

fn valor(k: Kind) -> Value {
    Value { kind: Some(k) }
}

fn punto(p: &Punto) -> Value {
    let f = [("x".to_string(), valor(Kind::NumberValue(p.x as f64))), ("y".to_string(), valor(Kind::NumberValue(p.y as f64)))];
    valor(Kind::StructValue(Struct { fields: f.into_iter().collect() }))
}

async fn administrar(dir: &str, codigo: &str, campos: Vec<(&str, Value)>) {
    let parametros = Some(Struct { fields: campos.into_iter().map(|(k, v)| (k.to_string(), v)).collect() });
    comun::administrar(dir, O::Administracion(OrdenDeAdministracion { codigo: codigo.into(), parametros })).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn ataca_lo_que_le_mandan_y_vuelve_a_ofrecerse() {
    let Some(p) = Prueba::nueva().await else { return };
    let (mut rey, _, _, casa) = p.rey().await;
    rey.mirar(pb::Rectangulo { min: Some(Punto { x: casa.x - 40_000, y: casa.y - 40_000 }), max: Some(Punto { x: casa.x + 40_000, y: casa.y + 40_000 }) }).await.expect("mirar");
    let bandidos = rey.observar().ambitos.values().flat_map(|a| a.entidades.values()).filter_map(|c| match c {
        Cuerpo::CampamentoBandidos(b) => Some((b.poder, b.id.clone())),
        _ => None,
    });
    let (_, bandidos) = bandidos.min().expect("hay bandidos cerca");
    p.contratar(&mut rey, "atacar", Objetivo::CampamentoId(bandidos)).await;
    comun::acelerar(&p.dir, 1440).await;
    p.hasta("cumple la orden", 120, |v| v.cumplio).await;
    p.hasta("acaba el contrato", 240, |v| !v.contratado).await;
    p.hasta("vuelve a ofrecerse", 120, |v| v.en_el_tablon).await;
    p.acabar().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn sin_contrato_sale_de_ronda_sin_dejar_el_tablon() {
    let Some(p) = Prueba::nueva().await else { return };
    let (x, y) = {
        let c = Credenciales { nick: "mirón".into(), clave: "clave del mirón".into() };
        acceso::registrar(&p.dir, &c, "").await.expect("alta");
        let mirón = Puerto::entrar_como(&p.dir, c, TipoCliente::ClienteUnity).await.expect("entrar");
        let casa = mirón.bienvenida.campamentos_para_elegir.iter().find(|k| k.campamento_id == p.campamento).and_then(|k| k.posicion.clone()).expect("su campamento");
        (casa.x, casa.y)
    };
    administrar(&p.dir, "colocar_bandidos", vec![("posicion", punto(&Punto { x: x + 15_000, y })), ("nivel", valor(Kind::NumberValue(1.0)))]).await;
    comun::acelerar(&p.dir, 360).await;
    p.hasta("sale de ronda", 120, |v| v.de_ronda).await;
    assert!(!p.visto().contratado);
    p.hasta("vuelve y sigue en el tablón", 180, |v| v.columna.is_none() && v.en_el_tablon).await;
    p.acabar().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn se_une_al_ejercito_que_acompana() {
    let Some(p) = Prueba::nueva().await else { return };
    let (mut rey, _, col, _) = p.rey().await;
    p.contratar(&mut rey, "acompanar", Objetivo::EjercitoId(col.clone())).await;
    comun::acelerar(&p.dir, 60).await;
    p.hasta("está en su columna", 120, |v| v.columna.as_ref().is_some_and(|c| c.0 == col)).await;
    p.acabar().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn guarda_la_plaza_y_va_donde_le_mandan() {
    let Some(p) = Prueba::nueva().await else { return };
    let (mut rey, yo, col, casa) = p.rey().await;
    comun::acelerar(&p.dir, 360).await;

    // Funda una plaza cerca y vuelve al campamento con oro en el carro.
    let (m, d) = (mapa::de(&rey).await.unwrap(), datos::de(&rey).await.unwrap());
    let sitio = (0..64)
        .flat_map(|i| {
            let a = i as f64 * 0.7;
            [50_000.0, 70_000.0].map(|r| Punto { x: casa.x + (r * a.cos()) as i64, y: casa.y + (r * a.sin()) as i64 })
        })
        .find(|s| m.fundable(s, &[], d.separacion_plazas, d.exclusion_campamento, d.bosque_denso))
        .expect("un sitio para fundar");
    administrar(&p.dir, "dar_caravana_fundacion", vec![("ejercito_id", valor(Kind::StringValue(col)))]).await;
    let marchar = |h: &str, s: &Punto| O::MarcharA(pb::OrdenMarcharA { heroe_id: h.into(), objetivo: Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(s.clone())) }) });
    aceptada(rey.actuar(marchar(&yo, &sitio)).await.unwrap());
    let mut plaza = None;
    for _ in 0..1200 {
        rey.atender(Duration::from_millis(100)).await.unwrap();
        let r = rey.actuar(O::Fundar(pb::OrdenFundar {})).await.unwrap();
        if r.resultado == Resultado::OrdenAceptada as i32 {
            plaza = r.creadas.iter().find(|c| c.entidad == TipoEntidad::Asentamiento as i32).map(|c| c.id.clone());
            break;
        }
    }
    let plaza = plaza.expect("fundó");
    // Quien funda entra a residir en la plaza y su columna se deshace dentro (el carro se queda con él): sale con su tropa y su carro.
    let escuadron_ids = rey.observar().ambitos.values().flat_map(|a| a.entidades.values()).filter_map(|c| match c {
        Cuerpo::Escuadron(e) if e.heroe_id == yo => Some(e.id.clone()),
        _ => None,
    });
    let salir = O::SalirAlMundo(pb::OrdenSalirAlMundo { asentamiento_id: plaza.clone(), heroe_id: yo.clone(), escuadron_ids: escuadron_ids.collect::<std::collections::BTreeSet<_>>().into_iter().collect(), carga: None });
    aceptada(rey.actuar(salir).await.unwrap());
    aceptada(rey.actuar(marchar(&yo, &casa)).await.unwrap());
    let objetivo = Some(pb::ObjetivoMercenario { tipo: Some(Objetivo::AsentamientoId(plaza.clone())) });
    let mut firmado = false;
    for _ in 0..1200 {
        rey.atender(Duration::from_millis(100)).await.unwrap();
        let o = O::ContratarMercenario(pb::OrdenContratarMercenario { mercenario_id: p.merc.clone(), trabajo: "defender".into(), objetivo: objetivo.clone(), plazo_dias: Some(1), residencia_asentamiento_id: None, pagar_con: None });
        if rey.actuar(o).await.unwrap().resultado == Resultado::OrdenAceptada as i32 {
            firmado = true;
            break;
        }
    }
    assert!(firmado, "no llegó a contratarlo");

    comun::acelerar(&p.dir, 60).await;
    p.hasta("entra en la plaza", 120, |v| v.dentro_de.as_deref() == Some(plaza.as_str())).await;

    // Ir a: sale y marcha hasta allí.
    let destino = (0..64).map(|i| i as f64 * 0.7).map(|a| Punto { x: sitio.x + (30_000.0 * a.cos()) as i64, y: sitio.y + (30_000.0 * a.sin()) as i64 }).find(|s| m.transitable(s)).expect("un sitio al que ir");
    let o = pb::OrdenOrdenarMercenario { mercenario_ids: vec![p.merc.clone()], tipo: "ir_a".into(), objetivo: Some(pb::ObjetivoMercenario { tipo: Some(Objetivo::Punto(destino.clone())) }) };
    aceptada(rey.actuar(O::OrdenarMercenario(o)).await.unwrap());
    p.hasta("llega al sitio", 120, |v| v.columna.as_ref().is_some_and(|c| ((c.1.x - destino.x) as f64).hypot((c.1.y - destino.y) as f64) < d.radio_encuentro as f64 + 1_000.0)).await;
    p.acabar().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn llena_sus_campamentos_en_orden() {
    let datos = tempfile::tempdir().unwrap();
    let Some((_servidor, dir)) = comun::servidor(datos.path()).await else { return };
    let lugares = Lugares { campamentos: vec!["camp-5".into(), "camp-2".into()], por_campamento: 1 };
    let cfg = Config { servidor: dir, codigo_bots: "bots".into(), registro: datos.path().join("registro.json"), total: 3, dias_llegada: 3.0 / 86_400.0, semilla: 1, cada_ms: 1_000, siempre: true, lugares };
    let vistos: Vistos = Default::default();
    let p = Proceso::arrancar(cfg, Mira(vistos.clone())).await.expect("arrancar");
    let residencias = || vistos.lock().unwrap().values().filter_map(|v| v.residencia.clone()).collect::<Vec<_>>();
    hasta("nacen los tres", 60, || residencias().len() == 3, &vistos).await;
    let mut r = residencias();
    r.sort();
    p.parar().await.expect("parar");
    assert_eq!(r, ["camp-2", "camp-2", "camp-5"], "uno llena el primero, y lleno todo, al que menos tiene");
}
