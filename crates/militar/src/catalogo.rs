//! El catálogo de tropas (`TROPAS_RECLUTABLES`, Doc 5.8 y Heroe_Y_Tropa §12): las 27 tropas con su edificio, su coste, su poder, su
//! velocidad, su escalón y el tamaño de su escuadra. Una **tropa** es el tipo; la escuadra (`escuadra`) es la instancia de un héroe.

use asentamiento::datos::mil;
use asentamiento::{Costo, Recurso, Tipo};
use base::Mil;
use serde_json::Value;

macro_rules! tropas {
    ($($v:ident = $n:literal),+ $(,)?) => {
        /// El tipo de una tropa. Los nombres son los ids de los datos y del contrato (`"lanceros_mimbre"`).
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Tropa { $($v),+ }
        /// Todas las tropas, en el orden del enum (el índice de `Tropa as usize`).
        pub const TROPAS: &[Tropa] = &[$(Tropa::$v),+];
        impl Tropa {
            pub fn nombre(self) -> &'static str { match self { $(Tropa::$v => $n),+ } }
            pub fn desde_nombre(s: &str) -> Option<Tropa> { match s { $($n => Some(Tropa::$v),)+ _ => None } }
        }
    };
}

tropas! {
    MiliciaLanceros = "milicia_lanceros", Lenadores = "lenadores", Granjeros = "granjeros", Mineros = "mineros",
    LancerosMimbre = "lanceros_mimbre", EspadachinesCobre = "espadachines_cobre", HacherosLigeros = "hacheros_ligeros",
    EspadachinesBronce = "espadachines_bronce", LancerosPesados = "lanceros_pesados", HacherosArmados = "hacheros_armados",
    Honderos = "honderos", EscaramuzadoresJabalina = "escaramuzadores_jabalina", Arqueros = "arqueros", ArquerosCompuesto = "arqueros_compuesto",
    ExploradoresCaballo = "exploradores_caballo", CarrosGuerra = "carros_guerra", JinetesAsirios = "jinetes_asirios",
    GuerrerosFilisteos = "guerreros_filisteos", Shardana = "shardana", Hequetai = "hequetai", HonderosRodios = "honderos_rodios",
    Peltastas = "peltastas", JinetesEscitas = "jinetes_escitas", HoplitasCiudadanos = "hoplitas_ciudadanos", CaballeriaAsiria = "caballeria_asiria",
    ArquerosEscitas = "arqueros_escitas", Espartiatas = "espartiatas",
}

/// De qué clase de población sale una tropa (Doc 5.8): los escalones 1 y 2, de pesants; el 3, de artesanos; el 4 y el 5, de nobleza.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Origen {
    Pesants,
    Artesanos,
    Nobleza,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefTropa {
    pub tropa: Tropa,
    /// La tecnología que la habilita (`leva_comunal`…): una puerta del sistema 6.
    pub tecnologia: String,
    pub nombre: String,
    /// Dónde se recluta: Centro Urbano, Barracón, Galería de tiro o Caballerizas.
    pub edificio: Tipo,
    /// Nivel interno mínimo del edificio.
    pub nivel_requerido: u8,
    /// Equipo por soldado, en milésimas de unidad (ordenado por recurso).
    pub costo_equipo: Costo,
    /// Caballos por soldado (se pagan en oro).
    pub caballos: i64,
    /// Poder de un soldado (placeholder del autor, Doc 5.8).
    pub poder_base: i64,
    /// Unidades de mapa por minuto, en milésimas.
    pub velocidad: Mil,
    /// 1 (leva) a 5 (élite): fija el coste de Liderazgo, el oro y la clase de población.
    pub escalon: u8,
    /// Tope de soldados de la escuadra: el jugador nunca elige cuántos.
    pub unidades: i64,
}

impl DefTropa {
    pub fn origen(&self) -> Origen {
        match self.escalon {
            0..=2 => Origen::Pesants,
            3 => Origen::Artesanos,
            _ => Origen::Nobleza,
        }
    }
}

/// Las 27 tropas, indexadas por [`Tropa`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalogo {
    tropas: Vec<DefTropa>,
}

impl Catalogo {
    /// Construye el catálogo del bloque `TROPAS_RECLUTABLES`. Un id desconocido, un dato que falta o una tropa sin definir es un error de
    /// programación (los datos están compilados dentro).
    pub fn de(bloque: &Value) -> Self {
        let mut tropas: Vec<Option<DefTropa>> = vec![None; TROPAS.len()];
        for x in bloque.as_array().expect("TROPAS_RECLUTABLES es una lista") {
            let texto = |k: &str| x[k].as_str().unwrap_or_else(|| panic!("falta «{k}»")).to_owned();
            let entero = |k: &str| x[k].as_i64().unwrap_or_else(|| panic!("falta el entero «{k}»"));
            let id = texto("id");
            let tropa = Tropa::desde_nombre(&id).unwrap_or_else(|| panic!("tropa «{id}» desconocida"));
            let mut costo_equipo: Costo = x["costoEquipo"]
                .as_object()
                .expect("costoEquipo")
                .iter()
                .map(|(k, n)| (Recurso::desde_nombre(k).unwrap_or_else(|| panic!("recurso «{k}»")), mil(n.as_f64().expect("cantidad"))))
                .collect();
            costo_equipo.sort();
            let edificio = texto("edificio");
            let def = DefTropa {
                tropa,
                tecnologia: texto("tecnologia"),
                nombre: texto("nombre"),
                edificio: Tipo::desde_nombre(&edificio).unwrap_or_else(|| panic!("edificio «{edificio}»")),
                nivel_requerido: entero("nivelRequerido") as u8,
                costo_equipo,
                caballos: x["caballos"].as_i64().unwrap_or(0),
                poder_base: entero("poderBase"),
                velocidad: mil(x["velocidad"].as_f64().expect("velocidad")),
                escalon: entero("escalon") as u8,
                unidades: entero("unidadesPorDefecto"),
            };
            assert!(tropas[tropa as usize].replace(def).is_none(), "tropa «{id}» repetida");
        }
        Self { tropas: tropas.into_iter().enumerate().map(|(i, t)| t.unwrap_or_else(|| panic!("falta la tropa «{}»", TROPAS[i].nombre()))).collect() }
    }

    pub fn tropa(&self, t: Tropa) -> &DefTropa {
        &self.tropas[t as usize]
    }

    pub fn iter(&self) -> impl Iterator<Item = &DefTropa> {
        self.tropas.iter()
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::{bloque, HEROE_Y_TROPA};

    fn catalogo() -> Catalogo {
        Catalogo::de(&bloque(HEROE_Y_TROPA, "TROPAS_RECLUTABLES"))
    }

    #[test]
    fn son_27_tropas_con_los_mineros_y_los_nombres_son_biyectivos() {
        let c = catalogo();
        assert_eq!(TROPAS.len(), 27);
        assert_eq!(c.iter().count(), 27);
        for (i, &t) in TROPAS.iter().enumerate() {
            assert_eq!(t as usize, i);
            assert_eq!(Tropa::desde_nombre(t.nombre()), Some(t));
            assert_eq!(c.tropa(t).tropa, t);
        }
        assert_eq!(Tropa::desde_nombre("ariete"), None);
    }

    #[test]
    fn los_mineros_son_leva_del_centro_urbano_sin_oro_y_con_piedra() {
        let m = catalogo().tropa(Tropa::Mineros).clone();
        assert_eq!((m.edificio, m.escalon, m.unidades, m.velocidad, m.poder_base), (Tipo::CentroUrbano, 1, 30, 30_000, 1));
        assert_eq!(m.costo_equipo, vec![(Recurso::Piedra, 1_000)]);
        assert_eq!((m.tecnologia.as_str(), m.nivel_requerido, m.caballos), ("leva_comunal", 1, 0));
    }

    #[test]
    fn las_velocidades_son_las_cuatro_clases_calibradas_del_canon() {
        let c = catalogo();
        let de = |t| c.tropa(t).velocidad;
        for t in [Tropa::ExploradoresCaballo, Tropa::JinetesAsirios, Tropa::JinetesEscitas, Tropa::CaballeriaAsiria] {
            assert_eq!(de(t), 42_000, "montada: {}", t.nombre());
        }
        for t in [
            Tropa::MiliciaLanceros,
            Tropa::Lenadores,
            Tropa::Granjeros,
            Tropa::Mineros,
            Tropa::Honderos,
            Tropa::LancerosMimbre,
            Tropa::EscaramuzadoresJabalina,
            Tropa::Peltastas,
            Tropa::HonderosRodios,
            Tropa::CarrosGuerra,
        ] {
            assert_eq!(de(t), 30_000, "ligera: {}", t.nombre());
        }
        for t in [
            Tropa::EspadachinesCobre,
            Tropa::EspadachinesBronce,
            Tropa::HacherosLigeros,
            Tropa::Arqueros,
            Tropa::GuerrerosFilisteos,
            Tropa::Shardana,
            Tropa::HoplitasCiudadanos,
            Tropa::ArquerosEscitas,
        ] {
            assert_eq!(de(t), 24_000, "media: {}", t.nombre());
        }
        for t in [Tropa::HacherosArmados, Tropa::LancerosPesados, Tropa::ArquerosCompuesto, Tropa::Hequetai, Tropa::Espartiatas] {
            assert_eq!(de(t), 18_000, "pesada: {}", t.nombre());
        }
    }

    #[test]
    fn el_escalon_fija_la_clase_de_poblacion_y_los_escalones_de_la_tabla_del_canon() {
        let c = catalogo();
        let por_escalon = |e: u8| -> Vec<Tropa> { c.iter().filter(|d| d.escalon == e).map(|d| d.tropa).collect() };
        assert_eq!(por_escalon(1), vec![Tropa::Lenadores, Tropa::Granjeros, Tropa::Mineros]);
        assert_eq!(por_escalon(5), vec![Tropa::ArquerosCompuesto, Tropa::Hequetai, Tropa::ArquerosEscitas]);
        assert_eq!(por_escalon(4), vec![Tropa::LancerosPesados, Tropa::CarrosGuerra, Tropa::Shardana, Tropa::Espartiatas]);
        assert_eq!(c.tropa(Tropa::MiliciaLanceros).origen(), Origen::Pesants);
        assert_eq!(c.tropa(Tropa::Mineros).origen(), Origen::Pesants);
        assert_eq!(c.tropa(Tropa::Arqueros).origen(), Origen::Artesanos);
        assert_eq!(c.tropa(Tropa::CarrosGuerra).origen(), Origen::Nobleza);
        assert_eq!(c.tropa(Tropa::ArquerosEscitas).origen(), Origen::Nobleza);
    }

    #[test]
    fn el_tamano_de_la_escuadra_baja_con_el_escalon() {
        let c = catalogo();
        // «Cuanto más de élite, menos cuerpos» (Doc 5.11.1): el tope del escalón es el de su tropa más pequeña.
        for e in 1..5u8 {
            let max_sup = c.iter().filter(|d| d.escalon == e + 1).map(|d| d.unidades).max().unwrap();
            let min_inf = c.iter().filter(|d| d.escalon == e).map(|d| d.unidades).min().unwrap();
            assert!(max_sup <= min_inf, "escalón {} vs {}", e, e + 1);
        }
    }

    #[test]
    fn caballos_y_equipo_de_ejemplos_del_canon() {
        let c = catalogo();
        let carros = c.tropa(Tropa::CarrosGuerra);
        assert_eq!(carros.caballos, 2);
        assert_eq!(carros.costo_equipo, vec![(Recurso::ArmaBronce, 1_000), (Recurso::CarroGuerra, 1_000)]);
        assert_eq!((carros.edificio, carros.nivel_requerido, carros.poder_base, carros.unidades), (Tipo::Caballerizas, 2, 13, 15));
        let compuesto = c.tropa(Tropa::ArquerosCompuesto);
        assert_eq!(compuesto.costo_equipo, vec![(Recurso::ArmaBronce, 3_000), (Recurso::ArmaduraIntermedia, 2_000)]);
        assert_eq!((compuesto.edificio, compuesto.nivel_requerido, compuesto.tecnologia.as_str()), (Tipo::GaleriaDeTiro, 3, "arco_compuesto"));
        assert_eq!(c.tropa(Tropa::Granjeros).costo_equipo, vec![]);
        assert_eq!(c.tropa(Tropa::Peltastas).costo_equipo, vec![(Recurso::ArmaHierro, 2_000)]);
    }
}
