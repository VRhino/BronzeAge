//! Órdenes del mercado de una plaza (Comercio_Caravanas §8.1-§8.2): ofertas en pie en **una** plaza, sin emparejamiento automático, que
//! caducan a su instante (evento) y dejan un historial de las últimas `MERCADO.historialPorPlaza` de cada plaza.
//!
//! Las órdenes **no reservan nada** (ni la mercancía de una venta ni el oro de una compra): todo se comprueba al liquidar en el mostrador.

use crate::datos::ReglasMercado;
use crate::tipos::{AsentamientoId, OrdenId};
use asentamiento::Recurso;
use base::{Instante, Mil};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoOrden {
    /// La plaza quiere comprar.
    Compra,
    /// La plaza vende.
    Venta,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EstadoOrden {
    Activa,
    Cumplida,
    Expirada,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorOrden {
    /// El actor no es Tesorero ni Gobernador de la plaza (residente y dentro).
    NoAutorizado,
    /// La plaza no tiene Mercado activo.
    SinMercado,
    CantidadNoPositiva,
    /// Una venta exige tener hoy `≥ cantidad` de ese recurso en el almacén.
    SinStock,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrdenMercado {
    pub id: OrdenId,
    pub asentamiento: AsentamientoId,
    pub tipo: TipoOrden,
    pub recurso: Recurso,
    pub cantidad: Mil,
    pub cantidad_cumplida: Mil,
    /// ppm de oro por unidad. Puede ser ≤ 0 (no se valida al colocar; entonces no se liquida).
    pub precio_unitario: i64,
    pub creado_en: Instante,
    pub expira_en: Instante,
    pub estado: EstadoOrden,
}

impl OrdenMercado {
    /// `colocarOrdenMercado` (§8.1). `precio` es el que fija el jugador; si no, el de referencia (que para el oro es 0: la orden se
    /// coloca igual y no se liquida). `stock_actual` es lo que hay hoy del recurso en el almacén (solo importa en una venta).
    #[allow(clippy::too_many_arguments)]
    pub fn colocar(
        r: &ReglasMercado,
        id: OrdenId,
        asentamiento: AsentamientoId,
        tipo: TipoOrden,
        recurso: Recurso,
        cantidad: Mil,
        precio: Option<i64>,
        precio_referencia: i64,
        ahora: Instante,
        puede_ordenar: bool,
        tiene_mercado: bool,
        stock_actual: Mil,
    ) -> Result<Self, ErrorOrden> {
        if !puede_ordenar {
            return Err(ErrorOrden::NoAutorizado);
        }
        if !tiene_mercado {
            return Err(ErrorOrden::SinMercado);
        }
        let cantidad = base::entero(cantidad); // solo unidades enteras (C3b)
        if cantidad <= 0 {
            return Err(ErrorOrden::CantidadNoPositiva);
        }
        if tipo == TipoOrden::Venta && stock_actual < cantidad {
            return Err(ErrorOrden::SinStock);
        }
        Ok(Self {
            id,
            asentamiento,
            tipo,
            recurso,
            cantidad,
            cantidad_cumplida: 0,
            precio_unitario: precio.unwrap_or(precio_referencia),
            creado_en: ahora,
            expira_en: ahora + r.plazo_orden,
            estado: EstadoOrden::Activa,
        })
    }

    pub fn pendiente(&self) -> Mil {
        (self.cantidad - self.cantidad_cumplida).max(0)
    }

    /// ¿Se puede tomar en `ahora`? (`activa` y sin caducar).
    pub fn vigente(&self, ahora: Instante) -> bool {
        self.estado == EstadoOrden::Activa && ahora < self.expira_en
    }
}

/// Las órdenes en pie y el historial de las cerradas.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibroOrdenes {
    pub ordenes: Vec<OrdenMercado>,
    /// Del más viejo al más nuevo; solo las últimas `historialPorPlaza` de cada plaza.
    pub historial: Vec<OrdenMercado>,
}

impl LibroOrdenes {
    pub fn colocar(&mut self, o: OrdenMercado) {
        self.ordenes.push(o);
    }

    pub fn orden(&self, id: OrdenId) -> Option<&OrdenMercado> {
        self.ordenes.iter().find(|o| o.id == id)
    }

    pub fn orden_mut(&mut self, id: OrdenId) -> Option<&mut OrdenMercado> {
        self.ordenes.iter_mut().find(|o| o.id == id)
    }

    /// El primer instante en que hay que mirar el libro (la caducidad de la orden activa más próxima).
    pub fn proxima_caducidad(&self) -> Option<Instante> {
        self.ordenes.iter().filter(|o| o.estado == EstadoOrden::Activa).map(|o| o.expira_en).min()
    }

    /// El evento de caducidad (§8.2): las `activa` con `ahora >= expira_en` pasan a `expirada` (devuelve sus ids, para
    /// `mercado.orden_expirada`) y, además, todas las que ya no están `activa` salen de las órdenes hacia el historial. Las cumplidas
    /// salen sin evento.
    pub fn caducar(&mut self, r: &ReglasMercado, ahora: Instante) -> Vec<OrdenId> {
        let mut expiradas = Vec::new();
        for o in self.ordenes.iter_mut().filter(|o| o.estado == EstadoOrden::Activa && ahora >= o.expira_en) {
            o.estado = EstadoOrden::Expirada;
            expiradas.push(o.id);
        }
        self.archivar_cerradas(r);
        expiradas
    }

    /// Mueve las cerradas (`cumplida` o `expirada`) al historial (`anexarAlHistorialDeOrdenes`): de cada plaza se recuerdan solo las
    /// últimas `historialPorPlaza`.
    pub fn archivar_cerradas(&mut self, r: &ReglasMercado) {
        let (cerradas, vivas): (Vec<_>, Vec<_>) = std::mem::take(&mut self.ordenes).into_iter().partition(|o| o.estado != EstadoOrden::Activa);
        self.ordenes = vivas;
        self.historial.extend(cerradas);
        let mut plazas: Vec<AsentamientoId> = Vec::new();
        for o in &self.historial {
            if !plazas.contains(&o.asentamiento) {
                plazas.push(o.asentamiento);
            }
        }
        for p in plazas {
            let n = self.historial.iter().filter(|o| o.asentamiento == p).count();
            if n > r.historial_por_plaza {
                let mut sobran = n - r.historial_por_plaza;
                self.historial.retain(|o| {
                    if o.asentamiento == p && sobran > 0 {
                        sobran -= 1;
                        false
                    } else {
                        true
                    }
                });
            }
        }
    }

    /// Busca una orden en pie o, si no, en el historial (para dar el error correcto en el mostrador).
    pub fn buscar(&self, id: OrdenId) -> Option<&OrdenMercado> {
        self.orden(id).or_else(|| self.historial.iter().find(|o| o.id == id))
    }

    /// Las órdenes en pie de una plaza que se pueden tomar en `ahora` (el escaparate que ve quien está en su puerta).
    pub fn escaparate(&self, plaza: AsentamientoId, ahora: Instante) -> Vec<&OrdenMercado> {
        self.ordenes.iter().filter(|o| o.asentamiento == plaza && o.vigente(ahora)).collect()
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::Reglas;

    fn r() -> ReglasMercado {
        Reglas::cargar().mercado
    }

    fn orden(id: OrdenId, plaza: u32, t: Instante) -> OrdenMercado {
        OrdenMercado::colocar(&r(), id, plaza, TipoOrden::Compra, Recurso::Madera, 10_000, Some(1_000_000), 0, t, true, true, 0).unwrap()
    }

    #[test]
    fn colocar_valida_actor_mercado_cantidad_y_stock_de_la_venta() {
        let r = r();
        let c = |tipo, cant, stock, aut, mer| OrdenMercado::colocar(&r, 1, 5, tipo, Recurso::Madera, cant, None, 1_000_000, 0, aut, mer, stock);
        assert_eq!(c(TipoOrden::Compra, 10_000, 0, false, true).unwrap_err(), ErrorOrden::NoAutorizado);
        assert_eq!(c(TipoOrden::Compra, 10_000, 0, true, false).unwrap_err(), ErrorOrden::SinMercado);
        assert_eq!(c(TipoOrden::Compra, 0, 0, true, true).unwrap_err(), ErrorOrden::CantidadNoPositiva);
        assert_eq!(c(TipoOrden::Compra, 999, 0, true, true).unwrap_err(), ErrorOrden::CantidadNoPositiva, "menos de una unidad entera es nada (C3b)");
        assert_eq!(c(TipoOrden::Venta, 10_000, 9_000, true, true).unwrap_err(), ErrorOrden::SinStock);
        assert!(c(TipoOrden::Venta, 10_000, 10_000, true, true).is_ok());
        assert_eq!(c(TipoOrden::Compra, 10_500, 0, true, true).unwrap().cantidad, 10_000, "la fracción se redondea hacia abajo");
        assert!(c(TipoOrden::Compra, 10_000, 0, true, true).is_ok(), "una compra no exige oro ni reserva nada");
    }

    #[test]
    fn el_precio_por_defecto_es_el_de_referencia_y_no_se_valida_el_dado() {
        let r = r();
        let o = OrdenMercado::colocar(&r, 1, 5, TipoOrden::Compra, Recurso::Piedra, 10_000, None, 1_200_000, 0, true, true, 0).unwrap();
        assert_eq!(o.precio_unitario, 1_200_000);
        let o = OrdenMercado::colocar(&r, 1, 5, TipoOrden::Compra, Recurso::Piedra, 10_000, Some(-3), 1_200_000, 0, true, true, 0).unwrap();
        assert_eq!(o.precio_unitario, -3, "un precio negativo se acepta y luego no se liquida");
        let o = OrdenMercado::colocar(&r, 1, 5, TipoOrden::Compra, Recurso::Oro, 10_000, None, 0, 0, true, true, 0).unwrap();
        assert_eq!(o.precio_unitario, 0, "el oro no tiene precio de referencia: se coloca igual");
    }

    #[test]
    fn caduca_a_los_200_minutos_exactos() {
        let r = r();
        let mut l = LibroOrdenes::default();
        l.colocar(orden(1, 5, 1_000));
        let vence = 1_000 + 200 * 60_000;
        assert_eq!(l.orden(1).unwrap().expira_en, vence);
        assert_eq!(l.proxima_caducidad(), Some(vence));
        assert!(l.caducar(&r, vence - 1).is_empty());
        assert!(l.orden(1).unwrap().vigente(vence - 1) && !l.orden(1).unwrap().vigente(vence));
        assert_eq!(l.caducar(&r, vence), vec![1]);
        assert!(l.ordenes.is_empty());
        assert_eq!(l.historial.len(), 1);
        assert_eq!(l.historial[0].estado, EstadoOrden::Expirada);
        assert_eq!(l.proxima_caducidad(), None);
        assert_eq!(l.buscar(1).unwrap().estado, EstadoOrden::Expirada, "se encuentra también en el historial");
    }

    #[test]
    fn las_cumplidas_salen_hacia_el_historial_sin_evento() {
        let r = r();
        let mut l = LibroOrdenes::default();
        l.colocar(orden(1, 5, 0));
        l.colocar(orden(2, 5, 0));
        l.orden_mut(1).unwrap().estado = EstadoOrden::Cumplida;
        assert!(l.caducar(&r, 10).is_empty(), "ninguna expira, y la cumplida no emite evento");
        assert_eq!(l.ordenes.iter().map(|o| o.id).collect::<Vec<_>>(), vec![2]);
        assert_eq!(l.historial.iter().map(|o| o.id).collect::<Vec<_>>(), vec![1]);
    }

    #[test]
    fn el_historial_recuerda_solo_las_ultimas_200_de_cada_plaza() {
        let r = r();
        let mut l = LibroOrdenes::default();
        for i in 0..250 {
            let mut o = orden(i, 1, 0);
            o.estado = EstadoOrden::Cumplida;
            l.colocar(o);
        }
        for i in 1000..1010 {
            let mut o = orden(i, 2, 0);
            o.estado = EstadoOrden::Cumplida;
            l.colocar(o);
        }
        l.archivar_cerradas(&r);
        let de = |p| l.historial.iter().filter(|o| o.asentamiento == p).map(|o| o.id).collect::<Vec<_>>();
        assert_eq!(de(1).len(), 200);
        assert_eq!((de(1)[0], de(1)[199]), (50, 249), "del más viejo al más nuevo; se pierden las 50 primeras");
        assert_eq!(de(2).len(), 10, "la otra plaza no se toca");
    }

    #[test]
    fn el_escaparate_solo_ensena_lo_vigente_de_esa_plaza() {
        let r = r();
        let mut l = LibroOrdenes::default();
        l.colocar(orden(1, 5, 0));
        l.colocar(orden(2, 6, 0));
        l.colocar(orden(3, 5, 0));
        l.orden_mut(3).unwrap().estado = EstadoOrden::Cumplida;
        assert_eq!(l.escaparate(5, 10).iter().map(|o| o.id).collect::<Vec<_>>(), vec![1]);
        assert!(l.escaparate(5, 200 * 60_000).is_empty());
        let _ = r;
    }
}
