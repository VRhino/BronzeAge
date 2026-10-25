//! Monta el `Informe` del sistema «política y Facciones» (5I): el del batch militar (la misma población de héroes bots, con todos sus invariantes) con lo político delante.

use crate::armar::{max, metrica, parametro, serie, INFO};
use crate::armarmil;
use crate::armarpob::{mediana, pct, percentil};
use crate::geo::{Cargado, Geo};
use crate::simmil::pol::{self, MuestraPolitica, Pol};
use crate::simmil::{Corrida, BELICOSA_PCT};
use crate::{EscenarioMilitar, EscenarioPolitica};
use informe::{Grafico, Informe, Metrica, TipoGrafico};
use std::time::Duration;

const DIA: f64 = 86_400_000.0;
const PUNTO: f64 = 1_000_000.0;

pub struct Entrada<'a> {
    pub c: &'a Cargado,
    pub geo: &'a Geo,
    pub e: &'a EscenarioPolitica,
    pub corrida: &'a Corrida,
    /// (misma semilla ⇒ mismo hash, foto `BAN40` a mitad ⇒ mismo hash que sin ella) en el escenario reducido.
    pub determinismo: Option<(bool, bool)>,
    pub pared_total: Duration,
}

pub fn armar(x: &Entrada) -> Informe {
    let mil = EscenarioMilitar { plazas: x.e.plazas, dias: x.e.dias, semilla: x.e.semilla, verificar_determinismo: x.e.verificar_determinismo };
    let mut i = armarmil::armar(&armarmil::Entrada { c: x.c, geo: x.geo, e: &mil, corrida: x.corrida, determinismo: x.determinismo, pared_total: x.pared_total });
    let p = x.corrida.pol.as_ref().expect("corrida del batch político");
    let mut metricas = Vec::new();
    metricas.extend(ciudadania(x, p));
    metricas.extend(diplomacia(x, p));
    metricas.extend(tributo_y_reputacion(x, p));
    metricas.extend(uniones_y_derrotas(x, p));
    metricas.extend(titulos_y_nivel(x, p));
    metricas.extend(capital_y_distancia(x, p));
    metricas.extend(nobleza(x, p));
    metricas.extend(invariantes(p));
    // Las del militar detrás, sin la línea de «Andamiaje» de su escenario.
    metricas.append(&mut i.metricas);
    i.anomalias = armarmil::anomalias(&metricas);
    i.metricas = metricas;
    i.sistema = "Política y Facciones".into();
    let ultima = p.muestras.last().cloned().unwrap_or_default();
    i.resumen = format!(
        "Se simularon {} días de mundo con {} plazas de {} Facciones creadas por sus Reyes ({} al final, {} con plazas) y {} héroes bots sobre el núcleo real: la población del batch militar con la política en manos de bots de diplomacia que solo dan órdenes del contrato. Resumen automático; la lectura la añade quien revise los resultados.",
        x.e.dias,
        x.corrida.plazas.len(),
        x.corrida.facciones,
        ultima.facciones,
        ultima.con_plazas,
        x.corrida.bots.len()
    );
    i.parametros.retain(|q| !q.nombre.starts_with("Andamiaje") && !q.nombre.starts_with("Escenario"));
    let mut cabeza = vec![
        parametro(
            "Escenario político",
            format!(
                "{} plazas por Facción fundadas en los primeros minutos, como en el batch militar; cada Facción la crea su Rey con `CrearFaccion` y parte del nivel {} (`FijarNivelFaccion`, cap 3 y una plaza de nivel 2) y sube con la experiencia; {} belicosas (el {} %), {} traicioneras y {} ambiciosas; sin parejas aliadas de salida",
                crate::simmil::PLAZAS_POR_FACCION,
                pol::NIVEL_INICIAL,
                x.corrida.belicosas.len(),
                BELICOSA_PCT,
                p.traicioneras.len(),
                p.ambiciosas.len()
            ),
        ),
        parametro("Diplomacia", format!("cada {} h el Rey (o el Embajador, la mitad de las veces que la orden lo deja) contesta solicitudes (admite el {} %) y propuestas, propone la paz pasadas {} h de guerra, se rebela si es más fuerte que su señora, designa capital al tener Palacio y toma como mucho una iniciativa con pesos: guerra (las belicosas, la mitad de las veces contra quien tiene un título y, si no, la mitad contra una que está en un vasallaje), alianza, vasallaje con {} milésimas de trigo por minuto, anexión (las ambiciosas, de Facciones sin plazas o vasallas pequeñas), fusión, romper, liberar, traición (las traicioneras), corona o admisión; las alianzas, con una de las {} vecinas más cercanas", pol::DIPLOMACIA_H, pol::ADMITE_PCT, pol::PAZ_TRAS_H, pol::TRIBUTO_POR_MINUTO, pol::VECINAS)),
        parametro("Colonos y secesión", format!("cada {} h nace un colono que pide entrar en dos Facciones con plazas y, admitido, se muda a la plaza suya con menos residentes; una vez al día, con el {} %, un Cazador (no Rey ni Embajador) de una Facción de {} ciudadanos o más la deja y, pasado el cooldown de creación, crea la suya (sin plazas, a refundar)", pol::COLONOS_CADA_H, pol::SECESION_PCT, pol::SECESION_MIN_CIUDADANOS)),
        parametro("Nobleza (riesgo R1)", if p.ajustes.nobleza { "como en las reglas (residentes ≥ 3 y Palacio activo)" } else { "EXPERIMENTO: sin nobleza (`nobleza.minCiudadanos` inalcanzable)" }),
        parametro("Andamiaje de administración", "el nivel de partida de cada Facción, las tecnologías de la Era I, la colonia de cada plaza (`crear_ejercito` + `DarCaravanaFundacion`) y colocar a cada héroe dentro de su plaza al nacer. La ciudadanía, la residencia, los cargos, la puerta, las relaciones, la capital, la anexión, la fusión y la refundación van con las órdenes del contrato"),
    ];
    cabeza.append(&mut i.parametros);
    i.parametros = cabeza;
    let mut series = vec![grafico_nivel(p), grafico_reputacion(p), grafico_relaciones(p), grafico_distancia(p)];
    series.append(&mut i.series);
    i.series = series;
    i
}

fn n_hecho(x: &Entrada, codigo: &str) -> f64 {
    x.corrida.medidas.hechos.get(codigo).copied().unwrap_or(0) as f64
}

fn aceptadas(x: &Entrada, etiqueta: &str) -> f64 {
    x.corrida.ordenes.get(etiqueta).map_or(0, |c| c.aceptadas) as f64
}

fn rechazos(x: &Entrada, etiqueta: &str) -> String {
    let Some(c) = x.corrida.ordenes.get(etiqueta) else { return "ninguno".into() };
    let mut v: Vec<(&&str, &u64)> = c.rechazos.iter().collect();
    v.sort_by_key(|(k, n)| (std::cmp::Reverse(**n), **k));
    if v.is_empty() {
        return "ninguno".into();
    }
    v.iter().take(5).map(|(k, n)| format!("{k} ({n})")).collect::<Vec<_>>().join(", ")
}

fn rechazo(x: &Entrada, etiqueta: &str, codigo: &str) -> f64 {
    x.corrida.ordenes.get(etiqueta).and_then(|c| c.rechazos.get(codigo)).copied().unwrap_or(0) as f64
}

fn ciudadania(x: &Entrada, p: &Pol) -> Vec<Metrica> {
    let ultima = p.muestras.last().cloned().unwrap_or_default();
    let n = &x.corrida.nucleo;
    let ciudadanos: usize = n.facciones().map(|f| f.ciudadanos.len()).sum();
    vec![
        metrica("pol_facciones_creadas", "Facciones creadas con `CrearFaccion`", "Facciones", INFO, Some(aceptadas(x, "CrearFaccion")), "Diplomacia_Facciones §3.1", &format!("Al empezar, una por Rey; después, las de los disidentes. Rechazos de las de los disidentes: {}.", rechazos(x, "CrearFaccion"))),
        metrica("pol_facciones_final", "Facciones al final", "Facciones", INFO, Some(ultima.facciones as f64), "Diplomacia_Facciones §2.1 y §8", &format!("Con plazas {}; las demás, derrotadas o sin fundar todavía. Ciudadanos en todas: {ciudadanos}.", ultima.con_plazas)),
        metrica("pol_ingresos", "Ingresos aceptados", "ingresos", INFO, Some(n_hecho(x, "faccion.ciudadania_union")), "Diplomacia_Facciones §3.2", &format!("Solicitudes {} (colonos nacidos {}); denegadas {}; aceptar se rechazó: {}.", n_hecho(x, "faccion.solicitud_ingreso"), p.m.colonos, n_hecho(x, "faccion.solicitud_denegada"), rechazos(x, "AceptarSolicitud"))),
        metrica("pol_secesiones", "Héroes que dejaron su Facción para crear la suya", "héroes", INFO, Some(p.m.secesiones as f64), "Diplomacia_Facciones §3.3", &format!("`faccion.abandonada` {}. Rechazos de `DejarFaccion`: {}.", n_hecho(x, "faccion.abandonada"), rechazos(x, "DejarFaccion"))),
        metrica("pol_mudanzas", "Mudanzas con `CambiarResidencia`", "órdenes", INFO, Some(aceptadas(x, "CambiarResidencia")), "Diplomacia_Facciones §4.2", &format!("Al nacer, al conquistar, los colonos y los desalojados. Rechazos: {}. Residentes en plazas al final: {} ({:.1} por plaza).", rechazos(x, "CambiarResidencia"), ultima.residentes, ultima.residentes as f64 / ultima.plazas.max(1) as f64)),
        metrica("pol_cargos", "Cargos locales asignados", "cargos", INFO, Some(n_hecho(x, "cargo.local_asignado")), "Diplomacia_Facciones §3.4 (Q4)", &format!("Reyes asignados {} (`AsignarRey` aceptadas {}), Embajadores {}.", n_hecho(x, "cargo.rey_asignado"), aceptadas(x, "AsignarRey"), n_hecho(x, "cargo.embajador_asignado"))),
        metrica("pol_puerta", "Puertas fijadas", "órdenes", INFO, Some(aceptadas(x, "FijarPuerta")), "Diplomacia_Facciones §4.3 (Q1)", &format!("Vetos {} y levantados {}; expulsados al mapa {}; reabastecer aliadas abierto {} veces.", aceptadas(x, "VetarJugador"), aceptadas(x, "LevantarVeto"), n_hecho(x, "heroe.expulsado"), aceptadas(x, "AlternarReabastecerAliados"))),
    ]
}

fn diplomacia(x: &Entrada, p: &Pol) -> Vec<Metrica> {
    let ultima = p.muestras.last().cloned().unwrap_or_default();
    let m = &p.m;
    vec![
        metrica("pol_alianzas", "Alianzas firmadas", "alianzas", INFO, Some(m.alianzas as f64), "Diplomacia_Facciones §5.3", &format!("Propuestas {} (rechazos al proponer: {}); rechazadas {}; activas al final {}.", aceptadas(x, "ProponerAlianza"), rechazos(x, "ProponerAlianza"), n_hecho(x, "diplomacia.relacion_rechazada"), ultima.alianzas)),
        metrica("pol_vasallajes", "Vasallajes firmados", "vasallajes", INFO, Some(m.vasallajes as f64), "Diplomacia_Facciones §5.3 y §5.6", &format!("Propuestos {} (rechazos al proponer: {}); activos al final {}; vasallas conquistadas por un tercero {}.", aceptadas(x, "ProponerVasallaje"), rechazos(x, "ProponerVasallaje"), ultima.vasallajes, n_hecho(x, "diplomacia.vasallo_conquistado"))),
        metrica("pol_guerras", "Guerras declaradas", "guerras", INFO, Some(n_hecho(x, "diplomacia.guerra_declarada")), "Diplomacia_Facciones §5.4", &format!("Con `DeclararGuerra` {} (rechazos: {}), al atacar y por rebelión las demás; Facciones arrastradas {}; activas al final {}.", aceptadas(x, "DeclararGuerra"), rechazos(x, "DeclararGuerra"), m.arrastradas, ultima.guerras)),
        metrica("pol_traiciones", "Traiciones (atacar a una aliada con `declara_guerra`)", "traiciones", INFO, Some(m.traiciones as f64), "Diplomacia_Facciones §5.4 (Q2)", &format!("Planeadas {} (los Capitanes de la traicionera atacan a su aliada durante {} días si la tienen a tiro).", m.traiciones_planeadas, pol::TRAICION_DIAS)),
        metrica("pol_paces", "Paces firmadas", "paces", INFO, Some(n_hecho(x, "diplomacia.paz_firmada")), "Diplomacia_Facciones §5.5", &format!("Ofrecidas {}.", n_hecho(x, "diplomacia.paz_propuesta"))),
        metrica("pol_rupturas", "Alianzas rotas", "alianzas", INFO, Some(m.alianzas_rotas as f64), "Diplomacia_Facciones §5.5", &format!("Con `RomperRelacion` {} (las demás, por traición); vasallajes rotos {}.", aceptadas(x, "RomperAlianza"), m.vasallajes_rotos)),
        metrica("pol_liberaciones", "Vasallas liberadas", "vasallas", INFO, Some(aceptadas(x, "LiberarVasalla") + n_hecho(x, "diplomacia.vasallo_liberado")), "Diplomacia_Facciones §5.5", &format!("Por su señora {} (`RomperRelacion`) y por el desarme del señor al perder su última plaza {}.", aceptadas(x, "LiberarVasalla"), n_hecho(x, "diplomacia.vasallo_liberado"))),
        metrica("pol_rebeliones", "Rebeliones de vasallas", "rebeliones", INFO, Some(n_hecho(x, "diplomacia.rebelion_vasallo")), "Diplomacia_Facciones §5.5", &format!("Rechazos: {}.", rechazos(x, "RebelionVasallo"))),
    ]
}

fn tributo_y_reputacion(x: &Entrada, p: &Pol) -> Vec<Metrica> {
    let m = &p.m;
    let ultima = p.muestras.last().cloned().unwrap_or_default();
    let rep: Vec<f64> = ultima.reputaciones.iter().map(|&r| r as f64 / PUNTO).collect();
    let perdido = m.tributo_pagado - m.tributo_recibido;
    let _ = x;
    vec![
        metrica("pol_tributo_pagado", "Tributo pagado", "unidades", INFO, Some(m.tributo_pagado as f64), "Diplomacia_Facciones §5.6 (Q3, 5A-2)", &format!("En {} pagos de plaza; recibido {} y perdido por no caber en la capital {} ({}).", m.tributos, m.tributo_recibido, perdido, pct(perdido.max(0) as usize, m.tributo_pagado.max(0) as usize).map_or("—".into(), |v| format!("{v:.1} %")))),
        metrica("pol_tributo_perdido", "Tributo perdido por no caber", "unidades", INFO, Some(perdido as f64), "Diplomacia_Facciones §5.6 (5A-2)", "Lo pagado es lo recibido más lo perdido."),
        metrica("pol_reputacion_mediana", "Reputación mediana de las Facciones al final", "puntos", INFO, mediana(&rep), "Diplomacia_Facciones §3.6", &format!("Mínima {}, máxima {}; {} cambios por sucesos.", rep.iter().copied().fold(f64::INFINITY, f64::min), rep.iter().copied().fold(f64::NEG_INFINITY, f64::max), m.rep_cambios)),
    ]
}

fn uniones_y_derrotas(x: &Entrada, p: &Pol) -> Vec<Metrica> {
    let m = &p.m;
    let sin_plazas: Vec<f64> = m.refundaciones.iter().map(|r| r.3 as f64 / DIA).collect();
    let derrotadas: std::collections::BTreeSet<u32> = m.derrotas.iter().map(|d| d.1).collect();
    vec![
        metrica("pol_anexiones", "Anexiones", "anexiones", INFO, Some(m.anexiones.len() as f64), "Diplomacia_Facciones §6.1", &format!("Propuestas {} (rechazos: {}); plazas trasladadas {}; aceptar se rechazó: {}.", aceptadas(x, "ProponerAnexion"), rechazos(x, "ProponerAnexion"), m.anexiones.iter().map(|a| a.3).sum::<usize>(), rechazos(x, "ResponderAnexion"))),
        metrica("pol_fusiones", "Fusiones", "fusiones", INFO, Some(m.fusiones.len() as f64), "Diplomacia_Facciones §6.2", &format!("Propuestas {} (rechazos: {}); plazas en las Facciones nuevas {}; aceptar se rechazó: {}.", aceptadas(x, "ProponerFusion"), rechazos(x, "ProponerFusion"), m.fusiones.iter().map(|f| f.4).sum::<usize>(), rechazos(x, "ResponderFusion"))),
        metrica("pol_derrotadas", "Facciones derrotadas (sin plazas)", "Facciones", INFO, Some(derrotadas.len() as f64), "Diplomacia_Facciones §8", &format!("Derrotas {} (por conquista {}, por colapso {}).", m.derrotas.len(), m.derrotas.iter().filter(|d| d.2 != 0).count(), m.derrotas.iter().filter(|d| d.2 == 0).count())),
        metrica("pol_refundadas", "Facciones sin plazas que volvieron a fundar", "refundaciones", INFO, Some(m.refundaciones.len() as f64), "Fundacion_Y_Refundacion §9", &format!("Tiempo mediano sin plazas {} días. Caravanas de Refundación compradas {}, columnas que salieron a fundar {} (fallidas {}); turnos sin fondo bastante {}. Aportes del almacén {} (y {} para hacer sitio en él), compras en el campamento {}; salidas a cazar bandidos del anillo para el fondo {} (oro traído {} u, vaciado en el almacén personal {} veces).", mediana(&sin_plazas).map_or("—".into(), |v| format!("{v:.1}")), aceptadas(x, "ComprarCaravanaDeRefundacion"), m.tareas, m.tareas_fallidas, m.sin_fondo, aceptadas(x, "AportarARefundacion"), aceptadas(x, "AportarParaHacerSitio"), aceptadas(x, "ComprarEnCampamento"), m.cazas, m.oro_de_caza / 1000, aceptadas(x, "VaciarCarroDeCaza"))),
    ]
}

fn titulos_y_nivel(x: &Entrada, p: &Pol) -> Vec<Metrica> {
    let m = &p.m;
    let ultima = p.muestras.last().cloned().unwrap_or_default();
    let niveles: Vec<f64> = ultima.niveles.iter().map(|&n| n as f64).collect();
    let cambian: u64 = m.titulos.values().map(|v| v.1).sum();
    vec![
        metrica("pol_titulos_cambian", "Títulos que cambiaron de manos", "cambios", INFO, Some(cambian as f64), "Diplomacia_Facciones §7", &format!("Nacieron {}. Por título: {}.", m.titulos.values().map(|v| v.0).sum::<u64>(), m.titulos.iter().map(|(k, v)| format!("{k} {} / {}", v.0, v.1)).collect::<Vec<_>>().join(", "))),
        metrica("pol_nivel_mediano", "Nivel de Facción mediano al final", "nivel", INFO, mediana(&niveles), "Diplomacia_Facciones §3.5", &format!("Desde el {} de partida; máximo {}; subidas de nivel {}.", pol::NIVEL_INICIAL, niveles.iter().copied().fold(0.0, f64::max), n_hecho(x, "faccion.nivel_subio"))),
    ]
}

fn capital_y_distancia(x: &Entrada, p: &Pol) -> Vec<Metrica> {
    let m = &p.m;
    let ultima = p.muestras.last().cloned().unwrap_or_default();
    let mut mult: Vec<f64> = ultima.multiplicadores.iter().map(|&v| v as f64 / PUNTO).collect();
    let lejos = m.ruinas.iter().filter(|r| r.2 > 0).count();
    let distancias: Vec<f64> = m.ruinas.iter().filter(|r| r.2 > 0).map(|r| r.2 as f64 / 1000.0).collect();
    vec![
        metrica("pol_capitales", "Capitales designadas", "órdenes", INFO, Some(aceptadas(x, "DesignarCapital")), "Diplomacia_Facciones §3.7", &format!("Palacios pedidos {} (rechazos: {}), con Palacio activo al final {} plazas; Gran Fundición pedida {} (rechazos: {}).", aceptadas(x, "AnadirPalacio"), rechazos(x, "AnadirPalacio"), ultima.con_palacio, aceptadas(x, "AnadirGranFundicion"), rechazos(x, "AnadirGranFundicion"))),
        metrica("pol_multiplicador_mediano", "Multiplicador del mantenimiento por la distancia a la capital (mediana, al final)", "×", INFO, mediana(&mult), "Economia_Construccion §10.1 (escalaDistancia 300 u)", &format!("De las {} plazas que no son su capital; percentil 90 {}, máximo {}.", mult.len(), percentil(&mut mult, 0.9).map_or("—".into(), |v| format!("{v:.2}")), mult.iter().copied().fold(0.0, f64::max))),
        metrica("pol_ruinas_lejos", "Plazas en ruinas lejos de su capital", "plazas", INFO, Some(lejos as f64), "Economia_Construccion §10.1 (5C-2)", &format!("De {} ruinas; distancia mediana a la capital de las lejanas {} u. Comprobaciones de ascenso insolventes {}, sin cupo de Facción {}.", m.ruinas.len(), mediana(&distancias).map_or("—".into(), |v| format!("{v:.0}")), rechazo(x, "SolicitarAscenso", "ascenso.insolvente"), rechazo(x, "SolicitarAscenso", "ascenso.sin_cupo_de_faccion"))),
    ]
}

fn nobleza(x: &Entrada, p: &Pol) -> Vec<Metrica> {
    let ultima = p.muestras.last().cloned().unwrap_or_default();
    vec![
        metrica("pol_nobles", "Nobles al final", "nobles", INFO, Some(ultima.nobles as f64), "Economia_Construccion §9 (Q8, riesgo R1)", &format!("En {} plazas (con Palacio activo {}); primeros nobles {}. Población total al final {}.", ultima.con_nobleza, ultima.con_palacio, n_hecho(x, "poblacion.primeros_nobles"), ultima.poblacion)),
    ]
}

fn invariantes(p: &Pol) -> Vec<Metrica> {
    let v = &p.v;
    let uno = |id: &str, nombre: &str, valor: u64, origen: &str, nota: &str| metrica(id, nombre, "violaciones", max(0.0), Some(valor as f64), origen, nota);
    vec![
        uno("inv_ciudadania", "Héroes en dos Facciones o con otra Facción que la que los cuenta", v.ciudadania, "Diplomacia_Facciones §3", "Cada media hora: cada héroe es ciudadano de una sola Facción y la Facción guarda la misma ciudadanía que el héroe."),
        uno("inv_rey", "Facciones con ciudadanos sin Rey, o con un Rey o Embajador que no es su ciudadano", v.rey, "Diplomacia_Facciones §3.4", "Cada media hora."),
        uno("inv_cargos", "Cargos locales de quien no reside o no es de la Facción dueña", v.cargos, "Diplomacia_Facciones §3.4 (Q4)", "Cada media hora, los cinco cargos de cada plaza."),
        uno("inv_relaciones", "Pares con dos relaciones activas o vasallas con dos señoras", v.relaciones, "Diplomacia_Facciones §5.2", "Cada media hora, y el índice de la activa de cada par."),
        uno("inv_arrastre", "Guerras con un arrastre que no es el de `guerra.confirmar` o con el bando sin guerra", v.arrastre, "Diplomacia_Facciones §5.4", "Tras cada `DeclararGuerra`, `RebelionVasallo` y `Atacar` con `declara_guerra` aceptados: el hecho existe, el atacante no está en su bando, todo el bando queda en guerra con él y las arrastradas son las de antes de la orden."),
        uno("inv_tributo", "Pagos de tributo incoherentes", v.tributo, "Diplomacia_Facciones §5.6", "Cada pago: lo pagado es positivo y no menor que lo recibido (lo perdido es la diferencia), de un vasallaje con tributo de esa vasalla; y lo pagado por cada vasallaje no pasa de lo pactado por minuto."),
        uno("inv_reputacion", "Reputaciones que se mueven sin sus sucesos", v.reputacion, "Diplomacia_Facciones §3.6", "Cada media hora: sin un suceso ni una relación de la Facción (ni el cruce del 0) su reputación sigue el ritmo de la observación anterior; ningún suceso pasa del mayor del bloque; nunca fuera de ±100. Se toleran 10 millonésimas de punto: el redondeo de reanclar el ritmo fuera de un minuto entero (la cita del umbral)."),
        uno("inv_traslado", "Anexiones o fusiones que pierden o duplican algo", v.traslado, "Diplomacia_Facciones §6.3", "En cada una aceptada, en el mismo instante: recursos (plazas, pueblos, almacenes personales, carros, columnas, flota, Caravanas de Fundación y fondos), héroes, escuadras, soldados, plazas y caravanas; los ciudadanos y las plazas de la que queda son la unión de los de las dos, todos con su Facción, y la que desaparece ya no existe."),
        uno("inv_ids_faccion", "Ids de Facción reutilizados", v.ids, "Diplomacia_Facciones §6.2 (5F-8)", "Una Facción desaparecida no vuelve; la que nace de una fusión no toma un id ya visto."),
        uno("inv_xp_faccion", "Experiencia de Facción que baja", v.experiencia, "Diplomacia_Facciones §3.5", "Cada media hora."),
    ]
}

// ───────────────────────────── Gráficos ─────────────────────────────

fn por_dia(m: &[MuestraPolitica], f: impl Fn(&MuestraPolitica) -> Option<f64>) -> Vec<(f64, f64)> {
    m.iter().filter_map(|u| Some((u.dia as f64, f(u)?))).collect()
}

fn grafico_nivel(p: &Pol) -> Grafico {
    let niveles = |u: &MuestraPolitica| u.niveles.iter().map(|&n| n as f64).collect::<Vec<_>>();
    Grafico {
        id: "nivel_faccion_dia".into(),
        titulo: "Nivel de Facción".into(),
        tipo: TipoGrafico::Linea,
        eje_x: "día de mundo".into(),
        eje_y: "nivel".into(),
        series: vec![
            serie("Mediana", por_dia(&p.muestras, |u| mediana(&niveles(u)))),
            serie("Máximo", por_dia(&p.muestras, |u| niveles(u).into_iter().reduce(f64::max))),
            serie("Mínimo", por_dia(&p.muestras, |u| niveles(u).into_iter().reduce(f64::min))),
        ],
    }
}

fn grafico_reputacion(p: &Pol) -> Grafico {
    let rep = |u: &MuestraPolitica| u.reputaciones.iter().map(|&r| r as f64 / PUNTO).collect::<Vec<_>>();
    Grafico {
        id: "reputacion_dia".into(),
        titulo: "Reputación de las Facciones".into(),
        tipo: TipoGrafico::Linea,
        eje_x: "día de mundo".into(),
        eje_y: "puntos".into(),
        series: vec![
            serie("Mediana", por_dia(&p.muestras, |u| mediana(&rep(u)))),
            serie("Máxima", por_dia(&p.muestras, |u| rep(u).into_iter().reduce(f64::max))),
            serie("Mínima", por_dia(&p.muestras, |u| rep(u).into_iter().reduce(f64::min))),
        ],
    }
}

fn grafico_relaciones(p: &Pol) -> Grafico {
    let s = |nombre: &str, f: &dyn Fn(&MuestraPolitica) -> usize| serie(nombre, por_dia(&p.muestras, |u| Some(f(u) as f64)));
    Grafico {
        id: "relaciones_dia".into(),
        titulo: "Relaciones activas y Facciones".into(),
        tipo: TipoGrafico::Linea,
        eje_x: "día de mundo".into(),
        eje_y: "cuántas".into(),
        series: vec![s("Alianzas", &|u| u.alianzas), s("Vasallajes", &|u| u.vasallajes), s("Guerras", &|u| u.guerras), s("Facciones", &|u| u.facciones), s("Facciones con plazas", &|u| u.con_plazas)],
    }
}

fn grafico_distancia(p: &Pol) -> Grafico {
    let mult = |u: &MuestraPolitica| u.multiplicadores.iter().map(|&v| v as f64 / PUNTO).collect::<Vec<_>>();
    Grafico {
        id: "distancia_dia".into(),
        titulo: "Mantenimiento por la distancia a la capital y nobleza".into(),
        tipo: TipoGrafico::Linea,
        eje_x: "día de mundo".into(),
        eje_y: "× / plazas".into(),
        series: vec![
            serie("Multiplicador mediano", por_dia(&p.muestras, |u| mediana(&mult(u)))),
            serie("Multiplicador p90", por_dia(&p.muestras, |u| percentil(&mut mult(u), 0.9))),
            serie("Plazas con nobleza", por_dia(&p.muestras, |u| Some(u.con_nobleza as f64))),
        ],
    }
}
