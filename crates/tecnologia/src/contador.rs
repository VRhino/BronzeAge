//! Los 24 contadores de los logros del servidor (TA §2.1) y de qué vía sale cada uno. Decisión técnica 3 del plan: **se cuentan en origen**, tipados,
//! sin reparsear los hechos de la versión; el núcleo llama a estas funciones donde ocurre el hecho (6B) y suma el [`Delta`] con
//! [`crate::EstadoTecnologia::aplicar`]. Las vías de un solo contador (la caravana capturada o interceptada, el campamento de bandidos destruido, el
//! animal comprado, el encuentro en campo abierto) son `Delta::uno`; las de varios tienen su función.
//!
//! | Contador | Vía |
//! |---|---|
//! | `extraido.*` | [`de_extraccion`]: lo que sale del nodo, en plazas **y pueblos** (decisión 4, D17) |
//! | `fabricado.*` | [`de_fabricacion`]: las cuatro recetas de bronce |
//! | `batallas.libradas`, `.conHoplitas` | [`combate_resuelto`]; [`batalla_unity`] |
//! | `batallas.campoAbierto` | `Delta::uno` por encuentro; [`batalla_unity`] |
//! | `asedios.*`, `conquistas.conMurallaCompleta` | [`asedio`]; [`batalla_unity`] (nunca de `batalla.aplicada`, D18) |
//! | `caravanas.destruidasOCapturadas`, `bandidos.campamentosDestruidos` | `Delta::uno`; [`batalla_unity`] |
//! | `animales.comprados` | `Delta::uno` |
//! | `reclutados.*` | [`reclutamiento`]: en una plaza (reponer incluido) **y en un campamento de mercenarios** (Q4 del autor) |
//! | `plazasEnNivel.N` | récord, no suma: [`crate::EstadoTecnologia::subir_record`] |

use asentamiento::{Recurso, Tipo};

macro_rules! contadores {
    ($($v:ident = $n:literal),+ $(,)?) => {
        /// Un contador del servidor (el `ContadorLogro` del TS). El orden del enum es el índice de los arrays del estado.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Contador { $($v),+ }
        impl Contador {
            pub const TODOS: [Contador; 24] = [$(Contador::$v),+];
            /// Clave de los datos y del contrato (`"extraido.cobre"`).
            pub fn nombre(self) -> &'static str { match self { $(Contador::$v => $n),+ } }
            pub fn desde_nombre(s: &str) -> Option<Contador> { match s { $($n => Some(Contador::$v),)+ _ => None } }
        }
    };
}

contadores! {
    ExtraidoCobre = "extraido.cobre", ExtraidoHierro = "extraido.hierro", ExtraidoPiedra = "extraido.piedra", ExtraidoEstano = "extraido.estano",
    FabricadoEquipoBronce = "fabricado.equipoBronce", FabricadoArmaduraBronce = "fabricado.armaduraBronce",
    CaravanasDestruidasOCapturadas = "caravanas.destruidasOCapturadas", BandidosCampamentosDestruidos = "bandidos.campamentosDestruidos",
    AnimalesComprados = "animales.comprados", BatallasLibradas = "batallas.libradas", BatallasCampoAbierto = "batallas.campoAbierto",
    BatallasConHoplitas = "batallas.conHoplitas", AsediosResistidosEnCombate = "asedios.resistidosEnCombate",
    AsediosResistidosConResidentes = "asedios.resistidosConResidentes", AsediosContraMurallaCompleta = "asedios.contraMurallaCompleta",
    ConquistasConMurallaCompleta = "conquistas.conMurallaCompleta", ReclutadosEscuadrones = "reclutados.escuadrones",
    ReclutadosArqueros = "reclutados.arqueros", ReclutadosCaballeria = "reclutados.caballeria",
    ReclutadosEscaramuzadores = "reclutados.escaramuzadores_jabalina", ReclutadosArquerosCompuesto = "reclutados.arqueros_compuesto",
    ReclutadosJinetesAsirios = "reclutados.jinetes_asirios", PlazasEnNivel2 = "plazasEnNivel.2", PlazasEnNivel3 = "plazasEnNivel.3",
}

impl Contador {
    pub const fn indice(self) -> usize {
        self as usize
    }

    /// `plazasEnNivel.N` es un récord (el máximo de plazas a la vez con nivel actual ≥ N), no una suma (TA §2.1, §13).
    pub fn es_record(self) -> bool {
        matches!(self, Contador::PlazasEnNivel2 | Contador::PlazasEnNivel3)
    }

    /// El récord de `plazasEnNivel.N`, para el índice de plazas por nivel que lleva el núcleo (decisión 5).
    pub fn de_plazas_en_nivel(nivel: u8) -> Option<Contador> {
        match nivel {
            2 => Some(Contador::PlazasEnNivel2),
            3 => Some(Contador::PlazasEnNivel3),
            _ => None,
        }
    }
}

/// Lo que suma un hecho a cada contador (el `DeltaContadores` del TS).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Delta(pub [u64; 24]);

impl Delta {
    pub fn uno(c: Contador) -> Delta {
        Delta::default().con(c, 1)
    }

    pub fn con(mut self, c: Contador, n: u64) -> Delta {
        self.0[c.indice()] += n;
        self
    }

    pub fn sumar(&mut self, o: &Delta) {
        self.0.iter_mut().zip(o.0).for_each(|(a, b)| *a += b);
    }

    pub fn get(&self, c: Contador) -> u64 {
        self.0[c.indice()]
    }

    /// Los contadores que suben, en el orden del enum.
    pub fn iter(&self) -> impl Iterator<Item = (Contador, u64)> + '_ {
        Contador::TODOS.into_iter().map(|c| (c, self.get(c))).filter(|&(_, n)| n > 0)
    }

    pub fn es_vacio(&self) -> bool {
        self.0.iter().all(|&n| n == 0)
    }
}

/// `extraido.*`: el contador de lo que sale de un nodo de ese recurso (cobre, hierro, piedra y estaño; el resto no cuenta).
pub fn de_extraccion(r: Recurso) -> Option<Contador> {
    match r {
        Recurso::Cobre => Some(Contador::ExtraidoCobre),
        Recurso::Hierro => Some(Contador::ExtraidoHierro),
        Recurso::Piedra => Some(Contador::ExtraidoPiedra),
        Recurso::Estano => Some(Contador::ExtraidoEstano),
        _ => None,
    }
}

/// `fabricado.*`: los contadores de lo fabricado de una receta. `equipoBronce` cuenta las cuatro piezas de bronce; `armaduraBronce`, solo esa.
pub fn de_fabricacion(r: Recurso) -> &'static [Contador] {
    match r {
        Recurso::ArmaBronce | Recurso::ArmaBronceCalidad | Recurso::ArmaduraBronceCalidad => &[Contador::FabricadoEquipoBronce],
        Recurso::ArmaduraBronce => &[Contador::FabricadoEquipoBronce, Contador::FabricadoArmaduraBronce],
        _ => &[],
    }
}

/// `combate.resuelto` (una batalla con números): `batallas.libradas` y, si en ella había hoplitas ciudadanos, `batallas.conHoplitas`.
pub fn combate_resuelto(con_hoplitas: bool) -> Delta {
    let d = Delta::uno(Contador::BatallasLibradas);
    if con_hoplitas {
        d.con(Contador::BatallasConHoplitas, 1)
    } else {
        d
    }
}

/// Un asedio con números resuelto (`combate.asedio_resistido` o `combate.asedio_conquista`). `en_combate`: hubo combate (no un rebote por protección
/// ni una plaza sin defensores); `con_residentes`: lucharon residentes; `muralla_completa`: algún recinto con integridad ≥ 1.
pub fn asedio(conquista: bool, en_combate: bool, con_residentes: bool, muralla_completa: bool) -> Delta {
    let mut d = Delta::default();
    if en_combate && !conquista {
        d = d.con(Contador::AsediosResistidosEnCombate, 1);
        if con_residentes {
            d = d.con(Contador::AsediosResistidosConResidentes, 1);
        }
    }
    if en_combate && muralla_completa {
        d = d.con(Contador::AsediosContraMurallaCompleta, 1);
    }
    if conquista && muralla_completa {
        d = d.con(Contador::ConquistasConMurallaCompleta, 1);
    }
    d
}

/// El contexto de una batalla de Unity, para sus contadores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextoBatalla {
    CampoAbierto,
    /// `muralla_completa`: la plaza tenía algún recinto con integridad ≥ 1; `con_heroe_defensor`: algún héroe combatió en el bando defensor.
    Asedio { muralla_completa: bool, con_heroe_defensor: bool },
    Caravana,
    CampamentoBandidos,
}

/// Los contadores de una batalla de Unity aplicada (`contadoresDeBatalla` de `typescript-final:src/session/resultadoBatalla.ts`): se suman al aplicar
/// el resultado (D10), nunca desde `batalla.aplicada` (D18).
pub fn batalla_unity(contexto: ContextoBatalla, gana_el_atacante: bool, con_hoplitas: bool) -> Delta {
    let d = combate_resuelto(con_hoplitas);
    match contexto {
        ContextoBatalla::CampoAbierto => d.con(Contador::BatallasCampoAbierto, 1),
        ContextoBatalla::Caravana if gana_el_atacante => d.con(Contador::CaravanasDestruidasOCapturadas, 1),
        ContextoBatalla::CampamentoBandidos if gana_el_atacante => d.con(Contador::BandidosCampamentosDestruidos, 1),
        ContextoBatalla::Asedio { muralla_completa, con_heroe_defensor } => {
            let mut d = if muralla_completa { d.con(Contador::AsediosContraMurallaCompleta, 1) } else { d };
            if !gana_el_atacante {
                d = d.con(Contador::AsediosResistidosEnCombate, 1);
                if con_heroe_defensor {
                    d = d.con(Contador::AsediosResistidosConResidentes, 1);
                }
            } else if muralla_completa {
                d = d.con(Contador::ConquistasConMurallaCompleta, 1);
            }
            d
        }
        _ => d,
    }
}

/// Un soldado de carro de guerra vale por 5 en `reclutados.caballeria` (TA §2.1; decisión del autor del 2026-10-01, `PESO_EN_CABALLERIA` del TS).
pub const PESO_CARROS_GUERRA: u64 = 5;

/// Un reclutamiento (`contadoresDeReclutamiento`): la escuadra nueva (`antes == 0`), los soldados de una tropa de Caballerizas en
/// `reclutados.caballeria` (× 5 los carros de guerra) y los de las tropas con contador propio (`reclutados.<tropaId>`). `edificio` es el de la tropa
/// en el catálogo militar (este crate no depende de `militar`). Cuenta igual en una plaza y en un campamento de mercenarios (Q4).
pub fn reclutamiento(tropa: &str, edificio: Tipo, soldados: u64, escuadra_nueva: bool) -> Delta {
    let mut d = if escuadra_nueva { Delta::uno(Contador::ReclutadosEscuadrones) } else { Delta::default() };
    if soldados == 0 {
        return d;
    }
    if edificio == Tipo::Caballerizas {
        d = d.con(Contador::ReclutadosCaballeria, soldados * if tropa == "carros_guerra" { PESO_CARROS_GUERRA } else { 1 });
    }
    let propio = Contador::TODOS
        .into_iter()
        .filter(|c| !matches!(c, Contador::ReclutadosEscuadrones | Contador::ReclutadosCaballeria))
        .find(|c| c.nombre().strip_prefix("reclutados.") == Some(tropa));
    match propio {
        Some(c) => d.con(c, soldados),
        None => d,
    }
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/tecnologia.test.ts` («contadores por eventos») y del `contadoresDeBatalla` de
    //! `typescript-final:src/session/resultadoBatalla.ts`.
    use super::*;
    use Contador::*;

    fn delta(pares: &[(Contador, u64)]) -> Delta {
        pares.iter().fold(Delta::default(), |d, &(c, n)| d.con(c, n))
    }

    #[test]
    fn los_nombres_son_los_del_ts_y_hay_24() {
        assert_eq!(Contador::TODOS.len(), 24);
        for (i, c) in Contador::TODOS.into_iter().enumerate() {
            assert_eq!(c.indice(), i);
            assert_eq!(Contador::desde_nombre(c.nombre()), Some(c));
        }
        assert_eq!(Contador::desde_nombre("extraido.oro"), None);
        assert!(PlazasEnNivel2.es_record() && PlazasEnNivel3.es_record() && !ExtraidoCobre.es_record());
        assert_eq!((Contador::de_plazas_en_nivel(2), Contador::de_plazas_en_nivel(4)), (Some(PlazasEnNivel2), None));
    }

    #[test]
    fn un_rebote_por_proteccion_no_cuenta_como_asedio_resistido_uno_en_combate_si_con_muralla_y_residentes() {
        assert!(asedio(false, false, false, true).es_vacio());
        assert_eq!(asedio(false, true, true, true), delta(&[(AsediosResistidosEnCombate, 1), (AsediosResistidosConResidentes, 1), (AsediosContraMurallaCompleta, 1)]));
        assert_eq!(asedio(true, false, false, true), delta(&[(ConquistasConMurallaCompleta, 1)]));
    }

    #[test]
    fn batallas_hoplitas_y_animales() {
        let mut d = combate_resuelto(true);
        d.sumar(&Delta::uno(AnimalesComprados));
        assert_eq!(d, delta(&[(BatallasLibradas, 1), (BatallasConHoplitas, 1), (AnimalesComprados, 1)]));
    }

    #[test]
    fn reclutar_cuenta_soldados_de_las_tropas_con_logro_y_escuadrones_nuevos() {
        assert_eq!(reclutamiento("arqueros", Tipo::GaleriaDeTiro, 18, true), delta(&[(ReclutadosEscuadrones, 1), (ReclutadosArqueros, 18)]));
        assert!(reclutamiento("milicia_lanceros", Tipo::Barracon, 5, false).es_vacio());
        // Caballerizas: × 5 los carros, × 1 el resto; los jinetes asirios también tienen el suyo.
        assert_eq!(reclutamiento("carros_guerra", Tipo::Caballerizas, 4, false), delta(&[(ReclutadosCaballeria, 20)]));
        assert_eq!(reclutamiento("jinetes_asirios", Tipo::Caballerizas, 3, false), delta(&[(ReclutadosCaballeria, 3), (ReclutadosJinetesAsirios, 3)]));
        // Una escuadra nueva sin soldados (no debería pasar) solo cuenta la escuadra.
        assert_eq!(reclutamiento("arqueros", Tipo::GaleriaDeTiro, 0, true), delta(&[(ReclutadosEscuadrones, 1)]));
    }

    #[test]
    fn extraccion_y_fabricacion() {
        assert_eq!([Recurso::Cobre, Recurso::Hierro, Recurso::Piedra, Recurso::Estano, Recurso::Oro].map(de_extraccion), [Some(ExtraidoCobre), Some(ExtraidoHierro), Some(ExtraidoPiedra), Some(ExtraidoEstano), None]);
        assert_eq!(de_fabricacion(Recurso::ArmaduraBronce), &[FabricadoEquipoBronce, FabricadoArmaduraBronce]);
        assert_eq!(de_fabricacion(Recurso::ArmaBronceCalidad), &[FabricadoEquipoBronce]);
        assert!(de_fabricacion(Recurso::ArmaCobre).is_empty());
    }

    #[test]
    fn una_batalla_de_unity_suma_por_su_contexto_y_su_ganador() {
        assert_eq!(batalla_unity(ContextoBatalla::CampoAbierto, true, true), delta(&[(BatallasLibradas, 1), (BatallasConHoplitas, 1), (BatallasCampoAbierto, 1)]));
        assert_eq!(batalla_unity(ContextoBatalla::Caravana, true, false), delta(&[(BatallasLibradas, 1), (CaravanasDestruidasOCapturadas, 1)]));
        assert_eq!(batalla_unity(ContextoBatalla::Caravana, false, false), delta(&[(BatallasLibradas, 1)]));
        assert_eq!(batalla_unity(ContextoBatalla::CampamentoBandidos, true, false), delta(&[(BatallasLibradas, 1), (BandidosCampamentosDestruidos, 1)]));
        let resiste = batalla_unity(ContextoBatalla::Asedio { muralla_completa: true, con_heroe_defensor: true }, false, false);
        assert_eq!(resiste, delta(&[(BatallasLibradas, 1), (AsediosContraMurallaCompleta, 1), (AsediosResistidosEnCombate, 1), (AsediosResistidosConResidentes, 1)]));
        let cae = batalla_unity(ContextoBatalla::Asedio { muralla_completa: true, con_heroe_defensor: true }, true, false);
        assert_eq!(cae, delta(&[(BatallasLibradas, 1), (AsediosContraMurallaCompleta, 1), (ConquistasConMurallaCompleta, 1)]));
        assert_eq!(batalla_unity(ContextoBatalla::Asedio { muralla_completa: false, con_heroe_defensor: false }, true, false), delta(&[(BatallasLibradas, 1)]));
    }
}
