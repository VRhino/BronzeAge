//! Monta el `Informe` del sistema «población, construcción y mantenimiento» con lo medido en la simulación de plazas.

use crate::armar::{hex, hoy, metrica, min, max, parametro, serie, INFO};
use crate::fondo;
use crate::geo::Cargado;
use crate::economia::Mapa;
use crate::simpob::{self as sp, Corrida, Villa};
use crate::EscenarioPoblacion;
use asentamiento::catalogo::{Tipo, TIPOS};
use asentamiento::datos::Reglas;
use informe::{Anomalia, Capa, Causa, Circulo, Grafico, Informe, Mapa as MapaInforme, Metrica, Punto, TipoGrafico};
use std::collections::BTreeMap;
use std::time::Duration;
use trazado::Rol;

const MIN_POR_DIA: f64 = 1440.0;

/// Todo lo medido, listo para montar.
pub struct Entrada<'a> {
    pub c: &'a Cargado,
    pub mapa: &'a Mapa,
    pub e: &'a EscenarioPoblacion,
    pub corrida: &'a Corrida,
    pub reglas: &'a Reglas,
    pub urb: &'a nucleo::urbanismo::Urbanismo,
    pub determinismo: Option<bool>,
    pub pared_total: Duration,
}

pub(crate) fn pct(a: usize, b: usize) -> Option<f64> {
    (b > 0).then(|| 100.0 * a as f64 / b as f64)
}

/// Percentil `p` (0..1) de una lista; la ordena.
pub(crate) fn percentil(v: &mut [f64], p: f64) -> Option<f64> {
    v.sort_by(f64::total_cmp);
    (!v.is_empty()).then(|| v[((v.len() - 1) as f64 * p).round() as usize])
}

pub(crate) fn mediana(v: &[f64]) -> Option<f64> {
    percentil(&mut v.to_vec(), 0.5)
}

/// Mediana de un instante por plaza (en minutos), en la unidad `div`; las que no llegan cuentan como `tope` (censura a la derecha).
fn mediana_censurada(v: &[Option<i64>], div: f64, tope: f64) -> Option<f64> {
    mediana(&v.iter().map(|m| m.map_or(tope, |m| m as f64 / div)).collect::<Vec<_>>())
}

pub fn armar(x: &Entrada) -> Informe {
    let c = x.corrida;
    let mut metricas = Vec::new();
    metricas.extend(colocacion(x));
    metricas.extend(nivel2(x));
    metricas.extend(salud(x));
    metricas.extend(construccion(x));
    metricas.extend(poblacion(x));
    metricas.extend(trigo_y_mantenimiento(x));
    metricas.extend(cuellos(x));
    metricas.extend(ciudad(x));
    metricas.extend(murallas(x));
    metricas.extend(rendimiento(x));
    metricas.push(metrica(
        "determinismo",
        "Determinismo (mismo escenario reducido, dos veces)",
        "1 = igual",
        (Some(1.0), Some(1.0)),
        x.determinismo.map(|ok| ok as u8 as f64),
        "Arquitectura/13 §4 (núcleo determinista)",
        "Se repite un escenario reducido (20 plazas, 3 días) con la misma semilla y se comparan los hashes del estado final de todas las plazas, los nodos y los reclamos.",
    ));
    let v = &c.violaciones;
    metricas.push(metrica(
        "invariantes",
        "Violaciones de los invariantes (§15)",
        "violaciones",
        max(0.0),
        Some(v.total() as f64),
        "Economia_Construccion §15 (Invariantes)",
        &format!(
            "Se comprueba tras cada paso: almacén en [0, capacidad] ({}), población y nutrición sin negativos ({}), medidor en [0, 100] ({}), nivel monótono y operativo ≤ alcanzado ({}); y al final, ningún nodo o parcela en dos edificios ({}) y reclamos coherentes con los edificios ({}). Entre paréntesis, las violaciones de cada una.",
            v.almacen, v.poblacion, v.medidor, v.nivel, v.duplicados, v.reclamos
        ),
    ));
    let ciudades = &c.ciudades;
    metricas.push(metrica(
        "invariantes_ciudad",
        "Violaciones de los invariantes de la ciudad (§13)",
        "violaciones",
        max(0.0),
        Some(ciudades.total() as f64),
        "Trazado_Urbano §13 (Invariantes)",
        &format!(
            "Se comprueban al final en las {} plazas que siguen en pie: piezas que se pisan ({}), piezas sobre una celda de calle o camino ({}), genéricos fuera del disco urbano de su `radioCiudad` o, en las afueras, fuera de su banda de 60 u.l. a max(radio, 96) ({}; anclas y satélites no miran el disco desde `LAYOUT_VERSION` 4), sin ninguna calle o camino pegado, es decir, sin salida a la red ({}), desacuerdos entre la ciudad y la economía (pieza sin su edificio con el mismo id y tipo, o edificio con huella en la rejilla sin pieza) ({}), y piezas sobre un muro o torre de un recinto o recintos de la ciudad que no son la proyección de los de la economía ({}). Entre paréntesis, las violaciones de cada una.",
            ciudades.plazas, ciudades.solape, ciudades.suelo, ciudades.disco, ciudades.sin_salida, ciudades.economia, ciudades.muro
        ),
    ));
    let anomalias = anomalias(&metricas);
    let resumen = format!(
        "Se simularon {} días de mundo con {} plazas sintéticas ({} colocadas) sobre el mundo real, minuto a minuto y en competencia por los nodos y parcelas. Resumen automático; la lectura la añade quien revise los resultados.",
        x.e.dias,
        x.e.plazas,
        c.villas.len()
    ) + " Cada plaza lleva su ciudad real (trazado urbano del núcleo).";
    Informe {
        sistema: "Población, construcción y mantenimiento".into(),
        fecha: hoy(),
        mundo_hash: x.c.mundo.hash().into(),
        semilla: x.e.semilla,
        dias_simulados: x.e.dias as f64,
        resumen,
        parametros: parametros(x),
        metricas,
        series: vec![grafico_pesants(x), grafico_nivel2(x), histograma_nivel2(x), grafico_edificios(x), grafico_sin_sitio(x), grafico_sin_sitio_ciudad(x)],
        mapa: Some(mapa(x)),
        anomalias,
        decisiones: vec![],
    }
}

fn parametros(x: &Entrada) -> Vec<informe::Parametro> {
    let r = x.reglas;
    let t = &x.mapa.reglas;
    let u = |v: i64| format!("{} u", v as f64 / 1000.0);
    let techos = t.techos.iter().map(|&v| (v / 1000).to_string()).collect::<Vec<_>>().join(" / ");
    let asc = |n: u32| r.nivel.ascenso.get(&n).map_or("—".to_string(), |a| format!("{:.1} días de obra", a.obra_minutos as f64 / MIN_POR_DIA));
    vec![
        parametro("Plazas pedidas / colocadas", format!("{} / {}", x.e.plazas, x.corrida.villas.len())),
        parametro("Días de mundo", x.e.dias),
        parametro("Huella del estado final (hash_estado del batch)", format!("{:016x}", sp::hash_estado(x.corrida))),
        parametro("Semilla", x.e.semilla),
        parametro("Hash del mundo", x.c.mundo.hash()),
        parametro("Escenario", "todas las plazas fundadas a la vez (t = 0), cada una de una Facción distinta; un bucle global por minuto avanza todas (competencia por nodos y parcelas)"),
        parametro("Colocación", format!("hasta {} candidatos a 30-120 u de un campamento (plazas repartidas por igual entre los {}), el primero Verde o el mejor Ámbar, descartando los sitios sin ninguna parcela de bosque a 10 u o menos (la plaza caería en ruinas: P1, 2026-10-18); hasta {} rondas si ninguno es fundable", sp::CANDIDATOS, x.mapa.campamentos.len(), sp::MAX_RONDAS)),
        parametro("Materiales iniciales", "FUNDACION.materialesIniciales (madera 50, piedra 20, trigo 100, oro 100) y 3 Viviendas iniciales"),
        parametro("Zona de cada plaza", format!("disco de {} al fundar, +{} por edificio completado, techos por nivel alcanzado {} u; SIN recorte por vecinas", u(t.radio_inicial), u(t.crecimiento_por_edificio), techos)),
        parametro("Cuota de parcelas por masa", format!("{} % por asentamiento", t.cuota_parcelas_pct)),
        parametro("Regeneración de nodos", format!("minerales a los {} min, ganado a los {} min (REGENERACION_NODOS)", crate::economia::REGENERA_MINERAL_MIN, crate::economia::REGENERA_GANADO_MIN)),
        parametro("Exterior", "fertilidad 1,0; sin tecnologías; nivel de Facción 1; 1 ciudadano; capital sin distancia; sin guarnición; factores extra 1,0; Era I (los recintos de muralla son de la economía de la plaza)"),
        parametro("Política del jugador", format!("cada {} min pide la subida de nivel si nada la bloquea{}; no activa políticas ni da otras órdenes manuales", sp::CADA_MIN_ASCENSO, if x.e.murallas { " y, sin recintos y con el nivel operativo que pide la empalizada (2), la compromete (la primera celda se levanta en el paso de ese mismo minuto)" } else { "; sin murallas (`--murallas 0`)" })),
        parametro("Cupo de Facción", format!("nivel 2: {:?} plazas con Facción de nivel 1 (cupo ocupado siempre 0: una plaza por Facción)", politica::nivel::cupo_nivel(&politica::Reglas::cargar().nivel, 1, 2))),
        parametro("Obra de ascenso al nivel 2", asc(2)),
        parametro("Ciudad de cada plaza", format!("la `Ciudad` del crate `trazado` con el mismo adaptador que el núcleo (`nucleo::urbanismo::Urbe`): fundada al nacer la plaza con semilla derivada de la del escenario ({}) y del índice de la plaza, con los edificios iniciales de la economía como piezas; los edificios se colocan de verdad (anclas gratuitas con el gate de nivel 2, puestos del Mercado, Granja que se muda al mejorar o la mejora espera hueco)", x.e.semilla)),
        parametro("Límites del batch", "sin pueblos extractores (minas: ninguna), sin tecnologías, sin recorte de zonas por vecinas, sin mejora del recinto a piedra (pide el nivel 3) ni ampliaciones, sin guarnición, saqueos ni conquistas (la muralla no se daña), sin otras órdenes manuales (no se construyen Mercados, Palacio ni edificios militares)"),
    ]
}

// ───────────────────────────── Métricas ─────────────────────────────

fn con_margen(mut m: Metrica, margen: f64) -> Metrica {
    m.margen_ambar = Some(margen);
    m
}

fn dias_de(v: &[Villa], f: impl Fn(&Villa) -> Option<i64>) -> Vec<Option<i64>> {
    v.iter().map(f).collect()
}

/// Cómo nacieron las plazas.
fn colocacion(x: &Entrada) -> Vec<Metrica> {
    let c = x.corrida;
    let v = &c.villas;
    vec![
        metrica("colocadas_verde_pct", "Plazas que nacen en sitio verde", "%", min(50.0), pct(v.iter().filter(|p| p.verde).count(), v.len()), "Doc 1.2 (piedra y bosque desde el principio)", "Parcelas libres y piedra en el disco inicial de 6 u, entre los 25 candidatos de la primera ronda (el primer verde, o el mejor ámbar si no hay)."),
        metrica("densidad_n1_ruinas", "Densidad total de bosque a 10 u de las plazas que caen en ruinas (mediana)", "madera/min", INFO, mediana(&v.iter().filter(|p| p.reg.ruinas.is_some()).map(|p| p.densidad_n1 as f64 / 1e6 * 5.0).collect::<Vec<_>>()).or(Some(0.0)), "Economia_Construccion §5.2 (Leñera 5 × densidad)", "Madera por minuto que daría el bosque a 10 u si se explotase entero (5 × la suma de densidades de sus parcelas), medido al fundar. Para comparar con el mantenimiento de madera (~1,65 al día 1, ~2,5 con 40 edificios). 0 si ninguna plaza cae."),
        metrica("densidad_n1_resto", "Densidad total de bosque a 10 u de las plazas que no caen (mediana)", "madera/min", INFO, mediana(&v.iter().filter(|p| p.reg.ruinas.is_none()).map(|p| p.densidad_n1 as f64 / 1e6 * 5.0).collect::<Vec<_>>()).or(Some(0.0)), "Economia_Construccion §5.2 (Leñera 5 × densidad)", "Lo mismo para las plazas que sobreviven."),
        metrica("sin_bosque_n1_pct", "Plazas sin ninguna parcela de bosque en el disco de 10 u", "%", max(10.0), pct(v.iter().filter(|p| p.parcelas_n1 == 0).count(), v.len()), "Doc 1.2 / D2 de bosque (sin bosque no hay Leñera)", "Parcelas del artefacto a 10 u o menos de la plaza (techo de la zona en el nivel 1) al fundar, antes de cualquier competencia."),
        metrica("sin_piedra_n1_pct", "Plazas sin ningún nodo de piedra en el disco de 10 u", "%", max(15.0), pct(v.iter().filter(|p| p.piedra_n1 == 0).count(), v.len()), "Doc 1.2 (sin piedra no hay Cantera ni nivel 2)", "Nodos de piedra a 10 u o menos de la plaza al fundar, antes de cualquier competencia."),
    ]
}

/// 1 y 2: la subida al nivel 2.
fn nivel2(x: &Entrada) -> Vec<Metrica> {
    let v = &x.corrida.villas;
    let dias = x.e.dias as f64;
    let n2 = dias_de(v, |p| p.reg.nivel2);
    let llegan = n2.iter().flatten().count();
    let ped = dias_de(v, |p| p.reg.ascenso_pedido);
    let con_sitio: Vec<&Villa> = v.iter().filter(|p| p.parcelas_n1 > 0 && p.piedra_n1 > 0).collect();
    vec![
        con_margen(metrica("nivel2_pct", &format!("Plazas que llegan al nivel 2 en {} días", x.e.dias), "%", min(90.0), pct(llegan, v.len()), "Doc 4.5 (ritmo de niveles aprobado; nivel 2 en torno a 3-5,8 días)", "Plazas con el evento `NivelSubio { nivel: 2 }` antes del final. Cada plaza es de una Facción distinta, así que el cupo de Facción no bloquea."), 10.0),
        con_margen(metrica("nivel2_con_sitio_pct", "Plazas con bosque y piedra a 10 u que llegan al nivel 2", "%", min(90.0), pct(con_sitio.iter().filter(|p| p.reg.nivel2.is_some()).count(), con_sitio.len()), "Doc 4.5 (ritmo de niveles aprobado)", &format!("Mide la economía sin el efecto del sitio: solo las {} plazas que nacieron con al menos una parcela de bosque y un nodo de piedra a 10 u o menos.", con_sitio.len())), 10.0),
        metrica("nivel2_mediana_dias", "Tiempo mediano hasta el nivel 2", "días", max(6.5), mediana_censurada(&n2, MIN_POR_DIA, dias), "Doc 4.5 (Era I: nivel 2 a los 5,8 días de mediana)", "Mediana sobre todas las plazas; las que no llegan cuentan como el tope de la simulación (la mediana real sería mayor)."),
        metrica("ascenso_pedido_mediana_dias", "Día mediano en que se pide la subida al nivel 2", "días", INFO, mediana_censurada(&ped, MIN_POR_DIA, dias), "Doc 4.5", "El jugador sintético mira cada hora; la obra de ascenso añade su duración (ver parámetros) hasta el nivel 2. Las que nunca la piden cuentan como el tope."),
    ]
}

/// 3 y 4: ruinas y hambre.
fn salud(x: &Entrada) -> Vec<Metrica> {
    let v = &x.corrida.villas;
    let en_ruinas: Vec<&Villa> = v.iter().filter(|p| p.reg.ruinas.is_some()).collect();
    let hambre = v.iter().filter(|p| p.reg.muertes > 0).count();
    let muertes: i64 = v.iter().map(|p| p.reg.muertes).sum();
    let sin_lenera = en_ruinas.iter().filter(|p| p.reg.primera_lenera.is_none()).count();
    let mut faltan: BTreeMap<&str, usize> = BTreeMap::new();
    en_ruinas.iter().flat_map(|p| p.reg.ruinas_faltantes.iter()).for_each(|r| *faltan.entry(r.nombre()).or_insert(0) += 1);
    let con_lenera: Vec<f64> = en_ruinas
        .iter()
        .filter(|p| p.reg.primera_lenera.is_some())
        .map(|p| p.parcelas.iter().map(|&i| x.mapa.densidad_ppm[i as usize]).max().unwrap_or(0) as f64 / 1e6)
        .collect();
    let dia = mediana(&en_ruinas.iter().filter_map(|p| p.reg.ruinas).map(|m| m as f64 / MIN_POR_DIA).collect::<Vec<_>>());
    let nota = format!(
        "Plazas que llegan a `Ruinas`: medidor a 0 en el nivel operativo 1 (no pudieron pagar el mantenimiento). De las {} en ruinas: {} nunca terminaron una Leñera; recurso que no pudieron pagar al caer: {}; día mediano de la caída: {}. Las {} que sí tuvieron Leñera tenían una parcela de densidad (la mejor) {} de mediana: con bosque ralo la madera no cubre el mantenimiento.",
        en_ruinas.len(),
        sin_lenera,
        if faltan.is_empty() { "—".to_string() } else { faltan.iter().map(|(r, k)| format!("{r} {k}")).collect::<Vec<_>>().join(", ") },
        dia.map_or("—".to_string(), |d| format!("{d:.1}")),
        con_lenera.len(),
        mediana(&con_lenera).map_or("—".to_string(), |d| format!("{d:.2}"))
    );
    vec![
        metrica("ruinas_pct", "Plazas en ruinas", "%", max(0.0), pct(en_ruinas.len(), v.len()), "Economia_Construccion §10.4 (ruinas)", &nota),
        metrica("hambre_pct", "Plazas con muertes por hambre", "%", max(5.0), pct(hambre, v.len()), "Economia_Construccion §9.2 (hambruna)", "Plazas con al menos un minuto de `Hambruna` (nutrición en el umbral de muerte)."),
        metrica("muertes_totales", "Muertes por hambre (total de habitantes)", "habitantes", INFO, Some(muertes as f64), "Economia_Construccion §9.2", "Suma de pesants, artesanos y nobles muertos por hambre en todas las plazas."),
    ]
}

/// 5 y 7: Leñera, Cantera y edificios.
fn construccion(x: &Entrada) -> Vec<Metrica> {
    let c = x.corrida;
    let v = &c.villas;
    let horas = x.e.dias as f64 * 24.0;
    let len = dias_de(v, |p| p.reg.primera_lenera);
    let can = dias_de(v, |p| p.reg.primera_cantera);
    let sin = |f: &dyn Fn(&Villa) -> bool| pct(v.iter().filter(|p| f(p)).count(), v.len());
    let afectadas = |k: usize| pct(v.iter().filter(|p| p.reg.sin_sitio.sin_ninguno[k] > 0).count(), v.len());
    let totales = |k: usize| v.iter().map(|p| p.reg.sin_sitio.total[k]).sum::<u64>();
    let med = |f: &dyn Fn(&Villa) -> f64| mediana(&v.iter().map(f).collect::<Vec<_>>());
    let tot = |m: &std::collections::BTreeMap<Tipo, u32>| m.values().sum::<u32>() as f64;
    let (dia_a, dias) = (c.dia_foto, x.e.dias);
    let nota_edificios = {
        let mut por_tipo: BTreeMap<Tipo, Vec<f64>> = BTreeMap::new();
        for p in v {
            for &t in p.reg.edificios_fin.keys().chain(p.reg.edificios_a.keys()) {
                por_tipo.entry(t).or_default();
            }
        }
        for (t, lista) in por_tipo.iter_mut() {
            *lista = v.iter().map(|p| *p.reg.edificios_fin.get(t).unwrap_or(&0) as f64).collect();
        }
        let mut orden: Vec<_> = por_tipo.iter().map(|(t, l)| (t.nombre(), mediana(l).unwrap_or(0.0))).collect();
        orden.sort_by(|a, b| b.1.total_cmp(&a.1));
        orden.iter().map(|(n, m)| format!("{n} {m}")).collect::<Vec<_>>().join(", ")
    };
    vec![
        metrica("primera_lenera_horas", "Hora mediana de la primera Leñera terminada", "h", INFO, mediana_censurada(&len, 60.0, horas), "Economia_Construccion §6.2 (auto-construcción)", "Desde la fundación. Las plazas que nunca la terminan cuentan como el tope de la simulación."),
        metrica("primera_cantera_horas", "Hora mediana de la primera Cantera terminada", "h", INFO, mediana_censurada(&can, 60.0, horas), "Economia_Construccion §6.2", "Desde la fundación. Las que nunca la terminan cuentan como el tope."),
        metrica("sin_lenera_pct", "Plazas que nunca terminan una Leñera", "%", max(10.0), sin(&|p| p.reg.primera_lenera.is_none()), "Doc 1.9c / D2 de bosque (sin parcela no hay Leñera)", &format!("Sin ninguna Leñera completada al final. De ellas, {} no tenían ni una parcela de bosque en el disco de 10 u (techo de zona del nivel 1) al fundar: sin bosque estructural; el resto lo perdió contra otras plazas o por la cuota de la masa.", fmt(pct(v.iter().filter(|p| p.reg.primera_lenera.is_none() && p.parcelas_n1 == 0).count(), v.iter().filter(|p| p.reg.primera_lenera.is_none()).count()))),),
        metrica("sin_cantera_pct", "Plazas que nunca terminan una Cantera", "%", max(10.0), sin(&|p| p.reg.primera_cantera.is_none()), "Doc 1.2 (sin piedra no hay Cantera ni nivel 2)", &format!("Sin ninguna Cantera completada al final. De ellas, {} no tenían ni un nodo de piedra en el disco de 10 u al fundar; el resto lo perdió contra otras plazas.", fmt(pct(v.iter().filter(|p| p.reg.primera_cantera.is_none() && p.piedra_n1 == 0).count(), v.iter().filter(|p| p.reg.primera_cantera.is_none()).count()))),),
        metrica("sin_sitio_lenera_pct", "Plazas a las que se negó sitio para su primera Leñera", "%", INFO, afectadas(0), "Doc 1.9c", &format!("Plazas en que `proponer(Lenera)` devolvió `None` mientras no tenían ninguna (sin parcela libre en el disco o cupo de la masa agotado). Total de consultas denegadas (también con alguna ya hecha): {} (se repiten cada minuto mientras la plaza quiere una Leñera).", totales(0))),
        metrica("sin_sitio_cantera_pct", "Plazas a las que se negó sitio para su primera Cantera", "%", INFO, afectadas(1), "Doc 1.2", &format!("Plazas en que `proponer(Cantera)` devolvió `None` mientras no tenían ninguna (sin nodo de piedra libre en el disco). Consultas denegadas en total: {}.", totales(1))),
        metrica("sin_sitio_corral_pct", "Plazas a las que se negó sitio para su primer Corral", "%", INFO, afectadas(2), "Doc 4.2.1 (Corral liga a un nodo de ganado)", &format!("Plazas en que `proponer(Corral)` devolvió `None` mientras no tenían ninguna (sin nodo de ganado libre en el disco). Consultas denegadas en total: {}.", totales(2))),
        metrica("corral_pct", "Plazas que terminan un Corral", "%", INFO, pct(v.iter().filter(|p| p.reg.primera_corral.is_some()).count(), v.len()), "Doc 4.2.1 (el Corral liga a un nodo de ganado)", "Plazas con un Corral completado al final. Sin Corral no hay livestock para Curtiduría ni Caballerizas."),
        metrica("ganado_15_pct", "Plazas con algún nodo de ganado a 15 u o menos", "%", INFO, pct(v.iter().filter(|p| !x.mapa.ganado_en(p.pos, 15_000).is_empty()).count(), v.len()), "Doc 4.2.1 (diagnóstico del Corral)", "Diagnóstico de la decisión P5: 15 u es el techo de la zona en el nivel 2."),
        metrica("ganado_30_pct", "Plazas con algún nodo de ganado a 30 u o menos", "%", INFO, pct(v.iter().filter(|p| !x.mapa.ganado_en(p.pos, 30_000).is_empty()).count(), v.len()), "Doc 4.2.1 (diagnóstico del Corral)", "30 u es el techo de la zona en el nivel 5."),
        metrica("ganado_30_mediana", "Nodos de ganado a 30 u o menos de la plaza (mediana)", "nodos", INFO, mediana(&v.iter().map(|p| x.mapa.ganado_en(p.pos, 30_000).len() as f64).collect::<Vec<_>>()), "Doc 4.2.1 (diagnóstico del Corral)", "Con el ganado ×1 del mundo actual (400 nodos)."),
        metrica("viviendas_fin", &format!("Viviendas por plaza a los {dias} días (mediana)"), "viviendas", INFO, med(&|p| *p.reg.viviendas_dia.last().unwrap_or(&0) as f64), "Economia_Construccion §6.5 (topes por nivel)", "Viviendas activas."),
        metrica("edificios_a", &format!("Edificios activos por plaza a los {dia_a} días (mediana)"), "edificios", INFO, med(&|p| tot(&p.reg.edificios_a)), "Economia_Construccion §6.2", "Todos los edificios activos, Centro Urbano y las 3 Viviendas y la Granja iniciales incluidos."),
        metrica("edificios_fin", &format!("Edificios activos por plaza a los {dias} días (mediana)"), "edificios", INFO, med(&|p| tot(&p.reg.edificios_fin)), "Economia_Construccion §6.2", &format!("Medianas por tipo a los {dias} días: {nota_edificios}.")),
    ]
}

/// 6: pesants.
fn poblacion(x: &Entrada) -> Vec<Metrica> {
    let c = x.corrida;
    let v = &c.villas;
    let en_dia = |d: usize| mediana(&v.iter().map(|p| p.reg.pesants_dia.get(d).copied().unwrap_or(0) as f64).collect::<Vec<_>>());
    let con200 = v.iter().filter(|p| p.reg.pesants_200.is_some()).count();
    let p200 = dias_de(v, |p| p.reg.pesants_200);
    vec![
        metrica("pesants_dia_foto", &format!("Pesants a los {} días (mediana)", c.dia_foto), "habitantes", INFO, en_dia(c.dia_foto as usize), "Economia_Construccion §9.1 (crecimiento logístico)", "Pesants enteros al final del día, mediana de las plazas."),
        metrica("pesants_fin", &format!("Pesants a los {} días (mediana)", x.e.dias), "habitantes", INFO, en_dia(x.e.dias as usize), "Economia_Construccion §9.1", "Pesants al final de la simulación, mediana de las plazas."),
        metrica("artesanos_max", "Artesanos de la plaza que más tiene al final", "habitantes", INFO, v.iter().map(|p| p.a.poblacion.artesanos).max().map(|m| m as f64), "Doc 4.5 (el nivel 3 pide 400 artesanos)", "Artesanos enteros. El cupo de artesanos de 40 Viviendas es justo 400, el mismo número que pide el nivel 3."),
        metrica("nivel3_pct", "Plazas que llegan al nivel 3", "%", INFO, pct(v.iter().filter(|p| p.reg.nivel3.is_some()).count(), v.len()), "Doc 4.5 (ritmo de niveles 2-3 aprobado)", "Este batch no da órdenes manuales ni sube la Facción de nivel 1: Barracón y Galería de tiro solo se añaden a mano y el cupo de nivel 3 de una Facción de nivel 1 es 0. Informativa."),
        metrica("pesants_200_pct", "Plazas que alcanzan 200 pesants", "%", INFO, pct(con200, v.len()), "Doc 4.5 (requisito del nivel 2: 200 pesants)", "Pesants máximos ≥ 200 en algún momento."),
        metrica("pesants_200_mediana_dias", "Día mediano en que se alcanzan los 200 pesants", "días", INFO, mediana_censurada(&p200, MIN_POR_DIA, x.e.dias as f64), "Doc 4.5", "Las que nunca llegan cuentan como el tope."),
    ]
}

/// 8 y 9: trigo y mantenimiento.
fn trigo_y_mantenimiento(x: &Entrada) -> Vec<Metrica> {
    let v = &x.corrida.villas;
    let pasos: u64 = v.iter().map(|p| p.reg.pasos).sum();
    let deficit: u64 = v.iter().map(|p| p.reg.min_deficit_trigo).sum();
    let bajo50 = v.iter().filter(|p| p.reg.medidor_min < 50_000).count();
    let colapsos: u32 = v.iter().map(|p| p.reg.colapsos).sum();
    vec![
        metrica("deficit_trigo_pct", "Tiempo con déficit de trigo", "% de plaza·minuto", max(5.0), (pasos > 0).then(|| 100.0 * deficit as f64 / pasos as f64), "Doc 4.2 (la respuesta a un déficit de trigo es construir otra Granja); umbral propuesto por el batch", "Minutos en que la nutrición baja (el trigo del almacén no cubre el consumo). El umbral del 5 % es una propuesta del batch, no una cifra del canon."),
        metrica("nutricion_min", "Nutrición mínima alcanzada (peor plaza)", "0-100", INFO, v.iter().map(|p| p.reg.nutricion_min).min().map(|m| m as f64 / 1000.0), "Economia_Construccion §9.2 (la gente muere con la nutrición en 0)", "El valor más bajo de la nutrición en cualquier plaza y minuto (100 = sin hambre)."),
        con_margen(metrica("medidor_bajo50_pct", "Plazas cuyo medidor de mantenimiento baja de 50", "%", max(5.0), pct(bajo50, v.len()), "Economia_Construccion §10.2 (medidor)", "Plazas con el medidor < 50 en algún minuto; el medidor baja con el déficit de mantenimiento y se regenera si paga completo."), 5.0),
        metrica("medidor_min", "Medidor de mantenimiento mínimo (peor plaza)", "0-100", INFO, v.iter().map(|p| p.reg.medidor_min).min().map(|m| m as f64 / 1000.0), "Economia_Construccion §10.2", "El valor más bajo del medidor en cualquier plaza y minuto."),
        metrica("degradacion_max", "Mayor caída del nivel operativo bajo el alcanzado", "niveles", INFO, v.iter().map(|p| p.reg.degradacion_max).max().map(f64::from), "Economia_Construccion §10.2 (colapso baja un escalón)", &format!("0 = ninguna plaza perdió nunca un nivel operativo. Colapsos totales de mantenimiento: {colapsos}.")),
    ]
}

/// Diagnóstico de los cuellos de botella del nivel 2.
fn cuellos(x: &Entrada) -> Vec<Metrica> {
    let c = x.corrida;
    let v = &c.villas;
    let dias = x.e.dias as f64;
    let libre = |k: usize| mediana_censurada(&dias_de(v, |p| p.reg.libre_de[k]), MIN_POR_DIA, dias);
    let en_n1 = v.iter().filter(|p| p.a.nivel == 1).count();
    let atascadas = |k: usize| pct(v.iter().filter(|p| p.a.nivel == 1 && p.reg.ultimos_bloqueos.contains(&k)).count(), en_n1);
    let mut metricas = vec![
        metrica("bloqueo_poblacion_dias", "Día mediano en que deja de faltar población para el nivel 2", "días", INFO, libre(2), "Doc 4.5 (200 pesants)", "Primera comprobación horaria sin el bloqueo «falta población» (las que nunca lo superan cuentan como el tope)."),
        metrica("bloqueo_edificios_dias", "Día mediano en que dejan de faltar edificios para el nivel 2", "días", INFO, libre(3), "Doc 4.5 (3 de los 7 tipos de extracción)", "Primera comprobación horaria sin el bloqueo «faltan edificios»."),
        metrica("bloqueo_recursos_dias", "Día mediano en que el almacén cubre el coste del ascenso", "días", INFO, libre(5), "Doc 4.5 (coste del ascenso)", "Primera comprobación horaria sin el bloqueo «recursos insuficientes»."),
        metrica("bloqueo_solvencia_dias", "Día mediano en que la producción cubre el mantenimiento del nivel 2", "días", INFO, libre(6), "Doc 4.5 (solvencia en el nivel nuevo)", "Primera comprobación horaria sin el bloqueo «insolvente»."),
    ];
    metricas.push(metrica("atascadas_nivel1_pct", "Plazas que terminan en el nivel 1", "%", INFO, pct(en_n1, v.len()), "Doc 4.5", &format!("De ellas, a la última comprobación pesaba: falta población {}, faltan edificios {}, recursos insuficientes {}, insolvente {} (porcentaje de las atascadas).", fmt(atascadas(2)), fmt(atascadas(3)), fmt(atascadas(5)), fmt(atascadas(6)))));
    metricas
}

fn fmt(v: Option<f64>) -> String {
    v.map_or("—".into(), |v| format!("{v:.0} %"))
}

fn nombres(tipos: &[Tipo]) -> String {
    tipos.iter().map(|t| t.nombre()).collect::<Vec<_>>().join(", ")
}

/// Esperas (minutos) de las mejoras que pidieron hueco para mudarse: las resueltas y, en las plazas que siguen en pie, las que siguen
/// esperando al terminar la simulación.
fn esperas_de(p: &Villa, fin_min: i64) -> Vec<i64> {
    let mut e = p.reg.ciudad.esperas.clone();
    if !p.a.destruido {
        e.extend(p.reg.ciudad.esperando.values().map(|desde| fin_min - desde));
    }
    e
}

/// 11: la ciudad real de cada plaza (Trazado_Urbano).
fn ciudad(x: &Entrada) -> Vec<Metrica> {
    let c = x.corrida;
    let v = &c.villas;
    let vivas: Vec<&Villa> = v.iter().filter(|p| !p.a.destruido).collect();
    let (dias, dia_a) = (x.e.dias, c.dia_foto);
    let fin_min = dias as i64 * 1440;
    let con_negado = |tipos: &[Tipo]| pct(v.iter().filter(|p| tipos.iter().any(|t| p.reg.ciudad.denegadas.contains_key(t))).count(), v.len());
    let consultas = |tipos: &[Tipo]| v.iter().flat_map(|p| tipos.iter().map(|t| p.reg.ciudad.denegadas.get(t).copied().unwrap_or(0))).sum::<u64>();
    let mut metricas = Vec::new();

    // Sin sitio en la ciudad: los cuatro tipos que no deben quedarse sin él (ancla del Centro Urbano o radio urbano libre), y los demás.
    let esperados: [(Tipo, &str, &str, &str); 4] = [
        (Tipo::Vivienda, "Viviendas", "Trazado_Urbano §6.5 (satélites del Centro Urbano, que es su ancla)", "Una Vivienda sin sitio frena la población y con ella el nivel 2."),
        (Tipo::Almacen, "Almacenes", "Trazado_Urbano §6.4 (genéricos: todo el radio urbano)", "Un Almacén sin sitio deja el almacén lleno y frena la construcción."),
        (Tipo::Granero, "Graneros", "Trazado_Urbano §6.4 (genéricos: todo el radio urbano)", "Un Granero sin sitio limita el trigo guardado."),
        (Tipo::Lenera, "Leñeras", "Trazado_Urbano §6.4 (genéricos: todo el radio urbano)", "Una Leñera sin sitio en la ciudad (teniendo ya parcela de bosque en el mapa) deja a la plaza sin madera."),
    ];
    for (tipo, plural, origen, efecto) in esperados {
        let id = format!("ciudad_sin_sitio_{}_pct", tipo.nombre().to_lowercase());
        metricas.push(metrica(
            &id,
            &format!("Plazas a las que la ciudad negó sitio para {plural}"),
            "%",
            max(5.0),
            con_negado(&[tipo]),
            origen,
            &format!("Plazas en que `proponer({})` devolvió `None` porque no cabía en la ciudad (el mapa sí daba sitio). {efecto} Consultas denegadas en total: {} (la auto-construcción repite la pregunta cada minuto).", tipo.nombre(), consultas(&[tipo])),
        ));
    }
    let talleres = [Tipo::Fundicion, Tipo::Curtiduria, Tipo::Armeria, Tipo::Carpinteria];
    metricas.push(metrica("ciudad_sin_sitio_granja_pct", "Plazas a las que la ciudad negó sitio para una Granja", "%", INFO, con_negado(&[Tipo::Granja]), "Trazado_Urbano §6.3 (afueras: banda de 60 a 96 u.l., caminos)", &format!("Consultas denegadas: {}. Las Granjas van a las afueras, fuera del radio de la ciudad.", consultas(&[Tipo::Granja]))));
    metricas.push(metrica("ciudad_sin_sitio_corral_pct", "Plazas a las que la ciudad negó sitio para un Corral", "%", INFO, con_negado(&[Tipo::Corral]), "Trazado_Urbano §6.3 (afueras)", &format!("Consultas denegadas: {}. El Corral exige además un nodo de ganado en la zona (ver «sin sitio» del mapa).", consultas(&[Tipo::Corral]))));
    metricas.push(metrica("ciudad_sin_sitio_talleres_pct", "Plazas a las que la ciudad negó sitio para un taller de transformación", "%", INFO, con_negado(&talleres), "Trazado_Urbano §7.1 (ancla de saturación con el gate de nivel 2)", &format!("Algún tipo de {} sin sitio. Consultas denegadas: {}. Sin ancla de su barrio un taller solo cabe si el nivel operativo abre el gate de las anclas (nivel 2).", nombres(&talleres), consultas(&talleres))));
    let por_tipo: BTreeMap<Tipo, usize> = TIPOS.iter().map(|&t| (t, v.iter().filter(|p| p.reg.ciudad.denegadas.contains_key(&t)).count())).filter(|&(_, n)| n > 0).collect();
    let negados: u64 = v.iter().map(|p| p.reg.ciudad.tomar_negado.values().sum::<u64>()).sum();
    metricas.push(metrica(
        "ciudad_sin_sitio_alguno_pct",
        "Plazas a las que la ciudad negó sitio para algún edificio",
        "%",
        INFO,
        pct(v.iter().filter(|p| !p.reg.ciudad.denegadas.is_empty()).count(), v.len()),
        "Trazado_Urbano §6 (colocación)",
        &format!(
            "Cualquier tipo. Plazas por tipo: {}. `tomar` negados tras una propuesta que sí cabía: {negados}.",
            if por_tipo.is_empty() { "ninguno".to_string() } else { por_tipo.iter().map(|(t, n)| format!("{} {n}", t.nombre())).collect::<Vec<_>>().join(", ") }
        ),
    ));

    // Piezas y anclas.
    let med = |f: &dyn Fn(&Villa) -> f64| mediana(&v.iter().map(f).collect::<Vec<_>>());
    metricas.push(metrica("piezas_a", &format!("Piezas en la ciudad a los {dia_a} días (mediana)"), "piezas", INFO, med(&|p| p.reg.piezas_a as f64), "Trazado_Urbano §2 (modelo de datos)", "Edificios con huella en la rejilla: los de la economía menos Cantera y minas (que solo ocupan su yacimiento) más las anclas y los puestos del Mercado."));
    metricas.push(metrica("piezas_fin", &format!("Piezas en la ciudad a los {dias} días (mediana)"), "piezas", INFO, med(&|p| p.reg.piezas_fin as f64), "Trazado_Urbano §2 (modelo de datos)", "Lo mismo al final de la simulación."));
    let n_anclas = |p: &Villa, t: Tipo| p.urbe.ciudad().piezas().iter().filter(|q| TIPOS[q.tipo as usize] == t).count() as f64;
    let es_ancla = |t: Tipo| x.urb.clase(t).is_some_and(|k| k.rol == Rol::AnclaSaturacion);
    let anclas: Vec<Tipo> = TIPOS.iter().copied().filter(|&t| es_ancla(t)).collect();
    for &t in &anclas {
        metricas.push(metrica(
            &format!("ancla_{}_mediana", t.nombre().to_lowercase()),
            &format!("Anclas de {} por plaza (mediana)", t.nombre()),
            "anclas",
            INFO,
            med(&|p| n_anclas(p, t)),
            "Trazado_Urbano §7.1 (anclas de saturación)",
            &format!("Plazas con al menos una: {}.", fmt(pct(v.iter().filter(|p| n_anclas(p, t) > 0.0).count(), v.len()))),
        ));
    }
    metricas.push(metrica("anclas_plazas_pct", "Plazas en que el trazado creó alguna ancla de saturación", "%", INFO, pct(v.iter().filter(|p| p.reg.primera_ancla.is_some()).count(), v.len()), "Trazado_Urbano §7.1", &format!("Tipos de ancla: {}. Son edificios gratuitos y activos (cuentan a la mitad en el mantenimiento y no agrandan la ciudad).", nombres(&anclas))));
    metricas.push(metrica(
        "ancla_primera_dia",
        "Día mediano de la primera ancla (entre las plazas que la tienen)",
        "días",
        INFO,
        mediana(&v.iter().filter_map(|p| p.reg.primera_ancla).map(|m| m as f64 / MIN_POR_DIA).collect::<Vec<_>>()).or(Some(0.0)),
        "Trazado_Urbano §7.1 (gate de nivel 2 para crear anclas)",
        "0 si ninguna plaza llegó a crear una. Sin ancla antes del nivel 2 salvo la del Centro Urbano (el barrio residencial y los genéricos no la necesitan); se crea al comprometer el primer edificio de un barrio que la pide.",
    ));

    // Mejoras de Granja que esperan hueco.
    let esperas: Vec<Vec<i64>> = v.iter().map(|p| esperas_de(p, fin_min)).collect();
    let peor: Vec<f64> = esperas.iter().filter_map(|e| e.iter().max()).map(|&m| m as f64 / 60.0).collect();
    let retenidas = esperas.iter().filter(|e| e.iter().any(|&m| m > 24 * 60)).count();
    let mejoradas = v.iter().filter(|p| p.a.edificios.iter().any(|e| e.tipo == Tipo::Granja && e.nivel_interno >= 2)).count();
    metricas.push(metrica("granja_mejorada_pct", "Plazas cuya Granja llegó al nivel interno 2 o más", "%", INFO, pct(mejoradas, v.len()), "Trazado_Urbano §6.7 (reubicación de un edificio que crece)", "La Granja es el único edificio que cambia de huella al mejorar (2×2, 2×3, 4×3, 6×6): al terminar la mejora se muda a un hueco de su tamaño nuevo."));
    metricas.push(metrica("mejora_espera_pct", "Plazas en que una mejora esperó hueco para mudarse", "%", INFO, pct(peor.len(), v.len()), "Trazado_Urbano §6.7", "Mejoras terminadas cuya Granja no halló hueco de la huella nueva: la mejora se queda esperando y conserva su cuadrilla. Cuenta las resueltas y las que siguen esperando al final. Desde el 2026-10-23 la mejora automática comprueba el hueco al arrancar, como la manual (decisión 2 de las DUDA), así que solo puede esperar si la ciudad cambió durante la obra."));
    metricas.push(metrica("mejora_espera_mediana_h", "Espera mediana de la peor mejora sin hueco (plazas que esperaron)", "h", INFO, mediana(&peor).or(Some(0.0)), "Trazado_Urbano §6.7", if peor.is_empty() { "Ninguna mejora esperó." } else { "Por plaza se toma su espera más larga; 0 si ninguna esperó." }));
    metricas.push(metrica("mejora_espera_max_h", "Espera máxima de una mejora sin hueco", "h", INFO, peor.iter().copied().max_by(f64::total_cmp).or(Some(0.0)), "Trazado_Urbano §6.7", "La mayor espera de cualquier plaza (la que siga esperando al final cuenta hasta el final de la simulación)."));
    metricas.push(metrica(
        "cuadrilla_retenida_24h",
        "Plazas con una cuadrilla retenida más de 24 h por una mejora sin hueco",
        "plazas",
        max(0.0),
        Some(retenidas as f64),
        "Economia_Construccion §6.4 (cuadrillas) y Trazado_Urbano §6.7",
        "Riesgo de bloqueo: una mejora terminada sin hueco retiene su cuadrilla mientras no haya donde mudar la Granja. Desde el 2026-10-23 la mejora automática solo arranca si hay hueco (decisión 2 de las DUDA): un valor mayor que 0 es un bloqueo real.",
    ));

    // Tamaño de la ciudad.
    let radios: Vec<(i32, i32)> = vivas
        .iter()
        .map(|p| {
            let prm = p.urbe.ciudad().params();
            (p.urbe.radio_ul(&p.a.urbano(x.reglas)), prm.maximo_por_nivel_ul[(p.a.nivel.clamp(1, 5) - 1) as usize])
        })
        .collect();
    let media = (!radios.is_empty()).then(|| radios.iter().map(|r| r.0 as f64).sum::<f64>() / radios.len() as f64);
    metricas.push(metrica("radio_ciudad_medio", &format!("Radio de la ciudad a los {dias} días (media)"), "u.l.", INFO, media, "Trazado_Urbano §2.1 (radioCiudad)", "Unidades locales (no del mapa): 30 al fundar, +5 por edificio completado, con el tope del nivel alcanzado (60 / 90 / 120 / 150 / 180). Plazas en pie."));
    let lejos: Vec<f64> = vivas
        .iter()
        .filter_map(|p| p.urbe.ciudad().piezas().iter().filter(|q| q.categoria != trazado::Categoria::Afueras).map(|q| q.rect.dist2_centro_al_origen()).max())
        .map(|d2| (d2 as f64).sqrt() / 1000.0)
        .collect();
    metricas.push(metrica("pieza_urbana_lejana_mediana", &format!("Distancia de la pieza urbana más lejana a los {dias} días (mediana)"), "u.l.", INFO, mediana(&lejos), "Trazado_Urbano §6.4-§6.5 (colocación dentro del radio urbano)", "Centro de la pieza más alejada del Centro Urbano sin contar las afueras (Granjas y Corrales). Si es mucho menor que el radio de la ciudad, el espacio no limita: la ciudad no está presionada por su disco."));
    metricas.push(metrica("radio_tope_pct", "Plazas cuyo radio de ciudad llega al máximo de su nivel", "%", INFO, pct(radios.iter().filter(|r| r.0 >= r.1).count(), radios.len()), "Trazado_Urbano §2.1 (radioCiudad)", "Plazas en pie con el radio en el tope de su nivel: ahí la ciudad solo crece subiendo de nivel."));

    // Mercado.
    let con_mercado = v.iter().filter(|p| p.reg.edificios_fin.get(&Tipo::Mercado).is_some_and(|&n| n > 0)).count();
    let puestos: u32 = v.iter().map(|p| p.reg.edificios_fin.get(&Tipo::PuestoMercado).copied().unwrap_or(0)).sum();
    metricas.push(metrica("mercado_plazas_pct", "Plazas con un Mercado terminado", "%", INFO, pct(con_mercado, v.len()), "Economia_Construccion §6.6 (puestos del Mercado)", if con_mercado == 0 { "Ninguna: el Mercado no se auto-construye y este batch no da órdenes manuales; los puestos se prueban en `nucleo/tests/urbanismo.rs`." } else { "Plazas con un Mercado activo al final." }));
    metricas.push(metrica("puestos_mercado_total", "Puestos del Mercado levantados (total)", "puestos", INFO, Some(puestos as f64), "Economia_Construccion §6.6 y Trazado_Urbano §2.4 (MERCADO_PUESTOS_POR_NIVEL)", if con_mercado == 0 { "Ninguno: sin Mercados no hay puestos." } else { "Puestos gratuitos activos colocados en la ciudad; los que no caben se saltan en silencio." }));
    metricas
}

/// 12: la muralla (paso 7 del plan de Murallas): la empalizada que compromete el jugador en el nivel 2.
fn murallas(x: &Entrada) -> Vec<Metrica> {
    let v = &x.corrida.villas;
    let dias = x.e.dias as f64;
    let pueden: Vec<&Villa> = v.iter().filter(|p| p.reg.nivel2.is_some()).collect();
    let comp: Vec<&Villa> = v.iter().filter(|p| p.reg.recinto_comprometido.is_some()).collect();
    let cerradas = comp.iter().filter(|p| p.reg.recinto_completado.is_some()).count();
    let med = |l: &[&Villa], f: &dyn Fn(&Villa) -> f64| mediana(&l.iter().map(|p| f(p)).collect::<Vec<_>>());
    let coste = |p: &Villa, r: asentamiento::Recurso| p.reg.recinto_coste.iter().filter(|(k, _)| *k == r).map(|&(_, n)| n).sum::<i64>() as f64 / 1000.0;
    // El peso de la muralla en el mantenimiento al final: su upkeep sobre el total (todos los recursos), en las plazas en pie con algún recinto.
    let suma = |c: &asentamiento::Costo| c.iter().map(|&(_, n)| n).sum::<i64>() as f64;
    let pesos: Vec<f64> = comp
        .iter()
        .filter(|p| !p.a.destruido)
        .filter_map(|p| {
            let (muro, total) = (suma(&asentamiento::recintos::upkeep(&x.reglas.muralla, &p.a.recintos)), suma(&p.a.coste_mantenimiento(x.reglas, 0)));
            (total > 0.0).then(|| 100.0 * muro / total)
        })
        .collect();
    let mut errores: BTreeMap<String, usize> = BTreeMap::new();
    pueden.iter().filter(|p| p.reg.recinto_comprometido.is_none()).for_each(|p| *errores.entry(p.reg.recinto_error.map_or("sin intento".into(), |e| format!("{e:?}"))).or_insert(0) += 1);
    let n2_con: Vec<Option<i64>> = comp.iter().map(|p| p.reg.nivel2).collect();
    let fin = x.e.dias as i64 * 1440;
    let obra: Vec<Option<i64>> = comp.iter().map(|p| Some(p.reg.recinto_completado.unwrap_or(fin) - p.reg.recinto_comprometido.unwrap_or(0))).collect();
    let fmt1 = |v: Option<f64>| v.map_or("—".to_string(), |d| format!("{d:.2}"));
    // La ciudad que deja de crecer tras amurallarse: sin las anclas, la ciudad no niega sitio a una Vivienda en este batch (0 sin murallas), así que una
    // negativa en una plaza con recinto es la muralla.
    let viviendas = |p: &Villa| p.a.edificios.iter().filter(|e| e.activo() && e.tipo == Tipo::Vivienda).count() as f64;
    let (bloq, libres): (Vec<&Villa>, Vec<&Villa>) = comp.iter().copied().filter(|p| !p.a.destruido).partition(|p| p.reg.ciudad.denegadas.contains_key(&Tipo::Vivienda));
    let fmt0 = |v: Option<f64>| v.map_or("—".to_string(), |d| format!("{d:.0}"));
    let mut metricas = vec![
        metrica("muralla_comprometida_pct", "Plazas de nivel 2 que comprometen la empalizada", "%", INFO, pct(comp.len(), pueden.len()), "Murallas §4 (empalizada con el nivel operativo 2)", &format!("Entre las que llegan al nivel 2. Las que no la comprometen, por motivo: {}.", if errores.is_empty() { "ninguna".to_string() } else { errores.iter().map(|(k, n)| format!("{k} {n}")).collect::<Vec<_>>().join(", ") })),
        metrica("muralla_cerrada_pct", "Plazas con la empalizada cerrada al final", "%", INFO, pct(cerradas, comp.len()), "Murallas §5 (una celda tras otra, pagada al levantarla)", "Entre las que la comprometieron."),
        metrica("muralla_compromiso_dia", "Día mediano en que se compromete la empalizada", "días", INFO, med(&comp, &|p| p.reg.recinto_comprometido.unwrap_or(0) as f64 / MIN_POR_DIA), "Murallas §4", &format!("Entre las que la comprometen; su nivel 2 llegó el día {} de mediana.", fmt1(mediana_censurada(&n2_con, MIN_POR_DIA, dias)))),
        metrica("muralla_obra_dias", "Días medianos de obra de la empalizada (del compromiso al cierre)", "días", INFO, mediana_censurada(&obra, MIN_POR_DIA, dias), "Murallas §5 y bloque MURALLA (minutos por celda)", "Las que no la cierran cuentan hasta el final de la simulación."),
        metrica("muralla_celdas_mediana", "Celdas del anillo (mediana)", "celdas", INFO, med(&comp, &|p| p.reg.recinto_celdas as f64), "Murallas §3 (trazo)", &format!("Puertas (mediana): {}.", fmt1(med(&comp, &|p| p.reg.recinto_puertas as f64)))),
        metrica("muralla_coste_madera_mediana", "Madera que cuesta levantar la empalizada entera (mediana)", "madera", INFO, med(&comp, &|p| coste(p, asentamiento::Recurso::Madera)), "Murallas §5 y bloque MURALLA (tarifa por celda; puertas ×4)", &format!("Piedra (mediana): {}. Se paga celda a celda y nunca por debajo de la reserva.", fmt1(med(&comp, &|p| coste(p, asentamiento::Recurso::Piedra))))),
        metrica("muralla_ciudad_bloqueada_pct", "Plazas amuralladas a las que la ciudad niega sitio para Viviendas", "%", max(0.0), pct(bloq.len(), bloq.len() + libres.len()), "Trazado_Urbano §6.5-§6.6 y Murallas §3 (el anillo abraza la ciudad del momento)", &format!("Plazas en pie con recinto. En ellas, Viviendas al final (mediana) {} y pesants {}, frente a {} y {} en las demás amuralladas; también se les niegan Armería y Carpintería (ver «sin sitio» de la ciudad). Sin murallas ninguna plaza recibe esta negativa. Dibujo de una en `ciudad_bloqueada.svg`.", fmt0(med(&bloq, &viviendas)), fmt0(med(&bloq, &|p| p.a.poblacion.pesants as f64)), fmt0(med(&libres, &viviendas)), fmt0(med(&libres, &|p| p.a.poblacion.pesants as f64)))),
        metrica("muralla_piezas_sin_salida", "Piezas sin salida a la red en plazas amuralladas", "piezas", max(0.0), Some(x.corrida.ciudades.sin_salida_muralla as f64), "Trazado_Urbano §5.1 y §13 (salida a la red dentro del cap)", "Al levantar el muro la red se recalcula con muros y torres ocupados y una sola puerta por cruce (Murallas §3): una pieza anterior cuyo corredor cruzaba el anillo, o salía de una calle que el muro quitó, puede no hallar otro dentro del cap (Granjas de las afueras, sobre todo). Fuera de `invariantes_ciudad`: la red sigue conexa y el código hace lo que dicen las reglas (también el TypeScript), pero la pieza queda sin calle."),
        metrica("muralla_upkeep_pct", "Peso de la muralla en el mantenimiento al final (mediana)", "%", INFO, mediana(&pesos), "Murallas §8 (upkeep por celda levantada) y Economia_Construccion §10", "Upkeep de los recintos sobre el coste total del mantenimiento por minuto (todos los recursos sumados), en las plazas en pie con recinto."),
    ];
    // Sin ninguna plaza amurallada (escenarios cortos o `--murallas 0`) todas valen 0, como las demás métricas «entre las que…».
    metricas.iter_mut().filter(|m| m.medido.is_none()).for_each(|m| m.medido = Some(0.0));
    metricas
}

/// 10: rendimiento.
fn rendimiento(x: &Entrada) -> Vec<Metrica> {
    let c = x.corrida;
    let us = (c.pasos > 0).then(|| c.pared.as_secs_f64() * 1e6 / c.pasos as f64);
    vec![
        metrica("pared_total", "Tiempo de pared total del escenario", "s", max(120.0), Some(x.pared_total.as_secs_f64()), "Fase 6 (rendimiento)", "Colocación de las plazas, simulación y comprobación de determinismo."),
        metrica("pasos", "Pasos simulados (plaza·minuto)", "pasos", INFO, Some(c.pasos as f64), "Fase 6 (rendimiento)", "Llamadas a `avanzar_minuto` (las plazas en ruinas dejan de contar)."),
        metrica("us_por_paso", "Tiempo por paso", "µs", INFO, us, "Fase 6 (rendimiento)", "Solo el bucle de simulación, sin colocar las plazas ni repetir para el determinismo."),
        metrica("agotamientos", "Veces que se agotó un nodo", "agotamientos", INFO, Some(c.recursos.agotamientos as f64), "Economia_Construccion §5.2 (regeneración)", "Nodos de piedra o ganado que llegaron a 0 y esperaron su regeneración."),
        metrica("sin_colocar", "Plazas que no encontraron punto fundable", "plazas", INFO, Some(c.sin_colocar as f64), "Doc 1.2", "Plazas pedidas que agotaron las rondas de candidatos sin hallar ninguno que pasara `validar`."),
    ]
}

// ───────────────────────────── Gráficos y mapa ─────────────────────────────

fn grafico_pesants(x: &Entrada) -> Grafico {
    let v = &x.corrida.villas;
    let por_dia = |p: f64| -> Vec<(f64, f64)> {
        (0..=x.e.dias as usize).filter_map(|d| percentil(&mut v.iter().map(|q| q.reg.pesants_dia.get(d).copied().unwrap_or(0) as f64).collect::<Vec<_>>(), p).map(|m| (d as f64, crate::armar::r3(m)))).collect()
    };
    Grafico {
        id: "pesants_por_dia".into(),
        titulo: "Pesants por plaza".into(),
        tipo: TipoGrafico::Linea,
        eje_x: "día de mundo".into(),
        eje_y: "pesants".into(),
        series: vec![serie("Percentil 10", por_dia(0.1)), serie("Mediana", por_dia(0.5)), serie("Percentil 90", por_dia(0.9))],
    }
}

fn grafico_nivel2(x: &Entrada) -> Grafico {
    let v = &x.corrida.villas;
    let puntos = (0..=x.e.dias).map(|d| (d as f64, crate::armar::r3(pct(v.iter().filter(|p| p.reg.nivel2.is_some_and(|m| m <= d as i64 * 1440)).count(), v.len()).unwrap_or(0.0)))).collect();
    Grafico { id: "nivel2_acumulado".into(), titulo: "Plazas en el nivel 2 (acumulado)".into(), tipo: TipoGrafico::Linea, eje_x: "día de mundo".into(), eje_y: "% de plazas".into(), series: vec![serie("Nivel 2", puntos)] }
}

fn histograma_nivel2(x: &Entrada) -> Grafico {
    let mut cubos: BTreeMap<i64, usize> = BTreeMap::new();
    x.corrida.villas.iter().filter_map(|p| p.reg.nivel2).for_each(|m| *cubos.entry(m / 1440).or_insert(0) += 1);
    let puntos = cubos.into_iter().map(|(d, k)| (d as f64, k as f64)).collect();
    Grafico { id: "dia_nivel2".into(), titulo: "Día en que cada plaza sube al nivel 2".into(), tipo: TipoGrafico::Histograma, eje_x: "día de mundo".into(), eje_y: "plazas".into(), series: vec![serie("Plazas", puntos)] }
}

fn grafico_edificios(x: &Entrada) -> Grafico {
    let c = x.corrida;
    let v = &c.villas;
    let mut tipos: Vec<(Tipo, f64, f64)> = asentamiento::catalogo::TIPOS
        .iter()
        .map(|&t| {
            let m = |f: &dyn Fn(&Villa) -> &BTreeMap<Tipo, u32>| mediana(&v.iter().map(|p| *f(p).get(&t).unwrap_or(&0) as f64).collect::<Vec<_>>()).unwrap_or(0.0);
            (t, m(&|p| &p.reg.edificios_a), m(&|p| &p.reg.edificios_fin))
        })
        .filter(|&(_, a, b)| a > 0.0 || b > 0.0)
        .collect();
    tipos.sort_by(|a, b| b.2.total_cmp(&a.2).then(b.1.total_cmp(&a.1)));
    let series = tipos.iter().take(12).map(|&(t, a, b)| serie(t.nombre(), vec![(c.dia_foto as f64, a), (x.e.dias as f64, b)])).collect();
    Grafico { id: "edificios_por_tipo".into(), titulo: "Edificios activos por tipo (mediana por plaza)".into(), tipo: TipoGrafico::Barras, eje_x: "día de mundo".into(), eje_y: "edificios".into(), series }
}

fn grafico_sin_sitio(x: &Entrada) -> Grafico {
    let v = &x.corrida.villas;
    let k = |i: usize| vec![(0.0, v.iter().filter(|p| p.reg.sin_sitio.sin_ninguno[i] > 0).count() as f64)];
    Grafico {
        id: "sin_sitio".into(),
        titulo: "Plazas sin sitio para su primer extractor ligado a un yacimiento".into(),
        tipo: TipoGrafico::Barras,
        eje_x: "extractor ligado a un yacimiento".into(),
        eje_y: "plazas".into(),
        series: vec![serie("Leñera sin parcela", k(0)), serie("Cantera sin piedra", k(1)), serie("Corral sin ganado", k(2))],
    }
}

fn grafico_sin_sitio_ciudad(x: &Entrada) -> Grafico {
    let v = &x.corrida.villas;
    let k = |tipos: &[Tipo]| vec![(0.0, v.iter().filter(|p| tipos.iter().any(|t| p.reg.ciudad.denegadas.contains_key(t))).count() as f64)];
    Grafico {
        id: "sin_sitio_ciudad".into(),
        titulo: "Plazas a las que la ciudad negó sitio, por tipo de edificio".into(),
        tipo: TipoGrafico::Barras,
        eje_x: "tipo de edificio".into(),
        eje_y: "plazas".into(),
        series: vec![
            serie("Vivienda", k(&[Tipo::Vivienda])),
            serie("Almacén", k(&[Tipo::Almacen])),
            serie("Granero", k(&[Tipo::Granero])),
            serie("Leñera", k(&[Tipo::Lenera])),
            serie("Granja", k(&[Tipo::Granja])),
            serie("Talleres", k(&[Tipo::Fundicion, Tipo::Curtiduria, Tipo::Armeria, Tipo::Carpinteria])),
        ],
    }
}

fn mapa(x: &Entrada) -> MapaInforme {
    let u = |v: i64| v as f64 / 1000.0;
    let capa = |nombre: &str, color: &str, circulos, puntos| Capa { nombre: nombre.into(), color: color.into(), circulos, puntos, lineas: vec![] };
    let niveles: [(&str, &str, std::ops::RangeInclusive<u32>); 3] = [("Plazas de nivel 1", "#e11d48", 1..=1), ("Plazas de nivel 2", "#2563eb", 2..=2), ("Plazas de nivel 3 o más", "#a21caf", 3..=u32::MAX)];
    let mut capas: Vec<Capa> = niveles
        .iter()
        .map(|(nombre, color, rango)| {
            let ps: Vec<&Villa> = x.corrida.villas.iter().filter(|p| rango.contains(&p.a.nivel)).collect();
            capa(nombre, color, ps.iter().map(|p| Circulo { x: u(p.pos.x), y: u(p.pos.y), r: u(p.radio) }).collect(), ps.iter().map(|p| Punto { x: u(p.pos.x), y: u(p.pos.y) }).collect())
        })
        .collect();
    capas.push(capa("Campamentos de mercenarios", "#16a34a", vec![], x.mapa.campamentos.iter().map(|p| Punto { x: u(p.x), y: u(p.y) }).collect()));
    for (nombre, color) in [("Piedra (píxel)", fondo::PIEDRA), ("Parcela de bosque (píxel)", fondo::PARCELA), ("Bosque", fondo::BOSQUE), ("Río", fondo::RIO), ("Agua", fondo::AGUA), ("Llano", fondo::LLANO), ("Colina", fondo::COLINA), ("Montaña", fondo::MONTANA)] {
        capas.push(capa(nombre, &hex(color), vec![], vec![]));
    }
    MapaInforme { ancho: u(x.mapa.ancho), alto: u(x.mapa.alto), fondo: fondo::fondo(&x.c.mundo), capas }
}

/// Anomalías automáticas: solo violaciones de invariante.
fn anomalias(metricas: &[Metrica]) -> Vec<Anomalia> {
    ["invariantes", "invariantes_ciudad", "determinismo"]
        .iter()
        .filter_map(|id| metricas.iter().find(|m| m.id == *id))
        .filter(|m| !matches!(informe::semaforo(m), informe::Semaforo::Verde | informe::Semaforo::Informativa | informe::Semaforo::SinDato))
        .map(|m| Anomalia {
            titulo: format!("Invariante incumplido: {}", m.nombre),
            detalle: format!("Medido {} {} (esperado entre {:?} y {:?}). {}", m.medido.unwrap_or(f64::NAN), m.unidad, m.esperado_min, m.esperado_max, m.nota.clone().unwrap_or_default()),
            causa: Some(Causa::Codigo),
        })
        .collect()
}
