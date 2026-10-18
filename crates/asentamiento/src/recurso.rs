//! Los 29 tipos de recurso de un almacén (`RECURSOS_TIPO`, Economia_Construccion §2).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Recurso {
    Madera,
    Piedra,
    Trigo,
    Cobre,
    Estano,
    Oro,
    Livestock,
    Hierro,
    LingoteCobre,
    LingoteEstano,
    LingoteBronce,
    LingoteHierro,
    Cuero,
    CueroCurtido,
    CueroCalidad,
    ArmaMadera,
    ArmaCobre,
    ArmaBronce,
    ArmaBronceCalidad,
    ArmaHierro,
    ArmaHierroCalidad,
    ArmaduraBasica,
    ArmaduraIntermedia,
    ArmaduraBronce,
    ArmaduraBronceCalidad,
    CarroGuerra,
    ChatarraCobre,
    ChatarraBronce,
    ChatarraHierro,
}

use Recurso::*;

/// Todos los recursos, en el orden del enum (el índice de `Recurso as usize`).
pub const TODOS: [Recurso; 29] = [
    Madera, Piedra, Trigo, Cobre, Estano, Oro, Livestock, Hierro, LingoteCobre, LingoteEstano, LingoteBronce, LingoteHierro, Cuero, CueroCurtido,
    CueroCalidad, ArmaMadera, ArmaCobre, ArmaBronce, ArmaBronceCalidad, ArmaHierro, ArmaHierroCalidad, ArmaduraBasica, ArmaduraIntermedia,
    ArmaduraBronce, ArmaduraBronceCalidad, CarroGuerra, ChatarraCobre, ChatarraBronce, ChatarraHierro,
];

const NOMBRES: [&str; 29] = [
    "madera", "piedra", "trigo", "cobre", "estano", "oro", "livestock", "hierro", "lingoteCobre", "lingoteEstano", "lingoteBronce", "lingoteHierro",
    "cuero", "cueroCurtido", "cueroCalidad", "armaMadera", "armaCobre", "armaBronce", "armaBronceCalidad", "armaHierro", "armaHierroCalidad",
    "armaduraBasica", "armaduraIntermedia", "armaduraBronce", "armaduraBronceCalidad", "carroGuerra", "chatarraCobre", "chatarraBronce", "chatarraHierro",
];

impl Recurso {
    pub const fn indice(self) -> usize {
        self as usize
    }

    /// Nombre de los datos de Mechanics (`"lingoteCobre"`).
    pub fn nombre(self) -> &'static str {
        NOMBRES[self.indice()]
    }

    pub fn desde_nombre(n: &str) -> Option<Recurso> {
        NOMBRES.iter().position(|x| *x == n).map(|i| TODOS[i])
    }
}

/// Lo que cuesta algo: recurso y cantidad en milésimas.
pub type Costo = Vec<(Recurso, base::Mil)>;

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn nombres_e_indices_son_coherentes() {
        for (i, r) in TODOS.iter().enumerate() {
            assert_eq!(r.indice(), i);
            assert_eq!(Recurso::desde_nombre(r.nombre()), Some(*r));
        }
        assert_eq!(Recurso::desde_nombre("oro"), Some(Recurso::Oro));
        assert_eq!(Recurso::desde_nombre("mithril"), None);
    }
}
