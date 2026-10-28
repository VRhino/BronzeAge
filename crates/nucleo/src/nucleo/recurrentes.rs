//! Las misiones recurrentes y el tablón en el núcleo (mecánica 55, `Docs/Mechanics/Misiones_Recurrentes.md`): la renovación de las 00:00, de dónde sale cada avance, aceptar, elegir,
//! fijar el catálogo y reclamar. Lo puro (el catálogo, los tablones, el periodo de cada misión y la foto) es de `crate::recurrentes`.
//!
//! La renovación es un plazo **global** de la cola (`Carga::Tablon`, entidad `entidad_tablon`, prioridad 10) a las próximas 00:00, que solo está en la cola mientras hay algún tablón. El
//! avance se suma **en origen**, con el de las misiones únicas del héroe ([`Nucleo::avance`]): va al tablón de la plaza donde reside y al de su Facción.

use super::misiones::id_de_dueno;
use super::*;
use crate::estado::{entidad_tablon, Dato, PRIO_TABLON};
use crate::misiones::Dueno;
use crate::recurrentes::{Aceptacion, Ambito, Catalogo, Duracion, EnTablon, EstadoRecurrentes, Progreso, Situacion, Tablon, Tipo, CONTADORES, DIA, HUECOS};
use asentamiento::Recurso;
use militar::heroe::Residencia;
use std::sync::Arc;

/// Los dueños que tienen tablón en el estado `e`: cada plaza y cada Facción que existe.
fn duenos_con_tablon(e: &crate::estado::Estado) -> Vec<Dueno> {
    e.asentamientos.keys().map(|&a| Dueno::Asentamiento(a)).chain(e.politica.facciones.iter().map(|f| Dueno::Faccion(f.id))).collect()
}

/// Una foto anterior a las recurrentes (BAN49 o antes): las semanas cuentan desde el día de la foto y cada plaza y cada Facción sacan su tablón ya (Misiones_Recurrentes §9). La
/// renovación la arma el primer evento u orden que llegue ([`Nucleo::armar_tablon`]).
pub(crate) fn arrancar_tablones(e: &mut crate::estado::Estado) {
    e.recurrentes = EstadoRecurrentes::nuevo(e.reglas.economia.recurrentes.clone(), e.ahora);
    for d in duenos_con_tablon(e) {
        if let Some(t) = e.recurrentes.tablon_nuevo(d, e.ahora) {
            e.recurrentes.tablones.insert(d, t);
        }
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// El tablón de una plaza o de una Facción.
    pub fn tablon_de(&self, d: Dueno) -> Option<&Tablon> {
        self.estado.recurrentes.tablones.get(&d)
    }

    /// El catálogo vigente, el origen de las semanas y la renovación en la cola.
    pub fn recurrentes(&self) -> &EstadoRecurrentes {
        &self.estado.recurrentes
    }

    // ───────────────────────────── El tablón y su renovación ─────────────────────────────

    /// Da su tablón a una plaza recién fundada o a una Facción recién nacida (si no lo tiene): sale ya, hasta el fin del día o de la semana en curso (§2).
    pub(super) fn asegurar_tablon(&mut self, t: Instante, d: Dueno) {
        if self.estado.recurrentes.tablones.contains_key(&d) {
            return;
        }
        if let Some(tablon) = self.estado.recurrentes.tablon_nuevo(d, t) {
            self.estado.recurrentes.tablones.insert(d, tablon);
            self.tocar_dueno(d);
            self.armar_tablon(t);
        }
    }

    /// Mete la renovación en la cola, a las próximas 00:00, si no está y hay algún tablón.
    pub(super) fn armar_tablon(&mut self, t: Instante) {
        let r = &self.estado.recurrentes;
        if r.renovacion_en.is_some() || r.tablones.is_empty() {
            return;
        }
        let en = EstadoRecurrentes::siguiente_dia(t);
        let clave = self.estado.generaciones.clave(entidad_tablon());
        self.estado.cola.programar(en, PRIO_TABLON, clave, Carga::Tablon);
        self.estado.recurrentes.renovacion_en = Some(en);
    }

    /// Las 00:00 (§2): cada tablón se renueva (en la semanal, con lo elegido), el de un dueño que ya no existe se va, el que falta sale, y se arma la siguiente.
    pub(super) fn renovar_tablones(&mut self, t: Instante) {
        let t0 = t.div_euclid(DIA) * DIA;
        self.estado.recurrentes.renovacion_en = None;
        let duenos = duenos_con_tablon(&self.estado);
        let mut tablones = std::mem::take(&mut self.estado.recurrentes.tablones);
        tablones.retain(|d, _| duenos.contains(d));
        for d in &duenos {
            match tablones.get_mut(d) {
                Some(tablon) => self.estado.recurrentes.renovar(Ambito::de(*d).expect("plaza o Facción"), tablon, t0),
                None => {
                    if let Some(tablon) = self.estado.recurrentes.tablon_nuevo(*d, t0) {
                        tablones.insert(*d, tablon);
                    }
                }
            }
        }
        self.estado.recurrentes.tablones = tablones;
        let (mut plazas, mut facciones) = (Vec::new(), Vec::new());
        for &d in &duenos {
            self.tocar_dueno(d);
            match d {
                Dueno::Asentamiento(a) => plazas.push(a),
                Dueno::Faccion(f) => facciones.push(f),
                Dueno::Heroe(_) => {}
            }
        }
        if !duenos.is_empty() {
            let semanal = i64::from(self.estado.recurrentes.es_semanal(t0));
            self.tr.emitir(&self.estado, EventoNucleo { asentamientos: plazas, facciones, ..EventoNucleo::nuevo("tablon.renovado", t, vec![("semanal", semanal)]) });
        }
        self.armar_tablon(t);
    }

    // ───────────────────────────── Avanzar ─────────────────────────────

    /// Lo que hizo el héroe `h` (un contador del héroe o de la participación, §7), en el tablón de la plaza donde reside y en el de su Facción: a sus individuales aceptadas y a las grupales.
    pub(super) fn avance_recurrente(&mut self, t: Instante, h: u64, contador: &str, n: u64) {
        if n == 0 || !CONTADORES.contains(&contador) || self.estado.recurrentes.tablones.is_empty() {
            return;
        }
        let mut duenos = Vec::new();
        if let Some(Residencia::Asentamiento(a)) = self.heroe(h).and_then(|x| x.heroe.residencia) {
            duenos.push(Dueno::Asentamiento(u64::from(a)));
        }
        if let Some(f) = self.estado.politica.facciones.faccion_de(h) {
            duenos.push(Dueno::Faccion(f));
        }
        for d in duenos {
            let Some(tablon) = self.estado.recurrentes.tablones.get_mut(&d) else { continue };
            let (mut cambio, mut cumplidas) = (false, Vec::new());
            for m in tablon.misiones.iter_mut().filter(|m| m.vigente(t) && m.def.condiciones.iter().any(|(c, _)| c == contador)) {
                let antes = m.progreso.clone();
                if m.sumar(h, contador, n) {
                    cumplidas.push((m.def.id.clone(), m.def.es_individual()));
                }
                cambio |= m.progreso != antes;
            }
            if cambio {
                self.tocar_dueno(d);
            }
            for (id, individual) in cumplidas {
                let valores = vec![("mision", Dato::Texto(id)), ("dueno", Dato::Texto(id_de_dueno(d)))];
                let heroes = if individual { vec![h] } else { vec![] };
                self.tr.emitir(&self.estado, self.hecho_de_tablon(d, EventoNucleo { valores, heroes, ..EventoNucleo::nuevo("mision.cumplida", t, vec![]) }));
            }
        }
    }

    /// El hecho de un tablón lo oyen su plaza (y quien está dentro) o su Facción.
    fn hecho_de_tablon(&self, d: Dueno, base: EventoNucleo) -> EventoNucleo {
        match d {
            Dueno::Asentamiento(a) => EventoNucleo { asentamientos: vec![a], punto: self.estado.asentamientos.get(&a).map(|x| x.posicion), ..base },
            Dueno::Faccion(f) => EventoNucleo { facciones: vec![f], ..base },
            Dueno::Heroe(_) => base,
        }
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// El tablón de `dueno`, o el rechazo.
    fn tablon_o_rechazo(&self, dueno: Dueno) -> Result<&Tablon, Respuesta> {
        let v = self.estado.version;
        self.estado.recurrentes.tablones.get(&dueno).ok_or_else(|| Respuesta::rechazo(rechazos::TABLON_DESCONOCIDO, format!("{} no tiene tablón", id_de_dueno(dueno)), v))
    }

    /// ¿Es `h` del ámbito del tablón: residente de la plaza o ciudadano de la Facción?
    fn del_ambito(&self, h: u64, dueno: Dueno) -> bool {
        match dueno {
            Dueno::Asentamiento(a) => self.heroe(h).is_some_and(|x| x.heroe.residencia == Some(Residencia::Asentamiento(a as u32))),
            Dueno::Faccion(f) => self.estado.politica.facciones.faccion_de(h) == Some(f),
            Dueno::Heroe(_) => false,
        }
    }

    /// ¿Es `h` titular del cargo `c` (su clave) en el ámbito del tablón?
    fn tiene_cargo(&self, h: u64, dueno: Dueno, c: &str) -> bool {
        match dueno {
            Dueno::Asentamiento(a) => {
                let Some(cargo) = asentamiento::Cargo::desde_clave(c) else { return false };
                self.estado.asentamientos.get(&a).is_some_and(|x| x.cargos.es_titular(cargo, h))
            }
            Dueno::Faccion(f) => self.estado.politica.facciones.get(f).is_some_and(|x| match c {
                "rey" => x.rey == Some(h),
                "embajador" => x.embajador == Some(h),
                _ => false,
            }),
            Dueno::Heroe(_) => false,
        }
    }

    /// `aceptarMision` (§3): el héroe del ámbito acepta una individual vigente del tablón, si su cargo, su periodo y el tope semanal lo dejan.
    pub(super) fn aceptar_mision(&mut self, t: Instante, p: &Peticion, heroe: u64, dueno: Dueno, mision: &str) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let tablon = self.tablon_o_rechazo(dueno)?;
        if !self.del_ambito(heroe, dueno) {
            let que = if matches!(dueno, Dueno::Faccion(_)) { "ciudadano de" } else { "residente de" };
            return Err(rechazo(rechazos::MISION_FUERA_DE_AMBITO, format!("{} no es {que} {}", ids::heroe(heroe), id_de_dueno(dueno))));
        }
        let Some(i) = tablon.vigente(mision, t) else {
            return Err(match self.estado.recurrentes.catalogo.buscar(mision) {
                Some(_) => rechazo(rechazos::MISION_NO_DISPONIBLE, format!("{mision} no está hoy en el tablón de {}", id_de_dueno(dueno))),
                None => rechazo(rechazos::MISION_DESCONOCIDA, format!("{mision} no es una misión recurrente")),
            });
        };
        let m = &tablon.misiones[i];
        let Tipo::Individual { limite_semanal } = m.def.tipo else {
            return Err(rechazo(rechazos::MISION_GRUPAL, format!("{mision} es grupal: cuenta lo de todos sin aceptarla")));
        };
        if let Some(c) = &m.def.cargo {
            if !self.tiene_cargo(heroe, dueno, c) {
                return Err(rechazo(rechazos::MISION_PIDE_CARGO, format!("{mision} es del cargo {c}")));
            }
        }
        if matches!(&m.progreso, Progreso::Individual(a) if a.contains_key(&heroe)) {
            return Err(rechazo(rechazos::MISION_YA_ACEPTADA, format!("ya aceptaste {mision} en este periodo")));
        }
        if tablon.aceptadas_semana.get(mision).copied().unwrap_or(0) >= limite_semanal {
            return Err(rechazo(rechazos::MISION_TOPE_SEMANAL, format!("{mision} ya se aceptó {limite_semanal} veces esta semana")));
        }
        let tablon = self.estado.recurrentes.tablones.get_mut(&dueno).expect("mirado");
        if let Progreso::Individual(a) = &mut tablon.misiones[i].progreso {
            a.insert(heroe, Aceptacion { contadores: BTreeMap::new(), situacion: Situacion::Aceptada });
        }
        *tablon.aceptadas_semana.entry(mision.to_owned()).or_insert(0) += 1;
        self.tocar_dueno(dueno);
        let valores = vec![("mision", Dato::Texto(mision.to_owned())), ("dueno", Dato::Texto(id_de_dueno(dueno)))];
        let evento = EventoNucleo { valores, heroes: vec![heroe], ..EventoNucleo::nuevo("mision.aceptada", t, vec![("heroe", heroe as i64)]) };
        self.tr.emitir(&self.estado, self.hecho_de_tablon(dueno, evento));
        Ok(())
    }

    /// `elegirTablon` (§4): el Gobernador de la plaza o el Rey de la Facción elige lo que sale desde la próxima renovación semanal. `Ok(false)`: es lo que ya estaba elegido.
    pub(super) fn elegir_tablon(&mut self, p: &Peticion, heroe: u64, dueno: Dueno, diarias: &[String], semanales: &[String]) -> Result<bool, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        self.autorizar_heroe(p, heroe, false)?;
        let tablon = self.tablon_o_rechazo(dueno)?;
        let manda = match dueno {
            Dueno::Asentamiento(a) => self.estado.asentamientos.get(&a).and_then(|x| x.cargos.titular(asentamiento::Cargo::Gobernador)) == Some(heroe),
            Dueno::Faccion(f) => self.estado.politica.facciones.get(f).is_some_and(|x| x.rey == Some(heroe)),
            Dueno::Heroe(_) => false,
        };
        if !p.admin && !manda {
            let quien = if matches!(dueno, Dueno::Faccion(_)) { "su Rey" } else { "su Gobernador" };
            return Err(rechazo(rechazos::NO_AUTORIZADO, format!("el tablón de {} lo elige {quien}", id_de_dueno(dueno))));
        }
        let ambito = Ambito::de(dueno).expect("tiene tablón");
        let catalogo = &self.estado.recurrentes.catalogo;
        for (lista, duracion, que) in [(diarias, Duracion::Dia, "diarias"), (semanales, Duracion::Semana, "semanales")] {
            if lista.len() > HUECOS {
                return Err(rechazo(rechazos::TABLON_ELECCION_INVALIDA, format!("a lo sumo {HUECOS} {que}")));
            }
            for (i, id) in lista.iter().enumerate() {
                if lista[..i].contains(id) {
                    return Err(rechazo(rechazos::TABLON_ELECCION_INVALIDA, format!("{id} está dos veces")));
                }
                if !catalogo.buscar(id).is_some_and(|m| m.ambito == ambito && m.duracion == duracion) {
                    return Err(rechazo(rechazos::TABLON_ELECCION_INVALIDA, format!("{id} no es una de las {que} del catálogo para {}", id_de_dueno(dueno))));
                }
            }
        }
        if tablon.elegidas_diarias.as_deref() == Some(diarias) && tablon.elegidas_semanales.as_deref() == Some(semanales) {
            return Ok(false);
        }
        let tablon = self.estado.recurrentes.tablones.get_mut(&dueno).expect("mirado");
        tablon.elegidas_diarias = Some(diarias.to_vec());
        tablon.elegidas_semanales = Some(semanales.to_vec());
        self.tocar_dueno(dueno);
        Ok(true)
    }

    /// `fijar_catalogo_misiones` (§6): el catálogo entero, validado, vale desde la siguiente renovación. `Ok(false)`: es el que ya estaba.
    pub(super) fn fijar_catalogo_misiones(&mut self, texto: &str) -> Result<bool, Respuesta> {
        let v = self.estado.version;
        let unicas: Vec<&str> = self.estado.reglas.economia.misiones.misiones.iter().map(|m| m.id.as_str()).collect();
        let nuevo = Catalogo::desde_texto(texto, &unicas).map_err(|e| Respuesta::rechazo(rechazos::MISION_CATALOGO_INVALIDO, e, v))?;
        if *self.estado.recurrentes.catalogo == nuevo {
            return Ok(false);
        }
        self.estado.recurrentes.catalogo = Arc::new(nuevo);
        // El catálogo de su ámbito viaja con cada tablón (para elegir).
        let duenos: Vec<Dueno> = self.estado.recurrentes.tablones.keys().copied().collect();
        for d in duenos {
            self.tocar_dueno(d);
        }
        Ok(true)
    }

    // ───────────────────────────── Reclamar ─────────────────────────────

    /// ¿Es `mision` una recurrente que está (vigente o vencida) en el tablón de `dueno`? Entonces `reclamarRecompensa` es de [`Nucleo::reclamar_recurrente`].
    pub(super) fn es_del_tablon(&self, dueno: Dueno, mision: &str) -> bool {
        self.estado.recurrentes.tablones.get(&dueno).is_some_and(|t| t.misiones.iter().any(|m| m.def.id == mision))
    }

    /// `reclamarRecompensa` de una recurrente (§5): la individual, el héroe que la cumplió (participación, oro y la `xp` a la Facción); la grupal, el Gobernador (o un residente) o el Rey
    /// (la `xp` a la Facción y la participación a partes iguales entre los que aportaron). Se reclama la más antigua cumplida; la vencida sin nada más por reclamar se va del tablón.
    pub(super) fn reclamar_recurrente(&mut self, t: Instante, p: &Peticion, heroe: u64, dueno: Dueno, mision: &str) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let tablon = self.tablon_o_rechazo(dueno)?;
        let de_la_mision = || tablon.misiones.iter().enumerate().filter(|(_, m)| m.def.id == mision);
        let individual = de_la_mision().next().is_some_and(|(_, m)| m.def.es_individual());
        let situacion_de = |m: &EnTablon| match &m.progreso {
            Progreso::Individual(a) => a.get(&heroe).map(|x| x.situacion),
            Progreso::Grupal { situacion, .. } => Some(*situacion),
        };
        let Some(i) = de_la_mision().find(|(_, m)| situacion_de(m) == Some(Situacion::Cumplida)).map(|(i, _)| i) else {
            return Err(if de_la_mision().any(|(_, m)| situacion_de(m) == Some(Situacion::Reclamada)) {
                rechazo(rechazos::MISION_YA_RECLAMADA, format!("{mision} ya se reclamó"))
            } else {
                rechazo(rechazos::MISION_NO_CUMPLIDA, format!("{mision} aún no se ha cumplido"))
            });
        };
        if !individual && !p.admin && !self.puede_reclamar(heroe, dueno) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, format!("no puedes reclamar las misiones de {}", id_de_dueno(dueno))));
        }
        let def = tablon.misiones[i].def.clone();
        let faccion = match dueno {
            Dueno::Asentamiento(a) => self.estado.asentamientos.get(&a).map(|x| x.faccion),
            Dueno::Faccion(f) => Some(f),
            Dueno::Heroe(_) => None,
        };
        let mut datos = Vec::new();
        if individual {
            if def.oro > 0 {
                let hn = self.heroe(heroe).expect("aceptó");
                if hn.heroe.residencia.is_none() {
                    return Err(rechazo(rechazos::HEROE_SIN_RESIDENCIA, "sin residencia no hay almacén personal donde dejar el oro".into()));
                }
                let lleva: Mil = hn.almacen_personal.values().sum();
                if lleva + def.oro > self.estado.reglas.economia.comercio.mercenarios.capacidad_almacen_personal {
                    return Err(rechazo(rechazos::ALMACEN_PERSONAL_LLENO, "el oro no cabe en tu almacén personal".into()));
                }
                *self.heroe_mut(heroe).almacen_personal.entry(Recurso::Oro).or_insert(0) += def.oro;
                self.tr.tocar(Clase::Heroe, heroe);
                datos.push(("oro", def.oro));
            }
            if let (Dueno::Asentamiento(a), true) = (dueno, def.participacion > 0) {
                self.sumar_participacion(a, &[heroe], def.participacion);
                datos.push(("participacion", def.participacion as i64));
            }
        } else if let (Dueno::Asentamiento(a), true) = (dueno, def.participacion > 0) {
            let aportaron: Vec<u64> = match &self.estado.recurrentes.tablones[&dueno].misiones[i].progreso {
                Progreso::Grupal { aportes, .. } => aportes.iter().filter(|(_, &n)| n > 0).map(|(&h, _)| h).collect(),
                Progreso::Individual(_) => unreachable!("grupal"),
            };
            let parte = def.participacion / (aportaron.len() as u64).max(1);
            self.sumar_participacion(a, &aportaron, parte);
            datos.push(("participacion", parte as i64));
        }
        if let (Some(f), true) = (faccion, def.xp > 0) {
            self.dar_experiencia(t, f, def.xp, super::experiencia::motivo::MISION);
            datos.push(("xp", def.xp));
        }
        let tablon = self.estado.recurrentes.tablones.get_mut(&dueno).expect("mirado");
        match &mut tablon.misiones[i].progreso {
            Progreso::Individual(a) => a.get_mut(&heroe).expect("cumplida").situacion = Situacion::Reclamada,
            Progreso::Grupal { situacion, .. } => *situacion = Situacion::Reclamada,
        }
        tablon.misiones.retain(|m| m.hasta > t || m.por_reclamar());
        self.tocar_dueno(dueno);
        let valores = vec![("mision", Dato::Texto(mision.to_owned())), ("dueno", Dato::Texto(id_de_dueno(dueno)))];
        let evento = EventoNucleo { valores, heroes: vec![heroe], ..EventoNucleo::nuevo("mision.reclamada", t, datos) };
        self.tr.emitir(&self.estado, self.hecho_de_tablon(dueno, evento));
        Ok(())
    }
}
