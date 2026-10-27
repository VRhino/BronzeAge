//! El archivo común de datos de juego (fase 7, a petición del frente de bots): las cifras de reglas que ve un jugador, para que los clientes no las copien a mano.
//!
//! Es un JSON con:
//! - `documentos`: cada bloque `<!-- parametros:NOMBRE -->` de los documentos de `Docs/Mechanics`, ya parseado, por documento y por nombre. Son la fuente de verdad de las cifras: el
//!   núcleo los incrusta y los lee igual.
//! - `reglas`: las pocas reglas de la partida que no salen de un bloque (`Reglas` del núcleo: territorio, aviso de víveres, idempotencia, batallas en Unity), en milésimas de unidad
//!   como el contrato.
//! - `derivados`: cifras que el núcleo calcula de varios bloques y que un cliente tendría que rehacer: el coste de la Caravana de Fundación y el de la refundación desde un campamento
//!   (`Fundacion_Y_Refundacion` §3), por recurso en milésimas.
//! - `huella_reglas`: la identidad de las reglas que usa el núcleo (`ReglasEconomia::huella`, la misma que va en las fotos).
//!
//! Su versión es el SHA-256 del JSON: va en `InfoMundo.version_datos_de_juego` y en la ruta (`/v1/datos/<versión>`), así que un cliente lo guarda por versión como el artefacto de mundo.

use asentamiento::Recurso;
use nucleo::Reglas;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Los documentos de Mechanics con bloques de parámetros (todos salvo el README, cuyo bloque es la versión del balance del TypeScript).
const DOCUMENTOS: &[(&str, &str)] = &[
    ("Batalla_Unity_Ciclo", include_str!("../../../Docs/Mechanics/Batalla_Unity_Ciclo.md")),
    ("Campamentos_Mercenarios", include_str!("../../../Docs/Mechanics/Campamentos_Mercenarios.md")),
    ("Combate", include_str!("../../../Docs/Mechanics/Combate.md")),
    ("Comercio_Caravanas", include_str!("../../../Docs/Mechanics/Comercio_Caravanas.md")),
    ("Diplomacia_Facciones", include_str!("../../../Docs/Mechanics/Diplomacia_Facciones.md")),
    ("Economia_Construccion", include_str!("../../../Docs/Mechanics/Economia_Construccion.md")),
    ("Fundacion_Y_Refundacion", include_str!("../../../Docs/Mechanics/Fundacion_Y_Refundacion.md")),
    ("Generacion_Del_Mundo", include_str!("../../../Docs/Mechanics/Generacion_Del_Mundo.md")),
    ("Heroe_Y_Tropa", include_str!("../../../Docs/Mechanics/Heroe_Y_Tropa.md")),
    ("Misiones", include_str!("../../../Docs/Mechanics/Misiones.md")),
    ("Misiones_Textos", include_str!("../../../Docs/Mechanics/Misiones_Textos.md")),
    ("Movimiento_Ejercitos", include_str!("../../../Docs/Mechanics/Movimiento_Ejercitos.md")),
    ("Murallas", include_str!("../../../Docs/Mechanics/Murallas.md")),
    ("Niebla_De_Guerra", include_str!("../../../Docs/Mechanics/Niebla_De_Guerra.md")),
    ("Presencia_Y_Convocatoria", include_str!("../../../Docs/Mechanics/Presencia_Y_Convocatoria.md")),
    ("Proyecciones_Y_Autorizacion", include_str!("../../../Docs/Mechanics/Proyecciones_Y_Autorizacion.md")),
    ("Reloj_Y_Eventos", include_str!("../../../Docs/Mechanics/Reloj_Y_Eventos.md")),
    ("Tecnologia_Aedas", include_str!("../../../Docs/Mechanics/Tecnologia_Aedas.md")),
    ("Trazado_Urbano", include_str!("../../../Docs/Mechanics/Trazado_Urbano.md")),
];

/// Los bloques de un documento, por nombre. Un bloque que no es JSON es un error de programación (los crates de reglas fallarían igual).
pub fn bloques(nombre_doc: &str, doc: &str) -> Map<String, Value> {
    let mut fuera = Map::new();
    let mut resto = doc;
    while let Some(i) = resto.find("<!-- parametros:") {
        let desde = &resto[i + "<!-- parametros:".len()..];
        let (nombre, tras) = desde.split_once("-->").unwrap_or_else(|| panic!("{nombre_doc}: marca de bloque sin cerrar"));
        let fin = tras.find("<!-- /parametros -->").unwrap_or_else(|| panic!("{nombre_doc}: el bloque {nombre} no se cierra"));
        let cuerpo = tras[..fin].trim();
        let cuerpo = cuerpo.strip_prefix("```json").and_then(|c| c.strip_suffix("```")).unwrap_or_else(|| panic!("{nombre_doc}: el bloque {nombre} no es json"));
        let v: Value = serde_json::from_str(cuerpo).unwrap_or_else(|e| panic!("{nombre_doc}: bloque {nombre}: {e}"));
        fuera.insert(nombre.trim().to_owned(), v);
        resto = &tras[fin..];
    }
    fuera
}

/// El archivo y su versión.
#[derive(Clone, Debug)]
pub struct DatosDeJuego {
    pub version: String,
    pub json: Vec<u8>,
}

/// Una carga como `{ <recurso>: <milésimas> }`, con los nombres de recurso del contrato.
fn por_recurso(c: &BTreeMap<Recurso, i64>) -> Value {
    Value::Object(c.iter().map(|(r, &n)| (r.nombre().to_owned(), json!(n))).collect())
}

/// Los datos de juego de una partida con estas reglas.
pub fn datos_de_juego(r: &Reglas) -> DatosDeJuego {
    let documentos: Map<String, Value> = DOCUMENTOS.iter().map(|(n, d)| ((*n).to_owned(), Value::Object(bloques(n, d)))).collect();
    let t = &r.territorio;
    let v = json!({
        "huella_reglas": r.economia.huella(),
        "reglas": {
            "territorio": {
                "radio_inicial": t.radio_inicial,
                "techos": t.techos,
                "crecimiento_por_edificio": t.crecimiento_por_edificio,
                "separacion_plazas": t.separacion_plazas,
                "exclusion_campamento": t.exclusion_campamento,
                "cuota_parcelas_pct": t.cuota_parcelas_pct,
            },
            "viveres_bajos_pct": r.viveres_bajos_pct,
            "limite_idempotencia": r.limite_idempotencia,
            "batallas_en_unity": r.batallas_en_unity,
        },
        "derivados": {
            "costo_caravana_fundacion": por_recurso(&r.costo_caravana_fundacion()),
            "costo_refundacion": por_recurso(&r.costo_refundacion()),
        },
        "documentos": documentos,
    });
    let json = serde_json::to_vec(&v).expect("JSON");
    let version = Sha256::digest(&json).iter().map(|b| format!("{b:02x}")).collect();
    DatosDeJuego { version, json }
}
