//! El asalto a una caravana con números (Combate §7.1, Doc 3.10): el servidor decide el **tipo** por la zona donde está la caravana en el
//! instante del ataque, el atacante tira **una** vez contra una cifra de defensa, y las consecuencias para el dueño (carros y carga) salen del
//! tipo. No hay captura del 50 % ni inmunidad: los carros no cambian de manos en el mundo, solo la carga.
//!
//! - **A, disputa de carros**: fuera de la zona del dueño y de sus aliados. Si gana el atacante, la caravana pierde la mitad de sus carros
//!   (hacia arriba) con su animal y la carga que llevaban.
//! - **B, aguante**: dentro de la zona del dueño o de un aliado. La defensa vale ×1,25. Si gana el atacante, el dueño pierde el 70 % de la carga,
//!   recurso a recurso, y también la mitad de los carros; el 30 % se salva siempre.
//!
//! Con Unity (`Batalla_Unity_Ciclo`) la tirada no existe: el servidor manda la modalidad y Unity devuelve los carros perdidos, que entran en
//! [`aplicar_al_dueno`] con las mismas consecuencias.

use crate::datos::Reglas;
use crate::resolucion::{aplicar_bajas, ganador_de, poder_de_bando, poder_total, tirada, Baja, Bando, CombateInvalido, Lado, SIN_DUENO};
use asentamiento::datos::UNO;
use base::{Instante, Mil, Rng};
use comercio::caravana::{Caravana, ErrorCaravana};
use comercio::datos::ReglasFlota;
use militar::escuadra::Escuadra;
use militar::{Carga, EscuadraId, HeroeId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modalidad {
    /// A: disputa de carros.
    Disputa,
    /// B: aguante.
    Aguante,
}

/// Dónde está la caravana respecto a las zonas, en el instante del ataque. El polígono lo mira el núcleo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zona {
    /// Zona de influencia del dueño o de un aliado suyo (alianza o vasallaje).
    DelDuenoOAliado,
    /// Zona de un enemigo, de un aliado de un enemigo o de un neutral.
    Ajena,
    /// Fuera de toda zona.
    Ninguna,
}

pub fn modalidad_de(zona: Zona) -> Modalidad {
    match zona {
        Zona::DelDuenoOAliado => Modalidad::Aguante,
        Zona::Ajena | Zona::Ninguna => Modalidad::Disputa,
    }
}

/// La defensa base de una caravana cuando juega Unity: una escuadra de carreteros sin dueño (`BATALLA.tropaCarreteros`: 13 de milicia de
/// lanceros, poder 26 en el combate con números).
pub fn carreteros(r: &Reglas, id: EscuadraId, ahora: Instante) -> Escuadra {
    let (tropa, unidades) = r.batalla.carreteros;
    Escuadra::nueva(&r.militar, id, SIN_DUENO, tropa, unidades, None, ahora)
}

/// Con qué se defiende una caravana, de más a menos protegida (Doc 3.10).
pub enum Defensa<'a> {
    /// Nada: los carreteros (`defensaBaseCaravana`, 26: un héroe solo no la vence).
    Base,
    /// Escolta sin héroe: escuadras cedidas, manejadas por la IA. Sufren bajas.
    Escolta(&'a mut [Escuadra]),
    /// Adjunta a un ejército: se defiende con su poder. En el acoso de los bandidos el ejército no sufre bajas (Generacion_Del_Mundo §13).
    Ejercito(&'a [Escuadra]),
}

/// La cifra de defensa (sin la ventaja del tipo): la base o el poder de las escuadras con cohesión.
pub fn defensa_de(r: &Reglas, d: &Defensa) -> Mil {
    match d {
        Defensa::Base => r.poder.defensa_base_caravana,
        Defensa::Escolta(e) => poder_total(r, e, true),
        Defensa::Ejercito(e) => poder_total(r, e, true),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultadoAsalto {
    pub gana_atacante: bool,
    /// Poder del atacante con su tirada, y la defensa con la ventaja del tipo.
    pub poder_atacante: Mil,
    pub defensa: Mil,
    pub bajas_atacante: Vec<Baja>,
    /// Las de la escolta sin héroe; vacío si no la hay.
    pub bajas_escolta: Vec<Baja>,
    /// Si gana la caravana, los héroes del atacante quedan heridos.
    pub heridos: Vec<HeroeId>,
}

/// El combate numérico contra una caravana: **una** tirada del atacante contra `defensa × ventajaDeTipo` (×1,25 en B, ×1 en A). Gana el
/// atacante solo si saca más. Bajas fijas: 5 % al que gana y 25 % al que pierde, escolta incluida; si cae la caravana, su escolta queda a 0 y
/// vuelve al campamento de su héroe (Doc 5.15.4).
pub fn resolver_asalto(r: &Reglas, atacante: Bando, defensa: Defensa, modalidad: Modalidad, rng: &mut Rng) -> Result<ResultadoAsalto, CombateInvalido> {
    if atacante.escuadras.iter().all(|e| !e.con_soldados()) && atacante.heroes.is_empty() {
        return Err(CombateInvalido::BandoVacio(Lado::Atacante));
    }
    let ventaja = if modalidad == Modalidad::Aguante { r.numericas.ventaja_aguante } else { UNO };
    let defensa_con_ventaja = (defensa_de(r, &defensa) as i128 * ventaja as i128 / UNO as i128) as Mil;
    let poder = (poder_de_bando(r, atacante.escuadras, atacante.heroes.len()) as i128 * tirada(r, rng) as i128 / UNO as i128) as Mil;
    let gana = ganador_de(poder, defensa_con_ventaja) == Lado::Atacante;

    // Las bajas fijas no producen chatarra aquí: el botín del asalto es la carga (Combate §7.1).
    let mut sin_uso = Carga::new();
    let n = &r.numericas;
    let (f_atacante, f_escolta) = if gana { (n.bajas_fijas_ganador, n.bajas_fijas_perdedor) } else { (n.bajas_fijas_perdedor, n.bajas_fijas_ganador) };
    let bajas_atacante = aplicar_bajas(r, atacante.escuadras, f_atacante, gana, &mut sin_uso);
    let bajas_escolta = match defensa {
        Defensa::Escolta(e) => {
            let bajas = aplicar_bajas(r, e, f_escolta, !gana, &mut sin_uso);
            if gana {
                e.iter_mut().for_each(|x| x.cantidad = 0);
            }
            bajas
        }
        Defensa::Base | Defensa::Ejercito(_) => vec![],
    };
    Ok(ResultadoAsalto { gana_atacante: gana, poder_atacante: poder, defensa: defensa_con_ventaja, bajas_atacante, bajas_escolta, heridos: if gana { vec![] } else { atacante.heroes.to_vec() } })
}

/// Cuántos carros pierde una caravana de `n`: la mitad, hacia arriba.
pub fn carros_a_perder(r: &Reglas, n: usize) -> usize {
    (n as i128 * r.numericas.carros_perdidos as i128 + UNO as i128 - 1).div_euclid(UNO as i128) as usize
}

/// Lo que perdió el dueño.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Perdida {
    pub carros_perdidos: usize,
    /// La carga perdida: lo que el atacante puede llevarse, hasta lo que le quepa (el resto se destruye).
    pub carga_perdida: Carga,
    /// No quedan carros: la caravana se elimina (y su escolta vuelve al campamento). Nunca en un aguante.
    pub destruida: bool,
}

/// Las consecuencias para el dueño cuando **gana el atacante**. `carros` son los índices de los carros perdidos si los dice Unity; sin ellos
/// (combate con números) se pierden los últimos `carros_a_perder` de la lista, los añadidos más tarde (Combate §7.1). En un **aguante nunca se
/// pierde el último carro**: si se perderían todos, el primero de la lista se queda y lleva el 30 % que se salva.
pub fn aplicar_al_dueno(r: &Reglas, flota: &ReglasFlota, caravana: &mut Caravana, modalidad: Modalidad, carros: Option<&[usize]>) -> Result<Perdida, ErrorCaravana> {
    let n = caravana.carros.len();
    let mut indices: Vec<usize> = match carros {
        Some(c) => c.to_vec(),
        None => (n - carros_a_perder(r, n).min(n)..n).collect(),
    };
    match modalidad {
        Modalidad::Disputa => {
            let p = caravana.aplicar_perdida_de_carros(flota, &indices)?;
            Ok(Perdida { carros_perdidos: p.carros_perdidos, carga_perdida: p.carga_perdida, destruida: p.destruida })
        }
        Modalidad::Aguante => {
            if indices.iter().enumerate().any(|(i, &x)| x >= n || indices[..i].contains(&x)) {
                return Err(ErrorCaravana::CarroInexistente);
            }
            if indices.len() == n {
                indices.retain(|&x| x != 0);
            }
            let carga_perdida = caravana.perder_fraccion_de_carga(r.numericas.carga_perdida_aguante);
            let salvada = caravana.contenido.clone();
            let p = caravana.aplicar_perdida_de_carros(flota, &indices)?;
            // El 30 % se salva siempre: los carros que se van no se llevan nada más.
            caravana.contenido = salvada;
            Ok(Perdida { carros_perdidos: p.carros_perdidos, carga_perdida, destruida: false })
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::resolucion::apoyo::*;
    use asentamiento::Recurso::{Madera, Piedra, Trigo};
    use base::Punto;
    use comercio::caravana::{Animal, Carro, TipoCarro};
    use militar::Tropa;

    fn caravana(carros: usize, carga: &[(asentamiento::Recurso, Mil)]) -> Caravana {
        let mut c = Caravana::nueva(1, 1, Punto::default());
        c.carros = vec![Carro { tipo: TipoCarro::Basico, animal: Some(Animal::Buey) }; carros];
        c.contenido = carga.iter().copied().collect();
        c
    }

    fn flota() -> ReglasFlota {
        comercio::Reglas::cargar().flota
    }

    #[test]
    fn la_zona_decide_el_tipo() {
        assert_eq!(modalidad_de(Zona::DelDuenoOAliado), Modalidad::Aguante);
        assert_eq!(modalidad_de(Zona::Ajena), Modalidad::Disputa);
        assert_eq!(modalidad_de(Zona::Ninguna), Modalidad::Disputa, "ya no hay refugio: fuera de toda zona también se disputa");
    }

    #[test]
    fn un_heroe_solo_no_roba_caravanas_ni_con_la_mejor_tirada() {
        let r = r();
        for m in [Modalidad::Disputa, Modalidad::Aguante] {
            for semilla in 0..2000 {
                let res = resolver_asalto(r, Bando { escuadras: &mut [], heroes: &[1] }, Defensa::Base, m, &mut Rng::nuevo(semilla)).unwrap();
                assert!(!res.gana_atacante, "15 × 1,15 = 17,25 < 26 (Doc 3.10)");
                assert_eq!(res.heridos, vec![1]);
            }
        }
    }

    #[test]
    fn la_defensa_base_son_26_y_el_aguante_la_multiplica_por_125() {
        let r = r();
        let a = resolver_asalto(r, Bando { escuadras: &mut [], heroes: &[1] }, Defensa::Base, Modalidad::Disputa, &mut Rng::nuevo(1)).unwrap();
        let b = resolver_asalto(r, Bando { escuadras: &mut [], heroes: &[1] }, Defensa::Base, Modalidad::Aguante, &mut Rng::nuevo(1)).unwrap();
        assert_eq!((a.defensa, b.defensa), (26_000, 32_500));
        assert_eq!(a.poder_atacante, b.poder_atacante, "la misma tirada");
    }

    #[test]
    fn consume_una_sola_tirada() {
        let r = r();
        let mut rng = Rng::nuevo(5);
        let mut espejo = Rng::nuevo(5);
        espejo.siguiente();
        resolver_asalto(r, Bando { escuadras: &mut [], heroes: &[1] }, Defensa::Base, Modalidad::Disputa, &mut rng).unwrap();
        assert_eq!(rng.estado(), espejo.estado());
    }

    #[test]
    fn un_atacante_vacio_es_un_error_y_no_toca_el_generador() {
        let r = r();
        let mut rng = Rng::nuevo(5);
        let antes = rng.estado();
        let mut muerta = [con_cantidad(escuadra(1, 1, Tropa::Hequetai), 0)];
        let e = resolver_asalto(r, Bando { escuadras: &mut muerta, heroes: &[] }, Defensa::Base, Modalidad::Disputa, &mut rng).unwrap_err();
        assert_eq!(e, CombateInvalido::BandoVacio(Lado::Atacante));
        assert_eq!(rng.estado(), antes);
    }

    #[test]
    fn una_escuadra_de_verdad_gana_a_la_defensa_base_y_sufre_el_cinco_por_ciento() {
        let r = r();
        let mut a = [escuadra(1, 1, Tropa::MiliciaLanceros)]; // 50,5 > 26 × 1,25 × 1,15 en el peor caso
        let res = resolver_asalto(r, Bando { escuadras: &mut a, heroes: &[1] }, Defensa::Base, Modalidad::Aguante, &mut Rng::nuevo(3)).unwrap();
        assert!(res.gana_atacante);
        assert_eq!(res.bajas_atacante, vec![Baja { escuadra: 1, tropa: Tropa::MiliciaLanceros, antes: 25, despues: 24 }], "round(25 × 0,05) = 1");
        assert_eq!(a[0].experiencia, 1_000);
        assert!(res.heridos.is_empty() && res.bajas_escolta.is_empty());
    }

    #[test]
    fn si_cae_la_escolta_queda_a_cero_con_la_experiencia_de_perder() {
        let r = r();
        let mut a = [escuadra(1, 1, Tropa::ArquerosCompuesto), escuadra(2, 1, Tropa::ArquerosCompuesto)];
        let mut escolta = [escuadra(10, 7, Tropa::MiliciaLanceros)];
        let res = resolver_asalto(r, Bando { escuadras: &mut a, heroes: &[] }, Defensa::Escolta(&mut escolta), Modalidad::Disputa, &mut Rng::nuevo(8)).unwrap();
        assert!(res.gana_atacante);
        assert_eq!(res.bajas_escolta[0], Baja { escuadra: 10, tropa: Tropa::MiliciaLanceros, antes: 25, despues: 25 - 6 }, "25 % de 25 = 6,25 → 6");
        assert_eq!((escolta[0].cantidad, escolta[0].experiencia), (0, 500), "a cero, pero la escuadra persiste con su experiencia");
    }

    #[test]
    fn si_aguanta_la_escolta_pierde_el_cinco_por_ciento_y_el_atacante_el_25_y_sus_heroes_se_hieren() {
        let r = r();
        let mut a = [escuadra(1, 1, Tropa::MiliciaLanceros)];
        let mut escolta = [escuadra(10, 7, Tropa::ArquerosCompuesto), escuadra(11, 7, Tropa::ArquerosCompuesto)];
        let res = resolver_asalto(r, Bando { escuadras: &mut a, heroes: &[1, 2] }, Defensa::Escolta(&mut escolta), Modalidad::Disputa, &mut Rng::nuevo(8)).unwrap();
        assert!(!res.gana_atacante);
        assert_eq!(res.bajas_atacante[0].despues, 25 - 6);
        assert_eq!(res.bajas_escolta[0].despues, 12 - 1, "round(12 × 0,05) = 1");
        assert_eq!(res.heridos, vec![1, 2]);
        assert_eq!((escolta[0].experiencia, a[0].experiencia), (1_000, 500));
        // La defensa de la escolta lleva cohesión: 2 escuadras ⇒ ×1,1.
        assert_eq!(res.defensa, poder_total(r, &[escuadra(10, 7, Tropa::ArquerosCompuesto), escuadra(11, 7, Tropa::ArquerosCompuesto)], true));
    }

    #[test]
    fn el_ejercito_escolta_defiende_con_su_poder_y_no_sufre_nada() {
        let r = r();
        let ejercito = [escuadra(20, 9, Tropa::Shardana)];
        let copia = ejercito.clone();
        let res = resolver_asalto(r, Bando { escuadras: &mut [], heroes: &[1] }, Defensa::Ejercito(&ejercito), Modalidad::Disputa, &mut Rng::nuevo(2)).unwrap();
        assert!(!res.gana_atacante && res.bajas_escolta.is_empty());
        assert_eq!(ejercito, copia);
        assert_eq!(res.defensa, copia[0].poder(&r.militar));
    }

    #[test]
    fn la_disputa_quita_la_mitad_de_los_carros_hacia_arriba_con_su_carga() {
        let r = r();
        let f = flota();
        for (n, quedan) in [(1usize, 0usize), (2, 1), (3, 1), (4, 2), (5, 2)] {
            let mut c = caravana(n, &[(Madera, 1_200), (Trigo, 7)]);
            let p = aplicar_al_dueno(r, &f, &mut c, Modalidad::Disputa, None).unwrap();
            assert_eq!(c.carros.len(), quedan, "{n} carros");
            assert_eq!(p.carros_perdidos, n - quedan);
            assert_eq!(p.destruida, quedan == 0);
            // La carga perdida es la de los carros perdidos (proporcional a su capacidad, y la restante se conserva).
            let restante: Mil = c.contenido.values().sum();
            assert_eq!(restante + p.carga_perdida.values().sum::<Mil>(), 1_207);
            if quedan == 0 {
                assert!(c.contenido.is_empty());
            }
        }
        let mut c = caravana(4, &[(Madera, 1_000)]);
        let p = aplicar_al_dueno(r, &f, &mut c, Modalidad::Disputa, None).unwrap();
        assert_eq!(p.carga_perdida.get(&Madera), Some(&500), "la mitad de los carros iguales lleva la mitad de la carga");
    }

    #[test]
    fn el_aguante_quita_el_70_por_ciento_de_la_carga_y_el_30_se_salva_siempre() {
        let r = r();
        let f = flota();
        let mut c = caravana(4, &[(Madera, 1_000), (Piedra, 99)]);
        let p = aplicar_al_dueno(r, &f, &mut c, Modalidad::Aguante, None).unwrap();
        assert_eq!(p.carga_perdida.get(&Madera), Some(&700));
        assert_eq!(p.carga_perdida.get(&Piedra), Some(&69), "70 % hacia abajo, recurso a recurso");
        assert_eq!(c.carros.len(), 2, "también la mitad de los carros");
        assert_eq!(c.contenido.get(&Madera), Some(&300), "el 30 % no se lo llevan los carros que se pierden");
        assert_eq!(c.contenido.get(&Piedra), Some(&30));
        assert!(!p.destruida);
    }

    #[test]
    fn en_un_aguante_nunca_se_pierde_el_ultimo_carro_y_el_30_por_ciento_sigue_en_el() {
        let r = r();
        let mut c = caravana(1, &[(Madera, 1_000)]);
        let p = aplicar_al_dueno(r, &flota(), &mut c, Modalidad::Aguante, None).unwrap();
        assert!(!p.destruida && p.carros_perdidos == 0, "con un carro, la mitad hacia arriba sería el único");
        assert_eq!((p.carga_perdida.get(&Madera), c.carros.len(), c.contenido.get(&Madera)), (Some(&700), 1, Some(&300)));
        // Unity tampoco puede quitarlos todos: se queda el primero de la lista.
        let mut c = caravana(3, &[(Madera, 1_000)]);
        c.carros[0].tipo = TipoCarro::Reforzado;
        let p = aplicar_al_dueno(r, &flota(), &mut c, Modalidad::Aguante, Some(&[2, 0, 1])).unwrap();
        assert_eq!((p.carros_perdidos, p.destruida), (2, false));
        assert_eq!((c.carros[0].tipo, c.contenido.get(&Madera)), (TipoCarro::Reforzado, Some(&300)));
    }

    #[test]
    fn con_unity_los_carros_perdidos_los_dice_unity_y_uno_inexistente_se_rechaza() {
        let r = r();
        let f = flota();
        let mut c = caravana(4, &[(Madera, 1_000)]);
        let p = aplicar_al_dueno(r, &f, &mut c, Modalidad::Disputa, Some(&[0])).unwrap();
        assert_eq!((p.carros_perdidos, c.carros.len()), (1, 3));
        assert_eq!(p.carga_perdida.get(&Madera), Some(&250));
        let antes = c.clone();
        assert_eq!(aplicar_al_dueno(r, &f, &mut c, Modalidad::Disputa, Some(&[9])).unwrap_err(), ErrorCaravana::CarroInexistente);
        assert_eq!(aplicar_al_dueno(r, &f, &mut c, Modalidad::Aguante, Some(&[9])).unwrap_err(), ErrorCaravana::CarroInexistente);
        assert_eq!(aplicar_al_dueno(r, &f, &mut c, Modalidad::Aguante, Some(&[1, 1])).unwrap_err(), ErrorCaravana::CarroInexistente);
        assert_eq!(c, antes, "un resultado inválido no toca la caravana");
    }

    #[test]
    fn los_carreteros_de_unity_pesan_lo_que_la_defensa_base() {
        let r = r();
        let c = carreteros(r, 77, 0);
        assert_eq!((c.tropa, c.cantidad, c.heroe, c.id), (Tropa::MiliciaLanceros, 13, SIN_DUENO, 77));
        assert_eq!(13 * r.militar.tropas.tropa(c.tropa).poder_base * 1_000, r.poder.defensa_base_caravana, "13 × 2 = 26 (Doc 3.10)");
    }

    #[test]
    fn carros_a_perder_es_la_mitad_hacia_arriba() {
        let r = r();
        assert_eq!([0, 1, 2, 3, 4, 5, 9].map(|n| carros_a_perder(r, n)), [0, 1, 1, 2, 2, 3, 5]);
    }
}
