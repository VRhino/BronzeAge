//! La economía de las plazas dentro del núcleo (Economia_Construccion): el paso de **un minuto de mundo** de cada plaza y las órdenes
//! sobre su cola de obras, mejoras, ascenso y políticas. Las reglas puras viven en el crate `asentamiento`; aquí está el reloj, el
//! mundo que ve (yacimientos, parcelas, zona) y el traslado de sus sucesos a eventos y versiones del núcleo.
//!
//! # El reloj de una plaza
//! Cada plaza tiene un evento recurrente `Carga::Minuto` con la fase de su fundación (los pasos caen en `fundado_en + k·60 s`), se mire
//! o no; no hay tick global. Prioridad [`PRIO_ASENTAMIENTO`]. Cada paso programa el siguiente **antes** de avanzar: si la plaza cae en
//! ruinas, subir su generación descarta ese evento sin que la plaza exista ya.
//!
//! # Lo que no espera al paso (2026-10-22)
//! La economía va por minuto por diseño (producción, consumo, recaudación, mantenimiento, población), pero lo que el jugador nota va en su
//! instante exacto, con una cita propia en la cola y la generación de la plaza (la ruina las descarta): `Carga::FinDeObras` por cada obra,
//! mejora y ascenso en curso (prioridad [`PRIO_FIN_DE_OBRAS`], antes del paso del mismo instante) y `Carga::CaducaPolitica` por cada política
//! activa (prioridad [`PRIO_CADUCA_POLITICA`], después del paso). Se programan al empezar lo que esperan (en `anotar_sucesos`, a partir de
//! `ObraIniciada`, `MejoraIniciada` y `AscensoIniciado`) y desaparecen al cumplirse: ninguna es recurrente. La obra de una avanzada tiene la suya, ligada a la avanzada y no a la
//! plaza (`SucesoCombate::FinObraAvanzada`), para poder cancelarla. Al acabar una obra la zona crece y la
//! cuadrilla se libera en ese mismo instante, y la siguiente de la cola arranca entonces; una obra encargada a mano arranca al instante de la orden.
//! Una cita que llega con la plaza ya cambiada (una conquista descartó el ascenso) no encuentra nada que cumplir.
//!
//! # Política de versión
//! La población, el almacén y el medidor cambian en cada paso: son **ritmos**, cambian el hash pero no la versión. El paso cierra
//! versión (y marca la plaza como tocada) solo si dejó algo discreto: algún suceso del crate (obra, mejora, ascenso, política,
//! hambruna, déficit...) o un cambio de la zona. Un paso sin novedades no sube la versión, **con o sin sesiones**: lo que el paso
//! cambia dentro de la plaza (almacén, población, medidor, nutrición, edificios parados, tendencia y almacén local de sus pueblos) lleva su
//! propia secuencia por plaza, [`Nucleo::version_interior`] (4H T3c, decisión del autor 2026-10-23), que el actor de sesiones compara con la
//! que tiene cada cliente; el contador `version` y el hash del mundo no dependen de si alguien escucha.

use super::*;
use crate::codec_economia::{recurso_a_u8, tipo_a_u8};
use crate::economia::{FuentesMundo, SitiosPlaza};
use crate::estado::{SucesoCombate, PRIO_ASENTAMIENTO, PRIO_CADUCA_POLITICA, PRIO_FIN_DE_OBRAS};
use crate::estado_combate::entidad_obra_de_avanzada;
use base::MS_POR_MINUTO;
use asentamiento::asentamiento::{Evento as EventoEco, Exterior, Tendencia};
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
// Las tecnologías de la Facción las da la administración (`FijarTecnologias`) hasta el sistema de tecnología; sin ellas, sin bonus.
const SIN_BONUS: fn(Recurso) -> i64 = |_| UNO;
// placeholder: lo dará el sistema de Eras del servidor.
const ORDEN_ERA: u32 = 1;
// placeholder: lo darán la ocupación/Aedas (factores extra). La ración de la guarnición es la de los residentes (`heroes`).
const FACTOR_EXTRA: i64 = UNO;

/// Lo que del interior de una plaza cambia con el paso de minuto y es la base de la tendencia «+N/min»: el almacén **exacto** (con su resto oculto: el neto real del
/// flujo, aunque lo visible vaya en unidades enteras) y la población entera (sus restos no viajan).
struct Contadores {
    almacen: [Mil; 29],
    poblacion: [i64; 3],
}

impl Contadores {
    fn de(e: &asentamiento::asentamiento::Asentamiento) -> Self {
        let (pesants, artesanos, nobleza) = e.poblacion.enteros();
        Contadores { almacen: asentamiento::recurso::TODOS.map(|r| e.almacen.disponible(r)), poblacion: [pesants, artesanos, nobleza] }
    }

    /// Lo que cambió desde `antes`: el neto de un paso de minuto.
    fn tendencia(&self, antes: &Contadores) -> Tendencia {
        let mut t = Tendencia::default();
        t.almacen.iter_mut().zip(self.almacen.iter().zip(&antes.almacen)).for_each(|(x, (ahora, antes))| *x = ahora - antes);
        t.poblacion.iter_mut().zip(self.poblacion.iter().zip(&antes.poblacion)).for_each(|(x, (ahora, antes))| *x = ahora - antes);
        t
    }
}

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
        ErrorObra::SoloPorPueblo => (OBRA_SOLO_POR_PUEBLO, "los minerales y toda Cantera tras la primera solo se explotan con un pueblo extractor"),
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
        EventoEco::EdificioCompletado { id, tipo, nivel } => ("construccion.edificio_completado", vec![("edificio", *id as i64), ("tipo", tipo_a_u8(*tipo) as i64), ("nivel", i64::from(*nivel))]),
        EventoEco::EdificioGratuito { id, tipo, nivel } => ("construccion.edificio_gratuito", vec![("edificio", *id as i64), ("tipo", tipo_a_u8(*tipo) as i64), ("nivel", *nivel as i64)]),
        EventoEco::ObraIniciada { id, tipo, completa_en } => ("construccion.obra_iniciada", vec![("edificio", *id as i64), ("tipo", tipo_a_u8(*tipo) as i64), ("completaEn", *completa_en)]),
        EventoEco::MejoraIniciada { id, nivel, completa_en } => ("construccion.mejora_iniciada", vec![("edificio", *id as i64), ("nivel", *nivel as i64), ("completaEn", *completa_en)]),
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
        EventoEco::AscensoIniciado { objetivo, completa_en } => ("asentamiento.ascenso_iniciado", vec![("objetivo", *objetivo as i64), ("completaEn", *completa_en)]),
        EventoEco::NivelSubio { nivel } => ("asentamiento.nivel_subio", vec![("nivel", *nivel as i64)]),
        EventoEco::PuebloMarcado { id, tipo, nodo } => ("pueblo.marcado", vec![("pueblo", *id as i64), ("tipo", tipo_a_u8(*tipo) as i64), ("nodo", *nodo as i64)]),
        EventoEco::PuebloEntrega { id, cantidad } => ("pueblo.materiales_entregados", vec![("pueblo", *id as i64), ("cantidad", *cantidad)]),
        EventoEco::PuebloObraIniciada { id, completa_en } => ("pueblo.obra_iniciada", vec![("pueblo", *id as i64), ("completaEn", *completa_en)]),
        EventoEco::PuebloCompletado { id } => ("pueblo.completado", vec![("pueblo", *id as i64)]),
        EventoEco::PuebloLleno { id } => ("pueblo.almacen_lleno", vec![("pueblo", *id as i64)]),
        EventoEco::PuebloDestruido { id } => ("pueblo.destruido", vec![("pueblo", *id as i64)]),
        EventoEco::AvanzadaObraMarcada { id, tipo, posicion } => ("avanzada.obra_marcada", vec![("avanzada", *id as i64), ("tipo", *tipo as i64), ("x", posicion.x), ("y", posicion.y)]),
        EventoEco::AvanzadaEntrega { id, cantidad } => ("avanzada.materiales_entregados", vec![("avanzada", *id as i64), ("cantidad", *cantidad)]),
        EventoEco::AvanzadaObraIniciada { id, tipo, completa_en } => ("avanzada.obra_iniciada", vec![("avanzada", *id as i64), ("tipo", *tipo as i64), ("completaEn", *completa_en)]),
        EventoEco::AvanzadaCompletada { id, tipo, etapa } => ("avanzada.completada", vec![("avanzada", *id as i64), ("tipo", *tipo as i64), ("etapa", etapa.indice() as i64)]),
        EventoEco::AvanzadaDestruida { id } => ("avanzada.destruida", vec![("avanzada", *id as i64), ("motivo", 0)]),
        EventoEco::AvanzadaObraCancelada { id, tipo } => ("avanzada.obra_cancelada", vec![("avanzada", *id as i64), ("tipo", *tipo as i64)]),
        EventoEco::RecintoComprometido { recinto, nivel, celdas, puertas } => ("construccion.recinto_comprometido", vec![("recinto", *recinto as i64), ("nivel", *nivel as i64), ("celdas", *celdas as i64), ("puertas", *puertas as i64)]),
        EventoEco::RecintoMejoraIniciada { recinto, nivel } => ("construccion.recinto_mejora_iniciada", vec![("recinto", *recinto as i64), ("nivel", *nivel as i64)]),
        EventoEco::RecintoAbandonado { recinto } => ("construccion.recinto_abandonado", vec![("recinto", *recinto as i64)]),
        EventoEco::RecintoCelda { recinto, levantadas, siguiente_en } => ("construccion.recinto_celda", vec![("recinto", *recinto as i64), ("levantadas", *levantadas as i64), ("siguienteEn", siguiente_en.unwrap_or(-1))]),
        EventoEco::RecintoCompletado { recinto, nivel, celdas, puertas } => ("construccion.recinto_completado", vec![("recinto", *recinto as i64), ("nivel", *nivel as i64), ("celdas", *celdas as i64), ("puertas", *puertas as i64)]),
        EventoEco::RecintoMejorado { recinto, nivel, celdas, puertas } => ("construccion.recinto_mejorado", vec![("recinto", *recinto as i64), ("nivel", *nivel as i64), ("celdas", *celdas as i64), ("puertas", *puertas as i64)]),
    };
    EventoNucleo { codigo: codigo.into(), en: t, ejercitos: vec![], asentamientos: vec![id], punto: Some(punto), datos, ..Default::default() }
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

    /// Programa la cita de fin de la obra de la avanzada `id` en `en`, con la generación vigente de su obra: cancelarla la sube y la descarta (prioridad de fin de obras).
    fn programar_fin_de_avanzada(&mut self, id: u64, en: Instante) {
        let clave = self.estado.generaciones.clave(entidad_obra_de_avanzada(id));
        self.estado.cola.programar(en, PRIO_FIN_DE_OBRAS, clave, Carga::Combate(SucesoCombate::FinObraAvanzada));
    }

    /// Programa la cita de fin de obra, mejora o ascenso de la plaza en `en`, con su generación vigente.
    fn programar_fin(&mut self, id: u64, en: Instante) {
        let clave = self.estado.generaciones.clave(id);
        self.estado.cola.programar(en, PRIO_FIN_DE_OBRAS, clave, Carga::FinDeObras);
    }

    /// Los «ciudadanos en el asentamiento» que pide la nobleza (`nobleza.minCiudadanos`): **Q8** (2026-10-24), todos los residentes de la plaza, fundadores y quienes se mudaron. Una consulta
    /// al índice de residentes.
    pub fn ciudadanos_de(&self, plaza: u64) -> u32 {
        self.residentes_en_el_paso(plaza).1
    }

    /// Saca la plaza del mapa mientras se trabaja con ella (así el resto de plazas se puede leer: las vecinas que recortan su zona),
    /// le monta el `Exterior` y ejecuta `f`. La vuelve a insertar siempre. `None` si no hay plaza o no hay mundo.
    pub(super) fn con_exterior<R>(&mut self, id: u64, t: Instante, f: impl FnOnce(&mut asentamiento::asentamiento::Asentamiento, &mut Exterior) -> R) -> Option<R> {
        let geo = self.geografia.as_ref()?;
        let (racion_guarnicion, ciudadanos) = self.residentes_en_el_paso(id);
        // La capital de su Facción y el nivel de esta (5C), mirados antes de sacar la plaza del mapa.
        if !self.estado.asentamientos.contains_key(&id) {
            return None;
        }
        let nivel_faccion = self.nivel_de_faccion(self.estado.asentamientos[&id].faccion);
        let (es_capital, distancia_capital, faccion_tiene_gran_fundicion) = self.exterior_de_capital(id, nivel_faccion);
        let mut a = self.estado.asentamientos.remove(&id)?;
        let r = {
            let Estado { reglas, asentamientos, parcelas_ocupadas, nodos_reclamados, yacimientos, contactos, militar, .. } = &mut self.estado;
            let faccion = a.faccion;
            let tiene_tecnologia = |t: &str| militar.tiene_tecnologia(&reglas.economia.militar, faccion, t);
            let mut fuentes = FuentesMundo { mapa: &geo.mapa, nodos: yacimientos, ahora: t };
            let mut sitios = SitiosPlaza {
                mapa: &geo.mapa,
                cuota_pct: reglas.territorio.cuota_parcelas_pct,
                id,
                yo: Disco { centro: a.posicion, radio: a.radio, faccion: a.faccion },
                parcelas: &mut a.parcelas,
                urbe: &mut a.urbe,
                urbanismo: &reglas.economia.urbanismo,
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
                tiene_tecnologia: &tiene_tecnologia,
                factor_tecnologia: &SIN_BONUS,
                nivel_faccion,
                orden_era: ORDEN_ERA,
                es_capital,
                faccion_tiene_gran_fundicion,
                ciudadanos,
                distancia_capital,
                racion_guarnicion,
                // Una plaza ocupada tras una conquista crece y recauda a la fracción de la ocupación (Combate §8): su mantenimiento ya lo congela `ocupado`.
                felicidad_extra: if a.ocupada_hasta.is_some() { reglas.economia.combate.ocupacion.factor_crecimiento } else { FACTOR_EXTRA },
                nobleza_extra: FACTOR_EXTRA,
                recaudacion_extra: if a.ocupada_hasta.is_some() { reglas.economia.combate.ocupacion.factor_recaudacion } else { FACTOR_EXTRA },
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
        // La tendencia «+N/min» es lo que cambió el almacén y la población en el paso entero (el de la guarnición, que come después, se le suma en `alimentar_guarnicion`).
        let Some((sucesos, racion)) = self.con_exterior(id, t, |eco, x| {
            let antes = Contadores::de(eco);
            let sucesos = eco.avanzar_minuto(t, x);
            eco.tendencia = Contadores::de(eco).tendencia(&antes);
            (sucesos, x.racion_guarnicion)
        }) else {
            return;
        };
        let ruinas = sucesos.iter().any(|e| matches!(e, EventoEco::Ruinas { .. }));
        self.anotar_sucesos(id, t, &sucesos);
        if ruinas {
            self.arruinar(id, t);
            return;
        }
        for s in &sucesos {
            match s {
                EventoEco::PuebloCompletado { id: pueblo } => self.dar_carro_a_pueblo(id, *pueblo),
                EventoEco::PuebloLleno { id: pueblo } => self.salir_acarreo(id, *pueblo, t),
                _ => {}
            }
        }
        // La guarnición come después de la población (Reloj_Y_Eventos §2), la ración de este mismo paso.
        self.alimentar_guarnicion(id, t, racion);
    }

    /// Acaba una obra, una mejora o un ascenso de la plaza `id` en `t` (el evento `Carga::FinDeObras`): el edificio pasa a activo, la zona crece
    /// uno por cada edificio completado y la cuadrilla liberada arranca la siguiente obra de la cola en este mismo instante.
    pub(super) fn fines_de_plaza(&mut self, id: u64, t: Instante) {
        let Some(sucesos) = self.con_exterior(id, t, |eco, x| eco.cumplir_fines(t, x)) else { return };
        self.anotar_sucesos(id, t, &sucesos);
        self.avanzada_completada(id, &sucesos, t);
        // La zona crece 1 unidad por edificio completado, hasta el techo del nivel (Economia_Construccion §3.1).
        let completados = self.estado.asentamientos.get_mut(&id).map_or(0, |a| a.economia.tomar_completados());
        if completados > 0 {
            self.agrandar_zona(id, completados as i64, t);
        }
    }

    /// Caduca una política de la plaza `id` en `t` (el evento `Carga::CaducaPolitica`).
    pub(super) fn caduca_politica(&mut self, id: u64, t: Instante) {
        let Some(a) = self.estado.asentamientos.get_mut(&id) else { return };
        let sucesos = a.economia.caducar_politicas(t);
        self.anotar_sucesos(id, t, &sucesos);
    }

    /// Traslada los sucesos de la economía a eventos del núcleo y marca la plaza como tocada (cierra versión). Programa también la cita de fin
    /// de cada obra, mejora y ascenso que acaba de empezar.
    pub(super) fn anotar_sucesos(&mut self, id: u64, t: Instante, sucesos: &[EventoEco]) {
        if sucesos.is_empty() {
            return;
        }
        for s in sucesos {
            match s {
                EventoEco::ObraIniciada { completa_en, .. } | EventoEco::MejoraIniciada { completa_en, .. } | EventoEco::AscensoIniciado { completa_en, .. } => self.programar_fin(id, *completa_en),
                EventoEco::AvanzadaObraIniciada { id: avanzada, completa_en, .. } => {
                    self.programar_fin_de_avanzada(*avanzada, *completa_en);
                    self.tr.tocar(Clase::Avanzada, *avanzada);
                }
                // Cada celda de una muralla deja la cita de la siguiente (la obra de un recinto no ocupa cuadrilla ni es recurrente).
                EventoEco::RecintoCelda { siguiente_en: Some(en), .. } => self.programar_fin(id, *en),
                EventoEco::PuebloMarcado { id: pueblo, .. }
                | EventoEco::PuebloEntrega { id: pueblo, .. }
                | EventoEco::PuebloObraIniciada { id: pueblo, .. }
                | EventoEco::PuebloCompletado { id: pueblo }
                | EventoEco::PuebloLleno { id: pueblo }
                | EventoEco::PuebloDestruido { id: pueblo } => self.tr.tocar(Clase::Pueblo, *pueblo),
                EventoEco::AvanzadaObraMarcada { id: avanzada, .. }
                | EventoEco::AvanzadaEntrega { id: avanzada, .. }
                | EventoEco::AvanzadaCompletada { id: avanzada, .. }
                | EventoEco::AvanzadaDestruida { id: avanzada }
                | EventoEco::AvanzadaObraCancelada { id: avanzada, .. } => self.tr.tocar(Clase::Avanzada, *avanzada),
                _ => {}
            }
        }
        // La ciudad guarda una proyección de los recintos: se rehace cuando el anillo cambia (al comprometer, abandonar o cerrarse una celda final).
        if sucesos.iter().any(|s| matches!(s, EventoEco::RecintoComprometido { .. } | EventoEco::RecintoAbandonado { .. } | EventoEco::RecintoCelda { .. })) {
            self.sincronizar_recintos(id);
        }
        let Some(a) = self.estado.asentamientos.get(&id) else { return };
        let (punto, reglas, faccion) = (a.posicion, &self.estado.reglas.economia.datos, a.faccion);
        for e in sucesos {
            self.tr.emitir(&self.estado, evento_nucleo(id, t, punto, e, reglas));
        }
        self.tr.tocar(Clase::Asentamiento, id);
        // La experiencia de Facción por construir y por ascender (§3.5), en el instante en que acaba la obra.
        for s in sucesos {
            let hecho = match s {
                EventoEco::EdificioCompletado { .. } => (politica::nivel::Hecho::EdificioCompletado, super::experiencia::motivo::EDIFICIO),
                &EventoEco::NivelSubio { nivel } => (politica::nivel::Hecho::Ascenso { nivel }, super::experiencia::motivo::ASCENSO),
                _ => continue,
            };
            self.dar_experiencia_por(t, faccion, hecho.0, hecho.1);
        }
    }

    /// Una plaza cayó en ruinas: desaparece del estado y libera sus parcelas y nodos. Sube su generación (descarta el paso de minuto ya
    /// programado) y el evento `asentamiento.ruinas` ya se anotó.
    fn arruinar(&mut self, id: u64, t: Instante) {
        let Some(a) = self.estado.asentamientos.remove(&id) else { return };
        // Los hechos de la cascada que se atribuyen a la plaza (sus caravanas perdidas, su campamento de bandidos...) siguen llegando a su Facción.
        self.tr.arruinadas.push((id, a.faccion));
        for p in &a.parcelas {
            self.estado.parcelas_ocupadas.remove(p);
        }
        for e in &a.economia.edificios {
            if let Some(Fuente::Nodo(n)) = e.fuente {
                self.estado.nodos_reclamados.remove(&n);
            }
        }
        // Los pueblos desaparecen con su madre (Doc 4.2.2); sus carros, que la tienen por origen, se pierden en la cascada de comercio. Con las avanzadas, la atalaya y el
        // fuerte desaparecen y la ciudadela queda como ruina sin dueño (Doc 5.17).
        for p in &a.economia.pueblos {
            self.estado.nodos_reclamados.remove(&p.nodo);
            self.tr.tocar(Clase::Pueblo, p.id);
        }
        // Quien estaba dentro de sus pueblos o avanzadas sale al mapa con lo suyo, a la puerta de cada uno.
        let nodos = self.geografia.as_ref().map_or(&[][..], |g| &g.mapa.nodos[..]);
        let pueblos: Vec<(u64, Punto)> = a.economia.pueblos.iter().filter_map(|p| nodos.get(p.nodo as usize).map(|n| (p.id, n.pos))).collect();
        let avanzadas: Vec<(u64, Punto)> = a.economia.avanzadas.iter().map(|v| (v.id, v.posicion)).collect();
        self.vaciar_pueblos_y_avanzadas(t, id, &pueblos, &avanzadas);
        self.avanzadas_de_la_madre_arruinada(a.economia.avanzadas, t);
        self.estado.contactos.retain(|&(x, y)| x != id && y != id);
        self.quitar_plaza(id, a.faccion);
        self.estado.generaciones.subir(id);
        // La cascada de la ruina, en su orden: los residentes se reubican; después, el comercio (trueques anulados sin penalizar, caravanas perdidas
        // o devueltas y órdenes de mercado borradas); su campamento de bandidos desaparece y, si era la última plaza de su Facción, esta queda derrotada
        // sin que nadie la derrotara.
        self.reubicar_residentes(id, a.posicion, t);
        self.cerrar_comercio(id, comercio::ruina::Motivo::Ruina, t);
        self.bandidos_de_plaza_arruinada(id, t);
        // La capital de su Facción puede cambiar (la Facción cambia); sin plazas, queda derrotada por colapso (`derrotadaPor` = nadie, §8).
        self.tr.tocar(Clase::Faccion, u64::from(a.faccion));
        if combate::conquista::faccion_derrotada(self.plazas_de(a.faccion).count()) {
            self.registrar_derrota(a.faccion, None);
            self.emitir_propio(t, "faccion.derrotada", vec![], vec![("faccion", a.faccion as i64), ("por", 0)]);
            self.reprogramar_anillos(t);
        }
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// ¿Puede esta petición mandar en la plaza? Hoy: administración, propietario de alguna columna de la Facción de la plaza o cuenta de algún
    /// héroe ciudadano de esa Facción (`FijarFaccionDeHeroe`). DUDA: los
    /// cargos (Gobernador, Maestro de Obras, Tesorero), la residencia y la presencia vendrán del sistema de héroes y residentes; hasta
    /// entonces esto es lo más simple y conservador (como `Fundar`, que exige ser el propietario de la columna).
    pub(super) fn manda_en(&self, p: &Peticion, faccion: u32) -> bool {
        p.admin || self.estado.ejercitos.values().any(|e| e.columna.faccion == Some(faccion) && self.manda_columna(p, e, false)) || self.heroes().any(|h| h.faccion == Some(faccion) && h.cuenta == p.actor)
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
        let sin_cambios = || Respuesta::sin_cambios(v);
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
        let Some(plaza) = self.estado.asentamientos.get(&id) else { return Err(rechazo(rechazos::ASENTAMIENTO_DESCONOCIDO, ids::asentamiento(id))) };
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
                let (nuevo, eventos) = self
                    .con_exterior(id, t, |eco, x| {
                        let mut ev = vec![];
                        (eco.anadir_manual(t, x.reglas, x, *tipo, &mut ev), ev)
                    })
                    .expect("la plaza existe y hay mundo");
                let eid = nuevo.map_err(rechazo_obra)?;
                creada = Some(eid as u64);
                sucesos.extend(eventos); // las anclas que el trazado creó antes de colocarlo, la orden encolada y, si había cuadrilla, la obra iniciada
            }
            Comando::QuitarDeCola { edificio, .. } => {
                let plaza = self.estado.asentamientos.get_mut(&id).expect("existe");
                let fuente = plaza.economia.edificio(*edificio).and_then(|e| e.fuente);
                plaza.economia.quitar_de_cola(&datos, *edificio).map_err(rechazo_obra)?;
                plaza.urbe.quitar(*edificio); // su pieza deja de ocupar suelo
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
                let (r, ev) = self
                    .con_exterior(id, t, |eco, x| {
                        let mut ev = vec![];
                        (eco.mejorar_ahora(t, x.reglas, x, *edificio, &mut ev), ev)
                    })
                    .expect("la plaza existe y hay mundo");
                r.map_err(rechazo_obra)?;
                self.emitir_propio(t, "construccion.mejora_forzada", vec![id], vec![("edificio", *edificio as i64)]);
                sucesos.extend(ev);
            }
            Comando::AlternarAutoConstruccion { pausada, .. } => {
                if !self.estado.asentamientos.get_mut(&id).expect("existe").economia.alternar_auto_construccion(*pausada) {
                    return Err(sin_cambios());
                }
                self.emitir_propio(t, "construccion.auto_alternada", vec![id], vec![("pausada", *pausada as i64)]);
            }
            Comando::AlternarReceta { recurso, pausada, .. } => {
                if !self.estado.asentamientos.get_mut(&id).expect("existe").economia.alternar_receta(*recurso, *pausada) {
                    return Err(sin_cambios());
                }
                self.emitir_propio(t, "construccion.receta_alternada", vec![id], vec![("recurso", recurso_a_u8(*recurso) as i64), ("pausada", *pausada as i64)]);
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
                // El cupo de la Facción en el nivel objetivo (`CUPO_NIVEL_ASENTAMIENTO`, por el nivel de Facción: `politica`, 5C).
                let cupo = politica::nivel::cupo_nivel(&self.estado.reglas.economia.politica.nivel, self.nivel_de_faccion(faccion), objetivo);
                let sin_cupo = cupo.is_some_and(|c| self.cupo_ocupado(id, faccion, objetivo) >= c);
                let (r, ev) = self
                    .con_exterior(id, t, |eco, x| {
                        let mut ev = vec![];
                        (eco.solicitar_ascenso(t, x.reglas, x, sin_cupo, &mut ev), ev)
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
                let nivel_faccion = self.nivel_de_faccion(faccion);
                let eco = &mut self.estado.asentamientos.get_mut(&id).expect("existe").economia;
                // DUDA: sin sistema de cargos, se da por ocupado el cargo que invoca la orden (ya se comprobó que el actor manda en la plaza).
                let activador = Activador { cargo: *cargo, cargo_ocupado: true, nivel_faccion, sala_consejo: eco.edificios.activos(Tipo::SalaConsejo) > 0 };
                let expira = eco.politicas.activar(&datos.politicas, politica, &activador, t).map_err(|e| {
                    let (c, m) = rechazo_de_politica(e);
                    rechazo(c, m.into())
                })?;
                let punto = self.estado.asentamientos[&id].posicion;
                let indice = datos.politicas.catalogo.iter().position(|q| &q.id == politica).map_or(-1, |i| i as i64);
                self.tr.emitir(&self.estado, EventoNucleo { codigo: "politica.activada".into(), en: t, ejercitos: vec![], asentamientos: vec![id], punto: Some(punto), datos: vec![("politica", indice), ("expiraEn", expira)], ..Default::default() });
                let clave = self.estado.generaciones.clave(id);
                self.estado.cola.programar(expira, PRIO_CADUCA_POLITICA, clave, Carga::CaducaPolitica);
            }
            _ => unreachable!("solo se llama con órdenes de economía"),
        }
        self.anotar_sucesos(id, t, &sucesos);
        self.tr.tocar(Clase::Asentamiento, id);
        Ok(creada)
    }
}
