//! La escuadra (Heroe_Y_Tropa §2.3 y §10): la instancia de una tropa que un héroe posee. Persiste a `cantidad` 0 (aniquilada) y se repone;
//! su nivel sube con experiencia; su moral es un **ritmo** (sube con ración y baja sin ella) y a 0 deserta.

use crate::catalogo::{Origen, Tropa};
use crate::datos::Reglas;
use crate::tipos::{AsentamientoId, AvanzadaId, BatallaId, CampamentoId, CaravanaId, ColumnaId, EscuadraId, HeroeId};
use asentamiento::datos::{por, UNO};
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
    /// En la guarnición de una avanzada de la plaza donde reside su héroe (Doc 5.17): prestada, come del almacén de la madre y defiende la avanzada.
    Avanzada(AvanzadaId),
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
    ///
    /// Si el ritmo ya es ese no toca nada: reanclar cambiaría `t0`, y de `t0` sale el instante en que la moral llegó a 0 (la fase de la
    /// deserción, [`Escuadra::desercion_a_partir_de`]).
    pub fn con_racion(&mut self, r: &Reglas, t: Instante, alimentada: bool) {
        let por_minuto = if alimentada { r.militar.regeneracion_moral } else { -r.militar.degradacion_moral };
        if self.moral.por_minuto != por_minuto {
            self.moral = self.moral.reanclado(t, por_minuto);
        }
    }

    /// Deja la moral quieta desde `t` (la escuadra deja de comer de una columna: ahora come en su campamento, que la anota en cada paso de
    /// hambre de su plaza).
    pub fn moral_quieta(&mut self, t: Instante) {
        if self.moral.por_minuto != 0 {
            self.moral = self.moral.reanclado(t, 0);
        }
    }

    /// Un paso de hambre de un minuto, tal como lo da la guarnición, que come en el paso de minuto de su plaza: con la ración entera
    /// (`suministro` = `UNO`) la moral sube `regeneracionMoralPorMinuto`; con una parte `f` de ella (ppm) baja `degradacionMoralSinRacion × (1 − f)`.
    /// El efecto se anota en `t` y la moral queda quieta hasta el paso siguiente (sin ritmo). No toca nada si la moral no cambia.
    pub fn paso_de_hambre(&mut self, r: &Reglas, t: Instante, suministro: i64) {
        let actual = self.moral_en(t);
        let nueva = if suministro >= UNO { (actual + r.militar.regeneracion_moral).min(MORAL_MAXIMA) } else { (actual - por(r.militar.degradacion_moral, UNO - suministro.max(0))).max(0) };
        if nueva != actual || self.moral.por_minuto != 0 {
            self.moral = Ritmo { valor_en_t0: nueva, t0: t, por_minuto: 0, ..self.moral };
        }
    }

    /// Primer instante `>= desde` en que la moral llega a 0 (donde empieza la deserción), si va a llegar.
    pub fn moral_a_cero_en(&self, desde: Instante) -> Option<Instante> {
        self.moral.instante_de_valor(0, desde)
    }

    /// Primer instante `>= desde` en que le toca desertar, si le va a tocar: la moral llega a 0 en `m0` (donde ya deserta una vez) y, **mientras siga
    /// sin ración**, cada [`INTERVALO_DESERCION`] después. La fase sale solo del ritmo de la moral (no se guarda ni se barre nada): sin
    /// soldados, o con la moral subiendo, no hay deserción que esperar.
    pub fn desercion_a_partir_de(&self, desde: Instante) -> Option<Instante> {
        if self.cantidad <= 0 || self.moral.por_minuto > 0 {
            return None;
        }
        let m0 = self.moral.instante_de_valor(0, self.moral.t0)?;
        Some(if desde <= m0 { m0 } else { m0 + (desde - m0 + INTERVALO_DESERCION - 1) / INTERVALO_DESERCION * INTERVALO_DESERCION })
    }

    /// ¿Le toca desertar justo en `t`? (la moral llegó a 0 en `t` o hace un número exacto de [`INTERVALO_DESERCION`]).
    pub fn deserta_en(&self, t: Instante) -> bool {
        self.desercion_a_partir_de(t) == Some(t)
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
    fn un_paso_de_hambre_sube_con_la_racion_entera_y_baja_en_proporcion_a_lo_que_falta() {
        let r = r();
        let mut e = escuadra(Tropa::Lenadores);
        e.paso_de_hambre(&r, 60_000, UNO);
        assert_eq!(e.moral, Ritmo { maximo: Some(MORAL_MAXIMA), minimo: Some(0), ..Ritmo::constante(MORAL_MAXIMA, 0) }, "a moral máxima y con ración no se toca nada");
        e.paso_de_hambre(&r, 60_000, 0);
        assert_eq!((e.moral_en(60_000), e.moral.t0), (80_000, 60_000), "sin nada de ración, 20 puntos");
        e.paso_de_hambre(&r, 120_000, 500_000);
        assert_eq!(e.moral_en(120_000), 70_000, "con la mitad, la mitad de la pérdida");
        assert_eq!(e.moral_en(150_000), 70_000, "y quieta hasta el paso siguiente");
        e.paso_de_hambre(&r, 180_000, UNO);
        e.paso_de_hambre(&r, 240_000, UNO);
        assert_eq!(e.moral_en(240_000), 80_000, "con ración entera sube cinco por paso");
        for k in 0..10 {
            e.paso_de_hambre(&r, 300_000 + k * 60_000, 0);
        }
        assert_eq!(e.moral_en(900_000), 0, "nunca por debajo de cero");
        assert_eq!(e.desertar(&r, 900_000), 2, "y a cero desertan ceil(30 × 0,05)");
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
    fn la_desercion_sin_racion_empieza_al_llegar_la_moral_a_cero_y_se_repite_cada_minuto_sin_guardar_nada() {
        let r = r();
        let mut e = escuadra(Tropa::MiliciaLanceros);
        assert_eq!(e.desercion_a_partir_de(0), None, "con la moral al máximo y quieta, no hay deserción que esperar");
        let t = 2 * base::MS_POR_MINUTO;
        e.con_racion(&r, t, false);
        let m0 = t + 5 * base::MS_POR_MINUTO;
        assert_eq!(e.desercion_a_partir_de(t), Some(m0), "la moral llega a 0 a los 5 minutos y deserta ya");
        assert_eq!(e.desercion_a_partir_de(m0), Some(m0));
        assert_eq!(e.desercion_a_partir_de(m0 + 1), Some(m0 + INTERVALO_DESERCION), "después, una vez por minuto");
        assert_eq!(e.desercion_a_partir_de(m0 + 3 * INTERVALO_DESERCION), Some(m0 + 3 * INTERVALO_DESERCION));
        assert!(e.deserta_en(m0 + 2 * INTERVALO_DESERCION) && !e.deserta_en(m0 + 2 * INTERVALO_DESERCION + 1));
        // Repetir el mismo hambre no cambia la fase (por eso `con_racion` no reancla si el ritmo es el mismo).
        e.con_racion(&r, m0 + 30_000, false);
        assert_eq!(e.desercion_a_partir_de(m0 + 1), Some(m0 + INTERVALO_DESERCION));
        // Con ración la moral sube y no se deserta; sin soldados tampoco.
        e.con_racion(&r, m0 + INTERVALO_DESERCION, true);
        assert_eq!(e.desercion_a_partir_de(m0 + INTERVALO_DESERCION), None);
        e.con_racion(&r, m0 + 2 * INTERVALO_DESERCION, false);
        e.cantidad = 0;
        assert_eq!(e.desercion_a_partir_de(0), None);
        // Quieta la moral (campamento) deja de haber deserción de columna; a 0 y quieta, sí la hay desde que se anotó.
        let mut q = escuadra(Tropa::Arqueros);
        q.con_racion(&r, 0, false);
        q.moral_quieta(10 * base::MS_POR_MINUTO);
        assert_eq!(q.moral_en(20 * base::MS_POR_MINUTO), 0);
        assert_eq!(q.moral.por_minuto, 0);
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
