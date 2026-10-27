//! Precio de referencia, tasa y comisión (Comercio_Caravanas §3). Todo en enteros: precios en ppm de oro por unidad, cantidades en
//! milésimas, el oro vale 0 al calcular la comisión (no cotiza contra sí mismo).

use crate::datos::{ReglasComision, ReglasPrecio, ReglasReputacion};
use crate::tipos::Carga;
use asentamiento::datos::{por, razon, UNO};
use asentamiento::{Almacen, Recurso};
use base::Mil;
use std::collections::BTreeMap;

/// Una unidad entera de distancia (mínimo del `max(1, distancia)` de la comisión).
const UNA_UNIDAD: Mil = 1_000;

/// `Σ` del stock disponible de `r` en todos los almacenes del mundo (§3.1: global, de todas las Facciones, también de bots, y sin
/// distancia). El núcleo entrega el stock **tal como quedó en el último paso de cada plaza**.
pub fn stock_global<'a>(r: Recurso, almacenes: impl IntoIterator<Item = &'a Almacen>) -> Mil {
    almacenes.into_iter().map(|a| a.cantidad(r)).sum()
}

/// Precio de referencia de `r` (ppm de oro por unidad) dado el stock global: `base × clamp(objetivo / max(1 unidad, stock), min, max)`.
/// El oro no tiene precio base y vale 0; un manufacturado parte del derivado de su receta (`ReglasPrecio::derivados`).
pub fn precio_referencia(reglas: &ReglasPrecio, r: Recurso, stock_global: Mil) -> i64 {
    let Some(base) = reglas.base_de(r) else { return 0 };
    let factor = razon(reglas.stock_objetivo_global, stock_global.max(UNA_UNIDAD)).clamp(reglas.factor_min, reglas.factor_max);
    por(base, factor)
}

/// Los precios de referencia de todos los recursos con cotización, calculados de una vez con los stocks dados.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TablaPrecios {
    precios: BTreeMap<Recurso, i64>,
}

impl TablaPrecios {
    /// Calcula el precio de cada recurso con precio base (`PRECIO_BASE` o derivado de su receta) sobre el stock global de `almacenes`.
    pub fn de<'a>(reglas: &ReglasPrecio, almacenes: impl IntoIterator<Item = &'a Almacen> + Clone) -> Self {
        let precios = reglas.cotizados().map(|r| (r, precio_referencia(reglas, r, stock_global(r, almacenes.clone())))).collect();
        Self { precios }
    }

    /// La tabla con estos precios (ppm por recurso), tal como se guardó.
    pub fn desde(precios: BTreeMap<Recurso, i64>) -> Self {
        Self { precios }
    }

    /// Cada recurso con cotización y su precio (ppm), en orden de recurso.
    pub fn iter(&self) -> impl Iterator<Item = (Recurso, i64)> + '_ {
        self.precios.iter().map(|(&r, &p)| (r, p))
    }

    /// Precio (ppm) de `r`; 0 si no cotiza (el oro).
    pub fn precio(&self, r: Recurso) -> i64 {
        self.precios.get(&r).copied().unwrap_or(0)
    }

    /// Valor total de una carga en milésimas de oro.
    pub fn valor(&self, carga: &Carga) -> Mil {
        carga.iter().map(|(&r, &n)| por(n, self.precio(r))).sum()
    }
}

/// Tasa de comisión (§3.2): la de la misma Facción, o la externa × el factor de las políticas del **destino**.
pub fn tasa(r: &ReglasComision, misma_faccion: bool, factor_comision_externa_destino: i64) -> i64 {
    if misma_faccion {
        r.tasa_misma_faccion
    } else {
        por(r.tasa_externa, factor_comision_externa_destino)
    }
}

/// Bonus por distancia (§3.3), en ppm (≥ 1,0): lineal hasta `bonus_max` a `distancia_para_bonus_max`, sobre la distancia en línea recta
/// (mínimo una unidad).
pub fn bonus_por_distancia(r: &ReglasComision, distancia_recta: Mil) -> i64 {
    let frac = razon(distancia_recta.max(UNA_UNIDAD), r.distancia_para_bonus_max).min(UNO);
    UNO + por(frac, r.bonus_por_distancia_max - UNO)
}

/// Lo que hace falta saber de una entrega para cobrar su comisión.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextoComision {
    /// Origen y destino son el mismo asentamiento: llevarse la carga a uno mismo no es comerciar.
    pub mismo_asentamiento: bool,
    pub misma_faccion: bool,
    /// Producto de `factorComisionExterna` en las políticas activas del destino (ppm; `UNO` si no hay).
    pub factor_comision_externa: i64,
    /// Distancia en línea recta origen → destino (no la longitud de la ruta).
    pub distancia_recta: Mil,
}

/// Comisión de una entrega de trueque o de caravana (§3.3): `valor × tasa × bonus`, 0 si origen y destino son el mismo asentamiento.
/// Se **acuña** (se suma al destino, topada por su capacidad de oro; no se resta de nadie).
pub fn comision_de_entrega(r: &ReglasComision, valor_total: Mil, c: &ContextoComision) -> Mil {
    if c.mismo_asentamiento || valor_total <= 0 {
        return 0;
    }
    // Un solo redondeo para toda la cadena de factores.
    let p = valor_total as i128 * tasa(r, c.misma_faccion, c.factor_comision_externa) as i128 * bonus_por_distancia(r, c.distancia_recta) as i128;
    let d = UNO as i128 * UNO as i128;
    ((p + d / 2) / d) as Mil
}

/// Recargo por mala reputación (§3.3): si las Facciones son distintas y la reputación del origen es `≤ umbralBajoParaComision`, esa
/// Facción paga de su bolsillo `comisión × (factor − 1)`, sin pasarse del oro que tenga el almacén de origen. `reputacion` en ppm de punto.
pub fn recargo_por_reputacion(rep: &ReglasReputacion, comision: Mil, misma_faccion: bool, reputacion_origen: i64, oro_origen: Mil) -> Mil {
    if misma_faccion || reputacion_origen > rep.umbral_bajo_para_comision {
        return 0;
    }
    por(comision, rep.factor_comision_por_reputacion_baja - UNO).min(oro_origen.max(0))
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::{Reglas, PUNTO};

    fn alm(r: Recurso, n: Mil) -> Almacen {
        let mut a = Almacen::nuevo(10_000_000);
        a.poner(r, n);
        a
    }

    #[test]
    fn precio_de_referencia_escasez_y_abundancia() {
        let r = Reglas::cargar().precio;
        // Stock en el objetivo (500): factor 1 → precio base.
        assert_eq!(precio_referencia(&r, Recurso::Cobre, 500_000), 3_000_000);
        // Escasez: 100 de stock → ×5, topado en ×3.
        assert_eq!(precio_referencia(&r, Recurso::Cobre, 100_000), 9_000_000);
        // Abundancia: 5000 → ×0,1, topado en ×0,4.
        assert_eq!(precio_referencia(&r, Recurso::Cobre, 5_000_000), 1_200_000);
        // Sin stock: max(1, 0) = 1 unidad → topado en ×3.
        assert_eq!(precio_referencia(&r, Recurso::Madera, 0), 3_000_000);
        // 1000 de stock → ×0,5.
        assert_eq!(precio_referencia(&r, Recurso::Hierro, 1_000_000), 2_000_000);
        assert_eq!(precio_referencia(&r, Recurso::Oro, 123), 0, "el oro no cotiza");
        assert_eq!(precio_referencia(&r, Recurso::ChatarraCobre, 123), 0, "lo que no tiene precio base ni receta que lo fabrique tampoco");
    }

    #[test]
    fn el_precio_de_un_manufacturado_sale_de_su_receta() {
        let r = Reglas::cargar().precio;
        let en_base = |rec| precio_referencia(&r, rec, 500_000);
        // Receta más barata de cada bien, a precio base (madera 1, ganado 2, cobre 3, estaño 6, hierro 4), en ppm de oro por unidad.
        assert_eq!(en_base(Recurso::ArmaMadera), 2_000_000, "2 de madera");
        assert_eq!(en_base(Recurso::LingoteCobre), 6_000_000, "2 de cobre");
        assert_eq!(en_base(Recurso::LingoteEstano), 30_000_000, "5 de estaño");
        assert_eq!(en_base(Recurso::LingoteHierro), 9_000_000, "2 de hierro y 1 de madera");
        assert_eq!(en_base(Recurso::LingoteBronce), 21_600_000, "1,6 de lingote de cobre y 0,4 de estaño: se resuelve en cadena");
        assert_eq!(en_base(Recurso::ArmaCobre), 7_000_000, "un lingote de cobre y 1 de madera");
        assert_eq!(en_base(Recurso::Cuero), 500_000, "la receta de la Curtiduría nivel 3 (0,25 de ganado) es la más barata");
        assert_eq!(en_base(Recurso::ArmaduraBasica), 2_500_000, "5 de cuero");
        assert_eq!(en_base(Recurso::ArmaduraBronce), 21_600_000 + 5 * 3_500_000, "un lingote de bronce y 5 de cuero de calidad");
        // Las materias primas conservan su precio de `PRECIO_BASE`, y lo que no se fabrica ni cotiza (oro, chatarra) sigue en 0.
        assert_eq!(en_base(Recurso::Cobre), 3_000_000);
        assert_eq!((en_base(Recurso::Oro), en_base(Recurso::ChatarraBronce)), (0, 0));
    }

    #[test]
    fn la_tabla_de_precios_cotiza_los_manufacturados_con_su_propio_stock() {
        let r = Reglas::cargar().precio;
        let a = [alm(Recurso::ArmaMadera, 1_000_000)];
        let t = TablaPrecios::de(&r, &a);
        assert_eq!(t.precio(Recurso::ArmaMadera), 1_000_000, "2 de base × 0,5 (1000 en stock contra 500 de objetivo)");
        assert_eq!(t.precio(Recurso::ArmaCobre), 21_000_000, "sin stock: ×3");
        let mut c = Carga::new();
        c.insert(Recurso::ArmaMadera, 10_000);
        assert_eq!(t.valor(&c), 10_000, "10 armas a 1 de oro");
    }

    #[test]
    fn el_stock_es_global_y_suma_todas_las_plazas() {
        let r = Reglas::cargar().precio;
        let a = [alm(Recurso::Madera, 200_000), alm(Recurso::Madera, 300_000), alm(Recurso::Piedra, 900_000)];
        assert_eq!(stock_global(Recurso::Madera, &a), 500_000);
        let t = TablaPrecios::de(&r, &a);
        assert_eq!(t.precio(Recurso::Madera), 1_000_000, "500 = objetivo");
        assert_eq!(t.precio(Recurso::Oro), 0);
        let mut c = Carga::new();
        c.insert(Recurso::Madera, 10_000);
        c.insert(Recurso::Oro, 99_000);
        assert_eq!(t.valor(&c), 10_000, "10 de madera a 1 de oro; el oro vale 0");
    }

    #[test]
    fn tasa_misma_faccion_y_externa_con_politicas() {
        let c = Reglas::cargar().comision;
        assert_eq!(tasa(&c, true, UNO), 30_000);
        assert_eq!(tasa(&c, false, UNO), 80_000);
        // Comercio Abierto ×0,6; Aranceles ×1,5; Arterias ×0,8; juntas se multiplican (el factor llega ya multiplicado).
        assert_eq!(tasa(&c, false, 600_000), 48_000);
        assert_eq!(tasa(&c, false, 1_500_000), 120_000);
        assert_eq!(tasa(&c, false, por(600_000, 800_000)), 38_400);
        assert_eq!(tasa(&c, true, 600_000), 30_000, "las políticas solo tocan la externa");
    }

    #[test]
    fn bonus_lineal_hasta_1_5_a_120_unidades() {
        let c = Reglas::cargar().comision;
        assert_eq!(bonus_por_distancia(&c, 0), UNO + 4_167, "la distancia mínima es 1 unidad: 1/120 del medio punto");
        assert_eq!(bonus_por_distancia(&c, 60_000), 1_250_000);
        assert_eq!(bonus_por_distancia(&c, 120_000), 1_500_000);
        assert_eq!(bonus_por_distancia(&c, 5_000_000), 1_500_000, "topado en ×1,5");
    }

    #[test]
    fn comision_de_los_ejemplos_del_documento() {
        let c = Reglas::cargar().comision;
        let ctx = |misma, dist| ContextoComision { mismo_asentamiento: false, misma_faccion: misma, factor_comision_externa: UNO, distancia_recta: dist };
        // 1000 de oro de valor, misma Facción, distancia máxima: 3 % × 1,5 = 4,5 % → 45.
        assert_eq!(comision_de_entrega(&c, 1_000_000, &ctx(true, 120_000)), 45_000);
        // Externa: 8 % × 1,5 = 12 % → 120.
        assert_eq!(comision_de_entrega(&c, 1_000_000, &ctx(false, 999_000)), 120_000);
        // Mitad de la distancia: bonus ×1,25 → 3 % × 1,25 = 3,75 %.
        assert_eq!(comision_de_entrega(&c, 1_000_000, &ctx(true, 60_000)), 37_500);
    }

    #[test]
    fn llevarse_la_carga_a_uno_mismo_o_el_oro_no_acuna() {
        let c = Reglas::cargar().comision;
        let mismo = ContextoComision { mismo_asentamiento: true, misma_faccion: true, factor_comision_externa: UNO, distancia_recta: 0 };
        assert_eq!(comision_de_entrega(&c, 1_000_000, &mismo), 0);
        let otro = ContextoComision { mismo_asentamiento: false, ..mismo };
        assert_eq!(comision_de_entrega(&c, 0, &otro), 0, "un trueque de oro por oro vale 0 y no acuña nada");
    }

    #[test]
    fn recargo_por_reputacion_baja_paga_el_extra_topado_por_el_oro() {
        let r = Reglas::cargar().reputacion;
        // Comisión 120 → recargo del 50 % = 60.
        assert_eq!(recargo_por_reputacion(&r, 120_000, false, -40 * PUNTO, 1_000_000), 60_000);
        assert_eq!(recargo_por_reputacion(&r, 120_000, false, -90 * PUNTO, 1_000_000), 60_000);
        assert_eq!(recargo_por_reputacion(&r, 120_000, false, -39 * PUNTO, 1_000_000), 0, "por encima del umbral");
        assert_eq!(recargo_por_reputacion(&r, 120_000, true, -90 * PUNTO, 1_000_000), 0, "misma Facción: nada");
        assert_eq!(recargo_por_reputacion(&r, 120_000, false, -90 * PUNTO, 25_000), 25_000, "paga lo que haya");
        assert_eq!(recargo_por_reputacion(&r, 120_000, false, -90 * PUNTO, 0), 0, "nunca negativo");
    }
}
