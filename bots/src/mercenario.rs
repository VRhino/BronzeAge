//! El cerebro mercenario (`Docs/Mechanics/Mercenarios_Por_Contrato.md` del motor, §14), aparte del de los bots libres: comparte con ellos el puerto y los datos, no sus planes. No funda
//! ni crea Facción. En cada vuelta deduce su etapa de lo que dice el servidor (contrato, orden, dónde está), así que un reinicio no le hace perder el hilo: poner orden en su
//! campamento, ronda de caza mientras sigue ofrecido, y con contrato prepararse, trabajar, esperar y volver.

use std::collections::BTreeMap;

use contrato::{
    almacen::Almacen,
    v1::{self as pb, entidad::Cuerpo, objetivo_mercenario::Tipo as Objetivo, orden::Cuerpo as O, orden_atacar, orden_marchar_a, orden_perseguir, ubicacion_heroe::Donde, EstadoColumna, Punto, Resultado},
};
use serde_json::json;

use crate::{
    azar::Azar,
    balance as b,
    cerebro::{columna, distancia, entidades, posicion},
    datos::{self, Datos},
    llegadas::{Llegada, TipoPerfil},
    runner::{Cerebro, Contexto},
    Fallo,
};

/// Lo que espera para repetir una orden que no le aceptaron.
const REINTENTO_MS: i64 = 3_600_000;
/// Lo que mira alrededor de donde está, en milésimas.
const VISTA: i64 = 45_000;

/// Lo que recuerda de una vuelta a otra: solo lo que el servidor no le dice.
#[derive(Default)]
pub struct Memoria {
    /// Dónde está su campamento: fuera de su vista no le llega.
    base: Option<Punto>,
    /// El tramo de la espiral por el que va buscando a su objetivo.
    busqueda: u32,
    /// De ronda: el tramo por el que va. Desde dentro del campamento no se ve el anillo: hay que salir a mirar.
    ronda: Option<u32>,
    /// El campamento de bandidos que ha encontrado en su ronda.
    presa: Option<String>,
    /// Los soldados que llegó a tener cada escuadra suya: con menos, la repone.
    tropa: BTreeMap<String, u32>,
    /// La etapa en que está, para el panel.
    etapa: &'static str,
}

/// Los cerebros que piensan con esta memoria (el propio [`Mercenario`], o uno que lo envuelve para mirarlo).
pub trait Tipos: Cerebro<Memoria = Memoria, Pizarra = ()> {}
impl<C: Cerebro<Memoria = Memoria, Pizarra = ()>> Tipos for C {}

pub struct Mercenario;

impl Cerebro for Mercenario {
    type Memoria = Memoria;
    type Pizarra = ();

    async fn pensar(&self, ctx: &mut Contexto<'_, Self>) -> Result<(), Fallo> {
        pensar(ctx).await
    }

    fn volcar(m: &Memoria) -> serde_json::Value {
        json!({ "tipo": m.etapa })
    }

    fn plan(semilla: u64, total: u32, dias: f64) -> Vec<Llegada> {
        plan_de_llegadas(semilla, total, dias)
    }

    /// Un mercenario retirado no se queda en el tablón.
    fn despedida() -> Option<O> {
        Some(O::RetirarOferta(pb::OrdenRetirarOferta {}))
    }
}

/// `total` mercenarios, uno a uno, repartidos en `dias` días de mundo. Misma semilla, mismas llegadas.
pub fn plan_de_llegadas(semilla: u64, total: u32, dias: f64) -> Vec<Llegada> {
    let mut azar = Azar::new(semilla ^ 0x3e7c);
    let ms = (dias * 86_400_000.0).round().max(1.0);
    let mut plan: Vec<Llegada> = (0..total).map(|grupo| Llegada { en_ms: (azar.real() * ms) as i64, grupo, perfil: TipoPerfil::Mercenario, cuantos: 1 }).collect();
    plan.sort_by_key(|l| (l.en_ms, l.grupo));
    plan
}

/// Su héroe entero: el contrato y la ficha solo vienen en su propio ámbito.
fn heroe(a: &Almacen, yo: &str) -> Option<pb::Heroe> {
    a.ambitos.get(&format!("HEROE:{yo}")).and_then(|x| {
        x.entidades.values().find_map(|c| match c {
            Cuerpo::Heroe(h) if h.id == yo => Some(h.clone()),
            _ => None,
        })
    })
}

fn donde(h: &pb::Heroe) -> Option<&Donde> {
    h.ubicacion.as_ref().and_then(|u| u.donde.as_ref())
}

fn su_campamento(a: &Almacen, id: &str) -> Option<pb::CampamentoMercenarios> {
    entidades(a).find_map(|c| match c {
        Cuerpo::CampamentoMercenarios(k) if k.id == id => Some(k.clone()),
        _ => None,
    })
}

/// Su tropa: la suya, la que no está guarneciendo nada.
fn tropa<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>) -> Vec<pb::Escuadron> {
    entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Escuadron(e) if e.heroe_id == ctx.yo && e.cantidad > 0 && !e.en_guarnicion => Some(e.clone()),
            _ => None,
        })
        .collect()
}

/// De su tropa, lo que cabe en su Liderazgo.
fn cabe<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, d: &Datos) -> Vec<String> {
    crate::plaza::lo_que_cabe(ctx.puerto.observar(), ctx.yo, d, &tropa(ctx))
}

fn ids(escuadras: Vec<pb::Escuadron>) -> Vec<String> {
    escuadras.into_iter().map(|e| e.id).collect()
}

/// Lo que piensa un mercenario.
pub async fn pensar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>) -> Result<(), Fallo> {
    let Some(h) = heroe(ctx.puerto.observar(), ctx.yo) else { return Ok(()) };
    let d = datos::de(ctx.puerto).await?;
    if let Some(p) = h.residencia_id.as_deref().and_then(|k| su_campamento(ctx.puerto.observar(), k)).and_then(|k| k.posicion) {
        ctx.memoria.base = Some(p);
    }
    mirar(ctx, &h).await?;
    match h.contrato.as_ref() {
        None => libre(ctx, &d, &h).await,
        Some(c) => match c.orden.clone() {
            None => {
                ctx.memoria.etapa = "Esperar órdenes";
                Ok(())
            }
            Some(o) => trabajar(ctx, &d, &h, o).await,
        },
    }
}

/// Mira alrededor de donde está: lo enemigo solo le llega por sus ojos.
async fn mirar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, h: &pb::Heroe) -> Result<(), Fallo> {
    let t = ctx.ahora_ms;
    let a = ctx.puerto.observar();
    let aqui = match donde(h) {
        Some(Donde::CampamentoId(k)) => su_campamento(a, k).and_then(|k| k.posicion),
        Some(Donde::AsentamientoId(p)) => entidades(a).find_map(|c| match c {
            Cuerpo::Asentamiento(x) if x.id == *p => x.posicion.clone(),
            _ => None,
        }),
        _ => columna(a, ctx.yo).map(|e| posicion(&e, t)),
    };
    let Some(p) = aqui else { return Ok(()) };
    let r = pb::Rectangulo { min: Some(Punto { x: p.x - VISTA, y: p.y - VISTA }), max: Some(Punto { x: p.x + VISTA, y: p.y + VISTA }) };
    match ctx.puerto.mirar(r).await {
        Err(Fallo::Vista(_)) => Ok(()),
        r => r,
    }
}

// ───────────────────────────── Sin contrato ─────────────────────────────

async fn libre<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, h: &pb::Heroe) -> Result<(), Fallo> {
    let Some(casa) = h.residencia_id.clone().filter(|r| r.starts_with("camp-")) else { return Ok(()) };
    match donde(h) {
        Some(Donde::CampamentoId(k)) if *k == casa => poner_orden(ctx, d, h, &casa).await,
        Some(Donde::CampamentoId(_) | Donde::AsentamientoId(_)) => a_campo(ctx, h).await.map(|_| ()),
        _ => match (ctx.memoria.presa.clone(), ctx.memoria.ronda) {
            (Some(presa), _) => cazar(ctx, d, presa).await,
            (None, Some(tramo)) => rondar(ctx, d, tramo).await,
            (None, None) => volver(ctx, d, &casa).await,
        },
    }
}

/// En su campamento: reclama lo que le deben, repone su tropa, se ofrece y, si puede, sale de ronda.
async fn poner_orden<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, h: &pb::Heroe, casa: &str) -> Result<(), Fallo> {
    ctx.memoria.etapa = "Poner orden en su campamento";
    reclamar_premios(ctx, d, h).await?;
    reponer(ctx).await?;
    let ofrecido = su_campamento(ctx.puerto.observar(), casa).is_some_and(|k| k.tablon.iter().any(|o| o.heroe_id == ctx.yo));
    if !ofrecido {
        ctx.intentar("ofrecerse", O::OfrecerseComoMercenario(pb::OrdenOfrecerseComoMercenario {}), REINTENTO_MS).await?;
        return Ok(());
    }
    ctx.memoria.etapa = "En el tablón";
    // La ronda: oro, experiencia y su medalla de cazador sin dejar el tablón, y cerca, para llegar pronto si le contratan.
    let herido = h.herido_hasta_ms.is_some_and(|f| f > ctx.ahora_ms);
    if herido || ctx.en_espera("ronda") || tropa(ctx).is_empty() {
        return Ok(());
    }
    ctx.esperar("ronda", b::ENTRE_RONDAS_MS);
    preparar(ctx, casa).await?;
    if salir_del_campamento(ctx, casa).await? {
        ctx.memoria.ronda = Some(0);
    }
    Ok(())
}

/// El campamento más débil del anillo con el que puede.
fn presa_a_tiro<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, d: &Datos) -> Option<String> {
    let puerta = ctx.memoria.base.as_ref()?;
    let fuerza = tropa(ctx).iter().map(|e| d.poder_de(e)).sum::<u32>() + d.poder_por_heroe;
    entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::CampamentoBandidos(x) if fuerza as f64 > x.poder as f64 * b::MARGEN_DE_RONDA && x.posicion.as_ref().is_some_and(|p| distancia(p, puerta) <= b::RADIO_DE_RONDA) => Some((x.poder, x.id.clone())),
            _ => None,
        })
        .min()
        .map(|(_, id)| id)
}

/// Fuera, por el anillo de su campamento: a lo primero que pueda vencer; si acaba los tramos sin nada, a casa.
async fn rondar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, tramo: u32) -> Result<(), Fallo> {
    ctx.memoria.etapa = "De ronda";
    if let Some(presa) = presa_a_tiro(ctx, d) {
        ctx.memoria.ronda = None;
        ctx.memoria.presa = Some(presa);
        return Ok(());
    }
    let (Some(col), Some(casa)) = (columna(ctx.puerto.observar(), ctx.yo), ctx.memoria.base.clone()) else { return Ok(()) };
    if col.estado != EstadoColumna::ColumnaEstacionada as i32 {
        return Ok(());
    }
    if tramo >= b::TRAMOS_DE_RONDA {
        ctx.memoria.ronda = None;
        return Ok(());
    }
    ctx.memoria.ronda = Some(tramo + 1);
    let angulo = ctx.azar.real() * std::f64::consts::TAU;
    let radio = b::RADIO_DE_RONDA as f64 * 0.7;
    marchar(ctx, Punto { x: casa.x + (radio * angulo.cos()) as i64, y: casa.y + (radio * angulo.sin()) as i64 }).await
}

/// Un premio de rango: la tropa más fuerte de su licencia. El contrato no dice cuáles entran, así que decide el servidor, y lo rechazado no se repite en 6 h.
async fn reclamar_premios<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, h: &pb::Heroe) -> Result<(), Fallo> {
    if h.ficha_mercenario.as_ref().is_none_or(|f| f.premios == 0) {
        return Ok(());
    }
    let mut tropas: Vec<(u32, String)> = d.poder.iter().map(|(t, p)| (*p, t.clone())).collect();
    tropas.sort();
    for (_, t) in tropas.into_iter().rev() {
        let o = O::ReclamarPremioDeRango(pb::OrdenReclamarPremioDeRango { tropa_id: t.clone() });
        if ctx.intentar(&format!("premio:{t}"), o, 6 * REINTENTO_MS).await?.is_some_and(|r| r.resultado == Resultado::OrdenAceptada as i32) {
            break;
        }
    }
    Ok(())
}

/// Repone con su oro las escuadras que han perdido gente.
async fn reponer<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>) -> Result<(), Fallo> {
    for e in tropa(ctx) {
        let tenia = ctx.memoria.tropa.entry(e.tropa_id.clone()).or_default();
        *tenia = (*tenia).max(e.cantidad);
        if e.cantidad < *tenia {
            let o = O::ReclutarEnCampamento(pb::OrdenReclutarEnCampamento { tropa_id: e.tropa_id.clone(), pagar_con: Some("almacenPersonal".into()) });
            // Sin oro no lo vuelve a intentar hasta el día siguiente: lo gana con las rondas y los contratos.
            ctx.intentar(&format!("reponer:{}", e.tropa_id), o, 24 * REINTENTO_MS).await?;
        }
    }
    Ok(())
}

/// Ha salido de ronda: va a por su presa y, de un golpe, a casa.
async fn cazar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, presa: String) -> Result<(), Fallo> {
    ctx.memoria.etapa = "De ronda";
    let objetivo = Objetivo::CampamentoId(presa);
    let Some(sitio) = punto_de(ctx, &objetivo) else {
        ctx.memoria.presa = None;
        return Ok(());
    };
    if ir(ctx, d, &sitio).await? {
        ctx.memoria.presa = None;
        atacar(ctx, objetivo).await?;
    }
    Ok(())
}

/// De vuelta a su campamento: entra y vacía el carro, que su oro se queda en su almacén.
async fn volver<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, casa: &str) -> Result<(), Fallo> {
    ctx.memoria.etapa = "Volver a su campamento";
    let Some(puerta) = ctx.memoria.base.clone() else { return Ok(()) };
    let yo = ctx.yo.to_string();
    let Some(col) = columna(ctx.puerto.observar(), &yo) else { return Ok(()) };
    if distancia(&posicion(&col, ctx.ahora_ms), &puerta) > d.radio_puerta {
        if col.estado == EstadoColumna::ColumnaEstacionada as i32 {
            marchar(ctx, puerta).await?;
        }
        return Ok(());
    }
    if col.estado == EstadoColumna::ColumnaEstacionada as i32 {
        ctx.puerto.actuar(O::EntrarEnCampamento(pb::OrdenEntrarEnCampamento { campamento_id: casa.into(), heroe_id: yo.clone() })).await?;
        ctx.puerto.actuar(O::VaciarCarro(pb::OrdenVaciarCarro { heroe_id: yo })).await?;
    }
    Ok(())
}

// ───────────────────────────── Con contrato ─────────────────────────────

/// Llena los víveres antes de irse. Con ellos llenos el servidor dice que no, y da igual.
async fn preparar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, casa: &str) -> Result<(), Fallo> {
    ctx.intentar("repostar", O::RepostarEnCampamento(pb::OrdenRepostarEnCampamento { campamento_id: Some(casa.into()) }), REINTENTO_MS).await?;
    Ok(())
}

async fn salir_del_campamento<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, casa: &str) -> Result<bool, Fallo> {
    let d = datos::de(ctx.puerto).await?;
    let escuadron_ids = cabe(ctx, &d);
    // Sale en columna personal: solo así se une luego a un ejército en el campo.
    let o = O::SalirDelCampamento(pb::OrdenSalirDelCampamento { campamento_id: casa.into(), heroe_id: ctx.yo.into(), escuadron_ids, carga: None, politica_de_union: None, objetivo: None, caravana_fundacion_id: None });
    Ok(ctx.intentar("salir", o, 10 * 60_000).await?.is_some_and(|r| r.resultado == Resultado::OrdenAceptada as i32))
}

/// Sale al mapa con su tropa. ¿Ya estaba fuera?
async fn a_campo<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, h: &pb::Heroe) -> Result<bool, Fallo> {
    match donde(h).cloned() {
        Some(Donde::CampamentoId(k)) => {
            preparar(ctx, &k).await?;
            salir_del_campamento(ctx, &k).await?;
            Ok(false)
        }
        Some(Donde::AsentamientoId(p)) => {
            // De visita sale con lo que entró; solo quien reside aquí elige su tropa.
            let heroe_id = ctx.yo.to_string();
            let o = if h.residencia_id.as_ref() == Some(&p) {
                let d = datos::de(ctx.puerto).await?;
                O::SalirAlMundo(pb::OrdenSalirAlMundo { asentamiento_id: p, heroe_id, escuadron_ids: cabe(ctx, &d), carga: None })
            } else {
                O::SalirDeAsentamiento(pb::OrdenSalirDeAsentamiento { asentamiento_id: p, heroe_id })
            };
            ctx.intentar("salir", o, REINTENTO_MS).await?;
            Ok(false)
        }
        Some(Donde::PuebloId(p)) => {
            ctx.intentar("salir", O::SalirDePueblo(pb::OrdenSalirDePueblo { pueblo_id: p, heroe_id: ctx.yo.to_string() }), REINTENTO_MS).await?;
            Ok(false)
        }
        _ => Ok(true),
    }
}

/// Dónde está lo que le han mandado, si lo sabe. Lo de su Facción le llega por su `HEROE`; lo enemigo, solo si lo ve.
fn punto_de<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, objetivo: &Objetivo) -> Option<Punto> {
    let t = ctx.ahora_ms;
    if let Objetivo::Punto(p) = objetivo {
        return Some(p.clone());
    }
    entidades(ctx.puerto.observar()).find_map(|c| match (objetivo, c) {
        (Objetivo::AsentamientoId(id), Cuerpo::Asentamiento(a)) if a.id == *id => a.posicion.clone(),
        (Objetivo::AsentamientoId(id) | Objetivo::PuebloId(id), Cuerpo::FichaConocida(f)) if f.asentamiento_id == *id => f.posicion.clone(),
        (Objetivo::PuebloId(id), Cuerpo::Pueblo(p)) if p.id == *id => p.posicion.clone(),
        (Objetivo::AvanzadaId(id), Cuerpo::Avanzada(a)) if a.id == *id => a.posicion.clone(),
        (Objetivo::CaravanaId(id), Cuerpo::Caravana(k)) if k.id == *id => k.posicion.clone(),
        (Objetivo::CampamentoId(id), Cuerpo::CampamentoBandidos(b)) if b.id == *id => b.posicion.clone(),
        (Objetivo::EjercitoId(id), Cuerpo::Ejercito(e)) if e.id == *id => Some(posicion(e, t)),
        _ => None,
    })
}

async fn marchar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, destino: Punto) -> Result<(), Fallo> {
    let clave = format!("ir:{}:{}", destino.x, destino.y);
    let objetivo = Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(destino)) });
    ctx.intentar(&clave, O::MarcharA(pb::OrdenMarcharA { heroe_id: ctx.yo.to_string(), objetivo }), REINTENTO_MS).await?;
    Ok(())
}

/// Va a `destino`. ¿Ha llegado?
async fn ir<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, destino: &Punto) -> Result<bool, Fallo> {
    let Some(col) = columna(ctx.puerto.observar(), ctx.yo) else { return Ok(false) };
    if distancia(&posicion(&col, ctx.ahora_ms), destino) <= d.radio_encuentro {
        return Ok(true);
    }
    if col.estado == EstadoColumna::ColumnaEstacionada as i32 {
        marchar(ctx, destino.clone()).await?;
    }
    Ok(false)
}

/// No ve a su objetivo enemigo: lo busca en espiral alrededor de su campamento hasta que lo vea.
async fn buscar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>) -> Result<(), Fallo> {
    ctx.memoria.etapa = "Buscar a su objetivo";
    let (Some(col), Some(casa)) = (columna(ctx.puerto.observar(), ctx.yo), ctx.memoria.base.clone()) else { return Ok(()) };
    if col.estado != EstadoColumna::ColumnaEstacionada as i32 {
        return Ok(());
    }
    let paso = ctx.memoria.busqueda % b::TRAMOS_DE_BUSQUEDA;
    ctx.memoria.busqueda += 1;
    let radio = (b::BUSQUEDA_RADIO_INICIAL + paso as i64 * b::BUSQUEDA_RADIO_POR_TRAMO) as f64;
    let angulo = ((paso * b::BUSQUEDA_GIRO) as f64).to_radians();
    marchar(ctx, Punto { x: casa.x + (radio * angulo.cos()) as i64, y: casa.y + (radio * angulo.sin()) as i64 }).await
}

/// Si el servidor le ofrece unirse a una batalla del lado defensor, se une: es lo que guarda.
async fn defender<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>) -> Result<bool, Fallo> {
    let t = ctx.ahora_ms;
    let batalla = entidades(ctx.puerto.observar()).find_map(|c| match c {
        Cuerpo::Batalla(b) if b.lado_para_unirse == Some(pb::LadoId::Defensor as i32) && b.convocatoria_hasta_ms.is_none_or(|h| h > t) => Some(b.id.clone()),
        _ => None,
    });
    let Some(batalla) = batalla else { return Ok(false) };
    let o = O::UnirseABatalla(pb::OrdenUnirseABatalla { heroe_id: ctx.yo.into(), battle_id: batalla.clone(), lado: Some("defensor".into()), escuadron_ids: vec![] });
    Ok(ctx.intentar(&format!("batalla:{batalla}"), o, REINTENTO_MS).await?.is_some_and(|r| r.resultado == Resultado::OrdenAceptada as i32))
}

/// Ataca una vez, aunque pueda perder: es lo que le pagan. Nunca declara la guerra: la orden solo vale con la guerra ya declarada.
async fn atacar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, objetivo: Objetivo) -> Result<(), Fallo> {
    let tipo = match objetivo {
        Objetivo::CampamentoId(x) => orden_atacar::objetivo::Tipo::CampamentoId(x),
        Objetivo::EjercitoId(x) => orden_atacar::objetivo::Tipo::EjercitoId(x),
        Objetivo::CaravanaId(x) => orden_atacar::objetivo::Tipo::CaravanaId(x),
        Objetivo::AsentamientoId(x) => orden_atacar::objetivo::Tipo::AsentamientoId(x),
        Objetivo::PuebloId(x) => orden_atacar::objetivo::Tipo::PuebloId(x),
        Objetivo::AvanzadaId(x) => orden_atacar::objetivo::Tipo::AvanzadaId(x),
        _ => return Ok(()),
    };
    let o = O::Atacar(pb::OrdenAtacar { heroe_id: ctx.yo.into(), objetivo: Some(orden_atacar::Objetivo { tipo: Some(tipo) }), declara_guerra: None });
    ctx.intentar("atacar", o, REINTENTO_MS).await?;
    Ok(())
}

/// Tras lo que se mueve va persiguiéndolo: marchar a donde lo vio siempre llega tarde.
async fn perseguir<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, objetivo: &Objetivo) -> Result<(), Fallo> {
    let tipo = match objetivo {
        Objetivo::EjercitoId(x) => orden_perseguir::objetivo::Tipo::EjercitoId(x.clone()),
        Objetivo::CaravanaId(x) => orden_perseguir::objetivo::Tipo::CaravanaId(x.clone()),
        _ => return Ok(()),
    };
    let o = O::Perseguir(pb::OrdenPerseguir { heroe_id: ctx.yo.into(), objetivo: Some(orden_perseguir::Objetivo { tipo: Some(tipo) }) });
    ctx.intentar("perseguir", o, 10 * 60_000).await?;
    Ok(())
}

/// Cumple la orden vigente.
async fn trabajar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, h: &pb::Heroe, orden: pb::OrdenMercenario) -> Result<(), Fallo> {
    (ctx.memoria.presa, ctx.memoria.ronda) = (None, None);
    let Some(objetivo) = orden.objetivo.and_then(|o| o.tipo) else { return Ok(()) };
    let yo = ctx.yo.to_string();
    ctx.memoria.etapa = match orden.tipo.as_str() {
        "guardar" => "Guardar",
        "ir_a" => "Ir al sitio",
        "atacar" => "Atacar",
        "unirse" => "Unirse",
        _ => "Trabajar",
    };
    if orden.tipo == "guardar" {
        if defender(ctx).await? {
            return Ok(());
        }
        if matches!((&objetivo, donde(h)), (Objetivo::AsentamientoId(p), Some(Donde::AsentamientoId(x))) if x == p) {
            return Ok(());
        }
    }
    if let ("unirse", Objetivo::ConvocatoriaId(c)) = (orden.tipo.as_str(), &objetivo) {
        let escuadron_ids = cabe(ctx, d);
        let o = O::UnirseAConvocatoria(pb::OrdenUnirseAConvocatoria { heroe_id: yo, convocatoria_id: c.clone(), escuadron_ids, carga: None });
        ctx.intentar(&format!("convocatoria:{c}"), o, REINTENTO_MS).await?;
        return Ok(());
    }
    if let (Objetivo::PuebloId(p), Some(Donde::PuebloId(x))) = (&objetivo, donde(h)) {
        if x == p {
            let escuadron_ids = ids(tropa(ctx));
            if !escuadron_ids.is_empty() {
                ctx.intentar(&format!("guardar:{p}"), O::GuarnecerPueblo(pb::OrdenGuarnecerPueblo { pueblo_id: p.clone(), escuadron_ids }), REINTENTO_MS).await?;
            }
            return Ok(());
        }
    }
    if !a_campo(ctx, h).await? {
        return Ok(());
    }
    let Some(sitio) = punto_de(ctx, &objetivo) else {
        return if orden.tipo == "atacar" { buscar(ctx).await } else { Ok(()) };
    };
    ctx.memoria.busqueda = 0;
    let en_movimiento = matches!(objetivo, Objetivo::EjercitoId(_) | Objetivo::CaravanaId(_));
    let llego = columna(ctx.puerto.observar(), &yo).is_some_and(|c| distancia(&posicion(&c, ctx.ahora_ms), &sitio) <= d.radio_encuentro);
    if !llego {
        return if orden.tipo == "atacar" && en_movimiento { perseguir(ctx, &objetivo).await } else { ir(ctx, d, &sitio).await.map(|_| ()) };
    }
    match (orden.tipo.as_str(), objetivo) {
        ("guardar", Objetivo::AsentamientoId(p)) => {
            ctx.intentar(&format!("guardar:{p}"), O::EntrarEnAsentamiento(pb::OrdenEntrarEnAsentamiento { asentamiento_id: p.clone(), heroe_id: yo }), REINTENTO_MS).await?;
        }
        // Un pueblo se guarnece desde dentro.
        ("guardar", Objetivo::PuebloId(p)) => {
            ctx.intentar(&format!("entrar:{p}"), O::EntrarEnPueblo(pb::OrdenEntrarEnPueblo { pueblo_id: p.clone(), heroe_id: yo }), REINTENTO_MS).await?;
        }
        ("guardar", Objetivo::AvanzadaId(a)) => {
            let escuadron_ids = ids(tropa(ctx));
            if !escuadron_ids.is_empty() {
                ctx.intentar(&format!("guardar:{a}"), O::GuarnecerAvanzada(pb::OrdenGuarnecerAvanzada { heroe_id: yo, avanzada_id: a.clone(), escuadron_ids }), REINTENTO_MS).await?;
            }
        }
        ("guardar", Objetivo::CaravanaId(k)) => {
            let escuadron_ids = ids(tropa(ctx));
            // Solo quien reside en su plaza de origen cede escolta (`seleccionar_escolta` del motor): el temporal, que vive en su campamento, se la lleva con su ejército si está parada a
            // la puerta. Una caravana en ruta solo la acompaña: `ir` la sigue.
            if h.residencia_id.as_ref().is_some_and(|r| r.starts_with("as-")) {
                if !escuadron_ids.is_empty() {
                    ctx.intentar(&format!("escolta:{k}"), O::AsignarEscolta(pb::OrdenAsignarEscolta { caravana_id: k.clone(), heroe_id: yo, escuadron_ids }), REINTENTO_MS).await?;
                }
            } else if let Some(c) = columna(ctx.puerto.observar(), &yo).filter(|c| c.caravanas_adjuntas_ids.iter().all(|x| *x != k)) {
                if c.tipo == pb::TipoColumna::ColumnaPersonal as i32 {
                    ctx.intentar("organizar", O::OrganizarEjercito(pb::OrdenOrganizarEjercito { heroe_id: yo, politica_de_union: "aceptar".into() }), REINTENTO_MS).await?;
                } else {
                    ctx.intentar(&format!("adjuntar:{k}"), O::AdjuntarCaravana(pb::OrdenAdjuntarCaravana { ejercito_id: c.id, caravana_id: k.clone(), heroe_id: yo }), REINTENTO_MS).await?;
                }
            }
        }
        ("unirse", Objetivo::EjercitoId(e)) => {
            if columna(ctx.puerto.observar(), &yo).is_none_or(|c| c.id != e) {
                ctx.intentar(&format!("unirse:{e}"), O::UnirseEnCampo(pb::OrdenUnirseEnCampo { ejercito_id: e.clone(), heroe_id: yo }), REINTENTO_MS).await?;
            }
        }
        ("atacar", o) => atacar(ctx, o).await?,
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn llegan_uno_a_uno_y_siempre_igual() {
        let plan = plan_de_llegadas(7, 12, 3.0);
        assert_eq!(plan, plan_de_llegadas(7, 12, 3.0));
        assert_eq!(plan.len(), 12);
        assert!(plan.iter().all(|l| l.cuantos == 1 && l.perfil == TipoPerfil::Mercenario && l.en_ms < 3 * 86_400_000));
        assert!(plan.windows(2).all(|w| w[0].en_ms <= w[1].en_ms));
    }
}
