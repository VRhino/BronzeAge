//! Los ámbitos propios, `HEROE` y `FACCION`, por el servidor WebSocket real y el actor del núcleo (4H, T3b): lo que lleva la foto inicial de cada uno, el delta de cada cosa que cambia
//! (reclutar, un loadout, una columna, una escolta, una caravana que se aparca, un trueque, una orden de mercado, una relación), el `QUITA` de lo que desaparece o deja de ser suyo, que
//! nada de otra Facción se filtre y la reanudación por versión.
//!
//! Cada prueba prepara el mundo directamente en el núcleo (dos plazas con Mercado, F1 y F2, y cuatro héroes: `ana` y `dora` de F1 dentro de su plaza, `beto` de F1 en el campamento y `gus`
//! de F2 dentro de la suya), lo pone detrás del servidor y da las órdenes por el contrato. Lo que hace el reloj de mundo por su cuenta llega a las sesiones como una `Delta` suelta: el
//! escenario deja el mundo recién entrado en un minuto, con las plazas sin construcción automática y las columnas a más de 30 u, y donde se cuenta lo que llegó se cuenta tras una orden que
//! el actor rechaza (`pendiente`), que sale después de todo lo que ya iba hacia la sesión. Van sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`) y se omiten si no está.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::Recurso::{self, *};
use base::{Instante, Punto, MS_POR_MINUTO};
use comercio::caravana::{Animal, TipoCarro};
use comun::sesion::*;
use contrato::v1::{self as pb, entidad::Cuerpo, mensaje_de_servidor as ms, orden, DecisionReanudacion as D, Entidad, TipoCliente, TipoEntidad};
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::proyeccion::ambito::Ambito;
use nucleo::proyeccion::ids;
use nucleo::{Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Resultado};
use prost_types::{value::Kind, Struct, Value};
use rutas::Rutas;
use servidor::prueba::{hola, ClientePrueba};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

const HORA: Instante = 60 * MS_POR_MINUTO;

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

// ───────────────────────────── El mundo antes de ponerlo detrás del servidor ─────────────────────────────

fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

/// Un sitio verde con mucho bosque a 10 u o más del campamento `i`: una plaza que no cae en ruinas por falta de madera.
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

struct Esc {
    n: Nucleo,
    /// La plaza de F1 y la de F2.
    a: u64,
    b: u64,
    /// `ana` (F1, reside y está dentro de A), `beto` (F1, en el campamento 0) y `gus` (F2, reside y está dentro de B).
    ana: u64,
    beto: u64,
    gus: u64,
    /// `dora` (F1, reside y está dentro de A).
    dora: u64,
    /// Los héroes de campo que fundaron cada plaza (cuentas `fundador_a` y `fundador_b`).
    fundadores: [u64; 2],
    peticiones: u64,
}

impl Esc {
    fn ord(&mut self, actor: &str, c: Comando) -> nucleo::Respuesta {
        self.peticiones += 1;
        let p = Peticion::nueva(actor, &format!("prep{}", self.peticiones), c);
        let t = self.n.ahora();
        self.n.aplicar_en(t, if actor == "admin" { p.de_admin() } else { p })
    }

    fn ok(&mut self, actor: &str, c: Comando) -> nucleo::Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
        r
    }

    fn dar(&mut self, plaza: u64, recurso: Recurso, cantidad: i64) {
        self.ok("admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad });
    }

    /// Un héroe de la cuenta, ciudadano de `faccion`, en el campamento 0; con `plaza`, reside en ella y está dentro.
    fn heroe(&mut self, cuenta: &str, faccion: u32, plaza: Option<u64>) -> u64 {
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano }).creadas[0];
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(faccion) });
        if let Some(p) = plaza {
            self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(p as u32)) });
            self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(p as u32) });
        }
        h
    }

    /// Deja el mundo recién entrado en un minuto: el paso de minuto de las plazas queda a 59 s de reloj de pared, y lo que el mundo hace por su cuenta antes (el encuentro de dos columnas que se
    /// ven, un hecho debido en el instante en que se crean) ya ocurrió cuando las sesiones empiezan.
    fn alinear(&mut self) {
        let ahora = self.n.ahora();
        self.n.avanzar_hasta((ahora / MS_POR_MINUTO + 1) * MS_POR_MINUTO + 1_000);
    }

    /// Una caravana de la plaza con un carro con buey, fuera del reparto automático.
    fn caravana(&mut self, actor: &str, plaza: u64) -> u64 {
        let id = self.ok(actor, Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
        self.ok(actor, Comando::ReservarCaravana { caravana: id, reservada: true });
        self.ok(actor, Comando::AnadirCarro { caravana: id, tipo: TipoCarro::Basico });
        self.ok(actor, Comando::AnadirAnimal { caravana: id, carro: 0, animal: Animal::Buey });
        id
    }
}

/// Dos plazas con Mercado (F1 y F2) y cuatro héroes, con el mundo recién entrado en un minuto.
fn escenario(m: &M) -> Esc {
    escenario_con(m, false)
}

/// El mismo mundo; con `unity`, las batallas van a Unity (`Reglas::batallas_en_unity`).
fn escenario_con(m: &M, unity: bool) -> Esc {
    let mut reglas = Reglas { batallas_en_unity: unity, ..Reglas::default() };
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let n = Nucleo::nuevo(Arranque { reglas, ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
    let mut x = Esc { n, a: 0, b: 0, ana: 0, beto: 0, gus: 0, dora: 0, fundadores: [0; 2], peticiones: 0 };
    let (sa, sb) = (sitio_rico(&x.n, &m.0, 0, 1), sitio_rico(&x.n, &m.0, 3, 2));
    for (cuenta, faccion, sitio) in [("fundador_a", 1, sa), ("fundador_b", 2, sb)] {
        let t = x.n.ahora();
        let columna = x.n.crear_ejercito(t, cuenta, faccion, sitio, 500, 100_000).unwrap();
        x.ok("admin", Comando::DarCaravanaFundacion { ejercito: columna });
        let plaza = x.ok(cuenta, Comando::Fundar { ejercito: columna }).creadas[0];
        let fundador = x.n.ejercito(columna).map_or(0, |e| e.columna.lider);
        if faccion == 1 {
            (x.a, x.fundadores[0]) = (plaza, fundador);
        } else {
            (x.b, x.fundadores[1]) = (plaza, fundador);
        }
    }
    for plaza in [x.a, x.b] {
        for r in [Madera, Piedra, Oro, Trigo] {
            x.dar(plaza, r, 350_000);
        }
        x.ok("admin", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
    }
    let t = x.n.ahora() + 9 * HORA;
    x.n.avanzar_hasta(t);
    for plaza in [x.a, x.b] {
        x.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
    }
    let (a, b) = (x.a, x.b);
    x.ana = x.heroe("ana", 1, Some(a));
    x.beto = x.heroe("beto", 1, None);
    x.gus = x.heroe("gus", 2, Some(b));
    x.dora = x.heroe("dora", 1, Some(a));
    x.alinear();
    x
}

// ───────────────────────────── Lectura de lo que llega ─────────────────────────────

fn ambito(a: &Option<pb::Ambito>) -> String {
    let a = a.as_ref().expect("un ámbito");
    format!("{}:{}", pb::AmbitoTipo::try_from(a.tipo).unwrap().as_str_name().trim_start_matches("AMBITO_"), a.id)
}

fn etiqueta(m: &ms::Cuerpo) -> String {
    match m {
        ms::Cuerpo::Foto(f) => format!("foto {}", ambito(&f.ambito)),
        ms::Cuerpo::Delta(d) => format!("delta {}", ambito(&d.ambito)),
        ms::Cuerpo::Revocado(r) => format!("revocado {}", ambito(&r.ambito)),
        ms::Cuerpo::Aviso(a) => format!("aviso {}", a.codigo),
        otro => format!("{otro:?}"),
    }
}

fn etiquetas(ms: &[ms::Cuerpo]) -> Vec<String> {
    ms.iter().map(etiqueta).collect()
}

/// Lo que ya estaba en camino hacia la sesión: el actor contesta a una orden que rechaza después de repartir todo lo anterior, y el servidor entrega en orden.
async fn pendiente(c: &mut ClientePrueba) -> Vec<ms::Cuerpo> {
    let (previos, r) = dar_todo(c, "barrera", orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: "ej-999999".into() })).await;
    assert_eq!(rechazo(&r), "ejercito.desconocido");
    previos
}

/// Las `Delta` de un ámbito (`HEROE:heroe-3`, `FACCION:f-1`...) entre lo que llegó.
fn deltas_de<'a>(mensajes: &'a [ms::Cuerpo], ambito_buscado: &str) -> Vec<&'a pb::Delta> {
    mensajes.iter().filter_map(|m| if let ms::Cuerpo::Delta(d) = m { Some(d) } else { None }).filter(|d| ambito(&d.ambito) == ambito_buscado).collect()
}

fn foto_de<'a>(mensajes: &'a [ms::Cuerpo], ambito_buscado: &str) -> &'a pb::Foto {
    mensajes.iter().find_map(|m| if let ms::Cuerpo::Foto(f) = m { Some(f).filter(|f| ambito(&f.ambito) == ambito_buscado) } else { None }).unwrap_or_else(|| panic!("no hay foto de {ambito_buscado}: {:?}", etiquetas(mensajes)))
}

/// Las entidades que una `Delta` actualiza.
fn nuevas(d: &pb::Delta) -> Vec<&Entidad> {
    d.cambios.iter().filter(|c| c.tipo == pb::TipoCambio::CambioActualiza as i32).filter_map(|c| c.datos.as_ref()).collect()
}

/// Los `QUITA` de una `Delta`: `(clase, id)`.
fn quitas(d: &pb::Delta) -> Vec<(TipoEntidad, String)> {
    d.cambios.iter().filter(|c| c.tipo == pb::TipoCambio::CambioQuita as i32).map(|c| (TipoEntidad::try_from(c.entidad).unwrap(), c.id.clone())).collect()
}

macro_rules! de_tipo {
    ($nombre:ident, $variante:ident, $tipo:ty) => {
        fn $nombre<'a>(es: impl IntoIterator<Item = &'a Entidad>) -> Vec<&'a $tipo> {
            es.into_iter().filter_map(|e| if let Some(Cuerpo::$variante(x)) = e.cuerpo.as_ref() { Some(x) } else { None }).collect()
        }
    };
}
de_tipo!(heroes, Heroe, pb::Heroe);
de_tipo!(escuadrones, Escuadron, pb::Escuadron);
de_tipo!(ejercitos, Ejercito, pb::Ejercito);
de_tipo!(caravanas, Caravana, pb::Caravana);
de_tipo!(asentamientos, Asentamiento, pb::Asentamiento);
de_tipo!(facciones, Faccion, pb::Faccion);
de_tipo!(tecnologias, Tecnologias, pb::TecnologiasFaccion);
de_tipo!(relaciones, Relacion, pb::RelacionPolitica);
de_tipo!(trueques, AcuerdoTrueque, pb::AcuerdoTrueque);
de_tipo!(ordenes, OrdenMercado, pb::OrdenMercado);
de_tipo!(pueblos, Pueblo, pb::Pueblo);
de_tipo!(obras, Obra, pb::ObraDeMapa);
de_tipo!(avanzadas, Avanzada, pb::Avanzada);
de_tipo!(convocatorias, Convocatoria, pb::Convocatoria);
de_tipo!(batallas, Batalla, pb::Batalla);

/// Lo que el núcleo fue apuntando por cada versión de a qué ámbitos pertenece cada entidad es lo mismo que sale de calcularlo de cero con el estado final.
fn pertenencias_al_dia(n: &mut Nucleo, cuentas: &[&str], facciones: &[u32]) {
    let ambitos: Vec<Ambito> = cuentas.iter().map(|c| Ambito::Heroe(n.heroe_de_cuenta(c).unwrap().heroe.id)).chain(facciones.iter().map(|&f| Ambito::Faccion(f))).collect();
    let antes: Vec<_> = ambitos.iter().map(|&a| n.entidades_de(a)).collect();
    assert!(antes.iter().any(|v| !v.is_empty()));
    n.seguir_pertenencias();
    let despues: Vec<_> = ambitos.iter().map(|&a| n.entidades_de(a)).collect();
    assert_eq!(antes, despues, "las pertenencias que se fueron apuntando coinciden con las de cero");
}

fn ids_de<T>(v: &[&T], id: fn(&T) -> &str) -> Vec<String> {
    v.iter().map(|x| id(x).to_owned()).collect()
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

fn aceptada(r: &pb::ResultadoOrden) {
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
}

/// El estado de un ámbito tal como lo reconstruye un cliente: la foto y, encima, cada delta en su orden.
#[derive(Default)]
struct Cliente {
    entidades: BTreeMap<(i32, String), Entidad>,
}

impl Cliente {
    fn desde(f: &pb::Foto) -> Self {
        let mut c = Cliente::default();
        for e in &f.entidades {
            c.poner(e);
        }
        c
    }

    fn poner(&mut self, e: &Entidad) {
        let (tipo, id) = match e.cuerpo.as_ref().expect("un cuerpo") {
            Cuerpo::Heroe(x) => (TipoEntidad::Heroe, x.id.clone()),
            Cuerpo::Escuadron(x) => (TipoEntidad::Escuadron, x.id.clone()),
            Cuerpo::Ejercito(x) => (TipoEntidad::Ejercito, x.id.clone()),
            Cuerpo::Caravana(x) => (TipoEntidad::Caravana, x.id.clone()),
            Cuerpo::Asentamiento(x) => (TipoEntidad::Asentamiento, x.id.clone()),
            Cuerpo::Faccion(x) => (TipoEntidad::Faccion, x.id.clone()),
            Cuerpo::Tecnologias(x) => (TipoEntidad::TecnologiaFaccion, x.faccion_id.clone()),
            Cuerpo::Relacion(x) => (TipoEntidad::RelacionPolitica, x.id.clone()),
            Cuerpo::AcuerdoTrueque(x) => (TipoEntidad::AcuerdoTrueque, x.id.clone()),
            Cuerpo::OrdenMercado(x) => (TipoEntidad::OrdenMercado, x.id.clone()),
            Cuerpo::Pueblo(x) => (TipoEntidad::Pueblo, x.id.clone()),
            Cuerpo::Obra(x) => (TipoEntidad::ObraDeMapa, x.id.clone()),
            Cuerpo::Avanzada(x) => (TipoEntidad::Avanzada, x.id.clone()),
            Cuerpo::Convocatoria(x) => (TipoEntidad::Convocatoria, x.id.clone()),
            Cuerpo::Batalla(x) => (TipoEntidad::Batalla, x.id.clone()),
            otro => panic!("una entidad que no viaja a HEROE ni a FACCION: {otro:?}"),
        };
        self.entidades.insert((tipo as i32, id), e.clone());
    }

    fn aplicar(&mut self, d: &pb::Delta) {
        for c in &d.cambios {
            if c.tipo == pb::TipoCambio::CambioQuita as i32 {
                self.entidades.remove(&(c.entidad, c.id.clone()));
            } else {
                assert!(c.reemplaza, "las entidades de estos ámbitos viajan completas");
                self.poner(c.datos.as_ref().expect("un ACTUALIZA trae la entidad"));
            }
        }
    }

    fn tiene(&self, tipo: TipoEntidad, id: &str) -> bool {
        self.entidades.contains_key(&(tipo as i32, id.to_owned()))
    }

    /// En qué se diferencia de otro estado (vacío si son iguales).
    fn diferencias(&self, otro: &Cliente) -> String {
        let claves: std::collections::BTreeSet<_> = self.entidades.keys().chain(otro.entidades.keys()).collect();
        claves.into_iter().filter(|k| self.entidades.get(*k) != otro.entidades.get(*k)).map(|k| format!("{}:{} [{:?} frente a {:?}]", k.0, k.1, self.entidades.get(k), otro.entidades.get(k))).collect::<Vec<_>>().join("
")
    }
}

// ───────────────────────────── 1. La foto inicial ─────────────────────────────

#[tokio::test]
async fn la_foto_inicial_de_heroe_y_faccion_lleva_todo_lo_propio_y_nada_de_otra_faccion() {
    let m = con_mundo!();
    let mut x = escenario(m);
    let (a, b) = (x.a, x.b);
    let (as_a, as_b, h_ana) = (ids::asentamiento(a), ids::asentamiento(b), ids::heroe(x.ana));
    // Lo que tiene F1: una escuadra de ana, una orden de mercado y una caravana de A, un trueque con B, tecnologías y una relación con F2.
    let escuadra = x.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: x.ana, tropa: Tropa::MiliciaLanceros }).creadas[0];
    x.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Madera, cantidad: 10_000, precio: None });
    let caravana = x.caravana("ana", a);
    x.ok("ana", Comando::ProponerTrueque { a, b, lineas_a: vec![(Madera, 10_000)], lineas_b: vec![(Piedra, 5_000)] });
    x.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["arco_compuesto".into()] });
    x.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(nucleo::Relacion::Alianza) });
    let cupo = x.n.cupo_de_guarnicion(a).unwrap();
    let w = arrancar_con(x.n).await;

    // ana, dentro de su plaza: su héroe con lo suyo y su Facción con todo lo de F1.
    let (_, b_ana, mensajes) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    assert_eq!(etiquetas(&mensajes), [format!("foto HEROE:{h_ana}"), "foto FACCION:f-1".into(), format!("foto ASENTAMIENTO:{as_a}")]);
    assert_eq!((b_ana.heroe_id.as_deref(), b_ana.faccion_id.as_deref()), (Some(h_ana.as_str()), Some("f-1")));

    let propia = foto_de(&mensajes, &format!("HEROE:{h_ana}"));
    let h = heroes(&propia.entidades);
    assert_eq!(h.len(), 1);
    let h = h[0];
    assert_eq!((h.id.as_str(), h.jugador_id.as_deref(), h.es_bot, h.nivel), (h_ana.as_str(), Some("ana"), false, 1));
    assert_eq!((h.display_name.as_str(), h.class_definition_id.as_str(), h.femenino), (format!("Héroe {}", x.ana).as_str(), "default", false), "valores por defecto hasta que Conquest publique su catálogo");
    assert_eq!(h.ubicacion.as_ref().and_then(|u| u.donde.clone()), Some(pb::ubicacion_heroe::Donde::AsentamientoId(as_a.clone())));
    assert_eq!(h.residencia_id.as_deref(), Some(as_a.as_str()), "reside en su plaza");
    assert_eq!(h.escuadrones_ids, [ids::escuadra(escuadra)]);
    assert_eq!((h.loadouts.len(), h.loadouts[0].display_name.as_str(), h.loadouts[0].activo), (1, "Default", true));
    assert!(h.liderazgo_base > 0);
    assert_eq!(i64::from(h.cupo_guarnicion), cupo, "derivado: el cupo de la guarnición de su residencia");
    assert!(h.monedas_heroe.as_ref().unwrap().bronce > 0);
    let e = escuadrones(&propia.entidades);
    assert_eq!(e.len(), 1);
    assert_eq!((e[0].id.as_str(), e[0].heroe_id.as_str(), e[0].tropa_id.as_str(), e[0].origen.as_str(), e[0].nivel), (ids::escuadra(escuadra).as_str(), h_ana.as_str(), "milicia_lanceros", "pesants", 1));
    assert_eq!(e[0].contenedor.as_ref().and_then(|c| c.donde.clone()), Some(pb::contenedor_escuadron::Donde::Campamento(true)));
    assert!(e[0].cantidad > 0 && e[0].moral.is_some());
    // La caravana de A no es de ninguna columna suya ni lleva escolta suya: no está en su héroe, está en su Facción.
    assert!(caravanas(&propia.entidades).is_empty() && ejercitos(&propia.entidades).is_empty() && convocatorias(&propia.entidades).is_empty() && batallas(&propia.entidades).is_empty());

    let f1 = foto_de(&mensajes, "FACCION:f-1");
    let f = facciones(&f1.entidades);
    assert_eq!((f.len(), f[0].id.as_str(), f[0].nivel), (1, "f-1", 1));
    assert_eq!(f[0].ciudadanos_ids, [ids::heroe(x.fundadores[0]), ids::heroe(x.ana), ids::heroe(x.beto), ids::heroe(x.dora)], "los ciudadanos de F1: el héroe de campo que fundó la plaza, ana, beto y dora");
    assert!(f[0].reputacion.is_some());
    assert_eq!(asentamientos(&f1.entidades).iter().map(|p| p.id.clone()).collect::<Vec<_>>(), [as_a.clone()], "su plaza en resumen: la de F2 no es suya");
    let ficha = asentamientos(&f1.entidades)[0];
    assert_eq!((ficha.faccion_id.as_str(), ficha.nivel, ficha.zona.is_empty()), ("f-1", 1, false));
    assert!(ficha.almacen.is_empty() && ficha.edificios.is_empty(), "el interior es del ámbito de la plaza");
    let t = tecnologias(&f1.entidades);
    assert!(t.len() == 1 && t[0].adoptadas.contains(&"arco_compuesto".to_owned()) && t[0].adoptadas.len() > 1, "las de arranque y las que dio la administración");
    let r = relaciones(&f1.entidades);
    assert_eq!(r.len(), 1);
    assert_eq!((r[0].id.as_str(), r[0].tipo, r[0].faccion_a_id.as_str(), r[0].faccion_b_id.as_str()), ("rel-1-2", pb::TipoRelacion::RelacionAlianza as i32, "f-1", "f-2"));
    let k = caravanas(&f1.entidades);
    assert_eq!(k.len(), 1);
    assert_eq!((k[0].id.as_str(), k[0].origen_asentamiento_id.as_str(), k[0].estado, k[0].carros.len(), k[0].reservada_manual), (ids::caravana(caravana).as_str(), as_a.as_str(), pb::EstadoCaravana::CaravanaDisponible as i32, 1, true));
    assert_eq!(k[0].carros[0].animal.as_deref(), Some("buey"));
    let o = ordenes(&f1.entidades);
    assert_eq!((o.len(), o[0].asentamiento_id.as_str(), o[0].es_compra, o[0].recurso.as_str(), o[0].estado), (1, as_a.as_str(), false, "madera", pb::EstadoOrden::OrdenActiva as i32));
    let tr = trueques(&f1.entidades);
    assert_eq!((tr.len(), tr[0].asentamiento_a_id.as_str(), tr[0].asentamiento_b_id.as_str(), tr[0].estado), (1, as_a.as_str(), as_b.as_str(), pb::EstadoTrueque::TruequePropuesto as i32));
    assert!(pueblos(&f1.entidades).is_empty() && obras(&f1.entidades).is_empty() && avanzadas(&f1.entidades).is_empty());

    // beto, en el campamento: reside y está dentro de él.
    let (_, _, mensajes) = w.conectar(hola_con("beto", TipoCliente::ClienteUnity, &[])).await;
    let h = heroes(&foto_de(&mensajes, &format!("HEROE:{}", ids::heroe(x.beto))).entidades)[0].clone();
    assert_eq!((h.residencia_id.as_deref(), h.ubicacion.and_then(|u| u.donde)), (Some("camp-0"), Some(pb::ubicacion_heroe::Donde::CampamentoId("camp-0".into()))));

    // gus, de F2 dentro de su plaza: la foto de su Facción lleva la plaza B, el trueque con A y la relación, y ni una orden, caravana, plaza ni ciudadano de F1.
    let (_, _, mensajes) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;
    let f2 = foto_de(&mensajes, "FACCION:f-2");
    assert_eq!(asentamientos(&f2.entidades).iter().map(|p| p.id.clone()).collect::<Vec<_>>(), [as_b]);
    assert_eq!(ids_de(&trueques(&f2.entidades), |t| &t.id).len(), 1, "un trueque con una plaza propia es de las dos Facciones");
    assert_eq!(ids_de(&relaciones(&f2.entidades), |r| &r.id), ["rel-1-2"]);
    assert!(caravanas(&f2.entidades).is_empty() && ordenes(&f2.entidades).is_empty(), "la flota y las órdenes de F1 no son de F2");
    assert_eq!(facciones(&f2.entidades)[0].ciudadanos_ids, [ids::heroe(x.fundadores[1]), ids::heroe(x.gus)], "ni un ciudadano de F1");
    assert!(foto_de(&mensajes, &format!("HEROE:{}", ids::heroe(x.gus))).entidades.iter().all(|e| !matches!(e.cuerpo, Some(Cuerpo::Escuadron(_)))), "las escuadras de ana no son de gus");
    w.parar().await;
}

// ───────────────────────────── 2. Héroe: reclutar y un loadout ─────────────────────────────

#[tokio::test]
async fn reclutar_y_guardar_un_loadout_llegan_a_su_heroe_y_a_nadie_mas() {
    let m = con_mundo!();
    let x = escenario(m);
    let (a, h_ana) = (x.a, ids::heroe(x.ana));
    let w = arrancar_con(x.n).await;
    let (mut ana, _, mensajes) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let mut vista_ana = Cliente::desde(foto_de(&mensajes, &format!("HEROE:{h_ana}")));
    let (mut beto, _, _) = w.conectar(hola_con("beto", TipoCliente::ClienteUnity, &[])).await;
    let (mut gus, _, _) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;

    // Reclutar una escuadra: la delta de su héroe trae la escuadra nueva y el héroe que ya la cuenta; el resultado dice cuál es.
    let reclutar = orden::Cuerpo::ReclutarTropa(pb::OrdenReclutarTropa { asentamiento_id: ids::asentamiento(a), heroe_id: h_ana.clone(), tropa_id: "milicia_lanceros".into(), origen: None });
    let (previos, r) = dar_todo(&mut ana, "r1", reclutar).await;
    aceptada(&r);
    let escuadra = r.creadas[0].clone();
    assert_eq!(TipoEntidad::try_from(escuadra.entidad).unwrap(), TipoEntidad::Escuadron);
    let del_heroe = deltas_de(&previos, &format!("HEROE:{h_ana}"));
    assert_eq!(del_heroe.len(), 1, "{:?}", etiquetas(&previos));
    let (nuevos_h, nuevas_e) = (heroes(nuevas(del_heroe[0])), escuadrones(nuevas(del_heroe[0])));
    assert_eq!((nuevos_h.len(), nuevos_h[0].escuadrones_ids.clone()), (1, vec![escuadra.id.clone()]));
    assert_eq!((nuevas_e.len(), nuevas_e[0].id.as_str(), nuevas_e[0].tropa_id.as_str(), nuevas_e[0].heroe_id.as_str()), (1, escuadra.id.as_str(), "milicia_lanceros", h_ana.as_str()));
    assert!(quitas(del_heroe[0]).is_empty());
    vista_ana.aplicar(del_heroe[0]);

    // Guardar un loadout con esa escuadra: el héroe lo trae entero, con su Liderazgo derivado.
    let loadout = orden::Cuerpo::GuardarLoadout(pb::OrdenGuardarLoadout { loadout_id: None, display_name: "Ofensivo".into(), squad_ids: vec![escuadra.id.clone()], perks_seleccionados: vec![], activo: None });
    let (previos, r) = dar_todo(&mut ana, "r2", loadout).await;
    aceptada(&r);
    assert_eq!(TipoEntidad::try_from(r.creadas[0].entidad).unwrap(), TipoEntidad::Loadout);
    let del_heroe = deltas_de(&previos, &format!("HEROE:{h_ana}"));
    assert_eq!(del_heroe.len(), 1);
    let nuevo = heroes(nuevas(del_heroe[0]))[0].loadouts.iter().find(|l| l.display_name == "Ofensivo").expect("el loadout nuevo").clone();
    assert_eq!((nuevo.id, nuevo.squad_ids.clone(), nuevo.activo), (r.creadas[0].id.clone(), vec![escuadra.id.clone()], false));
    assert!(nuevo.liderazgo_total > 0, "el Liderazgo del loadout se deriva de sus escuadras");
    vista_ana.aplicar(del_heroe[0]);

    // La Facción ve cambiar la plaza (reclutar le gasta población y almacén) pero ni una escuadra ni un loadout; el héroe es solo de ana, y la otra Facción no se entera de nada.
    assert_eq!(etiquetas(&pendiente(&mut beto).await), ["delta FACCION:f-1"]);
    assert!(pendiente(&mut gus).await.is_empty());
    // Lo que ana reconstruyó con la foto y los deltas es lo que hay.
    let (_, _, mensajes) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    assert!(vista_ana.entidades == Cliente::desde(foto_de(&mensajes, &format!("HEROE:{h_ana}"))).entidades, "foto + deltas = foto final");
    w.parar().await;
}

// ───────────────────────────── 3. Convocatoria y columna ─────────────────────────────

#[tokio::test]
async fn una_convocatoria_llega_a_su_faccion_con_las_peticiones_redactadas_y_la_columna_que_parte_a_sus_participantes() {
    let m = con_mundo!();
    let x = escenario(m);
    let (as_a, h_ana, h_dora) = (ids::asentamiento(x.a), ids::heroe(x.ana), ids::heroe(x.dora));
    let w = arrancar_con(x.n).await;
    let (mut ana, _, _) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let (mut dora, _, _) = w.conectar(hola_con("dora", TipoCliente::ClienteUnity, &[])).await;
    let (mut beto, _, _) = w.conectar(hola_con("beto", TipoCliente::ClienteUnity, &[])).await;
    let (mut gus, _, _) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;
    let (mut admin_c, _, _) = w.conectar(hola_con("admin", TipoCliente::ClienteAdminWeb, &[])).await;
    let faccion = "FACCION:f-1";

    // ana convoca en su plaza, preguntando: la convocatoria es de la Facción, la ven los tres de F1 y no gus.
    let convocar = orden::Cuerpo::ConvocarEjercito(pb::OrdenConvocarEjercito { heroe_id: h_ana.clone(), politica_de_union: "preguntar".into(), escuadron_ids: vec![], carga: None });
    let (previos, r) = dar_todo(&mut ana, "c1", convocar).await;
    aceptada(&r);
    let convocatoria = r.creadas[0].clone();
    assert_eq!(TipoEntidad::try_from(convocatoria.entidad).unwrap(), TipoEntidad::Convocatoria);
    let c = convocatorias(nuevas(deltas_de(&previos, faccion)[0]));
    assert_eq!((c.len(), c[0].id.as_str(), c[0].lider_id.as_str(), c[0].lugar_id.as_str(), c[0].lugar_es_campamento), (1, convocatoria.id.as_str(), h_ana.as_str(), as_a.as_str(), false));
    for compania in [&mut dora, &mut beto] {
        let recibido = pendiente(compania).await;
        assert_eq!(convocatorias(nuevas(deltas_de(&recibido, faccion)[0])).len(), 1, "{:?}", etiquetas(&recibido));
    }
    assert!(pendiente(&mut gus).await.is_empty());

    // dora pide unirse: su petición la ve el Líder y la ve ella, no beto.
    let unirse = orden::Cuerpo::UnirseAConvocatoria(pb::OrdenUnirseAConvocatoria { heroe_id: h_dora.clone(), convocatoria_id: convocatoria.id.clone(), escuadron_ids: vec![], carga: None });
    let (previos, r) = dar_todo(&mut dora, "c2", unirse).await;
    aceptada(&r);
    let peticiones = |mensajes: &[ms::Cuerpo]| convocatorias(nuevas(deltas_de(mensajes, faccion)[0]))[0].peticiones.iter().map(|p| p.heroe_id.clone()).collect::<Vec<_>>();
    assert_eq!(peticiones(&previos), [h_dora.clone()]);
    assert_eq!(peticiones(&pendiente(&mut ana).await), [h_dora.clone()], "el Líder ve todas las peticiones");
    assert!(peticiones(&pendiente(&mut beto).await).is_empty(), "un miembro de la Facción que no es Líder ni solicitante no ve la petición");
    let de_admin = pendiente(&mut admin_c).await;
    let en_admin: Vec<String> = deltas_de(&de_admin, "ADMIN:").into_iter().flat_map(|d| convocatorias(nuevas(d))).flat_map(|c| c.peticiones.iter().map(|p| p.heroe_id.clone())).collect();
    assert!(en_admin.ends_with(std::slice::from_ref(&h_dora)), "la administración ve todas las peticiones (4H T3e): {en_admin:?}");

    // ana la acepta y parte: la convocatoria desaparece (QUITA para toda la Facción) y la columna con los dos nace en el ámbito de cada uno.
    let responder = orden::Cuerpo::ResponderPeticionDeConvocatoria(pb::OrdenResponderPeticionDeConvocatoria { heroe_id: h_ana.clone(), convocatoria_id: convocatoria.id.clone(), solicitante_id: h_dora.clone(), aceptar: true });
    aceptada(&dar_todo(&mut ana, "c3", responder).await.1);
    pendiente(&mut dora).await;
    pendiente(&mut beto).await;
    let partir = orden::Cuerpo::PartirConvocatoria(pb::OrdenPartirConvocatoria { heroe_id: h_ana.clone() });
    let (previos, r) = dar_todo(&mut ana, "c4", partir).await;
    aceptada(&r);
    let columna = r.creadas.iter().find(|c| c.entidad == TipoEntidad::Ejercito as i32).expect("la columna que sale").id.clone();
    assert!(quitas(deltas_de(&previos, faccion)[0]).contains(&(TipoEntidad::Convocatoria, convocatoria.id.clone())));
    for (nombre, quien, h) in [("ana", &previos, h_ana.clone()), ("dora", &pendiente(&mut dora).await, h_dora.clone())] {
        let d = deltas_de(quien, &format!("HEROE:{h}"));
        assert_eq!(d.len(), 1, "{nombre}: {:?}", etiquetas(quien));
        let e = ejercitos(nuevas(d[0]));
        assert_eq!((e.len(), e[0].id.as_str(), e[0].participantes.len()), (1, columna.as_str(), 2), "{nombre}: la columna con los dos");
        assert_eq!(heroes(nuevas(d[0]))[0].ubicacion.as_ref().and_then(|u| u.donde.clone()), Some(pb::ubicacion_heroe::Donde::EjercitoId(columna.clone())));
    }
    let recibido = pendiente(&mut beto).await;
    assert_eq!(etiquetas(&recibido).iter().filter(|e| e.starts_with("delta HEROE")).count(), 0, "beto no va en la columna");
    assert!(quitas(deltas_de(&recibido, faccion)[0]).contains(&(TipoEntidad::Convocatoria, convocatoria.id.clone())));
    assert!(pendiente(&mut gus).await.is_empty());

    // dora se separa: la columna deja de ser suya (QUITA en su ámbito) y sigue siendo de ana; dora sale en una columna nueva.
    let (previos, r) = dar_todo(&mut dora, "c5", orden::Cuerpo::SepararseDelEjercito(pb::OrdenSepararseDelEjercito { heroe_id: h_dora.clone() })).await;
    aceptada(&r);
    let d = deltas_de(&previos, &format!("HEROE:{h_dora}"))[0];
    assert!(quitas(d).contains(&(TipoEntidad::Ejercito, columna.clone())), "la columna de la que se separa deja de ser suya");
    assert_eq!(ejercitos(nuevas(d)).len(), 1, "y se queda con la suya");
    let de_ana = pendiente(&mut ana).await;
    let e = ejercitos(nuevas(deltas_de(&de_ana, &format!("HEROE:{h_ana}"))[0]));
    assert_eq!((e[0].id.as_str(), e[0].participantes.len()), (columna.as_str(), 1));

    // ana guarnece su plaza con el ejército: la columna se disuelve y la baja llega a su ámbito.
    let guarnecer = orden::Cuerpo::Guarnecer(pb::OrdenGuarnecer { asentamiento_id: as_a, heroe_id: h_ana.clone() });
    let (previos, r) = dar_todo(&mut ana, "c6", guarnecer).await;
    aceptada(&r);
    let d = deltas_de(&previos, &format!("HEROE:{h_ana}"))[0];
    assert!(quitas(d).contains(&(TipoEntidad::Ejercito, columna)), "{:?}", d.cambios.iter().map(|c| (c.tipo, c.entidad, c.id.clone())).collect::<Vec<_>>());
    let mut n = w.parar().await;
    pertenencias_al_dia(&mut n, &["ana", "dora"], &[1]);
}

// ───────────────────────────── 4. Caravanas: escolta, adjunta y aparcada ─────────────────────────────

#[tokio::test]
async fn una_caravana_es_del_heroe_mientras_la_escolta_o_va_en_su_columna_y_de_la_faccion_siempre() {
    let m = con_mundo!();
    let mut x = escenario(m);
    let (a, as_a, h_ana) = (x.a, ids::asentamiento(x.a), ids::heroe(x.ana));
    let escuadra = ids::escuadra(x.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: x.ana, tropa: Tropa::MiliciaLanceros }).creadas[0]);
    let caravana = ids::caravana(x.caravana("ana", a));
    let w = arrancar_con(x.n).await;
    let (mut ana, _, mensajes) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let (mut beto, _, _) = w.conectar(hola_con("beto", TipoCliente::ClienteUnity, &[])).await;
    let (mut dora, _, _) = w.conectar(hola_con("dora", TipoCliente::ClienteUnity, &[])).await;
    let mut vista = Cliente::desde(foto_de(&mensajes, &format!("HEROE:{h_ana}")));
    assert!(!vista.tiene(TipoEntidad::Caravana, &caravana), "la caravana de la plaza, sin escolta suya ni columna suya, no está en su héroe");
    let (ambito_h, faccion) = (format!("HEROE:{h_ana}"), "FACCION:f-1");

    // Ceder la escuadra de escolta: la caravana entra en el héroe (con la escolta) y la escuadra cambia de contenedor; la Facción ve la caravana con su escolta.
    let escolta = orden::Cuerpo::AsignarEscolta(pb::OrdenAsignarEscolta { caravana_id: caravana.clone(), heroe_id: h_ana.clone(), escuadron_ids: vec![escuadra.clone()] });
    let (previos, r) = dar_todo(&mut ana, "k1", escolta).await;
    aceptada(&r);
    let d = deltas_de(&previos, &ambito_h)[0];
    let k = caravanas(nuevas(d));
    assert_eq!((k.len(), k[0].id.as_str(), k[0].escolta_ids.clone(), k[0].escoltada), (1, caravana.as_str(), vec![escuadra.clone()], true));
    let e = escuadrones(nuevas(d));
    assert_eq!(e[0].contenedor.as_ref().and_then(|c| c.donde.clone()), Some(pb::contenedor_escuadron::Donde::CaravanaId(caravana.clone())));
    vista.aplicar(d);
    let de_beto = pendiente(&mut beto).await;
    assert_eq!(caravanas(nuevas(deltas_de(&de_beto, faccion)[0]))[0].escolta_ids, [escuadra.clone()], "la Facción ve la escolta de la caravana de su plaza");
    assert!(deltas_de(&de_beto, &ambito_h).is_empty() && deltas_de(&pendiente(&mut dora).await, &ambito_h).is_empty(), "la escuadra y su héroe son solo de ana");

    // Retirarla: la caravana deja de ser del héroe (QUITA en su ámbito) y sigue siendo de la Facción (ACTUALIZA, ya sin escolta).
    let retirar = orden::Cuerpo::QuitarEscolta(pb::OrdenQuitarEscolta { caravana_id: caravana.clone(), heroe_id: h_ana.clone(), escuadron_ids: vec![escuadra.clone()] });
    let (previos, r) = dar_todo(&mut ana, "k2", retirar).await;
    aceptada(&r);
    let d = deltas_de(&previos, &ambito_h)[0];
    assert_eq!(quitas(d), [(TipoEntidad::Caravana, caravana.clone())]);
    vista.aplicar(d);
    assert!(!vista.tiene(TipoEntidad::Caravana, &caravana));
    let de_ana = deltas_de(&previos, faccion);
    assert_eq!(caravanas(nuevas(de_ana[0]))[0].escolta_ids.len(), 0);
    pendiente(&mut beto).await;

    // Sale como ejército (solo un ejército lleva caravanas), engancha la caravana y la aparca en la plaza: del héroe mientras va adjunta, de la Facción en cada estado.
    let salir = orden::Cuerpo::MovilizarEjercito(pb::OrdenMovilizarEjercito { asentamiento_id: as_a.clone(), heroe_id: h_ana.clone(), escuadron_ids: vec![escuadra.clone()], objetivo: None, politica_de_union: Some("aceptar".into()), carga: None });
    let (previos, r) = dar_todo(&mut ana, "k3", salir).await;
    aceptada(&r);
    let columna = r.creadas[0].id.clone();
    vista.aplicar(deltas_de(&previos, &ambito_h)[0]);
    pendiente(&mut beto).await;
    let adjuntar = orden::Cuerpo::AdjuntarCaravana(pb::OrdenAdjuntarCaravana { ejercito_id: columna.clone(), caravana_id: caravana.clone(), heroe_id: h_ana.clone() });
    let (previos, r) = dar_todo(&mut ana, "k4", adjuntar).await;
    aceptada(&r);
    let d = deltas_de(&previos, &ambito_h)[0];
    assert_eq!((caravanas(nuevas(d))[0].estado, ejercitos(nuevas(d))[0].caravanas_adjuntas_ids.clone()), (pb::EstadoCaravana::CaravanaAdjunta as i32, vec![caravana.clone()]));
    vista.aplicar(d);
    let de_beto = pendiente(&mut beto).await;
    assert_eq!(caravanas(nuevas(deltas_de(&de_beto, faccion)[0]))[0].estado, pb::EstadoCaravana::CaravanaAdjunta as i32);

    let aparcar = orden::Cuerpo::AparcarCaravana(pb::OrdenAparcarCaravana { caravana_id: caravana.clone(), asentamiento_id: as_a });
    let (previos, r) = dar_todo(&mut ana, "k5", aparcar).await;
    aceptada(&r);
    let d = deltas_de(&previos, &ambito_h)[0];
    assert_eq!(quitas(d), [(TipoEntidad::Caravana, caravana.clone())], "aparcada, la caravana deja su ámbito");
    assert!(ejercitos(nuevas(d))[0].caravanas_adjuntas_ids.is_empty());
    vista.aplicar(d);
    let de_beto = pendiente(&mut beto).await;
    assert_eq!(caravanas(nuevas(deltas_de(&de_beto, faccion)[0]))[0].estado, pb::EstadoCaravana::CaravanaAparcada as i32, "y es de la Facción, aparcada");
    assert!(deltas_de(&pendiente(&mut dora).await, &ambito_h).is_empty());

    // Lo que ana reconstruyó con la foto y los deltas es lo que hay.
    let (_, _, mensajes) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    assert!(vista.entidades == Cliente::desde(foto_de(&mensajes, &ambito_h)).entidades, "foto + deltas = foto final");
    let mut n = w.parar().await;
    pertenencias_al_dia(&mut n, &["ana"], &[1]);
}

// ───────────────────────────── 5. Mercado, trueques y relaciones ─────────────────────────────

#[tokio::test]
async fn las_ordenes_de_mercado_los_trueques_y_las_relaciones_llegan_a_la_faccion_que_los_tiene_y_a_ninguna_mas() {
    let m = con_mundo!();
    let x = escenario(m);
    let (as_a, as_b) = (ids::asentamiento(x.a), ids::asentamiento(x.b));
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (mut ana, _, _) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let (mut beto, _, _) = w.conectar(hola_con("beto", TipoCliente::ClienteUnity, &[])).await;
    let (mut gus, _, _) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;
    let (f1, f2) = ("FACCION:f-1", "FACCION:f-2");

    // Una orden de mercado de A: la ven los de F1 (también beto, que no está en la plaza) y no gus. Retirarla la deja en el historial, cancelada.
    let colocar = orden::Cuerpo::ColocarOrdenMercado(pb::OrdenColocarOrdenMercado { asentamiento_id: as_a.clone(), tipo: "venta".into(), recurso: "madera".into(), cantidad: 10_000, precio: None });
    let (previos, r) = dar_todo(&mut ana, "m1", colocar).await;
    aceptada(&r);
    let orden_id = r.creadas[0].id.clone();
    let o = ordenes(nuevas(deltas_de(&previos, f1)[0]));
    assert_eq!((o.len(), o[0].id.as_str(), o[0].asentamiento_id.as_str(), o[0].estado), (1, orden_id.as_str(), as_a.as_str(), pb::EstadoOrden::OrdenActiva as i32));
    let de_beto = pendiente(&mut beto).await;
    assert_eq!(ordenes(nuevas(deltas_de(&de_beto, f1)[0])).len(), 1);
    assert!(pendiente(&mut gus).await.is_empty(), "las órdenes de F1 no son de F2");
    let (previos, r) = dar_todo(&mut ana, "m2", orden::Cuerpo::CancelarOrdenMercado(pb::OrdenCancelarOrdenMercado { orden_id: orden_id.clone() })).await;
    aceptada(&r);
    assert_ne!(ordenes(nuevas(deltas_de(&previos, f1)[0]))[0].estado, pb::EstadoOrden::OrdenActiva as i32);
    pendiente(&mut beto).await;

    // Un trueque de A a B: es de las dos Facciones; gus lo rechaza y las dos lo ven rechazado.
    let lineas_a = vec![pb::orden_proponer_trueque::LineasA { recurso: "madera".into(), cantidad: 10_000 }];
    let lineas_b = vec![pb::orden_proponer_trueque::LineasB { recurso: "piedra".into(), cantidad: 5_000 }];
    let (previos, r) = dar_todo(&mut ana, "m3", orden::Cuerpo::ProponerTrueque(pb::OrdenProponerTrueque { asentamiento_a_id: as_a.clone(), lineas_a, asentamiento_b_id: as_b.clone(), lineas_b })).await;
    aceptada(&r);
    let acuerdo = r.creadas[0].id.clone();
    let t = trueques(nuevas(deltas_de(&previos, f1)[0]));
    assert_eq!((t.len(), t[0].id.as_str(), t[0].estado), (1, acuerdo.as_str(), pb::EstadoTrueque::TruequePropuesto as i32));
    let de_gus = pendiente(&mut gus).await;
    let t = trueques(nuevas(deltas_de(&de_gus, f2)[0]));
    assert_eq!((t.len(), t[0].id.as_str(), t[0].asentamiento_b_id.as_str()), (1, acuerdo.as_str(), as_b.as_str()), "la otra plaza lo ve en su Facción");
    assert!(deltas_de(&de_gus, f1).is_empty());
    pendiente(&mut beto).await;
    let (previos, r) = dar_todo(&mut gus, "m4", orden::Cuerpo::RechazarTrueque(pb::OrdenRechazarTrueque { acuerdo_id: acuerdo.clone() })).await;
    aceptada(&r);
    assert_eq!(trueques(nuevas(deltas_de(&previos, f2)[0]))[0].estado, pb::EstadoTrueque::TruequeRechazado as i32);
    let de_ana = pendiente(&mut ana).await;
    assert_eq!(trueques(nuevas(deltas_de(&de_ana, f1)[0]))[0].estado, pb::EstadoTrueque::TruequeRechazado as i32, "y la plaza que lo propuso, también");

    // Una relación entre las Facciones: la ven las dos, con el mismo id; quitarla es un QUITA para las dos.
    let fijar = |relacion: Option<&str>| {
        let mut p = vec![("faccion_a_id", "f-1"), ("faccion_b_id", "f-2")];
        p.extend(relacion.map(|r| ("relacion", r)));
        admin("fijar_relacion", &p)
    };
    let (_, r) = dar_todo(&mut admin_c, "m5", fijar(Some("alianza"))).await;
    aceptada(&r);
    for (c, ambito_f) in [(&mut ana, f1), (&mut gus, f2)] {
        let recibido = pendiente(c).await;
        let d = deltas_de(&recibido, ambito_f)[0];
        let rel = relaciones(nuevas(d));
        assert_eq!((rel.len(), rel[0].id.as_str(), rel[0].tipo), (1, "rel-1-2", pb::TipoRelacion::RelacionAlianza as i32), "{ambito_f}");
        assert_eq!(facciones(nuevas(d)).len(), 1, "su Facción cambió también: ya tiene una alianza");
    }
    let (_, r) = dar_todo(&mut admin_c, "m6", fijar(Some("guerra"))).await;
    aceptada(&r);
    assert_eq!(relaciones(nuevas(deltas_de(&pendiente(&mut ana).await, f1)[0]))[0].tipo, pb::TipoRelacion::RelacionGuerra as i32);
    pendiente(&mut gus).await;
    let (_, r) = dar_todo(&mut admin_c, "m7", fijar(None)).await;
    aceptada(&r);
    for (c, ambito_f) in [(&mut ana, f1), (&mut gus, f2)] {
        let recibido = pendiente(c).await;
        assert_eq!(quitas(deltas_de(&recibido, ambito_f)[0]), [(TipoEntidad::RelacionPolitica, "rel-1-2".to_owned())], "{ambito_f}");
    }
    w.parar().await;
}

// ───────────────────────────── 6. Pueblos y avanzadas ─────────────────────────────

/// Un sitio fundable, verde y con mucho bosque, a `r` del punto `c`.
fn sitio_junto_a(n: &Nucleo, c: Punto, r: i64) -> Option<Punto> {
    (0..36).map(|g| ((g * 10) as f64).to_radians()).map(|a| Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64)).find(|&p| {
        n.viabilidad(p, 1).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6 && !v.condenada)
    })
}

/// La plaza de `ana` (F1, nivel 2, zona agrandada) junto a un nodo de cobre con un pueblo levantado y activo, con su carro de acarreo; y una columna de `ana` a 8 u de la plaza con los
/// materiales de la primera obra de una avanzada. Devuelve el escenario, el pueblo y esa columna.
fn con_pueblo(m: &M) -> (Esc, u64, u64) {
    let mut reglas = Reglas::default();
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let n = Nucleo::nuevo(Arranque { reglas, ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
    let mut x = Esc { n, a: 0, b: 0, ana: 0, beto: 0, gus: 0, dora: 0, fundadores: [0; 2], peticiones: 0 };
    let mapa = nucleo::economia::Mapa::de(&m.0);
    let (cobre, sitio) = (0..mapa.nodos.len())
        .filter(|&i| mapa.nodos[i].tipo == Tipo::MinaCobre)
        .find_map(|i| sitio_junto_a(&x.n, mapa.nodos[i].pos, 6_000).map(|p| (i as u32, p)))
        .expect("hay un nodo de cobre junto a un sitio rico");
    let t = x.n.ahora();
    let columna = x.n.crear_ejercito(t, "ana", 1, sitio, 500, 100_000).unwrap();
    x.ok("admin", Comando::DarCaravanaFundacion { ejercito: columna });
    x.a = x.ok("ana", Comando::Fundar { ejercito: columna }).creadas[0];
    let a = x.a;
    x.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
    x.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel: 2 });
    for _ in 0..14 {
        x.ok("admin", Comando::AgrandarZona { asentamiento: a });
    }
    for r in [Madera, Piedra, Oro] {
        x.dar(a, r, 300_000);
    }
    let pueblo = x.ok("ana", Comando::MarcarPueblo { asentamiento: a, nodo: cobre }).creadas[0];
    let puerta = x.n.posicion_de_pueblo(pueblo).unwrap();
    let t = x.n.ahora();
    let obrera = x.n.crear_ejercito(t, "ana", 1, puerta, 500, 100_000).unwrap();
    x.ok("admin", Comando::DarSuministro { ejercito: obrera, recurso: Madera, cantidad: 100_000 });
    x.ok("admin", Comando::DarSuministro { ejercito: obrera, recurso: Oro, cantidad: 12_000 });
    x.ok("ana", Comando::EntregarMateriales { ejercito: obrera, pueblo });
    let asentamiento::pueblos::EstadoPueblo::EnObra { completa_en } = x.n.pueblo(pueblo).unwrap().1.estado else { panic!("debía estar en obra") };
    x.n.avanzar_hasta(completa_en + MS_POR_MINUTO);
    assert!(x.n.pueblo(pueblo).unwrap().1.activo());
    let centro = x.n.estado().asentamientos[&a].posicion;
    let t = x.n.ahora();
    let col = x.n.crear_ejercito(t, "ana", 1, Punto::new(centro.x + 8_000, centro.y), 500, 100_000).unwrap();
    for (r, c) in [(Madera, 150_000), (Piedra, 150_000), (Oro, 50_000)] {
        x.ok("admin", Comando::DarSuministro { ejercito: col, recurso: r, cantidad: c });
    }
    let ahora = x.n.ahora();
    x.n.avanzar_hasta((ahora / MS_POR_MINUTO + 1) * MS_POR_MINUTO + 1_000);
    (x, pueblo, col)
}

#[tokio::test]
async fn los_pueblos_y_las_avanzadas_de_la_faccion_viajan_en_resumen_con_su_obra_y_se_quitan_al_destruirlos() {
    let m = con_mundo!();
    let (x, pueblo, col) = con_pueblo(m);
    let (as_a, pueblo_id, carro) = (ids::asentamiento(x.a), ids::pueblo(pueblo), x.n.pueblo(pueblo).unwrap().1.carro.expect("el pueblo levantado trae su carro"));
    let heroe = ids::heroe(x.n.ejercito(col).unwrap().columna.lider);
    let centro = x.n.estado().asentamientos[&x.a].posicion;
    let sitio = Punto::new(centro.x + 8_000, centro.y);
    let w = arrancar_con(x.n).await;
    let (mut ana, _, mensajes) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let f1 = "FACCION:f-1";

    // La foto de la Facción: la plaza con los ids de su pueblo, el pueblo con su carro de acarreo (una caravana de la flota ligada a él); ni obra ni avanzada.
    let foto = foto_de(&mensajes, f1);
    let ficha = asentamientos(&foto.entidades)[0];
    assert_eq!((ficha.pueblos_ids.clone(), ficha.avanzadas_ids.len()), (vec![pueblo_id.clone()], 0));
    let p = pueblos(&foto.entidades);
    assert_eq!((p.len(), p[0].id.as_str(), p[0].asentamiento_madre_id.as_str(), p[0].tipo_extractor.as_str(), p[0].carro_acarreo_id.clone()), (1, pueblo_id.as_str(), as_a.as_str(), "minaCobre", Some(ids::caravana(carro))));
    assert!(p[0].posicion.is_some() && p[0].almacen_local.as_ref().unwrap().capacidad > 0);
    let k = caravanas(&foto.entidades);
    assert_eq!((k.len(), k[0].tipo, k[0].pueblo_id.clone()), (1, pb::TipoCaravana::CaravanaDeAcarreo as i32, Some(pueblo_id.clone())));
    assert!(obras(&foto.entidades).is_empty() && avanzadas(&foto.entidades).is_empty());

    // Marcar una avanzada deja una obra de mapa pendiente, que la entrega de materiales va llenando; destruirla la quita (sea cual sea la forma en que el cliente la tuviera).
    let colocar = orden::Cuerpo::ColocarAvanzada(pb::OrdenColocarAvanzada { asentamiento_id: as_a.clone(), heroe_id: heroe.clone(), posicion: Some(pb::Punto { x: sitio.x, y: sitio.y }) });
    let (previos, r) = dar_todo(&mut ana, "v1", colocar).await;
    aceptada(&r);
    let obra = r.creadas[0].id.clone();
    let o = obras(nuevas(deltas_de(&previos, f1)[0]));
    assert_eq!((o.len(), o[0].id.as_str(), o[0].tipo, o[0].requerido.as_ref().unwrap().por_id.is_empty(), o[0].entregado.as_ref().unwrap().por_id.values().sum::<i64>()), (1, obra.as_str(), pb::TipoObra::ObraAvanzada as i32, false, 0));
    let avanzada = ids::avanzada(ids::parse_obra(&obra).unwrap());
    assert!(quitas(deltas_de(&previos, f1)[0]).contains(&(TipoEntidad::Avanzada, avanzada.clone())), "mientras es una obra no es una avanzada");
    let entregar = orden::Cuerpo::EntregarEnObra(pb::OrdenEntregarEnObra { heroe_id: heroe.clone(), obra_id: obra.clone(), materiales: None });
    let (previos, r) = dar_todo(&mut ana, "v2", entregar).await;
    aceptada(&r);
    let o = obras(nuevas(deltas_de(&previos, f1)[0]));
    let (pedido, entregado) = (&o[0].requerido.as_ref().unwrap().por_id, &o[0].entregado.as_ref().unwrap().por_id);
    assert_eq!((entregado["piedra"], entregado["madera"] < pedido["madera"], o[0].termina_en_ms), (pedido["piedra"], true, None), "la columna entregó la piedra entera y parte de la madera: la obra sigue pendiente de materiales");
    let (previos, r) = dar_todo(&mut ana, "v3", orden::Cuerpo::DestruirAvanzada(pb::OrdenDestruirAvanzada { heroe_id: heroe.clone(), avanzada_id: avanzada.clone() })).await;
    aceptada(&r);
    let q = quitas(deltas_de(&previos, f1)[0]);
    assert!(q.contains(&(TipoEntidad::ObraDeMapa, obra)) && q.contains(&(TipoEntidad::Avanzada, avanzada)), "{q:?}");

    // Destruir el pueblo quita el pueblo y su carro: la caravana se pierde con él.
    let destruir = orden::Cuerpo::DestruirPueblo(pb::OrdenDestruirPueblo { heroe_id: heroe, pueblo_id: pueblo_id.clone() });
    let (previos, r) = dar_todo(&mut ana, "v4", destruir).await;
    aceptada(&r);
    let d = deltas_de(&previos, f1)[0];
    let q = quitas(d);
    assert!(q.contains(&(TipoEntidad::Pueblo, pueblo_id)) && q.contains(&(TipoEntidad::Caravana, ids::caravana(carro))), "{q:?}");
    assert!(asentamientos(nuevas(d))[0].pueblos_ids.is_empty(), "y la plaza ya no lo cuenta");
    w.parar().await;
}

// ───────────────────────────── 7. Una plaza cambia de dueño ─────────────────────────────

#[tokio::test]
async fn la_plaza_conquistada_y_lo_que_depende_de_ella_pasan_de_una_faccion_a_la_otra_con_su_baja_y_su_alta() {
    let m = con_mundo!();
    let mut x = escenario(m);
    let (a, b) = (x.a, x.b);
    let (as_a, as_b) = (ids::asentamiento(a), ids::asentamiento(b));
    // Lo que depende de A: una orden de mercado, una caravana y un trueque con B (cuyo dueño es F2).
    x.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Madera, cantidad: 10_000, precio: None });
    let caravana = ids::caravana(x.caravana("ana", a));
    let acuerdo = ids::acuerdo(x.ok("ana", Comando::ProponerTrueque { a, b, lineas_a: vec![(Madera, 10_000)], lineas_b: vec![(Piedra, 5_000)] }).creadas[0]);
    // gus tiene a la puerta de A un ejército de campo con soldados (y A no tiene quien la defienda: ni ana ni dora tienen escuadras).
    let puerta = x.n.estado().asentamientos[&a].posicion;
    let t = x.n.ahora();
    let columna = x.n.crear_ejercito(t, "gus", 2, Punto::new(puerta.x + 2_000, puerta.y), 500, 100_000).unwrap();
    let atacante = x.n.ejercito(columna).unwrap().columna.lider;
    let completa = militar::Reglas::cargar().tropas.tropa(Tropa::EspadachinesBronce).unidades;
    x.ok("admin", Comando::DarEscuadra { heroe: atacante, tropa: Tropa::EspadachinesBronce, cantidad: completa });
    let w = arrancar_con(x.n).await;
    let (mut ana, _, de_ana) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let (mut gus, _, de_gus) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;
    let mut f1 = Cliente::desde(foto_de(&de_ana, "FACCION:f-1"));
    let mut f2 = Cliente::desde(foto_de(&de_gus, "FACCION:f-2"));
    for (cliente, id) in [(&f1, &caravana), (&f1, &acuerdo)] {
        assert!(cliente.entidades.keys().any(|(_, i)| i == id), "{id} es de F1");
    }
    assert!(f1.tiene(TipoEntidad::Asentamiento, &as_a) && !f2.tiene(TipoEntidad::Asentamiento, &as_a));
    assert!(f2.tiene(TipoEntidad::AcuerdoTrueque, &acuerdo), "el trueque con A también es de F2");

    // gus ataca A (las dos Facciones no están en guerra: la declara) y A cae sin combate.
    let atacar = orden::Cuerpo::Atacar(pb::OrdenAtacar { heroe_id: ids::heroe(atacante), objetivo: Some(pb::orden_atacar::Objetivo { tipo: Some(pb::orden_atacar::objetivo::Tipo::AsentamientoId(as_a.clone())) }), declara_guerra: Some(true) });
    let (previos, r) = dar_todo(&mut gus, "q1", atacar).await;
    aceptada(&r);
    assert_eq!(faccion_de_la_plaza(&previos, "FACCION:f-2", &as_a), Some("f-2".to_owned()), "A llega a F2, que ahora la tiene en su Facción: {:?}", etiquetas(&previos));
    let mut todo_f2 = previos;
    todo_f2.extend(pendiente(&mut gus).await);
    for d in deltas_de(&todo_f2, "FACCION:f-2") {
        f2.aplicar(d);
    }
    let todo_f1 = pendiente(&mut ana).await;
    let d1 = deltas_de(&todo_f1, "FACCION:f-1");
    assert!(d1.iter().any(|d| quitas(d).contains(&(TipoEntidad::Asentamiento, as_a.clone()))), "F1 pierde la plaza: {:?}", etiquetas(&todo_f1));
    assert!(d1.iter().any(|d| quitas(d).contains(&(TipoEntidad::AcuerdoTrueque, acuerdo.clone()))), "y el trueque, que ahora es entre dos plazas de F2");
    for d in &d1 {
        f1.aplicar(d);
    }
    assert!(f1.entidades.keys().all(|(t, i)| *t != TipoEntidad::Asentamiento as i32 || *i != as_a), "F1 ya no cuenta la plaza");
    assert!(f1.tiene(TipoEntidad::Faccion, "f-1") && !f1.tiene(TipoEntidad::Asentamiento, &as_b));

    // Lo que cada uno reconstruyó con la foto y los deltas es lo que hay: F2 tiene las dos plazas; F1, ninguna.
    let (_, _, de_gus) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;
    let nueva = Cliente::desde(foto_de(&de_gus, "FACCION:f-2"));
    assert!(nueva.tiene(TipoEntidad::Asentamiento, &as_a) && nueva.tiene(TipoEntidad::Asentamiento, &as_b));
    assert!(f2.entidades == nueva.entidades, "foto + deltas = foto final (F2): {}", f2.diferencias(&nueva));
    let (_, _, de_ana) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let nueva = Cliente::desde(foto_de(&de_ana, "FACCION:f-1"));
    assert!(f1.entidades == nueva.entidades, "foto + deltas = foto final (F1): {}", f1.diferencias(&nueva));
    let mut n = w.parar().await;
    pertenencias_al_dia(&mut n, &["ana", "gus"], &[1, 2]);
}

/// La Facción dueña de la plaza `as_` en la última delta de `ambito` que la actualiza.
fn faccion_de_la_plaza(mensajes: &[ms::Cuerpo], ambito: &str, as_: &str) -> Option<String> {
    deltas_de(mensajes, ambito).iter().flat_map(|d| asentamientos(nuevas(d))).filter(|p| p.id == as_).map(|p| p.faccion_id.clone()).next_back()
}

// ───────────────────────────── 8. Reanudación ─────────────────────────────

fn colocar(heroe: u64, sitio: &str) -> orden::Cuerpo {
    admin("colocar_heroe", &[("heroe_id", &ids::heroe(heroe)), ("ubicacion", sitio)])
}

#[tokio::test]
async fn el_heroe_y_la_faccion_se_reanudan_por_deltas_aunque_el_heroe_cambiara_y_con_las_bajas_de_lo_que_desaparecio() {
    let m = con_mundo!();
    let mut x = escenario(m);
    let (as_a, h_ana, ana_id) = (ids::asentamiento(x.a), ids::heroe(x.ana), x.ana);
    x.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(nucleo::Relacion::Alianza) });
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (ana, b, mensajes) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let (k_heroe, k_faccion, k_plaza) = (format!("HEROE:{h_ana}"), "FACCION:f-1".to_owned(), format!("ASENTAMIENTO:{as_a}"));
    let v = b.ambitos[0].version_actual;
    let (mut heroe, mut faccion) = (Cliente::desde(foto_de(&mensajes, &k_heroe)), Cliente::desde(foto_de(&mensajes, &k_faccion)));
    assert!(faccion.tiene(TipoEntidad::RelacionPolitica, "rel-1-2"));
    ana.cerrar().await;

    // Mientras ana no está: le dan una escuadra, la relación desaparece, y su héroe sale de la plaza y vuelve (se toca dos veces).
    let cantidad = militar::Reglas::cargar().tropas.tropa(Tropa::MiliciaLanceros).unidades;
    let dar_escuadra = orden::Cuerpo::Administracion(pb::OrdenDeAdministracion {
        codigo: "dar_escuadra".into(),
        parametros: Some(Struct { fields: [("heroe_id".to_owned(), texto(&h_ana)), ("tropa_id".to_owned(), texto("milicia_lanceros")), ("cantidad".to_owned(), Value { kind: Some(Kind::NumberValue(cantidad as f64)) })].into() }),
    });
    aceptada(&dar_todo(&mut admin_c, "a1", dar_escuadra).await.1);
    aceptada(&dar_todo(&mut admin_c, "a2", admin("fijar_relacion", &[("faccion_a_id", "f-1"), ("faccion_b_id", "f-2")])).await.1);
    aceptada(&dar_todo(&mut admin_c, "a3", colocar(ana_id, &ids::campamento(0))).await.1);
    aceptada(&dar_todo(&mut admin_c, "a4", colocar(ana_id, &as_a)).await.1);

    // Al volver, los tres siguen por deltas: el héroe cambió, pero el registro dice a qué ámbito pertenecía cada cosa en cada versión y qué cambió en la plaza.
    let (_, b, mensajes) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[(&k_heroe, v), (&k_faccion, v), (&k_plaza, v)])).await;
    let decisiones: Vec<i32> = b.ambitos.iter().map(|a| a.decision).collect();
    assert_eq!(decisiones, [D::ReanudaConDeltas as i32; 3]);
    assert_eq!(etiquetas(&mensajes), [format!("delta {k_heroe}"), format!("delta {k_faccion}"), format!("delta {k_plaza}")]);
    let (d_heroe, d_faccion) = (deltas_de(&mensajes, &k_heroe)[0], deltas_de(&mensajes, &k_faccion)[0]);
    assert_eq!((d_heroe.desde, d_faccion.desde), (v, v));
    assert_eq!(escuadrones(nuevas(d_heroe)).len(), 1, "la escuadra que le dieron");
    assert_eq!(quitas(d_faccion), [(TipoEntidad::RelacionPolitica, "rel-1-2".to_owned())], "la relación que desapareció mientras estaba fuera");
    heroe.aplicar(d_heroe);
    faccion.aplicar(d_faccion);

    // Lo que reconstruyó con los deltas es lo que hay.
    let (_, _, nuevos) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let (h_final, f_final) = (Cliente::desde(foto_de(&nuevos, &k_heroe)), Cliente::desde(foto_de(&nuevos, &k_faccion)));
    assert!(heroe.entidades == h_final.entidades, "HEROE: foto + deltas = foto final: {}", heroe.diferencias(&h_final));
    assert!(faccion.entidades == f_final.entidades, "FACCION: foto + deltas = foto final: {}", faccion.diferencias(&f_final));

    // Una versión de antes de que arrancaran las sesiones (el registro de entonces no dice a qué ámbito iba cada cosa, como el que se repite al recuperar): foto, aunque el registro la alcance.
    let (_, b, _) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[(&k_heroe, 1), (&k_faccion, 1)])).await;
    assert_eq!(b.ambitos.iter().map(|a| a.decision).collect::<Vec<_>>()[..2], [D::ReanudaConFoto as i32; 2]);
    w.parar().await;
}

// ───────────────────────────── 9. La columna completa ─────────────────────────────

#[test]
fn la_columna_dice_a_quien_persigue_y_en_que_lugar_esta_dentro() {
    let m = con_mundo!();
    let mut x = escenario(m);
    let (puerta_a, puerta_b) = (x.n.estado().asentamientos[&x.a].posicion, x.n.estado().asentamientos[&x.b].posicion);
    let t = x.n.ahora();
    // gus persigue una columna de ana, lejos de las plazas.
    let presa = x.n.crear_ejercito(t, "ana", 1, Punto::new(puerta_a.x + 20_000, puerta_a.y + 20_000), 500, 100_000).unwrap();
    let cazador = x.n.crear_ejercito(t, "gus", 2, Punto::new(puerta_a.x + 25_000, puerta_a.y + 20_000), 500, 100_000).unwrap();
    let h_cazador = x.n.ejercito(cazador).unwrap().columna.lider;
    x.ok("gus", Comando::Perseguir { heroe: h_cazador, objetivo: militar::inspeccion::Objetivo::Ejercito(presa) });
    let proyectada = nucleo::proyeccion::propios::ejercito(&x.n, x.n.ejercito(cazador).unwrap());
    let p = proyectada.persiguiendo.expect("la columna persigue");
    assert_eq!((p.presa_es_caravana, p.presa_id.as_str()), (false, ids::ejercito(presa).as_str()));
    assert_eq!((proyectada.dentro_de, proyectada.en_batalla, proyectada.batalla_id), (None, false, None));
    assert!(nucleo::proyeccion::propios::ejercito(&x.n, x.n.ejercito(presa).unwrap()).persiguiendo.is_none());

    // Una columna de ana entra de visita en la plaza de F2, que deja pasar a los neutrales: sigue existiendo, dentro, y lo dice.
    let visitante = x.n.crear_ejercito(t, "ana", 1, Punto::new(puerta_b.x + 1_000, puerta_b.y), 500, 100_000).unwrap();
    let h_visitante = x.n.ejercito(visitante).unwrap().columna.lider;
    x.ok("ana", Comando::Guarnecer { asentamiento: x.b, heroe: h_visitante });
    let dentro = nucleo::proyeccion::propios::ejercito(&x.n, x.n.ejercito(visitante).expect("la columna visitante sigue existiendo"));
    assert_eq!(dentro.dentro_de, Some(ids::asentamiento(x.b)));
}

// ───────────────────────────── 10. Batallas ─────────────────────────────

#[tokio::test]
async fn la_batalla_de_unity_llega_a_sus_combatientes_en_su_ambito_y_resumida_a_las_facciones_de_los_dos_bandos() {
    let m = con_mundo!();
    let mut x = escenario_con(m, true);
    // Dos ejércitos de campo (los héroes de campo de ana y de gus, humanos y con tropa) a 2 u uno de otro, lejos de las plazas.
    let puerta = x.n.estado().asentamientos[&x.a].posicion;
    let sitio = Punto::new(puerta.x + 25_000, puerta.y + 25_000);
    let t = x.n.ahora();
    let (c_ana, c_gus) = (x.n.crear_ejercito(t, "ana", 1, sitio, 500, 350_000).unwrap(), x.n.crear_ejercito(t, "gus", 2, Punto::new(sitio.x + 2_000, sitio.y), 500, 350_000).unwrap());
    let (h_ana_campo, h_gus_campo) = (x.n.ejercito(c_ana).unwrap().columna.lider, x.n.ejercito(c_gus).unwrap().columna.lider);
    let completa = militar::Reglas::cargar().tropas.tropa(Tropa::EspadachinesBronce).unidades;
    for h in [h_ana_campo, h_gus_campo] {
        x.ok("admin", Comando::FijarControlador { heroe: h, controlador: Controlador::Humano });
        x.ok("admin", Comando::DarEscuadra { heroe: h, tropa: Tropa::EspadachinesBronce, cantidad: completa });
    }
    let (h_ana, h_gus) = (ids::heroe(x.ana), ids::heroe(x.gus));
    x.alinear();
    let w = arrancar_con(x.n).await;
    let (mut ana, _, de_ana) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    let (mut gus, _, _) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;
    let (mut beto, _, _) = w.conectar(hola_con("beto", TipoCliente::ClienteUnity, &[])).await;
    assert!(batallas(&foto_de(&de_ana, &format!("HEROE:{h_ana}")).entidades).is_empty());

    // ana ataca a gus (declarando la guerra): se abre una batalla de Unity, a la que va cada columna.
    let atacar = orden::Cuerpo::Atacar(pb::OrdenAtacar { heroe_id: ids::heroe(h_ana_campo), objetivo: Some(pb::orden_atacar::Objetivo { tipo: Some(pb::orden_atacar::objetivo::Tipo::EjercitoId(ids::ejercito(c_gus))) }), declara_guerra: Some(true) });
    let (previos, r) = dar_todo(&mut ana, "b1", atacar).await;
    aceptada(&r);
    // La batalla va a su ámbito propio, `BATALLA:<uuid>`, y no al del héroe; lo que cambia en la columna y en las escuadras sí va al héroe.
    let d = deltas_de(&previos, &format!("HEROE:{h_ana}"));
    assert_eq!(d.len(), 1, "{:?}", etiquetas(&previos));
    assert!(batallas(nuevas(d[0])).is_empty());
    let columna = ejercitos(nuevas(d[0])).into_iter().find(|c| c.id == ids::ejercito(c_ana)).expect("su columna");
    let foto_b = previos.iter().find_map(|m| if let ms::Cuerpo::Foto(f) = m { Some(f).filter(|f| ambito(&f.ambito).starts_with("BATALLA:")) } else { None }).expect("la foto de su batalla");
    let b = batallas(&foto_b.entidades);
    assert_eq!(b.len(), 1);
    let (batalla, uuid) = (b[0], b[0].id.clone());
    assert_eq!(ambito(&foto_b.ambito), format!("BATALLA:{uuid}"));
    assert_eq!((batalla.estado, batalla.lado_propio, batalla.iniciada_por.as_str()), (pb::EstadoBatalla::BatallaConvocando as i32, Some(pb::LadoId::Atacante as i32), ids::heroe(h_ana_campo).as_str()));
    assert!(batalla.ticket.is_some() && batalla.asignacion_propia.is_none() && batalla.lado_para_unirse.is_none(), "el combatiente ve el ticket; aún no hay asignación");
    assert_eq!(batalla.bloqueo_ejercito_ids.len(), 2);
    assert_eq!((columna.en_batalla, columna.batalla_id.as_deref()), (true, Some(uuid.as_str())));
    assert!(escuadrones(nuevas(d[0])).iter().all(|e| e.reserva_batalla.as_ref().is_some_and(|r| r.battle_id == uuid)), "las escuadras que combaten llevan su candado");
    // gus la ve desde el otro bando; beto, que no combate pero es de F1, la ve resumida por su Facción: sin ticket y sin bando.
    let de_gus = pendiente(&mut gus).await;
    let b = batallas(&foto_de(&de_gus, &format!("BATALLA:{uuid}")).entidades);
    assert_eq!((b[0].id.as_str(), b[0].lado_propio), (uuid.as_str(), Some(pb::LadoId::Defensor as i32)));
    assert!(deltas_de(&de_gus, &format!("HEROE:{h_gus}")).iter().all(|d| batallas(nuevas(d)).is_empty()));
    let de_beto = pendiente(&mut beto).await;
    assert!(de_beto.iter().all(|m| !etiqueta(m).starts_with("foto BATALLA")), "no combate: no tiene su ámbito");
    let b = batallas(nuevas(deltas_de(&de_beto, "FACCION:f-1")[0]));
    assert_eq!((b.len(), b[0].id.as_str(), b[0].lado_propio, b[0].ticket.is_none()), (1, uuid.as_str(), None, true));
    w.parar().await;
}

// ───────────────────────────── 11. Cambiar de Facción ─────────────────────────────

#[tokio::test]
async fn quien_cambia_de_faccion_pierde_la_vieja_con_todo_lo_suyo_y_recibe_solo_lo_de_la_nueva() {
    let m = con_mundo!();
    let mut x = escenario(m);
    let (a, b, h_ana) = (x.a, x.b, ids::heroe(x.ana));
    x.ok("ana", Comando::ColocarOrdenMercado { asentamiento: a, venta: true, recurso: Madera, cantidad: 10_000, precio: None });
    let caravana = ids::caravana(x.caravana("ana", a));
    x.ok("gus", Comando::ColocarOrdenMercado { asentamiento: b, venta: true, recurso: Piedra, cantidad: 10_000, precio: None });
    let w = arrancar_con(x.n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (mut ana, _, mensajes) = w.conectar(hola_con("ana", TipoCliente::ClienteUnity, &[])).await;
    assert!(Cliente::desde(foto_de(&mensajes, "FACCION:f-1")).tiene(TipoEntidad::Caravana, &caravana));

    // La administración la pasa a F2: se le retira la Facción vieja (y la plaza, que ya no es suya) y llega la foto de la nueva, que no trae nada de F1; su héroe sigue siendo suyo.
    let (_, r) = dar_todo(&mut admin_c, "f1", admin("fijar_faccion_de_heroe", &[("heroe_id", &h_ana), ("faccion_id", "f-2")])).await;
    aceptada(&r);
    let recibido = pendiente(&mut ana).await;
    assert_eq!(etiquetas(&recibido)[..3], ["revocado FACCION:f-1".to_owned(), format!("revocado ASENTAMIENTO:{}", ids::asentamiento(a)), "foto FACCION:f-2".to_owned()]);
    let f2 = foto_de(&recibido, "FACCION:f-2");
    assert_eq!(asentamientos(&f2.entidades).iter().map(|p| p.id.clone()).collect::<Vec<_>>(), [ids::asentamiento(b)]);
    assert!(caravanas(&f2.entidades).is_empty(), "la flota de A es de F1");
    assert_eq!(ids_de(&ordenes(&f2.entidades), |o| &o.asentamiento_id), [ids::asentamiento(b)], "solo la orden de la plaza de F2");
    assert!(facciones(&f2.entidades)[0].ciudadanos_ids.contains(&h_ana));
    // Y lo que cambie después en F1 ya no le llega (dora coloca una orden en A), mientras que lo de F2 sí (gus la coloca en B).
    let (mut dora, _, _) = w.conectar(hola_con("dora", TipoCliente::ClienteUnity, &[])).await;
    let (mut gus, _, _) = w.conectar(hola_con("gus", TipoCliente::ClienteUnity, &[])).await;
    let orden_en = |plaza: u64| orden::Cuerpo::ColocarOrdenMercado(pb::OrdenColocarOrdenMercado { asentamiento_id: ids::asentamiento(plaza), tipo: "compra".into(), recurso: "oro".into(), cantidad: 1_000, precio: None });
    aceptada(&dar_todo(&mut dora, "f2", orden_en(a)).await.1);
    assert!(pendiente(&mut ana).await.is_empty(), "lo de F1 ya no es asunto suyo");
    aceptada(&dar_todo(&mut gus, "f3", orden_en(b)).await.1);
    assert_eq!(etiquetas(&pendiente(&mut ana).await), ["delta FACCION:f-2"]);
    w.parar().await;
}

// ───────────────────────────── 12. Coste de llevar las pertenencias ─────────────────────────────

/// Lo que cuesta llevar la pertenencia de cada entidad a los ámbitos de jugador al cerrar cada versión: 500 columnas de siete cuentas, tres mil órdenes (marchar y estacionar) con sus
/// llegadas, con y sin `seguir_pertenencias`. Es el peor caso de la búsqueda del héroe de la sesión (todos los héroes son de campo, así que cada participante recorre los héroes); con héroes de verdad
/// es una consulta. `cargo test --release -p nucleo --test propios coste -- --ignored --nocapture`.
#[test]
#[ignore = "mide; en release"]
fn coste_de_llevar_las_pertenencias() {
    let medir = |con: bool| {
        let mut n = comun::nucleo(60, 60);
        let esc = comun::Escenario { w: 60, h: 60, n: 500, semilla: 3 };
        let ids = esc.crear(&mut n);
        if con {
            n.seguir_pertenencias();
        }
        let t0 = std::time::Instant::now();
        esc.ordenes(&mut n, &ids, 0, 3_000, &|n, t| n.avanzar_hasta(t));
        (t0.elapsed(), n.version(), n.entidades_de(Ambito::Heroe(2)).len())
    };
    let (sin, versiones, _) = medir(false);
    let (con, _, propias) = medir(true);
    eprintln!("{versiones} versiones: {sin:?} sin pertenencias, {con:?} con ellas ({:.1} µs más por versión); el héroe 2 tiene {propias} entidades", (con.as_secs_f64() - sin.as_secs_f64()) * 1e6 / versiones as f64);
}
