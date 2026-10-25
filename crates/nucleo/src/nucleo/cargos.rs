//! Los cargos locales, la matriz de autorización de las órdenes de plaza y la puerta (sistema 5, tanda 5E; Diplomacia_Facciones §3.4 y §4.3, Proyecciones §4.3 y §5.2, Doc 1.10.5 y 2.2).
//! Las reglas puras son el crate `politica` (`cargos`, `puerta`, `autorizacion`); aquí están el estado de cada plaza (`Asentamiento::{cargos, puerta}`), las tres órdenes y lo que arrastran.
//!
//! # La matriz
//! Quién da cada orden lo declara [`Comando::autoriza`]; [`Nucleo::autorizar_en_plaza`] lo aplica sobre la plaza de la orden con los predicados de `politica::autorizacion::Actor`.
//! **«La ciudadanía habilita, la presencia ejerce»** (Doc 2.5): el cargo exige ser su titular, residir en la plaza y estar dentro; el Rey designa al Gobernador y fija la puerta sin estar
//! presente. Sin héroe nombrado en la orden (las de plaza no lo nombran), vale cualquiera de los héroes de la cuenta, que se buscan por el índice cuenta → héroes (sin recorrer el mundo): la
//! cuenta de un jugador tiene uno; la de un escenario de la administración, varios héroes de campo. Un héroe fuera del mundo no cuenta.
//!
//! # Los cargos
//! Al Gobernador lo designa el Rey de la Facción dueña; a los otros cuatro, el Gobernador. El designado es ciudadano de la Facción dueña y **reside en la plaza** (Q4); el cargo se vacía al
//! dejar de residir (cualquier mudanza pasa por `mudar_de_residencia`), al perder la ciudadanía de la Facción dueña y en la conquista. Quien funda la plaza (el Líder de la columna, si es uno de
//! los fundadores) es su Gobernador de oficio (5E-3, `zonas::fundar`); el Rey puede cambiarlo.
//!
//! # La puerta (Q1)
//! Por grupos (neutrales, aliados —con vasallos y señores—, enemigos en guerra y los Aedas) y vetos; por defecto, cerrada a neutrales y enemigos (`PUERTA.cerradaAPorDefecto`). Las capas:
//! el residente entra siempre, el vetado no entra, la propia Facción entra siempre y los demás si su grupo no está cerrado. Cerrar o vetar saca al mapa, con todo lo suyo, a quien ya está
//! dentro de la plaza, de sus pueblos o de sus avanzadas (que la heredan) y ya no puede entrar. La conquista la devuelve a la de por defecto. El alias `FijarAcceso` (117) se retiró en 5H (5E-12).

use super::*;
use crate::estado::Dato;
use militar::heroe::Residencia;
use politica::autorizacion::Actor;
use politica::cargos::ErrorCargo;
use politica::puerta::{ErrorPuerta, Visitante};
use politica::Grupos;

impl Nucleo {
    // ───────────────────────────── La matriz ─────────────────────────────

    /// ¿Cumple el héroe `yo` lo que pide la orden sobre la plaza `a`? Solo para las clases de [`Autoriza`] que miran una plaza.
    fn cumple(&self, comando: &Comando, yo: u64, a: u64) -> bool {
        let x = Actor { yo, politica: &self.estado.politica, vista: self };
        let a = a as u32;
        match comando.autoriza() {
            Autoriza::Cargo(cargos) => x.algun_cargo(a, cargos),
            Autoriza::CargoDeLaOrden => match comando {
                Comando::ActivarPolitica { cargo, .. } | Comando::ComprometerRecinto { cargo, .. } | Comando::MejorarRecinto { cargo, .. } => x.cargo(a, *cargo),
                otro => unreachable!("{otro:?} no nombra cargo"),
            },
            Autoriza::ResidenteDentro => x.reside(a),
            Autoriza::CargoEnElSitio(cargos) => x.cargo_en_el_sitio(a, cargos),
            Autoriza::AsignaCargoLocal => match comando {
                Comando::AsignarCargoLocal { cargo, .. } => x.asigna_cargo_local(a, *cargo),
                otro => unreachable!("{otro:?} no designa cargos"),
            },
            Autoriza::FijaPuerta => x.fija_puerta(a),
            Autoriza::CompraIntel => x.compra_intel_en(a),
            Autoriza::DirigeAedas => x.dirige_aedas(a),
            Autoriza::CompraTecnologiaAeda => x.compra_tecnologia_aeda(a),
            otra => unreachable!("{otra:?} no es una autorización de plaza"),
        }
    }

    /// Aplica la matriz ([`Comando::autoriza`]) a una orden sobre la plaza `a` (que existe): con el héroe que nombra la orden (que ha de ser de su cuenta y estar en el mundo), o con
    /// cualquiera de los de la cuenta si no nombra ninguno. La administración puede con todas.
    pub(super) fn autorizar_en_plaza(&self, p: &Peticion, a: u64, nombrado: Option<u64>) -> Result<(), Respuesta> {
        if p.admin {
            return Ok(());
        }
        let puede = match nombrado {
            Some(h) => {
                self.autorizar_heroe(p, h, false)?;
                self.cumple(&p.comando, h, a)
            }
            None => self.heroes_en_el_mundo(&p.actor).any(|h| self.cumple(&p.comando, h, a)),
        };
        if puede {
            return Ok(());
        }
        Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, format!("en as-{a} no tienes lo que pide la orden: {}", quien_la_da(p.comando.autoriza())), self.estado.version))
    }

    /// `puedeOperarAparcada`: un héroe de la cuenta reside en el `origen` de la caravana y está dentro de la plaza `aparcada` donde está aparcada.
    pub(super) fn autorizar_aparcada(&self, p: &Peticion, origen: u64, aparcada: u64) -> Result<(), Respuesta> {
        if p.admin || self.heroes_en_el_mundo(&p.actor).any(|yo| (Actor { yo, politica: &self.estado.politica, vista: self }).opera_aparcada(origen as u32, aparcada as u32)) {
            return Ok(());
        }
        Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, format!("hace falta residir en as-{origen} y estar dentro de as-{aparcada}, donde está aparcada"), self.estado.version))
    }

    /// Los héroes de la cuenta que están en el mundo, por el índice cuenta → héroes.
    fn heroes_en_el_mundo<'a>(&'a self, cuenta: &str) -> impl Iterator<Item = u64> + 'a {
        let ids = self.estado.militar.heroes_de_cuenta(cuenta);
        ids.iter().copied().filter(|&h| self.heroe(h).is_some_and(|x| !x.heroe.presencia.fuera))
    }

    // ───────────────────────────── Los cargos ─────────────────────────────

    /// Vacía los cargos que `heroe` tiene en la plaza `a` (deja de residir en ella o de ser de su Facción). Sin hecho propio: lo cuenta el de la mudanza.
    pub(super) fn vaciar_cargos_de(&mut self, heroe: u64, a: u64) {
        if let Some(plaza) = self.estado.asentamientos.get_mut(&a) {
            if !plaza.cargos.vaciar_de(heroe).is_empty() {
                self.tr.tocar(Clase::Asentamiento, a);
            }
        }
    }

    // ───────────────────────────── La puerta ─────────────────────────────

    /// ¿Le abre la plaza su puerta a `heroe`? Por capas (`politica::puerta`): residente (si `contar_residencia`), vetado, propia Facción y grupo. `false` si el héroe o la plaza no existen.
    pub(super) fn puerta_deja_pasar(&self, heroe: u64, plaza: u64, contar_residencia: bool) -> bool {
        let (Some(hn), Some(a)) = (self.heroe(heroe), self.estado.asentamientos.get(&plaza)) else { return false };
        let residente = contar_residencia && hn.heroe.residencia == Some(Residencia::Asentamiento(plaza as u32));
        let v = Visitante { heroe, faccion: hn.faccion, residente };
        a.puerta.puede_entrar(&self.estado.reglas.economia.politica, &self.estado.politica.relaciones, a.faccion, v)
    }

    /// Los grupos a los que está cerrada la puerta de la plaza ahora (los de por defecto si nadie la ha fijado).
    pub fn puerta_cerrada_a(&self, plaza: u64) -> Option<Grupos> {
        self.estado.asentamientos.get(&plaza).map(|a| a.puerta.cerrada(&self.estado.reglas.economia.politica))
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// `asignarCargoLocal`, `fijarPuerta` y `vetarJugador`. Rechazos y «sin cambios» salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_cargos(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        match &p.comando {
            Comando::AsignarCargoLocal { heroe, asentamiento, cargo, designado } => {
                let faccion = self.plaza_o_rechazo(*asentamiento)?.faccion;
                self.autorizar_en_plaza(p, *asentamiento, Some(*heroe))?;
                let reside = self.heroe(*designado).ok_or_else(|| rechazo(rechazos::HEROE_DESCONOCIDO, ids::heroe(*designado)))?.heroe.residencia == Some(Residencia::Asentamiento(*asentamiento as u32));
                // Una Facción que no existe como entidad (la de un escenario de la administración) no tiene ciudadanos.
                let no_ciudadano = || rechazo(rechazos::CARGO_NO_CIUDADANO, format!("{} no es ciudadano de f-{faccion}", ids::heroe(*designado)));
                let duena = self.estado.politica.facciones.get(faccion).ok_or_else(no_ciudadano)?.clone();
                let plaza = self.estado.asentamientos.get_mut(asentamiento).expect("existe");
                if plaza.cargos.es_titular(*cargo, *designado) {
                    return Err(Respuesta::sin_cambios(v));
                }
                politica::cargos::asignar_cargo_local(&mut plaza.cargos, faccion, &duena, *cargo, *designado, reside).map_err(|e| match e {
                    ErrorCargo::NoCiudadano => no_ciudadano(),
                    ErrorCargo::NoReside => rechazo(rechazos::CARGO_NO_RESIDE, format!("{} no reside en as-{asentamiento}: solo se designa a quien reside", ids::heroe(*designado))),
                    ErrorCargo::SinGobernador => rechazo(rechazos::CARGO_SIN_GOBERNADOR, "la plaza no tiene Gobernador: primero lo nombra el Rey".into()),
                    ErrorCargo::PlazaAjena => unreachable!("la Facción es la dueña de la plaza"),
                })?;
                self.tr.tocar(Clase::Asentamiento, *asentamiento);
                let valores = vec![("cargo", Dato::Texto(cargo.clave().to_owned()))];
                let datos = vec![("asentamiento", *asentamiento as i64), ("faccion", i64::from(faccion)), ("heroe", *designado as i64)];
                self.tr.emitir(&self.estado, EventoNucleo { valores, asentamientos: vec![*asentamiento], ..EventoNucleo::nuevo("cargo.local_asignado", t, datos) });
                Ok(vec![])
            }
            Comando::FijarPuerta { heroe, asentamiento, cerrada_a } => {
                self.plaza_o_rechazo(*asentamiento)?;
                self.autorizar_en_plaza(p, *asentamiento, Some(*heroe))?;
                self.fijar_puerta(t, p, *asentamiento, *cerrada_a)
            }
            Comando::VetarJugador { heroe, asentamiento, vetado, vetar } => {
                self.plaza_o_rechazo(*asentamiento)?;
                self.autorizar_en_plaza(p, *asentamiento, Some(*heroe))?;
                let residente = self.heroe(*vetado).ok_or_else(|| rechazo(rechazos::HEROE_DESCONOCIDO, ids::heroe(*vetado)))?.heroe.residencia == Some(Residencia::Asentamiento(*asentamiento as u32));
                if *vetar && !p.admin && self.sitios_en_batalla(*asentamiento) {
                    return Err(rechazo(rechazos::BATALLA_BLOQUEADO, "un pueblo o una avanzada de la plaza está en una batalla: la puerta no se toca hasta que acabe".into()));
                }
                let plaza = self.estado.asentamientos.get_mut(asentamiento).expect("existe");
                match plaza.puerta.vetar(*vetado, *vetar, residente) {
                    Err(ErrorPuerta::Residente) => return Err(rechazo(rechazos::PUERTA_RESIDENTE, format!("{} reside en as-{asentamiento}: a un residente no se le veta", ids::heroe(*vetado)))),
                    Ok(false) => return Err(Respuesta::sin_cambios(v)),
                    Ok(true) => {}
                }
                self.tr.tocar(Clase::Asentamiento, *asentamiento);
                if *vetar {
                    self.cerrar_la_puerta(t, *asentamiento);
                }
                self.emitir_propio(t, "asentamiento.veto", vec![*asentamiento], vec![("heroe", *vetado as i64), ("vetado", *vetar as i64)]);
                Ok(vec![])
            }
            _ => unreachable!("solo se llama con órdenes de cargos y de la puerta"),
        }
    }

    /// Sustituye los grupos a los que está cerrada la puerta y saca al mapa a quien ya no puede estar dentro.
    fn fijar_puerta(&mut self, t: Instante, p: &Peticion, plaza: u64, cerrada_a: Grupos) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        if self.puerta_cerrada_a(plaza) == Some(cerrada_a) {
            return Err(Respuesta::sin_cambios(v));
        }
        // Un pueblo o una avanzada de la plaza en batalla no recibe órdenes, y echar a quien combate dentro sería una (la administración no lleva candado).
        if !p.admin && self.sitios_en_batalla(plaza) {
            return Err(Respuesta::rechazo(rechazos::BATALLA_BLOQUEADO, "un pueblo o una avanzada de la plaza está en una batalla: la puerta no se toca hasta que acabe", v));
        }
        self.estado.asentamientos.get_mut(&plaza).expect("existe").puerta.fijar(cerrada_a);
        self.tr.tocar(Clase::Asentamiento, plaza);
        self.cerrar_la_puerta(t, plaza);
        // Los Aedas (6D, Q5 del autor): el que está dentro y ya no puede entrar sale al instante; si la puerta se les abre, pueden nacer los que esperaban una plaza que los aceptase.
        self.puerta_de_los_aedas(plaza);
        let grupos = Dato::Lista(cerrada_a.iter().map(|g| Dato::Texto(g.nombre().to_owned())).collect());
        let punto = self.estado.asentamientos.get(&plaza).map(|a| a.posicion);
        self.tr.emitir(&self.estado, EventoNucleo { valores: vec![("cerrada_a", grupos)], asentamientos: vec![plaza], punto, ..EventoNucleo::nuevo("asentamiento.puerta_fijada", t, vec![("asentamiento", plaza as i64)]) });
        Ok(vec![])
    }
}

/// Lo que pide la clase de autorización, para el mensaje del rechazo.
fn quien_la_da(a: Autoriza) -> &'static str {
    match a {
        Autoriza::Cargo(_) | Autoriza::CargoDeLaOrden => "ser el titular del cargo, residir en la plaza y estar dentro",
        Autoriza::ResidenteDentro => "residir en la plaza y estar dentro",
        Autoriza::CargoEnElSitio(_) => "ser su Gobernador o su Maestro de Obras, residir en ella y estar con la columna en el sitio",
        Autoriza::AsignaCargoLocal => "al Gobernador lo nombra el Rey; a los demás, el Gobernador, dentro de la plaza",
        Autoriza::FijaPuerta => "ser su Gobernador, dentro de la plaza, o el Rey de su Facción",
        Autoriza::CompraIntel => "estar dentro y ser el Rey, el Embajador o el Gobernador",
        Autoriza::DirigeAedas => "estar dentro y ser el Rey de su Facción, su Gobernador o su Sacerdote",
        Autoriza::CompraTecnologiaAeda => "estar dentro y ser el Rey o el Gobernador",
        _ => "otra cosa",
    }
}
