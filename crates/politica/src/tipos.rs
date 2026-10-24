//! Tipos comunes de la política: identificadores, los cinco cargos locales y los grupos de la puerta.

pub type FaccionId = u32;
pub type HeroeId = u64;
pub type AsentamientoId = u32;
pub type CampamentoId = u32;
/// Las relaciones y las propuestas toman ids del contador común de la partida: un mismo par puede tener varias relaciones a lo largo del tiempo.
pub type RelacionId = u64;
pub type PropuestaId = u64;

/// Los cargos locales (Doc 2.2) son los de la economía (`asentamiento::Cargo`): el Gobernador designa a los otros cuatro; al Gobernador lo designa el Rey.
pub use asentamiento::Cargo;

/// Los cinco cargos, en el orden del enum.
pub const CARGOS: [Cargo; 5] = [Cargo::Gobernador, Cargo::Tesorero, Cargo::General, Cargo::MaestroObras, Cargo::Sacerdote];

/// Los titulares de los cargos locales de una plaza (`Asentamiento.cargos`). Un héroe puede ocupar varios.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cargos([Option<HeroeId>; 5]);

impl Cargos {
    pub fn titular(&self, c: Cargo) -> Option<HeroeId> {
        self.0[c as usize]
    }

    /// ¿Está cubierto el puesto? Pregunta de regla de juego («¿hay Gobernador?»), distinta de [`Cargos::es_titular`].
    pub fn ocupado(&self, c: Cargo) -> bool {
        self.titular(c).is_some()
    }

    /// ¿Es `h` quien lo ocupa? Pregunta de autorización («¿puedes tú?»).
    pub fn es_titular(&self, c: Cargo, h: HeroeId) -> bool {
        self.titular(c) == Some(h)
    }

    /// Escribe el titular (o lo libera con `None`) sin validar: las reglas de quién puede están en `cargos`.
    pub fn fijar(&mut self, c: Cargo, h: Option<HeroeId>) {
        self.0[c as usize] = h;
    }

    /// Vacía los cargos que ocupa `h` (al dejar de residir, al dejar la Facción, en la conquista, la ruina y el traslado) y dice cuáles eran.
    pub fn vaciar_de(&mut self, h: HeroeId) -> Vec<Cargo> {
        let vaciados: Vec<Cargo> = CARGOS.into_iter().filter(|&c| self.es_titular(c, h)).collect();
        vaciados.iter().for_each(|&c| self.fijar(c, None));
        vaciados
    }
}

/// Los grupos a los que se puede cerrar la puerta de una plaza (Doc 1.10.5; contrato `GrupoPuerta`). La propia Facción no es un grupo: nunca se cierra.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Grupo {
    Neutrales,
    Aliados,
    Enemigos,
    Aedas,
}

impl Grupo {
    pub const TODOS: [Grupo; 4] = [Grupo::Neutrales, Grupo::Aliados, Grupo::Enemigos, Grupo::Aedas];

    pub fn nombre(self) -> &'static str {
        match self {
            Grupo::Neutrales => "neutrales",
            Grupo::Aliados => "aliados",
            Grupo::Enemigos => "enemigos",
            Grupo::Aedas => "aedas",
        }
    }

    pub fn desde_nombre(n: &str) -> Option<Grupo> {
        Grupo::TODOS.into_iter().find(|g| g.nombre() == n)
    }
}

/// Un conjunto de grupos, sin repetidos y en orden fijo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Grupos(u8);

impl Grupos {
    pub fn de(grupos: impl IntoIterator<Item = Grupo>) -> Self {
        Self(grupos.into_iter().fold(0, |b, g| b | 1 << g as u8))
    }

    pub fn contiene(self, g: Grupo) -> bool {
        self.0 & 1 << g as u8 != 0
    }

    pub fn iter(self) -> impl Iterator<Item = Grupo> {
        Grupo::TODOS.into_iter().filter(move |&g| self.contiene(g))
    }

    /// Para la foto: un byte.
    pub fn bits(self) -> u8 {
        self.0
    }

    pub fn desde_bits(b: u8) -> Self {
        Self(b & 0b1111)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn los_cargos_se_vacian_por_titular() {
        let mut c = Cargos::default();
        c.fijar(Cargo::Gobernador, Some(7));
        c.fijar(Cargo::Tesorero, Some(7));
        c.fijar(Cargo::General, Some(8));
        assert!(c.ocupado(Cargo::Gobernador) && c.es_titular(Cargo::Gobernador, 7) && !c.es_titular(Cargo::Gobernador, 8));
        assert_eq!(c.vaciar_de(7), vec![Cargo::Gobernador, Cargo::Tesorero]);
        assert!(!c.ocupado(Cargo::Gobernador));
        assert_eq!(c.titular(Cargo::General), Some(8));
            }

    #[test]
    fn los_grupos_son_un_conjunto() {
        let g = Grupos::de([Grupo::Enemigos, Grupo::Neutrales, Grupo::Enemigos]);
        assert_eq!(g.iter().collect::<Vec<_>>(), vec![Grupo::Neutrales, Grupo::Enemigos]);
        assert_eq!(Grupos::desde_bits(g.bits()), g);
        assert_eq!(Grupo::desde_nombre("aedas"), Some(Grupo::Aedas));
    }
}
