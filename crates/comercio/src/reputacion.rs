//! Reputación de Facción (Comercio_Caravanas §4.6 y §13; Diplomacia_Facciones §3.6): un número −100..+100 que es un **ritmo** hacia 0, no
//! un barrido por minuto. Va en **ppm de punto** (1 punto = 1.000.000) porque 0,0005 por minuto no cabe en milésimas.
//!
//! ```text
//! rep > 0 :  rep − decaimientoPorMinuto, con suelo en 0
//! rep < 0 :  rep + decaimientoPorMinuto, con techo en 0
//! rep += alianzas activas × bonusPorMinutoAlianzaActiva         ; recortado a [−100, 100]
//! ```
//!
//! El estado guarda `(valor en t0, ritmo)`. El ritmo cambia de signo al cruzar el 0 por debajo: esa es una cita que el núcleo agenda
//! (`proximo_cambio`) y a la que responde con `reanclar`.

use crate::datos::{ReglasReputacion, PUNTO};
use base::{Instante, Ritmo};

/// Tope de la reputación, en ppm de punto.
pub const MAXIMO: i64 = 100 * PUNTO;

/// Lo que mueve la reputación de una Facción (cifras en `REPUTACION`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Suceso {
    TruequeCumplido,
    /// `fraccion` en ppm (0..UNO): la parte no entregada de un lado de un trueque vencido.
    TruequeIncumplido { fraccion: i64 },
    LiberarVasalloVoluntario,
    RebelionParaSenora,
    RomperAlianza,
    AtacarAliado,
    DefenderVasallo,
    NoDefenderVasallo,
    GuerraTrasRomperAlianza,
}

impl Suceso {
    /// Variación de reputación (ppm de punto).
    pub fn delta(self, r: &ReglasReputacion) -> i64 {
        match self {
            Suceso::TruequeCumplido => r.bonus_trueque_cumplido,
            Suceso::TruequeIncumplido { fraccion } => asentamiento::datos::por(r.penalizacion_trueque_incumplido, fraccion),
            Suceso::LiberarVasalloVoluntario => r.bonus_liberar_vasallo_voluntario,
            Suceso::RebelionParaSenora => r.penalizacion_rebelion_para_senora,
            Suceso::RomperAlianza => r.penalizacion_romper_alianza,
            Suceso::AtacarAliado => r.penalizacion_atacar_aliado,
            Suceso::DefenderVasallo => r.bonus_defender_vasallo,
            Suceso::NoDefenderVasallo => r.penalizacion_no_defender_vasallo,
            Suceso::GuerraTrasRomperAlianza => r.penalizacion_guerra_tras_romper_alianza,
        }
    }
}

/// La reputación de una Facción: valor en un instante más el ritmo con que se mueve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reputacion {
    ritmo: Ritmo,
    alianzas: u32,
}

impl Reputacion {
    /// Una Facción nueva: reputación 0.
    pub fn nueva(t0: Instante) -> Self {
        Self { ritmo: Ritmo::constante(0, t0), alianzas: 0 }
    }

    /// Restaurar de una foto: valor en `t0` (ppm de punto) y alianzas activas.
    pub fn desde(r: &ReglasReputacion, valor: i64, t0: Instante, alianzas: u32) -> Self {
        Self::anclada(r, valor.clamp(-MAXIMO, MAXIMO), t0, alianzas)
    }

    fn anclada(r: &ReglasReputacion, valor: i64, t0: Instante, alianzas: u32) -> Self {
        let bonus = r.bonus_por_minuto_alianza_activa * alianzas as i64;
        let ritmo = if valor >= 0 {
            // Decae hacia 0 (suelo en 0) y las alianzas empujan hacia arriba.
            Ritmo { valor_en_t0: valor, t0, por_minuto: bonus - r.decaimiento_por_minuto, maximo: Some(MAXIMO), minimo: Some(0) }
        } else {
            // Sube hacia 0; al llegar cambia de régimen (cita: `proximo_cambio`).
            Ritmo { valor_en_t0: valor, t0, por_minuto: bonus + r.decaimiento_por_minuto, maximo: Some(0), minimo: Some(-MAXIMO) }
        };
        Self { ritmo, alianzas }
    }

    /// Valor en `t` (ppm de punto).
    pub fn valor_en(&self, t: Instante) -> i64 {
        self.ritmo.valor_en(t)
    }

    pub fn alianzas(&self) -> u32 {
        self.alianzas
    }

    /// El ritmo con que se mueve (ppm de punto), para quien lo proyecta.
    pub fn ritmo(&self) -> Ritmo {
        self.ritmo
    }

    /// El ancla del ritmo `(valor en t0, t0)`, en ppm de punto: lo que una foto guarda para restaurarla con [`Reputacion::desde`].
    pub fn ancla(&self) -> (i64, Instante) {
        (self.ritmo.valor_en_t0, self.ritmo.t0)
    }

    /// Aplica una variación en `t` (recortada a ±100) y reancla el ritmo.
    pub fn ajustar(&mut self, r: &ReglasReputacion, t: Instante, delta: i64) {
        let v = (self.valor_en(t) + delta).clamp(-MAXIMO, MAXIMO);
        *self = Self::anclada(r, v, t, self.alianzas);
    }

    /// Aplica un [`Suceso`] en `t`.
    pub fn aplicar(&mut self, r: &ReglasReputacion, t: Instante, s: Suceso) {
        self.ajustar(r, t, s.delta(r));
    }

    /// Cambia el número de alianzas activas en `t`, sin saltos de valor.
    pub fn con_alianzas(&mut self, r: &ReglasReputacion, t: Instante, n: u32) {
        *self = Self::anclada(r, self.valor_en(t), t, n);
    }

    /// Reancla en `t` con el valor que tiene (tras la cita de cruce por 0).
    pub fn reanclar(&mut self, r: &ReglasReputacion, t: Instante) {
        *self = Self::anclada(r, self.valor_en(t), t, self.alianzas);
    }

    /// Próximo instante `> desde` en que el régimen cambia (cruce de 0 por debajo), para agendar la cita; `None` si no hay.
    pub fn proximo_cambio(&self, desde: Instante) -> Option<Instante> {
        (self.valor_en(desde) < 0).then(|| self.ritmo.instante_de_valor(0, desde)).flatten()
    }

    /// ¿Está la reputación `≤ umbral` en `t`? (`umbralBajoParaComision`, `umbralBajoParaEmbajador` son umbrales de este tipo.)
    pub fn esta_bajo(&self, umbral: i64, t: Instante) -> bool {
        self.valor_en(t) <= umbral
    }

    /// Primer instante `≥ desde` en que el valor cruza `umbral` (por abajo si decae, por arriba si crece); para agendar reevaluaciones.
    pub fn cruce_de(&self, umbral: i64, desde: Instante) -> Option<Instante> {
        self.ritmo.instante_de_valor(umbral, desde)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::Reglas;
    use base::MS_POR_MINUTO;

    fn r() -> ReglasReputacion {
        Reglas::cargar().reputacion
    }

    #[test]
    fn los_sucesos_usan_las_cifras_del_bloque() {
        let r = r();
        assert_eq!(Suceso::TruequeCumplido.delta(&r), 5 * PUNTO);
        assert_eq!(Suceso::TruequeIncumplido { fraccion: 1_000_000 }.delta(&r), -8 * PUNTO);
        assert_eq!(Suceso::TruequeIncumplido { fraccion: 250_000 }.delta(&r), -2 * PUNTO);
        assert_eq!(Suceso::AtacarAliado.delta(&r), -25 * PUNTO);
        assert_eq!(Suceso::DefenderVasallo.delta(&r), 15 * PUNTO);
    }

    #[test]
    fn decae_lentamente_hacia_cero_sin_pasarse() {
        let r = r();
        let mut rep = Reputacion::nueva(0);
        rep.aplicar(&r, 0, Suceso::TruequeCumplido);
        // 0,002 por minuto: tras 1000 min quedan 3 puntos; con 0,2 se habría borrado en 25 min.
        assert_eq!(rep.valor_en(1_000 * MS_POR_MINUTO), 3 * PUNTO);
        assert_eq!(rep.valor_en(30 * MS_POR_MINUTO), 5 * PUNTO - 60_000);
        assert_eq!(rep.valor_en(100_000 * MS_POR_MINUTO), 0, "suelo en 0");
    }

    #[test]
    fn una_reputacion_negativa_sube_hasta_cero_y_avisa_del_cruce() {
        let r = r();
        let mut rep = Reputacion::nueva(0);
        rep.aplicar(&r, 0, Suceso::TruequeIncumplido { fraccion: 1_000_000 });
        assert_eq!(rep.valor_en(0), -8 * PUNTO);
        let cruce = rep.proximo_cambio(0).unwrap();
        assert_eq!(cruce, 4_000 * MS_POR_MINUTO, "8 puntos a 0,002 por minuto");
        assert_eq!(rep.valor_en(cruce + 10 * MS_POR_MINUTO), 0, "sin la cita, el techo en 0 la sujeta");
        rep.reanclar(&r, cruce);
        assert_eq!(rep.proximo_cambio(cruce), None);
    }

    #[test]
    fn el_umbral_bajo_se_alcanza_y_se_recupera_despacio() {
        let r = r();
        let mut rep = Reputacion::nueva(0);
        rep.aplicar(&r, 0, Suceso::AtacarAliado);
        rep.aplicar(&r, 0, Suceso::RomperAlianza);
        rep.aplicar(&r, 0, Suceso::RomperAlianza);
        assert_eq!(rep.valor_en(0), -49 * PUNTO);
        assert!(rep.esta_bajo(r.umbral_bajo_para_comision, 0));
        // Sale del umbral (−40) cuando recupera 9 puntos: 4.500 minutos.
        let sale = rep.cruce_de(r.umbral_bajo_para_comision + 1, 0).unwrap();
        assert!(rep.esta_bajo(r.umbral_bajo_para_comision, sale - MS_POR_MINUTO));
        assert!(!rep.esta_bajo(r.umbral_bajo_para_comision, sale + MS_POR_MINUTO));
    }

    #[test]
    fn el_bonus_de_alianza_por_si_solo_no_la_sube() {
        let r = r();
        let mut rep = Reputacion::nueva(0);
        rep.con_alianzas(&r, 0, 1);
        assert_eq!(rep.valor_en(1_000_000 * MS_POR_MINUTO), 0, "0,0005 < 0,002: no se sostiene por encima de 0");
        // Con cinco alianzas el bonus (0,0025) supera al decaimiento y sí sube, hasta el tope.
        let mut rep = Reputacion::desde(&r, PUNTO, 0, 5);
        assert_eq!(rep.valor_en(1_000 * MS_POR_MINUTO), PUNTO + 500_000);
        assert_eq!(rep.valor_en(1_000_000_000 * MS_POR_MINUTO), MAXIMO);
        rep.con_alianzas(&r, 1_000 * MS_POR_MINUTO, 0);
        assert_eq!(rep.valor_en(1_000 * MS_POR_MINUTO), PUNTO + 500_000, "cambiar el ritmo no salta");
    }

    #[test]
    fn recorta_a_mas_menos_cien() {
        let r = r();
        let mut rep = Reputacion::nueva(0);
        for _ in 0..30 {
            rep.aplicar(&r, 0, Suceso::AtacarAliado);
        }
        assert_eq!(rep.valor_en(0), -MAXIMO);
        for _ in 0..30 {
            rep.aplicar(&r, 0, Suceso::DefenderVasallo);
        }
        assert_eq!(rep.valor_en(0), MAXIMO);
    }

    #[test]
    fn es_determinista() {
        let r = r();
        let hacer = || {
            let mut x = Reputacion::nueva(7);
            x.aplicar(&r, 1_000, Suceso::TruequeCumplido);
            x.con_alianzas(&r, 50_000, 2);
            x.aplicar(&r, 90_000, Suceso::TruequeIncumplido { fraccion: 333_333 });
            x.valor_en(7_000_000)
        };
        assert_eq!(hacer(), hacer());
    }
}
