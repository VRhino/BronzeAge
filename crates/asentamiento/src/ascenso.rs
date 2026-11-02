//! Ascenso de nivel de asentamiento (Doc 4.5, Economia_Construccion §11): lo pide el Gobernador, de uno en uno, con una obra de
//! ascenso que se paga entera al empezar. Hacen falta a la vez los requisitos, el cupo de la Facción, el coste en el almacén y la
//! solvencia en el nivel nuevo. Todos los bloqueos se informan juntos.

use crate::asentamiento::{Ascenso, Asentamiento, Evento, Exterior};
use crate::catalogo::Tipo;
use crate::edificio::Fuente;
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
    /// Lo que se produce y recauda por minuto no cubre el mantenimiento del nivel nuevo: los recursos que faltan.
    Insolvente(Vec<Recurso>),
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
                // La Leñera, como al producir: su base por la densidad de su parcela, sin factor de tecnología.
                (Tipo::Lenera, Some((rec, base))) if rec == recurso => {
                    let densidad = match e.fuente {
                        Some(Fuente::Parcela(p)) => x.fuentes.densidad_parcela(p),
                        _ => 0,
                    };
                    total += por(por(base, densidad), mano.pesants);
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
    pub fn bloqueos_de_ascenso(&self, t: Instante, r: &Reglas, x: &Exterior, sin_cupo_de_faccion: bool) -> Vec<Bloqueo> {
        let objetivo = self.nivel + 1;
        let mut b = vec![];
        if self.nivel >= r.nivel.nivel_maximo || objetivo > r.nivel.nivel_techo_provisional {
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
        let faltan: Vec<Recurso> = self
            .mantenimiento_en(r, x, objetivo)
            .iter()
            .filter(|&&(rec, coste)| self.ingreso_bruto(t, r, x, rec) < coste)
            .map(|&(rec, _)| rec)
            .collect();
        if !faltan.is_empty() {
            b.push(Bloqueo::Insolvente(faltan));
        }
        b
    }

    /// `iniciarAscenso`: si no hay bloqueos, cobra el coste entero del almacén y fija la obra.
    pub fn solicitar_ascenso(&mut self, t: Instante, r: &Reglas, x: &Exterior, sin_cupo_de_faccion: bool, ev: &mut Vec<Evento>) -> Result<(), Vec<Bloqueo>> {
        let bloqueos = self.bloqueos_de_ascenso(t, r, x, sin_cupo_de_faccion);
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

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::asentamiento::{Gratuito, Sitios, Urbano};
    use crate::datos::UNO;
    use crate::edificio::{Edificio, Estado};
    use crate::produccion::Fuentes;

    struct Parcela(i64);
    impl Fuentes for Parcela {
        fn densidad_parcela(&self, _: u32) -> i64 {
            self.0
        }
        fn extraer(&mut self, _: u32, _: Mil) -> Mil {
            0
        }
    }

    struct SinSitios;
    impl Sitios for SinSitios {
        fn proponer(&mut self, _: Tipo, _: &Urbano, _: &mut u32, _: &mut Vec<Gratuito>) -> Option<Option<Fuente>> {
            None
        }
        fn tomar(&mut self, _: u32, _: Tipo, _: Option<Fuente>, _: &Urbano, _: &mut u32) -> Option<Vec<Gratuito>> {
            None
        }
    }

    fn madera_por_minuto(r: &Reglas, a: &Asentamiento, densidad: i64, factor_tecnologia: i64) -> Mil {
        let (mut f, mut s) = (Parcela(densidad), SinSitios);
        let tec = move |_| factor_tecnologia;
        let x = Exterior {
            reglas: r,
            fuentes: &mut f,
            sitios: &mut s,
            fertilidad: UNO,
            tiene_tecnologia: &|_| false,
            factor_tecnologia: &tec,
            nivel_faccion: 1,
            orden_era: 1,
            es_capital: true,
            faccion_tiene_gran_fundicion: false,
            ciudadanos: 1,
            distancia_capital: 0,
            racion_guarnicion: 0,
            felicidad_extra: UNO,
            nobleza_extra: UNO,
            recaudacion_extra: UNO,
        };
        a.ingreso_bruto(0, r, &x, Recurso::Madera)
    }

    #[test]
    fn la_solvencia_cuenta_la_lenera_por_la_densidad_de_su_parcela_y_sin_tecnologia() {
        let r = Reglas::cargar();
        let mut a = Asentamiento::fundar(&r, 0, &[], r.viviendas_iniciales);
        a.edificios.retain(|e| e.tipo != Tipo::Lenera);
        let mut l = Edificio::nuevo(900, Tipo::Lenera, Estado::Activo);
        l.fuente = Some(Fuente::Parcela(1));
        a.edificios.push(l);
        let plena = madera_por_minuto(&r, &a, UNO, UNO);
        assert!(plena > 0);
        assert_eq!(madera_por_minuto(&r, &a, 500_000, UNO), plena / 2, "media densidad, media madera, como al producir");
        assert_eq!(madera_por_minuto(&r, &a, 500_000, 2 * UNO), plena / 2, "la tecnología no multiplica la Leñera");
        assert_eq!(madera_por_minuto(&r, &a, 0, UNO), 0, "sin parcela no cuenta");
    }
}
