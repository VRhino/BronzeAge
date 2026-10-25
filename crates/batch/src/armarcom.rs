//! Monta el `Informe` del sistema «economía y comercio» con lo medido en la simulación de plazas que comercian sobre el núcleo.

use crate::armar::{hex, hoy, max, metrica, min, parametro, r3, serie, INFO};
use crate::armarpob::{mediana, pct, percentil};
use crate::fondo;
use crate::geo::{Cargado, Geo};
use crate::simcom::{self as sc, Corrida, ESTADOS, LLENADO};
use crate::vigia::Viaje;
use crate::EscenarioComercio;
use asentamiento::catalogo::Tipo;
use asentamiento::Recurso;
use comercio::caravana::EstadoCaravana;
use informe::{Anomalia, Capa, Causa, Circulo, Grafico, Informe, Mapa, Metrica, Punto, TipoGrafico};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

const MIN: f64 = 60_000.0;
const DIA: f64 = 1_440.0 * MIN;
/// 1 unidad = 1.000 milésimas.
const U: f64 = 1_000.0;

/// Todo lo medido, listo para montar.
pub struct Entrada<'a> {
    pub c: &'a Cargado,
    pub geo: &'a Geo,
    pub e: &'a EscenarioComercio,
    pub corrida: &'a Corrida,
    /// La misma corrida sin comercio (solo funda y asciende), para medir lo que cuesta.
    pub control: Option<&'a Corrida>,
    /// (misma semilla ⇒ misma huella, reloj a pasos ⇒ misma huella que el acelerado).
    pub determinismo: Option<(bool, bool)>,
    pub pared_total: Duration,
}

pub fn armar(x: &Entrada) -> Informe {
    let mut metricas = Vec::new();
    metricas.extend(arranque(x));
    metricas.extend(trueques(x));
    metricas.extend(caravanas(x));
    metricas.extend(peajes(x));
    metricas.extend(reputacion(x));
    metricas.extend(red(x));
    metricas.extend(mercenarios(x));
    metricas.extend(pueblos(x));
    metricas.extend(invariantes(x));
    metricas.extend(rendimiento(x));
    metricas.extend(determinismo(x));
    let anomalias = anomalias(&metricas);
    let c = x.corrida;
    let resumen = format!(
        "Se simularon {} días de mundo con {} plazas colocadas de {} Facciones ({} pedidas) sobre el núcleo real: trueques, flota que viaja por el mapa, peajes, red de caminos, mercenarios y pueblos extractores con su acarreo. Resumen automático; la lectura la añade quien revise los resultados.",
        x.e.dias,
        c.plazas.len(),
        c.facciones,
        x.e.plazas
    );
    Informe {
        sistema: "Economía y comercio".into(),
        fecha: hoy(),
        mundo_hash: x.c.mundo.hash().into(),
        semilla: x.e.semilla,
        dias_simulados: x.e.dias as f64,
        resumen,
        parametros: parametros(x),
        metricas,
        series: vec![grafico_preparacion(x), grafico_trueques(x), grafico_duracion_trueques(x), grafico_viajes(x), grafico_reputacion(x), grafico_red(x), grafico_peajes(x), grafico_compras(x), grafico_pueblos(x), grafico_llenado(x), grafico_flota(x)],
        mapa: Some(mapa(x)),
        anomalias,
        decisiones: vec![],
    }
}

fn parametros(x: &Entrada) -> Vec<informe::Parametro> {
    let (e, c) = (x.corrida.nucleo.estado(), x.corrida);
    let r = &e.reglas.economia.comercio;
    let mercado = e.reglas.economia.datos.catalogo.tipo(Tipo::Mercado).nivel(1).expect("el Mercado tiene nivel 1");
    vec![
        parametro("Plazas pedidas / colocadas / Facciones", format!("{} / {} / {}", x.e.plazas, c.plazas.len(), c.facciones)),
        parametro("Días de mundo", x.e.dias),
        parametro("Huella del estado final (hash_estado)", sc::huella(c).iter().map(|b| format!("{b:02x}")).collect::<String>()),
        parametro("Semilla", x.e.semilla),
        parametro("Hash del mundo", x.c.mundo.hash()),
        parametro("Escenario", format!("{} plazas por Facción fundadas en los primeros minutos (cada una con su columna parada en la puerta, que funda con una Caravana de Fundación): la primera junto a un campamento, las demás junto a las que ya tiene; Facciones de nivel {} por administración (cap de fundación 6, cupo de 3 plazas de nivel 2)", sc::PLAZAS_POR_FACCION, sc::NIVEL_FACCION)),
        parametro("Economía de las plazas", "la real del núcleo (una por plaza, con su ciudad): producción, construcción automática, mantenimiento, población y ascenso, igual que en `batch territorio`; el «jugador» pide el ascenso cada hora"),
        parametro("Jugador mínimo: Mercado", format!("construye el Mercado (100 de madera) en cuanto le sobran {} de madera por encima de su coste", sc::RESERVA_MADERA / 1000)),
        parametro("Jugador mínimo: flota", format!("una caravana por plaza con {} carros básicos con buey (12 de oro cada buey, 20 de madera cada carro)", sc::CARROS_POR_CARAVANA)),
        parametro("Jugador mínimo: trueques", format!("cada hora, con probabilidad del {} %, si la flota está libre, propone un trueque de {} unidades como mucho a la vecina más cercana (de otra Facción el {} % de las veces) con la que haya doble coincidencia: un recurso que le sobra (≥ {} % del almacén) a quien le falta (≤ {} %) y otro que le falta a quien le sobra; los recursos básicos y el oro, siempre llenos, no entran; la otra parte contesta a los {} minutos y rechaza lo que ya no puede cumplir", sc::PROPONE_PCT, sc::OFERTA_MAX / 1000, sc::OTRA_FACCION_PCT, sc::SOBRA_PCT, sc::FALTA_PCT, sc::RESPUESTA_MIN)),
        parametro("Jugador mínimo: pueblos", format!("de nivel 2 en adelante marca una Cantera (hasta {} por plaza) y manda a su columna al campamento de mercenarios más cercano a comprar los materiales (con el oro del carro) y a entregarlos en la obra; si el campamento no los tiene vuelve a intentarlo {} h después", sc::PUEBLOS_OBJETIVO, sc::REINTENTO_OBRA_H)),
        parametro("Sondeos de columna", format!("las columnas de la primera plaza de las {} primeras Facciones sondean el cupo diario de un residente (día {}) y las de las {} primeras compran cobre y estaño para venderlos a su plaza por el mostrador (día {}) y entrar en los trueques", sc::FACCIONES_SONDEO, sc::DIA_SONDEO, sc::FACCIONES_MINERALES, sc::DIA_MINERALES)),
        parametro("Andamiaje de administración", format!("oro en el carro de las columnas (`DarSuministro`), el héroe de cada plaza (creado y colocado en su columna por administración, sin residencia salvo durante el sondeo de cupo, que le fija residencia y almacén personal), nivel de las Facciones; oro inyectado en total: {} u", r3(c.oro_inyectado as f64 / U))),
        parametro("Comisión / peaje", format!("{} % misma Facción, {} % otra; peaje {} % de lo que lleva a cada ciudad ajena que cruza", r.comision.tasa_misma_faccion as f64 / 1e4, r.comision.tasa_externa as f64 / 1e4, r.peaje as f64 / 1e4)),
        parametro("Trueque", format!("plazo de {} min para contestar y para cumplir", r.trueque.plazo / 60_000)),
        parametro("Flota", format!("cooldown de {} min entre caravanas; Mercado de nivel 1: {} caravanas y {} carros", r.flota.cooldown_creacion / 60_000, mercado.cupo_caravanas, mercado.cupo_carros)),
        parametro("Red de caminos", format!("las rutas caducan a los {} min sin lanzarse (las aristas no salen nunca); {} rutas hacen un camino y {} una calzada; sobre una arista el coste del tramo se multiplica por {}", r.red.caducidad / 60_000, r.red.escalon_camino, r.red.escalon_calzada, r.navegacion.factor_camino as f64 / 1e6)),
        parametro("Reputación", format!("+{} por trueque cumplido, {} por incumplido, decaimiento {} puntos por minuto, umbral bajo {}", r.reputacion.bonus_trueque_cumplido as f64 / 1e6, r.reputacion.penalizacion_trueque_incumplido as f64 / 1e6, r.reputacion.decaimiento_por_minuto as f64 / 1e6, r.reputacion.umbral_bajo_para_comision as f64 / 1e6)),
        parametro("Mercenarios", format!("el mercado se repone cada {} min con el {} % de lo comerciado entre Facciones (tope {} por bien); madera y piedra con pila propia y cupo diario de {} y {} por residente", r.mercenarios.cada / 60_000, r.mercenarios.proporcion_repone as f64 / 1e4, r.mercenarios.tope_por_bien / 1000, r.mercenarios.cupo_diario.get(&Recurso::Madera).copied().unwrap_or(0) / 1000, r.mercenarios.cupo_diario.get(&Recurso::Piedra).copied().unwrap_or(0) / 1000)),
        parametro("Límites del batch", "un héroe sin tropa por plaza y sin cargos (manda el propietario de la columna), sin tecnologías (solo hay pueblos de Cantera), sin guerra (ningún encuentro se resuelve: no hay caravanas perdidas por combate), sin mercado de la plaza más allá del sondeo de minerales"),
    ]
}

// ───────────────────────────── Ayudas ─────────────────────────────

fn n_hecho(x: &Entrada, codigo: &str) -> f64 {
    x.corrida.vigia.n(codigo) as f64
}

fn ordenes(x: &Entrada, etiqueta: &str) -> (u64, u64) {
    x.corrida.ordenes.get(etiqueta).map_or((0, 0), |c| (c.aceptadas, c.rechazos.values().sum()))
}

fn rechazo(x: &Entrada, etiqueta: &str, codigo: &str) -> f64 {
    x.corrida.ordenes.get(etiqueta).and_then(|c| c.rechazos.get(codigo)).copied().unwrap_or(0) as f64
}

fn minutos(ms: &[i64]) -> Vec<f64> {
    ms.iter().map(|&m| m as f64 / MIN).collect()
}

fn dia_de(t: i64) -> usize {
    (t as f64 / DIA) as usize
}

// ───────────────────────────── Métricas ─────────────────────────────

/// Cuántas plazas llegaron a tener Mercado y flota.
fn arranque(x: &Entrada) -> Vec<Metrica> {
    let (c, e) = (x.corrida, x.corrida.nucleo.estado());
    let vivas = e.asentamientos.len();
    let con_mercado = e.asentamientos.values().filter(|a| a.economia.edificios.iter().any(|b| b.tipo == Tipo::Mercado && b.activo())).count();
    let con_flota = e.asentamientos.values().filter(|a| c.nucleo.flota_de(a.id).iter().any(|k| k.carros.iter().any(|y| y.animal.is_some()))).count();
    let dia_mitad = c.muestras.iter().find(|m| 2 * m.con_mercado >= m.plazas_vivas.max(1)).map(|m| m.dia as f64);
    vec![
        metrica("colocadas", "Plazas colocadas", "plazas", INFO, Some(c.plazas.len() as f64), "Doc 1.2", &format!("De las {} pedidas; las demás no encontraron un sitio fundable (separación, exclusión junto a campamentos y madera suficiente a 10 u) o su Facción ya tenía el cap.", x.e.plazas)),
        metrica("plazas_vivas", "Plazas vivas al final", "plazas", INFO, Some(vivas as f64), "Economia_Construccion §5", "Las que no cayeron en ruinas."),
        metrica("mercado_pct", "Plazas con Mercado activo al final", "%", min(90.0), pct(con_mercado, vivas), "Comercio_Caravanas §5.1 (el Mercado activo es el requisito de la flota)", "El jugador lo encola cuando sobra madera; tarda 8 h en construirse."),
        metrica("mercado_dia_mitad", "Día en que la mitad de las plazas ya tiene Mercado", "días", INFO, dia_mitad, "Comercio_Caravanas §5.1", "Primer día de la foto diaria con al menos la mitad de las plazas vivas con Mercado activo."),
        metrica("flota_pct", "Plazas con una caravana con buey al final", "%", min(90.0), pct(con_flota, vivas), "Comercio_Caravanas §5.2", "Una caravana con al menos un carro con animal: es la que el reparto automático puede lanzar."),
    ]
}

fn trueques(x: &Entrada) -> Vec<Metrica> {
    let (c, v) = (x.corrida, &x.corrida.vigia);
    let propuestos = n_hecho(x, "comercio.trueque_propuesto");
    let aceptados = n_hecho(x, "comercio.trueque_aceptado");
    let cumplidos: Vec<_> = v.trueques.values().filter(|t| t.cumplido.is_some()).collect();
    let entre = cumplidos.iter().filter(|t| t.entre_facciones).count();
    let duran: Vec<f64> = cumplidos.iter().filter_map(|t| Some((t.cumplido? - t.aceptado?) as f64 / MIN)).collect();
    let repartos = minutos(&v.repartos_ms);
    let viajes = n_hecho(x, "comercio.caravana_sale");
    let dias = x.e.dias as f64;
    vec![
        metrica("trueques_propuestos", "Trueques propuestos", "trueques", INFO, Some(propuestos), "Comercio_Caravanas §4.1", "Órdenes `ProponerTrueque` aceptadas: la doble coincidencia entre dos plazas vecinas con Mercado y flota libre."),
        metrica("trueques_aceptados", "Trueques aceptados", "trueques", INFO, Some(aceptados), "Comercio_Caravanas §4.2", "La plaza B acepta si tiene flota libre y puede cumplir su lado."),
        metrica("trueques_aceptacion_pct", "Propuestas que la otra parte acepta", "%", min(80.0), pct(aceptados as usize, propuestos as usize), "Comercio_Caravanas §4.2", "El jugador solo propone lo que ambas partes pueden cumplir en ese momento; rechaza la otra parte lo que dejó de cumplirse en los diez minutos de respuesta."),
        metrica("trueques_cumplidos", "Trueques cumplidos", "trueques", INFO, Some(cumplidos.len() as f64), "Comercio_Caravanas §4.4", "Los dos lados saldados."),
        metrica("trueques_cumplimiento_pct", "Trueques aceptados que se cumplen", "%", min(95.0), pct(cumplidos.len(), aceptados as usize), "Comercio_Caravanas §4.3 (el reparto cumple lo pactado en el plazo)", "Cumplidos entre aceptados: los que quedan a medias al final de los 30 días también restan."),
        metrica("trueques_incumplidos", "Trueques vencidos sin cumplir", "trueques", max(0.0), Some(v.incumplidos as f64), "Comercio_Caravanas §4.3 (vencer)", "Aceptados que llegan a su plazo con lo pactado sin entregar (penalizan la reputación de ambas Facciones si son distintas)."),
        metrica("trueques_entre_facciones_pct", "Trueques cumplidos entre Facciones distintas", "%", INFO, pct(entre, cumplidos.len()), "Comercio_Caravanas §3.2", "Pagan la comisión externa y suman reputación; el jugador busca pareja de otra Facción el 60 % de las veces."),
        metrica("trueques_de_minerales", "Trueques cumplidos en los que se movió cobre, estaño o hierro", "trueques", INFO, Some(v.trueques_de_minerales as f64), "Comercio_Caravanas §4 y §9", "Los minerales solo existen si una columna los compra a los mercenarios (y los vende a su plaza por el mostrador): son lo único escaso que las plazas se pueden ofrecer."),
        metrica("trueques_por_plaza_dia", "Trueques cumplidos por plaza y día", "trueques", INFO, Some(cumplidos.len() as f64 / (c.plazas.len().max(1) as f64 * dias)), "Comercio_Caravanas §4", "Intensidad media del comercio. Sin consumo que vacíe los almacenes (el batch no gasta nada más que la construcción automática) la demanda se agota: véase la serie de trueques por día."),
        metrica("trueque_dura_mediana_min", "Tiempo mediano de un trueque, de la aceptación a cumplirse", "min", INFO, mediana(&duran), "Comercio_Caravanas §4.3", "Reparto más viaje de ida de cada lado, con las caravanas de repesca por el peaje."),
        metrica("trueque_dura_p90_min", "Tiempo del trueque en el percentil 90", "min", max(720.0), percentil(&mut duran.clone(), 0.9), "Comercio_Caravanas §4 (plazo de 1.440 min)", "Que no se acerque al plazo de 24 h: el límite verde es la mitad."),
        metrica("reparto_mediana_min", "Tiempo mediano de reparto, de la aceptación a la salida de la caravana", "min", max(5.0), mediana(&repartos), "Comercio_Caravanas §4.3 (se reevalúa al aceptar)", "Con la caravana libre y la mercancía en el almacén sale en el mismo instante; si no, espera el pulso de un minuto."),
        metrica("reparto_p90_min", "Tiempo de reparto en el percentil 90", "min", max(30.0), percentil(&mut repartos.clone(), 0.9), "Comercio_Caravanas §4.3", "Salidas de repesca incluidas (el segundo viaje de un lado que quedó corto por el peaje)."),
        metrica("viajes_por_trueque", "Salidas de caravana por trueque cumplido", "salidas", max(2.5), (!cumplidos.is_empty()).then(|| viajes / cumplidos.len() as f64), "Comercio_Caravanas §4.3 y §7.3 (una salida por lado)", "Dos si cada lado sale una vez. Más cuando el peaje (el 2 % de la carga a cada ciudad ajena que cruza) deja la entrega por debajo de lo pactado: la caravana vuelve y sale otra vez con el resto, y vuelve a pagar peaje."),
    ]
}

fn caravanas(x: &Entrada) -> Vec<Metrica> {
    let (c, v) = (x.corrida, &x.corrida.vigia);
    let e = c.nucleo.estado();
    let lanzadas = n_hecho(x, "comercio.caravana_sale");
    let en_ruta = e.comercio.flota.iter().filter(|k| k.tipo == comercio::caravana::TipoCaravana::Comercial && matches!(k.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando | EstadoCaravana::Preparando)).count();
    let viajes: Vec<f64> = v.viajes.iter().map(|y| y.dura_ms as f64 / MIN).collect();
    let cola = v.viajes.iter().filter(|y| y.carga < 1_000).count();
    let carga = v.carga_entregada as f64 / U;
    vec![
        metrica("caravanas_lanzadas", "Caravanas lanzadas", "salidas", INFO, Some(lanzadas), "Comercio_Caravanas §4.3 y §6", "Salidas del reparto automático (`comercio.caravana_sale`)."),
        metrica("caravanas_entregadas", "Caravanas que llegan a su destino", "llegadas", INFO, Some(n_hecho(x, "comercio.caravana_llega")), "Comercio_Caravanas §6", "Entregas hechas; las que siguen en camino al acabar no cuentan."),
        metrica("caravanas_perdidas", "Caravanas perdidas", "caravanas", max(0.0), Some(n_hecho(x, "comercio.caravana_perdida")), "Comercio_Caravanas §10", "Sin ruinas ni conquistas ni combate en el batch, ninguna debe perderse."),
        metrica("caravanas_retenidas", "Caravanas en camino al final de la simulación", "caravanas", INFO, Some(en_ruta as f64), "Comercio_Caravanas §6", "En tránsito, retornando o preparando en el último instante: no están retenidas por nada, solo cogidas a mitad de viaje; las demás esperan libres en su plaza."),
        metrica("viaje_mediana_min", "Duración mediana de un viaje de ida", "min", INFO, mediana(&viajes), "Comercio_Caravanas §6.1 (coste por tramo y velocidad del animal)", "De la salida a la entrega, con el peaje incluido."),
        metrica("viaje_p90_min", "Duración del viaje en el percentil 90", "min", INFO, percentil(&mut viajes.clone(), 0.9), "Comercio_Caravanas §6.1", "Los viajes largos son los que cruzan una ciudad ajena (la ruta pasa por su centro)."),
        metrica("viaje_max_min", "Viaje de ida más largo", "min", INFO, viajes.iter().copied().reduce(f64::max), "Comercio_Caravanas §6.1", "Vecinas a menos de 300 u."),
        metrica("carga_entregada", "Carga entregada por las caravanas", "unidades", INFO, Some(carga), "Comercio_Caravanas §4.4", "Suma de lo que llega a destino (ya sin peajes)."),
        metrica("carga_perdida_pct", "Carga que no cabe en el almacén de destino", "%", max(1.0), Some(100.0 * v.perdido_en_entrega as f64 / (v.carga_entregada.max(1)) as f64), "Comercio_Caravanas §4.4 (topado por la capacidad)", "El jugador solo ofrece lo que cabe al otro lado; lo que sobre se pierde."),
        metrica("viajes_de_repesca_pct", "Viajes de ida con menos de una unidad", "%", INFO, pct(cola, v.viajes.len()), "Comercio_Caravanas §4.3 (sale con cualquier carga positiva)", "Con la economía entera (C3b) todo viaja en unidades enteras, así que no debe haber ninguno; la regla de C3 (2026-10-19) que saldaba una línea con menos de una unidad pendiente se retiró (EE-6)."),
        metrica("comision_oro", "Comisión acuñada en las entregas", "oro", INFO, Some(v.comision as f64 / U), "Comercio_Caravanas §3.3 (se acuña en el destino)", "Una de las fuentes de oro del juego; solo entre plazas distintas. Es la que entra de verdad: se topa por el oro que cabe en el almacén del destino."),
        metrica("valor_entregado", "Valor de lo entregado por las caravanas", "oro", INFO, Some(v.valor_entregado as f64 / U), "Comercio_Caravanas §3.1 (precio base, también el derivado de la receta de los manufacturados, C4)", "Suma del `valor` de cada `comercio.caravana_llega`: lo que llevaba a precio vigente, sobre lo que se calcula la comisión (entre plazas de la misma Facción, 3 %; entre Facciones, 8 %; las dos por hasta ×1,5 por distancia)."),
        metrica("comision_sobre_valor_pct", "Comisión acuñada sobre el valor entregado", "%", INFO, (v.valor_entregado > 0).then(|| 100.0 * v.comision as f64 / v.valor_entregado as f64), "Comercio_Caravanas §3.3 (tasas 3 % y 8 %, bonus por distancia ×1-1,5)", "Si las tasas se cobrasen enteras saldría entre el 3 % y el 12 %; por debajo, lo que falta es la comisión que no cupo en el oro del destino (o entregas dentro de la misma plaza, que no pagan)."),
        metrica("recargo_oro", "Recargo por mala reputación pagado", "oro", INFO, Some(v.recargo as f64 / U), "Comercio_Caravanas §3.3", "0 mientras ninguna Facción baje del umbral de reputación."),
        metrica("animales_oro", "Oro destruido en animales", "oro", INFO, Some(v.animales_comprados as f64 * 12.0), "Comercio_Caravanas §5.3 (sumidero)", "Un buey cuesta 12 de oro: los animales comprados por las plazas."),
    ]
}

fn peajes(x: &Entrada) -> Vec<Metrica> {
    let v = &x.corrida.vigia;
    let total: i64 = v.peajes_pagados.values().map(|p| p.1).sum();
    let mayor = |m: &BTreeMap<u32, (u64, i64)>| (total > 0).then(|| 100.0 * m.values().map(|p| p.1).max().unwrap_or(0) as f64 / total as f64);
    vec![
        metrica("peajes_cobros", "Peajes cobrados", "peajes", INFO, Some(n_hecho(x, "comercio.peaje_paso")), "Comercio_Caravanas §7.3", "Un peaje por cada ciudad ajena que cruza la ruta (también de Facciones aliadas)."),
        metrica("peajes_unidades", "Peajes: unidades cobradas", "unidades", INFO, Some(total as f64 / U), "Comercio_Caravanas §7.3", "El 2 % de lo que lleva la caravana en cada paso."),
        metrica("peajes_pct_carga", "Peajes sobre la carga que sale", "%", max(10.0), (v.carga_entregada + total > 0).then(|| 100.0 * total as f64 / (v.carga_entregada + total) as f64), "Comercio_Caravanas §7.3", "Lo cobrado entre lo entregado más lo cobrado. Con una o dos ciudades ajenas por ruta es del 2 al 4 %."),
        metrica("peajes_facciones_pagan", "Facciones que pagan peaje", "Facciones", INFO, Some(v.peajes_pagados.len() as f64), "Comercio_Caravanas §7.3", "Facciones cuyas caravanas cruzaron una ciudad ajena."),
        metrica("peajes_facciones_cobran", "Facciones que cobran peaje", "Facciones", INFO, Some(v.peajes_cobrados.len() as f64), "Comercio_Caravanas §7.3", "Facciones con una ciudad cruzada por una caravana ajena."),
        metrica("peaje_top_pagador_pct", "Parte del peaje que paga la Facción que más paga", "%", INFO, mayor(&v.peajes_pagados), "Comercio_Caravanas §7.3", "Concentración de quien paga."),
        metrica("peaje_top_cobrador_pct", "Parte del peaje que cobra la Facción que más cobra", "%", INFO, mayor(&v.peajes_cobrados), "Comercio_Caravanas §7.3", "Concentración de quien cobra: la ciudad que queda en medio de las rutas."),
    ]
}

fn reputacion(x: &Entrada) -> Vec<Metrica> {
    let c = x.corrida;
    let r = &c.nucleo.estado().reglas.economia.comercio.reputacion;
    let punto = 1e6;
    let finales: Vec<f64> = c.muestras.last().map(|m| m.reputacion.iter().map(|&v| v as f64 / punto).collect()).unwrap_or_default();
    let todas: Vec<f64> = c.muestras.iter().flat_map(|m| m.reputacion.iter().map(|&v| v as f64 / punto)).collect();
    let con_dato = c.nucleo.estado().comercio.reputacion.len();
    let positivas = finales.iter().filter(|&&v| v > 0.0).count();
    vec![
        metrica("rep_facciones", "Facciones con reputación (han cumplido algún trueque entre Facciones)", "Facciones", INFO, Some(con_dato as f64), "Comercio_Caravanas §4.6", "Las demás valen 0."),
        metrica("rep_max_alcanzada", "Reputación más alta alcanzada por una Facción", "puntos", max(99.0), todas.iter().copied().reduce(f64::max), "Doc 2.7 (de −100 a +100)", "Medida en la foto diaria de cada Facción: que no se sature en el tope."),
        metrica("rep_min_alcanzada", "Reputación más baja alcanzada por una Facción", "puntos", min(-39.9), todas.iter().copied().reduce(f64::min), "Comercio_Caravanas §4.6 (umbral bajo: −40)", "Por encima del umbral que activa el recargo de la comisión."),
        metrica("rep_mediana_final", "Reputación mediana de las Facciones con dato al final", "puntos", INFO, mediana(&finales.iter().copied().filter(|&v| v != 0.0).collect::<Vec<_>>()), "Comercio_Caravanas §4.6", "Sin contar las Facciones a 0."),
        metrica("rep_positivas_final_pct", "Facciones con reputación positiva al final", "%", INFO, pct(positivas, finales.len()), "Doc 2.7 (decae lentamente)", "Con la reputación decayendo hacia 0, solo queda positiva la que comerció hace poco."),
        metrica("rep_vida_de_un_trueque_dias", "Días que tarda en borrarse el bonus de un trueque cumplido", "días", INFO, (r.decaimiento_por_minuto > 0).then(|| r.bonus_trueque_cumplido as f64 / r.decaimiento_por_minuto as f64 / 1440.0), "Comercio_Caravanas §4.6 (decaimiento)", "Bonus entre decaimiento: lo que dura un trueque cumplido si no se suma otro."),
    ]
}

fn red(x: &Entrada) -> Vec<Metrica> {
    let (c, v) = (x.corrida, &x.corrida.vigia);
    let nucleo = &c.nucleo;
    let (r, fin) = (&nucleo.estado().reglas.economia.comercio, nucleo.ahora());
    let pico = &c.pico_red;
    let pares: BTreeSet<(u64, u64)> = v.viajes.iter().map(|y| (y.origen, y.destino)).collect();
    let vigentes = nucleo.red_de_caminos().rutas().filter(|q| fin - q.ultimo_lanzamiento < r.red.caducidad).count();
    let (sin_peaje, con_peaje) = (aceleracion(x, false), aceleracion(x, true));
    vec![
        metrica("red_aristas", "Aristas de la red de caminos", "aristas", INFO, Some(nucleo.red_de_caminos().aristas().count() as f64), "Comercio_Caravanas §7.4 (no salen nunca)", "Todas las aristas que alguna ruta ha pisado: sobre cualquiera la caravana avanza más rápido."),
        metrica("red_rutas_registradas", "Pares origen-destino que han lanzado una caravana", "rutas", INFO, Some(pares.len() as f64), "Comercio_Caravanas §7.4", "Una ruta por par ordenado, que se sustituye en cada lanzamiento."),
        metrica("red_rutas_vigentes", "Rutas vigentes al final", "rutas", INFO, Some(vigentes as f64), "Comercio_Caravanas §7.4 (caducan a los 4.320 min)", "Las que se lanzaron en los últimos tres días."),
        metrica("red_rutas_caducadas", "Rutas que han caducado", "rutas", INFO, Some(pares.len().saturating_sub(vigentes) as f64), "Comercio_Caravanas §7.4 (poda)", "Registradas menos vigentes: la poda es perezosa y se hace al registrar la siguiente ruta; las aristas se quedan como sendero."),
        metrica("red_aristas_camino", "Aristas que llegaron a escalón de camino", "aristas", INFO, Some(pico.values().filter(|&&p| p >= r.red.escalon_camino).count() as f64), "Comercio_Caravanas §7.4", &format!("Pisadas alguna vez, en una foto diaria, por {} o más rutas vigentes.", r.red.escalon_camino)),
        metrica("red_aristas_calzada", "Aristas que llegaron a escalón de calzada", "aristas", INFO, Some(pico.values().filter(|&&p| p >= r.red.escalon_calzada).count() as f64), "Comercio_Caravanas §7.4", &format!("Pisadas alguna vez, en una foto diaria, por {} o más rutas vigentes.", r.red.escalon_calzada)),
        metrica("red_peso_max", "Mayor peso que tuvo una arista", "rutas", INFO, Some(pico.values().copied().max().unwrap_or(0) as f64), "Comercio_Caravanas §7.4", "Cuántas rutas vigentes pisaban a la vez la arista más usada, en las fotos diarias."),
        metrica("red_aceleracion", "Viaje más rápido entre el más lento de un par de plazas, sin peajes (mediana)", "razón", (Some(0.4), Some(0.7)), sin_peaje, "Comercio_Caravanas §7.4 (el primer viaje abre el camino, el segundo va el doble de rápido)", "Para cada par de plazas con al menos dos viajes de ida (en cualquiera de los dos sentidos) y ninguno con peaje: duración del más corto entre la del más largo. Esperado cerca del `factorCamino` (0,5): el primer viaje del par va sin camino y los demás por el camino que dejó. Los dos sentidos salen a la vez y el segundo ya aprovecha las aristas del primero, así que se compara el mejor con el peor y no el segundo con el primero."),
        metrica("red_aceleracion_con_peaje", "Lo mismo en los pares cuyos viajes cruzan una ciudad ajena (mediana)", "razón", (Some(0.4), Some(0.7)), con_peaje, "Comercio_Caravanas §7.4 y §7.2", "La ruta pasa por el centro de la ciudad ajena (un A* hasta ella y otro desde ella) y esos tramos de enganche no se acortan por igual."),
    ]
}

/// Para cada par de plazas con varios viajes de ida (en cualquier sentido), razón entre el más corto y el más largo. Con `con_peaje` solo
/// cuentan los viajes que pagaron peaje; sin él, solo los pares en los que ninguno lo pagó.
fn aceleracion(x: &Entrada, con_peaje: bool) -> Option<f64> {
    let par = |y: &Viaje| (y.origen.min(y.destino), y.origen.max(y.destino));
    let mut por_par: BTreeMap<(u64, u64), Vec<i64>> = BTreeMap::new();
    x.corrida.vigia.viajes.iter().filter(|y| y.dura_ms > 0 && y.origen != y.destino && (y.peajes > 0) == con_peaje).for_each(|y| por_par.entry(par(y)).or_default().push(y.dura_ms));
    let con_otro: BTreeSet<(u64, u64)> = x.corrida.vigia.viajes.iter().filter(|y| y.peajes > 0).map(par).collect();
    let razones: Vec<f64> = por_par.into_iter().filter(|(k, v)| v.len() >= 2 && (con_peaje || !con_otro.contains(k))).map(|(_, v)| *v.iter().min().unwrap_or(&1) as f64 / *v.iter().max().unwrap_or(&1) as f64).collect();
    mediana(&razones)
}

fn mercenarios(x: &Entrada) -> Vec<Metrica> {
    let (c, v) = (x.corrida, &x.corrida.vigia);
    let r = &c.nucleo.estado().reglas.economia.comercio.mercenarios;
    let unidades = |rec: Recurso| v.compras.get(&rec.indice()).map_or(0.0, |k| k.unidades as f64 / U);
    let maximo_cupo = |rec: Recurso| v.compras_de_residentes.iter().filter(|((_, _, i), _)| *i == rec.indice()).map(|(_, &n)| n as f64 / U).reduce(f64::max).unwrap_or(0.0);
    let (ok, no) = ordenes(x, "ComprarEnCampamento");
    let pila = r.pilas.get(&Recurso::Madera).copied().unwrap_or(1);
    let stock_min = c.nucleo.mercenarios().stock.keys().filter_map(|&k| c.nucleo.stock_de_campamento(k)).map(|s| 100.0 * s.de(Recurso::Madera) as f64 / pila as f64).reduce(f64::min);
    vec![
        metrica("merc_compras", "Compras a los mercenarios", "compras", INFO, Some(v.compras.values().map(|k| k.veces).sum::<u64>() as f64), "Comercio_Caravanas §9.2", "Órdenes `ComprarEnCampamento` aceptadas (materiales de las obras de pueblo, minerales y sondeos de cupo)."),
        metrica("merc_madera", "Madera comprada", "unidades", INFO, Some(unidades(Recurso::Madera)), "Comercio_Caravanas §9.2", "Casi toda para las obras de pueblo (90 por obra)."),
        metrica("merc_piedra", "Piedra comprada", "unidades", INFO, Some(unidades(Recurso::Piedra)), "Comercio_Caravanas §9.2", "Solo en el sondeo de cupo: la obra de Cantera no pide piedra."),
        metrica("merc_minerales", "Cobre y estaño comprados", "unidades", INFO, Some(unidades(Recurso::Cobre) + unidades(Recurso::Estano)), "Comercio_Caravanas §9.1", "Para los trueques de minerales, que solo existen si se compran aquí."),
        metrica("cobre_gastado", "Cobre que ya no está en el mundo (fundido y gastado en obras)", "unidades", INFO, c.balance_minerales.iter().find(|b| b.0 == Recurso::Cobre).map(|b| (b.1 - b.2) as f64 / U), "Economia_Construccion §6 (recetas y mejoras)", "Lo inicial más lo comprado menos lo que hay entre cobre y lingotes (2 de cobre por lingote): la Fundición lo funde sin tecnología y la mejora de la Armería gasta los lingotes; por eso solo el estaño y el hierro entran en el invariante de conservación."),
        metrica("merc_oro", "Oro cobrado por los campamentos (sumidero)", "oro", INFO, Some(v.oro_de_compras as f64 / U), "Comercio_Caravanas §9.1 (el oro cobrado se destruye)", "Lo gastado por las columnas con el oro del carro."),
        metrica("merc_rechazo_stock_pct", "Compras rechazadas por falta de existencias", "%", INFO, pct(rechazo(x, "ComprarEnCampamento", "mercenarios.sin_stock") as usize, (ok + no) as usize), "Comercio_Caravanas §9.3 (pila de 300 de madera, repuesta cada 3 h)", "Intentos de compra que no pudieron servirse porque el campamento más cercano ya no tenía el bien: varias plazas comparten campamento."),
        metrica("merc_cupo_agotado", "Compras rechazadas por cupo diario", "compras", INFO, Some(rechazo(x, "ComprarEnCampamento", "mercenarios.cupo_agotado")), "Comercio_Caravanas §9.2 (cupo diario de residentes)", "Solo las compran los residentes del sondeo; el cupo no frena a los visitantes."),
        metrica("merc_cupo_madera_max", "Mayor compra diaria de madera de un residente", "unidades", max(r.cupo_diario.get(&Recurso::Madera).copied().unwrap_or(0) as f64 / U), Some(maximo_cupo(Recurso::Madera)), "Comercio_Caravanas §9.2", "Nunca por encima del cupo diario."),
        metrica("merc_cupo_piedra_max", "Mayor compra diaria de piedra de un residente", "unidades", max(r.cupo_diario.get(&Recurso::Piedra).copied().unwrap_or(0) as f64 / U), Some(maximo_cupo(Recurso::Piedra)), "Comercio_Caravanas §9.2", "Nunca por encima del cupo diario."),
        metrica("merc_reposiciones", "Reposiciones del mercado de mercenarios", "reposiciones", INFO, Some(n_hecho(x, "mercenarios.repuesto")), "Comercio_Caravanas §9.3 (cada 3 h)", "Solo se programan si hay algo que reponer: lo comerciado entre Facciones o una pila por debajo de su tope."),
        metrica("merc_campamentos_tocados", "Campamentos cuyo stock se tocó", "campamentos", INFO, Some(c.nucleo.mercenarios().stock.len() as f64), "Comercio_Caravanas §9", &format!("De {} del mundo; el que nadie toca conserva el inicial sin guardarse.", x.geo.campamentos.len())),
        metrica("merc_madera_min_pct", "Pila de madera más baja al final", "% de la pila", INFO, stock_min, "Comercio_Caravanas §9.3", "Entre los campamentos tocados."),
    ]
}

fn pueblos(x: &Entrada) -> Vec<Metrica> {
    let (c, v) = (x.corrida, &x.corrida.vigia);
    let e = c.nucleo.estado();
    let marcados = v.pueblos_marcados.len();
    let completados = v.pueblos_completados.len();
    let obra: Vec<f64> = v.pueblos_completados.iter().filter_map(|(p, &t)| Some((t - v.pueblos_marcados.get(p)?) as f64 / (60.0 * MIN))).collect();
    let nivel2: Vec<_> = e.asentamientos.values().filter(|a| a.nivel() >= 2).collect();
    let con_pueblo = nivel2.iter().filter(|a| c.nucleo.pueblos_de(a.id).iter().any(|p| p.activo())).count();
    let viajes: Vec<f64> = c.viajes_por_obra.iter().map(|&n| n as f64).collect();
    let nominal: f64 = {
        let base = e.reglas.economia.datos.catalogo.tipo(Tipo::Cantera).produccion.map_or(0, |p| p.1) as f64;
        v.pueblos_completados.values().map(|&t| base * (c.nucleo.ahora() - t) as f64 / MIN).sum()
    };
    let sacado = v.acarreo_sacado as f64;
    let insolvente = rechazo(x, "SolicitarAscenso", "ascenso.insolvente");
    // Lo que el acarreo cuenta hoy en la solvencia del ascenso (C2): la media por minuto de lo que ingresó en las madres en las últimas 24 h, frente a la
    // producción nominal de los pueblos activos (lo que contaba antes de C2).
    let hora_fin = c.nucleo.ahora().div_euclid(60 * MIN as i64);
    let en_ventana: f64 = e.asentamientos.values().flat_map(|a| a.economia.ingresos_acarreo.iter()).filter(|i| i.hora > hora_fin - asentamiento::pueblos::HORAS_VENTANA).map(|i| i.cantidad as f64).sum::<f64>() / (asentamiento::pueblos::HORAS_VENTANA as f64 * 60.0);
    let nominal_hoy: f64 = {
        let base = e.reglas.economia.datos.catalogo.tipo(Tipo::Cantera).produccion.map_or(0, |p| p.1) as f64;
        base * e.asentamientos.values().map(|a| c.nucleo.pueblos_de(a.id).iter().filter(|p| p.activo()).count()).sum::<usize>() as f64
    };
    let marcar = x.corrida.ordenes.get("MarcarPueblo");
    let rechazos_marcar = marcar.map_or("ninguno".to_string(), |c| if c.rechazos.is_empty() { "ninguno".into() } else { c.rechazos.iter().map(|(k, n)| format!("{k} {n}")).collect::<Vec<_>>().join(", ") });
    let por_nivel = |n: u32| e.asentamientos.values().filter(|a| a.nivel() == n).count() as f64;
    vec![
        metrica("pueblos_marcados", "Pueblos marcados", "pueblos", INFO, Some(marcados as f64), "Economia_Construccion §14", "Obras de mapa pendientes de materiales (ocupan cupo desde que se marcan)."),
        metrica("pueblos_completados", "Pueblos levantados", "pueblos", INFO, Some(completados as f64), "Economia_Construccion §14", "Con todos los materiales entregados y la obra de 4 h terminada."),
        metrica("pueblos_completados_pct", "Pueblos marcados que se levantan", "%", min(90.0), pct(completados, marcados), "Economia_Construccion §14", "Los que faltan al final esperan materiales o una obra en curso."),
        metrica("pueblo_obra_mediana_h", "Tiempo mediano de la marca a la obra terminada", "h", INFO, mediana(&obra), "Economia_Construccion §14 (obra de 4 h)", "Incluye los viajes de la columna al campamento y las esperas por falta de existencias."),
        metrica("pueblo_viajes_por_obra", "Viajes de compra por obra (mediana)", "viajes", max(2.0), mediana(&viajes), "Economia_Construccion §14 (los materiales se llevan físicamente)", "Uno si el campamento tiene los materiales a la primera; más si las plazas vecinas se los han llevado."),
        metrica("pueblo_plazas_con_pueblo_pct", "Plazas de nivel 2 o más con algún pueblo activo", "%", INFO, pct(con_pueblo, nivel2.len()), "Economia_Construccion §14 (madre de nivel 2)", "El cupo de plazas de nivel 2 de la Facción (3) y su nivel de ascenso limitan cuántas llegan."),
        metrica("acarreo_viajes", "Viajes del carro de acarreo hasta la madre", "viajes", INFO, Some(n_hecho(x, "acarreo.llega")), "Comercio_Caravanas §12 (carro de acarreo)", "Cada vez que el almacén local de un pueblo se llena (500) sale el carro."),
        metrica("acarreo_ingresado", "Piedra que el acarreo ingresa en las madres", "unidades", INFO, Some(v.acarreo_carga as f64 / U), "Comercio_Caravanas §12", "Lo que cabe en el almacén de la madre al llegar."),
        metrica("acarreo_perdido_pct", "Acarreo que no cabe en el almacén de la madre", "%", max(5.0), (sacado > 0.0).then(|| 100.0 * v.acarreo_perdido as f64 / sacado), "Comercio_Caravanas §12 (topado por su capacidad: lo que no cabe se pierde)", "Con el almacén de piedra de la madre lleno, todo lo que acarrea el pueblo se pierde."),
        metrica("acarreo_nominal_pct", "Lo que sale por acarreo respecto a la producción nominal de los pueblos", "%", (Some(90.0), Some(100.0)), (nominal > 0.0).then(|| 100.0 * sacado / nominal), "Economia_Construccion §14 y §11 (solvencia)", "Producción de la Cantera sin mano de obra limitada (10 por minuto) desde que se levanta frente a lo que el carro saca. Antes de C2 el ascenso contaba la primera como ingreso; desde C2 (2026-10-19) cuenta lo que ingresa (ver `acarreo_en_solvencia_pct`)."),
        metrica("acarreo_en_solvencia_pct", "Acarreo que cuenta en la solvencia del ascenso al final, sobre la producción nominal de los pueblos activos", "%", INFO, (nominal_hoy > 0.0).then(|| 100.0 * en_ventana / nominal_hoy), "Economia_Construccion §11.2 y §14 (ventana de 24 h, C2)", &format!("Suma en todas las madres de la media por minuto de lo que el acarreo ingresó en las últimas {} h ({:.1} por minuto) frente a la Cantera nominal de los pueblos activos ({:.1} por minuto): con el almacén de la madre lleno, el pueblo apenas cuenta.", asentamiento::pueblos::HORAS_VENTANA, en_ventana / U, nominal_hoy / U)),
        metrica("pueblo_marcar_rechazos", "Órdenes `MarcarPueblo` rechazadas", "órdenes", INFO, Some(marcar.map_or(0, |c| c.rechazos.values().sum::<u64>()) as f64), "Economia_Construccion §14 (requisitos; sin ruta por tierra a la madre no se funda, decisión 5 de 2026-10-23)", &format!("Por código: {rechazos_marcar}. El jugador marca el primer nodo de Cantera de `candidatos_de_pueblo`, que no mira la ruta.")),
        metrica("plazas_nivel2_fin", "Plazas de nivel 2 al final", "plazas", INFO, Some(por_nivel(2)), "Doc 4.5", "Nivel alcanzado."),
        metrica("plazas_nivel3_fin", "Plazas de nivel 3 o más al final", "plazas", INFO, Some((3..=5).map(por_nivel).sum()), "Doc 4.5", "La Facción de nivel 8 da cupo de 3 plazas de nivel 2; el nivel 3 pide además edificios que el jugador no construye (Barracón, Galería de tiro)."),
        metrica("misiones_terminadas_pct", "Viajes de columna que terminan de vuelta en la plaza", "%", min(90.0), pct(c.misiones.values().map(|m| m.1 as usize).sum(), c.misiones.values().map(|m| m.0 as usize).sum()), "Movimiento_Ejercitos (marchas) y Comercio_Caravanas §9", &format!("Viajes de la columna al campamento (y a la obra o al mostrador) lanzados y terminados: {}.", c.misiones.iter().map(|(k, m)| format!("{k} {}/{}", m.1, m.0)).collect::<Vec<_>>().join(", "))),
        metrica("ascenso_insolvente", "Peticiones de ascenso rechazadas por insolvencia", "peticiones", INFO, Some(insolvente), "Economia_Construccion §11.2 (solvencia)", "Desde C2 el acarreo cuenta por lo que ingresa en la madre en las últimas 24 h: un pueblo cuyo acarreo se pierde contra un almacén lleno ya no hace solvente a nadie."),
        multiplicador_distancia(x),
    ]
}

/// 5I (5C-2): el multiplicador del mantenimiento por la distancia a la capital de las plazas que no son su capital, al final.
fn multiplicador_distancia(x: &Entrada) -> Metrica {
    let (n, e) = (&x.corrida.nucleo, x.corrida.nucleo.estado());
    let escala = e.reglas.economia.datos.mantenimiento.escala_distancia.max(1) as f64;
    let v: Vec<f64> = e.asentamientos.values().filter_map(|a| n.capital_de(a.faccion).filter(|&c| c != a.id).and_then(|c| e.asentamientos.get(&c)).map(|c| 1.0 + a.posicion.dist(c.posicion) as f64 / escala)).collect();
    let (p90, maximo) = (percentil(&mut v.clone(), 0.9), v.iter().copied().fold(0.0, f64::max));
    let nota = format!("De las {} plazas que no son su capital (`× (1 + distancia / escalaDistancia)`, 80 u); percentil 90 {}, máximo {maximo:.2}.", v.len(), p90.map_or("—".into(), |p| format!("{p:.2}")));
    metrica("multiplicador_distancia", "Multiplicador del mantenimiento por la distancia a la capital (mediana, al final)", "×", INFO, mediana(&v), "Economia_Construccion §10.1 (5C-2)", &nota)
}

fn invariantes(x: &Entrada) -> Vec<Metrica> {
    let v = &x.corrida.violaciones;
    let b = &x.corrida.balance_minerales;
    let detalle = b.iter().map(|(r, debe, hay)| format!("{}: deben ser {} u (inicial más comprado) y hay {} u", r.nombre(), r3(*debe as f64 / U), r3(*hay as f64 / U))).collect::<Vec<_>>().join("; ");
    let uno = |id: &str, nombre: &str, valor: u64, origen: &str, nota: &str| metrica(id, nombre, "violaciones", max(0.0), Some(valor as f64), origen, nota);
    vec![
        uno("inv_almacen", "Almacenes de plazas y pueblos fuera de [0, capacidad]", v.almacen, "Economia_Construccion §15", "Comprobado cada 6 horas de mundo en todas las plazas y pueblos."),
        uno("inv_carga", "Carros y caravanas por encima de su capacidad o con cantidades negativas", v.carga, "Comercio_Caravanas §5", "Carro de cada columna y contenido de cada caravana de la flota."),
        uno("inv_fracciones", "Cantidades almacenadas que no son unidades enteras", v.fraccion, "Economia_Construccion §4 y Comercio_Caravanas §1 (economía entera, C3b)", "Almacenes de plaza, pueblo y avanzada (con su resto oculto en (−1000, 1000)), carros, caravanas, almacenes personales, existencias de los campamentos, trueques y órdenes del mercado: cada 6 horas de mundo."),
        uno("inv_trueque", "Trueques incoherentes", v.trueque, "Comercio_Caravanas §4", "Más entregado que pactado, o un trueque activo con una de sus plazas ya inexistente."),
        uno("inv_sin_plan", "Caravanas en camino sin plan o sin su evento de llegada", v.sin_plan, "Comercio_Caravanas §6 y Reloj_Y_Eventos", "Toda caravana en tránsito o retornando debe tener un plan de al menos dos vértices y su llegada vigente en la cola."),
        uno("inv_cola", "Eventos de comercio vigentes que no corresponden al estado", v.cola, "Reloj_Y_Eventos §2 (invalidación por generación)", "Llegadas, peajes, fin de preparación, caducidades y pulsos cuya entidad ya no está en el estado que los justifica."),
        uno("inv_plaza_ociosa", "Repartos programados en plazas sin ningún trueque activo", v.ociosa, "Comercio_Caravanas §4.3 (nada recurre por plaza ociosa)", "El pulso de un minuto del reparto solo existe mientras hay un lado pendiente sin cargar."),
        uno("inv_minuto", "Plazas sin su evento de minuto, o con uno de más", v.minuto, "Economia_Construccion §3 (un paso de minuto por plaza)", "Los eventos de minuto vigentes de la cola deben coincidir con las plazas vivas, una por plaza."),
        uno("inv_fines", "Obras, mejoras, ascensos o políticas en curso sin su cita de fin", v.fines, "Reloj_Y_Eventos §2 y Economia_Construccion §3 (fines exactos)", "Todo lo que está en curso y aún no vence debe tener en la cola su cita de fin, programada al empezar, que lo cumple en su instante exacto."),
        uno("inv_registro", "Versiones del registro de cambios perdidas antes de leerlas", v.registro, "Reloj_Y_Eventos (registro por versión)", "El vigía lee el registro cada media hora de mundo; el núcleo retiene 65.536 versiones."),
        metrica("inv_conservacion", "Minerales que no cuadran con lo comprado a los mercenarios", "minerales", max(0.0), Some(v.conservacion as f64), "Comercio_Caravanas §4.4 (nada se crea ni se pierde)", &format!("Cobre, estaño y hierro no los produce ni los consume ninguna economía de plaza: lo que hay en almacenes, carros, caravanas y almacenes personales debe ser lo inicial más lo comprado. {detalle}.")),
    ]
}

fn rendimiento(x: &Entrada) -> Vec<Metrica> {
    let c = x.corrida;
    let eventos = c.nucleo.metricas().eventos as f64;
    let plaza_dias = c.plazas.len().max(1) as f64 * x.e.dias as f64;
    let coste = x.control.map(|k| {
        let (de_mas, tiempo) = (eventos - k.nucleo.metricas().eventos as f64, c.pared.as_secs_f64() - k.pared.as_secs_f64());
        (de_mas, tiempo)
    });
    vec![
        metrica("pared_total", "Tiempo de pared total del escenario", "s", max(300.0), Some(x.pared_total.as_secs_f64()), "Fase 6 (rendimiento)", "La corrida con comercio, la de control sin él y las tres reducidas del determinismo."),
        metrica("pared_corrida", "Tiempo de pared de la corrida con comercio", "s", INFO, Some(c.pared.as_secs_f64()), "Fase 6 (rendimiento)", "Colocación de las plazas y simulación, sin las repeticiones."),
        metrica("eventos", "Eventos procesados por el núcleo", "eventos", INFO, Some(eventos), "Fase 6 (rendimiento)", "Vigentes, de todas las clases (el paso de minuto de cada plaza es la inmensa mayoría)."),
        metrica("eventos_por_plaza_dia", "Eventos por plaza y día", "eventos", INFO, Some(eventos / plaza_dias), "Reloj_Y_Eventos", "1.440 son los pasos de minuto de la economía; el resto es el comercio, la construcción y los pueblos."),
        metrica("eventos_de_comercio", "Eventos que añade el comercio a la corrida de control", "eventos", INFO, coste.map(|c| c.0), "Fase 6 (rendimiento)", "Eventos de la corrida con comercio menos los de la misma sin él: salidas, llegadas, peajes, caducidades, acarreos, órdenes y misiones."),
        metrica("us_por_evento_de_comercio", "Coste de un evento de comercio", "µs", INFO, coste.and_then(|c| (c.0 > 0.0).then(|| c.1 * 1e6 / c.0)), "Fase 6 (rendimiento)", "Diferencia de tiempo de pared entre la corrida con comercio y la de control entre la diferencia de eventos; incluye las órdenes del jugador y las misiones. Una cota alta del coste real de un evento de caravana."),
        metrica("pared_control", "Tiempo de pared de la corrida sin comercio", "s", INFO, x.control.map(|k| k.pared.as_secs_f64()), "Fase 6 (rendimiento)", "Solo funda las plazas y pide el ascenso."),
    ]
}

fn determinismo(x: &Entrada) -> Vec<Metrica> {
    vec![
        metrica("determinismo", "Determinismo (mismo escenario reducido, dos veces)", "1 = igual", (Some(1.0), Some(1.0)), x.determinismo.map(|d| d.0 as u8 as f64), "Arquitectura/13 §4 (núcleo determinista)", "Se repite un escenario reducido (20 plazas, 4 días) con la misma semilla y se comparan los `hash_estado()` finales del núcleo."),
        metrica("determinismo_normal", "Determinismo del reloj a pasos frente al acelerado", "1 = igual", (Some(1.0), Some(1.0)), x.determinismo.map(|d| d.1 as u8 as f64), "Reloj_Y_Eventos §1 (el reloj de pared no entra en el estado)", "El mismo escenario reducido avanzando el núcleo cada 7 minutos de mundo (modo normal) en lugar de saltar de orden en orden (modo acelerado)."),
    ]
}

// ───────────────────────────── Gráficos y mapa ─────────────────────────────

fn por_dia(instantes: impl Iterator<Item = i64>, dias: u32) -> Vec<(f64, f64)> {
    let mut cubos = vec![0.0; dias as usize + 1];
    instantes.for_each(|t| {
        let d = dia_de(t).min(dias as usize);
        cubos[d] += 1.0;
    });
    cubos.into_iter().enumerate().map(|(d, n)| (d as f64, n)).collect()
}

fn grafico_preparacion(x: &Entrada) -> Grafico {
    let m = &x.corrida.muestras;
    Grafico {
        id: "preparacion_dia".into(),
        titulo: "Plazas con Mercado activo y con flota".into(),
        tipo: TipoGrafico::Linea,
        eje_x: "día de mundo".into(),
        eje_y: "plazas".into(),
        series: vec![serie("Con Mercado activo", m.iter().map(|k| (k.dia as f64, k.con_mercado as f64)).collect()), serie("Con una caravana con buey", m.iter().map(|k| (k.dia as f64, k.con_flota as f64)).collect())],
    }
}

fn grafico_trueques(x: &Entrada) -> Grafico {
    let v = &x.corrida.vigia;
    Grafico {
        id: "trueques_dia".into(),
        titulo: "Trueques por día".into(),
        tipo: TipoGrafico::Linea,
        eje_x: "día de mundo".into(),
        eje_y: "trueques".into(),
        series: vec![serie("Propuestos", por_dia(v.trueques.values().map(|t| t.propuesto), x.e.dias)), serie("Cumplidos", por_dia(v.trueques.values().filter_map(|t| t.cumplido), x.e.dias))],
    }
}

fn histograma(id: &str, titulo: &str, eje_x: &str, valores: &[f64], ancho: f64) -> Grafico {
    let mut cubos: BTreeMap<i64, usize> = BTreeMap::new();
    valores.iter().for_each(|v| *cubos.entry((v / ancho) as i64).or_insert(0) += 1);
    let puntos = cubos.into_iter().map(|(k, n)| (r3(k as f64 * ancho), n as f64)).collect();
    Grafico { id: id.into(), titulo: titulo.into(), tipo: TipoGrafico::Histograma, eje_x: eje_x.into(), eje_y: "trueques o viajes".into(), series: vec![serie("Casos", puntos)] }
}

fn grafico_duracion_trueques(x: &Entrada) -> Grafico {
    let duran: Vec<f64> = x.corrida.vigia.trueques.values().filter_map(|t| Some((t.cumplido? - t.aceptado?) as f64 / MIN)).collect();
    histograma("dura_trueques", "Duración de un trueque (de la aceptación a cumplirse)", "minutos", &duran, 10.0)
}

fn grafico_viajes(x: &Entrada) -> Grafico {
    let viajes: Vec<f64> = x.corrida.vigia.viajes.iter().map(|y| y.dura_ms as f64 / MIN).collect();
    histograma("dura_viajes", "Duración de un viaje de ida", "minutos", &viajes, 1.0)
}

fn grafico_reputacion(x: &Entrada) -> Grafico {
    let punto = 1e6;
    let linea = |p: f64| -> Vec<(f64, f64)> { x.corrida.muestras.iter().filter_map(|m| percentil(&mut m.reputacion.iter().map(|&v| v as f64 / punto).collect::<Vec<_>>(), p).map(|v| (m.dia as f64, r3(v)))).collect() };
    Grafico { id: "reputacion_dia".into(), titulo: "Reputación de las Facciones".into(), tipo: TipoGrafico::Linea, eje_x: "día de mundo".into(), eje_y: "puntos".into(), series: vec![serie("Mínima", linea(0.0)), serie("Mediana", linea(0.5)), serie("Máxima", linea(1.0))] }
}

fn grafico_red(x: &Entrada) -> Grafico {
    let m = &x.corrida.muestras;
    Grafico {
        id: "red_dia".into(),
        titulo: "Red de caminos".into(),
        tipo: TipoGrafico::Linea,
        eje_x: "día de mundo".into(),
        eje_y: "aristas o rutas".into(),
        series: vec![serie("Aristas", m.iter().map(|k| (k.dia as f64, k.aristas as f64)).collect()), serie("Rutas guardadas", m.iter().map(|k| (k.dia as f64, k.rutas as f64)).collect())],
    }
}

fn grafico_peajes(x: &Entrada) -> Grafico {
    let v = &x.corrida.vigia;
    let mut pagan: Vec<(u32, i64)> = v.peajes_pagados.iter().map(|(&f, p)| (f, p.1)).collect();
    pagan.sort_by_key(|&(f, n)| (std::cmp::Reverse(n), f));
    pagan.truncate(12);
    let orden: Vec<u32> = pagan.iter().map(|&(f, _)| f).collect();
    let punto = |m: &BTreeMap<u32, (u64, i64)>| -> Vec<(f64, f64)> { orden.iter().enumerate().map(|(i, f)| (i as f64, r3(m.get(f).map_or(0.0, |p| p.1 as f64 / U)))).collect() };
    Grafico {
        id: "peajes_faccion".into(),
        titulo: format!("Peajes de las 12 Facciones que más pagan (Facciones {})", orden.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(", ")),
        tipo: TipoGrafico::Barras,
        eje_x: "Facción (en el orden del título)".into(),
        eje_y: "unidades".into(),
        series: vec![serie("Pagado", punto(&v.peajes_pagados)), serie("Cobrado", punto(&v.peajes_cobrados))],
    }
}

fn grafico_compras(x: &Entrada) -> Grafico {
    let v = &x.corrida.vigia;
    let por: Vec<(Recurso, f64)> = v.compras.iter().map(|(&i, k)| (asentamiento::recurso::TODOS[i], k.unidades as f64 / U)).collect();
    Grafico {
        id: "compras_bien".into(),
        titulo: "Compras a los mercenarios por bien".into(),
        tipo: TipoGrafico::Barras,
        eje_x: "bien".into(),
        eje_y: "unidades".into(),
        series: por.into_iter().map(|(r, n)| serie(r.nombre(), vec![(0.0, r3(n))])).collect(),
    }
}

fn grafico_pueblos(x: &Entrada) -> Grafico {
    let v = &x.corrida.vigia;
    let acumulado = |instantes: Vec<i64>| -> Vec<(f64, f64)> { (0..=x.e.dias).map(|d| (d as f64, instantes.iter().filter(|&&t| dia_de(t) <= d as usize).count() as f64)).collect() };
    Grafico {
        id: "pueblos_dia".into(),
        titulo: "Pueblos extractores (acumulado)".into(),
        tipo: TipoGrafico::Linea,
        eje_x: "día de mundo".into(),
        eje_y: "pueblos".into(),
        series: vec![serie("Marcados", acumulado(v.pueblos_marcados.values().copied().collect())), serie("Levantados", acumulado(v.pueblos_completados.values().copied().collect()))],
    }
}

fn grafico_llenado(x: &Entrada) -> Grafico {
    let m = &x.corrida.muestras;
    Grafico {
        id: "llenado_dia".into(),
        titulo: "Ocupación media del almacén de las plazas".into(),
        tipo: TipoGrafico::Linea,
        eje_x: "día de mundo".into(),
        eje_y: "% de la capacidad".into(),
        series: LLENADO.iter().enumerate().map(|(i, r)| serie(r.nombre(), m.iter().map(|k| (k.dia as f64, k.llenado[i] as f64)).collect())).collect(),
    }
}

fn grafico_flota(x: &Entrada) -> Grafico {
    let ultima = x.corrida.muestras.last();
    Grafico {
        id: "flota_estado".into(),
        titulo: "Caravanas comerciales por estado al final".into(),
        tipo: TipoGrafico::Barras,
        eje_x: "estado".into(),
        eje_y: "caravanas".into(),
        series: ESTADOS.iter().enumerate().map(|(i, (_, nombre))| serie(nombre, vec![(0.0, ultima.map_or(0.0, |m| m.caravanas[i] as f64))])).collect(),
    }
}

/// Polilínea de una ruta: sus aristas encadenadas (las contiguas comparten extremo).
fn polilinea(aristas: &[comercio::red_caminos::Arista]) -> Vec<Punto> {
    let p = |a: (i64, i64)| Punto { x: a.0 as f64, y: a.1 as f64 };
    let mut l: Vec<Punto> = Vec::new();
    for a in aristas {
        if l.last().is_none_or(|u| (u.x, u.y) != (a.0 .0 as f64, a.0 .1 as f64)) {
            l.push(p(a.0));
        }
        l.push(p(a.1));
    }
    l
}

fn mapa(x: &Entrada) -> Mapa {
    let (c, n) = (x.corrida, &x.corrida.nucleo);
    let u = |v: i64| v as f64 / 1000.0;
    let capa = |nombre: &str, color: &str, circulos, puntos, lineas| Capa { nombre: nombre.into(), color: color.into(), circulos, puntos, lineas };
    let punto = |p: base::Punto| Punto { x: u(p.x), y: u(p.y) };
    let r = &n.estado().reglas.economia.comercio.red;
    let pico = &c.pico_red;
    let arista = |a: &comercio::red_caminos::Arista| vec![Punto { x: a.0 .0 as f64, y: a.0 .1 as f64 }, Punto { x: a.1 .0 as f64, y: a.1 .1 as f64 }];
    let todas: Vec<_> = n.red_de_caminos().aristas().collect();
    let senderos: Vec<Vec<Punto>> = todas.iter().filter(|a| pico.get(**a).copied().unwrap_or(0) < r.escalon_camino).map(|a| arista(a)).collect();
    let caminos: Vec<Vec<Punto>> = todas.iter().filter(|a| (r.escalon_camino..r.escalon_calzada).contains(&pico.get(**a).copied().unwrap_or(0))).map(|a| arista(a)).collect();
    let calzadas: Vec<Vec<Punto>> = todas.iter().filter(|a| pico.get(**a).copied().unwrap_or(0) >= r.escalon_calzada).map(|a| arista(a)).collect();
    let vigentes: Vec<Vec<Punto>> = n.red_de_caminos().rutas().filter(|q| n.ahora() - q.ultimo_lanzamiento < r.caducidad).map(|q| polilinea(&q.aristas)).collect();
    let estado = n.estado();
    let pueblos: Vec<Punto> = estado.asentamientos.values().flat_map(|a| n.pueblos_de(a.id).iter().filter_map(|p| n.posicion_de_pueblo(p.id))).map(punto).collect();
    let zonas = |nivel2: bool| -> Vec<Circulo> { estado.asentamientos.values().filter(|a| (a.nivel() >= 2) == nivel2).map(|a| Circulo { x: u(a.posicion.x), y: u(a.posicion.y), r: u(a.radio) }).collect() };
    let plazas = |nivel2: bool| -> Vec<Punto> { estado.asentamientos.values().filter(|a| (a.nivel() >= 2) == nivel2).map(|a| punto(a.posicion)).collect() };
    let mut capas = vec![
        capa("Zonas de nivel 1", "#3b82f6", zonas(false), vec![], vec![]),
        capa("Zonas de nivel 2 o más", "#8b5cf6", zonas(true), vec![], vec![]),
        capa("Sendero (arista de la red que nunca llegó a camino)", "#f59e0b", vec![], vec![], senderos),
        capa("Camino (llegó a 2 rutas vigentes)", "#ea580c", vec![], vec![], caminos),
        capa("Calzada (llegó a 6 rutas vigentes)", "#dc2626", vec![], vec![], calzadas),
        capa("Rutas vigentes al final", "#ec4899", vec![], vec![], vigentes),
        capa("Plazas de nivel 1", "#e11d48", vec![], plazas(false), vec![]),
        capa("Plazas de nivel 2 o más", "#a21caf", vec![], plazas(true), vec![]),
        capa("Pueblos extractores", "#14b8a6", vec![], pueblos, vec![]),
        capa("Campamentos de mercenarios", "#16a34a", vec![], x.geo.campamentos.iter().map(|&p| punto(p)).collect(), vec![]),
    ];
    for (nombre, color) in [("Piedra (píxel)", fondo::PIEDRA), ("Parcela de bosque (píxel)", fondo::PARCELA), ("Bosque", fondo::BOSQUE), ("Río", fondo::RIO), ("Agua", fondo::AGUA), ("Llano", fondo::LLANO), ("Colina", fondo::COLINA), ("Montaña", fondo::MONTANA)] {
        capas.push(capa(nombre, &hex(color), vec![], vec![], vec![]));
    }
    Mapa { ancho: u(x.geo.ancho), alto: u(x.geo.alto), fondo: fondo::fondo(&x.c.mundo), capas }
}

/// Anomalías automáticas: solo violaciones de invariante y de determinismo.
fn anomalias(metricas: &[Metrica]) -> Vec<Anomalia> {
    ["inv_almacen", "inv_carga", "inv_trueque", "inv_sin_plan", "inv_cola", "inv_plaza_ociosa", "inv_minuto", "inv_registro", "inv_conservacion", "determinismo", "determinismo_normal"]
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
