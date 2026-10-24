//! El mostrador (Comercio_Caravanas §8.3): un jugador toma una orden **en persona**, con su columna en la puerta de la plaza. Aquí está
//! la aritmética: cuánto se sirve, cuánto oro cambia de manos y cómo se aplica a la plaza y a la orden. El carro de la columna lo
//! modifica quien llama con los deltas devueltos.
//!
//! **El oro se conserva**: se calcula `oroMovido` **una vez** y se aplica simétrico (lo que sale de uno entra en el otro). La cantidad
//! servida se redondea hacia abajo y el coste hacia arriba, de modo que ni la plaza ni el carro pagan más de lo que tienen. La comisión
//! no se acuña: la paga quien toma la orden y se la queda la plaza (comprando se paga de más, vendiendo se cobra de menos); no lleva
//! bonus por distancia ni recargo por reputación.

use crate::datos::Reglas;
use crate::orden_mercado::{EstadoOrden, OrdenMercado, TipoOrden};
use crate::precio::tasa;
use crate::tipos::AsentamientoId;
use asentamiento::datos::{por, UNO};
use asentamiento::{Almacen, Recurso};
use base::{entero, entero_arriba, Instante, Mil, UNIDAD};

/// El carro de la columna, en lo que importa a esta orden.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Carro {
    pub oro: Mil,
    /// Lo que lleva del recurso de la orden.
    pub recurso: Mil,
    /// `max(0, capacidadCargaDe(columna, caravanas) − Σ suministro)`.
    pub espacio: Mil,
}

/// Lo que el núcleo comprueba de la persona y de la plaza.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextoMostrador {
    pub ahora: Instante,
    /// El actor es el Líder de **su** columna.
    pub es_lider: bool,
    /// La columna está a `≤ radioPuerta` de la plaza.
    pub en_puerta: bool,
    pub mercado_activo: bool,
    pub misma_faccion: bool,
    /// `factorComisionExterna` de las políticas de la plaza (ppm; `UNO` si no hay).
    pub factor_comision_externa: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorMostrador {
    NoEsLider,
    /// La orden no es de esta plaza.
    OrdenAjena,
    /// La orden ya no está `activa` (cumplida o expirada).
    OrdenNoActiva,
    OrdenCaducada,
    FueraDeLaPuerta,
    SinMercado,
    CantidadNoPositiva,
    /// `precioUnitario <= 0` (también el de una orden sobre el oro sin precio de referencia): no se liquida.
    PrecioNoPositivo,
    /// Una orden sobre el oro no tiene sentido en el mostrador (el oro es a la vez lo que se paga y lo que se compra): se rechaza aunque lleve
    /// precio (el TypeScript solo la frenaba por el precio de referencia 0; Comercio_Caravanas §8.3).
    RecursoOro,
    /// Ninguno de los topes deja servir nada (sin stock, sin oro, sin sitio…).
    NadaQueServir,
}

/// Lo que se sirvió.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Servido {
    pub cantidad: Mil,
    /// `cantidad × precio` y su comisión (informativos; la comisión ya va incluida en `oro_movido`).
    pub valor: Mil,
    pub comision: Mil,
    /// El oro que cambia de manos, calculado una vez.
    pub oro_movido: Mil,
    /// Variación del recurso en el carro (`+cantidad` si la plaza vende, `−cantidad` si compra).
    pub delta_carro_recurso: Mil,
    /// Variación del oro del carro (`−oroMovido` si la plaza vende, `+oroMovido` si compra).
    pub delta_carro_oro: Mil,
    /// La orden queda `cumplida`.
    pub cumplida: bool,
}

/// `ceil(a × b / UNO)` con `a, b ≥ 0`.
fn por_arriba(a: Mil, b: i64) -> Mil {
    ((a as i128 * b as i128 + UNO as i128 - 1) / UNO as i128) as Mil
}

/// `floor(oro / precio_neto)` en milésimas de unidad: la mayor `q` con `por_arriba(q, pn) <= oro`.
fn max_por_oro(oro: Mil, pn: i64) -> Mil {
    (oro.max(0) as i128 * UNO as i128 / pn as i128) as Mil
}

/// `comerciarEnPlaza` del motor. `plaza` es el almacén de la plaza de la orden. Comprueba, en este orden: Líder, orden de esta plaza,
/// `activa` y sin caducar, puerta, Mercado, `pedida > 0` y precio > 0. Sirve `min(pedida, pendiente, lo que tenga cada parte, topes del
/// carro)`; si no se puede servir nada, error (no se devuelve «0 servido»).
pub fn comerciar(
    r: &Reglas,
    orden: &mut OrdenMercado,
    plaza_id: AsentamientoId,
    plaza: &mut Almacen,
    carro: &Carro,
    pedida: Mil,
    c: &ContextoMostrador,
) -> Result<Servido, ErrorMostrador> {
    if !c.es_lider {
        return Err(ErrorMostrador::NoEsLider);
    }
    if orden.asentamiento != plaza_id {
        return Err(ErrorMostrador::OrdenAjena);
    }
    if orden.estado != EstadoOrden::Activa {
        return Err(ErrorMostrador::OrdenNoActiva);
    }
    if c.ahora >= orden.expira_en {
        return Err(ErrorMostrador::OrdenCaducada);
    }
    if !c.en_puerta {
        return Err(ErrorMostrador::FueraDeLaPuerta);
    }
    if !c.mercado_activo {
        return Err(ErrorMostrador::SinMercado);
    }
    if pedida <= 0 {
        return Err(ErrorMostrador::CantidadNoPositiva);
    }
    if orden.precio_unitario <= 0 {
        return Err(ErrorMostrador::PrecioNoPositivo);
    }
    if orden.recurso == Recurso::Oro {
        return Err(ErrorMostrador::RecursoOro);
    }
    let tasa = tasa(&r.comision, c.misma_faccion, c.factor_comision_externa);
    let vende = orden.tipo == TipoOrden::Venta;
    let precio_neto = if vende { por(orden.precio_unitario, UNO + tasa) } else { por(orden.precio_unitario, UNO - tasa) };
    if precio_neto <= 0 {
        return Err(ErrorMostrador::PrecioNoPositivo);
    }
    // Unidades enteras (C3b): la mercancía se sirve en unidades y el oro de cada lote se cobra en unidades enteras, hacia arriba (como en el campamento).
    let oro_de = |q: Mil| entero_arriba(por_arriba(q, precio_neto));
    // Cuánto crece el carro con `q` unidades: vende → entra mercancía y sale oro; compra → sale mercancía y entra oro.
    let crece = |q: Mil| if vende { q - oro_de(q) } else { oro_de(q) - q };

    let mut q = pedida.min(orden.pendiente());
    if vende {
        q = q.min(plaza.cantidad(orden.recurso)).min(max_por_oro(carro.oro, precio_neto));
        // Topado también por el oro que cabe en el almacén de la plaza: si no, parte del oro cobrado se perdería (Comercio_Caravanas §8.3).
        q = q.min(max_por_oro(plaza.espacio(Recurso::Oro), precio_neto));
    } else {
        q = q.min(carro.recurso).min(plaza.espacio(orden.recurso)).min(max_por_oro(plaza.cantidad(Recurso::Oro), precio_neto));
    }
    // Tope del carro: cuánto puede crecer. Aproximación cerrada y ajuste fino en unidades enteras (el redondeo del oro mueve el borde).
    let por_unidad = if vende { UNO - precio_neto } else { precio_neto - UNO };
    if por_unidad > 0 {
        q = q.min((carro.espacio.max(0) as i128 * UNO as i128 / por_unidad as i128) as Mil);
    }
    let paga = |q: Mil| if vende { oro_de(q) <= carro.oro.min(plaza.espacio(Recurso::Oro)) } else { oro_de(q) <= plaza.cantidad(Recurso::Oro) };
    q = entero(q);
    while q > 0 && (!paga(q) || (por_unidad > 0 && crece(q) > carro.espacio.max(0))) {
        q -= UNIDAD;
    }
    if q <= 0 {
        return Err(ErrorMostrador::NadaQueServir);
    }
    let oro_movido = oro_de(q);
    let valor = por(q, orden.precio_unitario);
    let comision = por(valor, tasa);
    if vende {
        plaza.quitar(orden.recurso, q);
        plaza.agregar(Recurso::Oro, oro_movido);
    } else {
        plaza.quitar(Recurso::Oro, oro_movido);
        plaza.agregar(orden.recurso, q);
    }
    orden.cantidad_cumplida += q;
    let cumplida = orden.cantidad_cumplida >= orden.cantidad;
    if cumplida {
        orden.estado = EstadoOrden::Cumplida;
    }
    let (delta_carro_recurso, delta_carro_oro) = if vende { (q, -oro_movido) } else { (-q, oro_movido) };
    Ok(Servido { cantidad: q, valor, comision, oro_movido, delta_carro_recurso, delta_carro_oro, cumplida })
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::orden_mercado::OrdenMercado;

    const PLAZA: u32 = 5;

    fn reglas() -> Reglas {
        Reglas::cargar()
    }

    fn ctx() -> ContextoMostrador {
        ContextoMostrador { ahora: 10, es_lider: true, en_puerta: true, mercado_activo: true, misma_faccion: true, factor_comision_externa: UNO }
    }

    fn orden(tipo: TipoOrden, cantidad: Mil, precio: i64) -> OrdenMercado {
        OrdenMercado::colocar(&reglas().mercado, 1, PLAZA, tipo, Recurso::Madera, cantidad, Some(precio), 0, 0, true, true, cantidad).unwrap()
    }

    fn almacen(madera: Mil, oro: Mil) -> Almacen {
        let mut a = Almacen::nuevo(10_000_000);
        a.poner(Recurso::Madera, madera);
        a.poner(Recurso::Oro, oro);
        a
    }

    fn carro(oro: Mil, recurso: Mil, espacio: Mil) -> Carro {
        Carro { oro, recurso, espacio }
    }

    /// El oro de la plaza más el del carro es el mismo antes y después.
    fn oro_total(plaza: &Almacen, carro_oro: Mil, s: Option<&Servido>) -> Mil {
        plaza.cantidad(Recurso::Oro) + carro_oro + s.map_or(0, |s| s.delta_carro_oro)
    }

    #[test]
    fn la_plaza_vende_comprando_se_paga_de_mas() {
        let r = reglas();
        let mut o = orden(TipoOrden::Venta, 100_000, 1_000_000);
        let mut plaza = almacen(100_000, 0);
        let s = comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(500_000, 0, 1_000_000), 50_000, &ctx()).unwrap();
        // Misma Facción: 3 % → 1,03 por unidad; 50 unidades = 51,5 de oro, que se cobran como 52 (oro en unidades enteras, hacia arriba).
        assert_eq!((s.cantidad, s.oro_movido, s.comision, s.valor), (50_000, 52_000, 1_500, 50_000));
        assert_eq!((s.delta_carro_recurso, s.delta_carro_oro), (50_000, -52_000));
        assert_eq!((plaza.cantidad(Recurso::Madera), plaza.cantidad(Recurso::Oro)), (50_000, 52_000), "la comisión se queda en la plaza");
        assert_eq!((o.cantidad_cumplida, o.estado), (50_000, EstadoOrden::Activa));
    }

    #[test]
    fn la_plaza_compra_vendiendo_se_cobra_de_menos_y_externa_paga_8() {
        let r = reglas();
        let mut o = orden(TipoOrden::Compra, 100_000, 2_000_000);
        let mut plaza = almacen(0, 1_000_000);
        let c = ContextoMostrador { misma_faccion: false, ..ctx() };
        let s = comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(0, 40_000, 1_000_000), 40_000, &c).unwrap();
        // Externa: 8 % → 2 × 0,92 = 1,84 por unidad; 40 unidades = 73,6 de oro, que la plaza paga como 74.
        assert_eq!((s.cantidad, s.oro_movido, s.comision), (40_000, 74_000, 6_400));
        assert_eq!((s.delta_carro_recurso, s.delta_carro_oro), (-40_000, 74_000));
        assert_eq!((plaza.cantidad(Recurso::Madera), plaza.cantidad(Recurso::Oro)), (40_000, 1_000_000 - 74_000));
    }

    #[test]
    fn las_politicas_de_la_plaza_tocan_solo_la_tasa_externa() {
        let r = reglas();
        let mut o = orden(TipoOrden::Venta, 100_000, 1_000_000);
        let mut plaza = almacen(100_000, 0);
        let c = ContextoMostrador { misma_faccion: false, factor_comision_externa: 600_000, ..ctx() }; // Comercio Abierto
        let s = comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(500_000, 0, 1_000_000), 10_000, &c).unwrap();
        assert_eq!(s.oro_movido, 11_000, "8 % × 0,6 = 4,8 %: 10,48 de oro, cobrados como 11");
    }

    #[test]
    fn sirve_lo_que_puede_cada_tope_y_el_oro_se_conserva() {
        let r = reglas();
        // Tope por el oro del carro: 10 de oro a 1,03 → 9,708 unidades, que en enteras son 9 (cuestan 9,27 → 10 de oro).
        let mut o = orden(TipoOrden::Venta, 100_000, 1_000_000);
        let mut plaza = almacen(100_000, 0);
        let (c0, antes) = (carro(10_000, 0, 1_000_000), oro_total(&plaza, 10_000, None));
        let s = comerciar(&r, &mut o, PLAZA, &mut plaza, &c0, 100_000, &ctx()).unwrap();
        assert_eq!(s.cantidad, 9_000);
        assert!(s.oro_movido <= 10_000);
        assert_eq!(oro_total(&plaza, 10_000, Some(&s)), antes, "el oro se conserva");
        // Tope por el stock de la plaza.
        let mut o = orden(TipoOrden::Venta, 100_000, 1_000_000);
        let mut plaza = almacen(7_000, 0);
        assert_eq!(comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(1_000_000, 0, 1_000_000), 100_000, &ctx()).unwrap().cantidad, 7_000);
        // Tope por lo pendiente de la orden y cierre al cumplirse.
        let mut o = orden(TipoOrden::Venta, 20_000, 1_000_000);
        let mut plaza = almacen(100_000, 0);
        let s = comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(1_000_000, 0, 1_000_000), 50_000, &ctx()).unwrap();
        assert_eq!(s.cantidad, 20_000);
        assert!(s.cumplida && o.estado == EstadoOrden::Cumplida);
        // Tope por el oro de la plaza al comprar, por lo que lleva el carro y por el hueco del almacén.
        let mut o = orden(TipoOrden::Compra, 100_000, 1_000_000);
        let mut plaza = almacen(0, 10_000);
        assert_eq!(comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(0, 100_000, 1_000_000), 100_000, &ctx()).unwrap().cantidad, 10_000, "10 de oro a 0,97: 10 unidades (9,7 → 10 de oro); 11 ya costarían 11");
        let mut o = orden(TipoOrden::Compra, 100_000, 1_000_000);
        let mut plaza = almacen(0, 1_000_000);
        assert_eq!(comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(0, 3_000, 1_000_000), 100_000, &ctx()).unwrap().cantidad, 3_000);
        let mut plaza = almacen(9_990_000, 1_000_000);
        assert_eq!(comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(0, 50_000, 1_000_000), 100_000, &ctx()).unwrap().cantidad, 10_000, "solo cabían 10");
    }

    #[test]
    fn el_oro_ocupa_carro_el_techo_fisico() {
        let r = reglas();
        // La plaza compra a 3 (neto 2,91 > 1): el carro crece 1,91 por unidad vendida; con 10 de espacio, 5 unidades enteras (15 de oro: crece 10).
        let mut o = orden(TipoOrden::Compra, 100_000, 3_000_000);
        let mut plaza = almacen(0, 10_000_000);
        let s = comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(0, 100_000, 10_000), 100_000, &ctx()).unwrap();
        assert!(s.delta_carro_oro + s.delta_carro_recurso <= 10_000, "no se pasa del espacio");
        assert!(s.delta_carro_oro + s.delta_carro_recurso > 10_000 - 2, "y casi lo llena: {s:?}");
        assert_eq!(s.cantidad, 5_000);
        // Con el carro lleno no se sirve nada.
        let mut o = orden(TipoOrden::Compra, 100_000, 3_000_000);
        assert_eq!(comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(0, 100_000, 0), 100_000, &ctx()), Err(ErrorMostrador::NadaQueServir));
        // Si el precio neto es < 1 al comprar de la plaza, el carro crece por la mercancía: 0,5 de precio → crece 0,485 por unidad.
        let mut o = orden(TipoOrden::Venta, 100_000, 500_000);
        let mut plaza = almacen(100_000, 0);
        let s = comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(1_000_000, 0, 4_850), 100_000, &ctx()).unwrap();
        assert_eq!(s.cantidad, 10_000);
    }

    #[test]
    fn falla_si_no_puede_servir_nada() {
        let r = reglas();
        let mut o = orden(TipoOrden::Venta, 100_000, 1_000_000);
        let mut plaza = almacen(0, 0);
        assert_eq!(comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(1_000_000, 0, 1_000_000), 1_000, &ctx()), Err(ErrorMostrador::NadaQueServir));
        let mut plaza = almacen(1_000, 0);
        assert_eq!(comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(0, 0, 1_000_000), 1_000, &ctx()), Err(ErrorMostrador::NadaQueServir), "sin oro en el carro");
        assert_eq!(o.cantidad_cumplida, 0, "un fallo no toca la orden");
    }

    #[test]
    fn las_condiciones_se_comprueban_en_orden() {
        let r = reglas();
        let mut o = orden(TipoOrden::Venta, 100_000, 1_000_000);
        let mut plaza = almacen(1_000, 0);
        let ca = carro(1_000_000, 0, 1_000_000);
        let expira = o.expira_en;
        let mut prueba = |o: &mut OrdenMercado, c: ContextoMostrador, p: Mil| comerciar(&r, o, PLAZA, &mut plaza, &ca, p, &c);
        assert_eq!(prueba(&mut o, ContextoMostrador { es_lider: false, ..ctx() }, 1_000), Err(ErrorMostrador::NoEsLider));
        assert_eq!(prueba(&mut o, ContextoMostrador { ahora: expira, ..ctx() }, 1_000), Err(ErrorMostrador::OrdenCaducada));
        assert_eq!(prueba(&mut o, ContextoMostrador { en_puerta: false, ..ctx() }, 1_000), Err(ErrorMostrador::FueraDeLaPuerta));
        assert_eq!(prueba(&mut o, ContextoMostrador { mercado_activo: false, ..ctx() }, 1_000), Err(ErrorMostrador::SinMercado));
        assert_eq!(prueba(&mut o, ctx(), 0), Err(ErrorMostrador::CantidadNoPositiva));
        let mut ajena = o.clone();
        ajena.asentamiento = 99;
        assert_eq!(prueba(&mut ajena, ctx(), 1_000), Err(ErrorMostrador::OrdenAjena));
        let mut cerrada = o.clone();
        cerrada.estado = EstadoOrden::Cumplida;
        assert_eq!(prueba(&mut cerrada, ctx(), 1_000), Err(ErrorMostrador::OrdenNoActiva));
    }

    #[test]
    fn una_orden_sin_precio_no_se_liquida() {
        let r = reglas();
        let mut plaza = almacen(1_000, 1_000);
        let ca = carro(1_000_000, 1_000, 1_000_000);
        for precio in [0, -5] {
            let mut o = orden(TipoOrden::Venta, 100_000, precio);
            assert_eq!(comerciar(&r, &mut o, PLAZA, &mut plaza, &ca, 1_000, &ctx()), Err(ErrorMostrador::PrecioNoPositivo));
        }
        // Una orden sobre el oro sin precio de referencia se coloca con precio 0 y tampoco se liquida.
        let mut oro = OrdenMercado::colocar(&r.mercado, 2, PLAZA, TipoOrden::Compra, Recurso::Oro, 10_000, None, 0, 0, true, true, 0).unwrap();
        assert_eq!(comerciar(&r, &mut oro, PLAZA, &mut plaza, &ca, 1_000, &ctx()), Err(ErrorMostrador::PrecioNoPositivo));
        oro.precio_unitario = 1_000_000;
        assert_eq!(comerciar(&r, &mut oro, PLAZA, &mut plaza, &ca, 1_000, &ctx()), Err(ErrorMostrador::RecursoOro));
    }

    #[test]
    fn varias_tomas_cierran_la_orden_y_conservan_el_oro() {
        let r = reglas();
        let mut o = orden(TipoOrden::Venta, 30_000, 1_700_000);
        let mut plaza = almacen(100_000, 5_000);
        let mut oro_carro = 200_000;
        let total_antes = plaza.cantidad(Recurso::Oro) + oro_carro;
        let mut vendido = 0;
        for _ in 0..10 {
            let ca = carro(oro_carro, 0, 1_000_000);
            match comerciar(&r, &mut o, PLAZA, &mut plaza, &ca, 7_777, &ctx()) {
                Ok(s) => {
                    oro_carro += s.delta_carro_oro;
                    vendido += s.cantidad;
                }
                Err(e) => {
                    assert_eq!(e, ErrorMostrador::OrdenNoActiva);
                    break;
                }
            }
            assert_eq!(plaza.cantidad(Recurso::Oro) + oro_carro, total_antes, "el oro se conserva a cada toma");
        }
        assert_eq!(vendido, 30_000);
        assert_eq!(o.estado, EstadoOrden::Cumplida);
        assert_eq!(plaza.cantidad(Recurso::Madera), 70_000);
    }

    #[test]
    fn es_determinista() {
        let r = reglas();
        let hacer = || {
            let mut o = orden(TipoOrden::Compra, 100_000, 1_234_567);
            let mut plaza = almacen(0, 777_777);
            comerciar(&r, &mut o, PLAZA, &mut plaza, &carro(0, 55_555, 12_345), 99_999, &ctx()).unwrap()
        };
        assert_eq!(hacer(), hacer());
    }
}
