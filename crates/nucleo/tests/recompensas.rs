//! El reparto de la recompensa al reclamar (mecánica 66, `Docs/Mechanics/Misiones.md` §4 y `Misiones_Recurrentes.md` §5): todo o nada, la tropa que nace en el campamento o se completa, el
//! destino de cada misión (héroe, plaza, capital de la Facción), la experiencia a la Facción de quien cobra, la participación de las cadenas de plaza y lo que cuenta el hecho
//! `mision.reclamada`. Sobre el artefacto de mundo real; las pruebas se omiten si no está.

mod comun;
use asentamiento::Recurso;
use base::{MS_POR_MINUTO, UNIDAD};
use comun::real::{self, sitio_rico, M};
use militar::catalogo::Tropa;
use militar::columna::Politica;
use militar::escuadra::Contenedor;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::misiones::Dueno;
use nucleo::proyeccion::ids;
use nucleo::recurrentes::{Catalogo, Progreso, Situacion, DIA};
use nucleo::{rechazos, Ajustes, Arranque, Comando, Dato, EventoNucleo, Nucleo, Peticion, Reglas, Respuesta, Resultado};
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

/// Los soldados de una escuadra completa de `tropa`.
fn completa(tropa: Tropa) -> i64 {
    militar::Reglas::cargar().tropas.tropa(tropa).unidades
}

/// Las cadenas de la prueba, todas sin previa: las del héroe se cumplen al salir del campamento y las de la plaza al subir a nivel 2. `HNUEVA` y `HTOPE` dan tropa; `HCOMPLETA` da de todo; `HOBJ`, objetos (entrada 67).
fn misiones() -> String {
    let tope = completa(Tropa::Arqueros);
    format!(
        r#"{{
  "heroe": [
    {{"id":"HNUEVA","condiciones":{{"salidas":1}},"recompensas":[{{"tipo":"tropa","tropa":"arqueros","cantidad":4}}]}},
    {{"id":"HTOPE","condiciones":{{"salidas":1}},"recompensas":[{{"tipo":"tropa","tropa":"arqueros","cantidad":{tope}}}]}},
    {{"id":"HSIN","condiciones":{{"salidas":1}},"recompensas":[{{"tipo":"xp","cantidad":3}},{{"tipo":"oro","cantidad":4}}]}},
    {{"id":"HCOMPLETA","condiciones":{{"salidas":1}},"recompensas":[{{"tipo":"oro","cantidad":4}},{{"tipo":"recurso","recurso":"madera","cantidad":3}},{{"tipo":"xp","cantidad":2}},{{"tipo":"tropa","tropa":"honderos","cantidad":2}}]}},
    {{"id":"HOBJ","condiciones":{{"salidas":1}},"recompensas":[{{"tipo":"objeto","objeto":"medalla_heroe_grande","cantidad":2}},{{"tipo":"objeto","objeto":"dagas_cobre","cantidad":1}}]}}
  ],
  "asentamiento": [
    {{"id":"AP","condiciones":{{"nivel":2}},"participacion":10,"recompensas":[{{"tipo":"recurso","recurso":"madera","cantidad":3}}]}},
    {{"id":"AQ","condiciones":{{"nivel":2}},"participacion":10,"recompensas":[{{"tipo":"recurso","recurso":"piedra","cantidad":3}}]}}
  ],
  "faccion": [
    {{"id":"FA","condiciones":{{"capital":1}},"xp":5}}
  ]
}}"#
    )
}

/// Las recurrentes de la prueba: `IT` pide sitio para el oro y la madera juntos, `I1` es la individual de siempre, `FO` es una grupal de Facción con oro (se cumple con una salida).
const RECURRENTES: &str = r#"{
  "asentamiento": [
    {"id":"I1","duracion":"dia","tipo":"individual","limiteSemanal":1,"condiciones":{"escuadrones":1},"participacion":10,"recompensas":[{"tipo":"xp","cantidad":2},{"tipo":"oro","cantidad":5}],"titulo":"t","texto":"t","explicacion":"e"},
    {"id":"IT","duracion":"dia","tipo":"individual","limiteSemanal":1,"condiciones":{"escuadrones":1},"participacion":10,"recompensas":[{"tipo":"xp","cantidad":2},{"tipo":"oro","cantidad":300},{"tipo":"recurso","recurso":"madera","cantidad":300}],"titulo":"t","texto":"t","explicacion":"e"}
  ],
  "faccion": [
    {"id":"FO","duracion":"semana","tipo":"grupal","condiciones":{"salidas":1},"xp":7,"recompensas":[{"tipo":"oro","cantidad":3}],"titulo":"t","texto":"t","explicacion":"e"}
  ]
}"#;

/// Las reglas de siempre con las cadenas y las recurrentes de la prueba, almacenes grandes y sin bandidos que acosen o aparezcan solos.
fn reglas() -> Reglas {
    let mut r = Reglas::default();
    r.economia.misiones = Arc::new(nucleo::misiones::Reglas::desde(&serde_json::from_str(&misiones()).unwrap(), &r.economia.militar).unwrap());
    r.economia.recurrentes = Arc::new(Catalogo::desde_texto(RECURRENTES, &[], &r.economia.militar).unwrap());
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
    fn nuevo(m: &M) -> X {
        let n = Nucleo::nuevo(Arranque { reglas: reglas(), ajustes: Ajustes::default(), semilla: 7, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) });
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
    /// Una plaza de la Facción 1 fundada por `cuenta` junto al campamento `i`, con la construcción parada y de todo para días.
    fn plaza(&mut self, m: &M, cuenta: &str, i: usize) -> u64 {
        let p = sitio_rico(&self.n, &m.0, i, 1);
        let a = real::fundar(&mut self.n, cuenta, 1, p);
        self.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: a, pausada: true });
        self.llenar(a);
        a
    }
    /// Un héroe de la cuenta, ciudadano de la Facción de la plaza, que reside en ella y está dentro.
    fn heroe_en(&mut self, cuenta: &str, plaza: u64) -> u64 {
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano, nombre: None, clase: "espada_escudo_mediano".into(), aspecto: Default::default() }).creadas[0];
        let f = self.n.estado().asentamientos[&plaza].faccion;
        self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(f) });
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
        h
    }
    /// Un héroe de la cuenta (de esa Facción) que reside en el campamento 0 y sale de él con un préstamo: va en una columna y ha cumplido las cadenas de héroe de la prueba. Devuelve (héroe, columna).
    fn saliendo(&mut self, cuenta: &str, faccion: Option<u32>) -> (u64, u64) {
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano, nombre: None, clase: "espada_escudo_mediano".into(), aspecto: Default::default() }).creadas[0];
        if faccion.is_some() {
            self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion });
        }
        let escuadras = self.ok(cuenta, Comando::PedirPrestamo { heroe: h, tropas: vec![Tropa::Granjeros] }).creadas;
        let col = self.ok(cuenta, Comando::SalirDelCampamento { campamento: 0, heroe: h, escuadras, carga: vec![], politica: Politica::Rechazar, caravana_fundacion: None }).creadas[0];
        (h, col)
    }
    /// Deja que la plaza construya sola unos días para que tenga población que reclutar.
    fn con_poblacion(&mut self, plaza: u64) {
        self.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: false });
        for d in 1..=3 {
            let t = d * DIA + HORA;
            self.n.avanzar_hasta(t);
            self.llenar(plaza);
        }
    }
    fn llenar(&mut self, plaza: u64) {
        for r in asentamiento::recurso::TODOS {
            let _ = self.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso: r, cantidad: 20_000_000 });
        }
    }
    fn gobernador(&self, plaza: u64) -> u64 {
        self.n.estado().asentamientos[&plaza].cargos.titular(asentamiento::Cargo::Gobernador).expect("quien funda la gobierna")
    }
    fn experiencia(&self, f: u32) -> i64 {
        self.n.estado().politica.facciones.get(f).unwrap().experiencia
    }
    fn experiencia_total(&self) -> i64 {
        self.n.estado().politica.facciones.iter().map(|f| f.experiencia).sum()
    }
    fn participacion(&self, plaza: u64, h: u64) -> u64 {
        self.n.participacion_en(plaza).and_then(|p| p.get(&h)).copied().unwrap_or(0)
    }
    fn participacion_total(&self, plaza: u64) -> u64 {
        self.n.participacion_en(plaza).map_or(0, |p| p.values().sum())
    }
    fn personal(&self, h: u64, r: Recurso) -> i64 {
        self.n.heroe(h).unwrap().almacen_personal.get(&r).copied().unwrap_or(0)
    }
    fn en_plaza(&self, a: u64, r: Recurso) -> i64 {
        self.n.economia(a).unwrap().almacen.cantidad(r)
    }
    /// Los hechos `mision.reclamada`, en orden.
    fn reclamadas(&self) -> Vec<EventoNucleo> {
        self.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == "mision.reclamada").cloned().collect()
    }
    /// ¿Sigue la cadena `id` de `d` cumplida y sin reclamar?
    fn por_reclamar(&self, d: Dueno, id: &str) -> bool {
        let i = self.n.estado().reglas.economia.misiones.buscar(d.clase(), id).unwrap();
        let x = self.n.misiones_de(d).unwrap();
        x.cumplidas.contains(&i) && !x.reclamadas.contains(&i)
    }
    fn situacion(&self, d: Dueno, id: &str, h: u64) -> Situacion {
        let m = self.n.tablon_de(d).unwrap().misiones.iter().find(|m| m.def.id == id).unwrap();
        match &m.progreso {
            Progreso::Individual(a) => a[&h].situacion,
            Progreso::Grupal { situacion, .. } => *situacion,
        }
    }
    fn escuadra(&self, h: u64, tropa: Tropa) -> &militar::escuadra::Escuadra {
        self.n.heroe(h).unwrap().heroe.escuadra_de(tropa).expect("tiene la escuadra")
    }
}

fn aceptar(heroe: u64, dueno: Dueno, mision: &str) -> Comando {
    Comando::AceptarMision { heroe, dueno, mision: mision.into() }
}

fn reclamar(heroe: u64, dueno: Dueno, mision: &str) -> Comando {
    Comando::ReclamarRecompensa { heroe, dueno, mision: mision.into() }
}

fn lista(l: Vec<Dato>) -> Dato {
    Dato::Lista(l)
}

fn texto(s: &str) -> Dato {
    Dato::Texto(s.into())
}

/// Los `datos` y los `valores` de un hecho.
type LoDado = (Vec<(&'static str, i64)>, Vec<(&'static str, Dato)>);

/// Lo que el hecho cuenta de la recompensa, sin `mision` ni `dueno`.
fn lo_dado(e: &EventoNucleo) -> LoDado {
    (e.datos.clone(), e.valores.iter().filter(|(k, _)| *k != "mision" && *k != "dueno").cloned().collect())
}

#[test]
fn reclamar_es_todo_o_nada_si_no_caben_juntos_el_oro_y_el_recurso_no_se_da_nada_y_se_reclama_despues() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza(m, "ana", 0);
    let pa = Dueno::Asentamiento(a);
    x.con_poblacion(a);
    let h = x.heroe_en("ana", a);
    // El almacén personal admite 2000 (desde el 2026-11-05): con 1600 de piedra cabe el oro (300) o la madera (300), pero no los dos juntos.
    x.ok("admin", Comando::DarAlmacenPersonal { heroe: h, recurso: Recurso::Piedra, cantidad: 1_600 * UNIDAD });
    x.ok("ana", aceptar(h, pa, "IT"));
    x.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::MiliciaLanceros });
    assert_eq!(x.situacion(pa, "IT", h), Situacion::Cumplida);

    let (almacen, xp, participacion) = (x.n.heroe(h).unwrap().almacen_personal.clone(), x.experiencia(1), x.participacion(a, h));
    assert_eq!(x.rechazo("ana", reclamar(h, pa, "IT")), rechazos::ALMACEN_PERSONAL_LLENO);
    assert_eq!(x.n.heroe(h).unwrap().almacen_personal, almacen, "ni el oro ni la madera");
    assert_eq!((x.experiencia(1), x.participacion(a, h)), (xp, participacion), "ni la experiencia ni la participación");
    assert_eq!(x.situacion(pa, "IT", h), Situacion::Cumplida, "sigue por reclamar");
    assert!(x.reclamadas().is_empty());

    // Con sitio para todo, se reclama: lo de siempre y ahora sí.
    x.ok("ana", Comando::SacarDelAlmacenPersonal { heroe: h, recurso: Recurso::Piedra, cantidad: 500 * UNIDAD });
    x.ok("ana", reclamar(h, pa, "IT"));
    assert_eq!((x.personal(h, Recurso::Oro), x.personal(h, Recurso::Madera)), (300 * UNIDAD, 300 * UNIDAD));
    assert_eq!((x.experiencia(1) - xp, x.participacion(a, h) - participacion), (2 * UNIDAD, 10));
    assert_eq!(x.situacion(pa, "IT", h), Situacion::Reclamada);
}

#[test]
fn la_tropa_nueva_nace_en_el_campamento_aunque_el_heroe_vaya_en_columna_y_el_hecho_la_cuenta() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (h, col) = x.saliendo("ana", None);
    assert_eq!(x.n.estado().ejercito_de_heroe(h), Some(col));
    assert!(x.n.heroe(h).unwrap().heroe.escuadra_de(Tropa::Arqueros).is_none());
    x.ok("ana", reclamar(h, Dueno::Heroe(h), "HNUEVA"));
    let e = x.escuadra(h, Tropa::Arqueros);
    assert_eq!((e.cantidad, e.contenedor, e.plaza_origen, e.prestada), (4, Contenedor::Campamento, None, None), "nace en el campamento, no en la columna");
    let id = e.id;
    let (datos, valores) = lo_dado(&x.reclamadas()[0]);
    assert_eq!((datos, valores), (vec![], vec![("escuadras", lista(vec![lista(vec![texto("arqueros"), Dato::Numero(4), texto(&ids::escuadra(id))])]))]));
}

#[test]
fn la_tropa_que_ya_tiene_se_completa_hasta_el_tope_lo_que_sobra_se_pierde_y_la_columna_se_recompone() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (h, col) = x.saliendo("ana", None);
    let tope = completa(Tropa::Arqueros);
    // Ya lleva una escuadra de arqueros en su columna, a dos soldados de estar completa.
    x.ok("admin", Comando::DarEscuadra { heroe: h, tropa: Tropa::Arqueros, cantidad: tope - 2 });
    let (id, contenedor) = (x.escuadra(h, Tropa::Arqueros).id, x.escuadra(h, Tropa::Arqueros).contenedor);
    assert_eq!(contenedor, Contenedor::Ejercito(col));
    let come = |x: &X| x.n.ejercito(col).unwrap().columna.viveres.por_minuto;
    let come_antes = come(&x);

    // La recompensa pide el tope entero: se suman dos, el resto se pierde y no hay rechazo.
    x.ok("ana", reclamar(h, Dueno::Heroe(h), "HTOPE"));
    let e = x.escuadra(h, Tropa::Arqueros);
    assert_eq!((e.id, e.cantidad, e.contenedor), (id, tope, Contenedor::Ejercito(col)), "la misma escuadra, completa y donde estaba");
    assert_eq!(x.n.heroe(h).unwrap().heroe.escuadras.iter().filter(|e| e.tropa == Tropa::Arqueros).count(), 1, "una escuadra por tropa");
    assert!(come(&x) < come_antes, "con más soldados la columna come más (el ritmo es más negativo): {} y {come_antes}", come(&x));
    let (datos, valores) = lo_dado(&x.reclamadas()[0]);
    assert_eq!((datos, valores), (vec![], vec![("escuadras", lista(vec![lista(vec![texto("arqueros"), Dato::Numero(2), texto(&ids::escuadra(id))])]))]), "cuenta los soldados que de verdad se sumaron");
}

#[test]
fn la_tropa_sin_residencia_se_rechaza_y_la_mision_sigue_por_reclamar() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (h, _) = x.saliendo("ana", None);
    let d = Dueno::Heroe(h);
    x.ok("admin", Comando::FijarResidencia { heroe: h, residencia: None });
    assert_eq!(x.rechazo("ana", reclamar(h, d, "HNUEVA")), rechazos::HEROE_SIN_RESIDENCIA, "la tropa nace en el campamento donde reside");
    assert!(x.n.heroe(h).unwrap().heroe.escuadra_de(Tropa::Arqueros).is_none());
    assert!(x.por_reclamar(d, "HNUEVA"));
    // Con residencia, se reclama.
    x.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Campamento(0)) });
    x.ok("ana", reclamar(h, d, "HNUEVA"));
    assert_eq!(x.escuadra(h, Tropa::Arqueros).cantidad, 4);
}

#[test]
fn la_grupal_de_faccion_con_oro_va_a_la_capital_efectiva_y_sin_plazas_se_rechaza() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    // Dos plazas de la Facción 1 (con el nivel para tenerlas): la capital efectiva es la más antigua. Y una Facción 2 sin ninguna plaza.
    x.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 3 });
    let (capital, otra) = (x.plaza(m, "ana", 0), x.plaza(m, "beto", 1));
    let sin_plazas = 2;
    x.ok("admin", Comando::FijarNivelFaccion { faccion: sin_plazas, nivel: 2 });
    assert_eq!(x.n.capital_de(1), Some(capital));
    // Cada uno sale del campamento siendo de su Facción: la grupal FO de cada Facción se cumple.
    x.saliendo("carla", Some(1));
    x.saliendo("dora", Some(sin_plazas));
    let (oro_capital, oro_otra, xp) = (x.en_plaza(capital, Recurso::Oro), x.en_plaza(otra, Recurso::Oro), x.experiencia(1));
    x.ok("admin", reclamar(0, Dueno::Faccion(1), "FO"));
    assert_eq!(x.en_plaza(capital, Recurso::Oro) - oro_capital, 3 * UNIDAD, "el oro va a la capital");
    assert_eq!(x.en_plaza(otra, Recurso::Oro), oro_otra, "y a ninguna otra plaza");
    assert_eq!(x.experiencia(1) - xp, 7 * UNIDAD);

    let xp = x.experiencia_total();
    assert_eq!(x.rechazo("admin", reclamar(0, Dueno::Faccion(sin_plazas), "FO")), rechazos::MISION_SIN_CAPITAL);
    assert_eq!(x.experiencia_total(), xp, "ni la experiencia: sigue por reclamar");
    assert_eq!(x.reclamadas().len(), 1);
}

#[test]
fn un_heroe_sin_faccion_cobra_el_oro_de_su_cadena_y_no_suma_experiencia() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (h, _) = x.saliendo("ana", None);
    assert_eq!(x.n.heroe(h).unwrap().faccion, None);
    let (oro, xp) = (x.personal(h, Recurso::Oro), x.experiencia_total());
    x.ok("ana", reclamar(h, Dueno::Heroe(h), "HSIN"));
    assert_eq!(x.personal(h, Recurso::Oro) - oro, 4 * UNIDAD, "cobra el oro");
    assert_eq!(x.experiencia_total(), xp, "y no hay Facción a la que sumar la xp");
    assert_eq!(lo_dado(&x.reclamadas()[0]), (vec![("oro", 4 * UNIDAD)], vec![]), "el hecho no cuenta la xp que no se dio");
}

#[test]
fn la_xp_de_una_recurrente_individual_de_plaza_va_a_la_faccion_del_heroe_y_no_a_la_de_la_plaza() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza(m, "ana", 0);
    let pa = Dueno::Asentamiento(a);
    x.con_poblacion(a);
    let h = x.heroe_en("ana", a);
    x.ok("ana", aceptar(h, pa, "I1"));
    x.ok("ana", Comando::ReclutarTropa { asentamiento: a, heroe: h, tropa: Tropa::MiliciaLanceros });
    // El héroe es ahora de otra Facción, aunque resida en la plaza de la 1.
    let otra = 2;
    x.ok("admin", Comando::FijarNivelFaccion { faccion: otra, nivel: 2 });
    x.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(otra) });
    let (propia, de_la_plaza) = (x.experiencia(otra), x.experiencia(1));
    x.ok("ana", reclamar(h, pa, "I1"));
    assert_eq!((x.experiencia(otra) - propia, x.experiencia(1) - de_la_plaza), (2 * UNIDAD, 0));
}

#[test]
fn la_cadena_de_plaza_da_10_de_participacion_a_quien_reclama_y_a_nadie_si_reclama_la_administracion() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza(m, "ana", 0);
    let pa = Dueno::Asentamiento(a);
    let g = x.gobernador(a);
    x.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel: 2 });
    assert!(x.por_reclamar(pa, "AP") && x.por_reclamar(pa, "AQ"));
    let total = x.participacion_total(a);
    x.ok("ana", reclamar(g, pa, "AP"));
    assert_eq!((x.participacion(a, g), x.participacion_total(a) - total), (10, 10), "10 a quien reclama");
    // La administración reclama con el héroe 0, que no existe: la participación no es de nadie.
    let total = x.participacion_total(a);
    x.ok("admin", reclamar(0, pa, "AQ"));
    assert_eq!(x.participacion_total(a), total);
    let hechos = x.reclamadas();
    assert_eq!(lo_dado(&hechos[0]).0, vec![("participacion", 10)]);
    assert!(lo_dado(&hechos[1]).0.is_empty(), "sin nadie con quien repartir, el hecho no cuenta participación");
}

/// Entrada 67: una misión personal da objetos, a la mochila del héroe que cobra (aunque no tenga residencia: no van al almacén personal ni al campamento), con su hecho `heroe.botin`
/// de origen `mision`. Los de la lista no se cuentan en `mision.reclamada`.
#[test]
fn una_mision_personal_da_objetos_a_la_mochila_de_quien_la_cobra() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (h, _) = x.saliendo("ana", None);
    x.ok("admin", Comando::FijarResidencia { heroe: h, residencia: None });
    let rng = x.n.estado().rng.estado();
    x.ok("ana", reclamar(h, Dueno::Heroe(h), "HOBJ"));
    assert_eq!(x.n.estado().rng.estado(), rng, "sin tabla: no tira nada");
    let objetos = vec![("medalla_heroe_grande".to_owned(), 2), ("dagas_cobre".to_owned(), 1)];
    assert_eq!(comun::botines(&x.n), vec![comun::Botin { heroe: h, origen: "mision".into(), mochila: objetos.clone(), almacen: vec![], perdido: vec![] }]);
    let mochila = &x.n.heroe(h).unwrap().heroe.mochila;
    assert_eq!(mochila.casillas.values().map(|c| (c.pieza.objeto.clone(), c.cantidad)).collect::<Vec<_>>(), objetos);
    assert_eq!(lo_dado(&x.reclamadas()[0]), (vec![], vec![]));
}

#[test]
fn el_hecho_cuenta_lo_que_se_dio_y_nada_de_lo_que_no() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let a = x.plaza(m, "ana", 0);
    let (h, _) = x.saliendo("ana", Some(1));
    // Cadena de héroe con de todo: oro, un recurso, xp y una tropa; sin participación (no es de plaza).
    x.ok("ana", reclamar(h, Dueno::Heroe(h), "HCOMPLETA"));
    let id = x.escuadra(h, Tropa::Honderos).id;
    assert_eq!(
        lo_dado(&x.reclamadas()[0]),
        (
            vec![("xp", 2 * UNIDAD), ("oro", 4 * UNIDAD)],
            vec![("recursos", lista(vec![lista(vec![texto("madera"), Dato::Numero(3 * UNIDAD)])])), ("escuadras", lista(vec![lista(vec![texto("honderos"), Dato::Numero(2), texto(&ids::escuadra(id))])]))]
        )
    );
    // Cadena de plaza: participación (a quien reclama) y un recurso, sin xp, oro ni tropa.
    x.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: a, nivel: 2 });
    let g = x.gobernador(a);
    x.ok("ana", reclamar(g, Dueno::Asentamiento(a), "AP"));
    assert_eq!(lo_dado(&x.reclamadas()[1]), (vec![("participacion", 10)], vec![("recursos", lista(vec![lista(vec![texto("madera"), Dato::Numero(3 * UNIDAD)])]))]));
    // Cadena de Facción: solo la xp.
    x.ok("admin", reclamar(0, Dueno::Faccion(1), "FA"));
    assert_eq!(lo_dado(&x.reclamadas()[2]), (vec![("xp", 5 * UNIDAD)], vec![]));
    assert_eq!(x.reclamadas().iter().map(|e| e.valores[0].clone()).collect::<Vec<_>>(), [("mision", texto("HCOMPLETA")), ("mision", texto("AP")), ("mision", texto("FA"))]);
}
