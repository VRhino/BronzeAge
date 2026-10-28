//! Monta el `Informe` del sistema «tecnología y Aedas» (6H): el del batch político (la misma población, con todos sus invariantes) con lo tecnológico delante. Informa, sin juzgar,
//! en qué día de mundo cae cada logro y cada Era (para la fase 7): las métricas de días son informativas.

use crate::armar::{max, metrica, parametro, serie, INFO};
use crate::armarmil;
use crate::armarpol;
use crate::geo::{Cargado, Geo};
use crate::simmil::tec::{self, MuestraTec, Tec};
use crate::simmil::Corrida;
use crate::{EscenarioPolitica, EscenarioTecnologia};
use informe::{Grafico, Informe, Metrica, TipoGrafico};
use std::time::Duration;

const DIA: f64 = 86_400_000.0;

pub struct Entrada<'a> {
    pub c: &'a Cargado,
    pub geo: &'a Geo,
    pub e: &'a EscenarioTecnologia,
    pub corrida: &'a Corrida,
    /// (misma semilla ⇒ mismo hash, foto a mitad ⇒ mismo hash que sin ella) en el escenario reducido.
    pub determinismo: Option<(bool, bool)>,
    pub pared_total: Duration,
}

pub fn armar(x: &Entrada) -> Informe {
    let pol = EscenarioPolitica { plazas: x.e.plazas, dias: x.e.dias, semilla: x.e.semilla, verificar_determinismo: x.e.verificar_determinismo, nobleza: true };
    let mut i = armarpol::armar(&armarpol::Entrada { c: x.c, geo: x.geo, e: &pol, corrida: x.corrida, determinismo: x.determinismo, pared_total: x.pared_total });
    let t = x.corrida.tec.as_ref().expect("corrida del batch tecnológico");
    let mut metricas = Vec::new();
    metricas.extend(dias(x, t));
    metricas.extend(vias(x, t));
    metricas.extend(aedas(x, t));
    metricas.extend(alijos(x, t));
    metricas.extend(invariantes(t));
    metricas.append(&mut i.metricas);
    i.anomalias = armarmil::anomalias(&metricas);
    i.metricas = metricas;
    i.sistema = "Tecnología y Aedas".into();
    let n = &x.corrida.nucleo;
    let r = &n.estado().reglas.economia.tecnologia;
    let e = n.tecnologia();
    i.resumen = format!(
        "Se simularon {} días de mundo con {} plazas y {} héroes bots sobre el núcleo real (la población del batch político) con bots de tecnología que solo dan órdenes del contrato. Era final: {}; logros cumplidos {} de {}; {} adopciones, {} compras a los Aedas, {} épicas cumplidas, {} alijos abiertos. Resumen automático; la lectura la añade quien revise los resultados.",
        x.e.dias,
        x.corrida.plazas.len(),
        x.corrida.bots.len(),
        r.eras[e.era as usize].nombre,
        e.logros.iter().flatten().count(),
        r.catalogo.iter().filter(|d| d.logro.is_some()).count(),
        t.m.adopciones,
        t.m.compras,
        t.m.epicas_cumplidas,
        t.m.alijos_abiertos
    );
    let mut cabeza = vec![
        parametro(
            "Escenario tecnológico",
            format!(
                "la población del batch político; las tecnologías solo por la vía de juego (sin `FijarTecnologias`). Umbrales de los logros: {}; la velocidad ({} u/min) y la estancia ({} min) de los Aedas, las del TypeScript (Q3)",
                if x.e.escala_logros > 1 { format!("EXPERIMENTO: los del TypeScript divididos por {} (como mínimo 1)", x.e.escala_logros) } else { "los del TypeScript (Q3)".into() },
                r.aedas.velocidad / 1000,
                r.aedas.estancia / 60_000
            ),
        ),
        parametro("Bots de tecnología", format!("el Rey adopta dentro de su capital lo que le aparece y puede pagar (cada hora); el Gobernador de la capital levanta o mejora, cada {} h, el primer edificio que pide el hito de una tecnología de una Era abierta; el de cada plaza compra a un Aeda parado lo que le reveló, le cierra la puerta el {} % de sus turnos con un Aeda dentro (la reabre a las {} h), encarga a cada residente sin épica la posible más a mano (por hecho de cada capítulo: obra 0; adopción, batalla y caravana 1; ascenso, defensa y conquista 3) y levanta o mejora la obra que pide su capítulo, y abandona una en curso el {} ‰ de sus turnos; las caravanas del Intendente llevan {} u de trigo (en el militar, {}) para que inspiren el capítulo `caravana`", tec::OBRAS_CADA_H, tec::PUERTA_AEDAS_PCT, tec::PUERTA_AEDAS_H, tec::ABANDONO_POR_MIL, tec::CARGA_DE_CARAVANA / 1000, crate::simmil::CARGA_DE_CARAVANA / 1000)),
        parametro("Buscadores de alijos", format!("{} héroes sin Facción, cada uno de un campamento: salen, abren hasta {} alijos a {} u como mucho, el {} % de las veces lo intentan otra vez (tiene que rechazarse), vuelven, vacían el carro y reclutan en su campamento con el oro; descansan {} h", tec::BUSCADORES, tec::ALIJOS_POR_SALIDA, tec::RADIO_ALIJOS / 1000, tec::REINTENTO_PCT, tec::DESCANSO_BUSCADOR_H)),
        parametro("Andamiaje de administración", "además del del batch político, el oro de los buscadores al nacer (`DarAlmacenPersonal`: dos reclutas de la tropa más barata de su campamento), porque el de los alijos no paga uno"),
        parametro("Crónica", format!("guardada en el diario de la partida (núcleo persistente desde el principio, en un directorio temporal): {} entradas emitidas, {} guardadas", t.m.cronica.0, t.m.cronica.1)),
    ];
    cabeza.append(&mut i.parametros);
    i.parametros = cabeza;
    let mut series = vec![grafico(t)];
    series.append(&mut i.series);
    i.series = series;
    i
}

fn n_hecho(x: &Entrada, codigo: &str) -> f64 {
    x.corrida.medidas.hechos.get(codigo).copied().unwrap_or(0) as f64
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

fn dia(t: i64) -> f64 {
    t as f64 / DIA
}

/// El día de cada logro y de cada Era, y el de la primera aparición de cada tecnología (informativo, para la fase 7).
fn dias(x: &Entrada, t: &Tec) -> Vec<Metrica> {
    let n = &x.corrida.nucleo;
    let r = &n.estado().reglas.economia.tecnologia;
    let e = n.tecnologia();
    let mut v = Vec::new();
    for (k, era) in r.eras.iter().enumerate().skip(1) {
        let cuando = t.m.eras.iter().find(|&&(_, orden, _)| orden == era.orden);
        let nota = match cuando {
            Some(&(_, _, por_plazo)) => format!("Empieza {} (mínimo de la anterior {} días, plazo {} días).", if por_plazo { "por plazo" } else { "con todos los logros de la anterior" }, r.eras[k - 1].minimo as f64 / DIA, r.eras[k - 1].plazo as f64 / DIA),
            None => format!("No empezó en la corrida (plazo de la anterior: {} días desde que empezó).", r.eras[k - 1].plazo as f64 / DIA),
        };
        v.push(metrica(&format!("dia_era_{}", era.orden), &format!("Día en que empieza la Era {} ({})", era.orden, era.nombre), "día", INFO, cuando.map(|&(en, _, _)| dia(en)), "Tecnologia_Aedas §3 paso 2", &nota));
    }
    for (k, d) in r.catalogo.iter().enumerate() {
        let Some(l) = d.logro else { continue };
        let primero = e.primeros[k].map_or("no aparece a nadie".into(), |p| format!("aparece por primera vez el día {:.2} (descubridora f-{})", dia(p.en), p.faccion));
        let nota = format!("Era {}; `{}` ≥ {} (al final, {}); {}.", r.eras[d.era as usize].orden, l.contador.nombre(), l.umbral, e.contador(l.contador), primero);
        v.push(metrica(&format!("dia_logro_{}", d.id), &format!("Día del logro de {}", d.nombre), "día", INFO, e.logros[k].map(dia), "Tecnologia_Aedas §2 y §11.1 (Q3: X del TypeScript)", &nota));
    }
    v
}

fn vias(x: &Entrada, t: &Tec) -> Vec<Metrica> {
    let m = &t.m;
    vec![
        metrica("tec_adopciones", "Tecnologías adoptadas por el Rey", "adopciones", INFO, Some(m.adopciones as f64), "Tecnologia_Aedas §4", &format!("`AdoptarTecnologia` dentro de la capital con la tarifa de su Era. Turnos con algo por adoptar y el Rey fuera de la capital: {} (en {} el Rey era un héroe bot militar; salió hacia ella desde otra plaza suya {} veces y entró {}); tecnologías que la capital no podía pagar: {}. Rechazos: {}.", x.corrida.medidas.adopcion_rey_fuera, x.corrida.medidas.adopcion_rey_bot, m.reyes_salen, m.reyes_llegan, x.corrida.medidas.adopcion_sin_tarifa, rechazos(x, "AdoptarTecnologia"))),
        metrica("tec_aparecidas", "Tecnologías aparecidas (por hito, compra o épica)", "apariciones", INFO, Some(m.aparecidas_por.iter().sum::<u64>() as f64), "Tecnologia_Aedas §3", &format!("Por hito {}, por compra a un Aeda {}, por épica {}.", m.aparecidas_por[0], m.aparecidas_por[1], m.aparecidas_por[2])),
        metrica("tec_obras_hito", "Obras hacia los hitos", "órdenes", INFO, Some(m.obras_hito as f64), "Tecnologia_Aedas §3.1", &format!("Edificios añadidos o mejorados por el Gobernador de la capital. Rechazos: {}.", rechazos(x, "ObraDeHito"))),
        metrica("tec_minas", "Pueblos mineros marcados hacia los logros de lo extraído", "pueblos", INFO, Some(m.minas as f64), "Tecnologia_Aedas §2.1", &format!("Cobre, estaño o hierro con un logro pendiente de una Era abierta. Rechazos: {}.", rechazos(x, "MarcarMina"))),
        metrica("tec_reclutas_campamento", "Reclutas en un campamento de mercenarios", "soldados", INFO, Some(m.reclutados_campamento as f64), "Tecnologia_Aedas §2.1 (Q4)", &format!("Órdenes aceptadas {}: cuentan para los logros `reclutados.*`. El recluta más barato de cada campamento de los buscadores cuesta {} u; el oro con que nacen (andamiaje declarado) suma {:.0} u. Rechazos: {}.", m.reclutas_campamento, m.precio_recluta.iter().map(|p| format!("{:.0}", *p as f64 / 1000.0)).collect::<Vec<_>>().join(", "), m.oro_andamiaje as f64 / 1000.0, rechazos(x, "ReclutarEnCampamento"))),
    ]
}

fn aedas(x: &Entrada, t: &Tec) -> Vec<Metrica> {
    let m = &t.m;
    let n = &x.corrida.nucleo;
    let perdidas = if m.epicas_perdidas.is_empty() { "ninguna".into() } else { m.epicas_perdidas.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", ") };
    vec![
        metrica("tec_itinerantes", "Aedas itinerantes al final", "Aedas", INFO, Some(n.aedas().len() as f64), "Tecnologia_Aedas §6.2", &format!("Cantos de descubrimientos {}, revelaciones {} (`aedas.revela`). Puertas cerradas a los Aedas con alguno dentro {} (salieron al instante {}).", m.cantos, m.revelaciones, m.puertas_cerradas, m.aedas_expulsados)),
        metrica("tec_compras", "Compras a los Aedas", "compras", INFO, Some(m.compras as f64), "Tecnologia_Aedas §6.4", &format!("Oro pagado {:.0} u; turnos con algo revelado y sin el oro {}. Rechazos: {}.", m.compras_oro as f64 / 1000.0, m.compra_sin_oro, rechazos(x, "ComprarTecnologiaAeda"))),
        metrica("tec_residentes", "Aedas residentes llegados", "Aedas", INFO, Some(m.residentes_llegan as f64), "Tecnologia_Aedas §7.1", &format!("Se fueron {}; al final {}.", m.residentes_se_van, n.residentes().aedas.len())),
        metrica("tec_epicas", "Épicas empezadas", "épicas", INFO, Some(m.epicas_empezadas as f64), "Tecnologia_Aedas §7.2", &format!("Capítulos cerrados {}, cumplidas {} (`aedas.epica_cumplida`: {}), abandonadas por orden {} (atascadas {} días sin un hecho: {}); perdidas por motivo: {}. Obras para un capítulo {} (rechazos: {}). Rechazos al empezar: {}.", m.capitulos, m.epicas_cumplidas, n_hecho(x, "aedas.epica_cumplida"), m.epicas_abandonadas, tec::ATASCADA_DIAS, m.epicas_atascadas, perdidas, m.obras_epica, rechazos(x, "ObraDeEpica"), rechazos(x, "EmpezarEpica"))),
    ]
}

fn alijos(x: &Entrada, t: &Tec) -> Vec<Metrica> {
    let m = &t.m;
    vec![metrica(
        "tec_alijos",
        "Alijos abiertos",
        "alijos",
        INFO,
        Some(m.alijos_abiertos as f64),
        "Generacion_Del_Mundo §12",
        &format!("Oro de los alijos {:.0} u; cargado de todo lo que traían (oro, madera, piedra o trigo) {:.0} u (lo que no cabe se pierde); segundos intentos (rechazados) {}; alijos a los que la columna no llegó {}. Rechazos de `AbrirAlijo`: {}.", m.alijos_oro as f64 / 1000.0, m.alijos_cargado as f64 / 1000.0, m.reintentos, m.alijos_inalcanzables, rechazos(x, "AbrirAlijo")),
    )]
}

fn invariantes(t: &Tec) -> Vec<Metrica> {
    let v = &t.v;
    let uno = |id: &str, nombre: &str, valor: u64, origen: &str, nota: &str| metrica(id, nombre, "violaciones", max(0.0), Some(valor as f64), origen, nota);
    vec![
        uno("inv_logro", "Logros que cambian, se fijan antes de su umbral o no se fijan al cruzarlo", v.logro, "Tecnologia_Aedas §2", "Tras cada orden: un logro fijado no cambia; uno nuevo se fijó en el tramo y con su contador cruzando el umbral en él; uno sin fijar no tiene el contador cruzado."),
        uno("inv_contador", "Contadores que bajan", v.contador, "Tecnologia_Aedas §2.1", "Tras cada orden, los 24."),
        uno("inv_adoptadas", "Adoptadas sin aparecer o reveladas ya aparecidas", v.adoptadas, "Tecnologia_Aedas §1", "Tras cada orden, cada Facción."),
        uno("inv_era_cerrada", "Tecnologías aparecidas de una Era cerrada", v.era_cerrada, "Tecnologia_Aedas §3", "En cada `tecnologia.aparece`, `aedas.venta` y `aedas.epica_tecnologia`, contra la Era de los hechos leídos hasta él; y tras cada orden, en el estado."),
        uno("inv_era", "Eras fuera de su mínimo o de su plazo", v.era, "Tecnologia_Aedas §3 paso 2 (mínimo de 1 semana, decisión del autor 2026-10-25)", "Tras cada orden: la Era pasa de una en una, no antes de su mínimo ni después de su plazo, ni sigue pasado su plazo ni, con todos sus logros, pasados su mínimo y su último logro."),
        uno("inv_sin_logro", "Tecnologías aparecidas o compradas sin su logro", v.sin_logro, "Tecnologia_Aedas §6.4 (comprar nunca salta el logro)", "En cada hecho de aparición y en cada compra aceptada (logro cumplido y conocida por los Aedas); y tras cada orden, en el estado."),
        uno("inv_epica", "Dos épicas de la misma tecnología en una Facción, o de una ya aparecida", v.epica, "Tecnologia_Aedas §7.2 (unificadas al trasladar, decisión del autor 2026-10-25)", "Tras cada orden, también tras una anexión o una fusión."),
        uno("inv_doble", "Contadores que no suben lo que dicen sus hechos", v.doble, "Tecnologia_Aedas §2.1 (D18)", "En cada observación, por tramo: los 14 contadores discretos suben exactamente lo que dicen los hechos leídos (un `combate.asedio_*` cuenta y su `batalla.aplicada` no); `conHoplitas` y `conResidentes`, que no viajan en el hecho, no pasan de los suyos."),
        uno("inv_itinerantes", "Itinerantes que bajan", v.itinerantes, "Tecnologia_Aedas §6.2", "Tras cada orden."),
        uno("inv_puerta_aedas", "Aedas parados en una plaza que les cierra la puerta", v.puerta, "Tecnologia_Aedas §6.2 (Q5)", "Tras cada orden, también justo después de cerrarles la puerta."),
        uno("inv_alijo", "Alijos abiertos dos veces por el mismo héroe", v.alijo, "Generacion_Del_Mundo §12", "En cada `alijo.abierto` (y lo cargado no pasa del oro), y en cada segundo intento: se rechaza con `alijo.ya_abierto` sin cambiar la versión."),
        uno("inv_cronica", "Entradas de la crónica guardada distintas de las emitidas", v.cronica, "Tecnologia_Aedas §8 (Q2)", "Al final, el diario entero leído por páginas contra lo que salió en los cambios de versión, también tras la foto ida y vuelta."),
    ]
}

fn grafico(t: &Tec) -> Grafico {
    let s = |nombre: &str, f: &dyn Fn(&MuestraTec) -> usize| serie(nombre, t.muestras.iter().map(|u| (u.dia as f64, f(u) as f64)).collect());
    Grafico {
        id: "tecnologia_dia".into(),
        titulo: "Logros y tecnologías".into(),
        tipo: TipoGrafico::Linea,
        eje_x: "día de mundo".into(),
        eje_y: "cuántas".into(),
        series: vec![s("Logros", &|u| u.logros), s("Aparecidas", &|u| u.aparecidas), s("Adoptadas", &|u| u.adoptadas), s("Reveladas", &|u| u.reveladas), s("Era", &|u| u.era as usize + 1), s("Aedas itinerantes", &|u| u.itinerantes), s("Aedas residentes", &|u| u.residentes), s("Épicas en curso", &|u| u.epicas)],
    }
}
