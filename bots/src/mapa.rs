//! El mapa público: el artefacto de mundo (contrato 06) que cualquier cliente baja de `Bienvenida.mundo.url_artefacto`. Se baja una vez por proceso y lo comparten todos los bots.
//! Rasters por filas, fila 0 al norte, celda de 4000 milésimas; `transitable` en bits, LSB primero (`crates/mundo/src/lib.rs` del motor).

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use contrato::v1::{ArtefactoDeMundo, ClaseTerreno, Punto, Raster};
use prost::Message as _;

use crate::{acceso, Fallo, Puerto};

fn capa(r: &Option<Raster>) -> Result<(Vec<u8>, &Raster), Fallo> {
    let r = r.as_ref().ok_or_else(|| Fallo::Red("artefacto sin capa".into()))?;
    let datos = if r.zstd { zstd::stream::decode_all(r.datos.as_slice()).map_err(|e| Fallo::Red(format!("capa: {e}")))? } else { r.datos.clone() };
    Ok((datos, r))
}

pub struct Mapa {
    columnas: usize,
    filas: usize,
    celda: i64,
    terreno: Vec<u8>,
    transitable: Vec<u8>,
    /// La densidad de bosque (0-255), con su propia rejilla; vacía en un artefacto sin la capa.
    bosque: Vec<u8>,
    bosque_columnas: usize,
    bosque_celda: i64,
    /// Los campamentos de mercenarios: no se funda cerca de ellos.
    pub campamentos: Vec<Punto>,
    /// Los centros de las parcelas de bosque (la madera del mantenimiento) y los yacimientos de piedra.
    parcelas: Vec<Punto>,
    piedras: Vec<Punto>,
    /// Los yacimientos (piedra, minerales, ganado): id, tipo y posición. Los pueblos extractores se marcan sobre ellos.
    pub nodos: Vec<(String, String, Punto)>,
    /// La fertilidad de cada cultivo de aldea (0-255), con su rejilla: trigo, vid, oliva, cebada y sésamo.
    fertilidad: Vec<(&'static str, Vec<u8>, usize, i64)>,
}

impl Mapa {
    pub fn desde_bytes(bytes: &[u8]) -> Result<Self, Fallo> {
        let a = ArtefactoDeMundo::decode(bytes).map_err(|e| Fallo::Red(format!("artefacto de mundo: {e}")))?;
        let (terreno, r) = capa(&a.terreno)?;
        let (transitable, _) = capa(&a.transitable)?;
        let (bosque, bosque_columnas, bosque_celda) = match capa(&a.densidad_bosque) {
            Ok((b, r)) => (b, r.columnas as usize, r.tamano_celda.max(1) as i64),
            Err(_) => (vec![], 0, 1),
        };
        let capas = [("trigo", &a.fertilidad), ("vid", &a.fertilidad_vid), ("oliva", &a.fertilidad_oliva), ("cebada", &a.fertilidad_cebada), ("sesamo", &a.fertilidad_sesamo)];
        let fertilidad = capas.into_iter().filter_map(|(c, r)| capa(r).ok().map(|(d, r)| (c, d, r.columnas as usize, r.tamano_celda.max(1) as i64))).collect();
        Ok(Self {
            fertilidad,
            bosque,
            bosque_columnas,
            bosque_celda,
            columnas: r.columnas as usize,
            filas: r.filas as usize,
            celda: r.tamano_celda.max(1) as i64,
            terreno,
            transitable,
            campamentos: a.campamentos.iter().filter_map(|c| c.posicion.clone()).collect(),
            parcelas: a.parcelas.iter().filter_map(|x| x.centro.clone()).collect(),
            piedras: a.nodos.iter().filter(|n| n.tipo == "piedra").filter_map(|n| n.posicion.clone()).collect(),
            // El contrato nombra cada yacimiento por su orden en el artefacto (`nodo-<i>`, `proyeccion::ids`), no por el id del generador.
            nodos: a.nodos.iter().enumerate().filter_map(|(i, n)| Some((format!("nodo-{i}"), n.tipo.clone(), n.posicion.clone()?))).collect(),
        })
    }

    /// La fertilidad de `cultivo` en `p`, de 0 a 255 (255 = 1,0, como el motor); 0 fuera del mapa o sin la capa.
    pub fn fertilidad(&self, cultivo: &str, p: &Punto) -> u8 {
        let Some((_, d, columnas, celda)) = self.fertilidad.iter().find(|(c, ..)| *c == cultivo) else { return 0 };
        if p.x < 0 || p.y < 0 || (p.x / celda) as usize >= *columnas {
            return 0;
        }
        d.get((p.y / celda) as usize * columnas + (p.x / celda) as usize).copied().unwrap_or(0)
    }

    fn indice(&self, p: &Punto) -> Option<usize> {
        if p.x < 0 || p.y < 0 {
            return None;
        }
        let (c, f) = ((p.x / self.celda) as usize, (p.y / self.celda) as usize);
        (c < self.columnas && f < self.filas).then_some(f * self.columnas + c)
    }

    pub fn terreno(&self, p: &Punto) -> Option<ClaseTerreno> {
        ClaseTerreno::try_from(*self.terreno.get(self.indice(p)?)? as i32).ok()
    }

    fn bosque(&self, p: &Punto) -> u8 {
        let (c, f) = ((p.x.max(0) / self.bosque_celda) as usize, (p.y.max(0) / self.bosque_celda) as usize);
        if c >= self.bosque_columnas {
            return 0;
        }
        self.bosque.get(f * self.bosque_columnas + c).copied().unwrap_or(0)
    }

    pub fn transitable(&self, p: &Punto) -> bool {
        self.indice(p).is_some_and(|i| self.transitable.get(i >> 3).is_some_and(|b| b >> (i & 7) & 1 == 1))
    }

    /// Se puede fundar aquí, según lo que se sabe: tierra firme que se pisa, a `exclusion` de los campamentos y a `separacion` de las `plazas` que conoce (Doc 1.2; `fundacion.*`).
    pub fn fundable(&self, p: &Punto, plazas: &[Punto], separacion: i64, exclusion: i64, bosque_denso: u8) -> bool {
        let lejos = |q: &Punto, d: i64| ((p.x - q.x) as f64).hypot((p.y - q.y) as f64) >= d as f64;
        matches!(self.terreno(p), Some(ClaseTerreno::TerrenoLlano | ClaseTerreno::TerrenoColina | ClaseTerreno::TerrenoCosta))
            && self.transitable(p)
            && self.bosque(p) < bosque_denso
            && self.campamentos.iter().all(|c| lejos(c, exclusion))
            && plazas.iter().all(|q| lejos(q, separacion))
    }
}

impl Mapa {
    /// El aviso de viabilidad del motor (`territorio::viabilidad` y la plaza «condenada» de `Economia_Construccion` §5), con lo que dice el mapa público: verde si nace con
    /// bosque y piedra en su zona (`radio_inicial`) y tiene bosque en la del techo del nivel 1 (`techo`), del que sale la madera del mantenimiento. ponytail: no sabe qué parcelas
    /// ya explota otra plaza (el aviso no viaja por el contrato); las que estén lejos de las plazas que conoce cuentan como libres.
    pub fn viable(&self, p: &Punto, radio_inicial: i64, techo: i64) -> bool {
        let cerca = |q: &Punto, r: i64| ((p.x - q.x) as f64).hypot((p.y - q.y) as f64) <= r as f64;
        self.parcelas.iter().any(|q| cerca(q, radio_inicial)) && self.piedras.iter().any(|q| cerca(q, radio_inicial)) && self.parcelas.iter().filter(|q| cerca(q, techo)).count() >= 2
    }
}

/// El mapa del mundo de `puerto`: se baja la primera vez y se guarda por hash.
pub async fn de(puerto: &Puerto) -> Result<Arc<Mapa>, Fallo> {
    static MAPAS: Mutex<Option<HashMap<String, Arc<Mapa>>>> = Mutex::new(None);
    let info = puerto.bienvenida.mundo.clone().unwrap_or_default();
    if let Some(m) = MAPAS.lock().unwrap().get_or_insert_with(HashMap::new).get(&info.hash) {
        return Ok(m.clone());
    }
    // ponytail: dos bots que lo piden a la vez lo bajan los dos; un candado por hash si pesa.
    let m = Arc::new(Mapa::desde_bytes(&acceso::get(puerto.servidor(), &info.url_artefacto).await?)?);
    MAPAS.lock().unwrap().get_or_insert_with(HashMap::new).insert(info.hash, m.clone());
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verde_con_bosque_y_piedra_en_la_zona() {
        let p = |x, y| Punto { x, y };
        let m = Mapa { columnas: 0, filas: 0, celda: 4000, terreno: vec![], transitable: vec![], bosque: vec![], bosque_columnas: 0, bosque_celda: 1, campamentos: vec![], fertilidad: vec![], parcelas: vec![p(3_000, 0), p(0, 9_000)], piedras: vec![p(0, 5_000)], nodos: vec![] };
        assert!(m.viable(&p(0, 0), 6_000, 10_000));
        assert!(!m.viable(&p(0, 0), 4_000, 10_000), "sin piedra en la zona con la que nace");
        assert!(!m.viable(&p(0, 0), 6_000, 8_000), "con un solo bosque al alcance del techo del nivel 1");
    }

    #[test]
    fn la_fertilidad_sale_de_la_capa_de_su_cultivo() {
        let p = |x, y| Punto { x, y };
        let fertilidad = vec![("trigo", vec![10, 200, 30, 40], 2, 4_000)];
        let m = Mapa { columnas: 0, filas: 0, celda: 4000, terreno: vec![], transitable: vec![], bosque: vec![], bosque_columnas: 0, bosque_celda: 1, campamentos: vec![], fertilidad, parcelas: vec![], piedras: vec![], nodos: vec![] };
        assert_eq!(m.fertilidad("trigo", &p(5_000, 0)), 200);
        assert_eq!(m.fertilidad("trigo", &p(0, 4_500)), 30);
        assert_eq!(m.fertilidad("vid", &p(5_000, 0)), 0, "sin su capa");
        assert_eq!(m.fertilidad("trigo", &p(9_000, 0)), 0, "fuera del mapa");
    }
}

#[cfg(test)]
mod volcado {
    /// `cargo test --lib volcar_nodos -- --ignored --nocapture`: los yacimientos del mundo de pruebas, para medir a qué distancia caen de las plazas.
    #[test]
    #[ignore]
    fn volcar_nodos() {
        let ruta = std::fs::read_dir("target/mundo-cec").unwrap().filter_map(|e| e.ok()).map(|e| e.path()).find(|p| p.file_name().unwrap().to_string_lossy().starts_with("mundo-")).unwrap();
        let m = super::Mapa::desde_bytes(&std::fs::read(ruta).unwrap()).unwrap();
        for (id, tipo, p) in &m.nodos {
            println!("NODO {id} {tipo} {} {}", p.x, p.y);
        }
    }
}
