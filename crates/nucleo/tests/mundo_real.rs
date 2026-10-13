//! Con el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); se omite si no está.

use base::Punto;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{Arranque, Comando, Nucleo, Peticion, Reglas, Ajustes};
use rutas::Rutas;
use std::sync::Arc;

fn artefacto() -> Option<Vec<u8>> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generador/salida/mundo");
    let f = std::fs::read_dir(dir).ok()?.flatten().find(|e| e.file_name().to_string_lossy().starts_with("mundo-"))?;
    std::fs::read(f.path()).ok()
}

fn correr(m: &Mundo, rutas: &Arc<Rutas>) -> ([u8; 32], Vec<(Punto, Option<i64>)>) {
    let mut n = Nucleo::nuevo(Arranque {
        reglas: Reglas::default(),
        ajustes: Ajustes::default(),
        semilla: 1,
        inicio: 0,
        rutas: rutas.clone(),
        mundo_hash: m.hash().into(),
    });
    let camps: Vec<Punto> = m.campamentos().iter().map(|c| c.posicion.map(|p| Punto::new(p.x, p.y)).unwrap()).collect();
    let ids: Vec<u64> = (0..camps.len())
        .map(|i| n.crear_ejercito(0, &format!("h{i}"), "ana", 1, camps[i], 500, 350_000, 350_000).unwrap())
        .collect();
    let peticiones = ids.iter().enumerate().map(|(i, &id)| {
        Peticion::nueva("ana", &format!("m{i}"), Comando::MarcharA { ejercito: id, destino: camps[(i + 3) % camps.len()] })
    });
    let respuestas = n.marchas_en_lote(0, peticiones.collect());
    assert!(respuestas.iter().all(|r| r.aceptada()), "{respuestas:?}");
    let fines: Vec<(Punto, Option<i64>)> =
        ids.iter().map(|id| (n.estado().ejercitos[id].plan.vertices.last().unwrap().p, n.estado().ejercitos[id].plan.fin())).collect();
    n.avanzar_hasta(12 * 3_600_000);
    for (i, id) in ids.iter().enumerate() {
        let e = &n.estado().ejercitos[id];
        assert!(!e.marchando(), "todas llegan antes de 12 h");
        assert_eq!(e.posicion, fines[i].0);
    }
    (n.hash_estado(), fines)
}

#[test]
fn campamentos_a_campamentos_en_el_mundo_real() {
    let Some(bytes) = artefacto() else {
        eprintln!("sin artefacto de mundo real: se omite");
        return;
    };
    let m = Mundo::cargar(&bytes, &OpcionesValidacion::default()).expect("artefacto válido");
    let rutas = Arc::new(Rutas::desde_mundo(&m));
    let (hash, fines) = correr(&m, &rutas);
    assert!(fines.iter().all(|(_, t)| t.is_some_and(|t| t > 60_000)), "son marchas largas");
    assert_eq!((hash, fines), correr(&m, &rutas), "repetible bit a bit");
}
