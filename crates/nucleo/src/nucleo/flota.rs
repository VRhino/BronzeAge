//! La flota de caravanas dentro del núcleo (Comercio_Caravanas §5, §10): crearlas, componerlas, lanzarlas a mano (sale ya o queda
//! `preparando` hasta su hora), la aparcada y la adjunta, la entrega manual de una adjunta, y el cierre del comercio de una plaza que cae en
//! ruinas o cambia de dueño. Las reglas puras viven en el crate `comercio`; el viaje (trazado, plan, peajes, llegada, reparto y red de
//! caminos) está en [`viaje`](super::viaje).
//!
//! # Lo que dispara el reparto
//! Una caravana que queda libre (se cancela su preparación, se le monta el primer animal, se libera de la reserva, vuelve vacía a su origen o
//! llega a casa) hace que su plaza reevalúe el reparto a los trueques activos (`Nucleo::reparto_de_plaza`).

use super::*;
use crate::estado::SucesoComercio;
use crate::estado_comercio::{entidad_caravana, entidad_trueque};
use crate::nucleo::mercado::{id32, nivel_mercado};
use asentamiento::edificio::Edificios;
use asentamiento::Recurso;
use comercio::caravana::{caravanas_comerciales_de, carros_en_uso, mover_carro, Caravana, ContextoCarro, ContextoCrear, ErrorCaravana, EstadoCaravana, Lanzamiento, TipoCaravana};
use comercio::evento::Evento as EventoComercio;
use comercio::ruina::{cerrar_dependientes, Accion, AccionCaravana, Motivo, TruequeCancelado};
use comercio::trueque::EstadoTrueque;
use comercio::{total, Carga as Mercancia, Lado};

pub(super) fn rechazo_de_caravana(e: ErrorCaravana) -> (&'static str, &'static str) {
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

    /// La caravana, su posición en la flota y la plaza de su origen, con la orden autorizada (Proyecciones §5.2.3-§5.2.4): quien reside en el origen y está dentro (`resideEnOrigen`) o, con la
    /// caravana aparcada, quien reside en el origen y está dentro de la plaza donde está aparcada (`puedeOperarAparcada`).
    fn caravana_de_orden(&self, p: &Peticion, id: u64) -> Result<(usize, u64), Respuesta> {
        let v = self.estado.version;
        let i = self.estado.comercio.posicion_caravana(id).ok_or_else(|| Respuesta::rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(id), v))?;
        let origen = self.estado.comercio.flota[i].origen as u64;
        self.plaza_o_rechazo(origen)?;
        if p.comando.autoriza() == Autoriza::OperaAparcada {
            // Regla (decisión del autor, 2026-10-25, 5E-5): sin aparcar (enviar al origen la que no lo está la rechaza la orden), la plaza donde hay que estar es su origen.
            let donde = self.estado.comercio.flota[i].anfitriona.map_or(origen, u64::from);
            self.autorizar_aparcada(p, origen, donde)?;
        } else {
            self.autorizar_en_plaza(p, origen, None)?;
        }
        Ok((i, origen))
    }

    /// Las órdenes de la flota. `Ok(Some(n))` = cambió algo y creó `n`; `Ok(None)` = cambió algo; rechazos y «sin cambios» salen como `Err` sin
    /// haber tocado el estado.
    pub(super) fn ejecutar_flota(&mut self, t: Instante, p: &Peticion) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let sin_cambios = || Respuesta::sin_cambios(v);
        let cr = self.estado.reglas.economia.comercio.clone();
        let de_caravana = |e: ErrorCaravana| {
            let (c, m) = rechazo_de_caravana(e);
            rechazo(c, m.into())
        };
        let desconocida = |id: u64| rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(id));
        match &p.comando {
            Comando::CrearCaravana { asentamiento } => {
                self.plaza_o_rechazo(*asentamiento)?;
                self.autorizar_en_plaza(p, *asentamiento, None)?;
                let plaza = &self.estado.asentamientos[asentamiento];
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
                self.tr.tocar(Clase::Asentamiento, origen);
                self.emitir_comercio(t, EventoComercio::CaravanaCarroAgregado { caravana: *caravana }, vec![origen], &[]);
                Ok(None)
            }
            Comando::AnadirAnimal { caravana, carro, animal } => {
                let (i, origen) = self.caravana_de_orden(p, *caravana)?;
                let Estado { comercio, asentamientos, .. } = &mut self.estado;
                let almacen = &mut asentamientos.get_mut(&origen).expect("existe").economia.almacen;
                comercio.flota[i].comprar_animal(&cr.flota, *carro as usize, *animal, almacen).map_err(de_caravana)?;
                self.tr.tocar(Clase::Asentamiento, origen);
                self.emitir_comercio(t, EventoComercio::CaravanaAnimalComprado { caravana: *caravana }, vec![origen], &[]);
                self.contar(t, tecnologia::contador::Delta::uno(tecnologia::Contador::AnimalesComprados));
                self.reparto_de_plaza(origen, t);
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
                self.reparto_de_plaza(origen, t);
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
                if !*reservada {
                    self.reparto_de_plaza(origen, t);
                }
                Ok(None)
            }
            Comando::PrepararCaravana { caravana, destino, carga, salir_en } => self.preparar_caravana(t, p, *caravana, *destino, carga, *salir_en),
            Comando::CancelarCaravana { caravana } => {
                let (i, origen) = self.caravana_de_orden(p, *caravana)?;
                let (devuelto, perdido) = self.cancelar_preparacion(i, origen).map_err(de_caravana)?;
                self.invalidar_comercio(entidad_caravana(*caravana));
                self.tr.tocar(Clase::Asentamiento, origen);
                self.emitir_comercio(t, EventoComercio::CaravanaCancelada { caravana: *caravana }, vec![origen], &[("devuelto", devuelto), ("perdido", perdido)]);
                self.reparto_de_plaza(origen, t);
                Ok(None)
            }
            Comando::MoverCargaAparcada { caravana, recurso, cantidad, cargar } => {
                let (i, origen) = self.caravana_de_orden(p, *caravana)?;
                let c = &self.estado.comercio.flota[i];
                if c.estado != EstadoCaravana::Aparcada {
                    return Err(de_caravana(ErrorCaravana::NoAparcada));
                }
                let Some(anfitriona) = c.anfitriona.map(u64::from) else { return Err(de_caravana(ErrorCaravana::NoAparcada)) };
                self.plaza_o_rechazo(anfitriona)?;
                let factor = self.factores_de(&self.estado.asentamientos[&origen]).capacidad;
                let Estado { comercio, asentamientos, .. } = &mut self.estado;
                let almacen = &mut asentamientos.get_mut(&anfitriona).expect("existe").economia.almacen;
                let c = &mut comercio.flota[i];
                let movido = if *cargar { c.cargar_aparcada(&cr.flota, factor, almacen, *recurso, *cantidad) } else { c.descargar_aparcada(almacen, *recurso, *cantidad) };
                let n = movido.map_err(de_caravana)?;
                self.tr.tocar(Clase::Asentamiento, anfitriona);
                self.emitir_comercio(t, EventoComercio::CargaCaravanaAparcada { caravana: *caravana }, vec![anfitriona], &[("cantidad", n), ("cargar", *cargar as i64)]);
                Ok(None)
            }
            Comando::EnviarCaravanaAlOrigen { caravana } => {
                let (i, origen) = self.caravana_de_orden(p, *caravana)?;
                let c = &self.estado.comercio.flota[i];
                if c.estado != EstadoCaravana::Aparcada {
                    return Err(de_caravana(ErrorCaravana::NoAparcada));
                }
                let posicion = self.estado.asentamientos[&origen].posicion;
                if c.contenido.is_empty() {
                    // Vacía: queda `disponible` en su origen al instante, sin viaje.
                    self.estado.comercio.flota[i].enviar_al_origen(posicion, None).map_err(de_caravana)?;
                    self.emitir_comercio(t, EventoComercio::CaravanaEnviadaAlOrigen { caravana: *caravana }, vec![origen], &[]);
                    self.reparto_de_plaza(origen, t);
                    return Ok(None);
                }
                // Con carga: vuelve viajando desde la plaza que la hospeda, sin atracción de la red ni peajes, y vuelca lo que lleva al llegar.
                let anfitriona = c.anfitriona.map(u64::from).and_then(|a| self.estado.asentamientos.get(&a)).map(|a| a.posicion).ok_or_else(|| de_caravana(ErrorCaravana::NoAparcada))?;
                let Some((plan, camino)) = self.plan_de_regreso(*caravana, anfitriona, t) else { return Err(de_caravana(ErrorCaravana::SinRuta)) };
                self.estado.comercio.flota[i].enviar_al_origen(posicion, Some(plan.clone())).map_err(de_caravana)?;
                let c = &mut self.estado.comercio.flota[i];
                (c.ruta, c.costes, c.posicion) = (camino.puntos, camino.costes, anfitriona);
                self.instalar_plan(*caravana, plan);
                self.emitir_comercio(t, EventoComercio::CaravanaEnviadaAlOrigen { caravana: *caravana }, vec![origen], &[]);
                Ok(None)
            }
            Comando::EntregarDeCaravana { ejercito, caravana, acuerdo } => self.entregar_de_caravana(t, p, *ejercito, *caravana, *acuerdo),
            Comando::AdjuntarCaravana { caravana, ejercito } => self.adjuntar_caravana(t, p, *caravana, *ejercito),
            Comando::AparcarCaravana { caravana, asentamiento } => self.aparcar_caravana(t, p, *caravana, *asentamiento, "ejercito.caravana_aparcada", &[]),
            _ => unreachable!("solo se llama con órdenes de la flota"),
        }
    }

    /// `prepararCaravana` (§5.6): valida con el crate sobre una copia, reserva la carga del almacén y deja la caravana `preparando` hasta
    /// `sale_en`, o la hace salir ya si no hay espera (un solo carro y sin `salir_en`). La ruta es el trazado de §7.2 (zonas ajenas con peaje y
    /// atracción de la red); se registra en la red aunque luego se cancele.
    fn preparar_caravana(&mut self, t: Instante, p: &Peticion, caravana: u64, destino: u64, carga: &[(Recurso, Mil)], salir_en: Option<Instante>) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let cr = self.estado.reglas.economia.comercio.clone();
        let (i, origen_id) = self.caravana_de_orden(p, caravana)?;
        self.plaza_o_rechazo(destino)?;
        let origen = &self.estado.asentamientos[&origen_id];
        let (cupos, factores) = (self.cupos_de(origen), self.factores_de(origen));
        // La ruta por tierra: sin ella el crate rechaza con `SinRuta` (después de las demás validaciones).
        let trazado = self.trazar_ruta_comercial(origen_id, destino, t);
        let peajes = trazado.as_ref().map(|z| comercio::viaje::peajes_de_paradas(&z.paradas, comercio::viaje::longitud_ruta(&z.puntos))).unwrap_or_default();
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
            ruta: trazado.as_ref().map(|z| z.puntos.clone()),
            peajes,
        };
        let mut copia = self.estado.comercio.flota[i].clone();
        let mut almacen = self.estado.asentamientos[&origen_id].economia.almacen.clone();
        let preparacion = copia.preparar(&cr.flota, lanzamiento, &mut almacen, t).map_err(|e| {
            let (c, m) = rechazo_de_caravana(e);
            rechazo(c, m.into())
        })?;
        let trazado = trazado.expect("`preparar` exige una ruta");
        copia.costes = trazado.costes.clone();
        self.estado.comercio.flota[i] = copia;
        self.estado.asentamientos.get_mut(&origen_id).expect("existe").economia.almacen = almacen;
        self.tr.tocar(Clase::Asentamiento, origen_id);
        if preparacion.estado == EstadoCaravana::Preparando {
            self.programar_comercio(preparacion.sale_en, entidad_caravana(caravana), SucesoComercio::FinPreparacion);
        } else {
            // Salida inmediata: sin `caravana_sale`, su plan empieza en este instante.
            let plan = self.plan_de(&self.estado.comercio.flota[i], &trazado.puntos, &trazado.costes, t);
            self.instalar_plan(caravana, plan);
        }
        self.emitir_comercio(t, EventoComercio::CaravanaPreparando { caravana }, vec![origen_id, destino], &[("saleEn", preparacion.sale_en)]);
        // La ruta entra en la red ya, aunque luego se cancele; la propia caravana no se rehace por ella.
        self.registrar_ruta_en_red(origen_id, destino, &trazado.puntos, t, &[caravana]);
        Ok(None)
    }

    /// Cancela la preparación de la caravana `i`: su carga vuelve al almacén del origen, topada por su capacidad. Devuelve lo que entró y lo que
    /// no cupo (que se pierde).
    fn cancelar_preparacion(&mut self, i: usize, origen: u64) -> Result<(Mil, Mil), ErrorCaravana> {
        let Estado { comercio, asentamientos, .. } = &mut self.estado;
        let almacen = &mut asentamientos.get_mut(&origen).expect("existe").economia.almacen;
        let llevaba = comercio.flota[i].contenido.clone();
        let antes: Mercancia = llevaba.keys().map(|&r| (r, almacen.cantidad(r))).collect();
        let escolta = comercio.flota[i].cancelar(almacen)?;
        let devuelto: Mil = llevaba.keys().map(|&r| almacen.cantidad(r) - antes[&r]).sum();
        self.devolver_escolta(&escolta);
        Ok((devuelto, total(&llevaba) - devuelto))
    }

    /// `entregarDeCaravana` (§4.5): una caravana adjunta a la columna entrega, a la puerta del que recibe, lo que le falta al trueque a la
    /// Facción de la columna. Cobra la misma comisión y el mismo recargo que una entrega de viaje y cierra por la misma vía.
    fn entregar_de_caravana(&mut self, t: Instante, p: &Peticion, ejercito: u64, caravana: u64, acuerdo: u64) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let e = self.estado.ejercitos.get(&ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(ejercito)))?;
        if !self.manda_columna(p, e, false) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, "la columna no es tuya".into()));
        }
        let (posicion_columna, faccion_columna) = (e.posicion_en(t), e.bandera());
        let ci = self.estado.comercio.posicion_caravana(caravana).ok_or_else(|| rechazo(rechazos::CARAVANA_DESCONOCIDA, ids::caravana(caravana)))?;
        let c = &self.estado.comercio.flota[ci];
        if c.estado != EstadoCaravana::Adjunta || !e.caravanas_adjuntas.contains(&caravana) {
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
        if posicion_columna.dist(self.plaza_o_rechazo(destino)?.posicion) > self.radio_puerta() {
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
    /// rareza (a) de §15). Es el gancho con el que las pruebas hacen llegar una caravana sin esperar al viaje (el evento de llegada
    /// real, `llegada_de_caravana`, usa las mismas piezas); no deja entrada en el registro (si se usa fuera de una orden o un evento registrado, la recuperación no la repite).
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
        self.tr.tocar(Clase::Caravana, caravana);
        if let Some((acuerdo, lado)) = vinculo {
            self.abonar_a_trueque(t, acuerdo, lado, &contenido);
        }
        let comercial = self.estado.comercio.flota[i].tipo == TipoCaravana::Comercial;
        self.hecho_de_caravana(caravana, comercial, origen, total(&contenido));
        self.emitir_comercio(t, EventoComercio::CaravanaLlega { caravana, destino: id32(destino), comision: entrega.comision, recargo: entrega.recargo }, vec![origen, destino], &[]);
        self.cerrar_version(t, true);
        Some(entrega)
    }

    // ───────────────────────────── Ruina y conquista ─────────────────────────────

    /// La cascada de comercio de una plaza que desaparece (`Motivo::Ruina`, tras sacarla del estado) o cambia de dueño (`Motivo::Conquista`,
    /// con la plaza ya en manos del conquistador): sus trueques vivos se anulan sin penalizar (`trueque_cancelado` a la socia), las caravanas
    /// se pierden, vuelven, regresan o se destruyen según §10 y, en la ruina, sus órdenes de mercado se borran. Las que iban hacia la plaza
    /// dan la vuelta con su carga desde donde están («las órdenes parten de la posición real»). Sin cerrar versión: lo hace quien la llama
    /// (el evento de ruina o la orden de conquista).
    ///
    /// La caravana aparcada en la plaza que cae se destruye, sea por ruina o por conquista (Doc 3.13.7, decidido el 2026-10-19).
    pub(super) fn cerrar_comercio(&mut self, plaza: u64, motivo: Motivo, t: Instante) {
        // La Caravana de Fundación suelta en la plaza cae con ella (5G-b-11).
        self.destruir_fundaciones_en(plaza, t);
        let pid = id32(plaza);
        // Los caminos de regreso se calculan antes (el A* necesita `&mut self`): una caravana en camino hacia la plaza solo regresa si lo hay.
        let en_camino: Vec<(u64, Punto, Punto)> = self
            .estado
            .comercio
            .flota
            .iter()
            .filter(|c| c.estado == EstadoCaravana::EnTransito && c.destino == Some(pid) && c.origen != pid)
            .filter_map(|c| Some((c.id, c.posicion_en(t), self.estado.asentamientos.get(&(c.origen as u64))?.posicion)))
            .collect();
        let mut caminos = std::collections::BTreeSet::new();
        for (id, desde, hasta) in en_camino {
            if self.camino_sin_red(desde, hasta).is_some() {
                caminos.insert(id);
            }
        }
        let cierre = {
            let Estado { comercio, asentamientos, .. } = &mut self.estado;
            let existe = |o: u32| asentamientos.contains_key(&(o as u64));
            cerrar_dependientes(motivo, pid, &mut comercio.acuerdos, &comercio.flota, &existe, &|c| caminos.contains(&c.id))
        };
        for TruequeCancelado { acuerdo, socia } in cierre.trueques {
            self.tr.tocar(Clase::Acuerdo, acuerdo);
            self.invalidar_comercio(entidad_trueque(acuerdo));
            self.emitir_comercio(t, EventoComercio::TruequeCancelado { acuerdo, socia }, vec![socia as u64], &[]);
        }
        for AccionCaravana { caravana, accion, .. } in cierre.caravanas {
            match accion {
                Accion::Pierde | Accion::Destruye => self.perder_caravana_de_ruina(t, caravana, plaza),
                Accion::Regresa => self.regresar_caravana(t, caravana, pid),
                Accion::Vuelve => self.hacer_volver_caravana(t, caravana),
            }
        }
        self.invalidar_red();
        match motivo {
            Motivo::Ruina => {
                let libro = &mut self.estado.comercio.libro;
                let activas: Vec<u64> = libro.ordenes.iter().filter(|o| o.asentamiento == pid).map(|o| o.id).collect();
                let borradas: Vec<u64> = libro.ordenes.iter().chain(&libro.historial).filter(|o| o.asentamiento == pid).map(|o| o.id).collect();
                libro.ordenes.retain(|o| o.asentamiento != pid);
                libro.historial.retain(|o| o.asentamiento != pid);
                for orden in activas {
                    self.invalidar_comercio(crate::estado_comercio::entidad_orden(orden));
                }
                self.tr.tocar_todos(Clase::OrdenMercado, borradas);
                self.estado.comercio.ultima_caravana.remove(&plaza);
                self.estado.comercio.reparto.remove(&plaza);
            }
            Motivo::Conquista => {
                if !cierre.flota_traspasada.is_empty() {
                    self.tr.tocar_todos(Clase::Caravana, cierre.flota_traspasada.iter().map(|&c| c as u64));
                    let n = cierre.flota_traspasada.len() as i64;
                    self.emitir_propio(t, "comercio.flota_traspasada", vec![plaza], vec![("caravanas", n)]);
                }
            }
        }
    }

    /// Quita la caravana del mundo (se pierde con su carga) y avisa a la plaza de su origen, si existe. Si iba adjunta, devuelve lo que su columna ya no puede llevar sin
    /// ella, que se pierde con sus carros (M10).
    pub(super) fn perder_caravana(&mut self, t: Instante, id: u64) -> Mercancia {
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return Mercancia::new() };
        let c = self.estado.comercio.flota.remove(i);
        self.tr.tocar(Clase::Caravana, id);
        self.devolver_escolta(&c.escolta);
        let de_la_columna = self.soltar_de_columnas(id);
        self.retirar_plan(id);
        let origen = c.origen as u64;
        if c.tipo == TipoCaravana::Acarreo {
            self.pueblo_sin_carro(origen, id);
        }
        let plazas = if self.estado.asentamientos.contains_key(&origen) { vec![origen] } else { vec![] };
        self.emitir_comercio(t, EventoComercio::CaravanaPerdida { caravana: id }, plazas, &[("carga", total(&c.contenido))]);
        de_la_columna
    }

    /// Pierde una caravana en la cascada de una plaza. Si iba adjunta a una columna (su origen cayó en ruinas, P7 del 2026-10-25), se pierde con su carga y con lo que
    /// la columna ya no puede llevar sin sus carros (M10), y la columna lo oye con `ejercito.caravanas_perdidas` (con `ruina`, la plaza arruinada).
    fn perder_caravana_de_ruina(&mut self, t: Instante, caravana: u64, plaza: u64) {
        let columna = self.estado.ejercitos.iter().find(|(_, e)| e.caravanas_adjuntas.contains(&caravana)).map(|(&id, _)| id);
        let Some(c) = self.estado.comercio.caravana(caravana) else { return };
        let (carros, mut carga) = (c.carros.len() as i64, c.contenido.clone());
        for (r, n) in self.perder_caravana(t, caravana) {
            comercio::sumar(&mut carga, r, n);
        }
        if let Some(columna) = columna {
            let datos = vec![("caravanas", 1), ("carros", carros), ("carga", total(&carga)), ("ruina", plaza as i64)];
            self.evento_de_columna("ejercito.caravanas_perdidas", t, columna, vec![], datos);
        }
    }

    /// Una `preparando` hacia la plaza que cae: la carga vuelve al almacén del origen (topada por su capacidad) y queda `disponible`.
    fn hacer_volver_caravana(&mut self, t: Instante, id: u64) {
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return };
        let origen = self.estado.comercio.flota[i].origen as u64;
        if !self.estado.asentamientos.contains_key(&origen) {
            self.perder_caravana(t, id);
            return;
        }
        let Ok((devuelto, perdido)) = self.cancelar_preparacion(i, origen) else { return };
        self.tr.tocar(Clase::Caravana, id);
        self.invalidar_comercio(entidad_caravana(id));
        self.tr.tocar(Clase::Asentamiento, origen);
        self.emitir_comercio(t, EventoComercio::CaravanaVuelve { caravana: id }, vec![origen], &[("devuelto", devuelto), ("perdido", perdido)]);
        self.reparto_de_plaza(origen, t);
    }

    /// Una en camino hacia la plaza que cae: da la vuelta con su carga por un plan nuevo desde donde está (sin atracción de la red ni peajes)
    /// y la vuelca en su origen al llegar. Sin camino por tierra se pierde.
    fn regresar_caravana(&mut self, t: Instante, id: u64, ruina: u32) {
        let Some(c) = self.estado.comercio.caravana(id) else { return };
        let (aqui, origen) = (c.posicion_en(t), c.origen as u64);
        let Some((plan, camino)) = self.plan_de_regreso(id, aqui, t) else { return drop(self.perder_caravana(t, id)) };
        let i = self.estado.comercio.posicion_caravana(id).expect("existe");
        let c = &mut self.estado.comercio.flota[i];
        if c.regresar_a_origen(ruina, plan.clone()).is_err() {
            return;
        }
        self.tr.tocar(Clase::Caravana, id);
        (c.ruta, c.costes, c.posicion) = (camino.puntos, camino.costes, aqui);
        let carga = total(&c.contenido);
        self.instalar_plan(id, plan);
        self.emitir_comercio(t, EventoComercio::CaravanaVuelve { caravana: id }, vec![origen], &[("regresa", 1), ("carga", carga)]);
    }
}
