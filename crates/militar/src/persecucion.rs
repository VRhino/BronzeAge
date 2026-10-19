//! Perseguir (Doc 5.12.3, Movimiento_Ejercitos §7): las condiciones para empezar y para cerrar una persecución. El plan de quien persigue (el
//! punto de intercepción contra el plan de la presa) es geometría del núcleo; aquí, las reglas.
//!
//! **Solo se persigue lo que se ve desde la propia columna** (decidido el 2026-10-19): la persecución se inicia con la presa dentro del
//! alcance de vista y se cancela si la presa sale de él, porque si no la trayectoria de quien persigue delataría dónde está bajo la niebla. No
//! hay umbral de recálculo: el plan se rehace cada vez que la presa cambia el suyo.

use crate::columna::TipoColumna;
use crate::datos::Reglas;
use base::Mil;

/// Quien persigue: una columna con la clase con la que salió y si lleva algún héroe sano.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Perseguidor {
    pub tipo: TipoColumna,
    pub hay_heroe_sano: bool,
}

/// Lo perseguible. Una caravana que no está en el mapa (`disponible`, `preparando` o `aparcada` en una plaza) no es presa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presa {
    Columna { tipo: TipoColumna, hay_heroe_sano: bool, propia_o_aliada: bool },
    Caravana { adjunta: bool, propia_o_aliada: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorPersecucion {
    /// Todos los héroes de la columna están heridos: un herido no persigue.
    SinHeroeSano,
    /// Todos los héroes de la presa están heridos: a un herido no se le persigue.
    PresaSinHeroeSano,
    /// Un ejército solo persigue a otro ejército, y una columna personal a otra o a una caravana.
    ClaseDistinta,
    /// No se persigue a los tuyos ni a un aliado.
    PropiaOAliada,
    /// Una caravana adjunta es parte de un ejército: se ataca al ejército.
    CaravanaAdjunta,
    /// Un ejército no persigue caravanas: las persigue una columna personal.
    UnEjercitoNoPersigueCaravanas,
    /// La presa no está a la vista de quien persigue.
    FueraDeVista,
}

/// ¿Está la presa a la vista? A ≤ `alcance` (el de [`crate::columna::alcance_de_vista`]).
pub fn a_la_vista(distancia: Mil, alcance: Mil) -> bool {
    distancia <= alcance
}

/// `perseguir`: las condiciones para empezar, en este orden. `distancia` es la de la presa a la columna en este instante y `alcance` el de
/// vista del perseguidor.
pub fn validar(perseguidor: &Perseguidor, presa: &Presa, distancia: Mil, alcance: Mil) -> Result<(), ErrorPersecucion> {
    if !perseguidor.hay_heroe_sano {
        return Err(ErrorPersecucion::SinHeroeSano);
    }
    match *presa {
        Presa::Columna { tipo, hay_heroe_sano, propia_o_aliada } => {
            if propia_o_aliada {
                return Err(ErrorPersecucion::PropiaOAliada);
            }
            if tipo != perseguidor.tipo {
                return Err(ErrorPersecucion::ClaseDistinta);
            }
            if !hay_heroe_sano {
                return Err(ErrorPersecucion::PresaSinHeroeSano);
            }
        }
        Presa::Caravana { adjunta, propia_o_aliada } => {
            if propia_o_aliada {
                return Err(ErrorPersecucion::PropiaOAliada);
            }
            if adjunta {
                return Err(ErrorPersecucion::CaravanaAdjunta);
            }
            if perseguidor.tipo == TipoColumna::Ejercito {
                return Err(ErrorPersecucion::UnEjercitoNoPersigueCaravanas);
            }
        }
    }
    if !a_la_vista(distancia, alcance) {
        return Err(ErrorPersecucion::FueraDeVista);
    }
    Ok(())
}

/// ¿Se alcanzó a la presa? A ≤ `radioEncuentro`, enemiga, con héroe sano (una columna) o no adjunta (una caravana). Se cierra la persecución
/// y se ofrece atacar; **no hay combate**: atacar es una orden aparte.
pub fn alcanzada(r: &Reglas, presa: &Presa, distancia: Mil) -> bool {
    let perseguible = match *presa {
        Presa::Columna { hay_heroe_sano, propia_o_aliada, .. } => hay_heroe_sano && !propia_o_aliada,
        Presa::Caravana { adjunta, propia_o_aliada } => !adjunta && !propia_o_aliada,
    };
    perseguible && distancia <= r.logistica.radio_encuentro
}

#[cfg(test)]
mod pruebas {
    use super::*;

    const YO: Perseguidor = Perseguidor { tipo: TipoColumna::Personal, hay_heroe_sano: true };
    const ELLA: Presa = Presa::Columna { tipo: TipoColumna::Personal, hay_heroe_sano: true, propia_o_aliada: false };
    const CARAVANA: Presa = Presa::Caravana { adjunta: false, propia_o_aliada: false };

    #[test]
    fn se_persigue_lo_que_se_ve_y_justo_en_el_limite_tambien() {
        assert_eq!(validar(&YO, &ELLA, 15_000, 15_000), Ok(()));
        assert_eq!(validar(&YO, &ELLA, 15_001, 15_000), Err(ErrorPersecucion::FueraDeVista));
        assert_eq!(validar(&YO, &CARAVANA, 29_999, 30_000), Ok(()));
        assert!(a_la_vista(30_000, 30_000) && !a_la_vista(30_001, 30_000));
    }

    #[test]
    fn un_ejercito_persigue_ejercitos_y_una_personal_personales_o_caravanas() {
        let ejercito = Perseguidor { tipo: TipoColumna::Ejercito, hay_heroe_sano: true };
        let otro = Presa::Columna { tipo: TipoColumna::Ejercito, hay_heroe_sano: true, propia_o_aliada: false };
        assert_eq!(validar(&ejercito, &otro, 0, 30_000), Ok(()));
        assert_eq!(validar(&ejercito, &ELLA, 0, 30_000), Err(ErrorPersecucion::ClaseDistinta));
        assert_eq!(validar(&YO, &otro, 0, 30_000), Err(ErrorPersecucion::ClaseDistinta));
        assert_eq!(validar(&ejercito, &CARAVANA, 0, 30_000), Err(ErrorPersecucion::UnEjercitoNoPersigueCaravanas));
        assert_eq!(validar(&YO, &CARAVANA, 0, 30_000), Ok(()));
    }

    #[test]
    fn no_se_persigue_a_los_tuyos_ni_a_un_aliado_ni_a_una_caravana_adjunta() {
        let propia = Presa::Columna { tipo: TipoColumna::Personal, hay_heroe_sano: true, propia_o_aliada: true };
        assert_eq!(validar(&YO, &propia, 0, 1), Err(ErrorPersecucion::PropiaOAliada));
        assert_eq!(validar(&YO, &Presa::Caravana { adjunta: false, propia_o_aliada: true }, 0, 1), Err(ErrorPersecucion::PropiaOAliada));
        assert_eq!(validar(&YO, &Presa::Caravana { adjunta: true, propia_o_aliada: false }, 0, 1), Err(ErrorPersecucion::CaravanaAdjunta));
    }

    #[test]
    fn un_herido_ni_persigue_ni_es_perseguido() {
        let herido = Perseguidor { hay_heroe_sano: false, ..YO };
        assert_eq!(validar(&herido, &ELLA, 0, 1), Err(ErrorPersecucion::SinHeroeSano));
        let presa_herida = Presa::Columna { tipo: TipoColumna::Personal, hay_heroe_sano: false, propia_o_aliada: false };
        assert_eq!(validar(&YO, &presa_herida, 0, 1), Err(ErrorPersecucion::PresaSinHeroeSano));
    }

    #[test]
    fn las_condiciones_se_comprueban_antes_de_la_vista() {
        // Una presa propia que además está lejos se rechaza por lo primero, no por la distancia.
        let propia = Presa::Caravana { adjunta: false, propia_o_aliada: true };
        assert_eq!(validar(&YO, &propia, 99_999, 15_000), Err(ErrorPersecucion::PropiaOAliada));
    }

    #[test]
    fn se_alcanza_a_tres_si_la_presa_es_enemiga_y_perseguible() {
        let r = Reglas::cargar();
        assert!(alcanzada(&r, &ELLA, 3_000));
        assert!(!alcanzada(&r, &ELLA, 3_001));
        assert!(alcanzada(&r, &CARAVANA, 0));
        assert!(!alcanzada(&r, &Presa::Caravana { adjunta: true, propia_o_aliada: false }, 0));
        assert!(!alcanzada(&r, &Presa::Columna { tipo: TipoColumna::Personal, hay_heroe_sano: false, propia_o_aliada: false }, 0), "una columna de heridos es intocable");
        assert!(!alcanzada(&r, &Presa::Columna { tipo: TipoColumna::Personal, hay_heroe_sano: true, propia_o_aliada: true }, 0));
    }
}
