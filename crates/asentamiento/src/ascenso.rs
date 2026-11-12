//! Ascenso de nivel de asentamiento (Doc 4.5, Economia_Construccion §11): lo pide el Gobernador, de uno en uno, con una obra de
//! ascenso que se paga entera al empezar. Hacen falta a la vez los requisitos, el cupo de la Facción y el coste en el almacén. Todos los
//! bloqueos se informan juntos. La solvencia en el nivel nuevo ya no bloquea (autor, 2026-11-10: una plaza que vive del comercio no
//! podría crecer): es una advertencia, [`Asentamiento::insolvencia_de_ascenso`].

use crate::asentamiento::{Ascenso, Asentamiento, Evento, Exterior};
use crate::catalogo::Tipo;
use crate::datos::{por, Reglas};
use crate::mantenimiento;
use crate::nivel::Falta;
use crate::produccion;
use crate::recintos;
use crate::recurso::{Costo, Recurso};
use base::{Instante, Mil, MS_POR_MINUTO};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bloqueo {
    NivelMaximo,
    AscensoEnCurso,
    FaltaPoblacion,
    FaltanEdificios,
    /// Falta el pueblo activo (aldea agrícola o extractor) que pide el nivel (el 2).
    FaltaPueblo,
    SinCupoDeFaccion,
    RecursosInsuficientes,
}

/// Un recurso del mantenimiento del nivel nuevo que lo que llega por minuto no cubre (en milésimas por minuto).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Insolvencia {
    pub recurso: Recurso,
    /// El ingreso bruto por minuto, con el acarreo de media desde que llega.
    pub llega: Mil,
    /// El mantenimiento por minuto en el nivel nuevo.
    pub pide: Mil,
}

impl Asentamiento {
    /// Ingreso bruto por minuto de un recurso (producción de los edificios y, para el oro, la recaudación). Es bruto a propósito: no
    /// descuenta lo que gasten recetas o tropas. Lo que llega por acarreo cuenta como la media de lo que **ingresó** en el almacén en las
    /// últimas 24 h (Doc 4.5, decidido el 2026-10-19): con el almacén lleno un pueblo no hace solvente a nadie. `t` es el instante de la pregunta.
    fn ingreso_bruto(&self, t: Instante, r: &Reglas, x: &Exterior, recurso: Recurso) -> Mil {
        let (p, a, _) = self.poblacion.enteros();
        let mano = produccion::mano_de_obra(&r.catalogo, &self.edificios, p, a, self.trabajadores_de_pueblos(r));
        let mut total = self.acarreo_por_minuto(t, recurso);
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
        mantenimiento::coste(&r.mantenimiento, &self.edificios, self.elementos_de_mapa_activos(), objetivo, x.distancia_capital, &recintos::upkeep(&r.muralla, &self.recintos))
    }

    /// Todos los bloqueos para pedir la subida al nivel siguiente (vacío = se puede). `sin_cupo_de_faccion`: la Facción ya ocupa todo su cupo del nivel objetivo
    /// (`CUPO_NIVEL_ASENTAMIENTO`, que da el nivel de Facción: lo calcula quien conoce la Facción, `politica::nivel::cupo_nivel`, desde 5C).
    pub fn bloqueos_de_ascenso(&self, r: &Reglas, sin_cupo_de_faccion: bool) -> Vec<Bloqueo> {
        let objetivo = self.nivel + 1;
        let mut b = vec![];
        if self.nivel >= r.nivel.nivel_maximo {
            b.push(Bloqueo::NivelMaximo);
        }
        if self.ascenso.is_some() {
            b.push(Bloqueo::AscensoEnCurso);
        }
        let (pesants, artesanos, _) = self.poblacion.enteros();
        for f in r.nivel.gates(objetivo, &self.edificios, pesants, artesanos, recintos::nivel_del_mejor_completo(&self.recintos), self.pueblos_activos() as u32).unwrap_or_default() {
            b.push(match f {
                Falta::Poblacion => Bloqueo::FaltaPoblacion,
                Falta::Edificios => Bloqueo::FaltanEdificios,
                Falta::Pueblo => Bloqueo::FaltaPueblo,
            });
        }
        if sin_cupo_de_faccion {
            b.push(Bloqueo::SinCupoDeFaccion);
        }
        if let Some(a) = r.nivel.ascenso.get(&objetivo) {
            if !self.almacen.tiene(&a.costo) {
                b.push(Bloqueo::RecursosInsuficientes);
            }
        }
        b
    }

    /// La advertencia de solvencia del nivel siguiente (autor, 2026-11-10; hasta entonces era el bloqueo `ascenso.insolvente`): los recursos
    /// del mantenimiento de ese nivel que lo que llega por minuto ([`Self::ingreso_bruto`], con el acarreo de media) no cubre, con lo que llega
    /// y lo que pide. No impide subir; vacía en el nivel máximo.
    pub fn insolvencia_de_ascenso(&self, t: Instante, r: &Reglas, x: &Exterior) -> Vec<Insolvencia> {
        let objetivo = self.nivel + 1;
        if self.nivel >= r.nivel.nivel_maximo {
            return vec![];
        }
        self.mantenimiento_en(r, x, objetivo)
            .iter()
            .map(|&(recurso, pide)| Insolvencia { recurso, llega: self.ingreso_bruto(t, r, x, recurso), pide })
            .filter(|f| f.llega < f.pide)
            .collect()
    }

    /// `iniciarAscenso`: si no hay bloqueos, cobra el coste entero del almacén y fija la obra.
    pub fn solicitar_ascenso(&mut self, t: Instante, r: &Reglas, sin_cupo_de_faccion: bool, ev: &mut Vec<Evento>) -> Result<(), Vec<Bloqueo>> {
        let bloqueos = self.bloqueos_de_ascenso(r, sin_cupo_de_faccion);
        if !bloqueos.is_empty() {
            return Err(bloqueos);
        }
        let objetivo = self.nivel + 1;
        let a = &r.nivel.ascenso[&objetivo];
        self.almacen.pagar(&a.costo);
        let completa_en = t + a.obra_minutos * MS_POR_MINUTO;
        self.ascenso = Some(Ascenso { nivel_objetivo: objetivo, iniciado_en: t, completa_en });
        ev.push(Evento::AscensoIniciado { objetivo, completa_en });
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
