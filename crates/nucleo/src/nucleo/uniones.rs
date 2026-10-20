//! Juntar y separar columnas, formar ejércitos, repostar y las caravanas al entrar (Movimiento_Ejercitos §8-§11, Doc 5.14): `unirseAEjercito`
//! (en la plaza), `unirseEnCampo` y su petición de 10 s, `separarseDelEjercito`, `cederLiderazgo`, `organizarEjercito`, `cancelarFormacion`,
//! `repostar`, y los plazos de una formación. Las reglas de quién puede qué son de `militar::columna`; aquí se aplican al estado, se reanotan el consumo
//! y la velocidad de lo que cambió de composición y se programan los plazos.
//!
//! Toda orden que puede fallar se **comprueba antes de tocar nada** (las reglas puras validan antes de mutar, y lo que hay que simular se simula sobre una
//! copia de la columna): un rechazo no deja rastro.

use super::*;
use crate::estado_militar::{entidad_formacion, entidad_peticion_de_union};
use crate::estado::{SucesoMilitar, PRIO_LLEGADA};
use crate::nucleo::mercado::id32;
use crate::nucleo::mercado::RADIO_PUERTA;
use crate::nucleo::salidas::Destino;
use asentamiento::datos::UNO;
use asentamiento::Recurso;
use militar::columna::{ErrorColumna, Politica};
use militar::convocatoria::Lugar;
use militar::escuadra::Contenedor;
use militar::presencia::Ubicacion;
use militar::racion;
use militar::reclutamiento;

impl Nucleo {
    // ───────────────────────────── Fundir y desarmar ─────────────────────────────

    /// La columna `otra` entra en `base` con lo que lleva (escuadras, carro y víveres), `unidoEn = t`: sus héroes pasan a ir en `base` y `otra`
    /// desaparece. Devuelve si una formación llegó con esto al mínimo y pasó a ser un ejército. No toca nada si no se puede (§10.2).
    pub(super) fn fusionar_columna(&mut self, t: Instante, base: u64, otra: u64) -> Result<bool, ErrorColumna> {
        let mr = self.estado.reglas.economia.militar.clone();
        let antes = self.velocidad_de_ejercito(base).unwrap_or(0);
        let oe = self.estado.ejercitos.get(&otra).ok_or(ErrorColumna::NoVaDentro)?;
        let distancia = oe.posicion_en(t).dist(self.estado.ejercitos[&base].posicion_en(t));
        let (heroes_otra, escuadras_otra, columna_otra) = (Self::participantes_de(oe), oe.columna.escuadras.clone(), oe.columna.clone());
        let Estado { ejercitos, militar, .. } = &mut self.estado;
        let b = ejercitos.get_mut(&base).ok_or(ErrorColumna::NoVaDentro)?;
        let mut heroes = Self::participantes_de(b);
        heroes.extend(&heroes_otra);
        let completa = b.columna.unir_en_campo(&mr, columna_otra, distancia, t, &|ids| militar.soldados_de(&heroes, ids))?;
        let oe = ejercitos.remove(&otra).expect("existe");
        let b = ejercitos.get_mut(&base).expect("existe");
        b.caravana_fundacion |= oe.caravana_fundacion;
        b.caravanas_adjuntas.extend(oe.caravanas_adjuntas);
        b.caravanas_adjuntas.sort_unstable();
        for h in &heroes_otra {
            self.ubicar_en_columna(*h, base);
            self.estado.generaciones.subir(entidad_peticion_de_union(*h));
        }
        self.poner_escuadras_en(&heroes_otra, &escuadras_otra, Contenedor::Ejercito(base), t);
        self.baja_de_columna(otra);
        if completa {
            self.estado.generaciones.subir(entidad_formacion(base));
            self.evento_de_columna("columna.formacion_completada", t, base, vec![], vec![("participantes", heroes.len() as i64)]);
        }
        self.recomponer_columna(base, t, antes);
        Ok(completa)
    }

    /// Desarma la columna (una formación, o un ejército que entra en un lugar): el Líder se queda con la columna, que vuelve a ser personal, y cada otro
    /// nace como columna personal donde estaba con lo suyo (`Columna::desarmar`). Para quieta, si marchaba. Devuelve los ids de las columnas nuevas.
    pub(super) fn desarmar_columna(&mut self, t: Instante, id: u64) -> Vec<u64> {
        let mr = self.estado.reglas.economia.militar.clone();
        if self.estado.ejercitos[&id].marchando() {
            self.detener(id, t);
        }
        let antes = self.velocidad_de_ejercito(id).unwrap_or(0);
        let posicion = self.estado.ejercitos[&id].posicion_en(t);
        let Estado { ejercitos, militar, siguiente_id, .. } = &mut self.estado;
        let e = ejercitos.get_mut(&id).expect("existe");
        let heroes = Self::participantes_de(e);
        let suyas_de = |h: u64| militar.heroes.get(&h).map_or(vec![], |hn| hn.heroe.escuadras.iter().filter(|s| s.contenedor == Contenedor::Ejercito(id)).map(|s| s.id).collect());
        let nuevas = e.columna.desarmar(
            &mr,
            t,
            &mut || {
                *siguiente_id += 1;
                *siguiente_id - 1
            },
            &suyas_de,
            &|ids| militar.soldados_de(&heroes, ids),
        );
        let ids: Vec<u64> = nuevas.iter().map(|c| c.id).collect();
        for c in nuevas {
            let heroe = c.lider;
            self.estado.generaciones.subir(entidad_peticion_de_union(heroe));
            let e = self.columna_suelta(c, posicion);
            self.alta_de_columna(t, e);
            self.heroe_mut(heroe).heroe.presencia.ubicacion = Ubicacion::Columna(*ids.iter().find(|&&n| self.estado.ejercitos[&n].columna.lider == heroe).expect("recién creada"));
        }
        self.estado.generaciones.subir(entidad_formacion(id));
        self.recomponer_columna(id, t, antes);
        ids
    }

    /// Deshace la formación `id` (cancelada, o vencida sin llegar al mínimo): cada uno vuelve a su columna personal con lo suyo.
    pub(super) fn deshacer_formacion(&mut self, id: u64, t: Instante) {
        let sueltas = self.desarmar_columna(t, id);
        self.evento_de_columna("columna.formacion_disuelta", t, id, vec![], vec![("columnas", sueltas.len() as i64 + 1)]);
    }

    // ───────────────────────────── Caravanas al entrar ─────────────────────────────

    /// Solo entra en una plaza un ejército cuyas caravanas adjuntas son **todas** de esa plaza (Movimiento_Ejercitos §11).
    pub(super) fn caravanas_del_lugar(&self, e: &Ejercito, plaza: u64) -> Result<(), Respuesta> {
        let ajena = e.caravanas_adjuntas.iter().filter_map(|&c| self.estado.comercio.caravana(c)).find(|c| c.origen as u64 != plaza);
        match ajena {
            Some(c) => Err(Respuesta::rechazo(rechazos::CARAVANA_AJENA_AL_LUGAR, format!("la caravana {} no es de este lugar: hay que soltarla o llevarla a su origen antes de entrar", c.id), self.estado.version)),
            None => Ok(()),
        }
    }

    /// Las caravanas adjuntas de la columna quedan aparcadas en la plaza donde entra.
    pub(super) fn aparcar_todas(&mut self, t: Instante, columna: u64, plaza: u64, posicion: Punto) {
        let adjuntas = std::mem::take(&mut self.estado.ejercitos.get_mut(&columna).expect("existe").caravanas_adjuntas);
        for c in adjuntas {
            if let Some(i) = self.estado.comercio.posicion_caravana(c) {
                if self.estado.comercio.flota[i].aparcar(id32(plaza), posicion).is_ok() {
                    self.evento("ejercito.caravana_aparcada", t, columna, None, vec![("caravana", c as i64), ("plaza", plaza as i64)]);
                }
            }
        }
    }

    // ───────────────────────────── Las órdenes ─────────────────────────────

    /// Las órdenes de juntar, separar, formar y repostar. `Ok(ids)` = cambió algo y creó esos ids; rechazos y «sin cambios» salen como `Err` sin haber
    /// tocado el estado.
    pub(super) fn ejecutar_uniones(&mut self, t: Instante, p: &Peticion) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let de_columna = |e: ErrorColumna| {
            let (c, m) = columnas::rechazo_de_columna(e);
            rechazo(c, m.into())
        };
        let mr = self.estado.reglas.economia.militar.clone();
        match &p.comando {
            Comando::UnirseAEjercito { asentamiento, heroe, ejercito, escuadras, carga } => {
                let plaza = self.plaza_o_rechazo(*asentamiento)?;
                let (faccion, posicion) = (plaza.faccion, plaza.posicion);
                self.autorizar_heroe(p, *heroe, false)?;
                self.sin_convocatoria(*heroe)?;
                let e = self.estado.ejercitos.get(ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{ejercito}")))?;
                if self.heroe(*heroe).expect("existe").heroe.residencia != Some(militar::heroe::Residencia::Asentamiento(*asentamiento as u32)) {
                    return Err(rechazo(rechazos::SALIDA_SOLO_DE_TU_RESIDENCIA, "solo un residente puede sacar tropas de esta plaza".into()));
                }
                if e.columna.faccion != Some(faccion) {
                    return Err(de_columna(ErrorColumna::OtraFaccion));
                }
                if !p.admin && e.posicion_en(t).dist(posicion) > RADIO_PUERTA {
                    return Err(rechazo(rechazos::ASENTAMIENTO_FUERA_DE_LA_PUERTA, "el ejército no está a la puerta de la plaza".into()));
                }
                if !Self::es_ejercito(e) {
                    return Err(de_columna(ErrorColumna::NoHayEjercito));
                }
                if e.columna.politica == Politica::Rechazar {
                    return Err(de_columna(ErrorColumna::NoAdmite));
                }
                let plan = self.validar_salida(t, *heroe, Lugar::Asentamiento(*asentamiento as u32), escuadras, carga, true, &mut Default::default())?;
                self.aplicar_salida(t, &plan, Destino::Unirse(*ejercito));
                self.evento_de_columna("ejercito.refuerzo", t, *ejercito, vec![*asentamiento], vec![("heroe", *heroe as i64), ("escuadras", plan.escuadras.len() as i64), ("cargaTotal", comercio::total(&plan.carga))]);
                Ok(vec![])
            }
            Comando::UnirseEnCampo { heroe, ejercito } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let (propia, e1) = self.columna_a_bordo(*heroe)?;
                let objetivo = self.estado.ejercitos.get(ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{ejercito}")))?;
                let distancia = e1.posicion_en(t).dist(objetivo.posicion_en(t));
                let preguntar = objetivo.columna.politica == Politica::Preguntar;
                objetivo.columna.validar_union_en_campo(&mr, &e1.columna, distancia).map_err(de_columna)?;
                if !preguntar {
                    let plazas = objetivo.columna.faccion.map_or(vec![], |_| vec![]);
                    self.fusionar_columna(t, *ejercito, propia).map_err(de_columna)?;
                    self.evento_de_columna("columna.union_en_campo", t, *ejercito, plazas, vec![("heroe", *heroe as i64), ("columna", propia as i64)]);
                    return Ok(vec![]);
                }
                let otra = e1.columna.clone();
                let expira_en = self.estado.ejercitos.get_mut(ejercito).expect("existe").columna.pedir_union(&mr, &otra, distancia, t).map_err(de_columna)?;
                self.estado.generaciones.subir(entidad_peticion_de_union(*heroe));
                self.programar_caducidad_de_peticion(*ejercito, *heroe, expira_en);
                self.tr.tocados.push(*ejercito);
                self.evento_de_columna("columna.union_pedida", t, *ejercito, vec![], vec![("heroe", *heroe as i64), ("expiraEn", expira_en)]);
                Ok(vec![])
            }
            Comando::ResponderPeticionDeUnion { heroe, ejercito, solicitante, aceptar } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let e = self.estado.ejercitos.get(ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{ejercito}")))?;
                // Se comprueba todo sobre una copia: la petición se retira, y si se acepta, el solicitante tiene que seguir junto a la columna.
                let mut copia = e.columna.clone();
                copia.retirar_peticion(*heroe, *solicitante, t).map_err(de_columna)?;
                let propia = if *aceptar {
                    let otra = self.columna_del_heroe(*solicitante).filter(|o| self.heroe(*solicitante).is_some_and(|h| h.heroe.presencia.ubicacion == Ubicacion::Columna(o.id)));
                    let otra = otra.ok_or_else(|| rechazo(rechazos::HEROE_SIN_COLUMNA, "el solicitante ya no va en una columna".into()))?;
                    copia.validar_union_en_campo(&mr, &otra.columna, otra.posicion_en(t).dist(e.posicion_en(t))).map_err(de_columna)?;
                    Some(otra.id)
                } else {
                    None
                };
                self.estado.ejercitos.get_mut(ejercito).expect("existe").columna.retirar_peticion(*heroe, *solicitante, t).expect("comprobado");
                self.estado.generaciones.subir(entidad_peticion_de_union(*solicitante));
                self.tr.tocados.push(*ejercito);
                match propia {
                    Some(otra) => {
                        self.fusionar_columna(t, *ejercito, otra).expect("comprobado");
                        self.evento_de_columna("columna.union_en_campo", t, *ejercito, vec![], vec![("heroe", *solicitante as i64), ("columna", otra as i64)]);
                    }
                    None => self.evento_de_columna("columna.union_rechazada", t, *ejercito, vec![], vec![("heroe", *solicitante as i64)]),
                }
                Ok(vec![])
            }
            Comando::SepararseDelEjercito { heroe } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let (id, e) = self.columna_a_bordo(*heroe)?;
                let nuevo = self.estado.siguiente_id;
                let suyas = self.escuadras_de_heroe_en(*heroe, id);
                let posicion = e.posicion_en(t);
                let mut heroes = Self::participantes_de(e);
                let mut copia = e.columna.clone();
                let militar = &self.estado.militar;
                let nueva = copia.separarse(&mr, *heroe, &suyas, nuevo, t, &|ids| militar.soldados_de(&heroes, ids)).map_err(de_columna)?;
                heroes.retain(|h| h != heroe);
                // Lo comprobado se hace de verdad: el ejército se queda con lo que quedó y el que se va nace donde estaba.
                let antes = self.velocidad_de_ejercito(id).unwrap_or(0);
                self.estado.siguiente_id += 1;
                self.estado.ejercitos.get_mut(&id).expect("existe").columna = copia;
                let e = self.columna_suelta(nueva, posicion);
                self.alta_de_columna(t, e);
                self.ubicar_en_columna(*heroe, nuevo);
                self.recomponer_columna(id, t, antes);
                self.evento_de_columna("columna.separacion", t, id, vec![], vec![("heroe", *heroe as i64), ("columna", nuevo as i64)]);
                Ok(vec![nuevo])
            }
            Comando::CederLiderazgo { heroe, sucesor } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let (id, _) = self.columna_a_bordo(*heroe)?;
                self.estado.ejercitos.get_mut(&id).expect("existe").columna.ceder_liderazgo(*heroe, *sucesor).map_err(de_columna)?;
                self.tr.tocados.push(id);
                self.evento_de_columna("columna.liderazgo_cedido", t, id, vec![], vec![("anterior", *heroe as i64), ("sucesor", *sucesor as i64)]);
                Ok(vec![])
            }
            Comando::OrganizarEjercito { heroe, politica } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let (id, e) = self.columna_a_bordo(*heroe)?;
                let heroes = Self::participantes_de(e);
                let militar = &self.estado.militar;
                // Antes de parar la columna se comprueba sobre una copia que se puede organizar.
                e.columna.clone().organizar(&mr, *politica, t, &|ids| militar.soldados_de(&heroes, ids)).map_err(de_columna)?;
                if e.marchando() {
                    self.detener(id, t);
                }
                let Estado { ejercitos, militar, .. } = &mut self.estado;
                let expira_en = ejercitos.get_mut(&id).expect("existe").columna.organizar(&mr, *politica, t, &|ids| militar.soldados_de(&heroes, ids)).expect("comprobado");
                let clave = self.estado.generaciones.clave(entidad_formacion(id));
                self.estado.cola.programar(expira_en, PRIO_LLEGADA, clave, Carga::Militar(SucesoMilitar::CaducaFormacion));
                self.reprogramar_columna(id, t);
                self.evento_de_columna("columna.formacion_iniciada", t, id, vec![], vec![("lider", *heroe as i64), ("expiraEn", expira_en)]);
                Ok(vec![])
            }
            Comando::CancelarFormacion { heroe } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let (id, e) = self.columna_a_bordo(*heroe)?;
                if !e.columna.es_formacion() {
                    return Err(de_columna(ErrorColumna::NoEsUnaFormacion));
                }
                if e.columna.lider != *heroe {
                    return Err(de_columna(ErrorColumna::SoloElLider));
                }
                self.deshacer_formacion(id, t);
                Ok(vec![])
            }
            Comando::Repostar { heroe, asentamiento } => self.repostar(t, p, *heroe, *asentamiento),
            _ => unreachable!("solo se llama con órdenes de juntar y separar columnas"),
        }
    }

    /// `repostar` (Movimiento_Ejercitos §8): el Líder, con la columna a la puerta de una plaza que le abre su almacén, rellena los víveres de la columna
    /// hasta donde quepan y sin bajar de la reserva de trigo de la plaza. Es la misma operación que llenarlos al salir.
    // DUDA: una columna personal reposta sola dentro de la zona de la plaza (evento de entrada en la zona más un ritmo); mientras no exista, la orden vale
    // también para ella (fila de paridad). `puedeRepostarEn` solo conoce la plaza propia: las aliadas (`permiteReabastecerAliados`) son del sistema político.
    fn repostar(&mut self, t: Instante, p: &Peticion, heroe: u64, asentamiento: u64) -> Result<Vec<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let mr = self.estado.reglas.economia.militar.clone();
        let plaza = self.plaza_o_rechazo(asentamiento)?;
        let (faccion, posicion) = (plaza.faccion, plaza.posicion);
        self.autorizar_heroe(p, heroe, false)?;
        let (id, e) = self.columna_a_bordo(heroe)?;
        if e.columna.lider != heroe {
            return Err(rechazo(rechazos::COLUMNA_SOLO_EL_LIDER, "solo el Líder reposta".into()));
        }
        if !p.admin && e.posicion_en(t).dist(posicion) > RADIO_PUERTA {
            return Err(rechazo(rechazos::ASENTAMIENTO_FUERA_DE_LA_PUERTA, "la columna no está a la puerta de la plaza".into()));
        }
        if e.columna.faccion != Some(faccion) {
            return Err(rechazo(rechazos::REPOSTAR_PLAZA_CERRADA, "esa plaza no abre su almacén a esta columna".into()));
        }
        let (llevaba, cap) = (e.columna.viveres.valor_en(t), e.columna.viveres.maximo.unwrap_or(0));
        if cap - llevaba <= 0 {
            return Err(rechazo(rechazos::COLUMNA_VIVERES_LLENOS, "los víveres ya están llenos".into()));
        }
        let consumo = self.con_exterior(asentamiento, t, |eco, x| eco.resumen_de_reclutamiento(x.reglas, x).consumo_poblacion).ok_or_else(|| rechazo(rechazos::SIN_MUNDO, "el núcleo no tiene mundo cargado".into()))?;
        let reserva = reclutamiento::reserva_de_trigo(&mr, consumo, self.soldados_del_campamento(asentamiento));
        let de_sobra = (self.estado.asentamientos[&asentamiento].economia.almacen.cantidad(Recurso::Trigo) - reserva).max(0);
        if de_sobra == 0 {
            return Err(rechazo(rechazos::REPOSTAR_SIN_EXISTENCIAS, "la plaza no tiene trigo de sobra: solo queda la reserva".into()));
        }
        let e = self.estado.ejercitos.get_mut(&id).expect("existe");
        let tomado = racion::llenar(&mut e.columna.viveres, t, de_sobra);
        self.estado.asentamientos.get_mut(&asentamiento).expect("existe").economia.almacen.quitar(Recurso::Trigo, tomado);
        self.tr.asentamientos.push(asentamiento);
        self.reprogramar_columna(id, t);
        // Solo es noticia lo que llena unos víveres que iban por debajo del umbral; el goteo no.
        if llevaba < cap * mr.logistica.umbral_narrar_reposte / UNO {
            self.evento_de_columna("ejercito.reabastecido", t, id, vec![asentamiento], vec![("heroe", heroe as i64), ("repuesto", tomado)]);
        }
        Ok(vec![])
    }

}
