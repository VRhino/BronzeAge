//! El tributo del vasallaje (Diplomacia_Facciones §5.6) con la decisión **Q3** del autor (2026-10-24): se paga **en el paso de minuto de cada plaza
//! vasalla**, sin eventos nuevos, y en **enteros con resto oculto** (decisión C3b): lo pactado (`cantidad_por_minuto`, milésimas) se reparte a partes iguales
//! entre las plazas de la vasalla; cada una acumula su parte en un resto oculto y paga las **unidades enteras** que tiene, sin pasar de su stock ni del hueco
//! del almacén de la capital de la señora. Lo que no puede pagar en ese minuto se pierde (no es deuda); el resto oculto solo guarda la fracción.

/// Una unidad en el resto oculto (millonésimas de unidad).
pub const UNIDAD: i64 = 1_000_000;

/// Lo que paga una plaza vasalla en un paso de minuto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pago {
    /// Unidades enteras que salen de la vasalla y entran en la capital de la señora (hecho `diplomacia.tributo_pagado` si es > 0).
    pub unidades: i64,
    /// El resto oculto de la plaza tras el pago (`0 ≤ resto < UNIDAD`).
    pub resto: i64,
}

/// El tributo de una plaza vasalla en un minuto. `plazas` son las de la vasalla (todas pagan), `resto` el que guarda la plaza, `stock` las unidades del recurso
/// en su almacén y `hueco` las que caben en el de la capital de la señora. Sin capital (el núcleo no llama), sin plazas o con un pacto ≤ 0, no paga nada.
// DUDA: Q3 dice «hasta su capacidad»; aquí lo que no cabe en la capital **no sale** de la vasalla (se conserva el tributo: lo que paga una es lo que recibe la
// otra). El TypeScript lo descontaba entero y la capital perdía el exceso.
pub fn pago(cantidad_por_minuto: i64, plazas: u32, resto: i64, stock: i64, hueco: i64) -> Pago {
    if plazas == 0 || cantidad_por_minuto <= 0 {
        return Pago { unidades: 0, resto };
    }
    // Milésimas × 1000 = millonésimas; la división entre plazas redondea hacia abajo (nunca se crea de más).
    let debido = resto + cantidad_por_minuto * 1_000 / plazas as i64;
    Pago { unidades: (debido / UNIDAD).min(stock.max(0)).min(hueco.max(0)), resto: debido % UNIDAD }
}

#[cfg(test)]
mod pruebas {
    //! Casos de `typescript-final:src/engine/__tests__/{diplomaciaDecisiones,eventosDominioDiplomacia}.test.ts` (el tributo), ya en enteros.
    use super::*;

    #[test]
    fn pagan_todas_las_plazas_a_partes_iguales() {
        // 10 de madera por minuto entre dos plazas con 100: cada una paga 5.
        let p = pago(10_000, 2, 0, 100, 1_000);
        assert_eq!(p, Pago { unidades: 5, resto: 0 });
        // Una sola plaza: paga las 10.
        assert_eq!(pago(10_000, 1, 0, 100, 1_000).unidades, 10);
    }

    #[test]
    fn cada_plaza_paga_hasta_su_parte_con_lo_que_tenga_y_la_que_no_tiene_no_paga() {
        assert_eq!(pago(10_000, 2, 0, 2, 1_000).unidades, 2);
        assert_eq!(pago(10_000, 2, 0, 0, 1_000).unidades, 0);
    }

    #[test]
    fn la_fraccion_se_acumula_en_el_resto_oculto() {
        // 1 unidad por minuto entre 3 plazas: cada una debe 0,333… y paga 1 cada tres minutos.
        let mut resto = 0;
        let mut pagado = vec![];
        for _ in 0..6 {
            let p = pago(1_000, 3, resto, 100, 100);
            pagado.push(p.unidades);
            resto = p.resto;
        }
        assert_eq!(pagado, vec![0, 0, 0, 1, 0, 0], "333.333 × 3 = 999.999: la división hacia abajo se nota al tercer minuto");
        assert!(resto < UNIDAD);
        // 2,5 por minuto en una plaza: 2, 3, 2, 3…
        let (a, b) = (pago(2_500, 1, 0, 100, 100), pago(2_500, 1, 500_000, 100, 100));
        assert_eq!((a.unidades, a.resto, b.unidades, b.resto), (2, 500_000, 3, 0));
    }

    #[test]
    fn lo_que_no_cabe_en_la_capital_no_sale_de_la_vasalla() {
        assert_eq!(pago(10_000, 1, 0, 100, 4).unidades, 4);
        assert_eq!(pago(10_000, 1, 0, 100, 0).unidades, 0);
    }

    #[test]
    fn sin_plazas_o_sin_pacto_no_hay_pago() {
        assert_eq!(pago(10_000, 0, 7, 100, 100), Pago { unidades: 0, resto: 7 });
        assert_eq!(pago(0, 2, 7, 100, 100), Pago { unidades: 0, resto: 7 });
    }
}
