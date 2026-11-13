//! El héroe como personaje (Doc 5.16, Heroe_Y_Tropa §2-§4, §6, §8 y §9): con qué nace, cómo sube de nivel y reparte sus puntos, sus loadouts,
//! el orden de sus escuadras y estar herido. Sus escuadras viven en él (`escuadras`), estén donde estén.
//!
//! La clase sale del arma puesta y los atributos son la base de esa clase más los puntos repartidos (Objetos_Del_Heroe §3.2, D16): el héroe guarda solo
//! los puntos, su equipo, su mochila y su almacén de objetos (D22), y las reglas de los objetos son del crate `objetos`. El avatar y el género los elige el jugador en Conquest
//! y los guarda el núcleo (`estado_militar::Aspecto`), no este crate. Las habilidades y los perks se abren solos con el nivel y los define Conquest (D15): el motor no los guarda.

use crate::datos::Reglas;
use crate::escuadra::Escuadra;
use crate::liderazgo;
use crate::presencia::{Presencia, Ubicacion};
use crate::tipos::{AsentamientoId, CampamentoId, EscuadraId, HeroeId};
use base::{Instante, Mil};
use objetos::datos::Atributos;
use objetos::{Equipo, Inventario};

pub type LoadoutId = u64;

/// Quién lo controla: decide si una batalla lleva Unity (hay algún humano) o se resuelve con números.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Controlador {
    Humano,
    Bot,
}

/// Dónde reside (Doc 2.5, versión mínima del sistema 4): una plaza o un campamento de mercenarios. La ciudadanía, los cargos, las casas
/// compradas y el cooldown son del sistema político.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Residencia {
    Asentamiento(AsentamientoId),
    Campamento(CampamentoId),
}

/// Lo que da por hecho quien lee la clase o los atributos de un héroe: su arma la puso el equipo inicial o `Equipo::equipar`, que solo ponen armas del catálogo.
const ARMA_DEL_CATALOGO: &str = "el arma puesta es un arma del catálogo (la ponen el equipo inicial y equipar, que lo comprueban)";

fn como_lista(a: Atributos) -> [i64; 4] {
    [a.fuerza, a.destreza, a.armadura, a.vitalidad]
}

/// Una selección de escuadras guardada para salir. El activo es el que defiende una plaza en persona. El Liderazgo que suma no se
/// guarda: se deriva.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loadout {
    pub id: LoadoutId,
    pub nombre: String,
    pub escuadras: Vec<EscuadraId>,
    pub activo: bool,
}

/// Lo que pide guardar un loadout. `id` ausente = uno nuevo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatosLoadout {
    pub id: Option<LoadoutId>,
    pub nombre: String,
    pub escuadras: Vec<EscuadraId>,
    /// Ausente = el que tenía, o inactivo si es nuevo.
    pub activo: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorHeroe {
    /// La clase pedida al crearlo no está en `CLASES_HEROE` (Heroe_Y_Tropa §3).
    ClaseDesconocida,
    /// Una cantidad de puntos negativa.
    PuntosNegativos,
    /// El reparto suma 0.
    NingunPunto,
    /// Más puntos que los que tiene sin gastar.
    PuntosInsuficientes { pedidos: i64, sin_gastar: i64 },
    /// Un atributo pasaría del tope.
    TopeDeAtributo,
    LoadoutSinNombre,
    LoadoutDesconocido,
    EscuadraRepetida,
    EscuadraAjena,
    /// Las escuadras superan su Liderazgo.
    SuperaLiderazgo,
    /// No reside en ningún asentamiento: no tiene guarnición.
    SinResidencia,
    EnBatalla,
    NoEnCampamento,
    YaEnGuarnicion,
    SuperaElCupo { ocupado: i64, cupo: i64 },
    NoEnGuarnicion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heroe {
    pub id: HeroeId,
    pub controlador: Controlador,
    /// Puntos de Liderazgo: nace en `LIDERAZGO.base` y sube `porNivel` con cada nivel hasta el de tope, sin pasar de `LIDERAZGO.tope`. Va en su propio
    /// campo para poder sumarle modificadores más adelante (D14); hoy nada más lo cambia.
    pub liderazgo_base: i64,
    pub presencia: Presencia,
    pub residencia: Option<Residencia>,
    /// El nivel, guardado al lado de la experiencia como en la escuadra (D14): sale de ella al ganarla ([`Heroe::con_experiencia`]).
    pub nivel: u32,
    /// Experiencia **acumulada**, en milésimas de punto (con números puede ganar 0,5). Lo que le falta para el siguiente nivel se calcula al proyectar.
    pub experiencia: Mil,
    /// Puntos de atributo sin gastar: uno por nivel hasta el de tope.
    pub puntos_de_atributo: i64,
    /// Los puntos que ha repartido (D16): sus atributos son estos más la base de la clase de su arma ([`Heroe::atributos`]).
    pub puntos_repartidos: Atributos,
    /// Lo que lleva puesto (Objetos_Del_Heroe §3.2): el arma, que define su clase, las cinco piezas de armadura y la montura.
    pub equipo: Equipo,
    /// Su mochila (`OBJETOS.casillasMochila`): va con él.
    pub mochila: Inventario,
    /// Su almacén de objetos (`OBJETOS.casillasAlmacen`, D22): está en su residencia y se traslada con él.
    pub almacen_objetos: Inventario,
    pub loadouts: Vec<Loadout>,
    /// Sus monedas (tanda 69, 2026-11-08): un solo saldo de monedas sueltas, aparte del oro de las plazas.
    pub monedas: i64,
    pub herido_hasta: Option<Instante>,
    /// Los víveres que lleva consigo cuando no va en una columna (la columna lleva los suyos mientras marcha).
    pub viveres: Mil,
    /// Cuándo recibió la última ración gratis del campamento de mercenarios donde reside (`MERCENARIOS.racion.cadaMinutos`).
    pub racion_en: Option<Instante>,
    /// **Todas** sus escuadras, estén donde estén.
    pub escuadras: Vec<Escuadra>,
}

impl Heroe {
    /// Un héroe recién creado (`crearHeroe`): nace dentro de un campamento de mercenarios y reside en él, sin columna, de nivel 1, sin puntos
    /// ni escuadras, con sus monedas iniciales, un loadout «Default» vacío y activo, el equipo inicial de su `clase` puesto y la mochila y el almacén de
    /// objetos vacíos (Heroe_Y_Tropa §3). Una clase que no está en `CLASES_HEROE` se rechaza.
    pub fn nuevo(r: &Reglas, id: HeroeId, controlador: Controlador, campamento: CampamentoId, loadout_inicial: LoadoutId, clase: &str) -> Result<Self, ErrorHeroe> {
        let equipo = Equipo::inicial(&r.objetos, clase).map_err(|_| ErrorHeroe::ClaseDesconocida)?;
        Ok(Self {
            id,
            controlador,
            liderazgo_base: r.liderazgo.base,
            presencia: Presencia::nueva(Ubicacion::Mercenarios(campamento)),
            residencia: Some(Residencia::Campamento(campamento)),
            nivel: 1,
            experiencia: 0,
            puntos_de_atributo: 0,
            puntos_repartidos: Atributos::default(),
            equipo,
            mochila: Inventario::vacio(r.objetos.casillas_mochila),
            almacen_objetos: Inventario::vacio(r.objetos.casillas_almacen),
            loadouts: vec![Loadout { id: loadout_inicial, nombre: "Default".into(), escuadras: vec![], activo: true }],
            monedas: r.heroe.monedas_iniciales,
            herido_hasta: None,
            viveres: 0,
            racion_en: None,
            escuadras: vec![],
        })
    }

    /// Su clase: la del arma puesta (Objetos_Del_Heroe §3.2).
    pub fn clase<'a>(&self, r: &'a Reglas) -> &'a str {
        self.equipo.clase(&r.objetos).expect(ARMA_DEL_CATALOGO)
    }

    /// Sus atributos (Objetos_Del_Heroe §3.2, D16): la base de la clase de su arma más los puntos repartidos. Lo que da el equipo no cuenta aquí: lo suma Conquest.
    pub fn atributos(&self, r: &Reglas) -> Atributos {
        self.equipo.atributos(&r.objetos, self.puntos_repartidos).expect(ARMA_DEL_CATALOGO)
    }

    pub fn escuadra(&self, id: EscuadraId) -> Option<&Escuadra> {
        self.escuadras.iter().find(|e| e.id == id)
    }

    pub fn escuadra_mut(&mut self, id: EscuadraId) -> Option<&mut Escuadra> {
        self.escuadras.iter_mut().find(|e| e.id == id)
    }

    /// Su escuadra de esta tropa, sin contar la prestada: como mucho una en toda la partida.
    pub fn escuadra_de(&self, tropa: crate::catalogo::Tropa) -> Option<&Escuadra> {
        self.escuadras.iter().find(|e| e.tropa == tropa && e.prestada.is_none())
    }

    /// Lo que le falta para el siguiente nivel (milésimas).
    pub fn falta_para_subir(&self, r: &Reglas) -> Mil {
        experiencia_para_nivel(r, self.nivel + 1) - self.experiencia
    }

    /// Suma experiencia de combate (milésimas) y sube los niveles que cruce (Heroe_Y_Tropa §4.3). Cada nivel hasta `nivelDeTope` da un punto de
    /// atributo; el Liderazgo pasa al de su nivel nuevo ([`liderazgo_para_nivel`], 100 en el `LIDERAZGO.nivelDelTope`) si es mayor; desde el siguiente al de tope, el nivel
    /// sube sin dar nada. Devuelve si subió.
    pub fn con_experiencia(&mut self, r: &Reglas, ganada: Mil) -> bool {
        let antes = self.nivel;
        self.experiencia += ganada.max(0);
        while self.experiencia >= experiencia_para_nivel(r, self.nivel + 1) {
            self.nivel += 1;
            if self.nivel <= r.progresion.nivel_de_tope {
                self.puntos_de_atributo += 1;
            }
            self.liderazgo_base = self.liderazgo_base.max(liderazgo_para_nivel(r, self.nivel));
        }
        self.nivel != antes
    }

    /// Reparte puntos de atributo (`fuerza, destreza, armadura, vitalidad`): cada punto suma 1. El tope de un atributo mira la suma con la base de la clase de su arma
    /// (Objetos_Del_Heroe §3.2). Todo o nada.
    pub fn repartir_atributos(&mut self, r: &Reglas, reparto: Atributos) -> Result<(), ErrorHeroe> {
        let pedidos: i64 = como_lista(reparto).iter().sum();
        if como_lista(reparto).iter().any(|&p| p < 0) {
            return Err(ErrorHeroe::PuntosNegativos);
        }
        if pedidos == 0 {
            return Err(ErrorHeroe::NingunPunto);
        }
        if pedidos > self.puntos_de_atributo {
            return Err(ErrorHeroe::PuntosInsuficientes { pedidos, sin_gastar: self.puntos_de_atributo });
        }
        let nuevo = self.puntos_repartidos.mas(reparto);
        if como_lista(self.equipo.atributos(&r.objetos, nuevo).expect(ARMA_DEL_CATALOGO)).iter().any(|&a| a > r.heroe.tope_atributo) {
            return Err(ErrorHeroe::TopeDeAtributo);
        }
        self.puntos_repartidos = nuevo;
        self.puntos_de_atributo -= pedidos;
        Ok(())
    }

    /// Liderazgo que suman las escuadras de un loadout: se deriva al servir, nunca se guarda.
    pub fn liderazgo_de_loadout(&self, r: &Reglas, loadout: &Loadout) -> i64 {
        liderazgo::comprometido(r, loadout.escuadras.iter().filter_map(|&id| self.escuadra(id)))
    }

    /// Guarda o reescribe un loadout y devuelve su id y el Liderazgo que suma. No mira dónde están las escuadras (se comprueba al salir con
    /// ellas). Marcarlo activo desactiva los demás; uno nuevo se añade al final. No hay tope de loadouts y la misma escuadra puede estar en
    /// varios.
    pub fn guardar_loadout(&mut self, r: &Reglas, datos: DatosLoadout, nuevo_id: impl FnOnce() -> LoadoutId) -> Result<(LoadoutId, i64), ErrorHeroe> {
        let nombre = datos.nombre.trim().to_owned();
        if nombre.is_empty() {
            return Err(ErrorHeroe::LoadoutSinNombre);
        }
        let anterior = match datos.id {
            Some(id) => Some(self.loadouts.iter().find(|l| l.id == id).ok_or(ErrorHeroe::LoadoutDesconocido)?.clone()),
            None => None,
        };
        let mut vistas = std::collections::BTreeSet::new();
        if !datos.escuadras.iter().all(|&id| vistas.insert(id)) {
            return Err(ErrorHeroe::EscuadraRepetida);
        }
        let suyas: Vec<&Escuadra> = datos.escuadras.iter().map(|&id| self.escuadra(id).ok_or(ErrorHeroe::EscuadraAjena)).collect::<Result<_, _>>()?;
        let total = liderazgo::comprometido(r, suyas);
        if total > self.liderazgo_base {
            return Err(ErrorHeroe::SuperaLiderazgo);
        }
        let loadout = Loadout {
            id: anterior.as_ref().map_or_else(nuevo_id, |a| a.id),
            nombre,
            escuadras: datos.escuadras,
            activo: datos.activo.or(anterior.as_ref().map(|a| a.activo)).unwrap_or(false),
        };
        let id = loadout.id;
        if loadout.activo {
            self.loadouts.iter_mut().for_each(|l| l.activo = false);
        }
        match self.loadouts.iter_mut().find(|l| l.id == id) {
            Some(l) => *l = loadout,
            None => self.loadouts.push(loadout),
        }
        Ok((id, total))
    }

    /// Borra un loadout. No garantiza que quede uno activo: sin loadout activo no se defiende en persona (la guarnición sí).
    pub fn borrar_loadout(&mut self, id: LoadoutId) -> Result<(), ErrorHeroe> {
        let i = self.loadouts.iter().position(|l| l.id == id).ok_or(ErrorHeroe::LoadoutDesconocido)?;
        self.loadouts.remove(i);
        Ok(())
    }

    /// Las escuadras del loadout activo (las que defienden la plaza en persona), en su orden.
    pub fn escuadras_del_loadout_activo(&self) -> &[EscuadraId] {
        self.loadouts.iter().find(|l| l.activo).map_or(&[], |l| &l.escuadras)
    }

    /// Pone las escuadras del héroe en el orden que elige: las de `ids` primero y el resto detrás como estaban. Es el orden en que entran
    /// en combate. Si va en una columna (`columna`, sus ids de escuadra), las suyas ocupan **los mismos huecos que ya tenían**, en el orden
    /// nuevo; las de otros héroes no se mueven.
    pub fn ordenar_escuadras(&mut self, ids: &[EscuadraId], columna: Option<&mut Vec<EscuadraId>>) -> Result<(), ErrorHeroe> {
        let mut vistas = std::collections::BTreeSet::new();
        if !ids.iter().all(|&id| vistas.insert(id)) {
            return Err(ErrorHeroe::EscuadraRepetida);
        }
        if ids.iter().any(|&id| self.escuadra(id).is_none()) {
            return Err(ErrorHeroe::EscuadraAjena);
        }
        let mut orden: Vec<Escuadra> = ids.iter().map(|&id| self.escuadra(id).expect("existe").clone()).collect();
        orden.extend(self.escuadras.iter().filter(|e| !vistas.contains(&e.id)).cloned());
        if let Some(columna) = columna {
            let en_la_columna: std::collections::BTreeSet<_> = columna.iter().copied().collect();
            let mut propias = orden.iter().map(|e| e.id).filter(|id| en_la_columna.contains(id));
            for hueco in columna.iter_mut().filter(|id| self.escuadras.iter().any(|e| e.id == **id)) {
                *hueco = propias.next().expect("tantas propias como huecos");
            }
        }
        self.escuadras = orden;
        Ok(())
    }

    /// ¿Está herido ahora? Se comprueba **al leer**: nada se dispara al vencer.
    pub fn esta_herido(&self, ahora: Instante) -> bool {
        self.herido_hasta.is_some_and(|h| ahora < h)
    }

    /// Hiere al héroe `heridoMinutos`. A quien ya lo está no se le alarga: un herido no entra en batallas, así que no ha podido volver a
    /// perder. Devuelve si se le hirió.
    pub fn herir(&mut self, r: &Reglas, ahora: Instante) -> bool {
        if self.esta_herido(ahora) {
            return false;
        }
        self.herido_hasta = Some(ahora + r.heroe.herido);
        true
    }
}

/// Lo que pide pasar del nivel `nivel` (1 o más) al siguiente (2026-11-13): `experienciaBase` del 1 al 2 y, desde ahí, lo del anterior más la `suma` del último tramo de
/// `incrementos` que ha empezado; desde el `nivelDeTope`, lo mismo que en él. Con la curva de hoy: 3, 6… 27 (al 10), 37… 127 (al 20), 147… 727 (al 50) y 747 desde ahí.
pub fn experiencia_para_subir(r: &Reglas, nivel: u32) -> Mil {
    let p = &r.progresion;
    (2..=nivel.min(p.nivel_de_tope)).fold(p.experiencia_base, |pide, n| pide + p.incrementos.iter().rev().find(|t| t.0 <= n).map_or(0, |t| t.1))
}

/// La experiencia acumulada con la que se llega al nivel `nivel` (1 o más): la suma de lo que pide cada nivel por debajo ([`experiencia_para_subir`]). Con la curva de hoy,
/// 135 en el 10, 955 en el 20, 3.325 en el 30 y 14.065 en el 50.
pub fn experiencia_para_nivel(r: &Reglas, nivel: u32) -> Mil {
    let tope = r.progresion.nivel_de_tope;
    let curva: Mil = (1..nivel.min(tope).max(1)).map(|n| experiencia_para_subir(r, n)).sum();
    curva + i64::from(nivel.saturating_sub(tope)) * experiencia_para_subir(r, tope)
}

/// El Liderazgo del nivel `nivel` (2026-11-13): de `LIDERAZGO.base` en el 1 a `LIDERAZGO.tope` en el `nivelDelTope`, repartido a partes iguales y redondeado hacia abajo
/// (14, 17, 21, 24… 100 en el 25 con los números de hoy).
pub fn liderazgo_para_nivel(r: &Reglas, nivel: u32) -> i64 {
    let l = &r.liderazgo;
    let tramo = i64::from(l.nivel_del_tope.max(2) - 1);
    l.base + (l.tope - l.base) * i64::from(nivel.clamp(1, l.nivel_del_tope) - 1) / tramo
}

/// Los ids de los héroes heridos en `ahora` (lo que los resolutores de combate necesitan para apartarlos).
pub fn heridos_en(heroes: &[Heroe], ahora: Instante) -> Vec<HeroeId> {
    heroes.iter().filter(|h| h.esta_herido(ahora)).map(|h| h.id).collect()
}

/// Hiere a los héroes `ids` (el bando que pierde). Devuelve a quiénes se hirió de verdad.
pub fn herir(r: &Reglas, heroes: &mut [Heroe], ids: &[HeroeId], ahora: Instante) -> Vec<HeroeId> {
    heroes.iter_mut().filter(|h| ids.contains(&h.id)).filter_map(|h| h.herir(r, ahora).then_some(h.id)).collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::catalogo::Tropa;
    use crate::escuadra::Contenedor;
    use base::MS_POR_MINUTO;

    fn r() -> Reglas {
        Reglas::cargar()
    }

    /// La clase de los héroes de estas pruebas: base 3/3/2/2 y armaduras media y pesada.
    const CLASE: &str = "espada_escudo_mediano";

    fn heroe() -> Heroe {
        Heroe::nuevo(&r(), 1, Controlador::Humano, 3, 100, CLASE).expect("una clase del catálogo")
    }

    /// Un héroe con el Liderazgo del tope (100), como si hubiera llegado al nivel 25: el de los loadouts con varias escuadras.
    fn heroe_de_tope() -> Heroe {
        Heroe { liderazgo_base: r().liderazgo.tope, ..heroe() }
    }

    fn con_escuadras(h: &mut Heroe, tropas: &[Tropa]) {
        let r = r();
        for (i, &t) in tropas.iter().enumerate() {
            h.escuadras.push(Escuadra::nueva(&r, 10 + i as u64, h.id, t, r.tropas.tropa(t).unidades, Some(1), 0));
        }
    }

    fn datos(id: Option<LoadoutId>, nombre: &str, escuadras: &[EscuadraId]) -> DatosLoadout {
        DatosLoadout { id, nombre: nombre.into(), escuadras: escuadras.to_vec(), activo: None }
    }

    #[test]
    fn nace_en_un_campamento_con_progresion_inicial_y_loadout_default() {
        let h = heroe();
        assert_eq!((h.nivel, h.experiencia, h.puntos_de_atributo), (1, 0, 0));
        assert_eq!(h.monedas, 1_500);
        assert_eq!((h.liderazgo_base, h.puntos_repartidos), (14, Atributos::default()), "nace con el Liderazgo del nivel 1");
        assert_eq!(h.presencia.ubicacion, Ubicacion::Mercenarios(3));
        assert_eq!(h.residencia, Some(Residencia::Campamento(3)));
        assert_eq!(h.loadouts, vec![Loadout { id: 100, nombre: "Default".into(), escuadras: vec![], activo: true }]);
        assert!(h.escuadras.is_empty() && h.herido_hasta.is_none());
    }

    #[test]
    fn nace_con_el_equipo_inicial_de_su_clase_puesto_y_la_mochila_y_el_almacen_vacios() {
        let r = r();
        for (id, clase) in &r.objetos.clases {
            let h = Heroe::nuevo(&r, 1, Controlador::Humano, 3, 100, id).unwrap();
            assert_eq!((&h.equipo, h.clase(&r)), (&clase.inicial, id.as_str()), "{id}: la clase es la del arma de inicio");
            assert_eq!(h.atributos(&r), clase.atributos_base, "{id}: sin puntos, la base de su clase");
            assert_eq!((h.mochila, h.almacen_objetos), (Inventario::vacio(72), Inventario::vacio(144)));
        }
        assert_eq!(Heroe::nuevo(&r, 1, Controlador::Humano, 3, 100, "").unwrap_err(), ErrorHeroe::ClaseDesconocida);
        assert_eq!(Heroe::nuevo(&r, 1, Controlador::Humano, 3, 100, "mandoble").unwrap_err(), ErrorHeroe::ClaseDesconocida, "la clase vieja de Conquest no está en CLASES_HEROE");
    }

    #[test]
    fn la_curva_pide_3_y_3_mas_hasta_el_10_10_mas_hasta_el_20_y_20_mas_hasta_el_50_y_luego_747_sin_fin() {
        let r = r();
        let pide = |n: u32| experiencia_para_nivel(&r, n + 1) - experiencia_para_nivel(&r, n);
        assert!((1..120).all(|n| pide(n) == experiencia_para_subir(&r, n)), "lo que pide cada nivel es la diferencia de las acumuladas");
        assert_eq!((experiencia_para_nivel(&r, 1), experiencia_para_nivel(&r, 2), experiencia_para_nivel(&r, 3)), (0, 3_000, 9_000));
        assert_eq!((pide(1), pide(2), pide(3), pide(9)), (3_000, 6_000, 9_000, 27_000), "de 3 en 3 hasta el 10");
        assert_eq!((pide(10), pide(11), pide(19)), (37_000, 47_000, 127_000), "de 10 en 10 hasta el 20");
        assert_eq!((pide(20), pide(21), pide(49)), (147_000, 167_000, 727_000), "de 20 en 20 hasta el 50");
        assert_eq!((experiencia_para_nivel(&r, 10), experiencia_para_nivel(&r, 20), experiencia_para_nivel(&r, 30), experiencia_para_nivel(&r, 50)), (135_000, 955_000, 3_325_000, 14_065_000));
        assert_eq!((pide(50), pide(51), pide(100)), (747_000, 747_000, 747_000), "desde el 50, 747 por nivel sin fin");
    }

    #[test]
    fn subir_de_nivel_da_un_punto_hasta_el_50_y_despues_nada() {
        let r = r();
        let mut h = heroe();
        assert!(!h.con_experiencia(&r, 2_999), "le falta una milésima");
        assert_eq!((h.nivel, h.falta_para_subir(&r)), (1, 1));
        assert!(h.con_experiencia(&r, 1));
        assert_eq!((h.nivel, h.puntos_de_atributo, h.liderazgo_base, h.falta_para_subir(&r)), (2, 1, 17, 6_000));
        // Del 2 al 50 de una vez: cada nivel cruzado cuenta.
        assert!(h.con_experiencia(&r, experiencia_para_nivel(&r, 50) - h.experiencia));
        assert_eq!((h.nivel, h.puntos_de_atributo, h.liderazgo_base, h.falta_para_subir(&r)), (50, 49, 100, 747_000));
        assert!(h.con_experiencia(&r, 747_000 * 3));
        assert_eq!((h.nivel, h.puntos_de_atributo, h.liderazgo_base), (53, 49, 100), "desde el 51 el nivel sube sin dar nada");
    }

    #[test]
    fn el_liderazgo_va_de_14_a_100_en_el_25_de_3_o_4_por_nivel() {
        let r = r();
        let mut h = heroe();
        let mut vistos = vec![h.liderazgo_base];
        for n in 2..=27 {
            h.con_experiencia(&r, experiencia_para_nivel(&r, n) - h.experiencia);
            vistos.push(h.liderazgo_base);
        }
        assert_eq!(&vistos[..6], &[14, 17, 21, 24, 28, 31]);
        assert_eq!((vistos[9], vistos[19], vistos[23], vistos[24], vistos[25], vistos[26]), (46, 82, 96, 100, 100, 100), "nivel 10, 20, 24, 25, 26 y 27");
        assert!(vistos.windows(2).take(24).all(|w| matches!(w[1] - w[0], 3 | 4)), "3 o 4 por nivel hasta el 25");
        assert!((1..=60).all(|n| liderazgo_para_nivel(&r, n) == vistos.get(n as usize - 1).copied().unwrap_or(100)));
    }

    #[test]
    fn repartir_puntos_suma_uno_por_punto_y_valida_todo_o_nada() {
        let r = r();
        let mut h = heroe();
        h.puntos_de_atributo = 5;
        assert_eq!(h.repartir_atributos(&r, Atributos { fuerza: 2, vitalidad: 1, ..Default::default() }), Ok(()));
        assert_eq!((h.puntos_repartidos.fuerza, h.puntos_repartidos.vitalidad, h.puntos_de_atributo), (2, 1, 2));
        assert_eq!(h.atributos(&r), Atributos { fuerza: 5, destreza: 3, armadura: 2, vitalidad: 3 }, "la base de la clase más lo repartido");
        assert_eq!(h.repartir_atributos(&r, Atributos::default()), Err(ErrorHeroe::NingunPunto));
        assert_eq!(h.repartir_atributos(&r, Atributos { destreza: -1, fuerza: 3, ..Default::default() }), Err(ErrorHeroe::PuntosNegativos));
        assert_eq!(h.repartir_atributos(&r, Atributos { armadura: 3, ..Default::default() }), Err(ErrorHeroe::PuntosInsuficientes { pedidos: 3, sin_gastar: 2 }));
        assert_eq!((h.puntos_repartidos.armadura, h.puntos_de_atributo), (0, 2), "un rechazo no deja rastro");
    }

    /// Desde el 2026-11-12 el tope mira la suma con la base de la clase (Objetos_Del_Heroe §3.2): con la fuerza 3 de la espada, 96 puntos dan 99.
    #[test]
    fn un_atributo_no_pasa_del_tope_contando_la_base_y_el_comando_entero_se_aborta() {
        let r = r();
        let mut h = heroe();
        h.puntos_de_atributo = 500;
        h.puntos_repartidos.fuerza = 96;
        assert_eq!(h.repartir_atributos(&r, Atributos { fuerza: 2, destreza: 1, ..Default::default() }), Err(ErrorHeroe::TopeDeAtributo));
        assert_eq!((h.puntos_repartidos.fuerza, h.puntos_repartidos.destreza, h.puntos_de_atributo), (96, 0, 500));
        assert_eq!(h.repartir_atributos(&r, Atributos { fuerza: 1, ..Default::default() }), Ok(()));
        assert_eq!((h.puntos_repartidos.fuerza, h.atributos(&r).fuerza), (97, 100));
    }

    #[test]
    fn guardar_un_loadout_nuevo_lo_deja_inactivo_al_final_y_devuelve_su_liderazgo() {
        let r = r();
        let mut h = heroe_de_tope();
        con_escuadras(&mut h, &[Tropa::Arqueros, Tropa::Hequetai]);
        let (id, lid) = h.guardar_loadout(&r, datos(None, "  Asalto ", &[10, 11]), || 101).unwrap();
        assert_eq!((id, lid), (101, 22 + 45));
        assert_eq!(h.loadouts.len(), 2);
        assert_eq!((h.loadouts[1].nombre.as_str(), h.loadouts[1].activo), ("Asalto", false));
        assert!(h.loadouts[0].activo, "el Default sigue activo");
        assert_eq!(h.liderazgo_de_loadout(&r, &h.loadouts[1].clone()), 67);
    }

    #[test]
    fn marcar_activo_desactiva_los_demas_y_reescribir_por_id_no_crea_otro() {
        let r = r();
        let mut h = heroe_de_tope();
        con_escuadras(&mut h, &[Tropa::Arqueros, Tropa::Honderos]);
        let (a, _) = h.guardar_loadout(&r, DatosLoadout { activo: Some(true), ..datos(None, "A", &[10]) }, || 101).unwrap();
        assert_eq!(h.loadouts.iter().filter(|l| l.activo).map(|l| l.id).collect::<Vec<_>>(), vec![a]);
        let (b, _) = h.guardar_loadout(&r, datos(Some(a), "A2", &[10, 11]), || unreachable!()).unwrap();
        assert_eq!((b, h.loadouts.len(), h.loadouts[1].nombre.as_str(), h.loadouts[1].activo), (a, 2, "A2", true), "conserva el id y el activo");
        assert_eq!(h.escuadras_del_loadout_activo(), &[10, 11]);
        h.borrar_loadout(a).unwrap();
        assert!(h.escuadras_del_loadout_activo().is_empty(), "borrar el activo no activa otro");
        assert_eq!(h.borrar_loadout(a), Err(ErrorHeroe::LoadoutDesconocido));
    }

    #[test]
    fn rechazos_de_loadout_en_su_orden() {
        let r = r();
        let mut h = heroe_de_tope();
        con_escuadras(&mut h, &[Tropa::Hequetai, Tropa::ArquerosEscitas, Tropa::ArquerosCompuesto]); // 10, 11, 12: tres élites, 45 cada una
        let nuevo = || 999;
        assert_eq!(h.guardar_loadout(&r, datos(None, "   ", &[]), nuevo), Err(ErrorHeroe::LoadoutSinNombre));
        assert_eq!(h.guardar_loadout(&r, datos(Some(55), "x", &[]), nuevo), Err(ErrorHeroe::LoadoutDesconocido));
        assert_eq!(h.guardar_loadout(&r, datos(None, "x", &[10, 10]), nuevo), Err(ErrorHeroe::EscuadraRepetida));
        assert_eq!(h.guardar_loadout(&r, datos(None, "x", &[10, 77]), nuevo), Err(ErrorHeroe::EscuadraAjena));
        assert_eq!(h.guardar_loadout(&r, datos(None, "x", &[10, 11, 12]), nuevo), Err(ErrorHeroe::SuperaLiderazgo));
        assert_eq!(h.loadouts.len(), 1, "los rechazos no dejan rastro");
        assert!(h.guardar_loadout(&r, datos(None, "x", &[10, 11]), nuevo).is_ok(), "90 caben");
    }

    #[test]
    fn el_loadout_no_mira_donde_estan_las_escuadras_ni_si_estan_en_otro() {
        let r = r();
        let mut h = heroe_de_tope();
        con_escuadras(&mut h, &[Tropa::Arqueros]);
        h.escuadras[0].contenedor = Contenedor::Ejercito(4);
        h.guardar_loadout(&r, datos(None, "uno", &[10]), || 101).unwrap();
        assert!(h.guardar_loadout(&r, datos(None, "dos", &[10]), || 102).is_ok(), "la misma escuadra puede estar en varios");
    }

    #[test]
    fn ordenar_pone_las_pedidas_primero_y_el_resto_como_estaba() {
        let mut h = heroe();
        con_escuadras(&mut h, &[Tropa::Arqueros, Tropa::Honderos, Tropa::Granjeros, Tropa::Mineros]); // 10, 11, 12, 13
        h.ordenar_escuadras(&[12, 10], None).unwrap();
        assert_eq!(h.escuadras.iter().map(|e| e.id).collect::<Vec<_>>(), vec![12, 10, 11, 13]);
        assert_eq!(h.ordenar_escuadras(&[10, 10], None), Err(ErrorHeroe::EscuadraRepetida));
        assert_eq!(h.ordenar_escuadras(&[99], None), Err(ErrorHeroe::EscuadraAjena));
        assert_eq!(h.escuadras.iter().map(|e| e.id).collect::<Vec<_>>(), vec![12, 10, 11, 13], "un rechazo no cambia nada");
    }

    #[test]
    fn en_una_columna_las_suyas_ocupan_los_mismos_huecos_y_las_ajenas_no_se_mueven() {
        let mut h = heroe();
        con_escuadras(&mut h, &[Tropa::Arqueros, Tropa::Honderos, Tropa::Granjeros]); // 10, 11, 12
        // La columna lleva, mezcladas, escuadras de otro héroe (90, 91) y tres suyas.
        let mut columna = vec![10, 90, 11, 91, 12];
        h.ordenar_escuadras(&[12, 11, 10], Some(&mut columna)).unwrap();
        assert_eq!(columna, vec![12, 90, 11, 91, 10], "sus huecos son los mismos, en el orden nuevo");
        // Solo algunas pedidas: las demás siguen tras ellas, como estaban.
        let mut columna = vec![10, 90, 11, 12];
        h.ordenar_escuadras(&[11], Some(&mut columna)).unwrap();
        assert_eq!(columna, vec![11, 90, 12, 10], "11 primero; 12 y 10 en el orden que tenían en el héroe tras la orden anterior");
    }

    #[test]
    fn herido_se_lee_al_mirar_dura_dos_minutos_y_no_se_alarga() {
        let r = r();
        let mut h = heroe();
        assert!(!h.esta_herido(0));
        assert!(h.herir(&r, 10_000));
        assert_eq!(h.herido_hasta, Some(10_000 + 2 * MS_POR_MINUTO));
        assert!(h.esta_herido(10_000 + 2 * MS_POR_MINUTO - 1));
        assert!(!h.esta_herido(10_000 + 2 * MS_POR_MINUTO), "vence en el instante exacto: nada se dispara");
        assert!(h.herir(&r, 10_000 + 2 * MS_POR_MINUTO), "ya curado, se le puede herir otra vez");
        let hasta = h.herido_hasta;
        assert!(!h.herir(&r, 11_000 + 2 * MS_POR_MINUTO), "a un herido no se le alarga");
        assert_eq!(h.herido_hasta, hasta);
    }

    #[test]
    fn herir_a_un_bando_y_listar_los_heridos() {
        let r = r();
        let mut hs = vec![heroe(), Heroe::nuevo(&r, 2, Controlador::Bot, 3, 200, CLASE).unwrap(), Heroe::nuevo(&r, 3, Controlador::Humano, 3, 300, CLASE).unwrap()];
        hs[1].herir(&r, 0);
        let heridos = herir(&r, &mut hs, &[1, 2, 99], 1_000);
        assert_eq!(heridos, vec![1], "el 2 ya lo estaba y el 99 no existe");
        assert_eq!(heridos_en(&hs, 1_000), vec![1, 2]);
        assert_eq!(heridos_en(&hs, 10 * MS_POR_MINUTO), Vec::<u64>::new());
    }
}
