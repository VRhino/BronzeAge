//! Los tipos de los contratos de mercenarios (diseño §4 a §9): el contrato y su orden, la ficha de progresión, las ofertas del tablón y lo que el núcleo averigua del
//! mundo para validar una firma u orden ([`Situacion`]). Los guarda el núcleo tal cual en [`EstadoContratos`].

use base::{Instante, Punto};
use std::collections::BTreeMap;

/// Los trabajos de la tabla `TRABAJOS_MERCENARIOS` (§4). Un id de la tabla que no es uno de estos hace fallar la carga.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Trabajo {
    Defender,
    Escoltar,
    Atacar,
    Acompanar,
    Permanente,
}

impl Trabajo {
    pub const TODOS: [Trabajo; 5] = [Trabajo::Defender, Trabajo::Escoltar, Trabajo::Atacar, Trabajo::Acompanar, Trabajo::Permanente];

    /// El id de la tabla de datos.
    pub fn id(self) -> &'static str {
        match self {
            Trabajo::Defender => "defender",
            Trabajo::Escoltar => "escoltar",
            Trabajo::Atacar => "atacar",
            Trabajo::Acompanar => "acompanar",
            Trabajo::Permanente => "permanente",
        }
    }
}

/// A qué apunta un contrato o una orden. `Caravana` cubre también el carro de acarreo de un pueblo, un encargo y una ruta: en el motor todo eso es una caravana (decisión 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Objetivo {
    Plaza(u64),
    Pueblo(u64),
    Avanzada(u64),
    Caravana(u64),
    /// Un campamento de bandidos.
    Bandidos(u64),
    Ejercito(u64),
    Convocatoria(u64),
    Faccion(u32),
    /// Solo para `ir_a`.
    Punto(Punto),
}

/// La clase de un [`Objetivo`], como la nombra la tabla de trabajos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaseObjetivo {
    Plaza,
    Pueblo,
    Avanzada,
    Caravana,
    Bandidos,
    Ejercito,
    Convocatoria,
    Faccion,
    Punto,
}

impl ClaseObjetivo {
    /// El nombre de la tabla de datos.
    pub fn desde_id(s: &str) -> Option<Self> {
        Some(match s {
            "plaza" => ClaseObjetivo::Plaza,
            "pueblo" => ClaseObjetivo::Pueblo,
            "avanzada" => ClaseObjetivo::Avanzada,
            "caravana" => ClaseObjetivo::Caravana,
            "bandidos" => ClaseObjetivo::Bandidos,
            "ejercito" => ClaseObjetivo::Ejercito,
            "convocatoria" => ClaseObjetivo::Convocatoria,
            "faccion" => ClaseObjetivo::Faccion,
            "punto" => ClaseObjetivo::Punto,
            _ => return None,
        })
    }
}

impl Objetivo {
    pub fn clase(&self) -> ClaseObjetivo {
        match self {
            Objetivo::Plaza(_) => ClaseObjetivo::Plaza,
            Objetivo::Pueblo(_) => ClaseObjetivo::Pueblo,
            Objetivo::Avanzada(_) => ClaseObjetivo::Avanzada,
            Objetivo::Caravana(_) => ClaseObjetivo::Caravana,
            Objetivo::Bandidos(_) => ClaseObjetivo::Bandidos,
            Objetivo::Ejercito(_) => ClaseObjetivo::Ejercito,
            Objetivo::Convocatoria(_) => ClaseObjetivo::Convocatoria,
            Objetivo::Faccion(_) => ClaseObjetivo::Faccion,
            Objetivo::Punto(_) => ClaseObjetivo::Punto,
        }
    }
}

/// Las cuatro órdenes de mando (§6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoOrden {
    Guardar,
    IrA,
    Atacar,
    Unirse,
}

impl TipoOrden {
    /// El nombre de la tabla de datos (la orden inicial de cada trabajo).
    pub fn desde_id(s: &str) -> Option<Self> {
        Some(match s {
            "guardar" => TipoOrden::Guardar,
            "ir_a" => TipoOrden::IrA,
            "atacar" => TipoOrden::Atacar,
            "unirse" => TipoOrden::Unirse,
            _ => return None,
        })
    }
}

/// El cargo con el que se firma o se ordena (§3, §6.5). Las variantes locales llevan su plaza. Quien firma u ordena no lo elige: el motor toma el de más rango que lo permita (decisión 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Puesto {
    Rey,
    Embajador,
    General(u64),
    Gobernador(u64),
    MaestroCaravanas(u64),
}

impl Puesto {
    /// El rango entre órdenes (§6.5, decisión B): Rey 4, Embajador 3, General y Gobernador 2, Maestro de Caravanas 1.
    pub fn rango(self) -> u8 {
        match self {
            Puesto::Rey => 4,
            Puesto::Embajador => 3,
            Puesto::General(_) | Puesto::Gobernador(_) => 2,
            Puesto::MaestroCaravanas(_) => 1,
        }
    }
}

/// Una orden de mando con su ciclo (§6.3): quién la dio, con qué puesto, cuándo y hasta cuándo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Orden {
    pub tipo: TipoOrden,
    pub objetivo: Objetivo,
    pub dada_por: u64,
    pub puesto: Puesto,
    pub dada_en: Instante,
    /// `ir_a` y `atacar`: caducan al llegar este instante (`plazoOrdenHoras`).
    pub vence: Option<Instante>,
    /// `atacar`: la Facción dueña del objetivo al darla; con la paz caduca (decisión 6).
    pub contra: Option<u32>,
    /// `unirse`: ya estuvo dentro de ese ejército o convocatoria; si sale, caduca.
    pub unido: bool,
}

/// De quién es un contrato (§5): de la Facción si lo firmó el Rey o el Embajador; de la plaza si lo firmó un cargo local de ella.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ambito {
    Faccion,
    Plaza(u64),
}

/// Por qué acabó un contrato (§7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fin {
    Plazo,
    Despido,
    /// La plaza que lo firmó cayó o cambió de dueño.
    PlazaPerdida,
    /// No queda en la Facción ningún ciudadano sin marca.
    SinCiudadanos,
    /// El mercenario ya no es ciudadano de esa Facción, o la Facción ya no existe.
    FaccionPerdida,
}

/// Un contrato vigente. Hay uno como mucho por mercenario y su clave es el id del héroe (decisión 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contrato {
    pub mercenario: u64,
    pub faccion: u32,
    pub ambito: Ambito,
    pub trabajo: Trabajo,
    pub objetivo: Objetivo,
    /// Días de mundo; `None` en el permanente.
    pub plazo: Option<u8>,
    pub firmado_por: u64,
    pub puesto_firma: Puesto,
    pub firmado_en: Instante,
    /// Fin del plazo del temporal (se alarga al prorrogar); `None` en el permanente.
    pub vence: Option<Instante>,
    /// El campamento donde se firmó: adonde vuelve el permanente al acabar.
    pub campamento_base: u32,
    /// La orden vigente; `None`: espera donde está hasta otra orden (§6.3).
    pub orden: Option<Orden>,
    /// El fin que espera a que acabe su batalla (§7, decisión C).
    pub fin_aplazado: Option<Fin>,
    /// Renombre ganado con este contrato y aún sin apuntar (el del temporal se apunta al acabar).
    pub renombre: u32,
}

/// Las medallas (§9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Medalla {
    PalabraCumplida,
    CazadorDeBandidos,
    Muralla,
    BuenCamino,
    Asaltante,
    Campana,
    BienValorado,
    Leal,
}

impl Medalla {
    pub const TODAS: [Medalla; 8] =
        [Medalla::PalabraCumplida, Medalla::CazadorDeBandidos, Medalla::Muralla, Medalla::BuenCamino, Medalla::Asaltante, Medalla::Campana, Medalla::BienValorado, Medalla::Leal];

    /// El id de la tabla de datos.
    pub fn id(self) -> &'static str {
        match self {
            Medalla::PalabraCumplida => "palabra_cumplida",
            Medalla::CazadorDeBandidos => "cazador_de_bandidos",
            Medalla::Muralla => "muralla",
            Medalla::BuenCamino => "buen_camino",
            Medalla::Asaltante => "asaltante",
            Medalla::Campana => "campana",
            Medalla::BienValorado => "bien_valorado",
            Medalla::Leal => "leal",
        }
    }
}

/// Lo que se cuenta de los contratos de un mercenario, sin nombres (§10).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Historial {
    pub firmados: u32,
    pub terminados: u32,
    pub despidos: u32,
    pub buenas: u32,
    pub malas: u32,
}

/// La progresión de un mercenario (§8, §9). Nace la primera vez que se ofrece y no se borra nunca.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ficha {
    pub renombre: u32,
    /// El rango más alto alcanzado (1 a 5); no baja aunque baje el renombre.
    pub rango: u8,
    /// Escuadras de premio sin reclamar.
    pub premios: u8,
    pub contadores: BTreeMap<Medalla, u32>,
    /// 1 bronce, 2 plata, 3 oro; nunca baja.
    pub grados: BTreeMap<Medalla, u8>,
    pub historial: Historial,
    /// Ya recibió la leva propia del rango 1.
    pub leva_recibida: bool,
}

impl Ficha {
    /// La ficha con la que nace: rango 1, sin renombre.
    pub fn nueva() -> Self {
        Ficha { rango: 1, ..Ficha::default() }
    }
}

/// Un bot ofrecido en el tablón de un campamento (§3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Oferta {
    pub campamento: u32,
    pub desde: Instante,
}

/// Un contrato acabado que aún se puede valorar (§8.2, duda 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Terminado {
    pub firmado_por: u64,
    /// Lo que se apuntó al acabar (0 con despido).
    pub renombre: u32,
    pub valorable_hasta: Instante,
    pub valorado: bool,
}

/// Lo que el núcleo averigua del objetivo de una firma u orden para que las reglas puras decidan (el crate no ve el mundo).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Situacion {
    /// La Facción dueña; `None` en bandidos y puntos.
    pub duena: Option<u32>,
    /// La plaza de la que cuelga: la plaza misma, la madre de un pueblo o una avanzada, el origen de una caravana.
    pub plaza: Option<u64>,
    /// La Facción de quien firma u ordena está en guerra con la dueña.
    pub en_guerra: bool,
    /// Bandidos: las plazas en cuya zona están.
    pub en_zona_de: Vec<u64>,
    /// Convocatoria o ejército: quien la convocó o lo lidera.
    pub cabeza: Option<u64>,
}

/// Todo el estado de los contratos que guarda el núcleo. `esperas` y `terminados` se podan de lo vencido al insertar, no al leer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EstadoContratos {
    /// Por héroe.
    pub ofertas: BTreeMap<u64, Oferta>,
    /// Por mercenario.
    pub contratos: BTreeMap<u64, Contrato>,
    /// Por héroe; nace al primer ofrecimiento.
    pub fichas: BTreeMap<u64, Ficha>,
    /// (mercenario, Facción) → hasta cuándo no se le puede recontratar tras un despido.
    pub esperas: BTreeMap<(u64, u32), Instante>,
    /// (mercenario, Facción) → el último contrato acabado, mientras se puede valorar.
    pub terminados: BTreeMap<(u64, u32), Terminado>,
}
