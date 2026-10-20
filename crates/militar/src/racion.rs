//! Comer en campaña (Doc 5.4 y 5.13, Movimiento_Ejercitos §6): la ración de una columna o de una guarnición, los víveres como **ritmo** y
//! cómo el hambre mueve la moral de las escuadras. Una sola regla del hambre para guarnición y campaña: solo cambia de qué despensa se come.
//!
//! La cantidad de víveres en cada instante es `víveres₀ − ritmo × (t − t₀)`. El núcleo programa el evento «víveres a 0» y lo reprograma
//! cuando cambia el ritmo (cambia el estado de la columna, muere tropa, se une o separa alguien, se reposta).

use crate::datos::Reglas;
use crate::escuadra::Escuadra;
use crate::tipos::Carga;
use asentamiento::datos::por;
use asentamiento::Recurso;
use base::{Instante, Mil, Ritmo};

/// Si la columna camina o está acampada: acampada come una décima parte de lo que come en marcha (nunca cero). Apartada por una batalla de Unity
/// (Reloj_Y_Eventos §6) no come: nada de lo que dura la partida se gasta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estado {
    Marchando,
    Estacionada,
    Apartada,
}

/// Soldados en pie de estas escuadras (los que comen y los que cuentan para la visión).
pub fn soldados<'a>(escuadras: impl IntoIterator<Item = &'a Escuadra>) -> i64 {
    escuadras.into_iter().map(|e| e.cantidad.max(0)).sum()
}

/// Trigo por minuto de la guarnición (o del campamento) de una plaza: ración entera, sin participantes.
pub fn racion_guarnicion(r: &Reglas, soldados: i64) -> Mil {
    soldados * r.militar.racion_por_soldado
}

/// Trigo por minuto de una columna: `(soldados × ración + participantes × consumoPorParticipante) × factor`, con el factor de marcha
/// (¼) y, acampada, además el de estacionamiento (÷10); apartada por una batalla, nada. Los héroes comen aunque no lleven tropa: un viajero no viaja gratis.
pub fn racion_columna(r: &Reglas, soldados: i64, participantes: usize, estado: Estado) -> Mil {
    let base = soldados * r.militar.racion_por_soldado + participantes as i64 * r.movimiento.consumo_por_participante;
    let factor = match estado {
        Estado::Marchando => r.logistica.factor_marcha,
        Estado::Estacionada => por(r.logistica.factor_marcha, r.logistica.factor_estacionado),
        Estado::Apartada => return 0,
    };
    por(base, factor)
}

/// Capacidad de los víveres de una columna: la suma de la de sus héroes.
pub fn capacidad_viveres(r: &Reglas, participantes: usize) -> Mil {
    participantes as i64 * r.logistica.capacidad_viveres
}

/// Unos víveres con `valor` (topado a la capacidad) desde `t`, sin consumo todavía.
pub fn viveres(r: &Reglas, participantes: usize, valor: Mil, t: Instante) -> Ritmo {
    let cap = capacidad_viveres(r, participantes);
    Ritmo { maximo: Some(cap), minimo: Some(0), ..Ritmo::constante(valor.clamp(0, cap), t) }
}

/// Cambia el consumo desde `t` sin saltos en la cantidad.
pub fn anotar_consumo(v: &mut Ritmo, t: Instante, consumo_por_minuto: Mil) {
    *v = v.reanclado(t, -consumo_por_minuto);
}

/// Cambia la capacidad desde `t` (se une o se separa alguien): la cantidad en `t` se conserva, topada por la nueva capacidad.
pub fn con_capacidad(v: &mut Ritmo, t: Instante, capacidad: Mil) {
    let ahora = v.valor_en(t);
    *v = Ritmo { valor_en_t0: ahora.min(capacidad), t0: t, maximo: Some(capacidad), ..*v };
}

/// Primer instante `>= desde` en que los víveres llegan a 0, si van a llegar (el evento «víveres a 0»).
pub fn se_acaban_en(v: &Ritmo, desde: Instante) -> Option<Instante> {
    v.instante_de_valor(0, desde)
}

/// ¿Hay de qué comer en `t`?
pub fn hay_viveres(v: &Ritmo, t: Instante) -> bool {
    v.valor_en(t) > 0
}

/// Llena los víveres desde `t` con lo que ofrece la fuente (`ofrecido`, ya descontada su reserva): `min(hueco, ofrecido)`. Devuelve lo que
/// se tomó de la fuente (al salir, al repostar, con la ración de un campamento).
pub fn llenar(v: &mut Ritmo, t: Instante, ofrecido: Mil) -> Mil {
    let cap = v.maximo.unwrap_or(0);
    let toma = (cap - v.valor_en(t)).max(0).min(ofrecido.max(0));
    *v = Ritmo { valor_en_t0: v.valor_en(t) + toma, t0: t, ..*v };
    toma
}

/// `pasarAViveres`: del carro a los víveres, nunca al revés: `min(pedido, trigo del carro, hueco)`. Devuelve lo movido.
pub fn pasar_a_viveres(carga: &mut Carga, v: &mut Ritmo, t: Instante, pedido: Mil) -> Mil {
    let en_el_carro = carga.get(&Recurso::Trigo).copied().unwrap_or(0);
    let movido = llenar(v, t, pedido.min(en_el_carro));
    if movido > 0 {
        let resto = en_el_carro - movido;
        if resto > 0 {
            carga.insert(Recurso::Trigo, resto);
        } else {
            carga.remove(&Recurso::Trigo);
        }
    }
    movido
}

/// Anota en cada escuadra, desde `t`, si tiene ración: la moral sube con ella y baja sin ella. Se llama cuando cambia el hambre.
pub fn alimentar(r: &Reglas, escuadras: &mut [Escuadra], t: Instante, con_racion: bool) {
    escuadras.iter_mut().for_each(|e| e.con_racion(r, t, con_racion));
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::catalogo::Tropa;
    use base::MS_POR_MINUTO as MIN;

    fn r() -> Reglas {
        Reglas::cargar()
    }

    #[test]
    fn la_racion_de_marcha_es_un_cuarto_y_acampada_una_decima_de_eso() {
        let r = r();
        // 100 soldados y 2 héroes a ración entera: 100 × 0,15 + 2 × 0,5 = 16.
        assert_eq!(racion_columna(&r, 100, 2, Estado::Marchando), 4_000, "un cuarto de 16");
        assert_eq!(racion_columna(&r, 100, 2, Estado::Estacionada), 400, "una décima de eso, nunca cero");
        assert_eq!(racion_guarnicion(&r, 100), 15_000, "la guarnición come la ración entera");
        assert_eq!(racion_columna(&r, 0, 1, Estado::Estacionada), 13, "un héroe solo acampado: 0,5 × 0,025");
        assert_eq!(racion_columna(&r, 100, 2, Estado::Apartada), 0, "apartada por una batalla, no come");
    }

    #[test]
    fn los_ejemplos_numericos_del_canon_5_11_y_5_13() {
        let r = r();
        // Tropa prestada completa de un campamento (85 soldados) más su héroe: ≈ 3,3 de trigo por minuto en marcha.
        let prestada = racion_columna(&r, 85, 1, Estado::Marchando);
        assert_eq!(prestada, 3_313);
        // 1.333 unidades a velocidad ligera (30) son 44 minutos y 146 de trigo: los 350 de víveres sobran.
        let minutos = 1_333_000 / 30_000;
        assert_eq!(minutos, 44);
        assert_eq!(prestada * minutos / 1_000, 145, "≈ 146 en el trayecto de 44,4 min");
        // La hueste de leva (14 escuadras de 25 = 350 hombres) vacía un carro de 500 en unos 38 minutos.
        let hueste = racion_guarnicion(&r, 350) / 4; // en marcha, sin héroe
        assert_eq!(500_000 / hueste, 38);
        // Dos escuadrones de élite (24 hombres) aguantan unas nueve horas con un carro.
        let elite = racion_guarnicion(&r, 24) / 4;
        assert_eq!(500_000 / elite / 60, 9);
    }

    #[test]
    fn la_capacidad_de_los_viveres_es_la_suma_de_la_de_sus_heroes() {
        let r = r();
        assert_eq!(capacidad_viveres(&r, 1), 350_000);
        assert_eq!(capacidad_viveres(&r, 4), 1_400_000);
        let v = viveres(&r, 2, 5_000_000, 0);
        assert_eq!(v.valor_en(0), 700_000, "se topa a la capacidad");
        assert_eq!(v.maximo, Some(700_000));
    }

    #[test]
    fn el_consumo_vacia_los_viveres_y_el_evento_cae_en_el_instante_exacto() {
        let r = r();
        let mut v = viveres(&r, 1, 350_000, 0);
        assert_eq!(se_acaban_en(&v, 0), None, "sin consumo nunca se acaban");
        let consumo = racion_columna(&r, 25, 1, Estado::Marchando); // (3.750 + 500) / 4 = 1.062,5 → 1.063
        assert_eq!(consumo, 1_063);
        anotar_consumo(&mut v, 0, consumo);
        assert_eq!(v.valor_en(10 * MIN), 350_000 - 10_630);
        let fin = se_acaban_en(&v, 0).unwrap();
        assert_eq!(v.valor_en(fin), 0);
        assert!(v.valor_en(fin - 1) > 0, "el instante es el primero con 0 víveres");
        assert!(hay_viveres(&v, fin - 1) && !hay_viveres(&v, fin));
        assert_eq!(v.valor_en(fin + 10 * MIN), 0, "no baja de 0");
    }

    #[test]
    fn cambiar_el_ritmo_no_da_saltos_ni_pierde_lo_consumido() {
        let r = r();
        let mut v = viveres(&r, 1, 350_000, 0);
        anotar_consumo(&mut v, 0, 1_000);
        let a = v.valor_en(100 * MIN);
        assert_eq!(a, 250_000);
        anotar_consumo(&mut v, 100 * MIN, 100); // se acampa
        assert_eq!(v.valor_en(100 * MIN), 250_000);
        assert_eq!(v.valor_en(110 * MIN), 249_000);
    }

    #[test]
    fn llenar_toma_lo_que_cabe_y_no_mas_de_lo_ofrecido() {
        let r = r();
        let mut v = viveres(&r, 1, 100_000, 0);
        assert_eq!(llenar(&mut v, 0, 1_000_000), 250_000, "hasta el tope");
        assert_eq!(v.valor_en(0), 350_000);
        assert_eq!(llenar(&mut v, 0, 5_000), 0, "ya está lleno");
        anotar_consumo(&mut v, 0, 1_000);
        assert_eq!(llenar(&mut v, 10 * MIN, 4_000), 4_000, "solo lo que ofrece la fuente");
        assert_eq!(v.valor_en(10 * MIN), 350_000 - 10_000 + 4_000);
        assert_eq!(llenar(&mut v, 10 * MIN, -3), 0);
    }

    #[test]
    fn pasar_a_viveres_va_del_carro_a_los_viveres_y_nunca_al_reves() {
        let r = r();
        let mut v = viveres(&r, 1, 300_000, 0);
        let mut carga = Carga::new();
        carga.insert(Recurso::Trigo, 80_000);
        carga.insert(Recurso::Madera, 10_000);
        assert_eq!(pasar_a_viveres(&mut carga, &mut v, 0, 30_000), 30_000);
        assert_eq!((v.valor_en(0), carga[&Recurso::Trigo]), (330_000, 50_000));
        assert_eq!(pasar_a_viveres(&mut carga, &mut v, 0, 1_000_000), 20_000, "min(pedido, trigo, hueco): el hueco son 20");
        assert_eq!((v.valor_en(0), carga[&Recurso::Trigo]), (350_000, 30_000));
        assert_eq!(pasar_a_viveres(&mut carga, &mut v, 0, 5_000), 0);
        let mut v2 = viveres(&r, 1, 0, 0);
        assert_eq!(pasar_a_viveres(&mut carga, &mut v2, 0, 1_000_000), 30_000);
        assert!(!carga.contains_key(&Recurso::Trigo), "vaciado el trigo, la clave desaparece");
        assert_eq!(carga[&Recurso::Madera], 10_000, "lo demás no se toca");
    }

    #[test]
    fn la_capacidad_cambia_sin_perder_lo_que_hay_y_topando() {
        let r = r();
        let mut v = viveres(&r, 2, 700_000, 0);
        con_capacidad(&mut v, 0, capacidad_viveres(&r, 3));
        assert_eq!((v.valor_en(0), v.maximo), (700_000, Some(1_050_000)));
        con_capacidad(&mut v, 0, capacidad_viveres(&r, 1));
        assert_eq!((v.valor_en(0), v.maximo), (350_000, Some(350_000)));
    }

    #[test]
    fn el_hambre_mueve_la_moral_de_todas_las_escuadras_a_la_vez() {
        let r = r();
        let mut es = vec![
            Escuadra::nueva(&r, 1, 1, Tropa::Arqueros, 18, None, 0),
            Escuadra::nueva(&r, 2, 1, Tropa::Honderos, 20, None, 0),
        ];
        assert_eq!(soldados(&es), 38);
        alimentar(&r, &mut es, 0, false);
        assert!(es.iter().all(|e| e.moral_en(2 * MIN) == 60_000));
        alimentar(&r, &mut es, 2 * MIN, true);
        assert!(es.iter().all(|e| e.moral_en(4 * MIN) == 70_000), "vuelve a subir cinco por minuto");
        es[1].cantidad = 0;
        assert_eq!(soldados(&es), 18, "una escuadra aniquilada no come");
    }
}
