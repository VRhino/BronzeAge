//! Reclutar tropa en una plaza (Doc 5.7-5.8, Heroe_Y_Tropa §10-§11): la cadena de condiciones, el coste y el efecto. Es el único origen de
//! escuadras fuera del préstamo y el recluta de un campamento de mercenarios (`mercenarios`).
//!
//! Una escuadra por (héroe, tropa) en toda la partida: reclutar la que ya se tiene **repone** hasta el tope (`unidades`) y solo donde está.
//! Reclutar no exige cargo: manda la residencia. Todo sale del **almacén de la plaza** y de su población, no del héroe.

use crate::catalogo::{DefTropa, Origen, Tropa};
use crate::datos::Reglas;
use crate::escuadra::{Contenedor, Escuadra};
use crate::heroe::Heroe;
use crate::racion;
use crate::tipos::{AsentamientoId, ColumnaId, EscuadraId};
use asentamiento::datos::por;
use asentamiento::edificio::Edificio;
use asentamiento::{Almacen, Costo, Poblacion, Recurso, Tipo};
use base::{Instante, Mil};

/// Qué puede hacer un héroe en una plaza (`puedeReclutarEn`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permiso {
    /// Reside en ella: escuadra nueva, reposición y cualquier tropa que el edificio permita.
    Todo,
    /// Ciudadano de la Facción de la plaza, sin residir y sin veto: solo repone una escuadra que ya tiene **aquí**.
    SoloReponer,
    No,
}

/// Residir manda; si no, hay que ser de la Facción de la plaza y no estar vetado.
pub fn permiso(reside: bool, de_la_faccion: bool, vetado: bool) -> Permiso {
    if reside {
        Permiso::Todo
    } else if !de_la_faccion || vetado {
        Permiso::No
    } else {
        Permiso::SoloReponer
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorReclutamiento {
    /// Ni resides ni es una plaza de tu Facción que lo permita.
    NoPuedes,
    /// La Facción de la plaza no ha adoptado la tecnología de la tropa.
    SinTecnologia,
    /// La escuadra está reservada para una batalla: no se repone hasta que acabe.
    Reservada,
    /// Ya tienes una escuadra de esa tropa y no está aquí: solo se repone donde está (en escolta o fuera del mundo, nunca).
    YaLaTienesFuera(Contenedor),
    /// Fuera de tu residencia solo se repone lo que ya tienes aquí.
    SoloReponer,
    AlTope,
    /// El pool de reclutamiento (la población que no hace falta para producir) no llega.
    SinPoblacion { origen: Origen, hacen_falta: i64, disponibles: i64 },
    SinEdificio,
    SinRecursos,
    /// El trigo del almacén no cubre `horizonteMinutosComida` minutos de lo que se come, contando la tropa nueva.
    SinReservaDeTrigo { hacen_falta: Mil, hay: Mil },
}

/// Lo que reclutar mira de la plaza y lo que cambia en ella.
pub struct Plaza<'a> {
    pub id: AsentamientoId,
    pub edificios: &'a [Edificio],
    pub almacen: &'a mut Almacen,
    pub poblacion: &'a mut Poblacion,
    /// El pool de reclutamiento por clase (enteros): pesants y artesanos sin los que hacen falta para producir (`disponibles_para_reclutar`)
    /// y toda la nobleza.
    pub disponibles: [i64; 3],
    /// «Leva Forzosa» (ppm; `UNO` si no hay): toca solo el equipo, no el oro.
    pub factor_costo: i64,
    /// Trigo por minuto que come la población.
    pub consumo_poblacion: Mil,
    /// Soldados que ya comen del almacén: el campamento de los residentes.
    pub soldados_en_el_campamento: i64,
}

pub struct Pedido<'a> {
    pub heroe: &'a mut Heroe,
    pub tropa: Tropa,
    pub permiso: Permiso,
    /// Reside en esta plaza: reponer una escuadra del campamento exige residir aquí.
    pub reside: bool,
    pub tecnologia_adoptada: bool,
    /// A cuánto de la puerta de la plaza está una columna (`None` si no existe): una escuadra que va en una se repone con la columna a la puerta.
    pub distancia_de_columna: &'a dyn Fn(ColumnaId) -> Option<Mil>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reclutado {
    pub escuadra: EscuadraId,
    /// Soldados añadidos.
    pub cantidad: i64,
    /// `true` si la escuadra es nueva (cuenta para `reclutados.escuadrones`).
    pub nueva: bool,
    pub origen: Origen,
    /// Lo que se pagó del almacén (equipo y oro).
    pub costo: Costo,
}

/// Qué cuesta reclutar `cantidad` soldados de esta tropa en una plaza: el equipo por soldado (× «Leva Forzosa») y, salvo en el Centro Urbano,
/// el oro de su escalón más el de sus caballos (que la Leva Forzosa **no** toca: no se conscribe moneda).
pub fn costo(r: &Reglas, def: &DefTropa, cantidad: i64, factor_costo: i64) -> Costo {
    let mut c: Costo = def.costo_equipo.iter().map(|&(rec, n)| (rec, por(n * cantidad, factor_costo))).filter(|&(_, n)| n > 0).collect();
    if def.edificio != Tipo::CentroUrbano {
        let oro = (r.reclutamiento.oro_por_escalon[def.escalon as usize - 1] + def.caballos * r.reclutamiento.oro_por_caballo) * cantidad;
        if oro > 0 {
            c.push((Recurso::Oro, oro));
        }
    }
    c.sort();
    c
}

/// Trigo que la plaza no puede tocar: lo que come su gente durante `horizonteMinutosComida` minutos (población + tropas, con la que se va a
/// reclutar ya sumada).
pub fn reserva_de_trigo(r: &Reglas, consumo_poblacion: Mil, soldados: i64) -> Mil {
    (consumo_poblacion + racion::racion_guarnicion(r, soldados)) * r.reclutamiento.horizonte_comida
}

/// `reclutarTropa`: comprueba las condiciones en su orden y, si todas se cumplen, paga del almacén, resta la población de su clase y añade la
/// escuadra (o repone la que hay: moral, nivel y experiencia no cambian). Un rechazo no deja rastro y no gasta ids (`nuevo_id` solo se llama
/// al crear una escuadra).
pub fn reclutar(r: &Reglas, plaza: Plaza, pedido: Pedido, nuevo_id: impl FnOnce() -> EscuadraId, ahora: Instante) -> Result<Reclutado, ErrorReclutamiento> {
    let Pedido { heroe, tropa, permiso, reside, tecnologia_adoptada, distancia_de_columna } = pedido;
    let def = r.tropas.tropa(tropa);
    if permiso == Permiso::No {
        return Err(ErrorReclutamiento::NoPuedes);
    }
    if !tecnologia_adoptada {
        return Err(ErrorReclutamiento::SinTecnologia);
    }
    let existente = heroe.escuadra_de(tropa);
    if let Some(e) = existente {
        if e.reserva_batalla.is_some() {
            return Err(ErrorReclutamiento::Reservada);
        }
        let aqui = match e.contenedor {
            Contenedor::Campamento => reside,
            Contenedor::Ejercito(c) => distancia_de_columna(c).is_some_and(|d| d <= r.movimiento.radio_puerta),
            Contenedor::Escolta(_) | Contenedor::Fuera => false,
        };
        if !aqui {
            return Err(ErrorReclutamiento::YaLaTienesFuera(e.contenedor));
        }
    } else if permiso == Permiso::SoloReponer {
        return Err(ErrorReclutamiento::SoloReponer);
    }
    let cantidad = def.unidades - existente.map_or(0, |e| e.cantidad);
    if cantidad <= 0 {
        return Err(ErrorReclutamiento::AlTope);
    }
    let origen = def.origen();
    let disponibles = plaza.disponibles[origen as usize];
    if disponibles < cantidad {
        return Err(ErrorReclutamiento::SinPoblacion { origen, hacen_falta: cantidad, disponibles });
    }
    let edificio_basta = plaza.edificios.iter().find(|e| e.tipo == def.edificio && e.activo()).is_some_and(|e| e.nivel_interno >= def.nivel_requerido);
    if !edificio_basta {
        return Err(ErrorReclutamiento::SinEdificio);
    }
    let costo = costo(r, def, cantidad, plaza.factor_costo);
    if !plaza.almacen.tiene(&costo) {
        return Err(ErrorReclutamiento::SinRecursos);
    }
    let hacen_falta = reserva_de_trigo(r, plaza.consumo_poblacion, plaza.soldados_en_el_campamento + cantidad);
    let hay = plaza.almacen.cantidad(Recurso::Trigo);
    if hay < hacen_falta {
        return Err(ErrorReclutamiento::SinReservaDeTrigo { hacen_falta, hay });
    }

    // Todo en regla: ahora sí se cambia el mundo.
    assert!(plaza.almacen.pagar(&costo), "el coste se comprobó");
    let clase = match origen {
        Origen::Pesants => &mut plaza.poblacion.pesants,
        Origen::Artesanos => &mut plaza.poblacion.artesanos,
        Origen::Nobleza => &mut plaza.poblacion.nobleza,
    };
    *clase -= cantidad * 1_000;
    let (escuadra, nueva) = match heroe.escuadras.iter_mut().find(|e| e.tropa == tropa && e.prestada.is_none()) {
        Some(e) => {
            e.cantidad += cantidad;
            (e.id, false)
        }
        None => {
            let id = nuevo_id();
            heroe.escuadras.push(Escuadra::nueva(r, id, heroe.id, tropa, cantidad, Some(plaza.id), ahora));
            (id, true)
        }
    };
    Ok(Reclutado { escuadra, cantidad, nueva, origen, costo })
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::heroe::{Controlador, Residencia};
    use asentamiento::datos::UNO;
    use asentamiento::edificio::Estado;

    fn r() -> Reglas {
        Reglas::cargar()
    }

    fn edificio(id: u32, tipo: Tipo, nivel: u8) -> Edificio {
        Edificio { nivel_interno: nivel, ..Edificio::nuevo(id, tipo, Estado::Activo) }
    }

    /// Una plaza con Centro Urbano, Barracón 2, Galería 1 y Caballerizas 2; todo el equipo y 10.000 de trigo.
    struct Mundo {
        edificios: Vec<Edificio>,
        almacen: Almacen,
        poblacion: Poblacion,
        heroe: Heroe,
        disponibles: [i64; 3],
        factor_costo: i64,
        consumo_poblacion: Mil,
        soldados: i64,
        adoptada: bool,
        columnas: Vec<(ColumnaId, Mil)>,
        reside: bool,
        permiso: Permiso,
    }

    fn mundo() -> Mundo {
        let r = r();
        let mut almacen = Almacen::nuevo(1_000_000_000);
        for rec in asentamiento::recurso::TODOS {
            almacen.poner(rec, 10_000_000);
        }
        let mut heroe = Heroe::nuevo(&r, 1, Controlador::Humano, 1, 1);
        heroe.residencia = Some(Residencia::Asentamiento(4));
        Mundo {
            edificios: vec![edificio(0, Tipo::CentroUrbano, 1), edificio(1, Tipo::Barracon, 2), edificio(2, Tipo::GaleriaDeTiro, 1), edificio(3, Tipo::Caballerizas, 2)],
            almacen,
            poblacion: Poblacion { pesants: 200_000, artesanos: 100_000, nobleza: 50_000, nutricion: 100_000 },
            heroe,
            disponibles: [200, 100, 50],
            factor_costo: UNO,
            consumo_poblacion: 2_000,
            soldados: 0,
            adoptada: true,
            columnas: vec![],
            reside: true,
            permiso: Permiso::Todo,
        }
    }

    fn reclutar_de(m: &mut Mundo, tropa: Tropa) -> Result<Reclutado, ErrorReclutamiento> {
        let r = r();
        let columnas = m.columnas.clone();
        let busca = move |c: ColumnaId| columnas.iter().find(|(k, _)| *k == c).map(|x| x.1);
        let plaza = Plaza {
            id: 4,
            edificios: &m.edificios,
            almacen: &mut m.almacen,
            poblacion: &mut m.poblacion,
            disponibles: m.disponibles,
            factor_costo: m.factor_costo,
            consumo_poblacion: m.consumo_poblacion,
            soldados_en_el_campamento: m.soldados,
        };
        let pedido = Pedido { heroe: &mut m.heroe, tropa, permiso: m.permiso, reside: m.reside, tecnologia_adoptada: m.adoptada, distancia_de_columna: &busca };
        reclutar(&r, plaza, pedido, || 77, 5_000)
    }

    #[test]
    fn los_permisos_residir_manda_luego_la_faccion_y_el_veto() {
        assert_eq!(permiso(true, false, true), Permiso::Todo, "un residente recluta aunque no lo parezca");
        assert_eq!(permiso(false, true, false), Permiso::SoloReponer);
        assert_eq!(permiso(false, false, false), Permiso::No, "otra Facción no recluta");
        assert_eq!(permiso(false, true, true), Permiso::No, "el veto");
    }

    #[test]
    fn la_milicia_del_centro_urbano_cuesta_solo_equipo_sin_oro() {
        let r = r();
        let c = costo(&r, r.tropas.tropa(Tropa::MiliciaLanceros), 25, UNO);
        assert_eq!(c, vec![(Recurso::Madera, 50_000)], "2 de madera × 25: 50 madera");
        assert_eq!(costo(&r, r.tropas.tropa(Tropa::Mineros), 30, UNO), vec![(Recurso::Piedra, 30_000)], "los mineros: 1 de piedra por soldado, sin oro");
        assert_eq!(costo(&r, r.tropas.tropa(Tropa::Granjeros), 30, UNO), Costo::new(), "los granjeros no cuestan nada");
    }

    #[test]
    fn el_oro_va_por_escalon_y_soldado_y_la_caballeria_paga_cinco_por_caballo() {
        let r = r();
        // Lanceros de mimbre (barracón, escalón 2): 1 arma de madera + 2 de oro por soldado; 25 soldados.
        assert_eq!(costo(&r, r.tropas.tropa(Tropa::LancerosMimbre), 25, UNO), vec![(Recurso::Oro, 50_000), (Recurso::ArmaMadera, 25_000)]);
        // Carros de guerra (escalón 4: 7 de oro + 2 caballos × 5): 15 soldados → 17 × 15 = 255 de oro.
        let carros = costo(&r, r.tropas.tropa(Tropa::CarrosGuerra), 15, UNO);
        assert!(carros.contains(&(Recurso::Oro, 255_000)));
        assert!(carros.contains(&(Recurso::ArmaBronce, 15_000)) && carros.contains(&(Recurso::CarroGuerra, 15_000)));
        // Jinetes asirios (escalón 2: 2 de oro + 1 caballo): 7 por soldado, 20 soldados.
        assert!(costo(&r, r.tropas.tropa(Tropa::JinetesAsirios), 20, UNO).contains(&(Recurso::Oro, 140_000)));
    }

    #[test]
    fn la_leva_forzosa_rebaja_el_equipo_pero_no_el_oro() {
        let r = r();
        let normal = costo(&r, r.tropas.tropa(Tropa::EspadachinesBronce), 18, UNO);
        let leva = costo(&r, r.tropas.tropa(Tropa::EspadachinesBronce), 18, 700_000);
        // 2 armas de bronce + 1 armadura intermedia por soldado, 18 soldados: 36 y 18; con 0,7: 25,2 y 12,6. Oro: escalón 3 → 4 × 18 = 72, intacto.
        assert!(normal.contains(&(Recurso::ArmaBronce, 36_000)) && normal.contains(&(Recurso::ArmaduraIntermedia, 18_000)));
        assert!(leva.contains(&(Recurso::ArmaBronce, 25_200)) && leva.contains(&(Recurso::ArmaduraIntermedia, 12_600)));
        assert!(normal.contains(&(Recurso::Oro, 72_000)) && leva.contains(&(Recurso::Oro, 72_000)), "no se conscribe moneda");
    }

    #[test]
    fn recluta_una_escuadra_nueva_paga_resta_poblacion_y_la_deja_en_el_campamento() {
        let mut m = mundo();
        let trigo0 = m.almacen.cantidad(Recurso::Trigo);
        let res = reclutar_de(&mut m, Tropa::MiliciaLanceros).unwrap();
        assert_eq!((res.escuadra, res.cantidad, res.nueva, res.origen), (77, 25, true, Origen::Pesants));
        assert_eq!(res.costo, vec![(Recurso::Madera, 50_000)]);
        assert_eq!(m.almacen.cantidad(Recurso::Madera), 10_000_000 - 50_000);
        assert_eq!(m.almacen.cantidad(Recurso::Trigo), trigo0, "reclutar no gasta el trigo: solo exige la reserva");
        assert_eq!(m.poblacion.pesants, 200_000 - 25_000);
        let e = m.heroe.escuadra(77).unwrap();
        assert_eq!((e.cantidad, e.nivel, e.experiencia, e.contenedor, e.en_guarnicion, e.plaza_origen), (25, 1, 0, Contenedor::Campamento, false, Some(4)));
        assert_eq!(e.moral_en(5_000), 100_000);
        assert_eq!(e.tropa, Tropa::MiliciaLanceros);
    }

    #[test]
    fn la_poblacion_sale_de_la_clase_del_escalon() {
        let mut m = mundo();
        reclutar_de(&mut m, Tropa::HacherosLigeros).unwrap(); // escalón 3: 18 artesanos
        assert_eq!((m.poblacion.pesants, m.poblacion.artesanos, m.poblacion.nobleza), (200_000, 100_000 - 18_000, 50_000));
        reclutar_de(&mut m, Tropa::CarrosGuerra).unwrap(); // escalón 4: 15 nobles
        assert_eq!(m.poblacion.nobleza, 50_000 - 15_000);
    }

    #[test]
    fn reclutar_de_nuevo_repone_hasta_el_tope_sin_cambiar_moral_nivel_ni_experiencia() {
        let mut m = mundo();
        reclutar_de(&mut m, Tropa::MiliciaLanceros).unwrap();
        {
            let e = m.heroe.escuadra_mut(77).unwrap();
            e.cantidad = 10;
            e.nivel = 4;
            e.experiencia = 40_000;
            e.moral = base::Ritmo::constante(30_000, 0);
        }
        let res = reclutar_de(&mut m, Tropa::MiliciaLanceros).unwrap();
        assert_eq!((res.cantidad, res.nueva, res.escuadra), (15, false, 77), "añade los 15 que faltan, a su coste");
        assert_eq!(res.costo, vec![(Recurso::Madera, 30_000)]);
        let e = m.heroe.escuadra(77).unwrap();
        assert_eq!((e.cantidad, e.nivel, e.experiencia, e.moral_en(0), m.heroe.escuadras.len()), (25, 4, 40_000, 30_000, 1));
        assert_eq!(reclutar_de(&mut m, Tropa::MiliciaLanceros), Err(ErrorReclutamiento::AlTope));
    }

    #[test]
    fn una_escuadra_reservada_no_se_repone() {
        let mut m = mundo();
        reclutar_de(&mut m, Tropa::MiliciaLanceros).unwrap();
        m.heroe.escuadra_mut(77).unwrap().cantidad = 5;
        m.heroe.escuadra_mut(77).unwrap().reserva_batalla = Some(3);
        assert_eq!(reclutar_de(&mut m, Tropa::MiliciaLanceros), Err(ErrorReclutamiento::Reservada));
    }

    #[test]
    fn solo_se_repone_donde_esta_la_escuadra() {
        let mut m = mundo();
        reclutar_de(&mut m, Tropa::MiliciaLanceros).unwrap();
        m.heroe.escuadra_mut(77).unwrap().cantidad = 5;
        // En una columna: solo con la columna a la puerta (≤ 2).
        m.heroe.escuadra_mut(77).unwrap().contenedor = Contenedor::Ejercito(9);
        assert_eq!(reclutar_de(&mut m, Tropa::MiliciaLanceros), Err(ErrorReclutamiento::YaLaTienesFuera(Contenedor::Ejercito(9))), "la columna ni existe");
        m.columnas = vec![(9, 2_001)];
        assert!(matches!(reclutar_de(&mut m, Tropa::MiliciaLanceros), Err(ErrorReclutamiento::YaLaTienesFuera(_))));
        m.columnas = vec![(9, 2_000)];
        assert_eq!(reclutar_de(&mut m, Tropa::MiliciaLanceros).map(|x| x.cantidad), Ok(20), "con la columna a la puerta sí");
        // En escolta o fuera del mundo: nunca. En el campamento de otra residencia: tampoco.
        m.heroe.escuadra_mut(77).unwrap().cantidad = 5;
        for c in [Contenedor::Escolta(3), Contenedor::Fuera] {
            m.heroe.escuadra_mut(77).unwrap().contenedor = c;
            assert_eq!(reclutar_de(&mut m, Tropa::MiliciaLanceros), Err(ErrorReclutamiento::YaLaTienesFuera(c)));
        }
        m.heroe.escuadra_mut(77).unwrap().contenedor = Contenedor::Campamento;
        m.reside = false;
        assert!(matches!(reclutar_de(&mut m, Tropa::MiliciaLanceros), Err(ErrorReclutamiento::YaLaTienesFuera(Contenedor::Campamento))));
    }

    #[test]
    fn un_ciudadano_que_no_reside_solo_repone_y_nunca_crea_una_escuadra_nueva() {
        let mut m = mundo();
        m.permiso = Permiso::SoloReponer;
        m.reside = false;
        assert_eq!(reclutar_de(&mut m, Tropa::MiliciaLanceros), Err(ErrorReclutamiento::SoloReponer));
        // Su columna a la puerta sí se repone.
        let mut e = Escuadra::nueva(&r(), 50, 1, Tropa::MiliciaLanceros, 10, None, 0);
        e.contenedor = Contenedor::Ejercito(9);
        m.heroe.escuadras.push(e);
        m.columnas = vec![(9, 500)];
        assert_eq!(reclutar_de(&mut m, Tropa::MiliciaLanceros).map(|x| (x.escuadra, x.cantidad, x.nueva)), Ok((50, 15, false)));
        m.permiso = Permiso::No;
        assert_eq!(reclutar_de(&mut m, Tropa::Lenadores), Err(ErrorReclutamiento::NoPuedes));
    }

    #[test]
    fn las_condiciones_en_su_orden() {
        // 1. Tecnología (antes que el pool, el edificio o el coste).
        let mut m = mundo();
        m.adoptada = false;
        m.disponibles = [0; 3];
        assert_eq!(reclutar_de(&mut m, Tropa::ArquerosCompuesto), Err(ErrorReclutamiento::SinTecnologia));
        // 2. Pool de población.
        m.adoptada = true;
        assert_eq!(reclutar_de(&mut m, Tropa::MiliciaLanceros), Err(ErrorReclutamiento::SinPoblacion { origen: Origen::Pesants, hacen_falta: 25, disponibles: 0 }));
        // 3. Edificio y nivel (la Galería es de nivel 1 y el arco compuesto pide 3; no hay otro con más).
        m.disponibles = [200, 100, 50];
        assert_eq!(reclutar_de(&mut m, Tropa::ArquerosCompuesto), Err(ErrorReclutamiento::SinEdificio));
        m.edificios.retain(|e| e.tipo != Tipo::Barracon);
        assert_eq!(reclutar_de(&mut m, Tropa::LancerosMimbre), Err(ErrorReclutamiento::SinEdificio), "sin Barracón no se recluta lo suyo");
        // 4. Coste.
        let mut m = mundo();
        m.almacen.poner(Recurso::Oro, 10_000);
        assert_eq!(reclutar_de(&mut m, Tropa::LancerosMimbre), Err(ErrorReclutamiento::SinRecursos));
        // 5. Reserva de trigo.
        let mut m = mundo();
        m.almacen.poner(Recurso::Trigo, 1_000);
        assert!(matches!(reclutar_de(&mut m, Tropa::MiliciaLanceros), Err(ErrorReclutamiento::SinReservaDeTrigo { hay: 1_000, .. })));
        assert_eq!(m.heroe.escuadras.len(), 0, "un rechazo no deja rastro");
        assert_eq!(m.poblacion.pesants, 200_000);
        assert_eq!(m.almacen.cantidad(Recurso::Madera), 10_000_000);
    }

    #[test]
    fn la_reserva_de_trigo_cuenta_la_poblacion_el_campamento_y_la_tropa_nueva_ocho_minutos() {
        let r = r();
        // Población 2/min, campamento de 100 soldados (15/min) y 25 nuevos (3,75/min): 20,75 × 8 = 166.
        assert_eq!(reserva_de_trigo(&r, 2_000, 125), 166_000);
        let mut m = mundo();
        m.soldados = 100;
        m.almacen.poner(Recurso::Trigo, 165_999);
        assert_eq!(reclutar_de(&mut m, Tropa::MiliciaLanceros), Err(ErrorReclutamiento::SinReservaDeTrigo { hacen_falta: 166_000, hay: 165_999 }));
        m.almacen.poner(Recurso::Trigo, 166_000);
        assert!(reclutar_de(&mut m, Tropa::MiliciaLanceros).is_ok(), "justo en la reserva sí");
    }

    #[test]
    fn el_id_solo_se_gasta_al_crear_una_escuadra() {
        let mut m = mundo();
        let r = r();
        let mut llamadas = 0;
        let busca = |_: ColumnaId| -> Option<Mil> { None };
        let plaza = Plaza {
            id: 4,
            edificios: &m.edificios,
            almacen: &mut m.almacen,
            poblacion: &mut m.poblacion,
            disponibles: [0; 3],
            factor_costo: UNO,
            consumo_poblacion: 0,
            soldados_en_el_campamento: 0,
        };
        let pedido = Pedido { heroe: &mut m.heroe, tropa: Tropa::MiliciaLanceros, permiso: Permiso::Todo, reside: true, tecnologia_adoptada: true, distancia_de_columna: &busca };
        assert!(reclutar(&r, plaza, pedido, || { llamadas += 1; 1 }, 0).is_err());
        assert_eq!(llamadas, 0);
    }
}
