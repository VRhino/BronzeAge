//! El mostrador (Comercio_Caravanas §8.3): un jugador toma una orden **en persona**, con su columna en la puerta de la plaza. Aquí está
//! la aritmética: cuánto se sirve, cuánto oro cambia de manos y cómo se aplica a la plaza y a la orden. El carro de la columna lo
//! modifica quien llama con los deltas devueltos.
//!
//! **El oro se conserva**: se calcula `oroMovido` **una vez** y se aplica simétrico (lo que sale de uno entra en el otro). La cantidad
//! servida se redondea hacia abajo y el coste hacia arriba, de modo que ni la plaza ni el carro pagan más de lo que tienen. La comisión
//! no se acuña: la paga quien toma la orden y se la queda la plaza (comprando se paga de más, vendiendo se cobra de menos); no lleva
//! bonus por distancia ni recargo por reputación.
//!
//! La caravana con encargo usa las mismas cuentas en una plaza ajena: [`vender`] y [`comprar`] sirven sus líneas a las órdenes de la plaza con
//! `servir`, en vez de tocar el almacén (Comercio_Caravanas §5.8.8).

use crate::datos::Reglas;
use crate::orden_mercado::{EstadoOrden, OrdenMercado, TipoOrden};
use crate::precio::tasa;
use crate::encargo::{LineaDejar, LineaRecoger};
use crate::tipos::{de, total, AsentamientoId, Carga, OrdenId};
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
    servir(r, orden, plaza, carro, pedida, c.misma_faccion, c.factor_comision_externa)
}

/// Sirve `orden` desde `plaza` contra `carro`: comprueba `pedida > 0`, precio > 0 y que el recurso no sea el oro, y aplica la aritmética a la plaza y a la
/// orden. Es lo que comparten el mostrador en persona y la caravana con encargo (Comercio_Caravanas §8.3).
fn servir(r: &Reglas, orden: &mut OrdenMercado, plaza: &mut Almacen, carro: &Carro, pedida: Mil, misma_faccion: bool, factor_comision_externa: i64) -> Result<Servido, ErrorMostrador> {
    if pedida <= 0 {
        return Err(ErrorMostrador::CantidadNoPositiva);
    }
    if orden.precio_unitario <= 0 {
        return Err(ErrorMostrador::PrecioNoPositivo);
    }
    if orden.recurso == Recurso::Oro {
        return Err(ErrorMostrador::RecursoOro);
    }
    let tasa = tasa(&r.comision, misma_faccion, factor_comision_externa);
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

/// La plaza ajena donde comercia una caravana con encargo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextoParada {
    pub plaza_id: AsentamientoId,
    pub ahora: Instante,
    /// `factorComisionExterna` de las políticas de la plaza (ppm; `UNO` si no hay).
    pub factor_comision_externa: i64,
}

/// Una orden servida a la caravana.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trato {
    pub orden: OrdenId,
    pub servido: Servido,
}

/// Lo que una línea del encargo pide a las órdenes de la plaza.
struct Pedido {
    recurso: Recurso,
    /// Tipo de las órdenes que se toman: las de compra al vender la caravana, las de venta al comprar.
    tipo: TipoOrden,
    /// Precio mínimo (órdenes de compra) o máximo (órdenes de venta) que acepta la caravana.
    limite: i64,
    /// Lo que le falta a la línea.
    falta: Mil,
}

/// Suma `delta` al recurso de la carga y quita la entrada si queda a 0.
fn aplicar(a_bordo: &mut Carga, recurso: Recurso, delta: Mil) {
    let n = de(a_bordo, recurso) + delta;
    if n > 0 {
        a_bordo.insert(recurso, n);
    } else {
        a_bordo.remove(&recurso);
    }
}

/// Sirve una línea a las órdenes activas de la plaza, sin caducar y del recurso, que cumplan el precio límite: la que mejor trata a la caravana primero
/// y, a igual precio, por id ascendente. Una orden que `servir` rechaza se salta. Se detiene al no faltar nada o al no quedar órdenes.
fn atender(r: &Reglas, ordenes: &mut [OrdenMercado], plaza: &mut Almacen, a_bordo: &mut Carga, capacidad: Mil, c: &ContextoParada, p: Pedido) -> Vec<Trato> {
    let mut tratos = Vec::new();
    if p.recurso == Recurso::Oro {
        return tratos;
    }
    // Las de compra se ordenan de mayor a menor precio y las de venta de menor a mayor: con el signo, ambas son «menor clave primero».
    let signo = match p.tipo {
        TipoOrden::Compra => -1,
        TipoOrden::Venta => 1,
    };
    let mut elegidas: Vec<usize> = (0..ordenes.len())
        .filter(|&i| {
            let o = &ordenes[i];
            o.asentamiento == c.plaza_id
                && o.estado == EstadoOrden::Activa
                && c.ahora < o.expira_en
                && o.tipo == p.tipo
                && o.recurso == p.recurso
                && signo * o.precio_unitario <= signo * p.limite
        })
        .collect();
    elegidas.sort_by_key(|&i| (signo * ordenes[i].precio_unitario, ordenes[i].id));
    let mut falta = p.falta;
    for i in elegidas {
        if falta <= 0 {
            break;
        }
        let carro = Carro { oro: de(a_bordo, Recurso::Oro), recurso: de(a_bordo, p.recurso), espacio: (capacidad - total(a_bordo)).max(0) };
        let Ok(servido) = servir(r, &mut ordenes[i], plaza, &carro, falta, false, c.factor_comision_externa) else { continue };
        aplicar(a_bordo, Recurso::Oro, servido.delta_carro_oro);
        aplicar(a_bordo, p.recurso, servido.delta_carro_recurso);
        falta -= servido.cantidad;
        tratos.push(Trato { orden: ordenes[i].id, servido });
    }
    tratos
}

/// La caravana con encargo vende en una plaza ajena (Comercio_Caravanas §5.8.8 y §8.3). Línea a línea, en el orden de `lineas`, toma las
/// órdenes de **compra** de la plaza con `precio_unitario >= precio_min`, de la que mejor paga a la que peor (a igual precio, por id ascendente), y vende
/// como mucho `min(tope, lo que lleva)` del recurso (sin tope, todo lo que lleva). `ordenes` es el libro entero: solo cuentan las de `c.plaza_id`,
/// activas y sin caducar. Una línea sin precio mínimo o de oro no comercia. Cada orden se sirve con las cuentas del mostrador como una columna de
/// otra facción; el oro y la mercancía se aplican a `a_bordo` y a `plaza`, así que el oro se conserva. `capacidad` es la carga máxima de la caravana.
pub fn vender(r: &Reglas, lineas: &[LineaDejar], ordenes: &mut [OrdenMercado], plaza: &mut Almacen, a_bordo: &mut Carga, capacidad: Mil, c: &ContextoParada) -> Vec<Trato> {
    let mut tratos = Vec::new();
    for l in lineas {
        let Some(limite) = l.precio_min else { continue };
        let lleva = de(a_bordo, l.recurso);
        let falta = l.tope.map_or(lleva, |t| t.min(lleva));
        tratos.extend(atender(r, ordenes, plaza, a_bordo, capacidad, c, Pedido { recurso: l.recurso, tipo: TipoOrden::Compra, limite, falta }));
    }
    tratos
}

/// La caravana con encargo compra en una plaza ajena (Comercio_Caravanas §5.8.8 y §8.3). Línea a línea, en el orden de `lineas`, toma las
/// órdenes de **venta** de la plaza con `precio_unitario <= precio_max`, de la más barata a la más cara (a igual precio, por id ascendente), y compra
/// como mucho la `cantidad` de la línea. Mismas reglas que [`vender`] para el libro, las líneas sin precio o de oro, el mostrador y la conservación del oro.
pub fn comprar(r: &Reglas, lineas: &[LineaRecoger], ordenes: &mut [OrdenMercado], plaza: &mut Almacen, a_bordo: &mut Carga, capacidad: Mil, c: &ContextoParada) -> Vec<Trato> {
    let mut tratos = Vec::new();
    for l in lineas {
        let Some(limite) = l.precio_max else { continue };
        tratos.extend(atender(r, ordenes, plaza, a_bordo, capacidad, c, Pedido { recurso: l.recurso, tipo: TipoOrden::Venta, limite, falta: l.cantidad }));
    }
    tratos
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

    const MADERA: Recurso = Recurso::Madera;

    fn ctx_parada() -> ContextoParada {
        ContextoParada { plaza_id: PLAZA, ahora: 10, factor_comision_externa: UNO }
    }

    /// Una orden del libro con id propio, en la plaza de las pruebas.
    fn en_libro(id: OrdenId, tipo: TipoOrden, cantidad: Mil, precio: i64) -> OrdenMercado {
        OrdenMercado { id, ..orden(tipo, cantidad, precio) }
    }

    fn a_bordo(cosas: &[(Recurso, Mil)]) -> Carga {
        cosas.iter().copied().collect()
    }

    fn dejar(recurso: Recurso, tope: Option<Mil>, precio_min: Option<i64>) -> LineaDejar {
        LineaDejar { recurso, tope, precio_min }
    }

    fn recoger(recurso: Recurso, cantidad: Mil, precio_max: Option<i64>) -> LineaRecoger {
        LineaRecoger { recurso, cantidad, precio_max }
    }

    fn cantidades(tratos: &[Trato]) -> Vec<(OrdenId, Mil)> {
        tratos.iter().map(|t| (t.orden, t.servido.cantidad)).collect()
    }

    #[test]
    fn vender_toma_primero_la_orden_que_mejor_paga_y_empata_por_id() {
        let r = reglas();
        let mut libro = vec![en_libro(2, TipoOrden::Compra, 10_000, 2_000_000), en_libro(1, TipoOrden::Compra, 10_000, 2_000_000), en_libro(3, TipoOrden::Compra, 10_000, 3_000_000)];
        let mut plaza = almacen(0, 1_000_000);
        let mut bordo = a_bordo(&[(MADERA, 25_000)]);
        let tratos = vender(&r, &[dejar(MADERA, None, Some(1_000_000))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        assert_eq!(cantidades(&tratos), [(3, 10_000), (1, 10_000), (2, 5_000)], "sin tope vende todo lo que lleva");
        assert_eq!(de(&bordo, MADERA), 0);
        assert!(!bordo.contains_key(&MADERA), "la entrada a 0 se quita");
    }

    #[test]
    fn comprar_toma_primero_la_orden_mas_barata_y_empata_por_id() {
        let r = reglas();
        let mut libro = vec![en_libro(2, TipoOrden::Venta, 10_000, 2_000_000), en_libro(1, TipoOrden::Venta, 10_000, 2_000_000), en_libro(3, TipoOrden::Venta, 10_000, 1_000_000)];
        let mut plaza = almacen(100_000, 0);
        let mut bordo = a_bordo(&[(Recurso::Oro, 1_000_000)]);
        let tratos = comprar(&r, &[recoger(MADERA, 25_000, Some(5_000_000))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        assert_eq!(cantidades(&tratos), [(3, 10_000), (1, 10_000), (2, 5_000)]);
        assert_eq!(de(&bordo, MADERA), 25_000);
    }

    #[test]
    fn el_precio_minimo_al_vender_excluye_las_ordenes_que_pagan_menos() {
        let r = reglas();
        let mut libro = vec![en_libro(1, TipoOrden::Compra, 10_000, 1_000_000), en_libro(2, TipoOrden::Compra, 10_000, 2_000_000), en_libro(3, TipoOrden::Compra, 10_000, 3_000_000)];
        let mut plaza = almacen(0, 1_000_000);
        let mut bordo = a_bordo(&[(MADERA, 30_000)]);
        let tratos = vender(&r, &[dejar(MADERA, None, Some(2_000_000))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        assert_eq!(cantidades(&tratos), [(3, 10_000), (2, 10_000)], "el precio mínimo es inclusivo");
        assert_eq!(de(&bordo, MADERA), 10_000);
        assert_eq!(libro[0].cantidad_cumplida, 0);
    }

    #[test]
    fn el_precio_maximo_al_comprar_excluye_las_ordenes_que_cuestan_mas() {
        let r = reglas();
        let mut libro = vec![en_libro(1, TipoOrden::Venta, 10_000, 1_000_000), en_libro(2, TipoOrden::Venta, 10_000, 2_000_000), en_libro(3, TipoOrden::Venta, 10_000, 3_000_000)];
        let mut plaza = almacen(100_000, 0);
        let mut bordo = a_bordo(&[(Recurso::Oro, 1_000_000)]);
        let tratos = comprar(&r, &[recoger(MADERA, 30_000, Some(2_000_000))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        assert_eq!(cantidades(&tratos), [(1, 10_000), (2, 10_000)], "el precio máximo es inclusivo");
        assert_eq!(libro[2].cantidad_cumplida, 0);
    }

    #[test]
    fn vender_respeta_el_tope_de_la_linea_y_lo_que_lleva() {
        let r = reglas();
        let mut libro = vec![en_libro(1, TipoOrden::Compra, 100_000, 2_000_000)];
        let mut plaza = almacen(0, 1_000_000);
        let mut bordo = a_bordo(&[(MADERA, 10_000)]);
        let tratos = vender(&r, &[dejar(MADERA, Some(4_000), Some(1_000_000))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        assert_eq!(cantidades(&tratos), [(1, 4_000)]);
        assert_eq!(de(&bordo, MADERA), 6_000);
        // Un tope mayor que lo que lleva no hace vender más.
        let tratos = vender(&r, &[dejar(MADERA, Some(50_000), Some(1_000_000))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        assert_eq!(cantidades(&tratos), [(1, 6_000)]);
    }

    #[test]
    fn comprar_respeta_la_cantidad_de_la_linea() {
        let r = reglas();
        let mut libro = vec![en_libro(1, TipoOrden::Venta, 100_000, 1_000_000)];
        let mut plaza = almacen(100_000, 0);
        let mut bordo = a_bordo(&[(Recurso::Oro, 1_000_000)]);
        let tratos = comprar(&r, &[recoger(MADERA, 6_000, Some(2_000_000))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        assert_eq!(cantidades(&tratos), [(1, 6_000)]);
        assert_eq!(de(&bordo, MADERA), 6_000);
    }

    #[test]
    fn comprar_se_queda_en_lo_que_alcanza_el_oro_de_a_bordo() {
        let r = reglas();
        let mut libro = vec![en_libro(1, TipoOrden::Venta, 100_000, 1_000_000)];
        let mut plaza = almacen(100_000, 0);
        // Externa: 1,08 por unidad. 5 de oro dan 4,6 unidades, que en enteras son 4 (cuestan 4,32 → 5 de oro).
        let mut bordo = a_bordo(&[(Recurso::Oro, 5_000)]);
        let tratos = comprar(&r, &[recoger(MADERA, 100_000, Some(2_000_000))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        assert_eq!(cantidades(&tratos), [(1, 4_000)]);
        assert_eq!((de(&bordo, Recurso::Oro), de(&bordo, MADERA)), (0, 4_000));
        assert!(!bordo.contains_key(&Recurso::Oro));
    }

    #[test]
    fn vender_se_queda_en_lo_que_cabe_en_la_caravana() {
        let r = reglas();
        let mut libro = vec![en_libro(1, TipoOrden::Compra, 100_000, 3_000_000)];
        let mut plaza = almacen(0, 1_000_000);
        // Lleva 10 y la capacidad es 14: quedan 4 de hueco. El oro cobrado ocupa más que la mercancía vendida (2,76 por unidad: crece 1,76):
        // 2 unidades dan 6 de oro (crece 4); 3 darían 9 (crece 6).
        let mut bordo = a_bordo(&[(MADERA, 10_000)]);
        let tratos = vender(&r, &[dejar(MADERA, None, Some(1_000_000))], &mut libro, &mut plaza, &mut bordo, 14_000, &ctx_parada());
        assert_eq!(cantidades(&tratos), [(1, 2_000)]);
        assert!(total(&bordo) <= 14_000);
    }

    #[test]
    fn comprar_se_queda_en_lo_que_cabe_en_la_caravana() {
        let r = reglas();
        let mut libro = vec![en_libro(1, TipoOrden::Venta, 100_000, 500_000)];
        let mut plaza = almacen(100_000, 0);
        // Lleva 10 de oro y la capacidad es 12: quedan 2 de hueco. A 0,54 por unidad la carga crece 0,46 por unidad comprada: 4 unidades (crece 1).
        let mut bordo = a_bordo(&[(Recurso::Oro, 10_000)]);
        let tratos = comprar(&r, &[recoger(MADERA, 100_000, Some(1_000_000))], &mut libro, &mut plaza, &mut bordo, 12_000, &ctx_parada());
        assert_eq!(cantidades(&tratos), [(1, 4_000)]);
        assert!(total(&bordo) <= 12_000);
    }

    #[test]
    fn una_orden_se_cumple_a_medias_y_la_otra_entera() {
        let r = reglas();
        let mut libro = vec![en_libro(1, TipoOrden::Compra, 10_000, 3_000_000), en_libro(2, TipoOrden::Compra, 10_000, 2_000_000)];
        let mut plaza = almacen(0, 1_000_000);
        let mut bordo = a_bordo(&[(MADERA, 15_000)]);
        let tratos = vender(&r, &[dejar(MADERA, None, Some(1_000_000))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        assert_eq!(cantidades(&tratos), [(1, 10_000), (2, 5_000)]);
        assert!(tratos[0].servido.cumplida && !tratos[1].servido.cumplida);
        assert_eq!((libro[0].estado, libro[0].cantidad_cumplida), (EstadoOrden::Cumplida, 10_000));
        assert_eq!((libro[1].estado, libro[1].cantidad_cumplida), (EstadoOrden::Activa, 5_000));
    }

    #[test]
    fn las_ordenes_de_otra_plaza_caducadas_o_no_activas_no_cuentan() {
        let r = reglas();
        let valida = en_libro(1, TipoOrden::Compra, 100_000, 2_000_000);
        let mut otra_plaza = en_libro(2, TipoOrden::Compra, 100_000, 9_000_000);
        otra_plaza.asentamiento = PLAZA + 1;
        let mut caducada = en_libro(3, TipoOrden::Compra, 100_000, 9_000_000);
        caducada.expira_en = 10;
        let mut cumplida = en_libro(4, TipoOrden::Compra, 100_000, 9_000_000);
        cumplida.estado = EstadoOrden::Cumplida;
        let mut libro = vec![otra_plaza, caducada, cumplida, valida];
        let mut plaza = almacen(0, 1_000_000);
        let mut bordo = a_bordo(&[(MADERA, 10_000)]);
        let tratos = vender(&r, &[dejar(MADERA, None, Some(1_000_000))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        assert_eq!(cantidades(&tratos), [(1, 10_000)]);
        assert!(libro[..3].iter().all(|o| o.cantidad_cumplida == 0));
    }

    #[test]
    fn una_linea_sin_precio_o_de_oro_no_comercia() {
        let r = reglas();
        let mut libro = vec![en_libro(1, TipoOrden::Compra, 100_000, 2_000_000), en_libro(2, TipoOrden::Venta, 100_000, 2_000_000)];
        let mut plaza = almacen(100_000, 1_000_000);
        let mut bordo = a_bordo(&[(MADERA, 10_000), (Recurso::Oro, 100_000)]);
        let antes = (bordo.clone(), plaza.clone());
        let vendido = vender(&r, &[dejar(MADERA, None, None), dejar(Recurso::Oro, None, Some(1))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        let comprado = comprar(&r, &[recoger(MADERA, 10_000, None), recoger(Recurso::Oro, 10_000, Some(5_000_000))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        assert!(vendido.is_empty() && comprado.is_empty());
        assert_eq!((bordo, plaza), antes);
        assert!(libro.iter().all(|o| o.cantidad_cumplida == 0));
    }

    #[test]
    fn vender_y_comprar_conservan_el_oro_total() {
        let r = reglas();
        let mut libro = vec![en_libro(1, TipoOrden::Compra, 100_000, 1_700_000), en_libro(2, TipoOrden::Venta, 100_000, 1_300_000)];
        let mut plaza = almacen(50_000, 300_000);
        let mut bordo = a_bordo(&[(MADERA, 33_333), (Recurso::Oro, 77_777)]);
        let total_antes = plaza.cantidad(Recurso::Oro) + de(&bordo, Recurso::Oro);
        let vendido = vender(&r, &[dejar(MADERA, Some(20_000), Some(1_000_000))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        assert!(!vendido.is_empty());
        assert_eq!(plaza.cantidad(Recurso::Oro) + de(&bordo, Recurso::Oro), total_antes, "tras vender");
        let comprado = comprar(&r, &[recoger(MADERA, 30_000, Some(2_000_000))], &mut libro, &mut plaza, &mut bordo, 1_000_000, &ctx_parada());
        assert!(!comprado.is_empty());
        assert_eq!(plaza.cantidad(Recurso::Oro) + de(&bordo, Recurso::Oro), total_antes, "tras comprar");
    }
}
