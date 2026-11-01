//! Ofrecerse en el tablón de un campamento, contratar y prorrogar (Mercenarios_Por_Contrato §1 a §3, §5 y §6.2). Tarea T1 del plan.
//!
//! # Quién firma
//! Firma el titular de un cargo ([`Nucleo::puestos_de`]) en persona: en el campamento del mercenario, dentro o con su columna a la puerta, y pagando con el oro del carro de esa columna.
//! No elige con qué cargo: las reglas puras toman el de más rango que pueda firmar ese trabajo sobre ese objetivo. El pago entra en el almacén personal del mercenario hasta llenarlo y
//! lo que no cabe desaparece: es un sumidero, y nunca bloquea una firma (decisión 9 del plan).

use super::*;
use crate::codec_contratos::trabajo_a_u8;
use crate::estado_militar::Relacion;
use ::mercenarios::contrato::{ambito_de, duracion, objetivo_valido, orden_inicial, puesto_para_firmar, renombre_del_plazo, tarifa, ErrorFirma};
use ::mercenarios::{Contrato, Ficha, Oferta, Puesto, Situacion};
use asentamiento::datos::UNO;
use asentamiento::{Cargo, Recurso};
use base::UNIDAD;
use comercio::{de, total};
use militar::heroe::Residencia;
use militar::mercenarios::precio;
use militar::presencia::Ubicacion;
use politica::Marca;

/// El rechazo de la versión `v` que corresponde a un fallo de las reglas puras de la firma.
fn rechazo_de_firma(e: ErrorFirma, v: u64) -> Respuesta {
    use rechazos::*;
    let (codigo, mensaje) = match e {
        ErrorFirma::ObjetivoInvalido => (MERCENARIO_OBJETIVO_INVALIDO, "el objetivo no es de una clase que admita ese trabajo o no vale para él"),
        ErrorFirma::PlazoInvalido => (MERCENARIO_PLAZO_INVALIDO, "el temporal lleva uno de los plazos de la tabla y el permanente, ninguno"),
        ErrorFirma::SinAlcance => (MERCENARIO_SIN_ALCANCE, "ninguno de tus cargos puede firmar ese trabajo sobre ese objetivo"),
    };
    Respuesta::rechazo(codigo, mensaje, v)
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Lo que las reglas puras necesitan saber de `o` para la Facción `faccion`, que firma u ordena (§3, §6.5). `None` si el objetivo no existe. Los bandidos solo dicen en
    /// cuáles de las plazas de esa Facción caen en su zona: son las únicas que importan a quien manda.
    pub(super) fn situacion(&self, faccion: u32, o: &Objetivo) -> Option<Situacion> {
        let en_guerra = |duena: u32| self.estado.relacion(faccion, duena) == Some(Relacion::Guerra);
        let de = |duena: u32, plaza: u64| Situacion { duena: Some(duena), plaza: Some(plaza), en_guerra: en_guerra(duena), ..Situacion::default() };
        let de_la_plaza = |plaza: u64| self.estado.asentamientos.get(&plaza).map(|a| de(a.faccion, plaza));
        match *o {
            Objetivo::Plaza(id) => de_la_plaza(id),
            Objetivo::Pueblo(id) => de_la_plaza(self.pueblo(id)?.0),
            // Unas ruinas de ciudadela sin dueño existen, pero no son de nadie.
            Objetivo::Avanzada(id) => Some(self.avanzada(id)?.0.and_then(de_la_plaza).unwrap_or_default()),
            Objetivo::Caravana(id) => de_la_plaza(u64::from(self.caravana(id)?.origen)),
            Objetivo::Bandidos(id) => {
                let donde = self.campamento_de_bandidos(id)?.posicion;
                Some(Situacion { en_zona_de: self.plazas_de(faccion).filter(|a| self.en_zona(a.id, donde)).map(|a| a.id).collect(), ..Situacion::default() })
            }
            Objetivo::Ejercito(id) => {
                let e = self.ejercito(id).filter(|e| Self::es_ejercito(e))?;
                Some(Situacion { duena: e.columna.faccion, en_guerra: e.columna.faccion.is_some_and(en_guerra), cabeza: Some(e.columna.lider), ..Situacion::default() })
            }
            Objetivo::Convocatoria(id) => {
                let c = self.convocatoria(id)?;
                Some(Situacion { duena: Some(c.faccion), en_guerra: en_guerra(c.faccion), cabeza: Some(c.lider), ..Situacion::default() })
            }
            Objetivo::Faccion(id) => self.faccion(id).map(|_| Situacion { duena: Some(id), ..Situacion::default() }),
            Objetivo::Punto(_) => Some(Situacion::default()),
        }
    }

    /// Los cargos con los que `heroe` firma u ordena (§3, §6.5), de más rango a menos: el trono y la embajada de su Facción y, si reside en una plaza suya, los cargos locales que
    /// ocupa en ella. Un marcado no tiene ninguno.
    pub(super) fn puestos_de(&self, heroe: u64) -> Vec<Puesto> {
        let Some(h) = self.heroe(heroe) else { return Vec::new() };
        let Some(f) = h.faccion.and_then(|f| self.faccion(f)) else { return Vec::new() };
        let mut puestos = Vec::new();
        if f.rey == Some(heroe) {
            puestos.push(Puesto::Rey);
        }
        if f.embajador == Some(heroe) {
            puestos.push(Puesto::Embajador);
        }
        if let Some(Residencia::Asentamiento(a)) = h.heroe.residencia {
            let id = u64::from(a);
            if let Some(plaza) = self.estado.asentamientos.get(&id).filter(|p| p.faccion == f.id) {
                let locales = [(Cargo::General, Puesto::General(id)), (Cargo::Gobernador, Puesto::Gobernador(id)), (Cargo::MaestroCaravanas, Puesto::MaestroCaravanas(id))];
                puestos.extend(locales.into_iter().filter(|&(cargo, _)| plaza.cargos.es_titular(cargo, heroe)).map(|(_, puesto)| puesto));
            }
        }
        puestos
    }

    /// Lo que cobra hoy el tablón por contratar a `mercenario`: `plazo` son los días de un temporal, o `None` el permanente. Su valor es 20 de oro por nivel más lo que costaría
    /// reclutar hoy cada escuadra suya, no prestada, con los soldados que tiene, sin recargo de reputación ni descuento (duda 1 del plan); lo demás lo hace
    /// [`::mercenarios::contrato::tarifa`]. El plazo ha de ser uno de la tabla.
    pub(crate) fn tarifa_de(&self, mercenario: u64, plazo: Option<u8>) -> Mil {
        let e = &self.estado.reglas.economia;
        let h = &self.heroe(mercenario).expect("el mercenario existe").heroe;
        let valor_tropa = h.escuadras.iter().filter(|s| s.prestada.is_none()).map(|s| precio(&e.militar, e.militar.tropas.tropa(s.tropa), s.cantidad, UNO, false)).sum();
        // Sin ficha todavía, el rango es el primero.
        let rango = self.estado.contratos.fichas.get(&mercenario).map_or(1, |f| f.rango);
        tarifa(&e.contratos, plazo, u8::try_from(h.nivel).unwrap_or(u8::MAX), valor_tropa, rango).expect("el plazo es uno de la tabla")
    }

    // ───────────────────────────── Ayudas ─────────────────────────────

    /// Quita la oferta de `heroe` del tablón y se lo cuenta, si la tenía. La retira él, o se retira sola al mudarse de residencia (§3).
    pub(in crate::nucleo) fn quitar_oferta(&mut self, t: Instante, heroe: u64) -> bool {
        let Some(oferta) = self.estado.contratos.ofertas.remove(&heroe) else { return false };
        self.tr.tocar(Clase::Heroe, heroe);
        self.tr.tocar(Clase::Campamento, u64::from(oferta.campamento));
        self.emitir_nombrando(t, "mercenarios.oferta_retirada", vec![], vec![heroe], vec![("heroe", heroe as i64), ("campamento", i64::from(oferta.campamento))]);
        true
    }

    /// Lo que hay que pagar por `mercenario` a `plazo` (la tarifa de hoy) si el carro de la `columna` de quien firma lleva oro bastante; si no, el rechazo `mercenarios.sin_oro` de la versión `v`.
    fn oro_a_cobrar(&self, columna: u64, mercenario: u64, plazo: Option<u8>, v: u64) -> Result<Mil, Respuesta> {
        let oro = self.tarifa_de(mercenario, plazo);
        let hay = de(&self.estado.ejercitos[&columna].columna.carga, Recurso::Oro);
        if hay < oro {
            return Err(Respuesta::rechazo(rechazos::MERCENARIOS_SIN_ORO, format!("la tarifa es de {} de oro y el carro lleva {}", oro / UNIDAD, hay / UNIDAD), v));
        }
        Ok(oro)
    }

    /// Cobra `oro` del carro de la `columna` de quien firma y lo ingresa en el almacén personal de `mercenario` hasta donde quepa; el resto desaparece (decisión 9 del plan).
    fn pagar(&mut self, columna: u64, mercenario: u64, oro: Mil) {
        let carro = &mut self.estado.ejercitos.get_mut(&columna).expect("la columna del carro existe").columna.carga;
        *carro.entry(Recurso::Oro).or_insert(0) -= oro;
        carro.retain(|_, n| *n > 0);
        self.tr.tocar(Clase::Ejercito, columna);
        let capacidad = self.estado.reglas.economia.comercio.mercenarios.capacidad_almacen_personal;
        let almacen = &mut self.heroe_mut(mercenario).almacen_personal;
        let cabe = base::entero((capacidad - total(almacen)).clamp(0, oro));
        *almacen.entry(Recurso::Oro).or_insert(0) += cabe;
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// `ofrecerse_como_mercenario` (§3): un bot sin Facción, residente de un campamento y dentro de él, se pone en su tablón. La primera vez nace su ficha y recibe su leva propia (§8.3): devuelve sus escuadras nuevas.
    pub(super) fn ofrecerse_como_mercenario(&mut self, t: Instante, p: &Peticion, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        use rechazos::*;
        self.autorizar_heroe(p, heroe, false)?;
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let h = self.heroe(heroe).expect("autorizado");
        if !self.es_bot(heroe) {
            return Err(rechazo(MERCENARIO_SOLO_BOTS, "solo un bot se ofrece como mercenario"));
        }
        if h.faccion.is_some() {
            return Err(rechazo(FACCION_YA_PERTENECE, "un mercenario ofrecido no tiene Facción"));
        }
        let campamento = self.campamento_de_residencia(heroe).ok_or_else(|| rechazo(MERCENARIOS_NO_RESIDES, "solo se ofrece quien reside en un campamento"))?;
        if h.heroe.presencia.ubicacion != Ubicacion::Mercenarios(campamento) {
            return Err(rechazo(CAMPAMENTO_FUERA_DE_LA_PUERTA, "hay que estar dentro del campamento donde se reside para ofrecerse en su tablón"));
        }
        if self.estado.contratos.ofertas.contains_key(&heroe) {
            return Err(Respuesta::sin_cambios(v));
        }
        self.estado.contratos.ofertas.insert(heroe, Oferta { campamento, desde: t });
        self.estado.contratos.fichas.entry(heroe).or_insert_with(Ficha::nueva);
        self.tr.tocar(Clase::Heroe, heroe);
        self.tr.tocar(Clase::Campamento, u64::from(campamento));
        self.emitir_nombrando(t, "mercenarios.ofrecido", vec![], vec![heroe], vec![("heroe", heroe as i64), ("campamento", i64::from(campamento))]);
        Ok(self.dar_leva_propia(t, heroe))
    }

    /// `retirar_oferta`: el héroe se quita del tablón.
    pub(super) fn retirar_oferta(&mut self, t: Instante, p: &Peticion, heroe: u64) -> Result<Vec<u64>, Respuesta> {
        self.autorizar_heroe(p, heroe, false)?;
        if self.quitar_oferta(t, heroe) {
            Ok(vec![])
        } else {
            Err(Respuesta::sin_cambios(self.estado.version))
        }
    }

    /// `contratar_mercenario` (§3, §5): un cargo de una Facción humana contrata, en persona y pagando con el oro de su carro, a un bot del tablón de un campamento. El mercenario
    /// entra en la Facción con su marca, con la orden inicial de su trabajo; el permanente pasa a residir en la plaza `residencia`, sin cooldown.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn contratar_mercenario(&mut self, t: Instante, p: &Peticion, heroe: u64, mercenario: u64, trabajo: Trabajo, objetivo: Objetivo, plazo: Option<u8>, residencia: Option<u64>) -> Result<Vec<u64>, Respuesta> {
        use rechazos::*;
        self.autorizar_heroe(p, heroe, false)?;
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let d = self.estado.reglas.economia.contratos.clone();
        // Quién firma y a quién: una Facción humana, un mercenario del tablón sin Facción, en persona y sin espera tras un despido.
        let faccion = self.heroe(heroe).and_then(|h| h.faccion).ok_or_else(|| rechazo(MERCENARIO_SIN_ALCANCE, "para contratar hay que pertenecer a una Facción"))?;
        if self.es_faccion_de_bots(faccion) {
            return Err(rechazo(MERCENARIO_FACCION_DE_BOTS, "las Facciones de bots no contratan"));
        }
        let oferta = *self.estado.contratos.ofertas.get(&mercenario).ok_or_else(|| rechazo(MERCENARIO_NO_OFRECIDO, "ese héroe no está en el tablón de ningún campamento"))?;
        if self.heroe(mercenario).is_some_and(|h| h.faccion.is_some()) {
            return Err(rechazo(FACCION_YA_PERTENECE, "el mercenario ya es ciudadano de una Facción"));
        }
        if !self.esta_en_el_campamento(heroe, oferta.campamento, t) {
            return Err(rechazo(CAMPAMENTO_FUERA_DE_LA_PUERTA, "hay que estar en el campamento del mercenario, dentro o con la columna a su puerta"));
        }
        let columna = self.columna_que_lleva(heroe).ok_or_else(|| rechazo(HEROE_SIN_COLUMNA, "se paga con el oro del carro de tu columna: no vas en ninguna"))?;
        if self.estado.contratos.esperas.get(&(mercenario, faccion)).is_some_and(|&hasta| t < hasta) {
            return Err(rechazo(MERCENARIO_RECONTRATO_EN_ESPERA, "se le despidió hace poco: aún no se le puede recontratar en esta Facción"));
        }
        // El plazo y la residencia casan con el trabajo.
        let renombre = renombre_del_plazo(&d, trabajo, plazo).map_err(|e| rechazo_de_firma(e, v))?;
        let residencia = match (trabajo, residencia) {
            (Trabajo::Permanente, Some(plaza)) if self.estado.asentamientos.get(&plaza).is_some_and(|a| a.faccion == faccion) => Some(plaza),
            (Trabajo::Permanente, _) => return Err(rechazo(MERCENARIO_OBJETIVO_INVALIDO, "el permanente reside en una plaza de la Facción")),
            (_, Some(_)) => return Err(rechazo(MERCENARIO_OBJETIVO_INVALIDO, "solo el permanente reside en una plaza")),
            (_, None) => None,
        };
        // El objetivo existe, vale para ese trabajo, y alguno de sus cargos alcanza a firmarlo.
        let s = self.situacion(faccion, &objetivo).ok_or_else(|| rechazo(MERCENARIO_OBJETIVO_INVALIDO, "el objetivo no existe"))?;
        objetivo_valido(&d, trabajo, &objetivo, faccion, &s).map_err(|e| rechazo_de_firma(e, v))?;
        let puesto = puesto_para_firmar(trabajo, &objetivo, &s, &self.puestos_de(heroe), heroe).map_err(|e| rechazo_de_firma(e, v))?;
        // La tarifa, con el oro del carro.
        let oro = self.oro_a_cobrar(columna, mercenario, plazo, v)?;

        self.pagar(columna, mercenario, oro);
        self.estado.contratos.ofertas.remove(&mercenario);
        self.tr.tocar(Clase::Campamento, u64::from(oferta.campamento));
        let marca = if trabajo == Trabajo::Permanente { Marca::Permanente } else { Marca::Temporal };
        self.estado.politica.facciones.otorgar_marcado(faccion, mercenario, marca).expect("el mercenario no tiene Facción y la del firmante existe");
        self.tras_cambiar_de_faccion(t, mercenario, None);
        if let Some(plaza) = residencia {
            self.mudar_de_residencia(mercenario, Some(Residencia::Asentamiento(plaza as u32)));
        }
        let mut contrato = Contrato {
            mercenario,
            faccion,
            ambito: ambito_de(puesto),
            trabajo,
            objetivo,
            plazo,
            firmado_por: heroe,
            puesto_firma: puesto,
            firmado_en: t,
            vence: plazo.map(|dias| t + duracion(dias)),
            campamento_base: oferta.campamento,
            orden: None,
            fin_aplazado: None,
            renombre,
        };
        contrato.orden = orden_inicial(&d, &contrato, residencia, s.duena, t);
        if let Some(vence) = contrato.vence {
            self.citar_contrato(mercenario, SucesoContrato::Vence, vence);
        }
        if let Some(vence) = contrato.orden.and_then(|o| o.vence) {
            self.citar_contrato(mercenario, SucesoContrato::VenceOrden, vence);
        }
        contrato.orden.iter().map(|o| o.objetivo).chain([objetivo]).for_each(|o| self.tocar_objetivo(&o));
        self.estado.contratos.contratos.insert(mercenario, contrato);
        self.estado.contratos.fichas.get_mut(&mercenario).expect("la ficha nace con el primer ofrecimiento").historial.firmados += 1;
        if trabajo == Trabajo::Permanente {
            self.citar_leal(mercenario, t);
        }
        let datos = vec![
            ("faccion", i64::from(faccion)),
            ("mercenario", mercenario as i64),
            ("firmante", heroe as i64),
            ("trabajo", i64::from(trabajo_a_u8(trabajo))),
            ("plazo", plazo.map_or(0, i64::from)),
            ("oro", oro),
            ("campamento", i64::from(oferta.campamento)),
        ];
        self.tr.emitir(&self.estado, EventoNucleo { heroes: vec![mercenario], facciones: vec![faccion], ..EventoNucleo::nuevo("mercenarios.contratado", t, datos) });
        Ok(vec![])
    }

    /// `prorrogar_contrato` (§6.2): quien podría firmar hoy el contrato temporal lo alarga `plazo` días, en persona y pagando la tarifa de ese plazo con el rango de hoy.
    pub(super) fn prorrogar_contrato(&mut self, t: Instante, p: &Peticion, heroe: u64, mercenario: u64, plazo: u8) -> Result<Vec<u64>, Respuesta> {
        use rechazos::*;
        self.autorizar_heroe(p, heroe, false)?;
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: &str| Respuesta::rechazo(c, m, v);
        let d = self.estado.reglas.economia.contratos.clone();
        let faccion = self.heroe(heroe).and_then(|h| h.faccion);
        let c = self.estado.contratos.contratos.get(&mercenario).filter(|c| Some(c.faccion) == faccion && c.fin_aplazado.is_none());
        let c = c.cloned().ok_or_else(|| rechazo(MERCENARIO_SIN_CONTRATO, "ese mercenario no tiene un contrato vigente con tu Facción"))?;
        if c.trabajo == Trabajo::Permanente {
            return Err(rechazo(MERCENARIO_TIPO_DE_CONTRATO, "solo se prorroga un contrato temporal"));
        }
        let s = self.situacion(c.faccion, &c.objetivo).ok_or_else(|| rechazo(MERCENARIO_OBJETIVO_INVALIDO, "el objetivo del contrato ya no existe"))?;
        puesto_para_firmar(c.trabajo, &c.objetivo, &s, &self.puestos_de(heroe), heroe).map_err(|e| rechazo_de_firma(e, v))?;
        // En persona: en su campamento base o con la columna junto a la del mercenario.
        let junto_a_el = match (self.columna_del_heroe(heroe), self.columna_del_heroe(mercenario)) {
            (Some(a), Some(b)) => a.posicion_en(t).dist(b.posicion_en(t)) <= self.radio_puerta(),
            _ => false,
        };
        if !junto_a_el && !self.esta_en_el_campamento(heroe, c.campamento_base, t) {
            return Err(rechazo(CAMPAMENTO_FUERA_DE_LA_PUERTA, "prorrogar es en persona: en el campamento base del mercenario o con tu columna junto a la suya"));
        }
        let renombre = renombre_del_plazo(&d, c.trabajo, Some(plazo)).map_err(|e| rechazo_de_firma(e, v))?;
        let columna = self.columna_que_lleva(heroe).ok_or_else(|| rechazo(HEROE_SIN_COLUMNA, "se paga con el oro del carro de tu columna: no vas en ninguna"))?;
        let oro = self.oro_a_cobrar(columna, mercenario, Some(plazo), v)?;

        self.pagar(columna, mercenario, oro);
        let vence = c.vence.expect("un temporal tiene vencimiento") + duracion(plazo);
        let guardado = self.estado.contratos.contratos.get_mut(&mercenario).expect("lo acabamos de leer");
        guardado.vence = Some(vence);
        guardado.renombre += renombre;
        self.citar_contrato(mercenario, SucesoContrato::Vence, vence);
        self.tr.tocar(Clase::Faccion, u64::from(c.faccion));
        let datos = vec![("faccion", i64::from(c.faccion)), ("mercenario", mercenario as i64), ("firmante", heroe as i64), ("plazo", i64::from(plazo)), ("oro", oro)];
        self.tr.emitir(&self.estado, EventoNucleo { heroes: vec![mercenario], facciones: vec![c.faccion], ..EventoNucleo::nuevo("mercenarios.prorrogado", t, datos) });
        Ok(vec![])
    }
}
