//! El encargo de una caravana comercial (rutas de caravanas automáticas, 2026-10-31; `Comercio_Caravanas.md` §5.8): un transporte con dos paradas, el origen
//! de la caravana y un destino, y en cada una qué deja y qué recoge. Una **ruta** va y viene entre dos plazas de la Facción hasta que se cancela; un **envío** hace
//! una sola ida y vuelta y puede ir a una plaza ajena, donde comercia con las órdenes de su mercado.
//!
//! Aquí están los datos del encargo y su validación; la caravana lo guarda en [`crate::caravana::Caravana::encargo`] y el núcleo programa sus pasos.

use crate::tipos::{de, sumar, total, AsentamientoId, Carga};
use asentamiento::{Almacen, Recurso};
use base::{entero, Instante, Mil, UNIDAD};

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
    /// Cuándo empezó la descarga o la carga en curso (2026-11-04, para la barra de tiempo del cliente): con `fin_paso`, el paso entero.
    pub inicio_paso: Option<Instante>,
    /// Ruta detenida (2026-11-04, «detener ruta»): acaba la ida y vuelta en curso comerciando como siempre y, al descargar en el origen, el encargo termina.
    pub detener: bool,
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
        Self { modo, destino, salir_en, en_origen, en_destino, vueltas: 0, punta: Punta::Origen, fase: Fase::Espera, fin_paso: None, cancelado: false, inicio_paso: None, detener: false }
    }

    /// Valida el encargo (Comercio_Caravanas §5.8.2 y §5.8.3), en el orden de [`ErrorEncargo`]. El origen es siempre propio: sus líneas no llevan precio.
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

/// Lo que una descarga en plaza propia mete en el almacén y lo que tira.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reparto {
    pub entra: Carga,
    pub tirado: Carga,
}

/// Qué entra al almacén al dejar (`Comercio_Caravanas.md` §5.8.5). Por línea, en su orden: se quiere dejar el `tope` (o todo lo que lleva
/// si no hay) sin pasar de lo que lleva; entra lo que cabe en el almacén, en unidades enteras; el resto se tira con `desechar` y, si no, sigue a bordo. No muta nada.
pub fn repartir_descarga(a_bordo: &Carga, dejar: &[LineaDejar], desechar: bool, almacen: &Almacen) -> Reparto {
    let mut reparto = Reparto::default();
    for l in dejar {
        let lleva = de(a_bordo, l.recurso);
        let quiere = entero(l.tope.map_or(lleva, |t| t.min(lleva)));
        let entra = entero(quiere.min(almacen.espacio(l.recurso)));
        sumar(&mut reparto.entra, l.recurso, entra);
        if desechar {
            sumar(&mut reparto.tirado, l.recurso, quiere - entra);
        }
    }
    reparto
}

/// Qué se toma del almacén al recoger (`Comercio_Caravanas.md` §5.8.5). Se pide la parte entera de cada cantidad. Con `esperar` y
/// algo que falta, `Err` con lo que falta de cada línea y no se toma nada. Si no, por línea, en su orden, se toma lo pedido, topado por lo que hay y por el hueco de la caravana
/// (`capacidad` menos lo que lleva y lo ya tomado). No muta nada.
pub fn a_recoger(a_bordo: &Carga, recoger: &[LineaRecoger], esperar: bool, capacidad: Mil, almacen: &Almacen) -> Result<Carga, Carga> {
    if esperar {
        let mut falta = Carga::new();
        for l in recoger {
            sumar(&mut falta, l.recurso, entero(l.cantidad) - almacen.cantidad(l.recurso));
        }
        if !falta.is_empty() {
            return Err(falta);
        }
    }
    let mut hueco = (capacidad - total(a_bordo)).max(0);
    let mut toma = Carga::new();
    for l in recoger {
        let n = entero(entero(l.cantidad).min(almacen.cantidad(l.recurso)).min(hueco));
        sumar(&mut toma, l.recurso, n);
        hueco -= n;
    }
    Ok(toma)
}

/// Carros que cuentan para el tiempo de un paso (`Comercio_Caravanas.md` §5.8.5): los que harían falta para mover `movido`, entre 1 y los carros con
/// animal; 0 si no se mueve nada o la capacidad es nula.
pub fn carros_del_paso(movido: Mil, capacidad: Mil, carros_con_animal: usize) -> u32 {
    if movido <= 0 || capacidad <= 0 {
        return 0;
    }
    let carros = carros_con_animal as i128;
    let necesarios = (movido as i128 * carros + capacidad as i128 - 1) / capacidad as i128;
    necesarios.clamp(1, carros) as u32
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

    fn carga(l: &[(Recurso, Mil)]) -> Carga {
        l.iter().copied().collect()
    }

    fn almacen(capacidad: Mil, l: &[(Recurso, Mil)]) -> Almacen {
        let mut a = Almacen::nuevo(capacidad);
        for &(r, n) in l {
            a.poner(r, n);
        }
        a
    }

    #[test]
    fn dejar_con_tope_deja_el_tope_y_sin_tope_todo_lo_que_lleva() {
        let a_bordo = carga(&[(Recurso::Madera, 80_000), (Recurso::Piedra, 30_000)]);
        let lineas = [dejar(Recurso::Madera, Some(50_000), None), dejar(Recurso::Piedra, None, None)];
        let r = repartir_descarga(&a_bordo, &lineas, false, &almacen(1_000_000, &[]));
        assert_eq!(r, Reparto { entra: carga(&[(Recurso::Madera, 50_000), (Recurso::Piedra, 30_000)]), tirado: Carga::new() });
    }

    #[test]
    fn dejar_un_tope_mayor_que_lo_que_lleva_deja_solo_lo_que_lleva() {
        let a_bordo = carga(&[(Recurso::Madera, 20_000)]);
        let r = repartir_descarga(&a_bordo, &[dejar(Recurso::Madera, Some(50_000), None)], true, &almacen(1_000_000, &[]));
        assert_eq!(r, Reparto { entra: carga(&[(Recurso::Madera, 20_000)]), tirado: Carga::new() });
    }

    #[test]
    fn lo_que_no_cabe_se_tira_con_desechar_y_se_queda_a_bordo_sin_el() {
        let a_bordo = carga(&[(Recurso::Madera, 80_000)]);
        let lleno = almacen(100_000, &[(Recurso::Madera, 70_000)]);
        let lineas = [dejar(Recurso::Madera, None, None)];
        let con = repartir_descarga(&a_bordo, &lineas, true, &lleno);
        assert_eq!(con, Reparto { entra: carga(&[(Recurso::Madera, 30_000)]), tirado: carga(&[(Recurso::Madera, 50_000)]) });
        let sin = repartir_descarga(&a_bordo, &lineas, false, &lleno);
        assert_eq!(sin, Reparto { entra: carga(&[(Recurso::Madera, 30_000)]), tirado: Carga::new() });
    }

    #[test]
    fn dejar_un_recurso_que_no_lleva_no_hace_nada() {
        let a_bordo = carga(&[(Recurso::Madera, 10_000)]);
        let r = repartir_descarga(&a_bordo, &[dejar(Recurso::Piedra, None, None), dejar(Recurso::Trigo, Some(5_000), None)], true, &almacen(1_000_000, &[]));
        assert_eq!(r, Reparto::default());
    }

    #[test]
    fn dejar_en_un_almacen_sin_espacio_no_mete_nada() {
        let a_bordo = carga(&[(Recurso::Madera, 10_000)]);
        let r = repartir_descarga(&a_bordo, &[dejar(Recurso::Madera, None, None)], false, &almacen(50_000, &[(Recurso::Madera, 50_000)]));
        assert_eq!(r, Reparto::default());
    }

    #[test]
    fn recoger_con_todo_toma_lo_pedido() {
        let alm = almacen(1_000_000, &[(Recurso::Madera, 300_000), (Recurso::Piedra, 300_000)]);
        let lineas = [recoger(Recurso::Madera, 100_000, None), recoger(Recurso::Piedra, 50_000, None)];
        assert_eq!(a_recoger(&Carga::new(), &lineas, false, 500_000, &alm), Ok(carga(&[(Recurso::Madera, 100_000), (Recurso::Piedra, 50_000)])));
        assert_eq!(a_recoger(&Carga::new(), &lineas, true, 500_000, &alm), Ok(carga(&[(Recurso::Madera, 100_000), (Recurso::Piedra, 50_000)])));
    }

    #[test]
    fn recoger_con_el_almacen_corto_sin_esperar_toma_lo_que_hay() {
        let alm = almacen(1_000_000, &[(Recurso::Madera, 30_000)]);
        let lineas = [recoger(Recurso::Madera, 100_000, None), recoger(Recurso::Piedra, 50_000, None)];
        assert_eq!(a_recoger(&Carga::new(), &lineas, false, 500_000, &alm), Ok(carga(&[(Recurso::Madera, 30_000)])));
    }

    #[test]
    fn recoger_con_el_almacen_corto_esperando_da_lo_que_falta_y_no_toma_nada() {
        let alm = almacen(1_000_000, &[(Recurso::Madera, 30_000), (Recurso::Trigo, 90_000)]);
        let lineas = [recoger(Recurso::Madera, 100_000, None), recoger(Recurso::Piedra, 50_000, None), recoger(Recurso::Trigo, 90_000, None)];
        assert_eq!(a_recoger(&Carga::new(), &lineas, true, 500_000, &alm), Err(carga(&[(Recurso::Madera, 70_000), (Recurso::Piedra, 50_000)])));
    }

    #[test]
    fn recoger_se_topa_por_el_hueco_con_carga_previa_a_bordo() {
        let alm = almacen(1_000_000, &[(Recurso::Madera, 300_000), (Recurso::Piedra, 300_000)]);
        let a_bordo = carga(&[(Recurso::Trigo, 120_000)]);
        let lineas = [recoger(Recurso::Madera, 100_000, None), recoger(Recurso::Piedra, 100_000, None)];
        assert_eq!(a_recoger(&a_bordo, &lineas, false, 200_000, &alm), Ok(carga(&[(Recurso::Madera, 80_000)])));
    }

    #[test]
    fn recoger_con_la_caravana_llena_no_toma_nada() {
        let alm = almacen(1_000_000, &[(Recurso::Madera, 300_000)]);
        let a_bordo = carga(&[(Recurso::Trigo, 250_000)]);
        assert_eq!(a_recoger(&a_bordo, &[recoger(Recurso::Madera, 100_000, None)], false, 200_000, &alm), Ok(Carga::new()));
    }

    #[test]
    fn recoger_y_dejar_trabajan_en_unidades_enteras() {
        let alm = almacen(1_000_000, &[(Recurso::Madera, 30_900), (Recurso::Piedra, 50_000)]);
        let lineas = [recoger(Recurso::Madera, 100_000, None), recoger(Recurso::Piedra, 20_700, None)];
        assert_eq!(a_recoger(&Carga::new(), &lineas, false, 500_000, &alm), Ok(carga(&[(Recurso::Madera, 30_000), (Recurso::Piedra, 20_000)])));
        let a_bordo = carga(&[(Recurso::Madera, 10_900)]);
        let r = repartir_descarga(&a_bordo, &[dejar(Recurso::Madera, None, None)], true, &almacen(1_000_000, &[]));
        assert_eq!(r.entra, carga(&[(Recurso::Madera, 10_000)]));
    }

    #[test]
    fn los_carros_del_paso_son_cero_si_no_se_mueve_nada() {
        assert_eq!(carros_del_paso(0, 500_000, 4), 0);
        assert_eq!(carros_del_paso(-1_000, 500_000, 4), 0);
        assert_eq!(carros_del_paso(100_000, 0, 4), 0);
    }

    #[test]
    fn los_carros_del_paso_se_redondean_hacia_arriba_y_se_topan() {
        assert_eq!(carros_del_paso(1_000, 400_000, 4), 1, "poco: un carro");
        assert_eq!(carros_del_paso(200_000, 400_000, 4), 2, "la mitad");
        assert_eq!(carros_del_paso(400_000, 400_000, 4), 4, "todo");
        assert_eq!(carros_del_paso(900_000, 400_000, 4), 4, "más que la capacidad: topado a los carros");
    }
}
