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
use asentamiento::asentamiento::{AvisoSolvencia, Evento as EventoEco, Exterior, Tendencia};
use asentamiento::ascenso::Bloqueo;
use asentamiento::catalogo::Tipo;
use asentamiento::datos::UNO;
use asentamiento::edificio::{Edificios, Fuente};
use asentamiento::obras::ErrorObra;
use asentamiento::recetas_unicas::ErrorReceta;
use asentamiento::politicas::{Activador, ErrorPolitica};
use asentamiento::Recurso;
use std::cell::OnceCell;
use territorio::Disco;

// ── Lo que el Exterior de la economía todavía no tiene (cada uno lo dará su sistema) ──
// Sin ocupación, la recaudación va entera (la ocupación la recorta). La ración de la guarnición es la de los residentes (`heroes`).
const FACTOR_EXTRA: i64 = UNO;

/// Los factores extra de la felicidad y de la nobleza de una plaza (Economia_Construccion §9.1, D11 del plan del sistema 6): la felicidad es el **producto** de la ocupación (si la plaza está ocupada
/// tras una conquista) y `1 + bono × residentes`; la nobleza, `1 + bono × residentes`. En ppm.
pub(super) fn factores_extra(r: &tecnologia::Reglas, ocupacion: Option<i64>, residentes: u32) -> (i64, i64) {
    use tecnologia::residentes::{factor_felicidad, factor_nobleza};
    let n = residentes as usize;
    (asentamiento::datos::por(ocupacion.unwrap_or(UNO), factor_felicidad(r, n)), factor_nobleza(r, n))
}

/// Lo que del interior de una plaza cambia con el paso de minuto y es la base de la tendencia «+N/min»: el almacén **exacto** (con su resto oculto: el neto real del
/// flujo, aunque lo visible vaya en unidades enteras) y la población entera (sus restos no viajan).
struct Contadores {
    almacen: [Mil; asentamiento::recurso::N],
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
        ErrorObra::PrestigioInsuficiente => (OBRA_PRESTIGIO_INSUFICIENTE, "la Facción no tiene el prestigio que piden la Maravilla o una estatua más"),
        ErrorObra::SoloEnCapital => (OBRA_SOLO_EN_CAPITAL, "la estatua solo se levanta en la capital de la Facción"),
        ErrorObra::TipoNoManual => (OBRA_TIPO_NO_MANUAL, "el Centro Urbano y los puestos del Mercado no se añaden a mano"),
        ErrorObra::Unico => (OBRA_UNICO, "ya hay uno y solo puede haber uno por plaza"),
        ErrorObra::TopeAlmacenes => (OBRA_TOPE_ALMACENES, "se alcanzó el máximo de Almacenes del nivel"),
        ErrorObra::TopeViviendas => (OBRA_TOPE_VIVIENDAS, "se alcanzó el máximo de Viviendas del nivel"),
        ErrorObra::NivelInsuficiente => (OBRA_NIVEL_INSUFICIENTE, "el nivel de la plaza no alcanza para ese edificio"),
        ErrorObra::FaltaEdificio => (OBRA_FALTA_EDIFICIO, "falta el edificio que pide (el Albergue de caravanas, un Mercado)"),
        ErrorObra::SoloEnAldea => (OBRA_SOLO_EN_ALDEA, "el Corral solo se levanta en una aldea agrícola"),
        ErrorObra::SoloPorPueblo => (OBRA_SOLO_POR_PUEBLO, "los minerales y la Cantera solo se explotan con un pueblo extractor"),
        ErrorObra::TopeTransformacion => (OBRA_TOPE_TRANSFORMACION, "se alcanzó el máximo de talleres de ese tipo del nivel"),
        ErrorObra::TopePorPlaza => (OBRA_TOPE_POR_PLAZA, "la plaza ya tiene todas las bodegas (o prensas) que caben"),
        ErrorObra::GranFundicion => (OBRA_GRAN_FUNDICION, "la Gran Fundición pide nivel de Facción, ser la capital y que no haya otra"),
        ErrorObra::ColaLlena => (OBRA_COLA_LLENA, "la cola de obras está llena"),
        ErrorObra::SinSitio => (OBRA_SIN_SITIO, "no hay sitio libre en la zona para ese edificio"),
        ErrorObra::RecursosInsuficientes => (OBRA_RECURSOS_INSUFICIENTES, "faltan recursos (o se bajaría de la reserva)"),
        ErrorObra::NoExiste => (EDIFICIO_DESCONOCIDO, "la plaza no tiene ese edificio"),
        ErrorObra::NoEstaEnCola => (OBRA_NO_ESTA_EN_COLA, "el edificio no está en la cola de obras"),
        ErrorObra::NoEstaEnObra => (OBRA_NO_ESTA_EN_CURSO, "el edificio no tiene obra en curso"),
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
        Bloqueo::FaltaPueblo => (ASCENSO_FALTA_ALDEA, "falta un pueblo activo: una aldea agrícola o un pueblo extractor".into()),
        Bloqueo::SinCupoDeFaccion => (ASCENSO_SIN_CUPO_DE_FACCION, "la Facción no tiene cupo para ese nivel".into()),
        Bloqueo::RecursosInsuficientes => (ASCENSO_RECURSOS_INSUFICIENTES, "faltan recursos para la obra de ascenso".into()),
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
        EventoEco::ObraCancelada { id, mejora } => ("construccion.obra_cancelada", vec![("edificio", *id as i64), ("mejora", *mejora as i64)]),
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
        EventoEco::AldeaMarcada { id, pos } => ("aldea.marcada", vec![("pueblo", *id as i64), ("x", pos.x), ("y", pos.y)]),
        EventoEco::AldeaMejoraMarcada { id, nivel } => ("aldea.mejora_marcada", vec![("pueblo", *id as i64), ("nivel", i64::from(*nivel))]),
        EventoEco::AldeaSubio { id, nivel } => ("aldea.subio", vec![("pueblo", *id as i64), ("nivel", i64::from(*nivel))]),
        EventoEco::ExtractorMejoraMarcada { id, nivel } => ("pueblo.mejora_marcada", vec![("pueblo", *id as i64), ("nivel", i64::from(*nivel))]),
        EventoEco::ExtractorSubio { id, nivel } => ("pueblo.subio", vec![("pueblo", *id as i64), ("nivel", i64::from(*nivel))]),
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
        let (maravilla_desbloqueada, estatuas_permitidas) = self.prestigio_para_obras(self.estado.asentamientos[&id].faccion);
        let residentes = self.residentes_en(id);
        let cupo_para_recuperar = self.cupo_para_recuperar(id, nivel_faccion);
        // La mejor fertilidad de su zona real: la comparten todas sus Granjas.
        let fertilidad = self.fertilidad_de_zona(id)?;
        let mut a = self.estado.asentamientos.remove(&id)?;
        let (nivel_antes, nobleza_antes, alcanzado_antes, subida_antes) = (a.nivel_actual(), a.economia.poblacion.nobleza > 0, a.nivel(), a.economia.ascenso.map(|s| s.nivel_objetivo));
        let (r, contado) = {
            let Estado { reglas, asentamientos, parcelas_ocupadas, nodos_reclamados, yacimientos, contactos, tecnologia, .. } = &mut self.estado;
            let rt = &reglas.economia.tecnologia;
            let adoptadas = tecnologia.de(rt, a.faccion).adoptadas;
            let tiene_tecnologia = |t: &str| crate::estado::adopta(rt, adoptadas, t);
            // El `bonusProduccion` de las adoptadas (6C: `canteria` ×1,5 la piedra), en plazas y pueblos, aplicado al ritmo antes del resto oculto.
            let factor_tecnologia = |rec: Recurso| tecnologia::adopcion::factor_produccion(rt, adoptadas, rec);
            let extra = factores_extra(rt, a.ocupada_hasta.map(|_| reglas.economia.combate.ocupacion.factor_crecimiento), residentes);
            let mut fuentes = FuentesMundo { mapa: &geo.mapa, nodos: yacimientos, ahora: t, catalogo: &reglas.economia.datos.catalogo, contado: [0; tecnologia::Contador::N], agotados: vec![], regenerados: vec![] };
            let mut sitios = SitiosPlaza {
                mapa: &geo.mapa,
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
                fertilidad,
                tiene_tecnologia: &tiene_tecnologia,
                factor_tecnologia: &factor_tecnologia,
                nivel_faccion,
                maravilla_desbloqueada,
                estatuas_permitidas,
                es_capital,
                faccion_tiene_gran_fundicion,
                ciudadanos,
                distancia_capital,
                racion_guarnicion,
                // Una plaza ocupada tras una conquista crece y recauda a la fracción de la ocupación (Combate §8): su mantenimiento ya lo congela `ocupado`. Sus Aedas residentes suben la
                // felicidad y la nobleza (6E), multiplicando lo de la ocupación (D11).
                felicidad_extra: extra.0,
                nobleza_extra: extra.1,
                recaudacion_extra: if a.ocupada_hasta.is_some() { reglas.economia.combate.ocupacion.factor_recaudacion } else { FACTOR_EXTRA },
                cupo_para_recuperar,
            };
            let r = f(&mut a.economia, &mut x);
            (r, (fuentes.contado, fuentes.agotados, fuentes.regenerados))
        };
        let (contado, agotados, regenerados) = contado;
        self.anotar_yacimientos(&a, t, &agotados, &regenerados);
        let (nivel_ahora, nobleza_ahora, faccion, alcanzado_ahora) = (a.nivel_actual(), a.economia.poblacion.nobleza > 0, a.faccion, a.nivel());
        // Una obra de subida que empieza o acaba ocupa o libera cupo de nivel (`Faccion.cupos_de_nivel`); el nivel actual lo toca `contar_nivel`.
        if subida_antes != a.economia.ascenso.map(|s| s.nivel_objetivo) {
            self.tr.tocar(Clase::Faccion, u64::from(faccion));
        }
        self.estado.asentamientos.insert(id, a);
        self.contar_alcanzado(Some(alcanzado_antes), Some(alcanzado_ahora));
        // El cupo de Aedas residentes solo cambia en el paso si la nobleza pasa de 0 a más (o al revés) o cambia el nivel actual (6E; el Palacio, en `anotar_sucesos` y la conquista).
        if nobleza_antes != nobleza_ahora || nivel_antes != nivel_ahora {
            self.revisar_residentes(id, t);
        }
        // Lo extraído y lo fabricado cuentan para los logros en origen (6B), y un nivel actual nuevo, para el récord de plazas por nivel.
        self.contar_flujos(t, &contado);
        self.contar_nivel(t, faccion, Some(nivel_antes), Some(nivel_ahora));
        // El paso de la capital mira `recursoEnCapital` si alguna candidata lo pide (6C, plan §5.1).
        if es_capital {
            self.mirar_recurso_en_capital(faccion);
        }
        Some(r)
    }

    /// Los hechos de los yacimientos que tocó el paso de la plaza `a`: `construccion.yacimiento_agotado` con el edificio (o el pueblo) que lo explota, y `mapa.yacimiento_regenerado`
    /// (Economia_Construccion §5, Generacion_Del_Mundo §10.1). Los oye la Facción de la plaza.
    fn anotar_yacimientos(&mut self, a: &crate::estado::Asentamiento, t: Instante, agotados: &[u32], regenerados: &[u32]) {
        let Some(geo) = self.geografia.as_ref() else { return };
        let catalogo = &self.estado.reglas.economia.datos.catalogo;
        let recurso = |nodo: u32| geo.mapa.nodos.get(nodo as usize).and_then(|n| catalogo.tipo(n.tipo).produccion).map_or(-1, |(r, _)| recurso_a_u8(r) as i64);
        let mut hechos = Vec::new();
        for &nodo in regenerados {
            hechos.push(("mapa.yacimiento_regenerado", vec![("nodo", nodo as i64), ("recurso", recurso(nodo))]));
        }
        for &nodo in agotados {
            let mut datos = vec![("nodo", nodo as i64), ("recurso", recurso(nodo))];
            if let Some(e) = a.economia.edificios.iter().find(|e| e.fuente == Some(Fuente::Nodo(nodo))) {
                datos.extend([("edificio", e.id as i64), ("tipo", tipo_a_u8(e.tipo) as i64)]);
            } else if let Some(p) = a.economia.pueblos.iter().find(|p| p.nodo() == Some(nodo)) {
                datos.extend([("pueblo", p.id as i64), ("tipo", tipo_a_u8(p.tipo) as i64)]);
            }
            hechos.push(("construccion.yacimiento_agotado", datos));
        }
        for (codigo, datos) in hechos {
            self.tr.emitir(&self.estado, EventoNucleo { codigo: codigo.into(), en: t, ejercitos: vec![], asentamientos: vec![a.id], punto: Some(a.posicion), datos, ..Default::default() });
        }
        if !(agotados.is_empty() && regenerados.is_empty()) {
            self.tr.tocar(Clase::Asentamiento, a.id);
        }
    }

    /// El paso de un minuto de la plaza `id` en `t` (el evento `Carga::Minuto`).
    pub(super) fn minuto_de_plaza(&mut self, id: u64, t: Instante) {
        if !self.estado.asentamientos.contains_key(&id) {
            return;
        }
        self.programar_minuto(id, t + MS_POR_MINUTO);
        // La tendencia «+N/min» es lo que cambió el almacén y la población en el paso entero (el de la guarnición, que come después, se le suma en `alimentar_guarnicion`), y lo que
        // entró y salió de cada recurso por separado (mecánica 60).
        let Some((sucesos, racion)) = self.con_exterior(id, t, |eco, x| {
            let antes = Contadores::de(eco);
            eco.almacen.registrar_flujos();
            let sucesos = eco.avanzar_minuto(t, x);
            let flujos = eco.almacen.tomar_flujos();
            eco.tendencia = Tendencia { producido: flujos.producido, consumido: flujos.consumido, produccion: flujos.produccion, ..Contadores::de(eco).tendencia(&antes) };
            // La advertencia de solvencia del nivel siguiente (2026-11-10), para la proyección: ya no bloquea la subida.
            eco.aviso_solvencia = AvisoSolvencia(eco.insolvencia_de_ascenso(t, x.reglas, x));
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
                EventoEco::PuebloCompletado { id: pueblo } => self.dar_carro_a_pueblo(id, *pueblo, t),
                _ => {}
            }
        }
        // El carro en casa sale en cuanto el almacén local tiene para llenarlo, no solo al llenarse ese almacén: desde el 2026-11-05 (almacenes al doble, carros igual) el
        // almacén local es mayor que el carro y guarda lo que se extrae mientras el carro va y viene.
        let con_carro: Vec<u64> = self.estado.asentamientos[&id].economia.pueblos.iter().filter(|p| p.carro.is_some()).map(|p| p.id).collect();
        for pueblo in con_carro {
            self.salir_acarreo(id, pueblo, t);
        }
        // Los pueblos sin carro lo reponen en cuanto la madre puede pagarlo (2026-10-27).
        self.reponer_carros(id, t);
        // La guarnición come después de la población (Reloj_Y_Eventos §2), la ración de este mismo paso.
        self.alimentar_guarnicion(id, t, racion);
        // El tributo de la vasalla, al final de su paso, tras comer su guarnición, para que la plaza atienda antes lo suyo (Q3; regla por decisión del autor, 2026-10-25, 5D-3).
        // Las columnas personales de su Facción que están en su zona repostan de lo que le sobra, antes del tributo (2026-10-27).
        self.repostar_en_la_zona(id, t);
        self.pagar_tributo(id, t);
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
                | EventoEco::PuebloDestruido { id: pueblo }
                | EventoEco::AldeaMarcada { id: pueblo, .. }
                | EventoEco::AldeaMejoraMarcada { id: pueblo, .. }
                | EventoEco::AldeaSubio { id: pueblo, .. }
                | EventoEco::ExtractorMejoraMarcada { id: pueblo, .. }
                | EventoEco::ExtractorSubio { id: pueblo, .. } => self.tr.tocar(Clase::Pueblo, *pueblo),
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
        self.misiones_de_sucesos(id, t, sucesos);
        // Un edificio que se completa o se mejora y un ascenso son disparadores de la aparición de su Facción (6C, `edificio` y `capitalEnNivel`).
        // Un pueblo cuenta como la obra de su extractor (Cantera o mina, desde el 2026-11-01); la aldea ganadera, como la de sus corrales, y las demás aldeas,
        // como la de su Granja (2026-11-11: antes toda aldea contaba como obra de corral).
        let datos = &self.estado.reglas.economia.datos;
        let obra_de_pueblo: Vec<(u64, Tipo)> = self.estado.asentamientos.get(&id).map(|a| a.economia.pueblos.iter().map(|p| (p.id, if p.tiene_corrales(datos) { Tipo::Corral } else { p.tipo })).collect()).unwrap_or_default();
        let tipo_de_pueblo = |p: u64| obra_de_pueblo.iter().find(|&&(x, _)| x == p).map(|&(_, t)| t);
        let pueblo_completado = sucesos.iter().any(|s| matches!(*s, EventoEco::PuebloCompletado { .. }));
        if pueblo_completado || sucesos.iter().any(|s| matches!(s, EventoEco::EdificioCompletado { .. } | EventoEco::MejoraCompletada { .. } | EventoEco::NivelSubio { .. })) {
            self.mirar_aparicion(faccion);
        }
        // Los hechos de épica de la plaza (6E, TA §7.2): la obra (con el tipo del edificio) y el ascenso. Un Palacio completado puede abrir el cupo de residentes.
        let mut palacio = false;
        for s in sucesos {
            let hecho = match *s {
                EventoEco::EdificioCompletado { id: e, tipo, .. } => {
                    palacio |= tipo == Tipo::Palacio;
                    Some(tecnologia::epicas::Hecho::obra(id as u32, faccion, e, tipo, None))
                }
                EventoEco::MejoraCompletada { id: e, nivel } => {
                    let tipo = self.estado.asentamientos.get(&id).and_then(|a| a.economia.edificios.iter().find(|x| x.id == e)).map(|x| x.tipo);
                    tipo.map(|tipo| tecnologia::epicas::Hecho::obra(id as u32, faccion, e, tipo, Some(nivel)))
                }
                EventoEco::NivelSubio { nivel } => Some(tecnologia::epicas::Hecho::ascenso(id as u32, faccion, nivel.min(u8::MAX as u32) as u8)),
                // El pueblo es la obra de su extractor y la aldea agrícola, la «obra de corral» (con su id de pueblo, apartado de los de edificio).
                EventoEco::PuebloCompletado { id: p } => tipo_de_pueblo(p).map(|tipo| tecnologia::epicas::Hecho::obra(id as u32, faccion, 0x8000_0000 | p as u32, tipo, None)),
                _ => None,
            };
            self.hecho_epico(hecho);
        }
        if palacio {
            self.revisar_residentes(id, t);
        }
        // La experiencia de Facción por construir y por ascender (§3.5), en el instante en que acaba la obra.
        for s in sucesos {
            // El prestigio del ascenso: `ascensoPorNivel` por el nivel nuevo.
            if let &EventoEco::NivelSubio { nivel } = s {
                let por_nivel = self.estado.reglas.economia.politica.prestigio.ascenso_por_nivel;
                self.dar_prestigio(t, faccion, por_nivel.saturating_mul(i64::from(nivel)), super::prestigio::motivo::ASCENSO);
            }
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
        let nivel_actual = a.nivel_actual();
        self.contar_alcanzado(Some(a.nivel()), None);
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
            if let Some(n) = p.nodo() {
                self.estado.nodos_reclamados.remove(&n);
            }
            self.tr.tocar(Clase::Pueblo, p.id);
            self.olvidar_ruta_de_acarreo(p.id);
            self.liberar_guarnicion_de_pueblo(p.id, |_| true);
        }
        // Lo que dejaron de visita en la plaza vuelve, intacto, al campamento de cada héroe (2026-11-04).
        self.liberar_guarnicion_de_visita(id, |_| true);
        // Quien estaba dentro de sus pueblos o avanzadas sale al mapa con lo suyo, a la puerta de cada uno.
        let nodos = self.geografia.as_ref().map_or(&[][..], |g| &g.mapa.nodos[..]);
        let pueblos: Vec<(u64, Punto)> = a.economia.pueblos.iter().filter_map(|p| crate::economia::posicion_de_pueblo(nodos, p).map(|pos| (p.id, pos))).collect();
        let avanzadas: Vec<(u64, Punto)> = a.economia.avanzadas.iter().map(|v| (v.id, v.posicion)).collect();
        self.vaciar_pueblos_y_avanzadas(t, id, &pueblos, &avanzadas);
        self.avanzadas_de_la_madre_arruinada(a.economia.avanzadas, t);
        self.estado.contactos.retain(|&(x, y)| x != id && y != id);
        self.quitar_plaza(id, a.faccion);
        self.residentes_tras_la_ruina(id);
        // La participación en la plaza se va con ella (Participacion §3).
        self.estado.participacion.remove(&id);
        // Y su tablón (Misiones_Recurrentes §2).
        self.estado.recurrentes.tablones.remove(&crate::misiones::Dueno::Asentamiento(id));
        self.contar_nivel(t, a.faccion, Some(nivel_actual), None);
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
            self.diplomacia_tras_derrota(t, a.faccion);
            self.emitir_propio(t, "faccion.derrotada", vec![], vec![("faccion", a.faccion as i64), ("por", 0)]);
            self.reprogramar_anillos(t);
        }
    }

    // ───────────────────────────── Órdenes ─────────────────────────────

    /// Plazas de la Facción (salvo `id`) que ocupan un puesto en el nivel `objetivo`: lo tienen como nivel actual o van hacia él.
    fn cupo_ocupado(&self, id: u64, faccion: u32, objetivo: u32) -> u32 {
        self.plazas_de(faccion).filter(|o| o.id != id && (o.nivel_actual() == objetivo || o.economia.ascenso.is_some_and(|s| s.nivel_objetivo == objetivo))).count() as u32
    }

    /// El cap de fundación y los cupos por nivel de la Facción, con lo que gasta y ocupa cada uno (2026-11-04, `Faccion.cap_fundacion` y `Faccion.cupos_de_nivel`).
    pub fn cupos_de_faccion(&self, faccion: u32) -> CuposDeFaccion {
        let r = &self.estado.reglas.economia.politica.nivel;
        let nivel = self.nivel_de_faccion(faccion);
        CuposDeFaccion {
            cap: politica::nivel::cap_fundacion(r, nivel),
            plazas: self.plazas_de(faccion).count() as u32,
            caravanas: self.fundaciones_de(faccion) as u32,
            niveles: (2..=5).filter_map(|o| politica::nivel::cupo_nivel(r, nivel, o).map(|c| (o, c, self.cupo_ocupado(u64::MAX, faccion, o)))).collect(),
        }
    }

    /// Si la plaza, degradada, puede recuperar el nivel siguiente: su Facción tiene cupo libre en él (2026-11-04, decisión del autor: recuperarse no salta el cupo). Sin degradar,
    /// sí (no se mira).
    fn cupo_para_recuperar(&self, id: u64, nivel_faccion: u32) -> bool {
        let a = &self.estado.asentamientos[&id];
        let siguiente = a.nivel_actual() + 1;
        a.nivel_actual() >= a.nivel()
            || politica::nivel::cupo_nivel(&self.estado.reglas.economia.politica.nivel, nivel_faccion, siguiente).is_none_or(|c| self.cupo_ocupado(id, a.faccion, siguiente) < c)
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
        | Comando::CancelarConstruccion { asentamiento, .. }
        | Comando::MoverEnCola { asentamiento, .. }
        | Comando::MejorarEdificio { asentamiento, .. }
        | Comando::AsignarBuey { asentamiento, .. }
        | Comando::ElegirReceta { asentamiento, .. }
        | Comando::AlternarAutoConstruccion { asentamiento, .. }
        | Comando::AlternarReceta { asentamiento, .. }
        | Comando::PausarConsumo { asentamiento, .. }
        | Comando::CalibrarReserva { asentamiento, .. }
        | Comando::SolicitarAscenso { asentamiento }
        | Comando::ActivarPolitica { asentamiento, .. }) = &p.comando
        else {
            unreachable!("solo se llama con órdenes de economía")
        };
        let id = *asentamiento;
        let Some(plaza) = self.estado.asentamientos.get(&id) else { return Err(rechazo(rechazos::ASENTAMIENTO_DESCONOCIDO, ids::asentamiento(id))) };
        let faccion = plaza.faccion;
        self.autorizar_en_plaza(p, id, None)?;
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
            Comando::CancelarConstruccion { edificio, .. } => {
                let plaza = self.estado.asentamientos.get_mut(&id).expect("existe");
                let fuente = plaza.economia.edificio(*edificio).and_then(|e| e.fuente);
                if plaza.economia.cancelar_obra(t, &datos, *edificio, &mut sucesos).map_err(rechazo_obra)? {
                    plaza.urbe.quitar(*edificio); // su pieza deja de ocupar suelo
                    self.liberar_fuente(id, fuente);
                }
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
            Comando::AsignarBuey { edificio, .. } => {
                // El buey cuesta lo que el de una caravana (`ANIMAL_CATALOGO.buey.costo`) y se queda en el molino.
                let precio = &self.estado.reglas.economia.comercio.flota.buey.costo;
                let eco = &mut self.estado.asentamientos.get_mut(&id).expect("existe").economia;
                let molino = eco.edificios.iter_mut().find(|e| e.id == *edificio).ok_or_else(|| rechazo_obra(ErrorObra::NoExiste))?;
                if molino.tipo != Tipo::Molino {
                    return Err(rechazo(rechazos::MOLINO_NO_ES_MOLINO, format!("ed-{edificio} no es un molino")));
                }
                if molino.buey {
                    return Err(rechazo(rechazos::MOLINO_YA_TIENE_BUEY, format!("el molino ed-{edificio} ya tiene su buey")));
                }
                if !eco.almacen.pagar(precio) {
                    return Err(rechazo_obra(ErrorObra::RecursosInsuficientes));
                }
                molino.buey = true;
                self.emitir_propio(t, "construccion.buey_asignado", vec![id], vec![("edificio", *edificio as i64)]);
            }
            Comando::ElegirReceta { edificio, insumo, .. } => {
                let eco = &mut self.estado.asentamientos.get_mut(&id).expect("existe").economia;
                eco.elegir_receta(&datos, *edificio, *insumo).map_err(|e| match e {
                    ErrorReceta::NoExiste => rechazo_obra(ErrorObra::NoExiste),
                    ErrorReceta::NoSeElige => rechazo(rechazos::RECETA_NO_SE_ELIGE, format!("ed-{edificio} no elige receta: solo la bodega y la prensa")),
                    ErrorReceta::Desconocida => rechazo(rechazos::RECETA_DESCONOCIDA, format!("ed-{edificio} no tiene ninguna receta con ese insumo")),
                    ErrorReceta::TopePorPlaza => rechazo(rechazos::RECETA_TOPE_POR_PLAZA, "la plaza ya trabaja tantas recetas distintas de ese edificio como permite su nivel".into()),
                    ErrorReceta::SinCambios => sin_cambios(),
                })?;
                self.emitir_propio(t, "construccion.receta_elegida", vec![id], vec![("edificio", *edificio as i64), ("insumo", recurso_a_u8(*insumo) as i64)]);
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
            Comando::PausarConsumo { recurso, pausado, .. } => {
                match self.estado.asentamientos.get_mut(&id).expect("existe").economia.pausar_consumo(&datos, *recurso, *pausado) {
                    Err(()) => return Err(rechazo(rechazos::CONSUMO_NO_SE_PAUSA, format!("{} no es un bien de consumo que se pueda pausar (el trigo nunca se pausa)", recurso.nombre()))),
                    Ok(false) => return Err(sin_cambios()),
                    Ok(true) => {}
                }
                self.emitir_propio(t, "consumo.pausado", vec![id], vec![("recurso", recurso_a_u8(*recurso) as i64), ("pausado", *pausado as i64)]);
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
                        (eco.solicitar_ascenso(t, x.reglas, sin_cupo, &mut ev), ev)
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
                let plaza = self.estado.asentamientos.get_mut(&id).expect("existe");
                // El cargo que invoca la orden está ocupado: lo ocupa quien la da (la matriz ya lo comprobó). La administración, que no tiene cargo, activa como si estuviera cubierto.
                let cargo_ocupado = p.admin || plaza.cargos.ocupado(*cargo);
                let eco = &mut plaza.economia;
                let activador = Activador { cargo: *cargo, cargo_ocupado, nivel_faccion, sala_consejo: eco.edificios.activos(Tipo::SalaConsejo) > 0 };
                let expira = eco.politicas.activar(&datos.politicas, politica, &activador, t).map_err(|e| {
                    let (c, m) = rechazo_de_politica(e);
                    rechazo(c, m.into())
                })?;
                let punto = self.estado.asentamientos[&id].posicion;
                let indice = datos.politicas.catalogo.iter().position(|q| &q.id == politica).map_or(-1, |i| i as i64);
                self.tr.emitir(&self.estado, EventoNucleo { codigo: "politica.activada".into(), en: t, ejercitos: vec![], asentamientos: vec![id], punto: Some(punto), datos: vec![("politica", indice), ("expiraEn", expira)], ..Default::default() });
                let faccion = self.estado.asentamientos[&id].faccion;
                self.avance(t, Dueno::Faccion(faccion), "politicas", 1);
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

/// El cap de fundación de una Facción y sus cupos por nivel (ver [`Nucleo::cupos_de_faccion`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CuposDeFaccion {
    pub cap: u32,
    /// Lo que gasta el cap: sus plazas de hoy y sus Caravanas de Fundación vivas.
    pub plazas: u32,
    pub caravanas: u32,
    /// `(nivel, cupo, ocupadas)` de los niveles 2 a 5.
    pub niveles: Vec<(u32, u32, u32)>,
}
