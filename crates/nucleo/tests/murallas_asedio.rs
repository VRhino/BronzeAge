//! Los recintos de muralla en el combate con números (Docs/Mechanics/Murallas.md §9.1 y §9.6, fase de Murallas, tercera tanda): el multiplicador del defensor en el
//! asedio a una plaza, que sigue siendo 1 para la columna, el daño de la conquista y su reparación solos, y lo que viaja a Unity (`recintos_en_pie`) y al dibujo
//! (`VistaCiudad.murallas`). Sobre el artefacto de mundo real (`generador/salida/mundo/mundo-*.bin`); las pruebas se omiten si no está.

mod comun;
use asentamiento::recintos::{self, Mejora, Recinto};
use asentamiento::{Cargo, Recurso};
use base::{Instante, Punto, MS_POR_MINUTO};
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use mundo::{Mundo, OpcionesValidacion};
use nucleo::{rechazos, Ajustes, Arranque, Carga, Comando, Nucleo, ObjetivoAtaque, Peticion, Reglas, ReglasEconomia, Resultado, Respuesta};
use rutas::Rutas;
use std::sync::{Arc, OnceLock};
use trazado::ClaseCelda;

const MIN: i64 = MS_POR_MINUTO;
const HORA: i64 = 60 * MIN;
const UNO: i64 = 1_000_000;

type M = (Arc<Mundo>, Arc<Rutas>);

fn mundo() -> Option<&'static M> {
    static M: OnceLock<Option<M>> = OnceLock::new();
    M.get_or_init(|| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generador/salida/mundo");
        let f = std::fs::read_dir(dir).ok()?.flatten().find(|e| e.file_name().to_string_lossy().starts_with("mundo-"))?;
        let m = Mundo::cargar(&std::fs::read(f.path()).ok()?, &OpcionesValidacion::default()).expect("artefacto válido");
        let r = Arc::new(Rutas::desde_mundo(&m));
        Some((Arc::new(m), r))
    })
    .as_ref()
}

macro_rules! con_mundo {
    () => {
        match mundo() {
            Some(m) => m,
            None => {
                eprintln!("sin artefacto de mundo real: se omite");
                return;
            }
        }
    };
}

/// Las reglas por defecto con almacenes de 100.000 por recurso (un anillo entero cuesta más de lo que cabe en el de partida) y sin bandidos (miden otra cosa).
fn reglas() -> Reglas {
    let mut economia = ReglasEconomia::cargar();
    Arc::make_mut(&mut economia.datos).capacidad_inicial = 100_000_000;
    let mut r = Reglas { economia, ..Reglas::default() };
    let b = &mut Arc::make_mut(&mut r.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    r
}

fn campamento(m: &Mundo, i: usize) -> Punto {
    let p = m.campamentos()[i].posicion.unwrap();
    Punto::new(p.x, p.y)
}

fn campamento_mas_cercano(m: &Mundo, p: Punto) -> u32 {
    (0..m.campamentos().len()).min_by_key(|&i| campamento(m, i).dist2(p)).unwrap() as u32
}

fn sitio_rico(n: &Nucleo, m: &Mundo, i: usize, faccion: u32) -> Punto {
    let c = campamento(m, i);
    for r in (0..8).map(|k| 50_000 + k * 10_000) {
        for g in 0..360 {
            let a = (g as f64).to_radians();
            let p = Punto::new(c.x + (r as f64 * a.cos()) as i64, c.y + (r as f64 * a.sin()) as i64);
            if n.viabilidad(p, faccion).is_some_and(|v| v.motivo.is_none() && v.color == territorio::Viabilidad::Verde && v.parcelas_en_techo >= 6) {
                return p;
            }
        }
    }
    panic!("sin sitio rico cerca del campamento {i}");
}

fn completa(tropa: Tropa) -> i64 {
    militar::Reglas::cargar().tropas.tropa(tropa).unidades
}

fn fundar(n: &mut Nucleo, t: Instante, nombre: &str, dueno: &str, faccion: u32, p: Punto) -> u64 {
    let e = n.crear_ejercito(t, dueno, faccion, p, 500, 100_000).unwrap();
    assert!(n.aplicar_en(t, Peticion::nueva("admin", &format!("c-{nombre}"), Comando::DarCaravanaFundacion { ejercito: e }).de_admin()).aceptada());
    let r = n.aplicar_en(t, Peticion::nueva(dueno, &format!("f-{nombre}"), Comando::Fundar { ejercito: e }));
    assert!(r.aceptada(), "{r:?}");
    r.creadas[0]
}

type Hecho = (Instante, Vec<u64>, Vec<(&'static str, i64)>);

fn hechos(n: &Nucleo, codigo: &str) -> Vec<Hecho> {
    n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).map(|e| (e.en, e.asentamientos.clone(), e.datos.clone())).collect()
}

fn dato(h: &Hecho, clave: &str) -> i64 {
    h.2.iter().find(|(k, _)| *k == clave).unwrap_or_else(|| panic!("el hecho no lleva «{clave}»: {h:?}")).1
}

/// Dos plazas de Facciones distintas (F1 con «ana» junto al campamento 0, F2 con «beto» junto al 3), sin auto-construcción. La B puede llevar su muralla.
struct Esc {
    n: Nucleo,
    b: u64,
    cuenta: u64,
}

impl Esc {
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.cuenta += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.cuenta), c);
        // Andamiaje (5E): una orden de plaza de quien mandaba en ella antes del sistema 5 se da como administración (`comun::mandaba`).
        let p = if actor == "admin" || comun::mandaba(&self.n, actor, &p.comando) { p.de_admin() } else { p };
        let t = self.n.ahora();
        self.n.aplicar_en(t, p)
    }
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    fn hasta(&mut self, t: Instante) {
        self.n.avanzar_hasta(t);
    }
    fn avanzar(&mut self, ms: i64) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    fn recintos_de_b(&self) -> &[Recinto] {
        &self.n.economia(self.b).unwrap().recintos
    }
    fn pos(&self, plaza: u64) -> Punto {
        self.n.estado().asentamientos[&plaza].posicion
    }
    /// Avanza hasta que el recinto `i` de la plaza B está cerrado (y sin mejora en curso).
    fn terminar(&mut self, i: usize) {
        for _ in 0..5_000 {
            let r = &self.recintos_de_b()[i];
            if r.completo() && r.mejora.is_none() {
                return;
            }
            // Una celda parada por falta de materiales conserva el instante que le tocaba, ya pasado: el paso de minuto la reintenta.
            let ahora = self.n.ahora();
            let t = r.siguiente_celda_en.filter(|&s| s > ahora).unwrap_or(ahora + MIN);
            self.hasta(t);
        }
        panic!("el recinto {i} no termina");
    }
    /// «beto» (humano, F2) dentro de la plaza B, que la defiende en persona con una escuadra de cada tropa dada.
    fn beto_dentro(&mut self, m: &Mundo, defensa: &[Tropa]) -> u64 {
        let campamento = campamento_mas_cercano(m, self.pos(self.b));
        let h = self.ok("beto", Comando::CrearHeroe { cuenta: "beto".into(), campamento, controlador: Controlador::Humano }).creadas[0];
        let b = self.b as u32;
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(2) });
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(b)) });
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(b) });
        if !defensa.is_empty() {
            let ids: Vec<u64> = defensa.iter().map(|&t| self.ok("admin", Comando::DarEscuadra { heroe: h, tropa: t, cantidad: completa(t) }).creadas[0]).collect();
            let loadout = self.n.heroe(h).unwrap().heroe.loadouts[0].id;
            self.ok("admin", Comando::GuardarLoadout { heroe: h, loadout: Some(loadout), nombre: "Default".into(), escuadras: ids, perks: vec![], activo: Some(true) });
        }
        h
    }
    /// Un ejército de administración de `cuenta`/`faccion` a 2 u de `p`, con las tropas dadas: devuelve (columna, héroe).
    fn ejercito(&mut self, cuenta: &str, faccion: u32, p: Punto, tropas: &[Tropa]) -> (u64, u64) {
        let t = self.n.ahora();
        let col = self.n.crear_ejercito(t, cuenta, faccion, Punto::new(p.x + 2_000, p.y), 500, 350_000).unwrap();
        let h = self.n.ejercito(col).unwrap().columna.lider;
        for &tropa in tropas {
            self.ok("admin", Comando::DarEscuadra { heroe: h, tropa, cantidad: completa(tropa) });
        }
        (col, h)
    }
    fn atacar(&mut self, actor: &str, heroe: u64, objetivo: ObjetivoAtaque) -> Respuesta {
        self.ord(actor, Comando::Atacar { heroe, objetivo, declara_guerra: true })
    }
    /// Las citas de fin de obras de la plaza B que hay en la cola son exactamente las de lo que está en curso: las obras, las mejoras, el ascenso y la celda que espera
    /// cada recinto. Ni falta ninguna ni sobra una.
    fn las_citas_cuadran(&self) {
        let (e, ahora) = (self.n.estado(), self.n.ahora());
        let eco = self.n.economia(self.b).unwrap();
        let mut esperadas: Vec<Instante> = eco.edificios.iter().filter_map(|x| x.completa_en.filter(|_| x.estado == asentamiento::edificio::Estado::EnConstruccion)).collect();
        esperadas.extend(eco.edificios.iter().filter_map(|x| x.mejora.map(|m| m.completa_en)));
        esperadas.extend(eco.ascenso.map(|s| s.completa_en));
        esperadas.extend(eco.recintos.iter().filter_map(|r| r.siguiente_celda_en));
        esperadas.retain(|&f| f > ahora);
        esperadas.sort_unstable();
        esperadas.dedup();
        let mut citas: Vec<Instante> = e.cola.volcar().iter().filter(|ev| matches!(ev.carga, Carga::FinDeObras) && ev.clave.entidad == self.b && e.generaciones.es_vigente(ev.clave)).map(|ev| ev.instante).collect();
        citas.sort_unstable();
        citas.dedup();
        assert_eq!(citas, esperadas, "las citas de fin de la cola no cuadran con lo que está en curso");
    }
    /// El único hecho de asedio de la partida (conquistada o resistida).
    fn hecho_de_asedio(&self) -> Hecho {
        let mut todos: Vec<Hecho> = ["combate.asedio_conquista", "combate.asedio_resistido"].iter().flat_map(|c| hechos(&self.n, c)).collect();
        assert_eq!(todos.len(), 1, "un asedio, un hecho: {todos:?}");
        todos.remove(0)
    }
}

/// Qué anillo lleva la plaza B al empezar la prueba.
#[derive(Clone, Copy, PartialEq)]
enum Anillo {
    /// Ninguno, a las 25 h.
    Ninguno,
    /// Una empalizada completa, a las 25 h.
    Cerrado,
    /// Ninguno todavía, a la hora y poco de fundar (la plaza, sin auto-construcción, cae en ruinas a las 55 h: una prueba que levanta y mejora un anillo no puede esperar antes).
    SinEmpezar,
}

/// Las dos plazas, con la B a nivel de plaza `nivel` y rica. Con `Cerrado`, la B levanta una empalizada completa; `Ninguno` y `Cerrado` llegan al mismo instante (25 h) para
/// que la única diferencia entre una con muralla y otra sin ella sea el anillo (las órdenes de administración, el reloj y el generador son los mismos).
fn montar(m: &M, semilla: u64, nivel: u32, anillo: Anillo) -> Esc {
    let a = Arranque { reglas: reglas(), ajustes: Ajustes::default(), semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) };
    let mut n = Nucleo::nuevo(a);
    let (pa, pb) = (sitio_rico(&n, &m.0, 0, 1), sitio_rico(&n, &m.0, 3, 2));
    let (ia, ib) = (fundar(&mut n, 0, "a", "ana", 1, pa), fundar(&mut n, 0, "b", "beto", 2, pb));
    let mut e = Esc { n, b: ib, cuenta: 0 };
    for plaza in [ia, ib] {
        e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
    }
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: ib, nivel });
    // De todo: sin auto-construcción nadie produce, y una plaza sin comida ni mantenimiento caería en ruinas a las pocas decenas de horas (una mejora de muralla dura más).
    for r in asentamiento::recurso::TODOS {
        let respuesta = e.ord("admin", Comando::DarRecursos { asentamiento: ib, recurso: r, cantidad: 5_000_000 });
        assert!(respuesta.aceptada() || respuesta.resultado == Resultado::SinCambios, "{respuesta:?}");
    }
    e.hasta(HORA + 30_000);
    if anillo == Anillo::Cerrado {
        e.ok("beto", Comando::ComprometerRecinto { asentamiento: ib, cargo: Cargo::MaestroObras, nivel: 1 });
        e.terminar(0);
        assert!(e.n.ahora() < 25 * HORA, "la empalizada se cierra antes de las 25 h ({} min)", e.n.ahora() / MIN);
    }
    if anillo != Anillo::SinEmpezar {
        e.hasta(25 * HORA + 30_000);
    }
    e
}

/// El multiplicador que dice la fórmula del documento para un anillo (`1 + (bono − 1) × integridad / puertas`, una sola división entera).
fn esperado(bono: i64, r: &Recinto) -> i64 {
    UNO + (bono - UNO) * r.levantadas() as i64 / (r.celdas.len() as i64 * r.puertas() as i64)
}

const ATAQUE: [Tropa; 2] = [Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto];

// ───────────────────────────── El multiplicador del asedio ─────────────────────────────

#[test]
fn el_asedio_con_numeros_multiplica_al_defensor_con_su_recinto_y_sin_recinto_es_el_de_antes() {
    let m = con_mundo!();
    let [mut con, mut sin] = [Anillo::Cerrado, Anillo::Ninguno].map(|anillo| montar(m, 7, 2, anillo));
    assert_eq!(con.n.estado().rng.estado(), sin.n.estado().rng.estado(), "el anillo no consume tiradas: las dos partidas llegan con el mismo generador");
    let rec = con.recintos_de_b()[0].clone();
    assert!(rec.completo() && rec.puertas() >= 1);
    let bono = con.n.estado().reglas.economia.datos.muralla.bono_defensa[0];
    assert_eq!(bono, 1_300_000, "la empalizada");
    let multiplicador = esperado(bono, &rec);
    assert!(multiplicador > UNO && multiplicador <= bono, "con {} puertas: ×{multiplicador}", rec.puertas());
    assert_eq!(con.n.multiplicador_de_plaza(con.b), multiplicador);
    assert_eq!(sin.n.multiplicador_de_plaza(sin.b), UNO, "sin recinto no multiplica");
    assert_eq!(sin.n.multiplicador_de_plaza(9_999), UNO, "ni una plaza que no existe");

    let mut poderes = vec![];
    for (e, esperado) in [(&mut con, multiplicador), (&mut sin, UNO)] {
        let b = e.b;
        e.beto_dentro(&m.0, &[Tropa::MiliciaLanceros]);
        let ana = e.ejercito("ana", 1, e.pos(b), &ATAQUE);
        let rng0 = e.n.estado().rng.estado();
        let r = e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b));
        assert!(r.aceptada(), "{r:?}");
        tras_la_convocatoria(&mut e.n);
        let h = e.hecho_de_asedio();
        assert_eq!((dato(&h, "multiplicadorMuralla"), dato(&h, "murallaCompleta")), (esperado, i64::from(esperado > UNO)));
        let mut espejo = base::Rng::nuevo(rng0);
        (0..2).for_each(|_| {
            espejo.siguiente();
        });
        assert_eq!(e.n.estado().rng.estado(), espejo.estado(), "el multiplicador no consume tirada: el asedio sigue gastando dos");
        poderes.push((dato(&h, "poderAtacante"), dato(&h, "poderDefensor")));
    }
    // Con las mismas tiradas y las mismas fuerzas, solo cambia el poder del defensor, en el multiplicador (salvo el redondeo de la última cifra).
    let ((atac_con, def_con), (atac_sin, def_sin)) = (poderes[0], poderes[1]);
    assert_eq!(atac_con, atac_sin, "el atacante no cambia");
    let esperado_def = def_sin as i128 * multiplicador as i128 / UNO as i128;
    assert!((def_con as i128 - esperado_def).abs() <= 2, "poder del defensor {def_con} contra {esperado_def} = {def_sin} × {multiplicador}");
    assert!(def_con > def_sin);
}

#[test]
fn durante_una_mejora_defiende_el_anillo_en_pie_con_su_nivel_de_antes() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 3, Anillo::Cerrado);
    let (b, antes) = (e.b, e.n.multiplicador_de_plaza(e.b));
    e.ok("beto", Comando::MejorarRecinto { asentamiento: b, cargo: Cargo::MaestroObras, recinto: 1 });
    e.hasta(e.n.ahora() + 3 * 30 * MIN);
    let rec = &e.recintos_de_b()[0];
    assert_eq!((rec.nivel, rec.completo(), rec.mejora.map(|x| x.nivel)), (1, true, Some(2)));
    assert!(rec.mejora.unwrap().avance >= 2, "la mejora lleva celdas hechas");
    assert_eq!(e.n.multiplicador_de_plaza(b), antes, "el anillo en pie sigue valiendo lo de antes (Q1), no la integridad 0 del TypeScript");
    e.beto_dentro(&m.0, &[Tropa::MiliciaLanceros]);
    let ana = e.ejercito("ana", 1, e.pos(b), &ATAQUE);
    assert!(e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b)).aceptada());
    tras_la_convocatoria(&mut e.n);
    let h = e.hecho_de_asedio();
    assert_eq!((dato(&h, "multiplicadorMuralla"), dato(&h, "murallaCompleta")), (antes, 1), "en pleno asedio defiende la empalizada y la plaza sigue teniendo muralla completa");
}

#[test]
fn la_columna_no_tiene_recinto_aunque_pelee_junto_a_una_plaza_amurallada() {
    let m = con_mundo!();
    let [mut con, mut sin] = [Anillo::Cerrado, Anillo::Ninguno].map(|anillo| montar(m, 7, 2, anillo));
    let mut poderes = vec![];
    for e in [&mut con, &mut sin] {
        let b = e.b;
        // Dos columnas de Facciones distintas pegadas a la plaza B: la de «beto» defiende a la puerta de su plaza amurallada, pero una columna no tiene recinto (K16).
        let beto = e.ejercito("beto", 2, e.pos(b), &[Tropa::MiliciaLanceros]);
        let ana = e.ejercito("ana", 1, e.pos(b), &ATAQUE);
        e.ok("admin", Comando::FijarRelacion { a: 1, b: 2, relacion: Some(nucleo::Relacion::Guerra) });
        let r = e.atacar("ana", ana.1, ObjetivoAtaque::Ejercito(beto.0));
        assert!(r.aceptada(), "{r:?}");
        let h = hechos(&e.n, "combate.resuelto");
        assert_eq!(h.len(), 1);
        poderes.push((dato(&h[0], "poderAtacante"), dato(&h[0], "poderDefensor")));
    }
    assert_eq!(poderes[0], poderes[1], "con o sin muralla en la plaza de al lado, una columna defiende con el mismo poder");
}

// ───────────────────────────── El daño de la conquista ─────────────────────────────

#[test]
fn la_conquista_daña_un_30_por_ciento_el_anillo_completo_y_se_repara_sola_a_costa_del_nuevo_dueno() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 3, Anillo::Cerrado);
    let b = e.b;
    let antes = e.recintos_de_b()[0].clone();
    let n = antes.celdas.len();
    let perdidas = n * 300_000 / UNO as usize;
    assert!(perdidas >= 5, "el anillo de partida tiene {n} celdas");
    let completado_en = antes.completado_en;
    assert!(e.n.ciudad(b).unwrap().recintos()[0].completo, "la ciudad ve el anillo completo");
    assert_eq!(recintos::nivel_del_mejor_completo(e.recintos_de_b()), 1);
    e.beto_dentro(&m.0, &[]);
    let ana = e.ejercito("ana", 1, e.pos(b), &ATAQUE);
    let t = e.n.ahora();
    assert!(e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b)).aceptada());
    tras_la_convocatoria(&mut e.n);
    // Sin defensores con soldados cae sin combate (como por Unity: el daño es de la conquista, no de cómo se resolvió).
    let a = &e.n.estado().asentamientos[&b];
    assert_eq!((a.faccion, a.economia.ocupado), (1, true));
    let c = &hechos(&e.n, "asentamiento.conquistada")[0];
    assert_eq!((dato(c, "murallaCompleta"), dato(c, "murallaDanada")), (1, 1), "tenía muralla completa justo antes del daño y la perdió en parte");
    // El anillo pierde floor(0,3 × celdas) y el nuevo dueño levanta ya la primera celda de la reparación: queda un 70 % más una celda, con su cita.
    let rec = &e.recintos_de_b()[0];
    assert_eq!(rec.levantadas(), n - perdidas + 1, "se dañaron {perdidas} de {n} y se repara una celda en el instante de la conquista");
    assert!(!rec.completo() && rec.danado() && rec.completado_en == completado_en, "dañado: no vuelve a estar «sin cerrar nunca»");
    assert_eq!(rec.siguiente_celda_en, Some(t + 30_000 + 15 * MIN), "desde la conquista, al acabar la convocatoria");
    e.las_citas_cuadran(); // la reparación deja su cita y no sobra ninguna
    // Dañado, ya no vale: ni muralla completa, ni requisito del nivel 4, ni preferencia intramuros; el multiplicador es el de su integridad.
    assert!(!recintos::muralla_completa(e.recintos_de_b()) && recintos::nivel_del_mejor_completo(e.recintos_de_b()) == 0);
    assert!(!e.n.ciudad(b).unwrap().recintos()[0].completo, "la ciudad se rehízo con la muralla incompleta");
    let bono = e.n.estado().reglas.economia.datos.muralla.bono_defensa[0];
    assert_eq!(e.n.multiplicador_de_plaza(b), esperado(bono, rec));
    assert!(e.n.multiplicador_de_plaza(b) < esperado(bono, &antes));
    // Un recinto que alguna vez estuvo completo no se abandona, ni dañado (Q2), y uno dañado bloquea mejorar y ampliar hasta repararse.
    let casa = e.n.estado().asentamientos[&b].faccion;
    assert_eq!(casa, 1);
    assert_eq!(codigo_de(&e.ord("ana", Comando::AbandonarRecinto { asentamiento: b, recinto: 1 })), rechazos::MURALLAS_YA_CERRADO);
    assert_eq!(codigo_de(&e.ord("ana", Comando::MejorarRecinto { asentamiento: b, cargo: Cargo::MaestroObras, recinto: 1 })), rechazos::MURALLAS_INCOMPLETO);
    // Se repara solo, una celda por cita, a costa de la plaza (el nuevo dueño), sin volver a anunciar el cierre.
    let madera = e.n.economia(b).unwrap().almacen.cantidad(Recurso::Madera);
    e.hasta(t + 30_000 + 15 * MIN);
    assert_eq!(e.recintos_de_b()[0].levantadas(), n - perdidas + 2, "la siguiente celda sale en su cita");
    assert!(e.n.economia(b).unwrap().almacen.cantidad(Recurso::Madera) < madera, "la reparación cuesta materiales de la plaza");
    e.terminar(0);
    let rec = &e.recintos_de_b()[0];
    assert!(rec.completo() && rec.completado_en == completado_en && rec.siguiente_celda_en.is_none());
    assert!(e.n.ciudad(b).unwrap().recintos()[0].completo, "vuelve la preferencia intramuros");
    assert_eq!(hechos(&e.n, "construccion.recinto_completado").len(), 1, "reparar no vuelve a anunciar el cierre");
    e.las_citas_cuadran(); // reparado, no queda ninguna cita de recinto
    assert_eq!(e.n.multiplicador_de_plaza(b), esperado(bono, &antes), "reparado, vale lo de antes");
}

fn codigo_de(r: &Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

#[test]
fn una_mejora_en_curso_no_recibe_dano_y_la_sigue_el_nuevo_dueno() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 3, Anillo::Cerrado);
    let b = e.b;
    e.ok("beto", Comando::MejorarRecinto { asentamiento: b, cargo: Cargo::MaestroObras, recinto: 1 });
    e.hasta(e.n.ahora() + 2 * 30 * MIN);
    let antes = e.recintos_de_b()[0].clone();
    let mejora: Mejora = antes.mejora.expect("mejorando");
    assert!(antes.completo() && mejora.avance >= 1);
    e.beto_dentro(&m.0, &[]);
    let ana = e.ejercito("ana", 1, e.pos(b), &ATAQUE);
    assert!(e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b)).aceptada());
    tras_la_convocatoria(&mut e.n);
    assert_eq!(e.n.estado().asentamientos[&b].faccion, 1);
    let c = &hechos(&e.n, "asentamiento.conquistada")[0];
    assert_eq!((dato(c, "murallaCompleta"), dato(c, "murallaDanada")), (1, 0), "tenía muralla completa y la mejora la protege del daño");
    let rec = &e.recintos_de_b()[0];
    assert_eq!((rec.avance, rec.nivel, rec.mejora, rec.siguiente_celda_en), (antes.avance, 1, antes.mejora, antes.siguiente_celda_en), "ni una celda perdida ni la cita tocada");
    assert!(e.n.ciudad(b).unwrap().recintos()[0].completo);
    e.las_citas_cuadran(); // la cita de la celda de la mejora sigue en la cola
    // La mejora la termina el nuevo dueño con la obra normal.
    e.terminar(0);
    assert_eq!(e.recintos_de_b()[0].nivel, 2);
}

#[test]
fn un_recinto_que_nunca_se_cerro_no_recibe_dano_ni_cuenta_como_muralla_completa() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2, Anillo::Ninguno);
    let b = e.b;
    e.ok("beto", Comando::ComprometerRecinto { asentamiento: b, cargo: Cargo::MaestroObras, nivel: 1 });
    e.hasta(e.n.ahora() + 4 * 15 * MIN);
    let antes = e.recintos_de_b()[0].clone();
    assert!(!antes.completo() && antes.levantadas() >= 4);
    e.beto_dentro(&m.0, &[]);
    let ana = e.ejercito("ana", 1, e.pos(b), &ATAQUE);
    assert!(e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b)).aceptada());
    tras_la_convocatoria(&mut e.n);
    let c = &hechos(&e.n, "asentamiento.conquistada")[0];
    assert_eq!((dato(c, "murallaCompleta"), dato(c, "murallaDanada")), (0, 0));
    assert_eq!(e.recintos_de_b()[0], antes, "en obra no se daña ni se toca");
    e.terminar(0);
    assert!(e.recintos_de_b()[0].completo(), "lo sigue levantando el nuevo dueño");
}

#[test]
fn una_plaza_sin_recintos_se_conquista_como_siempre() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2, Anillo::Ninguno);
    let b = e.b;
    e.beto_dentro(&m.0, &[]);
    let ana = e.ejercito("ana", 1, e.pos(b), &ATAQUE);
    assert!(e.atacar("ana", ana.1, ObjetivoAtaque::Asentamiento(b)).aceptada());
    tras_la_convocatoria(&mut e.n);
    let c = &hechos(&e.n, "asentamiento.conquistada")[0];
    assert_eq!((dato(c, "murallaCompleta"), dato(c, "murallaDanada")), (0, 0));
    assert!(hechos(&e.n, "construccion.recinto_celda").is_empty() && e.recintos_de_b().is_empty());
}

// ───────────────────────────── Lo que ve Unity y el dibujo ─────────────────────────────

#[test]
fn recintos_en_pie_da_solo_las_celdas_levantadas_con_el_nivel_en_pie_y_una_mejora_no_cambia_nada() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 3, Anillo::SinEmpezar);
    let b = e.b;
    assert!(e.n.recintos_en_pie(b).is_empty() && e.n.recintos_en_pie(9_999).is_empty(), "sin recintos, nada viaja");
    e.ok("beto", Comando::ComprometerRecinto { asentamiento: b, cargo: Cargo::MaestroObras, nivel: 1 });
    // A medio levantar: solo las celdas hasta `avance`, en su orden de recorrido.
    e.hasta(e.n.ahora() + 6 * 15 * MIN);
    let rec = e.recintos_de_b()[0].clone();
    let pie = e.n.recintos_en_pie(b);
    assert_eq!(pie.len(), 1);
    assert_eq!((pie[0].id, pie[0].nivel, pie[0].celdas.len()), (1, 1, rec.levantadas()));
    assert_eq!(pie[0].celdas[..], rec.celdas[..rec.levantadas()]);
    assert!(rec.levantadas() < rec.celdas.len(), "aún no está entero");
    // Cerrado, viaja el anillo entero; una mejora en curso no lo cambia (ni el nivel ni las celdas), por mucho que avance.
    e.terminar(0);
    let entero = e.n.recintos_en_pie(b);
    assert_eq!(entero[0].celdas, e.recintos_de_b()[0].celdas);
    e.ok("beto", Comando::MejorarRecinto { asentamiento: b, cargo: Cargo::MaestroObras, recinto: 1 });
    e.hasta(e.n.ahora() + 4 * 30 * MIN);
    assert!(e.recintos_de_b()[0].mejora.is_some());
    assert_eq!(e.n.recintos_en_pie(b), entero, "la mejora en curso no cambia lo que está en pie");
    // Al cerrarse la mejora cambia el nivel y las torres (la clase de las celdas de muro que pasan a torre).
    e.terminar(0);
    let mejorado = e.n.recintos_en_pie(b);
    assert_eq!((mejorado[0].nivel, mejorado[0].celdas.len()), (2, entero[0].celdas.len()));
    assert!(mejorado[0].celdas.iter().any(|c| c.clase == ClaseCelda::Torre) && entero[0].celdas.iter().all(|c| c.clase != ClaseCelda::Torre));
    assert_eq!(mejorado[0].celdas.iter().map(|c| c.celda).collect::<Vec<_>>(), entero[0].celdas.iter().map(|c| c.celda).collect::<Vec<_>>(), "el mismo anillo");
}

#[test]
fn la_vista_de_la_ciudad_dibuja_las_celdas_levantadas_por_clase_y_las_planificadas_aparte() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 3, Anillo::SinEmpezar);
    let b = e.b;
    assert!(e.n.vista_ciudad(b).unwrap().murallas.is_empty());
    e.ok("beto", Comando::ComprometerRecinto { asentamiento: b, cargo: Cargo::MaestroObras, nivel: 1 });
    e.hasta(e.n.ahora() + 10 * 15 * MIN);
    let rec = e.recintos_de_b()[0].clone();
    let celdas = |rs: &[trazado::RectLocal]| rs.iter().map(|r| (r.ancho / trazado::CELDA_UL) * (r.alto / trazado::CELDA_UL)).sum::<i32>() as usize;
    let v = &e.n.vista_ciudad(b).unwrap().murallas;
    assert_eq!(v.len(), 1);
    let en_pie = &rec.celdas[..rec.levantadas()];
    let cuenta = |clase| en_pie.iter().filter(|c| c.clase == clase).count();
    assert_eq!((v[0].id, v[0].nivel, v[0].mejorando_a), (1, 1, None));
    assert_eq!((celdas(&v[0].muros), celdas(&v[0].torres), celdas(&v[0].puertas)), (cuenta(ClaseCelda::Muro), 0, cuenta(ClaseCelda::Puerta)), "la empalizada no tiene torres");
    assert_eq!(celdas(&v[0].planificadas), rec.celdas.len() - rec.levantadas(), "las que faltan por levantar, de cualquier clase");
    assert!(!v[0].planificadas.is_empty() && !v[0].puertas.is_empty());
    // Cerrado y mejorándose: se dibuja con el nivel en pie y se avisa de a qué nivel va; con las torres del nivel nuevo al terminar.
    e.terminar(0);
    e.ok("beto", Comando::MejorarRecinto { asentamiento: b, cargo: Cargo::MaestroObras, recinto: 1 });
    let v = &e.n.vista_ciudad(b).unwrap().murallas;
    assert_eq!((v[0].nivel, v[0].mejorando_a, v[0].planificadas.len()), (1, Some(2), 0));
    assert_eq!(celdas(&v[0].muros) + celdas(&v[0].puertas), e.recintos_de_b()[0].celdas.len());
    e.terminar(0);
    let v = &e.n.vista_ciudad(b).unwrap().murallas;
    assert_eq!((v[0].nivel, v[0].mejorando_a), (2, None));
    assert!(celdas(&v[0].torres) > 0, "las torres del muro de piedra");
}

#[test]
fn con_un_recinto_real_los_edificios_nuevos_no_lo_pisan_y_los_urbanos_se_prefieren_intramuros() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2, Anillo::Cerrado);
    let b = e.b;
    let anillo: Vec<_> = e.recintos_de_b()[0].celdas.iter().filter(|c| c.clase != ClaseCelda::Puerta).map(|c| c.celda).collect();
    let interior = trazado::interior_de(e.n.ciudad(b).unwrap(), &e.recintos_de_b()[0].celdas).expect("el anillo aísla el Centro Urbano");
    // Construir de verdad: más viviendas y almacenes, esperando a que se levanten.
    let antes = e.n.ciudad(b).unwrap().piezas().len();
    for k in 0..6 {
        let tipo = if k % 2 == 0 { asentamiento::Tipo::Vivienda } else { asentamiento::Tipo::Almacen };
        e.ok("beto", Comando::AnadirEdificio { asentamiento: b, tipo });
        e.avanzar(3 * HORA);
    }
    let ciudad = e.n.ciudad(b).unwrap();
    let nuevas: Vec<_> = ciudad.piezas().iter().skip(antes).collect();
    assert!(nuevas.len() >= 4, "se colocaron {} piezas", nuevas.len());
    for p in &nuevas {
        let celdas: Vec<_> = (p.rect.col..p.rect.col + p.rect.ancho).flat_map(|c| (p.rect.row..p.rect.row + p.rect.alto).map(move |r| trazado::Celda::new(c, r))).collect();
        assert!(celdas.iter().all(|c| !anillo.contains(c)), "una pieza nueva pisa el anillo");
        assert!(celdas.iter().all(|c| interior.contiene(*c)), "con el anillo completo, los edificios urbanos nuevos caen intramuros mientras haya sitio: {:?}", p.tipo);
    }
    // Todo edificio conserva frente de calle con el anillo puesto.
    let red = |c: trazado::Celda| ciudad.es_calle(c) || ciudad.es_camino(c);
    let con_frente = |r: &trazado::Rect| {
        let (arriba, abajo, izq, der) = (r.row - 1, r.row + r.alto, r.col - 1, r.col + r.ancho);
        (r.col..r.col + r.ancho).any(|c| red(trazado::Celda::new(c, arriba)) || red(trazado::Celda::new(c, abajo))) || (r.row..r.row + r.alto).any(|f| red(trazado::Celda::new(izq, f)) || red(trazado::Celda::new(der, f)))
    };
    assert!(ciudad.piezas().iter().all(|p| con_frente(&p.rect)), "todo edificio tiene frente de calle");
}

#[test]
fn los_edificios_nuevos_no_pisan_las_celdas_sin_levantar_de_un_anillo_en_obra() {
    let m = con_mundo!();
    let mut e = montar(m, 7, 2, Anillo::SinEmpezar);
    let b = e.b;
    e.ok("beto", Comando::ComprometerRecinto { asentamiento: b, cargo: Cargo::MaestroObras, nivel: 1 });
    let rec = e.recintos_de_b()[0].clone();
    assert_eq!(rec.levantadas(), 1, "solo la primera celda");
    let por_levantar: Vec<_> = rec.celdas.iter().filter(|c| c.clase != ClaseCelda::Puerta).map(|c| c.celda).collect();
    let antes = e.n.ciudad(b).unwrap().piezas().len();
    for k in 0..6 {
        let tipo = if k % 2 == 0 { asentamiento::Tipo::Vivienda } else { asentamiento::Tipo::Almacen };
        e.ok("beto", Comando::AnadirEdificio { asentamiento: b, tipo });
        e.avanzar(3 * HORA);
    }
    let ciudad = e.n.ciudad(b).unwrap();
    assert!(ciudad.piezas().len() > antes + 3, "se colocaron edificios");
    for p in ciudad.piezas().iter().skip(antes) {
        let pisa = (p.rect.col..p.rect.col + p.rect.ancho).any(|c| (p.rect.row..p.rect.row + p.rect.alto).any(|f| por_levantar.contains(&trazado::Celda::new(c, f))));
        assert!(!pisa, "una pieza nueva pisa el anillo planificado: {:?}", p.tipo);
    }
    // Las celdas del anillo siguen ocupadas, levantadas o no, y el anillo (congelado) sigue siendo el mismo mientras la obra avanza.
    assert!(por_levantar.iter().all(|&c| ciudad.esta_ocupada(c)));
    assert_eq!(e.recintos_de_b()[0].celdas, rec.celdas);
    assert!(e.recintos_de_b()[0].levantadas() > 1, "y la obra avanzó mientras tanto");
}

// ───────────────────────────── Determinismo ─────────────────────────────

/// Una partida entera con las órdenes en instantes que no son de minuto: la B cierra su empalizada, la asedia el ejército de «ana» a las 26 h, cae, el anillo pierde el
/// 30 % y el nuevo dueño lo repara. `avanzar` mueve el reloj entre órdenes (normal, de un golpe o a trozos).
fn guion(m: &M, avanzar: &dyn Fn(&mut Nucleo, Instante)) -> Nucleo {
    let a = Arranque { reglas: reglas(), ajustes: Ajustes::default(), semilla: 5, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) };
    let mut n = Nucleo::nuevo(a);
    let admin = |i: &str, c| Peticion::nueva("admin", i, c).de_admin();
    let (pa, pb) = (sitio_rico(&n, &m.0, 0, 1), sitio_rico(&n, &m.0, 3, 2));
    let (ia, ib) = (fundar(&mut n, 31_000, "a", "ana", 1, pa), fundar(&mut n, 31_000, "b", "beto", 2, pb));
    let t = HORA + 13_579; // nunca coincide con un paso de minuto de la plaza
    avanzar(&mut n, t);
    for plaza in [ia, ib] {
        n.aplicar_en(t, admin(&format!("pausa-{plaza}"), Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true }));
    }
    n.aplicar_en(t, admin("nivel", Comando::FijarNivelAsentamiento { asentamiento: ib, nivel: 2 }));
    for r in asentamiento::recurso::TODOS {
        n.aplicar_en(t, admin(&format!("dar-{r:?}"), Comando::DarRecursos { asentamiento: ib, recurso: r, cantidad: 5_000_000 }));
    }
    assert!(n.aplicar_en(t + 1, comun::como_dueno(Peticion::nueva("beto", "muralla", Comando::ComprometerRecinto { asentamiento: ib, cargo: Cargo::MaestroObras, nivel: 1 }))).aceptada());
    // El asedio: «beto» dentro sin tropa y el ejército de «ana» a la puerta (cae sin combate: lo que se mide aquí es la conquista y la reparación).
    let ataque = 26 * HORA + 13_579;
    avanzar(&mut n, ataque);
    let ord = |n: &mut Nucleo, actor: &str, id: &str, c| {
        let p = Peticion::nueva(actor, id, c);
        n.aplicar_en(ataque, if actor == "admin" { p.de_admin() } else { p })
    };
    let campamento = campamento_mas_cercano(&m.0, n.estado().asentamientos[&ib].posicion);
    let beto = ord(&mut n, "beto", "h-beto", Comando::CrearHeroe { cuenta: "beto".into(), campamento, controlador: Controlador::Humano }).creadas[0];
    ord(&mut n, "admin", "f-beto", Comando::FijarFaccionDeHeroe { heroe: beto, faccion: Some(2) });
    ord(&mut n, "admin", "r-beto", Comando::FijarResidencia { heroe: beto, residencia: Some(Residencia::Asentamiento(ib as u32)) });
    ord(&mut n, "admin", "c-beto", Comando::ColocarHeroe { heroe: beto, ubicacion: Ubicacion::Asentamiento(ib as u32) });
    let p = n.estado().asentamientos[&ib].posicion;
    let col = n.crear_ejercito(ataque, "ana", 1, Punto::new(p.x + 2_000, p.y), 500, 350_000).unwrap();
    let lider = n.ejercito(col).unwrap().columna.lider;
    for tropa in ATAQUE {
        ord(&mut n, "admin", &format!("e-{tropa:?}"), Comando::DarEscuadra { heroe: lider, tropa, cantidad: completa(tropa) });
    }
    let r = ord(&mut n, "ana", "asedio", Comando::Atacar { heroe: lider, objetivo: ObjetivoAtaque::Asentamiento(ib), declara_guerra: true });
    assert!(r.aceptada(), "{r:?}");
    // La reparación del nuevo dueño: una celda cada 15 minutos durante unas horas.
    avanzar(&mut n, ataque + 4 * HORA);
    n.avanzar_hasta(ataque + 4 * HORA + 1);
    n
}

#[test]
fn la_conquista_y_la_reparacion_dan_el_mismo_mundo_normal_acelerado_y_a_trozos() {
    let m = con_mundo!();
    let normal = guion(m, &|n, t| n.avanzar_hasta(t));
    let b = hechos(&normal, "asentamiento.conquistada")[0].1[0];
    let rec = &normal.economia(b).unwrap().recintos[0];
    assert_eq!(hechos(&normal, "asentamiento.conquistada").iter().map(|h| dato(h, "murallaDanada")).collect::<Vec<_>>(), vec![1], "la conquista dañó el anillo");
    assert!(rec.completo(), "y a las 4 horas el nuevo dueño lo ha reparado ({} de {} celdas)", rec.levantadas(), rec.celdas.len());
    assert_eq!(normal.hash_estado(), guion(m, &|n, t| n.avanzar_hasta(t)).hash_estado(), "repetible");
    assert_eq!(normal.hash_estado(), guion(m, &|_, _| {}).hash_estado(), "acelerado: las órdenes llevan su instante");
    let a_trozos = guion(m, &|n, t| {
        let ahora = n.ahora();
        for s in 1..=7 {
            n.avanzar_hasta(ahora + (t - ahora).max(0) * s * s / 49);
        }
    });
    assert_eq!(normal.hash_estado(), a_trozos.hash_estado(), "en pasos pequeños e irregulares, como el modo en vivo");
    // La foto de después de la reparación recupera lo mismo.
    let bytes = normal.estado().serializar();
    let vuelta = nucleo::Estado::deserializar(&bytes, &reglas(), &m.0.hash().to_string()).unwrap();
    assert_eq!(vuelta.serializar(), bytes);
}

/// Un asedio con números se resuelve al acabar su convocatoria de 30 s (calibración del sistema 4, M8): deja pasar las que haya abiertas.
fn tras_la_convocatoria(n: &mut Nucleo) {
    let fin = n.batallas_activas().filter_map(|b| b.convocatoria_hasta).max();
    if let Some(h) = fin {
        n.avanzar_hasta(h);
    }
}
