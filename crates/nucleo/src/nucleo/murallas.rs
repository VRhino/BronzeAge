//! Los recintos de muralla dentro del núcleo (`Docs/Mechanics/Murallas.md`, fase de Murallas, tandas segunda y tercera): las órdenes `ComprometerRecinto`,
//! `MejorarRecinto` y `AbandonarRecinto` (123-125), la consulta de presupuesto, la sincronía con la ciudad de la plaza y lo que los recintos valen fuera de la economía: el
//! multiplicador del asedio con números y la geometría que viaja a Unity. Las reglas de la economía —coste, obra, mejora, defensa, mantenimiento, daño de la
//! conquista— son `asentamiento::recintos`; el trazo, `trazado::muralla`; aquí está lo que las une.
//!
//! # El trazo lo calcula el núcleo
//! La economía no conoce la ciudad. Al comprometer, el núcleo traza el anillo con la ciudad de la plaza (`trazar_recinto`), comprueba el arrabal y que envuelva
//! al anterior (`edificios_extramuros`, `contiene_a`) y le pasa a la economía el anillo congelado. Todas las comprobaciones van **antes** de tocar nada: un
//! rechazo no deja rastro. Las órdenes de plaza usan la autorización provisional de las demás (`manda_en`); el cargo que se invoca se da por ocupado (misma
//! `// DUDA:` que `ActivarPolitica`) y solo valen el Gobernador y el Maestro de Obras. Abandonar es del Gobernador (Murallas §7), pero no lleva cargo: hasta el
//! sistema 5 lo autoriza lo mismo que las demás.
//!
//! # La obra y el reloj
//! La primera celda se levanta **en el instante de la orden** (si hay con qué pagarla); cada celda deja una cita `Carga::FinDeObras` en el instante de la siguiente
//! (prioridad de fin de obras, antes del paso de minuto de la plaza: `anotar_sucesos` la programa a partir de `Evento::RecintoCelda`). Si al llegar la cita faltan
//! materiales, o se bajaría de la reserva, no se programa otra y el paso de minuto de la plaza reintenta la celda. Ninguna cita es recurrente ni hay una por plaza
//! ociosa: sin recinto en obra, la cola no guarda nada.
//!
//! # La ciudad es un derivado
//! `Urbe::set_recintos` recibe una **proyección** de los recintos de la economía (`recintos::proyeccion`: muros —muro y torre—, puertas y si el anillo está completo) y
//! recalcula la red entera, así que solo se llama cuando la proyección cambia: al comprometer, al cerrarse un anillo (o repararse), al abandonar y al sufrir daño.
//! No hace falta por cada celda (los muros ocupan suelo desde el compromiso) ni al mejorar (muro y torre ocupan igual). La foto no guarda recintos en la ciudad.

use super::*;
use crate::comando::rechazos::*;
use asentamiento::asentamiento::Evento as EventoEco;
use asentamiento::recintos::{self, ErrorRecinto, Recinto};
use asentamiento::{Cargo, Costo};
use combate::asedio;
use trazado::{contiene_a, edificios_extramuros, interior_de, trazar_recinto, CeldaMuro, Ciudad, ErrorMuralla, Trazo};

/// Lo que costaría y abarcaría un recinto de un nivel si se comprometiera ahora (Murallas §4: el número de puertas es irreversible y se enseña antes de pagar).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Presupuesto {
    pub nivel: u8,
    /// El anillo que se levantaría, en orden de recorrido, con sus puertas y torres, el área que encierra y los edificios que quedarían dentro y fuera.
    pub trazo: Trazo,
    /// Lo que costaría levantar el anillo entero (la tarifa acumulada del nivel por clase de celda).
    pub coste: Costo,
    /// Edificios urbanos fuera del recinto exterior actual (el arrabal que exige ampliar); 0 si aún no hay recintos.
    pub extramuros: usize,
    /// Si ya hay recintos, si el anillo nuevo envuelve estrictamente al exterior (sin eso no se puede ampliar); `true` si no hay ninguno.
    pub envuelve: bool,
}

/// Un recinto como lo ve una batalla de Unity: la geometría que está **en pie** (Batalla_Unity_Ciclo §3.3). Lo que la muralla vale dentro de la partida lo decide Unity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecintoEnPie {
    /// `n` del id de texto `recinto-<plaza>-<n>` del contrato.
    pub id: u32,
    /// El nivel físico en pie (una mejora en curso no lo cambia, Q1).
    pub nivel: u8,
    /// Las celdas levantadas, en su orden de recorrido, con la clase que tienen en el anillo en pie (muro, puerta o torre).
    pub celdas: Vec<CeldaMuro>,
}

/// Por qué no hay presupuesto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorPresupuesto {
    SinPlaza,
    NivelInexistente,
    Trazo(ErrorMuralla),
}

fn extramuros_de(ciudad: &Ciudad, recinto: &Recinto) -> usize {
    interior_de(ciudad, &recinto.celdas).map_or(0, |i| edificios_extramuros(ciudad, &i))
}

fn rechazo_de_recinto(e: ErrorRecinto) -> (&'static str, String) {
    match e {
        ErrorRecinto::NivelInexistente => (MURALLAS_NIVEL_INEXISTENTE, "no existe ese nivel de recinto (1 empalizada, 2 muro de piedra, 3 muralla con adarve)".into()),
        ErrorRecinto::NivelDePlaza { necesario, actual } => (MURALLAS_NIVEL_DE_PLAZA, format!("ese recinto pide una plaza de nivel {necesario} y esta es de nivel {actual}")),
        ErrorRecinto::EnObra => (MURALLAS_EN_OBRA, "hay un recinto sin cerrar (en obra o dañado por una conquista): hay que terminarlo antes".into()),
        ErrorRecinto::Arrabal { extramuros, minimo } => (MURALLAS_ARRABAL, format!("ampliar exige {minimo} edificios fuera del recinto y solo hay {extramuros}")),
        ErrorRecinto::NoExiste => (MURALLAS_RECINTO_DESCONOCIDO, "la plaza no tiene ese recinto".into()),
        ErrorRecinto::Incompleto => (MURALLAS_INCOMPLETO, "el recinto no está completo: primero se cierra el anillo".into()),
        ErrorRecinto::YaMejorando => (MURALLAS_YA_MEJORANDO, "el recinto ya se está mejorando".into()),
        ErrorRecinto::EnElMaximo => (MURALLAS_EN_EL_MAXIMO, "el recinto ya está en su nivel máximo".into()),
        ErrorRecinto::YaCerrado => (MURALLAS_YA_CERRADO, "un recinto que alguna vez estuvo completo no se abandona".into()),
    }
}

fn rechazo_de_trazo(e: ErrorMuralla) -> (&'static str, String) {
    match e {
        ErrorMuralla::SinCentroUrbano => (MURALLAS_SIN_CENTRO_URBANO, "la plaza no tiene Centro Urbano que amurallar".into()),
        ErrorMuralla::NoSePuedeCerrar => (MURALLAS_NO_SE_PUEDE_CERRAR, "no hay trazo válido: el anillo no se puede cerrar en esta ciudad".into()),
        ErrorMuralla::FueraDeVentana => (MURALLAS_FUERA_DE_LA_VENTANA, "el anillo no cabe en la ventana de la ciudad".into()),
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// El presupuesto de un recinto de `nivel` en la plaza: el trazo que saldría ahora, lo que costaría y, si ya hay recintos, el arrabal y si envuelve al exterior.
    /// No toca nada. No mira el nivel de la plaza ni los demás requisitos de comprometer (eso lo dice la orden).
    pub fn presupuesto_de_recinto(&self, plaza: u64, nivel: u8) -> Result<Presupuesto, ErrorPresupuesto> {
        let a = self.estado.asentamientos.get(&plaza).ok_or(ErrorPresupuesto::SinPlaza)?;
        let m = &self.estado.reglas.economia.datos.muralla;
        if !m.existe(nivel) {
            return Err(ErrorPresupuesto::NivelInexistente);
        }
        let ciudad = a.urbe.ciudad();
        let trazo = trazar_recinto(ciudad, &m.parametros_de_trazo(), nivel).map_err(ErrorPresupuesto::Trazo)?;
        let anterior = a.economia.recintos.last();
        Ok(Presupuesto {
            nivel,
            coste: m.coste_de_trazo(&trazo.celdas, nivel),
            extramuros: anterior.map_or(0, |r| extramuros_de(ciudad, r)),
            envuelve: anterior.map_or(true, |r| contiene_a(&trazo.celdas, &r.celdas)),
            trazo,
        })
    }

    /// El multiplicador del defensor (ppm) cuando la plaza se asedia con números: el del recinto de referencia (Murallas §9.1, `1 + (bono − 1) × integridad / puertas`) o 1,0 sin
    /// recinto. Defiende lo que está en pie (Q1): una mejora o una ampliación en curso no lo quita. Una plaza que no existe no multiplica nada.
    pub fn multiplicador_de_plaza(&self, plaza: u64) -> i64 {
        self.estado.asentamientos.get(&plaza).map_or(asedio::SIN_RECINTO, |a| recintos::multiplicador_defensivo(&self.estado.reglas.economia.datos.muralla, &a.economia.recintos))
    }

    /// Los recintos que viajan a una batalla de Unity (Murallas §9.1): por cada uno con alguna celda en pie, su id, su nivel físico y **solo** las celdas levantadas
    /// (`celdas[0..=avance]`). Una mejora en curso no cambia nada y uno sin celdas en pie no viaja. Del más interior al más exterior. El `layoutVersion` es
    /// [`trazado::LAYOUT_VERSION`] y las `unidadesPorCelda`, [`trazado::CELDA_UL`]. El ticket del asedio los congela en el contrato (4H).
    pub fn recintos_en_pie(&self, plaza: u64) -> Vec<RecintoEnPie> {
        let Some(a) = self.estado.asentamientos.get(&plaza) else { return vec![] };
        a.economia.recintos.iter().filter(|r| r.levantadas() > 0).map(|r| RecintoEnPie { id: r.id, nivel: r.nivel, celdas: r.celdas[..r.levantadas()].to_vec() }).collect()
    }

    // ───────────────────────────── La ciudad es un derivado ─────────────────────────────

    /// Deja la ciudad de la plaza con la proyección de sus recintos. Solo la cambia (y recalcula la red) si no coincide ya.
    pub(super) fn sincronizar_recintos(&mut self, id: u64) {
        let Some(a) = self.estado.asentamientos.get_mut(&id) else { return };
        let proyeccion = recintos::proyeccion(&a.economia.recintos);
        if a.urbe.ciudad().recintos() != proyeccion {
            a.urbe.set_recintos(proyeccion);
        }
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// `ComprometerRecinto`, `MejorarRecinto` y `AbandonarRecinto`. `Ok(Some(n))` = comprometió el recinto `n`; `Ok(None)` = cambió algo; los rechazos salen como `Err`
    /// sin haber tocado el estado.
    pub(super) fn ejecutar_murallas(&mut self, t: Instante, p: &Peticion) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let (Comando::ComprometerRecinto { asentamiento, .. } | Comando::MejorarRecinto { asentamiento, .. } | Comando::AbandonarRecinto { asentamiento, .. }) = &p.comando else {
            unreachable!("solo se llama con órdenes de murallas")
        };
        let id = *asentamiento;
        let Some(plaza) = self.estado.asentamientos.get(&id) else { return Err(rechazo(ASENTAMIENTO_DESCONOCIDO, format!("as-{id}"))) };
        if !self.manda_en(p, plaza.faccion) {
            return Err(rechazo(NO_AUTORIZADO, format!("no mandas en as-{id}")));
        }
        let datos = self.estado.reglas.economia.datos.clone();
        let de_recinto = |e: ErrorRecinto| {
            let (c, m) = rechazo_de_recinto(e);
            rechazo(c, m)
        };
        let con_cargo = |cargo: Cargo| match cargo {
            // DUDA: sin sistema de cargos, se da por ocupado el cargo que invoca la orden (ya se comprobó que el actor manda en la plaza).
            Cargo::Gobernador | Cargo::MaestroObras => Ok(()),
            _ => Err(rechazo(MURALLAS_SIN_CARGO, "solo el Gobernador o el Maestro de Obras encargan o mejoran una muralla".into())),
        };
        let mut sucesos: Vec<EventoEco> = vec![];
        let mut creada = None;
        let con_obra = match &p.comando {
            Comando::ComprometerRecinto { cargo, nivel, .. } => {
                con_cargo(*cargo)?;
                let eco = &plaza.economia;
                let anterior = eco.comprobar_comprometer(&datos, *nivel).map_err(de_recinto)?;
                let ciudad = plaza.urbe.ciudad();
                if let Some(ant) = anterior {
                    datos.muralla.arrabal_suficiente(extramuros_de(ciudad, ant)).map_err(de_recinto)?;
                }
                let trazo = trazar_recinto(ciudad, &datos.muralla.parametros_de_trazo(), *nivel).map_err(|e| {
                    let (c, m) = rechazo_de_trazo(e);
                    rechazo(c, m)
                })?;
                if anterior.is_some_and(|ant| !contiene_a(&trazo.celdas, &ant.celdas)) {
                    return Err(rechazo(MURALLAS_NO_ENVUELVE, "el recinto nuevo tiene que envolver por completo al anterior".into()));
                }
                let eco = &mut self.estado.asentamientos.get_mut(&id).expect("existe").economia;
                creada = Some(eco.comprometer_recinto(t, *nivel, trazo.celdas, &mut sucesos) as u64);
                true
            }
            Comando::MejorarRecinto { cargo, recinto, .. } => {
                con_cargo(*cargo)?;
                plaza.economia.comprobar_mejora(&datos, *recinto).map_err(de_recinto)?;
                self.estado.asentamientos.get_mut(&id).expect("existe").economia.iniciar_mejora(*recinto, &mut sucesos);
                true
            }
            Comando::AbandonarRecinto { recinto, .. } => {
                let eco = &mut self.estado.asentamientos.get_mut(&id).expect("existe").economia;
                eco.abandonar_recinto(*recinto, &mut sucesos).map_err(de_recinto)?;
                false
            }
            _ => unreachable!("solo se llama con órdenes de murallas"),
        };
        // La primera celda se levanta en el instante de la orden, si hay con qué.
        if con_obra {
            sucesos.extend(self.con_exterior(id, t, |eco, x| {
                let mut ev = vec![];
                eco.levantar_recintos(t, x, &mut ev);
                ev
            })
            .expect("la plaza existe y hay mundo"));
        }
        self.anotar_sucesos(id, t, &sucesos);
        self.tr.asentamientos.push(id);
        Ok(creada)
    }
}
