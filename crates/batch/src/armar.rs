//! Monta el `Informe` del sistema «mundo y territorio» con las medidas de la simulación.

use crate::fondo;
use crate::geo::{Cargado, Geo, ANILLOS};
use crate::medidas::{self as md, pct, Malla, Plaza, PLAZAS_POR_FOCO};
use crate::sim::{self, Corrida};
use crate::Escenario;
use informe::{Anomalia, Capa, Causa, Circulo, Grafico, Informe, Mapa, Metrica, Parametro, Punto, Semaforo, Serie, TipoGrafico};
use std::collections::BTreeMap;
use std::time::Duration;

/// Redondeo a 3 decimales: el JSON queda estable y legible.
pub(crate) fn r3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn metrica(id: &str, nombre: &str, unidad: &str, rango: (Option<f64>, Option<f64>), medido: Option<f64>, origen: &str, nota: &str) -> Metrica {
    Metrica {
        id: id.into(),
        nombre: nombre.into(),
        unidad: unidad.into(),
        esperado_min: rango.0,
        esperado_max: rango.1,
        medido: medido.filter(|v| v.is_finite()).map(r3),
        origen: origen.into(),
        nota: Some(nota.into()),
        margen_ambar: None,
    }
}

pub(crate) const INFO: (Option<f64>, Option<f64>) = (None, None);
pub(crate) const fn min(v: f64) -> (Option<f64>, Option<f64>) {
    (Some(v), None)
}
pub(crate) const fn max(v: f64) -> (Option<f64>, Option<f64>) {
    (None, Some(v))
}

pub(crate) fn serie(nombre: &str, puntos: Vec<(f64, f64)>) -> Serie {
    Serie { nombre: nombre.into(), puntos }
}

pub(crate) fn parametro(nombre: &str, valor: impl ToString) -> Parametro {
    Parametro { nombre: nombre.into(), valor: valor.to_string() }
}

/// Fecha de hoy (UTC) como `AAAA-MM-DD`.
pub fn hoy() -> String {
    let dias = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() / 86_400) as i64;
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{m:02}-{d:02}", yoe + era * 400 + (m <= 2) as i64)
}

/// Todo lo medido, listo para montar.
pub struct Entrada<'a> {
    pub c: &'a Cargado,
    pub geo: &'a Geo,
    pub e: &'a Escenario,
    pub corrida: &'a Corrida,
    pub malla: &'a Malla,
    pub determinismo: Option<bool>,
    /// Tiempo de pared de todo el escenario, hasta antes de montar el informe.
    pub pared_total: Duration,
}

pub fn armar(x: &Entrada) -> Informe {
    let n = &x.corrida.nucleo;
    let plazas = md::plazas_finales(n, x.geo);
    let mut metricas = Vec::new();
    metricas.extend(invariantes(x, &plazas));
    metricas.extend(fundaciones(x));
    metricas.extend(recursos_en_la_zona(x, &plazas));
    metricas.extend(vida_de_las_plazas(x, &plazas));
    metricas.extend(bosque(x, &plazas));
    metricas.extend(fronteras(x, &plazas));
    metricas.extend(reparto(x, &plazas));
    let (m13, grafico_metales) = metales(x, &plazas);
    metricas.extend(m13);
    metricas.extend(rendimiento(x));
    metricas.push(metrica(
        "determinismo",
        "Determinismo (mismo escenario reducido, dos veces)",
        "1 = igual",
        (Some(1.0), Some(1.0)),
        x.determinismo.map(|ok| ok as u8 as f64),
        "Arquitectura/13 §4 (núcleo determinista)",
        "Se repite un escenario reducido (25 Facciones, 2 días) con la misma semilla y se comparan los `hash_estado()` finales.",
    ));
    let anomalias = anomalias(&metricas);
    let resumen = format!(
        "Se simularon {} días con {} Facciones sintéticas sobre el mundo real: {} plazas fundadas. Resumen automático; la lectura la añade quien revise los resultados.",
        x.e.dias,
        x.e.facciones,
        plazas.len()
    );
    Informe {
        sistema: "Mundo y territorio".into(),
        fecha: hoy(),
        mundo_hash: x.c.mundo.hash().into(),
        semilla: x.e.semilla,
        dias_simulados: x.e.dias as f64,
        resumen,
        parametros: parametros(x),
        metricas,
        series: [vec![fundadas_por_dia(x), histograma_radios(&plazas), grafico_rechazos(x), grafico_ascensos(x)], vec![grafico_metales]].concat(),
        mapa: Some(mapa(x, &plazas)),
        anomalias,
        decisiones: vec![],
    }
}

fn parametros(x: &Entrada) -> Vec<Parametro> {
    let r = &x.corrida.nucleo.estado().reglas.territorio;
    let u = |v: i64| format!("{} u", v as f64 / 1000.0);
    let lista = |v: &[i64]| v.iter().map(|&t| (t / 1000).to_string()).collect::<Vec<_>>().join(" / ");
    vec![
        parametro("Facciones simuladas", x.e.facciones),
        parametro("Días de mundo", x.e.dias),
        parametro("Semilla", x.e.semilla),
        parametro("Hash del mundo", x.c.mundo.hash()),
        parametro("Radio inicial de la zona", u(r.radio_inicial)),
        parametro("Techos de zona por nivel", format!("{} u", lista(&r.techos))),
        parametro("Crecimiento de la zona por edificio completado (lo aplica la economía real)", format!("{} u", r.crecimiento_por_edificio as f64 / 1000.0)),
        parametro("Separación mínima entre plazas", u(r.separacion_plazas)),
        parametro("Exclusión junto a campamentos de mercenarios", u(r.exclusion_campamento)),
        parametro("Cap de fundación por nivel de Facción", format!("{:?}", r.cap_por_nivel)),
        parametro("Cuota de parcelas por masa y asentamiento", format!("{} %", r.cuota_parcelas_pct)),
        parametro("Supuesto del batch: nacimientos", format!("repartidos al azar en las primeras {} h, en el campamento i mod n", sim::NACEN_EN_HORAS)),
        parametro("Supuesto del batch: columna", format!("velocidad {} u/min, víveres {}", sim::VELOCIDAD as f64 * 0.06, sim::VIVERES / 1000)),
        parametro("Supuesto del batch: búsqueda de sitio", format!("{} candidatos; 30-120 u del campamento (expansión: 30-70 u de una plaza propia); se descartan los que el aviso de viabilidad da por condenados a ruina", sim::CANDIDATOS)),
        parametro("Supuesto del batch: reintento", format!("si todo es Rojo, a los {} min; máximo {} reintentos", sim::REINTENTO_MIN, sim::MAX_REINTENTOS)),
        parametro("Construcción, zona, nivel de plaza y parcelas de bosque", "la economía real del núcleo (crate `asentamiento`, un evento de un minuto por plaza): el batch no simula edificios, Leñeras ni niveles de plaza; solo funda y mide"),
        parametro("Supuesto del batch: ascenso", format!("el «jugador» sintético pide `SolicitarAscenso` a cada plaza viva cada {} min; lo concede o rechaza la economía real", sim::ASCENSO_CADA_MIN)),
        parametro("Supuesto del batch: nivel de la Facción", format!("solo fija el cap de fundación: nivel 2 a las {} h y nivel 3 a las {} h de fundar la primera plaza", sim::NIVEL_FACCION_2_H, sim::NIVEL_FACCION_3_H)),
        parametro("Supuesto del batch: expansión", format!("cada {} h, si la plaza viva más antigua alcanzó de verdad el nivel 2+ (`nivel()` de la economía) y el cap lo permite; tras abandonar una búsqueda, pausa de {} h", sim::EXPANSION_CADA_H, sim::EXPANSION_PAUSA_H)),
    ]
}

// ───────────────────────────── Métricas ─────────────────────────────

/// 1, 2 y 3: lo que las reglas prohíben.
fn invariantes(x: &Entrada, plazas: &[Plaza]) -> Vec<Metrica> {
    let (minima, ajenas) = md::separacion_y_zonas_ajenas(&x.corrida.nucleo, plazas);
    let excl = x.corrida.nucleo.estado().reglas.territorio.exclusion_campamento;
    let cerca = md::cerca_de_campamento(plazas, x.geo, excl);
    vec![
        metrica("separacion_min", "Distancia mínima entre plazas", "u", min(30.0), minima, "Doc 1.0a (separación entre plazas)", "Mínimo sobre todos los pares de plazas al final de la simulación. Invariante: el núcleo rechaza fundar a menos de 30 u de otra plaza."),
        metrica(
            "plazas_en_zona_ajena",
            "Plazas dentro de la zona de otra Facción",
            "plazas",
            max(0.0),
            Some(ajenas as f64),
            "Doc 1.2 (no se funda dentro de una zona ajena)",
            "Comprobación a posteriori con las zonas reales del instante final (disco recortado por las fronteras, `Nucleo::en_zona`) de las plazas de OTRAS Facciones. El núcleo ya rechaza fundar dentro de una zona; si una plaza fundada antes queda luego bajo el disco de otra ajena, la frontera dura debe dejarla fuera de su zona real. Un valor distinto de 0 es un fallo de la frontera.",
        ),
        metrica("plazas_cerca_campamento", "Plazas a menos de 20 u de un campamento de mercenarios", "plazas", max(0.0), Some(cerca as f64), "Doc 1.8 (exclusión junto al campamento)", "Plazas finales cuya distancia a algún campamento es menor que la exclusión (20 u). Invariante."),
    ]
}

/// 4, 5 y 6: cuántas fundaciones, cuánto tardan y cómo de buenos son los sitios.
fn fundaciones(x: &Entrada) -> Vec<Metrica> {
    let reg = &x.corrida.reg;
    let facciones = x.corrida.facciones as usize;
    let con_plaza = reg.fundaciones.iter().filter(|f| f.primera).count();
    let rechazos: u64 = reg.rechazos.values().sum();
    let verdes = reg.fundaciones.iter().filter(|f| f.verde).count();
    let por_anillo = |i: usize| pct(x.malla.verdes[i], x.malla.fundables[i]).map_or("sin datos".to_string(), |v| format!("{v:.1} %"));
    let nota_verde = format!(
        "Color del aviso de viabilidad en el punto exacto donde se fundó (bosque libre y piedra en la zona inicial). Medido sobre una malla de sondeo ANTES de fundar nada (hasta {} puntos fundables al azar por anillo; mide el mundo, no la política), los puntos fundables que son verdes son: foco {}, frontera {}, lejana {}.",
        md::PUNTOS_POR_ANILLO,
        por_anillo(0),
        por_anillo(1),
        por_anillo(2)
    );
    let mut v = vec![
        metrica("fundaciones_por_faccion", "Plazas fundadas por Facción", "plazas", INFO, (facciones > 0).then(|| reg.fundaciones.len() as f64 / facciones as f64), "Doc 1.2 (cap de fundación por nivel)", "Todas las plazas fundadas entre las Facciones nacidas (informativa; el cap por nivel de Facción las limita a 1, 2 o 3 con estos niveles)."),
        metrica("facciones_con_plaza", "Facciones con al menos una plaza", "%", INFO, pct(con_plaza, facciones), "Doc 1.2", "De las Facciones que quisieron fundar, las que lo lograron (informativa)."),
        metrica("renuncias", "Búsquedas de sitio abandonadas", "búsquedas", INFO, Some(reg.renuncias.len() as f64), "Doc 1.2", "Búsquedas que agotaron los reintentos (6 reintentos a 10 min). Incluye primeras plazas y expansiones (informativa)."),
        metrica("rechazos_totales", "Candidatos o fundaciones rechazados", "rechazos", INFO, Some(rechazos as f64), "Doc 1.2", "Suma de los motivos del gráfico «Motivos de rechazo»: candidatos rojos, rutas imposibles y fundaciones fallidas tras llegar."),
        metrica("tiempo_primera_plaza", "Tiempo medio hasta la primera plaza", "min", max(30.0), md::tiempo_primera_plaza_min(x.corrida), "Doc 1.2 (arranque rápido)", "Desde que nace la Facción hasta que funda (media solo de las Facciones que lo lograron). Caminar 30-120 u a 30 u/min son 1-4 min; el resto es buscar sitio."),
        metrica("plazas_verdes", "Plazas que nacen en sitio verde", "%", min(50.0), pct(verdes, reg.fundaciones.len()), "Doc 1.2 (piedra y bosque desde el principio)", &nota_verde),
    ];
    for (i, nombre) in ANILLOS.iter().enumerate() {
        v.push(metrica(&format!("malla_verde_{nombre}"), &format!("Puntos fundables verdes en el anillo {nombre} (malla de sondeo)"), "%", INFO, pct(x.malla.verdes[i], x.malla.fundables[i]), "Doc 1.2", "Malla de ~2000 puntos aleatorios de tierra firme por anillo antes de fundar nada: mide el mundo, no la política."));
    }
    v
}

/// 7: piedra y bosque al alcance de la zona máxima.
fn recursos_en_la_zona(x: &Entrada, plazas: &[Plaza]) -> Vec<Metrica> {
    let (sin_piedra, sin_bosque) = md::sin_recursos_en_zona_maxima(&x.corrida.nucleo, plazas, x.geo);
    vec![
        metrica("sin_piedra", "Plazas sin ninguna piedra en su zona máxima", "%", max(15.0), pct(sin_piedra, plazas.len()), "Doc 1.2 (sin piedra no hay Cantera ni nivel 2)", "Plazas sin ningún nodo de piedra a menos que el techo de su nivel actual (disco, sin recortes)."),
        metrica("sin_bosque", "Plazas sin ninguna parcela de bosque en su zona máxima", "%", max(10.0), pct(sin_bosque, plazas.len()), "Doc 1.2 (sin bosque no hay Leñera)", "Plazas sin el centro de ninguna parcela a menos que el techo de su nivel actual (disco, sin recortes)."),
    ]
}

/// Lo que la economía real de las plazas deja al territorio: cuántas sobreviven y qué techo de zona alcanzan (informativas).
fn vida_de_las_plazas(x: &Entrada, plazas: &[Plaza]) -> Vec<Metrica> {
    let reg = &x.corrida.reg;
    let fundadas = reg.fundaciones.len();
    let nivel_2 = plazas.iter().filter(|p| p.nivel >= 2).count();
    let nivel_3 = plazas.iter().filter(|p| p.nivel >= 3).count();
    let ascensos = |c: &str| reg.ascensos.get(c).copied().unwrap_or(0);
    vec![
        metrica("plazas_en_ruinas", "Plazas fundadas que cayeron en ruinas", "%", INFO, pct(fundadas - plazas.len().min(fundadas), fundadas), "Economia_Construccion (ruinas)", "Fundadas menos vivas al final. Las arruina la economía real de la plaza (p. ej. sin madera suficiente), no el batch (informativa)."),
        metrica("plazas_nivel_2", "Plazas vivas de nivel 2 o más", "%", INFO, pct(nivel_2, plazas.len()), "Doc 1.9c (techo de zona por nivel)", "El nivel lo sube la economía real con el ascenso que pide el «jugador» sintético (informativa). Con nivel 2 el techo de la zona es de 15 u."),
        metrica("plazas_nivel_3", "Plazas vivas de nivel 3 o más", "%", INFO, pct(nivel_3, plazas.len()), "Doc 1.9c (techo de zona por nivel)", "Con nivel 3 el techo es de 20 u; hasta entonces dos plazas a 30 u no pueden tocarse y no hay contactos de frontera (informativa)."),
        metrica("ascensos_aceptados", "Ascensos pedidos y aceptados por la economía", "órdenes", INFO, Some(ascensos("ok") as f64), "Economia_Construccion §11", "Órdenes `SolicitarAscenso` aceptadas, una petición por plaza y hora (informativa)."),
        metrica("ascensos_rechazados", "Ascensos pedidos y rechazados por la economía", "órdenes", INFO, Some(reg.ascensos.iter().filter(|(c, _)| *c != "ok").map(|(_, k)| k).sum::<u64>() as f64), "Economia_Construccion §11", "Rechazos por población, edificios, cupo, recursos... (informativa; el detalle por motivo no se desglosa)."),
    ]
}

/// Cuota por masa y ocupación del bosque.
fn bosque(x: &Entrada, plazas: &[Plaza]) -> Vec<Metrica> {
    let n = &x.corrida.nucleo;
    let (mayor, llenas) = md::cuota_y_masas_llenas(n, plazas, x.geo);
    let ocupadas = n.estado().parcelas_ocupadas.len();
    vec![
        metrica("cuota_max_masa", "Máxima fracción de una masa de bosque ocupada por un solo asentamiento", "%", max(50.0), mayor, "Doc 1.9c (cuota de parcelas por asentamiento)", "Mayor parcelas_del_asentamiento / parcelas_de_la_masa sobre todas las plazas y masas de 2 o más parcelas (con una sola parcela la cuota mínima es 1, así que 100 % no es una infracción)."),
        metrica("parcelas_ocupadas", "Parcelas ocupadas del mundo", "%", INFO, pct(ocupadas, x.geo.parcelas.len()), "Doc 1.9c", "Parcelas con Leñera sobre el total del artefacto (informativa). La economía real de cada plaza (no una petición sintética) reclama las parcelas, así que depende de cuánto construyan las plazas en el tiempo simulado."),
        metrica("masas_agotadas", "Masas de bosque con todas sus parcelas ocupadas", "masas", INFO, Some(llenas as f64), "Doc 1.9c", "Masas cuyas parcelas están todas explotadas (informativa). La economía real de la plaza (no una petición sintética) reclama las parcelas."),
    ]
}

/// 11: fronteras y área perdida.
fn fronteras(x: &Entrada, plazas: &[Plaza]) -> Vec<Metrica> {
    let n = &x.corrida.nucleo;
    let (media, maximo) = md::contactos(n, plazas);
    let (perdida, perdida_max) = md::area_perdida(n, plazas);
    vec![
        metrica("contactos_media", "Contactos de frontera por plaza (media)", "contactos", INFO, media, "Doc 1.2 (fronteras duras)", "Cada par de plazas de Facciones distintas cuyas zonas se han tocado cuenta un contacto en cada una (informativa). Con techos de zona de 10 y 15 u (niveles 1 y 2) y plazas a 30 u o más no hay contactos: hacen falta plazas de nivel 3."),
        metrica("contactos_max", "Contactos de frontera por plaza (máximo)", "contactos", INFO, Some(maximo as f64), "Doc 1.2", "La plaza con más vecinas de otra Facción en contacto (informativa)."),
        metrica("area_perdida", "Área de la zona perdida por recortes (media)", "%", max(25.0), perdida, "Doc 1.2 (las zonas no se estrangulan)", "Área del polígono real (`poligono_zona`) frente al polígono de 48 lados de su radio sin recortar, media sobre todas las plazas al final. Sin contactos (ver «Contactos de frontera») no hay recortes y la métrica sale en verde sin haberse puesto a prueba."),
        metrica("area_perdida_max", "Área de la zona perdida por recortes (máxima)", "%", INFO, perdida_max, "Doc 1.2", "La plaza más recortada (informativa)."),
    ]
}

/// 12: plazas por campamento y saturación del foco.
fn reparto(x: &Entrada, plazas: &[Plaza]) -> Vec<Metrica> {
    let campamentos = x.geo.campamentos.len();
    let mut total = vec![0usize; campamentos];
    let mut en_foco = vec![0usize; campamentos];
    for p in plazas {
        total[p.campamento] += 1;
        en_foco[p.campamento] += (p.anillo == 0) as usize;
    }
    let sat = |k: usize| 100.0 * k as f64 / PLAZAS_POR_FOCO;
    let media = (campamentos > 0).then(|| plazas.len() as f64 / campamentos as f64);
    vec![
        metrica("plazas_por_campamento", "Plazas fundadas por campamento (media)", "plazas", INFO, media, "Doc 1.9c (≈ 45 plazas por foco de 150 u)", "Cada plaza se asigna al campamento más cercano (informativa)."),
        metrica("saturacion_foco_media", "Saturación del foco (media)", "%", INFO, (campamentos > 0).then(|| sat(en_foco.iter().sum::<usize>()) / campamentos as f64), "Doc 1.9c", "Plazas en el anillo foco de cada campamento entre 45, media sobre los campamentos (informativa)."),
        metrica("saturacion_foco_max", "Saturación del foco (máxima)", "%", INFO, en_foco.iter().copied().max().map(sat), "Doc 1.9c", "El foco más lleno (informativa)."),
    ]
}

/// 13: cobertura de los metales por anillo, y su gráfico.
fn metales(x: &Entrada, plazas: &[Plaza]) -> (Vec<Metrica>, Grafico) {
    let (cob, por_anillo) = md::cobertura_de_metales(&x.corrida.nucleo, plazas, x.geo);
    let total: usize = por_anillo.iter().sum();
    let mut v = Vec::new();
    for (t, tipo) in ["estaño", "oro"].iter().enumerate() {
        for (a, anillo) in ANILLOS.iter().enumerate() {
            v.push(metrica(&format!("cobertura_{}_{anillo}", ["estano", "oro"][t]), &format!("Nodos de {tipo} cubiertos por alguna zona, anillo {anillo}"), "%", INFO, cob[t][a], "Doc 1.9c (gradiente de metales 5 / 25 / 70 %)", "Nodos dentro de la zona real de alguna plaza al final (informativa). El anillo es el declarado en el artefacto."));
        }
    }
    let eje = |s: &str, f: &dyn Fn(usize) -> f64| serie(s, (0..3).map(|a| (a as f64, r3(f(a)))).collect());
    let g = Grafico {
        id: "gradiente_metales".into(),
        titulo: "Metales: nodos por anillo y cobertura de las zonas".into(),
        tipo: TipoGrafico::Barras,
        eje_x: "anillo (0 foco, 1 frontera, 2 lejana)".into(),
        eje_y: "%".into(),
        series: vec![
            eje("Nodos del anillo (diseño 5 / 25 / 70)", &|a| 100.0 * por_anillo[a] as f64 / total.max(1) as f64),
            eje("Estaño cubierto", &|a| cob[0][a].unwrap_or(0.0)),
            eje("Oro cubierto", &|a| cob[1][a].unwrap_or(0.0)),
        ],
    };
    (v, g)
}

/// 14: rendimiento.
fn rendimiento(x: &Entrada) -> Vec<Metrica> {
    let c = x.corrida;
    let dias = x.e.dias.max(1) as f64;
    vec![
        metrica("pared_total", "Tiempo de pared total del escenario", "s", max(120.0), Some(x.pared_total.as_secs_f64()), "Fase 6 (rendimiento)", "Carga de datos, sondeo de la malla, simulación, determinismo."),
        metrica("pared_por_dia", "Tiempo de pared por día simulado", "s/día", INFO, Some(c.pared.as_secs_f64() / dias), "Fase 6 (rendimiento)", "Solo la simulación principal (informativa)."),
        metrica("eventos", "Eventos procesados por el núcleo", "eventos", INFO, Some(c.nucleo.metricas().eventos as f64), "Fase 6 (rendimiento)", "`Nucleo::metricas().eventos` (informativa)."),
        metrica("viabilidad_llamadas", "Llamadas a `viabilidad`", "llamadas", INFO, Some(c.reg.viab_llamadas as f64), "Fase 6 (rendimiento)", "Búsqueda de sitio y comprobación al llegar (informativa)."),
        metrica("viabilidad_tiempo", "Tiempo en `viabilidad`", "s", INFO, Some(c.reg.viab_tiempo.as_secs_f64()), "Fase 6 (rendimiento)", "Suma de lo que tardaron esas llamadas (informativa)."),
    ]
}

// ───────────────────────────── Gráficos y mapa ─────────────────────────────

fn fundadas_por_dia(x: &Entrada) -> Grafico {
    let fundaciones = &x.corrida.reg.fundaciones;
    let puntos = (0..=x.e.dias).map(|d| (d as f64, fundaciones.iter().filter(|f| f.instante <= d as i64 * 86_400_000).count() as f64)).collect();
    Grafico { id: "fundadas_por_dia".into(), titulo: "Plazas fundadas (acumuladas)".into(), tipo: TipoGrafico::Linea, eje_x: "día de mundo".into(), eje_y: "plazas".into(), series: vec![serie("Plazas", puntos)] }
}

fn histograma_radios(plazas: &[Plaza]) -> Grafico {
    let mut cubos = BTreeMap::new();
    plazas.iter().for_each(|p| *cubos.entry(p.radio / 1000).or_insert(0usize) += 1);
    let puntos = cubos.into_iter().map(|(r, k)| (r as f64, k as f64)).collect();
    Grafico { id: "radio_zona".into(), titulo: "Radio de la zona al final".into(), tipo: TipoGrafico::Histograma, eje_x: "radio (u)".into(), eje_y: "plazas".into(), series: vec![serie("Plazas", puntos)] }
}

fn grafico_rechazos(x: &Entrada) -> Grafico {
    let series = x.corrida.reg.rechazos.iter().map(|(m, k)| serie(m, vec![(0.0, *k as f64)])).collect();
    Grafico { id: "rechazos".into(), titulo: "Motivos de rechazo".into(), tipo: TipoGrafico::Barras, eje_x: "motivo".into(), eje_y: "veces".into(), series }
}

fn grafico_ascensos(x: &Entrada) -> Grafico {
    let series = x.corrida.reg.ascensos.iter().map(|(m, k)| serie(m, vec![(0.0, *k as f64)])).collect();
    Grafico { id: "ascensos".into(), titulo: "Ascensos pedidos: resultado de la economía real".into(), tipo: TipoGrafico::Barras, eje_x: "resultado".into(), eje_y: "órdenes (una por plaza y hora)".into(), series }
}

pub(crate) fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn mapa(x: &Entrada, plazas: &[Plaza]) -> Mapa {
    let u = |v: i64| v as f64 / 1000.0;
    let capa = |nombre: &str, color: &str, circulos, puntos| Capa { nombre: nombre.into(), color: color.into(), circulos, puntos };
    let punto = |p: base::Punto| Punto { x: u(p.x), y: u(p.y) };
    Mapa {
        ancho: u(x.geo.ancho),
        alto: u(x.geo.alto),
        fondo: fondo::fondo(&x.c.mundo),
        capas: vec![
            capa("Zonas", "#3b82f6", plazas.iter().map(|p| Circulo { x: u(p.pos.x), y: u(p.pos.y), r: u(p.radio) }).collect(), vec![]),
            capa("Plazas", "#dc2626", vec![], plazas.iter().map(|p| punto(p.pos)).collect()),
            capa("Campamentos de mercenarios", "#16a34a", vec![], x.geo.campamentos.iter().map(|&p| punto(p)).collect()),
            capa("Estaño", "#c026d3", vec![], x.geo.metales.iter().filter(|m| m.tipo == 0).map(|m| punto(m.p)).collect()),
            capa("Oro", "#facc15", vec![], x.geo.metales.iter().filter(|m| m.tipo == 1).map(|m| punto(m.p)).collect()),
            // Capas sin geometría: solo explican los colores del fondo (los dibuja `fondo`, un píxel por celda).
            capa("Piedra (píxel)", &hex(fondo::PIEDRA), vec![], vec![]),
            capa("Parcela de bosque (píxel)", &hex(fondo::PARCELA), vec![], vec![]),
            capa("Bosque", &hex(fondo::BOSQUE), vec![], vec![]),
            capa("Río", &hex(fondo::RIO), vec![], vec![]),
            capa("Agua", &hex(fondo::AGUA), vec![], vec![]),
            capa("Llano", &hex(fondo::LLANO), vec![], vec![]),
            capa("Colina", &hex(fondo::COLINA), vec![], vec![]),
            capa("Montaña", &hex(fondo::MONTANA), vec![], vec![]),
        ],
    }
}

/// Anomalías automáticas: solo violaciones de invariante (métricas 1, 2, 3 y 9).
fn anomalias(metricas: &[Metrica]) -> Vec<Anomalia> {
    ["separacion_min", "plazas_en_zona_ajena", "plazas_cerca_campamento", "cuota_max_masa"]
        .iter()
        .filter_map(|id| metricas.iter().find(|m| m.id == *id))
        .filter(|m| !matches!(informe::semaforo(m), Semaforo::Verde | Semaforo::Informativa | Semaforo::SinDato))
        .map(|m| Anomalia {
            titulo: format!("Invariante incumplido: {}", m.nombre),
            detalle: format!("Medido {} {} (esperado entre {:?} y {:?}). {}", m.medido.unwrap_or(f64::NAN), m.unidad, m.esperado_min, m.esperado_max, m.nota.clone().unwrap_or_default()),
            causa: Some(Causa::Codigo),
        })
        .collect()
}
