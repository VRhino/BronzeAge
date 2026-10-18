//! Imagen de fondo del mapa: un BMP de 24 bits (sin compresión, sin dependencias) como URI de datos en base64.

const ALFABETO: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(datos: &[u8]) -> String {
    let mut o = String::with_capacity(datos.len().div_ceil(3) * 4);
    for trozo in datos.chunks(3) {
        let n = (trozo[0] as u32) << 16 | (*trozo.get(1).unwrap_or(&0) as u32) << 8 | *trozo.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            if k <= trozo.len() {
                o.push(ALFABETO[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                o.push('=');
            }
        }
    }
    o
}

/// `rgb` son `columnas × filas` píxeles RGB, de arriba abajo. Devuelve `None` si el tamaño no cuadra.
pub fn bmp_uri(columnas: usize, filas: usize, rgb: &[u8]) -> Option<String> {
    if columnas == 0 || filas == 0 || rgb.len() != columnas * filas * 3 {
        return None;
    }
    let fila_bytes = (columnas * 3).div_ceil(4) * 4;
    let tam_pixeles = fila_bytes * filas;
    let mut b = Vec::with_capacity(54 + tam_pixeles);
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&((54 + tam_pixeles) as u32).to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&54u32.to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(columnas as i32).to_le_bytes());
    b.extend_from_slice(&(filas as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&24u16.to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&(tam_pixeles as u32).to_le_bytes());
    b.extend_from_slice(&[0; 16]);
    for f in (0..filas).rev() {
        let fila = &rgb[f * columnas * 3..(f + 1) * columnas * 3];
        for p in fila.chunks(3) {
            b.extend_from_slice(&[p[2], p[1], p[0]]);
        }
        b.resize(b.len() + fila_bytes - columnas * 3, 0);
    }
    Some(format!("data:image/bmp;base64,{}", base64(&b)))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn base64_conocido() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn bmp_tiene_cabecera_y_relleno_de_fila() {
        // 1 × 2 píxeles: cada fila ocupa 4 bytes (3 + 1 de relleno), 54 de cabecera, 62 en total.
        let uri = bmp_uri(1, 2, &[1, 2, 3, 4, 5, 6]).unwrap();
        assert!(uri.starts_with("data:image/bmp;base64,Qk0"), "{uri}"); // «BM»
        assert_eq!(uri.len() - "data:image/bmp;base64,".len(), 62usize.div_ceil(3) * 4);
        assert!(bmp_uri(2, 2, &[0; 5]).is_none());
        assert!(bmp_uri(0, 1, &[]).is_none());
    }
}
