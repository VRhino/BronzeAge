//! Perseguir (Movimiento_Ejercitos §7, Doc 5.12.3, fase 4C): `perseguir`, `dejarDePerseguir` y todo lo que ocurre mientras dura una persecución.
//! Las condiciones son de `militar::persecucion`; la geometría, de `encuentros`.
//!
//! # Una persecución es un plan que se rehace
//! Quien persigue lleva un plan como cualquier columna: la ruta (el A\* de siempre) hasta el **punto de intercepción**, el primer sitio donde, yendo
//! a su velocidad, queda a ≤ `radioEncuentro` de la presa según el plan de la presa (`encuentros::interceptar`). Ese plan se rehace **cada vez que la
//! presa cambia el suyo** (un evento, no un sondeo: no hay umbral de recálculo) y cuando quien persigue llega al punto sin haberla alcanzado (la
//! intercepción supone la velocidad nominal; si el terreno lo retrasó, la presa ya no está ahí). Todo cuelga del índice `perseguidores` de la
//! presa, que se consulta en el momento en que ella se anuncia, nunca por barrido.
//!
//! Dos eventos exactos, plan contra plan, terminan una persecución:
//! - **Alcance**: el encuentro de la rejilla entre el perseguidor y su presa (el primer instante a ≤ `radioEncuentro`). Borra la persecución y deja
//!   `columna.presa_alcanzada`; no hay combate, atacar es una orden aparte. Quien persigue sigue su plan y acampa al final de él.
//! - **Presa fuera de la vista** (`SucesoMilitar::PresaFueraDeVista`): el primer instante en que la presa queda a más de `alcanceDeVista` de quien la
//!   persigue. Cancela la persecución (`columna.presa_perdida`): si no, la trayectoria del perseguidor delataría dónde está bajo la niebla. Se
//!   reprograma con cada plan nuevo de cualquiera de los dos y con cada cambio del alcance (la tropa en pie).
//!
//! La presa también se pierde (`columna.presa_a_cubierto`) cuando sale del mapa —entra en una plaza o un campamento, se disuelve, va adjunta o la
//! aparta una batalla— y cuando no hay camino por tierra hasta ella (`columna.presa_perdida`).

use super::*;
use crate::estado_comercio::{entidad_caravana, es_entidad_caravana, id_de_entidad};
use crate::estado::SucesoMilitar;
use ::encuentros::{interceptar, PRIORIDAD_ENCUENTROS};
use militar::inspeccion::Objetivo;
use militar::persecucion::{self, ErrorPersecucion, Perseguidor, Presa};

/// Cómo termina una persecución sin que su Líder la suelte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Fin {
    Alcanzada,
    Perdida,
    ACubierto,
}

impl Fin {
    fn codigo(self) -> &'static str {
        match self {
            Fin::Alcanzada => "columna.presa_alcanzada",
            Fin::Perdida => "columna.presa_perdida",
            Fin::ACubierto => "columna.presa_a_cubierto",
        }
    }
}

/// Lo que hace falta para seguir a la presa desde donde está quien persigue.
enum Seguimiento {
    /// Ir por esta ruta hasta el punto de intercepción.
    Ruta(Camino),
    /// Ya está en el punto de intercepción: espera ahí a que la presa llegue.
    Quieto,
    /// No se puede seguir.
    Fin(Fin),
}

fn rechazo_de_persecucion(e: ErrorPersecucion) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorPersecucion::SinHeroeSano => (PERSECUCION_SIN_HEROE_SANO, "todos los héroes de tu columna están heridos: no pueden perseguir"),
        ErrorPersecucion::PresaSinHeroeSano => (PERSECUCION_PRESA_SIN_HEROE_SANO, "esa columna solo lleva héroes heridos: no se la puede perseguir"),
        ErrorPersecucion::ClaseDistinta => (PERSECUCION_CLASE_DISTINTA, "un ejército solo persigue a otro ejército, y una columna personal a otra o a una caravana"),
        ErrorPersecucion::PropiaOAliada => (PERSECUCION_PROPIA_O_ALIADA, "no se persigue a los tuyos ni a un aliado"),
        ErrorPersecucion::CaravanaAdjunta => (PERSECUCION_CARAVANA_ADJUNTA, "una caravana adjunta es parte de un ejército: se persigue al ejército"),
        ErrorPersecucion::UnEjercitoNoPersigueCaravanas => (PERSECUCION_EJERCITO_SIN_CARAVANAS, "un ejército no persigue caravanas: las persigue una columna personal"),
        ErrorPersecucion::ACubierto => (PERSECUCION_A_CUBIERTO, "la presa no está en el mapa"),
        ErrorPersecucion::ASiMisma => (PERSECUCION_A_SI_MISMA, "una columna no se persigue a sí misma"),
        ErrorPersecucion::FueraDeVista => (PERSECUCION_FUERA_DE_VISTA, "no la ves desde tu columna: no se persigue lo que está fuera de tu vista"),
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// A quién persigue la columna, si persigue a alguien.
    pub fn persiguiendo(&self, columna: u64) -> Option<Objetivo> {
        let presa = self.estado.ejercitos.get(&columna)?.persiguiendo?;
        Some(if es_entidad_caravana(presa) { Objetivo::Caravana(id_de_entidad(presa)) } else { Objetivo::Ejercito(presa) })
    }

    /// Las columnas que persiguen a este objetivo (una columna o una caravana), por id.
    pub fn perseguidores_de(&self, objetivo: Objetivo) -> Vec<u64> {
        let entidad = match objetivo {
            Objetivo::Ejercito(id) => id,
            Objetivo::Caravana(id) => entidad_caravana(id),
            Objetivo::Asentamiento(_) => return vec![],
        };
        self.espacio.perseguidores.get(&entidad).into_iter().flatten().copied().collect()
    }

    // ───────────────────────────── Fijar y terminar ─────────────────────────────

    /// Fija (o borra) a quién persigue la columna y mantiene el índice de perseguidores. Es el único sitio que toca `Ejercito::persiguiendo`.
    pub(super) fn fijar_persecucion(&mut self, id: u64, presa: Option<u64>) {
        let Some(e) = self.estado.ejercitos.get_mut(&id) else { return };
        if e.persiguiendo == presa {
            return;
        }
        if let Some(vieja) = std::mem::replace(&mut e.persiguiendo, presa) {
            if let Some(set) = self.espacio.perseguidores.get_mut(&vieja) {
                set.remove(&id);
                if set.is_empty() {
                    self.espacio.perseguidores.remove(&vieja);
                }
            }
        }
        if let Some(nueva) = presa {
            self.espacio.perseguidores.entry(nueva).or_default().insert(id);
        }
        self.tr.tocar(Clase::Ejercito, id);
    }

    /// La persecución de `perseguidor` termina y se cuenta.
    pub(super) fn terminar_persecucion(&mut self, perseguidor: u64, fin: Fin, t: Instante) {
        let Some(presa) = self.estado.ejercitos.get(&perseguidor).and_then(|e| e.persiguiendo) else { return };
        self.fijar_persecucion(perseguidor, None);
        let caravana = es_entidad_caravana(presa);
        let mut columnas = vec![perseguidor];
        columnas.extend(Some(presa).filter(|p| !caravana && self.estado.ejercitos.contains_key(p)));
        let punto = self.estado.ejercitos[&perseguidor].posicion_en(t);
        let id = if caravana { id_de_entidad(presa) } else { presa };
        self.tr.emitir(&self.estado, EventoNucleo { codigo: fin.codigo().into(), en: t, ejercitos: columnas, asentamientos: vec![], punto: Some(punto), datos: vec![("presa", id as i64), ("caravana", caravana as i64)], ..Default::default() });
    }

    /// La presa sale del mapa: quienes la persiguen la dan por puesta a cubierto.
    pub(super) fn soltar_perseguidores(&mut self, presa: u64, fin: Fin) {
        let t = self.estado.ahora;
        let Some(perseguidores) = self.espacio.perseguidores.get(&presa).cloned() else { return };
        for c in perseguidores {
            self.terminar_persecucion(c, fin, t);
        }
    }

    /// El encuentro exacto entre `perseguidor` y su `presa`: si se la puede alcanzar (enemiga, con héroe sano o no adjunta), la persecución termina.
    pub(super) fn presa_alcanzada(&mut self, perseguidor: u64, presa: u64, t: Instante) {
        let Some(faccion) = self.estado.ejercitos.get(&perseguidor).map(|e| e.columna.faccion) else { return };
        let Some(info) = self.presa_de(faccion, presa, t) else { return };
        // La distancia ya la garantiza el evento (exacto); la redondeada de las posiciones podría pasarse de una milésima.
        if persecucion::alcanzada(&self.estado.reglas.economia.militar, &info, 0) {
            self.terminar_persecucion(perseguidor, Fin::Alcanzada, t);
        }
    }

    /// Lo que las reglas de la persecución necesitan saber de la presa.
    fn presa_de(&self, faccion_del_perseguidor: Option<u32>, presa: u64, t: Instante) -> Option<Presa> {
        let cosa = self.cosa(presa)?;
        let propia_o_aliada = !self.hostiles(faccion_del_perseguidor, self.faccion_de(&cosa));
        Some(match cosa {
            Cosa::Columna(e) => Presa::Columna { tipo: e.columna.tipo, hay_heroe_sano: self.tiene_heroe_sano(e, t), propia_o_aliada },
            Cosa::Caravana(k) => Presa::Caravana { adjunta: k.estado == EstadoCaravana::Adjunta, propia_o_aliada },
            Cosa::Plaza(_) | Cosa::Avanzada(..) => return None,
        })
    }

    // ───────────────────────────── Seguir a la presa ─────────────────────────────

    /// Una cosa del espacio acaba de anunciarse: si persigue, se reprograma su vista; y quienes la persiguen rehacen su plan (si el suyo cambió) o
    /// solo su vista (si cambió su ritmo).
    pub(super) fn seguir_a(&mut self, entidad: u64, motivo: Motivo) {
        self.programar_vista(entidad);
        let Some(perseguidores) = self.espacio.perseguidores.get(&entidad).cloned() else { return };
        for c in perseguidores {
            match motivo {
                Motivo::Plan => {
                    self.seguir_presa(c);
                }
                Motivo::Ritmo => self.programar_vista(c),
            }
        }
    }

    /// Rehace el plan de `perseguidor` hacia donde estará su presa. `false` si la persecución terminó (la presa se perdió o se puso a cubierto). Una
    /// columna ya en proceso de rehacerse (dos que se persiguen entre sí) no se rehace dos veces en la misma cascada.
    pub(super) fn seguir_presa(&mut self, perseguidor: u64) -> bool {
        if self.siguiendo.contains(&perseguidor) {
            return true;
        }
        let t = self.estado.ahora;
        let Some(presa) = self.estado.ejercitos.get(&perseguidor).and_then(|e| e.persiguiendo) else { return false };
        self.siguiendo.push(perseguidor);
        let seguimiento = self.calcular_seguimiento(perseguidor, presa, t);
        let sigue = self.aplicar_seguimiento(perseguidor, presa, t, seguimiento);
        self.siguiendo.pop();
        sigue
    }

    /// El plan de la presa y dónde está en `t`, si sigue en el mapa y se la puede perseguir: una columna a bordo y sin batalla, o una caravana que viaja.
    fn plan_de_presa(&self, presa: u64, t: Instante) -> Option<(Plan, Punto)> {
        match self.cosa(presa)? {
            Cosa::Columna(e) => self.columna_en_el_mapa(e).then(|| (self.plan_de_columna(e, t), e.posicion_en(t))),
            Cosa::Caravana(k) if matches!(k.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando) => Some((k.plan.clone()?, k.posicion_en(t))),
            Cosa::Caravana(_) => None,
            Cosa::Plaza(_) | Cosa::Avanzada(..) => None,
        }
    }

    /// Qué hay que hacer para seguir a `presa` desde donde está `perseguidor` en `t`. No toca el estado (solo calcula la ruta).
    fn calcular_seguimiento(&mut self, perseguidor: u64, presa: u64, t: Instante) -> Seguimiento {
        let Some((plan_presa, donde_presa)) = self.plan_de_presa(presa, t) else { return Seguimiento::Fin(Fin::ACubierto) };
        let e = &self.estado.ejercitos[&perseguidor];
        let (origen, velocidad) = (e.posicion_en(t), self.velocidad_de_columna(e).max(1));
        let alcance = self.alcance_de_vista(perseguidor).unwrap_or(0);
        if origen.dist(donde_presa) > alcance {
            return Seguimiento::Fin(Fin::Perdida);
        }
        let radio = self.estado.reglas.economia.militar.logistica.radio_encuentro;
        let destino = interceptar(origen, t, velocidad, radio, &plan_presa).map_or(donde_presa, |i| i.punto);
        if destino == origen {
            return Seguimiento::Quieto;
        }
        self.metricas.rutas_calculadas += 1;
        match self.rutas.ruta_con(&mut self.escritorio, origen, destino, &Opciones::default()) {
            Some(c) if c.puntos.len() >= 2 => Seguimiento::Ruta(c),
            Some(_) => Seguimiento::Quieto,
            None => Seguimiento::Fin(Fin::Perdida),
        }
    }

    /// Hace lo que dice el seguimiento. `false` si la persecución terminó.
    fn aplicar_seguimiento(&mut self, perseguidor: u64, presa: u64, t: Instante, seguimiento: Seguimiento) -> bool {
        match seguimiento {
            Seguimiento::Ruta(camino) => self.poner_ruta(perseguidor, t, camino),
            Seguimiento::Quieto => {
                if self.estado.ejercitos[&perseguidor].marchando() {
                    // Parar la columna suelta la presa: se vuelve a fijar.
                    self.detener(perseguidor, t);
                    self.fijar_persecucion(perseguidor, Some(presa));
                }
                self.programar_vista(perseguidor);
            }
            Seguimiento::Fin(fin) => {
                self.terminar_persecucion(perseguidor, fin, t);
                return false;
            }
        }
        true
    }

    /// Programa el instante en que la presa saldrá del alcance de vista de `perseguidor`, plan contra plan, ligado a la generación de la presa.
    pub(super) fn programar_vista(&mut self, perseguidor: u64) {
        let Some(presa) = self.estado.ejercitos.get(&perseguidor).and_then(|e| e.persiguiendo) else { return };
        let (t, alcance) = (self.estado.ahora, self.alcance_de_vista(perseguidor).unwrap_or(0));
        let Some(salida) = self.espacio.rejilla.primera_salida(perseguidor, presa, alcance, t) else { return };
        let clave = self.estado.generaciones.clave(perseguidor);
        let presa_generacion = self.estado.generaciones.actual(presa);
        self.estado.cola.programar(salida, PRIORIDAD_ENCUENTROS, clave, Carga::Militar(SucesoMilitar::PresaFueraDeVista { presa, presa_generacion }));
    }

    /// Venció el instante en que la presa sale de la vista: si nadie cambió de plan, la persecución se cancela.
    pub(super) fn presa_fuera_de_vista(&mut self, perseguidor: u64, presa: u64, presa_generacion: u32, t: Instante) {
        if self.estado.generaciones.actual(presa) != presa_generacion {
            return;
        }
        if self.estado.ejercitos.get(&perseguidor).is_some_and(|e| e.persiguiendo == Some(presa)) {
            self.terminar_persecucion(perseguidor, Fin::Perdida, t);
        }
    }

    // ───────────────────────────── Las órdenes ─────────────────────────────

    /// `perseguir` y `dejarDePerseguir`. Las da el Líder de la columna a bordo. Todo se comprueba antes de tocar nada.
    pub(super) fn ejecutar_persecuciones(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let (heroe, objetivo) = match &p.comando {
            Comando::Perseguir { heroe, objetivo } => (*heroe, Some(*objetivo)),
            Comando::DejarDePerseguir { heroe } => (*heroe, None),
            _ => unreachable!("solo se llama con órdenes de perseguir"),
        };
        self.autorizar_heroe(p, heroe, false)?;
        let (id, e) = self.columna_a_bordo(heroe)?;
        if e.en_batalla {
            return Err(rechazo(rechazos::COLUMNA_EN_BATALLA, "la columna está apartada por una batalla"));
        }
        if e.columna.es_formacion() {
            return Err(rechazo(rechazos::COLUMNA_EN_FORMACION, "una formación no se mueve: espera a ser un ejército"));
        }
        if e.columna.lider != heroe {
            return Err(rechazo(rechazos::COLUMNA_SOLO_EL_LIDER, "solo el Líder decide a quién persigue la columna"));
        }
        let Some(objetivo) = objetivo else {
            if e.persiguiendo.is_none() {
                return Err(Respuesta::sin_cambios(v));
            }
            self.fijar_persecucion(id, None);
            self.evento_de_columna("columna.persecucion_abandonada", t, id, vec![], vec![("heroe", heroe as i64)]);
            return Ok(vec![]);
        };

        // Qué es la presa y dónde está.
        let (presa, info, en_el_mapa, donde) = match objetivo {
            Objetivo::Asentamiento(_) => return Err(rechazo(rechazos::OBJETIVO_NO_SOPORTADO, "a una plaza no se la persigue: se inspecciona o se asedia")),
            Objetivo::Ejercito(x) => {
                let o = self.estado.ejercitos.get(&x).ok_or_else(|| Respuesta::rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(x), v))?;
                (x, self.presa_de(e.columna.faccion, x, t).expect("existe"), self.columna_en_el_mapa(o), o.posicion_en(t))
            }
            Objetivo::Caravana(x) => {
                let k = self.estado.comercio.caravana(x).ok_or_else(|| Respuesta::rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(x), v))?;
                let entidad = entidad_caravana(x);
                (entidad, self.presa_de(e.columna.faccion, entidad, t).expect("existe"), matches!(k.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando), self.posicion_de_caravana(x, t).unwrap_or(k.posicion))
            }
        };
        let de_error = |er| {
            let (c, m) = rechazo_de_persecucion(er);
            rechazo(c, m)
        };
        if presa == id {
            return Err(de_error(ErrorPersecucion::ASiMisma));
        }
        let (distancia, alcance) = (e.posicion_en(t).dist(donde), self.alcance_de_vista(id).unwrap_or(0));
        let perseguidor = Perseguidor { tipo: e.columna.tipo, hay_heroe_sano: self.tiene_heroe_sano(e, t) };
        persecucion::validar(&perseguidor, &info, en_el_mapa, distancia, alcance).map_err(de_error)?;
        if e.persiguiendo == Some(presa) {
            return Err(Respuesta::sin_cambios(v));
        }

        // Ya está al alcance: la persecución empieza y termina en el mismo instante, sin moverse.
        let datos = vec![("heroe", heroe as i64), ("presa", if es_entidad_caravana(presa) { id_de_entidad(presa) } else { presa } as i64), ("caravana", es_entidad_caravana(presa) as i64)];
        if persecucion::alcanzada(&self.estado.reglas.economia.militar, &info, distancia) {
            self.fijar_persecucion(id, Some(presa));
            self.evento_de_columna("columna.persecucion_iniciada", t, id, vec![], datos);
            self.terminar_persecucion(id, Fin::Alcanzada, t);
            return Ok(vec![]);
        }
        // Si no hay camino por tierra, se rechaza sin tocar nada.
        let seguimiento = match self.calcular_seguimiento(id, presa, t) {
            Seguimiento::Fin(_) => return Err(rechazo(rechazos::RUTA_IMPOSIBLE, "no hay ruta por tierra hasta la presa")),
            s => s,
        };
        self.fijar_persecucion(id, Some(presa));
        self.aplicar_seguimiento(id, presa, t, seguimiento);
        self.evento_de_columna("columna.persecucion_iniciada", t, id, vec![], datos);
        Ok(vec![])
    }
}
