//! Las misiones en el núcleo (mecánica 49, `Docs/Mechanics/Misiones.md`): de dónde sale cada avance, los contadores de estado, cumplir y reclamar. Lo puro (el catálogo y el estado de cada
//! dueño) es de `crate::misiones`.
//!
//! Cada avance se suma **en origen**, en el sitio que produce el hecho ([`Nucleo::avance`]); los contadores de estado (`crate::misiones::DE_ESTADO`) se leen con [`Nucleo::mirar_misiones`] cuando
//! ocurre lo que puede cambiarlos y cuando una misión que los usa se activa.

use super::*;
use crate::estado::Dato;
use crate::misiones::{Dueno, Recompensa};
use asentamiento::catalogo::Tipo;
use asentamiento::asentamiento::Evento as EventoEco;
use asentamiento::Recurso;
use militar::heroe::Residencia;

/// El id de un dueño en el contrato: `heroe-N`, `as-N` o `faccion-N`.
pub fn id_de_dueno(d: Dueno) -> String {
    match d {
        Dueno::Heroe(h) => ids::heroe(h),
        Dueno::Asentamiento(a) => ids::asentamiento(a),
        Dueno::Faccion(f) => ids::faccion(f),
    }
}

/// El valor de un contador de estado de `d` (Misiones §3). Lo usa también la foto anterior a las misiones para arrancarlas.
pub(crate) fn valor_de_estado(e: &crate::estado::Estado, d: Dueno, contador: &str) -> u64 {
    match (d, contador) {
        (Dueno::Asentamiento(a), _) => {
            let Some(a) = e.asentamientos.get(&a) else { return 0 };
            let tiene = |t: Tipo| u64::from(a.economia.edificios.iter().any(|x| x.tipo == t && x.activo()));
            match contador {
                "mercado" => tiene(Tipo::Mercado),
                "barracon" => tiene(Tipo::Barracon),
                "nivel" => u64::from(a.nivel()),
                "artesanos" => u64::from(a.economia.poblacion.artesanos > 0),
                "nobles" => u64::from(a.economia.poblacion.nobleza > 0),
                _ => 0,
            }
        }
        (Dueno::Faccion(f), "capital") => u64::from(e.politica.facciones.get(f).is_some_and(|f| f.capital_designada_en.is_some())),
        _ => 0,
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Las misiones de un dueño (`None`: aún no tiene nada contado ni cumplido; vale como todo a 0).
    pub fn misiones_de(&self, d: Dueno) -> Option<&crate::misiones::DeDueno> {
        self.estado.misiones.de(d)
    }

    // ───────────────────────────── Avanzar ─────────────────────────────

    /// Suma `n` al contador de `d`, si alguna misión activa lo usa, y fija las que se cumplen.
    pub(super) fn avance(&mut self, t: Instante, d: Dueno, contador: &str, n: u64) {
        let r = self.estado.reglas.economia.misiones.clone();
        let antes = self.estado.misiones.de(d).cloned();
        let cumplidas = self.estado.misiones.sumar(&r, d, contador, n);
        self.tras_avance(t, d, cumplidas, false);
        self.tocar_si_cambio(d, antes);
    }

    /// El avance de las misiones viaja con su dueño: si cambió algo, se toca.
    fn tocar_si_cambio(&mut self, d: Dueno, antes: Option<crate::misiones::DeDueno>) {
        if self.estado.misiones.de(d) != antes.as_ref() {
            self.tocar_dueno(d);
        }
    }

    /// Lo mismo para cada héroe de la lista.
    pub(super) fn avance_de_heroes(&mut self, t: Instante, heroes: &[u64], contador: &str, n: u64) {
        for &h in heroes {
            self.avance(t, Dueno::Heroe(h), contador, n);
        }
    }

    /// Mira los contadores de estado que usan las misiones activas de `d` y fija las que se cumplen.
    pub(super) fn mirar_misiones(&mut self, t: Instante, d: Dueno) {
        let antes = self.estado.misiones.de(d).cloned();
        self.tras_avance(t, d, Vec::new(), true);
        self.tocar_si_cambio(d, antes);
    }

    /// Anuncia las misiones cumplidas y, si se cumplió alguna (que activa sus siguientes) o se pide `mirar`, lee los contadores de estado de las activas, hasta que no se cumple ninguna más.
    fn tras_avance(&mut self, t: Instante, d: Dueno, mut cumplidas: Vec<u16>, mut mirar: bool) {
        let r = self.estado.reglas.economia.misiones.clone();
        loop {
            mirar |= !cumplidas.is_empty();
            for i in std::mem::take(&mut cumplidas) {
                self.anunciar_cumplida(t, d, &r.misiones[i as usize].id);
            }
            if !mirar {
                return;
            }
            mirar = false;
            for c in self.estado.misiones.de_estado_activos(&r, d) {
                let valor = valor_de_estado(&self.estado, d, c);
                cumplidas.extend(self.estado.misiones.record(&r, d, c, valor));
            }
            if cumplidas.is_empty() {
                return;
            }
        }
    }

    /// `mision.cumplida { mision }` al dueño (el héroe, quien oye la plaza o la Facción) y su entidad tocada.
    fn anunciar_cumplida(&mut self, t: Instante, d: Dueno, id: &str) {
        let valores = vec![("mision", Dato::Texto(id.to_owned())), ("dueno", Dato::Texto(id_de_dueno(d)))];
        let base = EventoNucleo { valores, ..EventoNucleo::nuevo("mision.cumplida", t, vec![]) };
        let evento = match d {
            Dueno::Heroe(h) => EventoNucleo { heroes: vec![h], ..base },
            Dueno::Asentamiento(a) => EventoNucleo { asentamientos: vec![a], punto: self.estado.asentamientos.get(&a).map(|x| x.posicion), ..base },
            Dueno::Faccion(f) => EventoNucleo { facciones: vec![f], ..base },
        };
        self.tr.emitir(&self.estado, evento);
        self.tocar_dueno(d);
    }

    /// Las misiones de un dueño viajan con él: el héroe, la plaza o la Facción.
    pub(super) fn tocar_dueno(&mut self, d: Dueno) {
        match d {
            Dueno::Heroe(h) => self.tr.tocar(Clase::Heroe, h),
            Dueno::Asentamiento(a) => self.tr.tocar(Clase::Asentamiento, a),
            Dueno::Faccion(f) => self.tr.tocar(Clase::Faccion, u64::from(f)),
        }
    }

    /// Los sucesos de la economía de una plaza (`anotar_sucesos`): lo que completa y lo que puede cambiar sus contadores de estado (edificios, nivel, población).
    pub(super) fn misiones_de_sucesos(&mut self, id: u64, t: Instante, sucesos: &[EventoEco]) {
        let d = Dueno::Asentamiento(id);
        let mut mirar = false;
        for s in sucesos {
            match s {
                EventoEco::PuebloCompletado { .. } => self.avance(t, d, "pueblos", 1),
                EventoEco::RecintoCompletado { .. } => self.avance(t, d, "recintos", 1),
                EventoEco::AvanzadaCompletada { .. } => self.avance(t, d, "avanzadas", 1),
                EventoEco::EdificioCompletado { .. } | EventoEco::MejoraCompletada { .. } | EventoEco::NivelSubio { .. } | EventoEco::PrimerosArtesanos | EventoEco::PrimerosNobles => mirar = true,
                _ => {}
            }
        }
        if mirar {
            self.mirar_misiones(t, d);
        }
    }

    /// Los soldados reclutados por un héroe (Misiones §3): la escuadra nueva y los soldados por la clase de edificio de su tropa.
    pub(super) fn avance_de_reclutamiento(&mut self, t: Instante, heroe: u64, edificio: Tipo, soldados: u64, nueva: bool) {
        let d = Dueno::Heroe(heroe);
        if nueva {
            self.avance(t, d, "escuadrones", 1);
        }
        let clase = match edificio {
            Tipo::CentroUrbano | Tipo::Barracon => "infanteria",
            Tipo::GaleriaDeTiro => "tiro",
            Tipo::Caballerizas => "caballeria",
            _ => return,
        };
        self.avance(t, d, clase, soldados);
    }

    // ───────────────────────────── Reclamar ─────────────────────────────

    /// `reclamarRecompensa` (Misiones §4): el héroe de la cuenta reclama la recompensa de una misión cumplida de `dueno`.
    pub(super) fn reclamar_recompensa(&mut self, t: Instante, p: &Peticion, heroe: u64, dueno: Dueno, mision: &str) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        // La administración no necesita héroe.
        if !p.admin {
            self.autorizar_heroe(p, heroe, false)?;
        }
        let r = self.estado.reglas.economia.misiones.clone();
        let i = r.buscar(dueno.clase(), mision).ok_or_else(|| rechazo(rechazos::MISION_DESCONOCIDA, format!("{mision} no es una misión de {}", id_de_dueno(dueno))))?;
        let x = self.estado.misiones.de(dueno);
        if x.is_some_and(|x| x.reclamadas.contains(&i)) {
            return Err(rechazo(rechazos::MISION_YA_RECLAMADA, format!("{mision} ya se reclamó")));
        }
        if !x.is_some_and(|x| x.cumplidas.contains(&i)) {
            return Err(rechazo(rechazos::MISION_NO_CUMPLIDA, format!("{mision} aún no se ha cumplido")));
        }
        if !p.admin && !self.puede_reclamar(heroe, dueno) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, format!("no puedes reclamar las misiones de {}", id_de_dueno(dueno))));
        }
        let datos = match (&r.misiones[i as usize].recompensa, dueno) {
            (&Recompensa::Oro(oro), Dueno::Heroe(h)) => {
                let hn = self.heroe(h).expect("autorizado");
                if hn.heroe.residencia.is_none() {
                    return Err(rechazo(rechazos::HEROE_SIN_RESIDENCIA, "sin residencia no hay almacén personal donde dejar el oro".into()));
                }
                let lleva: Mil = hn.almacen_personal.values().sum();
                if lleva + oro > self.estado.reglas.economia.comercio.mercenarios.capacidad_almacen_personal {
                    return Err(rechazo(rechazos::ALMACEN_PERSONAL_LLENO, "el oro no cabe en tu almacén personal".into()));
                }
                *self.heroe_mut(h).almacen_personal.entry(Recurso::Oro).or_insert(0) += oro;
                vec![("oro", oro)]
            }
            (Recompensa::Recursos(lista), Dueno::Asentamiento(a)) => {
                let plaza = self.estado.asentamientos.get_mut(&a).ok_or_else(|| rechazo(rechazos::ASENTAMIENTO_DESCONOCIDO, ids::asentamiento(a)))?;
                if lista.iter().any(|&(rec, n)| plaza.economia.almacen.espacio(rec) < n) {
                    return Err(rechazo(rechazos::MISION_ALMACEN_LLENO, "la recompensa no cabe en el almacén de la plaza".into()));
                }
                for &(rec, n) in lista {
                    plaza.economia.almacen.agregar(rec, n);
                }
                self.tr.tocar(Clase::Asentamiento, a);
                vec![]
            }
            (&Recompensa::Experiencia(xp), Dueno::Faccion(f)) => {
                self.dar_experiencia(t, f, xp, super::experiencia::motivo::MISION);
                vec![("xp", xp)]
            }
            _ => unreachable!("cada clase de dueño tiene su recompensa (Reglas::desde)"),
        };
        self.estado.misiones.duenos.entry(dueno).or_default().reclamadas.insert(i);
        self.tocar_dueno(dueno);
        let valores = vec![("mision", Dato::Texto(mision.to_owned())), ("dueno", Dato::Texto(id_de_dueno(dueno)))];
        let base = EventoNucleo { valores, heroes: vec![heroe], ..EventoNucleo::nuevo("mision.reclamada", t, datos) };
        let evento = match dueno {
            Dueno::Heroe(_) => base,
            Dueno::Asentamiento(a) => EventoNucleo { asentamientos: vec![a], punto: self.estado.asentamientos.get(&a).map(|x| x.posicion), ..base },
            Dueno::Faccion(f) => EventoNucleo { facciones: vec![f], ..base },
        };
        self.tr.emitir(&self.estado, evento);
        Ok(())
    }

    /// ¿Puede `heroe` reclamar las misiones de `dueno`? Las suyas; las de una plaza, su Gobernador o, sin Gobernador, uno de sus residentes; las de una Facción, su Rey.
    fn puede_reclamar(&self, heroe: u64, dueno: Dueno) -> bool {
        match dueno {
            Dueno::Heroe(h) => h == heroe,
            Dueno::Asentamiento(a) => match self.estado.asentamientos.get(&a).and_then(|x| x.cargos.titular(asentamiento::Cargo::Gobernador)) {
                Some(g) => g == heroe,
                None => self.heroe(heroe).is_some_and(|h| h.heroe.residencia == Some(Residencia::Asentamiento(a as u32))),
            },
            Dueno::Faccion(f) => self.estado.politica.facciones.get(f).is_some_and(|x| x.rey == Some(heroe)),
        }
    }
}

/// Una foto anterior a las misiones (BAN45 o antes): cada plaza y cada Facción miran sus contadores de estado, sin hecho (Misiones §8).
pub(crate) fn arrancar_misiones(e: &mut crate::estado::Estado) {
    let r = e.reglas.economia.misiones.clone();
    let duenos: Vec<Dueno> = e.asentamientos.keys().map(|&a| Dueno::Asentamiento(a)).chain(e.politica.facciones.iter().map(|f| Dueno::Faccion(f.id))).collect();
    for d in duenos {
        loop {
            let mut nuevas = Vec::new();
            for c in e.misiones.de_estado_activos(&r, d) {
                let valor = valor_de_estado(e, d, c);
                nuevas.extend(e.misiones.record(&r, d, c, valor));
            }
            if nuevas.is_empty() {
                break;
            }
        }
    }
}
