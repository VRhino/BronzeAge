//! El catálogo de los hechos de dominio (4H T5; contrato 05 §8): cada documento de `Docs/Mechanics` lista en su bloque `<!-- eventos -->` los códigos que define, y
//! - todo código que **emite** el núcleo tiene que estar en el bloque de algún documento;
//! - todo código del catálogo que el núcleo **no** emite (su mecánica es de un sistema que aún no está: la política, la tecnología y los Aedas, la refundación...) tiene que figurar como paridad
//!   pendiente en `Docs/Mecanicas a desarrollar.md`, y desaparecer de ahí cuando se emita.
//!
//! Lo que emite el núcleo se lee de su código fuente (y del de `comercio`, que nombra sus propios hechos): todo literal `"familia.nombre"` que no es la constante de un rechazo
//! (`pub const X: &str = "…"`, que es como se escriben todos), ni la clave de un parámetro (los `datos.rs`, que leen los bloques de parámetros), ni el código de un aviso de sesión. Así, un hecho nuevo
//! que no esté en el catálogo no pasa esta prueba, aunque ninguna otra lo ejercite.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Donde se escriben los códigos de los hechos.
const FUENTES: [&str; 2] = ["src", "../comercio/src"];
/// Los códigos de `Aviso` de la sesión (`reloj.cambiado`, `vista.*`): no son hechos.
const AVISOS: [&str; 3] = ["reloj.cambiado", "vista.invalida", "vista.sin_derecho"];

fn raiz() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn es_codigo(s: &str) -> bool {
    s.split_once('.').is_some_and(|(a, b)| !a.is_empty() && !b.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c == '_' || c == '.') && s.matches('.').count() == 1)
}

fn ficheros(dir: &Path, salida: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            ficheros(&p, salida);
        } else if p.extension().is_some_and(|x| x == "rs") && p.file_name().is_some_and(|n| n != "datos.rs") {
            salida.push(p);
        }
    }
}

/// Los códigos que emite el núcleo: los literales `"familia.nombre"` de su código que no son rechazos, parámetros ni avisos.
fn emitidos() -> BTreeSet<String> {
    let mut rs = Vec::new();
    FUENTES.iter().for_each(|f| ficheros(&raiz().join(f), &mut rs));
    let mut codigos = BTreeSet::new();
    for p in rs {
        for linea in std::fs::read_to_string(&p).unwrap().lines() {
            let l = linea.trim_start();
            if l.starts_with("pub const ") && l.contains(": &str = \"") || l.starts_with("//") {
                continue;
            }
            codigos.extend(l.split('"').skip(1).step_by(2).filter(|s| es_codigo(s) && !AVISOS.contains(s)).map(str::to_owned));
        }
    }
    codigos
}

/// El catálogo: cada código con el documento de `Docs/Mechanics` cuyo bloque `<!-- eventos -->` lo lista.
fn catalogo() -> BTreeMap<String, String> {
    let dir = raiz().join("../../Docs/Mechanics");
    let mut cat = BTreeMap::new();
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "md") {
            continue;
        }
        let texto = std::fs::read_to_string(&p).unwrap();
        let Some((_, resto)) = texto.split_once("<!-- eventos -->") else { continue };
        let bloque = resto.split_once("<!-- /eventos -->").expect("el bloque se cierra").0;
        let json = bloque.trim().trim_start_matches("```json").trim_end_matches("```");
        let codigos: Vec<String> = serde_json::from_str(json).unwrap_or_else(|e| panic!("el bloque de {} no es una lista: {e}", p.display()));
        let doc = p.file_stem().unwrap().to_string_lossy().into_owned();
        for c in codigos {
            assert!(es_codigo(&c), "{c} (en {doc}) no es un código `familia.nombre`");
            if let Some(otro) = cat.insert(c.clone(), doc.clone()) {
                panic!("{c} está en el catálogo de {otro} y en el de {doc}");
            }
        }
    }
    cat
}

#[test]
fn todo_codigo_que_emite_el_nucleo_esta_en_el_catalogo_de_mechanics() {
    let (emitidos, cat) = (emitidos(), catalogo());
    assert!(emitidos.len() > 200 && emitidos.contains("ejercito.llega") && emitidos.contains("cronica.entrada"), "la lectura del código encuentra los hechos: {}", emitidos.len());
    let fuera: Vec<&String> = emitidos.iter().filter(|c| !cat.contains_key(*c)).collect();
    assert!(fuera.is_empty(), "códigos que el núcleo emite y ningún documento de Docs/Mechanics cataloga (bloque `<!-- eventos -->`): {fuera:?}");
}

#[test]
fn lo_que_el_catalogo_tiene_y_el_nucleo_no_emite_es_paridad_pendiente() {
    let (emitidos, cat) = (emitidos(), catalogo());
    let paridad = std::fs::read_to_string(raiz().join("../../Docs/Mecanicas a desarrollar.md")).unwrap();
    let pendientes: Vec<(&String, &String)> = cat.iter().filter(|(c, _)| !emitidos.contains(*c)).collect();
    let sin_fila: Vec<&(&String, &String)> = pendientes.iter().filter(|(c, _)| !paridad.contains(&format!("`{c}`"))).collect();
    assert!(sin_fila.is_empty(), "códigos del catálogo que el núcleo no emite sin su fila de paridad en «Mecanicas a desarrollar»: {sin_fila:?}");
    eprintln!("{} códigos en el catálogo, {} emitidos, {} pendientes", cat.len(), emitidos.len(), pendientes.len());
}
