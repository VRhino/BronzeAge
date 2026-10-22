//! Auditoría del registro de cambios: comprueba que **ninguna entidad cambia sin quedar en `CambioVersion::tocados`**.
//!
//! Es lo que permite a las proyecciones (fase 4H) confiar en el registro: un cliente solo recibe lo que está en él. Con `Ajustes::auditar_cambios`, cada vez que el
//! núcleo cierra una versión (y cuando no hay nada que cerrar) compara la huella de cada entidad con la de la última vez y falla si alguna cambió sin haber sido
//! marcada con `Transicion::tocar`. Es caro (da formato a todo el estado): solo para pruebas. `BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test` lo activa en toda la suite.

use super::*;
use militar::heroe::Residencia;
use std::collections::BTreeSet;

/// La huella de cada entidad con identidad: el `Debug` de su estado. No entra en el hash ni en las fotos.
pub(super) type Huellas = BTreeMap<(Clase, u64), String>;

fn huella(d: &impl std::fmt::Debug) -> String {
    format!("{d:?}")
}

/// Dónde difieren dos huellas, con un poco de contexto.
fn diferencia(antes: Option<&String>, ahora: Option<&String>) -> String {
    let (Some(a), Some(b)) = (antes, ahora) else { return if antes.is_none() { "nació".into() } else { "desapareció".into() } };
    let comun = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
    let desde = a[..comun].char_indices().rev().nth(60).map_or(0, |(i, _)| i);
    let corte = |s: &String| s[desde..].chars().take(160).collect::<String>();
    format!("antes «{}» ahora «{}»", corte(a), corte(b))
}

impl Nucleo {
    fn huellas(&self) -> Huellas {
        let e = &self.estado;
        let mut m = Huellas::new();
        m.extend(e.ejercitos.iter().map(|(&id, x)| ((Clase::Ejercito, id), huella(x))));
        for (&id, h) in &e.militar.heroes {
            m.insert((Clase::Heroe, id), huella(h));
            m.extend(h.heroe.escuadras.iter().map(|x| ((Clase::Escuadra, x.id), huella(x))));
        }
        m.extend(e.comercio.flota.iter().map(|c| ((Clase::Caravana, c.id), huella(c))));
        m.extend(e.comercio.acuerdos.iter().map(|a| ((Clase::Acuerdo, a.id), huella(a))));
        m.extend(e.comercio.libro.ordenes.iter().chain(&e.comercio.libro.historial).map(|o| ((Clase::OrdenMercado, o.id), huella(o))));
        for a in e.asentamientos.values() {
            // Lo que el paso de minuto extrae al almacén del pueblo (y su marca `lleno`) no cuenta: es del paso de minuto de la plaza, como su almacén.
            m.extend(a.economia.pueblos.iter().map(|p| ((Clase::Pueblo, p.id), huella(&(a.id, p.tipo, p.nodo, &p.estado, p.carro, e.combate.guarniciones.get(&p.id))))));
            m.extend(a.economia.avanzadas.iter().map(|v| ((Clase::Avanzada, v.id), huella(&(a.id, v, e.combate.avanzadas.get(&v.id))))));
        }
        m.extend(e.combate.huerfanas.iter().map(|(&id, v)| ((Clase::Avanzada, id), huella(&(v, e.combate.avanzadas.get(&id))))));
        m.extend(e.combate.bandidos.iter().map(|(&id, b)| ((Clase::Bandidos, id), huella(b))));
        m.extend(e.militar.convocatorias.iter().map(|(&id, c)| ((Clase::Convocatoria, id), huella(c))));
        m.extend(e.batallas.batallas.iter().map(|(&id, b)| ((Clase::Batalla, id), huella(b))));
        // Un campamento de mercenarios es su stock (el inicial mientras no se toca), el mercado global y quienes residen en él.
        let n = self.geografia.as_ref().map_or(0, |g| g.mapa.campamentos.len() as u32);
        let inicial = comercio::mercenarios::StockCampamento::nuevo(&e.reglas.economia.comercio);
        let residentes = |c: u32| e.militar.heroes.values().filter(|h| h.heroe.residencia == Some(Residencia::Campamento(c))).map(|h| h.heroe.id).collect::<Vec<_>>();
        m.extend((0..n).map(|c| ((Clase::Campamento, c as u64), huella(&(e.comercio.mercenarios.stock.get(&c).unwrap_or(&inicial), &e.comercio.mercenarios.mercado, residentes(c))))));
        // Una Facción es su nivel, su reputación, sus tecnologías, sus relaciones y sus ciudadanos.
        let facciones: BTreeSet<u32> = e
            .niveles_faccion
            .keys()
            .chain(e.comercio.reputacion.keys())
            .chain(e.militar.tecnologias.keys())
            .chain(e.militar.relaciones.keys().flat_map(|(a, b)| [a, b]))
            .copied()
            .chain(e.militar.heroes.values().filter_map(|h| h.faccion))
            .chain(e.asentamientos.values().map(|a| a.faccion))
            .collect();
        m.extend(facciones.into_iter().map(|f| {
            let relaciones: Vec<_> = e.militar.relaciones.iter().filter(|((a, b), _)| *a == f || *b == f).collect();
            let ciudadanos: Vec<u64> = e.militar.heroes.values().filter(|h| h.faccion == Some(f)).map(|h| h.heroe.id).collect();
            ((Clase::Faccion, f as u64), huella(&(e.niveles_faccion.get(&f), e.comercio.reputacion.get(&f), e.militar.tecnologias.get(&f), relaciones, ciudadanos)))
        }));
        m
    }

    /// Falla si alguna entidad cambió desde la última comprobación sin estar en las `tocadas` de la transición en curso. La primera vez solo toma la referencia.
    pub(super) fn auditar_cambios(&mut self) {
        if !self.ajustes.auditar_cambios {
            return;
        }
        let ahora = self.huellas();
        if let Some(antes) = &self.huellas_auditadas {
            for clave in antes.keys().chain(ahora.keys()).collect::<BTreeSet<_>>() {
                assert!(
                    antes.get(clave) == ahora.get(clave) || self.tr.tocados.contiene(clave.0, clave.1),
                    "{:?} {} cambió sin quedar en el registro de cambios (falta `tocar` donde lo cambia): {} [{}]",
                    clave.0,
                    clave.1,
                    self.causa_auditada,
                    diferencia(antes.get(clave), ahora.get(clave))
                );
            }
        }
        self.huellas_auditadas = Some(ahora);
    }

    /// Toma de nuevo la referencia, ya con las versiones puestas en las entidades que las llevan.
    pub(super) fn rebasar_auditoria(&mut self) {
        if self.ajustes.auditar_cambios {
            self.huellas_auditadas = Some(self.huellas());
        }
    }
}
