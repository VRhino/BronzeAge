//! El interior de la plaza y del campamento (4H, T3c) por el servidor WebSocket real y el actor del núcleo: la foto de la plaza vista desde dentro, los cambios parciales con máscara del paso
//! de minuto (con la tendencia «+N/min» de cada contador), una obra que termina en su instante, la mejora, el recinto, la política, la guarnición, el visitante que entra y sale, el acceso que
//! expulsa, el campamento con su compra y su préstamo, que nada llega a quien no está dentro, la reanudación por deltas y «foto + deltas = foto final».
//!
//! El reloj de mundo se **pausa** y se adelanta con `controlar_reloj` de la administración (`avanzar_hasta`): cada paso de minuto y cada fin de obra llegan en su instante, sin depender del
//! reloj de pared. Van sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`) y se omiten si no está.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::Recurso::*;
use base::{Instante, Punto, MS_POR_MINUTO};
use comun::sesion::*;
use contrato::v1::{self as pb, mensaje_de_servidor as ms, orden, DecisionReanudacion as D, TipoCliente};
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::proyeccion::{ids, interior};
use nucleo::{Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Resultado};
use prost_types::{value::Kind, Struct, Value};
use rutas::Rutas;
use servidor::prueba::{hola, ClientePrueba};
use std::sync::{Arc, OnceLock};

const MIN: i64 = MS_POR_MINUTO;

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

// ───────────────────────────── Un mundo con una plaza, dos campamentos y cinco héroes ─────────────────────────────

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

struct Aldea {
    n: Nucleo,
    plaza: u64,
    /// Ciudadanos de la Facción 1, residentes y dentro de la plaza.
    ana: u64,
    dora: u64,
    /// Ciudadano de la Facción 1, residente del campamento 0 y dentro de él, con almacén personal.
    beto: u64,
    /// Ciudadano de la Facción 2, residente del campamento 0 y dentro de él, con almacén personal.
    gus: u64,
    /// El pueblo de cobre levantado de la plaza (solo en `aldea_con(.., true)`).
    pueblo: u64,
    peticiones: u64,
}

impl Aldea {
    fn ok(&mut self, actor: &str, c: Comando) -> nucleo::Respuesta {
        self.peticiones += 1;
        let p = Peticion::nueva(actor, &format!("prep{}", self.peticiones), c);
        let t = self.n.ahora();
        let r = self.n.aplicar_en(t, if actor == "admin" { p.de_admin() } else { p });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
        r
    }

    fn heroe(&mut self, cuenta: &str, faccion: u32, residencia: Option<Residencia>, dentro: Option<Ubicacion>) -> u64 {
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano }).creadas[0];
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(faccion) });
        if let Some(r) = residencia {
            self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(r) });
        }
        if let Some(u) = dentro {
            self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: u });
        }
        h
    }
}

/// Un sitio fundable, verde y con mucho bosque, a `r` del punto `c`.
fn sitio_junto_a(n: &Nucleo, c: Punto, r: i64) -> Option<Punto> {
    (0..36).map(|g| ((g * 10) as f64).to_radians()).map(|a| Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64)).find(|&p| {
        n.viabilidad(p, 1).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6 && !v.condenada)
    })
}

/// Cómo se prepara la aldea de una prueba.
#[derive(Default)]
struct Opc<'a> {
    /// El núcleo persiste en este directorio (y deja una foto al terminar de prepararse, porque la preparación no pasa por el registro).
    dir: Option<&'a std::path::Path>,
    /// La plaza se funda junto a una mina de cobre y tiene su pueblo levantado.
    pueblo: bool,
    /// Ana no tiene escuadra: sin tropa que coma, el paso de minuto no toca a ningún héroe y no cierra versión por sí solo.
    sin_tropa: bool,
}

fn aldea(m: &M) -> Aldea {
    aldea_con(m, Opc::default())
}

/// La plaza en nivel 2 y con la zona agrandada (admite recintos), con de todo en el almacén y sin construcción automática; ana tiene una escuadra de lanceros en el campamento de la plaza.
fn aldea_con(m: &M, Opc { dir, pueblo, sin_tropa }: Opc) -> Aldea {
    let mut reglas = Reglas::default();
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let arranque = Arranque { reglas, ajustes, semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) };
    let n = match dir {
        Some(d) => Nucleo::con_persistencia(arranque, d, Default::default()).expect("el almacén se abre"),
        None => Nucleo::nuevo(arranque),
    };
    let mut x = Aldea { n, plaza: 0, ana: 0, dora: 0, beto: 0, gus: 0, pueblo: 0, peticiones: 0 };
    let mut nodo_cobre = 0;
    let sitio = if pueblo {
        let mapa = nucleo::economia::Mapa::de(&m.0);
        let (cobre, sitio) = (0..mapa.nodos.len())
            .filter(|&i| mapa.nodos[i].tipo == Tipo::MinaCobre)
            .find_map(|i| sitio_junto_a(&x.n, mapa.nodos[i].pos, 6_000).map(|p| (i as u32, p)))
            .expect("hay un nodo de cobre junto a un sitio rico");
        nodo_cobre = cobre;
        sitio
    } else {
        sitio_rico(&x.n, &m.0, 1)
    };
    let columna = x.n.crear_ejercito(0, "fundador", 1, sitio, 500, 100_000).unwrap();
    x.ok("admin", Comando::DarCaravanaFundacion { ejercito: columna });
    x.plaza = x.ok("fundador", Comando::Fundar { ejercito: columna }).creadas[0];
    let plaza = x.plaza;
    x.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
    x.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 2 });
    for _ in 0..14 {
        x.ok("admin", Comando::AgrandarZona { asentamiento: plaza });
    }
    for r in [Madera, Piedra, Oro, Trigo] {
        x.ok("admin", Comando::DarRecursos { asentamiento: plaza, recurso: r, cantidad: 300_000 });
    }
    let (en_la_plaza, en_el_campamento) = (Some(Residencia::Asentamiento(plaza as u32)), Some(Residencia::Campamento(0)));
    x.ana = x.heroe("ana", 1, en_la_plaza, Some(Ubicacion::Asentamiento(plaza as u32)));
    x.dora = x.heroe("dora", 1, en_la_plaza, Some(Ubicacion::Asentamiento(plaza as u32)));
    x.beto = x.heroe("beto", 1, en_el_campamento, None);
    x.gus = x.heroe("gus", 2, None, None);
    let ana = x.ana;
    if !sin_tropa {
        x.ok("admin", Comando::DarEscuadra { heroe: ana, tropa: Tropa::Lenadores, cantidad: militar::Reglas::cargar().tropas.tropa(Tropa::Lenadores).unidades });
    }
    // Las dos Facciones adoptan todas las tecnologías de la tropa: el campamento ofrece lo que su edificio militar admite.
    let tecnologias: Vec<String> = militar::Reglas::cargar().tropas.iter().map(|d| d.tecnologia.clone()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    for faccion in [1, 2] {
        x.ok("admin", Comando::FijarTecnologias { faccion, tecnologias: tecnologias.clone() });
    }
    for h in [x.beto, x.gus] {
        for (r, c) in [(Oro, 50_000_000), (Madera, 200_000), (Piedra, 200_000)] {
            x.ok("admin", Comando::DarAlmacenPersonal { heroe: h, recurso: r, cantidad: c });
        }
    }
    if pueblo {
        x.pueblo = x.ok("fundador", Comando::MarcarPueblo { asentamiento: plaza, nodo: nodo_cobre }).creadas[0];
        let puerta = x.n.posicion_de_pueblo(x.pueblo).unwrap();
        let t = x.n.ahora();
        let obrera = x.n.crear_ejercito(t, "fundador", 1, puerta, 500, 100_000).unwrap();
        x.ok("admin", Comando::DarSuministro { ejercito: obrera, recurso: Madera, cantidad: 100_000 });
        x.ok("admin", Comando::DarSuministro { ejercito: obrera, recurso: Oro, cantidad: 12_000 });
        let pueblo = x.pueblo;
        x.ok("fundador", Comando::EntregarMateriales { ejercito: obrera, pueblo });
        let asentamiento::pueblos::EstadoPueblo::EnObra { completa_en } = x.n.pueblo(pueblo).unwrap().1.estado else { panic!("debía estar en obra") };
        x.n.avanzar_hasta(completa_en + MIN);
        assert!(x.n.pueblo(pueblo).unwrap().1.activo());
    }
    // El mundo, un segundo después de un minuto de pared: nada del reloj del mundo cae entre medias (las pruebas lo pausan nada más conectar).
    let ahora = x.n.ahora();
    x.n.avanzar_hasta((ahora / MIN + 1) * MIN + 1_000);
    // La preparación (columna, fundación, héroes) no pasa por el registro: una foto la recoge.
    x.n.foto_ahora();
    x
}

// ───────────────────────────── Lo que llega ─────────────────────────────

fn ambito(a: &Option<pb::Ambito>) -> String {
    let a = a.as_ref().expect("un ámbito");
    format!("{}:{}", pb::AmbitoTipo::try_from(a.tipo).unwrap().as_str_name().trim_start_matches("AMBITO_"), a.id)
}

fn hola_con(cliente: &str, tipo: TipoCliente, conocidas: &[(&str, u64)]) -> pb::Hola {
    let mut h = hola(&format!("ok:{cliente}"), tipo);
    h.versiones_conocidas.extend(conocidas.iter().map(|(k, v)| ((*k).to_owned(), *v)));
    h
}

fn texto(x: &str) -> Value {
    Value { kind: Some(Kind::StringValue(x.into())) }
}

fn admin(codigo: &str, parametros: &[(&str, &str)]) -> orden::Cuerpo {
    orden::Cuerpo::Administracion(pb::OrdenDeAdministracion { codigo: codigo.into(), parametros: Some(Struct { fields: parametros.iter().map(|(k, v)| ((*k).to_owned(), texto(v))).collect() }) })
}

fn colocar(heroe: u64, sitio: &str) -> orden::Cuerpo {
    admin("colocar_heroe", &[("heroe_id", &ids::heroe(heroe)), ("ubicacion", sitio)])
}

fn s(x: &str) -> String {
    x.into()
}

fn aceptada(r: &pb::ResultadoOrden) {
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
}

/// Lo que ya estaba en camino hacia la sesión: el actor contesta a una orden que rechaza después de repartir todo lo anterior, y el servidor entrega en orden.
async fn pendiente(c: &mut ClientePrueba) -> Vec<ms::Cuerpo> {
    let (previos, r) = dar_todo(c, "barrera", orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: "ej-999999".into() })).await;
    assert_eq!(rechazo(&r), "ejercito.desconocido");
    previos
}

fn es_de(d: &pb::Delta, tipo: pb::AmbitoTipo) -> bool {
    d.ambito.as_ref().is_some_and(|a| a.tipo == tipo as i32)
}

/// La plaza de una foto de `ASENTAMIENTO`.
fn plaza_de_foto(f: &pb::Foto) -> pb::Asentamiento {
    let plazas: Vec<_> = f.entidades.iter().filter_map(|e| if let Some(pb::entidad::Cuerpo::Asentamiento(a)) = &e.cuerpo { Some(a.clone()) } else { None }).collect();
    assert_eq!(plazas.len(), 1, "una sola plaza en la foto de su ámbito");
    plazas.into_iter().next().unwrap()
}

/// Un cliente dentro de la plaza: su conexión y la plaza tal como la tiene (la foto y, aplicadas con su máscara, las deltas que le han llegado).
struct Dentro {
    c: ClientePrueba,
    plaza: pb::Asentamiento,
    /// Los pueblos que tiene, por id.
    pueblos: std::collections::BTreeMap<String, pb::Pueblo>,
    /// Lo último que sabe de la versión del ámbito.
    version: u64,
}

/// Lo que un cliente que se desconecta recuerda de la plaza: con ello se reconecta diciendo qué versión tiene.
struct Recuerdo {
    plaza: pb::Asentamiento,
    pueblos: std::collections::BTreeMap<String, pb::Pueblo>,
    version: u64,
}

impl Dentro {
    /// Conecta a `cuenta`. Con `anterior` es una reconexión: dice conocer la plaza en su versión y parte de ella (si el servidor la reanuda, solo le manda lo que cambió).
    async fn entrar(w: &Servicio, cuenta: &str, anterior: Option<&Recuerdo>) -> (Dentro, pb::Bienvenida) {
        let conocidas: Vec<(String, u64)> = anterior.iter().map(|a| (format!("ASENTAMIENTO:{}", a.plaza.id), a.version)).collect();
        let conocidas: Vec<(&str, u64)> = conocidas.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        let mut hola = hola_con(cuenta, TipoCliente::ClienteUnity, &conocidas);
        // La secuencia interior que conserva (4H T3c): con ella el servidor sabe si la plaza avanzó por pasos de minuto desde que se fue.
        hola.versiones_interiores_conocidas.extend(anterior.iter().map(|a| (format!("ASENTAMIENTO:{}", a.plaza.id), a.plaza.version_interior)));
        let (c, b, mensajes) = w.conectar(hola).await;
        let mut d = Dentro { c, plaza: anterior.map(|a| a.plaza.clone()).unwrap_or_default(), pueblos: anterior.map(|a| a.pueblos.clone()).unwrap_or_default(), version: anterior.map_or(0, |a| a.version) };
        for m in mensajes {
            match m {
                ms::Cuerpo::Foto(f) if f.ambito.as_ref().is_some_and(|a| a.tipo == pb::AmbitoTipo::AmbitoAsentamiento as i32) => {
                    d.plaza = plaza_de_foto(&f);
                    d.version = f.version;
                    for e in f.entidades {
                        if let Some(pb::entidad::Cuerpo::Pueblo(p)) = e.cuerpo {
                            d.pueblos.insert(p.id.clone(), p);
                        }
                    }
                }
                ms::Cuerpo::Delta(delta) if es_de(&delta, pb::AmbitoTipo::AmbitoAsentamiento) => d.aplicar(&delta),
                _ => {}
            }
        }
        (d, b)
    }

    /// Cierra la conexión y se queda con lo que sabía de la plaza.
    async fn cerrar(self) -> Recuerdo {
        self.c.cerrar().await;
        Recuerdo { plaza: self.plaza, pueblos: self.pueblos, version: self.version }
    }

    /// Aplica una delta de la plaza como lo hace el cliente: el cambio completo sustituye, el parcial copia los campos de su máscara.
    fn aplicar(&mut self, d: &pb::Delta) {
        assert_eq!(d.desde, self.version, "la cadena de versiones del ámbito es contigua: {d:?}");
        for c in &d.cambios {
            let Some(pb::Entidad { cuerpo }) = &c.datos else {
                if c.entidad == pb::TipoEntidad::Pueblo as i32 {
                    self.pueblos.remove(&c.id);
                }
                continue;
            };
            match cuerpo {
                Some(pb::entidad::Cuerpo::Asentamiento(a)) if c.reemplaza => self.plaza = a.clone(),
                Some(pb::entidad::Cuerpo::Asentamiento(a)) => interior::aplicar(&mut self.plaza, a, &c.campos),
                Some(pb::entidad::Cuerpo::Pueblo(p)) => {
                    self.pueblos.insert(p.id.clone(), p.clone());
                }
                otro => panic!("el ámbito de la plaza no lleva {otro:?}"),
            }
        }
        self.version = d.hasta;
    }

    /// Da una orden y espera su resultado. Las deltas de la plaza que llegaron antes del resultado (las de la orden y lo que el reloj ya tenía que decir) se aplican y se devuelven.
    async fn dar(&mut self, peticion: &str, cuerpo: orden::Cuerpo) -> (Vec<pb::Delta>, pb::ResultadoOrden) {
        let (previos, r) = dar_todo(&mut self.c, peticion, cuerpo).await;
        let deltas: Vec<pb::Delta> = previos.into_iter().filter_map(|m| if let ms::Cuerpo::Delta(d) = m { Some(d) } else { None }).filter(|d| es_de(d, pb::AmbitoTipo::AmbitoAsentamiento)).collect();
        deltas.iter().for_each(|d| self.aplicar(d));
        (deltas, r)
    }

    /// Todo lo que lleva en camino la plaza: aplica las deltas de `ASENTAMIENTO` y devuelve el resto (más los eventos de las deltas de plaza, con sus cambios).
    async fn al_dia(&mut self) -> Vec<pb::Delta> {
        let mut deltas = Vec::new();
        for m in pendiente(&mut self.c).await {
            match m {
                ms::Cuerpo::Delta(d) if es_de(&d, pb::AmbitoTipo::AmbitoAsentamiento) => {
                    self.aplicar(&d);
                    deltas.push(d);
                }
                ms::Cuerpo::Delta(_) | ms::Cuerpo::Foto(_) | ms::Cuerpo::Revocado(_) => {}
                otro => panic!("llegó {otro:?}"),
            }
        }
        deltas
    }
}

/// Pausa el reloj de mundo (la administración) para que solo avance con `avanzar_hasta`.
async fn pausar(admin_c: &mut ClientePrueba) {
    let (_, r) = dar(admin_c, "pausa", orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: "pausar".into(), factor_milesimas: None, hasta_ms: None })).await;
    aceptada(&r);
}

async fn avanzar_hasta(admin_c: &mut ClientePrueba, peticion: &str, t: Instante) {
    let (_, r) = dar(admin_c, peticion, orden::Cuerpo::ControlarReloj(pb::OrdenControlarReloj { accion: "avanzar_hasta".into(), factor_milesimas: None, hasta_ms: Some(t) })).await;
    aceptada(&r);
}

/// El instante del próximo paso de minuto de la plaza, estrictamente después de `ahora`.
fn proximo_paso(n: &Nucleo, plaza: u64) -> Instante {
    let fundado = n.estado().asentamientos[&plaza].fundado_en;
    fundado + ((n.ahora() - fundado) / MIN + 1) * MIN
}

fn existencia<'a>(a: &'a pb::Asentamiento, recurso: &str) -> &'a pb::Ritmo {
    a.almacen[recurso].cantidad.as_ref().unwrap_or_else(|| panic!("sin cantidad de {recurso}"))
}

fn edificio<'a>(a: &'a pb::Asentamiento, id: &str) -> &'a pb::Edificio {
    a.edificios.iter().find(|e| e.id == id).unwrap_or_else(|| panic!("la plaza no tiene {id}"))
}

/// Dice qué campos de la plaza difieren (nada, si son iguales): un `assert_eq!` de dos plazas vuelca cuatro pantallas.
fn iguales(esperada: &pb::Asentamiento, recibida: &pb::Asentamiento, que: &str) {
    let diferencias = interior::mascara(recibida, esperada).map(|(campos, _)| campos);
    assert!(diferencias.is_none(), "{que}: la plaza difiere en {diferencias:?}");
}

/// La plaza del núcleo tal como la proyecta ahora (lo que un cliente al día debe tener).
fn proyeccion(n: &Nucleo, plaza: u64) -> pb::Asentamiento {
    interior::asentamiento(n, plaza).expect("la plaza existe")
}

// ───────────────────────────── 1. La foto de la plaza desde dentro ─────────────────────────────

#[tokio::test]
async fn la_foto_de_la_plaza_trae_su_interior_entero_y_es_la_proyeccion_del_nucleo() {
    let m = con_mundo!();
    let x = aldea(m);
    let (plaza, as_) = (x.plaza, ids::asentamiento(x.plaza));
    let (h_ana, h_dora) = (ids::heroe(x.ana), ids::heroe(x.dora));
    let w = arrancar_con(x.n).await;
    let (ana, b) = Dentro::entrar(&w, "ana", None).await;
    assert_eq!(ana.plaza.id, as_);
    let p = &ana.plaza;

    // El almacén en escalones: los 29 recursos con su valor, su capacidad y su tendencia; la población entera por clase con la marca de recuperación.
    assert_eq!(p.almacen.len(), 29);
    assert!(p.almacen.values().all(|e| e.cantidad.as_ref().is_some_and(|c| c.en_escalones) && e.capacidad > 0));
    assert_eq!(existencia(p, "madera").maximo, Some(p.almacen["madera"].capacidad), "el contador no pasa de la capacidad");
    let pob = p.poblacion.as_ref().unwrap();
    for r in [&pob.pesants, &pob.artesanos, &pob.nobleza] {
        let r = r.as_ref().unwrap();
        assert!(r.en_escalones && r.valor_en_t0 % 1_000 == 0, "la población es entera: {r:?}");
    }
    assert!(pob.pesants.as_ref().unwrap().valor_en_t0 > 0);
    assert!(!pob.en_recuperacion);
    assert!(p.nutricion.as_ref().unwrap().en_escalones && p.medidor_mantenimiento.as_ref().unwrap().en_escalones);

    // La ciudad: los edificios con su estado, su sitio y su huella, el trazado y los recintos (ninguno aún).
    assert!(p.edificios.len() >= 5, "el Centro Urbano, las viviendas y la granja de la fundación");
    assert!(p.edificios.iter().all(|e| e.id.starts_with(&format!("ed-{plaza}-")) && e.posicion.is_some() && e.ancho > 0 && e.alto > 0));
    assert!(p.edificios.iter().any(|e| e.tipo == "centroUrbano" && e.estado == pb::EstadoEdificio::EdificioActivo as i32));
    let t = p.trazado.as_ref().unwrap();
    assert_eq!((t.layout_version, t.unidades_por_celda), (trazado::LAYOUT_VERSION, trazado::CELDA_UL as u32));
    assert!(t.radio_ul > 0 && !t.calles.is_empty());
    assert!(p.recintos.is_empty() && p.ascenso.is_none() && p.politicas_activas.is_empty());

    // Quién vive y quién está dentro, el acceso por defecto (cerrado a los enemigos), la guarnición y la cuenta de la flota.
    assert_eq!(p.casas_compradas, [h_ana.clone(), h_dora.clone()]);
    assert_eq!(p.presentes_ids, [h_ana, h_dora]);
    assert_eq!(p.puerta_cerrada_a, [pb::GrupoPuerta::GrupoEnemigos as i32]);
    assert!(p.guarnicion_ids.is_empty() && p.caravanas_aparcadas_ids.is_empty() && p.flota_ids.is_empty() && p.batalla_id.is_none());
    assert!(b.ambitos.iter().any(|a| ambito(&a.ambito) == format!("ASENTAMIENTO:{as_}") && a.decision == D::ReanudaConFoto as i32));

    // Es exactamente lo que el núcleo proyecta ahora.
    let n = w.parar().await;
    iguales(&proyeccion(&n, plaza), &ana.plaza, "la foto");
    assert_eq!(ana.plaza.cupo_guarnicion, u32::try_from(n.cupo_de_guarnicion(plaza).unwrap()).unwrap(), "el cupo de guarnición del nivel");
}

// ───────────────────────────── 2. El paso de minuto: parcial, en escalones y con su tendencia ─────────────────────────────

#[tokio::test]
async fn el_paso_de_minuto_manda_solo_lo_que_cambia_con_su_mascara_y_la_tendencia_es_el_neto_del_paso() {
    let m = con_mundo!();
    let x = aldea(m);
    let (plaza, as_) = (x.plaza, ids::asentamiento(x.plaza));
    let paso = proximo_paso(&x.n, plaza);
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    pausar(&mut admin_c).await;
    let (mut ana, _) = Dentro::entrar(&w, "ana", None).await;
    let antes = ana.plaza.clone();

    // Un solo paso de minuto: llega una delta de la plaza, parcial, con la máscara de lo que cambió.
    avanzar_hasta(&mut admin_c, "m1", paso).await;
    let deltas = ana.al_dia().await;
    assert_eq!(deltas.len(), 1, "una delta por paso");
    let cambio = &deltas[0].cambios[0];
    assert_eq!((cambio.id.as_str(), cambio.reemplaza), (as_.as_str(), false));
    assert_eq!(cambio.campos[..2], ["version", "version_interior"]);
    // El paso de minuto avanza la secuencia propia del interior (de uno en uno) y no la versión de la plaza (que sale en la misma máscara).
    assert_eq!(ana.plaza.version_interior, antes.version_interior + 1);
    assert_eq!(ana.plaza.version, antes.version, "la ficha de la plaza no cambió: la versión de la plaza es la del mundo en que cambió por última vez");
    let permitidos = ["version", "version_interior", "almacen", "poblacion", "nutricion", "medidor_mantenimiento", "racha_mantenimiento_sano"];
    assert!(cambio.campos.iter().all(|c| permitidos.contains(&c.as_str())), "solo el almacén y la población, no la ciudad: {:?}", cambio.campos);
    assert!(cambio.campos.iter().any(|c| c == "almacen"));
    let datos = match cambio.datos.as_ref().unwrap().cuerpo.as_ref().unwrap() {
        pb::entidad::Cuerpo::Asentamiento(a) => a,
        otro => panic!("{otro:?}"),
    };
    assert!(datos.edificios.is_empty() && datos.trazado.is_none() && datos.zona.is_empty() && datos.recintos.is_empty(), "lo que no cambia no viaja");

    // La tendencia es el neto real del paso: lo que cambió cada recurso y cada clase de la población (con el escalón del servidor, sin interpolar).
    let mut algo_cambio = false;
    for (r, e) in &ana.plaza.almacen {
        let (antes_r, ahora_r) = (antes.almacen[r].cantidad.as_ref().unwrap(), e.cantidad.as_ref().unwrap());
        assert!(ahora_r.en_escalones);
        assert_eq!(ahora_r.valor_en_t0 - antes_r.valor_en_t0, ahora_r.por_minuto, "tendencia de {r}");
        algo_cambio |= ahora_r.por_minuto != 0;
    }
    assert!(algo_cambio, "la granja produce y la población come");
    let (pa, pn) = (antes.poblacion.as_ref().unwrap(), ana.plaza.poblacion.as_ref().unwrap());
    for (a, n) in [(&pa.pesants, &pn.pesants), (&pa.artesanos, &pn.artesanos), (&pa.nobleza, &pn.nobleza)] {
        let (a, n) = (a.as_ref().unwrap(), n.as_ref().unwrap());
        assert_eq!(n.valor_en_t0 - a.valor_en_t0, n.por_minuto, "tendencia de la población");
        assert_eq!(n.valor_en_t0 % 1_000, 0);
    }

    // Otro paso: otra delta, con la tendencia del nuevo paso. Foto + deltas = lo que el núcleo proyecta.
    let antes = ana.plaza.clone();
    avanzar_hasta(&mut admin_c, "m2", paso + MIN).await;
    assert_eq!(ana.al_dia().await.len(), 1);
    for (r, e) in &ana.plaza.almacen {
        let (a, n) = (antes.almacen[r].cantidad.as_ref().unwrap(), e.cantidad.as_ref().unwrap());
        assert_eq!(n.valor_en_t0 - a.valor_en_t0, n.por_minuto, "tendencia de {r} en el segundo paso");
    }
    assert_eq!(ana.plaza.version_interior, antes.version_interior + 1);
    let n = w.parar().await;
    iguales(&proyeccion(&n, plaza), &ana.plaza, "plaza");
    assert_eq!(ana.plaza.version_interior, n.version_interior(plaza).unwrap());
    assert_eq!(n.estado().asentamientos[&plaza].economia.tendencia.almacen[Trigo.indice()], existencia(&ana.plaza, "trigo").por_minuto, "la tendencia que viaja es la guardada");
}


// ───────────────────────────── 2b. El almacén local de los pueblos con el paso de minuto ─────────────────────────────

/// El pueblo de la plaza viaja con ella: el paso de minuto refresca su almacén local sin subir la versión del mundo, con la secuencia interior de su plaza madre (`Pueblo.version_interior`); la Facción, en
/// cambio, no lo recibe por minuto (DUDA de T3c: su almacén local llega con el último cambio discreto del pueblo).
#[tokio::test]
async fn el_almacen_local_del_pueblo_se_refresca_en_el_paso_de_minuto_por_el_interior_de_la_plaza() {
    let m = con_mundo!();
    let x = aldea_con(m, Opc { pueblo: true, sin_tropa: true, ..Default::default() });
    let (plaza, pueblo) = (x.plaza, x.pueblo);
    let (as_, pueblo_id, paso) = (ids::asentamiento(plaza), ids::pueblo(pueblo), proximo_paso(&x.n, plaza));
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    pausar(&mut admin_c).await;
    let (mut ana, _) = Dentro::entrar(&w, "ana", None).await;
    assert_eq!(ana.pueblos.len(), 1, "el pueblo viene en la foto de la plaza: {:?}", ana.pueblos.keys());
    assert_eq!(ana.pueblos[&pueblo_id].asentamiento_madre_id, as_);
    assert_eq!(ana.pueblos[&pueblo_id].version_interior, ana.plaza.version_interior);
    let almacen = |c: &Dentro| c.pueblos[&pueblo_id].almacen_local.as_ref().unwrap().cantidad.as_ref().unwrap().valor_en_t0;
    let (inicial, version_inicial) = (almacen(&ana), ana.plaza.version_interior);

    // Tres pasos de minuto: la plaza trae el pueblo con su almacén y la secuencia de cada paso; a la Facción no le llega ningún pueblo.
    for (i, k) in [paso, paso + MIN, paso + 2 * MIN].into_iter().enumerate() {
        avanzar_hasta(&mut admin_c, &format!("m{i}"), k).await;
        let mut de_plaza = 0;
        for m in pendiente(&mut ana.c).await {
            let ms::Cuerpo::Delta(d) = m else { continue };
            if es_de(&d, pb::AmbitoTipo::AmbitoAsentamiento) {
                assert!(d.cambios.iter().any(|c| c.entidad == pb::TipoEntidad::Pueblo as i32 && c.id == pueblo_id), "el pueblo viaja con el interior: {:?}", d.cambios.iter().map(|c| (c.entidad, &c.id)).collect::<Vec<_>>());
                ana.aplicar(&d);
                de_plaza += 1;
            } else if es_de(&d, pb::AmbitoTipo::AmbitoFaccion) {
                assert!(d.cambios.iter().all(|c| c.entidad != pb::TipoEntidad::Pueblo as i32), "a la Facción no le llega el almacén local por minuto");
            }
        }
        assert_eq!(de_plaza, 1, "una delta de la plaza por paso");
        // La plaza de este escenario tiene el almacén lleno y nada visible cambia en su paso (su máscara va vacía y no se manda): el pueblo, en cambio, dice de qué minuto es su almacén.
        assert_eq!(ana.pueblos[&pueblo_id].version_interior, version_inicial + i as u64 + 1, "el pueblo lleva la secuencia de su plaza en el paso {i}");
    }
    assert!(almacen(&ana) > inicial, "el pueblo extrae: {inicial} -> {}", almacen(&ana));
    let n = w.parar().await;
    let (_, real) = n.pueblo(pueblo).unwrap();
    assert_eq!(ana.pueblos[&pueblo_id], nucleo::proyeccion::propios::pueblo(&n, plaza, real), "foto + deltas = el pueblo del núcleo");
}

// ───────────────────────────── 3. Obras, mejoras y recintos en su instante ─────────────────────────────

#[tokio::test]
async fn una_obra_termina_en_su_instante_con_la_cola_y_la_mejora_y_el_recinto_viajan_con_su_mascara() {
    let m = con_mundo!();
    let x = aldea(m);
    let (plaza, as_) = (x.plaza, ids::asentamiento(x.plaza));
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    pausar(&mut admin_c).await;
    let (mut ana, _) = Dentro::entrar(&w, "ana", None).await;
    let (mut dora, _) = Dentro::entrar(&w, "dora", None).await;

    // La granja de la fundación ya se está mejorando sola (la plaza tiene recursos): la mejora viaja con su fin exacto. Y una vivienda a mano arranca al instante de la orden, con el suyo; la
    // delta de la plaza lleva solo los edificios (y lo que cambia con ellos).
    let granja = ana.plaza.edificios.iter().find(|e| e.tipo == "granja").expect("la granja de la fundación").clone();
    let mejora = granja.mejora.expect("la granja se mejora sola");
    assert_eq!(mejora.nivel_objetivo, granja.nivel_interno.unwrap() + 1);
    let (deltas, r) = ana.dar("o1", orden::Cuerpo::AnadirEdificioManualmente(pb::OrdenAnadirEdificioManualmente { asentamiento_id: as_.clone(), cargo: s("gobernador"), tipo: s("vivienda") })).await;
    aceptada(&r);
    let id = r.creadas[0].id.clone();
    assert!(!deltas.is_empty());
    let nueva = edificio(&ana.plaza, &id).clone();
    assert_eq!(nueva.estado, pb::EstadoEdificio::EdificioEnConstruccion as i32);
    let fin_vivienda = nueva.completa_en_ms.expect("una obra en marcha lleva su fin exacto");
    let campos: Vec<&String> = deltas.iter().flat_map(|d| d.cambios.iter().flat_map(|c| &c.campos)).collect();
    assert!(campos.iter().any(|c| *c == "edificios"));
    assert_eq!(dora.al_dia().await.len(), deltas.len(), "el otro de dentro recibe lo mismo");
    iguales(&ana.plaza, &dora.plaza, "la plaza de dora");

    // Cada fin llega en su milisegundo: un milisegundo antes sigue en curso; en su milisegundo está hecho, con el hecho en ese instante.
    let mut citas = [("vivienda", fin_vivienda), ("mejora", mejora.completa_en_ms)];
    citas.sort_by_key(|c| c.1);
    for (que, t) in citas {
        avanzar_hasta(&mut admin_c, &format!("{que}-antes"), t - 1).await;
        ana.al_dia().await;
        let en_curso = if que == "vivienda" { edificio(&ana.plaza, &id).completa_en_ms.is_some() } else { edificio(&ana.plaza, &granja.id).mejora.is_some() };
        assert!(en_curso, "un milisegundo antes de {que} sigue en curso");
        avanzar_hasta(&mut admin_c, &format!("{que}-fin"), t).await;
        ana.al_dia().await;
        if que == "vivienda" {
            assert_eq!(edificio(&ana.plaza, &id).estado, pb::EstadoEdificio::EdificioActivo as i32);
            assert!(edificio(&ana.plaza, &id).completa_en_ms.is_none());
                } else {
            let g = edificio(&ana.plaza, &granja.id);
            assert_eq!((g.mejora.is_none(), g.nivel_interno), (true, Some(mejora.nivel_objetivo)));
        }
    }

    // Un recinto: el anillo trazado entra con sus celdas y su avance; cada celda levantada es un cambio parcial de `recintos`.
    let (deltas, r) = ana.dar("o3", orden::Cuerpo::ComprometerRecinto(pb::OrdenComprometerRecinto { asentamiento_id: as_.clone(), cargo: s("gobernador"), nivel: s("1") })).await;
    aceptada(&r);
    assert!(deltas.iter().flat_map(|d| d.cambios.iter().flat_map(|c| &c.campos)).any(|c| c == "recintos"));
    let r1 = ana.plaza.recintos[0].clone();
    assert_eq!((r1.id.as_str(), r1.nivel), (format!("recinto-{plaza}-1").as_str(), 1));
    assert!(!r1.celdas.is_empty() && r1.avance < r1.celdas.len() as u32);
    let siguiente = r1.siguiente_celda_en_ms.expect("la primera celda tiene su cita");
    avanzar_hasta(&mut admin_c, "f3", siguiente).await;
    ana.al_dia().await;
    assert!(ana.plaza.recintos[0].avance > r1.avance, "la celda se levantó en su instante");

    dora.al_dia().await;
    let n = w.parar().await;
    assert_eq!((ana.plaza.clone(), dora.plaza), (proyeccion(&n, plaza), proyeccion(&n, plaza)));
}


// ───────────────────────────── 3b. La versión del mundo no depende de las sesiones ─────────────────────────────

/// Decisión del autor (2026-10-23): el paso de minuto lleva su propia secuencia por plaza y no sube la versión del mundo. Con el mismo registro de órdenes, el contador `version` y el hash son idénticos con
/// sesiones (el núcleo lleva pertenencias e interiores, como con el actor delante) y sin ellas, minuto a minuto, con héroes que entran y salen y obras.
#[test]
fn con_las_mismas_ordenes_la_version_y_el_hash_son_los_mismos_con_sesiones_y_sin_ellas() {
    let m = con_mundo!();
    let sin_tropa = || aldea_con(m, Opc { sin_tropa: true, ..Default::default() });
    let (mut sin, mut con) = (sin_tropa(), sin_tropa());
    con.n.seguir_pertenencias();
    assert_eq!((sin.n.version(), sin.n.hash_estado()), (con.n.version(), con.n.hash_estado()), "partida igual");
    let (plaza, ana) = (sin.plaza, sin.ana);
    for paso in 1..=12 {
        for x in [&mut sin, &mut con] {
            let t = x.n.ahora() + 20_000;
            x.n.avanzar_hasta(t);
            match paso {
                2 => drop(x.ok("admin", Comando::ColocarHeroe { heroe: ana, ubicacion: Ubicacion::Mercenarios(0) })),
                3 => drop(x.ok("admin", Comando::ColocarHeroe { heroe: ana, ubicacion: Ubicacion::Asentamiento(plaza as u32) })),
                5 => drop(x.ok("admin", Comando::DarRecursos { asentamiento: plaza, recurso: Trigo, cantidad: 50_000 })),
                7 => drop(x.ok("ana", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Vivienda })),
                _ => {}
            }
            x.n.avanzar_hasta(x.n.ahora() + MIN);
        }
        assert_eq!((sin.n.version(), sin.n.hash_estado()), (con.n.version(), con.n.hash_estado()), "tras el paso {paso}");
        assert_eq!(sin.n.version_interior(plaza), con.n.version_interior(plaza));
    }
    // Pasaron muchos minutos y la versión del mundo no cuenta los pasos: solo las órdenes y los sucesos.
    assert!(sin.n.version_interior(plaza).unwrap() >= 12 && sin.n.version() < 12 + 60, "versión {}", sin.n.version());
}

// ───────────────────────────── 4. Política, guarnición, visitantes y acceso ─────────────────────────────

#[tokio::test]
async fn la_politica_la_guarnicion_el_visitante_y_el_acceso_que_expulsa_viajan_a_quien_esta_dentro_y_a_nadie_mas() {
    let m = con_mundo!();
    let x = aldea(m);
    let (plaza, as_) = (x.plaza, ids::asentamiento(x.plaza));
    let (h_ana, h_dora, h_gus) = (ids::heroe(x.ana), ids::heroe(x.dora), ids::heroe(x.gus));
    let (ana_id, gus_id) = (x.ana, x.gus);
    let escuadra = ids::escuadra(x.n.heroe(ana_id).unwrap().heroe.escuadras[0].id);
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    pausar(&mut admin_c).await;
    let (mut ana, _) = Dentro::entrar(&w, "ana", None).await;
    let (mut gus, _, _) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;
    let (mut beto, _, _) = w.conectar(hola_con("beto", TipoCliente::ClienteUnity, &[])).await;

    // Una política: activa con su cargo, su inicio y su caducidad; caduca en su instante exacto.
    let (_, r) = ana.dar("p1", orden::Cuerpo::ActivarPolitica(pb::OrdenActivarPolitica { asentamiento_id: as_.clone(), cargo: s("gobernador"), politica_id: s("racionamiento") })).await;
    aceptada(&r);
    ana.al_dia().await;
    let pol = ana.plaza.politicas_activas[0].clone();
    assert_eq!((pol.politica_id.as_str(), pol.cargo.as_str()), ("racionamiento", "gobernador"));
    assert_eq!(pol.expira_en_ms - pol.activada_en_ms, 150 * MIN, "la duración de la política");
    avanzar_hasta(&mut admin_c, "p2a", pol.expira_en_ms - 1).await;
    ana.al_dia().await;
    assert_eq!(ana.plaza.politicas_activas.len(), 1, "un milisegundo antes sigue activa");
    avanzar_hasta(&mut admin_c, "p2", pol.expira_en_ms).await;
    ana.al_dia().await;
    assert!(ana.plaza.politicas_activas.is_empty(), "caducó en su instante");

    // El cupo de guarnición sale de los edificios: un Barracón que termina lo sube, en el instante en que termina.
    assert_eq!(ana.plaza.cupo_guarnicion, 0);
    let (_, r) = ana.dar("b1", orden::Cuerpo::AnadirEdificioManualmente(pb::OrdenAnadirEdificioManualmente { asentamiento_id: as_.clone(), cargo: s("gobernador"), tipo: s("barracon") })).await;
    aceptada(&r);
    let fin = edificio(&ana.plaza, &r.creadas[0].id).completa_en_ms.expect("el Barracón se está construyendo");
    avanzar_hasta(&mut admin_c, "b2", fin).await;
    ana.al_dia().await;
    assert!(ana.plaza.cupo_guarnicion > 0, "el Barracón da cupo de guarnición");

    // La guarnición: la escuadra de ana entra en ella y sale.
    let (_, r) = ana.dar("g1", orden::Cuerpo::AsignarGuarnicion(pb::OrdenAsignarGuarnicion { squad_id: escuadra.clone() })).await;
    aceptada(&r);
    ana.al_dia().await;
    assert_eq!(ana.plaza.guarnicion_ids, std::slice::from_ref(&escuadra));
    let (_, r) = ana.dar("g2", orden::Cuerpo::RetirarGuarnicion(pb::OrdenRetirarGuarnicion { squad_id: escuadra.clone() })).await;
    aceptada(&r);
    ana.al_dia().await;
    assert!(ana.plaza.guarnicion_ids.is_empty());

    // Un visitante de otra Facción entra (la administración lo coloca) y sale por su pie: ana ve quién está dentro; gus pierde su campamento y no recibe nada de la plaza.
    let (_, r) = dar(&mut admin_c, "v1", colocar(gus_id, &as_)).await;
    aceptada(&r);
    ana.al_dia().await;
    assert_eq!(ana.plaza.presentes_ids, [h_ana.clone(), h_dora, h_gus.clone()]);
    assert!(!ana.plaza.casas_compradas.contains(&h_gus), "visita, no reside");
    let (_, r) = dar(&mut gus, "v2", orden::Cuerpo::SalirDeAsentamiento(pb::OrdenSalirDeAsentamiento { asentamiento_id: as_.clone(), heroe_id: h_gus.clone() })).await;
    aceptada(&r);
    ana.al_dia().await;
    assert!(!ana.plaza.presentes_ids.contains(&h_gus), "se fue");

    // Entra otra vez, ahora por su pie (su columna está a la puerta), y ana cierra la puerta a los neutrales: sale a la fuerza, y la plaza lo cuenta (la puerta y quién queda dentro).
    let (_, r) = dar(&mut gus, "v3", orden::Cuerpo::EntrarEnAsentamiento(pb::OrdenEntrarEnAsentamiento { asentamiento_id: as_.clone(), heroe_id: h_gus.clone() })).await;
    aceptada(&r);
    ana.al_dia().await;
    assert!(ana.plaza.presentes_ids.contains(&h_gus));
    let (deltas, r) = ana.dar("a1", orden::Cuerpo::FijarPuerta(pb::OrdenFijarPuerta { asentamiento_id: as_.clone(), heroe_id: h_ana.clone(), cerrada_a: vec![s("neutrales"), s("enemigos")] })).await;
    aceptada(&r);
    assert_eq!(ana.plaza.puerta_cerrada_a, [pb::GrupoPuerta::GrupoNeutrales as i32, pb::GrupoPuerta::GrupoEnemigos as i32]);
    assert!(!ana.plaza.presentes_ids.contains(&h_gus), "la puerta cerrada lo expulsó");
    assert!(!deltas.is_empty());

    // Nada de esto llegó a quien no está dentro: gus (de otra Facción) y beto (de la Facción, pero en el campamento) no recibieron un solo mensaje del ámbito de la plaza; en la Facción solo viaja la ficha.
    for c in [&mut gus, &mut beto] {
        for m in pendiente(c).await {
            match m {
                ms::Cuerpo::Delta(d) => {
                    assert!(!es_de(&d, pb::AmbitoTipo::AmbitoAsentamiento), "una delta de la plaza llegó a quien no está dentro");
                    for cambio in &d.cambios {
                        if let Some(pb::entidad::Cuerpo::Asentamiento(a)) = cambio.datos.as_ref().and_then(|e| e.cuerpo.as_ref()) {
                            assert!(a.almacen.is_empty() && a.edificios.is_empty() && a.presentes_ids.is_empty() && a.trazado.is_none(), "solo la ficha, nunca el interior");
                        }
                    }
                }
                ms::Cuerpo::Foto(f) => assert!(!es_de_foto(&f, pb::AmbitoTipo::AmbitoAsentamiento)),
                _ => {}
            }
        }
    }
    let n = w.parar().await;
    iguales(&proyeccion(&n, plaza), &ana.plaza, "plaza");
}

fn es_de_foto(f: &pb::Foto, tipo: pb::AmbitoTipo) -> bool {
    f.ambito.as_ref().is_some_and(|a| a.tipo == tipo as i32)
}

// ───────────────────────────── 5. El campamento de mercenarios desde dentro ─────────────────────────────

/// El campamento de una `Foto` de `CAMPAMENTO`.
fn campamento_de_foto(f: &pb::Foto) -> pb::CampamentoMercenarios {
    let v: Vec<_> = f.entidades.iter().filter_map(|e| if let Some(pb::entidad::Cuerpo::CampamentoMercenarios(c)) = &e.cuerpo { Some(c.clone()) } else { None }).collect();
    assert_eq!(v.len(), 1, "un solo campamento en la foto de su ámbito");
    v.into_iter().next().unwrap()
}

/// Los mensajes de `CAMPAMENTO` entre los que llegaron: el campamento completo que traen (el último, si hay varios).
fn campamento_de(mensajes: &[ms::Cuerpo]) -> Option<pb::CampamentoMercenarios> {
    mensajes.iter().rev().find_map(|m| match m {
        ms::Cuerpo::Foto(f) if es_de_foto(f, pb::AmbitoTipo::AmbitoCampamento) => Some(campamento_de_foto(f)),
        ms::Cuerpo::Delta(d) if es_de(d, pb::AmbitoTipo::AmbitoCampamento) => d.cambios.iter().rev().find_map(|c| match c.datos.as_ref()?.cuerpo.as_ref()? {
            pb::entidad::Cuerpo::CampamentoMercenarios(x) => {
                assert!(c.reemplaza, "el campamento viaja entero");
                Some(x.clone())
            }
            _ => None,
        }),
        _ => None,
    })
}

fn deltas_de(previos: Vec<ms::Cuerpo>) -> Vec<ms::Cuerpo> {
    previos.into_iter().filter(|m| matches!(m, ms::Cuerpo::Delta(_))).collect()
}

#[tokio::test]
async fn el_campamento_desde_dentro_trae_reclutas_stock_y_gente_y_una_compra_una_leva_y_un_prestamo_lo_cambian() {
    let m = con_mundo!();
    let x = aldea(m);
    let (camp, h_beto, h_gus) = (ids::campamento(0), ids::heroe(x.beto), ids::heroe(x.gus));
    let w = arrancar_con(x.n).await;
    let (mut beto, _, inicial) = w.conectar(hola_con("beto", TipoCliente::ClienteUnity, &[])).await;
    let (mut gus, _, de_gus) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;
    let (mut ana, _, _) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let mut c = campamento_de(&inicial).expect("la foto del campamento");

    // La foto: reclutas como un ritmo hasta el tope de sus viviendas, el stock en venta, los contadores del mercado, quién vive y quién está dentro, lo que ofrece y el cupo diario.
    assert_eq!((c.id.as_str(), c.origen < 10), (camp.as_str(), true));
    let reclutas = c.poblacion.unwrap();
    assert!(reclutas.valor_en_t0 > 0 && reclutas.maximo == Some(reclutas.valor_en_t0) && reclutas.por_minuto > 0 && !reclutas.en_escalones, "nace lleno y se recupera como un ritmo: {reclutas:?}");
    assert!(c.mercado.as_ref().unwrap().por_id["madera"] > 0 && c.mercado.as_ref().unwrap().por_id.len() > 3);
    assert!(c.residentes_ids.contains(&h_beto) && c.residentes_ids.contains(&h_gus));
    assert_eq!(c.presentes_ids, [h_beto.clone(), h_gus.clone()]);
    assert!(c.edificios.len() > 1 && c.prestamos_ids.is_empty(), "{:?} {:?}", c.edificios, c.prestamos_ids);
    assert!(!c.tropas_ofrecidas.is_empty(), "con las tecnologías adoptadas, el campamento ofrece tropa");
    assert_eq!(c.cupo_diario.as_ref().unwrap().por_id["madera"], 60_000);
    assert_eq!(campamento_de(&de_gus), Some(c.clone()), "el campamento no depende de quien lo mira");

    // Una compra de beto: el stock baja en lo comprado, para beto y para el visitante que está dentro; el cupo y el almacén personal van en su héroe.
    let antes = c.mercado.as_ref().unwrap().por_id["madera"];
    let (previos, r) = dar_todo(&mut beto, "c1", orden::Cuerpo::ComprarEnCampamento(pb::OrdenComprarEnCampamento { recurso: s("madera"), cantidad: 10_000, campamento_id: None })).await;
    aceptada(&r);
    c = campamento_de(&previos).expect("la delta del campamento antes del resultado");
    assert_eq!(c.mercado.as_ref().unwrap().por_id["madera"], antes - 10_000);
    let heroe = previos
        .iter()
        .find_map(|m| match m {
            ms::Cuerpo::Delta(d) if es_de(d, pb::AmbitoTipo::AmbitoHeroe) => d.cambios.iter().find_map(|c| if let Some(pb::entidad::Cuerpo::Heroe(h)) = c.datos.as_ref()?.cuerpo.as_ref() { Some(h.clone()) } else { None }),
            _ => None,
        })
        .expect("su héroe cambió");
    assert_eq!(heroe.cupo_campamento.unwrap().comprado["madera"], 10_000, "el cupo diario lo lleva su héroe");
    assert_eq!(campamento_de(&deltas_de(pendiente(&mut gus).await)), Some(c.clone()), "el otro de dentro lo ve igual");

    // Una leva: gasta reclutas del campamento (el ritmo vuelve a empezar de un valor menor).
    let tropa = c.tropas_ofrecidas[0].clone();
    let (previos, r) = dar_todo(&mut beto, "c2", orden::Cuerpo::ReclutarEnCampamento(pb::OrdenReclutarEnCampamento { tropa_id: tropa, pagar_con: Some(s("almacenPersonal")) })).await;
    aceptada(&r);
    c = campamento_de(&previos).expect("la delta del campamento");
    let ahora = c.poblacion.unwrap();
    assert!(ahora.valor_en_t0 < reclutas.valor_en_t0 && ahora.t0_ms >= reclutas.t0_ms, "{ahora:?}");

    // Un préstamo: la escuadra prestada figura en el campamento.
    let (previos, r) = dar_todo(&mut beto, "c3", orden::Cuerpo::PedirPrestamo(pb::OrdenPedirPrestamo { tropa_ids: vec![s("milicia_lanceros")] })).await;
    aceptada(&r);
    let prestada = r.creadas[0].id.clone();
    c = campamento_de(&previos).expect("la delta del campamento");
    assert_eq!(c.prestamos_ids, [prestada]);

    // Nada de esto llegó a ana, que está en su plaza: ni un mensaje del ámbito del campamento.
    for m in pendiente(&mut ana).await {
        if let ms::Cuerpo::Delta(d) = m {
            assert!(!es_de(&d, pb::AmbitoTipo::AmbitoCampamento));
        }
    }
    let n = w.parar().await;
    assert_eq!(interior::campamento(&n, 0), Some(c));
}

// ───────────────────────────── 6. Reanudación y «foto + deltas = foto final» ─────────────────────────────

#[tokio::test]
async fn al_reconectar_la_plaza_y_el_campamento_se_reanudan_por_deltas_aunque_el_heroe_saliera_y_foto_mas_deltas_es_la_foto_final() {
    let m = con_mundo!();
    let x = aldea(m);
    let (plaza, as_, camp) = (x.plaza, ids::asentamiento(x.plaza), ids::campamento(0));
    let (ana_id, paso) = (x.ana, proximo_paso(&x.n, x.plaza));
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    pausar(&mut admin_c).await;
    let (ana, _) = Dentro::entrar(&w, "ana", None).await;
    let (mut dora, _) = Dentro::entrar(&w, "dora", None).await;
    let (beto, _, inicial) = w.conectar(hola_con("beto", TipoCliente::ClienteUnity, &[])).await;
    let c0 = campamento_de(&inicial).unwrap();
    let v_camp = inicial.iter().find_map(|m| if let ms::Cuerpo::Foto(f) = m { f.ambito.as_ref().filter(|a| a.tipo == pb::AmbitoTipo::AmbitoCampamento as i32).map(|_| f.version) } else { None }).unwrap();
    let decision = |b: &pb::Bienvenida, a: &str| b.ambitos.iter().find(|x| ambito(&x.ambito) == a).map(|x| x.decision).expect("el ámbito está en la bienvenida");
    let k_plaza = format!("ASENTAMIENTO:{as_}");

    // Nada cambió: la plaza se reanuda sin novedad y sin un solo mensaje de ella.
    let recuerdo = ana.cerrar().await;
    let (mut ana, b) = Dentro::entrar(&w, "ana", Some(&recuerdo)).await;
    assert_eq!(decision(&b, &k_plaza), D::ReanudaSinNovedad as i32);
    assert!(pendiente(&mut ana.c).await.iter().all(|m| !matches!(m, ms::Cuerpo::Delta(d) if es_de(d, pb::AmbitoTipo::AmbitoAsentamiento))));

    // Mientras ana no está: pasa un minuto y dora encarga una vivienda. Al volver, una sola delta de la versión que tenía a hoy.
    let conocida = ana.version;
    let recuerdo = ana.cerrar().await;
    avanzar_hasta(&mut admin_c, "r1", paso).await;
    let (_, r) = dora.dar("r2", orden::Cuerpo::AnadirEdificioManualmente(pb::OrdenAnadirEdificioManualmente { asentamiento_id: as_.clone(), cargo: s("gobernador"), tipo: s("vivienda") })).await;
    aceptada(&r);
    dora.al_dia().await;
    let (mut ana, b) = Dentro::entrar(&w, "ana", Some(&recuerdo)).await;
    assert_eq!(decision(&b, &k_plaza), D::ReanudaConDeltas as i32);
    assert!(ana.version > conocida);
    iguales(&dora.plaza, &ana.plaza, "plaza");
    assert!(pendiente(&mut ana.c).await.iter().all(|m| !matches!(m, ms::Cuerpo::Delta(d) if es_de(d, pb::AmbitoTipo::AmbitoAsentamiento))), "y no queda nada pendiente");

    // Aunque el héroe salga de la plaza y vuelva mientras tanto, se reanuda por deltas (el registro lo dice todo): ya no se pide foto.
    let recuerdo = ana.cerrar().await;
    let (_, r) = dar(&mut admin_c, "r3", colocar(ana_id, &camp)).await;
    aceptada(&r);
    let (_, r) = dar(&mut admin_c, "r4", colocar(ana_id, &as_)).await;
    aceptada(&r);
    avanzar_hasta(&mut admin_c, "r5", paso + MIN).await;
    dora.al_dia().await;
    let (ana, b) = Dentro::entrar(&w, "ana", Some(&recuerdo)).await;
    assert_eq!(decision(&b, &k_plaza), D::ReanudaConDeltas as i32);
    iguales(&dora.plaza, &ana.plaza, "plaza");

    // Y una foto nueva (una conexión que no sabe nada) es la misma plaza que lo acumulado a base de deltas.
    dora.cerrar().await;
    let (nueva, b) = Dentro::entrar(&w, "dora", None).await;
    assert_eq!(decision(&b, &k_plaza), D::ReanudaConFoto as i32);
    assert_eq!(nueva.plaza, ana.plaza);

    // El campamento: lo que cambia mientras beto no está llega al volver como una delta del campamento entero.
    beto.cerrar().await;
    let (mut gus, _, _) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;
    let (_, r) = dar(&mut gus, "c1", orden::Cuerpo::ComprarEnCampamento(pb::OrdenComprarEnCampamento { recurso: s("madera"), cantidad: 10_000, campamento_id: None })).await;
    aceptada(&r);
    let k_camp = format!("CAMPAMENTO:{camp}");
    let (_, b, mensajes) = w.conectar(hola_con("beto", TipoCliente::ClienteUnity, &[(&k_camp, v_camp)])).await;
    assert_eq!(decision(&b, &k_camp), D::ReanudaConDeltas as i32);
    let c1 = campamento_de(&mensajes).expect("la delta del campamento");
    assert_eq!(c1.mercado.as_ref().unwrap().por_id["madera"], c0.mercado.as_ref().unwrap().por_id["madera"] - 10_000);

    let n = w.parar().await;
    assert_eq!(nueva.plaza, proyeccion(&n, plaza));
}


// ───────────────────────────── 6b. Recuperar el registro con una sesión conectada ─────────────────────────────

/// El problema que cerró la secuencia propia del interior (duda 1 de T3c): con sesiones el paso de minuto cerraba una versión por plaza y minuto y la repetición del registro sin sesiones acababa en una
/// versión menor, de modo que un cliente que conservaba la de antes se veía «del futuro» y recibía una foto. Ahora el núcleo recuperado tiene la misma versión, el mismo hash y la misma secuencia interior que
/// el que se apagó, y el cliente reanuda `ASENTAMIENTO`: sin novedad si está al día, con la plaza entera si su secuencia es más vieja y con foto solo si es mayor que la de hoy.
#[tokio::test]
async fn al_recuperar_el_registro_la_sesion_reanuda_la_plaza_sin_verse_del_futuro() {
    let m = con_mundo!();
    let dir = tempfile::tempdir().unwrap();
    let x = aldea_con(m, Opc { dir: Some(dir.path()), sin_tropa: true, ..Default::default() });
    let (plaza, as_) = (x.plaza, ids::asentamiento(x.plaza));
    let paso = proximo_paso(&x.n, plaza);
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    pausar(&mut admin_c).await;
    let (mut ana, _) = Dentro::entrar(&w, "ana", None).await;
    let (mut dora, _) = Dentro::entrar(&w, "dora", None).await;

    // Pasan minutos (solo interior), después dora encarga una vivienda (sube la versión del mundo) y pasan dos minutos más: ana queda al día de las dos secuencias.
    avanzar_hasta(&mut admin_c, "r1", paso + 2 * MIN).await;
    let (_, r) = dora.dar("r2", orden::Cuerpo::AnadirEdificioManualmente(pb::OrdenAnadirEdificioManualmente { asentamiento_id: as_.clone(), cargo: s("gobernador"), tipo: s("vivienda") })).await;
    aceptada(&r);
    avanzar_hasta(&mut admin_c, "r3", paso + 4 * MIN).await;
    ana.al_dia().await;
    dora.cerrar().await;
    let (k, ki) = (ana.version, ana.plaza.version_interior);
    let recuerdo = ana.cerrar().await;
    let n = w.parar().await;
    let vivo = (n.version(), n.hash_estado(), n.version_interior(plaza).unwrap());
    assert!(k <= vivo.0 && ki == vivo.2, "ana tenía la secuencia de la plaza al día ({ki}) y una versión del mundo no posterior ({k} <= {})", vivo.0);
    assert!(ki >= 5, "pasaron al menos cinco pasos de minuto: {ki}");
    drop(n);

    // Se recupera el registro (foto + entradas, sin sesiones): es el mismo mundo, con la misma versión, el mismo hash y la misma secuencia interior.
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let mut reglas = Reglas::default();
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let arranque = Arranque { reglas, ajustes, semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) };
    let recuperado = Nucleo::con_persistencia(arranque, dir.path(), Default::default()).expect("se recupera");
    assert!(recuperado.recuperado());
    assert_eq!((recuperado.version(), recuperado.hash_estado(), recuperado.version_interior(plaza).unwrap()), vivo, "la versión, el hash y la secuencia interior no dependen de si había sesiones");
    let w = arrancar_con(recuperado).await;
    // El reloj sigue pausado (el estado de pausa viaja con el registro): nada cae entre medias.
    let decision = |b: &pb::Bienvenida| b.ambitos.iter().find(|x| ambito(&x.ambito) == format!("ASENTAMIENTO:{as_}")).map(|x| (x.decision, x.version_interior_actual)).expect("el ámbito está en la bienvenida");

    // Al día: sin novedad y sin un mensaje de la plaza (ni foto ni delta).
    let (mut ana, b) = Dentro::entrar(&w, "ana", Some(&recuerdo)).await;
    assert_eq!(decision(&b), (D::ReanudaSinNovedad as i32, ki));
    assert!(pendiente(&mut ana.c).await.iter().all(|m| !matches!(m, ms::Cuerpo::Foto(_)) && !matches!(m, ms::Cuerpo::Delta(d) if es_de(d, pb::AmbitoTipo::AmbitoAsentamiento))));
    ana.cerrar().await;

    // Con una secuencia interior más vieja (se perdió algún paso de minuto): por deltas, con la plaza entera, que es la de hoy.
    let vieja = Recuerdo { plaza: pb::Asentamiento { version_interior: ki - 3, ..recuerdo.plaza.clone() }, pueblos: recuerdo.pueblos.clone(), version: k };
    let (ana, b) = Dentro::entrar(&w, "ana", Some(&vieja)).await;
    assert_eq!(decision(&b), (D::ReanudaConDeltas as i32, ki));
    assert_eq!(ana.plaza.version_interior, ki);
    let entera = ana.plaza.clone();
    ana.cerrar().await;

    // Con una mayor que la de hoy sí sería «del futuro»: foto.
    let futura = Recuerdo { plaza: pb::Asentamiento { version_interior: ki + 1, ..recuerdo.plaza.clone() }, pueblos: recuerdo.pueblos.clone(), version: k };
    let (ana, b) = Dentro::entrar(&w, "ana", Some(&futura)).await;
    assert_eq!(decision(&b), (D::ReanudaConFoto as i32, ki));
    assert_eq!(ana.plaza.version_interior, ki);
    ana.cerrar().await;
    let n = w.parar().await;
    iguales(&proyeccion(&n, plaza), &entera, "la plaza entera de la reanudación por deltas");
}

// ───────────────────────────── 7. Coste de proyectar ─────────────────────────────

/// Lo que cuesta rehacer la plaza vista desde dentro y la máscara contra la anterior: es lo que paga el actor por cada plaza abierta y cada versión que la toca (con sesiones, una por minuto).
/// `cargo test --release -p nucleo --test interior coste -- --ignored --nocapture`.
#[test]
#[ignore = "mide; en release"]
fn coste_de_proyectar_la_plaza_y_su_mascara() {
    let m = con_mundo!();
    let x = aldea(m);
    let (n, plaza) = (x.n, x.plaza);
    const VECES: u32 = 2_000;
    let mut antes = proyeccion(&n, plaza);
    let (inicio, mut campos) = (std::time::Instant::now(), 0);
    for _ in 0..VECES {
        let ahora = proyeccion(&n, plaza);
        campos += interior::mascara(&antes, &ahora).map_or(0, |(c, _)| c.len());
        antes = ahora;
    }
    let por_vez = inicio.elapsed().as_secs_f64() * 1e6 / f64::from(VECES);
    println!("proyectar la plaza ({} edificios, {} calles) y comparar con la anterior: {por_vez:.1} µs por versión", antes.edificios.len(), antes.trazado.as_ref().map_or(0, |t| t.calles.len()));
    assert_eq!(campos, 0, "el mismo estado no cambia nada");
}
