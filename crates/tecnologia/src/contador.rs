//! Los 36 contadores de los logros del servidor (TA §2.1) y de qué vía sale cada uno. Decisión técnica 3 del plan: **se cuentan en origen**, tipados,
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
//! | `plazasEnNivel.N`, `caminos.compartidos` | récord, no suma: [`crate::EstadoTecnologia::subir_record`] |
//! | `batallas.conTresHeroes`, `.ganadasEnInferioridad`, `.campoAbiertoConCaballeria`, `.armasCombinadas`, `.contraCoalicion`, `.conRey`, `.campoAbiertoConCaballeriaElite`, `escuadras.eliteDestruidas` | [`de_batalla`], con un [`ResumenBatalla`] |
//! | `persecuciones.deCaballeria` | [`persecucion`], con el bando atacante |
//! | `escuadras.veteranas` | la experiencia de la escuadra (lo cuenta el núcleo) |

use crate::FaccionId;
use asentamiento::{Recurso, Tipo};
use std::collections::BTreeSet;

macro_rules! contadores {
    ($($v:ident = $n:literal),+ $(,)?) => {
        /// Un contador del servidor (el `ContadorLogro` del TS). El orden del enum es el índice de los arrays del estado.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Contador { $($v),+ }
        impl Contador {
            /// Cuántos hay: el tamaño de los arrays del estado y de la foto.
            pub const N: usize = [$(Contador::$v),+].len();
            pub const TODOS: [Contador; Contador::N] = [$(Contador::$v),+];
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
    // Eras IV y V (Eras_IV_Y_V §3, 2026-11-11).
    EscuadrasVeteranas = "escuadras.veteranas", BatallasConTresHeroes = "batallas.conTresHeroes", CaminosCompartidos = "caminos.compartidos",
    BatallasGanadasEnInferioridad = "batallas.ganadasEnInferioridad", EscuadrasEliteDestruidas = "escuadras.eliteDestruidas",
    BatallasCampoAbiertoConCaballeria = "batallas.campoAbiertoConCaballeria", BatallasArmasCombinadas = "batallas.armasCombinadas",
    BatallasContraCoalicion = "batallas.contraCoalicion", BatallasConRey = "batallas.conRey", PersecucionesDeCaballeria = "persecuciones.deCaballeria",
    BatallasCampoAbiertoConCaballeriaElite = "batallas.campoAbiertoConCaballeriaElite", FabricadoArmaHierroCalidad = "fabricado.armaHierroCalidad",
}

impl Contador {
    pub const fn indice(self) -> usize {
        self as usize
    }

    /// `plazasEnNivel.N` es un récord (el máximo de plazas a la vez con nivel actual ≥ N), no una suma (TA §2.1, §13); `caminos.compartidos` también: 0 o 1, a 1 cuando se
    /// abre el primer camino compartido (Eras_IV_Y_V §3).
    pub fn es_record(self) -> bool {
        matches!(self, Contador::PlazasEnNivel2 | Contador::PlazasEnNivel3 | Contador::CaminosCompartidos)
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delta(pub [u64; Contador::N]);

/// A mano: un array de más de 32 no tiene `Default`.
impl Default for Delta {
    fn default() -> Self {
        Delta([0; Contador::N])
    }
}

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

/// `fabricado.*`: los contadores de lo fabricado de una receta. `equipoBronce` cuenta las cuatro piezas de bronce; `armaduraBronce`, solo esa; `armaHierroCalidad`, el arma de
/// hierro de calidad (Eras_IV_Y_V §3).
pub fn de_fabricacion(r: Recurso) -> &'static [Contador] {
    match r {
        Recurso::ArmaBronce | Recurso::ArmaBronceCalidad | Recurso::ArmaduraBronceCalidad => &[Contador::FabricadoEquipoBronce],
        Recurso::ArmaduraBronce => &[Contador::FabricadoEquipoBronce, Contador::FabricadoArmaduraBronce],
        Recurso::ArmaHierroCalidad => &[Contador::FabricadoArmaHierroCalidad],
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

/// `batallas.conTresHeroes`: algún bando lleva al menos estos héroes (Eras_IV_Y_V §3; el número es el nombre del contador).
pub const HEROES_DE_UNA_BATALLA_GRANDE: u32 = 3;
/// El escalón desde el que una escuadra es de élite: `escuadras.eliteDestruidas` y `batallas.campoAbiertoConCaballeriaElite` (Eras_IV_Y_V §3: «escalón 4 o 5»).
pub const ESCALON_ELITE: u8 = 4;

/// Una escuadra de la batalla, con lo que miran los contadores de las Eras IV y V: su edificio y su escalón (los de su tropa) y si acabó sin soldados. Solo las que empezaron con alguno.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscuadraDeBatalla {
    pub edificio: Tipo,
    pub escalon: u8,
    pub sin_soldados: bool,
}

/// Un bando de la batalla: las Facciones de sus héroes, cuántos héroes combaten, sus soldados al empezar y sus escuadras.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BandoDeBatalla {
    pub facciones: BTreeSet<FaccionId>,
    pub heroes: u32,
    pub soldados: u64,
    pub escuadras: Vec<EscuadraDeBatalla>,
}

impl BandoDeBatalla {
    fn con(&self, edificio: Tipo, escalon: u8) -> bool {
        self.escuadras.iter().any(|e| e.edificio == edificio && e.escalon >= escalon)
    }
}

/// El resumen puro de una batalla para los contadores de las Eras IV y V (Eras_IV_Y_V §3): lo arma el núcleo donde cuenta `batallas.libradas` (el encuentro con números y toda
/// batalla de Unity). `con_rey`: combate el Rey de alguna Facción; `campo_abierto`: el encuentro con números o la batalla de Unity de campo abierto.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResumenBatalla {
    pub atacante: BandoDeBatalla,
    pub defensor: BandoDeBatalla,
    pub con_rey: bool,
    pub gana_el_atacante: bool,
    pub campo_abierto: bool,
}

/// Los contadores de batalla de las Eras IV y V que sube una batalla (Eras_IV_Y_V §3), salvo `persecuciones.deCaballeria` ([`persecucion`]) y `escuadras.veteranas` (la
/// experiencia de la escuadra).
pub fn de_batalla(b: &ResumenBatalla) -> Delta {
    let (ganador, perdedor) = if b.gana_el_atacante { (&b.atacante, &b.defensor) } else { (&b.defensor, &b.atacante) };
    let bandos = [&b.atacante, &b.defensor];
    let elite_destruidas = bandos.iter().flat_map(|x| &x.escuadras).filter(|e| e.escalon >= ESCALON_ELITE && e.sin_soldados).count() as u64;
    let armas_combinadas = bandos.iter().any(|x| [Tipo::Barracon, Tipo::GaleriaDeTiro, Tipo::Caballerizas].iter().all(|&t| x.con(t, 0)));
    let reglas = [
        (Contador::BatallasConTresHeroes, bandos.iter().any(|x| x.heroes >= HEROES_DE_UNA_BATALLA_GRANDE)),
        (Contador::BatallasGanadasEnInferioridad, ganador.soldados < perdedor.soldados),
        (Contador::BatallasCampoAbiertoConCaballeria, b.campo_abierto && ganador.con(Tipo::Caballerizas, 0)),
        (Contador::BatallasArmasCombinadas, armas_combinadas),
        (Contador::BatallasContraCoalicion, ganador.facciones.len() == 1 && perdedor.facciones.len() >= 2),
        (Contador::BatallasConRey, b.con_rey),
        (Contador::BatallasCampoAbiertoConCaballeriaElite, b.campo_abierto && ganador.con(Tipo::Caballerizas, ESCALON_ELITE)),
    ];
    reglas.into_iter().filter(|&(_, si)| si).fold(Delta::default().con(Contador::EscuadrasEliteDestruidas, elite_destruidas), |d, (c, _)| d.con(c, 1))
}

/// `persecuciones.deCaballeria`: una columna que lleva alguna escuadra de Caballerizas ataca a otra columna o a una caravana, con números o en Unity, gane o pierda (Eras_IV_Y_V §3).
pub fn persecucion(atacante: &BandoDeBatalla) -> Delta {
    if atacante.con(Tipo::Caballerizas, 0) {
        Delta::uno(Contador::PersecucionesDeCaballeria)
    } else {
        Delta::default()
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
    fn los_nombres_son_los_del_ts_y_hay_36() {
        assert_eq!(Contador::TODOS.len(), 36);
        for (i, c) in Contador::TODOS.into_iter().enumerate() {
            assert_eq!(c.indice(), i);
            assert_eq!(Contador::desde_nombre(c.nombre()), Some(c));
        }
        assert_eq!(Contador::desde_nombre("extraido.oro"), None);
        assert!(PlazasEnNivel2.es_record() && PlazasEnNivel3.es_record() && CaminosCompartidos.es_record() && !ExtraidoCobre.es_record());
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

    fn esc(edificio: Tipo, escalon: u8, sin_soldados: bool) -> EscuadraDeBatalla {
        EscuadraDeBatalla { edificio, escalon, sin_soldados }
    }

    fn bando(facciones: &[FaccionId], heroes: u32, soldados: u64, escuadras: &[EscuadraDeBatalla]) -> BandoDeBatalla {
        BandoDeBatalla { facciones: facciones.iter().copied().collect(), heroes, soldados, escuadras: escuadras.to_vec() }
    }

    /// Una batalla sin nada de las Eras IV y V: un héroe por bando, de dos Facciones, infantería de leva contra infantería de leva; gana el atacante, que es el más numeroso.
    fn sencilla() -> ResumenBatalla {
        ResumenBatalla {
            atacante: bando(&[1], 1, 30, &[esc(Tipo::Barracon, 1, false)]),
            defensor: bando(&[2], 1, 20, &[esc(Tipo::Barracon, 1, true)]),
            con_rey: false,
            gana_el_atacante: true,
            campo_abierto: true,
        }
    }

    #[test]
    fn una_batalla_sin_nada_de_las_eras_iv_y_v_no_sube_ninguno() {
        assert!(de_batalla(&sencilla()).es_vacio());
        assert!(persecucion(&sencilla().atacante).es_vacio());
    }

    #[test]
    fn tres_heroes_en_un_bando_cuentan_dos_no() {
        let mut b = sencilla();
        b.defensor.heroes = 2;
        assert!(de_batalla(&b).es_vacio());
        b.defensor.heroes = 3;
        assert_eq!(de_batalla(&b), delta(&[(BatallasConTresHeroes, 1)]), "gane o pierda");
    }

    #[test]
    fn gana_en_inferioridad_quien_empezo_con_menos_soldados() {
        let mut b = sencilla();
        b.gana_el_atacante = false;
        assert_eq!(de_batalla(&b), delta(&[(BatallasGanadasEnInferioridad, 1)]));
        b.defensor.soldados = 30;
        assert!(de_batalla(&b).es_vacio(), "con los mismos soldados no es inferioridad");
    }

    #[test]
    fn cada_escuadra_de_elite_de_cualquier_bando_que_acaba_sin_soldados_suma_una() {
        let mut b = sencilla();
        b.atacante.escuadras = vec![esc(Tipo::Barracon, 5, true), esc(Tipo::Caballerizas, 4, false)];
        b.defensor.escuadras = vec![esc(Tipo::GaleriaDeTiro, 4, true), esc(Tipo::Barracon, 3, true)];
        assert_eq!(de_batalla(&b).get(EscuadrasEliteDestruidas), 2, "la de escalón 3 y la que sigue en pie no cuentan");
        b.atacante.escuadras = vec![esc(Tipo::Barracon, 5, false)];
        b.defensor.escuadras = vec![esc(Tipo::Barracon, 3, true)];
        assert!(de_batalla(&b).es_vacio());
    }

    #[test]
    fn caballeria_que_gana_a_campo_abierto_y_la_de_elite() {
        let mut b = sencilla();
        b.atacante.escuadras.push(esc(Tipo::Caballerizas, 3, false));
        assert_eq!(de_batalla(&b), delta(&[(BatallasCampoAbiertoConCaballeria, 1)]));
        b.atacante.escuadras.push(esc(Tipo::Caballerizas, 4, false));
        assert_eq!(de_batalla(&b), delta(&[(BatallasCampoAbiertoConCaballeria, 1), (BatallasCampoAbiertoConCaballeriaElite, 1)]));
        // Fuera del campo abierto, o si la caballería pierde, no.
        b.campo_abierto = false;
        assert!(de_batalla(&b).es_vacio());
        b.campo_abierto = true;
        b.gana_el_atacante = false;
        assert_eq!(de_batalla(&b), delta(&[(BatallasGanadasEnInferioridad, 1)]));
    }

    #[test]
    fn armas_combinadas_pide_los_tres_edificios_en_un_mismo_bando() {
        let mut b = sencilla();
        b.atacante.escuadras = vec![esc(Tipo::Barracon, 1, false), esc(Tipo::GaleriaDeTiro, 1, false)];
        b.defensor.escuadras = vec![esc(Tipo::Caballerizas, 2, false)];
        assert!(de_batalla(&b).es_vacio(), "repartidos entre los dos bandos no cuentan");
        b.defensor.escuadras = vec![esc(Tipo::Barracon, 1, false), esc(Tipo::GaleriaDeTiro, 1, false), esc(Tipo::Caballerizas, 2, false)];
        assert_eq!(de_batalla(&b), delta(&[(BatallasArmasCombinadas, 1)]), "aunque pierda");
    }

    #[test]
    fn contra_coalicion_gana_una_sola_faccion_a_un_bando_de_dos() {
        let mut b = sencilla();
        b.defensor.facciones = [2, 3].into_iter().collect();
        assert_eq!(de_batalla(&b), delta(&[(BatallasContraCoalicion, 1)]));
        let mut gana_la_coalicion = b.clone();
        (gana_la_coalicion.gana_el_atacante, gana_la_coalicion.defensor.soldados) = (false, 40);
        assert!(de_batalla(&gana_la_coalicion).es_vacio(), "si gana la coalición, no");
        b.atacante.facciones = [1, 4].into_iter().collect();
        assert!(de_batalla(&b).es_vacio(), "coalición contra coalición, tampoco");
    }

    #[test]
    fn con_rey_suma_si_combate_un_rey() {
        let mut b = sencilla();
        b.con_rey = true;
        assert_eq!(de_batalla(&b), delta(&[(BatallasConRey, 1)]));
    }

    #[test]
    fn una_persecucion_cuenta_si_el_atacante_lleva_caballeria() {
        let mut b = sencilla().atacante;
        assert!(persecucion(&b).es_vacio());
        b.escuadras.push(esc(Tipo::Caballerizas, 1, true));
        assert_eq!(persecucion(&b), delta(&[(PersecucionesDeCaballeria, 1)]), "gane o pierda");
    }

    #[test]
    fn el_arma_de_hierro_de_calidad_cuenta_y_la_de_hierro_no() {
        assert_eq!(de_fabricacion(Recurso::ArmaHierroCalidad), &[FabricadoArmaHierroCalidad]);
        assert!(de_fabricacion(Recurso::ArmaHierro).is_empty());
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
