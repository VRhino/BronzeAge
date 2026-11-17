//! La bodega y la prensa trabajan una sola receta (`TALLERES_DE_UNA_RECETA`, 2026-11-09): receta de partida, tope de edificios por plaza,
//! elección con tope de recetas distintas por nivel y recetas paradas cuando la plaza baja de nivel.

use asentamiento::asentamiento::{Asentamiento, Exterior, SitiosLibres};
use asentamiento::catalogo::Tipo;
use asentamiento::datos::{Reglas, UNO};
use asentamiento::edificio::{Edificio, Estado};
use asentamiento::obras::ErrorObra;
use asentamiento::produccion::Fuentes;
use asentamiento::recetas_unicas::{insumo_activo, ErrorReceta};
use asentamiento::recurso::Recurso::{self, *};
use base::Mil;

struct SinMundo;
impl Fuentes for SinMundo {
    fn densidad_parcela(&self, _: u32) -> i64 {
        0
    }
    fn extraer(&mut self, _: u32, _: Mil) -> Mil {
        0
    }
}

macro_rules! exterior {
    ($r:expr, $sitios:expr, $mundo:expr) => {
        Exterior {
            reglas: $r,
            fuentes: &mut $mundo,
            sitios: &mut $sitios,
            fertilidad: UNO,
            tiene_tecnologia: &|_| false,
            factor_tecnologia: &|_| UNO,
            nivel_faccion: 1,
            maravilla_desbloqueada: false,
            estatuas_permitidas: 0,
            es_capital: true,
            faccion_tiene_gran_fundicion: false,
            ciudadanos: 0,
            distancia_capital: 0,
            racion_guarnicion: 0,
            felicidad_extra: UNO,
            nobleza_extra: UNO,
            recaudacion_extra: UNO,
            cupo_para_recuperar: true,
        }
    };
}

/// Una plaza rica del nivel dado (alcanzado y operativo) con las bodegas y prensas que se pidan, activas y con su insumo (`None` = sin elegir).
fn plaza(r: &Reglas, nivel: u32, talleres: &[(u32, Tipo, Option<Recurso>)]) -> Asentamiento {
    let mut a = Asentamiento::fundar(r, 0, &[], 3);
    for &rec in asentamiento::recurso::TODOS.iter() {
        a.almacen.ampliar(rec, 10_000_000);
        a.almacen.poner(rec, 5_000_000);
    }
    a.nivel = nivel;
    a.medidor.nivel_actual = nivel;
    for &(id, tipo, receta) in talleres {
        a.edificios.push(Edificio { receta, ..Edificio::nuevo(id, tipo, Estado::Activo) });
    }
    a
}

fn bodegas(r: &Reglas, nivel: u32, insumos: &[Recurso]) -> Asentamiento {
    let talleres: Vec<_> = insumos.iter().enumerate().map(|(k, &i)| (100 + k as u32, Tipo::Bodega, Some(i))).collect();
    plaza(r, nivel, &talleres)
}

fn activo(a: &Asentamiento, r: &Reglas, id: u32) -> Option<Recurso> {
    insumo_activo(&r.catalogo, a.edificio(id).unwrap())
}

#[test]
fn sin_elegir_la_bodega_y_la_prensa_trabajan_la_primera_receta_del_catalogo() {
    let r = Reglas::cargar();
    let a = plaza(&r, 5, &[(100, Tipo::Bodega, None), (101, Tipo::Prensa, None), (102, Tipo::Granja, None)]);
    assert_eq!([100, 101, 102].map(|id| activo(&a, &r, id)), [Some(Uva), Some(Aceituna), None], "la granja no elige receta");
    let b = plaza(&r, 5, &[(100, Tipo::Bodega, Some(Cebada)), (101, Tipo::Prensa, Some(Sesamo))]);
    assert_eq!([100, 101].map(|id| activo(&b, &r, id)), [Some(Cebada), Some(Sesamo)]);
}

#[test]
fn una_plaza_admite_tres_bodegas_y_tres_prensas_contando_todos_los_estados() {
    let r = Reglas::cargar();
    let (mut a, mut s, mut w) = (plaza(&r, 5, &[]), SitiosLibres, SinMundo);
    let mut x = exterior!(&r, s, w);
    for tipo in [Tipo::Bodega, Tipo::Prensa] {
        for _ in 0..3 {
            a.anadir_manual(0, &r, &mut x, tipo, &mut vec![]).unwrap();
        }
        assert_eq!(a.anadir_manual(0, &r, &mut x, tipo, &mut vec![]), Err(ErrorObra::TopePorPlaza), "{tipo:?}: la cuarta, no (las de la cola cuentan)");
    }
}

#[test]
fn el_edificio_nuevo_nace_con_la_receta_de_la_plaza_o_con_la_primera_del_catalogo() {
    let r = Reglas::cargar();
    let (mut s, mut w) = (SitiosLibres, SinMundo);
    let mut a = plaza(&r, 5, &[]);
    let mut x = exterior!(&r, s, w);
    let primera = a.anadir_manual(0, &r, &mut x, Tipo::Bodega, &mut vec![]).unwrap();
    assert_eq!(a.edificio(primera).unwrap().receta, Some(Uva), "sin ninguna, la primera del catálogo");
    a.elegir_receta(&r, primera, Cebada).unwrap();
    let segunda = a.anadir_manual(0, &r, &mut x, Tipo::Bodega, &mut vec![]).unwrap();
    assert_eq!(a.edificio(segunda).unwrap().receta, Some(Cebada), "la receta que la plaza trabaja");
    let prensa = a.anadir_manual(0, &r, &mut x, Tipo::Prensa, &mut vec![]).unwrap();
    assert_eq!(a.edificio(prensa).unwrap().receta, Some(Aceituna), "cada tipo, con la suya");
    assert_eq!(a.anadir_manual(0, &r, &mut x, Tipo::Molino, &mut vec![]).map(|id| a.edificio(id).unwrap().receta), Ok(None), "un molino no elige");
}

#[test]
fn la_receta_de_partida_salta_las_paradas_y_toma_la_del_menor_id_que_trabaja() {
    let r = Reglas::cargar();
    // En el nivel 4 solo cabe una bebida: la cerveza (id 100, la primera en usarse) trabaja y el vino (101) está parado.
    let (mut s, mut w) = (SitiosLibres, SinMundo);
    let mut a = bodegas(&r, 4, &[Cebada, Uva]);
    let mut x = exterior!(&r, s, w);
    let id = a.anadir_manual(0, &r, &mut x, Tipo::Bodega, &mut vec![]).unwrap();
    assert_eq!(a.edificio(id).unwrap().receta, Some(Cebada));
}

#[test]
fn elegir_receta_rechaza_lo_que_no_es_de_una_bodega_o_una_prensa() {
    let r = Reglas::cargar();
    let mut a = plaza(&r, 5, &[(100, Tipo::Bodega, None), (101, Tipo::Prensa, None), (102, Tipo::Molino, None)]);
    assert_eq!(a.elegir_receta(&r, 999, Uva), Err(ErrorReceta::NoExiste));
    assert_eq!(a.elegir_receta(&r, 102, Trigo), Err(ErrorReceta::NoSeElige), "el molino no elige");
    assert_eq!(a.elegir_receta(&r, 100, Aceituna), Err(ErrorReceta::Desconocida), "la bodega no consume aceituna");
    assert_eq!(a.elegir_receta(&r, 101, Cebada), Err(ErrorReceta::Desconocida));
    assert_eq!(a.elegir_receta(&r, 100, Uva), Err(ErrorReceta::SinCambios), "la receta de partida cuenta como la actual");
    assert_eq!(a.elegir_receta(&r, 100, Cebada), Ok(()));
    assert_eq!(a.edificio(100).unwrap().receta, Some(Cebada));
    assert_eq!(a.elegir_receta(&r, 100, Cebada), Err(ErrorReceta::SinCambios));
}

#[test]
fn la_bodega_admite_una_bebida_distinta_en_los_niveles_3_y_4_y_dos_en_el_5() {
    let r = Reglas::cargar();
    for nivel in [3, 4] {
        let mut a = bodegas(&r, nivel, &[Uva, Uva]);
        assert_eq!(a.elegir_receta(&r, 101, Cebada), Err(ErrorReceta::TopePorPlaza), "nivel {nivel}: vino y cerveza a la vez, no");
        assert_eq!(a.elegir_receta(&r, 101, Uva), Err(ErrorReceta::SinCambios));
        // Cambiar la única bodega de una receta a otra sí cabe: sigue habiendo una sola bebida.
        let mut sola = bodegas(&r, nivel, &[Uva]);
        assert_eq!(sola.elegir_receta(&r, 100, Cebada), Ok(()));
        // Y todas a la vez, de una en una, no: la primera que se cambia ya reparte dos bebidas.
        assert_eq!(a.elegir_receta(&r, 100, Cebada), Err(ErrorReceta::TopePorPlaza));
    }
    let mut a = bodegas(&r, 5, &[Uva, Uva, Uva]);
    assert_eq!(a.elegir_receta(&r, 101, Cebada), Ok(()), "en el nivel 5, vino y cerveza");
    assert_eq!(a.elegir_receta(&r, 102, Cebada), Ok(()));
    assert!(a.recetas_paradas(&r).is_empty());
}

#[test]
fn la_prensa_trabaja_siempre_una_sola_fuente_de_aceite() {
    let r = Reglas::cargar();
    for nivel in [4, 5] {
        let mut a = plaza(&r, nivel, &[(100, Tipo::Prensa, None), (101, Tipo::Prensa, None)]);
        assert_eq!(a.elegir_receta(&r, 101, Sesamo), Err(ErrorReceta::TopePorPlaza), "nivel {nivel}: aceituna y sésamo a la vez, nunca");
        let mut sola = plaza(&r, nivel, &[(100, Tipo::Prensa, None)]);
        assert_eq!(sola.elegir_receta(&r, 100, Sesamo), Ok(()));
    }
}

#[test]
fn el_tope_se_comprueba_con_los_edificios_de_cualquier_estado() {
    let r = Reglas::cargar();
    let mut a = bodegas(&r, 4, &[Uva]);
    a.edificios.push(Edificio { receta: Some(Cebada), ..Edificio::nuevo(101, Tipo::Bodega, Estado::EnCola) });
    // La cerveza ya está en una bodega que espera en la cola (en el nivel 4 sobra una de las dos).
    assert_eq!(a.recetas_paradas(&r), vec![101]);
    assert_eq!(a.elegir_receta(&r, 100, Cebada), Ok(()), "cambiar la única de vino a cerveza deja una sola bebida");
    assert!(a.recetas_paradas(&r).is_empty());
}

#[test]
fn al_bajar_de_nivel_paran_las_recetas_que_sobran_y_vuelven_al_recuperarlo() {
    let r = Reglas::cargar();
    // Vino en 100 y 102, cerveza en 101: por el menor id, el vino va primero. En el nivel 5 trabajan las dos.
    let mut a = bodegas(&r, 5, &[Uva, Cebada, Uva]);
    assert!(a.recetas_paradas(&r).is_empty());
    a.medidor.nivel_actual = 4;
    assert_eq!(a.recetas_paradas(&r), vec![101], "queda el vino (el menor id, 100); la cerveza para");
    // El orden lo da el menor id de cada insumo, no el recurso.
    let mut b = bodegas(&r, 5, &[Cebada, Uva, Cebada]);
    b.medidor.nivel_actual = 4;
    assert_eq!(b.recetas_paradas(&r), vec![101], "ahora trabaja la cerveza y para el vino");
    // Elegir otra receta resuelve la parada.
    assert_eq!(a.elegir_receta(&r, 101, Uva), Ok(()));
    assert!(a.recetas_paradas(&r).is_empty());
    // Recuperar el nivel las devuelve sin tocar nada guardado.
    let mut c = bodegas(&r, 5, &[Uva, Cebada]);
    c.medidor.nivel_actual = 3;
    assert_eq!(c.recetas_paradas(&r), vec![101]);
    c.medidor.nivel_actual = 5;
    assert!(c.recetas_paradas(&r).is_empty());
}

#[test]
fn cada_tipo_cuenta_sus_propias_recetas_paradas() {
    let r = Reglas::cargar();
    // En el nivel 4 cada tipo deja una sola receta, la del menor id; la bodega y la prensa no se mezclan.
    let mut a = plaza(&r, 5, &[(10, Tipo::Bodega, Some(Cebada)), (11, Tipo::Bodega, Some(Uva)), (12, Tipo::Prensa, Some(Sesamo)), (13, Tipo::Prensa, Some(Aceituna))]);
    a.medidor.nivel_actual = 4;
    assert_eq!(a.recetas_paradas(&r), vec![11, 13], "bodega: para el vino; prensa: para la aceituna (el sésamo tiene el menor id)");
}

#[test]
fn bajar_de_nivel_no_cambia_la_receta_elegida_ni_la_proyecta_distinta() {
    let r = Reglas::cargar();
    let mut a = bodegas(&r, 5, &[Uva, Cebada]);
    a.medidor.nivel_actual = 4;
    assert_eq!(a.edificio(101).unwrap().receta, Some(Cebada), "parada no es cambiada: nada se borra");
}
