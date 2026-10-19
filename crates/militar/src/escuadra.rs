//! La escuadra (Heroe_Y_Tropa §2.3 y §10): la instancia de una tropa que un héroe posee. Persiste a `cantidad` 0 (aniquilada) y se repone;
//! su nivel sube con experiencia; su moral es un **ritmo** (sube con ración y baja sin ella) y a 0 deserta.

use crate::catalogo::{Origen, Tropa};
use crate::datos::Reglas;
use crate::tipos::{AsentamientoId, BatallaId, CampamentoId, CaravanaId, ColumnaId, EscuadraId, HeroeId};
use asentamiento::datos::UNO;
use base::{Instante, Mil, Ritmo};

/// Moral máxima (100 puntos, en milésimas).
pub const MORAL_MAXIMA: Mil = 100_000;
/// Cada cuánto deserta una escuadra con la moral a 0 y sin ración (un minuto de mundo, como en el motor TypeScript).
pub const INTERVALO_DESERCION: i64 = base::MS_POR_MINUTO;

/// Dónde está una escuadra (Heroe_Y_Tropa §2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contenedor {
    /// En el campamento de su héroe (a donde reside).
    Campamento,
    /// En una columna.
    Ejercito(ColumnaId),
    /// Cedida como escolta de una caravana.
    Escolta(CaravanaId),
    /// Fuera del mundo, con su héroe desconectado.
    Fuera,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Escuadra {
    pub id: EscuadraId,
    pub heroe: HeroeId,
    pub tropa: Tropa,
    /// La clase de población de la que salió.
    pub origen: Origen,
    /// La plaza donde se reclutó: a ella se atribuyen los avisos de deserción. `None` en la tropa prestada.
    pub plaza_origen: Option<AsentamientoId>,
    /// Soldados. Persiste a 0 y se repone reclutando; nunca desaparece.
    pub cantidad: i64,
    /// 1 a `nivelMaximoEscuadra`.
    pub nivel: u8,
    /// Experiencia **acumulada**, en milésimas de punto.
    pub experiencia: Mil,
    /// 0 a 100 (milésimas). A 0 hay deserción continua.
    pub moral: Ritmo,
    pub contenedor: Contenedor,
    /// En la guarnición de la plaza donde reside su héroe (solo con `Contenedor::Campamento`).
    pub en_guarnicion: bool,
    /// Candado mientras está reservada para una batalla de Unity.
    pub reserva_batalla: Option<BatallaId>,
    /// Tropa prestada por un campamento de mercenarios: no es del héroe, no gana experiencia y se pierde al dejar de residir allí.
    pub prestada: Option<CampamentoId>,
}

/// Nivel de una escuadra con esta experiencia acumulada: sube cada vez que llena lo que pide su nivel, hasta `nivelMaximoEscuadra`.
pub fn nivel_de_escuadra(r: &Reglas, experiencia: Mil) -> u8 {
    let (mut nivel, mut resto) = (1u8, experiencia);
    for &pide in &r.militar.experiencia_para_subir {
        if nivel >= r.militar.nivel_maximo || resto < pide {
            break;
        }
        resto -= pide;
        nivel += 1;
    }
    nivel
}

/// Soldados que desertan en un paso de hambre: `ceil(cantidad × fracción)`, sin pasar de los que hay.
pub fn desertores(r: &Reglas, cantidad: i64) -> i64 {
    ((cantidad * r.militar.desercion + UNO - 1) / UNO).clamp(0, cantidad.max(0))
}

impl Escuadra {
    /// Una escuadra recién reclutada: nivel 1, sin experiencia y con la moral al máximo, en el campamento de su héroe.
    pub fn nueva(r: &Reglas, id: EscuadraId, heroe: HeroeId, tropa: Tropa, cantidad: i64, plaza_origen: Option<AsentamientoId>, ahora: Instante) -> Self {
        Self {
            id,
            heroe,
            tropa,
            origen: r.tropas.tropa(tropa).origen(),
            plaza_origen,
            cantidad,
            nivel: 1,
            experiencia: 0,
            moral: Ritmo { maximo: Some(MORAL_MAXIMA), minimo: Some(0), ..Ritmo::constante(MORAL_MAXIMA, ahora) },
            contenedor: Contenedor::Campamento,
            en_guarnicion: false,
            reserva_batalla: None,
            prestada: None,
        }
    }

    /// Una escuadra prestada por un campamento: completa (`unidades` de su tropa) y sin plaza de origen.
    pub fn prestada(r: &Reglas, id: EscuadraId, heroe: HeroeId, tropa: Tropa, campamento: CampamentoId, ahora: Instante) -> Self {
        Self { prestada: Some(campamento), ..Self::nueva(r, id, heroe, tropa, r.tropas.tropa(tropa).unidades, None, ahora) }
    }

    /// ¿Queda alguien en pie? Una escuadra aniquilada no frena a nadie, no come y no combate.
    pub fn con_soldados(&self) -> bool {
        self.cantidad > 0
    }

    pub fn moral_en(&self, t: Instante) -> Mil {
        self.moral.valor_en(t)
    }

    /// Anota desde `t` si la escuadra tiene ración (la moral sube `regeneracionMoralPorMinuto`) o no (baja `degradacionMoralSinRacion`). Lo
    /// llama quien cambia el hambre: se acabaron los víveres, se repostó, el trigo de la plaza llegó a 0 o volvió.
    pub fn con_racion(&mut self, r: &Reglas, t: Instante, alimentada: bool) {
        let por_minuto = if alimentada { r.militar.regeneracion_moral } else { -r.militar.degradacion_moral };
        self.moral = self.moral.reanclado(t, por_minuto);
    }

    /// Primer instante `>= desde` en que la moral llega a 0 (donde empieza la deserción), si va a llegar.
    pub fn moral_a_cero_en(&self, desde: Instante) -> Option<Instante> {
        self.moral.instante_de_valor(0, desde)
    }

    /// Un paso de deserción: si la moral está a 0 en `t` y quedan soldados, desertan `ceil(cantidad × fracción)`. Devuelve cuántos.
    pub fn desertar(&mut self, r: &Reglas, t: Instante) -> i64 {
        if self.moral_en(t) > 0 {
            return 0;
        }
        let n = desertores(r, self.cantidad);
        self.cantidad -= n;
        n
    }

    /// Suma experiencia y sube el nivel que toque. La tropa prestada no gana (D45).
    pub fn con_experiencia(&mut self, r: &Reglas, ganada: Mil) {
        if self.prestada.is_some() {
            return;
        }
        self.experiencia += ganada;
        self.nivel = nivel_de_escuadra(r, self.experiencia);
    }

    /// Poder de combate, en milésimas: `poderBase × cantidad × (1 + nivel × bonus)`.
    pub fn poder(&self, r: &Reglas) -> Mil {
        let bonus = UNO + self.nivel as i64 * r.militar.bonus_poder_por_nivel;
        (r.tropas.tropa(self.tropa).poder_base as i128 * self.cantidad as i128 * bonus as i128 / (UNO / 1_000) as i128) as Mil
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn r() -> Reglas {
        Reglas::cargar()
    }

    fn escuadra(tropa: Tropa) -> Escuadra {
        let r = r();
        Escuadra::nueva(&r, 1, 10, tropa, r.tropas.tropa(tropa).unidades, Some(7), 0)
    }

    #[test]
    fn nace_en_el_campamento_con_la_moral_al_maximo_y_sin_progreso() {
        let e = escuadra(Tropa::MiliciaLanceros);
        assert_eq!((e.cantidad, e.nivel, e.experiencia, e.moral_en(0)), (25, 1, 0, 100_000));
        assert_eq!((e.contenedor, e.en_guarnicion, e.reserva_batalla, e.prestada), (Contenedor::Campamento, false, None, None));
        assert_eq!((e.origen, e.plaza_origen), (Origen::Pesants, Some(7)));
        assert!(e.con_soldados());
    }

    #[test]
    fn el_nivel_sigue_la_curva_5_10_20_40_y_luego_40_por_nivel() {
        let r = r();
        let n = |puntos: f64| nivel_de_escuadra(&r, (puntos * 1000.0) as Mil);
        assert_eq!((n(0.0), n(4.999), n(5.0), n(14.999), n(15.0), n(35.0), n(75.0)), (1, 1, 2, 2, 3, 4, 5));
        assert_eq!(n(274.999), 9, "falta una milésima para el máximo");
        assert_eq!(n(275.0), 10, "275 de experiencia llevan al nivel 10 (Heroe_Y_Tropa §10.2)");
        assert_eq!(n(10_000.0), 10, "el nivel no pasa del máximo");
    }

    #[test]
    fn la_experiencia_se_acumula_con_victoria_y_derrota_y_sube_de_nivel() {
        let r = r();
        let mut e = escuadra(Tropa::Arqueros);
        for _ in 0..5 {
            e.con_experiencia(&r, 1_000); // victoria: 1
        }
        assert_eq!((e.experiencia, e.nivel), (5_000, 2));
        e.con_experiencia(&r, 500); // derrota: 0,5
        assert_eq!((e.experiencia, e.nivel), (5_500, 2));
    }

    #[test]
    fn la_tropa_prestada_no_gana_experiencia() {
        let r = r();
        let mut p = Escuadra::prestada(&r, 2, 10, Tropa::Granjeros, 3, 0);
        assert_eq!((p.cantidad, p.prestada, p.plaza_origen), (30, Some(3), None));
        p.con_experiencia(&r, 50_000);
        assert_eq!((p.experiencia, p.nivel), (0, 1));
    }

    #[test]
    fn el_poder_suma_un_uno_por_ciento_por_nivel() {
        let r = r();
        let mut e = escuadra(Tropa::MiliciaLanceros); // 2 × 25 = 50
        assert_eq!(e.poder(&r), 50_500, "el nivel 1 ya cuenta +1 %");
        e.nivel = 10;
        assert_eq!(e.poder(&r), 55_000);
        let compuesto = escuadra(Tropa::ArquerosCompuesto); // 15 × 12 = 180
        assert_eq!(compuesto.poder(&r), 181_800);
        let mut vacia = escuadra(Tropa::Hequetai);
        vacia.cantidad = 0;
        assert_eq!(vacia.poder(&r), 0);
    }

    #[test]
    fn la_moral_baja_sin_racion_y_llega_a_cero_en_cinco_minutos() {
        let r = r();
        let mut e = escuadra(Tropa::Lenadores);
        let t = 3 * base::MS_POR_MINUTO;
        e.con_racion(&r, t, false);
        assert_eq!(e.moral_en(t), 100_000);
        assert_eq!(e.moral_en(t + 2 * base::MS_POR_MINUTO), 60_000, "20 por minuto");
        assert_eq!(e.moral_a_cero_en(t), Some(t + 5 * base::MS_POR_MINUTO));
        assert_eq!(e.moral_en(t + 60 * base::MS_POR_MINUTO), 0, "nunca por debajo de 0");
    }

    #[test]
    fn la_moral_sube_cinco_por_minuto_con_racion_y_no_pasa_de_cien() {
        let r = r();
        let mut e = escuadra(Tropa::Granjeros);
        e.con_racion(&r, 0, false);
        let t = 3 * base::MS_POR_MINUTO;
        assert_eq!(e.moral_en(t), 40_000);
        e.con_racion(&r, t, true);
        assert_eq!(e.moral_en(t + 4 * base::MS_POR_MINUTO), 60_000);
        assert_eq!(e.moral_en(t + 600 * base::MS_POR_MINUTO), 100_000);
        assert_eq!(e.moral_a_cero_en(t), None, "con ración no llega nunca a cero");
    }

    #[test]
    fn deserta_el_cinco_por_ciento_redondeado_hacia_arriba_solo_con_la_moral_a_cero() {
        let r = r();
        let mut e = escuadra(Tropa::MiliciaLanceros); // 25
        assert_eq!(e.desertar(&r, 0), 0, "con la moral alta no deserta nadie");
        e.con_racion(&r, 0, false);
        let t = 5 * base::MS_POR_MINUTO;
        assert_eq!(e.moral_en(t), 0);
        assert_eq!(e.desertar(&r, t), 2, "ceil(25 × 0,05) = 2");
        assert_eq!(e.cantidad, 23);
        assert_eq!(e.desertar(&r, t + INTERVALO_DESERCION), 2);
        assert_eq!(desertores(&r, 1), 1, "el último soldado también deserta");
        assert_eq!(desertores(&r, 0), 0);
        assert_eq!(desertores(&r, 20), 1);
        assert_eq!(desertores(&r, 21), 2);
    }

    #[test]
    fn una_escuadra_aniquilada_conserva_la_identidad() {
        let r = r();
        let mut e = escuadra(Tropa::Espartiatas);
        e.experiencia = 20_000;
        e.nivel = 3;
        e.cantidad = 1;
        e.con_racion(&r, 0, false);
        assert_eq!(e.desertar(&r, 10 * base::MS_POR_MINUTO), 1);
        assert_eq!((e.cantidad, e.nivel, e.experiencia, e.id), (0, 3, 20_000, 1));
        assert!(!e.con_soldados());
        assert_eq!(e.desertar(&r, 11 * base::MS_POR_MINUTO), 0);
    }
}
