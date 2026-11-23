//! Dónde se puede fundar (Doc 1.2, 1.7, 1.8) y el aviso de viabilidad.

use crate::zona::{en_zona, Disco};
use base::{Mil, Punto};

/// Lo que el mundo dice del terreno en el punto (lo calcula quien tiene el artefacto).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terreno {
    FueraDelMapa,
    Agua,
    /// Tierra firme con más pendiente de la que deja fundar (`FUNDACION.pendienteMaximaParaFundar`): se pisa, no se funda.
    Empinado,
    /// Tierra firme que no se puede pisar (río sin vado, pendiente): la plaza quedaría incomunicada.
    NoTransitable,
    /// Tierra firme y pisable, pero en bosque cerrado (`FUNDACION.densidadBosqueMaximaParaFundar` o más): se cruza, no se funda.
    BosqueDenso,
    /// Tierra firme y pisable, pero tan cerca del trazo de un río que la ciudad acabaría dentro del cauce que se ve (`ORILLA_RIO`, decisión del
    /// autor del 2026-11-18).
    JuntoAlRio,
    Firme,
}

/// Por qué no se puede fundar. El código es estable (`RechazoOrden.codigo`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motivo {
    FueraDelMapa,
    SobreAgua,
    NoTransitable,
    Empinado,
    BosqueDenso,
    JuntoAlRio,
    DentroDeZona,
    CercaDePlaza,
    CercaDeCampamento,
    /// A menos de `FUNDACION.distanciaMinimaANodo` (12 u) de un nodo de recurso de cualquier tipo: al crecer la plaza lo taparía (decisión del autor del 2026-11-23).
    CercaDeNodo,
}

impl Motivo {
    pub fn codigo(self) -> &'static str {
        match self {
            Motivo::FueraDelMapa => "fundacion.fuera_del_mapa",
            Motivo::SobreAgua => "fundacion.sobre_agua",
            Motivo::Empinado => "fundacion.pendiente",
            Motivo::NoTransitable => "fundacion.no_transitable",
            Motivo::BosqueDenso => "fundacion.bosque_denso",
            Motivo::JuntoAlRio => "fundacion.junto_al_rio",
            Motivo::DentroDeZona => "fundacion.dentro_de_zona",
            Motivo::CercaDePlaza => "fundacion.cerca_de_plaza",
            Motivo::CercaDeCampamento => "fundacion.cerca_de_campamento",
            Motivo::CercaDeNodo => "fundacion.cerca_de_nodo",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DistanciasDeFundacion {
    /// Entre plazas, de centro a centro (`FUNDACION.separacionPlazas`: 60).
    pub separacion_plazas: Mil,
    /// A un campamento de mercenarios (`MERCENARIOS.radioExclusionFundar`: 60; a la distancia exacta sí se funda).
    pub exclusion_campamento: Mil,
    /// Del centro de la plaza a cualquier nodo de recurso (`FUNDACION.distanciaMinimaANodo`: 12; a la distancia exacta sí se funda).
    pub distancia_nodo: Mil,
}

/// Valida un punto de fundación, en este orden: mapa, agua, paso, pendiente, bosque cerrado, zonas, plazas, campamentos, nodos. `plazas` son los discos
/// vigentes de todas las plazas, de cualquier Facción (también la propia); `nodos`, las posiciones de todos los nodos de recurso del mundo.
pub fn validar(p: Punto, terreno: Terreno, plazas: &[Disco], campamentos: impl IntoIterator<Item = Punto>, nodos: impl IntoIterator<Item = Punto>, d: &DistanciasDeFundacion) -> Result<(), Motivo> {
    match terreno {
        Terreno::FueraDelMapa => return Err(Motivo::FueraDelMapa),
        Terreno::Agua => return Err(Motivo::SobreAgua),
        Terreno::NoTransitable => return Err(Motivo::NoTransitable),
        Terreno::Empinado => return Err(Motivo::Empinado),
        Terreno::BosqueDenso => return Err(Motivo::BosqueDenso),
        Terreno::JuntoAlRio => return Err(Motivo::JuntoAlRio),
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
    if nodos.into_iter().any(|n| p.dist2(n) < (d.distancia_nodo as i128).pow(2)) {
        return Err(Motivo::CercaDeNodo);
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

    const D: DistanciasDeFundacion = DistanciasDeFundacion { separacion_plazas: 30_000, exclusion_campamento: 20_000, distancia_nodo: 12_000 };

    fn plaza(x: Mil, y: Mil, r: Mil, f: u32) -> Disco {
        Disco { centro: Punto::new(x, y), radio: r, faccion: f }
    }

    #[test]
    fn terreno_y_mapa() {
        let p = Punto::new(500_000, 500_000);
        for (t, m) in [(Terreno::FueraDelMapa, Motivo::FueraDelMapa), (Terreno::Agua, Motivo::SobreAgua), (Terreno::Empinado, Motivo::Empinado), (Terreno::NoTransitable, Motivo::NoTransitable), (Terreno::BosqueDenso, Motivo::BosqueDenso), (Terreno::JuntoAlRio, Motivo::JuntoAlRio)] {
            assert_eq!(validar(p, t, &[], [], [], &D), Err(m));
        }
        assert_eq!(validar(p, Terreno::Firme, &[], [], [], &D), Ok(()));
    }

    #[test]
    fn zona_y_separacion() {
        let plazas = [plaza(0, 0, 10_000, 1)];
        assert_eq!(validar(Punto::new(10_000, 0), Terreno::Firme, &plazas, [], [], &D), Err(Motivo::DentroDeZona));
        // Fuera de la zona pero a menos de 30: la separación entre plazas manda.
        assert_eq!(validar(Punto::new(20_000, 0), Terreno::Firme, &plazas, [], [], &D), Err(Motivo::CercaDePlaza));
        // A 30 exactos se funda.
        assert_eq!(validar(Punto::new(30_000, 0), Terreno::Firme, &plazas, [], [], &D), Ok(()));
    }

    #[test]
    fn una_zona_recortada_deja_hueco() {
        // La plaza de nivel 5 (r 30) de la Facción 1 tiene encima otra de la Facción 2 (r 30) a 40: la frontera queda a 20,
        // así que (25, 0) ya no es de la Facción 1 pero sí de la 2 (está a 15 de ella).
        let plazas = [plaza(0, 0, 30_000, 1), plaza(40_000, 0, 30_000, 2)];
        assert_eq!(validar(Punto::new(25_000, 0), Terreno::Firme, &plazas, [], [], &D), Err(Motivo::DentroDeZona));
    }

    #[test]
    fn campamentos_a_veinte() {
        let c = [Punto::new(0, 0)];
        assert_eq!(validar(Punto::new(19_999, 0), Terreno::Firme, &[], c, [], &D), Err(Motivo::CercaDeCampamento));
        assert_eq!(validar(Punto::new(20_000, 0), Terreno::Firme, &[], c, [], &D), Ok(()));
    }

    #[test]
    fn ningun_nodo_a_menos_de_doce() {
        let n = [Punto::new(0, 0), Punto::new(500_000, 0)];
        assert_eq!(validar(Punto::new(0, 11_999), Terreno::Firme, &[], [], n, &D), Err(Motivo::CercaDeNodo));
        assert_eq!(validar(Punto::new(0, 12_000), Terreno::Firme, &[], [], n, &D), Ok(()));
        // El nodo se mira el último: la plaza y el campamento mandan antes.
        assert_eq!(validar(Punto::new(0, 0), Terreno::Firme, &[], [Punto::new(0, 0)], n, &D), Err(Motivo::CercaDeCampamento));
    }

    #[test]
    fn semaforo_de_viabilidad() {
        assert_eq!(viabilidad(false, 3, 3), Viabilidad::Rojo);
        assert_eq!(viabilidad(true, 3, 1), Viabilidad::Verde);
        assert_eq!(viabilidad(true, 0, 1), Viabilidad::Ambar);
        assert_eq!(viabilidad(true, 3, 0), Viabilidad::Ambar);
    }
}
