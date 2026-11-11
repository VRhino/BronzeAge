//! Las formaciones de una escuadra en una batalla de Unity (Heroe_Y_Tropa §10.4, decisión del autor del 2026-11-11). El motor no sabe cómo es cada
//! formación (la forma, lo apretada que va y dónde va cada soldado son de Conquest, por tropa): solo guarda **qué familias permite la tecnología de
//! la Facción** y **cuál ha elegido el héroe** para cada escuadra, y las congela en el ticket.
//!
//! La primera familia del bloque `FORMACIONES` es la **de reposo**: la que adopta toda tropa al seguir a su héroe o al pararse, siempre disponible.
//! Las familias se nombran en el contrato por su id de cadena y en el estado por su posición en el bloque (solo se añaden al final).

use serde_json::Value;

/// La familia de reposo: la primera del bloque, siempre permitida.
pub const REPOSO: u8 = 0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamiliaFormacion {
    pub id: String,
    pub nombre: String,
    /// La tecnología que la permite; `None` = desde el principio.
    pub tecnologia: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasFormaciones {
    pub familias: Vec<FamiliaFormacion>,
}

impl ReglasFormaciones {
    pub fn de(v: &Value) -> Self {
        let familias: Vec<FamiliaFormacion> = v["familias"]
            .as_array()
            .expect("FORMACIONES.familias")
            .iter()
            .map(|f| FamiliaFormacion {
                id: f["id"].as_str().expect("id de formación").to_owned(),
                nombre: f["nombre"].as_str().expect("nombre de formación").to_owned(),
                tecnologia: f["tecnologia"].as_str().map(str::to_owned),
            })
            .collect();
        assert!(familias.first().is_some_and(|f| f.tecnologia.is_none()), "la primera formación es la de reposo y no pide tecnología");
        assert!(familias.len() <= 32, "caben en una máscara de 32 bits");
        Self { familias }
    }

    /// La posición de una familia por su id.
    pub fn por_id(&self, id: &str) -> Option<u8> {
        self.familias.iter().position(|f| f.id == id).map(|i| i as u8)
    }

    /// El id de una familia (el de reposo si la posición no existe).
    pub fn id(&self, i: u8) -> &str {
        &self.familias.get(i as usize).unwrap_or(&self.familias[REPOSO as usize]).id
    }

    /// ¿La permite quien tiene esas tecnologías? La de reposo y las que no piden tecnología, siempre.
    pub fn permitida(&self, i: u8, tiene: impl Fn(&str) -> bool) -> bool {
        self.familias.get(i as usize).is_some_and(|f| f.tecnologia.as_deref().is_none_or(&tiene))
    }

    /// Las familias que permite quien tiene esas tecnologías, en el orden del bloque (la de reposo, la primera).
    pub fn permitidas(&self, tiene: impl Fn(&str) -> bool) -> Vec<u8> {
        (0..self.familias.len() as u8).filter(|&i| self.permitida(i, &tiene)).collect()
    }

    /// Las mismas como máscara de bits por posición (el bit 0, la de reposo).
    pub fn mascara(&self, tiene: impl Fn(&str) -> bool) -> u32 {
        self.permitidas(tiene).into_iter().fold(0, |m, i| m | 1 << i)
    }

    /// Las posiciones de una máscara, en orden.
    pub fn de_mascara(&self, mascara: u32) -> Vec<u8> {
        (0..self.familias.len() as u8).filter(|&i| mascara & (1 << i) != 0).collect()
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::Reglas;

    #[test]
    fn la_de_reposo_va_primero_y_las_cerradas_piden_disciplina_de_formacion() {
        let f = Reglas::cargar().formaciones;
        let ids: Vec<&str> = f.familias.iter().map(|x| x.id.as_str()).collect();
        assert_eq!(ids, ["reposo", "linea", "columna", "dispersa", "muro_escudos", "testudo", "cuadro", "cuna"]);
        assert_eq!(f.por_id("reposo"), Some(REPOSO));
        assert_eq!(f.por_id("falange"), None);
        assert_eq!(f.permitidas(|_| false), vec![0, 1, 2, 3], "sin tecnologías: reposo, línea, columna y dispersa");
        assert_eq!(f.permitidas(|t| t == "disciplina_formacion"), (0..8).collect::<Vec<u8>>());
        assert!(!f.permitida(f.por_id("testudo").unwrap(), |_| false));
        assert!(!f.permitida(99, |_| true), "una posición que no existe no se permite");
        assert_eq!(f.id(99), "reposo");
        assert_eq!(f.mascara(|_| false), 0b1111);
        assert_eq!(f.de_mascara(f.mascara(|t| t == "disciplina_formacion")), (0..8).collect::<Vec<u8>>());
    }
}
