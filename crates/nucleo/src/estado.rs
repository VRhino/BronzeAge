//! El estado determinista del mundo: arena de ejércitos, cola de eventos, generaciones, azar, versión y reloj. Es TODO lo
//! que entra en `hash_estado` y en las fotos; lo que no está aquí (sesiones, registro de cambios, métricas) se reconstruye.

use crate::codec::{ErrorFormato, Escritor, Lector};
use crate::codec_ciudad;
use crate::codec_comercio;
use crate::codec_economia;
use crate::economia::StockNodo;
use crate::estado_comercio::EstadoComercio;
use crate::urbanismo::{Urbanismo, Urbe};
use base::{Instante, Mil, Plan, Punto, Ritmo, Rng, Vertice};
use cola::{Clave, Cola, Evento, Generaciones};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::{Arc, OnceLock};

/// Prioridad de los umbrales de víveres: «consumo de población y de tropa» (Reloj_Y_Eventos §2).
pub const PRIO_UMBRAL: u8 = 3;
/// Prioridad del comercio: «comercio, trueques, órdenes de mercado, caravanas» (Reloj_Y_Eventos §2). Va después de los mantenimientos y las ruinas
/// (5): una plaza arruinada ya no existe para el comercio de ese instante. Dentro de la prioridad el orden es el de [`SucesoComercio::rango`].
pub const PRIO_COMERCIO: u8 = 6;
/// Prioridad de las llegadas: «llegadas y encuentros de columnas».
pub const PRIO_LLEGADA: u8 = 8;
/// Prioridad de la reputación: «diplomacia, tributos, experiencia, reputación».
pub const PRIO_REPUTACION: u8 = 9;
/// Prioridad del paso de un minuto de una plaza: la de «fin de obras, de ascensos y de políticas» (Reloj_Y_Eventos §2). El paso es
/// atómico y, dentro de él, el crate `asentamiento` ya respeta el orden de la regla: construcción y políticas, ascenso, comida,
/// crecimiento y recaudación, y por último mantenimiento y ruinas. Con la prioridad 2 una obra que acaba en el instante del paso usa las
/// políticas antes de que expiren, y la plaza come antes que las tropas (3) y que el comercio (6), que aún no existen.
pub const PRIO_ASENTAMIENTO: u8 = 2;

/// BAN5: BAN4 más el comercio (trueques, órdenes, flota, reputación) y los eventos de comercio de la cola. BAN4 se rechaza.
const MAGIA: &[u8; 4] = b"BAN5";
const MAGIA_ANTERIOR: &[u8; 4] = b"BAN4";

/// Reglas de juego mínimas del núcleo genérico. Entran en el hash y en las fotos: dos mundos con reglas distintas no se
/// pueden confundir (recuperar con otras reglas es un error).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reglas {
    /// Víveres que gasta una columna en marcha, en milésimas por minuto (≈ 0,83: Movimiento_Ejercitos §6).
    pub consumo_marcha: Mil,
    /// Ídem acampada.
    pub consumo_estacionado: Mil,
    /// Aviso de víveres bajos al caer por debajo de este porcentaje de la capacidad (0 = sin aviso).
    pub viveres_bajos_pct: u32,
    /// Peticiones recordadas para la idempotencia (`INFRA.limiteIdempotencia`).
    pub limite_idempotencia: usize,
    pub territorio: ReglasTerritorio,
    /// Las reglas de población, economía y construcción de las plazas (Economia_Construccion).
    pub economia: ReglasEconomia,
}

impl Default for Reglas {
    fn default() -> Self {
        Self { consumo_marcha: 830, consumo_estacionado: 83, viveres_bajos_pct: 25, limite_idempotencia: 500, territorio: ReglasTerritorio::default(), economia: ReglasEconomia::default() }
    }
}

/// Las reglas del crate `asentamiento` (datos del documento de Mechanics, ya convertidos a enteros), las huellas del trazado urbano
/// (bloques del documento `Trazado_Urbano.md`) y las del comercio (crate `comercio`). No se serializan: en las fotos va su **huella**, el
/// sha256 de los bloques de parámetros de los documentos incrustados (no de su prosa); recuperar con otras reglas falla igual que con
/// `ReglasTerritorio`.
#[derive(Clone, Debug)]
pub struct ReglasEconomia {
    pub datos: Arc<asentamiento::Reglas>,
    /// Tipo → huella y categoría de la ciudad de cada plaza.
    pub urbanismo: Arc<Urbanismo>,
    /// Las cifras del comercio (precios, comisiones, trueques, mercado, flota, reputación).
    pub comercio: Arc<comercio::Reglas>,
    huella: String,
}

impl ReglasEconomia {
    /// Carga las cifras de los documentos incrustados (parsea los bloques JSON: hágase una vez).
    pub fn cargar() -> Self {
        let bloques = asentamiento::datos::texto_de_parametros() + &asentamiento::datos::texto_de_parametros_de(crate::urbanismo::DOC_TRAZADO) + &comercio::datos::texto_de_parametros();
        let huella = crate::hex(&Sha256::digest(bloques).into());
        let datos = asentamiento::Reglas::cargar();
        let urbanismo = Arc::new(Urbanismo::cargar(&datos));
        Self { datos: Arc::new(datos), urbanismo, comercio: Arc::new(comercio::Reglas::cargar()), huella }
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

/// Una columna mínima: lo justo para demostrar planes (movimiento) y ritmos (víveres).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ejercito {
    pub id: u64,
    /// Héroe que la dirige (`Orden*.heroe_id`).
    pub lider: String,
    /// Cliente (cuenta) al que obedece; solo él (o un administrador) puede darle órdenes.
    pub propietario: String,
    pub faccion: u32,
    /// Milésimas de unidad por segundo, antes del coste del terreno (30 unidades/min = 500).
    pub velocidad: Mil,
    /// Dónde está cuando no tiene plan.
    pub posicion: Punto,
    /// Sin vértices = quieta en `posicion`.
    pub plan: Plan,
    pub viveres: Ritmo,
    pub capacidad_viveres: Mil,
    /// Lleva enganchada una Caravana de Fundación (Doc 1.8): puede fundar donde está. La compra y el lanzamiento son de
    /// la economía; hasta entonces la da la administración.
    pub caravana_fundacion: bool,
    /// Versión de mundo del último cambio.
    pub version: u64,
}

impl Ejercito {
    pub fn marchando(&self) -> bool {
        !self.plan.vertices.is_empty()
    }
    /// Posición en `t`, evaluada del plan vigente (o la fija si está quieta).
    pub fn posicion_en(&self, t: Instante) -> Punto {
        self.plan.posicion_en(t).unwrap_or(self.posicion)
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
    /// Evento de un [`GeneradorDeEventos`](crate::GeneradorDeEventos): `generador` es su posición en la lista.
    Externa { generador: u16, tipo: u16, a: u64, b: u64 },
    /// Un plazo del comercio (la entidad del evento es el trueque, la orden o la caravana, con la etiqueta de `estado_comercio`).
    /// `aplazos` cuenta las veces que ya se dejó pasar a los de rango menor del mismo instante ([`SucesoComercio::rango`]).
    Comercio { suceso: SucesoComercio, aplazos: u8 },
    /// Cita de la reputación de una Facción (la entidad): cruza el 0 o sale del umbral bajo. Se recalcula tras cada cambio.
    Reputacion(CitaReputacion),
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

/// Qué plazo de comercio vence. El orden de salida dentro de la prioridad 6 es: llegadas, fin de preparación, caducidades y reparto
/// (lo que llega cambia de estado antes de lo que caduca): se consigue con [`SucesoComercio::rango`], porque la cola solo ordena por
/// `(instante, prioridad, secuencia)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SucesoComercio {
    /// El fin del tiempo de preparación de una caravana lanzada a mano (la entidad es la caravana).
    FinPreparacion,
    /// Caducidad de un trueque `propuesto` o `activo` (la entidad es el trueque).
    CaducaTrueque,
    /// Caducidad de una orden de mercado (la entidad es la orden).
    CaducaOrden,
}

impl SucesoComercio {
    /// Posición dentro de la prioridad 6 (0 = llegadas, aún sin implementar; 3 = reparto, ídem). Un evento de rango `r` deja pasar `r`
    /// veces a los eventos del mismo instante que ya estaban en la cola antes de ejecutarse.
    pub fn rango(self) -> u8 {
        match self {
            SucesoComercio::FinPreparacion => 1,
            SucesoComercio::CaducaTrueque | SucesoComercio::CaducaOrden => 2,
        }
    }
    pub(crate) fn a_u8(self) -> u8 {
        match self {
            SucesoComercio::FinPreparacion => 0,
            SucesoComercio::CaducaTrueque => 1,
            SucesoComercio::CaducaOrden => 2,
        }
    }
    pub(crate) fn de_u8(i: u8) -> Option<Self> {
        match i {
            0 => Some(SucesoComercio::FinPreparacion),
            1 => Some(SucesoComercio::CaducaTrueque),
            2 => Some(SucesoComercio::CaducaOrden),
            _ => None,
        }
    }
}

/// Un hecho ocurrido (crónica y avisos), no el cambio de estado: ese viaja en las entidades.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventoNucleo {
    /// `familia.nombre`, como `ejercito.llega`.
    pub codigo: Cow<'static, str>,
    pub en: Instante,
    /// Columnas implicadas: deciden quién lo oye.
    pub ejercitos: Vec<u64>,
    /// Plazas implicadas.
    pub asentamientos: Vec<u64>,
    pub punto: Option<Punto>,
    pub datos: Vec<(&'static str, i64)>,
}

/// Registro de cambios de una versión: lo que alimenta `Delta` y `Foto`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CambioVersion {
    pub version: u64,
    pub en: Instante,
    /// Ejércitos cuyo estado cambió (ordenados, sin repetir).
    pub ejercitos: Vec<u64>,
    /// Plazas cuyo estado cambió (ordenadas, sin repetir).
    pub asentamientos: Vec<u64>,
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
    /// Nodo de piedra o ganado (índice de `economia::Mapa::nodos`) → asentamiento que lo explota. Se reconstruye de las fuentes de los
    /// edificios, igual que `parcelas_ocupadas` de las parcelas.
    pub nodos_reclamados: BTreeMap<u32, u64>,
    /// Stock de los nodos que alguien ha tocado (los demás tienen su cantidad inicial).
    pub yacimientos: BTreeMap<u32, StockNodo>,
    /// Pares de plazas `(menor id, mayor id)` de Facciones distintas cuyas zonas ya se han tocado (nunca se separan).
    pub contactos: BTreeSet<(u64, u64)>,
    /// Nivel de cada Facción (1 si no consta): fija su cap de fundación. Lo gobernará la experiencia de Facción.
    pub niveles_faccion: BTreeMap<u32, u32>,
    /// Trueques, órdenes de mercado, flota de caravanas y reputación entre Facciones (Comercio_Caravanas §2).
    pub comercio: EstadoComercio,
    pub generaciones: Generaciones,
    pub cola: Cola<Carga>,
    por_lider: HashMap<String, u64>,
    /// `(actor, peticion_id)` en orden de llegada, con la versión que produjeron.
    vistos: VecDeque<(String, String)>,
    vistos_version: HashMap<(String, String), u64>,
}

impl Estado {
    pub fn nuevo(reglas: Reglas, mundo_hash: String, semilla: u64, ahora: Instante) -> Self {
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
            comercio: EstadoComercio::default(),
            generaciones: Generaciones::nuevas(),
            cola: Cola::nueva(),
            por_lider: HashMap::new(),
            vistos: VecDeque::new(),
            vistos_version: HashMap::new(),
        }
    }

    pub fn ejercito_de_lider(&self, lider: &str) -> Option<u64> {
        self.por_lider.get(lider).copied()
    }

    pub(crate) fn insertar(&mut self, e: Ejercito) {
        self.por_lider.insert(e.lider.clone(), e.id);
        self.ejercitos.insert(e.id, e);
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
        w.i64(r.consumo_marcha);
        w.i64(r.consumo_estacionado);
        w.u32(r.viveres_bajos_pct);
        w.u64(r.limite_idempotencia as u64);
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
            w.u64(e.id);
            w.texto(&e.lider);
            w.texto(&e.propietario);
            w.u32(e.faccion);
            w.i64(e.velocidad);
            w.punto(e.posicion);
            w.u32(e.plan.generacion);
            w.u32(e.plan.vertices.len() as u32);
            for v in &e.plan.vertices {
                w.punto(v.p);
                w.i64(v.en);
            }
            let v = &e.viveres;
            w.i64(v.valor_en_t0);
            w.i64(v.t0);
            w.i64(v.por_minuto);
            w.opt_i64(v.maximo);
            w.opt_i64(v.minimo);
            w.i64(e.capacidad_viveres);
            w.bool(e.caravana_fundacion);
            w.u64(e.version);
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
                Carga::Comercio { suceso, aplazos } => {
                    w.u8(4);
                    w.u8(suceso.a_u8());
                    w.u8(*aplazos);
                }
                Carga::Reputacion(c) => {
                    w.u8(5);
                    w.u8(c.a_u8());
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
        match r.magia()? {
            m if m == *MAGIA => {}
            m if m == *MAGIA_ANTERIOR => return Err(ErrorFormato("foto del formato BAN4 (sin comercio): ya no se admite")),
            _ => return Err(ErrorFormato("no es un estado del núcleo")),
        }
        if r.texto()? != mundo_hash {
            return Err(ErrorFormato("la foto es de otro mundo (hash del artefacto distinto)"));
        }
        let guardadas = Reglas {
            consumo_marcha: r.i64()?,
            consumo_estacionado: r.i64()?,
            viveres_bajos_pct: r.u32()?,
            limite_idempotencia: r.u64()? as usize,
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
            let (id, lider, propietario, faccion, velocidad) = (r.u64()?, r.texto()?, r.texto()?, r.u32()?, r.i64()?);
            let posicion = r.punto()?;
            let generacion = r.u32()?;
            let mut vertices = Vec::new();
            for _ in 0..r.u32()? {
                vertices.push(Vertice { p: r.punto()?, en: r.i64()? });
            }
            let viveres = Ritmo {
                valor_en_t0: r.i64()?,
                t0: r.i64()?,
                por_minuto: r.i64()?,
                maximo: r.opt_i64()?,
                minimo: r.opt_i64()?,
            };
            let (capacidad_viveres, caravana_fundacion, version) = (r.i64()?, r.bool()?, r.u64()?);
            s.insertar(Ejercito {
                id,
                lider,
                propietario,
                faccion,
                velocidad,
                posicion,
                plan: Plan { generacion, vertices },
                viveres,
                capacidad_viveres,
                caravana_fundacion,
                version,
            });
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
            let urbe = Urbe::desde_ciudad(codec_ciudad::leer(&mut r)?);
            for e in &economia.edificios {
                if let Some(asentamiento::edificio::Fuente::Nodo(n)) = e.fuente {
                    s.nodos_reclamados.insert(n, id);
                }
            }
            s.asentamientos.insert(id, Asentamiento { id, faccion, posicion, fundado_en, radio, parcelas, version, economia, urbe });
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
                4 => Carga::Comercio { suceso: SucesoComercio::de_u8(r.u8()?).ok_or(ErrorFormato("suceso de comercio desconocido"))?, aplazos: r.u8()? },
                5 => Carga::Reputacion(CitaReputacion::de_u8(r.u8()?).ok_or(ErrorFormato("cita de reputación desconocida"))?),
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
