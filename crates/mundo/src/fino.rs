//! El relieve fino del paquete de relieve (`PaqueteRelieve`, 06 §8) leído en el servidor, y qué agua es mar y cuál lago. Lo usa el recorte del lugar de
//! las batallas de Unity (entrada 78; `Campo_De_Batalla_Del_Lugar.md` de Conquest §3). Las teselas se descomprimen al pedirlas: un recorte toca unas pocas.

use contrato::v1::PaqueteRelieve;
use prost::Message;

use crate::{Mundo, TAMANO_CELDA};

/// El paquete de relieve decodificado, con sus teselas aún comprimidas.
#[derive(Debug)]
pub struct RelieveFino {
    paquete: PaqueteRelieve,
}

/// Una tesela descomprimida: (lado + 1)² alturas y bits de agua.
pub struct TeselaFina {
    alturas: Vec<u16>,
    agua: Vec<u8>,
}

impl RelieveFino {
    /// Del paquete ya validado ([`crate::validar_relieve`]).
    pub fn desde(bytes: &[u8]) -> Result<Self, prost::DecodeError> {
        Ok(Self { paquete: PaqueteRelieve::decode(bytes)? })
    }

    /// Milésimas entre muestras (500).
    pub fn paso(&self) -> i64 {
        i64::from(self.paquete.paso)
    }

    /// Muestras del mapa: (columnas, filas).
    pub fn muestras(&self) -> (i64, i64) {
        (i64::from(self.paquete.columnas), i64::from(self.paquete.filas))
    }

    fn lado(&self) -> i64 {
        i64::from(self.paquete.lado_tesela)
    }

    /// La tesela `(tx, ty)` descomprimida, con la predicción deshecha (izquierda + arriba − diagonal, módulo 2^16).
    pub fn tesela(&self, tx: i64, ty: i64) -> TeselaFina {
        let n = (self.lado() + 1) as usize;
        let t = &self.paquete.teselas[(ty * i64::from(self.paquete.teselas_x) + tx) as usize];
        let crudo = zstd::decode_all(t.altura.as_slice()).expect("validado");
        let mut alturas = vec![0u16; n * n];
        for j in 0..n {
            for i in 0..n {
                let k = j * n + i;
                let d = u16::from_le_bytes([crudo[2 * k], crudo[2 * k + 1]]);
                let pred = match (i, j) {
                    (0, 0) => 0,
                    (_, 0) => alturas[k - 1],
                    (0, _) => alturas[k - n],
                    _ => alturas[k - 1].wrapping_add(alturas[k - n]).wrapping_sub(alturas[k - n - 1]),
                };
                alturas[k] = d.wrapping_add(pred);
            }
        }
        let agua = if t.agua.is_empty() { vec![] } else { zstd::decode_all(t.agua.as_slice()).expect("validado") };
        TeselaFina { alturas, agua }
    }

    /// La tesela y la posición dentro de ella de la muestra `(i, j)` del mapa (recortada al mapa).
    pub fn donde(&self, i: i64, j: i64) -> ((i64, i64), usize) {
        let (c, f) = self.muestras();
        let (i, j, l) = (i.clamp(0, c - 1), j.clamp(0, f - 1), self.lado());
        let (tx, ty) = ((i / l).min(i64::from(self.paquete.teselas_x) - 1), (j / l).min(i64::from(self.paquete.teselas_y) - 1));
        ((tx, ty), ((j - ty * l) * (l + 1) + (i - tx * l)) as usize)
    }
}

impl TeselaFina {
    pub fn altura(&self, k: usize) -> u16 {
        self.alturas[k]
    }
    pub fn agua(&self, k: usize) -> bool {
        !self.agua.is_empty() && self.agua[k >> 3] >> (k & 7) & 1 == 1
    }
}

/// Las celdas de agua del raster de terreno que son mar: las de una masa conexa (4 vecinos) que toca el borde del mapa o tiene 2000 celdas o más. Las demás son
/// lagos. Es la regla de la mesa del campo (`marDelTerreno`, `pruebas/campo3d/banco.ts` de BronzeAgeClient).
pub(crate) fn mar_del_terreno(m: &Mundo) -> Vec<bool> {
    const MASA_DE_MAR: usize = 2000;
    let (c, f) = m.rejilla();
    let agua = |k: usize| m.terreno[k] == contrato::v1::ClaseTerreno::TerrenoAgua as u8;
    let (mut mar, mut visto, mut cola) = (vec![false; c * f], vec![false; c * f], Vec::new());
    for k0 in 0..c * f {
        if visto[k0] || !agua(k0) {
            continue;
        }
        cola.clear();
        cola.push(k0);
        visto[k0] = true;
        let (mut ini, mut borde) = (0, false);
        while ini < cola.len() {
            let k = cola[ini];
            ini += 1;
            let (i, j) = (k % c, k / c);
            borde |= i == 0 || j == 0 || i == c - 1 || j == f - 1;
            for v in [(i > 0).then(|| k - 1), (i + 1 < c).then(|| k + 1), (j > 0).then(|| k - c), (j + 1 < f).then(|| k + c)].into_iter().flatten() {
                if !visto[v] && agua(v) {
                    visto[v] = true;
                    cola.push(v);
                }
            }
        }
        if borde || cola.len() >= MASA_DE_MAR {
            for &k in &cola {
                mar[k] = true;
            }
        }
    }
    mar
}

impl Mundo {
    /// Pone el relieve fino (una vez; después no cambia).
    pub fn poner_relieve(&self, r: RelieveFino) {
        let _ = self.relieve.set(r);
    }

    pub fn relieve(&self) -> Option<&RelieveFino> {
        self.relieve.get()
    }

    /// ¿Es mar el agua de `(x, y)`? Lo es si la celda o una de sus 8 vecinas es de una masa de mar del raster de terreno (como en la mesa del campo).
    pub fn mar_en(&self, x: i64, y: i64) -> bool {
        let mar = self.mar.get_or_init(|| mar_del_terreno(self));
        let (c, f) = self.rejilla();
        let t = i64::from(TAMANO_CELDA);
        let (ci, cj) = (x.div_euclid(t), y.div_euclid(t));
        (-1..=1).any(|dj| (-1..=1).any(|di| mar[((cj + dj).clamp(0, f as i64 - 1) as usize) * c + (ci + di).clamp(0, c as i64 - 1) as usize]))
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use contrato::v1::TeselaRelieve;

    /// Una tesela de 2 × 2 intervalos (3 × 3 muestras) con alturas 100, 101, …, 108 en orden de filas y el agua en la muestra central.
    #[test]
    fn la_tesela_deshace_la_prediccion_y_lee_el_agua() {
        let v: Vec<u16> = (100..109).collect();
        let mut d = vec![0u16; 9];
        for j in 0..3 {
            for i in 0..3 {
                let k = j * 3 + i;
                let pred = match (i, j) {
                    (0, 0) => 0u16,
                    (_, 0) => v[k - 1],
                    (0, _) => v[k - 3],
                    _ => v[k - 1].wrapping_add(v[k - 3]).wrapping_sub(v[k - 4]),
                };
                d[k] = v[k].wrapping_sub(pred);
            }
        }
        let bytes: Vec<u8> = d.iter().flat_map(|x| x.to_le_bytes()).collect();
        let p = PaqueteRelieve {
            paso: 500,
            lado_tesela: 2,
            columnas: 3,
            filas: 3,
            teselas_x: 1,
            teselas_y: 1,
            teselas: vec![TeselaRelieve { altura: zstd::encode_all(bytes.as_slice(), 3).unwrap(), agua: zstd::encode_all([0b1_0000u8, 0].as_slice(), 3).unwrap() }],
            ..Default::default()
        };
        let r = RelieveFino::desde(&p.encode_to_vec()).unwrap();
        let t = r.tesela(0, 0);
        assert_eq!((0..9).map(|k| t.altura(k)).collect::<Vec<_>>(), v);
        assert_eq!((0..9).filter(|&k| t.agua(k)).collect::<Vec<_>>(), vec![4]);
        assert_eq!(r.donde(2, 1), ((0, 0), 5));
        assert_eq!(r.donde(9, -3), ((0, 0), 2), "fuera del mapa, la muestra del borde");
    }
}
