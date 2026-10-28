//! Los hechos de dominio (4H T5; contrato 05 §8, Proyecciones_Y_Autorizacion §6.4): a quién se cuenta cada hecho, que nadie oiga lo que su niebla oculta, la crónica pública, los quince
//! mensajes tipados y que todo eso no cambie la versión ni el hash del mundo.
//!
//! La primera mitad corre en el núcleo sobre la retícula sintética de `comun`; la segunda, por el servidor WebSocket real y el actor del núcleo sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`, se omite si no está), con el reloj **parado**: el tiempo solo anda cuando la administración lo avanza.

mod comun;

use base::{Instante, Punto, MS_POR_MINUTO};
use comun::sesion::*;
use comun::c;
use contrato::v1::{self as pb, evento_dominio::Tipado, mensaje_de_servidor as ms, orden, TipoCliente};
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::proyeccion::eventos::evento_dominio;
use nucleo::proyeccion::ids;
use nucleo::{AccionReloj, Ajustes, Arranque, Comando, Dato, EventoNucleo, Nucleo, ObjetivoAtaque, Peticion, Relacion, Reglas, Resultado};
use prost::Message;
use prost_types::{value::Kind, Struct, Value};
use rutas::Rutas;
use servidor::prueba::{hola, ClientePrueba};
use std::sync::{Arc, OnceLock};

const MIN: Instante = MS_POR_MINUTO;

fn admin(id: &str, comando: Comando) -> Peticion {
    Peticion::nueva("admin", id, comando).de_admin()
}

/// Una columna de la cuenta en `p`: devuelve (columna, héroe).
fn columna(n: &mut Nucleo, cuenta: &str, faccion: u32, p: Punto) -> (u64, u64) {
    let id = n.crear_ejercito(n.ahora(), cuenta, faccion, p, 500, 350_000).unwrap();
    (id, n.ejercito(id).unwrap().columna.lider)
}

fn dar(n: &mut Nucleo, id: &str, heroe: u64, tropa: Tropa) {
    let unidades = militar::Reglas::cargar().tropas.tropa(tropa).unidades;
    assert!(n.aplicar_en(n.ahora(), admin(id, Comando::DarEscuadra { heroe, tropa, cantidad: unidades })).aceptada());
}

/// Los hechos del registro con el código dado.
fn hechos<'a>(n: &'a Nucleo, codigo: &str) -> Vec<&'a EventoNucleo> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).collect()
}

fn textos(v: &Value) -> Vec<String> {
    match &v.kind {
        Some(Kind::ListValue(l)) => l.values.iter().map(|x| if let Some(Kind::StringValue(s)) = &x.kind { s.clone() } else { panic!("{x:?}") }).collect(),
        otro => panic!("se esperaba una lista: {otro:?}"),
    }
}

// ───────────────────────────── 1. En el núcleo: a quién se cuenta, la crónica y los tipados ─────────────────────────────

/// Ana (F1) ataca la columna de beto (F2) declarando la guerra: la guerra la oyen las dos Facciones enteras; el combate y el encuentro, los héroes de las dos columnas y, el encuentro, los
/// dos hogares; la crónica la oyen todos y no nombra a nadie ni dice dónde. Gus (F3) no está en nada de lo que no es público.
#[test]
fn al_declarar_la_guerra_atacando_la_oyen_los_dos_bandos_y_la_cronica_todos() {
    let mut n = comun::nucleo(40, 5);
    let (ana, beto) = (columna(&mut n, "ana", 1, c(10, 2)), columna(&mut n, "beto", 2, Punto::new(c(10, 2).x + 2_000, c(10, 2).y)));
    let gus = columna(&mut n, "gus", 3, c(30, 2));
    dar(&mut n, "d1", ana.1, Tropa::ArquerosCompuesto);
    let r = n.aplicar_en(MIN, Peticion::nueva("ana", "a1", Comando::Atacar { heroe: ana.1, objetivo: ObjetivoAtaque::Ejercito(beto.0), declara_guerra: true }));
    assert!(r.aceptada(), "{r:?}");

    let guerra = hechos(&n, "diplomacia.guerra_declarada");
    assert_eq!((guerra.len(), guerra[0].facciones.clone(), guerra[0].heroes.is_empty(), guerra[0].publico), (1, vec![1, 2], true, false));
    let resuelto = hechos(&n, "combate.resuelto")[0];
    assert_eq!((resuelto.heroes.clone(), resuelto.facciones.clone()), (vec![ana.1, beto.1], vec![]), "el combate lo oyen las dos columnas");
    let encuentro = hechos(&n, "combate.encuentro");
    assert_eq!(encuentro.len(), 1, "Combate §5, paso 5: el cruce se narra a los dos hogares");
    let e = encuentro[0];
    assert_eq!((e.ejercitos.clone(), e.heroes.clone(), e.facciones.clone()), (vec![ana.0, beto.0], vec![ana.1, beto.1], vec![1, 2]));
    let ganador = if resuelto.dato("ganaElAtacante") == Some(1) { ana.0 } else { beto.0 };
    assert_eq!((e.dato("ejercitoA"), e.dato("ejercitoB"), e.dato("ganador")), (Some(ana.0 as i64), Some(beto.0 as i64), Some(ganador as i64)));
    assert!([guerra[0], resuelto, e].iter().all(|h| !h.heroes.contains(&gus.1) && !h.facciones.contains(&3)), "gus no está en nada de esto");

    let cronica = hechos(&n, "cronica.entrada");
    assert_eq!(cronica.len(), 1, "la guerra se canta");
    let ce = cronica[0];
    assert!(ce.publico && ce.heroes.is_empty() && ce.facciones.is_empty() && ce.asentamientos.is_empty() && ce.ejercitos.is_empty() && ce.punto.is_none());
    let d = evento_dominio(&n, ce, 1);
    let campos = &d.payload.as_ref().unwrap().fields;
    assert_eq!((&campos["tipo"].kind, textos(&campos["faccionIds"])), (&Some(Kind::StringValue("diplomacia.guerra_declarada".into())), vec!["f-1".to_owned(), "f-2".to_owned()]));
    assert!(d.asentamiento_id.is_none() && d.heroes_ids.is_empty() && d.facciones_ids.is_empty() && d.tipado.is_none());

    // La guerra, tipada, dice lo mismo que su payload.
    let g = evento_dominio(&n, guerra[0], 2);
    assert_eq!(g.facciones_ids, ["f-1", "f-2"]);
    assert_eq!(g.tipado, Some(Tipado::GuerraDeclarada(pb::GuerraDeclarada { atacante_faccion_id: "f-1".into(), contra_faccion_id: "f-2".into(), arrastradas_ids: vec![] })));
}

/// Con servidores de batalla el mismo ataque abre una batalla: `batalla.abierta` lleva su `battleId` del contrato en el payload, y tipado el contexto del ticket, quién la inició y los dos bandos.
/// Lo oyen las columnas bloqueadas y las dos Facciones.
#[test]
fn la_batalla_abierta_se_tipa_con_el_contexto_de_su_ticket_y_la_oyen_los_dos_bandos() {
    let mut a = comun::arranque(comun::reticula(40, 5));
    a.reglas.batallas_en_unity = true;
    let mut n = Nucleo::nuevo(a);
    let (ana, beto) = (columna(&mut n, "ana", 1, c(10, 2)), columna(&mut n, "beto", 2, Punto::new(c(10, 2).x + 2_000, c(10, 2).y)));
    dar(&mut n, "d1", ana.1, Tropa::EspadachinesBronce);
    dar(&mut n, "d2", beto.1, Tropa::MiliciaLanceros);
    assert!(n.aplicar_en(n.ahora(), admin("humano", Comando::FijarControlador { heroe: ana.1, controlador: Controlador::Humano })).aceptada());
    assert!(n.aplicar_en(n.ahora(), admin("guerra", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) })).aceptada());
    let r = n.aplicar_en(MIN, Peticion::nueva("ana", "a1", Comando::Atacar { heroe: ana.1, objetivo: ObjetivoAtaque::Ejercito(beto.0), declara_guerra: false }));
    assert!(r.aceptada(), "{r:?}");
    let abierta = hechos(&n, "batalla.abierta")[0];
    assert_eq!((abierta.heroes.clone(), abierta.facciones.clone()), (vec![ana.1, beto.1], vec![1, 2]));
    let b = abierta.dato("batalla").unwrap() as u64;
    let d = evento_dominio(&n, abierta, 5);
    assert_eq!(d.payload.as_ref().unwrap().fields["battleId"].kind, Some(Kind::StringValue(n.uuid_de_batalla(b))));
    let Some(Tipado::BatallaAbierta(t)) = d.tipado else { panic!("{:?}", d.tipado) };
    assert_eq!((t.battle_id, t.iniciada_por, t.atacante_faccion_id, t.defensor_faccion_id), (n.uuid_de_batalla(b), Some(ids::heroe(ana.1)), Some("f-1".into()), Some("f-2".into())));
    assert!(matches!(t.contexto.and_then(|c| c.tipo), Some(pb::contexto_estrategico::Tipo::CampoAbierto(_))));
}

/// Un hecho de prueba con sus datos enteros, a una plaza.
fn hecho(codigo: &'static str, plaza: Option<u64>, columna: Option<u64>, datos: Vec<(&'static str, i64)>) -> EventoNucleo {
    EventoNucleo { asentamientos: plaza.into_iter().collect(), ejercitos: columna.into_iter().collect(), ..EventoNucleo::nuevo(codigo, 1_000, datos) }
}

/// El payload genérico lleva los ids del contrato, con la clave terminada en `Id`, como los tipados (decisión del autor, 2026-10-24); lo que no nombra una entidad, tal cual.
#[test]
fn el_payload_generico_lleva_los_ids_del_contrato() {
    let n = comun::nucleo(5, 5);
    let campo = |ev: &EventoNucleo, k: &str| nucleo::proyeccion::eventos::evento_dominio(&n, ev, 1).payload.unwrap().fields.get(k).and_then(|v| v.kind.clone());
    let texto = |s: &str| Some(Kind::StringValue(s.into()));
    let ev = hecho("tropas.reclutadas", Some(3), Some(7), vec![("heroe", 5), ("escuadra", 9), ("reclutados", 12), ("faccion", 2)]);
    assert_eq!((campo(&ev, "heroeId"), campo(&ev, "escuadraId"), campo(&ev, "faccionId"), campo(&ev, "ejercitoId")), (texto("heroe-5"), texto("esc-9"), texto("f-2"), texto("ej-7")));
    assert_eq!((campo(&ev, "heroe"), campo(&ev, "reclutados")), (None, Some(Kind::NumberValue(12.0))), "sin el número suelto; lo que no es un id, tal cual");
    // Un campamento es de bandidos en sus hechos y de mercenarios en los demás; un edificio, de la plaza del hecho.
    assert_eq!(campo(&hecho("bandidos.campamento_aparece", None, None, vec![("campamento", 4)]), "campamentoId"), texto("band-4"));
    assert_eq!(campo(&hecho("heroe.creado", None, None, vec![("campamento", 4)]), "campamentoId"), texto("camp-4"));
    assert_eq!(campo(&hecho("construccion.edificio_completado", Some(3), None, vec![("edificio", 8)]), "edificioId"), texto("ed-3-8"));
    assert_eq!(campo(&hecho("batalla.asignada", None, None, vec![("batalla", 4)]), "battleId"), Some(Kind::StringValue(n.uuid_de_batalla(4))));
    // El lugar de quien sale expulsado, por su clase; la parcela de bosque; las dos Facciones de una guerra; los bandidos no son una Facción.
    let expulsado = hecho("heroe.expulsado", Some(3), None, vec![("lugar", 2), ("sitio", 6)]);
    assert_eq!((campo(&expulsado, "sitioId"), campo(&expulsado, "lugar")), (texto("av-6"), Some(Kind::NumberValue(2.0))));
    assert_eq!(campo(&hecho("parcela.reclamada", Some(3), None, vec![("parcela", 12)]), "parcelaId"), texto("parcela-12"));
    assert_eq!(campo(&hecho("diplomacia.guerra_declarada", None, None, vec![("atacante", 1), ("contra", 2)]), "atacanteId"), texto("f-1"));
    assert_eq!(campo(&hecho("batalla.abierta", None, None, vec![("faccionAtacante", nucleo::SIN_BANDERA as i64)]), "faccionAtacanteId"), texto(""), "sin bandera, ninguna Facción");
}

/// Cada uno de los quince mensajes tipados sale de su código (los de batalla también con el prefijo `evento_pve.`) con lo que dicen sus datos traducido a ids del contrato, y el
/// `EventoDominio` entero vuelve igual de sus bytes. Un hecho que no mueve la interfaz no lleva tipado.
#[test]
fn los_quince_mensajes_tipados_dicen_lo_que_su_payload_y_van_y_vuelven_en_bytes() {
    let n = comun::nucleo(5, 5);
    use pb::{LadoId, ObraTerminada as O};
    let casos: Vec<(EventoNucleo, Tipado)> = vec![
        (hecho("evento_pve.asignada", None, None, vec![("batalla", 4)]), Tipado::BatallaAsignada(pb::BatallaAsignada { battle_id: n.uuid_de_batalla(4) })),
        (
            hecho("batalla.aplicada", Some(3), None, vec![("batalla", 4), ("ganaElAtacante", 0)]),
            Tipado::BatallaCerrada(pb::BatallaCerrada { battle_id: n.uuid_de_batalla(4), estado: pb::EstadoBatalla::BatallaAplicada as i32, ganador: Some(LadoId::Defensor as i32), razon: None }),
        ),
        (hecho("batalla.cancelada", None, None, vec![("batalla", 4)]), Tipado::BatallaCerrada(pb::BatallaCerrada { battle_id: n.uuid_de_batalla(4), estado: pb::EstadoBatalla::BatallaCancelada as i32, ganador: None, razon: None })),
        (
            hecho("asentamiento.conquistada", Some(3), None, vec![("faccion", 2), ("faccionAnterior", 1)]),
            Tipado::PlazaConquistada(pb::PlazaConquistada { asentamiento_id: "as-3".into(), atacante_faccion_id: Some("f-2".into()), defensor_faccion_id: Some("f-1".into()) }),
        ),
        (hecho("asentamiento.ruinas", Some(3), None, vec![("faltantes", 1 << 1)]), Tipado::PlazaEnRuinas(pb::PlazaEnRuinas { asentamiento_id: "as-3".into(), recursos_faltantes: vec![asentamiento::recurso::TODOS[1].nombre().into()] })),
        (hecho("combate.asedio_resistido", Some(3), Some(7), vec![("faccionAtacante", 2)]), Tipado::AsedioResistido(pb::AsedioResistido { asentamiento_id: "as-3".into(), atacante_faccion_id: Some("f-2".into()) })),
        (
            hecho("columna.union_pedida", None, Some(7), vec![("heroe", 9), ("expiraEn", 5_000)]),
            Tipado::UnionPedida(pb::UnionPedida { destino_id: "ej-7".into(), destino_es_convocatoria: false, solicitante_id: "heroe-9".into(), expira_en_ms: 5_000 }),
        ),
        (
            hecho("convocatoria.union_pedida", Some(3), None, vec![("convocatoria", 8), ("heroe", 9), ("expiraEn", 5_000)]),
            Tipado::UnionPedida(pb::UnionPedida { destino_id: "conv-8".into(), destino_es_convocatoria: true, solicitante_id: "heroe-9".into(), expira_en_ms: 5_000 }),
        ),
        (
            hecho("convocatoria.convocada", None, None, vec![("convocatoria", 8), ("lider", 9), ("campamento", 2)]),
            Tipado::ConvocatoriaConvocada(pb::ConvocatoriaConvocada { convocatoria_id: "conv-8".into(), lugar_id: "camp-2".into(), lugar_es_campamento: true, lider_id: "heroe-9".into() }),
        ),
        (hecho("convocatoria.partio", Some(3), Some(7), vec![("convocatoria", 8)]), Tipado::ConvocatoriaPartio(pb::ConvocatoriaPartio { convocatoria_id: "conv-8".into(), ejercito_id: "ej-7".into() })),
        (
            hecho("heroe.expulsado", Some(3), None, vec![("heroe", 9), ("lugar", 1), ("sitio", 5), ("ejercito", 7), ("motivo", 0)]),
            Tipado::HeroeExpulsado(pb::HeroeExpulsado { heroe_id: "heroe-9".into(), lugar_id: "pueblo-5".into(), motivo: "puerta_cerrada".into(), ejercito_id: Some("ej-7".into()) }),
        ),
        (
            hecho("heroe.expulsado", Some(3), None, vec![("heroe", 9), ("lugar", 2), ("sitio", 6), ("motivo", 3)]),
            Tipado::HeroeExpulsado(pb::HeroeExpulsado { heroe_id: "heroe-9".into(), lugar_id: "av-6".into(), motivo: "destruido".into(), ejercito_id: None }),
        ),
        (hecho("heroe.desalojado", Some(3), None, vec![("heroe", 9)]), Tipado::HeroeExpulsado(pb::HeroeExpulsado { heroe_id: "heroe-9".into(), lugar_id: "as-3".into(), motivo: "desalojo".into(), ejercito_id: None })),
        (hecho("ejercito.sin_viveres", None, Some(7), vec![]), Tipado::ColumnaConHambre(pb::ColumnaConHambre { ejercito_id: "ej-7".into(), sin_viveres: true })),
        (hecho("ejercito.viveres_bajos", None, Some(7), vec![("viveres", 10)]), Tipado::ColumnaConHambre(pb::ColumnaConHambre { ejercito_id: "ej-7".into(), sin_viveres: false })),
        (hecho("tropas.desercion", Some(3), Some(7), vec![("heroe", 9), ("escuadra", 11), ("desertores", 2)]), Tipado::DesercionPorHambre(pb::DesercionPorHambre { heroe_id: "heroe-9".into(), escuadra_id: "esc-11".into(), desertores: 2 })),
        (hecho("construccion.edificio_completado", Some(3), None, vec![("edificio", 4), ("tipo", 1), ("nivel", 1)]), Tipado::ObraTerminada(O { asentamiento_id: "as-3".into(), clase: "edificio".into(), objeto_id: "ed-3-4".into(), nivel: Some(1) })),
        (hecho("construccion.mejora_completada", Some(3), None, vec![("edificio", 4), ("nivel", 2)]), Tipado::ObraTerminada(O { asentamiento_id: "as-3".into(), clase: "mejora".into(), objeto_id: "ed-3-4".into(), nivel: Some(2) })),
        (hecho("construccion.recinto_completado", Some(3), None, vec![("recinto", 1), ("nivel", 3)]), Tipado::ObraTerminada(O { asentamiento_id: "as-3".into(), clase: "recinto".into(), objeto_id: "recinto-3-1".into(), nivel: Some(3) })),
        (hecho("pueblo.completado", Some(3), None, vec![("pueblo", 5)]), Tipado::ObraTerminada(O { asentamiento_id: "as-3".into(), clase: "pueblo".into(), objeto_id: "pueblo-5".into(), nivel: None })),
        (hecho("avanzada.completada", Some(3), None, vec![("avanzada", 6), ("etapa", 0)]), Tipado::ObraTerminada(O { asentamiento_id: "as-3".into(), clase: "avanzada".into(), objeto_id: "av-6".into(), nivel: Some(1) })),
        // La batalla abierta se tipa con lo que dice su hecho, aunque la batalla (la 99) no esté en el estado: un asedio de la plaza 3.
        (
            EventoNucleo {
                punto: Some(Punto::new(30_000, 40_000)),
                ..hecho("batalla.abierta", Some(3), None, vec![("batalla", 99), ("contexto", 0), ("plaza", 3), ("faccionAtacante", 2), ("faccionDefensora", 1), ("iniciador", 9), ("convocatoriaHasta", 7_000)])
            },
            Tipado::BatallaAbierta(pb::BatallaAbierta {
                battle_id: n.uuid_de_batalla(99),
                contexto: Some(pb::ContextoEstrategico { tipo: Some(pb::contexto_estrategico::Tipo::Asedio(pb::ContextoAsedio { asentamiento_id: "as-3".into() })) }),
                punto: Some(pb::Punto { x: 30_000, y: 40_000 }),
                iniciada_por: Some("heroe-9".into()),
                atacante_faccion_id: Some("f-2".into()),
                defensor_faccion_id: Some("f-1".into()),
                convocatoria_hasta_ms: Some(7_000),
            }),
        ),
        (hecho("heroe.herido", None, None, vec![("heroe", 9), ("hasta", 9_000)]), Tipado::HeroeHerido(pb::HeroeHerido { heroe_id: "heroe-9".into(), hasta_ms: 9_000 })),
        (hecho("diplomacia.guerra_declarada", None, None, vec![("atacante", 1), ("contra", 2)]), Tipado::GuerraDeclarada(pb::GuerraDeclarada { atacante_faccion_id: "f-1".into(), contra_faccion_id: "f-2".into(), arrastradas_ids: vec![] })),
    ];
    let mut mensajes = std::collections::HashSet::new();
    for (ev, esperado) in casos {
        let d = evento_dominio(&n, &ev, 3);
        assert_eq!(d.tipado.as_ref(), Some(&esperado), "{}", ev.codigo);
        mensajes.insert(std::mem::discriminant(&esperado));
        assert_eq!(pb::EventoDominio::decode(d.encode_to_vec().as_slice()).unwrap(), d, "{} va y vuelve en bytes", ev.codigo);
    }
    assert_eq!(mensajes.len(), 15);
    assert!(evento_dominio(&n, &hecho("ejercito.llega", None, Some(7), vec![]), 3).tipado.is_none());
    assert!(evento_dominio(&n, &hecho("batalla.abierta", None, None, vec![("batalla", 99)]), 3).tipado.is_none(), "un hecho de apertura sin su contexto no se tipa");
    // Los valores que no son un entero viajan como texto y lista.
    let ev = EventoNucleo { valores: vec![("x", Dato::Lista(vec![Dato::Numero(2), Dato::Texto("f-1".into())]))], ..hecho("cronica.entrada", None, None, vec![]) };
    let Some(Kind::ListValue(l)) = &evento_dominio(&n, &ev, 3).payload.unwrap().fields["x"].kind else { panic!() };
    assert_eq!((l.values[0].kind.clone(), l.values[1].kind.clone()), (Some(Kind::NumberValue(2.0)), Some(Kind::StringValue("f-1".into()))));
}

/// Un hecho de prueba con sus datos enteros y sus valores de texto (y de lista).
fn hecho_con(codigo: &'static str, plaza: Option<u64>, datos: Vec<(&'static str, i64)>, valores: Vec<(&'static str, Dato)>) -> EventoNucleo {
    EventoNucleo { valores, ..hecho(codigo, plaza, None, datos) }
}

fn t(s: &str) -> Dato {
    Dato::Texto(s.into())
}

/// 6G: los quince hechos de la tecnología y los Aedas (bloque `<!-- eventos -->` de Tecnologia_Aedas) llevan sus diez mensajes tipados, con los ids del contrato, y vuelven igual de sus bytes;
/// el público de la épica cumplida no dice la tecnología ni la plaza, y el motivo de la épica perdida viaja tal cual (también `unificada`).
#[test]
fn los_hechos_de_la_tecnologia_y_los_aedas_llevan_sus_diez_mensajes_tipados() {
    let n = comun::nucleo(5, 5);
    let epica = |aeda: &str, como: &str, plaza: Option<&str>, tec: Option<&str>, capitulo: u32, capitulos: u32, motivo: Option<&str>| {
        Tipado::EpicaCambia(pb::EpicaCambia {
            aeda_id: aeda.into(),
            faccion_id: "f-1".into(),
            asentamiento_id: plaza.map(Into::into),
            tecnologia_id: tec.map(Into::into),
            como: como.into(),
            capitulo,
            capitulos,
            motivo: motivo.map(Into::into),
        })
    };
    let casos: Vec<(EventoNucleo, Tipado)> = vec![
        (hecho_con("tecnologia.logro", None, vec![("umbral", 300)], vec![("contador", t("extraido.cobre"))]), Tipado::LogroCumplido(pb::LogroCumplido { contador: "extraido.cobre".into(), umbral: 300, en_ms: 1_000 })),
        (hecho_con("era.comienza", None, vec![("orden", 2), ("porPlazo", 1)], vec![("era", t("edad_bronce"))]), Tipado::EraComenzada(pb::EraComenzada { era_id: "edad_bronce".into(), orden: 2, por_plazo: true })),
        (
            hecho_con("tecnologia.aparece", Some(3), vec![("faccion", 1), ("primera", 1)], vec![("tecnologia", t("canteria"))]),
            Tipado::TecnologiaAparecida(pb::TecnologiaAparecida { faccion_id: "f-1".into(), tecnologia_id: "canteria".into(), primera: true }),
        ),
        (
            hecho_con("tecnologia.adoptada", Some(3), vec![("faccion", 1), ("capital", 3)], vec![("tecnologia", t("canteria"))]),
            Tipado::TecnologiaAdoptada(pb::TecnologiaAdoptada { faccion_id: "f-1".into(), capital_id: "as-3".into(), tecnologia_id: "canteria".into() }),
        ),
        (
            hecho_con("aedas.canta_descubrimiento", None, vec![("faccion", 2)], vec![("tecnologia", t("canteria"))]),
            Tipado::DescubrimientoCantado(pb::DescubrimientoCantado { descubridora_faccion_id: "f-2".into(), tecnologia_id: "canteria".into() }),
        ),
        (
            hecho_con("aedas.revela", Some(3), vec![("faccion", 1), ("descubridora", 2)], vec![("tecnologia", t("canteria")), ("faltan", Dato::Lista(vec![Dato::Numero(0), Dato::Numero(2)]))]),
            Tipado::RevelacionDeAeda(pb::RevelacionDeAeda { faccion_id: "f-1".into(), asentamiento_id: "as-3".into(), descubridora_faccion_id: "f-2".into(), tecnologia_id: "canteria".into(), faltan: vec![0, 2] }),
        ),
        (
            hecho_con("aedas.venta", Some(3), vec![("faccion", 1), ("aeda", 4), ("precio", 200_000)], vec![("tecnologia", t("canteria"))]),
            Tipado::CompraAAeda(pb::CompraAAeda { faccion_id: "f-1".into(), asentamiento_id: "as-3".into(), aeda_id: "aeda-4".into(), tecnologia_id: "canteria".into(), precio: 200_000 }),
        ),
        (
            hecho_con("aedas.residente_llega", Some(3), vec![("aedaResidente", 2), ("faccion", 1)], vec![]),
            Tipado::ResidenteCambia(pb::ResidenteCambia { aeda_id: "aeda-r2".into(), faccion_id: "f-1".into(), asentamiento_id: "as-3".into(), llega: true, cupo: None }),
        ),
        (
            hecho_con("aedas.residente_se_va", Some(3), vec![("aedaResidente", 2), ("faccion", 1), ("cupo", 0)], vec![]),
            Tipado::ResidenteCambia(pb::ResidenteCambia { aeda_id: "aeda-r2".into(), faccion_id: "f-1".into(), asentamiento_id: "as-3".into(), llega: false, cupo: Some(0) }),
        ),
        (hecho_con("aedas.epica_empieza", Some(3), vec![("capitulo", 0), ("capitulos", 2), ("aedaResidente", 2), ("faccion", 1)], vec![("tecnologia", t("canteria"))]), epica("aeda-r2", "empieza", Some("as-3"), Some("canteria"), 0, 2, None)),
        (hecho_con("aedas.epica_capitulo", Some(3), vec![("aedaResidente", 2), ("faccion", 1), ("capitulo", 1), ("capitulos", 2)], vec![("tecnologia", t("canteria"))]), epica("aeda-r2", "capitulo", Some("as-3"), Some("canteria"), 1, 2, None)),
        (hecho_con("aedas.epica_tecnologia", Some(3), vec![("aedaResidente", 2), ("faccion", 1), ("capitulo", 2), ("capitulos", 2)], vec![("tecnologia", t("canteria"))]), epica("aeda-r2", "tecnologia", Some("as-3"), Some("canteria"), 2, 2, None)),
        (hecho_con("aedas.epica_cumplida", None, vec![("aedaResidente", 2), ("faccion", 1)], vec![]), epica("aeda-r2", "cumplida", None, None, 0, 0, None)),
        (
            hecho_con("aedas.epica_abandonada", Some(3), vec![("aedaResidente", 2), ("faccion", 1)], vec![("tecnologia", t("canteria")), ("motivo", t("orden"))]),
            epica("aeda-r2", "abandonada", Some("as-3"), Some("canteria"), 0, 0, Some("orden")),
        ),
        // La perdida por conquista o al unificarse va a la Facción, con la plaza en sus datos.
        (
            hecho_con("aedas.epica_abandonada", None, vec![("aedaResidente", 2), ("faccion", 1), ("plaza", 3)], vec![("tecnologia", t("canteria")), ("motivo", t("unificada"))]),
            epica("aeda-r2", "abandonada", Some("as-3"), Some("canteria"), 0, 0, Some("unificada")),
        ),
        (
            EventoNucleo { heroes: vec![9], ..hecho_con("alijo.abierto", None, vec![("heroe", 9), ("cantidad", 5_000), ("cargado", 3_000)], vec![("alijo", t("alijo-veta-3")), ("recurso", t("piedra"))]) },
            Tipado::AlijoAbierto(pb::AlijoAbierto { heroe_id: "heroe-9".into(), alijo_id: "alijo-veta-3".into(), cantidad: 5_000, cargado: 3_000, recurso: "piedra".into() }),
        ),
    ];
    let mut mensajes = std::collections::HashSet::new();
    let mut codigos = std::collections::BTreeSet::new();
    for (ev, esperado) in casos {
        let d = evento_dominio(&n, &ev, 3);
        assert_eq!(d.tipado.as_ref(), Some(&esperado), "{}", ev.codigo);
        mensajes.insert(std::mem::discriminant(&esperado));
        codigos.insert(ev.codigo.to_string());
        assert_eq!(pb::EventoDominio::decode(d.encode_to_vec().as_slice()).unwrap(), d, "{} va y vuelve en bytes", ev.codigo);
    }
    assert_eq!((mensajes.len(), codigos.len()), (10, 15), "los diez mensajes cubren los quince códigos");
    // El payload genérico también nombra al Aeda itinerante y a la descubridora con sus ids del contrato.
    let p = evento_dominio(&n, &hecho_con("aedas.venta", Some(3), vec![("aeda", 4), ("faccion", 1)], vec![]), 3).payload.unwrap();
    assert_eq!(p.fields["aedaId"].kind, Some(Kind::StringValue("aeda-4".into())));
    let p = evento_dominio(&n, &hecho_con("aedas.revela", Some(3), vec![("descubridora", 2)], vec![]), 3).payload.unwrap();
    assert_eq!(p.fields["descubridoraId"].kind, Some(Kind::StringValue("f-2".into())));
}

// ───────────────────────────── 2. Por la sesión: quién oye qué, la niebla y el mundo igual con sesiones y sin ellas ─────────────────────────────

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

/// Un sitio verde con mucho bosque cerca del campamento `i`: una plaza que no cae en ruinas por falta de madera.
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

/// Dos plazas, A de F1 y B de F2; `ana` (F1) dentro de A, `beto` (F1) en el campamento 0, `gus` (F2) dentro de B y `hugo` (F3) con su columna y su Caravana de Fundación en un buen sitio, a punto de fundar.
/// El reloj, parado.
struct Esc {
    n: Nucleo,
    a: u64,
    ana: u64,
    beto: u64,
    gus: u64,
    hugo: u64,
    peticiones: u64,
}

impl Esc {
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
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0];
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(faccion) });
        if let Some(p) = plaza {
            self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(p as u32)) });
            self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(p as u32) });
            // 5E: el primer residente que está dentro gobierna la plaza (sus órdenes de plaza pasan la matriz real).
            if comun::sin_gobernador_dentro(&self.n, p) {
                comun::gobierna(&mut self.n, p, h);
            }
        }
        h
    }
}

fn escenario(m: &M) -> Esc {
    let mut reglas = Reglas::default();
    let b = &mut Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let n = Nucleo::nuevo(Arranque { reglas, ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
    let mut x = Esc { n, a: 0, ana: 0, beto: 0, gus: 0, hugo: 0, peticiones: 0 };
    let mut plazas = Vec::new();
    for (cuenta, faccion, i) in [("fundador_a", 1, 0), ("fundador_b", 2, 3)] {
        let sitio = sitio_rico(&x.n, &m.0, i, faccion);
        let col = x.n.crear_ejercito(x.n.ahora(), cuenta, faccion, sitio, 500, 100_000).unwrap();
        x.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
        let plaza = x.ok(cuenta, Comando::Fundar { ejercito: col }).creadas[0];
        x.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
        plazas.push(plaza);
    }
    x.a = plazas[0];
    x.ana = x.heroe("ana", 1, Some(plazas[0]));
    x.beto = x.heroe("beto", 1, None);
    x.gus = x.heroe("gus", 2, Some(plazas[1]));
    // Hugo nace en el campamento 1, sale con su columna, anda hasta un buen sitio y recibe su Caravana de Fundación.
    x.hugo = x.ok("hugo", Comando::CrearHeroe { cuenta: "hugo".into(), campamento: 1, controlador: Controlador::Humano, nombre: None }).creadas[0];
    let hugo = x.hugo;
    x.ok("admin", Comando::FijarFaccionDeHeroe { heroe: hugo, faccion: Some(3) });
    let col = x.ok("hugo", Comando::SalirDelCampamento { campamento: 1, heroe: hugo, escuadras: vec![], carga: vec![], politica: militar::columna::Politica::Rechazar, caravana_fundacion: None }).creadas[0];
    let sitio = sitio_rico(&x.n, &m.0, 1, 3);
    x.ok("hugo", Comando::MarcharA { ejercito: col, destino: sitio });
    let llega = x.n.ejercito(col).unwrap().plan.fin().unwrap();
    x.n.avanzar_hasta(llega);
    x.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
    let t = x.n.ahora();
    x.n.avanzar_hasta((t / MIN + 1) * MIN + 1_000);
    x.ok("admin", Comando::Reloj(AccionReloj::Pausar));
    x
}

fn ambito(a: &Option<pb::Ambito>) -> String {
    let a = a.as_ref().expect("un ámbito");
    format!("{}:{}", pb::AmbitoTipo::try_from(a.tipo).unwrap().as_str_name().trim_start_matches("AMBITO_"), a.id)
}

/// Los hechos que llegaron, con el ámbito de la `Delta` que los traía: `("FACCION:f-1", "construccion.auto_alternada")`.
fn oidos(mensajes: &[ms::Cuerpo]) -> Vec<(String, String)> {
    mensajes.iter().filter_map(|m| if let ms::Cuerpo::Delta(d) = m { Some(d) } else { None }).flat_map(|d| d.eventos.iter().map(move |e| (ambito(&d.ambito), e.codigo.clone()))).collect()
}

fn hechos_de(mensajes: &[ms::Cuerpo]) -> Vec<&pb::EventoDominio> {
    mensajes.iter().filter_map(|m| if let ms::Cuerpo::Delta(d) = m { Some(d) } else { None }).flat_map(|d| d.eventos.iter()).collect()
}

/// Lo que ya estaba en camino hacia la sesión (una orden rechazada por el núcleo sale después de todo lo anterior).
async fn pendiente(c: &mut ClientePrueba) -> Vec<ms::Cuerpo> {
    let (previos, r) = dar_todo(c, "barrera", orden::Cuerpo::EstacionarEjercito(pb::OrdenEstacionarEjercito { ejercito_id: "ej-999999".into() })).await;
    assert_eq!(rechazo(&r), "ejercito.desconocido");
    previos
}

fn texto(x: &str) -> Value {
    Value { kind: Some(Kind::StringValue(x.into())) }
}

type Paso = (&'static str, &'static str, orden::Cuerpo, Comando);

/// Los pasos de las pruebas por la sesión: (petición, cuenta, orden del contrato, comando del núcleo).
fn pasos(x: &Esc) -> Vec<Paso> {
    let relacion = [("faccion_a_id", "f-1"), ("faccion_b_id", "f-2"), ("relacion", "alianza")];
    let admin = pb::OrdenDeAdministracion { codigo: "fijar_relacion".into(), parametros: Some(Struct { fields: relacion.iter().map(|(k, v)| ((*k).to_owned(), texto(v))).collect() }) };
    let convocar = pb::OrdenConvocarEjercito { heroe_id: ids::heroe(x.beto), politica_de_union: "aceptar".into(), escuadron_ids: vec![], carga: None };
    vec![
        ("p1", "ana", orden::Cuerpo::AlternarAutoConstruccion(pb::OrdenAlternarAutoConstruccion { asentamiento_id: ids::asentamiento(x.a), pausada: false }), Comando::AlternarAutoConstruccion { asentamiento: x.a, pausada: false }),
        ("p2", "beto", orden::Cuerpo::Desconectarse(pb::OrdenDesconectarse { heroe_id: ids::heroe(x.beto) }), Comando::Desconectarse { heroe: x.beto }),
        ("p3", "admin", orden::Cuerpo::Administracion(admin), Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Alianza) }),
        ("p4", "beto", orden::Cuerpo::Conectarse(pb::OrdenConectarse { heroe_id: ids::heroe(x.beto) }), Comando::Conectarse { heroe: x.beto }),
        ("p5", "beto", orden::Cuerpo::ConvocarEjercito(convocar), Comando::ConvocarEjercito { heroe: x.beto, escuadras: vec![], carga: vec![], politica: militar::columna::Politica::Aceptar }),
        ("p6", "hugo", orden::Cuerpo::Fundar(pb::OrdenFundar {}), Comando::Fundar { ejercito: x.n.estado().ejercito_de_heroe(x.hugo).unwrap() }),
    ]
}

/// Lo que oye cada sesión tras cada paso.
type Oido = (&'static str, &'static str, Vec<ms::Cuerpo>);

/// Cada hecho llega una sola vez a cada sesión que lo oye, por el ámbito que le toca: el de una plaza, a quien está dentro por su `ASENTAMIENTO` y al resto de su Facción por `FACCION`; el que
/// nombra a un héroe, por su `HEROE`; el de una Facción entera (una relación, una convocatoria), por `FACCION`; el público (la crónica de la fundación), a todos por su `HEROE`; y la administración
/// los oye todos por `ADMIN`. Nadie oye lo que no le toca.
#[tokio::test]
async fn cada_hecho_llega_una_sola_vez_a_quien_lo_oye_y_por_su_ambito() {
    let m = con_mundo!();
    let x = escenario(m);
    let (a, ana, beto, gus, hugo) = (x.a, ids::heroe(x.ana), ids::heroe(x.beto), ids::heroe(x.gus), ids::heroe(x.hugo));
    let pasos = pasos(&x);
    let w = arrancar_con(x.n).await;
    let mut clientes: Vec<(&'static str, ClientePrueba)> = Vec::new();
    for (cuenta, tipo) in [("ana", TipoCliente::ClienteUnity), ("beto", TipoCliente::ClienteBot), ("gus", TipoCliente::ClienteUnity), ("hugo", TipoCliente::ClienteBot), ("admin", TipoCliente::ClienteAdminWeb)] {
        let (c, _, _) = w.conectar(hola(&format!("ok:{cuenta}"), tipo)).await;
        clientes.push((cuenta, c));
    }
    let mut todos: Vec<Oido> = Vec::new();
    for (peticion, cuenta, cuerpo, _) in pasos {
        // Primero la orden; después, lo que le llegó a cada uno (también al que la dio).
        let (_, c) = clientes.iter_mut().find(|(q, _)| *q == cuenta).unwrap();
        let (previos, r) = dar_todo(c, peticion, cuerpo).await;
        assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{peticion}: {r:?}");
        for (quien, c) in clientes.iter_mut() {
            let mut mensajes = if *quien == cuenta { previos.clone() } else { Vec::new() };
            mensajes.extend(pendiente(c).await);
            todos.push((peticion, quien, mensajes));
        }
    }
    // Por qué ámbitos oyó `quien` el hecho `codigo` en el paso `paso`.
    let de = |paso: &str, quien: &str, codigo: &str| -> Vec<String> {
        todos.iter().filter(|(p, q, _)| *p == paso && *q == quien).flat_map(|(_, _, m)| oidos(m)).filter(|(_, c)| c == codigo).map(|(a, _)| a).collect()
    };
    // Un hecho de la plaza A: dentro, por `ASENTAMIENTO`; el resto de F1, por `FACCION`; F2 y F3, nada.
    let codigo = "construccion.auto_alternada";
    assert_eq!(de("p1", "ana", codigo), [format!("ASENTAMIENTO:{}", ids::asentamiento(a))]);
    assert_eq!(de("p1", "beto", codigo), ["FACCION:f-1"]);
    assert!(de("p1", "gus", codigo).is_empty() && de("p1", "hugo", codigo).is_empty());
    assert_eq!(de("p1", "admin", codigo), ["ADMIN:"]);
    // El que nombra a un héroe sin plaza: solo él.
    let codigo = "heroe.se_desconecta";
    assert_eq!(de("p2", "beto", codigo), [format!("HEROE:{beto}")]);
    assert!(de("p2", "ana", codigo).is_empty() && de("p2", "gus", codigo).is_empty() && de("p2", "hugo", codigo).is_empty());
    assert_eq!(de("p2", "admin", codigo), ["ADMIN:"]);
    // El de dos Facciones: las dos enteras, por `FACCION`, aunque ana esté dentro de su plaza; F3, nada.
    let codigo = "diplomacia.relacion_aceptada";
    assert_eq!((de("p3", "ana", codigo), de("p3", "beto", codigo), de("p3", "gus", codigo)), (vec!["FACCION:f-1".to_owned()], vec!["FACCION:f-1".to_owned()], vec!["FACCION:f-2".to_owned()]));
    assert!(de("p3", "hugo", codigo).is_empty());
    // Una convocatoria en un campamento: su Facción entera (el Líder, por su `HEROE`), con el mensaje tipado.
    let codigo = "convocatoria.convocada";
    assert_eq!((de("p5", "beto", codigo), de("p5", "ana", codigo)), (vec![format!("HEROE:{beto}")], vec!["FACCION:f-1".to_owned()]));
    assert!(de("p5", "gus", codigo).is_empty() && de("p5", "hugo", codigo).is_empty());
    let e = todos.iter().filter(|(p, q, _)| *p == "p5" && *q == "ana").flat_map(|(_, _, m)| hechos_de(m)).find(|e| e.codigo == codigo).unwrap();
    let Some(Tipado::ConvocatoriaConvocada(t)) = &e.tipado else { panic!("{e:?}") };
    assert_eq!((t.lugar_id.as_str(), t.lugar_es_campamento, t.lider_id.as_str()), ("camp-0", true, beto.as_str()));
    assert_eq!((e.facciones_ids.clone(), e.heroes_ids.clone(), e.asentamiento_id.clone()), (vec!["f-1".to_owned()], vec![beto.clone()], None));
    // La fundación de hugo: la plaza nueva es de F3 y la oye él; la crónica, todos, cada uno por su `HEROE`.
    assert_eq!(de("p6", "hugo", "fundacion.asentamiento_fundado"), [format!("HEROE:{hugo}")]);
    for (quien, h) in [("ana", &ana), ("beto", &beto), ("gus", &gus), ("hugo", &hugo)] {
        assert_eq!(de("p6", quien, "cronica.entrada"), [format!("HEROE:{h}")], "{quien}");
        assert!(quien == "hugo" || de("p6", quien, "fundacion.asentamiento_fundado").is_empty(), "{quien}");
    }
    assert_eq!(de("p6", "admin", "cronica.entrada"), ["ADMIN:"]);
    // Una sola vez cada hecho a cada sesión; la administración, todos; gus, solo lo de F2 y lo público.
    let suyos = |quien: &str| todos.iter().filter(|(_, q, _)| *q == quien).flat_map(|(_, _, m)| hechos_de(m).into_iter().map(|e| (e.version, e.codigo.clone()))).collect::<Vec<_>>();
    for quien in ["ana", "beto", "gus", "hugo", "admin"] {
        let (todos_suyos, distintos) = (suyos(quien), suyos(quien).into_iter().collect::<std::collections::BTreeSet<_>>());
        assert_eq!(distintos.len(), todos_suyos.len(), "{quien} oyó algo dos veces: {todos_suyos:?}");
    }
    let codigos = |quien: &str| suyos(quien).into_iter().map(|(_, c)| c).collect::<std::collections::BTreeSet<_>>();
    assert!(["ana", "beto", "gus", "hugo"].iter().all(|q| codigos(q).is_subset(&codigos("admin"))), "la administración lo oye todo");
    assert_eq!(codigos("gus"), ["cronica.entrada".to_owned(), "diplomacia.relacion_aceptada".to_owned()].into_iter().collect());
    w.parar().await;
}

/// Los hechos de una plaza los oye también quien está dentro sin ser de su Facción (un visitante, aliado o neutral), por su `HEROE` (decisión del autor, 2026-10-24).
#[tokio::test]
async fn el_visitante_que_esta_dentro_de_una_plaza_oye_sus_hechos() {
    let m = con_mundo!();
    let mut x = escenario(m);
    let (a, gus) = (x.a, x.gus);
    x.ok("admin", Comando::ColocarHeroe { heroe: gus, ubicacion: Ubicacion::Asentamiento(a as u32) });
    let w = arrancar_con(x.n).await;
    let (mut ana_c, _, _) = w.conectar(hola("ok:ana", TipoCliente::ClienteUnity)).await;
    let (mut gus_c, _, _) = w.conectar(hola("ok:gus", TipoCliente::ClienteUnity)).await;
    let (_, r) = dar_todo(&mut ana_c, "p1", orden::Cuerpo::AlternarAutoConstruccion(pb::OrdenAlternarAutoConstruccion { asentamiento_id: ids::asentamiento(a), pausada: false })).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    let oido = oidos(&pendiente(&mut gus_c).await);
    assert_eq!(oido.iter().filter(|(_, c)| c == "construccion.auto_alternada").map(|(a, _)| a.clone()).collect::<Vec<_>>(), [format!("HEROE:{}", ids::heroe(gus))], "{oido:?}");
    w.parar().await;
}

/// La `CambiarVista` de todo el mapa.
fn vista_de_todo() -> contrato::v1::mensaje_de_cliente::Cuerpo {
    contrato::v1::mensaje_de_cliente::Cuerpo::Vista(pb::CambiarVista { rectangulo: Some(pb::Rectangulo { min: Some(pb::Punto { x: 0, y: 0 }), max: Some(pb::Punto { x: 100_000_000, y: 100_000_000 }) }) })
}

/// Lo que la niebla oculta no llega en un hecho: la marcha de ana (con su destino) la oye solo ella, aunque beto (su Facción) vea su columna en la `VISTA`; gus (F2), con la vista sobre todo el
/// mapa, no oye nada de ana ni recibe el destino en ningún mensaje.
#[tokio::test]
async fn lo_que_la_niebla_oculta_no_llega_en_un_hecho() {
    let m = con_mundo!();
    let x = escenario(m);
    let (a, ana) = (x.a, x.ana);
    let w = arrancar_con(x.n).await;
    let (mut ana_c, _, _) = w.conectar(hola("ok:ana", TipoCliente::ClienteUnity)).await;
    let (mut beto_c, _, _) = w.conectar(hola("ok:beto", TipoCliente::ClienteBot)).await;
    let (mut gus_c, _, _) = w.conectar(hola("ok:gus", TipoCliente::ClienteUnity)).await;
    for c in [&mut beto_c, &mut gus_c] {
        c.enviar(vista_de_todo()).await;
        assert!(matches!(c.esperar().await.cuerpo, Some(ms::Cuerpo::Foto(_))), "la foto de la vista");
    }
    let salir = orden::Cuerpo::SalirAlMundo(pb::OrdenSalirAlMundo { asentamiento_id: ids::asentamiento(a), heroe_id: ids::heroe(ana), escuadron_ids: vec![], carga: None });
    let (mut de_ana, r) = dar_todo(&mut ana_c, "s1", salir).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    let destino = m.0.campamentos()[2].posicion.unwrap();
    use pb::orden_marchar_a::{objetivo::Tipo, Objetivo};
    let marchar = orden::Cuerpo::MarcharA(pb::OrdenMarcharA { heroe_id: ids::heroe(ana), objetivo: Some(Objetivo { tipo: Some(Tipo::Punto(pb::Punto { x: destino.x, y: destino.y })) }) });
    let (mas, r) = dar_todo(&mut ana_c, "s2", marchar).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    de_ana.extend(mas);
    let (de_beto, de_gus) = (pendiente(&mut beto_c).await, pendiente(&mut gus_c).await);
    let codigos = |ms: &[ms::Cuerpo]| hechos_de(ms).into_iter().map(|e| e.codigo.clone()).collect::<Vec<_>>();
    let marcha = hechos_de(&de_ana).into_iter().find(|e| e.codigo == "jugador.marcha_fijada").unwrap_or_else(|| panic!("{:?}", codigos(&de_ana)));
    assert_eq!(marcha.payload.as_ref().unwrap().fields["objetivoX"].kind, Some(Kind::NumberValue(destino.x as f64)));
    assert!(codigos(&de_beto).contains(&"jugador.sale_al_mundo".to_owned()), "la salida de su plaza la oye su Facción: {:?}", codigos(&de_beto));
    assert!(!codigos(&de_beto).contains(&"jugador.marcha_fijada".to_owned()), "la marcha es de quien marcha");
    assert!(oidos(&de_beto).iter().all(|(a, _)| a != "VISTA:f-1"), "ningún hecho de F1 va por la vista");
    assert!(hechos_de(&de_gus).is_empty(), "gus no oye nada de ana: {:?}", codigos(&de_gus));
    let bytes: Vec<u8> = de_gus.iter().flat_map(|m| pb::MensajeDeServidor { secuencia: 0, cuerpo: Some(m.clone()), ..Default::default() }.encode_to_vec()).collect();
    assert!(!bytes.windows(9).any(|w| w == b"objetivoX"), "el destino de ana no llega a gus en ningún mensaje");
    w.parar().await;
}

/// Las mismas órdenes con cinco sesiones detrás del servidor y aplicadas al núcleo sin sesión ninguna: la misma versión, el mismo hash y los mismos hechos, cada uno con los mismos héroes,
/// Facciones y plazas (a quién se cuenta se decide al emitirlo, no al repartirlo).
#[tokio::test]
async fn la_version_el_hash_y_los_hechos_son_los_mismos_con_sesiones_y_sin_ellas() {
    let m = con_mundo!();
    let (con, mut sin) = (escenario(m), escenario(m));
    sin.n.seguir_pertenencias();
    let v0 = con.n.version();
    assert_eq!((v0, con.n.hash_estado()), (sin.n.version(), sin.n.hash_estado()), "partida igual");
    let pasos = pasos(&con);
    let w = arrancar_con(con.n).await;
    let mut clientes = std::collections::BTreeMap::new();
    for (cuenta, tipo) in [("ana", TipoCliente::ClienteUnity), ("beto", TipoCliente::ClienteBot), ("gus", TipoCliente::ClienteUnity), ("hugo", TipoCliente::ClienteBot), ("admin", TipoCliente::ClienteAdminWeb)] {
        let (c, _, _) = w.conectar(hola(&format!("ok:{cuenta}"), tipo)).await;
        clientes.insert(cuenta, c);
    }
    for (peticion, cuenta, cuerpo, _) in &pasos {
        let r = dar_todo(clientes.get_mut(cuenta).unwrap(), peticion, cuerpo.clone()).await.1;
        assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    }
    let con = w.parar().await;
    for (peticion, cuenta, _, comando) in pasos {
        let p = Peticion { actor: cuenta.into(), admin: cuenta == "admin", peticion_id: peticion.into(), version_esperada: None, comando };
        let t = sin.n.ahora();
        assert!(sin.n.aplicar_en(t, p).aceptada());
    }
    assert_eq!((con.version(), con.hash_estado()), (sin.n.version(), sin.n.hash_estado()), "las sesiones no cambian el mundo");
    let hechos = |n: &Nucleo| n.cambios_desde(v0).unwrap().flat_map(|c| c.eventos.iter().map(move |e| (c.version, e.clone()))).collect::<Vec<_>>();
    let (h_con, h_sin) = (hechos(&con), hechos(&sin.n));
    assert!(h_con.iter().any(|(_, e)| e.codigo == "cronica.entrada"), "{h_con:?}");
    assert_eq!(h_con, h_sin, "los mismos hechos, a los mismos oyentes");
}
