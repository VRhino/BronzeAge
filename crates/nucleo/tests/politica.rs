//! La Facción como entidad, la ciudadanía y la residencia en el núcleo (sistema 5, tanda 5B; Diplomacia_Facciones §3-§4): crear una Facción, pedir el ingreso y responderlo, el Rey y el
//! Embajador, dejarla con la sucesión del trono (y Q7), las tres órdenes de residencia con su cooldown, los fundadores que pasan a residir, los ciudadanos de la nobleza (Q8), la proyección de
//! la Facción y la foto `BAN28` (hoy `BAN42`). De 5C: la experiencia y el nivel derivado, la capital (`designar_capital`, la más antigua, la distancia en el mantenimiento y la Gran Fundición única),
//! el nombre de las plazas (`nombreDeCiudadLibre`, `renombrar_asentamiento`) y la foto que rechaza `BAN29`. Casos portados de `typescript-final:src/session/__tests__/{comandosMembresiaFaccion,comandosResidencia,faccionDeLaColumna}.test.ts` y
//! `engine/__tests__/residenciaMercenaria.test.ts`. Sobre el artefacto de mundo real; las pruebas se omiten si no está.

mod comun;

use base::{Instante, MS_POR_MINUTO};
use comun::real::{campamento, fundar, sitio_rico, M};
use militar::columna::TipoColumna;
use militar::heroe::{Controlador, Residencia};
use militar::presencia::Ubicacion;
use nucleo::proyeccion::{ids, propios};
use nucleo::{rechazos, Ajustes, Arranque, Comando, Estado, Nucleo, Peticion, Reglas, Respuesta, Resultado};

const DIA: Instante = 24 * 60 * MS_POR_MINUTO;

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

/// Sin acoso ni anillo de bandidos: estas pruebas miden otra cosa.
fn arranque(m: &M, semilla: u64) -> Arranque {
    let mut reglas = Reglas::default();
    let b = &mut std::sync::Arc::make_mut(&mut reglas.economia.combate).bandidos;
    (b.radio_ataque_caravana, b.anillo_minimo, b.anillo_maximo) = (0, 0, 0);
    Arranque { reglas, ajustes: Ajustes::default(), semilla, inicio: 0, rutas: m.1.clone(), mundo_hash: m.0.hash().into(), mundo: Some(m.0.clone()) }
}

fn codigo(r: &Respuesta) -> &'static str {
    match &r.resultado {
        Resultado::Rechazada { codigo, .. } => codigo,
        otro => panic!("se esperaba un rechazo: {otro:?}"),
    }
}

struct E {
    n: Nucleo,
    cuenta: u64,
}

impl E {
    fn nuevo(m: &M) -> Self {
        E { n: Nucleo::nuevo(arranque(m, 5)), cuenta: 0 }
    }
    fn ord(&mut self, actor: &str, c: Comando) -> Respuesta {
        self.cuenta += 1;
        let p = Peticion::nueva(actor, &format!("o{}", self.cuenta), c);
        // 5I (5E-10): sin el andamiaje de `comun::mandaba`; cada cuenta da sus órdenes de plaza con la matriz real.
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
    fn avanzar(&mut self, ms: Instante) {
        let t = self.n.ahora() + ms;
        self.n.avanzar_hasta(t);
    }
    /// El héroe de la cuenta, nacido (y residente) dentro del campamento 0.
    fn heroe(&mut self, cuenta: &str) -> u64 {
        self.ok(cuenta, Comando::CrearHeroe { cuenta: cuenta.into(), campamento: 0, controlador: Controlador::Humano, nombre: None }).creadas[0]
    }
    fn crear(&mut self, cuenta: &str, heroe: u64, nombre: &str) -> u32 {
        self.ok(cuenta, Comando::CrearFaccion { heroe, nombre: nombre.into(), sigilo: None }).creadas[0] as u32
    }
    fn faccion(&self, f: u32) -> &politica::faccion::Faccion {
        self.n.faccion(f).expect("la Facción existe")
    }
    fn faccion_de(&self, h: u64) -> Option<u32> {
        self.n.heroe(h).unwrap().faccion
    }
    fn residencia(&self, h: u64) -> Option<Residencia> {
        self.n.heroe(h).unwrap().heroe.residencia
    }
    fn hechos(&self, codigo: &str) -> Vec<nucleo::EventoNucleo> {
        self.n.cambios_desde(0).unwrap().flat_map(|c| c.eventos.iter()).filter(|e| e.codigo == codigo).cloned().collect()
    }
    /// La plaza de la Facción 1 que funda la cuenta `fundador` cerca del campamento `i`: su héroe de campo es ciudadano (la Facción nace al nombrarla) y reside en ella.
    fn plaza(&mut self, m: &M, i: usize, fundador: &str) -> u64 {
        let p = sitio_rico(&self.n, &m.0, i, 1);
        fundar(&mut self.n, fundador, 1, p)
    }
}

fn dato(e: &nucleo::EventoNucleo, clave: &str) -> i64 {
    e.dato(clave).unwrap_or_else(|| panic!("{} no lleva «{clave}»", e.codigo))
}

/// El sigilo libre de la Facción 1, con una pieza que no es del catálogo.
fn sigilo_inventado() -> politica::sigilo::Sigilo {
    let mut s = Reglas::default().economia.politica.sigilo.sigilo_libre("f-1", [].iter());
    s.emblema = "dragon".into();
    s
}

// ───────────────────────────── 1. Crear, entrar, salir ─────────────────────────────

#[test]
fn crear_pedir_responder_nombrar_y_dejar_la_faccion_con_la_sucesion_del_trono() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (ana, bea, cai) = (e.heroe("ana"), e.heroe("bea"), e.heroe("cai"));

    // Crear: nombre no vacío y libre, sigilo del catálogo, 1 héroe 1 Facción. Quien la crea es su primer ciudadano y su Rey.
    assert_eq!(e.no("ana", Comando::CrearFaccion { heroe: ana, nombre: "   ".into(), sigilo: None }), rechazos::FACCION_NOMBRE_VACIO);
    let mal = sigilo_inventado();
    assert_eq!(e.no("ana", Comando::CrearFaccion { heroe: ana, nombre: "Micenas".into(), sigilo: Some(mal) }), rechazos::FACCION_SIGILO_INVALIDO);
    assert_eq!(e.no("bea", Comando::CrearFaccion { heroe: ana, nombre: "Micenas".into(), sigilo: None }), rechazos::NO_AUTORIZADO, "nadie crea a nombre de otro");
    let r = e.ok("ana", Comando::CrearFaccion { heroe: ana, nombre: " Micenas ".into(), sigilo: None });
    assert_eq!((r.creadas.clone(), r.tipo_creadas), (vec![1], Some(nucleo::TipoCreado::Faccion)));
    let f = e.faccion(1);
    assert_eq!((f.nombre.as_str(), f.rey, f.embajador, f.ciudadanos.clone()), ("Micenas", Some(ana), None, vec![ana]));
    assert_eq!(e.faccion_de(ana), Some(1));
    let creada = &e.hechos("faccion.creada")[0];
    assert_eq!((dato(creada, "faccion"), dato(creada, "heroe"), creada.facciones.clone()), (1, ana as i64, vec![1]));
    assert_eq!(creada.valores, vec![("nombre", nucleo::Dato::Texto("Micenas".into()))]);
    assert_eq!(e.no("bea", Comando::CrearFaccion { heroe: bea, nombre: "MICENAS".into(), sigilo: None }), rechazos::FACCION_NOMBRE_DUPLICADO);
    let ocupado = e.faccion(1).sigilo.clone();
    assert_eq!(e.no("bea", Comando::CrearFaccion { heroe: bea, nombre: "Troya".into(), sigilo: Some(ocupado) }), rechazos::FACCION_SIGILO_DUPLICADO);
    assert_eq!(e.no("ana", Comando::CrearFaccion { heroe: ana, nombre: "Tebas".into(), sigilo: None }), rechazos::FACCION_YA_PERTENECE);

    // Pedir y responder: solo el Rey, y sin excepción (una Facción que no existe se rechaza con su código).
    e.ok("bea", Comando::SolicitarIngreso { heroe: bea, faccion: 1 });
    assert_eq!(e.ord("bea", Comando::SolicitarIngreso { heroe: bea, faccion: 1 }).resultado, Resultado::SinCambios, "pedir dos veces es pedir lo mismo");
    e.ok("cai", Comando::SolicitarIngreso { heroe: cai, faccion: 1 });
    assert_eq!(e.no("cai", Comando::SolicitarIngreso { heroe: cai, faccion: 9 }), rechazos::FACCION_NO_EXISTE);
    assert_eq!(e.faccion(1).solicitudes, vec![bea, cai]);
    assert_eq!(e.no("bea", Comando::ResponderSolicitud { heroe: bea, faccion: 1, solicitante: bea, aceptar: true }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.no("ana", Comando::ResponderSolicitud { heroe: ana, faccion: 9, solicitante: bea, aceptar: true }), rechazos::FACCION_NO_EXISTE);
    e.ok("ana", Comando::ResponderSolicitud { heroe: ana, faccion: 1, solicitante: bea, aceptar: true });
    e.ok("ana", Comando::ResponderSolicitud { heroe: ana, faccion: 1, solicitante: cai, aceptar: false });
    assert_eq!(e.no("ana", Comando::ResponderSolicitud { heroe: ana, faccion: 1, solicitante: cai, aceptar: true }), rechazos::FACCION_NO_HA_PEDIDO);
    let f = e.faccion(1);
    assert_eq!((f.ciudadanos.clone(), f.solicitudes.clone()), (vec![ana, bea], vec![]));
    assert_eq!((e.faccion_de(bea), e.faccion_de(cai)), (Some(1), None));
    let union = &e.hechos("faccion.ciudadania_union")[0];
    assert_eq!((dato(union, "heroe"), union.heroes.clone()), (bea as i64, vec![bea]), "el aceptado lo oye aunque no fuera de la Facción");
    assert_eq!(dato(&e.hechos("faccion.solicitud_denegada")[0], "heroe"), cai as i64);

    // Rey y Embajador: el Embajador lo nombra el Rey; el designado es ciudadano; el Rey se traspasa.
    assert_eq!(e.no("bea", Comando::AsignarEmbajador { heroe: bea, faccion: 1, designado: bea }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.no("ana", Comando::AsignarEmbajador { heroe: ana, faccion: 1, designado: cai }), rechazos::FACCION_NO_CIUDADANO);
    e.ok("ana", Comando::AsignarEmbajador { heroe: ana, faccion: 1, designado: bea });
    assert_eq!(e.ord("ana", Comando::AsignarEmbajador { heroe: ana, faccion: 1, designado: bea }).resultado, Resultado::SinCambios);
    assert_eq!(e.no("bea", Comando::AsignarRey { heroe: bea, faccion: 1, designado: bea }), rechazos::NO_AUTORIZADO, "con Rey, solo él traspasa el trono");
    assert_eq!(e.no("cai", Comando::AsignarRey { heroe: cai, faccion: 1, designado: cai }), rechazos::NO_AUTORIZADO, "quien no es ciudadano no nombra Rey");
    assert_eq!(e.faccion(1).embajador, Some(bea));

    // Dejar la Facción: el trono pasa al primero de los que quedan, que era el Embajador (la embajada queda vacante); estampa el cooldown de creación.
    e.ok("ana", Comando::DejarFaccion { heroe: ana });
    let f = e.faccion(1);
    assert_eq!((f.rey, f.embajador, f.ciudadanos.clone()), (Some(bea), None, vec![bea]));
    assert_eq!(e.faccion_de(ana), None);
    assert_eq!(e.no("ana", Comando::DejarFaccion { heroe: ana }), rechazos::FACCION_NO_PERTENECE);
    assert_eq!(e.no("ana", Comando::CrearFaccion { heroe: ana, nombre: "Tebas".into(), sigilo: None }), rechazos::FACCION_COOLDOWN_CREACION);
    e.ok("ana", Comando::SolicitarIngreso { heroe: ana, faccion: 1 });
    e.avanzar(7 * DIA);
    let tebas = e.crear("ana", ana, "Tebas");
    assert_eq!((tebas, e.faccion(1).solicitudes.clone()), (2, vec![]), "crear borra sus solicitudes en las demás");
    assert_eq!(e.ord("bea", Comando::AsignarRey { heroe: bea, faccion: 1, designado: bea }).resultado, Resultado::SinCambios, "ya es el Rey");
    let abandonada = &e.hechos("faccion.abandonada")[0];
    assert_eq!((dato(abandonada, "faccion"), dato(abandonada, "heroe")), (1, ana as i64));
    // La última que se va deja la Facción sin Rey; quien entra después es su Rey (5A-8).
    e.ok("bea", Comando::DejarFaccion { heroe: bea });
    assert_eq!((e.faccion(1).rey, e.faccion(1).ciudadanos.len()), (None, 0));
    e.ok("cai", Comando::SolicitarIngreso { heroe: cai, faccion: 1 });
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: cai, faccion: Some(1) });
    assert_eq!((e.faccion(1).rey, e.faccion(1).solicitudes.clone()), (Some(cai), vec![]));
    e.ok("cai", Comando::AsignarEmbajador { heroe: cai, faccion: 1, designado: cai });
    assert!(!e.hechos("cargo.embajador_asignado").is_empty());
}

#[test]
fn la_columna_de_quien_va_solo_toma_la_faccion_y_en_un_ejercito_no_se_deja() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let ana = e.heroe("ana");
    // Sale del campamento sola y sin bandera; al crear la Facción, su columna la toma.
    let col = e.ok("ana", Comando::SalirDelCampamento { campamento: 0, heroe: ana, escuadras: vec![], carga: vec![], politica: militar::columna::Politica::Rechazar, caravana_fundacion: None }).creadas[0];
    assert_eq!(e.n.ejercito(col).unwrap().columna.faccion, None);
    let f = e.crear("ana", ana, "Micenas");
    assert_eq!(e.n.ejercito(col).unwrap().columna.faccion, Some(f));
    // Q7 (5A-4): en un ejército, aunque sea solo, no se deja la Facción.
    let fundador = e.n.crear_ejercito(e.n.ahora(), "gus", f, campamento(&m.0, 1), 500, 100_000).unwrap();
    let gus = e.n.ejercito(fundador).unwrap().columna.lider;
    assert_eq!(e.n.ejercito(fundador).unwrap().columna.tipo, TipoColumna::Ejercito);
    assert_eq!(e.faccion(f).ciudadanos, vec![ana, gus], "el héroe de campo de la administración también es ciudadano");
    assert_eq!(e.no("gus", Comando::DejarFaccion { heroe: gus }), rechazos::FACCION_EN_EJERCITO);
    // La columna personal sí: queda sin bandera con ella.
    e.ok("ana", Comando::DejarFaccion { heroe: ana });
    assert_eq!((e.n.ejercito(col).unwrap().columna.faccion, e.faccion(f).rey), (None, Some(gus)));
}

// ───────────────────────────── 2. La residencia ─────────────────────────────

#[test]
fn las_tres_ordenes_de_residencia_con_su_cooldown_y_quien_deja_su_casa_dentro_sigue_dentro() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let plaza = e.plaza(m, 0, "fundador");
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 });
    let otra = e.plaza(m, 0, "fundadora");
    let ana = e.heroe("ana");
    assert_eq!(e.n.heroe(ana).unwrap().heroe.presencia.ubicacion, Ubicacion::Mercenarios(0));

    // Solo a una plaza de su Facción.
    assert_eq!(e.no("ana", Comando::CambiarResidencia { heroe: ana, asentamiento: plaza }), rechazos::RESIDENCIA_PLAZA_AJENA);
    assert_eq!(e.no("ana", Comando::DejarResidencia { heroe: ana }), rechazos::RESIDENCIA_SIN_PLAZA, "reside en un campamento");
    assert_eq!(e.no("ana", Comando::ResidirEnCampamento { heroe: ana, campamento: 0 }), rechazos::RESIDENCIA_YA_RESIDE);
    e.ok("ana", Comando::SolicitarIngreso { heroe: ana, faccion: 1 });
    let rey = e.faccion(1).rey.unwrap();
    e.ok("fundador", Comando::ResponderSolicitud { heroe: rey, faccion: 1, solicitante: ana, aceptar: true });

    // Se muda: estaba dentro del campamento donde residía y sigue dentro, de visita (un campamento admite a todos: 5B-1); su casa es la plaza.
    e.ok("ana", Comando::CambiarResidencia { heroe: ana, asentamiento: plaza });
    let h = e.n.heroe(ana).unwrap();
    assert_eq!((h.heroe.residencia, h.heroe.presencia.ubicacion, h.columna), (Some(Residencia::Asentamiento(plaza as u32)), Ubicacion::Mercenarios(0), None));
    let cambiada = &e.hechos("ciudadania.residencia_cambiada")[0];
    assert_eq!((dato(cambiada, "asentamiento"), cambiada.dato("ejercito"), cambiada.asentamientos.clone()), (plaza as i64, None, vec![plaza]));
    assert_eq!(e.n.ciudadanos_de(plaza), 2, "Q8: el fundador y ella");

    // El cooldown: tres días entre cambios hechos por orden suya.
    assert_eq!(e.no("ana", Comando::CambiarResidencia { heroe: ana, asentamiento: otra }), rechazos::RESIDENCIA_COOLDOWN);
    assert_eq!(e.no("ana", Comando::ResidirEnCampamento { heroe: ana, campamento: 1 }), rechazos::RESIDENCIA_COOLDOWN);
    assert_eq!(e.no("ana", Comando::DejarResidencia { heroe: ana }), rechazos::RESIDENCIA_COOLDOWN);
    e.avanzar(3 * DIA);
    assert_eq!(e.no("ana", Comando::CambiarResidencia { heroe: ana, asentamiento: plaza }), rechazos::RESIDENCIA_YA_RESIDE);
    e.ok("ana", Comando::CambiarResidencia { heroe: ana, asentamiento: otra });
    assert_eq!((e.n.ciudadanos_de(plaza), e.n.ciudadanos_de(otra)), (1, 2));

    // Dejar la residencia: al campamento de mercenarios más cercano a la plaza que deja (5B-4), no a donde está (dentro del campamento 0).
    e.avanzar(3 * DIA);
    e.ok("ana", Comando::DejarResidencia { heroe: ana });
    let donde = e.n.estado().asentamientos[&otra].posicion;
    let cercano = (0..m.0.campamentos().len()).min_by_key(|&i| (campamento(&m.0, i).dist2(donde), i)).unwrap() as u32;
    assert_eq!(e.residencia(ana), Some(Residencia::Campamento(cercano)));
    let dejada = &e.hechos("ciudadania.residencia_dejada")[0];
    assert_eq!((dato(dejada, "asentamiento"), dato(dejada, "campamento")), (otra as i64, i64::from(cercano)));
    assert_eq!(e.faccion_de(ana), Some(1), "sigue siendo ciudadana");

    // Residir en un campamento: cualquiera, de cualquier Facción.
    e.avanzar(3 * DIA);
    let otro = (cercano + 1) % m.0.campamentos().len() as u32;
    e.ok("ana", Comando::ResidirEnCampamento { heroe: ana, campamento: otro });
    assert_eq!(e.residencia(ana), Some(Residencia::Campamento(otro)));
    assert_eq!(dato(&e.hechos("ciudadania.residencia_en_campamento")[0], "campamento"), i64::from(otro));
}

#[test]
fn quien_deja_la_faccion_deja_su_plaza_por_el_campamento_mas_cercano_a_ella() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let plaza = e.plaza(m, 0, "fundador");
    let ana = e.heroe("ana");
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: ana, faccion: Some(1) });
    e.ok("ana", Comando::CambiarResidencia { heroe: ana, asentamiento: plaza });
    let mudanza = e.n.ahora();
    e.avanzar(DIA);
    e.ok("ana", Comando::DejarFaccion { heroe: ana });
    let p = e.n.estado().asentamientos[&plaza].posicion;
    let cercano = (0..m.0.campamentos().len()).min_by_key(|&i| (campamento(&m.0, i).dist2(p), i)).unwrap() as u32;
    assert_eq!((e.residencia(ana), e.faccion_de(ana)), (Some(Residencia::Campamento(cercano)), None));
    assert_eq!(e.n.ciudadanos_de(plaza), 1);
    let h = &e.hechos("faccion.abandonada")[0];
    assert_eq!((dato(h, "asentamiento"), dato(h, "campamento")), (plaza as i64, i64::from(cercano)));
    // No cuenta para el cooldown de residencia: el último cambio sigue siendo el de la mudanza.
    assert_eq!(e.n.heroe(ana).unwrap().residencia_cambiada_en, Some(mudanza));
}

/// 5B-1 (decisión del autor del 2026-10-24): quien deja su casa estando dentro de ella sigue dentro de visita si la puerta le admite; su tropa y su almacén personal pasan sin viaje a la
/// casa nueva (el campamento más cercano a la plaza que deja, 5B-4). Si deja la Facción y la puerta ya no le admite, sale expulsado al mapa solo, con su carro.
#[test]
fn quien_deja_su_casa_dentro_sigue_de_visita_si_la_puerta_le_admite_y_si_no_sale_solo_al_mapa() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let plaza = e.plaza(m, 0, "fundador");
    let p = e.n.estado().asentamientos[&plaza].posicion;
    let cercano = (0..m.0.campamentos().len()).min_by_key(|&i| (campamento(&m.0, i).dist2(p), i)).unwrap() as u32;
    let dentro = Ubicacion::Asentamiento(plaza as u32);
    let (ana, bea) = (e.heroe("ana"), e.heroe("bea"));
    let mut escuadras = Vec::new();
    for h in [ana, bea] {
        e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: h, faccion: Some(1) });
        e.ok("admin", Comando::FijarResidencia { heroe: h, residencia: Some(Residencia::Asentamiento(plaza as u32)) });
        e.ok("admin", Comando::ColocarHeroe { heroe: h, ubicacion: dentro });
        escuadras.push(e.ok("admin", Comando::DarEscuadra { heroe: h, tropa: militar::catalogo::Tropa::MiliciaLanceros, cantidad: 10 }).creadas[0]);
        e.ok("admin", Comando::DarAlmacenPersonal { heroe: h, recurso: asentamiento::Recurso::Madera, cantidad: 30_000 });
    }
    let suya = |e: &E, h: u64, s: u64| e.n.heroe(h).unwrap().heroe.escuadras.iter().find(|x| x.id == s).map(|x| x.contenedor);
    let personal = e.n.heroe(ana).unwrap().almacen_personal.clone();

    // Deja la residencia y sigue dentro (es de la Facción): su tropa y su almacén van con la casa nueva sin moverse.
    e.ok("ana", Comando::DejarResidencia { heroe: ana });
    let h = e.n.heroe(ana).unwrap();
    assert_eq!((h.heroe.residencia, h.heroe.presencia.ubicacion, h.columna), (Some(Residencia::Campamento(cercano)), dentro, None));
    assert_eq!((suya(&e, ana, escuadras[0]), &h.almacen_personal), (Some(militar::escuadra::Contenedor::Campamento), &personal));
    assert!(e.hechos("heroe.expulsado").is_empty());

    // Con la puerta abierta a los neutrales (por defecto, cerrada: Q1, 5E), quien deja la Facción también sigue dentro.
    e.ok("admin", comun::acceso(plaza, true, true, false));
    e.avanzar(DIA);
    e.ok("ana", Comando::DejarFaccion { heroe: ana });
    assert_eq!((e.n.heroe(ana).unwrap().heroe.presencia.ubicacion, e.faccion_de(ana)), (dentro, None));

    // Con la puerta cerrada a los neutrales, sale expulsada al mapa sola: su tropa se queda en su casa nueva; se lleva su carro y sus víveres.
    e.ok("admin", comun::acceso(plaza, true, false, false));
    let fuera = e.hechos("heroe.expulsado");
    assert_eq!(fuera.iter().map(|x| dato(x, "heroe")).collect::<Vec<_>>(), vec![ana as i64], "cerrar la puerta echa a la que ya no es de la Facción");
    e.ok("bea", Comando::DejarFaccion { heroe: bea });
    let h = e.n.heroe(bea).unwrap();
    let Ubicacion::Columna(c) = h.heroe.presencia.ubicacion else { panic!("sale al mapa: {:?}", h.heroe.presencia.ubicacion) };
    assert_eq!((h.heroe.residencia, suya(&e, bea, escuadras[1])), (Some(Residencia::Campamento(cercano)), Some(militar::escuadra::Contenedor::Campamento)));
    let col = &e.n.ejercito(c).unwrap().columna;
    assert_eq!((col.escuadras.len(), col.faccion, e.n.ejercito(c).unwrap().posicion), (0, None, p));
    let echada = e.hechos("heroe.expulsado").into_iter().find(|x| dato(x, "heroe") == bea as i64).expect("bea sale expulsada");
    assert_eq!((dato(&echada, "ejercito"), dato(&echada, "motivo")), (c as i64, 0));
    let abandonada = e.hechos("faccion.abandonada").into_iter().find(|x| dato(x, "heroe") == bea as i64).unwrap();
    assert_eq!((dato(&abandonada, "ejercito"), dato(&abandonada, "campamento")), (c as i64, i64::from(cercano)));
}

/// 5B-6 (2026-10-24): yendo en un ejército no se pide el ingreso ni se acepta (un ejército es de una sola Facción); denegarlo, sí.
#[test]
fn en_un_ejercito_no_se_pide_el_ingreso_ni_se_acepta() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (ana, bea) = (e.heroe("ana"), e.heroe("bea"));
    let f = e.crear("ana", ana, "Micenas");
    // Pide el ingreso y después va en un ejército (de otra Facción, por la administración).
    e.ok("bea", Comando::SolicitarIngreso { heroe: bea, faccion: f });
    let col = e.n.crear_ejercito(e.n.ahora(), "gus", 7, campamento(&m.0, 1), 500, 100_000).unwrap();
    e.ok("admin", Comando::ColocarHeroe { heroe: bea, ubicacion: Ubicacion::Columna(col) });
    assert_eq!(e.no("bea", Comando::SolicitarIngreso { heroe: bea, faccion: f }), rechazos::FACCION_EN_EJERCITO);
    assert_eq!(e.no("ana", Comando::ResponderSolicitud { heroe: ana, faccion: f, solicitante: bea, aceptar: true }), rechazos::FACCION_EN_EJERCITO);
    assert_eq!(e.faccion_de(bea), None);
    e.ok("ana", Comando::ResponderSolicitud { heroe: ana, faccion: f, solicitante: bea, aceptar: false });
    assert!(e.faccion(f).solicitudes.is_empty());
}

// ───────────────────────────── 3. Fundar y la nobleza ─────────────────────────────

#[test]
fn los_ciudadanos_de_la_columna_pasan_a_residir_en_la_plaza_que_fundan_y_sin_ninguno_no_se_funda() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let p = sitio_rico(&e.n, &m.0, 0, 1);
    // Una columna de la Facción 1 con su héroe de campo (ciudadano) y una heroína sin Facción.
    let col = e.n.crear_ejercito(0, "gus", 1, p, 500, 100_000).unwrap();
    let gus = e.n.ejercito(col).unwrap().columna.lider;
    let ana = e.heroe("ana");
    e.ok("admin", Comando::ColocarHeroe { heroe: ana, ubicacion: Ubicacion::Columna(col) });
    e.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
    // Sin ningún ciudadano de su Facción en la columna, no hay fundadores.
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: gus, faccion: None });
    assert_eq!(e.n.ejercito(col).unwrap().columna.faccion, Some(1), "la columna de un ejército no cambia de Facción");
    assert_eq!(e.no("gus", Comando::Fundar { ejercito: col }), rechazos::FUNDACION_SIN_FUNDADORES);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: gus, faccion: Some(1) });
    let plaza = e.ok("gus", Comando::Fundar { ejercito: col }).creadas[0];
    assert_eq!(e.residencia(gus), Some(Residencia::Asentamiento(plaza as u32)));
    assert_eq!(e.residencia(ana), Some(Residencia::Campamento(0)), "quien no es de la Facción no funda");
    assert_eq!(e.n.heroe(gus).unwrap().residencia_cambiada_en, None, "fundar no es una mudanza: no estampa el cooldown");
    // Q8: cuentan todos los residentes.
    assert_eq!(e.n.ciudadanos_de(plaza), 1);
    e.ok("admin", Comando::FijarFaccionDeHeroe { heroe: ana, faccion: Some(1) });
    e.ok("ana", Comando::CambiarResidencia { heroe: ana, asentamiento: plaza });
    assert_eq!(e.n.ciudadanos_de(plaza), 2);
}

// ───────────────────────────── 4. Proyección, foto y determinismo ─────────────────────────────

#[test]
fn la_faccion_se_proyecta_con_su_nombre_sigilo_rey_embajador_y_ciudadanos_en_orden_de_ingreso() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let (ana, bea) = (e.heroe("ana"), e.heroe("bea"));
    let f = e.crear("ana", ana, "Micenas");
    e.ok("bea", Comando::SolicitarIngreso { heroe: bea, faccion: f });
    let pb = propios::faccion(&e.n, f);
    assert_eq!((pb.nombre.as_str(), pb.rey_id.clone(), pb.ciudadanos_ids.clone(), pb.solicitudes_ids.clone()), ("Micenas", Some(ids::heroe(ana)), vec![ids::heroe(ana)], vec![ids::heroe(bea)]));
    let s = pb.sigilo.expect("con sigilo");
    assert_eq!(s.emblema_id, e.faccion(f).sigilo.emblema);
    e.ok("ana", Comando::ResponderSolicitud { heroe: ana, faccion: f, solicitante: bea, aceptar: true });
    e.ok("ana", Comando::AsignarEmbajador { heroe: ana, faccion: f, designado: bea });
    let pb = propios::faccion(&e.n, f);
    assert_eq!((pb.embajador_id, pb.ciudadanos_ids, pb.derrotada_por, pb.version), (Some(ids::heroe(bea)), vec![ids::heroe(ana), ids::heroe(bea)], None, e.n.version()));
}

/// Un guion con todas las órdenes de 5B; `paso` avanza el reloj entre unas y otras.
fn guion(m: &M, paso: &dyn Fn(&mut E, Instante)) -> E {
    let mut e = E::nuevo(m);
    let plaza = e.plaza(m, 0, "fundador");
    let (ana, bea) = (e.heroe("ana"), e.heroe("bea"));
    e.crear("ana", ana, "Micenas");
    e.ok("bea", Comando::SolicitarIngreso { heroe: bea, faccion: 1 });
    e.ok("bea", Comando::SolicitarIngreso { heroe: bea, faccion: 2 });
    paso(&mut e, DIA);
    let rey = e.faccion(1).rey.unwrap();
    e.ok("fundador", Comando::ResponderSolicitud { heroe: rey, faccion: 1, solicitante: bea, aceptar: true });
    e.ok("bea", Comando::CambiarResidencia { heroe: bea, asentamiento: plaza });
    e.ok("bea", Comando::RenombrarAsentamiento { heroe: bea, asentamiento: plaza, nombre: "Nueva Micenas".into() });
    paso(&mut e, 4 * DIA);
    e.ok("bea", Comando::DejarResidencia { heroe: bea });
    e.ok("ana", Comando::DejarFaccion { heroe: ana });
    paso(&mut e, 2 * DIA);
    e
}

#[test]
fn mismas_ordenes_dan_el_mismo_mundo_normal_y_a_trozos_y_nada_espera_en_la_cola() {
    let m = con_mundo!();
    let normal = guion(m, &|e, ms| e.avanzar(ms));
    let a_trozos = guion(m, &|e, ms| (0..8).for_each(|_| e.avanzar(ms / 8)));
    assert_eq!(normal.n.hash_estado(), a_trozos.n.hash_estado());
    // Una Facción, una solicitud o un cooldown no esperan ningún evento: la cola es la de un mundo sin política (lo que piden las plazas y sus bandidos).
    let mut sin_politica = E::nuevo(m);
    sin_politica.plaza(m, 0, "fundador");
    sin_politica.avanzar(7 * DIA);
    let citas = |n: &Nucleo| n.estado().cola.volcar().iter().filter(|x| n.estado().generaciones.es_vigente(x.clave)).count();
    assert_eq!(citas(&normal.n), citas(&sin_politica.n));
}

#[test]
fn la_politica_viaja_en_la_foto_ban32_y_las_ban31_ban30_ban29_y_ban27_se_rechazan_con_su_motivo() {
    let m = con_mundo!();
    let e = guion(m, &|e, ms| e.avanzar(ms));
    let (reglas, hash) = (Reglas::default(), m.0.hash().to_string());
    let bytes = e.n.estado().serializar();
    assert_eq!(&bytes[..5], b"BAN49");
    let s = Estado::deserializar(&bytes, &reglas, &hash).unwrap();
    assert_eq!((s.politica.clone(), s.hash()), (e.n.estado().politica.clone(), e.n.hash_estado()));
    assert!(s.politica.facciones.iter().count() == 2 && s.militar.heroes.values().any(|h| h.residencia_cambiada_en.is_some()));
    let vieja = [b"BAN27".as_slice(), &bytes[5..]].concat();
    let error = Estado::deserializar(&vieja, &reglas, &hash).err().expect("BAN27 se rechaza");
    assert!(error.0.contains("BAN27") && error.0.contains("Facciones no existen como entidad"), "{}", error.0);
    let vieja = [b"BAN31".as_slice(), &bytes[5..]].concat();
    let error = Estado::deserializar(&vieja, &reglas, &hash).err().expect("BAN31 se rechaza");
    assert!(error.0.contains("BAN31") && error.0.contains("relaciones son pares sin dirección"), "{}", error.0);
    let vieja = [b"BAN30".as_slice(), &bytes[5..]].concat();
    let error = Estado::deserializar(&vieja, &reglas, &hash).err().expect("BAN30 se rechaza");
    assert!(error.0.contains("BAN30") && error.0.contains("desde cuándo son de su Facción"), "{}", error.0);
    let vieja = [b"BAN29".as_slice(), &bytes[5..]].concat();
    let error = Estado::deserializar(&vieja, &reglas, &hash).err().expect("BAN29 se rechaza");
    assert!(error.0.contains("BAN29") && error.0.contains("nombre ni marca de capital"), "{}", error.0);
    // El nombre de la plaza y su marca de capital viajan en la foto y mueven el hash (5C).
    let plaza = *s.asentamientos.keys().next().unwrap();
    assert_eq!(s.asentamientos[&plaza].nombre, "Nueva Micenas");
    let mut otro = Estado::deserializar(&bytes, &reglas, &hash).unwrap();
    otro.asentamientos.get_mut(&plaza).unwrap().capital_de = Some(1);
    assert_ne!(otro.hash(), s.hash());
    // Cada pieza de la Facción mueve el hash.
    let mut otro = Estado::deserializar(&bytes, &reglas, &hash).unwrap();
    otro.politica.facciones.otorgar(1, 999_999).unwrap();
    assert_ne!(otro.hash(), s.hash());
}

// ───────────────────────────── 5. La experiencia, la capital y el nombre (5C) ─────────────────────────────

/// Una Facción con dos plazas vecinas (la A, la más antigua, y la B junto a ella, las dos cerca del campamento 0): la experiencia de fundar la segunda, la capital efectiva y la designada.
#[test]
fn la_capital_es_la_mas_antigua_hasta_que_el_rey_designa_otra_con_palacio_y_solo_ella_levanta_la_gran_fundicion() {
    let m = con_mundo!();
    // Sin mantenimiento (gracia infinita): las plazas suben de nivel a mano y esperan días a sus obras; aquí se mide la capital, no la economía.
    let mut e = E::nuevo(m);
    let mut reglas = arranque(m, 5).reglas;
    std::sync::Arc::make_mut(&mut reglas.economia.datos).mantenimiento.gracia_minutos = i64::MAX / 4;
    e.n = Nucleo::nuevo(Arranque { reglas, ..arranque(m, 5) });
    let a = e.plaza(m, 0, "fundador");
    assert_eq!((e.faccion(1).experiencia, e.n.nivel_de_faccion(1)), (0, 1), "la primera plaza no da experiencia");
    // El cap de fundación sale del nivel de Facción: con el 1, una sola plaza.
    let p = sitio_rico(&e.n, &m.0, 0, 1);
    let col = e.n.crear_ejercito(e.n.ahora(), "fundadora", 1, p, 500, 100_000).unwrap();
    e.ok("admin", Comando::DarCaravanaFundacion { ejercito: col });
    assert_eq!(e.no("fundadora", Comando::Fundar { ejercito: col }), rechazos::CAP_ALCANZADO);
    // La administración fija el nivel poniendo la experiencia en el umbral (decisión 9).
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 });
    assert_eq!((e.faccion(1).experiencia, e.n.nivel_de_faccion(1)), (100_000, 2));
    let b = e.ok("fundadora", Comando::Fundar { ejercito: col }).creadas[0];
    assert_eq!(e.faccion(1).experiencia, 120_000, "la segunda plaza da 20");
    let fundacion = e.hechos("faccion.experiencia").into_iter().find(|h| dato(h, "motivo") == 7).expect("experiencia de fundación");
    assert_eq!((dato(&fundacion, "cantidad"), dato(&fundacion, "experiencia")), (20_000, 120_000));

    // La capital efectiva: la más antigua; la proyección la lleva.
    assert_eq!(e.n.capital_de(1), Some(a));
    assert_eq!(propios::faccion(&e.n, 1).capital_id, Some(ids::asentamiento(a)));
    let rey = e.faccion(1).rey.unwrap();
    let cuenta_rey = e.n.heroe(rey).unwrap().cuenta.clone();
    let otro = e.n.ejercito(col).unwrap().columna.lider;
    // Solo el Rey; una plaza propia con Palacio activo que no sea ya la capital.
    assert_eq!(e.no("fundadora", Comando::DesignarCapital { heroe: otro, faccion: 1, asentamiento: b }), rechazos::NO_AUTORIZADO);
    assert_eq!(e.no(&cuenta_rey, Comando::DesignarCapital { heroe: rey, faccion: 1, asentamiento: b }), rechazos::CAPITAL_SIN_PALACIO);
    assert_eq!(e.no(&cuenta_rey, Comando::DesignarCapital { heroe: rey, faccion: 9, asentamiento: b }), rechazos::FACCION_NO_EXISTE);
    // Recursos para obras grandes: un almacén lleno de lo que piden, con la auto-construcción parada.
    use asentamiento::Recurso::{Madera, Oro, Piedra, Trigo};
    let llenar = |e: &mut E, plaza: u64| {
        for r in [Madera, Piedra, Oro, Trigo] {
            e.ord("admin", Comando::DarRecursos { asentamiento: plaza, recurso: r, cantidad: 2_000_000 });
        }
    };
    let gf = asentamiento::Tipo::GranFundicion;
    // El Palacio de B (nivel 2, y un Almacén para que quepan sus 600 de madera).
    e.ok("admin", Comando::AlternarAutoConstruccion { asentamiento: b, pausada: true });
    e.ok("admin", Comando::FijarNivelAsentamiento { asentamiento: b, nivel: 2 });
    llenar(&mut e, b);
    e.ok("admin", Comando::AnadirEdificio { asentamiento: b, tipo: asentamiento::Tipo::Almacen });
    e.avanzar(7 * 60 * MS_POR_MINUTO);
    llenar(&mut e, b);
    e.ok("admin", Comando::AnadirEdificio { asentamiento: b, tipo: asentamiento::Tipo::Palacio });
    e.avanzar(3 * DIA + 60 * MS_POR_MINUTO);
    assert!(e.n.economia(b).unwrap().edificios.iter().any(|x| x.tipo == asentamiento::Tipo::Palacio && x.activo()), "el Palacio de B está en pie");
    // La Gran Fundición: nivel de Facción 6, la capital (todavía A) y ninguna otra en la Facción.
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 6 });
    for plaza in [a, b] {
        e.ord("admin", Comando::AlternarAutoConstruccion { asentamiento: plaza, pausada: true });
        e.ord("admin", Comando::FijarNivelAsentamiento { asentamiento: plaza, nivel: 4 });
        llenar(&mut e, plaza);
    }
    assert_eq!(e.no("admin", Comando::AnadirEdificio { asentamiento: b, tipo: gf }), rechazos::OBRA_GRAN_FUNDICION, "B no es la capital");
    e.ok("admin", Comando::AnadirEdificio { asentamiento: a, tipo: gf });
    // El Rey designa B; la marca pasa a ella y la Facción estampa el instante.
    let t = e.n.ahora();
    e.ok(&cuenta_rey, Comando::DesignarCapital { heroe: rey, faccion: 1, asentamiento: b });
    assert_eq!((e.n.capital_de(1), e.faccion(1).capital_designada_en), (Some(b), Some(t)));
    assert_eq!((e.n.estado().asentamientos[&b].capital_de, e.n.estado().asentamientos[&a].capital_de), (Some(1), None));
    let designada = &e.hechos("faccion.capital_designada")[0];
    assert_eq!((dato(designada, "faccion"), dato(designada, "asentamiento"), designada.asentamientos.clone()), (1, b as i64, vec![b]));
    assert_eq!(e.no(&cuenta_rey, Comando::DesignarCapital { heroe: rey, faccion: 1, asentamiento: b }), rechazos::CAPITAL_YA_ES_LA_CAPITAL);
    assert_eq!(e.no(&cuenta_rey, Comando::DesignarCapital { heroe: rey, faccion: 1, asentamiento: a }), rechazos::CAPITAL_SIN_PALACIO);
    let ficha = propios::faccion(&e.n, 1);
    assert_eq!((ficha.capital_id, ficha.capital_designada_en_ms, ficha.nivel), (Some(ids::asentamiento(b)), Some(t), e.n.nivel_de_faccion(1)));
    // B es ya la capital, pero la Facción tiene su Gran Fundición (en A): no se levanta otra.
    assert_eq!(e.no("admin", Comando::AnadirEdificio { asentamiento: b, tipo: gf }), rechazos::OBRA_GRAN_FUNDICION, "una sola por Facción");
}

/// La distancia a la capital sube el mantenimiento (Economia_Construccion §10.1, 5C): una segunda plaza de la Facción a más de 1.000 u de la capital paga más madera que sin ese factor
/// (escala infinita). Con la escala de 300 u (P1, 2026-10-25) la plaza a 1.345 u (×5,5) sigue en pie; con la de 80 u de antes (×17,8) caía en ruinas en un día.
#[test]
fn la_distancia_a_la_capital_sube_el_mantenimiento_de_la_plaza_lejana() {
    let m = con_mundo!();
    let lejana = |escala: Option<i64>| {
        let mut e = E::nuevo(m);
        if let Some(x) = escala {
            let mut reglas = arranque(m, 5).reglas;
            std::sync::Arc::make_mut(&mut reglas.economia.datos).mantenimiento.escala_distancia = x;
            e.n = Nucleo::nuevo(Arranque { reglas, ..arranque(m, 5) });
        }
        let a = e.plaza(m, 0, "fundador");
        e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 });
        let b = e.plaza(m, 2, "fundadora");
        let d = e.n.estado().asentamientos[&a].posicion.dist(e.n.estado().asentamientos[&b].posicion);
        assert!(d > 1_000_000, "a {d} milésimas");
        e.avanzar(2 * DIA);
        e.n.economia(b).map(|x| x.almacen.cantidad(asentamiento::Recurso::Madera))
    };
    let (vigente, sin_factor) = (lejana(None), lejana(Some(i64::MAX / 4)));
    assert!(vigente.is_some() && sin_factor.is_some(), "con 300 u la plaza lejana sigue en pie (P1)");
    assert!(vigente < sin_factor, "la plaza lejana paga ×(1 + d/300): {vigente:?} de madera frente a {sin_factor:?}");
    assert!(lejana(Some(80_000)).is_none(), "con la escala de 80 u de antes caía en ruinas");
}

/// El nombre: cada plaza nace con uno libre de la lista (`nombreDeCiudadLibre`); quien reside en ella lo cambia (recortado; vacío, sin nombre) y la ficha lo lleva.
#[test]
fn cada_plaza_nace_con_un_nombre_libre_y_quien_reside_en_ella_lo_cambia() {
    let m = con_mundo!();
    let mut e = E::nuevo(m);
    let a = e.plaza(m, 0, "fundador");
    e.ok("admin", Comando::FijarNivelFaccion { faccion: 1, nivel: 2 });
    let b = e.plaza(m, 0, "fundadora");
    let nombre = |e: &E, p: u64| e.n.estado().asentamientos[&p].nombre.clone();
    let (na, nb) = (nombre(&e, a), nombre(&e, b));
    assert!(politica::nombres::NOMBRES_DE_CIUDADES.contains(&na.as_str()) && politica::nombres::NOMBRES_DE_CIUDADES.contains(&nb.as_str()) && na != nb, "{na} y {nb}");
    assert_eq!(na, politica::nombres::ciudad_libre([], &ids::asentamiento(a)).unwrap(), "el de su id, sobre una partida sin nombres");
    let fundada = &e.hechos("fundacion.asentamiento_fundado")[0];
    assert_eq!(fundada.valores, vec![("nombre", nucleo::Dato::Texto(na.clone()))]);
    // Solo quien reside en ella (el fundador de A); el nombre se recorta.
    let fundador = e.n.estado().militar.residentes_de(a).next().unwrap().heroe.id;
    let ana = e.heroe("ana");
    assert_eq!(e.no("ana", Comando::RenombrarAsentamiento { heroe: ana, asentamiento: a, nombre: "Mía".into() }), rechazos::NO_AUTORIZADO);
    e.ok("fundador", Comando::RenombrarAsentamiento { heroe: fundador, asentamiento: a, nombre: "  Micenas Alta  ".into() });
    assert_eq!(nombre(&e, a), "Micenas Alta");
    assert_eq!(e.ord("fundador", Comando::RenombrarAsentamiento { heroe: fundador, asentamiento: a, nombre: "Micenas Alta".into() }).resultado, Resultado::SinCambios);
    let renombrado = &e.hechos("asentamiento.renombrado")[0];
    assert_eq!((renombrado.asentamientos.clone(), renombrado.valores.clone()), (vec![a], vec![("nombre", nucleo::Dato::Texto("Micenas Alta".into()))]));
    let ficha = nucleo::proyeccion::ficha_de_asentamiento(&e.n.estado().asentamientos[&a], &[], None);
    assert_eq!(ficha.nombre.as_deref(), Some("Micenas Alta"));
    // Como mucho 32 caracteres (no bytes) y único entre las plazas vivas, sin mirar tildes ni mayúsculas (5C-6).
    let renombrar = |e: &mut E, nombre: &str| e.ord("fundador", Comando::RenombrarAsentamiento { heroe: fundador, asentamiento: a, nombre: nombre.into() });
    assert_eq!(codigo(&renombrar(&mut e, &"x".repeat(33))), rechazos::ASENTAMIENTO_NOMBRE_LARGO);
    assert!(renombrar(&mut e, &"á".repeat(32)).aceptada(), "32 caracteres de dos bytes caben");
    let otra = nb.to_uppercase();
    assert_eq!(codigo(&renombrar(&mut e, &otra)), rechazos::ASENTAMIENTO_NOMBRE_DUPLICADO, "{otra}: el de B, en mayúsculas");
    // Vacío: la plaza se queda sin nombre (y dos plazas sin nombre no chocan).
    e.ok("fundador", Comando::RenombrarAsentamiento { heroe: fundador, asentamiento: a, nombre: "   ".into() });
    assert_eq!(nucleo::proyeccion::ficha_de_asentamiento(&e.n.estado().asentamientos[&a], &[], None).nombre, None);
}
