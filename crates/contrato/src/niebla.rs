//! La niebla parcial (contrato 05 §5, 4H T6): a quien ya tiene la `NieblaProyectada` le llegan solo los tramos de bytes que cambiaron de cada máscara. [`cambio_de_niebla`] es lo que calcula
//! el servidor; [`aplicar_niebla`], lo que hace el cliente con la niebla de una `Foto` o de una `Delta`.

use crate::v1 as pb;

/// Bytes iguales que se mandan dentro de un tramo antes que abrir otro (un tramo cuesta unos 5 bytes de cabecera).
const HUECO_EN_UN_TRAMO: usize = 8;

/// Los tramos de bytes de `ahora` que difieren de `antes` (de la misma longitud), juntando los separados por menos de [`HUECO_EN_UN_TRAMO`] bytes iguales.
fn tramos(antes: &[u8], ahora: &[u8]) -> Vec<pb::TramoDeNiebla> {
    let mut v: Vec<pb::TramoDeNiebla> = Vec::new();
    let mut i = 0;
    while i < ahora.len() {
        if antes[i] == ahora[i] {
            i += 1;
            continue;
        }
        let mut fin = i + 1;
        while fin < ahora.len() && ahora[fin..].iter().zip(&antes[fin..]).take(HUECO_EN_UN_TRAMO).any(|(a, b)| a != b) {
            fin += 1;
        }
        v.push(pb::TramoDeNiebla { desde: i as u32, datos: ahora[i..fin].to_vec() });
        i = fin;
    }
    v
}

/// La niebla que se manda a quien tiene `antes` y le toca `ahora` (4H T6): un cambio parcial con los tramos de bytes que cambiaron de cada máscara, o la niebla entera si cambiaron las dimensiones
/// o los tramos no ahorran nada. La máscara de lo visible cambia en casi cada muestreo de 250 ms (los ojos andan), pero solo en el borde de sus discos: unos cientos de bytes de los 26,7 KB de cada
/// máscara del mapa real.
pub fn cambio_de_niebla(antes: &pb::NieblaProyectada, ahora: &pb::NieblaProyectada) -> pb::NieblaProyectada {
    let dimensiones = |x: &pb::NieblaProyectada| (x.tamano_celda, x.columnas, x.filas, x.celdas.len(), x.visibles.len());
    if dimensiones(antes) != dimensiones(ahora) || antes.parcial || ahora.parcial {
        return ahora.clone();
    }
    let (tramos_celdas, tramos_visibles) = (tramos(&antes.celdas, &ahora.celdas), tramos(&antes.visibles, &ahora.visibles));
    let bytes: usize = tramos_celdas.iter().chain(&tramos_visibles).map(|t| t.datos.len() + 5).sum();
    if bytes >= ahora.celdas.len() + ahora.visibles.len() {
        return ahora.clone();
    }
    pb::NieblaProyectada { tamano_celda: ahora.tamano_celda, columnas: ahora.columnas, filas: ahora.filas, parcial: true, tramos_celdas, tramos_visibles, ..Default::default() }
}

/// Lo que hace el cliente con la niebla de una `Delta` o de una `Foto`: la entera sustituye a la que tiene; la parcial sobrescribe los bytes de sus tramos en la que tiene.
pub fn aplicar_niebla(destino: &mut Option<pb::NieblaProyectada>, llega: &pb::NieblaProyectada) {
    if !llega.parcial {
        *destino = Some(llega.clone());
        return;
    }
    let d = destino.as_mut().expect("una niebla parcial llega sobre la que el cliente ya tiene");
    for (mascara, tramos) in [(&mut d.celdas, &llega.tramos_celdas), (&mut d.visibles, &llega.tramos_visibles)] {
        for t in tramos {
            mascara[t.desde as usize..t.desde as usize + t.datos.len()].copy_from_slice(&t.datos);
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn niebla(celdas: &[u8], visibles: &[u8]) -> pb::NieblaProyectada {
        pb::NieblaProyectada { tamano_celda: 5_000, columnas: 8 * celdas.len() as u32, filas: 1, celdas: celdas.to_vec(), visibles: visibles.to_vec(), ..Default::default() }
    }

    #[test]
    fn la_niebla_parcial_lleva_solo_los_tramos_que_cambiaron_y_aplicada_da_la_nueva() {
        let mut antes = vec![0u8; 400];
        antes[10] = 0xFF;
        let (mut celdas, mut visibles) = (antes.clone(), antes.clone());
        celdas[11] = 1;
        visibles[100] = 3;
        visibles[104] = 3; // a menos de 8 bytes del anterior: el mismo tramo
        visibles[300] = 7;
        let (a, b) = (niebla(&antes, &antes), niebla(&celdas, &visibles));
        let c = cambio_de_niebla(&a, &b);
        assert!(c.parcial && c.celdas.is_empty() && c.visibles.is_empty());
        assert_eq!(c.tramos_celdas.iter().map(|t| (t.desde, t.datos.len())).collect::<Vec<_>>(), [(11, 1)]);
        assert_eq!(c.tramos_visibles.iter().map(|t| (t.desde, t.datos.len())).collect::<Vec<_>>(), [(100, 5), (300, 1)]);
        let mut cliente = Some(a.clone());
        aplicar_niebla(&mut cliente, &c);
        assert_eq!(cliente.as_ref(), Some(&b));
        // Sin cambios: ningún tramo. Todo cambiado, o con otras dimensiones: la niebla entera.
        assert!(cambio_de_niebla(&b, &b).tramos_celdas.is_empty() && cambio_de_niebla(&b, &b).tramos_visibles.is_empty());
        let todo = niebla(&[0xAA; 400], &[0x55; 400]);
        assert_eq!(cambio_de_niebla(&a, &todo), todo);
        assert_eq!(cambio_de_niebla(&a, &niebla(&[1; 10], &[1; 10])), niebla(&[1; 10], &[1; 10]));
        let mut cliente = Some(a);
        aplicar_niebla(&mut cliente, &todo);
        assert_eq!(cliente, Some(todo));
    }
}
