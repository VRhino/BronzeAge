//! La economía de las plazas dentro del núcleo (Economia_Construccion): el paso de **un minuto de mundo** de cada plaza y las órdenes
//! sobre su cola de obras, mejoras, ascenso y políticas. Las reglas puras viven en el crate `asentamiento`; aquí está el reloj, el
//! mundo que ve (yacimientos, parcelas, zona) y el traslado de sus sucesos a eventos y versiones del núcleo.
//!
//! # El reloj de una plaza
//! Cada plaza tiene un evento recurrente `Carga::Minuto` con la fase de su fundación (los pasos caen en `fundado_en + k·60 s`), se mire
//! o no; no hay tick global. Prioridad [`PRIO_ASENTAMIENTO`]. Cada paso programa el siguiente **antes** de avanzar: si la plaza cae en
//! ruinas, subir su generación descarta ese evento sin que la plaza exista ya.
//!
//! **Límite: precisión de un minuto.** Las obras, mejoras, políticas y ascensos que el crate resuelve dentro del paso se aplican en el
//! primer paso con `t >= completa_en`: hasta 59,999 s después de su instante exacto. No se programa un evento aparte por cada fin.
//!
//! # Política de versión
//! La población, el almacén y el medidor cambian en cada paso: son **ritmos**, cambian el hash pero no la versión. El paso cierra
//! versión (y marca la plaza como tocada) solo si dejó algo discreto: algún suceso del crate (obra, mejora, ascenso, política,
//! hambruna, déficit...) o un cambio de la zona. Un paso sin novedades no sube la versión.

use super::*;
use crate::codec_economia::{recurso_a_u8, tipo_a_u8};
use crate::economia::{FuentesMundo, SitiosPlaza};
use crate::estado::PRIO_ASENTAMIENTO;
use base::MS_POR_MINUTO;
use asentamiento::asentamiento::{Evento as EventoEco, Exterior};
use asentamiento::ascenso::Bloqueo;
use asentamiento::catalogo::Tipo;
use asentamiento::datos::UNO;
use asentamiento::edificio::{Edificios, Fuente};
use asentamiento::obras::ErrorObra;
use asentamiento::politicas::{Activador, ErrorPolitica};
use asentamiento::Recurso;
use std::cell::OnceCell;
use territorio::Disco;

// ── Lo que el Exterior de la economía todavía no tiene (cada uno lo dará su sistema) ──
// placeholder: lo dará la fertilidad de la zona del mundo (`fertilidadZona`).
const FERTILIDAD: i64 = UNO;
// placeholder: lo darán las tecnologías de la Facción (sin tecnologías adoptadas, sin bonus).
const SIN_TECNOLOGIA: fn(&str) -> bool = |_| false;
const SIN_BONUS: fn(Recurso) -> i64 = |_| UNO;
// placeholder: lo dará el sistema de héroes y residentes (ciudadanos con casa en la plaza).
const CIUDADANOS: u32 = 1;
// placeholder: lo dará la geografía de la Facción (distancia a su capital) y el sistema de capitales (`es_capital`, Gran Fundición).
const DISTANCIA_CAPITAL: Mil = 0;
const ES_CAPITAL: bool = true;
const FACCION_TIENE_GRAN_FUNDICION: bool = false;
// placeholder: lo dará el sistema de Eras del servidor.
const ORDEN_ERA: u32 = 1;
// placeholder: lo darán las Murallas (recintos), las guarniciones y la ocupación/Aedas (factores extra).
const RECINTO_NIVEL: u32 = 0;
const RACION_GUARNICION: Mil = 0;
const FACTOR_EXTRA: i64 = UNO;

fn rechazo_de_obra(e: ErrorObra) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorObra::TecnologiaFaltante => (OBRA_TECNOLOGIA_FALTANTE, "la Facción no tiene la tecnología que pide el edificio"),
        ErrorObra::EraInsuficiente => (OBRA_ERA_INSUFICIENTE, "el servidor aún no ha llegado a la Era que pide el edificio"),
        ErrorObra::TipoNoManual => (OBRA_TIPO_NO_MANUAL, "el Centro Urbano y los puestos del Mercado no se añaden a mano"),
        ErrorObra::Unico => (OBRA_UNICO, "ya hay uno y solo puede haber uno por plaza"),
        ErrorObra::TopeAlmacenes => (OBRA_TOPE_ALMACENES, "se alcanzó el máximo de Almacenes del nivel"),
        ErrorObra::TopeViviendas => (OBRA_TOPE_VIVIENDAS, "se alcanzó el máximo de Viviendas del nivel"),
        ErrorObra::NivelInsuficiente => (OBRA_NIVEL_INSUFICIENTE, "el nivel de la plaza no alcanza para ese edificio"),
        ErrorObra::TopeTransformacion => (OBRA_TOPE_TRANSFORMACION, "se alcanzó el máximo de talleres de ese tipo del nivel"),
        ErrorObra::GranFundicion => (OBRA_GRAN_FUNDICION, "la Gran Fundición pide nivel de Facción, ser la capital y que no haya otra"),
        ErrorObra::ColaLlena => (OBRA_COLA_LLENA, "la cola de obras está llena"),
        ErrorObra::SinSitio => (OBRA_SIN_SITIO, "no hay sitio libre en la zona para ese edificio"),
        ErrorObra::RecursosInsuficientes => (OBRA_RECURSOS_INSUFICIENTES, "faltan recursos (o se bajaría de la reserva)"),
        ErrorObra::NoExiste => (EDIFICIO_DESCONOCIDO, "la plaza no tiene ese edificio"),
        ErrorObra::NoEstaEnCola => (OBRA_NO_ESTA_EN_COLA, "el edificio no está en la cola de obras"),
        ErrorObra::NoEsMejorable => (OBRA_NO_MEJORABLE, "el edificio no se puede mejorar ahora"),
        ErrorObra::SinCuadrilla => (OBRA_SIN_CUADRILLA, "no queda cuadrilla libre"),
    }
}

fn rechazo_de_bloqueo(b: &Bloqueo) -> (&'static str, String) {
    use rechazos::*;
    match b {
        Bloqueo::NivelMaximo => (ASCENSO_NIVEL_MAXIMO, "ya está en el nivel máximo".into()),
        Bloqueo::AscensoEnCurso => (ASCENSO_EN_CURSO, "ya hay un ascenso en curso".into()),
        Bloqueo::FaltaPoblacion => (ASCENSO_FALTA_POBLACION, "falta población".into()),
        Bloqueo::FaltanEdificios => (ASCENSO_FALTAN_EDIFICIOS, "faltan edificios".into()),
        Bloqueo::SinCupoDeFaccion => (ASCENSO_SIN_CUPO_DE_FACCION, "la Facción no tiene cupo para ese nivel".into()),
        Bloqueo::RecursosInsuficientes => (ASCENSO_RECURSOS_INSUFICIENTES, "faltan recursos para la obra de ascenso".into()),
        Bloqueo::Insolvente(r) => (ASCENSO_INSOLVENTE, format!("no cubriría el mantenimiento del nivel nuevo: {}", r.iter().map(|r| r.nombre()).collect::<Vec<_>>().join(", "))),
    }
}

fn rechazo_de_politica(e: ErrorPolitica) -> (&'static str, &'static str) {
    use rechazos::*;
    match e {
        ErrorPolitica::Desconocida => (POLITICA_DESCONOCIDA, "no existe esa política"),
        ErrorPolitica::NoEsDeSuPool => (POLITICA_FUERA_DE_POOL, "esa política no es de este cargo"),
        ErrorPolitica::CargoVacio => (POLITICA_CARGO_VACIO, "el cargo no está ocupado"),
        ErrorPolitica::YaActiva => (POLITICA_YA_ACTIVA, "esa política ya está activa en este cargo"),
        ErrorPolitica::SinSlot => (POLITICA_SIN_SLOT, "el cargo no tiene slots libres"),
    }
}

/// Un suceso del crate `asentamiento` como hecho del núcleo (crónica). Los `datos` son enteros: los tipos van por índice del enum, los
/// recursos faltantes por máscara de bits de su índice y la política por su posición en el catálogo.
fn evento_nucleo(id: u64, t: Instante, punto: Punto, e: &EventoEco, reglas: &asentamiento::Reglas) -> EventoNucleo {
    let (codigo, datos): (&'static str, Vec<(&'static str, i64)>) = match e {
        EventoEco::EdificioCompletado { id, tipo } => ("construccion.edificio_completado", vec![("edificio", *id as i64), ("tipo", tipo_a_u8(*tipo) as i64)]),
        EventoEco::ObraIniciada { id, tipo, completa_en } => ("construccion.obra_iniciada", vec![("edificio", *id as i64), ("tipo", tipo_a_u8(*tipo) as i64), ("completaEn", *completa_en)]),
        EventoEco::MejoraIniciada { id, nivel } => ("construccion.mejora_iniciada", vec![("edificio", *id as i64), ("nivel", *nivel as i64)]),
        EventoEco::MejoraCompletada { id, nivel } => ("construccion.mejora_completada", vec![("edificio", *id as i64), ("nivel", *nivel as i64)]),
        EventoEco::NecesidadDetectada { id, tipo, prioridad } => ("construccion.necesidad_detectada", vec![("edificio", *id as i64), ("tipo", tipo_a_u8(*tipo) as i64), ("prioridad", *prioridad)]),
        EventoEco::Encolada { id, tipo } => ("construccion.encolada", vec![("edificio", *id as i64), ("tipo", tipo_a_u8(*tipo) as i64)]),
        EventoEco::Desencolada { id } => ("construccion.desencolada", vec![("edificio", *id as i64)]),
        EventoEco::ColaReordenada => ("construccion.cola_reordenada", vec![]),
        EventoEco::PrimerosArtesanos => ("poblacion.primeros_artesanos", vec![]),
        EventoEco::PrimerosNobles => ("poblacion.primeros_nobles", vec![]),
        EventoEco::Hambruna { pesants, artesanos, nobleza } => ("poblacion.hambruna", vec![("pesants", *pesants), ("artesanos", *artesanos), ("nobleza", *nobleza)]),
        EventoEco::PoliticaExpirada { id } => ("politica.expirada", vec![("politica", reglas.politicas.catalogo.iter().position(|p| &p.id == id).map_or(-1, |i| i as i64))]),
        EventoEco::MantenimientoDeficit => ("mantenimiento.deficit", vec![]),
        EventoEco::MantenimientoColapsado { nivel } => ("mantenimiento.colapsado", vec![("nivel", *nivel as i64)]),
        EventoEco::MantenimientoRecuperado { nivel } => ("mantenimiento.recuperado", vec![("nivel", *nivel as i64)]),
        EventoEco::Ruinas { faltantes } => ("asentamiento.ruinas", vec![("faltantes", faltantes.iter().fold(0i64, |m, &r| m | 1 << recurso_a_u8(r)))]),
        EventoEco::AscensoIniciado { objetivo } => ("asentamiento.ascenso_iniciado", vec![("objetivo", *objetivo as i64)]),
        EventoEco::NivelSubio { nivel } => ("asentamiento.nivel_subio", vec![("nivel", *nivel as i64)]),
    };
    EventoNucleo { codigo: codigo.into(), en: t, ejercitos: vec![], asentamientos: vec![id], punto: Some(punto), datos }
}

impl Nucleo {
    // ───────────────────────────── Consultas ─────────────────────────────

    /// La economía de una plaza (solo lectura): población, almacén, edificios, cola, políticas, ascenso.
    pub fn economia(&self, id: u64) -> Option<&asentamiento::asentamiento::Asentamiento> {
        self.estado.asentamientos.get(&id).map(|a| &a.economia)
    }

    // ───────────────────────────── El reloj de una plaza ─────────────────────────────

    /// Programa el paso de un minuto de la plaza en `en`, con su generación vigente.
    pub(super) fn programar_minuto(&mut self, id: u64, en: Instante) {
        let clave = self.estado.generaciones.clave(id);
        self.estado.cola.programar(en, PRIO_ASENTAMIENTO, clave, Carga::Minuto);
    }

    /// Saca la plaza del mapa mientras se trabaja con ella (así el resto de plazas se puede leer: las vecinas que recortan su zona),
    /// le monta el `Exterior` y ejecuta `f`. La vuelve a insertar siempre. `None` si no hay plaza o no hay mundo.
    fn con_exterior<R>(&mut self, id: u64, t: Instante, f: impl FnOnce(&mut asentamiento::asentamiento::Asentamiento, &mut Exterior) -> R) -> Option<R> {
        let geo = self.geografia.as_ref()?;
        let mut a = self.estado.asentamientos.remove(&id)?;
        let r = {
            let Estado { reglas, asentamientos, parcelas_ocupadas, nodos_reclamados, yacimientos, contactos, niveles_faccion, .. } = &mut self.estado;
            let mut fuentes = FuentesMundo { mapa: &geo.mapa, nodos: yacimientos, ahora: t };
            let mut sitios = SitiosPlaza {
                mapa: &geo.mapa,
                cuota_pct: reglas.territorio.cuota_parcelas_pct,
                id,
                yo: Disco { centro: a.posicion, radio: a.radio, faccion: a.faccion },
                parcelas: &mut a.parcelas,
                parcelas_ocupadas,
                nodos_reclamados,
                otras: asentamientos,
                contactos,
                vecinos: OnceCell::new(),
            };
            let mut x = Exterior {
                reglas: &reglas.economia.datos,
                fuentes: &mut fuentes,
                sitios: &mut sitios,
                fertilidad: FERTILIDAD,
                tiene_tecnologia: &SIN_TECNOLOGIA,
                factor_tecnologia: &SIN_BONUS,
                nivel_faccion: niveles_faccion.get(&a.faccion).copied().unwrap_or(1),
                orden_era: ORDEN_ERA,
                es_capital: ES_CAPITAL,
                faccion_tiene_gran_fundicion: FACCION_TIENE_GRAN_FUNDICION,
                ciudadanos: CIUDADANOS,
                distancia_capital: DISTANCIA_CAPITAL,
                recinto_nivel: RECINTO_NIVEL,
                racion_guarnicion: RACION_GUARNICION,
                felicidad_extra: FACTOR_EXTRA,
                nobleza_extra: FACTOR_EXTRA,
                recaudacion_extra: FACTOR_EXTRA,
            };
            f(&mut a.economia, &mut x)
        };
        self.estado.asentamientos.insert(id, a);
        Some(r)
    }

    /// El paso de un minuto de la plaza `id` en `t` (el evento `Carga::Minuto`).
    pub(super) fn minuto_de_plaza(&mut self, id: u64, t: Instante) {
        if !self.estado.asentamientos.contains_key(&id) {
            return;
        }
        self.programar_minuto(id, t + MS_POR_MINUTO);
        let Some(sucesos) = self.con_exterior(id, t, |eco, x| eco.avanzar_minuto(t, x)) else { return };
        let ruinas = sucesos.iter().any(|e| matches!(e, EventoEco::Ruinas { .. }));
        self.anotar_sucesos(id, t, &sucesos);
        if ruinas {
            self.arruinar(id);
            return;
        }
        // La zona crece 1 unidad por edificio completado, hasta el techo del nivel (Economia_Construccion §3.1).
        let completados = self.estado.asentamientos.get_mut(&id).map_or(0, |a| a.economia.tomar_completados());
        if completados > 0 {
            self.agrandar_zona(id, completados as i64, t);
        }
    }

    /// Traslada los sucesos de la economía a eventos del núcleo y marca la plaza como tocada (cierra versión).
    fn anotar_sucesos(&mut self, id: u64, t: Instante, sucesos: &[EventoEco]) {
        if sucesos.is_empty() {
            return;
        }
        let Some(a) = self.estado.asentamientos.get(&id) else { return };
        let (punto, reglas) = (a.posicion, &self.estado.reglas.economia.datos);
        self.tr.eventos.extend(sucesos.iter().map(|e| evento_nucleo(id, t, punto, e, reglas)));
        self.tr.asentamientos.push(id);
    }

    /// Una plaza cayó en ruinas: desaparece del estado y libera sus parcelas y nodos. Sube su generación (descarta el paso de minuto ya
    /// programado) y el evento `asentamiento.ruinas` ya se anotó.
    fn arruinar(&mut self, id: u64) {
        let Some(a) = self.estado.asentamientos.remove(&id) else { return };
        for p in &a.parcelas {
            self.estado.parcelas_ocupadas.remove(p);
        }
        for e in &a.economia.edificios {
            if let Some(Fuente::Nodo(n)) = e.fuente {
                self.estado.nodos_reclamados.remove(&n);
            }
        }
        self.estado.contactos.retain(|&(x, y)| x != id && y != id);
        self.estado.generaciones.subir(id);
        // Gancho: la cascada completa de una ruina (residentes reubicados, caravanas y trueques, campamento de bandidos, Facción
        // derrotada si era la última) es de otros sistemas y se ejecutará aquí, en este orden y en este instante.
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// ¿Puede esta petición mandar en la plaza? Hoy: administración o propietario de alguna columna de la Facción de la plaza. DUDA: los
    /// cargos (Gobernador, Maestro de Obras, Tesorero), la residencia y la presencia vendrán del sistema de héroes y residentes; hasta
    /// entonces esto es lo más simple y conservador (como `Fundar`, que exige ser el propietario de la columna).
    fn manda_en(&self, p: &Peticion, faccion: u32) -> bool {
        p.admin || self.estado.ejercitos.values().any(|e| e.faccion == faccion && e.propietario == p.actor)
    }

    /// Plazas de la Facción (salvo `id`) que ocupan un puesto en el nivel `objetivo`: lo tienen como nivel actual o van hacia él.
    fn cupo_ocupado(&self, id: u64, faccion: u32, objetivo: u32) -> u32 {
        self.plazas_de(faccion).filter(|o| o.id != id && (o.nivel_actual() == objetivo || o.economia.ascenso.is_some_and(|s| s.nivel_objetivo == objetivo))).count() as u32
    }

    /// Libera la parcela o el nodo que tenía tomado un edificio que desaparece de la cola.
    fn liberar_fuente(&mut self, id: u64, fuente: Option<Fuente>) {
        match fuente {
            Some(Fuente::Parcela(p)) => {
                self.estado.parcelas_ocupadas.remove(&p);
                if let Some(a) = self.estado.asentamientos.get_mut(&id) {
                    a.parcelas.remove(&p);
                }
            }
            Some(Fuente::Nodo(n)) => {
                self.estado.nodos_reclamados.remove(&n);
            }
            None => {}
        }
    }

    /// Ejecuta las órdenes de economía y construcción. `Ok(Some(n))` = cambió algo y creó el edificio `n`; `Ok(None)` = cambió algo;
    /// rechazos y «sin cambios» salen como `Err` sin haber tocado el estado.
    pub(super) fn ejecutar_economia(&mut self, t: Instante, p: &Peticion) -> Result<Option<u64>, Respuesta> {
        let v = self.estado.version;
        let rechazo = |c: &'static str, m: String| Respuesta::rechazo(c, m, v);
        let sin_cambios = || Respuesta { resultado: Resultado::SinCambios, version_tras: v, creadas: vec![] };
        let (Comando::AnadirEdificio { asentamiento, .. }
        | Comando::QuitarDeCola { asentamiento, .. }
        | Comando::MoverEnCola { asentamiento, .. }
        | Comando::MejorarEdificio { asentamiento, .. }
        | Comando::AlternarAutoConstruccion { asentamiento, .. }
        | Comando::AlternarReceta { asentamiento, .. }
        | Comando::CalibrarReserva { asentamiento, .. }
        | Comando::SolicitarAscenso { asentamiento }
        | Comando::ActivarPolitica { asentamiento, .. }) = &p.comando
        else {
            unreachable!("solo se llama con órdenes de economía")
        };
        let id = *asentamiento;
        let Some(plaza) = self.estado.asentamientos.get(&id) else { return Err(rechazo(rechazos::ASENTAMIENTO_DESCONOCIDO, format!("as-{id}"))) };
        let faccion = plaza.faccion;
        if !self.manda_en(p, faccion) {
            return Err(rechazo(rechazos::NO_AUTORIZADO, format!("no mandas en as-{id}")));
        }
        let datos = self.estado.reglas.economia.datos.clone();
        let rechazo_obra = |e: ErrorObra| {
            let (c, m) = rechazo_de_obra(e);
            rechazo(c, m.into())
        };
        let mut creada = None;
        let mut sucesos: Vec<EventoEco> = vec![];
        match &p.comando {
            Comando::AnadirEdificio { tipo, .. } => {
                let nuevo = self.con_exterior(id, t, |eco, x| eco.anadir_manual(x.reglas, x, *tipo)).expect("la plaza existe y hay mundo");
                let eid = nuevo.map_err(rechazo_obra)?;
                creada = Some(eid as u64);
                sucesos.push(EventoEco::Encolada { id: eid, tipo: *tipo });
            }
            Comando::QuitarDeCola { edificio, .. } => {
                let eco = &mut self.estado.asentamientos.get_mut(&id).expect("existe").economia;
                let fuente = eco.edificio(*edificio).and_then(|e| e.fuente);
                eco.quitar_de_cola(&datos, *edificio).map_err(rechazo_obra)?;
                self.liberar_fuente(id, fuente);
                sucesos.push(EventoEco::Desencolada { id: *edificio });
            }
            Comando::MoverEnCola { edificio, arriba, .. } => {
                let eco = &mut self.estado.asentamientos.get_mut(&id).expect("existe").economia;
                let antes: Vec<i64> = eco.edificios.iter().map(|e| e.prioridad).collect();
                eco.mover_en_cola(*edificio, *arriba).map_err(rechazo_obra)?;
                if eco.edificios.iter().map(|e| e.prioridad).eq(antes) {
                    return Err(sin_cambios());
                }
                sucesos.push(EventoEco::ColaReordenada);
            }
            Comando::MejorarEdificio { edificio, .. } => {
                let r = self.con_exterior(id, t, |eco, x| eco.mejorar_ahora(t, x.reglas, x, *edificio)).expect("la plaza existe y hay mundo");
                r.map_err(rechazo_obra)?;
                let nivel = self.estado.asentamientos[&id].economia.edificio(*edificio).and_then(|e| e.mejora).map_or(0, |m| m.nivel_objetivo);
                sucesos.push(EventoEco::MejoraIniciada { id: *edificio, nivel });
            }
            Comando::AlternarAutoConstruccion { pausada, .. } => {
                if !self.estado.asentamientos.get_mut(&id).expect("existe").economia.alternar_auto_construccion(*pausada) {
                    return Err(sin_cambios());
                }
            }
            Comando::AlternarReceta { recurso, pausada, .. } => {
                if !self.estado.asentamientos.get_mut(&id).expect("existe").economia.alternar_receta(*recurso, *pausada) {
                    return Err(sin_cambios());
                }
            }
            Comando::CalibrarReserva { recurso, valor, .. } => {
                let eco = &mut self.estado.asentamientos.get_mut(&id).expect("existe").economia;
                let antes = eco.reserva_manual[recurso.indice()];
                eco.calibrar_reserva(&datos, *recurso, *valor);
                if eco.reserva_manual[recurso.indice()] == antes {
                    return Err(sin_cambios());
                }
            }
            Comando::SolicitarAscenso { .. } => {
                let objetivo = self.estado.asentamientos[&id].nivel() + 1;
                let cupo = self.cupo_ocupado(id, faccion, objetivo);
                let (r, ev) = self
                    .con_exterior(id, t, |eco, x| {
                        let mut ev = vec![];
                        (eco.solicitar_ascenso(t, x.reglas, x, cupo, &[], &mut ev), ev)
                    })
                    .expect("la plaza existe y hay mundo");
                if let Err(bloqueos) = r {
                    let (codigo, _) = rechazo_de_bloqueo(&bloqueos[0]);
                    let todos: Vec<String> = bloqueos.iter().map(|b| rechazo_de_bloqueo(b).1).collect();
                    return Err(rechazo(codigo, format!("no se puede ascender: {}", todos.join("; "))));
                }
                sucesos.extend(ev);
            }
            Comando::ActivarPolitica { politica, cargo, .. } => {
                let nivel_faccion = self.estado.niveles_faccion.get(&faccion).copied().unwrap_or(1);
                let eco = &mut self.estado.asentamientos.get_mut(&id).expect("existe").economia;
                // DUDA: sin sistema de cargos, se da por ocupado el cargo que invoca la orden (ya se comprobó que el actor manda en la plaza).
                let activador = Activador { cargo: *cargo, cargo_ocupado: true, nivel_faccion, sala_consejo: eco.edificios.activos(Tipo::SalaConsejo) > 0 };
                let expira = eco.politicas.activar(&datos.politicas, politica, &activador, t).map_err(|e| {
                    let (c, m) = rechazo_de_politica(e);
                    rechazo(c, m.into())
                })?;
                let punto = self.estado.asentamientos[&id].posicion;
                let indice = datos.politicas.catalogo.iter().position(|q| &q.id == politica).map_or(-1, |i| i as i64);
                self.tr.eventos.push(EventoNucleo { codigo: "politica.activada".into(), en: t, ejercitos: vec![], asentamientos: vec![id], punto: Some(punto), datos: vec![("politica", indice), ("expiraEn", expira)] });
            }
            _ => unreachable!("solo se llama con órdenes de economía"),
        }
        self.anotar_sucesos(id, t, &sucesos);
        self.tr.asentamientos.push(id);
        Ok(creada)
    }
}
