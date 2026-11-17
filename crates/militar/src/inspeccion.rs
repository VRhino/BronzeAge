//! Inspeccionar (Doc 5.12.3, Movimiento_Ejercitos §12): a ≤ `radioInspeccion` se ve de cerca la composición de una columna ajena, la defensa de una
//! plaza de otra Facción o lo que lleva una caravana. No es gratis: el observado recibe aviso. Lo visto queda como **informe** del héroe, con su fecha y
//! el lugar donde se vio, `INSPECCION.informeMinutos` (10 minutos); un informe por objetivo, y mirar de nuevo lo renueva.
//!
//! Aquí están las condiciones y la forma del informe; qué se ve de cada cosa y dónde se guarda es del núcleo.

use crate::catalogo::Tropa;
use crate::datos::Reglas;
use crate::tipos::{FaccionId, HeroeId};
use asentamiento::Recurso;
use base::{Instante, Mil, Punto};
use trazado::CeldaMuro;

/// A qué se apunta con `inspeccionar` o `perseguir`: una columna (`ejercito` en el contrato, sea personal o ejército), una caravana o una plaza.
/// Las ids son las del mundo (`u64`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Objetivo {
    Ejercito(u64),
    Caravana(u64),
    Asentamiento(u64),
}

/// Una escuadra vista de cerca: qué tropa, cuántos soldados en pie y de qué héroe. Nada de nivel, moral ni experiencia.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Avistada {
    pub tropa: Tropa,
    pub cantidad: i64,
    pub heroe: HeroeId,
}

/// Un recinto de muralla visto de cerca (Murallas §9.1): su anillo entero y cuántas celdas hay levantadas. Solo se ven los que ya tienen alguna.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecintoAvistado {
    /// El nivel del anillo en pie.
    pub nivel: u8,
    /// Todas las celdas del anillo, en su orden de recorrido, con su clase.
    pub celdas: Vec<CeldaMuro>,
    /// Las celdas levantadas (`celdas[..avance]`).
    pub avance: u32,
}

/// Lo que se saca de mirar de cerca.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Contenido {
    /// Una columna ajena: su Facción, los héroes que lleva y cada escuadra con soldados.
    Columna { faccion: Option<FaccionId>, heroes: Vec<HeroeId>, escuadras: Vec<Avistada> },
    /// Una caravana: si va escoltada y **qué** recursos lleva (nombres, ordenados), nunca cuánto.
    Caravana { escoltada: bool, recursos: Vec<Recurso> },
    /// La defensa de una plaza ajena: la guarnición de sus residentes, los héroes que hay dentro y sus recintos de muralla. El almacén, no.
    Plaza { faccion: FaccionId, heroes: Vec<HeroeId>, guarnicion: Vec<Avistada>, recintos: Vec<RecintoAvistado> },
}

/// Lo último que inspeccionó un héroe de un objetivo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Informe {
    pub objetivo: Objetivo,
    pub visto_en: Instante,
    /// `visto_en + INSPECCION.informeMinutos`: vale mientras `ahora < expira_en`.
    pub expira_en: Instante,
    /// Dónde estaba el objetivo cuando se miró: lo que se ve envejece y el cliente lo enseña con su fecha.
    pub donde: Punto,
    pub contenido: Contenido,
}

impl Informe {
    pub fn vigente(&self, ahora: Instante) -> bool {
        ahora < self.expira_en
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorInspeccion {
    /// Esa columna es la del propio observador.
    LaTuya,
    /// Esa plaza es de la Facción del observador: se inspecciona la ajena.
    PlazaPropia,
    /// Hay que acercarse a `radioInspeccion`.
    Lejos,
}

/// Las condiciones de `inspeccionar`, en este orden. `propio`: la columna es la del observador, o la plaza es de su Facción (una caravana no tiene
/// condición de propiedad: inspeccionar una propia no añade nada pero no se prohíbe).
pub fn validar(r: &Reglas, objetivo: Objetivo, propio: bool, distancia: Mil) -> Result<(), ErrorInspeccion> {
    match objetivo {
        Objetivo::Ejercito(_) if propio => return Err(ErrorInspeccion::LaTuya),
        Objetivo::Asentamiento(_) if propio => return Err(ErrorInspeccion::PlazaPropia),
        _ => {}
    }
    if distancia > r.movimiento.radio_inspeccion {
        return Err(ErrorInspeccion::Lejos);
    }
    Ok(())
}

/// Anota lo visto: sustituye el informe anterior del mismo objetivo y barre los que ya vencieron.
pub fn anotar(r: &Reglas, informes: &mut Vec<Informe>, objetivo: Objetivo, ahora: Instante, donde: Punto, contenido: Contenido) {
    informes.retain(|i| i.vigente(ahora) && i.objetivo != objetivo);
    informes.push(Informe { objetivo, visto_en: ahora, expira_en: ahora + r.inspeccion.informe, donde, contenido });
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use base::MS_POR_MINUTO;

    fn caravana(escoltada: bool) -> Contenido {
        Contenido::Caravana { escoltada, recursos: vec![Recurso::Trigo] }
    }

    #[test]
    fn se_inspecciona_hasta_ocho_y_no_lo_propio() {
        let r = Reglas::cargar();
        assert_eq!(validar(&r, Objetivo::Ejercito(3), false, 8_000), Ok(()), "justo en el límite");
        assert_eq!(validar(&r, Objetivo::Ejercito(3), false, 8_001), Err(ErrorInspeccion::Lejos));
        assert_eq!(validar(&r, Objetivo::Caravana(3), false, 0), Ok(()));
        assert_eq!(validar(&r, Objetivo::Asentamiento(3), false, 5_000), Ok(()));
        assert_eq!(validar(&r, Objetivo::Ejercito(3), true, 99_000), Err(ErrorInspeccion::LaTuya), "lo propio se comprueba antes de la distancia");
        assert_eq!(validar(&r, Objetivo::Asentamiento(3), true, 0), Err(ErrorInspeccion::PlazaPropia));
        assert_eq!(validar(&r, Objetivo::Caravana(3), true, 0), Ok(()));
    }

    #[test]
    fn el_informe_dura_diez_minutos_y_mirar_de_nuevo_lo_renueva() {
        let r = Reglas::cargar();
        let mut informes = Vec::new();
        anotar(&r, &mut informes, Objetivo::Caravana(7), 0, Punto::new(1, 2), caravana(false));
        anotar(&r, &mut informes, Objetivo::Ejercito(9), 4 * MS_POR_MINUTO, Punto::new(3, 4), Contenido::Columna { faccion: None, heroes: vec![1], escuadras: vec![] });
        assert_eq!(informes.len(), 2);
        assert!(informes[0].vigente(10 * MS_POR_MINUTO - 1) && !informes[0].vigente(10 * MS_POR_MINUTO));
        // Mirar otra vez la caravana, ya escoltada, sustituye al viejo informe; el de la columna sigue.
        anotar(&r, &mut informes, Objetivo::Caravana(7), 6 * MS_POR_MINUTO, Punto::new(5, 6), caravana(true));
        assert_eq!(informes.len(), 2);
        let c = informes.iter().find(|i| i.objetivo == Objetivo::Caravana(7)).unwrap();
        assert_eq!((c.visto_en, c.expira_en, c.donde, &c.contenido), (6 * MS_POR_MINUTO, 16 * MS_POR_MINUTO, Punto::new(5, 6), &caravana(true)));
        // Al anotar a los 15 minutos se barre el que venció (la columna, a los 14).
        anotar(&r, &mut informes, Objetivo::Asentamiento(1), 15 * MS_POR_MINUTO, Punto::new(0, 0), Contenido::Plaza { faccion: 2, heroes: vec![], guarnicion: vec![], recintos: vec![] });
        assert_eq!(informes.iter().map(|i| i.objetivo).collect::<Vec<_>>(), [Objetivo::Caravana(7), Objetivo::Asentamiento(1)]);
    }
}
