//! Lo que da renombre y medallas a un mercenario (BAN54, entrada 48; Mercenarios_Por_Contrato §8.2 y §9): el temporal apunta al acabar lo que ganó bajo contrato (con despido, nada), el
//! permanente lo apunta en el momento; la valoración de quien lo contrató lo cambia; los ataques ganados, las defensas ganadas, las escoltas que llegan, las campañas que se siguen hasta el
//! final y los días de Leal lo dan, y cada rango o medalla que cruzan da su premio y su hecho. Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas se
//! omiten si no está.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::{Cargo, Recurso};
use base::{Instante, Punto, MS_POR_MINUTO};
use combate::ciclo::{Asignacion, EstadoBatalla, Instancia, Token};
use combate::resolucion::Lado;
use combate::resultado::{Razon, Resultado as ResultadoDeBatalla, ResultadoEscuadra, ResultadoHeroe};
use comercio::caravana::{Animal, EstadoCaravana, TipoCarro};
use comun::mercenarios as mc;
use comun::real::{campamento, fundar, pisable, sitio_rico, M};
use mercenarios::contrato::duracion;
use mercenarios::{Ficha, Medalla, Objetivo, Terminado, TipoOrden, Trabajo};
use militar::catalogo::Tropa;
use militar::columna::Politica;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::{rechazos, Comando, Dato, EventoNucleo, Nucleo, ObjetivoAtaque, Peticion, Relacion, Respuesta};
use std::sync::Arc;

macro_rules! con_mundo {
    () => {
        match comun::real::mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

const MIN: Instante = MS_POR_MINUTO;
const HORA: Instante = 60 * MIN;

/// La Facción 1 (humana) con su Rey «rey», que espera con 500 de oro en el carro de su columna a la puerta del campamento 0; su plaza, junto al campamento 3, fundada por «fundador»
/// (Gobernador de oficio, fuera, en su columna), y la plaza de la Facción 2 (de «gus»), junto al campamento 0. Los mercenarios y los demás firmantes los pone cada prueba.
struct E {
    n: Nucleo,
    rey: u64,
    plaza: u64,
    ajena: u64,
}

impl E {
    fn nuevo(m: &M) -> Self {
        Self::sobre(m, mc::nucleo(m))
    }

    fn sobre(m: &M, mut n: Nucleo) -> Self {
        // El primer ciudadano de la Facción 1 es su Rey.
        let rey = mc::firmante(&mut n, "rey", 1, 0, mc::ORO, Controlador::Humano);
        let (sa, sb) = (sitio_rico(&n, &m.0, 3, 1), sitio_rico(&n, &m.0, 0, 2));
        let plaza = fundar(&mut n, "fundador", 1, sa);
        let ajena = fundar(&mut n, "gus", 2, sb);
        E { n, rey, plaza, ajena }
    }

    /// Un bot en el tablón del campamento 0.
    fn bot(&mut self, cuenta: &str) -> u64 {
        mc::bot_ofrecido(&mut self.n, cuenta, 0)
    }

    /// Un ciudadano de la Facción 1 con 500 de oro en el carro a la puerta del campamento 0, sin cargo.
    fn ciudadano(&mut self, cuenta: &str) -> u64 {
        mc::firmante(&mut self.n, cuenta, 1, 0, mc::ORO, Controlador::Humano)
    }

    /// Un Gobernador de la plaza, nombrado por el Rey, a la puerta del campamento 0 con 500 de oro.
    fn gobernador(&mut self, cuenta: &str) -> u64 {
        let g = self.ciudadano(cuenta);
        let (rey, plaza) = (self.rey, self.plaza);
        mc::con_cargo(&mut self.n, "rey", rey, plaza, Cargo::Gobernador, g);
        g
    }

    /// `firmante` contrata, con su cuenta, un trabajo temporal (sin plaza donde residir): la respuesta tal cual.
    fn contratar(&mut self, firmante: u64, mercenario: u64, trabajo: Trabajo, objetivo: Objetivo, plazo: u8) -> Respuesta {
        mc::contratar(&mut self.n, firmante, mercenario, trabajo, objetivo, Some(plazo), None)
    }

    /// Lo mismo, exigiendo que se acepte.
    fn contrata(&mut self, firmante: u64, mercenario: u64, trabajo: Trabajo, objetivo: Objetivo, plazo: u8) {
        let r = self.contratar(firmante, mercenario, trabajo, objetivo, plazo);
        assert!(r.aceptada(), "{trabajo:?} {objetivo:?}: {r:?}");
    }

    /// El Rey contrata a un permanente que reside en la plaza.
    fn permanente(&mut self, mercenario: u64) {
        let (rey, plaza) = (self.rey, self.plaza);
        let r = mc::contratar(&mut self.n, rey, mercenario, Trabajo::Permanente, Objetivo::Faccion(1), None, Some(plaza));
        assert!(r.aceptada(), "{r:?}");
    }

    /// `heroe` despide, con su cuenta, a `mercenario`.
    fn despide(&mut self, heroe: u64, mercenario: u64) {
        let cuenta = self.n.heroe(heroe).expect("quien despide existe").cuenta.clone();
        mc::ok(&mut self.n, &cuenta, Comando::DespedirMercenario { heroe, mercenario });
    }

    /// `heroe` valora, con su cuenta, a `mercenario`: la respuesta tal cual.
    fn valorar(&mut self, heroe: u64, mercenario: u64, buena: bool) -> Respuesta {
        let cuenta = self.n.heroe(heroe).expect("quien valora existe").cuenta.clone();
        mc::dar(&mut self.n, &cuenta, Comando::ValorarMercenario { heroe, mercenario, buena })
    }

    /// Lo mismo, exigiendo que se acepte.
    fn valora(&mut self, heroe: u64, mercenario: u64, buena: bool) {
        let r = self.valorar(heroe, mercenario, buena);
        assert!(r.aceptada(), "{r:?}");
    }

    /// Lo mismo, exigiendo un rechazo que no deje rastro: devuelve su código.
    fn valoracion_rechazada(&mut self, heroe: u64, mercenario: u64, buena: bool) -> &'static str {
        let cuenta = self.n.heroe(heroe).expect("quien valora existe").cuenta.clone();
        mc::rechazo(&mut self.n, &cuenta, Comando::ValorarMercenario { heroe, mercenario, buena })
    }

    /// La ficha del mercenario.
    fn ficha(&self, mercenario: u64) -> Ficha {
        self.n.estado().contratos.fichas[&mercenario].clone()
    }

    /// Lo que queda por valorar de su contrato con la Facción 1.
    fn terminado(&self, mercenario: u64) -> Terminado {
        self.n.estado().contratos.terminados[&(mercenario, 1)]
    }

    /// Los datos de las reglas de los contratos.
    fn datos(&self) -> Arc<mercenarios::Datos> {
        self.n.estado().reglas.economia.contratos.clone()
    }

    /// Cuánto renombre ha apuntado el contrato vigente de `mercenario` sin pasarlo aún a la ficha.
    fn del_contrato(&self, mercenario: u64) -> u32 {
        mc::contrato(&self.n, mercenario).expect("tiene contrato").renombre
    }

    /// El mercenario, hasta que vence su contrato.
    fn hasta_el_fin(&mut self, mercenario: u64) -> Instante {
        let vence = mc::contrato(&self.n, mercenario).and_then(|c| c.vence).expect("un temporal vence");
        self.n.avanzar_hasta(vence);
        vence
    }
}

/// El texto `clave` de un hecho.
fn texto<'a>(h: &'a EventoNucleo, clave: &str) -> Option<&'a str> {
    h.valores.iter().find_map(|(k, v)| match v {
        Dato::Texto(t) if *k == clave => Some(t.as_str()),
        _ => None,
    })
}

/// La cuenta y el grado de una medalla en su ficha.
fn medalla(f: &Ficha, m: Medalla) -> (u32, u8) {
    (f.contadores.get(&m).copied().unwrap_or(0), f.grados.get(&m).copied().unwrap_or(0))
}

/// Las medallas que han dado un grado, en el orden en que se dieron: (mercenario, medalla, grado).
fn medallas_dadas(n: &Nucleo) -> Vec<(u64, String, i64)> {
    mc::hechos(n, "mercenarios.medalla")
        .iter()
        .map(|h| {
            assert_eq!((h.facciones.clone(), h.asentamientos.clone(), h.ejercitos.clone()), (vec![], vec![], vec![]), "lo oye solo el mercenario: {h:?}");
            (h.heroes[0], texto(h, "medalla").expect("lleva su medalla").to_owned(), h.dato("grado").expect("lleva su grado"))
        })
        .collect()
}

/// Un ejército de la Facción 2 con esas tropas asedia la plaza de la Facción 1 (declarando la guerra) y el asedio se resuelve con números al acabar su convocatoria: devuelve ese instante.
/// La plaza ha de haber pasado ya su día de protección.
fn asediar_la_plaza(e: &mut E, tropas: &[Tropa]) -> Instante {
    let pos = e.n.estado().asentamientos[&e.plaza].posicion;
    let atacante = e.n.crear_ejercito(e.n.ahora(), "otro", 2, Punto::new(pos.x + 1_000, pos.y), 500, 350_000).unwrap();
    let lider = e.n.ejercito(atacante).unwrap().columna.lider;
    for &tropa in tropas {
        mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: lider, tropa, cantidad: completa(tropa) });
    }
    let plaza = e.plaza;
    mc::ok(&mut e.n, "otro", Comando::Atacar { heroe: lider, objetivo: ObjetivoAtaque::Asentamiento(plaza), declara_guerra: true });
    let convocatoria = e.n.batallas_activas().filter_map(|b| b.convocatoria_hasta).max().expect("un asedio convoca");
    e.n.avanzar_hasta(convocatoria);
    convocatoria
}

/// Los soldados de una escuadra completa de esa tropa.
fn completa(tropa: Tropa) -> i64 {
    militar::Reglas::cargar().tropas.tropa(tropa).unidades
}

// ───────────────────────────── El fin del contrato ─────────────────────────────

#[test]
fn el_temporal_que_vence_apunta_el_renombre_de_su_contrato_y_cuenta_palabra_cumplida() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    e.contrata(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), 3);
    let (d, c) = (e.datos(), mc::contrato(&e.n, bot).unwrap());
    let del_plazo = d.plazo(3).unwrap().renombre;
    assert_eq!(c.renombre, del_plazo, "el contrato nace con el renombre de su plazo");

    // Mientras dura, el renombre está en el contrato y no en la ficha.
    let vence = c.vence.unwrap();
    e.n.avanzar_hasta(vence - 1);
    assert_eq!((e.ficha(bot).renombre, e.del_contrato(bot)), (0, del_plazo));
    assert!(mc::hechos(&e.n, "mercenarios.medalla").is_empty());

    // Al vencer se apunta, y el plazo cumplido cuenta Palabra cumplida, cuyo bronce da su renombre en el momento.
    e.n.avanzar_hasta(vence);
    let f = e.ficha(bot);
    let bronce = d.renombre.grado_medalla[0];
    assert_eq!((f.renombre, medalla(&f, Medalla::PalabraCumplida)), (del_plazo + bronce, (1, 1)));
    assert_eq!(e.terminado(bot), Terminado { firmado_por: rey, renombre: del_plazo, palabra: true, valorable_hasta: vence + d.valoracion, valorado: false });
    assert_eq!(medallas_dadas(&e.n), vec![(bot, "palabra_cumplida".to_owned(), 1)]);
    assert_eq!(f.historial.terminados, 1);
}

#[test]
fn un_despido_no_apunta_nada_ni_cuenta_palabra_cumplida() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    e.contrata(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), 7);
    assert!(e.del_contrato(bot) > 0, "tenía renombre guardado en el contrato");

    e.despide(rey, bot);
    let f = e.ficha(bot);
    assert_eq!((f.renombre, f.contadores.is_empty(), f.historial.despidos), (0, true, 1));
    let hasta = e.n.ahora() + e.datos().valoracion;
    assert_eq!(e.terminado(bot), Terminado { firmado_por: rey, renombre: 0, palabra: false, valorable_hasta: hasta, valorado: false });
    assert!(mc::hechos(&e.n, "mercenarios.medalla").is_empty());

    // Valorarlo bien no le da renombre (no hay nada apuntado sobre lo que añadir) pero sí cuenta una valoración buena.
    e.valora(rey, bot, true);
    let f = e.ficha(bot);
    assert_eq!((f.renombre, f.historial.buenas, medalla(&f, Medalla::BienValorado)), (0, 1, (1, 0)));
}

#[test]
fn el_fin_por_plaza_perdida_apunta_el_renombre_sin_palabra_cumplida() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let plaza = e.plaza;
    // La plaza nace protegida un día: pasado, se puede asediar.
    e.n.avanzar_hasta(e.n.protegida_hasta(plaza).expect("la fundación la protege"));
    let (bot, gob) = (e.bot("bot-1"), e.gobernador("gob"));
    e.contrata(gob, bot, Trabajo::Defender, Objetivo::Plaza(plaza), 7);
    let del_plazo = e.del_contrato(bot);

    // La Facción 2 conquista la plaza que firmó el contrato.
    let convocatoria = asediar_la_plaza(&mut e, &[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto]);
    assert_eq!(e.n.estado().asentamientos[&plaza].faccion, 2);

    // Acabó sin despido: se apunta lo que traía, pero no es un plazo cumplido.
    assert!(mc::contrato(&e.n, bot).is_none());
    let f = e.ficha(bot);
    assert_eq!((f.renombre, f.contadores.is_empty()), (del_plazo, true));
    assert_eq!(e.terminado(bot), Terminado { firmado_por: gob, renombre: del_plazo, palabra: false, valorable_hasta: convocatoria + e.datos().valoracion, valorado: false });
    assert!(mc::hechos(&e.n, "mercenarios.medalla").is_empty());
    assert_eq!(mc::hechos(&e.n, "mercenarios.terminado")[0].heroes, vec![bot]);
}

// ───────────────────────────── La valoración ─────────────────────────────

/// Los hechos `mercenarios.valorado`: lo oyen la Facción y el mercenario, y nadie más.
fn valoraciones(n: &Nucleo) -> Vec<EventoNucleo> {
    let hechos = mc::hechos(n, "mercenarios.valorado");
    for h in &hechos {
        assert_eq!((h.facciones.clone(), h.asentamientos.clone(), h.ejercitos.clone()), (vec![1], vec![], vec![]), "{h:?}");
        assert_eq!(h.heroes, vec![h.dato("mercenario").expect("nombra al mercenario") as u64], "lo oye el mercenario: {h:?}");
    }
    hechos
}

#[test]
fn una_buena_valoracion_suma_lo_que_anade_buena_pct_a_lo_apuntado_y_cuenta_bien_valorado() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    e.contrata(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), 3);
    e.hasta_el_fin(bot);
    let (d, antes) = (e.datos(), e.ficha(bot));
    let apuntado = e.terminado(bot).renombre;
    let extra = apuntado * (d.buena_pct - 100) / 100;
    assert!(extra > 0);

    e.valora(rey, bot, true);
    let f = e.ficha(bot);
    assert_eq!((f.renombre, f.historial.buenas, f.historial.malas, medalla(&f, Medalla::BienValorado)), (antes.renombre + extra, 1, 0, (1, 0)));
    assert!(e.terminado(bot).valorado);
    assert_eq!(f.contadores[&Medalla::PalabraCumplida], 1, "una buena no toca la Palabra cumplida");
    let h = valoraciones(&e.n);
    assert_eq!(h.len(), 1);
    assert_eq!((h[0].dato("faccion"), h[0].dato("valorador"), h[0].dato("buena"), h[0].dato("renombre")), (Some(1), Some(rey as i64), Some(1), Some(i64::from(extra))));
}

#[test]
fn una_mala_valoracion_quita_lo_que_el_contrato_deja_de_valer_y_la_palabra_cumplida_que_contaba() {
    let m = con_mundo!();
    // Con la mala por defecto (todo lo apuntado) y con una que deja valer el 40 %.
    for mala_pct in [0, 40] {
        let mut e = E::sobre(m, mc::nucleo_con(m, |r| Arc::make_mut(&mut r.economia.contratos).mala_pct = mala_pct));
        let (bot, gob, plaza) = (e.bot("bot-1"), e.gobernador("gob"), e.plaza);
        e.contrata(gob, bot, Trabajo::Defender, Objetivo::Plaza(plaza), 3);
        e.hasta_el_fin(bot);
        let (antes, apuntado) = (e.ficha(bot), e.terminado(bot).renombre);
        let quita = apuntado * (100 - mala_pct) / 100;
        assert_eq!((medalla(&antes, Medalla::PalabraCumplida), quita > 0), ((1, 1), true));

        e.valora(gob, bot, false);
        let f = e.ficha(bot);
        assert_eq!(f.renombre, antes.renombre - quita, "malaPct {mala_pct}");
        // Pierde la cuenta de esa Palabra cumplida, pero el grado ganado se queda, y con él su renombre.
        assert_eq!((f.historial.malas, f.historial.buenas, medalla(&f, Medalla::PalabraCumplida)), (1, 0, (0, 1)));
        let h = valoraciones(&e.n);
        assert_eq!((h.len(), h[0].dato("buena"), h[0].dato("renombre"), h[0].dato("valorador")), (1, Some(0), Some(i64::from(quita)), Some(gob as i64)));
    }
}

#[test]
fn una_mala_valoracion_de_un_contrato_sin_palabra_cumplida_no_toca_esa_medalla() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    // Un contrato que cuenta Palabra cumplida, y otro (de un despido) que no.
    e.contrata(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), 1);
    e.hasta_el_fin(bot);
    mc::ok(&mut e.n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });
    e.contrata(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), 1);
    e.despide(rey, bot);
    assert_eq!((medalla(&e.ficha(bot), Medalla::PalabraCumplida), e.terminado(bot).palabra), ((1, 1), false));

    e.valora(rey, bot, false);
    assert_eq!(medalla(&e.ficha(bot), Medalla::PalabraCumplida), (1, 1));
}

#[test]
fn solo_valora_quien_firmo_o_el_rey_una_vez_y_mientras_esta_abierto() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, otro, rey, plaza) = (e.bot("bot-1"), e.bot("bot-2"), e.rey, e.plaza);
    let (gob, sin_cargo) = (e.gobernador("gob"), e.ciudadano("ana"));
    let rey_de_la_2 = e.n.faccion(2).and_then(|f| f.rey).expect("la Facción 2 tiene Rey");
    // «bot» lo firma el Gobernador y «otro», el Rey, los dos por un día.
    e.contrata(gob, bot, Trabajo::Defender, Objetivo::Plaza(plaza), 1);
    e.contrata(rey, otro, Trabajo::Defender, Objetivo::Plaza(plaza), 1);
    let (sin_valoracion, sin_alcance) = (rechazos::MERCENARIO_SIN_VALORACION, rechazos::MERCENARIO_SIN_ALCANCE);

    // Mientras dura el contrato, no hay nada que valorar.
    assert_eq!(e.valoracion_rechazada(gob, bot, true), sin_valoracion);
    let fin = e.hasta_el_fin(bot);
    let hasta = fin + e.datos().valoracion;

    // Quien no lo firmó ni es el Rey no alcanza (ni el Gobernador al contrato del Rey); el Rey de otra Facción no tiene nada de ese mercenario que valorar.
    assert_eq!(e.valoracion_rechazada(sin_cargo, bot, true), sin_alcance);
    assert_eq!(e.valoracion_rechazada(gob, otro, true), sin_alcance);
    assert_eq!(e.valoracion_rechazada(rey_de_la_2, bot, true), sin_valoracion);
    assert!(valoraciones(&e.n).is_empty());

    // El Rey valora un contrato que firmó otro, y una sola vez: la segunda, aunque cambie de opinión, no hay nada abierto.
    e.valora(rey, bot, true);
    assert_eq!(e.valoracion_rechazada(rey, bot, false), sin_valoracion);
    assert_eq!(e.valoracion_rechazada(gob, bot, false), sin_valoracion);
    assert_eq!((valoraciones(&e.n).len(), e.ficha(bot).historial.malas), (1, 0));

    // Sigue abierta hasta el último instante: un instante antes se rechaza por alcance, que se mira después; a su hora, ya no hay valoración.
    e.n.avanzar_hasta(hasta - 1);
    assert_eq!(e.valoracion_rechazada(sin_cargo, otro, true), sin_alcance);
    e.n.avanzar_hasta(hasta);
    assert_eq!(e.valoracion_rechazada(rey, otro, true), sin_valoracion);
    assert_eq!(e.valoracion_rechazada(sin_cargo, otro, true), sin_valoracion);
    assert_eq!(valoraciones(&e.n).len(), 1);
}

// ───────────────────────────── Atacar ─────────────────────────────

/// Un claro cerca del campamento 0, a salvo de su protección (junto a un campamento de mercenarios nadie inicia un combate).
fn claro(e: &E, m: &M) -> Punto {
    pisable(&m.0, campamento(&m.0, 0), e.n.estado().reglas.economia.militar.mercenarios.radio_proteccion + 8_000, 0)
}

/// Un campamento de bandidos de ese nivel lo más cerca que se pueda de `centro` sin caer en una zona de influencia ni junto a un campamento de mercenarios.
fn bandidos(e: &mut E, m: &M, centro: Punto, nivel: u8) -> u64 {
    for radio in (0..40).map(|k| 4_000 + 2_000 * k) {
        for grado in (0..360).step_by(45) {
            let a = f64::from(grado).to_radians();
            let p = pisable(&m.0, Punto::new(centro.x + (radio as f64 * a.cos()) as i64, centro.y + (radio as f64 * a.sin()) as i64), 0, 0);
            let r = mc::dar(&mut e.n, "admin", Comando::ColocarBandidos { posicion: p, nivel });
            if r.aceptada() {
                return r.creadas[0];
            }
        }
    }
    panic!("sin sitio para unos bandidos cerca de {centro:?}");
}

/// El Rey contrata siete días a un mercenario («bot-1») para atacar `objetivo`; éste trae después sus tropas (para no encarecer la tarifa), forma su ejército con una convocatoria de uno
/// (un ejército solo combate con otro ejército), parte y marcha hasta junto a `destino`. Devuelve su id.
fn enviar_a_atacar(e: &mut E, m: &M, objetivo: Objetivo, tropas: &[Tropa], destino: Punto) -> u64 {
    let (rey, bot) = (e.rey, e.bot("bot-1"));
    e.contrata(rey, bot, Trabajo::Atacar, objetivo, 7);
    let escuadras: Vec<u64> = tropas.iter().map(|&t| mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: bot, tropa: t, cantidad: completa(t) }).creadas[0]).collect();
    mc::ok(&mut e.n, "bot-1", Comando::ConvocarEjercito { heroe: bot, escuadras, carga: vec![], politica: Politica::Aceptar });
    let columna = mc::ok(&mut e.n, "bot-1", Comando::PartirConvocatoria { heroe: bot, caravana_fundacion: None }).creadas[0];
    mc::ok(&mut e.n, "bot-1", Comando::MarcharA { ejercito: columna, destino: pisable(&m.0, destino, 1_000, 0) });
    let llegada = e.n.ejercito(columna).unwrap().plan.fin().expect("marcha");
    e.n.avanzar_hasta(llegada);
    bot
}

#[test]
fn destruir_unos_bandidos_da_el_renombre_de_su_nivel_y_cuenta_cazador_y_perder_no_da_nada() {
    let m = con_mundo!();
    // Una escuadra de lanceros destruye un campamento de nivel 1 y no puede con el de nivel 3; con tres escuadras, uno de nivel 2 cae.
    let fuertes = [Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto, Tropa::MiliciaLanceros];
    for (nivel, tropas, destruye) in [(1, &[Tropa::MiliciaLanceros][..], true), (2, &fuertes[..], true), (3, &[Tropa::MiliciaLanceros][..], false)] {
        let mut e = E::nuevo(m);
        let donde = claro(&e, m);
        let campamento = bandidos(&mut e, m, donde, nivel);
        let sitio = e.n.campamento_de_bandidos(campamento).unwrap().posicion;
        let bot = enviar_a_atacar(&mut e, m, Objetivo::Bandidos(campamento), tropas, sitio);
        let (d, antes) = (e.datos(), e.del_contrato(bot));
        assert_eq!(e.n.estado().contratos.contratos[&bot].orden.map(|o| o.tipo), Some(TipoOrden::Atacar));

        let r = mc::dar(&mut e.n, "bot-1", Comando::Atacar { heroe: bot, objetivo: ObjetivoAtaque::Campamento(campamento), declara_guerra: false });
        assert!(r.aceptada(), "{r:?}");
        assert_eq!(e.n.campamento_de_bandidos(campamento).is_none(), destruye, "nivel {nivel}");
        let f = e.ficha(bot);
        if destruye {
            // El renombre queda en el contrato hasta que acabe (es un temporal); la medalla da el suyo en el momento.
            assert_eq!(e.del_contrato(bot), antes + d.renombre.atacar.bandidos_por_nivel * u32::from(nivel));
            assert_eq!(medalla(&f, Medalla::CazadorDeBandidos), (1, 0));
            assert_eq!(f.renombre, 0);
        } else {
            assert_eq!((e.del_contrato(bot), f.renombre, f.contadores.is_empty()), (antes, 0, true), "nivel {nivel}");
        }
        assert_eq!(mc::hechos(&e.n, "mercenarios.orden_cumplida").len(), 1, "la orden se cumple se gane o no");
    }
}

#[test]
fn tomar_una_plaza_con_la_orden_atacar_da_el_renombre_de_su_clase_y_cuenta_asaltante() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let ajena = e.ajena;
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    e.n.avanzar_hasta(e.n.protegida_hasta(ajena).expect("la fundación la protege"));
    let sitio = e.n.estado().asentamientos[&ajena].posicion;
    let bot = enviar_a_atacar(&mut e, m, Objetivo::Plaza(ajena), &[Tropa::MiliciaLanceros], sitio);
    let (d, antes) = (e.datos(), e.del_contrato(bot));

    // Nadie defiende la plaza con soldados: el asedio la toma al acabar su convocatoria.
    mc::ok(&mut e.n, "bot-1", Comando::Atacar { heroe: bot, objetivo: ObjetivoAtaque::Asentamiento(ajena), declara_guerra: false });
    let convocatoria = e.n.batallas_activas().filter_map(|b| b.convocatoria_hasta).max().expect("un asedio convoca");
    assert_eq!(e.del_contrato(bot), antes, "mientras se asedia no ha ganado nada");
    e.n.avanzar_hasta(convocatoria);
    assert_eq!(e.n.estado().asentamientos[&ajena].faccion, 1, "la plaza cambió de dueño");
    let f = e.ficha(bot);
    assert_eq!(e.del_contrato(bot), antes + d.renombre.atacar.plaza);
    assert_eq!(medalla(&f, Medalla::Asaltante), (1, 1));
    assert_eq!(f.renombre, d.renombre.grado_medalla[0], "solo el de la medalla, que va al momento");
    assert_eq!(medallas_dadas(&e.n), vec![(bot, "asaltante".to_owned(), 1)]);
}

// ───────────────────────────── Defender ─────────────────────────────

/// Un permanente de la plaza que reside en ella y está dentro, con esas tropas en su loadout activo: defiende en persona.
fn defensor_de_la_plaza(e: &mut E, cuenta: &str, tropas: &[Tropa]) -> u64 {
    let (bot, plaza) = (e.bot(cuenta), e.plaza);
    e.permanente(bot);
    mc::ok(&mut e.n, "admin", Comando::ColocarHeroe { heroe: bot, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
    let escuadras: Vec<u64> = tropas.iter().map(|&tropa| mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: bot, tropa, cantidad: completa(tropa) }).creadas[0]).collect();
    let loadout = e.n.heroe(bot).unwrap().heroe.loadouts[0].id;
    mc::ok(&mut e.n, "admin", Comando::GuardarLoadout { heroe: bot, loadout: Some(loadout), nombre: "Default".into(), escuadras, activo: Some(true) });
    bot
}

#[test]
fn defender_la_plaza_con_guardar_da_defensa_ganada_al_momento_al_permanente_y_cuenta_muralla_y_perderla_no_da_nada() {
    let m = con_mundo!();
    // Tres escuadras dentro resisten a una que asedia; una sola cae ante tres.
    let fuertes = [Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto, Tropa::MiliciaLanceros];
    for (defensa, ataque, resiste) in [(&fuertes[..], &[Tropa::MiliciaLanceros][..], true), (&[Tropa::MiliciaLanceros][..], &fuertes[..], false)] {
        let mut e = E::nuevo(m);
        let (plaza, d) = (e.plaza, e.datos());
        // Con el trigo a 2/3 (2026-11-11) la Granja sola no da para un día de guarnición (desertaría entera): el defensor entra al acabar la protección,
        // con trigo de sobra para su ración.
        e.n.avanzar_hasta(e.n.protegida_hasta(plaza).expect("la fundación la protege"));
        let bot = defensor_de_la_plaza(&mut e, "bot-1", defensa);
        mc::ok(&mut e.n, "admin", Comando::DarRecursos { asentamiento: plaza, recurso: Recurso::Trigo, cantidad: 500_000 });
        assert_eq!(e.n.estado().contratos.contratos[&bot].orden.map(|o| (o.tipo, o.objetivo)), Some((TipoOrden::Guardar, Objetivo::Plaza(plaza))));

        asediar_la_plaza(&mut e, ataque);
        assert_eq!(e.n.estado().asentamientos[&plaza].faccion == 1, resiste);
        let f = e.ficha(bot);
        if resiste {
            // Es un permanente: apunta en el momento, y su contrato no guarda nada. Una Muralla no basta aún para su bronce.
            assert_eq!((f.renombre, e.del_contrato(bot), medalla(&f, Medalla::Muralla)), (d.renombre.defensa_ganada, 0, (1, 0)));
            assert!(mc::hechos(&e.n, "mercenarios.medalla").is_empty());
        } else {
            assert_eq!((f.renombre, f.contadores.is_empty()), (0, true), "la plaza cayó");
        }

        // Despedido después, no hay nada que apuntar ni que quitar: ya lo tiene.
        if resiste {
            let rey = e.rey;
            e.despide(rey, bot);
            assert_eq!(e.ficha(bot).renombre, d.renombre.defensa_ganada);
            assert_eq!((e.terminado(bot).renombre, e.terminado(bot).palabra), (0, false));
        }
    }
}

// ───────────────────────────── Rangos ─────────────────────────────

#[test]
fn el_renombre_que_apunta_el_contrato_sube_de_rango_y_da_el_premio_de_ese_rango() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey, plaza) = (e.bot("bot-1"), e.rey, e.plaza);
    let d = e.datos();
    // A 90 de renombre, le faltan 10 para el rango 2 (lanza): el contrato de 3 días trae 25 y la Palabra cumplida, 10 más.
    let (n, _dir) = mc::retocado(m, &e.n, |s| s.contratos.fichas.get_mut(&bot).unwrap().renombre = d.rangos[1].renombre - 10);
    e.n = n;
    e.contrata(rey, bot, Trabajo::Defender, Objetivo::Plaza(plaza), 3);
    let antes = e.ficha(bot);
    assert_eq!((antes.rango, antes.premios, antes.renombre), (1, 0, d.rangos[1].renombre - 10));

    e.hasta_el_fin(bot);
    let f = e.ficha(bot);
    let del_plazo = d.plazo(3).unwrap().renombre;
    assert_eq!(f.renombre, d.rangos[1].renombre - 10 + del_plazo + d.renombre.grado_medalla[0]);
    assert_eq!((f.rango, f.premios), (2, 1), "cruzó el rango 2 con el renombre del contrato y ganó su escuadra de premio");
    let h = mc::hechos(&e.n, "mercenarios.rango_subido");
    assert_eq!((h.len(), h[0].dato("rango"), h[0].heroes.clone()), (1, Some(2), vec![bot]));
}

// ───────────────────────────── Leal ─────────────────────────────

/// El instante en que se firmó el contrato de `mercenario`.
fn firmado_en(e: &E, mercenario: u64) -> Instante {
    mc::contrato(&e.n, mercenario).expect("tiene contrato").firmado_en
}

#[test]
fn un_permanente_cuenta_leal_a_los_siete_dias_y_a_los_treinta() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, d) = (e.bot("bot-1"), e.datos());
    e.permanente(bot);
    let t0 = firmado_en(&e, bot);
    let [bronce, plata, _] = d.umbrales(Medalla::Leal);

    // Un instante antes de los siete días, nada; a los siete, la cuenta de siete con su bronce y el renombre de ese grado.
    e.n.avanzar_hasta(t0 + duracion(bronce as u8) - 1);
    assert!(medalla(&e.ficha(bot), Medalla::Leal) == (0, 0) && mc::hechos(&e.n, "mercenarios.medalla").is_empty());
    e.n.avanzar_hasta(t0 + duracion(bronce as u8));
    let f = e.ficha(bot);
    assert_eq!((medalla(&f, Medalla::Leal), f.renombre), ((bronce, 1), d.renombre.grado_medalla[0]));
    assert_eq!(medallas_dadas(&e.n), vec![(bot, "leal".to_owned(), 1)]);

    // A los treinta, la plata: cuenta los 23 días que le faltaban, no treinta más.
    e.n.avanzar_hasta(t0 + Instante::from(plata) * duracion(1));
    let f = e.ficha(bot);
    assert_eq!((medalla(&f, Medalla::Leal), f.renombre), ((plata, 2), d.renombre.grado_medalla[0] + d.renombre.grado_medalla[1]));
    assert_eq!(medallas_dadas(&e.n), vec![(bot, "leal".to_owned(), 1), (bot, "leal".to_owned(), 2)]);
}

#[test]
fn un_permanente_despedido_y_vuelto_a_firmar_no_cuenta_los_dias_del_contrato_anterior() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (bot, rey) = (e.bot("bot-1"), e.rey);
    e.permanente(bot);
    let primero = firmado_en(&e, bot);
    let siete = duracion(7);
    // Lo despide a los dos días y, pasada la espera de recontrato (tres días), lo firma de nuevo: la cita de los siete días del primero cae a los dos del segundo y no cuenta.
    e.n.avanzar_hasta(primero + duracion(2));
    e.despide(rey, bot);
    e.n.avanzar_hasta(primero + duracion(2) + e.datos().espera_recontrato);
    mc::ok(&mut e.n, "bot-1", Comando::OfrecerseComoMercenario { heroe: bot });
    e.permanente(bot);
    let segundo = firmado_en(&e, bot);
    e.n.avanzar_hasta(primero + siete + 1);
    assert_eq!(medalla(&e.ficha(bot), Medalla::Leal), (0, 0));
    // Los siete días del segundo contrato, sí.
    e.n.avanzar_hasta(segundo + siete);
    assert_eq!(medalla(&e.ficha(bot), Medalla::Leal), (7, 1));
    assert_eq!(medallas_dadas(&e.n), vec![(bot, "leal".to_owned(), 1)]);
}

// ───────────────────────────── Escoltar ─────────────────────────────

/// Los Mercados de las dos plazas en pie y una caravana de la Facción 1 con un buey, fuera del reparto automático.
fn caravana_para_comerciar(e: &mut E) -> u64 {
    let (plaza, ajena) = (e.plaza, e.ajena);
    for p in [plaza, ajena] {
        mc::ok(&mut e.n, "admin", Comando::DarRecursos { asentamiento: p, recurso: Recurso::Madera, cantidad: 350_000 });
        mc::ok(&mut e.n, "admin", Comando::AnadirEdificio { asentamiento: p, tipo: Tipo::Mercado });
    }
    e.n.avanzar_hasta(e.n.ahora() + 9 * HORA);
    for recurso in [Recurso::Cobre, Recurso::Madera] {
        // Sin exigir que entre: tras 9 horas la madera puede estar ya en su tope.
        let _ = mc::dar(&mut e.n, "admin", Comando::DarRecursos { asentamiento: plaza, recurso, cantidad: 100_000 });
    }
    let k = mc::ok(&mut e.n, "admin", Comando::CrearCaravana { asentamiento: plaza }).creadas[0];
    mc::ok(&mut e.n, "admin", Comando::ReservarCaravana { caravana: k, reservada: true });
    mc::ok(&mut e.n, "admin", Comando::AnadirCarro { caravana: k, tipo: TipoCarro::Basico });
    mc::ok(&mut e.n, "admin", Comando::AnadirAnimal { caravana: k, carro: 0, animal: Animal::Buey });
    k
}

#[test]
fn una_caravana_que_llega_da_el_renombre_de_la_escolta_y_cuenta_buen_camino_a_quien_la_guarda_con_su_orden() {
    let m = con_mundo!();
    // El escolta cede una milicia a la caravana; en el segundo caso, antes de que salga el Rey le manda otra cosa y ya no la guarda.
    for cambia_de_orden in [false, true] {
        let mut e = E::nuevo(m);
        let (rey, plaza, ajena) = (e.rey, e.plaza, e.ajena);
        let k = caravana_para_comerciar(&mut e);
        let bot = e.bot("bot-1");
        e.contrata(rey, bot, Trabajo::Escoltar, Objetivo::Caravana(k), 7);
        mc::ok(&mut e.n, "admin", Comando::FijarResidencia { heroe: bot, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
        mc::ok(&mut e.n, "admin", Comando::ColocarHeroe { heroe: bot, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
        let milicia = mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: bot, tropa: Tropa::MiliciaLanceros, cantidad: 20 }).creadas[0];
        mc::ok(&mut e.n, "bot-1", Comando::AsignarEscolta { caravana: k, heroe: bot, escuadras: vec![milicia] });
        if cambia_de_orden {
            let destino = Objetivo::Punto(pisable(&m.0, campamento(&m.0, 0), 30_000, 0));
            mc::ok(&mut e.n, "rey", Comando::OrdenarMercenario { heroe: rey, mercenarios: vec![bot], tipo: TipoOrden::IrA, objetivo: destino });
        }
        let (d, antes) = (e.datos(), e.del_contrato(bot));
        assert_eq!(antes, d.plazo(7).unwrap().renombre);

        mc::ok(&mut e.n, "admin", Comando::PrepararCaravana { caravana: k, destino: ajena, carga: vec![(Recurso::Cobre, 10_000)], salir_en: None, escolta: None });
        let llegada = e.n.caravana(k).and_then(|c| c.plan.as_ref()?.fin()).expect("sale y viaja");
        e.n.avanzar_hasta(llegada - 1);
        assert_eq!(e.del_contrato(bot), antes, "mientras viaja no ha ganado nada");
        e.n.avanzar_hasta(llegada);
        assert_ne!(e.n.caravana(k).map(|c| c.estado), Some(EstadoCaravana::EnTransito), "la caravana llegó");

        let f = e.ficha(bot);
        if cambia_de_orden {
            assert_eq!((e.del_contrato(bot), f.contadores.is_empty(), f.renombre), (antes, true, 0), "ya no la guardaba");
        } else {
            assert_eq!((e.del_contrato(bot), medalla(&f, Medalla::BuenCamino), f.renombre), (antes + d.renombre.escolta_llegada, (1, 0), 0));
        }
    }
}

// ───────────────────────────── Acompañar: Campaña ─────────────────────────────

/// Un Líder de la Facción 1 («capi») con una escuadra y su convocatoria abierta a quien quiera en el campamento 0: (héroe, convocatoria).
fn convocatoria_de_capi(e: &mut E) -> (u64, u64) {
    let lider = mc::heroe_en(&mut e.n, "capi", 0, Controlador::Humano);
    mc::ok(&mut e.n, "admin", Comando::FijarFaccionDeHeroe { heroe: lider, faccion: Some(1) });
    let escuadra = mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: lider, tropa: Tropa::MiliciaLanceros, cantidad: 10 }).creadas[0];
    let convocatoria = mc::ok(&mut e.n, "capi", Comando::ConvocarEjercito { heroe: lider, escuadras: vec![escuadra], carga: vec![], politica: Politica::Aceptar }).creadas[0];
    (lider, convocatoria)
}

/// El mercenario que sirve a la cuenta «bot-k» se une, con su cuenta, a esa convocatoria.
fn se_une(e: &mut E, cuenta: &str, bot: u64, convocatoria: u64) {
    mc::ok(&mut e.n, cuenta, Comando::UnirseAConvocatoria { heroe: bot, convocatoria, escuadras: vec![], carga: vec![] });
}

#[test]
fn cancelar_una_convocatoria_cuenta_campana_a_quien_la_acompanaba_desde_dentro_y_a_nadie_mas() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let rey = e.rey;
    let (lider, convocatoria) = convocatoria_de_capi(&mut e);
    let (dentro, fuera, salio) = (e.bot("bot-1"), e.bot("bot-2"), e.bot("bot-3"));
    for b in [dentro, fuera, salio] {
        e.contrata(rey, b, Trabajo::Acompanar, Objetivo::Convocatoria(convocatoria), 7);
    }
    // «dentro» se une y sigue; «salio» se une y se va antes del final; «fuera» nunca entra.
    se_une(&mut e, "bot-1", dentro, convocatoria);
    se_une(&mut e, "bot-3", salio, convocatoria);
    mc::ok(&mut e.n, "bot-3", Comando::SepararseDeConvocatoria { heroe: salio });
    assert!(medallas_dadas(&e.n).is_empty());

    mc::ok(&mut e.n, "capi", Comando::CancelarConvocatoria { heroe: lider });
    assert!(e.n.convocatoria(convocatoria).is_none());
    let d = e.datos();
    let (f, otros) = (e.ficha(dentro), [e.ficha(fuera), e.ficha(salio)]);
    assert_eq!((medalla(&f, Medalla::Campana), f.renombre), ((1, 1), d.renombre.grado_medalla[0]));
    assert!(otros.iter().all(|f| medalla(f, Medalla::Campana) == (0, 0) && f.renombre == 0), "{otros:?}");
    assert_eq!(medallas_dadas(&e.n), vec![(dentro, "campana".to_owned(), 1)]);
}

#[test]
fn disolverse_un_ejercito_cuenta_campana_al_mercenario_que_lo_acompano_hasta_el_final() {
    /// Lo que tarda en salir del mundo quien se desconecta.
    const SALE_A_LOS: i64 = 150_000;
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let rey = e.rey;
    let (lider, convocatoria) = convocatoria_de_capi(&mut e);
    let bot = e.bot("bot-1");
    e.contrata(rey, bot, Trabajo::Acompanar, Objetivo::Convocatoria(convocatoria), 7);
    se_une(&mut e, "bot-1", bot, convocatoria);
    // Parte: el contrato y la orden siguen, ahora sobre el ejército.
    let ejercito = mc::ok(&mut e.n, "capi", Comando::PartirConvocatoria { heroe: lider, caravana_fundacion: None }).creadas[0];
    assert_eq!(mc::contrato(&e.n, bot).map(|c| c.objetivo), Some(Objetivo::Ejercito(ejercito)));

    // El Líder se desconecta y sale del mundo: queda el mercenario, con el mando, y el ejército sigue.
    let t = e.n.ahora();
    mc::ok(&mut e.n, "capi", Comando::Desconectarse { heroe: lider });
    e.n.avanzar_hasta(t + SALE_A_LOS);
    assert!(e.n.ejercito(ejercito).is_some_and(|x| x.columna.participa(bot) && !x.columna.participa(lider)));
    assert_eq!(medalla(&e.ficha(bot), Medalla::Campana), (0, 0));

    // Se desconecta él, solo: el ejército se disuelve y, con él, la campaña acaba.
    mc::ok(&mut e.n, "bot-1", Comando::Desconectarse { heroe: bot });
    e.n.avanzar_hasta(t + 2 * SALE_A_LOS);
    assert!(e.n.ejercito(ejercito).is_none(), "se disolvió");
    assert_eq!(medalla(&e.ficha(bot), Medalla::Campana), (1, 1));
    assert_eq!(medallas_dadas(&e.n), vec![(bot, "campana".to_owned(), 1)]);
}

#[test]
fn desarmar_un_ejercito_a_la_puerta_de_un_lugar_cuenta_campana_a_quien_lo_acompano_y_no_a_quien_salio_antes() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let rey = e.rey;
    let (lider, convocatoria) = convocatoria_de_capi(&mut e);
    let (bot, otro) = (e.bot("bot-1"), e.bot("bot-2"));
    for (cuenta, b) in [("bot-1", bot), ("bot-2", otro)] {
        e.contrata(rey, b, Trabajo::Acompanar, Objetivo::Convocatoria(convocatoria), 7);
        se_une(&mut e, cuenta, b, convocatoria);
    }
    let ejercito = mc::ok(&mut e.n, "capi", Comando::PartirConvocatoria { heroe: lider, caravana_fundacion: None }).creadas[0];
    // «bot-2» se separa antes de acabar: su orden caduca y no sigue hasta el final.
    mc::ok(&mut e.n, "bot-2", Comando::SepararseDelEjercito { heroe: otro });

    // El Líder lo lleva a la puerta del campamento y entra: el ejército se desarma y la campaña acaba.
    mc::ok(&mut e.n, "capi", Comando::EntrarEnCampamento { campamento: 0, heroe: lider });
    assert_eq!(medalla(&e.ficha(bot), Medalla::Campana), (1, 1));
    assert_eq!(medalla(&e.ficha(otro), Medalla::Campana), (0, 0));
    assert_eq!(medallas_dadas(&e.n), vec![(bot, "campana".to_owned(), 1)]);
    assert!(e.n.ejercito(ejercito).is_none_or(|x| !x.columna.participa(bot)), "se desarmó");
}

// ───────────────────────────── El fin de quien convoca para atacar ─────────────────────────────

/// En guerra la Facción 1 con la 2, un mercenario («bot-1») contratado `plazo` días para atacar la plaza ajena convoca un ejército con una escuadra suya: (mercenario, convocatoria).
fn convoca_para_atacar(e: &mut E, plazo: u8) -> (u64, u64) {
    let (rey, ajena) = (e.rey, e.ajena);
    mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
    let bot = e.bot("bot-1");
    e.contrata(rey, bot, Trabajo::Atacar, Objetivo::Plaza(ajena), plazo);
    let escuadra = mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: bot, tropa: Tropa::MiliciaLanceros, cantidad: completa(Tropa::MiliciaLanceros) }).creadas[0];
    let convocatoria = mc::ok(&mut e.n, "bot-1", Comando::ConvocarEjercito { heroe: bot, escuadras: vec![escuadra], carga: vec![], politica: Politica::Aceptar }).creadas[0];
    assert!(e.n.convocatoria(convocatoria).is_some_and(|c| c.lider == bot));
    (bot, convocatoria)
}

#[test]
fn si_acaba_el_contrato_de_quien_lidera_una_convocatoria_esta_se_cancela_y_quien_iba_en_ella_cuenta_campana() {
    let m = con_mundo!();
    // Despedido el mercenario Líder, o vencido su plazo: el mismo desenlace, sin pánico.
    for despido in [true, false] {
        let mut e = E::nuevo(m);
        let rey = e.rey;
        let (lider, convocatoria) = convoca_para_atacar(&mut e, 3);
        let acompanante = e.bot("bot-2");
        e.contrata(rey, acompanante, Trabajo::Acompanar, Objetivo::Convocatoria(convocatoria), 7);
        se_une(&mut e, "bot-2", acompanante, convocatoria);
        assert_eq!(e.n.convocatoria(convocatoria).map(|c| c.integrantes.len()), Some(2));

        if despido {
            e.despide(rey, lider);
        } else {
            e.hasta_el_fin(lider);
        }
        assert!(mc::contrato(&e.n, lider).is_none());
        assert!(e.n.convocatoria(convocatoria).is_none(), "despido {despido}: la convocatoria se cancela con su Líder");
        let h = mc::hechos(&e.n, "convocatoria.cancelada");
        assert_eq!((h.len(), h[0].dato("convocatoria"), h[0].dato("lider")), (1, Some(convocatoria as i64), Some(lider as i64)));
        // El contrato de quien iba en ella sigue, sin orden vigente sobre lo que ya no existe; la campaña cuenta.
        assert!(mc::contrato(&e.n, acompanante).is_some());
        assert_eq!(medalla(&e.ficha(acompanante), Medalla::Campana), (1, 1), "despido {despido}");
        assert_eq!(medalla(&e.ficha(lider), Medalla::Campana), (0, 0));
    }
}

#[test]
fn si_acaba_el_contrato_de_quien_iba_en_una_convocatoria_solo_se_separa_de_ella() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let rey = e.rey;
    let (lider, convocatoria) = convocatoria_de_capi(&mut e);
    let bot = e.bot("bot-1");
    e.contrata(rey, bot, Trabajo::Acompanar, Objetivo::Convocatoria(convocatoria), 7);
    se_une(&mut e, "bot-1", bot, convocatoria);
    e.despide(rey, bot);
    // La convocatoria sigue, con su Líder, y no se cuenta Campaña: quien se va antes del final no la hizo.
    assert!(e.n.convocatoria(convocatoria).is_some_and(|c| c.lider == lider && c.integrantes.iter().all(|i| i.heroe != bot)));
    assert!(mc::hechos(&e.n, "convocatoria.cancelada").is_empty());
    assert_eq!(medalla(&e.ficha(bot), Medalla::Campana), (0, 0));
}

// ───────────────────────────── Con servidores de batalla (Unity) ─────────────────────────────

/// Un mensaje del servidor de batalla `srv`.
fn msg(n: &mut Nucleo, t: Instante, id: &str, comando: Comando) -> Respuesta {
    n.aplicar_en(t, Peticion::de_servidor_de_batalla("srv", id, comando))
}

/// La asignación `a1` de la batalla en su revisión, con un token para cada humano.
fn asignacion(revision: u32, humanos: &[u64]) -> Asignacion {
    Asignacion {
        intento: "a1".into(),
        servidor: "ignorado: lo pone el actor".into(),
        revision,
        instancia: Instancia { host: "10.0.0.1".into(), puerto: 7777, protocolo: "udp".into() },
        tokens: humanos.iter().map(|&heroe| Token { heroe, token: format!("t{heroe}"), expira_en: 9_999_999 }).collect(),
    }
}

/// Un resultado válido de la batalla `id`: de cada escuadra mueren dos soldados, gana `ganador` y todos los héroes participan.
fn resultado(n: &Nucleo, id: u64, ganador: Lado) -> ResultadoDeBatalla {
    let b = n.batalla(id).unwrap();
    ResultadoDeBatalla {
        resultado_id: format!("res-{id}"),
        revision: b.ticket.revision,
        intento: "a1".into(),
        inicio: 1_000,
        fin: 61_000,
        ganador,
        razon: Razon::ObjetivosCapturados,
        objetivos: vec![("bandera".into(), Some(ganador))],
        por_escuadra: b
            .escuadras_de()
            .map(|e| {
                let m = 2.min(e.efectivos);
                ResultadoEscuadra { escuadra: e.id, desplegados: e.efectivos, supervivientes_al_cierre: e.efectivos - m, muertos: m }
            })
            .collect(),
        por_heroe: b.heroes_que_combaten().map(|h| ResultadoHeroe { heroe: h, participo: true, sobrevivio_al_cierre: true, derrotado: None, botin: None }).collect(),
        version_servidor: "v1".into(),
        carros_perdidos: vec![],
    }
}

#[test]
fn con_servidores_de_batalla_el_ataque_da_su_renombre_al_aplicarse_el_resultado_solo_si_gana_el_atacante() {
    let m = con_mundo!();
    for ganador in [Lado::Atacante, Lado::Defensor] {
        let mut e = E::sobre(m, mc::nucleo_con(m, |r| r.batallas_en_unity = true));
        mc::ok(&mut e.n, "admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(Relacion::Guerra) });
        // Un ejército de la Facción 2 en un claro, con un humano al mando (así la batalla se juega en Unity), y el mercenario que lo ataca.
        let donde = claro(&e, m);
        let ejercito = e.n.crear_ejercito(e.n.ahora(), "beto", 2, donde, 500, 100_000).unwrap();
        let rival = e.n.ejercito(ejercito).unwrap().columna.lider;
        mc::ok(&mut e.n, "admin", Comando::DarEscuadra { heroe: rival, tropa: Tropa::MiliciaLanceros, cantidad: completa(Tropa::MiliciaLanceros) });
        let bot = enviar_a_atacar(&mut e, m, Objetivo::Ejercito(ejercito), &[Tropa::EspadachinesBronce], donde);
        mc::ok(&mut e.n, "admin", Comando::FijarControlador { heroe: rival, controlador: Controlador::Humano });
        let (d, antes) = (e.datos(), e.del_contrato(bot));

        mc::ok(&mut e.n, "bot-1", Comando::Atacar { heroe: bot, objetivo: ObjetivoAtaque::Ejercito(ejercito), declara_guerra: false });
        let id = e.n.batallas().next().expect("se abrió una batalla").id;
        assert_eq!(e.del_contrato(bot), antes, "mientras se juega no ha ganado nada");
        let t = e.n.ahora() + MIN;
        let revision = e.n.batalla(id).unwrap().ticket.revision;
        assert!(msg(&mut e.n, t, "as", Comando::RegistrarAsignacion { batalla: id, asignacion: asignacion(revision, &[rival]) }).aceptada());
        assert!(msg(&mut e.n, t, "in", Comando::ConfirmarInicio { batalla: id, intento: "a1".into(), revision }).aceptada());
        assert_eq!(e.n.batalla(id).unwrap().estado, EstadoBatalla::EnCurso);
        let res = resultado(&e.n, id, ganador);
        assert!(msg(&mut e.n, t + MIN, "res", Comando::AplicarResultado { batalla: id, resultado: res }).aceptada());

        let gana = ganador == Lado::Atacante;
        assert_eq!(e.del_contrato(bot), antes + if gana { d.renombre.atacar.ejercito } else { 0 }, "{ganador:?}");
    }
}
