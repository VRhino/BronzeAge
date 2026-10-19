//! Ascenso de nivel de asentamiento (Doc 4.5, Economia_Construccion §11): lo pide el Gobernador, de uno en uno, con una obra de
//! ascenso que se paga entera al empezar. Hacen falta a la vez los requisitos, el cupo de la Facción, el coste en el almacén y la
//! solvencia en el nivel nuevo. Todos los bloqueos se informan juntos.

use crate::asentamiento::{Ascenso, Asentamiento, Evento, Exterior};
use crate::catalogo::Tipo;
use crate::datos::{por, Reglas};
use crate::mantenimiento;
use crate::nivel::Falta;
use crate::produccion;
use crate::recurso::{Costo, Recurso};
use base::{Instante, Mil, MS_POR_MINUTO};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bloqueo {
    NivelMaximo,
    AscensoEnCurso,
    FaltaPoblacion,
    FaltanEdificios,
    SinCupoDeFaccion,
    RecursosInsuficientes,
    /// Lo que se produce y recauda por minuto no cubre el mantenimiento del nivel nuevo: los recursos que faltan.
    Insolvente(Vec<Recurso>),
}

impl Asentamiento {
    /// Ingreso bruto por minuto de un recurso (producción de los edificios y, para el oro, la recaudación). Es bruto a propósito: no
    /// descuenta lo que gasten recetas o tropas. Lo que llega por acarreo cuenta como la producción de los pueblos con carro (Doc 4.5; la
    /// otra forma que admite el documento, la media de las llegadas en 24 h, no se guarda).
    fn ingreso_bruto(&self, r: &Reglas, x: &Exterior, recurso: Recurso) -> Mil {
        let (p, a, _) = self.poblacion.enteros();
        let mano = produccion::mano_de_obra(&r.catalogo, &self.edificios, p, a, self.trabajadores_de_pueblos(r));
        let mut total = self.acarreo_por_minuto(r, &mano, x.factor_tecnologia, recurso);
        for e in self.edificios.iter().filter(|e| e.activo()) {
            let def = r.catalogo.tipo(e.tipo);
            match (e.tipo, def.produccion) {
                (Tipo::Granja, _) if recurso == Recurso::Trigo => {
                    let base = def.nivel(e.nivel_interno).map_or(0, |n| n.produccion_base_trigo);
                    total += por(por(por(base, x.fertilidad), mano.pesants), self.politicas.factor(&r.politicas, "factorProduccionTrigo"));
                }
                (t, Some((rec, base))) if t.usa_pesants() && t != Tipo::Granja && rec == recurso => {
                    total += por(por(base, mano.pesants), (x.factor_tecnologia)(rec));
                }
                _ => {}
            }
        }
        if recurso == Recurso::Oro {
            let entorno = self.entorno_poblacion(r, x);
            total += self.poblacion.recaudacion_por_minuto(&r.poblacion, &entorno);
        }
        total
    }

    /// Mantenimiento que pagaría en el nivel `objetivo` (con ese nivel como operativo).
    fn mantenimiento_en(&self, r: &Reglas, x: &Exterior, objetivo: u32) -> Costo {
        mantenimiento::coste(&r.mantenimiento, &self.edificios, self.pueblos_activos(), objetivo, x.distancia_capital, &vec![])
    }

    /// Todos los bloqueos para pedir la subida al nivel siguiente (vacío = se puede). `cupo_ocupado` es cuántos asentamientos de la
    /// Facción ocupan ya plaza en el nivel objetivo (con ese nivel o con una obra hacia él).
    pub fn bloqueos_de_ascenso(&self, r: &Reglas, x: &Exterior, cupo_ocupado: u32) -> Vec<Bloqueo> {
        let objetivo = self.nivel + 1;
        let mut b = vec![];
        if self.nivel >= r.nivel.nivel_maximo || objetivo > r.nivel.nivel_techo_provisional {
            b.push(Bloqueo::NivelMaximo);
        }
        if self.ascenso.is_some() {
            b.push(Bloqueo::AscensoEnCurso);
        }
        let (pesants, artesanos, _) = self.poblacion.redondeados();
        for f in r.nivel.gates(objetivo, &self.edificios, pesants, artesanos, x.recinto_nivel).unwrap_or_default() {
            b.push(match f {
                Falta::Poblacion => Bloqueo::FaltaPoblacion,
                Falta::Edificios => Bloqueo::FaltanEdificios,
            });
        }
        if r.nivel.cupo_faccion(objetivo, x.nivel_faccion).is_some_and(|cupo| cupo_ocupado >= cupo) {
            b.push(Bloqueo::SinCupoDeFaccion);
        }
        if let Some(a) = r.nivel.ascenso.get(&objetivo) {
            if !self.almacen.tiene(&a.costo) {
                b.push(Bloqueo::RecursosInsuficientes);
            }
        }
        let faltan: Vec<Recurso> = self
            .mantenimiento_en(r, x, objetivo)
            .iter()
            .filter(|&&(rec, coste)| self.ingreso_bruto(r, x, rec) < coste)
            .map(|&(rec, _)| rec)
            .collect();
        if !faltan.is_empty() {
            b.push(Bloqueo::Insolvente(faltan));
        }
        b
    }

    /// `iniciarAscenso`: si no hay bloqueos, cobra el coste entero del almacén y fija la obra.
    pub fn solicitar_ascenso(&mut self, t: Instante, r: &Reglas, x: &Exterior, cupo_ocupado: u32, ev: &mut Vec<Evento>) -> Result<(), Vec<Bloqueo>> {
        let bloqueos = self.bloqueos_de_ascenso(r, x, cupo_ocupado);
        if !bloqueos.is_empty() {
            return Err(bloqueos);
        }
        let objetivo = self.nivel + 1;
        let a = &r.nivel.ascenso[&objetivo];
        self.almacen.pagar(&a.costo);
        self.ascenso = Some(Ascenso { nivel_objetivo: objetivo, iniciado_en: t, completa_en: t + a.obra_minutos * MS_POR_MINUTO });
        ev.push(Evento::AscensoIniciado { objetivo });
        Ok(())
    }

    /// `avanzarAscenso`: al vencer, sube el nivel alcanzado y, solo si no estaba degradado, también el operativo.
    pub(crate) fn terminar_ascenso(&mut self, t: Instante, ev: &mut Vec<Evento>) {
        let Some(a) = self.ascenso else { return };
        if t < a.completa_en {
            return;
        }
        if self.medidor.nivel_actual == self.nivel {
            self.medidor.nivel_actual = a.nivel_objetivo;
        }
        self.nivel = a.nivel_objetivo;
        self.ascenso = None;
        ev.push(Evento::NivelSubio { nivel: a.nivel_objetivo });
    }
}
