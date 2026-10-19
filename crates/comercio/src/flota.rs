//! La flota de un asentamiento (Comercio_Caravanas §5.1, §5.4): cupos que da el Mercado activo y factores de las políticas del origen.
//!
//! Los cupos salen del **Mercado activo** y valen 0 sin él: `cupoCaravanas` y `cupoCarros` del nivel interno del Mercado (más
//! `cupoCaravanaExtra` de "Ampliación de Flota", que se suma), y `cupoEscolta` en puntos de Liderazgo **por caravana**. Las políticas
//! "Carga Ampliada" (capacidad ×1,5), "Rutas Rápidas" (velocidad ×1,5) y las de comisión externa entran como factores ya multiplicados;
//! el **carro de acarreo no recibe** ni "Carga Ampliada" ni "Rutas Rápidas" (decidido el 2026-10-18: se pasan `UNO`).

use crate::datos::ReglasFlota;
use asentamiento::datos::{ReglasPoliticas, UNO};
use asentamiento::{Catalogo, Politicas, Tipo};

/// Los factores de comercio de las políticas activas de un asentamiento (ppm; `UNO` si ninguna los trae).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FactoresPolitica {
    /// `factorComisionExterna`: producto de "Comercio Abierto" ×0,6, "Aranceles Proteccionistas" ×1,5 y "Arterias Comerciales" ×0,8.
    pub comision_externa: i64,
    /// `factorCapacidadCaravana` ("Carga Ampliada" ×1,5): se aplica al **cargar** (reparto, `prepararCaravana`, adjunta, aparcada).
    pub capacidad: i64,
    /// `factorVelocidadCaravana` ("Rutas Rápidas" ×1,5): se aplica al **mover** una comercial, ida y vuelta.
    pub velocidad: i64,
    /// `cupoCaravanaExtra` ("Ampliación de Flota" +1): se suma al cupo de caravanas.
    pub cupo_caravana_extra: i64,
}

impl FactoresPolitica {
    /// Sin ninguna política (también el caso del origen que ya no existe: «sin política»).
    pub const NINGUNO: FactoresPolitica = FactoresPolitica { comision_externa: UNO, capacidad: UNO, velocidad: UNO, cupo_caravana_extra: 0 };

    pub fn de(p: &Politicas, r: &ReglasPoliticas) -> Self {
        Self {
            comision_externa: p.factor(r, "factorComisionExterna"),
            capacidad: p.factor(r, "factorCapacidadCaravana"),
            velocidad: p.factor(r, "factorVelocidadCaravana"),
            cupo_caravana_extra: p.suma(r, "cupoCaravanaExtra"),
        }
    }
}

/// Los tres cupos de la flota de un asentamiento.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cupos {
    /// Caravanas `comercial` con este origen, en cualquier estado.
    pub caravanas: u32,
    /// Carros de la flota entera, repartidos como se quiera entre sus caravanas.
    pub carros: u32,
    /// Puntos de Liderazgo de escolta, **por caravana**.
    pub escolta: i64,
}

impl Cupos {
    pub const NINGUNO: Cupos = Cupos { caravanas: 0, carros: 0, escolta: 0 };
}

/// Los cupos de un asentamiento: `nivel_mercado` es el nivel interno del Mercado **activo** (`None` = sin Mercado ⇒ todo 0).
pub fn cupos(catalogo: &Catalogo, flota: &ReglasFlota, nivel_mercado: Option<u8>, f: &FactoresPolitica) -> Cupos {
    let Some(n) = nivel_mercado else { return Cupos::NINGUNO };
    let Some(nivel) = catalogo.tipo(Tipo::Mercado).nivel(n) else { return Cupos::NINGUNO };
    Cupos {
        caravanas: (nivel.cupo_caravanas as i64 + f.cupo_caravana_extra).max(0) as u32,
        carros: nivel.cupo_carros,
        escolta: escolta_de_nivel(flota, n),
    }
}

/// `CARAVANA_ESCOLTA.liderazgoPorNivelMercado[nivel − 1]` (el último valor si el nivel se pasa de la tabla).
pub fn escolta_de_nivel(flota: &ReglasFlota, nivel: u8) -> i64 {
    let t = &flota.liderazgo_por_nivel_mercado;
    t.get((nivel as usize).saturating_sub(1)).or_else(|| t.last()).copied().unwrap_or(0)
}

/// Cupo de escolta del carro de acarreo (Doc 3.13.8): el del Mercado de la plaza madre si lo tiene (100/200/300) y **100** si no.
pub fn cupo_escolta_de_acarreo(flota: &ReglasFlota, nivel_mercado_madre: Option<u8>) -> i64 {
    escolta_de_nivel(flota, nivel_mercado_madre.unwrap_or(1))
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::Reglas;
    use asentamiento::politicas::{Activador, Cargo};

    fn activador(cargo: Cargo) -> Activador {
        Activador { cargo, cargo_ocupado: true, nivel_faccion: 1, sala_consejo: false }
    }

    #[test]
    fn los_cupos_del_mercado_por_nivel_interno() {
        let a = asentamiento::Reglas::cargar();
        let f = Reglas::cargar().flota;
        let c = |n| cupos(&a.catalogo, &f, n, &FactoresPolitica::NINGUNO);
        assert_eq!(c(Some(1)), Cupos { caravanas: 2, carros: 3, escolta: 100 });
        assert_eq!(c(Some(2)), Cupos { caravanas: 4, carros: 9, escolta: 200 });
        assert_eq!(c(Some(3)), Cupos { caravanas: 6, carros: 18, escolta: 300 });
        assert_eq!(c(None), Cupos::NINGUNO, "sin Mercado activo todo vale 0");
        assert_eq!(c(Some(9)), Cupos::NINGUNO, "un nivel inexistente no da nada");
    }

    #[test]
    fn ampliacion_de_flota_suma_un_cupo_pero_solo_con_mercado() {
        let a = asentamiento::Reglas::cargar();
        let f = Reglas::cargar().flota;
        let mut p = Politicas::default();
        p.activar(&a.politicas, "cupo_caravana_extra", &activador(Cargo::Tesorero), 0).unwrap();
        let fac = FactoresPolitica::de(&p, &a.politicas);
        assert_eq!(fac.cupo_caravana_extra, 1);
        assert_eq!(cupos(&a.catalogo, &f, Some(1), &fac).caravanas, 3);
        assert_eq!(cupos(&a.catalogo, &f, None, &fac), Cupos::NINGUNO);
    }

    #[test]
    fn los_factores_de_las_politicas_se_multiplican() {
        let a = asentamiento::Reglas::cargar();
        assert_eq!(FactoresPolitica::de(&Politicas::default(), &a.politicas), FactoresPolitica::NINGUNO);
        let mut p = Politicas::default();
        for (id, cargo) in [("carga_ampliada", Cargo::Tesorero), ("rutas_rapidas", Cargo::Gobernador), ("comercio_abierto", Cargo::Tesorero), ("arterias_comerciales", Cargo::Gobernador)] {
            p.activar(&a.politicas, id, &activador(cargo), 0).unwrap();
        }
        let f = FactoresPolitica::de(&p, &a.politicas);
        assert_eq!((f.capacidad, f.velocidad), (1_500_000, 1_500_000));
        assert_eq!(f.comision_externa, 480_000, "×0,6 y ×0,8 se multiplican");
    }

    #[test]
    fn la_escolta_del_acarreo_es_la_del_mercado_de_la_madre_o_100() {
        let f = Reglas::cargar().flota;
        assert_eq!(cupo_escolta_de_acarreo(&f, None), 100);
        assert_eq!(cupo_escolta_de_acarreo(&f, Some(3)), 300);
        assert_eq!(escolta_de_nivel(&f, 7), 300, "más allá de la tabla, el último");
    }
}
