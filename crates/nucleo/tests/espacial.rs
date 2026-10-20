//! Encuentros exactos y visión conectados al núcleo (generadores `GeneradorEncuentros` y `GeneradorVision`).

mod comun;

use comun::{c, marchar, nucleo};
use nucleo::espacial::{GeneradorEncuentros, GeneradorVision};
use nucleo::{Ajustes, Arranque, Nucleo, Reglas};

fn escena() -> (Nucleo, u64, u64) {
    let mut n = nucleo(40, 5);
    n.agregar_generador(Box::new(GeneradorEncuentros::nuevo()));
    // Dos columnas en la misma fila, una hacia la otra, a 500 milésimas/s (30 unidades/min): se cruzan hacia la mitad.
    let a = n.crear_ejercito(0, "ana", 1, c(0, 2), 500, 350_000).unwrap();
    let b = n.crear_ejercito(0, "beto", 2, c(39, 2), 500, 350_000).unwrap();
    (n, a, b)
}

fn encuentros(n: &Nucleo, desde: u64) -> Vec<(i64, Vec<u64>)> {
    n.cambios_desde(desde)
        .unwrap()
        .flat_map(|cv| cv.eventos.iter().filter(|e| e.codigo == "ejercito.encuentro").map(|e| (e.en, e.ejercitos.clone())))
        .collect()
}

#[test]
fn dos_columnas_que_se_cruzan_producen_un_solo_encuentro_exacto() {
    let (mut n, a, b) = escena();
    assert!(n.aplicar_en(0, marchar("ana", "m1", a, c(39, 2))).aceptada());
    assert!(n.aplicar_en(0, marchar("beto", "m2", b, c(0, 2))).aceptada());
    n.avanzar_hasta(3_600_000);
    let e = encuentros(&n, 0);
    assert_eq!(e.len(), 1, "un único encuentro por par: {e:?}");
    assert_eq!(e[0].1, vec![a.min(b), a.max(b)]);
    // Separadas 39·8000 = 312.000; se acercan a 1.000 milésimas/s entre las dos: llegan a 3.000 de distancia a los 309 s.
    assert_eq!(e[0].0, 309_000, "instante exacto, sin sondeo");
}

#[test]
fn cambiar_de_rumbo_antes_del_cruce_anula_el_encuentro() {
    let (mut n, a, b) = escena();
    n.aplicar_en(0, marchar("ana", "m1", a, c(39, 2)));
    n.aplicar_en(0, marchar("beto", "m2", b, c(0, 2)));
    // A los 100 s, Ana se desvía a otra fila y nunca se cruzan a 3 unidades.
    n.avanzar_hasta(100_000);
    n.aplicar_en(100_000, marchar("ana", "m3", a, c(20, 0)));
    n.aplicar_en(100_000, marchar("beto", "m4", b, c(39, 4)));
    n.avanzar_hasta(3_600_000);
    assert!(encuentros(&n, 0).iter().all(|(t, _)| *t != 309_000), "el cruce original ya no existe");
}

#[test]
fn mismo_resultado_al_repetir_y_la_visión_registra_el_primer_contacto() {
    let correr = || {
        let mut n = Nucleo::nuevo(Arranque { ajustes: Ajustes { muestreo_ms: 250, ..Ajustes::default() }, reglas: Reglas::default(), ..comun::arranque(comun::reticula(40, 5)) });
        n.agregar_generador(Box::new(GeneradorEncuentros::nuevo()));
        n.agregar_generador(Box::new(GeneradorVision::nuevo()));
        let a = n.crear_ejercito(0, "ana", 1, c(0, 2), 500, 350_000).unwrap();
        let b = n.crear_ejercito(0, "beto", 2, c(39, 2), 500, 350_000).unwrap();
        n.aplicar_en(0, marchar("ana", "m1", a, c(39, 2)));
        n.aplicar_en(0, marchar("beto", "m2", b, c(0, 2)));
        n.avanzar_hasta(1_200_000);
        let contactos: Vec<(i64, Vec<u64>)> = n
            .cambios_desde(0)
            .unwrap()
            .flat_map(|cv| cv.eventos.iter().filter(|e| e.codigo == "vision.contacto").map(|e| (e.en, e.ejercitos.clone())))
            .collect();
        (n.hash_estado(), contactos)
    };
    let (h1, c1) = correr();
    let (h2, c2) = correr();
    assert_eq!(h1, h2, "determinista");
    assert_eq!(c1, c2);
    assert_eq!(c1.len(), 2, "cada facción ve a la otra por primera vez: {c1:?}");
    // Radio de visión de 30 unidades: se ven a ≤ 30.000 de distancia, bastante antes del encuentro (309 s).
    assert!(c1.iter().all(|(t, _)| *t < 309_000 && *t > 0), "{c1:?}");
}
