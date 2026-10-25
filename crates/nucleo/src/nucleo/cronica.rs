//! La crónica de los Aedas (Tecnologia_Aedas §8): no es estado, es una **vista de los hechos**. Al cerrar cada versión, de los hechos que los Aedas cantan se deriva un hecho público
//! `cronica.entrada { tipo, faccionIds, asentamientoId?, nivel? }` que dice lo que cualquiera puede saber, sin sitio ni detalle: lo oyen todos (`EventoNucleo::publico`).
//!
//! | Hecho | Entrada |
//! |---|---|
//! | `asentamiento.conquistada` | la caída de la plaza: la conquistadora y la que la tenía |
//! | `asentamiento.ruinas` | las ruinas de la plaza: la que la tenía |
//! | `fundacion.asentamiento_fundado` | la fundación: la fundadora |
//! | `diplomacia.guerra_declarada` | la guerra: la que declara y la declarada (5D: también la de la orden `declararGuerra` y la de una rebelión) |
//! | `diplomacia.rebelion_vasallo`, `.vasallo_liberado`, `.vasallo_conquistado` | la rebelión, la liberación y el traspaso de una vasalla: la señora (o la conquistadora) y la vasalla (5D) |
//! | `diplomacia.anexion` | la anexión: la absorbente y la absorbida (5F) |
//! | `diplomacia.fusion` | la fusión: las dos que se disuelven y la que nace (5F) |
//! | `titulo.nace`, `titulo.cambia_manos` | el título: su poseedor (y el anterior); la entrada lleva además la clave del título (5F) |
//! | `asentamiento.nivel_subio` con nivel ≥ 3 | solo si es la primera plaza del mundo que llega a ese nivel (ninguna otra, fuera de las que suben en la misma versión, lo tenía ya); si suben dos a la vez, la primera |
//!
//! DUDA: el canon canta la caída por `combate.asedio_conquista`, que en Rust solo da el asedio con números; la conquista por Unity no lo emite, así que la entrada sale de
//! `asentamiento.conquistada`, que dan las dos vías. La frase la compone el cliente (el contrato no lleva texto): la entrada no trae la variante de §8. Faltan los públicos de la tecnología y los
//! Aedas (sistema 6): filas de paridad.

use super::*;
use crate::estado::Dato;
use crate::proyeccion::ids;
use std::collections::BTreeSet;

/// El nivel a partir del que se canta la primera plaza del mundo que lo alcanza (Tecnologia_Aedas §8).
const NIVEL_MINIMO_CANTADO: i64 = 3;

impl Nucleo {
    /// Las entradas de la crónica que salen de los hechos de una versión, en su orden.
    pub(super) fn cronica(&self, hechos: &[EventoNucleo]) -> Vec<EventoNucleo> {
        let plaza = |h: &EventoNucleo| h.asentamientos.first().copied();
        let datos = |h: &EventoNucleo, claves: &[&str]| claves.iter().filter_map(|k| h.dato(k)).filter_map(|f| u32::try_from(f).ok()).collect::<Vec<u32>>();
        let mut cantados = BTreeSet::new();
        let mut entradas = Vec::new();
        for h in hechos {
            let facciones = match h.codigo.as_ref() {
                "asentamiento.conquistada" => datos(h, &["faccion", "faccionAnterior"]),
                "fundacion.asentamiento_fundado" => datos(h, &["faccion"]),
                "diplomacia.guerra_declarada" => datos(h, &["atacante", "contra"]),
                "diplomacia.rebelion_vasallo" | "diplomacia.vasallo_liberado" | "diplomacia.vasallo_conquistado" | "diplomacia.anexion" => datos(h, &["faccionA", "faccionB"]),
                "diplomacia.fusion" => datos(h, &["faccionA", "faccionB", "faccion"]),
                "titulo.nace" | "titulo.cambia_manos" => datos(h, &["faccion", "faccionAnterior"]),
                "asentamiento.ruinas" => h.facciones.clone(),
                "asentamiento.nivel_subio" => match h.dato("nivel") {
                    Some(n) if n >= NIVEL_MINIMO_CANTADO && self.primera_en_llegar(hechos, plaza(h), n) && cantados.insert(n) => h.facciones.clone(),
                    _ => continue,
                },
                _ => continue,
            };
            entradas.push(entrada(h, facciones, plaza(h)));
        }
        entradas
    }

    /// ¿Ninguna plaza del mundo, salvo las que suben a `nivel` en esta misma versión, tenía ya ese nivel?
    fn primera_en_llegar(&self, hechos: &[EventoNucleo], plaza: Option<u64>, nivel: i64) -> bool {
        let suben: BTreeSet<u64> = hechos.iter().filter(|h| h.codigo == "asentamiento.nivel_subio" && h.dato("nivel") == Some(nivel)).filter_map(|h| h.asentamientos.first().copied()).collect();
        plaza.is_some() && !self.estado.asentamientos.values().any(|a| !suben.contains(&a.id) && i64::from(a.nivel()) >= nivel)
    }
}

/// La entrada de crónica del hecho `h`: pública, sin sitio, con su tipo, las Facciones y, si la hay, la plaza y su nivel.
fn entrada(h: &EventoNucleo, facciones: Vec<u32>, plaza: Option<u64>) -> EventoNucleo {
    let mut valores = vec![("tipo", Dato::Texto(h.codigo.to_string())), ("faccionIds", Dato::Lista(facciones.iter().map(|&f| Dato::Texto(ids::faccion(f))).collect()))];
    valores.extend(plaza.map(|p| ("asentamientoId", Dato::Texto(ids::asentamiento(p)))));
    // El título cantado (5F): su clave estable.
    valores.extend(h.valores.iter().filter(|(k, _)| *k == "titulo").cloned());
    let datos = h.dato("nivel").filter(|_| h.codigo == "asentamiento.nivel_subio").map(|n| ("nivel", n)).into_iter().collect();
    EventoNucleo { valores, publico: true, ..EventoNucleo::nuevo("cronica.entrada", h.en, datos) }
}
