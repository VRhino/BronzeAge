//! La participación (mecánica 54, `Docs/Mechanics/Participacion.md`): quién suma en cada plaza al defenderla y al ganar una batalla, lo que se ve de ella en la plaza y en la Facción
//! y que viaja en la foto. Con números, sobre el artefacto de mundo real (las pruebas se omiten si no está); el resultado de Unity lo mira `tests/batalla_unity.rs`.

mod comun;
use base::{Punto, MS_POR_MINUTO};
use comun::real::{self, campamento, pisable, sitio_rico, M};
use militar::catalogo::Tropa;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::proyeccion::{ids, interior, propios};
use nucleo::{Ajustes, Arranque, Comando, Estado, Nucleo, ObjetivoAtaque, Peticion, Reglas, Respuesta};
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

/// Las reglas de siempre, sin bandidos que acosen o aparezcan solos.
fn reglas() -> Reglas {
    let mut r = Reglas::default();
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
    fn avanzar(&mut self, ms: i64) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    /// Una columna de administración de la cuenta en `p` con esas escuadras completas: (columna, héroe).
    fn columna(&mut self, cuenta: &str, faccion: u32, p: Punto, tropas: &[Tropa]) -> (u64, u64) {
        let t = self.n.ahora();
        let col = self.n.crear_ejercito(t, cuenta, faccion, p, 500, 350_000).unwrap();
        let h = self.n.ejercito(col).unwrap().columna.lider;
        for &tropa in tropas {
            self.ok("admin", Comando::DarEscuadra { heroe: h, tropa, cantidad: unidades(tropa) });
        }
        (col, h)
    }
    /// El héroe pasa a ser ciudadano de la Facción de la plaza y a residir en ella (sin moverse de donde está).
    fn residir(&mut self, h: u64, plaza: u64) {
        let f = self.n.estado().asentamientos[&plaza].faccion;
        let _ = self.ord("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(f) });
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
    }
    fn posicion(&self, plaza: u64) -> Punto {
        self.n.estado().asentamientos[&plaza].posicion
    }
    fn de(&self, plaza: u64) -> BTreeMap<u64, u64> {
        self.n.participacion_en(plaza).cloned().unwrap_or_default()
    }
}

fn unidades(t: Tropa) -> i64 {
    militar::Reglas::cargar().tropas.tropa(t).unidades
}

/// Dos plazas de Facciones distintas, con la construcción parada: la de «ana» (F1) junto al campamento 0 y la de «beto» (F2) junto al 3. A las 25 horas, sin protección.
fn dos_plazas(x: &mut X, m: &M) -> (u64, u64) {
    let pa = sitio_rico(&x.n, &m.0, 0, 1);
    let a = real::fundar(&mut x.n, "ana", 1, pa);
    let pb = sitio_rico(&x.n, &m.0, 3, 2);
    let b = real::fundar(&mut x.n, "beto", 2, pb);
    for p in [a, b] {
        x.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: p, pausada: true });
        // De todo para días: sin auto-construcción la plaza no se sostiene sola.
        for r in asentamiento::recurso::TODOS {
            let _ = x.ord("admin", Comando::DarRecursos { asentamiento: p, recurso: r, cantidad: 20_000_000 });
        }
    }
    x.avanzar(25 * HORA);
    (a, b)
}

/// Un defensor de «beto» que reside en B y está dentro con esas escuadras en su loadout activo.
fn defensor(x: &mut X, b: u64, defensa: &[Tropa]) -> u64 {
    let h = x.ok("beto", Comando::CrearHeroe { cuenta: "beto".into(), campamento: 3, controlador: Controlador::Humano, nombre: None }).creadas[0];
    x.residir(h, b);
    x.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(b as u32) });
    let escuadras: Vec<u64> = defensa.iter().map(|&t| x.ok("admin", Comando::DarEscuadra { heroe: h, tropa: t, cantidad: unidades(t) }).creadas[0]).collect();
    let loadout = x.n.heroe(h).unwrap().heroe.loadouts[0].id;
    x.ok("admin", Comando::GuardarLoadout { heroe: h, loadout: Some(loadout), nombre: "Default".into(), escuadras, perks: vec![], activo: Some(true) });
    h
}

/// «ana» asedia B con números desde 2 u y se resuelve al acabar la convocatoria. Devuelve su héroe.
fn asediar(x: &mut X, b: u64, ataque: &[Tropa], reside_en: Option<u64>) -> u64 {
    let pb = x.posicion(b);
    let (_, ana) = x.columna("ana", 1, Punto::new(pb.x + 2_000, pb.y), ataque);
    if let Some(a) = reside_en {
        x.residir(ana, a);
    }
    x.ok("ana", Comando::Atacar { heroe: ana, objetivo: ObjetivoAtaque::Asentamiento(b), declara_guerra: true });
    if let Some(fin) = x.n.batallas_activas().filter_map(|b| b.convocatoria_hasta).max() {
        x.n.avanzar_hasta(fin);
    }
    ana
}

#[test]
fn quien_defiende_una_plaza_que_resiste_suma_la_defensa_y_la_batalla_ganada_y_se_ve_en_la_plaza_en_su_faccion_y_en_la_foto() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (a, b) = dos_plazas(&mut x, m);
    let beto = defensor(&mut x, b, &[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto]);
    // Ana reside en A pero pierde: no suma nada.
    let ana = asediar(&mut x, b, &[Tropa::MiliciaLanceros], Some(a));
    assert_eq!(x.n.estado().asentamientos[&b].faccion, 2, "la plaza resiste");
    assert_eq!(x.de(b), BTreeMap::from([(beto, 10)]), "defensa (5) y batalla ganada (5), las dos en B, donde reside");
    assert!(x.de(a).is_empty() && !x.n.estado().participacion.values().any(|p| p.contains_key(&ana)));
    // De paso cuentan `defensas` y `victorias` para el tablón de su Facción (Misiones_Recurrentes §7): FS1 (victorias) y FS2 (defensas), grupales.
    let aportes = |x: &X, id: &str| match &x.n.tablon_de(nucleo::misiones::Dueno::Faccion(2)).unwrap().misiones.iter().find(|m| m.def.id == id).unwrap().progreso {
        nucleo::recurrentes::Progreso::Grupal { aportes, .. } => aportes.clone(),
        otro => panic!("{otro:?}"),
    };
    assert_eq!((aportes(&x, "FS1"), aportes(&x, "FS2")), (BTreeMap::from([(beto, 1)]), BTreeMap::from([(beto, 1)])));

    // En el interior de la plaza, por id de héroe; en la Facción, cada ciudadano con su residencia, sus cargos y la suma.
    let p = interior::asentamiento(&x.n, b).unwrap();
    assert_eq!(p.participacion, BTreeMap::from([(ids::heroe(beto), 10)]));
    let f = propios::faccion(&x.n, 2);
    let mb = f.miembros.iter().find(|q| q.heroe_id == ids::heroe(beto)).expect("beto es miembro");
    assert_eq!((mb.residencia_id.as_deref(), mb.cargos.is_empty(), mb.participacion), (Some(ids::asentamiento(b).as_str()), true, 10));
    let rey = f.miembros.iter().find(|q| q.heroe_id != ids::heroe(beto)).expect("el fundador");
    assert_eq!((rey.cargos.first().map(String::as_str), rey.participacion), (Some("rey"), 0), "{rey:?}");
    assert!(propios::faccion_publica(&x.n, 2).miembros.is_empty(), "la forma pública no lleva miembros");

    // La foto la guarda.
    let bytes = x.n.estado().serializar();
    let e = Estado::deserializar(&bytes, &x.n.estado().reglas, &x.n.estado().mundo_hash).unwrap();
    assert_eq!(e.participacion, x.n.estado().participacion);
}

#[test]
fn la_conquista_con_combate_da_la_defensa_a_quien_cae_y_la_batalla_ganada_al_atacante_en_su_plaza_y_la_participacion_se_queda_con_la_plaza() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (a, b) = dos_plazas(&mut x, m);
    let beto = defensor(&mut x, b, &[Tropa::MiliciaLanceros]);
    let ana = asediar(&mut x, b, &[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto, Tropa::EspadachinesBronce], Some(a));
    assert_eq!(x.n.estado().asentamientos[&b].faccion, 1, "la plaza cae");
    assert_eq!(x.de(b), BTreeMap::from([(beto, 5)]), "la defensa se queda con la plaza, que ahora es de F1");
    assert_eq!(x.de(a), BTreeMap::from([(ana, 5)]), "la batalla ganada, en la plaza donde reside");
    let suma = |f: u32, h: u64| x.n.miembros_de(f).into_iter().find(|q| q.heroe == h).map(|q| q.participacion);
    assert_eq!((suma(1, ana), suma(2, beto)), (Some(5), Some(0)), "B ya no es de F2: no le suma");
}

#[test]
fn una_plaza_que_cae_sin_combate_no_da_nada() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (a, b) = dos_plazas(&mut x, m);
    asediar(&mut x, b, &[Tropa::EspadachinesBronce], Some(a));
    assert_eq!(x.n.estado().asentamientos[&b].faccion, 1, "la plaza cae");
    assert!(x.n.estado().participacion.is_empty(), "{:?}", x.n.estado().participacion);
}

#[test]
fn destruir_un_campamento_de_bandidos_suma_la_batalla_ganada_solo_a_quien_reside_en_una_plaza() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (a, _) = dos_plazas(&mut x, m);
    let destruir = |x: &mut X, k: i64, reside: bool| {
        let donde = pisable(&m.0, campamento(&m.0, 0), 40_000 + k * 8_000, 30_000);
        let (_, h) = x.columna("ana", 1, donde, &[Tropa::ArquerosCompuesto]);
        if reside {
            x.residir(h, a);
        }
        let b = x.ok("admin", Comando::ColocarBandidos { posicion: pisable(&m.0, donde, 1_000, 0), nivel: 1 }).creadas[0];
        x.ok("ana", Comando::Atacar { heroe: h, objetivo: ObjetivoAtaque::Campamento(b), declara_guerra: false });
        assert!(x.n.campamento_de_bandidos(b).is_none(), "el campamento cae");
        h
    };
    let sin_plaza = destruir(&mut x, 0, false);
    assert!(x.n.estado().participacion.is_empty(), "sin residencia en una plaza no suma");
    let h = destruir(&mut x, 1, true);
    assert_eq!(x.de(a), BTreeMap::from([(h, 5)]));
    assert_ne!(h, sin_plaza);
}

#[test]
fn la_columna_que_gana_un_combate_suma_la_batalla_ganada_y_la_que_pierde_nada() {
    let m = con_mundo!();
    let mut x = X::nuevo(m);
    let (a, b) = dos_plazas(&mut x, m);
    let donde = pisable(&m.0, campamento(&m.0, 0), 40_000, 30_000);
    let (_, ana) = x.columna("ana", 1, donde, &[Tropa::EspadachinesBronce, Tropa::ArquerosCompuesto]);
    let (beto_col, beto) = x.columna("beto", 2, Punto::new(donde.x + 2_000, donde.y), &[Tropa::MiliciaLanceros]);
    x.residir(ana, a);
    x.residir(beto, b);
    x.ok("ana", Comando::Atacar { heroe: ana, objetivo: ObjetivoAtaque::Ejercito(beto_col), declara_guerra: true });
    assert_eq!(x.de(a), BTreeMap::from([(ana, 5)]));
    assert!(x.de(b).is_empty(), "una columna no defiende ninguna plaza");
}
