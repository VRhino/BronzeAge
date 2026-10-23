//! Auditoría del registro de cambios: comprueba que **ninguna entidad cambia sin quedar en `CambioVersion::tocados`**.
//!
//! Es lo que permite a las proyecciones (fase 4H) confiar en el registro: un cliente solo recibe lo que está en él. Con `Ajustes::auditar_cambios`, cada vez que el
//! núcleo cierra una versión (y cuando no hay nada que cerrar) compara la huella de cada entidad con la de la última vez y falla si alguna cambió sin haber sido
//! marcada con `Transicion::tocar`. Es caro (da formato a todo el estado): solo para pruebas. `BRONZEAGE_AUDITAR_CAMBIOS=1 cargo test` lo activa en toda la suite.

use super::*;
use crate::estado::Asentamiento;
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
    /// Lo que ve de una plaza quien está dentro (`proyeccion::interior`), sin proyectarlo (la proyección entera, con su ciudad y su polígono, es demasiado cara para mirarla en cada versión de cada plaza
    /// en una compilación sin optimizar): la ficha, los edificios con su cola, los recintos, las políticas, el ascenso, lo configurable, quién vive, está dentro y guarda la plaza, las caravanas y el asedio.
    /// Falta lo que cambia el paso de minuto (el almacén, la población, el medidor, la nutrición y los edificios parados por el almacén lleno): ese paso solo marca con sesiones, y la auditoría no debe
    /// cambiar las versiones.
    fn huella_de_plaza(&self, a: &Asentamiento) -> String {
        let (e, eco) = (&self.estado, &a.economia);
        let edificios: Vec<_> = eco.edificios.iter().map(|b| (b.id, b.tipo, b.estado, b.completa_en, b.prioridad, b.nivel_interno, b.mejora, b.danado, b.fuente)).collect();
        // La zona se recorta contra las vecinas con las que se toca: cuentan sus radios.
        let vecinas: Vec<_> = e.contactos.iter().filter_map(|&(x, y)| if x == a.id { Some(y) } else { (y == a.id).then_some(x) }).filter_map(|o| e.asentamientos.get(&o).map(|o| (o.id, o.radio))).collect();
        let residentes: Vec<u64> = e.militar.residentes_de(a.id).map(|h| h.heroe.id).collect();
        let presentes: Vec<u64> = e.militar.heroes.values().filter(|h| matches!(h.heroe.presencia.ubicacion, militar::presencia::Ubicacion::Asentamiento(x) if u64::from(x) == a.id)).map(|h| h.heroe.id).collect();
        let guarnicion: Vec<u64> = residentes.iter().filter_map(|h| e.militar.heroes.get(h)).flat_map(|h| &h.heroe.escuadras).filter(|s| s.en_guarnicion && s.con_soldados()).map(|s| s.id).collect();
        let flota: Vec<u64> = e.comercio.flota.iter().filter(|c| u64::from(c.origen) == a.id).map(|c| c.id).collect();
        let aparcadas: Vec<u64> = e.comercio.flota.iter().filter(|c| c.estado == comercio::caravana::EstadoCaravana::Aparcada && c.anfitriona.is_some_and(|x| u64::from(x) == a.id)).map(|c| c.id).collect();
        let pueblos: Vec<_> = eco.pueblos.iter().map(|p| p.id).collect();
        let avanzadas: Vec<_> = eco.avanzadas.iter().map(|v| v.id).collect();
        huella(&(
            (a.faccion, a.radio, vecinas, a.nivel(), a.nivel_actual(), a.protegida_hasta, a.ocupada_hasta, a.acceso),
            (edificios, &eco.recintos, &eco.politicas, eco.ascenso, &eco.recetas_pausadas, eco.auto_pausada, eco.reserva_manual),
            (residentes, presentes, guarnicion, flota, aparcadas, pueblos, avanzadas, self.batalla_sobre(militar::presencia::Ubicacion::Asentamiento(a.id as u32)).map(|b| b.id)),
        ))
    }

    /// El campamento `c` como lo ve quien está dentro, sin proyectarlo: su stock (el inicial mientras no se toca), el mercado global, los reclutas, quienes residen y están en él, sus préstamos y las
    /// tecnologías de las que depende lo que ofrece.
    fn huella_de_campamento(&self, c: u32, inicial: &comercio::mercenarios::StockCampamento) -> String {
        let e = &self.estado;
        let residentes: Vec<u64> = e.militar.heroes.values().filter(|h| h.heroe.residencia == Some(Residencia::Campamento(c))).map(|h| h.heroe.id).collect();
        let presentes: Vec<u64> = e.militar.heroes.values().filter(|h| h.heroe.presencia.ubicacion == militar::presencia::Ubicacion::Mercenarios(c)).map(|h| h.heroe.id).collect();
        let prestamos: Vec<u64> = e.militar.heroes.values().flat_map(|h| &h.heroe.escuadras).filter(|s| s.prestada == Some(c)).map(|s| s.id).collect();
        huella(&(e.comercio.mercenarios.stock.get(&c).unwrap_or(inicial), &e.comercio.mercenarios.mercado, residentes, presentes, prestamos, e.militar.reclutas.get(&c), &e.militar.tecnologias))
    }

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
            // Cualquier cambio de lo que ve quien está dentro marca la plaza o su interior (`huella_de_plaza`). Tampoco se mira el almacén de un pueblo ni su marca `lleno`: los marca el paso de minuto solo con sesiones.
            m.insert((Clase::Asentamiento, a.id), self.huella_de_plaza(a));
            m.extend(a.economia.pueblos.iter().map(|p| ((Clase::Pueblo, p.id), huella(&(a.id, p.tipo, p.nodo, &p.estado, p.carro, e.combate.guarniciones.get(&p.id))))));
            m.extend(a.economia.avanzadas.iter().map(|v| ((Clase::Avanzada, v.id), huella(&(a.id, v, e.combate.avanzadas.get(&v.id))))));
        }
        m.extend(e.combate.huerfanas.iter().map(|(&id, v)| ((Clase::Avanzada, id), huella(&(v, e.combate.avanzadas.get(&id))))));
        m.extend(e.combate.bandidos.iter().map(|(&id, b)| ((Clase::Bandidos, id), huella(b))));
        m.extend(e.militar.convocatorias.iter().map(|(&id, c)| ((Clase::Convocatoria, id), huella(c))));
        m.extend(e.batallas.batallas.iter().map(|(&id, b)| ((Clase::Batalla, id), huella(b))));
        // Un campamento de mercenarios es lo que se proyecta a quien está dentro: su stock (el inicial mientras no se toca), el mercado global, los reclutas, quienes residen y están en él y sus préstamos.
        let n = self.geografia.as_ref().map_or(0, |g| g.mapa.campamentos.len() as u32);
        let inicial = comercio::mercenarios::StockCampamento::nuevo(&e.reglas.economia.comercio);
        m.extend((0..n).map(|c| ((Clase::Campamento, c as u64), self.huella_de_campamento(c, &inicial))));
        // Una Facción es su nivel, su reputación, sus tecnologías, sus relaciones y sus ciudadanos.
        m.extend(crate::proyeccion::audiencia::facciones(e).into_iter().map(|f| {
            let relaciones: Vec<_> = e.militar.relaciones.iter().filter(|((a, b), _)| *a == f || *b == f).collect();
            let ciudadanos: Vec<u64> = e.militar.heroes.values().filter(|h| h.faccion == Some(f)).map(|h| h.heroe.id).collect();
            ((Clase::Faccion, f as u64), huella(&(e.niveles_faccion.get(&f), e.comercio.reputacion.get(&f), e.militar.tecnologias.get(&f), relaciones, ciudadanos)))
        }));
        m.extend(e.militar.relaciones.iter().map(|(&(a, b), r)| ((Clase::Relacion, Clase::par(a, b)), huella(r))));
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
                // El interior de una plaza (lo que cambia sin cambiar su ficha) tiene su propia marca.
                let tocada = self.tr.tocados.contiene(clave.0, clave.1) || (clave.0 == Clase::Asentamiento && self.tr.tocados.contiene(Clase::Interior, clave.1));
                assert!(
                    antes.get(clave) == ahora.get(clave) || tocada,
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
            if self.interiores.is_none() {
                self.seguir_interiores();
            }
            self.huellas_auditadas = Some(self.huellas());
        }
    }
}
