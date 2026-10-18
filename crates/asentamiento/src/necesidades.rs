//! Auto-construcción por necesidad (Doc 4.2, Economia_Construccion §6.2): el asentamiento decide solo qué construir según lo que
//! le falta (comida, madera, Viviendas, almacén, talleres). Cada candidato recibe un score = base de su banda + urgencia, y se
//! compromete (se paga y se encola) en orden de score mientras haya cupo y fondos. Nunca elige dónde: eso es del trazado.
//!
//! Cambios de la reconstrucción respecto a TypeScript, ya decididos: los minerales y toda Cantera salvo la primera van por pueblo
//! extractor (Doc 4.2.2), así que la cola automática solo propone la primera Cantera y el Corral; un candidato que no se
//! compromete no deja rastro.

use crate::asentamiento::{Asentamiento, Evento, Exterior};
use crate::catalogo::Tipo;
use crate::datos::{por, razon, Reglas, UNO};
use crate::edificio::{Edificio, Edificios, Estado, Fuente};
use crate::produccion::{self, Mano};
use base::{Instante, Mil};

struct Candidato {
    tipo: Tipo,
    score: i64,
    fuente: Option<Fuente>,
}

impl Asentamiento {
    /// ¿Algún edificio de ese tipo que no esté activo? (`proyectoEnCurso`)
    fn pendiente(&self, tipo: Tipo) -> bool {
        self.edificios.iter().any(|e| e.tipo == tipo && !e.activo())
    }

    /// `puedePagar`: la Facción tiene la tecnología y el almacén entero, con la reserva, cubre el coste.
    fn puede_pagar(&self, r: &Reglas, x: &Exterior, tipo: Tipo, reserva: &[Mil; 29]) -> bool {
        let def = r.catalogo.tipo(tipo);
        def.tecnologia.as_deref().map_or(true, |t| (x.tiene_tecnologia)(t)) && self.puede_iniciar(r, tipo, &def.costo, reserva)
    }

    /// `tieneInsumoDeArranque`: alguna receta del nivel 1 consume un insumo que haya en el almacén (sin recetas, verdadero).
    fn tiene_insumo_de_arranque(&self, r: &Reglas, tipo: Tipo) -> bool {
        let Some(n1) = r.catalogo.tipo(tipo).nivel(1) else { return true };
        n1.recetas.is_empty() || n1.recetas.iter().any(|rc| rc.consume_por_unidad.iter().any(|&(i, _)| self.almacen.cantidad(i) > 0))
    }

    fn alcanzo_tope_transformacion(&self, r: &Reglas, tipo: Tipo, ya_comprometidos: usize) -> bool {
        r.construccion.necesidades.maximo_transformacion_por_nivel.get(&self.nivel_actual()).is_some_and(|&t| (self.edificios.en_cualquier_estado(tipo) + ya_comprometidos) as u32 >= t)
    }

    /// Mayor ocupación (ppm) de un recurso con capacidad, y la del trigo.
    fn ocupacion_maxima(&self) -> i64 {
        crate::recurso::TODOS.iter().filter(|&&r| self.almacen.capacidad(r) > 0).map(|&r| razon(self.almacen.cantidad(r), self.almacen.capacidad(r))).max().unwrap_or(0)
    }

    /// `hayMejoraGranjaDisponible`: queda una cuadrilla libre tras reservar la de construir y alguna Granja puede pagar ya su
    /// siguiente mejora. Entonces se prefiere mejorar a construir otra (mejorar no diluye la mano de obra).
    fn hay_mejora_granja_disponible(&self, r: &Reglas, x: &Exterior, reserva: &[Mil; 29]) -> bool {
        if self.edificios.cuadrillas_ocupadas() + 1 >= r.construccion.necesidades.maximo_en_construccion_simultanea {
            return false;
        }
        self.edificios.iter().any(|e| {
            e.tipo == Tipo::Granja && self.elegible_para_mejora(r, e, x) && {
                let costo = r.catalogo.tipo(Tipo::Granja).nivel(e.nivel_interno + 1).map(|n| n.costo_mejora.clone()).unwrap_or_default();
                self.puede_iniciar(r, Tipo::Granja, &costo, reserva)
            }
        })
    }

    pub(crate) fn evaluar_necesidades(&mut self, _t: Instante, x: &mut Exterior, mano: &Mano, reserva: &[Mil; 29], ev: &mut Vec<Evento>) {
        let r = x.reglas;
        let c = &r.construccion;
        let n = &c.necesidades;
        let mut candidatos: Vec<Candidato> = vec![];
        let mut propuestos_extractor: Vec<Tipo> = vec![];
        // Las preguntas de sitio de esta pasada ven la ciudad tal como está al empezar: lo que se comprometa se confirma al final.
        let u = self.urbano(r);

        // Déficit de trigo: real y proyectado (con la población que cabe en las Viviendas construidas o en camino).
        let (cap_p, cap_a, cap_n) = self.capacidades(r);
        let pop = &self.poblacion;
        let entorno = (cap_p, cap_a, cap_n);
        let consumo_hab = |hab: Mil| por(hab, r.poblacion.consumo_por_habitante);
        let factor_consumo = self.politicas.factor(&r.politicas, "factorConsumoComida");
        let consumo_actual = por(consumo_hab(pop.total()), factor_consumo) + x.racion_guarnicion;
        let objetivo = pop.pesants.max(entorno.0) + pop.artesanos.max(entorno.1) + pop.nobleza.max(entorno.2);
        let consumo_objetivo = por(consumo_hab(objetivo), factor_consumo) + x.racion_guarnicion;
        let produccion = produccion::produccion_trigo_por_minuto(&r.catalogo, &self.edificios, mano, x.fertilidad, self.politicas.factor(&r.politicas, "factorProduccionTrigo"));
        let en_deficit = produccion < consumo_actual;
        let en_deficit_proyectado = produccion < consumo_objetivo;

        // 1. Granja.
        let activas = self.edificios.activos(Tipo::Granja);
        let pendientes = self.edificios.iter().filter(|e| e.tipo == Tipo::Granja && !e.activo()).count() as u32;
        let limite = if en_deficit { n.maximo_granjas_pendientes_en_deficit } else { 1 };
        if (activas == 0 || en_deficit_proyectado) && pendientes < limite && !self.hay_mejora_granja_disponible(r, x, reserva) && self.puede_pagar(r, x, Tipo::Granja, reserva) {
            if let Some(fuente) = x.sitios.proponer(Tipo::Granja, &u) {
                let urgencia = if activas == 0 { 100 } else { (razon(consumo_objetivo - produccion, consumo_objetivo.max(1)) / 10_000).clamp(0, 100) };
                candidatos.push(Candidato { tipo: Tipo::Granja, score: c.bandas.supervivencia + urgencia, fuente });
            }
        }

        // 2. Leñera.
        if (self.edificios.activos(Tipo::Lenera) as u32) < c.extraccion_maxima_por_tipo && !self.pendiente(Tipo::Lenera) && self.puede_pagar(r, x, Tipo::Lenera, reserva) {
            if let Some(fuente) = x.sitios.proponer(Tipo::Lenera, &u) {
                let urgencia = if self.edificios.activos(Tipo::Lenera) == 0 {
                    100
                } else if self.almacen.cantidad(crate::recurso::Recurso::Madera) < reserva[crate::recurso::Recurso::Madera.indice()] {
                    90
                } else {
                    30
                };
                candidatos.push(Candidato { tipo: Tipo::Lenera, score: c.bandas.supervivencia + urgencia, fuente });
            }
        }

        // 3. Extractores internos: la primera Cantera (sin piedra no hay ascenso al nivel 2) y el Corral.
        for tipo in [Tipo::Cantera, Tipo::Corral] {
            let existentes = self.edificios.en_cualquier_estado(tipo) as u32;
            let necesita = if tipo == Tipo::Cantera { existentes == 0 } else { existentes < c.extraccion_maxima_por_tipo };
            if !necesita || self.pendiente(tipo) || !self.puede_pagar(r, x, tipo, reserva) {
                continue;
            }
            if let Some(fuente) = x.sitios.proponer(tipo, &u) {
                let sin_cupo = self.extractor_minutos_sin_cupo.iter().find(|(t, _)| *t == tipo).map_or(0, |p| p.1);
                let urgencia = if existentes > 0 { 40 } else { 100 } + (sin_cupo * c.extractor_bonus_por_minuto).min(c.extractor_bonus_maximo);
                candidatos.push(Candidato { tipo, score: c.bandas.extractor_base + urgencia, fuente });
                propuestos_extractor.push(tipo);
            }
        }

        // 4. Vivienda: cuando la ocupación llega al umbral (o no hay cupo de pesants) y no se alcanzó el tope del nivel.
        let ocup = |hab: Mil, cap: Mil| if cap <= 0 { UNO } else { razon(hab, cap) };
        let ocupacion_viviendas = ocup(pop.pesants, cap_p).max(ocup(pop.artesanos, cap_a));
        if (cap_p == 0 || ocupacion_viviendas >= n.umbral_vivienda_ocupada)
            && !self.pendiente(Tipo::Vivienda)
            && (self.edificios.en_cualquier_estado(Tipo::Vivienda) as u32) < self.maximo_viviendas(r)
            && self.puede_pagar(r, x, Tipo::Vivienda, reserva)
        {
            if let Some(fuente) = x.sitios.proponer(Tipo::Vivienda, &u) {
                candidatos.push(Candidato { tipo: Tipo::Vivienda, score: c.bandas.crecimiento + ocupacion_viviendas / 10_000, fuente });
            }
        }

        // 5. Granero: el trigo pasa el umbral de ocupación y todavía no hay (uno por asentamiento).
        let trigo = crate::recurso::Recurso::Trigo;
        let ocupacion_trigo = if self.almacen.capacidad(trigo) > 0 { razon(self.almacen.cantidad(trigo), self.almacen.capacidad(trigo)) } else { 0 };
        if ocupacion_trigo >= n.umbral_almacen_ampliacion && self.edificios.en_cualquier_estado(Tipo::Granero) == 0 && self.puede_pagar(r, x, Tipo::Granero, reserva) {
            if let Some(fuente) = x.sitios.proponer(Tipo::Granero, &u) {
                candidatos.push(Candidato { tipo: Tipo::Granero, score: c.bandas.crecimiento + ocupacion_trigo / 10_000, fuente });
            }
        }

        // 6. Almacén: la mayor ocupación de cualquier recurso pasa el umbral.
        let ocupacion = self.ocupacion_maxima();
        let tope_almacenes = n.maximo_almacenes_por_nivel.get(&self.nivel_actual()).is_some_and(|&m| self.edificios.en_cualquier_estado(Tipo::Almacen) as u32 >= m);
        if ocupacion >= n.umbral_almacen_ampliacion && !self.pendiente(Tipo::Almacen) && !tope_almacenes && self.puede_pagar(r, x, Tipo::Almacen, reserva) {
            if let Some(fuente) = x.sitios.proponer(Tipo::Almacen, &u) {
                candidatos.push(Candidato { tipo: Tipo::Almacen, score: c.bandas.crecimiento + ocupacion / 10_000, fuente });
            }
        }

        // 7. Curtiduría / Armería / Fundición: una en vuelo de las tres y solo una propuesta por pasada.
        let base_fundicion = r.catalogo.tipo(Tipo::Fundicion).nivel_construccion;
        let tres = [Tipo::Curtiduria, Tipo::Armeria, Tipo::Fundicion];
        if self.nivel_actual() >= base_fundicion && !tres.iter().any(|&t| self.pendiente(t)) {
            for tipo in tres {
                if self.alcanzo_tope_transformacion(r, tipo, 0) || !self.tiene_insumo_de_arranque(r, tipo) || !self.puede_pagar(r, x, tipo, reserva) {
                    continue;
                }
                if let Some(fuente) = x.sitios.proponer(tipo, &u) {
                    candidatos.push(Candidato { tipo, score: c.bandas.transformacion, fuente });
                    break;
                }
            }
        }

        // 8. Carpintería.
        if self.nivel_actual() >= r.catalogo.tipo(Tipo::Carpinteria).nivel_construccion
            && !self.alcanzo_tope_transformacion(r, Tipo::Carpinteria, 0)
            && !self.pendiente(Tipo::Carpinteria)
            && self.puede_pagar(r, x, Tipo::Carpinteria, reserva)
        {
            if let Some(fuente) = x.sitios.proponer(Tipo::Carpinteria, &u) {
                candidatos.push(Candidato { tipo: Tipo::Carpinteria, score: c.bandas.transformacion, fuente });
            }
        }

        // Commit único: por score descendente (estable: a igual score, el orden de propuesta), mientras haya cupo de cola. Las
        // reconstrucciones dañadas no cuentan contra el cupo (Doc 4.2).
        candidatos.sort_by_key(|c| std::cmp::Reverse(c.score));
        let mut cupo = n.maximo_en_cola.saturating_sub(self.edificios.iter().filter(|e| e.estado == Estado::EnCola && !e.danado).count());
        let mut comprometidos: Vec<Tipo> = vec![];
        for cand in candidatos {
            if cupo == 0 {
                break;
            }
            let def = r.catalogo.tipo(cand.tipo);
            let en_este_commit = comprometidos.iter().filter(|&&t| t == cand.tipo).count();
            if matches!(cand.tipo, Tipo::Fundicion | Tipo::Curtiduria | Tipo::Armeria | Tipo::Carpinteria) && self.alcanzo_tope_transformacion(r, cand.tipo, en_este_commit) {
                continue;
            }
            if !self.puede_iniciar(r, cand.tipo, &def.costo, reserva) {
                continue;
            }
            // El sitio se toma antes de pagar: otro proyecto de esta misma pasada puede habérselo quedado (entonces no se cobra).
            let id = self.siguiente_edificio;
            self.siguiente_edificio += 1;
            let u = self.urbano(r);
            let Some(gratuitos) = x.sitios.tomar(id, cand.tipo, cand.fuente, &u, &mut self.siguiente_edificio) else {
                self.siguiente_edificio = id;
                continue;
            };
            self.almacen.pagar(&def.costo);
            self.alta_gratuitos(gratuitos, ev);
            let mut e = Edificio::nuevo(id, cand.tipo, Estado::EnCola);
            e.prioridad = cand.score;
            e.fuente = cand.fuente;
            self.edificios.push(e);
            ev.push(Evento::NecesidadDetectada { id, tipo: cand.tipo, prioridad: cand.score });
            comprometidos.push(cand.tipo);
            cupo -= 1;
        }
        for tipo in propuestos_extractor {
            let entrada = self.extractor_minutos_sin_cupo.iter_mut().find(|(t, _)| *t == tipo);
            let comprometido = comprometidos.contains(&tipo);
            match entrada {
                Some(p) => p.1 = if comprometido { 0 } else { p.1 + 1 },
                None if !comprometido => self.extractor_minutos_sin_cupo.push((tipo, 1)),
                None => {}
            }
        }
    }
}
