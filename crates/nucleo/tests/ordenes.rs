//! Las órdenes del contrato a través de la sesión (4H, T2): una orden real de cada familia por el servidor WebSocket y el actor del núcleo, con su respuesta causal (las `Delta` que la
//! causan llegan antes del `ResultadoOrden`, que lleva lo creado con su clase y su id del contrato). La traducción orden a orden está en `sesiones::ordenes`; aquí se prueba que lo
//! traducido llega al núcleo con la autorización de la sesión.
//!
//! Los escenarios con plazas, campamentos y pueblos van sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`) y se omiten si no está; los de columnas y combate, sobre
//! la retícula sintética y se montan por el propio contrato con la orden de administración.

mod comun;

use base::Punto;
use comun::sesion::*;
use comun::*;
use contrato::v1::{self as pb, orden, TipoCliente, TipoEntidad};
use militar::catalogo::Tropa;
use prost_types::{value::Kind, Struct, Value};

fn texto(x: &str) -> Value {
    Value { kind: Some(Kind::StringValue(x.into())) }
}
fn numero(x: i64) -> Value {
    Value { kind: Some(Kind::NumberValue(x as f64)) }
}
fn punto(p: Punto) -> Value {
    Value { kind: Some(Kind::StructValue(Struct { fields: [("x".to_owned(), numero(p.x)), ("y".to_owned(), numero(p.y))].into() })) }
}

/// Una orden de administración con sus parámetros.
fn admin(codigo: &str, parametros: Vec<(&str, Value)>) -> orden::Cuerpo {
    orden::Cuerpo::Administracion(pb::OrdenDeAdministracion { codigo: codigo.into(), parametros: Some(Struct { fields: parametros.into_iter().map(|(k, v)| (k.to_owned(), v)).collect() }) })
}

fn marchar_a_punto(heroe: &str, p: Punto) -> orden::Cuerpo {
    use pb::orden_marchar_a::{objetivo::Tipo, Objetivo};
    orden::Cuerpo::MarcharA(pb::OrdenMarcharA { heroe_id: heroe.into(), objetivo: Some(Objetivo { tipo: Some(Tipo::Punto(pb::Punto { x: p.x, y: p.y })) }) })
}

fn atacar_ejercito(heroe: &str, objetivo: &str, declara_guerra: Option<bool>) -> orden::Cuerpo {
    use pb::orden_atacar::{objetivo::Tipo, Objetivo};
    orden::Cuerpo::Atacar(pb::OrdenAtacar { heroe_id: heroe.into(), objetivo: Some(Objetivo { tipo: Some(Tipo::EjercitoId(objetivo.into())) }), declara_guerra })
}

/// Lo que la orden creó: `(clase, id)`.
fn creadas(r: &pb::ResultadoOrden) -> Vec<(TipoEntidad, &str)> {
    r.creadas.iter().map(|c| (TipoEntidad::try_from(c.entidad).unwrap(), c.id.as_str())).collect()
}

/// Los ids de las entidades de ejército de una `Delta`.
fn ejercitos_de(d: &pb::Delta) -> Vec<&str> {
    d.cambios.iter().map(|c| ejercito(c.datos.as_ref().unwrap()).id.as_str()).collect()
}

#[tokio::test]
async fn la_administracion_monta_un_mundo_por_el_contrato_y_los_jugadores_lo_juegan_con_sus_ordenes() {
    // Un mundo sin nada, con los diez primeros minutos ya pasados.
    let mut n = nucleo(30, 30);
    n.avanzar_hasta(10 * 60_000);
    let w = arrancar_con(n).await;
    let (mut admin_c, _) = w.entrar("admin", TipoCliente::ClienteAdminWeb).await;
    let (mut ana, f) = w.entrar("ana", TipoCliente::ClienteUnity).await;
    let (mut beto, _) = w.entrar("beto", TipoCliente::ClienteUnity).await;
    assert!(f.entidades.is_empty());

    // Quien no administra no puede (la orden se rechaza antes de llegar al núcleo).
    let crear = |cuenta: &str, faccion: &str, p: Punto| admin("crear_ejercito", vec![("cuenta", texto(cuenta)), ("faccion_id", texto(faccion)), ("posicion", punto(p)), ("velocidad", numero(4_000)), ("viveres", numero(350_000))]);
    let (_, r) = dar(&mut ana, "a0", crear("ana", "f-1", c(10, 10))).await;
    assert_eq!(rechazo(&r), "no_autorizado");

    // La administración crea las dos columnas (cada una con su héroe de campo, que nacen bots) y, a cada una, una escuadra; y pone a las Facciones en guerra.
    let (deltas, r) = dar(&mut admin_c, "m1", crear("ana", "f-1", c(10, 10))).await;
    assert_eq!((creadas(&r), deltas.len()), (vec![(TipoEntidad::Ejercito, "ej-1")], 0), "la administración no ve las columnas de nadie");
    let d = delta(&mut ana).await;
    assert_eq!(ejercitos_de(&d), ["ej-1"], "la dueña recibe su columna");
    let (_, r) = dar(&mut admin_c, "m2", crear("beto", "f-2", Punto::new(c(10, 10).x + 2_000, c(10, 10).y))).await;
    assert_eq!(creadas(&r), [(TipoEntidad::Ejercito, "ej-3")]);
    assert_eq!(ejercitos_de(&delta(&mut beto).await), ["ej-3"]);
    // Las columnas de ana y beto están a 2 u: el mundo programa su encuentro (un hecho, `ejercito.encuentro`) para el instante en que se vean, unos milisegundos de reloj después de crearlas. Llega a los dos
    // por su cuenta, no por una orden; si no se espera aquí, según el reloj se junta con la `Delta` de la orden siguiente o llega suelta y la cuenta de `Delta` de esa orden cambia.
    esperar_hecho(&mut ana, "ejercito.encuentro").await;
    esperar_hecho(&mut beto, "ejercito.encuentro").await;
    for (peticion, heroe) in [("m3", "heroe-2"), ("m4", "heroe-4")] {
        let cantidad = militar::Reglas::cargar().tropas.tropa(Tropa::MiliciaLanceros).unidades;
        let (_, r) = dar(&mut admin_c, peticion, admin("dar_escuadra", vec![("heroe_id", texto(heroe)), ("tropa_id", texto("milicia_lanceros")), ("cantidad", numero(cantidad))])).await;
        assert_eq!(creadas(&r)[0].0, TipoEntidad::Escuadron);
        delta(if heroe == "heroe-2" { &mut ana } else { &mut beto }).await;
    }
    let (_, r) = dar(&mut admin_c, "m5", admin("fijar_relacion", vec![("faccion_a_id", texto("f-1")), ("faccion_b_id", texto("f-2")), ("relacion", texto("guerra"))])).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);

    // Columnas: ana marcha; su plan llega en la Delta que precede al resultado. Beto no puede mandar su columna.
    let (deltas, r) = dar(&mut ana, "p1", marchar_a_punto("heroe-2", Punto::new(c(10, 10).x, c(10, 10).y + 4_000))).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    assert_eq!(deltas.len(), 1);
    assert_eq!(ejercito(deltas[0].cambios[0].datos.as_ref().unwrap()).estado, pb::EstadoColumna::ColumnaMarchando as i32);
    let (_, r) = dar(&mut beto, "b0", marchar_a_punto("heroe-2", c(1, 1))).await;
    assert_eq!(rechazo(&r), "no_autorizado");
    let (deltas, r) = dar(&mut ana, "p2", orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: "ej-1".into() })).await;
    assert_eq!((r.resultado, deltas.len()), (pb::Resultado::OrdenAceptada as i32, 1));
    assert_eq!(ejercito(deltas[0].cambios[0].datos.as_ref().unwrap()).estado, pb::EstadoColumna::ColumnaEstacionada as i32);

    // Combate: ana ataca a beto (las Facciones ya están en guerra, no hace falta declararla); el resultado es del núcleo y beto no manda el héroe de ana.
    let (_, r) = dar(&mut ana, "p3", atacar_ejercito("heroe-2", "ej-3", None)).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    let (_, r) = dar(&mut beto, "b1", atacar_ejercito("heroe-2", "ej-3", None)).await;
    assert_eq!(rechazo(&r), "no_autorizado", "beto no manda el héroe de ana");
    // Un id que no es de ninguna columna se rechaza con el código de la entidad.
    let (_, r) = dar(&mut ana, "p4", atacar_ejercito("heroe-2", "heroe-4", None)).await;
    assert_eq!(rechazo(&r), "ejercito.desconocido");
    let n = w.parar().await;
    // Entre las dos columnas hubo un combate con números: alguien quedó herido o la tropa sufrió bajas.
    assert!(!n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo.starts_with("combate.")).collect::<Vec<_>>().is_empty());
}
