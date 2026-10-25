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
//! | `tecnologia.logro`, `era.comienza` | el logro (su contador y su umbral, sin la tecnología que abre) y la Era nueva (su id, su orden y si llegó por plazo), sin Facciones (6C) |
//! | `aedas.canta_descubrimiento` | el descubrimiento: la descubridora y la tecnología (6C) |
//! | `aedas.epica_cumplida` | la épica cumplida: la Facción, sin la tecnología que la épica hizo aparecer (6E) |
//! | `asentamiento.nivel_subio` con nivel ≥ 3 | solo si es la primera plaza del mundo que llega a ese nivel (ninguna otra, fuera de las que suben en la misma versión, lo tenía ya); si suben dos a la vez, la primera |
//!
//! **Guardada entera** (sistema 6, decisión Q2; §5.3 del plan): en el núcleo persistente cada entrada se guarda, ya como el `EventoDominio` que viaja en las `Delta`, en un diario aparte
//! (`<dir>/cronica/`, [`persistencia::Diario`]) con la versión que la produjo. No entra en la foto ni en el hash. La escribe por lotes el hilo de la persistencia (fuera del camino de la
//! orden), que antes de escribir una foto escribe lo anterior; al recuperar se trunca a la versión de la foto y la repetición del registro la vuelve a escribir igual. `PedirCronica` lee una
//! página en ese mismo hilo, detrás de lo que ya se le mandó: llega hasta la versión del mundo al pedirla, sin tocar el estado ni bloquear el actor.
//!
//! C4 (cerrada en 6C: queda la regla de Rust): el canon canta la caída por `combate.asedio_conquista`, que en Rust solo da el asedio con números; la conquista por Unity no lo emite, así que la entrada sale de
//! `asentamiento.conquistada`, que dan las dos vías (C4 del plan del sistema 6: así queda). La frase la compone el cliente (el contrato no lleva texto): la entrada no trae la variante de §8. Los
//! públicos del sistema 6 (6C y 6E) son hechos públicos que además dejan su entrada, como los títulos.

use super::*;
use crate::estado::Dato;
use crate::proyeccion::{eventos::evento_dominio, ids};
use contrato::v1 as pb;
use prost::Message;
use std::collections::BTreeSet;

/// El subdirectorio del almacén de la partida donde se guarda la crónica.
pub(super) const DIRECTORIO: &str = "cronica";
/// Entradas de una página si no se dice (Proy l.614) y el tope.
pub const LIMITE_POR_DEFECTO: u32 = 200;
pub const LIMITE_MAXIMO: u32 = 1_000;

/// Una entrada por guardar: la versión que la produjo, su instante y el `EventoDominio` codificado.
pub(super) type Apunte = (u64, Instante, Vec<u8>);
/// Quien recibe la página pedida (el actor de sesiones la deposita en el canal de la sesión).
pub type Responder = Box<dyn FnOnce(pb::Cronica) + Send>;

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
                "tecnologia.logro" | "era.comienza" => Vec::new(),
                "aedas.canta_descubrimiento" | "aedas.epica_cumplida" => datos(h, &["faccion"]),
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

impl Nucleo {
    /// Deja las entradas de la versión `v` para el diario (solo el núcleo persistente).
    pub(super) fn apuntar_cronica(&mut self, v: u64, entradas: &[EventoNucleo]) {
        if entradas.is_empty() || self.cronica.is_none() {
            return;
        }
        let apuntes: Vec<Apunte> = entradas.iter().map(|e| (v, e.en, evento_dominio(self, e, v).encode_to_vec())).collect();
        self.cronica.as_mut().expect("mirado").extend(apuntes);
    }

    /// Manda al hilo de la persistencia las entradas cerradas que aún no tiene.
    pub(super) fn volcar_cronica(&mut self) {
        if let (Some(p), Some(c)) = (&self.persist, &mut self.cronica) {
            if !c.is_empty() {
                let _ = p.fotos.send(MsgFoto::Cronica(std::mem::take(c)));
            }
        }
    }

    /// Pide una página de la crónica guardada (`PedirCronica`, contrato 05 §8). La contesta el hilo de la persistencia por `responder`, después de escribir todo lo cerrado hasta ahora, así
    /// que llega hasta la versión de este momento. Sin persistencia no hay crónica guardada: la página va vacía.
    pub fn pedir_cronica(&mut self, pedir: pb::PedirCronica, responder: Responder) {
        self.volcar_cronica();
        let version = self.estado.version;
        let vacia = |pedir: &pb::PedirCronica| pb::Cronica { peticion_id: pedir.peticion_id.clone(), version, ..Default::default() };
        let Some(p) = &self.persist else { return responder(vacia(&pedir)) };
        if let Err(mpsc::SendError(MsgFoto::PedirCronica { pedir, responder, .. })) = p.fotos.send(MsgFoto::PedirCronica { pedir, version, responder }) {
            responder(vacia(&pedir));
        }
    }
}

/// La página pedida, leída del diario (vacía si la crónica no se pudo escribir o una entrada no se lee: se avisa y no se inventa nada).
pub(super) fn pagina(diario: Option<&Diario>, pedir: &pb::PedirCronica, version: u64) -> pb::Cronica {
    let limite = match pedir.limite {
        0 => LIMITE_POR_DEFECTO,
        l => l.min(LIMITE_MAXIMO),
    };
    let mut c = pb::Cronica { peticion_id: pedir.peticion_id.clone(), version, ..Default::default() };
    let Some(d) = diario else { return c };
    let leida = d.pagina(pedir.desde_version, pedir.hasta_version, u64::from(limite)).and_then(|p| {
        let entradas = p.apuntes.iter().map(|a| pb::EventoDominio::decode(a.datos.as_slice()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))).collect::<io::Result<_>>()?;
        Ok((entradas, p.mas_antiguas, p.mas_recientes))
    });
    match leida {
        Ok((entradas, antiguas, recientes)) => (c.entradas, c.mas_antiguas, c.mas_recientes) = (entradas, antiguas, recientes),
        Err(e) => eprintln!("nucleo: no se pudo leer la crónica: {e}"),
    }
    c
}

/// La entrada de crónica del hecho `h`: pública, sin sitio, con su tipo, las Facciones y, si la hay, la plaza y su nivel.
fn entrada(h: &EventoNucleo, facciones: Vec<u32>, plaza: Option<u64>) -> EventoNucleo {
    let mut valores = vec![("tipo", Dato::Texto(h.codigo.to_string())), ("faccionIds", Dato::Lista(facciones.iter().map(|&f| Dato::Texto(ids::faccion(f))).collect()))];
    valores.extend(plaza.map(|p| ("asentamientoId", Dato::Texto(ids::asentamiento(p)))));
    // El título cantado (5F): su clave estable. De los públicos del sistema 6 (6C), lo que ya dicen a todos: el contador, la Era o la tecnología cantada.
    valores.extend(h.valores.iter().filter(|(k, _)| matches!(*k, "titulo" | "contador" | "era" | "tecnologia")).cloned());
    let datos = match h.codigo.as_ref() {
        "asentamiento.nivel_subio" => h.datos.iter().filter(|(k, _)| *k == "nivel").copied().collect(),
        "tecnologia.logro" | "era.comienza" => h.datos.clone(),
        _ => Vec::new(),
    };
    EventoNucleo { valores, publico: true, ..EventoNucleo::nuevo("cronica.entrada", h.en, datos) }
}
