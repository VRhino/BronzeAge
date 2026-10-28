//! Nivel de asentamiento (Doc 4.5, Economia_Construccion §11): requisitos de población y edificios, techo de población, obra de
//! ascenso y cupo de la Facción. Solo reglas: el estado (nivel alcanzado, obra en curso) vive en `Asentamiento`.

use crate::catalogo::Tipo;
use crate::datos::mil;
use crate::edificio::{Edificio, Edificios};
use crate::recurso::{Costo, Recurso};
use base::Mil;
use serde_json::Value;
use std::collections::BTreeMap;

/// Viviendas de más que el tope del nivel (Economia_Construccion §6.5): sin ellas el cupo de artesanos del nivel 2 (40 × 10) es
/// exactamente el requisito del nivel 3 (400) y la población, que se acerca al cupo sin tocarlo, no llegaría.
pub const MARGEN_VIVIENDAS: u32 = 2;

#[derive(Clone, Debug, Default)]
pub struct Requisito {
    pub pesants: i64,
    pub artesanos: i64,
    pub edificios: Vec<Tipo>,
    /// Cuántos de `edificios` hacen falta (por defecto, todos).
    pub edificios_minimo: Option<usize>,
    /// Nivel mínimo del recinto de muralla (2 = de piedra), con integridad completa.
    pub recinto_nivel_minimo: Option<u32>,
    pub nivel_interno_minimo: BTreeMap<Tipo, u8>,
    /// Edificios que hacen falta además de los de `edificios`, cada uno activo y con al menos ese nivel interno (`edificiosConNivel`; el nivel 2 pide una Granja a
    /// nivel 2, decisión del autor del 2026-10-28).
    pub con_nivel: BTreeMap<Tipo, u8>,
    /// Aldeas agrícolas activas que pide (`aldeasAgricolas`; el nivel 2 pide una).
    pub aldeas: u32,
}

#[derive(Clone, Debug)]
pub struct CostoAscenso {
    pub costo: Costo,
    pub obra_minutos: i64,
}

#[derive(Clone, Debug)]
pub struct ReglasNivel {
    pub nivel_maximo: u32,
    pub requisitos: BTreeMap<u32, Requisito>,
    /// Techo de población total por nivel alcanzado (milésimas de habitante).
    pub techo_poblacion: BTreeMap<u32, Mil>,
    pub ascenso: BTreeMap<u32, CostoAscenso>,
    pub nivel_techo_provisional: u32,
}

/// Qué falta para cumplir los requisitos de un nivel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Falta {
    Poblacion,
    Edificios,
    /// Falta la aldea agrícola activa que pide el nivel.
    Aldea,
}

impl ReglasNivel {
    pub fn de(nivel: &Value, ascenso: &Value) -> Self {
        let por_nivel = |v: &Value| -> Vec<(u32, Value)> { v.as_object().expect("objeto por nivel").iter().map(|(k, x)| (k.parse().expect("nivel"), x.clone())).collect() };
        let niveles = |v: &Value| -> BTreeMap<Tipo, u8> { v.as_object().map(|o| o.iter().filter_map(|(k, x)| Some((Tipo::desde_nombre(k)?, x.as_f64()? as u8))).collect()).unwrap_or_default() };
        let tipos = |v: &Value| -> Vec<Tipo> { v.as_array().map(|a| a.iter().filter_map(|x| Tipo::desde_nombre(x.as_str()?)).collect()).unwrap_or_default() };
        let requisitos = por_nivel(&nivel["requisitos"])
            .into_iter()
            .map(|(n, v)| {
                let r = Requisito {
                    pesants: v["pesants"].as_f64().unwrap_or(0.0) as i64,
                    artesanos: v["artesanos"].as_f64().unwrap_or(0.0) as i64,
                    edificios: tipos(&v["edificios"]),
                    edificios_minimo: v["edificiosMinimo"].as_f64().map(|x| x as usize),
                    recinto_nivel_minimo: v["recintoCompletoNivelMinimo"].as_f64().map(|x| x as u32),
                    nivel_interno_minimo: niveles(&v["nivelInternoMinimo"]),
                    con_nivel: niveles(&v["edificiosConNivel"]),
                    aldeas: v["aldeasAgricolas"].as_f64().unwrap_or(0.0) as u32,
                };
                (n, r)
            })
            .collect();
        let costo = |v: &Value| -> Costo { v.as_object().expect("costo").iter().map(|(k, x)| (Recurso::desde_nombre(k).expect("recurso"), mil(x.as_f64().expect("cantidad")))).collect() };
        ReglasNivel {
            nivel_maximo: nivel["nivelMaximo"].as_f64().expect("nivelMaximo") as u32,
            requisitos,
            techo_poblacion: por_nivel(&nivel["techoPoblacion"]).into_iter().map(|(n, v)| (n, mil(v.as_f64().expect("techo")))).collect(),
            ascenso: por_nivel(&ascenso["porNivelObjetivo"])
                .into_iter()
                .map(|(n, v)| (n, CostoAscenso { costo: costo(&v["costo"]), obra_minutos: v["obraMinutos"].as_f64().expect("obraMinutos") as i64 }))
                .collect(),
            nivel_techo_provisional: ascenso["nivelTechoProvisional"].as_f64().expect("nivelTechoProvisional") as u32,
        }
    }

    /// Techo de población del nivel alcanzado.
    pub fn techo(&self, nivel: u32) -> Mil {
        self.techo_poblacion.get(&nivel).copied().unwrap_or(0)
    }

    /// `evaluarGatesDeNivel`: lo que falta para el nivel objetivo (vacío = se cumple). `None` si ese nivel no tiene requisito.
    /// `recinto_nivel` es el nivel del mejor recinto completo (0 si no hay); `aldeas`, las aldeas agrícolas activas de la plaza.
    pub fn gates(&self, objetivo: u32, edificios: &[Edificio], pesants: i64, artesanos: i64, recinto_nivel: u32, aldeas: u32) -> Option<Vec<Falta>> {
        let req = self.requisitos.get(&objetivo)?;
        let mut faltas = vec![];
        if pesants < req.pesants || artesanos < req.artesanos {
            faltas.push(Falta::Poblacion);
        }
        let construidos = req.edificios.iter().filter(|&&t| edificios.hay_activo_con_nivel(t, req.nivel_interno_minimo.get(&t).copied().unwrap_or(1))).count();
        let cumple_recinto = req.recinto_nivel_minimo.map_or(true, |m| recinto_nivel >= m);
        let cumple_con_nivel = req.con_nivel.iter().all(|(&t, &n)| edificios.hay_activo_con_nivel(t, n));
        if construidos < req.edificios_minimo.unwrap_or(req.edificios.len()) || !cumple_recinto || !cumple_con_nivel {
            faltas.push(Falta::Edificios);
        }
        if aldeas < req.aldeas {
            faltas.push(Falta::Aldea);
        }
        Some(faltas)
    }

    /// `maximoViviendasPorNivel(n)` (cupos por Vivienda en habitantes enteros): las que da el nivel siguiente (mayor de los cupos de pesants y artesanos que exige)
    /// más un margen de `MARGEN_VIVIENDAS` (decidido el 2026-10-18: con el cupo justo, una clase nunca llega al requisito), o, en el
    /// nivel máximo, las que llenan el techo de población. Más Viviendas de las necesarias no sirven hasta subir.
    pub fn maximo_viviendas(&self, nivel: u32, capacidad_pesants: i64, capacidad_artesanos: i64) -> u32 {
        let techo_alcanza = |hab: i64, cap: i64| if cap <= 0 { 0 } else { (hab + cap - 1) / cap };
        match self.requisitos.get(&(nivel + 1)) {
            Some(r) => techo_alcanza(r.pesants, capacidad_pesants).max(techo_alcanza(r.artesanos, capacidad_artesanos)) as u32 + MARGEN_VIVIENDAS,
            None => techo_alcanza(self.techo(nivel) / 1000, capacidad_pesants) as u32,
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::Reglas;
    use crate::edificio::Estado;

    fn activos(tipos: &[(Tipo, u8)]) -> Vec<Edificio> {
        tipos
            .iter()
            .enumerate()
            .map(|(i, &(t, n))| {
                let mut e = Edificio::nuevo(i as u32, t, Estado::Activo);
                e.nivel_interno = n;
                e
            })
            .collect()
    }

    #[test]
    fn requisitos_techos_y_costes_de_ascenso_son_los_del_canon() {
        let r = Reglas::cargar().nivel;
        assert_eq!((r.requisitos[&2].pesants, r.requisitos[&3].artesanos, r.requisitos[&4].recinto_nivel_minimo), (200, 400, Some(2)));
        assert_eq!((r.techo(1), r.techo(3), r.techo(5)), (300_000, 6_000_000, 20_000_000));
        let a2 = &r.ascenso[&2];
        assert_eq!((a2.obra_minutos, a2.costo.clone()), (4320, vec![(Recurso::Madera, 600_000), (Recurso::Oro, 150_000), (Recurso::Piedra, 400_000)]));
        assert_eq!(r.ascenso[&3].obra_minutos, 10_080);
        assert_eq!(r.nivel_techo_provisional, 4);
    }

    #[test]
    fn el_nivel_2_pide_200_pesants_dos_de_cantera_lenera_y_granja_una_granja_a_nivel_2_y_una_aldea_agricola() {
        let r = Reglas::cargar().nivel;
        let uno = activos(&[(Tipo::Cantera, 1), (Tipo::Corral, 1), (Tipo::Mina, 1)]);
        assert_eq!(r.gates(2, &uno, 200, 0, 0, 1), Some(vec![Falta::Edificios]), "ni el Corral ni las minas cuentan ya");
        let dos = activos(&[(Tipo::Cantera, 1), (Tipo::Granja, 1), (Tipo::Granja, 2)]);
        assert_eq!(r.gates(2, &dos, 200, 0, 0, 1), Some(vec![]), "dos Granjas cuentan como un solo tipo");
        // Además de los dos tipos, una Granja a nivel 2 (2026-10-28): Cantera y Leñera con Granjas a nivel 1 no bastan.
        let sin_granja_2 = activos(&[(Tipo::Cantera, 1), (Tipo::Lenera, 1), (Tipo::Granja, 1)]);
        assert_eq!(r.gates(2, &sin_granja_2, 200, 0, 0, 1), Some(vec![Falta::Edificios]));
        assert_eq!(r.gates(2, &activos(&[(Tipo::Cantera, 1), (Tipo::Lenera, 1)]), 200, 0, 0, 1), Some(vec![Falta::Edificios]));
        assert_eq!(r.gates(2, &activos(&[(Tipo::Cantera, 1), (Tipo::Lenera, 1), (Tipo::Granja, 2)]), 200, 0, 0, 1), Some(vec![]));
        assert_eq!(r.gates(2, &dos, 200, 0, 0, 0), Some(vec![Falta::Aldea]));
        assert_eq!(r.gates(2, &dos, 199, 0, 0, 1), Some(vec![Falta::Poblacion]));
        assert_eq!(r.gates(9, &dos, 0, 0, 0, 0), None);
        assert_eq!(r.requisitos[&3].aldeas, 0, "solo el nivel 2 pide la aldea");
    }

    #[test]
    fn el_nivel_4_pide_un_recinto_de_piedra_y_el_5_el_palacio_a_nivel_3() {
        let r = Reglas::cargar().nivel;
        let e = activos(&[(Tipo::SalaConsejo, 1)]);
        assert_eq!(r.gates(4, &e, 1000, 800, 1, 0), Some(vec![Falta::Edificios]), "la empalizada (nivel 1) no basta");
        assert_eq!(r.gates(4, &e, 1000, 800, 2, 0), Some(vec![]));
        let p = activos(&[(Tipo::Palacio, 2)]);
        assert_eq!(r.gates(5, &p, 2000, 1600, 0, 0), Some(vec![Falta::Edificios]));
        assert_eq!(r.gates(5, &activos(&[(Tipo::Palacio, 3)]), 2000, 1600, 0, 0), Some(vec![]));
    }

    #[test]
    fn el_tope_de_viviendas_sale_del_nivel_siguiente() {
        let r = Reglas::cargar().nivel;
        // Nivel 1 → pide 200 pesants (15 por Vivienda → 14) y 0 artesanos.
        assert_eq!(r.maximo_viviendas(1, 15, 10), 14 + MARGEN_VIVIENDAS);
        // Nivel 2 → 500 pesants (34) y 400 artesanos (40): el mayor.
        assert_eq!(r.maximo_viviendas(2, 15, 10), 40 + MARGEN_VIVIENDAS, "cupo de 420 artesanos para un requisito de 400");
        // Nivel 5, el máximo: las que llenan el techo de 20.000.
        assert_eq!(r.maximo_viviendas(5, 15, 10), 1334, "en el nivel máximo no hay margen: solo llenan el techo");
    }
}
