//! Monta el `Informe` del sistema «militar y combate» con lo medido en la simulación de héroes que reclutan, salen, persiguen y combaten con números.

use crate::armar::{hex, hoy, max, metrica, parametro, r3, serie, INFO};
use crate::armarpob::{mediana, pct, percentil};
use crate::fondo;
use crate::geo::{Cargado, Geo};
use crate::simmil::{self as sm, Corrida, TIPOS};
use crate::EscenarioMilitar;
use asentamiento::Recurso;
use informe::{Anomalia, Capa, Causa, Grafico, Informe, Mapa, Metrica, Punto, TipoGrafico};
use militar::presencia::Ubicacion;
use std::time::Duration;

const MIN: f64 = 60_000.0;
const DIA: f64 = 1_440.0 * MIN;
/// 1 unidad = 1.000 milésimas.
const U: f64 = 1_000.0;

/// Todo lo medido, listo para montar.
pub struct Entrada<'a> {
    pub c: &'a Cargado,
    pub geo: &'a Geo,
    pub e: &'a EscenarioMilitar,
    pub corrida: &'a Corrida,
    /// (misma semilla ⇒ mismo hash, foto `BAN28` a mitad ⇒ mismo hash que sin ella) en el escenario reducido.
    pub determinismo: Option<(bool, bool)>,
    pub pared_total: Duration,
}

pub fn armar(x: &Entrada) -> Informe {
    let mut metricas = Vec::new();
    metricas.extend(escenario(x));
    metricas.extend(ejercitos(x));
    metricas.extend(combates(x));
    metricas.extend(bajas(x));
    metricas.extend(hambre(x));
    metricas.extend(caravanas(x));
    metricas.extend(pueblos_y_avanzadas(x));
    metricas.extend(bandidos(x));
    metricas.extend(conquistas(x));
    metricas.extend(persecuciones(x));
    metricas.extend(economia(x));
    metricas.extend(invariantes(x));
    metricas.extend(rendimiento(x));
    metricas.extend(determinismo(x));
    let anomalias = anomalias(&metricas);
    let c = x.corrida;
    let resumen = format!(
        "Se simularon {} días de mundo con {} plazas colocadas de {} Facciones ({} pedidas) y {} héroes bots sobre el núcleo real, sin servidores de batalla: reclutar, salir, comer, perseguir, convocar y combatir con números contra columnas, caravanas, bandidos, pueblos, avanzadas y plazas. Resumen automático; la lectura la añade quien revise los resultados.",
        x.e.dias,
        c.plazas.len(),
        c.facciones,
        x.e.plazas,
        c.bots.len()
    );
    Informe {
        sistema: "Militar y combate".into(),
        fecha: hoy(),
        mundo_hash: x.c.mundo.hash().into(),
        semilla: x.e.semilla,
        dias_simulados: x.e.dias as f64,
        resumen,
        parametros: parametros(x),
        metricas,
        series: vec![grafico_combates(x), grafico_ejercito(x), grafico_politica(x), grafico_bajas(x), grafico_rechazos(x)],
        mapa: Some(mapa(x)),
        anomalias,
        decisiones: vec![],
    }
}

fn parametros(x: &Entrada) -> Vec<informe::Parametro> {
    let c = x.corrida;
    let hash: String = sm::huella(c).iter().map(|b| format!("{b:02x}")).collect();
    let mr = &c.nucleo.estado().reglas.economia.militar;
    vec![
        parametro("Plazas pedidas / colocadas / Facciones / héroes", format!("{} / {} / {} / {}", x.e.plazas, c.plazas.len(), c.facciones, c.bots.len())),
        parametro("Días de mundo", x.e.dias),
        parametro("Semilla", x.e.semilla),
        parametro("Hash del mundo", x.c.mundo.hash()),
        parametro("Huella del estado final (hash_estado)", hash),
        parametro("Escenario", format!("{} plazas por Facción fundadas en los primeros minutos (la primera junto a un campamento de mercenarios, las demás junto a las suyas), Facciones de nivel {} por parejas aliadas; {} de {} belicosas (el {} % por semilla); la Era I adoptada por administración ({})", sm::PLAZAS_POR_FACCION, sm::NIVEL_FACCION, c.belicosas.len(), c.facciones, sm::BELICOSA_PCT, sm::TECNOLOGIAS.join(", "))),
        parametro("Héroes", format!("cuatro por plaza, residentes y ciudadanos por administración: Guardián, Capitán, Cazador e Intendente (la cuenta de cada papel la sortea la semilla); el {} % de los Cazadores son temerarios (salen solos, una vez al día, contra el campamento de bandidos más cercano). Liderazgo base {}", sm::TEMERARIO_PCT, mr.liderazgo.base)),
        parametro("Bots: turno", format!("cada {} min de mundo; recluta o repone una escuadra por hora (la mejor tropa con tecnología y edificio, sin pasar del Liderazgo con las nuevas), loadout activo con lo del campamento; vuelven a casa por debajo del {} % de víveres (antes pasan a víveres el trigo del carro), tras {} objetivos o heridos; {} min de descanso entre salidas", sm::TURNO_MIN, sm::VOLVER_VIVERES_PCT, sm::OBJETIVOS_POR_SALIDA, sm::DESCANSO_MIN)),
        parametro("Bots: campaña", format!("el Capitán convoca el {} % de las veces (parte a los 10 min) y si no sale como ejército y los compañeros libres se le unen en la puerta con probabilidad del {} %; lleva {} u de trigo en el carro; busca a {} u o menos un ejército enemigo a la vista, una plaza (tras inspeccionarla, si su poder es al menos el {} % de la defensa que le supone: la inspección dice quién está dentro y el bot sabe con qué defiende cada uno, con la cohesión, la ventaja del defensor y la muralla), un pueblo con al menos {} u que saquear, una avanzada o bandidos a {} u de casa (cada {} h), en un orden que sortea la semilla", sm::CONVOCA_PCT, sm::SE_UNE_PCT, sm::TRIGO_DE_CAMPANA / 1000, sm::RADIO_CAMPANA / 1000, sm::MARGEN_ASEDIO_PCT, sm::SAQUEO_MINIMO / 1000, sm::RADIO_BANDIDOS / 1000, sm::BANDIDOS_CADA_H)),
        parametro("Bots: caza e intendencia", format!("el Cazador explora a {}-{} u de casa, persigue lo que ve (columnas personales y caravanas enemigas) y ataca bandidos con probabilidad del 30 %; el Intendente lleva al pueblo y a la atalaya los materiales del almacén de su plaza y cada {} h manda una caravana ({} u de trigo) a la plaza hermana, alternando escolta cedida y caravana adjunta a su ejército", sm::RADIO_EXPLORACION / 2000, sm::RADIO_EXPLORACION / 1000, sm::CARAVANA_CADA_H, sm::CARGA_DE_CARAVANA / 1000)),
        parametro("Papeles", {
            let mut por: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
            c.bots.iter().for_each(|b| *por.entry(b.rol.nombre()).or_insert(0) += 1);
            format!("{}; temerarios {}", por.iter().map(|(r, n)| format!("{r} {n}")).collect::<Vec<_>>().join(", "), c.bots.iter().filter(|b| b.temerario).count())
        }),
        parametro("Foto ida y vuelta a mitad de la corrida", c.foto.map_or("no se hizo".into(), |(a, d)| format!("hash antes {}…, reconstruido {}…", a[..6].iter().map(|b| format!("{b:02x}")).collect::<String>(), d[..6].iter().map(|b| format!("{b:02x}")).collect::<String>()))),
        parametro("Andamiaje de administración", "nivel de Facción, tecnologías de la Era I, alianzas por parejas y, de cada héroe, la ciudadanía, la residencia y la colocación en su plaza; la columna de fundación entra en su plaza. Nada más: oro, trigo, materiales y tropa salen de las plazas"),
        parametro("Batallas", "sin servidores de batalla (`Reglas::batallas_en_unity` falsa): todo combate se resuelve con números, también el acoso de los bandidos"),
    ]
}

// ───────────────────────────── Ayudas ─────────────────────────────

fn n_hecho(x: &Entrada, codigo: &str) -> f64 {
    x.corrida.medidas.hechos.get(codigo).copied().unwrap_or(0) as f64
}

fn aceptadas(x: &Entrada, etiqueta: &str) -> f64 {
    x.corrida.ordenes.get(etiqueta).map_or(0, |c| c.aceptadas) as f64
}

fn rechazos_de(x: &Entrada, etiqueta: &str) -> String {
    let Some(c) = x.corrida.ordenes.get(etiqueta) else { return "ninguno".into() };
    let mut v: Vec<(&&str, &u64)> = c.rechazos.iter().collect();
    v.sort_by_key(|(k, n)| (std::cmp::Reverse(**n), **k));
    if v.is_empty() {
        return "ninguno".into();
    }
    v.iter().take(5).map(|(k, n)| format!("{k} ({n})")).collect::<Vec<_>>().join(", ")
}

fn de_tipo<'a>(x: &'a Entrada, tipo: &'a str) -> impl Iterator<Item = &'a sm::Combate> {
    x.corrida.medidas.combates.iter().filter(move |c| c.tipo == tipo)
}

fn dia_de(t: i64) -> usize {
    (t as f64 / DIA) as usize
}

// ───────────────────────────── Métricas ─────────────────────────────

fn escenario(x: &Entrada) -> Vec<Metrica> {
    let c = x.corrida;
    let ultima = c.muestras.last().cloned().unwrap_or_default();
    vec![
        metrica("colocadas", "Plazas colocadas", "plazas", INFO, Some(c.plazas.len() as f64), "Doc 1.2", &format!("De las {} pedidas.", x.e.plazas)),
        metrica("heroes", "Héroes bots", "héroes", INFO, Some(c.bots.len() as f64), "Heroe_Y_Tropa §1", "Cuatro por plaza colocada."),
        metrica("plazas_vivas", "Plazas en pie al final", "plazas", INFO, Some(ultima.plazas as f64), "Combate §8 y Economia_Construccion §5", "Las que no cayeron en ruinas."),
        metrica("facciones_vivas", "Facciones con alguna plaza al final", "Facciones", INFO, Some(ultima.facciones_vivas as f64), "Combate §10", "Las que perdieron todas sus plazas quedan derrotadas."),
        metrica("nivel2", "Plazas de nivel 2 o más al final", "plazas", INFO, Some(c.nucleo.estado().asentamientos.values().filter(|a| a.nivel() >= 2).count() as f64), "Economia_Construccion §11", "El nivel 2 abre pueblos extractores, avanzadas, la empalizada y las tropas de Barracón y Galería de tiro."),
        metrica("con_barracon", "Plazas con Barracón activo al final", "plazas", INFO, Some(c.nucleo.estado().asentamientos.values().filter(|a| a.economia.edificios.iter().any(|e| e.tipo == asentamiento::catalogo::Tipo::Barracon && e.activo())).count() as f64), "Heroe_Y_Tropa §10 (edificio de la tropa)", &format!("Con Galería de tiro: {}. Construir: {}.", c.nucleo.estado().asentamientos.values().filter(|a| a.economia.edificios.iter().any(|e| e.tipo == asentamiento::catalogo::Tipo::GaleriaDeTiro && e.activo())).count(), rechazos_de(x, "AnadirEdificio"))),
        metrica("heroes_exiliados", "Héroes que residen en un campamento al final", "héroes", INFO, Some(ultima.heroes_exiliados as f64), "Combate §9", "Desalojados de una plaza conquistada sin otra plaza de su Facción: pasan al campamento de mercenarios más cercano."),
    ]
}

fn ejercitos(x: &Entrada) -> Vec<Metrica> {
    let (c, m) = (x.corrida, &x.corrida.medidas);
    let reclutados: i64 = m.reclutados_por_tropa.values().sum();
    let ultima = c.muestras.last().cloned().unwrap_or_default();
    let salidas = aceptadas(x, "MovilizarEjercito") + aceptadas(x, "SalirAlMundo") + aceptadas(x, "PartirConvocatoria");
    vec![
        metrica("reclutamientos", "Órdenes de reclutar aceptadas", "órdenes", INFO, Some(aceptadas(x, "ReclutarTropa")), "Heroe_Y_Tropa §10", &format!("Escuadras nuevas y reposiciones. Rechazos más frecuentes: {}.", rechazos_de(x, "ReclutarTropa"))),
        metrica("reclutados", "Soldados reclutados", "soldados", INFO, Some(reclutados as f64), "Heroe_Y_Tropa §10-§11", &format!("Por tropa: {}.", m.reclutados_por_tropa.iter().map(|(t, n)| format!("{t:?} {n}")).collect::<Vec<_>>().join(", "))),
        metrica("soldados_final", "Soldados en pie al final", "soldados", INFO, Some(ultima.soldados as f64), "Heroe_Y_Tropa §2", &format!("{} en columnas y {} en guarniciones de plaza.", ultima.en_columna, ultima.en_guarnicion)),
        metrica("salidas", "Salidas al mundo", "salidas", INFO, Some(salidas), "Movimiento_Ejercitos §3", &format!("`MovilizarEjercito` {}, `SalirAlMundo` {}, `PartirConvocatoria` {}. Rechazos de movilizar: {}.", aceptadas(x, "MovilizarEjercito"), aceptadas(x, "SalirAlMundo"), aceptadas(x, "PartirConvocatoria"), rechazos_de(x, "MovilizarEjercito"))),
        metrica("guarniciones", "Escuadras puestas en la guarnición", "órdenes", INFO, Some(aceptadas(x, "AsignarGuarnicion")), "Heroe_Y_Tropa §7", &format!("La leva del Guardián, mientras quepa en el cupo. Rechazos: {}.", rechazos_de(x, "AsignarGuarnicion"))),
        metrica("convocatorias", "Convocatorias abiertas", "convocatorias", INFO, Some(n_hecho(x, "convocatoria.convocada")), "Presencia_Y_Convocatoria §9", &format!("Partieron {}, canceladas {}; héroes unidos {}.", n_hecho(x, "convocatoria.partio"), n_hecho(x, "convocatoria.cancelada"), n_hecho(x, "convocatoria.unido"))),
        metrica("uniones", "Uniones a un ejército en marcha", "uniones", INFO, Some(n_hecho(x, "ejercito.refuerzo") + n_hecho(x, "columna.union_en_campo")), "Movimiento_Ejercitos §10", &format!("Desde la plaza (`UnirseAEjercito`) {} y en campo (`UnirseEnCampo`) {}.", n_hecho(x, "ejercito.refuerzo"), n_hecho(x, "columna.union_en_campo"))),
        metrica("guerras_declaradas", "Guerras declaradas al atacar", "guerras", INFO, Some(n_hecho(x, "faccion.guerra_declarada")), "Combate §2", &format!("Pares de Facciones en guerra al final: {}.", ultima.guerras)),
    ]
}

fn combates(x: &Entrada) -> Vec<Metrica> {
    let m = &x.corrida.medidas;
    let mut v = vec![metrica("combates", "Combates resueltos con números", "combates", INFO, Some(m.combates.len() as f64), "Combate §4-§7", "Los que ordenan los bots y los acosos de los bandidos a las caravanas.")];
    for tipo in TIPOS {
        let todos: Vec<&sm::Combate> = de_tipo(x, tipo).collect();
        let ganan = todos.iter().filter(|c| c.gana_atacante).count();
        let ratios: Vec<f64> = todos.iter().filter(|c| c.poder_a > 0 && c.poder_d > 0).map(|c| c.poder_a.min(c.poder_d) as f64 / c.poder_a.max(c.poder_d) as f64).collect();
        let fuerza: Vec<f64> = todos.iter().filter(|c| c.poder_d > 0).map(|c| c.poder_a as f64 / c.poder_d as f64).collect();
        v.push(metrica(&format!("combates_{tipo}"), &format!("Combates: {tipo}"), "combates", INFO, Some(todos.len() as f64), "Combate §4-§7", &format!("Gana el atacante el {}; ratio de poder (menor entre mayor) mediano {}, poder atacante entre defensor mediano {} (percentil 10: {}, 90: {}).", pct(ganan, todos.len()).map_or("—".into(), |p| format!("{:.1} %", p)), mediana(&ratios).map_or("—".into(), |r| format!("{:.2}", r)), mediana(&fuerza).map_or("—".into(), |r| format!("{:.2}", r)), percentil(&mut fuerza.clone(), 0.1).map_or("—".into(), |r| format!("{:.2}", r)), percentil(&mut fuerza.clone(), 0.9).map_or("—".into(), |r| format!("{:.2}", r)))));
        v.push(metrica(&format!("gana_atacante_{tipo}"), &format!("Gana el atacante: {tipo}"), "%", INFO, pct(ganan, todos.len()), "Combate §4", "En el acoso, el atacante es el campamento de bandidos."));
    }
    v
}

fn bajas(x: &Entrada) -> Vec<Metrica> {
    let m = &x.corrida.medidas;
    let total: i64 = m.bajas_por_tropa.values().sum();
    let mut v = vec![metrica("bajas", "Soldados perdidos en combate", "soldados", INFO, Some(total as f64), "Combate §4 (bajas permanentes)", "Bajas de las órdenes de ataque y lo que las reglas dejan a 0 en la misma orden (la escolta de una caravana que cae, el campamento de los residentes desalojados, la guarnición de jugadores de una avanzada perdida); sin las del acoso de los bandidos.")];
    for (t, n) in &m.bajas_por_tropa {
        v.push(metrica(&format!("bajas_{t:?}").to_lowercase(), &format!("Bajas: {t:?}"), "soldados", INFO, Some(*n as f64), "Combate §4", "Por tropa; las guarniciones de serie son los mineros de los pueblos y la milicia de las avanzadas."));
    }
    v.push(metrica("heridos", "Héroes heridos en combate", "heridas", INFO, Some(m.heridos as f64), "Combate §11", &format!("Cada herida dura `heridoMinutos`; un herido no combate. Tiempo herido en total: {} min. Ataques rechazados por estar herido: {}.", r3(m.ms_herido as f64 / MIN), x.corrida.ordenes.get("Atacar").and_then(|c| c.rechazos.get(nucleo::rechazos::HEROE_HERIDO)).copied().unwrap_or(0))));
    v
}

fn hambre(x: &Entrada) -> Vec<Metrica> {
    let m = &x.corrida.medidas;
    let desertores: i64 = m.desertores_por_tropa.values().sum();
    vec![
        metrica("sin_viveres", "Columnas que se quedaron sin víveres", "veces", INFO, Some(n_hecho(x, "ejercito.sin_viveres")), "Movimiento_Ejercitos §6.3", "Evento «víveres a 0» de cada columna."),
        metrica("desertores", "Soldados desertores", "soldados", INFO, Some(desertores as f64), "Movimiento_Ejercitos §6.3", &format!("En columna {} y en guarnición {}; por tropa: {}.", m.desertores_columna, m.desertores_guarnicion, m.desertores_por_tropa.iter().map(|(t, n)| format!("{t:?} {n}")).collect::<Vec<_>>().join(", "))),
        metrica("repostar", "Veces que una columna repostó en su plaza", "órdenes", INFO, Some(aceptadas(x, "Repostar")), "Movimiento_Ejercitos §8", &format!("Rechazos: {}.", rechazos_de(x, "Repostar"))),
        metrica("pasar_a_viveres", "Trigo del carro pasado a víveres", "unidades", INFO, Some(m.trigo_a_viveres as f64 / U), "Movimiento_Ejercitos §6.1", &format!("En {} órdenes `PasarAViveres`.", aceptadas(x, "PasarAViveres"))),
    ]
}

fn caravanas(x: &Entrada) -> Vec<Metrica> {
    let m = &x.corrida.medidas;
    let por = |aguante: bool| m.asaltos.iter().filter(|a| a.0 == aguante).count();
    let perdidas = |aguante: bool| m.asaltos.iter().filter(|a| a.0 == aguante && a.1).count();
    let carros: i64 = m.asaltos.iter().map(|a| a.2).sum();
    let acosos: Vec<&sm::Combate> = de_tipo(x, "acoso").collect();
    vec![
        metrica("caravanas_lanzadas", "Caravanas que salieron de su plaza", "salidas", INFO, Some(n_hecho(x, "comercio.caravana_sale")), "Comercio_Caravanas §6", &format!("Escoltadas por una escuadra cedida (`AsignarEscolta` {}) o adjuntas a un ejército (`AdjuntarCaravana` {}).", aceptadas(x, "AsignarEscolta"), aceptadas(x, "AdjuntarCaravana"))),
        metrica("asaltos_disputa", "Caravanas asaltadas con éxito: disputa (A)", "asaltos", INFO, Some(por(false) as f64), "Combate §7.1", &format!("Perdidas enteras {}; resistieron {}.", perdidas(false), m.asaltos_resistidos[0])),
        metrica("asaltos_aguante", "Caravanas asaltadas con éxito: aguante (B)", "asaltos", INFO, Some(por(true) as f64), "Combate §7.1", &format!("Perdidas enteras {} (debe ser 0: un aguante nunca deja la caravana sin carros); resistieron {}.", perdidas(true), m.asaltos_resistidos[1])),
        metrica("carros_perdidos", "Carros perdidos en asaltos", "carros", INFO, Some(carros as f64), "Combate §7.1", &format!("Y {} carros de caravanas adjuntas a columnas derrotadas (Combate §5).", m.carros_perdidos_adjuntas)),
        metrica("acosos", "Acosos de bandidos a caravanas", "acosos", INFO, Some(acosos.len() as f64), "Generacion_Del_Mundo §13", &format!("Las caravanas perdidas por los bandidos: {}; escaparon {}.", acosos.iter().filter(|c| c.gana_atacante).count(), acosos.iter().filter(|c| !c.gana_atacante).count())),
        metrica("caravanas_perdidas", "Caravanas perdidas por cualquier causa", "caravanas", INFO, Some(n_hecho(x, "comercio.caravana_perdida")), "Comercio_Caravanas §10", "Asaltos que se llevan todos los carros, acosos de bandidos, adjuntas de una columna derrotada y conquistas o ruinas."),
    ]
}

fn pueblos_y_avanzadas(x: &Entrada) -> Vec<Metrica> {
    let (c, m) = (x.corrida, &x.corrida.medidas);
    let e = c.nucleo.estado();
    let activos = e.asentamientos.values().flat_map(|a| a.economia.pueblos.iter()).filter(|p| p.activo()).count();
    let operativas = e.asentamientos.values().flat_map(|a| a.economia.avanzadas.iter()).filter(|a| a.operativa()).count();
    let repone: Vec<f64> = m.reposiciones_pueblo.iter().map(|&t| t as f64 / MIN).collect();
    vec![
        metrica("pueblos_activos", "Pueblos extractores activos al final", "pueblos", INFO, Some(activos as f64), "Economia_Construccion §14", &format!("Marcados {}, materiales entregados {} veces.", aceptadas(x, "MarcarPueblo"), aceptadas(x, "EntregarMateriales"))),
        metrica("pueblos_saqueados", "Pueblos saqueados", "saqueos", INFO, Some(n_hecho(x, "combate.pueblo_saqueado")), "Combate §7.3", &format!("Resistieron {}. Saqueado: {} u (lo que había por encima de la mitad de su almacén).", n_hecho(x, "combate.pueblo_resiste"), r3(m.saqueado_de_pueblos as f64 / U))),
        metrica("guarnicion_repuesta", "Guarniciones de pueblo repuestas enteras", "reposiciones", INFO, Some(m.reposiciones_pueblo.len() as f64), "Combate §7.3 (un minero cada 12 min)", &format!("Tiempo mediano de la primera baja a la guarnición completa: {} min.", mediana(&repone).map_or("—".into(), |v| format!("{v:.0}")))),
        metrica("avanzadas_operativas", "Avanzadas operativas al final", "avanzadas", INFO, Some(operativas as f64), "Economia_Construccion §14.1", &format!("Colocadas {} (rechazos: {}).", aceptadas(x, "ColocarAvanzada"), rechazos_de(x, "ColocarAvanzada"))),
        metrica("avanzadas_atacadas", "Ataques a avanzadas que ganó el atacante", "ataques", INFO, Some(n_hecho(x, "combate.avanzada_destruida") + n_hecho(x, "combate.avanzada_en_ruinas") + n_hecho(x, "combate.avanzada_saqueada")), "Combate §7.4", &format!("Resistieron {}; guarniciones de serie repuestas enteras (soldado a soldado, 6 h desde 0): {}.", n_hecho(x, "combate.avanzada_resiste"), m.reposiciones_avanzada.len())),
    ]
}

fn bandidos(x: &Entrada) -> Vec<Metrica> {
    let (c, m) = (x.corrida, &x.corrida.medidas);
    let oro: Vec<f64> = m.oro_por_heroe.iter().map(|&o| o as f64 / U).collect();
    let solos = c.bots.iter().filter(|b| b.temerario).count();
    vec![
        metrica("bandidos_destruidos", "Campamentos de bandidos destruidos", "campamentos", INFO, Some(n_hecho(x, "combate.campamento_destruido")), "Combate §7.2", &format!("Ataques fallidos {}; campamentos que aparecieron (al empezar, al fundar y al reaparecer) {}; quedan {} al final.", n_hecho(x, "combate.ataque_campamento_fallido"), n_hecho(x, "bandidos.campamento_aparece"), c.muestras.last().map_or(0, |u| u.bandidos))),
        metrica("oro_por_heroe", "Oro de bandidos por héroe que combatió (mediana)", "oro", INFO, mediana(&oro), "Combate §7.2 (`botinDeBandidos`)", &format!("Máximo {}; total {}.", oro.iter().copied().fold(0.0, f64::max), r3(oro.iter().sum::<f64>()))),
        metrica("ataques_solos", "Ataques de un héroe sin soldados a un campamento de bandidos", "ataques", INFO, Some(m.ataques_solos as f64), "Combate §7.2 (un héroe solo no destruye ningún campamento)", &format!("Los de los {solos} Cazadores temerarios, que salen solos una vez al día contra el campamento de casa; ninguno puede destruirlo (invariante `inv_heroe_solo`).")),
    ]
}

fn conquistas(x: &Entrada) -> Vec<Metrica> {
    let (c, m) = (x.corrida, &x.corrida.medidas);
    let e = c.nucleo.estado();
    let completas = e.asentamientos.values().filter(|a| asentamiento::recintos::muralla_completa(&a.economia.recintos)).count();
    let reparadas = m.murallas_danadas.values().filter(|(_, r)| r.is_some()).count();
    let tiempos: Vec<f64> = m.murallas_danadas.values().filter_map(|(d, r)| Some((r.as_ref()? - d) as f64 / MIN)).collect();
    let primer_dia = m.conquistas.iter().filter(|&&(t, p, ..)| e.asentamientos.get(&p).is_some_and(|a| t < a.fundado_en + 24 * 60 * base::MS_POR_MINUTO)).count();
    let rebotan = x.corrida.ordenes.get("Atacar").and_then(|c| c.rechazos.get("combate.plaza_protegida")).copied().unwrap_or(0);
    vec![
        metrica("asedios_abiertos", "Asedios abiertos (con su convocatoria de 30 s)", "asedios", INFO, Some(m.asedios_abiertos as f64), "Combate §6, Batalla_Unity_Ciclo §4 (M8)", &format!("Columnas que se unieron a la convocatoria {}; conquistas con más de una columna {}.", m.uniones_a_asedios, m.conquistas_con_convocatoria)),
        metrica("conquistas_primer_dia", "Plazas conquistadas en su primer día", "conquistas", INFO, Some(primer_dia as f64), "Doc 1.2, Fundacion_Y_Refundacion §5 (M5)", &format!("Ataques a plazas que rebotaron por la protección (al fundar o tras una conquista): {rebotan}.")),
        metrica("conquistas", "Plazas conquistadas", "conquistas", INFO, Some(m.conquistas.len() as f64), "Combate §6 y §8", &format!("Plazas distintas {}; asedios resistidos {}; asedios descartados tras inspeccionar {}; Facciones derrotadas {}.", m.conquistas.iter().map(|c| c.1).collect::<std::collections::BTreeSet<_>>().len(), n_hecho(x, "combate.asedio_resistido"), m.asedios_descartados, n_hecho(x, "faccion.derrotada"))),
        metrica("ruinas", "Plazas en ruinas", "plazas", INFO, Some(m.ruinas.len() as f64), "Economia_Construccion §5", "Por falta de mantenimiento; arrastran su comercio, sus pueblos y sus avanzadas."),
        metrica("murallas_comprometidas", "Empalizadas comprometidas", "recintos", INFO, Some(aceptadas(x, "ComprometerRecinto")), "Murallas §4", &format!("Completas al final en {completas} plazas. Rechazos: {}.", rechazos_de(x, "ComprometerRecinto"))),
        metrica("murallas_danadas", "Murallas dañadas por una conquista", "plazas", INFO, Some(m.murallas_danadas.len() as f64), "Murallas §9.6", &format!("Reparadas {reparadas}; tiempo mediano de reparación {} min.", mediana(&tiempos).map_or("—".into(), |v| format!("{v:.0}")))),
    ]
}

fn persecuciones(x: &Entrada) -> Vec<Metrica> {
    let m = &x.corrida.medidas;
    let iniciadas = n_hecho(x, "columna.persecucion_iniciada");
    vec![
        metrica("persecuciones", "Persecuciones iniciadas", "persecuciones", INFO, Some(iniciadas), "Movimiento_Ejercitos §7", &format!("Rechazos de `Perseguir`: {}.", rechazos_de(x, "Perseguir"))),
        metrica("persecuciones_cumplidas", "Presas alcanzadas", "persecuciones", INFO, Some(n_hecho(x, "columna.presa_alcanzada")), "Movimiento_Ejercitos §7", &format!("De las iniciadas, el {}. Al llegar al punto de intercepción la presa ya no estaba a tiro {} veces.", pct(n_hecho(x, "columna.presa_alcanzada") as usize, iniciadas as usize).map_or("—".into(), |p| format!("{p:.1} %")), m.persecuciones_sin_alcance)),
        metrica("persecuciones_canceladas", "Persecuciones canceladas", "persecuciones", INFO, Some(n_hecho(x, "columna.presa_perdida") + n_hecho(x, "columna.presa_a_cubierto") + n_hecho(x, "columna.persecucion_abandonada")), "Movimiento_Ejercitos §7", &format!("Presa perdida de vista {}, a cubierto {}, abandonadas {}.", n_hecho(x, "columna.presa_perdida"), n_hecho(x, "columna.presa_a_cubierto"), n_hecho(x, "columna.persecucion_abandonada"))),
    ]
}

fn economia(x: &Entrada) -> Vec<Metrica> {
    let m = &x.corrida.medidas;
    let gasto = |etiquetas: &[&str], r: Recurso| m.gasto.iter().filter(|((e, rc), _)| etiquetas.contains(e) && *rc == r).map(|(_, n)| n).sum::<i64>() as f64 / U;
    let salidas = ["MovilizarEjercito", "SalirAlMundo", "ConvocarEjercito", "UnirseAEjercito", "Repostar"];
    let equipo: Vec<String> = m.gasto.iter().filter(|((e, r), _)| *e == "ReclutarTropa" && *r != Recurso::Oro).map(|((_, r), n)| format!("{} {}", r.nombre(), r3(*n as f64 / U))).collect();
    vec![
        metrica("oro_reclutar", "Oro gastado en reclutar", "oro", INFO, Some(gasto(&["ReclutarTropa"], Recurso::Oro)), "Heroe_Y_Tropa §11 (oro por escalón)", &format!("Equipo gastado: {}.", if equipo.is_empty() { "nada".into() } else { equipo.join(", ") })),
        metrica("trigo_salidas", "Trigo sacado de las plazas al salir y al repostar", "unidades", INFO, Some(gasto(&salidas, Recurso::Trigo)), "Movimiento_Ejercitos §6.1 y §8", "Víveres al salir, el trigo del carro de campaña y lo que se repone en la puerta."),
        metrica("materiales_obras", "Madera y piedra sacadas para las obras de mapa", "unidades", INFO, Some(gasto(&["SalirAlMundo"], Recurso::Madera) + gasto(&["SalirAlMundo"], Recurso::Piedra)), "Economia_Construccion §14 y §14.1", "Lo que el Intendente lleva al pueblo y a la atalaya."),
        metrica("chatarra", "Chatarra generada por las bajas", "unidades", INFO, Some(m.chatarra_generada as f64 / U), "Combate §4 (CHATARRA)", "La recoge quien gana: al carro (hasta su capacidad) o al almacén de la plaza o de la madre que resiste."),
    ]
}

fn invariantes(x: &Entrada) -> Vec<Metrica> {
    let c = x.corrida;
    let (v, k, mv) = (&c.violaciones, &c.combate, &c.militares);
    let uno = |id: &str, nombre: &str, valor: u64, origen: &str, nota: &str| metrica(id, nombre, "violaciones", max(0.0), Some(valor as f64), origen, nota);
    vec![
        uno("inv_bajas", "Bajas de un bando que no son los soldados que perdieron sus escuadras", k.bajas, "Combate §4", "En cada orden de ataque: lo que pierden las escuadras del atacante es `bajasAtacante` y lo que pierde el resto del mundo es `bajasDefensor` (más `bajasEscolta`), salvo lo que la regla deja a 0 en la misma orden."),
        uno("inv_ajenas", "Escuadras ajenas que cambian en un combate entre columnas", k.ajenas, "Combate §5", "Solo cambian las escuadras de las dos columnas."),
        uno("inv_experiencia", "Experiencia o nivel que bajan", k.experiencia + mv.experiencia, "Combate §4 (experiencia de escuadra)", "En cada orden de ataque y en cada observación, de cada escuadra y de cada héroe."),
        uno("inv_heridos", "Heridos que combaten o heridas que no duran lo que dicen las reglas", k.heridos + mv.herido, "Combate §11", "Un herido no pierde soldados de su columna en un combate; quien queda herido lo está `heridoMinutos` exactos y a quien ya lo estaba no se le alarga; nadie está herido más allá de ese plazo."),
        uno("inv_hambre", "Columnas cuyo hambre no sigue la regla", mv.hambre, "Movimiento_Ejercitos §6.3", "Toda columna sin víveres y con soldados tiene su cita de deserción, ninguna con víveres la tiene y ninguna deserta con víveres."),
        uno("inv_conservacion", "Recursos creados por un combate", k.conservacion, "Combate §5-§7 (botín, chatarra, bandidos)", "En cada orden de ataque ningún recurso crece salvo la chatarra (hasta lo que dicen los hechos) y el oro de los bandidos."),
        uno("inv_carros", "Caravanas atacadas cuyos carros o carga no siguen la regla", k.carros, "Combate §7.1", "Disputa: la mitad de los carros, hacia arriba, y sin carros desaparece; aguante: igual pero nunca el último y se salva el 30 % de la carga; si resiste, no pierde nada."),
        uno("inv_botin", "Botín que no es lo que entró en el carro o que no cabe", k.botin, "Combate §5 (cargarBotin)", "El carro del atacante crece en el botín que dice el hecho y nunca pasa de su capacidad."),
        uno("inv_heroe_solo", "Campamentos destruidos por un héroe solo", k.heroe_solo, "Combate §7.2", "Decisión del autor (2026-10-23): un héroe sin soldados no destruye ningún campamento."),
        uno("inv_cascada", "Conquistas que no siguen las cascadas", k.cascada, "Combate §8-§9, Comercio_Caravanas §10, Murallas §9.6", "La plaza pasa al vencedor, protegida y ocupada, sin nadie dentro ni residiendo, sin caravanas aparcadas ni trueques vivos, y la muralla completa pierde su fracción de celdas."),
        uno("inv_ruina", "Plazas en ruinas que dejan algo atrás", mv.ruina, "Comercio_Caravanas §10 y Combate §9", "Sin flota, sin residentes y sin nadie dentro."),
        uno("inv_reposicion", "Guarniciones de serie repuestas a deshora", mv.reposicion, "Combate §7.3 y §7.4", "El pueblo se completa a un minero cada 12 min desde la primera baja (con todas las bajas del episodio); la avanzada, a un soldado cada 6 h / soldados de su guarnición completa, también desde la primera baja."),
        uno("inv_batalla", "Batallas o cosas apartadas fuera de la convocatoria de un asedio", mv.batalla, "Batalla_Unity_Ciclo §2 y §4", "Sin servidores de batalla todo se resuelve con números; solo un asedio abre una batalla, que dura su convocatoria de 30 s (M8): ninguna otra, ninguna activa pasada la convocatoria y nada apartado fuera de ellas."),
        uno("inv_foto", "Foto ida y vuelta a mitad de la corrida con otro hash", mv.foto, "Reloj_Y_Eventos (foto `BAN28`)", "A mitad de la corrida se serializa el núcleo, se reconstruye por la persistencia y se sigue con el reconstruido."),
        uno("inv_almacen", "Almacenes fuera de [0, capacidad]", v.almacen, "Economia_Construccion §15", "Cada 6 horas de mundo."),
        uno("inv_carga", "Carros y caravanas por encima de su capacidad", v.carga, "Comercio_Caravanas §5", "Cada 6 horas de mundo."),
        uno("inv_fracciones", "Cantidades almacenadas que no son unidades enteras", v.fraccion, "Economia_Construccion §4 (economía entera, C3b)", "Almacenes, carros, caravanas, almacenes personales, existencias, trueques y órdenes: cada 6 horas de mundo."),
        uno("inv_cola", "Eventos de comercio vigentes que no corresponden al estado", v.cola + v.sin_plan + v.ociosa, "Reloj_Y_Eventos §2", "Llegadas, peajes y preparaciones de caravanas, y caravanas en camino sin su llegada."),
        uno("inv_minuto", "Plazas sin su minuto o con uno de más", v.minuto + v.fines, "Economia_Construccion §3", "Y obras en curso sin su cita de fin."),
        uno("inv_registro", "Versiones del registro perdidas antes de leerlas", v.registro, "Reloj_Y_Eventos", "El batch lee el registro cada media hora de mundo."),
    ]
}

fn rendimiento(x: &Entrada) -> Vec<Metrica> {
    let c = x.corrida;
    let eventos = c.nucleo.metricas().eventos as f64;
    let ordenes: u64 = c.ordenes.values().map(|o| o.aceptadas + o.sin_cambios + o.rechazos.values().sum::<u64>()).sum();
    vec![
        metrica("pared_total", "Tiempo de pared total del escenario", "s", max(1200.0), Some(x.pared_total.as_secs_f64()), "Fase 6 (rendimiento)", "La corrida y las tres reducidas del determinismo."),
        metrica("pared_corrida", "Tiempo de pared de la corrida", "s", INFO, Some(c.pared.as_secs_f64()), "Fase 6 (rendimiento)", "Colocación de plazas y héroes y simulación, con las comprobaciones de cada orden."),
        metrica("cpu_por_dia", "Coste de CPU por día de mundo", "s", INFO, Some(c.pared.as_secs_f64() / x.e.dias.max(1) as f64), "Fase 6 (rendimiento)", "Pared de la corrida entre los días simulados."),
        metrica("eventos", "Eventos procesados por el núcleo", "eventos", INFO, Some(eventos), "Reloj_Y_Eventos", "De todas las clases."),
        metrica("ordenes", "Órdenes dadas por los bots", "órdenes", INFO, Some(ordenes as f64), "Proyecciones_Y_Autorizacion §5", "Aceptadas, sin cambios y rechazadas."),
    ]
}

fn determinismo(x: &Entrada) -> Vec<Metrica> {
    vec![
        metrica("determinismo", "Reproducción del azar (mismo escenario reducido, dos veces)", "1 = igual", (Some(1.0), Some(1.0)), x.determinismo.map(|d| d.0 as u8 as f64), "Arquitectura/13 §4 (núcleo determinista)", "El escenario reducido con la misma semilla dos veces: mismo `hash_estado()` final."),
        metrica("determinismo_foto", "Foto ida y vuelta (BAN28) a mitad del escenario reducido", "1 = igual", (Some(1.0), Some(1.0)), x.determinismo.map(|d| d.1 as u8 as f64), "Reloj_Y_Eventos (foto)", "El escenario reducido reconstruido desde su foto a mitad de camino acaba con el mismo hash que sin interrumpir."),
    ]
}

// ───────────────────────────── Gráficos y mapa ─────────────────────────────

fn grafico_combates(x: &Entrada) -> Grafico {
    let por_dia = |tipo: &str| {
        let mut v = vec![0.0; x.e.dias as usize + 1];
        de_tipo(x, tipo).for_each(|c| v[dia_de(c.t).min(x.e.dias as usize)] += 1.0);
        v.into_iter().enumerate().map(|(d, n)| (d as f64, n)).collect()
    };
    Grafico { id: "combates_dia".into(), titulo: "Combates por día y tipo".into(), tipo: TipoGrafico::Linea, eje_x: "día de mundo".into(), eje_y: "combates".into(), series: TIPOS.iter().map(|t| serie(t, por_dia(t))).collect() }
}

fn grafico_ejercito(x: &Entrada) -> Grafico {
    let m = &x.corrida.muestras;
    let s = |nombre: &str, f: &dyn Fn(&sm::Muestra) -> i64| serie(nombre, m.iter().map(|u| (u.dia as f64, f(u) as f64)).collect());
    Grafico {
        id: "ejercito_dia".into(),
        titulo: "Soldados en pie".into(),
        tipo: TipoGrafico::Linea,
        eje_x: "día de mundo".into(),
        eje_y: "soldados".into(),
        series: vec![s("Todos", &|u| u.soldados), s("En columnas", &|u| u.en_columna), s("En guarniciones de plaza", &|u| u.en_guarnicion)],
    }
}

fn grafico_politica(x: &Entrada) -> Grafico {
    let m = &x.corrida.muestras;
    let s = |nombre: &str, f: &dyn Fn(&sm::Muestra) -> usize| serie(nombre, m.iter().map(|u| (u.dia as f64, f(u) as f64)).collect());
    Grafico {
        id: "politica_dia".into(),
        titulo: "Plazas, Facciones, guerras y bandidos".into(),
        tipo: TipoGrafico::Linea,
        eje_x: "día de mundo".into(),
        eje_y: "cuántos".into(),
        series: vec![s("Plazas en pie", &|u| u.plazas), s("Facciones vivas", &|u| u.facciones_vivas), s("Pares en guerra", &|u| u.guerras), s("Campamentos de bandidos", &|u| u.bandidos), s("Columnas en el mapa", &|u| u.columnas_en_el_mapa)],
    }
}

fn grafico_bajas(x: &Entrada) -> Grafico {
    let m = &x.corrida.medidas;
    Grafico { id: "bajas_tropa".into(), titulo: "Bajas por tropa".into(), tipo: TipoGrafico::Barras, eje_x: "tropa".into(), eje_y: "soldados".into(), series: m.bajas_por_tropa.iter().map(|(t, n)| serie(&format!("{t:?}"), vec![(0.0, *n as f64)])).collect() }
}

fn grafico_rechazos(x: &Entrada) -> Grafico {
    let mut v: Vec<(String, u64)> = x.corrida.ordenes.iter().flat_map(|(e, c)| c.rechazos.iter().map(move |(k, n)| (format!("{e}: {k}"), *n))).collect();
    v.sort_by_key(|(k, n)| (std::cmp::Reverse(*n), k.clone()));
    Grafico { id: "rechazos".into(), titulo: "Órdenes rechazadas más frecuentes".into(), tipo: TipoGrafico::Barras, eje_x: "orden y código".into(), eje_y: "rechazos".into(), series: v.into_iter().take(15).map(|(k, n)| serie(&k, vec![(0.0, n as f64)])).collect() }
}

fn mapa(x: &Entrada) -> Mapa {
    let (c, n) = (x.corrida, &x.corrida.nucleo);
    let e = n.estado();
    let u = |v: i64| v as f64 / 1000.0;
    let punto = |p: base::Punto| Punto { x: u(p.x), y: u(p.y) };
    let capa = |nombre: &str, color: &str, puntos: Vec<Punto>| Capa { nombre: nombre.into(), color: color.into(), circulos: vec![], puntos, lineas: vec![] };
    let conquistadas: std::collections::BTreeSet<u64> = c.medidas.conquistas.iter().map(|k| k.1).collect();
    let plazas = |cayo: bool| e.asentamientos.values().filter(|a| conquistadas.contains(&a.id) == cayo).map(|a| punto(a.posicion)).collect();
    let columnas: Vec<Punto> = e.ejercitos.values().filter(|x| n.en_el_mapa(x.id)).map(|x| punto(x.posicion_en(n.ahora()))).collect();
    let exiliados = n.heroes().filter(|h| matches!(h.heroe.presencia.ubicacion, Ubicacion::Mercenarios(_))).count();
    let mut capas = vec![
        capa("Plazas nunca conquistadas", "#2563eb", plazas(false)),
        capa("Plazas conquistadas alguna vez", "#dc2626", plazas(true)),
        capa("Campamentos de bandidos al final", "#111827", n.campamentos_de_bandidos().map(|b| punto(b.posicion)).collect()),
        capa("Pueblos extractores", "#14b8a6", e.asentamientos.values().flat_map(|a| a.economia.pueblos.iter()).filter_map(|p| n.posicion_de_pueblo(p.id)).map(punto).collect()),
        capa("Avanzadas", "#a16207", e.asentamientos.values().flat_map(|a| a.economia.avanzadas.iter()).map(|a| punto(a.posicion)).collect()),
        capa("Columnas en el mapa al final", "#f59e0b", columnas),
        capa(&format!("Campamentos de mercenarios ({exiliados} héroes dentro al final)"), "#16a34a", x.geo.campamentos.iter().map(|&p| punto(p)).collect()),
    ];
    for (nombre, color) in [("Bosque", fondo::BOSQUE), ("Río", fondo::RIO), ("Agua", fondo::AGUA), ("Llano", fondo::LLANO), ("Colina", fondo::COLINA), ("Montaña", fondo::MONTANA)] {
        capas.push(capa(nombre, &hex(color), vec![]));
    }
    Mapa { ancho: u(x.geo.ancho), alto: u(x.geo.alto), fondo: fondo::fondo(&x.c.mundo), capas }
}

/// Anomalías automáticas: solo violaciones de invariante y de determinismo.
fn anomalias(metricas: &[Metrica]) -> Vec<Anomalia> {
    metricas
        .iter()
        .filter(|m| m.id.starts_with("inv_") || m.id.starts_with("determinismo"))
        .filter(|m| !matches!(informe::semaforo(m), informe::Semaforo::Verde | informe::Semaforo::Informativa | informe::Semaforo::SinDato))
        .map(|m| Anomalia {
            titulo: format!("Invariante incumplido: {}", m.nombre),
            detalle: format!("Medido {} {} (esperado entre {:?} y {:?}). {}", m.medido.unwrap_or(f64::NAN), m.unidad, m.esperado_min, m.esperado_max, m.nota.clone().unwrap_or_default()),
            causa: Some(Causa::Codigo),
        })
        .collect()
}
