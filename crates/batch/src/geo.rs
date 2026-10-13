//! Lo que el batch necesita del artefacto de mundo, ya preparado: campamentos, piedra, metales, parcelas y anillos.

use base::Punto;
use mundo::{Mundo, OpcionesValidacion};
use rutas::Rutas;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Nombres de los anillos de tierra, por distancia al campamento de mercenarios más cercano (Doc 1.9c).
pub const ANILLOS: [&str; 3] = ["foco", "frontera", "lejana"];
const LIMITE_FOCO: i64 = 150_000;
const LIMITE_FRONTERA: i64 = 250_000;

/// El mundo cargado y su grafo de rutas.
pub struct Cargado {
    pub mundo: Arc<Mundo>,
    pub rutas: Arc<Rutas>,
}

/// Un nodo de estaño u oro.
pub struct Metal {
    /// 0 = estaño, 1 = oro.
    pub tipo: usize,
    pub anillo: usize,
    pub p: Punto,
}

pub struct Geo {
    pub campamentos: Vec<Punto>,
    pub piedra: Vec<Punto>,
    pub metales: Vec<Metal>,
    /// Centro de cada parcela y su masa de bosque.
    pub parcelas: Vec<(Punto, usize)>,
    pub total_por_masa: Vec<usize>,
    pub ancho: i64,
    pub alto: i64,
}

/// Punto del contrato (`Option<contrato::v1::Punto>`) a `base::Punto`, sin depender de ese crate.
macro_rules! pt {
    ($o:expr) => {
        $o.as_ref().map(|p| Punto::new(p.x, p.y)).unwrap_or_default()
    };
}

/// Ruta del único artefacto `mundo-*.bin` de una carpeta.
pub fn artefacto_en(dir: &Path) -> Result<PathBuf, String> {
    let mut hallados: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("no se puede leer {}: {e}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("mundo-") && n.to_string_lossy().ends_with(".bin")))
        .collect();
    hallados.sort();
    match hallados.len() {
        1 => Ok(hallados.remove(0)),
        0 => Err(format!("no hay ningún mundo-*.bin en {}", dir.display())),
        n => Err(format!("hay {n} artefactos en {}: indica uno con --mundo", dir.display())),
    }
}

/// Carpeta por defecto de los artefactos: la del repositorio (relativa a este crate) si no existe en el directorio actual.
pub fn carpeta_por_defecto() -> PathBuf {
    let local = PathBuf::from("generador/salida/mundo");
    if local.is_dir() {
        local
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generador/salida/mundo")
    }
}

pub fn cargar(ruta: &Path) -> Result<Cargado, String> {
    let bytes = std::fs::read(ruta).map_err(|e| format!("no se puede leer {}: {e}", ruta.display()))?;
    let mundo = Mundo::cargar(&bytes, &OpcionesValidacion::default()).map_err(|e| format!("artefacto no válido: {e:?}"))?;
    let rutas = Arc::new(Rutas::desde_mundo(&mundo));
    Ok(Cargado { mundo: Arc::new(mundo), rutas })
}

impl Geo {
    pub fn de(m: &Mundo) -> Geo {
        let campamentos: Vec<Punto> = m.campamentos().iter().map(|c| pt!(c.posicion)).collect();
        let masas: HashMap<&str, usize> = m.bosques().iter().enumerate().map(|(i, b)| (b.id.as_str(), i)).collect();
        let parcelas: Vec<(Punto, usize)> = m.parcelas().iter().map(|p| (pt!(p.centro), masas[p.bosque_id.as_str()])).collect();
        let mut total_por_masa = vec![0; masas.len()];
        parcelas.iter().for_each(|&(_, b)| total_por_masa[b] += 1);
        let piedra = m.nodos().iter().filter(|n| n.tipo == "piedra").map(|n| pt!(n.posicion)).collect();
        let metales = m
            .nodos()
            .iter()
            .filter_map(|n| {
                let tipo = ["estano", "oro"].iter().position(|t| *t == n.tipo)?;
                let p = pt!(n.posicion);
                let anillo = n.anillo.as_deref().and_then(|a| ANILLOS.iter().position(|x| *x == a)).unwrap_or_else(|| anillo_de(&campamentos, p).0);
                Some(Metal { tipo, anillo, p })
            })
            .collect();
        let (ancho, alto) = m.tamano();
        Geo { campamentos, piedra, metales, parcelas, total_por_masa, ancho, alto }
    }

    /// (anillo, índice del campamento más cercano) de un punto.
    pub fn anillo(&self, p: Punto) -> (usize, usize) {
        anillo_de(&self.campamentos, p)
    }
}

/// (anillo, campamento más cercano) de un punto.
pub fn anillo_de(campamentos: &[Punto], p: Punto) -> (usize, usize) {
    let (i, d) = campamentos.iter().enumerate().map(|(i, c)| (i, p.dist(*c))).min_by_key(|&(_, d)| d).unwrap_or((0, i64::MAX));
    let anillo = if d <= LIMITE_FOCO {
        0
    } else if d <= LIMITE_FRONTERA {
        1
    } else {
        2
    };
    (anillo, i)
}
