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
    /// Falta lo que cambia el paso de minuto (el almacén, la población, el medidor, la nutrición y los edificios parados por el almacén lleno): ese paso no marca nada, lleva su propia secuencia por plaza
    /// (`Nucleo::version_interior`).
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
            (a.faccion, a.radio, vecinas, a.nivel(), a.nivel_actual(), a.protegida_hasta, a.ocupada_hasta, &a.puerta, a.cargos, &a.fundadores, a.permite_reabastecer_aliados, e.comercio.ultima_caravana.get(&a.id)),
            (edificios, &eco.recintos, &eco.politicas, eco.ascenso, &eco.recetas_pausadas, eco.auto_pausada, eco.reserva_manual),
            (residentes, presentes, guarnicion, flota, aparcadas, pueblos, avanzadas, self.batalla_sobre(militar::presencia::Ubicacion::Asentamiento(a.id as u32)).map(|b| b.id)),
        ))
    }

    /// El campamento `c` como lo ve quien está dentro, sin proyectarlo: su stock (el inicial mientras no se toca), el mercado global, los reclutas, quienes residen y están en él, sus préstamos y las
    /// tecnologías que desbloquea sin contar el plazo tras la primera (6C): ese cambia solo con el tiempo, en la cita de campamento de cada tecnología, que toca todos los campamentos (si se mirara
    /// aquí, un suceso del mismo instante anterior a la cita lo vería ya cambiado).
    fn huella_de_campamento(&self, c: u32, inicial: &comercio::mercenarios::StockCampamento) -> String {
        let e = &self.estado;
        let residentes: Vec<u64> = e.militar.heroes.values().filter(|h| h.heroe.residencia == Some(Residencia::Campamento(c))).map(|h| h.heroe.id).collect();
        let presentes: Vec<u64> = e.militar.heroes.values().filter(|h| h.heroe.presencia.ubicacion == militar::presencia::Ubicacion::Mercenarios(c)).map(|h| h.heroe.id).collect();
        let prestamos: Vec<u64> = e.militar.heroes.values().flat_map(|h| &h.heroe.escuadras).filter(|s| s.prestada == Some(c)).map(|s| s.id).collect();
        let vivas: Vec<u32> = self.espacio.plazas_por_faccion.keys().copied().collect();
        let tecnologias = e.tecnologia.tecnologias_del_campamento(&e.reglas.economia.tecnologia, &vivas, Instante::MIN);
        huella(&(e.comercio.mercenarios.stock.get(&c).unwrap_or(inicial), &e.comercio.mercenarios.mercado, residentes, presentes, prestamos, e.militar.reclutas.get(&c), tecnologias, e.fundacion.fondos.get(&c)))
    }

    fn huellas(&self) -> Huellas {
        let e = &self.estado;
        let mut m = Huellas::new();
        m.extend(e.ejercitos.iter().map(|(&id, x)| ((Clase::Ejercito, id), huella(x))));
        for (&id, h) in &e.militar.heroes {
            // Con sus solicitudes de ingreso (5H: viajan en su `Heroe`).
            let solicitudes: Vec<u32> = e.politica.facciones.iter().filter(|f| f.solicitudes.contains(&id)).map(|f| f.id).collect();
            m.insert((Clase::Heroe, id), huella(&(h, solicitudes)));
            m.extend(h.heroe.escuadras.iter().map(|x| ((Clase::Escuadra, x.id), huella(x))));
        }
        m.extend(e.comercio.flota.iter().map(|c| ((Clase::Caravana, c.id), huella(c))));
        m.extend(e.fundacion.caravanas.values().map(|c| ((Clase::Caravana, c.id), huella(c))));
        m.extend(e.comercio.acuerdos.iter().map(|a| ((Clase::Acuerdo, a.id), huella(a))));
        m.extend(e.comercio.libro.ordenes.iter().chain(&e.comercio.libro.historial).map(|o| ((Clase::OrdenMercado, o.id), huella(o))));
        for a in e.asentamientos.values() {
            // Cualquier cambio de lo que ve quien está dentro marca la plaza o su interior (`huella_de_plaza`). Tampoco se mira el almacén de un pueblo ni su marca `lleno`: los cambia el paso de minuto, que no marca nada (va por la secuencia interior de la plaza).
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
        // Una Facción es la entidad (sus ciudadanos, solicitudes, cargos, su experiencia...), su reputación, sus tecnologías y sus relaciones.
        m.extend(crate::proyeccion::audiencia::facciones(e).into_iter().map(|f| {
            let relaciones: Vec<_> = e.politica.relaciones.iter().filter(|r| r.incluye(f)).collect();
            // Con sus Aedas residentes, sus épicas y las cumplidas (6E).
            let aedas: Vec<_> = e.residentes.aedas.iter().filter(|a| a.faccion == f).collect();
            ((Clase::Faccion, f as u64), huella(&(e.politica.facciones.get(f), e.comercio.reputacion.get(&f), e.tecnologia.por_faccion.get(&f), relaciones, self.liga_de(f), aedas, e.residentes.cumplidas.get(&f))))
        }));
        m.extend(e.politica.relaciones.iter().map(|r| ((Clase::Relacion, r.id), huella(r))));
        m.extend(e.politica.titulos.iter().map(|x| ((Clase::Titulo, x.id.indice()), huella(x))));
        m.extend(e.politica.propuestas.iter().map(|p| ((Clase::Propuesta, p.id), huella(p))));
        m
    }

    /// Falla si alguna entidad cambió desde la última comprobación sin estar en las `tocadas` de la transición en curso. La primera vez solo toma la referencia.
    pub(super) fn auditar_cambios(&mut self) {
        if !self.ajustes.auditar_cambios {
            return;
        }
        let fracciones = self.fracciones_almacenadas();
        assert!(fracciones.is_empty(), "una cantidad almacenada no es entera (C3b): {:?} [{}]", fracciones, self.causa_auditada);
        // La Facción de cada héroe es la copia del índice de ciudadanía de la política (5B).
        let fs = &self.estado.politica.facciones;
        let descuadre: Vec<u64> = self.estado.militar.heroes.values().filter(|h| h.faccion != fs.faccion_de(h.heroe.id)).map(|h| h.heroe.id).collect();
        assert!(descuadre.is_empty(), "la Facción de los héroes {descuadre:?} no es la de su ciudadanía [{}]", self.causa_auditada);
        // El índice de las plazas de cada Facción (5C) es el de las plazas.
        let mut indice: BTreeMap<u32, std::collections::BTreeSet<u64>> = BTreeMap::new();
        self.estado.asentamientos.values().for_each(|a| {
            indice.entry(a.faccion).or_default().insert(a.id);
        });
        assert_eq!(indice, self.espacio.plazas_por_faccion, "el índice de plazas por Facción no es el de las plazas [{}]", self.causa_auditada);
        // El índice de plazas por nivel actual (6B) es el de las plazas.
        let mut por_nivel = [0u32; super::logros::NIVELES_INDICE];
        self.estado.asentamientos.values().for_each(|a| por_nivel[(a.nivel_actual() as usize).min(super::logros::NIVELES_INDICE - 1)] += 1);
        assert_eq!(por_nivel, self.espacio.plazas_por_nivel, "el índice de plazas por nivel actual no es el de las plazas [{}]", self.causa_auditada);
        // El índice de residentes por plaza (6E) es el de los residentes, que viven en plazas que existen y son de su dueña; cada cita de llegada es de una plaza que espera.
        assert_eq!(super::residentes::por_plaza(&self.estado.residentes), self.espacio.residentes_por_plaza, "el índice de residentes por plaza no es el de los residentes [{}]", self.causa_auditada);
        let ajenos: Vec<u32> = self.estado.residentes.aedas.iter().filter(|a| self.estado.asentamientos.get(&u64::from(a.plaza)).is_none_or(|p| p.faccion != a.faccion)).map(|a| a.id).collect();
        assert!(ajenos.is_empty(), "los residentes {ajenos:?} no viven en una plaza de su Facción [{}]", self.causa_auditada);
        let sin_espera: Vec<u64> = self.estado.residentes_citas.keys().copied().filter(|p| !self.estado.residentes.espera_desde.contains_key(&(*p as u32))).collect();
        assert!(sin_espera.is_empty(), "las plazas {sin_espera:?} tienen cita de llegada sin esperar [{}]", self.causa_auditada);
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

    /// Toda cantidad **almacenada** que no es un número entero de unidades (decisión C3b del autor, 2026-10-24: no existen decimales en los almacenes), con dónde
    /// está: almacenes de plaza, pueblo y avanzada (y su resto oculto fuera de `(−1000, 1000)`), carros de columna y de héroe, caravanas, almacén personal, lo que un
    /// héroe recuerda de una plaza, existencias de los campamentos, lo pactado y entregado de los trueques y las órdenes del mercado. Debe estar vacío: lo exige
    /// la auditoría de cambios en cada versión y lo cuenta el batch.
    pub fn fracciones_almacenadas(&self) -> Vec<String> {
        let e = &self.estado;
        let u = base::UNIDAD;
        let mut v = Vec::new();
        let mut carga = |donde: String, c: &comercio::Carga| {
            if let Some((r, n)) = c.iter().find(|(_, n)| **n % u != 0) {
                v.push(format!("{donde} lleva {n} de {}", r.nombre()));
            }
        };
        for c in e.ejercitos.values() {
            carga(format!("el carro de la columna {}", c.id), &c.columna.carga);
        }
        for h in e.militar.heroes.values() {
            carga(format!("el carro del héroe {}", h.heroe.id), &h.carro);
            carga(format!("el almacén personal del héroe {}", h.heroe.id), &h.almacen_personal);
            for (p, r) in &h.plazas_recordadas {
                carga(format!("lo que el héroe {} recuerda de la plaza {p}", h.heroe.id), &r.almacen);
            }
        }
        for c in &e.comercio.flota {
            carga(format!("la caravana {}", c.id), &c.contenido);
        }
        for (id, s) in &e.comercio.mercenarios.stock {
            carga(format!("las existencias del campamento {id}"), &s.stock);
        }
        let lineas = e.comercio.acuerdos.iter().flat_map(|a| a.lineas_a.iter().chain(&a.lineas_b).map(move |l| (a.id, l.cantidad_total, l.cantidad_entregada)));
        v.extend(lineas.filter(|&(_, t, d)| t % u != 0 || d % u != 0).map(|(id, t, d)| format!("el trueque {id} pacta {t} y lleva entregado {d}")));
        let ordenes = e.comercio.libro.ordenes.iter().chain(&e.comercio.libro.historial);
        v.extend(ordenes.filter(|o| o.cantidad % u != 0 || o.cantidad_cumplida % u != 0).map(|o| format!("la orden {} pide {} y lleva {}", o.id, o.cantidad, o.cantidad_cumplida)));
        let mut almacen = |donde: String, a: &asentamiento::Almacen| {
            let (cantidad, _, resto) = a.partes();
            if let Some(i) = (0..29).find(|&i| cantidad[i] % u != 0 || !(-u + 1..u).contains(&resto[i]) || cantidad[i] + resto[i] < 0) {
                v.push(format!("{donde} guarda {} de {} con un resto de {}", cantidad[i], asentamiento::recurso::TODOS[i].nombre(), resto[i]));
            }
        };
        for a in e.asentamientos.values() {
            almacen(format!("la plaza {}", a.id), &a.economia.almacen);
            a.economia.pueblos.iter().for_each(|p| almacen(format!("el pueblo {}", p.id), &p.almacen));
            a.economia.avanzadas.iter().for_each(|x| almacen(format!("la avanzada {}", x.id), &x.almacen));
        }
        e.combate.huerfanas.iter().for_each(|(id, x)| almacen(format!("la avanzada huérfana {id}"), &x.almacen));
        v
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
