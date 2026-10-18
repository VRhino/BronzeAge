//! Políticas activas de un asentamiento (Doc 4.4, Economia_Construccion §12): slots por cargo, duración, caducidad y los
//! factores que se multiplican entre las activas. Sin reloj: quien lo usa pasa el instante y programa el evento de caducidad.

use crate::datos::{ReglasPoliticas, UNO};
use base::Instante;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cargo {
    Gobernador,
    Tesorero,
    General,
    MaestroObras,
    Sacerdote,
}

impl Cargo {
    /// Clave en los datos (`slotsPorCargo`, `cargo` de cada política).
    pub fn clave(self) -> &'static str {
        match self {
            Cargo::Gobernador => "gobernador",
            Cargo::Tesorero => "tesorero",
            Cargo::General => "general",
            Cargo::MaestroObras => "maestroObras",
            Cargo::Sacerdote => "sacerdote",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Activa {
    pub id: String,
    pub cargo: Cargo,
    pub expira_en: Instante,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorPolitica {
    Desconocida,
    /// El cargo no es el Gobernador y la política no es de su pool.
    NoEsDeSuPool,
    CargoVacio,
    YaActiva,
    SinSlot,
}

/// Lo que se sabe de quien activa: el cargo, si está ocupado, el nivel de la Facción y si hay Sala del Consejo.
#[derive(Clone, Copy, Debug)]
pub struct Activador {
    pub cargo: Cargo,
    pub cargo_ocupado: bool,
    pub nivel_faccion: u32,
    pub sala_consejo: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Politicas {
    activas: Vec<Activa>,
}

/// Slots de un cargo: el Gobernador `min(máximo, base + nivel_faccion / n) + (Sala del Consejo ? 1 : 0)`; el resto, `base`.
pub fn slots_disponibles(r: &ReglasPoliticas, a: &Activador) -> u32 {
    let s = &r.slots[a.cargo.clave()];
    if a.cargo != Cargo::Gobernador {
        return s.base;
    }
    s.maximo.min(s.base + a.nivel_faccion / r.nivel_faccion_por_slot_extra_gobernador.max(1)) + if a.sala_consejo { r.slot_sala_consejo } else { 0 }
}

impl Politicas {
    pub fn activas(&self) -> &[Activa] {
        &self.activas
    }

    /// Activa la política `id` con la duración por defecto. No se cancela; solo caduca. Devuelve su instante de caducidad.
    pub fn activar(&mut self, r: &ReglasPoliticas, id: &str, a: &Activador, ahora: Instante) -> Result<Instante, ErrorPolitica> {
        let def = r.catalogo.iter().find(|p| p.id == id).ok_or(ErrorPolitica::Desconocida)?;
        if a.cargo != Cargo::Gobernador && def.cargo != a.cargo.clave() {
            return Err(ErrorPolitica::NoEsDeSuPool);
        }
        if !a.cargo_ocupado {
            return Err(ErrorPolitica::CargoVacio);
        }
        if self.activas.iter().any(|x| x.id == id && x.cargo == a.cargo) {
            return Err(ErrorPolitica::YaActiva);
        }
        let usadas = self.activas.iter().filter(|x| x.cargo == a.cargo).count() as u32;
        if usadas >= slots_disponibles(r, a) {
            return Err(ErrorPolitica::SinSlot);
        }
        let expira_en = ahora + r.duracion_minutos * base::MS_POR_MINUTO;
        self.activas.push(Activa { id: id.to_string(), cargo: a.cargo, expira_en });
        Ok(expira_en)
    }

    /// Borra las que vencen en `ahora` (`ahora >= expira_en`) y las devuelve.
    pub fn caducar(&mut self, ahora: Instante) -> Vec<Activa> {
        let (vencidas, vivas) = std::mem::take(&mut self.activas).into_iter().partition(|x| ahora >= x.expira_en);
        self.activas = vivas;
        vencidas
    }

    /// El primer instante en que caduca alguna (para programar el evento).
    pub fn proxima_caducidad(&self) -> Option<Instante> {
        self.activas.iter().map(|x| x.expira_en).min()
    }

    /// Producto en ppm del factor `nombre` (`factorConsumoComida`…) entre las activas; `UNO` si ninguna lo trae.
    pub fn factor(&self, r: &ReglasPoliticas, nombre: &str) -> i64 {
        self.activas
            .iter()
            .filter_map(|x| r.catalogo.iter().find(|p| p.id == x.id)?.factores.get(nombre).copied())
            .fold(UNO, crate::datos::por)
    }

    /// Suma de un aditivo (`cupoCaravanaExtra`, `cupoGuarnicionExtra`) entre las activas.
    pub fn suma(&self, r: &ReglasPoliticas, nombre: &str) -> i64 {
        self.activas.iter().filter_map(|x| r.catalogo.iter().find(|p| p.id == x.id)?.sumas.get(nombre).copied()).sum()
    }

    /// El perfil de trazado que fija la primera ordenanza activa, si hay (Doc 4.4: las ordenanzas de trazado).
    pub fn perfil_trazado(&self, r: &ReglasPoliticas) -> Option<String> {
        self.activas.iter().find_map(|x| r.catalogo.iter().find(|p| p.id == x.id)?.perfil_trazado.clone())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::Reglas;
    use base::MS_POR_MINUTO;

    fn ctx(cargo: Cargo) -> Activador {
        Activador { cargo, cargo_ocupado: true, nivel_faccion: 1, sala_consejo: false }
    }

    #[test]
    fn slots_del_gobernador_crecen_con_el_nivel_de_faccion_y_la_sala() {
        let r = Reglas::cargar().politicas;
        let g = |nivel, sala| slots_disponibles(&r, &Activador { nivel_faccion: nivel, sala_consejo: sala, ..ctx(Cargo::Gobernador) });
        assert_eq!((g(1, false), g(3, false), g(6, false), g(30, false)), (2, 3, 4, 5));
        assert_eq!(g(30, true), 6, "la Sala del Consejo suma uno aunque ya esté en el máximo");
        assert_eq!(slots_disponibles(&r, &ctx(Cargo::Sacerdote)), 1);
        assert_eq!(slots_disponibles(&r, &ctx(Cargo::Tesorero)), 2);
    }

    #[test]
    fn activar_valida_pool_cargo_repeticion_y_slots() {
        let r = Reglas::cargar().politicas;
        let mut p = Politicas::default();
        assert_eq!(p.activar(&r, "nada", &ctx(Cargo::Gobernador), 0), Err(ErrorPolitica::Desconocida));
        assert_eq!(p.activar(&r, "via_rapida", &ctx(Cargo::Sacerdote), 0), Err(ErrorPolitica::NoEsDeSuPool));
        assert_eq!(p.activar(&r, "racionamiento", &Activador { cargo_ocupado: false, ..ctx(Cargo::Sacerdote) }, 0), Err(ErrorPolitica::CargoVacio));
        assert_eq!(p.activar(&r, "racionamiento", &ctx(Cargo::Sacerdote), 1_000), Ok(1_000 + 150 * MS_POR_MINUTO));
        assert_eq!(p.activar(&r, "racionamiento", &ctx(Cargo::Sacerdote), 2_000), Err(ErrorPolitica::YaActiva));
        assert_eq!(p.activar(&r, "culto_fertilidad", &ctx(Cargo::Sacerdote), 2_000), Err(ErrorPolitica::SinSlot), "el Sacerdote solo tiene un slot");
        // El Gobernador, con pool completa, puede activar la de otro cargo con sus propios slots.
        assert!(p.activar(&r, "via_rapida", &ctx(Cargo::Gobernador), 0).is_ok());
        assert!(p.activar(&r, "edicto_cosecha", &ctx(Cargo::Gobernador), 0).is_ok());
        assert_eq!(p.activar(&r, "presion_fiscal", &ctx(Cargo::Gobernador), 0), Err(ErrorPolitica::SinSlot));
    }

    #[test]
    fn los_factores_se_multiplican_y_los_cupos_se_suman() {
        let r = Reglas::cargar().politicas;
        let mut p = Politicas::default();
        assert_eq!(p.factor(&r, "factorProduccionTrigo"), UNO);
        p.activar(&r, "edicto_cosecha", &ctx(Cargo::Gobernador), 0).unwrap();
        p.activar(&r, "presion_fiscal", &ctx(Cargo::Tesorero), 0).unwrap();
        assert_eq!(p.factor(&r, "factorProduccionTrigo"), 1_500_000);
        assert_eq!(p.factor(&r, "factorRecaudacion"), 1_600_000);
        assert_eq!(p.factor(&r, "factorCrecimientoPoblacion"), 800_000);
        // Dos políticas con el mismo factor se multiplican: Presión Fiscal (×0,8) y Plazas Mayores (×1,25) se compensan.
        p.activar(&r, "plazas_mayores", &ctx(Cargo::MaestroObras), 0).unwrap();
        assert_eq!(p.factor(&r, "factorCrecimientoPoblacion"), 1_000_000);
        assert_eq!(p.perfil_trazado(&r).as_deref(), Some("nucleos"));
        p.activar(&r, "cupo_caravana_extra", &ctx(Cargo::Tesorero), 0).unwrap();
        assert_eq!(p.suma(&r, "cupoCaravanaExtra"), 1);
    }

    #[test]
    fn caducan_exactamente_al_vencer() {
        let r = Reglas::cargar().politicas;
        let mut p = Politicas::default();
        let fin = p.activar(&r, "racionamiento", &ctx(Cargo::Sacerdote), 0).unwrap();
        assert_eq!(p.proxima_caducidad(), Some(fin));
        assert!(p.caducar(fin - 1).is_empty());
        let v = p.caducar(fin);
        assert_eq!(v.len(), 1);
        assert_eq!(p.proxima_caducidad(), None);
        assert_eq!(p.factor(&r, "factorConsumoComida"), UNO);
    }
}
