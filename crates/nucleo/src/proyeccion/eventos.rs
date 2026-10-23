//! Los hechos del núcleo como `EventoDominio` del contrato 05 §8 (4H T5): el `payload` genérico de todos y el mensaje **tipado** de los quince que mueven la interfaz (decisión 8 del autor).
//!
//! - **A quién se atribuye**: `asentamiento_id` (la primera plaza), `heroes_ids` (los héroes nombrados) y `facciones_ids` (las Facciones que lo oyen enteras), lo que `EventoNucleo::completar` dejó al emitirlo.
//! - **`payload`**: `ejercitoId` (la primera columna que nombra), `x`/`y` (dónde ocurrió), los datos enteros tal cual (los ids del núcleo como números, como en el resto del núcleo), los valores de
//!   texto y lista y, en los hechos de batalla (`batalla.*` y `evento_pve.*`), `battleId` con el id de la batalla en el contrato.
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
//! | `GuerraDeclarada` | `faccion.guerra_declarada` |
//! | `ObraTerminada` | `construccion.edificio_completado`, `construccion.mejora_completada`, `construccion.recinto_completado`, `pueblo.completado`, `avanzada.completada` |
//! | `HeroeHerido` | `heroe.herido` |
//!
//! DUDA: `BatallaAbierta` lleva el contexto del ticket de la batalla: si la batalla ya se retiró del estado cuando se proyecta el hecho (una reanudación más de una hora después) el hecho viaja sin
//! tipado; `ObraTerminada` no lleva `nivel` para un edificio recién terminado, un pueblo ni una avanzada (el hecho no lo dice); `HeroeExpulsado.motivo` de un lugar que desaparece (destruido, en
//! ruinas o con la plaza madre en ruinas) es `ruina`.

use super::{batalla, ids, punto};
use crate::estado::{Dato, EventoNucleo};
use crate::nucleo::Nucleo;
use asentamiento::recurso::TODOS as RECURSOS;
use combate::ciclo::EstadoBatalla;
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

fn payload(n: &Nucleo, ev: &EventoNucleo) -> Struct {
    let mut campos = std::collections::BTreeMap::new();
    if let Some(&id) = ev.ejercitos.first() {
        campos.insert("ejercitoId".to_owned(), texto(ids::ejercito(id)));
    }
    if let Some(p) = ev.punto {
        campos.insert("x".to_owned(), numero(p.x));
        campos.insert("y".to_owned(), numero(p.y));
    }
    campos.extend(ev.datos.iter().map(|&(k, v)| (k.to_owned(), numero(v))));
    campos.extend(ev.valores.iter().map(|(k, d)| ((*k).to_owned(), valor(d))));
    if let Some(b) = batalla_de(ev) {
        campos.insert("battleId".to_owned(), texto(n.uuid_de_batalla(b)));
    }
    Struct { fields: campos }
}

/// El mensaje tipado de los quince hechos que mueven la interfaz (ver la tabla del módulo); `None` en el resto.
fn tipado(n: &Nucleo, ev: &EventoNucleo) -> Option<Tipado> {
    let dato = |k: &str| ev.dato(k);
    let id = |k: &str| dato(k).map(|v| v as u64);
    let faccion = |k: &str| dato(k).and_then(|v| u32::try_from(v).ok()).map(ids::faccion);
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
        (_, Some("abierta")) => {
            let b = n.batalla(batalla_de(ev)?)?;
            Tipado::BatallaAbierta(pb::BatallaAbierta {
                battle_id: battle_id(),
                contexto: Some(batalla::contexto(n, &b.ticket.contexto)),
                punto: Some(punto(b.punto)),
                iniciada_por: n.heroe(b.iniciada_por).map(|_| ids::heroe(b.iniciada_por)),
                atacante_faccion_id: b.ticket.atacante.faccion.map(ids::faccion),
                defensor_faccion_id: b.ticket.defensor.faccion.map(ids::faccion),
                convocatoria_hasta_ms: b.convocatoria_hasta,
            })
        }
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
            let lugar = id("id")?;
            let lugar_id = match dato("lugar")? {
                0 => ids::asentamiento(lugar),
                1 => ids::pueblo(lugar),
                _ => ids::avanzada(lugar),
            };
            // Los motivos de `nucleo::dentro`: 0 la puerta se cierra, 1 el lugar desaparece, 2 su plaza madre cambia de dueño.
            let motivo = match dato("motivo")? {
                0 => "puerta_cerrada",
                1 => "ruina",
                _ => "conquista",
            };
            Tipado::HeroeExpulsado(pb::HeroeExpulsado { heroe_id: heroe("heroe"), lugar_id, motivo: motivo.into(), ejercito_id: id("ejercito").map(ids::ejercito) })
        }
        ("heroe.desalojado", _) => Tipado::HeroeExpulsado(pb::HeroeExpulsado { heroe_id: heroe("heroe"), lugar_id: plaza_id(), motivo: "desalojo".into(), ejercito_id: None }),
        ("ejercito.viveres_bajos" | "ejercito.sin_viveres", _) => Tipado::ColumnaConHambre(pb::ColumnaConHambre { ejercito_id: columna(), sin_viveres: codigo == "ejercito.sin_viveres" }),
        ("tropas.desercion", _) => Tipado::DesercionPorHambre(pb::DesercionPorHambre { heroe_id: heroe("heroe"), escuadra_id: ids::escuadra(id("escuadra")?), desertores: u32::try_from(dato("desertores")?).ok()? }),
        ("faccion.guerra_declarada", _) => Tipado::GuerraDeclarada(pb::GuerraDeclarada { atacante_faccion_id: faccion("atacante")?, contra_faccion_id: faccion("contra")?, arrastradas_ids: Vec::new() }),
        ("construccion.edificio_completado", _) => obra("edificio", ids::edificio(plaza?, u32::try_from(dato("edificio")?).ok()?), None),
        ("construccion.mejora_completada", _) => obra("mejora", ids::edificio(plaza?, u32::try_from(dato("edificio")?).ok()?), dato("nivel").and_then(|v| u32::try_from(v).ok())),
        ("construccion.recinto_completado", _) => obra("recinto", ids::recinto(plaza?, u32::try_from(dato("recinto")?).ok()?), dato("nivel").and_then(|v| u32::try_from(v).ok())),
        ("pueblo.completado", _) => obra("pueblo", ids::pueblo(id("pueblo")?), None),
        ("avanzada.completada", _) => obra("avanzada", ids::avanzada(id("avanzada")?), None),
        ("heroe.herido", _) => Tipado::HeroeHerido(pb::HeroeHerido { heroe_id: heroe("heroe"), hasta_ms: dato("hasta")? }),
        _ => return None,
    })
}
