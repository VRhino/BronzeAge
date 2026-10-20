use super::*;
use crate::resolucion::apoyo::r;
use crate::resultado::{Botin, ObjetoBotin, Razon, ResultadoEscuadra, ResultadoHeroe};
use base::MS_POR_MINUTO;

const MIN: i64 = MS_POR_MINUTO;
const ASEDIO: Contexto = Contexto::Asedio { asentamiento: 9 };
const CAMPAL: Contexto = Contexto::CampoAbierto { punto: Punto { x: 0, y: 0 }, solitarios: false };

fn part(heroe: HeroeId, controlador: Controlador, escuadras: &[(EscuadraId, i64)]) -> Participante {
    Participante {
        heroe,
        faccion: Some(heroe as u32),
        controlador,
        casillas_libres: 2,
        escuadras: escuadras.iter().map(|&(id, efectivos)| EscuadraEnBatalla { id, tropa: Tropa::MiliciaLanceros, dueno: Some(heroe), efectivos }).collect(),
    }
}

/// Atacante: héroe 1 (humano, escuadra 10 de 25). Defensor: héroe 2 (bot, escuadra 20 de 25) y una escuadra sin dueño (30, 13).
fn ticket(contexto: Contexto) -> Ticket {
    Ticket::nuevo(
        r(),
        contexto,
        BandoDeTicket::nuevo(Some(1), vec![part(1, Controlador::Humano, &[(10, 25)])], vec![]),
        BandoDeTicket::nuevo(Some(2), vec![part(2, Controlador::Bot, &[(20, 25)])], vec![EscuadraEnBatalla { id: 30, tropa: Tropa::MiliciaLanceros, dueno: None, efectivos: 13 }]),
    )
}

fn batalla(contexto: Contexto) -> Batalla {
    Batalla::abrir(r(), 1, 1, Punto::default(), ticket(contexto), Bloqueo { ejercitos: vec![100, 200], caravanas: vec![5], asentamiento: Some(9), ..Default::default() }, 1_000)
}

fn asignacion(intento: &str, servidor: &str, revision: u32, tokens: &[HeroeId]) -> Asignacion {
    Asignacion {
        intento: intento.into(),
        servidor: servidor.into(),
        revision,
        instancia: Instancia { host: "10.0.0.1".into(), puerto: 7777, protocolo: "udp".into() },
        tokens: tokens.iter().map(|&heroe| Token { heroe, token: format!("t{heroe}"), expira_en: 99 }).collect(),
    }
}

/// Una batalla asignada y en curso.
fn en_curso(contexto: Contexto) -> Batalla {
    let mut b = batalla(contexto);
    let t = if contexto.tiene_convocatoria() { 1_000 + 30_000 } else { 1_000 };
    b.registrar_asignacion(r(), t, asignacion("a1", "srv", 0, &[1])).unwrap();
    b.confirmar_inicio(r(), t + 10, "a1", "srv", 0).unwrap();
    b
}

/// Un resultado válido para `b`: todos sobreviven menos 5 por escuadra; gana el atacante.
fn resultado(b: &Batalla) -> Resultado {
    Resultado {
        resultado_id: "res-1".into(),
        revision: b.ticket.revision,
        intento: "a1".into(),
        inicio: 10_000,
        fin: 70_000,
        ganador: Lado::Atacante,
        razon: Razon::ObjetivosCapturados,
        objetivos: vec![("bandera".into(), Some(Lado::Atacante))],
        por_escuadra: b
            .escuadras_de()
            .map(|e| ResultadoEscuadra { escuadra: e.id, desplegados: e.efectivos, supervivientes_al_cierre: e.efectivos - 5, muertos: 5, xp_ganada: 3 })
            .collect(),
        por_heroe: b.heroes_que_combaten().map(|h| ResultadoHeroe { heroe: h, participo: true, sobrevivio_al_cierre: true, xp_ganada: 7, derrotado: None, botin: None }).collect(),
        version_servidor: "v1".into(),
        carros_perdidos: vec![],
    }
}

fn sin_objetos(_: &str) -> bool {
    false
}

#[test]
fn el_contexto_decide_capacidad_duracion_y_convocatoria() {
    let r = r();
    let p = Punto::default();
    let asedio_like = [ASEDIO, Contexto::Avanzada { avanzada: 1, etapa: Etapa::Fuerte, punto: p }];
    let resto = [
        CAMPAL,
        Contexto::CampoAbierto { punto: p, solitarios: true },
        Contexto::Caravana { caravana: 1, punto: p, modalidad: Modalidad::Disputa, carros: 3 },
        Contexto::CampamentoBandidos { campamento: 1, punto: p },
        Contexto::SaqueoPueblo { pueblo: 1, punto: p },
    ];
    for c in asedio_like {
        assert_eq!((c.capacidad(r), c.duracion(r)), (15, 30 * MIN));
    }
    for c in resto {
        assert_eq!((c.capacidad(r), c.duracion(r)), (5, 15 * MIN), "{c:?}: también el saqueo de un pueblo");
        assert!(!c.tiene_convocatoria());
    }
    assert!(ASEDIO.tiene_convocatoria());
}

#[test]
fn abrir_congela_el_ticket_y_fija_el_plazo_de_asignacion() {
    let b = batalla(CAMPAL);
    assert_eq!((b.estado, b.ticket.revision, b.convocatoria_hasta), (EstadoBatalla::Convocando, 0, None));
    assert_eq!(b.expira_en, 1_000 + 5 * MIN);
    assert_eq!((b.ticket.atacante.capacidad, b.ticket.defensor.capacidad, b.ticket.duracion_maxima), (5, 5, 15 * MIN));
    let a = batalla(ASEDIO);
    assert_eq!(a.convocatoria_hasta, Some(1_000 + 30_000));
    assert_eq!(a.expira_en, 1_000 + 30_000 + 5 * MIN, "el plazo de asignación corre desde el fin de la convocatoria");
    assert_eq!((a.ticket.atacante.capacidad, a.ticket.duracion_maxima), (15, 30 * MIN));
}

#[test]
fn el_ticket_no_se_recorta_por_capacidad_al_abrir() {
    let muchos: Vec<Participante> = (1..=7).map(|h| part(h, Controlador::Humano, &[])).collect();
    let t = Ticket::nuevo(r(), CAMPAL, BandoDeTicket::nuevo(Some(1), muchos, vec![]), BandoDeTicket::nuevo(None, vec![], vec![]));
    assert_eq!((t.atacante.participantes.len(), t.atacante.capacidad), (7, 5), "los que sobran esperan en la cola de Unity");
}

#[test]
fn solo_una_convocando_fuera_de_convocatoria_se_ofrece_al_servidor_de_batalla() {
    let a = batalla(ASEDIO);
    assert!(!a.pendiente_de_asignar(1_000 + 29_999));
    assert!(a.pendiente_de_asignar(1_000 + 30_000));
    assert!(!a.pendiente_de_asignar(a.expira_en), "ni una vencida");
    assert!(batalla(CAMPAL).pendiente_de_asignar(1_000));
}

#[test]
fn el_candado_es_de_las_escuadras_con_dueno() {
    let b = batalla(CAMPAL);
    assert_eq!(b.escuadras_con_candado(), vec![10, 20], "la 30, sin dueño (bandidos, carreteros), no se bloquea");
    assert_eq!(b.escuadras_de().count(), 3);
    assert_eq!(b.heroes_que_combaten().collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!((b.lado_de(1), b.lado_de(2), b.lado_de(9)), (Some(Lado::Atacante), Some(Lado::Defensor), None));
}

#[test]
fn vencer_cierra_sin_castigo_y_suelta_los_candados() {
    for estado in [0, 1, 2] {
        let mut b = if estado == 0 { batalla(CAMPAL) } else { en_curso(CAMPAL) };
        if estado == 1 {
            b.estado = EstadoBatalla::Asignada;
        }
        assert_eq!(b.vencer(b.expira_en - 1), None, "antes de su plazo no vence");
        assert_eq!(b.vencer(b.expira_en), Some(vec![10, 20]));
        assert_eq!((b.estado, b.cerrada_en), (EstadoBatalla::Fallida, Some(b.expira_en)));
        assert_eq!(b.vencer(b.expira_en + 1), None, "una cerrada no vuelve a vencer");
        assert!(!b.activa(b.expira_en - 1), "cerrada ya no está activa");
    }
}

#[test]
fn una_vencida_deja_de_ser_activa_aunque_su_evento_no_haya_pasado() {
    let b = batalla(CAMPAL);
    assert!(b.activa(b.expira_en - 1) && !b.activa(b.expira_en));
    assert_eq!(candados_vencidos(&[b.clone()], b.expira_en - 1), Vec::<EscuadraId>::new());
    assert_eq!(candados_vencidos(&[b.clone()], b.expira_en), vec![10, 20]);
    assert_eq!(b.estado, EstadoBatalla::Convocando, "no se cierra: eso es del evento");
    let mut cerrada = b.clone();
    cerrada.vencer(b.expira_en);
    assert!(candados_vencidos(&[cerrada], b.expira_en + 1).is_empty(), "una ya cerrada no vuelve a soltar");
}

#[test]
fn los_bloqueos_salen_solo_de_las_activas() {
    let mut otra = batalla(CAMPAL);
    otra.id = 2;
    otra.bloqueo = Bloqueo { ejercitos: vec![300], campamento: Some(4), pueblo: Some(6), avanzada: Some(8), ..Default::default() };
    otra.expira_en = 50;
    let b = batalla(ASEDIO);
    let bl = bloqueos_de(&[b.clone(), otra.clone()], 1_001);
    assert_eq!(bl.ejercitos, BTreeSet::from([100, 200]));
    assert_eq!((bl.caravanas, bl.asentamientos, bl.heroes), (BTreeSet::from([5]), BTreeSet::from([9]), BTreeSet::from([1, 2])));
    assert!(bl.campamentos.is_empty() && bl.pueblos.is_empty(), "la vencida no bloquea");
    otra.expira_en = 5_000;
    let bl = bloqueos_de(&[otra], 1_001);
    assert_eq!((bl.campamentos, bl.pueblos, bl.avanzadas), (BTreeSet::from([4]), BTreeSet::from([6]), BTreeSet::from([8])));
}

#[test]
fn quien_esta_dentro_de_la_plaza_asediada_tampoco_da_ordenes() {
    let bl = bloqueos_de(&[batalla(ASEDIO)], 1_001);
    assert!(bl.heroe_bloqueado(1, None), "combate");
    assert!(bl.heroe_bloqueado(77, Some(9)), "dentro de la plaza asediada");
    assert!(!bl.heroe_bloqueado(77, Some(3)));
    assert!(!bl.heroe_bloqueado(77, None));
}

#[test]
fn unirse_en_la_convocatoria_entra_en_el_ticket_sin_subir_la_revision() {
    let mut b = batalla(ASEDIO);
    let sec = b.unirse(1_010, Lado::Defensor, vec![part(3, Controlador::Humano, &[(40, 25)])], 300).unwrap();
    assert!(sec.is_empty() && b.incorporaciones.is_empty());
    assert_eq!((b.ticket.defensor.participantes.len(), b.ticket.revision), (2, 0));
    assert!(b.bloqueo.ejercitos.contains(&300) && b.escuadras_con_candado().contains(&40));
    b.unirse(1_011, Lado::Defensor, vec![part(4, Controlador::Bot, &[])], 300).unwrap();
    assert_eq!(b.bloqueo.ejercitos.iter().filter(|&&c| c == 300).count(), 1, "la columna no se duplica");
}

#[test]
fn unirse_despues_es_una_incorporacion_con_secuencia_y_el_ticket_no_cambia() {
    let mut b = batalla(CAMPAL);
    let antes = b.ticket.clone();
    let s1 = b.unirse(2_000, Lado::Atacante, vec![part(3, Controlador::Humano, &[(40, 25)]), part(4, Controlador::Humano, &[])], 300).unwrap();
    let s2 = b.unirse(2_001, Lado::Atacante, vec![part(5, Controlador::Bot, &[])], 301).unwrap();
    assert_eq!((s1, s2), (vec![1, 2], vec![3]));
    assert_eq!(b.ticket, antes, "ni cambia ni sube de revisión");
    assert_eq!(b.incorporaciones.iter().map(|i| (i.secuencia, i.lado, i.participante.heroe)).collect::<Vec<_>>(), vec![(1, Lado::Atacante, 3), (2, Lado::Atacante, 4), (3, Lado::Atacante, 5)]);
    assert_eq!(b.lado_de(5), Some(Lado::Atacante));
    assert!(b.escuadras_con_candado().contains(&40));
}

#[test]
fn un_bando_lleno_no_admite_a_nadie_y_la_cola_es_solo_de_los_del_principio() {
    let mut b = batalla(CAMPAL); // 1 atacante dentro, capacidad 5
    b.unirse(2_000, Lado::Atacante, (3..=6).map(|h| part(h, Controlador::Bot, &[])).collect(), 300).unwrap();
    assert_eq!(b.unirse(2_001, Lado::Atacante, vec![part(7, Controlador::Bot, &[])], 301), Err(ErrorBatalla::LadoLleno));
    assert!(!b.bloqueo.ejercitos.contains(&301), "rechazado no bloquea nada");
    assert!(b.unirse(2_001, Lado::Defensor, vec![part(7, Controlador::Bot, &[])], 301).is_ok(), "el otro bando tiene sitio");
}

#[test]
fn un_heroe_no_entra_dos_veces() {
    let mut b = batalla(CAMPAL);
    assert_eq!(b.unirse(2_000, Lado::Defensor, vec![part(1, Controlador::Bot, &[])], 300), Err(ErrorBatalla::YaCombate(1)), "ya combate en el otro bando");
    assert_eq!(b.unirse(2_000, Lado::Atacante, vec![part(3, Controlador::Bot, &[]), part(3, Controlador::Bot, &[])], 300), Err(ErrorBatalla::YaCombate(3)), "repetido en la misma columna");
    assert!(b.incorporaciones.is_empty() && !b.bloqueo.ejercitos.contains(&300));
}

#[test]
fn no_se_une_a_una_cerrada_ni_vencida_ni_quien_ya_salio() {
    let mut b = batalla(CAMPAL);
    assert_eq!(b.unirse(b.expira_en, Lado::Atacante, vec![part(3, Controlador::Bot, &[])], 300), Err(ErrorBatalla::NoActiva));
    b.salidas.push(Salida { heroe: 3, motivo: MotivoSalida::NoConectado, derrotado: false, en: 1 });
    assert_eq!(b.unirse(2_000, Lado::Atacante, vec![part(3, Controlador::Bot, &[])], 300), Err(ErrorBatalla::YaSalio(3)));
}

#[test]
fn la_asignacion_pasa_a_asignada_con_el_plazo_de_inicio_y_repetirla_no_cambia_nada() {
    let mut b = batalla(CAMPAL);
    let a = asignacion("a1", "srv", 0, &[1]);
    assert_eq!(b.registrar_asignacion(r(), 2_000, a.clone()), Ok(Efecto::Aplicado(())));
    assert_eq!((b.estado, b.expira_en), (EstadoBatalla::Asignada, 2_000 + 5 * MIN));
    let foto = b.clone();
    assert_eq!(b.registrar_asignacion(r(), 3_000, a), Ok(Efecto::SinCambios));
    assert_eq!(b, foto, "sin cambios de verdad");
    assert_eq!(b.registrar_asignacion(r(), 3_000, asignacion("a2", "srv", 0, &[1])), Err(ErrorBatalla::YaAsignada), "otro intento");
    assert_eq!(b.registrar_asignacion(r(), 3_000, asignacion("a1", "otro", 0, &[1])), Err(ErrorBatalla::YaAsignada), "otro servidor: gana el primero");
}

#[test]
fn la_asignacion_exige_fuera_de_convocatoria_revision_vigente_y_tokens_de_humanos_de_la_batalla() {
    let mut a = batalla(ASEDIO);
    assert_eq!(a.registrar_asignacion(r(), 1_010, asignacion("a1", "srv", 0, &[1])), Err(ErrorBatalla::EnConvocatoria));
    let mut b = batalla(CAMPAL);
    assert_eq!(b.registrar_asignacion(r(), 2_000, asignacion("a1", "srv", 1, &[1])), Err(ErrorBatalla::RevisionVieja));
    assert_eq!(b.registrar_asignacion(r(), 2_000, asignacion("a1", "srv", 0, &[2])), Err(ErrorBatalla::TokenInvalido(2)), "los bots no se conectan");
    assert_eq!(b.registrar_asignacion(r(), 2_000, asignacion("a1", "srv", 0, &[1, 9])), Err(ErrorBatalla::TokenInvalido(9)), "ni quien no combate");
    assert_eq!(b.estado, EstadoBatalla::Convocando, "un rechazo no cambia nada");
    assert_eq!(b.registrar_asignacion(r(), b.expira_en, asignacion("a1", "srv", 0, &[1])), Err(ErrorBatalla::NoActiva));
}

#[test]
fn el_inicio_lo_confirma_solo_el_servidor_de_la_asignacion_y_fija_el_plazo_de_la_partida() {
    let mut b = batalla(CAMPAL);
    assert_eq!(b.confirmar_inicio(r(), 2_000, "a1", "srv", 0), Err(ErrorBatalla::ServidorAjeno), "sin asignación");
    b.registrar_asignacion(r(), 2_000, asignacion("a1", "srv", 0, &[1])).unwrap();
    assert_eq!(b.confirmar_inicio(r(), 2_100, "a1", "otro", 0), Err(ErrorBatalla::ServidorAjeno));
    assert_eq!(b.confirmar_inicio(r(), 2_100, "a9", "srv", 0), Err(ErrorBatalla::ServidorAjeno));
    assert_eq!(b.confirmar_inicio(r(), 2_100, "a1", "srv", 3), Err(ErrorBatalla::RevisionVieja));
    assert_eq!(b.confirmar_inicio(r(), 2_100, "a1", "srv", 0), Ok(Efecto::Aplicado(())));
    assert_eq!((b.estado, b.iniciada_en, b.expira_en), (EstadoBatalla::EnCurso, Some(2_100), 2_100 + 15 * MIN + 5 * MIN), "duración máxima + margen");
    assert_eq!(b.confirmar_inicio(r(), 2_200, "a1", "srv", 0), Ok(Efecto::SinCambios));
}

#[test]
fn un_asedio_dura_30_minutos_mas_el_margen() {
    let b = en_curso(ASEDIO);
    assert_eq!(b.expira_en, b.iniciada_en.unwrap() + 30 * MIN + 5 * MIN);
}

#[test]
fn un_token_nuevo_del_mismo_heroe_sustituye_al_anterior() {
    let mut b = en_curso(CAMPAL);
    b.unirse(3_000, Lado::Atacante, vec![part(3, Controlador::Humano, &[])], 300).unwrap();
    let nuevo = |h, t: &str| Token { heroe: h, token: t.into(), expira_en: 1 };
    b.registrar_tokens(3_001, "a1", "srv", vec![nuevo(3, "x"), nuevo(1, "y")]).unwrap();
    let toks = &b.asignacion.as_ref().unwrap().tokens;
    assert_eq!(toks.len(), 2);
    assert_eq!(toks.iter().find(|t| t.heroe == 1).unwrap().token, "y");
    assert_eq!(b.registrar_tokens(3_002, "a1", "srv", vec![nuevo(2, "bot")]), Err(ErrorBatalla::TokenInvalido(2)));
    assert_eq!(b.registrar_tokens(3_002, "a1", "otro", vec![]), Err(ErrorBatalla::ServidorAjeno));
    let mut sin_asignar = batalla(CAMPAL);
    assert_eq!(sin_asignar.registrar_tokens(2_000, "a1", "srv", vec![]), Err(ErrorBatalla::ServidorAjeno));
}

fn salida(heroe: HeroeId, motivo: MotivoSalida, derrotado: bool, escuadras: &[(EscuadraId, i64, i64)], revision: u32) -> SalidaBatalla {
    SalidaBatalla {
        intento: "a1".into(),
        revision,
        heroe,
        motivo,
        derrotado,
        escuadras: escuadras.iter().map(|&(escuadra, supervivientes, muertos)| EscuadraDeSalida { escuadra, supervivientes, muertos }).collect(),
    }
}

fn asignada_con_tres() -> Batalla {
    let mut b = batalla(CAMPAL);
    b.unirse(1_500, Lado::Atacante, vec![part(3, Controlador::Humano, &[(40, 25)])], 300).unwrap();
    b.registrar_asignacion(r(), 2_000, asignacion("a1", "srv", 0, &[1, 3])).unwrap();
    b
}

#[test]
fn no_conectado_antes_de_empezar_sube_la_revision_reabre_y_no_castiga() {
    let mut b = asignada_con_tres();
    let Ok(Efecto::Aplicado(s)) = b.registrar_salida(r(), 2_500, "srv", salida(3, MotivoSalida::NoConectado, false, &[], 0)) else { panic!() };
    assert_eq!((s.liberar.clone(), s.herido, s.cancelada, s.reabierta), (vec![40], false, false, true));
    assert!(s.bajas.is_empty(), "sus escuadras quedan intactas");
    assert_eq!((b.estado, b.ticket.revision, b.asignacion.is_none(), b.expira_en), (EstadoBatalla::Convocando, 1, true, 2_500 + 5 * MIN));
    assert!(b.participante(3).is_none() && b.incorporaciones.is_empty(), "su participación se quita y no entra nadie en su lugar");
    assert!(!b.escuadras_con_candado().contains(&40));
    // Repetir la salida de quien ya salió no cambia nada, aunque la revisión y la asignación ya no sean las del mensaje.
    let foto = b.clone();
    assert_eq!(b.registrar_salida(r(), 2_600, "srv", salida(3, MotivoSalida::NoConectado, false, &[], 0)), Ok(Efecto::SinCambios));
    assert_eq!(b, foto);
}

#[test]
fn la_asignacion_vieja_deja_de_valer_tras_una_salida_previa() {
    let mut b = asignada_con_tres();
    b.registrar_salida(r(), 2_500, "srv", salida(3, MotivoSalida::NoConectado, false, &[], 0)).unwrap();
    assert_eq!(b.confirmar_inicio(r(), 2_600, "a1", "srv", 0), Err(ErrorBatalla::ServidorAjeno));
    assert_eq!(b.registrar_asignacion(r(), 2_600, asignacion("a1", "srv", 0, &[1])), Err(ErrorBatalla::RevisionVieja), "la revisión vigente es la 1");
    assert!(b.registrar_asignacion(r(), 2_600, asignacion("a2", "srv", 1, &[1])).is_ok());
}

#[test]
fn si_no_queda_ningun_humano_o_el_atacante_se_queda_sin_heroes_se_cancela_sin_castigo() {
    let mut b = batalla(CAMPAL);
    b.registrar_asignacion(r(), 2_000, asignacion("a1", "srv", 0, &[1])).unwrap();
    let Ok(Efecto::Aplicado(s)) = b.registrar_salida(r(), 2_100, "srv", salida(1, MotivoSalida::NoConectado, false, &[], 0)) else { panic!() };
    assert!(s.cancelada && !s.reabierta && !s.herido);
    assert_eq!((b.estado, b.cerrada_en), (EstadoBatalla::Cancelada, Some(2_100)));
    assert_eq!(s.liberar, vec![20, 10], "todos los candados");

    // Con humanos en el otro bando pero sin ningún héroe atacante tampoco sigue.
    let mut c = Batalla::abrir(
        r(),
        2,
        1,
        Punto::default(),
        Ticket::nuevo(r(), CAMPAL, BandoDeTicket::nuevo(Some(1), vec![part(1, Controlador::Humano, &[(10, 25)])], vec![]), BandoDeTicket::nuevo(Some(2), vec![part(2, Controlador::Humano, &[])], vec![])),
        Bloqueo::default(),
        1_000,
    );
    c.registrar_asignacion(r(), 2_000, asignacion("a1", "srv", 0, &[1, 2])).unwrap();
    let Ok(Efecto::Aplicado(s)) = c.registrar_salida(r(), 2_100, "srv", salida(1, MotivoSalida::NoConectado, false, &[], 0)) else { panic!() };
    assert!(s.cancelada, "queda un humano, pero el atacante se quedó sin héroes");
}

#[test]
fn eliminado_en_curso_aplica_las_bajas_hiere_si_derrotado_y_no_toca_el_ticket() {
    let mut b = en_curso(CAMPAL);
    b.unirse(3_000, Lado::Atacante, vec![part(3, Controlador::Humano, &[(40, 25)])], 300).unwrap();
    let ticket = b.ticket.clone();
    let Ok(Efecto::Aplicado(s)) = b.registrar_salida(r(), 4_000, "srv", salida(3, MotivoSalida::Eliminado, true, &[(40, 9, 16)], 0)) else { panic!() };
    assert_eq!((s.bajas, s.liberar, s.herido, s.cancelada, s.reabierta), (vec![(40, 9)], vec![40], true, false, false));
    assert_eq!((b.estado, b.ticket == ticket), (EstadoBatalla::EnCurso, true));
    assert!(b.salio(3) && b.participante(3).is_none());
    assert_eq!(b.salidas[0], Salida { heroe: 3, motivo: MotivoSalida::Eliminado, derrotado: true, en: 4_000 });
    // El resultado final ya no lo repite: ni su héroe ni sus escuadras.
    let res = resultado(&b);
    assert!(res.por_heroe.iter().all(|h| h.heroe != 3) && res.por_escuadra.iter().all(|e| e.escuadra != 40));
    assert_eq!(b.validar_resultado("srv", &res, &sin_objetos), Ok(()));
}

#[test]
fn una_salida_con_escuadras_ajenas_repetidas_o_que_no_cierran_se_rechaza_sin_cambios() {
    let mut b = en_curso(CAMPAL);
    let foto = b.clone();
    for escuadras in [&[(20, 20, 5)][..], &[(10, 10, 15), (10, 10, 15)], &[(10, 10, 14)], &[(10, -1, 26)]] {
        assert_eq!(b.registrar_salida(r(), 4_000, "srv", salida(1, MotivoSalida::Abandono, false, escuadras, 0)), Err(ErrorBatalla::SalidaInvalida), "{escuadras:?}");
    }
    assert_eq!(b, foto);
}

#[test]
fn el_motivo_tiene_que_corresponder_al_estado_y_el_heroe_combatir() {
    let mut asignada = asignada_con_tres();
    assert_eq!(asignada.registrar_salida(r(), 2_100, "srv", salida(3, MotivoSalida::Eliminado, true, &[], 0)), Err(ErrorBatalla::EstadoInvalido), "eliminado solo con la partida en curso");
    let mut curso = en_curso(CAMPAL);
    assert_eq!(curso.registrar_salida(r(), 4_000, "srv", salida(1, MotivoSalida::NoConectado, false, &[], 0)), Err(ErrorBatalla::EstadoInvalido), "no_conectado solo antes de empezar");
    assert_eq!(curso.registrar_salida(r(), 4_000, "srv", salida(9, MotivoSalida::Abandono, false, &[], 0)), Err(ErrorBatalla::NoCombate(9)));
    assert_eq!(curso.registrar_salida(r(), 4_000, "otro", salida(1, MotivoSalida::Abandono, false, &[], 0)), Err(ErrorBatalla::ServidorAjeno));
    assert_eq!(curso.registrar_salida(r(), 4_000, "srv", salida(1, MotivoSalida::Abandono, false, &[], 7)), Err(ErrorBatalla::RevisionVieja));
}

#[test]
fn si_sale_quien_la_inicio_pasa_al_primer_atacante_que_queda() {
    let mut b = en_curso(CAMPAL);
    b.unirse(3_000, Lado::Atacante, vec![part(3, Controlador::Humano, &[]), part(4, Controlador::Bot, &[])], 300).unwrap();
    b.registrar_salida(r(), 4_000, "srv", salida(1, MotivoSalida::Desconexion, false, &[], 0)).unwrap();
    assert_eq!(b.iniciada_por, 3);
    b.registrar_salida(r(), 4_001, "srv", salida(2, MotivoSalida::Abandono, false, &[], 0)).unwrap();
    assert_eq!(b.iniciada_por, 3, "si sale otro, no cambia");
}

#[test]
fn quien_la_inicio_la_cancela_antes_de_empezar_y_suelta_los_candados() {
    let mut b = batalla(ASEDIO);
    assert_eq!(b.cancelar(1_010, 2), Err(ErrorBatalla::NoPuedeCancelar), "solo quien la inició");
    assert_eq!(b.cancelar(1_010, 1), Ok(vec![10, 20]));
    assert_eq!((b.estado, b.cerrada_en), (EstadoBatalla::Cancelada, Some(1_010)));
    assert_eq!(b.cancelar(1_011, 1), Err(ErrorBatalla::NoActiva));
    let mut curso = en_curso(CAMPAL);
    assert_eq!(curso.cancelar(4_000, 1), Err(ErrorBatalla::NoPuedeCancelar), "empezada la partida, ya no");
    let mut asignada = asignada_con_tres();
    assert!(asignada.cancelar(2_100, 1).is_ok(), "asignada todavía se puede");
}

#[test]
fn un_resultado_valido_se_aplica_en_una_mutacion_y_devuelve_los_efectos() {
    let mut b = en_curso(CAMPAL);
    let res = resultado(&b);
    let Ok(Efecto::Aplicado(e)) = b.aplicar_resultado(5_000, "srv", res.clone(), &sin_objetos) else { panic!() };
    assert_eq!((b.estado, b.aplicada_en, b.cerrada_en, b.resultado.as_ref()), (EstadoBatalla::Aplicada, Some(5_000), Some(5_000), Some(&res)));
    assert_eq!(e.ganador, Lado::Atacante);
    assert_eq!(e.heridos, vec![2], "sin `derrotado`, el bando perdedor");
    assert_eq!(e.soltar, vec![10, 20]);
    let por_id = |id| e.escuadras.iter().find(|x| x.escuadra == id).unwrap();
    assert_eq!((por_id(10).cantidad, por_id(10).xp, por_id(30).cantidad), (20, 3_000, 8), "xp en milésimas; la sin dueño también viene");
    assert!(e.heroes.iter().all(|h| h.xp == 7 && h.botin.is_none()));
}

#[test]
fn el_herido_sale_de_derrotado_y_no_del_bando() {
    let mut b = en_curso(CAMPAL);
    let mut res = resultado(&b);
    res.por_heroe[0].derrotado = Some(true); // gana el atacante, pero Unity lo da por derrotado
    res.por_heroe[1].derrotado = Some(false); // pierde el defensor, pero explícitamente no
    let Ok(Efecto::Aplicado(e)) = b.aplicar_resultado(5_000, "srv", res, &sin_objetos) else { panic!() };
    assert_eq!(e.heridos, vec![1]);
}

#[test]
fn repetir_el_mismo_resultado_no_cambia_nada_y_otro_distinto_se_rechaza() {
    let mut b = en_curso(CAMPAL);
    let res = resultado(&b);
    b.aplicar_resultado(5_000, "srv", res.clone(), &sin_objetos).unwrap();
    let foto = b.clone();
    assert_eq!(b.aplicar_resultado(9_000, "srv", res.clone(), &sin_objetos), Ok(Efecto::SinCambios));
    assert_eq!(b, foto);
    let mut otro = res;
    otro.resultado_id = "res-2".into();
    assert_eq!(b.aplicar_resultado(9_000, "srv", otro, &sin_objetos), Err(ErrorBatalla::OtroResultado));
    assert_eq!(b, foto);
}

#[test]
fn sobre_una_cancelada_fallida_o_sin_empezar_se_rechaza_y_una_vencida_en_curso_tambien() {
    let res = resultado(&en_curso(CAMPAL));
    let mut cancelada = batalla(CAMPAL);
    cancelada.cancelar(1_100, 1).unwrap();
    assert_eq!(cancelada.aplicar_resultado(1_200, "srv", res.clone(), &sin_objetos), Err(ErrorBatalla::EstadoInvalido));
    assert_eq!(batalla(CAMPAL).aplicar_resultado(1_200, "srv", res.clone(), &sin_objetos), Err(ErrorBatalla::EstadoInvalido), "convocando");
    let mut b = en_curso(CAMPAL);
    let tarde = b.expira_en;
    assert_eq!(b.aplicar_resultado(tarde, "srv", res, &sin_objetos), Err(ErrorBatalla::NoActiva), "fuera de plazo");
    assert_eq!(b.estado, EstadoBatalla::EnCurso);
}

fn rechazo(modifica: impl FnOnce(&mut Resultado)) -> ErrorResultado {
    let b = en_curso(CAMPAL);
    let mut res = resultado(&b);
    modifica(&mut res);
    b.validar_resultado("srv", &res, &|i| i == "existente").unwrap_err()
}

#[test]
fn el_checklist_rechaza_cada_fallo_y_nunca_aplica_a_medias() {
    use ErrorResultado::*;
    assert_eq!(rechazo(|r| r.intento = "otro".into()), AsignacionNoVigente);
    assert_eq!(rechazo(|r| r.revision = 4), AsignacionNoVigente);
    assert_eq!(rechazo(|r| r.por_escuadra[0].muertos = -1), Forma);
    assert_eq!(rechazo(|r| r.por_heroe[0].xp_ganada = -1), Forma);
    assert_eq!(rechazo(|r| r.por_escuadra[1] = r.por_escuadra[0].clone()), IdRepetido);
    assert_eq!(rechazo(|r| r.por_heroe[1] = r.por_heroe[0].clone()), IdRepetido);
    assert_eq!(rechazo(|r| drop(r.por_escuadra.pop())), EscuadrasIncompletas);
    assert_eq!(rechazo(|r| r.por_escuadra[0].escuadra = 99), EscuadrasIncompletas);
    assert_eq!(rechazo(|r| drop(r.por_heroe.pop())), HeroesIncompletos);
    assert_eq!(rechazo(|r| r.por_escuadra[0].supervivientes_al_cierre += 1), EscuadraNoCierra(10));
    assert_eq!(rechazo(|r| r.por_escuadra[0].desplegados = 26), EscuadraNoCierra(10));
    assert_eq!(rechazo(|r| r.razon = Razon::TiempoAgotado), TiempoAgotadoSinDefensor);
    assert_eq!(rechazo(|r| r.fin = r.inicio), Duracion);
    assert_eq!(rechazo(|r| r.fin = r.inicio + 15 * MIN + 1), Duracion);
}

#[test]
fn el_tiempo_agotado_lo_gana_el_defensor_y_la_duracion_maxima_es_valida() {
    let b = en_curso(CAMPAL);
    let mut res = resultado(&b);
    res.razon = Razon::TiempoAgotado;
    res.ganador = Lado::Defensor;
    res.fin = res.inicio + 15 * MIN;
    assert_eq!(b.validar_resultado("srv", &res, &sin_objetos), Ok(()));
    let efectos = b.efectos_del_resultado(&res);
    assert_eq!(efectos.heridos, vec![1], "pierde el atacante");
}

#[test]
fn los_carros_perdidos_son_posiciones_de_la_lista_de_una_caravana_sin_repetir() {
    use ErrorResultado::CarroInexistente;
    let caravana = Contexto::Caravana { caravana: 1, punto: Punto::default(), modalidad: Modalidad::Disputa, carros: 3 };
    let b = en_curso(caravana);
    let con = |carros: &[&str]| {
        let mut res = resultado(&b);
        res.carros_perdidos = carros.iter().map(|c| c.to_string()).collect();
        b.validar_resultado("srv", &res, &sin_objetos).map(|()| b.efectos_del_resultado(&res).carros_perdidos)
    };
    assert_eq!(con(&["2", "0"]), Ok(vec![2, 0]));
    assert_eq!(con(&[]), Ok(vec![]));
    for malo in [&["3"][..], &["x"], &["1", "1"], &["-1"]] {
        assert!(matches!(con(malo), Err(CarroInexistente(_))), "{malo:?}");
    }
    // Fuera de un asalto de caravana no hay carros que perder.
    let campal = en_curso(CAMPAL);
    let mut res = resultado(&campal);
    res.carros_perdidos = vec!["0".into()];
    assert_eq!(campal.validar_resultado("srv", &res, &sin_objetos), Err(CarroInexistente("0".into())));
}

#[test]
fn el_botin_exige_participar_objetos_nuevos_y_casillas() {
    use ErrorResultado::*;
    let objeto = |i: &str| ObjetoBotin { definicion: "espada".into(), cantidad: 1, instancia: Some(i.into()) };
    let con = |objetos: Vec<ObjetoBotin>| Botin { objetos, monedas: Default::default() };
    assert_eq!(rechazo(|r| r.por_heroe[0].botin = Some(con(vec![objeto("a"), objeto("a")]))), ObjetoRepetido("a".into()));
    assert_eq!(rechazo(|r| r.por_heroe[0].botin = Some(con(vec![objeto("existente")]))), ObjetoRepetido("existente".into()));
    assert_eq!(rechazo(|r| r.por_heroe[0].botin = Some(con(vec![objeto("a"), objeto("b"), objeto("c")]))), SinCasillas(1), "tenía 2 casillas libres");
    assert_eq!(
        rechazo(|r| {
            r.por_heroe[0].participo = false;
            r.por_heroe[0].botin = Some(con(vec![objeto("a")]));
        }),
        BotinSinParticipar(1)
    );
    assert_eq!(rechazo(|r| r.por_heroe[0].botin = Some(con(vec![ObjetoBotin { definicion: "x".into(), cantidad: 0, instancia: None }]))), Forma);
    assert_eq!(rechazo(|r| r.por_heroe[0].botin = Some(Botin { objetos: vec![], monedas: militar::heroe::Monedas { bronce: -1, plata: 0, oro: 0 } })), Forma);
    // Un botín válido (con y sin identidad propia) pasa, y el mismo `itemInstanceId` en dos héroes no.
    let b = en_curso(CAMPAL);
    let mut res = resultado(&b);
    res.por_heroe[0].botin = Some(con(vec![objeto("a"), ObjetoBotin { definicion: "poción".into(), cantidad: 3, instancia: None }]));
    assert_eq!(b.validar_resultado("srv", &res, &sin_objetos), Ok(()));
    res.por_heroe[1].botin = Some(con(vec![objeto("a")]));
    assert_eq!(b.validar_resultado("srv", &res, &sin_objetos), Err(ObjetoRepetido("a".into())));
}

#[test]
fn el_servidor_del_resultado_tiene_que_ser_el_de_la_asignacion() {
    let b = en_curso(CAMPAL);
    assert_eq!(b.validar_resultado("otro", &resultado(&b), &sin_objetos), Err(ErrorResultado::AsignacionNoVigente));
}

#[test]
fn una_batalla_cerrada_se_retira_pasada_la_retencion() {
    let r = r();
    let mut b = batalla(CAMPAL);
    assert!(!b.retirable(r, i64::MAX), "una activa nunca");
    b.cancelar(2_000, 1).unwrap();
    assert!(!b.retirable(r, 2_000 + 60 * MIN - 1));
    assert!(b.retirable(r, 2_000 + 60 * MIN));
    let mut vieja = b.clone();
    vieja.cerrada_en = None;
    vieja.aplicada_en = Some(500);
    assert!(vieja.retirable(r, 500 + 60 * MIN), "sin cerradaEn cuenta aplicadaEn");
    vieja.aplicada_en = None;
    assert!(!vieja.retirable(r, vieja.expira_en + 60 * MIN - 1) && vieja.retirable(r, vieja.expira_en + 60 * MIN), "y en su defecto expiraEn");
}

#[test]
fn el_battle_id_es_un_uuid_determinista_del_game_id_y_el_contador() {
    assert_eq!(uuid_de_batalla("partida-1", 7), "ce4631ea-8538-59b0-0000-000000000007");
    assert_eq!(uuid_de_batalla("", 0), "811c9dc5-0100-0193-0000-000000000000");
    assert_eq!(uuid_de_batalla("partida-1", 7), uuid_de_batalla("partida-1", 7));
    let ids: BTreeSet<String> = (0..1000).map(|n| uuid_de_batalla("partida-1", n)).collect();
    assert_eq!(ids.len(), 1000, "el contador los distingue");
    assert_ne!(uuid_de_batalla("partida-1", 7), uuid_de_batalla("partida-2", 7));
    let u = uuid_de_batalla("x", u64::MAX);
    assert_eq!(u.len(), 36);
    assert_eq!(u.matches('-').count(), 4);
    assert!(u.chars().all(|c| c == '-' || c.is_ascii_hexdigit()));
}

/// Propiedad: sea cual sea la secuencia de mensajes, un rechazo no cambia nada, una cerrada no vuelve a cambiar, la revisión solo sube, nadie combate
/// dos veces y todo candado es de una escuadra de la batalla.
#[test]
fn ninguna_secuencia_de_mensajes_deja_la_batalla_incoherente() {
    use base::Rng;
    for semilla in 0..300 {
        let mut rng = Rng::nuevo(semilla);
        let mut b = batalla(if semilla % 2 == 0 { CAMPAL } else { ASEDIO });
        let mut ahora = 1_000;
        let mut intento = 0;
        for _ in 0..40 {
            ahora += rng.por_debajo(40_000) as i64;
            let antes = b.clone();
            let heroe = 1 + rng.por_debajo(8);
            let lado = if rng.por_debajo(2) == 0 { Lado::Atacante } else { Lado::Defensor };
            let revision = if rng.por_debajo(4) == 0 { 9 } else { b.ticket.revision };
            let cambio = match rng.por_debajo(9) {
                0 | 1 => b.unirse(ahora, lado, vec![part(heroe + 2, if heroe % 2 == 0 { Controlador::Humano } else { Controlador::Bot }, &[(100 + heroe, 25)])], 300 + heroe).is_ok(),
                2 => {
                    intento += 1;
                    matches!(b.registrar_asignacion(r(), ahora, asignacion(&format!("a{intento}"), "srv", revision, &[1])), Ok(Efecto::Aplicado(_)))
                }
                3 => matches!(b.confirmar_inicio(r(), ahora, &format!("a{intento}"), "srv", revision), Ok(Efecto::Aplicado(_))),
                4 => {
                    let motivo = [MotivoSalida::NoConectado, MotivoSalida::Eliminado, MotivoSalida::Abandono][rng.por_debajo(3) as usize];
                    matches!(b.registrar_salida(r(), ahora, "srv", SalidaBatalla { intento: format!("a{intento}"), revision, heroe, motivo, derrotado: false, escuadras: vec![] }), Ok(Efecto::Aplicado(_)))
                }
                5 => b.cancelar(ahora, 1 + rng.por_debajo(3)).is_ok(),
                6 => b.vencer(ahora).is_some(),
                7 => {
                    let mut res = resultado(&b);
                    res.intento = format!("a{intento}");
                    matches!(b.aplicar_resultado(ahora, "srv", res, &sin_objetos), Ok(Efecto::Aplicado(_)))
                }
                _ => b.registrar_tokens(ahora, &format!("a{intento}"), "srv", vec![]).is_ok(),
            };
            if !cambio {
                assert_eq!(b, antes, "semilla {semilla}: un mensaje que no se aplica no puede cambiar nada");
            }
            if antes.cerrada() {
                assert_eq!(b, antes, "semilla {semilla}: una cerrada no cambia");
            }
            assert!(b.ticket.revision >= antes.ticket.revision && b.ticket.revision <= antes.ticket.revision + 1);
            let combatientes: Vec<HeroeId> = b.heroes_que_combaten().collect();
            assert_eq!(combatientes.iter().collect::<BTreeSet<_>>().len(), combatientes.len(), "semilla {semilla}: nadie combate dos veces");
            let escuadras: BTreeSet<EscuadraId> = b.escuadras_de().map(|e| e.id).collect();
            assert!(b.escuadras_con_candado().iter().all(|e| escuadras.contains(e)), "semilla {semilla}");
            assert!(b.ticket.atacante.participantes.len() + b.ticket.defensor.participantes.len() + b.incorporaciones.len() >= combatientes.len());
        }
    }
}
