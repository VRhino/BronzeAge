//! Secuela de una derrota en campo abierto y reparto del botín (Combate §5, Batalla_Unity_Ciclo §9.4): el perdedor entrega la mitad de su carro,
//! y lo robado más la chatarra se reparte entre las columnas del que gana, por rondas, hasta donde les quepa. Lo que no cabe se pierde.

use crate::datos::Reglas;
use asentamiento::datos::UNO;
use base::Mil;
use militar::{sumar, total, Carga};

/// `trasDerrota`: quita a `carga` `fraccionRobada` (la mitad, hacia abajo) de **cada recurso** y devuelve lo robado. El carro es de la columna: se
/// pierde la mitad entera aunque el héroe que lo llevaba estuviera herido y no combatiera (decisión del autor, 2026-09-14).
pub fn tras_derrota(r: &Reglas, carga: &mut Carga) -> Carga {
    let mut robado = Carga::new();
    for (&rec, n) in carga.iter_mut() {
        let x = (*n as i128 * r.fraccion_robada as i128 / UNO as i128) as Mil;
        if x > 0 {
            robado.insert(rec, x);
            *n -= x;
        }
    }
    carga.retain(|_, n| *n > 0);
    robado
}

/// `cargarBotin`: el ganador carga `botin` hasta `capacidad` (en total, no por recurso); un ladrón sin sitio deja lo que no le cabe. Devuelve lo
/// que cogió.
pub fn cargar_botin(carga: &mut Carga, botin: &Carga, capacidad: Mil) -> Carga {
    let mut cogido = Carga::new();
    let mut lleva = total(carga);
    for (&rec, &n) in botin {
        let x = n.min((capacidad - lleva).max(0));
        if x > 0 {
            sumar(carga, rec, x);
            sumar(&mut cogido, rec, x);
            lleva += x;
        }
    }
    cogido
}

/// El carro de una columna que gana, con lo que ya lleva y lo que le cabe en total (`capacidadCargaDe`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CarroDeColumna {
    pub carga: Carga,
    pub capacidad: Mil,
}

impl CarroDeColumna {
    fn libre(&self) -> Mil {
        self.capacidad - total(&self.carga)
    }
}

/// `repartirBotin`: cada ronda divide lo pendiente a partes iguales entre las columnas con sitio (el resto de la división, de uno en uno a las
/// primeras); la que se llena sale de la siguiente. Termina en, como mucho, tantas rondas como columnas, porque en cada una o se reparte todo o se
/// llena alguna. Devuelve lo que no cupo en ninguna (se pierde).
pub fn repartir_botin(carros: &mut [CarroDeColumna], botin: &Carga) -> Carga {
    let mut pendiente: Carga = botin.iter().filter(|&(_, &n)| n > 0).map(|(&r, &n)| (r, n)).collect();
    for _ in 0..carros.len() {
        let con_sitio: Vec<usize> = (0..carros.len()).filter(|&i| carros[i].libre() > 0).collect();
        if con_sitio.is_empty() || pendiente.is_empty() {
            break;
        }
        let k = con_sitio.len() as i64;
        let de_la_ronda = pendiente.clone();
        for (pos, &i) in con_sitio.iter().enumerate() {
            let parte: Carga = de_la_ronda.iter().map(|(&r, &n)| (r, n / k + ((pos as i64) < n % k) as i64)).filter(|&(_, n)| n > 0).collect();
            let cogido = cargar_botin(&mut carros[i].carga, &parte, carros[i].capacidad);
            for (rec, n) in cogido {
                *pendiente.get_mut(&rec).expect("lo cogido salía de lo pendiente") -= n;
            }
        }
        pendiente.retain(|_, n| *n > 0);
    }
    pendiente
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::resolucion::apoyo::r;
    use asentamiento::Recurso::{Madera, Oro, Piedra, Trigo};

    fn carga(v: &[(asentamiento::Recurso, Mil)]) -> Carga {
        v.iter().copied().collect()
    }

    #[test]
    fn el_derrotado_pierde_la_mitad_de_cada_recurso_hacia_abajo() {
        let mut c = carga(&[(Madera, 10_000), (Piedra, 7), (Oro, 1)]);
        let robado = tras_derrota(r(), &mut c);
        assert_eq!(robado, carga(&[(Madera, 5_000), (Piedra, 3)]), "la mitad hacia abajo; 1 milésima no da nada");
        assert_eq!(c, carga(&[(Madera, 5_000), (Piedra, 4), (Oro, 1)]));
        let mut vacio = Carga::new();
        assert!(tras_derrota(r(), &mut vacio).is_empty());
    }

    #[test]
    fn tras_derrota_conserva_el_total() {
        for n in [0, 1, 2, 999, 1000, 123_457] {
            let mut c = carga(&[(Trigo, n), (Madera, n * 3 + 1)]);
            let antes = total(&c);
            let robado = tras_derrota(r(), &mut c);
            assert_eq!(total(&c) + total(&robado), antes);
        }
    }

    #[test]
    fn el_ladron_carga_hasta_su_capacidad_y_deja_el_resto() {
        let mut c = carga(&[(Madera, 6_000)]);
        let botin = carga(&[(Oro, 3_000), (Piedra, 5_000)]);
        let cogido = cargar_botin(&mut c, &botin, 10_000);
        assert_eq!(cogido, carga(&[(Piedra, 4_000)]), "cabían 4 en total; el orden de recurso pone la piedra antes que el oro");
        assert_eq!(total(&c), 10_000);
        assert!(cargar_botin(&mut c, &botin, 10_000).is_empty(), "lleno no coge nada");
        assert!(cargar_botin(&mut c, &botin, 5_000).is_empty(), "una capacidad menor que lo que lleva tampoco");
    }

    #[test]
    fn el_reparto_por_rondas_rellena_a_las_que_tienen_sitio() {
        let mut carros = vec![CarroDeColumna { carga: Carga::new(), capacidad: 10 }, CarroDeColumna { carga: Carga::new(), capacidad: 1_000 }];
        let perdido = repartir_botin(&mut carros, &carga(&[(Madera, 100)]));
        assert_eq!(carros[0].carga, carga(&[(Madera, 10)]));
        assert_eq!(carros[1].carga, carga(&[(Madera, 90)]), "50 en la primera ronda y los 40 que no cupieron a la primera, en la segunda");
        assert!(perdido.is_empty());
    }

    #[test]
    fn lo_que_no_cabe_en_ninguna_se_pierde() {
        let mut carros = vec![CarroDeColumna { carga: Carga::new(), capacidad: 10 }, CarroDeColumna { carga: carga(&[(Trigo, 5)]), capacidad: 20 }];
        let perdido = repartir_botin(&mut carros, &carga(&[(Madera, 100)]));
        assert_eq!(perdido, carga(&[(Madera, 75)]));
        assert_eq!((total(&carros[0].carga), total(&carros[1].carga)), (10, 20));
    }

    #[test]
    fn el_resto_de_la_division_va_a_las_primeras_columnas() {
        let mut carros = vec![CarroDeColumna { carga: Carga::new(), capacidad: 100 }; 3];
        let perdido = repartir_botin(&mut carros, &carga(&[(Madera, 10)]));
        assert!(perdido.is_empty());
        assert_eq!(carros.iter().map(|c| total(&c.carga)).collect::<Vec<_>>(), vec![4, 3, 3]);
    }

    #[test]
    fn sin_columnas_o_sin_botin_no_pasa_nada() {
        assert_eq!(repartir_botin(&mut [], &carga(&[(Madera, 5)])), carga(&[(Madera, 5)]));
        let mut carros = vec![CarroDeColumna { carga: Carga::new(), capacidad: 100 }];
        assert!(repartir_botin(&mut carros, &Carga::new()).is_empty());
        assert!(repartir_botin(&mut carros, &carga(&[(Madera, 0), (Oro, -3)])).is_empty());
    }

    #[test]
    fn conserva_el_botin_cogido_mas_perdido() {
        for (cap_a, cap_b, cap_c) in [(0, 0, 0), (5, 5, 5), (1, 50, 3), (100, 100, 100), (7, 0, 20)] {
            let mut carros: Vec<CarroDeColumna> = [cap_a, cap_b, cap_c].iter().map(|&capacidad| CarroDeColumna { carga: Carga::new(), capacidad }).collect();
            let botin = carga(&[(Madera, 37), (Piedra, 12), (Oro, 5)]);
            let perdido = repartir_botin(&mut carros, &botin);
            let cogido: Mil = carros.iter().map(|c| total(&c.carga)).sum();
            assert_eq!(cogido + total(&perdido), total(&botin), "capacidades {cap_a}/{cap_b}/{cap_c}");
            assert!(carros.iter().all(|c| total(&c.carga) <= c.capacidad), "ninguna columna pasa de su capacidad");
        }
    }
}
