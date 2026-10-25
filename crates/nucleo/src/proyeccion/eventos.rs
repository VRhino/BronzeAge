//! Los hechos del núcleo como `EventoDominio` del contrato 05 §8 (4H T5): el `payload` genérico de todos y el mensaje **tipado** de los que mueven la interfaz (decisión 8 del autor: los quince de
//! T5, desde 5H los ocho de la política y desde 6G los diez de la tecnología y los Aedas, que cubren sus quince códigos).
//!
//! - **A quién se atribuye**: `asentamiento_id` (la primera plaza), `heroes_ids` (los héroes nombrados) y `facciones_ids` (las Facciones que lo oyen enteras), lo que `EventoNucleo::completar` dejó al emitirlo.
//! - **`payload`**: `ejercitoId` (la primera columna que nombra), `x`/`y` (dónde ocurrió), los datos enteros y los valores de texto y lista. Los datos que nombran una entidad viajan **con el id del
//!   contrato** y la clave con `Id` (`heroe: 3` → `heroeId: "heroe-3"`, como en los tipados; decisión del autor, 2026-10-24, [`id_del_contrato`]); la batalla, como `battleId`.
//! - **`tipado`**, con el mismo contenido que el payload ya traducido a ids del contrato:
//!
//! | Mensaje | Códigos |
//! |---|---|
//! | `BatallaAbierta` | `batalla.abierta`, `evento_pve.abierta` |
//! | `BatallaAsignada` | `batalla.asignada`, `evento_pve.asignada` |
//! | `BatallaCerrada` | `batalla.aplicada`, `.cancelada`, `.fallida` (y sus `evento_pve.`) |
//! | `PlazaConquistada` | `asentamiento.conquistada` |
//! | `PlazaEnRuinas` | `asentamiento.ruinas` |
//! | `AsedioResistido` | `combate.asedio_resistido` |
//! | `UnionPedida` | `columna.union_pedida`, `convocatoria.union_pedida` |
//! | `ConvocatoriaConvocada`, `ConvocatoriaPartio` | `convocatoria.convocada`, `convocatoria.partio` |
//! | `HeroeExpulsado` | `heroe.expulsado`, `heroe.desalojado` |
//! | `ColumnaConHambre` | `ejercito.viveres_bajos`, `ejercito.sin_viveres` |
//! | `DesercionPorHambre` | `tropas.desercion` |
//! | `GuerraDeclarada` | `diplomacia.guerra_declarada` (con las arrastradas, 5D) |
//! | `ObraTerminada` | `construccion.edificio_completado`, `construccion.mejora_completada`, `construccion.recinto_completado`, `pueblo.completado`, `avanzada.completada` |
//! | `HeroeHerido` | `heroe.herido` |
//! | `PropuestaDiplomatica` (5H) | `diplomacia.relacion_propuesta`, `.anexion_propuesta`, `.fusion_propuesta` |
//! | `PropuestaResuelta` (5H) | `diplomacia.relacion_aceptada`, `.relacion_rechazada`, `.relacion_retirada`, `.anexion_rechazada`, `.anexion_retirada`, `.fusion_rechazada`, `.fusion_retirada` |
//! | `RelacionTerminada` (5H) | `diplomacia.relacion_rota`, `.paz_firmada`, `.rebelion_vasallo`, `.vasallo_liberado` |
//! | `FaccionesUnidas` (5H) | `diplomacia.anexion`, `diplomacia.fusion` |
//! | `CargoAsignado` (5H) | `cargo.local_asignado`, `cargo.rey_asignado`, `cargo.embajador_asignado` |
//! | `TituloCambiado` (5H) | `titulo.nace`, `titulo.cambia_manos` |
//! | `SolicitudDeIngreso` (5H) | `faccion.solicitud_ingreso` |
//! | `CiudadaniaCambiada` (5H) | `faccion.creada`, `faccion.ciudadania_union`, `faccion.solicitud_denegada`, `faccion.abandonada` |
//! | `LogroCumplido` (6G) | `tecnologia.logro` (público) |
//! | `EraComenzada` (6G) | `era.comienza` (público) |
//! | `TecnologiaAparecida`, `TecnologiaAdoptada` (6G) | `tecnologia.aparece`, `tecnologia.adoptada` |
//! | `DescubrimientoCantado` (6G) | `aedas.canta_descubrimiento` (público) |
//! | `RevelacionDeAeda`, `CompraAAeda` (6G) | `aedas.revela`, `aedas.venta` |
//! | `ResidenteCambia` (6G) | `aedas.residente_llega`, `aedas.residente_se_va` |
//! | `EpicaCambia` (6G) | `aedas.epica_empieza`, `.epica_capitulo`, `.epica_tecnologia`, `.epica_cumplida` (pública, sin la tecnología), `.epica_abandonada` |
//! | `AlijoAbierto` (6G) | `alijo.abierto` |
//!
//! `BatallaAbierta` sale de los datos del propio hecho (contra qué se juega, quién la inició, los dos bandos, el plazo de la convocatoria), así que se tipa igual aunque la batalla ya se haya
//! retirado del estado al proyectarlo (una reanudación tardía); de su contexto, lo que el ticket no congela (los carros de una caravana, lo protegido de un pueblo) se lee del estado de ahora, como
//! en el ticket. `ObraTerminada.nivel`: el de la mejora o del recinto, el nivel interno de un edificio recién terminado y la etapa de una avanzada (1 Atalaya, 2 Fuerte, 3 Ciudadela); un pueblo no
//! tiene nivel. `HeroeExpulsado.motivo`: `puerta_cerrada`, `conquista`, `destruido` (lo destruye su dueño o un ataque), `ruina` (queda en ruinas o cae su plaza madre) o `desalojo`.

use super::{batalla, ids, punto};
use crate::estado::{Dato, EventoNucleo, SIN_BANDERA};
use crate::nucleo::Nucleo;
use asentamiento::recurso::TODOS as RECURSOS;
use combate::caravana::Modalidad;
use combate::ciclo::{CaravanaEnBatalla, Contexto, EstadoBatalla};
use contrato::v1 as pb;
use pb::evento_dominio::Tipado;
use prost_types::{value::Kind, ListValue, Struct, Value};

/// ¿Es un hecho de una batalla (`batalla.*` o, contra un campamento de bandidos, `evento_pve.*`)?
pub fn es_de_batalla(codigo: &str) -> bool {
    codigo.starts_with("batalla.") || codigo.starts_with("evento_pve.")
}

/// La batalla de un hecho de batalla (su dato `batalla`).
pub fn batalla_de(ev: &EventoNucleo) -> Option<u64> {
    ev.dato("batalla").filter(|_| es_de_batalla(&ev.codigo)).map(|b| b as u64)
}

/// El hecho como `EventoDominio` del contrato, con la versión que lo produjo.
pub fn evento_dominio(n: &Nucleo, ev: &EventoNucleo, version: u64) -> pb::EventoDominio {
    pb::EventoDominio {
        codigo: ev.codigo.to_string(),
        version,
        en_ms: ev.en,
        asentamiento_id: ev.asentamientos.first().map(|&id| ids::asentamiento(id)),
        heroes_ids: ev.heroes.iter().map(|&h| ids::heroe(h)).collect(),
        facciones_ids: ev.facciones.iter().map(|&f| ids::faccion(f)).collect(),
        payload: Some(payload(n, ev)),
        tipado: tipado(n, ev),
    }
}

fn numero(n: i64) -> Value {
    Value { kind: Some(Kind::NumberValue(n as f64)) }
}

fn texto(s: String) -> Value {
    Value { kind: Some(Kind::StringValue(s)) }
}

fn valor(d: &Dato) -> Value {
    match d {
        Dato::Numero(n) => numero(*n),
        Dato::Texto(s) => texto(s.clone()),
        Dato::Lista(v) => Value { kind: Some(Kind::ListValue(ListValue { values: v.iter().map(valor).collect() })) },
    }
}

/// El id del contrato de un dato entero que nombra una entidad (`None`: es un número que no nombra nada, o una marca, y viaja tal cual). Las claves son las de los hechos del núcleo; un
/// campamento es de bandidos en sus hechos ([`de_bandidos`]) y de mercenarios en los demás; un edificio y un recinto, de la plaza del hecho.
fn id_del_contrato(n: &Nucleo, ev: &EventoNucleo, clave: &str, v: i64) -> Option<String> {
    let (u, plaza) = (v as u64, ev.asentamientos.first().copied());
    let faccion = || id_de_faccion(v).unwrap_or_default();
    Some(match clave {
        "heroe" | "lider" | "sucesor" | "solicitante" | "observador" | "iniciador" => ids::heroe(u),
        "ejercito" | "ejercitoA" | "ejercitoB" | "columna" | "presa" => ids::ejercito(u),
        "caravana" | "caravanaB" => ids::caravana(u),
        "plaza" | "ciudad" | "destino" | "socia" | "asentamientoArruinado" => ids::asentamiento(u),
        "faccion" | "faccionA" | "faccionB" | "faccionAtacante" | "faccionDefensora" | "faccionAnterior" | "faccionVista" | "faccionQueVe" | "atacante" | "contra" | "por" => faccion(),
        "relacion" => ids::relacion(u),
        "aedaResidente" => ids::aeda_residente(v as u32),
        "aeda" => ids::aeda(v as u32),
        "descubridora" => faccion(),
        "propuesta" => ids::propuesta(u),
        "capital" => ids::asentamiento(u),
        "pueblo" => ids::pueblo(u),
        "avanzada" => ids::avanzada(u),
        "convocatoria" => ids::convocatoria(u),
        "acuerdo" => ids::acuerdo(u),
        // El `orden` de una Era (`era.comienza` y su entrada de crónica) es un número, no una orden de mercado.
        "orden" if !matches!(ev.codigo.as_ref(), "era.comienza" | "cronica.entrada") => ids::orden_de_mercado(u),
        "escuadra" => ids::escuadra(u),
        "nodo" => ids::nodo(v as u32),
        "campamento" if de_bandidos(&ev.codigo) => ids::bandidos(u),
        "campamento" => ids::campamento(v as u32),
        "edificio" => ids::edificio(plaza?, v as u32),
        "recinto" => ids::recinto(plaza?, v as u32),
        "parcela" => ids::parcela(v as u32),
        // El lugar del que sale un héroe expulsado: su clase va en `lugar` (0 plaza, 1 pueblo, 2 avanzada).
        "sitio" if ev.codigo == "heroe.expulsado" => lugar_expulsado(ev, u)?,
        "batalla" => return Some(n.uuid_de_batalla(u)),
        _ => return None,
    })
}

/// El id del contrato de una Facción en un dato: ni 0 ni `SIN_BANDERA` (los bandidos) nombran una.
fn id_de_faccion(v: i64) -> Option<String> {
    u32::try_from(v).ok().filter(|&f| f > 0 && f != SIN_BANDERA).map(ids::faccion)
}

/// El lugar de un `heroe.expulsado` (`sitio`, con su clase en `lugar`).
fn lugar_expulsado(ev: &EventoNucleo, id: u64) -> Option<String> {
    Some(match ev.dato("lugar")? {
        0 => ids::asentamiento(id),
        1 => ids::pueblo(id),
        _ => ids::avanzada(id),
    })
}

/// El contexto de una batalla tal como lo cuenta su hecho de apertura (`contexto` y lo que añade `objetivo_de_contexto` en el núcleo).
fn contexto_del_hecho(ev: &EventoNucleo) -> Option<Contexto> {
    let (punto, dato) = (ev.punto?, |k: &str| ev.dato(k));
    Some(match dato("contexto")? {
        0 => Contexto::Asedio { asentamiento: u32::try_from(dato("plaza")?).ok()? },
        1 => Contexto::CampoAbierto { punto, solitarios: dato("solitarios")? == 1 },
        2 => {
            let Some(Dato::Lista(lista)) = ev.valores.iter().find(|(k, _)| *k == "caravanasIds").map(|(_, d)| d) else { return None };
            let caravanas = lista.iter().filter_map(|d| if let Dato::Texto(s) = d { ids::parse_caravana(s) } else { None }).map(|id| CaravanaEnBatalla { id, carros: 0 }).collect();
            Contexto::Caravana { caravanas, punto, modalidad: if dato("modalidad")? == 1 { Modalidad::Aguante } else { Modalidad::Disputa } }
        }
        3 => Contexto::CampamentoBandidos { campamento: dato("campamento")?.try_into().ok()?, punto },
        4 => Contexto::SaqueoPueblo { pueblo: dato("pueblo")? as u64, punto },
        _ => Contexto::Avanzada { avanzada: dato("avanzada")? as u64, etapa: *asentamiento::avanzadas::Etapa::TODAS.get(usize::try_from(dato("etapa")?).ok()?)?, punto },
    })
}

/// ¿Es un hecho de un campamento de bandidos (su aparición, su ataque, su batalla)?
fn de_bandidos(codigo: &str) -> bool {
    codigo.starts_with("bandidos.") || codigo.starts_with("evento_pve.") || matches!(codigo, "combate.campamento_destruido" | "combate.ataque_campamento_fallido")
}

fn payload(n: &Nucleo, ev: &EventoNucleo) -> Struct {
    let mut campos = std::collections::BTreeMap::new();
    if let Some(&id) = ev.ejercitos.first() {
        campos.insert("ejercitoId".to_owned(), texto(ids::ejercito(id)));
    }
    if let Some(p) = ev.punto {
        campos.insert("x".to_owned(), numero(p.x));
        campos.insert("y".to_owned(), numero(p.y));
    }
    for &(k, v) in &ev.datos {
        match id_del_contrato(n, ev, k, v) {
            Some(id) if k == "batalla" => campos.insert("battleId".to_owned(), texto(id)),
            Some(id) => campos.insert(format!("{k}Id"), texto(id)),
            None => campos.insert(k.to_owned(), numero(v)),
        };
    }
    campos.extend(ev.valores.iter().map(|(k, d)| ((*k).to_owned(), valor(d))));
    Struct { fields: campos }
}

/// La clase de propuesta de un hecho de diplomacia (`diplomacia.<clase>_...`).
fn clase_de_propuesta(codigo: &str) -> &'static str {
    if codigo.starts_with("diplomacia.anexion") {
        "anexion"
    } else if codigo.starts_with("diplomacia.fusion") {
        "fusion"
    } else {
        "relacion"
    }
}

/// Un valor de texto del hecho.
fn texto_de(ev: &EventoNucleo, k: &str) -> Option<String> {
    ev.valores.iter().find_map(|(c, d)| if let Dato::Texto(s) = d { (*c == k).then(|| s.clone()) } else { None })
}

/// El mensaje tipado de los hechos que mueven la interfaz (ver la tabla del módulo); `None` en el resto.
fn tipado(n: &Nucleo, ev: &EventoNucleo) -> Option<Tipado> {
    let dato = |k: &str| ev.dato(k);
    let id = |k: &str| dato(k).map(|v| v as u64);
    let faccion = |k: &str| dato(k).and_then(id_de_faccion);
    let heroe = |k: &str| id(k).map(ids::heroe).unwrap_or_default();
    let plaza = ev.asentamientos.first().copied();
    let plaza_id = || plaza.map(ids::asentamiento).unwrap_or_default();
    let columna = || ev.ejercitos.first().map(|&e| ids::ejercito(e)).unwrap_or_default();
    let battle_id = || batalla_de(ev).map(|b| n.uuid_de_batalla(b)).unwrap_or_default();
    let codigo = ev.codigo.as_ref();
    // El nombre sin el prefijo de batalla: `batalla.abierta` y `evento_pve.abierta` se tipan igual.
    let de_batalla = es_de_batalla(codigo).then(|| codigo.split_once('.').map_or(codigo, |(_, nombre)| nombre));
    let obra = |clase: &str, objeto: String, nivel: Option<u32>| Tipado::ObraTerminada(pb::ObraTerminada { asentamiento_id: plaza_id(), clase: clase.into(), objeto_id: objeto, nivel });
    Some(match (codigo, de_batalla) {
        (_, Some("abierta")) => Tipado::BatallaAbierta(pb::BatallaAbierta {
            battle_id: battle_id(),
            contexto: Some(batalla::contexto(n, &contexto_del_hecho(ev)?)),
            punto: ev.punto.map(punto),
            iniciada_por: id("iniciador").map(ids::heroe),
            atacante_faccion_id: faccion("faccionAtacante"),
            defensor_faccion_id: faccion("faccionDefensora"),
            convocatoria_hasta_ms: dato("convocatoriaHasta"),
        }),
        (_, Some("asignada")) => Tipado::BatallaAsignada(pb::BatallaAsignada { battle_id: battle_id() }),
        (_, Some(nombre @ ("aplicada" | "cancelada" | "fallida"))) => {
            let estado = match nombre {
                "aplicada" => pb::EstadoBatalla::BatallaAplicada,
                "cancelada" => pb::EstadoBatalla::BatallaCancelada,
                _ => pb::EstadoBatalla::BatallaFallida,
            };
            let ganador = dato("ganaElAtacante").map(|a| if a == 1 { pb::LadoId::Atacante } else { pb::LadoId::Defensor } as i32);
            let razon = batalla_de(ev).and_then(|b| n.batalla(b)).filter(|b| b.estado == EstadoBatalla::Aplicada).and_then(|b| b.resultado.as_ref()).map(|r| batalla::razon(r.razon) as i32);
            Tipado::BatallaCerrada(pb::BatallaCerrada { battle_id: battle_id(), estado: estado as i32, ganador, razon })
        }
        ("asentamiento.conquistada", _) => Tipado::PlazaConquistada(pb::PlazaConquistada { asentamiento_id: plaza_id(), atacante_faccion_id: faccion("faccion"), defensor_faccion_id: faccion("faccionAnterior") }),
        ("asentamiento.ruinas", _) => {
            let faltantes = dato("faltantes").unwrap_or(0);
            let recursos_faltantes = RECURSOS.iter().filter(|r| faltantes & (1 << crate::codec_economia::recurso_a_u8(**r)) != 0).map(|r| r.nombre().to_owned()).collect();
            Tipado::PlazaEnRuinas(pb::PlazaEnRuinas { asentamiento_id: plaza_id(), recursos_faltantes })
        }
        ("combate.asedio_resistido", _) => Tipado::AsedioResistido(pb::AsedioResistido { asentamiento_id: plaza_id(), atacante_faccion_id: faccion("faccionAtacante") }),
        ("columna.union_pedida", _) => Tipado::UnionPedida(pb::UnionPedida { destino_id: columna(), destino_es_convocatoria: false, solicitante_id: heroe("heroe"), expira_en_ms: dato("expiraEn")? }),
        ("convocatoria.union_pedida", _) => {
            Tipado::UnionPedida(pb::UnionPedida { destino_id: ids::convocatoria(id("convocatoria")?), destino_es_convocatoria: true, solicitante_id: heroe("heroe"), expira_en_ms: dato("expiraEn")? })
        }
        ("convocatoria.convocada", _) => {
            let campamento = dato("campamento").and_then(|c| u32::try_from(c).ok());
            let lugar_id = campamento.map_or_else(plaza_id, ids::campamento);
            Tipado::ConvocatoriaConvocada(pb::ConvocatoriaConvocada { convocatoria_id: ids::convocatoria(id("convocatoria")?), lugar_id, lugar_es_campamento: campamento.is_some(), lider_id: heroe("lider") })
        }
        ("convocatoria.partio", _) => Tipado::ConvocatoriaPartio(pb::ConvocatoriaPartio { convocatoria_id: ids::convocatoria(id("convocatoria")?), ejercito_id: columna() }),
        ("heroe.expulsado", _) => {
            // Los motivos de `nucleo::dentro`: 0 la puerta se cierra, 1 queda en ruinas o cae su plaza madre, 2 su plaza madre cambia de dueño, 3 se destruye.
            let motivo = match dato("motivo")? {
                0 => "puerta_cerrada",
                1 => "ruina",
                2 => "conquista",
                _ => "destruido",
            };
            Tipado::HeroeExpulsado(pb::HeroeExpulsado { heroe_id: heroe("heroe"), lugar_id: lugar_expulsado(ev, id("sitio")?)?, motivo: motivo.into(), ejercito_id: id("ejercito").map(ids::ejercito) })
        }
        ("heroe.desalojado", _) => Tipado::HeroeExpulsado(pb::HeroeExpulsado { heroe_id: heroe("heroe"), lugar_id: plaza_id(), motivo: "desalojo".into(), ejercito_id: None }),
        ("ejercito.viveres_bajos" | "ejercito.sin_viveres", _) => Tipado::ColumnaConHambre(pb::ColumnaConHambre { ejercito_id: columna(), sin_viveres: codigo == "ejercito.sin_viveres" }),
        ("tropas.desercion", _) => Tipado::DesercionPorHambre(pb::DesercionPorHambre { heroe_id: heroe("heroe"), escuadra_id: ids::escuadra(id("escuadra")?), desertores: u32::try_from(dato("desertores")?).ok()? }),
        ("diplomacia.guerra_declarada", _) => {
            // Las arrastradas por vasallaje viajan en `valores` como ids del contrato (5D).
            let arrastradas_ids = match ev.valores.iter().find(|(k, _)| *k == "arrastradas").map(|(_, d)| d) {
                Some(Dato::Lista(l)) => l.iter().filter_map(|d| if let Dato::Texto(s) = d { Some(s.clone()) } else { None }).collect(),
                _ => Vec::new(),
            };
            Tipado::GuerraDeclarada(pb::GuerraDeclarada { atacante_faccion_id: faccion("atacante")?, contra_faccion_id: faccion("contra")?, arrastradas_ids })
        }
        ("construccion.edificio_completado", _) => obra("edificio", ids::edificio(plaza?, u32::try_from(dato("edificio")?).ok()?), dato("nivel").and_then(|v| u32::try_from(v).ok())),
        ("construccion.mejora_completada", _) => obra("mejora", ids::edificio(plaza?, u32::try_from(dato("edificio")?).ok()?), dato("nivel").and_then(|v| u32::try_from(v).ok())),
        ("construccion.recinto_completado", _) => obra("recinto", ids::recinto(plaza?, u32::try_from(dato("recinto")?).ok()?), dato("nivel").and_then(|v| u32::try_from(v).ok())),
        // DUDA: un pueblo no tiene nivel (Economia_Construccion §14): su `ObraTerminada.nivel` va ausente.
        ("pueblo.completado", _) => obra("pueblo", ids::pueblo(id("pueblo")?), None),
        ("avanzada.completada", _) => obra("avanzada", ids::avanzada(id("avanzada")?), dato("etapa").and_then(|v| u32::try_from(v + 1).ok())),
        ("heroe.herido", _) => Tipado::HeroeHerido(pb::HeroeHerido { heroe_id: heroe("heroe"), hasta_ms: dato("hasta")? }),
        // La política (5H).
        ("diplomacia.relacion_propuesta" | "diplomacia.anexion_propuesta" | "diplomacia.fusion_propuesta", _) => Tipado::PropuestaDiplomatica(pb::PropuestaDiplomatica {
            propuesta_id: ids::propuesta(id("propuesta")?),
            clase: clase_de_propuesta(codigo).into(),
            faccion_a_id: faccion("faccionA")?,
            faccion_b_id: faccion("faccionB")?,
            propuesta_por: heroe("heroe"),
            tipo: (codigo == "diplomacia.relacion_propuesta").then(|| if dato("tipo") == Some(1) { pb::TipoRelacion::RelacionVasallaje } else { pb::TipoRelacion::RelacionAlianza } as i32),
            nombre: texto_de(ev, "nombre"),
        }),
        (
            "diplomacia.relacion_aceptada" | "diplomacia.relacion_rechazada" | "diplomacia.relacion_retirada" | "diplomacia.anexion_rechazada" | "diplomacia.anexion_retirada" | "diplomacia.fusion_rechazada"
            | "diplomacia.fusion_retirada",
            _,
        ) => Tipado::PropuestaResuelta(pb::PropuestaResuelta {
            propuesta_id: id("propuesta").map(ids::propuesta),
            clase: clase_de_propuesta(codigo).into(),
            resultado: codigo.rsplit('_').next().unwrap_or_default().into(),
            faccion_a_id: faccion("faccionA")?,
            faccion_b_id: faccion("faccionB")?,
            relacion_id: id("relacion").map(ids::relacion),
        }),
        ("diplomacia.relacion_rota" | "diplomacia.paz_firmada" | "diplomacia.rebelion_vasallo" | "diplomacia.vasallo_liberado", _) => Tipado::RelacionTerminada(pb::RelacionTerminada {
            relacion_id: ids::relacion(id("relacion")?),
            faccion_a_id: faccion("faccionA")?,
            faccion_b_id: faccion("faccionB")?,
            como: match codigo {
                "diplomacia.relacion_rota" => "rota",
                "diplomacia.paz_firmada" => "paz",
                "diplomacia.rebelion_vasallo" => "rebelion",
                _ => "liberacion",
            }
            .into(),
            por_faccion_id: faccion("faccion"),
        }),
        ("diplomacia.anexion" | "diplomacia.fusion", _) => {
            let fusion = codigo == "diplomacia.fusion";
            Tipado::FaccionesUnidas(pb::FaccionesUnidas {
                clase: if fusion { "fusion" } else { "anexion" }.into(),
                faccion_a_id: faccion("faccionA")?,
                faccion_b_id: faccion("faccionB")?,
                resultante_id: if fusion { faccion("faccion")? } else { faccion("faccionA")? },
                nombre: texto_de(ev, "nombre"),
            })
        }
        ("cargo.local_asignado" | "cargo.rey_asignado" | "cargo.embajador_asignado", _) => Tipado::CargoAsignado(pb::CargoAsignado {
            heroe_id: heroe("heroe"),
            cargo: match codigo {
                "cargo.rey_asignado" => "rey".to_owned(),
                "cargo.embajador_asignado" => "embajador".to_owned(),
                _ => texto_de(ev, "cargo")?,
            },
            faccion_id: faccion("faccion")?,
            asentamiento_id: (codigo == "cargo.local_asignado").then(plaza_id),
        }),
        ("titulo.nace" | "titulo.cambia_manos", _) => Tipado::TituloCambiado(pb::TituloCambiado { titulo_id: texto_de(ev, "titulo")?, faccion_id: faccion("faccion")?, faccion_anterior_id: faccion("faccionAnterior") }),
        ("faccion.solicitud_ingreso", _) => Tipado::SolicitudDeIngreso(pb::SolicitudDeIngreso { heroe_id: heroe("heroe"), faccion_id: faccion("faccion")? }),
        ("faccion.creada" | "faccion.ciudadania_union" | "faccion.solicitud_denegada" | "faccion.abandonada", _) => Tipado::CiudadaniaCambiada(pb::CiudadaniaCambiada {
            heroe_id: heroe("heroe"),
            faccion_id: faccion("faccion")?,
            como: match codigo {
                "faccion.creada" => "creada",
                "faccion.ciudadania_union" => "ingreso",
                "faccion.solicitud_denegada" => "denegada",
                _ => "abandono",
            }
            .into(),
        }),
        // La tecnología y los Aedas (6G).
        ("tecnologia.logro", _) => Tipado::LogroCumplido(pb::LogroCumplido { contador: texto_de(ev, "contador")?, umbral: dato("umbral")?, en_ms: ev.en }),
        ("era.comienza", _) => Tipado::EraComenzada(pb::EraComenzada { era_id: texto_de(ev, "era")?, orden: u32::try_from(dato("orden")?).ok()?, por_plazo: dato("porPlazo") == Some(1) }),
        ("tecnologia.aparece", _) => Tipado::TecnologiaAparecida(pb::TecnologiaAparecida { faccion_id: faccion("faccion")?, tecnologia_id: texto_de(ev, "tecnologia")?, primera: dato("primera") == Some(1) }),
        ("tecnologia.adoptada", _) => Tipado::TecnologiaAdoptada(pb::TecnologiaAdoptada { faccion_id: faccion("faccion")?, capital_id: ids::asentamiento(id("capital")?), tecnologia_id: texto_de(ev, "tecnologia")? }),
        ("aedas.canta_descubrimiento", _) => Tipado::DescubrimientoCantado(pb::DescubrimientoCantado { descubridora_faccion_id: faccion("faccion")?, tecnologia_id: texto_de(ev, "tecnologia")? }),
        ("aedas.revela", _) => {
            let faltan = match ev.valores.iter().find(|(k, _)| *k == "faltan").map(|(_, d)| d) {
                Some(Dato::Lista(l)) => l.iter().filter_map(|d| if let Dato::Numero(i) = d { u32::try_from(*i).ok() } else { None }).collect(),
                _ => Vec::new(),
            };
            Tipado::RevelacionDeAeda(pb::RevelacionDeAeda { faccion_id: faccion("faccion")?, asentamiento_id: plaza_id(), descubridora_faccion_id: faccion("descubridora")?, tecnologia_id: texto_de(ev, "tecnologia")?, faltan })
        }
        ("aedas.venta", _) => Tipado::CompraAAeda(pb::CompraAAeda { faccion_id: faccion("faccion")?, asentamiento_id: plaza_id(), aeda_id: ids::aeda(u32::try_from(dato("aeda")?).ok()?), tecnologia_id: texto_de(ev, "tecnologia")?, precio: dato("precio")? }),
        ("aedas.residente_llega" | "aedas.residente_se_va", _) => Tipado::ResidenteCambia(pb::ResidenteCambia {
            aeda_id: ids::aeda_residente(u32::try_from(dato("aedaResidente")?).ok()?),
            faccion_id: faccion("faccion")?,
            asentamiento_id: plaza_id(),
            llega: codigo == "aedas.residente_llega",
            cupo: dato("cupo").and_then(|c| u32::try_from(c).ok()),
        }),
        ("aedas.epica_empieza" | "aedas.epica_capitulo" | "aedas.epica_tecnologia" | "aedas.epica_cumplida" | "aedas.epica_abandonada", _) => Tipado::EpicaCambia(pb::EpicaCambia {
            aeda_id: ids::aeda_residente(u32::try_from(dato("aedaResidente")?).ok()?),
            faccion_id: faccion("faccion")?,
            // La de la plaza del hecho o, la perdida por conquista o por aparecer, la de su dato `plaza` (el hecho va a la Facción de antes, no a la plaza).
            asentamiento_id: plaza.or_else(|| id("plaza")).map(ids::asentamiento),
            tecnologia_id: texto_de(ev, "tecnologia"),
            como: codigo.rsplit('_').next().unwrap_or_default().into(),
            capitulo: dato("capitulo").and_then(|c| u32::try_from(c).ok()).unwrap_or(0),
            capitulos: dato("capitulos").and_then(|c| u32::try_from(c).ok()).unwrap_or(0),
            motivo: texto_de(ev, "motivo"),
        }),
        ("alijo.abierto", _) => Tipado::AlijoAbierto(pb::AlijoAbierto { heroe_id: heroe("heroe"), alijo_id: texto_de(ev, "alijo")?, oro: dato("oro")?, cargado: dato("cargado")? }),
        _ => return None,
    })
}
