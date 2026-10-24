//! La entrega de una caravana (Comercio_Caravanas §4.4, §3.3): lo que llega al almacén del destino, la comisión que se acuña y el
//! recargo por mala reputación que sale del almacén de origen. El abono al trueque lo hace [`crate::trueque::AcuerdoTrueque::abonar`]
//! con el `contenido` que la caravana llevaba al llegar (tras los peajes), no con lo que entró.

use crate::datos::Reglas;
use crate::precio::{comision_de_entrega, recargo_por_reputacion, ContextoComision, TablaPrecios};
use crate::tipos::Carga;
use asentamiento::{Almacen, Recurso};
use base::Mil;

/// Lo que hace falta saber de la entrega.
pub struct DatosEntrega<'a> {
    pub comision: ContextoComision,
    /// Reputación de la Facción del **origen** (ppm de punto) en el instante de llegar.
    pub reputacion_origen: i64,
    /// Oro del almacén de origen (el recargo no puede pasar de él; si el origen no existe, 0).
    pub oro_origen: Mil,
    pub precios: &'a TablaPrecios,
}

/// Resultado de entregar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultadoEntrega {
    /// Lo que entró de verdad en el almacén del destino.
    pub ingresado: Carga,
    /// Lo que no cupo y se pierde (`agregarRecurso` topa por capacidad).
    pub perdido: Carga,
    /// Valor de lo que llevaba, en milésimas de oro (el oro vale 0).
    pub valor: Mil,
    /// Comisión **acuñada** en el destino (topada por su capacidad de oro).
    pub comision: Mil,
    /// Recargo por reputación: sale del oro del almacén de origen y se suma al del destino (se conserva). El núcleo lo resta del origen.
    pub recargo: Mil,
}

/// Entrega `contenido` en el almacén del destino: cada recurso con `agregar` (lo que no cabe se pierde), luego la comisión (se acuña,
/// topada) y, si procede, el recargo por reputación (conservado: no se acuña), topado también por el hueco de oro que deja la comisión: el origen
/// solo paga lo que entró (Comercio_Caravanas §3.3).
pub fn entregar(r: &Reglas, contenido: &Carga, destino: &mut Almacen, d: &DatosEntrega) -> ResultadoEntrega {
    let mut ingresado = Carga::new();
    let mut perdido = Carga::new();
    for (&rec, &n) in contenido.iter().filter(|(_, &n)| n > 0) {
        let entra = destino.agregar(rec, n);
        if entra > 0 {
            ingresado.insert(rec, entra);
        }
        if entra < n {
            perdido.insert(rec, n - entra);
        }
    }
    let valor = d.precios.valor(contenido);
    let calculada = comision_de_entrega(&r.comision, valor, &d.comision);
    let comision = destino.agregar(Recurso::Oro, calculada);
    let recargo_debido = recargo_por_reputacion(&r.reputacion, calculada, d.comision.misma_faccion, d.reputacion_origen, d.oro_origen);
    let recargo = destino.agregar(Recurso::Oro, recargo_debido);
    ResultadoEntrega { ingresado, perdido, valor, comision, recargo }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::PUNTO;
    use asentamiento::datos::UNO;

    fn alm(items: &[(Recurso, Mil)]) -> Almacen {
        let mut a = Almacen::nuevo(1_000_000);
        for &(r, n) in items {
            a.poner(r, n);
        }
        a
    }

    fn carga(items: &[(Recurso, Mil)]) -> Carga {
        items.iter().copied().collect()
    }

    fn precios_a_uno(r: &Reglas) -> TablaPrecios {
        // Stock global = objetivo → todos los precios en su base.
        let mut a = Almacen::nuevo(10_000_000);
        for rec in r.precio.cotizados() {
            a.poner(rec, 500_000);
        }
        TablaPrecios::de(&r.precio, [&a])
    }

    fn datos<'a>(p: &'a TablaPrecios, misma: bool, mismo_asent: bool, rep: i64, oro_origen: Mil) -> DatosEntrega<'a> {
        DatosEntrega {
            comision: ContextoComision { mismo_asentamiento: mismo_asent, misma_faccion: misma, factor_comision_externa: UNO, distancia_recta: 120_000 },
            reputacion_origen: rep,
            oro_origen,
            precios: p,
        }
    }

    #[test]
    fn entrega_todo_y_acuna_la_comision() {
        let r = Reglas::cargar();
        let p = precios_a_uno(&r);
        let mut dest = alm(&[]);
        // 100 de madera (valor 100) + 50 de cobre (valor 150) = 250 de oro; misma Facción, distancia máxima: 4,5 % = 11,25, que acuñan 11 (hacia abajo).
        let e = entregar(&r, &carga(&[(Recurso::Madera, 100_000), (Recurso::Cobre, 50_000)]), &mut dest, &datos(&p, true, false, 0, 0));
        assert_eq!(e.valor, 250_000);
        assert_eq!(e.comision, 11_000);
        assert_eq!(dest.cantidad(Recurso::Oro), 11_000, "la comisión se acuña en el destino, en unidades enteras");
        assert_eq!((dest.cantidad(Recurso::Madera), dest.cantidad(Recurso::Cobre)), (100_000, 50_000));
        assert!(e.perdido.is_empty() && e.recargo == 0);
    }

    #[test]
    fn un_trueque_de_armas_por_ganado_ya_acuna_comision() {
        let r = Reglas::cargar();
        let p = precios_a_uno(&r);
        // Antes las armas de madera no cotizaban y su mitad del trueque valía 0. 10 armas de madera (2 de oro cada una: 2 de madera de la
        // receta) = 20; externa a distancia máxima: 12 % = 2,4 → 2. Y 10 de ganado (2 cada uno) = 20 → otros 2.
        let mut dest = alm(&[]);
        let armas = entregar(&r, &carga(&[(Recurso::ArmaMadera, 10_000)]), &mut dest, &datos(&p, false, false, 0, 0));
        assert_eq!((armas.valor, armas.comision), (20_000, 2_000));
        let ganado = entregar(&r, &carga(&[(Recurso::Livestock, 10_000)]), &mut dest, &datos(&p, false, false, 0, 0));
        assert_eq!((ganado.valor, ganado.comision), (20_000, 2_000));
        assert_eq!(dest.cantidad(Recurso::Oro), 4_000);
    }

    #[test]
    fn lo_que_no_cabe_se_pierde_y_el_trueque_lo_da_por_entregado() {
        let r = Reglas::cargar();
        let p = precios_a_uno(&r);
        let mut dest = alm(&[(Recurso::Madera, 950_000)]);
        let lleva = carga(&[(Recurso::Madera, 100_000)]);
        let e = entregar(&r, &lleva, &mut dest, &datos(&p, true, false, 0, 0));
        assert_eq!((e.ingresado[&Recurso::Madera], e.perdido[&Recurso::Madera]), (50_000, 50_000));
        assert_eq!(dest.cantidad(Recurso::Madera), 1_000_000);
        assert_eq!(lleva[&Recurso::Madera], 100_000, "el abono usa lo que llevaba, no lo que entró");
        assert_eq!(e.valor, 100_000, "y el valor de la comisión también");
    }

    #[test]
    fn la_comision_se_topa_por_la_capacidad_de_oro() {
        let r = Reglas::cargar();
        let p = precios_a_uno(&r);
        let mut dest = alm(&[(Recurso::Oro, 999_000)]);
        let e = entregar(&r, &carga(&[(Recurso::Cobre, 100_000)]), &mut dest, &datos(&p, true, false, 0, 0));
        assert_eq!(e.comision, 1_000, "300 de valor × 4,5 % = 13,5, pero solo cabe 1");
        assert_eq!(dest.cantidad(Recurso::Oro), 1_000_000);
    }

    #[test]
    fn llevarse_la_carga_a_uno_mismo_no_acuna_nada() {
        let r = Reglas::cargar();
        let p = precios_a_uno(&r);
        let mut dest = alm(&[]);
        let e = entregar(&r, &carga(&[(Recurso::Cobre, 100_000)]), &mut dest, &datos(&p, false, true, -90 * PUNTO, 500_000));
        assert_eq!((e.comision, e.recargo), (0, 0));
        assert_eq!(dest.cantidad(Recurso::Oro), 0);
    }

    #[test]
    fn un_trueque_de_oro_por_oro_no_acuna_nada() {
        let r = Reglas::cargar();
        let p = precios_a_uno(&r);
        let mut dest = alm(&[]);
        let e = entregar(&r, &carga(&[(Recurso::Oro, 100_000)]), &mut dest, &datos(&p, false, false, 0, 0));
        assert_eq!((e.valor, e.comision), (0, 0));
        assert_eq!(dest.cantidad(Recurso::Oro), 100_000, "solo el oro entregado");
    }

    #[test]
    fn la_mala_reputacion_paga_el_extra_de_su_bolsillo_y_se_conserva() {
        let r = Reglas::cargar();
        let p = precios_a_uno(&r);
        let mut dest = alm(&[]);
        // Externa: 300 de valor × 12 % = 36; recargo = 50 % de 36 = 18, del oro del origen.
        let lleva = carga(&[(Recurso::Cobre, 100_000)]);
        let e = entregar(&r, &lleva, &mut dest, &datos(&p, false, false, -40 * PUNTO, 1_000_000));
        assert_eq!((e.comision, e.recargo), (36_000, 18_000));
        assert_eq!(dest.cantidad(Recurso::Oro), 36_000 + 18_000);
        // Sin oro en el origen: se queda en lo que tenga, nunca negativo.
        let mut dest = alm(&[]);
        let e = entregar(&r, &lleva, &mut dest, &datos(&p, false, false, -40 * PUNTO, 5_000));
        assert_eq!(e.recargo, 5_000);
        // Misma Facción: nada. Reputación por encima del umbral: nada.
        let mut dest = alm(&[]);
        assert_eq!(entregar(&r, &lleva, &mut dest, &datos(&p, true, false, -90 * PUNTO, 1_000_000)).recargo, 0);
        assert_eq!(entregar(&r, &lleva, &mut dest, &datos(&p, false, false, -39 * PUNTO, 1_000_000)).recargo, 0);
    }

    #[test]
    fn el_oro_se_conserva_o_se_acuna_nunca_se_destruye() {
        let r = Reglas::cargar();
        let p = precios_a_uno(&r);
        let lleva = carga(&[(Recurso::Cobre, 100_000)]);
        for oro_destino in [0, 990_000, 1_000_000] {
            let mut dest = alm(&[(Recurso::Oro, oro_destino)]);
            let e = entregar(&r, &lleva, &mut dest, &datos(&p, false, false, -90 * PUNTO, 1_000_000));
            // Lo que entró de oro en el destino es exactamente comisión + recargo: nada se pierde del recargo.
            assert_eq!(dest.cantidad(Recurso::Oro) - oro_destino, e.comision + e.recargo);
        }
    }
}
