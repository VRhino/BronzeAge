//! El cliente de ejemplo (4H T7; la guía de `contratos/README.md`, «Cómo se escribe un cliente»): una partida corta jugada **solo por el contrato**, por el servidor WebSocket real y con
//! `servidor::prueba::ClienteDeEjemplo`, que guarda lo que le llega en el almacén de referencia (`contrato::almacen`). Un jugador nace en un campamento, compra en su mercado, sale con su columna,
//! explora bajo la niebla, funda una plaza, construye, recluta, sale con su tropa y ataca a unos bandidos, lo que abre una batalla de Unity. La administración (otra sesión del contrato) pone el
//! andamiaje que aún no tiene orden de jugador (la Facción, la Caravana de Fundación, la residencia, los recursos, los bandidos y el reloj, parado y adelantado a saltos: determinista). Al final
//! lo que cada uno acumuló es lo que daría una conexión nueva, y reanudar con ello no trae nada.
//!
//! Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); se omite si no está.

mod comun;

use base::{Mil, Punto};
use comun::real::{self, campamento, en_tierra, sitio_rico};
use comun::sesion::arrancar_con;
use contrato::almacen::Almacen;
use contrato::v1::{self as pb, entidad::Cuerpo, orden::Cuerpo as O, TipoCliente, TipoEntidad};
use nucleo::{Ajustes, Arranque, Nucleo, Reglas};
use prost_types::{value::Kind, Struct, Value};
use servidor::prueba::{hola, ClienteDeEjemplo};
use std::sync::Arc;

const U: Mil = 1_000;

fn arranque(m: &real::M) -> Arranque {
    let mut reglas = Reglas { batallas_en_unity: true, ..Reglas::default() };
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    Arranque { reglas, ajustes: Ajustes::default(), semilla: 11, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn texto(s: &str) -> Kind {
    Kind::StringValue(s.into())
}

fn numero(n: i64) -> Kind {
    Kind::NumberValue(n as f64)
}

fn punto(p: Punto) -> Option<pb::Punto> {
    Some(pb::Punto { x: p.x, y: p.y })
}

/// Una orden de administración (`OrdenDeAdministracion{codigo, parametros}`): el andamiaje hasta los sistemas 5 y 6.
fn admin(codigo: &str, campos: Vec<(&str, Kind)>) -> O {
    let fields = campos.into_iter().map(|(k, v)| (k.to_owned(), Value { kind: Some(v) })).collect();
    O::Administracion(pb::OrdenDeAdministracion { codigo: codigo.into(), parametros: Some(Struct { fields }) })
}

fn reloj(accion: &str, hasta_ms: Option<i64>) -> O {
    O::ControlarReloj(pb::OrdenControlarReloj { accion: accion.into(), hasta_ms, ..Default::default() })
}

/// El id del contrato de lo que la orden creó, de la clase pedida.
fn creada(r: &pb::ResultadoOrden, tipo: TipoEntidad) -> String {
    r.creadas.iter().find(|c| c.entidad == tipo as i32).unwrap_or_else(|| panic!("la orden no creó {tipo:?}: {r:?}")).id.clone()
}

/// Una entidad de un ámbito del almacén.
fn entidad<'a>(a: &'a Almacen, ambito: &str, tipo: TipoEntidad, id: &str) -> Option<&'a Cuerpo> {
    a.ambitos.get(ambito)?.entidades.get(&(tipo as i32, id.to_owned()))
}

fn columna<'a>(a: &'a Almacen, heroe: &str, id: &str) -> &'a pb::Ejercito {
    match entidad(a, &format!("HEROE:{heroe}"), TipoEntidad::Ejercito, id) {
        Some(Cuerpo::Ejercito(e)) => e,
        otro => panic!("la columna {id} en HEROE:{heroe}: {otro:?}"),
    }
}

/// Las celdas exploradas de la niebla de una `VISTA` (bits a 1).
fn explorado(a: &Almacen, ambito: &str) -> u32 {
    a.ambitos[ambito].niebla.as_ref().expect("la vista lleva su niebla").celdas.iter().map(|b| b.count_ones()).sum()
}

/// Lleva el reloj de mundo (parado) hasta el final del plan de la columna y pone al día al jugador.
async fn llegar(adm: &mut ClienteDeEjemplo, ana: &mut ClienteDeEjemplo, heroe: &str, col: &str) {
    let fin = columna(&ana.almacen, heroe, col).plan.as_ref().and_then(|p| p.vertices.last()).expect("la columna marcha").en_ms;
    adm.aceptada(reloj("avanzar_hasta", Some(fin + 1_000))).await;
    ana.al_dia().await;
}

#[tokio::test]
async fn un_cliente_que_solo_habla_el_contrato_nace_comercia_explora_funda_construye_recluta_y_abre_una_batalla() {
    let Some(m) = real::mundo() else {
        eprintln!("sin artefacto de mundo real: se omite");
        return;
    };
    let n = Nucleo::nuevo(arranque(m));
    let (camp0, sitio) = (campamento(&m.0, 0), sitio_rico(&n, &m.0, 0, 1));
    let w = arrancar_con(n).await;
    let dir = w.srv.direccion;
    let (min, max) = (pb::Punto { x: 0, y: 0 }, pb::Punto { x: 3_000 * U, y: 3_000 * U });

    // La administración para el reloj: todo lo que pasa en el mundo lo deciden las órdenes y los saltos de reloj.
    let mut adm = ClienteDeEjemplo::entrar(dir, hola("ok:adm", TipoCliente::ClienteAdminWeb), Almacen::default()).await;
    adm.aceptada(reloj("pausar", None)).await;

    // 1. Sin héroe, la `Bienvenida` dice dónde puede nacer; nace en el campamento 0 y abre su `HEROE` y el `CAMPAMENTO` donde está.
    let mut ana = ClienteDeEjemplo::entrar(dir, hola("ok:ana", TipoCliente::ClienteUnity), Almacen::default()).await;
    assert!(ana.bienvenida.heroe_id.is_none() && ana.almacen.ambitos.is_empty());
    let camp = ana.bienvenida.campamentos_para_elegir.iter().find(|c| c.campamento_id == "camp-0").expect("el campamento 0 para nacer").campamento_id.clone();
    let r = ana.aceptada(O::CrearHeroe(pb::OrdenCrearHeroe { display_name: "Ana".into(), campamento_id: camp.clone(), ..Default::default() })).await;
    let heroe = creada(&r, TipoEntidad::Heroe);
    assert!(ana.almacen.ambitos.contains_key(&format!("HEROE:{heroe}")) && ana.almacen.ambitos.contains_key(&format!("CAMPAMENTO:{camp}")), "{:?}", ana.almacen.ambitos.keys());

    // 2. Comercia: compra madera en el mercado de su campamento con el oro de su almacén personal.
    adm.aceptada(admin("dar_almacen_personal", vec![("heroe_id", texto(&heroe)), ("recurso", texto("oro")), ("cantidad", numero(50_000))])).await;
    ana.al_dia().await;
    ana.aceptada(O::ComprarEnCampamento(pb::OrdenComprarEnCampamento { recurso: "madera".into(), cantidad: 5_000, campamento_id: None })).await;
    let Some(Cuerpo::Heroe(h)) = entidad(&ana.almacen, &format!("HEROE:{heroe}"), TipoEntidad::Heroe, &heroe) else { panic!("su héroe") };
    assert!(h.almacen_personal.as_ref().unwrap().por_id.get("madera").is_some_and(|&x| x > 0), "la madera comprada, en su almacén personal: {:?}", h.almacen_personal);

    // 3. Crea su Facción (es su primer ciudadano y su Rey), mira el mapa bajo la niebla y sale del campamento con su columna.
    let r = ana.aceptada(O::CrearFaccion(pb::OrdenCrearFaccion { nombre: "Micenas".into(), sigilo: None })).await;
    assert_eq!(creada(&r, TipoEntidad::Faccion), "f-1");
    ana.al_dia().await;
    assert!(ana.almacen.ambitos.contains_key("FACCION:f-1"));
    ana.mirar(min, max).await;
    let antes = explorado(&ana.almacen, "VISTA:f-1");
    let r = ana.aceptada(O::SalirDelCampamento(pb::OrdenSalirDelCampamento { campamento_id: camp.clone(), heroe_id: heroe.clone(), ..Default::default() })).await;
    let col = creada(&r, TipoEntidad::Ejercito);
    assert!(!ana.almacen.ambitos.contains_key(&format!("CAMPAMENTO:{camp}")), "fuera del campamento, su ámbito se revoca");

    // 4. Explora: marcha hasta un buen sitio; lo andado se explora y la niebla llega parcial.
    adm.aceptada(admin("dar_caravana_fundacion", vec![("ejercito_id", texto(&col))])).await;
    let marcha = |p: Punto| O::MarcharA(pb::OrdenMarcharA { heroe_id: heroe.clone(), objetivo: Some(pb::orden_marchar_a::Objetivo { tipo: Some(pb::orden_marchar_a::objetivo::Tipo::Punto(punto(p).unwrap())) }) });
    ana.aceptada(marcha(sitio)).await;
    llegar(&mut adm, &mut ana, &heroe, &col).await;
    assert!(explorado(&ana.almacen, "VISTA:f-1") > antes, "lo andado se explora");

    // 5. Funda (el fundador pasa a residir en la plaza) y entra: se abre el interior de su plaza.
    let r = ana.aceptada(O::Fundar(pb::OrdenFundar {})).await;
    let plaza = creada(&r, TipoEntidad::Asentamiento);
    for (recurso, cantidad) in [("madera", 300_000), ("piedra", 100_000), ("oro", 100_000), ("trigo", 200_000)] {
        adm.aceptada(admin("dar_recursos", vec![("asentamiento_id", texto(&plaza)), ("recurso", texto(recurso)), ("cantidad", numero(cantidad))])).await;
    }
    ana.al_dia().await;
    ana.aceptada(O::EntrarEnAsentamiento(pb::OrdenEntrarEnAsentamiento { asentamiento_id: plaza.clone(), heroe_id: heroe.clone() })).await;
    let interior = format!("ASENTAMIENTO:{plaza}");
    assert!(ana.almacen.ambitos.contains_key(&interior), "dentro de su plaza, su interior");

    // 6. Construye: las obras las da el Gobernador (o el Maestro de Obras), residente y dentro (5E). Ana fundó la plaza: es su Gobernadora de oficio (5E-3) y reside en ella (Q4); la obra entra
    //    en la cola de la plaza con el id que devuelve el resultado.
    let Some(Cuerpo::Asentamiento(a)) = entidad(&ana.almacen, &interior, TipoEntidad::Asentamiento, &plaza) else { panic!("la plaza") };
    assert_eq!(a.cargos.as_ref().and_then(|c| c.gobernador_id.clone()), Some(heroe.clone()), "quien funda es su Gobernadora de oficio");
    let r = ana.aceptada(O::AnadirEdificioManualmente(pb::OrdenAnadirEdificioManualmente { asentamiento_id: plaza.clone(), cargo: "gobernador".into(), tipo: "vivienda".into() })).await;
    let vivienda = creada(&r, TipoEntidad::Edificio);
    let Some(Cuerpo::Asentamiento(a)) = entidad(&ana.almacen, &interior, TipoEntidad::Asentamiento, &plaza) else { panic!("la plaza") };
    assert!(a.edificios.iter().any(|e| e.id == vivienda), "la vivienda está en la plaza que tiene el cliente");

    // 7. Un día después (la plaza crece: el interior llega por su secuencia propia), recluta y sale al mundo con su tropa: el interior se revoca y el héroe se queda con la foto de lo que dejó.
    let (fundada, secuencia) = (a.fundado_en_ms, a.version_interior);
    adm.aceptada(reloj("avanzar_hasta", Some(fundada + 24 * 60 * 60 * 1_000))).await;
    ana.al_dia().await;
    let Some(Cuerpo::Asentamiento(a)) = entidad(&ana.almacen, &interior, TipoEntidad::Asentamiento, &plaza) else { panic!() };
    assert!(a.version_interior > secuencia, "los pasos de minuto llegan como cambios parciales del interior");
    let r = ana.aceptada(O::ReclutarTropa(pb::OrdenReclutarTropa { asentamiento_id: plaza.clone(), heroe_id: heroe.clone(), tropa_id: "milicia_lanceros".into(), origen: None })).await;
    let escuadra = creada(&r, TipoEntidad::Escuadron);
    let r = ana.aceptada(O::SalirAlMundo(pb::OrdenSalirAlMundo { asentamiento_id: plaza.clone(), heroe_id: heroe.clone(), escuadron_ids: vec![escuadra.clone()], carga: None })).await;
    let col = creada(&r, TipoEntidad::Ejercito);
    assert!(!ana.almacen.ambitos.contains_key(&interior));
    assert_eq!(columna(&ana.almacen, &heroe, &col).escuadron_ids, [escuadra]);
    let Some(Cuerpo::Heroe(h)) = entidad(&ana.almacen, &format!("HEROE:{heroe}"), TipoEntidad::Heroe, &heroe) else { panic!() };
    assert!(h.plazas_recordadas.get(&plaza).is_some_and(|x| x.almacen.contains_key("madera")), "recuerda el interior de su plaza");

    // 8. Ataca a unos bandidos fuera de toda zona (los pone la administración): con servidores de batalla, la batalla se abre y llega por su ámbito `BATALLA`.
    let lejos = Punto::new(sitio.x + (sitio.x - camp0.x).signum() * 45 * U, sitio.y);
    let destino = en_tierra(&m.0, lejos, 0, 0);
    ana.aceptada(marcha(destino)).await;
    llegar(&mut adm, &mut ana, &heroe, &col).await;
    let r = adm.aceptada(admin("colocar_bandidos", vec![("posicion", Kind::StructValue(Struct { fields: [("x", destino.x + U), ("y", destino.y)].into_iter().map(|(k, v)| (k.to_owned(), Value { kind: Some(numero(v)) })).collect() })), ("nivel", numero(1))])).await;
    let bandidos = creada(&r, TipoEntidad::CampamentoBandidos);
    ana.al_dia().await;
    let ataque = O::Atacar(pb::OrdenAtacar { heroe_id: heroe.clone(), objetivo: Some(pb::orden_atacar::Objetivo { tipo: Some(pb::orden_atacar::objetivo::Tipo::CampamentoId(bandidos)) }), declara_guerra: None });
    ana.aceptada(ataque).await;
    let batalla = ana.almacen.ambitos.keys().find(|k| k.starts_with("BATALLA:")).expect("la batalla abierta, en su ámbito").clone();
    assert!(ana.almacen.ambitos[&batalla].entidades.values().any(|c| matches!(c, Cuerpo::Batalla(_))));
    assert!(ana.almacen.hechos.iter().any(|(_, e)| e.codigo == "evento_pve.abierta" && matches!(e.tipado, Some(pb::evento_dominio::Tipado::BatallaAbierta(_)))), "con su hecho tipado");

    // 9. Lo acumulado es lo que daría una conexión nueva; y quien reanuda con lo que conserva queda igual y no recibe ni un hecho más.
    for (cliente, token, tipo) in [(ana, "ok:ana", TipoCliente::ClienteUnity), (adm, "ok:adm", TipoCliente::ClienteAdminWeb)] {
        let mut cliente = cliente;
        cliente.al_dia().await;
        let (acumulado, conserva) = (cliente.almacen.contenido(), cliente.almacen.clone());
        let con_vista = conserva.ambitos.keys().any(|k| k.starts_with("VISTA:"));
        cliente.sesion.cerrar().await;
        let mut nueva = ClienteDeEjemplo::entrar(dir, hola(token, tipo), Almacen::default()).await;
        if con_vista {
            nueva.mirar(min, max).await;
        }
        assert_eq!(nueva.almacen.contenido(), acumulado, "{token}: foto + deltas = foto nueva");
        nueva.sesion.cerrar().await;
        let hechos = conserva.hechos.len();
        let mut reanuda = ClienteDeEjemplo::entrar(dir, conserva.hola(token, tipo), conserva).await;
        if con_vista {
            reanuda.mirar(min, max).await;
        }
        assert_eq!(reanuda.almacen.contenido(), acumulado, "{token}: reanudar no cambia nada");
        assert_eq!(reanuda.almacen.hechos.len(), hechos, "{token}: ni repite un hecho");
        reanuda.sesion.cerrar().await;
    }
    w.parar().await;
}
