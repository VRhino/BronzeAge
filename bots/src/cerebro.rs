//! El cerebro de un bot (`Docs/Arquitectura/12_NPC_Fuera_Del_Motor.md` §3, §12 y §16 del motor), sobre lo que le llega por el contrato. Hoy, el bot **sin plaza**: se busca una
//! Facción (la crea o pide entrar, según su perfil, D57; el Rey acepta toda solicitud), pide la leva prestada, sale a buscar los bandidos del anillo de su campamento y los caza
//! con lo que puede vencer, abre los alijos que pisa, vuelve, vacía el carro y, si su Facción ya tiene una plaza, se muda a ella.
//!
//! El orden de las órdenes sigue a los bots del batch de Rust (`crates/batch/src/simmil/pol.rs`) y las decisiones al cerebro TypeScript (`typescript-final`,
//! `src/bots/cerebro/sinPlaza.ts`). Las cifras de reglas salen del archivo de datos de juego del motor ([`crate::datos`]) y el terreno, del mapa público ([`crate::mapa`]).

use std::collections::BTreeMap;

use serde_json::json;

use contrato::almacen::Almacen;
use contrato::v1::{
    self as pb, contenedor_escuadron::Donde as Contenedor, entidad::Cuerpo, orden::Cuerpo as O, orden_atacar, orden_marchar_a, ubicacion_heroe::Donde, EstadoColumna, Punto, Resultado,
};

use crate::{
    balance,
    datos::{self, Datos},
    llegadas::Perfil,
    libro::{vida_media, Libro},
    mapa, plaza, politica,
    runner::{Cerebro, Contexto, ESPERA_TRAS_RECHAZO_MS},
    Fallo,
};

/// La leva que pide prestada: la milicia de lanceros (con las tres tropas comería el triple).
const PRESTAMO: &str = "milicia_lanceros";
/// Una unidad en las `Cantidades` del contrato (milésimas): lo almacenado va en unidades enteras.
const UNIDAD: i64 = 1000;
/// Lo que mira alrededor al pensar, más allá del anillo de bandidos.
const MARGEN_VISTA: i64 = 20_000;
/// Dónde buscar otro sitio para fundar si `fundar` dice que no: cerca de donde está (`crates/batch/src/simmil/pol.rs`); y cuántas veces.
const OTRO_SITIO: (i64, i64) = (20_000, 60_000);
const INTENTOS_DE_FUNDAR: u32 = 3;
/// Lo que el que lleva al grupo espera en la puerta a que se le unan.
const ESPERA_GRUPO_MS: i64 = 10 * 60_000;
/// Puntos de la vuelta al anillo del explorador.
const VUELTA: u32 = 8;
/// Lo que vale un bandido apuntado en la pizarra, y lo que espera respuesta a una solicitud de ingreso antes de fundar la suya.
const VIGENCIA_BANDIDO_MS: i64 = 3 * 3_600_000;
/// ponytail: un día y no 2 h (TypeScript): el Rey bot acepta solo en sus horas de conexión (D55), y con 2 h los tardíos fundaban la suya (30 bots, 19 Facciones en la partida de
/// 7 días). A calibrar con el batch.
const ESPERA_SOLICITUD_MS: i64 = 24 * 3_600_000;

/// Lo que un bot recuerda de sí: desechable, se rehace mirando el mundo.
#[derive(Default)]
pub struct Memoria {
    plan: Option<Plan>,
    solicitud: Option<(String, i64)>,
    /// Los lanceros que llegó a tener en su leva: con menos, la repone.
    leva_max: u32,
    /// Lo que hace fuera quien salió de su plaza.
    pub(crate) salida: Option<plaza::Salida>,
    /// La Facción con plaza a la que pide entrar tras dejar la suya, que no lograba fundar.
    destino: Option<String>,
    /// Por qué está en casa sin salir, para el panel («caza en espera 1,5 h · sin tropa libre»).
    pub(crate) en_casa: Option<String>,
}

enum Plan {
    /// Recorre el anillo de su campamento sin tropa: empezó en el paso `inicio` de la vuelta y va por `paso`.
    Explorar { inicio: u32, paso: u32 },
    /// Va a por un campamento de bandidos.
    Cazar { bandidos: String },
    /// Lleva al grupo: espera en la puerta, desde `desde`, a que se le unan `cuantos` para ir a por `bandidos`.
    Esperar { bandidos: String, cuantos: usize, desde: i64 },
    /// Va en la columna de un compañero; `salio`, ya se alejó de la puerta (volver a ella es volver a casa).
    Unirse { salio: bool },
    /// Vuelve a su campamento.
    Volver,
    /// Va a vivir a la plaza de su Facción.
    Mudarse,
    /// Lleva la Caravana de Fundación a `sitio` para fundar; `intentos`, los sitios donde `fundar` ya dijo que no.
    Fundar { sitio: Punto, intentos: u32 },
    /// Sale a fundar en grupo: espera en la puerta, desde `desde`, a que se le unan `cuantos` (la caravana se defiende con el poder de la columna que la lleva).
    EsperarFundar { cuantos: usize, desde: i64, inicio: i64 },
}

/// Lo que se cuentan los bots de una Facción: los bandidos que vieron (los campamentos de bandidos no tienen memoria en la vista) y quién explora qué anillo.
#[derive(Default)]
pub struct Pizarra {
    pub(crate) bandidos: BTreeMap<String, (Punto, u32, i64)>,
    /// Lo que va a hacer cada uno y qué grupo se reúne en qué puerta ([`crate::tablero`]): para no ir dos por separado a lo mismo y salir juntos los que coinciden.
    pub(crate) tablero: crate::tablero::Tablero,
    /// La defensa de las plazas ajenas inspeccionadas: el poder de su guarnición, los héroes que había dentro y cuándo se vio.
    pub(crate) defensas: BTreeMap<String, (u32, usize, i64)>,
    /// Lo que la Facción recuerda de las demás: va al registro.
    pub(crate) libro: Libro,
    /// El Rey de la Facción, cuyo carácter pesa en el libro.
    pub(crate) rey: Option<String>,
    /// El nivel de cada Facción que se ve.
    pub(crate) niveles: BTreeMap<String, u32>,
    /// Los hechos ya apuntados, porque los oyen varios bots.
    pub(crate) vistos: std::collections::VecDeque<u64>,
    /// Con quién tiene cada trueque en marcha, por su número.
    pub(crate) acuerdos: BTreeMap<String, String>,
    /// Las plazas recuperadas que el Rey tiene que renombrar.
    pub(crate) por_renombrar: Vec<String>,
    /// Cuándo se repasó el libro por última vez.
    pub(crate) repasado_en: i64,
    /// El día y las compras de intel de ese día.
    pub(crate) intel: (i64, u32),
    /// En la pizarra común: quién tiene a quién por enemigo jurado.
    pub(crate) jurados: BTreeMap<String, String>,
    /// Las plazas ajenas que vio algún bot de la Facción, con su dueña y dónde están: la vista solo trae lo que cae en su rectángulo, y el motor no manda la `MemoriaFaccion`.
    pub(crate) ajenas: BTreeMap<String, (String, Punto)>,
    /// Los pueblos que vio algún bot de la Facción, por lo mismo: su madre (si se vio el pueblo) o su Facción (si se recuerda), dónde están y si tenían guarnición o batalla.
    pub(crate) pueblos: BTreeMap<String, (String, String, Punto, bool)>,
    /// Cuándo vio la Facción por última vez cada casilla de la rejilla ([`balance::CASILLA`]): lo que miran todos sus bots fuera de un campamento.
    pub(crate) rejilla: BTreeMap<(i64, i64), i64>,
    /// Lo que un rechazo vetó para toda la Facción hasta un instante (`fundar` con el cupo de plazas lleno, atacar una plaza protegida): las esperas de cada bot son suyas,
    /// y sin esto cada residente vuelve a probar lo que a otro ya le dijeron que no.
    pub(crate) vetos: BTreeMap<String, i64>,
    /// Cuántas plazas tenía la Facción la última vez que miró su Rey: si baja, ha perdido una y pide la paz.
    pub(crate) plazas_propias: Option<usize>,
    /// Dónde reside cada bot de la Facción: cada uno apunta la suya al pensar (lo que la Facción ve de una plaza ajena a su interior no lleva quién vive en ella).
    pub(crate) residencias: BTreeMap<String, String>,
    /// Los soldados propios de cada bot de la Facción y las escuadras que le faltan (una de cada tropa que su plaza sabe hacer): cada uno apunta lo suyo al reclutar (las
    /// escuadras de un compañero no se ven).
    pub(crate) soldados: BTreeMap<String, (u32, usize)>,
    /// Cuándo acabó el último explorador su vuelta al anillo de cada campamento.
    explorado: BTreeMap<String, i64>,
    /// Quién está dentro de qué campamento, listo para salir a cazar, con el poder de su leva y hasta cuándo vale.
    listos: BTreeMap<String, (String, u32, i64)>,
    /// La Facción de cada Rey que la apuntó: los amigos buscan la de su líder; los tardíos, una cualquiera. ponytail: hace de la lista pública de Facciones, que el contrato no manda.
    facciones: BTreeMap<String, String>,
    /// Cuántos tardíos ha recibido cada Facción.
    ingresos: BTreeMap<String, u32>,
    /// Las veces que a la Facción, aún sin plaza, le ha salido mal fundar.
    fracasos_fundar: u32,
    /// Cuántos eventos de cada tipo ha tenido la Facción ([`evento`]): reuniones, grupos que salen o se deshacen, uniones, despertares... Para el panel.
    pub(crate) cuentas: BTreeMap<String, u64>,
    /// Los hechos que ya despertaron a alguien: los oyen varios bots de la Facción y basta con una vez.
    oidos: std::collections::VecDeque<u64>,
    /// Los bots de la Facción (cada uno se apunta al pensar): un hecho solo da puntos a los suyos.
    pub(crate) miembros: std::collections::BTreeSet<String>,
    /// Los últimos logros de la Facción, para el panel ([`balance::LOGROS_GUARDADOS`]).
    pub(crate) logros: std::collections::VecDeque<Logro>,
    /// Los puntos de cada bot por clase de logro (guerra, obras, exploracion, comercio). No se olvidan; van al registro.
    pub(crate) puntos: BTreeMap<String, BTreeMap<String, u64>>,
    /// Los yacimientos que conoce la Facción (estaño, cobre, hierro, oro, piedra): su tipo, dónde y cuándo se supo de él. Van al registro y al panel.
    pub(crate) nodos: BTreeMap<String, (String, i64, i64, i64)>,
}

/// Algo que la Facción consiguió: cuándo, qué, quién lo logró y quiénes estuvieron, y lo que vale.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Logro {
    pub en: i64,
    pub tipo: String,
    pub texto: String,
    pub clase: String,
    pub puntos: u64,
    pub logrador: Option<String>,
    pub participantes: Vec<String>,
    pub plaza: Option<String>,
}

/// Apunta un logro y reparte sus puntos: enteros a quien lo logró, [`balance::PARTE_DE_PARTICIPANTE`] a cada uno de los demás.
pub(crate) fn apuntar_logro(p: &mut Pizarra, l: Logro) {
    let parte = (l.puntos as f64 * balance::PARTE_DE_PARTICIPANTE).round() as u64;
    if let Some(h) = &l.logrador {
        *p.puntos.entry(h.clone()).or_default().entry(l.clase.clone()).or_default() += l.puntos;
    }
    for h in &l.participantes {
        *p.puntos.entry(h.clone()).or_default().entry(l.clase.clone()).or_default() += parte;
    }
    *p.cuentas.entry("logro".into()).or_default() += 1;
    p.logros.push_back(l);
    while p.logros.len() > balance::LOGROS_GUARDADOS {
        p.logros.pop_front();
    }
}

/// Los puntos de un bot: los de una clase, o el total con `None`.
pub(crate) fn puntos_de(p: &Pizarra, heroe: &str, clase: Option<&str>) -> u64 {
    p.puntos.get(heroe).map_or(0, |c| match clase {
        Some(k) => c.get(k).copied().unwrap_or(0),
        None => c.values().sum(),
    })
}

/// El logro que da un hecho del motor, si es de los que cuentan: tipo, clase, puntos y la frase.
fn logro_de_hecho(h: &pb::EventoDominio) -> Option<(&'static str, &'static str, u64, String)> {
    use balance::puntos as pt;
    let donde = h.asentamiento_id.clone().unwrap_or_default();
    Some(match h.codigo.as_str() {
        "combate.campamento_destruido" => ("campamento_destruido", "guerra", pt::CAMPAMENTO_DESTRUIDO, "destruye un campamento de bandidos".into()),
        "asentamiento.conquistada" => ("plaza_conquistada", "guerra", pt::PLAZA_CONQUISTADA, format!("conquista {donde}")),
        "combate.asedio_resistido" => ("asedio_resistido", "guerra", pt::ASEDIO_RESISTIDO, format!("resiste el asedio de {donde}")),
        "combate.pueblo_saqueado" => ("pueblo_saqueado", "guerra", pt::PUEBLO_SAQUEADO, "saquea un pueblo".into()),
        "fundacion.asentamiento_fundado" => ("plaza_fundada", "exploracion", pt::PLAZA_FUNDADA, format!("funda {donde}")),
        "pueblo.materiales_entregados" | "avanzada.materiales_entregados" => ("materiales_entregados", "obras", pt::MATERIALES_ENTREGADOS, "lleva materiales a una obra".into()),
        "pueblo.completado" | "avanzada.completada" => ("obra_terminada", "obras", pt::OBRA_TERMINADA, format!("termina una obra de {donde}")),
        "comercio.trueque_cumplido" => ("trueque_cumplido", "comercio", pt::TRUEQUE_CUMPLIDO, "cumple un trueque".into()),
        "comercio.caravana_en_casa" => ("caravana_en_casa", "comercio", pt::CARAVANA_EN_CASA, "trae una caravana a casa".into()),
        // Lo oye la capital, sin héroes: el logro es de la Facción.
        "tecnologia.aparece" => match &h.tipado {
            Some(pb::evento_dominio::Tipado::TecnologiaAparecida(x)) if x.primera => ("tecnologia_descubierta", "obras", pt::TECNOLOGIA_DESCUBIERTA, format!("descubre {}, la primera del mundo", x.tecnologia_id)),
            Some(pb::evento_dominio::Tipado::TecnologiaAparecida(x)) => ("tecnologia_aparecida", "obras", pt::TECNOLOGIA_APARECIDA, format!("le aparece {}", x.tecnologia_id)),
            _ => return None,
        },
        _ => return None,
    })
}

/// El cerebro de los bots.
#[derive(Default)]
pub struct Bot;

impl Cerebro for Bot {
    type Memoria = Memoria;
    type Pizarra = Pizarra;

    async fn pensar(&self, ctx: &mut Contexto<'_, Self>) -> Result<(), Fallo> {
        pensar(ctx).await
    }

    fn guardar(p: &Pizarra) -> serde_json::Value {
        json!({ "libro": p.libro, "puntos": p.puntos, "logros": p.logros, "nodos": p.nodos })
    }

    fn cargar(g: &serde_json::Value) -> Pizarra {
        // Un registro de antes guardaba solo el libro.
        if g.get("libro").is_none() {
            return Pizarra { libro: serde_json::from_value(g.clone()).unwrap_or_default(), ..Default::default() };
        }
        Pizarra {
            libro: serde_json::from_value(g["libro"].clone()).unwrap_or_default(),
            puntos: serde_json::from_value(g["puntos"].clone()).unwrap_or_default(),
            logros: serde_json::from_value(g["logros"].clone()).unwrap_or_default(),
            nodos: serde_json::from_value(g["nodos"].clone()).unwrap_or_default(),
            ..Default::default()
        }
    }

    fn publico(p: &Pizarra, t: i64) -> serde_json::Value {
        let v = vida_media(p.rey.as_deref());
        serde_json::Value::Object(p.libro.paginas.keys().map(|f| (f.clone(), json!(p.libro.estado(f, t, v)))).collect())
    }

    /// Lo que despierta a los compañeros en cuanto se oye: una batalla en una plaza suya llama a quienes viven en ella (a defenderla), y una plaza fundada, a los que aún
    /// esperan en un campamento (a mudarse).
    fn oir(h: &pb::EventoDominio, p: &mut Pizarra) -> Vec<String> {
        let huella = crate::azar::hash(&format!("{}|{}|{:?}|{:?}", h.codigo, h.en_ms, h.asentamiento_id, h.heroes_ids));
        if p.oidos.contains(&huella) {
            return Vec::new();
        }
        p.oidos.push_back(huella);
        if p.oidos.len() > 256 {
            p.oidos.pop_front();
        }
        // Un logro de la Facción: quien lo nombra el hecho primero, y los demás de los suyos que nombra, estuvieron. Sin héroes nombrados, cuenta si es en una plaza suya.
        if let Some((tipo, clase, puntos, texto)) = logro_de_hecho(h) {
            let suyos: Vec<String> = h.heroes_ids.iter().filter(|x| p.miembros.contains(*x)).cloned().collect();
            let en_casa = h.asentamiento_id.as_ref().is_some_and(|a| p.residencias.values().any(|r| r == a));
            if !suyos.is_empty() || (h.heroes_ids.is_empty() && en_casa) {
                let (logrador, participantes) = match suyos.split_first() {
                    Some((l, resto)) => (Some(l.clone()), resto.to_vec()),
                    None => (None, Vec::new()),
                };
                apuntar_logro(p, Logro { en: h.en_ms, tipo: tipo.into(), texto, clase: clase.into(), puntos, logrador, participantes, plaza: h.asentamiento_id.clone() });
            }
        }
        let despiertan: Vec<String> = match h.codigo.as_str() {
            "batalla.abierta" => h.asentamiento_id.as_ref().map(|x| p.residencias.iter().filter(|(_, r)| *r == x).map(|(h, _)| h.clone()).collect()).unwrap_or_default(),
            "fundacion.asentamiento_fundado" => p.listos.keys().cloned().collect(),
            _ => Vec::new(),
        };
        if !despiertan.is_empty() {
            *p.cuentas.entry("despertar".into()).or_default() += 1;
        }
        despiertan
    }

    fn resumen(p: &Pizarra, t: i64) -> serde_json::Value {
        resumen_de_pizarra(p, t)
    }

    fn detalle(p: &Pizarra, t: i64) -> serde_json::Value {
        detalle_de_pizarra(p, t)
    }

    fn volcar(m: &Memoria) -> serde_json::Value {
        let mut v = json!({ "leva": m.leva_max });
        if let Some(plan) = m.salida.as_ref().map(plaza::Salida::volcar).or(m.plan.as_ref().map(Plan::volcar)).or(m.en_casa.as_ref().map(|x| json!({ "tipo": x }))) {
            v["plan"] = plan;
        }
        if let Some((faccion, desde)) = &m.solicitud {
            v["solicitud"] = json!({ "faccionId": faccion, "desde": desde });
        }
        v
    }
}

/// Las necesidades (supervivencia) del tablero para el panel: con quién la hace (un cargo, `libre`, `residentes`) y quién la lleva ahora.
fn necesidades_para_el_panel(p: &Pizarra, t: i64) -> Vec<serde_json::Value> {
    p.tablero
        .necesidades(t)
        .into_iter()
        .map(|(k, n, lleva)| json!({"clave": k, "texto": n.texto, "prioridad": n.prioridad, "plazaId": n.plaza, "hasta": n.hasta, "quien": n.quien, "cubiertaPor": lleva}))
        .collect()
}

/// Las metas (progreso) del tablero para el panel, por prioridad: cada una con sus pasos, quién hace cada uno y quién lo lleva ahora.
fn metas_para_el_panel(p: &Pizarra, t: i64) -> Vec<serde_json::Value> {
    p.tablero
        .metas(t)
        .into_iter()
        .map(|(k, m, lleva)| {
            let pasos: Vec<_> = m.pasos.iter().zip(lleva).map(|(x, l)| json!({"clave": x.clave, "texto": x.texto, "quien": x.quien, "llevaId": l})).collect();
            json!({"clave": k, "texto": m.texto, "prioridad": m.prioridad, "plazaId": m.plaza, "hasta": m.hasta, "pasos": pasos})
        })
        .collect()
}

/// Lo que el panel ve de una pizarra cada 2 s (`PizarraInfo` del contrato de control): lo que ya pinta (encargos, bandidos, salidas, anillos, listos, residencias) y,
/// además, el tablero entero, la rejilla, los contadores de cooperación y la relación con las demás Facciones.
pub(crate) fn resumen_de_pizarra(p: &Pizarra, t: i64) -> serde_json::Value {
    let tareas: Vec<_> = p.tablero.tareas().iter().filter(|(_, x)| x.hasta > t).collect();
    let reuniones: Vec<_> = tareas.iter().filter_map(|(k, x)| x.reunion.as_ref().filter(|r| r.hasta > t).map(|r| (k, x, r))).collect();
    let v = vida_media(p.rey.as_deref());
    json!({
        "encargos": tareas.iter().map(|(k, x)| json!({"clave": k, "heroeId": x.lider, "hasta": x.hasta})).collect::<Vec<_>>(),
        "bandidos": p.bandidos.len(),
        "salidas": reuniones.len(),
        "explorados": p.explorado.len(),
        "detalle": {
            "bandidosVistos": p.bandidos.iter().map(|(id, (q, poder, visto))| json!({"id": id, "x": q.x, "y": q.y, "poder": poder, "vistoEn": visto})).collect::<Vec<_>>(),
            "salidasAbiertas": reuniones.iter().map(|(k, x, r)| json!({
                "campamentoId": r.puerta, "ejercitoId": r.columna, "liderId": x.lider, "hasta": r.hasta, "para": if k.starts_with("fundar:") { "fundar" } else if k.starts_with("campana:") { "campana" } else { "cazar" },
                "tarea": k, "objetivo": r.objetivo, "unidos": r.unidos, "fuerza": r.fuerza,
            })).collect::<Vec<_>>(),
            "exploradosEn": p.explorado.iter().map(|(k, en)| json!({"campamentoId": k, "en": en})).collect::<Vec<_>>(),
            "residencias": p.residencias.iter().map(|(h, r)| json!({"heroeId": h, "plazaId": r})).collect::<Vec<_>>(),
            "listos": p.listos.iter().map(|(h, (k, poder, hasta))| json!({"heroeId": h, "campamentoId": k, "conRacion": *poder > 0, "poder": poder, "hasta": hasta})).collect::<Vec<_>>(),
        },
        "rejilla": {"vistas": p.rejilla.len(), "apartadas": tareas.iter().filter(|(k, _)| k.starts_with("casilla:")).count()},
        "necesidades": necesidades_para_el_panel(p, t),
        "metas": metas_para_el_panel(p, t),
        "cooperacion": p.cuentas,
        "puntos": puntos_para_el_panel(p),
        "logros": p.logros.iter().rev().take(20).collect::<Vec<_>>(),
        "relaciones": p.libro.paginas.keys().map(|f| (f.clone(), json!({"puntaje": p.libro.relacion(f, t, v), "estado": p.libro.estado(f, t, v)}))).collect::<serde_json::Map<_, _>>(),
        "rey": p.rey,
        "estanoVisto": p.libro.estano_visto.len(),
        "nodos": p.nodos.values().fold(BTreeMap::<String, usize>::new(), |mut m, (tipo, ..)| {
            *m.entry(tipo.clone()).or_default() += 1;
            m
        }),
        "fracasosFundar": p.fracasos_fundar,
    })
}

/// La pizarra entera, para el panel a petición: el resumen y todo lo demás que la Facción sabe y decide, con su libro de agravios y lo que pesa cada asiento hoy.
pub(crate) fn detalle_de_pizarra(p: &Pizarra, t: i64) -> serde_json::Value {
    let v = vida_media(p.rey.as_deref());
    let punto = |q: &Punto| json!({"x": q.x, "y": q.y});
    json!({
        "resumen": resumen_de_pizarra(p, t),
        "tablero": p.tablero.tareas().iter().map(|(k, x)| json!({
            "clave": k, "liderId": x.lider, "hasta": x.hasta,
            "reunion": x.reunion.as_ref().map(|r| json!({"puerta": r.puerta, "columna": r.columna, "objetivo": r.objetivo, "hasta": r.hasta, "unidos": r.unidos, "fuerza": r.fuerza})),
        })).collect::<Vec<_>>(),
        "necesidades": necesidades_para_el_panel(p, t),
        "metas": metas_para_el_panel(p, t),
        "rejilla": {"lado": balance::CASILLA, "casillas": p.rejilla.iter().map(|((x, y), en)| json!([x, y, en])).collect::<Vec<_>>()},
        "defensas": p.defensas.iter().map(|(id, (poder, heroes, visto))| (id.clone(), json!({"poder": poder, "heroes": heroes, "vistoEn": visto}))).collect::<serde_json::Map<_, _>>(),
        "ajenas": p.ajenas.iter().map(|(id, (f, q))| (id.clone(), json!({"faccionId": f, "posicion": punto(q)}))).collect::<serde_json::Map<_, _>>(),
        "pueblos": p.pueblos.iter().map(|(id, (madre, f, q, defendido))| (id.clone(), json!({"madreId": madre, "faccionId": f, "posicion": punto(q), "defendido": defendido}))).collect::<serde_json::Map<_, _>>(),
        "niveles": p.niveles,
        "vetos": p.vetos.iter().filter(|(_, h)| **h > t).map(|(k, h)| (k.clone(), json!(h))).collect::<serde_json::Map<_, _>>(),
        "acuerdos": p.acuerdos,
        "jurados": p.jurados,
        "plazasPropias": p.plazas_propias,
        "porRenombrar": p.por_renombrar,
        "intel": {"dia": p.intel.0, "compras": p.intel.1},
        "repasadoEn": p.repasado_en,
        "ingresos": p.ingresos,
        "libro": p.libro,
        "relaciones": p.libro.paginas.iter().map(|(f, pagina)| (f.clone(), json!({
            "puntaje": p.libro.relacion(f, t, v),
            "estado": p.libro.estado(f, t, v),
            "asientos": pagina.vigentes(t, v).into_iter().map(|(a, peso)| json!({"asiento": a, "peso": peso})).collect::<Vec<_>>(),
            "traicion": pagina.traicion,
            "confianzaHasta": pagina.confianza_hasta,
        }))).collect::<serde_json::Map<_, _>>(),
        "vidaMediaMs": v,
        "logros": p.logros.iter().rev().collect::<Vec<_>>(),
        "puntos": puntos_para_el_panel(p),
        "nodos": p.nodos.iter().map(|(id, (tipo, x, y, en))| json!({"id": id, "tipo": tipo, "x": x, "y": y, "conocidoEn": en})).collect::<Vec<_>>(),
    })
}

/// Los puntos de cada bot de la Facción para el panel: por clase y el total.
fn puntos_para_el_panel(p: &Pizarra) -> serde_json::Value {
    serde_json::Value::Object(
        p.puntos
            .iter()
            .map(|(h, c)| {
                let mut v = json!(c);
                v["total"] = json!(c.values().sum::<u64>());
                (h.clone(), v)
            })
            .collect(),
    )
}

/// Un evento de su Facción para el panel: lo cuenta en su pizarra y lo manda en vivo.
pub(crate) fn evento<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, tipo: &str, texto: String, datos: serde_json::Value) {
    ctx.con_pizarra(|p| *p.cuentas.entry(tipo.into()).or_default() += 1);
    ctx.evento(tipo, texto, datos);
}

impl Plan {
    /// Para el panel: los `tipo` que conoce llevan sus campos; el resto es una frase, que pinta tal cual.
    fn volcar(&self) -> serde_json::Value {
        match self {
            Plan::Explorar { paso, .. } => json!({ "tipo": format!("Recorrer el anillo de su campamento sin tropa (paso {paso})") }),
            Plan::Cazar { bandidos } => json!({ "tipo": "cazar", "campamentoId": bandidos }),
            Plan::Esperar { bandidos, cuantos, .. } => json!({ "tipo": format!("Esperar en la puerta a {cuantos} para cazar {bandidos}") }),
            Plan::Unirse { salio } => json!({ "tipo": format!("Ir en la columna de un compañero{}", if *salio { " · ya salió de la puerta" } else { "" }) }),
            Plan::Volver => json!({ "tipo": "Volver a su campamento" }),
            Plan::Mudarse => json!({ "tipo": "Mudarse a la plaza de su Facción" }),
            Plan::Fundar { sitio, intentos } => json!({ "tipo": format!("Fundar en ({}, {}), {intentos} sitios descartados", sitio.x / 1000, sitio.y / 1000) }),
            Plan::EsperarFundar { cuantos, .. } => json!({ "tipo": format!("Esperar en la puerta a {cuantos} para salir a fundar") }),
        }
    }
}

/// Los cerebros que piensan con la memoria y la pizarra de [`Bot`] (el propio [`Bot`], o uno que lo envuelve para mirarlo).
pub trait Tipos: Cerebro<Memoria = Memoria, Pizarra = Pizarra> {}
impl<C: Cerebro<Memoria = Memoria, Pizarra = Pizarra>> Tipos for C {}

/// Lo que piensa un bot.
pub async fn pensar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>) -> Result<(), Fallo> {
    let Some(h) = heroe(ctx.puerto.observar(), ctx.yo) else { return Ok(()) };
    let d = datos::de(ctx.puerto).await?;
    let (t, yo) = (ctx.ahora_ms, ctx.yo.to_string());
    ctx.con_pizarra(|p| {
        p.tablero.limpiar(t);
        p.miembros.insert(yo);
    });
    politica::apuntar(ctx);
    politica::repasar(ctx);
    misiones(ctx, &h).await?;
    rey(ctx).await?;
    mirar(ctx, &d, &h).await?;
    if let Some(casa) = h.residencia_id.clone().filter(|r| r.starts_with("as-")) {
        return plaza::con_plaza(ctx, &d, h, &casa).await;
    }
    sin_plaza(ctx, &d, h).await
}

// ───────────────────────────── Lecturas ─────────────────────────────

pub(crate) fn entidades(a: &Almacen) -> impl Iterator<Item = &Cuerpo> {
    a.ambitos.values().flat_map(|x| x.entidades.values())
}

pub(crate) fn heroe(a: &Almacen, yo: &str) -> Option<pb::Heroe> {
    entidades(a).find_map(|c| match c {
        Cuerpo::Heroe(h) if h.id == yo => Some(h.clone()),
        _ => None,
    })
}

fn faccion(a: &Almacen, id: &str) -> Option<pb::Faccion> {
    entidades(a).find_map(|c| match c {
        Cuerpo::Faccion(f) if f.id == id => Some(f.clone()),
        _ => None,
    })
}

fn campamento(a: &Almacen, id: &str) -> Option<pb::CampamentoMercenarios> {
    entidades(a).find_map(|c| match c {
        Cuerpo::CampamentoMercenarios(k) if k.id == id => Some(k.clone()),
        _ => None,
    })
}

/// Su columna, si va en una.
pub fn columna(a: &Almacen, yo: &str) -> Option<pb::Ejercito> {
    entidades(a).find_map(|c| match c {
        Cuerpo::Ejercito(e) if e.participantes.iter().any(|p| p.heroe_id == yo) => Some(e.clone()),
        _ => None,
    })
}

/// Dónde está una columna en `t`: la interpolación de su plan (contrato, `common.proto`).
pub fn posicion(e: &pb::Ejercito, t: i64) -> Punto {
    let v = e.plan.as_ref().map(|p| p.vertices.as_slice()).unwrap_or_default();
    let quieto = || e.posicion.clone().unwrap_or_default();
    let Some(primero) = v.first() else { return quieto() };
    if t <= primero.en_ms {
        return primero.p.clone().unwrap_or_default();
    }
    for w in v.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if t < b.en_ms {
            let (pa, pb_) = (a.p.clone().unwrap_or_default(), b.p.clone().unwrap_or_default());
            let f = (t - a.en_ms) as f64 / (b.en_ms - a.en_ms).max(1) as f64;
            return Punto { x: pa.x + ((pb_.x - pa.x) as f64 * f) as i64, y: pa.y + ((pb_.y - pa.y) as f64 * f) as i64 };
        }
    }
    v.last().and_then(|x| x.p.clone()).unwrap_or_else(quieto)
}

/// ¿Tiene su Facción explorada la celda de `q`? La máscara de la niebla: celda `i` = fila × columnas + columna, byte `i >> 3`, bit `i & 7` (Niebla_De_Guerra §2.1).
pub(crate) fn explorado_en(n: &pb::NieblaProyectada, q: &Punto) -> bool {
    let l = n.tamano_celda.max(1) as i64;
    let (c, f) = (q.x.div_euclid(l), q.y.div_euclid(l));
    let i = (f * n.columnas as i64 + c) as usize;
    c >= 0 && f >= 0 && c < n.columnas as i64 && f < n.filas as i64 && n.celdas.get(i >> 3).is_some_and(|b| b >> (i & 7) & 1 == 1)
}

/// La casilla de la rejilla en que cae `p`.
pub(crate) fn casilla(p: &Punto) -> (i64, i64) {
    (p.x.div_euclid(balance::CASILLA), p.y.div_euclid(balance::CASILLA))
}

/// El centro de una casilla de la rejilla.
pub(crate) fn centro_de_casilla((cx, cy): (i64, i64)) -> Punto {
    let l = balance::CASILLA;
    Punto { x: cx * l + l / 2, y: cy * l + l / 2 }
}

/// Las casillas que caen enteras en el círculo de radio `mira` alrededor de `p` (su centro, a menos de `mira` menos media casilla) quedan vistas en `t`: es lo que se ve
/// de seguro, también el estaño, que se apunta en ese círculo. Lo que se ve de las esquinas del cuadrado de la vista no cuenta.
pub(crate) fn marcar_vistas(rejilla: &mut BTreeMap<(i64, i64), i64>, p: &Punto, mira: i64, t: i64) {
    let (a, b) = (casilla(&Punto { x: p.x - mira, y: p.y - mira }), casilla(&Punto { x: p.x + mira, y: p.y + mira }));
    for cx in a.0..=b.0 {
        for cy in a.1..=b.1 {
            let c = centro_de_casilla((cx, cy));
            if distancia(&c, p) <= mira - balance::CASILLA / 2 {
                rejilla.insert((cx, cy), t);
            }
        }
    }
}

pub(crate) fn distancia(a: &Punto, b: &Punto) -> i64 {
    (((a.x - b.x) as f64).hypot((b.y - a.y) as f64)) as i64
}

/// Sus escuadras que están en el campamento y tienen gente.
fn tropa_en_campamento(a: &Almacen, yo: &str) -> Vec<pb::Escuadron> {
    entidades(a)
        .filter_map(|c| match c {
            Cuerpo::Escuadron(e) if e.heroe_id == yo && e.cantidad > 0 && matches!(e.contenedor.as_ref().and_then(|c| c.donde.clone()), Some(Contenedor::Campamento(_))) => Some(e.clone()),
            _ => None,
        })
        .collect()
}

/// El poder de la leva que lleva en su columna.
fn poder_en_columna(d: &Datos, a: &Almacen, yo: &str, columna: &str) -> u32 {
    entidades(a)
        .filter_map(|c| match c {
            Cuerpo::Escuadron(e) if e.heroe_id == yo && matches!(e.contenedor.as_ref().and_then(|c| c.donde.as_ref()), Some(Contenedor::EjercitoId(x)) if x == columna) => {
                Some(d.poder_de(e))
            }
            _ => None,
        })
        .sum()
}

/// La fuerza de una columna: la de sus escuadras a la vista y, por las que no se ven (la tropa de otros), la media de las que sí; más la de cada héroe.
pub(crate) fn poder_de_columna(d: &Datos, a: &Almacen, col: &pb::Ejercito) -> u32 {
    let vistas: Vec<u32> = entidades(a)
        .filter_map(|c| match c {
            Cuerpo::Escuadron(e) if col.escuadron_ids.contains(&e.id) => Some(d.poder_de(e)),
            _ => None,
        })
        .collect();
    let tropa = if vistas.is_empty() { 0 } else { (vistas.iter().map(|&x| x as u64).sum::<u64>() * col.escuadron_ids.len() as u64 / vistas.len() as u64) as u32 };
    tropa + d.poder_por_heroe * col.participantes.len().max(1) as u32
}

/// Abre, o alarga `espera_ms`, la reunión de su grupo en `puerta` con su `columna` (la tarea `clave`, a `objetivo`) y despierta a los compañeros que están allí: los que
/// viven en esa plaza o esperan listos en ese campamento. Es el evento «salgo; quien venga, que se una». ¿La abre?
pub(crate) fn reunir<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, clave: &str, puerta: &str, columna: &str, objetivo: &str, espera_ms: i64) -> bool {
    let (yo, t) = (ctx.yo.to_string(), ctx.ahora_ms);
    let (abierta, nueva, vecinos) = ctx.con_pizarra(|p| {
        let nueva = p.tablero.tareas().get(clave).is_none_or(|x| x.reunion.is_none() || x.lider != yo || x.hasta <= t);
        let abierta = p.tablero.reunir(clave, &yo, puerta, columna, objetivo, t, t + espera_ms, t + balance::TAREA_VALE_MS);
        let vecinos: Vec<String> = p
            .residencias
            .iter()
            .filter(|(_, r)| *r == puerta)
            .map(|(h, _)| h.clone())
            .chain(p.listos.iter().filter(|(_, (k, _, hasta))| k == puerta && *hasta > t).map(|(h, _)| h.clone()))
            .collect();
        (abierta, nueva, vecinos)
    });
    if abierta {
        if nueva {
            let datos = json!({"tarea": clave, "puerta": puerta, "columna": columna, "objetivo": objetivo, "despertados": vecinos});
            evento(ctx, "reunion", format!("reúne un grupo en {puerta} para {clave} y despierta a {}", vecinos.len()), datos);
        }
        ctx.avisar(vecinos);
    }
    abierta
}

/// Lo que hace el líder de un grupo que se reúne en la puerta.
pub(crate) enum Grupo {
    Salir,
    Esperar,
    Deshacer,
}

/// El líder de un grupo que se reúne en `puerta` desde `inicio` para la tarea `clave`: mientras la reunión sigue abierta, espera (salvo que ya estén los `cuantos` que
/// esperaba y basten); acabada, sale si la fuerza real de su columna `basta`, o la alarga otra vez mientras no pase `tope` desde el inicio; después se deshace y suelta la
/// tarea. Así sale con lo que de verdad lleva, no con lo que prometían los que estaban en casa.
#[allow(clippy::too_many_arguments)]
pub(crate) fn decidir_grupo<C: Tipos + ?Sized>(
    ctx: &Contexto<'_, C>,
    d: &Datos,
    clave: &str,
    puerta: &str,
    objetivo: &str,
    inicio: i64,
    tope: i64,
    espera_ms: i64,
    cuantos: usize,
    basta: impl Fn(u32) -> bool,
) -> Grupo {
    let (yo, t) = (ctx.yo.to_string(), ctx.ahora_ms);
    let Some(col) = columna(ctx.puerto.observar(), &yo) else { return Grupo::Deshacer };
    let poder = poder_de_columna(d, ctx.puerto.observar(), &col);
    let fuerte = basta(poder);
    let unidos = col.participantes.len();
    ctx.con_pizarra(|p| p.tablero.anotar_fuerza(puerta, &yo, unidos, poder));
    let completo = cuantos > 0 && unidos >= cuantos;
    if ctx.con_pizarra(|p| p.tablero.reuniendo(puerta, &yo, t)) && !(completo && fuerte) {
        return Grupo::Esperar;
    }
    let datos = json!({"tarea": clave, "puerta": puerta, "objetivo": objetivo, "unidos": unidos, "fuerza": poder, "esperaMs": t - inicio});
    if fuerte {
        ctx.con_pizarra(|p| p.tablero.cerrar_reunion(puerta, &yo));
        evento(ctx, "grupo_sale", format!("sale de {puerta} para {clave} con {unidos} héroes y fuerza {poder}"), datos);
        return Grupo::Salir;
    }
    if t - inicio < tope && reunir(ctx, clave, puerta, &col.id, objetivo, espera_ms) {
        return Grupo::Esperar;
    }
    ctx.con_pizarra(|p| {
        p.tablero.cerrar_reunion(puerta, &yo);
        p.tablero.soltar(clave, &yo);
    });
    evento(ctx, "grupo_se_deshace", format!("se deshace en {puerta}: {unidos} héroes y fuerza {poder} no bastan para {clave}"), datos);
    Grupo::Deshacer
}

fn aceptada(r: &Option<pb::ResultadoOrden>) -> bool {
    r.as_ref().is_some_and(|r| r.resultado == Resultado::OrdenAceptada as i32 || r.resultado == Resultado::OrdenDuplicada as i32)
}

fn a_punto(p: Punto) -> Option<orden_marchar_a::Objetivo> {
    Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(p)) })
}

// ───────────────────────────── Comportamiento ─────────────────────────────

/// El Rey bot acepta toda solicitud de ingreso (§12) y apunta su Facción en la pizarra para que los demás la encuentren.
async fn rey<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>) -> Result<(), Fallo> {
    let Some(f) = ctx.faccion().and_then(|id| faccion(ctx.puerto.observar(), &id)) else { return Ok(()) };
    if f.rey_id.as_deref() != Some(ctx.yo) {
        return Ok(());
    }
    let (yo, id) = (ctx.yo.to_string(), f.id.clone());
    ctx.con_pizarra_en("facciones", |p| p.facciones.insert(yo, id));
    for h in &f.solicitudes_ids {
        let orden = O::ResponderSolicitud(pb::OrdenResponderSolicitud { faccion_id: f.id.clone(), heroe_id: h.clone(), aceptar: true });
        ctx.intentar(&format!("aceptar:{h}"), orden, 10 * 60_000).await?;
    }
    // La diplomacia, con plaza o sin ella: la Facción derrotada no tiene plazas y su Rey tiene que poder dejarse anexionar.
    plaza::diplomacia(ctx, &f.id).await
}

/// Las misiones: reclama lo cumplido que le toca (lo suyo; lo de la Facción, si es Rey; lo de su plaza, si la gobierna) y acepta las del tablón que pueda. La experiencia de Facción
/// sube su nivel, y de ese nivel depende el cupo de plazas que deja ascender.
async fn misiones<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, h: &pb::Heroe) -> Result<(), Fallo> {
    let cumplida = pb::EstadoMision::MisionCumplida as i32;
    let recurrente = pb::EstadoMisionRecurrente::RecurrenteCumplida as i32;
    let individual = pb::TipoMision::MisionIndividual as i32;
    let yo = ctx.yo.to_string();
    let mut reclamar: Vec<(String, String)> = h.misiones.iter().filter(|m| m.estado == cumplida).map(|m| (h.id.clone(), m.id.clone())).collect();
    let mut aceptar: Vec<(String, String)> = Vec::new();
    let t = ctx.ahora_ms;
    let mut tablones: Vec<(String, pb::Tablon, bool)> = Vec::new();
    for c in entidades(ctx.puerto.observar()) {
        match c {
            Cuerpo::Faccion(f) if Some(&f.id) == h.faccion_id.as_ref() => {
                let es_rey = f.rey_id.as_deref() == Some(yo.as_str());
                if es_rey {
                    f.misiones.iter().filter(|m| m.estado == cumplida).for_each(|m| reclamar.push((f.id.clone(), m.id.clone())));
                }
                tablones.extend(f.tablon.clone().map(|x| (f.id.clone(), x, es_rey)));
            }
            Cuerpo::Asentamiento(a) if h.residencia_id.as_deref() == Some(a.id.as_str()) => {
                let gobierna = a.cargos.as_ref().is_some_and(|x| x.gobernador_id.as_deref() == Some(yo.as_str()));
                if gobierna {
                    a.misiones.iter().filter(|m| m.estado == cumplida).for_each(|m| reclamar.push((a.id.clone(), m.id.clone())));
                }
                tablones.extend(a.tablon.clone().map(|x| (a.id.clone(), x, gobierna)));
            }
            _ => {}
        }
    }
    for (dueno, x, manda) in &tablones {
        for m in &x.misiones {
            let Some(def) = &m.definicion else { continue };
            if def.tipo == individual {
                match m.aceptaciones.iter().find(|a| a.heroe_id == yo) {
                    Some(a) if a.estado == recurrente => reclamar.push((dueno.clone(), def.id.clone())),
                    None if m.hasta_ms > t && def.cargo.is_none() && m.aceptadas_semana < def.limite_semanal => aceptar.push((dueno.clone(), def.id.clone())),
                    _ => {}
                }
            } else if *manda && m.estado == recurrente {
                reclamar.push((dueno.clone(), def.id.clone()));
            }
        }
    }
    for (dueno_id, mision_id) in reclamar {
        ctx.intentar(&format!("reclamar:{dueno_id}:{mision_id}"), O::ReclamarRecompensa(pb::OrdenReclamarRecompensa { dueno_id, mision_id }), 12 * ESPERA_TRAS_RECHAZO_MS).await?;
    }
    for (dueno_id, mision_id) in aceptar {
        ctx.intentar(&format!("aceptar:{dueno_id}:{mision_id}"), O::AceptarMision(pb::OrdenAceptarMision { dueno_id, mision_id }), 6 * ESPERA_TRAS_RECHAZO_MS).await?;
    }
    Ok(())
}

/// Mira el anillo alrededor de donde está y apunta en la pizarra los bandidos que ve.
async fn mirar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, h: &pb::Heroe) -> Result<(), Fallo> {
    let t = ctx.ahora_ms;
    let aqui = match h.ubicacion.as_ref().and_then(|u| u.donde.as_ref()) {
        Some(Donde::CampamentoId(k)) => campamento(ctx.puerto.observar(), k).and_then(|k| k.posicion),
        Some(Donde::AsentamientoId(p)) => entidades(ctx.puerto.observar()).find_map(|c| match c {
            Cuerpo::Asentamiento(a) if a.id == *p => a.posicion.clone(),
            _ => None,
        }),
        _ => columna(ctx.puerto.observar(), ctx.yo).map(|e| posicion(&e, t)),
    };
    let Some(p) = aqui else { return Ok(()) };
    let mira = d.anillo.1 + MARGEN_VISTA;
    // El mapa trae todos los yacimientos, pero la Facción solo cuenta los que han pasado a la vista de sus bots.
    let mapa = mapa::de(ctx.puerto).await?;
    // También el que su Facción tiene explorado aunque ningún bot lo viera con sus ojos (un rumor de la taberna, un jugador de la Facción): la niebla de la vista lo dice.
    let niebla = ctx.puerto.observar().ambitos.values().find_map(|x| x.niebla.clone());
    let explorado = |q: &Punto| niebla.as_ref().is_some_and(|n| explorado_en(n, q));
    // Los yacimientos que conoce la Facción: los que ve y los que tiene explorados. El primero que ve con sus ojos uno que la Facción no conocía lo descubre (un logro:
    // uno por yacimiento de estaño; los demás, uno por tipo y vistazo, con los puntos de todos).
    let conocidos: Vec<(String, String, Punto, bool)> = mapa
        .nodos
        .iter()
        .filter_map(|(id, tipo, q)| {
            let a_ojo = distancia(q, &p) <= mira;
            (a_ojo || explorado(q)).then(|| (id.clone(), tipo.clone(), q.clone(), a_ojo))
        })
        .collect();
    if !conocidos.is_empty() {
        let yo = ctx.yo.to_string();
        ctx.con_pizarra(|pz| {
            let mut descubiertos: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for (id, tipo, q, a_ojo) in &conocidos {
                if pz.nodos.contains_key(id) || (tipo == "estano" && pz.libro.estano_visto.contains_key(id)) {
                    continue;
                }
                pz.nodos.insert(id.clone(), (tipo.clone(), q.x, q.y, t));
                if tipo == "estano" {
                    pz.libro.estano_visto.insert(id.clone(), (q.x, q.y));
                }
                if *a_ojo {
                    descubiertos.entry(tipo.clone()).or_default().push(id.clone());
                }
            }
            for (tipo, ids) in descubiertos {
                let (nombre, puntos) = match tipo.as_str() {
                    "estano" => ("estaño", balance::puntos::ESTANO_DESCUBIERTO),
                    "cobre" => ("cobre", balance::puntos::COBRE_DESCUBIERTO),
                    "hierro" => ("hierro", balance::puntos::HIERRO_DESCUBIERTO),
                    "oro" => ("oro", balance::puntos::ORO_DESCUBIERTO),
                    "piedra" => ("piedra", balance::puntos::PIEDRA_DESCUBIERTA),
                    _ => continue,
                };
                let grupos: Vec<Vec<String>> = if tipo == "estano" { ids.into_iter().map(|i| vec![i]).collect() } else { vec![ids] };
                for g in grupos {
                    let texto = if g.len() == 1 { format!("descubre el yacimiento de {nombre} {}", g[0]) } else { format!("descubre {} yacimientos de {nombre}", g.len()) };
                    let l = Logro { en: t, tipo: format!("{tipo}_descubierto"), texto, clase: "exploracion".into(), puntos: puntos * g.len() as u64, logrador: Some(yo.clone()), participantes: Vec::new(), plaza: None };
                    apuntar_logro(pz, l);
                }
            }
        });
    }
    let r = pb::Rectangulo { min: Some(Punto { x: p.x - mira, y: p.y - mira }), max: Some(Punto { x: p.x + mira, y: p.y + mira }) };
    match ctx.puerto.mirar(r).await {
        Err(Fallo::Vista(_)) => return Ok(()), // sin derecho a ver: piensa con lo que tiene
        r => r?,
    }
    // Lo que ve queda visto en la rejilla de la Facción, salvo desde dentro de un campamento: desde ahí no se ven los bandidos.
    if !matches!(h.ubicacion.as_ref().and_then(|u| u.donde.as_ref()), Some(Donde::CampamentoId(_))) {
        ctx.con_pizarra(|pz| marcar_vistas(&mut pz.rejilla, &p, mira, t));
    }
    let vistos: Vec<_> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::CampamentoBandidos(b) => Some((b.id.clone(), (b.posicion.clone().unwrap_or_default(), b.poder, t))),
            _ => None,
        })
        .collect();
    // Lo que dicen los Informes comprados vale como una inspección.
    let informes: Vec<_> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Informe(x) => Some((x.asentamiento_id.clone(), (x.guarnicion.iter().map(|g| g.cantidad * d.poder.get(&g.tropa_id).copied().unwrap_or(0)).sum::<u32>(), x.heroes_ids.len(), x.conocido_en_ms))),
            _ => None,
        })
        .collect();
    let pueblos: Vec<_> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Pueblo(p) => Some((p.id.clone(), (p.asentamiento_madre_id.clone(), String::new(), p.posicion.clone()?, p.guarnicion_escuadron_id.is_some() || p.batalla_id.is_some()))),
            Cuerpo::FichaConocida(f) if f.asentamiento_id.starts_with("pueblo-") && !f.faccion_id.is_empty() => Some((f.asentamiento_id.clone(), (String::new(), f.faccion_id.clone(), f.posicion.clone()?, false))),
            _ => None,
        })
        .collect();
    ctx.con_pizarra(|pz| {
        pz.bandidos.retain(|_, (_, _, visto)| t - *visto <= balance::RECUERDO_BANDIDO_MS);
        pz.bandidos.extend(vistos);
        for (id, informe) in informes {
            if pz.defensas.get(&id).is_none_or(|(_, _, en)| *en < informe.2) {
                pz.defensas.insert(id, informe);
            }
        }
        for (id, (madre, faccion, p, defendido)) in pueblos {
            let e = pz.pueblos.entry(id).or_insert_with(|| (String::new(), String::new(), p.clone(), false));
            if !madre.is_empty() {
                (e.0, e.3) = (madre, defendido);
            }
            if !faccion.is_empty() {
                e.1 = faccion;
            }
            e.2 = p;
        }
    });
    Ok(())
}

async fn sin_plaza<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, h: pb::Heroe) -> Result<(), Fallo> {
    let dentro = match h.ubicacion.as_ref().and_then(|u| u.donde.as_ref()) {
        Some(Donde::CampamentoId(k)) => Some(k.clone()),
        _ => None,
    };
    let Some(faccion_id) = h.faccion_id.clone() else {
        if dentro.is_some() {
            buscar_faccion(ctx, &h).await?;
        }
        return Ok(());
    };
    ctx.memoria.solicitud = None;
    if mudarse(ctx, &faccion_id, &dentro).await? {
        return Ok(());
    }
    // Sin plaza tras varios intentos fallidos: deja su Facción, desde dentro del campamento y fuera de todo ejército (`faccion.en_ejercito`), para pedir entrar en una que ya tenga.
    if dentro.is_some() && columna(ctx.puerto.observar(), ctx.yo).is_none() {
        if let Some(destino) = rendirse(ctx, &faccion_id) {
            let yo = ctx.yo.to_string();
            ctx.con_pizarra(|p| p.tablero.soltar_todas(&yo));
            let r = ctx.intentar("dejar_faccion", O::DejarFaccion(pb::OrdenDejarFaccion {}), ESPERA_TRAS_RECHAZO_MS).await?;
            if aceptada(&r) {
                evento(ctx, "se_rinde", format!("deja {faccion_id}, que no logra fundar, para pedir entrar en {destino}"), json!({"destino": destino}));
                ctx.memoria.destino = Some(destino);
                ctx.memoria.plan = None;
            }
            return Ok(());
        }
    }
    let Some(casa) = h.residencia_id.clone().filter(|r| r.starts_with("camp-")) else { return Ok(()) };
    match dentro {
        Some(_) => en_el_campamento(ctx, d, &casa).await,
        None => en_campo(ctx, d, &h, &casa).await,
    }
}

/// D57: el líder de los amigos y el solitario crean la suya; los demás piden entrar (los amigos, a la de su líder) y, sin respuesta en 2 h, crean la suya.
async fn buscar_faccion<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, h: &pb::Heroe) -> Result<(), Fallo> {
    let crear = |nombre: String| O::CrearFaccion(pb::OrdenCrearFaccion { nombre, sigilo: None });
    // Quien dejó su Facción para entrar en otra no funda la suya: si no le contestan a tiempo, prueba con otra.
    if ctx.memoria.destino.is_some() && ctx.memoria.solicitud.as_ref().is_some_and(|(_, desde)| ctx.ahora_ms - desde >= ESPERA_SOLICITUD_MS) {
        (ctx.memoria.destino, ctx.memoria.solicitud) = (None, None);
    }
    let funda = ctx.memoria.destino.is_none() && match (&ctx.memoria.solicitud, ctx.perfil) {
        (Some((_, desde)), _) => ctx.ahora_ms - desde >= ESPERA_SOLICITUD_MS,
        (None, Perfil::Solitario) => true,
        (None, Perfil::Amigos { lider }) => lider == ctx.yo,
        (None, _) => false,
    };
    if funda {
        ctx.memoria.solicitud = None;
        let nombre = format!("Casa de {}", h.display_name);
        let r = ctx.intentar("crear_faccion", crear(nombre.clone()), ESPERA_TRAS_RECHAZO_MS).await?;
        // Otro ya se llama así (restos de otra corrida en la misma partida): se distingue por el id.
        if r.as_ref().and_then(|r| r.rechazo.as_ref()).is_some_and(|x| x.codigo.contains("nombre")) {
            ctx.puerto.actuar(crear(format!("{nombre} ({})", ctx.yo))).await?;
        }
        return Ok(());
    }
    if ctx.memoria.solicitud.is_some() {
        return Ok(());
    }
    let lider = match ctx.perfil {
        Perfil::Amigos { lider } => Some(lider.clone()),
        _ => None,
    };
    let destino = match (ctx.memoria.destino.clone(), ctx.perfil) {
        (Some(d), _) => Some(d),
        (None, Perfil::Refuerzo { faccion }) => Some(faccion.clone()),
        (None, _) => ctx.con_pizarra_en("facciones", |p| match &lider {
            Some(l) => p.facciones.get(l).cloned(),
            // Un tardío va a la Facción con menos gente: repartidos, todas pueden juntar un grupo que funde.
            None => {
                let f = p.facciones.values().min_by_key(|f| (p.ingresos.get(*f).copied().unwrap_or(0), (*f).clone())).cloned();
                if let Some(f) = &f {
                    *p.ingresos.entry(f.clone()).or_default() += 1;
                }
                f
            }
        }),
    };
    let Some(destino) = destino else { return Ok(()) }; // el líder aún no la ha creado
    let r = ctx.intentar("solicitar_ingreso", O::SolicitarIngreso(pb::OrdenSolicitarIngreso { faccion_id: destino.clone() }), 10 * 60_000).await?;
    if aceptada(&r) {
        ctx.memoria.solicitud = Some((destino, ctx.ahora_ms));
    }
    Ok(())
}

/// ¿Se rinde su Facción? Sin ninguna plaza suya a la vista y con bastantes fracasos al fundar, la Facción de bots que ya tenga plaza y más plazas se le conozcan.
fn rendirse<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, mia: &str) -> Option<String> {
    let a = ctx.puerto.observar();
    if entidades(a).any(|c| matches!(c, Cuerpo::Asentamiento(x) if x.faccion_id == mia)) || ctx.con_pizarra(|p| p.fracasos_fundar) < balance::FRACASOS_PARA_RENDIRSE {
        return None;
    }
    let de_bots: Vec<String> = ctx.con_pizarra_en("facciones", |p| p.facciones.values().cloned().collect());
    let mut plazas: BTreeMap<String, usize> = BTreeMap::new();
    for f in ctx.con_pizarra(|p| p.ajenas.values().map(|(f, _)| f.clone()).collect::<Vec<_>>()) {
        *plazas.entry(f).or_default() += 1;
    }
    for c in entidades(a) {
        if let Cuerpo::Asentamiento(x) = c {
            plazas.entry(x.faccion_id.clone()).or_insert(1);
        }
    }
    plazas.into_iter().filter(|(f, _)| f != mia && de_bots.contains(f)).max_by_key(|(f, n)| (*n, f.clone())).map(|(f, _)| f)
}

/// Le ha salido mal fundar: lo apunta su Facción.
fn fracaso_al_fundar<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>) {
    ctx.con_pizarra(|p| p.fracasos_fundar += 1);
}

/// Si su Facción tiene una plaza que conoce, va a vivir a ella: sale del campamento con lo suyo, cambia de residencia y marcha.
async fn mudarse<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, faccion_id: &str, dentro: &Option<String>) -> Result<bool, Fallo> {
    if matches!(ctx.memoria.plan, Some(Plan::Mudarse)) {
        return Ok(true);
    }
    // La de su Facción con menos residentes, según la pizarra (§16: reparte sus héroes entre sus plazas).
    let plazas: Vec<String> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Asentamiento(a) if a.faccion_id == faccion_id => Some(a.id.clone()),
            _ => None,
        })
        .collect();
    let plaza = ctx.con_pizarra(|p| plazas.iter().map(|id| (p.residencias.values().filter(|r| *r == id).count(), id.clone())).min().map(|(_, id)| id));
    let Some(plaza) = plaza else { return Ok(false) };
    if let Some(k) = dentro {
        if !salir(ctx, k, false).await? {
            return Ok(false);
        }
    }
    let r = ctx.intentar(&format!("mudarse:{plaza}"), O::CambiarResidencia(pb::OrdenCambiarResidencia { destino_id: plaza.clone(), heroe_id: ctx.yo.into() }), ESPERA_TRAS_RECHAZO_MS).await?;
    if !aceptada(&r) {
        return Ok(false);
    }
    let objetivo = Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::AsentamientoId(plaza)) });
    ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: ctx.yo.into(), objetivo })).await?;
    ctx.memoria.plan = Some(Plan::Mudarse);
    Ok(true)
}

/// Sale del campamento, con su tropa (`con_tropa`) o sin ella; el rumbo se da después con `marchar_a`, como el batch.
async fn salir<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, campamento: &str, con_tropa: bool) -> Result<bool, Fallo> {
    salir_con(ctx, campamento, con_tropa, None, None).await
}

/// Como [`salir`], con la política de unión de su columna (`aceptar`: los compañeros se le unen en la puerta) y la Caravana de Fundación que espera dentro y se lleva.
async fn salir_con<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, campamento: &str, con_tropa: bool, politica: Option<&str>, caravana: Option<&str>) -> Result<bool, Fallo> {
    let escuadron_ids = if con_tropa { crate::plaza::lo_que_cabe(ctx.puerto.observar(), ctx.yo, &*datos::de(ctx.puerto).await?, &tropa_en_campamento(ctx.puerto.observar(), ctx.yo)) } else { vec![] };
    let politica_de_union = politica.map(String::from);
    let o = O::SalirDelCampamento(pb::OrdenSalirDelCampamento { campamento_id: campamento.into(), heroe_id: ctx.yo.into(), escuadron_ids, carga: None, politica_de_union, objetivo: None, caravana_fundacion_id: caravana.map(String::from) });
    let sale = aceptada(&ctx.intentar(&format!("salir:{campamento}"), o, 10 * 60_000).await?);
    if sale {
        let yo = ctx.yo.to_string();
        ctx.con_pizarra(|p| p.listos.remove(&yo));
    }
    Ok(sale)
}

/// Dentro de su campamento: la leva, y salir a cazar lo que puede vencer o, sin presa conocida, a explorar el anillo.
async fn en_el_campamento<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, casa: &str) -> Result<(), Fallo> {
    ctx.memoria.en_casa = None;
    // Dentro no lleva nada en curso: lo que tuviera en el tablero se suelta ya.
    {
        let yo = ctx.yo.to_string();
        ctx.con_pizarra(|p| p.tablero.soltar_todas(&yo));
    }
    let a = ctx.puerto.observar();
    let leva: Vec<u32> = tropa_en_campamento(a, ctx.yo).iter().filter(|e| e.prestada_por_campamento_id.is_some()).map(|e| e.cantidad).collect();
    let lanceros: u32 = leva.iter().sum();
    ctx.memoria.leva_max = ctx.memoria.leva_max.max(lanceros);
    if leva.is_empty() {
        ctx.intentar("prestamo", O::PedirPrestamo(pb::OrdenPedirPrestamo { tropa_ids: vec![PRESTAMO.into()] }), ESPERA_TRAS_RECHAZO_MS).await?;
    } else if lanceros < ctx.memoria.leva_max {
        ctx.intentar("reponer", O::ReponerPrestamo(pb::OrdenReponerPrestamo {}), ESPERA_TRAS_RECHAZO_MS).await?;
    }
    let a = ctx.puerto.observar();
    let Some(k) = campamento(a, casa) else { return Ok(()) };
    let puerta = k.posicion.clone().unwrap_or_default();
    // Su Facción aún no tiene plaza (si la tuviera, ya se habría mudado): el fondo, la caravana y fundar.
    let poder: u32 = tropa_en_campamento(a, ctx.yo).iter().map(|e| d.poder_de(e)).sum();
    let t = ctx.ahora_ms;
    let yo = ctx.yo.to_string();
    let caravana = caravana_propia(a, ctx.yo);
    if let Some(caravana) = &caravana {
        // Solo sale si el grupo puede con lo que hay en el anillo; si no, sigue cazando para que haya menos bandidos y más gente lista.
        let fuerza = ctx.con_pizarra(|p| {
            let juntos = poder + p.listos.iter().filter(|(h, (k, _, hasta))| k == casa && *hasta > t && **h != yo).map(|(_, (_, x, _))| *x).sum::<u32>();
            // Quien sale del campamento juzga por lo que ve ahora; la memoria larga es para los caminos.
            let peor = p.bandidos.values().filter(|(pos, _, visto)| t - *visto <= VIGENCIA_BANDIDO_MS && distancia(pos, &puerta) <= d.anillo.1).map(|(_, x, _)| *x).max().unwrap_or(0);
            juntos as f64 >= peor as f64 * balance::MARGEN_PARA_FUNDAR
        });
        if fuerza && poder > 0 {
            return salir_a_fundar(ctx, casa, caravana).await;
        }
    } else if let Some(f) = heroe(a, ctx.yo).and_then(|h| h.faccion_id) {
        if aportar_y_comprar(ctx, d, &k, &f).await? {
            return Ok(());
        }
    }
    // Un compañero acaba de salir y le espera en la puerta: se le une.
    let salida = ctx.con_pizarra(|p| p.tablero.a_unirse(casa, &yo, t).map(|(e, _)| e));
    if let Some(ejercito_id) = salida {
        if poder > 0 && salir(ctx, casa, true).await? {
            let r = ctx.puerto.actuar(O::UnirseEnCampo(pb::OrdenUnirseEnCampo { ejercito_id: ejercito_id.clone(), heroe_id: yo.clone() })).await?;
            ctx.memoria.plan = (r.resultado == Resultado::OrdenAceptada as i32).then_some(Plan::Unirse { salio: false });
            if ctx.memoria.plan.is_some() {
                evento(ctx, "se_une", format!("se une a {ejercito_id} en {casa}"), json!({"columna": ejercito_id, "puerta": casa}));
            }
        }
        return Ok(());
    }
    // Los que están dentro y listos, con su poder: juntos pueden con más.
    let grupo: Vec<(String, u32)> = ctx.con_pizarra(|p| {
        if poder > 0 {
            p.listos.insert(yo.clone(), (casa.into(), poder, t + balance::LISTO_MS));
        }
        p.listos.retain(|_, (_, _, hasta)| *hasta > t);
        p.listos.iter().filter(|(_, (k, _, _))| k == casa).map(|(h, (_, poder, _))| (h.clone(), *poder)).collect()
    });
    let juntos: u32 = grupo.iter().map(|(_, p)| p).sum();
    // La presa: el bandido apuntado del anillo más cercano que pueden vencer los que están listos y que no tenga ya otro de la Facción.
    let presa = ctx.con_pizarra(|p| {
        let (tablero, vetos) = (&p.tablero, &p.vetos);
        p.bandidos
            .iter()
            .filter(|(id, (pos, poder_b, visto))| {
                t - *visto <= VIGENCIA_BANDIDO_MS
                    && *poder_b < juntos
                    && distancia(pos, &puerta) <= d.anillo.1
                    && tablero.libre(&format!("cazar:{id}"), &yo, t)
                    && vetos.get(&format!("bandido:{id}")).is_none_or(|&h| h <= t)
            })
            .min_by_key(|(id, (pos, _, _))| (distancia(pos, &puerta), (*id).clone()))
            .map(|(id, (pos, poder_b, _))| (id.clone(), pos.clone(), *poder_b))
    });
    if let Some((bandidos, sitio, poder_b)) = presa {
        if poder_b < poder {
            // Solo puede: la reserva antes de salir (los compañeros piensan a la vez: con la reserva después, salían varios a por la misma) y sale derecho a ella.
            reservar(ctx, &bandidos);
            if salir(ctx, casa, true).await? {
                ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: yo, objetivo: a_punto(sitio) })).await?;
                ctx.memoria.plan = Some(Plan::Cazar { bandidos });
            } else {
                ctx.con_pizarra(|p| p.tablero.soltar(&format!("cazar:{bandidos}"), &yo));
            }
        } else if grupo.first().is_some_and(|(h, _)| *h == yo) && salir_con(ctx, casa, true, Some("aceptar"), None).await? {
            // Lleva al grupo el de menor id: sale, espera en la puerta y los demás se le unen al pensar.
            if let Some(col) = columna(ctx.puerto.observar(), &yo) {
                reservar(ctx, &bandidos);
                reunir(ctx, &format!("cazar:{bandidos}"), casa, &col.id, &bandidos, ESPERA_GRUPO_MS);
                ctx.memoria.plan = Some(Plan::Esperar { bandidos, cuantos: grupo.len(), desde: t });
            }
        }
        return Ok(());
    }
    // Nadie del anillo sabe de una presa que se pueda: si nadie lo ha recorrido hace poco, uno sale sin tropa (no come) a dar la vuelta, y el siguiente cuando acabe. Toma la
    // vuelta antes de salir: los compañeros piensan a la vez y, con la tarea tomada después, salían todos.
    let yo = ctx.yo.to_string();
    let vuelta = format!("explorar:{casa}");
    let toca = ctx.con_pizarra(|p| p.explorado.get(casa).is_none_or(|&en| t - en > balance::EXPLORAR_ANILLO_CADA_MS) && p.tablero.tomar(&vuelta, &yo, t, t + balance::TAREA_VALE_MS));
    if toca {
        if salir(ctx, casa, false).await? {
            let inicio = (t / 60_000) as u32 % VUELTA;
            ctx.memoria.plan = Some(Plan::Explorar { inicio, paso: inicio });
            return Ok(());
        }
        ctx.con_pizarra(|p| p.tablero.soltar(&vuelta, &yo));
    }
    let conocidos = ctx.con_pizarra(|p| p.bandidos.values().filter(|(q, _, visto)| t - *visto <= VIGENCIA_BANDIDO_MS && distancia(q, &puerta) <= d.anillo.1).count());
    ctx.memoria.en_casa = Some(if conocidos == 0 {
        "En el campamento: no conoce bandidos en el anillo y otro lo recorre o lo recorrió hace poco".into()
    } else {
        format!("En el campamento: {conocidos} campamentos de bandidos en el anillo, ninguno libre que puedan vencer los {} listos (fuerza {juntos})", grupo.len())
    });
    Ok(())
}

/// Fuera: abre los alijos que pisa y sigue su plan; sin plan, vuelve a casa.
async fn en_campo<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, h: &pb::Heroe, casa: &str) -> Result<(), Fallo> {
    let a = ctx.puerto.observar();
    let Some(col) = columna(a, ctx.yo) else { return Ok(()) };
    let Some(puerta) = campamento(a, casa).and_then(|k| k.posicion) else { return Ok(()) };
    let aqui = posicion(&col, ctx.ahora_ms);
    let parada = col.estado == EstadoColumna::ColumnaEstacionada as i32;
    let alijos: Vec<String> = entidades(a)
        .filter_map(|c| match c {
            Cuerpo::Alijo(x) if x.posicion.as_ref().is_some_and(|p| distancia(p, &aqui) <= d.radio_puerta) && !h.alijos_abiertos.contains(&x.id) => Some(x.id.clone()),
            _ => None,
        })
        .collect();
    for alijo in alijos {
        ctx.intentar(&format!("alijo:{alijo}"), O::AbrirAlijo(pb::OrdenAbrirAlijo { alijo_id: alijo.clone() }), ESPERA_TRAS_RECHAZO_MS).await?;
    }
    let yo = ctx.yo.to_string();
    if col.lider_id != yo {
        // Su Facción ya tiene plaza (la fundó quien llevaba la columna): se separa y sigue por su cuenta (vuelve y se muda).
        let f = ctx.faccion();
        let con_plaza = f.is_some() && entidades(ctx.puerto.observar()).any(|c| matches!(c, Cuerpo::Asentamiento(x) if Some(&x.faccion_id) == f.as_ref()));
        if con_plaza {
            ctx.puerto.actuar(O::SepararseDelEjercito(pb::OrdenSepararseDelEjercito { heroe_id: yo })).await?;
            ctx.memoria.plan = None;
            return Ok(());
        }
        // Va donde va quien la lleva; de vuelta en la puerta, se separa y entra (se entra solo).
        let lejos = distancia(&puerta, &aqui) > d.radio_puerta;
        let salio = matches!(ctx.memoria.plan, Some(Plan::Unirse { salio: true })) || lejos;
        if salio && !lejos && parada {
            ctx.puerto.actuar(O::SepararseDelEjercito(pb::OrdenSepararseDelEjercito { heroe_id: yo.clone() })).await?;
            ctx.puerto.actuar(O::EntrarEnCampamento(pb::OrdenEntrarEnCampamento { campamento_id: casa.into(), heroe_id: yo })).await?;
            ctx.memoria.plan = None;
        } else {
            ctx.memoria.plan = Some(Plan::Unirse { salio });
        }
        return Ok(());
    }
    let marchar = |p: Punto| O::MarcharA(pb::OrdenMarcharA { heroe_id: yo.clone(), objetivo: a_punto(p) });
    match ctx.memoria.plan.take() {
        Some(Plan::Esperar { bandidos, cuantos, desde }) => {
            // Sale cuando lo que lleva de verdad puede con ella; si no se junta bastante a tiempo, se deshace y vuelve a entrar.
            let presa = ctx.con_pizarra(|p| p.bandidos.get(&bandidos).cloned());
            let clave = format!("cazar:{bandidos}");
            let decision = match &presa {
                Some((_, poder_b, _)) => decidir_grupo(ctx, d, &clave, casa, &bandidos, desde, balance::ESPERA_GRUPO_TOPE_MS, ESPERA_GRUPO_MS, cuantos, |poder| poder > *poder_b),
                None => Grupo::Deshacer,
            };
            match (decision, presa) {
                (Grupo::Esperar, _) => ctx.memoria.plan = Some(Plan::Esperar { bandidos, cuantos, desde }),
                (Grupo::Salir, Some((s, _, _))) => {
                    ctx.puerto.actuar(marchar(s)).await?;
                    ctx.memoria.plan = Some(Plan::Cazar { bandidos });
                }
                _ => {
                    let t = ctx.ahora_ms;
                    ctx.con_pizarra(|p| {
                        p.tablero.cerrar_reunion(casa, &yo);
                        p.tablero.soltar(&clave, &yo);
                        p.vetos.insert(format!("bandido:{bandidos}"), t + balance::VETO_TRAS_DESHACER_MS);
                    });
                    volver(ctx, d, casa, &col, &puerta, &aqui, parada).await?
                }
            }
        }
        Some(Plan::Explorar { inicio, paso }) if !parada => ctx.memoria.plan = Some(Plan::Explorar { inicio, paso }),
        Some(Plan::Explorar { inicio, paso }) if paso >= inicio + VUELTA => {
            // Vuelta acabada: el anillo queda explorado para la pizarra, y a casa.
            let t = ctx.ahora_ms;
            ctx.con_pizarra(|p| p.explorado.insert(casa.into(), t));
            volver(ctx, d, casa, &col, &puerta, &aqui, parada).await?;
        }
        Some(Plan::Explorar { inicio, paso }) => {
            // El siguiente punto de la vuelta, a mitad del anillo.
            let radio = (d.anillo.0 + d.anillo.1) as f64 / 2.0;
            let angulo = paso as f64 * std::f64::consts::TAU / VUELTA as f64;
            let p = Punto { x: puerta.x + (radio * angulo.cos()) as i64, y: puerta.y + (radio * angulo.sin()) as i64 };
            // Un punto en el agua o fuera del mapa se rechaza: se salta.
            ctx.puerto.actuar(marchar(p)).await?;
            ctx.memoria.plan = Some(Plan::Explorar { inicio, paso: paso + 1 });
        }
        Some(Plan::Cazar { bandidos }) => {
            let sitio = ctx.con_pizarra(|p| p.bandidos.get(&bandidos).map(|b| b.0.clone()));
            // En el sitio y sin campamento a la vista: ya no existe.
            let lo_ve = entidades(ctx.puerto.observar()).any(|c| matches!(c, Cuerpo::CampamentoBandidos(b) if b.id == bandidos));
            let sitio = sitio.filter(|s| lo_ve || distancia(s, &aqui) > d.radio_encuentro);
            if sitio.is_none() {
                ctx.con_pizarra(|p| p.bandidos.remove(&bandidos));
            }
            match sitio {
                Some(s) if parada && distancia(&s, &aqui) <= d.radio_encuentro => {
                    let objetivo = Some(orden_atacar::Objetivo { tipo: Some(orden_atacar::objetivo::Tipo::CampamentoId(bandidos.clone())) });
                    let r = ctx.puerto.actuar(O::Atacar(pb::OrdenAtacar { heroe_id: ctx.yo.into(), objetivo, declara_guerra: None })).await?;
                    // Atacado, o ya no existe (lo destruyó otro): fuera de la pizarra, o todos volverían a por él una y otra vez.
                    if r.resultado == Resultado::OrdenAceptada as i32 || r.rechazo.as_ref().is_some_and(|x| x.codigo == "bandidos.desconocido") {
                        ctx.con_pizarra(|p| p.bandidos.remove(&bandidos));
                    }
                    ctx.con_pizarra(|p| p.tablero.soltar(&format!("cazar:{bandidos}"), &yo));
                    ctx.memoria.plan = Some(Plan::Volver);
                    ctx.puerto.actuar(marchar(puerta)).await?;
                }
                // De camino: sigue (su columna ya va) y la sigue teniendo; si el bandido ya no está apuntado, vuelve.
                Some(_) if !parada || poder_en_columna(d, ctx.puerto.observar(), ctx.yo, &col.id) > 0 => {
                    reservar(ctx, &bandidos);
                    if parada {
                        ctx.puerto.actuar(marchar(sitio.unwrap())).await?;
                    }
                    ctx.memoria.plan = Some(Plan::Cazar { bandidos });
                }
                _ => {
                    ctx.con_pizarra(|p| p.tablero.soltar(&format!("cazar:{bandidos}"), &yo));
                    ctx.memoria.plan = Some(Plan::Volver);
                    ctx.puerto.actuar(marchar(puerta)).await?;
                }
            }
        }
        Some(Plan::Mudarse) => ctx.memoria.plan = Some(Plan::Mudarse),
        Some(Plan::Fundar { sitio, intentos }) => fundar(ctx, d, sitio, intentos, aqui, parada).await?,
        Some(Plan::EsperarFundar { cuantos, desde, inicio }) if col.participantes.len() < cuantos && ctx.ahora_ms - desde < ESPERA_GRUPO_MS => {
            ctx.memoria.plan = Some(Plan::EsperarFundar { cuantos, desde, inicio });
        }
        Some(Plan::EsperarFundar { inicio, .. }) => {
            // Sale con lo que de verdad lleva la columna, no con lo que prometían los listos: si no puede con el camino, sigue esperando a más compañeros, hasta un tope.
            // ponytail: supone que cada compañero trae lo mismo que él (la tropa de los demás no siempre está a la vista).
            let t = ctx.ahora_ms;
            let poder = (poder_en_columna(d, ctx.puerto.observar(), &yo, &col.id) + d.poder_por_heroe) * col.participantes.len().max(1) as u32;
            if !marchar_a_fundar(ctx, d, &puerta, poder).await? {
                if t - inicio < balance::ESPERA_PARA_FUNDAR_MS {
                    reunir(ctx, &format!("fundar:{casa}"), casa, &col.id, "fundar", ESPERA_GRUPO_MS);
                    let cuantos = col.participantes.len() + 1;
                    ctx.memoria.plan = Some(Plan::EsperarFundar { cuantos, desde: t, inicio });
                } else {
                    ctx.con_pizarra(|p| {
                        p.tablero.cerrar_reunion(casa, &yo);
                        p.tablero.soltar(&format!("fundar:{casa}"), &yo);
                    });
                    fracaso_al_fundar(ctx);
                }
            } else {
                ctx.con_pizarra(|p| p.tablero.cerrar_reunion(casa, &yo));
            }
        }
        _ => volver(ctx, d, casa, &col, &puerta, &aqui, parada).await?,
    }
    Ok(())
}

/// Dice a su Facción que esa presa es suya.
fn reservar<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, bandidos: &str) {
    let (yo, t) = (ctx.yo.to_string(), ctx.ahora_ms);
    ctx.con_pizarra(|p| p.tablero.tomar(&format!("cazar:{bandidos}"), &yo, t, t + balance::CAZA_RESERVADA_MS));
}

/// A casa: en la puerta, entra y vacía el carro en su almacén personal; si no, marcha a ella.
async fn volver<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, casa: &str, col: &pb::Ejercito, puerta: &Punto, aqui: &Punto, parada: bool) -> Result<(), Fallo> {
    let yo = ctx.yo.to_string();
    ctx.con_pizarra(|p| p.tablero.soltar(&format!("explorar:{casa}"), &yo));
    if distancia(puerta, aqui) > d.radio_puerta {
        ctx.memoria.plan = Some(Plan::Volver);
        if parada {
            ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: yo, objetivo: a_punto(puerta.clone()) })).await?;
        }
        return Ok(());
    }
    // Con compañeros en la columna, espera a que se separen: el último entra.
    if parada && col.participantes.len() > 1 {
        ctx.memoria.plan = Some(Plan::Volver);
    } else if parada {
        ctx.puerto.actuar(O::EntrarEnCampamento(pb::OrdenEntrarEnCampamento { campamento_id: casa.into(), heroe_id: yo.clone() })).await?;
        ctx.puerto.actuar(O::VaciarCarro(pb::OrdenVaciarCarro { heroe_id: yo })).await?;
    }
    Ok(())
}

// ───────────────────────────── Fundar ─────────────────────────────

/// Su Caravana de Fundación, disponible para sacarla.
fn caravana_propia(a: &Almacen, yo: &str) -> Option<String> {
    entidades(a).find_map(|c| match c {
        Cuerpo::Caravana(x) if x.tipo == pb::TipoCaravana::CaravanaDeFundacion as i32 && x.titular_id.as_deref() == Some(yo) && x.estado == pb::EstadoCaravana::CaravanaDisponible as i32 => {
            Some(x.id.clone())
        }
        _ => None,
    })
}

/// El fondo de su Facción en este campamento (D39): si ya alcanza, compra la caravana; si no, pone lo que falta de lo que tiene, comprando en el mercado con el oro de su almacén
/// personal lo que no tenga, y el oro al final. Devuelve si compró la caravana.
async fn aportar_y_comprar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, k: &pb::CampamentoMercenarios, faccion: &str) -> Result<bool, Fallo> {
    // Una caravana de su Facción a la vez: si ya la tiene otro, no paga otra.
    if entidades(ctx.puerto.observar()).any(|c| matches!(c, Cuerpo::Caravana(x) if x.tipo == pb::TipoCaravana::CaravanaDeFundacion as i32 && x.faccion_id.as_deref() == Some(faccion))) {
        return Ok(false);
    }
    let fondo = k.fondos.get(faccion).map(|c| c.por_id.clone()).unwrap_or_default();
    let falta = |r: &str, n: i64| (n - fondo.get(r).copied().unwrap_or(0)).max(0);
    let mut coste = d.costo_refundacion.clone();
    coste.sort_by_key(|(r, _)| r == "oro");
    if coste.iter().all(|(r, n)| falta(r, *n) == 0) {
        let r = ctx.intentar(&format!("caravana:{}", k.id), O::ComprarCaravanaDeRefundacion(pb::OrdenComprarCaravanaDeRefundacion {}), 10 * 60_000).await?;
        return Ok(aceptada(&r));
    }
    let materiales_listos = coste.iter().all(|(r, n)| r == "oro" || falta(r, *n) == 0);
    for (recurso, n) in &coste {
        let recurso = recurso.as_str();
        let pide = falta(recurso, *n);
        // El oro paga primero los materiales en el mercado; al fondo, el que sobre cuando ya estén.
        if pide == 0 || (recurso == "oro" && !materiales_listos) {
            continue;
        }
        if recurso != "oro" && tiene(ctx, recurso) < pide && tiene(ctx, "oro") > 0 {
            let compra = pb::OrdenComprarEnCampamento { pagar_con: None, recurso: recurso.into(), cantidad: pide - tiene(ctx, recurso), campamento_id: Some(k.id.clone()) };
            ctx.intentar(&format!("comprar:{recurso}"), O::ComprarEnCampamento(compra), 60 * 60_000).await?;
        }
        let pon = pide.min(tiene(ctx, recurso));
        if pon > 0 {
            let aporte = pb::OrdenAportarARefundacion { recurso: recurso.into(), cantidad: pon, lado: "almacen".into() };
            ctx.intentar(&format!("aportar:{recurso}"), O::AportarARefundacion(aporte), 10 * 60_000).await?;
        }
    }
    Ok(false)
}

/// Lo que tiene de `recurso` en su almacén personal, en unidades enteras (en milésimas).
fn tiene<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, recurso: &str) -> i64 {
    heroe(ctx.puerto.observar(), ctx.yo).and_then(|h| h.almacen_personal).and_then(|c| c.por_id.get(recurso).copied()).unwrap_or(0) / UNIDAD * UNIDAD
}

/// Un sitio fundable al azar a entre `radios` de `centro`, con el mapa público y las plazas que conoce; `None` si no da con ninguno.
pub(crate) async fn sitio_para_fundar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, centro: &Punto, radios: (i64, i64)) -> Result<Option<Punto>, Fallo> {
    let mapa = mapa::de(ctx.puerto).await?;
    let plazas: Vec<Punto> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Asentamiento(a) => a.posicion.clone(),
            Cuerpo::FichaConocida(f) => f.posicion.clone(),
            _ => None,
        })
        .collect();
    // Mejor un sitio verde (bosque y piedra: sin bosque la plaza no paga el mantenimiento y cae en ruinas) y, de los verdes, el que más vale por los yacimientos que la
    // Facción conoce en su zona y necesita ([`valor_del_sitio`]); sin ninguno que valga, uno verde con cobre en su zona (sin cobre no hay fundición ni bronce); si no,
    // verde; si no aparece, uno que solo sea fundable.
    let candidatos: Vec<Punto> = (0..128).map(|_| punto_en_anillo(ctx, centro, radios)).filter(|p| mapa.fundable(p, &plazas, d.separacion_plazas, d.exclusion_campamento, d.bosque_denso)).collect();
    let verdes: Vec<&Punto> = candidatos.iter().filter(|p| mapa.viable(p, d.radio_inicial, d.techo_nivel_1)).collect();
    let f = ctx.faccion().unwrap_or_default();
    let mias: Vec<String> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Asentamiento(a) if a.faccion_id == f => Some(a.id.clone()),
            _ => None,
        })
        .collect();
    let sacados: Vec<String> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Pueblo(p) if mias.contains(&p.asentamiento_madre_id) => mapa.nodos.iter().find(|(id, ..)| *id == p.nodo_id).map(|(_, tipo, _)| tipo.clone()),
            _ => None,
        })
        .collect();
    let conocidos: Vec<(String, Punto)> = ctx.con_pizarra(|p| p.nodos.values().map(|(tipo, x, y, _)| (tipo.clone(), Punto { x: *x, y: *y })).collect());
    let mejor = verdes.iter().map(|p| (valor_del_sitio(p, &conocidos, &sacados), *p)).filter(|(v, _)| *v > 0.0).max_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, p)| p);
    let con_cobre = |p: &&&Punto| mapa.nodos.iter().any(|(_, tipo, q)| tipo == "cobre" && distancia(q, p) <= balance::NODO_EN_ZONA);
    Ok(mejor.or(verdes.iter().find(con_cobre).copied()).or(verdes.first().copied()).or(candidatos.first()).cloned())
}

/// Lo que vale fundar en `sitio` por los yacimientos `conocidos` (tipo y dónde) a [`balance::NODO_EN_ZONA`]: cada tipo una vez, con su peso
/// ([`balance::PESO_NODO_PARA_FUNDAR`]) entre 1 + los pueblos que la Facción ya saca de él (`sacados`, un tipo por pueblo).
fn valor_del_sitio(sitio: &Punto, conocidos: &[(String, Punto)], sacados: &[String]) -> f64 {
    let mut tipos: Vec<&str> = conocidos.iter().filter(|(_, q)| distancia(q, sitio) <= balance::NODO_EN_ZONA).map(|(t, _)| t.as_str()).collect();
    tipos.sort();
    tipos.dedup();
    tipos
        .into_iter()
        .filter_map(|t| balance::PESO_NODO_PARA_FUNDAR.iter().find(|(x, _)| *x == t).map(|(_, peso)| peso / (1 + sacados.iter().filter(|s| *s == t).count()) as f64))
        .sum()
}

/// Entre los sitios para fundar que encuentra y a los que `poder` llega sin que lo pare el peor bandido del camino, el del camino con menos bandidos conocidos a su paso.
async fn sitio_seguro<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, puerta: &Punto, poder: u32) -> Result<Option<Punto>, Fallo> {
    let anillo = (d.anillo.0.max(d.exclusion_campamento), d.anillo.1 + balance::FUNDAR_MAS_ALLA);
    let mut mejor: Option<(usize, Punto)> = None;
    for _ in 0..balance::SITIOS_A_COMPARAR {
        let Some(s) = sitio_para_fundar(ctx, d, puerta, anillo).await? else { continue };
        if !camino_seguro(ctx, poder, puerta, &s) {
            continue;
        }
        let peligro = ctx.con_pizarra(|p| p.bandidos.values().filter(|(q, _, _)| cerca_del_camino(q, puerta, &s, balance::RADIO_ACOSO)).count());
        if mejor.as_ref().is_none_or(|(n, _)| peligro < *n) {
            mejor = Some((peligro, s));
        }
    }
    Ok(mejor.map(|(_, s)| s))
}

/// ¿Puede `poder` con el peor campamento que conoce a lo largo del camino de `a` a `b`?
pub(crate) fn camino_seguro<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, poder: u32, a: &Punto, b: &Punto) -> bool {
    let peor = ctx.con_pizarra(|p| p.bandidos.values().filter(|(q, _, _)| cerca_del_camino(q, a, b, balance::ANCHO_DEL_CAMINO)).map(|(_, x, _)| *x).max().unwrap_or(0));
    poder as f64 >= peor as f64 * balance::MARGEN_PARA_FUNDAR
}

/// ¿Está `q` a menos de `radio` del tramo de `a` a `b`?
fn cerca_del_camino(q: &Punto, a: &Punto, b: &Punto, radio: i64) -> bool {
    let (dx, dy) = ((b.x - a.x) as f64, (b.y - a.y) as f64);
    let largo = dx * dx + dy * dy;
    let u = if largo == 0.0 { 0.0 } else { (((q.x - a.x) as f64 * dx + (q.y - a.y) as f64 * dy) / largo).clamp(0.0, 1.0) };
    ((q.x as f64 - a.x as f64 - u * dx).hypot(q.y as f64 - a.y as f64 - u * dy)) <= radio as f64
}

/// Un punto al azar a entre `min` y `max` de `centro`.
fn punto_en_anillo<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, centro: &Punto, (min, max): (i64, i64)) -> Punto {
    let angulo = ctx.azar.real() * std::f64::consts::TAU;
    let radio = min as f64 + ctx.azar.real() * (max - min) as f64;
    Punto { x: centro.x + (radio * angulo.cos()) as i64, y: centro.y + (radio * angulo.sin()) as i64 }
}

/// El titular sale con su tropa llevándose la caravana, que espera dentro, y reposta en la puerta. Los bandidos acosan la caravana con el poder de la columna que la lleva y
/// el anillo está lleno de ellos, así que espera a los que están listos en el campamento (como la caza en grupo, TypeScript `salirAFundar`); después marcha a fundar.
async fn salir_a_fundar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, casa: &str, caravana: &str) -> Result<(), Fallo> {
    let (t, yo) = (ctx.ahora_ms, ctx.yo.to_string());
    let cuantos = ctx.con_pizarra(|p| p.listos.iter().filter(|(h, (k, _, hasta))| k == casa && *hasta > t && **h != yo).count()) + 1;
    // Como ejército (que admite compañía): una columna personal no lleva caravanas.
    if !salir_con(ctx, casa, true, Some("aceptar"), Some(caravana)).await? {
        return Ok(());
    }
    ctx.puerto.actuar(O::RepostarEnCampamento(pb::OrdenRepostarEnCampamento { campamento_id: Some(casa.into()) })).await?;
    if let Some(col) = columna(ctx.puerto.observar(), &yo) {
        reunir(ctx, &format!("fundar:{casa}"), casa, &col.id, "fundar", ESPERA_GRUPO_MS);
        ctx.memoria.plan = Some(Plan::EsperarFundar { cuantos, desde: t, inicio: t });
    }
    Ok(())
}

/// Con el grupo en la columna (o sin más que esperar): a un sitio fundable del anillo al que se pueda llegar y cuyo camino pueda pasar `poder`. ¿Ha salido?
async fn marchar_a_fundar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, puerta: &Punto, poder: u32) -> Result<bool, Fallo> {
    let yo = ctx.yo.to_string();
    if std::env::var("BOTS_DEPURA").is_ok() {
        let col = columna(ctx.puerto.observar(), &yo);
        let bandidos = ctx.con_pizarra(|p| p.bandidos.values().filter(|(q, _, _)| distancia(q, puerta) <= d.anillo.1 + 20_000).map(|(_, x, _)| *x).collect::<Vec<_>>());
        eprintln!("DEPURA fundar {} {:?} heroes {} escuadras {} bandidos {bandidos:?}", yo, ctx.faccion(), col.as_ref().map_or(0, |c| c.participantes.len()), col.as_ref().map_or(0, |c| c.escuadron_ids.len()));
    }
    // Un sitio fundable, más allá del anillo de bandidos, al que se pueda llegar (sin camino, `ruta.imposible`).
    for _ in 0..8 {
        let Some(sitio) = sitio_seguro(ctx, d, puerta, poder).await? else { break };
        if ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: yo.clone(), objetivo: a_punto(sitio.clone()) })).await?.resultado == Resultado::OrdenAceptada as i32 {
            ctx.memoria.plan = Some(Plan::Fundar { sitio, intentos: 0 });
            return Ok(true);
        }
    }
    Ok(false)
}

/// Con la caravana enganchada: de camino, sigue; parado, funda donde está. Si `fundar` dice que no, prueba otro sitio cerca, hasta [`INTENTOS_DE_FUNDAR`]; después vuelve.
async fn fundar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, sitio: Punto, intentos: u32, aqui: Punto, parada: bool) -> Result<(), Fallo> {
    if !parada {
        ctx.memoria.plan = Some(Plan::Fundar { sitio, intentos });
        return Ok(());
    }
    let yo = ctx.yo;
    let lleva = entidades(ctx.puerto.observar()).any(|c| {
        matches!(c, Cuerpo::Caravana(x) if x.tipo == pb::TipoCaravana::CaravanaDeFundacion as i32 && x.titular_id.as_deref() == Some(yo) && x.estado == pb::EstadoCaravana::CaravanaAdjunta as i32)
    });
    if lleva && ctx.puerto.actuar(O::Fundar(pb::OrdenFundar {})).await?.resultado == Resultado::OrdenAceptada as i32 {
        // Fundada: la tarea está hecha y sale del tablero ya, no cuando caduque.
        {
            let yo = ctx.yo.to_string();
            ctx.con_pizarra(|p| p.tablero.soltar_de("fundar:", &yo));
        }
        evento(ctx, "funda", format!("funda en ({}, {}) desde el campamento", aqui.x / 1000, aqui.y / 1000), json!({"x": aqui.x, "y": aqui.y}));
        if let Some(f) = ctx.faccion() {
            ctx.pedir_refuerzos(&f);
        }
        return Ok(()); // ya reside en la plaza nueva: la siguiente vez piensa con plaza
    }
    if !lleva || intentos + 1 >= INTENTOS_DE_FUNDAR {
        fracaso_al_fundar(ctx);
        return Ok(()); // sin plan: vuelve a casa
    }
    let Some(otro) = sitio_para_fundar(ctx, d, &aqui, OTRO_SITIO).await? else { return Ok(()) };
    ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: ctx.yo.into(), objetivo: a_punto(otro.clone()) })).await?;
    ctx.memoria.plan = Some(Plan::Fundar { sitio: otro, intentos: intentos + 1 });
    Ok(())
}

#[cfg(test)]
mod rejilla {
    use super::*;

    #[test]
    fn lo_que_se_ve_queda_visto_en_las_casillas_enteras_del_circulo() {
        let mut r = BTreeMap::new();
        marcar_vistas(&mut r, &Punto { x: 0, y: 0 }, 45_000, 7);
        // Centros a −30, −10, 10 y 30 mil en cada eje, dentro de 35 mil del centro: 4 × 4 menos las cuatro esquinas.
        assert_eq!(r.len(), 12);
        assert!(r.contains_key(&(1, 0)) && !r.contains_key(&(1, 1)), "(30, 10) entra; (30, 30) no");
        assert_eq!(r.get(&(0, 0)), Some(&7));
        assert!(r.contains_key(&(-2, 0)) && !r.contains_key(&(-2, 1)) && !r.contains_key(&(2, 0)) && !r.contains_key(&(-3, 0)));
        assert_eq!(casilla(&Punto { x: -1, y: 19_999 }), (-1, 0));
        assert_eq!(centro_de_casilla((-1, 0)), Punto { x: -10_000, y: 10_000 });
    }

    #[test]
    fn la_niebla_dice_que_celdas_estan_exploradas() {
        // 10 columnas × 2 filas de 5 u: la celda (3, 1) es la 13 → byte 1, bit 5.
        let n = pb::NieblaProyectada { tamano_celda: 5_000, columnas: 10, filas: 2, celdas: vec![0, 1 << 5, 0], ..Default::default() };
        assert!(explorado_en(&n, &Punto { x: 17_000, y: 6_000 }));
        assert!(!explorado_en(&n, &Punto { x: 12_000, y: 6_000 }) && !explorado_en(&n, &Punto { x: -1, y: 6_000 }) && !explorado_en(&n, &Punto { x: 17_000, y: 12_000 }));
    }
}

#[cfg(test)]
mod panel {
    use super::*;

    #[test]
    fn la_pizarra_sale_con_lo_que_el_panel_ya_pinta_y_con_el_tablero() {
        let mut p = Pizarra::default();
        p.tablero.tomar("cazar:b1", "h1", 0, 100);
        p.tablero.reunir("campana:as-9", "h2", "as-1", "ej-1", "as-9", 0, 50, 100);
        p.tablero.anotar_fuerza("as-1", "h2", 3, 420);
        p.bandidos.insert("b1".into(), (Punto { x: 1, y: 2 }, 50, 0));
        let r = resumen_de_pizarra(&p, 10);
        assert_eq!(r["encargos"].as_array().map(Vec::len), Some(2));
        assert_eq!(r["bandidos"], 1);
        assert_eq!(r["salidas"], 1);
        let s = &r["detalle"]["salidasAbiertas"][0];
        assert_eq!((s["campamentoId"].as_str(), s["ejercitoId"].as_str(), s["unidos"].as_u64(), s["fuerza"].as_u64()), (Some("as-1"), Some("ej-1"), Some(3), Some(420)));
        assert!(r["detalle"]["listos"].is_array() && r["detalle"]["residencias"].is_array() && r["relaciones"].is_object());
        let d = detalle_de_pizarra(&p, 10);
        assert_eq!(d["tablero"].as_array().map(Vec::len), Some(2));
        assert!(d["libro"].is_object() && d["rejilla"]["casillas"].is_array());
    }
}

#[cfg(test)]
mod logros {
    use super::*;

    #[test]
    fn un_hecho_da_puntos_a_los_suyos_y_el_registro_los_guarda() {
        let mut p = Pizarra::default();
        p.miembros.extend(["h1".to_string(), "h2".to_string()]);
        let h = pb::EventoDominio { codigo: "combate.campamento_destruido".into(), en_ms: 5, heroes_ids: vec!["h1".into(), "h2".into(), "ajeno".into()], ..Default::default() };
        Bot::oir(&h, &mut p);
        Bot::oir(&h, &mut p);
        assert_eq!((puntos_de(&p, "h1", Some("guerra")), puntos_de(&p, "h2", None), puntos_de(&p, "ajeno", None)), (10, 5, 0), "una vez, entero al que lo logra y la mitad a los demás");
        let l = &p.logros[0];
        assert_eq!((l.logrador.as_deref(), l.participantes.clone()), (Some("h1"), vec!["h2".to_string()]));
        let vuelta = Bot::cargar(&Bot::guardar(&p));
        assert_eq!((puntos_de(&vuelta, "h1", None), vuelta.logros.len()), (10, 1));
        let viejo = Bot::cargar(&serde_json::to_value(&p.libro).unwrap());
        assert!(viejo.puntos.is_empty(), "un registro de antes se sigue leyendo");
    }
}

#[cfg(test)]
mod fundar_junto_a_lo_que_hace_falta {
    use super::*;

    #[test]
    fn el_sitio_vale_por_los_tipos_que_conoce_en_su_zona_y_menos_los_que_ya_saca() {
        let p = |x| Punto { x, y: 0 };
        let conocidos = vec![("estano".to_string(), p(10_000)), ("cobre".to_string(), p(-10_000)), ("cobre".to_string(), p(-12_000)), ("piedra".to_string(), p(40_000))];
        assert_eq!(valor_del_sitio(&p(0), &conocidos, &[]), 9.0, "estaño 5 y cobre 4 (una vez); la piedra queda fuera de la zona");
        assert_eq!(valor_del_sitio(&p(0), &conocidos, &["cobre".into()]), 7.0, "con una mina de cobre ya, el cobre vale la mitad");
        assert_eq!(valor_del_sitio(&p(100_000), &conocidos, &[]), 0.0, "nada que conozca cerca");
    }
}
