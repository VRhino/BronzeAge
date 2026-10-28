//! Las misiones recurrentes y el tablón (mecánica 55, `Docs/Mechanics/Misiones_Recurrentes.md`): el tablón que sale al fundar, la renovación de las 00:00 (las diarias cada día, lo
//! que sale cada semana), elegir, aceptar con sus topes y su cargo, contar desde que se acepta, la grupal con su reparto, reclamar después del periodo, el catálogo en caliente, la
//! proyección y la foto. Sobre el artefacto de mundo real; las pruebas se omiten si no está.

mod comun;
use base::{MS_POR_MINUTO, UNIDAD};
use comun::real::{self, sitio_rico, M};
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::misiones::Dueno;
use nucleo::proyeccion::{ids, interior, propios};
use nucleo::recurrentes::{Catalogo, Progreso, Situacion, DIA};
use nucleo::{rechazos, Ajustes, Arranque, Comando, Nucleo, Peticion, Reglas, Respuesta, Resultado};
use std::collections::BTreeMap;
use std::sync::Arc;

const HORA: i64 = 60 * MS_POR_MINUTO;

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

/// Un catálogo pequeño para mirar cada regla: I1 individual diaria con tope 1 y oro, G1 grupal diaria, C1 semanal del Tesorero y FG grupal semanal de la Facción.
const CATALOGO: &str = r#"{
  "asentamiento": [
    {"id":"I1","duracion":"dia","tipo":"individual","limiteSemanal":1,"condiciones":{"escuadrones":1},"xp":2,"participacion":10,"oro":5,"titulo":"t","texto":"t","explicacion":"e"},
    {"id":"G1","duracion":"dia","tipo":"grupal","condiciones":{"escuadrones":2},"xp":4,"participacion":21,"titulo":"t","texto":"t","explicacion":"e"},
    {"id":"C1","duracion":"semana","tipo":"individual","limiteSemanal":3,"cargo":"tesorero","condiciones":{"batallas":1},"xp":1,"titulo":"t","texto":"t","explicacion":"e"}
  ],
  "faccion": [
    {"id":"FG","duracion":"semana","tipo":"grupal","condiciones":{"escuadrones":2},"xp":7,"titulo":"t","texto":"t","explicacion":"e"}
  ]
}"#;

/// Las reglas de siempre (con el catálogo dado), almacenes grandes y sin bandidos que acosen o aparezcan solos.
fn reglas(catalogo: Option<&str>) -> Reglas {
    let mut r = Reglas::default();
    if let Some(c) = catalogo {
        r.economia.recurrentes = Arc::new(Catalogo::desde_texto(c, &[]).unwrap());
    }
    let b = &mut Arc::make_mut(&mut r.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    Arc::make_mut(&mut r.economia.datos).capacidad_inicial = 100_000_000;
    r
}

struct X {
    n: Nucleo,
    cuenta: u64,
}

impl X {
    fn nuevo(m: &M, catalogo: Option<&str>) -> X {
        let n = Nucleo::nuevo(Arranque { reglas: reglas(catalogo), ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
        X { n, cuenta: 0 }
    }
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.cuenta += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.cuenta), c);
        let p = if actor == "admin" { p.de_admin() } else { p };
        let t = self.n.ahora();
        self.n.aplicar_en(t, p)
    }
    fn ok(&mut self, actor: &str, c: Comando) -> Respuesta {
        let r = self.ord(actor, c);
        assert!(r.aceptada(), "{r:?}");
        r
    }
    fn rechazo(&mut self, actor: &str, c: Comando) -> &'static str {
        match self.ord(actor, c).resultado {
            Resultado::Rechazada { codigo, .. } => codigo,
            otro => panic!("se esperaba un rechazo: {otro:?}"),
        }
    }
    fn hasta(&mut self, t: i64) {
        self.n.avanzar_hasta(t);
    }
    /// Una plaza de «ana» (Facción 1) junto al campamento 0, con la construcción parada y de todo para días.
    fn plaza(&mut self, m: &M) -> u64 {
        let p = sitio_rico(&self.n, &m.0, 0, 1);
        let a = real::fundar(&mut self.n, "ana", 1, p);
        self.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
        self.llenar(a);
        a
    }
    fn llenar(&mut self, plaza: u64) {
        for r in asentamiento::recurso::TODOS {
            let _ = self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso: r, cantidad: 20_000_000 });
        }
    }
    /// Un héroe de la cuenta, ciudadano de la Facción de la plaza, que reside en ella y está dentro.
    fn heroe_en(&mut self, cuenta: &str, plaza: u64) -> u64 {
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0];
        let f = self.n.estado().asentamientos[&plaza].faccion;
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(f) });
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
        h
    }
    fn gobernador(&self, plaza: u64) -> u64 {
        self.n.estado().asentamientos[&plaza].cargos.titular(asentamiento::Cargo::Gobernador).expect("quien funda la gobierna")
    }
    /// Las ids de lo que hay en el tablón de `d`, con el inicio de su periodo.
    fn en_tablon(&self, d: Dueno) -> Vec<(String, i64)> {
        self.n.tablon_de(d).unwrap().misiones.iter().map(|m| (m.def.id.clone(), m.desde)).collect()
    }
    fn mision(&self, d: Dueno, id: &str) -> &nucleo::recurrentes::EnTablon {
        let t = self.n.ahora();
        let tablon = self.n.tablon_de(d).unwrap();
        tablon.misiones.iter().find(|m| m.def.id == id && m.vigente(t)).unwrap_or_else(|| panic!("{id} no está vigente"))
    }
    fn hechos(&self, codigo: &str) -> usize {
        self.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).count()
    }
    /// Los hechos `codigo` de esas misiones (las únicas del héroe también dan `mision.cumplida`).
    fn hechos_de(&self, codigo: &str, misiones: &[&str]) -> usize {
        let es = |e: &nucleo::EventoNucleo| e.valores.iter().any(|(k, v)| *k == "mision" && misiones.iter().any(|m| *v == nucleo::Dato::Texto(m.to_string())));
        self.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo && es(e)).count()
    }
    fn experiencia(&self, f: u32) -> i64 {
        self.n.estado().politica.facciones.get(f).unwrap().experiencia
    }
    fn participacion(&self, plaza: u64, h: u64) -> u64 {
        self.n.participacion_en(plaza).and_then(|p| p.get(&h)).copied().unwrap_or(0)
    }
    fn oro_personal(&self, h: u64) -> i64 {
        self.n.heroe(h).unwrap().almacen_personal.get(&asentamiento::Recurso::Oro).copied().unwrap_or(0)
    }
}

fn aceptar(heroe: u64, dueno: Dueno, mision: &str) -> Comando {
    Comando::AceptarMision { heroe, dueno, mision: mision.into() }
}

fn reclamar(heroe: u64, dueno: Dueno, mision: &str) -> Comando {
    Comando::ReclamarRecompensa { heroe, dueno, mision: mision.into() }
}

fn ids_de(l: &[&str]) -> Vec<String> {
    l.iter().map(|s| s.to_string()).collect()
}

#[test]
fn el_tablon_sale_al_fundar_las_diarias_se_renuevan_cada_dia_y_lo_elegido_sale_en_la_renovacion_semanal() {
    let m = con_mundo!();
    let mut x = X::nuevo(m, None);
    let a = x.plaza(m);
    let (pa, f1) = (Dueno::Asentamiento(a), Dueno::Faccion(1));
    let fundada = x.n.estado().asentamientos[&a].fundado_en;
    // Al fundar: las 7 primeras diarias y las 7 primeras semanales del catálogo, desde ya; la Facción, las suyas (4 y 4).
    let t = x.n.tablon_de(pa).expect("la plaza nace con tablón");
    assert_eq!(t.diarias, ids_de(&["PD1", "PD2", "PD3", "PD4", "PD5", "PD6", "PD7"]));
    assert_eq!(t.semanales, ids_de(&["PS1", "PS2", "PS3", "PS4", "PS5", "PS6", "PS7"]));
    assert!(t.misiones.iter().all(|m| m.desde == fundada && m.hasta == if m.def.id.starts_with("PD") { DIA } else { 7 * DIA }));
    assert_eq!(x.n.tablon_de(f1).expect("y su Facción").misiones.len(), 8);
    assert_eq!(x.n.recurrentes().renovacion_en, Some(DIA));

    // Elegir: solo el Gobernador, hasta 7 por lista y del catálogo con su ámbito y su duración.
    let g = x.gobernador(a);
    let otro = x.heroe_en("ana", a);
    let elegir = |h: u64, d: &[&str], s: &[&str]| Comando::ElegirTablon { heroe: h, dueno: pa, diarias: ids_de(d), semanales: ids_de(s) };
    assert_eq!(x.rechazo("ana", elegir(otro, &["PD8"], &[])), rechazos::NO_AUTORIZADO);
    assert_eq!(x.rechazo("ana", elegir(g, &["PD1", "PD2", "PD3", "PD4", "PD5", "PD6", "PD7", "PD8"], &[])), rechazos::TABLON_ELECCION_INVALIDA);
    assert_eq!(x.rechazo("ana", elegir(g, &["PS1"], &[])), rechazos::TABLON_ELECCION_INVALIDA, "una semanal entre las diarias");
    assert_eq!(x.rechazo("ana", elegir(g, &["FD1"], &[])), rechazos::TABLON_ELECCION_INVALIDA, "una de Facción en una plaza");
    assert_eq!(x.rechazo("ana", elegir(g, &["PD8", "PD8"], &[])), rechazos::TABLON_ELECCION_INVALIDA);
    x.ok("ana", elegir(g, &["PD9", "PD8"], &["PS10"]));
    assert_eq!(x.ord("ana", elegir(g, &["PD9", "PD8"], &["PS10"])).resultado, Resultado::SinCambios);
    let antes = x.en_tablon(pa);
    assert_eq!(x.n.tablon_de(pa).unwrap().diarias[0], "PD1", "lo elegido no cambia lo que sale hasta la renovación semanal");

    // Las 00:00: las diarias salen otra vez (las mismas de la semana); las semanales siguen.
    x.hasta(DIA + 1);
    let ahora = x.en_tablon(pa);
    assert_eq!(ahora.len(), 14);
    for (id, desde) in &ahora {
        assert_eq!(*desde, if id.starts_with("PD") { DIA } else { fundada }, "{id}");
    }
    let ids = |v: &[(String, i64)]| v.iter().map(|(i, _)| i.clone()).collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids(&ahora), ids(&antes), "las mismas de la semana");
    assert_eq!(x.hechos("tablon.renovado"), 1);
    assert_eq!(x.n.recurrentes().renovacion_en, Some(2 * DIA));

    // La renovación semanal (7 días desde el primer día de la partida): sale lo elegido.
    x.llenar(a);
    x.hasta(7 * DIA + 1);
    let t = x.n.tablon_de(pa).unwrap();
    assert_eq!((t.diarias.clone(), t.semanales.clone()), (ids_de(&["PD9", "PD8"]), ids_de(&["PS10"])));
    assert_eq!(x.en_tablon(pa), [("PD9".into(), 7 * DIA), ("PD8".into(), 7 * DIA), ("PS10".into(), 7 * DIA)]);
    assert_eq!(x.n.tablon_de(f1).unwrap().misiones.len(), 8, "la Facción sigue con lo de por defecto");
    assert_eq!(x.hechos("tablon.renovado"), 7);

    // La proyección: lo que sale, lo elegido, las dos renovaciones y el catálogo de su ámbito.
    let p = interior::asentamiento(&x.n, a).unwrap().tablon.expect("la plaza lleva su tablón");
    assert_eq!(p.misiones.len(), 3);
    assert_eq!((p.renovacion_diaria_ms, p.renovacion_semanal_ms), (8 * DIA, 14 * DIA));
    assert!(p.con_eleccion && p.elegidas_diarias_ids == ["PD9", "PD8"]);
    assert_eq!(p.catalogo.len(), 20);
    let f = propios::faccion(&x.n, 1);
    assert_eq!(f.tablon.as_ref().unwrap().catalogo.len(), 8);
    assert!(propios::faccion_publica(&x.n, 1).tablon.is_none(), "la forma pública no lo lleva");
}

#[test]
fn aceptar_contar_desde_entonces_repartir_la_grupal_y_reclamar_despues_del_periodo() {
    let m = con_mundo!();
    let mut x = X::nuevo(m, Some(CATALOGO));
    let a = x.plaza(m);
    let (pa, f1) = (Dueno::Asentamiento(a), Dueno::Faccion(1));
    // Población para reclutar dos escuadras: la plaza vuelve a construir sola unos días.
    x.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: false });
    for d in 1..=3 {
        x.hasta(d * DIA + HORA);
        x.llenar(a);
    }
    let g = x.gobernador(a);
    let (h, h2) = (x.heroe_en("ana", a), x.heroe_en("carla", a));
    let fuera = x.ok("beto", Comando::CrearHeroe { cuenta: "beto".into(), campamento: 3, controlador: Controlador::Humano, nombre: None }).creadas[0];

    // Los rechazos de aceptar.
    assert_eq!(x.rechazo("beto", aceptar(fuera, pa, "I1")), rechazos::MISION_FUERA_DE_AMBITO);
    assert_eq!(x.rechazo("ana", aceptar(h, pa, "G1")), rechazos::MISION_GRUPAL);
    assert_eq!(x.rechazo("ana", aceptar(h, pa, "C1")), rechazos::MISION_PIDE_CARGO);
    assert_eq!(x.rechazo("ana", aceptar(h, pa, "ZZ")), rechazos::MISION_DESCONOCIDA);
    assert_eq!(x.rechazo("ana", aceptar(h, Dueno::Heroe(h), "I1")), rechazos::TABLON_DESCONOCIDO);
    assert_eq!(x.rechazo("beto", aceptar(h, pa, "I1")), rechazos::NO_AUTORIZADO, "solo la cuenta del héroe");
    x.ok("ana", aceptar(h, pa, "I1"));
    assert_eq!(x.hechos("mision.aceptada"), 1);
    assert_eq!(x.rechazo("ana", aceptar(h, pa, "I1")), rechazos::MISION_YA_ACEPTADA);
    assert_eq!(x.rechazo("carla", aceptar(h2, pa, "I1")), rechazos::MISION_TOPE_SEMANAL, "tope 1 por semana en el tablón");

    // h recluta una escuadra: cumple su I1 y aporta a las grupales de la plaza y de la Facción.
    x.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::MiliciaLanceros });
    let i1 = x.mision(pa, "I1");
    assert!(matches!(&i1.progreso, Progreso::Individual(m) if m[&h].situacion == Situacion::Cumplida));
    let g1 = x.mision(pa, "G1");
    assert!(matches!(&g1.progreso, Progreso::Grupal { aportes, situacion: Situacion::Aceptada, .. } if *aportes == BTreeMap::from([(h, 1)])));
    // h2 recluta la suya: la grupal se cumple con lo de los dos.
    x.ok("carla", Comando::ReclutarTropa { asentamiento: a, heroe: h2, tropa: Tropa::MiliciaLanceros });
    assert!(matches!(&x.mision(pa, "G1").progreso, Progreso::Grupal { aportes, situacion: Situacion::Cumplida, .. } if *aportes == BTreeMap::from([(h, 1), (h2, 1)])));
    assert!(matches!(&x.mision(f1, "FG").progreso, Progreso::Grupal { situacion: Situacion::Cumplida, .. }));
    assert_eq!(x.hechos_de("mision.cumplida", &["I1", "G1", "FG"]), 3);

    // Reclamar la individual: solo quien la cumplió; oro a su almacén personal, participación en la plaza y la experiencia a la Facción.
    assert_eq!(x.rechazo("carla", reclamar(h2, pa, "I1")), rechazos::MISION_NO_CUMPLIDA);
    let (xp, oro) = (x.experiencia(1), x.oro_personal(h));
    x.ok("ana", reclamar(h, pa, "I1"));
    assert_eq!((x.experiencia(1) - xp, x.oro_personal(h) - oro, x.participacion(a, h)), (2 * UNIDAD, 5 * UNIDAD, 10));
    assert_eq!(x.rechazo("ana", reclamar(h, pa, "I1")), rechazos::MISION_YA_RECLAMADA);

    // La proyección lleva cada aceptación con su héroe y su estado, y lo que aportó cada uno a la grupal.
    let p = interior::asentamiento(&x.n, a).unwrap().tablon.unwrap();
    let pi1 = p.misiones.iter().find(|m| m.definicion.as_ref().unwrap().id == "I1").unwrap();
    assert_eq!((pi1.aceptaciones.len(), pi1.aceptaciones[0].heroe_id.clone(), pi1.aceptadas_semana), (1, ids::heroe(h), 1));
    assert_eq!(pi1.aceptaciones[0].estado, contrato::v1::EstadoMisionRecurrente::RecurrenteReclamada as i32);
    let pg1 = p.misiones.iter().find(|m| m.definicion.as_ref().unwrap().id == "G1").unwrap();
    assert_eq!(pg1.aportes, BTreeMap::from([(ids::heroe(h), 1), (ids::heroe(h2), 1)]));

    // Pasan las 00:00: la G1 cumplida sigue en el tablón como vencida hasta que se reclama, y sale otra; el tope de I1 sigue (es de la semana).
    x.hasta(4 * DIA + HORA);
    let vencidas: Vec<_> = x.n.tablon_de(pa).unwrap().misiones.iter().filter(|m| m.def.id == "G1").map(|m| m.desde).collect();
    assert_eq!(vencidas, [3 * DIA, 4 * DIA]);
    assert_eq!(x.rechazo("carla", aceptar(h2, pa, "I1")), rechazos::MISION_TOPE_SEMANAL);
    // La grupal la reclama el Gobernador: la participación a partes iguales (21 / 2 = 10; el resto se pierde) y la experiencia a la Facción.
    assert_eq!(x.rechazo("carla", reclamar(h2, pa, "G1")), rechazos::NO_AUTORIZADO);
    let xp = x.experiencia(1);
    x.ok("ana", reclamar(g, pa, "G1"));
    assert_eq!((x.participacion(a, h), x.participacion(a, h2), x.experiencia(1) - xp), (20, 10, 4 * UNIDAD));
    assert_eq!(x.n.tablon_de(pa).unwrap().misiones.iter().filter(|m| m.def.id == "G1").count(), 1, "la vencida ya reclamada se va");
    assert_eq!(x.rechazo("ana", reclamar(g, pa, "G1")), rechazos::MISION_NO_CUMPLIDA, "la de hoy no se ha cumplido");
    // La de la Facción: su Rey o la administración.
    let xp = x.experiencia(1);
    x.ok("admin", reclamar(0, f1, "FG"));
    assert_eq!(x.experiencia(1) - xp, 7 * UNIDAD);

    // La foto lleva los tablones (BAN50) y se lee igual.
    let bytes = x.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN50");
    let vuelta = nucleo::Estado::deserializar(&bytes, &reglas(Some(CATALOGO)), &x.n.estado().mundo_hash).unwrap();
    assert_eq!(vuelta.recurrentes, x.n.estado().recurrentes);
    assert_eq!(vuelta.serializar(), bytes);
}

#[test]
fn el_catalogo_se_cambia_en_caliente_y_vale_desde_la_siguiente_renovacion() {
    let m = con_mundo!();
    let mut x = X::nuevo(m, Some(CATALOGO));
    let a = x.plaza(m);
    let pa = Dueno::Asentamiento(a);
    let fijar = |c: &str| Comando::FijarCatalogoMisiones { catalogo: c.into() };
    // Solo la administración, y entero o nada.
    assert_eq!(x.rechazo("ana", fijar(CATALOGO)), rechazos::NO_AUTORIZADO);
    assert_eq!(x.rechazo("admin", fijar("{")), rechazos::MISION_CATALOGO_INVALIDO);
    assert_eq!(x.rechazo("admin", fijar(&CATALOGO.replace("\"I1\"", "\"A1\""))), rechazos::MISION_CATALOGO_INVALIDO, "A1 es una misión única");
    assert_eq!(x.rechazo("admin", fijar(&CATALOGO.replace("\"escuadrones\":1", "\"volar\":1"))), rechazos::MISION_CATALOGO_INVALIDO);
    assert_eq!(x.ord("admin", fijar(CATALOGO)).resultado, Resultado::SinCambios);
    // I1 da ahora 9 de experiencia: la de hoy sigue con 2; la de mañana sale con 9.
    let nuevo = CATALOGO.replace("\"xp\":2", "\"xp\":9");
    x.ok("admin", fijar(&nuevo));
    assert_eq!(x.mision(pa, "I1").def.xp, 2 * UNIDAD);
    assert_eq!(x.n.recurrentes().catalogo.buscar("I1").unwrap().xp, 9 * UNIDAD);
    x.hasta(DIA + 1);
    assert_eq!(x.mision(pa, "I1").def.xp, 9 * UNIDAD);
    // Una que se quita del catálogo deja de salir en la siguiente renovación.
    let sin_g1 = Catalogo::desde_texto(&nuevo, &[]).unwrap();
    let mut v = sin_g1.a_json();
    v["asentamiento"].as_array_mut().unwrap().retain(|m| m["id"] != "G1");
    x.ok("admin", fijar(&v.to_string()));
    x.hasta(2 * DIA + 1);
    assert!(x.n.tablon_de(pa).unwrap().misiones.iter().all(|m| m.def.id != "G1"));
    // El catálogo viaja en la foto.
    let bytes = x.n.estado().serializar();
    let vuelta = nucleo::Estado::deserializar(&bytes, &reglas(Some(CATALOGO)), &x.n.estado().mundo_hash).unwrap();
    assert_eq!(*vuelta.recurrentes.catalogo, *x.n.recurrentes().catalogo);
}
