//! El encargo de una caravana comercial (rutas de caravanas automáticas, 2026-10-31; `Comercio_Caravanas.md` §5.8): un transporte con dos paradas, el origen
//! de la caravana y un destino, y en cada una qué deja y qué recoge. Una **ruta** va y viene entre dos plazas de la Facción hasta que se cancela; un **envío** hace
//! una sola ida y vuelta y puede ir a una plaza ajena, donde comercia con las órdenes de su mercado.
//!
//! Aquí están los datos del encargo y su validación; la caravana lo guarda en [`crate::caravana::Caravana::encargo`] y el núcleo programa sus pasos.

use crate::tipos::AsentamientoId;
use asentamiento::Recurso;
use base::{Instante, Mil, UNIDAD};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modo {
    /// Va y viene entre dos plazas de la Facción sin parar.
    Ruta,
    /// Una sola ida y vuelta, a la hora elegida; puede ir a una plaza ajena.
    Envio,
}

/// Una de las dos puntas del encargo: donde espera la caravana, o hacia donde va.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Punta {
    Origen,
    Destino,
}

/// Qué hace la caravana mientras está `Esperando` en una punta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fase {
    /// Espera su hora (el envío programado) o las existencias que pidió.
    Espera,
    /// Deja (o vende, en una plaza ajena) y espera `tiempoDescarga` por carro.
    Descarga,
    /// Recoge (o compra, en una plaza ajena) y espera `tiempoCarga` por carro.
    Carga,
}

/// Recoger `cantidad` de un recurso. En una plaza ajena es comprar a sus órdenes de venta con precio ≤ `precio_max`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineaRecoger {
    pub recurso: Recurso,
    pub cantidad: Mil,
    pub precio_max: Option<i64>,
}

/// Dejar un recurso, hasta `tope` si lo hay (sin tope, todo lo que lleve). En una plaza ajena es venderle a sus órdenes de compra con precio ≥ `precio_min`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineaDejar {
    pub recurso: Recurso,
    pub tope: Option<Mil>,
    pub precio_min: Option<i64>,
}

/// Lo que la caravana hace en una punta: primero deja, luego recoge.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Parada {
    pub recoger: Vec<LineaRecoger>,
    /// No carga hasta que el almacén tiene todo lo pedido (sin plazo); si no, sale con lo que haya.
    pub esperar_existencias: bool,
    pub dejar: Vec<LineaDejar>,
    /// Lo que no cabe en el almacén se tira; si no, se queda a bordo. En una plaza ajena no cuenta.
    pub desechar_sobrante: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encargo {
    pub modo: Modo,
    pub destino: AsentamientoId,
    /// Solo el envío: la hora a la que empieza a cargar en el origen.
    pub salir_en: Option<Instante>,
    pub en_origen: Parada,
    pub en_destino: Parada,
    /// Idas y vueltas hechas.
    pub vueltas: u32,
    pub punta: Punta,
    pub fase: Fase,
    /// Cuándo acaba la descarga o la carga en curso.
    pub fin_paso: Option<Instante>,
    /// Cancelado en ruta: acaba el tramo sin comerciar, vuelve y deja todo en el origen.
    pub cancelado: bool,
}

/// Lo que se rechaza al dar o cambiar un encargo, en el orden en que se comprueba.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorEncargo {
    /// El destino es el origen.
    MismoOrigen,
    /// Una ruta a una plaza de otra Facción.
    DestinoAjeno,
    /// Una ruta con hora de salida, o un envío con la hora fuera de `(ahora, ahora + maxProgramacion]`.
    SalidaInvalida,
    /// Cantidad o tope de menos de una unidad, recurso repetido en una lista, oro en una parada de plaza ajena o precio en una parada propia.
    LineaInvalida,
    /// Una línea sin precio (o con precio ≤ 0) en la parada de una plaza ajena.
    FaltaPrecio,
    /// Lo que se recoge en una parada pasa de la capacidad de la caravana.
    SuperaCapacidad,
}

/// Lo que hace falta saber para validar un encargo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextoEncargo {
    pub origen: AsentamientoId,
    /// El destino es de la Facción del origen.
    pub destino_propio: bool,
    /// `capacidadEfectiva` de la caravana (con «Carga Ampliada» si la tiene).
    pub capacidad: Mil,
    pub ahora: Instante,
    /// `maxProgramacionDias` en milisegundos.
    pub max_programacion: Instante,
}

impl Encargo {
    /// Un encargo recién dado: sin vueltas, esperando en el origen.
    pub fn nuevo(modo: Modo, destino: AsentamientoId, salir_en: Option<Instante>, en_origen: Parada, en_destino: Parada) -> Self {
        Self { modo, destino, salir_en, en_origen, en_destino, vueltas: 0, punta: Punta::Origen, fase: Fase::Espera, fin_paso: None, cancelado: false }
    }

    /// Valida el encargo (rutas de caravanas §2 y §7), en el orden de [`ErrorEncargo`]. El origen es siempre propio: sus líneas no llevan precio.
    pub fn validar(&self, c: &ContextoEncargo) -> Result<(), ErrorEncargo> {
        if self.destino == c.origen {
            return Err(ErrorEncargo::MismoOrigen);
        }
        if self.modo == Modo::Ruta && !c.destino_propio {
            return Err(ErrorEncargo::DestinoAjeno);
        }
        let salida_valida = match (self.modo, self.salir_en) {
            (_, None) => true,
            (Modo::Ruta, Some(_)) => false,
            (Modo::Envio, Some(s)) => s > c.ahora && s <= c.ahora + c.max_programacion,
        };
        if !salida_valida {
            return Err(ErrorEncargo::SalidaInvalida);
        }
        if !lineas_validas(&self.en_origen, true) || !lineas_validas(&self.en_destino, c.destino_propio) {
            return Err(ErrorEncargo::LineaInvalida);
        }
        if !c.destino_propio && !con_precios(&self.en_destino) {
            return Err(ErrorEncargo::FaltaPrecio);
        }
        if [&self.en_origen, &self.en_destino].iter().any(|p| p.recoger.iter().map(|l| l.cantidad).sum::<Mil>() > c.capacidad) {
            return Err(ErrorEncargo::SuperaCapacidad);
        }
        Ok(())
    }
}

/// Cantidades y topes de al menos una unidad, sin recursos repetidos en una lista; en una parada propia sin precios, en una ajena sin oro.
fn lineas_validas(p: &Parada, propia: bool) -> bool {
    let recoger = p.recoger.iter().map(|l| (l.recurso, Some(l.cantidad), l.precio_max));
    let dejar = p.dejar.iter().map(|l| (l.recurso, l.tope, l.precio_min));
    let bien = |(recurso, cantidad, precio): (Recurso, Option<Mil>, Option<i64>)| {
        cantidad.is_none_or(|n| n >= UNIDAD) && if propia { precio.is_none() } else { recurso != Recurso::Oro }
    };
    let sin_repetir = |rs: Vec<Recurso>| rs.iter().enumerate().all(|(i, r)| !rs[..i].contains(r));
    recoger.clone().all(bien) && dejar.clone().all(bien) && sin_repetir(recoger.map(|l| l.0).collect()) && sin_repetir(dejar.map(|l| l.0).collect())
}

/// Toda línea de la parada lleva un precio positivo.
fn con_precios(p: &Parada) -> bool {
    p.recoger.iter().map(|l| l.precio_max).chain(p.dejar.iter().map(|l| l.precio_min)).all(|x| x.is_some_and(|x| x > 0))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    const ORIGEN: AsentamientoId = 1;
    const DESTINO: AsentamientoId = 2;
    const DIA: Instante = 24 * 60 * 60_000;

    fn ctx(destino_propio: bool) -> ContextoEncargo {
        ContextoEncargo { origen: ORIGEN, destino_propio, capacidad: 500_000, ahora: 1_000, max_programacion: 3 * DIA }
    }

    fn recoger(recurso: Recurso, cantidad: Mil, precio_max: Option<i64>) -> LineaRecoger {
        LineaRecoger { recurso, cantidad, precio_max }
    }

    fn dejar(recurso: Recurso, tope: Option<Mil>, precio_min: Option<i64>) -> LineaDejar {
        LineaDejar { recurso, tope, precio_min }
    }

    fn parada(recoger: Vec<LineaRecoger>, dejar: Vec<LineaDejar>) -> Parada {
        Parada { recoger, dejar, ..Parada::default() }
    }

    fn ruta(en_origen: Parada, en_destino: Parada) -> Encargo {
        Encargo::nuevo(Modo::Ruta, DESTINO, None, en_origen, en_destino)
    }

    #[test]
    fn un_encargo_nuevo_espera_en_el_origen_sin_vueltas() {
        let e = ruta(Parada::default(), Parada::default());
        assert_eq!((e.vueltas, e.punta, e.fase, e.fin_paso, e.cancelado), (0, Punta::Origen, Fase::Espera, None, false));
    }

    #[test]
    fn una_ruta_entre_plazas_propias_es_valida() {
        let e = ruta(parada(vec![recoger(Recurso::Madera, 200_000, None)], vec![dejar(Recurso::Trigo, None, None)]), parada(vec![recoger(Recurso::Trigo, 500_000, None)], vec![dejar(Recurso::Madera, Some(50_000), None)]));
        assert_eq!(e.validar(&ctx(true)), Ok(()));
    }

    #[test]
    fn el_destino_no_puede_ser_el_origen() {
        let mut e = ruta(Parada::default(), Parada::default());
        e.destino = ORIGEN;
        assert_eq!(e.validar(&ctx(true)), Err(ErrorEncargo::MismoOrigen));
    }

    #[test]
    fn una_ruta_no_va_a_una_plaza_ajena_pero_un_envio_si() {
        let destino = parada(vec![recoger(Recurso::Trigo, 1_000, Some(2_000_000))], vec![]);
        assert_eq!(ruta(Parada::default(), destino.clone()).validar(&ctx(false)), Err(ErrorEncargo::DestinoAjeno));
        assert_eq!(Encargo::nuevo(Modo::Envio, DESTINO, None, Parada::default(), destino).validar(&ctx(false)), Ok(()));
    }

    #[test]
    fn la_hora_de_salida_solo_vale_en_un_envio_y_dentro_del_maximo() {
        let envio = |s| Encargo::nuevo(Modo::Envio, DESTINO, Some(s), Parada::default(), Parada::default());
        let c = ctx(true);
        assert_eq!(envio(c.ahora + 1).validar(&c), Ok(()));
        assert_eq!(envio(c.ahora + 3 * DIA).validar(&c), Ok(()), "justo en el máximo");
        assert_eq!(envio(c.ahora).validar(&c), Err(ErrorEncargo::SalidaInvalida), "no en el pasado ni ahora");
        assert_eq!(envio(c.ahora + 3 * DIA + 1).validar(&c), Err(ErrorEncargo::SalidaInvalida));
        let mut r = ruta(Parada::default(), Parada::default());
        r.salir_en = Some(c.ahora + 1);
        assert_eq!(r.validar(&c), Err(ErrorEncargo::SalidaInvalida), "la ruta sale en cuanto se le da");
    }

    #[test]
    fn las_lineas_invalidas_se_rechazan() {
        let c = ctx(true);
        let casos = [
            parada(vec![recoger(Recurso::Madera, 999, None)], vec![]),
            parada(vec![], vec![dejar(Recurso::Madera, Some(0), None)]),
            parada(vec![recoger(Recurso::Madera, 1_000, None), recoger(Recurso::Madera, 1_000, None)], vec![]),
            parada(vec![], vec![dejar(Recurso::Trigo, None, None), dejar(Recurso::Trigo, Some(5_000), None)]),
            parada(vec![recoger(Recurso::Madera, 1_000, Some(1))], vec![]),
            parada(vec![], vec![dejar(Recurso::Madera, None, Some(1))]),
        ];
        for p in casos {
            assert_eq!(ruta(p.clone(), Parada::default()).validar(&c), Err(ErrorEncargo::LineaInvalida), "en el origen: {p:?}");
            assert_eq!(ruta(Parada::default(), p.clone()).validar(&c), Err(ErrorEncargo::LineaInvalida), "en un destino propio: {p:?}");
        }
        let mismo_recurso_en_las_dos_listas = parada(vec![recoger(Recurso::Madera, 1_000, None)], vec![dejar(Recurso::Madera, None, None)]);
        assert_eq!(ruta(mismo_recurso_en_las_dos_listas, Parada::default()).validar(&c), Ok(()));
    }

    #[test]
    fn en_una_plaza_ajena_no_se_comercia_con_oro_y_toda_linea_lleva_precio() {
        let envio = |p| Encargo::nuevo(Modo::Envio, DESTINO, None, parada(vec![recoger(Recurso::Oro, 100_000, None)], vec![dejar(Recurso::Oro, None, None)]), p);
        let c = ctx(false);
        assert_eq!(envio(parada(vec![recoger(Recurso::Oro, 1_000, Some(1))], vec![])).validar(&c), Err(ErrorEncargo::LineaInvalida));
        assert_eq!(envio(parada(vec![], vec![dejar(Recurso::Oro, None, Some(1))])).validar(&c), Err(ErrorEncargo::LineaInvalida));
        assert_eq!(envio(parada(vec![recoger(Recurso::Trigo, 1_000, None)], vec![])).validar(&c), Err(ErrorEncargo::FaltaPrecio));
        assert_eq!(envio(parada(vec![], vec![dejar(Recurso::Madera, None, Some(0))])).validar(&c), Err(ErrorEncargo::FaltaPrecio));
        assert_eq!(envio(parada(vec![recoger(Recurso::Trigo, 1_000, Some(3_000_000))], vec![dejar(Recurso::Madera, Some(20_000), Some(2_000_000))])).validar(&c), Ok(()), "el oro se recoge y se deja en el origen");
    }

    #[test]
    fn lo_que_se_recoge_en_cada_parada_no_pasa_de_la_capacidad() {
        let c = ctx(true);
        let lleno = parada(vec![recoger(Recurso::Madera, 300_000, None), recoger(Recurso::Piedra, 200_000, None)], vec![]);
        assert_eq!(ruta(lleno.clone(), lleno.clone()).validar(&c), Ok(()), "justo la capacidad en cada una");
        let pasado = parada(vec![recoger(Recurso::Madera, 300_000, None), recoger(Recurso::Piedra, 201_000, None)], vec![]);
        assert_eq!(ruta(pasado.clone(), Parada::default()).validar(&c), Err(ErrorEncargo::SuperaCapacidad));
        assert_eq!(ruta(Parada::default(), pasado).validar(&c), Err(ErrorEncargo::SuperaCapacidad));
    }
}
