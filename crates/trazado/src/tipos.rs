//! Tipos de entrada y salida del trazado. El catálogo (otro crate) decide el tipo, la categoría y la huella de cada
//! edificio y los pasa en un [`Pedido`]; aquí no hay tablas de edificios.

use crate::azar::sorteo;
use crate::geom::{Celda, Huella, Rect};
use base::Punto;

/// Despacho de colocación (T §6.1) y categoría de barrio (T §6.1 `CATEGORIA_POR_TIPO`) en un solo enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Categoria {
    /// Granja y corral: campo, fuera de la zona urbana, conectado por caminos (T §6.3).
    Afueras,
    /// Palacio, almacén, granero, leñera, taberna, Sala del Consejo: sin ancla ni barrio (T §6.4).
    Generico,
    Residencial,
    Industria,
    Militar,
    Mercado,
}

impl Categoria {
    /// Categorías con barrio: sus piezas se agrupan en torno a un ancla (T §6.5).
    pub fn tiene_barrio(self) -> bool {
        !matches!(self, Categoria::Afueras | Categoria::Generico)
    }
}

/// Papel de una pieza respecto a las anclas (T §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rol {
    Normal,
    /// El Centro Urbano: siembra el anillo de calle de la fundación; es el primer ancla (residencial).
    Centro,
    /// Ancla primaria de su categoría (el Mercado): nace por el árbol de anclas (T §7.2).
    AnclaPrimaria,
    /// Ancla de saturación (plaza, pozo, parque, plaza de armas, patio de gremios): marcador gratuito (T §7.1).
    AnclaSaturacion,
}

impl Rol {
    pub fn es_ancla(self) -> bool {
        self != Rol::Normal
    }
}

/// Perfil de trazado = permutación del orden de desempate de los satélites (T §8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Perfil {
    Nucleos,
    Caminera,
    Compacta,
    Gremial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Clave {
    Hueco,
    Nivel,
    Borde,
    BordeAfin,
    Centro,
}

impl Perfil {
    pub const TODOS: [Perfil; 4] = [Perfil::Nucleos, Perfil::Caminera, Perfil::Compacta, Perfil::Gremial];

    /// Perfil por tradición del asentamiento: `PERFILES_TRAZADO[floor(pa(semilla) × 4)]`.
    pub fn por_tradicion(semilla: u64) -> Perfil {
        Perfil::TODOS[sorteo(semilla, "perfil-trazado", &[], 4) as usize]
    }

    pub(crate) fn orden(self) -> [Clave; 5] {
        use Clave::*;
        match self {
            Perfil::Nucleos => [Hueco, Nivel, Borde, BordeAfin, Centro],
            Perfil::Caminera => [Nivel, Hueco, Borde, BordeAfin, Centro],
            Perfil::Compacta => [Centro, Hueco, Nivel, Borde, BordeAfin],
            Perfil::Gremial => [BordeAfin, Hueco, Nivel, Borde, Centro],
        }
    }
}

/// Revisión de la geometría. Se sube al cambiar cualquier huella, el tamaño de celda, el fondo de manzana, una constante de
/// [`Parametros`] que mueva lo ya colocado o el algoritmo de determinismo. Un `Ciudad` guardado con otra revisión debe
/// rechazarse al cargar. **3**: `pseudoAleatorio` con `sin` sustituido por hash entero (FNV-1a + SplitMix64) y tabla de
/// senos entera para las ranuras. **4** (2026-10-23, como el TypeScript): sin disco urbano para anclas ni satélites, el círculo de
/// barrido del núcleo de un ancla, el ancla llena en cuanto no cabe la huella pedida, la Granja que crece busca sitio conservando
/// su camino y el ancla que nace al proponer. **5** (2026-10-24, Murallas M1): una puerta por cruce no parte la red y la red de las puertas se siembra
/// conectada. **6** (2026-10-24, Cal-6): ningún anillo tiene más de 4 puertas; los cruces cortados rodean el muro hasta la puerta más cercana.
pub const LAYOUT_VERSION: u32 = 6;

/// Constantes de T §2.1 y §12 más las dos locales (`FONDO_MANZANA`, `NIVEL_GATE_ANCLAS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parametros {
    pub largo_fila_min: i32,
    pub largo_fila_max: i32,
    /// En unidades locales (no celdas).
    pub radio_afueras_min_ul: i32,
    /// En unidades locales (no celdas).
    pub ancho_banda_afueras_ul: i32,
    pub separacion_minima_anclas: i32,
    pub separacion_seguridad_anclas: i32,
    pub ancho_calle: i32,
    pub cap_corredor_urbano: u16,
    pub cap_corredor_afueras: u16,
    pub fondo_manzana: i32,
    pub nivel_gate_anclas: u8,
    /// `radioCiudad` inicial, en unidades locales.
    pub radio_inicial_ul: i32,
    /// Crecimiento del `radioCiudad` por edificio completado.
    pub crecimiento_por_edificio_ul: i32,
    /// Tope del `radioCiudad` por nivel de asentamiento 1..=5.
    pub maximo_por_nivel_ul: [i32; 5],
    /// Viviendas que trae la fundación (T §11).
    pub viviendas_iniciales: u32,
}

impl Default for Parametros {
    fn default() -> Self {
        Self {
            largo_fila_min: 4,
            largo_fila_max: 8,
            radio_afueras_min_ul: 60,
            ancho_banda_afueras_ul: 36,
            separacion_minima_anclas: 6,
            separacion_seguridad_anclas: 3,
            ancho_calle: 1,
            cap_corredor_urbano: 12,
            cap_corredor_afueras: 200,
            fondo_manzana: 2,
            nivel_gate_anclas: 2,
            radio_inicial_ul: 30,
            crecimiento_por_edificio_ul: 5,
            maximo_por_nivel_ul: [60, 90, 120, 150, 180],
            viviendas_iniciales: 3,
        }
    }
}

/// `radioCiudad` en unidades locales: nace en `radio_inicial_ul`, crece por edificio completado y topa por nivel (T §2.1).
pub fn radio_ciudad(p: &Parametros, edificios_completados: u32, nivel: u8) -> i32 {
    let tope = p.maximo_por_nivel_ul[(nivel.clamp(1, 5) - 1) as usize];
    (p.radio_inicial_ul + p.crecimiento_por_edificio_ul * edificios_completados as i32).min(tope)
}

/// Un edificio colocado. El orden del array de piezas es el de construcción y es significativo (T §2.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pieza {
    pub id: u32,
    /// Código de tipo del catálogo; opaco para el trazado, salvo que dos piezas del mismo `tipo` son afines.
    pub tipo: u16,
    pub categoria: Categoria,
    pub rol: Rol,
    /// Rectángulo ocupado, ya girado si procede.
    pub rect: Rect,
    pub rotado: bool,
    /// (anclas) su núcleo ya no admite más satélites; latch permanente.
    pub ancla_llena: bool,
    /// (anclas) sus 5 ranuras de crecimiento se agotaron; latch permanente.
    pub semilla_saturada: bool,
}

impl Pieza {
    /// Posición = centro del rectángulo, en milésimas de unidad local.
    pub fn posicion(&self) -> Punto {
        self.rect.centro()
    }
}

/// Un recinto de muralla visto desde el trazado (T §4 punto 3, §6.6; Murallas.md). El trazado no lo construye: el llamador
/// pasa las celdas ya trazadas, levantadas o no (basta con que el recinto esté comprometido).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recinto {
    /// Celdas de muro y torre: ocupan suelo siempre.
    pub muros: Vec<Celda>,
    /// Celdas de puerta: transitables, no ocupan suelo.
    pub puertas: Vec<Celda>,
    /// La red que el recinto conserva en sus puertas (`LAYOUT_VERSION` 5): cada puerta y su tramo de calle o camino a cada lado, tal como era al
    /// trazarlo. Se siembra conectada a la red al rehacerla, en el punto en que se trazó (T §5.1, Murallas.md §3).
    pub red_de_puertas: Vec<CeldaDeRed>,
    /// La pieza de mayor id que había al trazarlo: la red de las puertas entra tras ella al rehacer la red.
    pub tras_pieza: u32,
    /// `integridad ≥ 1`: solo un recinto completo activa la preferencia intramuros (T §6.6).
    pub completo: bool,
}

/// Una celda de la red de calles: calle (urbana) o camino (rural).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CeldaDeRed {
    pub celda: Celda,
    pub camino: bool,
}

/// Qué se quiere colocar. `radio_ul` es el `radioCiudad` en unidades locales (T §2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pedido {
    pub id: u32,
    pub tipo: u16,
    pub categoria: Categoria,
    pub rol: Rol,
    /// Huella sin girar.
    pub huella: Huella,
    /// Si la huella no es cuadrada, barrer también la girada (nunca la granja; sí los puestos de mercado).
    pub permitir_rotacion: bool,
    pub perfil: Perfil,
    pub radio_ul: i32,
}

/// Dónde cae una pieza.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colocacion {
    pub rect: Rect,
    pub rotado: bool,
}

impl Colocacion {
    pub fn posicion(&self) -> Punto {
        self.rect.centro()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorTrazado {
    IdRepetido(u32),
    /// El rectángulo (o su margen de una celda) sale de la ventana de ±[`crate::MITAD`] celdas.
    FueraDeVentana,
    /// Pisa un edificio, una calle, un camino o un muro.
    Ocupada(Celda),
    /// `desde_partes` con un `layout_version` distinto del vigente.
    VersionDeGeometria(u32),
}

/// Un tipo de ancla de saturación que el llamador ofrece al sorteo de [`crate::Ciudad::asegurar_ancla_para`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpcionAncla {
    pub tipo: u16,
    pub huella: Huella,
}

/// Resultado de garantizar un ancla con hueco para una categoría (T §7.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ancla {
    /// Ya hay una con sitio; es el id de la pieza.
    Existente(u32),
    /// Se creó una de saturación; es el id de la pieza nueva (ya colocada).
    Creada(u32),
    /// No hay ancla con sitio y no se pudo (o no toca) crear otra: reintentar en la siguiente reevaluación.
    Ninguna,
}
