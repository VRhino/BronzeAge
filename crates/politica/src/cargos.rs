//! Cargos locales de una plaza (Diplomacia_Facciones §3.4, Doc 2.2). El designado es ciudadano de la Facción dueña y, por la decisión **Q4** del autor
//! (2026-10-24), **reside en la plaza**; el cargo se vacía al dejar de residir (`Cargos::vaciar_de`). Al Gobernador lo designa el Rey; los otros cuatro, el
//! Gobernador vigente (lo exige la autorización), y solo si ya hay Gobernador.

use crate::faccion::Faccion;
use crate::tipos::{Cargo, Cargos, FaccionId, HeroeId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCargo {
    /// La plaza no es de esa Facción.
    PlazaAjena,
    /// El designado no es ciudadano de la Facción dueña.
    NoCiudadano,
    /// Q4: el designado no reside en la plaza.
    NoReside,
    /// Para designar Tesorero, General, Maestro de Obras o Sacerdote hace falta Gobernador.
    SinGobernador,
}

/// `asignarCargoLocal`. `dueña` es la Facción de la plaza (`faccion_de_la_plaza` la comprueba); `reside` dice si `h` reside en ella.
pub fn asignar_cargo_local(cargos: &mut Cargos, faccion_de_la_plaza: FaccionId, dueña: &Faccion, cargo: Cargo, h: HeroeId, reside: bool) -> Result<(), ErrorCargo> {
    if faccion_de_la_plaza != dueña.id {
        return Err(ErrorCargo::PlazaAjena);
    }
    if !dueña.es_ciudadano(h) {
        return Err(ErrorCargo::NoCiudadano);
    }
    if !reside {
        return Err(ErrorCargo::NoReside);
    }
    if cargo != Cargo::Gobernador && !cargos.ocupado(Cargo::Gobernador) {
        return Err(ErrorCargo::SinGobernador);
    }
    cargos.fijar(cargo, Some(h));
    Ok(())
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/cargos.test.ts` (`asignarCargoLocal`), más Q4.
    use super::*;
    use crate::faccion::pruebas::con_faccion;
    use crate::faccion::Facciones;

    fn micenas() -> Facciones {
        let mut fs = Facciones::default();
        con_faccion(&mut fs, 1, "Micenas", 10, &[11]);
        fs
    }

    #[test]
    fn el_designado_debe_ser_ciudadano_de_la_faccion() {
        let fs = micenas();
        let mut c = Cargos::default();
        assert_eq!(asignar_cargo_local(&mut c, 1, fs.get(1).unwrap(), Cargo::Gobernador, 99, true), Err(ErrorCargo::NoCiudadano));
        assert_eq!(asignar_cargo_local(&mut c, 2, fs.get(1).unwrap(), Cargo::Gobernador, 10, true), Err(ErrorCargo::PlazaAjena));
    }

    #[test]
    fn el_resto_de_cargos_exige_que_ya_haya_gobernador() {
        let fs = micenas();
        let mut c = Cargos::default();
        assert_eq!(asignar_cargo_local(&mut c, 1, fs.get(1).unwrap(), Cargo::Tesorero, 11, true), Err(ErrorCargo::SinGobernador));
        asignar_cargo_local(&mut c, 1, fs.get(1).unwrap(), Cargo::Gobernador, 10, true).unwrap();
        asignar_cargo_local(&mut c, 1, fs.get(1).unwrap(), Cargo::Tesorero, 11, true).unwrap();
        assert_eq!(c.titular(Cargo::Tesorero), Some(11));
        asignar_cargo_local(&mut c, 1, fs.get(1).unwrap(), Cargo::General, 10, true).unwrap();
        assert_eq!(c.vaciar_de(10), vec![Cargo::Gobernador, Cargo::General], "un héroe puede ocupar varios");
    }

    #[test]
    fn q4_hay_que_residir_para_ser_designado() {
        let fs = micenas();
        let mut c = Cargos::default();
        assert_eq!(asignar_cargo_local(&mut c, 1, fs.get(1).unwrap(), Cargo::Gobernador, 11, false), Err(ErrorCargo::NoReside));
        assert!(!c.ocupado(Cargo::Gobernador));
    }
}
