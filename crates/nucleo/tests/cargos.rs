//! Los cargos locales, la matriz de autorización de las órdenes de plaza y la puerta (sistema 5, tanda 5E; Diplomacia_Facciones §3.4 y §4.3, Proyecciones §4.3 y §5.2, Presencia §8): quién designa
//! cada cargo y a quién (Q4: hay que residir), cuándo se vacía, que el cargo exige residir y estar dentro («la ciudadanía habilita, la presencia ejerce»), la puerta por grupos con su defecto
//! (Q1: cerrada a neutrales y enemigos), los vetos que expulsan y la foto `BAN36`. Aquí las órdenes las da cada cuenta tal cual, sin el andamiaje de los demás escenarios (`comun::mandaba`).
//! Casos portados de `typescript-final:src/session/__tests__/{comandosCargos,autorizacionComandos,comandosPresencia}.test.ts` y `engine/__tests__/{cargos,puerta}.test.ts`.

mod comun;

use asentamiento::catalogo::Tipo;
use asentamiento::{Cargo, Recurso};
use comun::real::{fundar, sitio_rico, M};
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::{rechazos, Ajustes, Arranque, Comando, Estado, Nucleo, Peticion, Reglas, Relacion, Respuesta, Resultado};
use politica::{Grupo, Grupos};

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

fn arranque(m: &M) -> Arranque {
    let mut reglas = Reglas::default();
    let b = &mut std::sync::Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    Arranque { reglas, ajustes: Ajustes::default(), semilla: 5, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn codigo(r: &Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

/// La plaza de F1 que fundó «rey» (su héroe de campo, Rey de F1, reside en ella y está fuera, en su columna). Dentro, residentes y sin cargo, «ana», «bea» y «eli». «cai» es de F1 y reside en
/// el campamento 0; «dan», de F2, sin relación con F1.
struct E {
    n: Nucleo,
    plaza: u64,
    rey: u64,
    ana: u64,
    bea: u64,
    eli: u64,
    cai: u64,
    dan: u64,
    cuenta: u64,
}

impl E {
    fn nuevo(m: &M) -> Self {
        let mut n = Nucleo::nuevo(arranque(m));
        let p = sitio_rico(&n, &m.0, 0, 1);
        let plaza = fundar(&mut n, "rey", 1, p);
        let rey = n.faccion(1).and_then(|f| f.rey).expect("F1 tiene Rey");
        let mut e = E { n, plaza, rey, ana: 0, bea: 0, eli: 0, cai: 0, dan: 0, cuenta: 0 };
        e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
        for r in [Recurso::Madera, Recurso::Piedra, Recurso::Oro, Recurso::Trigo] {
            e.ok("admin", Comando::DarRecursos { asentamiento: plaza, recurso: r, cantidad: 300_000 });
        }
        e.ana = e.residente_dentro("ana");
        e.bea = e.residente_dentro("bea");
        e.eli = e.residente_dentro("eli");
        e.cai = e.heroe("cai", Some(1));
        e.dan = e.heroe("dan", Some(2));
        e
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
    fn no(&mut self, actor: &str, c: Comando) -> &'static str {
        let r = self.ord(actor, c);
        codigo(&r)
    }
    fn heroe(&mut self, cuenta: &str, faccion: Option<u32>) -> u64 {
        let h = self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano }).creadas[0];
        if faccion.is_some() {
            self.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion });
        }
        h
    }
    fn residente_dentro(&mut self, cuenta: &str) -> u64 {
        let h = self.heroe(cuenta, Some(1));
        let plaza = self.plaza as u32;
        self.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(plaza)) });
        self.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(plaza) });
        h
    }
    fn titular(&self, c: Cargo) -> Option<u64> {
        self.n.estado().asentamientos[&self.plaza].cargos.titular(c)
    }
    fn asignar(&mut self, actor: &str, heroe: u64, cargo: Cargo, designado: u64) -> Respuesta {
        let plaza = self.plaza;
        self.ord(actor, Comando::AsignarCargoLocal { heroe, asentamiento: plaza, cargo, designado })
    }
    fn hechos(&self, codigo: &str) -> Vec<nucleo::EventoNucleo> {
        self.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).cloned().collect()
    }
    fn ubicacion(&self, h: u64) -> Ubicacion {
        self.n.heroe(h).unwrap().heroe.presencia.ubicacion
    }
    /// El residente sale de su plaza al mundo, sin tropa ni carga, y vuelve a entrar con la columna con la que salió.
    fn salir(&mut self, cuenta: &str, h: u64) {
        let plaza = self.plaza;
        self.ok(cuenta, Comando::SalirAlMundo { asentamiento: plaza, heroe: h, escuadras: vec![], carga: vec![] });
    }
    fn entrar(&mut self, cuenta: &str, h: u64) {
        let plaza = self.plaza;
        self.ok(cuenta, Comando::EntrarEnAsentamiento { asentamiento: plaza, heroe: h });
    }
}

#[test]
fn al_gobernador_lo_nombra_el_rey_sin_estar_presente_y_a_los_demas_el_gobernador_dentro_y_solo_a_quien_reside() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, ana, bea, eli, cai, dan) = (e.rey, e.ana, e.bea, e.eli, e.cai, e.dan);
    // 5E-3 (decisión del autor, 2026-10-25): quien funda (el Líder de la columna, aquí el Rey) es su Gobernador de oficio, con su hecho.
    assert_eq!(e.titular(Cargo::Gobernador), Some(rey), "quien funda es su Gobernador de oficio");
    let h = e.hechos("cargo.local_asignado");
    assert_eq!((h.len(), h[0].dato("heroe"), h[0].asentamientos.clone()), (1, Some(rey as i64), vec![e.plaza]));
    assert!(matches!(e.ubicacion(rey), Ubicacion::Columna(_)), "el Rey está fuera, en su columna");
    // Deja de residir en ella: el cargo se vacía y la plaza se queda sin Gobernador.
    e.ok("admin", Comando::FijarResidencia { heroe: rey, residencia: Some(Residencia::Campamento(0)) });
    assert!(Cargo::Gobernador as usize == 0 && e.titular(Cargo::Gobernador).is_none(), "sin residir no hay cargo");

    // Quién designa: el Gobernador, solo el Rey (sin estar presente); los demás, solo el Gobernador.
    assert_eq!(codigo(&e.asignar("ana", ana, Cargo::Gobernador, ana)), rechazos::NO_AUTORIZADO, "una residente sin cargo no se nombra Gobernadora");
    assert_eq!(codigo(&e.asignar("rey", rey, Cargo::Tesorero, ana)), rechazos::NO_AUTORIZADO, "el Rey no designa a los otros cuatro");
    assert_eq!(codigo(&e.asignar("ana", rey, Cargo::Gobernador, ana)), rechazos::NO_AUTORIZADO, "nadie actúa a nombre de otro");
    // A quién: ciudadano de la Facción dueña que reside en la plaza (Q4).
    assert_eq!(codigo(&e.asignar("rey", rey, Cargo::Gobernador, cai)), rechazos::CARGO_NO_RESIDE, "cai es de F1 pero reside en un campamento");
    assert_eq!(codigo(&e.asignar("rey", rey, Cargo::Gobernador, dan)), rechazos::CARGO_NO_CIUDADANO);
    assert_eq!(codigo(&e.asignar("rey", rey, Cargo::Gobernador, 9_999)), rechazos::HEROE_DESCONOCIDO);
    assert_eq!(codigo(&e.asignar("admin", rey, Cargo::Tesorero, bea)), rechazos::CARGO_SIN_GOBERNADOR, "sin Gobernador no hay Tesorero, tampoco por la administración");
    assert!(e.titular(Cargo::Gobernador).is_none(), "un rechazo no deja rastro");

    let v = e.n.version();
    assert!(e.asignar("rey", rey, Cargo::Gobernador, ana).aceptada());
    assert_eq!((e.titular(Cargo::Gobernador), e.n.version()), (Some(ana), v + 1));
    assert_eq!(e.asignar("rey", rey, Cargo::Gobernador, ana).resultado, Resultado::SinCambios);
    let h = e.hechos("cargo.local_asignado");
    assert_eq!((h.len(), h[1].dato("heroe"), h[1].dato("asentamiento"), h[1].asentamientos.clone()), (2, Some(ana as i64), Some(e.plaza as i64), vec![e.plaza]));
    assert!(h[1].valores.iter().any(|(k, v)| *k == "cargo" && *v == nucleo::Dato::Texto("gobernador".into())));

    // La Gobernadora, dentro, designa al resto; uno puede tener varios cargos.
    assert!(e.asignar("ana", ana, Cargo::Tesorero, bea).aceptada());
    assert!(e.asignar("ana", ana, Cargo::Sacerdote, ana).aceptada());
    assert_eq!(codigo(&e.asignar("bea", bea, Cargo::MaestroObras, eli)), rechazos::NO_AUTORIZADO, "el Tesorero no designa");
    // Fuera de la plaza no gobierna: la presencia ejerce.
    e.salir("ana", ana);
    assert_eq!(codigo(&e.asignar("ana", ana, Cargo::MaestroObras, eli)), rechazos::NO_AUTORIZADO, "una Gobernadora de campaña no designa desde el camino");
    assert_eq!(e.titular(Cargo::Gobernador), Some(ana), "pero sigue siéndolo");
    e.entrar("ana", ana);
    assert!(e.asignar("ana", ana, Cargo::MaestroObras, eli).aceptada());
    // El Rey nombra a otra Gobernadora: sustituye a la anterior, que se queda con lo demás.
    assert!(e.asignar("rey", rey, Cargo::Gobernador, bea).aceptada());
    assert_eq!((e.titular(Cargo::Gobernador), e.titular(Cargo::Sacerdote)), (Some(bea), Some(ana)));
    // Desde fuera, la ficha de la plaza propia (`FACCION`) lleva sus cargos; el interior (`ASENTAMIENTO`), también.
    use nucleo::proyeccion::{ids, interior, propios};
    let ficha = propios::proyectar(&e.n, propios::Espectador::Heroe(Some(rey)), nucleo::Clase::Asentamiento, e.plaza).existen.remove(0).2;
    let contrato::v1::entidad::Cuerpo::Asentamiento(ficha) = ficha else { panic!("la ficha de la plaza") };
    let cargos = ficha.cargos.expect("la ficha lleva los cargos");
    assert_eq!((cargos.gobernador_id, cargos.tesorero_id, cargos.maestro_obras_id, cargos.sacerdote_id, cargos.general_id), (Some(ids::heroe(bea)), Some(ids::heroe(bea)), Some(ids::heroe(eli)), Some(ids::heroe(ana)), None));
    assert_eq!(interior::asentamiento(&e.n, e.plaza).and_then(|p| p.cargos), Some(contrato::v1::Cargos { gobernador_id: Some(ids::heroe(bea)), tesorero_id: Some(ids::heroe(bea)), maestro_obras_id: Some(ids::heroe(eli)), sacerdote_id: Some(ids::heroe(ana)), general_id: None }));
}

#[test]
fn el_cargo_se_vacia_al_dejar_de_residir_o_de_ser_de_la_faccion() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, ana, bea, eli) = (e.rey, e.ana, e.bea, e.eli);
    assert!(e.asignar("rey", rey, Cargo::Gobernador, ana).aceptada());
    assert!(e.asignar("ana", ana, Cargo::Tesorero, bea).aceptada());
    assert!(e.asignar("ana", ana, Cargo::MaestroObras, eli).aceptada());
    // Dejar la residencia vacía sus cargos (Q4)...
    e.ok("bea", Comando::DejarResidencia { heroe: bea });
    assert_eq!(e.titular(Cargo::Tesorero), None);
    // ... también la reubicación de la administración ...
    e.ok("admin", Comando::FijarResidencia { heroe: eli, residencia: Some(Residencia::Campamento(0)) });
    assert_eq!(e.titular(Cargo::MaestroObras), None);
    // ... y dejar de ser de la Facción dueña, aunque la administración no la mude.
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: ana, faccion: None });
    assert_eq!((e.titular(Cargo::Gobernador), e.n.heroe(ana).unwrap().heroe.residencia), (None, Some(Residencia::Asentamiento(e.plaza as u32))));
}

#[test]
fn la_matriz_pide_el_titular_del_cargo_que_resida_y_este_dentro() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, ana, bea, eli, plaza) = (e.rey, e.ana, e.bea, e.eli, e.plaza);
    assert!(e.asignar("rey", rey, Cargo::Gobernador, ana).aceptada());
    assert!(e.asignar("ana", ana, Cargo::Tesorero, bea).aceptada());
    assert!(e.asignar("ana", ana, Cargo::MaestroObras, eli).aceptada());
    let obra = Comando::AnadirEdificio { asentamiento: plaza, tipo: Tipo::Vivienda };

    // Las obras: el Gobernador o el Maestro de Obras, dentro.
    assert_eq!(e.no("bea", obra.clone()), rechazos::NO_AUTORIZADO, "el Tesorero no construye");
    assert_eq!(e.no("rey", obra.clone()), rechazos::NO_AUTORIZADO, "el Rey sin cargo y fuera, tampoco");
    assert_eq!(e.no("dan", obra.clone()), rechazos::NO_AUTORIZADO);
    e.ok("eli", obra.clone());
    e.ok("ana", obra.clone());
    e.ok("ana", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: false });
    assert_eq!(e.no("bea", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true }), rechazos::NO_AUTORIZADO);
    // La reserva, solo el Tesorero; reabastecer a los aliados, el Gobernador o el Tesorero; el ascenso, solo el Gobernador.
    assert_eq!(e.no("ana", Comando::CalibrarReserva { asentamiento: plaza, recurso: Recurso::Trigo, valor: 50_000 }), rechazos::NO_AUTORIZADO);
    e.ok("bea", Comando::CalibrarReserva { asentamiento: plaza, recurso: Recurso::Trigo, valor: 50_000 });
    e.ok("bea", Comando::AlternarReabastecerAliados { asentamiento: plaza, permitido: true });
    assert_eq!(e.no("eli", Comando::SolicitarAscenso { asentamiento: plaza }), rechazos::NO_AUTORIZADO);
    assert_ne!(e.no("ana", Comando::SolicitarAscenso { asentamiento: plaza }), rechazos::NO_AUTORIZADO, "la Gobernadora lo pide (y lo rechaza la economía)");
    // El cargo que nombra la orden: el de quien la da; que sirva para la acción lo mira la orden.
    assert_eq!(e.no("ana", Comando::ActivarPolitica { asentamiento: plaza, politica: "racionamiento".into(), cargo: Cargo::Sacerdote }), rechazos::NO_AUTORIZADO, "no es la Sacerdotisa");
    assert_eq!(e.no("bea", Comando::ComprometerRecinto { asentamiento: plaza, cargo: Cargo::Tesorero, nivel: 1 }), rechazos::MURALLAS_SIN_CARGO, "es el Tesorero, pero el Tesorero no encarga murallas");
    // Una caravana: la crea y la manda quien reside y está dentro, tenga cargo o no.
    assert_ne!(e.no("bea", Comando::CrearCaravana { asentamiento: plaza }), rechazos::NO_AUTORIZADO, "residente dentro: lo que falte es el Mercado");
    assert_eq!(e.no("cai", Comando::CrearCaravana { asentamiento: plaza }), rechazos::NO_AUTORIZADO, "de la Facción pero sin residir");
    // Fuera de la plaza, el Gobernador no gobierna; la administración puede con todo.
    e.salir("ana", ana);
    assert_eq!(e.no("ana", obra.clone()), rechazos::NO_AUTORIZADO);
    e.ok("admin", obra);
}

#[test]
fn la_puerta_por_defecto_cierra_a_neutrales_y_enemigos_la_fija_el_gobernador_o_el_rey_y_solo_el_gobernador_veta() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, ana, bea, cai, dan, plaza) = (e.rey, e.ana, e.bea, e.cai, e.dan, e.plaza);
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 3, relacion: Some(Relacion::Alianza) });
    e.ok("admin", Comando::FijarRelacion { a: 1, b: 4, relacion: Some(Relacion::Guerra) });
    let (ali, ene, nadie) = (e.heroe("ali", Some(3)), e.heroe("ene", Some(4)), e.heroe("nadie", None));
    let puede = |e: &E| [cai, ali, dan, nadie, ene].map(|h| e.n.puede_entrar_en_plaza(h, plaza));

    // Por defecto (Q1): entran la propia Facción y los aliados; los neutrales (también sin Facción) y los enemigos no. Los Aedas, sí.
    assert_eq!(e.n.puerta_cerrada_a(plaza), Some(Grupos::de([Grupo::Neutrales, Grupo::Enemigos])));
    assert_eq!(puede(&e), [true, true, false, false, false]);
    assert!(e.n.estado().asentamientos[&plaza].puerta.admite_aedas(&e.n.estado().reglas.economia.politica));

    // La fija el Rey sin estar presente, o la Gobernadora dentro; una residente sin cargo, no. Sustituye el conjunto entero.
    let fijar = |heroe: u64, grupos: &[Grupo]| Comando::FijarPuerta { heroe, asentamiento: plaza, cerrada_a: Grupos::de(grupos.iter().copied()) };
    assert!(e.asignar("rey", rey, Cargo::Gobernador, ana).aceptada());
    assert_eq!(e.no("bea", fijar(bea, &[])), rechazos::NO_AUTORIZADO);
    assert_eq!(e.no("dan", fijar(dan, &[])), rechazos::NO_AUTORIZADO);
    assert_eq!(e.ord("rey", fijar(rey, &[Grupo::Neutrales, Grupo::Enemigos])).resultado, Resultado::SinCambios, "es la de por defecto");
    e.ok("rey", fijar(rey, &[]));
    assert_eq!(puede(&e), [true; 5], "abierta a todos");
    // Con un neutral dentro, la Gobernadora cierra a los neutrales y a los Aedas: el neutral sale al mapa con lo suyo; el aliado y la propia Facción se quedan.
    for h in [dan, ali, cai] {
        e.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
    }
    e.ok("ana", fijar(ana, &[Grupo::Neutrales, Grupo::Aedas]));
    assert_eq!(puede(&e), [true, true, false, false, true]);
    assert!(!e.n.estado().asentamientos[&plaza].puerta.admite_aedas(&e.n.estado().reglas.economia.politica));
    assert!(matches!(e.ubicacion(dan), Ubicacion::Columna(_)), "el neutral sale");
    assert_eq!((e.ubicacion(ali), e.ubicacion(cai)), (Ubicacion::Asentamiento(plaza as u32), Ubicacion::Asentamiento(plaza as u32)));
    let fijadas = e.hechos("asentamiento.puerta_fijada");
    assert_eq!(fijadas.len(), 2);
    assert!(fijadas[1].valores.iter().any(|(k, v)| *k == "cerrada_a" && *v == nucleo::Dato::Lista(vec![nucleo::Dato::Texto("neutrales".into()), nucleo::Dato::Texto("aedas".into())])));

    // Vetar: solo la Gobernadora (el Rey no); a un residente no; el veto alcanza a la propia Facción y saca a quien está dentro; levantarlo le deja volver.
    let vetar = |heroe: u64, vetado: u64, si: bool| Comando::VetarJugador { heroe, asentamiento: plaza, vetado, vetar: si };
    assert_eq!(e.no("rey", vetar(rey, cai, true)), rechazos::NO_AUTORIZADO, "el Rey fija la puerta pero no veta");
    assert_eq!(e.no("ana", vetar(ana, bea, true)), rechazos::PUERTA_RESIDENTE);
    assert_eq!(e.no("ana", vetar(ana, 9_999, true)), rechazos::HEROE_DESCONOCIDO);
    e.ok("ana", vetar(ana, cai, true));
    assert_eq!(e.ord("ana", vetar(ana, cai, true)).resultado, Resultado::SinCambios);
    assert!(!e.n.puede_entrar_en_plaza(cai, plaza) && matches!(e.ubicacion(cai), Ubicacion::Columna(_)), "vetado: sale, aunque sea de la Facción");
    assert_eq!(e.n.estado().asentamientos[&plaza].puerta.vetados.iter().copied().collect::<Vec<_>>(), vec![cai]);
    let vetos = e.hechos("asentamiento.veto");
    assert_eq!((vetos.len(), vetos[0].dato("heroe"), vetos[0].dato("vetado")), (1, Some(cai as i64), Some(1)));
    e.ok("ana", vetar(ana, cai, false));
    assert!(e.n.puede_entrar_en_plaza(cai, plaza));
    // El alias `FijarAcceso` (117) se retiró en 5H (5E-12): ahora la puerta solo se fija por grupos (lo prueba `comando.rs`).
}

#[test]
fn un_vetado_no_repone_en_la_plaza_ni_se_muda_a_ella() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, ana, cai, plaza) = (e.rey, e.ana, e.cai, e.plaza);
    assert!(e.asignar("rey", rey, Cargo::Gobernador, ana).aceptada());
    e.ok("admin", Comando::ColocarHeroe { heroe: cai, ubicacion: Ubicacion::Asentamiento(plaza as u32) });
    let antes = e.no("cai", Comando::ReclutarTropa { asentamiento: plaza, heroe: cai, tropa: militar::catalogo::Tropa::MiliciaLanceros });
    e.ok("ana", Comando::VetarJugador { heroe: ana, asentamiento: plaza, vetado: cai, vetar: true });
    let despues = e.no("cai", Comando::ReclutarTropa { asentamiento: plaza, heroe: cai, tropa: militar::catalogo::Tropa::MiliciaLanceros });
    assert_eq!((antes, despues), (rechazos::RECLUTAMIENTO_SOLO_REPONER, rechazos::RECLUTAMIENTO_NO_PUEDES), "el veto cambia el permiso: de solo reponer a no poder");
    // Ni se muda a ella.
    assert_eq!(e.no("cai", Comando::CambiarResidencia { heroe: cai, asentamiento: plaza }), rechazos::RESIDENCIA_VETADO);
}

#[test]
fn los_cargos_y_la_puerta_viajan_en_la_foto_ban33_y_la_ban32_se_rechaza() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (rey, ana, bea, cai, plaza) = (e.rey, e.ana, e.bea, e.cai, e.plaza);
    let hash0 = e.n.hash_estado();
    assert!(e.asignar("rey", rey, Cargo::Gobernador, ana).aceptada());
    let hash1 = e.n.hash_estado();
    assert!(e.asignar("ana", ana, Cargo::Tesorero, bea).aceptada());
    e.ok("ana", Comando::FijarPuerta { heroe: ana, asentamiento: plaza, cerrada_a: Grupos::de([Grupo::Enemigos]) });
    let hash2 = e.n.hash_estado();
    e.ok("ana", Comando::VetarJugador { heroe: ana, asentamiento: plaza, vetado: cai, vetar: true });
    assert!(hash0 != hash1 && hash1 != hash2 && hash2 != e.n.hash_estado(), "cada pieza mueve el hash");
    let bytes = e.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN36");
    let reglas = e.n.estado().reglas.clone();
    let vuelta = Estado::deserializar(&bytes, &reglas, &e.n.estado().mundo_hash).unwrap();
    let a = &vuelta.asentamientos[&plaza];
    assert_eq!((a.cargos.titular(Cargo::Gobernador), a.cargos.titular(Cargo::Tesorero), a.puerta.cerrada_a, a.puerta.vetados.len()), (Some(ana), Some(bea), Some(Grupos::de([Grupo::Enemigos])), 1));
    assert_eq!((vuelta.serializar(), vuelta.hash()), (bytes.clone(), e.n.hash_estado()));
    let vieja = [b"BAN33".as_slice(), &bytes[5..]].concat();
    let error = Estado::deserializar(&vieja, &reglas, &e.n.estado().mundo_hash).err().expect("BAN33 se rechaza");
    assert!(error.0.contains("BAN33") && error.0.contains("cargos"), "{}", error.0);
}
