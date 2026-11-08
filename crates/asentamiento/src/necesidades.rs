//! Auto-construcción por necesidad (Doc 4.2, Economia_Construccion §6.2): el asentamiento decide solo qué construir según lo que
//! le falta (comida, madera, Viviendas, almacén, talleres). Cada candidato recibe un score = base de su banda + urgencia, y se
//! compromete (se paga y se encola) en orden de score mientras haya cupo y fondos. Nunca elige dónde: eso es del trazado.
//!
//! Cambios de la reconstrucción respecto a TypeScript, ya decididos: los minerales y la Cantera van por pueblo extractor (Doc 4.2.2; la
//! primera Cantera también desde el 2026-11-01) y el Corral es de las aldeas agrícolas (§14.2), así que la cola automática no propone
//! ningún extractor; tampoco una Granja (2026-11-02: la ciudad solo tiene la de la fundación); un candidato que no se compromete no deja rastro.

use crate::asentamiento::{Asentamiento, Evento, Exterior};
use crate::catalogo::Tipo;
use crate::datos::{razon, Reglas, UNO};
use crate::edificio::{Edificio, Edificios, Estado, Fuente};
use crate::recurso::N;
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
    fn puede_pagar(&self, r: &Reglas, x: &Exterior, tipo: Tipo, reserva: &[Mil; N]) -> bool {
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

    pub(crate) fn evaluar_necesidades(&mut self, _t: Instante, x: &mut Exterior, reserva: &[Mil; N], ev: &mut Vec<Evento>) {
        let r = x.reglas;
        let c = &r.construccion;
        let n = &c.necesidades;
        let mut candidatos: Vec<Candidato> = vec![];
        // Lo que se comprometa en esta pasada se confirma al final; preguntar solo cambia la ciudad si el trazado crea un ancla (Trazado_Urbano §7.1).
        let u = self.urbano(r);
        let (cap_p, cap_a, _) = self.capacidades(r);
        let (hab_p, hab_a) = (self.poblacion.pesants * 1000, self.poblacion.artesanos * 1000);

        // 1. Granja: ninguna (2026-11-02, decisión del autor). La ciudad tiene solo la de la fundación, que se mejora como cualquier edificio; el trigo de más, de las aldeas
        //    agrícolas.

        // 2. Leñera.
        if (self.edificios.activos(Tipo::Lenera) as u32) < c.extraccion_maxima_por_tipo && !self.pendiente(Tipo::Lenera) && self.puede_pagar(r, x, Tipo::Lenera, reserva) {
            if let Some(fuente) = self.proponer(x, Tipo::Lenera, &u, ev) {
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

        // 3. Extractores: ninguno. La Cantera y las minas son de pueblo extractor y el Corral, de las aldeas agrícolas (2026-11-01).

        // 4. Vivienda: cuando la ocupación llega al umbral (o no hay cupo de pesants) y no se alcanzó el tope del nivel.
        let ocup = |hab: Mil, cap: Mil| if cap <= 0 { UNO } else { razon(hab, cap) };
        let ocupacion_viviendas = ocup(hab_p, cap_p).max(ocup(hab_a, cap_a));
        if (cap_p == 0 || ocupacion_viviendas >= n.umbral_vivienda_ocupada)
            && !self.pendiente(Tipo::Vivienda)
            && (self.edificios.en_cualquier_estado(Tipo::Vivienda) as u32) < self.maximo_viviendas(r)
            && self.puede_pagar(r, x, Tipo::Vivienda, reserva)
        {
            if let Some(fuente) = self.proponer(x, Tipo::Vivienda, &u, ev) {
                candidatos.push(Candidato { tipo: Tipo::Vivienda, score: c.bandas.crecimiento + ocupacion_viviendas / 10_000, fuente });
            }
        }

        // 5. Granero: el trigo pasa el umbral de ocupación y todavía no hay (uno por asentamiento).
        let trigo = crate::recurso::Recurso::Trigo;
        let ocupacion_trigo = if self.almacen.capacidad(trigo) > 0 { razon(self.almacen.cantidad(trigo), self.almacen.capacidad(trigo)) } else { 0 };
        if ocupacion_trigo >= n.umbral_almacen_ampliacion && self.edificios.en_cualquier_estado(Tipo::Granero) == 0 && self.puede_pagar(r, x, Tipo::Granero, reserva) {
            if let Some(fuente) = self.proponer(x, Tipo::Granero, &u, ev) {
                candidatos.push(Candidato { tipo: Tipo::Granero, score: c.bandas.crecimiento + ocupacion_trigo / 10_000, fuente });
            }
        }

        // 6. Almacén: la mayor ocupación de cualquier recurso pasa el umbral.
        let ocupacion = self.ocupacion_maxima();
        let tope_almacenes = n.maximo_almacenes_por_nivel.get(&self.nivel_actual()).is_some_and(|&m| self.edificios.en_cualquier_estado(Tipo::Almacen) as u32 >= m);
        if ocupacion >= n.umbral_almacen_ampliacion && !self.pendiente(Tipo::Almacen) && !tope_almacenes && self.puede_pagar(r, x, Tipo::Almacen, reserva) {
            if let Some(fuente) = self.proponer(x, Tipo::Almacen, &u, ev) {
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
                if let Some(fuente) = self.proponer(x, tipo, &u, ev) {
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
            if let Some(fuente) = self.proponer(x, Tipo::Carpinteria, &u, ev) {
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
    }
}
