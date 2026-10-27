//! Los campamentos de bandidos (Doc 1.9, Combate §7.2, Generacion_Del_Mundo §13): cuándo aparecen y reaparecen, cuándo acosan a una caravana,
//! qué pasa cuando una columna los ataca y el botín que dan. Planes y decisiones puras: el sitio donde caen (el bosque no reclamado más cercano)
//! y la geometría del acoso (el radio, las zonas) son del núcleo, que programa los eventos y llama aquí.
//!
//! Los bandidos **no actúan dentro de ninguna zona de influencia**, ni junto a un campamento de mercenarios. Es PvE: no exige guerra.

use crate::caravana::{defensa_de, Defensa};
use crate::datos::{NivelBandidos, Reglas};
use crate::resolucion::{aplicar_bajas, ganador_de, poder_de_bando, tirada, Baja, Bando, CombateInvalido, Lado};
use asentamiento::datos::UNO;
use base::{Instante, Mil, Punto, Rng};
use comercio::caravana::EstadoCaravana;
use militar::{Carga, HeroeId};

/// Niveles de un campamento: 1, 2 y 3.
pub fn nivel_de(r: &Reglas, nivel: u8) -> &NivelBandidos {
    &r.bandidos.niveles[nivel.clamp(1, 3) as usize - 1]
}

/// `nivelAlAzar`: una tirada; `t −= peso` para los niveles 1, 2, 3 en ese orden y el primero que deja `t < 0`. Si ninguno, el 1.
pub fn nivel_al_azar(r: &Reglas, rng: &mut Rng) -> u8 {
    let mut t = rng.por_debajo(UNO as u64) as i64;
    for (i, n) in r.bandidos.niveles.iter().enumerate() {
        t -= n.peso;
        if t < 0 {
            return i as u8 + 1;
        }
    }
    1
}

/// Un campamento por plaza, siempre: aparece uno cuando no tiene y ha vencido su plazo de reaparición (una plaza sin plazo lo recibe ya).
pub fn aparece_junto_a_plaza(tiene_campamento: bool, reaparece_en: Option<Instante>, ahora: Instante) -> bool {
    !tiene_campamento && reaparece_en.is_none_or(|t| ahora >= t)
}

/// Destruido el campamento de una plaza, ese es el instante en que vuelve el suyo.
pub fn reaparece_en(r: &Reglas, ahora: Instante) -> Instante {
    ahora + r.bandidos.respawn
}

/// Cuántos campamentos le tocan al anillo de un campamento de mercenarios según su demanda: `residentes sin plaza × bandidosPorResidente`
/// entre el mínimo y el máximo. Cuentan los residentes cuya Facción no tiene asentamientos o que no tienen Facción.
pub fn campamentos_que_tocan(r: &Reglas, residentes_sin_plaza: i64) -> i64 {
    let b = &r.bandidos;
    (residentes_sin_plaza * b.bandidos_por_residente).clamp(b.anillo_minimo, b.anillo_maximo)
}

/// ¿Toca crear uno en el anillo? Faltan campamentos y ya venció su plazo (`bandidosEn`).
pub fn toca_crear_en_el_anillo(r: &Reglas, existentes: i64, residentes_sin_plaza: i64, bandidos_en: Option<Instante>, ahora: Instante) -> bool {
    bandidos_en.is_none_or(|t| ahora >= t) && existentes < campamentos_que_tocan(r, residentes_sin_plaza)
}

/// Tras crear uno en el anillo, cuándo puede aparecer el siguiente.
pub fn proxima_aparicion_en_el_anillo(r: &Reglas, ahora: Instante) -> Instante {
    ahora + r.bandidos.reaparicion_anillo
}

/// Intentos de colocar un campamento en el anillo antes de dejarlo para la próxima cita.
pub const INTENTOS_ANILLO: usize = 20;

/// Un punto del anillo (`anillo.radioMin`‥`radioMax`) alrededor de `centro` que cumpla `valido` (tierra firme, fuera de toda zona y de la
/// protección de cualquier campamento: lo sabe el núcleo). Como el TypeScript (`puntoDelAnillo`), cada intento sortea primero el ángulo y
/// luego el radio, uniforme en el radio; en enteros, con el ángulo en grados enteros y la tabla de senos del trazado. Hasta
/// [`INTENTOS_ANILLO`] intentos de **dos** tiradas cada uno; sin sitio, `None` (y se reintenta en la próxima cita, gastando tiradas).
pub fn punto_del_anillo(r: &Reglas, centro: Punto, rng: &mut Rng, mut valido: impl FnMut(Punto) -> bool) -> Option<Punto> {
    let (min, max) = (r.bandidos.anillo_min, r.bandidos.anillo_max);
    for _ in 0..INTENTOS_ANILLO {
        let (cos, sen) = trazado::cos_sen_1000(rng.por_debajo(360) as i32);
        let radio = min + rng.por_debajo((max - min + 1) as u64) as i64;
        let p = Punto::new(centro.x + radio * cos as i64 / 1000, centro.y + radio * sen as i64 / 1000);
        if valido(p) {
            return Some(p);
        }
    }
    None
}

/// ¿Puede el campamento acosar a esta caravana? No si está parada en una plaza (`disponible`, `preparando`, `aparcada`), ni si está dentro de
/// cualquier zona de influencia, ni en la protección de un campamento de mercenarios, ni más lejos que `radioAtaqueCaravana`.
pub fn puede_acosar(r: &Reglas, estado: EstadoCaravana, distancia: Mil, en_zona: bool, en_proteccion_de_campamento: bool) -> bool {
    !matches!(estado, EstadoCaravana::Disponible | EstadoCaravana::Preparando | EstadoCaravana::Aparcada)
        && !en_zona
        && !en_proteccion_de_campamento
        && distancia <= r.bandidos.radio_ataque_caravana
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Acoso {
    /// La caravana desaparece (no hay quien reciba la carga) y su escolta sin héroe queda a 0 y vuelve al campamento de su héroe.
    pub gana_el_bandido: bool,
    pub poder_bandido: Mil,
    pub defensa: Mil,
    pub bajas_escolta: Vec<Baja>,
}

/// El acoso: **una** tirada, `poder del campamento × tirada > defensa`. La defensa es el ejército al que va adjunta (no sufre nada), la escolta sin
/// héroe (25 % si el bandido gana, 5 % si no; a 0 si cae) o la base. Los bandidos no tienen ventaja de tipo.
pub fn resolver_acoso(r: &Reglas, nivel: u8, defensa: Defensa, rng: &mut Rng) -> Acoso {
    let poder = (nivel_de(r, nivel).poder as i128 * tirada(r, rng) as i128 / UNO as i128) as Mil;
    let d = defensa_de(r, &defensa);
    let gana = ganador_de(poder, d) == Lado::Atacante;
    let n = &r.numericas;
    let bajas_escolta = match defensa {
        Defensa::Escolta(e) => {
            let bajas = aplicar_bajas(r, e, if gana { n.bajas_fijas_perdedor } else { n.bajas_fijas_ganador }, !gana, &mut Carga::new());
            if gana {
                e.iter_mut().for_each(|x| x.cantidad = 0);
            }
            bajas
        }
        Defensa::Base | Defensa::Ejercito(_) => vec![],
    };
    Acoso { gana_el_bandido: gana, poder_bandido: poder, defensa: d, bajas_escolta }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtaqueACampamento {
    /// El campamento cae (su oro lo da [`botin_de_bandidos`]); si no, aguanta.
    pub destruido: bool,
    pub poder_atacante: Mil,
    pub poder_campamento: Mil,
    pub bajas: Vec<Baja>,
    /// Si el campamento aguanta, los héroes de la columna quedan heridos y la columna pierde la mitad del carro (`tras_derrota`; nadie se la queda).
    pub heridos: Vec<HeroeId>,
}

/// Una columna ataca un campamento: `poder × tirada > poder del nivel` con **una** tirada. Gana → bajas del 5 %; pierde → del 25 %. Basta con
/// un bando que combata (el TypeScript pedía soldados): un héroe solo puede atacar, pero nunca destruye ningún campamento (15 × 1,15 < 20,
/// Combate §7.2).
pub fn resolver_campamento(r: &Reglas, atacante: Bando, nivel: u8, rng: &mut Rng) -> Result<AtaqueACampamento, CombateInvalido> {
    if atacante.escuadras.iter().all(|e| !e.con_soldados()) && atacante.heroes.is_empty() {
        return Err(CombateInvalido::BandoVacio(Lado::Atacante));
    }
    let poder = (poder_de_bando(r, atacante.escuadras, atacante.heroes.len()) as i128 * tirada(r, rng) as i128 / UNO as i128) as Mil;
    let objetivo = nivel_de(r, nivel).poder;
    let destruido = ganador_de(poder, objetivo) == Lado::Atacante;
    let n = &r.numericas;
    let bajas = aplicar_bajas(r, atacante.escuadras, if destruido { n.bajas_fijas_ganador } else { n.bajas_fijas_perdedor }, destruido, &mut Carga::new());
    Ok(AtaqueACampamento { destruido, poder_atacante: poder, poder_campamento: objetivo, bajas, heridos: if destruido { vec![] } else { atacante.heroes.to_vec() } })
}

/// El oro de un campamento destruido para **un** héroe, con rendimientos decrecientes (D22, D26, D27): en la ventana de 24 h, completo las primeras
/// `completas` veces, luego `caidaPorCada` menos por cada una más, y desde `soloExperienciaDesde`, nada. `recientes` son los instantes en que ese
/// héroe destruyó campamentos: se podan los de fuera de la ventana y se anota este.
pub fn botin_de_bandidos(r: &Reglas, recientes: &mut Vec<Instante>, nivel: u8, ahora: Instante) -> Mil {
    let b = &r.bandidos;
    recientes.retain(|&t| t > ahora - b.ventana);
    let n = recientes.len() as i64 + 1;
    let factor = if n <= b.completas {
        UNO
    } else if n >= b.solo_experiencia_desde {
        0
    } else {
        (UNO - b.caida_por_cada * (n - b.completas)).max(0)
    };
    recientes.push(ahora);
    // En unidades enteras de oro, hacia abajo (C3b): nunca se crea de más.
    base::entero((nivel_de(r, nivel).oro_por_heroe as i128 * factor as i128 / UNO as i128) as Mil)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::resolucion::apoyo::*;
    use base::MS_POR_MINUTO;
    use militar::Tropa;

    #[test]
    fn el_nivel_al_azar_usa_una_tirada_y_los_pesos_50_30_20() {
        let r = r();
        let mut rng = Rng::nuevo(11);
        let mut espejo = Rng::nuevo(11);
        let mut cuenta = [0u32; 3];
        for _ in 0..20_000 {
            cuenta[nivel_al_azar(r, &mut rng) as usize - 1] += 1;
            espejo.siguiente();
        }
        assert_eq!(rng.estado(), espejo.estado(), "una tirada por campamento");
        let pct = cuenta.map(|c| c as f64 / 200.0);
        assert!((pct[0] - 50.0).abs() < 2.0 && (pct[1] - 30.0).abs() < 2.0 && (pct[2] - 20.0).abs() < 2.0, "{pct:?}");
    }

    #[test]
    fn el_poder_de_cada_nivel_sale_de_los_datos() {
        let r = r();
        assert_eq!([1, 2, 3].map(|n| nivel_de(r, n).poder), [20_000, 60_000, 120_000]);
        assert_eq!(nivel_de(r, 9).poder, 120_000, "un nivel fuera de rango se acota");
    }

    #[test]
    fn una_plaza_recibe_su_campamento_cuando_no_tiene_y_vencio_el_plazo() {
        assert!(aparece_junto_a_plaza(false, None, 0));
        assert!(!aparece_junto_a_plaza(true, None, 0));
        assert!(!aparece_junto_a_plaza(false, Some(100), 99));
        assert!(aparece_junto_a_plaza(false, Some(100), 100), "justo al cumplirse el plazo");
        assert_eq!(reaparece_en(r(), 5 * MS_POR_MINUTO), 365 * MS_POR_MINUTO, "6 h después");
    }

    #[test]
    fn el_anillo_pide_uno_por_cada_dos_residentes_sin_plaza_entre_1_y_6() {
        let r = r();
        assert_eq!([0, 1, 2, 3, 4, 5, 6, 7, 40].map(|n| campamentos_que_tocan(r, n)), [4, 4, 4, 6, 8, 10, 12, 12, 12]);
        assert!(toca_crear_en_el_anillo(r, 0, 0, None, 0));
        assert!(!toca_crear_en_el_anillo(r, 4, 2, None, 0), "ya tiene los que le tocan");
        assert!(toca_crear_en_el_anillo(r, 4, 3, None, 0));
        assert!(!toca_crear_en_el_anillo(r, 0, 3, Some(50), 49), "espera su plazo");
        assert!(toca_crear_en_el_anillo(r, 0, 3, Some(50), 50));
        assert_eq!(proxima_aparicion_en_el_anillo(r, 0), 10 * MS_POR_MINUTO);
    }

    #[test]
    fn el_punto_del_anillo_cae_entre_los_radios_y_es_reproducible() {
        let r = r();
        let centro = Punto::new(1_000_000, -2_000_000);
        for semilla in 0..300 {
            let p = punto_del_anillo(r, centro, &mut Rng::nuevo(semilla), |_| true).expect("sin restricciones, el primer intento vale");
            let d = p.dist(centro);
            assert!((11_990..=25_010).contains(&d), "{d} (la tabla de senos redondea a la milésima)");
            assert_eq!(Some(p), punto_del_anillo(r, centro, &mut Rng::nuevo(semilla), |_| true));
        }
    }

    #[test]
    fn el_punto_del_anillo_es_uniforme_en_el_radio_como_el_typescript() {
        let r = r();
        let mut rng = Rng::nuevo(5);
        let n = 20_000;
        let cerca = (0..n).filter(|_| punto_del_anillo(r, Punto::default(), &mut rng, |_| true).unwrap().dist(Punto::default()) < 18_500).count();
        // Uniforme en el radio, la mitad interior (12-18,5) recibe la mitad; uniforme en el área recibiría bastante menos.
        let pct = cerca as f64 * 100.0 / n as f64;
        assert!((pct - 50.0).abs() < 1.5, "{pct} %");
    }

    #[test]
    fn sin_sitio_valido_gasta_las_20_tiradas_dobles_y_no_devuelve_nada() {
        let r = r();
        let mut rng = Rng::nuevo(3);
        let mut espejo = Rng::nuevo(3);
        for _ in 0..2 * INTENTOS_ANILLO {
            espejo.siguiente();
        }
        assert_eq!(punto_del_anillo(r, Punto::default(), &mut rng, |_| false), None);
        assert_eq!(rng.estado(), espejo.estado());
    }

    #[test]
    fn solo_acosa_a_una_caravana_en_ruta_cerca_y_fuera_de_toda_zona() {
        let r = r();
        let ok = |e, d, z, p| puede_acosar(r, e, d, z, p);
        assert!(ok(EstadoCaravana::EnTransito, 7_000, false, false), "justo en el radio de ataque (7)");
        assert!(ok(EstadoCaravana::Retornando, 0, false, false));
        assert!(!ok(EstadoCaravana::EnTransito, 7_001, false, false));
        assert!(!ok(EstadoCaravana::EnTransito, 1, true, false), "dentro de una zona (de quien sea)");
        assert!(!ok(EstadoCaravana::EnTransito, 1, false, true), "protección de un campamento de mercenarios");
        for parada in [EstadoCaravana::Disponible, EstadoCaravana::Preparando, EstadoCaravana::Aparcada] {
            assert!(!ok(parada, 1, false, false));
        }
    }

    #[test]
    fn contra_la_defensa_base_un_nivel_1_nunca_gana_y_un_nivel_3_casi_siempre() {
        let r = r();
        let (mut n1, mut n3) = (0, 0);
        for semilla in 0..1000 {
            n1 += resolver_acoso(r, 1, Defensa::Base, &mut Rng::nuevo(semilla)).gana_el_bandido as u32;
            n3 += resolver_acoso(r, 3, Defensa::Base, &mut Rng::nuevo(semilla)).gana_el_bandido as u32;
        }
        assert_eq!((n1, n3), (0, 1000), "20 × 1,15 = 23 < 26 < 120 × 0,85");
        let a = resolver_acoso(r, 2, Defensa::Base, &mut Rng::nuevo(1));
        assert_eq!(a.defensa, 26_000);
    }

    #[test]
    fn la_escolta_pierde_el_25_si_cae_y_el_5_si_aguanta_y_el_ejercito_nada() {
        let r = r();
        let mut cae = [escuadra(1, 5, Tropa::MiliciaLanceros)];
        let a = resolver_acoso(r, 3, Defensa::Escolta(&mut cae), &mut Rng::nuevo(1));
        assert!(a.gana_el_bandido);
        assert_eq!(a.bajas_escolta[0].despues, 25 - 6);
        assert_eq!((cae[0].cantidad, cae[0].experiencia), (0, 500), "queda a 0 y vuelve a su héroe, con su experiencia");

        let mut aguanta = [escuadra(2, 5, Tropa::ArquerosCompuesto), escuadra(3, 5, Tropa::ArquerosCompuesto)];
        let a = resolver_acoso(r, 1, Defensa::Escolta(&mut aguanta), &mut Rng::nuevo(1));
        assert!(!a.gana_el_bandido);
        assert_eq!(a.bajas_escolta.iter().map(|b| b.antes - b.despues).collect::<Vec<_>>(), vec![1, 1]);
        assert_eq!(aguanta[0].experiencia, 1_000);

        let ejercito = [escuadra(4, 6, Tropa::Shardana)];
        let a = resolver_acoso(r, 3, Defensa::Ejercito(&ejercito), &mut Rng::nuevo(1));
        assert!(a.bajas_escolta.is_empty() && ejercito[0].cantidad == r.militar.tropas.tropa(Tropa::Shardana).unidades);
    }

    #[test]
    fn el_acoso_consume_una_tirada() {
        let r = r();
        let mut rng = Rng::nuevo(8);
        let mut espejo = Rng::nuevo(8);
        espejo.siguiente();
        resolver_acoso(r, 2, Defensa::Base, &mut rng);
        assert_eq!(rng.estado(), espejo.estado());
    }

    #[test]
    fn un_heroe_solo_no_destruye_ni_el_campamento_mas_flojo() {
        let r = r();
        for semilla in 0..1000 {
            let a = resolver_campamento(r, Bando { escuadras: &mut [], heroes: &[1] }, 1, &mut Rng::nuevo(semilla)).unwrap();
            assert!(!a.destruido, "15 × 1,15 = 17,25 < 20");
            assert_eq!(a.heridos, vec![1]);
        }
    }

    #[test]
    fn una_columna_con_tropa_destruye_el_campamento_y_sufre_el_cinco_por_ciento() {
        let r = r();
        let mut col = [escuadra(1, 1, Tropa::ArquerosCompuesto)]; // 181,8 > 120 × 1,0
        let a = resolver_campamento(r, Bando { escuadras: &mut col, heroes: &[1] }, 3, &mut Rng::nuevo(2)).unwrap();
        assert!(a.destruido && a.heridos.is_empty());
        assert_eq!((a.poder_campamento, a.bajas[0].antes - a.bajas[0].despues), (120_000, 1), "round(12 × 0,05) = 1");
        assert_eq!(col[0].experiencia, 1_000);
    }

    #[test]
    fn si_el_campamento_aguanta_las_bajas_son_del_25_y_los_heroes_se_hieren() {
        let r = r();
        let mut col = [escuadra(1, 1, Tropa::MiliciaLanceros)]; // 50,5 < 120 × 0,85
        let a = resolver_campamento(r, Bando { escuadras: &mut col, heroes: &[1, 2] }, 3, &mut Rng::nuevo(2)).unwrap();
        assert!(!a.destruido);
        assert_eq!((a.bajas[0].despues, a.heridos), (25 - 6, vec![1, 2]));
        assert_eq!(col[0].experiencia, 500);
    }

    #[test]
    fn un_atacante_vacio_o_un_campamento_no_consume_tirada() {
        let r = r();
        let mut rng = Rng::nuevo(1);
        let antes = rng.estado();
        assert_eq!(resolver_campamento(r, Bando { escuadras: &mut [], heroes: &[] }, 1, &mut rng).unwrap_err(), CombateInvalido::BandoVacio(Lado::Atacante));
        assert_eq!(rng.estado(), antes);
    }

    #[test]
    fn el_botin_es_completo_ocho_veces_y_luego_decrece_hasta_cero_en_la_catorce() {
        let r = r();
        let mut hist = vec![];
        let oro: Vec<Mil> = (0..15).map(|i| botin_de_bandidos(r, &mut hist, 3, i * 1000)).collect();
        assert_eq!(oro[..8], [36_000; 8]);
        assert_eq!(oro[8..14], [30_000, 25_000, 19_000, 14_000, 9_000, 0], "n = 9‥14: 1 − 0,15 × (n − 8) en unidades enteras hacia abajo, y desde la 14.ª, solo experiencia");
        assert_eq!(oro[14], 0);
        assert_eq!(hist.len(), 15);
    }

    #[test]
    fn el_historial_solo_cuenta_las_ultimas_24_horas() {
        let r = r();
        let mut hist = vec![];
        for i in 0..14 {
            botin_de_bandidos(r, &mut hist, 1, i);
        }
        assert_eq!(botin_de_bandidos(r, &mut hist, 1, 0 + 24 * 60 * MS_POR_MINUTO + 14), 30_000, "pasadas 24 h todo vuelve a ser completo");
        assert_eq!(hist.len(), 1, "se podan los antiguos");
        // Justo en el borde de la ventana ya no cuenta (t > ahora − ventana).
        let mut h = vec![0];
        botin_de_bandidos(r, &mut h, 1, 24 * 60 * MS_POR_MINUTO);
        assert_eq!(h, vec![24 * 60 * MS_POR_MINUTO]);
    }

    #[test]
    fn cada_nivel_da_su_oro() {
        let r = r();
        assert_eq!([1, 2, 3].map(|n| botin_de_bandidos(r, &mut vec![], n, 0)), [30_000, 33_000, 36_000]);
    }
}
