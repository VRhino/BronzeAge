//! Cargador y validador del artefacto de mundo (`ArtefactoDeMundo`, `Docs/Coordinacion/06_Contrato_de_Mundo.md`).
//!
//! `Mundo::cargar(bytes, &opciones)` decodifica, valida TODAS las reglas de 06 §3 y devuelve un mundo de solo lectura;
//! un artefacto que incumple una sola regla no se carga (`ErrorMundo`, una variante por regla).
//!
//! # Convenciones fijas del escritor (generador TypeScript)
//!
//! - **Unidades**: milésimas de unidad estratégica (`sint64`). Mundo real: 2.667.000 × 2.000.000.
//! - **Rasters**: `tamano_celda` = 4000; orden por filas, fila 0 al norte; se muestrea el centro de la celda
//!   (la celda de un punto `(x, y)` es `(x / 4000, y / 4000)`). Columnas = ⌈ancho / 4000⌉, filas = ⌈alto / 4000⌉.
//!   `altura` es `u16` LE; `terreno` es `u8` = `ClaseTerreno`; `fertilidad`, `humedad` y `densidad_bosque` son `u8`;
//!   `anillos` es `u8` = `Anillo`; `transitable` va en bits (celda `i` → byte `i >> 3`, bit `i & 7`, LSB primero).
//!   `coste` (opcional) es `u16` LE: coste de movimiento en milésimas (1000 = llano), 0 justo en las celdas no transitables.
//!   Todos con `zstd = true`.
//! - **Hash**: `manifiesto.hash` = sha256 hex minúsculas de la serialización prost del `ArtefactoDeMundo` con
//!   `manifiesto.hash = ""`. Cada `CapaDeMundo.sha256` = sha256 hex de los `datos` del raster tal como se guardan
//!   (comprimidos). `CapaDeMundo.nombre` ∈ {altura, terreno, fertilidad, humedad, densidad_bosque, anillos, transitable, coste};
//!   se ignoran otras capas declaradas y el campo `bytes`.
//!
//! # Reglas que se comprueban (en este orden)
//!
//! hash del contenido; tamaño (si `tamano_esperado`); sha256 y dimensiones/encoding/longitud de cada capa; coste mayor que
//! 0 justo en las celdas transitables (si trae la capa); ids únicos
//! por colección; aristas del grafo (nodos existentes, origen y destino transitables); nodos de recurso, campamentos
//! (mercenarios y bandidos) y alijos en tierra transitable y fuera del agua; cuotas por anillo de estaño y oro
//! (5/25/70 % por resto mayor sobre el total presente, según el `anillo` declarado de cada nodo); parcelas con bosque
//! existente y separadas ≥ 2·rp; campamentos de mercenarios separados ≥ 600 estratégicas.
//!
//! El paquete de relieve de los clientes 3D (`PaqueteRelieve`) va aparte: [`validar_relieve`] lo comprueba contra el artefacto.

mod error;
mod relieve;
mod validar;

pub use error::ErrorMundo;
pub use relieve::{validar_relieve, ErrorRelieve};

use contrato::v1::*;
use prost::Message;
use sha2::{Digest, Sha256};

/// Milésimas por unidad estratégica.
pub const MILESIMAS: i64 = 1000;
/// Lado de celda de todos los rasters, en milésimas.
pub const TAMANO_CELDA: u32 = 4000;

/// Parámetros de la validación. `Default` = mundo real y cifras del canon (Doc 1.9c).
#[derive(Debug, Clone)]
pub struct OpcionesValidacion {
    /// Tamaño que debe declarar el manifiesto, en milésimas; `None` no lo exige (mundos pequeños de prueba).
    pub tamano_esperado: Option<(i64, i64)>,
    /// Radio de parcela `rp` en estratégicas: las parcelas se separan ≥ 2·rp.
    pub rp: i64,
    /// Separación mínima entre campamentos de mercenarios, en estratégicas.
    pub separacion_campamentos: i64,
}

impl Default for OpcionesValidacion {
    fn default() -> Self {
        Self { tamano_esperado: Some((2_667_000, 2_000_000)), rp: 4, separacion_campamentos: 600 }
    }
}

/// Mundo validado e inmutable.
#[derive(Debug)]
pub struct Mundo {
    artefacto: ArtefactoDeMundo,
    columnas: usize,
    filas: usize,
    altura: Vec<u16>,
    terreno: Vec<u8>,
    fertilidad: Vec<u8>,
    humedad: Vec<u8>,
    densidad_bosque: Vec<u8>,
    anillos: Vec<u8>,
    transitable: Vec<u8>, // bits
    coste: Vec<u16>,      // vacío si el artefacto no trae la capa
}

/// sha256 en hex minúsculas.
pub fn sha256_hex(datos: &[u8]) -> String {
    Sha256::digest(datos).iter().map(|b| format!("{b:02x}")).collect()
}

impl Mundo {
    /// Decodifica y valida el artefacto; `Err` si incumple cualquier regla.
    pub fn cargar(bytes: &[u8], opciones: &OpcionesValidacion) -> Result<Mundo, ErrorMundo> {
        let artefacto = ArtefactoDeMundo::decode(bytes).map_err(|e| ErrorMundo::Decodificacion(e.to_string()))?;
        validar::validar(artefacto, opciones)
    }

    // ── Consultas por punto (milésimas). Fuera del mundo: `false` / `None`. ──

    fn celda(&self, x: i64, y: i64) -> Option<usize> {
        if x < 0 || y < 0 {
            return None;
        }
        let (c, f) = ((x / TAMANO_CELDA as i64) as usize, (y / TAMANO_CELDA as i64) as usize);
        (c < self.columnas && f < self.filas).then_some(f * self.columnas + c)
    }

    /// Autoridad única de transitabilidad.
    pub fn transitable_en(&self, x: i64, y: i64) -> bool {
        self.celda(x, y).is_some_and(|i| self.transitable[i >> 3] >> (i & 7) & 1 == 1)
    }

    pub fn anillo_en(&self, x: i64, y: i64) -> Option<Anillo> {
        Anillo::try_from(self.anillos[self.celda(x, y)?] as i32).ok()
    }

    pub fn terreno_en(&self, x: i64, y: i64) -> Option<ClaseTerreno> {
        ClaseTerreno::try_from(self.terreno[self.celda(x, y)?] as i32).ok()
    }

    pub fn altura_en(&self, x: i64, y: i64) -> Option<u16> {
        self.celda(x, y).map(|i| self.altura[i])
    }

    pub fn fertilidad_en(&self, x: i64, y: i64) -> Option<u8> {
        self.celda(x, y).map(|i| self.fertilidad[i])
    }

    pub fn humedad_en(&self, x: i64, y: i64) -> Option<u8> {
        self.celda(x, y).map(|i| self.humedad[i])
    }

    pub fn densidad_bosque_en(&self, x: i64, y: i64) -> Option<u8> {
        self.celda(x, y).map(|i| self.densidad_bosque[i])
    }

    /// Coste de movimiento en la celda, en milésimas (1000 = llano; 0 = no transitable). `None` fuera del mundo o si el
    /// artefacto no trae la capa `coste`.
    pub fn coste_en(&self, x: i64, y: i64) -> Option<u16> {
        if self.coste.is_empty() {
            return None;
        }
        self.celda(x, y).map(|i| self.coste[i])
    }

    /// ¿Trae el artefacto la capa `coste`? Sin ella las rutas no se suavizan.
    pub fn tiene_coste(&self) -> bool {
        !self.coste.is_empty()
    }

    // ── Acceso de solo lectura ──

    /// Hash del artefacto (identifica el mundo).
    pub fn hash(&self) -> &str {
        &self.artefacto.manifiesto.as_ref().expect("validado").hash
    }
    pub fn manifiesto(&self) -> &Manifiesto {
        self.artefacto.manifiesto.as_ref().expect("validado")
    }
    /// Tamaño del mundo en milésimas.
    pub fn tamano(&self) -> (i64, i64) {
        let t = self.manifiesto().tamano.expect("validado");
        (t.x, t.y)
    }
    /// Rejilla de los rasters: (columnas, filas).
    pub fn rejilla(&self) -> (usize, usize) {
        (self.columnas, self.filas)
    }
    pub fn artefacto(&self) -> &ArtefactoDeMundo {
        &self.artefacto
    }
    pub fn rios(&self) -> &[Rio] {
        &self.artefacto.rios
    }
    pub fn bosques(&self) -> &[ZonaBosque] {
        &self.artefacto.bosques
    }
    pub fn parcelas(&self) -> &[ParcelaBosque] {
        &self.artefacto.parcelas
    }
    pub fn nodos(&self) -> &[NodoRecurso] {
        &self.artefacto.nodos
    }
    pub fn vetas(&self) -> &[Veta] {
        &self.artefacto.vetas
    }
    pub fn campamentos(&self) -> &[CampamentoMercenarios] {
        &self.artefacto.campamentos
    }
    pub fn bandidos_iniciales(&self) -> &[CampamentoBandidos] {
        &self.artefacto.bandidos_iniciales
    }
    pub fn alijos(&self) -> &[Alijo] {
        &self.artefacto.alijos
    }
    pub fn navegacion(&self) -> &GrafoNavegacion {
        self.artefacto.navegacion.as_ref().expect("validado")
    }
}
