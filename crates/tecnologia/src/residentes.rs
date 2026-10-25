//! Aedas residentes (TA §7.1): viven en una plaza con Palacio activo y nobleza, llegan de uno en uno mientras hay cupo, se van los últimos en llegar si
//! el cupo baja, siguen en la plaza si cambia de dueño (pierden la épica) y desaparecen con ella. Sin tick (decisión técnica 2): el núcleo llama a
//! [`Residentes::ajustar`] cuando cambia el cupo de una plaza (Palacio, nobleza 0/>0, nivel actual) y programa la llegada con [`llegada`] solo
//! mientras la plaza espera con cupo libre. Sus épicas, en `epicas`.

use crate::datos::Reglas;
use crate::epicas::Epica;
use crate::{AsentamientoId, FaccionId, TecId};
use asentamiento::datos::UNO;
use base::Instante;
use std::collections::BTreeMap;

/// Los nombres de los residentes, en el orden en que llegan (`NOMBRES` de `typescript-final:src/engine/aedasResidentes.ts`).
pub const NOMBRES: [&str; 24] = [
    "Femio", "Demódoco", "Orfeo", "Lino", "Tamiris", "Museo", "Eumolpo", "Filamón", "Arión", "Hesíodo", "Terpandro", "Alcmán", "Arquíloco", "Tirteo", "Mimnermo",
    "Estesícoro", "Íbico", "Anacreonte", "Simónides", "Corina", "Safo", "Alceo", "Olén", "Anfión",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AedaResidente {
    /// El N de `aeda-rN` (decisión 7).
    pub id: u32,
    pub plaza: AsentamientoId,
    /// La dueña de la plaza (cambia con ella).
    pub faccion: FaccionId,
    pub llegada_en: Instante,
    pub epica: Option<Epica>,
}

impl AedaResidente {
    /// `NOMBRES[(N − 1) mod 24]`.
    pub fn nombre(&self) -> &'static str {
        NOMBRES[(self.id as usize - 1) % NOMBRES.len()]
    }
}

/// `EstadoAedasResidentes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Residentes {
    pub aedas: Vec<AedaResidente>,
    /// El N del próximo; solo crece.
    pub siguiente: u32,
    /// Desde cuándo espera cada plaza con cupo libre.
    pub espera_desde: BTreeMap<AsentamientoId, Instante>,
    /// Épicas cumplidas por Facción (Mecenas de los Aedas: `cumplidas × 10 + residentes`).
    pub cumplidas: BTreeMap<FaccionId, u32>,
}

impl Default for Residentes {
    fn default() -> Self {
        Residentes { aedas: Vec::new(), siguiente: 1, espera_desde: BTreeMap::new(), cumplidas: BTreeMap::new() }
    }
}

/// `cupoDeResidentes`: ninguno sin Palacio activo o sin nobleza; si no, `cupoPorNivel[nivel actual]` (0 si el bloque no da cupo; decisión 5).
pub fn cupo(r: &Reglas, palacio_activo: bool, hay_nobleza: bool, nivel_actual: u8) -> u32 {
    if !palacio_activo || !hay_nobleza {
        return 0;
    }
    r.aedas.cupo_por_nivel.get(nivel_actual as usize).copied().unwrap_or(0)
}

/// Qué hacer con una plaza que espera (decisión técnica 6, la semántica del TS «se mide al comparar»): llega uno ahora, o hay que volver a mirar en ese
/// instante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Llegada {
    Llega,
    MirarEn(Instante),
}

/// La llegada de una plaza que espera desde `espera`: vence a `espera + llegada`; si entonces la reputación de la Facción está en el umbral bajo o por
/// debajo (`baja`), vence a `espera + llegada × factor` o cuando la reputación vuelva a subir del umbral (`sube_en`, el cruce de su ritmo), lo primero.
pub fn llegada(r: &Reglas, espera: Instante, ahora: Instante, baja: bool, sube_en: Option<Instante>) -> Llegada {
    let (normal, lenta) = (espera + r.aedas.llegada, espera + r.aedas.llegada * r.aedas.factor_llegada_reputacion_baja);
    if ahora < normal {
        Llegada::MirarEn(normal)
    } else if !baja || ahora >= lenta {
        Llegada::Llega
    } else {
        Llegada::MirarEn(sube_en.map_or(lenta, |s| s.min(lenta)))
    }
}

/// El factor (ppm) de los residentes en la felicidad y en el crecimiento de la nobleza: `1 + bono × residentes` (Economia_Construccion §9.1).
pub fn factor_felicidad(r: &Reglas, residentes: usize) -> i64 {
    UNO + r.aedas.bono_felicidad * residentes as i64
}

pub fn factor_nobleza(r: &Reglas, residentes: usize) -> i64 {
    UNO + r.aedas.bono_nobleza * residentes as i64
}

/// Lo que resulta de ajustar una plaza a su cupo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ajuste {
    /// Los que se van (`aedas.residente_se_va`): los últimos en llegar, en su orden de llegada.
    pub salen: Vec<u32>,
    /// Si queda cupo libre, desde cuándo espera la plaza (para [`llegada`]); `None` con el cupo lleno.
    pub espera: Option<Instante>,
}

impl Residentes {
    /// Los residentes de una plaza, por orden de llegada (y por id).
    pub fn de_plaza(&self, plaza: AsentamientoId) -> Vec<&AedaResidente> {
        let mut v: Vec<&AedaResidente> = self.aedas.iter().filter(|a| a.plaza == plaza).collect();
        v.sort_by_key(|a| (a.llegada_en, a.id));
        v
    }

    pub fn en_plaza(&self, plaza: AsentamientoId) -> usize {
        self.aedas.iter().filter(|a| a.plaza == plaza).count()
    }

    /// Residentes de las plazas de la Facción (para Mecenas de los Aedas).
    pub fn de_faccion(&self, f: FaccionId) -> usize {
        self.aedas.iter().filter(|a| a.faccion == f).count()
    }

    pub fn cumplidas_de(&self, f: FaccionId) -> u32 {
        self.cumplidas.get(&f).copied().unwrap_or(0)
    }

    pub fn get(&self, id: u32) -> Option<&AedaResidente> {
        self.aedas.iter().find(|a| a.id == id)
    }

    /// TA §7.1 paso 3 para una plaza con `cupo`: si sobran, se van los últimos en llegar; con el cupo lleno se olvida la espera; si queda cupo, la
    /// plaza espera desde la primera vez que se la vio con hueco.
    pub fn ajustar(&mut self, plaza: AsentamientoId, cupo: u32, ahora: Instante) -> Ajuste {
        let propios: Vec<u32> = self.de_plaza(plaza).iter().map(|a| a.id).collect();
        let salen: Vec<u32> = propios.iter().skip(cupo as usize).copied().collect();
        self.aedas.retain(|a| !salen.contains(&a.id));
        let espera = if propios.len() >= cupo as usize {
            self.espera_desde.remove(&plaza);
            None
        } else {
            Some(*self.espera_desde.entry(plaza).or_insert(ahora))
        };
        Ajuste { salen, espera }
    }

    /// Llega uno a la plaza (`aedas.residente_llega`); se olvida la espera (si sigue habiendo cupo, la espera vuelve a empezar con el siguiente
    /// [`Residentes::ajustar`]). Devuelve su id.
    pub fn llega(&mut self, plaza: AsentamientoId, faccion: FaccionId, ahora: Instante) -> u32 {
        let id = self.siguiente;
        self.siguiente += 1;
        self.aedas.push(AedaResidente { id, plaza, faccion, llegada_en: ahora, epica: None });
        self.espera_desde.remove(&plaza);
        id
    }

    /// La plaza cambia de dueño (conquista, anexión): los Aedas se quedan, pasan a la nueva dueña y **pierden la épica** (decisión 11: el núcleo emite
    /// `aedas.epica_abandonada { motivo: conquista }` por cada una). Devuelve las épicas perdidas.
    pub fn cambiar_de_dueno(&mut self, plaza: AsentamientoId, nueva: FaccionId) -> Vec<(u32, TecId)> {
        let mut perdidas = Vec::new();
        for a in self.aedas.iter_mut().filter(|a| a.plaza == plaza && a.faccion != nueva) {
            a.faccion = nueva;
            if let Some(e) = a.epica.take() {
                perdidas.push((a.id, e.tec));
            }
        }
        perdidas
    }

    /// La tecnología le apareció a la Facción por otra vía: la épica sobra (`motivo: aparecida`). Devuelve los Aedas que la cantaban.
    pub fn al_aparecer(&mut self, f: FaccionId, t: TecId) -> Vec<u32> {
        let mut sobran = Vec::new();
        for a in self.aedas.iter_mut().filter(|a| a.faccion == f && a.epica.as_ref().is_some_and(|e| e.tec == t)) {
            a.epica = None;
            sobran.push(a.id);
        }
        sobran
    }

    /// La plaza ya no existe (ruina): se lleva a sus Aedas y su espera. Devuelve los que desaparecen.
    pub fn quitar_plaza(&mut self, plaza: AsentamientoId) -> Vec<u32> {
        let fuera: Vec<u32> = self.aedas.iter().filter(|a| a.plaza == plaza).map(|a| a.id).collect();
        self.aedas.retain(|a| a.plaza != plaza);
        self.espera_desde.remove(&plaza);
        fuera
    }
}

#[cfg(test)]
pub(crate) mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/aedasResidentes.test.ts` («ciclo de vida» y «efectos»; Mecenas de los Aedas es de
    //! `politica::titulos`).
    use super::*;
    use crate::epicas::Epica;

    pub fn r() -> Reglas {
        Reglas::default()
    }

    /// Un residente `aeda-r1` en la plaza 10 de la Facción 1, llegado en 0.
    pub fn con_residente(epica: Option<Epica>) -> Residentes {
        Residentes { aedas: vec![AedaResidente { id: 1, plaza: 10, faccion: 1, llegada_en: 0, epica }], siguiente: 2, ..Residentes::default() }
    }

    #[test]
    fn cupo_por_nivel_sin_palacio_o_sin_nobleza_ninguno() {
        let r = r();
        assert_eq!([2, 3, 4, 5].map(|n| cupo(&r, true, true, n)), [1, 2, 3, 4]);
        assert_eq!(cupo(&r, true, false, 3), 0);
        assert_eq!(cupo(&r, false, true, 3), 0);
        assert_eq!(cupo(&r, true, true, 1), 0);
        assert_eq!(cupo(&r, true, true, 9), 0, "sin cupo en el bloque, ninguno");
    }

    #[test]
    fn llega_uno_cuando_se_cumple_el_plazo_y_no_mas_que_el_cupo() {
        let r = r();
        let mut s = Residentes::default();
        let espera = s.ajustar(10, 1, 0).espera.unwrap();
        assert_eq!(llegada(&r, espera, r.aedas.llegada - 1, false, None), Llegada::MirarEn(r.aedas.llegada));
        assert_eq!(llegada(&r, espera, r.aedas.llegada, false, None), Llegada::Llega);
        assert_eq!(s.llega(10, 1, r.aedas.llegada), 1);
        assert_eq!((s.aedas[0].nombre(), s.siguiente), ("Femio", 2));
        // Con el cupo lleno no queda espera pendiente.
        assert_eq!(s.ajustar(10, 1, 3 * r.aedas.llegada), Ajuste { salen: vec![], espera: None });
        assert!(s.espera_desde.is_empty());
    }

    #[test]
    fn con_la_reputacion_por_los_suelos_tardan_tres_veces_mas() {
        let r = r();
        let l = r.aedas.llegada;
        assert_eq!(llegada(&r, 0, 3 * l - 1, true, None), Llegada::MirarEn(3 * l));
        assert_eq!(llegada(&r, 0, 3 * l, true, None), Llegada::Llega);
    }

    #[test]
    fn el_plazo_se_mide_con_la_reputacion_de_cuando_se_compara() {
        let r = r();
        let l = r.aedas.llegada;
        assert_eq!(llegada(&r, 0, l, true, None), Llegada::MirarEn(3 * l), "si baja durante la espera, tarda más");
        assert_eq!(llegada(&r, 0, l, true, Some(l + 5)), Llegada::MirarEn(l + 5), "si vuelve a subir antes, se mira entonces");
        assert_eq!(llegada(&r, 0, l + 5, false, None), Llegada::Llega);
        assert_eq!(llegada(&r, 0, 3 * l, true, None), Llegada::Llega);
    }

    #[test]
    fn si_la_plaza_pierde_la_nobleza_se_van_y_si_el_cupo_baja_se_va_el_ultimo_en_llegar() {
        let mut dos = Residentes::default();
        dos.llega(10, 1, 0);
        dos.llega(10, 1, 10);
        let mut sin_nobleza = dos.clone();
        assert_eq!(sin_nobleza.ajustar(10, 0, 20).salen, vec![1, 2]);
        assert!(sin_nobleza.aedas.is_empty());
        let mut baja = dos.clone();
        assert_eq!(baja.ajustar(10, 1, 20), Ajuste { salen: vec![2], espera: None });
        assert_eq!(baja.aedas.iter().map(|a| a.id).collect::<Vec<_>>(), vec![1]);
    }

    #[test]
    fn una_plaza_conquistada_cambia_de_dueno_la_epica_se_pierde_y_el_aeda_se_queda() {
        let mut s = con_residente(Some(Epica { tec: 2, capitulo: 1, hechos: 1, ..Epica::nueva(2) }));
        assert_eq!(s.cambiar_de_dueno(10, 2), vec![(1, 2)]);
        assert_eq!((s.aedas[0].faccion, s.aedas[0].plaza, s.aedas[0].epica.is_none()), (2, 10, true));
    }

    #[test]
    fn una_plaza_arrasada_se_lleva_a_sus_aedas() {
        let mut s = con_residente(None);
        s.espera_desde.insert(10, 0);
        assert_eq!(s.quitar_plaza(10), vec![1]);
        assert!(s.aedas.is_empty() && s.espera_desde.is_empty());
    }

    #[test]
    fn una_epica_cuya_tecnologia_ya_ha_aparecido_por_otra_via_se_cierra() {
        let mut s = con_residente(Some(Epica::nueva(2)));
        assert!(s.al_aparecer(1, 3).is_empty());
        assert_eq!(s.al_aparecer(1, 2), vec![1]);
        assert!(s.aedas[0].epica.is_none());
    }

    #[test]
    fn cada_residente_suma_felicidad_y_nobleza() {
        let r = r();
        assert_eq!((factor_felicidad(&r, 0), factor_felicidad(&r, 4)), (UNO, 1_200_000));
        assert_eq!(factor_nobleza(&r, 2), 1_500_000);
    }

    #[test]
    fn los_nombres_dan_la_vuelta() {
        let a = |id| AedaResidente { id, plaza: 1, faccion: 1, llegada_en: 0, epica: None };
        assert_eq!((a(1).nombre(), a(24).nombre(), a(25).nombre()), ("Femio", "Anfión", "Femio"));
    }
}
