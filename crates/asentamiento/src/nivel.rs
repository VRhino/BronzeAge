//! Nivel de asentamiento (Doc 4.5, Economia_Construccion §11): requisitos de población y edificios, techo de población, obra de
//! ascenso y cupo de la Facción. Solo reglas: el estado (nivel alcanzado, obra en curso) vive en `Asentamiento`.

use crate::catalogo::Tipo;
use crate::datos::mil;
use crate::edificio::{Edificio, Edificios};
use crate::recurso::{Costo, Recurso};
use base::Mil;
use serde_json::Value;
use std::collections::BTreeMap;

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
    pub cupo_nivel2: Vec<u32>,
    pub cupo_nivel3: Vec<u32>,
}

/// Qué falta para cumplir los requisitos de un nivel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Falta {
    Poblacion,
    Edificios,
}

impl ReglasNivel {
    pub fn de(nivel: &Value, ascenso: &Value, cupo: &Value) -> Self {
        let por_nivel = |v: &Value| -> Vec<(u32, Value)> { v.as_object().expect("objeto por nivel").iter().map(|(k, x)| (k.parse().expect("nivel"), x.clone())).collect() };
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
                    nivel_interno_minimo: v["nivelInternoMinimo"]
                        .as_object()
                        .map(|o| o.iter().filter_map(|(k, x)| Some((Tipo::desde_nombre(k)?, x.as_f64()? as u8))).collect())
                        .unwrap_or_default(),
                };
                (n, r)
            })
            .collect();
        let costo = |v: &Value| -> Costo { v.as_object().expect("costo").iter().map(|(k, x)| (Recurso::desde_nombre(k).expect("recurso"), mil(x.as_f64().expect("cantidad")))).collect() };
        let lista = |v: &Value| -> Vec<u32> { v.as_array().expect("curva").iter().map(|x| x.as_u64().expect("cupo") as u32).collect() };
        ReglasNivel {
            nivel_maximo: nivel["nivelMaximo"].as_f64().expect("nivelMaximo") as u32,
            requisitos,
            techo_poblacion: por_nivel(&nivel["techoPoblacion"]).into_iter().map(|(n, v)| (n, mil(v.as_f64().expect("techo")))).collect(),
            ascenso: por_nivel(&ascenso["porNivelObjetivo"])
                .into_iter()
                .map(|(n, v)| (n, CostoAscenso { costo: costo(&v["costo"]), obra_minutos: v["obraMinutos"].as_f64().expect("obraMinutos") as i64 }))
                .collect(),
            nivel_techo_provisional: ascenso["nivelTechoProvisional"].as_f64().expect("nivelTechoProvisional") as u32,
            cupo_nivel2: lista(&cupo["maxNivel2"]),
            cupo_nivel3: lista(&cupo["maxNivel3"]),
        }
    }

    /// Techo de población del nivel alcanzado.
    pub fn techo(&self, nivel: u32) -> Mil {
        self.techo_poblacion.get(&nivel).copied().unwrap_or(0)
    }

    /// `evaluarGatesDeNivel`: lo que falta para el nivel objetivo (vacío = se cumple). `None` si ese nivel no tiene requisito.
    /// `recinto_nivel` es el nivel del mejor recinto completo (0 si no hay).
    pub fn gates(&self, objetivo: u32, edificios: &[Edificio], pesants: i64, artesanos: i64, recinto_nivel: u32) -> Option<Vec<Falta>> {
        let req = self.requisitos.get(&objetivo)?;
        let mut faltas = vec![];
        if pesants < req.pesants || artesanos < req.artesanos {
            faltas.push(Falta::Poblacion);
        }
        let construidos = req.edificios.iter().filter(|&&t| edificios.hay_activo_con_nivel(t, req.nivel_interno_minimo.get(&t).copied().unwrap_or(1))).count();
        let cumple_recinto = req.recinto_nivel_minimo.map_or(true, |m| recinto_nivel >= m);
        if construidos < req.edificios_minimo.unwrap_or(req.edificios.len()) || !cumple_recinto {
            faltas.push(Falta::Edificios);
        }
        Some(faltas)
    }

    /// Cupo de la Facción para ocupar un nivel (2 y 3); los demás no tienen cupo. `nivel_faccion` se recorta a 1..=10.
    pub fn cupo_faccion(&self, objetivo: u32, nivel_faccion: u32) -> Option<u32> {
        let curva = match objetivo {
            2 => &self.cupo_nivel2,
            3 => &self.cupo_nivel3,
            _ => return None,
        };
        curva.get((nivel_faccion.clamp(1, 10) - 1) as usize).copied()
    }

    /// `maximoViviendasPorNivel(n)` (cupos por Vivienda en habitantes enteros): las que da el nivel siguiente (mayor de los cupos de pesants y artesanos que exige), o, en el
    /// nivel máximo, las que llenan el techo de población. Más Viviendas de las necesarias no sirven hasta subir.
    pub fn maximo_viviendas(&self, nivel: u32, capacidad_pesants: i64, capacidad_artesanos: i64) -> u32 {
        let techo_alcanza = |hab: i64, cap: i64| if cap <= 0 { 0 } else { (hab + cap - 1) / cap };
        match self.requisitos.get(&(nivel + 1)) {
            Some(r) => techo_alcanza(r.pesants, capacidad_pesants).max(techo_alcanza(r.artesanos, capacidad_artesanos)) as u32,
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
    fn el_nivel_2_pide_200_pesants_y_tres_de_los_siete_tipos_de_extraccion() {
        let r = Reglas::cargar().nivel;
        let dos = activos(&[(Tipo::Cantera, 1), (Tipo::Lenera, 1)]);
        assert_eq!(r.gates(2, &dos, 200, 0, 0), Some(vec![Falta::Edificios]));
        let tres = activos(&[(Tipo::Cantera, 1), (Tipo::Lenera, 1), (Tipo::Granja, 1), (Tipo::Granja, 1)]);
        assert_eq!(r.gates(2, &tres, 200, 0, 0), Some(vec![]), "dos Granjas cuentan como un solo tipo");
        assert_eq!(r.gates(2, &tres, 199, 0, 0), Some(vec![Falta::Poblacion]));
        assert_eq!(r.gates(9, &tres, 0, 0, 0), None);
    }

    #[test]
    fn el_nivel_4_pide_un_recinto_de_piedra_y_el_5_el_palacio_a_nivel_3() {
        let r = Reglas::cargar().nivel;
        let e = activos(&[(Tipo::SalaConsejo, 1)]);
        assert_eq!(r.gates(4, &e, 1000, 800, 1), Some(vec![Falta::Edificios]), "la empalizada (nivel 1) no basta");
        assert_eq!(r.gates(4, &e, 1000, 800, 2), Some(vec![]));
        let p = activos(&[(Tipo::Palacio, 2)]);
        assert_eq!(r.gates(5, &p, 2000, 1600, 0), Some(vec![Falta::Edificios]));
        assert_eq!(r.gates(5, &activos(&[(Tipo::Palacio, 3)]), 2000, 1600, 0), Some(vec![]));
    }

    #[test]
    fn el_cupo_de_faccion_solo_existe_para_los_niveles_2_y_3() {
        let r = Reglas::cargar().nivel;
        assert_eq!((r.cupo_faccion(2, 1), r.cupo_faccion(2, 10), r.cupo_faccion(3, 1), r.cupo_faccion(3, 3)), (Some(1), Some(3), Some(0), Some(1)));
        assert_eq!(r.cupo_faccion(4, 5), None);
        assert_eq!(r.cupo_faccion(2, 99), Some(3), "el nivel de Facción se recorta a 10");
    }

    #[test]
    fn el_tope_de_viviendas_sale_del_nivel_siguiente() {
        let r = Reglas::cargar().nivel;
        // Nivel 1 → pide 200 pesants (15 por Vivienda → 14) y 0 artesanos.
        assert_eq!(r.maximo_viviendas(1, 15, 10), 14);
        // Nivel 2 → 500 pesants (34) y 400 artesanos (40): el mayor.
        assert_eq!(r.maximo_viviendas(2, 15, 10), 40);
        // Nivel 5, el máximo: las que llenan el techo de 20.000.
        assert_eq!(r.maximo_viviendas(5, 15, 10), 1334);
    }
}
