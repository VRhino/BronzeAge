//! Utilidades de las pruebas: una retícula sintética como mundo y un escenario aleatorio determinista.
#![allow(dead_code)]

use base::{Instante, Punto, Rng};
use contrato::v1::{AristaGrafo, GrafoNavegacion, NodoGrafo, Punto as PuntoPb};
use nucleo::{Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas};
use rutas::Rutas;
use std::sync::Arc;

pub mod sesion;

pub const ESP: i64 = 8_000;

/// Centro de la celda `(x, y)` de la retícula.
pub fn c(x: i64, y: i64) -> Punto {
    Punto::new(x * ESP + ESP / 2, y * ESP + ESP / 2)
}

/// Retícula `w × h` abierta con 8 vecinos (como la de las pruebas de `rutas`).
pub fn reticula(w: i64, h: i64) -> Arc<Rutas> {
    let mut g = GrafoNavegacion { espaciado_min: ESP as u32, ..Default::default() };
    for y in 0..h {
        for x in 0..w {
            let p = c(x, y);
            g.nodos.push(NodoGrafo { id: (y * w + x) as u32, posicion: Some(PuntoPb { x: p.x, y: p.y }) });
        }
    }
    for y in 0..h {
        for x in 0..w {
            for (dx, dy) in [(1, 0), (0, 1), (1, 1), (-1, 1)] {
                let (x2, y2) = (x + dx, y + dy);
                if !(0..w).contains(&x2) || !(0..h).contains(&y2) {
                    continue;
                }
                g.aristas.push(AristaGrafo {
                    a: (y * w + x) as u32,
                    b: (y2 * w + x2) as u32,
                    coste_milesimas: if dx != 0 && dy != 0 { 1414 } else { 1000 },
                    vado: false,
                });
            }
        }
    }
    Arc::new(Rutas::desde_grafo(&g))
}

pub fn arranque(rutas: Arc<Rutas>) -> Arranque {
    Arranque { reglas: reglas(), ajustes: Ajustes::default(), semilla: 42, inicio: 0, rutas, mundo_hash: "hash-de-prueba".into(), mundo: None }
}

pub fn nucleo(w: i64, h: i64) -> Nucleo {
    Nucleo::nuevo(arranque(reticula(w, h)))
}

pub fn marchar(actor: &str, id: &str, ejercito: u64, destino: Punto) -> Peticion {
    Peticion::nueva(actor, id, Comando::MarcharA { ejercito, destino })
}

/// Escenario aleatorio determinista sobre una retícula de `w × h`: `n` columnas y `pasos` órdenes (marchar o estacionar) en
/// instantes crecientes, con avances intercalados. Devuelve el instante final.
pub struct Escenario {
    pub w: i64,
    pub h: i64,
    pub n: u64,
    pub semilla: u64,
}

impl Escenario {
    pub fn crear(&self, nucleo: &mut Nucleo) -> Vec<u64> {
        let mut rng = Rng::derivado(self.semilla, "crear");
        (0..self.n)
            .map(|i| {
                let p = c(rng.por_debajo(self.w as u64) as i64, rng.por_debajo(self.h as u64) as i64);
                // Una columna come 125 por minuto en marcha y el aviso salta a 3.500: unas se quedan sin víveres y otras cruzan el aviso.
                let viveres = if i % 2 == 0 { 20 + rng.por_debajo(300) as i64 } else { 3_550 + rng.por_debajo(700) as i64 };
                nucleo
                    .crear_ejercito(0, &format!("c{}", i % 7), (i % 4) as u32, p, 2_000 + rng.por_debajo(4_000) as i64, viveres)
                    .expect("alta")
            })
            .collect()
    }

    /// Órdenes `desde..hasta` (índices de paso); cada paso a `t = paso × 3.000 ms`. `avanzar` decide cómo se mueve el reloj
    /// entre órdenes (para probar que no importa cómo se trocee el avance).
    pub fn ordenes(&self, nucleo: &mut Nucleo, ids: &[u64], desde: u64, hasta: u64, avanzar: &dyn Fn(&mut Nucleo, Instante)) {
        let mut rng = Rng::derivado(self.semilla, "ordenes");
        for _ in 0..desde * 4 {
            rng.siguiente(); // mismo flujo que si se hubieran ejecutado los pasos anteriores
        }
        for paso in desde..hasta {
            let t: Instante = paso as i64 * 3_000;
            avanzar(nucleo, t);
            let id = ids[rng.por_debajo(ids.len() as u64) as usize];
            let accion = rng.por_debajo(8);
            let destino = c(rng.por_debajo(self.w as u64) as i64, rng.por_debajo(self.h as u64) as i64);
            let lider = nucleo.estado().ejercitos[&id].columna.lider;
            let actor = nucleo.estado().militar.heroes[&lider].cuenta.clone();
            let comando = if accion == 0 { Comando::Estacionar { ejercito: id } } else { Comando::MarcharA { ejercito: id, destino } };
            nucleo.aplicar_en(t, Peticion::nueva(&actor, &format!("p{paso}"), comando));
        }
    }
}

/// Las reglas de `arranque`.
pub fn reglas() -> Reglas {
    Reglas { viveres_bajos_pct: 1, ..Reglas::default() }
}
