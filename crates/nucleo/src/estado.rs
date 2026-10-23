//! El estado determinista del mundo: arena de ejércitos, cola de eventos, generaciones, azar, versión y reloj. Es TODO lo
//! que entra en `hash_estado` y en las fotos; lo que no está aquí (sesiones, registro de cambios, métricas) se reconstruye.

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::codec_batallas;
use crate::codec_ciudad;
use crate::codec_columna;
use crate::codec_combate;
use crate::codec_comercio;
use crate::codec_economia;
use crate::codec_militar;
use crate::economia::StockNodo;
use crate::memoria::{self, Memoria};
use crate::estado_batallas::EstadoBatallas;
use crate::estado_combate::EstadoCombate;
use crate::estado_comercio::EstadoComercio;
use crate::estado_militar::{Acceso, EstadoMilitar};
use crate::proyeccion::ambito::Ambito;
use crate::urbanismo::{Urbanismo, Urbe};
use base::{Instante, Mil, Plan, Punto, Rng};
use cola::{Clave, Cola, Evento, Generaciones};
use militar::columna::Columna;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::{Arc, OnceLock};

/// Prioridad de la salida del mundo de quien se desconectó: la de «cierre de batallas vencidas y salidas del mundo» (Reloj_Y_Eventos §2). Sale
/// antes que todo lo demás del mismo instante.
pub const PRIO_SALIDA: u8 = 1;
/// Prioridad de los plazos de una batalla de Unity: el fin de la convocatoria, el vencimiento de su estado y la retirada de la cerrada. Es la misma
/// fila que la salida del mundo (Reloj_Y_Eventos §2): una batalla que vence en un instante ya no bloquea en ese mismo instante.
pub const PRIO_BATALLA: u8 = 1;
/// Prioridad de la regeneración, la aparición y el ataque de los bandidos (Reloj_Y_Eventos §2): ven a las columnas tal como estaban antes de moverse
/// en ese instante (las llegadas y los encuentros son de la 8).
pub const PRIO_BANDIDOS: u8 = 7;
/// Prioridad de los umbrales de víveres y de la deserción por hambre: «consumo de población y de tropa» (Reloj_Y_Eventos §2).
pub const PRIO_UMBRAL: u8 = 3;
/// Prioridad del comercio: «comercio, trueques, órdenes de mercado, caravanas» (Reloj_Y_Eventos §2). Va después de los mantenimientos y las ruinas
/// (5): una plaza arruinada ya no existe para el comercio de ese instante. Dentro de la prioridad el orden es el de [`SucesoComercio::rango`].
pub const PRIO_COMERCIO: u8 = 6;
/// Prioridad de las llegadas: «llegadas y encuentros de columnas». También la de la caducidad de una formación y de una petición de unión (son
/// asuntos de la columna que no tienen fila propia en la tabla de Reloj_Y_Eventos §2).
pub const PRIO_LLEGADA: u8 = 8;
/// Prioridad de la reputación: «diplomacia, tributos, experiencia, reputación».
pub const PRIO_REPUTACION: u8 = 9;
/// Prioridad del fin de una obra, una mejora o un ascenso de una plaza (`Carga::FinDeObras`, Reloj_Y_Eventos §2): la fila 1, compartida con las
/// batallas y las salidas del mundo (no interactúan con ellas). Sale **antes** del paso de minuto de la plaza del mismo instante, para que el
/// edificio terminado (o el nivel nuevo) cuente en ese minuto, como siempre.
pub const PRIO_FIN_DE_OBRAS: u8 = 1;
/// Prioridad del paso de un minuto de una plaza (Reloj_Y_Eventos §2). El paso es atómico y, dentro de él, el crate `asentamiento` ya respeta el
/// orden de la regla: pueblos y arranque de la cola, producción y recetas, auto-construcción, comida, crecimiento y recaudación, y por último
/// mantenimiento y ruinas. La plaza come antes que las tropas (3) y que el comercio (6).
pub const PRIO_ASENTAMIENTO: u8 = 2;
/// Prioridad de la caducidad de una política (`Carga::CaducaPolitica`): **después** del paso de minuto de la plaza (2), para que una política de
/// `d` minutos se aplique en los `d` pasos de su duración, incluido el que cae en su instante de caducidad. Comparte fila con el consumo (3).
///
/// DUDA: la definición del 2026-10-22 decía que «el fin va antes del paso» para todas las citas (pensando en el edificio terminado); para la política
/// se conserva el orden de siempre (`caducar` iba después de producir), que es el que evita acortar en un minuto toda política activada en un paso.
/// Anotada en `Docs/Mecanicas a desarrollar.md`, sección de paridad.
pub const PRIO_CADUCA_POLITICA: u8 = 3;

/// BAN23: BAN22 con la memoria de la niebla (4H T3d, `memoria`): lo explorado por cada Facción y por cada héroe sin bandera (máscaras de bits sobre la rejilla de celdas de 5 unidades), la última ficha de cada
/// sitio ajeno que vio cada Facción (plazas, pueblos y avanzadas, con el instante en que se vio), desde cuándo tiene grabado cada columna lo que ve (su vigía) y desde cuándo vale lo que se ve de cada sitio.
/// BAN22 se rechaza: no trae lo explorado, que no se puede deducir del estado.
///
/// BAN22: BAN21 con la tendencia «+N/min» de cada plaza (4H T3c, `asentamiento::Asentamiento::tendencia`, en `codec_economia`): lo que cambió cada recurso del almacén y cada clase de la población en el
/// último paso de minuto (`asentamiento::asentamiento::Tendencia`), que el contador del cliente rotula junto al valor en escalones. BAN21 se rechaza: sus plazas no traen la tendencia (y no se puede deducir del estado).
///
/// BAN21: BAN20 con lo que 4H (T4) congela en el ticket de una batalla de Unity (`codec_batallas`): la plaza de un asedio (`Ticket::plaza`: edificios internos y recintos con las celdas en pie, `layoutVersion` y
/// `unidadesPorCelda`) y el nivel, la experiencia y la moral de cada escuadra al abrir (`EscuadraEnBatalla`); y, en el informe de inspección de una plaza ajena, sus recintos con el anillo entero
/// y cuántas celdas están levantadas (`codec_militar`). BAN20 se rechaza: su ticket no congelaba nada de esto.
///
/// BAN20: BAN19 con los recintos de muralla (fase de Murallas, segunda tanda, 2026-10-22): cada plaza los guarda en su economía (`codec_economia`: id, nivel y celdas del anillo en pie con su
/// clase, avance, mejora aparte, cita de la siguiente celda, cuándo se comprometió y cuándo se cerró por primera vez) y la ciudad de la plaza (`codec_ciudad`) **deja de guardarlos**: los
/// recibe derivados al leerla. El bloque `MURALLA` entra en la huella de las reglas. BAN19 se rechaza: su ciudad guardaba solo muros, puertas y un «completo» por recinto, sin nivel, avance ni mejora.
///
/// BAN19: BAN18 con el contexto de una batalla de caravana en lista: lleva **todas** las caravanas en juego, cada una con sus carros (`Contexto::Caravana { caravanas, .. }`,
/// `codec_batallas`; quinta ronda de Entrar con todo, 2026-10-22), porque el acoso de los bandidos a una columna pone en juego todas sus caravanas adjuntas. El resto del formato es el
/// mismo. BAN18 se rechaza: su contexto nombraba una sola caravana.
///
/// BAN18: BAN17 con dos ubicaciones nuevas del héroe, dentro de un pueblo extractor y dentro de una avanzada (`Ubicacion::Pueblo` y `Ubicacion::Avanzada`, etiquetas 4 y 5 en
/// `codec_militar`; Entrar con todo, 2026-10-22). El resto del formato es el mismo. BAN17 se rechaza: sus héroes solo podían estar dentro de una plaza o un campamento.
///
/// BAN17: BAN16 más el acceso de cada plaza (`Asentamiento::acceso`: aliados, neutrales y enemigos; Entrar con todo, 2026-10-22). El resto del formato es el mismo: el carro que
/// un héroe lleva sin columna (`HeroeNucleo::carro`, antes `carro_fuera`) ya estaba en la foto. BAN16 se rechaza: sus plazas no traen acceso.
///
/// BAN16: BAN15 con el fin de la obra de una avanzada en una cita propia (`SucesoCombate::FinObraAvanzada`, de la entidad `entidad_obra_de_avanzada`) en lugar de la
/// cita de fin de obras de su plaza (`Carga::FinDeObras`): así cancelarla, o perderla con la avanzada, la descarta sin tocar nada más. Las cifras del bloque `AVANZADAS`
/// cambian (la huella de reglas). BAN15 se rechaza: sus obras de avanzada esperaban en la cita de la plaza y su huella de reglas no es la de hoy.
///
/// BAN15: BAN14 más las avanzadas (fase 4F, 2026-10-22): cada plaza guarda las suyas en su economía (`codec_economia`: etapa, estado, obra de mapa y almacén), el
/// estado del combate guarda su guarnición de serie con las tropas que eligió el Gobernador y las ruinas de ciudadela sin dueño (`codec_combate`), las escuadras
/// pueden estar en la guarnición de una avanzada (`Contenedor::Avanzada`, `codec_militar`) y la cola gana dos plazos de combate (`SucesoCombate::FinInvulnerabilidad` y
/// `ReponeAvanzada`). BAN14 se rechaza: no traía nada de esto y su huella de reglas no incluye las cifras de las avanzadas.
///
/// BAN14: BAN13 más la economía por minuto con fines exactos (2026-10-22): la población es entera con un resto oculto por clase y una marca de
/// recuperación del hambre (`codec_economia`), y dos cargas nuevas de la cola, `Carga::FinDeObras` y `Carga::CaducaPolitica`. BAN13 se rechaza: su
/// población era continua (milésimas de habitante, sin resto ni marca) y no traía las citas de fin de obra, de mejora, de ascenso y de política.
///
/// Del BAN13 (fase 4E) vienen: las batallas de Unity (`codec_batallas`: ticket congelado, incorporaciones, salidas, bloqueo, asignación con sus tokens y
/// resultado), la regla `batallas_en_unity` en la cabecera, el plan en pausa de las columnas apartadas (`codec_columna`), el estado `Apartada` de la ración y
/// una carga nueva de la cola (`Carga::Batalla`). BAN12 se rechaza: no traía nada de esto y sus columnas apartadas no guardaban su plan.
///
/// Del BAN12 (fase 4D) vienen el estado del combate (`codec_combate`: campamentos de bandidos y sus plazos, guarnición de serie de los pueblos y lo que cada
/// héroe ha destruido), el fin de la protección y de la ocupación de cada plaza y `Carga::Combate`.
const MAGIA: &[u8; 5] = b"BAN23";

/// Reglas de juego mínimas del núcleo genérico. Entran en el hash y en las fotos: dos mundos con reglas distintas no se
/// pueden confundir (recuperar con otras reglas es un error).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reglas {
    /// Aviso de víveres bajos al caer por debajo de este porcentaje de la capacidad (0 = sin aviso).
    pub viveres_bajos_pct: u32,
    /// Peticiones recordadas para la idempotencia (`INFRA.limiteIdempotencia`). DUDA (4H T6): son las últimas de toda la partida, no de cada cuenta: con 500 jugadores a una orden cada 10 s
    /// alcanzan ≈ 10 s, y un reintento tras reconectar más tarde se aplicaría otra vez.
    pub limite_idempotencia: usize,
    /// El proceso tiene servidores de batalla (`SERVIDORES_BATALLA`, Batalla_Unity_Ciclo §8): `atacar` abre siempre una batalla de Unity, también bot contra bot
    /// (cuarta ronda de Entrar con todo, 2026-10-22). Sin ellos el combate se resuelve con números (Combate.md). Es una regla de la partida y no puede
    /// cambiar entre una foto y su recuperación.
    pub batallas_en_unity: bool,
    pub territorio: ReglasTerritorio,
    /// Las reglas de población, economía y construcción de las plazas (Economia_Construccion).
    pub economia: ReglasEconomia,
}

impl Default for Reglas {
    fn default() -> Self {
        Self { viveres_bajos_pct: 25, limite_idempotencia: 500, batallas_en_unity: false, territorio: ReglasTerritorio::default(), economia: ReglasEconomia::default() }
    }
}

/// Las reglas del crate `asentamiento` (datos del documento de Mechanics, ya convertidos a enteros), las huellas del trazado urbano
/// (bloques del documento `Trazado_Urbano.md`), las del comercio (crate `comercio`), las militares (crate `militar`) y las del combate (crate `combate`). No se serializan: en las fotos va su **huella**, el
/// sha256 de los bloques de parámetros de los documentos incrustados (no de su prosa); recuperar con otras reglas falla igual que con
/// `ReglasTerritorio`.
#[derive(Clone, Debug)]
pub struct ReglasEconomia {
    pub datos: Arc<asentamiento::Reglas>,
    /// Tipo → huella y categoría de la ciudad de cada plaza.
    pub urbanismo: Arc<Urbanismo>,
    /// Las cifras del comercio (precios, comisiones, trueques, mercado, flota, reputación).
    pub comercio: Arc<comercio::Reglas>,
    /// Las cifras militares (tropas, Liderazgo, guarnición, reclutamiento, ración, presencia, campamentos de mercenarios).
    pub militar: Arc<militar::Reglas>,
    /// Las cifras del combate (poder, bajas, botín, ocupación, bandidos, pueblos, capacidad de las batallas).
    pub combate: Arc<combate::Reglas>,
    huella: String,
}

impl ReglasEconomia {
    /// Carga las cifras de los documentos incrustados (parsea los bloques JSON: hágase una vez).
    pub fn cargar() -> Self {
        let bloques = asentamiento::datos::texto_de_parametros()
            + &asentamiento::datos::texto_de_parametros_de(crate::urbanismo::DOC_TRAZADO)
            + &comercio::datos::texto_de_parametros()
            + &militar::datos::texto_de_parametros()
            + &combate::datos::texto_de_parametros();
        let huella = crate::hex(&Sha256::digest(bloques).into());
        let datos = asentamiento::Reglas::cargar();
        let urbanismo = Arc::new(Urbanismo::cargar(&datos));
        Self { datos: Arc::new(datos), urbanismo, comercio: Arc::new(comercio::Reglas::cargar()), militar: Arc::new(militar::Reglas::cargar()), combate: Arc::new(combate::Reglas::cargar()), huella }
    }
    /// Identidad de las reglas (sha256 en hexadecimal de los bloques de parámetros de los documentos de Mechanics de los que salen).
    pub fn huella(&self) -> &str {
        &self.huella
    }
}

impl Default for ReglasEconomia {
    fn default() -> Self {
        static REGLAS: OnceLock<ReglasEconomia> = OnceLock::new();
        REGLAS.get_or_init(ReglasEconomia::cargar).clone()
    }
}

impl PartialEq for ReglasEconomia {
    fn eq(&self, o: &Self) -> bool {
        self.huella == o.huella
    }
}
impl Eq for ReglasEconomia {}

/// Mundo y territorio (Doc 1.0a, 1.2, 1.7, 1.9c). Distancias y radios en milésimas de unidad estratégica.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReglasTerritorio {
    /// Radio con el que nace la zona de una plaza (`ZONA_INFLUENCIA.radioInicial`: 6).
    pub radio_inicial: Mil,
    /// Techo del radio por nivel del asentamiento (10 / 15 / 20 / 25 / 30).
    pub techos: Vec<Mil>,
    /// Cuánto agranda la zona cada edificio completado (`crecimientoPorEdificioCompletado`: 1). La zona no crece con el tiempo.
    pub crecimiento_por_edificio: Mil,
    /// Distancia mínima entre plazas, de centro a centro (30).
    pub separacion_plazas: Mil,
    /// Distancia mínima a un campamento de mercenarios para fundar (`radioExclusionFundar`: 20).
    pub exclusion_campamento: Mil,
    /// Plazas que puede fundar una Facción por nivel (`CAP_FUNDACION_POR_NIVEL`).
    pub cap_por_nivel: Vec<u32>,
    /// Parte de las parcelas de una masa de bosque que puede ocupar un asentamiento (`cuotaParcelasPorAsentamiento`: 50).
    pub cuota_parcelas_pct: u32,
}

impl Default for ReglasTerritorio {
    fn default() -> Self {
        Self {
            radio_inicial: 6_000,
            techos: vec![10_000, 15_000, 20_000, 25_000, 30_000],
            crecimiento_por_edificio: 1_000,
            separacion_plazas: 30_000,
            exclusion_campamento: 20_000,
            cap_por_nivel: vec![1, 2, 3, 3, 4, 5, 5, 6, 6, 7],
            cuota_parcelas_pct: 50,
        }
    }
}

impl ReglasTerritorio {
    /// Techo del radio de un asentamiento de ese nivel (los niveles fuera de la tabla usan el último).
    pub fn techo(&self, nivel: u32) -> Mil {
        let i = (nivel.max(1) as usize).min(self.techos.len().max(1)) - 1;
        self.techos.get(i).copied().unwrap_or(self.radio_inicial)
    }
}

/// Bandera que ve la visión y la rejilla de encuentros en una columna sin Facción (`Columna::faccion = None`): una que no coincide con ninguna
/// Facción real, así que esas columnas no se ven como compañeras de nadie.
pub const SIN_BANDERA: u32 = u32::MAX;

/// Una columna en el mapa, personal o ejército (Movimiento_Ejercitos §2). Lo que la compone y la manda —tipo, Líder, participantes, política de
/// unión, escuadras, carro y víveres— es de [`Columna`] (crate `militar`); aquí están su posición y su plan, y lo que solo existe en el mundo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ejercito {
    pub id: u64,
    pub columna: Columna,
    /// Dónde está cuando no tiene plan.
    pub posicion: Punto,
    /// Sin vértices = quieta en `posicion`.
    pub plan: Plan,
    /// Solo la administración (`CrearEjercito`): milésimas de unidad por segundo, antes del coste del terreno (30 unidades/min = 500), con las que
    /// marcha esta columna sea cual sea su composición. Sin ella (`None`, lo normal) la velocidad es el mínimo de sus escuadras con soldados y de
    /// sus caravanas adjuntas (`Nucleo::velocidad_de_columna`).
    pub velocidad_fija: Option<Mil>,
    /// Lleva enganchada una Caravana de Fundación (Doc 1.8): puede fundar donde está. La compra y el lanzamiento son de
    /// la economía; hasta entonces la da la administración.
    pub caravana_fundacion: bool,
    /// Caravanas enganchadas a la columna (ids ascendentes, §9): viajan con ella y el reparto automático no las ve.
    pub caravanas_adjuntas: Vec<u64>,
    /// A quién persigue (Movimiento_Ejercitos §7): la **entidad** de la presa, que es el id de una columna o, con etiqueta, el de una caravana
    /// (`entidad_caravana`). Se borra al alcanzarla, perderla de vista o a cubierto, y con `marcharA`, `estacionar` o `dejarDePerseguir`.
    pub persiguiendo: Option<u64>,
    /// Apartada por una batalla de Unity (Reloj_Y_Eventos §6): no se mueve, no se la persigue ni se la ataca, no tiene encuentros y no se le dan
    /// órdenes de movimiento. La pone y la quita el sistema militar (`Nucleo::columna_entra_en_batalla`).
    pub en_batalla: bool,
    /// El plan que llevaba cuando una batalla de Unity la apartó, si marchaba: al acabar la batalla lo retoma desde donde está (`columna_sale_de_batalla`).
    pub plan_en_pausa: Option<PlanEnPausa>,
    /// Versión de mundo del último cambio.
    pub version: u64,
}

/// El plan de una columna apartada por una batalla, tal como lo llevaba al entrar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanEnPausa {
    /// Cuándo la apartó la batalla.
    pub desde: Instante,
    /// La velocidad (por segundo) que llevaba: si cambia con las bajas, el resto del plan se reescala.
    pub velocidad: Mil,
    pub plan: Plan,
}

impl Ejercito {
    pub fn marchando(&self) -> bool {
        !self.plan.vertices.is_empty()
    }
    /// Posición en `t`, evaluada del plan vigente (o la fija si está quieta).
    pub fn posicion_en(&self, t: Instante) -> Punto {
        self.plan.posicion_en(t).unwrap_or(self.posicion)
    }
    /// La bandera de la columna para la visión y los encuentros (`SIN_BANDERA` si no tiene Facción).
    pub fn bandera(&self) -> u32 {
        self.columna.faccion.unwrap_or(SIN_BANDERA)
    }
}

/// Una plaza. La zona de influencia es un disco cuyo radio crece con cada edificio completado, hasta el techo de su nivel; el polígono
/// recortado por las fronteras se deriva (`Nucleo::poligono_zona`) y no se guarda. Lleva su economía (población, almacén, edificios,
/// políticas, ascenso: el crate `asentamiento`), que avanza sola un minuto de mundo por evento.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asentamiento {
    pub id: u64,
    pub faccion: u32,
    pub posicion: Punto,
    /// Instante de fundación (el mismo que `economia.fundado_en`): la fase de los pasos de minuto.
    pub fundado_en: Instante,
    /// Radio de la zona, en milésimas.
    pub radio: Mil,
    /// Parcelas de bosque que explota (índices del artefacto).
    pub parcelas: BTreeSet<u32>,
    pub version: u64,
    pub economia: asentamiento::asentamiento::Asentamiento,
    /// Su ciudad: dónde está cada edificio de la economía (los mismos ids). Se guarda en las fotos y entra en el hash.
    pub urbe: Urbe,
    /// Inatacable hasta este instante (`OCUPACION.proteccionMinutos` tras una conquista): un asedio no se admite mientras `ahora < protegida_hasta`.
    pub protegida_hasta: Option<Instante>,
    /// Ocupada hasta este instante (`OCUPACION.duracionMinutos`): su recaudación y su crecimiento valen el `factorRecaudacion` y el `factorCrecimiento` de la
    /// ocupación y su mantenimiento está congelado (`economia.ocupado`). Un evento de la cola la levanta.
    pub ocupada_hasta: Option<Instante>,
    /// Quién puede entrar en la plaza, por relación con su Facción (Entrar con todo, 2026-10-22). Lo fija quien manda en ella (`FijarAcceso`) y lo heredan sus pueblos
    /// extractores y sus avanzadas.
    pub acceso: Acceso,
}

impl Asentamiento {
    /// Nivel alcanzado (vive en la economía; fija el techo de la zona y de la población).
    pub fn nivel(&self) -> u32 {
        self.economia.nivel
    }
    /// Nivel operativo (baja si el mantenimiento no se paga).
    pub fn nivel_actual(&self) -> u32 {
        self.economia.nivel_actual()
    }
}

/// Qué ocurre cuando vence un evento de la cola.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Carga {
    /// El plan llegó a su último vértice.
    Llegada,
    /// Los víveres cruzaron este valor (milésimas).
    Umbral(Mil),
    /// Paso de un minuto de la economía de la plaza (la entidad del evento). Recurrente: cada paso programa el siguiente.
    Minuto,
    /// Acaba una obra, una mejora o un ascenso de la plaza (la entidad del evento; prioridad [`PRIO_FIN_DE_OBRAS`]): cumple lo que vence, libera la
    /// cuadrilla y arranca la cola en ese instante. Una por obra, mejora o ascenso en curso, programada al empezar; no es recurrente.
    FinDeObras,
    /// Caduca una política de la plaza (la entidad del evento; prioridad [`PRIO_CADUCA_POLITICA`]). Una por política activa, programada al activarla.
    CaducaPolitica,
    /// Evento de un [`GeneradorDeEventos`](crate::GeneradorDeEventos): `generador` es su posición en la lista.
    Externa { generador: u16, tipo: u16, a: u64, b: u64 },
    /// Un plazo del comercio (la entidad del evento es el trueque, la orden o la caravana, con la etiqueta de `estado_comercio`).
    /// `aplazos` cuenta las veces que ya se dejó pasar a los de rango menor del mismo instante ([`SucesoComercio::rango`]).
    Comercio { suceso: SucesoComercio, aplazos: u8 },
    /// Cita de la reputación de una Facción (la entidad): cruza el 0 o sale del umbral bajo. Se recalcula tras cada cambio.
    Reputacion(CitaReputacion),
    /// Vence la salida del mundo de un héroe que se desconectó (la entidad es el héroe): `desconectarse` la programa a `retardoDesconexionMs` y
    /// `conectarse` la invalida.
    SalidaDelMundo,
    /// Un plazo de una columna o de una convocatoria (la clave de la entidad depende del suceso).
    Militar(SucesoMilitar),
    /// El primer acercamiento exacto a ≤ `radioEncuentro` (Reloj_Y_Eventos §2, prioridad 8) entre la entidad del evento —la columna o la caravana
    /// cuyo plan acaba de cambiar— y `otra` (una columna, una caravana o una plaza). Vale mientras ninguna de las dos cambie de plan: `otra_generacion`
    /// es la de `otra` al calcularlo (se ignora si `otra` es una plaza, que no se mueve).
    Encuentro { otra: u64, otra_generacion: u32 },
    /// Un plazo del combate: la aparición de bandidos, el fin de la protección o de la ocupación de una plaza, la reposición de la guarnición de
    /// un pueblo (la clave de la entidad depende del suceso).
    Combate(SucesoCombate),
    /// Un plazo de una batalla de Unity (la entidad es `entidad_batalla`; prioridad 1).
    Batalla(SucesoBatalla),
}

/// Qué plazo de una batalla de Unity vence (Batalla_Unity_Ciclo §6 y §3.3). Los tres comparten la entidad de la batalla: cada cambio de estado sube su
/// generación y se reprograman los que sigan haciendo falta, así que a una batalla activa solo le quedan en la cola los suyos y nunca hay uno por entidad ociosa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SucesoBatalla {
    /// Acaba la convocatoria de un asedio (`convocatoria_hasta`, 30 s exactos): el ticket ya puede publicarse y asignarse.
    FinConvocatoria,
    /// Vence el plazo del estado en que está (`expira_en`): pasa a `fallida` sin que nadie pierda nada.
    Vence,
    /// Una cerrada cumple su retención (`conservarCerradasMinutos`): sale del estado.
    Retira,
}

impl SucesoBatalla {
    pub(crate) fn escribir(self, w: &mut Escritor) {
        w.u8(match self {
            SucesoBatalla::FinConvocatoria => 0,
            SucesoBatalla::Vence => 1,
            SucesoBatalla::Retira => 2,
        });
    }
    pub(crate) fn leer(r: &mut Lector) -> Result<Self, ErrorFormato> {
        Ok(match r.u8()? {
            0 => SucesoBatalla::FinConvocatoria,
            1 => SucesoBatalla::Vence,
            2 => SucesoBatalla::Retira,
            _ => return Err(ErrorFormato("suceso de batalla desconocido")),
        })
    }
}

/// Qué plazo del combate vence. Ninguno es recurrente por entidad ociosa: cada uno se programa cuando hay algo que esperar y se detiene solo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SucesoCombate {
    /// Toca que la plaza (la entidad, con su generación) tenga campamento de bandidos: al fundarla y al vencer el plazo de reaparición del que se
    /// destruyó. Prioridad 7. Consume el generador de la partida solo si encuentra sitio.
    BandidosDeLaPlaza,
    /// Toca mirar si al anillo del campamento de mercenarios (la entidad es `entidad_anillo`) le falta un campamento de bandidos. Prioridad 7. Al
    /// crear uno programa la siguiente cita a `reaparicionMinutos`; si ya no falta ninguno, se detiene.
    BandidosDelAnillo,
    /// Acaba la protección de una plaza conquistada (la entidad es `entidad_conquista`). Prioridad 2.
    FinProteccion,
    /// Acaba la ocupación de una plaza conquistada (la entidad es `entidad_conquista`): vuelven su recaudación, su crecimiento y su mantenimiento.
    /// Prioridad 2.
    FinOcupacion,
    /// Se repone la guarnición de serie de un pueblo (la entidad es `entidad_guarnicion`). Prioridad 3 (el consumo de población y de tropa).
    ReponeGuarnicion,
    /// Acaba la invulnerabilidad de un fuerte en ruinas (la entidad es `entidad_avanzada`): vuelve a poder atacarse. Prioridad 2, como el fin de la protección
    /// de una plaza conquistada.
    FinInvulnerabilidad,
    /// Se repone la guarnición de serie de una avanzada (la entidad es `entidad_avanzada`). Prioridad 3, como la de un pueblo.
    ReponeAvanzada,
    /// Acaba la obra de una avanzada (la entidad es `entidad_obra_de_avanzada`): en ese instante exacto la avanzada pasa a operativa, sube de etapa o sale de sus
    /// ruinas. Prioridad 1 ([`PRIO_FIN_DE_OBRAS`]), como el fin de cualquier obra de la plaza.
    FinObraAvanzada,
}

impl SucesoCombate {
    pub(crate) fn escribir(self, w: &mut Escritor) {
        w.u8(match self {
            SucesoCombate::BandidosDeLaPlaza => 0,
            SucesoCombate::BandidosDelAnillo => 1,
            SucesoCombate::FinProteccion => 2,
            SucesoCombate::FinOcupacion => 3,
            SucesoCombate::ReponeGuarnicion => 4,
            SucesoCombate::FinInvulnerabilidad => 5,
            SucesoCombate::ReponeAvanzada => 6,
            SucesoCombate::FinObraAvanzada => 7,
        });
    }
    pub(crate) fn leer(r: &mut Lector) -> Result<Self, ErrorFormato> {
        Ok(match r.u8()? {
            0 => SucesoCombate::BandidosDeLaPlaza,
            1 => SucesoCombate::BandidosDelAnillo,
            2 => SucesoCombate::FinProteccion,
            3 => SucesoCombate::FinOcupacion,
            4 => SucesoCombate::ReponeGuarnicion,
            5 => SucesoCombate::FinInvulnerabilidad,
            6 => SucesoCombate::ReponeAvanzada,
            7 => SucesoCombate::FinObraAvanzada,
            _ => return Err(ErrorFormato("suceso de combate desconocido")),
        })
    }
}

/// Qué plazo militar vence. Ninguno es recurrente por entidad ociosa: la deserción solo existe mientras una columna se está quedando sin tropa
/// por hambre, los tres siguientes duran lo que una formación o una petición de unión (10 s y 10 min) y el último, lo que dure una persecución.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SucesoMilitar {
    /// Toca desertar a alguna escuadra de la columna sin víveres (la entidad es la columna, con su generación: cualquier cambio de ritmo la
    /// reprograma, prioridad 3 como el resto del hambre). Cada escuadra deserta en su propia fase (`Escuadra::desercion_a_partir_de`).
    Desercion,
    /// Vence el plazo de una formación sin llegar al mínimo (la entidad es `entidad_formacion(columna)`): se deshace.
    CaducaFormacion,
    /// Caduca una petición de unión en campo (la entidad es `entidad_peticion_de_union(solicitante)`): se retira y se avisa.
    CaducaPeticion { columna: u64 },
    /// Caduca una petición de unión a una convocatoria (la entidad es `entidad_peticion_de_convocatoria(solicitante)`).
    CaducaPeticionDeConvocatoria { convocatoria: u64 },
    /// La presa de la persecución de la columna (la entidad del evento) sale del alcance de vista de quien la persigue (Movimiento_Ejercitos §7): la
    /// persecución se cancela. Es el primer instante, plan contra plan, en que la distancia pasa de `alcanceDeVista`; se reprograma cada vez que
    /// cualquiera de las dos cambia de plan. `presa_generacion` es la de la presa al calcularlo (se compara al vencer).
    PresaFueraDeVista { presa: u64, presa_generacion: u32 },
}

impl SucesoMilitar {
    pub(crate) fn escribir(self, w: &mut Escritor) {
        match self {
            SucesoMilitar::Desercion => w.u8(0),
            SucesoMilitar::CaducaFormacion => w.u8(1),
            SucesoMilitar::CaducaPeticion { columna } => {
                w.u8(2);
                w.u64(columna);
            }
            SucesoMilitar::CaducaPeticionDeConvocatoria { convocatoria } => {
                w.u8(3);
                w.u64(convocatoria);
            }
            SucesoMilitar::PresaFueraDeVista { presa, presa_generacion } => {
                w.u8(4);
                w.u64(presa);
                w.u32(presa_generacion);
            }
        }
    }
    pub(crate) fn leer(r: &mut Lector) -> Result<Self, ErrorFormato> {
        Ok(match r.u8()? {
            0 => SucesoMilitar::Desercion,
            1 => SucesoMilitar::CaducaFormacion,
            2 => SucesoMilitar::CaducaPeticion { columna: r.u64()? },
            3 => SucesoMilitar::CaducaPeticionDeConvocatoria { convocatoria: r.u64()? },
            4 => SucesoMilitar::PresaFueraDeVista { presa: r.u64()?, presa_generacion: r.u32()? },
            _ => return Err(ErrorFormato("suceso militar desconocido")),
        })
    }
}

/// Qué cita de la reputación de una Facción vence (Reputación como ritmo, Comercio_Caravanas §4.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CitaReputacion {
    /// La reputación negativa sube hasta cruzar el 0: el ritmo cambia de régimen y se reancla.
    CruceDeCero,
    /// La reputación sale del umbral bajo (`umbralBajoParaComision`): deja de pagar el recargo.
    SalidaDelUmbral,
}

impl CitaReputacion {
    pub(crate) fn a_u8(self) -> u8 {
        match self {
            CitaReputacion::CruceDeCero => 0,
            CitaReputacion::SalidaDelUmbral => 1,
        }
    }
    pub(crate) fn de_u8(i: u8) -> Option<Self> {
        match i {
            0 => Some(CitaReputacion::CruceDeCero),
            1 => Some(CitaReputacion::SalidaDelUmbral),
            _ => None,
        }
    }
}

/// Qué plazo de comercio vence. El orden de salida dentro de la prioridad 6 es: llegadas y peajes, fin de preparación, caducidades y reparto
/// (lo que llega cambia de estado antes de lo que caduca): se consigue con [`SucesoComercio::rango`], porque la cola solo ordena por
/// `(instante, prioridad, secuencia)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SucesoComercio {
    /// El fin del tiempo de preparación de una caravana lanzada a mano (la entidad es la caravana): sale con su plan.
    FinPreparacion,
    /// Caducidad de un trueque `propuesto` o `activo` (la entidad es el trueque).
    CaducaTrueque,
    /// Caducidad de una orden de mercado (la entidad es la orden).
    CaducaOrden,
    /// El plan de una caravana llegó a su último vértice (la entidad es la caravana): entrega, regreso o llegada a casa.
    LlegadaCaravana,
    /// El plan de una caravana cruza una ciudad ajena que cobra peaje (la entidad es la caravana; un evento por peaje pendiente).
    PeajeCaravana,
    /// Pulso del reparto automático de una plaza de origen con un lado pendiente sin cargar (la entidad es la plaza); recurrente cada minuto
    /// mientras haya algo que repartir y nada que lo impida, y se detiene solo.
    PulsoReparto,
    /// Reposición de los mercados de mercenarios (la entidad es el mercado, único); solo se programa si algo cambió.
    ReponeMercenarios,
}

impl SucesoComercio {
    /// Posición dentro de la prioridad 6: 0 = llegadas y peajes, 1 = fin de preparación, 2 = caducidades, 3 = reparto y reposición. Un evento
    /// de rango `r` deja pasar `r` veces a los eventos del mismo instante que ya estaban en la cola antes de ejecutarse.
    pub fn rango(self) -> u8 {
        match self {
            SucesoComercio::LlegadaCaravana | SucesoComercio::PeajeCaravana => 0,
            SucesoComercio::FinPreparacion => 1,
            SucesoComercio::CaducaTrueque | SucesoComercio::CaducaOrden => 2,
            SucesoComercio::PulsoReparto | SucesoComercio::ReponeMercenarios => 3,
        }
    }
    pub(crate) fn a_u8(self) -> u8 {
        match self {
            SucesoComercio::FinPreparacion => 0,
            SucesoComercio::CaducaTrueque => 1,
            SucesoComercio::CaducaOrden => 2,
            SucesoComercio::LlegadaCaravana => 3,
            SucesoComercio::PeajeCaravana => 4,
            SucesoComercio::PulsoReparto => 5,
            SucesoComercio::ReponeMercenarios => 6,
        }
    }
    pub(crate) fn de_u8(i: u8) -> Option<Self> {
        match i {
            0 => Some(SucesoComercio::FinPreparacion),
            1 => Some(SucesoComercio::CaducaTrueque),
            2 => Some(SucesoComercio::CaducaOrden),
            3 => Some(SucesoComercio::LlegadaCaravana),
            4 => Some(SucesoComercio::PeajeCaravana),
            5 => Some(SucesoComercio::PulsoReparto),
            6 => Some(SucesoComercio::ReponeMercenarios),
            _ => None,
        }
    }
}

/// Un dato de un hecho que no es un entero suelto (los enteros van en `EventoNucleo::datos`): un texto o una lista (de números, textos o listas). Viaja en el `payload` como su valor de `Struct`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dato {
    Numero(i64),
    Texto(String),
    Lista(Vec<Dato>),
}

/// Las claves de `datos` que nombran a un héroe: lo oye aunque no vea la plaza (como `heroesNombrados` del TypeScript: `heroeId` y `liderId`).
const NOMBRAN_HEROE: [&str; 2] = ["heroe", "lider"];
/// La clave de `datos` que nombra la columna de la que trata el hecho (la que entra, la que conquista...): la oyen sus participantes, como las de `ejercitos`.
const NOMBRA_COLUMNA: &str = "ejercito";
/// Las claves de `datos` que nombran a una Facción: la oye entera (los dos bandos de un combate o de un encuentro, la Facción de un ajuste de reputación...).
const NOMBRAN_FACCION: [&str; 6] = ["faccion", "faccionA", "faccionB", "faccionAtacante", "faccionDefensora", "faccionAnterior"];

/// Un hecho ocurrido (crónica y avisos), no el cambio de estado: ese viaja en las entidades.
///
/// **Quién lo oye** (contrato 05 §8, 4H T5) lo dicen `heroes`, `facciones`, `asentamientos` y `publico`, que se completan al emitirlo ([`EventoNucleo::completar`], con el estado de ese instante: una
/// columna que se disuelve en la misma transición sigue nombrando a sus héroes): los héroes de las columnas que nombra (en `ejercitos` y en la clave `ejercito` de `datos`) y los de las claves `heroe` y
/// `lider`, y las Facciones dueñas de sus plazas y las de las claves de Facción de `datos`. Un hecho sin nadie a quien contárselo solo lo oye la administración; los de todos (`publico`) son solo los que no revelan nada que oculte la niebla.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventoNucleo {
    /// `familia.nombre`, como `ejercito.llega`.
    pub codigo: Cow<'static, str>,
    pub en: Instante,
    /// Columnas implicadas: sus participantes lo oyen.
    pub ejercitos: Vec<u64>,
    /// Plazas a las que se atribuye: lo oyen su Facción y quien está dentro.
    pub asentamientos: Vec<u64>,
    pub punto: Option<Punto>,
    pub datos: Vec<(&'static str, i64)>,
    /// Los datos que no son un entero suelto (textos y listas).
    pub valores: Vec<(&'static str, Dato)>,
    /// Los héroes nombrados (ordenados, sin repetir): lo oyen aunque no vean la plaza.
    pub heroes: Vec<u64>,
    /// Las Facciones que lo oyen enteras (ordenadas, sin repetir).
    pub facciones: Vec<u32>,
    /// Lo oyen todos los jugadores: solo los hechos públicos (la crónica, la reposición de los mercados), que no nombran a nadie ni dicen dónde.
    pub publico: bool,
}

impl EventoNucleo {
    /// Un hecho con sus datos enteros y sin nadie más a quien contárselo que lo que se le añada.
    pub fn nuevo(codigo: impl Into<Cow<'static, str>>, en: Instante, datos: Vec<(&'static str, i64)>) -> Self {
        EventoNucleo { codigo: codigo.into(), en, datos, ..Default::default() }
    }

    /// El dato entero `clave`, si lo lleva.
    pub fn dato(&self, clave: &str) -> Option<i64> {
        self.datos.iter().find(|(k, _)| *k == clave).map(|&(_, v)| v)
    }

    /// Completa a quién se cuenta con el estado del instante en que ocurre: los participantes de sus columnas y los héroes que nombran sus datos, que existan; las Facciones dueñas de sus plazas (también
    /// la de una plaza que acaba de caer en ruinas en la misma transición, `arruinadas`) y las que nombran sus datos (no la «sin bandera» de los bandidos).
    pub(crate) fn completar(&mut self, e: &Estado, arruinadas: &[(u64, u32)]) {
        let columna = self.dato(NOMBRA_COLUMNA).map(|c| c as u64);
        let participantes = self.ejercitos.iter().chain(&columna).filter_map(|id| e.ejercitos.get(id)).flat_map(|x| x.columna.participantes.iter().map(|p| p.heroe));
        let nombrados = self.datos.iter().filter(|(k, _)| NOMBRAN_HEROE.contains(k)).map(|&(_, v)| v as u64);
        let heroes: Vec<u64> = participantes.chain(nombrados).filter(|h| e.militar.heroes.contains_key(h)).collect();
        self.heroes.extend(heroes);
        self.heroes.sort_unstable();
        self.heroes.dedup();
        let duena = |p: &u64| e.asentamientos.get(p).map(|a| a.faccion).or_else(|| arruinadas.iter().find(|(x, _)| x == p).map(|&(_, f)| f));
        let duenas = self.asentamientos.iter().filter_map(duena);
        let nombradas = self.datos.iter().filter(|(k, _)| NOMBRAN_FACCION.contains(k)).filter_map(|&(_, v)| u32::try_from(v).ok());
        let facciones: Vec<u32> = duenas.chain(nombradas).filter(|&f| f != SIN_BANDERA).collect();
        self.facciones.extend(facciones);
        self.facciones.sort_unstable();
        self.facciones.dedup();
    }
}

/// La clase de una entidad con identidad propia en el contrato 05 (las que un cambio de estado puede tocar). Los ids son los del núcleo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Clase {
    Ejercito,
    Asentamiento,
    /// Lo que cambia del interior de una plaza sin cambiar su ficha (id de la plaza): quién y qué hay dentro (los héroes presentes, los residentes, la guarnición, las caravanas aparcadas y la flota). Solo lo lee
    /// `ASENTAMIENTO`, así que no mueve la ficha de `FACCION` ni los derechos. Solo se marca si hay sesiones (`Nucleo::seguir_pertenencias`) y siempre dentro de una versión que ya se cierra por otra razón (un héroe o una
    /// caravana tocados). **No lo marca el paso de minuto**: lo que este cambia en la plaza (almacén, población, medidor, nutrición, tendencia, almacén local de sus pueblos) lleva su propia secuencia por plaza,
    /// `Nucleo::version_interior`, y no sube la versión del mundo.
    Interior,
    /// Su nivel, reputación, tecnologías, relaciones y ciudadanos.
    Faccion,
    Heroe,
    /// Las escuadras de un héroe tocado se dan por tocadas (viajan con él).
    Escuadra,
    Caravana,
    Acuerdo,
    OrdenMercado,
    Pueblo,
    Avanzada,
    /// Campamento de mercenarios (su índice del artefacto de mundo).
    Campamento,
    Bandidos,
    Convocatoria,
    Batalla,
    /// Una relación entre dos Facciones (el stub de administración `FijarRelacion`): su id es el par `(menor, mayor)` empaquetado por [`Clase::par`].
    Relacion,
}

impl Clase {
    /// El id de una relación entre dos Facciones: `(menor << 32) | mayor`.
    pub fn par(a: u32, b: u32) -> u64 {
        (u64::from(a.min(b)) << 32) | u64::from(a.max(b))
    }

    /// Las dos Facciones de una relación (la inversa de [`Clase::par`]).
    pub fn facciones_del_par(par: u64) -> (u32, u32) {
        ((par >> 32) as u32, par as u32)
    }

    pub const TODAS: [Clase; 16] = [
        Clase::Ejercito,
        Clase::Asentamiento,
        Clase::Interior,
        Clase::Faccion,
        Clase::Heroe,
        Clase::Escuadra,
        Clase::Caravana,
        Clase::Acuerdo,
        Clase::OrdenMercado,
        Clase::Pueblo,
        Clase::Avanzada,
        Clase::Campamento,
        Clase::Bandidos,
        Clase::Convocatoria,
        Clase::Batalla,
        Clase::Relacion,
    ];
}

/// Las entidades cuyo estado cambió en una versión, por clase. Al cerrar la versión cada lista queda ordenada y sin repetir.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tocados([Vec<u64>; 16]);

impl Tocados {
    pub fn marcar(&mut self, clase: Clase, id: u64) {
        self.0[clase as usize].push(id);
    }
    pub fn de(&self, clase: Clase) -> &[u64] {
        &self.0[clase as usize]
    }
    pub fn esta_vacio(&self) -> bool {
        self.0.iter().all(Vec::is_empty)
    }
    /// Ordena y quita los repetidos de cada lista.
    pub(crate) fn cerrar(&mut self) {
        for ids in &mut self.0 {
            ids.sort_unstable();
            ids.dedup();
        }
    }
    pub fn contiene(&self, clase: Clase, id: u64) -> bool {
        self.de(clase).contains(&id)
    }
}

/// A qué ámbitos pertenece una entidad al cerrarse una versión y de cuáles salió (4H, T3b): es lo que dice **quién recibe qué**. Solo entran las entidades que viajan a los ámbitos de
/// un jugador (`HEROE`, `FACCION`, `ASENTAMIENTO` y `BATALLA`; la `VISTA` la decide la niebla, no el registro) y solo las tocadas en la versión o las que cambiaron de ámbito sin que nadie las tocara (los pueblos de una plaza
/// que cambia de dueño). `quitan` son los ámbitos que tenían la entidad antes y ya no (porque desapareció o porque dejó de ser suya): reciben un `QUITA`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pertenencia {
    pub clase: Clase,
    pub id: u64,
    pub ahora: Vec<Ambito>,
    pub quitan: Vec<Ambito>,
}

/// Registro de cambios de una versión: lo que alimenta `Delta` y `Foto`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CambioVersion {
    pub version: u64,
    pub en: Instante,
    /// Las entidades cuyo estado cambió, por clase (ordenadas, sin repetir).
    pub tocados: Tocados,
    /// A qué ámbitos de jugador pertenece cada entidad que viaja a ellos y de cuáles salió, por clase e id.
    pub pertenencias: Vec<Pertenencia>,
    pub eventos: Vec<EventoNucleo>,
}

pub struct Estado {
    pub reglas: Reglas,
    /// Hash del artefacto de mundo con el que se calcularon las rutas: reproducir sobre otro mundo no tiene sentido.
    pub mundo_hash: String,
    pub version: u64,
    pub ahora: Instante,
    pub factor_milesimas: u32,
    pub pausado: bool,
    pub rng: Rng,
    pub siguiente_id: u64,
    pub ejercitos: BTreeMap<u64, Ejercito>,
    pub asentamientos: BTreeMap<u64, Asentamiento>,
    /// Parcela de bosque (índice del artefacto) → asentamiento que la explota.
    pub parcelas_ocupadas: BTreeMap<u32, u64>,
    /// Nodo de piedra, mineral o ganado (índice de `economia::Mapa::nodos`) → asentamiento que lo explota (con un edificio o con un pueblo
    /// extractor). Se reconstruye de las fuentes de los edificios y de los pueblos, igual que `parcelas_ocupadas` de las parcelas.
    pub nodos_reclamados: BTreeMap<u32, u64>,
    /// Stock de los nodos que alguien ha tocado (los demás tienen su cantidad inicial).
    pub yacimientos: BTreeMap<u32, StockNodo>,
    /// Pares de plazas `(menor id, mayor id)` de Facciones distintas cuyas zonas ya se han tocado (nunca se separan).
    pub contactos: BTreeSet<(u64, u64)>,
    /// Nivel de cada Facción (1 si no consta): fija su cap de fundación. Lo gobernará la experiencia de Facción.
    pub niveles_faccion: BTreeMap<u32, u32>,
    /// Trueques, órdenes de mercado, flota de caravanas y reputación entre Facciones (Comercio_Caravanas §2).
    pub comercio: EstadoComercio,
    /// Héroes con sus escuadras, reclutas de los campamentos y tecnologías de las Facciones (Heroe_Y_Tropa, Campamentos_Mercenarios).
    pub militar: EstadoMilitar,
    /// Campamentos de bandidos, guarnición de serie de los pueblos y rendimientos de lo que se destruye (Combate, Generacion_Del_Mundo §13).
    pub combate: EstadoCombate,
    /// Las batallas de Unity abiertas y las cerradas recientes (Batalla_Unity_Ciclo §3).
    pub batallas: EstadoBatallas,
    /// Lo que cada Facción ha explorado y recuerda de lo ajeno (Niebla_De_Guerra §2, §4).
    pub memoria: Memoria,
    pub generaciones: Generaciones,
    pub cola: Cola<Carga>,
    /// `(actor, peticion_id)` en orden de llegada, con la versión que produjeron.
    vistos: VecDeque<(String, String)>,
    vistos_version: HashMap<(String, String), u64>,
}

impl Estado {
    pub fn nuevo(reglas: Reglas, mundo_hash: String, semilla: u64, ahora: Instante) -> Self {
        let mut comercio = EstadoComercio::default();
        comercio.mercenarios.mercado = comercio::mercenarios::MercadoMercenario::nuevo(&reglas.economia.comercio.mercenarios, ahora);
        Self {
            reglas,
            mundo_hash,
            version: 0,
            ahora,
            factor_milesimas: 1000,
            pausado: false,
            rng: Rng::derivado(semilla, "nucleo"),
            siguiente_id: 1,
            ejercitos: BTreeMap::new(),
            asentamientos: BTreeMap::new(),
            parcelas_ocupadas: BTreeMap::new(),
            nodos_reclamados: BTreeMap::new(),
            yacimientos: BTreeMap::new(),
            contactos: BTreeSet::new(),
            niveles_faccion: BTreeMap::new(),
            comercio,
            militar: EstadoMilitar::default(),
            combate: EstadoCombate::default(),
            batallas: EstadoBatallas::default(),
            memoria: Memoria::default(),
            generaciones: Generaciones::nuevas(),
            cola: Cola::nueva(),
            vistos: VecDeque::new(),
            vistos_version: HashMap::new(),
        }
    }

    /// La columna de un héroe: aquella en la que va o con la que entró de visita en un lugar ajeno (dentro, fuera del mapa).
    pub fn ejercito_de_heroe(&self, heroe: u64) -> Option<u64> {
        self.militar.heroes.get(&heroe)?.columna
    }

    /// Versión que produjo una petición ya aceptada.
    pub fn visto(&self, actor: &str, peticion_id: &str) -> Option<u64> {
        self.vistos_version.get(&(actor.to_owned(), peticion_id.to_owned())).copied()
    }

    pub(crate) fn recordar(&mut self, actor: &str, peticion_id: &str, version: u64) {
        let clave = (actor.to_owned(), peticion_id.to_owned());
        if self.vistos_version.insert(clave.clone(), version).is_none() {
            self.vistos.push_back(clave);
        }
        while self.vistos.len() > self.reglas.limite_idempotencia {
            if let Some(vieja) = self.vistos.pop_front() {
                self.vistos_version.remove(&vieja);
            }
        }
    }

    /// Estado canónico en bytes: orden fijo (ejércitos por id, cola por `(instante, prioridad, secuencia)`), solo eventos
    /// vigentes (los invalidados que aún no salieron de la cola no son estado: depende de cuándo se descartaron).
    pub fn serializar(&self) -> Vec<u8> {
        let mut w = Escritor::default();
        w.0.extend_from_slice(MAGIA);
        w.texto(&self.mundo_hash);
        let r = &self.reglas;
        w.u32(r.viveres_bajos_pct);
        w.u64(r.limite_idempotencia as u64);
        w.bool(r.batallas_en_unity);
        let t = &r.territorio;
        w.i64(t.radio_inicial);
        w.u32(t.techos.len() as u32);
        t.techos.iter().for_each(|&x| w.i64(x));
        w.i64(t.crecimiento_por_edificio);
        w.i64(t.separacion_plazas);
        w.i64(t.exclusion_campamento);
        w.u32(t.cap_por_nivel.len() as u32);
        t.cap_por_nivel.iter().for_each(|&x| w.u32(x));
        w.u32(t.cuota_parcelas_pct);
        w.texto(r.economia.huella());
        w.u64(self.version);
        w.i64(self.ahora);
        w.u32(self.factor_milesimas);
        w.bool(self.pausado);
        w.u64(self.rng.estado());
        w.u64(self.siguiente_id);

        w.u64(self.ejercitos.len() as u64);
        for e in self.ejercitos.values() {
            codec_columna::escribir(&mut w, e);
        }

        w.u64(self.asentamientos.len() as u64);
        for a in self.asentamientos.values() {
            w.u64(a.id);
            w.u32(a.faccion);
            w.punto(a.posicion);
            w.i64(a.fundado_en);
            w.i64(a.radio);
            w.u32(a.parcelas.len() as u32);
            a.parcelas.iter().for_each(|&p| w.u32(p));
            w.u64(a.version);
            codec_economia::escribir(&mut w, &a.economia);
            codec_ciudad::escribir(&mut w, a.urbe.ciudad());
            w.opt_i64(a.protegida_hasta);
            w.opt_i64(a.ocupada_hasta);
            w.bool(a.acceso.aliados);
            w.bool(a.acceso.neutrales);
            w.bool(a.acceso.enemigos);
        }
        w.u64(self.yacimientos.len() as u64);
        for (&nodo, st) in &self.yacimientos {
            w.u32(nodo);
            w.i64(st.stock);
            w.opt_i64(st.vacio_desde);
        }
        w.u64(self.contactos.len() as u64);
        for &(a, b) in &self.contactos {
            w.u64(a);
            w.u64(b);
        }
        w.u64(self.niveles_faccion.len() as u64);
        for (&f, &n) in &self.niveles_faccion {
            w.u32(f);
            w.u32(n);
        }
        codec_comercio::escribir(&mut w, &self.comercio);
        codec_militar::escribir(&mut w, &self.militar);
        codec_combate::escribir(&mut w, &self.combate);
        codec_batallas::escribir(&mut w, &self.batallas);
        memoria::escribir(&mut w, &self.memoria);

        let gens = self.generaciones.volcar();
        w.u64(gens.len() as u64);
        for (id, g) in gens {
            w.u64(id);
            w.u32(g);
        }

        let eventos: Vec<_> = self.cola.volcar().into_iter().filter(|e| self.generaciones.es_vigente(e.clave)).collect();
        w.u64(self.cola.siguiente_secuencia());
        w.u64(eventos.len() as u64);
        for e in &eventos {
            w.i64(e.instante);
            w.u8(e.prioridad);
            w.u64(e.secuencia);
            w.u64(e.clave.entidad);
            w.u32(e.clave.generacion);
            match &e.carga {
                Carga::Llegada => w.u8(0),
                Carga::Umbral(v) => {
                    w.u8(1);
                    w.i64(*v);
                }
                Carga::Minuto => w.u8(3),
                Carga::FinDeObras => w.u8(11),
                Carga::CaducaPolitica => w.u8(12),
                Carga::Comercio { suceso, aplazos } => {
                    w.u8(4);
                    w.u8(suceso.a_u8());
                    w.u8(*aplazos);
                }
                Carga::Reputacion(c) => {
                    w.u8(5);
                    w.u8(c.a_u8());
                }
                Carga::SalidaDelMundo => w.u8(6),
                Carga::Militar(s) => {
                    w.u8(7);
                    s.escribir(&mut w);
                }
                Carga::Encuentro { otra, otra_generacion } => {
                    w.u8(8);
                    w.u64(*otra);
                    w.u32(*otra_generacion);
                }
                Carga::Combate(s) => {
                    w.u8(9);
                    s.escribir(&mut w);
                }
                Carga::Batalla(s) => {
                    w.u8(10);
                    s.escribir(&mut w);
                }
                Carga::Externa { generador, tipo, a, b } => {
                    w.u8(2);
                    w.u16(*generador);
                    w.u16(*tipo);
                    w.u64(*a);
                    w.u64(*b);
                }
            }
        }

        w.u64(self.vistos.len() as u64);
        for clave in &self.vistos {
            w.texto(&clave.0);
            w.texto(&clave.1);
            w.u64(self.vistos_version[clave]);
        }
        w.0
    }

    pub fn hash(&self) -> [u8; 32] {
        Sha256::digest(self.serializar()).into()
    }

    /// Inversa de [`Estado::serializar`]. Falla si las reglas o el mundo de la foto no son los que se piden.
    pub fn deserializar(bytes: &[u8], reglas: &Reglas, mundo_hash: &str) -> Result<Estado, ErrorFormato> {
        let mut r = Lector::nuevo(bytes);
        // La cabecera es de 5 bytes (`BAN23`): los cuatro primeros dicen la familia («BAN2» son los formatos 20 a 23, y solo el 23 se admite; «BAN1» son los formatos 10 a 19, ya no admitidos) y el quinto el formato.
        let magia = r.magia()?;
        if magia == MAGIA[..4] {
            match r.u8()? {
                b'3' => {}
                b'2' => return Err(ErrorFormato("foto del formato BAN22 (sin la memoria de la niebla: lo explorado por cada Facción y las fichas de lo ajeno que vio, que no se pueden deducir del estado): ya no se admite")),
                b'1' => return Err(ErrorFormato("foto del formato BAN21 (las plazas no traen la tendencia «+N/min» del último paso de minuto, que no se puede deducir del estado): ya no se admite")),
                b'0' => return Err(ErrorFormato("foto del formato BAN20 (el ticket de una batalla no congela la plaza del asedio ni el nivel, la experiencia y la moral de cada escuadra, y el informe de inspección de una plaza no trae sus recintos): ya no se admite")),
                _ => return Err(ErrorFormato("no es un estado del núcleo")),
            }
        } else {
            match (magia == *b"BAN1").then(|| r.u8()).transpose()? {
                Some(b'0') => return Err(ErrorFormato("foto del formato BAN10 (sin persecuciones ni informes de inspección, con los encuentros en un generador externo): ya no se admite")),
                Some(b'1') => return Err(ErrorFormato("foto del formato BAN11 (sin estado de combate, sin protección ni ocupación de las plazas y con una huella de reglas que no incluye las del combate): ya no se admite")),
                Some(b'2') => return Err(ErrorFormato("foto del formato BAN12 (sin batallas de Unity, sin la regla batallas_en_unity y con las columnas apartadas sin su plan en pausa): ya no se admite")),
                Some(b'3') => return Err(ErrorFormato("foto del formato BAN13 (población continua en milésimas, sin resto ni marca de recuperación del hambre y sin las citas de fin de obra y de política): ya no se admite")),
                Some(b'4') => return Err(ErrorFormato("foto del formato BAN14 (sin avanzadas, sin guarnición de avanzada en las escuadras y con una huella de reglas que no incluye las cifras de las avanzadas): ya no se admite")),
                Some(b'5') => return Err(ErrorFormato("foto del formato BAN15 (el fin de la obra de una avanzada espera en la cita de su plaza, sin cita propia que se pueda cancelar, y con las cifras de las avanzadas de antes de su tamaño): ya no se admite")),
                Some(b'6') => return Err(ErrorFormato("foto del formato BAN16 (las plazas no traen el acceso por relación de Entrar con todo): ya no se admite")),
                Some(b'7') => return Err(ErrorFormato("foto del formato BAN17 (los héroes no pueden estar dentro de un pueblo extractor ni de una avanzada): ya no se admite")),
                Some(b'8') => return Err(ErrorFormato("foto del formato BAN18 (el contexto de una batalla de caravana nombra una sola caravana, no la lista de las que se juegan): ya no se admite")),
                Some(b'9') => return Err(ErrorFormato("foto del formato BAN19 (los recintos de muralla se guardan con la ciudad de la plaza, solo muros, puertas y si están completos, sin nivel, avance ni mejora): ya no se admite")),
                _ => return Err(ErrorFormato("no es un estado del núcleo")),
            }
        }
        if r.texto()? != mundo_hash {
            return Err(ErrorFormato("la foto es de otro mundo (hash del artefacto distinto)"));
        }
        let guardadas = Reglas {
            viveres_bajos_pct: r.u32()?,
            limite_idempotencia: r.u64()? as usize,
            batallas_en_unity: r.bool()?,
            territorio: {
                let radio_inicial = r.i64()?;
                let mut techos = Vec::new();
                for _ in 0..r.u32()? {
                    techos.push(r.i64()?);
                }
                let (crecimiento_por_edificio, separacion_plazas, exclusion_campamento) = (r.i64()?, r.i64()?, r.i64()?);
                let mut cap_por_nivel = Vec::new();
                for _ in 0..r.u32()? {
                    cap_por_nivel.push(r.u32()?);
                }
                ReglasTerritorio { radio_inicial, techos, crecimiento_por_edificio, separacion_plazas, exclusion_campamento, cap_por_nivel, cuota_parcelas_pct: r.u32()? }
            },
            economia: {
                let huella = r.texto()?;
                if huella != reglas.economia.huella() {
                    return Err(ErrorFormato("las reglas de economía de la foto no son las del documento incrustado (huella distinta)"));
                }
                reglas.economia.clone()
            },
        };
        if &guardadas != reglas {
            return Err(ErrorFormato("las reglas de la foto no coinciden con las pedidas"));
        }
        let mut s = Estado::nuevo(guardadas, mundo_hash.to_owned(), 0, 0);
        s.version = r.u64()?;
        s.ahora = r.i64()?;
        s.factor_milesimas = r.u32()?;
        s.pausado = r.bool()?;
        s.rng = Rng::nuevo(r.u64()?);
        s.siguiente_id = r.u64()?;

        for _ in 0..r.cuenta()? {
            let e = codec_columna::leer(&mut r)?;
            s.ejercitos.insert(e.id, e);
        }

        for _ in 0..r.cuenta()? {
            let (id, faccion, posicion, fundado_en) = (r.u64()?, r.u32()?, r.punto()?, r.i64()?);
            let radio = r.i64()?;
            let mut parcelas = BTreeSet::new();
            for _ in 0..r.u32()? {
                let p = r.u32()?;
                s.parcelas_ocupadas.insert(p, id);
                parcelas.insert(p);
            }
            let version = r.u64()?;
            let economia = codec_economia::leer(&mut r)?;
            let urbe = Urbe::desde_ciudad(codec_ciudad::leer(&mut r, asentamiento::recintos::proyeccion(&economia.recintos))?);
            let (protegida_hasta, ocupada_hasta) = (r.opt_i64()?, r.opt_i64()?);
            let acceso = Acceso { aliados: r.bool()?, neutrales: r.bool()?, enemigos: r.bool()? };
            for e in &economia.edificios {
                if let Some(asentamiento::edificio::Fuente::Nodo(n)) = e.fuente {
                    s.nodos_reclamados.insert(n, id);
                }
            }
            for p in &economia.pueblos {
                s.nodos_reclamados.insert(p.nodo, id);
            }
            s.asentamientos.insert(id, Asentamiento { id, faccion, posicion, fundado_en, radio, parcelas, version, economia, urbe, protegida_hasta, ocupada_hasta, acceso });
        }
        for _ in 0..r.cuenta()? {
            let nodo = r.u32()?;
            s.yacimientos.insert(nodo, StockNodo { stock: r.i64()?, vacio_desde: r.opt_i64()? });
        }
        for _ in 0..r.cuenta()? {
            s.contactos.insert((r.u64()?, r.u64()?));
        }
        for _ in 0..r.cuenta()? {
            s.niveles_faccion.insert(r.u32()?, r.u32()?);
        }
        s.comercio = codec_comercio::leer(&mut r, &s.reglas.economia.comercio)?;
        s.militar = codec_militar::leer(&mut r)?;
        s.combate = codec_combate::leer(&mut r)?;
        s.batallas = codec_batallas::leer(&mut r)?;
        s.memoria = memoria::leer(&mut r)?;

        let mut gens = Vec::new();
        for _ in 0..r.cuenta()? {
            gens.push((r.u64()?, r.u32()?));
        }
        s.generaciones = Generaciones::restaurar(gens);

        let siguiente_secuencia = r.u64()?;
        let mut eventos = Vec::new();
        for _ in 0..r.cuenta()? {
            let (instante, prioridad, secuencia) = (r.i64()?, r.u8()?, r.u64()?);
            let clave = Clave { entidad: r.u64()?, generacion: r.u32()? };
            let carga = match r.u8()? {
                0 => Carga::Llegada,
                1 => Carga::Umbral(r.i64()?),
                3 => Carga::Minuto,
                11 => Carga::FinDeObras,
                12 => Carga::CaducaPolitica,
                4 => Carga::Comercio { suceso: SucesoComercio::de_u8(r.u8()?).ok_or(ErrorFormato("suceso de comercio desconocido"))?, aplazos: r.u8()? },
                5 => Carga::Reputacion(CitaReputacion::de_u8(r.u8()?).ok_or(ErrorFormato("cita de reputación desconocida"))?),
                6 => Carga::SalidaDelMundo,
                7 => Carga::Militar(SucesoMilitar::leer(&mut r)?),
                8 => Carga::Encuentro { otra: r.u64()?, otra_generacion: r.u32()? },
                9 => Carga::Combate(SucesoCombate::leer(&mut r)?),
                10 => Carga::Batalla(SucesoBatalla::leer(&mut r)?),
                2 => Carga::Externa { generador: r.u16()?, tipo: r.u16()?, a: r.u64()?, b: r.u64()? },
                _ => return Err(ErrorFormato("tipo de carga desconocido")),
            };
            eventos.push(Evento { instante, prioridad, secuencia, clave, carga });
        }
        s.cola = Cola::restaurar(eventos, siguiente_secuencia);

        for _ in 0..r.cuenta()? {
            let (actor, peticion, version) = (r.texto()?, r.texto()?, r.u64()?);
            s.recordar(&actor, &peticion, version);
        }
        r.fin()?;
        Ok(s)
    }
}
