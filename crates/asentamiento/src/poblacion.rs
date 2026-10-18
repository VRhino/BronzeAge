//! Población, comida y recaudación de un asentamiento (Doc 4.1, Economia_Construccion §9). Tres clases que crecen con la misma
//! fórmula logística y difieren en la tasa; la nutrición mide el hambre; la recaudación es el espejo del consumo.
//!
//! **Continua, sin tirada** (Reloj_Y_Eventos §10): la población se guarda con decimales (milésimas de habitante) y se lee entera
//! (suelo). Cada paso es de **un minuto de mundo** y solo usa enteros, así que dos máquinas dan el mismo resultado. Con hambre
//! sostenida la población se multiplica cada minuto por `1 − fracción`, que es exactamente el decaimiento exponencial decidido
//! el 2026-10-18 (sin redondeo hacia arriba ni eventos por minuto).

use crate::datos::{por, razon, ReglasPoblacion, UNO};
use base::Mil;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Poblacion {
    /// Habitantes por clase, en milésimas.
    pub pesants: Mil,
    pub artesanos: Mil,
    pub nobleza: Mil,
    /// 0..100 en milésimas.
    pub nutricion: Mil,
}

/// Lo que condiciona el paso de un minuto y que sale de otros sistemas (viviendas, edificios, políticas, nivel).
#[derive(Clone, Debug)]
pub struct Entorno {
    /// Cupos en habitantes (milésimas): Viviendas activas × 15 y × 10, y los nobles del Palacio.
    pub capacidad_pesants: Mil,
    pub capacidad_artesanos: Mil,
    pub capacidad_nobles: Mil,
    /// Techo de población total del nivel alcanzado (milésimas).
    pub techo: Mil,
    /// Hay algún taller de transformación activo (aparecen los artesanos).
    pub hay_transformacion: bool,
    /// Hay Palacio activo.
    pub hay_palacio: bool,
    /// Ciudadanos (héroes con casa) en el asentamiento: la nobleza pide un mínimo.
    pub ciudadanos: u32,
    /// `factorConsumoComida` de las políticas (ppm).
    pub factor_consumo: i64,
    /// Felicidad (ppm): `factorCrecimientoPoblacion` × ocupación × Aedas. La estabilidad vale 1.
    pub felicidad: i64,
    /// `factorCrecimientoNobleza` × (1 + bono de Aedas) (ppm).
    pub factor_nobleza: i64,
    /// `factorRecaudacion` de las políticas × ocupación (ppm).
    pub factor_recaudacion: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Suceso {
    PrimerosArtesanos,
    PrimerosNobles,
    /// Pérdida de población de este minuto por hambre (milésimas por clase).
    Hambruna { pesants: Mil, artesanos: Mil, nobleza: Mil },
}

impl Poblacion {
    pub fn nueva(r: &ReglasPoblacion) -> Self {
        Self { pesants: r.pesants_inicial, artesanos: 0, nobleza: 0, nutricion: r.hambre.nutricion_inicial }
    }

    /// Total en milésimas.
    pub fn total(&self) -> Mil {
        self.pesants + self.artesanos + self.nobleza
    }

    /// Habitantes enteros por clase (suelo), que es lo que ven los demás sistemas.
    pub fn enteros(&self) -> (i64, i64, i64) {
        (self.pesants / 1000, self.artesanos / 1000, self.nobleza / 1000)
    }

    /// Habitantes por clase redondeados al entero más cercano. Es la lectura de los **requisitos de nivel** (decidido el 2026-10-18):
    /// con el suelo, una clase que se acerca al cupo sin tocarlo (399,997 de 400) nunca los cumpliría.
    pub fn redondeados(&self) -> (i64, i64, i64) {
        ((self.pesants + 500) / 1000, (self.artesanos + 500) / 1000, (self.nobleza + 500) / 1000)
    }

    /// Trigo que come toda la población en un minuto (milésimas), sin importar la clase.
    pub fn consumo_por_minuto(&self, r: &ReglasPoblacion, e: &Entorno) -> Mil {
        por(por(self.total(), r.consumo_por_habitante), e.factor_consumo)
    }

    /// Un minuto de comida y hambre (`avanzarNutricionPoblacion`). `trigo` es lo que hay en el almacén; devuelve lo que se
    /// come. Con la nutrición en el umbral (0) muere la fracción de cada clase.
    pub fn nutrir(&mut self, r: &ReglasPoblacion, e: &Entorno, trigo: Mil, sucesos: &mut Vec<Suceso>) -> Mil {
        let h = &r.hambre;
        let consumo = self.consumo_por_minuto(r, e);
        let suministro = if consumo > 0 { razon(trigo.max(0), consumo).min(UNO) } else { UNO };
        let comido = consumo.min(trigo.max(0));
        self.nutricion = if suministro >= UNO {
            (self.nutricion + h.regeneracion_por_minuto).min(100_000)
        } else {
            (self.nutricion - por(h.degradacion_sin_comida, UNO - suministro)).max(0)
        };
        if self.nutricion <= h.umbral_muerte {
            let (mp, ma, mn) = (por(self.pesants, h.muerte_por_minuto), por(self.artesanos, h.muerte_por_minuto), por(self.nobleza, h.muerte_nobleza_por_minuto));
            self.pesants -= mp;
            self.artesanos -= ma;
            self.nobleza -= mn;
            if mp + ma + mn > 0 {
                sucesos.push(Suceso::Hambruna { pesants: mp, artesanos: ma, nobleza: mn });
            }
        }
        comido
    }

    /// Un minuto de crecimiento (`crecerPoblacion`): las tres clases a la vez, con el techo del nivel repartido en proporción.
    pub fn crecer(&mut self, r: &ReglasPoblacion, e: &Entorno, sucesos: &mut Vec<Suceso>) {
        let h = &r.hambre;
        let comida = h.factor_crecimiento_minimo + por(UNO - h.factor_crecimiento_minimo, razon(self.nutricion, 100_000));
        let espacio = |p: Mil, cap: Mil| if cap <= 0 { 0 } else { razon(cap - p, cap).clamp(0, UNO) };
        let tasa = |base: i64, esp: i64| por(por(por(base, comida), esp), e.felicidad);

        let (esp_p, esp_a, esp_n) = (espacio(self.pesants, e.capacidad_pesants), espacio(self.artesanos, e.capacidad_artesanos), espacio(self.nobleza, e.capacidad_nobles));
        let mut inc_p = por(self.pesants, tasa(r.tasa_pesants, esp_p));
        let (mut inc_a, mut inc_n) = (0, 0);

        if e.hay_transformacion && esp_a > 0 {
            if self.artesanos == 0 {
                self.artesanos = 1000;
                sucesos.push(Suceso::PrimerosArtesanos);
            } else {
                inc_a = por(self.artesanos, tasa(r.tasa_artesanos, esp_a));
            }
        }
        if e.ciudadanos >= r.min_ciudadanos_nobleza && e.hay_palacio && esp_n > 0 {
            if self.nobleza == 0 {
                self.nobleza = 1000;
                sucesos.push(Suceso::PrimerosNobles);
            } else {
                inc_n = por(por(self.nobleza, tasa(r.tasa_nobleza, esp_n)), e.factor_nobleza);
            }
        }

        // El techo nunca purga población ya asentada: solo recorta lo que se iba a añadir.
        let bruto = inc_p + inc_a + inc_n;
        let libre = (e.techo - self.total()).max(0);
        let factor = if bruto > libre && bruto > 0 { razon(libre, bruto) } else { UNO };
        inc_p = por(inc_p, factor);
        self.pesants += inc_p;
        self.artesanos += por(inc_a, factor);
        self.nobleza += por(inc_n, factor);
    }

    /// Oro que recauda en un minuto (`recaudacionOro`): Σ habitantes de la clase × tasa de la clase × factor.
    pub fn recaudacion_por_minuto(&self, r: &ReglasPoblacion, e: &Entorno) -> Mil {
        let base = por(self.pesants, r.impuesto_pesants) + por(self.artesanos, r.impuesto_artesanos) + por(self.nobleza, r.impuesto_nobleza);
        por(base, e.factor_recaudacion)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::datos::Reglas;

    fn entorno() -> Entorno {
        Entorno {
            capacidad_pesants: 150_000,
            capacidad_artesanos: 100_000,
            capacidad_nobles: 0,
            techo: 300_000,
            hay_transformacion: false,
            hay_palacio: false,
            ciudadanos: 0,
            factor_consumo: UNO,
            felicidad: UNO,
            factor_nobleza: UNO,
            factor_recaudacion: UNO,
        }
    }

    #[test]
    fn nace_con_veinte_pesants_y_nutricion_llena() {
        let r = Reglas::cargar().poblacion;
        let p = Poblacion::nueva(&r);
        assert_eq!((p.enteros(), p.nutricion), ((20, 0, 0), 100_000));
        let casi = Poblacion { pesants: 199_600, artesanos: 399_997, nobleza: 499, nutricion: 0 };
        assert_eq!((casi.enteros(), casi.redondeados()), ((199, 399, 0), (200, 400, 0)), "suelo para la mano de obra, redondeo para los requisitos de nivel");
    }

    #[test]
    fn el_consumo_es_una_decima_por_habitante_y_el_racionamiento_lo_baja() {
        let r = Reglas::cargar().poblacion;
        let p = Poblacion::nueva(&r);
        let mut e = entorno();
        assert_eq!(p.consumo_por_minuto(&r, &e), 2_000, "20 habitantes × 0,1");
        e.factor_consumo = 800_000;
        assert_eq!(p.consumo_por_minuto(&r, &e), 1_600);
    }

    #[test]
    fn crece_en_logistica_y_se_para_en_el_cupo() {
        let r = Reglas::cargar().poblacion;
        let (mut p, e, mut s) = (Poblacion::nueva(&r), entorno(), vec![]);
        let mut antes = p.pesants;
        for _ in 0..600 {
            p.nutrir(&r, &e, 1_000_000, &mut s);
            p.crecer(&r, &e, &mut s);
            assert!(p.pesants >= antes, "nunca baja con comida");
            assert!(p.pesants <= e.capacidad_pesants, "nunca pasa del cupo de Viviendas");
            antes = p.pesants;
        }
        assert!(p.pesants > 149_000, "tras 10 h llena las Viviendas: {}", p.pesants);
        assert!(s.is_empty());
    }

    #[test]
    fn el_primer_ritmo_es_el_esperado() {
        // 20 pesants, tasa 0,12 × comida 1 × espacio (1 − 20/150) = 0,104 → +2,08 por minuto.
        let r = Reglas::cargar().poblacion;
        let (mut p, e) = (Poblacion::nueva(&r), entorno());
        p.crecer(&r, &e, &mut vec![]);
        assert_eq!(p.pesants - 20_000, 2_080);
    }

    #[test]
    fn sin_trigo_la_nutricion_cae_a_cero_en_cinco_minutos_y_luego_muere_la_gente() {
        let r = Reglas::cargar().poblacion;
        let (mut p, e, mut s) = (Poblacion::nueva(&r), entorno(), vec![]);
        for _ in 0..5 {
            assert_eq!(p.nutrir(&r, &e, 0, &mut s), 0);
        }
        assert_eq!(p.nutricion, 0, "de 100 a 0 en 5 minutos (20 por minuto)");
        assert_eq!(s.len(), 1, "en el quinto minuto ya muere el 5 %");
        let antes = p.pesants;
        p.nutrir(&r, &e, 0, &mut s);
        assert_eq!(p.pesants, antes - antes * 50_000 / UNO, "el 5 % exacto, sin redondear hacia arriba");
        // El decaimiento es exponencial: tras n minutos queda 0,95^n.
        let mut q = Poblacion { pesants: 100_000, artesanos: 0, nobleza: 100_000, nutricion: 0 };
        for _ in 0..10 {
            q.nutrir(&r, &e, 0, &mut vec![]);
        }
        assert!((q.pesants as f64 - 100_000.0 * 0.95f64.powi(10)).abs() < 5.0, "{}", q.pesants);
        assert!((q.nobleza as f64 - 100_000.0 * 0.99f64.powi(10)).abs() < 5.0, "{}", q.nobleza);
    }

    #[test]
    fn el_hambre_frena_el_crecimiento_sin_pararlo() {
        let r = Reglas::cargar().poblacion;
        let e = entorno();
        let crecimiento = |nutricion: Mil| {
            let mut p = Poblacion { nutricion, ..Poblacion::nueva(&r) };
            p.crecer(&r, &e, &mut vec![]);
            p.pesants - 20_000
        };
        assert_eq!(crecimiento(100_000), 2_080);
        assert_eq!(crecimiento(0), 2_080 / 5, "con nutrición 0 el factor es 0,2");
        assert!(crecimiento(50_000) > crecimiento(0) && crecimiento(50_000) < crecimiento(100_000));
    }

    #[test]
    fn los_artesanos_y_la_nobleza_aparecen_cuando_se_cumple_su_condicion() {
        let r = Reglas::cargar().poblacion;
        let (mut p, mut e, mut s) = (Poblacion::nueva(&r), entorno(), vec![]);
        p.crecer(&r, &e, &mut s);
        assert_eq!((p.artesanos, p.nobleza), (0, 0), "sin taller ni Palacio no hay clases nuevas");
        e.hay_transformacion = true;
        p.crecer(&r, &e, &mut s);
        assert_eq!(p.artesanos, 1000, "el primero llega sin tirada");
        assert_eq!(s, vec![Suceso::PrimerosArtesanos]);
        e.hay_palacio = true;
        e.capacidad_nobles = 80_000;
        p.crecer(&r, &e, &mut s);
        assert_eq!(p.nobleza, 0, "la nobleza pide un mínimo de ciudadanos");
        e.ciudadanos = 3;
        p.crecer(&r, &e, &mut s);
        assert_eq!(p.nobleza, 1000);
        assert_eq!(s.last(), Some(&Suceso::PrimerosNobles));
    }

    #[test]
    fn el_techo_recorta_lo_que_se_anade_pero_no_purga() {
        let r = Reglas::cargar().poblacion;
        let (mut p, mut e) = (Poblacion::nueva(&r), entorno());
        e.techo = 21_000;
        p.crecer(&r, &e, &mut vec![]);
        assert_eq!(p.total(), 21_000, "el bruto de 2,08 se recorta a lo que cabe bajo el techo");
        e.techo = 10_000;
        p.crecer(&r, &e, &mut vec![]);
        assert_eq!(p.total(), 21_000, "ya por encima del techo no crece, pero tampoco se purga");
    }

    #[test]
    fn la_recaudacion_es_el_espejo_del_consumo_y_la_presion_fiscal_la_sube() {
        let r = Reglas::cargar().poblacion;
        let mut e = entorno();
        let p = Poblacion { pesants: 100_000, artesanos: 50_000, nobleza: 10_000, nutricion: 100_000 };
        // 100 × 0,004 + 50 × 0,015 + 10 × 0,06 = 0,4 + 0,75 + 0,6 = 1,75
        assert_eq!(p.recaudacion_por_minuto(&r, &e), 1_750);
        e.factor_recaudacion = 1_600_000;
        assert_eq!(p.recaudacion_por_minuto(&r, &e), 2_800);
    }

    #[test]
    fn es_determinista() {
        let r = Reglas::cargar().poblacion;
        let correr = || {
            let (mut p, e, mut s) = (Poblacion::nueva(&r), entorno(), vec![]);
            for i in 0..2000 {
                p.nutrir(&r, &e, if i % 7 == 0 { 0 } else { 5_000 }, &mut s);
                p.crecer(&r, &e, &mut s);
            }
            (p, s)
        };
        assert_eq!(correr(), correr());
    }
}
