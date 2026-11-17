//! Los cerebros con plaza (`12_NPC_Fuera_Del_Motor.md` §16 del motor; TypeScript `typescript-final`, `src/bots/cerebro/gobierno.ts` y `militar.ts`): lo que hace cada residente en
//! casa y lo que su cargo le pide. Primera parte:
//!
//! - **Volver a casa**: quien reside en la plaza y anda fuera (recién mudado, de vuelta) marcha a ella y entra.
//! - **Rey**: toda plaza de su Facción tiene Gobernador (el residente de id más bajo) y adopta las tecnologías que le aparecen.
//! - **Gobernador**: Tesorero, el mercado, los edificios militares, la taberna y el palacio, el primer recinto y la subida de nivel. El servidor dice si se puede
//!   (coste, gates, sitio): un rechazo espera y se reintenta.
//! - **Tesorero**: las caravanas comerciales que le caben, los trueques que le proponen (acepta si le sobra lo que entrega), el trueque de supervivencia a tiempo y el mercado
//!   (vende lo que sobra, compra lo que escasea).
//! - **Residente**: recluta la mejor tropa que la plaza sepa hacer y guarnece con sus escuadras más fuertes hasta el cupo, sin meter nunca la última que le queda fuera.
//! - **Aedas**: el Rey o el Gobernador compran al itinerante que está en la plaza una tecnología revelada; quien dirige la plaza pone a cantar épicas a sus residentes.
//! - **Sin cargo**: se une a la campaña que sale de casa, caza el campamento de bandidos de su plaza si puede con él, explora para su Facción (inspecciona las plazas ajenas
//!   que conoce), sale de campaña contra lo que puede ganar y se queda en lo que conquista, o se muda a la plaza de su Facción con 2 residentes menos.

use std::collections::BTreeMap;

use contrato::v1::{self as pb, entidad::Cuerpo, Punto, orden::Cuerpo as O, orden_marchar_a, ubicacion_heroe::Donde, EstadoColumna, EstadoEdificio};

use crate::{
    cerebro::{columna, decidir_grupo, distancia, entidades, posicion, reunir, Grupo, Tipos},
    balance as b,
    datos::{Datos, Hito, Tecnologia, Tropa},
    libro::{rasgos, Estado, Rasgo, Via},
    politica,
    runner::{Contexto, ESPERA_TRAS_RECHAZO_MS},
    Fallo,
};

/// Lo que espera un edificio, un cargo o una subida tras un rechazo.
const ESPERA_OBRA_MS: i64 = 10 * 60_000;

pub(crate) async fn con_plaza<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, h: pb::Heroe, casa: &str) -> Result<(), Fallo> {
    let (yo, c) = (ctx.yo.to_string(), casa.to_string());
    ctx.con_pizarra(|p| p.residencias.insert(yo, c));
    rey(ctx).await?;
    let dentro = matches!(h.ubicacion.as_ref().and_then(|u| u.donde.as_ref()), Some(Donde::AsentamientoId(a)) if a == casa);
    if !dentro {
        if en_salida(ctx, d, casa).await? {
            return Ok(());
        }
        return volver_a_casa(ctx, d, casa).await;
    }
    // En casa no lleva nada en curso: lo que tuviera en el tablero (una salida que acabó sin pasar por su final, una que no llegó a salir) se suelta ya.
    {
        let yo = ctx.yo.to_string();
        (ctx.memoria.salida, ctx.memoria.en_casa) = (None, None);
        ctx.con_pizarra(|p| p.tablero.soltar_todas(&yo));
    }
    let Some(plaza) = plaza(ctx, casa) else { return Ok(()) };
    let cargos = plaza.cargos.clone().unwrap_or_default();
    // La avanzada la coloca quien tiene el cargo (Gobernador o Maestro de Obras) con su columna en el sitio: el Maestro de Obras sale a ello.
    if cargos.maestro_obras_id.as_deref() == Some(ctx.yo) && colocar_avanzada(ctx, d, &plaza).await? {
        return Ok(());
    }
    if cargos.gobernador_id.as_deref() == Some(ctx.yo) {
        gobernador(ctx, d, &plaza, &cargos).await?;
    }
    if cargos.tesorero_id.as_deref() == Some(ctx.yo) {
        tesorero(ctx, d, &plaza).await?;
    }
    let es_rey = entidades(ctx.puerto.observar()).any(|c| matches!(c, Cuerpo::Faccion(f) if f.id == plaza.faccion_id && f.rey_id.as_deref() == Some(ctx.yo)));
    if es_rey || cargos.gobernador_id.as_deref() == Some(ctx.yo) || cargos.sacerdote_id.as_deref() == Some(ctx.yo) {
        aedas(ctx, &plaza, es_rey || cargos.gobernador_id.as_deref() == Some(ctx.yo)).await?;
    }
    reclutar(ctx, d, &plaza).await?;
    guarnecer(ctx, d, &plaza).await?;
    salir(ctx, d, &plaza, &cargos).await
}

/// La plaza `id` con su interior (la de `ASENTAMIENTO`) o, si no, la que vea.
fn plaza<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, id: &str) -> Option<pb::Asentamiento> {
    let a = ctx.puerto.observar();
    let interior = a.ambitos.get(&format!("ASENTAMIENTO:{id}")).into_iter().flat_map(|x| x.entidades.values());
    interior.chain(entidades(a)).find_map(|c| match c {
        Cuerpo::Asentamiento(p) if p.id == id => Some(p.clone()),
        _ => None,
    })
}

/// Fuera de casa: marcha a ella y, en la puerta, entra.
async fn volver_a_casa<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, casa: &str) -> Result<(), Fallo> {
    let (Some(col), Some(p)) = (columna(ctx.puerto.observar(), ctx.yo), plaza(ctx, casa).and_then(|p| p.posicion)) else { return Ok(()) };
    if col.estado != EstadoColumna::ColumnaEstacionada as i32 {
        return Ok(());
    }
    let yo = ctx.yo.to_string();
    if col.lider_id != yo {
        // En la columna de otro: de vuelta en la puerta, se separa y entra.
        if distancia(&posicion(&col, ctx.ahora_ms), &p) <= d.radio_puerta {
            ctx.puerto.actuar(O::SepararseDelEjercito(pb::OrdenSepararseDelEjercito { heroe_id: yo.clone() })).await?;
            ctx.puerto.actuar(O::EntrarEnAsentamiento(pb::OrdenEntrarEnAsentamiento { asentamiento_id: casa.into(), heroe_id: yo })).await?;
        }
        return Ok(());
    }
    if distancia(&posicion(&col, ctx.ahora_ms), &p) <= d.radio_puerta {
        // Solo se entra con `entrar`; un ejército entra con `guarnecer`.
        let o = if col.tipo == pb::TipoColumna::ColumnaEjercito as i32 {
            O::Guarnecer(pb::OrdenGuarnecer { asentamiento_id: casa.into(), heroe_id: yo })
        } else {
            O::EntrarEnAsentamiento(pb::OrdenEntrarEnAsentamiento { asentamiento_id: casa.into(), heroe_id: yo })
        };
        ctx.intentar(&format!("entrar:{casa}"), o, 5 * 60_000).await?;
    } else {
        let objetivo = Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::AsentamientoId(casa.into())) });
        ctx.intentar(&format!("ir:{casa}"), O::MarcharA(pb::OrdenMarcharA { heroe_id: yo, objetivo }), 5 * 60_000).await?;
    }
    Ok(())
}

// ───────────────────────────── Personalidad ─────────────────────────────

/// La personalidad de una Facción de bots (§16; pregunta 6 de la fase 7): cambia prioridades, nunca reglas. Sale del id de la Facción, así que la comparten todos sus bots
/// y no hay que guardarla.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Personalidad {
    /// Antes funda que conquista: expande en cuanto puede.
    Expansionista,
    /// Mercado, caravanas y trueques; sale poco de campaña y con mucho margen.
    Mercantil,
    /// Recluta, explora más y sale de campaña con menos margen.
    Militarista,
}

impl Personalidad {
    pub fn de(faccion: &str) -> Self {
        match crate::azar::hash(faccion) % 3 {
            0 => Self::Expansionista,
            1 => Self::Mercantil,
            _ => Self::Militarista,
        }
    }

    /// El margen sobre la defensa vista para salir de campaña. ponytail: pesos a calibrar con el batch.
    fn margen(self) -> f64 {
        match self {
            Self::Expansionista => MARGEN_SOBRE_DEFENSA,
            Self::Mercantil => 2.0,
            Self::Militarista => 1.2,
        }
    }

    /// Desde qué nivel sale de campaña su plaza.
    fn nivel_para_campana(self) -> u32 {
        match self {
            Self::Mercantil => NIVEL_PARA_CAMPANA + 1,
            _ => NIVEL_PARA_CAMPANA,
        }
    }

    /// Cuántos residentes pide para salir a fundar.
    fn residentes_para_expandir(self) -> usize {
        match self {
            Self::Expansionista => RESIDENTES_PARA_EXPANDIR,
            _ => RESIDENTES_PARA_EXPANDIR + 1,
        }
    }

    /// Cada cuánto sale a una batida su explorador.
    fn entre_batidas(self) -> i64 {
        match self {
            Self::Militarista => b::ENTRE_BATIDAS_MS / 2,
            _ => b::ENTRE_BATIDAS_MS,
        }
    }
}

// ───────────────────────────── Rey ─────────────────────────────

/// Un Gobernador para cada plaza de su Facción que no lo tenga, y las tecnologías que le aparecen.
async fn rey<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>) -> Result<(), Fallo> {
    let Some(f) = ctx.faccion() else { return Ok(()) };
    let a = ctx.puerto.observar();
    let es_rey = entidades(a).any(|c| matches!(c, Cuerpo::Faccion(x) if x.id == f && x.rey_id.as_deref() == Some(ctx.yo)));
    if !es_rey {
        return Ok(());
    }
    // El Gobernador de cada plaza es el residente con más puntos en total; al que ya lo es solo se le quita si otro le supera con margen (una vez al día por plaza).
    let plazas: Vec<(String, Option<String>, Vec<String>)> = entidades(a)
        .filter_map(|c| match c {
            Cuerpo::Asentamiento(p) if p.faccion_id == f => Some((p.id.clone(), p.cargos.as_ref().and_then(|x| x.gobernador_id.clone()), p.casas_compradas.clone())),
            _ => None,
        })
        .collect();
    let sin_gobernador: Vec<(String, String)> = plazas
        .into_iter()
        .filter_map(|(plaza, titular, residentes)| {
            let mejor = ctx.con_pizarra(|p| mejor_para_el_cargo(p, &residentes, None, &[]))?;
            match titular {
                None => Some((plaza, mejor)),
                Some(t) if t != mejor && !vetado(ctx, &format!("cargos:{plaza}:gobernador")) && ctx.con_pizarra(|p| supera(p, &mejor, &t, None)) => Some((plaza, mejor)),
                _ => None,
            }
        })
        .collect();
    let por_adoptar: Vec<String> = entidades(a)
        .filter_map(|c| match c {
            Cuerpo::Tecnologias(t) if t.faccion_id == f => Some(t.aparecidas.iter().filter(|x| !t.adoptadas.contains(x)).cloned().collect::<Vec<_>>()),
            _ => None,
        })
        .next()
        .unwrap_or_default();
    for (plaza, heroe) in sin_gobernador {
        nombrar(ctx, &plaza, "gobernador", &heroe).await?;
    }
    // ponytail: adopta en cuanto el servidor le deja pagar; guardar el doble de la tarifa como margen (TypeScript) cuando la tecnología tenga su tanda.
    for t in por_adoptar {
        let o = O::AdoptarTecnologia(pb::OrdenAdoptarTecnologia { faccion_id: f.clone(), tecnologia_id: t.clone() });
        ctx.intentar(&format!("tecnologia:{t}"), o, ESPERA_TRAS_RECHAZO_MS).await?;
    }
    Ok(())
}

/// La diplomacia mínima del Rey bot (pregunta 6 de la fase 7): acepta la alianza que le proponen (salvo un militarista que tenga más plazas que quien la propone), no
/// entrega su Facción (rechaza vasallajes, anexiones y fusiones, como el TypeScript) salvo a quien la derrotó cuando se queda sin plazas, propone la anexión a las que derrota y pide la paz en sus guerras cuando pierde una plaza o cuando se la piden a él.
/// El resto de la diplomacia queda para `Ideas a diseñar.md`.
pub(crate) async fn diplomacia<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, f: &str) -> Result<(), Fallo> {
    let a = ctx.puerto.observar();
    let plazas = |x: &str| entidades(a).filter(|c| matches!(c, Cuerpo::Asentamiento(p) if p.faccion_id == x)).count();
    let mias = plazas(f);
    let militarista = Personalidad::de(f) == Personalidad::Militarista;
    let derrotada_por = entidades(a).find_map(|c| match c {
        Cuerpo::Faccion(x) if x.id == f => x.derrotada_por.clone().filter(|d| !d.is_empty()),
        _ => None,
    });
    let mut respuestas: Vec<(String, O)> = Vec::new();
    let tope_alianzas = entidades(a).find_map(|c| match c {
        Cuerpo::Faccion(x) if x.id == f => Some(x.tope_alianzas as usize),
        _ => None,
    });
    let alianzas_activas = entidades(a)
        .filter(|c| matches!(c, Cuerpo::Relacion(r) if r.estado == pb::EstadoRelacion::RelacionActiva as i32 && r.tipo == pb::TipoRelacion::RelacionAlianza as i32 && (r.faccion_a_id == f || r.faccion_b_id == f)))
        .count();
    let cabe_alianza = tope_alianzas.is_none_or(|tope| alianzas_activas < tope);
    // Las que derrotó (sin plazas, `derrotada_por` = la suya): les propone la anexión (§16: una Facción de bots derrotada por otra se le anexiona).
    // ponytail: la que colapsó sin vencedor (sus plazas en ruinas) sigue como está; sus héroes vuelven a los campamentos por las reglas del motor.
    for c in entidades(a) {
        if let Cuerpo::Faccion(x) = c {
            if x.id != f && x.derrotada_por.as_deref() == Some(f) {
                respuestas.push((format!("anexar:{}", x.id), O::ProponerAnexion(pb::OrdenProponerAnexion { faccion_a_id: f.into(), faccion_b_id: x.id.clone() })));
            }
        }
    }
    for c in entidades(a) {
        let Cuerpo::Propuesta(p) = c else { continue };
        let o = match &p.clase {
            Some(pb::propuesta::Clase::Relacion(r)) if r.faccion_b_id == f => {
                // Se alía con quien aprecia y nunca con quien le traicionó; acepta el vasallaje solo acorralado, frente a quien tiene el triple que él.
                let otra = &r.faccion_a_id;
                let alianza = r.tipo == pb::TipoRelacion::RelacionAlianza as i32;
                let aprecia = politica::estado(ctx, otra) >= Estado::Amistad && !ctx.con_pizarra(|l| l.libro.paginas.get(otra.as_str()).is_some_and(|x| x.traicion));
                let aceptar = if alianza { cabe_alianza && aprecia && !(militarista && mias > plazas(otra)) } else { mias > 0 && mias * 3 <= plazas(otra) };
                O::ResponderRelacion(pb::OrdenResponderRelacion { propuesta_id: p.id.clone(), aceptar })
            }
            // Sin plazas y derrotada por otra Facción, se deja anexionar por ella (§16); si no, no entrega su Facción.
            Some(pb::propuesta::Clase::Anexion(x)) if x.absorbida_id == f => {
                O::ResponderAnexion(pb::OrdenResponderAnexion { propuesta_id: p.id.clone(), aceptar: mias == 0 && derrotada_por.as_deref() == Some(x.absorbente_id.as_str()) })
            }
            Some(pb::propuesta::Clase::Fusion(x)) if x.faccion_b_id == f => O::ResponderFusion(pb::OrdenResponderFusion { propuesta_id: p.id.clone(), aceptar: false }),
            _ => continue,
        };
        respuestas.push((format!("propuesta:{}", p.id), o));
    }
    // La paz: en cada guerra activa, si perdió una plaza desde la última vez o si el otro ya la propuso (proponerla también es aceptarla).
    let antes = ctx.con_pizarra(|p| p.plazas_propias.replace(mias));
    let perdio = antes.is_some_and(|n| mias < n);
    let t = ctx.ahora_ms;
    if perdio {
        ctx.con_pizarra(|p| p.libro.cautela_hasta = t + b::CICATRIZ_DERROTA_MS);
    }
    let rey = entidades(a).find_map(|c| match c {
        Cuerpo::Faccion(x) if x.id == f => x.rey_id.clone(),
        _ => None,
    });
    let caracter = rey.as_deref().map(rasgos).unwrap_or([Rasgo::Cauto, Rasgo::Cauto]);
    let (cauto, oportunista, leal) = (caracter.contains(&Rasgo::Cauto), caracter.contains(&Rasgo::Oportunista), caracter.contains(&Rasgo::Leal));
    let mut en_guerra: Vec<String> = Vec::new();
    let mut aliadas: Vec<String> = Vec::new();
    for c in entidades(a) {
        let Cuerpo::Relacion(r) = c else { continue };
        if r.estado != pb::EstadoRelacion::RelacionActiva as i32 || (r.faccion_a_id != f && r.faccion_b_id != f) {
            continue;
        }
        let otra = if r.faccion_a_id == f { &r.faccion_b_id } else { &r.faccion_a_id };
        if r.tipo == pb::TipoRelacion::RelacionGuerra as i32 {
            en_guerra.push(otra.clone());
            let enfado = politica::estado(ctx, otra);
            let se_la_piden = r.paz_propuesta_por.as_deref().is_some_and(|x| x != f);
            // Pide la paz si pierde plazas y es cauto, o si el rencor se apagó; la acepta salvo que la odie.
            let quiere = (perdio && (cauto || enfado > Estado::Odio)) || enfado >= Estado::Neutral || (se_la_piden && enfado > Estado::Odio);
            if quiere && r.paz_propuesta_por.as_deref() != Some(f) {
                respuestas.push((format!("paz:{}", r.id), O::ProponerPaz(pb::OrdenProponerPaz { relacion_id: r.id.clone(), faccion_id: f.into() })));
            }
        } else if r.tipo == pb::TipoRelacion::RelacionAlianza as i32 {
            aliadas.push(otra.clone());
            // El aliado que ya no le quiere, o que flaquea si es oportunista, deja de serlo.
            if !leal && (politica::estado(ctx, otra) <= Estado::Recelo || (oportunista && plazas(otra) * 2 < mias)) {
                respuestas.push((format!("romper:{}", r.id), O::RomperRelacion(pb::OrdenRomperRelacion { relacion_id: r.id.clone(), iniciador_faccion_id: f.into() })));
            }
        } else if r.tipo == pb::TipoRelacion::RelacionVasallaje as i32 && r.faccion_b_id == f && plazas(&r.faccion_a_id) <= mias {
            // Vasalla de quien ya no puede con ella: se rebela.
            respuestas.push((format!("rebelion:{}", r.id), O::RebelionVasallo(pb::OrdenRebelionVasallo { relacion_id: r.id.clone() })));
        }
    }
    // Lo que el libro pide a su Rey: aliarse con quien aprecia, declarar la guerra a quien acosa y someter a quien ya no puede resistir.
    let (v, umbral) = (politica::vida(ctx), ctx.con_pizarra(|p| p.libro.umbral_de_alianza()));
    let paginas: Vec<(String, f64, bool)> = ctx.con_pizarra(|p| p.libro.paginas.iter().map(|(o, x)| (o.clone(), x.relacion(t, v), x.traicion)).collect());
    for (otra, relacion, traicion) in paginas {
        if en_guerra.contains(&otra) || aliadas.contains(&otra) {
        } else if relacion < 0.0 && plazas(&otra) > 0 && plazas(&otra) * 3 <= mias && !vetado(ctx, &format!("vasallaje:{otra}")) {
            vetar(ctx, &format!("vasallaje:{otra}"), 24 * 3_600_000);
            respuestas.push((format!("vasallaje:{otra}"), O::ProponerRelacion(pb::OrdenProponerRelacion { tipo: "vasallaje".into(), faccion_a_id: f.into(), faccion_b_id: otra.clone(), tributo_recurso: None, tributo_cantidad: None })));
        } else if relacion >= umbral && !traicion && cabe_alianza && !vetado(ctx, &format!("aliar:{otra}")) {
            vetar(ctx, &format!("aliar:{otra}"), 24 * 3_600_000);
            respuestas.push((format!("aliar:{otra}"), O::ProponerRelacion(pb::OrdenProponerRelacion { tipo: "alianza".into(), faccion_a_id: f.into(), faccion_b_id: otra.clone(), tributo_recurso: None, tributo_cantidad: None })));
        } else if politica::acosable(ctx, &otra) && !vetado(ctx, &format!("guerra:{otra}")) {
            vetar(ctx, &format!("guerra:{otra}"), 24 * 3_600_000);
            respuestas.push((format!("guerra:{otra}"), O::DeclararGuerra(pb::OrdenDeclararGuerra { faccion_a_id: f.into(), faccion_b_id: otra.clone() })));
        }
    }
    for (clave, o) in respuestas {
        ctx.intentar(&clave, o, ESPERA_TRAS_RECHAZO_MS).await?;
    }
    Ok(())
}

// ───────────────────────────── Gobernador ─────────────────────────────

fn tiene(plaza: &pb::Asentamiento, tipo: &str) -> usize {
    plaza.edificios.iter().filter(|e| e.tipo == tipo).count()
}

/// El residente con más puntos de `clase` (o en total, con `None`), sin los de `fuera`; a igualdad, el de más puntos en total y luego el de id más bajo.
fn mejor_para_el_cargo(p: &crate::cerebro::Pizarra, residentes: &[String], clase: Option<&str>, fuera: &[String]) -> Option<String> {
    use crate::cerebro::puntos_de;
    residentes.iter().filter(|h| !fuera.contains(h)).max_by_key(|h| (puntos_de(p, h, clase), puntos_de(p, h, None), std::cmp::Reverse((*h).clone()))).cloned()
}

/// ¿Le quita `aspirante` el cargo a `titular`? Si tiene más puntos que él por [`b::MARGEN_PARA_EL_CARGO`].
fn supera(p: &crate::cerebro::Pizarra, aspirante: &str, titular: &str, clase: Option<&str>) -> bool {
    let (a, t) = (crate::cerebro::puntos_de(p, aspirante, clase), crate::cerebro::puntos_de(p, titular, clase));
    a > 0 && a as f64 > t as f64 * b::MARGEN_PARA_EL_CARGO
}

/// Da un cargo de la plaza (el motor se lo quita al que lo tuviera) y lo cuenta al panel. Una vez cada [`b::REVISION_CARGOS_MS`] por plaza y cargo.
async fn nombrar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &str, cargo: &str, heroe: &str) -> Result<(), Fallo> {
    let clave = format!("cargos:{plaza}:{cargo}");
    let o = O::AsignarCargoLocal(pb::OrdenAsignarCargoLocal { asentamiento_id: plaza.into(), cargo: cargo.into(), heroe_id: heroe.into() });
    let ok = aceptada(&ctx.intentar(&clave, o, ESPERA_OBRA_MS).await?);
    if ok {
        vetar(ctx, &clave, b::REVISION_CARGOS_MS);
        crate::cerebro::evento(ctx, "cargo", format!("nombra a {heroe} {cargo} de {plaza} por sus puntos"), serde_json::json!({"plaza": plaza, "cargo": cargo, "heroeId": heroe}));
    }
    Ok(())
}

/// Los cargos de la plaza por puntos (decisión de Víctor, 2026-11-11): el General, el residente con más puntos de guerra; el Maestro de Obras, el de obras; el Tesorero, el de
/// comercio. Sin él mismo, que es Gobernador, y sin repetir a nadie; si no queda nadie para el Tesorero, lo es él. Al titular solo se le quita si otro le supera con margen.
async fn repartir_cargos<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &pb::Asentamiento, cargos: &pb::Cargos) -> Result<(), Fallo> {
    let (yo, casa) = (ctx.yo.to_string(), plaza.id.clone());
    let residentes: Vec<String> = ctx.con_pizarra(|p| p.residencias.iter().filter(|(_, r)| **r == casa).map(|(h, _)| h.clone()).collect());
    let mut fuera = vec![yo.clone()];
    for (cargo, clase, titular) in [("general", "guerra", &cargos.general_id), ("maestroObras", "obras", &cargos.maestro_obras_id), ("tesorero", "comercio", &cargos.tesorero_id)] {
        let mejor = ctx.con_pizarra(|p| mejor_para_el_cargo(p, &residentes, Some(clase), &fuera)).or_else(|| (cargo == "tesorero").then(|| yo.clone()));
        let Some(mejor) = mejor else { continue };
        let nombra = match titular {
            None => true,
            Some(t) => *t != mejor && !vetado(ctx, &format!("cargos:{casa}:{cargo}")) && ctx.con_pizarra(|p| supera(p, &mejor, t, Some(clase))),
        };
        if nombra {
            nombrar(ctx, &casa, cargo, &mejor).await?;
            fuera.push(mejor);
        } else if let Some(t) = titular {
            fuera.push(t.clone());
        }
    }
    Ok(())
}

async fn gobernador<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento, cargos: &pb::Cargos) -> Result<(), Fallo> {
    let id = plaza.id.clone();
    repartir_cargos(ctx, plaza, cargos).await?;
    let nivel = plaza.nivel_actual;
    let t = ctx.ahora_ms;
    // Lo que piden los hitos de las tecnologías que persigue la Facción (en su plaza mayor; no las que esperan otra tecnología) y la tropa siguiente de la plaza: lo que
    // levantar y lo que mejorar.
    let (tec, _) = tecnologias(ctx, &plaza.faccion_id);
    let mut pasos: Vec<Paso> = Vec::new();
    let mut tomadas: Vec<String> = Vec::new();
    if plaza_mayor(ctx, &plaza.faccion_id).as_deref() == Some(plaza.id.as_str()) {
        for (x, faltan) in pendientes(ctx, d, &plaza.faccion_id) {
            if faltan.iter().any(|h| matches!(h, Hito::Tecnologia(_))) {
                continue;
            }
            let antes = pasos.len();
            for h in faltan {
                if let Hito::Edificio(tipo, n) = h {
                    if !HITOS_DE_PUEBLO.contains(&tipo.as_str()) {
                        pasos_para(d, plaza, &tec.adoptadas, tipo, *n, &mut pasos);
                    }
                }
            }
            if pasos[antes..].iter().any(|p| !matches!(p, Paso::Falta(_))) {
                tomadas.push(format!("tecnologia:{}", x.id));
            }
        }
    }
    if let Some(s) = tropa_siguiente(d, plaza, &tec.adoptadas, t) {
        if s.pasos.iter().any(|p| !matches!(p, Paso::Falta(_))) {
            tomadas.push(format!("tropa:{}", plaza.id));
        }
        pasos.extend(s.pasos);
    }
    {
        let yo = ctx.yo.to_string();
        ctx.con_pizarra(|p| {
            for k in tomadas {
                p.tablero.tomar(&k, &yo, t, t + b::NECESIDAD_VALE_MS);
            }
        });
    }
    // Lo que pide la subida siguiente (los que faltan de su lista, o los distintos que falten hasta su mínimo), el mercado y los edificios militares, la taberna y el palacio; solo
    // lo que el nivel de la plaza ya deja construir. La Granja no: la ciudad solo tiene la que trae la fundación, y el trigo de más sale de las aldeas.
    let quiere: Vec<String> = {
        let mut q: Vec<String> = Vec::new();
        if let Some(r) = d.requisitos.get(&(nivel + 1)) {
            let faltan: Vec<&String> = r.edificios.iter().filter(|t| tiene(plaza, t) == 0).collect();
            let hechos = r.edificios.len() - faltan.len();
            let cuantos = r.minimo.map_or(faltan.len(), |m| m.saturating_sub(hechos));
            q.extend(faltan.into_iter().take(cuantos).cloned());
        }
        for p in &pasos {
            if let Paso::Levantar(x) = p {
                if !q.contains(x) {
                    q.push(x.clone());
                }
            }
        }
        // Con el oro escaso, ni taberna ni palacio: el oro, para reclutar (§15).
        let oro_de_sobra = plaza.almacen.get("oro").is_some_and(|e| fraccion(e, t) >= COLCHON);
        q.extend(["mercado", "barracon", "galeriaDeTiro", "taberna", "palacio"].into_iter().filter(|t| tiene(plaza, t) == 0 && (oro_de_sobra || !matches!(*t, "taberna" | "palacio"))).map(String::from));
        // La comida: la población come primero pan (pesants), queso (artesanos), vino, cerveza o aceite (nobleza), y crece más con ellos. Molino y horno siempre; la quesera,
        // con la leche de la aldea ganadera (motor 2026-11-11); la bodega y la prensa, cuando una aldea de la plaza da su insumo.
        let cultivos = cultivos_de(ctx, &plaza.id);
        let da = |c: &[&str]| cultivos.iter().any(|x| c.contains(&x.as_str()));
        let comida = [("molino", true), ("horno", true), ("quesera", da(&["ganado"])), ("bodega", da(&["vid", "cebada"])), ("prensa", da(&["oliva", "sesamo"]))];
        q.extend(comida.into_iter().filter(|(t, si)| *si && tiene(plaza, t) == 0).map(|(t, _)| t.to_string()));
        q.retain(|t| d.nivel_para_construir.get(t).copied().unwrap_or(1) <= nivel);
        q
    };
    // Lo que la subida pide a cierto nivel: mejora el más alto de ese tipo si ninguno llega (el servidor dice si hay recursos o ya mejora).
    for (tipo, minimo) in d.requisitos.get(&(nivel + 1)).map_or(&[][..], |r| &r.con_nivel[..]) {
        let de_tipo = plaza.edificios.iter().filter(|e| e.tipo == *tipo);
        if de_tipo.clone().all(|e| e.nivel_interno.unwrap_or(1) < *minimo) {
            if let Some(e) = de_tipo.max_by_key(|e| e.nivel_interno.unwrap_or(1)) {
                let o = O::MejorarEdificioAhora(pb::OrdenMejorarEdificioAhora { asentamiento_id: id.clone(), cargo: "gobernador".into(), edificio_id: e.id.clone() });
                ctx.intentar(&format!("mejorar:{}", e.id), o, ESPERA_OBRA_MS).await?;
            }
        }
    }
    let mut mejoradas: Vec<&str> = Vec::new();
    for p in &pasos {
        if let Paso::Mejorar(e, ..) = p {
            if !mejoradas.contains(&e.as_str()) {
                mejoradas.push(e);
                let o = O::MejorarEdificioAhora(pb::OrdenMejorarEdificioAhora { asentamiento_id: id.clone(), cargo: "gobernador".into(), edificio_id: e.clone() });
                ctx.intentar(&format!("mejorar:{e}"), o, ESPERA_OBRA_MS).await?;
            }
        }
    }
    for tipo in quiere {
        let o = O::AnadirEdificioManualmente(pb::OrdenAnadirEdificioManualmente { asentamiento_id: id.clone(), cargo: "gobernador".into(), tipo: tipo.clone() });
        ctx.intentar(&format!("edificio:{id}:{tipo}"), o, ESPERA_OBRA_MS).await?;
    }
    if plaza.recintos.is_empty() {
        let o = O::ComprometerRecinto(pb::OrdenComprometerRecinto { asentamiento_id: id.clone(), cargo: "gobernador".into(), nivel: "1".into() });
        ctx.intentar(&format!("recinto:{id}"), o, ESPERA_TRAS_RECHAZO_MS).await?;
    }
    // Subir de nivel en cuanto cumple: el servidor decide los gates y el coste.
    // El motor ya no la rechaza por solvencia (2026-11-10): solo avisa en `insolvencia_ascenso`. Con algo sin cubrir, no sube: no pagaría el nivel nuevo con lo que produce.
    if plaza.ascenso.is_none() && plaza.insolvencia_ascenso.is_empty() {
        ctx.intentar(&format!("ascenso:{id}"), O::SolicitarAscenso(pb::OrdenSolicitarAscenso { asentamiento_id: id }), ESPERA_TRAS_RECHAZO_MS).await?;
    }
    // Una plaza recobrada lleva su historia en el nombre.
    if ctx.con_pizarra(|p| p.por_renombrar.contains(&plaza.id)) {
        ctx.con_pizarra(|p| p.por_renombrar.retain(|x| *x != plaza.id));
        if let Some(n) = plaza.nombre.as_ref().filter(|n| !n.ends_with("Recobrada")) {
            ctx.puerto.actuar(O::RenombrarAsentamiento(pb::OrdenRenombrarAsentamiento { asentamiento_id: plaza.id.clone(), nombre: format!("{n} Recobrada") })).await?;
        }
    }
    marcar_aldea(ctx, d, plaza).await?;
    cocina(ctx, plaza).await?;
    marcar_pueblo(ctx, d, plaza).await?;
    mejorar_pueblos(ctx, plaza).await?;
    comprar_intel(ctx, plaza).await?;
    comprar_rumor_de_estano(ctx, plaza).await?;
    reponer_carros(ctx, plaza).await
}

/// El rumor de las vetas de estaño (taberna, 2026-11-10): una plaza que ya lo necesita (su nivel lo pide) y no sabe de estaño a su alcance lo compra en su taberna, si le
/// sobra el oro. Deja explorados los yacimientos de estaño de la región de la taberna, y los bots los apuntan al leer la niebla de su Facción ([`crate::cerebro`], `mirar`).
/// Uno por plaza cada [`b::ENTRE_RUMORES_MS`]: el motor rechaza sin cobrar si no hay nada nuevo (`rumor.sin_novedad`).
async fn comprar_rumor_de_estano<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let t = ctx.ahora_ms;
    let clave = format!("rumor:{}", plaza.id);
    let taberna = plaza.edificios.iter().any(|e| e.tipo == "taberna" && e.estado == EstadoEdificio::EdificioActivo as i32);
    let oro = plaza.almacen.get("oro").map_or(0, |e| cantidad(e, t));
    let casa = plaza.posicion.clone().unwrap_or_default();
    if !taberna || plaza.nivel_actual < b::ESTANO_NIVEL_PLAZA || oro < (b::RUMOR_ESTANO_ORO + b::RUMOR_ESTANO_RESERVA) * 1000 || vetado(ctx, &clave) || estano_a_su_alcance(ctx, &casa) || estano_resuelto(ctx, &plaza.faccion_id) {
        return Ok(());
    }
    vetar(ctx, &clave, b::ENTRE_RUMORES_MS);
    let origen = Some(pb::orden_comprar_rumor::Origen { tipo: "asentamiento".into(), id: plaza.id.clone() });
    let r = ctx.puerto.actuar(O::ComprarRumor(pb::OrdenComprarRumor { origen, tipo: "vetas_estano".into() })).await?;
    if r.resultado == pb::Resultado::OrdenAceptada as i32 {
        crate::cerebro::evento(ctx, "rumor", format!("compra en {} el rumor de las vetas de estaño", plaza.id), serde_json::json!({"plaza": plaza.id}));
    }
    Ok(())
}

/// La intel de la taberna, con un presupuesto por Facción y día: el Informe de la plaza ajena que toca mirar y, sin ninguna, una Mirada lejos de casa. Solo con oro de sobra.
async fn comprar_intel<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let t = ctx.ahora_ms;
    let taberna = plaza.edificios.iter().any(|e| e.tipo == "taberna" && e.estado == EstadoEdificio::EdificioActivo as i32);
    let hoy = t / 86_400_000;
    let gastado = ctx.con_pizarra(|p| if p.intel.0 == hoy { p.intel.1 } else { 0 });
    if !taberna || gastado >= b::INTEL_POR_DIA || !plaza.almacen.get("oro").is_some_and(|e| fraccion(e, t) >= COLCHON) {
        return Ok(());
    }
    let centro = plaza.posicion.clone().unwrap_or_default();
    let o = match siguiente_a_explorar(ctx, &plaza.faccion_id, &centro) {
        Some(asentamiento_id) => {
            let origen = Some(pb::orden_comprar_informe_plaza::Origen { tipo: "asentamiento".into(), id: plaza.id.clone() });
            O::ComprarInformePlaza(pb::OrdenComprarInformePlaza { origen, asentamiento_id })
        }
        None => {
            let (angulo, radio) = (ctx.azar.real() * std::f64::consts::TAU, b::MIRADA_A_CIEGAS as f64);
            let punto = Punto { x: centro.x + (radio * angulo.cos()) as i64, y: centro.y + (radio * angulo.sin()) as i64 };
            let origen = Some(pb::orden_comprar_mirada::Origen { tipo: "asentamiento".into(), id: plaza.id.clone() });
            O::ComprarMirada(pb::OrdenComprarMirada { origen, centro: Some(punto) })
        }
    };
    if aceptada(&ctx.intentar(&format!("intel:{}", plaza.id), o, ESPERA_TRAS_RECHAZO_MS).await?) {
        ctx.con_pizarra(|p| p.intel = (hoy, gastado + 1));
    }
    Ok(())
}

// ───────────────────────────── Obras de mapa ─────────────────────────────

/// Los pueblos y avanzadas de una plaza, levantados o en obra.
fn obras_de<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, plaza: &str) -> (usize, usize) {
    let (mut pueblos, mut avanzadas) = (0, 0);
    for c in entidades(ctx.puerto.observar()) {
        match c {
            Cuerpo::Pueblo(p) if p.asentamiento_madre_id == plaza => pueblos += 1,
            Cuerpo::Avanzada(a) if a.asentamiento_madre_id == plaza => avanzadas += 1,
            Cuerpo::Obra(o) if o.asentamiento_madre_id == plaza && o.tipo == pb::TipoObra::ObraPueblo as i32 => pueblos += 1,
            Cuerpo::Obra(o) if o.asentamiento_madre_id == plaza && o.tipo == pb::TipoObra::ObraAvanzada as i32 => avanzadas += 1,
            _ => {}
        }
    }
    (pueblos, avanzadas)
}

fn tiene_aldea<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, plaza: &str) -> bool {
    entidades(ctx.puerto.observar()).any(|c| match c {
        Cuerpo::Pueblo(p) => p.asentamiento_madre_id == plaza && p.tipo_extractor == "aldeaAgricola",
        Cuerpo::Obra(o) => o.asentamiento_madre_id == plaza && o.tipo == pb::TipoObra::ObraAldea as i32,
        _ => false,
    })
}

/// El Gobernador pone a trabajar la comida: el buey del molino (12 de oro, el doble de harina) cuando sobra oro, y en la bodega y la prensa la receta del insumo que
/// dan sus aldeas (el motor empieza con vino y con aceite de aceituna). ponytail: una receta por plaza; dos en la bodega a nivel 5 cuando haga falta.
async fn cocina<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let t = ctx.ahora_ms;
    let id = plaza.id.clone();
    let oro_de_sobra = plaza.almacen.get("oro").is_some_and(|e| fraccion(e, t) >= COLCHON);
    let cultivos = cultivos_de(ctx, &id);
    let insumo = |edificio: &str| {
        let opciones: &[(&str, &str)] = if edificio == "bodega" { &[("vid", "uva"), ("cebada", "cebada")] } else { &[("oliva", "aceituna"), ("sesamo", "sesamo")] };
        opciones.iter().find(|(c, _)| cultivos.iter().any(|x| x == c)).map(|(_, i)| i.to_string())
    };
    for e in plaza.edificios.iter().filter(|e| e.estado == pb::EstadoEdificio::EdificioActivo as i32) {
        let o = match e.tipo.as_str() {
            "molino" if !e.buey && oro_de_sobra => O::AsignarBuey(pb::OrdenAsignarBuey { asentamiento_id: id.clone(), cargo: "gobernador".into(), edificio_id: e.id.clone() }),
            "bodega" | "prensa" => match insumo(&e.tipo).filter(|i| e.receta.as_deref() != Some(i.as_str())) {
                Some(insumo) => O::ElegirReceta(pb::OrdenElegirReceta { asentamiento_id: id.clone(), cargo: "gobernador".into(), edificio_id: e.id.clone(), insumo }),
                None => continue,
            },
            _ => continue,
        };
        ctx.intentar(&format!("cocina:{}", e.id), o, ESPERA_TRAS_RECHAZO_MS).await?;
    }
    Ok(())
}

/// Los cultivos de las aldeas hechas de una plaza (la obra no dice el suyo).
fn cultivos_de<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, plaza: &str) -> Vec<String> {
    entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Pueblo(p) if p.asentamiento_madre_id == plaza && p.tipo_extractor == "aldeaAgricola" => Some(p.cultivo.clone().unwrap_or_else(|| "trigo".into())),
            _ => None,
        })
        .collect()
}

fn aldea_en_obra<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, plaza: &str) -> bool {
    entidades(ctx.puerto.observar()).any(|c| matches!(c, Cuerpo::Obra(o) if o.asentamiento_madre_id == plaza && o.tipo == pb::TipoObra::ObraAldea as i32))
}

/// El nivel más alto que ha alcanzado una plaza de la Facción: el que desbloquea los cultivos.
fn nivel_de_faccion<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, f: &str) -> u32 {
    entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Asentamiento(a) if a.faccion_id == f => Some(a.nivel),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

/// Sube de nivel sus pueblos hasta el de la plaza (el motor, 2026-11-10: la Cantera y las minas de 1 a 3, 10 / 20 / 30 por minuto, como la aldea), uno a la vez: primero
/// el estaño y el cobre, luego la cantera, el resto de minas y al final las aldeas. La mejora queda pendiente de materiales; los lleva quien sale a las obras
/// ([`llevar_materiales`]). Un rechazo (sin nivel de madre, mejora en curso) veta ese pueblo un rato.
async fn mejorar_pueblos<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let a = ctx.puerto.observar();
    let en_obra = entidades(a).any(|c| matches!(c, Cuerpo::Obra(o) if o.asentamiento_madre_id == plaza.id && o.tipo == pb::TipoObra::ObraMejoraAldea as i32));
    if en_obra {
        return Ok(());
    }
    let orden = |tipo: &str| match tipo {
        "minaEstano" => 0,
        "minaCobre" => 1,
        "cantera" => 2,
        "aldeaAgricola" | "" => 4,
        _ => 3,
    };
    let pueblo = entidades(a)
        .filter_map(|c| match c {
            Cuerpo::Pueblo(p) if p.asentamiento_madre_id == plaza.id && p.nivel_aldea.unwrap_or(1) < plaza.nivel_actual.min(b::NIVEL_MAXIMO_PUEBLO) => Some((orden(&p.tipo_extractor), p.nivel_aldea.unwrap_or(1), p.id.clone())),
            _ => None,
        })
        .filter(|(_, _, id)| !vetado(ctx, &format!("mejorar:{id}")))
        .min();
    let Some((_, _, pueblo)) = pueblo else { return Ok(()) };
    let clave = format!("mejorar:{pueblo}");
    let o = O::MejorarAldea(pb::OrdenMejorarAldea { heroe_id: ctx.yo.into(), pueblo_id: pueblo.clone() });
    if !aceptada(&ctx.intentar(&clave, o, ESPERA_TRAS_RECHAZO_MS).await?) {
        vetar(ctx, &clave, 6 * 3_600_000);
    }
    Ok(())
}

/// El Gobernador marca sus aldeas agrícolas en la tierra más fértil de su zona, como el panel del motor: primero la de trigo; después, cuando ya tiene un pueblo extractor
/// y le queda sitio en el cupo, la ganadera (carne y leche, desde el 2026-11-11) y luego el mejor cultivo que su Facción ya tenga desbloqueado y la plaza no tenga aún.
/// El servidor rechaza lo que el mapa no sabe (sin ruta, demasiado cerca de algo); cada punto rechazado se veta un día.
async fn marcar_aldea<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    // Una a la vez: la obra no dice su cultivo.
    if aldea_en_obra(ctx, &plaza.id) {
        return Ok(());
    }
    let tiene = cultivos_de(ctx, &plaza.id);
    let cultivo = if tiene.is_empty() {
        "trigo".to_string()
    } else {
        // Antes que otra aldea, un pueblo extractor (la cantera, el estaño o el cobre: `marcar_pueblo`), si no lo tiene ya: el cupo de pueblos es corto.
        let extractores = entidades(ctx.puerto.observar()).any(|c| match c {
            Cuerpo::Pueblo(p) => p.asentamiento_madre_id == plaza.id && p.tipo_extractor != "aldeaAgricola",
            Cuerpo::Obra(o) => o.asentamiento_madre_id == plaza.id && o.tipo == pb::TipoObra::ObraPueblo as i32,
            _ => false,
        });
        let cupo = d.pueblos_por_nivel.get(&plaza.nivel_actual).copied().unwrap_or(0);
        let nivel = nivel_de_faccion(ctx, &plaza.faccion_id);
        // Luego la ganadera (la carne de la guarnición y la leche de la quesera, motor 2026-11-11) y después el mejor cultivo que su Facción ya tenga desbloqueado.
        let siguiente = d
            .cultivos
            .iter()
            .filter(|(c, n)| *n <= nivel && c != "trigo" && !tiene.contains(c))
            .max_by_key(|(c, n)| (c == "ganado", *n, c.clone()))
            .map(|(c, _)| c.clone());
        match siguiente.filter(|_| extractores && obras_de(ctx, &plaza.id).0 < cupo) {
            Some(c) => c,
            None => return Ok(()),
        }
    };
    // La ganadera pide la fertilidad de la capa del trigo: pasta donde se daría el trigo.
    let capa = if cultivo == "ganado" { "trigo".to_string() } else { cultivo.clone() };
    let centro = plaza.posicion.clone().unwrap_or_default();
    let mapa = crate::mapa::de(ctx.puerto).await?;
    let mut puntos: Vec<(u8, Punto)> = (0..)
        .map(|k| d.separacion_aldea + 500 + k * 500)
        .take_while(|r| *r <= plaza.radio_potencial)
        .flat_map(|r| (0..24).map(move |g| (r, g)))
        .map(|(r, g)| {
            let a = ((g * 15) as f64).to_radians();
            Punto { x: centro.x + (r as f64 * a.cos()) as i64, y: centro.y + (r as f64 * a.sin()) as i64 }
        })
        .filter(|p| en_zona(plaza, p))
        .map(|p| (mapa.fertilidad(&capa, &p), p))
        .filter(|(f, _)| *f >= d.fertilidad_minima)
        .collect();
    puntos.sort_by_key(|(f, _)| std::cmp::Reverse(*f));
    let mut probados = 0;
    for (_, punto) in puntos {
        let clave = format!("aldea:{}:{}:{}", plaza.id, punto.x, punto.y);
        if vetado(ctx, &clave) {
            continue;
        }
        let o = O::MarcarAldea(pb::OrdenMarcarAldea { cultivo: Some(cultivo.clone()), asentamiento_id: plaza.id.clone(), heroe_id: ctx.yo.into(), punto: Some(punto) });
        let r = ctx.puerto.actuar(o).await?;
        if r.resultado == pb::Resultado::OrdenAceptada as i32 {
            return Ok(());
        }
        // Sin cargo, sin nivel o sin cupo no es cosa del punto: lo intenta el turno que viene.
        if r.rechazo.as_ref().is_some_and(|x| ["no_autorizado", "pueblo.sin_cupo", "pueblo.nivel_insuficiente", "pueblo.cultivo_bloqueado"].contains(&x.codigo.as_str())) {
            return Ok(());
        }
        vetar(ctx, &clave, 24 * 3_600_000);
        probados += 1;
        if probados == 6 {
            break;
        }
    }
    Ok(())
}

/// El Gobernador marca un pueblo extractor (Economia_Construccion §14) en el yacimiento libre más cercano de la zona, hasta el cupo de su nivel: la obra queda pendiente
/// de materiales y la madre no paga nada todavía. El servidor dice si hay ruta, cupo y tecnología.
async fn marcar_pueblo<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let cupo = d.pueblos_por_nivel.get(&plaza.nivel_actual).copied().unwrap_or(0);
    // Un sitio del cupo es de la aldea, aunque aún no la haya encontrado.
    if plaza.nivel_actual < d.pueblos_nivel_madre || obras_de(ctx, &plaza.id).0 + usize::from(!tiene_aldea(ctx, &plaza.id)) >= cupo {
        return Ok(());
    }
    let centro = plaza.posicion.clone().unwrap_or_default();
    let tomados: Vec<(String, String)> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Pueblo(p) => Some((p.nodo_id.clone(), p.asentamiento_madre_id.clone())),
            Cuerpo::Obra(o) => Some((o.nodo_id.clone()?, o.asentamiento_madre_id.clone())),
            _ => None,
        })
        .collect();
    // Los yacimientos salen del mapa público (la vista no los trae todos): el más cercano libre dentro de la zona; el servidor dice si hay ruta y tecnología. Primero el estaño,
    // al que se funda a propósito con un solo sitio libre a nivel 1; luego la cantera, que no se construye en la plaza y su piedra mantiene el nivel 2; luego el cobre, sin el
    // que no hay fundición ni bronce.
    let mapa = crate::mapa::de(ctx.puerto).await?;
    let con_cantera = mapa.nodos.iter().any(|(id, tipo, _)| tipo == "piedra" && tomados.iter().any(|(n, madre)| n == id && *madre == plaza.id));
    let turno = |tipo: &str| match tipo {
        "estano" => 0,
        "piedra" if !con_cantera => 1,
        "cobre" => 2,
        _ => 3,
    };
    let nodo = mapa
        .nodos
        .iter()
        .filter(|(id, tipo, p)| !tomados.iter().any(|(n, _)| n == id) && tipo != "ganado" && distancia(p, &centro) <= plaza.radio_potencial && !vetado(ctx, &format!("pueblo:{id}")))
        .map(|(id, tipo, p)| (turno(tipo), distancia(p, &centro), id.clone()))
        .min();
    let Some((_, _, nodo)) = nodo else { return Ok(()) };
    if vetado(ctx, &format!("pueblo:{nodo}")) {
        return Ok(());
    }
    let o = O::MarcarPueblo(pb::OrdenMarcarPueblo { asentamiento_id: plaza.id.clone(), heroe_id: ctx.yo.into(), nodo_id: nodo.clone() });
    if ctx.intentar(&format!("pueblo:{nodo}"), o, ESPERA_TRAS_RECHAZO_MS).await?.is_some_and(|r| r.resultado == pb::Resultado::OrdenRechazada as i32) {
        vetar(ctx, &format!("pueblo:{nodo}"), 24 * 3_600_000);
    }
    Ok(())
}

/// Una avanzada (Doc 5.17) hacia la plaza ajena que conoce más cerca o, sin ninguna, en una dirección al azar: a 4/5 del radio de su zona, para ver más lejos. La coloca
/// un residente sin cargo con su columna en el sitio (`avanzada.fuera_del_sitio` si no): sale, marcha y, parado allí, la coloca.
/// ponytail: un punto y un rechazo; el sitio con el mapa (`comprobar_sitio`) cuando haga falta.
async fn colocar_avanzada<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let cupo = d.avanzadas_por_nivel * plaza.nivel_actual as usize;
    let clave = format!("avanzada:{}", plaza.id);
    let yo = ctx.yo.to_string();
    let libre = ctx.con_pizarra(|p| p.tablero.libre(&clave, &yo, ctx.ahora_ms));
    if plaza.nivel_actual < d.avanzada_nivel_madre || obras_de(ctx, &plaza.id).1 >= cupo || vetado(ctx, &clave) || !libre {
        return Ok(false);
    }
    let centro = plaza.posicion.clone().unwrap_or_default();
    let foco = politica::faccion_objetivo(ctx);
    let hacia = plazas_ajenas(ctx, &plaza.faccion_id).into_iter().min_by_key(|(_, f, p)| (foco.as_ref() != Some(f), distancia(p, &centro))).map(|(_, _, p)| p);
    let angulo = match hacia {
        Some(p) => ((p.y - centro.y) as f64).atan2((p.x - centro.x) as f64),
        None => ctx.azar.real() * std::f64::consts::TAU,
    };
    // Desde dentro de la plaza, como la aldea (motor, 2026-11-04): el primer punto de su zona en esa dirección, de fuera hacia dentro, que sea tierra que se pisa.
    let mapa = crate::mapa::de(ctx.puerto).await?;
    let sitio = [0.8, 0.65, 0.5]
        .into_iter()
        .flat_map(|f| [0.0, 0.4, -0.4].map(move |giro| (f, giro)))
        .map(|(f, giro)| {
            let (r, a) = (plaza.radio_potencial as f64 * f, angulo + giro);
            Punto { x: centro.x + (r * a.cos()) as i64, y: centro.y + (r * a.sin()) as i64 }
        })
        .find(|p| en_zona(plaza, p) && mapa.transitable(p));
    let Some(sitio) = sitio else {
        vetar(ctx, &clave, 6 * 3_600_000);
        return Ok(false);
    };
    let o = O::ColocarAvanzada(pb::OrdenColocarAvanzada { asentamiento_id: plaza.id.clone(), heroe_id: yo, posicion: Some(sitio) });
    if !aceptada(&ctx.intentar(&clave, o, ESPERA_TRAS_RECHAZO_MS).await?) {
        vetar(ctx, &clave, 6 * 3_600_000);
    }
    Ok(false)
}

/// ¿Cae `p` en la zona real de la plaza? El polígono que manda el motor, recortado por las fronteras; sin él, el círculo de su radio.
fn en_zona(plaza: &pb::Asentamiento, p: &Punto) -> bool {
    let z = &plaza.zona;
    if z.len() < 3 {
        return plaza.posicion.as_ref().is_some_and(|c| distancia(c, p) <= plaza.radio_potencial);
    }
    let mut dentro = false;
    let mut j = z.len() - 1;
    for i in 0..z.len() {
        let (a, b) = (&z[i], &z[j]);
        if (a.y > p.y) != (b.y > p.y) && (p.x as f64) < (b.x - a.x) as f64 * (p.y - a.y) as f64 / (b.y - a.y) as f64 + a.x as f64 {
            dentro = !dentro;
        }
        j = i;
    }
    dentro
}

/// Lleva a una obra de mapa de su plaza lo que le falta: la primera que nadie lleva y a la que la plaza tiene algo que llevar (la toma antes de salir: los compañeros
/// piensan a la vez), sale con el carro cargado del almacén de la plaza (lo que quepa), marcha a ella y lo entrega. Un porteador por obra.
async fn llevar_materiales<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let t = ctx.ahora_ms;
    let yo = ctx.yo.to_string();
    let obras: Vec<(String, Punto, BTreeMap<String, i64>)> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Obra(o) if o.asentamiento_madre_id == plaza.id && o.termina_en_ms.is_none() => {
                let falta: BTreeMap<String, i64> = o
                    .requerido
                    .as_ref()
                    .map(|r| r.por_id.iter().map(|(k, n)| (k.clone(), n - o.entregado.as_ref().and_then(|e| e.por_id.get(k)).copied().unwrap_or(0))).filter(|(_, n)| *n > 0).collect())
                    .unwrap_or_default();
                // Lleva lo que la plaza tiene de lo que falta (en unidades enteras).
                let carga: BTreeMap<String, i64> = falta
                    .into_iter()
                    .filter_map(|(r, n)| {
                        let hay = plaza.almacen.get(&r).map_or(0, |e| cantidad(e, t)) / 1000 * 1000;
                        let lleva = n.min(hay) / 1000 * 1000;
                        (lleva > 0).then_some((r, lleva))
                    })
                    .collect();
                (!carga.is_empty()).then(|| (o.id.clone(), o.posicion.clone().unwrap_or_default(), carga))
            }
            _ => None,
        })
        .collect();
    let elegida = obras.into_iter().find(|(obra, ..)| {
        let clave = format!("obra:{obra}");
        !vetado(ctx, &clave) && ctx.con_pizarra(|p| p.tablero.tomar(&clave, &yo, t, t + b::TAREA_VALE_MS))
    });
    let Some((obra, sitio, carga)) = elegida else { return Ok(false) };
    let clave = format!("obra:{obra}");
    let o = O::SalirAlMundo(pb::OrdenSalirAlMundo { asentamiento_id: plaza.id.clone(), heroe_id: yo.clone(), escuadron_ids: vec![], carga: Some(pb::Cantidades { por_id: carga }) });
    if !aceptada(&ctx.intentar(&clave, o, ESPERA_TRAS_RECHAZO_MS).await?) {
        ctx.con_pizarra(|p| p.tablero.soltar(&clave, &yo));
        vetar(ctx, &clave, 3_600_000);
        return Ok(false);
    }
    ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: yo, objetivo: Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(sitio)) }) })).await?;
    ctx.memoria.salida = Some(Salida::Entregar { obra });
    Ok(true)
}

/// Un pueblo activo sin carro de acarreo: el Gobernador lo repone (la madre paga carro y buey).
async fn reponer_carros<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let sin_carro: Vec<String> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Pueblo(p) if p.asentamiento_madre_id == plaza.id && p.carro_acarreo_id.is_none() => Some(p.id.clone()),
            _ => None,
        })
        .collect();
    for p in sin_carro {
        ctx.intentar(&format!("carro:{p}"), O::ReponerCarroDeAcarreo(pb::OrdenReponerCarroDeAcarreo { heroe_id: ctx.yo.into(), pueblo_id: p.clone() }), ESPERA_TRAS_RECHAZO_MS).await?;
    }
    Ok(())
}

// ───────────────────────────── Tecnologías y tropas ─────────────────────────────

/// Un paso hacia un edificio a cierto nivel: levantarlo, mejorar el que hay (su id, su tipo y el nivel al que sube) o lo que aún no deja.
#[derive(Debug, PartialEq)]
enum Paso {
    Levantar(String),
    Mejorar(String, String, u32),
    Falta(String),
}

impl Paso {
    fn texto(&self) -> String {
        match self {
            Paso::Levantar(t) => format!("levantar {t}"),
            Paso::Mejorar(_, t, n) => format!("subir {t} a nivel {n}"),
            Paso::Falta(x) => x.clone(),
        }
    }
}

/// Los hitos que no son edificios de la ciudad sino pueblos: la cantera (`marcar_pueblo`) y el corral de la aldea ganadera (`marcar_aldea`).
const HITOS_DE_PUEBLO: &[&str] = &["cantera", "corral"];

/// ¿Tiene la plaza `tipo` activo con nivel interno `n` o más?
fn activo_a(plaza: &pb::Asentamiento, tipo: &str, n: u32) -> bool {
    plaza.edificios.iter().any(|e| e.tipo == tipo && e.estado == EstadoEdificio::EdificioActivo as i32 && e.nivel_interno.unwrap_or(1) >= n)
}

/// Los pasos para tener `tipo` activo con nivel interno `nivel` en la plaza: levantarlo si no lo tiene o mejorar el más alto, y antes el edificio que pida esa mejora; lo que
/// aún no deja (el nivel de la plaza, una tecnología), como `Falta`. Con él en obra, ninguno: espera.
fn pasos_para(d: &Datos, plaza: &pb::Asentamiento, adoptadas: &[String], tipo: &str, nivel: u32, pasos: &mut Vec<Paso>) {
    let Some(e) = plaza.edificios.iter().filter(|e| e.tipo == tipo).max_by_key(|e| e.nivel_interno.unwrap_or(1)) else {
        let para = d.nivel_para_construir.get(tipo).copied().unwrap_or(1);
        pasos.push(match d.tecnologia_de_edificio.get(tipo).filter(|t| !adoptadas.contains(t)) {
            Some(t) => Paso::Falta(format!("{t} para levantar {tipo}")),
            None if para > plaza.nivel_actual => Paso::Falta(format!("la plaza a nivel {para} para levantar {tipo}")),
            None => Paso::Levantar(tipo.into()),
        });
        return;
    };
    let actual = e.nivel_interno.unwrap_or(1);
    if actual >= nivel || e.estado != EstadoEdificio::EdificioActivo as i32 {
        return;
    }
    let Some(m) = d.mejoras.get(&(tipo.to_string(), actual + 1)) else {
        pasos.push(Paso::Falta(format!("{tipo} no pasa del nivel {actual}")));
        return;
    };
    if m.nivel_plaza > plaza.nivel_actual {
        pasos.push(Paso::Falta(format!("la plaza a nivel {} para subir {tipo} a {}", m.nivel_plaza, actual + 1)));
        return;
    }
    if let Some((otro, n)) = &m.edificio {
        let antes = pasos.len();
        pasos_para(d, plaza, adoptadas, otro, *n, pasos);
        if pasos.len() > antes || !activo_a(plaza, otro, *n) {
            return;
        }
    }
    pasos.push(Paso::Mejorar(e.id.clone(), tipo.into(), actual + 1));
}

/// Las tecnologías de la Facción `f` (aparecidas, adoptadas) y el estado del servidor (la Era, sus contadores y las partes pendientes de los logros).
fn tecnologias<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, f: &str) -> (pb::TecnologiasFaccion, Option<pb::EstadoTecnologia>) {
    let a = ctx.puerto.observar();
    let propias = entidades(a).find_map(|c| match c {
        Cuerpo::Tecnologias(t) if t.faccion_id == f => Some(t.clone()),
        _ => None,
    });
    let estado = entidades(a).find_map(|c| match c {
        Cuerpo::EstadoTecnologia(e) => Some(e.clone()),
        _ => None,
    });
    (propias.unwrap_or_default(), estado)
}

fn capital_de<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, f: &str) -> Option<String> {
    entidades(ctx.puerto.observar()).find_map(|c| match c {
        Cuerpo::Faccion(x) if x.id == f => x.capital_id.clone(),
        _ => None,
    })
}

/// La plaza de la Facción que levanta los hitos de sus tecnologías: la de más nivel y, a igualdad, la capital y luego la de id más bajo.
fn plaza_mayor<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, f: &str) -> Option<String> {
    let capital = capital_de(ctx, f);
    entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Asentamiento(p) if p.faccion_id == f => Some(p),
            _ => None,
        })
        .max_by_key(|p| (p.nivel_actual, capital.as_deref() == Some(p.id.as_str()), std::cmp::Reverse(p.id.clone())))
        .map(|p| p.id.clone())
}

/// Las tecnologías que la Facción persigue, por prioridad ([`b::PRIORIDAD_TECNOLOGIA`]): las de su Era o de antes que aún no le han aparecido, con las partes de su hito
/// que no cumple. Los edificios cuentan en cualquiera de sus plazas (la cantera y el corral, en sus pueblos); el yacimiento no lo sabe y no lo da por cumplido.
fn pendientes<'d, C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, d: &'d Datos, f: &str) -> Vec<(&'d Tecnologia, Vec<&'d Hito>)> {
    let t = ctx.ahora_ms;
    let (tec, estado) = tecnologias(ctx, f);
    let era = estado.as_ref().and_then(|e| d.eras.get(&e.era)).copied().unwrap_or(1);
    let a = ctx.puerto.observar();
    let ids: Vec<String> = entidades(a)
        .filter_map(|c| match c {
            Cuerpo::Asentamiento(p) if p.faccion_id == f => Some(p.id.clone()),
            _ => None,
        })
        .collect();
    let pueblos: Vec<(String, Option<String>)> = entidades(a)
        .filter_map(|c| match c {
            Cuerpo::Pueblo(p) if ids.contains(&p.asentamiento_madre_id) => Some((p.tipo_extractor.clone(), p.cultivo.clone())),
            _ => None,
        })
        .collect();
    let plazas: Vec<pb::Asentamiento> = ids.iter().filter_map(|id| plaza(ctx, id)).collect();
    let capital = capital_de(ctx, f).and_then(|c| plazas.iter().find(|p| p.id == c));
    let cumple = |h: &Hito| match h {
        Hito::Edificio(x, _) if x == "cantera" => pueblos.iter().any(|(tipo, _)| tipo == "cantera"),
        Hito::Edificio(x, _) if x == "corral" => pueblos.iter().any(|(_, c)| c.as_deref() == Some("ganado")),
        Hito::Edificio(x, n) => plazas.iter().any(|p| activo_a(p, x, *n)),
        Hito::Tecnologia(x) => tec.adoptadas.contains(x),
        Hito::EnCapital(r) => capital.is_some_and(|p| p.almacen.get(r).is_some_and(|e| cantidad(e, t) > 0)),
        Hito::Capital(n, e) => capital.is_some_and(|p| p.nivel_actual >= *n && activo_a(p, e, 1)),
        Hito::Yacimiento(_) => false,
    };
    let mut v: Vec<(&Tecnologia, Vec<&Hito>)> = d
        .tecnologias
        .iter()
        .filter(|x| x.era <= era && !tec.aparecidas.contains(&x.id) && !tec.adoptadas.contains(&x.id))
        .map(|x| (x, x.hito.iter().filter(|h| !cumple(h)).collect()))
        .collect();
    v.sort_by_key(|(x, _)| (b::prioridad_tecnologia(&x.id), x.era, x.id.clone()));
    v
}

fn texto_de_hito(h: &Hito) -> String {
    match h {
        Hito::Edificio(x, 1) => x.clone(),
        Hito::Edificio(x, n) => format!("{x} a nivel {n}"),
        Hito::Tecnologia(x) => format!("antes {x}"),
        Hito::EnCapital(r) => format!("{r} en la capital"),
        Hito::Capital(n, e) => format!("la capital a nivel {n} con {e}"),
        Hito::Yacimiento(r) => format!("un yacimiento de {r} en su zona"),
    }
}

/// Lo que le falta del logro del servidor a una tecnología: cada parte pendiente, con lo que lleva el mundo y su umbral de ahora (el de una parte por plaza es su cifra por
/// las plazas vivas del mundo, y así se reconoce entre las pendientes).
fn texto_del_logro(x: &Tecnologia, estado: Option<&pb::EstadoTecnologia>) -> Option<String> {
    let e = estado?;
    let partes: Vec<String> = x
        .logro
        .iter()
        .filter(|(c, cifra, por_plaza)| {
            let umbral = if *por_plaza { cifra * e.plazas.max(1) as i64 } else { *cifra };
            e.logros_pendientes.iter().any(|p| p.contador == *c && p.umbral == umbral)
        })
        .map(|(c, cifra, por_plaza)| {
            let umbral = if *por_plaza { cifra * e.plazas.max(1) as i64 } else { *cifra };
            format!("{c} {} de {umbral}", e.contadores.get(c).copied().unwrap_or(0))
        })
        .collect();
    (!partes.is_empty()).then(|| partes.join(" y "))
}

/// Las tropas que la Facción sabe hacer (sin tecnología o con la suya adoptada), de la mejor a la peor: escalón, poder e id.
fn tropas_por_orden<'d>(d: &'d Datos, adoptadas: &[String]) -> Vec<&'d Tropa> {
    let mut v: Vec<&Tropa> = d.tropas.iter().filter(|x| x.tecnologia.as_ref().is_none_or(|y| adoptadas.contains(y))).collect();
    v.sort_by_key(|x| (std::cmp::Reverse(x.escalon), std::cmp::Reverse(d.poder.get(&x.id).copied().unwrap_or(0)), x.id.clone()));
    v
}

/// Lo que le falta a la plaza para pagar `n` soldados de la tropa: el equipo (por bien) y el oro, en milésimas.
fn falta_para(x: &Tropa, plaza: &pb::Asentamiento, n: u32, t: i64) -> (Vec<(String, i64)>, i64) {
    let hay = |r: &str| plaza.almacen.get(r).map_or(0, |e| cantidad(e, t));
    let n = n as i64 * 1000;
    let equipo = x.equipo.iter().filter_map(|(r, por)| Some((r.clone(), por * n - hay(r))).filter(|(_, falta)| *falta > 0)).collect();
    (equipo, (x.oro * n - hay("oro")).max(0))
}

/// La tropa siguiente de la plaza: la que tiene que llegar a reclutar.
struct Siguiente<'d> {
    tropa: &'d Tropa,
    /// Lo que pide su edificio (levantarlo, subirlo) y lo que no deja aún.
    pasos: Vec<Paso>,
    /// El equipo y el oro que faltan para una escuadra entera, en milésimas.
    equipo: Vec<(String, i64)>,
    oro: i64,
}

/// Entre las tropas que la Facción sabe hacer y son mejores que la mejor que la plaza ya recluta y paga, la más cercana a esa que la plaza puede ir haciendo (sin nada que
/// aún no deje); si todas esperan algo así, la más cercana.
fn tropa_siguiente<'d>(d: &'d Datos, plaza: &pb::Asentamiento, adoptadas: &[String], t: i64) -> Option<Siguiente<'d>> {
    let v = tropas_por_orden(d, adoptadas);
    let lista = |x: &Tropa| activo_a(plaza, &x.edificio, x.nivel_edificio) && falta_para(x, plaza, x.unidades, t) == (Vec::new(), 0);
    let tope = v.iter().position(|x| lista(x)).unwrap_or(v.len());
    let candidatas: Vec<Siguiente> = v[..tope]
        .iter()
        .rev()
        .map(|x| {
            let mut pasos = Vec::new();
            pasos_para(d, plaza, adoptadas, &x.edificio, x.nivel_edificio, &mut pasos);
            let (equipo, oro) = falta_para(x, plaza, x.unidades, t);
            Siguiente { tropa: x, pasos, equipo, oro }
        })
        .collect();
    let i = candidatas.iter().position(|s| !s.pasos.iter().any(|p| matches!(p, Paso::Falta(_)))).unwrap_or(0);
    candidatas.into_iter().nth(i)
}

// ───────────────────────────── Residente ─────────────────────────────

/// La meta de cada bot es una escuadra entera de cada tropa que su plaza sabe hacer (las que sobran van a las guarniciones): recluta (o repone), una por turno, la mejor que
/// le falte y pueda pagar (la de escalón y poder más altos con su edificio activo a su nivel, su tecnología adoptada y en el almacén el equipo y el oro de lo que le falta);
/// si el servidor rechaza una (la población), prueba la siguiente en el turno de después.
async fn reclutar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let t = ctx.ahora_ms;
    let (tec, _) = tecnologias(ctx, &plaza.faccion_id);
    let mias: Vec<(String, u32)> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Escuadron(e) if e.heroe_id == ctx.yo && e.prestada_por_campamento_id.is_none() && e.cantidad > 0 => Some((e.tropa_id.clone(), e.cantidad)),
            _ => None,
        })
        .collect();
    let ya = |id: &str| mias.iter().filter(|(x, _)| x == id).map(|(_, n)| *n).max().unwrap_or(0);
    let tropas = tropas_por_orden(d, &tec.adoptadas);
    let por_completar = tropas.iter().filter(|x| activo_a(plaza, &x.edificio, x.nivel_edificio) && ya(&x.id) < x.unidades).count();
    let (yo, soldados) = (ctx.yo.to_string(), mias.iter().map(|(_, n)| n).sum::<u32>());
    ctx.con_pizarra(|p| p.soldados.insert(yo, (soldados, por_completar)));
    // Reclutar gasta la población que pide el ascenso (M9): con una escuadra propia ya, no recluta mientras la plaza no tenga los pesants de la subida siguiente.
    let pesants = plaza.poblacion.as_ref().and_then(|p| p.pesants.as_ref()).map_or(0, |r| r.valor_en_t0);
    if b::LEVA_ESPERA_AL_ASCENSO && !mias.is_empty() && d.requisitos.get(&(plaza.nivel_actual + 1)).is_some_and(|r| pesants < r.pesants) {
        return Ok(());
    }
    for x in tropas {
        let faltan = x.unidades.saturating_sub(ya(&x.id));
        let clave = format!("reclutar:{}", x.id);
        if faltan == 0 || !activo_a(plaza, &x.edificio, x.nivel_edificio) || ctx.esperando(&clave) || falta_para(x, plaza, faltan, t) != (Vec::new(), 0) {
            continue;
        }
        let o = O::ReclutarTropa(pb::OrdenReclutarTropa { asentamiento_id: plaza.id.clone(), heroe_id: ctx.yo.into(), tropa_id: x.id.clone(), origen: None });
        ctx.intentar(&clave, o, ESPERA_TRAS_RECHAZO_MS).await?;
        break;
    }
    Ok(())
}

/// Lo mejor para él y lo demás a las guarniciones: retira de la guarnición de la plaza las escuadras que entran en lo que lleva al salir ([`para_llevar`]) y guarnece con
/// las que le sobran, de la más fuerte a la más débil, hasta llenar el cupo. Lo que no cabe, a sus pueblos y avanzadas ([`guarnecer_pueblo`], [`guarnecer_avanzada`]).
async fn guarnecer<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let todas = propias(ctx);
    let lleva = para_llevar(d, &todas, liderazgo(ctx, d));
    for e in todas.iter().filter(|e| e.en_guarnicion && lleva.contains(&e.id)) {
        ctx.intentar(&format!("retirar:{}", e.id), O::RetirarGuarnicion(pb::OrdenRetirarGuarnicion { squad_id: e.id.clone() }), ESPERA_TRAS_RECHAZO_MS).await?;
    }
    let a = ctx.puerto.observar();
    let dentro: u32 = entidades(a)
        .filter_map(|c| match c {
            Cuerpo::Escuadron(e) if plaza.guarnicion_ids.contains(&e.id) => Some(e.cantidad),
            _ => None,
        })
        .sum();
    let mut sobran: Vec<&pb::Escuadron> = todas.iter().filter(|e| !e.en_guarnicion && !lleva.contains(&e.id)).collect();
    sobran.sort_by_key(|e| (std::cmp::Reverse(d.poder_de(e)), e.id.clone()));
    let mut cabe = plaza.cupo_guarnicion.saturating_sub(dentro);
    for e in sobran {
        if e.cantidad > cabe {
            continue;
        }
        let r = ctx.intentar(&format!("guarnicion:{}", e.id), O::AsignarGuarnicion(pb::OrdenAsignarGuarnicion { squad_id: e.id.clone() }), ESPERA_TRAS_RECHAZO_MS).await?;
        if r.is_some_and(|r| r.resultado == pb::Resultado::OrdenAceptada as i32) {
            cabe -= e.cantidad;
        }
    }
    Ok(())
}

// ───────────────────────────── Salir de casa ─────────────────────────────

/// Lo que vale una inspección: más vieja, la defensa de una plaza ya no dice nada.
const VIGENCIA_INSPECCION_MS: i64 = 2 * 3_600_000;
/// Margen sobre la defensa vista: los héroes que había dentro defienden con lo suyo, que no se ve. ponytail: el de TypeScript, a calibrar con el batch.
const MARGEN_SOBRE_DEFENSA: f64 = 1.5;
/// Desde qué nivel sale de campaña una plaza, y lo que espera el que lleva la campaña en la puerta a que se le unan.
const NIVEL_PARA_CAMPANA: u32 = 2;
/// El poder que se supone a la escuadra de un héroe ajeno, sin inspeccionarlo.
const PODER_TIPICO_DE_ESCUADRA: u32 = 100;
const ESPERA_CAMPANA_MS: i64 = 10 * 60_000;

/// Lo que hace fuera quien salió de su plaza; si se pierde (reinicio), vuelve a casa.
pub(crate) enum Salida {
    /// Va a por un campamento de bandidos cerca de su plaza.
    Cazar { bandidos: String },
    /// Espera en la puerta, desde `inicio`, a los compañeros para ir a por un campamento que no puede solo.
    CazarEnGrupo { bandidos: String, inicio: i64 },
    /// El buscador de su Facción recorre la rejilla de su región hasta ver estaño a su alcance; va por el tramo `paso`.
    BuscarEstano { paso: u32 },
    /// El explorador de su Facción va a inspeccionar una plaza ajena.
    Explorar { plaza: String },
    /// El explorador, sin plaza ajena conocida, recorre el mapa a buscarlas; va por el paso `paso`.
    Buscar { paso: u32 },
    /// De campaña contra una plaza; `lidera`, la lleva él; `marcha`, ya salió de la puerta hacia ella; `inicio`, cuándo empezó a reunir a su grupo.
    Campana { plaza: String, lidera: bool, marcha: bool, inicio: i64 },
    /// Lleva la Caravana de Fundación de su plaza a `sitio`; `intentos`, los sitios donde `fundar` ya dijo que no.
    Fundar { sitio: Punto, intentos: u32 },
    /// Con la Caravana de Fundación, espera en la puerta desde `inicio` a que se le unan bastantes para pasar el camino hasta `sitio`.
    FundarEnGrupo { sitio: Punto, caravana: String, inicio: i64 },
    /// Lleva materiales a una obra de mapa de su plaza.
    Entregar { obra: String },
    /// Va a saquear un pueblo ajeno.
    Saquear { pueblo: String },
    /// Lleva una escuadra a guarnecer una avanzada de su plaza.
    GuarnecerAvanzada { avanzada: String, escuadra: String },
    /// Persigue una caravana ajena para asaltarla.
    Asaltar { caravana: String },
    /// Lleva una escuadra a guarnecer un pueblo de su plaza.
    GuarnecerPueblo { pueblo: String, escuadra: String },
    /// Persigue a la columna de una Facción odiada.
    Perseguir { ejercito: String },
}

impl Salida {
    /// La tarea del tablero de su Facción que lleva quien sale a esto: la renueva mientras sigue y la suelta al acabar.
    pub(crate) fn tarea(&self, casa: &str) -> Option<String> {
        match self {
            Self::Cazar { bandidos } | Self::CazarEnGrupo { bandidos, .. } => Some(format!("cazar:{bandidos}")),
            Self::Explorar { .. } => Some("explorador".into()),
            Self::Buscar { .. } => Some(format!("batida:{casa}")),
            Self::Campana { plaza, lidera: true, .. } => Some(format!("campana:{plaza}")),
            Self::Entregar { obra } => Some(format!("obra:{obra}")),
            Self::Saquear { pueblo } => Some(format!("saquear:{pueblo}")),
            Self::Asaltar { caravana } => Some(format!("asaltar:{caravana}")),
            Self::GuarnecerAvanzada { avanzada, .. } => Some(format!("guarnecer:{avanzada}")),
            Self::GuarnecerPueblo { pueblo, .. } => Some(format!("guarnecer:{pueblo}")),
            _ => None,
        }
    }

    /// Para el panel (`volcarMemoria`): los `tipo` que conoce llevan sus campos; el resto es una frase, que pinta tal cual.
    pub(crate) fn volcar(&self) -> serde_json::Value {
        use serde_json::json;
        match self {
            Self::Cazar { bandidos } | Self::CazarEnGrupo { bandidos, .. } => json!({ "tipo": "cazar", "campamentoId": bandidos }),
            Self::Explorar { plaza } => json!({ "tipo": "explorar", "plazaId": plaza }),
            Self::Buscar { paso } => json!({ "tipo": format!("Buscar plazas ajenas (tramo {paso})") }),
            Self::BuscarEstano { paso, .. } => json!({ "tipo": format!("Buscar estaño (tramo {paso})") }),
            Self::Campana { plaza, lidera, .. } => json!({ "tipo": "campana", "plazaId": plaza, "lidera": lidera }),
            Self::FundarEnGrupo { sitio, .. } => json!({ "tipo": format!("Esperar a su grupo para fundar en ({}, {})", sitio.x / 1000, sitio.y / 1000) }),
            Self::Fundar { sitio, intentos } => json!({ "tipo": format!("Fundar en ({}, {}), {intentos} sitios descartados", sitio.x / 1000, sitio.y / 1000) }),
            Self::Entregar { obra } => json!({ "tipo": format!("Llevar materiales a {obra}") }),
            Self::Saquear { pueblo } => json!({ "tipo": format!("Saquear {pueblo}") }),
            Self::GuarnecerAvanzada { avanzada, .. } => json!({ "tipo": format!("Guarnecer {avanzada}") }),
            Self::Asaltar { caravana } => json!({ "tipo": format!("Asaltar {caravana}") }),
            Self::GuarnecerPueblo { pueblo, .. } => json!({ "tipo": format!("Guarnecer {pueblo}") }),
            Self::Perseguir { ejercito } => json!({ "tipo": format!("Perseguir {ejercito}") }),
        }
    }
}

/// Residentes que tiene que tener una plaza para que uno salga a fundar (se quedan al menos dos), y dónde busca sitio alrededor de ella
/// (`crates/batch/src/sim.rs`, `ANILLO_PLAZA`). ponytail: cifras del batch, a calibrar.
const RESIDENTES_PARA_EXPANDIR: usize = 3;
const ANILLO_PLAZA: (i64, i64) = (30_000, 70_000);
/// A qué distancia de un yacimiento de estaño funda para tenerlo en su zona inicial.
const ANILLO_ESTANO: (i64, i64) = (2_000, 5_000);
/// Cuánto puede apartarse del camino una etapa hacia el estaño.
const ANILLO_ETAPA: (i64, i64) = (0, 15_000);
const INTENTOS_DE_FUNDAR: u32 = 3;
/// La batida del explorador que no conoce plazas ajenas: hasta dónde se aleja de cada punto, cuántos tramos da y cada cuánto vuelve a salir. ponytail: a calibrar.
const BATIDA: (i64, i64) = (200_000, 600_000);
const PASOS_DE_BATIDA: u32 = 8;

fn vetado<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, clave: &str) -> bool {
    let t = ctx.ahora_ms;
    ctx.con_pizarra(|p| p.vetos.get(clave).is_some_and(|&hasta| hasta > t))
}

fn vetar<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, clave: &str, ms: i64) {
    let hasta = ctx.ahora_ms + ms;
    ctx.con_pizarra(|p| p.vetos.insert(clave.into(), hasta));
}

/// Reposta a la puerta de `lugar` (su plaza, o la más cercana si falta): §15, antes de salir; el servidor dice si hay sitio y trigo.
async fn repostar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, lugar: Option<String>) -> Result<(), Fallo> {
    ctx.puerto.actuar(O::Repostar(pb::OrdenRepostar { heroe_id: ctx.yo.into(), lugar_id: lugar })).await?;
    Ok(())
}

/// Margen sobre el trigo justo de ida y vuelta: el camino no es recto y el bot piensa cada pocos minutos. ponytail: el de TypeScript, a calibrar.
const MARGEN_VIVERES: f64 = 1.2;

/// ¿Le llegan los víveres de su columna para ir a `destino` y volver? §15: el alcance sale de los víveres del ejército (su ritmo, que baja más en marcha que parado)
/// y de la tropa más lenta. Sin consumo (sin tropa) o sin ritmo a la vista, no hay límite.
fn le_llega<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, d: &Datos, destino: &Punto) -> bool {
    let a = ctx.puerto.observar();
    let Some(col) = columna(a, ctx.yo) else { return true };
    let Some(v) = col.viveres.as_ref() else { return true };
    let lenta = entidades(a)
        .filter_map(|c| match c {
            Cuerpo::Escuadron(e) if col.escuadron_ids.contains(&e.id) => d.velocidad.get(&e.tropa_id).copied(),
            _ => None,
        })
        .fold(f64::MAX, f64::min);
    if lenta == f64::MAX {
        return true;
    }
    alcanza(v, ctx.ahora_ms, d.marcha_sobre_parada, lenta, distancia(&posicion(&col, ctx.ahora_ms), destino))
}

/// Las cuentas de [`le_llega`]: con los víveres `v` en `t` (que bajan `marcha` veces más rápido en marcha), a `velocidad` unidades por minuto, ¿ida y vuelta de `milesimas`?
fn alcanza(v: &pb::Ritmo, t: i64, marcha: f64, velocidad: f64, milesimas: i64) -> bool {
    if v.por_minuto >= 0 || velocidad <= 0.0 {
        return true;
    }
    let ahora = (v.valor_en_t0 + v.por_minuto * (t - v.t0_ms).max(0) / 60_000).max(0) as f64;
    let minutos = ahora / (-v.por_minuto as f64 * marcha);
    2.0 * (milesimas as f64 / 1000.0 / velocidad) * MARGEN_VIVERES <= minutos
}

/// Sus escuadras propias con gente en el campamento de su plaza, dentro o fuera de la guarnición (no las de un pueblo, una avanzada, una columna o una escolta).
fn propias<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>) -> Vec<pb::Escuadron> {
    use pb::contenedor_escuadron::Donde as En;
    entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Escuadron(e)
                if e.heroe_id == ctx.yo && e.cantidad > 0 && e.prestada_por_campamento_id.is_none() && matches!(e.contenedor.as_ref().and_then(|x| x.donde.as_ref()), None | Some(En::Campamento(_))) =>
            {
                Some(e.clone())
            }
            _ => None,
        })
        .collect()
}

/// Su Liderazgo: el suyo o, sin él, el de todo héroe.
fn liderazgo<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, d: &Datos) -> u32 {
    crate::cerebro::heroe(ctx.puerto.observar(), ctx.yo).map(|h| h.liderazgo_base).filter(|l| *l > 0).unwrap_or(d.liderazgo)
}

/// Lo que cabe en su Liderazgo de `escuadras` ([`para_llevar`]): para quien sale de un campamento o de visita, sin guarniciones que llenar.
pub(crate) fn lo_que_cabe(a: &contrato::almacen::Almacen, yo: &str, d: &Datos, escuadras: &[pb::Escuadron]) -> Vec<String> {
    let liderazgo = crate::cerebro::heroe(a, yo).map(|h| h.liderazgo_base).filter(|l| *l > 0).unwrap_or(d.liderazgo);
    para_llevar(d, escuadras, liderazgo)
}

/// Lo que lleva al salir, de sus `escuadras`: las mejores (por poder) que caben en su `liderazgo`, cada una con el coste del escalón de su tropa. Lo demás es para las
/// guarniciones. ponytail: voraz, no la mejor combinación posible; basta mientras el coste va por escalón y el poder sube con él.
fn para_llevar(d: &Datos, escuadras: &[pb::Escuadron], liderazgo: u32) -> Vec<String> {
    let mut v: Vec<&pb::Escuadron> = escuadras.iter().collect();
    v.sort_by_key(|e| (std::cmp::Reverse(d.poder_de(e)), e.id.clone()));
    let mut usado = 0;
    let mut lleva = Vec::new();
    for e in v {
        let coste = d.coste_liderazgo_de(&e.tropa_id);
        if usado + coste <= liderazgo {
            usado += coste;
            lleva.push(e.id.clone());
        }
    }
    lleva
}

/// Las escuadras con las que sale: lo mejor que cabe en su Liderazgo ([`para_llevar`]) de lo que está fuera de la guarnición (lo que esté dentro lo retira [`guarnecer`]).
fn libres<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, d: &Datos) -> Vec<pb::Escuadron> {
    let todas = propias(ctx);
    let lleva = para_llevar(d, &todas, liderazgo(ctx, d));
    todas.into_iter().filter(|e| !e.en_guarnicion && lleva.contains(&e.id)).collect()
}

/// Las que le sobran: fuera de la guarnición y de lo que lleva al salir. Son las de las guarniciones (la plaza, sus pueblos, sus avanzadas).
fn sobrantes<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, d: &Datos) -> Vec<pb::Escuadron> {
    let todas = propias(ctx);
    let lleva = para_llevar(d, &todas, liderazgo(ctx, d));
    todas.into_iter().filter(|e| !e.en_guarnicion && !lleva.contains(&e.id)).collect()
}

fn aceptada(r: &Option<pb::ResultadoOrden>) -> bool {
    r.as_ref().is_some_and(|r| r.resultado == pb::Resultado::OrdenAceptada as i32)
}

/// Las plazas ajenas que conoce (las que ve y las que recuerda su Facción): id, Facción y posición. Lo visto gana.
pub(crate) fn plazas_ajenas<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, mia: &str) -> Vec<(String, String, Punto)> {
    // Lo que se ve ahora gana a lo apuntado (una plaza conquistada cambia de dueña, también a la propia); lo apuntado cubre lo que cae fuera de la vista.
    let mut vistas: BTreeMap<String, (String, Punto)> = BTreeMap::new();
    for c in entidades(ctx.puerto.observar()) {
        match c {
            Cuerpo::FichaConocida(f) if !f.faccion_id.is_empty() && f.asentamiento_id.starts_with("as-") => {
                vistas.entry(f.asentamiento_id.clone()).or_insert((f.faccion_id.clone(), f.posicion.clone().unwrap_or_default()));
            }
            Cuerpo::Asentamiento(a) => {
                vistas.insert(a.id.clone(), (a.faccion_id.clone(), a.posicion.clone().unwrap_or_default()));
            }
            _ => {}
        }
    }
    let todas = ctx.con_pizarra(|p| {
        p.ajenas.extend(vistas);
        p.ajenas.clone()
    });
    todas.into_iter().filter(|(_, (f, _))| f != mia).map(|(id, (f, p))| (id, f, p)).collect()
}

/// ¿Es aliada o vasalla (o señora) de su Facción?
fn aliada<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, mia: &str, otra: &str) -> bool {
    entidades(ctx.puerto.observar()).any(|c| {
        matches!(c, Cuerpo::Relacion(r) if r.estado == pb::EstadoRelacion::RelacionActiva as i32 && r.tipo != pb::TipoRelacion::RelacionGuerra as i32
            && ((r.faccion_a_id == mia && r.faccion_b_id == otra) || (r.faccion_a_id == otra && r.faccion_b_id == mia)))
    })
}

/// El que está en casa sin cargo decide si sale (§16, TypeScript `militar.ts`): a unirse a la campaña que sale de casa, a cazar el campamento de bandidos de su plaza, a
/// explorar para su Facción, de campaña contra lo que puede ganar o a vivir a la plaza de su Facción que menos residentes tiene.
async fn salir<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento, cargos: &pb::Cargos) -> Result<(), Fallo> {
    let yo = ctx.yo.to_string();
    apuntar_necesidades(ctx, d, plaza).await?;
    // El General sí sale: es el de más puntos de guerra y lleva las campañas.
    let cargo = [("gobernador", &cargos.gobernador_id), ("tesorero", &cargos.tesorero_id), ("maestro de obras", &cargos.maestro_obras_id), ("sacerdote", &cargos.sacerdote_id)]
        .into_iter()
        .find(|(_, c)| c.as_deref() == Some(yo.as_str()));
    if let Some((cargo, _)) = cargo {
        ctx.memoria.en_casa = Some(format!("En casa: es {cargo} de la plaza y no sale"));
        return Ok(());
    }
    if defender(ctx).await? || unirse_a_la_campana(ctx, d, plaza).await? {
        return Ok(());
    }
    // Primero lo que pide el tablero, por prioridad (a igual prioridad, las necesidades antes que las metas); después, lo de siempre y lo que su personalidad mira antes
    // (fundar, cazar, explorar o salir de campaña; la escalera decide si hay a quién acosar).
    use Accion::*;
    let t = ctx.ahora_ms;
    let mut orden: Vec<Accion> = ctx.con_pizarra(|p| p.tablero.para_libres(&plaza.id, &yo, t)).iter().filter_map(|k| accion_de(k)).collect();
    orden.extend([Materiales, GuarnecerAvanzada, GuarnecerPueblo, Escoltar, CazarALaVista, BuscarEstano]);
    orden.extend(match Personalidad::de(&plaza.faccion_id) {
        Personalidad::Expansionista => [Fundar, Cazar, Explorar, Campana, Saquear, Asaltar],
        Personalidad::Mercantil => [Cazar, Fundar, Explorar, Campana, Asaltar, Saquear],
        Personalidad::Militarista => [Cazar, Saquear, Asaltar, Explorar, Campana, Fundar],
    });
    let mut probadas: Vec<Accion> = Vec::new();
    for a in orden {
        if probadas.contains(&a) {
            continue;
        }
        probadas.push(a);
        if hacer(ctx, d, plaza, a).await? {
            return Ok(());
        }
    }
    repartirse(ctx, d, plaza).await?;
    if ctx.memoria.salida.is_none() {
        ctx.memoria.en_casa = Some(por_que_no_sale(ctx, d, plaza));
    }
    Ok(())
}

/// El tablero de su Facción desde su plaza (lo apunta cualquiera de sus residentes al pensar; lo que deja de hacer falta caduca). Las **necesidades** son de supervivencia:
/// la comida que le falta (el Tesorero), los campamentos de bandidos de alrededor y sus pueblos sin guarnición (uno libre), la guarnición de la plaza bajo la mitad del cupo
/// y sus residentes sin tropa (cada residente). Las **metas** son de progreso, con su prioridad y sus pasos, cada uno de alguien (un cargo con su titular, uno libre, cada
/// residente, o el mundo, que llega solo): asegurar el estaño, el bronce del nivel 3, las tecnologías y su adopción, sus obras, explorar alrededor, la tropa siguiente y
/// sus escuadras, y la comida de lujo para crecer.
async fn apuntar_necesidades<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    use crate::tablero::{Meta, Necesidad, PasoDeMeta};
    let t = ctx.ahora_ms;
    let hasta = t + b::NECESIDAD_VALE_MS;
    let casa = plaza.posicion.clone().unwrap_or_default();
    let f = plaza.faccion_id.clone();
    let pid = plaza.id.clone();
    let paso = |clave: String, texto: String, quien: &str| PasoDeMeta { clave, texto, quien: quien.into() };
    let necesidad = |texto: String, prioridad: u8, quien: &str| Necesidad { texto, prioridad, plaza: pid.clone(), hasta, quien: quien.into() };
    let meta = |texto: String, prioridad: u8, plaza: String, pasos: Vec<PasoDeMeta>| Meta { texto, prioridad, plaza, hasta, pasos };
    let mut necesidades: Vec<(String, Necesidad)> = Vec::new();
    let mut nuevas: Vec<(String, Meta)> = Vec::new();

    // Necesidades: los bandidos de alrededor y los pueblos sin guarnición (uno libre), la guarnición de la plaza y los residentes sin tropa (cada residente).
    let mut obras: Vec<PasoDeMeta> = Vec::new();
    for c in entidades(ctx.puerto.observar()) {
        match c {
            Cuerpo::Obra(o) if o.asentamiento_madre_id == plaza.id && o.termina_en_ms.is_none() => {
                obras.push(paso(format!("obra:{}", o.id), format!("Llevar materiales a la obra {}", o.id), "libre"));
            }
            Cuerpo::CampamentoBandidos(x) if x.asentamiento_id.as_deref() == Some(plaza.id.as_str()) || x.posicion.as_ref().is_some_and(|p| distancia(p, &casa) <= b::RADIO_DE_CAZA) => {
                necesidades.push((format!("cazar:{}", x.id), necesidad(format!("Limpiar el campamento de bandidos {} (poder {}) cerca de {}", x.id, x.poder, plaza.id), 2, "libre")));
            }
            Cuerpo::Pueblo(x) if x.asentamiento_madre_id == plaza.id && x.guarnicion_escuadron_id.is_none() => {
                necesidades.push((format!("guarnecer:{}", x.id), necesidad(format!("Guarnecer el pueblo {} de {}", x.id, plaza.id), 3, "libre")));
            }
            _ => {}
        }
    }
    let (sin_tropa, por_completar) = ctx.con_pizarra(|p| {
        let de: Vec<(u32, usize)> = plaza.casas_compradas.iter().filter_map(|h| p.soldados.get(h).copied()).collect();
        (de.iter().filter(|(n, _)| *n == 0).count(), de.iter().map(|(_, k)| k).sum::<usize>())
    });
    if sin_tropa > 0 {
        necesidades.push((format!("ejercito:{}", plaza.id), necesidad(format!("Reclutar en {}: {sin_tropa} residentes sin tropa", plaza.id), 2, "residentes")));
    }
    let dentro: u32 = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Escuadron(e) if plaza.guarnicion_ids.contains(&e.id) => Some(e.cantidad),
            _ => None,
        })
        .sum();
    if (dentro as f64) < plaza.cupo_guarnicion as f64 * b::GUARNICION_MINIMA {
        necesidades.push((format!("guarnicion:{}", plaza.id), necesidad(format!("Guarnecer {}: {dentro} de {} soldados", plaza.id, plaza.cupo_guarnicion), 2, "residentes")));
    }
    // Lo que le falta de lo que guarda ([`metas`]): la comida es necesidad; lo demás, un paso del Tesorero en la meta de su razón.
    let mut por_razon: BTreeMap<&'static str, Vec<PasoDeMeta>> = BTreeMap::new();
    for (bien, (objetivo, por)) in metas(ctx, d, plaza) {
        let hay = plaza.almacen.get(&bien).map_or(0, |e| cantidad(e, t));
        if hay >= objetivo {
            continue;
        }
        let texto = format!("Conseguir {} de {bien} para {por} (hay {})", (objetivo - hay) / 1000, hay / 1000);
        if b::clase_de_bien(&bien) == "comida" {
            necesidades.push((format!("comprar:{bien}"), necesidad(format!("{texto} en {}", plaza.id), 1, "tesorero")));
        } else {
            por_razon.entry(por).or_default().push(paso(format!("comprar:{bien}"), texto, "tesorero"));
        }
    }

    // Metas. El estaño, de toda la Facción: buscarlo hasta que sepa de uno libre (o ya lo saque) e ir a por él por la vía del libro.
    let mut estano: Vec<PasoDeMeta> = Vec::new();
    if !estano_resuelto(ctx, &f) && !estano_a_su_alcance(ctx, &casa) {
        estano.push(paso("buscador".into(), "Buscar estaño donde nadie de la Facción ha mirado".into(), "libre"));
    }
    if let Some(e) = ctx.con_pizarra(|p| p.libro.estano.clone()) {
        estano.push(match &e.via {
            Via::Propio => paso(String::new(), format!("Marcar la mina de estaño de {}", e.nodo), "gobernador"),
            Via::Fundar => paso(format!("fundar:{}", plaza.id), format!("Fundar junto al estaño de {}", e.nodo), "libre"),
            Via::Trueque { faccion } => paso(String::new(), format!("Conseguir estaño de {faccion} por trueque"), "tesorero"),
            Via::Guerra { faccion } => paso(String::new(), format!("Quitarle a {faccion} el estaño de {}", e.nodo), "general"),
        });
    }
    if !estano.is_empty() {
        nuevas.push(("meta:estano".into(), meta("Asegurar el estaño (sin él no hay bronce)".into(), 1, String::new(), estano)));
    }
    if let Some(pasos) = por_razon.remove("el bronce del nivel 3") {
        nuevas.push((format!("meta:bronce:{}", plaza.id), meta(format!("El bronce del nivel 3 en {}", plaza.id), 1, plaza.id.clone(), pasos)));
    }
    // Las tecnologías, en la plaza mayor de la Facción: cada parte del hito que falta (el Gobernador levanta y mejora, el Tesorero lleva a la capital) o, con él cumplido,
    // lo que pide el mundo (los campamentos de bandidos los puede destruir uno libre). Y la adopción de las que ya puede, del Rey en la capital.
    let (tec, estado) = tecnologias(ctx, &f);
    let (mayor, capital) = (plaza_mayor(ctx, &f).unwrap_or_default(), capital_de(ctx, &f).unwrap_or_default());
    for (x, faltan) in pendientes(ctx, d, &f) {
        let mut pasos: Vec<PasoDeMeta> = faltan
            .iter()
            .map(|h| match h {
                Hito::Edificio(tipo, _) if HITOS_DE_PUEBLO.contains(&tipo.as_str()) => paso(String::new(), format!("{} (un pueblo)", texto_de_hito(h)), "gobernador"),
                Hito::Edificio(..) => paso(format!("tecnologia:{}", x.id), format!("{} en {mayor}", texto_de_hito(h)), "gobernador"),
                Hito::EnCapital(r) => paso(format!("comprar:{r}"), texto_de_hito(h), "tesorero"),
                Hito::Tecnologia(_) => paso(String::new(), texto_de_hito(h), "mundo"),
                Hito::Capital(..) | Hito::Yacimiento(_) => paso(String::new(), texto_de_hito(h), "gobernador"),
            })
            .collect();
        if faltan.is_empty() {
            if let Some(l) = texto_del_logro(x, estado.as_ref()) {
                pasos.push(paso(String::new(), format!("lo pide el mundo: {l}"), "mundo"));
                if x.logro.iter().any(|(c, ..)| c.starts_with("bandidos.")) {
                    pasos.push(paso("cazar".into(), "destruir campamentos de bandidos".into(), "libre"));
                }
            }
        }
        if !pasos.is_empty() {
            nuevas.push((format!("meta:tecnologia:{}", x.id), meta(format!("La tecnología {}", x.nombre), b::prioridad_tecnologia(&x.id), mayor.clone(), pasos)));
        }
    }
    let mut tarifa_en_capital = por_razon.remove("adoptar una tecnología").unwrap_or_default();
    por_razon.remove("el hito de una tecnología");
    for id in tec.aparecidas.iter().filter(|x| !tec.adoptadas.contains(x)) {
        let x = d.tecnologias.iter().find(|x| x.id == *id);
        let tarifa = x.and_then(|x| d.tarifas.get(&x.era)).map(|v| v.iter().map(|(r, n)| format!("{n} de {r}")).collect::<Vec<_>>().join(" y ")).unwrap_or_default();
        let mut pasos = vec![paso(format!("adoptar:{id}"), format!("Adoptarla en la capital (cuesta {tarifa})"), "rey")];
        pasos.append(&mut tarifa_en_capital);
        nuevas.push((format!("meta:adoptar:{id}"), meta(format!("Adoptar {}", x.map_or(id.as_str(), |x| x.nombre.as_str())), 1, capital.clone(), pasos)));
    }
    // Sus obras (uno libre lleva los materiales; el Tesorero compra lo que falta), explorar alrededor, la tropa siguiente y sus escuadras, y la comida de lujo.
    obras.extend(por_razon.remove("sus obras").unwrap_or_default());
    if !obras.is_empty() {
        nuevas.push((format!("meta:obras:{}", plaza.id), meta(format!("Terminar las obras de {}", plaza.id), 2, plaza.id.clone(), obras)));
    }
    if casilla_por_ver(ctx, &casa, radio_de_batida(ctx, &f), &casa, b::CASILLA_VALE_MS).is_some() {
        let pasos = vec![paso(format!("batida:{}", plaza.id), format!("Batir lo que no se ha visto alrededor de {}", plaza.id), "libre")];
        nuevas.push((format!("meta:explorar:{}", plaza.id), meta(format!("Conocer los alrededores de {}", plaza.id), 3, plaza.id.clone(), pasos)));
    }
    let mut tropa: Vec<PasoDeMeta> = Vec::new();
    let mut que_tropa = String::new();
    if let Some(s) = tropa_siguiente(d, plaza, &tec.adoptadas, t) {
        que_tropa = format!(": {}", s.tropa.id);
        for p in &s.pasos {
            tropa.push(match p {
                Paso::Falta(_) => paso(String::new(), p.texto(), "mundo"),
                _ => paso(format!("tropa:{}", plaza.id), p.texto(), "gobernador"),
            });
        }
        if s.oro > 0 {
            tropa.push(paso(String::new(), format!("{} de oro", s.oro / 1000), "tesorero"));
        }
    }
    tropa.extend(por_razon.remove("equipar la tropa siguiente").unwrap_or_default());
    if por_completar > 0 {
        tropa.push(paso(String::new(), format!("Completar {por_completar} escuadras (una de cada tropa que sabe hacer la plaza)"), "residentes"));
    }
    if !tropa.is_empty() {
        nuevas.push((format!("meta:tropa:{}", plaza.id), meta(format!("Mejor tropa en {}{que_tropa}", plaza.id), 3, plaza.id.clone(), tropa)));
    }
    let resto: Vec<PasoDeMeta> = por_razon.into_values().flatten().collect();
    if !resto.is_empty() {
        nuevas.push((format!("meta:crecer:{}", plaza.id), meta(format!("Comida de lujo para crecer en {}", plaza.id), 3, plaza.id.clone(), resto)));
    }
    ctx.con_pizarra(|p| {
        for (clave, n) in necesidades {
            p.tablero.apuntar_necesidad(&clave, n);
        }
        for (clave, m) in nuevas {
            p.tablero.apuntar_meta(&clave, m);
        }
    });
    Ok(())
}

/// ¿Tiene ya la Facción su estaño? Lo saca de un pueblo suyo o sabe de un yacimiento libre (sin pueblo ni obra de nadie encima). Entonces buscarlo está hecho: el estaño no
/// se mueve, e ir a por él es cosa del libro ([`Via`]).
fn estano_resuelto<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, f: &str) -> bool {
    let mias: Vec<String> = plazas_propias(ctx, f).into_iter().map(|p| p.id).collect();
    let mut tomados: Vec<String> = Vec::new();
    for c in entidades(ctx.puerto.observar()) {
        match c {
            Cuerpo::Pueblo(p) if p.tipo_extractor == "minaEstano" && mias.contains(&p.asentamiento_madre_id) => return true,
            Cuerpo::Pueblo(p) => tomados.push(p.nodo_id.clone()),
            Cuerpo::Obra(o) => tomados.extend(o.nodo_id.clone()),
            _ => {}
        }
    }
    ctx.con_pizarra(|p| p.libro.estano_visto.keys().any(|n| !tomados.contains(n)))
}

/// Lo que puede hacer uno libre al salir de casa: cada una de las salidas, para ordenarlas por lo que pide el tablero.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Accion {
    Materiales,
    GuarnecerAvanzada,
    GuarnecerPueblo,
    Escoltar,
    CazarALaVista,
    BuscarEstano,
    Fundar,
    Cazar,
    Explorar,
    Campana,
    Saquear,
    Asaltar,
}

/// La salida que hace la tarea `clave` del tablero, por su prefijo.
fn accion_de(clave: &str) -> Option<Accion> {
    Some(match clave.split(':').next().unwrap_or_default() {
        "obra" => Accion::Materiales,
        "guarnecer" if clave.starts_with("guarnecer:av") => Accion::GuarnecerAvanzada,
        "guarnecer" => Accion::GuarnecerPueblo,
        "cazar" => Accion::Cazar,
        "batida" | "explorador" => Accion::Explorar,
        "buscador" => Accion::BuscarEstano,
        "fundar" => Accion::Fundar,
        "campana" => Accion::Campana,
        _ => return None,
    })
}

/// Intenta la salida `a`. ¿Ha salido?
async fn hacer<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento, a: Accion) -> Result<bool, Fallo> {
    match a {
        Accion::Materiales => llevar_materiales(ctx, plaza).await,
        Accion::GuarnecerAvanzada => guarnecer_avanzada(ctx, d, plaza).await,
        Accion::GuarnecerPueblo => guarnecer_pueblo(ctx, d, plaza).await,
        Accion::Escoltar => escoltar(ctx, d, plaza).await,
        Accion::CazarALaVista => cazar_a_la_vista(ctx, d, plaza).await,
        Accion::BuscarEstano => buscar_estano(ctx, plaza).await,
        Accion::Fundar => fundar_desde_casa(ctx, d, plaza).await,
        Accion::Cazar => cazar(ctx, d, plaza).await,
        Accion::Explorar => explorar(ctx, plaza).await,
        Accion::Campana => campana(ctx, d, plaza).await,
        Accion::Saquear => saquear(ctx, d, plaza).await,
        Accion::Asaltar => asaltar(ctx, d, plaza).await,
    }
}

/// Por qué no sale quien está en casa sin cargo: las esperas de su plaza y de su Facción, si no tiene tropa libre, y lo que la Facción necesita sin que nadie lo cubra.
fn por_que_no_sale<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> String {
    let t = ctx.ahora_ms;
    let mut motivos: Vec<String> = Vec::new();
    if libres(ctx, d).is_empty() {
        motivos.push("sin tropa libre".into());
    }
    for (clave, que) in [(format!("caza:{}", plaza.id), "caza"), (format!("batida:{}", plaza.id), "batida"), ("buscador".into(), "búsqueda de estaño"), ("fundar".into(), "fundar")] {
        if let Some(h) = ctx.con_pizarra(|p| p.vetos.get(&clave).copied()).filter(|h| *h > t) {
            motivos.push(format!("{que} en espera {:.1} h", (h - t) as f64 / 3_600_000.0));
        }
    }
    let lleno = entidades(ctx.puerto.observar()).any(|c| matches!(c, Cuerpo::Faccion(x) if x.id == plaza.faccion_id && x.cap_fundacion.as_ref().is_some_and(|k| k.plazas + k.caravanas >= k.cap)));
    if lleno {
        motivos.push("tope de plazas lleno".into());
    }
    let yo = ctx.yo.to_string();
    let sin_cubrir = ctx.con_pizarra(|p| p.tablero.para_libres(&plaza.id, &yo, t).len());
    if sin_cubrir > 0 {
        motivos.push(format!("{sin_cubrir} tareas libres del tablero que ahora no puede hacer"));
    }
    if motivos.is_empty() {
        "En casa: nada a su alcance".into()
    } else {
        format!("En casa: {}", motivos.join(" · "))
    }
}

/// Desde su plaza de nivel 2 con lo que cuesta y residentes de sobra, lanza la Caravana de Fundación, sale con su tropa como ejército (una columna personal no lleva
/// caravanas), la engancha y la lleva a un sitio verde del anillo de la plaza. Una por Facción.
async fn fundar_desde_casa<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let t = ctx.ahora_ms;
    let paga = d.costo_caravana_fundacion.iter().all(|(r, n)| plaza.almacen.get(r).is_some_and(|e| cantidad(e, t) >= *n));
    let ya = entidades(ctx.puerto.observar()).any(|c| matches!(c, Cuerpo::Caravana(x) if x.tipo == pb::TipoCaravana::CaravanaDeFundacion as i32 && x.faccion_id.as_deref() == Some(plaza.faccion_id.as_str())));
    let escuadras = libres(ctx, d);
    if plaza.nivel_actual < 2 || plaza.casas_compradas.len() < Personalidad::de(&plaza.faccion_id).residentes_para_expandir() || !paga || ya || escuadras.is_empty() {
        return Ok(false);
    }
    if vetado(ctx, "fundar") || ctx.con_pizarra(|p| p.libro.cautela_hasta > t) {
        return Ok(false);
    }
    let centro = plaza.posicion.clone().unwrap_or_default();
    // Si busca estaño libre, funda a su lado o, si queda lejos, una etapa más cerca: una caravana suelta no cruza medio mapa.
    let hacia_el_estano = ctx.con_pizarra(|p| {
        p.libro.estano.as_ref().filter(|e| e.via == Via::Fundar).and_then(|e| p.libro.estano_visto.get(&e.nodo)).map(|(x, y)| Punto { x: *x, y: *y })
    });
    let sitio = match &hacia_el_estano {
        Some(n) if distancia(&centro, n) <= b::ESTANO_A_MANO => crate::cerebro::sitio_para_fundar(ctx, d, n, ANILLO_ESTANO).await?,
        Some(n) => {
            let f = b::ESTANO_PASO as f64 / distancia(&centro, n) as f64;
            let etapa = Punto { x: centro.x + ((n.x - centro.x) as f64 * f) as i64, y: centro.y + ((n.y - centro.y) as f64 * f) as i64 };
            crate::cerebro::sitio_para_fundar(ctx, d, &etapa, ANILLO_ETAPA).await?
        }
        None => None,
    };
    // Como desde el campamento: sin fuerza para lo que hay en el camino no sale, y el estaño a su alcance espera si el paso no es seguro. La fuerza es la del grupo que
    // saldría con él: los compañeros que están en casa se le unen en la puerta, como en una campaña (si no hay otra saliendo).
    let solo = escuadras.iter().map(|e| d.poder_de(e)).sum::<u32>() + d.poder_por_heroe;
    let ocupada = ctx.con_pizarra(|p| p.tablero.reunida_en(&plaza.id, t).is_some());
    let poder = if ocupada { solo } else { solo * plaza.presentes_ids.len().clamp(1, 4) as u32 };
    let al_alcance = hacia_el_estano.as_ref().is_some_and(|n| distancia(&centro, n) <= b::ESTANO_ALCANCE);
    // La última plaza que le deja fundar su cap es para el estaño, si sabe dónde hay uno libre: el cap solo sube con el nivel, y el nivel 3 pide bronce.
    let ultima = entidades(ctx.puerto.observar())
        .find_map(|c| match c {
            Cuerpo::Faccion(x) if x.id == plaza.faccion_id => x.cap_fundacion.as_ref().map(|k| k.cap.saturating_sub(k.plazas + k.caravanas) <= 1),
            _ => None,
        })
        .unwrap_or(false);
    let sitio = match sitio {
        Some(s) if crate::cerebro::camino_seguro(ctx, poder, &centro, &s) => s,
        Some(_) if al_alcance => return Ok(false),
        _ if ultima && hacia_el_estano.is_some() => return Ok(false),
        _ => {
            let Some(s) = crate::cerebro::sitio_para_fundar(ctx, d, &centro, ANILLO_PLAZA).await? else { return Ok(false) };
            if !crate::cerebro::camino_seguro(ctx, poder, &centro, &s) {
                return Ok(false);
            }
            s
        }
    };
    let en_grupo = !crate::cerebro::camino_seguro(ctx, solo, &centro, &sitio);
    let lanzada = ctx.intentar(&format!("fundar:{}", plaza.id), O::LanzarCaravanaFundacion(pb::OrdenLanzarCaravanaFundacion { origen_asentamiento_id: plaza.id.clone() }), 24 * 3_600_000).await?;
    if lanzada.as_ref().and_then(|r| r.rechazo.as_ref()).is_some_and(|x| x.codigo == "fundacion.cap_alcanzado") {
        vetar(ctx, "fundar", 24 * 3_600_000);
    }
    let Some(caravana) = lanzada.filter(|r| r.resultado == pb::Resultado::OrdenAceptada as i32).and_then(|r| r.creadas.first().map(|c| c.id.clone())) else { return Ok(false) };
    vetar(ctx, "fundar", b::ENTRE_CARAVANAS_DE_FUNDACION_MS);
    let yo = ctx.yo.to_string();
    // Sin objetivo (`objetivo.no_soportado`): el rumbo, después, con `marchar_a`.
    let o = O::MovilizarEjercito(pb::OrdenMovilizarEjercito {
        asentamiento_id: plaza.id.clone(),
        heroe_id: yo.clone(),
        escuadron_ids: escuadras.into_iter().map(|e| e.id).collect(),
        objetivo: None,
        politica_de_union: en_grupo.then(|| "aceptar".into()),
        carga: None,
        // Espera dentro: se la lleva el ejército al salir.
        caravana_fundacion_id: Some(caravana.clone()),
    });
    if ctx.puerto.actuar(o).await?.resultado != pb::Resultado::OrdenAceptada as i32 {
        // Sin ejército con que llevarla, la deshace y recupera lo que costó.
        ctx.puerto.actuar(O::DesarmarCaravanaFundacion(pb::OrdenDesarmarCaravanaFundacion { caravana_id: caravana })).await?;
        return Ok(false);
    }
    repostar(ctx, Some(plaza.id.clone())).await?;
    if en_grupo {
        // Espera en la puerta a los compañeros, que se le unen como a una campaña (`unirse_a_la_campana`).
        if let Some(col) = columna(ctx.puerto.observar(), &yo) {
            reunir(ctx, &format!("fundar:{}", plaza.id), &plaza.id, &col.id, "fundar", ESPERA_CAMPANA_MS);
        }
        ctx.memoria.salida = Some(Salida::FundarEnGrupo { sitio, caravana, inicio: t });
        return Ok(true);
    }
    ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: yo, objetivo: Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(sitio.clone())) }) })).await?;
    ctx.memoria.salida = Some(Salida::Fundar { sitio, intentos: 0 });
    Ok(true)
}

/// Defensa: si el servidor le ofrece unirse a una batalla del lado defensor (`Batalla.lado_para_unirse`, que ya mira la relación y si llega), se une. Así defienden los suyos
/// sus pueblos, avanzadas, caravanas y plazas atacadas. ponytail: sin mirar si la defensa puede ganar; se une siempre que se lo ofrecen.
async fn defender<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>) -> Result<bool, Fallo> {
    let t = ctx.ahora_ms;
    let batalla = entidades(ctx.puerto.observar()).find_map(|c| match c {
        Cuerpo::Batalla(b) if b.lado_para_unirse == Some(pb::LadoId::Defensor as i32) && b.convocatoria_hasta_ms.is_none_or(|h| h > t) => Some(b.id.clone()),
        _ => None,
    });
    let Some(batalla) = batalla else { return Ok(false) };
    let o = O::UnirseABatalla(pb::OrdenUnirseABatalla { heroe_id: ctx.yo.into(), battle_id: batalla.clone(), lado: Some("defensor".into()), escuadron_ids: vec![] });
    Ok(aceptada(&ctx.intentar(&format!("defender:{batalla}"), o, ESPERA_TRAS_RECHAZO_MS).await?))
}

/// Guarnecer avanzadas (§15): con una escuadra que le sobra (fuera de lo que lleva al salir), lleva la más débil a una avanzada activa de su plaza que no tenga ninguna de
/// jugador.
async fn guarnecer_avanzada<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let mut escuadras = sobrantes(ctx, d);
    if escuadras.is_empty() {
        return Ok(false);
    }
    let avanzada = entidades(ctx.puerto.observar()).find_map(|c| match c {
        Cuerpo::Avanzada(a) if a.asentamiento_madre_id == plaza.id && a.estado == pb::EstadoAvanzada::AvanzadaActiva as i32 && a.guarnicion_jugadores_ids.is_empty() => {
            Some((a.id.clone(), a.posicion.clone().unwrap_or_default()))
        }
        _ => None,
    });
    let Some((avanzada, sitio)) = avanzada else { return Ok(false) };
    let clave = format!("guarnecer:{avanzada}");
    let yo = ctx.yo.to_string();
    if !ctx.con_pizarra(|p| p.tablero.libre(&clave, &yo, ctx.ahora_ms)) || vetado(ctx, &clave) {
        return Ok(false);
    }
    escuadras.sort_by_key(|e| (e.cantidad * (e.nivel + 1), e.id.clone()));
    let escuadra = escuadras[0].id.clone();
    let o = O::MovilizarEjercito(pb::OrdenMovilizarEjercito {
        asentamiento_id: plaza.id.clone(),
        heroe_id: yo.clone(),
        escuadron_ids: vec![escuadra.clone()],
        objetivo: None,
        politica_de_union: None,
        carga: None,
        caravana_fundacion_id: None,
    });
    if !aceptada(&ctx.intentar(&clave, o, ESPERA_TRAS_RECHAZO_MS).await?) {
        vetar(ctx, &clave, 6 * 3_600_000);
        return Ok(false);
    }
    ctx.con_pizarra(|p| p.tablero.tomar(&clave, &yo, ctx.ahora_ms, ctx.ahora_ms + b::TAREA_VALE_MS));
    ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: yo, objetivo: Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(sitio)) }) })).await?;
    ctx.memoria.salida = Some(Salida::GuarnecerAvanzada { avanzada, escuadra });
    Ok(true)
}

/// Lleva una escuadra que le sobra (fuera de lo que lleva al salir), la más débil, a un pueblo propio sin guarnición.
async fn guarnecer_pueblo<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let mut escuadras = sobrantes(ctx, d);
    if escuadras.is_empty() {
        return Ok(false);
    }
    let pueblo = entidades(ctx.puerto.observar()).find_map(|c| match c {
        Cuerpo::Pueblo(p) if p.asentamiento_madre_id == plaza.id && p.guarnicion_escuadron_id.is_none() => Some((p.id.clone(), p.posicion.clone()?)),
        _ => None,
    });
    let Some((pueblo, sitio)) = pueblo else { return Ok(false) };
    let clave = format!("guarnecer:{pueblo}");
    let yo = ctx.yo.to_string();
    if !ctx.con_pizarra(|p| p.tablero.libre(&clave, &yo, ctx.ahora_ms)) || vetado(ctx, &clave) {
        return Ok(false);
    }
    escuadras.sort_by_key(|e| (e.cantidad * (e.nivel + 1), e.id.clone()));
    let escuadra = escuadras[0].id.clone();
    let o = O::MovilizarEjercito(pb::OrdenMovilizarEjercito {
        asentamiento_id: plaza.id.clone(),
        heroe_id: yo.clone(),
        escuadron_ids: vec![escuadra.clone()],
        objetivo: None,
        politica_de_union: None,
        carga: None,
        caravana_fundacion_id: None,
    });
    if !aceptada(&ctx.intentar(&clave, o, ESPERA_TRAS_RECHAZO_MS).await?) {
        vetar(ctx, &clave, 6 * 3_600_000);
        return Ok(false);
    }
    ctx.con_pizarra(|p| p.tablero.tomar(&clave, &yo, ctx.ahora_ms, ctx.ahora_ms + b::TAREA_VALE_MS));
    ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: yo, objetivo: Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(sitio)) }) })).await?;
    ctx.memoria.salida = Some(Salida::GuarnecerPueblo { pueblo, escuadra });
    Ok(true)
}

/// Cicatriz de un asalto: cede una escuadra de escolta a una caravana comercial de su plaza, mientras la Facción lo recuerde.
async fn escoltar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let t = ctx.ahora_ms;
    let escuadras = libres(ctx, d);
    if escuadras.len() < 2 || ctx.con_pizarra(|p| p.libro.escoltas_hasta <= t) {
        return Ok(false);
    }
    let caravana = entidades(ctx.puerto.observar()).find_map(|c| match c {
        Cuerpo::Caravana(x) if x.tipo == pb::TipoCaravana::CaravanaComercial as i32 && x.origen_asentamiento_id == plaza.id && !x.escoltada => Some(x.id.clone()),
        _ => None,
    });
    let Some(caravana) = caravana else { return Ok(false) };
    let o = O::AsignarEscolta(pb::OrdenAsignarEscolta { caravana_id: caravana.clone(), heroe_id: ctx.yo.into(), escuadron_ids: vec![escuadras[0].id.clone()] });
    ctx.intentar(&format!("escolta:{caravana}"), o, ESPERA_TRAS_RECHAZO_MS).await?;
    Ok(false)
}

/// Lo que se calcula de una columna ajena sin inspeccionarla: de su tropa solo se ve cuántos héroes la llevan. ponytail: una escuadra típica por héroe, a calibrar.
fn poder_estimado(e: &pb::Ejercito, d: &Datos) -> u32 {
    e.participantes.len().max(1) as u32 * (d.poder_por_heroe + PODER_TIPICO_DE_ESCUADRA)
}

/// Ataque a la vista: la columna de una Facción odiada que ronda su plaza sale a por ella si puede ganarle.
async fn cazar_a_la_vista<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let mia = plaza.faccion_id.clone();
    let (t, centro) = (ctx.ahora_ms, plaza.posicion.clone().unwrap_or_default());
    let escuadras = libres(ctx, d);
    let poder: u32 = escuadras.iter().map(|e| d.poder_de(e)).sum::<u32>() + d.poder_por_heroe;
    if escuadras.is_empty() {
        return Ok(false);
    }
    let presa = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            // Un ejército solo persigue a otro ejército.
            Cuerpo::Ejercito(e) if e.faccion_id != mia && e.tipo == pb::TipoColumna::ColumnaEjercito as i32 && !e.en_batalla && e.dentro_de.is_none() && distancia(&posicion(e, t), &centro) <= plaza.radio_potencial.max(1) => Some(e),
            _ => None,
        })
        .find(|e| politica::a_la_vista(ctx, &e.faccion_id) && poder > poder_estimado(e, d) && !vetado(ctx, &format!("perseguir:{}", e.id)))
        .map(|e| e.id.clone());
    let Some(ejercito) = presa else { return Ok(false) };
    let yo = ctx.yo.to_string();
    let o = O::MovilizarEjercito(pb::OrdenMovilizarEjercito {
        asentamiento_id: plaza.id.clone(),
        heroe_id: yo.clone(),
        escuadron_ids: escuadras.into_iter().map(|e| e.id).collect(),
        objetivo: None,
        politica_de_union: None,
        carga: None,
        caravana_fundacion_id: None,
    });
    if !aceptada(&ctx.intentar(&format!("perseguir:{ejercito}"), o, ESPERA_TRAS_RECHAZO_MS).await?) {
        return Ok(false);
    }
    vetar(ctx, &format!("perseguir:{ejercito}"), 3_600_000);
    let objetivo = Some(pb::orden_perseguir::Objetivo { tipo: Some(pb::orden_perseguir::objetivo::Tipo::EjercitoId(ejercito.clone())) });
    ctx.puerto.actuar(O::Perseguir(pb::OrdenPerseguir { heroe_id: yo, objetivo })).await?;
    ctx.memoria.salida = Some(Salida::Perseguir { ejercito });
    Ok(true)
}

/// Ya fuera: si cruza a un miembro de una Facción odiada, le ataca, sea cual sea su plan. Devuelve si atacó.
async fn atacar_en_camino<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos) -> Result<bool, Fallo> {
    let Some(mia) = ctx.faccion() else { return Ok(false) };
    let Some(col) = columna(ctx.puerto.observar(), ctx.yo) else { return Ok(false) };
    if col.en_batalla || col.escuadron_ids.is_empty() || col.lider_id != ctx.yo {
        return Ok(false);
    }
    let (t, aqui) = (ctx.ahora_ms, posicion(&col, ctx.ahora_ms));
    let poder: u32 = entidades(ctx.puerto.observar())
        .map(|c| match c {
            Cuerpo::Escuadron(e) if col.escuadron_ids.contains(&e.id) => d.poder_de(e),
            _ => 0,
        })
        .sum::<u32>()
        + d.poder_por_heroe * col.participantes.len() as u32;
    let presa = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Ejercito(e) if e.faccion_id != mia && !e.en_batalla && e.dentro_de.is_none() && distancia(&posicion(e, t), &aqui) <= d.radio_encuentro => Some(e),
            _ => None,
        })
        .find(|e| politica::a_la_vista(ctx, &e.faccion_id) && poder > poder_estimado(e, d))
        .map(|e| e.id.clone());
    let Some(ejercito) = presa else { return Ok(false) };
    let objetivo = Some(pb::orden_atacar::Objetivo { tipo: Some(pb::orden_atacar::objetivo::Tipo::EjercitoId(ejercito.clone())) });
    let o = O::Atacar(pb::OrdenAtacar { heroe_id: ctx.yo.into(), objetivo, declara_guerra: Some(true) });
    Ok(aceptada(&ctx.intentar(&format!("atacar:{ejercito}"), o, 3_600_000).await?))
}

/// Asalto a caravanas (§15, guerra logística): el militarista sale con su columna personal (un ejército solo persigue a otro ejército) tras una caravana ajena sin
/// escolta a la vista, de una Facción que no es aliada. Declara la guerra al atacarla.
async fn asaltar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let mia = plaza.faccion_id.clone();
    let centro = plaza.posicion.clone().unwrap_or_default();
    let presa = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Caravana(x) if !x.escoltada && x.faccion_id.as_deref().is_some_and(|f| f != mia) && x.posicion.as_ref().is_some_and(|p| distancia(p, &centro) <= 2 * plaza.radio_potencial.max(1)) => {
                Some((x.id.clone(), x.faccion_id.clone().unwrap_or_default()))
            }
            _ => None,
        })
        .find(|(id, f)| !aliada(ctx, &mia, f) && politica::acosable(ctx, f) && !vetado(ctx, &format!("asaltar:{id}")) && ctx.con_pizarra(|p| p.tablero.libre(&format!("asaltar:{id}"), ctx.yo, ctx.ahora_ms)));
    let escuadras = libres(ctx, d);
    let Some((caravana, _)) = presa else { return Ok(false) };
    if escuadras.is_empty() {
        return Ok(false);
    }
    let yo = ctx.yo.to_string();
    let o = O::SalirAlMundo(pb::OrdenSalirAlMundo { asentamiento_id: plaza.id.clone(), heroe_id: yo.clone(), escuadron_ids: escuadras.into_iter().map(|e| e.id).collect(), carga: None });
    if !aceptada(&ctx.intentar(&format!("asaltar:{caravana}"), o, ESPERA_TRAS_RECHAZO_MS).await?) {
        return Ok(false);
    }
    let t = ctx.ahora_ms;
    ctx.con_pizarra(|p| p.tablero.tomar(&format!("asaltar:{caravana}"), &yo, t, t + b::TAREA_VALE_MS));
    let objetivo = Some(pb::orden_perseguir::Objetivo { tipo: Some(pb::orden_perseguir::objetivo::Tipo::CaravanaId(caravana.clone())) });
    if ctx.puerto.actuar(O::Perseguir(pb::OrdenPerseguir { heroe_id: yo, objetivo })).await?.resultado != pb::Resultado::OrdenAceptada as i32 {
        vetar(ctx, &format!("asaltar:{caravana}"), 24 * 3_600_000);
        return Ok(true); // fuera y sin presa: vuelve a casa
    }
    ctx.memoria.salida = Some(Salida::Asaltar { caravana });
    Ok(true)
}

/// Saqueo (§15, guerra logística): un pueblo ajeno sin guarnición, a la vista, de una Facción que no es aliada, al alcance de sus víveres. Lo que hacen el militarista y,
/// cuando no tiene a qué más salir, el expansionista. Toda agresión declara la guerra: el bot la acepta con estas reglas (decisión del autor).
async fn saquear<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let mia = plaza.faccion_id.clone();
    let centro = plaza.posicion.clone().unwrap_or_default();
    // Los pueblos que recuerda la Facción (la vista solo trae los de su rectángulo): su Facción es la que se recuerda o la de su madre.
    let madres: BTreeMap<String, String> = plazas_ajenas(ctx, &mia).into_iter().map(|(id, f, _)| (id, f)).collect();
    let pueblos = ctx.con_pizarra(|p| p.pueblos.clone());
    let mut objetivo = None;
    for (id, (madre, f, p, defendido)) in &pueblos {
        let dist = distancia(p, &centro);
        // Con la madre conocida manda su dueña de ahora (la recordada puede ser de antes de una conquista); una madre propia no está en `madres`.
        let f = if madre.is_empty() { f.clone() } else { madres.get(madre).cloned().unwrap_or_default() };
        if *defendido || f.is_empty() || f == mia || dist > 2 * plaza.radio_potencial.max(1) {
            continue;
        }
        if !aliada(ctx, &mia, &f) && politica::acosable(ctx, &f) && !vetado(ctx, &format!("saquear:{id}")) && ctx.con_pizarra(|p| p.tablero.libre(&format!("saquear:{id}"), ctx.yo, ctx.ahora_ms)) && objetivo.as_ref().is_none_or(|(d0, _): &(i64, String)| dist < *d0) {
            objetivo = Some((dist, id.clone()));
        }
    }
    let Some((_, pueblo)) = objetivo else { return Ok(false) };
    let escuadras = libres(ctx, d);
    if escuadras.is_empty() {
        return Ok(false);
    }
    let yo = ctx.yo.to_string();
    let o = O::MovilizarEjercito(pb::OrdenMovilizarEjercito {
        asentamiento_id: plaza.id.clone(),
        heroe_id: yo.clone(),
        escuadron_ids: escuadras.into_iter().map(|e| e.id).collect(),
        objetivo: None,
        politica_de_union: None,
        carga: None,
        caravana_fundacion_id: None,
    });
    if !aceptada(&ctx.intentar(&format!("saquear:{pueblo}"), o, ESPERA_TRAS_RECHAZO_MS).await?) {
        return Ok(false);
    }
    let t = ctx.ahora_ms;
    ctx.con_pizarra(|p| p.tablero.tomar(&format!("saquear:{pueblo}"), &yo, t, t + b::TAREA_VALE_MS));
    repostar(ctx, Some(plaza.id.clone())).await?;
    let sitio = pueblos.get(&pueblo).map(|(_, _, p, _)| p.clone());
    let Some(sitio) = sitio.filter(|s| le_llega(ctx, d, s)) else {
        vetar(ctx, &format!("saquear:{pueblo}"), 24 * 3_600_000);
        ctx.puerto.actuar(O::Guarnecer(pb::OrdenGuarnecer { asentamiento_id: plaza.id.clone(), heroe_id: yo })).await?;
        return Ok(true);
    };
    ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: yo, objetivo: Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(sitio)) }) })).await?;
    ctx.memoria.salida = Some(Salida::Saquear { pueblo });
    Ok(true)
}

/// Caza el campamento de bandidos de su plaza o el más débil de su alrededor, si lo ve, nadie va ya y puede con él: solo o, si no, con los compañeros que están en casa, que se le
/// unen en la puerta como en una campaña.
async fn cazar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let (t, yo) = (ctx.ahora_ms, ctx.yo.to_string());
    if vetado(ctx, &format!("caza:{}", plaza.id)) {
        return Ok(false);
    }
    let casa = plaza.posicion.clone().unwrap_or_default();
    let cerca: Vec<_> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::CampamentoBandidos(b) if b.asentamiento_id.as_deref() == Some(plaza.id.as_str()) || b.posicion.as_ref().is_some_and(|p| distancia(p, &casa) <= b::RADIO_DE_CAZA) => Some(b.clone()),
            _ => None,
        })
        .collect();
    let bandidos = cerca.into_iter().filter(|b| !vetado(ctx, &format!("bandido:{}", b.id)) && ctx.con_pizarra(|p| p.tablero.libre(&format!("cazar:{}", b.id), &yo, t))).min_by_key(|b| (b.poder, b.id.clone()));
    let Some(b) = bandidos else { return Ok(false) };
    let clave = format!("bandido:{}", b.id);
    let escuadras = libres(ctx, d);
    let poder: u32 = escuadras.iter().map(|e| d.poder_de(e)).sum();
    let solo = poder > b.poder;
    // Como en una campaña: supone que los compañeros traen lo mismo que él.
    let grupo = plaza.presentes_ids.len().clamp(1, 4) as u32;
    let en_grupo = ((poder + d.poder_por_heroe) * grupo) as f64 > b.poder as f64 * b::MARGEN_PARA_FUNDAR;
    let ocupada = ctx.con_pizarra(|p| p.tablero.reunida_en(&plaza.id, t).is_some());
    if escuadras.is_empty() || !(solo || en_grupo && !ocupada) {
        return Ok(false);
    }
    let o = O::MovilizarEjercito(pb::OrdenMovilizarEjercito {
        asentamiento_id: plaza.id.clone(),
        heroe_id: yo.clone(),
        escuadron_ids: escuadras.into_iter().map(|e| e.id).collect(),
        objetivo: None,
        politica_de_union: (!solo).then(|| "aceptar".into()),
        carga: None,
        caravana_fundacion_id: None,
    });
    // Rechazada, no sale: que el resto de `salir` (explorar, campaña, repartirse) siga su turno.
    if !aceptada(&ctx.intentar(&format!("cazar:{}", b.id), o, ESPERA_TRAS_RECHAZO_MS).await?) {
        return Ok(false);
    }
    vetar(ctx, &format!("caza:{}", plaza.id), b::ENTRE_CAZAS_MS);
    repostar(ctx, Some(plaza.id.clone())).await?;
    // Sin víveres para ir y volver, no sale: vuelve a entrar y el bandido queda vetado un día para la Facción.
    if !b.posicion.as_ref().is_none_or(|p| le_llega(ctx, d, p)) {
        vetar(ctx, &clave, 24 * 3_600_000);
        ctx.puerto.actuar(O::Guarnecer(pb::OrdenGuarnecer { asentamiento_id: plaza.id.clone(), heroe_id: yo })).await?;
        return Ok(true);
    }
    let tarea = format!("cazar:{}", b.id);
    ctx.con_pizarra(|p| p.tablero.tomar(&tarea, &yo, t, t + b::TAREA_VALE_MS));
    if !solo {
        // Abre la reunión en la puerta: despierta a los que viven en la plaza para que se le unan.
        if let Some(col) = columna(ctx.puerto.observar(), &yo) {
            reunir(ctx, &tarea, &plaza.id, &col.id, &b.id, ESPERA_CAMPANA_MS);
        }
        ctx.memoria.salida = Some(Salida::CazarEnGrupo { bandidos: b.id, inicio: t });
        return Ok(true);
    }
    let destino = b.posicion.clone().map(|p| orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(p)) });
    ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: yo.clone(), objetivo: destino })).await?;
    ctx.memoria.salida = Some(Salida::Cazar { bandidos: b.id });
    Ok(true)
}

/// Exploradores, sin tropa, si la Facción tiene al menos tres héroes. Uno por Facción inspecciona la plaza ajena más cercana cuya defensa no está en la pizarra o está
/// vieja. Sin ninguna que inspeccionar, cada plaza bate su región por la rejilla de la Facción ([`batir`]): una batida a la vez por plaza, como mucho cada `entre_batidas`,
/// y solo si queda algo por ver.
async fn explorar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let (yo, t) = (ctx.yo.to_string(), ctx.ahora_ms);
    let f = plaza.faccion_id.clone();
    let ciudadanos = entidades(ctx.puerto.observar()).find_map(|c| match c {
        Cuerpo::Faccion(x) if x.id == f => Some(x.ciudadanos_ids.len()),
        _ => None,
    });
    if ciudadanos.unwrap_or(0) < 3 {
        return Ok(false);
    }
    let casa = plaza.posicion.clone().unwrap_or_default();
    // La tarea de explorador caduca: si quien lo tenía perdió el plan (un reinicio, una batida sin camino), otro lo toma al día siguiente.
    let libre = ctx.con_pizarra(|p| p.tablero.libre("explorador", &yo, t)) || !vetado(ctx, "explorador");
    let objetivo = if libre { siguiente_a_explorar(ctx, &f, &casa) } else { None };
    let batida = format!("batida:{}", plaza.id);
    let toca_batida = objetivo.is_none()
        && plaza.presentes_ids.len() >= 2
        && !vetado(ctx, &batida)
        && ctx.con_pizarra(|p| p.tablero.libre(&batida, &yo, t))
        && casilla_por_ver(ctx, &casa, radio_de_batida(ctx, &f), &casa, b::CASILLA_VALE_MS).is_some();
    if objetivo.is_none() && !toca_batida {
        return Ok(false);
    }
    // Toma la tarea antes de salir: los compañeros piensan a la vez y, con la tarea tomada después, salían varios a lo mismo.
    let clave = if objetivo.is_some() { "explorador".to_string() } else { batida.clone() };
    if !ctx.con_pizarra(|p| p.tablero.tomar(&clave, &yo, t, t + b::TAREA_VALE_MS)) {
        return Ok(false);
    }
    let salida = O::SalirAlMundo(pb::OrdenSalirAlMundo { asentamiento_id: plaza.id.clone(), heroe_id: yo.clone(), escuadron_ids: vec![], carga: None });
    if !aceptada(&ctx.intentar("salir:explorar", salida, ESPERA_TRAS_RECHAZO_MS).await?) {
        ctx.con_pizarra(|p| p.tablero.soltar(&clave, &yo));
        return Ok(false);
    }
    match objetivo {
        Some(objetivo) => {
            vetar(ctx, "explorador", 24 * 3_600_000);
            if marchar_a_plaza(ctx, &objetivo).await? {
                ctx.memoria.salida = Some(Salida::Explorar { plaza: objetivo });
            }
        }
        None => {
            vetar(ctx, &batida, Personalidad::de(&f).entre_batidas());
            batir(ctx, &casa, 0).await?;
        }
    }
    Ok(true)
}

/// Hasta dónde bate una Facción: su región mientras es joven; estable, la batida lejana, a buscar plazas ajenas.
fn radio_de_batida<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, mia: &str) -> i64 {
    if estable(ctx, mia) {
        BATIDA.1
    } else {
        b::REGION
    }
}

/// La casilla por ver de la rejilla de su Facción, a no más de `radio` de `centro`: no vista desde hace `vale` ([`b::CASILLA_VALE_MS`] para lo que se mueve, como los
/// bandidos; `i64::MAX` para lo que no, como el estaño) y lejos ([`b::SEPARACION_EXPLORADORES`]) de las que ya llevan otros exploradores. Así cada uno abre su propio
/// frente en vez de ir al mismo sitio.
fn casilla_por_ver<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, centro: &Punto, radio: i64, desde: &Punto, vale: i64) -> Option<((i64, i64), Punto)> {
    use crate::cerebro::{casilla, centro_de_casilla};
    let (yo, t) = (ctx.yo, ctx.ahora_ms);
    let (cx, cy) = casilla(centro);
    let n = radio / b::CASILLA + 1;
    ctx.con_pizarra(|p| {
        let otras: Vec<Punto> = p
            .tablero
            .de_otros("casilla:", yo, t)
            .iter()
            .filter_map(|k| {
                let (x, y) = k.strip_prefix("casilla:")?.split_once(',')?;
                Some(centro_de_casilla((x.parse().ok()?, y.parse().ok()?)))
            })
            .collect();
        (-n..=n)
            .flat_map(|i| (-n..=n).map(move |j| (cx + i, cy + j)))
            .map(|c| (c, centro_de_casilla(c)))
            .filter(|(c, q)| {
                distancia(q, centro) <= radio && p.rejilla.get(c).is_none_or(|&v| t - v >= vale) && otras.iter().all(|o| distancia(o, q) >= b::SEPARACION_EXPLORADORES)
            })
            // Primero lo que nunca se vio, por cercanía: así avanza el borde de lo conocido. Visto todo, lo que lleva más días sin verse; si no, volvería cada día al
            // primer anillo, el primero en caducar.
            .min_by_key(|(c, q)| (p.rejilla.get(c).map_or(i64::MIN, |&v| v / b::CASILLA_VALE_MS), distancia(q, desde), *c))
    })
}

fn plazas_propias<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, mia: &str) -> Vec<pb::Asentamiento> {
    entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Asentamiento(a) if a.faccion_id == mia => Some(a.clone()),
            _ => None,
        })
        .collect()
}

/// ¿Ha llegado su Facción a un punto estable? Hasta entonces no sale de su región.
fn estable<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, mia: &str) -> bool {
    let a = ctx.puerto.observar();
    plazas_propias(ctx, mia).iter().any(|p| p.nivel_actual >= b::ESTABLE_NIVEL && entidades(a).filter(|c| matches!(c, Cuerpo::Pueblo(x) if x.asentamiento_madre_id == p.id)).count() >= b::ESTABLE_PUEBLOS)
}

/// ¿Puede ir a `sitio`? Estable, a cualquier parte; si no, solo cerca de alguna de sus plazas.
fn en_su_region<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, mia: &str, sitio: &Punto) -> bool {
    estable(ctx, mia) || plazas_propias(ctx, mia).iter().any(|p| p.posicion.as_ref().is_some_and(|q| distancia(q, sitio) <= b::REGION))
}

fn estano_a_su_alcance<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, casa: &Punto) -> bool {
    ctx.con_pizarra(|p| p.libro.estano_visto.values().any(|&(x, y)| distancia(&Punto { x, y }, casa) <= b::ESTANO_ALCANCE))
}

/// Un buscador de estaño por Facción, desde su primera plaza: sale sin tropa mientras no sepa de estaño a su alcance. Lo que ve queda en el libro de su Facción.
async fn buscar_estano<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let f = plaza.faccion_id.clone();
    let ciudadanos = entidades(ctx.puerto.observar()).find_map(|c| match c {
        Cuerpo::Faccion(x) if x.id == f => Some(x.ciudadanos_ids.len()),
        _ => None,
    });
    let casa = plaza.posicion.clone().unwrap_or_default();
    if ciudadanos.unwrap_or(0) < 2 || vetado(ctx, "buscador") || estano_a_su_alcance(ctx, &casa) || estano_resuelto(ctx, &f) || anillo_del_estano(ctx, &casa, &casa).is_none() {
        return Ok(false);
    }
    // Toma la tarea antes de salir: los compañeros piensan a la vez.
    let (yo, t) = (ctx.yo.to_string(), ctx.ahora_ms);
    if !ctx.con_pizarra(|p| p.tablero.tomar("buscador", &yo, t, t + b::TAREA_VALE_MS)) {
        return Ok(false);
    }
    let salida = O::SalirAlMundo(pb::OrdenSalirAlMundo { asentamiento_id: plaza.id.clone(), heroe_id: yo.clone(), escuadron_ids: vec![], carga: None });
    if !aceptada(&ctx.intentar("salir:buscador", salida, ESPERA_TRAS_RECHAZO_MS).await?) {
        ctx.con_pizarra(|p| p.tablero.soltar("buscador", &yo));
        return Ok(false);
    }
    vetar(ctx, "buscador", b::ENTRE_BUSQUEDAS_MS);
    tramo_de_busqueda(ctx, &casa, &casa, 0).await?;
    Ok(true)
}

/// El siguiente tramo del buscador de estaño: la casilla de la rejilla de su Facción que nadie vio nunca ([`ir_a_casilla`]) en el anillo de búsqueda de su plaza
/// ([`anillo_del_estano`]), hasta [`b::TRAMOS_DE_BUSQUEDA`] tramos. El estaño que pasa a su vista queda en el libro de su Facción.
async fn tramo_de_busqueda<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, casa: &Punto, desde: &Punto, paso: u32) -> Result<(), Fallo> {
    let Some(radio) = anillo_del_estano(ctx, casa, desde) else { return Ok(()) };
    if paso < b::TRAMOS_DE_BUSQUEDA && ir_a_casilla(ctx, casa, radio, desde, i64::MAX).await? {
        ctx.memoria.salida = Some(Salida::BuscarEstano { paso: paso + 1 });
    }
    Ok(())
}

/// Hasta dónde busca estaño: el primer anillo ([`b::ESTANO_ANILLO`], el doble, el triple...) alrededor de `casa` en el que queda una casilla que nadie de su Facción vio
/// nunca. El estaño no se mueve: lo ya visto no se vuelve a buscar. Una Facción joven, solo en su región. Sin nada por ver, no busca.
fn anillo_del_estano<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, casa: &Punto, desde: &Punto) -> Option<i64> {
    let mia = ctx.faccion().unwrap_or_default();
    let radios: Vec<i64> = if estable(ctx, &mia) { (1..=b::ESTANO_ANILLOS).map(|k| k * b::ESTANO_ANILLO).collect() } else { vec![b::REGION] };
    radios.into_iter().find(|&r| casilla_por_ver(ctx, casa, r, desde, i64::MAX).is_some())
}

/// El siguiente tramo de la batida: la casilla por ver ([`ir_a_casilla`]) alrededor de su plaza más cercana.
async fn batir<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, desde: &Punto, paso: u32) -> Result<(), Fallo> {
    let mia = ctx.faccion().unwrap_or_default();
    let centro = plazas_propias(ctx, &mia).into_iter().filter_map(|p| p.posicion).min_by_key(|p| distancia(p, desde)).unwrap_or_else(|| desde.clone());
    let radio = radio_de_batida(ctx, &mia);
    if ir_a_casilla(ctx, &centro, radio, desde, b::CASILLA_VALE_MS).await? {
        ctx.memoria.salida = Some(Salida::Buscar { paso: paso + 1 });
    }
    Ok(())
}

/// Marcha a la casilla por ver ([`casilla_por_ver`]) a no más de `radio` de `centro`, y la aparta para que otro explorador de la Facción no vaya a lo mismo (suelta la
/// que llevaba). A una casilla a la que no se llega (agua, sin camino) la da por vista y prueba otra. ¿Ha salido?
async fn ir_a_casilla<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, centro: &Punto, radio: i64, desde: &Punto, vale: i64) -> Result<bool, Fallo> {
    let (yo, t) = (ctx.yo.to_string(), ctx.ahora_ms);
    ctx.con_pizarra(|p| p.tablero.soltar_de("casilla:", &yo));
    for _ in 0..8 {
        let Some((c, p)) = casilla_por_ver(ctx, centro, radio, desde, vale) else { return Ok(false) };
        let destino = Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(p)) });
        if ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: yo.clone(), objetivo: destino })).await?.resultado == pb::Resultado::OrdenAceptada as i32 {
            ctx.con_pizarra(|pz| pz.tablero.tomar(&format!("casilla:{},{}", c.0, c.1), &yo, t, t + b::CASILLA_APARTADA_MS));
            return Ok(true);
        }
        ctx.con_pizarra(|pz| pz.rejilla.insert(c, t));
    }
    Ok(false)
}

fn siguiente_a_explorar<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, mia: &str, desde: &Punto) -> Option<String> {
    let t = ctx.ahora_ms;
    let mut ajenas = plazas_ajenas(ctx, mia);
    ajenas.retain(|(id, _, s)| en_su_region(ctx, mia, s) && ctx.con_pizarra(|p| p.defensas.get(id).is_none_or(|(_, _, visto)| t - visto > VIGENCIA_INSPECCION_MS)));
    ajenas.into_iter().min_by_key(|(id, f, p)| (!politica::es_objetivo(ctx, id, f), distancia(p, desde), id.clone())).map(|(id, _, _)| id)
}

/// Campaña contra una plaza rival inspeccionada hace poco que puede ganar con margen (§16): ni aliada, ni la última de su Facción, ni protegida. Sale como líder con lo que
/// tiene libre; los demás que estén en casa se le unen en la puerta. Una campaña por plaza, y la casa nunca se vacía.
/// Toda agresión declara la guerra (Diplomacia): el bot la acepta cuando se cumplen estas reglas (decisión pendiente del autor, recomendada así).
async fn campana<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let t = ctx.ahora_ms;
    let ya = ctx.con_pizarra(|p| p.tablero.reunida_en(&plaza.id, t).is_some());
    let personalidad = Personalidad::de(&plaza.faccion_id);
    if std::env::var("BOTS_DEPURA").is_ok() {
        let todas = entidades(ctx.puerto.observar()).filter(|c| matches!(c, Cuerpo::Escuadron(e) if e.heroe_id == ctx.yo && e.cantidad > 0)).count();
        eprintln!("DEPURA entra {} {:?} nivel {} casas {} ya {ya} libres {} mias {todas} ajenas {}", plaza.id, personalidad, plaza.nivel_actual, plaza.casas_compradas.len(), libres(ctx, d).len(), plazas_ajenas(ctx, &plaza.faccion_id).len());
    }
    if plaza.nivel_actual < personalidad.nivel_para_campana() || plaza.casas_compradas.len() < 2 || ya || ctx.con_pizarra(|p| p.libro.cautela_hasta > t) {
        return Ok(false);
    }
    let escuadras = libres(ctx, d);
    let poder: u32 = escuadras.iter().map(|e| d.poder_de(e)).sum();
    if escuadras.is_empty() {
        return Ok(false);
    }
    // El grupo que saldrá: él y los compañeros que están en casa (se le unen en la puerta), hasta 4; cada héroe suma su `poderPorHeroe`.
    // ponytail: supone que los compañeros traen lo mismo que él; antes se multiplicaba la defensa por los héroes de dentro y nunca salía ninguna campaña.
    let grupo = plaza.presentes_ids.len().clamp(1, 4) as u32;
    let poder_grupo = (poder + d.poder_por_heroe) * grupo;
    let mia = plaza.faccion_id.clone();
    let casa = plaza.posicion.clone().unwrap_or_default();
    let ajenas = plazas_ajenas(ctx, &mia);
    let plazas_de = |f: &str| ajenas.iter().filter(|(_, x, _)| x == f).count();
    let protegida = |id: &str| entidades(ctx.puerto.observar()).any(|c| matches!(c, Cuerpo::Asentamiento(a) if a.id == id && a.protegida_hasta_ms.is_some_and(|h| h > t)));
    if std::env::var("BOTS_DEPURA").is_ok() {
        for (id, f, _) in &ajenas {
            if let Some((defensa, heroes, visto)) = ctx.con_pizarra(|p| p.defensas.get(id).copied()) {
                eprintln!(
                    "DEPURA campana {} {:?} poder_grupo {poder_grupo} defensa {defensa} heroes {heroes} edad_min {} plazas_de {} aliada {} protegida {} vetado {}",
                    plaza.id, personalidad, (t - visto) / 60_000, plazas_de(f), aliada(ctx, &mia, f), protegida(id), vetado(ctx, &format!("atacar:{id}"))
                );
            }
        }
    }
    let objetivo = ajenas
        .iter()
        // Ni una a la que ya va otro grupo de la Facción (desde esta plaza o desde otra): dos campañas por separado contra la misma plaza son dos ejércitos a medias.
        .filter(|(id, f, s)| {
            plazas_de(f) > 1
                && en_su_region(ctx, &mia, s)
                && !aliada(ctx, &mia, f)
                && !protegida(id)
                && !vetado(ctx, &format!("atacar:{id}"))
                && politica::acosable(ctx, f)
                && ctx.con_pizarra(|p| p.tablero.libre(&format!("campana:{id}"), ctx.yo, t))
        })
        .filter(|(id, f, _)| {
            let margen = margen_contra(ctx, &mia, f);
            ctx.con_pizarra(|p| p.defensas.get(id).is_some_and(|&(defensa, heroes, visto)| t - visto <= VIGENCIA_INSPECCION_MS && poder_grupo as f64 > (defensa + heroes as u32 * d.poder_por_heroe) as f64 * margen))
        })
        .min_by_key(|(id, f, p)| (!politica::es_objetivo(ctx, id, f), distancia(p, &casa), id.clone()))
        .map(|(id, _, _)| id.clone());
    let Some(objetivo) = objetivo else { return Ok(false) };
    let yo = ctx.yo.to_string();
    // Sale sin rumbo y espera en la puerta a los compañeros; luego marcha (`en_salida`).
    let o = O::MovilizarEjercito(pb::OrdenMovilizarEjercito {
        asentamiento_id: plaza.id.clone(),
        heroe_id: yo.clone(),
        escuadron_ids: escuadras.into_iter().map(|e| e.id).collect(),
        objetivo: None,
        politica_de_union: Some("aceptar".into()),
        carga: None,
        caravana_fundacion_id: None,
    });
    if !aceptada(&ctx.intentar(&format!("campana:{objetivo}"), o, ESPERA_TRAS_RECHAZO_MS).await?) {
        return Ok(false);
    }
    if let Some(col) = columna(ctx.puerto.observar(), &yo) {
        reunir(ctx, &format!("campana:{objetivo}"), &plaza.id, &col.id, &objetivo, ESPERA_CAMPANA_MS);
    }
    ctx.memoria.salida = Some(Salida::Campana { plaza: objetivo, lidera: true, marcha: false, inicio: t });
    Ok(true)
}

/// El margen sobre la defensa con que sale de campaña contra una plaza de `f`: el de su personalidad, menos si se atreve con ella (quien resistió un asedio se atreve con
/// quien lo sitió, y un Rey orgulloso con quien le provoca).
fn margen_contra<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, mia: &str, f: &str) -> f64 {
    let t = ctx.ahora_ms;
    let confianza = ctx.con_pizarra(|p| {
        let orgulloso = p.rey.as_deref().is_some_and(|r| rasgos(r).contains(&Rasgo::Orgulloso));
        if p.libro.paginas.get(f).is_some_and(|x| x.confianza_hasta > t) || orgulloso {
            b::FACTOR_CONFIANZA
        } else {
            1.0
        }
    });
    Personalidad::de(mia).margen() * confianza
}

/// Si un compañero acaba de salir de campaña desde casa, se le une con lo que tiene libre, sin vaciar la casa (se queda al menos uno).
async fn unirse_a_la_campana<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<bool, Fallo> {
    let (t, yo) = (ctx.ahora_ms, ctx.yo.to_string());
    let Some((ejercito, objetivo)) = ctx.con_pizarra(|p| p.tablero.a_unirse(&plaza.id, &yo, t)) else {
        return Ok(false);
    };
    let escuadras = libres(ctx, d);
    if escuadras.is_empty() || plaza.presentes_ids.len() <= 1 {
        return Ok(false);
    }
    let o = O::UnirseAEjercito(pb::OrdenUnirseAEjercito { ejercito_id: ejercito.clone(), asentamiento_id: plaza.id.clone(), heroe_id: yo, escuadron_ids: escuadras.into_iter().map(|e| e.id).collect() });
    if aceptada(&ctx.intentar(&format!("unirse:{ejercito}"), o, ESPERA_TRAS_RECHAZO_MS).await?) {
        crate::cerebro::evento(ctx, "se_une", format!("se une a {ejercito} en {} para {objetivo}", plaza.id), serde_json::json!({"columna": ejercito, "puerta": plaza.id, "objetivo": objetivo}));
        ctx.memoria.salida = Some(Salida::Campana { plaza: objetivo, lidera: false, marcha: true, inicio: t });
        return Ok(true);
    }
    Ok(false)
}

/// Reparto de la defensa (§16): si su plaza tiene 2 residentes o más que la de su Facción que menos tiene, se muda a ella.
async fn repartirse<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    // Quién vive dónde lo dice la pizarra: la plaza ajena a su interior llega sin sus residentes.
    let plazas: Vec<String> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Asentamiento(p) if p.faccion_id == plaza.faccion_id => Some(p.id.clone()),
            _ => None,
        })
        .collect();
    let cuenta = ctx.con_pizarra(|p| plazas.iter().map(|id| (p.residencias.values().filter(|r| *r == id).count(), id.clone())).collect::<Vec<_>>());
    let aqui = cuenta.iter().find(|(_, id)| *id == plaza.id).map_or(0, |(n, _)| *n);
    let menos = cuenta.into_iter().filter(|(_, id)| *id != plaza.id).min();
    let Some((_, destino)) = menos.filter(|(n, _)| aqui >= n + 2) else { return rotar_guarnicion(ctx, d, plaza).await };
    let yo = ctx.yo.to_string();
    let salida = O::SalirAlMundo(pb::OrdenSalirAlMundo { asentamiento_id: plaza.id.clone(), heroe_id: yo.clone(), escuadron_ids: vec![], carga: None });
    if !aceptada(&ctx.intentar(&format!("mudarse:{destino}"), salida, ESPERA_TRAS_RECHAZO_MS).await?) {
        return Ok(());
    }
    // Ya vive allí: `volver_a_casa` le lleva y le hace entrar.
    ctx.puerto.actuar(O::CambiarResidencia(pb::OrdenCambiarResidencia { destino_id: destino, heroe_id: yo })).await?;
    Ok(())
}

/// Rotación de guarnición (§15): con la guarnición de su plaza llena y dos escuadras libres o más, se muda con ellas a la plaza de su Facción que no tiene guarnición, y
/// al llegar entra como ejército (`guarnecer`): así refuerza la plaza que lo necesita sin esperar a una conquista.
async fn rotar_guarnicion<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let a = ctx.puerto.observar();
    let dentro: u32 = entidades(a)
        .filter_map(|c| match c {
            Cuerpo::Escuadron(e) if plaza.guarnicion_ids.contains(&e.id) => Some(e.cantidad),
            _ => None,
        })
        .sum();
    let escuadras = libres(ctx, d);
    if escuadras.len() < 2 || dentro < plaza.cupo_guarnicion {
        return Ok(());
    }
    let destino = entidades(ctx.puerto.observar()).find_map(|c| match c {
        Cuerpo::Asentamiento(p) if p.faccion_id == plaza.faccion_id && p.id != plaza.id && p.guarnicion_ids.is_empty() => Some(p.id.clone()),
        _ => None,
    });
    let Some(destino) = destino else { return Ok(()) };
    let clave = format!("rotar:{destino}");
    if vetado(ctx, &clave) {
        return Ok(());
    }
    vetar(ctx, &clave, 24 * 3_600_000); // uno por día y plaza
    let yo = ctx.yo.to_string();
    let o = O::MovilizarEjercito(pb::OrdenMovilizarEjercito {
        asentamiento_id: plaza.id.clone(),
        heroe_id: yo.clone(),
        escuadron_ids: escuadras.into_iter().map(|e| e.id).collect(),
        objetivo: None,
        politica_de_union: None,
        carga: None,
        caravana_fundacion_id: None,
    });
    if aceptada(&ctx.intentar(&clave, o, ESPERA_TRAS_RECHAZO_MS).await?) {
        repostar(ctx, Some(plaza.id.clone())).await?;
        // Ya vive allí: `volver_a_casa` le lleva y, en la puerta, entra con `guarnecer`.
        ctx.puerto.actuar(O::CambiarResidencia(pb::OrdenCambiarResidencia { destino_id: destino, heroe_id: yo })).await?;
    }
    Ok(())
}

/// Fuera, con un plan. Devuelve si sigue con él (si no, vuelve a casa). Mientras sigue, renueva su tarea en el tablero de la Facción; al acabar, la suelta.
async fn en_salida<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, casa: &str) -> Result<bool, Fallo> {
    let (yo, t) = (ctx.yo.to_string(), ctx.ahora_ms);
    let tarea = ctx.memoria.salida.as_ref().and_then(|s| s.tarea(casa));
    if let Some(x) = &tarea {
        ctx.con_pizarra(|p| p.tablero.tomar(x, &yo, t, t + b::TAREA_VALE_MS));
    }
    let sigue = seguir_salida(ctx, d, casa).await?;
    if !sigue {
        ctx.con_pizarra(|p| {
            if let Some(x) = &tarea {
                p.tablero.soltar(x, &yo);
            }
            p.tablero.soltar_de("casilla:", &yo);
        });
    }
    Ok(sigue)
}

/// Lo que hace fuera según su plan. Devuelve si sigue con él.
async fn seguir_salida<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, casa: &str) -> Result<bool, Fallo> {
    let Some(salida) = ctx.memoria.salida.take() else { return Ok(false) };
    let Some(col) = columna(ctx.puerto.observar(), ctx.yo) else { return Ok(false) };
    let parada = col.estado == EstadoColumna::ColumnaEstacionada as i32;
    let aqui = posicion(&col, ctx.ahora_ms);
    let yo = ctx.yo.to_string();
    if atacar_en_camino(ctx, d).await? {
        ctx.memoria.salida = Some(salida);
        return Ok(true);
    }
    match salida {
        Salida::GuarnecerPueblo { pueblo, escuadra } => {
            if !parada {
                ctx.memoria.salida = Some(Salida::GuarnecerPueblo { pueblo, escuadra });
                return Ok(true);
            }
            ctx.puerto.actuar(O::EntrarEnPueblo(pb::OrdenEntrarEnPueblo { pueblo_id: pueblo.clone(), heroe_id: yo.clone() })).await?;
            ctx.puerto.actuar(O::GuarnecerPueblo(pb::OrdenGuarnecerPueblo { pueblo_id: pueblo.clone(), escuadron_ids: vec![escuadra] })).await?;
            ctx.puerto.actuar(O::SalirDePueblo(pb::OrdenSalirDePueblo { pueblo_id: pueblo.clone(), heroe_id: yo.clone() })).await?;
            Ok(false)
        }
        Salida::Perseguir { ejercito } => {
            let donde = entidades(ctx.puerto.observar()).find_map(|c| match c {
                Cuerpo::Ejercito(e) if e.id == ejercito => Some(posicion(e, ctx.ahora_ms)),
                _ => None,
            });
            match donde {
                Some(p) if distancia(&p, &aqui) <= d.radio_encuentro => {
                    let objetivo = Some(pb::orden_atacar::Objetivo { tipo: Some(pb::orden_atacar::objetivo::Tipo::EjercitoId(ejercito)) });
                    ctx.puerto.actuar(O::Atacar(pb::OrdenAtacar { heroe_id: yo, objetivo, declara_guerra: Some(true) })).await?;
                    Ok(false)
                }
                Some(_) if !parada => {
                    ctx.memoria.salida = Some(Salida::Perseguir { ejercito });
                    Ok(true)
                }
                _ => Ok(false),
            }
        }
        Salida::Cazar { bandidos } => {
            let sitio = entidades(ctx.puerto.observar()).find_map(|c| match c {
                Cuerpo::CampamentoBandidos(b) if b.id == bandidos => b.posicion.clone(),
                _ => None,
            });
            match sitio {
                Some(_) if !parada => {
                    ctx.memoria.salida = Some(Salida::Cazar { bandidos });
                    return Ok(true);
                }
                Some(s) if distancia(&s, &aqui) <= d.radio_encuentro => {
                    let objetivo = Some(pb::orden_atacar::Objetivo { tipo: Some(pb::orden_atacar::objetivo::Tipo::CampamentoId(bandidos.clone())) });
                    ctx.puerto.actuar(O::Atacar(pb::OrdenAtacar { heroe_id: yo.clone(), objetivo, declara_guerra: None })).await?;
                }
                // Ya no está: que la Facción lo olvide, o seguiría juzgando sus caminos por él.
                None => {
                    ctx.con_pizarra(|p| p.bandidos.remove(&bandidos));
                }
                _ => {}
            }
            Ok(false)
        }
        Salida::BuscarEstano { paso } => {
            if !parada {
                ctx.memoria.salida = Some(Salida::BuscarEstano { paso });
                return Ok(true);
            }
            let mia = ctx.faccion().unwrap_or_default();
            let Some(hogar) = plaza_de(ctx, casa).filter(|p| !estano_a_su_alcance(ctx, p) && !estano_resuelto(ctx, &mia)) else { return Ok(false) };
            vetar(ctx, "buscador", b::ENTRE_BUSQUEDAS_MS);
            tramo_de_busqueda(ctx, &hogar, &aqui, paso).await?;
            Ok(ctx.memoria.salida.is_some())
        }
        Salida::CazarEnGrupo { bandidos, inicio } => {
            // Sale cuando lo que lleva de verdad puede con el campamento, con el margen de la caza en grupo; si no se junta bastante a tiempo, se deshace y vuelve a entrar.
            let casa_id = casa.to_string();
            let poder_b = ctx.con_pizarra(|p| p.bandidos.get(&bandidos).map(|x| x.1)).or_else(|| {
                entidades(ctx.puerto.observar()).find_map(|c| match c {
                    Cuerpo::CampamentoBandidos(x) if x.id == bandidos => Some(x.poder),
                    _ => None,
                })
            });
            let decision = match poder_b {
                Some(n) => decidir_grupo(ctx, d, &format!("cazar:{bandidos}"), &casa_id, &bandidos, inicio, b::ESPERA_GRUPO_TOPE_MS, ESPERA_CAMPANA_MS, 0, |poder| poder as f64 > n as f64 * b::MARGEN_PARA_FUNDAR),
                None => Grupo::Deshacer,
            };
            match decision {
                Grupo::Esperar => {
                    ctx.memoria.salida = Some(Salida::CazarEnGrupo { bandidos, inicio });
                    return Ok(true);
                }
                Grupo::Deshacer => {
                    vetar(ctx, &format!("bandido:{bandidos}"), b::VETO_TRAS_DESHACER_MS);
                    return Ok(false);
                }
                Grupo::Salir => {}
            }
            repostar(ctx, Some(casa_id)).await?;
            let destino = ctx.con_pizarra(|p| p.bandidos.get(&bandidos).map(|b| b.0.clone())).map(|p| orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(p)) });
            ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: yo, objetivo: destino })).await?;
            ctx.memoria.salida = Some(Salida::Cazar { bandidos });
            Ok(true)
        }
        Salida::Buscar { paso } => {
            if !parada {
                ctx.memoria.salida = Some(Salida::Buscar { paso });
                return Ok(true);
            }
            // Ya conoce alguna y nadie la inspecciona: a inspeccionarla. Si no, otro tramo o, acabada la batida, a casa.
            let t = ctx.ahora_ms;
            let libre = ctx.con_pizarra(|p| p.tablero.libre("explorador", &yo, t));
            if let Some(otra) = siguiente_a_explorar(ctx, &ctx.faccion().unwrap_or_default(), &aqui).filter(|_| libre) {
                if marchar_a_plaza(ctx, &otra).await? {
                    ctx.con_pizarra(|p| {
                        p.tablero.tomar("explorador", &yo, t, t + b::TAREA_VALE_MS);
                        p.tablero.soltar(&format!("batida:{casa}"), &yo);
                        p.tablero.soltar_de("casilla:", &yo);
                    });
                    ctx.memoria.salida = Some(Salida::Explorar { plaza: otra });
                    return Ok(true);
                }
            }
            if paso < PASOS_DE_BATIDA {
                batir(ctx, &aqui, paso).await?;
            }
            Ok(ctx.memoria.salida.is_some())
        }
        Salida::Explorar { plaza } => {
            let sitio = plazas_ajenas(ctx, &ctx.faccion().unwrap_or_default()).into_iter().find(|(id, _, _)| *id == plaza).map(|(_, _, p)| p);
            let Some(sitio) = sitio else {
                return Ok(false);
            };
            if distancia(&sitio, &aqui) <= d.radio_inspeccion {
                let objetivo = Some(pb::orden_inspeccionar::Objetivo { tipo: Some(pb::orden_inspeccionar::objetivo::Tipo::AsentamientoId(plaza.clone())) });
                ctx.puerto.actuar(O::Inspeccionar(pb::OrdenInspeccionar { heroe_id: yo.clone(), objetivo })).await?;
                let visto = entidades(ctx.puerto.observar()).find_map(|c| match c {
                    Cuerpo::InformeInspeccion(i) if i.objetivo_id == plaza && i.heroe_id == yo => match &i.contenido {
                        Some(pb::informe_inspeccion::Contenido::Plaza(x)) => {
                            let poder: u32 = x.guarnicion.iter().map(|g| g.cantidad * d.poder.get(&g.tropa_id).copied().unwrap_or(0)).sum();
                            Some((poder, x.heroes_ids.len(), i.visto_en_ms))
                        }
                        _ => None,
                    },
                    _ => None,
                });
                if let Some(v) = visto {
                    ctx.con_pizarra(|p| p.defensas.insert(plaza.clone(), v));
                }
                // La siguiente; sin más que mirar, a casa.
                match siguiente_a_explorar(ctx, &ctx.faccion().unwrap_or_default(), &aqui) {
                    Some(otra) => {
                        let sale = marchar_a_plaza(ctx, &otra).await?;
                        if sale {
                            ctx.memoria.salida = Some(Salida::Explorar { plaza: otra });
                        }
                        Ok(sale)
                    }
                    None => {
                        Ok(false)
                    }
                }
            } else {
                if parada {
                    let destino = Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::AsentamientoId(plaza.clone())) });
                    let r = ctx.intentar(&format!("explorar:{plaza}"), O::MarcharA(pb::OrdenMarcharA { heroe_id: yo.clone(), objetivo: destino }), 10 * 60_000).await?;
                    if r.is_some_and(|r| olvidar_si_desconocida(ctx, &plaza, &r)) {
                        return Ok(false);
                    }
                }
                ctx.memoria.salida = Some(Salida::Explorar { plaza });
                Ok(true)
            }
        }
        Salida::GuarnecerAvanzada { avanzada, escuadra } => {
            if !parada {
                ctx.memoria.salida = Some(Salida::GuarnecerAvanzada { avanzada, escuadra });
                return Ok(true);
            }
            let sitio = entidades(ctx.puerto.observar()).find_map(|c| match c {
                Cuerpo::Avanzada(a) if a.id == avanzada => a.posicion.clone(),
                _ => None,
            });
            if sitio.is_some_and(|s| distancia(&s, &aqui) <= d.radio_puerta) {
                let o = O::GuarnecerAvanzada(pb::OrdenGuarnecerAvanzada { heroe_id: yo.clone(), avanzada_id: avanzada.clone(), escuadron_ids: vec![escuadra] });
                if ctx.puerto.actuar(o).await?.resultado != pb::Resultado::OrdenAceptada as i32 {
                    vetar(ctx, &format!("guarnecer:{avanzada}"), 24 * 3_600_000);
                }
            }
            Ok(false)
        }
        Salida::Asaltar { caravana } => {
            let donde = entidades(ctx.puerto.observar()).find_map(|c| match c {
                Cuerpo::Caravana(x) if x.id == caravana => x.posicion.clone(),
                _ => None,
            });
            match donde {
                Some(p) if distancia(&p, &aqui) <= d.radio_encuentro => {
                    let objetivo = Some(pb::orden_atacar::Objetivo { tipo: Some(pb::orden_atacar::objetivo::Tipo::CaravanaId(caravana.clone())) });
                    ctx.puerto.actuar(O::Atacar(pb::OrdenAtacar { heroe_id: yo, objetivo, declara_guerra: Some(true) })).await?;
                    vetar(ctx, &format!("asaltar:{caravana}"), 24 * 3_600_000);
                    Ok(false)
                }
                // La persigue mientras la ve y su columna anda; perdida o parada, a casa.
                Some(_) if !parada => {
                    ctx.memoria.salida = Some(Salida::Asaltar { caravana });
                    Ok(true)
                }
                _ => Ok(false),
            }
        }
        Salida::Entregar { obra } => {
            if !parada {
                ctx.memoria.salida = Some(Salida::Entregar { obra });
                return Ok(true);
            }
            let sitio = entidades(ctx.puerto.observar()).find_map(|c| match c {
                Cuerpo::Obra(o) if o.id == obra => o.posicion.clone(),
                _ => None,
            });
            if sitio.is_some_and(|s| distancia(&s, &aqui) <= d.radio_puerta) {
                ctx.puerto.actuar(O::EntregarEnObra(pb::OrdenEntregarEnObra { heroe_id: yo.clone(), obra_id: obra.clone(), materiales: None })).await?;
            }
            Ok(false)
        }
        Salida::Saquear { pueblo } => {
            let sitio = ctx.con_pizarra(|p| p.pueblos.get(&pueblo).map(|(_, _, q, _)| q.clone()));
            match sitio {
                Some(_) if !parada => {
                    ctx.memoria.salida = Some(Salida::Saquear { pueblo });
                    Ok(true)
                }
                // Solo un ejército saquea (`combate.solo_un_ejercito`): una columna personal vuelve a casa.
                Some(s) if distancia(&s, &aqui) <= d.radio_encuentro && col.tipo == pb::TipoColumna::ColumnaEjercito as i32 => {
                    // Un mismo pueblo no se saquea sin pausa: se vuelve cuando tenga algo que llevarse.
                    vetar(ctx, &format!("saquear:{pueblo}"), b::ENTRE_SAQUEOS_MS);
                    let objetivo = Some(pb::orden_atacar::Objetivo { tipo: Some(pb::orden_atacar::objetivo::Tipo::PuebloId(pueblo.clone())) });
                    let r = ctx.puerto.actuar(O::Atacar(pb::OrdenAtacar { heroe_id: yo, objetivo, declara_guerra: Some(true) })).await?;
                    if r.resultado != pb::Resultado::OrdenAceptada as i32 {
                        vetar(ctx, &format!("saquear:{pueblo}"), 24 * 3_600_000);
                    }
                    Ok(false)
                }
                _ => Ok(false),
            }
        }
        Salida::FundarEnGrupo { sitio, caravana, inicio } => {
            // Sale cuando lo que lleva de verdad la columna puede con el camino; si no se juntan bastantes a tiempo, deshace la caravana (recupera lo que costó) y vuelve.
            let casa_id = casa.to_string();
            let puerta = plaza_de(ctx, casa).unwrap_or_default();
            let vista: &Contexto<'_, C> = ctx;
            let decision = decidir_grupo(vista, d, &format!("fundar:{casa}"), &casa_id, "fundar", inicio, b::ESPERA_PARA_FUNDAR_MS, ESPERA_CAMPANA_MS, 0, |poder| crate::cerebro::camino_seguro(vista, poder, &puerta, &sitio));
            match decision {
                Grupo::Esperar => {
                    ctx.memoria.salida = Some(Salida::FundarEnGrupo { sitio, caravana, inicio });
                    Ok(true)
                }
                Grupo::Salir => {
                    repostar(ctx, Some(casa_id)).await?;
                    ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: yo, objetivo: Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(sitio.clone())) }) })).await?;
                    ctx.memoria.salida = Some(Salida::Fundar { sitio, intentos: 0 });
                    Ok(true)
                }
                Grupo::Deshacer => {
                    ctx.puerto.actuar(O::DesarmarCaravanaFundacion(pb::OrdenDesarmarCaravanaFundacion { caravana_id: caravana })).await?;
                    Ok(false)
                }
            }
        }
        Salida::Fundar { sitio, intentos } => {
            if !parada {
                ctx.memoria.salida = Some(Salida::Fundar { sitio, intentos });
                return Ok(true);
            }
            if ctx.puerto.actuar(O::Fundar(pb::OrdenFundar {})).await?.resultado == pb::Resultado::OrdenAceptada as i32 {
                // Fundada: la tarea está hecha y sale del tablero ya, no cuando caduque.
                {
                    let yo = ctx.yo.to_string();
                    ctx.con_pizarra(|p| p.tablero.soltar_de("fundar:", &yo));
                }
                crate::cerebro::evento(ctx, "funda", format!("funda en ({}, {}) desde {casa}", aqui.x / 1000, aqui.y / 1000), serde_json::json!({"x": aqui.x, "y": aqui.y, "desde": casa}));
                if let Some(f) = ctx.faccion() {
                    ctx.pedir_refuerzos(&f);
                }
                return Ok(false); // ya reside en la plaza nueva
            }
            if intentos + 1 >= INTENTOS_DE_FUNDAR {
                return Ok(false);
            }
            let Some(otro) = crate::cerebro::sitio_para_fundar(ctx, d, &aqui, ANILLO_PLAZA).await? else { return Ok(false) };
            ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: yo, objetivo: Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::Punto(otro.clone())) }) })).await?;
            ctx.memoria.salida = Some(Salida::Fundar { sitio: otro, intentos: intentos + 1 });
            Ok(true)
        }
        Salida::Campana { plaza, lidera, marcha, inicio } => {
            if !lidera {
                // Va donde va quien la lleva: de vuelta en casa se separa (`volver_a_casa`).
                let lejos = plaza_de(ctx, casa).is_none_or(|p| distancia(&p, &aqui) > d.radio_puerta);
                if lejos || !parada {
                    ctx.memoria.salida = Some(Salida::Campana { plaza, lidera, marcha, inicio });
                }
                return Ok(lejos || !parada);
            }
            if !parada {
                ctx.memoria.salida = Some(Salida::Campana { plaza, lidera, marcha, inicio });
                return Ok(true);
            }
            if !marcha {
                // En la puerta: sale cuando lo que lleva de verdad puede con la defensa que vio el explorador, con su margen; si no se junta bastante a tiempo, se deshace.
                let (casa_id, mia) = (casa.to_string(), ctx.faccion().unwrap_or_default());
                let suya = plazas_ajenas(ctx, &mia).into_iter().find(|(id, _, _)| *id == plaza);
                let margen = suya.as_ref().map_or(1.0, |(_, f, _)| margen_contra(ctx, &mia, f));
                let necesaria = ctx.con_pizarra(|p| p.defensas.get(&plaza).map(|&(defensa, heroes, _)| (defensa + heroes as u32 * d.poder_por_heroe) as f64 * margen));
                let decision = decidir_grupo(ctx, d, &format!("campana:{plaza}"), &casa_id, &plaza, inicio, b::ESPERA_GRUPO_TOPE_MS, ESPERA_CAMPANA_MS, 0, |poder| necesaria.is_none_or(|n| poder as f64 > n));
                match decision {
                    Grupo::Esperar => {
                        ctx.memoria.salida = Some(Salida::Campana { plaza, lidera, marcha, inicio });
                        return Ok(true);
                    }
                    Grupo::Deshacer => {
                        vetar(ctx, &format!("atacar:{plaza}"), b::VETO_TRAS_DESHACER_MS);
                        return Ok(false);
                    }
                    Grupo::Salir => {}
                }
                repostar(ctx, Some(casa_id)).await?;
                // Sin víveres para ir y volver, no sale: la campaña se deshace en la puerta y la plaza queda vetada un día.
                if suya.as_ref().is_some_and(|(_, _, p)| !le_llega(ctx, d, p)) {
                    vetar(ctx, &format!("atacar:{plaza}"), 24 * 3_600_000);
                    return Ok(false);
                }
                if !marchar_a_plaza(ctx, &plaza).await? {
                    return Ok(false);
                }
                ctx.memoria.salida = Some(Salida::Campana { plaza, lidera, marcha: true, inicio });
                return Ok(true);
            }
            let mia = ctx.faccion().unwrap_or_default();
            let suya = entidades(ctx.puerto.observar()).find_map(|c| match c {
                Cuerpo::Asentamiento(a) if a.id == plaza => Some((a.faccion_id.clone(), a.posicion.clone().unwrap_or_default())),
                _ => None,
            });
            match suya {
                // Quien conquista se queda (§16): se muda a la plaza tomada y la guarnece, si deja al menos otro residente en casa.
                Some((f, _)) if f == mia => {
                    let quedan = self::plaza(ctx, casa).map_or(0, |p| p.casas_compradas.len());
                    if quedan >= 2 && aceptada(&Some(ctx.puerto.actuar(O::CambiarResidencia(pb::OrdenCambiarResidencia { destino_id: plaza.clone(), heroe_id: yo.clone() })).await?)) {
                        ctx.puerto.actuar(O::Guarnecer(pb::OrdenGuarnecer { asentamiento_id: plaza, heroe_id: yo })).await?;
                    }
                    Ok(false)
                }
                Some((_, p)) if distancia(&p, &aqui) <= d.radio_encuentro => {
                    let objetivo = Some(pb::orden_atacar::Objetivo { tipo: Some(pb::orden_atacar::objetivo::Tipo::AsentamientoId(plaza.clone())) });
                    let r = ctx.puerto.actuar(O::Atacar(pb::OrdenAtacar { heroe_id: yo, objetivo, declara_guerra: Some(true) })).await?;
                    // Protegida (recién fundada o conquistada): nadie de la Facción vuelve a por ella en un día.
                    if r.rechazo.as_ref().is_some_and(|x| x.codigo == "combate.plaza_protegida") {
                        vetar(ctx, &format!("atacar:{plaza}"), 24 * 3_600_000);
                    }
                    // Aceptado: el resultado se ve la próxima vez.
                    let sigue = r.resultado == pb::Resultado::OrdenAceptada as i32;
                    if sigue {
                        ctx.memoria.salida = Some(Salida::Campana { plaza, lidera, marcha, inicio });
                    }
                    Ok(sigue)
                }
                _ => Ok(false),
            }
        }
    }
}

/// Marcha hacia una plaza ajena. Si el servidor no la conoce (cayó en ruinas), la Facción la olvida: si no, sus bots vuelven a por ella cada vez que piensan.
async fn marchar_a_plaza<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &str) -> Result<bool, Fallo> {
    let destino = Some(orden_marchar_a::Objetivo { tipo: Some(orden_marchar_a::objetivo::Tipo::AsentamientoId(plaza.into())) });
    let r = ctx.puerto.actuar(O::MarcharA(pb::OrdenMarcharA { heroe_id: ctx.yo.to_string(), objetivo: destino })).await?;
    olvidar_si_desconocida(ctx, plaza, &r);
    Ok(r.resultado == pb::Resultado::OrdenAceptada as i32)
}

fn olvidar_si_desconocida<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, plaza: &str, r: &pb::ResultadoOrden) -> bool {
    let desconocida = r.rechazo.as_ref().is_some_and(|x| x.codigo == "asentamiento.desconocido");
    if desconocida {
        ctx.con_pizarra(|p| {
            p.ajenas.remove(plaza);
            p.defensas.remove(plaza);
        });
    }
    desconocida
}

fn plaza_de<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, id: &str) -> Option<Punto> {
    plaza(ctx, id).and_then(|p| p.posicion)
}

// ───────────────────────────── Aedas ─────────────────────────────

/// Los Aedas (Doc 6.7, TypeScript `gobierno.ts`): con un itinerante en la plaza, el Rey o el Gobernador (`compra`) le compran una tecnología revelada (una por turno; el
/// servidor dice si la plaza puede pagarla); y cada residente de la plaza sin épica empieza la primera que su Facción puede cantar, sin repetir.
/// ponytail: sin el margen de TypeScript (guardar el doble de la tarifa de adopción): llega con la calibración de la tecnología.
async fn aedas<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &pb::Asentamiento, compra: bool) -> Result<(), Fallo> {
    let a = ctx.puerto.observar();
    let tec = entidades(a).find_map(|c| match c {
        Cuerpo::Tecnologias(t) if t.faccion_id == plaza.faccion_id => Some(t.clone()),
        _ => None,
    });
    let Some(tec) = tec else { return Ok(()) };
    let itinerante = entidades(a).any(|c| matches!(c, Cuerpo::AedaItinerante(x) if x.en_asentamiento_id.as_deref() == Some(plaza.id.as_str())));
    let libres: Vec<String> = entidades(a)
        .filter_map(|c| match c {
            Cuerpo::AedaResidente(x) if x.asentamiento_id == plaza.id && x.epica.is_none() => Some(x.id.clone()),
            _ => None,
        })
        .collect();
    if compra && itinerante {
        if let Some(t) = tec.reveladas.iter().find(|t| !tec.adoptadas.contains(t)) {
            let o = O::ComprarTecnologiaAeda(pb::OrdenComprarTecnologiaAeda { asentamiento_id: plaza.id.clone(), tecnologia_id: t.clone() });
            ctx.intentar(&format!("aeda:{t}"), o, 60 * 60_000).await?;
        }
    }
    // `epicas_posibles` ya deja fuera las que canta otro Aeda de la Facción; lo revelado se puede comprar, así que las épicas, para lo demás.
    let mut posibles: Vec<&String> = tec.epicas_posibles.iter().collect();
    posibles.sort_by_key(|t| tec.reveladas.contains(t));
    for (aeda, t) in libres.into_iter().zip(posibles) {
        let o = O::EmpezarEpica(pb::OrdenEmpezarEpica { asentamiento_id: plaza.id.clone(), aeda_id: aeda.clone(), tecnologia_id: t.clone() });
        ctx.intentar(&format!("epica:{aeda}"), o, 60 * 60_000).await?;
    }
    Ok(())
}

// ───────────────────────────── Tesorero ─────────────────────────────

/// Por debajo de esta fracción de su capacidad, de un recurso no sobra (ni para pagar ni para vender); por encima de la primera se vende, por debajo de la segunda se compra.
const COLCHON: f64 = 0.3;
const EXCEDENTE: f64 = 0.7;
const ESCASEZ: f64 = 0.2;
const FRACCION_A_VENDER: f64 = 0.25;
/// El trueque de supervivencia salta cuando lo que queda de un recurso que baja no cubre estas horas (§16).
const HORAS_DE_RESERVA: i64 = 4;
/// Lo que pasa entre una orden de mercado y la siguiente del mismo recurso.
const ESPERA_TRAS_MERCADO_MS: i64 = 6 * 3_600_000;

/// Cuánto hay de un recurso ahora: su ritmo evaluado en `t`, entre su mínimo y su capacidad.
fn cantidad(e: &pb::Existencia, t: i64) -> i64 {
    let Some(r) = e.cantidad.as_ref() else { return 0 };
    let mut v = if r.en_escalones { r.valor_en_t0 } else { r.valor_en_t0 + r.por_minuto * (t - r.t0_ms).max(0) / 60_000 };
    if let Some(max) = r.maximo {
        v = v.min(max);
    }
    v.clamp(r.minimo.unwrap_or(0), e.capacidad.max(0))
}

fn fraccion(e: &pb::Existencia, t: i64) -> f64 {
    if e.capacidad <= 0 {
        0.0
    } else {
        cantidad(e, t) as f64 / e.capacidad as f64
    }
}

async fn tesorero<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    presion_fiscal(ctx, d, plaza).await?;
    pausar_lujo(ctx, plaza).await?;
    let mercado = plaza.edificios.iter().any(|e| e.tipo == "mercado" && e.estado == EstadoEdificio::EdificioActivo as i32);
    // Contesta aunque no tenga mercado: sin él no tiene caravanas para su parte, y lo rechaza para que el otro no espere a que caduque.
    contestar_trueques(ctx, d, plaza, mercado).await?;
    if !mercado {
        return Ok(());
    }
    caravanas_comerciales(ctx, plaza).await?;
    pedir_estano(ctx, plaza).await?;
    proponer_trueque(ctx, d, plaza).await?;
    mercado_y_supervivencia(ctx, d, plaza).await
}

/// Lo que la plaza quiere guardar de cada bien (milésimas), y por qué: desde el nivel 2, la meta del bronce ([`b::META_BRONCE`]); lo que les falta a sus obras; y de cada
/// comida que se le gasta, lo de [`b::META_HORAS_DE_COMIDA`]. Nunca más de lo que le cabe. Lo de la meta no se vende y se compra antes que nada.
pub(crate) fn metas<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> BTreeMap<String, (i64, &'static str)> {
    let mut m: BTreeMap<String, (i64, &'static str)> = BTreeMap::new();
    if plaza.nivel_actual >= 2 {
        for (bien, n) in b::META_BRONCE {
            m.insert(bien.to_string(), (n * 1000, "el bronce del nivel 3"));
        }
    }
    for c in entidades(ctx.puerto.observar()) {
        if let Cuerpo::Obra(o) = c {
            if o.asentamiento_madre_id == plaza.id && o.termina_en_ms.is_none() {
                for (r, n) in o.requerido.as_ref().map(|x| x.por_id.clone()).unwrap_or_default() {
                    let falta = n - o.entregado.as_ref().and_then(|e| e.por_id.get(&r)).copied().unwrap_or(0);
                    if falta > 0 {
                        let e = m.entry(r).or_insert((0, "sus obras"));
                        e.0 += falta;
                    }
                }
            }
        }
    }
    for (r, e) in &plaza.almacen {
        let gasto = e.cantidad.as_ref().map_or(0, |x| -x.por_minuto);
        if gasto > 0 && matches!(b::clase_de_bien(r), "comida" | "lujo") {
            let n = gasto * 60 * b::META_HORAS_DE_COMIDA;
            let x = m.entry(r.clone()).or_insert((0, "lo que come su gente"));
            x.0 = x.0.max(n);
        }
    }
    // La capital guarda la tarifa de la tecnología que ya puede adoptar (menos el oro, que no se compra) y algo de lo que un hito pide en ella (el estaño de la aleación).
    let t = ctx.ahora_ms;
    let mut subir = |r: &str, n: i64, por: &'static str| {
        let x = m.entry(r.to_string()).or_insert((0, por));
        x.0 = x.0.max(n);
    };
    let f = &plaza.faccion_id;
    let (tec, _) = tecnologias(ctx, f);
    if capital_de(ctx, f).as_deref() == Some(plaza.id.as_str()) {
        let por_adoptar = tec.aparecidas.iter().filter(|x| !tec.adoptadas.contains(x)).find_map(|id| d.tecnologias.iter().find(|x| x.id == *id));
        for (r, n) in por_adoptar.and_then(|x| d.tarifas.get(&x.era)).into_iter().flatten().filter(|(r, _)| r != "oro") {
            subir(r, n * 1000, "adoptar una tecnología");
        }
        for (_, faltan) in pendientes(ctx, d, f) {
            for h in faltan {
                if let Hito::EnCapital(r) = h {
                    subir(r, b::RESERVA_DE_HITO * 1000, "el hito de una tecnología");
                }
            }
        }
    }
    // El equipo de una escuadra entera de la tropa siguiente, cuando ya tiene su edificio.
    if let Some(s) = tropa_siguiente(d, plaza, &tec.adoptadas, t).filter(|s| s.pasos.is_empty()) {
        for (r, falta) in &s.equipo {
            subir(r, plaza.almacen.get(r).map_or(0, |e| cantidad(e, t)) + falta, "equipar la tropa siguiente");
        }
    }
    for (r, x) in m.iter_mut() {
        if let Some(e) = plaza.almacen.get(r) {
            x.0 = x.0.min(e.capacidad.max(0));
        }
    }
    m
}

/// La comida de lujo (queso, vino, cerveza, aceite) deja de comerse para venderla cuando a la plaza le falta oro y le sobra de ella; vuelve a comerse cuando el oro se recupera
/// o se le acaba. Lo da el Tesorero o el Gobernador (`pausar_consumo`, motor 2026-11-10).
async fn pausar_lujo<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let t = ctx.ahora_ms;
    let oro = plaza.almacen.get("oro").map_or(1.0, |e| fraccion(e, t));
    for (r, e) in &plaza.almacen {
        if b::clase_de_bien(r) != "lujo" {
            continue;
        }
        let pausado = plaza.consumo_pausado.contains(r);
        let quiere = if pausado { oro < COLCHON && fraccion(e, t) > ESCASEZ } else { oro < b::LUJO_PAUSA_ORO && fraccion(e, t) > b::LUJO_PAUSA_SOBRA };
        if quiere != pausado {
            let o = O::PausarConsumo(pb::OrdenPausarConsumo { asentamiento_id: plaza.id.clone(), recurso: r.clone(), pausado: quiere });
            ctx.intentar(&format!("pausar:{}:{r}", plaza.id), o, ESPERA_TRAS_RECHAZO_MS).await?;
        }
    }
    Ok(())
}

/// Cada cuánto propone un trueque el Tesorero: el mercantil, más a menudo. ponytail: a calibrar con el batch.
const ENTRE_TRUEQUES_MS: i64 = 6 * 3_600_000;

/// Lo que cada plaza con mercado anuncia en su mostrador: lo que le sobra y lo que le falta, con cuánto (milésimas), y cuándo lo anunció. Como el tablón de mostradores
/// del TypeScript: lo que un jugador sabría preguntando en el mercado. Por servidor y plaza. ponytail: global del proceso; a la pizarra si los bots se reparten en procesos.
type Mostrador = (Vec<(String, i64)>, Vec<(String, i64)>, i64);
static MOSTRADORES: std::sync::Mutex<BTreeMap<(String, String), Mostrador>> = std::sync::Mutex::new(BTreeMap::new());
/// Un mostrador más viejo que esto ya no cuenta.
const MOSTRADOR_VALE_MS: i64 = 12 * 3_600_000;

/// Comercio entre bots (Comercio_Caravanas §4): el Tesorero anuncia su mostrador y propone un trueque a la plaza más cercana (de otra Facción que no esté en guerra con la
/// suya, o de la suya) cuyo mostrador ofrezca lo que a él le escasea (por debajo de [`ESCASEZ`]); a cambio da de lo que le sobra (por encima de [`EXCEDENTE`]), mejor si es
/// lo que a la otra le falta. La otra plaza lo acepta si de verdad le sobra (`contestar_trueques`), y las caravanas comerciales de las dos lo llevan solas.
async fn proponer_trueque<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let t = ctx.ahora_ms;
    let unidades = |n: i64| n / 1000 * 1000;
    let metas = metas(ctx, d, plaza);
    let meta = |r: &str| metas.get(r).map_or(0, |x| x.0);
    // Sobra lo que pasa del excedente sin bajar de su meta; falta lo que no llega a su meta y lo escaso.
    let mut sobra: Vec<(String, i64)> = plaza
        .almacen
        .iter()
        .filter(|(_, e)| fraccion(e, t) >= EXCEDENTE)
        .map(|(r, e)| (r.clone(), unidades((((cantidad(e, t) as f64 - e.capacidad as f64 * EXCEDENTE) * FRACCION_A_VENDER) as i64).min(cantidad(e, t) - meta(r)))))
        .filter(|(_, n)| *n > 0)
        .collect();
    let mut falta: Vec<(String, i64)> = plaza
        .almacen
        .iter()
        .filter_map(|(r, e)| {
            let ahora = cantidad(e, t);
            let n = (meta(r) - ahora).max(if fraccion(e, t) <= ESCASEZ { (e.capacidad as f64 * ESCASEZ) as i64 - ahora } else { 0 });
            (unidades(n) > 0).then(|| (r.clone(), unidades(n)))
        })
        .collect();
    // Lo que vale más primero: así sale antes el bronce que la madera.
    let valor = |r: &str, n: i64| (d.precio(r) * n as f64) as i64;
    sobra.sort_by_key(|(r, n)| (std::cmp::Reverse(valor(r, *n)), r.clone()));
    falta.sort_by_key(|(r, n)| (meta(r) == 0, std::cmp::Reverse(valor(r, *n)), r.clone()));
    let servidor = ctx.puerto.servidor().to_string();
    let mostradores: BTreeMap<String, Mostrador> = {
        let mut m = MOSTRADORES.lock().unwrap();
        m.insert((servidor.clone(), plaza.id.clone()), (sobra.clone(), falta.clone(), t));
        m.iter().filter(|((s, id), (_, _, en))| *s == servidor && *id != plaza.id && t - en <= MOSTRADOR_VALE_MS).map(|((_, id), x)| (id.clone(), x.clone())).collect()
    };
    let clave = format!("proponer:{}", plaza.id);
    if vetado(ctx, &clave) || sobra.is_empty() || falta.is_empty() {
        return Ok(());
    }
    let mia = plaza.faccion_id.clone();
    let centro = plaza.posicion.clone().unwrap_or_default();
    let en_guerra = |ctx: &Contexto<'_, C>, otra: &str| {
        entidades(ctx.puerto.observar()).any(|c| {
            matches!(c, Cuerpo::Relacion(r) if r.estado == pb::EstadoRelacion::RelacionActiva as i32 && r.tipo == pb::TipoRelacion::RelacionGuerra as i32
                && ((r.faccion_a_id == mia && r.faccion_b_id == otra) || (r.faccion_a_id == otra && r.faccion_b_id == mia)))
        })
    };
    // Las que le rechazaron uno en el último día, fuera.
    let rechazan: Vec<String> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::AcuerdoTrueque(x) if x.asentamiento_a_id == plaza.id && x.estado == pb::EstadoTrueque::TruequeRechazado as i32 && t - x.creado_en_ms < 86_400_000 => {
                Some(x.asentamiento_b_id.clone())
            }
            _ => None,
        })
        .collect();
    let mut candidatas: Vec<(String, Punto)> = plazas_ajenas(ctx, &mia).into_iter().filter(|(_, f, _)| !en_guerra(ctx, f) && politica::estado(ctx, f) > Estado::Recelo).map(|(id, _, p)| (id, p)).collect();
    candidatas.extend(entidades(ctx.puerto.observar()).filter_map(|c| match c {
        Cuerpo::Asentamiento(a) if a.faccion_id == mia && a.id != plaza.id => Some((a.id.clone(), a.posicion.clone().unwrap_or_default())),
        _ => None,
    }));
    candidatas.retain(|(id, _)| !rechazan.contains(id));
    candidatas.sort_by_key(|(id, p)| (distancia(p, &centro), id.clone()));
    // La primera que ofrezca algo de lo que le falta: lo que pide, cuánto, y lo que da (lo que a ella le falta si puede).
    let trato = candidatas.iter().find_map(|(id, _)| {
        let (su_sobra, su_falta, _) = mostradores.get(id)?;
        let (pide, n_pide) = falta.iter().find_map(|(r, n)| su_sobra.iter().find(|(s, _)| s == r).map(|(_, m)| (r.clone(), (*n).min(*m))))?;
        let (da, n_da) = sobra.iter().filter(|(r, _)| *r != pide).find(|(r, _)| su_falta.iter().any(|(f, _)| f == r)).or(sobra.iter().find(|(r, _)| *r != pide))?;
        // Por valor, no por cantidad: da lo que vale lo que pide; si no le sobra tanto, pide menos.
        let (p_pide, p_da) = (d.precio(&pide).max(0.01), d.precio(da).max(0.01));
        let da_justo = unidades(((n_pide as f64 * p_pide / p_da) as i64 + 999) / 1000 * 1000);
        let (n_da, n_pide) = if da_justo <= *n_da { (da_justo.max(1000), n_pide) } else { (*n_da, unidades((*n_da as f64 * p_da / p_pide) as i64)) };
        Some((id.clone(), pide, da.clone(), n_pide, n_da))
    });
    let Some((otra, pide, da, n_pide, n_da)) = trato.filter(|(_, _, _, n_pide, n_da)| *n_pide > 0 && *n_da > 0) else { return Ok(()) };
    let yo = ctx.yo.to_string();
    ctx.con_pizarra(|p| p.tablero.tomar(&format!("comprar:{pide}"), &yo, t, t + b::NECESIDAD_VALE_MS));
    let o = O::ProponerTrueque(pb::OrdenProponerTrueque {
        asentamiento_a_id: plaza.id.clone(),
        lineas_a: vec![pb::orden_proponer_trueque::LineasA { recurso: da, cantidad: n_da }],
        asentamiento_b_id: otra,
        lineas_b: vec![pb::orden_proponer_trueque::LineasB { recurso: pide, cantidad: n_pide }],
    });
    let entre = if Personalidad::de(&mia) == Personalidad::Mercantil { ENTRE_TRUEQUES_MS / 3 } else { ENTRE_TRUEQUES_MS };
    vetar(ctx, &clave, entre);
    ctx.intentar(&clave, o, ESPERA_TRAS_RECHAZO_MS).await?;
    Ok(())
}

/// El estaño por trueque: el Tesorero lo pide a la plaza más cercana de quien lo tiene y da a cambio lo que más le sobra. A un jugador le llega como a cualquier otra plaza, sin
/// mostrador. Cada petición sin fruto acerca la guerra si esa Facción no es amiga.
async fn pedir_estano<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let t = ctx.ahora_ms;
    let Some(f) = ctx.con_pizarra(|p| match p.libro.estano.as_ref().map(|e| &e.via) {
        Some(Via::Trueque { faccion }) => Some(faccion.clone()),
        _ => None,
    }) else {
        return Ok(());
    };
    let clave = format!("estano:{}", plaza.id);
    if vetado(ctx, &clave) {
        return Ok(());
    }
    let centro = plaza.posicion.clone().unwrap_or_default();
    let Some((otra, _, _)) = plazas_ajenas(ctx, &plaza.faccion_id).into_iter().filter(|(_, x, _)| *x == f).min_by_key(|(_, _, p)| distancia(p, &centro)) else { return Ok(()) };
    let da = plaza.almacen.iter().filter(|(r, e)| r.as_str() != "estano" && fraccion(e, t) >= EXCEDENTE).max_by_key(|(_, e)| cantidad(e, t)).map(|(r, _)| r.clone());
    let Some(da) = da else { return Ok(()) };
    vetar(ctx, &clave, b::ENTRE_PETICIONES_ESTANO_MS);
    ctx.con_pizarra(|p| {
        if let Some(e) = &mut p.libro.estano {
            e.peticiones += 1;
        }
    });
    let n = b::ESTANO_POR_TRUEQUE;
    let o = O::ProponerTrueque(pb::OrdenProponerTrueque {
        asentamiento_a_id: plaza.id.clone(),
        lineas_a: vec![pb::orden_proponer_trueque::LineasA { recurso: da, cantidad: n }],
        asentamiento_b_id: otra,
        lineas_b: vec![pb::orden_proponer_trueque::LineasB { recurso: "estano".into(), cantidad: n }],
    });
    ctx.intentar(&clave, o, ESPERA_TRAS_RECHAZO_MS).await?;
    Ok(())
}

/// Presión Fiscal (§15): con el oro escaso (por debajo de [`ESCASEZ`] de su capacidad, o bajando) y la población que pide la subida siguiente ya en la plaza, el Tesorero
/// activa la política: recauda más a costa de crecer menos, cuando crecer ya no es lo que falta. ponytail: el criterio, a calibrar con el batch.
async fn presion_fiscal<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let t = ctx.ahora_ms;
    if plaza.politicas_activas.iter().any(|p| p.politica_id == "presion_fiscal") {
        return Ok(());
    }
    let oro_escaso = plaza.almacen.get("oro").is_some_and(|e| fraccion(e, t) < ESCASEZ || e.cantidad.as_ref().is_some_and(|r| r.por_minuto < 0));
    let pesants = plaza.poblacion.as_ref().and_then(|p| p.pesants.as_ref()).map_or(0, |r| r.valor_en_t0);
    let poblada = d.requisitos.get(&(plaza.nivel_actual + 1)).is_none_or(|r| pesants >= r.pesants);
    if oro_escaso && poblada {
        let o = O::ActivarPolitica(pb::OrdenActivarPolitica { asentamiento_id: plaza.id.clone(), cargo: "tesorero".into(), politica_id: "presion_fiscal".into() });
        ctx.intentar(&format!("presion_fiscal:{}", plaza.id), o, ESPERA_TRAS_RECHAZO_MS).await?;
    }
    Ok(())
}

/// Todas las caravanas comerciales que le quepan (carro básico con buey), completando antes las que quedaron a medias. El cupo lo dice el servidor al rechazar la siguiente.
async fn caravanas_comerciales<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let propias: Vec<pb::Caravana> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Caravana(x) if x.tipo == pb::TipoCaravana::CaravanaComercial as i32 && x.origen_asentamiento_id == plaza.id => Some(x.clone()),
            _ => None,
        })
        .collect();
    let mut a_medias = false;
    for c in &propias {
        match c.carros.first() {
            None => {
                a_medias = true;
                ctx.intentar(&format!("carro:{}", c.id), O::AgregarCarroCaravana(pb::OrdenAgregarCarroCaravana { caravana_id: c.id.clone(), tipo_carro: "basico".into() }), ESPERA_OBRA_MS).await?;
            }
            Some(carro) if carro.animal.is_none() => {
                a_medias = true;
                let o = O::ComprarAnimalCaravana(pb::OrdenComprarAnimalCaravana { caravana_id: c.id.clone(), carro_indice: 0, tipo_animal: "buey".into() });
                ctx.intentar(&format!("animal:{}", c.id), o, ESPERA_OBRA_MS).await?;
            }
            _ => {}
        }
    }
    if !a_medias {
        // Con el cupo lleno, no vuelve a probar hasta el día siguiente (sube con el nivel del mercado).
        ctx.intentar(&format!("caravana:{}", plaza.id), O::CrearCaravana(pb::OrdenCrearCaravana { asentamiento_id: plaza.id.clone() }), 86_400_000).await?;
    }
    Ok(())
}

/// Acepta los trueques que le proponen si tiene mercado (sus caravanas llevan su parte) y le sobra lo que tendría que entregar; si no, los rechaza.
async fn contestar_trueques<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento, mercado: bool) -> Result<(), Fallo> {
    let t = ctx.ahora_ms;
    let metas = metas(ctx, d, plaza);
    let valor = |l: &[pb::LineaTrueque]| l.iter().map(|x| d.precio(&x.recurso) * x.cantidad_total as f64).sum::<f64>();
    let duenas: BTreeMap<String, String> = plazas_ajenas(ctx, &plaza.faccion_id).into_iter().map(|(id, f, _)| (id, f)).collect();
    let codicioso = ctx.con_pizarra(|p| p.rey.as_deref().is_some_and(|r| rasgos(r).contains(&Rasgo::Codicioso)));
    let propuestos: Vec<(String, bool)> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::AcuerdoTrueque(x) if x.estado == pb::EstadoTrueque::TruequePropuesto as i32 && x.asentamiento_b_id == plaza.id => {
                // Le sobra lo que entrega sin bajar de su meta, y lo que recibe vale lo que da (a precio base).
                let sobra = mercado
                    && x.lineas_b.iter().all(|l| plaza.almacen.get(&l.recurso).is_some_and(|e| fraccion(e, t) > COLCHON && cantidad(e, t) - l.cantidad_total >= metas.get(&l.recurso).map_or(0, |m| m.0)))
                    && valor(&x.lineas_a) >= valor(&x.lineas_b) * b::TRUEQUE_VALOR_MINIMO;
                let f = duenas.get(&x.asentamiento_a_id).cloned().unwrap_or_default();
                // Embargo: con recelo no comercia, salvo un Rey codicioso.
                Some((x.id.clone(), sobra && (codicioso || politica::estado(ctx, &f) > Estado::Recelo)))
            }
            _ => None,
        })
        .collect();
    for (id, sobra) in propuestos {
        let o = if sobra { O::AceptarTrueque(pb::OrdenAceptarTrueque { acuerdo_id: id.clone() }) } else { O::RechazarTrueque(pb::OrdenRechazarTrueque { acuerdo_id: id.clone() }) };
        ctx.intentar(&format!("trueque:{id}"), o, ESPERA_OBRA_MS).await?;
    }
    Ok(())
}

/// En su mercado: el trueque de supervivencia (lo que baja y no cubre [`HORAS_DE_RESERVA`] se compra a tiempo), y luego vende lo que le sobra y compra lo que le escasea, sin
/// duplicar órdenes. ponytail: compra en su propio mercado; el trueque con una plaza que lo vende (TypeScript, la pizarra de mostradores) llega con el explorador de plazas.
async fn mercado_y_supervivencia<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, d: &Datos, plaza: &pb::Asentamiento) -> Result<(), Fallo> {
    let t = ctx.ahora_ms;
    let activas: Vec<String> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::OrdenMercado(o) if o.asentamiento_id == plaza.id && o.estado == pb::EstadoOrden::OrdenActiva as i32 => Some(o.recurso.clone()),
            _ => None,
        })
        .collect();
    // Las órdenes de otras plazas que ve: solo pone la suya si hay quien esté al otro lado (sin contraparte, caducan todas).
    // ponytail: sin una orden contraria a la vista no comercia; los trueques entre bots llegan con su paso.
    let contrarias: Vec<(String, bool)> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::OrdenMercado(o) if o.asentamiento_id != plaza.id && o.estado == pb::EstadoOrden::OrdenActiva as i32 => Some((o.recurso.clone(), o.es_compra)),
            _ => None,
        })
        .collect();
    let metas = metas(ctx, d, plaza);
    let mut recursos: Vec<(&String, &pb::Existencia)> = plaza.almacen.iter().collect();
    recursos.sort_by_key(|(r, _)| (*r).clone());
    for (recurso, e) in recursos {
        if e.capacidad <= 0 || activas.contains(recurso) {
            continue;
        }
        let ahora = cantidad(e, t);
        let meta = metas.get(recurso).map_or(0, |m| m.0);
        // La tendencia: en escalones, la del minuto siguiente; si no, la del ritmo.
        let baja = e.cantidad.as_ref().map_or(0, |r| -r.por_minuto);
        let reserva = baja * 60 * HORAS_DE_RESERVA;
        let (es_compra, n) = if baja > 0 && ahora < reserva {
            (true, (reserva - ahora).min(e.capacidad - ahora))
        } else if ahora < meta {
            // Lo de su meta se compra antes que nada.
            (true, meta - ahora)
        } else if fraccion(e, t) >= EXCEDENTE {
            (false, (((ahora as f64 - e.capacidad as f64 * EXCEDENTE) * FRACCION_A_VENDER) as i64).min(ahora - meta))
        } else if fraccion(e, t) <= ESCASEZ {
            (true, (e.capacidad as f64 * ESCASEZ) as i64 - ahora)
        } else {
            continue;
        };
        // Lo pactado va en unidades enteras.
        let n = n / 1000 * 1000;
        if n <= 0 || !contrarias.iter().any(|(r, compra)| r == recurso && *compra != es_compra) {
            continue;
        }
        let tipo = if es_compra { "compra" } else { "venta" };
        let o = O::ColocarOrdenMercado(pb::OrdenColocarOrdenMercado { asentamiento_id: plaza.id.clone(), tipo: tipo.into(), recurso: recurso.clone(), cantidad: n, precio: None });
        let clave = format!("{tipo}:{}:{recurso}", plaza.id);
        // Una orden que caducó sin nadie al otro lado no se repone enseguida: el mercado no tenía con quién.
        if ctx.intentar(&clave, o, ESPERA_TRAS_RECHAZO_MS).await?.is_some_and(|r| r.resultado == pb::Resultado::OrdenAceptada as i32) {
            ctx.esperar(&clave, ESPERA_TRAS_MERCADO_MS);
            if es_compra {
                let yo = ctx.yo.to_string();
                ctx.con_pizarra(|p| p.tablero.tomar(&format!("comprar:{recurso}"), &yo, t, t + ESPERA_TRAS_MERCADO_MS));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_alcance_sale_de_los_viveres_que_quedan() {
        // 350 de víveres que bajan 1/min parados (2,5/min en marcha): 140 min de marcha; a 30 u/min, 140 × 30 / 2,4 = 1.750 u de ida.
        let v = pb::Ritmo { valor_en_t0: 350_000, t0_ms: 0, por_minuto: -1_000, ..Default::default() };
        assert!(alcanza(&v, 0, 2.5, 30.0, 1_700_000));
        assert!(!alcanza(&v, 0, 2.5, 30.0, 1_800_000));
        assert!(!alcanza(&v, 100 * 60_000, 2.5, 30.0, 1_300_000), "con 100 min ya gastados quedan 250: 100 min de marcha");
        assert!(alcanza(&pb::Ritmo::default(), 0, 2.5, 30.0, 9_999_999), "sin consumo, sin límite");
    }

    #[test]
    fn las_tres_personalidades_salen_y_no_cambian() {
        let todas: Vec<Personalidad> = (1..=9).map(|i| Personalidad::de(&format!("f-{i}"))).collect();
        assert_eq!(todas, (1..=9).map(|i| Personalidad::de(&format!("f-{i}"))).collect::<Vec<_>>());
        for p in [Personalidad::Expansionista, Personalidad::Mercantil, Personalidad::Militarista] {
            assert!(todas.contains(&p), "{p:?} no sale en f-1..f-9: {todas:?}");
        }
        assert!(Personalidad::Militarista.margen() < Personalidad::Expansionista.margen() && Personalidad::Expansionista.margen() < Personalidad::Mercantil.margen());
    }

    #[test]
    fn cada_tarea_del_tablero_lleva_a_su_salida() {
        assert_eq!(accion_de("obra:ob-2"), Some(Accion::Materiales));
        assert_eq!(accion_de("guarnecer:av-3"), Some(Accion::GuarnecerAvanzada));
        assert_eq!(accion_de("guarnecer:pu-7"), Some(Accion::GuarnecerPueblo));
        assert_eq!(accion_de("cazar"), Some(Accion::Cazar), "el paso de una tecnología que pide campamentos destruidos");
        assert_eq!(accion_de("buscador"), Some(Accion::BuscarEstano));
        assert_eq!(accion_de("comprar:trigo"), None, "lo del Tesorero no es de uno libre");
    }

    #[test]
    fn la_cantidad_sigue_su_ritmo_entre_minimo_y_capacidad() {
        let ritmo = |v, por_minuto| pb::Existencia { cantidad: Some(pb::Ritmo { valor_en_t0: v, t0_ms: 0, por_minuto, ..Default::default() }), capacidad: 100_000, ..Default::default() };
        assert_eq!(cantidad(&ritmo(50_000, -1_000), 10 * 60_000), 40_000);
        assert_eq!(cantidad(&ritmo(50_000, -1_000), 90 * 60_000), 0, "no baja de cero");
        assert_eq!(cantidad(&ritmo(90_000, 1_000), 60 * 60_000), 100_000, "ni pasa de la capacidad");
        assert_eq!(fraccion(&ritmo(25_000, 0), 0), 0.25);
    }
}

#[cfg(test)]
mod zona {
    use super::*;

    #[test]
    fn en_zona_mira_el_poligono_y_si_no_el_circulo() {
        let p = |x, y| Punto { x, y };
        let mut a = pb::Asentamiento { posicion: Some(p(0, 0)), radio_potencial: 10_000, ..Default::default() };
        assert!(en_zona(&a, &p(9_000, 0)) && !en_zona(&a, &p(11_000, 0)), "sin polígono, el círculo");
        a.zona = vec![p(-10_000, -10_000), p(5_000, -10_000), p(5_000, 10_000), p(-10_000, 10_000)];
        assert!(en_zona(&a, &p(4_000, 0)), "dentro del polígono");
        assert!(!en_zona(&a, &p(9_000, 0)), "fuera del polígono, aunque dentro del radio");
    }
}

#[cfg(test)]
mod cargos_por_puntos {
    use super::*;

    #[test]
    fn el_cargo_es_del_que_mas_puntos_tiene_y_solo_cambia_con_margen() {
        let mut p = crate::cerebro::Pizarra::default();
        p.puntos.insert("h1".into(), [("guerra".to_string(), 40u64)].into());
        p.puntos.insert("h2".into(), [("guerra".to_string(), 45u64), ("obras".to_string(), 30)].into());
        let r = vec!["h1".to_string(), "h2".to_string(), "h3".to_string()];
        assert_eq!(mejor_para_el_cargo(&p, &r, Some("guerra"), &[]).as_deref(), Some("h2"));
        assert_eq!(mejor_para_el_cargo(&p, &r, Some("guerra"), &["h2".into()]).as_deref(), Some("h1"), "sin repetir");
        assert!(!supera(&p, "h2", "h1", Some("guerra")), "45 no pasa de 40 × 1,25");
        assert!(supera(&p, "h2", "h3", Some("guerra")) && !supera(&p, "h3", "h1", Some("guerra")));
    }
}

#[cfg(test)]
mod tecnologias_y_tropas {
    use super::*;
    use serde_json::json;

    /// Unos datos de juego con lo justo: la milicia, los honderos y los escaramuzadores; el barracón que sube con la carpintería, la galería y la fundición; la aleación del bronce.
    fn datos() -> Datos {
        let tropa = |id: &str, edificio: &str, nivel: u32, escalon: u32, poder: u32, equipo: serde_json::Value| {
            json!({"id": id, "tecnologia": "hostigamiento_tribal", "edificio": edificio, "nivelRequerido": nivel, "costoEquipo": equipo, "poderBase": poder, "velocidad": 30, "escalon": escalon, "unidadesPorDefecto": 25})
        };
        let v = json!({
            "derivados": {"costo_refundacion": {}, "costo_caravana_fundacion": {}},
            "reglas": {"territorio": {"separacion_plazas": 1, "exclusion_campamento": 1, "radio_inicial": 1, "techos": [1]}},
            "documentos": {
                "Heroe_Y_Tropa": {
                    "TROPAS_RECLUTABLES": [
                        tropa("milicia_lanceros", "centroUrbano", 1, 2, 2, json!({"madera": 2})),
                        tropa("honderos", "galeriaDeTiro", 1, 2, 5, json!({"armaMadera": 1})),
                        tropa("escaramuzadores_jabalina", "galeriaDeTiro", 2, 3, 8, json!({"armaCobre": 1})),
                    ],
                    "RECLUTAMIENTO_ORO_POR_ESCALON": {"2": 2, "3": 4}, "ORO_POR_CABALLO": 5,
                    "LIDERAZGO": {"base": 100, "costePorEscalon": {"2": 14, "3": 22}}
                },
                "Movimiento_Ejercitos": {"MOVIMIENTO": {"radioPuerta": 1, "radioInspeccion": 1}, "LOGISTICA": {"radioEncuentro": 1}},
                "Generacion_Del_Mundo": {"CAMPAMENTOS_BANDIDOS": {"anillo": {"radioMin": 1, "radioMax": 2}}},
                "Economia_Construccion": {
                    "EDIFICIO_CATALOGO": {
                        "galeriaDeTiro": {"requisitoNivelAsentamientoConstruccion": 2, "niveles": {"1": {}, "2": {"requisitoNivelAsentamiento": 2}, "3": {"requisitoNivelAsentamiento": 3}}},
                        "barracon": {"requisitoNivelAsentamientoConstruccion": 2, "niveles": {"1": {}, "2": {"requisitoNivelAsentamiento": 2, "requiereEdificio": "carpinteria"}}},
                        "carpinteria": {"requisitoNivelAsentamientoConstruccion": 2, "niveles": {"1": {}}},
                        "caballerizas": {"requiereTecnologia": "cria_caballar", "requisitoNivelAsentamientoConstruccion": 2, "niveles": {"1": {}}},
                        "fundicion": {"requisitoNivelAsentamientoConstruccion": 2, "niveles": {"1": {}, "2": {"requisitoNivelAsentamiento": 2}}}
                    },
                    "NIVEL_ASENTAMIENTO": {"requisitos": {}}
                },
                "Tecnologia_Aedas": {
                    "ERAS": {"reinos_palaciales": {"orden": 1}, "crisis_adaptacion": {"orden": 2}},
                    "TARIFA_ADOPCION": {"reinos_palaciales": {"oro": 100, "madera": 200}},
                    "TECNOLOGIAS": {
                        "metalurgia_cobre": {"nombre": "Metalurgia del cobre", "era": "reinos_palaciales", "logro": {"contador": "extraido.cobre", "porPlaza": 1000}, "hito": [{"tipo": "edificio", "edificio": "fundicion"}]},
                        "aleacion_bronce": {"nombre": "Aleación del bronce", "era": "reinos_palaciales",
                            "logro": {"contador": "extraido.cobre", "porPlaza": 2000, "ademas": {"contador": "extraido.estano", "porPlaza": 1000}},
                            "hito": [{"tipo": "edificio", "edificio": "fundicion", "nivelInterno": 2}, {"tipo": "recursoEnCapital", "recurso": "estano"}]},
                        "carpinteria_militar": {"nombre": "Carpintería militar", "era": "crisis_adaptacion", "logro": {"contador": "conquistas.conMurallaCompleta", "umbral": 1}, "hito": []}
                    }
                }
            }
        });
        Datos::desde_json(&v).expect("datos de prueba")
    }

    fn plaza(nivel: u32, edificios: &[(&str, u32)], almacen: &[(&str, i64)]) -> pb::Asentamiento {
        pb::Asentamiento {
            id: "p-1".into(),
            nivel_actual: nivel,
            edificios: edificios
                .iter()
                .enumerate()
                .map(|(i, (tipo, n))| pb::Edificio { id: format!("e-{i}"), tipo: tipo.to_string(), nivel_interno: Some(*n), estado: EstadoEdificio::EdificioActivo as i32, ..Default::default() })
                .collect(),
            almacen: almacen
                .iter()
                .map(|(r, n)| (r.to_string(), pb::Existencia { cantidad: Some(pb::Ritmo { valor_en_t0: n * 1000, ..Default::default() }), capacidad: 1_000_000_000, ..Default::default() }))
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn el_catalogo_se_lee_con_su_era_su_hito_su_logro_y_la_tarifa() {
        let d = datos();
        let a = d.tecnologias.iter().find(|x| x.id == "aleacion_bronce").unwrap();
        assert_eq!(a.hito, vec![Hito::Edificio("fundicion".into(), 2), Hito::EnCapital("estano".into())]);
        assert_eq!(a.logro, vec![("extraido.cobre".into(), 2000, true), ("extraido.estano".into(), 1000, true)]);
        assert_eq!(d.tecnologias.last().unwrap().id, "carpinteria_militar", "las de la Era II, detrás");
        assert_eq!(d.tarifas[&1], vec![("madera".to_string(), 200), ("oro".to_string(), 100)]);
        let h = d.tropas.iter().find(|x| x.id == "honderos").unwrap();
        assert_eq!((h.oro, h.equipo.clone(), h.unidades), (2, vec![("armaMadera".to_string(), 1)], 25));
        assert_eq!(d.tropas.iter().find(|x| x.id == "milicia_lanceros").unwrap().oro, 0, "la milicia del Centro Urbano no paga oro");
    }

    #[test]
    fn para_un_edificio_a_un_nivel_levanta_o_mejora_y_antes_lo_que_pide_la_mejora() {
        let d = datos();
        let pasos = |p: &pb::Asentamiento, tipo: &str, n: u32| {
            let mut v = Vec::new();
            pasos_para(&d, p, &[], tipo, n, &mut v);
            v
        };
        let p = plaza(2, &[("fundicion", 1), ("barracon", 1)], &[]);
        assert_eq!(pasos(&p, "fundicion", 2), vec![Paso::Mejorar("e-0".into(), "fundicion".into(), 2)], "la aleación del bronce pide la fundición a 2");
        assert_eq!(pasos(&p, "barracon", 2), vec![Paso::Levantar("carpinteria".into())], "el barracón 2 pide antes la carpintería");
        assert!(pasos(&p, "fundicion", 1).is_empty(), "ya la tiene");
        assert_eq!(pasos(&p, "caballerizas", 1), vec![Paso::Falta("cria_caballar para levantar caballerizas".into())]);
        assert_eq!(pasos(&plaza(1, &[], &[]), "galeriaDeTiro", 1), vec![Paso::Falta("la plaza a nivel 2 para levantar galeriaDeTiro".into())]);
        assert_eq!(pasos(&plaza(2, &[("galeriaDeTiro", 2)], &[]), "galeriaDeTiro", 3), vec![Paso::Falta("la plaza a nivel 3 para subir galeriaDeTiro a 3".into())]);
        let mut en_obra = plaza(2, &[("fundicion", 1)], &[]);
        en_obra.edificios[0].estado = EstadoEdificio::EdificioEnConstruccion as i32;
        assert!(pasos(&en_obra, "fundicion", 2).is_empty(), "en obra, espera");
    }

    #[test]
    fn la_tropa_siguiente_es_la_mas_cercana_por_encima_de_la_que_ya_paga() {
        let d = datos();
        let sabe = ["hostigamiento_tribal".to_string()];
        // Con madera para la milicia: la siguiente son los honderos, que piden la galería, sus armas de madera y su oro.
        let p = plaza(2, &[("centroUrbano", 1)], &[("madera", 100)]);
        let s = tropa_siguiente(&d, &p, &sabe, 0).unwrap();
        assert_eq!(s.tropa.id, "honderos");
        assert_eq!(s.pasos, vec![Paso::Levantar("galeriaDeTiro".into())]);
        assert_eq!((s.equipo.clone(), s.oro), (vec![("armaMadera".to_string(), 25_000)], 50_000), "25 armas de madera y 50 de oro para una escuadra entera");
        // Con la galería y oro, solo le faltan las armas: es lo que el tesorero guarda (y compra).
        let p = plaza(2, &[("centroUrbano", 1), ("galeriaDeTiro", 1)], &[("madera", 100), ("oro", 500)]);
        let s = tropa_siguiente(&d, &p, &sabe, 0).unwrap();
        assert_eq!((s.tropa.id.as_str(), s.pasos.len(), s.equipo.clone()), ("honderos", 0, vec![("armaMadera".to_string(), 25_000)]));
        // Con todo para los honderos, los escaramuzadores: la galería a nivel 2 (y sus armas de cobre).
        let p = plaza(2, &[("centroUrbano", 1), ("galeriaDeTiro", 1)], &[("madera", 100), ("oro", 500), ("armaMadera", 30)]);
        let s = tropa_siguiente(&d, &p, &sabe, 0).unwrap();
        assert_eq!((s.tropa.id.as_str(), s.pasos), ("escaramuzadores_jabalina", vec![Paso::Mejorar("e-1".into(), "galeriaDeTiro".into(), 2)]));
        // Sin la tecnología de los honderos, ninguna tropa por encima de la milicia... que tampoco sabe hacer: no hay siguiente.
        assert!(tropa_siguiente(&d, &plaza(2, &[("centroUrbano", 1)], &[("madera", 100)]), &[], 0).is_none());
    }

    #[test]
    fn sale_con_lo_mejor_que_cabe_en_su_liderazgo_y_lo_demas_es_para_guarniciones() {
        let d = datos();
        let e = |id: &str, tropa: &str, cantidad| pb::Escuadron { id: id.into(), tropa_id: tropa.into(), cantidad, ..Default::default() };
        // Poder: escaramuzadores 25 × 8 = 200, honderos 25 × 5 = 125, milicia 25 × 2 = 50; Liderazgo: 22, 14 y 14.
        let todas = [e("m", "milicia_lanceros", 25), e("h", "honderos", 25), e("x", "escaramuzadores_jabalina", 25)];
        assert_eq!(para_llevar(&d, &todas, 100), vec!["x", "h", "m"], "con 100 caben todas");
        assert_eq!(para_llevar(&d, &todas, 40), vec!["x", "h"], "con 40, la milicia se queda para la guarnición");
        assert_eq!(para_llevar(&d, &todas, 30), vec!["x"], "los honderos ya no caben con los escaramuzadores");
        assert_eq!(d.liderazgo, 100);
    }

    #[test]
    fn el_logro_pendiente_se_reconoce_por_su_umbral_de_ahora() {
        let d = datos();
        let tec = |id: &str| d.tecnologias.iter().find(|x| x.id == id).unwrap();
        let pendiente = |c: &str, umbral| pb::LogroPendiente { contador: c.into(), umbral };
        let mut e = pb::EstadoTecnologia { plazas: 3, ..Default::default() };
        e.contadores.insert("extraido.cobre".into(), 1200);
        e.logros_pendientes = vec![pendiente("extraido.cobre", 3000), pendiente("extraido.cobre", 6000), pendiente("extraido.estano", 3000)];
        assert_eq!(texto_del_logro(tec("metalurgia_cobre"), Some(&e)).as_deref(), Some("extraido.cobre 1200 de 3000"));
        assert_eq!(texto_del_logro(tec("aleacion_bronce"), Some(&e)).as_deref(), Some("extraido.cobre 1200 de 6000 y extraido.estano 0 de 3000"));
        e.logros_pendientes.remove(0);
        assert_eq!(texto_del_logro(tec("metalurgia_cobre"), Some(&e)), None, "cumplido el del cobre, solo le falta el hito");
    }
}
