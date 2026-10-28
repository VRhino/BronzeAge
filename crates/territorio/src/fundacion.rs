//! Dónde se puede fundar (Doc 1.2, 1.7, 1.8) y el aviso de viabilidad.

use crate::zona::{en_zona, Disco};
use base::{Mil, Punto};

/// Lo que el mundo dice del terreno en el punto (lo calcula quien tiene el artefacto).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terreno {
    FueraDelMapa,
    Agua,
    Cima,
    /// Tierra firme que no se puede pisar (río sin vado, pendiente): la plaza quedaría incomunicada.
    NoTransitable,
    /// Tierra firme y pisable, pero en bosque cerrado (`FUNDACION.densidadBosqueMaximaParaFundar` o más): se cruza, no se funda.
    BosqueDenso,
    Firme,
}

/// Por qué no se puede fundar. El código es estable (`RechazoOrden.codigo`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motivo {
    FueraDelMapa,
    SobreAgua,
    SobreCima,
    NoTransitable,
    BosqueDenso,
    DentroDeZona,
    CercaDePlaza,
    CercaDeCampamento,
}

impl Motivo {
    pub fn codigo(self) -> &'static str {
        match self {
            Motivo::FueraDelMapa => "fundacion.fuera_del_mapa",
            Motivo::SobreAgua => "fundacion.sobre_agua",
            Motivo::SobreCima => "fundacion.sobre_cima",
            Motivo::NoTransitable => "fundacion.no_transitable",
            Motivo::BosqueDenso => "fundacion.bosque_denso",
            Motivo::DentroDeZona => "fundacion.dentro_de_zona",
            Motivo::CercaDePlaza => "fundacion.cerca_de_plaza",
            Motivo::CercaDeCampamento => "fundacion.cerca_de_campamento",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DistanciasDeFundacion {
    /// Entre plazas, de centro a centro (Doc 1.0a: 30).
    pub separacion_plazas: Mil,
    /// A un campamento de mercenarios (`MERCENARIOS.radioExclusionFundar`: 20; a la distancia exacta sí se funda).
    pub exclusion_campamento: Mil,
}

/// Valida un punto de fundación, en este orden: mapa, agua, cima, paso, bosque cerrado, zonas, plazas, campamentos. `plazas` son los discos
/// vigentes de todas las plazas, de cualquier Facción (también la propia).
pub fn validar(p: Punto, terreno: Terreno, plazas: &[Disco], campamentos: impl IntoIterator<Item = Punto>, d: &DistanciasDeFundacion) -> Result<(), Motivo> {
    match terreno {
        Terreno::FueraDelMapa => return Err(Motivo::FueraDelMapa),
        Terreno::Agua => return Err(Motivo::SobreAgua),
        Terreno::Cima => return Err(Motivo::SobreCima),
        Terreno::NoTransitable => return Err(Motivo::NoTransitable),
        Terreno::BosqueDenso => return Err(Motivo::BosqueDenso),
        Terreno::Firme => {}
    }
    // Solo hace falta mirar la zona de las plazas cuyo disco contiene el punto; su recorte sí usa a todas.
    if plazas.iter().any(|a| p.dist2(a.centro) <= (a.radio as i128).pow(2) && en_zona(p, a, plazas)) {
        return Err(Motivo::DentroDeZona);
    }
    if plazas.iter().any(|a| p.dist2(a.centro) < (d.separacion_plazas as i128).pow(2)) {
        return Err(Motivo::CercaDePlaza);
    }
    if campamentos.into_iter().any(|c| p.dist2(c) < (d.exclusion_campamento as i128).pow(2)) {
        return Err(Motivo::CercaDeCampamento);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Viabilidad {
    /// Fundable, con bosque libre y piedra en la zona con la que nacería.
    Verde,
    /// Fundable, pero le falta el bosque libre o la piedra.
    Ambar,
    /// No es fundable.
    Rojo,
}

pub fn viabilidad(valida: bool, parcelas_libres_en_zona: usize, piedra_en_zona: usize) -> Viabilidad {
    match (valida, parcelas_libres_en_zona > 0 && piedra_en_zona > 0) {
        (false, _) => Viabilidad::Rojo,
        (true, true) => Viabilidad::Verde,
        (true, false) => Viabilidad::Ambar,
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    const D: DistanciasDeFundacion = DistanciasDeFundacion { separacion_plazas: 30_000, exclusion_campamento: 20_000 };

    fn plaza(x: Mil, y: Mil, r: Mil, f: u32) -> Disco {
        Disco { centro: Punto::new(x, y), radio: r, faccion: f }
    }

    #[test]
    fn terreno_y_mapa() {
        let p = Punto::new(500_000, 500_000);
        for (t, m) in [(Terreno::FueraDelMapa, Motivo::FueraDelMapa), (Terreno::Agua, Motivo::SobreAgua), (Terreno::Cima, Motivo::SobreCima), (Terreno::NoTransitable, Motivo::NoTransitable), (Terreno::BosqueDenso, Motivo::BosqueDenso)] {
            assert_eq!(validar(p, t, &[], [], &D), Err(m));
        }
        assert_eq!(validar(p, Terreno::Firme, &[], [], &D), Ok(()));
    }

    #[test]
    fn zona_y_separacion() {
        let plazas = [plaza(0, 0, 10_000, 1)];
        assert_eq!(validar(Punto::new(10_000, 0), Terreno::Firme, &plazas, [], &D), Err(Motivo::DentroDeZona));
        // Fuera de la zona pero a menos de 30: la separación entre plazas manda.
        assert_eq!(validar(Punto::new(20_000, 0), Terreno::Firme, &plazas, [], &D), Err(Motivo::CercaDePlaza));
        // A 30 exactos se funda.
        assert_eq!(validar(Punto::new(30_000, 0), Terreno::Firme, &plazas, [], &D), Ok(()));
    }

    #[test]
    fn una_zona_recortada_deja_hueco() {
        // La plaza de nivel 5 (r 30) de la Facción 1 tiene encima otra de la Facción 2 (r 30) a 40: la frontera queda a 20,
        // así que (25, 0) ya no es de la Facción 1 pero sí de la 2 (está a 15 de ella).
        let plazas = [plaza(0, 0, 30_000, 1), plaza(40_000, 0, 30_000, 2)];
        assert_eq!(validar(Punto::new(25_000, 0), Terreno::Firme, &plazas, [], &D), Err(Motivo::DentroDeZona));
    }

    #[test]
    fn campamentos_a_veinte() {
        let c = [Punto::new(0, 0)];
        assert_eq!(validar(Punto::new(19_999, 0), Terreno::Firme, &[], c, &D), Err(Motivo::CercaDeCampamento));
        assert_eq!(validar(Punto::new(20_000, 0), Terreno::Firme, &[], c, &D), Ok(()));
    }

    #[test]
    fn semaforo_de_viabilidad() {
        assert_eq!(viabilidad(false, 3, 3), Viabilidad::Rojo);
        assert_eq!(viabilidad(true, 3, 1), Viabilidad::Verde);
        assert_eq!(viabilidad(true, 0, 1), Viabilidad::Ambar);
        assert_eq!(viabilidad(true, 3, 0), Viabilidad::Ambar);
    }
}
