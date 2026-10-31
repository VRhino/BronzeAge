//! Los encargos de caravana en el núcleo (Comercio_Caravanas §5.8; `Rutas_De_Caravanas.md` §3 y §4): rutas y envíos automáticos entre una plaza propia y otra plaza.
//!
//! # Las órdenes
//! `EncargarCaravana`, `CambiarEncargo` y `CancelarEncargo` validan todo sobre una copia antes de tocar nada. Un encargo no guarda su ruta: se traza al salir de cada punta.
//!
//! # El ciclo
//! En cada punta la caravana llega, deja, recoge y sale. Cada paso con espera (la hora de salida de un envío, la descarga y la carga) es un
//! [`SucesoComercio::FinPasoEncargo`] de la caravana; la espera de existencias es un [`SucesoComercio::PulsoEncargo`] cada minuto, solo mientras falte algo. Todos van ligados a la
//! generación de la entidad de la caravana: salir, terminar o reprogramar los descarta. Las reglas de cuánto deja, cuánto recoge y cuántos carros cuentan son puras y viven en
//! `comercio::encargo`; aquí se leen, se aplican, se programan y se emiten.
//!
//! # El comercio en plaza ajena
//! Un envío a una plaza de otra Facción con Mercado activo comercia con las órdenes de esa plaza (rutas de caravanas §6): la descarga es vender y la carga, comprar, con los mismos pasos
//! y tiempos que en una plaza propia. Sin Mercado activo da la vuelta con lo que lleva. Si es propia o ajena, y si tiene Mercado, se mira al empezar cada paso y no se guarda.

use super::*;
use crate::estado::SucesoComercio;
use crate::estado_comercio::entidad_caravana;
use crate::nucleo::flota::rechazo_de_caravana;
use crate::estado_comercio::EstadoComercio;
use crate::nucleo::mercado::{id32, nivel_mercado};
use base::MS_POR_MINUTO;
use comercio::caravana::{Caravana, ErrorCaravana};
use comercio::encargo::{a_recoger, carros_del_paso, repartir_descarga, ContextoEncargo, Encargo, ErrorEncargo, Fase, Modo, Parada, Punta};
use comercio::evento::{Evento as EventoComercio, MotivoEncargo};
use comercio::mostrador::{comprar, vender, ContextoParada};
use comercio::viaje::{longitud_ruta, peajes_de_paradas};
use comercio::{total, Carga as Mercancia};

/// El código y el mensaje de rechazo de un [`ErrorEncargo`] (rutas de caravanas §7): la hora de salida inválida es la misma de lanzar a mano.
fn rechazo_de_encargo(e: ErrorEncargo) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorEncargo::MismoOrigen => (ENCARGO_MISMO_ORIGEN, "el destino del encargo es el origen de la caravana"),
        ErrorEncargo::DestinoAjeno => (ENCARGO_DESTINO_AJENO, "una ruta solo va a una plaza de tu Facción; a una ajena va un envío"),
        ErrorEncargo::SalidaInvalida => rechazo_de_caravana(ErrorCaravana::SalidaInvalida),
        ErrorEncargo::LineaInvalida => (ENCARGO_LINEA_INVALIDA, "una línea del encargo tiene menos de una unidad, repite un recurso, lleva precio en una plaza propia u oro en una ajena"),
        ErrorEncargo::FaltaPrecio => (ENCARGO_FALTA_PRECIO, "toda línea hacia una plaza ajena necesita un precio positivo"),
        ErrorEncargo::SuperaCapacidad => (ENCARGO_SUPERA_CAPACIDAD, "lo que se recoge en una parada pasa de la capacidad de la caravana"),
    }
}

/// La plaza de la parada en que está la caravana: su origen o su destino según la punta.
fn plaza_de_parada(c: &Caravana) -> Option<u64> {
    let e = c.encargo.as_ref()?;
    Some(u64::from(if e.punta == Punta::Origen { c.origen } else { e.destino }))
}

/// El rechazo de un [`ErrorCaravana`] sobre una copia de la caravana.
fn rechazo_en_caravana(e: ErrorCaravana, version: u64) -> Respuesta {
    let (codigo, mensaje) = rechazo_de_caravana(e);
    Respuesta::rechazo(codigo, mensaje, version)
}

impl Nucleo {
    /// Las órdenes de encargo (`EncargarCaravana`, `CambiarEncargo`, `CancelarEncargo`). `Ok(())` = cambió algo; rechazos y «sin cambios»
    /// salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_encargos(&mut self, t: Instante, p: &Peticion) -> Result<(), Respuesta> {
        match &p.comando {
            Comando::EncargarCaravana { caravana, destino, modo, salir_en, en_origen, en_destino } => {
                self.encargar(t, p, *caravana, Encargo::nuevo(*modo, id32(*destino), *salir_en, en_origen.clone(), en_destino.clone()))
            }
            Comando::CambiarEncargo { caravana, destino, salir_en, en_origen, en_destino } => {
                self.cambiar_encargo(t, p, *caravana, *destino, *salir_en, [en_origen.clone(), en_destino.clone()])
            }
            Comando::CancelarEncargo { caravana } => self.cancelar_encargo(t, p, *caravana),
            _ => unreachable!("solo se llama con órdenes de encargo"),
        }
    }

    /// Rutas de caravanas §2 y §7: valida `nuevo` como encargo de la caravana `i` (de la plaza `origen`) y lo compara con las dos plazas.
    fn validar_encargo(&self, nuevo: &Encargo, i: usize, origen: u64, t: Instante) -> Result<(), Respuesta> {
        let (o, d) = (&self.estado.asentamientos[&origen], &self.estado.asentamientos[&u64::from(nuevo.destino)]);
        let contexto = ContextoEncargo {
            origen: id32(origen),
            destino_propio: o.faccion == d.faccion,
            capacidad: self.capacidad_de_caravana(&self.estado.comercio.flota[i]),
            ahora: t,
            max_programacion: self.estado.reglas.economia.comercio.flota.max_programacion,
        };
        nuevo.validar(&contexto).map_err(|e| {
            let (codigo, mensaje) = rechazo_de_encargo(e);
            Respuesta::rechazo(codigo, mensaje, self.estado.version)
        })
    }

    /// `EncargarCaravana` (rutas de caravanas §2 y §4): la caravana comercial `disponible` queda `Esperando` en su origen con el encargo; con hora de salida espera a ella, y sin
    /// hora empieza ya a cargar.
    fn encargar(&mut self, t: Instante, p: &Peticion, caravana: u64, nuevo: Encargo) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let (i, origen) = self.caravana_de_orden(p, caravana)?;
        let destino = u64::from(nuevo.destino);
        self.plaza_o_rechazo(destino)?;
        if self.estado.comercio.flota[i].encargo.is_some() {
            return Err(Respuesta::rechazo(rechazos::ENCARGO_YA_TIENE, "la caravana ya tiene un encargo: se cambia o se cancela", v));
        }
        self.validar_encargo(&nuevo, i, origen, t)?;
        let (modo, salir_en) = (nuevo.modo, nuevo.salir_en);
        let mut copia = self.estado.comercio.flota[i].clone();
        copia.dar_encargo(nuevo).map_err(|e| rechazo_en_caravana(e, v))?;
        if self.trazar_ruta_comercial(origen, destino, t).is_none() {
            return Err(rechazo_en_caravana(ErrorCaravana::SinRuta, v));
        }
        self.estado.comercio.flota[i] = copia;
        self.tr.tocar(Clase::Caravana, caravana);
        self.emitir_comercio(t, EventoComercio::EncargoDado { caravana, destino: id32(destino), modo }, vec![origen, destino], &[]);
        match salir_en {
            Some(hora) => self.programar_comercio(hora, entidad_caravana(caravana), SucesoComercio::FinPasoEncargo),
            None => self.cargar(t, caravana, false),
        }
        Ok(())
    }

    /// `CambiarEncargo` (rutas de caravanas §4 y §7): sustituye el destino y las paradas y conserva `vueltas`, `punta`, `fase`, `fin_paso` y `cancelado`, así que el paso en curso sigue
    /// y lo nuevo vale desde el siguiente. El destino solo cambia con la caravana `Esperando` en el origen; la hora de salida solo cuenta mientras el envío espera a ella.
    fn cambiar_encargo(&mut self, t: Instante, p: &Peticion, caravana: u64, destino: u64, salir_en: Option<Instante>, paradas: [Parada; 2]) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let [en_origen, en_destino] = paradas;
        let (i, origen) = self.caravana_de_orden(p, caravana)?;
        self.plaza_o_rechazo(destino)?;
        let c = &self.estado.comercio.flota[i];
        let Some(actual) = c.encargo.clone() else {
            return Err(Respuesta::rechazo(rechazos::ENCARGO_NO_TIENE, "la caravana no tiene encargo", v));
        };
        let en_origen_esperando = c.estado == EstadoCaravana::Esperando && actual.punta == Punta::Origen;
        if id32(destino) != actual.destino && !en_origen_esperando {
            return Err(rechazo_en_caravana(ErrorCaravana::NoDisponible, v));
        }
        let espera_su_hora = en_origen_esperando && actual.vueltas == 0 && actual.fase == Fase::Espera && actual.salir_en.is_some_and(|hora| hora > t);
        let mut nuevo = Encargo { destino: id32(destino), salir_en: if espera_su_hora { salir_en } else { None }, en_origen, en_destino, ..actual.clone() };
        self.validar_encargo(&nuevo, i, origen, t)?;
        if !espera_su_hora {
            nuevo.salir_en = actual.salir_en;
        }
        if nuevo == actual {
            return Err(Respuesta::sin_cambios(v));
        }
        if nuevo.destino != actual.destino && self.trazar_ruta_comercial(origen, destino, t).is_none() {
            return Err(rechazo_en_caravana(ErrorCaravana::SinRuta, v));
        }
        let nueva_hora = nuevo.salir_en;
        let cambia_la_hora = nueva_hora != actual.salir_en;
        self.estado.comercio.flota[i].encargo = Some(nuevo);
        self.tr.tocar(Clase::Caravana, caravana);
        if espera_su_hora && cambia_la_hora {
            self.invalidar_comercio(entidad_caravana(caravana));
            match nueva_hora {
                Some(hora) => self.programar_comercio(hora, entidad_caravana(caravana), SucesoComercio::FinPasoEncargo),
                None => self.cargar(t, caravana, false),
            }
        }
        Ok(())
    }

    /// `CancelarEncargo` (rutas de caravanas §4, fila «Cancelar»): esperando en el origen termina ya; esperando en el destino sale ya de vuelta sin dejar ni recoger más; en ruta marca
    /// el encargo como cancelado y la caravana acaba el tramo, no comercia en el destino y termina al llegar al origen.
    fn cancelar_encargo(&mut self, t: Instante, p: &Peticion, caravana: u64) -> Result<(), Respuesta> {
        let v = self.estado.version;
        let (i, _) = self.caravana_de_orden(p, caravana)?;
        let c = &self.estado.comercio.flota[i];
        let Some(e) = c.encargo.as_ref() else {
            return Err(Respuesta::rechazo(rechazos::ENCARGO_NO_TIENE, "la caravana no tiene encargo", v));
        };
        if e.cancelado {
            return Err(Respuesta::sin_cambios(v));
        }
        let (esperando, punta) = (c.estado == EstadoCaravana::Esperando, e.punta);
        if esperando && punta == Punta::Origen {
            self.terminar(t, caravana, MotivoEncargo::Cancelado);
            return Ok(());
        }
        self.estado.comercio.flota[i].encargo.as_mut().expect("comprobado arriba").cancelado = true;
        self.tr.tocar(Clase::Caravana, caravana);
        if esperando {
            self.invalidar_comercio(entidad_caravana(caravana));
            self.salir(t, caravana);
        }
        Ok(())
    }

    // ───────────────────────────── El ciclo ─────────────────────────────

    /// «El destino sigue vigente» (rutas de caravanas §3): existe y, en una ruta, es de la Facción del origen.
    fn destino_vigente(&self, c: &Caravana) -> bool {
        let Some(e) = c.encargo.as_ref() else { return false };
        self.destino_propio(c).is_some_and(|propio| propio || e.modo == Modo::Envio)
    }

    /// Si el destino del encargo existe, si es de la Facción del origen.
    fn destino_propio(&self, c: &Caravana) -> Option<bool> {
        let destino = self.estado.asentamientos.get(&u64::from(c.encargo.as_ref()?.destino))?;
        Some(self.estado.asentamientos.get(&u64::from(c.origen)).is_some_and(|o| o.faccion == destino.faccion))
    }

    /// Paso 1 · Llegada (rutas de caravanas §3): la caravana acaba de quedar `Esperando` en una punta, con los peajes ya cobrados.
    pub(super) fn llegada_a_parada(&mut self, t: Instante, id: u64, punta: Punta) {
        self.retirar_plan(id);
        let Some(c) = self.estado.comercio.caravana(id) else { return };
        let Some(e) = c.encargo.as_ref() else { return };
        let cancelado = e.cancelado;
        let vigente = self.destino_vigente(c);
        if punta == Punta::Origen {
            // Un destino perdido gana a un encargo cancelado (rutas de caravanas §4, «Destino de una ruta»).
            if !vigente {
                self.terminar(t, id, MotivoEncargo::DestinoPerdido);
            } else if cancelado {
                self.terminar(t, id, MotivoEncargo::Cancelado);
            } else {
                self.descargar(t, id);
            }
        } else if cancelado || !vigente {
            self.salir(t, id);
        } else {
            self.descargar(t, id);
        }
    }

    /// Paso 2 · Descarga: en una plaza propia deja lo que dice la parada y tira o conserva el sobrante. Una ruta, y toda descarga en el origen, deja directo en el almacén; un envío en
    /// su destino la liquida como una entrega (comisión y recargo). En una plaza ajena vende a sus órdenes de compra ([`Nucleo::comerciar_caravana`]). Tarda `tiempoDescarga` por carro que mueve algo.
    fn descargar(&mut self, t: Instante, id: u64) {
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return };
        let c = &self.estado.comercio.flota[i];
        let Some(e) = c.encargo.as_ref() else { return };
        let (origen, en_origen) = (u64::from(c.origen), e.punta == Punta::Origen);
        let plaza = if en_origen { origen } else { u64::from(e.destino) };
        let Some(a) = self.estado.asentamientos.get(&plaza) else { return };
        if !en_origen && self.destino_propio(c) == Some(false) {
            return self.vender_en_ajena(t, id, i, plaza);
        }
        let parada = if en_origen { &e.en_origen } else { &e.en_destino };
        let reparto = repartir_descarga(&c.contenido, &parada.dejar, parada.desechar_sobrante, &a.economia.almacen);
        let carros = carros_del_paso(total(&reparto.entra) + total(&reparto.tirado), self.capacidad_de_caravana(c), c.carros_con_traccion().count());
        let liquida = e.modo == Modo::Envio && !en_origen;
        let comision = if liquida {
            self.liquidar_entrega(t, origen, plaza, &reparto.entra).map_or(0, |entrega| entrega.comision)
        } else {
            let almacen = &mut self.estado.asentamientos.get_mut(&plaza).expect("existe").economia.almacen;
            for (&r, &n) in &reparto.entra {
                almacen.agregar(r, n);
            }
            0
        };
        let c = &mut self.estado.comercio.flota[i];
        for (r, &n) in reparto.entra.iter().chain(&reparto.tirado) {
            *c.contenido.get_mut(r).expect("lo que deja lo lleva") -= n;
        }
        c.contenido.retain(|_, n| *n > 0);
        self.cerrar_descarga(t, id, reparto.entra, total(&reparto.tirado), carros, comision);
    }

    /// Rutas de caravanas §6 · Descarga en una plaza ajena: sin Mercado activo la caravana sale ya con lo que lleva; con él, vende y tarda `tiempoDescarga` por carro de lo vendido. No tira nada.
    fn vender_en_ajena(&mut self, t: Instante, id: u64, i: usize, plaza: u64) {
        let Some(vendido) = self.comerciar_caravana(t, i, plaza, true) else { return self.salir(t, id) };
        let c = &self.estado.comercio.flota[i];
        let carros = carros_del_paso(total(&vendido), self.capacidad_de_caravana(c), c.carros_con_traccion().count());
        self.cerrar_descarga(t, id, vendido, 0, carros, 0);
    }

    /// Cierra el paso de descarga ya aplicado a la caravana: lo anota en su encargo, emite el hecho y espera lo que tarden sus `carros` o sigue ya.
    fn cerrar_descarga(&mut self, t: Instante, id: u64, entra: Mercancia, tirado: Mil, carros: u32, comision: Mil) {
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return };
        let Some(plaza) = plaza_de_parada(&self.estado.comercio.flota[i]) else { return };
        let fin_paso = (carros > 0).then(|| t + self.estado.reglas.economia.comercio.encargo.tiempo_descarga * i64::from(carros));
        let c = &mut self.estado.comercio.flota[i];
        let a_bordo = total(&c.contenido);
        let e = c.encargo.as_mut().expect("plaza_de_parada comprobó el encargo");
        (e.fase, e.fin_paso) = (Fase::Descarga, fin_paso);
        self.tr.tocar(Clase::Caravana, id);
        self.tr.tocar(Clase::Asentamiento, plaza);
        let hecho = EventoComercio::CaravanaDescarga { caravana: id, plaza: id32(plaza), entra, tirado, a_bordo, carros, fin_paso: fin_paso.unwrap_or(0), comision };
        self.emitir_comercio(t, hecho, vec![plaza], &[]);
        match fin_paso {
            Some(fin) => self.programar_comercio(fin, entidad_caravana(id), SucesoComercio::FinPasoEncargo),
            None => self.acabar_descarga(t, id),
        }
    }

    /// Acaba la descarga: en el origen de un envío el encargo está cumplido; en los demás casos sigue la carga.
    fn acabar_descarga(&mut self, t: Instante, id: u64) {
        let Some(e) = self.estado.comercio.caravana(id).and_then(|c| c.encargo.as_ref()) else { return };
        if e.modo == Modo::Envio && e.punta == Punta::Origen {
            self.terminar(t, id, MotivoEncargo::Cumplido);
        } else {
            self.cargar(t, id, false);
        }
    }

    /// Paso 3 · Carga: en una plaza ajena compra a sus órdenes de venta (sin esperar existencias: lleva lo que haya). En una plaza propia toma del almacén lo que dice la parada. Con `esperar_existencias` y algo que falta espera con un pulso por minuto (el hecho sale al empezar la
    /// espera, `en_pulso` es que ya estaba esperando); si no, sube lo tomado y tarda `tiempoCarga` por carro que mueve algo. Antes de cargar en el origen, el destino debe seguir vigente.
    fn cargar(&mut self, t: Instante, id: u64, en_pulso: bool) {
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return };
        let c = &self.estado.comercio.flota[i];
        let Some(e) = c.encargo.as_ref() else { return };
        let en_origen = e.punta == Punta::Origen;
        if en_origen && !self.destino_vigente(c) {
            return self.terminar(t, id, MotivoEncargo::DestinoPerdido);
        }
        let plaza = if en_origen { u64::from(c.origen) } else { u64::from(e.destino) };
        let Some(a) = self.estado.asentamientos.get(&plaza) else { return self.salir(t, id) };
        if !en_origen && self.destino_propio(c) == Some(false) {
            return self.comprar_en_ajena(t, id, i, plaza);
        }
        let parada = if en_origen { &e.en_origen } else { &e.en_destino };
        let capacidad = self.capacidad_de_caravana(c);
        let carros_con_animal = c.carros_con_traccion().count();
        let toma = match a_recoger(&c.contenido, &parada.recoger, parada.esperar_existencias, capacidad, &a.economia.almacen) {
            Ok(toma) => toma,
            Err(falta) => {
                let e = self.estado.comercio.flota[i].encargo.as_mut().expect("comprobado arriba");
                (e.fase, e.fin_paso) = (Fase::Espera, None);
                self.tr.tocar(Clase::Caravana, id);
                self.programar_comercio(t + MS_POR_MINUTO, entidad_caravana(id), SucesoComercio::PulsoEncargo);
                if !en_pulso {
                    self.emitir_comercio(t, EventoComercio::CaravanaEspera { caravana: id, plaza: id32(plaza), falta }, vec![plaza], &[]);
                }
                return;
            }
        };
        let carros = carros_del_paso(total(&toma), capacidad, carros_con_animal);
        let almacen = &mut self.estado.asentamientos.get_mut(&plaza).expect("existe").economia.almacen;
        let c = &mut self.estado.comercio.flota[i];
        for (&r, &n) in &toma {
            *c.contenido.entry(r).or_insert(0) += almacen.quitar(r, n);
        }
        self.cerrar_carga(t, id, toma, carros);
    }

    /// Rutas de caravanas §6 · Carga en una plaza ajena: sin Mercado activo la caravana sale sin comprar; con él, compra lo que haya y tarda
    /// `tiempoCarga` por carro de lo comprado.
    fn comprar_en_ajena(&mut self, t: Instante, id: u64, i: usize, plaza: u64) {
        let Some(comprado) = self.comerciar_caravana(t, i, plaza, false) else { return self.salir(t, id) };
        let c = &self.estado.comercio.flota[i];
        let carros = carros_del_paso(total(&comprado), self.capacidad_de_caravana(c), c.carros_con_traccion().count());
        self.cerrar_carga(t, id, comprado, carros);
    }

    /// Cierra el paso de carga ya aplicado a la caravana: sin carros que muevan algo sale ya; si no, lo anota en su encargo, emite el hecho y espera lo que tarden sus `carros`.
    fn cerrar_carga(&mut self, t: Instante, id: u64, tomado: Mercancia, carros: u32) {
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return };
        let Some(plaza) = plaza_de_parada(&self.estado.comercio.flota[i]) else { return };
        self.tr.tocar(Clase::Caravana, id);
        if carros == 0 {
            return self.salir(t, id);
        }
        self.tr.tocar(Clase::Asentamiento, plaza);
        let fin_paso = t + self.estado.reglas.economia.comercio.encargo.tiempo_carga * i64::from(carros);
        let e = self.estado.comercio.flota[i].encargo.as_mut().expect("plaza_de_parada comprobó el encargo");
        (e.fase, e.fin_paso) = (Fase::Carga, Some(fin_paso));
        self.emitir_comercio(t, EventoComercio::CaravanaCarga { caravana: id, plaza: id32(plaza), tomado, carros, fin_paso }, vec![plaza], &[]);
        self.programar_comercio(fin_paso, entidad_caravana(id), SucesoComercio::FinPasoEncargo);
    }

    /// Rutas de caravanas §6 · La caravana `i` comercia en la plaza ajena `plaza` con las órdenes de su mercado: vende con las líneas de `dejar` de la parada de destino o compra con las de
    /// `recoger` ([`vender`], [`comprar`]). `None` si la plaza no tiene Mercado activo o ya no existe ella o el origen: no se comercia. Si no, lo servido por recurso (vacío si nada casó).
    fn comerciar_caravana(&mut self, t: Instante, i: usize, plaza: u64, vende: bool) -> Option<Mercancia> {
        let c = &self.estado.comercio.flota[i];
        let parada = c.encargo.as_ref()?.en_destino.clone();
        let (id, capacidad) = (c.id, self.capacidad_de_caravana(c));
        let faccion = self.estado.asentamientos.get(&u64::from(c.origen))?.faccion;
        let pd = self.estado.asentamientos.get(&plaza)?;
        nivel_mercado(pd)?;
        let contexto = ContextoParada { plaza_id: id32(plaza), ahora: t, factor_comision_externa: self.factores_de(pd).comision_externa };
        let cr = self.estado.reglas.economia.comercio.clone();
        let antes = self.ordenes_del_libro();
        let Estado { comercio, asentamientos, .. } = &mut self.estado;
        let almacen = &mut asentamientos.get_mut(&plaza)?.economia.almacen;
        let EstadoComercio { flota, libro, .. } = comercio;
        let a_bordo = &mut flota[i].contenido;
        let tratos = if vende {
            vender(&cr, &parada.dejar, &mut libro.ordenes, almacen, a_bordo, capacidad, &contexto)
        } else {
            comprar(&cr, &parada.recoger, &mut libro.ordenes, almacen, a_bordo, capacidad, &contexto)
        };
        let servido = self.por_recurso(&tratos);
        if !tratos.is_empty() {
            self.tras_servir(t, plaza, &antes, ("caravana", id), faccion, &tratos);
        }
        Some(servido)
    }

    /// Paso 4 · Salida hacia la otra punta con ruta nueva. Del origen, si el destino ya no sigue vigente o no hay ruta, el encargo termina; del destino la vuelta usa siempre la Facción
    /// del origen y, sin ruta (no debería pasar: la ida existía), la caravana se pierde.
    fn salir(&mut self, t: Instante, id: u64) {
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return };
        let c = &self.estado.comercio.flota[i];
        let Some(e) = c.encargo.as_ref() else { return };
        let (origen, destino, de_ida) = (u64::from(c.origen), u64::from(e.destino), e.punta == Punta::Origen);
        let trazado = if de_ida {
            if !self.destino_vigente(c) {
                return self.terminar(t, id, MotivoEncargo::DestinoPerdido);
            }
            self.trazar_ruta_comercial(origen, destino, t)
        } else {
            let aqui = c.posicion;
            let del_origen = self.estado.asentamientos.get(&origen).map(|o| (o.posicion, o.faccion));
            del_origen.and_then(|(pos_origen, faccion)| self.trazar_ruta(destino, origen, aqui, pos_origen, faccion, t))
        };
        let Some(z) = trazado else {
            if de_ida {
                self.terminar(t, id, MotivoEncargo::DestinoPerdido);
            } else {
                self.perder_caravana(t, id);
            }
            return;
        };
        let peajes = peajes_de_paradas(&z.paradas, longitud_ruta(&z.puntos));
        let plan = self.plan_de(&self.estado.comercio.flota[i], &z.puntos, &z.costes, t);
        if self.estado.comercio.flota[i].salir_de_parada(z.puntos.clone(), z.costes, peajes, plan.clone()).is_err() {
            return;
        }
        self.instalar_plan(id, plan);
        self.emitir_comercio(t, EventoComercio::CaravanaSale { caravana: id }, vec![origen, destino], &[]);
        self.registrar_ruta_en_red(origen, destino, &z.puntos, t, &[id]);
    }

    /// Paso 7 · Terminar: la caravana queda `Disponible` en el origen sin encargo; su carga se vuelca en el almacén del origen (lo que no cabe se pierde), la escolta vuelve al campamento
    /// y la caravana libre puede servir a un trueque.
    pub(super) fn terminar(&mut self, t: Instante, id: u64, motivo: MotivoEncargo) {
        self.invalidar_comercio(entidad_caravana(id));
        let Some(i) = self.estado.comercio.posicion_caravana(id) else { return };
        let origen = u64::from(self.estado.comercio.flota[i].origen);
        let Ok((carga, escolta)) = self.estado.comercio.flota[i].terminar_encargo() else { return };
        self.devolver_escolta(&escolta);
        let volcado = self.estado.asentamientos.get_mut(&origen).map_or(0, |a| carga.iter().map(|(&r, &n)| a.economia.almacen.agregar(r, n)).sum::<Mil>());
        self.tr.tocar(Clase::Caravana, id);
        self.tr.tocar(Clase::Asentamiento, origen);
        self.emitir_comercio(t, EventoComercio::EncargoTerminado { caravana: id, motivo, volcado, perdido: total(&carga) - volcado }, vec![origen], &[]);
        self.reparto_de_plaza(origen, t);
    }

    /// La caravana de la que se trata, si sigue `Esperando` con encargo: en qué fase está.
    fn fase_esperando(&self, id: u64) -> Option<Fase> {
        let c = self.estado.comercio.caravana(id)?;
        (c.estado == EstadoCaravana::Esperando).then_some(c.encargo.as_ref()?.fase)
    }

    /// Vence el fin del paso en curso: la hora de salida de un envío empieza la carga, la descarga acabada sigue con la carga (o termina el envío) y la carga acabada sale.
    pub(super) fn fin_de_paso_de_encargo(&mut self, id: u64, t: Instante) {
        match self.fase_esperando(id) {
            None => {}
            Some(Fase::Espera) => self.cargar(t, id, false),
            Some(Fase::Descarga) => self.acabar_descarga(t, id),
            Some(Fase::Carga) => self.salir(t, id),
        }
    }

    /// Vence el pulso de una caravana que espera existencias: repite la carga con la parada vigente.
    pub(super) fn pulso_de_encargo(&mut self, id: u64, t: Instante) {
        if self.fase_esperando(id) == Some(Fase::Espera) {
            self.cargar(t, id, true);
        }
    }
}
