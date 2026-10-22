//! Un asentamiento fundado vive varios días de mundo, minuto a minuto, con un mundo de pruebas (un bosque, un nodo de piedra y
//! uno de ganado). Se vigilan los invariantes de Economia_Construccion §15 y que la economía arranque sola: Leñera, Cantera,
//! Viviendas, crecimiento y la subida al nivel 2.

use asentamiento::asentamiento::{Ascenso, Asentamiento, Evento, Exterior, Gratuito, Sitios, Urbano};
use asentamiento::catalogo::Tipo;
use asentamiento::datos::{Reglas, UNO};
use asentamiento::edificio::{Edificios, Fuente};
use asentamiento::produccion::Fuentes;
use asentamiento::recurso::{Recurso, TODOS};
use base::{Instante, Mil, MS_POR_MINUTO};
use std::collections::HashMap;

/// El mundo de pruebas: nodos con stock que se regeneran a los 10 minutos de agotarse y parcelas de bosque.
struct Mundo {
    stock: HashMap<u32, (Mil, Mil)>,
    vacio_desde: HashMap<u32, Instante>,
    ahora: Instante,
}

impl Fuentes for Mundo {
    fn densidad_parcela(&self, _: u32) -> i64 {
        800_000
    }
    fn extraer(&mut self, nodo: u32, pedido: Mil) -> Mil {
        let (stock, inicial) = self.stock[&nodo];
        if stock == 0 {
            match self.vacio_desde.get(&nodo) {
                Some(&desde) if self.ahora - desde >= 10 * MS_POR_MINUTO => {
                    self.vacio_desde.remove(&nodo);
                    self.stock.insert(nodo, (inicial, inicial));
                }
                None => {
                    self.vacio_desde.insert(nodo, self.ahora);
                    return 0;
                }
                _ => return 0,
            }
        }
        let (stock, inicial) = self.stock[&nodo];
        let sale = pedido.min(stock);
        self.stock.insert(nodo, (stock - sale, inicial));
        sale
    }
}

/// Un bosque con 6 parcelas, un nodo de piedra y uno de ganado; todo lo demás cabe siempre.
struct Terreno {
    parcelas_libres: Vec<u32>,
    cantera_libre: bool,
    corral_libre: bool,
}

impl Sitios for Terreno {
    fn proponer(&mut self, tipo: Tipo, _: &Urbano) -> Option<Option<Fuente>> {
        match tipo {
            Tipo::Lenera => self.parcelas_libres.first().map(|&p| Some(Fuente::Parcela(p))),
            Tipo::Cantera => self.cantera_libre.then_some(Some(Fuente::Nodo(1))),
            Tipo::Corral => self.corral_libre.then_some(Some(Fuente::Nodo(2))),
            Tipo::Mina | Tipo::MinaCobre | Tipo::MinaEstano | Tipo::MinaHierro => None,
            _ => Some(None),
        }
    }
    fn tomar(&mut self, _: u32, tipo: Tipo, fuente: Option<Fuente>, _: &Urbano, _: &mut u32) -> Option<Vec<Gratuito>> {
        match (tipo, fuente) {
            (Tipo::Lenera, Some(Fuente::Parcela(p))) => self.parcelas_libres.retain(|&x| x != p),
            (Tipo::Cantera, _) => self.cantera_libre = false,
            (Tipo::Corral, _) => self.corral_libre = false,
            _ => {}
        }
        Some(vec![])
    }
}

fn correr(dias: i64, semilla_hambre: bool) -> (Asentamiento, Vec<(i64, Evento)>) {
    let reglas = Reglas::cargar();
    let materiales: Vec<(Recurso, Mil)> = vec![(Recurso::Madera, 50_000), (Recurso::Piedra, 20_000), (Recurso::Trigo, 100_000), (Recurso::Oro, 100_000)];
    let mut a = Asentamiento::fundar(&reglas, 0, &materiales, 3);
    if semilla_hambre {
        a.almacen.poner(Recurso::Trigo, 0);
        a.edificios.retain(|e| e.tipo != Tipo::Granja);
        a.auto_pausada = true;
    }
    let mut mundo = Mundo { stock: HashMap::from([(1, (1_000_000, 1_000_000)), (2, (30_000, 30_000))]), vacio_desde: HashMap::new(), ahora: 0 };
    let mut terreno = Terreno { parcelas_libres: (0..6).collect(), cantera_libre: true, corral_libre: true };
    let sin_tecnologia = |_: &str| false;
    let sin_bonus = |_: Recurso| UNO;
    let mut eventos = vec![];
    let mut anterior_nivel = a.nivel;
    for minuto in 1..=dias * 1440 {
        let t = minuto * MS_POR_MINUTO;
        mundo.ahora = t;
        let mut x = Exterior {
            reglas: &reglas,
            fuentes: &mut mundo,
            sitios: &mut terreno,
            fertilidad: UNO,
            tiene_tecnologia: &sin_tecnologia,
            factor_tecnologia: &sin_bonus,
            nivel_faccion: 1,
            orden_era: 1,
            es_capital: true,
            faccion_tiene_gran_fundicion: false,
            ciudadanos: 1,
            distancia_capital: 0,
            recinto_nivel: 0,
            racion_guarnicion: 0,
            felicidad_extra: UNO,
            nobleza_extra: UNO,
            recaudacion_extra: UNO,
        };
        let mut ev = a.avanzar_hasta(t, &mut x);
        // Un bot pide la subida en cuanto puede.
        if a.ascenso.is_none() && minuto % 60 == 0 {
            let _ = a.solicitar_ascenso(t, &reglas, &x, 0, &mut ev);
        }
        for e in ev {
            eventos.push((minuto, e));
        }
        // Invariantes de §15.
        for &r in TODOS.iter() {
            assert!(a.almacen.cantidad(r) >= 0 && a.almacen.cantidad(r) <= a.almacen.capacidad(r), "almacén fuera de rango: {r:?} en el minuto {minuto}");
        }
        assert!(a.poblacion.pesants >= 0 && a.poblacion.artesanos >= 0 && a.poblacion.nobleza >= 0);
        assert!((0..=100_000).contains(&a.medidor.valor), "medidor {} en el minuto {minuto}", a.medidor.valor);
        assert!(a.nivel >= anterior_nivel && a.nivel_actual() <= a.nivel, "el nivel nunca baja y el operativo no pasa del alcanzado");
        anterior_nivel = a.nivel;
        if a.destruido {
            break;
        }
    }
    (a, eventos)
}

fn primer(ev: &[(i64, Evento)], f: impl Fn(&Evento) -> bool) -> Option<i64> {
    ev.iter().find(|(_, e)| f(e)).map(|(m, _)| *m)
}

#[test]
fn la_economia_arranca_sola_y_sube_al_nivel_2_sin_romper_ningun_invariante() {
    let (a, ev) = correr(10, false);
    assert!(!a.destruido, "no cae en ruinas");
    let leñera = primer(&ev, |e| matches!(e, Evento::EdificioCompletado { tipo: Tipo::Lenera, .. }));
    let cantera = primer(&ev, |e| matches!(e, Evento::EdificioCompletado { tipo: Tipo::Cantera, .. }));
    let ascenso = primer(&ev, |e| matches!(e, Evento::AscensoIniciado { objetivo: 2, .. }));
    let nivel2 = primer(&ev, |e| matches!(e, Evento::NivelSubio { nivel: 2 }));
    println!("Leñera {leñera:?} · Cantera {cantera:?} · ascenso pedido {ascenso:?} · nivel 2 {nivel2:?} (minutos)");
    println!("pesants {} · viviendas {} · edificios {}", a.poblacion.pesants, a.edificios.en_cualquier_estado(Tipo::Vivienda), a.edificios.len());
    assert!(leñera.is_some() && cantera.is_some(), "construye su Leñera y su Cantera solo");
    assert!(nivel2.is_some_and(|m| m < 10 * 1440), "sube al nivel 2 dentro de los 10 días");
    assert_eq!(a.nivel, 2);
    let _: Option<Ascenso> = a.ascenso;
}

#[test]
fn es_determinista() {
    let (a, ev) = correr(3, false);
    let (b, ev2) = correr(3, false);
    assert_eq!(ev, ev2);
    assert_eq!((a.poblacion, a.edificios.len()), (b.poblacion, b.edificios.len()));
}

#[test]
fn sin_trigo_ni_granja_la_gente_muere_y_la_nutricion_llega_a_cero() {
    let (a, ev) = correr(1, true);
    assert_eq!(a.medidor.nivel_actual, 1);
    assert!(a.poblacion.pesants < 20, "la población se ha reducido: {}", a.poblacion.pesants);
    assert!(primer(&ev, |e| matches!(e, Evento::Hambruna { .. })).is_some());
}

/// Un minuto de una plaza en un mundo sin yacimientos ni sitios (solo importa la comida y el hambre).
fn paso_de_hambre(a: &mut Asentamiento, reglas: &Reglas, minuto: i64) -> Vec<Evento> {
    let (mut mundo, mut terreno) = (Mundo { stock: HashMap::new(), vacio_desde: HashMap::new(), ahora: 0 }, Terreno { parcelas_libres: vec![], cantera_libre: false, corral_libre: false });
    let (sin_tecnologia, sin_bonus) = (|_: &str| false, |_: Recurso| UNO);
    let mut x = Exterior {
        reglas,
        fuentes: &mut mundo,
        sitios: &mut terreno,
        fertilidad: UNO,
        tiene_tecnologia: &sin_tecnologia,
        factor_tecnologia: &sin_bonus,
        nivel_faccion: 1,
        orden_era: 1,
        es_capital: true,
        faccion_tiene_gran_fundicion: false,
        ciudadanos: 1,
        distancia_capital: 0,
        recinto_nivel: 0,
        racion_guarnicion: 0,
        felicidad_extra: UNO,
        nobleza_extra: UNO,
        recaudacion_extra: UNO,
    };
    a.avanzar_hasta(minuto * MS_POR_MINUTO, &mut x)
}

#[test]
fn tras_tocar_cero_de_nutricion_la_plaza_no_crece_hasta_recuperar_cien() {
    let reglas = Reglas::cargar();
    let mut a = Asentamiento::fundar(&reglas, 0, &[(Recurso::Oro, 100_000)], 3);
    a.edificios.retain(|e| e.tipo != Tipo::Granja);
    a.auto_pausada = true;
    // Sin trigo la nutrición baja 20 por minuto (y mientras sea positiva la plaza todavía crece, frenada). En el quinto toca 0: entra en
    // recuperación, no crece y muere el 5 % de los pesants (entre 20 y 39: una persona).
    let mut ev = vec![];
    for m in 1..=4 {
        ev.extend(paso_de_hambre(&mut a, &reglas, m));
    }
    let antes = a.poblacion.pesants;
    assert!((20..40).contains(&antes) && !a.poblacion.recuperando && ev.is_empty(), "{antes} {ev:?}");
    ev = paso_de_hambre(&mut a, &reglas, 5);
    assert_eq!((a.poblacion.nutricion, a.poblacion.recuperando, a.poblacion.pesants), (0, true, antes - 1));
    assert!(ev.contains(&Evento::Hambruna { pesants: 1, artesanos: 0, nobleza: 0 }), "{ev:?}");

    // Llega la comida: la nutrición sube 5 por minuto (20 minutos hasta 100) y en todo ese tiempo la población queda quieta.
    a.almacen.poner(Recurso::Trigo, 1_000_000);
    let quietos = a.poblacion.pesants;
    for m in 6..=24 {
        let ev = paso_de_hambre(&mut a, &reglas, m);
        assert!(!ev.iter().any(|e| matches!(e, Evento::Hambruna { .. })), "con nutrición positiva no muere nadie");
        assert_eq!((a.poblacion.pesants, a.poblacion.recuperando), (quietos, true), "minuto {m}, nutrición {}", a.poblacion.nutricion);
    }
    // El minuto 25 completa los 100 y la plaza vuelve a crecer en ese mismo paso; unos minutos después hay más gente.
    paso_de_hambre(&mut a, &reglas, 25);
    assert_eq!((a.poblacion.nutricion, a.poblacion.recuperando), (100_000, false));
    for m in 26..=30 {
        paso_de_hambre(&mut a, &reglas, m);
    }
    assert!(a.poblacion.pesants > quietos, "crece de nuevo: {} frente a {quietos}", a.poblacion.pesants);
}
