//! El ciclo de la batalla de Unity en el núcleo (fase 4E; Batalla_Unity_Ciclo, Reloj_Y_Eventos §6): abrir con `atacar`, el ticket congelado, el candado de las
//! escuadras y el bloqueo de las columnas, la convocatoria de un asedio, unirse, los cinco mensajes del servidor de batalla con su idempotencia, los plazos como
//! eventos de prioridad 1, las consecuencias del resultado y la retención de las cerradas. Las pruebas de la primera mitad corren sobre la retícula sintética de
//! `comun` (columnas de administración, bandidos de administración); las de la segunda, sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`) y se omiten si no está.

mod comun;

use base::{Instante, Punto, MS_POR_MINUTO};
use combate::ciclo::{Asignacion, EscuadraDeSalida, EstadoBatalla, Instancia, MotivoSalida, SalidaBatalla, Token};
use combate::resolucion::Lado;
use combate::resultado::{Botin, ObjetoBotin, Razon, Resultado as ResultadoDeBatalla, ResultadoEscuadra, ResultadoHeroe};
use comun::c;
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Monedas};
use nucleo::{rechazos, Ajustes, Arranque, Carga, Comando, Nucleo, ObjetivoAtaque, Peticion, Relacion, Reglas, Resultado, Respuesta, SucesoBatalla};
use std::collections::BTreeSet;

const MIN: Instante = MS_POR_MINUTO;
const HORA: Instante = 60 * MIN;

type Hecho = (Instante, Vec<u64>, Vec<u64>, Vec<(&'static str, i64)>);

fn hechos(n: &Nucleo, codigo: &str) -> Vec<Hecho> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).map(|e| (e.en, e.ejercitos.clone(), e.asentamientos.clone(), e.datos.clone())).collect()
}

fn dato(h: &Hecho, clave: &str) -> i64 {
    h.3.iter().find(|(k, _)| *k == clave).unwrap_or_else(|| panic!("el hecho no lleva «{clave}»: {h:?}")).1
}

fn codigo(r: &Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

fn admin(id: &str, comando: Comando) -> Peticion {
    Peticion::nueva("admin", id, comando).de_admin()
}

fn rastro(n: &Nucleo) -> (u64, [u8; 32], u64, u64) {
    (n.version(), n.hash_estado(), n.estado().rng.estado(), n.estado().siguiente_id)
}

fn completa(tropa: Tropa) -> i64 {
    militar::Reglas::cargar().tropas.tropa(tropa).unidades
}

/// Un núcleo sobre la retícula sintética con servidores de batalla (`batallas_en_unity`).
fn nucleo_unity() -> Nucleo {
    let mut a = comun::arranque(comun::reticula(40, 5));
    a.reglas.batallas_en_unity = true;
    Nucleo::nuevo(a)
}

/// Una columna de la cuenta en `p`: devuelve (columna, héroe).
fn columna(n: &mut Nucleo, cuenta: &str, faccion: u32, p: Punto) -> (u64, u64) {
    let id = n.crear_ejercito(n.ahora(), cuenta, faccion, p, 500, 350_000).unwrap();
    (id, n.ejercito(id).unwrap().columna.lider)
}

fn dar(n: &mut Nucleo, id: &str, heroe: u64, tropa: Tropa) -> u64 {
    let r = n.aplicar_en(n.ahora(), admin(id, Comando::DarEscuadra { heroe, tropa, cantidad: completa(tropa) }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

fn humano(n: &mut Nucleo, heroe: u64) {
    let r = n.aplicar_en(n.ahora(), admin(&format!("humano-{heroe}"), Comando::FijarControlador { heroe, controlador: Controlador::Humano }));
    assert!(r.aceptada(), "{r:?}");
}

fn atacar(n: &mut Nucleo, t: Instante, actor: &str, id: &str, heroe: u64, objetivo: ObjetivoAtaque, guerra: bool) -> Respuesta {
    n.aplicar_en(t, Peticion::nueva(actor, id, Comando::Atacar { heroe, objetivo, declara_guerra: guerra }))
}

/// Un mensaje del servidor de batalla `srv`.
fn msg(n: &mut Nucleo, t: Instante, id: &str, comando: Comando) -> Respuesta {
    n.aplicar_en(t, Peticion::de_servidor_de_batalla("srv", id, comando))
}

fn asignacion(intento: &str, revision: u32, tokens: &[u64]) -> Asignacion {
    Asignacion {
        intento: intento.into(),
        servidor: "ignorado: lo pone el actor".into(),
        revision,
        instancia: Instancia { host: "10.0.0.1".into(), puerto: 7777, protocolo: "udp".into() },
        tokens: tokens.iter().map(|&heroe| Token { heroe, token: format!("t{heroe}"), expira_en: 9_999_999 }).collect(),
    }
}

/// Registra la asignación `a1` y confirma el inicio de la batalla `id` en el instante `t`.
fn en_curso(n: &mut Nucleo, id: u64, t: Instante, humanos: &[u64]) {
    let revision = n.batalla(id).unwrap().ticket.revision;
    assert!(msg(n, t, &format!("as-{id}-{t}"), Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", revision, humanos) }).aceptada());
    assert!(msg(n, t, &format!("in-{id}-{t}"), Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision }).aceptada());
    assert_eq!(n.batalla(id).unwrap().estado, EstadoBatalla::EnCurso);
}

/// Un resultado válido de la batalla `id`: de cada escuadra mueren `muertos` y el resto sobrevive; gana `ganador`; todos los héroes participan.
fn resultado(n: &Nucleo, id: u64, ganador: Lado, muertos: i64) -> ResultadoDeBatalla {
    let b = n.batalla(id).unwrap();
    ResultadoDeBatalla {
        resultado_id: format!("res-{id}"),
        revision: b.ticket.revision,
        intento: "a1".into(),
        inicio: 1_000,
        fin: 61_000,
        ganador,
        razon: Razon::ObjetivosCapturados,
        objetivos: vec![("bandera".into(), Some(ganador))],
        por_escuadra: b
            .escuadras_de()
            .map(|e| {
                let m = muertos.min(e.efectivos);
                ResultadoEscuadra { escuadra: e.id, desplegados: e.efectivos, supervivientes_al_cierre: e.efectivos - m, muertos: m, xp_ganada: 4 }
            })
            .collect(),
        por_heroe: b.heroes_que_combaten().map(|h| ResultadoHeroe { heroe: h, participo: true, sobrevivio_al_cierre: true, xp_ganada: 7, derrotado: None, botin: None }).collect(),
        version_servidor: "v1".into(),
        carros_perdidos: vec![],
    }
}

/// Un mensaje que se rechaza no deja rastro: ni versión, ni hecho, ni tirada, ni id (el reloj sí avanza hasta su instante, y por eso se mide después).
fn rechazado(n: &mut Nucleo, t: Instante, id: &str, comando: Comando) -> &'static str {
    n.avanzar_hasta(t);
    let antes = rastro(n);
    let r = msg(n, t, id, comando);
    assert_eq!(rastro(n), antes, "{r:?}");
    codigo(&r)
}

/// Un mensaje ya aplicado, repetido: responde bien y no cambia nada.
fn repetido(n: &mut Nucleo, t: Instante, id: &str, comando: Comando) {
    n.avanzar_hasta(t);
    let antes = rastro(n);
    assert_eq!(msg(n, t, id, comando).resultado, Resultado::SinCambios);
    assert_eq!(rastro(n), antes, "ni versión, ni hecho, ni tirada, ni id");
}

fn aplicar(n: &mut Nucleo, t: Instante, id: &str, batalla: u64, r: ResultadoDeBatalla) -> Respuesta {
    msg(n, t, id, Comando::AplicarResultado { batalla, resultado: r })
}

fn soldados(n: &Nucleo, heroe: u64, escuadra: u64) -> i64 {
    n.heroe(heroe).unwrap().heroe.escuadra(escuadra).unwrap().cantidad
}

fn candado(n: &Nucleo, heroe: u64, escuadra: u64) -> Option<u64> {
    n.heroe(heroe).unwrap().heroe.escuadra(escuadra).unwrap().reserva_batalla
}

/// Dos columnas de Facciones distintas, a menos de `radioEncuentro` una de otra y en guerra. Ana (F1, humana) lleva espadachines y arqueros; beto (F2, bot), una
/// milicia. Reglas con servidores de batalla.
struct Choque {
    n: Nucleo,
    ana: (u64, u64),
    beto: (u64, u64),
    fuerte: Vec<u64>,
    debil: u64,
}

fn choque() -> Choque {
    let mut n = nucleo_unity();
    let ana = columna(&mut n, "ana", 1, c(10, 2));
    let beto = columna(&mut n, "beto", 2, Punto::new(c(10, 2).x + 2_000, c(10, 2).y));
    let fuerte = vec![dar(&mut n, "d1", ana.1, Tropa::EspadachinesBronce), dar(&mut n, "d2", ana.1, Tropa::ArquerosCompuesto)];
    let debil = dar(&mut n, "d3", beto.1, Tropa::MiliciaLanceros);
    humano(&mut n, ana.1);
    let r = n.aplicar_en(n.ahora(), admin("guerra", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) }));
    assert!(r.aceptada(), "{r:?}");
    Choque { n, ana, beto, fuerte, debil }
}

/// El choque con la batalla ya abierta por ana a los 10 minutos. Devuelve el id de la batalla.
fn batalla_abierta() -> (Choque, u64) {
    let mut x = choque();
    let r = atacar(&mut x.n, 10 * MIN, "ana", "a1", x.ana.1, ObjetivoAtaque::Ejercito(x.beto.0), false);
    assert!(r.aceptada(), "{r:?}");
    let id = *x.n.batallas().map(|b| &b.id).next().expect("se abrió una batalla");
    (x, id)
}

// ───────────────────────────── 1. Abrir ─────────────────────────────

#[test]
fn atacar_con_un_humano_y_servidores_de_batalla_abre_una_batalla_en_lugar_de_resolver_con_numeros() {
    let mut x = choque();
    let (ana, beto) = (x.ana, x.beto);
    let (v, rng0, ids0) = (x.n.version(), x.n.estado().rng.estado(), x.n.estado().siguiente_id);
    let r = atacar(&mut x.n, 10 * MIN, "ana", "a1", ana.1, ObjetivoAtaque::Ejercito(beto.0), false);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(x.n.version(), v + 1, "una orden aceptada sube la versión una vez");
    assert_eq!(x.n.estado().rng.estado(), rng0, "abrir una batalla no gasta una tirada: Unity decide lo de dentro");
    assert!(hechos(&x.n, "combate.resuelto").is_empty(), "no se resolvió con números");

    let b = x.n.batallas().next().unwrap();
    assert_eq!(b.id, ids0, "toma el siguiente id de la partida");
    assert_eq!(x.n.estado().siguiente_id, ids0 + 1, "y ninguno más: las escuadras de un combate campal ya tenían el suyo");
    assert_eq!((b.estado, b.iniciada_por, b.ticket.revision), (EstadoBatalla::Convocando, ana.1, 0));
    assert_eq!((b.convocatoria_hasta, b.expira_en), (None, 10 * MIN + 5 * MIN), "sin convocatoria: el plazo de asignación corre desde ya");
    assert_eq!((b.ticket.duracion_maxima, b.ticket.atacante.capacidad, b.ticket.defensor.capacidad), (15 * MIN, 5, 5));
    assert_eq!((b.ticket.atacante.faccion, b.ticket.defensor.faccion), (Some(1), Some(2)));
    assert_eq!(b.ticket.atacante.participantes.len() + b.ticket.defensor.participantes.len(), 2);
    let a = &b.ticket.atacante.participantes[0];
    assert_eq!((a.heroe, a.controlador, a.casillas_libres), (ana.1, Controlador::Humano, 72));
    assert_eq!(a.escuadras.iter().map(|e| (e.id, e.efectivos, e.dueno)).collect::<Vec<_>>(), x.fuerte.iter().map(|&e| (e, soldados(&x.n, ana.1, e), Some(ana.1))).collect::<Vec<_>>());
    assert_eq!(b.bloqueo.ejercitos, vec![ana.0, beto.0]);
    // Lo que combate queda apartado: columnas fuera del espacio, escuadras con el candado de la batalla, nada que comer.
    let id = b.id;
    for col in [ana.0, beto.0] {
        let e = x.n.ejercito(col).unwrap();
        assert!(e.en_batalla && !x.n.en_el_mapa(col));
        assert_eq!(e.columna.estado, militar::racion::Estado::Apartada);
        assert_eq!(e.columna.viveres.valor_en(100 * MIN), e.columna.viveres.valor_en(10 * MIN), "apartada no come");
    }
    assert_eq!([candado(&x.n, ana.1, x.fuerte[0]), candado(&x.n, ana.1, x.fuerte[1]), candado(&x.n, beto.1, x.debil)], [Some(id); 3]);
    assert_eq!(x.n.batallas_pendientes(), vec![id], "sin convocatoria sale hacia Unity al abrirse");
    assert!(x.n.heroe_en_batalla(ana.1) && x.n.heroe_en_batalla(beto.1));
    let bl = x.n.bloqueos();
    assert_eq!((bl.heroes.len(), bl.ejercitos.len()), (2, 2));
    // El hecho de apertura lo oyen las dos columnas.
    let h = &hechos(&x.n, "batalla.abierta");
    assert_eq!(h.len(), 1);
    assert_eq!((h[0].0, h[0].1.clone(), dato(&h[0], "batalla"), dato(&h[0], "contexto"), dato(&h[0], "heroesAtacante"), dato(&h[0], "heroesDefensor")), (10 * MIN, vec![ana.0, beto.0], id as i64, 1, 1, 1));
    // La batalla programa su vencimiento (prioridad 1) y nada más: ni un evento por entidad ociosa.
    let citas: Vec<_> = x.n.estado().cola.volcar().into_iter().filter(|e| matches!(e.carga, Carga::Batalla(_)) && x.n.estado().generaciones.es_vigente(e.clave)).collect();
    assert_eq!(citas.len(), 1);
    assert_eq!((citas[0].instante, citas[0].prioridad, citas[0].carga.clone()), (15 * MIN, nucleo::PRIO_BATALLA, Carga::Batalla(SucesoBatalla::Vence)));
    // El uuid del contrato es determinista, único por batalla y reversible.
    let uuid = x.n.uuid_de_batalla(id);
    assert_eq!((uuid.len(), x.n.id_de_batalla(&uuid)), (36, Some(id)));
    assert_eq!(x.n.id_de_batalla("00000000-0000-0000-0000-000000000000"), None);
}

#[test]
fn bot_contra_bot_y_sin_servidores_de_batalla_siguen_con_numeros() {
    // Bot contra bot, aunque haya servidores de batalla.
    let mut x = choque();
    let (ana, beto) = (x.ana, x.beto);
    assert!(x.n.aplicar_en(MIN, admin("bot", Comando::FijarControlador { heroe: ana.1, controlador: Controlador::Bot })).aceptada());
    assert!(atacar(&mut x.n, 10 * MIN, "ana", "a1", ana.1, ObjetivoAtaque::Ejercito(beto.0), false).aceptada());
    assert_eq!((x.n.batallas().count(), hechos(&x.n, "combate.resuelto").len()), (0, 1), "se resolvió con números");
    assert!(!x.n.ejercito(ana.0).unwrap().en_batalla);
    // Con un humano pero sin servidores de batalla (el interruptor es una regla de la partida), también.
    let mut n = comun::nucleo(40, 5);
    let (ana, beto) = (columna(&mut n, "ana", 1, c(10, 2)), columna(&mut n, "beto", 2, Punto::new(c(10, 2).x + 2_000, c(10, 2).y)));
    dar(&mut n, "d1", ana.1, Tropa::EspadachinesBronce);
    dar(&mut n, "d2", beto.1, Tropa::MiliciaLanceros);
    humano(&mut n, ana.1);
    assert!(n.aplicar_en(n.ahora(), admin("g", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) })).aceptada());
    assert!(atacar(&mut n, 10 * MIN, "ana", "a1", ana.1, ObjetivoAtaque::Ejercito(beto.0), false).aceptada());
    assert_eq!((n.batallas().count(), hechos(&n, "combate.resuelto").len()), (0, 1));
}

#[test]
fn las_validaciones_del_combate_con_numeros_valen_tambien_para_abrir_y_una_rechazada_no_deja_rastro() {
    let mut x = choque();
    let (ana, beto) = (x.ana, x.beto);
    let lejana = columna(&mut x.n, "hugo", 2, c(30, 2));
    humano(&mut x.n, lejana.1);
    let antes = rastro(&x.n);
    // Sin guerra (la Facción 3 no está en guerra con la 1), lejos y con un herido: nada de eso abre una batalla.
    let tercera = columna(&mut x.n, "gus", 3, Punto::new(c(10, 2).x, c(10, 2).y + 1_000));
    let antes_tercera = rastro(&x.n);
    assert_eq!(codigo(&atacar(&mut x.n, 10 * MIN, "ana", "r1", ana.1, ObjetivoAtaque::Ejercito(tercera.0), false)), rechazos::COMBATE_SIN_GUERRA);
    assert_eq!(codigo(&atacar(&mut x.n, 10 * MIN, "ana", "r2", ana.1, ObjetivoAtaque::Ejercito(lejana.0), true)), rechazos::COMBATE_LEJOS);
    assert!(x.n.aplicar_en(10 * MIN, admin("h", Comando::HerirHeroe { heroe: ana.1 })).aceptada());
    assert_eq!(codigo(&atacar(&mut x.n, 10 * MIN, "ana", "r3", ana.1, ObjetivoAtaque::Ejercito(beto.0), false)), rechazos::HEROE_HERIDO);
    assert_eq!(x.n.batallas().count(), 0);
    assert_eq!(x.n.estado().rng.estado(), antes.2);
    assert_eq!(x.n.estado().siguiente_id, antes_tercera.3, "ninguna rechazada gastó un id");
    assert_eq!(x.n.relacion(1, 3), None, "ni declaró la guerra");
}

// ───────────────────────────── 2. Bloqueo y candado ─────────────────────────────

#[test]
fn lo_que_combate_no_se_mueve_ni_recibe_ordenes_ni_se_ataca_y_la_administracion_si_puede() {
    let (mut x, id) = batalla_abierta();
    let (ana, beto) = (x.ana, x.beto);
    let tercera = columna(&mut x.n, "gus", 3, Punto::new(c(10, 2).x, c(10, 2).y + 1_000));
    dar(&mut x.n, "dg", tercera.1, Tropa::EspadachinesBronce);
    humano(&mut x.n, tercera.1);
    assert!(x.n.aplicar_en(11 * MIN, admin("g3", Comando::FijarRelacion { a: 3, b: 2, relacion: Some(Relacion::Guerra) })).aceptada());
    let antes = rastro(&x.n);
    let con = |x: &mut Choque, actor: &str, i: &str, c: Comando| codigo(&x.n.aplicar_en(11 * MIN, Peticion::nueva(actor, i, c)));
    // Sus héroes y su columna: ni marchar, ni estacionar, ni ninguna orden del héroe.
    assert_eq!(con(&mut x, "ana", "b1", Comando::MarcharA { ejercito: ana.0, destino: c(20, 2) }), rechazos::BATALLA_BLOQUEADO);
    assert_eq!(con(&mut x, "ana", "b2", Comando::Estacionar { ejercito: ana.0 }), rechazos::BATALLA_BLOQUEADO);
    assert_eq!(con(&mut x, "ana", "b3", Comando::PasarAViveres { heroe: ana.1, cantidad: 1_000 }), rechazos::BATALLA_BLOQUEADO);
    assert_eq!(con(&mut x, "beto", "b4", Comando::DejarDePerseguir { heroe: beto.1 }), rechazos::BATALLA_BLOQUEADO);
    // Nadie más la ataca ni la sigue ni la inspecciona: el objetivo está apartado.
    assert_eq!(con(&mut x, "gus", "b5", Comando::Atacar { heroe: tercera.1, objetivo: ObjetivoAtaque::Ejercito(beto.0), declara_guerra: true }), rechazos::BATALLA_BLOQUEADO);
    assert_eq!(con(&mut x, "gus", "b6", Comando::Perseguir { heroe: tercera.1, objetivo: militar::inspeccion::Objetivo::Ejercito(ana.0) }), rechazos::BATALLA_BLOQUEADO);
    assert_eq!(con(&mut x, "gus", "b7", Comando::Inspeccionar { heroe: tercera.1, objetivo: militar::inspeccion::Objetivo::Ejercito(ana.0) }), rechazos::BATALLA_BLOQUEADO);
    assert_eq!(rastro(&x.n), antes, "un rechazo por el candado no deja rastro: ni versión, ni tirada, ni id");
    // Lo que no toca la batalla sigue: la tercera columna marcha, y la administración (andamiaje de escenarios) no lleva candado.
    assert!(x.n.aplicar_en(11 * MIN, Peticion::nueva("gus", "ok1", Comando::MarcharA { ejercito: tercera.0, destino: c(20, 3) })).aceptada());
    assert!(x.n.aplicar_en(11 * MIN, admin("ok2", Comando::DarSuministro { ejercito: ana.0, recurso: asentamiento::Recurso::Trigo, cantidad: 1_000 })).aceptada());
    // Salir del mundo se anota sin candado y no ocurre mientras combate (§5.1): volver a conectarse lo anula.
    assert!(x.n.aplicar_en(11 * MIN, Peticion::nueva("ana", "d1", Comando::Desconectarse { heroe: ana.1 })).aceptada());
    x.n.avanzar_hasta(14 * MIN + 31_000);
    assert!(!x.n.heroe(ana.1).unwrap().heroe.presencia.fuera, "pasó el retardo de 2 min 30 s y sigue en el mundo: combate");
    assert!(x.n.aplicar_en(14 * MIN + 32_000, Peticion::nueva("ana", "c1", Comando::Conectarse { heroe: ana.1 })).aceptada());
    assert!(x.n.batalla(id).unwrap().activa(x.n.ahora()));
}

#[test]
fn cancelar_antes_de_empezar_lo_hace_quien_la_inicio_y_nadie_pierde_nada() {
    let (mut x, id) = batalla_abierta();
    let (ana, beto) = (x.ana, x.beto);
    x.n.avanzar_hasta(11 * MIN);
    let antes = rastro(&x.n);
    let cancela = |x: &mut Choque, t: Instante, actor: &str, i: &str, heroe: u64| x.n.aplicar_en(t, Peticion::nueva(actor, i, Comando::CancelarBatalla { heroe, batalla: id }));
    assert_eq!(codigo(&cancela(&mut x, 11 * MIN, "beto", "c1", beto.1)), rechazos::BATALLA_INVALIDA, "solo quien la inició");
    assert_eq!(codigo(&cancela(&mut x, 11 * MIN, "beto", "c2", ana.1)), rechazos::NO_AUTORIZADO, "ni con el héroe de otro");
    assert_eq!(codigo(&x.n.aplicar_en(11 * MIN, Peticion::nueva("ana", "c3", Comando::CancelarBatalla { heroe: ana.1, batalla: id + 100 }))), rechazos::BATALLA_NO_EXISTE);
    assert_eq!(rastro(&x.n), antes);
    let r = cancela(&mut x, 11 * MIN, "ana", "c4", ana.1);
    assert!(r.aceptada(), "{r:?}");
    let b = x.n.batalla(id).unwrap();
    assert_eq!((b.estado, b.cerrada_en), (EstadoBatalla::Cancelada, Some(11 * MIN)));
    assert_eq!(hechos(&x.n, "batalla.cancelada").len(), 1);
    // Todo vuelve: columnas al espacio, escuadras sin candado, héroes sanos y sin castigo, ni una baja.
    for col in [ana.0, beto.0] {
        assert!(!x.n.ejercito(col).unwrap().en_batalla && x.n.en_el_mapa(col));
        assert_eq!(x.n.ejercito(col).unwrap().columna.estado, militar::racion::Estado::Estacionada);
    }
    assert_eq!([candado(&x.n, ana.1, x.fuerte[0]), candado(&x.n, ana.1, x.fuerte[1]), candado(&x.n, beto.1, x.debil)], [None; 3]);
    assert!(!x.n.heroe_herido(ana.1) && !x.n.heroe_herido(beto.1) && !x.n.heroe_en_batalla(ana.1));
    assert_eq!(codigo(&cancela(&mut x, 12 * MIN, "ana", "c5", ana.1)), rechazos::BATALLA_NO_EXISTE, "una cerrada ya no existe para el contrato");
    // Y se puede atacar de nuevo: la batalla anterior ya no bloquea.
    assert!(atacar(&mut x.n, 12 * MIN, "ana", "a2", ana.1, ObjetivoAtaque::Ejercito(beto.0), false).aceptada());
}

#[test]
fn no_se_cancela_una_partida_que_ya_empezo() {
    let (mut x, id) = batalla_abierta();
    en_curso(&mut x.n, id, 11 * MIN, &[x.ana.1]);
    let r = x.n.aplicar_en(12 * MIN, Peticion::nueva("ana", "c1", Comando::CancelarBatalla { heroe: x.ana.1, batalla: id }));
    assert_eq!(codigo(&r), rechazos::BATALLA_INVALIDA);
    assert_eq!(x.n.batalla(id).unwrap().estado, EstadoBatalla::EnCurso);
}

// ───────────────────────────── 3. Los cinco mensajes ─────────────────────────────

#[test]
fn el_ciclo_completo_con_los_cinco_mensajes_sus_rechazos_y_su_idempotencia() {
    let (mut x, id) = batalla_abierta();
    let (ana, beto) = (x.ana, x.beto);
    // Antes de la asignación no hay token para nadie.
    assert_eq!(x.n.token_de_jugador(ana.1, id), nucleo::TokenDeJugador::SinToken);
    assert_eq!(x.n.token_de_jugador(ana.1, id + 100), nucleo::TokenDeJugador::NoExiste);
    // 1. Asignación: solo con la credencial de un servidor de batalla, con la revisión vigente y con tokens de humanos de esta batalla.
    x.n.avanzar_hasta(11 * MIN);
    let antes = rastro(&x.n);
    let r = x.n.aplicar_en(11 * MIN, Peticion::nueva("ana", "m0", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", 0, &[ana.1]) }));
    assert_eq!(rastro(&x.n), antes);
    assert_eq!(codigo(&r), rechazos::BATALLA_SIN_CREDENCIAL, "ni la sesión de un jugador ni la administración");
    assert_eq!(rechazado(&mut x.n, 11 * MIN, "m1", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", 0, &[beto.1]) }), rechazos::BATALLA_INVALIDA, "los bot no se conectan: no llevan token");
    assert_eq!(rechazado(&mut x.n, 11 * MIN, "m2", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", 3, &[ana.1]) }), rechazos::BATALLA_INVALIDA, "otra revisión del ticket");
    assert_eq!(rechazado(&mut x.n, 11 * MIN, "m3", Comando::RegistrarAsignacion { batalla: id + 100, asignacion: asignacion("a1", 0, &[ana.1]) }), rechazos::BATALLA_NO_EXISTE);
    assert_eq!(rechazado(&mut x.n, 11 * MIN, "m4", Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision: 0 }), rechazos::BATALLA_INVALIDA, "sin asignación no hay inicio");

    let r = msg(&mut x.n, 11 * MIN, "asignar", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", 0, &[ana.1]) });
    assert!(r.aceptada(), "{r:?}");
    let b = x.n.batalla(id).unwrap();
    assert_eq!((b.estado, b.expira_en), (EstadoBatalla::Asignada, 16 * MIN), "plazo de inicio: 5 minutos desde la asignación");
    assert_eq!(b.asignacion.as_ref().unwrap().servidor, "srv", "el servidor lo pone la credencial del actor");
    assert_eq!(hechos(&x.n, "batalla.asignada").len(), 1);
    assert_eq!(x.n.batallas_pendientes(), Vec::<u64>::new(), "ya no está pendiente de asignar");
    match x.n.token_de_jugador(ana.1, id) {
        nucleo::TokenDeJugador::Token { instancia, token, expira_en } => assert_eq!((instancia.puerto, token.as_str(), expira_en), (7777, format!("t{}", ana.1).as_str(), 9_999_999)),
        otro => panic!("{otro:?}"),
    }
    assert_eq!(x.n.token_de_jugador(beto.1, id), nucleo::TokenDeJugador::SinToken, "un bot combate, pero no se conecta: no tiene token");
    assert_eq!(x.n.token_de_jugador(9_999, id), nucleo::TokenDeJugador::NoCombate, "quien no combate en ella no lo lee");
    // Repetir la misma asignación (mismo intento y mismo servidor): responde bien sin cambiar nada.
    repetido(&mut x.n, 11 * MIN + 1_000, "asignar-otra-vez", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", 0, &[ana.1]) });
    // Otro servidor con otro intento: gana la primera.
    x.n.avanzar_hasta(11 * MIN + 2_000);
    let antes = rastro(&x.n);
    let r = x.n.aplicar_en(11 * MIN + 2_000, Peticion::de_servidor_de_batalla("otro", "m5", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a2", 0, &[ana.1]) }));
    assert_eq!((rastro(&x.n), codigo(&r)), (antes, rechazos::BATALLA_YA_ASIGNADA));

    // 2. Inicio: solo el servidor de la asignación activa, con su revisión.
    x.n.avanzar_hasta(12 * MIN);
    let antes = rastro(&x.n);
    let r = x.n.aplicar_en(12 * MIN, Peticion::de_servidor_de_batalla("otro", "m6", Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision: 0 }));
    assert_eq!((rastro(&x.n), codigo(&r)), (antes, rechazos::BATALLA_INVALIDA), "no es su asignación");
    assert_eq!(rechazado(&mut x.n, 12 * MIN, "m7", Comando::ConfirmarInicio { batalla: id, intento: "a2".into(), revision: 0 }), rechazos::BATALLA_INVALIDA, "otro intento");
    assert_eq!(rechazado(&mut x.n, 12 * MIN, "m8", Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision: 1 }), rechazos::BATALLA_INVALIDA, "otra revisión");
    let r = msg(&mut x.n, 12 * MIN, "inicio", Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision: 0 });
    assert!(r.aceptada(), "{r:?}");
    let b = x.n.batalla(id).unwrap();
    assert_eq!((b.estado, b.iniciada_en, b.expira_en), (EstadoBatalla::EnCurso, Some(12 * MIN), 12 * MIN + 15 * MIN + 5 * MIN), "duración máxima más 5 minutos de margen");
    assert_eq!(hechos(&x.n, "batalla.en_curso").len(), 1);
    repetido(&mut x.n, 12 * MIN + 1, "inicio-otra-vez", Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision: 0 });

    // 3. Tokens: solo de humanos, el nuevo sustituye al anterior y no deja hecho.
    let t = |h: u64, token: &str| Token { heroe: h, token: token.into(), expira_en: 1 };
    assert_eq!(rechazado(&mut x.n, 13 * MIN, "m9", Comando::RegistrarTokens { batalla: id, intento: "a1".into(), tokens: vec![t(beto.1, "x")] }), rechazos::BATALLA_INVALIDA);
    assert_eq!(rechazado(&mut x.n, 13 * MIN, "m10", Comando::RegistrarTokens { batalla: id, intento: "a2".into(), tokens: vec![t(ana.1, "x")] }), rechazos::BATALLA_INVALIDA, "solo del servidor de la asignación activa");
    let eventos0 = x.n.cambios_desde(0).unwrap().map(|c| c.eventos.len()).sum::<usize>();
    let r = msg(&mut x.n, 13 * MIN, "tokens", Comando::RegistrarTokens { batalla: id, intento: "a1".into(), tokens: vec![t(ana.1, "nuevo")] });
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(x.n.cambios_desde(0).unwrap().map(|c| c.eventos.len()).sum::<usize>(), eventos0, "sin hecho: que alguien tenga su token no es noticia");
    assert_eq!(x.n.batalla(id).unwrap().asignacion.as_ref().unwrap().tokens, vec![t(ana.1, "nuevo")]);
    repetido(&mut x.n, 13 * MIN, "tokens-otra-vez", Comando::RegistrarTokens { batalla: id, intento: "a1".into(), tokens: vec![t(ana.1, "nuevo")] });

    // 4. Resultado: el checklist entero o nada.
    x.n.avanzar_hasta(14 * MIN);
    let antes = rastro(&x.n);
    let bueno = resultado(&x.n, id, Lado::Atacante, 5);
    let malo = |x: &mut Choque, i: &str, f: &dyn Fn(&mut ResultadoDeBatalla)| {
        let mut r = bueno.clone();
        f(&mut r);
        let r = aplicar(&mut x.n, 14 * MIN, i, id, r);
        assert_eq!(rastro(&x.n), antes, "un resultado rechazado no cambia nada: {r:?}");
        codigo(&r)
    };
    assert_eq!(malo(&mut x, "r1", &|r| r.intento = "a2".into()), rechazos::BATALLA_INVALIDA);
    assert_eq!(malo(&mut x, "r2", &|r| r.revision = 1), rechazos::BATALLA_INVALIDA);
    assert_eq!(malo(&mut x, "r3", &|r| r.por_escuadra.push(r.por_escuadra[0].clone())), rechazos::BATALLA_INVALIDA);
    assert_eq!(malo(&mut x, "r4", &|r| {
        r.por_escuadra.pop();
    }), rechazos::BATALLA_INVALIDA);
    assert_eq!(malo(&mut x, "r5", &|r| {
        r.por_heroe.pop();
    }), rechazos::BATALLA_INVALIDA);
    assert_eq!(malo(&mut x, "r6", &|r| r.por_escuadra[0].muertos += 1), rechazos::BATALLA_INVALIDA, "no cierra sobre lo autorizado");
    assert_eq!(malo(&mut x, "r7", &|r| (r.razon, r.ganador) = (Razon::TiempoAgotado, Lado::Atacante)), rechazos::BATALLA_INVALIDA);
    assert_eq!(malo(&mut x, "r8", &|r| r.fin = r.inicio + 15 * MIN + 1), rechazos::BATALLA_INVALIDA, "dura más que la duración máxima");
    assert_eq!(malo(&mut x, "r9", &|r| r.por_heroe[1].botin = Some(Botin { objetos: vec![], monedas: Monedas { bronce: -1, plata: 0, oro: 0 } })), rechazos::BATALLA_INVALIDA);
    assert_eq!(malo(&mut x, "r10", &|r| r.carros_perdidos = vec!["0".into()]), rechazos::BATALLA_INVALIDA, "no hay carros que perder fuera de una caravana");
    let r = x.n.aplicar_en(14 * MIN, Peticion::nueva("ana", "r11", Comando::AplicarResultado { batalla: id, resultado: bueno.clone() }));
    assert_eq!(codigo(&r), rechazos::BATALLA_SIN_CREDENCIAL);

    let r = aplicar(&mut x.n, 14 * MIN, "resultado", id, bueno.clone());
    assert!(r.aceptada(), "{r:?}");
    let b = x.n.batalla(id).unwrap();
    assert_eq!((b.estado, b.aplicada_en, b.cerrada_en, b.resultado.as_ref()), (EstadoBatalla::Aplicada, Some(14 * MIN), Some(14 * MIN), Some(&bueno)));
    let h = &hechos(&x.n, "batalla.aplicada")[0];
    assert_eq!((h.0, dato(h, "ganaElAtacante")), (14 * MIN, 1));
    // Repetido: bien y sin cambios. Otro distinto: «ya se aplicó con otro resultado».
    repetido(&mut x.n, 15 * MIN, "resultado-otra-vez", Comando::AplicarResultado { batalla: id, resultado: bueno.clone() });
    let mut otro = bueno.clone();
    otro.ganador = Lado::Defensor;
    let antes = rastro(&x.n);
    let r = aplicar(&mut x.n, 15 * MIN, "resultado-otro", id, otro);
    assert_eq!(codigo(&r), rechazos::BATALLA_INVALIDA);
    assert!(matches!(&r.resultado, Resultado::Rechazada { mensaje, .. } if mensaje.contains("otro resultado")), "{r:?}");
    assert_eq!(rastro(&x.n), antes);
    // Una cerrada rechaza todo mensaje salvo ese resultado repetido: 404 en la asignación, el inicio, los tokens y la salida.
    for (i, c) in [
        Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", 0, &[ana.1]) },
        Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision: 0 },
        Comando::RegistrarTokens { batalla: id, intento: "a1".into(), tokens: vec![] },
        Comando::RegistrarSalida { batalla: id, salida: SalidaBatalla { intento: "a1".into(), revision: 0, heroe: ana.1, motivo: MotivoSalida::Abandono, derrotado: false, escuadras: vec![] } },
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(codigo(&msg(&mut x.n, 16 * MIN, &format!("cerrada-{i}"), c)), rechazos::BATALLA_NO_EXISTE);
    }
}

// ───────────────────────────── 4. Aplicar el resultado ─────────────────────────────

/// Juega la batalla entera: cada escuadra pierde `muertos`, gana ana, y `ajusta` toca el resultado antes de mandarlo. Beto lleva un carro con trigo y madera.
fn jugada(muertos: i64, ajusta: &dyn Fn(&Choque, &mut ResultadoDeBatalla)) -> (Choque, u64) {
    let mut x = choque();
    for (id, r, q) in [("s1", asentamiento::Recurso::Trigo, 100_000), ("s2", asentamiento::Recurso::Madera, 41_001)] {
        assert!(x.n.aplicar_en(0, admin(id, Comando::DarSuministro { ejercito: x.beto.0, recurso: r, cantidad: q })).aceptada());
    }
    let (ana, beto) = (x.ana, x.beto);
    assert!(atacar(&mut x.n, 10 * MIN, "ana", "a1", ana.1, ObjetivoAtaque::Ejercito(beto.0), false).aceptada());
    let id = x.n.batallas().next().unwrap().id;
    en_curso(&mut x.n, id, 11 * MIN, &[ana.1]);
    let mut r = resultado(&x.n, id, Lado::Atacante, muertos);
    ajusta(&x, &mut r);
    let resp = aplicar(&mut x.n, 20 * MIN, "res", id, r);
    assert!(resp.aceptada(), "{resp:?}");
    (x, id)
}

#[test]
fn el_resultado_deja_las_bajas_la_experiencia_el_botin_el_carro_y_la_chatarra_y_las_columnas_vuelven_al_mundo() {
    let botin = Botin { objetos: vec![ObjetoBotin { definicion: "espada".into(), cantidad: 1, instancia: Some("i-1".into()) }], monedas: Monedas { bronce: 5, plata: 2, oro: 1 } };
    let (x, id) = jugada(5, &|x, r| r.por_heroe.iter_mut().find(|h| h.heroe == x.ana.1).unwrap().botin = Some(botin.clone()));
    let (ana, beto) = (x.ana, x.beto);
    let bronce0 = militar::Reglas::cargar().heroe.bronce_inicial;
    // Cada escuadra cierra sobre sus supervivientes, con la experiencia que dijo Unity (puntos enteros, guardados en milésimas).
    for (h, e, antes) in [(ana.1, x.fuerte[0], completa(Tropa::EspadachinesBronce)), (ana.1, x.fuerte[1], completa(Tropa::ArquerosCompuesto)), (beto.1, x.debil, completa(Tropa::MiliciaLanceros))] {
        let s = x.n.heroe(h).unwrap().heroe.escuadra(e).unwrap();
        assert_eq!((s.cantidad, s.experiencia, s.reserva_batalla), (antes - 5, 4_000, None), "supervivientes, 4 puntos y sin candado");
    }
    // El héroe suma su experiencia y las monedas del botín; el objeto no tiene dónde caer (CQ-004) y el nivel no sube (CQ-001).
    let h = &x.n.heroe(ana.1).unwrap().heroe;
    assert_eq!((h.experiencia, h.nivel, h.monedas), (7, 1, Monedas { bronce: bronce0 + 5, plata: 2, oro: 1 }));
    // Heridos: el bando que pierde (sin `derrotado`); el que gana, no.
    assert!(x.n.heroe_herido(beto.1) && !x.n.heroe_herido(ana.1));
    assert_eq!(x.n.heroe(beto.1).unwrap().heroe.herido_hasta, Some(20 * MIN + 2 * MIN));
    // El que pierde entrega la mitad (hacia abajo) de su carro y el ganador cobra además la chatarra de los caídos de los dos bandos.
    let carga = |col: u64| x.n.ejercito(col).unwrap().columna.carga.clone();
    assert_eq!((carga(beto.0)[&asentamiento::Recurso::Trigo], carga(beto.0)[&asentamiento::Recurso::Madera]), (50_000, 20_501));
    let ganada = carga(ana.0);
    assert_eq!((ganada[&asentamiento::Recurso::Trigo], ganada[&asentamiento::Recurso::Madera]), (50_000, 20_500));
    let chatarra: i64 = ganada.iter().filter(|(r, _)| matches!(r, asentamiento::Recurso::ChatarraCobre | asentamiento::Recurso::ChatarraBronce | asentamiento::Recurso::ChatarraHierro)).map(|(_, n)| n).sum();
    assert!(chatarra > 0, "los caídos dejan chatarra");
    // Lo apartado vuelve: al espacio, quieto (no había plan), comiendo otra vez.
    for col in [ana.0, beto.0] {
        let e = x.n.ejercito(col).unwrap();
        assert!(!e.en_batalla && x.n.en_el_mapa(col) && e.columna.estado == militar::racion::Estado::Estacionada);
        assert!(e.columna.viveres.valor_en(100 * MIN) < e.columna.viveres.valor_en(21 * MIN), "vuelve a comer");
    }
    assert!(!x.n.heroe_en_batalla(ana.1) && x.n.bloqueos().ejercitos.is_empty());
    assert_eq!(x.n.batalla(id).unwrap().estado, EstadoBatalla::Aplicada);
    assert_eq!(x.n.estado().rng.estado(), base::Rng::derivado(42, "nucleo").estado(), "Unity decide: ni abrir ni aplicar el resultado gastan una tirada");
}

#[test]
fn quien_queda_herido_sale_de_derrotado_y_no_del_bando() {
    // Sin `derrotado` pierde el bando perdedor; `derrotado: false` explícito protege al perdedor; `derrotado: true` hiere aunque su bando ganara.
    let (x, _) = jugada(2, &|x, r| {
        r.por_heroe.iter_mut().find(|h| h.heroe == x.beto.1).unwrap().derrotado = Some(false);
        r.por_heroe.iter_mut().find(|h| h.heroe == x.ana.1).unwrap().derrotado = Some(true);
    });
    assert!(!x.n.heroe_herido(x.beto.1) && x.n.heroe_herido(x.ana.1));
    let (x, _) = jugada(2, &|_, _| {});
    assert!(x.n.heroe_herido(x.beto.1) && !x.n.heroe_herido(x.ana.1));
}

#[test]
fn si_pierde_el_atacante_es_el_quien_entrega_la_mitad_de_su_carro_al_defensor() {
    let mut x = choque();
    let (ana, beto) = (x.ana, x.beto);
    assert!(x.n.aplicar_en(0, admin("s1", Comando::DarSuministro { ejercito: ana.0, recurso: asentamiento::Recurso::Piedra, cantidad: 30_001 })).aceptada());
    assert!(atacar(&mut x.n, 10 * MIN, "ana", "a1", ana.1, ObjetivoAtaque::Ejercito(beto.0), false).aceptada());
    let id = x.n.batallas().next().unwrap().id;
    en_curso(&mut x.n, id, 11 * MIN, &[ana.1]);
    let mut r = resultado(&x.n, id, Lado::Defensor, 0);
    r.razon = Razon::TiempoAgotado;
    assert!(aplicar(&mut x.n, 12 * MIN, "res", id, r).aceptada());
    assert!(x.n.heroe_herido(ana.1) && !x.n.heroe_herido(beto.1), "si se agota el tiempo gana el defensor");
    let carga = |c: u64, r| x.n.ejercito(c).unwrap().columna.carga.get(&r).copied().unwrap_or(0);
    assert_eq!((carga(ana.0, asentamiento::Recurso::Piedra), carga(beto.0, asentamiento::Recurso::Piedra)), (15_001, 15_000));
}

// ───────────────────────────── 5. Los plazos ─────────────────────────────

fn citas_de_batalla(n: &Nucleo) -> Vec<(Instante, u8, SucesoBatalla)> {
    n.estado().cola.volcar().into_iter().filter(|e| n.estado().generaciones.es_vigente(e.clave)).filter_map(|e| if let Carga::Batalla(s) = e.carga { Some((e.instante, e.prioridad, s)) } else { None }).collect()
}

fn libres(x: &Choque) {
    for col in [x.ana.0, x.beto.0] {
        let e = x.n.ejercito(col).unwrap();
        assert!(!e.en_batalla && x.n.en_el_mapa(col), "la columna {col} volvió al mundo");
    }
    assert_eq!([candado(&x.n, x.ana.1, x.fuerte[0]), candado(&x.n, x.ana.1, x.fuerte[1]), candado(&x.n, x.beto.1, x.debil)], [None; 3]);
    assert!(!x.n.heroe_herido(x.ana.1) && !x.n.heroe_herido(x.beto.1), "fallida: nadie gana ni pierde");
}

#[test]
fn una_batalla_que_nadie_asigna_falla_a_los_cinco_minutos_sin_castigo_y_a_su_hora_exacta() {
    let (mut x, id) = batalla_abierta();
    x.n.avanzar_hasta(15 * MIN - 1);
    assert_eq!(x.n.batalla(id).unwrap().estado, EstadoBatalla::Convocando);
    assert!(x.n.heroe_en_batalla(x.ana.1));
    x.n.avanzar_hasta(15 * MIN);
    let b = x.n.batalla(id).unwrap();
    assert_eq!((b.estado, b.cerrada_en), (EstadoBatalla::Fallida, Some(15 * MIN)));
    assert_eq!(hechos(&x.n, "batalla.fallida").len(), 1);
    assert_eq!(hechos(&x.n, "batalla.fallida")[0].0, 15 * MIN);
    libres(&x);
    // Una batalla que vence en un instante no bloquea en ese mismo instante: las órdenes de ese instante ya ven la columna libre.
    assert!(x.n.aplicar_en(15 * MIN, Peticion::nueva("ana", "m", Comando::MarcharA { ejercito: x.ana.0, destino: c(20, 2) })).aceptada());
    // Queda la retirada, a la hora de retención desde el cierre, y nada más.
    assert_eq!(citas_de_batalla(&x.n), vec![(15 * MIN + HORA, nucleo::PRIO_BATALLA, SucesoBatalla::Retira)]);
    assert_eq!(msg(&mut x.n, 16 * MIN, "tarde", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", 0, &[x.ana.1]) }).resultado, Resultado::Rechazada { codigo: rechazos::BATALLA_NO_EXISTE, mensaje: format!("batalla-{id}") });
}

#[test]
fn una_asignada_que_nadie_inicia_y_una_en_curso_sin_resultado_fallan_cuando_vence_su_plazo() {
    let (mut x, id) = batalla_abierta();
    let revision = x.n.batalla(id).unwrap().ticket.revision;
    assert!(msg(&mut x.n, 11 * MIN, "as", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", revision, &[x.ana.1]) }).aceptada());
    assert_eq!(citas_de_batalla(&x.n), vec![(16 * MIN, nucleo::PRIO_BATALLA, SucesoBatalla::Vence)], "asignarla reprograma el plazo: la cita vieja murió con su generación");
    x.n.avanzar_hasta(16 * MIN - 1);
    assert_eq!(x.n.batalla(id).unwrap().estado, EstadoBatalla::Asignada);
    x.n.avanzar_hasta(16 * MIN);
    assert_eq!((x.n.batalla(id).unwrap().estado, x.n.batalla(id).unwrap().cerrada_en), (EstadoBatalla::Fallida, Some(16 * MIN)));
    libres(&x);

    // En curso: la duración máxima (15 min) más el margen (5 min) desde el inicio.
    let (mut x, id) = batalla_abierta();
    en_curso(&mut x.n, id, 11 * MIN, &[x.ana.1]);
    assert_eq!(x.n.batalla(id).unwrap().expira_en, 31 * MIN);
    x.n.avanzar_hasta(31 * MIN - 1);
    assert_eq!(x.n.batalla(id).unwrap().estado, EstadoBatalla::EnCurso);
    // Al llegar el instante exacto del plazo, la batalla ya está fallida (409: no está en curso) y el resultado se rechaza.
    let r = resultado(&x.n, id, Lado::Atacante, 1);
    let (v, h) = (x.n.version(), x.n.hash_estado());
    let tarde = aplicar(&mut x.n, 31 * MIN, "tarde", id, r);
    assert_eq!(codigo(&tarde), rechazos::BATALLA_INVALIDA);
    assert_eq!(x.n.batalla(id).unwrap().estado, EstadoBatalla::Fallida);
    assert!(x.n.version() > v && x.n.hash_estado() != h, "lo que cambió fue el vencimiento, no el rechazo");
    libres(&x);
    assert_eq!(soldados(&x.n, x.ana.1, x.fuerte[0]), completa(Tropa::EspadachinesBronce), "ni una baja");
}

#[test]
fn la_convocatoria_de_un_asedio_y_los_plazos_son_citas_de_prioridad_uno_sin_eventos_por_entidad_ociosa() {
    // Sin batallas la cola no guarda ninguna cita de batalla.
    let x = choque();
    assert!(citas_de_batalla(&x.n).is_empty());
    let (mut x, id) = batalla_abierta();
    let b = x.n.batalla(id).unwrap();
    assert_eq!(b.convocatoria_hasta, None);
    // Cerrada y retirada, la cola queda como estaba.
    x.n.avanzar_hasta(15 * MIN + HORA);
    assert!(x.n.batalla(id).is_none() && citas_de_batalla(&x.n).is_empty());
}

// ───────────────────────────── 6. Las salidas ─────────────────────────────

fn salida(intento: &str, revision: u32, heroe: u64, motivo: MotivoSalida, derrotado: bool, escuadras: Vec<EscuadraDeSalida>) -> SalidaBatalla {
    SalidaBatalla { intento: intento.into(), revision, heroe, motivo, derrotado, escuadras }
}

#[test]
fn quien_no_llega_a_conectarse_sale_sin_castigo_la_revision_sube_y_sin_humanos_se_cancela() {
    let (mut x, id) = batalla_abierta();
    let (ana, beto) = (x.ana, x.beto);
    let revision = x.n.batalla(id).unwrap().ticket.revision;
    assert!(msg(&mut x.n, 11 * MIN, "as", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", revision, &[ana.1]) }).aceptada());
    // Solo se sale así antes de empezar, el servidor de la asignación y quien combata.
    x.n.avanzar_hasta(12 * MIN);
    let antes = rastro(&x.n);
    let sale = |x: &mut Choque, i: &str, s: SalidaBatalla| msg(&mut x.n, 12 * MIN, i, Comando::RegistrarSalida { batalla: id, salida: s });
    assert_eq!(codigo(&sale(&mut x, "s0", salida("a1", 0, 9_999, MotivoSalida::NoConectado, false, vec![]))), rechazos::BATALLA_INVALIDA, "no combate en ella");
    assert_eq!(codigo(&sale(&mut x, "s1", salida("a2", 0, beto.1, MotivoSalida::NoConectado, false, vec![]))), rechazos::BATALLA_INVALIDA, "otro intento");
    assert_eq!(codigo(&sale(&mut x, "s2", salida("a1", 0, beto.1, MotivoSalida::Eliminado, false, vec![]))), rechazos::BATALLA_INVALIDA, "eliminado solo con la partida en curso");
    assert_eq!(rastro(&x.n), antes);
    // Beto (un bot) no llega: la batalla vuelve a `convocando` con el ticket sin él y una revisión más; su columna queda libre.
    let r = sale(&mut x, "s3", salida("a1", 0, beto.1, MotivoSalida::NoConectado, false, vec![]));
    assert!(r.aceptada(), "{r:?}");
    let b = x.n.batalla(id).unwrap();
    assert_eq!((b.estado, b.ticket.revision, b.asignacion.is_none(), b.expira_en), (EstadoBatalla::Convocando, 1, true, 12 * MIN + 5 * MIN), "plazo de asignación otra vez");
    assert!(b.ticket.defensor.participantes.is_empty() && b.salio(beto.1));
    assert_eq!(hechos(&x.n, "batalla.revisada").len(), 1);
    assert!(!x.n.ejercito(beto.0).unwrap().en_batalla && x.n.ejercito(ana.0).unwrap().en_batalla, "su columna deja de estar quieta; la de ana sigue");
    assert_eq!(candado(&x.n, beto.1, x.debil), None);
    assert_eq!(citas_de_batalla(&x.n), vec![(17 * MIN, nucleo::PRIO_BATALLA, SucesoBatalla::Vence)]);
    assert_eq!(x.n.token_de_jugador(ana.1, id), nucleo::TokenDeJugador::SinToken, "la asignación y los tokens anteriores dejan de valer");
    // Repetir la salida de quien ya salió: bien y sin cambios (se mira antes de comprobar la asignación, que ya no existe).
    let antes = rastro(&x.n);
    assert_eq!(sale(&mut x, "s3-otra-vez", salida("a1", 0, beto.1, MotivoSalida::NoConectado, false, vec![])).resultado, Resultado::SinCambios);
    assert_eq!(rastro(&x.n), antes);
    // Quien salió no vuelve a entrar.
    let gus = columna(&mut x.n, "gus", 3, c(10, 2));
    assert!(x.n.aplicar_en(12 * MIN + 1, Peticion::nueva("gus", "u0", Comando::UnirseABatalla { heroe: beto.1, batalla: id, lado: None })).resultado != Resultado::Aceptada);
    let _ = gus;
    // La asignación vieja ya no vale; la nueva, con la revisión 1.
    assert_eq!(codigo(&msg(&mut x.n, 13 * MIN, "as-vieja", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a3", 0, &[ana.1]) })), rechazos::BATALLA_INVALIDA);
    assert!(msg(&mut x.n, 13 * MIN, "as-nueva", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a3", 1, &[ana.1]) }).aceptada());
    // Ana, el último humano, tampoco llega: se cancela sin castigo y se sueltan todos los candados.
    let r = sale(&mut x, "s4", salida("a3", 1, ana.1, MotivoSalida::NoConectado, false, vec![]));
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(x.n.batalla(id).unwrap().estado, EstadoBatalla::Cancelada);
    assert_eq!(hechos(&x.n, "batalla.cancelada").len(), 1);
    libres(&x);
}

#[test]
fn con_la_partida_en_curso_la_salida_aplica_las_bajas_hiere_al_derrotado_y_el_resultado_ya_no_la_repite() {
    let (mut x, id) = batalla_abierta();
    let (ana, beto) = (x.ana, x.beto);
    en_curso(&mut x.n, id, 11 * MIN, &[ana.1]);
    let autorizadas = completa(Tropa::MiliciaLanceros);
    let sale = |x: &mut Choque, i: &str, s: SalidaBatalla| msg(&mut x.n, 12 * MIN, i, Comando::RegistrarSalida { batalla: id, salida: s });
    x.n.avanzar_hasta(12 * MIN);
    let antes = rastro(&x.n);
    // Las escuadras tienen que ser suyas, no repetirse y cerrar sobre lo autorizado.
    let (fuerte0, debil) = (x.fuerte[0], x.debil);
    let mala = |e: Vec<EscuadraDeSalida>| salida("a1", 0, beto.1, MotivoSalida::Eliminado, true, e);
    let e = |escuadra: u64, supervivientes: i64, muertos: i64| EscuadraDeSalida { escuadra, supervivientes, muertos };
    assert_eq!(codigo(&sale(&mut x, "s1", mala(vec![e(fuerte0, 1, 1)]))), rechazos::BATALLA_INVALIDA, "la escuadra es de ana");
    assert_eq!(codigo(&sale(&mut x, "s2", mala(vec![e(debil, 1, 1), e(debil, 1, 1)]))), rechazos::BATALLA_INVALIDA, "repetida");
    assert_eq!(codigo(&sale(&mut x, "s3", mala(vec![e(debil, 1, 1)]))), rechazos::BATALLA_INVALIDA, "no cierra sobre lo autorizado");
    assert_eq!(codigo(&sale(&mut x, "s4", salida("a1", 0, beto.1, MotivoSalida::NoConectado, false, vec![]))), rechazos::BATALLA_INVALIDA, "no_conectado solo antes de empezar");
    assert_eq!(rastro(&x.n), antes);
    let r = sale(&mut x, "s5", mala(vec![e(debil, 10, autorizadas - 10)]));
    assert!(r.aceptada(), "{r:?}");
    let b = x.n.batalla(id).unwrap();
    assert_eq!((b.estado, b.ticket.revision, b.salidas.len(), b.salio(beto.1)), (EstadoBatalla::EnCurso, 0, 1, true), "el ticket no cambia");
    assert_eq!(soldados(&x.n, beto.1, x.debil), 10, "las bajas, aplicadas");
    assert!(x.n.heroe_herido(beto.1) && !x.n.heroe_herido(ana.1));
    assert_eq!(candado(&x.n, beto.1, x.debil), None, "libre");
    assert!(!x.n.ejercito(beto.0).unwrap().en_batalla, "su columna ya no está quieta");
    assert_eq!(hechos(&x.n, "batalla.salida").len(), 1);
    // El resultado final cubre solo a los que siguen combatiendo.
    let mut con_todos = resultado(&x.n, id, Lado::Atacante, 3);
    con_todos.por_heroe.push(ResultadoHeroe { heroe: beto.1, participo: true, sobrevivio_al_cierre: false, xp_ganada: 0, derrotado: None, botin: None });
    assert_eq!(codigo(&aplicar(&mut x.n, 13 * MIN, "r1", id, con_todos)), rechazos::BATALLA_INVALIDA);
    let bueno = resultado(&x.n, id, Lado::Atacante, 3);
    assert_eq!((bueno.por_heroe.len(), bueno.por_escuadra.len()), (1, 2), "ana y sus dos escuadras");
    assert!(aplicar(&mut x.n, 13 * MIN, "r2", id, bueno).aceptada());
    assert_eq!(soldados(&x.n, beto.1, x.debil), 10, "lo suyo no se vuelve a tocar");
    assert_eq!(x.n.heroe(beto.1).unwrap().heroe.herido_hasta, Some(12 * MIN + 2 * MIN), "ni se le hiere dos veces");
}

// ───────────────────────────── 7. Unirse ─────────────────────────────

/// Ana (F1, humana, con arqueros) ataca a las 10 min un campamento de bandidos de nivel 2 que tiene a un paso. Devuelve la batalla y el campamento.
struct Bandidos {
    n: Nucleo,
    ana: (u64, u64),
    campamento: u64,
    id: u64,
}

fn contra_bandidos() -> Bandidos {
    let mut n = nucleo_unity();
    let ana = columna(&mut n, "ana", 1, c(10, 2));
    dar(&mut n, "d1", ana.1, Tropa::ArquerosCompuesto);
    humano(&mut n, ana.1);
    let r = n.aplicar_en(n.ahora(), admin("b1", Comando::ColocarBandidos { posicion: Punto::new(c(10, 2).x + 1_000, c(10, 2).y), nivel: 2 }));
    let campamento = r.creadas[0];
    n.avanzar_hasta(5 * MIN);
    let r = atacar(&mut n, 10 * MIN, "ana", "a1", ana.1, ObjetivoAtaque::Campamento(campamento), false);
    assert!(r.aceptada(), "{r:?}");
    let id = n.batallas().next().expect("se abrió").id;
    Bandidos { n, ana, campamento, id }
}

#[test]
fn contra_un_campamento_de_bandidos_se_juega_un_evento_pve_cualquiera_se_une_como_atacante_y_el_campamento_cae() {
    let mut x = contra_bandidos();
    let (ana, id) = (x.ana, x.id);
    let b = x.n.batalla(id).unwrap();
    // El ticket: los bandidos son una escuadra de milicia sin dueño ni Facción, con las unidades de su nivel; nadie ayuda a los bandidos.
    assert!(matches!(b.ticket.contexto, combate::ciclo::Contexto::CampamentoBandidos { campamento, .. } if u64::from(campamento) == x.campamento));
    assert_eq!((b.ticket.defensor.faccion, b.ticket.defensor.participantes.len()), (None, 0));
    let bandidos = &b.ticket.defensor.sin_heroe[0];
    assert_eq!((bandidos.tropa, bandidos.dueno, bandidos.efectivos), (Tropa::MiliciaLanceros, None, combate::Reglas::cargar().bandidos.niveles[1].unidades));
    assert!(bandidos.id > ana.1, "tiene un id de la partida, que no es de ningún héroe");
    assert_eq!(b.bloqueo.campamento, Some(x.campamento as u32));
    assert_eq!(hechos(&x.n, "evento_pve.abierta").len(), 1);
    assert!(hechos(&x.n, "batalla.abierta").is_empty(), "contra bandidos el prefijo es evento_pve.");
    // Mientras dura, ni el campamento se ataca otra vez ni se simula.
    let otra = columna(&mut x.n, "gus", 3, Punto::new(c(10, 2).x, c(10, 2).y + 1_000));
    dar(&mut x.n, "dg", otra.1, Tropa::EspadachinesBronce);
    humano(&mut x.n, otra.1);
    assert_eq!(codigo(&atacar(&mut x.n, 11 * MIN, "gus", "x1", otra.1, ObjetivoAtaque::Campamento(x.campamento), false)), rechazos::BATALLA_BLOQUEADO);

    // Unirse: sin candado de batalla, pero con sus condiciones.
    let une = |x: &mut Bandidos, t: Instante, actor: &str, i: &str, heroe: u64, lado: Option<Lado>| x.n.aplicar_en(t, Peticion::nueva(actor, i, Comando::UnirseABatalla { heroe, batalla: id, lado }));
    x.n.avanzar_hasta(11 * MIN);
    let antes = rastro(&x.n);
    assert_eq!(codigo(&une(&mut x, 11 * MIN, "ana", "u1", ana.1, None)), rechazos::BATALLA_INVALIDA, "ya combate");
    assert_eq!(codigo(&une(&mut x, 11 * MIN, "gus", "u2", ana.1, None)), rechazos::NO_AUTORIZADO, "el héroe es de otro");
    assert_eq!(codigo(&une(&mut x, 11 * MIN, "gus", "u3", otra.1, Some(Lado::Defensor))), rechazos::BATALLA_INVALIDA, "nadie ayuda a los bandidos");
    assert_eq!(codigo(&x.n.aplicar_en(11 * MIN, Peticion::nueva("gus", "u4", Comando::UnirseABatalla { heroe: otra.1, batalla: id + 100, lado: None }))), rechazos::BATALLA_NO_EXISTE);
    assert_eq!(rastro(&x.n), antes);
    let lejos = columna(&mut x.n, "hugo", 2, c(30, 2));
    dar(&mut x.n, "dh", lejos.1, Tropa::EspadachinesBronce);
    assert_eq!(codigo(&une(&mut x, 11 * MIN, "hugo", "u5", lejos.1, None)), rechazos::BATALLA_INVALIDA, "a más de radioEncuentro de la batalla");
    assert!(x.n.aplicar_en(11 * MIN, admin("h", Comando::HerirHeroe { heroe: otra.1 })).aceptada());
    assert_eq!(codigo(&une(&mut x, 11 * MIN, "gus", "u6", otra.1, None)), rechazos::BATALLA_INVALIDA, "herido");
    assert!(x.n.aplicar_en(11 * MIN, admin("h2", Comando::HerirHeroe { heroe: lejos.1 })).aceptada());
    x.n.avanzar_hasta(13 * MIN);

    // Gus, que es de una Facción cualquiera, se une como atacante: una incorporación, y el ticket no cambia ni sube de revisión.
    let ticket0 = x.n.batalla(id).unwrap().ticket.clone();
    let r = une(&mut x, 13 * MIN, "gus", "u7", otra.1, None);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(r.creadas, vec![1], "la secuencia de su incorporación");
    let b = x.n.batalla(id).unwrap();
    assert_eq!(b.ticket, ticket0);
    assert_eq!((b.incorporaciones.len(), b.incorporaciones[0].secuencia, b.incorporaciones[0].lado, b.incorporaciones[0].participante.heroe), (1, 1, Lado::Atacante, otra.1));
    assert_eq!(b.bloqueo.ejercitos, vec![ana.0, otra.0]);
    assert!(x.n.ejercito(otra.0).unwrap().en_batalla && x.n.heroe_en_batalla(otra.1));
    assert_eq!(candado(&x.n, otra.1, x.n.ejercito(otra.0).unwrap().columna.escuadras[0]), Some(id));
    assert_eq!(hechos(&x.n, "evento_pve.refuerzos").len(), 1);
    assert_eq!(x.n.token_de_jugador(otra.1, id), nucleo::TokenDeJugador::SinToken, "combate en ella");
    // Llena el bando (5 héroes por bando): el sexto no entra.
    let mut mas = vec![];
    for k in 0..4 {
        let col = columna(&mut x.n, &format!("c{k}"), 3, Punto::new(c(10, 2).x, c(10, 2).y + 1_000));
        dar(&mut x.n, &format!("dc{k}"), col.1, Tropa::MiliciaLanceros);
        mas.push(col);
    }
    for (k, col) in mas.iter().enumerate() {
        let r = une(&mut x, 13 * MIN, &format!("c{k}"), &format!("uu{k}"), col.1, None);
        if k < 3 {
            assert!(r.aceptada(), "{r:?}");
        } else {
            assert_eq!(codigo(&r), rechazos::BATALLA_INVALIDA, "ese bando está lleno");
        }
    }
    assert_eq!(x.n.batalla(id).unwrap().incorporaciones.len(), 4);
    assert_eq!(x.n.incorporaciones_vigentes(id).iter().map(|i| i.secuencia).collect::<Vec<_>>(), vec![1, 2, 3, 4]);
    assert_eq!(x.n.ticket_publicado(id).map(|t| t.revision), Some(0));

    // Juegan y ganan: el campamento desaparece y paga su oro (nivel 2) a cada héroe atacante, al carro de su columna.
    en_curso(&mut x.n, id, 14 * MIN, &[ana.1]);
    let res = resultado(&x.n, id, Lado::Atacante, 3);
    assert_eq!(res.por_heroe.len(), 5, "los héroes de las incorporaciones también");
    let heridos0 = x.n.heroe_herido(ana.1);
    assert!(aplicar(&mut x.n, 15 * MIN, "res", id, res).aceptada());
    assert!(x.n.campamento_de_bandidos(x.campamento).is_none());
    let oro = |col: u64| x.n.ejercito(col).unwrap().columna.carga.get(&asentamiento::Recurso::Oro).copied().unwrap_or(0);
    let total_oro: i64 = [ana.0, otra.0].into_iter().chain(mas.iter().take(3).map(|m| m.0)).map(oro).sum();
    assert_eq!(total_oro, 5 * 33_000, "33 de oro por héroe del nivel 2, repartido por rondas entre las cinco columnas");
    assert_eq!(hechos(&x.n, "evento_pve.aplicada").len(), 1);
    assert!(!heridos0 && !x.n.heroe_herido(ana.1), "gana el atacante");
    for col in [ana.0, otra.0].into_iter().chain(mas.iter().take(3).map(|m| m.0)) {
        assert!(!x.n.ejercito(col).unwrap().en_batalla);
    }
    assert!(!x.n.ejercito(mas[3].0).unwrap().en_batalla, "la que no entró nunca estuvo apartada");
    assert_eq!(x.n.estado().combate.reaparecen.len() + x.n.estado().combate.bandidos.len(), x.n.estado().combate.reaparecen.len(), "el campamento destruido no está");
}

#[test]
fn en_una_batalla_campal_nadie_de_fuera_se_une_y_la_persecucion_pide_bando() {
    let (mut x, id) = batalla_abierta();
    let otra = columna(&mut x.n, "gus", 3, Punto::new(c(10, 2).x, c(10, 2).y + 1_000));
    dar(&mut x.n, "dg", otra.1, Tropa::EspadachinesBronce);
    x.n.avanzar_hasta(11 * MIN);
    let antes = rastro(&x.n);
    let r = x.n.aplicar_en(11 * MIN, Peticion::nueva("gus", "u1", Comando::UnirseABatalla { heroe: otra.1, batalla: id, lado: Some(Lado::Atacante) }));
    assert_eq!(codigo(&r), rechazos::BATALLA_INVALIDA);
    assert!(matches!(&r.resultado, Resultado::Rechazada { mensaje, .. } if mensaje.contains("nadie de fuera")), "{r:?}");
    assert_eq!(rastro(&x.n), antes);
}

// ───────────────────────────── 8. Retención, fotos y determinismo ─────────────────────────────

#[test]
fn una_cerrada_se_conserva_una_hora_para_repetir_su_resultado_y_luego_sale_del_estado_y_va_al_historico() {
    let (mut x, id) = batalla_abierta();
    en_curso(&mut x.n, id, 11 * MIN, &[x.ana.1]);
    let res = resultado(&x.n, id, Lado::Atacante, 1);
    assert!(aplicar(&mut x.n, 14 * MIN, "res", id, res.clone()).aceptada());
    assert_eq!(citas_de_batalla(&x.n), vec![(14 * MIN + HORA, nucleo::PRIO_BATALLA, SucesoBatalla::Retira)], "al cerrarse solo queda la retirada");
    // Durante la hora se repite el resultado idéntico (bien, sin cambios) y se consulta.
    x.n.avanzar_hasta(14 * MIN + HORA - 1);
    repetido(&mut x.n, 14 * MIN + HORA - 1, "otra-vez", Comando::AplicarResultado { batalla: id, resultado: res.clone() });
    assert!(x.n.batalla(id).is_some() && x.n.tomar_batallas_retiradas().is_empty());
    // A la hora exacta sale del estado y de la cola; lo retirado se recoge una vez para el histórico.
    x.n.avanzar_hasta(14 * MIN + HORA);
    assert!(x.n.batalla(id).is_none() && citas_de_batalla(&x.n).is_empty());
    assert_eq!(hechos(&x.n, "batalla.retirada")[0].0, 14 * MIN + HORA);
    let retiradas = x.n.tomar_batallas_retiradas();
    assert_eq!((retiradas.len(), retiradas[0].id, retiradas[0].estado, retiradas[0].resultado.as_ref()), (1, id, EstadoBatalla::Aplicada, Some(&res)));
    assert!(x.n.tomar_batallas_retiradas().is_empty());
    // Después de retirada no existe para el contrato: ni siquiera un resultado idéntico.
    assert_eq!(rechazado(&mut x.n, 14 * MIN + HORA + 1, "tarde", Comando::AplicarResultado { batalla: id, resultado: res }), rechazos::BATALLA_NO_EXISTE);
    assert!(x.n.batallas_pendientes().is_empty() && x.n.batallas().next().is_none());
}

/// La batalla del campo a mitad de partida: asignada con su token, en curso, con una incorporación... no; con una salida y un resultado pendiente.
fn a_mitad() -> (Nucleo, u64) {
    let mut x = contra_bandidos();
    let otra = columna(&mut x.n, "gus", 3, Punto::new(c(10, 2).x, c(10, 2).y + 1_000));
    dar(&mut x.n, "dg", otra.1, Tropa::EspadachinesBronce);
    assert!(x.n.aplicar_en(11 * MIN, Peticion::nueva("gus", "u", Comando::UnirseABatalla { heroe: otra.1, batalla: x.id, lado: None })).aceptada());
    en_curso(&mut x.n, x.id, 12 * MIN, &[x.ana.1]);
    assert!(msg(&mut x.n, 13 * MIN, "tk", Comando::RegistrarTokens { batalla: x.id, intento: "a1".into(), tokens: vec![Token { heroe: x.ana.1, token: "otro".into(), expira_en: 5 }] }).aceptada());
    (x.n, x.id)
}

fn reglas_unity() -> Reglas {
    Reglas { batallas_en_unity: true, ..comun::reglas() }
}

#[test]
fn la_foto_a_mitad_de_una_batalla_recupera_lo_mismo_rechaza_el_formato_anterior_y_cada_campo_cambia_el_hash() {
    let (n, id) = a_mitad();
    let reglas = reglas_unity();
    let bytes = n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN17");
    let vuelta = nucleo::Estado::deserializar(&bytes, &reglas, "hash-de-prueba").unwrap();
    assert_eq!(vuelta.serializar(), bytes, "serializar → deserializar → serializar es idéntico");
    assert_eq!(vuelta.batallas, n.estado().batallas);
    assert_eq!(vuelta.hash(), n.hash_estado());
    // El formato anterior se rechaza con su motivo, y las reglas tienen que ser las mismas (el interruptor de los servidores de batalla es una regla).
    let viejo = [b"BAN12".as_slice(), &bytes[5..]].concat();
    let Err(e) = nucleo::Estado::deserializar(&viejo, &reglas, "hash-de-prueba") else { panic!("BAN12 no debe admitirse") };
    assert!(e.to_string().contains("BAN12") && e.to_string().contains("batallas"), "{e}");
    assert!(nucleo::Estado::deserializar(&bytes, &comun::reglas(), "hash-de-prueba").is_err(), "sin servidores de batalla las reglas no coinciden");
    assert!(nucleo::Estado::deserializar(&bytes[..bytes.len() - 7], &reglas, "hash-de-prueba").is_err(), "truncada");

    // Cada campo del estado de las batallas cambia el hash (y ninguno coincide con otro cambio).
    type Cambio = (&'static str, Box<dyn Fn(&mut nucleo::Estado)>);
    let batalla = move |f: fn(&mut combate::ciclo::Batalla)| -> Box<dyn Fn(&mut nucleo::Estado)> { Box::new(move |s| f(s.batallas.batallas.get_mut(&id).unwrap())) };
    let cambios: Vec<Cambio> = vec![
        ("estado", batalla(|b| b.estado = EstadoBatalla::Asignada)),
        ("iniciada_por", batalla(|b| b.iniciada_por += 1)),
        ("punto", batalla(|b| b.punto.x += 1)),
        ("revision", batalla(|b| b.ticket.revision += 1)),
        ("duracion", batalla(|b| b.ticket.duracion_maxima += 1)),
        ("contexto", batalla(|b| b.ticket.contexto = combate::ciclo::Contexto::SaqueoPueblo { pueblo: 1, punto: Punto::new(0, 0) })),
        ("capacidad", batalla(|b| b.ticket.atacante.capacidad += 1)),
        ("faccion_del_bando", batalla(|b| b.ticket.defensor.faccion = Some(9))),
        ("controlador", batalla(|b| b.ticket.atacante.participantes[0].controlador = Controlador::Bot)),
        ("casillas", batalla(|b| b.ticket.atacante.participantes[0].casillas_libres -= 1)),
        ("efectivos", batalla(|b| b.ticket.atacante.participantes[0].escuadras[0].efectivos -= 1)),
        ("tropa", batalla(|b| b.ticket.atacante.participantes[0].escuadras[0].tropa = Tropa::Arqueros)),
        ("sin_heroe", batalla(|b| b.ticket.defensor.sin_heroe[0].efectivos -= 1)),
        ("incorporacion.lado", batalla(|b| b.incorporaciones[0].lado = Lado::Defensor)),
        ("incorporacion.secuencia", batalla(|b| b.incorporaciones[0].secuencia += 1)),
        ("salidas", batalla(|b| b.salidas.push(combate::ciclo::Salida { heroe: 1, motivo: MotivoSalida::Abandono, derrotado: true, en: 3 }))),
        ("bloqueo.ejercitos", batalla(|b| b.bloqueo.ejercitos.push(77))),
        ("bloqueo.caravanas", batalla(|b| b.bloqueo.caravanas.push(77))),
        ("bloqueo.asentamiento", batalla(|b| b.bloqueo.asentamiento = Some(3))),
        ("bloqueo.pueblo", batalla(|b| b.bloqueo.pueblo = Some(3))),
        ("expira_en", batalla(|b| b.expira_en += 1)),
        ("convocatoria_hasta", batalla(|b| b.convocatoria_hasta = Some(5))),
        ("iniciada_en", batalla(|b| b.iniciada_en = None)),
        ("aplicada_en", batalla(|b| b.aplicada_en = Some(1))),
        ("cerrada_en", batalla(|b| b.cerrada_en = Some(1))),
        ("asignacion.token", batalla(|b| b.asignacion.as_mut().unwrap().tokens[0].token.push('x'))),
        ("asignacion.puerto", batalla(|b| b.asignacion.as_mut().unwrap().instancia.puerto += 1)),
        ("asignacion.servidor", batalla(|b| b.asignacion.as_mut().unwrap().servidor.push('x'))),
        ("asignacion.ninguna", batalla(|b| b.asignacion = None)),
        ("resultado", batalla(|b| b.estado = EstadoBatalla::Cancelada)),
        ("ninguna", Box::new(move |s| {
            s.batallas.batallas.remove(&id);
        })),
        ("plan_en_pausa", Box::new(|s| s.ejercitos.values_mut().next().unwrap().plan_en_pausa = Some(nucleo::PlanEnPausa { desde: 1, velocidad: 2, plan: Default::default() }))),
        ("en_batalla", Box::new(|s| s.ejercitos.values_mut().next().unwrap().en_batalla ^= true)),
        ("reserva", Box::new(|s| s.militar.heroes.values_mut().next().unwrap().heroe.escuadras[0].reserva_batalla = None)),
    ];
    let base = n.hash_estado();
    let mut vistos = BTreeSet::from([base]);
    for (nombre, cambio) in &cambios {
        let mut e = nucleo::Estado::deserializar(&bytes, &reglas, "hash-de-prueba").unwrap();
        cambio(&mut e);
        assert!(vistos.insert(e.hash()), "el cambio «{nombre}» no cambia el hash (o coincide con otro)");
        // Y lo que cambia se serializa y se lee igual.
        let otra = nucleo::Estado::deserializar(&e.serializar(), &reglas, "hash-de-prueba").unwrap();
        assert_eq!(otra.hash(), e.hash(), "«{nombre}» no vuelve igual de la foto");
    }
    // Un resultado distinto también cambia el hash.
    let mut e = nucleo::Estado::deserializar(&bytes, &reglas, "hash-de-prueba").unwrap();
    let mut res = resultado(&n, id, Lado::Atacante, 1);
    e.batallas.batallas.get_mut(&id).unwrap().resultado = Some(res.clone());
    let h1 = e.hash();
    res.por_heroe[0].botin = Some(Botin { objetos: vec![ObjetoBotin { definicion: "x".into(), cantidad: 2, instancia: None }], monedas: Monedas { bronce: 1, plata: 0, oro: 0 } });
    e.batallas.batallas.get_mut(&id).unwrap().resultado = Some(res);
    assert_ne!(e.hash(), h1);
    assert_eq!(nucleo::Estado::deserializar(&e.serializar(), &reglas, "hash-de-prueba").unwrap().hash(), e.hash());
}

fn arranque_persistente() -> Arranque {
    let mut a = comun::arranque(comun::reticula(40, 5));
    a.reglas = reglas_unity();
    a.ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    a
}

#[test]
fn una_foto_con_la_batalla_asignada_y_el_registro_reconstruyen_el_mismo_mundo_y_los_plazos_vencen_a_su_hora() {
    let dir = tempfile::tempdir().unwrap();
    let ana_y_el_resto = |n: &mut Nucleo| {
        let ana = columna(n, "ana", 1, c(10, 2));
        dar(n, "d1", ana.1, Tropa::ArquerosCompuesto);
        humano(n, ana.1);
        let b = n.aplicar_en(n.ahora(), admin("b1", Comando::ColocarBandidos { posicion: Punto::new(c(10, 2).x + 1_000, c(10, 2).y), nivel: 1 })).creadas[0];
        (ana, b)
    };
    // El mundo que no se interrumpe.
    let sin_parar = {
        let mut n = Nucleo::nuevo(arranque_persistente());
        let (ana, b) = ana_y_el_resto(&mut n);
        assert!(atacar(&mut n, 10 * MIN, "ana", "a1", ana.1, ObjetivoAtaque::Campamento(b), false).aceptada());
        let id = n.batallas().next().unwrap().id;
        let revision = n.batalla(id).unwrap().ticket.revision;
        assert!(msg(&mut n, 11 * MIN, "as", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", revision, &[ana.1]) }).aceptada());
        assert!(msg(&mut n, 11 * MIN + 30_000, "in", Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision }).aceptada());
        n.avanzar_hasta(40 * MIN);
        (n.hash_estado(), n.batalla(id).unwrap().clone())
    };
    assert_eq!(sin_parar.1.estado, EstadoBatalla::Fallida, "el resultado no llegó a tiempo");
    // El que se apaga a mitad de partida, con una foto después de asignarla.
    let (a_mitad, id, ana) = {
        let mut n = Nucleo::con_persistencia(arranque_persistente(), dir.path(), Default::default()).unwrap();
        let (ana, b) = ana_y_el_resto(&mut n);
        assert!(atacar(&mut n, 10 * MIN, "ana", "a1", ana.1, ObjetivoAtaque::Campamento(b), false).aceptada());
        let id = n.batallas().next().unwrap().id;
        let revision = n.batalla(id).unwrap().ticket.revision;
        assert!(msg(&mut n, 11 * MIN, "as", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", revision, &[ana.1]) }).aceptada());
        n.foto_ahora();
        assert!(msg(&mut n, 11 * MIN + 30_000, "in", Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision }).aceptada());
        assert!(n.metricas().ordenes_aceptadas >= 7, "los mensajes del servidor de batalla entran al registro como las demás órdenes");
        n.apagar().unwrap();
        (n.hash_estado(), id, ana)
    };
    let mut n = Nucleo::con_persistencia(arranque_persistente(), dir.path(), Default::default()).unwrap();
    assert!(n.recuperado());
    assert_eq!(n.hash_estado(), a_mitad, "foto + registro reconstruyen el mismo mundo, con la batalla en curso");
    let b = n.batalla(id).unwrap();
    assert_eq!((b.estado, b.iniciada_en, b.expira_en), (EstadoBatalla::EnCurso, Some(11 * MIN + 30_000), 11 * MIN + 30_000 + 20 * MIN));
    assert!(n.ejercito(ana.0).unwrap().en_batalla && n.heroe_en_batalla(ana.1));
    // El plazo vence a su hora aunque se recuperara de una foto y de un registro.
    n.avanzar_hasta(11 * MIN + 30_000 + 20 * MIN - 1);
    assert_eq!(n.batalla(id).unwrap().estado, EstadoBatalla::EnCurso);
    n.avanzar_hasta(40 * MIN);
    assert_eq!(n.batalla(id).unwrap(), &sin_parar.1);
    assert_eq!(n.hash_estado(), sin_parar.0);
    n.apagar().unwrap();
}

/// Un guion de dos batallas con las órdenes en instantes fijos: `avanzar` solo decide cómo se mueve el reloj entre ellas.
fn guion(avanzar: &dyn Fn(&mut Nucleo, Instante)) -> Nucleo {
    let mut x = choque();
    let (ana, beto) = (x.ana, x.beto);
    avanzar(&mut x.n, 10 * MIN);
    assert!(atacar(&mut x.n, 10 * MIN, "ana", "a1", ana.1, ObjetivoAtaque::Ejercito(beto.0), false).aceptada());
    let id = x.n.batallas().next().unwrap().id;
    avanzar(&mut x.n, 11 * MIN);
    let revision = x.n.batalla(id).unwrap().ticket.revision;
    assert!(msg(&mut x.n, 11 * MIN, "as", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", revision, &[ana.1]) }).aceptada());
    avanzar(&mut x.n, 12 * MIN);
    assert!(msg(&mut x.n, 12 * MIN, "in", Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision }).aceptada());
    avanzar(&mut x.n, 14 * MIN);
    let res = resultado(&x.n, id, Lado::Atacante, 4);
    assert!(aplicar(&mut x.n, 14 * MIN, "res", id, res).aceptada());
    // La segunda: contra bandidos, y nadie la asigna.
    avanzar(&mut x.n, 20 * MIN);
    let b = x.n.aplicar_en(20 * MIN, admin("b1", Comando::ColocarBandidos { posicion: Punto::new(c(10, 2).x, c(10, 2).y + 1_000), nivel: 3 })).creadas[0];
    avanzar(&mut x.n, 21 * MIN);
    assert!(atacar(&mut x.n, 21 * MIN, "ana", "a2", ana.1, ObjetivoAtaque::Campamento(b), false).aceptada());
    x.n.avanzar_hasta(3 * HORA);
    x.n
}

#[test]
fn mismas_ordenes_dan_el_mismo_hash_normal_acelerado_y_a_trozos() {
    let normal = guion(&|n, t| n.avanzar_hasta(t));
    assert_eq!(normal.hash_estado(), guion(&|n, t| n.avanzar_hasta(t)).hash_estado(), "repetible");
    assert_eq!(normal.hash_estado(), guion(&|_, _| {}).hash_estado(), "acelerado: las órdenes llevan su instante");
    let a_trozos = guion(&|n, t| {
        let ahora = n.ahora();
        for s in 1..=5 {
            n.avanzar_hasta(ahora + (t - ahora).max(0) * s * s / 25);
        }
    });
    assert_eq!(normal.hash_estado(), a_trozos.hash_estado(), "en pasos pequeños e irregulares, como el modo en vivo");
    // El guion hizo lo que se espera: una aplicada y una fallida, ya retiradas al cabo de la hora, y sin citas pendientes.
    assert!(normal.batallas().next().is_none() && citas_de_batalla(&normal).is_empty());
    for h in ["batalla.abierta", "batalla.asignada", "batalla.en_curso", "batalla.aplicada", "evento_pve.abierta", "evento_pve.fallida", "batalla.retirada"] {
        assert!(!hechos(&normal, h).is_empty(), "el guion no produjo «{h}»");
    }
    assert_eq!(hechos(&normal, "batalla.retirada").len(), 2);
}

// ═════════════════════════════ El mundo real ═════════════════════════════

use asentamiento::catalogo::Tipo;
use asentamiento::pueblos::EstadoPueblo;
use asentamiento::Recurso::{self, *};
use comercio::caravana::{Animal, EstadoCaravana, TipoCarro};
use militar::heroe::Residencia;
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use rutas::Rutas;
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

fn arranque_mundo(m: &M, semilla: u64) -> Arranque {
    Arranque { reglas: reglas_unity(), ajustes: Ajustes::default(), semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

fn campamento_mas_cercano(m: &Mundo, p: Punto) -> u32 {
    (0..m.campamentos().len()).min_by_key(|&i| campamento(m, i).dist2(p)).unwrap() as u32
}

/// Un sitio verde con mucho bosque a 10 u del campamento `i`: una plaza que no cae en ruinas por falta de madera.
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

fn fundar(n: &mut Nucleo, t: Instante, nombre: &str, dueno: &str, faccion: u32, p: Punto) -> u64 {
    let e = n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap();
    assert!(n.aplicar_en(t, admin(&format!("c-{nombre}"), Comando::DarCaravanaFundacion { ejercito: e })).aceptada());
    let r = n.aplicar_en(t, Peticion::nueva(dueno, &format!("f-{nombre}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

/// Dos plazas de Facciones distintas (F1 con «ana» junto al campamento 0, F2 con «beto» junto al 3) sobre un núcleo con servidores de batalla.
struct Esc {
    n: Nucleo,
    a: u64,
    b: u64,
    cuenta: u64,
}

impl Esc {
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.cuenta += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.cuenta), c);
        self.n.aplicar_en(self.n.ahora(), if actor == "admin" { p.de_admin() } else { p })
    }
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    fn avanzar(&mut self, ms: i64) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    fn pos(&self, plaza: u64) -> Punto {
        self.n.estado().asentamientos[&plaza].posicion
    }
    fn dar(&mut self, plaza: u64, recurso: Recurso, cantidad: i64) {
        let r = self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad });
        assert!(r.aceptada() || r.resultado == Resultado::SinCambios, "{r:?}");
    }
    /// Un héroe humano de la cuenta, ciudadano de `faccion`, que reside en la plaza y está dentro.
    fn heroe_en(&mut self, m: &Mundo, cuenta: &str, faccion: u32, plaza: u64) -> u64 {
        let campamento = campamento_mas_cercano(m, self.pos(plaza));
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento, controlador: Controlador::Humano }).creadas[0];
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(faccion) });
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
        h
    }
    /// Escuadras completas de esas tropas para el héroe, en su campamento, y su loadout activo las lleva: defiende en persona.
    fn defensor(&mut self, heroe: u64, tropas: &[Tropa]) -> Vec<u64> {
        let ids: Vec<u64> = tropas.iter().map(|&t| self.ok("admin", Comando::DarEscuadra { heroe, tropa: t, cantidad: completa(t) }).creadas[0]).collect();
        let loadout = self.n.heroe(heroe).unwrap().heroe.loadouts[0].id;
        self.ok("admin", Comando::GuardarLoadout { heroe, loadout: Some(loadout), nombre: "Default".into(), escuadras: ids.clone(), perks: vec![], activo: Some(true) });
        ids
    }
    /// Un ejército de administración de `cuenta`/`faccion` a 2 u de `p`, con las tropas dadas y su héroe humano.
    fn ejercito(&mut self, cuenta: &str, faccion: u32, p: Punto, tropas: &[Tropa]) -> (u64, u64) {
        let (col, h) = columna(&mut self.n, cuenta, faccion, Punto::new(p.x + 2_000, p.y));
        for &t in tropas {
            self.ok("admin", Comando::DarEscuadra { heroe: h, tropa: t, cantidad: completa(t) });
        }
        humano(&mut self.n, h);
        (col, h)
    }
    fn atacar(&mut self, actor: &str, heroe: u64, objetivo: ObjetivoAtaque, guerra: bool) -> Respuesta {
        self.cuenta += 1;
        let t = self.n.ahora();
        atacar(&mut self.n, t, actor, &format!("o{}", self.cuenta), heroe, objetivo, guerra)
    }
    fn carga(&self, columna: u64, r: Recurso) -> i64 {
        self.n.ejercito(columna).unwrap().columna.carga.get(&r).copied().unwrap_or(0)
    }
}

fn montar(m: &M, semilla: u64) -> Esc {
    let mut n = Nucleo::nuevo(arranque_mundo(m, semilla));
    let (pa, pb) = (sitio_rico(&n, &m.0, 0, 1), sitio_rico(&n, &m.0, 3, 2));
    let (a, b) = (fundar(&mut n, 0, "a", "ana", 1, pa), fundar(&mut n, 0, "b", "beto", 2, pb));
    let mut e = Esc { n, a, b, cuenta: 0 };
    for plaza in [a, b] {
        e.dar(plaza, Madera, 350_000);
        e.ok("admin", Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Mercado });
    }
    e.avanzar(9 * HORA);
    e
}

// ───────────────────────────── Asedio ─────────────────────────────

/// Beto (humano, F2) dentro de la plaza B con su loadout; el ejército de ana (humana, F1) a su puerta; carla (F2) llega con su columna.
struct Asedio {
    e: Esc,
    beto: u64,
    ana: (u64, u64),
}

fn asedio(m: &'static M, semilla: u64) -> Asedio {
    let mut e = montar(m, semilla);
    let b = e.b;
    let beto = e.heroe_en(&m.0, "beto", 2, b);
    e.defensor(beto, &[Tropa::MiliciaLanceros]);
    let ana = e.ejercito("ana", 1, e.pos(b), &[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto]);
    Asedio { e, beto, ana }
}

#[test]
fn en_la_convocatoria_defienden_los_aliados_y_los_vasallos_que_estan_dentro_y_los_neutrales_y_los_enemigos_no_aunque_esten_dentro() {
    let m = con_mundo!();
    let Asedio { mut e, beto, ana } = asedio(m, 7);
    let b = e.b;
    // La plaza B es de la Facción 2: la 4 es su aliada, la 5 su vasalla, la 3 no tiene relación con ella y la 6 está en guerra con ella (a esta, «beto» le abre la puerta).
    for (x, r) in [(4, Relacion::Alianza), (5, Relacion::Vasallaje), (6, Relacion::Guerra)] {
        e.ok("admin", Comando::FijarRelacion { a: 2, b: x, relacion: Some(r) });
    }
    e.ok("beto", Comando::FijarAcceso { asentamiento: b, aliados: true, neutrales: true, enemigos: true });
    let gente: Vec<(&str, u32, (u64, u64))> = [("alia", 4), ("vasa", 5), ("nora", 3), ("enri", 6)].into_iter().map(|(c, f)| (c, f, e.ejercito(c, f, e.pos(b), &[Tropa::MiliciaLanceros]))).collect();
    for (cuenta, _, col) in &gente {
        e.ok(cuenta, Comando::Guarnecer { asentamiento: b, heroe: col.1 });
        assert_eq!(e.n.heroe(col.1).unwrap().heroe.presencia.ubicacion, Ubicacion::Asentamiento(b as u32), "{cuenta} está dentro");
    }
    assert!(e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b), true).aceptada());
    let id = e.n.batallas().next().unwrap().id;
    let t = e.n.ahora();
    for (k, (cuenta, faccion, col)) in gente.iter().enumerate() {
        let r = e.n.aplicar_en(t + 1_000 * (k as i64 + 1), Peticion::nueva(cuenta, &format!("u{k}"), Comando::UnirseABatalla { heroe: col.1, batalla: id, lado: None }));
        if matches!(faccion, 4 | 5) {
            assert!(r.aceptada(), "{cuenta} (F{faccion}), aliado o vasallo y dentro, defiende: {r:?}");
        } else {
            assert_eq!(codigo(&r), rechazos::BATALLA_INVALIDA, "{cuenta} (F{faccion}) no defiende nunca");
            assert!(matches!(&r.resultado, Resultado::Rechazada { mensaje, .. } if mensaje.contains("no combate aquí")), "{r:?}");
            assert!(!e.n.ejercito(col.0).unwrap().en_batalla, "su columna sigue libre");
        }
    }
    let defensores: Vec<u64> = e.n.batalla(id).unwrap().ticket.defensor.participantes.iter().map(|p| p.heroe).collect();
    assert_eq!(defensores, vec![beto, gente[0].2 .1, gente[1].2 .1], "el residente que estaba dentro y los dos aliados que se unieron");
}

#[test]
fn un_asedio_tiene_convocatoria_de_30_segundos_quien_se_une_entra_en_el_ticket_y_si_gana_el_atacante_hay_conquista() {
    let m = con_mundo!();
    let Asedio { mut e, beto, ana } = asedio(m, 7);
    let b = e.b;
    let visitante = e.ejercito("carla", 2, e.pos(b), &[Tropa::MiliciaLanceros]);
    let neutral = e.ejercito("nora", 3, e.pos(b), &[Tropa::MiliciaLanceros]);
    let tarde = e.ejercito("dora", 2, e.pos(b), &[Tropa::MiliciaLanceros]);
    let a_la_puerta = e.ejercito("fede", 2, e.pos(b), &[Tropa::MiliciaLanceros]);
    // Solo defiende quien está dentro: Carla y Dora, de la Facción del defensor, entran de visita con su ejército (que se desarma en la puerta y entra con todo); Fede, de la misma Facción, se queda en el mapa a la puerta. Nora, de una Facción sin relación con ella, entra también (la puerta se cierra solo a los enemigos), pero dentro tampoco defiende.
    for (cuenta, h) in [("carla", visitante.1), ("dora", tarde.1), ("nora", neutral.1)] {
        e.ok(cuenta, Comando::Guarnecer { asentamiento: b, heroe: h });
    }
    assert!(!e.n.en_el_mapa(visitante.0) && !e.n.en_el_mapa(tarde.0) && e.n.en_el_mapa(a_la_puerta.0));
    e.n.avanzar_hasta(e.n.ahora() + MIN);
    let t = e.n.ahora();
    // Abre `atacar` (con la guerra declarada): una batalla de asedio con convocatoria, sin gastar una tirada ni resolver nada.
    let rng0 = e.n.estado().rng.estado();
    let r = e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b), true);
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(e.n.estado().rng.estado(), rng0);
    assert!(hechos(&e.n, "combate.asedio_conquista").is_empty() && hechos(&e.n, "combate.asedio_resistido").is_empty());
    assert_eq!(e.n.relacion(1, 2), Some(Relacion::Guerra));
    let id = e.n.batallas().next().unwrap().id;
    let bt = e.n.batalla(id).unwrap();
    assert_eq!((bt.convocatoria_hasta, bt.expira_en), (Some(t + 30_000), t + 30_000 + 5 * MIN));
    assert_eq!((bt.ticket.duracion_maxima, bt.ticket.atacante.capacidad), (30 * MIN, 15), "asedio de 15 contra 15, 30 minutos");
    assert_eq!(bt.ticket.defensor.faccion, Some(2));
    assert_eq!(bt.ticket.defensor.participantes.iter().map(|p| p.heroe).collect::<Vec<_>>(), vec![beto], "defiende el residente que está dentro, con su loadout");
    assert_eq!(bt.ticket.defensor.participantes[0].escuadras.len(), 1);
    assert_eq!(bt.bloqueo.asentamiento, Some(e.b as u32));
    assert_eq!(citas_de_batalla(&e.n), vec![(t + 30_000, nucleo::PRIO_BATALLA, SucesoBatalla::FinConvocatoria), (t + 30_000 + 5 * MIN, nucleo::PRIO_BATALLA, SucesoBatalla::Vence)]);
    // La plaza sigue produciendo, pero nadie dentro da órdenes, ni se la asedia otra vez, ni se marcha su ejército.
    assert_eq!(codigo(&e.ord("beto", Comando::PasarAViveres { heroe: beto, cantidad: 1 })), rechazos::BATALLA_BLOQUEADO, "quien está dentro de la plaza asediada no da órdenes");
    let otro = e.ejercito("gus", 3, e.pos(b), &[Tropa::EspadachinesBronce]);
    assert_eq!(codigo(&e.atacar("gus", otro.1, ObjetivoAtaque::Asentamiento(b), true)), rechazos::BATALLA_BLOQUEADO, "una plaza que ya está en una batalla no se asedia otra vez");
    assert_eq!(codigo(&e.ord("ana", Comando::MarcharA { ejercito: ana.0, destino: e.pos(b) })), rechazos::BATALLA_BLOQUEADO);
    // En la convocatoria no se publica ni se asigna.
    assert!(e.n.batallas_pendientes().is_empty() && e.n.ticket_publicado(id).is_none(), "el ticket no se publica durante la convocatoria");
    let revision = e.n.batalla(id).unwrap().ticket.revision;
    assert_eq!(rechazado(&mut e.n, t + 10_000, "as0", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion("a1", revision, &[ana.1]) }), rechazos::BATALLA_INVALIDA);
    // Carla (de la Facción del defensor) se une: durante la convocatoria entra EN EL TICKET, como una de las del principio (revisión 0, sin incorporación).
    e.n.avanzar_hasta(t + 10_000);
    let r = e.n.aplicar_en(t + 10_000, Peticion::nueva("carla", "u1", Comando::UnirseABatalla { heroe: visitante.1, batalla: id, lado: None }));
    assert!(r.aceptada(), "{r:?}");
    assert!(r.creadas.is_empty(), "sin secuencia: no es una incorporación");
    let bt = e.n.batalla(id).unwrap();
    assert_eq!((bt.ticket.revision, bt.incorporaciones.len(), bt.ticket.defensor.participantes.len()), (0, 0, 2));
    assert_eq!(hechos(&e.n, "batalla.refuerzos").len(), 1);
    // Fede es de la Facción del defensor y está a la puerta, a ≤ radioEncuentro de la batalla, pero en el mapa no se defiende: hay que estar dentro.
    let ticket_antes = e.n.batalla(id).unwrap().ticket.clone();
    let r = e.n.aplicar_en(t + 10_500, Peticion::nueva("fede", "u1b", Comando::UnirseABatalla { heroe: a_la_puerta.1, batalla: id, lado: None }));
    assert_eq!(codigo(&r), rechazos::BATALLA_INVALIDA);
    assert!(matches!(&r.resultado, Resultado::Rechazada { mensaje, .. } if mensaje.contains("hay que estar dentro")), "{r:?}");
    let r = e.n.aplicar_en(t + 10_600, Peticion::nueva("fede", "u1c", Comando::UnirseABatalla { heroe: a_la_puerta.1, batalla: id, lado: Some(Lado::Defensor) }));
    assert_eq!(codigo(&r), rechazos::BATALLA_INVALIDA, "pedir el bando que le toca tampoco lo salva");
    assert!(e.n.batalla(id).unwrap().ticket == ticket_antes && !e.n.ejercito(a_la_puerta.0).unwrap().en_batalla, "su columna sigue libre y el ticket no cambió");
    // Una Facción que no es de nadie en el asedio no combate aquí.
    let r = e.n.aplicar_en(t + 11_000, Peticion::nueva("nora", "u2", Comando::UnirseABatalla { heroe: neutral.1, batalla: id, lado: None }));
    assert_eq!(codigo(&r), rechazos::BATALLA_INVALIDA);
    assert!(matches!(&r.resultado, Resultado::Rechazada { mensaje, .. } if mensaje.contains("no combate aquí")), "{r:?}");
    // La convocatoria acaba en un evento exacto: ya se publica, y el ticket no se asignaba hasta entonces.
    e.n.avanzar_hasta(t + 30_000 - 1);
    assert!(e.n.batallas_pendientes().is_empty() && hechos(&e.n, "batalla.convocatoria_cerrada").is_empty());
    e.n.avanzar_hasta(t + 30_000);
    assert_eq!(e.n.batallas_pendientes(), vec![id]);
    assert!(e.n.ticket_publicado(id).is_some());
    assert_eq!(hechos(&e.n, "batalla.convocatoria_cerrada")[0].0, t + 30_000);
    // Quien está dentro y llega después entra como incorporación: el ticket no cambia.
    let ticket0 = e.n.batalla(id).unwrap().ticket.clone();
    let r = e.n.aplicar_en(t + 40_000, Peticion::nueva("dora", "u3", Comando::UnirseABatalla { heroe: tarde.1, batalla: id, lado: None }));
    assert!(r.aceptada(), "{r:?}");
    assert_eq!(r.creadas, vec![1]);
    let bt = e.n.batalla(id).unwrap();
    assert_eq!((bt.ticket == ticket0, bt.incorporaciones.len(), bt.incorporaciones[0].lado), (true, 1, Lado::Defensor));

    // Asignada a los dos humanos del lado de los defensores y a ana, en curso y resultado: gana el atacante.
    let defensores = [beto, visitante.1, tarde.1];
    en_curso(&mut e.n, id, t + 50_000, &[ana.1, beto]);
    let vivos0 = defensores.map(|h| e.n.heroe(h).unwrap().heroe.escuadras.iter().map(|s| s.cantidad).sum::<i64>());
    let res = resultado(&e.n, id, Lado::Atacante, 4);
    assert_eq!(res.por_heroe.len(), 4, "ana, beto, carla (en el ticket) y dora (incorporada)");
    let faccion_vieja = e.n.estado().asentamientos[&b].faccion;
    let r = aplicar(&mut e.n, t + 5 * MIN, "res", id, res);
    assert!(r.aceptada(), "{r:?}");
    let a = &e.n.estado().asentamientos[&b];
    assert_eq!((faccion_vieja, a.faccion), (2, 1), "la plaza pasa a la Facción atacante");
    assert_eq!((a.protegida_hasta, a.ocupada_hasta), (Some(t + 5 * MIN + 24 * HORA), Some(t + 5 * MIN + 90 * MIN)));
    assert!(a.economia.ocupado);
    let c = &hechos(&e.n, "asentamiento.conquistada")[0];
    assert_eq!((c.2.clone(), dato(c, "faccion"), dato(c, "faccionAnterior"), dato(c, "ejercito")), (vec![b], 1, 2, ana.0 as i64));
    // Los defensores: heridos (perdieron), desalojados con las escuadras que les quedaron; el residente a su nueva casa.
    assert!(defensores.iter().all(|&h| e.n.heroe_herido(h)) && !e.n.heroe_herido(ana.1));
    assert!(matches!(e.n.heroe(beto).unwrap().heroe.presencia.ubicacion, Ubicacion::Columna(_)), "sale a la calle");
    assert_eq!(e.n.heroe(beto).unwrap().heroe.residencia, Some(Residencia::Campamento(campamento_mas_cercano(&m.0, e.pos(b)))));
    assert!(vivos0.iter().all(|&v| v > 0));
    assert_eq!(hechos(&e.n, "batalla.aplicada")[0].0, t + 5 * MIN);
    assert!(e.n.bloqueos().asentamientos.is_empty() && !e.n.ejercito(ana.0).unwrap().en_batalla && !e.n.ejercito(visitante.0).unwrap().en_batalla);
    // Una conquista nueva descarta los plazos de la anterior y la plaza ya no está bloqueada: sus órdenes vuelven.
    assert_eq!(codigo(&e.atacar("gus", otro.1, ObjetivoAtaque::Asentamiento(b), true)), rechazos::COMBATE_PLAZA_PROTEGIDA);
}

#[test]
fn un_asedio_que_gana_el_defensor_no_conquista_hiere_al_atacante_y_la_chatarra_se_queda_en_la_plaza() {
    let m = con_mundo!();
    let Asedio { mut e, beto, ana } = asedio(m, 7);
    let b = e.b;
    let metal0: i64 = [ChatarraCobre, ChatarraBronce, ChatarraHierro].iter().map(|&r| e.n.economia(b).unwrap().almacen.cantidad(r)).sum();
    let t = e.n.ahora();
    assert!(e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b), true).aceptada());
    let id = e.n.batallas().next().unwrap().id;
    e.n.avanzar_hasta(t + 40_000);
    en_curso(&mut e.n, id, t + 40_000, &[ana.1, beto]);
    let res = resultado(&e.n, id, Lado::Defensor, 6);
    assert!(aplicar(&mut e.n, t + 10 * MIN, "res", id, res).aceptada());
    let a = &e.n.estado().asentamientos[&b];
    assert_eq!((a.faccion, a.protegida_hasta, a.ocupada_hasta, a.economia.ocupado), (2, None, None, false));
    assert!(e.n.heroe_herido(ana.1) && !e.n.heroe_herido(beto));
    assert_eq!(e.n.heroe(beto).unwrap().heroe.presencia.ubicacion, Ubicacion::Asentamiento(b as u32), "los defensores siguen dentro");
    let metal: i64 = [ChatarraCobre, ChatarraBronce, ChatarraHierro].iter().map(|&r| e.n.economia(b).unwrap().almacen.cantidad(r)).sum();
    assert!(metal > metal0, "la chatarra de los caídos, al almacén de la plaza que resiste");
    assert!(hechos(&e.n, "asentamiento.conquistada").is_empty());
}

// ───────────────────────────── Caravanas ─────────────────────────────

impl Esc {
    /// Una caravana de la plaza con `carros` carros con buey, fuera del reparto automático.
    fn caravana(&mut self, actor: &str, plaza: u64, carros: u32) -> u64 {
        let id = self.ok(actor, Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
        self.ok(actor, Comando::ReservarCaravana { caravana: id, reservada: true });
        for k in 0..carros {
            self.ok(actor, Comando::AnadirCarro { caravana: id, tipo: TipoCarro::Basico });
            self.ok(actor, Comando::AnadirAnimal { caravana: id, carro: k, animal: Animal::Buey });
        }
        id
    }
    /// Prepara la caravana de A hacia B con 100 de cobre y avanza hasta que sale. Devuelve el instante de salida.
    fn lanzar(&mut self, caravana: u64) -> Instante {
        let (a, b) = (self.a, self.b);
        self.dar(a, Cobre, 300_000);
        self.ok("ana", Comando::PrepararCaravana { caravana, destino: b, carga: vec![(Cobre, 100_000)], salir_en: None });
        for _ in 0..600 {
            if self.n.caravana(caravana).unwrap().estado == EstadoCaravana::EnTransito {
                return self.n.caravana(caravana).unwrap().plan.as_ref().unwrap().inicio().unwrap();
            }
            self.avanzar(MIN);
        }
        panic!("la caravana no salió");
    }
}

#[test]
fn una_caravana_atacada_por_un_humano_se_juega_en_unity_y_unity_dice_que_carros_se_pierden() {
    let m = con_mundo!();
    let mut e = montar(m, 7);
    let (a, b) = (e.a, e.b);
    // Dos carros y una escolta de milicia cedida por un héroe residente.
    let ca = e.caravana("ana", a, 2);
    let carla = e.heroe_en(&m.0, "carla", 1, a);
    let milicia = e.ok("admin", Comando::DarEscuadra { heroe: carla, tropa: Tropa::MiliciaLanceros, cantidad: 25 }).creadas[0];
    e.ok("carla", Comando::AsignarEscolta { caravana: ca, heroe: carla, escuadras: vec![milicia] });
    let t0 = e.lanzar(ca);
    e.n.avanzar_hasta(t0);
    let atacante = e.ejercito("beto", 2, e.pos(a), &[Tropa::ArquerosCompuesto, Tropa::EspadachinesBronce]);
    let (v0, rng0) = (e.n.version(), e.n.estado().rng.estado());
    assert!(e.atacar("beto", atacante.1, ObjetivoAtaque::Caravana(ca), true).aceptada());
    assert_eq!((e.n.version(), e.n.estado().rng.estado()), (v0 + 1, rng0));
    let id = e.n.batallas().next().unwrap().id;
    let bt = e.n.batalla(id).unwrap();
    // En la zona del dueño: aguante. Defiende la escolta cedida; la caravana va en el ticket con sus dos carros.
    assert!(matches!(bt.ticket.contexto, combate::ciclo::Contexto::Caravana { caravana, modalidad: combate::caravana::Modalidad::Aguante, carros: 2, .. } if caravana == ca));
    assert_eq!((bt.ticket.defensor.faccion, bt.ticket.defensor.participantes.len()), (Some(1), 0));
    assert_eq!(bt.ticket.defensor.sin_heroe.iter().map(|s| (s.id, s.dueno)).collect::<Vec<_>>(), vec![(milicia, Some(carla))]);
    assert_eq!((bt.bloqueo.caravanas.clone(), bt.bloqueo.ejercitos.clone()), (vec![ca], vec![atacante.0]));
    assert_eq!(candado(&e.n, carla, milicia), Some(id), "la escolta lleva el candado");
    // La caravana queda apartada (su plan en pausa) y nadie la toca; el hecho llega a su plaza de origen.
    assert_eq!(e.n.caravana(ca).unwrap().estado, EstadoCaravana::EnBatalla);
    assert_eq!(codigo(&e.ord("ana", Comando::AnadirCarro { caravana: ca, tipo: TipoCarro::Basico })), rechazos::BATALLA_BLOQUEADO);
    assert_eq!(hechos(&e.n, "batalla.abierta")[0].2, vec![a], "se entera la plaza de origen de la caravana");
    // Gana el atacante y Unity dice que se pierde el carro 1: en un aguante se salva el 30 % de la carga.
    let t = e.n.ahora() + MIN;
    en_curso(&mut e.n, id, t, &[atacante.1]);
    let mut res = resultado(&e.n, id, Lado::Atacante, 5);
    res.carros_perdidos = vec!["1".into()];
    // Un carro que no existe se rechaza entero.
    let mut mala = res.clone();
    mala.carros_perdidos = vec!["2".into()];
    assert_eq!(rechazado(&mut e.n, t + MIN, "mala", Comando::AplicarResultado { batalla: id, resultado: mala }), rechazos::BATALLA_INVALIDA);
    assert!(aplicar(&mut e.n, t + 2 * MIN, "res", id, res).aceptada());
    let k = e.n.caravana(ca).unwrap();
    assert_eq!((k.carros.len(), k.contenido[&Cobre], k.estado), (1, 30_000, EstadoCaravana::EnTransito), "pierde el carro y el 70 % de la carga, y sigue su viaje");
    assert_eq!(e.carga(atacante.0, Cobre), 70_000, "lo perdido, al carro de los atacantes");
    let s = e.n.heroe(carla).unwrap().heroe.escuadra(milicia).unwrap();
    assert_eq!((s.cantidad, s.contenedor, s.reserva_batalla), (0, militar::escuadra::Contenedor::Campamento, None), "la escolta cae: a 0 y de vuelta al campamento de su héroe");
    assert!(k.escolta.is_empty() && !e.n.ejercito(atacante.0).unwrap().en_batalla);
    assert!(!hechos(&e.n, "comercio.caravana_sale_de_batalla").is_empty());
    let _ = b;
}

#[test]
fn si_la_caravana_resiste_en_unity_conserva_sus_carros_su_escolta_sus_supervivientes_y_el_atacante_queda_herido() {
    let m = con_mundo!();
    let mut e = montar(m, 7);
    let a = e.a;
    let ca = e.caravana("ana", a, 2);
    let carla = e.heroe_en(&m.0, "carla", 1, a);
    let milicia = e.ok("admin", Comando::DarEscuadra { heroe: carla, tropa: Tropa::MiliciaLanceros, cantidad: 25 }).creadas[0];
    e.ok("carla", Comando::AsignarEscolta { caravana: ca, heroe: carla, escuadras: vec![milicia] });
    let t0 = e.lanzar(ca);
    e.n.avanzar_hasta(t0);
    let atacante = e.ejercito("beto", 2, e.pos(a), &[Tropa::ArquerosCompuesto]);
    assert!(e.atacar("beto", atacante.1, ObjetivoAtaque::Caravana(ca), true).aceptada());
    let id = e.n.batallas().next().unwrap().id;
    let t = e.n.ahora() + MIN;
    en_curso(&mut e.n, id, t, &[atacante.1]);
    let res = resultado(&e.n, id, Lado::Defensor, 5);
    assert!(aplicar(&mut e.n, t + MIN, "res", id, res).aceptada());
    // La caravana sigue su viaje intacta; la escolta, con los supervivientes que dijo Unity y todavía cedida; el atacante, herido y sin nada.
    let k = e.n.caravana(ca).unwrap();
    assert_eq!((k.carros.len(), k.contenido[&Cobre], k.estado, k.escolta.len()), (2, 100_000, EstadoCaravana::EnTransito, 1));
    let s = e.n.heroe(carla).unwrap().heroe.escuadra(milicia).unwrap();
    assert_eq!((s.cantidad, s.contenedor, s.reserva_batalla), (20, militar::escuadra::Contenedor::Escolta(ca), None));
    assert!(e.n.heroe_herido(atacante.1) && !e.n.ejercito(atacante.0).unwrap().en_batalla);
    assert_eq!(e.carga(atacante.0, Cobre), 0);
}

#[test]
fn si_unity_da_por_perdidos_todos_los_carros_la_caravana_desaparece() {
    let m = con_mundo!();
    let mut e = montar(m, 7);
    let (a, b) = (e.a, e.b);
    let ca = e.caravana("ana", a, 1);
    let t0 = e.lanzar(ca);
    let fin = e.n.caravana(ca).unwrap().plan.as_ref().unwrap().fin().unwrap();
    // A mitad de camino no hay zona de nadie: disputa de carros.
    e.n.avanzar_hasta(t0 + (fin - t0) / 2);
    let donde = e.n.posicion_de_caravana(ca, e.n.ahora()).unwrap();
    let atacante = e.ejercito("beto", 2, donde, &[Tropa::ArquerosCompuesto]);
    assert!(e.atacar("beto", atacante.1, ObjetivoAtaque::Caravana(ca), true).aceptada());
    let id = e.n.batallas().next().unwrap().id;
    let bt = e.n.batalla(id).unwrap();
    assert!(matches!(bt.ticket.contexto, combate::ciclo::Contexto::Caravana { modalidad: combate::caravana::Modalidad::Disputa, carros: 1, .. }));
    // Sin escolta defienden los carreteros, que no son de nadie (una escuadra sin dueño con id de la partida, fuera de cualquier héroe).
    let carreteros = &bt.ticket.defensor.sin_heroe[0];
    assert_eq!((carreteros.dueno, carreteros.efectivos, carreteros.tropa), (None, 13, Tropa::MiliciaLanceros));
    assert_eq!(e.n.escuadra(carreteros.id).map(|_| ()), None);
    let t = e.n.ahora() + MIN;
    en_curso(&mut e.n, id, t, &[atacante.1]);
    let mut res = resultado(&e.n, id, Lado::Atacante, 5);
    res.carros_perdidos = vec!["0".into()];
    assert!(aplicar(&mut e.n, t + MIN, "res", id, res).aceptada());
    assert!(e.n.caravana(ca).is_none());
    assert!(hechos(&e.n, "comercio.caravana_perdida").iter().any(|h| dato(h, "caravana") == ca as i64));
    assert_eq!(e.carga(atacante.0, Cobre), 100_000, "toda la carga, para quien la tomó");
    let _ = b;
}

// ───────────────────────────── Pueblos ─────────────────────────────

fn sitio_junto_a(n: &Nucleo, c: Punto, r: i64) -> Option<Punto> {
    (0..36).map(|g| ((g * 10) as f64).to_radians()).map(|a| Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64)).find(|&p| {
        n.viabilidad(p, 1).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6 && !v.condenada)
    })
}

#[test]
fn el_saqueo_de_un_pueblo_en_unity_se_lleva_lo_que_hay_sobre_la_mitad_y_la_guarnicion_se_repone_a_las_6_horas() {
    let m = con_mundo!();
    let mut n = Nucleo::nuevo(arranque_mundo(m, 7));
    let mapa = nucleo::economia::Mapa::de(&m.0);
    let (cobre, sitio) = (0..mapa.nodos.len())
        .filter(|&i| mapa.nodos[i].tipo == Tipo::MinaCobre)
        .find_map(|i| sitio_junto_a(&n, mapa.nodos[i].pos, 6_000).map(|p| (i as u32, p)))
        .expect("hay un nodo de cobre junto a un sitio rico");
    let madre = fundar(&mut n, 0, "m", "ana", 1, sitio);
    let mut e = Esc { n, a: madre, b: 0, cuenta: 0 };
    e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: madre, pausada: true });
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: madre, nivel: 2 });
    for _ in 0..9 {
        e.ok("admin", Comando::AgrandarZona { asentamiento: madre });
    }
    let pueblo = e.ok("ana", Comando::MarcarPueblo { asentamiento: madre, nodo: cobre }).creadas[0];
    let sitio = e.n.posicion_de_pueblo(pueblo).unwrap();
    let col = e.n.crear_ejercito(0, "ana", 1, sitio, 500, 100_000).unwrap();
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Madera, cantidad: 100_000 });
    e.ok("admin", Comando::DarSuministro { ejercito: col, recurso: Oro, cantidad: 12_000 });
    e.ok("ana", Comando::EntregarMateriales { ejercito: col, pueblo });
    let EstadoPueblo::EnObra { completa_en } = e.n.pueblo(pueblo).unwrap().1.estado else { panic!("debía estar en obra") };
    e.n.avanzar_hasta(completa_en + HORA);
    let guardado = e.n.pueblo(pueblo).unwrap().1.almacen.cantidad(Cobre);
    let saqueador = e.ejercito("beto", 2, sitio, &[Tropa::ArquerosCompuesto, Tropa::EspadachinesBronce]);
    let t = e.n.ahora();
    assert!(e.atacar("beto", saqueador.1, ObjetivoAtaque::Pueblo(pueblo), true).aceptada());
    let id = e.n.batallas().next().unwrap().id;
    let bt = e.n.batalla(id).unwrap();
    assert!(matches!(bt.ticket.contexto, combate::ciclo::Contexto::SaqueoPueblo { pueblo: p, .. } if p == pueblo));
    assert_eq!((bt.ticket.defensor.faccion, bt.ticket.defensor.sin_heroe.len(), bt.ticket.defensor.sin_heroe[0].efectivos), (Some(1), 1, 30), "los 30 mineros de serie, con la Facción de la plaza madre");
    assert_eq!(bt.bloqueo.pueblo, Some(pueblo));
    assert_eq!(codigo(&e.ord("ana", Comando::DestruirPueblo { pueblo })), rechazos::BATALLA_BLOQUEADO, "un pueblo en una batalla no recibe órdenes");
    let otro = e.ejercito("gus", 3, sitio, &[Tropa::EspadachinesBronce]);
    assert_eq!(codigo(&e.atacar("gus", otro.1, ObjetivoAtaque::Pueblo(pueblo), true)), rechazos::BATALLA_BLOQUEADO);
    en_curso(&mut e.n, id, t + MIN, &[saqueador.1]);
    let res = resultado(&e.n, id, Lado::Atacante, 10);
    assert!(aplicar(&mut e.n, t + 2 * MIN, "res", id, res).aceptada());
    // Se lleva lo que está por encima de la mitad; la guarnición de serie queda en 20 y se repone a las 6 horas.
    assert_eq!(e.carga(saqueador.0, Cobre), guardado - 250_000, "lo saqueado");
    assert_eq!(e.n.pueblo(pueblo).unwrap().1.almacen.cantidad(Cobre), 250_000, "el pueblo conserva la mitad protegida");
    let g = &e.n.estado().combate.guarniciones[&pueblo];
    assert_eq!((g.escuadra.cantidad, g.repone_en), (20, Some(t + 2 * MIN + 6 * HORA)));
    assert_eq!(e.n.estado().asentamientos[&madre].faccion, 1);
    e.n.avanzar_hasta(t + 2 * MIN + 6 * HORA);
    assert_eq!(e.n.estado().combate.guarniciones[&pueblo].escuadra.cantidad, 30);
}
