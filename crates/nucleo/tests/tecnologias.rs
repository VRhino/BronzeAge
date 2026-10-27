//! La aparición, la adopción, los campamentos y las citas de cada descubrimiento en el núcleo (sistema 6, tanda 6C; Tecnologia_Aedas §3-§6.1 y §8). Sobre el artefacto de mundo real
//! (`generador/salida/mundo/mundo-*.bin`; las pruebas se omiten si no está). Portan los casos de `typescript-final:src/session/__tests__/comandosTecnologia.test.ts` (la adopción y sus
//! rechazos), `engine/__tests__/tecnologia.test.ts` (aparición y Era cerrada), `canteria.test.ts` (la Cantera rinde ×1,5), `reclutamientoMercenario.test.ts`
//! (`tecnologiasDelCampamento` por plazo) y la parte pública de `cronica.test.ts` (los descubrimientos cantados); las reglas puras están en el crate `tecnologia` (6A).
//!
//! Para no esperar semanas, las pruebas que lo necesitan cambian en las reglas el logro de las tecnologías que miran por `plazasEnNivel.2` con umbral 1: lo cumple
//! `FijarNivelAsentamiento` a nivel 2. Los hitos y las demás cifras son las de Tecnologia_Aedas salvo donde se dice.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::edificio::Edificios;
use asentamiento::Recurso;
use base::{Instante, MS_POR_MINUTO};
use comun::real::{self, M};
use militar::catalogo::Tropa;
use nucleo::{entidad_tecnologia, rechazos, Ajustes, Arranque, Carga, Clase, Comando, Dato, Estado, EventoNucleo, Nucleo, Peticion, Reglas, Resultado, SucesoTecnologia};
use tecnologia::datos::{Condicion, Logro};
use tecnologia::{Contador, TecId};

const MIN: Instante = MS_POR_MINUTO;
const HORA: Instante = 60 * MIN;
const DIA: Instante = 24 * HORA;

macro_rules! con_mundo {
    () => {
        match real::mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

/// Las reglas por defecto sin bandidos (estas pruebas miden otra cosa) y con los cambios de `tec` en el catálogo de tecnologías.
fn reglas(tec: impl FnOnce(&mut tecnologia::Reglas)) -> Reglas {
    let mut r = comun::reglas();
    let b = &mut std::sync::Arc::make_mut(&mut r.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    let t = std::sync::Arc::make_mut(&mut r.economia.tecnologia);
    tec(t);
    t.indexar();
    r
}

/// Con almacenes de 100.000 por recurso: ni la piedra de la plaza llena el suyo (y la Cantera no para) ni falta sitio para la tarifa.
fn con_almacen_grande(mut r: Reglas) -> Reglas {
    std::sync::Arc::make_mut(&mut r.economia.datos).capacidad_inicial = 100_000_000;
    r
}

/// El logro de `id` pasa a ser `plazasEnNivel.2 ≥ 1`.
fn con_logro_de_nivel_2(r: &mut tecnologia::Reglas, id: &str) {
    let t = r.buscar(id).unwrap();
    r.catalogo[t as usize].logro = Some(Logro { contador: Contador::PlazasEnNivel2, umbral: 1 });
}

fn arranque(m: &M, reglas: Reglas, ajustes: Ajustes) -> Arranque {
    Arranque { reglas, ajustes, semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

struct Escenario {
    n: Nucleo,
    plaza: u64,
    /// El Rey de la Facción 1 (el héroe de «ana», que la fundó).
    rey: u64,
    k: u32,
}

impl Escenario {
    /// Una plaza de la Facción 1 recién fundada por «ana», su Rey.
    fn nuevo(m: &M, reglas: Reglas) -> Self {
        Self::en(Nucleo::nuevo(arranque(m, reglas, Ajustes::default())), m)
    }

    fn en(mut n: Nucleo, m: &M) -> Self {
        let p = real::sitio_rico(&n, &m.0, 0, 1);
        let plaza = real::fundar(&mut n, "ana", 1, p);
        let rey = n.heroes().find(|h| h.cuenta == "ana").unwrap().heroe.id;
        assert_eq!(n.facciones().find(|f| f.id == 1).and_then(|f| f.rey), Some(rey));
        Escenario { n, plaza, rey, k: 0 }
    }

    fn ord(&mut self, actor: &str, c: Comando) -> nucleo::Respuesta {
        self.k += 1;
        let id = format!("o{}", self.k);
        let p = if actor == "admin" { Peticion::nueva("admin", &id, c).de_admin() } else { Peticion::nueva(actor, &id, c) };
        self.n.aplicar_en(self.n.ahora(), p)
    }

    fn ok(&mut self, actor: &str, c: Comando) {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
    }

    fn rechazo(&mut self, actor: &str, c: Comando) -> String {
        let v = self.n.version();
        match self.ord(actor, c).resultado {
            Resultado::Rechazada { codigo, .. } => {
                assert_eq!(self.n.version(), v, "un rechazo no deja rastro");
                codigo.to_string()
            }
            otro => panic!("se esperaba un rechazo: {otro:?}"),
        }
    }

    fn tec(&self, id: &str) -> TecId {
        self.n.estado().reglas.economia.tecnologia.buscar(id).unwrap()
    }

    fn aparecida(&self, f: u32, id: &str) -> bool {
        let r = &self.n.estado().reglas.economia.tecnologia;
        self.n.tecnologia().de(r, f).aparecidas.contiene(self.tec(id))
    }

    fn adoptar(&self, id: &str) -> Comando {
        Comando::AdoptarTecnologia { heroe: self.rey, faccion: 1, tecnologia: self.tec(id) }
    }

    fn cantera_activa(&self) -> bool {
        self.n.economia(self.plaza).unwrap().edificios.activos(Tipo::Cantera) > 0
    }

    /// Avanza hora a hora hasta que la plaza tiene su Cantera activa (la levanta la auto-construcción).
    fn hasta_la_cantera(&mut self, tope: Instante) {
        while !self.cantera_activa() {
            assert!(self.n.ahora() < tope, "la plaza no levantó su Cantera a tiempo");
            let t = self.n.ahora() + HORA;
            self.n.avanzar_hasta(t);
        }
    }
}

fn hechos(n: &Nucleo, codigo: &str) -> Vec<EventoNucleo> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).cloned().collect()
}

fn texto(h: &EventoNucleo, clave: &str) -> Option<String> {
    h.valores.iter().find(|(k, _)| *k == clave).and_then(|(_, v)| if let Dato::Texto(s) = v { Some(s.clone()) } else { None })
}

fn cronica(n: &Nucleo, tipo: &str) -> Vec<EventoNucleo> {
    hechos(n, "cronica.entrada").into_iter().filter(|h| texto(h, "tipo").as_deref() == Some(tipo)).collect()
}

/// La versión en la que se emitió el único hecho de ese código para esa tecnología.
fn version_de(n: &Nucleo, codigo: &str, tecnologia: &str) -> u64 {
    let v: Vec<u64> = n.cambios_desde(0).unwrap().filter(|c| c.eventos.iter().any(|e| e.codigo == codigo && texto(e, "tecnologia").as_deref() == Some(tecnologia))).map(|c| c.version).collect();
    assert_eq!(v.len(), 1, "{codigo} {tecnologia}: {v:?}");
    v[0]
}

// ───────────────────────────── 1. La aparición por disparadores ─────────────────────────────

/// La cantería y el bronce de calidad militar (Era II, con el hito de la Cantera para la prueba) con el logro de nivel 2, y la Era I de 3 días.
fn reglas_de_la_aparicion() -> Reglas {
    reglas(|r| {
        con_logro_de_nivel_2(r, "canteria");
        con_logro_de_nivel_2(r, "bronce_calidad_militar");
        let b = r.buscar("bronce_calidad_militar").unwrap();
        r.catalogo[b as usize].hito = vec![Condicion::Edificio { tipo: Tipo::Cantera, nivel: 1 }];
        r.eras[0].plazo = 3 * DIA;
    })
}

#[test]
fn un_logro_sin_hito_no_hace_aparecer_nada_el_edificio_que_lo_cumple_si_en_su_version_y_la_era_cerrada_espera_a_que_empiece() {
    let m = con_mundo!();
    let mut e = Escenario::nuevo(m, reglas_de_la_aparicion());
    assert!(!e.cantera_activa(), "la plaza nace sin Cantera");
    // El logro: se fija y se canta, pero sin el hito no aparece nada.
    let plaza = e.plaza;
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 2 });
    let logros = hechos(&e.n, "tecnologia.logro");
    assert_eq!(logros.len(), 2, "cantería y bronce, los dos de `plazasEnNivel.2 ≥ 1`");
    assert!(logros.iter().all(|h| h.publico && texto(h, "contador").as_deref() == Some("plazasEnNivel.2") && h.dato("umbral") == Some(1)));
    assert!(hechos(&e.n, "tecnologia.aparece").is_empty());
    // A la crónica, con el contador y el umbral (nunca la tecnología que abre) y sin Facciones.
    let c = cronica(&e.n, "tecnologia.logro");
    assert_eq!(c.len(), 2);
    assert!(c.iter().all(|h| h.publico && h.dato("umbral") == Some(1) && texto(h, "contador").as_deref() == Some("plazasEnNivel.2") && texto(h, "tecnologia").is_none()));

    // La Cantera completada es el disparador: la cantería aparece en la misma versión, atribuida a la capital y como primera del mundo; el bronce no (su Era no ha empezado).
    e.hasta_la_cantera(3 * DIA);
    let aparece = hechos(&e.n, "tecnologia.aparece");
    assert_eq!(aparece.len(), 1);
    let a = &aparece[0];
    assert_eq!((texto(a, "tecnologia").as_deref(), a.dato("faccion"), a.dato("primera"), a.asentamientos.clone(), a.publico), (Some("canteria"), Some(1), Some(1), vec![plaza], false));
    let completada = e.n.cambios_desde(0).unwrap().find(|c| c.eventos.iter().any(|h| h.codigo == "construccion.edificio_completado" && h.dato("tipo") == Some(Tipo::Cantera as u8 as i64))).unwrap().version;
    assert_eq!(version_de(&e.n, "tecnologia.aparece", "canteria"), completada, "en la versión de la obra que la cumple");
    assert!(e.aparecida(1, "canteria") && !e.n.tiene_tecnologia(1, "canteria"), "aparecida, no adoptada");
    assert!(!e.aparecida(1, "bronce_calidad_militar"), "una tecnología de una Era cerrada no aparece aunque su logro y su hito se cumplan");
    let p = e.n.tecnologia().primeros[e.tec("canteria") as usize].unwrap();
    assert_eq!((p.faccion, p.en), (1, a.en));

    // Con la Era II (por plazo), el bronce aparece en la versión de `era.comienza`.
    e.n.avanzar_hasta(3 * DIA);
    let eras = hechos(&e.n, "era.comienza");
    assert_eq!((eras.len(), eras[0].dato("porPlazo"), eras[0].en), (1, Some(1), 3 * DIA));
    assert!(e.aparecida(1, "bronce_calidad_militar"));
    assert_eq!(version_de(&e.n, "tecnologia.aparece", "bronce_calidad_militar"), e.n.cambios_desde(0).unwrap().find(|c| c.eventos.iter().any(|h| h.codigo == "era.comienza")).unwrap().version);
    let c = cronica(&e.n, "era.comienza");
    assert_eq!((c.len(), c[0].dato("orden"), c[0].dato("porPlazo"), texto(&c[0], "era").as_deref()), (1, Some(2), Some(1), Some("crisis_adaptacion")));
    // Aparecer es para siempre y no se repite.
    e.n.avanzar_hasta(4 * DIA);
    assert_eq!(hechos(&e.n, "tecnologia.aparece").len(), 2);
}

#[test]
fn el_recurso_en_la_capital_lo_mira_el_paso_de_minuto_de_la_capital() {
    let m = con_mundo!();
    // La aleación del bronce con el logro de nivel 2 y, para la prueba, solo su condición de estaño en la capital.
    let r = reglas(|r| {
        con_logro_de_nivel_2(r, "aleacion_bronce");
        let b = r.buscar("aleacion_bronce").unwrap();
        r.catalogo[b as usize].hito = vec![Condicion::RecursoEnCapital(Recurso::Estano)];
    });
    let mut e = Escenario::nuevo(m, r);
    let plaza = e.plaza;
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 2 });
    e.n.avanzar_hasta(10 * MIN);
    assert!(!e.aparecida(1, "aleacion_bronce"), "sin estaño no aparece");
    // Dar estaño a mitad de minuto no es un disparador: aparece en el siguiente paso de minuto de la capital.
    let dado = e.n.ahora() + 30_000;
    e.n.avanzar_hasta(dado);
    e.ok("admin", Comando::DarRecursos { asentamiento: plaza, recurso: Recurso::Estano, cantidad: 5_000 });
    assert!(!e.aparecida(1, "aleacion_bronce"));
    e.n.avanzar_hasta(dado + MIN);
    let a = hechos(&e.n, "tecnologia.aparece");
    assert_eq!(a.len(), 1);
    assert!(a[0].en > dado && a[0].en <= dado + MIN && (a[0].en - e.n.estado().asentamientos[&plaza].fundado_en) % MIN == 0, "en el paso de minuto: {}", a[0].en);
}

// ───────────────────────────── 2. Las citas de un descubrimiento ─────────────────────────────

/// La cantería aparecida a la Facción 1 (su logro de nivel 2 y la Cantera de la plaza), con su descubridora.
fn con_canteria(m: &M) -> (Escenario, Instante) {
    con_canteria_y(m, |_| {})
}

/// Lo mismo, con más cambios en las reglas de tecnología y almacenes grandes.
fn con_canteria_y(m: &M, mas: impl FnOnce(&mut tecnologia::Reglas)) -> (Escenario, Instante) {
    let mut e = Escenario::nuevo(
        m,
        con_almacen_grande(reglas(|r| {
            con_logro_de_nivel_2(r, "canteria");
            mas(r);
        })),
    );
    let plaza = e.plaza;
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 2 });
    e.hasta_la_cantera(5 * DIA);
    let en = e.n.tecnologia().primeros[e.tec("canteria") as usize].expect("descubierta").en;
    (e, en)
}

#[test]
fn los_aedas_cantan_el_descubrimiento_una_vez_al_pasar_el_retraso_nombrando_a_la_descubridora_y_entra_en_la_cronica() {
    let m = con_mundo!();
    let (mut e, en) = con_canteria(m);
    let retraso = e.n.estado().reglas.economia.tecnologia.aedas.retraso_conocimiento;
    assert_eq!(retraso, DIA);
    e.n.avanzar_hasta(en + retraso - 1);
    assert!(hechos(&e.n, "aedas.canta_descubrimiento").is_empty() && e.n.tecnologia().cantadas.is_empty());
    e.n.avanzar_hasta(en + retraso);
    let cantos = hechos(&e.n, "aedas.canta_descubrimiento");
    assert_eq!(cantos.len(), 1);
    let c = &cantos[0];
    assert_eq!((c.en, c.publico, c.dato("faccion"), texto(c, "tecnologia").as_deref()), (en + retraso, true, Some(1), Some("canteria")));
    assert!(e.n.tecnologia().cantadas.contiene(e.tec("canteria")));
    let entrada = cronica(&e.n, "aedas.canta_descubrimiento");
    assert_eq!(entrada.len(), 1);
    assert_eq!((texto(&entrada[0], "tecnologia").as_deref(), entrada[0].valores.iter().find(|(k, _)| *k == "faccionIds").map(|(_, v)| v.clone())), (Some("canteria"), Some(Dato::Lista(vec![Dato::Texto("f-1".into())]))));
    e.n.avanzar_hasta(en + 4 * DIA);
    assert_eq!(hechos(&e.n, "aedas.canta_descubrimiento").len(), 1, "una sola vez");
}

#[test]
fn cada_descubrimiento_deja_dos_citas_y_al_vencer_no_queda_ninguna() {
    let m = con_mundo!();
    let (mut e, en) = con_canteria(m);
    let canteria = e.tec("canteria");
    let citas = |n: &Nucleo| {
        let mut v: Vec<(Instante, Carga)> = n.estado().cola.volcar().into_iter().filter(|x| x.clave.entidad == entidad_tecnologia(canteria)).map(|x| (x.instante, x.carga)).collect();
        v.sort_by_key(|x| x.0);
        v
    };
    assert_eq!(citas(&e.n), vec![(en + DIA, Carga::Tecnologia(SucesoTecnologia::Conocimiento)), (en + 72 * HORA, Carga::Tecnologia(SucesoTecnologia::Campamento))]);
    // Nadie más programa nada por la tecnología: ni el resto del catálogo ni las Facciones.
    let de_tecnologia = |n: &Nucleo| n.estado().cola.volcar().into_iter().filter(|x| matches!(x.carga, Carga::Tecnologia(_))).count();
    assert_eq!(de_tecnologia(&e.n), 2);
    e.n.avanzar_hasta(en + 72 * HORA);
    assert_eq!(de_tecnologia(&e.n), 0, "vencidas, no se vuelven a programar");
}

#[test]
fn el_campamento_desbloquea_a_las_72_horas_de_la_descubridora_aunque_nadie_la_tenga_adoptada() {
    let m = con_mundo!();
    let mut e = Escenario::nuevo(m, reglas(|_| {}));
    let ncamp = m.0.campamentos().len() as u32;
    let ofrecen = |n: &Nucleo| (0..ncamp).filter(|&c| n.tropas_del_campamento(c).unwrap().contains(&Tropa::LancerosMimbre)).count();
    // La administración la da (descubridora: la Facción 1, ahora) y se la quita: aparecida para siempre, sin adoptar.
    e.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["escudos_ligeros".into()] });
    let en = e.n.ahora();
    assert!(ofrecen(&e.n) > 0, "con la mitad de las vivas adoptándola, al instante");
    e.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec![] });
    assert!(!e.n.tiene_tecnologia(1, "escudos_ligeros") && e.aparecida(1, "escudos_ligeros"));
    assert_eq!(ofrecen(&e.n), 0, "sin adoptar y antes del plazo, no");
    e.n.avanzar_hasta(en + 72 * HORA - 1);
    assert_eq!(ofrecen(&e.n), 0);
    e.n.avanzar_hasta(en + 72 * HORA);
    assert!(ofrecen(&e.n) > 0, "a las 72 h, sí");
    // La cita marca los campamentos (cambia lo que ofrecen) en su instante.
    let cita = e.n.cambios_desde(0).unwrap().find(|c| c.en == en + 72 * HORA && !c.tocados.de(Clase::Campamento).is_empty()).expect("la cita toca los campamentos");
    assert_eq!(cita.tocados.de(Clase::Campamento).len(), ncamp as usize);
}

// ───────────────────────────── 3. La adopción ─────────────────────────────

#[test]
fn el_rey_en_la_capital_adopta_la_aparecida_y_la_paga_el_almacen_de_la_capital() {
    let m = con_mundo!();
    // La tarifa de la Era I pasa a 1.000 de oro: más de lo que la plaza ha recaudado en estos días.
    let (mut e, _) = con_canteria_y(m, |r| r.eras[0].tarifa = vec![(Recurso::Oro, 1_000_000)]);
    let (plaza, rey) = (e.plaza, e.rey);
    // El Rey está en su columna, a la puerta: fuera de la capital.
    assert_eq!(e.rechazo("ana", e.adoptar("canteria")), rechazos::TECNOLOGIA_REY_FUERA_DE_LA_CAPITAL);
    e.ok("ana", Comando::Guarnecer { asentamiento: plaza, heroe: rey });
    // Sin la tarifa de su Era en el almacén de la capital, no; y no se cobra nada.
    let almacen = |e: &Escenario, r: Recurso| e.n.economia(plaza).unwrap().almacen.cantidad(r);
    let (oro, madera) = (almacen(&e, Recurso::Oro), almacen(&e, Recurso::Madera));
    assert!(oro < 1_000_000, "{oro}");
    assert_eq!(e.rechazo("ana", e.adoptar("canteria")), rechazos::TECNOLOGIA_SIN_TARIFA);
    assert_eq!((almacen(&e, Recurso::Oro), almacen(&e, Recurso::Madera)), (oro, madera));
    e.ok("admin", Comando::DarRecursos { asentamiento: plaza, recurso: Recurso::Oro, cantidad: 1_000_000 });
    let (oro, madera) = (almacen(&e, Recurso::Oro), almacen(&e, Recurso::Madera));
    let t = e.n.ahora();
    e.ok("ana", e.adoptar("canteria"));
    assert!(e.n.tiene_tecnologia(1, "canteria"));
    assert_eq!((almacen(&e, Recurso::Oro), almacen(&e, Recurso::Madera)), (oro - 1_000_000, madera), "la tarifa de su Era, en unidades enteras, y nada más");
    let h = hechos(&e.n, "tecnologia.adoptada");
    assert_eq!(h.len(), 1);
    assert_eq!((h[0].en, h[0].dato("faccion"), h[0].dato("capital"), h[0].asentamientos.clone(), texto(&h[0], "tecnologia").as_deref(), h[0].publico), (t, Some(1), Some(plaza as i64), vec![plaza], Some("canteria"), false));
    assert!(h[0].facciones.contains(&1), "lo oye su Facción");
    // Los demás rechazos, en el orden del TypeScript, sin dejar rastro.
    assert_eq!(e.rechazo("ana", e.adoptar("canteria")), rechazos::TECNOLOGIA_YA_ADOPTADA);
    assert_eq!(e.rechazo("ana", e.adoptar("leva_comunal")), rechazos::TECNOLOGIA_YA_ADOPTADA, "las de arranque ya lo están");
    assert_eq!(e.rechazo("ana", e.adoptar("metalurgia_cobre")), rechazos::TECNOLOGIA_NO_APARECIDA);
    assert_eq!(e.rechazo("ana", Comando::AdoptarTecnologia { heroe: rey, faccion: 1, tecnologia: 200 }), rechazos::TECNOLOGIA_DESCONOCIDA);
    assert_eq!(e.rechazo("ana", Comando::AdoptarTecnologia { heroe: rey, faccion: 99, tecnologia: 2 }), rechazos::FACCION_NO_EXISTE);
    // Quien no es el Rey, no; tampoco el héroe de otra cuenta.
    let beto = e.n.crear_ejercito(e.n.ahora(), "beto", 1, real::en_tierra(&m.0, e.n.estado().asentamientos[&plaza].posicion, 20_000, 0), 500, 100_000).unwrap();
    let beto = e.n.ejercito(beto).unwrap().columna.lider;
    assert_eq!(e.rechazo("beto", Comando::AdoptarTecnologia { heroe: beto, faccion: 1, tecnologia: e.tec("canteria") }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.rechazo("beto", e.adoptar("canteria")), rechazos::NO_AUTORIZADO);
    // Una Facción sin plazas no tiene capital.
    let carla = e.n.crear_ejercito(e.n.ahora(), "carla", 2, real::en_tierra(&m.0, e.n.estado().asentamientos[&plaza].posicion, -20_000, 0), 500, 100_000).unwrap();
    let carla = e.n.ejercito(carla).unwrap().columna.lider;
    assert_eq!(e.rechazo("carla", Comando::AdoptarTecnologia { heroe: carla, faccion: 2, tecnologia: e.tec("canteria") }), rechazos::TECNOLOGIA_SIN_CAPITAL);
}

#[test]
fn con_la_canteria_adoptada_las_canteras_de_la_faccion_rinden_un_50_por_ciento_mas() {
    let m = con_mundo!();
    let (mut sin, _) = con_canteria(m);
    let (mut con, _) = con_canteria(m);
    assert_eq!(sin.n.hash_estado(), con.n.hash_estado(), "el mismo mundo");
    con.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["canteria".into()] });
    let desde = sin.n.ahora();
    let piedra = |n: &Nucleo| {
        let e = n.tecnologia();
        e.contador(Contador::ExtraidoPiedra) * 1_000 + e.restos[Contador::ExtraidoPiedra.indice()]
    };
    let (a0, b0) = (piedra(&sin.n), piedra(&con.n));
    sin.n.avanzar_hasta(desde + HORA);
    con.n.avanzar_hasta(desde + HORA);
    let (a, b) = (piedra(&sin.n) - a0, piedra(&con.n) - b0);
    assert!(a > 0, "la Cantera saca piedra");
    assert!((b * 100).abs_diff(a * 150) <= a * 2, "×1,5: sin {a}, con {b}");
}

// ───────────────────────────── 4. La foto y el determinismo ─────────────────────────────

#[test]
fn la_foto_ban38_lleva_las_citas_de_cada_descubrimiento_y_ban37_se_rechaza() {
    let m = con_mundo!();
    let (e, _) = con_canteria(m);
    let reglas = e.n.estado().reglas.clone();
    let mundo_hash = e.n.estado().mundo_hash.clone();
    let bytes = e.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN45");
    let vuelta = Estado::deserializar(&bytes, &reglas, &mundo_hash).unwrap();
    assert_eq!(vuelta.serializar(), bytes);
    assert_eq!(vuelta.hash(), e.n.estado().hash());
    let citas: Vec<Carga> = vuelta.cola.volcar().into_iter().filter(|x| x.clave.entidad == entidad_tecnologia(e.tec("canteria"))).map(|x| x.carga).collect();
    assert_eq!(citas.len(), 2, "las dos citas viajan en la cola");
    let vieja = [b"BAN37".as_slice(), &bytes[5..]].concat();
    let error = Estado::deserializar(&vieja, &reglas, &mundo_hash).err().expect("BAN37 se rechaza");
    assert!(error.0.contains("BAN37") && error.0.contains("citas"), "{}", error.0);
}

/// Funda, cumple el logro de la cantería y da y quita los escudos (dos descubrimientos con sus citas), y avanza 4 días: cubre la aparición, las dos citas de cada una y el canto.
fn guion(m: &M, n: Nucleo, avanzar: &dyn Fn(&mut Nucleo, Instante)) -> Nucleo {
    let mut e = Escenario::en(n, m);
    let plaza = e.plaza;
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 2 });
    e.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec!["escudos_ligeros".into()] });
    e.ok("admin", Comando::FijarTecnologias { faccion: 1, tecnologias: vec![] });
    avanzar(&mut e.n, 4 * DIA);
    assert!(e.aparecida(1, "canteria") && e.n.tecnologia().cantadas.contiene(e.tec("canteria")), "el guion hizo lo que se espera");
    e.n
}

#[test]
fn la_aparicion_y_las_citas_dan_el_mismo_mundo_de_una_vez_a_trozos_y_tras_una_foto() {
    let m = con_mundo!();
    let r = || reglas(|r| con_logro_de_nivel_2(r, "canteria"));
    let nuevo = || Nucleo::nuevo(arranque(m, r(), Ajustes::default()));
    let de_una = guion(m, nuevo(), &|n, t| n.avanzar_hasta(t));
    let a_trozos = guion(m, nuevo(), &|n, t| {
        let desde = n.ahora();
        for s in 1..=7 {
            n.avanzar_hasta(desde + (t - desde) * s * s / 49);
        }
    });
    assert_eq!(de_una.hash_estado(), a_trozos.hash_estado(), "en pasos pequeños e irregulares, como el modo en vivo");
    // Con una foto entre el descubrimiento y sus citas, recuperado sigue igual que el que no se interrumpió.
    let ajustes = Ajustes { foto_cada: 0, latido_ms: 0, ..Ajustes::default() };
    let dir = tempfile::tempdir().unwrap();
    let n = Nucleo::con_persistencia(arranque(m, r(), ajustes.clone()), dir.path(), Default::default()).unwrap();
    let mut cortado = guion(m, n, &|n, t| {
        n.avanzar_hasta(t / 3);
        n.foto_ahora();
        n.avanzar_hasta(t / 3 + HORA);
        n.apagar().unwrap();
        *n = Nucleo::con_persistencia(arranque(m, r(), ajustes.clone()), dir.path(), Default::default()).unwrap();
        assert!(n.recuperado());
        n.avanzar_hasta(t);
    });
    assert_eq!(cortado.hash_estado(), de_una.hash_estado());
    cortado.apagar().unwrap();
}

// ───────────────────────────── 5. Por la sesión ─────────────────────────────

#[tokio::test]
async fn por_la_sesion_el_rey_adopta_en_la_capital_y_su_faccion_oye_el_hecho() {
    use comun::sesion::*;
    use contrato::v1::{self as pb, orden, TipoCliente};
    let m = con_mundo!();
    let (mut e, _) = con_canteria(m);
    let plaza = e.plaza;
    // Por la sesión se juega con un héroe de verdad (el de administración que fundó no es el de la cuenta): «ana» crea el suyo, entra en la Facción y el fundador le cede el trono.
    let r = e.ord("ana", Comando::CrearHeroe { cuenta: "ana".into(), campamento: 0, controlador: militar::heroe::Controlador::Humano, nombre: None });
    let rey = r.creadas[0];
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: rey, faccion: Some(1) });
    let fundador = e.rey;
    e.ok("ana", Comando::AsignarRey { heroe: fundador, faccion: 1, designado: rey });
    let w = arrancar_con(e.n).await;
    let (mut c, _) = w.entrar("ana", TipoCliente::ClienteUnity).await;
    let adoptar = |x: &str| orden::Cuerpo::AdoptarTecnologia(pb::OrdenAdoptarTecnologia { faccion_id: "f-1".into(), tecnologia_id: x.into() });
    let (_, r) = dar(&mut c, "t1", adoptar("no_existe")).await;
    assert_eq!(rechazo(&r), "tecnologia.desconocida", "ya no es `orden.no_disponible`");
    let (_, r) = dar(&mut c, "t2", adoptar("canteria")).await;
    assert_eq!(rechazo(&r), "tecnologia.rey_fuera_de_la_capital");
    // La administración lo pone dentro de la capital.
    c.cerrar().await;
    let n = w.parar().await;
    let mut e = Escenario { n, plaza, rey, k: 100 };
    e.ok("admin", Comando::ColocarHeroe { heroe: rey, ubicacion: militar::presencia::Ubicacion::Asentamiento(plaza as u32) });
    let w = arrancar_con(e.n).await;
    let (mut c, _) = w.entrar("ana", TipoCliente::ClienteUnity).await;
    let (deltas, r) = dar(&mut c, "t4", adoptar("canteria")).await;
    assert_eq!(r.resultado, pb::Resultado::OrdenAceptada as i32, "{r:?}");
    assert!(deltas.iter().flat_map(|d| &d.eventos).any(|x| x.codigo == "tecnologia.adoptada"), "el hecho llega a la Facción");
    let (_, r) = dar(&mut c, "t5", adoptar("canteria")).await;
    assert_eq!(rechazo(&r), "tecnologia.ya_adoptada");
    c.cerrar().await;
    let n = w.parar().await;
    assert!(n.tiene_tecnologia(1, "canteria"));
}

#[test]
fn con_todos_sus_logros_la_era_espera_a_su_minimo_de_una_semana() {
    let m = con_mundo!();
    // La Era I con un solo logro, `plazasEnNivel.2 ≥ 1`, que se cumple fijando el nivel de la plaza.
    let mut e = Escenario::nuevo(
        m,
        reglas(|t| {
            for d in t.catalogo.iter_mut().filter(|d| d.era == 0) {
                d.logro = None;
            }
            con_logro_de_nivel_2(t, "metalurgia_cobre");
        }),
    );
    let r = e.n.estado().reglas.economia.tecnologia.clone();
    let (desde, semana) = (e.n.tecnologia().era_desde, r.eras[0].minimo);
    assert_eq!((semana, e.n.estado().era_en), (7 * DIA, Some(desde + r.eras[0].plazo)), "sin el logro, la cita es al fin del plazo");
    // El último logro llega antes del mínimo: la cita se mueve al mínimo, no pasa en el acto.
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: e.plaza, nivel: 2 });
    assert!(e.n.tecnologia().todos_los_logros_de_la_era(&r));
    assert_eq!((e.n.tecnologia().era, e.n.estado().era_en), (0, Some(desde + semana)));
    e.n.avanzar_hasta(desde + semana - 1);
    assert_eq!(e.n.tecnologia().era, 0, "antes del mínimo no pasa");
    e.n.avanzar_hasta(desde + semana);
    let h = hechos(&e.n, "era.comienza");
    assert_eq!((e.n.tecnologia().era, h.len(), h[0].en, h[0].dato("porPlazo")), (1, 1, desde + semana, Some(0)));
    // La Era II, sin sus logros, vuelve a su plazo.
    assert_eq!(e.n.estado().era_en, Some(desde + semana + r.eras[1].plazo));
}
