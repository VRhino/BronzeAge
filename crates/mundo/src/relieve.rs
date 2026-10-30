//! El paquete de relieve (`PaqueteRelieve`, contrato 06 §8): la altura fina de todo el mapa para los clientes 3D. El servidor no lo usa;
//! lo valida al arrancar para no servir uno roto o de otro mundo, y lo sirve tal cual.
//!
//! Reglas (en este orden): se decodifica; hash de la serialización con `hash = ""`; es del artefacto vigente; `paso` y `lado_tesela`
//! mayores que 0; `columnas` = ⌈ancho / paso⌉ + 1 y `filas` = ⌈alto / paso⌉ + 1; `teselas_x` = ⌈(columnas − 1) / lado⌉ y `teselas_y` igual;
//! tantas teselas como `teselas_x · teselas_y`; cada altura descomprime a (lado + 1)² u16 y cada agua, si la hay, a ⌈(lado + 1)² / 8⌉ bytes.

use contrato::v1::PaqueteRelieve;
use prost::Message;

use crate::sha256_hex;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorRelieve {
    #[error("el paquete de relieve no se puede decodificar: {0}")]
    Decodificacion(String),
    #[error("hash del paquete de relieve: declara {esperado}, el contenido da {obtenido}")]
    Hash { esperado: String, obtenido: String },
    #[error("el paquete de relieve es del artefacto {obtenido}, no del vigente ({esperado})")]
    OtroArtefacto { esperado: String, obtenido: String },
    #[error("paquete de relieve: {campo} = {obtenido}, se esperaba {esperado}")]
    Dimension { campo: &'static str, esperado: u64, obtenido: u64 },
    #[error("paquete de relieve: la tesela {tesela} ({capa}) no descomprime: {detalle}")]
    Descompresion { tesela: usize, capa: &'static str, detalle: String },
    #[error("paquete de relieve: la tesela {tesela} ({capa}) tiene {obtenido} bytes en vez de {esperado}")]
    Longitud { tesela: usize, capa: &'static str, esperado: usize, obtenido: usize },
}

/// Valida el paquete contra el artefacto (`hash_artefacto` y `tamano` en milésimas, los del manifiesto). Devuelve el hash del paquete.
pub fn validar_relieve(bytes: &[u8], hash_artefacto: &str, tamano: (i64, i64)) -> Result<String, ErrorRelieve> {
    let mut p = PaqueteRelieve::decode(bytes).map_err(|e| ErrorRelieve::Decodificacion(e.to_string()))?;
    let hash = std::mem::take(&mut p.hash);
    let obtenido = sha256_hex(&p.encode_to_vec());
    if obtenido != hash {
        return Err(ErrorRelieve::Hash { esperado: hash, obtenido });
    }
    if p.hash_artefacto != hash_artefacto {
        return Err(ErrorRelieve::OtroArtefacto { esperado: hash_artefacto.into(), obtenido: p.hash_artefacto });
    }
    let positivo = |campo, v: u32| if v == 0 { Err(ErrorRelieve::Dimension { campo, esperado: 1, obtenido: 0 }) } else { Ok(v as u64) };
    let (paso, lado) = (positivo("paso", p.paso)?, positivo("lado_tesela", p.lado_tesela)?);
    let muestras = |m: i64| (m.max(0) as u64).div_ceil(paso) + 1;
    let exacto = |campo, esperado: u64, obtenido: u64| if esperado == obtenido { Ok(()) } else { Err(ErrorRelieve::Dimension { campo, esperado, obtenido }) };
    exacto("columnas", muestras(tamano.0), p.columnas as u64)?;
    exacto("filas", muestras(tamano.1), p.filas as u64)?;
    exacto("teselas_x", (p.columnas as u64 - 1).div_ceil(lado), p.teselas_x as u64)?;
    exacto("teselas_y", (p.filas as u64 - 1).div_ceil(lado), p.teselas_y as u64)?;
    exacto("teselas", p.teselas_x as u64 * p.teselas_y as u64, p.teselas.len() as u64)?;
    let n = ((lado + 1) * (lado + 1)) as usize;
    for (i, t) in p.teselas.iter().enumerate() {
        for (capa, datos, esperado) in [("altura", &t.altura, n * 2), ("agua", &t.agua, n.div_ceil(8))] {
            if capa == "agua" && datos.is_empty() {
                continue;
            }
            let crudo = zstd::decode_all(datos.as_slice()).map_err(|e| ErrorRelieve::Descompresion { tesela: i, capa, detalle: e.to_string() })?;
            if crudo.len() != esperado {
                return Err(ErrorRelieve::Longitud { tesela: i, capa, esperado, obtenido: crudo.len() });
            }
        }
    }
    Ok(hash)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use contrato::v1::TeselaRelieve;

    /// Un paquete de 10 × 6 u con paso de 0,5 u y teselas de 8 intervalos: 21 × 13 muestras, 3 × 2 teselas.
    fn paquete(hash_artefacto: &str) -> PaqueteRelieve {
        let n = 9 * 9;
        let tesela = |agua: bool| TeselaRelieve {
            altura: zstd::encode_all(vec![7u8; n * 2].as_slice(), 3).unwrap(),
            agua: if agua { zstd::encode_all(vec![1u8; (n as usize).div_ceil(8)].as_slice(), 3).unwrap() } else { vec![] },
        };
        PaqueteRelieve {
            hash: String::new(),
            hash_artefacto: hash_artefacto.into(),
            paso: 500,
            lado_tesela: 8,
            columnas: 21,
            filas: 13,
            teselas_x: 3,
            teselas_y: 2,
            teselas: (0..6).map(|k| tesela(k == 4)).collect(),
        }
    }

    fn con_hash(mut p: PaqueteRelieve) -> Vec<u8> {
        p.hash = String::new();
        p.hash = sha256_hex(&p.encode_to_vec());
        p.encode_to_vec()
    }

    const TAMANO: (i64, i64) = (10_000, 6_000);

    #[test]
    fn un_paquete_bien_hecho_pasa_y_devuelve_su_hash() {
        let bytes = con_hash(paquete("abc"));
        let hash = PaqueteRelieve::decode(bytes.as_slice()).unwrap().hash;
        assert_eq!(validar_relieve(&bytes, "abc", TAMANO), Ok(hash));
    }

    #[test]
    fn se_rechaza_si_es_de_otro_mundo_si_no_cuadra_su_hash_o_sus_dimensiones_o_una_tesela() {
        assert!(matches!(validar_relieve(&con_hash(paquete("otro")), "abc", TAMANO), Err(ErrorRelieve::OtroArtefacto { .. })));
        let mut p = paquete("abc");
        p.hash = "falso".into();
        assert!(matches!(validar_relieve(&p.encode_to_vec(), "abc", TAMANO), Err(ErrorRelieve::Hash { .. })));
        assert_eq!(validar_relieve(&con_hash(paquete("abc")), "abc", (10_500, 6_000)), Err(ErrorRelieve::Dimension { campo: "columnas", esperado: 22, obtenido: 21 }));
        let mut p = paquete("abc");
        p.teselas.pop();
        assert_eq!(validar_relieve(&con_hash(p), "abc", TAMANO), Err(ErrorRelieve::Dimension { campo: "teselas", esperado: 6, obtenido: 5 }));
        let mut p = paquete("abc");
        p.teselas[2].altura = zstd::encode_all([0u8; 10].as_slice(), 3).unwrap();
        assert_eq!(validar_relieve(&con_hash(p), "abc", TAMANO), Err(ErrorRelieve::Longitud { tesela: 2, capa: "altura", esperado: 162, obtenido: 10 }));
        let mut p = paquete("abc");
        p.teselas[1].agua = vec![1, 2, 3];
        assert!(matches!(validar_relieve(&con_hash(p), "abc", TAMANO), Err(ErrorRelieve::Descompresion { tesela: 1, capa: "agua", .. })));
    }
}
