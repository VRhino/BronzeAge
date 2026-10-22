//! Integración: servidor real (WebSocket) + actor del núcleo + `ClientePrueba`.

mod comun;

use base::Punto;
use comun::sesion::*;
use comun::*;
use contrato::v1::{
    self as pb, mensaje_de_cliente as mc, mensaje_de_servidor as ms, orden, orden_marchar_a, DecisionReanudacion, Orden,
    OrdenControlarReloj, OrdenMarcharA, TipoCliente,
};
use nucleo::Nucleo;
use servidor::{
    prueba::{hola, ClientePrueba},
    ComandoSesion, Difusion,
};
use tokio::sync::{mpsc, oneshot};

async fn arrancar() -> Servicio {
    let mut n = nucleo(30, 30);
    n.crear_ejercito(0, "ana", 1, c(1, 1), 4_000, 350_000).unwrap(); // ej-1, con el héroe 2
    n.crear_ejercito(0, "beto", 2, c(10, 10), 4_000, 350_000).unwrap(); // ej-3, con el héroe 4
    arrancar_con(n).await
}

fn marchar_a(peticion: &str, heroe: &str, p: Punto) -> mc::Cuerpo {
    mc::Cuerpo::Orden(Orden {
        peticion_id: peticion.into(),
        cuerpo: Some(orden::Cuerpo::MarcharA(OrdenMarcharA {
            heroe_id: heroe.into(),
            objetivo: Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(pb::Punto { x: p.x, y: p.y })) }),
        })),
        ..Default::default()
    })
}

fn reloj_orden(peticion: &str, accion: &str, factor: Option<i64>) -> mc::Cuerpo {
    mc::Cuerpo::Orden(Orden {
        peticion_id: peticion.into(),
        cuerpo: Some(orden::Cuerpo::ControlarReloj(OrdenControlarReloj { accion: accion.into(), factor_milesimas: factor, hasta_ms: None })),
        ..Default::default()
    })
}

#[tokio::test]
async fn conectar_ordenar_marcha_y_ver_llegar() {
    let w = arrancar().await;
    let (mut ana, b) = ClientePrueba::entrar(w.srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    assert_eq!(b.heroe_id.as_deref(), Some("ana"));
    assert_eq!(b.mundo.unwrap().hash, "hash-de-prueba");
    assert_eq!(b.ambitos[0].decision, DecisionReanudacion::ReanudaConFoto as i32);
    let f = foto(&mut ana).await;
    assert_eq!((f.entidades.len(), f.ambito.unwrap().id.as_str()), (1, "ana"), "solo lo suyo");
    let e = ejercito(&f.entidades[0]);
    assert_eq!((e.id.as_str(), e.estado, e.plan.is_none()), ("ej-1", pb::EstadoColumna::ColumnaEstacionada as i32, true));

    let (mut beto, _) = ClientePrueba::entrar(w.srv.direccion, hola("ok:beto", TipoCliente::ClienteUnity)).await;
    foto(&mut beto).await;
    let (mut admin, _) = ClientePrueba::entrar(w.srv.direccion, hola("ok:admin", TipoCliente::ClienteAdminWeb)).await;
    assert!(foto(&mut admin).await.entidades.is_empty());

    // El administrador acelera el reloj ×600: el resultado y luego el aviso a todos.
    admin.enviar(reloj_orden("r1", "fijar_factor", Some(600_000))).await;
    assert_eq!(resultado(&mut admin).await.resultado, pb::Resultado::OrdenAceptada as i32);
    for c in [&mut admin, &mut ana, &mut beto] {
        match c.esperar().await.cuerpo {
            Some(ms::Cuerpo::Aviso(a)) => assert_eq!((a.codigo.as_str(), a.reloj.unwrap().factor_milesimas), ("reloj.cambiado", 600_000)),
            o => panic!("se esperaba el Aviso, llegó {o:?}"),
        }
    }
    // Un jugador no toca el reloj.
    ana.enviar(reloj_orden("r2", "pausar", None)).await;
    let r = resultado(&mut ana).await;
    assert_eq!((r.resultado, r.rechazo.unwrap().codigo.as_str()), (pb::Resultado::OrdenRechazada as i32, "no_autorizado"));
    // Ni manda columnas ajenas.
    ana.enviar(marchar_a("m0", "heroe-4", c(5, 5))).await;
    assert_eq!(resultado(&mut ana).await.rechazo.unwrap().codigo, "no_autorizado");

    // La orden de marcha: primero la Delta (con el plan), después el resultado, con la causalidad del contrato.
    ana.enviar(marchar_a("m1", "heroe-2", c(9, 1))).await;
    let d = delta(&mut ana).await;
    let r = resultado(&mut ana).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32);
    assert_eq!(d.desde, f.version, "la delta continúa donde acaba la foto");
    assert!(d.hasta <= r.version_tras);
    let e = ejercito(d.cambios[0].datos.as_ref().unwrap());
    assert_eq!(e.estado, pb::EstadoColumna::ColumnaMarchando as i32);
    let plan = e.plan.as_ref().unwrap();
    assert!(plan.vertices.len() >= 2);
    assert_eq!(plan.vertices.last().unwrap().p.as_ref().map(|p| (p.x, p.y)), Some((c(9, 1).x, c(9, 1).y)));

    // Idempotencia: la misma petición no se ejecuta otra vez. Y la llegada llega sola, sin que nadie pida nada (16 s de
    // mundo ≈ 27 ms con el reloj ×600): puede cruzarse con la respuesta, así que se aceptan en cualquier orden.
    ana.enviar(marchar_a("m1", "heroe-2", c(1, 9))).await;
    let (mut dup, mut d2) = (None, None);
    while dup.is_none() || d2.is_none() {
        match ana.esperar().await.cuerpo {
            Some(ms::Cuerpo::Resultado(x)) => dup = Some(x),
            Some(ms::Cuerpo::Delta(x)) => d2 = Some(x),
            o => panic!("llegó {o:?}"),
        }
    }
    let (dup, d2) = (dup.unwrap(), d2.unwrap());
    assert_eq!((dup.resultado, dup.version_tras), (pb::Resultado::OrdenDuplicada as i32, r.version_tras));
    assert_eq!(d2.desde, d.hasta);
    let e = ejercito(d2.cambios[0].datos.as_ref().unwrap());
    assert_eq!((e.estado, e.plan.is_none()), (pb::EstadoColumna::ColumnaEstacionada as i32, true));
    assert_eq!(e.posicion.as_ref().map(|p| (p.x, p.y)), Some((c(9, 1).x, c(9, 1).y)));
    assert_eq!(d2.eventos.len(), 1);
    assert_eq!(d2.eventos[0].codigo, "ejercito.llega");
    assert_eq!(d2.eventos[0].heroes_ids, ["heroe-2"]);

    // Lo de ana no le llega a beto (ni nada en absoluto).
    silencio(&mut beto).await;
    let nucleo = w.parar().await;
    assert_eq!(nucleo.estado().ejercitos[&1].posicion, c(9, 1));
}

#[tokio::test]
async fn reanudacion_sin_novedad_con_deltas_y_con_foto() {
    let w = arrancar().await;
    let (mut admin, _) = ClientePrueba::entrar(w.srv.direccion, hola("ok:admin", TipoCliente::ClienteAdminWeb)).await;
    foto(&mut admin).await;
    let (mut ana, _) = ClientePrueba::entrar(w.srv.direccion, hola("ok:ana", TipoCliente::ClienteUnity)).await;
    let f = foto(&mut ana).await;
    ana.cerrar().await;

    // Cambia el mundo sin tocar nada de ana: beto marcha. Ana, al volver, "sin novedad" aunque la versión global avanzó.
    let (mut beto, _) = ClientePrueba::entrar(w.srv.direccion, hola("ok:beto", TipoCliente::ClienteUnity)).await;
    foto(&mut beto).await;
    beto.enviar(marchar_a("b1", "heroe-4", c(12, 10))).await;
    delta(&mut beto).await;
    assert!(resultado(&mut beto).await.version_tras > f.version);
    let mut h = hola("ok:ana", TipoCliente::ClienteUnity);
    h.versiones_conocidas.insert("HEROE:ana".into(), f.version);
    let (mut ana, b) = ClientePrueba::entrar(w.srv.direccion, h).await;
    assert_eq!(b.ambitos[0].decision, DecisionReanudacion::ReanudaSinNovedad as i32);
    silencio(&mut ana).await;
    ana.cerrar().await;

    // Ahora el administrador manda la columna de ana mientras está fuera: al volver recibe un delta consolidado.
    admin.enviar(marchar_a("a1", "heroe-2", c(3, 1))).await;
    assert_eq!(resultado(&mut admin).await.resultado, pb::Resultado::OrdenAceptada as i32); // admin no ve lo de ana
    let mut h = hola("ok:ana", TipoCliente::ClienteUnity);
    h.versiones_conocidas.insert("HEROE:ana".into(), f.version);
    let (mut ana, b) = ClientePrueba::entrar(w.srv.direccion, h).await;
    assert_eq!(b.ambitos[0].decision, DecisionReanudacion::ReanudaConDeltas as i32);
    let d = delta(&mut ana).await;
    assert_eq!(d.desde, f.version);
    assert_eq!(d.hasta, b.ambitos[0].version_actual);
    assert_eq!(d.cambios.len(), 1);

    // Una versión del futuro (el servidor volvió de una copia vieja) o ninguna: foto.
    let mut h = hola("ok:ana", TipoCliente::ClienteUnity);
    h.versiones_conocidas.insert("HEROE:ana".into(), 999_999);
    let (mut ana, b) = ClientePrueba::entrar(w.srv.direccion, h).await;
    assert_eq!(b.ambitos[0].decision, DecisionReanudacion::ReanudaConFoto as i32);
    foto(&mut ana).await;
    w.parar().await;
}

#[tokio::test]
async fn el_cliente_lento_se_descarta_sin_bloquear_el_mundo() {
    let w = arrancar().await;
    // Sesión directa con un canal de capacidad 1 que nadie lee: la Foto lo llena.
    let (dtx, mut drx) = mpsc::channel(1);
    let (rtx, rrx) = oneshot::channel();
    let h = hola("ok:ana", TipoCliente::ClienteUnity);
    let identidad = servidor::Identidad { cliente_id: "ana".into(), roles: vec![TipoCliente::ClienteUnity] };
    w.tx.send(ComandoSesion::Conecta { sesion: 777, identidad, hola: h, difusion: dtx, respuesta: rtx }).await.unwrap();
    rrx.await.unwrap().expect("bienvenida");

    // El administrador mueve la columna de ana: el actor no puede depositar y la descarta; el mundo sigue.
    let (mut admin, _) = ClientePrueba::entrar(w.srv.direccion, hola("ok:admin", TipoCliente::ClienteAdminWeb)).await;
    foto(&mut admin).await;
    admin.enviar(marchar_a("a1", "heroe-2", c(3, 1))).await;
    assert_eq!(resultado(&mut admin).await.resultado, pb::Resultado::OrdenAceptada as i32);
    admin.enviar(marchar_a("a2", "heroe-2", c(5, 1))).await;
    assert_eq!(resultado(&mut admin).await.resultado, pb::Resultado::OrdenAceptada as i32);

    assert!(matches!(drx.recv().await, Some(Difusion::Foto(_))), "lo que ya estaba en el canal se entrega");
    assert!(drx.recv().await.is_none(), "después, el canal se cierra: la sesión se descartó");
    w.parar().await;
}

#[tokio::test]
async fn lo_que_el_contrato_no_puede_traducir_se_rechaza_con_un_codigo_estable() {
    let w = arrancar().await;
    let (mut ana, _) = w.entrar("ana", TipoCliente::ClienteUnity).await;
    // Una orden del contrato que el núcleo aún no tiene no se descarta: se rechaza con su código.
    let (deltas, r) = dar(&mut ana, "x", orden::Cuerpo::CrearFaccion(pb::OrdenCrearFaccion::default())).await;
    assert_eq!((deltas.len(), rechazo(&r)), (0, "orden.no_disponible"));
    // Una orden vacía, tampoco.
    ana.enviar(mc::Cuerpo::Orden(Orden { peticion_id: "v".into(), ..Default::default() })).await;
    assert_eq!(resultado(&mut ana).await.rechazo.unwrap().codigo, "orden.no_soportada");
    // Un id que no es de ningún héroe, y la orden de administración de quien no administra.
    ana.enviar(marchar_a("y", "nadie", c(1, 1))).await;
    assert_eq!(resultado(&mut ana).await.rechazo.unwrap().codigo, "heroe.desconocido");
    let (_, r) = dar(&mut ana, "z", orden::Cuerpo::Administracion(pb::OrdenDeAdministracion { codigo: "herir_heroe".into(), parametros: None })).await;
    assert_eq!(rechazo(&r), "no_autorizado");
    w.parar().await;
}

// ───────────────────────────── El servidor de batalla ─────────────────────────────

fn de_batalla(cuerpo: pb::mensaje_de_servidor_de_batalla::Cuerpo) -> mc::Cuerpo {
    mc::Cuerpo::Batalla(pb::MensajeDeServidorDeBatalla { cuerpo: Some(cuerpo) })
}

#[tokio::test]
async fn los_mensajes_del_servidor_de_batalla_llegan_por_su_sesion_y_el_nucleo_los_aplica_con_idempotencia() {
    use pb::mensaje_de_servidor_de_batalla::Cuerpo;
    use militar::catalogo::Tropa;
    use nucleo::{Comando, ObjetivoAtaque, Peticion, Relacion};
    // Una batalla abierta a los 10 minutos: ana (humana) ataca a beto (bot).
    let mut n = nucleo_con_batallas();
    let (ana, beto) = (n.crear_ejercito(0, "ana", 1, c(10, 10), 4_000, 350_000).unwrap(), n.crear_ejercito(0, "beto", 2, base::Punto::new(c(10, 10).x + 2_000, c(10, 10).y), 4_000, 350_000).unwrap());
    let (h_ana, h_beto) = (n.ejercito(ana).unwrap().columna.lider, n.ejercito(beto).unwrap().columna.lider);
    for (h, t) in [(h_ana, Tropa::EspadachinesBronce), (h_beto, Tropa::MiliciaLanceros)] {
        let cantidad = militar::Reglas::cargar().tropas.tropa(t).unidades;
        assert!(n.aplicar_en(0, Peticion::nueva("admin", &format!("d{h}"), Comando::DarEscuadra { heroe: h, tropa: t, cantidad }).de_admin()).aceptada());
    }
    assert!(n.aplicar_en(0, Peticion::nueva("admin", "h", Comando::FijarControlador { heroe: h_ana, controlador: militar::heroe::Controlador::Humano }).de_admin()).aceptada());
    assert!(n.aplicar_en(0, Peticion::nueva("admin", "g", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) }).de_admin()).aceptada());
    assert!(n.aplicar_en(10 * 60_000, Peticion::nueva("ana", "a", Comando::Atacar { heroe: h_ana, objetivo: ObjetivoAtaque::Ejercito(beto), declara_guerra: false })).aceptada());
    let id = n.batallas().next().expect("se abrió").id;
    let uuid = n.uuid_de_batalla(id);
    let escuadras: Vec<u64> = n.batalla(id).unwrap().escuadras_de().map(|e| e.id).collect();
    let w = arrancar_con(n).await;
    let (mut svc, _) = ClientePrueba::entrar(w.srv.direccion, hola("svc:srv", TipoCliente::ClienteServidorBatalla)).await;
    foto(&mut svc).await;
    let inicio = |revision| de_batalla(Cuerpo::Inicio(pb::InicioBatalla { schema_version: 1, battle_id: uuid.clone(), ticket_revision: revision, intento_asignacion_id: "a1".into() }));
    let asignacion = de_batalla(Cuerpo::Asignacion(pb::BattleServerAssignment {
        schema_version: 1,
        battle_id: uuid.clone(),
        ticket_revision: 0,
        intento_asignacion_id: "a1".into(),
        instancia: Some(pb::InstanciaDeBatalla { host: "10.0.0.1".into(), puerto: 7777, protocolo: "udp".into() }),
        tokens_participante: vec![pb::TokenParticipante { heroe_id: format!("heroe-{h_ana}"), token: "t".into(), expira_en_ms: 99 }],
    }));
    // Lo rechazado vuelve como un aviso con el código estable: inicio sin asignación, batalla que no es de esta partida, esquema de otra versión.
    svc.enviar(inicio(0)).await;
    assert_eq!(aviso(&mut svc).await.codigo, "batalla.invalida");
    svc.enviar(de_batalla(Cuerpo::Inicio(pb::InicioBatalla { schema_version: 1, battle_id: "00000000-0000-0000-0000-000000000000".into(), ticket_revision: 0, intento_asignacion_id: "a1".into() }))).await;
    assert_eq!(aviso(&mut svc).await.codigo, "batalla.no_existe");
    svc.enviar(de_batalla(Cuerpo::Inicio(pb::InicioBatalla { schema_version: 2, battle_id: uuid.clone(), ticket_revision: 0, intento_asignacion_id: "a1".into() }))).await;
    assert_eq!(aviso(&mut svc).await.codigo, "batalla.invalida");
    // Asignación, inicio y resultado se aplican; repetirlos responde bien y no avisa de nada.
    svc.enviar(asignacion.clone()).await;
    svc.enviar(asignacion).await;
    svc.enviar(inicio(0)).await;
    svc.enviar(inicio(0)).await;
    let resultado = pb::BattleResult {
        schema_version: 1,
        battle_id: uuid.clone(),
        result_id: "res-1".into(),
        ticket_revision: 0,
        intento_asignacion_id: "a1".into(),
        inicio_ms: 1_000,
        fin_ms: 61_000,
        ganador: pb::LadoId::Atacante as i32,
        razon: pb::RazonVictoria::ObjetivosCapturados as i32,
        objetivos: vec![pb::ObjetivoResultado { objetivo_id: "bandera".into(), capturado_por: Some(pb::LadoId::Atacante as i32) }],
        por_escuadra: escuadras.iter().map(|e| pb::ResultadoEscuadra { squad_id: format!("esc-{e}"), desplegados: 5, supervivientes_al_cierre: 5, muertos: 0, xp_ganada: 2 }).collect(),
        por_heroe: [h_ana, h_beto].iter().map(|h| pb::ResultadoHeroe { heroe_id: format!("heroe-{h}"), participo: true, sobrevivio_al_cierre: true, xp_ganada: 3, derrotado: None, botin: None }).collect(),
        version_servidor: "v1".into(),
        carros_perdidos: vec![],
    };
    // Con una escuadra que no existe, el resultado se rechaza entero; con unas cifras que no cierran, también.
    let mut mal = resultado.clone();
    mal.por_escuadra[0].squad_id = "cualquiera".into();
    svc.enviar(de_batalla(Cuerpo::Resultado(mal))).await;
    assert_eq!(aviso(&mut svc).await.codigo, "batalla.invalida");
    svc.enviar(de_batalla(Cuerpo::Resultado(resultado.clone()))).await; // las cifras (5 + 0 ≠ lo autorizado) no cierran
    assert_eq!(aviso(&mut svc).await.codigo, "batalla.invalida");
    silencio(&mut svc).await;
    let n = w.parar().await;
    let b = n.batalla(id).unwrap();
    assert_eq!((b.estado, b.asignacion.as_ref().unwrap().servidor.as_str()), (combate::ciclo::EstadoBatalla::EnCurso, "srv"), "el servidor es el de la sesión");
    assert!(b.iniciada_en.is_some() && b.resultado.is_none());
    assert_eq!((n.metricas().ordenes_aceptadas, n.metricas().ordenes_duplicadas), (7 + 2, 2), "las siete preparatorias y los dos mensajes que cambiaron algo; los repetidos son duplicados");
}

fn nucleo_con_batallas() -> Nucleo {
    let mut a = arranque(reticula(30, 30));
    a.reglas.batallas_en_unity = true;
    Nucleo::nuevo(a)
}
