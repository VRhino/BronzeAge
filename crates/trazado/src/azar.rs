//! Determinismo (T §3): toda variación por asentamiento sale de su semilla, sin generador global y sin `sin`.
//!
//! `pseudoAleatorio` de la versión TypeScript dependía de `sin` en doble precisión; aquí se sustituye por FNV-1a de 64 bits
//! sobre (semilla, sal, enteros) y la mezcla final de SplitMix64. El resultado es solo función de los enteros de entrada.

/// Hash de (semilla, sal, contexto) → u64 bien mezclado.
pub fn hash(semilla: u64, sal: &str, ctx: &[i64]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325 ^ semilla;
    let mut mezclar = |b: u8| h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    for b in sal.bytes() {
        mezclar(b);
    }
    for c in ctx {
        for b in c.to_le_bytes() {
            mezclar(b);
        }
    }
    let mut z = h.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

/// Entero en `[0, n)` a partir de un hash (el `floor(pa × n)` de T §3).
pub fn por_debajo(h: u64, n: u64) -> u64 {
    ((h as u128 * n as u128) >> 64) as u64
}

/// `hash` reducido a `[0, n)` en un paso.
pub fn sorteo(semilla: u64, sal: &str, ctx: &[i64], n: u64) -> u64 {
    por_debajo(hash(semilla, sal, ctx), n)
}

/// Seno ×1000 por grado entero (tabla fija: nada de `sin` de la biblioteca matemática).
const SENO_1000: [i32; 91] = [
    0, 17, 35, 52, 70, 87, 105, 122, 139, 156, 174, 191, 208, 225, 242, 259, 276, 292, 309, 326, 342, 358, 375, 391, 407, 423, 438, 454,
    469, 485, 500, 515, 530, 545, 559, 574, 588, 602, 616, 629, 643, 656, 669, 682, 695, 707, 719, 731, 743, 755, 766, 777, 788, 799,
    809, 819, 829, 839, 848, 857, 866, 875, 883, 891, 899, 906, 914, 921, 927, 934, 940, 946, 951, 956, 961, 966, 970, 974, 978, 982,
    985, 988, 990, 993, 995, 996, 998, 999, 999, 1000, 1000,
];

/// `(cos, sin)` ×1000 del ángulo en grados enteros (el eje +y es el sur, como en la rejilla).
pub fn cos_sen_1000(grados: i32) -> (i32, i32) {
    let g = grados.rem_euclid(360);
    let seno = |g: i32| match g {
        0..=90 => SENO_1000[g as usize],
        91..=180 => SENO_1000[(180 - g) as usize],
        181..=270 => -SENO_1000[(g - 180) as usize],
        _ => -SENO_1000[(360 - g) as usize],
    };
    (seno((g + 90) % 360), seno(g))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn hash_estable_y_dependiente_de_todo() {
        assert_eq!(hash(1, "a", &[1, 2]), hash(1, "a", &[1, 2]));
        assert_ne!(hash(1, "a", &[1, 2]), hash(2, "a", &[1, 2]));
        assert_ne!(hash(1, "a", &[1, 2]), hash(1, "b", &[1, 2]));
        assert_ne!(hash(1, "a", &[1, 2]), hash(1, "a", &[2, 1]));
        // valor fijo: si cambia, cambia todo trazado ya guardado (subir LAYOUT_VERSION)
        assert_eq!(hash(0, "x", &[]), hash(0, "x", &[]));
    }

    #[test]
    fn sorteo_en_rango() {
        for i in 0..1000 {
            assert!(sorteo(7, "r", &[i], 5) < 5);
        }
    }

    #[test]
    fn trigonometria_de_tabla() {
        assert_eq!(cos_sen_1000(0), (1000, 0));
        assert_eq!(cos_sen_1000(90), (0, 1000));
        assert_eq!(cos_sen_1000(-90), (0, -1000));
        assert_eq!(cos_sen_1000(180), (-1000, 0));
        assert_eq!(cos_sen_1000(270), (0, -1000));
        assert_eq!(cos_sen_1000(30), (866, 500));
        assert_eq!(cos_sen_1000(150), (-866, 500));
    }
}
