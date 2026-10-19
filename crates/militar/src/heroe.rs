//! El héroe como personaje (Doc 5.16, Heroe_Y_Tropa §2-§4, §6, §8 y §9): con qué nace, cómo reparte sus puntos, sus loadouts, el orden de
//! sus escuadras y estar herido. Sus escuadras viven en él (`escuadras`), estén donde estén.
//!
//! Lo que el servidor guarda y sirve sin interpretar (clase, avatar, género, perks, inventario, equipo y habilidades de las escuadras) es de
//! Conquest y no se modela aquí.

use crate::datos::Reglas;
use crate::escuadra::Escuadra;
use crate::liderazgo;
use crate::presencia::{Presencia, Ubicacion};
use crate::tipos::{AsentamientoId, CampamentoId, EscuadraId, HeroeId};
use base::{Instante, Mil};

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Atributos {
    pub fuerza: i64,
    pub destreza: i64,
    pub armadura: i64,
    pub vitalidad: i64,
}

impl Atributos {
    fn como_lista(&self) -> [i64; 4] {
        [self.fuerza, self.destreza, self.armadura, self.vitalidad]
    }
}

/// Las tres monedas del héroe: no tienen relación con el oro de las plazas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Monedas {
    pub bronce: i64,
    pub plata: i64,
    pub oro: i64,
}

/// Una selección de escuadras (y perks) guardada para salir. El activo es el que defiende una plaza en persona. El Liderazgo que suma no se
/// guarda: se deriva.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loadout {
    pub id: LoadoutId,
    pub nombre: String,
    pub escuadras: Vec<EscuadraId>,
    pub perks: Vec<u32>,
    pub activo: bool,
}

/// Lo que pide guardar un loadout. `id` ausente = uno nuevo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatosLoadout {
    pub id: Option<LoadoutId>,
    pub nombre: String,
    pub escuadras: Vec<EscuadraId>,
    pub perks: Vec<u32>,
    /// Ausente = el que tenía, o inactivo si es nuevo.
    pub activo: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorHeroe {
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
    PerkNoDesbloqueado,
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
    /// Puntos de Liderazgo: nace en `LIDERAZGO.base` y nada lo modifica hoy.
    pub liderazgo_base: i64,
    pub presencia: Presencia,
    pub residencia: Option<Residencia>,
    pub nivel: u32,
    /// Experiencia hacia el siguiente nivel (no acumulada).
    pub experiencia: i64,
    pub puntos_de_atributo: i64,
    pub puntos_de_perk: i64,
    pub atributos: Atributos,
    pub perks_desbloqueados: Vec<u32>,
    pub loadouts: Vec<Loadout>,
    pub monedas: Monedas,
    pub herido_hasta: Option<Instante>,
    /// Los víveres que lleva consigo cuando no va en una columna (la columna lleva los suyos mientras marcha).
    pub viveres: Mil,
    /// **Todas** sus escuadras, estén donde estén.
    pub escuadras: Vec<Escuadra>,
}

impl Heroe {
    /// Un héroe recién creado (`crearHeroe`): nace dentro de un campamento de mercenarios y reside en él, sin columna, de nivel 1, sin puntos
    /// ni escuadras, con su bronce inicial y un loadout «Default» vacío y activo.
    pub fn nuevo(r: &Reglas, id: HeroeId, controlador: Controlador, campamento: CampamentoId, loadout_inicial: LoadoutId) -> Self {
        Self {
            id,
            controlador,
            liderazgo_base: r.liderazgo.base,
            presencia: Presencia::nueva(Ubicacion::Mercenarios(campamento)),
            residencia: Some(Residencia::Campamento(campamento)),
            nivel: 1,
            experiencia: 0,
            puntos_de_atributo: 0,
            puntos_de_perk: 0,
            atributos: Atributos::default(),
            perks_desbloqueados: vec![],
            loadouts: vec![Loadout { id: loadout_inicial, nombre: "Default".into(), escuadras: vec![], perks: vec![], activo: true }],
            monedas: Monedas { bronce: r.heroe.bronce_inicial, plata: 0, oro: 0 },
            herido_hasta: None,
            viveres: 0,
            escuadras: vec![],
        }
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

    /// Reparte puntos de atributo (`fuerza, destreza, armadura, vitalidad`): cada punto suma 1. Todo o nada.
    pub fn repartir_atributos(&mut self, r: &Reglas, reparto: Atributos) -> Result<(), ErrorHeroe> {
        let pedidos: i64 = reparto.como_lista().iter().sum();
        if reparto.como_lista().iter().any(|&p| p < 0) {
            return Err(ErrorHeroe::PuntosNegativos);
        }
        if pedidos == 0 {
            return Err(ErrorHeroe::NingunPunto);
        }
        if pedidos > self.puntos_de_atributo {
            return Err(ErrorHeroe::PuntosInsuficientes { pedidos, sin_gastar: self.puntos_de_atributo });
        }
        let nuevo = Atributos {
            fuerza: self.atributos.fuerza + reparto.fuerza,
            destreza: self.atributos.destreza + reparto.destreza,
            armadura: self.atributos.armadura + reparto.armadura,
            vitalidad: self.atributos.vitalidad + reparto.vitalidad,
        };
        if nuevo.como_lista().iter().any(|&a| a > r.heroe.tope_atributo) {
            return Err(ErrorHeroe::TopeDeAtributo);
        }
        self.atributos = nuevo;
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
        if datos.perks.iter().any(|p| !self.perks_desbloqueados.contains(p)) {
            return Err(ErrorHeroe::PerkNoDesbloqueado);
        }
        let loadout = Loadout {
            id: anterior.as_ref().map_or_else(nuevo_id, |a| a.id),
            nombre,
            escuadras: datos.escuadras,
            perks: datos.perks,
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

    fn heroe() -> Heroe {
        Heroe::nuevo(&r(), 1, Controlador::Humano, 3, 100)
    }

    fn con_escuadras(h: &mut Heroe, tropas: &[Tropa]) {
        let r = r();
        for (i, &t) in tropas.iter().enumerate() {
            h.escuadras.push(Escuadra::nueva(&r, 10 + i as u64, h.id, t, r.tropas.tropa(t).unidades, Some(1), 0));
        }
    }

    fn datos(id: Option<LoadoutId>, nombre: &str, escuadras: &[EscuadraId]) -> DatosLoadout {
        DatosLoadout { id, nombre: nombre.into(), escuadras: escuadras.to_vec(), perks: vec![], activo: None }
    }

    #[test]
    fn nace_en_un_campamento_con_progresion_inicial_y_loadout_default() {
        let h = heroe();
        assert_eq!((h.nivel, h.experiencia, h.puntos_de_atributo, h.puntos_de_perk), (1, 0, 0, 0));
        assert_eq!(h.monedas, Monedas { bronce: 500, plata: 0, oro: 0 });
        assert_eq!((h.liderazgo_base, h.atributos), (100, Atributos::default()));
        assert_eq!(h.presencia.ubicacion, Ubicacion::Mercenarios(3));
        assert_eq!(h.residencia, Some(Residencia::Campamento(3)));
        assert_eq!(h.loadouts, vec![Loadout { id: 100, nombre: "Default".into(), escuadras: vec![], perks: vec![], activo: true }]);
        assert!(h.escuadras.is_empty() && h.herido_hasta.is_none());
    }

    #[test]
    fn repartir_puntos_suma_uno_por_punto_y_valida_todo_o_nada() {
        let r = r();
        let mut h = heroe();
        h.puntos_de_atributo = 5;
        assert_eq!(h.repartir_atributos(&r, Atributos { fuerza: 2, vitalidad: 1, ..Default::default() }), Ok(()));
        assert_eq!((h.atributos.fuerza, h.atributos.vitalidad, h.puntos_de_atributo), (2, 1, 2));
        assert_eq!(h.repartir_atributos(&r, Atributos::default()), Err(ErrorHeroe::NingunPunto));
        assert_eq!(h.repartir_atributos(&r, Atributos { destreza: -1, fuerza: 3, ..Default::default() }), Err(ErrorHeroe::PuntosNegativos));
        assert_eq!(h.repartir_atributos(&r, Atributos { armadura: 3, ..Default::default() }), Err(ErrorHeroe::PuntosInsuficientes { pedidos: 3, sin_gastar: 2 }));
        assert_eq!((h.atributos.armadura, h.puntos_de_atributo), (0, 2), "un rechazo no deja rastro");
    }

    #[test]
    fn un_atributo_no_pasa_del_tope_y_el_comando_entero_se_aborta() {
        let r = r();
        let mut h = heroe();
        h.puntos_de_atributo = 500;
        h.atributos.fuerza = 99;
        assert_eq!(h.repartir_atributos(&r, Atributos { fuerza: 2, destreza: 1, ..Default::default() }), Err(ErrorHeroe::TopeDeAtributo));
        assert_eq!((h.atributos.fuerza, h.atributos.destreza, h.puntos_de_atributo), (99, 0, 500));
        assert_eq!(h.repartir_atributos(&r, Atributos { fuerza: 1, ..Default::default() }), Ok(()));
        assert_eq!(h.atributos.fuerza, 100);
    }

    #[test]
    fn guardar_un_loadout_nuevo_lo_deja_inactivo_al_final_y_devuelve_su_liderazgo() {
        let r = r();
        let mut h = heroe();
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
        let mut h = heroe();
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
        let mut h = heroe();
        con_escuadras(&mut h, &[Tropa::Hequetai, Tropa::ArquerosEscitas, Tropa::ArquerosCompuesto]); // 10, 11, 12: tres élites, 45 cada una
        let nuevo = || 999;
        assert_eq!(h.guardar_loadout(&r, datos(None, "   ", &[]), nuevo), Err(ErrorHeroe::LoadoutSinNombre));
        assert_eq!(h.guardar_loadout(&r, datos(Some(55), "x", &[]), nuevo), Err(ErrorHeroe::LoadoutDesconocido));
        assert_eq!(h.guardar_loadout(&r, datos(None, "x", &[10, 10]), nuevo), Err(ErrorHeroe::EscuadraRepetida));
        assert_eq!(h.guardar_loadout(&r, datos(None, "x", &[10, 77]), nuevo), Err(ErrorHeroe::EscuadraAjena));
        assert_eq!(h.guardar_loadout(&r, datos(None, "x", &[10, 11, 12]), nuevo), Err(ErrorHeroe::SuperaLiderazgo));
        assert_eq!(h.guardar_loadout(&r, DatosLoadout { perks: vec![7], ..datos(None, "x", &[10]) }, nuevo), Err(ErrorHeroe::PerkNoDesbloqueado));
        assert_eq!(h.loadouts.len(), 1, "los rechazos no dejan rastro");
        h.perks_desbloqueados.push(7);
        assert!(h.guardar_loadout(&r, DatosLoadout { perks: vec![7], ..datos(None, "x", &[10, 11]) }, nuevo).is_ok(), "90 caben, y el perk ya es suyo");
    }

    #[test]
    fn el_loadout_no_mira_donde_estan_las_escuadras_ni_si_estan_en_otro() {
        let r = r();
        let mut h = heroe();
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
        let mut hs = vec![heroe(), Heroe::nuevo(&r, 2, Controlador::Bot, 3, 200), Heroe::nuevo(&r, 3, Controlador::Humano, 3, 300)];
        hs[1].herir(&r, 0);
        let heridos = herir(&r, &mut hs, &[1, 2, 99], 1_000);
        assert_eq!(heridos, vec![1], "el 2 ya lo estaba y el 99 no existe");
        assert_eq!(heridos_en(&hs, 1_000), vec![1, 2]);
        assert_eq!(heridos_en(&hs, 10 * MS_POR_MINUTO), Vec::<u64>::new());
    }
}
