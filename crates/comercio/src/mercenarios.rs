//! El mercado de un campamento de mercenarios (Comercio_Caravanas §9): un vendedor neutral que solo **vende**, a `margen` sobre el precio
//! de referencia, con stock limitado que se repone con el pulso del comercio real del mundo. **El oro cobrado se destruye** (sumidero).
//! Las compras son en unidades **enteras**; el stock y el almacén personal los guarda quien llama, aquí está la aritmética.

use crate::datos::{Reglas, ReglasMercenarios};
use crate::tipos::Carga;
use asentamiento::datos::{por, UNO};
use asentamiento::Recurso;
use base::{Instante, Mil};
use std::collections::BTreeMap;

/// Milésimas por unidad entera.
const U: Mil = 1_000;
/// Milisegundos de un día de mundo (el día del cupo es `floor(instante / 86 400 000)`).
pub const MS_DIA: i64 = 86_400_000;

/// Los bienes en venta: los que tienen `PRECIO_BASE` (el oro no, y tampoco los manufacturados, cuyo precio sale de su receta).
pub fn bienes_en_venta(r: &Reglas) -> Vec<Recurso> {
    r.precio.base.keys().copied().collect()
}

/// Precio unitario (ppm) de un bien: `precioReferencia × margen`. Un bien sin precio no se vende.
pub fn precio_unitario(r: &ReglasMercenarios, precio_referencia: i64) -> Option<i64> {
    (precio_referencia > 0).then(|| por(precio_referencia, r.margen))
}

/// El stock en venta de un campamento.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StockCampamento {
    pub stock: BTreeMap<Recurso, Mil>,
}

impl StockCampamento {
    /// Un campamento nuevo: `stockInicial` de cada bien en venta y, sobre eso, sus pilas (que sustituyen al stock inicial de esos bienes).
    pub fn nuevo(r: &Reglas) -> Self {
        let m = &r.mercenarios;
        let mut stock: BTreeMap<Recurso, Mil> = bienes_en_venta(r).into_iter().map(|b| (b, m.stock_inicial)).collect();
        stock.extend(m.pilas.iter().map(|(&b, &n)| (b, n)));
        Self { stock }
    }

    pub fn de(&self, r: Recurso) -> Mil {
        self.stock.get(&r).copied().unwrap_or(0)
    }
}

/// Lo comerciado en el mundo desde la última reposición, y cuándo toca la siguiente. **Global**, no de un campamento.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MercadoMercenario {
    /// Lo comerciado entre Facciones desde la última reposición, por bien (lo que se enseña).
    pub contadores: BTreeMap<Recurso, Mil>,
    /// Lo que repondrá cada bien: por cada comercio, `proporcionRepone` de lo comerciado en unidades enteras hacia abajo y **como mínimo una**
    /// (EE-5, 2026-10-24).
    pub repone: BTreeMap<Recurso, Mil>,
    pub repone_en: Instante,
}

impl MercadoMercenario {
    /// El primer `reponeEn` es `creación + cadaHoras` horas.
    pub fn nuevo(r: &ReglasMercenarios, creacion: Instante) -> Self {
        Self { contadores: BTreeMap::new(), repone: BTreeMap::new(), repone_en: creacion + r.cada }
    }

    /// Suma lo intercambiado a los contadores. Solo cuenta el comercio **entre Facciones**: el `intercambiado` de cada
    /// `comercio.trueque_cumplido` con `entreFacciones` y la `cantidad` de cada `mercado.comercio_en_plaza` con `entreFacciones`; lo que
    /// una Facción comercia consigo misma no cuenta. Cada comercio anota además lo que repondrá ese bien: `proporcionRepone` de lo comerciado, en
    /// unidades enteras hacia abajo y como mínimo una (EE-5).
    pub fn registrar_comercio(&mut self, r: &ReglasMercenarios, entre_facciones: bool, carga: &Carga) {
        if !entre_facciones {
            return;
        }
        for (&b, &n) in carga {
            if n > 0 {
                *self.contadores.entry(b).or_insert(0) += n;
                *self.repone.entry(b).or_insert(0) += base::entero(por(n, r.proporcion_repone)).max(U);
            }
        }
    }

    /// `reponerMercados` (evento agendado a `repone_en`): para cada campamento, las pilas vuelven a su tope y cada bien comerciado que no sea
    /// una pila gana lo anotado en `repone` (por cada comercio, `proporcionRepone` de lo comerciado, como mínimo una unidad), topado por
    /// `topePorBien`. Después los contadores vuelven a `{}` y `repone_en = ahora + cadaHoras`. Devuelve `false` si aún no toca.
    pub fn reponer(&mut self, r: &ReglasMercenarios, ahora: Instante, campamentos: &mut [StockCampamento]) -> bool {
        if ahora < self.repone_en {
            return false;
        }
        for c in campamentos.iter_mut() {
            for (&b, &tope) in &r.pilas {
                c.stock.insert(b, tope);
            }
            for (&b, &sube) in self.repone.iter().filter(|(b, &n)| n > 0 && !r.pilas.contains_key(b)) {
                let nuevo = base::entero((c.de(b) + sube).min(r.tope_por_bien)); // existencias en unidades enteras (C3b)
                c.stock.insert(b, nuevo);
            }
        }
        self.contadores.clear();
        self.repone.clear();
        self.repone_en = ahora + r.cada;
        true
    }
}

/// El cupo diario de un héroe en un campamento: lo comprado hoy de cada bien con tope (madera y piedra), un contador por recurso
/// (Comercio_Caravanas §9.2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CupoDiario {
    pub dia: i64,
    pub comprado: BTreeMap<Recurso, Mil>,
}

pub fn dia_de(instante: Instante) -> i64 {
    instante.div_euclid(MS_DIA)
}

impl CupoDiario {
    /// Lo que le queda hoy de `recurso`: `None` si ese bien no tiene cupo diario.
    pub fn restante(&self, r: &ReglasMercenarios, recurso: Recurso, ahora: Instante) -> Option<Mil> {
        let tope = *r.cupo_diario.get(&recurso)?;
        let hecho = if self.dia == dia_de(ahora) { self.comprado.get(&recurso).copied().unwrap_or(0) } else { 0 };
        Some((tope - hecho).max(0))
    }

    /// Anota una compra de hoy (si cambió el día, el contador empieza de cero).
    pub fn anotar(&mut self, recurso: Recurso, cantidad: Mil, ahora: Instante) {
        let hoy = dia_de(ahora);
        if self.dia != hoy {
            *self = CupoDiario { dia: hoy, comprado: BTreeMap::new() };
        }
        *self.comprado.entry(recurso).or_insert(0) += cantidad;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorMercenarios {
    /// Pedir menos de 1 unidad, o pedir oro.
    CantidadInvalida,
    /// El bien no está en venta (sin precio de referencia, como el oro).
    SinPrecio,
    /// El trigo no se vende al carro: se reposta gratis a víveres.
    TrigoNoVaAlCarro,
    SinStock,
    CupoAgotado,
    SinOro,
    /// El almacén personal (o el carro) no tiene sitio.
    SinSitio,
    /// Los víveres ya están llenos.
    VivereLlenos,
}

/// Una compra servida: cuánto se lleva el comprador y cuánto oro se cobra (se destruye).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Compra {
    pub cantidad: Mil,
    pub oro_cobrado: Mil,
}

/// `coste(n) = ceil(n × precio)` en unidades **enteras** de oro (`n` en unidades).
fn coste(n: i64, precio: i64) -> i64 {
    ((n as i128 * precio as i128 + UNO as i128 - 1) / UNO as i128) as i64
}

/// Resuelve las reglas de cantidad entera: baja `q` mientras `falla(q)`.
fn bajar(mut q: i64, falla: impl Fn(i64) -> bool) -> i64 {
    while q > 0 && falla(q) {
        q -= 1;
    }
    q
}

fn comprobar(recurso: Recurso, pedida: Mil, precio: Option<i64>) -> Result<i64, ErrorMercenarios> {
    if recurso == Recurso::Oro || pedida < U {
        return Err(ErrorMercenarios::CantidadInvalida);
    }
    precio.filter(|&p| p > 0).ok_or(ErrorMercenarios::SinPrecio)
}

/// `comprarEnCampamento` de un héroe que **reside** en él: compra cualquier bien en venta; paga del oro de su almacén personal
/// (`guardado`) y lo comprado va a ese almacén; con cupo diario por bien (`cupo_restante`, `None` = sin cupo). `sitio` es lo que queda en
/// el almacén personal (`ALMACEN_PERSONAL.capacidad − total`): en el almacén cada unidad ocupa 1 y libera lo que cuesta en oro.
///
/// ```text
/// cantidad = min(floor(pedida), floor(stock), cupo restante hoy, floor(guardado / precio))
/// mientras cantidad − coste(cantidad) > sitio : cantidad−−
/// mientras coste(cantidad) > guardado         : cantidad−−
/// ```
#[allow(clippy::too_many_arguments)]
pub fn comprar_residente(recurso: Recurso, precio: Option<i64>, pedida: Mil, stock: Mil, cupo_restante: Option<Mil>, guardado_oro: Mil, sitio: Mil) -> Result<Compra, ErrorMercenarios> {
    let precio = comprobar(recurso, pedida, precio)?;
    let (stock_u, cupo_u, guardado_u, sitio_u) = (stock.max(0) / U, cupo_restante.map_or(i64::MAX, |c| c.max(0) / U), guardado_oro.max(0) / U, sitio.div_euclid(U));
    let por_oro = (guardado_u as i128 * UNO as i128 / precio as i128) as i64;
    let q = (pedida / U).min(stock_u).min(cupo_u).min(por_oro);
    let q = bajar(q, |n| n - coste(n, precio) > sitio_u);
    let q = bajar(q, |n| coste(n, precio) > guardado_u);
    if q <= 0 {
        return Err(if stock_u == 0 {
            ErrorMercenarios::SinStock
        } else if cupo_u == 0 {
            ErrorMercenarios::CupoAgotado
        } else if por_oro == 0 {
            ErrorMercenarios::SinOro
        } else {
            ErrorMercenarios::SinSitio
        });
    }
    Ok(Compra { cantidad: q * U, oro_cobrado: coste(q, precio) * U })
}

/// `comprarEnCampamento` de un héroe que **no reside** (de visita, desde la puerta): compra cualquier bien en venta **salvo el trigo**,
/// al mismo precio, pagando con el oro de su carro (`oro_carro`) y llevándoselo al carro de su columna; **sin cupo diario**, pero compite por
/// el mismo stock. `libre` es la capacidad de carga de la columna menos su suministro.
///
/// ```text
/// cantidad = min(floor(pedida), floor(stock), floor(oroCarro / precio))
/// mientras cantidad − coste(cantidad) > libre : cantidad−−      ; el carro solo crece si el precio es < 1
/// mientras coste(cantidad) > oroCarro         : cantidad−−
/// ```
pub fn comprar_visitante(recurso: Recurso, precio: Option<i64>, pedida: Mil, stock: Mil, oro_carro: Mil, libre: Mil) -> Result<Compra, ErrorMercenarios> {
    if recurso == Recurso::Trigo {
        return Err(ErrorMercenarios::TrigoNoVaAlCarro);
    }
    let precio = comprobar(recurso, pedida, precio)?;
    let (stock_u, oro_u, libre_u) = (stock.max(0) / U, oro_carro.max(0) / U, libre.div_euclid(U));
    let por_oro = (oro_u as i128 * UNO as i128 / precio as i128) as i64;
    let q = (pedida / U).min(stock_u).min(por_oro);
    let q = bajar(q, |n| n - coste(n, precio) > libre_u);
    let q = bajar(q, |n| coste(n, precio) > oro_u);
    if q <= 0 {
        return Err(if stock_u == 0 {
            ErrorMercenarios::SinStock
        } else if por_oro == 0 {
            ErrorMercenarios::SinOro
        } else {
            ErrorMercenarios::SinSitio
        });
    }
    Ok(Compra { cantidad: q * U, oro_cobrado: coste(q, precio) * U })
}

/// `repostarEnCampamento`: cualquier héroe, residente o no, llena sus víveres hasta `capacidadViveresPorHeroe`, **gratis** y sin gastar
/// el stock; falla si ya los tiene llenos. Devuelve la cantidad cargada. El trigo no pasa por el carro ni por el almacén personal.
pub fn repostar(r: &ReglasMercenarios, viveres: Mil) -> Result<Mil, ErrorMercenarios> {
    let falta = r.capacidad_viveres_por_heroe - viveres.max(0);
    if falta <= 0 {
        Err(ErrorMercenarios::VivereLlenos)
    } else {
        Ok(falta)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::Reglas;
    use Recurso::*;

    fn reglas() -> Reglas {
        Reglas::cargar()
    }

    #[test]
    fn el_campamento_nace_con_stock_inicial_y_pilas() {
        let r = reglas();
        let c = StockCampamento::nuevo(&r);
        assert_eq!((c.de(Cobre), c.de(Estano), c.de(Hierro), c.de(Livestock)), (100_000, 100_000, 100_000, 100_000));
        assert_eq!((c.de(Madera), c.de(Piedra), c.de(Trigo)), (300_000, 60_000, 300_000), "las pilas sustituyen al stock inicial");
        assert_eq!(c.de(Oro), 0, "el oro no está en venta");
        assert_eq!(bienes_en_venta(&r).len(), 7);
        assert!(!bienes_en_venta(&r).contains(&Recurso::ArmaMadera), "los manufacturados cotizan (comisión) pero los mercenarios no los venden");
    }

    #[test]
    fn precio_con_margen_30_por_ciento_y_sin_precio_no_se_vende() {
        let r = reglas().mercenarios;
        assert_eq!(precio_unitario(&r, 3_000_000), Some(3_900_000));
        assert_eq!(precio_unitario(&r, 0), None);
    }

    #[test]
    fn solo_el_comercio_entre_facciones_cuenta() {
        let r = reglas().mercenarios;
        let mut m = MercadoMercenario::nuevo(&r, 1_000);
        assert_eq!(m.repone_en, 1_000 + 3 * 3_600_000);
        let c: Carga = [(Cobre, 50_000)].into_iter().collect();
        m.registrar_comercio(&r, false, &c);
        assert!(m.contadores.is_empty() && m.repone.is_empty(), "una Facción comerciando consigo misma no cuenta");
        m.registrar_comercio(&r, true, &c);
        m.registrar_comercio(&r, true, &c);
        assert_eq!(m.contadores[&Cobre], 100_000);
        assert_eq!(m.repone[&Cobre], 2_000, "el 3 % de 50 es 1,5: una unidad por comercio, hacia abajo y como mínimo una (EE-5)");
        m.registrar_comercio(&r, true, &[(Estano, 10_000)].into_iter().collect());
        assert_eq!(m.repone[&Estano], 1_000, "el 3 % de 10 no llega a una unidad: repone el mínimo, una");
    }

    #[test]
    fn reponer_cada_tres_horas_con_el_tres_por_ciento_tope_y_pilas() {
        let r = reglas();
        let mut m = MercadoMercenario::nuevo(&r.mercenarios, 0);
        let mut camps = vec![StockCampamento::nuevo(&r), StockCampamento::nuevo(&r)];
        camps[0].stock.insert(Madera, 10_000); // la pila se había gastado
        camps[1].stock.insert(Cobre, 0);
        m.registrar_comercio(&r.mercenarios, true, &[(Cobre, 1_000_000), (Madera, 500_000), (Hierro, 20_000_000)].into_iter().collect());
        // se comerciaron 1000 de cobre; la madera es pila: su contador no suma; el hierro, enorme: topa en 300
        let t = 3 * 3_600_000;
        assert!(!m.reponer(&r.mercenarios, t - 1, &mut camps), "aún no toca");
        assert!(m.reponer(&r.mercenarios, t + 5, &mut camps));
        assert_eq!(camps[0].de(Madera), 300_000, "la pila vuelve a su tope sin depender del comercio");
        assert_eq!(camps[0].de(Cobre), 100_000 + 30_000, "3 % de 1000 = 30");
        assert_eq!(camps[1].de(Cobre), 30_000);
        assert_eq!(camps[0].de(Hierro), 300_000, "topado por topePorBien");
        assert_eq!(camps[0].de(Estano), 100_000, "un bien que nadie comercia no se repone");
        assert!(m.contadores.is_empty() && m.repone.is_empty());
        assert_eq!(m.repone_en, t + 5 + 3 * 3_600_000);
    }

    #[test]
    fn el_cupo_diario_se_cuenta_por_dia_de_mundo() {
        let r = reglas().mercenarios;
        let mut c = CupoDiario::default();
        assert_eq!(c.restante(&r, Madera, 0), Some(60_000));
        assert_eq!(c.restante(&r, Piedra, 0), Some(10_000));
        assert_eq!(c.restante(&r, Cobre, 0), None, "el cobre no tiene cupo");
        c.anotar(Madera, 45_000, 1_000);
        assert_eq!(c.restante(&r, Madera, 2_000), Some(15_000));
        assert_eq!(c.restante(&r, Piedra, 2_000), Some(10_000));
        assert_eq!(c.restante(&r, Madera, MS_DIA - 1), Some(15_000));
        assert_eq!(c.restante(&r, Madera, MS_DIA), Some(60_000), "al día siguiente se renueva");
        c.anotar(Madera, 1_000, MS_DIA + 5);
        assert_eq!((c.dia, c.comprado[&Madera]), (1, 1_000));
        assert_eq!(dia_de(-1), -1);
    }

    // Madera de referencia 1 → con margen 1,3.
    const P: Option<i64> = Some(1_300_000);

    #[test]
    fn el_residente_paga_ceil_de_su_almacen_personal() {
        // 10 unidades a 1,3 = 13 de oro; guardado 100; sitio de sobra.
        let c = comprar_residente(Madera, P, 10_000, 300_000, Some(60_000), 100_000, 900_000).unwrap();
        assert_eq!((c.cantidad, c.oro_cobrado), (10_000, 13_000));
        // Cantidad fraccionaria: se pide 10,9 y solo se sirven enteras.
        assert_eq!(comprar_residente(Madera, P, 10_900, 300_000, None, 100_000, 900_000).unwrap().cantidad, 10_000);
    }

    #[test]
    fn el_residente_topa_por_stock_cupo_oro_y_sitio() {
        let comprar = |pedida, stock, cupo, oro, sitio| comprar_residente(Madera, P, pedida, stock, cupo, oro, sitio);
        assert_eq!(comprar(500_000, 7_000, None, 900_000, 900_000).unwrap().cantidad, 7_000, "stock");
        assert_eq!(comprar(500_000, 300_000, Some(25_000), 900_000, 900_000).unwrap().cantidad, 25_000, "cupo");
        let c = comprar(500_000, 300_000, None, 13_000, 900_000).unwrap();
        assert_eq!((c.cantidad, c.oro_cobrado), (10_000, 13_000), "oro: floor(13 / 1,3) = 10");
        // A precio < 1 el almacén personal crece (cantidad − oro): con sitio para 5, 11 unidades cuestan 6.
        let c = comprar_residente(Madera, Some(500_000), 500_000, 300_000, None, 900_000, 5_000).unwrap();
        assert_eq!((c.cantidad, c.oro_cobrado), (11_000, 6_000));
        assert!(c.cantidad - c.oro_cobrado <= 5_000);
        // A precio > 1 el almacén no crece nunca: el sitio no limita.
        assert_eq!(comprar(500_000, 300_000, None, 900_000, 0).unwrap().cantidad, 300_000);
    }

    #[test]
    fn el_residente_falla_con_el_motivo_si_no_puede_servir_nada() {
        let comprar = |stock, cupo, oro, sitio| comprar_residente(Madera, P, 5_000, stock, cupo, oro, sitio);
        assert_eq!(comprar(0, None, 100_000, 900_000), Err(ErrorMercenarios::SinStock));
        assert_eq!(comprar(100_000, Some(0), 100_000, 900_000), Err(ErrorMercenarios::CupoAgotado));
        assert_eq!(comprar(100_000, None, 1_000, 900_000), Err(ErrorMercenarios::SinOro));
        // Almacén ya pasado de capacidad: ni siquiera una unidad (que cuesta ≥ 1 de oro) cabe.
        assert_eq!(comprar_residente(Madera, Some(500_000), 5_000, 100_000, None, 100_000, -1_000), Err(ErrorMercenarios::SinSitio));
        assert_eq!(comprar_residente(Madera, P, 999, 100_000, None, 100_000, 900_000), Err(ErrorMercenarios::CantidadInvalida), "menos de 1");
        assert_eq!(comprar_residente(Oro, P, 5_000, 100_000, None, 100_000, 900_000), Err(ErrorMercenarios::CantidadInvalida), "pedir oro");
        assert_eq!(comprar_residente(Cuero, None, 5_000, 100_000, None, 100_000, 900_000), Err(ErrorMercenarios::SinPrecio));
    }

    #[test]
    fn el_visitante_compra_al_carro_cualquier_cosa_menos_trigo_sin_cupo() {
        let c = comprar_visitante(Cobre, Some(3_900_000), 10_000, 100_000, 200_000, 500_000).unwrap();
        assert_eq!((c.cantidad, c.oro_cobrado), (10_000, 39_000));
        assert_eq!(comprar_visitante(Trigo, Some(1_950_000), 10_000, 100_000, 200_000, 500_000), Err(ErrorMercenarios::TrigoNoVaAlCarro), "pedirlo es un error");
        let c = comprar_visitante(Cobre, Some(3_900_000), 100_000, 100_000, 100_000, 500_000).unwrap();
        assert_eq!(c.cantidad, 25_000, "floor(100 / 3,9) = 25: el oro del carro manda");
        assert_eq!(comprar_visitante(Cobre, Some(3_900_000), 10_000, 100_000, 3_000, 500_000), Err(ErrorMercenarios::SinOro));
        assert_eq!(comprar_visitante(Cobre, Some(3_900_000), 10_000, 0, 200_000, 500_000), Err(ErrorMercenarios::SinStock));
    }

    #[test]
    fn el_carro_del_visitante_solo_crece_si_el_precio_es_menor_que_uno() {
        // Precio 0,5: 10 unidades cuestan 5 de oro y el carro crece 5. Con 3 de sitio, 7 unidades (cuestan ceil(3,5) = 4: crece 3).
        let c = comprar_visitante(Madera, Some(500_000), 10_000, 100_000, 100_000, 3_000).unwrap();
        assert_eq!((c.cantidad, c.oro_cobrado), (7_000, 4_000));
        // Precio > 1: cambia oro por menos mercancía; el carro nunca crece, ni con el carro lleno.
        let c = comprar_visitante(Cobre, Some(3_900_000), 10_000, 100_000, 100_000, 0).unwrap();
        assert_eq!(c.cantidad, 10_000);
        assert_eq!(comprar_visitante(Madera, Some(500_000), 10_000, 100_000, 100_000, -1_000), Err(ErrorMercenarios::SinSitio), "carro pasado de capacidad");
    }

    #[test]
    fn repostar_es_gratis_hasta_llenar_y_falla_si_ya_esta_lleno() {
        let r = reglas().mercenarios;
        assert_eq!(repostar(&r, 100_000), Ok(250_000), "hasta los 350");
        assert_eq!(repostar(&r, 0), Ok(350_000));
        assert_eq!(repostar(&r, 350_000), Err(ErrorMercenarios::VivereLlenos));
    }

    #[test]
    fn las_compras_son_deterministas() {
        let h = || comprar_residente(Hierro, Some(5_200_000), 37_000, 100_000, None, 77_000, 400_000);
        assert_eq!(h(), h());
    }
}
