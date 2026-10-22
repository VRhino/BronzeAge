//! El espacio del mundo reactivo dentro del núcleo (Reloj_Y_Eventos §2, Combate §2, Movimiento_Ejercitos §5 y §7, fase 4C): qué hay en el mapa, cuándo se
//! cruzan dos de sus cosas y qué hecho deja cada cruce.
//!
//! # Qué está en la rejilla
//! La [`Rejilla`] de `encuentros` reúne todo lo que cuenta en el mapa, cada cosa con su plan:
//! - una **columna en el mapa**, en marcha (su plan) o acampada (quieta hasta el horizonte). Está en el mapa si alguno de sus héroes va a bordo y no está
//!   apartada por una batalla. Una columna que ha entrado en un lugar —su gente dentro, con todo lo suyo— o está en batalla **sale**, y con ella se van sus
//!   encuentros y sus perseguidores (`columna.presa_a_cubierto`);
//! - una **caravana** en tránsito o de regreso (su plan; las adjuntas viajan con su columna y no están);
//! - una **plaza**, fija desde que se funda hasta que cae en ruinas;
//! - una **avanzada** (fase 4F), fija desde que acaba su primera obra hasta que la destruyen o su madre cae en ruinas (con sus ruinas, que un ataque puede derribar
//!   pasado el día de invulnerabilidad, y sin las ruinas de ciudadela sin dueño).
//!
//! No es estado: se reconstruye de [`Estado`] al abrir el núcleo (`reconstruir_espacio`), así que ni entra en el hash ni en las fotos. Los encuentros
//! que ya estaban programados viajan en la cola (`Carga::Encuentro`).
//!
//! # Cuándo se recalcula
//! Cada vez que una cosa cambia de plan, de generación (un ritmo nuevo sube la de la columna y descarta lo programado) o entra en el mapa, se apunta de
//! nuevo y se calculan **solo sus pares**: cada par a ≤ `radioEncuentro` es un evento exacto a su primer instante, ligado a la generación de las dos
//! cosas. Una columna que ya estaba junto a otra cuando cambia de plan no la «encuentra» otra vez ([`Entradas::Nuevas`]); una que entra en el mapa
//! junto a otra, sí (`Entradas::Todas`).
//!
//! # Qué deja un encuentro
//! Solo un **hecho**. Se filtra al vencer, con el estado de ese instante exacto, y solo lo ven columnas en el mapa y con algún héroe sano, de Facciones
//! distintas y no aliadas, de la misma clase (personal con personal, ejército con ejército): una caravana, una plaza o una avanzada no tienen clase. No abre ningún
//! combate: la orden de atacar es del jugador o del bot y se resuelve en la fase 4D. Los hechos: `ejercito.encuentro` (dos columnas),
//! `caravana.encuentro` (una caravana con una columna o con otra caravana), `asentamiento.encuentro` (una columna junto a una plaza ajena) y `avanzada.encuentro`
//! (una columna junto a una avanzada ajena que se puede atacar ahora: ni una obra, ni unas ruinas invulnerables).

use super::*;
use crate::estado::{Asentamiento, SIN_BANDERA};
use crate::estado_combate::{entidad_avanzada, entidad_bandidos, es_entidad_avanzada, es_entidad_bandidos};
use crate::estado_comercio::{entidad_caravana, es_entidad_caravana, id_de_entidad};
use super::persecuciones::Fin;
use crate::estado_militar::Relacion;
use ::encuentros::{plan_quieto, Entradas, Rejilla, PRIORIDAD_ENCUENTROS};
use asentamiento::avanzadas::{Avanzada, Estado as EstadoAvanzada};
use cola::Clave;
use combate::avanzada::atacable;
use comercio::caravana::Caravana;
use militar::presencia::Ubicacion;
use std::collections::{BTreeMap, BTreeSet};

/// Lo que el núcleo sabe del espacio y no es estado: la rejilla y, de cada presa, quién la persigue (índice de `Ejercito::persiguiendo`).
pub(super) struct Espacio {
    pub rejilla: Rejilla,
    pub perseguidores: BTreeMap<u64, BTreeSet<u64>>,
}

impl Espacio {
    pub fn nuevo() -> Self {
        Self { rejilla: Rejilla::para_mundo_real(), perseguidores: BTreeMap::new() }
    }
}

/// Por qué se vuelve a anunciar una columna: si cambió su **plan** (también si es nueva en el mapa o la quitan) sus perseguidores rehacen el suyo; si
/// solo cambió su **ritmo** (víveres, tropa), el plan es el mismo y solo hay que reprogramar lo que dependía de su generación.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Motivo {
    Plan,
    Ritmo,
}

/// Una cosa del espacio, según su entidad.
pub(super) enum Cosa<'a> {
    Columna(&'a Ejercito),
    Caravana(&'a Caravana),
    Plaza(&'a Asentamiento),
    /// Una avanzada con su plaza madre.
    Avanzada(&'a Avanzada, &'a Asentamiento),
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// ¿Está la columna en el mapa? Alguno de sus héroes va a bordo (no dentro de un lugar ni fuera del mundo) y no la aparta una batalla.
    pub fn en_el_mapa(&self, columna: u64) -> bool {
        self.estado.ejercitos.get(&columna).is_some_and(|e| self.columna_en_el_mapa(e))
    }

    /// Cuántas cosas hay ahora en el espacio (columnas en el mapa, caravanas en tránsito y plazas).
    pub fn cosas_en_el_espacio(&self) -> usize {
        self.espacio.rejilla.len()
    }

    /// La relación que la administración fijó entre dos Facciones, si la hay.
    pub fn relacion(&self, a: u32, b: u32) -> Option<Relacion> {
        self.estado.militar.relacion(a, b)
    }

    pub(super) fn columna_en_el_mapa(&self, e: &Ejercito) -> bool {
        !e.en_batalla && e.columna.participantes.iter().any(|p| self.heroe(p.heroe).is_some_and(|h| h.heroe.presencia.ubicacion == Ubicacion::Columna(e.id)))
    }

    /// ¿Lleva la columna algún héroe sano en `t`? (Un herido no persigue, no se le persigue y no entra en batallas; si todos lo están, es intocable.)
    pub(super) fn tiene_heroe_sano(&self, e: &Ejercito, t: Instante) -> bool {
        e.columna.participantes.iter().any(|p| self.heroe(p.heroe).is_some_and(|h| !h.heroe.esta_herido(t)))
    }

    pub(super) fn cosa(&self, entidad: u64) -> Option<Cosa<'_>> {
        if es_entidad_caravana(entidad) {
            return self.estado.comercio.caravana(id_de_entidad(entidad)).map(Cosa::Caravana);
        }
        if es_entidad_avanzada(entidad) {
            let id = id_de_entidad(entidad);
            return self.estado.asentamientos.values().find_map(|a| a.economia.avanzada(id).map(|v| Cosa::Avanzada(v, a)));
        }
        self.estado.ejercitos.get(&entidad).map(Cosa::Columna).or_else(|| self.estado.asentamientos.get(&entidad).map(Cosa::Plaza))
    }

    /// La Facción de una cosa: la de la columna (puede no tener), la de la plaza o la de la plaza de origen de la caravana.
    pub(super) fn faccion_de(&self, c: &Cosa<'_>) -> Option<u32> {
        match c {
            Cosa::Columna(e) => e.columna.faccion,
            Cosa::Plaza(a) | Cosa::Avanzada(_, a) => Some(a.faccion),
            Cosa::Caravana(k) => self.estado.asentamientos.get(&(k.origen as u64)).map(|a| a.faccion),
        }
    }

    /// Dónde está una cosa en `t`.
    pub(super) fn donde(&self, c: &Cosa<'_>, t: Instante) -> Punto {
        match c {
            Cosa::Columna(e) => e.posicion_en(t),
            Cosa::Plaza(a) => a.posicion,
            Cosa::Avanzada(v, _) => v.posicion,
            Cosa::Caravana(k) => self.posicion_de_caravana(k.id, t).unwrap_or(k.posicion),
        }
    }

    /// ¿Son enemigas dos Facciones? Distintas y no aliadas; sin Facción nadie es de los tuyos.
    pub(super) fn hostiles(&self, a: Option<u32>, b: Option<u32>) -> bool {
        !(a.is_some() && a == b) && !self.estado.militar.aliadas(a, b)
    }

    // ───────────────────────────── Altas y bajas ─────────────────────────────

    /// Reconstruye el espacio de `estado`: lo que se hace al abrir el núcleo, nuevo o recuperado de una foto. Lo que ya está programado en la cola no
    /// se toca.
    pub(super) fn reconstruir_espacio(&mut self) {
        let t = self.estado.ahora;
        let mut puestos: Vec<(u64, Plan)> = Vec::new();
        puestos.extend(self.estado.ejercitos.values().filter(|e| self.columna_en_el_mapa(e)).map(|e| (e.id, self.plan_de_columna(e, t))));
        puestos.extend(self.estado.comercio.flota.iter().filter(|k| matches!(k.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando)).filter_map(|k| Some((entidad_caravana(k.id), k.plan.clone()?))));
        puestos.extend(self.estado.asentamientos.values().map(|a| (a.id, plan_quieto(a.posicion, t, 0))));
        let puestos_de_mapa = self.estado.asentamientos.values().flat_map(|a| a.economia.avanzadas.iter()).filter(|v| v.estado != EstadoAvanzada::EnConstruccion);
        puestos.extend(puestos_de_mapa.map(|v| (entidad_avanzada(v.id), plan_quieto(v.posicion, t, 0))));
        puestos.extend(self.estado.combate.bandidos.values().map(|b| (entidad_bandidos(b.id), plan_quieto(b.posicion, t, 0))));
        for (entidad, plan) in puestos {
            let g = if self.estado.asentamientos.contains_key(&entidad) { 0 } else { self.estado.generaciones.actual(entidad) };
            self.espacio.rejilla.apuntar(entidad, g, &plan);
        }
        for e in self.estado.ejercitos.values() {
            if let Some(presa) = e.persiguiendo {
                self.espacio.perseguidores.entry(presa).or_default().insert(e.id);
            }
        }
    }

    /// El plan con el que una columna está en el espacio: el suyo si marcha, o quieta donde está.
    pub(super) fn plan_de_columna(&self, e: &Ejercito, t: Instante) -> Plan {
        if e.plan.vertices.len() > 1 {
            e.plan.clone()
        } else {
            plan_quieto(e.posicion_en(t), t, 0)
        }
    }

    /// Una plaza recién fundada es una cosa fija del espacio.
    pub(super) fn situar_plaza(&mut self, id: u64, posicion: Punto) {
        self.espacio.rejilla.apuntar(id, 0, &plan_quieto(posicion, self.estado.ahora, 0));
    }

    /// Una plaza cayó en ruinas: ya no está en el espacio.
    pub(super) fn quitar_plaza(&mut self, id: u64) {
        self.espacio.rejilla.quitar(id);
    }

    // ───────────────────────────── Anunciar ─────────────────────────────

    /// Cambió algo de una columna que mueve sus eventos (su plan, su generación, quién va a bordo): se vuelve a apuntar y a calcular sus encuentros, o se
    /// la saca del espacio si ya no está en el mapa o ya no existe.
    pub(super) fn anunciar_columna(&mut self, id: u64, motivo: Motivo) {
        let t = self.estado.ahora;
        let plan = match self.estado.ejercitos.get(&id) {
            Some(e) if self.columna_en_el_mapa(e) => self.plan_de_columna(e, t),
            _ => return self.apartar(id),
        };
        self.situar(id, plan, motivo);
    }

    /// Una caravana tiene plan nuevo (`Some`) o ya no está en el mapa (`None`: llegó, se perdió, se aparcó o la aparta una batalla).
    pub(super) fn anunciar_caravana(&mut self, entidad: u64, plan: Option<Plan>) {
        match plan {
            Some(plan) => self.situar(entidad, plan, Motivo::Plan),
            None => self.apartar(entidad),
        }
    }

    /// Una columna o una caravana sale del espacio: sus encuentros pendientes mueren con su generación, deja de perseguir y quien la perseguía la da por
    /// puesta a cubierto.
    pub(super) fn apartar(&mut self, entidad: u64) {
        self.espacio.rejilla.quitar(entidad);
        self.fijar_persecucion(entidad, None);
        self.soltar_perseguidores(entidad, Fin::ACubierto);
    }

    /// Apunta `entidad` con `plan`, programa los encuentros de ese plan con lo que ya hay y rehace lo que depende de él (las persecuciones).
    fn situar(&mut self, entidad: u64, plan: Plan, motivo: Motivo) {
        let (t, g) = (self.estado.ahora, self.estado.generaciones.actual(entidad));
        let entradas = if self.espacio.rejilla.contiene(entidad) { Entradas::Nuevas } else { Entradas::Todas };
        self.espacio.rejilla.apuntar(entidad, g, &plan);
        let radio = self.estado.reglas.economia.militar.logistica.radio_encuentro;
        let clave = Clave { entidad, generacion: g };
        for e in self.espacio.rejilla.encuentros_de(entidad, &plan, radio, entradas) {
            if self.interesa(entidad, e.otra) {
                self.estado.cola.programar(e.instante.max(t), PRIORIDAD_ENCUENTROS, clave, Carga::Encuentro { otra: e.otra, otra_generacion: e.otra_generacion });
            }
        }
        if self.atrae_bandidos(entidad) {
            self.programar_acosos(entidad, &plan, entradas);
        }
        self.seguir_a(entidad, motivo);
    }

    /// ¿Merece programarse el encuentro entre `entidad` (una columna o una caravana) y `otra`? Es un filtro barato que solo descarta lo que nunca será
    /// noticia: lo de la misma Facción y una caravana junto a una plaza. El resto se decide al vencer, con el estado de ese instante.
    fn interesa(&self, entidad: u64, otra: u64) -> bool {
        let (Some(a), Some(b)) = (self.cosa(entidad), self.cosa(otra)) else { return false };
        if matches!((&a, &b), (Cosa::Caravana(_), Cosa::Plaza(_) | Cosa::Avanzada(..))) {
            return false;
        }
        let (fa, fb) = (self.faccion_de(&a), self.faccion_de(&b));
        !(fa.is_some() && fa == fb)
    }

    // ───────────────────────────── El encuentro ─────────────────────────────

    /// Vence el primer acercamiento exacto entre `entidad` y `otra`. La clave de la cola ya garantiza que `entidad` no cambió de plan; falta la otra.
    pub(super) fn encuentro_vence(&mut self, entidad: u64, otra: u64, otra_generacion: u32, t: Instante) {
        if es_entidad_bandidos(entidad) || es_entidad_bandidos(otra) {
            return self.acoso_vence(entidad, otra, otra_generacion, t);
        }
        let (Some(a), Some(b)) = (self.cosa(entidad), self.cosa(otra)) else { return };
        if !matches!(b, Cosa::Plaza(_)) && self.estado.generaciones.actual(otra) != otra_generacion {
            return;
        }
        if !self.en_juego(&a, t) || !self.en_juego(&b, t) || !self.hostiles(self.faccion_de(&a), self.faccion_de(&b)) {
            return;
        }
        if let (Cosa::Columna(x), Cosa::Columna(y)) = (&a, &b) {
            if x.columna.tipo != y.columna.tipo {
                return;
            }
        }
        let (pa, pb) = (self.donde(&a, t), self.donde(&b, t));
        let punto = Punto::new((pa.x + pb.x) / 2, (pa.y + pb.y) / 2);
        let bandera = |c: &Cosa<'_>| self.faccion_de(c).unwrap_or(SIN_BANDERA) as i64;
        let (lo, hi) = (entidad.min(otra), entidad.max(otra));
        let evento = match (&a, &b) {
            (Cosa::Columna(_), Cosa::Columna(_)) => {
                let (fa, fb) = if entidad < otra { (bandera(&a), bandera(&b)) } else { (bandera(&b), bandera(&a)) };
                EventoNucleo { codigo: "ejercito.encuentro".into(), en: t, ejercitos: vec![lo, hi], asentamientos: vec![], punto: Some(punto), datos: vec![("faccionA", fa), ("faccionB", fb)] }
            }
            (Cosa::Columna(c), Cosa::Caravana(k)) | (Cosa::Caravana(k), Cosa::Columna(c)) => {
                let (fc, fk) = if matches!(a, Cosa::Columna(_)) { (bandera(&a), bandera(&b)) } else { (bandera(&b), bandera(&a)) };
                EventoNucleo { codigo: "caravana.encuentro".into(), en: t, ejercitos: vec![c.id], asentamientos: vec![], punto: Some(punto), datos: vec![("faccionA", fc), ("faccionB", fk), ("caravana", k.id as i64)] }
            }
            (Cosa::Caravana(x), Cosa::Caravana(y)) => {
                let (x, y) = if entidad < otra { (x, y) } else { (y, x) };
                let (fx, fy) = if entidad < otra { (bandera(&a), bandera(&b)) } else { (bandera(&b), bandera(&a)) };
                EventoNucleo { codigo: "caravana.encuentro".into(), en: t, ejercitos: vec![], asentamientos: vec![], punto: Some(punto), datos: vec![("faccionA", fx), ("faccionB", fy), ("caravana", x.id as i64), ("caravanaB", y.id as i64)] }
            }
            (Cosa::Columna(c), Cosa::Plaza(p)) | (Cosa::Plaza(p), Cosa::Columna(c)) => {
                let (fc, fp) = if matches!(a, Cosa::Columna(_)) { (bandera(&a), bandera(&b)) } else { (bandera(&b), bandera(&a)) };
                EventoNucleo { codigo: "asentamiento.encuentro".into(), en: t, ejercitos: vec![c.id], asentamientos: vec![p.id], punto: Some(punto), datos: vec![("faccionA", fc), ("faccionB", fp)] }
            }
            (Cosa::Columna(c), Cosa::Avanzada(v, m)) | (Cosa::Avanzada(v, m), Cosa::Columna(c)) => {
                let (fc, fm) = if matches!(a, Cosa::Columna(_)) { (bandera(&a), bandera(&b)) } else { (bandera(&b), bandera(&a)) };
                EventoNucleo { codigo: "avanzada.encuentro".into(), en: t, ejercitos: vec![c.id], asentamientos: vec![m.id], punto: Some(punto), datos: vec![("faccionA", fc), ("faccionB", fm), ("avanzada", v.id as i64)] }
            }
            _ => return,
        };
        self.tr.eventos.push(evento);
        for (perseguidor, presa) in [(entidad, otra), (otra, entidad)] {
            if self.estado.ejercitos.get(&perseguidor).is_some_and(|e| e.persiguiendo == Some(presa)) {
                self.presa_alcanzada(perseguidor, presa, t);
            }
        }
    }

    /// ¿Cuenta la cosa para un encuentro en `t`? Una columna, en el mapa y con algún héroe sano; una caravana, mientras viaja; una plaza, siempre; una avanzada, mientras se
    /// puede atacar.
    fn en_juego(&self, c: &Cosa<'_>, t: Instante) -> bool {
        match c {
            Cosa::Columna(e) => self.columna_en_el_mapa(e) && self.tiene_heroe_sano(e, t),
            Cosa::Caravana(k) => matches!(k.estado, EstadoCaravana::EnTransito | EstadoCaravana::Retornando),
            Cosa::Plaza(_) => true,
            Cosa::Avanzada(v, _) => atacable(v.estado, t),
        }
    }

    // ───────────────────────────── Relaciones ─────────────────────────────

    /// `fijarRelacion` (andamiaje de administración, Q6 del sistema 4): fija, cambia o quita la relación entre dos Facciones. Lo sustituirá el sistema
    /// político.
    pub(super) fn fijar_relacion(&mut self, t: Instante, p: &Peticion, a: u32, b: u32, relacion: Option<Relacion>) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        if !p.admin {
            return Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, "solo administración fija relaciones entre Facciones", v));
        }
        if a == b {
            return Err(Respuesta::rechazo(rechazos::RELACION_CONSIGO_MISMA, "una Facción no tiene relación consigo misma", v));
        }
        if self.estado.militar.relacion(a, b) == relacion {
            return Err(Respuesta { resultado: Resultado::SinCambios, version_tras: v, creadas: vec![] });
        }
        let clave = (a.min(b), a.max(b));
        match relacion {
            Some(r) => self.estado.militar.relaciones.insert(clave, r),
            None => self.estado.militar.relaciones.remove(&clave),
        };
        let codigo = relacion.map_or(-1, |r| r as i64);
        self.emitir_propio(t, "faccion.relacion", vec![], vec![("faccionA", clave.0 as i64), ("faccionB", clave.1 as i64), ("relacion", codigo)]);
        Ok(vec![])
    }
}
