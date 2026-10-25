//! Utilidades de las pruebas: una retícula sintética como mundo y un escenario aleatorio determinista.
#![allow(dead_code)]

use base::{Instante, Punto, Rng};
use contrato::v1::{AristaGrafo, GrafoNavegacion, NodoGrafo, Punto as PuntoPb};
use nucleo::{Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas};
use rutas::Rutas;
use std::sync::Arc;

pub mod real;
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

/// Andamiaje de los escenarios (5E): ¿mandaba la cuenta en la plaza de esta orden con la autorización de antes del sistema 5 (`manda_en`: un héroe suyo ciudadano de la Facción de la
/// plaza)? Entonces la orden de plaza —las que piden un cargo, residir y estar dentro o fijar la puerta— se da como administración: estas pruebas miden otra cosa y se escribieron con
/// aquella regla (decisión 9 del plan del sistema 5: el andamiaje de administración se queda para montar escenarios). Quien no mandaba sigue siendo un jugador y la matriz real le rechaza.
/// Quién puede darlas de verdad lo prueba `tests/cargos.rs`.
pub fn mandaba(n: &Nucleo, actor: &str, c: &Comando) -> bool {
    if actor == "admin" || !c.autoriza().es_de_plaza() {
        return false;
    }
    // Unas ruinas de ciudadela sin dueño las repara un ciudadano de la Facción que tiene ahora su zona (la orden lo mira).
    if let Comando::RepararAvanzada { avanzada, .. } = c {
        if n.avanzada(*avanzada).is_some_and(|(madre, _)| madre.is_none()) {
            return n.heroes().any(|h| h.cuenta == actor && h.faccion.is_some());
        }
    }
    let plaza = |id: u64| n.estado().asentamientos.get(&id).map(|a| a.faccion);
    let origen = |caravana: u64| n.caravana(caravana).and_then(|x| plaza(u64::from(x.origen)));
    let faccion = match c {
        Comando::AnadirEdificio { asentamiento, .. }
        | Comando::QuitarDeCola { asentamiento, .. }
        | Comando::MoverEnCola { asentamiento, .. }
        | Comando::MejorarEdificio { asentamiento, .. }
        | Comando::AlternarAutoConstruccion { asentamiento, .. }
        | Comando::AlternarReceta { asentamiento, .. }
        | Comando::CalibrarReserva { asentamiento, .. }
        | Comando::SolicitarAscenso { asentamiento }
        | Comando::ActivarPolitica { asentamiento, .. }
        | Comando::ComprometerRecinto { asentamiento, .. }
        | Comando::MejorarRecinto { asentamiento, .. }
        | Comando::AbandonarRecinto { asentamiento, .. }
        | Comando::ColocarOrdenMercado { asentamiento, .. }
        | Comando::CrearCaravana { asentamiento }
        | Comando::MarcarPueblo { asentamiento, .. }
        | Comando::FijarPuerta { asentamiento, .. }
        | Comando::VetarJugador { asentamiento, .. }
        | Comando::AsignarCargoLocal { asentamiento, .. }
        | Comando::AlternarReabastecerAliados { asentamiento, .. } => plaza(*asentamiento),
        Comando::ProponerTrueque { a, .. } => plaza(*a),
        Comando::AceptarTrueque { acuerdo } | Comando::RechazarTrueque { acuerdo } => n.estado().comercio.acuerdos.iter().find(|x| x.id == *acuerdo).and_then(|x| plaza(u64::from(x.b))),
        Comando::CancelarOrdenMercado { orden } => n.estado().comercio.libro.buscar(*orden).and_then(|o| plaza(u64::from(o.asentamiento))),
        Comando::AnadirCarro { caravana, .. }
        | Comando::AnadirAnimal { caravana, .. }
        | Comando::ReservarCaravana { caravana, .. }
        | Comando::PrepararCaravana { caravana, .. }
        | Comando::CancelarCaravana { caravana }
        | Comando::MoverCargaAparcada { caravana, .. }
        | Comando::EnviarCaravanaAlOrigen { caravana } => origen(*caravana),
        Comando::MoverCarro { de, .. } => origen(*de),
        Comando::DestruirPueblo { pueblo } | Comando::ReponerCarro { pueblo } => n.pueblo(*pueblo).and_then(|(m, _)| plaza(m)),
        Comando::MejorarAvanzada { avanzada, .. }
        | Comando::RepararAvanzada { avanzada, .. }
        | Comando::DestruirAvanzada { avanzada, .. }
        | Comando::CancelarObraDeAvanzada { avanzada, .. }
        | Comando::ElegirTropaDeAvanzada { avanzada, .. } => n.avanzada(*avanzada).and_then(|(m, _)| m).and_then(plaza),
        _ => None,
    };
    faccion.is_some_and(|f| n.heroes().any(|h| h.cuenta == actor && h.faccion == Some(f)))
}

/// Los cargos de la plaza con los que sus órdenes pasan la matriz real (5E): Gobernador, Tesorero y Maestro de Obras para `heroe`, que reside en ella (Q4). Los designa la administración (el
/// andamiaje); el héroe tiene que estar dentro para ejercerlos.
pub fn gobierna(n: &mut Nucleo, plaza: u64, heroe: u64) {
    use asentamiento::Cargo::*;
    for (i, cargo) in [Gobernador, Tesorero, MaestroObras].into_iter().enumerate() {
        let p = Peticion::nueva("admin", &format!("cargo-{plaza}-{heroe}-{i}"), Comando::AsignarCargoLocal { heroe, asentamiento: plaza, cargo, designado: heroe }).de_admin();
        let t = n.ahora();
        let r = n.aplicar_en(t, p);
        assert!(r.aceptada() || r.resultado == nucleo::Resultado::SinCambios, "{cargo:?} para {heroe} en as-{plaza}: {r:?}");
    }
}

/// La puerta por relación de antes de 5E (lo que daba el alias `FijarAcceso`, retirado en 5H, 5E-12): cerrada a los grupos que van a `false` (aliados, neutrales, enemigos) y abierta a los
/// demás, los Aedas incluidos. Es una `FijarPuerta` sin héroe: para darla como administración (o por una cuenta que mandaba, [`mandaba`]).
pub fn acceso(asentamiento: u64, aliados: bool, neutrales: bool, enemigos: bool) -> Comando {
    use politica::Grupo;
    let cerradas = [(Grupo::Aliados, aliados), (Grupo::Neutrales, neutrales), (Grupo::Enemigos, enemigos)].into_iter().filter(|(_, abierta)| !abierta).map(|(g, _)| g);
    Comando::FijarPuerta { heroe: 0, asentamiento, cerrada_a: politica::Grupos::de(cerradas) }
}

/// Andamiaje de 5E-1: `heroe` puede colocar avanzadas de la plaza (reside en ella y es su Maestro de Obras, lo nombra la administración); su columna tiene que estar en el sitio.
pub fn coloca_avanzadas(n: &mut Nucleo, plaza: u64, heroe: u64) {
    for (i, c) in [
        Comando::FijarResidencia { heroe, residencia: Some(militar::heroe::Residencia::Asentamiento(plaza as u32)) },
        Comando::AsignarCargoLocal { heroe, asentamiento: plaza, cargo: asentamiento::Cargo::MaestroObras, designado: heroe },
    ]
    .into_iter()
    .enumerate()
    {
        let t = n.ahora();
        let r = n.aplicar_en(t, Peticion::nueva("admin", &format!("maestro-{plaza}-{heroe}-{i}"), c).de_admin());
        assert!(r.aceptada() || r.resultado == nucleo::Resultado::SinCambios, "{r:?}");
    }
}

/// ¿Le falta a la plaza un Gobernador que la gobierne desde dentro? Sin titular, o con el de oficio del fundador (5E-3) fuera de ella (su columna sigue en el mapa): los escenarios de las
/// pruebas dan entonces el cargo al primer residente que está dentro.
pub fn sin_gobernador_dentro(n: &Nucleo, plaza: u64) -> bool {
    n.estado().asentamientos[&plaza].cargos.titular(asentamiento::Cargo::Gobernador).is_none_or(|g| n.heroe(g).is_none_or(|h| h.heroe.presencia.ubicacion != militar::presencia::Ubicacion::Asentamiento(plaza as u32)))
}

/// Un Gobernador de oficio de `cuenta` en la plaza (5E): un héroe de campo suyo nuevo que reside en ella, entra (su columna se deshace dentro) y recibe sus cargos ([`gobierna`]). Las órdenes de
/// plaza de la cuenta que no nombran héroe pasan entonces la matriz real con él; el héroe de la sesión de la cuenta sigue siendo el de antes (el de id menor). Devuelve su id.
pub fn gobernador_de_oficio(n: &mut Nucleo, cuenta: &str, plaza: u64) -> u64 {
    let (faccion, puerta) = { let a = &n.estado().asentamientos[&plaza]; (a.faccion, a.posicion) };
    let t = n.ahora();
    let col = n.crear_ejercito(t, cuenta, faccion, puerta, 500, 100_000).unwrap();
    let heroe = n.ejercito(col).unwrap().columna.lider;
    for (i, c) in [
        Comando::FijarResidencia { heroe, residencia: Some(militar::heroe::Residencia::Asentamiento(plaza as u32)) },
        Comando::Guarnecer { asentamiento: plaza, heroe },
    ]
    .into_iter()
    .enumerate()
    {
        let r = n.aplicar_en(t, Peticion::nueva("admin", &format!("oficio-{plaza}-{heroe}-{i}"), c).de_admin());
        assert!(r.aceptada(), "{r:?}");
    }
    gobierna(n, plaza, heroe);
    heroe
}

/// Andamiaje de las pruebas cuya cuenta es siempre la dueña de la plaza (5E): sus órdenes de plaza se dan como administración, como si mandara con la regla de antes del sistema 5 (ver
/// [`mandaba`]).
pub fn como_dueno(p: Peticion) -> Peticion {
    if p.comando.autoriza().es_de_plaza() {
        p.de_admin()
    } else {
        p
    }
}
