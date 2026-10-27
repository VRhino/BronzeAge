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
use crate::nucleo::salidas::Destino;
use asentamiento::datos::UNO;
use asentamiento::Recurso;
use militar::columna::{ErrorColumna, Politica, TipoColumna};
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
        b.caravanas_adjuntas.extend(oe.caravanas_adjuntas);
        b.caravanas_adjuntas.sort_unstable();
        // Sus Caravanas de Fundación siguen con su titular, que va ahora en `base`.
        self.reenganchar_fundaciones(otra, base);
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

    /// Desarma la columna (una formación, o un ejército que entra en un lugar): el Líder se queda con la columna, que vuelve a ser personal (salvo que lleve
    /// caravanas adjuntas), y cada otro nace como columna personal donde estaba con lo suyo (`Columna::desarmar`). Para quieta, si marchaba. Devuelve los ids de las
    /// columnas nuevas.
    pub(super) fn desarmar_columna(&mut self, t: Instante, id: u64) -> Vec<u64> {
        let mr = self.estado.reglas.economia.militar.clone();
        if self.estado.ejercitos[&id].marchando() {
            self.detener(id, t);
        }
        let antes = self.velocidad_de_ejercito(id).unwrap_or(0);
        let posicion = self.estado.ejercitos[&id].posicion_en(t);
        let con_fundacion = self.estado.fundacion.de_columna(id).is_some();
        let Estado { ejercitos, militar, siguiente_id, .. } = &mut self.estado;
        let e = ejercitos.get_mut(&id).expect("existe");
        // Un ejército que entra en un lugar con caravanas adjuntas las conserva con su Líder, y una personal no las lleva: sigue siendo un ejército mientras las lleve.
        let (tipo, politica) = (e.columna.tipo, e.columna.politica);
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
        if !e.caravanas_adjuntas.is_empty() || con_fundacion {
            (e.columna.tipo, e.columna.politica) = (tipo, politica);
        }
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
                let e = self.estado.ejercitos.get(ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(*ejercito)))?;
                if self.heroe(*heroe).expect("existe").heroe.residencia != Some(militar::heroe::Residencia::Asentamiento(*asentamiento as u32)) {
                    return Err(rechazo(rechazos::SALIDA_SOLO_DE_TU_RESIDENCIA, "solo un residente puede sacar tropas de esta plaza".into()));
                }
                if e.columna.faccion != Some(faccion) {
                    return Err(de_columna(ErrorColumna::OtraFaccion));
                }
                if !p.admin && e.posicion_en(t).dist(posicion) > self.radio_puerta() {
                    return Err(rechazo(rechazos::ASENTAMIENTO_FUERA_DE_LA_PUERTA, "el ejército no está a la puerta de la plaza".into()));
                }
                if !Self::es_ejercito(e) {
                    return Err(de_columna(ErrorColumna::NoHayEjercito));
                }
                if e.columna.politica == Politica::Rechazar {
                    return Err(de_columna(ErrorColumna::NoAdmite));
                }
                let plan = self.validar_salida(t, *heroe, Lugar::Asentamiento(*asentamiento as u32), escuadras, carga, true, &mut Default::default())?;
                self.comprobar_fusion(&plan, faccion)?;
                self.aplicar_salida(t, &plan, Destino::Unirse(*ejercito));
                self.evento_de_columna("ejercito.refuerzo", t, *ejercito, vec![*asentamiento], vec![("heroe", *heroe as i64), ("escuadras", plan.escuadras.len() as i64), ("cargaTotal", comercio::total(&plan.carga))]);
                self.avance(t, Dueno::Heroe(*heroe), "uniones", 1);
                Ok(vec![])
            }
            Comando::UnirseEnCampo { heroe, ejercito } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let (propia, e1) = self.columna_a_bordo(*heroe)?;
                let objetivo = self.estado.ejercitos.get(ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(*ejercito)))?;
                let distancia = e1.posicion_en(t).dist(objetivo.posicion_en(t));
                let preguntar = objetivo.columna.politica == Politica::Preguntar;
                objetivo.columna.validar_union_en_campo(&mr, &e1.columna, distancia).map_err(de_columna)?;
                if !preguntar {
                    let plazas = objetivo.columna.faccion.map_or(vec![], |_| vec![]);
                    self.fusionar_columna(t, *ejercito, propia).map_err(de_columna)?;
                    self.evento_de_columna("columna.union_en_campo", t, *ejercito, plazas, vec![("heroe", *heroe as i64), ("columna", propia as i64)]);
                    self.avance(t, Dueno::Heroe(*heroe), "uniones", 1);
                    return Ok(vec![]);
                }
                let otra = e1.columna.clone();
                let expira_en = self.estado.ejercitos.get_mut(ejercito).expect("existe").columna.pedir_union(&mr, &otra, distancia, t).map_err(de_columna)?;
                self.estado.generaciones.subir(entidad_peticion_de_union(*heroe));
                self.programar_caducidad_de_peticion(*ejercito, *heroe, expira_en);
                self.tr.tocar(Clase::Ejercito, *ejercito);
                self.evento_de_columna("columna.union_pedida", t, *ejercito, vec![], vec![("heroe", *heroe as i64), ("expiraEn", expira_en)]);
                Ok(vec![])
            }
            Comando::ResponderPeticionDeUnion { heroe, ejercito, solicitante, aceptar } => {
                self.autorizar_heroe(p, *heroe, false)?;
                let e = self.estado.ejercitos.get(ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(*ejercito)))?;
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
                self.tr.tocar(Clase::Ejercito, *ejercito);
                match propia {
                    Some(otra) => {
                        self.fusionar_columna(t, *ejercito, otra).expect("comprobado");
                        self.evento_de_columna("columna.union_en_campo", t, *ejercito, vec![], vec![("heroe", *solicitante as i64), ("columna", otra as i64)]);
                        self.avance(t, Dueno::Heroe(*solicitante), "uniones", 1);
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
                self.tr.tocar(Clase::Ejercito, id);
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
    // La orden vale también para una columna personal, que además reposta sola dentro de la zona de una plaza de su Facción ([`Nucleo::repostar_en_la_zona`]). `puedeRepostarEn`
    // (5D): la plaza propia, y la de una aliada o vasalla que lo permite (`permiteReabastecerAliados`).
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
        if !p.admin && e.posicion_en(t).dist(posicion) > self.radio_puerta() {
            return Err(rechazo(rechazos::ASENTAMIENTO_FUERA_DE_LA_PUERTA, "la columna no está a la puerta de la plaza".into()));
        }
        if e.columna.faccion != Some(faccion) && !self.abre_su_almacen_a(asentamiento, e.columna.faccion) {
            return Err(rechazo(rechazos::REPOSTAR_PLAZA_CERRADA, "esa plaza no abre su almacén a esta columna".into()));
        }
        let (llevaba, cap) = (e.columna.viveres.valor_en(t), e.columna.viveres.maximo.unwrap_or(0));
        if cap - llevaba < base::UNIDAD {
            return Err(rechazo(rechazos::COLUMNA_VIVERES_LLENOS, "los víveres ya están llenos".into()));
        }
        let consumo = self.con_exterior(asentamiento, t, |eco, x| eco.resumen_de_reclutamiento(x.reglas, x).consumo_poblacion).ok_or_else(|| rechazo(rechazos::SIN_MUNDO, "el núcleo no tiene mundo cargado".into()))?;
        let reserva = reclutamiento::reserva_de_trigo(&mr, consumo, self.soldados_del_campamento(asentamiento));
        let de_sobra = (self.estado.asentamientos[&asentamiento].economia.almacen.cantidad(Recurso::Trigo) - reserva).max(0);
        if de_sobra == 0 {
            return Err(rechazo(rechazos::REPOSTAR_SIN_EXISTENCIAS, "la plaza no tiene trigo de sobra: solo queda la reserva".into()));
        }
        let e = self.estado.ejercitos.get_mut(&id).expect("existe");
        let tomado = racion::llenar_en_unidades(&mut e.columna.viveres, t, de_sobra);
        self.estado.asentamientos.get_mut(&asentamiento).expect("existe").economia.almacen.quitar(Recurso::Trigo, tomado);
        self.tr.tocar(Clase::Asentamiento, asentamiento);
        self.reprogramar_columna(id, t);
        self.avance(t, Dueno::Heroe(heroe), "repostajes", 1);
        // Solo es noticia lo que llena unos víveres que iban por debajo del umbral; el goteo no.
        if llevaba < cap * mr.logistica.umbral_narrar_reposte / UNO {
            self.evento_de_columna("ejercito.reabastecido", t, id, vec![asentamiento], vec![("heroe", heroe as i64), ("repuesto", tomado)]);
        }
        Ok(vec![])
    }

    /// Repostaje automático (Movimiento_Ejercitos §8, decisión del autor del 2026-10-27), en el paso de minuto de la plaza `plaza`: cada columna **personal** de su Facción que está en el
    /// mapa dentro de su zona real y no tiene los víveres llenos recibe `factorRepostajeEnZona` veces lo que come en un minuto (en unidades enteras hacia arriba, al menos una), hasta su
    /// tope y solo de lo que a la plaza le sobra por encima de su reserva de trigo; sin sobra no reposta a nadie. Las columnas van por id. No se narra: es un goteo.
    pub(super) fn repostar_en_la_zona(&mut self, plaza: u64, t: Instante) {
        let Some((yo, vecinos)) = self.zona_real(plaza) else { return };
        let factor = self.estado.reglas.economia.militar.logistica.factor_repostaje_en_zona;
        let candidatas: Vec<(u64, Mil)> = self
            .estado
            .ejercitos
            .values()
            .filter(|e| e.columna.tipo == TipoColumna::Personal && e.columna.faccion == Some(yo.faccion) && !e.en_batalla && self.columna_en_el_mapa(e))
            .filter(|e| e.columna.viveres.maximo.unwrap_or(0) - e.columna.viveres.valor_en(t) >= base::UNIDAD)
            .filter(|e| territorio::en_zona(e.posicion_en(t), &yo, &vecinos))
            .map(|e| {
                let por_minuto = asentamiento::datos::por((-e.columna.viveres.por_minuto).max(0), factor);
                (e.id, ((por_minuto + base::UNIDAD - 1) / base::UNIDAD).max(1) * base::UNIDAD)
            })
            .collect();
        if candidatas.is_empty() {
            return;
        }
        let mr = self.estado.reglas.economia.militar.clone();
        let Some(consumo) = self.con_exterior(plaza, t, |eco, x| eco.resumen_de_reclutamiento(x.reglas, x).consumo_poblacion) else { return };
        let reserva = reclutamiento::reserva_de_trigo(&mr, consumo, self.soldados_del_campamento(plaza));
        let mut de_sobra = (self.estado.asentamientos[&plaza].economia.almacen.cantidad(Recurso::Trigo) - reserva).max(0);
        let mut total = 0;
        for (id, oferta) in candidatas {
            if de_sobra < base::UNIDAD {
                break;
            }
            let e = self.estado.ejercitos.get_mut(&id).expect("existe");
            let tomado = racion::llenar_en_unidades(&mut e.columna.viveres, t, oferta.min(de_sobra));
            if tomado > 0 {
                de_sobra -= tomado;
                total += tomado;
                self.reprogramar_columna(id, t);
            }
        }
        if total > 0 {
            self.estado.asentamientos.get_mut(&plaza).expect("existe").economia.almacen.quitar(Recurso::Trigo, total);
            self.tr.tocar(Clase::Asentamiento, plaza);
        }
    }

    /// Dónde reposta el Líder cuando la orden no lo dice (`repostar` sin `lugar_id`): la plaza o la avanzada operativa más cercana a la puerta (`radioPuerta`) de la columna
    /// del héroe, con las plazas antes que las avanzadas y el menor id si empatan. `None` si no hay ninguna: la orden se rechaza con `repostar.sin_lugar`.
    pub fn lugar_de_repostar(&self, heroe: u64) -> Option<Ubicacion> {
        let aqui = self.columna_del_heroe(heroe)?.posicion_en(self.estado.ahora);
        let plazas = self.estado.asentamientos.values().map(|a| (aqui.dist(a.posicion), 0, a.id, Ubicacion::Asentamiento(a.id as u32)));
        let avanzadas = self.estado.asentamientos.values().flat_map(|a| &a.economia.avanzadas).filter(|v| v.operativa()).map(|v| (aqui.dist(v.posicion), 1, v.id, Ubicacion::Avanzada(v.id)));
        plazas.chain(avanzadas).filter(|&(d, ..)| d <= self.radio_puerta()).min_by_key(|&(d, rango, id, _)| (d, rango, id)).map(|(.., lugar)| lugar)
    }
}
