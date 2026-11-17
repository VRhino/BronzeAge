//! Lo que una Facción de bots hace con su libro de agravios: apunta los hechos que oye y, una vez por hora de mundo, repasa el roce con sus vecinas, quién domina y cuál es su objetivo.
//! Las reglas puras están en [`crate::libro`]; lo que cada bot hace con el resultado, en `cerebro` y `plaza`.

use std::collections::BTreeMap;

use contrato::v1::{self as pb, entidad::Cuerpo, evento_dominio::Tipado, EventoDominio};

use crate::{
    azar::hash,
    balance as b,
    cerebro::{distancia, entidades, Tipos},
    libro::{rasgos, via_del_estano, vida_media, Asiento, Estado, Libro, Objetivo, PlanEstano, Rasgo, Via},
    plaza::{plazas_ajenas, Personalidad},
    runner::Contexto,
};

const HORA_MS: i64 = 3_600_000;
/// Cuántos hechos recuerda haber apuntado la Facción, porque los oyen varios de sus bots.
const HECHOS_VISTOS: usize = 2048;

pub(crate) fn vida<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>) -> i64 {
    ctx.con_pizarra(|p| vida_media(p.rey.as_deref()))
}

pub(crate) fn estado<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, otra: &str) -> Estado {
    let (t, v) = (ctx.ahora_ms, vida(ctx));
    ctx.con_pizarra(|p| p.libro.estado(otra, t, v))
}

pub(crate) fn nueva<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, otra: &str) -> bool {
    let t = ctx.ahora_ms;
    ctx.con_pizarra(|p| p.libro.nueva(otra, p.niveles.get(otra).copied().unwrap_or(1), t))
}

/// ¿Se la puede acosar? Hostilidad o más, y nunca a una Facción nueva.
pub(crate) fn acosable<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, otra: &str) -> bool {
    !otra.is_empty() && estado(ctx, otra) <= Estado::Hostilidad && !nueva(ctx, otra)
}

/// ¿Se la ataca a la vista? Solo con odio, y nunca a una Facción nueva: un héroe sin Facción no tiene página.
pub(crate) fn a_la_vista<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, otra: &str) -> bool {
    !otra.is_empty() && estado(ctx, otra) == Estado::Odio && !nueva(ctx, otra)
}

/// La plaza que su objetivo quiere recuperar, o la de su enemigo jurado más cercana.
pub(crate) fn es_objetivo<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>, plaza: &str, faccion: &str) -> bool {
    ctx.con_pizarra(|p| match p.libro.objetivo.as_ref().map(|(o, _)| o) {
        Some(Objetivo::Recuperar { plaza: x, .. } | Objetivo::Estano { plaza: x, .. }) => x == plaza,
        Some(Objetivo::Enemigo { faccion: f }) => f == faccion,
        None => false,
    })
}

/// La Facción que su objetivo señala, para orientar avanzadas y exploradores.
pub(crate) fn faccion_objetivo<C: Tipos + ?Sized>(ctx: &Contexto<'_, C>) -> Option<String> {
    ctx.con_pizarra(|p| p.libro.objetivo.as_ref().map(|(o, _)| o.faccion().to_string()))
}

/// Mira a quién ve y apunta en el libro los hechos que oyó. Los oyen varios bots de la Facción: la pizarra recuerda los ya apuntados.
pub(crate) fn apuntar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>) {
    let Some(mia) = ctx.faccion() else { return };
    let t = ctx.ahora_ms;
    let (mut ciudadanos, mut niveles, mut rey) = (BTreeMap::new(), Vec::new(), None);
    for c in entidades(ctx.puerto.observar()) {
        if let Cuerpo::Faccion(f) = c {
            niveles.push((f.id.clone(), f.nivel));
            ciudadanos.extend(f.ciudadanos_ids.iter().map(|h| (h.clone(), f.id.clone())));
            if f.id == mia {
                rey = f.rey_id.clone();
            }
        }
    }
    let ajenas: BTreeMap<String, String> = plazas_ajenas(ctx, &mia).into_iter().map(|(id, f, _)| (id, f)).collect();
    let mias: Vec<String> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Asentamiento(a) if a.faccion_id == mia => Some(a.id.clone()),
            _ => None,
        })
        .collect();
    // Con quién tiene cada trueque en marcha: al cerrarse, el hecho solo trae su número.
    let acuerdos: Vec<(String, String)> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::AcuerdoTrueque(x) if x.estado == pb::EstadoTrueque::TruequeActivo as i32 => {
                let otra = if mias.contains(&x.asentamiento_a_id) { &x.asentamiento_b_id } else { &x.asentamiento_a_id };
                Some((numero(&x.id)?, ajenas.get(otra)?.clone()))
            }
            _ => None,
        })
        .collect();
    let v = ctx.con_pizarra(|p| {
        p.rey = rey.clone();
        p.acuerdos.extend(acuerdos);
        for (f, nivel) in niveles {
            p.libro.vistas.entry(f.clone()).or_insert(t);
            p.niveles.insert(f, nivel);
        }
        for f in ajenas.values() {
            p.libro.vistas.entry(f.clone()).or_insert(t);
        }
        vida_media(rey.as_deref())
    });
    let hechos = ctx.hechos.clone();
    ctx.con_pizarra(|p| {
        for h in &hechos {
            let clave = hash(&format!("{}:{}:{:?}:{:?}:{:?}", h.codigo, h.en_ms, h.asentamiento_id, h.tipado, h.payload));
            if p.vistos.contains(&clave) {
                continue;
            }
            if p.vistos.len() == HECHOS_VISTOS {
                p.vistos.pop_front();
            }
            p.vistos.push_back(clave);
            leer(&mut p.libro, &mut p.acuerdos, &mut p.por_renombrar, h, &mia, &mias, &ciudadanos, v);
        }
    });
}

fn numero(id: &str) -> Option<String> {
    let n: String = id.chars().rev().take_while(char::is_ascii_digit).collect();
    (!n.is_empty()).then(|| n.chars().rev().collect())
}

/// El número de un dato del hecho.
fn dato(h: &EventoDominio, k: &str) -> Option<String> {
    match h.payload.as_ref()?.fields.get(k)?.kind.as_ref()? {
        prost_types::value::Kind::NumberValue(n) => Some((*n as i64).to_string()),
        _ => None,
    }
}

/// Lo que un hecho significa para el libro.
#[allow(clippy::too_many_arguments)]
fn leer(l: &mut Libro, acuerdos: &mut BTreeMap<String, String>, por_renombrar: &mut Vec<String>, h: &EventoDominio, mia: &str, mias: &[String], ciudadanos: &BTreeMap<String, String>, v: i64) {
    let t = h.en_ms;
    // Quién lo hizo, en los hechos de combate que no dan la Facción: la primera de los héroes nombrados que no es la suya.
    let autor = || h.heroes_ids.iter().filter_map(|x| ciudadanos.get(x)).find(|f| *f != mia).cloned();
    match &h.tipado {
        Some(Tipado::PlazaConquistada(x)) => {
            let (Some(a), d) = (x.atacante_faccion_id.as_deref(), x.defensor_faccion_id.as_deref()) else { return };
            if d == Some(mia) {
                anotar(l, a, Asiento::PlazaConquistada, t, v);
                l.plazas_perdidas.insert(x.asentamiento_id.clone(), (a.to_string(), t));
            } else if a == mia {
                if l.plazas_perdidas.remove(&x.asentamiento_id).is_some() {
                    por_renombrar.push(x.asentamiento_id.clone());
                }
            } else if d.is_some_and(|d| l.estado(d, t, v) <= Estado::Recelo) {
                // Golpeó a alguien a quien no quiere bien.
                anotar(l, a, Asiento::EnemigoComun, t, v);
            } else if d.is_some_and(|d| l.estado(d, t, v) >= Estado::Amistad) {
                anotar(l, a, Asiento::GuerraDeclarada, t, v);
            }
        }
        Some(Tipado::AsedioResistido(x)) if mias.contains(&x.asentamiento_id) => {
            if let Some(a) = &x.atacante_faccion_id {
                anotar(l, a, Asiento::PlazaSitiada, t, v);
                l.pagina(a).confianza_hasta = t + b::CICATRIZ_ASEDIO_MS;
            }
        }
        Some(Tipado::GuerraDeclarada(x)) => {
            if x.contra_faccion_id == mia {
                anotar(l, &x.atacante_faccion_id, Asiento::GuerraDeclarada, t, v);
            } else if x.atacante_faccion_id != mia && l.estado(&x.contra_faccion_id, t, v) <= Estado::Recelo {
                anotar(l, &x.atacante_faccion_id, Asiento::EnemigoComun, t, v);
            }
        }
        Some(Tipado::RelacionTerminada(x)) if matches!(x.como.as_str(), "rota" | "rebelion") && (x.faccion_a_id == mia || x.faccion_b_id == mia) => {
            let otra = if x.faccion_a_id == mia { &x.faccion_b_id } else { &x.faccion_a_id };
            if x.por_faccion_id.as_deref() != Some(mia) {
                anotar(l, otra, Asiento::AlianzaRota, t, v);
                l.pagina(otra).traicion = true;
                l.traicionada = true;
            }
        }
        _ => match h.codigo.as_str() {
            "combate.pueblo_saqueado" => {
                if let Some(f) = autor() {
                    anotar(l, &f, Asiento::PuebloSaqueado, t, v);
                    l.pueblos_hasta = t + b::CICATRIZ_SAQUEO_MS;
                }
            }
            "combate.caravana_interceptada" => {
                if let Some(f) = autor() {
                    anotar(l, &f, Asiento::CaravanaAsaltada, t, v);
                }
                l.escoltas_hasta = t + b::CICATRIZ_CARAVANAS_MS;
            }
            // Los bandidos no tienen página, pero la cicatriz es la misma.
            "bandidos.caravana_interceptada" => l.escoltas_hasta = t + b::CICATRIZ_CARAVANAS_MS,
            "comercio.trueque_cumplido" | "comercio.trueque_expirado" => {
                let Some(f) = dato(h, "acuerdo").and_then(|a| acuerdos.remove(&a)) else { return };
                anotar(l, &f, if h.codigo.ends_with("cumplido") { Asiento::TruequeCumplido } else { Asiento::TruequeIncumplido }, t, v);
            }
            _ => {}
        },
    }
}

fn anotar(l: &mut Libro, f: &str, a: Asiento, t: i64, v: i64) {
    l.pagina(f).anotar(a, a.peso(), t, v);
}

/// El repaso de cada hora de mundo. Lo hace el primer bot de la Facción que piensa.
pub(crate) fn repasar<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>) {
    let Some(mia) = ctx.faccion() else { return };
    let t = ctx.ahora_ms;
    if ctx.con_pizarra(|p| t - p.repasado_en < HORA_MS) {
        return;
    }
    let horas = ctx.con_pizarra(|p| {
        let h = if p.repasado_en == 0 { 1.0 } else { (t - p.repasado_en) as f64 / HORA_MS as f64 };
        p.repasado_en = t;
        h.min(24.0)
    });
    // Un hecho propio no le abre página: nadie se guarda rencor a sí mismo.
    ctx.con_pizarra(|p| p.libro.paginas.remove(&mia));
    let ajenas = plazas_ajenas(ctx, &mia);
    let propias: Vec<pb::Punto> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Asentamiento(a) if a.faccion_id == mia => a.posicion.clone(),
            _ => None,
        })
        .collect();
    let v = vida(ctx);
    let roce = match Personalidad::de(&mia) {
        Personalidad::Expansionista => 1.0,
        Personalidad::Mercantil => 0.5,
        Personalidad::Militarista => 2.0,
    };
    let mut vecinas: Vec<&str> = ajenas.iter().filter(|(_, _, p)| propias.iter().any(|q| distancia(p, q) <= b::DISTANCIA_FRONTERA)).map(|(_, f, _)| f.as_str()).collect();
    vecinas.sort_unstable();
    vecinas.dedup();
    // Quién domina, con lo que la Facción conoce del mapa.
    let mut cuenta: BTreeMap<&str, usize> = BTreeMap::new();
    ajenas.iter().for_each(|(_, f, _)| *cuenta.entry(f).or_default() += 1);
    *cuenta.entry(&mia).or_default() += propias.len();
    let media = cuenta.values().sum::<usize>() as f64 / cuenta.len().max(1) as f64;
    let lider = cuenta.iter().max_by_key(|(f, n)| (**n, std::cmp::Reverse(**f))).filter(|(_, n)| **n >= b::DOMINIO_PLAZAS_MIN && **n as f64 >= b::DOMINIO_VECES * media).map(|(f, _)| *f);
    ctx.con_pizarra(|p| {
        for f in &vecinas {
            p.libro.pagina(f).anotar(Asiento::Frontera, Asiento::Frontera.peso() * roce * horas, t, v);
        }
        if let Some(l) = lider.filter(|l| *l != mia) {
            p.libro.pagina(l).anotar(Asiento::Dominio, Asiento::Dominio.peso() * horas, t, v);
            // Lo que une a quienes temen al mismo.
            for f in cuenta.keys().filter(|f| **f != l && **f != mia) {
                p.libro.pagina(f).anotar(Asiento::EnemigoComun, b::UNION_CONTRA_DOMINIO_POR_HORA * horas, t, v);
            }
        }
    });
    planear_estano(ctx, &mia, &ajenas, &propias, v);
    elegir_objetivo(ctx, &mia, v);
}

/// El estaño: en cuanto una plaza llega al nivel que lo pide y la Facción no lo extrae, decide cómo conseguirlo del yacimiento conocido más cercano. Solo cuenta lo que han visto
/// sus bots. Sin ninguno a la vista, los exploradores salen más a menudo.
fn planear_estano<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, mia: &str, ajenas: &[(String, String, pb::Punto)], propias: &[pb::Punto], v: i64) {
    let t = ctx.ahora_ms;
    let mias: Vec<(String, u32, pb::Punto, i64)> = entidades(ctx.puerto.observar())
        .filter_map(|c| match c {
            Cuerpo::Asentamiento(a) if a.faccion_id == mia => Some((a.id.clone(), a.nivel_actual, a.posicion.clone()?, a.radio_potencial)),
            _ => None,
        })
        .collect();
    let extrae = entidades(ctx.puerto.observar()).any(|c| matches!(c, Cuerpo::Pueblo(x) if x.tipo_extractor == "minaEstano" && mias.iter().any(|m| m.0 == x.asentamiento_madre_id)));
    if extrae || mias.iter().all(|m| m.1 < b::ESTANO_NIVEL_PLAZA) {
        ctx.con_pizarra(|p| p.libro.estano = None);
        return;
    }
    let punto = |x: &(i64, i64)| pb::Punto { x: x.0, y: x.1 };
    let de_casa = |sitio: &pb::Punto| mias.iter().map(|m| distancia(&m.2, sitio)).min().unwrap_or(i64::MAX);
    // El yacimiento: mejor uno sin dueña que uno que haya que disputar, y mejor cerca que lejos.
    let de_quien = |sitio: &pb::Punto| ajenas.iter().filter(|(_, _, p)| distancia(p, sitio) <= b::ZONA_DE_UNA_PLAZA).min_by_key(|(_, _, p)| distancia(p, sitio));
    let en_mi_zona = |sitio: &pb::Punto| mias.iter().any(|m| distancia(&m.2, sitio) <= m.3);
    let vistos = ctx.con_pizarra(|p| p.libro.estano_visto.clone());
    let Some((nodo, sitio)) = vistos.iter().min_by_key(|(id, x)| (!en_mi_zona(&punto(x)) && de_quien(&punto(x)).is_some(), de_casa(&punto(x)), (*id).clone())).map(|(id, x)| (id.clone(), punto(x))) else { return };
    let plaza_cerca = de_quien(&sitio);
    let peticiones = ctx.con_pizarra(|p| p.libro.estano.as_ref().filter(|x| x.nodo == nodo).map_or(0, |x| x.peticiones));
    let via = if en_mi_zona(&sitio) {
        Via::Propio
    } else {
        let belicoso = match Personalidad::de(mia) {
            Personalidad::Militarista => 2,
            Personalidad::Expansionista => 0,
            Personalidad::Mercantil => -2,
        };
        let rey = ctx.con_pizarra(|p| p.rey.as_deref().map(rasgos)).unwrap_or([Rasgo::Cauto, Rasgo::Cauto]);
        let dueno = plaza_cerca.map(|(_, f, _)| f.as_str());
        let suyas = dueno.map_or(0, |f| ajenas.iter().filter(|(_, x, _)| x == f).count());
        let estado = dueno.map(|f| (f, ctx.con_pizarra(|p| p.libro.estado(f, t, v))));
        via_del_estano(estado, belicoso, rey, propias.len(), suyas, peticiones)
    };
    ctx.con_pizarra(|p| {
        if let (Via::Guerra { faccion }, Some((plaza, _, _))) = (&via, plaza_cerca) {
            // La disputa enfría la relación hasta la guerra; a una Facción nueva se la respeta, tenga lo que tenga.
            if !p.libro.nueva(faccion, p.niveles.get(faccion).copied().unwrap_or(1), t) {
                let h = Asiento::DisputaEstano;
                p.libro.pagina(faccion).anotar(h, h.peso() / 24.0, t, v);
                if p.libro.objetivo.as_ref().is_none_or(|(o, _)| matches!(o, Objetivo::Enemigo { .. })) {
                    p.libro.objetivo = Some((Objetivo::Estano { plaza: plaza.clone(), faccion: faccion.clone() }, t));
                }
            }
        }
        p.libro.estano = Some(PlanEstano { nodo, via, peticiones });
    });
}

/// El objetivo de varios días: recuperar lo perdido y, si no, ir a por su enemigo jurado, que nace de un odio con agravios graves. Una Facción solo admite tantos enemigos jurados.
fn elegir_objetivo<C: Tipos + ?Sized>(ctx: &mut Contexto<'_, C>, mia: &str, v: i64) {
    let t = ctx.ahora_ms;
    let jurados = ctx.con_pizarra_en("facciones", |p| p.jurados.clone());
    let enemigo = ctx.con_pizarra(|p| {
        let l = &mut p.libro;
        // El rencor apagado deja de ser enemigo jurado.
        if l.enemigo.as_deref().is_some_and(|f| l.relacion(f, t, v) > b::UMBRAL_HOSTILIDAD) {
            l.enemigo = None;
        }
        if l.enemigo.is_none() {
            l.enemigo = l
                .paginas
                .iter()
                .filter(|(f, x)| x.relacion(t, v) <= b::UMBRAL_ODIO && x.agravio_grave(t, v) <= b::PESO_PUEBLO_SAQUEADO && jurados.iter().filter(|(de, a)| *de != mia && a == f).count() < b::FACCIONES_CONTRA_UNA)
                .min_by(|a, b| a.1.relacion(t, v).total_cmp(&b.1.relacion(t, v)))
                .map(|(f, _)| f.clone());
        }
        let con_estano = matches!(l.estano.as_ref().map(|e| &e.via), Some(Via::Guerra { .. }));
        if l.objetivo.as_ref().is_some_and(|(o, desde)| t - desde > b::OBJETIVO_MS || matches!(o, Objetivo::Recuperar { plaza, .. } if !l.plazas_perdidas.contains_key(plaza)) || (matches!(o, Objetivo::Estano { .. }) && !con_estano)) {
            l.objetivo = None;
        }
        l.plazas_perdidas.retain(|_, (_, en)| t - *en <= 2 * b::OBJETIVO_MS);
        if l.objetivo.is_none() {
            l.objetivo = match l.plazas_perdidas.iter().min_by_key(|(_, (_, en))| *en) {
                Some((plaza, (faccion, _))) => Some((Objetivo::Recuperar { plaza: plaza.clone(), faccion: faccion.clone() }, t)),
                None => l.enemigo.clone().map(|faccion| (Objetivo::Enemigo { faccion }, t)),
            };
        }
        l.enemigo.clone()
    });
    let mia = mia.to_string();
    ctx.con_pizarra_en("facciones", |p| match enemigo {
        Some(e) => {
            p.jurados.insert(mia, e);
        }
        None => {
            p.jurados.remove(&mia);
        }
    });
}
