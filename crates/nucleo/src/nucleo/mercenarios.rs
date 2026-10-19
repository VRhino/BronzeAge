//! El mercado de los campamentos de mercenarios dentro del núcleo (Comercio_Caravanas §9): comprar, repostar gratis y la reposición cada
//! `cadaHoras` con lo comerciado entre Facciones. Las cuentas son del crate `comercio::mercenarios`; aquí están el estado, la puerta del
//! campamento, el reloj de la reposición y el traslado a eventos.
//!
//! # La reposición sin coste por campamento ocioso
//! No hay evento recurrente mientras nada cambie: el mercado nace lleno y solo se programa la reposición (a la fase de siempre,
//! `creación + k × cadaHoras`) cuando algo la hace falta: se comerció entre Facciones (contadores > 0) o una compra dejó una pila por debajo
//! de su tope. Tras reponer, si ya no hay nada que reponer, el evento no se vuelve a programar. El stock de un campamento que nadie ha tocado
//! no se guarda (es el inicial).
//!
//! # Lo provisional
//! La columna hace de héroe (hasta el sistema de héroes): su Líder es su propietario, reside en un campamento si la administración lo fijó
//! (`FijarResidencia`) y su almacén personal (`DarAlmacenPersonal`) es lo que paga y recibe una compra de residente.

use super::*;
use crate::estado::SucesoComercio;
use crate::estado_comercio::entidad_mercenarios;
use crate::nucleo::mercado::RADIO_PUERTA;
use asentamiento::Recurso;
use comercio::evento::Evento as EventoComercio;
use comercio::mercenarios::{comprar_residente, comprar_visitante, precio_unitario, ErrorMercenarios, StockCampamento};
use comercio::precio::{precio_referencia, stock_global};
use comercio::{de, total, Carga as Mercancia};

fn rechazo_de_mercenarios(e: ErrorMercenarios) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorMercenarios::CantidadInvalida => (MERCENARIOS_CANTIDAD_INVALIDA, "se compran unidades enteras (al menos una) y el oro no se vende"),
        ErrorMercenarios::SinPrecio => (MERCENARIOS_SIN_PRECIO, "ese bien no está en venta"),
        ErrorMercenarios::TrigoNoVaAlCarro => (MERCENARIOS_TRIGO_NO_VA_AL_CARRO, "el trigo no se compra para el carro: se reposta gratis a víveres"),
        ErrorMercenarios::SinStock => (MERCENARIOS_SIN_STOCK, "el campamento no tiene existencias de ese bien"),
        ErrorMercenarios::CupoAgotado => (MERCENARIOS_CUPO_AGOTADO, "se agotó el cupo diario de ese bien"),
        ErrorMercenarios::SinOro => (MERCENARIOS_SIN_ORO, "no hay oro bastante para una unidad"),
        ErrorMercenarios::SinSitio => (MERCENARIOS_SIN_SITIO, "no hay sitio para la compra"),
        ErrorMercenarios::VivereLlenos => (MERCENARIOS_VIVERES_LLENOS, "los víveres ya están llenos"),
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// El stock en venta de un campamento del artefacto (el inicial si nadie lo ha tocado). `None` si no existe.
    pub fn stock_de_campamento(&self, campamento: u32) -> Option<StockCampamento> {
        let n = self.geografia.as_ref()?.mapa.campamentos.len();
        ((campamento as usize) < n).then(|| self.estado.comercio.mercenarios.stock.get(&campamento).cloned().unwrap_or_else(|| StockCampamento::nuevo(&self.estado.reglas.economia.comercio)))
    }

    /// El estado del mercado de mercenarios (contadores, cupos, residencias y almacenes personales).
    pub fn mercenarios(&self) -> &crate::estado_comercio::EstadoMercenarios {
        &self.estado.comercio.mercenarios
    }

    /// Dónde está la puerta del campamento `i` del artefacto.
    fn puerta_de_campamento(&self, campamento: u32) -> Option<Punto> {
        self.geografia.as_ref()?.mapa.campamentos.get(campamento as usize).copied()
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// `comprarEnCampamento`, `repostarEnCampamento` y el andamiaje de residencia. `Ok(Some(n))` = cambió algo y creó `n`; `Ok(None)` = cambió algo;
    /// los rechazos y «sin cambios» salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_mercenarios(&mut self, t: Instante, p: &Peticion) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let sin_cambios = || Respuesta { resultado: Resultado::SinCambios, version_tras: v, creadas: vec![] };
        let de_mercenarios = |e: ErrorMercenarios| {
            let (c, m) = rechazo_de_mercenarios(e);
            rechazo(c, m.into())
        };
        let cr = self.estado.reglas.economia.comercio.clone();
        // La columna: su propietario (el Líder, provisional) o la administración.
        let columna = |s: &Self, ejercito: u64| -> Result<(), Respuesta> {
            let e = s.estado.ejercitos.get(&ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{ejercito}")))?;
            if p.admin || e.propietario == p.actor {
                Ok(())
            } else {
                Err(rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya".into()))
            }
        };
        match &p.comando {
            Comando::ComprarEnCampamento { ejercito, campamento, recurso, cantidad } => {
                columna(self, *ejercito)?;
                let puerta = self.puerta_de_campamento(*campamento).ok_or_else(|| rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, format!("campamento-{campamento}")))?;
                if self.estado.ejercitos[ejercito].posicion_en(t).dist(puerta) > RADIO_PUERTA {
                    return Err(rechazo(rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA, "la columna no está en la puerta del campamento".into()));
                }
                let referencia = precio_referencia(&cr.precio, *recurso, stock_global(*recurso, self.estado.asentamientos.values().map(|a| &a.economia.almacen)));
                let precio = precio_unitario(&cr.mercenarios, referencia);
                let stock = self.stock_de_campamento(*campamento).expect("el campamento existe").de(*recurso);
                let residente = self.estado.comercio.mercenarios.residencias.get(ejercito) == Some(campamento);
                let compra = if residente {
                    let m = &self.estado.comercio.mercenarios;
                    let personal = m.personal.get(ejercito).cloned().unwrap_or_default();
                    let cupo = m.cupos.get(ejercito).cloned().unwrap_or_default().restante(&cr.mercenarios, *recurso, t);
                    comprar_residente(*recurso, precio, *cantidad, stock, cupo, de(&personal, Recurso::Oro), cr.mercenarios.capacidad_almacen_personal - total(&personal)).map_err(de_mercenarios)?
                } else {
                    let carro = &self.estado.ejercitos[ejercito].suministro;
                    let libre = self.capacidad_de_carga(*ejercito).unwrap_or(0) - total(carro);
                    comprar_visitante(*recurso, precio, *cantidad, stock, de(carro, Recurso::Oro), libre).map_err(de_mercenarios)?
                };
                // El oro cobrado se destruye (sumidero); la compra va al almacén personal (residente) o al carro (visitante).
                let destino = if residente { None } else { Some(*ejercito) };
                let carga = match destino {
                    Some(e) => &mut self.estado.ejercitos.get_mut(&e).expect("existe").suministro,
                    None => self.estado.comercio.mercenarios.personal.entry(*ejercito).or_default(),
                };
                *carga.entry(Recurso::Oro).or_insert(0) -= compra.oro_cobrado;
                *carga.entry(*recurso).or_insert(0) += compra.cantidad;
                carga.retain(|_, n| *n > 0);
                let m = &mut self.estado.comercio.mercenarios;
                if residente && cr.mercenarios.cupo_diario.contains_key(recurso) {
                    m.cupos.entry(*ejercito).or_default().anotar(*recurso, compra.cantidad, t);
                }
                let mut s = m.stock.get(campamento).cloned().unwrap_or_else(|| StockCampamento::nuevo(&cr));
                *s.stock.entry(*recurso).or_insert(0) -= compra.cantidad;
                m.stock.insert(*campamento, s);
                self.tr.tocados.push(*ejercito);
                let ev = EventoComercio::MercenariosComprado { campamento: *campamento, heroe: *ejercito, recurso: *recurso, cantidad: compra.cantidad };
                self.emitir_comercio(t, ev, vec![], &[("oro", compra.oro_cobrado), ("residente", residente as i64)]);
                self.armar_reposicion(t);
                Ok(None)
            }
            Comando::RepostarEnCampamento { ejercito, campamento } => {
                columna(self, *ejercito)?;
                let puerta = self.puerta_de_campamento(*campamento).ok_or_else(|| rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, format!("campamento-{campamento}")))?;
                let e = &self.estado.ejercitos[ejercito];
                if e.posicion_en(t).dist(puerta) > RADIO_PUERTA {
                    return Err(rechazo(rechazos::CAMPAMENTO_FUERA_DE_LA_PUERTA, "la columna no está en la puerta del campamento".into()));
                }
                // DUDA: el tope es la capacidad de víveres de la columna (la suma de sus héroes), no `capacidadViveresPorHeroe` suelta.
                let falta = e.capacidad_viveres - e.viveres.valor_en(t);
                if falta <= 0 {
                    return Err(de_mercenarios(ErrorMercenarios::VivereLlenos));
                }
                let e = self.estado.ejercitos.get_mut(ejercito).expect("existe");
                e.viveres = Ritmo { valor_en_t0: e.capacidad_viveres, t0: t, ..e.viveres };
                self.reprogramar_columna(*ejercito, t);
                self.evento("mercenarios.repostado", t, *ejercito, None, vec![("campamento", *campamento as i64), ("heroe", *ejercito as i64), ("cantidad", falta)]);
                Ok(None)
            }
            Comando::FijarResidencia { ejercito, campamento } => {
                if !p.admin {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, "solo administración".into()));
                }
                columna(self, *ejercito)?;
                if let Some(c) = campamento {
                    self.puerta_de_campamento(*c).ok_or_else(|| rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, format!("campamento-{c}")))?;
                }
                let m = &mut self.estado.comercio.mercenarios;
                if m.residencias.get(ejercito).copied() == *campamento {
                    return Err(sin_cambios());
                }
                match campamento {
                    Some(c) => m.residencias.insert(*ejercito, *c),
                    None => m.residencias.remove(ejercito),
                };
                Ok(None)
            }
            Comando::DarAlmacenPersonal { ejercito, recurso, cantidad } => {
                if !p.admin {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, "solo administración".into()));
                }
                columna(self, *ejercito)?;
                if *cantidad <= 0 {
                    return Err(rechazo(rechazos::CANTIDAD_NO_POSITIVA, "la cantidad debe ser mayor que cero".into()));
                }
                let m = &mut self.estado.comercio.mercenarios;
                if !m.residencias.contains_key(ejercito) {
                    return Err(rechazo(rechazos::CAMPAMENTO_DESCONOCIDO, "la columna no reside en ningún campamento".into()));
                }
                let personal = m.personal.entry(*ejercito).or_default();
                let n = (*cantidad).min((cr.mercenarios.capacidad_almacen_personal - total(personal)).max(0));
                if n == 0 {
                    return Err(sin_cambios());
                }
                *personal.entry(*recurso).or_insert(0) += n;
                Ok(None)
            }
            _ => unreachable!("solo se llama con órdenes de los campamentos"),
        }
    }

    /// Cambió algo de la columna que mueve sus eventos (los víveres): sube su generación y reprograma llegada, umbrales y encuentros.
    pub(super) fn reprogramar_columna(&mut self, id: u64, t: Instante) {
        let g = self.estado.generaciones.subir(id);
        let e = self.estado.ejercitos.get_mut(&id).expect("existe");
        let marchando = e.marchando();
        if marchando {
            e.plan.generacion = g;
        }
        self.tr.tocados.push(id);
        self.programar_ejercito(id, t);
        if marchando {
            self.avisar_plan(id, true);
        }
    }

    // ───────────────────────────── Reposición ─────────────────────────────

    /// Anota lo comerciado entre Facciones en los contadores de reposición (§9.3) y, si cuenta, arma la reposición.
    pub(super) fn registrar_comercio_de_mercenarios(&mut self, entre_facciones: bool, carga: &Mercancia) {
        self.estado.comercio.mercenarios.mercado.registrar_comercio(entre_facciones, carga);
        if entre_facciones && !carga.is_empty() {
            let t = self.estado.ahora;
            self.armar_reposicion(t);
        }
    }

    /// ¿Hay algo que reponer? Contadores de comercio sin aplicar, o una pila por debajo de su tope.
    fn hay_que_reponer(&self) -> bool {
        let m = &self.estado.comercio.mercenarios;
        let pilas = &self.estado.reglas.economia.comercio.mercenarios.pilas;
        !m.mercado.contadores.is_empty() || m.stock.values().any(|s| pilas.iter().any(|(&b, &tope)| s.de(b) < tope))
    }

    /// Programa la reposición en la próxima fase de `cadaHoras` (la de siempre: `creación + k × cada`) si hace falta y no hay ya una programada.
    pub(super) fn armar_reposicion(&mut self, t: Instante) {
        if self.estado.comercio.mercenarios.pulso.is_some() || !self.hay_que_reponer() {
            return;
        }
        let cada = self.estado.reglas.economia.comercio.mercenarios.cada;
        let m = &mut self.estado.comercio.mercenarios;
        // Las citas en las que no había nada que reponer no hicieron nada: se salta a la primera fase que no ha pasado.
        if m.mercado.repone_en < t {
            m.mercado.repone_en += (t - m.mercado.repone_en + cada - 1) / cada * cada;
        }
        let due = m.mercado.repone_en;
        m.pulso = Some(due);
        self.programar_comercio(due, entidad_mercenarios(), SucesoComercio::ReponeMercenarios);
    }

    /// Vence la reposición (`reponerMercados`, §9.3).
    pub(super) fn repone_mercenarios(&mut self, t: Instante) {
        self.estado.comercio.mercenarios.pulso = None;
        let cr = self.estado.reglas.economia.comercio.clone();
        let n = self.geografia.as_ref().map_or(0, |g| g.mapa.campamentos.len() as u32);
        let m = &mut self.estado.comercio.mercenarios;
        let mut campamentos: Vec<StockCampamento> = (0..n).map(|i| m.stock.get(&i).cloned().unwrap_or_else(|| StockCampamento::nuevo(&cr))).collect();
        if m.mercado.reponer(&cr.mercenarios, t, &mut campamentos) {
            let inicial = StockCampamento::nuevo(&cr);
            for (i, s) in campamentos.into_iter().enumerate() {
                if s == inicial {
                    m.stock.remove(&(i as u32));
                } else {
                    m.stock.insert(i as u32, s);
                }
            }
            self.emitir_propio(t, "mercenarios.repuesto", vec![], vec![("campamentos", n as i64)]);
        }
        self.armar_reposicion(t);
    }
}
