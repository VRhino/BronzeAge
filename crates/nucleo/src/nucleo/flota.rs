//! La flota de caravanas dentro del núcleo (Comercio_Caravanas §5, §10): crearlas, componerlas, lanzarlas a mano (queda `preparando`; el
//! movimiento es de la fase 3B-2), la aparcada, la entrega manual de una adjunta, y el cierre del comercio de una plaza que cae en ruinas o
//! cambia de dueño. Las reglas puras viven en el crate `comercio`.
//!
//! # Lo que aún no se mueve
//! Ninguna caravana viaja todavía: no hay plan, ni peajes, ni reparto automático. `prepararCaravana` guarda la polilínea que calcula
//! `crates/rutas` (sin la atracción de la red de caminos) y no admite la salida inmediata (un solo carro y sin `salir_en`), que entraría en
//! `en_transito` sin plan: con preparación, el evento de fin se limita a registrarla (`comercio.preparacion_terminada`) y la caravana sigue
//! `preparando` hasta que la cancelen.

use super::*;
use crate::estado::SucesoComercio;
use crate::estado_comercio::{entidad_caravana, entidad_trueque};
use crate::nucleo::mercado::{id32, nivel_mercado, RADIO_PUERTA};
use asentamiento::edificio::Edificios;
use asentamiento::Recurso;
use comercio::caravana::{caravanas_comerciales_de, carros_en_uso, mover_carro, Caravana, ContextoCarro, ContextoCrear, ErrorCaravana, EstadoCaravana, Lanzamiento};
use comercio::evento::Evento as EventoComercio;
use comercio::ruina::{cerrar_dependientes, Accion, AccionCaravana, Motivo, TruequeCancelado};
use comercio::trueque::EstadoTrueque;
use comercio::{total, Carga as Mercancia, Lado};

fn rechazo_de_caravana(e: ErrorCaravana) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorCaravana::NoEsComercial => (CARAVANA_NO_ES_COMERCIAL, "la caravana no es comercial"),
        ErrorCaravana::NoDisponible => (CARAVANA_NO_DISPONIBLE, "la caravana no está disponible (parada en su origen)"),
        ErrorCaravana::NoPreparando => (CARAVANA_NO_PREPARANDO, "la caravana no está preparando"),
        ErrorCaravana::NoAparcada => (CARAVANA_NO_APARCADA, "la caravana no está aparcada"),
        ErrorCaravana::EstadoInvalido => (CARAVANA_ESTADO_INVALIDO, "la caravana no puede pasar a ese estado desde el actual"),
        ErrorCaravana::SinMercado => (CARAVANA_SIN_MERCADO, "la plaza no tiene un Mercado activo"),
        ErrorCaravana::EnCooldown => (CARAVANA_EN_COOLDOWN, "aún no pasó el tiempo mínimo desde la última caravana creada"),
        ErrorCaravana::CupoDeCaravanas => (CARAVANA_CUPO_DE_CARAVANAS, "la plaza ya tiene todas las caravanas que su Mercado permite"),
        ErrorCaravana::CupoDeCarros => (CARAVANA_CUPO_DE_CARROS, "la flota ya tiene todos los carros que el Mercado permite"),
        ErrorCaravana::SinCarpinteria => (CARAVANA_SIN_CARPINTERIA, "el carro reforzado necesita una Carpintería activa"),
        ErrorCaravana::SinRecursos => (CARAVANA_SIN_RECURSOS, "el almacén no tiene el coste"),
        ErrorCaravana::CarroInexistente => (CARAVANA_CARRO_INEXISTENTE, "la caravana no tiene ese carro"),
        ErrorCaravana::CarroYaTieneAnimal => (CARAVANA_CARRO_YA_TIENE_ANIMAL, "el carro ya tiene un animal"),
        ErrorCaravana::OrigenDistinto => (CARAVANA_ORIGEN_DISTINTO, "las dos caravanas deben ser distintas y del mismo origen"),
        ErrorCaravana::SinCapacidad => (CARAVANA_SIN_CAPACIDAD, "la caravana no tiene ningún carro con animal"),
        ErrorCaravana::CargaInvalida => (CARAVANA_CARGA_INVALIDA, "la carga está vacía, es negativa o pasa de la capacidad"),
        ErrorCaravana::SinStock => (CARAVANA_SIN_STOCK, "el almacén de origen no tiene lo que se quiere cargar"),
        ErrorCaravana::CupoDeEscolta => (CARAVANA_CUPO_DE_ESCOLTA, "la escolta pasa del cupo de Liderazgo de la caravana"),
        ErrorCaravana::EscoltaInvalida => (CARAVANA_ESCOLTA_INVALIDA, "la escolta no es válida"),
        ErrorCaravana::SinRuta => (CARAVANA_SIN_RUTA, "no hay ruta por tierra hasta el destino"),
        ErrorCaravana::SalidaInvalida => (CARAVANA_SALIDA_INVALIDA, "la hora de salida debe estar entre ahora y el máximo de programación"),
        ErrorCaravana::NoHaLlegado => (CARAVANA_NO_HA_LLEGADO, "la caravana aún no ha llegado"),
        ErrorCaravana::SinPlan => (CARAVANA_SIN_PLAN, "la caravana no tiene plan que seguir"),
        ErrorCaravana::NadaQueMover => (CARAVANA_NADA_QUE_MOVER, "no hay nada que mover"),
    }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// Una caravana por id.
    pub fn caravana(&self, id: u64) -> Option<&Caravana> {
        self.estado.comercio.caravana(id)
    }

    /// La flota comercial de una plaza (las caravanas con ese origen, en cualquier estado), en orden de id.
    pub fn flota_de(&self, plaza: u64) -> Vec<&Caravana> {
        self.estado.comercio.flota_de(plaza).collect()
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// La caravana, su posición en la flota y la plaza de su origen; el actor debe mandar en esa plaza (residente del origen, §11).
    fn caravana_de_orden(&self, p: &Peticion, id: u64) -> Result<(usize, u64), Respuesta> {
        let v = self.estado.version;
        let i = self.estado.comercio.posicion_caravana(id).ok_or_else(|| Respuesta::rechazo(rechazos::CARAVANA_DESCONOCIDA, format!("caravana-{id}"), v))?;
        let origen = self.estado.comercio.flota[i].origen as u64;
        let faccion = self.plaza_o_rechazo(origen)?.faccion;
        if !self.manda_en(p, faccion) {
            return Err(Respuesta::rechazo(rechazos::NO_AUTORIZADO, format!("no mandas en as-{origen}"), v));
        }
        Ok((i, origen))
    }

    /// Las órdenes de la flota. `Ok(Some(n))` = cambió algo y creó `n`; `Ok(None)` = cambió algo; rechazos y «sin cambios» salen como `Err` sin
    /// haber tocado el estado.
    pub(super) fn ejecutar_flota(&mut self, t: Instante, p: &Peticion) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let sin_cambios = || Respuesta { resultado: Resultado::SinCambios, version_tras: v, creadas: vec![] };
        let solo_admin = || (!p.admin).then(|| rechazo(rechazos::NO_AUTORIZADO, "solo administración".into()));
        let cr = self.estado.reglas.economia.comercio.clone();
        let de_caravana = |e: ErrorCaravana| {
            let (c, m) = rechazo_de_caravana(e);
            rechazo(c, m.into())
        };
        let desconocida = |id: u64| rechazo(rechazos::CARAVANA_DESCONOCIDA, format!("caravana-{id}"));
        match &p.comando {
            Comando::CrearCaravana { asentamiento } => {
                let plaza = self.plaza_o_rechazo(*asentamiento)?;
                if !self.manda_en(p, plaza.faccion) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, format!("no mandas en as-{asentamiento}")));
                }
                let ctx = ContextoCrear {
                    mercado_activo: nivel_mercado(plaza).is_some(),
                    ultima_creada: self.estado.comercio.ultima_caravana.get(asentamiento).copied(),
                    existentes: caravanas_comerciales_de(&self.estado.comercio.flota, id32(*asentamiento)),
                    cupo_caravanas: self.cupos_de(plaza).caravanas,
                };
                let id = self.estado.comercio.siguiente_caravana;
                let caravana = Caravana::crear(&cr.flota, &ctx, t, id, id32(*asentamiento), plaza.posicion).map_err(de_caravana)?;
                self.estado.comercio.siguiente_caravana += 1;
                self.estado.comercio.flota.push(caravana);
                self.estado.comercio.ultima_caravana.insert(*asentamiento, t);
                self.emitir_comercio(t, EventoComercio::CaravanaConstruida { caravana: id }, vec![*asentamiento], &[]);
                Ok(Some(id))
            }
            Comando::AnadirCarro { caravana, tipo } => {
                let (i, origen) = self.caravana_de_orden(p, *caravana)?;
                let plaza = &self.estado.asentamientos[&origen];
                let ctx = ContextoCarro {
                    carpinteria_activa: plaza.economia.edificios.activos(asentamiento::Tipo::Carpinteria) > 0,
                    carros_en_uso: carros_en_uso(&self.estado.comercio.flota, id32(origen)),
                    cupo_carros: self.cupos_de(plaza).carros,
                };
                let Estado { comercio, asentamientos, .. } = &mut self.estado;
                let almacen = &mut asentamientos.get_mut(&origen).expect("existe").economia.almacen;
                comercio.flota[i].agregar_carro(&cr.flota, *tipo, &ctx, almacen).map_err(de_caravana)?;
                self.tr.asentamientos.push(origen);
                self.emitir_comercio(t, EventoComercio::CaravanaCarroAgregado { caravana: *caravana }, vec![origen], &[]);
                Ok(None)
            }
            Comando::AnadirAnimal { caravana, carro, animal } => {
                let (i, origen) = self.caravana_de_orden(p, *caravana)?;
                let Estado { comercio, asentamientos, .. } = &mut self.estado;
                let almacen = &mut asentamientos.get_mut(&origen).expect("existe").economia.almacen;
                comercio.flota[i].comprar_animal(&cr.flota, *carro as usize, *animal, almacen).map_err(de_caravana)?;
                self.tr.asentamientos.push(origen);
                self.emitir_comercio(t, EventoComercio::CaravanaAnimalComprado { caravana: *caravana }, vec![origen], &[]);
                Ok(None)
            }
            Comando::MoverCarro { de, a, carro } => {
                let (i, origen) = self.caravana_de_orden(p, *de)?;
                let j = self.estado.comercio.posicion_caravana(*a).ok_or_else(|| desconocida(*a))?;
                if i == j {
                    return Err(de_caravana(ErrorCaravana::OrigenDistinto));
                }
                let flota = &mut self.estado.comercio.flota;
                let (origen_de, destino_de) = if i < j {
                    let (l, r) = flota.split_at_mut(j);
                    (&mut l[i], &mut r[0])
                } else {
                    let (l, r) = flota.split_at_mut(i);
                    (&mut r[0], &mut l[j])
                };
                mover_carro(origen_de, destino_de, *carro as usize).map_err(de_caravana)?;
                self.emitir_comercio(t, EventoComercio::CaravanaCarroMovido { caravana: *de }, vec![origen], &[("hacia", *a as i64)]);
                Ok(None)
            }
            Comando::ReservarCaravana { caravana, reservada } => {
                let (i, origen) = self.caravana_de_orden(p, *caravana)?;
                let c = &mut self.estado.comercio.flota[i];
                if c.reservada_manual == *reservada {
                    return Err(sin_cambios());
                }
                c.reservar(*reservada);
                self.emitir_comercio(t, EventoComercio::CaravanaReserva { caravana: *caravana, reservada: *reservada }, vec![origen], &[]);
                Ok(None)
            }
            Comando::PrepararCaravana { caravana, destino, carga, salir_en } => self.preparar_caravana(t, p, *caravana, *destino, carga, *salir_en),
            Comando::CancelarCaravana { caravana } => {
                let (i, origen) = self.caravana_de_orden(p, *caravana)?;
                let (devuelto, perdido) = self.cancelar_preparacion(i, origen).map_err(de_caravana)?;
                self.invalidar_comercio(entidad_caravana(*caravana));
                self.tr.asentamientos.push(origen);
                self.emitir_comercio(t, EventoComercio::CaravanaCancelada { caravana: *caravana }, vec![origen], &[("devuelto", devuelto), ("perdido", perdido)]);
                Ok(None)
            }
            Comando::MoverCargaAparcada { caravana, recurso, cantidad, cargar } => {
                let (i, origen) = self.caravana_de_orden(p, *caravana)?;
                let c = &self.estado.comercio.flota[i];
                if c.estado != EstadoCaravana::Aparcada {
                    return Err(de_caravana(ErrorCaravana::NoAparcada));
                }
                // DUDA: el documento exige además estar presente en la plaza que la hospeda siendo residente del origen: sin héroes ni residencia,
                // basta con mandar en el origen.
                let Some(anfitriona) = c.anfitriona.map(u64::from) else { return Err(de_caravana(ErrorCaravana::NoAparcada)) };
                self.plaza_o_rechazo(anfitriona)?;
                let factor = self.factores_de(&self.estado.asentamientos[&origen]).capacidad;
                let Estado { comercio, asentamientos, .. } = &mut self.estado;
                let almacen = &mut asentamientos.get_mut(&anfitriona).expect("existe").economia.almacen;
                let c = &mut comercio.flota[i];
                let movido = if *cargar { c.cargar_aparcada(&cr.flota, factor, almacen, *recurso, *cantidad) } else { c.descargar_aparcada(almacen, *recurso, *cantidad) };
                let n = movido.map_err(de_caravana)?;
                self.tr.asentamientos.push(anfitriona);
                self.emitir_comercio(t, EventoComercio::CargaCaravanaAparcada { caravana: *caravana }, vec![anfitriona], &[("cantidad", n), ("cargar", *cargar as i64)]);
                Ok(None)
            }
            Comando::EnviarCaravanaAlOrigen { caravana } => {
                let (i, origen) = self.caravana_de_orden(p, *caravana)?;
                let c = &self.estado.comercio.flota[i];
                if c.estado != EstadoCaravana::Aparcada {
                    return Err(de_caravana(ErrorCaravana::NoAparcada));
                }
                if !c.contenido.is_empty() {
                    return Err(rechazo(rechazos::CARAVANA_REQUIERE_VIAJE, "con carga la caravana vuelve viajando, y el movimiento llega en la fase 3B-2".into()));
                }
                let posicion = self.estado.asentamientos[&origen].posicion;
                self.estado.comercio.flota[i].enviar_al_origen(posicion, None).map_err(de_caravana)?;
                self.emitir_comercio(t, EventoComercio::CaravanaEnviadaAlOrigen { caravana: *caravana }, vec![origen], &[]);
                Ok(None)
            }
            Comando::EntregarDeCaravana { ejercito, caravana, acuerdo } => self.entregar_de_caravana(t, p, *ejercito, *caravana, *acuerdo),
            Comando::AdjuntarCaravana { caravana, ejercito } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                if !self.estado.ejercitos.contains_key(ejercito) {
                    return Err(rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{ejercito}")));
                }
                let i = self.estado.comercio.posicion_caravana(*caravana).ok_or_else(|| desconocida(*caravana))?;
                self.estado.comercio.flota[i].adjuntar().map_err(de_caravana)?;
                self.estado.comercio.adjuntas.insert(*caravana, *ejercito);
                Ok(None)
            }
            Comando::AparcarCaravana { caravana, asentamiento } => {
                if let Some(r) = solo_admin() {
                    return Err(r);
                }
                let i = self.estado.comercio.posicion_caravana(*caravana).ok_or_else(|| desconocida(*caravana))?;
                let anfitriona = self.plaza_o_rechazo(*asentamiento)?;
                let (faccion, posicion) = (anfitriona.faccion, anfitriona.posicion);
                let origen = self.estado.comercio.flota[i].origen as u64;
                if self.estado.asentamientos.get(&origen).map(|o| o.faccion) != Some(faccion) {
                    return Err(rechazo(rechazos::CARAVANA_PLAZA_AJENA, "una caravana solo se aparca en una plaza de su Facción".into()));
                }
                self.estado.comercio.flota[i].aparcar(id32(*asentamiento), posicion).map_err(de_caravana)?;
                self.estado.comercio.adjuntas.remove(caravana);
                Ok(None)
            }
            _ => unreachable!("solo se llama con órdenes de la flota"),
        }
    }

    /// `prepararCaravana` (§5.6) sin salida: valida con el crate sobre una copia, reserva la carga del almacén y deja la caravana `preparando`
    /// hasta `sale_en`. La ruta es la de `crates/rutas` (sin la red de caminos); los peajes de zonas ajenas llegan con la fase 3B-2.
    fn preparar_caravana(&mut self, t: Instante, p: &Peticion, caravana: u64, destino: u64, carga: &[(Recurso, Mil)], salir_en: Option<Instante>) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let cr = self.estado.reglas.economia.comercio.clone();
        let (i, origen_id) = self.caravana_de_orden(p, caravana)?;
        let posicion_destino = self.plaza_o_rechazo(destino)?.posicion;
        let origen = &self.estado.asentamientos[&origen_id];
        let (posicion_origen, cupos, factores) = (origen.posicion, self.cupos_de(origen), self.factores_de(origen));
        // La ruta por tierra: sin ella el crate rechaza con `SinRuta` (después de las demás validaciones).
        let camino = self.rutas.ruta_con(&mut self.escritorio, posicion_origen, posicion_destino, &Opciones::default());
        self.metricas.rutas_calculadas += 1;
        let ruta = camino.map(|c| if c.puntos.len() >= 2 { c.puntos } else { vec![posicion_origen, posicion_destino] });
        let mut carga_por_recurso = Mercancia::new();
        for &(r, n) in carga {
            *carga_por_recurso.entry(r).or_insert(0) += n;
        }
        let lanzamiento = Lanzamiento {
            destino: id32(destino),
            carga: carga_por_recurso,
            escolta_nueva: vec![],
            cupo_escolta: cupos.escolta,
            salir_en,
            factor_capacidad: factores.capacidad,
            ruta,
            peajes: vec![],
        };
        let mut copia = self.estado.comercio.flota[i].clone();
        let mut almacen = self.estado.asentamientos[&origen_id].economia.almacen.clone();
        let preparacion = copia.preparar(&cr.flota, lanzamiento, &mut almacen, t).map_err(|e| {
            let (c, m) = rechazo_de_caravana(e);
            rechazo(c, m.into())
        })?;
        // DUDA: una salida inmediata (un solo carro y sin `salir_en`) pasaría a `en_transito` sin que exista el movimiento: hasta la fase 3B-2 se
        // rechaza (con dos carros o una hora de salida programada sí se prepara).
        if preparacion.estado != EstadoCaravana::Preparando {
            return Err(rechazo(rechazos::CARAVANA_SALIDA_INMEDIATA, "la salida inmediata necesita el movimiento de las caravanas (fase 3B-2): usa dos carros o una hora de salida".into()));
        }
        self.estado.comercio.flota[i] = copia;
        self.estado.asentamientos.get_mut(&origen_id).expect("existe").economia.almacen = almacen;
        self.programar_comercio(preparacion.sale_en, entidad_caravana(caravana), SucesoComercio::FinPreparacion);
        self.tr.asentamientos.push(origen_id);
        self.emitir_comercio(t, EventoComercio::CaravanaPreparando { caravana }, vec![origen_id, destino], &[("saleEn", preparacion.sale_en)]);
        Ok(None)
    }

    /// Cancela la preparación de la caravana `i`: su carga vuelve al almacén del origen, topada por su capacidad. Devuelve lo que entró y lo que
    /// no cupo (que se pierde).
    fn cancelar_preparacion(&mut self, i: usize, origen: u64) -> Result<(Mil, Mil), ErrorCaravana> {
        let Estado { comercio, asentamientos, .. } = &mut self.estado;
        let almacen = &mut asentamientos.get_mut(&origen).expect("existe").economia.almacen;
        let llevaba = comercio.flota[i].contenido.clone();
        let antes: Mercancia = llevaba.keys().map(|&r| (r, almacen.cantidad(r))).collect();
        comercio.flota[i].cancelar(almacen)?;
        let devuelto: Mil = llevaba.keys().map(|&r| almacen.cantidad(r) - antes[&r]).sum();
        Ok((devuelto, total(&llevaba) - devuelto))
    }

    /// El plazo de preparación terminó. Solo se registra: la salida real (plan, peajes, `caravana_sale`) es de la fase 3B-2.
    pub(super) fn fin_de_preparacion(&mut self, id: u64, t: Instante) {
        let Some(c) = self.estado.comercio.caravana(id) else { return };
        if c.estado != EstadoCaravana::Preparando || c.prepara_hasta.is_none_or(|h| h > t) {
            return;
        }
        let origen = c.origen as u64;
        // DUDA: el evento no está en Comercio_Caravanas §12 (allí el fin de preparación emite `comercio.caravana_sale`, que exige el movimiento).
        self.emitir_propio(t, "comercio.preparacion_terminada", vec![origen], vec![("caravana", id as i64)]);
    }

    /// `entregarDeCaravana` (§4.5): una caravana adjunta a la columna entrega, a la puerta del que recibe, lo que le falta al trueque a la
    /// Facción de la columna. Cobra la misma comisión y el mismo recargo que una entrega de viaje y cierra por la misma vía.
    fn entregar_de_caravana(&mut self, t: Instante, p: &Peticion, ejercito: u64, caravana: u64, acuerdo: u64) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let e = self.estado.ejercitos.get(&ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, format!("ej-{ejercito}")))?;
        if !(p.admin || e.propietario == p.actor) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya".into()));
        }
        let (posicion_columna, faccion_columna) = (e.posicion_en(t), e.faccion);
        let ci = self.estado.comercio.posicion_caravana(caravana).ok_or_else(|| rechazo(rechazos::CARAVANA_DESCONOCIDA, format!("caravana-{caravana}")))?;
        let c = &self.estado.comercio.flota[ci];
        if c.estado != EstadoCaravana::Adjunta || self.estado.comercio.adjuntas.get(&caravana) != Some(&ejercito) {
            return Err(rechazo(rechazos::CARAVANA_NO_ESTA_ADJUNTA, "la caravana no va enganchada a esa columna".into()));
        }
        let ai = self.estado.comercio.posicion_acuerdo(acuerdo).ok_or_else(|| rechazo(rechazos::TRUEQUE_DESCONOCIDO, format!("trueque-{acuerdo}")))?;
        let ac = &self.estado.comercio.acuerdos[ai];
        if ac.estado != EstadoTrueque::Activo {
            return Err(rechazo(rechazos::TRUEQUE_NO_ACTIVO, "el trueque no está activo".into()));
        }
        let (a, b) = (ac.a as u64, ac.b as u64);
        let (faccion_a, faccion_b) = (self.plaza_o_rechazo(a)?.faccion, self.plaza_o_rechazo(b)?.faccion);
        let lado = ac.lado_que_debe(faccion_columna, faccion_a, faccion_b).ok_or_else(|| rechazo(rechazos::ENTREGA_NO_DEBE, "tu Facción no debe nada a ese trueque".into()))?;
        let (origen, destino) = (ac.origen_de(lado) as u64, ac.destino_de(lado) as u64);
        if posicion_columna.dist(self.plaza_o_rechazo(destino)?.posicion) > RADIO_PUERTA {
            return Err(rechazo(rechazos::ENTREGA_FUERA_DE_LA_PUERTA, "la columna no está en la puerta de la plaza que recibe".into()));
        }
        let entregable = ac.entregable(lado, &c.contenido);
        if entregable.is_empty() {
            return Err(rechazo(rechazos::ENTREGA_INVALIDA, "la caravana no lleva nada que el trueque pida".into()));
        }
        let c = &mut self.estado.comercio.flota[ci];
        for (&r, &n) in &entregable {
            let queda = c.contenido[&r] - n;
            if queda > 0 {
                c.contenido.insert(r, queda);
            } else {
                c.contenido.remove(&r);
            }
        }
        let entrega = self.liquidar_entrega(t, origen, destino, &entregable).expect("las dos plazas existen");
        self.emitir_comercio(
            t,
            EventoComercio::EntregaEscoltada { acuerdo, caravana },
            vec![origen, destino],
            &[("comision", entrega.comision), ("recargo", entrega.recargo), ("perdido", total(&entrega.perdido))],
        );
        self.abonar_a_trueque(t, acuerdo, lado, &entregable);
        Ok(None)
    }

    /// Entrega todo lo que lleva la caravana en la plaza `destino` como si hubiera llegado (§4.4): cada recurso topado por la capacidad del
    /// destino, la comisión acuñada, el recargo por mala reputación y, con `vinculo`, el abono al `lado` del trueque (que no mira su estado:
    /// rareza (a) de §15). Es el gancho que usará el evento de llegada de la fase 3B-2 y con el que las pruebas hacen llegar una caravana de
    /// pruebas; no deja entrada en el registro (si se usa fuera de una orden o un evento registrado, la recuperación no la repite).
    /// `None` si la caravana o las plazas no existen. Procesa los eventos hasta `t` y cierra la versión.
    pub fn entregar_contenido(&mut self, t: Instante, caravana: u64, destino: u64, vinculo: Option<(u64, Lado)>) -> Option<comercio::entrega::ResultadoEntrega> {
        let t = t.max(self.estado.ahora);
        self.procesar_hasta(t);
        let i = self.estado.comercio.posicion_caravana(caravana)?;
        let origen = self.estado.comercio.flota[i].origen as u64;
        let contenido = std::mem::take(&mut self.estado.comercio.flota[i].contenido);
        let Some(entrega) = self.liquidar_entrega(t, origen, destino, &contenido) else {
            self.estado.comercio.flota[i].contenido = contenido;
            return None;
        };
        if let Some((acuerdo, lado)) = vinculo {
            self.abonar_a_trueque(t, acuerdo, lado, &contenido);
        }
        self.emitir_comercio(t, EventoComercio::CaravanaLlega { caravana, destino: id32(destino), comision: entrega.comision, recargo: entrega.recargo }, vec![origen, destino], &[]);
        self.cerrar_version(t, true);
        Some(entrega)
    }

    // ───────────────────────────── Ruina y conquista ─────────────────────────────

    /// La cascada de comercio de una plaza que desaparece (`Motivo::Ruina`, tras sacarla del estado) o cambia de dueño (`Motivo::Conquista`,
    /// con la plaza ya en manos del conquistador): sus trueques vivos se anulan sin penalizar (`trueque_cancelado` a la socia), las caravanas
    /// se pierden, vuelven, regresan o se destruyen según §10 y, en la ruina, sus órdenes de mercado se borran. Sin cerrar versión: lo hace
    /// quien la llama (el evento de ruina o la orden de conquista).
    ///
    /// DUDA: una caravana aparcada en la plaza conquistada y con carga debería volver viajando (fase 3B-2); hasta entonces se queda aparcada.
    /// La que `regresa` (en camino hacia la plaza) no puede existir mientras nada se mueva: si apareciera, se pierde.
    pub(super) fn cerrar_comercio(&mut self, plaza: u64, motivo: Motivo, t: Instante) {
        let pid = id32(plaza);
        let cierre = {
            let Estado { comercio, asentamientos, .. } = &mut self.estado;
            let existe = |o: u32| asentamientos.contains_key(&(o as u64));
            cerrar_dependientes(motivo, pid, &mut comercio.acuerdos, &comercio.flota, &existe, &|_| true)
        };
        for TruequeCancelado { acuerdo, socia } in cierre.trueques {
            self.invalidar_comercio(entidad_trueque(acuerdo));
            self.emitir_comercio(t, EventoComercio::TruequeCancelado { acuerdo, socia }, vec![socia as u64], &[]);
        }
        for AccionCaravana { caravana, accion, .. } in cierre.caravanas {
            match accion {
                Accion::Pierde | Accion::Destruye | Accion::Regresa => self.perder_caravana(t, caravana),
                Accion::Vuelve => self.hacer_volver_caravana(t, caravana),
                Accion::EnviaAlOrigen => self.enviar_caravana_a_casa(t, caravana),
            }
        }
        match motivo {
            Motivo::Ruina => {
                let libro = &mut self.estado.comercio.libro;
                let activas: Vec<u64> = libro.ordenes.iter().filter(|o| o.asentamiento == pid).map(|o| o.id).collect();
                libro.ordenes.retain(|o| o.asentamiento != pid);
                libro.historial.retain(|o| o.asentamiento != pid);
                for orden in activas {
                    self.invalidar_comercio(crate::estado_comercio::entidad_orden(orden));
                }
                self.estado.comercio.ultima_caravana.remove(&plaza);
            }
            Motivo::Conquista => {
                if !cierre.flota_traspasada.is_empty() {
                    let n = cierre.flota_traspasada.len() as i64;
                    self.emitir_propio(t, "comercio.flota_traspasada", vec![plaza], vec![("caravanas", n)]);
                }
            }
        }
    }

    /// La plaza `plaza` cambió de dueño (la llama el sistema militar, una vez cambiada su Facción): su flota pasa con ella, los trueques con
    /// la plaza se anulan sin penalizar y las caravanas que iban hacia ella regresan. Procesa los eventos hasta `t` y cierra la versión.
    /// Dentro de una orden o un evento que ya se registra, usa la ruta interna (`cerrar_comercio`) en vez de esta. `false` si la plaza no existe.
    pub fn cerrar_comercio_por_conquista(&mut self, t: Instante, plaza: u64) -> bool {
        let t = t.max(self.estado.ahora);
        self.procesar_hasta(t);
        if !self.estado.asentamientos.contains_key(&plaza) {
            return false;
        }
        self.cerrar_comercio(plaza, Motivo::Conquista, t);
        self.cerrar_version(t, true);
        true
    }

    /// Quita la caravana del mundo (se pierde con su carga) y avisa a la plaza de su origen, si existe.
    fn perder_caravana(&mut self, t: Instante, id: u64) {
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return };
        let c = self.estado.comercio.flota.remove(i);
        self.estado.comercio.adjuntas.remove(&id);
        self.invalidar_comercio(entidad_caravana(id));
        let origen = c.origen as u64;
        let plazas = if self.estado.asentamientos.contains_key(&origen) { vec![origen] } else { vec![] };
        self.emitir_comercio(t, EventoComercio::CaravanaPerdida { caravana: id }, plazas, &[("carga", total(&c.contenido))]);
    }

    /// Una `preparando` hacia la plaza que cae: la carga vuelve al almacén del origen (topada por su capacidad) y queda `disponible`.
    fn hacer_volver_caravana(&mut self, t: Instante, id: u64) {
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return };
        let origen = self.estado.comercio.flota[i].origen as u64;
        if !self.estado.asentamientos.contains_key(&origen) {
            return self.perder_caravana(t, id);
        }
        let Ok((devuelto, perdido)) = self.cancelar_preparacion(i, origen) else { return };
        self.invalidar_comercio(entidad_caravana(id));
        self.tr.asentamientos.push(origen);
        self.emitir_comercio(t, EventoComercio::CaravanaVuelve { caravana: id }, vec![origen], &[("devuelto", devuelto), ("perdido", perdido)]);
    }

    /// Una aparcada en la plaza conquistada vuelve a su origen (§5.7): vacía, al instante.
    fn enviar_caravana_a_casa(&mut self, t: Instante, id: u64) {
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return };
        let origen = self.estado.comercio.flota[i].origen as u64;
        let Some(posicion) = self.estado.asentamientos.get(&origen).map(|o| o.posicion) else { return self.perder_caravana(t, id) };
        if self.estado.comercio.flota[i].enviar_al_origen(posicion, None).is_ok() {
            self.emitir_comercio(t, EventoComercio::CaravanaEnviadaAlOrigen { caravana: id }, vec![origen], &[]);
        }
    }
}
