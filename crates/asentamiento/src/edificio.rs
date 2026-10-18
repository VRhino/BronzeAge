//! Un edificio de un asentamiento y las consultas de uso común sobre la lista de edificios (Economia_Construccion §2).
//! El orden de la lista es el historial de crecimiento y **no se reordena nunca** (lo que se reordena en la cola es `prioridad`).

use crate::catalogo::Tipo;
use base::Instante;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estado {
    /// Pagado, esperando cuadrilla.
    EnCola,
    EnConstruccion,
    Activo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mejora {
    pub nivel_objetivo: u8,
    pub completa_en: Instante,
}

/// Lo que explota un extractor: un nodo finito del mapa o una parcela de bosque (Leñera).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fuente {
    Nodo(u32),
    Parcela(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edificio {
    pub id: u32,
    pub tipo: Tipo,
    pub estado: Estado,
    /// Solo en construcción: cuándo pasa a activo.
    pub completa_en: Option<Instante>,
    /// Score con que se comprometió; ordena el arranque de obra.
    pub prioridad: i64,
    /// 1 en adelante.
    pub nivel_interno: u8,
    pub mejora: Option<Mejora>,
    /// Un saqueo lo dejó en cola para reconstruir.
    pub danado: bool,
    pub fuente: Option<Fuente>,
    pub pausado_por_almacen: bool,
}

impl Edificio {
    pub fn nuevo(id: u32, tipo: Tipo, estado: Estado) -> Self {
        Self { id, tipo, estado, completa_en: None, prioridad: 0, nivel_interno: 1, mejora: None, danado: false, fuente: None, pausado_por_almacen: false }
    }

    pub fn activo(&self) -> bool {
        self.estado == Estado::Activo
    }

    /// Ocupa una cuadrilla: en obra (salvo reconstrucción tras saqueo) o mejorándose.
    pub fn ocupa_cuadrilla(&self) -> bool {
        (self.estado == Estado::EnConstruccion && !self.danado) || self.mejora.is_some()
    }
}

/// Consultas sobre la lista de edificios.
pub trait Edificios {
    fn activos(&self, t: Tipo) -> usize;
    fn en_cualquier_estado(&self, t: Tipo) -> usize;
    /// ¿Hay un edificio activo de ese tipo con al menos ese nivel interno?
    fn hay_activo_con_nivel(&self, t: Tipo, nivel: u8) -> bool;
    fn cuadrillas_ocupadas(&self) -> usize;
    fn en_cola(&self) -> usize;
}

impl Edificios for [Edificio] {
    fn activos(&self, t: Tipo) -> usize {
        self.iter().filter(|e| e.tipo == t && e.activo()).count()
    }
    fn en_cualquier_estado(&self, t: Tipo) -> usize {
        self.iter().filter(|e| e.tipo == t).count()
    }
    fn hay_activo_con_nivel(&self, t: Tipo, nivel: u8) -> bool {
        self.iter().any(|e| e.tipo == t && e.activo() && e.nivel_interno >= nivel)
    }
    fn cuadrillas_ocupadas(&self) -> usize {
        self.iter().filter(|e| e.ocupa_cuadrilla()).count()
    }
    fn en_cola(&self) -> usize {
        self.iter().filter(|e| e.estado == Estado::EnCola).count()
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn cuadrillas_cuentan_obras_y_mejoras_pero_no_reconstrucciones() {
        let mut v = vec![Edificio::nuevo(1, Tipo::Vivienda, Estado::EnConstruccion), Edificio::nuevo(2, Tipo::Granja, Estado::Activo)];
        assert_eq!(v.cuadrillas_ocupadas(), 1);
        v[1].mejora = Some(Mejora { nivel_objetivo: 2, completa_en: 10 });
        assert_eq!(v.cuadrillas_ocupadas(), 2, "una mejora ocupa cuadrilla");
        v[0].danado = true;
        assert_eq!(v.cuadrillas_ocupadas(), 1, "la reconstrucción tras saqueo no ocupa cuadrilla");
        assert_eq!((v.activos(Tipo::Granja), v.en_cualquier_estado(Tipo::Vivienda), v.activos(Tipo::Vivienda)), (1, 1, 0));
        assert!(v.hay_activo_con_nivel(Tipo::Granja, 1) && !v.hay_activo_con_nivel(Tipo::Granja, 2));
    }
}
