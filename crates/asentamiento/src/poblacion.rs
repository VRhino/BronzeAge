//! Población, comida y recaudación de un asentamiento (Doc 4.1, Economia_Construccion §9). Tres clases que crecen con la misma
//! fórmula logística y difieren en la tasa; la nutrición mide el hambre; la recaudación es el espejo del consumo.
//!
//! **Siempre entera** (decidido el 2026-10-22): cada clase guarda habitantes enteros y un **resto oculto** en milésimas de habitante. El
//! crecimiento de cada minuto se suma al resto y entra una persona cuando llega a 1; las muertes por hambre usan el mismo resto con el signo
//! cambiado, así que con 10 habitantes y un 5 % por minuto muere una persona cada dos minutos. Cada paso es de **un minuto de mundo** y solo
//! usa enteros, así que dos máquinas dan el mismo resultado sin tirada.
//!
//! **Hambre** (D7 y D8, Economia_Construccion §9.2): con la nutrición en 0 no crece ninguna clase y solo hay mortalidad. Al tocar 0 la plaza
//! entra en **recuperación** y no vuelve a crecer hasta recuperar la nutrición 100; durante la recuperación con nutrición positiva la
//! población queda quieta, y si la nutrición vuelve a 0 vuelve la mortalidad.

use crate::datos::{por, razon, ReglasPoblacion, UNO};
use base::Mil;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Poblacion {
    /// Habitantes enteros por clase.
    pub pesants: i64,
    pub artesanos: i64,
    pub nobleza: i64,
    /// Resto oculto de cada clase (pesants, artesanos, nobleza) en milésimas de habitante, siempre en `(−1000, 1000)`: positivo es
    /// crecimiento a medias, negativo son muertes a medias. Nunca lo leen los demás sistemas.
    pub restos: [Mil; 3],
    /// 0..100 en milésimas.
    pub nutricion: Mil,
    /// La plaza tocó nutrición 0 y no ha vuelto a 100: no crece (D8).
    pub recuperando: bool,
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
    /// Personas que murieron de hambre este minuto, por clase.
    Hambruna { pesants: i64, artesanos: i64, nobleza: i64 },
}

/// Suma `delta` milésimas al resto y devuelve las personas enteras que entran (+) o salen (−); el resto queda con lo que sobra. Un resto
/// del signo contrario (de antes de que cambiara el régimen: de la mortalidad al crecimiento o al revés) se descarta.
fn acumular(resto: &mut Mil, delta: Mil) -> i64 {
    if *resto * delta < 0 {
        *resto = 0;
    }
    *resto += delta;
    let personas = *resto / 1000;
    *resto -= personas * 1000;
    personas
}

/// Las muertes de un minuto en una clase: la `fraccion` de sus habitantes enteros va al resto, y salen las personas que se completan.
fn mueren(clase: &mut i64, resto: &mut Mil, fraccion: i64) -> i64 {
    let muertos = -acumular(resto, -por(*clase * 1000, fraccion));
    *clase -= muertos;
    muertos
}

impl Poblacion {
    pub fn nueva(r: &ReglasPoblacion) -> Self {
        Self { pesants: r.pesants_inicial / 1000, artesanos: 0, nobleza: 0, restos: [0; 3], nutricion: r.hambre.nutricion_inicial, recuperando: false }
    }

    /// Total de habitantes.
    pub fn total(&self) -> i64 {
        self.pesants + self.artesanos + self.nobleza
    }

    /// Habitantes por clase (pesants, artesanos, nobleza): lo que ven los demás sistemas.
    pub fn enteros(&self) -> (i64, i64, i64) {
        (self.pesants, self.artesanos, self.nobleza)
    }

    /// Trigo que come toda la población en un minuto (milésimas), sin importar la clase.
    pub fn consumo_por_minuto(&self, r: &ReglasPoblacion, e: &Entorno) -> Mil {
        por(por(self.total() * 1000, r.consumo_por_habitante), e.factor_consumo)
    }

    /// Un minuto de comida y hambre (`avanzarNutricionPoblacion`). `trigo` es lo que hay en el almacén; devuelve lo que se
    /// come. Con la nutrición en el umbral (0) la plaza entra en recuperación y muere la fracción de cada clase; al llegar a 100 la
    /// recuperación termina.
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
            self.recuperando = true;
            let mp = mueren(&mut self.pesants, &mut self.restos[0], h.muerte_por_minuto);
            let ma = mueren(&mut self.artesanos, &mut self.restos[1], h.muerte_por_minuto);
            let mn = mueren(&mut self.nobleza, &mut self.restos[2], h.muerte_nobleza_por_minuto);
            if mp + ma + mn > 0 {
                sucesos.push(Suceso::Hambruna { pesants: mp, artesanos: ma, nobleza: mn });
            }
        } else if self.nutricion >= 100_000 {
            self.recuperando = false;
        }
        comido
    }

    /// Un minuto de crecimiento (`crecerPoblacion`): las tres clases a la vez, con el techo del nivel repartido en proporción. La tasa se
    /// aplica a los habitantes **con su resto** (la población continua de siempre); solo entran personas enteras. En recuperación o con la
    /// nutrición en 0 no crece nadie.
    pub fn crecer(&mut self, r: &ReglasPoblacion, e: &Entorno, sucesos: &mut Vec<Suceso>) {
        let h = &r.hambre;
        if self.recuperando || self.nutricion <= h.umbral_muerte {
            return;
        }
        let comida = h.factor_crecimiento_minimo + por(UNO - h.factor_crecimiento_minimo, razon(self.nutricion, 100_000));
        let espacio = |p: Mil, cap: Mil| if cap <= 0 { 0 } else { razon(cap - p, cap).clamp(0, UNO) };
        let tasa = |base: i64, esp: i64| por(por(por(base, comida), esp), e.felicidad);
        let con_resto = |personas: i64, resto: Mil| personas * 1000 + resto.max(0);

        let (pes, art, nob) = (con_resto(self.pesants, self.restos[0]), con_resto(self.artesanos, self.restos[1]), con_resto(self.nobleza, self.restos[2]));
        let (esp_p, esp_a, esp_n) = (espacio(pes, e.capacidad_pesants), espacio(art, e.capacidad_artesanos), espacio(nob, e.capacidad_nobles));
        let mut inc_p = por(pes, tasa(r.tasa_pesants, esp_p));
        let (mut inc_a, mut inc_n) = (0, 0);

        if e.hay_transformacion && esp_a > 0 {
            if self.artesanos == 0 {
                self.artesanos = 1;
                sucesos.push(Suceso::PrimerosArtesanos);
            } else {
                inc_a = por(art, tasa(r.tasa_artesanos, esp_a));
            }
        }
        if e.ciudadanos >= r.min_ciudadanos_nobleza && e.hay_palacio && esp_n > 0 {
            if self.nobleza == 0 {
                self.nobleza = 1;
                sucesos.push(Suceso::PrimerosNobles);
            } else {
                inc_n = por(por(nob, tasa(r.tasa_nobleza, esp_n)), e.factor_nobleza);
            }
        }

        // El techo nunca purga población ya asentada: solo recorta lo que se iba a añadir.
        let bruto = inc_p + inc_a + inc_n;
        let total = con_resto(self.pesants, self.restos[0]) + con_resto(self.artesanos, self.restos[1]) + con_resto(self.nobleza, self.restos[2]);
        let libre = (e.techo - total).max(0);
        let factor = if bruto > libre && bruto > 0 { razon(libre, bruto) } else { UNO };
        inc_p = por(inc_p, factor);
        self.pesants += acumular(&mut self.restos[0], inc_p);
        self.artesanos += acumular(&mut self.restos[1], por(inc_a, factor));
        self.nobleza += acumular(&mut self.restos[2], por(inc_n, factor));
    }

    /// Oro que recauda en un minuto (`recaudacionOro`): Σ habitantes de la clase × tasa de la clase × factor.
    pub fn recaudacion_por_minuto(&self, r: &ReglasPoblacion, e: &Entorno) -> Mil {
        let base = por(self.pesants * 1000, r.impuesto_pesants) + por(self.artesanos * 1000, r.impuesto_artesanos) + por(self.nobleza * 1000, r.impuesto_nobleza);
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

    /// Pesants con su resto, en milésimas: la población continua de siempre.
    fn continua(p: &Poblacion) -> Mil {
        p.pesants * 1000 + p.restos[0]
    }

    fn con(pesants: i64, artesanos: i64, nobleza: i64, nutricion: Mil) -> Poblacion {
        Poblacion { pesants, artesanos, nobleza, restos: [0; 3], nutricion, recuperando: false }
    }

    #[test]
    fn nace_con_veinte_pesants_enteros_y_nutricion_llena() {
        let r = Reglas::cargar().poblacion;
        let p = Poblacion::nueva(&r);
        assert_eq!((p.enteros(), p.restos, p.nutricion, p.recuperando), ((20, 0, 0), [0; 3], 100_000, false));
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
    fn crece_en_logistica_en_personas_enteras_y_se_para_en_el_cupo() {
        let r = Reglas::cargar().poblacion;
        let (mut p, e, mut s) = (Poblacion::nueva(&r), entorno(), vec![]);
        let mut antes = p.pesants;
        for _ in 0..600 {
            p.nutrir(&r, &e, 1_000_000, &mut s);
            p.crecer(&r, &e, &mut s);
            assert!(p.pesants >= antes, "nunca baja con comida");
            assert!(continua(&p) <= e.capacidad_pesants, "nunca pasa del cupo de Viviendas");
            assert!((0..1000).contains(&p.restos[0]), "el resto es una fracción de persona: {}", p.restos[0]);
            antes = p.pesants;
        }
        assert!(p.pesants >= 149, "tras 10 h llena las Viviendas: {}", p.pesants);
        assert!(s.is_empty());
    }

    #[test]
    fn el_primer_ritmo_es_el_de_siempre_y_el_resto_guarda_lo_que_no_llega_a_persona() {
        // 20 pesants, tasa 0,12 × comida 1 × espacio (1 − 20/150) = 0,104 → +2,08 por minuto: entran 2 y quedan 0,08.
        let r = Reglas::cargar().poblacion;
        let (mut p, e) = (Poblacion::nueva(&r), entorno());
        p.crecer(&r, &e, &mut vec![]);
        assert_eq!((p.pesants, p.restos[0]), (22, 80));
        // El segundo minuto crece sobre 22,08 (con su resto), no sobre 22: +2,26 y el resto pasa a 0,34.
        p.crecer(&r, &e, &mut vec![]);
        assert_eq!((p.pesants, p.restos[0]), (24, 340));
    }

    #[test]
    fn sin_trigo_la_nutricion_cae_a_cero_en_cinco_minutos_y_luego_muere_la_gente() {
        let r = Reglas::cargar().poblacion;
        let (mut p, e, mut s) = (Poblacion::nueva(&r), entorno(), vec![]);
        for _ in 0..4 {
            assert_eq!(p.nutrir(&r, &e, 0, &mut s), 0);
        }
        assert!(s.is_empty() && !p.recuperando, "con nutrición 20 todavía no muere nadie ni hay recuperación");
        p.nutrir(&r, &e, 0, &mut s);
        assert_eq!(p.nutricion, 0, "de 100 a 0 en 5 minutos (20 por minuto)");
        assert!(p.recuperando, "al tocar 0 entra en recuperación");
        assert_eq!((p.pesants, p.restos[0], s.clone()), (19, 0, vec![Suceso::Hambruna { pesants: 1, artesanos: 0, nobleza: 0 }]), "en el quinto minuto el 5 % de 20 es una persona entera");
    }

    #[test]
    fn la_muerte_es_simetrica_al_crecimiento_con_diez_habitantes_muere_uno_cada_dos_minutos() {
        let r = Reglas::cargar().poblacion;
        let (mut p, e, mut s) = (con(10, 0, 0, 0), entorno(), vec![]);
        let mut vivos = vec![];
        for _ in 0..4 {
            p.nutrir(&r, &e, 0, &mut s);
            vivos.push((p.pesants, p.restos[0]));
        }
        assert_eq!(vivos, vec![(10, -500), (9, 0), (9, -450), (9, -900)], "la mitad cada minuto y, con 9, el 5 % de 9 es 0,45");
        // El decaimiento sigue la fracción de lo que queda, sin redondear hacia arriba: 100 nobles al 1 % durante 10 minutos son 91.
        let mut q = con(100, 0, 100, 0);
        for _ in 0..10 {
            q.nutrir(&r, &e, 0, &mut vec![]);
        }
        assert_eq!(q.nobleza, 91);
        assert!((58..=62).contains(&q.pesants), "0,95^10 de 100 es 59,9: {}", q.pesants);
    }

    #[test]
    fn d7_con_nutricion_cero_nadie_crece_solo_hay_mortalidad() {
        let r = Reglas::cargar().poblacion;
        let mut e = entorno();
        e.hay_transformacion = true;
        e.hay_palacio = true;
        e.ciudadanos = 3;
        e.capacidad_nobles = 80_000;
        // Aunque no haya pasado por `nutrir` (sin marca de recuperación), con nutrición 0 no crece ninguna clase ni aparece una nueva.
        let mut p = con(20, 5, 5, 0);
        p.crecer(&r, &e, &mut vec![]);
        assert_eq!((p.enteros(), p.restos), ((20, 5, 5), [0; 3]));
        let mut q = con(20, 0, 0, 0);
        let mut s = vec![];
        q.crecer(&r, &e, &mut s);
        assert!((q.enteros(), s.is_empty()) == ((20, 0, 0), true), "ni siquiera los primeros artesanos o nobles");
    }

    #[test]
    fn d8_tras_el_hambre_no_crece_hasta_recuperar_cien_y_si_vuelve_a_cero_vuelve_la_mortalidad() {
        let r = Reglas::cargar().poblacion;
        let (mut p, e, mut s) = (con(30, 0, 0, 20_000), entorno(), vec![]);
        // Toca 0: entra en recuperación y muere el 5 %: 1,5 personas por minuto.
        p.nutrir(&r, &e, 0, &mut s);
        assert!(p.recuperando && p.nutricion == 0);
        assert_eq!((p.pesants, p.restos[0]), (29, -500));
        // Con comida la nutrición sube 5 por minuto: en recuperación con nutrición positiva la población queda quieta (y el resto no cambia).
        for minuto in 1..=19 {
            p.nutrir(&r, &e, 1_000_000, &mut s);
            p.crecer(&r, &e, &mut s);
            assert_eq!((p.enteros(), p.restos[0], p.recuperando), ((29, 0, 0), -500, true), "minuto {minuto}, nutrición {}", p.nutricion);
        }
        // Una hambruna a medias: vuelve a 0 y vuelve la mortalidad, y sigue pendiente llegar a 100.
        p.nutricion = 5_000;
        p.nutrir(&r, &e, 0, &mut s);
        assert_eq!((p.nutricion, p.pesants), (0, 28), "el resto de muertes de antes se aprovecha: −0,5 −1,45 = −1,95 → una muerte");
        for _ in 0..19 {
            p.nutrir(&r, &e, 1_000_000, &mut s);
        }
        assert!(p.recuperando && p.nutricion == 95_000);
        // Con 100 vuelve a crecer en ese mismo minuto.
        p.nutrir(&r, &e, 1_000_000, &mut s);
        assert_eq!((p.nutricion, p.recuperando), (100_000, false));
        let antes = continua(&p);
        p.crecer(&r, &e, &mut s);
        assert!(continua(&p) > antes);
    }

    #[test]
    fn sin_pasar_hambre_crece_con_nutricion_intermedia_como_siempre() {
        let r = Reglas::cargar().poblacion;
        let e = entorno();
        let crecimiento = |nutricion: Mil| {
            let mut p = con(20, 0, 0, nutricion);
            p.crecer(&r, &e, &mut vec![]);
            continua(&p) - 20_000
        };
        assert_eq!(crecimiento(100_000), 2_080);
        assert_eq!(crecimiento(1), 2_080 / 5, "con la nutrición apenas por encima de 0 el factor es 0,2");
        assert!(crecimiento(50_000) > crecimiento(1) && crecimiento(50_000) < crecimiento(100_000));
    }

    #[test]
    fn los_artesanos_y_la_nobleza_aparecen_cuando_se_cumple_su_condicion() {
        let r = Reglas::cargar().poblacion;
        let (mut p, mut e, mut s) = (Poblacion::nueva(&r), entorno(), vec![]);
        p.crecer(&r, &e, &mut s);
        assert_eq!((p.artesanos, p.nobleza), (0, 0), "sin taller ni Palacio no hay clases nuevas");
        e.hay_transformacion = true;
        p.crecer(&r, &e, &mut s);
        assert_eq!(p.artesanos, 1, "el primero llega sin tirada");
        assert_eq!(s, vec![Suceso::PrimerosArtesanos]);
        e.hay_palacio = true;
        e.capacidad_nobles = 80_000;
        p.crecer(&r, &e, &mut s);
        assert_eq!(p.nobleza, 0, "la nobleza pide un mínimo de ciudadanos");
        e.ciudadanos = 3;
        p.crecer(&r, &e, &mut s);
        assert_eq!(p.nobleza, 1);
        assert_eq!(s.last(), Some(&Suceso::PrimerosNobles));
    }

    #[test]
    fn el_techo_recorta_lo_que_se_anade_pero_no_purga() {
        let r = Reglas::cargar().poblacion;
        let (mut p, mut e) = (Poblacion::nueva(&r), entorno());
        e.techo = 21_000;
        p.crecer(&r, &e, &mut vec![]);
        assert_eq!(continua(&p), 21_000, "el bruto de 2,08 se recorta a lo que cabe bajo el techo");
        assert_eq!((p.pesants, p.restos[0]), (21, 0), "y la persona entera entra");
        e.techo = 10_000;
        p.crecer(&r, &e, &mut vec![]);
        assert_eq!(p.total(), 21, "ya por encima del techo no crece, pero tampoco se purga");
    }

    #[test]
    fn la_recaudacion_es_el_espejo_del_consumo_y_la_presion_fiscal_la_sube() {
        let r = Reglas::cargar().poblacion;
        let mut e = entorno();
        let p = con(100, 50, 10, 100_000);
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
