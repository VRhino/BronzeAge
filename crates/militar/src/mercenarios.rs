//! La tropa en un campamento de mercenarios (Campamentos_Mercenarios §4-§6, Doc 5.8 y 1.9b): reclutar con oro y población del campamento, la
//! tropa **prestada** y su retirada. La planta, el mercado y el almacén personal son de otros módulos y sistemas.

use crate::catalogo::{DefTropa, Tropa};
use crate::datos::Reglas;
use crate::escuadra::{Contenedor, Escuadra};
use crate::heroe::Heroe;
use crate::liderazgo;
use crate::tipos::{CampamentoId, ColumnaId, EscuadraId};
use asentamiento::datos::UNO;
use asentamiento::Tipo;
use base::{Instante, Mil, MS_POR_MINUTO};

const MS_POR_HORA: i64 = 60 * MS_POR_MINUTO;
/// La única tecnología de la tropa prestada: la leva comunal (milicia, leñadores, granjeros y mineros).
pub const TECNOLOGIA_DE_LEVA: &str = "leva_comunal";

/// Los reclutas de un campamento: se guardan como valor e instante y la cantidad de ahora se calcula al mirar (no se escribe por minuto).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reclutas {
    /// Reclutas en `en`, en milésimas.
    pub poblacion: Mil,
    pub en: Instante,
}

impl Reclutas {
    /// Un campamento nace lleno.
    pub fn llenos(r: &Reglas, viviendas: usize, ahora: Instante) -> Self {
        Self { poblacion: tope(r, viviendas), en: ahora }
    }

    /// Reclutas disponibles en `t`, en milésimas: `min(tope, poblacion + horas(t − en) × poblacionPorHora)`.
    pub fn actual(&self, r: &Reglas, viviendas: usize, t: Instante) -> Mil {
        let crece = (t - self.en).max(0) as i128 * (r.mercenarios.poblacion_por_hora * 1_000) as i128 / MS_POR_HORA as i128;
        (self.poblacion as i128 + crece).min(tope(r, viviendas) as i128) as Mil
    }
}

/// El tope de reclutas: viviendas × `poblacionPorVivienda`, en milésimas.
pub fn tope(r: &Reglas, viviendas: usize) -> Mil {
    viviendas as i64 * r.mercenarios.poblacion_por_vivienda * 1_000
}

/// Las tropas que un campamento ofrece (`tropasDelCampamento`): su edificio existe en él (Barracón, Galería de tiro o Caballerizas; el
/// Centro Urbano no, así que la leva de arranque no se recluta), su nivel entra en `nivelEdificios` y su tecnología está desbloqueada en el
/// campamento (`tecnologia`, del sistema 6).
pub fn tropas_del_campamento(r: &Reglas, edificios: &[Tipo], tecnologia: &dyn Fn(&str) -> bool) -> Vec<Tropa> {
    r.tropas
        .iter()
        .filter(|d| d.edificio != Tipo::CentroUrbano && edificios.contains(&d.edificio) && d.nivel_requerido <= r.mercenarios.nivel_edificios && tecnologia(&d.tecnologia))
        .map(|d| d.tropa)
        .collect()
}

/// Lo que cobra el campamento por reclutar `cantidad` soldados, en oro y en milésimas (siempre unidades enteras de oro):
/// `base = oroDelEscalón + caballos × oroPorCaballo + unidadesDeEquipo × valorEquipoEnOro`, `× recargo × factorReputación`, y
/// `× descuentoSinAsentamientos` si la Facción no tiene ninguno; el total se redondea hacia arriba. El campamento **cobra todo en oro**: no
/// recibe equipo fabricado. Un héroe sin Facción no tiene recargo por reputación ni descuento (`UNO`, `false`).
pub fn precio(r: &Reglas, def: &DefTropa, cantidad: i64, factor_reputacion: i64, sin_asentamientos: bool) -> Mil {
    let equipo: Mil = def.costo_equipo.iter().map(|&(_, n)| n).sum::<Mil>() * r.mercenarios.valor_equipo_en_oro / 1_000;
    let base = r.reclutamiento.oro_por_escalon[def.escalon as usize - 1] + def.caballos * r.reclutamiento.oro_por_caballo + equipo;
    let descuento = if sin_asentamientos { r.mercenarios.descuento_sin_asentamientos } else { UNO };
    let numerador = base as i128 * r.mercenarios.recargo as i128 * factor_reputacion as i128 * descuento as i128 * cantidad as i128;
    let denominador = 1_000 * UNO as i128 * UNO as i128 * UNO as i128;
    ((numerador + denominador - 1) / denominador) as Mil * 1_000
}

/// Lo que cuesta un soldado de `def` antes de redondear (milésimas de oro, truncado): el `base × recargo × factorReputación × descuento` de [`precio`], que cobra
/// `⌈precio_por_soldado × cantidad / 1.000⌉` unidades de oro. Lo enseña el campamento a su residente (2026-10-27).
pub fn precio_por_soldado(r: &Reglas, def: &DefTropa, factor_reputacion: i64, sin_asentamientos: bool) -> Mil {
    let equipo: Mil = def.costo_equipo.iter().map(|&(_, n)| n).sum::<Mil>() * r.mercenarios.valor_equipo_en_oro / 1_000;
    let base = r.reclutamiento.oro_por_escalon[def.escalon as usize - 1] + def.caballos * r.reclutamiento.oro_por_caballo + equipo;
    let descuento = if sin_asentamientos { r.mercenarios.descuento_sin_asentamientos } else { UNO };
    (base as i128 * r.mercenarios.recargo as i128 * factor_reputacion as i128 * descuento as i128 / (UNO as i128 * UNO as i128 * UNO as i128)) as Mil
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorMercenarios {
    /// Solo se recluta en el campamento donde se reside.
    NoResides,
    /// Hay que estar en él: dentro o con la columna a la puerta.
    Lejos,
    /// El campamento no ofrece esa tropa.
    NoDisponible,
    /// Ya tienes una escuadra de esa tropa y no está aquí (campamento, o columna a la puerta): solo se repone donde está.
    YaLaTienesFuera(Contenedor),
    AlTope,
    /// El campamento no tiene tantos reclutas.
    SinReclutas { hacen_falta: i64, hay: i64 },
    SinOro,
    /// Un préstamo pide al menos una tropa.
    SinTropas,
    /// Solo la leva comunal se presta.
    NoSePresta,
    Repetida,
    YaPrestada,
    NadaQueReponer,
}

pub struct Campamento<'a> {
    pub id: CampamentoId,
    pub viviendas: usize,
    pub reclutas: &'a mut Reclutas,
    /// Qué ofrece (`tropas_del_campamento`).
    pub ofrece: &'a [Tropa],
}

pub struct Pedido<'a> {
    pub heroe: &'a mut Heroe,
    pub tropa: Tropa,
    /// Reside en este campamento.
    pub reside: bool,
    /// Está en él: dentro, o con la columna a la puerta.
    pub esta_en_el_campamento: bool,
    /// A cuánto de la puerta del campamento está una columna.
    pub distancia_de_columna: &'a dyn Fn(ColumnaId) -> Option<Mil>,
    /// Oro con el que paga (su almacén personal, o el carro de su columna a la puerta).
    pub oro_disponible: Mil,
    pub factor_reputacion: i64,
    pub sin_asentamientos: bool,
    /// La columna a la puerta de la que es Líder, si la hay: una escuadra nueva que le cabe en el Liderazgo nace directamente en ella.
    pub columna_del_lider: Option<ColumnaId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reclutado {
    pub escuadra: EscuadraId,
    pub cantidad: i64,
    /// Oro cobrado, en milésimas (enteras).
    pub oro: Mil,
    /// La columna a la que se unió la escuadra nueva (para que quien llama la añada a sus ids).
    pub se_une: Option<ColumnaId>,
}

/// `reclutarEnCampamento`. Cuesta **solo oro** y reclutas del campamento: sin equipo fabricado, sin reserva de trigo y sin población de plaza.
/// Una escuadra por tropa en toda la partida: si ya la tiene, repone hasta el tope y solo cobra lo que falta. La escuadra nueva nace en el
/// campamento, salvo que su héroe sea el Líder de una columna a la puerta y le quepa en el Liderazgo contando lo que ya lleva fuera del
/// campamento: entonces se une a la columna, para no obligarle a entrar y volver a salir. Reponer no mueve la escuadra.
pub fn reclutar(r: &Reglas, campamento: Campamento, pedido: Pedido, nuevo_id: impl FnOnce() -> EscuadraId, ahora: Instante) -> Result<Reclutado, ErrorMercenarios> {
    let Pedido { heroe, tropa, reside, esta_en_el_campamento, distancia_de_columna, oro_disponible, factor_reputacion, sin_asentamientos, columna_del_lider } = pedido;
    if !reside {
        return Err(ErrorMercenarios::NoResides);
    }
    if !esta_en_el_campamento {
        return Err(ErrorMercenarios::Lejos);
    }
    if !campamento.ofrece.contains(&tropa) {
        return Err(ErrorMercenarios::NoDisponible);
    }
    let def = r.tropas.tropa(tropa);
    let existente = heroe.escuadra_de(tropa);
    if let Some(e) = existente {
        let aqui = match e.contenedor {
            Contenedor::Campamento => true,
            Contenedor::Ejercito(c) => distancia_de_columna(c).is_some_and(|d| d <= r.movimiento.radio_puerta),
            Contenedor::Escolta(_) | Contenedor::Fuera | Contenedor::Avanzada(_) | Contenedor::Pueblo(_) => false,
        };
        if !aqui {
            return Err(ErrorMercenarios::YaLaTienesFuera(e.contenedor));
        }
    }
    let cantidad = def.unidades - existente.map_or(0, |e| e.cantidad);
    if cantidad <= 0 {
        return Err(ErrorMercenarios::AlTope);
    }
    let hay = campamento.reclutas.actual(r, campamento.viviendas, ahora);
    if hay < cantidad * 1_000 {
        return Err(ErrorMercenarios::SinReclutas { hacen_falta: cantidad, hay: hay / 1_000 });
    }
    let oro = precio(r, def, cantidad, factor_reputacion, sin_asentamientos);
    if oro_disponible < oro {
        return Err(ErrorMercenarios::SinOro);
    }

    *campamento.reclutas = Reclutas { poblacion: hay - cantidad * 1_000, en: ahora };
    if let Some(e) = heroe.escuadras.iter_mut().find(|e| e.tropa == tropa && e.prestada.is_none()) {
        e.cantidad += cantidad;
        return Ok(Reclutado { escuadra: e.id, cantidad, oro, se_une: None });
    }
    let id = nuevo_id();
    let mut nueva = Escuadra::nueva(r, id, heroe.id, tropa, cantidad, None, ahora);
    let fuera = heroe.escuadras.iter().filter(|e| matches!(e.contenedor, Contenedor::Ejercito(_)));
    let se_une = columna_del_lider.filter(|_| liderazgo::puede_llevar(r, heroe.liderazgo_base, fuera.chain(std::iter::once(&nueva))));
    if let Some(c) = se_une {
        nueva.contenedor = Contenedor::Ejercito(c);
    }
    heroe.escuadras.push(nueva);
    Ok(Reclutado { escuadra: id, cantidad, oro, se_une })
}

/// `pedirPrestamo`: al menos una tropa, solo de leva comunal, sin repetir y sin una que ya tenga prestada. Cada una nace completa
/// (`unidades`) en el campamento del héroe, con la moral al máximo, marcada como prestada. Todo o nada. Devuelve los ids creados.
pub fn pedir_prestamo(r: &Reglas, heroe: &mut Heroe, campamento: CampamentoId, tropas: &[Tropa], ids: &mut dyn FnMut() -> EscuadraId, ahora: Instante) -> Result<Vec<EscuadraId>, ErrorMercenarios> {
    if tropas.is_empty() {
        return Err(ErrorMercenarios::SinTropas);
    }
    for (i, &t) in tropas.iter().enumerate() {
        if r.tropas.tropa(t).tecnologia != TECNOLOGIA_DE_LEVA {
            return Err(ErrorMercenarios::NoSePresta);
        }
        if tropas[..i].contains(&t) {
            return Err(ErrorMercenarios::Repetida);
        }
        if heroe.escuadras.iter().any(|e| e.tropa == t && e.prestada.is_some()) {
            return Err(ErrorMercenarios::YaPrestada);
        }
    }
    let creadas: Vec<Escuadra> = tropas.iter().map(|&t| Escuadra::prestada(r, ids(), heroe.id, t, campamento, ahora)).collect();
    let nuevos = creadas.iter().map(|e| e.id).collect();
    heroe.escuadras.extend(creadas);
    Ok(nuevos)
}

/// `reponerPrestamo`: repone **gratis** hasta su tamaño completo las prestadas de este campamento que estén en él o en una columna a su
/// puerta. Devuelve cuántas escuadras repuso; si no había nada que reponer, error.
pub fn reponer_prestamo(r: &Reglas, heroe: &mut Heroe, campamento: CampamentoId, distancia_de_columna: &dyn Fn(ColumnaId) -> Option<Mil>) -> Result<usize, ErrorMercenarios> {
    let mut repuestas = 0;
    for e in heroe.escuadras.iter_mut().filter(|e| e.prestada == Some(campamento)) {
        let aqui = match e.contenedor {
            Contenedor::Campamento => true,
            Contenedor::Ejercito(c) => distancia_de_columna(c).is_some_and(|d| d <= r.movimiento.radio_puerta),
            Contenedor::Escolta(_) | Contenedor::Fuera | Contenedor::Avanzada(_) | Contenedor::Pueblo(_) => false,
        };
        let completa = r.tropas.tropa(e.tropa).unidades;
        if aqui && e.cantidad < completa {
            e.cantidad = completa;
            repuestas += 1;
        }
    }
    if repuestas == 0 {
        return Err(ErrorMercenarios::NadaQueReponer);
    }
    Ok(repuestas)
}

/// La retirada de la tropa prestada: toda escuadra prestada por un campamento donde su héroe ya no reside se retira **de donde esté** (de la
/// columna, de una escolta, de la guarnición). Devuelve las retiradas para que quien llama las quite de las listas de ids y lo narre.
pub fn retirar_prestamos_ajenos(heroe: &mut Heroe, reside_en: &dyn Fn(CampamentoId) -> bool) -> Vec<Escuadra> {
    let (ajenas, propias): (Vec<_>, Vec<_>) = std::mem::take(&mut heroe.escuadras).into_iter().partition(|e| e.prestada.is_some_and(|c| !reside_en(c)));
    heroe.escuadras = propias;
    ajenas
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::heroe::{Controlador, Residencia};

    fn r() -> Reglas {
        Reglas::cargar()
    }

    fn heroe() -> Heroe {
        let mut h = Heroe::nuevo(&r(), 1, Controlador::Humano, 3, 1);
        h.residencia = Some(Residencia::Campamento(3));
        h
    }

    const TODOS_LOS_EDIFICIOS: [Tipo; 3] = [Tipo::Barracon, Tipo::GaleriaDeTiro, Tipo::Caballerizas];

    fn ofrece(r: &Reglas) -> Vec<Tropa> {
        tropas_del_campamento(r, &TODOS_LOS_EDIFICIOS, &|_| true)
    }

    struct Mundo {
        h: Heroe,
        reclutas: Reclutas,
        ofrece: Vec<Tropa>,
        oro: Mil,
        columna: Option<ColumnaId>,
        columnas: Vec<(ColumnaId, Mil)>,
        reside: bool,
        esta: bool,
    }

    fn mundo() -> Mundo {
        let r = r();
        Mundo { h: heroe(), reclutas: Reclutas::llenos(&r, 10, 0), ofrece: ofrece(&r), oro: 100_000_000, columna: None, columnas: vec![], reside: true, esta: true }
    }

    fn recluta(m: &mut Mundo, tropa: Tropa, ahora: Instante) -> Result<Reclutado, ErrorMercenarios> {
        let r = r();
        let cs = m.columnas.clone();
        let busca = move |c: ColumnaId| cs.iter().find(|(k, _)| *k == c).map(|x| x.1);
        let camp = Campamento { id: 3, viviendas: 10, reclutas: &mut m.reclutas, ofrece: &m.ofrece };
        let pedido = Pedido {
            heroe: &mut m.h,
            tropa,
            reside: m.reside,
            esta_en_el_campamento: m.esta,
            distancia_de_columna: &busca,
            oro_disponible: m.oro,
            factor_reputacion: UNO,
            sin_asentamientos: false,
            columna_del_lider: m.columna,
        };
        reclutar(&r, camp, pedido, || 50, ahora)
    }

    // ───────────── reclutas del campamento ─────────────

    #[test]
    fn nace_lleno_cien_con_diez_viviendas_y_recupera_diez_por_hora() {
        let r = r();
        let rc = Reclutas::llenos(&r, 10, 0);
        assert_eq!(rc.actual(&r, 10, 0), 100_000);
        let gastado = Reclutas { poblacion: 20_000, en: 0 };
        assert_eq!(gastado.actual(&r, 10, 0), 20_000);
        assert_eq!(gastado.actual(&r, 10, MS_POR_HORA / 2), 25_000, "5 recuperados en media hora");
        assert_eq!(gastado.actual(&r, 10, 3 * MS_POR_HORA), 50_000);
        assert_eq!(gastado.actual(&r, 10, 100 * MS_POR_HORA), 100_000, "no pasa del tope");
        assert_eq!(gastado.actual(&r, 5, 100 * MS_POR_HORA), 50_000, "el tope son las viviendas × 10");
        assert_eq!(gastado.actual(&r, 10, -MS_POR_HORA), 20_000, "el tiempo no es negativo");
    }

    #[test]
    fn el_campamento_ofrece_solo_las_tropas_de_sus_edificios_militares_nunca_la_leva_de_arranque() {
        let r = r();
        let todas = ofrece(&r);
        assert!(!todas.contains(&Tropa::MiliciaLanceros) && !todas.contains(&Tropa::Mineros), "el Centro Urbano no existe aquí");
        assert!(todas.contains(&Tropa::Arqueros) && todas.contains(&Tropa::ArquerosCompuesto) && todas.contains(&Tropa::CarrosGuerra));
        assert_eq!(todas.len(), 27 - 4, "las 23 tropas de Barracón, Galería y Caballerizas (nivel ≤ 3)");
        let solo_barracon = tropas_del_campamento(&r, &[Tipo::Barracon], &|_| true);
        assert!(solo_barracon.contains(&Tropa::Shardana) && !solo_barracon.contains(&Tropa::Arqueros));
        let sin_hierro = tropas_del_campamento(&r, &TODOS_LOS_EDIFICIOS, &|t| t != "forja_hierro_temprana" && t != "equitacion_militar");
        assert!(!sin_hierro.contains(&Tropa::GuerrerosFilisteos) && !sin_hierro.contains(&Tropa::JinetesAsirios), "la tecnología tiene que estar desbloqueada en el campamento");
        assert!(sin_hierro.contains(&Tropa::Arqueros));
    }

    // ───────────── el precio ─────────────

    #[test]
    fn el_precio_es_oro_escalon_mas_caballos_mas_equipo_a_ocho_por_recargo() {
        let r = r();
        // Lanceros de mimbre: escalón 2 (2) + 1 arma de madera × 8 = 10; × 1,5 = 15 por soldado; 25 soldados.
        assert_eq!(precio(&r, r.tropas.tropa(Tropa::LancerosMimbre), 25, UNO, false), 375_000);
        // Carros de guerra: escalón 4 (7) + 2 caballos × 5 + (1 carro + 1 arma) × 8 = 33; × 1,5 = 49,5; 15 soldados = 742,5 → 743.
        assert_eq!(precio(&r, r.tropas.tropa(Tropa::CarrosGuerra), 15, UNO, false), 743_000);
    }

    #[test]
    fn el_precio_por_soldado_por_las_unidades_redondeado_es_el_precio() {
        let r = r();
        for d in r.tropas.iter() {
            for (f, sin) in [(UNO, false), (UNO * 3 / 2, false), (UNO, true)] {
                let por = precio_por_soldado(&r, d, f, sin);
                for n in [1, 7, d.unidades] {
                    assert!((precio(&r, d, n, f, sin) - por * n).abs() < 1_000 + n, "{:?} × {n}", d.tropa);
                }
            }
        }
    }

    #[test]
    fn la_mala_reputacion_recarga_y_la_faccion_sin_asentamientos_tiene_descuento() {
        let r = r();
        let d = r.tropas.tropa(Tropa::LancerosMimbre);
        assert_eq!(precio(&r, d, 25, 1_500_000, false), 563_000, "375 × 1,5 = 562,5 → 563");
        assert_eq!(precio(&r, d, 25, UNO, true), 225_000, "× 0,6");
        assert_eq!(precio(&r, d, 25, 1_500_000, true), 338_000, "375 × 1,5 × 0,6 = 337,5 → 338");
        assert_eq!(precio(&r, d, 0, UNO, false), 0);
    }

    #[test]
    fn el_precio_se_redondea_hacia_arriba_una_sola_vez_sobre_el_total() {
        let r = r();
        // Un soldado de lanceros de mimbre cuesta 15; de espadachines de cobre (esc. 2: 2 + (1 arma + 1 armadura) × 8 = 18 → 27).
        assert_eq!(precio(&r, r.tropas.tropa(Tropa::EspadachinesCobre), 1, UNO, false), 27_000);
        // Con 0,6: 16,2 por soldado → 16,2 × 3 = 48,6 → 49 (y no 3 × 17).
        assert_eq!(precio(&r, r.tropas.tropa(Tropa::EspadachinesCobre), 3, UNO, true), 49_000);
    }

    // ───────────── reclutar ─────────────

    #[test]
    fn recluta_cobra_el_oro_gasta_reclutas_y_deja_la_escuadra_nueva_en_el_campamento() {
        let mut m = mundo();
        let res = recluta(&mut m, Tropa::LancerosMimbre, 0).unwrap();
        assert_eq!(res, Reclutado { escuadra: 50, cantidad: 25, oro: 375_000, se_une: None });
        assert_eq!(m.reclutas, Reclutas { poblacion: 75_000, en: 0 });
        let e = m.h.escuadra(50).unwrap();
        assert_eq!((e.cantidad, e.contenedor, e.plaza_origen, e.prestada), (25, Contenedor::Campamento, None, None));
    }

    #[test]
    fn reponer_solo_cobra_lo_que_falta_y_no_mueve_la_escuadra() {
        let mut m = mundo();
        recluta(&mut m, Tropa::LancerosMimbre, 0).unwrap();
        m.h.escuadra_mut(50).unwrap().cantidad = 15;
        let res = recluta(&mut m, Tropa::LancerosMimbre, 0).unwrap();
        assert_eq!((res.cantidad, res.oro, res.se_une, res.escuadra), (10, 150_000, None, 50));
        assert_eq!(m.h.escuadra(50).unwrap().cantidad, 25);
        assert_eq!(recluta(&mut m, Tropa::LancerosMimbre, 0), Err(ErrorMercenarios::AlTope));
    }

    #[test]
    fn solo_se_recluta_donde_se_reside_y_estando_en_el() {
        let mut m = mundo();
        m.reside = false;
        assert_eq!(recluta(&mut m, Tropa::Arqueros, 0), Err(ErrorMercenarios::NoResides));
        m.reside = true;
        m.esta = false;
        assert_eq!(recluta(&mut m, Tropa::Arqueros, 0), Err(ErrorMercenarios::Lejos));
        m.esta = true;
        assert_eq!(recluta(&mut m, Tropa::MiliciaLanceros, 0), Err(ErrorMercenarios::NoDisponible), "la leva de arranque no se recluta aquí");
    }

    #[test]
    fn no_hay_reclutas_ni_oro_no_se_recluta_y_no_se_toca_nada() {
        let mut m = mundo();
        m.reclutas = Reclutas { poblacion: 24_000, en: 0 };
        assert_eq!(recluta(&mut m, Tropa::LancerosMimbre, 0), Err(ErrorMercenarios::SinReclutas { hacen_falta: 25, hay: 24 }));
        assert_eq!(recluta(&mut m, Tropa::LancerosMimbre, MS_POR_HORA / 10), Ok(Reclutado { escuadra: 50, cantidad: 25, oro: 375_000, se_une: None }), "una hora después hay uno más: justo 25");
        let mut m = mundo();
        m.oro = 374_999;
        assert_eq!(recluta(&mut m, Tropa::LancerosMimbre, 0), Err(ErrorMercenarios::SinOro));
        assert_eq!((m.reclutas.poblacion, m.h.escuadras.len()), (100_000, 0));
    }

    #[test]
    fn una_escuadra_que_va_en_una_columna_solo_se_repone_con_la_columna_a_la_puerta() {
        let mut m = mundo();
        recluta(&mut m, Tropa::Arqueros, 0).unwrap();
        m.h.escuadra_mut(50).unwrap().cantidad = 10;
        m.h.escuadra_mut(50).unwrap().contenedor = Contenedor::Ejercito(8);
        assert_eq!(recluta(&mut m, Tropa::Arqueros, 0), Err(ErrorMercenarios::YaLaTienesFuera(Contenedor::Ejercito(8))));
        m.columnas = vec![(8, 1_999)];
        assert_eq!(recluta(&mut m, Tropa::Arqueros, 0).map(|x| x.cantidad), Ok(8));
        m.h.escuadra_mut(50).unwrap().cantidad = 1;
        m.h.escuadra_mut(50).unwrap().contenedor = Contenedor::Escolta(2);
        assert_eq!(recluta(&mut m, Tropa::Arqueros, 0), Err(ErrorMercenarios::YaLaTienesFuera(Contenedor::Escolta(2))), "en escolta, nunca");
    }

    #[test]
    fn la_escuadra_nueva_se_une_a_la_columna_del_lider_si_le_cabe_en_el_liderazgo() {
        let mut m = mundo();
        m.columna = Some(8);
        // Lleva fuera 3 pesadas (96) del Liderazgo de 100: una veterana más (22) no cabe, una leva... tampoco (103); sin nada fuera cabe todo.
        let r = r();
        for (i, t) in [Tropa::Shardana, Tropa::Espartiatas, Tropa::LancerosPesados].into_iter().enumerate() {
            let mut e = Escuadra::nueva(&r, 10 + i as u64, 1, t, 15, None, 0);
            e.contenedor = Contenedor::Ejercito(8);
            m.h.escuadras.push(e);
        }
        let res = recluta(&mut m, Tropa::Arqueros, 0).unwrap();
        assert_eq!(res.se_une, None, "96 + 22 > 100: nace en el campamento");
        assert_eq!(m.h.escuadra(50).unwrap().contenedor, Contenedor::Campamento);
        // Sin lo que llevaba fuera, cabe y nace ya en la columna.
        let mut m = mundo();
        m.columna = Some(8);
        let res = recluta(&mut m, Tropa::Arqueros, 0).unwrap();
        assert_eq!(res.se_une, Some(8));
        assert_eq!(m.h.escuadra(50).unwrap().contenedor, Contenedor::Ejercito(8));
        // Una repuesta no se mueve aunque haya columna.
        m.h.escuadra_mut(50).unwrap().cantidad = 3;
        m.h.escuadra_mut(50).unwrap().contenedor = Contenedor::Campamento;
        assert_eq!(recluta(&mut m, Tropa::Arqueros, 0).map(|x| x.se_une), Ok(None));
        assert_eq!(m.h.escuadra(50).unwrap().contenedor, Contenedor::Campamento);
    }

    // ───────────── la tropa prestada ─────────────

    fn ids() -> impl FnMut() -> EscuadraId {
        let mut n = 100;
        move || {
            n += 1;
            n
        }
    }

    #[test]
    fn el_prestamo_da_escuadras_completas_de_leva_sin_experiencia_y_marcadas() {
        let r = r();
        let mut h = heroe();
        let creadas = pedir_prestamo(&r, &mut h, 3, &[Tropa::Granjeros, Tropa::Mineros], &mut ids(), 0).unwrap();
        assert_eq!(creadas, vec![101, 102]);
        let e = h.escuadra(102).unwrap();
        assert_eq!((e.tropa, e.cantidad, e.prestada, e.contenedor, e.moral_en(0)), (Tropa::Mineros, 30, Some(3), Contenedor::Campamento, 100_000));
        assert!(h.escuadra_de(Tropa::Granjeros).is_none(), "la prestada no cuenta como «una escuadra por tropa»");
    }

    #[test]
    fn el_prestamo_es_todo_o_nada_y_solo_de_leva_comunal() {
        let r = r();
        let mut h = heroe();
        assert_eq!(pedir_prestamo(&r, &mut h, 3, &[], &mut ids(), 0), Err(ErrorMercenarios::SinTropas));
        assert_eq!(pedir_prestamo(&r, &mut h, 3, &[Tropa::Granjeros, Tropa::Arqueros], &mut ids(), 0), Err(ErrorMercenarios::NoSePresta));
        assert_eq!(pedir_prestamo(&r, &mut h, 3, &[Tropa::Granjeros, Tropa::Granjeros], &mut ids(), 0), Err(ErrorMercenarios::Repetida));
        assert!(h.escuadras.is_empty(), "un rechazo no crea nada");
        pedir_prestamo(&r, &mut h, 3, &[Tropa::MiliciaLanceros], &mut ids(), 0).unwrap();
        assert_eq!(pedir_prestamo(&r, &mut h, 3, &[Tropa::Lenadores, Tropa::MiliciaLanceros], &mut ids(), 0), Err(ErrorMercenarios::YaPrestada));
        assert_eq!(h.escuadras.len(), 1);
    }

    #[test]
    fn una_prestada_no_impide_reclutar_la_propia_de_esa_tropa() {
        let r = r();
        let mut h = heroe();
        pedir_prestamo(&r, &mut h, 3, &[Tropa::MiliciaLanceros], &mut ids(), 0).unwrap();
        assert!(h.escuadra_de(Tropa::MiliciaLanceros).is_none());
        h.escuadras.push(Escuadra::nueva(&r, 7, 1, Tropa::MiliciaLanceros, 25, Some(2), 0));
        assert_eq!(h.escuadra_de(Tropa::MiliciaLanceros).map(|e| e.id), Some(7));
    }

    #[test]
    fn reponer_el_prestamo_es_gratis_y_solo_donde_esta_a_la_puerta() {
        let r = r();
        let mut h = heroe();
        pedir_prestamo(&r, &mut h, 3, &[Tropa::Granjeros, Tropa::Lenadores, Tropa::MiliciaLanceros], &mut ids(), 0).unwrap();
        h.escuadras[0].cantidad = 4; // en el campamento
        h.escuadras[1].cantidad = 6;
        h.escuadras[1].contenedor = Contenedor::Ejercito(8); // en una columna lejos
        h.escuadras[2].cantidad = 9;
        h.escuadras[2].contenedor = Contenedor::Ejercito(9); // en una columna a la puerta
        let busca = |c: ColumnaId| if c == 9 { Some(2_000) } else { Some(2_001) };
        assert_eq!(reponer_prestamo(&r, &mut h, 3, &busca), Ok(2));
        assert_eq!((h.escuadras[0].cantidad, h.escuadras[1].cantidad, h.escuadras[2].cantidad), (30, 6, 25));
        assert_eq!(reponer_prestamo(&r, &mut h, 3, &busca), Err(ErrorMercenarios::NadaQueReponer));
        assert_eq!(reponer_prestamo(&r, &mut h, 4, &busca), Err(ErrorMercenarios::NadaQueReponer), "las de otro campamento no se tocan");
    }

    #[test]
    fn dejar_de_residir_retira_la_prestada_de_donde_este() {
        let r = r();
        let mut h = heroe();
        pedir_prestamo(&r, &mut h, 3, &[Tropa::Granjeros, Tropa::Lenadores], &mut ids(), 0).unwrap();
        h.escuadras[1].contenedor = Contenedor::Escolta(5);
        h.escuadras.push(Escuadra::nueva(&r, 9, 1, Tropa::Arqueros, 18, Some(2), 0));
        // Sigue residiendo allí: no se retira nada.
        assert!(retirar_prestamos_ajenos(&mut h, &|c| c == 3).is_empty());
        // Se muda: salen las dos prestadas, también la que iba de escolta; la propia se queda.
        let retiradas = retirar_prestamos_ajenos(&mut h, &|_| false);
        assert_eq!(retiradas.iter().map(|e| (e.id, e.contenedor)).collect::<Vec<_>>(), vec![(101, Contenedor::Campamento), (102, Contenedor::Escolta(5))]);
        assert_eq!(h.escuadras.iter().map(|e| e.id).collect::<Vec<_>>(), vec![9]);
    }
}
