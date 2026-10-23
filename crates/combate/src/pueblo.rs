//! El saqueo de un pueblo extractor (Combate §7.3, Doc 4.2.2): una batalla propia de saqueo, **nunca de conquista**. Defienden los 30 mineros de
//! serie (IA, sin dueño, fuera de cupo) y quien lo defienda; el pueblo **protege el 50 % de su capacidad** y el atacante se lleva solo lo que
//! esté por encima, recurso a recurso, hasta lo que le quepa en los carros. La guarnición de serie que pierde mineros se repone gratis, uno a
//! uno (un minero cada 12 minutos: 30 en 6 horas), desde la primera baja.

use crate::botin::{repartir_botin, CarroDeColumna};
use crate::datos::Reglas;
use crate::resolucion::{resolver_combate, Bando, CombateInvalido, Informe, SIN_DUENO};
use asentamiento::datos::UNO;
use asentamiento::recurso::TODOS;
use asentamiento::Almacen;
use base::{Instante, Mil, Rng};
use militar::escuadra::Escuadra;
use militar::{total, Carga, EscuadraId, Tropa};

/// La guarnición de serie de un pueblo: los mineros, sin dueño. Con 30 mineros de poder base 1 (leva) vale 30 y se reparten entre varias
/// batallas con la misma identidad (la escuadra persiste a 0 y se repone).
pub fn guarnicion_de_serie(r: &Reglas, id: EscuadraId, ahora: Instante) -> Escuadra {
    Escuadra::nueva(&r.militar, id, SIN_DUENO, Tropa::Mineros, r.pueblos.mineros, None, ahora)
}

/// Cuándo vuelve el siguiente minero de una guarnición mermada (desde la baja o desde el minero anterior).
pub fn reposicion_en(r: &Reglas, desde: Instante) -> Instante {
    desde + r.pueblos.cada_minero
}

/// Vuelve un minero a la guarnición de serie, con su identidad (la experiencia y el nivel que ganó se conservan). `true` si ya está completa.
pub fn reponer(r: &Reglas, guarnicion: &mut Escuadra) -> bool {
    guarnicion.cantidad = (guarnicion.cantidad + 1).min(r.pueblos.mineros);
    guarnicion.cantidad >= r.pueblos.mineros
}

/// El combate de un saqueo: `resolverCombate` con la **ventaja del defensor del aguante** (×1,25) como multiplicador del defensor, que se suma a la
/// ventaja de 1,05 de todo defensor: ×1,3125 (Combate §7.3).
pub fn resolver_saqueo(r: &Reglas, atacante: Bando, defensor: Bando, rng: &mut Rng) -> Result<Informe, CombateInvalido> {
    resolver_combate(r, atacante, defensor, r.numericas.ventaja_aguante, rng)
}

/// Lo que el pueblo deja llevarse: por recurso, lo que su almacén guarda por encima de la mitad de su capacidad.
pub fn saqueable(r: &Reglas, almacen: &Almacen) -> Carga {
    let mut c = Carga::new();
    for &rec in TODOS.iter() {
        let protegido = (almacen.capacidad(rec) as i128 * r.numericas.protegida_pueblo as i128 / UNO as i128) as Mil;
        let libre = almacen.cantidad(rec) - protegido;
        if libre > 0 {
            c.insert(rec, libre);
        }
    }
    c
}

/// Si gana el atacante: reparte lo saqueable entre los carros de sus columnas (hasta lo que quepa) y quita del almacén **solo lo que se llevan**.
/// Devuelve lo llevado. Si gana el defensor, no se llama.
pub fn saquear(r: &Reglas, almacen: &mut Almacen, carros: &mut [CarroDeColumna]) -> Carga {
    let botin = saqueable(r, almacen);
    let perdido = repartir_botin(carros, &botin);
    let mut llevado = Carga::new();
    for (&rec, &n) in &botin {
        let x = n - perdido.get(&rec).copied().unwrap_or(0);
        if x > 0 {
            almacen.quitar(rec, x);
            llevado.insert(rec, x);
        }
    }
    debug_assert!(total(&llevado) <= total(&botin));
    llevado
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::resolucion::apoyo::*;
    use crate::resolucion::Lado;
    use asentamiento::Recurso::{Cobre, Madera, Piedra};
    use base::MS_POR_MINUTO;

    fn almacen_con(rec: asentamiento::Recurso, cantidad: Mil) -> Almacen {
        let mut a = Almacen::nuevo(500_000);
        a.poner(rec, cantidad);
        a
    }

    #[test]
    fn los_30_mineros_son_una_escuadra_sin_dueno_de_poder_30() {
        let r = r();
        let g = guarnicion_de_serie(r, 5, 0);
        assert_eq!((g.tropa, g.cantidad, g.heroe, g.prestada, g.plaza_origen), (Tropa::Mineros, 30, SIN_DUENO, None, None));
        assert_eq!(g.poder(&r.militar), 30_300, "30 mineros de poder 1, con el +1 % del nivel 1");
    }

    #[test]
    fn la_reposicion_gratuita_es_un_minero_cada_12_minutos_y_30_en_seis_horas() {
        let r = r();
        assert_eq!(reposicion_en(r, 10), 10 + 12 * MS_POR_MINUTO);
        let mut g = guarnicion_de_serie(r, 5, 0);
        g.cantidad = 0;
        g.experiencia = 7_000;
        let (mut t, mut citas) = (0, 0);
        loop {
            t = reposicion_en(r, t);
            citas += 1;
            if reponer(r, &mut g) {
                break;
            }
        }
        assert_eq!((citas, t), (30, 6 * 60 * MS_POR_MINUTO), "una cita por minero; desde 0, completa a las 6 h");
        assert_eq!((g.cantidad, g.experiencia), (30, 7_000), "conserva su experiencia");
    }

    #[test]
    fn el_ejemplo_del_almacen_de_500_con_250_275_y_500() {
        let r = r();
        assert!(saqueable(r, &almacen_con(Cobre, 250_000)).is_empty(), "con 250 o menos, nada");
        assert!(saqueable(r, &almacen_con(Cobre, 0)).is_empty());
        assert_eq!(saqueable(r, &almacen_con(Cobre, 275_000)).get(&Cobre), Some(&25_000));
        assert_eq!(saqueable(r, &almacen_con(Cobre, 500_000)).get(&Cobre), Some(&250_000));
    }

    #[test]
    fn el_saquear_reparte_lo_que_cabe_y_solo_lo_que_se_llevan_sale_del_almacen() {
        let r = r();
        let mut alm = almacen_con(Cobre, 500_000);
        alm.poner(Madera, 400_000);
        let mut carros = vec![CarroDeColumna { carga: Carga::new(), capacidad: 100_000 }, CarroDeColumna { carga: Carga::new(), capacidad: 60_000 }];
        let llevado = saquear(r, &mut alm, &mut carros);
        assert_eq!(total(&llevado), 160_000, "los carros se llenan: 100 + 60");
        let en_carros: Mil = carros.iter().map(|c| total(&c.carga)).sum();
        assert_eq!(en_carros, 160_000);
        assert_eq!(alm.cantidad(Cobre) + alm.cantidad(Madera), 900_000 - 160_000, "lo que no cupo se queda en el pueblo");
        assert!(alm.cantidad(Cobre) >= 250_000 && alm.cantidad(Madera) >= 250_000, "nunca se baja de la mitad protegida");
    }

    #[test]
    fn sin_carros_no_se_lleva_nada_y_el_almacen_no_cambia() {
        let r = r();
        let mut alm = almacen_con(Piedra, 500_000);
        let antes = alm.clone();
        assert!(saquear(r, &mut alm, &mut []).is_empty());
        assert_eq!(alm, antes);
    }

    #[test]
    fn el_defensor_tiene_la_ventaja_del_aguante_ademas_de_la_suya() {
        let r = r();
        let mut g = [guarnicion_de_serie(r, 1, 0)];
        let mut a = [escuadra(2, 7, Tropa::LancerosMimbre)];
        let inf = resolver_saqueo(r, Bando { escuadras: &mut a, heroes: &[7] }, Bando { escuadras: &mut g, heroes: &[] }, &mut Rng::nuevo(4)).unwrap();
        let mut rng = Rng::nuevo(4);
        rng.siguiente();
        let jd = crate::resolucion::tirada(r, &mut rng);
        let u = UNO as i128;
        assert_eq!(inf.defensor.poder, (30_300i128 * jd as i128 * 1_250_000 * 1_050_000 / (u * u * u)) as Mil);
    }

    #[test]
    fn unos_mineros_aguantan_a_un_heroe_solo_y_caen_ante_un_ejercito() {
        let r = r();
        let (mut solo, mut ejercito) = (0, 0);
        for semilla in 0..300 {
            let mut g = [guarnicion_de_serie(r, 1, 0)];
            let i = resolver_saqueo(r, Bando { escuadras: &mut [], heroes: &[7] }, Bando { escuadras: &mut g, heroes: &[] }, &mut Rng::nuevo(semilla)).unwrap();
            solo += (i.ganador == Lado::Atacante) as u32;
            let mut g = [guarnicion_de_serie(r, 1, 0)];
            let mut a = [escuadra(2, 7, Tropa::ArquerosCompuesto), escuadra(3, 7, Tropa::ArquerosCompuesto)];
            let i = resolver_saqueo(r, Bando { escuadras: &mut a, heroes: &[7] }, Bando { escuadras: &mut g, heroes: &[] }, &mut Rng::nuevo(semilla)).unwrap();
            ejercito += (i.ganador == Lado::Atacante) as u32;
        }
        assert_eq!((solo, ejercito), (0, 300));
    }
}
