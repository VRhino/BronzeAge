//! Las órdenes de los jugadores sobre un asentamiento: cola manual, reembolsos, mejoras y bloqueos del ascenso.

use asentamiento::asentamiento::{Asentamiento, Evento, Exterior, SitiosLibres};
use asentamiento::ascenso::Bloqueo;
use asentamiento::catalogo::Tipo;
use asentamiento::datos::{Reglas, UNO};
use asentamiento::edificio::{Edificios, Estado};
use asentamiento::obras::ErrorObra;
use asentamiento::produccion::Fuentes;
use asentamiento::recurso::Recurso;
use base::{Mil, MS_POR_MINUTO};

struct SinMundo;
impl Fuentes for SinMundo {
    fn densidad_parcela(&self, _: u32) -> i64 {
        0
    }
    fn extraer(&mut self, _: u32, _: Mil) -> Mil {
        0
    }
}

/// Un asentamiento rico para probar órdenes sin esperar a la economía.
fn rico(r: &Reglas) -> Asentamiento {
    let mut a = Asentamiento::fundar(r, 0, &[], 3);
    for &rec in asentamiento::recurso::TODOS.iter() {
        a.almacen.ampliar(rec, 10_000_000);
        a.almacen.poner(rec, 5_000_000);
    }
    a
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
            orden_era: 1,
            es_capital: true,
            faccion_tiene_gran_fundicion: false,
            ciudadanos: 0,
            distancia_capital: 0,
            recinto_nivel: 0,
            racion_guarnicion: 0,
            felicidad_extra: UNO,
            nobleza_extra: UNO,
            recaudacion_extra: UNO,
        }
    };
}

#[test]
fn la_cola_manual_paga_al_encolar_respeta_los_topes_y_se_reembolsa_entera_al_quitar() {
    let r = Reglas::cargar();
    let (mut a, mut s, mut w) = (rico(&r), SitiosLibres, SinMundo);
    let mut x = exterior!(&r, s, w);
    let antes = a.almacen.cantidad(Recurso::Madera);
    let id = a.anadir_manual(&r, &mut x, Tipo::Vivienda).unwrap();
    assert_eq!(a.almacen.cantidad(Recurso::Madera), antes - 10_000, "paga los 10 de madera al encolar");
    assert_eq!(a.edificio(id).unwrap().prioridad, 900, "banda «manual»");
    assert_eq!(a.anadir_manual(&r, &mut x, Tipo::CentroUrbano), Err(ErrorObra::TipoNoManual));
    assert_eq!(a.anadir_manual(&r, &mut x, Tipo::Palacio), Err(ErrorObra::NivelInsuficiente), "el Palacio pide nivel 2");
    assert_eq!(a.anadir_manual(&r, &mut x, Tipo::Maravilla), Err(ErrorObra::EraInsuficiente));
    a.quitar_de_cola(&r, id).unwrap();
    assert_eq!(a.almacen.cantidad(Recurso::Madera), antes, "se devuelve el coste completo");
    assert!(a.edificio(id).is_none());
    assert_eq!(a.quitar_de_cola(&r, id), Err(ErrorObra::NoExiste));
    // Cola de cuatro: la quinta se rechaza.
    for _ in 0..4 {
        a.anadir_manual(&r, &mut x, Tipo::Granja).unwrap();
    }
    assert_eq!(a.anadir_manual(&r, &mut x, Tipo::Granja), Err(ErrorObra::ColaLlena));
}

#[test]
fn los_tipos_unicos_y_los_topes_de_nivel_se_hacen_cumplir() {
    let r = Reglas::cargar();
    let (mut a, mut s, mut w) = (rico(&r), SitiosLibres, SinMundo);
    a.medidor.nivel_actual = 2;
    a.nivel = 2;
    let mut x = exterior!(&r, s, w);
    a.anadir_manual(&r, &mut x, Tipo::Taberna).unwrap();
    assert_eq!(a.anadir_manual(&r, &mut x, Tipo::Taberna), Err(ErrorObra::Unico));
    assert_eq!(a.anadir_manual(&r, &mut x, Tipo::GranFundicion), Err(ErrorObra::NivelInsuficiente), "pide capital de nivel 4");
    // Tres Fundiciones en el nivel 2; la cuarta choca con el tope de transformación.
    for _ in 0..3 {
        let id = a.anadir_manual(&r, &mut x, Tipo::Fundicion).unwrap();
        a.edificios.iter_mut().find(|e| e.id == id).unwrap().estado = Estado::EnConstruccion; // libera el cupo de la cola
    }
    assert_eq!(a.anadir_manual(&r, &mut x, Tipo::Fundicion), Err(ErrorObra::TopeTransformacion));
}

#[test]
fn mover_en_cola_intercambia_prioridades_con_el_vecino() {
    let r = Reglas::cargar();
    let (mut a, mut s, mut w) = (rico(&r), SitiosLibres, SinMundo);
    let mut x = exterior!(&r, s, w);
    let ids: Vec<u32> = (0..3).map(|_| a.anadir_manual(&r, &mut x, Tipo::Granja).unwrap()).collect();
    for (k, &id) in ids.iter().enumerate() {
        a.edificios.iter_mut().find(|e| e.id == id).unwrap().prioridad = 900 - k as i64; // orden: 0, 1, 2
    }
    a.mover_en_cola(ids[2], true).unwrap();
    let prioridad = |a: &Asentamiento, id| a.edificio(id).unwrap().prioridad;
    assert_eq!((prioridad(&a, ids[2]), prioridad(&a, ids[1])), (899, 898), "sube una posición y el vecino baja");
    a.mover_en_cola(ids[2], true).unwrap();
    assert_eq!(prioridad(&a, ids[2]), 900, "llega a la cabeza");
    a.mover_en_cola(ids[2], true).unwrap();
    assert_eq!(prioridad(&a, ids[2]), 900, "en el extremo no hace nada");
}

#[test]
fn una_mejora_manual_paga_ocupa_cuadrilla_y_sube_el_nivel_al_terminar() {
    let r = Reglas::cargar();
    let (mut a, mut s, mut w) = (rico(&r), SitiosLibres, SinMundo);
    let granja = a.edificios.iter().find(|e| e.tipo == Tipo::Granja).unwrap().id;
    let x = exterior!(&r, s, w);
    a.mejorar_ahora(0, &r, &x, granja).unwrap();
    assert_eq!(a.edificios.cuadrillas_ocupadas(), 1);
    assert_eq!(a.mejorar_ahora(0, &r, &x, granja), Err(ErrorObra::NoEsMejorable), "ya se está mejorando");
    // La obra base de la Granja son 120 minutos; el nivel 2 tarda ×2 = 240.
    let m = a.edificio(granja).unwrap().mejora.unwrap();
    assert_eq!((m.nivel_objetivo, m.completa_en), (2, 240 * MS_POR_MINUTO));
    let mut ev = vec![];
    let mut s2 = SitiosLibres;
    let mut w2 = SinMundo;
    let mut x2 = exterior!(&r, s2, w2);
    for minuto in 1..=240 {
        ev.extend(a.avanzar_minuto(minuto * MS_POR_MINUTO, &mut x2));
    }
    assert!(ev.contains(&Evento::MejoraCompletada { id: granja, nivel: 2 }));
    assert_eq!(a.edificio(granja).unwrap().nivel_interno, 2);
}

#[test]
fn el_ascenso_informa_todos_los_bloqueos_a_la_vez() {
    let r = Reglas::cargar();
    let (mut a, mut s, mut w) = (Asentamiento::fundar(&r, 0, &[(Recurso::Madera, 50_000)], 3), SitiosLibres, SinMundo);
    let x = exterior!(&r, s, w);
    let b = a.bloqueos_de_ascenso(&r, &x, 0, &[]);
    assert!(b.contains(&Bloqueo::FaltaPoblacion) && b.contains(&Bloqueo::FaltanEdificios) && b.contains(&Bloqueo::RecursosInsuficientes), "{b:?}");
    assert!(b.iter().any(|x| matches!(x, Bloqueo::Insolvente(_))), "sin Cantera no hay ingreso de piedra para el nivel 2");
    // Sin cupo de Facción: con el cupo del nivel 2 ocupado.
    assert!(a.bloqueos_de_ascenso(&r, &x, 1, &[]).contains(&Bloqueo::SinCupoDeFaccion));
    // El techo provisional detiene en el nivel 4.
    a.nivel = 4;
    assert!(a.bloqueos_de_ascenso(&r, &x, 0, &[]).contains(&Bloqueo::NivelMaximo));
}

#[test]
fn pausar_la_auto_construccion_y_las_recetas_es_idempotente() {
    let r = Reglas::cargar();
    let mut a = Asentamiento::fundar(&r, 0, &[], 3);
    assert!(a.alternar_auto_construccion(true));
    assert!(!a.alternar_auto_construccion(true), "ya estaba pausada");
    assert!(a.alternar_receta(Recurso::LingoteCobre, true) && !a.alternar_receta(Recurso::LingoteCobre, true));
    assert!(a.alternar_receta(Recurso::LingoteCobre, false));
    a.calibrar_reserva(&r, Recurso::Madera, 5_000_000);
    assert_eq!(a.reserva_manual[Recurso::Madera.indice()], 999_000, "tope de la reserva manual");
}
