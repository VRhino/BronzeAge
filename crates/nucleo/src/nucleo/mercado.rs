//! El comercio estático dentro del núcleo (Comercio_Caravanas §4, §8, §12): trueques, órdenes del mercado de una plaza, mostrador y
//! reputación entre Facciones, con sus plazos como eventos de la cola. Las reglas puras viven en el crate `comercio`; aquí están el estado,
//! el reloj, la autorización provisional y el traslado de los sucesos a eventos y versiones del núcleo. La flota de caravanas y los cierres
//! por ruina o conquista están en [`flota`](super::flota); su viaje y el reparto, en [`viaje`](super::viaje); el mercado de los
//! campamentos de mercenarios, en [`mercenarios`](super::mercenarios).
//!
//! # Los plazos
//! Un trueque, una orden y el fin de preparación de una caravana son eventos `Carga::Comercio` de prioridad 6, ligados a la generación de
//! su entidad (clave con etiqueta, `estado_comercio`): contestar, cancelar o cerrar la entidad sube la generación y el evento viejo se
//! descarta al salir. Dentro del mismo instante el orden es llegadas, fin de preparación, caducidades y reparto (`SucesoComercio::rango`):
//! un evento de rango `r` se reprograma `r` veces en ese instante, de modo que sale detrás de los de rango menor que ya estaban en la cola.
//!
//! # La reputación
//! Es un ritmo por Facción (`comercio::reputacion`). Tras cada cambio se recalcula su cita (prioridad 9): el cruce del 0 por debajo (se
//! reancla) o la salida del umbral bajo `umbralBajoParaComision`, que deja de pagar el recargo de la comisión.
//!
//! # Política de versión
//! Una orden aceptada cierra versión (como el resto). Los plazos cierran versión porque emiten hechos (`trueque_expirado`,
//! `orden_expirada`...); una reputación que sube o baja sola no.

use super::*;
use crate::estado::{Asentamiento, CitaReputacion, SucesoComercio, PRIO_COMERCIO, PRIO_REPUTACION};
use crate::estado_comercio::{entidad_orden, entidad_reputacion, entidad_trueque, id_de_entidad, EstadoComercio};
use asentamiento::catalogo::Tipo;
use asentamiento::Recurso;
use cola::Clave;
use comercio::entrega::{entregar, DatosEntrega, ResultadoEntrega};
use comercio::evento::Evento as EventoComercio;
use comercio::flota::{cupos, Cupos, FactoresPolitica};
use comercio::mostrador::{comerciar, Carro, ContextoMostrador, ErrorMostrador};
use comercio::orden_mercado::{ErrorOrden, EstadoOrden, OrdenMercado, TipoOrden};
use comercio::precio::{precio_referencia, stock_global, ContextoComision, TablaPrecios};
use comercio::reputacion::Suceso;
use comercio::trueque::{podar, AcuerdoTrueque, ErrorTrueque, Vencimiento};
use comercio::{total, Carga as Mercancia, Lado};

/// Capacidad del carro de una columna (`LOGISTICA.capacidadCarroPorJugador`: 500).
/// La capacidad total de la columna (`participantes × capacidadCarroPorJugador`, más las caravanas adjuntas y «Carga Ampliada») es
/// `Nucleo::capacidad_de_carga`; esta es la de un héroe, que es lo que cabe en el carro con que sale quien sale de un lugar.
pub(super) const CAPACIDAD_CARRO: Mil = 500_000;

/// Los ids de plaza del crate `comercio` son `u32`; los del núcleo, `u64` de un contador que no llega a ese tope.
pub(super) fn id32(id: u64) -> u32 {
    id as u32
}

/// El nivel interno del Mercado activo de la plaza (`edificiosPorTipoYEstado(…, 'mercado')[0]`), si lo hay.
pub(super) fn nivel_mercado(a: &Asentamiento) -> Option<u8> {
    a.economia.edificios.iter().find(|e| e.tipo == Tipo::Mercado && e.activo()).map(|e| e.nivel_interno)
}

fn rechazo_de_trueque(e: ErrorTrueque) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorTrueque::MismoAsentamiento => (TRUEQUE_MISMO_ASENTAMIENTO, "un trueque necesita dos asentamientos distintos"),
        ErrorTrueque::LadoVacio => (TRUEQUE_LADO_VACIO, "cada lado del trueque necesita al menos una línea"),
        ErrorTrueque::CantidadNoPositiva => (TRUEQUE_CANTIDAD_NO_POSITIVA, "las cantidades del trueque deben ser mayores que cero"),
        ErrorTrueque::RecursoRepetido => (TRUEQUE_RECURSO_REPETIDO, "un lado del trueque no puede repetir recurso"),
        ErrorTrueque::NoAutorizado => (NO_AUTORIZADO, "no mandas en esa plaza"),
        ErrorTrueque::NoEstaPropuesto => (TRUEQUE_NO_ESTA_PROPUESTO, "el trueque ya no está pendiente de respuesta"),
        ErrorTrueque::PropuestaVencida => (TRUEQUE_PROPUESTA_VENCIDA, "el plazo para contestar ya venció"),
    }
}

fn rechazo_de_orden(e: ErrorOrden) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorOrden::NoAutorizado => (NO_AUTORIZADO, "no mandas en esa plaza"),
        ErrorOrden::SinMercado => (MERCADO_SIN_MERCADO, "la plaza no tiene un Mercado activo"),
        ErrorOrden::CantidadNoPositiva => (ORDEN_CANTIDAD_NO_POSITIVA, "la cantidad debe ser mayor que cero"),
        ErrorOrden::SinStock => (ORDEN_SIN_STOCK, "el almacén no tiene esa cantidad para vender"),
    }
}

fn rechazo_de_mostrador(e: ErrorMostrador) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorMostrador::NoEsLider => (MOSTRADOR_NO_ES_LIDER, "solo el Líder de la columna comercia en el mostrador"),
        ErrorMostrador::OrdenAjena => (MOSTRADOR_ORDEN_AJENA, "la orden no es de esa plaza"),
        ErrorMostrador::OrdenNoActiva => (ORDEN_NO_ACTIVA, "la orden ya no está activa"),
        ErrorMostrador::OrdenCaducada => (MOSTRADOR_ORDEN_CADUCADA, "la orden ya caducó"),
        ErrorMostrador::FueraDeLaPuerta => (MOSTRADOR_FUERA_DE_LA_PUERTA, "la columna no está en la puerta de la plaza"),
        ErrorMostrador::SinMercado => (MERCADO_SIN_MERCADO, "la plaza no tiene un Mercado activo"),
        ErrorMostrador::CantidadNoPositiva => (MOSTRADOR_CANTIDAD_NO_POSITIVA, "la cantidad debe ser mayor que cero"),
        ErrorMostrador::PrecioNoPositivo => (MOSTRADOR_PRECIO_NO_POSITIVO, "una orden sin precio positivo no se liquida"),
        ErrorMostrador::RecursoOro => (MOSTRADOR_RECURSO_ORO, "las órdenes sobre el oro no se liquidan en el mostrador"),
        ErrorMostrador::NadaQueServir => (MOSTRADOR_NADA_QUE_SERVIR, "no se puede servir nada: faltan existencias, oro o sitio"),
    }
}

/// Los `datos` de un hecho de comercio (enteros, como el resto de eventos del núcleo).
fn datos_de(ev: &EventoComercio) -> Vec<(&'static str, i64)> {
    use EventoComercio::*;
    match ev {
        TruequePropuesto { acuerdo } | TruequeAceptado { acuerdo } | TruequeRechazado { acuerdo } | TruequeExpirado { acuerdo } => vec![("acuerdo", *acuerdo as i64)],
        TruequeCumplido { acuerdo, entre_facciones, intercambiado } => {
            let mut d = vec![("acuerdo", *acuerdo as i64), ("entreFacciones", *entre_facciones as i64)];
            d.extend(intercambiado.iter().map(|(r, n)| (r.nombre(), *n)));
            d
        }
        TruequeCancelado { acuerdo, socia } => vec![("acuerdo", *acuerdo as i64), ("socia", *socia as i64)],
        CaravanaConstruida { caravana }
        | CaravanaCarroAgregado { caravana }
        | CaravanaCarroMovido { caravana }
        | CaravanaPreparando { caravana }
        | CaravanaCancelada { caravana }
        | CaravanaEscoltaCedida { caravana }
        | CaravanaEscoltaRetirada { caravana }
        | CargaCaravanaAparcada { caravana }
        | CaravanaEnviadaAlOrigen { caravana }
        | CaravanaAnimalComprado { caravana }
        | CaravanaSale { caravana }
        | CaravanaPerdida { caravana }
        | CaravanaVuelve { caravana } => vec![("caravana", *caravana as i64)],
        CaravanaReserva { caravana, reservada } => vec![("caravana", *caravana as i64), ("reservada", *reservada as i64)],
        PeajePaso { caravana, ciudad, cobrado } => {
            let mut d = vec![("caravana", *caravana as i64), ("ciudad", *ciudad as i64)];
            d.extend(cobrado.iter().map(|(r, n)| (r.nombre(), *n)));
            d
        }
        CaravanaLlega { caravana, destino, comision, recargo } => vec![("caravana", *caravana as i64), ("destino", *destino as i64), ("comision", *comision), ("recargo", *recargo)],
        EntregaEscoltada { acuerdo, caravana } => vec![("acuerdo", *acuerdo as i64), ("caravana", *caravana as i64)],
        OrdenColocada { orden } | OrdenExpirada { orden } => vec![("orden", *orden as i64)],
        ComercioEnPlaza { orden, cantidad, entre_facciones } => vec![("orden", *orden as i64), ("cantidad", *cantidad), ("entreFacciones", *entre_facciones as i64)],
        MercenariosComprado { campamento, heroe, recurso, cantidad } => vec![("campamento", *campamento as i64), ("heroe", *heroe as i64), ("recurso", crate::codec_economia::recurso_a_u8(*recurso) as i64), ("cantidad", *cantidad)],
        MercenariosRepostado { campamento, heroe, cantidad } => vec![("campamento", *campamento as i64), ("heroe", *heroe as i64), ("cantidad", *cantidad)],
    }
}

impl Nucleo {
    /// Radio de la puerta de una plaza, de un pueblo, de una avanzada o de un campamento (`MOVIMIENTO.radioPuerta`: 2 tras la calibración de distancias,
    /// Movimiento_Ejercitos §14).
    pub(super) fn radio_puerta(&self) -> Mil {
        self.estado.reglas.economia.militar.movimiento.radio_puerta
    }

    // ───────────────────────────── Consultas ─────────────────────────────

    /// Un trueque por id (también los terminados que aún no se podaron).
    pub fn acuerdo(&self, id: u64) -> Option<&AcuerdoTrueque> {
        self.estado.comercio.acuerdo(id)
    }

    /// Los trueques en los que participa la plaza, en orden de id.
    pub fn acuerdos_de(&self, plaza: u64) -> Vec<&AcuerdoTrueque> {
        self.estado.comercio.acuerdos.iter().filter(|a| a.a as u64 == plaza || a.b as u64 == plaza).collect()
    }

    /// El libro de órdenes de mercado de todas las plazas: las en pie y el historial de las cerradas.
    pub fn libro_ordenes(&self) -> &comercio::orden_mercado::LibroOrdenes {
        &self.estado.comercio.libro
    }

    /// El escaparate de una plaza: las órdenes que se pueden tomar ahora. Solo lo ve quien está en su puerta: `presente` lo comprueba quien
    /// llama (la proyección de cada jugador, con su columna), y sin él la lista sale vacía.
    pub fn escaparate(&self, plaza: u64, presente: bool) -> Vec<&OrdenMercado> {
        if !presente {
            return Vec::new();
        }
        self.estado.comercio.libro.escaparate(id32(plaza), self.estado.ahora)
    }

    /// La reputación de una Facción ahora (ppm de punto: 1 punto = 1.000.000; de −100 a +100 puntos; 0 si no consta). Es un número por Facción, no
    /// una relación entre dos (Comercio_Caravanas §15).
    pub fn reputacion(&self, faccion: u32) -> i64 {
        self.estado.comercio.reputacion_en(faccion, self.estado.ahora)
    }

    /// El carro (`suministro`) de una columna.
    pub fn suministro(&self, ejercito: u64) -> Mercancia {
        self.estado.ejercitos.get(&ejercito).map(|e| e.columna.carga.clone()).unwrap_or_default()
    }

    /// Los cupos de la flota de la plaza (Mercado activo y políticas): `None` si la plaza no existe.
    pub fn cupos_de_flota(&self, plaza: u64) -> Option<Cupos> {
        self.estado.asentamientos.get(&plaza).map(|a| self.cupos_de(a))
    }

    // ───────────────────────────── Ayudas ─────────────────────────────

    pub(super) fn plaza_o_rechazo(&self, id: u64) -> Result<&Asentamiento, Respuesta> {
        self.estado.asentamientos.get(&id).ok_or_else(|| Respuesta::rechazo(rechazos::ASENTAMIENTO_DESCONOCIDO, ids::asentamiento(id), self.estado.version))
    }

    pub(super) fn factores_de(&self, a: &Asentamiento) -> FactoresPolitica {
        FactoresPolitica::de(&a.economia.politicas, &self.estado.reglas.economia.datos.politicas)
    }

    pub(super) fn cupos_de(&self, a: &Asentamiento) -> Cupos {
        let e = &self.estado.reglas.economia;
        cupos(&e.datos.catalogo, &e.comercio.flota, nivel_mercado(a), &self.factores_de(a))
    }

    /// Emite un hecho de comercio con el código del documento (§12) y sus datos; `extra` añade cifras que la variante no lleva.
    pub(super) fn emitir_comercio(&mut self, t: Instante, ev: EventoComercio, plazas: Vec<u64>, extra: &[(&'static str, i64)]) {
        let mut datos = datos_de(&ev);
        datos.extend_from_slice(extra);
        self.emitir_propio(t, ev.codigo(), plazas, datos);
    }

    /// Un hecho del núcleo que el documento de comercio no nombra (reputación, experiencia, administración...).
    pub(super) fn emitir_propio(&mut self, t: Instante, codigo: &'static str, plazas: Vec<u64>, datos: Vec<(&'static str, i64)>) {
        self.emitir_nombrando(t, codigo, plazas, vec![], datos);
    }

    /// Como [`Nucleo::emitir_propio`], nombrando además a `heroes`: los de una columna que acaba de desarmarse para entrar en un lugar ya no la tienen, y sin esto no lo oirían.
    pub(super) fn emitir_nombrando(&mut self, t: Instante, codigo: &'static str, plazas: Vec<u64>, heroes: Vec<u64>, datos: Vec<(&'static str, i64)>) {
        let punto = plazas.first().and_then(|p| self.estado.asentamientos.get(p)).map(|a| a.posicion);
        self.tr.emitir(&self.estado, EventoNucleo { asentamientos: plazas, punto, heroes, ..EventoNucleo::nuevo(codigo, t, datos) });
    }

    /// Programa un plazo de comercio con la generación vigente de su entidad.
    pub(super) fn programar_comercio(&mut self, en: Instante, entidad: u64, suceso: SucesoComercio) {
        let clave = self.estado.generaciones.clave(entidad);
        self.estado.cola.programar(en, PRIO_COMERCIO, clave, Carga::Comercio { suceso, aplazos: 0 });
    }

    /// Invalida los plazos programados de una entidad de comercio.
    pub(super) fn invalidar_comercio(&mut self, entidad: u64) {
        self.estado.generaciones.subir(entidad);
    }

    /// Los ids de todas las órdenes del libro, en pie o en el historial.
    fn ordenes_del_libro(&self) -> std::collections::BTreeSet<u64> {
        let libro = &self.estado.comercio.libro;
        libro.ordenes.iter().chain(&libro.historial).map(|o| o.id).collect()
    }

    /// Anota en el registro de cambios las órdenes dadas y las que estaban en el libro (`antes`) y ya no están (podadas del historial).
    fn anotar_ordenes_cambiadas(&mut self, antes: &std::collections::BTreeSet<u64>, cambiadas: &[u64]) {
        let ahora = self.ordenes_del_libro();
        self.tr.tocar_todos(Clase::OrdenMercado, antes.difference(&ahora).copied().chain(cambiadas.iter().copied()));
    }

    /// `podarAcuerdosTerminados` (§4.7): se hace al tocar los trueques, no en cada paso (no hay tick).
    // DUDA: el documento poda al final de cada tick; sin tick, la poda es perezosa (al proponer, vencer o cerrar un trueque) y solo cambia
    // cuánto tiempo se conservan los terminados en memoria, no ninguna regla.
    fn podar_trueques(&mut self, t: Instante) {
        let cr = self.estado.reglas.economia.comercio.clone();
        let EstadoComercio { acuerdos, flota, .. } = &mut self.estado.comercio;
        let antes: Vec<u64> = acuerdos.iter().map(|a| a.id).collect();
        podar(acuerdos, &cr, t, |id| flota.iter().any(|c| c.origen_acuerdo.is_some_and(|(a, _)| a == id)));
        let podados: Vec<u64> = antes.into_iter().filter(|&id| acuerdos.binary_search_by_key(&id, |a| a.id).is_err()).collect();
        self.tr.tocar_todos(Clase::Acuerdo, podados);
    }

    // ───────────────────────────── Reputación ─────────────────────────────

    /// Aplica un suceso de reputación a una Facción, reprograma su cita y emite el hecho (y el aviso si entra en el umbral bajo).
    pub(super) fn aplicar_reputacion(&mut self, t: Instante, faccion: u32, suceso: Suceso) {
        let cr = self.estado.reglas.economia.comercio.clone();
        let rep = self.estado.comercio.reputacion.entry(faccion).or_insert_with(|| comercio::reputacion::Reputacion::nueva(t));
        let antes = rep.valor_en(t);
        self.tr.tocar(Clase::Faccion, u64::from(faccion));
        rep.aplicar(&cr.reputacion, t, suceso);
        let despues = rep.valor_en(t);
        let umbral = cr.reputacion.umbral_bajo_para_comision;
        self.emitir_propio(t, "reputacion.cambio", vec![], vec![("faccion", faccion as i64), ("delta", despues - antes), ("valor", despues)]);
        if antes > umbral && despues <= umbral {
            self.emitir_propio(t, "reputacion.umbral_bajo", vec![], vec![("faccion", faccion as i64), ("entra", 1), ("valor", despues)]);
        }
        self.programar_reputacion(faccion, t);
    }

    /// Recalcula la cita de la reputación de una Facción desde `t`: la anterior se invalida y se programa la próxima, si hay. Solo hay umbral bajo (−40): se
    /// anuncian el cruce del 0 y la entrada y salida del −40 (Comercio_Caravanas §15).
    pub(super) fn programar_reputacion(&mut self, faccion: u32, t: Instante) {
        let entidad = entidad_reputacion(faccion);
        self.estado.generaciones.subir(entidad);
        let umbral = self.estado.reglas.economia.comercio.reputacion.umbral_bajo_para_comision;
        let Some(rep) = self.estado.comercio.reputacion.get(&faccion) else { return };
        let cero = rep.proximo_cambio(t).map(|c| (c, CitaReputacion::CruceDeCero));
        let salida = if rep.esta_bajo(umbral, t) { rep.cruce_de(umbral + 1, t).map(|c| (c, CitaReputacion::SalidaDelUmbral)) } else { None };
        let proxima = match (cero, salida) {
            (Some(a), Some(b)) => Some(if b.0 <= a.0 { b } else { a }),
            (a, b) => a.or(b),
        };
        if let Some((en, cita)) = proxima {
            let clave = self.estado.generaciones.clave(entidad);
            self.estado.cola.programar(en, PRIO_REPUTACION, clave, Carga::Reputacion(cita));
        }
    }

    /// Vence la cita de la reputación de una Facción (la entidad del evento).
    pub(super) fn cita_de_reputacion(&mut self, entidad: u64, cita: CitaReputacion, t: Instante) {
        let faccion = id_de_entidad(entidad) as u32;
        let cr = self.estado.reglas.economia.comercio.clone();
        let Some(rep) = self.estado.comercio.reputacion.get_mut(&faccion) else { return };
        let valor = rep.valor_en(t);
        self.tr.tocar(Clase::Faccion, u64::from(faccion));
        rep.reanclar(&cr.reputacion, t);
        if cita == CitaReputacion::SalidaDelUmbral {
            self.emitir_propio(t, "reputacion.umbral_bajo", vec![], vec![("faccion", faccion as i64), ("entra", 0), ("valor", valor)]);
        }
        self.programar_reputacion(faccion, t);
    }

    // ───────────────────────────── Plazos ─────────────────────────────

    /// Vence un plazo de comercio. Antes de actuar deja pasar a los eventos del mismo instante de rango menor (ver el módulo).
    pub(super) fn comercio_vence(&mut self, clave: Clave, suceso: SucesoComercio, aplazos: u8, t: Instante) {
        if aplazos < suceso.rango() {
            self.estado.cola.programar(t, PRIO_COMERCIO, clave, Carga::Comercio { suceso, aplazos: aplazos + 1 });
            return;
        }
        let id = id_de_entidad(clave.entidad);
        match suceso {
            SucesoComercio::CaducaTrueque => self.caduca_trueque(id, t),
            SucesoComercio::CaducaOrden => self.caduca_orden(id, t),
            SucesoComercio::FinPreparacion => self.salida_de_preparacion(id, t),
            SucesoComercio::LlegadaCaravana => self.llegada_de_caravana(id, t),
            SucesoComercio::PeajeCaravana => self.peaje_de_caravana(id, t),
            SucesoComercio::PulsoReparto => self.pulso_de_reparto(id, t),
            SucesoComercio::ReponeMercenarios => self.repone_mercenarios(t),
        }
    }

    /// Un trueque `propuesto` caduca sin penalizar; uno `activo` vence y cada lado que no cumplió baja la reputación de su Facción (solo
    /// entre Facciones distintas). Lo ya entregado no se devuelve.
    fn caduca_trueque(&mut self, id: u64, t: Instante) {
        let Some(i) = self.estado.comercio.posicion_acuerdo(id) else { return };
        let (a, b) = (self.estado.comercio.acuerdos[i].a as u64, self.estado.comercio.acuerdos[i].b as u64);
        let facciones = (self.estado.asentamientos.get(&a).map_or(0, |p| p.faccion), self.estado.asentamientos.get(&b).map_or(0, |p| p.faccion));
        let Some(vencimiento) = self.estado.comercio.acuerdos[i].vencer(t, facciones) else { return };
        self.tr.tocar(Clase::Acuerdo, id);
        self.emitir_comercio(t, EventoComercio::TruequeExpirado { acuerdo: id }, vec![a, b], &[("incumplido", matches!(vencimiento, Vencimiento::Incumplido { .. }) as i64)]);
        if let Vencimiento::Incumplido { reputacion } = vencimiento {
            for (faccion, suceso) in reputacion {
                self.aplicar_reputacion(t, faccion, suceso);
            }
        }
        self.podar_trueques(t);
    }

    /// Caducan las órdenes activas vencidas y las cerradas pasan al historial.
    fn caduca_orden(&mut self, _id: u64, t: Instante) {
        let cr = self.estado.reglas.economia.comercio.clone();
        let antes = self.ordenes_del_libro();
        let expiradas = self.estado.comercio.libro.caducar(&cr.mercado, t);
        self.anotar_ordenes_cambiadas(&antes, &expiradas);
        for orden in expiradas {
            let plaza = self.estado.comercio.libro.buscar(orden).map(|o| o.asentamiento as u64);
            self.emitir_comercio(t, EventoComercio::OrdenExpirada { orden }, plaza.into_iter().collect(), &[]);
        }
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// Las órdenes de trueque, mercado y mostrador, y el andamiaje de administración que las alimenta. `Ok(Some(n))` = cambió algo y creó `n`;
    /// `Ok(None)` = cambió algo; rechazos y «sin cambios» salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_mercado(&mut self, t: Instante, p: &Peticion) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let sin_cambios = || Respuesta::sin_cambios(v);
        let cr = self.estado.reglas.economia.comercio.clone();
        let de_trueque = |e: ErrorTrueque| {
            let (c, m) = rechazo_de_trueque(e);
            rechazo(c, m.into())
        };
        match &p.comando {
            Comando::ProponerTrueque { a, b, lineas_a, lineas_b } => {
                let faccion_a = self.plaza_o_rechazo(*a)?.faccion;
                self.plaza_o_rechazo(*b)?;
                let puede = self.manda_en(p, faccion_a);
                let id = self.estado.comercio.siguiente_acuerdo;
                let acuerdo = AcuerdoTrueque::proponer(&cr.trueque, id, id32(*a), id32(*b), lineas_a, lineas_b, t, puede).map_err(de_trueque)?;
                self.estado.comercio.siguiente_acuerdo += 1;
                let expira = acuerdo.expira_en;
                self.estado.comercio.acuerdos.push(acuerdo);
                self.programar_comercio(expira, entidad_trueque(id), SucesoComercio::CaducaTrueque);
                self.podar_trueques(t);
                self.emitir_comercio(t, EventoComercio::TruequePropuesto { acuerdo: id }, vec![*a, *b], &[]);
                Ok(Some(id))
            }
            Comando::AceptarTrueque { acuerdo } | Comando::RechazarTrueque { acuerdo } => {
                let aceptar = matches!(p.comando, Comando::AceptarTrueque { .. });
                let i = self.estado.comercio.posicion_acuerdo(*acuerdo).ok_or_else(|| rechazo(rechazos::TRUEQUE_DESCONOCIDO, format!("trueque-{acuerdo}")))?;
                let (a, b) = (self.estado.comercio.acuerdos[i].a as u64, self.estado.comercio.acuerdos[i].b as u64);
                let faccion_b = self.plaza_o_rechazo(b)?.faccion;
                let puede = self.manda_en(p, faccion_b);
                let ac = &mut self.estado.comercio.acuerdos[i];
                if aceptar {
                    let vence = ac.aceptar(&cr.trueque, t, puede).map_err(de_trueque)?;
                    self.invalidar_comercio(entidad_trueque(*acuerdo));
                    self.programar_comercio(vence, entidad_trueque(*acuerdo), SucesoComercio::CaducaTrueque);
                    self.emitir_comercio(t, EventoComercio::TruequeAceptado { acuerdo: *acuerdo }, vec![a, b], &[]);
                    // El reparto se reevalúa al aceptar: cada plaza carga lo suyo con sus caravanas libres.
                    self.reparto_de_plaza(a, t);
                    self.reparto_de_plaza(b, t);
                } else {
                    ac.rechazar(t, puede).map_err(de_trueque)?;
                    self.invalidar_comercio(entidad_trueque(*acuerdo));
                    self.emitir_comercio(t, EventoComercio::TruequeRechazado { acuerdo: *acuerdo }, vec![a, b], &[]);
                }
                Ok(None)
            }
            Comando::ColocarOrdenMercado { asentamiento, venta, recurso, cantidad, precio } => {
                let plaza = self.plaza_o_rechazo(*asentamiento)?;
                let (faccion, tiene_mercado, stock) = (plaza.faccion, nivel_mercado(plaza).is_some(), plaza.economia.almacen.cantidad(*recurso));
                let puede = self.manda_en(p, faccion);
                // El precio de referencia solo se calcula si hace falta: suma el stock de todas las plazas.
                let referencia = if precio.is_none() {
                    precio_referencia(&cr.precio, *recurso, stock_global(*recurso, self.estado.asentamientos.values().map(|a| &a.economia.almacen)))
                } else {
                    0
                };
                let id = self.estado.comercio.siguiente_orden;
                let tipo = if *venta { TipoOrden::Venta } else { TipoOrden::Compra };
                let orden = OrdenMercado::colocar(&cr.mercado, id, id32(*asentamiento), tipo, *recurso, *cantidad, *precio, referencia, t, puede, tiene_mercado, stock).map_err(|e| {
                    let (c, m) = rechazo_de_orden(e);
                    rechazo(c, m.into())
                })?;
                self.estado.comercio.siguiente_orden += 1;
                let expira = orden.expira_en;
                self.estado.comercio.libro.colocar(orden);
                self.programar_comercio(expira, entidad_orden(id), SucesoComercio::CaducaOrden);
                self.emitir_comercio(t, EventoComercio::OrdenColocada { orden: id }, vec![*asentamiento], &[("precio", self.estado.comercio.libro.orden(id).map_or(0, |o| o.precio_unitario))]);
                Ok(Some(id))
            }
            Comando::CancelarOrdenMercado { orden } => {
                let Some(o) = self.estado.comercio.libro.buscar(*orden) else { return Err(rechazo(rechazos::ORDEN_DESCONOCIDA, ids::orden_de_mercado(*orden))) };
                if o.estado != EstadoOrden::Activa || self.estado.comercio.libro.orden(*orden).is_none() {
                    return Err(rechazo(rechazos::ORDEN_NO_ACTIVA, format!("{} ya no está activa", ids::orden_de_mercado(*orden))));
                }
                let plaza = o.asentamiento as u64;
                let faccion = self.plaza_o_rechazo(plaza)?.faccion;
                if !self.manda_en(p, faccion) {
                    return Err(rechazo(rechazos::NO_AUTORIZADO, format!("no mandas en as-{plaza}")));
                }
                // Retirarla antes de plazo la deja como una expirada (no hay estado «cancelada») y avisa con su propio hecho (Comercio_Caravanas §8.2).
                let antes = self.ordenes_del_libro();
                self.estado.comercio.libro.orden_mut(*orden).expect("está en pie").estado = EstadoOrden::Expirada;
                self.estado.comercio.libro.archivar_cerradas(&cr.mercado);
                self.anotar_ordenes_cambiadas(&antes, &[*orden]);
                self.invalidar_comercio(entidad_orden(*orden));
                self.emitir_propio(t, "mercado.orden_cancelada", vec![plaza], vec![("orden", *orden as i64)]);
                Ok(None)
            }
            Comando::ComerciarEnPlaza { ejercito, orden, cantidad } => self.comerciar_en_plaza(t, p, *ejercito, *orden, *cantidad),
            Comando::DarRecursos { asentamiento, recurso, cantidad } => {
                self.plaza_o_rechazo(*asentamiento)?;
                if *cantidad <= 0 {
                    return Err(rechazo(rechazos::CANTIDAD_NO_POSITIVA, "la cantidad debe ser mayor que cero".into()));
                }
                let entra = self.estado.asentamientos.get_mut(asentamiento).expect("existe").economia.almacen.agregar(*recurso, *cantidad);
                if entra == 0 {
                    return Err(sin_cambios());
                }
                self.tr.tocar(Clase::Asentamiento, *asentamiento);
                Ok(None)
            }
            Comando::DarSuministro { ejercito, recurso, cantidad } => {
                if !self.estado.ejercitos.contains_key(ejercito) {
                    return Err(rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(*ejercito)));
                }
                if *cantidad <= 0 {
                    return Err(rechazo(rechazos::CANTIDAD_NO_POSITIVA, "la cantidad debe ser mayor que cero".into()));
                }
                let capacidad = self.capacidad_de_carga(*ejercito).unwrap_or(CAPACIDAD_CARRO);
                let carro = &mut self.estado.ejercitos.get_mut(ejercito).expect("existe").columna.carga;
                let n = base::entero((*cantidad).min((capacidad - total(carro)).max(0))); // unidades enteras (C3b)
                if n == 0 {
                    return Err(sin_cambios());
                }
                *carro.entry(*recurso).or_insert(0) += n;
                self.tr.tocar(Clase::Ejercito, *ejercito);
                Ok(None)
            }
            _ => unreachable!("solo se llama con órdenes de mercado"),
        }
    }

    /// `comerciarEnPlaza` (§8.3): el Líder de una columna a la puerta toma una orden. La plaza y el carro se mueven con los deltas del crate; el
    /// oro se conserva (la comisión se la queda la plaza).
    fn comerciar_en_plaza(&mut self, t: Instante, p: &Peticion, ejercito: u64, orden: u64, cantidad: Mil) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let cr = self.estado.reglas.economia.comercio.clone();
        let e = self.estado.ejercitos.get(&ejercito).ok_or_else(|| rechazo(rechazos::EJERCITO_DESCONOCIDO, ids::ejercito(ejercito)))?;
        // Dirige la columna su Líder; la administración manda en todas.
        let (es_lider, pos_columna, faccion_columna) = (self.manda_columna(p, e, true), e.posicion_en(t), e.bandera());
        let o = self.estado.comercio.libro.buscar(orden).ok_or_else(|| rechazo(rechazos::ORDEN_DESCONOCIDA, ids::orden_de_mercado(orden)))?;
        let (plaza_id, recurso) = (o.asentamiento as u64, o.recurso);
        let plaza = self.plaza_o_rechazo(plaza_id)?;
        let factor = self.factores_de(plaza).comision_externa;
        let ctx = ContextoMostrador {
            ahora: t,
            es_lider,
            en_puerta: pos_columna.dist(plaza.posicion) <= self.radio_puerta(),
            mercado_activo: nivel_mercado(plaza).is_some(),
            misma_faccion: faccion_columna == plaza.faccion,
            factor_comision_externa: factor,
        };
        let faccion_plaza = plaza.faccion;
        let antes = self.ordenes_del_libro();
        let capacidad = self.capacidad_de_carga(ejercito).unwrap_or(CAPACIDAD_CARRO);
        let Estado { comercio, asentamientos, ejercitos, .. } = &mut self.estado;
        let suministro = ejercitos.get(&ejercito).map(|e| e.columna.carga.clone()).unwrap_or_default();
        let carro = Carro { oro: comercio::de(&suministro, Recurso::Oro), recurso: comercio::de(&suministro, recurso), espacio: (capacidad - total(&suministro)).max(0) };
        let almacen = &mut asentamientos.get_mut(&plaza_id).expect("existe").economia.almacen;
        // Una orden ya cerrada vive en el historial: se comprueba sobre una copia, que `comerciar` rechaza sin tocar.
        let en_pie = comercio.libro.ordenes.iter().position(|o| o.id == orden);
        let mut copia;
        let o: &mut OrdenMercado = match en_pie {
            Some(i) => &mut comercio.libro.ordenes[i],
            None => {
                copia = comercio.libro.buscar(orden).cloned().expect("la encontró antes");
                &mut copia
            }
        };
        let s = comerciar(&cr, o, id32(plaza_id), almacen, &carro, cantidad, &ctx).map_err(|e| {
            let (c, m) = rechazo_de_mostrador(e);
            rechazo(c, m.into())
        })?;
        let sum = &mut ejercitos.get_mut(&ejercito).expect("existe").columna.carga;
        *sum.entry(Recurso::Oro).or_insert(0) += s.delta_carro_oro;
        *sum.entry(recurso).or_insert(0) += s.delta_carro_recurso;
        sum.retain(|_, n| *n > 0);
        self.tr.tocar(Clase::Ejercito, ejercito);
        if s.cumplida {
            comercio.libro.archivar_cerradas(&cr.mercado);
            self.estado.generaciones.subir(entidad_orden(orden));
        }
        self.anotar_ordenes_cambiadas(&antes, &[orden]);
        self.tr.tocar(Clase::Asentamiento, plaza_id);
        let entre_facciones = faccion_columna != faccion_plaza;
        self.registrar_comercio_de_mercenarios(entre_facciones, &[(recurso, s.cantidad)].into_iter().collect());
        self.emitir_comercio(
            t,
            EventoComercio::ComercioEnPlaza { orden, cantidad: s.cantidad, entre_facciones },
            vec![plaza_id],
            &[("ejercito", ejercito as i64), ("valor", s.valor), ("comision", s.comision), ("oroMovido", s.oro_movido)],
        );
        Ok(None)
    }

    // ───────────────────────────── Entregas ─────────────────────────────

    /// Entrega `contenido` en el almacén de la plaza `destino` (§4.4, §3.3): cada recurso topado por su capacidad, la comisión **acuñada** y,
    /// si procede, el recargo por mala reputación que sale del oro del almacén de `origen` (se conserva). Devuelve lo que ocurrió, o `None`
    /// si alguna de las dos plazas ya no existe. No abona nada a ningún trueque.
    pub(super) fn liquidar_entrega(&mut self, t: Instante, origen: u64, destino: u64, contenido: &Mercancia) -> Option<ResultadoEntrega> {
        let cr = self.estado.reglas.economia.comercio.clone();
        let (po, pd) = (self.estado.asentamientos.get(&origen)?, self.estado.asentamientos.get(&destino)?);
        let precios = TablaPrecios::de(&cr.precio, self.estado.asentamientos.values().map(|a| &a.economia.almacen));
        let datos = DatosEntrega {
            comision: ContextoComision {
                mismo_asentamiento: origen == destino,
                misma_faccion: po.faccion == pd.faccion,
                factor_comision_externa: self.factores_de(pd).comision_externa,
                distancia_recta: po.posicion.dist(pd.posicion),
            },
            reputacion_origen: self.estado.comercio.reputacion_en(po.faccion, t),
            oro_origen: po.economia.almacen.cantidad(Recurso::Oro),
            precios: &precios,
        };
        let res = if origen == destino {
            let almacen = &mut self.estado.asentamientos.get_mut(&destino)?.economia.almacen;
            entregar(&cr, contenido, almacen, &datos)
        } else {
            let res = entregar(&cr, contenido, &mut self.estado.asentamientos.get_mut(&destino)?.economia.almacen, &datos);
            // El recargo sale del oro del origen: el crate lo suma al destino y aquí se resta, así el oro se conserva.
            self.estado.asentamientos.get_mut(&origen)?.economia.almacen.quitar(Recurso::Oro, res.recargo);
            res
        };
        self.tr.tocar_todos(Clase::Asentamiento, [origen, destino]);
        Some(res)
    }

    /// `aplicarEntregaATrueque` (§4.4): abona lo entregado al `lado` del trueque; si salda los dos lados lo cierra (`cumplido`), emite el
    /// hecho y reparte reputación y experiencia (solo entre Facciones distintas). Devuelve si lo cerró.
    pub(super) fn abonar_a_trueque(&mut self, t: Instante, acuerdo: u64, lado: Lado, carga: &Mercancia) -> bool {
        let Some(i) = self.estado.comercio.posicion_acuerdo(acuerdo) else { return false };
        let (a, b) = (self.estado.comercio.acuerdos[i].a as u64, self.estado.comercio.acuerdos[i].b as u64);
        let facciones = (self.estado.asentamientos.get(&a).map_or(0, |p| p.faccion), self.estado.asentamientos.get(&b).map_or(0, |p| p.faccion));
        let cierre = self.estado.comercio.acuerdos[i].abonar(lado, carga, facciones);
        self.tr.tocar(Clase::Acuerdo, acuerdo);
        let Some(cierre) = cierre else { return false };
        self.invalidar_comercio(entidad_trueque(acuerdo));
        self.registrar_comercio_de_mercenarios(cierre.entre_facciones, &cierre.intercambiado);
        let ev = EventoComercio::TruequeCumplido { acuerdo, entre_facciones: cierre.entre_facciones, intercambiado: cierre.intercambiado };
        self.emitir_comercio(t, ev, vec![a, b], &[("trasVencer", cierre.tras_vencer as i64)]);
        for (faccion, suceso) in cierre.reputacion {
            self.aplicar_reputacion(t, faccion, suceso);
        }
        for faccion in cierre.experiencia {
            self.dar_experiencia_por(t, faccion, politica::nivel::Hecho::TruequeCumplido, super::experiencia::motivo::TRUEQUE);
        }
        self.podar_trueques(t);
        true
    }
}
