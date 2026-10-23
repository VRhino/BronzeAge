//! La resolución numérica (Combate §3.3 y §4): el motor común de un choque que el servidor calcula con números (sin
//! servidores de batalla). Dos tiradas del generador, **atacante primero**; las ramas sin combate no tocan el generador.
//!
//! Todo en enteros: poder en milésimas (`Escuadra::poder`), factores en ppm. Las bajas y la experiencia se aplican **en el sitio**, sobre las
//! escuadras que el llamador presta; el informe dice qué le pasó a cada una.

use crate::datos::Reglas;
use asentamiento::datos::UNO;
use base::{Mil, Rng};
use militar::escuadra::Escuadra;
use militar::{Carga, EscuadraId, HeroeId, Tropa};

/// El dueño de una escuadra que no es de ningún héroe (carreteros, mineros del pueblo, guarniciones de serie): la maneja la IA del juego.
// DUDA: `Escuadra.heroe` no admite «sin dueño»; se usa el id 0, que el contador de ids de mundo no debe dar a un héroe.
pub const SIN_DUENO: HeroeId = 0;

/// Los dos bandos de un combate. El atacante es quien lo inicia.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lado {
    Atacante,
    Defensor,
}

impl Lado {
    pub fn otro(self) -> Lado {
        match self {
            Lado::Atacante => Lado::Defensor,
            Lado::Defensor => Lado::Atacante,
        }
    }
}

/// Un bando: las escuadras que combaten (se les aplican bajas y experiencia) y los héroes **sanos** que combaten con ellas, que suman
/// `poderPorHeroe` cada uno aunque no lleven ninguna escuadra.
pub struct Bando<'a> {
    pub escuadras: &'a mut [Escuadra],
    pub heroes: &'a [HeroeId],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombateInvalido {
    /// Un bando sin una escuadra con soldados y sin un héroe sano: no hay con quién combatir.
    BandoVacio(Lado),
}

/// Cómo acabó una escuadra.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Baja {
    pub escuadra: EscuadraId,
    pub tropa: Tropa,
    pub antes: i64,
    pub despues: i64,
}

/// Lo que le pasó a un bando.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InformeBando {
    /// Poder con el que entró, ya con su tirada (y la cohesión y las ventajas si defendía).
    pub poder: Mil,
    pub heroes: Vec<HeroeId>,
    pub bajas: Vec<Baja>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Informe {
    pub ganador: Lado,
    pub atacante: InformeBando,
    pub defensor: InformeBando,
    /// `min(poder) / max(poder)` en ppm. Un combate es **digno** si llega a `NIVEL_FACCION.ratioCombateDigno` (sistema 5): ver [`Informe::digno`].
    pub ratio: i64,
    /// La chatarra de las bajas de los dos bandos: se la queda quien gana.
    pub chatarra: Carga,
}

impl Informe {
    pub fn perdedor(&self) -> Lado {
        self.ganador.otro()
    }

    pub fn bando(&self, lado: Lado) -> &InformeBando {
        match lado {
            Lado::Atacante => &self.atacante,
            Lado::Defensor => &self.defensor,
        }
    }

    /// ¿Combate digno? El más débil tiene al menos `ratio_digno` (ppm) del poder del más fuerte. Solo uno digno da experiencia de Facción.
    pub fn digno(&self, ratio_digno: i64) -> bool {
        self.ratio >= ratio_digno
    }

    /// Los héroes del bando que pierde: en el combate con números quedan heridos (Doc 5.16.4).
    pub fn heroes_del_perdedor(&self) -> &[HeroeId] {
        &self.bando(self.perdedor()).heroes
    }
}

/// Número de escuadras con soldados.
fn con_soldados(escuadras: &[Escuadra]) -> impl Iterator<Item = &Escuadra> {
    escuadras.iter().filter(|e| e.con_soldados())
}

/// Suma del poder de las escuadras con soldados, con la cohesión del defensor si se pide (`1 + bonus × (n − 1)` con más de una).
pub fn poder_total(r: &Reglas, escuadras: &[Escuadra], con_cohesion: bool) -> Mil {
    let suma: Mil = con_soldados(escuadras).map(|e| e.poder(&r.militar)).sum();
    let n = con_soldados(escuadras).count() as i64;
    if con_cohesion && n > 1 {
        (suma as i128 * (UNO + r.poder.cohesion * (n - 1)) as i128 / UNO as i128) as Mil
    } else {
        suma
    }
}

/// Poder de un bando antes de la cohesión y de la tirada: sus escuadras más los héroes sanos que combaten.
pub fn poder_de_bando(r: &Reglas, escuadras: &[Escuadra], heroes: usize) -> Mil {
    poder_total(r, escuadras, false) + heroes as i64 * r.poder.poder_por_heroe
}

/// La tirada de un bando: `1 + (2t − 1) × varianza`, con `t` uniforme en `[0, 1)`. **Una** llamada al generador.
pub fn tirada(r: &Reglas, rng: &mut Rng) -> i64 {
    let t = rng.por_debajo(UNO as u64) as i64;
    UNO + (2 * t - UNO) * r.poder.varianza / UNO
}

fn redondear(cantidad: i64, fraccion: i64) -> i64 {
    (cantidad * fraccion + UNO / 2) / UNO
}

/// Aplica a las escuadras con soldados una fracción de bajas (`round(cantidad × fracción)`, sin bajar de 0) y la experiencia del resultado. La
/// escuadra **persiste** a `cantidad` 0. La chatarra de los caídos se suma a `chatarra`.
pub fn aplicar_bajas(r: &Reglas, escuadras: &mut [Escuadra], fraccion: i64, victoria: bool, chatarra: &mut Carga) -> Vec<Baja> {
    let xp = if victoria { r.poder.xp_victoria } else { r.poder.xp_derrota };
    let mut v = vec![];
    for e in escuadras.iter_mut().filter(|e| e.con_soldados()) {
        let antes = e.cantidad;
        let muertos = redondear(antes, fraccion).clamp(0, antes);
        e.con_experiencia(&r.militar, xp);
        e.cantidad = antes - muertos;
        for (rec, n) in r.chatarra.de_muertos(e.tropa, muertos) {
            militar::sumar(chatarra, rec, n);
        }
        v.push(Baja { escuadra: e.id, tropa: e.tropa, antes, despues: e.cantidad });
    }
    v
}

fn bando_vacio(b: &Bando) -> bool {
    con_soldados(b.escuadras).next().is_none() && b.heroes.is_empty()
}

/// Gana el atacante solo si saca **más** que el defensor: el empate exacto es del defensor.
pub fn ganador_de(poder_atacante: Mil, poder_defensor: Mil) -> Lado {
    if poder_atacante > poder_defensor {
        Lado::Atacante
    } else {
        Lado::Defensor
    }
}

/// `resolverCombate`: el motor de encuentro, asedio, saqueo y avanzada.
///
/// ```text
/// jitterA = 1 + (rng·2 − 1) · varianza           // primero el atacante
/// jitterD = 1 + (rng·2 − 1) · varianza
/// poderA  = poderBando(A) × jitterA
/// poderD  = poderBando(D) × (1 + cohesión·(nD − 1)) × jitterD × multiplicadorDefensor × ventajaDefensor
/// gana A solo si poderA > poderD (el empate es del defensor)
/// ```
///
/// `multiplicador_defensor` (ppm) es el del recinto de la plaza asediada (`asentamiento::recintos::multiplicador_defensivo`), 1,0 donde no hay recinto que valga, o la ventaja del
/// aguante en el saqueo de un pueblo. El poder de los héroes entra en la suma antes de la cohesión, la tirada y las ventajas, y la cohesión cuenta solo
/// escuadras: el héroe no cuenta para ella (Combate §3.3).
pub fn resolver_combate(r: &Reglas, atacante: Bando, defensor: Bando, multiplicador_defensor: i64, rng: &mut Rng) -> Result<Informe, CombateInvalido> {
    if bando_vacio(&atacante) {
        return Err(CombateInvalido::BandoVacio(Lado::Atacante));
    }
    if bando_vacio(&defensor) {
        return Err(CombateInvalido::BandoVacio(Lado::Defensor));
    }
    let (ja, jd) = (tirada(r, rng), tirada(r, rng));
    let suma_a = poder_de_bando(r, atacante.escuadras, atacante.heroes.len()) as i128;
    let suma_d = poder_de_bando(r, defensor.escuadras, defensor.heroes.len()) as i128;
    let n_d = con_soldados(defensor.escuadras).count() as i64;
    let cohesion = if n_d > 1 { UNO + r.poder.cohesion * (n_d - 1) } else { UNO };
    let poder_a = (suma_a * ja as i128 / UNO as i128) as Mil;
    let u = UNO as i128;
    let poder_d = (suma_d * cohesion as i128 * jd as i128 * multiplicador_defensor as i128 * r.poder.ventaja_defensor as i128 / (u * u * u * u)) as Mil;

    let ganador = ganador_de(poder_a, poder_d);
    let (menor, mayor) = (poder_a.min(poder_d), poder_a.max(poder_d).max(1_000));
    let ratio = (menor as i128 * u / mayor as i128) as i64;
    let n = &r.numericas;
    // Cuanto más ajustado el combate, más bajas sufre también el ganador; el perdedor siempre pierde más.
    let bajas_ganador = n.bajas_ganador_base + n.bajas_ganador_por_ratio * ratio / UNO;
    let bajas_perdedor = n.bajas_perdedor_base + n.bajas_perdedor_por_ratio * (UNO - ratio) / UNO;
    let fraccion = |lado| if lado == ganador { bajas_ganador } else { bajas_perdedor };

    let mut chatarra = Carga::new();
    let bajas_a = aplicar_bajas(r, atacante.escuadras, fraccion(Lado::Atacante), ganador == Lado::Atacante, &mut chatarra);
    let bajas_d = aplicar_bajas(r, defensor.escuadras, fraccion(Lado::Defensor), ganador == Lado::Defensor, &mut chatarra);
    Ok(Informe {
        ganador,
        atacante: InformeBando { poder: poder_a, heroes: atacante.heroes.to_vec(), bajas: bajas_a },
        defensor: InformeBando { poder: poder_d, heroes: defensor.heroes.to_vec(), bajas: bajas_d },
        ratio,
        chatarra,
    })
}

#[cfg(test)]
pub(crate) mod apoyo {
    use super::*;

    /// Las reglas, cargadas una sola vez para todas las pruebas.
    pub fn r() -> &'static Reglas {
        static R: std::sync::OnceLock<Reglas> = std::sync::OnceLock::new();
        R.get_or_init(Reglas::cargar)
    }

    /// Una escuadra completa de `tropa` (cantidad de catálogo), nivel 1, de un héroe.
    pub fn escuadra(id: EscuadraId, heroe: HeroeId, tropa: Tropa) -> Escuadra {
        let r = r();
        Escuadra::nueva(&r.militar, id, heroe, tropa, r.militar.tropas.tropa(tropa).unidades, Some(1), 0)
    }

    pub fn es_chatarra(rec: asentamiento::Recurso) -> bool {
        use asentamiento::Recurso::*;
        matches!(rec, ChatarraCobre | ChatarraBronce | ChatarraHierro)
    }

    pub fn con_cantidad(mut e: Escuadra, cantidad: i64) -> Escuadra {
        e.cantidad = cantidad;
        e
    }
}

#[cfg(test)]
mod pruebas {
    use super::apoyo::*;
    use super::*;

    fn resolver(r: &Reglas, a: &mut [Escuadra], ha: &[HeroeId], d: &mut [Escuadra], hd: &[HeroeId], mult: i64, semilla: u64) -> Result<Informe, CombateInvalido> {
        resolver_combate(r, Bando { escuadras: a, heroes: ha }, Bando { escuadras: d, heroes: hd }, mult, &mut Rng::nuevo(semilla))
    }

    #[test]
    fn el_poder_de_un_heroe_suma_quince_y_el_de_una_escuadra_lo_que_dice_el_catalogo() {
        let r = r();
        assert_eq!(poder_de_bando(&r, &[], 1), 15_000, "un héroe solo vale 15 (Doc 5.1)");
        let milicia = escuadra(1, 1, Tropa::MiliciaLanceros);
        assert_eq!(poder_de_bando(&r, std::slice::from_ref(&milicia), 0), 50_500, "milicia completa: 25 × 2 con el +1 % del nivel 1");
        assert_eq!(poder_de_bando(&r, std::slice::from_ref(&milicia), 1), 65_500, "el héroe suma además de sus escuadras");
        let compuesto = escuadra(2, 1, Tropa::ArquerosCompuesto);
        assert_eq!(poder_total(&r, std::slice::from_ref(&compuesto), false), 181_800);
    }

    #[test]
    fn la_cohesion_solo_cuenta_con_mas_de_una_escuadra_con_soldados() {
        let r = r();
        let (a, b, c) = (escuadra(1, 1, Tropa::MiliciaLanceros), escuadra(2, 1, Tropa::MiliciaLanceros), con_cantidad(escuadra(3, 1, Tropa::MiliciaLanceros), 0));
        assert_eq!(poder_total(&r, &[a.clone()], true), 50_500);
        assert_eq!(poder_total(&r, &[a.clone(), b.clone()], true), 111_100, "101 000 × 1,1");
        assert_eq!(poder_total(&r, &[a.clone(), b.clone(), c.clone()], true), 111_100, "la aniquilada no cuenta ni suma cohesión");
        assert_eq!(poder_total(&r, &[a, b], false), 101_000);
    }

    #[test]
    fn la_tirada_va_de_085_a_115_y_consume_una_llamada() {
        let r = r();
        let mut rng = Rng::nuevo(7);
        let mut espejo = Rng::nuevo(7);
        for _ in 0..1000 {
            let j = tirada(&r, &mut rng);
            assert!((850_000..1_150_000).contains(&j), "{j}");
            espejo.siguiente();
        }
        assert_eq!(rng.estado(), espejo.estado(), "exactamente una llamada por tirada");
    }

    #[test]
    fn consume_exactamente_dos_llamadas_y_el_atacante_va_primero() {
        let r = r();
        let (mut a, mut d) = ([escuadra(1, 1, Tropa::MiliciaLanceros)], [escuadra(2, 2, Tropa::MiliciaLanceros)]);
        let mut rng = Rng::nuevo(42);
        let mut espejo = Rng::nuevo(42);
        let ja = tirada(&r, &mut espejo);
        let jd = tirada(&r, &mut espejo);
        let inf = resolver_combate(&r, Bando { escuadras: &mut a, heroes: &[] }, Bando { escuadras: &mut d, heroes: &[] }, UNO, &mut rng).unwrap();
        assert_eq!(rng.estado(), espejo.estado());
        assert_eq!(inf.atacante.poder, (50_500i128 * ja as i128 / UNO as i128) as Mil);
        assert_eq!(inf.defensor.poder, (50_500i128 * jd as i128 * 1_050_000 / (UNO as i128 * UNO as i128)) as Mil);
    }

    #[test]
    fn un_bando_vacio_es_un_error_y_no_toca_el_generador() {
        let r = r();
        let mut rng = Rng::nuevo(1);
        let antes = rng.estado();
        let mut a = [escuadra(1, 1, Tropa::MiliciaLanceros)];
        let mut vacia = [con_cantidad(escuadra(2, 2, Tropa::MiliciaLanceros), 0)];
        assert_eq!(resolver_combate(&r, Bando { escuadras: &mut a, heroes: &[] }, Bando { escuadras: &mut vacia, heroes: &[] }, UNO, &mut rng).unwrap_err(), CombateInvalido::BandoVacio(Lado::Defensor));
        assert_eq!(resolver_combate(&r, Bando { escuadras: &mut [], heroes: &[] }, Bando { escuadras: &mut a, heroes: &[] }, UNO, &mut rng).unwrap_err(), CombateInvalido::BandoVacio(Lado::Atacante));
        assert_eq!(rng.estado(), antes);
        assert_eq!(a[0].cantidad, 25);
        // Un héroe sin escuadras sí es un bando.
        assert!(resolver_combate(&r, Bando { escuadras: &mut [], heroes: &[1] }, Bando { escuadras: &mut a, heroes: &[] }, UNO, &mut rng).is_ok());
    }

    #[test]
    fn dos_heroes_solos_se_pelean_y_gana_quien_saca_mas() {
        let r = r();
        let mut resultados = [0, 0];
        for semilla in 0..200 {
            let inf = resolver(&r, &mut [], &[1], &mut [], &[2], UNO, semilla).unwrap();
            resultados[(inf.ganador == Lado::Defensor) as usize] += 1;
            assert!(inf.atacante.bajas.is_empty() && inf.defensor.bajas.is_empty());
        }
        assert!(resultados[0] > 0 && resultados[1] > resultados[0], "{resultados:?}: el defensor tiene +5 % y gana el empate");
    }

    #[test]
    fn el_empate_exacto_es_del_defensor() {
        assert_eq!(ganador_de(10_000, 10_000), Lado::Defensor);
        assert_eq!(ganador_de(10_001, 10_000), Lado::Atacante);
        assert_eq!(ganador_de(9_999, 10_000), Lado::Defensor);
    }

    #[test]
    fn las_bajas_del_ganador_y_del_perdedor_salen_del_ratio() {
        let r = r();
        // 200 contra 50: gana con holgura el atacante; el ratio es bajo y el perdedor pierde casi el 70 %.
        let mut a = [escuadra(1, 1, Tropa::ArquerosCompuesto), escuadra(2, 1, Tropa::ArquerosCompuesto)];
        let mut d = [escuadra(3, 2, Tropa::MiliciaLanceros)];
        let inf = resolver(&r, &mut a, &[], &mut d, &[], UNO, 3).unwrap();
        assert_eq!(inf.ganador, Lado::Atacante);
        let ratio = inf.ratio;
        assert!(ratio < 400_000);
        let esperado_perdedor = 300_000 + 400_000 * (UNO - ratio) / UNO;
        assert_eq!(inf.defensor.bajas[0], Baja { escuadra: 3, tropa: Tropa::MiliciaLanceros, antes: 25, despues: 25 - (25 * esperado_perdedor + UNO / 2) / UNO });
        let esperado_ganador = 50_000 + 150_000 * ratio / UNO;
        assert_eq!(inf.atacante.bajas[0].antes - inf.atacante.bajas[0].despues, (12 * esperado_ganador + UNO / 2) / UNO);
        assert!(inf.defensor.bajas[0].despues < 10 && inf.atacante.bajas[0].despues >= 11);
    }

    #[test]
    fn la_experiencia_es_uno_por_ganar_y_medio_por_perder() {
        let r = r();
        let mut a = [escuadra(1, 1, Tropa::ArquerosCompuesto)];
        let mut d = [escuadra(2, 2, Tropa::MiliciaLanceros)];
        let inf = resolver(&r, &mut a, &[], &mut d, &[], UNO, 5).unwrap();
        assert_eq!(inf.ganador, Lado::Atacante);
        assert_eq!((a[0].experiencia, d[0].experiencia), (1_000, 500));
    }

    #[test]
    fn el_defensor_gana_con_el_multiplicador_y_el_atacante_sin_el() {
        let r = r();
        let (mut a, mut d) = ([escuadra(1, 1, Tropa::LancerosMimbre)], [escuadra(2, 2, Tropa::LancerosMimbre)]);
        let (mut a2, mut d2) = (a.clone(), d.clone());
        let normal = resolver(&r, &mut a, &[], &mut d, &[], UNO, 9).unwrap();
        let con_muralla = resolver(&r, &mut a2, &[], &mut d2, &[], 3_000_000, 9).unwrap();
        assert_eq!(con_muralla.atacante.poder, normal.atacante.poder);
        assert!((con_muralla.defensor.poder - normal.defensor.poder * 3).abs() <= 3, "la muralla multiplica solo al defensor (salvo el redondeo)");
        assert_eq!(con_muralla.ganador, Lado::Defensor);
    }

    #[test]
    fn una_escuadra_aniquilada_no_gana_experiencia_ni_baja_y_las_demas_conservan_identidad() {
        let r = r();
        let mut a = [escuadra(1, 1, Tropa::ArquerosCompuesto), con_cantidad(escuadra(9, 1, Tropa::Hequetai), 0)];
        let mut d = [escuadra(2, 2, Tropa::MiliciaLanceros)];
        let inf = resolver(&r, &mut a, &[], &mut d, &[], UNO, 5).unwrap();
        assert_eq!(inf.atacante.bajas.len(), 1);
        assert_eq!((a[1].cantidad, a[1].experiencia), (0, 0));
    }

    #[test]
    fn la_chatarra_sale_de_las_bajas_de_los_dos_bandos() {
        let r = r();
        let mut a = [escuadra(1, 1, Tropa::EspadachinesBronce)];
        let mut d = [escuadra(2, 2, Tropa::EspadachinesBronce)];
        let inf = resolver(&r, &mut a, &[], &mut d, &[], UNO, 11).unwrap();
        let muertos = |i: &InformeBando| i.bajas.iter().map(|b| b.antes - b.despues).sum::<i64>();
        let total = muertos(&inf.atacante) + muertos(&inf.defensor);
        let esperado: Mil = r.chatarra.de_muertos(Tropa::EspadachinesBronce, muertos(&inf.atacante)).iter().chain(&r.chatarra.de_muertos(Tropa::EspadachinesBronce, muertos(&inf.defensor))).map(|x| x.1).sum();
        assert!(total > 0 && esperado > 0);
        assert_eq!(militar::total(&inf.chatarra), esperado);
        assert!(inf.chatarra.keys().all(|&rec| es_chatarra(rec)));
    }

    #[test]
    fn determinismo_con_semilla_y_conservacion_de_soldados() {
        let r = r();
        for semilla in 0..300 {
            let montar = || ([escuadra(1, 1, Tropa::Hequetai), escuadra(2, 1, Tropa::Arqueros)], [escuadra(3, 2, Tropa::LancerosPesados), escuadra(4, 2, Tropa::Peltastas)]);
            let (mut a1, mut d1) = montar();
            let (mut a2, mut d2) = montar();
            let i1 = resolver(&r, &mut a1, &[1], &mut d1, &[2], UNO, semilla).unwrap();
            let i2 = resolver(&r, &mut a2, &[1], &mut d2, &[2], UNO, semilla).unwrap();
            assert_eq!(i1, i2, "misma semilla, mismo combate");
            assert_eq!(a1, a2);
            for e in a1.iter().chain(&d1) {
                assert!((0..=r.militar.tropas.tropa(e.tropa).unidades).contains(&e.cantidad), "las bajas nunca dejan una cantidad imposible");
            }
            let bajas = |i: &InformeBando| i.bajas.iter().map(|b| b.antes - b.despues).sum::<i64>();
            let vivos: i64 = a1.iter().chain(&d1).map(|e| e.cantidad).sum();
            let antes: i64 = i1.atacante.bajas.iter().chain(&i1.defensor.bajas).map(|b| b.antes).sum();
            assert_eq!(vivos + bajas(&i1.atacante) + bajas(&i1.defensor), antes, "ni se crean ni se pierden soldados fuera del informe");
        }
    }

    #[test]
    fn el_ratio_y_el_combate_digno() {
        let r = r();
        let mut a = [escuadra(1, 1, Tropa::LancerosMimbre)];
        let mut d = [escuadra(2, 2, Tropa::LancerosMimbre)];
        let inf = resolver(&r, &mut a, &[], &mut d, &[], UNO, 1).unwrap();
        let (mn, mx) = (inf.atacante.poder.min(inf.defensor.poder), inf.atacante.poder.max(inf.defensor.poder));
        assert_eq!(inf.ratio, (mn as i128 * UNO as i128 / mx as i128) as i64);
        assert!(inf.digno(0) && !inf.digno(UNO + 1));
        assert_eq!(inf.perdedor(), inf.ganador.otro());
        assert_eq!(inf.heroes_del_perdedor(), inf.bando(inf.perdedor()).heroes.as_slice());
    }

    #[test]
    fn simetria_los_bandos_iguales_dan_el_mismo_poder_salvo_ventaja_y_tirada() {
        let r = r();
        // Sin tirada de por medio (varianza 0 simulada con la misma tirada en los dos): el defensor vale exactamente la ventaja más.
        let (mut a, mut d) = ([escuadra(1, 1, Tropa::Shardana)], [escuadra(2, 2, Tropa::Shardana)]);
        let base = poder_de_bando(&r, &a, 0);
        let inf = resolver(&r, &mut a, &[], &mut d, &[], UNO, 4).unwrap();
        let ja = tirada(&r, &mut Rng::nuevo(4));
        let mut rng = Rng::nuevo(4);
        rng.siguiente();
        let jd = tirada(&r, &mut rng);
        assert_eq!(inf.atacante.poder, (base as i128 * ja as i128 / UNO as i128) as Mil);
        assert_eq!(inf.defensor.poder, (base as i128 * jd as i128 * 1_050_000 / (UNO as i128 * UNO as i128)) as Mil);
    }
}
